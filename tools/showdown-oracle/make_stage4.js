// Builds tools/rig/data/stage4.json for the driver's --stage4 and --focus4 battles: the legal species that can
// be brought (no Megas, no battle-only forms) with their legal abilities, the abilities the resolver implements
// (read from engine/src/sim/abilities.rs), and the stage-4 move pool (the stage-3 pool plus the moves that
// change or suppress abilities, and Imprison).
// usage: node make_stage4.js
'use strict';
const fs = require('fs');
const path = require('path');
const {SIM, FORMAT} = require('./oracle');
const {Dex} = require(path.join(SIM, 'dex'));

const REPO = path.resolve(__dirname, '..', '..');
const dex = Dex.forFormat(FORMAT);
const toID = s => String(s || '').toLowerCase().replace(/[^a-z0-9]/g, '');

const src = fs.readFileSync(path.join(REPO, 'engine', 'src', 'sim', 'abilities.rs'), 'utf8');
const at = src.indexOf('pub const IMPLEMENTED_ABILITIES');
const block = src.slice(at, src.indexOf('];', at));
const implemented = [...block.matchAll(/"([a-z0-9]+)"/g)].map(m => m[1]).filter(a => a !== 'noability');

const csv = fs.readFileSync(path.join(REPO, 'docs', 'investigation', 'coverage', '03-coverage-species.csv'), 'utf8')
  .trim().split(/\r?\n/).slice(1).map(l => l.split(','));
const species = [];
const legalAbilities = new Set();
for (const row of csv) {
  const id = row[1];
  if (row[3] !== '1') continue;
  const sp = dex.species.get(id);
  if (!sp.exists || sp.isMega || sp.battleOnly || sp.requiredItem || sp.requiredItems) continue;
  const abilities = [...new Set(Object.values(sp.abilities).map(toID))];
  for (const a of abilities) legalAbilities.add(a);
  species.push({species: sp.name, id: sp.id, gender: sp.gender || '', abilities});
}
// every ability any legal form has, Megas included
for (const row of csv) {
  if (row[3] !== '1') continue;
  const sp = dex.species.get(row[1]);
  if (sp.exists) for (const a of Object.values(sp.abilities)) legalAbilities.add(toID(a));
}

const s3 = JSON.parse(fs.readFileSync(path.join(REPO, 'tools', 'rig', 'data', 'stage3_moves.json'), 'utf8'));
const added = ['skillswap', 'roleplay', 'entrainment', 'simplebeam', 'worryseed', 'gastroacid', 'imprison'];
const moves = [...s3.moves, ...added.filter(m => !s3.moves.includes(m))].sort();
// moves that meet abilities often: ability changes, contact, sound, punch, bite, pulse, slicing, wind, bullet,
// powder, multi-hit, priority, recoil, secondaries, weather, terrain and status moves
const interacting = moves.filter(id => {
  const m = dex.moves.get(id);
  const f = m.flags || {};
  return added.includes(id) || f.sound || f.punch || f.bite || f.pulse || f.slicing || f.wind || f.bullet || f.powder ||
    m.multihit || m.priority > 0 || m.recoil || m.hasCrashDamage || (m.secondaries && m.secondaries.length) ||
    m.weather || m.terrain || m.status || m.volatileStatus || (m.boosts && m.target !== 'self') ||
    ['Normal'].includes(m.type) && m.category !== 'Status';
});
const unimplemented = [...legalAbilities].filter(a => !implemented.includes(a)).sort();
const out = {
  showdown: require(path.join(REPO, 'tools', 'rig', 'data', 'stage3_moves.json')).showdown,
  generator: 'tools/showdown-oracle/make_stage4.js',
  rule: 'legal species without Megas and battle-only forms, each with its legal abilities that the resolver implements; ' +
    'moves: the stage-3 pool plus the ability-changing moves and Imprison; items off',
  abilities: implemented.sort(),
  legal_abilities: [...legalAbilities].sort(),
  unimplemented,
  species,
  moves,
  added,
  interacting,
};
fs.writeFileSync(path.join(REPO, 'tools', 'rig', 'data', 'stage4.json'), JSON.stringify(out, null, 1) + '\n');
console.log(JSON.stringify({species: species.length, implemented: implemented.length, legal: legalAbilities.size,
  unimplemented: unimplemented.length, moves: moves.length, interacting: interacting.length}));
