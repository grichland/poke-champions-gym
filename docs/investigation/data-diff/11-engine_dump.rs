// 11-engine_dump.rs: dumps poke-engine data (after Champions modifiers) as JSON on stdout.
// Built with --features champions,bss. Enumerates enums by integer value until the From impl panics.
use poke_engine::choices::{Choice, Choices, Effect, MoveCategory, MoveTarget, MultiHitMove, StatBoosts, MOVES};
use poke_engine::engine::abilities::Abilities;
use poke_engine::engine::damage_calc::type_effectiveness_modifier;
use poke_engine::engine::items::Items;
use poke_engine::pokemon::PokemonName;
use poke_engine::state::{Pokemon, PokemonNature, PokemonType};
use std::panic;

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}
fn tgt(t: &MoveTarget) -> String {
    q(match t {
        MoveTarget::User => "User",
        MoveTarget::Opponent => "Opponent",
    })
}
fn boosts(b: &StatBoosts) -> String {
    format!(
        "{{\"atk\":{},\"def\":{},\"spa\":{},\"spd\":{},\"spe\":{},\"accuracy\":{}}}",
        b.attack, b.defense, b.special_attack, b.special_defense, b.speed, b.accuracy
    )
}
fn f(x: f32) -> String {
    format!("{}", x)
}
fn opt_f(x: &Option<f32>) -> String {
    match x {
        Some(v) => f(*v),
        None => "null".into(),
    }
}

fn mv(c: &Choice) -> String {
    let fl = &c.flags;
    let flags = format!(
        "{{\"bite\":{},\"bullet\":{},\"charge\":{},\"contact\":{},\"drag\":{},\"heal\":{},\"powder\":{},\"protect\":{},\"pulse\":{},\"punch\":{},\"recharge\":{},\"reflectable\":{},\"slicing\":{},\"sound\":{},\"pivot\":{},\"wind\":{}}}",
        fl.bite, fl.bullet, fl.charge, fl.contact, fl.drag, fl.heal, fl.powder, fl.protect, fl.pulse,
        fl.punch, fl.recharge, fl.reflectable, fl.slicing, fl.sound, fl.pivot, fl.wind
    );
    let boost = match &c.boost {
        Some(b) => format!("{{\"target\":{},\"boosts\":{}}}", tgt(&b.target), boosts(&b.boosts)),
        None => "null".into(),
    };
    let heal = match &c.heal {
        Some(h) => format!("{{\"target\":{},\"amount\":{}}}", tgt(&h.target), f(h.amount)),
        None => "null".into(),
    };
    let status = match &c.status {
        Some(s) => format!("{{\"target\":{},\"status\":{}}}", tgt(&s.target), q(&format!("{:?}", s.status))),
        None => "null".into(),
    };
    let vs = match &c.volatile_status {
        Some(s) => format!("{{\"target\":{},\"volatile_status\":{}}}", tgt(&s.target), q(&format!("{:?}", s.volatile_status))),
        None => "null".into(),
    };
    let sc = match &c.side_condition {
        Some(s) => format!("{{\"target\":{},\"condition\":{}}}", tgt(&s.target), q(&format!("{:?}", s.condition))),
        None => "null".into(),
    };
    let secs = match &c.secondaries {
        Some(v) => {
            let items: Vec<String> = v
                .iter()
                .map(|s| {
                    let eff = match &s.effect {
                        Effect::VolatileStatus(v) => format!("{{\"kind\":\"VolatileStatus\",\"value\":{}}}", q(&format!("{:?}", v))),
                        Effect::Boost(b) => format!("{{\"kind\":\"Boost\",\"value\":{}}}", boosts(b)),
                        Effect::Status(st) => format!("{{\"kind\":\"Status\",\"value\":{}}}", q(&format!("{:?}", st))),
                        Effect::Heal(h) => format!("{{\"kind\":\"Heal\",\"value\":{}}}", f(*h)),
                        Effect::RemoveItem => "{\"kind\":\"RemoveItem\",\"value\":null}".to_string(),
                    };
                    format!("{{\"chance\":{},\"target\":{},\"effect\":{}}}", f(s.chance), tgt(&s.target), eff)
                })
                .collect();
            format!("[{}]", items.join(","))
        }
        None => "null".into(),
    };
    let cat = match c.category {
        MoveCategory::Physical => "Physical",
        MoveCategory::Special => "Special",
        MoveCategory::Status => "Status",
        MoveCategory::Switch => "Switch",
    };
    let mh = match c.multi_hit() {
        MultiHitMove::None => "null",
        MultiHitMove::DoubleHit => "\"DoubleHit\"",
        MultiHitMove::TripleHit => "\"TripleHit\"",
        MultiHitMove::TwoToFiveHits => "\"TwoToFiveHits\"",
        MultiHitMove::PopulationBomb => "\"PopulationBomb\"",
        MultiHitMove::TripleAxel => "\"TripleAxel\"",
    };
    format!(
        "{{\"id\":{},\"move_type\":{},\"category\":{},\"base_power\":{},\"accuracy\":{},\"priority\":{},\"target\":{},\"flags\":{},\"boost\":{},\"drain\":{},\"recoil\":{},\"crash\":{},\"heal\":{},\"status\":{},\"volatile_status\":{},\"side_condition\":{},\"secondaries\":{},\"multi_hit\":{},\"increased_crit_ratio\":{},\"guaranteed_crit\":{}}}",
        q(&format!("{:?}", c.move_id)),
        q(&format!("{:?}", c.move_type)),
        q(cat),
        f(c.base_power),
        f(c.accuracy),
        c.priority,
        tgt(&c.target),
        flags,
        boost,
        opt_f(&c.drain),
        opt_f(&c.recoil),
        opt_f(&c.crash),
        heal,
        status,
        vs,
        sc,
        secs,
        mh,
        c.move_id.increased_crit_ratio(),
        c.move_id.guaranteed_crit()
    )
}

