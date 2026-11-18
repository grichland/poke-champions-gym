//! Endgame chains (`pcsa/players/chain.py`): with at most two members left per side, the rest of the game
//! as a run of 1v1 duels. Pure arithmetic over an attack matrix: an expected-value race where each side
//! attacks (a KO first) or recovers; order by priority then speed, ties to the opponent; a duel past
//! `HORIZON` turns is a draw; a voluntary switch costs a turn, replacements after a faint are free; the
//! opponent answers after seeing our choice. No engine calls; bounded by the member counts.

pub const MAX_MEMBERS: usize = 4;
pub const MAX_ATTACKS: usize = 4;
/// Accuracy from which a KO-capable hit is taken to land.
const LIKELY: f32 = 0.5;
/// HP fraction under which recovery is considered.
const RECOVER_BELOW: f32 = 0.5;
pub const HORIZON: u32 = 30;

/// One move against one target; fractions of the target's max HP. `recover` marks the recovery action.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Attack {
    pub id: u16,
    pub fraction: f32,
    pub priority: i8,
    pub accuracy: f32,
    pub ko_fraction: f32,
    pub recover: bool,
}

const RECOVER_ACTION: Attack = Attack { id: 0, fraction: 0.0, priority: 0, accuracy: 1.0, ko_fraction: 0.0, recover: true };

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Attacks {
    pub a: [Attack; MAX_ATTACKS],
    pub n: u8,
}

impl Attacks {
    pub fn push(&mut self, x: Attack) {
        if (self.n as usize) < MAX_ATTACKS {
            self.a[self.n as usize] = x;
            self.n += 1;
        }
    }
    pub fn as_slice(&self) -> &[Attack] {
        &self.a[..self.n as usize]
    }
}

/// One member: HP fraction, speed (higher first), recovery, per-turn HP change, loss per hit it takes.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Combatant {
    pub hp: f32,
    pub speed: i32,
    pub recover: f32,
    pub residual: f32,
    pub passive_on_hit: f32,
    pub can_switch_on_faint: bool,
}

/// Attacks both ways keyed [our i][their j].
#[derive(Clone, Copy, Debug, Default)]
pub struct Matchups {
    pub ours: [[Attacks; MAX_MEMBERS]; MAX_MEMBERS],
    pub theirs: [[Attacks; MAX_MEMBERS]; MAX_MEMBERS],
}

/// What an option leads to: we win, and the HP margin; `first` our first action when we stay.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChainResult {
    pub wins: bool,
    pub margin: f32,
    pub first: Option<Attack>,
}

pub fn lands_ko(a: &Attack, target_hp: f32) -> bool {
    a.fraction > 0.0 && a.accuracy >= LIKELY && a.ko_fraction >= target_hp
}

/// First maximal element by a key.
fn first_max<T: Copy, K: PartialOrd>(xs: &[T], key: impl Fn(&T) -> K) -> Option<T> {
    let mut best: Option<(T, K)> = None;
    for x in xs {
        let k = key(x);
        if best.as_ref().map_or(true, |(_, b)| k > *b) {
            best = Some((*x, k));
        }
    }
    best.map(|x| x.0)
}

/// A KO if one exists (highest priority, then accuracy, then damage), else the most expected damage.
pub fn best_attack(attacks: &[Attack], target_hp: f32) -> Option<Attack> {
    if attacks.is_empty() {
        return None;
    }
    let mut kos = Attacks::default();
    for a in attacks {
        if lands_ko(a, target_hp) {
            kos.push(*a);
        }
    }
    if kos.n > 0 {
        return first_max(kos.as_slice(), |a| (a.priority, a.accuracy, a.fraction));
    }
    first_max(attacks, |a| (a.fraction * a.accuracy, a.priority))
}

#[derive(Clone, Copy)]
struct Fighter {
    unit: Combatant,
    attacks: Attacks,
    hp: f32,
}

