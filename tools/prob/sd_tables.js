// Compares CG's per-move random-event numbers (prob_check --tables) with Showdown's Champions dex:
// accuracy, crit ratio, willCrit, multihit, multiaccuracy, OHKO, secondary chances, self chance.
// Usage: node sd_tables.js CG_TABLES.json OUT.json
'use strict';
const fs = require('fs');
const path = require('path');
const SD = process.env.SHOWDOWN_DIR || 'C:/Users/user/Code/pokemon_champions/pcsa/.tools/vendor/pokemon-showdown';
const {Dex} = require(path.join(SD, 'dist', 'sim', 'dex'));
const DEX = Dex.forFormat('gen9championsbssregmc');

const [cgPath, outPath] = process.argv.slice(2);
const cg = JSON.parse(fs.readFileSync(cgPath, 'utf8'));
const diffs = [];
let n = 0;
for (const id of Object.keys(cg)) {
  const m = DEX.moves.get(id);
  if (!m.exists) { diffs.push({id, field: 'exists', cg: true, sd: false}); continue; }
  n++;
  const e = cg[id];
  const sd = {
    accuracy: m.accuracy,
    critRatio: m.critRatio || 1,
    willCrit: m.willCrit === undefined ? null : m.willCrit,
    multihit: m.multihit === undefined ? [0, 0] : Array.isArray(m.multihit) ? m.multihit : [m.multihit, m.multihit],
    multiaccuracy: !!m.multiaccuracy,
    ohko: m.ohko ? (m.ohko === true ? true : String(m.ohko).toLowerCase()) : false,
    secondaries: (m.secondaries || []).map(s => (s.chance === undefined ? null : s.chance)),
    self: m.self ? (m.self.chance === undefined ? 'always' : m.self.chance) : null,
  };
  for (const f of Object.keys(sd)) {
    if (JSON.stringify(sd[f]) !== JSON.stringify(e[f])) diffs.push({id, field: f, cg: e[f], sd: sd[f]});
  }
}
fs.writeFileSync(outPath, JSON.stringify({moves: n, differences: diffs.length, diffs}, null, 1));
console.log(`${n} moves compared, ${diffs.length} differences`);
for (const d of diffs.slice(0, 60)) console.log(JSON.stringify(d));
