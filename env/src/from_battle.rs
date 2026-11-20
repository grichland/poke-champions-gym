//! An engine State from one seat's Battle and the caller's guesses for what the Battle does not show
//! (the opponent's unseen moves, items, abilities, natures, stat points and the unseen Pokémon brought).
//! CG only checks the guesses: a guess that is not legal, or that contradicts what the Battle revealed,
//! is reported and replaced by a fallback. Nothing is sampled here.
//!
//! What the Battle cannot know is set by fixed rules (see `UNKNOWN`), and `state_diff` names the parts of
//! two States that differ, to measure the build against the true State in self-play.

use battle::{Battle, ItemState, Lock, MonState, ReqKind, Side, VolDetail, ROOMS, SIDE_CONDS};
use cg_engine::sim::data::*;
use cg_engine::sim::outcome::Seeded;
use cg_engine::sim::state::{FieldCond, Mon, Request, SideCond, State, Volatile, MAX_VOLATILES, NO_MON, SIDES, TEAM};
use cg_engine::sim::MonSpec;

/// Stat points: at most this many in one stat and in all six together.
pub const POINTS_STAT: u8 = 32;
pub const POINTS_TOTAL: u16 = 66;

/// The State pieces a Battle cannot determine and the rule used for each.
pub const UNKNOWN: [(&str, &str); 21] = [
    ("opponent HP", "the Battle shows a whole percent: the middle of the HPs of the guessed max HP that show that percent (rounded up, 1..99 while not full)"),
    ("opposing hidden set", "the caller's guesses, checked; a guess not used falls back (revealed moves only, the item shown or none, the ability shown or the species' first, Serious, no points)"),
    ("gender", "not in the Battle: the caller's (`genders` for ours, `gender` for the opponent's), else the species' only gender, else male"),
    ("sleep turns left", "3 minus the 'cant slp' rows seen, at least 1"),
    ("freeze counter", "2"),
    ("confusion turns left", "4 minus the turns since it began, at least 1"),
    ("partial trap turns left", "5 minus the turns since it began, at least 1"),
    ("locked move turns left", "3 minus its uses so far, at least 1"),
    ("Substitute HP", "a fresh Substitute's (a quarter of max HP)"),
    ("Choice lock", "the Battle's, else the last move of a Choice item holder that has moved since it came in"),
    ("field effects an item extends", "the setter's item as known or guessed (Light Clay, weather rocks, Terrain Extender), else the normal duration"),
    ("types changed without a row", "the form's types (Double Shock, Burn Up)"),
    ("slot conditions", "Wish, Future Sight and Healing Wish pending are not kept"),
    ("team order", "Showdown's side.pokemon positions as if the active came in by one switch from the lead's position"),
    ("seeded source", "a new seeded source (`seed`)"),
    ("an opposing lock that ended while confused", "the log shows no end then (no fatigue row): the opponent stays locked into Outrage-like moves in the State"),
    ("Transform", "the copied form, types and moves (5 PP less those used); the stats are the opposing active's when it still is that form, else the form's with the user's own points"),
    ("one switch the game refused", "a refused bench member has no State flag: the State still offers it (a refused move is disabled; every switch refused traps)"),
    ("Illusion", "what the Battle saw is taken as true: a disguised lead is the Pokemon shown, so the true lead given second is moved first and reported as a 'brought' rejection"),
    ("a replace before its rows", "the Battle's last turn with our Pokemon on the field (one already sent in this turn included) to be replaced; with nobody left to send in no Game is built"),
    ("counters the log does not show", "attackedBy, last damage, a benched Pokemon's turn counters, action speed: as at a switch-in"),
];

/// One opposing preview member the caller says was brought, with its guessed set. `None` fields fall back.
#[derive(Clone, Debug, Default)]
pub struct OppGuess {
    /// preview index 0..5
    pub slot: u8,
    /// the full move list in set order (revealed moves included)
    pub moves: Vec<MoveId>,
    pub item: Option<ItemId>,
    pub ability: Option<AbilityId>,
    pub nature: Option<u8>,
    pub points: Option<[u8; 6]>,
    pub gender: Option<u8>,
}

/// A guess that was not used: which Pokémon, which field, why.
#[derive(Clone, Debug, PartialEq)]
pub struct Rejected {
    pub mon: String,
    pub field: &'static str,
    pub reason: String,
}

fn species_name(s: SpeciesId) -> String {
    s.id().to_string()
}

fn gender_of(sp: SpeciesId, g: Option<u8>) -> u8 {
    match sp.data().gender {
        "M" => 1,
        "F" => 2,
        "N" => 0,
        _ => g.filter(|x| *x == 1 || *x == 2).unwrap_or(1),
    }
}

fn points_ok(p: &[u8; 6]) -> Result<(), String> {
    if let Some(i) = p.iter().position(|&x| x > POINTS_STAT) {
        return Err(format!("{} points in stat {} (at most {})", p[i], i, POINTS_STAT));
    }
    let t: u16 = p.iter().map(|&x| x as u16).sum();
    if t > POINTS_TOTAL {
        return Err(format!("{} points in all (at most {})", t, POINTS_TOTAL));
    }
    Ok(())
}

/// Whether `item` may be held by `sp`: a Mega Stone only by the species it Mega Evolves.
fn item_ok(sp: SpeciesId, item: ItemId) -> Result<(), String> {
    let stones = item.data().mega_stone;
    if !stones.is_empty() && !stones.iter().any(|(from, _)| *from == sp) {
        return Err(format!("{} is the Mega Stone of another species", item.id()));
    }
    Ok(())
}

