//! The controller's questions on LIVE 0.5 (LV5Q-n, LV5-8, LV5-9): the event shapes the phone can send, as
//! scripted streams, and what the Builder derives from them.

mod common;
use battle::*;
use common::*;
use cg_engine::sim::data::*;

fn req(t: &mut T, seq: i64, kind: ReqKind, retry: bool) {
    t.input(Input::Msg(msg(Body::DecisionRequest { seq, kind, deadline_ms: None, refused: None, retry, screen_offer: None })));
}

fn order_move(t: &mut T, m: &str) {
    t.input(Input::Order(Order::Move { mv: m.into(), mega: false }));
}

fn own_pp_left(t: &T, slot: u8, i: usize) -> Option<u8> {
    t.bt().pp_left(Who { side: Side::Own, slot })[i]
}

/// LV5Q-1: a fallback with no order: our choice this turn is not known.
#[test]
fn fallback_without_an_order() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    req(&mut t, 1, ReqKind::Turn, false);
    order_move(&mut t, "shadowball");
    assert_eq!(t.bt().ctx.own_choice, MoveId::from_id("shadowball"));
    let f = Fallback { seq: Some(1), order: None, reason: Some("deadline".into()), picks: None };
    t.input(Input::Msg(msg(Body::Events { events: vec![], fallback: Some(vec![f]) })));
    assert_eq!(t.bt().ctx.own_choice, None);
    // with an order, the fallback's move is ours
    let f = Fallback { seq: Some(1), order: Some(Order::Move { mv: "makeitrain".into(), mega: false }), reason: None, picks: None };
    t.input(Input::Msg(msg(Body::Events { events: vec![], fallback: Some(vec![f]) })));
    assert_eq!(t.bt().ctx.own_choice, MoveId::from_id("makeitrain"));
    // the turn's rows still show our move: PP and the Choice Scarf lock come from them
    t.es(&[&mv("own", "gholdengo", "makeitrain")]);
    assert_eq!(t.own_active().choice_lock, MoveId::from_id("makeitrain"));
    assert_eq!(own_pp_left(&t, 0, 0), Some(MoveId::from_id("makeitrain").unwrap().data().max_pp - 1));
    t.assert_clean();
}

fn damp_team() -> Vec<SetMsg> {
    let mut v = team();
    v[0] = set("gholdengo", None, "damp", ["makeitrain", "recover", "nastyplot", "shadowball"], 162);
    v[1] = set("farigiraf", None, "armortail", ["psychic", "hypervoice", "calmmind", "protect"], 180);
    v
}

