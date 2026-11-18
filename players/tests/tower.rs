//! Tower, spec test list 20-48 (49-50 are the pivot rule, not built).

mod common;
use battle::*;
use common::*;
use players::*;
use cg_engine::sim::data::*;

fn vs(team: Vec<SetMsg>, opp: &str) -> T {
    let mut pv = vec![opp];
    for p in PREVIEW {
        if p != opp && pv.len() < 6 {
            pv.push(p);
        }
    }
    T::new(fill(team), &pv)
}

fn only(c: &[u32; N_ACTIONS], allowed: &[usize]) -> bool {
    (0..N_ACTIONS).all(|i| c[i] == 0 || allowed.contains(&i))
}

fn chomp(moves: &[&str]) -> SetMsg {
    set("garchomp", None, "roughskin", moves)
}

#[test]
fn t20_perish_one_switches_uniformly() {
    let mut t = vs(vec![chomp(&["earthquake", "dragonclaw", "protect", "rest"])], "gengar");
    t.e(r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"perishsong","detail":1}"#);
    let c = t.tower_counts(400);
    assert!(only(&c, &[9, 10]), "{:?}", c);
    assert!(c[9] > 150 && c[10] > 150, "{:?}", c);
    // count 2: no escape yet
    let mut t = vs(vec![chomp(&["earthquake", "dragonclaw", "protect", "rest"])], "gengar");
    t.e(r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"perishsong","detail":2}"#);
    assert!(t.tower(1) < 8);
}

#[test]
fn t21_perish_one_but_trapped_uses_the_move_rules() {
    let mut t = vs(vec![chomp(&["earthquake", "dragonclaw", "protect", "rest"])], "gengar");
    t.e(r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"perishsong","detail":1}"#);
    t.e(&vol("own", "garchomp", "trapped"));
    assert!(t.legal().trapped);
    let c = t.tower_counts(100);
    assert!(only(&c, &[0, 1, 2, 3]), "{:?}", c);
}

#[test]
fn t22_a_ko_beats_an_available_status_move() {
    let mut t = vs(vec![chomp(&["thunderwave", "earthquake", "protect", "rest"])], "gengar");
    t.e(&hp_opp("gengar", 100.0, 5.0));
    assert_eq!(t.sorted().ko, vec![1]);
    let c = t.tower_counts(100);
    assert_eq!(c[1], 100, "{:?}", c);
}

#[test]
fn t23_priority_ko_first_else_uniform_among_kos() {
    let mut t = vs(vec![chomp(&["iceshard", "earthquake", "dragonclaw", "protect"])], "gengar");
    t.e(&hp_opp("gengar", 100.0, 5.0));
    assert_eq!(t.sorted().ko, vec![0, 1, 2]);
    assert_eq!(t.tower_counts(100)[0], 100);
    let mut t = vs(vec![chomp(&["earthquake", "dragonclaw", "protect", "rest"])], "gengar");
    t.e(&hp_opp("gengar", 100.0, 5.0));
    let c = t.tower_counts(400);
    assert!(only(&c, &[0, 1]) && c[0] > 150 && c[1] > 150, "{:?}", c);
}

#[test]
fn t24_ko_means_expected_damage_at_least_hp() {
    let t0 = vs(vec![chomp(&["dragonclaw", "protect", "rest", "swordsdance"])], "dragonite");
    let e = t0.exp(0) as f64 * 100.0;
    assert!(e > 2.0 && e < 98.0, "{}", e);
    let mut t = vs(vec![chomp(&["dragonclaw", "protect", "rest", "swordsdance"])], "dragonite");
    t.e(&hp_opp("dragonite", 100.0, (e - 1.0).floor()));
    assert_eq!(t.sorted().ko, vec![0]);
    let mut t = vs(vec![chomp(&["dragonclaw", "protect", "rest", "swordsdance"])], "dragonite");
    t.e(&hp_opp("dragonite", 100.0, (e + 1.0).ceil()));
    assert!(t.sorted().ko.is_empty());
    // accuracy counts: Stone Edge (80%) needs its expected value, not its hit, to reach the HP
    let t1 = vs(vec![chomp(&["stoneedge", "protect", "rest", "swordsdance"])], "dragonite");
    let b = t1.bt();
    let h = Estimator::new(b).hit(b, t1.own(), false, MoveId::from_id("stoneedge").unwrap(), t1.opp());
    assert!((h.expected() - h.frac() * 0.8).abs() < 1e-6);
}

