// Whole random games on Showdown through the rig oracle (record mode: Showdown's own sodium PRNG), with the
// same aggregates as prob_check --games: turns, accuracy misses, crits, full paralysis, multi-hit counts
// (from the oracle's events), and miss / move protocol lines.
// Usage: node sd_games.js TEAMS_JSON FIRST_GAME GAMES OUT.jsonl [TIME_CAP_S]
'use strict';
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const oracle = require(path.join(__dirname, '..', 'showdown-oracle', 'oracle.js'));

const [teamsPath, firstArg, gamesArg, out, capArg] = process.argv.slice(2);
const teams = JSON.parse(fs.readFileSync(teamsPath, 'utf8')).filter(t => t.length === 6);
const first = Number(firstArg), games = Number(gamesArg), cap = Number(capArg || 570);

// choices from a seeded xorshift128+ (independent of the battle's PRNG)
let s0 = 0x9e3779b97f4a7c15n ^ BigInt(first), s1 = 0xbf58476d1ce4e5b9n;
const M = (1n << 64n) - 1n;
function next() {
  let x = s0, y = s1;
  s0 = y; x ^= (x << 23n) & M; s1 = x ^ y ^ (x >> 17n) ^ (y >> 26n);
  return Number(((s1 + y) & M) >> 11n) / 2 ** 53;
}
const below = n => Math.floor(next() * n);
for (let i = 0; i < 20; i++) next();

function choices(o) {
  const out = [];
  if (o.kind === 'turn') {
    for (const m of o.moves) { out.push(`move ${m}`); if (o.mega) out.push(`move ${m} mega`); }
    for (const s of o.switches) out.push(`switch ${s}`);
  } else if (o.kind === 'replace') {
    for (const s of o.switches) out.push(`switch ${s}`);
  }
  return out;
}

const t0 = Date.now();
let done = 0, errors = 0;
for (let g = first; g < first + games; g++) {
  if ((Date.now() - t0) / 1000 > cap) break;
  const a = below(teams.length);
  let b = below(teams.length - 1); if (b >= a) b++;
  const picks = {};
  for (const sid of ['p1', 'p2']) {
    const v = [0, 1, 2, 3, 4, 5];
    for (let i = 0; i < 3; i++) { const j = i + below(6 - i); [v[i], v[j]] = [v[j], v[i]]; }
    picks[sid] = v.slice(0, 3);
  }
  const id = 'g' + g;
  const seed = 'sodium,' + crypto.createHash('sha256').update('game:' + g).digest('hex');
  let r = oracle.handle({op: 'new', id, seed, teams: {p1: teams[a], p2: teams[b]}, picks});
  if (!r.ok) { errors++; continue; }
  const st = {g, acc: [0, 0], crit: [0, 0], para: [0, 0], hits: [0, 0], moves: 0, miss_rows: 0, error: null};
  const tally = (rep) => {
    for (const e of rep.events || []) {
      if (e.key === 'accuracy') { st.acc[1]++; if (e.value === false) st.acc[0]++; }
      if (e.key === 'crit') { st.crit[1]++; if (e.value === true) st.crit[0]++; }
      if (e.key === 'fullpara') { st.para[1]++; if (e.value === true) st.para[0]++; }
      if (e.key === 'hits') { st.hits[1]++; st.hits[0] += e.value; }
    }
    for (const l of rep.log || []) {
      if (l.startsWith('|move|')) st.moves++;
      if (l.startsWith('|-miss|')) st.miss_rows++;
    }
  };
  tally(r);
  let k = 0;
  while (k < 1000) {
    const o = r.options;
    if (o.p1.kind === 'done') break;
    const ch = {};
    for (const sid of ['p1', 'p2']) {
      const cs = choices(o[sid]);
      if (cs.length) ch[sid] = cs[below(cs.length)];
    }
    r = oracle.handle({op: 'step', id, choices: ch});
    if (!r.ok) { st.error = r.error; errors++; break; }
    tally(r);
    k++;
  }
  const h = oracle.battles.get(id);
  st.turns = h.battle.turn;
  st.ended = !!h.battle.ended;
  oracle.handle({op: 'release', id});
  fs.appendFileSync(out, JSON.stringify(st) + '\n');
  done++;
}
console.log(JSON.stringify({first, games_done: done, errors, wall_s: (Date.now() - t0) / 1000}));
