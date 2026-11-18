//! Focused pools for the mechanics the real teams rarely meet (field and side counts, traps, called and
//! copied moves, stage swaps, item and ability exchanges, PP loss, single-turn and lasting volatiles,
//! charges with delayed attacks, the end of the turn, Imprison, trapping abilities never shown): random
//! Players on both seats, every check on. Each pool must give no unexplained disagreement of a seat's
//! Battle with the State (a), legal sets that agree but for the explained classes (b), no anomaly, no
//! opposing choice with prior 0, no wrong truth label, no hidden-information difference, and every Battle
//! must rebuild from its record (d). The runner plays the same pools at scale:
//! `cargo test -p env --test mechanics write_pools -- --ignored` writes them to bench/data/pools/.

use battle::ReqKind;
use env::teams::{TeamMode, TeamPool};
use env::{BatchEnv, EnvConfig, RequestKind};
use players::{by_name, Rng};
use std::sync::Arc;

/// A set as teams.json writes it.
fn set(species: &str, ability: &str, item: &str, moves: &[&str]) -> String {
    let mv: Vec<String> = moves.iter().map(|m| format!("\"{}\"", m)).collect();
    format!(
        "{{\"species\":\"{}\",\"ability\":\"{}\",\"item\":\"{}\",\"moves\":[{}],\"nature\":\"Serious\",\"evs\":{{\"hp\":20,\"atk\":10,\"def\":12,\"spa\":10,\"spd\":12,\"spe\":2}},\"gender\":\"M\"}}",
        species,
        ability,
        item,
        mv.join(",")
    )
}

fn json(teams: &[Vec<String>]) -> String {
    let t: Vec<String> = teams.iter().map(|x| format!("[{}]", x.join(","))).collect();
    format!("[\n{}\n]\n", t.join(",\n"))
}

fn pool(teams: &[Vec<String>]) -> TeamPool {
    for s in teams.iter().flatten() {
        env::teams::mon_spec(&env::teams::Json::parse(s).unwrap()).unwrap_or_else(|e| panic!("{}: {}", s, e));
    }
    TeamPool::from_json(&json(teams)).unwrap_or_else(|e| panic!("pool: {}", e))
}

fn play(p: TeamPool, games: u64, seed: u64) -> env::Counters {
    assert!(p.teams.len() >= 2, "a set did not parse");
    let cfg = EnvConfig { games: 1, seed, team_mode: TeamMode::AsGiven, max_steps: 400, checks: true, keep_records: true, auto_locked: true, replace_before_rows: true };
    let mut env = BatchEnv::new(Arc::new(p), cfg);
    let mut ps = [by_name("random").unwrap(), by_name("random").unwrap()];
    let mut rng = [Rng::new(seed), Rng::new(seed ^ 7)];
    let mut legal = Vec::new();
    while env.games_done < games {
        let pn = env.pending(0);
        let seat = pn.seat as usize;
        let bt = *env.battle(0, seat);
        let act = match pn.kind {
            RequestKind::Preview => env::batch::preview_action(ps[seat].teampreview(&bt, &mut rng[seat])).unwrap(),
            k => ps[seat].choose_move(&bt, if k == RequestKind::Replace { ReqKind::Replace } else { ReqKind::Turn }, &mut rng[seat]) as u16,
        };
        env.legal_actions(0, &mut legal);
        let act = if legal.contains(&act) { act } else { legal[0] };
        env.act(&[act]).unwrap();
    }
    env.counters
}

