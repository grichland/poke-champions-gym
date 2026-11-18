//! Labels against what was actually played: own actions, the opposing choice as one of 13 actions on the
//! observer's rows, reveal-order move slots, opp_shown against the recorded events, opp_mask never ruling
//! out the real choice, replace and locked cases, preview picks.

mod common;

use battle::{Body, Event, Input, ReqKind, Side, Who};
use env::batch::preview_action;
use env::observe::{ObsBatch, ROWS_LEN, ROW_IDS_LEN};
use env::{BatchEnv, EnvConfig, RequestKind, TeamMode, N_ALL_ACTIONS};
use obs::layout::*;
use obs::{vocab, FIELD, LAST, LAST_IDS, MASK};
use players::{by_name, Rng};
use cg_engine::sim::data::MoveId;
use std::collections::HashMap;
use std::sync::Arc;

/// What was known at one decision.
#[derive(Clone)]
struct Stored {
    seat: usize,
    kind: RequestKind,
    action: u16,
    /// observation row species ids
    species: [i32; 12],
    rows: [Option<Who>; 12],
    /// the observer's view of the opposing active and its move slots
    opp_active: Option<Who>,
    /// own active's move slots (truth for this seat)
    own_moves: [Option<MoveId>; 4],
    /// opponent rows 6..12: ids, brought and not-brought flags
    opp_ids: [[i32; 10]; 6],
    opp_flags: [(f32, f32); 6],
    /// the opposing seat's State team (the three brought) as (species, set moves), in pick order
    opp_state: Vec<(i32, Vec<i32>)>,
    opp_transformed: bool,
}

#[derive(Default, Debug)]
struct Tally {
    labels: u64,
    moves: u64,
    switches: u64,
    forced: u64,
    replace: u64,
    previews: u64,
    shown: u64,
    hidden: u64,
    mask_violations: HashMap<&'static str, u64>,
    truth_checked: u64,
    /// observed move slots holding a called move (not the Pokémon's own)
    called_in_slot: u64,
    slot_checked: u64,
}

/// Every event a seat was emitted, delivered (its record) then held back (the live order): in both orders
/// this sequence only grows at its end.
fn all_events(recs: &[Input], held: &[Input]) -> Vec<Event> {
    let mut out = vec![];
    for i in recs.iter().chain(held) {
        if let Input::Msg(m) = i {
            if let Body::Events { events, .. } = &m.body {
                out.extend(events.iter().cloned());
            }
        }
    }
    out
}

