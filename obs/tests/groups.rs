//! One hand-built Battle per feature group, through the builder, with expected values.

mod common;

use battle::*;
use common::*;
use obs::layout::*;
use obs::vocab;
use players::estimate::estimate;
use players::view::{Fighter, FieldView};
use cg_engine::sim::data::*;

fn sp(id: &str) -> i32 {
    vocab::species(SpeciesId::from_id(id))
}
fn m(id: &str) -> MoveId {
    MoveId::from_id(id).unwrap()
}

/// Our garchomp (team index 1) leads against their Rotom-Wash; turn 1 has begun.
fn start() -> G {
    let mut g = G::new(gold(), &PREVIEW);
    g.picks([1, 0, 3]);
    g.evs(vec![enter(own("garchomp")), enter(opp("rotomwash")), turn(1)]);
    g
}

fn no_anomalies(g: &G) {
    assert!(g.b.anomalies.is_empty(), "{:?}", g.b.anomalies);
}

#[test]
fn row_order_picks_first() {
    let mut g = start();
    let rows = obs::row_order(g.battle());
    let slots: Vec<Option<u8>> = rows.iter().map(|w| w.map(|w| w.slot)).collect();
    // own: the picks in pick order, then the rest in team order; opp: active, then preview order
    assert_eq!(slots, vec![Some(1), Some(0), Some(3), Some(2), Some(4), Some(5), Some(0), Some(1), Some(2), Some(3), Some(4), Some(5)]);
    assert!(rows[..6].iter().all(|w| w.unwrap().side == Side::Own));
    // after a switch to the second pick the rows stay put; only is_active moves
    g.evs(vec![withdraw(own("garchomp")), enter(own("gholdengo")), turn(2)]);
    no_anomalies(&g);
    assert_eq!(obs::row_order(g.battle()), rows);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 0, F3_9), row(&o, 1, F3_9), row(&o, 2, F3_9)), (0.0, 1.0, 0.0));
    // before the picks: team order
    let p = G::new(gold(), &PREVIEW);
    let slots: Vec<Option<u8>> = obs::row_order(p.battle())[..6].iter().map(|w| w.map(|w| w.slot)).collect();
    assert_eq!(slots, (0..6).map(Some).collect::<Vec<_>>());
}

/// A later preview fallback (S10-3): the rows follow the new picks from then on; picks that contradict
/// the log leave them as they were.
#[test]
fn row_order_follows_a_later_picks_fallback() {
    let mut g = start();
    g.picks([1, 4, 3]);
    let slots: Vec<Option<u8>> = obs::row_order(g.battle())[..6].iter().map(|w| w.map(|w| w.slot)).collect();
    assert_eq!(slots, vec![Some(1), Some(4), Some(3), Some(0), Some(2), Some(5)]);
    assert_eq!(g.b.corrections.len(), 1);
    no_anomalies(&g);
    g.picks([0, 1, 3]);
    assert_eq!(obs::row_order(g.battle())[1].map(|w| w.slot), Some(4));
    assert_eq!(g.b.anomalies.len(), 1);
}

/// Every legal switch action 8 + p names the Pokémon in row p.
#[test]
fn switch_actions_name_their_rows() {
    let mut rng = players::Rng::new(5);
    let mut checked = 0;
    for _ in 0..3000 {
        let (b, k) = players::gen::random_battle(&mut rng, false);
        let l = legal(&b, k);
        let rows = obs::row_order(&b);
        let mut o = Box::new(obs::ObsBuffers::default());
        obs::observe(&b, k, &mut o);
        for p in 0..3 {
            if !l.mask[8 + p] {
                continue;
            }
            let Some(Order::Switch { mon }) = order_for(&b, &l, 8 + p) else { panic!("switch {} has no order", p) };
            let w = rows[p].expect("legal switch to an empty row");
            let m = b.mon(w);
            assert_eq!(Some(mon.as_str()), m.species.map(|s| s.id()), "action {} vs row {}", 8 + p, p);
            assert_eq!(o.row_ids[p][F1_3], obs::vocab::species(m.form.or(m.species)));
            checked += 1;
        }
    }
    assert!(checked > 1000, "{}", checked);
}

