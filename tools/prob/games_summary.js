// Aggregates per-game lines (sd_games.js or prob_check --games-out) into the same ratios as prob_check:
// ratio of sums with a per-game (cluster) standard error.
// Usage: node games_summary.js OUT.json FILE.jsonl [FILE.jsonl ...]
'use strict';
const fs = require('fs');
const [out, ...files] = process.argv.slice(2);
const G = [];
for (const f of files) for (const l of fs.readFileSync(f, 'utf8').split('\n')) if (l.trim()) G.push(JSON.parse(l));
function ratio(x, n) {
  const sx = G.reduce((a, g) => a + x(g), 0), sn = G.reduce((a, g) => a + n(g), 0);
  const r = sx / sn, k = G.length, nbar = sn / k;
  const v = G.reduce((a, g) => a + (x(g) - r * n(g)) ** 2, 0) / (k - 1);
  return {value: r, se: Math.sqrt(v / k) / nbar, num: sx, den: sn};
}
const k = G.length;
const mt = G.reduce((a, g) => a + g.turns, 0) / k;
const sdt = Math.sqrt(G.reduce((a, g) => a + (g.turns - mt) ** 2, 0) / (k - 1));
const res = {
  games: k,
  errors: G.filter(g => g.error).length,
  unfinished: G.filter(g => g.ended === false).length,
  turns: {value: mt, se: sdt / Math.sqrt(k)},
  miss_share_of_accuracy_draws: ratio(g => g.acc[0], g => g.acc[1]),
  crit_share_of_crit_draws: ratio(g => g.crit[0], g => g.crit[1]),
  fullpara_share: ratio(g => g.para[0], g => g.para[1]),
  mean_hits_variable_multihit: ratio(g => g.hits[0], g => g.hits[1]),
  miss_rows_per_move_row: ratio(g => g.miss_rows, g => g.moves),
};
fs.writeFileSync(out, JSON.stringify(res, null, 1));
console.log(JSON.stringify(res, null, 1));
