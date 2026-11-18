//! Properties over many Battles: finite and in range, determinism, hidden information, seat symmetry.

mod common;

use battle::*;
use common::*;
use obs::layout::*;
use obs::{observe, vocab, Block, ObsBuffers};
use players::gen::random_battle;
use players::Rng;
use cg_engine::sim::data::*;

/// Feature index of each element of the row and field blocks.
fn owners(block: Block, len: usize) -> Vec<usize> {
    let mut out = vec![usize::MAX; len];
    for (k, f) in FEATURES.iter().enumerate().filter(|(_, f)| f.block == block) {
        let starts: Vec<usize> = if f.slot { (0..4).map(|s| MOVE_BASE + MOVE_STRIDE * s + f.offset).collect() } else { vec![f.offset] };
        let span = if f.slot { f.size } else { f.size * f.repeats };
        for s in starts {
            for i in s..s + span {
                out[i] = k;
            }
        }
    }
    out
}

/// A random Battle with more kinds of events than `players::gen` (Wish, Future Sight, Protect, faints,
/// items, abilities, prepared moves, side effects with layers, volatile details, Mega Evolution).
fn rich_battle(rng: &mut Rng) -> (Battle, ReqKind) {
    let team = gold();
    let mut g = G::new(team.clone(), &PREVIEW);
    let mut picks = [0u8, 1, 2, 3, 4, 5];
    for i in 0..3 {
        let j = i + rng.below(6 - i);
        picks.swap(i, j);
    }
    g.picks([picks[0], picks[1], picks[2]]);
    let own_names: Vec<String> = picks[..3].iter().map(|&i| team[i as usize].species.clone()).collect();
    let opp_names: Vec<&str> = PREVIEW[..3].to_vec();
    let (mut oa, mut pa) = (0usize, 0usize);
    g.evs(vec![enter(own(&own_names[0])), enter(opp(opp_names[0]))]);
    let turns = 1 + rng.below(12) as u32;
    let mut kind = ReqKind::Turn;
    let mut hp_own: Vec<i32> = picks.iter().map(|&i| team[i as usize].stats[0]).collect();
    let mut hp_opp = [100.0f64; 3];
    let mut fainted_own = [false; 3];
    let mut fainted_opp = [false; 3];
    let all_moves = ["wish", "futuresight", "protect", "swordsdance", "earthquake", "uturn", "encore", "taunt", "outrage", "solarbeam", "hydropump", "disable"];
    for t in 1..=turns {
        g.ev(turn(t));
        if t == turns {
            break;
        }
        for _ in 0..1 + rng.below(7) {
            let mine = rng.below(2) == 0;
            let (who, side) = if mine { (own(&own_names[oa]), Side::Own) } else { (opp(opp_names[pa]), Side::Opp) };
            if (mine && fainted_own[oa]) || (!mine && fainted_opp[pa]) {
                continue;
            }
            let e = match rng.below(20) {
                0..=2 => {
                    let m = if mine && rng.below(3) > 0 {
                        let set = team.iter().find(|s| s.species == own_names[oa]).unwrap();
                        set.moves[rng.below(4)].clone()
                    } else {
                        all_moves[rng.below(all_moves.len())].to_string()
                    };
                    mv(who, &m)
                }
                3 => {
                    if mine {
                        let before = hp_own[oa];
                        let after = rng.below(before as usize + 1) as i32;
                        hp_own[oa] = after.max(1);
                        Event::Hp { who, dir: HpDir::Lost, hp: Hp::Own { before, after: after.max(1) } }
                    } else {
                        let before = hp_opp[pa];
                        let after = (rng.below(before as usize + 1) as f64).max(1.0);
                        hp_opp[pa] = after;
                        Event::Hp { who, dir: HpDir::Lost, hp: Hp::Opp { before_pct: before, after_pct: after } }
                    }
                }
                4 => boost(who, BOOST_IDS[rng.below(7)], rng.below(7) as i32 - 3),
                5 => Event::Status { who, status: ["brn", "par", "slp", "psn", "tox", "frz"][rng.below(6)].into(), effect: None },
                6 => vol(who, ["taunt", "encore", "substitute", "perishsong", "whirlpool", "stockpile", "confusion", "disable"][rng.below(8)], if rng.below(2) == 0 { Some(Detail::Num(rng.below(4) as i64)) } else { None }),
                7 => side_start(side, ["reflect", "lightscreen", "tailwind", "spikes", "toxicspikes", "stealthrock", "stickyweb", "safeguard", "auroraveil"][rng.below(9)]),
                8 => Event::Weather { weather: ["raindance", "sunnyday", "sandstorm", "snowscape", "desolateland"][rng.below(5)].into(), what: StartEnd::Start },
                9 => Event::Field { effect: ["electricterrain", "grassyterrain", "mistyterrain", "psychicterrain", "trickroom", "gravity", "magicroom", "wonderroom"][rng.below(8)].into(), what: StartEnd::Start },
                10 => Event::Item { who: Some(who), item: ["leftovers", "sitrusberry", "choicescarf", "focussash"][rng.below(4)].into(), what: ItemWhat::Activated, of: None },
                11 if !mine => Event::Ability { who, ability: "levitate".into() },
                12 => Event::Cant { who: Some(who), reason: ["slp", "par", "flinch"][rng.below(3)].into() },
                13 => Event::Prepare { who, mv: Some("solarbeam".into()) },
                14 => {
                    if mine {
                        let next = (oa + 1 + rng.below(2)) % 3;
                        if fainted_own[next] {
                            continue;
                        }
                        g.ev(withdraw(who));
                        oa = next;
                        enter(own(&own_names[oa]))
                    } else {
                        let next = (pa + 1 + rng.below(2)) % 3;
                        if fainted_opp[next] {
                            continue;
                        }
                        g.ev(withdraw(who));
                        pa = next;
                        enter(opp(opp_names[pa]))
                    }
                }
                15 if mine => {
                    fainted_own[oa] = true;
                    g.ev(Event::Faint { who });
                    match (0..3).find(|&i| !fainted_own[i]) {
                        Some(n) => {
                            oa = n;
                            enter(own(&own_names[oa]))
                        }
                        None => continue,
                    }
                }
                15 => {
                    fainted_opp[pa] = true;
                    g.ev(Event::Faint { who });
                    let next = (0..3).find(|&i| !fainted_opp[i]);
                    match next {
                        Some(n) => {
                            pa = n;
                            enter(opp(opp_names[pa]))
                        }
                        None => continue,
                    }
                }
                16 => Event::Cause { effect: "wish".into(), who: Some(who), of: None },
                17 => Event::MegaStone { who, stone: "garchompitez".into() },
                18 => Event::Ability { who, ability: "pressure".into() },
                _ => continue,
            };
            g.ev(e);
        }
    }
    if rng.below(6) == 0 && !fainted_own[oa] {
        g.ev(Event::Faint { who: own(&own_names[oa]) });
        kind = ReqKind::Replace;
    }
    (g.b.battle, kind)
}