/// Nothing unexplained in any check.
fn assert_clean(name: &str, c: &env::Counters) {
    assert_eq!(c.errors, 0, "{}: engine errors", name);
    assert_eq!(c.rebuild_equal, c.rebuilt, "{}: rebuilt Battles differ", name);
    eprintln!(
        "{}: {} decisions checked, (a) {}, legal {}, anomalies {}, prior0 {}, truth {}, hidden {} ({} games {} steps), refusals {} (trapped {})",
        name,
        c.checked,
        c.agree.unexplained(),
        c.legal.unexplained(),
        c.anomalies.unexplained(),
        c.prior0.unexplained(),
        c.truth.unexplained(),
        c.hidden.unexplained(),
        c.hidden_games,
        c.hidden_steps,
        c.refusals,
        c.refused_trapped
    );
    assert_eq!(c.agree.unexplained(), 0, "{}: disagreements {:?}", name, c.agree.examples);
    assert_eq!(c.legal.unexplained(), 0, "{}: legal-set classes {:?}", name, c.legal.examples);
    assert_eq!(c.anomalies.unexplained(), 0, "{}: anomalies {:?}", name, c.anomalies.examples);
    assert_eq!(c.prior0.unexplained(), 0, "{}: prior 0 {:?}", name, c.prior0.examples);
    assert_eq!(c.truth.unexplained(), 0, "{}: truth {:?}", name, c.truth.examples);
    assert_eq!(c.hidden.unexplained(), 0, "{}: hidden {:?}", name, c.hidden.examples);
}

fn team(mons: &[(&str, &str, &str, &[&str])]) -> Vec<String> {
    mons.iter().map(|m| set(m.0, m.1, m.2, m.3)).collect()
}

fn field() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Garchomp", "Rough Skin", "Leftovers", &["Tailwind", "Spikes", "Earthquake", "Protect"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Toxic Spikes", "Haze", "Surf", "Recover"]),
            ("Corviknight", "Pressure", "Light Clay", &["Tailwind", "Reflect", "Brave Bird", "Roost"]),
            ("Hippowdon", "Sand Stream", "Smooth Rock", &["Stealth Rock", "Spikes", "Earthquake", "Slack Off"]),
            ("Snorlax", "Thick Fat", "Leftovers", &["Body Slam", "Rest", "Sleep Talk", "Curse"]),
            ("Rotom-Wash", "Levitate", "Sitrus Berry", &["Hydro Pump", "Volt Switch", "Will-O-Wisp", "Light Screen"]),
        ]),
        team(&[
            ("Tyranitar", "Sand Stream", "Leftovers", &["Stealth Rock", "Crunch", "Rock Slide", "Tailwind"]),
            ("Glimmora", "Toxic Debris", "Focus Sash", &["Spikes", "Toxic Spikes", "Power Gem", "Mortal Spin"]),
            ("Pelipper", "Drizzle", "Damp Rock", &["Tailwind", "Hurricane", "Surf", "U-turn"]),
            ("Rillaboom", "Grassy Surge", "Terrain Extender", &["Grassy Glide", "Wood Hammer", "U-turn", "Knock Off"]),
            ("Indeedee", "Psychic Surge", "Light Clay", &["Psychic", "Trick Room", "Reflect", "Light Screen"]),
            ("Ninetales-Alola", "Snow Warning", "Icy Rock", &["Aurora Veil", "Blizzard", "Moonblast", "Encore"]),
        ]),
    ]
}

fn traps() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Umbreon", "Inner Focus", "Leftovers", &["Mean Look", "Foul Play", "Wish", "Protect"]),
            ("Snorlax", "Thick Fat", "Leftovers", &["Block", "Body Slam", "Rest", "Curse"]),
            ("Garchomp", "Rough Skin", "Life Orb", &["Jaw Lock", "Earthquake", "Outrage", "Swords Dance"]),
            ("Kingambit", "Defiant", "Black Glasses", &["Kowtow Cleave", "Iron Head", "Sucker Punch", "Swords Dance"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Infestation", "Toxic", "Recover", "Haze"]),
            ("Gengar", "Cursed Body", "Focus Sash", &["Shadow Ball", "Sludge Bomb", "Perish Song", "Destiny Bond"]),
        ]),
        team(&[
            ("Annihilape", "Defiant", "Leftovers", &["No Retreat", "Rage Fist", "Drain Punch", "Bulk Up"]),
            ("Pawmot", "Volt Absorb", "Sitrus Berry", &["Charge", "Thunder Punch", "Close Combat", "Nuzzle"]),
            ("Rotom-Heat", "Levitate", "Leftovers", &["Charge", "Thunderbolt", "Overheat", "Will-O-Wisp"]),
            ("Corviknight", "Pressure", "Rocky Helmet", &["Brave Bird", "U-turn", "Roost", "Iron Defense"]),
            ("Dragonite", "Multiscale", "Leftovers", &["Fire Spin", "Dragon Dance", "Extreme Speed", "Roost"]),
            ("Gyarados", "Intimidate", "Leftovers", &["Whirlpool", "Waterfall", "Dragon Dance", "Taunt"]),
        ]),
    ]
}

