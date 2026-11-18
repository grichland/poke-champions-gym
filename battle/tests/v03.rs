//! Contract 0.3 additions (LIVE.md, the controller's review CT-n).

mod common;
use battle::*;
use common::*;
use cg_engine::sim::data::*;

#[test]
fn multi_stat_boost_and_limit() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "dragondance"),
        r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk","spe"],"by":1}"#,
        r#"{"k":"boost","side":"own","mon":"gholdengo","stats":["accuracy","evasion"],"by":-1}"#,
        r#"{"k":"boost_limit","side":"opp","mon":"garchomp","stats":["def","spd"],"dir":"up"}"#,
    ]);
    let b = t.opp_active().boosts;
    assert_eq!((b[0], b[1], b[3], b[4]), (1, 6, 6, 1));
    assert_eq!(t.own_active().boosts[5..], [-1, -1]);
    t.assert_clean();
}

#[test]
fn mega_form_from_stone_above_or_own_set() {
    let mut t = T::new("garchomp", "dragonite");
    t.turn(1);
    // our Garchomp holds Garchompite Z: the row naming no form is Mega Garchomp Z
    t.e(r#"{"k":"mega","side":"own","mon":"garchomp"}"#);
    assert_eq!(t.own(1).form, SpeciesId::from_id("garchompmegaz"));
    // the opponent's from the stone row above
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        r#"{"k":"mega_stone","side":"opp","mon":"garchomp","stone":"garchompite"}"#,
        r#"{"k":"mega","side":"opp","mon":"garchomp"}"#,
    ]);
    assert_eq!(t.opp(0).form, SpeciesId::from_id("garchompmega"));
    assert!(t.bt().side(Side::Opp).mega_used);
    t.assert_clean();
    // no stone and two Megas: unknown form is an anomaly, the Mega is still used
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(r#"{"k":"mega","side":"opp","mon":"garchomp"}"#);
    assert!(t.bt().side(Side::Opp).mega_used && t.opp(0).mega);
    assert_eq!(t.b.anomalies.len(), 1);
}

#[test]
fn types_lost() {
    let mut t = T::new("gholdengo", "rotomwash");
    t.turn(1);
    // Rotom-Wash is Electric/Water: Double Shock leaves Water
    t.e(r#"{"k":"types","side":"opp","mon":"rotomwash","lose":"electric"}"#);
    assert_eq!(t.bt().types(Who { side: Side::Opp, slot: 3 }), Some([Type::Water, Type::Typeless]));
    t.e(r#"{"k":"types","side":"opp","mon":"rotomwash","lose":"water"}"#);
    assert_eq!(t.bt().types(Who { side: Side::Opp, slot: 3 }), Some([Type::Typeless, Type::Typeless]));
    t.assert_clean();
}

#[test]
fn rest_row_sticky_web_on_a_mon_gem_without_a_mon() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "rest"),
        r#"{"k":"status","side":"opp","mon":"garchomp","status":"slp","effect":"slept_and_restored"}"#,
        r#"{"k":"side","side":"own","effect":"stickyweb","what":"start"}"#,
    ]);
    assert_eq!(t.opp(0).status, Status::Slp);
    t.turn(2);
    t.es(&[
        r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"enter","side":"own","mon":"garchomp"}"#,
        r#"{"k":"side","side":"own","effect":"stickyweb","what":"acted","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "fakeout"),
        r#"{"k":"item","item":"normalgem","what":"used"}"#,
    ]);
    assert_eq!(t.opp(0).item, ItemState::Gone(ItemId::from_id("normalgem")));
    t.assert_clean();
}

#[test]
fn wording_ids_are_accepted() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "drainpunch"),
        &hp_own("gholdengo", 162, 100),
        r#"{"k":"cause","effect":"energy_drained","side":"opp","mon":"garchomp"}"#,
        &hp_opp("garchomp", 100, 100),
        r#"{"k":"cause","effect":"lost_some_hp","side":"opp","mon":"garchomp"}"#,
        &hp_opp("garchomp", 100, 90),
        r#"{"k":"cause","effect":"pain_split"}"#,
        r#"{"k":"volatile_hit","effect":"endured_hit","side":"opp","mon":"garchomp"}"#,
        r#"{"k":"volatile","side":"opp","mon":"garchomp","effect":"type_power_rose","detail":"electric"}"#,
    ]);
    assert!(t.opp_active().vols.iter().flatten().any(|v| v.cond.id() == "charge"));
    t.assert_clean();
}

