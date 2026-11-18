//! What the estimate and the Players read of one Pokémon and of the field, filled from a Battle by fixed
//! rules (D7): own Pokémon from the set, the opponent's from what was revealed, with neutral nature, no stat
//! points and no item for everything unrevealed.

use battle::{items, Active, Battle, ItemState, Side, VolDetail, Who, N_BOOSTS, R_GRAVITY, R_MAGICROOM};
use cg_engine::sim::calc::{apply_stage, chain, modify, stat_from_points};
use cg_engine::sim::data::*;

/// Volatile conditions the estimate reads.
pub mod vf {
    pub const MAGNETRISE: u16 = 1 << 0;
    pub const SMACKDOWN: u16 = 1 << 1;
    pub const INGRAIN: u16 = 1 << 2;
    pub const FLASHFIRE: u16 = 1 << 3;
    pub const CHARGE: u16 = 1 << 4;
    pub const SUBSTITUTE: u16 = 1 << 5;
    pub const CONFUSION: u16 = 1 << 6;
    pub const YAWN: u16 = 1 << 7;
    pub const TAUNT: u16 = 1 << 8;
}

/// One Pokémon as an attacker or a defender.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub side: Side,
    /// None for a species known only from preview
    pub who: Option<Who>,
    pub form: SpeciesId,
    pub types: [Type; 2],
    /// hp, atk, def, spa, spd, spe: own exact, opponent by the fixed rule
    pub stats: [u16; 6],
    pub boosts: [i8; N_BOOSTS],
    /// own, or revealed; None when unknown
    pub ability: Option<AbilityId>,
    /// held and known, and not suppressed (Magic Room, Klutz)
    pub item: Option<ItemId>,
    /// it is known to hold no item (own with none, or the opponent's was used up or removed)
    pub no_item: bool,
    pub status: Status,
    /// percent of max HP
    pub hp: f32,
    pub vols: u16,
    pub active: bool,
}

/// Stats of a species with neutral nature and no stat points (the fixed rule for unknown sets).
pub fn neutral_stats(sp: SpeciesId) -> [u16; 6] {
    let base = sp.data().base_stats;
    let neutral = nature_from_id("serious").unwrap_or(0);
    let mut out = [0u16; 6];
    for (i, st) in [Stat::Hp, Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe].into_iter().enumerate() {
        out[i] = stat_from_points(base[i], 0, st, neutral);
    }
    out
}

/// Own stats in a form other than the set's species (Mega): exact when nature and points were read,
/// otherwise the shown stat plus the change in base stat (exact for a neutral nature).
pub fn own_form_stats(b: &Battle, slot: usize, form: SpeciesId) -> [u16; 6] {
    let set = &b.sets[slot];
    if form == set.species || set.stats[0] == 0 {
        return set.stats;
    }
    let base = set.species.data().base_stats;
    let fb = form.data().base_stats;
    let mut out = set.stats;
    for i in 1..6 {
        let st = [Stat::Hp, Stat::Atk, Stat::Def, Stat::Spa, Stat::Spd, Stat::Spe][i];
        out[i] = match (set.nature, set.points) {
            (Some(n), Some(p)) => stat_from_points(fb[i], p[i], st, n),
            _ => (set.stats[i] as i32 + fb[i] as i32 - base[i] as i32).max(1) as u16,
        };
    }
    out
}

fn vol_flags(a: &Active) -> u16 {
    let mut f = 0;
    for v in a.vols.iter().flatten() {
        f |= match v.cond {
            c if c == conds::MAGNETRISE => vf::MAGNETRISE,
            c if c == conds::SMACKDOWN => vf::SMACKDOWN,
            c if c == conds::INGRAIN => vf::INGRAIN,
            c if c == conds::FLASHFIRE => vf::FLASHFIRE,
            c if c == conds::CHARGE => vf::CHARGE,
            c if c == conds::SUBSTITUTE => vf::SUBSTITUTE,
            c if c == conds::CONFUSION => vf::CONFUSION,
            c if c == conds::YAWN => vf::YAWN,
            c if c == conds::TAUNT => vf::TAUNT,
            _ => 0,
        };
    }
    f
}

/// The volatile condition `c` on the active Pokémon of a side.
pub fn has_vol(b: &Battle, side: Side, c: Cond) -> Option<VolDetail> {
    b.side(side).active.vols.iter().flatten().find(|v| v.cond == c).map(|v| v.detail)
}