fn play(a: &str, b: &str, games: u64, seed: u64, replace_before_rows: bool) -> Tally {
    let pool = Arc::new(common::pool());
    let n = 16;
    let cfg = EnvConfig { games: n, seed, team_mode: TeamMode::Stage3, max_steps: 1000, checks: false, keep_records: true, auto_locked: true, replace_before_rows };
    let mut env = BatchEnv::new(pool, cfg);
    let mut ps: Vec<[Box<dyn players::Player>; 2]> = (0..n).map(|_| [by_name(a).unwrap(), by_name(b).unwrap()]).collect();
    let mut rng = Rng::new(seed);
    let mut stored: HashMap<(usize, u64), Stored> = HashMap::new();
    let mut t = Tally::default();
    let mut legal_v = Vec::new();
    let (mut rows, mut ids) = (vec![0f32; n * ROWS_LEN], vec![0i32; n * ROW_IDS_LEN]);
    let (mut field, mut last, mut lids, mut mask) = (vec![0f32; n * FIELD], vec![0f32; n * LAST], vec![0i32; n * LAST_IDS], vec![0u8; n * MASK]);
    let (mut acts_m, mut seat_v, mut kind_v, mut dec_v, mut prior) = (vec![0u8; n * N_ALL_ACTIONS], vec![0u8; n], vec![0u8; n], vec![0i64; n], vec![0f32; n * 13]);
    while env.games_done < games {
        env.observe_batch(ObsBatch { rows: &mut rows, row_ids: &mut ids, field: &mut field, last: &mut last, last_ids: &mut lids, mask: &mut mask, actions: &mut acts_m, seat: &mut seat_v, kind: &mut kind_v, decision: &mut dec_v, opp_prior: &mut prior })
            .unwrap();
        let mut acts = vec![0u16; n];
        let mut rec_len = vec![[0usize; 2]; n];
        for g in 0..n {
            let p = env.pending(g);
            let seat = p.seat as usize;
            let bt = env.battle(g, seat);
            let rk = if p.kind == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn };
            let act = if p.kind == RequestKind::Preview { preview_action(ps[g][seat].teampreview(bt, &mut rng)).unwrap() } else { ps[g][seat].choose_move(bt, rk, &mut rng) as u16 };
            env.legal_actions(g, &mut legal_v);
            acts[g] = if legal_v.contains(&act) { act } else { legal_v[0] };
            let mut species = [0; 12];
            for r in 0..12 {
                species[r] = ids[g * ROW_IDS_LEN + r * ROWIDS_LEN + F1_3];
            }
            let oa = &bt.side(Side::Own).active;
            // while transformed the Battle holds the copied moves in reveal order only (IN-4): not compared
            let own_moves = if oa.transformed.is_some() { [None; 4] } else { bt.active(Side::Own).map_or([None; 4], |w| bt.mon(w).moves) };
            let full = if p.kind == RequestKind::Preview { acts[g] + 11 } else { acts[g] };
            let pr = &prior[g * 13..(g + 1) * 13];
            assert!(pr.iter().all(|x| (0.0..=1.0).contains(x)), "prior {:?}", pr);
            let k = bt.sides[1].team.iter().filter(|m| m.seen).count();
            let rws = obs::row_order(bt);
            if p.kind != RequestKind::Preview && bt.active(Side::Opp).is_some() && !bt.mon(bt.active(Side::Opp).unwrap()).fainted && pr[8..].iter().any(|x| *x > 0.0) {
                let unseen: f32 = (7..12).filter(|&r| rws[r].map_or(false, |w| !bt.mon(w).seen)).map(|r| pr[8 + r - 7]).sum();
                assert!((unseen - (3 - k.min(3)) as f32).abs() < 1e-5, "unseen switch prior {} with {} seen", unseen, k);
            }
            let mut opp_ids = [[0; 10]; 6];
            let mut opp_flags = [(0.0, 0.0); 6];
            for i in 0..6 {
                opp_ids[i].copy_from_slice(&ids[g * ROW_IDS_LEN + (6 + i) * ROWIDS_LEN..g * ROW_IDS_LEN + (7 + i) * ROWIDS_LEN]);
                let r = &rows[g * ROWS_LEN + (6 + i) * ROW_LEN..];
                opp_flags[i] = (r[F4_1], r[F4_2]);
            }
            let o = 1 - seat;
            let opp_state = if p.kind == RequestKind::Preview {
                vec![]
            } else {
                env.slot(g).game.state.sides[o].mons.iter().map(|m| (vocab::species(Some(m.set_species)), (if m.transformed { &m.base_moves } else { &m.moves })[..m.nmoves as usize].iter().map(|x| vocab::mv(Some(x.id))).collect())).collect()
            };
            let opp_transformed = bt.side(Side::Opp).active.transformed.is_some();
            stored.insert((g, dec_v[g] as u64), Stored { seat, kind: p.kind, action: full, species, rows: rws, opp_active: bt.active(Side::Opp), own_moves, opp_ids, opp_flags, opp_state, opp_transformed });
            let sl = env.slot(g);
            rec_len[g] = [all_events(&sl.seats.records[0], &sl.seats.held[0]).len(), all_events(&sl.seats.records[1], &sl.seats.held[1]).len()];
        }
        env.act(&acts).unwrap();
        for g in 0..n {
            let labs = env.labels()[g];
            let ended = env.results()[g].done;
            for seat in 0..2 {
                let l = labs[seat];
                if !l.valid {
                    continue;
                }
                t.labels += 1;
                let st = stored.get(&(g, l.decision)).expect("label for an unknown decision").clone();
                assert_eq!(st.seat, seat);
                assert_eq!(l.own_action, st.action as i16, "own action");
                if l.refused {
                    continue;
                }
                let o = 1 - seat;
                // truth: never contradicted by what the observation showed, and equal to the State's sets
                for i in 0..6 {
                    let ids = &st.opp_ids[i];
                    let (brought, not_brought) = st.opp_flags[i];
                    assert!(brought == 0.0 || l.opp_true_brought[i] == 1, "row {} seen but not brought", 6 + i);
                    assert!(not_brought == 0.0 || l.opp_true_brought[i] == 0, "row {} known not brought", 6 + i);
                    if !(i == 0 && st.opp_transformed) {
                        for j in 0..4 {
                            let shown = ids[F7_13 + j];
                            if shown != 0 && shown != l.opp_true_moves[i][j] {
                                // the Battle keeps a move called by a calling move (Copycat into Uproar) as revealed
                                let callers = ["copycat", "metronome", "sleeptalk", "assist", "mefirst", "mirrormove"];
                                let caller = ids[F7_13..F7_13 + 4].iter().any(|&x| x > 0 && callers.contains(&MoveId(x as u16).id()));
                                assert!(caller && !l.opp_true_moves[i].contains(&shown), "row {} slot {}: shown {} truth {:?}", 6 + i, j, MoveId(shown as u16).id(), l.opp_true_moves[i]);
                                t.called_in_slot += 1;
                            }
                        }
                    }
                    assert!(ids[F2_6] == 0 || ids[F2_6] == l.opp_true_item[i], "item");
                    assert!(ids[F2_7] == 0 || ids[F2_7] == l.opp_true_ability[i], "ability");
                    if l.opp_true_brought[i] == 1 && !st.opp_state.is_empty() {
                        let mut truth: Vec<i32> = l.opp_true_moves[i].iter().copied().filter(|&x| x != 0).collect();
                        truth.sort();
                        let found = st.opp_state.iter().any(|(sp, mv)| {
                            let mut m = mv.clone();
                            m.sort();
                            (*sp == ids[F1_3] || ids[F1_3] == 0) && m == truth
                        });
                        assert!(found, "row {} truth moves {:?} not in the State's team {:?}", 6 + i, truth, st.opp_state);
                        t.truth_checked += 1;
                    }
                }
                // the other seat's decision of the same step, if it was asked
                let other = labs[o].valid.then(|| stored.get(&(g, labs[o].decision)).cloned()).flatten();
                match st.kind {
                    RequestKind::Preview => {
                        t.previews += 1;
                        let ob = other.expect("both seats pick at preview");
                        let pk = env::batch::preview_picks(ob.action - 11).unwrap();
                        assert_eq!(l.opp_picks, [pk[0] as i8, pk[1] as i8, pk[2] as i8]);
                        continue;
                    }
                    RequestKind::Replace => {
                        t.replace += 1;
                        assert_eq!(l.opp_action, -1);
                        continue;
                    }
                    RequestKind::Turn => {}
                }
                if l.opp_forced == 1 {
                    t.forced += 1;
                    assert!(!labs[o].valid, "a forced opponent was asked");
                }
                if l.opp_action >= 0 && l.opp_prior[l.opp_action as usize] == 0.0 {
                    let class = if l.opp_action < 4 { "move" } else if l.opp_action < 8 { "mega" } else { "switch" };
                    *t.mask_violations.entry(class).or_default() += 1;
                }
                let sl = env.slot(g);
                let all = if ended { all_events(&sl.last_records.as_ref().unwrap()[seat], &[]) } else { all_events(&sl.seats.records[seat], &sl.seats.held[seat]) };
                let evs = all[rec_len[g][seat].min(all.len())..].to_vec();
                if l.opp_action >= 8 {
                    t.switches += 1;
                    let r = 7 + (l.opp_action - 8) as usize;
                    let who = st.rows[r].unwrap();
                    assert!(st.species[r] > 0);
                    if let (Some(ob), false) = (&other, ended) {
                        // the other seat switched to its pick p: that team slot is the Pokémon of row r
                        let p = (ob.action - 8) as usize;
                        assert!(ob.action >= 8 && ob.action < 11);
                        assert_eq!(env.slot(g).picks[o].unwrap()[p], who.slot, "switch row");
                    }
                    let shown = evs.iter().any(|e| matches!(e, Event::Withdraw { who, .. } if who.side == Side::Opp) || matches!(e, Event::Enter { who, forced: false } if who.side == Side::Opp));
                    assert_eq!(l.opp_shown == 1, shown);
                } else if l.opp_move != 0 {
                    t.moves += 1;
                    if let Some(ob) = &other {
                        if ob.action < 8 {
                            let mv = ob.own_moves[(ob.action % 4) as usize];
                            if let Some(mv) = mv {
                                if l.opp_action >= 0 {
                                    assert_eq!(l.opp_move, vocab::mv(Some(mv)), "move id: label {} own {:?} action {} opp_forced {}", MoveId(l.opp_move as u16).id(), ob.own_moves.map(|x| x.map(|m| m.id())), ob.action, l.opp_forced);
                                }
                            }
                        }
                    }
                    let id = MoveId(l.opp_move as u16).id();
                    let shown = evs.iter().any(|e| matches!(e, Event::Move { who, mv, called_by: None, .. } if who.side == Side::Opp && mv == id));
                    assert_eq!(l.opp_shown == 1, shown, "opp_shown for {}", id);
                    // reveal order: once shown, the move sits in the labelled slot
                    if shown && l.opp_action >= 0 && !ended && env.slot(g).seats.held_events[seat] == 0 {
                        if let Some(w) = st.opp_active {
                            let b = env.battle(g, seat);
                            if !(b.side(Side::Opp).active.slot == Some(w.slot) && b.side(Side::Opp).active.transformed.is_some()) {
                                assert_eq!(b.mon(w).moves[(l.opp_action % 4) as usize].map(|m| m.id()), Some(id), "slot of {}", id);
                                t.slot_checked += 1;
                            }
                        }
                    }
                }
                t.shown += l.opp_shown as u64;
                t.hidden += (l.opp_shown == 0 && l.opp_action >= 0) as u64;
            }
        }
    }
    eprintln!("{} vs {}: {:?}", a, b, t);
    t
}

#[test]
fn labels_match_what_was_played() {
    for (a, b, seed, rbr) in [("tower", "max_damage", 3, true), ("random", "random", 4, true), ("tower", "max_damage", 3, false), ("random", "random", 4, false)] {
        let t = play(a, b, 300, seed, rbr);
        assert!(t.moves > 1000 && t.replace > 50 && t.previews > 500, "{:?}", t);
        // tower and max damage almost never switch by choice
        assert!(a != "random" || t.switches > 50, "{:?}", t);
        assert!(t.slot_checked > 500, "{:?}", t);
        assert!(t.hidden > 0, "some choices stay hidden (faint or flinch first): {:?}", t);
        assert!(t.mask_violations.is_empty(), "opp_prior 0 on the real choice: {:?}", t.mask_violations);
        assert!(t.truth_checked > 1000, "{:?}", t);
    }
}

#[test]
fn locked_opponents_are_forced() {
    let t = play("random", "random", 600, 9, true);
    assert!(t.forced > 0, "{:?}", t);
}
