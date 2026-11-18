// Dump the description text of every legal Champions move, item and ability. Read-only on Showdown.
// usage: node 10-dump_descriptions.js <showdown dir> <showdown-champions.json> <out.json>
const path = require('path');
const fs = require('fs');
const [SD, dumpPath, out] = process.argv.slice(2);
const {Dex} = require(path.join(SD, 'dist', 'sim'));
const ch = Dex.mod('champions');
const dump = JSON.parse(fs.readFileSync(dumpPath, 'utf8'));
const text = ch.loadTextData();

// newest description for the mod: champions override, else gen9, else base
function descs(table, id) {
  const e = (text[table] || {})[id] || {};
  const pick = (k) => (e.champions && e.champions[k]) || (e.gen9 && e.gen9[k]) || e[k] || '';
  return {name: e.name || id, shortDesc: pick('shortDesc'), desc: pick('desc')};
}

const res = {};
const counts = {};
for (const [kind, table] of [['moves', 'Moves'], ['items', 'Items'], ['abilities', 'Abilities']]) {
  res[kind] = {};
  let n = 0, none = 0, empty = 0;
  for (const id of dump.legal[kind]) {
    const d = descs(table, id);
    res[kind][id] = d;
    n++;
    if (!d.shortDesc && !d.desc) empty++;
    else if (/^No additional effect|^No competitive use/.test(d.shortDesc || d.desc)) none++;
  }
  counts[kind] = {legal: n, no_effect: none, no_text: empty, with_effect: n - none - empty};
}
console.log(JSON.stringify(counts));
fs.writeFileSync(out, JSON.stringify({counts, ...res}, null, 1));
