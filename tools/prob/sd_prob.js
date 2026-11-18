// Showdown's measured frequencies for the situations of bench/src/prob_check.rs: each trial is a new battle
// with its own sodium seed, played with Showdown's own PRNG; outcomes are read from the protocol log.
// Usage: node sd_prob.js OUT.jsonl TRIALS [name,name,...]
'use strict';
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const SD = process.env.SHOWDOWN_DIR || 'C:/Users/user/Code/pokemon_champions/pcsa/.tools/vendor/pokemon-showdown';
const {Battle} = require(path.join(SD, 'dist', 'sim', 'battle'));
const FORMAT = 'gen9championsbssregmc';

const set = (species, ability, item, moves, gender = 'M') =>
  ({species, ability, item, moves, nature: 'Serious', evs: {hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0}, level: 50, gender});
const SDANCE = ['swordsdance'];
function team(lead, side) {
  const b = side === 0 ? [set('skarmory', 'sturdy', '', SDANCE), set('dragonite', 'innerfocus', '', SDANCE)]
    : [set('corviknight', 'pressure', '', SDANCE), set('umbreon', 'synchronize', '', SDANCE)];
  return [lead, ...b, set('slowbro', 'owntempo', '', SDANCE), set('toxapex', 'regenerator', '', SDANCE), set('clefable', 'magicguard', '', SDANCE)];
}
const P1 = set('garchomp', 'roughskin', '', ['dragonclaw']);
const P2 = set('snorlax', 'thickfat', '', SDANCE);
const n = (k) => Array(k).fill([1, 1]);
const after = (k) => [[1, 1], ...Array(k - 1).fill([2, 1])];

// helpers on the protocol lines of the trial
const has = (L, re) => L.some(l => re.test(l));
const count = (L, re) => L.filter(l => re.test(l)).length;
const miss1 = L => has(L, /^\|-miss\|p1a/);
const acc = L => (miss1(L) ? 'miss' : 'hit');
const hits = L => { const l = L.find(x => x.startsWith('|-hitcount|')); return l ? l.split('|')[3] : null; };
const statusOf = (L, side) => { const l = L.find(x => x.startsWith(`|-status|${side}a`)); return l ? l.split('|')[3] : 'none'; };
const flinch2 = L => (has(L, /^\|cant\|p2a[^|]*\|flinch/) ? 'flinch' : 'none');
const firstSide = (L, re) => { const l = L.find(x => re.test(x)); return l ? l.split('|')[2].slice(0, 2) : 'none'; };
const SLOT2 = {Corviknight: '1', Umbreon: '2'};
const SLOT1 = {Skarmory: '1', Dragonite: '2'};
const dragged = (L, side, map) => { const l = L.find(x => x.startsWith(`|drag|${side}a: `)); return l ? map[l.split('|')[2].split(': ')[1]] : 'none'; };

const S = [];
const sc = (name, o) => S.push(Object.assign({name, p1: P1, p2: P2, turns: n(1), setup: () => {}}, o));
const fb = (ab, it) => set('garchomp', ab, it, ['focusblast']);
sc('acc70', {p1: fb('roughskin', ''), cls: acc});
sc('acc70_accm1', {p1: fb('roughskin', ''), setup: b => { b.p1.active[0].boosts.accuracy = -1; }, cls: acc});
sc('acc70_evap6', {p1: fb('roughskin', ''), setup: b => { b.p2.active[0].boosts.evasion = 6; }, cls: acc});
sc('acc_widelens', {p1: fb('roughskin', 'widelens'), cls: acc});
sc('acc_sheercold', {p1: set('garchomp', 'roughskin', '', ['sheercold']), cls: acc});
sc('acc_thunder_sun', {p1: set('garchomp', 'drought', '', ['thunder']), cls: acc});
sc('tripleaxel', {p1: set('garchomp', 'roughskin', '', ['tripleaxel']), p2: set('skarmory', 'sturdy', '', SDANCE), cls: L => (miss1(L) ? '0' : hits(L) || '1?')});
sc('popbomb', {p1: set('garchomp', 'roughskin', '', ['populationbomb']), p2: set('skarmory', 'sturdy', '', SDANCE), cls: L => (miss1(L) ? '0' : hits(L) || '1?')});
sc('crit_r1', {cls: L => (has(L, /^\|-crit\|/) ? 'crit' : 'no')});
sc('crit_r2', {p1: set('garchomp', 'roughskin', '', ['nightslash']), cls: L => (has(L, /^\|-crit\|/) ? 'crit' : 'no')});
sc('crit_r3', {p1: set('garchomp', 'roughskin', 'scopelens', ['nightslash']), cls: L => (has(L, /^\|-crit\|/) ? 'crit' : 'no')});
sc('hits_2to5', {p1: set('garchomp', 'roughskin', '', ['bulletseed']), p2: set('skarmory', 'sturdy', '', SDANCE), cls: L => hits(L) || 'none'});
sc('sec_bodyslam', {p1: set('garchomp', 'roughskin', '', ['bodyslam']), cls: L => (statusOf(L, 'p2') === 'par' ? 'par' : 'none')});
sc('sec_bodyslam_sg', {p1: set('garchomp', 'serenegrace', '', ['bodyslam']), cls: L => (statusOf(L, 'p2') === 'par' ? 'par' : 'none')});
sc('sec_airslash_sg', {p1: set('garchomp', 'serenegrace', '', ['airslash']), cls: flinch2});
sc('sec_kingsrock_sg', {p1: set('garchomp', 'serenegrace', 'kingsrock', ['earthquake']), cls: flinch2});
sc('triattack', {p1: set('garchomp', 'roughskin', '', ['triattack']), cls: L => statusOf(L, 'p2')});
sc('direclaw', {p1: set('garchomp', 'roughskin', '', ['direclaw']), cls: L => statusOf(L, 'p2')});
const gsd = set('garchomp', 'roughskin', '', SDANCE);
sc('speedtie_move', {p1: gsd, p2: gsd, cls: L => firstSide(L, /^\|move\|/)});
sc('speedtie_entry', {p1: set('garchomp', 'intimidate', '', SDANCE), p2: set('garchomp', 'intimidate', '', SDANCE), turns: [], all: true,
  cls: L => firstSide(L, /^\|-ability\|p[12]a: [^|]*\|Intimidate/)});
