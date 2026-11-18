//! One hand-written event list per derivation rule of LIVE.md section 5.

mod common;
use battle::*;
use common::*;
use cg_engine::sim::data::*;

fn id(m: &str) -> MoveId {
    MoveId::from_id(m).unwrap()
}

#[test]
fn start_fills_own_sets_and_preview() {
    let t = T::new("gholdengo", "garchomp");
    t.assert_clean();
    let b = t.bt();
    assert_eq!(b.n_own, 6);
    assert_eq!(b.picks, [Some(0), Some(1), Some(2)]);
    assert_eq!(t.own(0).item, ItemState::Held(ItemId::from_id("choicescarf").unwrap()));
    assert_eq!(t.own(0).hp, 100.0);
    assert!(t.own(0).was_lead && t.own(0).seen);
    // the opponent: preview only, item and ability unknown, moves unknown
    let g = t.opp(0);
    assert_eq!(g.species, SpeciesId::from_id("garchomp"));
    assert_eq!(g.item, ItemState::Unknown);
    assert_eq!(g.ability, None);
    assert_eq!(g.moves, [None; 4]);
    assert!(t.opp(1).species.is_none() && t.opp(1).preview == SpeciesId::from_id("gengar"));
    assert_eq!(b.active(Side::Opp), Some(Who { side: Side::Opp, slot: 0 }));
}

