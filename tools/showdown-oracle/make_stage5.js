// Builds tools/rig/data/stage5.json for the driver's --stage5 and --focus5 battles: the legal species that can
// be brought (no Megas, no battle-only forms) with their legal abilities, the legal items (Mega stones by the
// species they are for), the full legal move pool, and the lists --focus5 draws from (the abilities and moves
// that meet items, Mega-capable species).
// usage: node make_stage5.js
'use strict';
const fs = require('fs');
const path = require('path');
const {SIM, FORMAT} = require('./oracle');
const {Dex} = require(path.join(SIM, 'dex'));

const REPO = path.resolve(__dirname, '..', '..');
const dex = Dex.forFormat(FORMAT);
const toID = s => String(s || '').toLowerCase().replace(/[^a-z0-9]/g, '');
const csv = name => fs.readFileSync(path.join(REPO, 'docs', 'investigation', 'coverage', name), 'utf8')
  .trim().split(/\r?\n/).slice(1).map(l => l.split(','));

const species = [];
for (const row of csv('03-coverage-species.csv')) {
  if (row[3] !== '1') continue;
  const sp = dex.species.get(row[1]);
  if (!sp.exists || sp.isMega || sp.battleOnly || sp.requiredItem || sp.requiredItems) continue;
  const abilities = [...new Set(Object.values(sp.abilities).map(toID))];
  species.push({species: sp.name, id: sp.id, gender: sp.gender || '', abilities, stones: []});
}
const byName = new Map(species.map(s => [s.species, s]));

const items = [];
const stones = [];
for (const row of csv('03-coverage-items.csv')) {
  if (row[3] !== '1') continue;
  const it = dex.items.get(row[1]);
  if (!it.exists) continue;
  if (it.megaStone) {
    stones.push(it.name);
    for (const from of Object.keys(it.megaStone)) {
      const s = byName.get(from);
      if (s) s.stones.push(it.name);
    }
  } else {
    items.push(it.name);
  }
}
items.sort();
stones.sort();

const moves = [];
for (const row of csv('03-coverage-moves.csv')) {
  if (row[3] !== '1') continue;
  const m = dex.moves.get(row[1]);
  if (!m.exists || m.id === 'struggle') continue;
  moves.push(m.id);
}
moves.sort();

// moves that need items, and moves that meet items often: contact (Rocky Helmet, Sticky Hold, Pickpocket),
// multi-hit (Focus Sash, resist berries), Normal attacks (Normal Gem, Chilan Berry), Ground attacks (Air
// Balloon, Iron Ball), statuses and confusion (cure berries), Mental Herb's volatiles, terrains (seeds), weather
// (rocks), screens (Light Clay), rooms and Gravity, stat drops (White Herb), drains (Big Root, Shell Bell), pivots
// (Eject Button), phazing (Red Card), priority (Quick Claw), Substitute and recoil
const itemMoves = ['acrobatics', 'belch', 'bugbite', 'corrosivegas', 'covet', 'fling', 'knockoff', 'pluck',
  'poltergeist', 'recycle', 'stuffcheeks', 'teatime', 'thief', 'trick', 'switcheroo'];
const mental = ['attract', 'taunt', 'encore', 'torment', 'disable', 'healblock'];
const interacting = moves.filter(id => {
  const m = dex.moves.get(id);
  const f = m.flags || {};
  const lowers = m.boosts && Object.values(m.boosts).some(v => v < 0);
  const selfDrop = m.self && m.self.boosts && Object.values(m.self.boosts).some(v => v < 0);
  return itemMoves.includes(id) || mental.includes(id) || (f.contact && m.category !== 'Status') ||
    m.multihit || (m.type === 'Normal' && m.category !== 'Status') || (m.type === 'Ground' && m.category !== 'Status') ||
    m.status || m.volatileStatus === 'confusion' || m.terrain || m.weather || m.sideCondition || m.pseudoWeather ||
    lowers || selfDrop || m.drain || m.selfSwitch || m.forceSwitch || m.priority > 0 || m.recoil ||
    ['substitute', 'leechseed', 'ingrain', 'aquaring', 'strengthsap', 'bellydrum', 'protect'].includes(id);
});

const itemAbilities = ['unburden', 'harvest', 'pickup', 'cudchew', 'ripen', 'gluttony', 'unnerve', 'magician',
  'pickpocket', 'stickyhold', 'cheekpouch', 'klutz', 'symbiosis', 'opportunist', 'stalwart', 'sheerforce',
  'serenegrace', 'stench', 'magicguard', 'sturdy', 'contrary', 'simple', 'moldbreaker', 'prankster', 'trace',
  'intimidate', 'defiant', 'competitive', 'mirrorarmor', 'clearbody', 'longreach', 'myceliummight', 'quickdraw',
  'neutralizinggas', 'liquidooze', 'poisonheal', 'skilllink', 'technician', 'drought', 'drizzle', 'sandstream',
  'snowwarning', 'electricsurge', 'grassysurge', 'mistysurge', 'psychicsurge', 'levitate', 'multiscale', 'regenerator',
  'emergencyexit', 'wimpout', 'suctioncups', 'guarddog', 'truant', 'parentalbond', 'toughclaws', 'unseenfist',
  'gulpmissile', 'iceface', 'shieldsdown', 'terashell', 'asoneglastrier', 'asonespectrier'];

// the abilities the resolver implements (engine/src/sim/abilities.rs): the focus list keeps only those
const absrc = fs.readFileSync(path.join(REPO, 'engine', 'src', 'sim', 'abilities.rs'), 'utf8');
const at = absrc.indexOf('pub const IMPLEMENTED_ABILITIES');
const implemented = new Set([...absrc.slice(at, absrc.indexOf('];', at)).matchAll(/"([a-z0-9]+)"/g)].map(m => m[1]));

const megaSpecies = species.filter(s => s.stones.length).map(s => s.species);
const out = {
  showdown: require(path.join(REPO, 'tools', 'rig', 'data', 'stage3_moves.json')).showdown,
  generator: 'tools/showdown-oracle/make_stage5.js',
  rule: 'legal species without Megas and battle-only forms, each with its legal abilities; legal items (a Mega ' +
    'stone only on its species); the full legal move pool',
  species,
  items,
  stones,
  moves,
  item_moves: itemMoves,
  interacting,
  item_abilities: itemAbilities.filter(a => dex.abilities.get(a).exists && implemented.has(a)),
  mega_species: megaSpecies,
};
fs.writeFileSync(path.join(REPO, 'tools', 'rig', 'data', 'stage5.json'), JSON.stringify(out, null, 1) + '\n');
console.log(JSON.stringify({species: species.length, items: items.length, stones: stones.length, moves: moves.length,
  interacting: interacting.length, mega_species: megaSpecies.length}));