fn calls() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Snorlax", "Thick Fat", "Leftovers", &["Sleep Talk", "Rest", "Body Slam", "Copycat"]),
            ("Ditto", "Imposter", "Choice Scarf", &["Transform"]),
            ("Clefable", "Unaware", "Leftovers", &["Copycat", "Moonblast", "Soft-Boiled", "Instruct"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Earthquake", "Dragon Claw", "Swords Dance", "Protect"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Toxic", "Recover", "Imprison"]),
            ("Gengar", "Cursed Body", "Life Orb", &["Shadow Ball", "Sludge Bomb", "Encore", "Disable"]),
        ]),
        team(&[
            ("Clefable", "Magic Guard", "Leftovers", &["Copycat", "Moonblast", "Moonlight", "Transform"]),
            ("Slowbro", "Regenerator", "Leftovers", &["Sleep Talk", "Rest", "Scald", "Slack Off"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Psychic", "Draining Kiss", "Calm Mind", "Mystical Fire"]),
            ("Kingambit", "Defiant", "Leftovers", &["Kowtow Cleave", "Iron Head", "Sucker Punch", "Swords Dance"]),
            ("Rotom-Wash", "Levitate", "Leftovers", &["Hydro Pump", "Volt Switch", "Will-O-Wisp", "Thunder Wave"]),
            ("Ditto", "Imposter", "Leftovers", &["Transform"]),
        ]),
    ]
}

fn stages() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Toxapex", "Regenerator", "Leftovers", &["Haze", "Toxic", "Recover", "Clear Smog"]),
            ("Malamar", "Contrary", "Leftovers", &["Topsy-Turvy", "Superpower", "Psycho Cut", "Knock Off"]),
            ("Espeon", "Magic Bounce", "Leftovers", &["Power Swap", "Guard Swap", "Psych Up", "Psychic"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Swords Dance", "Earthquake", "Dragon Claw", "Protect"]),
            ("Volcarona", "Flame Body", "Leftovers", &["Quiver Dance", "Fire Blast", "Bug Buzz", "Roost"]),
            ("Snorlax", "Thick Fat", "White Herb", &["Curse", "Body Slam", "Rest", "Belly Drum"]),
        ]),
        team(&[
            ("Polteageist", "Cursed Body", "White Herb", &["Shell Smash", "Shadow Ball", "Giga Drain", "Strength Sap"]),
            ("Dragonite", "Multiscale", "Leftovers", &["Dragon Dance", "Extreme Speed", "Earthquake", "Roost"]),
            ("Gyarados", "Intimidate", "Leftovers", &["Dragon Dance", "Waterfall", "Haze", "Taunt"]),
            ("Clefable", "Unaware", "Leftovers", &["Calm Mind", "Moonblast", "Soft-Boiled", "Psych Up"]),
            ("Lucario", "Justified", "Life Orb", &["Swords Dance", "Close Combat", "Extreme Speed", "Nasty Plot"]),
            ("Kingambit", "Defiant", "Leftovers", &["Swords Dance", "Kowtow Cleave", "Iron Head", "Sucker Punch"]),
        ]),
    ]
}

