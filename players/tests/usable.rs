//! PS-Q4: status moves the Battle shows cannot work are dropped by tower.

mod common;
use common::*;
use players::*;

fn vs(team: Vec<battle::SetMsg>, opp: &str) -> T {
    let mut pv = vec![opp];
    for p in PREVIEW {
        if p != opp && pv.len() < 6 {
            pv.push(p);
        }
    }
    T::new(fill(team), &pv)
}

fn gengar(moves: &[&str]) -> battle::SetMsg {
    set("gengar", None, "cursedbody", moves)
}

fn usable(t: &T, slot: usize) -> bool {
    t.sorted().usable.contains(&slot)
}

#[test]
fn type_immunities() {
    // Thunder Wave on Ground, Will-O-Wisp on Fire, Toxic on Steel, a powder move on Grass
    let t = vs(vec![gengar(&["thunderwave", "willowisp", "toxic", "shadowball"])], "garchomp");
    assert!(!usable(&t, 0) && usable(&t, 1) && usable(&t, 2));
    let t = vs(vec![gengar(&["thunderwave", "willowisp", "toxic", "shadowball"])], "cinderace");
    assert!(usable(&t, 0) && !usable(&t, 1) && usable(&t, 2));
    let t = vs(vec![gengar(&["thunderwave", "willowisp", "toxic", "shadowball"])], "corviknight");
    assert!(usable(&t, 0) && usable(&t, 1) && !usable(&t, 2));
    let t = vs(vec![set("primarina", None, "torrent", &["sleeppowder", "moonblast", "protect", "rest"])], "rillaboom");
    assert!(!usable(&t, 0));
    // with the option off, Thunder Wave on Ground is kept (as the Python did)
    let o = Options { skip_status_that_cannot_work: false, ..Options::default() };
    let t = vs(vec![gengar(&["thunderwave", "willowisp", "toxic", "shadowball"])], "garchomp");
    assert!(tower::inspect(t.bt(), battle::ReqKind::Turn, &o).unwrap().usable.contains(&0));
}

#[test]
fn substitute_terrain_safeguard_and_revealed_abilities() {
    let mut t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "dragonite");
    assert!(usable(&t, 0) && usable(&t, 1) && usable(&t, 2));
    t.e(&vol("opp", "dragonite", "substitute"));
    assert!(!usable(&t, 0) && !usable(&t, 1));
    // Taunt bypasses Substitute
    assert!(usable(&t, 2));
    let mut t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "garchomp");
    t.e(&field("mistyterrain"));
    assert!(!usable(&t, 0) && !usable(&t, 1));
    let mut t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "garchomp");
    t.e(&side_cond("opp", "safeguard"));
    assert!(!usable(&t, 0) && !usable(&t, 1) && usable(&t, 2));
    let mut t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "dragonite");
    t.e(&ability("opp", "dragonite", "owntempo"));
    assert!(usable(&t, 0) && !usable(&t, 1));
    let mut t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "gholdengo");
    t.e(&ability("opp", "gholdengo", "goodasgold"));
    assert!(!usable(&t, 0) && !usable(&t, 1) && !usable(&t, 2));
    // an unrevealed ability blocks nothing
    let t = vs(vec![gengar(&["willowisp", "confuseray", "taunt", "shadowball"])], "gholdengo");
    assert!(usable(&t, 0) && usable(&t, 2));
}

#[test]
fn repeat_taunt_and_leech_seed() {
    let mut t = vs(vec![set("rillaboom", None, "grassysurge", &["leechseed", "taunt", "woodhammer", "protect"])], "dragonite");
    assert!(usable(&t, 0) && usable(&t, 1));
    t.e(&vol("opp", "dragonite", "leechseed"));
    t.e(&vol("opp", "dragonite", "taunt"));
    assert!(!usable(&t, 0) && !usable(&t, 1));
    let t = vs(vec![set("rillaboom", None, "grassysurge", &["leechseed", "taunt", "woodhammer", "protect"])], "meowscarada");
    assert!(!usable(&t, 0), "Grass types cannot be seeded");
}