fn check(b: &Battle, kind: ReqKind, o: &mut ObsBuffers, row_own: &[usize], field_own: &[usize], last_own: &[usize], seen: &mut [bool]) {
    observe(b, kind, o);
    for r in 0..12 {
        for (i, x) in o.rows[r].iter().enumerate() {
            let f = &FEATURES[row_own[i]];
            assert!(x.is_finite() && *x >= f.lo && *x <= f.hi, "{} = {} out of {}..{}", f.id, x, f.lo, f.hi);
            if *x != 0.0 {
                seen[row_own[i]] = true;
            }
        }
        let ids = &o.row_ids[r];
        assert!(ids[F1_3] >= 0 && (ids[F1_3] as usize) < vocab::SPECIES_VOCAB);
        assert!(ids[F2_6] >= 0 && (ids[F2_6] as usize) < vocab::ITEM_VOCAB);
        assert!(ids[F2_7] >= 0 && (ids[F2_7] as usize) < vocab::ABILITY_VOCAB);
        for j in 0..4 {
            assert!(ids[F7_13 + j] >= 0 && (ids[F7_13 + j] as usize) < vocab::MOVE_VOCAB);
        }
        for j in 0..3 {
            assert!(ids[F8_10 + j] >= 0 && (ids[F8_10 + j] as usize) < vocab::ABILITY_VOCAB);
        }
    }
    for (i, x) in o.field.iter().enumerate() {
        let f = &FEATURES[field_own[i]];
        assert!(x.is_finite() && *x >= f.lo && *x <= f.hi, "{} = {}", f.id, x);
        if *x != 0.0 {
            seen[field_own[i]] = true;
        }
    }
    for (i, x) in o.last.iter().enumerate() {
        let f = &FEATURES[last_own[i]];
        assert!(x.is_finite() && *x >= f.lo && *x <= f.hi, "{} = {}", f.id, x);
        if *x != 0.0 {
            seen[last_own[i]] = true;
        }
    }
    for (i, id) in o.last_ids.iter().enumerate() {
        let vocab = if i % 2 == 0 { vocab::MOVE_VOCAB } else { vocab::SPECIES_VOCAB };
        assert!(*id >= 0 && (*id as usize) < vocab);
    }
    assert_eq!(o.mask.map(|x| x == 1), legal(b, kind).mask);
}

