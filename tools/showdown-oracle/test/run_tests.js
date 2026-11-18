// Oracle tests: round trip (record then force), event counts, targeted forced outcomes, protocol, speed.
// usage: node test/run_tests.js [--battles N] [--seed S] [--out results-file-name]
// Writes test/results/; exits non-zero on any failure.
'use strict';
const fs = require('fs');
const path = require('path');
const {spawnSync} = require('child_process');
const oracle = require('../oracle');

const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : d; };
const BATTLES = Number(opt('--battles', 300));
const SEED = Number(opt('--seed', 1));
const OUT = opt('--out', 'run_tests.json');
const MAX_STEPS = 400;
const RESULTS = path.join(__dirname, 'results');
const TEAMS = JSON.parse(fs.readFileSync(path.join(__dirname, '..', '..', 'rig', 'data', 'teams.json'), 'utf8'));

const failures = [];
const out = {};
function fail(msg) { failures.push(msg); console.log('FAIL ' + msg); }
function check(cond, msg) { if (!cond) fail(msg); return cond; }

function rng(seed) {
  let s = seed | 0;
  return () => {
    s = (s + 0x6D2B79F5) | 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const pick = (r, a) => a[Math.floor(r() * a.length)];

let nextId = 0;
function call(req) {
  const res = oracle.handle(req);
  return JSON.parse(JSON.stringify(res));
}

function randomChoice(r, o) {
  if (o.kind === 'replace') return 'switch ' + pick(r, o.switches);
  if (o.kind !== 'turn') return undefined;
  if (o.switches.length && r() < 0.15) return 'switch ' + pick(r, o.switches);
  const m = pick(r, o.moves);
  return 'move ' + m + (o.mega && r() < 0.5 ? ' mega' : '');
}

// ------------------------------------------------------------------------------------------- round trip
function playRecorded(r, spec) {
  const id = 'rec' + nextId++;
  const first = call(Object.assign({op: 'new', id}, spec));
  if (!first.ok) throw new Error('new failed: ' + JSON.stringify(first));
  const steps = [];
  let cur = first;
  let t0 = process.hrtime.bigint();
  while (!cur.state.ended && steps.length < MAX_STEPS) {
    const choices = {};
    for (const sid of ['p1', 'p2']) {
      const c = randomChoice(r, cur.options[sid]);
      if (c !== undefined) choices[sid] = c;
    }
    const res = call({op: 'step', id, choices});
    if (!res.ok) throw new Error('step failed: ' + JSON.stringify(res) + ' choices ' + JSON.stringify(choices));
    steps.push({choices, res});
    cur = res;
  }
  const ns = Number(process.hrtime.bigint() - t0);
  call({op: 'release', id});
  return {first, steps, ns};
}

function replayForced(spec, rec) {
  const id = 'rep' + nextId++;
  const t0 = process.hrtime.bigint();
  const first = call(Object.assign({op: 'new', id, force: rec.first.events}, spec));
  const problems = [];
  const same = (a, b, what) => {
    const x = JSON.stringify(a), y = JSON.stringify(b);
    if (x !== y) problems.push(what + ': ' + firstDiff(a, b));
  };
  if (!first.ok) { problems.push('new: ' + JSON.stringify(first)); return {problems}; }
  same(first.state, rec.first.state, 'new state');
  same(first.options, rec.first.options, 'new options');
  rec.steps.forEach((s, i) => {
    if (problems.length) return;
    const res = call({op: 'step', id, choices: s.choices, force: s.res.events});
    if (!res.ok) { problems.push(`step ${i}: ${JSON.stringify(res)}`); return; }
    same(res.state, s.res.state, `step ${i} state`);
    same(res.options, s.res.options, `step ${i} options`);
    same(res.events, s.res.events, `step ${i} events`);
    same(cleanLog(res.log), cleanLog(s.res.log), `step ${i} log`);
  });
  call({op: 'release', id});
  return {problems, ns: Number(process.hrtime.bigint() - t0)};
}

const cleanLog = log => log.filter(l => !l.startsWith('|t:|'));

function firstDiff(a, b, p = '') {
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) {
    return a === b ? null : `${p}: ${JSON.stringify(a)} vs ${JSON.stringify(b)}`;
  }
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  for (const k of keys) {
    const d = firstDiff(a[k], b[k], p + '.' + k);
    if (d) return d;
  }
  return null;
}

function battleSpec(r) {
  const t1 = pick(r, TEAMS), t2 = pick(r, TEAMS);
  const picks = () => { const a = [0, 1, 2, 3, 4, 5]; for (let i = 5; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a.slice(0, 3); };
  return {seed: [1, 2, 3, 4].map(() => Math.floor(r() * 65536)), teams: {p1: t1, p2: t2}, picks: {p1: picks(), p2: picks()}};
}

function roundTrip() {
  const r = rng(SEED);
  const counts = {};
  const rawSites = {};
  const volatileKeys = {};
  const byEffect = {};
  let steps = 0, recNs = 0, repNs = 0, mismatched = 0, events = 0, ended = 0;
  const examples = [];
  for (let n = 0; n < BATTLES; n++) {
    const spec = battleSpec(r);
    let rec;
    try { rec = playRecorded(r, spec); } catch (e) { fail(`battle ${n} record: ${e.message}`); continue; }
    steps += rec.steps.length;
    recNs += rec.ns;
    if (rec.steps.length && rec.steps[rec.steps.length - 1].res.state.ended) ended++;
    for (const ev of [rec.first.events, ...rec.steps.map(s => s.res.events)].flat()) {
      events++;
      counts[ev.key] = (counts[ev.key] || 0) + 1;
      if (ev.effect !== undefined && ev.key !== 'raw') {
        const k = `${ev.key}|${ev.effect}`;
        byEffect[k] = (byEffect[k] || 0) + 1;
      }
      if (ev.key === 'raw') {
        const k = `${ev.effect}|${ev.kind}|${JSON.stringify(ev.args)}`;
        rawSites[k] = (rawSites[k] || 0) + 1;
      }
    }
    for (const s of rec.steps) {
      for (const sid of ['p1', 'p2']) {
        for (const m of s.res.state.sides[sid].pokemon) {
          for (const [v, d] of Object.entries(m.volatiles)) volatileKeys[v] = Object.keys(d).join(',');
        }
      }
    }
    const rep = replayForced(spec, rec);
    repNs += rep.ns || 0;
    if (rep.problems.length) {
      mismatched++;
      if (examples.length < 10) examples.push({battle: n, spec, problems: rep.problems.slice(0, 3)});
      fail(`battle ${n} replay: ${rep.problems[0]}`);
    }
  }
  const raw = counts.raw || 0;
  out.roundTrip = {
    battles: BATTLES, finished: ended, steps, events, mismatchedBattles: mismatched,
    recordStepsPerSecond: Math.round(steps / (recNs / 1e9)),
    forceStepsPerSecond: Math.round(steps / (repNs / 1e9)),
    eventsByKey: Object.fromEntries(Object.entries(counts).sort((a, b) => b[1] - a[1])),
    rawShare: events ? raw / events : 0,
    eventsByEffect: Object.fromEntries(Object.entries(byEffect).sort((a, b) => b[1] - a[1])),
    rawSites: Object.fromEntries(Object.entries(rawSites).sort((a, b) => b[1] - a[1])),
    volatilesSeen: volatileKeys,
    mismatchExamples: examples,
  };
  check(mismatched === 0, `${mismatched} battles did not replay identically`);
  check(out.roundTrip.rawShare < 0.01, `raw share ${out.roundTrip.rawShare} not under 1%`);
}

// ------------------------------------------------------------------------------------------- targeted
const FILL = TEAMS[0];
function teamWith(lead, second) {
  const t = FILL.map(s => JSON.parse(JSON.stringify(s)));
  t[0] = lead;
  if (second) t[1] = second;
  return t;
}
const set = (species, ability, moves, item = '', extra = {}) => Object.assign({species, item, ability, moves, nature: 'Serious',
  evs: {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0}, level: 50, gender: 'M'}, extra);
const GARCHOMP = set('Garchomp', 'Rough Skin', ['Earthquake', 'Stone Edge', 'Fire Fang', 'Swords Dance']);
const SNORLAX = set('Snorlax', 'Thick Fat', ['Yawn', 'Body Slam', 'Curse', 'Rest']);
const AMOONGUSS = set('Amoonguss', 'Regenerator', ['Spore', 'Giga Drain', 'Clear Smog', 'Protect']);

function duel(p1, p2) {
  const id = 't' + nextId++;
  const res = call({op: 'new', id, seed: [7, 7, 7, 7], teams: {p1: teamWith(p1), p2: teamWith(p2)}, picks: {p1: [0, 1, 2], p2: [0, 1, 2]}});
  if (!res.ok) throw new Error('new: ' + JSON.stringify(res));
  return {id, res};
}

const ev = (key, side, value, extra = {}) => Object.assign({key, side, value}, extra);

// Showdown's damage chain for a plain hit: no items, abilities, weather or screens
function handDamage(level, bp, atk, def, roll, crit, stab, eff) {
  const tr = Math.trunc;
  const modify = (v, m) => tr((tr(v * tr(m * 4096)) + 2048 - 1) / 4096);
  let d = tr(tr(tr(tr(2 * level / 5 + 2) * bp * atk) / def) / 50) + 2;
  if (crit) d = tr(d * 1.5);
  d = tr(tr(d * (100 - roll)) / 100);
  if (stab) d = modify(d, 1.5);
  if (eff > 1) d *= eff;
  if (eff < 1) d = tr(d * eff);
  return Math.max(1, d);
}

function targeted() {
  const res = {};
  // damage: Earthquake (accuracy 100: no event) with crit and roll forced
  const dmg = {};
  for (const [crit, roll] of [[false, 0], [false, 15], [true, 0], [false, 7]]) {
    const {id, res: r0} = duel(GARCHOMP, SNORLAX);
    const hp0 = r0.state.sides.p2.pokemon[0].hp;
    const s = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 0'},
      force: [ev('crit', 'p1', crit, {move: 'earthquake'}), ev('roll', 'p1', roll, {move: 'earthquake'})]});
    if (!check(s.ok, `damage step crit=${crit} roll=${roll}: ${JSON.stringify(s)}`)) continue;
    const got = hp0 - s.state.sides.p2.pokemon[0].hp;
    const atk = r0.state.sides.p1.pokemon[0].stats.atk, def = r0.state.sides.p2.pokemon[0].stats.def;
    const want = handDamage(50, 100, atk, def, roll, crit, true, 1);
    dmg[`crit=${crit} roll=${roll}`] = {got, want};
    check(got === want, `damage crit=${crit} roll=${roll}: got ${got}, hand formula ${want}`);
    call({op: 'release', id});
  }
  check(dmg['crit=false roll=0'] && dmg['crit=false roll=15'] && dmg['crit=false roll=0'].got > dmg['crit=false roll=15'].got,
    'roll 0 must be the maximum roll');
  res.damage = dmg;

  // miss: Stone Edge (accuracy 80)
  {
    const {id, res: r0} = duel(GARCHOMP, SNORLAX);
    const s = call({op: 'step', id, choices: {p1: 'move 1', p2: 'move 2'}, force: [ev('accuracy', 'p1', false, {move: 'stoneedge'})]});
    const ok = check(s.ok, 'miss step: ' + JSON.stringify(s)) &&
      check(s.state.sides.p2.pokemon[0].hp === r0.state.sides.p2.pokemon[0].hp, 'forced miss changed the target HP') &&
      check(s.log.some(l => l.startsWith('|-miss|')), 'forced miss: no -miss line');
    res.miss = ok ? 'ok' : 'failed';
    const s2 = call({op: 'step', id, choices: {p1: 'move 1', p2: 'move 2'},
      force: [ev('accuracy', 'p1', true), ev('crit', 'p1', true), ev('roll', 'p1', 3)]});
    check(s2.ok && s2.log.some(l => l.startsWith('|-crit|')), 'forced hit and crit: no -crit line ' + JSON.stringify(s2).slice(0, 300));
    call({op: 'release', id});
  }

  // full paralysis: Body Slam's 30% paralysis forced, then Garchomp fully paralysed, then not
  {
    const {id} = duel(GARCHOMP, SNORLAX);
    const s1 = call({op: 'step', id, choices: {p1: 'move 3', p2: 'move 1'},
      force: [ev('crit', 'p2', false), ev('roll', 'p2', 0), ev('secondary', 'p2', true, {index: 0})]});
    let ok = check(s1.ok && s1.state.sides.p1.pokemon[0].status === 'par', 'forced Body Slam paralysis: ' + JSON.stringify(s1).slice(0, 400));
    const hp = s1.ok ? s1.state.sides.p2.pokemon[0].hp : 0;
    const s2 = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 2'}, force: [ev('fullpara', 'p1', true)]});
    ok = check(s2.ok, 'full paralysis step: ' + JSON.stringify(s2).slice(0, 400)) &&
      check(s2.log.some(l => l.startsWith('|cant|p1a: Garchomp|par')), 'forced full paralysis: no cant line') &&
      check(s2.state.sides.p2.pokemon[0].hp === hp, 'fully paralysed Garchomp still did damage') && ok;
    const s3 = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 2'},
      force: [ev('fullpara', 'p1', false), ev('crit', 'p1', false), ev('roll', 'p1', 0)]});
    ok = check(s3.ok && s3.state.sides.p2.pokemon[0].hp < hp, 'not fully paralysed: Garchomp should hit ' + JSON.stringify(s3).slice(0, 300)) && ok;
    res.fullpara = ok ? 'ok' : 'failed';
    call({op: 'release', id});
  }

  // sleep of each length: Spore (accuracy 100: no event), sleep length forced
  res.sleep = {};
  for (const turns of [2, 3]) {
    const {id} = duel(GARCHOMP, AMOONGUSS);
    const s1 = call({op: 'step', id, choices: {p1: 'move 3', p2: 'move 0'}, force: [ev('sleepturns', 'p1', turns)]});
    if (!check(s1.ok && s1.state.sides.p1.pokemon[0].status === 'slp', `spore ${turns}: ${JSON.stringify(s1).slice(0, 400)}`)) continue;
    check(s1.state.sides.p1.pokemon[0].statusData.turns === turns, `sleep counter ${s1.state.sides.p1.pokemon[0].statusData.turns}, forced ${turns}`);
    let asleep = 0, woke = false;
    for (let t = 0; t < 5 && !woke; t++) {
      const s = call({op: 'step', id, choices: {p1: 'move 3', p2: 'move 3'}});
      if (!check(s.ok, 'sleep turn: ' + JSON.stringify(s).slice(0, 300))) break;
      if (s.log.some(l => l.startsWith('|cant|p1a: Garchomp|slp'))) asleep++;
      if (s.log.some(l => l.startsWith('|-curestatus|p1a: Garchomp|slp'))) woke = true;
    }
    res.sleep[turns] = {turnsWithoutMoving: asleep, woke};
    check(woke && asleep === turns - 1, `sleep forced ${turns}: ${asleep} turns without moving, woke ${woke}`);
    call({op: 'release', id});
  }
  {
    const {id} = duel(GARCHOMP, AMOONGUSS);
    const s = call({op: 'step', id, choices: {p1: 'move 3', p2: 'move 0'}, force: [ev('sleepturns', 'p1', 4)]});
    check(!s.ok && s.error === 'tape', 'an impossible sleep length must be a tape error');
    call({op: 'release', id});
  }

  // 10% secondary: Fire Fang's burn (index 0) forced to happen, flinch (index 1) not
  {
    const {id} = duel(GARCHOMP, SNORLAX);
    const s = call({op: 'step', id, choices: {p1: 'move 2', p2: 'move 0'},
      force: [ev('accuracy', 'p1', true), ev('crit', 'p1', false), ev('roll', 'p1', 0),
        ev('secondary', 'p1', true, {move: 'firefang', index: 0}), ev('secondary', 'p1', false, {move: 'firefang', index: 1})]});
    const ok = check(s.ok && s.state.sides.p2.pokemon[0].status === 'brn', '10% burn forced: ' + JSON.stringify(s).slice(0, 500));
    if (s.ok) {
      const sec = s.events.filter(e => e.key === 'secondary');
      check(sec.length === 2 && sec[0].p === 0.1 && sec[1].p === 0.9, 'secondary p values ' + JSON.stringify(sec));
    }
    res.secondary10 = ok ? 'ok' : 'failed';
    call({op: 'release', id});
  }

  // speed tie between identical Garchomp: forced each way, the winner moves first
  res.speedtie = {};
  // (other ties between the two Pokemon, such as the order of end-of-turn effects, keep their recorded values)
  const rec = duel(GARCHOMP, GARCHOMP);
  const recStep = call({op: 'step', id: rec.id, choices: {p1: 'move 3', p2: 'move 3'}});
  call({op: 'release', id: rec.id});
  for (const first of ['p1', 'p2']) {
    const {id} = duel(GARCHOMP, GARCHOMP);
    const force = recStep.events.map(e => (e.key === 'speedtie' && e.effect === undefined ? Object.assign({}, e, {value: first}) : e));
    check(force.some(e => e.key === 'speedtie' && e.effect === undefined), 'mirror match must have an action speed tie');
    const s = call({op: 'step', id, choices: {p1: 'move 3', p2: 'move 3'}, force});
    if (!check(s.ok, 'speedtie step: ' + JSON.stringify(s).slice(0, 300))) continue;
    const moves = s.log.filter(l => l.startsWith('|move|')).map(l => l.split('|')[2].slice(0, 2));
    res.speedtie[first] = moves;
    check(moves[0] === first, `speedtie forced ${first}, ${moves[0]} moved first`);
    call({op: 'release', id});
  }

  // multi-hit: Rock Blast forced to 5 hits (accuracy 90, then crit and roll per hit)
  {
    const TTAR = set('Tyranitar', 'Sand Stream', ['Rock Blast', 'Crunch', 'Protect', 'Dragon Dance']);
    const {id} = duel(TTAR, SNORLAX);
    const force = [ev('accuracy', 'p1', true), ev('hits', 'p1', 5)];
    for (let i = 0; i < 5; i++) force.push(ev('crit', 'p1', false, {hit: i}), ev('roll', 'p1', 0, {hit: i}));
    const s = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 2'}, force});
    check(s.ok && s.log.some(l => /^\|-hitcount\|p2a: Snorlax\|5$/.test(l)), 'forced 5 hits: ' + JSON.stringify(s).slice(0, 400));
    if (s.ok) check(s.events.find(e => e.key === 'hits').p === 0.15, 'p of 5 hits must be 0.15');
    res.hits = s.ok ? 'ok' : 'failed';
    call({op: 'release', id});
  }

  // Mega Evolution: species and megaUsed change, the option goes away
  {
    const MEGA = set('Garchomp', 'Rough Skin', ['Earthquake', 'Stone Edge', 'Fire Fang', 'Swords Dance'], 'Garchompite');
    const {id, res: r0} = duel(MEGA, SNORLAX);
    check(r0.options.p1.mega === true, 'Garchompite holder must be offered Mega');
    const s = call({op: 'step', id, choices: {p1: 'move 3 mega', p2: 'move 2'}});
    const ok = check(s.ok, 'mega step: ' + JSON.stringify(s).slice(0, 300)) &&
      check(s.state.sides.p1.megaUsed === true && s.state.sides.p1.pokemon[0].species === 'garchompmega' &&
        s.state.sides.p1.pokemon[0].baseSpecies === 'garchomp', 'mega state ' + JSON.stringify(s.state.sides.p1.pokemon[0].species)) &&
      check(s.options.p1.mega === false, 'mega option must be gone after Mega Evolution');
    res.mega = ok ? 'ok' : 'failed';
    call({op: 'release', id});
  }

  // tape errors leave the battle unchanged
  {
    const {id, res: r0} = duel(GARCHOMP, SNORLAX);
    const a = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 0'}, force: [ev('crit', 'p1', false)]});
    check(!a.ok && a.error === 'tape', 'missing roll must be a tape error');
    const b = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 0'},
      force: [ev('crit', 'p1', false), ev('roll', 'p1', 0), ev('crit', 'p1', false)]});
    check(!b.ok && b.error === 'tape', 'unused event must be a tape error');
    const c = call({op: 'step', id, choices: {p1: 'move 0', p2: 'move 0'}, force: [ev('crit', 'p1', false), ev('roll', 'p1', 0)]});
    const want = handDamage(50, 100, r0.state.sides.p1.pokemon[0].stats.atk, r0.state.sides.p2.pokemon[0].stats.def, 0, false, true, 1);
    check(c.ok && r0.state.sides.p2.pokemon[0].hp - c.state.sides.p2.pokemon[0].hp === want && c.state.turn === 2,
      'after tape errors the battle must be as before: ' + JSON.stringify(c).slice(0, 300));
    // a failed step leaves the seeded generator where it was: later recorded steps match a battle without it
    const r = rng(5);
    const spec = battleSpec(r);
    const ia = 'ta' + nextId++, b2 = 'tb' + nextId++;
    let ra = call(Object.assign({op: 'new', id: ia}, spec));
    call(Object.assign({op: 'new', id: b2}, spec));
    let same = true;
    for (let i = 0; i < 8 && !ra.state.ended; i++) {
      const choices = {};
      for (const sid of ['p1', 'p2']) { const c = randomChoice(r, ra.options[sid]); if (c !== undefined) choices[sid] = c; }
      const bad = call({op: 'step', id: b2, choices, force: [ev('pick', 'p1', 0, {effect: 'nothing'})]});
      same = check(!bad.ok && bad.error === 'tape', 'bogus tape must fail') && same;
      ra = call({op: 'step', id: ia, choices});
      const rb = call({op: 'step', id: b2, choices});
      same = check(ra.ok && rb.ok && JSON.stringify([ra.state, ra.events]) === JSON.stringify([rb.state, rb.events]),
        `step ${i} after a failed step differs from the battle without it`) && same;
    }
    call({op: 'release', id: ia});
    call({op: 'release', id: b2});
    res.tapeErrors = same ? 'ok' : 'failed';
    call({op: 'release', id});
  }
  out.targeted = res;
}