/// The abilities a set of `sp` may name: the species' own and its Mega forms' (Champions sets name the
/// Mega's ability for a Mega Stone holder).
fn abilities_of(sp: SpeciesId) -> Vec<AbilityId> {
    let mut v: Vec<AbilityId> = sp.data().abilities.iter().copied().filter(|a| a.0 != 0).collect();
    for i in 0..SPECIES.len() {
        let d = &SPECIES[i];
        if d.mega_from == Some(sp) {
            v.extend(d.abilities.iter().copied().filter(|a| a.0 != 0));
        }
    }
    v
}

fn ability_ok(sp: SpeciesId, a: AbilityId) -> Result<(), String> {
    if a.0 == 0 || !abilities_of(sp).contains(&a) {
        return Err(format!("{} cannot have {}", sp.id(), a.id()));
    }
    Ok(())
}

/// The opposing Pokémon's set: what the Battle revealed wins over the guess; a bad guess falls back.
/// Fallbacks: the legal guessed moves and the revealed ones (a Pokemon with none is an error); item: the
/// one shown, else none; ability: the one shown, else the species' first; nature: Serious; points: none.
fn opp_spec(m: &MonState, g: &OppGuess, rej: &mut Vec<Rejected>) -> Result<MonSpec, String> {
    let sp = m.species.or(m.preview).ok_or_else(|| format!("preview slot {}: no species", g.slot))?;
    let name = species_name(sp);
    let mut bad = |field: &'static str, reason: String| rej.push(Rejected { mon: name.clone(), field, reason });
    // moves: every revealed move, in the guess's order where it names them
    let revealed: Vec<MoveId> = m.moves.iter().flatten().copied().collect();
    let mut moves: Vec<MoveId> = Vec::new();
    for &mv in &g.moves {
        if mv.is_none() || moves.contains(&mv) {
            continue;
        }
        if !obs::can_learn(sp, mv) {
            bad("moves", format!("{} cannot learn {}", sp.id(), mv.id()));
            continue;
        }
        moves.push(mv);
    }
    for &mv in &revealed {
        if !moves.contains(&mv) {
            if moves.len() >= 4 {
                // a guessed move the Battle did not see gives way to one it saw
                let k = moves.iter().rposition(|x| !revealed.contains(x)).expect("a non-revealed move");
                let gone = moves.remove(k);
                bad("moves", format!("{} was used, not among the guessed moves; {} dropped", mv.id(), gone.id()));
            } else {
                bad("moves", format!("{} was used, not among the guessed moves; added", mv.id()));
            }
            moves.push(mv);
        }
    }
    if moves.len() > 4 {
        bad("moves", format!("{} moves guessed; the first four kept", moves.len()));
        moves.truncate(4);
    }
    if moves.is_empty() {
        return Err(format!("{}: no move known or guessed", sp.id()));
    }
    // item: what was shown wins
    let shown_item = match m.item {
        ItemState::Held(i) => Some(i),
        ItemState::Gone(Some(i)) => Some(i),
        _ => None,
    };
    // item as brought: the guess (an item shown may have been exchanged since: Trick, Switcheroo), except
    // that a Pokemon that Mega Evolved held its Mega Stone; the item now comes from the Battle
    let stone = m.form.filter(|f| f.data().is_mega).and_then(|f| f.data().mega_stone);
    let item = match (stone, g.item) {
        (Some(st), Some(x)) if x != st => {
            bad("item", format!("it Mega Evolved: it held {}, not {}", st.id(), x.id()));
            st
        }
        (Some(st), _) => st,
        (None, Some(x)) => match item_ok(sp, x) {
            Ok(()) => x,
            Err(e) => {
                bad("item", e);
                shown_item.unwrap_or(ItemId::NONE)
            }
        },
        (None, None) => shown_item.unwrap_or(ItemId::NONE),
    };
    // ability as brought: what was shown wins
    // a Mega shows its form's ability, not the one it was brought with
    let mega = m.mega || m.form.map_or(false, |f| f.data().is_mega);
    let shown_ab = m.base_ability.filter(|a| if mega { sp.data().abilities.contains(a) } else { abilities_of(sp).contains(a) });
    let ability = match (shown_ab, g.ability) {
        (Some(a), Some(x)) if x != a => {
            bad("ability", format!("{} was shown, not {}", a.id(), x.id()));
            a
        }
        (Some(a), _) => a,
        (None, Some(x)) => match ability_ok(sp, x) {
            Ok(()) => x,
            Err(e) => {
                bad("ability", e);
                sp.data().abilities[0]
            }
        },
        (None, None) => sp.data().abilities[0],
    };
    let nature = match g.nature {
        Some(n) if (n as usize) < NATURES.len() => n,
        Some(n) => {
            bad("nature", format!("nature {} unknown", n));
            nature_from_id("serious").unwrap_or(0)
        }
        None => nature_from_id("serious").unwrap_or(0),
    };
    let evs = match g.points {
        Some(p) => match points_ok(&p) {
            Ok(()) => p,
            Err(e) => {
                bad("points", e);
                [0; 6]
            }
        },
        None => [0; 6],
    };
    let mut mv = [MoveId::NONE; 4];
    for (i, x) in moves.iter().enumerate() {
        mv[i] = *x;
    }
    Ok(MonSpec { species: sp, ability, item, moves: mv, nmoves: moves.len() as u8, nature, evs, gender: gender_of(sp, g.gender) })
}