/// LV5Q-2: "X cannot use MOVE!" after the holder's Ability row names the Pokémon that could not move, with
/// the move id as `reason`; a move id that is also a condition's (Taunt) is still the move.
#[test]
fn ability_stops_a_move() {
    let mut t = T::from_team(damp_team(), [0, 1, 2], "gholdengo", "gengar");
    t.turn(1);
    t.es(&[r#"{"k":"ability","side":"own","mon":"gholdengo","ability":"damp"}"#, r#"{"k":"cant","side":"opp","mon":"gengar","reason":"explosion"}"#]);
    let now = t.bt().side(Side::Opp).now;
    assert_eq!(now.act, Act::Cant);
    assert_eq!(now.cant, Some(CantReason::Move(MoveId::from_id("explosion").unwrap())));
    assert_eq!(t.bt().side(Side::Own).now.act, Act::None);
    t.assert_clean();
    let mut t = T::from_team(damp_team(), [1, 0, 2], "farigiraf", "gengar");
    t.turn(1);
    t.es(&[r#"{"k":"ability","side":"own","mon":"farigiraf","ability":"armortail"}"#, r#"{"k":"cant","side":"opp","mon":"gengar","reason":"taunt"}"#]);
    assert_eq!(t.bt().side(Side::Opp).now.cant, Some(CantReason::Move(MoveId::from_id("taunt").unwrap())));
    // the opponent's ability is learnt from its row
    let mut t = T::new("gholdengo", "gengar");
    t.turn(1);
    t.es(&[r#"{"k":"ability","side":"opp","mon":"gengar","ability":"damp"}"#, r#"{"k":"cant","side":"own","mon":"gholdengo","reason":"explosion"}"#]);
    assert_eq!(t.opp(1).ability, AbilityId::from_id("damp"));
    assert_eq!(t.bt().side(Side::Own).now.act, Act::Cant);
}

/// LV5Q-2, LV5-9: the nameless "no PP left" row after the move row: not a use (no Choice lock, no last
/// move), and the move has no PP left.
#[test]
fn no_pp_left_after_the_move_row() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"cant","reason":"nopp"}"#]);
    let a = t.own_active();
    assert_eq!(a.choice_lock, None);
    assert_eq!(a.last_move, None);
    assert_eq!(own_pp_left(&t, 0, 1), Some(0));
    let now = t.bt().side(Side::Own).now;
    assert_eq!((now.act, now.cant), (Act::Cant, Some(CantReason::Other)));
    assert_eq!(now.order, 1);
    t.assert_clean();
    // the opponent's: its move is shown and has no PP left
    t.es(&[&mv("opp", "garchomp", "outrage"), r#"{"k":"cant","reason":"nopp"}"#]);
    let o = t.bt().side(Side::Opp);
    assert_eq!(o.active.lock, Lock::None);
    assert_eq!(t.bt().pp_left(Who { side: Side::Opp, slot: 0 })[0], Some(0));
    assert_eq!(o.now.order, 2);
    // the 0.5 form (a mon, no move row) still reads
    t.turn(2);
    t.e(r#"{"k":"cant","side":"opp","mon":"garchomp","reason":"nopp"}"#);
    t.assert_clean();
    // nameless with no move before it: an anomaly
    t.turn(3);
    t.e(r#"{"k":"cant","reason":"nopp"}"#);
    assert_eq!(t.anomalies(), ["nameless row with no move before it"]);
}

/// LV5Q-3: Future Sight's landing is `once futuresight` naming the target, and the cause of the HP row
/// after it: not the user's move, at the end of the turn.
#[test]
fn future_sight_lands_as_once() {
    let mut t = T::new("gholdengo", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "futuresight"), r#"{"k":"once","effect":"futuresight_start","side":"opp","mon":"gengar"}"#]);
    t.turn(2);
    t.turn(3);
    t.es(&[&mv("opp", "gengar", "shadowball"), &hp_own("gholdengo", 162, 150)]);
    let dealt = t.bt().side(Side::Opp).now.damage;
    t.es(&[r#"{"k":"once","effect":"futuresight","side":"own","mon":"gholdengo"}"#, &hp_own("gholdengo", 150, 100)]);
    assert_eq!(t.bt().side(Side::Opp).now.damage, dealt);
    assert_eq!(t.bt().side(Side::Opp).now.mv, MoveId::from_id("shadowball"));
    assert!(t.bt().ctx.residual);
    t.assert_clean();
}

/// LV5Q-4: Trick is `once trick` naming the user, then one `received` row (no `of`) per Pokémon that got an
/// item; one that got none holds nothing.
#[test]
fn trick_exchanges_items() {
    let mut t = T::new("gholdengo", "gengar");
    t.turn(1);
    t.es(&[
        &mv("opp", "gengar", "trick"),
        r#"{"k":"once","effect":"trick","side":"opp","mon":"gengar"}"#,
        r#"{"k":"item","side":"own","mon":"gholdengo","item":"lifeorb","what":"received"}"#,
        r#"{"k":"item","side":"opp","mon":"gengar","item":"choicescarf","what":"received"}"#,
    ]);
    assert_eq!(t.own(0).item, ItemState::Held(ItemId::from_id("lifeorb").unwrap()));
    assert_eq!(t.opp(1).item, ItemState::Held(ItemId::from_id("choicescarf").unwrap()));
    assert_eq!(t.own_active().choice_lock, None);
    t.assert_clean();
    // the opponent held nothing: we receive nothing and hold nothing
    let mut t = T::new("gholdengo", "gengar");
    t.turn(1);
    t.es(&[
        &mv("opp", "gengar", "trick"),
        r#"{"k":"once","effect":"trick","side":"opp","mon":"gengar"}"#,
        r#"{"k":"item","side":"opp","mon":"gengar","item":"choicescarf","what":"received"}"#,
    ]);
    assert_eq!(t.own(0).item, ItemState::Gone(None));
    assert_eq!(t.opp(1).item, ItemState::Held(ItemId::from_id("choicescarf").unwrap()));
    t.assert_clean();
}

/// LV5Q-5: "X stole and ate its target's BERRY!" is `item removed` with `of` = the eater and no mon: the
/// Berry was the other Pokémon's; the eater's Held Item row for it is not the eater's own item.
#[test]
fn bug_bite_names_only_the_eater() {
    let mut t = T::new("cinderace", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "bugbite"),
        &hp_own("cinderace", 155, 120),
        r#"{"k":"item","item":"leppaberry","what":"removed","of":{"side":"opp","mon":"garchomp"}}"#,
        r#"{"k":"item","side":"opp","mon":"garchomp","item":"leppaberry","what":"activated"}"#,
    ]);
    assert_eq!(t.own(5).item, ItemState::Gone(ItemId::from_id("leppaberry")));
    assert_eq!(t.opp(0).item, ItemState::Unknown);
    t.assert_clean();
    // ours eating theirs: their Sitrus Berry is gone, our Leppa Berry stays
    let mut t = T::new("cinderace", "garchomp");
    t.turn(1);
    t.es(&[
        r#"{"k":"item","item":"sitrusberry","what":"removed","of":{"side":"own","mon":"cinderace"}}"#,
        r#"{"k":"item","side":"own","mon":"cinderace","item":"sitrusberry","what":"activated"}"#,
        &hp_own("cinderace", 100, 138),
    ]);
    assert_eq!(t.opp(0).item, ItemState::Gone(ItemId::from_id("sitrusberry")));
    assert_eq!(t.own(5).item, ItemState::Held(ItemId::from_id("leppaberry").unwrap()));
}

/// LV5Q-6: a repeated replacement screen sent as `retry` is the same request: nothing refused.
#[test]
fn replace_retry_changes_nothing() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 0), r#"{"k":"faint","side":"own","mon":"gholdengo"}"#]);
    req(&mut t, 2, ReqKind::Replace, false);
    let l = legal(t.bt(), ReqKind::Replace);
    req(&mut t, 3, ReqKind::Replace, true);
    assert_eq!(legal(t.bt(), ReqKind::Replace), l);
    assert_eq!(t.b.retries, 1);
    t.assert_clean();
}

/// LV5Q-7: the replace request before the rows of the turn it ends (as the phone sends it) gives the same
/// legal set, and the same Battle at the next request, as after them.
#[test]
fn replace_before_its_rows() {
    let rows: Vec<String> = vec![
        vol("opp", "garchomp", "endure"),
        mv("own", "gholdengo", "shadowball"),
        hp_opp("garchomp", 100, 60),
        mv("opp", "garchomp", "earthquake"),
        hp_own("gholdengo", 162, 0),
        r#"{"k":"faint","side":"own","mon":"gholdengo"}"#.into(),
        r#"{"k":"weather","weather":"sandstorm","what":"end"}"#.into(),
    ];
    let after: Vec<String> = vec![r#"{"k":"enter","side":"own","mon":"garchomp"}"#.into(), r#"{"k":"weather","weather":"raindance","what":"start"}"#.into()];
    let run = |live: bool| {
        let mut t = T::new("gholdengo", "garchomp");
        t.turn(1);
        t.e(r#"{"k":"weather","weather":"sandstorm","what":"start"}"#);
        req(&mut t, 1, ReqKind::Turn, false);
        order_move(&mut t, "shadowball");
        let replace = |t: &mut T| {
            req(t, 2, ReqKind::Replace, false);
            let l = legal(t.bt(), ReqKind::Replace);
            t.input(Input::Order(Order::Switch { mon: "garchomp".into() }));
            l
        };
        let l = if live { replace(&mut t) } else { Legal::default() };
        for r in &rows {
            t.e(r);
        }
        let l = if live { l } else { replace(&mut t) };
        for r in &after {
            t.e(r);
        }
        t.turn(2);
        req(&mut t, 3, ReqKind::Turn, false);
        t.assert_clean();
        (l, t.b.battle)
    };
    let (lp, bp) = run(false);
    let (ll, bl) = run(true);
    assert_eq!(lp, ll);
    assert_eq!(lp.mask[8..], [false, true, true]);
    assert_eq!(bp, bl);
    // the rain set by the replacement counts from turn 2 either way
    assert_eq!(bl.field.weather.map(|w| w.start), Some(2));
}

/// LV5Q-7: we switched this turn and the Pokémon we switched to fainted; the replace request comes before
/// the rows that show the switch: it replaces the Pokémon we switched to, and the one that left may come back.
#[test]
fn replace_before_its_rows_after_our_switch() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    req(&mut t, 1, ReqKind::Turn, false);
    t.input(Input::Order(Order::Switch { mon: "garchomp".into() }));
    req(&mut t, 2, ReqKind::Replace, false);
    let l = legal(t.bt(), ReqKind::Replace);
    assert_eq!(l.mask[8..], [true, false, true]);
    t.input(Input::Order(Order::Switch { mon: "gholdengo".into() }));
    // the answer changes nothing a retry of the same screen would be asked with
    assert_eq!(legal(t.bt(), ReqKind::Replace), l);
    t.es(&[
        r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("garchomp", 183, 0),
        r#"{"k":"faint","side":"own","mon":"garchomp"}"#,
        r#"{"k":"enter","side":"own","mon":"gholdengo"}"#,
    ]);
    t.turn(2);
    t.assert_clean();
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 0 }));
    // after the rows, the same request is asked as before
    assert_eq!(legal(t.bt(), ReqKind::Turn).mask[8..], [false, false, true]);
}

/// LV5-8: a called move with no `called_by` is known from the move row right before it.
#[test]
fn called_move_without_called_by() {
    let mut t = T::new("cinderace", "garchomp");
    t.turn(1);
    t.e(r#"{"k":"status","side":"own","mon":"cinderace","status":"slp"}"#);
    t.turn(2);
    t.es(&[&mv("own", "cinderace", "sleeptalk"), &mv("own", "cinderace", "pyroball")]);
    let now = t.bt().side(Side::Own).now;
    assert_eq!(now.mv, MoveId::from_id("sleeptalk"));
    assert_eq!(now.called, MoveId::from_id("pyroball"));
    let pp = t.bt().pp_left(Who { side: Side::Own, slot: 0 });
    assert_eq!(pp[0], Some(MoveId::from_id("pyroball").unwrap().data().max_pp));
    t.assert_clean();
}

fn req_refused(t: &mut T, seq: i64, mon: &str) {
    let o = Order::Switch { mon: mon.into() };
    t.input(Input::Msg(msg(Body::Events { events: vec![Event::Refused { order: o.clone() }], fallback: None })));
    t.input(Input::Msg(msg(Body::DecisionRequest { seq, kind: ReqKind::Replace, deadline_ms: None, refused: Some(o), retry: false, screen_offer: None })));
}

fn answer(t: &mut T, mon: &str) {
    t.input(Input::Order(Order::Switch { mon: mon.into() }));
}

/// LVA-2: two replace requests in one turn before any row (our U-turn, then the Pokémon it brought in
/// faints): the second replaces the one our first answer sent in, and the U-turn user may come back. A
/// retry of the first screen keeps its set; a refusal takes the refused one out and leaves the answer unsent.
#[test]
fn second_replace_before_any_row() {
    // picks: golisopod, gholdengo, garchomp
    let mut t = T::new("golisopod", "garchomp");
    t.turn(1);
    req(&mut t, 1, ReqKind::Turn, false);
    order_move(&mut t, "uturn");
    assert!(t.bt().ctx.unread);
    req(&mut t, 2, ReqKind::Replace, false);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, true, true]);
    answer(&mut t, "gholdengo");
    req(&mut t, 2, ReqKind::Replace, true);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, true, true], "a retry is the same screen");
    answer(&mut t, "gholdengo");
    req(&mut t, 3, ReqKind::Replace, false);
    assert_eq!(t.bt().ctx.sent_in, Some(0));
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [true, false, true]);
    // a refusal: the game did not take garchomp; the answer that follows is the one sent in
    answer(&mut t, "garchomp");
    req_refused(&mut t, 4, "garchomp");
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [true, false, false]);
    assert_eq!(t.bt().ctx.replace_answer, None);
    answer(&mut t, "golisopod");
    t.es(&[
        &mv("own", "golisopod", "uturn"),
        &hp_opp("garchomp", 100, 80),
        r#"{"k":"withdraw","side":"own","mon":"golisopod","pivot":true}"#,
        r#"{"k":"enter","side":"own","mon":"gholdengo"}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("gholdengo", 162, 0),
        r#"{"k":"faint","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"golisopod"}"#,
    ]);
    assert!(!t.bt().ctx.unread && t.bt().ctx.sent_in.is_none());
    t.turn(2);
    t.assert_clean();
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 2 }));
}