/// Attack, or recover when under half, the healed HP survives the foe's best hit and the heal outweighs it.
fn choose(me: &Fighter, foe: &Fighter) -> Option<Attack> {
    let attack = best_attack(me.attacks.as_slice(), foe.hp);
    if let Some(a) = attack {
        if lands_ko(&a, foe.hp) {
            return attack;
        }
    }
    if me.unit.recover <= 0.0 || me.hp >= RECOVER_BELOW {
        return attack;
    }
    let healed = (me.hp + me.unit.recover).min(1.0);
    if let Some(t) = best_attack(foe.attacks.as_slice(), healed) {
        if lands_ko(&t, healed) || t.fraction * t.accuracy >= healed - me.hp {
            return attack;
        }
    }
    Some(RECOVER_ACTION)
}

fn act(actor: &mut Fighter, target: &mut Fighter, action: &Attack) {
    if actor.hp <= 0.0 || target.hp <= 0.0 {
        return;
    }
    if action.recover {
        actor.hp = (actor.hp + actor.unit.recover).min(1.0);
        return;
    }
    target.hp = if lands_ko(action, target.hp) { 0.0 } else { (target.hp - action.fraction * action.accuracy).max(0.0) };
    if action.fraction > 0.0 {
        actor.hp = (actor.hp - target.unit.passive_on_hit * action.accuracy).max(0.0);
    }
}

fn x_first(x: &Fighter, ax: &Attack, y: &Fighter, ay: &Attack, x_wins_tie: bool) -> bool {
    let (kx, ky) = ((ax.priority, x.unit.speed), (ay.priority, y.unit.speed));
    kx > ky || (kx == ky && x_wins_tie)
}

/// One turn; None means that side is switching or has nothing to do.
fn turn(x: &mut Fighter, y: &mut Fighter, ax: Option<Attack>, ay: Option<Attack>, x_wins_tie: bool) {
    let xf = match (ax, ay) {
        (Some(a), Some(b)) => x_first(x, &a, y, &b, x_wins_tie),
        (Some(_), None) => true,
        _ => false,
    };
    if xf {
        if let Some(a) = ax {
            act(x, y, &a);
        }
        if let Some(a) = ay {
            act(y, x, &a);
        }
    } else {
        if let Some(a) = ay {
            act(y, x, &a);
        }
        if let Some(a) = ax {
            act(x, y, &a);
        }
    }
    for f in [x, y] {
        if f.hp > 0.0 {
            f.hp = (f.hp + f.unit.residual).clamp(0.0, 1.0);
        }
    }
}

/// HP left of a and b after racing until one faints or the horizon passes.
fn duel(a: Fighter, b: Fighter) -> (f32, f32) {
    let (mut x, mut y) = (a, b);
    if x.hp <= 0.0 || y.hp <= 0.0 {
        return (x.hp, y.hp);
    }
    for _ in 0..HORIZON {
        let (ax, ay) = (choose(&x, &y), choose(&y, &x));
        turn(&mut x, &mut y, ax, ay, false);
        if x.hp <= 0.0 || y.hp <= 0.0 {
            break;
        }
    }
    (x.hp, y.hp)
}

/// (status 1 win / -1 loss / 0, our HP sum minus theirs).
type Score = (i8, f32);

fn better(a: Score, b: Score) -> bool {
    a.0 > b.0 || (a.0 == b.0 && a.1 > b.1)
}

#[derive(Clone, Copy)]
struct Hp {
    ours: [f32; MAX_MEMBERS],
    theirs: [f32; MAX_MEMBERS],
}

struct Game<'a> {
    ours: &'a [Combatant],
    theirs: &'a [Combatant],
    m: &'a Matchups,
}

/// Indices with HP left.
fn alive(v: &[f32]) -> ([usize; MAX_MEMBERS], usize) {
    let mut out = [0; MAX_MEMBERS];
    let mut n = 0;
    for (i, &h) in v.iter().enumerate() {
        if h > 0.0 {
            out[n] = i;
            n += 1;
        }
    }
    (out, n)
}