fn own_spec(b: &Battle, i: usize, gender: Option<u8>) -> MonSpec {
    let s = &b.sets[i];
    let mut mv = [MoveId::NONE; 4];
    let mut n = 0;
    for m in s.moves.iter().flatten() {
        mv[n] = *m;
        n += 1;
    }
    MonSpec {
        species: s.species,
        ability: s.ability,
        item: s.item.unwrap_or(ItemId::NONE),
        moves: mv,
        nmoves: n as u8,
        nature: s.nature.unwrap_or_else(|| nature_from_id("serious").unwrap_or(0)),
        evs: s.points.unwrap_or([0; 6]),
        gender: gender_of(s.species, gender),
    }
}

/// The opponent's HP from a shown percent and the max HP (the rule in `UNKNOWN`).
pub fn hp_from_pct(pct: f32, maxhp: u16) -> u16 {
    if pct <= 0.0 {
        return 0;
    }
    let p = pct.round() as u64;
    if p >= 100 {
        return maxhp;
    }
    let hits: Vec<u16> = (1..=maxhp).filter(|&h| emit::view::opp_pct(h, maxhp) as u64 == p).collect();
    match (hits.first(), hits.last()) {
        (Some(&a), Some(&z)) => ((a as u32 + z as u32 + 1) / 2) as u16,
        _ => ((pct as f64 * maxhp as f64 / 100.0).round() as u16).clamp(1, maxhp.saturating_sub(1).max(1)),
    }
}

/// Turns left of a timed effect that began on `start`: its duration (or the longer one an item gives)
/// less the turns since.
fn left(b: &Battle, c: Cond, start: u16, long: bool) -> i8 {
    let d = c.data().duration as i32;
    if d == 0 {
        return 0;
    }
    let d = if long { 8 } else { d };
    (d - b.elapsed(start) as i32).max(1) as i8
}

/// Whether the Pokémon that set a field effect holds the item that extends it.
fn extended(by: Option<battle::Who>, specs: &[[MonSpec; 6]; 2], sides: [usize; 2], c: Cond) -> bool {
    let Some(w) = by else { return false };
    let it = specs[w.side.idx()][w.slot as usize].item;
    let _ = sides;
    match c.id() {
        "reflect" | "lightscreen" | "auroraveil" => it.id() == "lightclay",
        "raindance" => it.id() == "damprock",
        "sunnyday" => it.id() == "heatrock",
        "sandstorm" => it.id() == "smoothrock",
        "snowscape" | "hail" => it.id() == "icyrock",
        "electricterrain" | "grassyterrain" | "mistyterrain" | "psychicterrain" => it.id() == "terrainextender",
        _ => false,
    }
}