impl Fighter {
    /// A Pokémon of either side as the Battle shows it now. `mega`: our active one as it would be after
    /// Mega Evolving this turn.
    pub fn of(b: &Battle, w: Who, mega: bool) -> Fighter {
        match w.side {
            Side::Own => Fighter::own(b, w.slot as usize, mega),
            Side::Opp => Fighter::opp(b, w.slot as usize),
        }
    }

    pub fn own(b: &Battle, slot: usize, mega: bool) -> Fighter {
        let w = Who { side: Side::Own, slot: slot as u8 };
        let m = b.mon(w);
        let set = &b.sets[slot];
        let side = b.side(Side::Own);
        let active = b.active(Side::Own) == Some(w);
        let mut form = m.form.unwrap_or(set.species);
        let mut mega_form = None;
        if mega {
            if let ItemState::Held(it) = m.item {
                mega_form = items::mega_form(it, form);
            }
        }
        if let Some(f) = mega_form {
            form = f;
        }
        let mut stats = own_form_stats(b, slot, form);
        let mut types = form.data().types;
        if active && mega_form.is_none() {
            if let Some(t) = b.types(w) {
                types = t;
            }
            if let Some(sp) = side.active.transformed {
                // Transform copies everything but HP; the target's stats are ours only if it was ours
                let hp = stats[0];
                stats = neutral_stats(sp);
                stats[0] = hp;
                form = sp;
            }
        }
        let mut ability = m.ability;
        if let Some(f) = mega_form {
            let ab = f.data().abilities;
            if ab[0].0 != 0 {
                ability = Some(ab[0]);
            }
        }
        let (item, no_item) = item_of(b, m.item, ability);
        Fighter {
            side: Side::Own,
            who: Some(w),
            form,
            types,
            stats,
            boosts: if active { side.active.boosts } else { [0; N_BOOSTS] },
            ability,
            item,
            no_item,
            status: m.status,
            hp: m.hp,
            vols: if active { vol_flags(&side.active) } else { 0 },
            active,
        }
    }

    pub fn opp(b: &Battle, slot: usize) -> Fighter {
        let w = Who { side: Side::Opp, slot: slot as u8 };
        let m = b.mon(w);
        let side = b.side(Side::Opp);
        let active = b.active(Side::Opp) == Some(w);
        let mut form = m.form.or(m.species).or(m.preview).unwrap_or_default();
        if active {
            if let Some(sp) = side.active.transformed {
                form = sp;
            }
        }
        let stats = neutral_stats(form);
        let types = if active { b.types(w).unwrap_or(form.data().types) } else { form.data().types };
        let (item, no_item) = item_of(b, m.item, m.ability);
        Fighter {
            side: Side::Opp,
            who: Some(w),
            form,
            types,
            stats,
            boosts: if active { side.active.boosts } else { [0; N_BOOSTS] },
            ability: m.ability,
            item,
            no_item,
            status: m.status,
            hp: m.hp,
            vols: if active { vol_flags(&side.active) } else { 0 },
            active,
        }
    }

    /// A species known only by name (opponent's preview): neutral stats, nothing revealed, full HP.
    pub fn species(sp: SpeciesId, side: Side) -> Fighter {
        Fighter {
            side,
            who: None,
            form: sp,
            types: sp.data().types,
            stats: neutral_stats(sp),
            boosts: [0; N_BOOSTS],
            ability: None,
            item: None,
            no_item: false,
            status: Status::None,
            hp: 100.0,
            vols: 0,
            active: false,
        }
    }

    pub fn max_hp(&self) -> u32 {
        self.stats[0].max(1) as u32
    }
    pub fn cur_hp(&self) -> u32 {
        ((self.hp.clamp(0.0, 100.0) as f64 * self.max_hp() as f64 / 100.0).round() as u32).max(if self.hp > 0.0 { 1 } else { 0 })
    }
    pub fn has_type(&self, t: Type) -> bool {
        self.types.contains(&t)
    }
    pub fn ab(&self) -> &'static str {
        self.ability.map_or("", |a| a.id())
    }
    pub fn it(&self) -> &'static str {
        self.item.map_or("", |i| i.id())
    }
    pub fn weight_hg(&self) -> u32 {
        let w = self.form.data().weight_hg.max(1);
        match self.ab() {
            "heavymetal" => w * 2,
            "lightmetal" => (w / 2).max(1),
            _ => w,
        }
    }
}

