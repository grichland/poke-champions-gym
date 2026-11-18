//! Max damage, spec test list 1-19 (the estimate parts of 12-19 are checked through the Player's input).

mod common;
use battle::*;
use common::*;
use players::max_damage::{best_attack, preview_picks};
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

#[test]
fn t01_picks_the_clearly_best_attack() {
    let t = vs(vec![set("garchomp", None, "roughskin", &["dragonclaw", "earthquake", "rockslide", "crunch"])], "gengar");
    let e: Vec<f32> = (0..4).map(|i| t.exp(i)).collect();
    assert!(e[1] > e[0] && e[1] > e[2] && e[1] > e[3], "{:?}", e);
    assert_eq!(t.md(), 1);
}

#[test]
fn t02_equal_estimates_go_to_the_lower_slot() {
    let t = vs(vec![set("snorlax", None, "immunity", &["swordsdance", "ironhead", "crunch", "protect"])], "garchomp");
    assert_eq!(t.exp(1), t.exp(2));
    assert!(t.exp(1) > 0.0);
    assert_eq!(t.md(), 1);
}

#[test]
fn t03_all_moves_immune_lowest_slot_never_a_switch() {
    let t = vs(vec![set("garchomp", None, "roughskin", &["swordsdance", "earthquake", "bulldoze", "highhorsepower"])], "corviknight");
    for i in 1..4 {
        assert_eq!(t.exp(i), 0.0);
    }
    assert_eq!(t.md(), 0);
}

#[test]
fn t04_attack_over_a_status_move_in_the_first_slot() {
    let t = vs(vec![set("garchomp", None, "roughskin", &["swordsdance", "earthquake", "dragonclaw", "protect"])], "gengar");
    assert_eq!(t.md(), 1);
}

#[test]
fn t05_mega_variant_of_the_best_move() {
    let t = vs(vec![set("garchomp", Some("garchompitez"), "roughskin", &["dragonclaw", "earthquake", "rockslide", "crunch"])], "gengar");
    assert!(t.legal().can_mega);
    assert_eq!(t.md(), 5);
}

#[test]
fn t06_mega_taken_even_when_the_plain_order_scores_higher() {
    // Scrappy Body Slam hits Gengar; Mega Kangaskhan's Parental Bond does not
    let t = vs(vec![set("kangaskhan", Some("kangaskhanite"), "scrappy", &["bodyslam", "protect", "swordsdance", "rest"])], "gengar");
    let b = t.bt();
    let mut est = Estimator::new(b);
    let mv = MoveId::from_id("bodyslam").unwrap();
    assert!(est.hit(b, t.own(), false, mv, t.opp()).expected() > 0.0);
    assert_eq!(est.hit(b, t.own(), true, mv, t.opp()).expected(), 0.0);
    assert_eq!(t.md(), 4, "PS-Q3: Mega when offered, zero rule among the Mega variants");
    let opts = Options { mega_when_offered: false, ..Options::default() };
    assert_eq!(MaxDamagePlayer::new(opts).choose_move(b, ReqKind::Turn, &mut Rng::new(1)), 0);
}

#[test]
fn t07_never_switches_while_a_move_is_legal() {
    let mut t = vs(vec![set("garchomp", None, "roughskin", &["swordsdance", "protect", "earthquake", "rest"])], "corviknight");
    t.e(&hp_own("garchomp", 183, 5));
    let a = t.md();
    assert!(a < 8, "{}", a);
}