/// A State for the decision `kind` of our seat `seat` (0 or 1 in the new State) from our Battle `b`,
/// the opponent's three brought (`brought`: preview indices, lead first, with their guessed sets) and our
/// genders by team index. Returns the State and the guesses not used.
pub fn state_from_battle(b: &Battle, kind: ReqKind, seat: usize, brought: &[OppGuess], genders: &[Option<u8>; 6], seed: u64) -> Result<(State, Vec<Rejected>), String> {
    if !b.started {
        return Err("the Battle has no battle_start".into());
    }
    if b.result.is_some() {
        return Err("the battle has ended".into());
    }
    let own_picks: Vec<u8> = b.picks.iter().flatten().copied().collect();
    if own_picks.len() != TEAM {
        return Err("our picks are not known yet (team preview)".into());
    }
    if b.active(Side::Own).is_none() && b.sides[0].active.slot.is_none() {
        return Err("no Pokémon has entered yet".into());
    }
    let mut rej = Vec::new();
    // the opponent's three: every one seen must be among them, the lead first
    if brought.len() != TEAM {
        return Err(format!("{} opposing Pokémon given, need the three brought", brought.len()));
    }
    let opp = &b.sides[1];
    let mut slots: Vec<u8> = brought.iter().map(|g| g.slot).collect();
    if slots.iter().any(|&s| s >= 6) || slots[0] == slots[1] || slots[0] == slots[2] || slots[1] == slots[2] {
        return Err(format!("opposing slots {:?}: need three distinct preview indices 0..5", slots));
    }
    let seen: Vec<u8> = (0..6u8).filter(|&i| opp.team[i as usize].seen).collect();
    for &s in &seen {
        if !slots.contains(&s) {
            return Err(format!("{} was seen but is not among the opposing Pokémon given", opp.team[s as usize].species.map_or("?".into(), species_name)));
        }
    }
    if let Some(lead) = (0..6u8).find(|&i| opp.team[i as usize].was_lead) {
        if slots[0] != lead {
            rej.push(Rejected { mon: opp.team[lead as usize].species.map_or("?".into(), species_name), field: "brought", reason: "it led: put first".into() });
            let k = slots.iter().position(|&x| x == lead).expect("seen lead is given");
            slots.swap(0, k);
        }
    }
    let order: Vec<&OppGuess> = slots.iter().map(|s| brought.iter().find(|g| g.slot == *s).expect("given")).collect();
    let mut own_team = [MonSpec::default(); 6];
    for i in 0..b.n_own as usize {
        own_team[i] = own_spec(b, i, genders[i]);
    }
    let mut opp_team = [MonSpec::default(); 6];
    for (k, g) in order.iter().enumerate() {
        let _ = k;
        opp_team[g.slot as usize] = opp_spec(&opp.team[g.slot as usize], g, &mut rej)?;
    }
    let teams_by_side = [own_team, opp_team];
    let pick_own = [own_picks[0], own_picks[1], own_picks[2]];
    let pick_opp = [slots[0], slots[1], slots[2]];
    // State sides: our seat holds our team
    let (t0, t1, p0, p1) = if seat == 0 { (&own_team, &opp_team, pick_own, pick_opp) } else { (&opp_team, &own_team, pick_opp, pick_own) };
    let mut rng = Seeded::new(seed);
    let mut st = State::start([&t0[..], &t1[..]], [p0, p1], &mut rng)?;
    let sides = [seat, 1 - seat];
    // at a replace request asked before the rows of its turn, the Pokemon a replacement already sent in
    // this turn is the one on the field (the one to be replaced now)
    let own_field = if kind == ReqKind::Replace { battle::legal::on_field(b) } else { b.sides[0].active.slot };
    let fields = [own_field, b.sides[1].active.slot];
    let slot_of = |bi: usize, picks: &[u8; 3]| fields[bi].and_then(|t| picks.iter().position(|&x| x == t)).unwrap_or(0);
    let act = [slot_of(0, &pick_own), slot_of(1, &pick_opp)];
    for (bi, picks) in [(0usize, pick_own), (1usize, pick_opp)] {
        let foe = (sides[1 - bi] * 4 + act[1 - bi]) as u8;
        set_side(&mut st, b, bi, sides[bi], &picks, &teams_by_side, sides, foe, fields[bi]);
    }
    for bi in 0..2 {
        transformed(&mut st, b, bi, sides);
    }
    set_field(&mut st, b, &teams_by_side, sides);
    st.turn = b.turn.max(1);
    st.mid_turn = false;
    st.queue.clear();
    st.nfaints = 0;
    st.active_pokemon = NO_MON;
    st.active_target = NO_MON;
    // what a turn start sets: trapped, disabled moves, the move request
    st.turn_start_flags();
    if kind == ReqKind::Replace {
        // our active must be replaced: it fainted (or is leaving) and the other side waits
        st.sides[1 - seat].request = Request::Wait;
        st.request = Request::Switch;
        let sd = &mut st.sides[seat];
        sd.request = Request::Switch;
        let a = sd.active as usize;
        if !sd.mons[a].fainted {
            sd.mons[a].switch_flag = true;
        }
        // the choice after our Revival Blessing: a fainted Pokemon, when the Battle offers one
        let l = battle::legal(b, kind);
        let offers_fainted = (0..TEAM).any(|i| l.mask[8 + i] && sd.mons[i].fainted);
        if b.ctx.own_choice == Some(moves::REVIVALBLESSING) && offers_fainted {
            sd.mons[a].switch_flag_move = moves::REVIVALBLESSING;
            let n = sd.nslot_conds as usize;
            if n < sd.slot_conds.len() {
                sd.slot_conds[n] = SideCond { id: conds::REVIVALBLESSING, duration: 1, source: (seat * 4 + a) as u8, ..Default::default() };
                sd.nslot_conds += 1;
            }
        }
    }
    within_battle(&mut st, b, kind, seat, &pick_own);
    // a Game must be steppable: every seat the State asks has something to choose
    let o = st.options();
    for side in 0..SIDES {
        let mut any = false;
        crate::batch::each_turn_action(&o[side], |_| any = true);
        if matches!(o[side].kind, cg_engine::sim::Kind::Turn | cg_engine::sim::Kind::Replace) && !any {
            let who = if side == seat { "our seat" } else { "the opposing seat" };
            return Err(format!(
                "{} must {} but has nothing to choose (a replace request with no Pokemon left to send in: the rows of this turn are not read yet)",
                who,
                if o[side].kind == cg_engine::sim::Kind::Replace { "replace" } else { "move" }
            ));
        }
    }
    Ok((st, rej))
}

/// Our seat may not be offered what our Battle knows is unavailable: a move slot the Battle forbids (a
/// refusal at this request, or a rule the State has not applied) is disabled, and with every switch
/// forbidden at a turn our active is trapped. A single switch the Battle forbids (a refused one) has no
/// State flag and stays (`UNKNOWN`).
fn within_battle(st: &mut State, b: &Battle, kind: ReqKind, seat: usize, picks: &[u8; 3]) {
    let l = battle::legal(b, kind);
    if !l.mask.iter().any(|x| *x) {
        return;
    }
    let sd = &mut st.sides[seat];
    if !sd.has_active {
        return;
    }
    let active = sd.active as usize;
    let team = picks[active] as usize;
    let m = &mut sd.mons[active];
    let transformed = b.sides[0].active.transformed.is_some();
    if kind == ReqKind::Turn {
        match l.forced {
            // nothing usable that the Battle knows of (Imprison of moves it saw, no PP): Struggle
            Some(battle::Forced::Struggle) => {
                for k in 0..m.nmoves as usize {
                    m.moves[k].disabled = true;
                }
            }
            Some(battle::Forced::Move(mv)) => {
                if m.moves[..m.nmoves as usize].iter().any(|x| x.id == mv) {
                    for k in 0..m.nmoves as usize {
                        m.moves[k].disabled |= m.moves[k].id != mv;
                    }
                }
            }
            Some(battle::Forced::Recharge) => {}
            None if (0..8).any(|i| l.mask[i]) => {
                // the Battle's move slots: the copied ones while transformed
                let ids = if transformed { b.sides[0].active.t_moves } else { b.sets[team].moves };
                for k in 0..m.nmoves as usize {
                    let id = m.moves[k].id;
                    if let Some(j) = ids.iter().position(|x| *x == Some(id)) {
                        if !l.mask[j] && !l.mask[4 + j] {
                            m.moves[k].disabled = true;
                        }
                    }
                }
            }
            None => {}
        }
    }
    if kind == ReqKind::Turn && !(8..11).any(|i| l.mask[i]) {
        m.trapped = true;
    }
}