fn item_of(b: &Battle, st: ItemState, ability: Option<AbilityId>) -> (Option<ItemId>, bool) {
    match st {
        ItemState::Held(i) => {
            let suppressed = b.field.rooms[R_MAGICROOM].is_some() || ability.map(|a| a.id()) == Some("klutz");
            (if suppressed { None } else { Some(i) }, false)
        }
        ItemState::Gone(_) => (None, true),
        ItemState::Unknown => (None, false),
    }
}

/// Weather, terrain, rooms and side conditions as the estimate reads them.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct FieldView {
    /// none while a revealed Cloud Nine or Air Lock is on the field
    pub weather: Option<Cond>,
    pub terrain: Option<Cond>,
    pub gravity: bool,
    pub trick_room: bool,
    pub wonder_room: bool,
    /// per side (0 own, 1 opponent): Reflect, Light Screen, Aurora Veil, Tailwind, Safeguard
    pub screens: [[bool; 5]; 2],
}

impl FieldView {
    pub fn of(b: &Battle) -> FieldView {
        let mut f = FieldView::default();
        let suppressed = [Side::Own, Side::Opp].iter().any(|&s| {
            b.active(s).and_then(|w| b.mon(w).ability).map_or(false, |a| a.data().suppress_weather)
        });
        if !suppressed {
            f.weather = b.field.weather.map(|t| t.cond);
        }
        f.terrain = b.field.terrain.map(|t| t.cond);
        f.trick_room = b.field.rooms[0].is_some();
        f.wonder_room = b.field.rooms[2].is_some();
        f.gravity = b.field.rooms[R_GRAVITY].is_some();
        for s in 0..2 {
            for i in 0..5 {
                f.screens[s][i] = b.sides[s].conds[i].is_some();
            }
        }
        f
    }
    pub fn sun(&self) -> bool {
        matches!(self.weather.map(|c| c.id()), Some("sunnyday") | Some("desolateland"))
    }
    pub fn rain(&self) -> bool {
        matches!(self.weather.map(|c| c.id()), Some("raindance") | Some("primordialsea"))
    }
    pub fn sand(&self) -> bool {
        self.weather == Some(conds::SANDSTORM)
    }
    pub fn snow(&self) -> bool {
        matches!(self.weather.map(|c| c.id()), Some("snowscape") | Some("hail"))
    }
    pub fn terrain_is(&self, id: &str) -> bool {
        self.terrain.map(|c| c.id()) == Some(id)
    }
}

/// Showdown's isGrounded from what is known.
pub fn grounded(f: &Fighter, field: &FieldView) -> bool {
    if field.gravity || f.it() == "ironball" || f.vols & (vf::SMACKDOWN | vf::INGRAIN) != 0 {
        return true;
    }
    !(f.has_type(Type::Flying) || f.ab() == "levitate" || f.it() == "airballoon" || f.vols & vf::MAGNETRISE != 0)
}

/// Effective speed (getActionSpeed without the random tie): stage, then the known modifiers.
pub fn speed(f: &Fighter, field: &FieldView) -> u32 {
    let mut s = apply_stage(f.stats[5] as u32, f.boosts[Stat::Spe.boost_index()]);
    let mut m = 4096;
    let doubled = match f.ab() {
        "swiftswim" => field.rain(),
        "chlorophyll" => field.sun(),
        "sandrush" => field.sand(),
        "slushrush" => field.snow(),
        "surgesurfer" => field.terrain_is("electricterrain"),
        _ => false,
    };
    if doubled {
        m = chain(m, 8192);
    }
    if f.ab() == "quickfeet" && f.status != Status::None {
        m = chain(m, 6144);
    }
    match f.it() {
        "choicescarf" => m = chain(m, 6144),
        "ironball" => m = chain(m, 2048),
        _ => {}
    }
    let side = if f.side == Side::Own { 0 } else { 1 };
    if field.screens[side][3] {
        m = chain(m, 8192);
    }
    s = modify(s, m);
    if f.status == Status::Par && f.ab() != "quickfeet" {
        s = s / 2;
    }
    s.min(10000)
}

/// T-13: strictly faster after Trick Room; equal speed is faster for neither.
pub fn outspeeds(a: &Fighter, d: &Fighter, field: &FieldView) -> bool {
    let (x, y) = (speed(a, field), speed(d, field));
    if field.trick_room {
        x < y
    } else {
        x > y
    }
}
