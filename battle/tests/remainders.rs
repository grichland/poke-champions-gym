//! The Battle against the rows behind the remainders of the player-view hunt (U1 to U17): each one the
//! builder once got wrong, scripted as the events the game writes.

mod common;

use battle::*;
use common::*;
use cg_engine::sim::data::ItemId;

fn request(t: &mut T, kind: ReqKind, screen_offer: Option<json::Json>) {
    t.input(Input::Msg(msg(Body::DecisionRequest { seq: 1, kind, deadline_ms: None, refused: None, retry: false, screen_offer })));
}

fn charging(a: &Active) -> bool {
    matches!(a.lock, Lock::Charging { .. })
}

/// U2: a Future Sight landing names its user's side, but that Pokémon did not move: its charge holds.
#[test]
fn future_sight_landing_keeps_a_charge() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[
        &mv("own", "primarina", "solarbeam"),
        r#"{"k":"prepare","side":"own","mon":"primarina"}"#,
        r#"{"k":"once","effect":"futuresight","side":"opp","mon":"gengar"}"#,
        r#"{"k":"result","how":"immune","side":"opp","mon":"gengar"}"#,
    ]);
    t.turn(2);
    assert!(charging(t.own_active()), "{:?}", t.own_active().lock);
    let l = t.legal();
    assert!(l.mask[3] && l.mask.iter().filter(|x| **x).count() == 1);
    // one use counted
    let max = cg_engine::sim::data::MoveId::from_id("solarbeam").unwrap().data().max_pp;
    assert_eq!(t.bt().pp_left(Who { side: Side::Own, slot: 3 })[3], Some(max - 1));
    t.assert_clean();
}

/// Future Sight's use row (`futuresight_start`, naming the user) is not a landing: the move stays its
/// user's; the landing two turns later (`futuresight`, naming the target) is the delayed attack.
#[test]
fn future_sight_use_and_landing_are_two_rows() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "futuresight"), r#"{"k":"once","effect":"futuresight_start","side":"opp","mon":"gengar"}"#]);
    let c = &t.bt().ctx;
    assert_eq!(c.last_move.map(|x| (x.0.side, x.1.id())), Some((Side::Opp, "futuresight")));
    assert!(!c.delayed);
    t.turn(2);
    assert_eq!(t.bt().track.sides[1].future_turn, Some(1));
    t.turn(3);
    t.es(&[
        r#"{"k":"once","effect":"futuresight","side":"own","mon":"primarina"}"#,
        r#"{"k":"hp","side":"own","mon":"primarina","dir":"lost","before":155,"after":60}"#,
    ]);
    let c = &t.bt().ctx;
    assert!(c.delayed);
    assert_eq!(c.last_move.map(|x| (x.0.side, x.1.id(), x.2.map(|w| w.side))), Some((Side::Opp, "futuresight", Some(Side::Own))));
    assert_eq!(t.bt().track.sides[1].future_turn, None);
    t.assert_clean();
}

/// U7: a confusion self-hit's HP row is caused by the self-hit, not by the last move (the opponent's
/// charging move goes on charging, and is not credited with the damage).
#[test]
fn confusion_self_hit_is_its_own_cause() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&vol("own", "primarina", "confusion")]);
    t.turn(2);
    t.es(&[
        &mv("opp", "gengar", "phantomforce"),
        r#"{"k":"prepare","side":"opp","mon":"gengar"}"#,
        r#"{"k":"volatile_hit","effect":"confusion","side":"own","mon":"primarina"}"#,
        r#"{"k":"volatile_hit","effect":"confusion_self_hit"}"#,
        &hp_own("primarina", 155, 130),
    ]);
    assert!(charging(t.opp_active()), "{:?}", t.opp_active().lock);
    assert_eq!(t.bt().side(Side::Opp).now.damage, 0.0);
    t.assert_clean();
}

/// U8: a Substitute broken by the move shows the move went on: no charging turn follows.
#[test]
fn substitute_broken_releases_a_charge() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&vol("opp", "gengar", "substitute")]);
    t.turn(2);
    t.es(&[
        &mv("own", "primarina", "solarbeam"),
        r#"{"k":"prepare","side":"own","mon":"primarina"}"#,
        r#"{"k":"volatile_end","side":"opp","mon":"gengar","effect":"substitute"}"#,
    ]);
    t.turn(3);
    assert!(!charging(t.own_active()));
    t.assert_clean();
}

/// U3: a faint is replaced after the end of the turn: Endure is over at the replacement request.
#[test]
fn replacement_after_a_faint_comes_after_the_turn_end() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "endure"), &vol("opp", "gengar", "endure"), &hp_own("primarina", 155, 0), r#"{"k":"faint","side":"own","mon":"primarina"}"#]);
    assert!(t.opp_active().vols.iter().flatten().any(|v| v.cond.id() == "endure"));
    request(&mut t, ReqKind::Replace, None);
    assert!(!t.opp_active().vols.iter().flatten().any(|v| v.cond.id() == "endure"));
    assert!(t.bt().ctx.post_residual);
    t.assert_clean();
}