#[test]
fn t25_zero_damage_is_never_a_ko() {
    let mut t = vs(vec![chomp(&["earthquake", "protect", "rest", "swordsdance"])], "corviknight");
    t.e(&hp_opp("corviknight", 100.0, 1.0));
    let s = t.sorted();
    assert!(s.ko.is_empty() && s.best.is_none(), "{:?}", s);
}

fn lax(moves: &[&str]) -> SetMsg {
    set("snorlax", None, "thickfat", moves)
}

#[test]
fn t26_slower_with_a_speed_down_attack_goes_high() {
    let t = vs(vec![lax(&["icywind", "bodyslam", "protect", "rest"])], "dragapult");
    assert!(t.sorted().high.contains(&0));
}

#[test]
fn t27_faster_speed_down_is_not_forced_high() {
    let t = vs(vec![chomp(&["icywind", "dragonclaw", "protect", "rest"])], "snorlax");
    assert!(!t.sorted().high.contains(&0));
}

#[test]
fn t28_equal_speed_is_not_faster_and_trick_room_reverses() {
    let t = vs(vec![chomp(&["icywind", "dragonclaw", "protect", "rest"])], "garchomp");
    let b = t.bt();
    let f = FieldView::of(b);
    assert_eq!(view::speed(&Fighter::of(b, t.own(), false), &f), view::speed(&Fighter::of(b, t.opp(), false), &f));
    assert!(t.sorted().high.contains(&0));
    let mut t = vs(vec![lax(&["icywind", "bodyslam", "protect", "rest"])], "dragapult");
    t.e(&field("trickroom"));
    assert!(!t.sorted().high.contains(&0));
}

#[test]
fn t29_fake_out_on_the_first_turn_only() {
    let mut t = vs(vec![set("cinderace", None, "blaze", &["fakeout", "pyroball", "protect", "rest"])], "dragonite");
    assert!(t.sorted().high.contains(&0));
    t.e(&mv("own", "cinderace", "pyroball"));
    t.turn(2);
    assert!(!t.sorted().usable.contains(&0));
    assert!(!t.legal().mask[0]);
}

#[test]
fn t30_setup_high_only_when_we_survive() {
    let mut t = vs(vec![chomp(&["swordsdance", "dragonclaw", "protect", "rest"])], "dragonite");
    t.e(&mv("opp", "dragonite", "aquajet"));
    assert!(t.sorted().high.contains(&0));
    let gmax = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("garchomp", gmax, gmax / 10));
    let s = t.sorted();
    assert!(!s.high.contains(&0) && s.status.contains(&0), "{:?}", s);
}

#[test]
fn t31_setup_with_every_raised_stat_at_six_is_dropped() {
    let mut t = vs(vec![chomp(&["swordsdance", "dragonclaw", "dragondance", "rest"])], "dragonite");
    t.e(&boost("own", "garchomp", "atk", 6));
    let s = t.sorted();
    assert!(!s.usable.contains(&0), "{:?}", s);
    // Dragon Dance raises speed too, not yet +6: kept
    assert!(s.usable.contains(&2));
}

#[test]
fn t32_no_revealed_damaging_move_setup_is_safe() {
    let mut t = vs(vec![chomp(&["swordsdance", "dragonclaw", "protect", "rest"])], "dragonite");
    let gmax = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("garchomp", gmax, 2));
    t.e(&mv("opp", "dragonite", "dragondance"));
    assert!(t.sorted().high.contains(&0));
    let opts = Options { setup_safe_without_revealed_attacks: false, ..Options::default() };
    let s = tower::inspect(t.bt(), ReqKind::Turn, &opts).unwrap();
    assert!(!s.high.contains(&0));
}

#[test]
fn t33_thunder_wave_on_an_unstatused_foe_is_high_either_way() {
    // T-6: an INFLICT move on a foe with no status is high whether or not we are faster (the spec's test
    // 33 expects `status` when faster; the rules it lists say high)
    let t = vs(vec![chomp(&["thunderwave", "dragonclaw", "protect", "rest"])], "snorlax");
    assert!(t.sorted().high.contains(&0));
    let t = vs(vec![lax(&["thunderwave", "bodyslam", "protect", "rest"])], "dragapult");
    assert!(t.sorted().high.contains(&0));
}

#[test]
fn t34_thunder_wave_on_a_statused_foe_is_dropped() {
    let mut t = vs(vec![chomp(&["thunderwave", "dragonclaw", "protect", "rest"])], "snorlax");
    t.e(&status("opp", "snorlax", "brn"));
    assert!(!t.sorted().usable.contains(&0));
}