fn exchanges() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Rotom-Wash", "Levitate", "Choice Scarf", &["Trick", "Hydro Pump", "Volt Switch", "Will-O-Wisp"]),
            ("Alakazam", "Magic Guard", "Choice Scarf", &["Switcheroo", "Psychic", "Focus Blast", "Shadow Ball"]),
            ("Mr. Rime", "Screen Cleaner", "Leftovers", &["Skill Swap", "Freeze-Dry", "Psychic", "Rapid Spin"]),
            ("Trevenant", "Natural Cure", "Leftovers", &["Forest's Curse", "Trick-or-Treat", "Wood Hammer", "Poltergeist"]),
            ("Pelipper", "Drizzle", "Leftovers", &["Soak", "Surf", "Hurricane", "Roost"]),
            ("Weavile", "Pressure", "Life Orb", &["Knock Off", "Ice Shard", "Triple Axel", "Swords Dance"]),
        ]),
        team(&[
            ("Gardevoir", "Trace", "Leftovers", &["Moonblast", "Psychic", "Reflect Type", "Calm Mind"]),
            ("Garchomp", "Rough Skin", "Rocky Helmet", &["Earthquake", "Dragon Claw", "Swords Dance", "Protect"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Toxic", "Recover", "Haze"]),
            ("Slowking-Galar", "Regenerator", "Leftovers", &["Sludge Bomb", "Psychic", "Flamethrower", "Ice Beam"]),
            ("Corviknight", "Pressure", "Leftovers", &["Brave Bird", "U-turn", "Roost", "Iron Defense"]),
            ("Starmie", "Natural Cure", "Life Orb", &["Hydro Pump", "Psychic", "Ice Beam", "Rapid Spin"]),
        ]),
    ]
}

fn pp() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Gengar", "Cursed Body", "Leftovers", &["Spite", "Shadow Ball", "Destiny Bond", "Perish Song"]),
            ("Indeedee", "Psychic Surge", "Leftovers", &["Eerie Spell", "Psychic", "Calm Mind", "Healing Wish"]),
            ("Kingambit", "Defiant", "Leftovers", &["Throat Chop", "Iron Head", "Sucker Punch", "Torment"]),
            ("Slowbro", "Regenerator", "Leftovers", &["Yawn", "Scald", "Slack Off", "Future Sight"]),
            ("Clefable", "Unaware", "Leftovers", &["Encore", "Moonblast", "Wish", "Protect"]),
            ("Umbreon", "Inner Focus", "Leftovers", &["Taunt", "Foul Play", "Wish", "Protect"]),
        ]),
        team(&[
            ("Primarina", "Torrent", "Leftovers", &["Sparkling Aria", "Hyper Voice", "Moonblast", "Encore"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Earthquake", "Dragon Claw", "Swords Dance", "Substitute"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Psychic", "Draining Kiss", "Calm Mind", "Mystical Fire"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Toxic", "Recover", "Baneful Bunker"]),
            ("Dragonite", "Multiscale", "Leftovers", &["Dragon Dance", "Extreme Speed", "Outrage", "Roost"]),
            ("Sylveon", "Pixilate", "Leftovers", &["Hyper Voice", "Wish", "Protect", "Calm Mind"]),
        ]),
    ]
}

/// U1: abilities changed by Trace, Skill Swap, Worry Seed, Entrainment, Role Play, Simple Beam, Gastro
/// Acid and Mega Evolution, kept or lost across fainting and Revival Blessing.
fn abilities() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Gardevoir", "Trace", "Leftovers", &["Skill Swap", "Moonblast", "Psychic", "Calm Mind"]),
            ("Mr. Rime", "Screen Cleaner", "Leftovers", &["Skill Swap", "Freeze-Dry", "Psychic", "Rapid Spin"]),
            ("Pawmot", "Volt Absorb", "Leftovers", &["Revival Blessing", "Thunder Punch", "Close Combat", "Nuzzle"]),
            ("Garchomp", "Rough Skin", "Garchompite", &["Earthquake", "Dragon Claw", "Swords Dance", "Protect"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Psychic", "Draining Kiss", "Worry Seed", "Calm Mind"]),
            ("Clefable", "Unaware", "Leftovers", &["Role Play", "Moonblast", "Moonlight", "Entrainment"]),
        ]),
        team(&[
            ("Gardevoir", "Trace", "Gardevoirite", &["Moonblast", "Psychic", "Skill Swap", "Calm Mind"]),
            ("Excadrill", "Mold Breaker", "Excadrite", &["Iron Head", "Earthquake", "Rapid Spin", "Swords Dance"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Toxic", "Recover", "Gastro Acid"]),
            ("Pawmot", "Natural Cure", "Leftovers", &["Revival Blessing", "Thunder Punch", "Close Combat", "Nuzzle"]),
            ("Sylveon", "Pixilate", "Leftovers", &["Hyper Voice", "Simple Beam", "Protect", "Calm Mind"]),
            ("Corviknight", "Pressure", "Leftovers", &["Brave Bird", "U-turn", "Roost", "Iron Defense"]),
        ]),
    ]
}