/// U1: an ability gained on the field (Trace, Skill Swap) is the brought one again after a faint.
#[test]
fn faint_restores_the_brought_ability() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[r#"{"k":"ability_change","side":"opp","mon":"gengar","from":"cursedbody","to":"levitate"}"#]);
    assert_eq!(t.opp(1).ability.map(|a| a.id()), Some("levitate"));
    t.es(&[&hp_opp("gengar", 100, 0), r#"{"k":"faint","side":"opp","mon":"gengar"}"#]);
    assert_eq!(t.opp(1).ability.map(|a| a.id()), Some("cursedbody"));
    t.assert_clean();
}

/// U4: a theft takes the item; Trick's two rows swap; a flung Berry is the thrower's.
#[test]
fn item_thefts_swaps_and_flung_berries() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "thief"), r#"{"k":"item","side":"opp","mon":"gengar","item":"leftovers","what":"received","of":{"side":"own","mon":"primarina"}}"#]);
    assert_eq!(t.own(3).item, ItemState::Gone(None));
    assert_eq!(t.opp(1).item, ItemState::Held(ItemId::from_id("leftovers").unwrap()));
    t.turn(2);
    t.es(&[
        &mv("opp", "gengar", "trick"),
        r#"{"k":"item","side":"opp","mon":"gengar","item":"leftovers","what":"received","of":{"side":"own","mon":"primarina"}}"#,
        r#"{"k":"item","side":"own","mon":"primarina","item":"choicescarf","what":"received","of":{"side":"opp","mon":"gengar"}}"#,
    ]);
    assert_eq!(t.own(3).item, ItemState::Held(ItemId::from_id("choicescarf").unwrap()));
    t.turn(3);
    t.es(&[
        &mv("opp", "gengar", "fling"),
        &hp_own("primarina", 155, 140),
        r#"{"k":"item","side":"own","mon":"primarina","item":"sitrusberry","what":"activated","of":{"side":"opp","mon":"gengar"}}"#,
        &hp_own("primarina", 140, 155),
    ]);
    assert_eq!(t.own(3).item, ItemState::Held(ItemId::from_id("choicescarf").unwrap()));
    assert_eq!(t.opp(1).item, ItemState::Gone(ItemId::from_id("sitrusberry")));
}

/// U5: transformed into the imprisoner, every copied move is one it knows: Struggle.
#[test]
fn transformed_into_the_imprisoner_struggles() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[r#"{"k":"transform","side":"own","mon":"primarina","into":"gengar"}"#, &mv("opp", "gengar", "imprison"), &vol("opp", "gengar", "imprison")]);
    t.turn(2);
    let mut so = json::Json::obj();
    so.set("moves", vec![json::Json::from("shadowball"), json::Json::from("sludgebomb"), json::Json::from("imprison"), json::Json::from("protect")]);
    request(&mut t, ReqKind::Turn, Some(so));
    let l = t.legal();
    assert_eq!(l.forced, Some(Forced::Struggle));
    t.assert_clean();
}

/// U9: a confusion another Pokémon's move causes is not the fatigue of a locked move.
#[test]
fn confusion_from_a_move_is_not_fatigue() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("own", "garchomp", "outrage"), &mv("opp", "gengar", "confuseray"), &vol("own", "garchomp", "confusion")]);
    assert!(matches!(t.own_active().lock, Lock::Locked { .. }));
    t.assert_clean();
}

/// U16: Throat Chop's hit silences sound moves for that turn and the next, with no row.
#[test]
fn throat_chop_hit_silences_sound_moves() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "throatchop"), &hp_own("primarina", 155, 100)]);
    t.turn(2);
    assert!(!t.legal().mask[1], "Hyper Voice is a sound move");
    t.turn(3);
    assert!(t.legal().mask[1]);
    t.assert_clean();
}

/// U18: a healing move failing because HP is full shows the HP (after a silent Regenerator heal).
#[test]
fn hp_full_failure_shows_full_hp() {
    let mut t = T::new("primarina", "gengar");
    t.turn(1);
    t.es(&[&hp_opp("gengar", 100, 70)]);
    t.turn(2);
    t.es(&[&mv("opp", "gengar", "recover"), r#"{"k":"fail","side":"opp","mon":"gengar","reason":"hpfull"}"#]);
    assert_eq!(t.opp(1).hp, 100.0);
}

/// U17: a confusion self-hit is an attempt to move: Fake Out is no longer the first action.
#[test]
fn self_hit_counts_as_a_move_action() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&vol("own", "garchomp", "confusion"), r#"{"k":"volatile_hit","effect":"confusion","side":"own","mon":"garchomp"}"#, r#"{"k":"volatile_hit","effect":"confusion_self_hit"}"#, &hp_own("garchomp", 183, 160)]);
    t.turn(2);
    assert!(!t.legal().mask[3], "Fake Out only on the first move action");
    t.assert_clean();
}