#[test]
fn t35_toxic_dropped_confuse_ray_high_and_not_repeated() {
    let mut t = vs(vec![set("gengar", None, "cursedbody", &["toxic", "confuseray", "shadowball", "protect"])], "snorlax");
    t.e(&status("opp", "snorlax", "par"));
    let s = t.sorted();
    assert!(!s.usable.contains(&0) && s.high.contains(&1), "{:?}", s);
    t.e(&vol("opp", "snorlax", "confusion"));
    assert!(!t.sorted().usable.contains(&1), "PS-B6");
    let opts = Options { skip_repeat_volatile: false, ..Options::default() };
    assert!(tower::inspect(t.bt(), ReqKind::Turn, &opts).unwrap().high.contains(&1));
}

#[test]
fn t36_taunted_drops_every_status_move() {
    let mut t = vs(vec![chomp(&["swordsdance", "dragonclaw", "stealthrock", "rest"])], "dragonite");
    t.e(&vol("own", "garchomp", "taunt"));
    assert_eq!(t.sorted().usable, vec![1]);
}

#[test]
fn t37_dream_eater_is_not_in_champions() {
    assert!(MoveId::from_id("dreameater").is_none());
}

#[test]
fn t38_screens_already_up_are_dropped() {
    let mut t = vs(vec![set("gholdengo", None, "goodasgold", &["reflect", "lightscreen", "makeitrain", "auroraveil"])], "dragonite");
    let s = t.sorted();
    assert!(s.high.contains(&0) && s.high.contains(&1) && s.high.contains(&3));
    t.e(&side_cond("own", "reflect"));
    t.e(&side_cond("own", "auroraveil"));
    let s = t.sorted();
    assert!(!s.usable.contains(&0) && s.usable.contains(&1) && !s.usable.contains(&3), "{:?}", s);
}

#[test]
fn t39_recovery_at_full_hp_dropped_at_98_kept() {
    let mut t = vs(vec![set("gholdengo", None, "goodasgold", &["recover", "makeitrain", "protect", "nastyplot"])], "dragonite");
    t.e(&mv("opp", "dragonite", "extremespeed"));
    assert!(!t.sorted().usable.contains(&0));
    let max = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("gholdengo", max, (max as f32 * 0.98).floor() as i32));
    let hp = t.bt().mon(t.own()).hp;
    assert!(hp < 99.0 && hp >= 97.0, "{}", hp);
    assert!(t.sorted().status.contains(&0));
}

#[test]
fn t40_hazard_caps() {
    let mut t = vs(vec![chomp(&["stealthrock", "spikes", "dragonclaw", "rest"])], "dragonite");
    let s = t.sorted();
    assert!(s.status.contains(&0) && s.status.contains(&1));
    t.e(&side_cond("opp", "stealthrock"));
    t.e(&side_cond("opp", "spikes"));
    t.e(&side_cond("opp", "spikes"));
    let s = t.sorted();
    assert!(!s.usable.contains(&0) && s.usable.contains(&1), "{:?}", s);
    t.e(&side_cond("opp", "spikes"));
    assert!(!t.sorted().usable.contains(&1));
}

#[test]
fn t41_yawn_on_a_drowsy_foe_is_dropped() {
    let mut t = vs(vec![lax(&["yawn", "bodyslam", "protect", "rest"])], "dragonite");
    assert!(t.sorted().usable.contains(&0));
    t.e(&vol("opp", "dragonite", "yawn"));
    assert!(!t.sorted().usable.contains(&0));
}

#[test]
fn t42_no_ko_no_high_uniform_over_status_and_best() {
    let t = vs(vec![chomp(&["protect", "rest", "dragonclaw", "earthquake"])], "dragonite");
    let s = t.sorted();
    assert!(s.ko.is_empty() && s.high.is_empty(), "{:?}", s);
    assert_eq!(s.status, vec![0]);
    assert_eq!(s.best, Some(2));
    let c = t.tower_counts(400);
    assert!(only(&c, &[0, 2]) && c[0] > 150 && c[2] > 150, "{:?}", c);
}

#[test]
fn t43_high_plus_best_with_double_weight() {
    let t = vs(vec![lax(&["icywind", "swordsdance", "protect", "rest"])], "dragapult");
    let s = t.sorted();
    assert_eq!(s.best, Some(0));
    assert!(s.high.contains(&0) && s.high.contains(&1));
    let c = t.tower_counts(900);
    assert!(only(&c, &[0, 1]), "{:?}", c);
    // icywind appears twice in [icywind, swordsdance, icywind]
    assert!(c[0] > 520 && c[0] < 680, "{:?}", c);
}