/// LVA-4: a replace request before the rows in a turn we chose Revival Blessing: it may be the revival's
/// choice or a plain replacement, so both the fainted and the healthy ones are offered and the game
/// refuses the wrong kind. The revival's answer does not send anyone in.
#[test]
fn revival_blessing_before_its_rows() {
    let mut team = team();
    team[0].moves[3] = "revivalblessing".into();
    let mut t = T::from_team(team, [0, 1, 2], "gholdengo", "garchomp");
    t.turn(1);
    req(&mut t, 1, ReqKind::Turn, false);
    t.input(Input::Order(Order::Switch { mon: "garchomp".into() }));
    t.es(&[
        r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("garchomp", 183, 0),
        r#"{"k":"faint","side":"own","mon":"garchomp"}"#,
    ]);
    req(&mut t, 2, ReqKind::Replace, false);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [true, false, true], "rows read: the old rule");
    answer(&mut t, "gholdengo");
    t.e(r#"{"k":"enter","side":"own","mon":"gholdengo"}"#);
    t.turn(2);
    req(&mut t, 3, ReqKind::Turn, false);
    order_move(&mut t, "revivalblessing");
    req(&mut t, 4, ReqKind::Replace, false);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, true, true]);
    answer(&mut t, "golisopod");
    req_refused(&mut t, 5, "golisopod");
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, true, false]);
    answer(&mut t, "garchomp");
    assert_eq!(t.bt().ctx.replace_answer, None, "a revival sends no one in");
    // our Revival Blessing user then faints: a new replace request; refusals of the revival are forgotten
    req(&mut t, 6, ReqKind::Replace, false);
    assert_eq!(t.bt().ctx.sent_in, None);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, true, true]);
    answer(&mut t, "golisopod");
    t.es(&[
        &mv("own", "gholdengo", "revivalblessing"),
        r#"{"k":"revive","side":"own","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("gholdengo", 162, 0),
        r#"{"k":"faint","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"golisopod"}"#,
    ]);
    t.turn(3);
    t.assert_clean();
    assert!(!t.own(1).fainted && t.own(0).fainted);
    assert_eq!(legal(t.bt(), ReqKind::Turn).mask[8..], [false, true, false]);
}

