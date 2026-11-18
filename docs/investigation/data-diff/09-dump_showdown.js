// Dump Champions-mod data plus Champions-vs-gen9 raw-data diffs for 09-data-diff. Read-only on Showdown.
// usage: node 09-dump_showdown.js <showdown dir> <out.json>
const path = require('path');
const fs = require('fs');
const SD = process.argv[2];
const out = process.argv[3];
const {Dex} = require(path.join(SD, 'dist', 'sim'));
const ch = Dex.mod('champions');
const g9 = Dex.mod('gen9');

const rep = (k, v) => typeof v === 'function' ? '[fn]' : v;
const plain = (o) => o === undefined ? undefined : JSON.parse(JSON.stringify(o, rep));

// handler names, recursing into nested objects (condition, secondary, self)
function handlers(o, prefix = '', acc = [], depth = 0) {
  if (!o || typeof o !== 'object' || depth > 3) return acc;
  for (const k of Object.keys(o)) {
    const v = o[k];
    if (typeof v === 'function') acc.push(prefix + k);
    else if (v && typeof v === 'object' && !Array.isArray(v) && ['condition', 'secondary', 'self'].includes(k)) handlers(v, prefix + k + '.', acc, depth + 1);
    else if (Array.isArray(v) && k === 'secondaries') v.forEach((s, i) => handlers(s, `${prefix}${k}[${i}].`, acc, depth + 1));
  }
  return acc;
}

// stringify with function source, for diffs
const srcRep = (k, v) => typeof v === 'function' ? 'FN:' + v.toString() : v;
const sstr = (v) => v === undefined ? undefined : JSON.stringify(v, srcRep);

function diffEntry(a, b, prefix = '', acc = []) {
  // a = champions raw, b = gen9 raw
  const keys = new Set([...Object.keys(a || {}), ...Object.keys(b || {})]);
  for (const k of keys) {
    if (k === 'inherit') continue;
    const va = a ? a[k] : undefined, vb = b ? b[k] : undefined;
    if (sstr(va) === sstr(vb)) continue;
    const isObj = (x) => x && typeof x === 'object' && !Array.isArray(x);
    if (isObj(va) && isObj(vb) && ['condition', 'flags', 'secondary', 'self', 'boosts'].includes(k)) {
      diffEntry(va, vb, prefix + k + '.', acc);
    } else {
      const show = (x) => typeof x === 'function' ? '[fn]' : (x === undefined ? null : plain(x));
      acc.push({field: prefix + k, champions: show(va), gen9: show(vb)});
    }
  }
  return acc;
}

function tableDiff(name) {
  const A = ch.data[name] || {}, B = g9.data[name] || {};
  const res = {};
  for (const id of new Set([...Object.keys(A), ...Object.keys(B)])) {
    const d = diffEntry(A[id], B[id]);
    if (d.length) res[id] = d;
  }
  return res;
}

const moves = {};
for (const m of ch.moves.all()) {
  const o = plain(m);
  o.handlers = handlers(m);
  moves[m.id] = o;
}
const items = {};
for (const i of ch.items.all()) {
  const o = plain(i);
  o.handlers = handlers(i);
  items[i.id] = o;
}
const abilities = {};
for (const a of ch.abilities.all()) {
  const o = plain(a);
  o.handlers = handlers(a);
  abilities[a.id] = o;
}
const species = {};
for (const s of ch.species.all()) {
  species[s.id] = {
    id: s.id, name: s.name, num: s.num, baseSpecies: s.baseSpecies, forme: s.forme, types: s.types,
    baseStats: s.baseStats, abilities: s.abilities, weightkg: s.weightkg, requiredItem: s.requiredItem,
    requiredItems: s.requiredItems, isMega: !!s.isMega, isNonstandard: s.isNonstandard, tier: s.tier,
    battleOnly: s.battleOnly, changesFrom: s.changesFrom, otherFormes: s.otherFormes, cosmeticFormes: s.cosmeticFormes,
  };
}
const typechart = {};
for (const t of ch.types.all()) typechart[t.name] = {damageTaken: plain(t.damageTaken), isNonstandard: t.isNonstandard};
const typechart9 = {};
for (const t of g9.types.all()) typechart9[t.name] = {damageTaken: plain(t.damageTaken)};
const natures = {};
for (const n of ch.natures.all()) natures[n.id] = {name: n.name, plus: n.plus || null, minus: n.minus || null};

const diffs = {Moves: tableDiff('Moves'), Items: tableDiff('Items'), Abilities: tableDiff('Abilities'), Species: tableDiff('Pokedex'), Conditions: tableDiff('Conditions')};
const modDir = path.join(SD, 'data', 'mods', 'champions');
const modFiles = fs.readdirSync(modDir);

const legal = {
  moves: Object.values(moves).filter(m => !m.isNonstandard).map(m => m.id),
  items: Object.values(items).filter(i => !i.isNonstandard).map(i => i.id),
  abilities: Object.values(abilities).filter(a => !a.isNonstandard).map(a => a.id),
  species: Object.values(species).filter(s => !s.isNonstandard).map(s => s.id),
  species_tier: Object.values(species).filter(s => !s.isNonstandard && s.tier !== 'Illegal' && s.tier !== undefined).map(s => s.id),
};
const counts = {};
for (const k of Object.keys(legal)) counts[k] = legal[k].length;
for (const k of Object.keys(diffs)) counts['diff_' + k] = Object.keys(diffs[k]).length;
console.log(JSON.stringify(counts));
fs.writeFileSync(out, JSON.stringify({counts, legal, moves, items, abilities, species, typechart, typechart9, natures, diffs, modFiles}));