fn enumerate<T, F: Fn(i32) -> Option<T> + panic::RefUnwindSafe>(f: F) -> Vec<T> {
    let mut out = vec![];
    let mut i = 0;
    loop {
        match panic::catch_unwind(|| f(i)) {
            Ok(Some(v)) => out.push(v),
            _ => break,
        }
        i += 1;
    }
    out
}

fn main() {
    panic::set_hook(Box::new(|_| {}));

    // moves
    let mut move_ids: Vec<&Choices> = MOVES.keys().collect();
    move_ids.sort_by_key(|c| format!("{:?}", c));
    let moves: Vec<String> = move_ids
        .iter()
        .map(|k| format!("{}:{}", q(&format!("{:?}", k)), mv(&MOVES[*k])))
        .collect();
    let choices_enum: Vec<String> = enumerate(|i| { let c = Choices::from(i as u16); if c as i32 == i { Some(format!("{:?}", c)) } else { None } });

    let items: Vec<Items> = enumerate(|i| { if i > 255 { return None; } let c = Items::from(i as u8); if c as i32 == i { Some(c) } else { None } });
    let abilities: Vec<String> = enumerate(|i| { let c = Abilities::from(i as i16); if c as i32 == i { Some(format!("{:?}", c)) } else { None } });
    let names: Vec<PokemonName> = enumerate(|i| { let c = PokemonName::from(i as i16); if c as i32 == i { Some(c) } else { None } });

    // per-species base stats where the engine has them
    let mut base_stats = vec![];
    for n in &names {
        let n2 = *n;
        if let Ok(b) = panic::catch_unwind(move || n2.base_stats()) {
            base_stats.push(format!(
                "{}:[{},{},{},{},{},{}]",
                q(&format!("{:?}", n)), b.0, b.1, b.2, b.3, b.4, b.5
            ));
        }
    }

    // mega evolutions: all (species, item) pairs
    let mut megas = vec![];
    for n in &names {
        for it in &items {
            if let Some(m) = n.mega_evolve_target(*it) {
                let b = m.base_stats;
                let table_bs = {
                    let id = m.id;
                    match panic::catch_unwind(move || id.base_stats()) {
                        Ok(t) => format!("[{},{},{},{},{},{}]", t.0, t.1, t.2, t.3, t.4, t.5),
                        Err(_) => "null".into(),
                    }
                };
                megas.push(format!(
                    "{{\"base\":{},\"item\":{},\"mega\":{},\"types\":[{},{}],\"ability\":{},\"base_stats\":[{},{},{},{},{},{}],\"base_stats_table\":{}}}",
                    q(&format!("{:?}", n)), q(&format!("{:?}", it)), q(&format!("{:?}", m.id)),
                    q(&format!("{:?}", m.types.0)), q(&format!("{:?}", m.types.1)),
                    q(&format!("{:?}", m.ability)), b.0, b.1, b.2, b.3, b.4, b.5, table_bs
                ));
            }
        }
    }

    // type chart via the public effectiveness function, defender mono-typed (t, TYPELESS)
    let types: Vec<PokemonType> = enumerate(|i| { if i > 255 { return None; } let c = PokemonType::from(i as u8); if c as i32 == i { Some(c) } else { None } });
    let mut chart = vec![];
    for a in &types {
        let mut row = vec![];
        for d in &types {
            let mut p = Pokemon::default();
            p.types = (*d, PokemonType::TYPELESS);
            row.push(format!("{}:{}", q(&format!("{:?}", d)), f(type_effectiveness_modifier(a, &p))));
        }
        chart.push(format!("{}:{{{}}}", q(&format!("{:?}", a)), row.join(",")));
    }

    // natures: compare computed stats against SERIOUS on a species with known base stats
    let natures: Vec<PokemonNature> = enumerate(|i| { if i > 255 { return None; } let c = PokemonNature::from(i as u8); if c.clone() as i32 == i { Some(c) } else { None } });
    let mut nat = vec![];
    let mut base = Pokemon::default();
    base.id = PokemonName::VENUSAURMEGA;
    base.nature = PokemonNature::SERIOUS;
    let neutral = base.calculate_stats_from_base_stats();
    let neutral = [neutral.1, neutral.2, neutral.3, neutral.4, neutral.5];
    let stat_names = ["atk", "def", "spa", "spd", "spe"];
    for n in &natures {
        let mut p = base.clone();
        p.nature = n.clone();
        let s = p.calculate_stats_from_base_stats();
        let s = [s.1, s.2, s.3, s.4, s.5];
        let mut plus = "null".to_string();
        let mut minus = "null".to_string();
        for i in 0..5 {
            if s[i] > neutral[i] { plus = q(stat_names[i]); }
            if s[i] < neutral[i] { minus = q(stat_names[i]); }
        }
        nat.push(format!("{}:{{\"plus\":{},\"minus\":{}}}", q(&format!("{:?}", n)), plus, minus));
    }

    let js = |v: Vec<String>| v.iter().map(|s| q(s)).collect::<Vec<_>>().join(",");
    println!(
        "{{\"features\":[\"champions\",\"bss\"],\"moves\":{{{}}},\"choices_enum\":[{}],\"items\":[{}],\"abilities\":[{}],\"pokemon_names\":[{}],\"base_stats\":{{{}}},\"megas\":[{}],\"typechart\":{{{}}},\"natures\":{{{}}}}}",
        moves.join(","),
        js(choices_enum),
        js(items.iter().map(|i| format!("{:?}", i)).collect()),
        js(abilities),
        js(names.iter().map(|i| format!("{:?}", i)).collect()),
        base_stats.join(","),
        megas.join(","),
        chart.join(","),
        nat.join(",")
    );
}