#[test]
fn t08_forced_switch_to_the_best_attacker_hp_ignored() {
    let team = vec![
        set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "protect", "rest"]),
        set("primarina", None, "torrent", &["moonblast", "hypervoice", "protect", "rest"]),
        set("cinderace", None, "blaze", &["pyroball", "protect", "rest", "uturn"]),
    ];
    let mut t = vs(team, "corviknight");
    let cmax = t.bt().sets[2].stats[0];
    t.e(&format!(r#"{{"k":"withdraw","side":"own","mon":"garchomp"}}"#));
    t.e(r#"{"k":"enter","side":"own","mon":"cinderace"}"#);
    t.e(&hp_own("cinderace", cmax as i32, 3));
    t.e(r#"{"k":"withdraw","side":"own","mon":"cinderace"}"#);
    t.e(r#"{"k":"enter","side":"own","mon":"garchomp"}"#);
    t.e(r#"{"k":"faint","side":"own","mon":"garchomp"}"#);
    let l = legal(t.bt(), ReqKind::Replace);
    assert_eq!(&l.mask[8..], &[false, true, true]);
    assert_eq!(t.md_kind(ReqKind::Replace), 10);
}

#[test]
fn t09_forced_switch_tie_goes_to_the_first() {
    let team = vec![
        set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "protect", "rest"]),
        set("primarina", None, "torrent", &["protect", "rest", "calmmind", "sing"]),
        set("cinderace", None, "blaze", &["protect", "rest", "bulkup", "taunt"]),
    ];
    let mut t = vs(team, "corviknight");
    t.e(r#"{"k":"faint","side":"own","mon":"garchomp"}"#);
    assert_eq!(t.md_kind(ReqKind::Replace), 9);
}

fn six() -> Vec<SetMsg> {
    vec![
        set("snorlax", None, "thickfat", &["protect", "rest", "yawn", "curse"]),
        set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "stoneedge", "firefang"]),
        set("primarina", None, "torrent", &["moonblast", "hypervoice", "protect", "rest"]),
        set("cinderace", None, "blaze", &["pyroball", "protect", "rest", "uturn"]),
        set("gholdengo", None, "goodasgold", &["makeitrain", "shadowball", "nastyplot", "recover"]),
        set("lucario", None, "innerfocus", &["protect", "swordsdance", "rest", "calmmind"]),
    ]
}

fn expected_preview(b: &Battle) -> [u8; 3] {
    let field = FieldView::default();
    let foes: Vec<Fighter> = b.side(Side::Opp).team.iter().filter_map(|m| m.preview).map(|sp| Fighter::species(sp, Side::Opp)).collect();
    let mut s: Vec<(f32, u8)> = (0..6)
        .map(|i| {
            let me = Fighter::own(b, i, false);
            (foes.iter().map(|f| best_attack(&field, &me, &b.sets[i].moves, f, None)).sum(), i as u8)
        })
        .collect();
    s.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
    [s[0].1, s[1].1, s[2].1]
}

#[test]
fn t10_preview_three_best_summed_scores_lead_first() {
    let t = T::start(six(), &PREVIEW);
    let picks = MaxDamagePlayer::default().teampreview(t.bt(), &mut Rng::new(1));
    assert_eq!(picks, expected_preview(t.bt()));
    // the status-only Pokémon (scores 0) are never brought ahead of an attacker
    assert!(!picks.contains(&0) && !picks.contains(&5), "{:?}", picks);
    // ties keep the lower index: two status-only members and a team of three with zero scores
    let t = T::start(vec![six()[0].clone(), six()[5].clone(), set("sylveon", None, "pixilate", &["protect", "rest", "calmmind", "wish"])], &PREVIEW);
    assert_eq!(preview_picks(t.bt()), [0, 1, 2]);
}

#[test]
fn t11_preview_skips_unread_entries() {
    let pv = ["garchomp", "", "corviknight", "", "dragonite", ""];
    let t = T::start(six(), &pv);
    let picks = preview_picks(t.bt());
    assert_eq!(picks, expected_preview(t.bt()));
    let t = T::start(six(), &["", "", "", "", "", ""]);
    assert_eq!(preview_picks(t.bt()), [0, 1, 2]);
}

#[test]
fn t12_revealed_levitate_zeroes_ground() {
    let mut t = vs(vec![set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "protect", "rest"])], "rotomwash");
    assert!(t.exp(0) > 0.0);
    assert_eq!(t.md(), 0);
    t.e(&ability("opp", "rotomwash", "levitate"));
    assert_eq!(t.exp(0), 0.0);
    assert_eq!(t.md(), 1);
}

#[test]
fn t13_revealed_air_balloon_zeroes_ground() {
    let mut t = vs(vec![set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "protect", "rest"])], "excadrill");
    assert!(t.exp(0) > 0.0);
    t.e(&item("opp", "excadrill", "airballoon"));
    assert_eq!(t.exp(0), 0.0);
}

#[test]
fn t14_stages_move_the_estimate() {
    let mut t = vs(vec![set("garchomp", None, "roughskin", &["earthquake", "dragonclaw", "protect", "rest"])], "dragonite");
    let base = t.exp(1);
    t.e(&boost("opp", "dragonite", "def", 2));
    let lower = t.exp(1);
    t.e(&boost("own", "garchomp", "atk", 2));
    t.e(&boost("own", "garchomp", "atk", 2));
    let higher = t.exp(1);
    assert!(lower < base && higher > base, "{} {} {}", base, lower, higher);
}

#[test]
fn t15_no_guard_makes_a_70_percent_move_sure() {
    let mut t = vs(vec![set("garchomp", None, "noguard", &["stoneedge", "protect", "rest", "swordsdance"])], "dragonite");
    let b = t.bt();
    let f = FieldView::of(b);
    let me = Fighter::of(b, t.own(), false);
    let foe = Fighter::of(b, t.opp(), false);
    let se = MoveId::from_id("stoneedge").unwrap();
    assert_eq!(estimate::accuracy(&f, &me, se, &foe), 1.0);
    t = vs(vec![set("garchomp", None, "roughskin", &["stoneedge", "protect", "rest", "swordsdance"])], "dragonite");
    let b = t.bt();
    let me = Fighter::of(b, t.own(), false);
    assert_eq!(estimate::accuracy(&f, &me, se, &foe), 0.8);
    // the foe's No Guard counts too
    let mut foe2 = foe;
    foe2.ability = AbilityId::from_id("noguard");
    assert_eq!(estimate::accuracy(&f, &me, se, &foe2), 1.0);
}