#[test]
fn identity_and_build() {
    let mut g = start();
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    // our garchomp
    assert_eq!(o.row_ids[0][F1_3], sp("garchomp"));
    assert_eq!(row(&o, 0, F1_1 + Type::Dragon as usize), 1.0);
    assert_eq!(row(&o, 0, F1_1 + Type::Ground as usize), 1.0);
    assert_eq!(o.rows[0][F1_1..F1_1 + 18].iter().sum::<f32>(), 2.0);
    assert!(close(row(&o, 0, F1_2_1), 108.0 / 255.0) && close(row(&o, 0, F1_2_6), 102.0 / 255.0));
    assert_eq!((row(&o, 0, F2_1), row(&o, 0, F2_2), row(&o, 0, F2_3), row(&o, 0, F2_5)), (1.0, 0.0, 1.0, 1.0));
    let st = stats_of("garchomp", "jolly", [2, 32, 0, 0, 0, 32]);
    for i in 0..6 {
        assert!(close(row(&o, 0, F2_4_1 + i), st[i] as f32 / 400.0));
    }
    assert_eq!(o.row_ids[0][F2_6], ItemId::from_id("garchompitez").unwrap().0 as i32 + 1);
    assert_eq!(o.row_ids[0][F2_7], AbilityId::from_id("roughskin").unwrap().0 as i32);
    assert_eq!(&o.row_ids[0][F8_10..F8_10 + 3], &[AbilityId::from_id("roughskin").unwrap().0 as i32, 0, 0]);
    // golisopod holds nothing: known, gone, id "none"
    assert_eq!(o.row_ids[3][F1_3], sp("golisopod"));
    assert_eq!((row(&o, 3, F2_1), row(&o, 3, F2_2)), (1.0, 1.0));
    assert_eq!(o.row_ids[3][F2_6], vocab::ITEM_NONE);
    // their rotom: nothing revealed
    assert_eq!(o.row_ids[6][F1_3], sp("rotomwash"));
    assert_eq!(row(&o, 6, F1_1 + Type::Electric as usize) + row(&o, 6, F1_1 + Type::Water as usize), 2.0);
    assert_eq!((row(&o, 6, F2_1), row(&o, 6, F2_2), row(&o, 6, F2_3), row(&o, 6, F2_5)), (0.0, 0.0, 0.0, 0.0));
    assert!(o.rows[6][F2_4_1..F2_4_1 + 6].iter().all(|x| *x == 0.0));
    assert_eq!((o.row_ids[6][F2_6], o.row_ids[6][F2_7]), (vocab::ITEM_UNKNOWN, 0));
    let abs = SpeciesId::from_id("rotomwash").unwrap().data().abilities;
    assert_eq!(o.row_ids[6][F8_10], abs[0].0 as i32);
    assert_eq!(row(&o, 6, F3_1), 1.0);
    // an unseen preview member: species facts from the preview, HP full
    assert_eq!(o.row_ids[7][F1_3], sp("snorlax"));
    assert_eq!((row(&o, 7, F3_1), row(&o, 7, F4_1)), (1.0, 0.0));

    // item and ability shown, then the item knocked off
    g.ev(Event::Item { who: Some(opp("rotomwash")), item: "leftovers".into(), what: ItemWhat::Activated, of: None });
    g.ev(Event::Ability { who: opp("rotomwash"), ability: "levitate".into() });
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 6, F2_1), row(&o, 6, F2_3)), (1.0, 1.0));
    assert_eq!(o.row_ids[6][F2_6], ItemId::from_id("leftovers").unwrap().0 as i32 + 1);
    assert_eq!(o.row_ids[6][F2_7], AbilityId::from_id("levitate").unwrap().0 as i32);
    g.ev(Event::Item { who: Some(opp("rotomwash")), item: "leftovers".into(), what: ItemWhat::Removed, of: None });
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 6, F2_1), row(&o, 6, F2_2), o.row_ids[6][F2_6]), (1.0, 1.0, vocab::ITEM_NONE));
    no_anomalies(&g);
}