sc('speedtie_residual', {p1: set('garchomp', 'roughskin', 'leftovers', SDANCE), p2: set('garchomp', 'roughskin', 'leftovers', SDANCE),
  setup: b => { b.p1.active[0].hp -= 50; b.p2.active[0].hp -= 50; }, cls: L => firstSide(L, /^\|-heal\|.*\[from\] item: Leftovers/)});
sc('sleep_turns', {p1: set('garchomp', 'roughskin', '', ['spore', 'swordsdance']), turns: after(5), cls: L => String(count(L, /^\|cant\|p2a[^|]*\|slp/))});
sc('confusion_turns', {p1: set('garchomp', 'roughskin', '', ['confuseray', 'swordsdance']), turns: after(7), cls: L => String(count(L, /^\|-activate\|p2a[^|]*\|confusion/))});
sc('confusion_hit', {p1: set('garchomp', 'roughskin', '', ['confuseray']), cls: L => (has(L, /^\|-damage\|p2a.*\[from\] confusion/) ? 'hit' : 'none')});
sc('par_full', {setup: b => { b.p2.active[0].setStatus('par', b.p2.active[0], null, true); }, cls: L => (has(L, /^\|cant\|p2a[^|]*\|par/) ? 'cant' : 'moves')});
sc('frz_thaw', {p1: set('garchomp', 'roughskin', '', SDANCE), turns: n(4), setup: b => { b.p2.active[0].setStatus('frz', b.p2.active[0], null, true); },
  cls: L => String(count(L, /^\|cant\|p2a[^|]*\|frz/))});
sc('protect_chain', {p1: set('snorlax', 'thickfat', '', SDANCE), p2: set('garchomp', 'roughskin', '', ['protect']), turns: n(3), byTurn: true,
  cls: T => [1, 2].map(i => (has(T[i] || [], /^\|-fail\|p2a/) ? 'F' : 'S')).join('')});
sc('lock_turns', {p1: set('umbreon', 'synchronize', '', ['outrage']), p2: set('skarmory', 'sturdy', '', SDANCE), turns: n(4), byTurn: true,
  cls: T => { const i = T.findIndex(L => has(L, /^\|-start\|p1a[^|]*\|confusion/)); return i < 0 ? 'none' : String(i + 1); }});
sc('trap_turns', {p1: set('garchomp', 'roughskin', '', ['infestation', 'calmmind']), turns: after(8), cls: L => String(count(L, /^\|-damage\|p2a.*\[partiallytrapped\]/))});
sc('drag_roar', {p1: set('garchomp', 'roughskin', '', ['roar']), cls: L => dragged(L, 'p2', SLOT2)});
sc('drag_redcard', {p2: set('snorlax', 'thickfat', 'redcard', SDANCE), cls: L => dragged(L, 'p1', SLOT1)});
const st1 = L => statusOf(L, 'p1');
sc('ab_static', {p2: set('snorlax', 'static', '', SDANCE), cls: st1});
sc('ab_effectspore', {p2: set('snorlax', 'effectspore', '', SDANCE), cls: st1});
sc('ab_cursedbody', {p2: set('snorlax', 'cursedbody', '', SDANCE), cls: L => (has(L, /^\|-start\|p1a[^|]*\|Disable/) ? 'disable' : 'none')});
sc('ab_quickdraw', {p1: set('snorlax', 'quickdraw', '', ['bodyslam']), p2: set('garchomp', 'roughskin', '', SDANCE), cls: L => (has(L, /ability: Quick Draw/) ? 'proc' : 'none')});
sc('ab_shedskin', {p1: gsd, p2: set('snorlax', 'shedskin', '', SDANCE), setup: b => { b.p2.active[0].setStatus('brn', b.p2.active[0], null, true); },
  cls: L => (has(L, /^\|-curestatus\|p2a/) ? 'cure' : 'none')});