/// One side of the State from side `bi` of the Battle (0 ours, 1 the opponent's).
#[allow(clippy::too_many_arguments)]
fn set_side(st: &mut State, b: &Battle, bi: usize, side: usize, picks: &[u8; 3], specs: &[[MonSpec; 6]; 2], sides: [usize; 2], foe: u8, active_team: Option<u8>) {
    let bs = &b.sides[bi];
    let sd = &mut st.sides[side];
    let mut active = 0usize;
    for (slot, &ti) in picks.iter().enumerate() {
        let bm = &bs.team[ti as usize];
        let m = &mut sd.mons[slot];
        if Some(ti) == active_team {
            active = slot;
        }
        // HP
        m.hp = if bm.fainted {
            0
        } else if bi == 0 {
            ((bm.hp as f64 * m.maxhp as f64 / 100.0).round() as u16).min(m.maxhp)
        } else {
            hp_from_pct(bm.hp, m.maxhp)
        };
        m.fainted = bm.fainted;
        m.status = if bm.fainted { Status::None } else { bm.status };
        m.status_time = match bm.status {
            Status::Slp => (3 - bm.status_count as i8).max(1),
            Status::Frz => 2,
            _ => 0,
        };
        m.tox_stage = if bm.status == Status::Tox { bm.status_count } else { 0 };
        // the form and Mega state
        if let Some(f) = bm.form {
            if f != m.species {
                if f.data().is_mega {
                    mega_into(m, f);
                } else {
                    // another form (Disguise busted, ...): its types; the stats stay
                    let d = f.data();
                    m.species = f;
                    let nt = if d.types[1] == Type::Typeless { 1 } else { 2 };
                    m.set_types(&d.types[..nt]);
                }
            }
        }
        // item now
        match bm.item {
            ItemState::Held(i) => m.item = i,
            ItemState::Gone(x) => {
                m.item = ItemId::NONE;
                if let Some(i) = x {
                    m.last_item = i;
                }
            }
            ItemState::Unknown => {}
        }
        if let Some(a) = bm.ability {
            if bm.form.map_or(true, |f| !f.data().is_mega) {
                m.ability = a;
            }
        }
        // PP
        let pp = b.pp_left(battle::Who { side: if bi == 0 { Side::Own } else { Side::Opp }, slot: ti });
        let ids = if bi == 0 { b.sets[ti as usize].moves } else { bm.moves };
        for k in 0..m.nmoves as usize {
            let id = m.moves[k].id;
            if let Some(j) = ids.iter().position(|x| *x == Some(id)) {
                if let Some(l) = pp[j] {
                    m.moves[k].pp = l.min(m.moves[k].max_pp);
                    m.base_moves[k].pp = m.moves[k].pp;
                }
            }
        }
        m.is_active = false;
        m.boosts = [0; 7];
    }
    sd.mega_spent = bs.mega_used;
    if bs.mega_used {
        // a side that Mega Evolved has no Mega Evolution left
        for m in sd.mons.iter_mut() {
            m.can_mega = None;
        }
    }
    sd.total_fainted = sd.mons.iter().filter(|m| m.fainted).count() as u8;
    sd.pokemon_left = TEAM as u8 - sd.total_fainted;
    // the active Pokémon
    sd.active = active as u8;
    sd.has_active = true;
    // Showdown swaps the positions of the two Pokémon of a switch: as if the active came in once
    let mut order = [0u8, 1, 2];
    order.swap(0, active);
    sd.order = order;
    let a = &bs.active;
    {
        let m = &mut sd.mons[active];
        m.is_active = !m.fainted;
        m.boosts = a.boosts;
        m.last_move = a.last_move.unwrap_or(MoveId::NONE);
        m.last_move_used = m.last_move;
        m.active_move_actions = a.move_actions as u16;
        // turns begun since it came in (a lead's entry is written under turn 1's heading)
        let lead_entry = bs.team[active_team.unwrap_or(0) as usize].was_lead && bs.team[active_team.unwrap_or(0) as usize].entries == 1 && a.entered <= 1;
        m.active_turns = (b.turn + lead_entry as u16).saturating_sub(a.entered) as u16;
        m.newly_switched = false;
        if let Some(t) = a.types {
            let n = if t[1] == Type::Typeless || t[1] == t[0] { 1 } else { 2 };
            m.set_types(&t[..n]);
        }
        if let Some(t) = a.added_type {
            m.added_type = t;
        }
    }
    for (i, m) in sd.mons.iter_mut().enumerate() {
        if i != active {
            m.newly_switched = false;
            m.active_turns = 0;
            m.last_move = MoveId::NONE;
            m.last_move_used = MoveId::NONE;
            m.active_move_actions = 0;
        }
    }
    // volatiles
    sd.volatiles = [Volatile::default(); MAX_VOLATILES];
    sd.nvolatiles = 0;
    let add = |sd: &mut cg_engine::sim::state::Side, v: Volatile| {
        if (sd.nvolatiles as usize) < MAX_VOLATILES && !sd.has_vol(v.id) {
            let n = sd.nvolatiles as usize;
            sd.volatiles[n] = v;
            sd.nvolatiles += 1;
        }
    };
    let me = side as u8 * 4 + active as u8;
    for v in a.vols.iter().flatten() {
        let c = v.cond;
        let mut x = Volatile { id: c, duration: left(b, c, v.start, false), source: if v.by.is_some() { foe } else { me }, ..Default::default() };
        match v.detail {
            VolDetail::Move(mv) => x.mv = mv,
            VolDetail::Num(n) => x.data = n as i8,
            VolDetail::None => {}
        }
        match c.id() {
            "confusion" => {
                x.data = (4 - b.elapsed(v.start) as i8).max(1);
                x.duration = 0;
            }
            // its duration is cut by one when the target moves after it (the usual case)
            "disable" => x.duration = (x.duration - 1).max(1),
            "partiallytrapped" => {
                x.duration = (5 - b.elapsed(v.start) as i8).max(1);
                x.data = 8;
            }
            "substitute" => x.hp = 2 * (sd.mons[active].maxhp / 4) as i16,
            "stall" => x.counter = 3,
            _ => {}
        }
        add(sd, x);
    }
    match a.lock {
        Lock::Locked { mv, uses, .. } => add(sd, Volatile { id: conds::LOCKEDMOVE, duration: 1, mv, data: (3 - uses as i8).max(1), source: me, ..Default::default() }),
        Lock::Charging { mv: Some(mv), .. } => {
            add(sd, Volatile { id: conds::TWOTURNMOVE, duration: 1, mv, source: foe, ..Default::default() });
            if let Some(c) = Cond::from_id(mv.id()) {
                add(sd, Volatile { id: c, duration: (c.data().duration as i8 - 1).max(0), source: foe, ..Default::default() });
            }
        }
        Lock::Recharge { .. } => add(sd, Volatile { id: conds::MUSTRECHARGE, duration: 1, source: me, ..Default::default() }),
        _ => {}
    }
    // a Choice item's lock: ours as the Battle tracks it; the opponent's also inferred from the last move
    // of a holder that has moved since it came in (not a move that exchanged the item: the lock comes
    // from a move used while holding it)
    let m = sd.mons[active];
    let choice = matches!(m.item.id(), "choiceband" | "choicespecs" | "choicescarf");
    let exchanged = matches!(a.last_move.map(|x| x.id()), Some("trick" | "switcheroo" | "bestow"));
    let lock = if bi == 0 { a.choice_lock } else { a.choice_lock.or(a.last_move.filter(|_| choice && a.move_actions > 0 && !exchanged)) };
    if let Some(mv) = lock.filter(|mv| !mv.is_none() && *mv != moves::STRUGGLE) {
        if choice || a.choice_lock.is_some() {
            add(sd, Volatile { id: conds::CHOICELOCK, mv, source: me, ..Default::default() });
        }
    }
    // what the last turn leaves that the log does not announce
    let last = &bs.last;
    if last.act == battle::Act::Move && last.mon == active_team && !last.failed {
        match last.mv.map(|x| x.id()) {
            Some("glaiverush") => add(sd, Volatile { id: conds::GLAIVERUSH, duration: 0, source: me, ..Default::default() }),
            Some("protect" | "detect" | "kingsshield" | "spikyshield" | "banefulbunker" | "silktrap" | "burningbulwark" | "obstruct" | "endure") => {
                add(sd, Volatile { id: conds::STALL, duration: 1, counter: 3, source: me, ..Default::default() })
            }
            _ => {}
        }
    }
    if m.ability == abilities::UNBURDEN && m.item.is_none() && matches!(bs.team[active_team.unwrap_or(0) as usize].item, ItemState::Gone(_)) {
        add(sd, Volatile { id: conds::UNBURDEN, duration: 0, source: me, ..Default::default() });
    }
    // side conditions
    sd.conds = [SideCond::default(); cg_engine::sim::state::MAX_SIDE_CONDS];
    sd.nconds = 0;
    for (i, id) in SIDE_CONDS.iter().enumerate() {
        let Some(c) = bs.conds[i] else { continue };
        let Some(cond) = Cond::from_id(id) else { continue };
        let hazard = matches!(*id, "spikes" | "toxicspikes" | "stealthrock" | "stickyweb");
        let n = sd.nconds as usize;
        let long = extended(c.by, specs, sides, cond);
        sd.conds[n] = SideCond { id: cond, layers: if hazard { c.layers } else { 0 }, duration: left(b, cond, c.start, long), source: NO_MON, ..Default::default() };
        sd.nconds += 1;
    }
    sd.nslot_conds = 0;
}