#[test]
fn condition() {
    let mut g = start();
    let hp = stats_of("garchomp", "jolly", [2, 32, 0, 0, 0, 32])[0];
    g.evs(vec![
        mv(own("garchomp"), "protect"),
        mv(opp("rotomwash"), "hydropump"),
        Event::Protect { who: own("garchomp") },
        boost(own("garchomp"), "atk", 2),
        boost(opp("rotomwash"), "spe", -1),
        boost(opp("rotomwash"), "accuracy", 1),
        Event::Status { who: opp("rotomwash"), status: "tox".into(), effect: None },
        vol(opp("rotomwash"), "taunt", None),
        vol(opp("rotomwash"), "whirlpool", None),
        vol(own("garchomp"), "perishsong", Some(Detail::Num(2))),
        hp_own("garchomp", hp, hp / 2),
        turn(2),
        mv(own("garchomp"), "protect"),
        turn(3),
    ]);
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    assert!(close(row(&o, 0, F3_1), (hp / 2) as f32 / hp as f32));
    assert!(close(row(&o, 0, F3_4_1), 0.5));
    assert!(close(row(&o, 6, F3_4_5), (2.0f32 / 3.0).log2() / 2.0));
    assert!(close(row(&o, 6, F3_4_6), (4.0f32 / 3.0).log2() / 2.0));
    assert_eq!(row(&o, 6, F3_2 + 5), 1.0);
    assert!(close(row(&o, 6, F3_3), 2.0 / 5.0));
    let v = |r: usize, name: &str| row(&o, r, F3_5 + obs::vols::VOLS.iter().position(|(n, _)| *n == name).unwrap());
    assert_eq!((v(6, "taunt"), v(6, "partiallytrapped"), v(6, "whirlpool"), v(6, "bind")), (1.0, 1.0, 1.0, 0.0));
    assert_eq!((v(0, "perish2"), v(0, "perish3")), (1.0, 0.0));
    assert_eq!(o.rows[0][F3_5..F3_5 + 87].iter().sum::<f32>(), 1.0);
    // two protects in a row, on the field since the start, has moved
    assert!(close(row(&o, 0, F3_7), 2.0 / 3.0));
    assert!(close(row(&o, 0, F3_8), 0.3));
    assert_eq!((row(&o, 0, F3_9), row(&o, 1, F3_9), row(&o, 0, F3_6_1)), (1.0, 0.0, 0.0));
    // bench rows carry no boosts or volatiles
    assert!(o.rows[1][F3_4_1..F3_6_1].iter().all(|x| *x == 0.0));

    // sleep counter, Mega Evolution
    g.evs(vec![
        Event::MegaStone { who: own("garchomp"), stone: "garchompitez".into() },
        Event::Mega { who: own("garchomp"), form: None },
        Event::Status { who: opp("rotomwash"), status: "slp".into(), effect: None },
        Event::Cant { who: Some(opp("rotomwash")), reason: "slp".into() },
        turn(4),
        Event::Cant { who: Some(opp("rotomwash")), reason: "slp".into() },
    ]);
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    assert!(close(row(&o, 6, F3_3), 0.4));
    assert_eq!((row(&o, 0, F3_11), o.row_ids[0][F1_3]), (1.0, sp("garchompmegaz")));
    let mz = SpeciesId::from_id("garchompmegaz").unwrap().data();
    assert!(close(row(&o, 0, F1_2_6), mz.base_stats[5] as f32 / 255.0));
    assert_eq!(o.rows[0][F1_1..F1_1 + 18].iter().sum::<f32>(), 1.0);
}

#[test]
fn first_turn_out_and_preparing() {
    let mut g = start();
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 0, F3_6_1), row(&o, 6, F3_6_1)), (1.0, 1.0));
    g.evs(vec![Event::Prepare { who: opp("rotomwash"), mv: Some("solarbeam".into()) }, turn(2)]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 6, F3_6_2), row(&o, 6, F3_6_3)), (1.0, 0.0));
    g.evs(vec![mv(opp("rotomwash"), "solarbeam"), mv(own("garchomp"), "gigaimpact"), turn(3)]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 6, F3_6_2), row(&o, 0, F3_6_3)), (0.0, 1.0));
}

