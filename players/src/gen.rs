//! Random Battles built through the Battle builder from random event lists, for the property test and
//! the speed run. Not part of any Player.

use crate::Rng;
use battle::*;
use cg_engine::sim::calc::stat_from_points;
use cg_engine::sim::data::*;

fn pick_species(rng: &mut Rng, taken: &[SpeciesId]) -> SpeciesId {
    loop {
        let i = rng.below(SPECIES.len());
        let sp = SpeciesId(i as u16);
        let d = sp.data();
        if d.is_mega || d.base_species != sp {
            continue;
        }
        if taken.iter().any(|t| t.data().base_species == d.base_species) {
            continue;
        }
        return sp;
    }
}

fn random_set(rng: &mut Rng, sp: SpeciesId) -> SetMsg {
    let d = sp.data();
    // a matching Mega stone a third of the time, else any non-stone item, else none
    let stone = ITEMS.iter().position(|it| it.mega_stone.iter().any(|&(b, _)| b == sp));
    let item = match (stone, rng.below(3)) {
        (Some(s), 0) => Some(ITEMS[s].id.to_string()),
        (_, 2) => None,
        _ => loop {
            let it = &ITEMS[1 + rng.below(ITEMS.len() - 1)];
            if it.mega_stone.is_empty() {
                break Some(it.id.to_string());
            }
        },
    };
    let abs: Vec<AbilityId> = d.abilities.iter().copied().filter(|a| a.0 != 0).collect();
    let ability = if abs.is_empty() { "noability".to_string() } else { abs[rng.below(abs.len())].id().to_string() };
    let mut moves: Vec<String> = vec![];
    while moves.len() < 4 {
        let m = &MOVES[1 + rng.below(MOVES.len() - 1)];
        if m.id == "struggle" || moves.iter().any(|x| x == m.id) {
            continue;
        }
        moves.push(m.id.to_string());
    }
    let nature = rng.below(NATURES.len()) as u8;
    let mut points = [0u8; 6];
    let mut left = 66u32;
    for p in points.iter_mut() {
        let x = (rng.below(33) as u32).min(left);
        *p = x as u8;
        left -= x;
    }
    let st = [Stat::Hp, Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe];
    let mut stats = [0i32; 6];
    for i in 0..6 {
        stats[i] = stat_from_points(d.base_stats[i], points[i], st[i], nature) as i32;
    }
    let read = rng.below(2) == 0;
    SetMsg {
        species: d.id.to_string(),
        item,
        ability,
        moves,
        nature: if read { Some(NATURES[nature as usize].id.to_string()) } else { None },
        points: if read { Some(points.map(|x| x as i32)) } else { None },
        stats,
    }
}

fn msg(body: Body) -> Message {
    Message { v: CONTRACT_VERSION.into(), battle: "gen".into(), body }
}

fn ev(b: &mut Builder, e: Event) {
    b.apply(&Input::Msg(msg(Body::Events { events: vec![e], fallback: None })));
}

fn mon(side: Side, sp: &str) -> Mon {
    Mon::new(side, sp)
}