/// U2, U7: charges and recharges with Future Sight landing, confusion self-hits and Substitutes broken.
fn charges() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Archaludon", "Stamina", "Leftovers", &["Electro Shot", "Flash Cannon", "Draco Meteor", "Protect"]),
            ("Charizard", "Blaze", "Charizardite Y", &["Solar Beam", "Fire Blast", "Air Slash", "Roost"]),
            ("Ceruledge", "Flash Fire", "Leftovers", &["Phantom Force", "Bitter Blade", "Swords Dance", "Shadow Sneak"]),
            ("Slowbro", "Regenerator", "Leftovers", &["Future Sight", "Scald", "Slack Off", "Confuse Ray"]),
            ("Pelipper", "Drizzle", "Damp Rock", &["Hurricane", "Surf", "U-turn", "Roost"]),
            ("Hydreigon", "Levitate", "Life Orb", &["Meteor Beam", "Draco Meteor", "Dark Pulse", "Substitute"]),
        ]),
        team(&[
            ("Gengar", "Cursed Body", "Leftovers", &["Future Sight", "Shadow Ball", "Confuse Ray", "Substitute"]),
            ("Archaludon", "Sturdy", "White Herb", &["Electro Shot", "Flash Cannon", "Body Press", "Iron Defense"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Dig", "Earthquake", "Outrage", "Substitute"]),
            ("Torkoal", "Drought", "Heat Rock", &["Solar Beam", "Lava Plume", "Yawn", "Rapid Spin"]),
            ("Dragonite", "Multiscale", "Leftovers", &["Hyper Beam", "Fly", "Dragon Dance", "Roost"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Future Sight", "Psychic", "Swagger", "Calm Mind"]),
        ]),
    ]
}

/// U3: what lasts to the end of the turn (Endure, Yawn, partial traps) at decisions after it: faint
/// replacements, Emergency Exit on residual damage, pivots.
fn turn_end() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Golisopod", "Emergency Exit", "Leftovers", &["First Impression", "Endure", "Liquidation", "Spikes"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Endure", "Earthquake", "Sandstorm", "Protect"]),
            ("Slowbro", "Regenerator", "Leftovers", &["Yawn", "Scald", "Slack Off", "Rest"]),
            ("Dragonite", "Multiscale", "Leftovers", &["Fire Spin", "Extreme Speed", "Endure", "Roost"]),
            ("Rillaboom", "Grassy Surge", "Leftovers", &["Grassy Glide", "U-turn", "Wood Hammer", "Endure"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Toxic", "Infestation", "Recover", "Toxic Spikes"]),
        ]),
        team(&[
            ("Golisopod", "Emergency Exit", "Sitrus Berry", &["First Impression", "Leech Life", "Endure", "Aqua Jet"]),
            ("Tyranitar", "Sand Stream", "Leftovers", &["Stealth Rock", "Crunch", "Endure", "Rock Slide"]),
            ("Hippowdon", "Sand Stream", "Leftovers", &["Yawn", "Earthquake", "Slack Off", "Whirlwind"]),
            ("Umbreon", "Inner Focus", "Leftovers", &["Yawn", "Foul Play", "Wish", "Protect"]),
            ("Corviknight", "Pressure", "Leftovers", &["Brave Bird", "U-turn", "Roost", "Endure"]),
            ("Gengar", "Cursed Body", "Focus Sash", &["Shadow Ball", "Destiny Bond", "Will-O-Wisp", "Endure"]),
        ]),
    ]
}

