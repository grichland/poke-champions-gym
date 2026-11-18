//! The live session's protocol (battle::live): answers kept by seq, refusals, retries, stale requests,
//! restarts, a last resort when nothing is legal, and bad input.

mod common;
use battle::json::Json;
use battle::live::{AskKind, Open, Session, Step};
use battle::*;
use common::*;

fn start() -> String {
    msg(Body::BattleStart { own_team: team(), opp_preview: preview() }).to_json().to_string()
}

fn m(body: Body) -> String {
    msg(body).to_json().to_string()
}

fn events(list: &[&str]) -> String {
    let evs = list.iter().map(|j| Event::from_json(&Json::parse(j).unwrap()).unwrap()).collect();
    m(Body::Events { events: evs, fallback: None })
}

fn request(seq: i64, kind: ReqKind, refused: Option<Order>, retry: bool) -> String {
    m(Body::DecisionRequest { seq, kind, deadline_ms: None, refused, retry, screen_offer: None })
}

fn reply(s: Step) -> Json {
    match s {
        Step::Reply(j) => j,
        x => panic!("{:?}", x),
    }
}

fn order(j: &Json) -> Order {
    Order::from_json(j.get("order").unwrap()).unwrap()
}

/// battle_start, picks [0, 1, 2], the leads and turn 1.
fn session() -> Session {
    let mut s = Session::new();
    assert_eq!(s.handle_text(&start()), Step::None);
    assert_eq!(s.handle_text(&m(Body::PreviewRequest { deadline_ms: None })), Step::Ask(Open { seq: -1, kind: AskKind::Preview }));
    assert_eq!(s.answer_picks(&[0, 1, 2], "t").to_string(), r#"{"picks":[0,1,2]}"#);
    let e = events(&[r#"{"k":"turn","n":1}"#, r#"{"k":"enter","side":"opp","mon":"garchomp"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    assert_eq!(s.handle_text(&e), Step::None);
    s
}

#[test]
fn answers_repeat_by_seq_and_after_a_restart() {
    let mut s = session();
    assert_eq!(s.handle_text(&request(1, ReqKind::Turn, None, false)), Step::Ask(Open { seq: 1, kind: AskKind::Turn }));
    let a = s.answer(1, "t");
    assert_eq!(order(&a), Order::Move { mv: "shadowball".into(), mega: false });
    // the same seq, or a retry with a new one: the same order, no Player asked
    assert_eq!(order(&reply(s.handle_text(&request(1, ReqKind::Turn, None, false)))), order(&a));
    let r = reply(s.handle_text(&request(2, ReqKind::Turn, None, true)));
    assert_eq!((order(&r), r.get("seq").unwrap().as_i64()), (order(&a), Some(2)));
    // a restart: everything again from battle_start; the answered requests are not asked again
    let b = s.battle().clone();
    s.handle_text(&start());
    assert_eq!(s.handle_text(&m(Body::PreviewRequest { deadline_ms: None })), Step::Reply(Picks { picks: vec![0, 1, 2] }.to_json()));
    s.handle_text(&events(&[r#"{"k":"turn","n":1}"#, r#"{"k":"enter","side":"opp","mon":"garchomp"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]));
    assert_eq!(order(&reply(s.handle_text(&request(1, ReqKind::Turn, None, false)))), order(&a));
    assert_eq!(order(&reply(s.handle_text(&request(2, ReqKind::Turn, None, true)))), order(&a));
    assert_eq!(s.battle(), &b);
    assert_eq!(s.restarts, 1);
    assert!(s.notes.is_empty(), "{:?}", s.notes);
}

#[test]
fn refusal_takes_the_order_out() {
    let mut s = session();
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    let a = order(&s.answer(1, "t"));
    let before = s.legal().unwrap();
    assert_eq!(s.handle_text(&request(2, ReqKind::Turn, Some(a.clone()), false)), Step::Ask(Open { seq: 2, kind: AskKind::Turn }));
    let after = s.legal().unwrap();
    assert!(before.mask[1] && !after.mask[1] && after.count() + 1 == before.count());
    // the Player names the refused action anyway: the first legal one goes instead, noted
    let r = s.answer(1, "t");
    assert_ne!(order(&r), a);
    assert!(r.get("notes").unwrap().get("replaced").is_some());
    assert_eq!(s.notes.len(), 1);
}

#[test]
fn stale_request_is_not_applied() {
    let mut s = session();
    s.handle_text(&request(5, ReqKind::Turn, None, false));
    s.answer(0, "t");
    let b = s.builder.clone();
    let r = reply(s.handle_text(&request(4, ReqKind::Replace, None, false)));
    assert!(r.get("error").is_some());
    assert_eq!(s.builder, b);
}

#[test]
fn nothing_legal_gets_a_last_resort() {
    let mut s = session();
    s.handle_text(&request(1, ReqKind::Replace, None, false));
    for (q, mon) in [(2, "garchomp"), (3, "golisopod")] {
        s.handle_text(&request(q, ReqKind::Replace, Some(Order::Switch { mon: mon.into() }), false));
    }
    assert_eq!(s.legal().unwrap().count(), 0);
    let r = s.answer(9, "t");
    let n = r.get("notes").unwrap();
    assert_eq!(n.get("last_resort").and_then(|x| x.as_bool()), Some(true));
    assert!(matches!(order(&r), Order::Switch { .. }));
}

#[test]
fn bad_input_is_answered_not_raised() {
    let mut s = Session::new();
    for t in ["", "nonsense", "[]", "{}", r#"{"type":"events","battle":"b-test","v":"0.6","events":[]}"#] {
        assert!(matches!(s.handle_text(t), Step::Reply(j) if j.get("error").is_some()), "{}", t);
    }
    let mut s = session();
    let r = reply(s.handle_text(r#"{"v":"0.6","battle":"b-test","type":"events","events":[{"k":"turn","n":2},{"k":"nonsense"}]}"#));
    assert_eq!(r.get("dropped").unwrap().as_arr().unwrap().len(), 1);
    assert_eq!(s.battle().turn, 2);
    let r = reply(s.handle_text(r#"{"v":"0.6","battle":"other","type":"decision_request","seq":1,"kind":"turn"}"#));
    assert!(r.get("error").is_some());
    assert!(s.handle_text(&request(1, ReqKind::Turn, None, false)) == Step::Ask(Open { seq: 1, kind: AskKind::Turn }));
}

/// A message with one more field (the 0.7 fields are read from the message itself).
fn with(text: &str, k: &str, v: &str) -> String {
    let mut j = Json::parse(text).unwrap();
    j.set(k, Json::parse(v).unwrap());
    j.to_string()
}

fn leads(own: &str) -> String {
    events(&[r#"{"k":"turn","n":1}"#, r#"{"k":"enter","side":"opp","mon":"garchomp"}"#, &format!(r#"{{"k":"enter","side":"own","mon":"{}"}}"#, own)])
}

fn switch_targets(s: &Session) -> Vec<Order> {
    let l = s.legal().unwrap();
    (8..N_ACTIONS).filter_map(|a| order_for(s.battle(), &l, a)).collect()
}

fn session_with_lead(picks: &[i64], lead: &str) -> Session {
    let mut s = Session::new();
    s.handle_text(&start());
    s.handle_text(&m(Body::PreviewRequest { deadline_ms: None }));
    s.answer_picks(picks, "t");
    s.handle_text(&leads(lead));
    s
}

/// S8-1a: `battle_start.picks`: the Player is not asked at preview; the reply is those picks, noted.
#[test]
fn picks_given_at_battle_start() {
    let mut s = Session::new();
    s.handle_text(&with(&start(), "picks", "[3,4,0]"));
    assert_eq!(s.battle().picks, [Some(3), Some(4), Some(0)]);
    let r = reply(s.handle_text(&m(Body::PreviewRequest { deadline_ms: None })));
    assert_eq!(r.to_string(), r#"{"notes":{"given":true},"picks":[3,4,0]}"#);
    s.handle_text(&leads("primarina"));
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    assert_eq!(switch_targets(&s), vec![Order::Switch { mon: "mamoswine".into() }, Order::Switch { mon: "gholdengo".into() }]);
    assert!(s.builder.anomalies.is_empty() && s.builder.corrections.is_empty() && s.notes.is_empty());
    // picks that are not three team indices are ignored, noted
    let mut s = Session::new();
    s.handle_text(&with(&start(), "picks", "[1,1,2]"));
    assert_eq!(s.battle().picks, [None; 3]);
    assert_eq!(s.notes.len(), 1);
}

/// S8-1b: a preview fallback on the next `events` message gives the three brought.
#[test]
fn preview_fallback_sets_the_picks() {
    let mut s = Session::new();
    s.handle_text(&start());
    s.handle_text(&m(Body::PreviewRequest { deadline_ms: None }));
    s.answer_picks(&[0, 1, 2], "t");
    s.handle_text(&with(&leads("primarina"), "fallback", r#"{"picks":[3,5,1],"reason":"overruled"}"#));
    assert_eq!(s.battle().picks, [Some(3), Some(5), Some(1)]);
    assert!(s.builder.anomalies.is_empty() && s.builder.corrections.is_empty());
    // several fallbacks in one message apply in their order
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(0, "t");
    s.handle_text(&with(&events(&[]), "fallback", r#"[{"seq":1,"reason":"late"},{"seq":1,"order":{"kind":"move","move":"moonblast"}}]"#));
    assert_eq!(s.battle().ctx.own_choice, cg_engine::sim::data::MoveId::from_id("moonblast"));
    assert_eq!(s.builder.fallbacks.len(), 3);
}

/// S8-1c, rule 32: the picks held were not played. The lead is the first pick (it swaps with it, or takes
/// its place); a Pokémon outside full picks takes the place of the first pick not seen entering; the rows
/// and switch actions follow the corrected picks.
#[test]
fn picks_corrected_from_the_log() {
    let mut s = session_with_lead(&[0, 1, 2], "primarina");
    assert_eq!(s.battle().picks, [Some(3), Some(1), Some(2)]);
    assert_eq!(s.builder.corrections.len(), 1);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    // picks not contradicted stay
    assert_eq!(switch_targets(&s), vec![Order::Switch { mon: "garchomp".into() }, Order::Switch { mon: "golisopod".into() }]);
    s.answer(0, "t");
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"primarina"}"#, r#"{"k":"enter","side":"own","mon":"cinderace"}"#, r#"{"k":"turn","n":2}"#]));
    assert_eq!(s.battle().picks, [Some(3), Some(5), Some(2)]);
    s.handle_text(&request(2, ReqKind::Turn, None, false));
    assert_eq!(switch_targets(&s), vec![Order::Switch { mon: "primarina".into() }, Order::Switch { mon: "golisopod".into() }]);
    assert_eq!(s.builder.corrections.len(), 2);
    // a lead among the picks swaps with the first
    let s = session_with_lead(&[0, 1, 2], "golisopod");
    assert_eq!(s.battle().picks, [Some(2), Some(1), Some(0)]);
    // the right lead, a back pick outside the held picks: it takes the first place not seen entering
    let mut s = session_with_lead(&[0, 1, 2], "gholdengo");
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(0, "t");
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"golisopod"}"#, r#"{"k":"turn","n":2}"#]));
    s.handle_text(&request(2, ReqKind::Turn, None, false));
    s.answer(0, "t");
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"golisopod"}"#, r#"{"k":"enter","side":"own","mon":"mamoswine"}"#, r#"{"k":"turn","n":3}"#]));
    assert_eq!(s.battle().picks, [Some(0), Some(4), Some(2)]);
    assert_eq!(s.builder.corrections.len(), 1);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
    // a fourth own Pokémon after three entered is still an anomaly
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"mamoswine"}"#, r#"{"k":"enter","side":"own","mon":"cinderace"}"#]));
    assert_eq!(s.builder.anomalies.iter().filter(|a| a.what.contains("not picked")).count(), 1);
    assert_eq!(s.battle().picks, [Some(0), Some(4), Some(2)]);
    // no picks held (no preview answer): learnt in order of entry, nothing corrected
    let mut s = Session::new();
    s.handle_text(&start());
    s.handle_text(&leads("primarina"));
    assert_eq!(s.battle().picks, [Some(3), None, None]);
    assert!(s.builder.corrections.is_empty());
}

/// S8-2: a replayed request with `played` is recorded as our answer, and the Player is not asked; without
/// `played` and with no earlier answer the answer is unknown.
#[test]
fn replayed_requests() {
    let mut s = Session::new();
    s.handle_text(&with(&start(), "picks", "[0,1,2]"));
    s.handle_text(&with(&m(Body::PreviewRequest { deadline_ms: None }), "replay", "true"));
    s.handle_text(&leads("gholdengo"));
    let r = reply(s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "played", r#"{"kind":"move","move":"nastyplot"}"#)));
    assert_eq!(order(&r), Order::Move { mv: "nastyplot".into(), mega: false });
    let n = r.get("notes").unwrap();
    assert_eq!(n.get("replayed").and_then(|x| x.as_bool()), Some(true));
    assert_eq!(n.get("player").and_then(|x| x.as_str()), Some("played"));
    assert_eq!(n.get("action").and_then(|x| x.as_i64()), Some(2));
    assert_eq!(s.battle().ctx.own_choice, cg_engine::sim::data::MoveId::from_id("nastyplot"));
    assert_eq!(s.open, None);
    // replayed, answer unknown: no order, our choice not known
    s.handle_text(&events(&[&mv("own", "gholdengo", "nastyplot"), r#"{"k":"turn","n":2}"#]));
    let r = reply(s.handle_text(&with(&request(2, ReqKind::Turn, None, false), "replay", "true")));
    assert_eq!(r.to_string(), r#"{"replayed":true,"seq":2}"#);
    assert_eq!(s.battle().ctx.own_choice, None);
    // the live request after the replay is asked as usual
    s.handle_text(&events(&[r#"{"k":"turn","n":3}"#]));
    assert_eq!(s.handle_text(&request(3, ReqKind::Turn, None, false)), Step::Ask(Open { seq: 3, kind: AskKind::Turn }));
    assert!(s.notes.is_empty(), "{:?}", s.notes);
}

/// S8-2: a resend into a session that was not restarted (same tag) gives the earlier answers and the same
/// Battle, with or without the replay marks; `played` that differs from the earlier answer wins, noted.
#[test]
fn resend_into_the_same_session() {
    let mut s = session();
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    let a = s.answer(1, "t");
    s.handle_text(&events(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"turn","n":2}"#]));
    s.handle_text(&request(2, ReqKind::Turn, None, false));
    let b2 = s.answer(1, "t");
    let first = s.builder.battle;
    let played = a.get("order").unwrap().to_string();
    for marks in [false, true] {
        s.handle_text(&start());
        let pv = m(Body::PreviewRequest { deadline_ms: None });
        let r = reply(s.handle_text(&if marks { with(&pv, "replay", "true") } else { pv }));
        assert_eq!(r.get("picks").unwrap().to_string(), "[0,1,2]");
        s.handle_text(&leads("gholdengo"));
        let q1 = request(1, ReqKind::Turn, None, false);
        let r1 = reply(s.handle_text(&if marks { with(&q1, "played", &played) } else { q1 }));
        assert_eq!(order(&r1), order(&a));
        s.handle_text(&events(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"turn","n":2}"#]));
        let q2 = request(2, ReqKind::Turn, None, false);
        let r2 = reply(s.handle_text(&if marks { with(&q2, "replay", "true") } else { q2 }));
        assert_eq!(order(&r2), order(&b2));
        assert_eq!(s.battle(), &first);
    }
    assert!(s.notes.is_empty(), "{:?}", s.notes);
    // played differs from what CG answered: the game's order is kept, noted
    s.handle_text(&start());
    s.handle_text(&m(Body::PreviewRequest { deadline_ms: None }));
    s.handle_text(&leads("gholdengo"));
    let r = reply(s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "played", r#"{"kind":"move","move":"recover","mega":false}"#)));
    assert_eq!(order(&r), Order::Move { mv: "recover".into(), mega: false });
    assert_eq!(s.notes.len(), 1);
    // the next resend without marks repeats the order the game took
    s.handle_text(&start());
    s.handle_text(&m(Body::PreviewRequest { deadline_ms: None }));
    s.handle_text(&leads("gholdengo"));
    assert_eq!(order(&reply(s.handle_text(&request(1, ReqKind::Turn, None, false)))), Order::Move { mv: "recover".into(), mega: false });
}