#[test]
fn red_card_and_eject_button_switches() {
    // Red Card: our attacker is dragged out, no withdraw row
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("own", "gholdengo", "shadowball"),
        r#"{"k":"item","side":"opp","mon":"garchomp","item":"redcard","what":"used","of":{"side":"own","mon":"gholdengo"}}"#,
        r#"{"k":"enter","side":"own","mon":"garchomp","forced":true}"#,
    ]);
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 1 }));
    assert!(t.bt().side(Side::Own).now.dragged);
    // Eject Button: the item row then an enter is a switch
    t.turn(2);
    t.es(&[
        &mv("own", "garchomp", "earthquake"),
        r#"{"k":"item","side":"opp","mon":"garchomp","item":"ejectbutton","what":"used"}"#,
        r#"{"k":"enter","side":"opp","mon":"gengar"}"#,
    ]);
    assert_eq!(t.bt().active(Side::Opp), Some(Who { side: Side::Opp, slot: 1 }));
    t.assert_clean();
}

#[test]
fn illusion_is_re_addressed() {
    // Zoroark-Hisui appears as Garchomp; its rows carry the shown name until the illusion ends
    let mut t = T { b: Builder::new(), rec: vec![] };
    let pv = ["garchomp", "zoroarkhisui", "corviknight", "rotomwash", "dragonite", "ditto"];
    let opp_preview = pv.iter().map(|s| Some(s.to_string())).collect();
    t.input(Input::Msg(msg(Body::BattleStart { own_team: team(), opp_preview })));
    t.input(Input::Picks(Picks { picks: vec![0, 1, 2] }));
    t.e(r#"{"k":"enter","side":"own","mon":"gholdengo"}"#);
    t.e(r#"{"k":"enter","side":"opp","mon":"corviknight"}"#);
    t.turn(1);
    t.es(&[
        r#"{"k":"withdraw","side":"opp","mon":"corviknight"}"#,
        r#"{"k":"enter","side":"opp","mon":"garchomp"}"#,
        &mv("opp", "garchomp", "shadowball"),
        &mv("own", "gholdengo", "makeitrain"),
        &hp_opp("garchomp", 100, 40),
        r#"{"k":"volatile_end","side":"opp","mon":"zoroarkhisui","effect":"illusion"}"#,
    ]);
    let zor = t.bt().active(Side::Opp).unwrap();
    assert_eq!(t.bt().mon(zor).species, SpeciesId::from_id("zoroarkhisui"));
    assert_eq!(t.bt().mon(zor).hp, 40.0);
    assert_eq!(t.bt().mon(zor).moves[0], MoveId::from_id("shadowball"));
    // the shown Garchomp never entered
    assert!(!t.opp(0).seen && t.opp(0).hp == 100.0 && t.opp(0).moves[0].is_none());
    t.assert_clean();
}

#[test]
fn leads_after_the_first_turn_event_and_forfeit() {
    let mut t = T { b: Builder::new(), rec: vec![] };
    t.input(Input::Msg(msg(Body::BattleStart { own_team: team(), opp_preview: preview() })));
    t.input(Input::Picks(Picks { picks: vec![0, 1, 2] }));
    t.turn(1);
    t.e(r#"{"k":"enter","side":"opp","mon":"garchomp"}"#);
    t.e(r#"{"k":"enter","side":"own","mon":"gholdengo"}"#);
    assert!(t.own(0).was_lead && t.opp(0).was_lead);
    assert_eq!(t.bt().side(Side::Own).now.act, Act::None);
    t.input(Input::Msg(msg(Body::BattleEnd { result: "forfeit".into() })));
    assert_eq!(t.bt().result, Some(Outcome::Forfeit));
    t.assert_clean();
}

#[test]
fn switching_moves_and_abilities_without_a_withdraw_row() {
    // the recordings show no withdraw row after Baton Pass: the move's switch, boosts passed
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "swordsdance"),
        r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk"],"by":2}"#,
    ]);
    t.turn(2);
    t.es(&[&mv("opp", "garchomp", "batonpass"), r#"{"k":"enter","side":"opp","mon":"gengar"}"#]);
    assert_eq!(t.opp_active().boosts[0], 2);
    assert_eq!(t.bt().side(Side::Opp).now.act, Act::Pivot);
    // Emergency Exit: the ability row, then the enter
    let mut t = T::new("golisopod", "garchomp");
    t.turn(1);
    t.es(&[
        &mv("opp", "garchomp", "earthquake"),
        &hp_own("golisopod", 150, 60),
        r#"{"k":"ability","side":"own","mon":"golisopod","ability":"emergencyexit"}"#,
        r#"{"k":"enter","side":"own","mon":"gholdengo"}"#,
    ]);
    assert_eq!(t.bt().active(Side::Own), Some(Who { side: Side::Own, slot: 0 }));
    t.assert_clean();
}