#[test]
fn t16_weather_accuracy() {
    let t = vs(vec![set("rotomwash", None, "levitate", &["thunder", "hurricane", "blizzard", "protect"])], "dragonite");
    let b = t.bt();
    let me = Fighter::of(b, t.own(), false);
    let foe = Fighter::of(b, t.opp(), false);
    let acc = |w: Option<&str>, m: &str| {
        let mut f = FieldView::default();
        f.weather = w.map(|x| Cond::from_id(x).unwrap());
        estimate::accuracy(&f, &me, MoveId::from_id(m).unwrap(), &foe)
    };
    assert_eq!(acc(Some("raindance"), "thunder"), 1.0);
    assert_eq!(acc(Some("sunnyday"), "thunder"), 0.5);
    assert_eq!(acc(Some("sunnyday"), "hurricane"), 0.5);
    assert_eq!(acc(Some("snowscape"), "blizzard"), 1.0);
    assert_eq!(acc(None, "thunder"), 0.7);
    assert_eq!(acc(None, "blizzard"), 0.7);
}

#[test]
fn t17_multi_hit_counts() {
    let t = vs(vec![set("garchomp", None, "roughskin", &["bulletseed", "tripleaxel", "rockblast", "protect"])], "dragonite");
    let b = t.bt();
    let mut est = Estimator::new(b);
    let h = est.hit(b, t.own(), false, MoveId::from_id("bulletseed").unwrap(), t.opp());
    assert!((h.hits - 3.1).abs() < 1e-6 && h.max_hits == 5.0);
    // Triple Axel: hit k has k times the power and needs k accuracy checks: 1 + 2(0.9) + 3(0.81)
    let h = est.hit(b, t.own(), false, MoveId::from_id("tripleaxel").unwrap(), t.opp());
    assert!((h.hits - 5.23).abs() < 1e-4, "{}", h.hits);
    assert_eq!(h.max_hits, 6.0);
    assert!((h.accuracy - 0.9).abs() < 1e-6);
    // Skill Link: five hits
    let t = vs(vec![set("garchomp", None, "skilllink", &["bulletseed", "protect", "rest", "swordsdance"])], "dragonite");
    let b = t.bt();
    let h = Estimator::new(b).hit(b, t.own(), false, MoveId::from_id("bulletseed").unwrap(), t.opp());
    assert_eq!(h.hits, 5.0);
}

#[test]
fn t18_foul_play_uses_the_foes_attack() {
    let mut t = vs(vec![set("gengar", None, "cursedbody", &["foulplay", "protect", "rest", "swordsdance"])], "snorlax");
    let base = t.exp(0);
    t.e(&boost("own", "gengar", "atk", 6));
    assert_eq!(t.exp(0), base, "own attack stage does not count");
    t.e(&boost("opp", "snorlax", "atk", 2));
    assert!(t.exp(0) > base);
}

#[test]
fn t19_grass_knot_bands_and_facade() {
    let gk = MoveId::from_id("grassknot").unwrap();
    let me = Fighter::species(SpeciesId::from_id("primarina").unwrap(), Side::Own);
    let f = FieldView::default();
    let heavy = Fighter::species(SpeciesId::from_id("snorlax").unwrap(), Side::Opp); // 460 kg
    let light = Fighter::species(SpeciesId::from_id("gengar").unwrap(), Side::Opp); // 40.5 kg
    assert!(heavy.weight_hg() >= 2000 && light.weight_hg() < 500 && light.weight_hg() >= 250);
    // same stats and types, only the weight differs: power 120 against 60
    let mut light = light;
    light.stats = heavy.stats;
    light.types = heavy.types;
    let h_heavy = estimate(&f, &me, gk, &heavy, None);
    let h_light = estimate(&f, &me, gk, &light, None);
    assert!(h_light.max > 0);
    assert!((h_heavy.max as i32 - 2 * h_light.max as i32).abs() <= 2, "{} {}", h_heavy.max, h_light.max);
    let facade = MoveId::from_id("facade").unwrap();
    let mut user = Fighter::species(SpeciesId::from_id("snorlax").unwrap(), Side::Own);
    let target = Fighter::species(SpeciesId::from_id("garchomp").unwrap(), Side::Opp);
    let plain = estimate(&f, &user, facade, &target, None).mid;
    user.status = Status::Par;
    let doubled = estimate(&f, &user, facade, &target, None).mid;
    assert!(doubled as f32 > plain as f32 * 1.8, "{} {}", plain, doubled);
    // burn does not halve Facade
    user.status = Status::Brn;
    assert_eq!(estimate(&f, &user, facade, &target, None).mid, doubled);
}