#[test]
fn knowledge_and_evidence() {
    let mut g = start();
    g.evs(vec![
        mv(own("garchomp"), "earthquake"),
        hp_opp("rotomwash", 100.0, 40.0),
        mv(opp("rotomwash"), "hydropump"),
        turn(2),
        withdraw(opp("rotomwash")),
        enter(opp("snorlax")),
        mv(own("garchomp"), "earthquake"),
        hp_opp("snorlax", 100.0, 70.0),
        turn(3),
        withdraw(opp("snorlax")),
        enter(opp("rotomwash")),
        turn(4),
        withdraw(opp("rotomwash")),
        enter(opp("glimmora")),
        turn(5),
    ]);
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    let rows = obs::row_order(g.battle());
    assert_eq!(rows[6].unwrap().slot, 2);
    assert_eq!((rows[7].unwrap().slot, rows[8].unwrap().slot), (0, 1));
    // three seen: the other three were not brought
    for r in 6..9 {
        assert_eq!((row(&o, r, F4_1), row(&o, r, F4_2)), (1.0, 0.0));
    }
    for r in 9..12 {
        assert_eq!((row(&o, r, F4_1), row(&o, r, F4_2)), (0.0, 1.0));
    }
    assert_eq!((row(&o, 7, F4_4), row(&o, 6, F4_4), row(&o, 0, F4_4)), (1.0, 0.0, 1.0));
    assert!(close(row(&o, 7, F6_1), 0.6) && close(row(&o, 8, F6_1), 0.3));
    // turn 1: we moved first, they second
    assert!(close(row(&o, 0, F6_2_1), 0.2) && close(row(&o, 0, F6_2_2), 0.0));
    assert!(close(row(&o, 7, F6_2_2), 0.2));
    assert!(close(row(&o, 7, F6_3), 0.5) && close(row(&o, 0, F6_3), 0.25));
    // own bench: brought from the picks
    assert_eq!((row(&o, 1, F4_1), row(&o, 2, F4_1), row(&o, 3, F4_1)), (1.0, 1.0, 0.0));
}

#[test]
fn move_facts_and_state() {
    let mut g = start();
    g.evs(vec![mv(own("garchomp"), "earthquake"), mv(opp("rotomwash"), "hydropump"), turn(2), mv(own("garchomp"), "dragonclaw")]);
    g.ev(vol(own("garchomp"), "encore", None));
    g.ev(mv(opp("rotomwash"), "disable"));
    g.ev(vol(own("garchomp"), "disable", Some(Detail::Id("earthquake".into()))));
    g.ev(vol(opp("rotomwash"), "taunt", None));
    g.ev(turn(3));
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    // earthquake: ground, physical, 100 power, sure hit
    assert_eq!((slot(&o, 0, 0, F7_1 + Type::Ground as usize), slot(&o, 0, 0, F7_2)), (1.0, 1.0));
    assert!(close(slot(&o, 0, 0, F7_3), 100.0 / 150.0) && close(slot(&o, 0, 0, F7_4), 1.0));
    let eq = m("earthquake").data();
    assert!(close(slot(&o, 0, 0, F7_7), (eq.max_pp - 1) as f32 / eq.max_pp as f32));
    assert_eq!((slot(&o, 0, 0, F7_8_1), slot(&o, 0, 1, F7_8_1)), (1.0, 0.0));
    assert_eq!((slot(&o, 0, 1, F7_8_2), slot(&o, 0, 0, F7_8_2)), (1.0, 0.0));
    // swords dance: status, setup; protect: priority 4
    assert_eq!((slot(&o, 0, 2, F7_2 + 2), slot(&o, 0, 2, F7_6_2)), (1.0, 1.0));
    assert!(close(slot(&o, 0, 3, F7_5), 0.8));
    assert_eq!(o.row_ids[0][F7_13..F7_13 + 4], [m("earthquake").0 as i32, m("dragonclaw").0 as i32, m("swordsdance").0 as i32, m("protect").0 as i32]);
    // golisopod's U-turn pivots; gholdengo's Recover heals
    assert_eq!(slot(&o, 3, 0, F7_6_1), 1.0);
    assert_eq!(slot(&o, 1, 3, F7_6_3), 1.0);
    assert_eq!((slot(&o, 1, 0, F7_8_1), slot(&o, 1, 0, F7_8_2)), (0.0, 0.0));
    // theirs: two moves revealed in order of use
    assert_eq!(o.row_ids[6][F7_13..F7_13 + 4], [m("hydropump").0 as i32, m("disable").0 as i32, 0, 0]);
    assert_eq!((slot(&o, 6, 0, F7_8_3), slot(&o, 6, 2, F7_8_3)), (1.0, 0.0));
    let hp = m("hydropump").data();
    assert!(close(slot(&o, 6, 0, F7_7), (hp.max_pp - 1) as f32 / hp.max_pp as f32));
    assert!(o.rows[6][MOVE_BASE + 2 * MOVE_STRIDE..MOVE_BASE + 4 * MOVE_STRIDE].iter().all(|x| *x == 0.0));
    // timers: Encore and Taunt began on turn 2
    assert!(close(row(&o, 0, F9_1_1), 1.0 / 3.0) && close(row(&o, 0, F9_1_2), 0.0));
    assert!(close(row(&o, 6, F9_1_2), 1.0 / 3.0));
    // their Encore shown: locked on their active too
    let mut g2 = start();
    g2.evs(vec![mv(opp("rotomwash"), "hydropump"), turn(2)]);
    g2.ev(vol(opp("rotomwash"), "encore", None));
    let o = g2.obs(ReqKind::Turn);
    assert_eq!(slot(&o, 6, 0, F7_8_2), 1.0);
}