/// A Pokemon transformed on the field (Battle side `bi`): the form, types, stats and moves it copied, the
/// copied moves with 5 PP less those used. The stats are the opposing active's when it is that form, else
/// that form's with the user's own points and nature.
fn transformed(st: &mut State, b: &Battle, bi: usize, sides: [usize; 2]) {
    let a = &b.sides[bi].active;
    let Some(sp) = a.transformed else { return };
    let (side, foe) = (sides[bi], sides[1 - bi]);
    if !st.sides[side].has_active || st.sides[side].active_mon().fainted {
        return;
    }
    let tm = st.sides[foe].active_mon();
    let target = (tm.species == sp).then_some(*tm);
    let m = st.sides[side].active_mon_mut();
    let mut own = [MoveId::NONE; 4];
    for i in 0..m.nmoves as usize {
        own[i] = m.base_moves[i].id;
    }
    let spec = MonSpec { species: sp, ability: sp.data().abilities[0], item: m.item, moves: own, nmoves: m.nmoves, nature: m.nature, evs: m.evs, gender: m.gender };
    let form = cg_engine::sim::options::build_mon(&spec);
    m.base_moves = m.moves;
    m.transformed = true;
    m.species = sp;
    m.types = target.map_or(form.types, |t| t.types);
    m.ntypes = target.map_or(form.ntypes, |t| t.ntypes);
    m.stats = target.map_or(form.stats, |t| t.stats);
    m.weight_hg = target.map_or(form.weight_hg, |t| t.weight_hg);
    if let Some(t) = target {
        m.ability = t.ability;
    }
    let mut slots = [cg_engine::sim::state::MoveSlot::default(); 4];
    let mut n = 0;
    for (i, mv) in a.t_moves.iter().enumerate() {
        if let Some(id) = mv {
            let pp = id.data().max_pp.min(5);
            slots[n] = cg_engine::sim::state::MoveSlot { id: *id, pp: pp.saturating_sub(a.t_pp_used[i]), max_pp: pp, ..Default::default() };
            n += 1;
        }
    }
    if n > 0 {
        m.moves = slots;
        m.nmoves = n as u8;
    }
}