fn active_first(v: &[f32], active: usize) -> ([usize; MAX_MEMBERS], usize) {
    let mut out = [active; MAX_MEMBERS];
    let mut n = 1;
    let (al, k) = alive(v);
    for &i in &al[..k] {
        if i != active {
            out[n] = i;
            n += 1;
        }
    }
    (out, n)
}

/// Who a side can field after a faint: anyone alive if its active fell, else the active (or a free swap).
fn next(units: &[Combatant], v: &[f32], active: usize) -> ([usize; MAX_MEMBERS], usize) {
    if v[active] <= 0.0 {
        return alive(v);
    }
    if units[active].can_switch_on_faint {
        active_first(v, active)
    } else {
        let mut out = [active; MAX_MEMBERS];
        out[0] = active;
        (out, 1)
    }
}

fn terminal(g: &Game, hp: &Hp) -> Score {
    let o = alive(&hp.ours[..g.ours.len()]).1 > 0;
    let t = alive(&hp.theirs[..g.theirs.len()]).1 > 0;
    let status = if o && !t {
        1
    } else if t && !o {
        -1
    } else {
        0
    };
    let s: f32 = hp.ours[..g.ours.len()].iter().sum::<f32>() - hp.theirs[..g.theirs.len()].iter().sum::<f32>();
    (status, s)
}

impl<'a> Game<'a> {
    fn pair(&self, hp: &Hp, i: usize, j: usize) -> (Fighter, Fighter) {
        (
            Fighter { unit: self.ours[i], attacks: self.m.ours[i][j], hp: hp.ours[i] },
            Fighter { unit: self.theirs[j], attacks: self.m.theirs[i][j], hp: hp.theirs[j] },
        )
    }

    /// Our i against their j from `hp`, then the rest.
    fn fight(&self, hp: &Hp, i: usize, j: usize) -> Score {
        let (a, b) = self.pair(hp, i, j);
        let (ah, bh) = duel(a, b);
        let mut after = *hp;
        after.ours[i] = ah;
        after.theirs[j] = bh;
        if ah > 0.0 && bh > 0.0 {
            terminal(self, &after)
        } else {
            self.after(&after, i, j)
        }
    }

    /// After a faint between our i and their j: replacements (and free swaps), then the next duel.
    fn after(&self, hp: &Hp, i: usize, j: usize) -> Score {
        let (no, nt) = (self.ours.len(), self.theirs.len());
        if alive(&hp.ours[..no]).1 == 0 || alive(&hp.theirs[..nt]).1 == 0 {
            return terminal(self, hp);
        }
        let (ks, nk) = next(self.ours, &hp.ours[..no], i);
        let (ms, nm) = next(self.theirs, &hp.theirs[..nt], j);
        if hp.ours[i] > 0.0 {
            // their member fell: they send first, we answer
            let mut low: Option<Score> = None;
            for &m in &ms[..nm] {
                let mut high: Option<Score> = None;
                for &k in &ks[..nk] {
                    let s = self.fight(hp, k, m);
                    if high.map_or(true, |h| better(s, h)) {
                        high = Some(s);
                    }
                }
                let h = high.unwrap();
                if low.map_or(true, |l| better(l, h)) {
                    low = Some(h);
                }
            }
            return low.unwrap();
        }
        let mut high: Option<Score> = None;
        for &k in &ks[..nk] {
            let mut low: Option<Score> = None;
            for &m in &ms[..nm] {
                let s = self.fight(hp, k, m);
                if low.map_or(true, |l| better(l, s)) {
                    low = Some(s);
                }
            }
            let l = low.unwrap();
            if high.map_or(true, |h| better(l, h)) {
                high = Some(l);
            }
        }
        high.unwrap()
    }