#[test]
fn choice_lock_shows_as_locked() {
    let mut g = G::new(gold(), &PREVIEW);
    g.picks([0, 1, 3]);
    g.evs(vec![enter(own("gholdengo")), enter(opp("rotomwash")), turn(1), mv(own("gholdengo"), "makeitrain"), turn(2)]);
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((slot(&o, 0, 0, F7_8_2), slot(&o, 0, 1, F7_8_2)), (1.0, 0.0));
}

#[test]
fn dex_hits_and_multipliers() {
    assert!(close(obs::dex_hits(m("bulletseed")), 3.1));
    assert!(close(obs::dex_hits(m("tripleaxel")), 6.0));
    assert!(close(obs::dex_hits(m("earthquake")), 1.0));
    assert!(close(obs::type_multiplier(Type::Ground, [Type::Electric, Type::Water]), 2.0));
    assert!(close(obs::type_multiplier(Type::Ground, [Type::Flying, Type::Typeless]), 0.0));
    assert!(close(obs::type_multiplier(Type::Ice, [Type::Dragon, Type::Ground]), 4.0));
}

#[test]
fn damage_and_speed() {
    let mut g = start();
    g.evs(vec![mv(opp("rotomwash"), "hydropump"), mv(opp("rotomwash"), "voltswitch"), turn(2)]);
    let b = g.battle().clone();
    let o = g.obs(ReqKind::Turn);
    let field = FieldView::of(&b);
    let ours = Fighter::of(&b, Who { side: Side::Own, slot: 1 }, false);
    let theirs = Fighter::of(&b, Who { side: Side::Opp, slot: 0 }, false);
    let h = estimate(&field, &ours, m("earthquake"), &theirs, Some(&b));
    assert!(h.mid > 0);
    assert!(close(slot(&o, 0, 0, F7_10_1), h.expected().min(2.0)));
    assert!(close(slot(&o, 0, 0, F7_10_2), h.max_frac().min(2.0)));
    assert_eq!(slot(&o, 0, 0, F7_11_2), (h.max_frac() >= 1.0) as u8 as f32);
    assert!(close(slot(&o, 0, 0, F7_9), 0.5));
    // a status move has no effectiveness and no damage
    assert_eq!((slot(&o, 0, 2, F7_9), slot(&o, 0, 2, F7_10_1)), (0.0, 0.0));
    // incoming on our active: the best of their two revealed moves
    let a = estimate(&field, &theirs, m("hydropump"), &ours, Some(&b));
    let c = estimate(&field, &theirs, m("voltswitch"), &ours, Some(&b));
    assert!(close(row(&o, 0, F5_3_1), a.expected().max(c.expected()).min(2.0)));
    assert!(close(row(&o, 0, F5_3_2), a.max_frac().max(c.max_frac()).min(2.0)));
    // their moves against our active, on their row
    assert!(close(slot(&o, 6, 0, F7_10_1), a.expected().min(2.0)));
    assert_eq!(slot(&o, 6, 1, F7_9), 0.0); // electric into ground
    // a bench row: incoming computed against it too
    let gh = Fighter::of(&b, Who { side: Side::Own, slot: 0 }, false);
    let x = estimate(&field, &theirs, m("hydropump"), &gh, Some(&b)).expected();
    let y = estimate(&field, &theirs, m("voltswitch"), &gh, Some(&b)).expected();
    assert!(close(row(&o, 1, F5_3_1), x.max(y).min(2.0)));

    // speed: jolly 32-point garchomp against snorlax's whole range (scarf included) and Trick Room
    let mut g = G::new(gold(), &PREVIEW);
    g.picks([1, 0, 3]);
    g.evs(vec![enter(own("garchomp")), enter(opp("snorlax")), turn(1)]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 0, F5_1_1), row(&o, 0, F5_1_2)), (1.0, 0.0));
    assert_eq!((row(&o, 6, F5_1_1), row(&o, 6, F5_1_2)), (0.0, 1.0));
    g.evs(vec![Event::Field { effect: "trickroom".into(), what: StartEnd::Start }]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 0, F5_1_1), row(&o, 0, F5_1_2)), (0.0, 1.0));
    // against an even match nothing is sure
    let mut g = G::new(gold(), &PREVIEW);
    g.picks([1, 0, 3]);
    g.evs(vec![enter(own("garchomp")), enter(opp("dragonite")), turn(1)]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((row(&o, 0, F5_1_1), row(&o, 0, F5_1_2)), (0.0, 0.0));
}

