// Generates data/ (species, moves, natures, SOURCE) from Showdown's Champions mod. Read-only on Showdown.
// usage: node tools/data/gen_data.js [out_dir]   (Showdown from $SHOWDOWN_DIR, default below)
'use strict';
const path = require('path');
const fs = require('fs');
const {execFileSync} = require('child_process');

const SD = process.env.SHOWDOWN_DIR ||
  'C:\\Users\\user\\Code\\pokemon_champions\\pcsa\\.tools\\vendor\\pokemon-showdown';
const OUT = process.argv[2] || path.join(__dirname, '..', '..', 'data');
const FORMAT = 'gen9championsbssregmc';

const {Dex, Battle} = require(path.join(SD, 'dist', 'sim'));
const ch = Dex.mod('champions');
const toID = (s) => ('' + (s || '')).toLowerCase().replace(/[^a-z0-9]+/g, '');

// Sorted keys at every level; one top-level entry per line.
function sortDeep(v) {
  if (Array.isArray(v)) return v.map(sortDeep);
  if (v && typeof v === 'object') {
    const o = {};
    for (const k of Object.keys(v).sort()) o[k] = sortDeep(v[k]);
    return o;
  }
  return v;
}
function write(name, obj) {
  const s = sortDeep(obj);
  const lines = Object.keys(s).map((k) => `${JSON.stringify(k)}: ${JSON.stringify(s[k])}`);
  fs.writeFileSync(path.join(OUT, name), '{\n' + lines.join(',\n') + '\n}\n');
  return lines.length;
}

fs.mkdirSync(OUT, {recursive: true});

// Species: the 392 legal ids (not isNonstandard), as in docs/investigation/data-diff/09-dump_showdown.js.
const species = {};
const legal = ch.species.all().filter((s) => !s.isNonstandard);
for (const s of legal) {
  const abilities = {};
  for (const slot of Object.keys(s.abilities)) abilities[slot] = toID(s.abilities[slot]);
  const e = {
    name: s.name,
    baseSpecies: toID(s.baseSpecies),
    forme: s.forme || '',
    types: s.types.map(toID),
    baseStats: {...s.baseStats},
    abilities,
    weightkg: s.weightkg,
    isMega: !!s.isMega,
  };
  if (s.isMega) {
    e.megaStone = toID(s.requiredItem);
    e.megaFrom = toID(s.changesFrom || s.baseSpecies);
  }
  species[s.id] = e;
}
// Base species: the stones that Mega Evolve it, stone -> Mega id.
for (const id of Object.keys(species)) {
  const e = species[id];
  if (!e.isMega) continue;
  const base = species[e.megaFrom];
  if (!base) throw new Error(`mega ${id} without legal base ${e.megaFrom}`);
  (base.megas = base.megas || {})[e.megaStone] = id;
}

// Moves: base PP after the mod's cap and max PP from the format's calculatePP.
const battle = new Battle({formatid: FORMAT});
const moves = {};
for (const m of ch.moves.all()) {
  if (m.isNonstandard) continue;
  const ppUps = m.noPPBoosts || m.id === 'trumpcard' ? 0 : 3; // as sim/pokemon.ts
  const maxpp = battle.calculatePP(battle.dex.moves.get(m.id), ppUps);
  moves[m.id] = {name: m.name, pp: m.pp, maxpp, noPPBoosts: !!m.noPPBoosts};
}
battle.destroy();

const natures = {};
for (const n of ch.natures.all()) natures[n.id] = {name: n.name, plus: n.plus || '', minus: n.minus || ''};

let commit = '';
try {
  commit = execFileSync('git', ['-C', SD, 'rev-parse', 'HEAD'], {encoding: 'utf8'}).trim();
} catch (e) {
  throw new Error('cannot read the Showdown commit: ' + e.message);
}
const counts = {
  species: write('species.json', species),
  moves: write('moves.json', moves),
  natures: write('natures.json', natures),
};
write('SOURCE.json', {
  showdown: {repository: 'https://github.com/smogon/pokemon-showdown', commit, mod: 'champions', format: FORMAT},
  generator: 'tools/data/gen_data.js',
  legality: 'species and moves without isNonstandard in Dex.mod("champions")',
  counts,
});
console.log(JSON.stringify(counts));