/// Garchomp with only status moves, none usable: full HP (Rest, Recover), +6 Attack, Stealth Rock up.
const STUCK: [&str; 4] = ["rest", "recover", "swordsdance", "stealthrock"];

fn stuck(t: &mut T) {
    t.e(&boost("own", "garchomp", "atk", 6));
    t.e(&side_cond("opp", "stealthrock"));
}

#[test]
fn t44_nothing_usable_goes_to_the_smart_switch() {
    let mut t = vs(vec![chomp(&STUCK)], "dragonite");
    stuck(&mut t);
    let s = t.sorted();
    assert!(s.usable.is_empty(), "{:?}", s);
    let c = t.tower_counts(200);
    assert!(only(&c, &[9, 10]), "{:?}", c);
}

#[test]
fn t44b_struggle_counts_as_an_attack() {
    // Taunt blocks every move: Struggle is legal with the switches, and tower uses it as its best attack
    let mut t = vs(vec![chomp(&STUCK)], "dragonite");
    t.e(&vol("own", "garchomp", "taunt"));
    let l = t.legal();
    assert_eq!(l.forced, Some(Forced::Struggle));
    assert!(l.mask[9] && l.mask[10]);
    assert_eq!(t.sorted().best, Some(0));
    assert_eq!(t.tower_counts(50)[0], 50);
    assert_eq!(t.md(), 0);
}

fn switch_team() -> Vec<SetMsg> {
    vec![
        chomp(&STUCK),
        set("primarina", None, "torrent", &["moonblast", "icebeam", "hydropump", "protect"]),
        set("cinderace", None, "blaze", &["stoneedge", "pyroball", "protect", "rest"]),
    ]
}

#[test]
fn t45_smart_switch_weights_by_super_effective_moves() {
    let mut t = vs(switch_team(), "dragonite");
    stuck(&mut t);
    let c = t.tower_counts(900);
    assert!(only(&c, &[9, 10]), "{:?}", c);
    // Primarina 2 (Moonblast, Ice Beam), Cinderace 1 (Stone Edge)
    assert!(c[9] > 520 && c[9] < 680, "{:?}", c);
    // all zero: uniform
    let team = vec![
        chomp(&STUCK),
        set("primarina", None, "torrent", &["hydropump", "protect", "rest", "calmmind"]),
        set("cinderace", None, "blaze", &["pyroball", "protect", "rest", "bulkup"]),
    ];
    let mut t = vs(team, "snorlax");
    stuck(&mut t);
    let c = t.tower_counts(600);
    assert!(only(&c, &[9, 10]) && c[9] > 230 && c[10] > 230, "{:?}", c);
}

#[test]
fn t46_forced_switch_uses_the_smart_switch() {
    let mut t = vs(switch_team(), "dragonite");
    t.e(r#"{"k":"faint","side":"own","mon":"garchomp"}"#);
    let mut c = [0u32; N_ACTIONS];
    let mut p = TowerPlayer::default();
    for s in 0..900 {
        c[p.choose_move(t.bt(), ReqKind::Replace, &mut Rng::new(s))] += 1;
    }
    assert!(only(&c, &[9, 10]) && c[9] > 520 && c[9] < 680, "{:?}", c);
}

#[test]
fn t47_preview_matches_max_damage() {
    let team = vec![
        set("snorlax", None, "thickfat", &["protect", "rest", "yawn", "curse"]),
        set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "firefang"]),
        set("primarina", None, "torrent", &["moonblast", "hypervoice", "protect", "rest"]),
        set("cinderace", None, "blaze", &["pyroball", "protect", "rest", "uturn"]),
        set("gholdengo", None, "goodasgold", &["makeitrain", "shadowball", "nastyplot", "recover"]),
        set("lucario", None, "innerfocus", &["protect", "swordsdance", "rest", "calmmind"]),
    ];
    let t = T::start(team, &PREVIEW);
    let mut r = Rng::new(3);
    assert_eq!(TowerPlayer::default().teampreview(t.bt(), &mut r), MaxDamagePlayer::default().teampreview(t.bt(), &mut r));
}

#[test]
fn t48_same_seed_same_choices() {
    let mut g = Rng::new(42);
    let set: Vec<_> = (0..300).map(|_| gen::random_battle(&mut g, false)).collect();
    let run = |seed: u64| -> Vec<usize> {
        let mut p = TowerPlayer::default();
        let mut r = Rng::new(seed);
        set.iter().map(|(b, k)| p.choose_move(b, *k, &mut r)).collect()
    };
    assert_eq!(run(7), run(7));
    assert_ne!(run(7), run(8));
}