#[test]
fn rule1_nameless_rows_belong_to_the_last_move() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"result","how":"super"}"#]);
    t.es(&[&mv("opp", "garchomp", "earthquake"), r#"{"k":"result","how":"immune","side":"own","mon":"gholdengo"}"#]);
    t.turn(2);
    t.es(&[&mv("opp", "garchomp", "scaleshot"), r#"{"k":"hits","n":4}"#]);
    t.es(&[&mv("own", "gholdengo", "nastyplot"), r#"{"k":"fail"}"#]);
    t.assert_clean();
    let own_last = t.bt().side(Side::Own).last;
    assert_eq!(own_last.eff, Some(Eff::Super));
    assert_eq!(t.bt().side(Side::Opp).last.eff, Some(Eff::Immune));
    assert_eq!(t.bt().side(Side::Opp).now.hits, 4);
    assert!(t.bt().side(Side::Own).now.failed);
    assert!(!t.bt().side(Side::Opp).now.failed);
    // a nameless row with no move before it is an anomaly
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(r#"{"k":"fail"}"#);
    assert_eq!(t.b.anomalies.len(), 1);
}

#[test]
fn rule1_confusion_self_hit_is_the_last_named() {
    // "X is confused!" then the nameless self-hit, after the other side's move
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("own", "gholdengo", "shadowball"),
        &hp_opp("garchomp", 100, 60),
        r#"{"k":"volatile_hit","effect":"confusion","side":"opp","mon":"garchomp"}"#,
        r#"{"k":"volatile_hit","effect":"confusion"}"#,
        &hp_opp("garchomp", 60, 50),
    ]);
    let opp = t.bt().side(Side::Opp).now;
    assert!(opp.confusion_hit && opp.mon == Some(0));
    assert!(!t.bt().side(Side::Own).now.confusion_hit);
    // a nameless self-hit right after an own HP row is ours
    t.turn(2);
    t.es(&[&hp_own("gholdengo", 162, 150), r#"{"k":"volatile_hit","effect":"confusion"}"#]);
    assert!(t.bt().side(Side::Own).now.confusion_hit);
    t.assert_clean();
}

#[test]
fn silent_regenerator_on_switch_out() {
    let team = vec![
        set("gholdengo", None, "goodasgold", ["makeitrain", "shadowball", "nastyplot", "recover"], 162),
        set("slowbro", None, "regenerator", ["scald", "slackoff", "psychic", "calmmind"], 201),
        set("golisopod", None, "emergencyexit", ["firstimpression", "liquidation", "knockoff", "uturn"], 150),
    ];
    let mut t = T::from_team(team, [1, 0, 2], "slowbro", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "earthquake"), &hp_own("slowbro", 201, 100)]);
    t.turn(2);
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"slowbro"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    // 100 + floor(201 / 3) = 167 of 201
    assert!((t.own(1).hp - 167.0 * 100.0 / 201.0).abs() < 0.01);
    // the opponent's only once shown
    t.es(&[&mv("own", "gholdengo", "shadowball"), &hp_opp("garchomp", 100, 40)]);
    t.es(&[r#"{"k":"withdraw","side":"opp","mon":"garchomp"}"#, r#"{"k":"enter","side":"opp","mon":"gengar"}"#]);
    assert_eq!(t.opp(0).hp, 40.0);
    t.turn(3);
    t.es(&[
        r#"{"k":"withdraw","side":"opp","mon":"gengar"}"#,
        r#"{"k":"enter","side":"opp","mon":"corviknight"}"#,
        r#"{"k":"ability","side":"opp","mon":"corviknight","ability":"regenerator"}"#,
        r#"{"k":"status","side":"opp","mon":"corviknight","status":"brn"}"#,
        &hp_opp("corviknight", 100, 70),
        r#"{"k":"withdraw","side":"opp","mon":"corviknight"}"#,
        r#"{"k":"enter","side":"opp","mon":"garchomp"}"#,
    ]);
    assert!((t.opp(2).hp - 100.0).abs() < 0.01);
    t.assert_clean();
}

#[test]
fn rule1_protect_and_miss() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "protect"), &mv("own", "gholdengo", "makeitrain"), r#"{"k":"protect","side":"opp","mon":"garchomp"}"#]);
    assert!(t.bt().side(Side::Own).now.protected);
    t.turn(2);
    t.es(&[&mv("opp", "garchomp", "stoneedge"), r#"{"k":"miss","side":"own","mon":"gholdengo"}"#]);
    assert!(t.bt().side(Side::Opp).now.missed);
    t.assert_clean();
}

#[test]
fn rule2_cause_of_hp_change() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    // damage from a move: no cause row, so the move took it
    t.es(&[&mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 81)]);
    assert_eq!(t.bt().side(Side::Opp).now.damage, 50.0);
    assert_eq!(t.own(0).hp, 50.0);
    assert_eq!(t.own(0).biggest_hit, 50.0);
    // a cause row before the HP row: not the move's
    t.es(&[r#"{"k":"cause","effect":"brn","side":"own","mon":"gholdengo"}"#, &hp_own("gholdengo", 81, 71)]);
    assert_eq!(t.bt().side(Side::Opp).now.damage, 50.0);
    // a cause row right after the HP row takes it off the move, and reveals the item on its holder
    t.es(&[&mv("own", "gholdengo", "makeitrain"), &hp_opp("garchomp", 100, 60), &hp_opp("garchomp", 60, 50)]);
    t.e(r#"{"k":"cause","effect":"lifeorb","side":"opp","mon":"garchomp"}"#);
    assert_eq!(t.bt().side(Side::Own).now.damage, 40.0);
    assert_eq!(t.opp(0).biggest_hit, 40.0);
    assert_eq!(t.opp(0).item, ItemState::Held(ItemId::from_id("lifeorb").unwrap()));
    // `of` names the holder of the item that caused it
    t.e(r#"{"k":"cause","effect":"rockyhelmet","side":"own","mon":"gholdengo","of":{"side":"opp","mon":"garchomp"}}"#);
    assert_eq!(t.opp(0).item, ItemState::Held(ItemId::from_id("rockyhelmet").unwrap()));
    t.assert_clean();
}

#[test]
fn rule3_setter_of_weather_terrain_side() {
    let mut t = T::new("gholdengo", "garchomp");
    t.es(&[r#"{"k":"ability","side":"opp","mon":"garchomp","ability":"sandstream"}"#, r#"{"k":"weather","weather":"sandstorm","what":"start"}"#]);
    let w = t.bt().field.weather.unwrap();
    assert_eq!(w.cond.id(), "sandstorm");
    assert_eq!(w.by, Some(Who { side: Side::Opp, slot: 0 }));
    assert_eq!(w.start, START_TURN);
    t.turn(1);
    t.es(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"side","side":"opp","effect":"stealthrock","what":"start"}"#]);
    let c = t.bt().side(Side::Opp).conds[SIDE_CONDS.iter().position(|x| *x == "stealthrock").unwrap()].unwrap();
    assert_eq!(c.by, Some(Who { side: Side::Own, slot: 0 }));
    assert_eq!(c.start, 1);
    t.es(&[&mv("opp", "garchomp", "trickroom"), r#"{"k":"field","effect":"trickroom","what":"start"}"#]);
    assert_eq!(t.bt().field.rooms[0].unwrap().by, Some(Who { side: Side::Opp, slot: 0 }));
    t.assert_clean();
}

#[test]
fn rule4_stat_limit() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "swordsdance"), r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk"],"by":2}"#]);
    assert_eq!(t.opp_active().boosts[0], 2);
    t.e(r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk"],"by":12}"#);
    assert_eq!(t.opp_active().boosts[0], 6);
    t.e(r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["spe"],"by":-1}"#);
    t.es(&[&mv("own", "gholdengo", "nastyplot"), r#"{"k":"boost_limit","side":"own","mon":"gholdengo","stats":["spa"],"dir":"up"}"#]);
    assert_eq!(t.own_active().boosts[2], 6);
    t.assert_clean();
    // with no move or ability before it, the row cannot be tied: an anomaly, the stage still set
    let mut t = T::new("gholdengo", "garchomp");
    t.e(r#"{"k":"boost_limit","side":"own","mon":"gholdengo","stats":["def"],"dir":"down"}"#);
    assert_eq!(t.own_active().boosts[1], -6);
    assert_eq!(t.b.anomalies.len(), 1);
}

#[test]
fn rule5_field_turns_counted_from_start() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "sunnyday"), r#"{"k":"weather","weather":"sunnyday","what":"start"}"#]);
    t.turn(2);
    t.turn(3);
    assert_eq!(t.bt().elapsed(t.bt().field.weather.unwrap().start), 2);
    // a new weather replaces the old and restarts the count
    t.es(&[&mv("own", "gholdengo", "shadowball"), r#"{"k":"weather","weather":"raindance","what":"start"}"#]);
    assert_eq!(t.bt().field.weather.unwrap().cond.id(), "raindance");
    assert_eq!(t.bt().field.weather.unwrap().start, 3);
    t.e(r#"{"k":"weather","weather":"raindance","what":"end"}"#);
    assert!(t.bt().field.weather.is_none());
    // terrain, a room, a screen and layered hazards
    t.es(&[
        r#"{"k":"field","effect":"grassyterrain","what":"start"}"#,
        r#"{"k":"field","effect":"gravity","what":"start"}"#,
        r#"{"k":"side","side":"opp","effect":"reflect","what":"start"}"#,
        r#"{"k":"side","side":"own","effect":"spikes","what":"start"}"#,
        r#"{"k":"side","side":"own","effect":"spikes","what":"start"}"#,
    ]);
    assert_eq!(t.bt().field.terrain.unwrap().cond.id(), "grassyterrain");
    assert!(t.bt().field.rooms[R_GRAVITY].is_some());
    let spikes = SIDE_CONDS.iter().position(|x| *x == "spikes").unwrap();
    assert_eq!(t.bt().side(Side::Own).conds[spikes].unwrap().layers, 2);
    t.turn(4);
    t.es(&[r#"{"k":"side","side":"opp","effect":"reflect","what":"end"}"#, r#"{"k":"field","effect":"grassyterrain","what":"end"}"#]);
    assert!(t.bt().side(Side::Opp).conds[0].is_none());
    assert!(t.bt().field.terrain.is_none());
    t.assert_clean();
    // an end row for something that was not up is an anomaly
    t.e(r#"{"k":"weather","weather":"snowscape","what":"end"}"#);
    assert_eq!(t.b.anomalies.len(), 1);
}

#[test]
fn rule6_pp_counted_with_pressure() {
    let mut t = T::new("gholdengo", "corviknight");
    let ball = id("shadowball");
    let max = ball.data().max_pp;
    t.turn(1);
    t.e(&mv("own", "gholdengo", "shadowball"));
    assert_eq!(t.own(0).pp_used[1], 1);
    // an opposing Pressure shown: two per use on a move that targets it
    t.e(r#"{"k":"ability","side":"opp","mon":"corviknight","ability":"pressure"}"#);
    t.turn(2);
    t.e(&mv("own", "gholdengo", "shadowball"));
    assert_eq!(t.own(0).pp_used[1], 3);
    // a move on the user itself pays one
    t.turn(3);
    t.e(&mv("own", "gholdengo", "nastyplot"));
    assert_eq!(t.own(0).pp_used[2], 1);
    assert_eq!(t.bt().pp_left(Who { side: Side::Own, slot: 0 })[1], Some(max - 3));
    // the opponent's moves are counted as they are seen
    t.e(&mv("opp", "corviknight", "bravebird"));
    t.turn(4);
    t.e(&mv("opp", "corviknight", "roost"));
    t.e(&mv("opp", "corviknight", "bravebird"));
    let c = t.opp(2);
    assert_eq!(c.moves[0], Some(id("bravebird")));
    assert_eq!(c.moves[1], Some(id("roost")));
    assert_eq!(c.pp_used, [2, 1, 0, 0]);
    assert_eq!(t.bt().pp_left(Who { side: Side::Opp, slot: 2 })[2], None);
    t.assert_clean();
}

#[test]
fn rule6_pp_of_locked_and_charged_moves() {
    let mut t = T::new("garchomp", "corviknight");
    for n in 1..=3 {
        t.turn(n);
        t.e(&mv("own", "garchomp", "outrage"));
    }
    // a locked move pays only on its first turn
    assert_eq!(t.own(1).pp_used[1], 1);
    let mut t = T::new("primarina", "corviknight");
    t.turn(1);
    t.es(&[&mv("own", "primarina", "solarbeam"), r#"{"k":"prepare","side":"own","mon":"primarina"}"#]);
    t.turn(2);
    t.e(&mv("own", "primarina", "solarbeam"));
    assert_eq!(t.own(3).pp_used[3], 1);
    // a charge row with no move row before it pays when it names the move
    t.turn(3);
    t.e(r#"{"k":"prepare","side":"own","mon":"primarina","move":"solarbeam"}"#);
    t.turn(4);
    t.e(&mv("own", "primarina", "solarbeam"));
    assert_eq!(t.own(3).pp_used[3], 2);
    t.assert_clean();
}

#[test]
fn rule6_called_moves_pay_nothing_and_reveal() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "sleeptalk"), r#"{"k":"move","side":"opp","mon":"garchomp","move":"earthquake","called_by":"sleeptalk"}"#]);
    assert_eq!(t.opp(0).moves[0], Some(id("sleeptalk")));
    assert_eq!(t.opp(0).moves[1], Some(id("earthquake")));
    assert_eq!(t.opp(0).pp_used, [1, 0, 0, 0]);
    assert_eq!(t.bt().side(Side::Opp).now.called, Some(id("earthquake")));
    t.assert_clean();
}

#[test]
fn rule7_items_activated_table() {
    let item = |s: &str| ItemId::from_id(s).unwrap();
    assert!(battle::items::spent_when_activated(item("sitrusberry")));
    assert!(battle::items::spent_when_activated(item("focussash")));
    assert!(battle::items::spent_when_activated(item("whiteherb")));
    assert!(!battle::items::spent_when_activated(item("leftovers")));
    assert!(!battle::items::spent_when_activated(item("rockyhelmet")));
    assert!(!battle::items::spent_when_activated(item("airballoon")));
    for id in battle::items::SPENT_NON_BERRIES {
        assert!(ItemId::from_id(id).is_some(), "{}", id);
    }
    let mut t = T::new("gholdengo", "garchomp");
    t.e(r#"{"k":"item","side":"opp","mon":"garchomp","item":"airballoon","what":"activated"}"#);
    assert_eq!(t.opp(0).item, ItemState::Held(item("airballoon")));
    t.e(r#"{"k":"item","side":"opp","mon":"garchomp","item":"airballoon","what":"removed"}"#);
    assert_eq!(t.opp(0).item, ItemState::Gone(Some(item("airballoon"))));
    t.e(r#"{"k":"enter","side":"opp","mon":"corviknight","forced":true}"#);
    t.e(r#"{"k":"item","side":"opp","mon":"corviknight","item":"sitrusberry","what":"activated"}"#);
    assert_eq!(t.opp(2).item, ItemState::Gone(Some(item("sitrusberry"))));
    t.e(r#"{"k":"item","side":"opp","mon":"corviknight","item":"leftovers","what":"activated"}"#);
    assert_eq!(t.opp(2).item, ItemState::Held(item("leftovers")));
    t.e(r#"{"k":"item","side":"opp","mon":"corviknight","item":"focussash","what":"used"}"#);
    assert_eq!(t.opp(2).item, ItemState::Gone(Some(item("focussash"))));
    t.e(r#"{"k":"item","side":"opp","mon":"corviknight","item":"choicescarf","what":"found"}"#);
    assert_eq!(t.opp(2).item, ItemState::Held(item("choicescarf")));
    t.assert_clean();
}

#[test]
fn rule7_leppa_restores_pp() {
    let mut t = T::new("cinderace", "corviknight");
    for n in 1..=8 {
        t.turn(n);
        t.e(&mv("own", "cinderace", "protect"));
    }
    let max = id("protect").data().max_pp;
    assert_eq!(t.own(5).pp_used[3], max.min(8));
    t.e(r#"{"k":"item","side":"own","mon":"cinderace","item":"leppaberry","what":"activated"}"#);
    assert_eq!(t.own(5).item, ItemState::Gone(Some(ItemId::from_id("leppaberry").unwrap())));
    assert_eq!(t.own(5).pp_used[3], max.min(8).saturating_sub(10));
}

#[test]
fn rule8_own_hp_as_percent() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 54)]);
    assert!((t.own(0).hp - 100.0 / 3.0).abs() < 1e-4);
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"golisopod"}"#]);
    // entering with no HP row: 100 if never seen
    assert_eq!(t.own(2).hp, 100.0);
    t.turn(2);
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"golisopod"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    // the percent it last held
    assert!((t.own(0).hp - 100.0 / 3.0).abs() < 1e-4);
    // an HP row that disagrees with what the Battle held is an anomaly; the row wins
    t.e(&hp_own("gholdengo", 100, 81));
    assert_eq!(t.own(0).hp, 50.0);
    assert_eq!(t.b.anomalies.len(), 1);
}

#[test]
fn rule9_switch_pivot_drag() {
    let mut t = T::new("golisopod", "garchomp");
    t.turn(1);
    t.es(&[r#"{"k":"withdraw","side":"opp","mon":"garchomp"}"#, r#"{"k":"enter","side":"opp","mon":"gengar"}"#]);
    let r = t.bt().side(Side::Opp).now;
    assert_eq!((r.act, r.mon, r.to, r.order), (Act::Switch, Some(0), Some(1), 1));
    assert_eq!(t.opp(1).species, SpeciesId::from_id("gengar"));
    assert!(t.opp(1).seen);
    // a pivot: the move, the withdraw with pivot, our replace decision, then the enter
    t.es(&[&mv("own", "golisopod", "uturn"), r#"{"k":"withdraw","side":"own","mon":"golisopod","pivot":true}"#]);
    assert!(t.bt().active(Side::Own).is_none());
    let l = legal(t.bt(), ReqKind::Replace);
    assert_eq!(l.mask[8..], [false, true, true]);
    t.e(r#"{"k":"enter","side":"own","mon":"gholdengo"}"#);
    let r = t.bt().side(Side::Own).now;
    assert_eq!((r.act, r.mv, r.to, r.order), (Act::Pivot, Some(id("uturn")), Some(0), 2));
    // a drag: forced enter with no withdraw
    t.turn(2);
    t.es(&[&mv("opp", "gengar", "roar"), r#"{"k":"enter","side":"own","mon":"garchomp","forced":true}"#]);
    let r = t.bt().side(Side::Own).now;
    assert!(r.dragged && r.to == Some(1));
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 1 }));
    t.assert_clean();
    // a pivot withdraw with no move before it is accepted (the Move row can be lost)
    t.turn(3);
    t.e(r#"{"k":"withdraw","side":"own","mon":"garchomp","pivot":true}"#);
    t.assert_clean();
}

#[test]
fn rule9_last_turn_record_rotates() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "earthquake"), &mv("own", "gholdengo", "shadowball")]);
    t.turn(2);
    let (o, p) = (t.bt().side(Side::Own).last, t.bt().side(Side::Opp).last);
    assert_eq!((o.mv, o.order), (Some(id("shadowball")), 2));
    assert_eq!((p.mv, p.order), (Some(id("earthquake")), 1));
    assert_eq!(t.bt().side(Side::Own).now, TurnRecord::default());
}

#[test]
fn baton_pass_and_shed_tail_pass_state() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "swordsdance"),
        r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk"],"by":2}"#,
        &vol("opp", "garchomp", "substitute"),
    ]);
    t.turn(2);
    t.es(&[&mv("opp", "garchomp", "batonpass"), r#"{"k":"withdraw","side":"opp","mon":"garchomp","pivot":true}"#, r#"{"k":"enter","side":"opp","mon":"dragonite"}"#]);
    assert_eq!(t.opp_active().boosts[0], 2);
    assert!(t.opp_active().vols[0].map(|v| v.cond.id()) == Some("substitute"));
    t.assert_clean();
}

#[test]
fn status_counters() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(r#"{"k":"status","side":"own","mon":"gholdengo","status":"slp"}"#);
    t.turn(2);
    t.e(r#"{"k":"cant","side":"own","mon":"gholdengo","reason":"slp"}"#);
    t.turn(3);
    t.e(r#"{"k":"cant","side":"own","mon":"gholdengo","reason":"slp"}"#);
    assert_eq!(t.own(0).status, Status::Slp);
    assert_eq!((t.own(0).status_start, t.own(0).status_count), (1, 2));
    t.e(r#"{"k":"cure","side":"own","mon":"gholdengo","status":"slp"}"#);
    assert_eq!(t.own(0).status, Status::None);
    // an item cure reveals and spends a Berry
    t.e(r#"{"k":"status","side":"opp","mon":"garchomp","status":"par"}"#);
    t.e(r#"{"k":"cure","side":"opp","mon":"garchomp","status":"par","item":"lumberry"}"#);
    assert_eq!(t.opp(0).item, ItemState::Gone(ItemId::from_id("lumberry")));
    t.assert_clean();
}

#[test]
fn volatiles_with_turns() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(&mv("own", "gholdengo", "shadowball"));
    t.turn(2);
    t.es(&[&mv("opp", "garchomp", "encore"), &vol("own", "gholdengo", "encore"), &vol("own", "gholdengo", "confusion")]);
    let v = t.own_active().vols[0].unwrap();
    assert_eq!((v.cond.id(), v.start, v.detail), ("encore", 2, VolDetail::Move(id("shadowball"))));
    t.e(r#"{"k":"volatile","side":"opp","mon":"garchomp","effect":"perishsong","detail":3}"#);
    t.e(r#"{"k":"volatile","side":"opp","mon":"garchomp","effect":"perishsong","detail":2}"#);
    assert_eq!(t.opp_active().vols[0].unwrap().detail, VolDetail::Num(2));
    t.e(r#"{"k":"volatile_end","side":"own","mon":"gholdengo","effect":"encore"}"#);
    assert_eq!(t.own_active().vols[0].unwrap().cond.id(), "confusion");
    assert!(t.own_active().vols[1].is_none());
    // a trapping move row names the move; the condition is partiallytrapped
    t.e(&vol("own", "gholdengo", "firespin"));
    let v = t.own_active().vols[1].unwrap();
    assert_eq!((v.cond.id(), v.detail), ("partiallytrapped", VolDetail::Move(id("firespin"))));
    assert_eq!(v.by, Some(Who { side: Side::Opp, slot: 0 }));
    // switching clears them
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"garchomp"}"#]);
    assert!(t.own_active().vols.iter().all(|v| v.is_none()));
    t.assert_clean();
}

#[test]
fn mega_evolution() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[
        r#"{"k":"mega_stone","side":"opp","mon":"gengar","stone":"gengarite"}"#,
        r#"{"k":"mega","side":"opp","mon":"gengar","form":"gengarmega"}"#,
        r#"{"k":"mega_stone","side":"own","mon":"garchomp","stone":"garchompitez"}"#,
        r#"{"k":"mega","side":"own","mon":"garchomp","form":"garchompmegaz"}"#,
    ]);
    let g = t.opp(1);
    assert!(g.mega && g.form == SpeciesId::from_id("gengarmega"));
    assert_eq!(g.item, ItemState::Held(ItemId::from_id("gengarite").unwrap()));
    // a Mega has one ability, so it is known
    assert_eq!(g.ability.map(|a| a.id()), Some("shadowtag"));
    assert!(t.bt().side(Side::Opp).mega_used && t.bt().side(Side::Own).mega_used);
    assert_eq!(t.own(1).form, SpeciesId::from_id("garchompmegaz"));
    // the contract still names it as brought
    t.es(&[&mv("own", "garchomp", "earthquake")]);
    t.assert_clean();
    // Mega without the stone row: the stone is known from the data
    let mut t = T::new("gholdengo", "gengar");
    t.e(r#"{"k":"mega","side":"opp","mon":"gengar","form":"gengarmega"}"#);
    assert_eq!(t.opp(1).item, ItemState::Held(ItemId::from_id("gengarite").unwrap()));
}

#[test]
fn abilities_items_types_forms() {
    let mut t = T::new("gholdengo", "rotomwash");
    t.e(r#"{"k":"ability_change","side":"opp","mon":"rotomwash","from":"trace","to":"goodasgold"}"#);
    assert_eq!(t.opp(3).base_ability.map(|a| a.id()), Some("trace"));
    assert_eq!(t.opp(3).ability.map(|a| a.id()), Some("goodasgold"));
    t.e(r#"{"k":"types","side":"own","mon":"gholdengo","type":"water"}"#);
    assert_eq!(t.bt().types(Who { side: Side::Own, slot: 0 }), Some([Type::Water, Type::Typeless]));
    t.e(r#"{"k":"item","side":"own","mon":"gholdengo","item":"choicescarf","what":"removed","of":{"side":"opp","mon":"rotomwash"}}"#);
    assert_eq!(t.own(0).item, ItemState::Gone(ItemId::from_id("choicescarf")));
    t.e(r#"{"k":"transform","side":"opp","mon":"rotomwash","into":"gholdengo"}"#);
    // transformed into ours: our moves are its moves
    assert_eq!(t.opp_active().t_moves, t.own(0).moves);
    t.assert_clean();
}

#[test]
fn faint_revive_unread_end() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[&mv("opp", "garchomp", "earthquake"), &hp_own("gholdengo", 162, 0), r#"{"k":"faint","side":"own","mon":"gholdengo"}"#]);
    assert!(t.own(0).fainted && t.own(0).hp == 0.0);
    assert!(t.bt().active(Side::Own).is_none());
    t.e(r#"{"k":"enter","side":"own","mon":"garchomp"}"#);
    t.turn(2);
    t.es(&[&mv("own", "garchomp", "earthquake"), r#"{"k":"revive","side":"own","mon":"gholdengo"}"#]);
    assert!(!t.own(0).fainted);
    assert_eq!(t.own(0).hp, 50.0);
    t.e(r#"{"k":"unread","text":"???"}"#);
    t.e(r#"{"k":"unread","text":"!!!"}"#);
    assert_eq!(t.bt().unread, 2);
    t.e(r#"{"k":"end","result":"win"}"#);
    assert_eq!(t.bt().result, Some(Outcome::Win));
    t.assert_clean();
}

#[test]
fn gaps_mark_the_battle_incomplete() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(&mv("own", "gholdengo", "shadowball"));
    assert!(!t.bt().incomplete);
    t.turn(2);
    t.e(r#"{"k":"gap","what":"turn","n":2}"#);
    t.turn(3);
    t.e(r#"{"k":"gap","what":"rows"}"#);
    t.e(&mv("own", "gholdengo", "shadowball"));
    assert!(t.bt().incomplete && t.bt().gaps == 2);
    assert_eq!(t.bt().turn, 3);
    // a rebuild gives the same Battle
    assert_eq!(Builder::rebuild(&t.rec).battle, *t.bt());
    t.assert_clean();
}

#[test]
fn bad_events_are_anomalies_not_panics() {
    let mut t = T::new("gholdengo", "garchomp");
    t.es(&[
        r#"{"k":"move","side":"opp","mon":"mewtwo","move":"psychic"}"#,
        r#"{"k":"move","side":"opp","mon":"garchomp","move":"notamove"}"#,
        r#"{"k":"enter","side":"own","mon":"pikachu"}"#,
        r#"{"k":"hp","side":"opp","mon":"garchomp","dir":"lost","before_pct":50,"after_pct":5}"#,
        r#"{"k":"refused","order":{"kind":"move","move":"tackle","mega":false}}"#,
        r#"{"k":"volatile","side":"own","mon":"gholdengo","effect":"made_up"}"#,
        r#"{"k":"boost","side":"own","mon":"gholdengo","stats":["luck"],"by":1}"#,
        r#"{"k":"weather","weather":"meteors","what":"start"}"#,
        r#"{"k":"side","side":"own","effect":"mist","what":"start"}"#,
    ]);
    assert_eq!(t.b.anomalies.len(), 9, "{:?}", t.anomalies());
    assert_eq!(t.bt().anomalies, 9);
    // events before battle_start
    let mut b = Builder::new();
    b.apply_event(&Event::Turn { n: 1 });
    assert_eq!(b.anomalies.len(), 1);
}

/// Random event lists drawn from a pool: the builder must never panic.
#[test]
fn random_event_lists_never_panic() {
    let pool: Vec<Event> = {
        let mons = ["gholdengo", "garchomp", "golisopod", "gengar", "corviknight", "mewtwo"];
        let mut v = vec![];
        for (i, m) in mons.iter().enumerate() {
            let side = if i % 2 == 0 { Side::Own } else { Side::Opp };
            for s in [Side::Own, Side::Opp, side] {
                let who = Mon::new(s, m);
                v.push(Event::Enter { who: who.clone(), forced: i % 3 == 0 });
                v.push(Event::Withdraw { who: who.clone(), pivot: i % 2 == 1 });
                v.push(Event::Move { who: who.clone(), mv: "outrage".into(), priority: None, called_by: None });
                v.push(Event::Move { who: who.clone(), mv: "hyperbeam".into(), priority: None, called_by: None });
                v.push(Event::Prepare { who: who.clone(), mv: None });
                v.push(Event::Faint { who: who.clone() });
                v.push(Event::Revive { who: who.clone() });
                v.push(Event::Volatile { who: who.clone(), effect: "encore".into(), detail: None });
                v.push(Event::VolatileEnd { who: who.clone(), effect: "illusion".into() });
                v.push(Event::Item { who: Some(who.clone()), item: "ejectbutton".into(), what: ItemWhat::Used, of: None });
                v.push(Event::Transform { who: who.clone(), into: "ditto".into() });
                v.push(Event::Hp { who: who.clone(), dir: HpDir::Lost, hp: Hp::Own { before: 300, after: -5 } });
                v.push(Event::Hp { who: who.clone(), dir: HpDir::Lost, hp: Hp::Opp { before_pct: 50.0, after_pct: 250.0 } });
                v.push(Event::Cant { who: Some(who.clone()), reason: "slp".into() });
                v.push(Event::Mega { who: who.clone(), form: Some("garchompmegaz".into()) });
            }
        }
        v.push(Event::Turn { n: 0 });
        v.push(Event::Turn { n: 70000 });
        v.push(Event::Fail { who: None, reason: None });
        v.push(Event::BoostReset { who: None, what: ResetWhat::Copy, of: None, stats: None });
        v.push(Event::Refused { order: Order::Switch { mon: "x".into() } });
        v
    };
    let mut seed: u64 = 0x9e3779b97f4a7c15;
    for _ in 0..300 {
        let mut t = T::new("gholdengo", "garchomp");
        for _ in 0..80 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            t.b.apply_event(&pool[(seed % pool.len() as u64) as usize]);
            let _ = legal(t.bt(), ReqKind::Turn);
            let _ = legal(t.bt(), ReqKind::Replace);
        }
    }
}