#[test]
fn finite_and_in_range_over_random_battles() {
    let row_own = owners(Block::Row, ROW_LEN);
    let field_own = owners(Block::Field, FIELD_LEN);
    let last_own = owners(Block::Last, LAST_LEN);
    let mut seen = vec![false; FEATURES.len()];
    let mut o = Box::new(ObsBuffers::default());
    let mut rng = Rng::new(7);
    for _ in 0..5000 {
        let (b, k) = random_battle(&mut rng, false);
        check(&b, k, &mut o, &row_own, &field_own, &last_own, &mut seen);
    }
    for _ in 0..500 {
        let (b, k) = random_battle(&mut rng, true);
        check(&b, k, &mut o, &row_own, &field_own, &last_own, &mut seen);
    }
    let mut rng = Rng::new(11);
    for _ in 0..5000 {
        let (b, k) = rich_battle(&mut rng);
        check(&b, k, &mut o, &row_own, &field_own, &last_own, &mut seen);
    }
    let never: Vec<&str> = FEATURES
        .iter()
        .zip(seen.iter())
        .filter(|(f, s)| !**s && matches!(f.block, Block::Row | Block::Field | Block::Last))
        .map(|(f, _)| f.id)
        .collect();
    eprintln!("float features never non-zero over these Battles: {:?}", never);
    // Mist is not in Champions
    assert_eq!(never, vec!["G3-1.6", "G3-3.6"]);
}

#[test]
fn deterministic() {
    let mut rng = Rng::new(3);
    for _ in 0..500 {
        let (b, k) = rich_battle(&mut rng);
        let c = b;
        let mut x = Box::new(ObsBuffers::default());
        let mut y = Box::new(ObsBuffers::default());
        observe(&b, k, &mut x);
        // a dirty buffer is fully overwritten
        y.rows[3][7] = 9.0;
        y.row_ids[11][9] = 77;
        y.field[65] = 5.0;
        observe(&c, k, &mut y);
        assert_eq!(x, y);
    }
}