/// U4: every item exchange and removal (Trick, Switcheroo, Thief, Covet, Knock Off, Bug Bite, Pluck,
/// Recycle, Fling, Poltergeist, Corrosive Gas, Magician, Pickpocket, Symbiosis), with Berries and Choice items.
fn items() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Rotom-Wash", "Levitate", "Choice Scarf", &["Trick", "Hydro Pump", "Volt Switch", "Thief"]),
            ("Alakazam", "Magic Guard", "Choice Scarf", &["Switcheroo", "Psychic", "Focus Blast", "Fling"]),
            ("Weavile", "Pickpocket", "Sitrus Berry", &["Knock Off", "Ice Shard", "Triple Axel", "Covet"]),
            ("Scizor", "Technician", "Leftovers", &["Bug Bite", "Bullet Punch", "Pluck", "Recycle"]),
            ("Delphox", "Magician", "Life Orb", &["Mystical Fire", "Psychic", "Trick", "Fling"]),
            ("Trevenant", "Natural Cure", "Lum Berry", &["Poltergeist", "Wood Hammer", "Recycle", "Will-O-Wisp"]),
        ]),
        team(&[
            ("Sableye", "Prankster", "Leftovers", &["Knock Off", "Trick", "Recover", "Thief"]),
            ("Garchomp", "Rough Skin", "Rocky Helmet", &["Earthquake", "Dragon Claw", "Covet", "Protect"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Toxic", "Recover", "Corrosive Gas"]),
            ("Snorlax", "Thick Fat", "Sitrus Berry", &["Body Slam", "Recycle", "Belly Drum", "Fling"]),
            ("Meowscarada", "Protean", "Choice Scarf", &["Flower Trick", "Knock Off", "U-turn", "Trick"]),
            ("Hatterene", "Symbiosis", "Oran Berry", &["Psychic", "Draining Kiss", "Calm Mind", "Mystical Fire"]),
        ]),
    ]
}

/// U5: Imprison with Transform, Imposter and Copycat (moves of the imprisoner the Battle has not seen).
fn imprison() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Toxapex", "Regenerator", "Leftovers", &["Imprison", "Scald", "Recover", "Toxic"]),
            ("Ditto", "Imposter", "Choice Scarf", &["Transform"]),
            ("Clefable", "Magic Guard", "Leftovers", &["Imprison", "Copycat", "Moonblast", "Transform"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Imprison", "Psychic", "Calm Mind", "Draining Kiss"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Earthquake", "Dragon Claw", "Protect", "Swords Dance"]),
            ("Snorlax", "Thick Fat", "Leftovers", &["Copycat", "Body Slam", "Rest", "Protect"]),
        ]),
        team(&[
            ("Ditto", "Imposter", "Leftovers", &["Transform"]),
            ("Clefable", "Unaware", "Leftovers", &["Copycat", "Moonblast", "Protect", "Imprison"]),
            ("Toxapex", "Regenerator", "Leftovers", &["Scald", "Recover", "Toxic", "Protect"]),
            ("Hatterene", "Magic Bounce", "Leftovers", &["Psychic", "Calm Mind", "Draining Kiss", "Protect"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Earthquake", "Dragon Claw", "Protect", "Imprison"]),
            ("Gengar", "Cursed Body", "Leftovers", &["Shadow Ball", "Sludge Bomb", "Protect", "Copycat"]),
        ]),
    ]
}

