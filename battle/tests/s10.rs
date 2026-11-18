//! S10: a later preview fallback (S10-3), and our recorded choice against the rows (S10-4).

mod common;
use battle::json::Json;
use battle::live::{AskKind, Open, Session, Step};
use battle::*;
use cg_engine::sim::data::MoveId;
use common::*;

fn m(body: Body) -> String {
    msg(body).to_json().to_string()
}

fn events(list: &[&str]) -> String {
    let evs = list.iter().map(|j| Event::from_json(&Json::parse(j).unwrap()).unwrap()).collect();
    m(Body::Events { events: evs, fallback: None })
}

fn with(text: &str, k: &str, v: &str) -> String {
    let mut j = Json::parse(text).unwrap();
    j.set(k, Json::parse(v).unwrap());
    j.to_string()
}

fn request(seq: i64, kind: ReqKind, refused: Option<Order>, retry: bool) -> String {
    m(Body::DecisionRequest { seq, kind, deadline_ms: None, refused, retry, screen_offer: None })
}

fn order(s: Step) -> Order {
    match s {
        Step::Reply(j) => Order::from_json(j.get("order").unwrap()).unwrap(),
        x => panic!("{:?}", x),
    }
}

fn switch(mon: &str) -> Order {
    Order::Switch { mon: mon.into() }
}

fn targets(s: &Session) -> Vec<Order> {
    let l = s.legal().unwrap();
    (8..N_ACTIONS).filter_map(|a| order_for(s.battle(), &l, a)).collect()
}

/// battle_start with `team`, CG's picks [0, 1, 2], `fallback` (if any) on the leads' message.
fn session(team: Vec<SetMsg>, fallback: Option<&str>) -> Session {
    let mut s = Session::new();
    s.handle_text(&m(Body::BattleStart { own_team: team, opp_preview: preview() }));
    s.handle_text(&m(Body::PreviewRequest { deadline_ms: None }));
    s.answer_picks(&[0, 1, 2], "t");
    let leads = events(&[r#"{"k":"turn","n":1}"#, r#"{"k":"enter","side":"opp","mon":"garchomp"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    s.handle_text(&match fallback {
        Some(f) => with(&leads, "fallback", f),
        None => leads,
    });
    s
}