    /// First turn with both actives in: each side stays (i, j) or switches (to k, m).
    fn opening(&self, hp: &Hp, i: usize, k: usize, j: usize, m: usize) -> Score {
        if k == i && m == j {
            return self.fight(hp, i, j);
        }
        let (old_x, old_y) = self.pair(hp, i, j);
        let (mut x, mut y) = self.pair(hp, k, m);
        let ax = if k == i { remap(choose(&old_x, &old_y), &x.attacks, y.hp) } else { None };
        let ay = if m == j { remap(choose(&old_y, &old_x), &y.attacks, x.hp) } else { None };
        turn(&mut x, &mut y, ax, ay, false);
        let mut after = *hp;
        after.ours[k] = x.hp;
        after.theirs[m] = y.hp;
        if x.hp > 0.0 && y.hp > 0.0 {
            self.fight(&after, k, m)
        } else {
            self.after(&after, k, m)
        }
    }

    fn first_move(&self, hp: &Hp, i: usize, j: usize) -> Option<Attack> {
        let (x, y) = self.pair(hp, i, j);
        choose(&x, &y)
    }
}

/// The same move against a new target (its best attack if that move is missing).
fn remap(action: Option<Attack>, attacks: &Attacks, target_hp: f32) -> Option<Attack> {
    let a = action?;
    if a.recover {
        return Some(a);
    }
    attacks.as_slice().iter().copied().find(|x| x.id == a.id).or_else(|| best_attack(attacks.as_slice(), target_hp))
}

/// Each first member we could field (stay, switch, or replacement), in the order tried, with its result
/// against their best reply.
pub fn chain_options(ours: &[Combatant], theirs: &[Combatant], m: &Matchups, active_ours: usize, active_theirs: usize) -> ([(usize, ChainResult); MAX_MEMBERS], usize) {
    let g = Game { ours, theirs, m };
    let mut hp = Hp { ours: [0.0; MAX_MEMBERS], theirs: [0.0; MAX_MEMBERS] };
    for (k, c) in ours.iter().enumerate() {
        hp.ours[k] = c.hp;
    }
    for (k, c) in theirs.iter().enumerate() {
        hp.theirs[k] = c.hp;
    }
    let none = ChainResult { wins: false, margin: 0.0, first: None };
    let mut out = [(0, none); MAX_MEMBERS];
    let mut n = 0;
    let (no, nt) = (ours.len(), theirs.len());
    if alive(&hp.ours[..no]).1 == 0 || alive(&hp.theirs[..nt]).1 == 0 {
        return (out, 0);
    }
    let (i, j) = (active_ours, active_theirs);
    let result = |s: Score, first: Option<Attack>| ChainResult { wins: s.0 > 0, margin: s.1, first };
    if hp.ours[i] > 0.0 && hp.theirs[j] > 0.0 {
        let (ks, nk) = active_first(&hp.ours[..no], i);
        let (ms, nm) = active_first(&hp.theirs[..nt], j);
        for &k in &ks[..nk] {
            let mut low: Option<Score> = None;
            for &mm in &ms[..nm] {
                let s = g.opening(&hp, i, k, j, mm);
                if low.map_or(true, |l| better(l, s)) {
                    low = Some(s);
                }
            }
            let first = if k == i { g.first_move(&hp, i, j) } else { None };
            out[n] = (k, result(low.unwrap(), first));
            n += 1;
        }
        return (out, n);
    }
    // replacement: we commit first (pessimistic)
    let (ks, nk) = next(ours, &hp.ours[..no], i);
    let (ms, nm) = next(theirs, &hp.theirs[..nt], j);
    for &k in &ks[..nk] {
        let mut low: Option<(Score, usize)> = None;
        for &mm in &ms[..nm] {
            let s = g.fight(&hp, k, mm);
            if low.map_or(true, |(l, _)| better(l, s)) {
                low = Some((s, mm));
            }
        }
        let (s, mm) = low.unwrap();
        out[n] = (k, result(s, g.first_move(&hp, k, mm)));
        n += 1;
    }
    (out, n)
}