#[test]
fn field_block() {
    let mut g = start();
    g.evs(vec![
        Event::Weather { weather: "sunnyday".into(), what: StartEnd::Start },
        side_start(Side::Own, "reflect"),
        mv(own("garchomp"), "protect"),
        mv(opp("rotomwash"), "futuresight"),
        side_start(Side::Opp, "spikes"),
        side_start(Side::Opp, "spikes"),
        turn(2),
        Event::Field { effect: "grassyterrain".into(), what: StartEnd::Start },
        Event::Field { effect: "trickroom".into(), what: StartEnd::Start },
        side_start(Side::Opp, "stealthrock"),
        side_start(Side::Own, "tailwind"),
        mv(own("garchomp"), "swordsdance"),
        mv(opp("rotomwash"), "wish"),
        turn(3),
    ]);
    let o = g.obs(ReqKind::Turn);
    let f = &o.field;
    assert!(close(f[G1_1], 0.1));
    assert_eq!((f[G1_2_1], f[G1_2_2]), (0.0, 0.0));
    assert_eq!((f[G1_4_1], f[G1_4_2]), (1.0, 1.0));
    assert!(close(f[G1_5], 1.0 / 3.0));
    let l = legal(g.battle(), ReqKind::Turn);
    assert!(close(f[G1_7_1], l.mask[..4].iter().filter(|x| **x).count() as f32 / 4.0));
    assert!(close(f[G1_7_2], 1.0));
    assert_eq!((f[G2_1], f[G2_1 + 1], f[G2_2]), (1.0, 0.0, 0.0));
    assert!(close(f[G9_1], 2.0 / 8.0));
    assert_eq!(f[G2_4 + 1], 1.0);
    assert!(close(f[G2_5], 1.0 / 8.0));
    assert_eq!((f[G2_6_1], f[G2_6_2]), (1.0, 0.0));
    assert!(close(f[G2_7], 0.2));
    assert_eq!((f[G3_1_1], f[G3_1_4], f[G3_1_6]), (1.0, 1.0, 0.0));
    assert!(close(f[G3_2_1], 2.0 / 8.0) && close(f[G3_2_4], 1.0 / 4.0));
    assert!(close(f[G3_3_8], 2.0 / 3.0));
    assert_eq!((f[G3_3_7], f[G3_1_7]), (1.0, 0.0));
    // tendencies over two turns: ours protect + setup, theirs two attacks
    assert!(close(f[G6_1_2], 0.5) && close(f[G6_1_3], 0.5) && close(f[G6_1_4], 0.0));
    assert!(close(f[G6_2_4], 0.5) && close(f[G6_2_1], 0.0));
    // their Wish (turn 2) heals them this turn; their Future Sight (turn 1) lands on us at its end
    assert_eq!((f[G9_3_1], f[G9_2_1]), (1.0, 0.0));
    assert!(close(f[G9_2_2], 0.5) && close(f[G9_3_2], 0.0));
    g.evs(vec![Event::Cause { effect: "wish".into(), who: Some(opp("rotomwash")), of: None }, turn(4)]);
    let o = g.obs(ReqKind::Turn);
    assert_eq!((o.field[G9_3_1], o.field[G9_2_2]), (0.0, 0.0));
    no_anomalies(&g);

    // a faint and a replace request
    g.evs(vec![Event::Faint { who: own("garchomp") }]);
    let o = g.obs(ReqKind::Replace);
    assert_eq!(o.field[G1_2_1], 1.0);
    assert!(close(o.field[G1_4_1], 2.0 / 3.0));
    assert_eq!(o.mask, [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    assert_eq!(row(&o, 0, F3_9), 0.0);
}

#[test]
fn trapped_flag() {
    let mut g = start();
    g.ev(vol(own("garchomp"), "whirlpool", None));
    let o = g.obs(ReqKind::Turn);
    assert_eq!(o.field[G1_2_2], 1.0);
    assert_eq!(&o.mask[8..], &[0, 0, 0]);
}

#[test]
fn last_turn_block() {
    let mut g = start();
    let hp = stats_of("garchomp", "jolly", [2, 32, 0, 0, 0, 32])[0];
    g.evs(vec![
        mv(own("garchomp"), "earthquake"),
        hp_opp("rotomwash", 100.0, 40.0),
        mv(opp("rotomwash"), "hydropump"),
        hp_own("garchomp", hp, hp - hp / 4),
        Event::Item { who: Some(opp("rotomwash")), item: "leftovers".into(), what: ItemWhat::Activated, of: None },
        hp_opp("rotomwash", 40.0, 46.0),
        Event::Ability { who: opp("rotomwash"), ability: "levitate".into() },
    ]);
    // before the turn ends: nothing yet in the last-turn block
    let o = g.obs(ReqKind::Turn);
    assert_eq!((o.last[L1_1 + 2], o.last[L1_4 + 2], o.last[L3_2]), (1.0, 1.0, 1.0));
    g.ev(turn(2));
    no_anomalies(&g);
    let o = g.obs(ReqKind::Turn);
    let l = &o.last;
    assert_eq!((l[L1_1], l[L1_4], l[L1_2_2], l[L1_5_2]), (1.0, 1.0, 1.0, 0.0));
    assert!(close(l[L1_6_1], 0.6));
    assert!(close(l[L1_3_1], (hp / 4) as f32 / hp as f32));
    assert_eq!((l[L1_6_3], l[L1_6_4], l[L1_3_3], l[L1_3_4]), (1.0, 1.0, 0.0, 0.0));
    assert_eq!((l[L3_2], l[L1_3_2]), (0.0, 0.0));
    assert_eq!(o.last_ids, [m("earthquake").0 as i32, 0, m("hydropump").0 as i32, 0]);
    // a switch, and our faint
    g.evs(vec![withdraw(opp("rotomwash")), enter(opp("snorlax")), mv(own("garchomp"), "earthquake"), Event::Faint { who: own("garchomp") }, turn(3)]);
    let o = g.obs(ReqKind::Replace);
    assert_eq!((o.last[L1_4 + 1], o.last_ids[L2_2_2]), (1.0, sp("snorlax")));
    assert!(close(o.last[L1_3_2], 1.0 / 3.0));
    assert_eq!((o.last[L1_2_2], o.last[L1_5_2]), (0.0, 0.0));
}

#[test]
fn mask_is_the_legal_rule() {
    let mut g = start();
    g.evs(vec![mv(own("garchomp"), "earthquake"), turn(2)]);
    for kind in [ReqKind::Turn, ReqKind::Replace] {
        let o = g.obs(kind);
        let l = legal(g.battle(), kind);
        assert_eq!(o.mask.map(|x| x == 1), l.mask);
    }
}

#[test]
fn learnset_table() {
    let mut t = vec![0u8; vocab::SPECIES_VOCAB * vocab::MOVE_VOCAB];
    obs::learnset_table(&mut t);
    let g = SpeciesId::from_id("garchomp").unwrap();
    assert!(obs::can_learn(g, m("earthquake")));
    assert!(!obs::can_learn(g, m("makeitrain")));
    assert_eq!(t[(g.0 as usize + 1) * vocab::MOVE_VOCAB + m("earthquake").0 as usize], 1);
    assert!(t[..vocab::MOVE_VOCAB].iter().all(|x| *x == 0));
    // a Mega uses its base species' learnset
    let mz = SpeciesId::from_id("garchompmegaz").unwrap();
    assert!(obs::can_learn(mz, m("earthquake")));
    assert_eq!((vocab::SPECIES_VOCAB, vocab::MOVE_VOCAB, vocab::ITEM_VOCAB, vocab::ABILITY_VOCAB), (393, 516, 168, 319));
}