/// N1: trapping abilities never shown (Shadow Tag, Arena Trap, Magnet Pull write no row): the Battle
/// allows a switch, the game refuses it, the Battle learns the trap.
fn hidden_trap() -> Vec<Vec<String>> {
    vec![
        team(&[
            ("Gengar", "Shadow Tag", "Leftovers", &["Shadow Ball", "Sludge Bomb", "Protect", "Will-O-Wisp"]),
            ("Excadrill", "Arena Trap", "Leftovers", &["Earthquake", "Iron Head", "Rapid Spin", "Swords Dance"]),
            ("Rotom-Wash", "Magnet Pull", "Leftovers", &["Hydro Pump", "Thunderbolt", "Will-O-Wisp", "Protect"]),
            ("Gardevoir", "Shadow Tag", "Leftovers", &["Moonblast", "Psychic", "Calm Mind", "Protect"]),
            ("Hippowdon", "Arena Trap", "Leftovers", &["Earthquake", "Slack Off", "Stealth Rock", "Yawn"]),
            ("Bellibolt", "Magnet Pull", "Leftovers", &["Thunderbolt", "Slack Off", "Muddy Water", "Toxic"]),
        ]),
        team(&[
            ("Corviknight", "Pressure", "Leftovers", &["Brave Bird", "U-turn", "Roost", "Iron Defense"]),
            ("Kingambit", "Defiant", "Leftovers", &["Kowtow Cleave", "Iron Head", "Sucker Punch", "Swords Dance"]),
            ("Archaludon", "Stamina", "Leftovers", &["Flash Cannon", "Draco Meteor", "Body Press", "Protect"]),
            ("Garchomp", "Rough Skin", "Leftovers", &["Earthquake", "Dragon Claw", "Protect", "Swords Dance"]),
            ("Snorlax", "Shadow Tag", "Leftovers", &["Body Slam", "Curse", "Rest", "Protect"]),
            ("Scizor", "Technician", "Leftovers", &["Bullet Punch", "U-turn", "Swords Dance", "Roost"]),
        ]),
    ]
}

/// Every focused pool by name.
fn pools() -> Vec<(&'static str, Vec<Vec<String>>)> {
    vec![
        ("field", field()),
        ("traps", traps()),
        ("calls", calls()),
        ("stages", stages()),
        ("exchanges", exchanges()),
        ("pp", pp()),
        ("abilities", abilities()),
        ("charges", charges()),
        ("turn_end", turn_end()),
        ("items", items()),
        ("imprison", imprison()),
        ("hidden_trap", hidden_trap()),
    ]
}

#[test]
#[ignore]
fn write_pools() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../bench/data/pools");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, t) in pools() {
        pool(&t);
        std::fs::write(dir.join(format!("{}.json", name)), json(&t)).unwrap();
    }
}

fn check(name: &str, seed: u64) -> env::Counters {
    let t = pools().into_iter().find(|p| p.0 == name).unwrap().1;
    let c = play(pool(&t), 150, seed);
    assert_clean(name, &c);
    c
}

#[test]
fn field_and_side_counts() {
    check("field", 1);
}

#[test]
fn traps_and_self_locks() {
    check("traps", 2);
}

#[test]
fn called_and_copied_moves() {
    check("calls", 3);
}

#[test]
fn stages_swapped_and_reset() {
    check("stages", 4);
}

#[test]
fn items_abilities_and_types_exchanged() {
    check("exchanges", 5);
}

#[test]
fn pp_and_volatiles() {
    check("pp", 6);
}

#[test]
fn abilities_changed_kept_and_restored() {
    check("abilities", 7);
}

#[test]
fn charges_with_delayed_attacks_and_self_hits() {
    check("charges", 8);
}

#[test]
fn turn_end_before_the_turn_row() {
    check("turn_end", 9);
}

#[test]
fn item_moves_and_abilities() {
    check("items", 10);
}

#[test]
fn imprison_with_copied_moves() {
    check("imprison", 11);
}

#[test]
fn trapping_abilities_never_shown_are_refused() {
    let c = check("hidden_trap", 12);
    assert!(c.refused_trapped > 20, "hidden traps refused {}", c.refused_trapped);
}
