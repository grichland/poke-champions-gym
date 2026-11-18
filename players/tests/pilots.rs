//! The four pilots: each signature rule where it applies (taken in about `rule_p` of the seeds, else tower)
//! and where it must not; the preview with the gimmick members first.

mod common;
use battle::*;
use common::*;
use players::pilots::{foe_last, PilotParams};
use players::*;

const N: u64 = 400;

fn fired(f: &[(&'static str, u64)], name: &str) -> u64 {
    f.iter().find(|x| x.0 == name).map_or(0, |x| x.1)
}

/// Taken in about 85% of N seeds (binomial, 4 sigma).
fn about_rule_p(k: u64) -> bool {
    let p = PilotParams::default().rule_p;
    let (m, s) = (N as f64 * p, (N as f64 * p * (1.0 - p)).sqrt());
    (k as f64 - m).abs() < 4.0 * s
}

fn vs(team: Vec<SetMsg>, opp: &str) -> T {
    let mut pv = vec![opp];
    for p in PREVIEW {
        if p != opp && pv.len() < 6 {
            pv.push(p);
        }
    }
    T::new(fill(team), &pv)
}

fn whimsi() -> SetMsg {
    set("whimsicott", None, "infiltrator", &["encore", "moonblast", "swordsdance", "uturn"])
}

#[test]
fn encore_after_a_status_or_recovery_move() {
    for last in ["swordsdance", "roost"] {
        let mut t = vs(vec![whimsi()], "dragonite");
        t.e(&mv("opp", "dragonite", last));
        t.turn(2);
        assert_eq!(foe_last(t.bt()), Some(Some((cg_engine::sim::data::MoveId::from_id(last).unwrap(), last == "swordsdance" || last == "roost"))));
        let (c, f) = t.counts("pilot_encore", N);
        assert!(about_rule_p(fired(&f, "encore")), "{:?}", f);
        assert_eq!(fired(&f, "applies"), N);
        assert!(c[0] as u64 >= fired(&f, "encore"));
    }
}

#[test]
fn no_encore_after_an_attack_a_switch_or_nothing() {
    let mut t = vs(vec![whimsi()], "dragonite");
    t.e(&mv("opp", "dragonite", "dragonclaw"));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_encore", 50).1, "applies"), 0);
    let mut t = vs(vec![whimsi()], "dragonite");
    t.switch("opp", "dragonite", "gengar");
    t.turn(2);
    assert_eq!(foe_last(t.bt()), Some(None));
    assert_eq!(fired(&t.counts("pilot_encore", 50).1, "applies"), 0);
    let t = vs(vec![whimsi()], "dragonite");
    assert_eq!(fired(&t.counts("pilot_encore", 50).1, "applies"), 0);
}

#[test]
fn setup_while_the_foe_is_encored() {
    let mut t = vs(vec![whimsi()], "dragonite");
    t.e(&vol("opp", "dragonite", "encore"));
    let (c, f) = t.counts("pilot_encore", N);
    assert!(about_rule_p(fired(&f, "setup during encore")), "{:?}", f);
    assert!(c[2] as u64 >= fired(&f, "setup during encore"));
    // maxed: nothing to set up
    t.e(&boost("own", "whimsicott", "atk", 6));
    assert_eq!(fired(&t.counts("pilot_encore", 50).1, "applies"), 0);
}

fn pelipper() -> SetMsg {
    set("pelipper", None, "drizzle", &["hurricane", "surf", "uturn", "roost"])
}

#[test]
fn weather_attack_when_our_weather_is_up() {
    let mut t = vs(vec![pelipper()], "garchomp");
    t.e(&weather("raindance"));
    let (c, f) = t.counts("pilot_weather", N);
    let k = fired(&f, "rain attack");
    assert!(about_rule_p(k), "{:?}", f);
    assert!(c[0] as u64 + c[1] as u64 >= k);
    // sun is not ours: no boosted-move rule
    let mut t = vs(vec![pelipper()], "garchomp");
    t.e(&weather("sunnyday"));
    let f = t.counts("pilot_weather", 50).1;
    assert_eq!(fired(&f, "rain attack") + fired(&f, "sun attack"), 0, "{:?}", f);
}

#[test]
fn set_weather_then_switch_to_a_setter() {
    let rain = set("politoed", None, "waterabsorb", &["raindance", "surf", "icebeam", "protect"]);
    let t = vs(vec![rain.clone()], "garchomp");
    let f = t.counts("pilot_weather", N).1;
    assert!(about_rule_p(fired(&f, "set weather")), "{:?}", f);
    // no setter move: switch to the bench's Drizzle
    let t = vs(vec![set("gyarados", None, "intimidate", &["waterfall", "earthquake", "protect", "dragondance"]), pelipper()], "garchomp");
    let (c, f) = t.counts("pilot_weather", N);
    assert!(about_rule_p(fired(&f, "switch to weather setter")), "{:?}", f);
    assert!(c[9] as u64 >= fired(&f, "switch to weather setter"));
    // the foe can KO us: neither
    let mut t = vs(vec![rain], "garchomp");
    t.e(&mv("opp", "garchomp", "earthquake"));
    let max = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("politoed", max, max / 20));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_weather", 50).1, "applies"), 0);
    // a team without weather never applies
    let t = vs(vec![set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"])], "gengar");
    assert_eq!(fired(&t.counts("pilot_weather", 50).1, "applies"), 0);
}

fn rocker() -> SetMsg {
    set("hippowdon", None, "sandstream", &["stealthrock", "spikes", "earthquake", "whirlwind"])
}

#[test]
fn hazards_until_capped_and_not_under_threat() {
    let mut t = vs(vec![rocker()], "dragonite");
    let (c, f) = t.counts("pilot_hazard", N);
    assert!(about_rule_p(fired(&f, "set stealthrock")), "{:?}", f);
    assert!(c[0] as u64 >= fired(&f, "set stealthrock"));
    t.e(&side_cond("opp", "stealthrock"));
    let f = t.counts("pilot_hazard", N).1;
    assert!(about_rule_p(fired(&f, "set spikes")) && fired(&f, "set stealthrock") == 0, "{:?}", f);
    for _ in 0..3 {
        t.e(&side_cond("opp", "spikes"));
    }
    // capped; 4 layers up: a phaze instead
    let f = t.counts("pilot_hazard", N).1;
    assert_eq!(fired(&f, "set spikes"), 0);
    assert!(about_rule_p(fired(&f, "phaze")), "{:?}", f);
    // under a KO threat no hazard
    let mut t = vs(vec![rocker()], "dragonite");
    t.e(&mv("opp", "dragonite", "icebeam"));
    let max = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("hippowdon", max, max / 10));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_hazard", 50).1, "applies"), 0);
}

#[test]
fn phaze_a_boosted_foe_and_pivot_under_threat() {
    let mut t = vs(vec![set("hippowdon", None, "sandstream", &["earthquake", "whirlwind", "slackoff", "protect"])], "dragonite");
    t.e(&boost("opp", "dragonite", "atk", 1));
    assert!(about_rule_p(fired(&t.counts("pilot_hazard", N).1, "phaze")));
    let mut t = vs(vec![set("scizor", None, "technician", &["uturn", "bulletpunch", "swordsdance", "roost"])], "charizard");
    t.e(&side_cond("opp", "stealthrock"));
    t.e(&mv("opp", "charizard", "flamethrower"));
    t.turn(2);
    let (c, f) = t.counts("pilot_hazard", N);
    assert!(about_rule_p(fired(&f, "pivot")), "{:?}", f);
    assert!(c[0] as u64 >= fired(&f, "pivot"));
    // no layer: no pivot
    let mut t = vs(vec![set("scizor", None, "technician", &["uturn", "bulletpunch", "swordsdance", "roost"])], "charizard");
    t.e(&mv("opp", "charizard", "flamethrower"));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_hazard", 50).1, "applies"), 0);
}

#[test]
fn fake_out_on_the_first_turn_if_it_can_hit() {
    let ace = || set("cinderace", None, "blaze", &["fakeout", "pyroball", "suckerpunch", "uturn"]);
    let t = vs(vec![ace()], "dragonite");
    assert!(about_rule_p(fired(&t.counts("pilot_priority", N).1, "fake out")));
    let t = vs(vec![ace()], "gengar");
    assert_eq!(fired(&t.counts("pilot_priority", 50).1, "fake out"), 0);
    let mut t = vs(vec![ace()], "dragonite");
    t.e(&mv("own", "cinderace", "pyroball"));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_priority", 50).1, "fake out"), 0);
}

#[test]
fn priority_ko_and_priority_when_outsped() {
    let sciz = || set("scizor", None, "technician", &["bulletpunch", "uturn", "swordsdance", "roost"]);
    let mut t = vs(vec![sciz()], "dragonite");
    t.e(&mv("own", "scizor", "uturn"));
    t.e(&hp_opp("dragonite", 100.0, 5.0));
    t.turn(2);
    assert!(about_rule_p(fired(&t.counts("pilot_priority", N).1, "priority ko")));
    // slower, and the foe can KO us
    let mut t = vs(vec![sciz()], "dragonite");
    t.e(&mv("own", "scizor", "uturn"));
    t.e(&mv("opp", "dragonite", "flamethrower"));
    let max = t.bt().sets[0].stats[0] as i32;
    t.e(&hp_own("scizor", max, max / 10));
    t.turn(2);
    assert!(about_rule_p(fired(&t.counts("pilot_priority", N).1, "priority when outsped")));
    // neither: no rule
    let mut t = vs(vec![sciz()], "dragonite");
    t.e(&mv("own", "scizor", "uturn"));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_priority", 50).1, "applies"), 0);
}

#[test]
fn sucker_punch_only_after_a_damaging_move() {
    let ace = || set("kingambit", None, "defiant", &["suckerpunch", "ironhead", "swordsdance", "protect"]);
    let mut t = vs(vec![ace()], "gengar");
    t.e(&mv("own", "kingambit", "ironhead"));
    t.e(&mv("opp", "gengar", "willowisp"));
    t.e(&hp_opp("gengar", 100.0, 5.0));
    t.turn(2);
    assert_eq!(fired(&t.counts("pilot_priority", 50).1, "applies"), 0);
    let mut t = vs(vec![ace()], "gengar");
    t.e(&mv("own", "kingambit", "ironhead"));
    t.e(&mv("opp", "gengar", "shadowball"));
    t.e(&hp_opp("gengar", 100.0, 5.0));
    t.turn(2);
    let f = t.counts("pilot_priority", N).1;
    assert!(about_rule_p(fired(&f, "priority ko")), "{:?}", f);
}

#[test]
fn preview_brings_the_gimmick_first() {
    let team = vec![
        set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"]),
        set("snorlax", None, "thickfat", &["bodyslam", "crunch", "protect", "rest"]),
        set("lucario", None, "innerfocus", &["closecombat", "crunch", "protect", "swordsdance"]),
        set("sylveon", None, "pixilate", &["hypervoice", "moonblast", "protect", "calmmind"]),
        rocker(),
        set("whimsicott", None, "prankster", &["encore", "moonblast", "tailwind", "uturn"]),
    ];
    let t = T::start(team, &PREVIEW);
    let base = max_damage::preview_picks(t.bt());
    let pick = |n: &str| by_name(n).unwrap().teampreview(t.bt(), &mut Rng::new(1));
    assert_eq!(pick("pilot_hazard")[0], 4);
    assert_eq!(pick("pilot_encore")[0], 5);
    let w = pick("pilot_weather");
    assert_eq!(w[0], 4, "Hippowdon's Sand Stream");
    // no gimmick on the team: tower's picks
    let t2 = T::start(vec![set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "protect"]), set("snorlax", None, "thickfat", &["bodyslam", "crunch", "protect", "rest"]), set("lucario", None, "innerfocus", &["closecombat", "crunch", "protect", "swordsdance"])], &PREVIEW);
    assert_eq!(by_name("pilot_encore").unwrap().teampreview(t2.bt(), &mut Rng::new(1)), max_damage::preview_picks(t2.bt()));
    let _ = base;
    // the rest keeps tower's order
    let h = pick("pilot_hazard");
    let rest: Vec<u8> = base.iter().copied().filter(|&x| x != 4).collect();
    assert_eq!(&h[1..], &rest[..2]);
}