/// An unseen Red Card drag, then a refused replacement, then a new replace request in the same turn: the
/// Pokémon refused at the first screen (it was the one on the field) is the answer at the second, and the
/// legal set is never empty.
#[test]
fn refusal_holds_for_one_replace_screen() {
    // picks: gholdengo, garchomp, golisopod (Emergency Exit); garchomp faints on turn 1
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    req(&mut t, 1, ReqKind::Turn, false);
    t.input(Input::Order(Order::Switch { mon: "garchomp".into() }));
    t.es(&[
        r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("garchomp", 183, 0),
        r#"{"k":"faint","side":"own","mon":"garchomp"}"#,
    ]);
    req(&mut t, 2, ReqKind::Replace, false);
    answer(&mut t, "gholdengo");
    t.e(r#"{"k":"enter","side":"own","mon":"gholdengo"}"#);
    t.turn(2);
    req(&mut t, 3, ReqKind::Turn, false);
    order_move(&mut t, "makeitrain");
    // unseen: Red Card drags golisopod in, its Emergency Exit asks for a replacement
    req(&mut t, 4, ReqKind::Replace, false);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, false, true]);
    answer(&mut t, "golisopod");
    req_refused(&mut t, 5, "golisopod");
    // nothing else: the one taken to be on the field was dragged out
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [true, false, false]);
    answer(&mut t, "gholdengo");
    // a retry of the same screen keeps its refusal
    req(&mut t, 5, ReqKind::Replace, true);
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [true, false, false]);
    answer(&mut t, "gholdengo");
    // gholdengo faints to Stealth Rock: a new screen, where golisopod is the answer
    req(&mut t, 6, ReqKind::Replace, false);
    assert_eq!(t.bt().ctx.sent_in, Some(0));
    assert_eq!(legal(t.bt(), ReqKind::Replace).mask[8..], [false, false, true]);
    // even with it refused too, the set is never empty
    answer(&mut t, "golisopod");
    req_refused(&mut t, 7, "golisopod");
    assert!(legal(t.bt(), ReqKind::Replace).mask[8..].iter().any(|x| *x));
    answer(&mut t, "golisopod");
    t.es(&[
        &mv("own", "gholdengo", "makeitrain"),
        &hp_opp("garchomp", 100, 40),
        r#"{"k":"item","side":"opp","mon":"garchomp","item":"redcard","what":"activated"}"#,
        r#"{"k":"enter","side":"own","mon":"golisopod","forced":true}"#,
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("golisopod", 150, 60),
        r#"{"k":"ability","side":"own","mon":"golisopod","ability":"emergencyexit"}"#,
        r#"{"k":"enter","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"side","side":"own","effect":"stealthrock","what":"start"}"#,
        r#"{"k":"cause","effect":"stealthrock","side":"own","mon":"gholdengo"}"#,
        &hp_own("gholdengo", 162, 0),
        r#"{"k":"faint","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"golisopod"}"#,
    ]);
    t.turn(3);
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 2 }));
    assert!(!t.bt().ctx.unread);
}