/// The opponent's truth differs only in what is never shown (items, abilities, unused moves, nature and
/// points, which unseen members exist beyond the preview); the events show the same, so the tensors match.
#[test]
fn hidden_information_does_not_reach_the_tensors() {
    struct Truth {
        item: &'static str,
        ability: &'static str,
        unused: [&'static str; 3],
    }
    let truths = [
        Truth { item: "choicescarf", ability: "levitate", unused: ["voltswitch", "willowisp", "protect"] },
        Truth { item: "leftovers", ability: "levitate", unused: ["thunderbolt", "trick", "painsplit"] },
    ];
    let play = |t: &Truth| {
        // the engine's emitter would show nothing of `t` in these turns: no item or ability row, and the
        // only move used is the one both truths share
        let _ = (t.item, t.ability, t.unused);
        let mut g = G::new(gold(), &PREVIEW);
        g.picks([1, 0, 3]);
        g.evs(vec![enter(own("garchomp")), enter(opp("rotomwash")), turn(1), mv(opp("rotomwash"), "hydropump"), mv(own("garchomp"), "earthquake"), hp_opp("rotomwash", 100.0, 55.0), turn(2)]);
        g
    };
    let a = play(&truths[0]);
    let b = play(&truths[1]);
    assert_eq!(a.battle(), b.battle());
    assert_eq!(a.obs(ReqKind::Turn), b.obs(ReqKind::Turn));
    // the unknown item and ability are encoded as unknown, not as either truth
    let o = a.obs(ReqKind::Turn);
    assert_eq!((o.row_ids[6][F2_6], o.row_ids[6][F2_7], o.rows[6][F2_1], o.rows[6][F2_3]), (0, 0, 0.0, 0.0));
    assert_eq!(&o.row_ids[6][F7_13 + 1..F7_13 + 4], &[0, 0, 0]);
}

// ------------------------------------------------------------------ seat symmetry

#[derive(Clone)]
enum Ne {
    Turn(u32),
    Enter(usize, &'static str),
    Withdraw(usize, &'static str),
    Move(usize, &'static str, &'static str),
    /// seat, species, HP before and after as whole numbers
    Hp(usize, &'static str, i32, i32),
    Boost(usize, &'static str, &'static str, i32),
    Status(usize, &'static str, &'static str),
    Vol(usize, &'static str, &'static str),
    SideStart(usize, &'static str),
    Weather(&'static str),
}

fn seat_team(seat: usize) -> Vec<SetMsg> {
    if seat == 0 {
        gold()
    } else {
        vec![
            set("rotomwash", Some("leftovers"), "levitate", ["hydropump", "voltswitch", "willowisp", "protect"]),
            set("snorlax", Some("sitrusberry"), "thickfat", ["bodyslam", "curse", "rest", "earthquake"]),
            set("glimmora", Some("focussash"), "toxicdebris", ["stealthrock", "powergem", "mortalspin", "earthpower"]),
            set("dragonite", Some("choicescarf"), "multiscale", ["extremespeed", "outrage", "earthquake", "firepunch"]),
            set("tyranitar", Some("leftovers"), "sandstream", ["stoneedge", "crunch", "earthquake", "protect"]),
            set("mamoswine", Some("lifeorb"), "thickfat", ["earthquake", "iciclecrash", "iceshard", "stealthrock"]),
        ]
    }
}

fn to_seat(e: &Ne, seat: usize, teams: &[Vec<SetMsg>; 2]) -> Event {
    let who = |p: usize, sp: &str| Mon::new(if p == seat { Side::Own } else { Side::Opp }, sp);
    match e {
        Ne::Turn(n) => turn(*n),
        Ne::Enter(p, sp) => enter(who(*p, sp)),
        Ne::Withdraw(p, sp) => withdraw(who(*p, sp)),
        Ne::Move(p, sp, m) => mv(who(*p, sp), m),
        Ne::Hp(p, sp, before, after) => {
            let max = teams[*p].iter().find(|s| s.species == *sp).unwrap().stats[0];
            let dir = if after < before { HpDir::Lost } else { HpDir::Restored };
            let hp = if *p == seat {
                Hp::Own { before: *before, after: *after }
            } else {
                let pct = |x: i32| x as f64 * 100.0 / max as f64;
                Hp::Opp { before_pct: pct(*before), after_pct: pct(*after) }
            };
            Event::Hp { who: who(*p, sp), dir, hp }
        }
        Ne::Boost(p, sp, st, by) => boost(who(*p, sp), st, *by),
        Ne::Status(p, sp, st) => Event::Status { who: who(*p, sp), status: st.to_string(), effect: None },
        Ne::Vol(p, sp, ef) => vol(who(*p, sp), ef, None),
        Ne::SideStart(p, ef) => side_start(if *p == seat { Side::Own } else { Side::Opp }, ef),
        Ne::Weather(w) => Event::Weather { weather: w.to_string(), what: StartEnd::Start },
    }
}

#[test]
fn the_other_seat_sees_the_same_known_facts_mirrored() {
    let teams = [seat_team(0), seat_team(1)];
    let hp = |p: usize, sp: &str| teams[p].iter().find(|s| s.species == sp).unwrap().stats[0];
    let (g0, r0, s0) = (hp(0, "garchomp"), hp(1, "rotomwash"), hp(1, "snorlax"));
    let script = vec![
        Ne::Enter(0, "garchomp"),
        Ne::Enter(1, "rotomwash"),
        Ne::Turn(1),
        Ne::Move(0, "garchomp", "earthquake"),
        Ne::Hp(1, "rotomwash", r0, r0 * 2 / 5),
        Ne::Move(1, "rotomwash", "willowisp"),
        Ne::Status(0, "garchomp", "brn"),
        Ne::Vol(1, "rotomwash", "taunt"),
        Ne::SideStart(0, "reflect"),
        Ne::SideStart(1, "spikes"),
        Ne::Weather("sunnyday"),
        Ne::Turn(2),
        Ne::Withdraw(1, "rotomwash"),
        Ne::Enter(1, "snorlax"),
        Ne::Move(0, "garchomp", "swordsdance"),
        Ne::Boost(0, "garchomp", "atk", 2),
        Ne::Turn(3),
        Ne::Move(0, "garchomp", "earthquake"),
        Ne::Hp(1, "snorlax", s0, s0 / 2),
        Ne::Move(1, "snorlax", "bodyslam"),
        Ne::Hp(0, "garchomp", g0, g0 * 3 / 4),
        Ne::Turn(4),
    ];
    let mut seats = [G::new(teams[0].clone(), &["rotomwash", "snorlax", "glimmora", "dragonite", "tyranitar", "mamoswine"]), G::new(teams[1].clone(), &["gholdengo", "garchomp", "golisopod", "primarina", "mamoswine", "cinderace"])];
    seats[0].picks([1, 0, 3]);
    seats[1].picks([0, 1, 2]);
    let mut compared = 0;
    for e in &script {
        for s in 0..2 {
            seats[s].ev(to_seat(e, s, &teams));
        }
        for s in 0..2 {
            assert!(seats[s].b.anomalies.is_empty(), "seat {}: {:?}", s, seats[s].b.anomalies);
        }
        let a = seats[0].obs(ReqKind::Turn);
        let b = seats[1].obs(ReqKind::Turn);
        compared += compare(&seats, &a, &b);
        compared += compare(&[G { b: seats[1].b.clone() }, G { b: seats[0].b.clone() }], &b, &a);
    }
    // rows and revealed moves were compared, not skipped
    assert!(compared > 100, "{}", compared);
}

/// Every Pokémon both seats have seen: its row on one side equals its row on the other, on the columns
/// both know. Field and last-turn columns swap own and opponent.
fn compare(seats: &[G], a: &ObsBuffers, b: &ObsBuffers) -> usize {
    let mut n = 0;
    let ra = obs::row_order(seats[0].battle());
    let rb = obs::row_order(seats[1].battle());
    let both = [F1_1, F1_2_1, F3_1, F3_2, F3_3, F3_4_1, F3_5, F3_6_1, F3_6_2, F3_6_3, F3_7, F3_8, F3_9, F3_11, F4_4, F6_1, F6_2_1, F6_2_2, F6_3, F9_1_1, F9_1_2];
    let sizes = |off: usize| FEATURES.iter().find(|f| f.block == Block::Row && !f.slot && f.offset == off).unwrap().size;
    for (i, w) in ra[..6].iter().enumerate() {
        let Some(w) = w else { continue };
        let mon = seats[0].battle().mon(*w);
        if !mon.seen {
            continue;
        }
        let j = 6 + rb[6..].iter().position(|x| x.map(|x| x.slot) == Some(w.slot)).unwrap();
        assert_eq!(a.row_ids[i][F1_3], b.row_ids[j][F1_3]);
        n += 1;
        for &off in &both {
            for k in off..off + sizes(off) {
                assert!((a.rows[i][k] - b.rows[j][k]).abs() < 1e-4, "own row {} vs opp row {} col {}: {} vs {}", i, j, k, a.rows[i][k], b.rows[j][k]);
            }
        }
        // revealed moves: same facts, by move id
        for s in 0..4 {
            let id = b.row_ids[j][F7_13 + s];
            if id == 0 {
                continue;
            }
            let t = (0..4).find(|&t| a.row_ids[i][F7_13 + t] == id).unwrap();
            let sa = &a.rows[i][MOVE_BASE + MOVE_STRIDE * t..MOVE_BASE + MOVE_STRIDE * (t + 1)];
            let sb = &b.rows[j][MOVE_BASE + MOVE_STRIDE * s..MOVE_BASE + MOVE_STRIDE * (s + 1)];
            for k in [F7_1, F7_2, F7_3, F7_4, F7_5, F7_6_1, F7_6_2, F7_6_3, F7_7, F7_8_1, F7_8_3, F7_9] {
                let n = FEATURES.iter().find(|f| f.slot && f.offset == k).unwrap().size;
                assert_eq!(&sa[k..k + n], &sb[k..k + n], "move {} col {}", id, k);
            }
            n += 10;
        }
    }
    let swap = |x: &[f32], own: (usize, usize), opp: (usize, usize)| (x[own.0..own.0 + own.1].to_vec(), x[opp.0..opp.0 + opp.1].to_vec());
    for (own, opp) in [((G1_4_1, 1), (G1_4_2, 1)), ((G3_1_1, 15), (G3_3_1, 15)), ((G6_1_1, 4), (G6_2_1, 4)), ((G9_2_1, 2), (G9_3_1, 2))] {
        let (x, y) = swap(&a.field, own, opp);
        let (p, q) = swap(&b.field, own, opp);
        assert_eq!((x, y), (q, p));
    }
    for k in [G1_1, G2_1, G2_1 + 1, G2_1 + 2, G2_1 + 3, G2_2, G2_5, G2_6_1, G9_1] {
        assert_eq!(a.field[k], b.field[k]);
    }
    assert_eq!(&a.last[L1_1..L1_4], &b.last[L1_4..L3_2]);
    assert_eq!(a.last[L3_2], b.last[L3_2]);
    assert_eq!(a.last_ids, [b.last_ids[2], b.last_ids[3], b.last_ids[0], b.last_ids[1]]);
    n
}