/// One random Battle and the request kind to ask for, or a preview-time Battle when `preview`.
pub fn random_battle(rng: &mut Rng, preview: bool) -> (Battle, ReqKind) {
    let mut taken = vec![];
    let mut team = vec![];
    for _ in 0..6 {
        let sp = pick_species(rng, &taken);
        taken.push(sp);
        team.push(random_set(rng, sp));
    }
    let mut opp = vec![];
    for _ in 0..6 {
        let sp = pick_species(rng, &taken);
        taken.push(sp);
        opp.push(sp);
    }
    let preview_ids: Vec<Option<String>> =
        opp.iter().map(|s| if rng.below(10) == 0 { None } else { Some(s.id().to_string()) }).collect();
    let mut b = Builder::new();
    b.apply(&Input::Msg(msg(Body::BattleStart { own_team: team.clone(), opp_preview: preview_ids })));
    if preview {
        return (b.battle, ReqKind::Turn);
    }
    let picks = crate::random::random_picks(6, rng);
    b.apply(&Input::Picks(Picks { picks: picks.to_vec() }));
    let own: Vec<String> = picks.iter().map(|&i| team[i as usize].species.clone()).collect();
    let opp_ids: Vec<String> = opp.iter().take(3).map(|s| s.id().to_string()).collect();
    ev(&mut b, Event::Enter { who: mon(Side::Own, &own[0]), forced: false });
    ev(&mut b, Event::Enter { who: mon(Side::Opp, &opp_ids[0]), forced: false });
    let turns = rng.below(6) as u32;
    let mut own_active = 0usize;
    let mut opp_active = 0usize;
    let mut kind = ReqKind::Turn;
    for t in 1..=turns + 1 {
        ev(&mut b, Event::Turn { n: t });
        if t == turns + 1 {
            break;
        }
        let n_ev = 1 + rng.below(6);
        for _ in 0..n_ev {
            let side = if rng.below(2) == 0 { Side::Own } else { Side::Opp };
            let name = if side == Side::Own { own[own_active].clone() } else { opp_ids[opp_active].clone() };
            let who = mon(side, &name);
            let e = match rng.below(16) {
                0 => {
                    let mv = if side == Side::Own {
                        let set = team.iter().find(|s| s.species == name).unwrap();
                        set.moves[rng.below(4)].clone()
                    } else {
                        MOVES[1 + rng.below(MOVES.len() - 1)].id.to_string()
                    };
                    Event::Move { who, mv, priority: None, called_by: None }
                }
                1 => Event::Boost { who, stats: vec![BOOST_IDS[rng.below(7)].to_string()], by: rng.below(5) as i32 - 2 },
                2 => {
                    let hp = match side {
                        Side::Own => {
                            let max = team.iter().find(|s| s.species == name).unwrap().stats[0];
                            let after = rng.below(max as usize + 1) as i32;
                            Hp::Own { before: max, after: after.max(1) }
                        }
                        Side::Opp => Hp::Opp { before_pct: 100.0, after_pct: (1 + rng.below(99)) as f64 },
                    };
                    Event::Hp { who, dir: HpDir::Lost, hp }
                }
                3 => Event::Status { who, status: ["brn", "par", "slp", "psn", "tox", "frz"][rng.below(6)].to_string(), effect: None },
                4 => Event::Weather { weather: ["raindance", "sunnyday", "sandstorm", "snowscape"][rng.below(4)].to_string(), what: StartEnd::Start },
                5 => Event::Field {
                    effect: ["electricterrain", "grassyterrain", "mistyterrain", "psychicterrain", "trickroom", "gravity"][rng.below(6)].to_string(),
                    what: StartEnd::Start,
                },
                6 => Event::SideCond { side, effect: SIDE_CONDS[rng.below(SIDE_CONDS.len())].to_string(), what: SideWhat::Start, mon: None },
                7 => Event::Volatile {
                    who,
                    effect: ["taunt", "substitute", "confusion", "yawn", "perishsong", "leechseed", "encore", "magnetrise"][rng.below(8)].to_string(),
                    detail: if rng.below(2) == 0 { Some(Detail::Num(1 + rng.below(3) as i64)) } else { None },
                },
                8 if side == Side::Opp => {
                    let sp = SpeciesId::from_id(&name).unwrap();
                    let abs: Vec<AbilityId> = sp.data().abilities.iter().copied().filter(|a| a.0 != 0).collect();
                    if abs.is_empty() {
                        continue;
                    }
                    Event::Ability { who, ability: abs[rng.below(abs.len())].id().to_string() }
                }
                9 if side == Side::Opp => {
                    Event::Item { who: Some(who), item: ITEMS[1 + rng.below(ITEMS.len() - 1)].id.to_string(), what: ItemWhat::Activated, of: None }
                }
                10 => {
                    // Mega Evolution when the stone allows it
                    let sp = SpeciesId::from_id(&name).unwrap();
                    let form = ITEMS.iter().flat_map(|it| it.mega_stone.iter()).find(|&&(b, _)| b == sp).map(|&(_, m)| m);
                    match form {
                        Some(f) if side == Side::Opp || team.iter().any(|s| s.species == name && s.item.as_deref().and_then(ItemId::from_id).map_or(false, |i| i.data().mega_stone.iter().any(|&(b, _)| b == sp))) => {
                            Event::Mega { who, form: Some(f.id().to_string()) }
                        }
                        _ => continue,
                    }
                }
                11 => {
                    // a switch
                    if side == Side::Own {
                        let next = (own_active + 1 + rng.below(2)) % 3;
                        ev(&mut b, Event::Withdraw { who: who.clone(), pivot: false });
                        own_active = next;
                        Event::Enter { who: mon(side, &own[next]), forced: false }
                    } else {
                        let next = (opp_active + 1 + rng.below(2)) % 3;
                        ev(&mut b, Event::Withdraw { who: who.clone(), pivot: false });
                        opp_active = next;
                        Event::Enter { who: mon(side, &opp_ids[next]), forced: false }
                    }
                }
                12 => Event::Types { who, ty: Some(TYPE_IDS[rng.below(18)].to_string()), lose: None, add: None },
                _ => continue,
            };
            ev(&mut b, e);
        }
    }
    // sometimes our active faints and a replacement is asked for
    if rng.below(8) == 0 {
        ev(&mut b, Event::Faint { who: mon(Side::Own, &own[own_active]) });
        kind = ReqKind::Replace;
    }
    (b.battle, kind)
}
