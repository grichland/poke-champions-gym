//! One hand-written event list per legality rule. Actions: 0-3 moves, 4-7 moves with Mega, 8-10 switches
//! to picks[0..3].

mod common;
use battle::*;
use common::*;
use cg_engine::sim::data::*;

fn moves_mask(l: &Legal) -> [bool; 4] {
    [l.mask[0], l.mask[1], l.mask[2], l.mask[3]]
}
fn switch_mask(l: &Legal) -> [bool; 3] {
    [l.mask[8], l.mask[9], l.mask[10]]
}

#[test]
fn layout_moves_mega_switches() {
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    let l = t.legal();
    assert_eq!(l.mask, [true, true, true, true, true, true, true, true, false, true, true]);
    assert!(l.can_mega && !l.trapped && l.forced.is_none());
    assert_eq!(order_for(t.bt(), &l, 0), Some(Order::Move { mv: "earthquake".into(), mega: false }));
    assert_eq!(order_for(t.bt(), &l, 5), Some(Order::Move { mv: "outrage".into(), mega: true }));
    assert_eq!(order_for(t.bt(), &l, 9), Some(Order::Switch { mon: "gholdengo".into() }));
    assert_eq!(order_for(t.bt(), &l, 8), None);
    assert_eq!(l.count(), 10);
}