fn picks_fallback(p: &str) -> String {
    with(&events(&[]), "fallback", &format!(r#"{{"picks":{},"reason":"corrected"}}"#, p))
}

/// A second preview fallback before any back member entered replaces the picks; a refused switch stays
/// with its Pokémon, and the switch actions follow the new picks.
#[test]
fn second_picks_fallback_before_a_back_member_entered() {
    let mut s = session(team(), Some(r#"{"picks":[0,1,3],"reason":"overruled"}"#));
    assert_eq!(s.battle().picks, [Some(0), Some(1), Some(3)]);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    assert_eq!(targets(&s), vec![switch("garchomp"), switch("primarina")]);
    s.answer(0, "t");
    s.handle_text(&events(&[&mv("own", "gholdengo", "makeitrain"), &mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 0), r#"{"k":"faint","side":"own","mon":"gholdengo"}"#]));
    s.handle_text(&request(2, ReqKind::Replace, None, false));
    s.answer(9, "t");
    // the game refused Primarina (index 2 of the picks held)
    s.handle_text(&request(3, ReqKind::Replace, Some(switch("primarina")), false));
    assert!(s.battle().refused[10]);
    assert_eq!(targets(&s), vec![switch("garchomp")]);
    // the true picks put Primarina second and Golisopod third: the refusal follows Primarina
    s.handle_text(&picks_fallback("[0,3,2]"));
    assert_eq!(s.battle().picks, [Some(0), Some(3), Some(2)]);
    assert!(s.battle().refused[9] && !s.battle().refused[10]);
    assert_eq!(targets(&s), vec![switch("golisopod")]);
    assert_eq!(s.builder.corrections.len(), 1, "{:?}", s.builder.corrections);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
}

/// After a back member entered: picks that hold the lead first and every Pokémon that entered replace the
/// picks; picks that contradict the log are an anomaly and the picks held stay.
#[test]
fn second_picks_fallback_after_a_back_member_entered() {
    let mut s = session(team(), None);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(8, "t");
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"garchomp"}"#, r#"{"k":"turn","n":2}"#]));
    s.handle_text(&picks_fallback("[0,1,4]"));
    assert_eq!(s.battle().picks, [Some(0), Some(1), Some(4)]);
    assert_eq!(s.builder.corrections.len(), 1);
    s.handle_text(&request(2, ReqKind::Turn, None, false));
    assert_eq!(targets(&s), vec![switch("gholdengo"), switch("mamoswine")]);
    // Garchomp entered: picks without it are refused
    s.handle_text(&picks_fallback("[0,4,3]"));
    // the lead was Gholdengo: picks leading with another are refused
    s.handle_text(&picks_fallback("[1,0,4]"));
    // not three team indices: the message's fallback does not read (an error reply, a session note)
    let r = s.handle_text(&picks_fallback("[0,1,9]"));
    assert!(matches!(r, Step::Reply(ref j) if j.get("error").is_some()), "{:?}", r);
    assert_eq!(s.battle().picks, [Some(0), Some(1), Some(4)]);
    assert_eq!(s.builder.anomalies.len(), 2, "{:?}", s.builder.anomalies);
    assert_eq!(s.builder.corrections.len(), 1);
    // the same picks again change nothing
    s.handle_text(&picks_fallback("[0,1,4]"));
    assert_eq!(s.builder.corrections.len(), 1);
}

/// A preview fallback applies before the events of its message: with the first back member's `enter` in
/// the same message, the picks are already right when it enters (no rule-32 correction follows); a wrong
/// one there is corrected by rule 32 at that `enter`.
#[test]
fn picks_fallback_with_rows_in_the_same_message() {
    let rows = events(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"mamoswine"}"#, r#"{"k":"turn","n":2}"#]);
    let mut s = session(team(), None);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(8, "t");
    s.handle_text(&with(&rows, "fallback", r#"{"picks":[0,4,2],"reason":"corrected"}"#));
    assert_eq!(s.battle().picks, [Some(0), Some(4), Some(2)]);
    assert_eq!(s.builder.corrections.len(), 1, "{:?}", s.builder.corrections);
    assert!(s.builder.corrections[0].what.contains("picks fallback"));
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
    // the fallback names Cinderace for the slot Mamoswine took: rule 32 corrects it at the enter
    let mut s = session(team(), None);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(8, "t");
    s.handle_text(&with(&rows, "fallback", r#"{"picks":[0,5,2],"reason":"corrected"}"#));
    assert_eq!(s.battle().picks, [Some(0), Some(4), Some(2)]);
    assert_eq!(s.builder.corrections.len(), 2);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
}

/// Gholdengo with Thunderbolt and Charge, charged on turn 1; turn 2's request is open.
fn charged() -> Session {
    let mut t = team();
    t[0] = set("gholdengo", Some("leftovers"), "goodasgold", ["thunderbolt", "shadowball", "charge", "recover"], 162);
    let mut s = session(t, None);
    s.handle_text(&request(1, ReqKind::Turn, None, false));
    s.answer(2, "t");
    s.handle_text(&events(&[
        &mv("own", "gholdengo", "charge"),
        r#"{"k":"volatile","side":"own","mon":"gholdengo","effect":"type_power_rose","detail":"electric"}"#,
        r#"{"k":"turn","n":2}"#,
    ]));
    assert!(has_charge(&s).is_some());
    s.handle_text(&request(2, ReqKind::Turn, None, false));
    s
}

/// Our Charge: None when gone, else whether it is marked uncertain.
fn has_charge(s: &Session) -> Option<bool> {
    s.battle().side(Side::Own).active.vols.iter().flatten().find(|v| v.cond.id() == "charge").map(|v| v.maybe_gone)
}

const PARALYSED: &str = r#"{"k":"cant","side":"own","mon":"gholdengo","reason":"par"}"#;

/// Rule 22 with a wrong recorded choice: the `cant` row names no move, so nothing corrects it. With the
/// choice unknown (a fallback without an order) Charge is kept, marked uncertain.
#[test]
fn charge_with_a_wrong_or_unknown_choice() {
    // recorded Thunderbolt (a tap the game took): Charge ends when the move is stopped
    let mut s = charged();
    s.answer(0, "t");
    s.handle_text(&events(&[PARALYSED]));
    assert_eq!(has_charge(&s), None);
    // recorded Shadow Ball, but the game took Thunderbolt: Charge wrongly kept, no row tells
    let mut s = charged();
    s.answer(1, "t");
    s.handle_text(&events(&[PARALYSED]));
    assert_eq!(has_charge(&s), Some(false));
    assert!(s.builder.corrections.is_empty());
    // unknown: kept, marked uncertain
    let mut s = charged();
    s.answer(1, "t");
    s.handle_text(&with(&events(&[PARALYSED]), "fallback", r#"{"seq":2,"reason":"tap not confirmed"}"#));
    assert_eq!(has_charge(&s), Some(true));
    // recorded Thunderbolt, the row shows Shadow Ball: corrected, and Charge stays (not an Electric move)
    let mut s = charged();
    s.answer(0, "t");
    s.handle_text(&events(&[&mv("own", "gholdengo", "shadowball")]));
    assert_eq!(s.battle().ctx.own_choice, MoveId::from_id("shadowball"));
    assert_eq!(has_charge(&s), Some(false));
    assert_eq!(s.builder.corrections.len(), 1, "{:?}", s.builder.corrections);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
    // unknown, then the row shows the move: learnt, no correction
    let mut s = charged();
    s.answer(1, "t");
    s.handle_text(&with(&events(&[&mv("own", "gholdengo", "thunderbolt")]), "fallback", r#"{"seq":2,"reason":"tap not confirmed"}"#));
    assert_eq!(s.battle().ctx.own_choice, MoveId::from_id("thunderbolt"));
    assert_eq!(has_charge(&s), None);
    assert!(s.builder.corrections.is_empty());
}

/// Rule 18 with a wrong recorded choice: a `replace` request before the rows takes the Pokémon our
/// recorded switch named as the one on the field; no row has come yet, so nothing corrects it there. The
/// rows that follow do (a correction). Unknown takes the active one as on the field.
#[test]
fn replace_before_rows_with_a_wrong_or_unknown_choice() {
    let faint = [&mv("own", "gholdengo", "makeitrain") as &str, &mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 0), r#"{"k":"faint","side":"own","mon":"gholdengo"}"#];
    // recorded switch to Golisopod (played), the game took Make It Rain and Gholdengo fainted
    let mut s = session(team(), None);
    let q = with(&request(1, ReqKind::Turn, None, false), "played", r#"{"kind":"switch","mon":"golisopod"}"#);
    s.handle_text(&q);
    assert_eq!(s.handle_text(&request(2, ReqKind::Replace, None, false)), Step::Ask(Open { seq: 2, kind: AskKind::Replace }));
    // Golisopod is taken to be on the field: not offered; Gholdengo (fainted, unknown yet) is
    assert_eq!(targets(&s), vec![switch("gholdengo"), switch("garchomp")]);
    s.answer(9, "t");
    s.handle_text(&events(&faint));
    assert_eq!(s.builder.corrections.len(), 1, "{:?}", s.builder.corrections);
    assert_eq!(s.battle().ctx.own_switch, None);
    // unknown: the active one is taken to be on the field, which is right when the game took a move
    let mut s = session(team(), None);
    s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "replay", "true"));
    s.handle_text(&request(2, ReqKind::Replace, None, false));
    assert_eq!(targets(&s), vec![switch("garchomp"), switch("golisopod")]);
    // a recorded switch to another Pokémon than the rows show is corrected
    let mut s = session(team(), None);
    s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "played", r#"{"kind":"switch","mon":"golisopod"}"#));
    s.handle_text(&events(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"garchomp"}"#]));
    assert_eq!(s.battle().ctx.own_switch, Some(1));
    assert_eq!(s.builder.corrections.len(), 1, "{:?}", s.builder.corrections);
    assert!(s.builder.anomalies.is_empty(), "{:?}", s.builder.anomalies);
}

/// A `retry` repeats the last order CG holds: a wrong `played` is repeated; unknown asks the Player.
#[test]
fn retry_after_a_wrong_or_unknown_choice() {
    let mut s = session(team(), None);
    s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "played", r#"{"kind":"move","move":"recover"}"#));
    assert_eq!(order(s.handle_text(&request(2, ReqKind::Turn, None, true))), Order::Move { mv: "recover".into(), mega: false });
    let mut s = session(team(), None);
    s.handle_text(&with(&request(1, ReqKind::Turn, None, false), "replay", "true"));
    assert_eq!(s.handle_text(&request(2, ReqKind::Turn, None, true)), Step::Ask(Open { seq: 2, kind: AskKind::Turn }));
}