/// Mega Evolution of `m` into `f`: species, types, ability and stats of the form.
fn mega_into(m: &mut Mon, f: SpeciesId) {
    let mut mv = [MoveId::NONE; 4];
    for i in 0..m.nmoves as usize {
        mv[i] = m.base_moves[i].id;
    }
    let spec = MonSpec { species: f, ability: f.data().abilities[0], item: m.item, moves: mv, nmoves: m.nmoves, nature: m.nature, evs: m.evs, gender: m.gender };
    let n = cg_engine::sim::options::build_mon(&spec);
    m.species = f;
    m.base_species = f;
    m.types = n.types;
    m.ntypes = n.ntypes;
    m.ability = n.ability;
    m.base_ability = n.ability;
    m.weight_hg = n.weight_hg;
    m.stats = n.stats;
    m.speed = n.speed;
    m.can_mega = None;
}

fn set_field(st: &mut State, b: &Battle, specs: &[[MonSpec; 6]; 2], sides: [usize; 2]) {
    let f = &b.field;
    st.field.weather = match f.weather {
        Some(w) => FieldCond { id: w.cond, duration: left(b, w.cond, w.start, extended(w.by, specs, sides, w.cond)), targeted: false },
        None => FieldCond::default(),
    };
    st.field.terrain = match f.terrain {
        Some(t) => FieldCond { id: t.cond, duration: left(b, t.cond, t.start, extended(t.by, specs, sides, t.cond)), targeted: false },
        None => FieldCond::default(),
    };
    st.field.pseudo = Default::default();
    st.field.npseudo = 0;
    for (i, r) in ROOMS.iter().enumerate() {
        if let (Some(t), Some(c)) = (f.rooms[i], Cond::from_id(r)) {
            let n = st.field.npseudo as usize;
            st.field.pseudo[n] = FieldCond { id: c, duration: left(b, c, t.start, false), targeted: false };
            st.field.npseudo += 1;
        }
    }
}

/// The parts of two States that differ, by name (empty when they agree on everything named), with our
/// side (`seat`) named `own_` and the other `opp_`. `a` is compared with `b` side by side, slot by slot.
pub fn state_diff(a: &State, b: &State, seat: usize) -> Vec<String> {
    state_diff_why(a, b, seat).into_iter().map(|(k, _)| k).collect()
}