#[test]
fn mega_once_per_battle_with_a_stone() {
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.es(&[r#"{"k":"mega","side":"own","mon":"garchomp","form":"garchompmegaz"}"#, &mv("own", "garchomp", "earthquake")]);
    t.turn(2);
    assert!(!t.legal().can_mega);
    // another Pokémon with a stone cannot either: one Mega per side
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"garchomp"}"#, r#"{"k":"enter","side":"own","mon":"golisopod"}"#]);
    t.turn(3);
    assert!(!t.legal().can_mega);
    assert_eq!(t.legal().mask[4..8], [false; 4]);
    // no stone, no Mega
    let mut t = T::new("primarina", "corviknight");
    t.turn(1);
    assert!(!t.legal().can_mega);
    // the opponent's Mega does not use ours
    let mut t = T::new("golisopod", "gengar");
    t.e(r#"{"k":"mega","side":"opp","mon":"gengar","form":"gengarmega"}"#);
    t.turn(1);
    assert!(t.legal().can_mega);
}

#[test]
fn pp_zero() {
    let mut t = T::new("primarina", "corviknight");
    let max = MoveId::from_id("moonblast").unwrap().data().max_pp;
    for n in 1..=max as u32 {
        t.turn(n);
        t.e(&mv("own", "primarina", "moonblast"));
    }
    t.turn(max as u32 + 1);
    assert_eq!(moves_mask(&t.legal()), [false, true, true, true]);
    assert_eq!(battle::legal::move_blocked(t.bt(), 0), Some("no PP"));
}

#[test]
fn disable_encore_taunt_torment() {
    let mut t = T::new("garchomp", "gengar");
    // Fake Out (slot 3) is out after the first action in every check below
    t.turn(1);
    t.es(&[&mv("own", "garchomp", "earthquake"), r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"disable","detail":"earthquake"}"#]);
    t.turn(2);
    assert_eq!(moves_mask(&t.legal()), [false, true, true, false]);
    t.e(r#"{"k":"volatile_end","side":"own","mon":"garchomp","effect":"disable"}"#);
    t.es(&[&mv("own", "garchomp", "swordsdance"), &vol("own", "garchomp", "encore")]);
    t.turn(3);
    assert_eq!(moves_mask(&t.legal()), [false, false, true, false]);
    t.e(r#"{"k":"volatile_end","side":"own","mon":"garchomp","effect":"encore"}"#);
    t.e(&vol("own", "garchomp", "taunt"));
    t.turn(4);
    assert_eq!(moves_mask(&t.legal()), [true, true, false, false]);
    t.e(r#"{"k":"volatile_end","side":"own","mon":"garchomp","effect":"taunt"}"#);
    t.es(&[&mv("own", "garchomp", "earthquake"), &vol("own", "garchomp", "torment")]);
    t.turn(5);
    assert_eq!(moves_mask(&t.legal()), [false, true, true, false]);
    t.assert_clean();
}

#[test]
fn heal_block_throat_chop_gravity() {
    let mut t = T::new("gholdengo", "gengar");
    t.e(&vol("own", "gholdengo", "healblock"));
    t.turn(1);
    assert_eq!(moves_mask(&t.legal()), [true, true, true, false]);
    let mut t = T::new("primarina", "gengar");
    t.e(&vol("own", "primarina", "throatchop"));
    t.turn(1);
    assert_eq!(moves_mask(&t.legal()), [true, false, true, true]);
    let team = vec![
        set("corviknight", Some("leftovers"), "pressure", ["bravebird", "highjumpkick", "roost", "uturn"], 205),
        set("garchomp", None, "roughskin", ["earthquake", "outrage", "swordsdance", "fakeout"], 183),
        set("excadrill", Some("airballoon"), "sandrush", ["earthquake", "ironhead", "rapidspin", "swordsdance"], 185),
    ];
    let mut t = T::from_team(team, [0, 1, 2], "corviknight", "gengar");
    t.e(r#"{"k":"field","effect":"gravity","what":"start"}"#);
    t.turn(1);
    assert_eq!(moves_mask(&t.legal()), [true, false, true, true]);
    t.assert_clean();
}

#[test]
fn imprison_blocks_moves_the_imprisoner_showed() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "earthquake"), &mv("opp", "gengar", "imprison"), &vol("opp", "gengar", "imprison")]);
    t.turn(2);
    assert_eq!(moves_mask(&t.legal()), [false, true, true, true]);
}

#[test]
fn choice_lock_from_our_item() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(&mv("own", "gholdengo", "shadowball"));
    t.turn(2);
    assert_eq!(moves_mask(&t.legal()), [false, true, false, false]);
    assert_eq!(battle::legal::move_blocked(t.bt(), 0), Some("Choice lock"));
    // switching out ends it
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"garchomp"}"#]);
    t.turn(3);
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"garchomp"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    t.turn(4);
    assert_eq!(moves_mask(&t.legal()), [true; 4]);
    t.e(&mv("own", "gholdengo", "makeitrain"));
    t.turn(5);
    assert_eq!(moves_mask(&t.legal()), [true, false, false, false]);
    // losing the item ends it
    t.e(r#"{"k":"item","side":"own","mon":"gholdengo","item":"choicescarf","what":"removed"}"#);
    t.turn(6);
    assert_eq!(moves_mask(&t.legal()), [true; 4]);
    // a non-choice holder is never locked
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.e(&mv("own", "garchomp", "swordsdance"));
    t.turn(2);
    assert_eq!(moves_mask(&t.legal()), [true, true, true, false]);
    t.assert_clean();
}

#[test]
fn struggle_when_nothing_is_usable() {
    let mut t = T::new("gholdengo", "garchomp");
    t.turn(1);
    t.e(&mv("own", "gholdengo", "nastyplot"));
    t.e(&vol("own", "gholdengo", "taunt"));
    t.turn(2);
    let l = t.legal();
    assert_eq!(l.forced, Some(Forced::Struggle));
    assert_eq!(l.mask[..8], [true, false, false, false, false, false, false, false]);
    assert_eq!(order_for(t.bt(), &l, 0), Some(Order::Move { mv: "struggle".into(), mega: false }));
    // switching stays open
    assert_eq!(switch_mask(&l), [false, true, true]);
    t.e(&mv("own", "gholdengo", "struggle"));
    t.assert_clean();
}

#[test]
fn gigaton_hammer_not_twice_in_a_row() {
    let mut t = T::new("mamoswine", "corviknight");
    t.turn(1);
    t.e(&mv("own", "mamoswine", "gigatonhammer"));
    t.turn(2);
    assert_eq!(moves_mask(&t.legal()), [true, true, false, true]);
    t.e(&mv("own", "mamoswine", "earthquake"));
    t.turn(3);
    assert_eq!(moves_mask(&t.legal()), [true; 4]);
}

#[test]
fn fake_out_and_first_impression_first_action_only() {
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    assert!(t.legal().mask[3]);
    t.e(&mv("own", "garchomp", "earthquake"));
    t.turn(2);
    assert!(!t.legal().mask[3]);
    // a `cant` row counts as an action
    let mut t = T::new("golisopod", "corviknight");
    t.turn(1);
    t.e(r#"{"k":"cant","side":"own","mon":"golisopod","reason":"flinch"}"#);
    t.turn(2);
    assert!(!t.legal().mask[0]);
    // back in: usable again
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"golisopod"}"#, r#"{"k":"enter","side":"own","mon":"gholdengo"}"#]);
    t.turn(3);
    t.es(&[r#"{"k":"withdraw","side":"own","mon":"gholdengo"}"#, r#"{"k":"enter","side":"own","mon":"golisopod"}"#]);
    t.turn(4);
    assert!(t.legal().mask[0]);
}

#[test]
fn recharge() {
    let mut t = T::new("primarina", "corviknight");
    t.turn(1);
    t.es(&[&mv("own", "primarina", "hyperbeam"), &hp_opp("corviknight", 100, 40)]);
    t.turn(2);
    let l = t.legal();
    assert_eq!(l.forced, Some(Forced::Recharge));
    assert_eq!(l.count(), 1);
    assert!(l.trapped);
    assert_eq!(order_for(t.bt(), &l, 0), Some(Order::Move { mv: "recharge".into(), mega: false }));
    t.e(r#"{"k":"cant","side":"own","mon":"primarina","reason":"recharge"}"#);
    t.turn(3);
    assert!(t.legal().forced.is_none());
    // a miss sets up no recharge
    t.es(&[&mv("own", "primarina", "hyperbeam"), r#"{"k":"miss","side":"opp","mon":"corviknight"}"#]);
    t.turn(4);
    assert!(t.legal().forced.is_none());
    t.assert_clean();
}

#[test]
fn locked_move_until_fatigue() {
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.e(&mv("own", "garchomp", "outrage"));
    t.turn(2);
    let l = t.legal();
    assert_eq!(l.mask, [false, true, false, false, false, false, false, false, false, false, false]);
    t.es(&[&mv("own", "garchomp", "outrage"), &vol("own", "garchomp", "confusion")]);
    t.turn(3);
    let l = t.legal();
    assert!(l.forced.is_none() && !l.trapped);
    assert_eq!(moves_mask(&l), [true, true, true, false]);
    // a lock that is not continued (the user could not move) ends
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.e(&mv("own", "garchomp", "outrage"));
    t.turn(2);
    t.e(r#"{"k":"cant","side":"own","mon":"garchomp","reason":"par"}"#);
    t.turn(3);
    assert!(t.legal().mask[0]);
    t.assert_clean();
}

#[test]
fn charging_move() {
    let mut t = T::new("primarina", "corviknight");
    t.turn(1);
    t.es(&[&mv("own", "primarina", "solarbeam"), r#"{"k":"prepare","side":"own","mon":"primarina"}"#]);
    t.turn(2);
    let l = t.legal();
    assert_eq!(l.count(), 1);
    assert!(l.mask[3] && l.trapped);
    t.e(&mv("own", "primarina", "solarbeam"));
    t.turn(3);
    assert_eq!(t.legal().count(), 6);
    // a charge row naming no move and with no move row: our only charging move
    t.e(r#"{"k":"prepare","side":"own","mon":"primarina"}"#);
    t.turn(4);
    let l = t.legal();
    assert!(l.mask[3] && l.count() == 1);
}

#[test]
fn trapping_conditions_and_ghost() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "meanlook"), &vol("own", "garchomp", "trapped")]);
    t.turn(2);
    let l = t.legal();
    assert!(l.trapped);
    assert_eq!(switch_mask(&l), [false; 3]);
    // a Ghost is never trapped
    let mut t = T::new("gholdengo", "gengar");
    t.e(&vol("own", "gholdengo", "trapped"));
    t.turn(1);
    assert!(!t.legal().trapped);
    // Shed Shell frees from traps
    let mut t = T::new("mamoswine", "gengar");
    t.e(&vol("own", "mamoswine", "trapped"));
    t.turn(1);
    assert!(!t.legal().trapped);
    // Run Away frees from traps in Champions
    let team = vec![
        set("garchomp", None, "runaway", ["earthquake", "outrage", "swordsdance", "fakeout"], 183),
        set("gholdengo", None, "goodasgold", ["makeitrain", "shadowball", "nastyplot", "recover"], 162),
        set("primarina", None, "torrent", ["moonblast", "hypervoice", "hyperbeam", "solarbeam"], 155),
    ];
    let mut t = T::from_team(team, [0, 1, 2], "garchomp", "gengar");
    t.e(&vol("own", "garchomp", "trapped"));
    t.turn(1);
    assert!(!t.legal().trapped);
}

#[test]
fn partial_trap_lasts_while_its_user_stays() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "firespin"), &vol("own", "garchomp", "firespin")]);
    t.turn(2);
    assert!(t.legal().trapped);
    t.es(&[r#"{"k":"withdraw","side":"opp","mon":"gengar"}"#, r#"{"k":"enter","side":"opp","mon":"corviknight"}"#]);
    t.turn(3);
    assert!(!t.legal().trapped);
}

#[test]
fn trapping_abilities_once_shown() {
    // Shadow Tag: known from the Mega's only ability
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    assert!(!t.legal().trapped);
    t.e(r#"{"k":"mega","side":"opp","mon":"gengar","form":"gengarmega"}"#);
    t.turn(2);
    assert!(t.legal().trapped);
    // Arena Trap: grounded only; Magnet Pull: Steel only
    let team = vec![
        set("garchomp", None, "roughskin", ["earthquake", "outrage", "swordsdance", "fakeout"], 183),
        set("corviknight", Some("leftovers"), "pressure", ["bravebird", "roost", "ironhead", "uturn"], 205),
        set("excadrill", Some("airballoon"), "sandrush", ["earthquake", "ironhead", "rapidspin", "swordsdance"], 185),
    ];
    let mut t = T::from_team(team.clone(), [0, 1, 2], "garchomp", "garchomp");
    t.e(r#"{"k":"ability","side":"opp","mon":"garchomp","ability":"arenatrap"}"#);
    t.turn(1);
    assert!(t.legal().trapped);
    t.es(&[r#"{"k":"enter","side":"own","mon":"corviknight","forced":true}"#]);
    assert!(!t.legal().trapped, "Flying is not grounded");
    t.es(&[r#"{"k":"enter","side":"own","mon":"excadrill","forced":true}"#]);
    assert!(!t.legal().trapped, "Air Balloon is not grounded");
    t.e(r#"{"k":"field","effect":"gravity","what":"start"}"#);
    assert!(t.legal().trapped, "Gravity grounds");
    let mut t = T::from_team(team, [0, 1, 2], "garchomp", "garchomp");
    t.e(r#"{"k":"ability","side":"opp","mon":"garchomp","ability":"magnetpull"}"#);
    t.turn(1);
    assert!(!t.legal().trapped);
    t.es(&[r#"{"k":"enter","side":"own","mon":"excadrill","forced":true}"#]);
    assert!(t.legal().trapped);
}

#[test]
fn fairy_lock_for_the_next_turn() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "fairylock"), r#"{"k":"field","effect":"fairylock","what":"start"}"#]);
    t.turn(2);
    assert!(t.legal().trapped);
    t.turn(3);
    assert!(!t.legal().trapped);
}

#[test]
fn refused_orders_are_illegal_for_the_request() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.e(r#"{"k":"refused","order":{"kind":"move","move":"outrage","mega":false}}"#);
    let l = t.legal();
    assert!(!l.mask[1] && !l.mask[5] && l.mask[0]);
    t.e(r#"{"k":"refused","order":{"kind":"switch","mon":"gholdengo"}}"#);
    assert_eq!(switch_mask(&t.legal()), [false; 3]);
    // the next turn asks afresh for moves; the switch stays refused while the Pokémon that trapped us by
    // something not shown stays in, and is free once it leaves
    t.e(&mv("own", "garchomp", "earthquake"));
    t.turn(2);
    let l = t.legal();
    assert!(l.mask[1]);
    assert_eq!(switch_mask(&l), [false; 3]);
    t.es(&[r#"{"k":"withdraw","side":"opp","mon":"gengar"}"#, r#"{"k":"enter","side":"opp","mon":"garchomp"}"#]);
    t.turn(3);
    assert_eq!(switch_mask(&t.legal()), [false, true, true]);
    // the request's `refused` field does the same
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.input(Input::Msg(msg(Body::DecisionRequest {
        seq: 3,
        kind: ReqKind::Turn,
        deadline_ms: None,
        refused: Some(Order::Move { mv: "earthquake".into(), mega: true }),
        retry: false,
        screen_offer: None,
    })));
    assert!(!t.legal().mask[0]);
    t.assert_clean();
}

fn request(t: &mut T, seq: i64, kind: ReqKind) {
    t.input(Input::Msg(msg(Body::DecisionRequest { seq, kind, deadline_ms: None, refused: None, retry: false, screen_offer: None })));
}

/// CF-8: a repeated tap is the same request again; it refuses nothing and keeps what was refused.
#[test]
fn retry_is_not_a_refusal() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    request(&mut t, 1, ReqKind::Turn);
    let before = t.legal().mask;
    let retry = |seq, refused| Body::DecisionRequest { seq, kind: ReqKind::Turn, deadline_ms: None, refused, retry: true, screen_offer: None };
    t.input(Input::Msg(msg(retry(2, None))));
    assert_eq!(t.legal().mask, before);
    assert_eq!(t.b.retries, 1);
    // a refusal, then a retry of the next order: the refused move stays out
    t.input(Input::Msg(msg(Body::DecisionRequest {
        seq: 3,
        kind: ReqKind::Turn,
        deadline_ms: None,
        refused: Some(Order::Move { mv: "earthquake".into(), mega: false }),
        retry: false,
        screen_offer: None,
    })));
    t.input(Input::Msg(msg(retry(4, None))));
    let l = t.legal();
    assert!(!l.mask[0] && l.mask[1]);
    assert_eq!(t.b.retries, 2);
    t.assert_clean();
}

#[test]
fn refused_switch_at_a_replace_request_is_only_that_target() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "moonblast"), r#"{"k":"faint","side":"own","mon":"garchomp"}"#]);
    request(&mut t, 1, ReqKind::Replace);
    t.input(Input::Msg(msg(Body::DecisionRequest {
        seq: 2,
        kind: ReqKind::Replace,
        deadline_ms: None,
        refused: Some(Order::Switch { mon: "gholdengo".into() }),
        retry: false,
        screen_offer: None,
    })));
    assert_eq!(switch_mask(&legal(t.bt(), ReqKind::Replace)), [false, false, true]);
    t.assert_clean();
}

#[test]
fn locked_move_lasts_until_shown_otherwise() {
    // turns with no row of ours (lost or skipped) keep the lock
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.e(&mv("own", "garchomp", "outrage"));
    t.turn(2);
    t.turn(3);
    assert_eq!(t.legal().forced, None);
    assert_eq!(moves_mask(&t.legal()), [false, true, false, false]);
    // our turn request ends it: the game shows no action screen while we are locked
    request(&mut t, 1, ReqKind::Turn);
    assert!(matches!(t.own_active().lock, Lock::None));
    assert_eq!(moves_mask(&t.legal()), [true, true, true, false]);
    // a replace request does not
    let mut t = T::new("garchomp", "corviknight");
    t.turn(1);
    t.e(&mv("own", "garchomp", "outrage"));
    request(&mut t, 1, ReqKind::Replace);
    assert!(matches!(t.own_active().lock, Lock::Locked { .. }));
    // the opponent's lock continues over turns, a different move ends it
    t.turn(2);
    t.e(&mv("opp", "corviknight", "outrage"));
    t.turn(3);
    t.turn(4);
    assert!(matches!(t.opp_active().lock, Lock::Locked { uses: 1, .. }));
    t.e(&mv("opp", "corviknight", "outrage"));
    assert!(matches!(t.opp_active().lock, Lock::Locked { uses: 2, .. }));
    assert_eq!(t.opp(2).pp_used[0], 1);
    t.turn(5);
    t.e(&mv("opp", "corviknight", "bravebird"));
    assert!(matches!(t.opp_active().lock, Lock::None));
    t.assert_clean();
}

#[test]
fn replace_after_a_faint() {
    let mut t = T::new("garchomp", "gengar");
    t.turn(1);
    t.es(&[&mv("opp", "gengar", "moonblast"), &hp_own("garchomp", 183, 0), r#"{"k":"faint","side":"own","mon":"garchomp"}"#]);
    let l = legal(t.bt(), ReqKind::Replace);
    assert_eq!(l.mask[..8], [false; 8]);
    assert_eq!(switch_mask(&l), [false, true, true]);
    t.e(r#"{"k":"enter","side":"own","mon":"golisopod"}"#);
    t.turn(2);
    t.es(&[&mv("opp", "gengar", "moonblast"), r#"{"k":"faint","side":"own","mon":"golisopod"}"#]);
    assert_eq!(switch_mask(&legal(t.bt(), ReqKind::Replace)), [false, true, false]);
}

#[test]
fn champions_has_no_assault_vest_and_one_choice_item() {
    assert!(ItemId::from_id("assaultvest").is_none());
    let choice: Vec<_> = ITEMS.iter().filter(|i| i.is_choice).map(|i| i.id).collect();
    assert_eq!(choice, ["choicescarf"]);
}