// ------------------------------------------------------------------------------------------- protocol
function protocol() {
  const r = rng(99);
  const spec = battleSpec(r);
  const lines = [JSON.stringify(Object.assign({op: 'new', id: 'x'}, spec))];
  lines.push(JSON.stringify({op: 'step', id: 'x', choices: {p1: 'move 0', p2: 'move 0'}}));
  lines.push(JSON.stringify({op: 'step', id: 'x', choices: {p1: 'move 9'}}));
  lines.push(JSON.stringify({op: 'release', id: 'x'}));
  lines.push('not json');
  const p = spawnSync(process.execPath, [path.join(__dirname, '..', 'oracle.js')], {input: lines.join('\n') + '\n', encoding: 'utf8'});
  const replies = p.stdout.trim().split('\n').map(l => JSON.parse(l));
  check(replies.length === 5, `protocol: ${replies.length} replies for 5 lines`);
  check(replies[0].ok && replies[0].state && replies[0].options, 'protocol: new');
  check(replies[1].ok || replies[1].error, 'protocol: step');
  check(replies[2].ok === false, 'protocol: a bad choice must fail');
  check(replies[3].ok === true, 'protocol: release');
  check(replies[4].ok === false, 'protocol: bad json');
  out.protocol = {replies: replies.length, errors: replies.filter(x => !x.ok).map(x => x.error)};
}

// -------------------------------------------------------------------------------------------
const t0 = Date.now();
fs.mkdirSync(RESULTS, {recursive: true});
for (const [name, fn] of [['targeted', targeted], ['protocol', protocol], ['roundTrip', roundTrip]]) {
  try { fn(); } catch (e) { fail(`${name} crashed: ${e.stack}`); }
}
out.failures = failures;
out.seconds = (Date.now() - t0) / 1000;
fs.writeFileSync(path.join(RESULTS, OUT), JSON.stringify(out, null, 1) + '\n');
const rt = out.roundTrip || {};
console.log(JSON.stringify({battles: rt.battles, steps: rt.steps, events: rt.events, mismatched: rt.mismatchedBattles,
  rawShare: rt.rawShare, recordStepsPerSecond: rt.recordStepsPerSecond, forceStepsPerSecond: rt.forceStepsPerSecond,
  eventsByKey: rt.eventsByKey, rawSites: rt.rawSites, targeted: out.targeted, failures: failures.length, seconds: out.seconds}, null, 1));
process.exit(failures.length ? 1 : 0);