/// `state_diff` with each difference's values (built, true).
pub fn state_diff_why(a: &State, b: &State, seat: usize) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    put(&mut out, "turn", a.turn != b.turn);
    put(&mut out, "weather", a.field.weather.id != b.field.weather.id);
    put(&mut out, "weather_turns", a.field.weather.id == b.field.weather.id && a.field.weather.duration != b.field.weather.duration);
    put(&mut out, "terrain", a.field.terrain.id != b.field.terrain.id);
    put(&mut out, "terrain_turns", a.field.terrain.id == b.field.terrain.id && a.field.terrain.duration != b.field.terrain.duration);
    put(&mut out, "rooms", a.field.pseudo[..a.field.npseudo as usize].iter().map(|c| (c.id, c.duration)).ne(b.field.pseudo[..b.field.npseudo as usize].iter().map(|c| (c.id, c.duration))));
    for s in 0..SIDES {
        let (x, y) = (&a.sides[s], &b.sides[s]);
        let p = if s == seat { "own_" } else { "opp_" };
        let k = |n: &str| format!("{}{}", p, n);
        put(&mut out, &k("active"), x.active != y.active || x.has_active != y.has_active);
        put(&mut out, &k("mega_spent"), x.mega_used() != y.mega_used());
        if x.order != y.order {
            out.push((k("order"), format!("built {:?} true {:?} active {}", x.order, y.order, y.active)));
        }
        let xc: Vec<_> = x.conds().iter().map(|c| (c.id, c.layers.max(1))).collect();
        let yc: Vec<_> = y.conds().iter().map(|c| (c.id, c.layers.max(1))).collect();
        let mut xs = xc.clone();
        let mut ys = yc.clone();
        xs.sort();
        ys.sort();
        put(&mut out, &k("side_conds"), xs != ys);
        let mut xt: Vec<_> = x.conds().iter().map(|c| (c.id, c.duration)).collect();
        let mut yt: Vec<_> = y.conds().iter().map(|c| (c.id, c.duration)).collect();
        xt.sort();
        yt.sort();
        put(&mut out, &k("side_cond_turns"), xs == ys && xt != yt);
        put(&mut out, &k("slot_conds"), x.slot_conds().iter().map(|c| c.id).ne(y.slot_conds().iter().map(|c| c.id)));
        let mut xv: Vec<Cond> = x.volatiles().iter().map(|v| v.id).collect();
        let mut yv: Vec<Cond> = y.volatiles().iter().map(|v| v.id).collect();
        xv.sort();
        yv.sort();
        if xv != yv {
            let extra: Vec<&str> = xv.iter().filter(|c| !yv.contains(c)).map(|c| c.id()).collect();
            let miss: Vec<&str> = yv.iter().filter(|c| !xv.contains(c)).map(|c| c.id()).collect();
            for e in extra {
                out.push((format!("{}vol_extra:{}", p, e), String::new()));
            }
            for m in miss {
                out.push((format!("{}vol_missing:{}", p, m), format!("{:?}", y.vol(Cond::from_id(m).unwrap()))));
            }
        } else {
            for v in x.volatiles() {
                let w = y.vol(v.id).unwrap();
                if v.duration != w.duration || v.data != w.data || v.mv != w.mv || v.counter != w.counter || v.hp != w.hp {
                    out.push((format!("{}vol_state:{}", p, v.id.id()), format!("built {:?} true {:?}", v, w)));
                }
            }
        }
        for i in 0..TEAM {
            let (m, n) = (&x.mons[i], &y.mons[i]);
            let mk = |f: &str| format!("{}mon_{}", p, f);
            put(&mut out, &mk("species"), m.set_species != n.set_species);
            if m.set_species != n.set_species {
                continue;
            }
            if m.species != n.species {
                out.push((mk("form"), format!("built {} true {} active {}", m.species.id(), n.species.id(), n.is_active)));
            }
            if m.hp != n.hp {
                out.push((mk("hp"), format!("{} built {}/{} true {}/{}", m.set_species.id(), m.hp, m.maxhp, n.hp, n.maxhp)));
            }
            put(&mut out, &mk("maxhp_stats"), m.maxhp != n.maxhp || m.stats != n.stats);
            put(&mut out, &mk("fainted"), m.fainted != n.fainted);
            put(&mut out, &mk("status"), m.status != n.status);
            if m.status == n.status && m.status_time != n.status_time {
                out.push((mk("status_time"), format!("{:?} built {} true {}", m.status, m.status_time, n.status_time)));
            }
            put(&mut out, &mk("tox_stage"), m.tox_stage != n.tox_stage);
            put(&mut out, &mk("item"), m.item != n.item);
            if m.ability != n.ability || m.base_ability != n.base_ability {
                out.push((mk("ability"), format!("{} built {} {} true {} {}", m.set_species.id(), m.ability.id(), m.base_ability.id(), n.ability.id(), n.base_ability.id())));
            }
            put(&mut out, &mk("moves"), m.moves.iter().map(|x| x.id).ne(n.moves.iter().map(|x| x.id)));
            put(&mut out, &mk("pp"), m.moves.iter().map(|x| x.pp).ne(n.moves.iter().map(|x| x.pp)));
            put(&mut out, &mk("boosts"), m.boosts != n.boosts);
            if m.types != n.types || m.ntypes != n.ntypes || m.added_type != n.added_type {
                out.push((mk("types"), format!("{} built {:?} {} {:?} true {:?} {} {:?}", m.species.id(), m.types, m.ntypes, m.added_type, n.types, n.ntypes, n.added_type)));
            }
            put(&mut out, &mk("last_move"), m.last_move != n.last_move);
            put(&mut out, &mk("gender"), m.gender != n.gender);
            if n.is_active && (m.active_turns != n.active_turns || m.active_move_actions != n.active_move_actions) {
                out.push((mk("active_turns"), format!("built {} {} true {} {} active {}", m.active_turns, m.active_move_actions, n.active_turns, n.active_move_actions, n.is_active)));
            }
        }
    }
    out
}

fn put(out: &mut Vec<(String, String)>, k: &str, x: bool) {
    if x {
        out.push((k.to_string(), String::new()));
    }
}

/// A Game at our open request of a live Battle (`builder`: the Session's Builder; `seq` and `kind`: the open
/// request), with views: our seat's Battle is the live one and goes on with the rows of each step; the
/// opposing seat's Battle is not known (`Views::known`). `seat`: our seat in the new State.
#[allow(clippy::too_many_arguments)]
pub fn game_from_builder(builder: &battle::Builder, kind: ReqKind, seq: i64, seat: usize, brought: &[OppGuess], genders: &[Option<u8>; 6], seed: u64, tag: &str) -> Result<(crate::Game, crate::views::Tracked, Vec<Rejected>), String> {
    let (st, rej) = state_from_battle(&builder.battle, kind, seat, brought, genders, seed)?;
    let mut seats = emit::Seats::new(tag.to_string(), false);
    seats.replace_before_rows = true;
    seats.builders[seat] = builder.clone();
    seats.emitter = emit::Emitter::new(&st);
    seats.seq = seq.max(0);
    let mut v = crate::views::Views::live(seats, seat, kind, seq);
    v.opts[seat] = crate::batch::battle_options(&builder.battle, if kind == ReqKind::Replace { cg_engine::sim::Kind::Replace } else { cg_engine::sim::Kind::Turn });
    let mut any = false;
    crate::batch::each_turn_action(&v.opts[seat], |_| any = true);
    if !any {
        v.opts[seat] = st.options()[seat];
    }
    let game = crate::Game { state: st, rng: Seeded::new(seed) };
    Ok((game, crate::views::Tracked::new(st, v), rej))
}