sc('ab_moody', {p1: gsd, p2: set('snorlax', 'moody', '', SDANCE), cls: L => {
  const at = L.findIndex(l => /^\|-ability\|p2a[^|]*\|Moody/.test(l));
  const R = at < 0 ? [] : L.slice(at + 1);
  const up = R.find(l => /^\|-boost\|p2a[^|]*\|[a-z]+\|2$/.test(l));
  const dn = R.find(l => /^\|-unboost\|p2a[^|]*\|[a-z]+\|1$/.test(l));
  return `${up ? up.split('|')[3] : '-'}+${dn ? dn.split('|')[3] : '-'}-`;
}});
sc('attract_cant', {p1: set('garchomp', 'roughskin', '', ['attract']), p2: set('snorlax', 'thickfat', '', SDANCE, 'F'), cls: L => (has(L, /^\|cant\|p2a[^|]*\|Attract/) ? 'cant' : 'moves')});
sc('item_quickclaw', {p1: set('snorlax', 'thickfat', 'quickclaw', ['bodyslam']), p2: set('garchomp', 'roughskin', '', SDANCE), cls: L => (has(L, /item: Quick Claw/) ? 'proc' : 'none')});
sc('item_focusband', {p2: set('snorlax', 'thickfat', 'focusband', SDANCE), setup: b => { b.p2.active[0].hp = 1; }, cls: L => (has(L, /item: Focus Band/) ? 'proc' : 'none')});
sc('move_sleeptalk', {p1: gsd, p2: set('snorlax', 'thickfat', '', ['sleeptalk', 'bodyslam', 'swordsdance', 'curse']),
  setup: b => { const p = b.p2.active[0]; p.setStatus('slp', p, null, true); p.statusState.time = 3; p.statusState.startTime = 3; },
  cls: L => { const l = L.find(x => /^\|move\|p2a[^|]*\|[^|]*\|.*\[from\]/.test(x)); return l ? l.split('|')[3].toLowerCase().replace(/[^a-z]/g, '') : 'none'; }});

function seedFor(name, i) {
  return 'sodium,' + crypto.createHash('sha256').update(`${name}:${i}`).digest('hex');
}

function choose(b, sid, k) {
  const side = sid === 'p1' ? b.p1 : b.p2;
  const r = side.activeRequest;
  if (!r || r.wait) return;
  if (r.forceSwitch || !b.choose(sid, `move ${k}`)) {
    if (side.choice && side.choice.actions.length) b.undoChoice(sid);
    b.choose(sid, 'default');
  }
}

// protocol lines (the omniscient side of split lines)
function lines(log, from) {
  const out = [];
  for (let i = from; i < log.length; i++) {
    if (log[i].startsWith('|split|')) { out.push(log[i + 1]); i += 2; continue; }
    out.push(log[i]);
  }
  return out;
}

function trial(s, i) {
  const b = new Battle({formatid: FORMAT, seed: seedFor(s.name, i)});
  b.setPlayer('p1', {name: 'p1', team: team(s.p1, 0).map(x => Object.assign({}, x))});
  b.setPlayer('p2', {name: 'p2', team: team(s.p2, 1).map(x => Object.assign({}, x))});
  const from0 = b.log.length;
  b.choose('p1', 'team 123');
  b.choose('p2', 'team 123');
  s.setup(b);
  const from = s.all ? from0 : b.log.length;
  const per = [];
  for (const [m1, m2] of s.turns) {
    if (b.ended) break;
    const at = b.log.length;
    choose(b, 'p1', m1);
    choose(b, 'p2', m2);
    per.push(lines(b.log, at));
  }
  return s.byTurn ? s.cls(per) : s.cls(lines(b.log, from));
}

const [out, trialsArg, onlyArg] = process.argv.slice(2);
const trials = Number(trialsArg || 20000);
const only = onlyArg ? onlyArg.split(',') : null;
for (const s of S) {
  if (only && !only.includes(s.name)) continue;
  const t0 = Date.now();
  const counts = {};
  for (let i = 0; i < trials; i++) {
    const l = trial(s, i);
    counts[l] = (counts[l] || 0) + 1;
  }
  const row = {name: s.name, trials, counts, wall_s: (Date.now() - t0) / 1000, showdown: 'c0845f33d', format: FORMAT};
  fs.appendFileSync(out, JSON.stringify(row) + '\n');
  console.log(JSON.stringify(row));
}
