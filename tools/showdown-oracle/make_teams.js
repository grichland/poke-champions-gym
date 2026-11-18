// Builds tools/rig/data/teams.json: ladder rosters with a set per member, validated by Showdown.
// usage: node make_teams.js <ladder.json> <top_builds.json> <out.json>
// A member's set: the build seen with that roster, else a seen build drawn by count, else the top builds,
// skipping builds Showdown's validator rejects or whose item is already taken (Item Clause). A roster with a
// member left without a set, or a team the validator rejects, is dropped.
'use strict';
const fs = require('fs');
const path = require('path');
const {SIM, FORMAT} = require('./oracle');
const {TeamValidator} = require(path.join(SIM, 'team-validator'));
const {Dex} = require(path.join(SIM, 'dex'));

const [ladderPath, topPath, outPath] = process.argv.slice(2);
if (!outPath) { console.error('usage: node make_teams.js <ladder.json> <top_builds.json> <out.json>'); process.exit(2); }
const ladder = JSON.parse(fs.readFileSync(ladderPath, 'utf8'));
const top = JSON.parse(fs.readFileSync(topPath, 'utf8'));
const dex = Dex.forFormat(FORMAT);
const validator = TeamValidator.get(FORMAT);
const toID = s => String(s || '').toLowerCase().replace(/[^a-z0-9]/g, '');

let seed = 20261117;
function rand() { // mulberry32
  seed = (seed + 0x6D2B79F5) | 0;
  let t = seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}
function weighted(items, w) {
  const total = w.reduce((a, b) => a + b, 0);
  let x = rand() * total;
  for (let i = 0; i < items.length; i++) { x -= w[i]; if (x < 0) return items[i]; }
  return items[items.length - 1];
}

const seen = ladder.seen_builds.species;
const topBy = {};
for (const [name, v] of Object.entries(top.species)) topBy[toID(name)] = {name, builds: v.builds};

function asSet(species, b) {
  const sp = dex.species.get(species);
  const gender = sp.gender || (rand() < 0.5 ? 'M' : 'F');
  const evs = {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0};
  for (const k of Object.keys(evs)) evs[k] = Number(b.points[k] || 0);
  return {species: sp.name, item: b.item || '', ability: b.ability, moves: b.moves.slice(0, 4), nature: b.nature,
    evs, level: 50, gender};
}

function candidates(name, exactIndex) {
  const key = toID(name);
  const out = [];
  const s = seen[key] || [];
  if (exactIndex !== undefined && s[exactIndex]) out.push(s[exactIndex]);
  if (s.length) {
    const order = s.slice();
    const first = weighted(order, order.map(e => e.count || 1));
    out.push(first, ...order.filter(e => e !== first));
  }
  for (const b of (topBy[key] ? topBy[key].builds : [])) out.push(b);
  return out;
}

const teams = [];
const rejected = [];
let noBuild = 0, setRejects = 0;
const missing = {};
ladder.rosters.forEach((roster, n) => {
  const exact = (ladder.roster_builds && ladder.roster_builds[n]) || {};
  const team = [];
  const items = new Set();
  for (const name of roster) {
    let set = null;
    for (const b of candidates(name, exact[toID(name)])) {
      if (b.item && items.has(toID(b.item))) continue;
      const s = asSet(b.species || name, b);
      if (validator.validateSet(JSON.parse(JSON.stringify(s)), {})) { setRejects++; continue; }
      set = s;
      break;
    }
    if (!set) { noBuild++; missing[name] = (missing[name] || 0) + 1; return; }
    if (set.item) items.add(toID(set.item));
    team.push(set);
  }
  const problems = validator.validateTeam(JSON.parse(JSON.stringify(team)));
  if (problems) { rejected.push({row: n, problems}); return; }
  teams.push(team);
});

fs.mkdirSync(path.dirname(outPath), {recursive: true});
fs.writeFileSync(outPath, '[\n' + teams.map(t => JSON.stringify(t)).join(',\n') + '\n]\n');
const report = {rosters: ladder.rosters.length, kept: teams.length, rejected: rejected.length, noBuild, setRejects, missing,
  rejections: rejected.map(r => ({row: r.row, problems: r.problems.slice(0, 3)}))};
console.log(JSON.stringify(report, null, 1));
