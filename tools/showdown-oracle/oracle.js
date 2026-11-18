// Showdown oracle for the test rig: implements tools/rig/CONTRACT.md (new, step, release) over JSON lines.
// Showdown is loaded from SHOWDOWN_DIR (a built checkout with dist/sim).
'use strict';
const path = require('path');
const readline = require('readline');

const SHOWDOWN_DIR = process.env.SHOWDOWN_DIR ||
  'C:/Users/user/Code/pokemon_champions/pcsa/.tools/vendor/pokemon-showdown';
const SIM = path.join(SHOWDOWN_DIR, 'dist', 'sim');
const {Battle} = require(path.join(SIM, 'battle'));
const {PRNG} = require(path.join(SIM, 'prng'));
const {Pokemon} = require(path.join(SIM, 'pokemon'));
const {Dex} = require(path.join(SIM, 'dex'));
const FORMAT = 'gen9championsbssregmc';
const DEX = Dex.forFormat(FORMAT);

class TapeError extends Error {}
class RigError extends Error {}

const toID = s => String(s || '').toLowerCase().replace(/[^a-z0-9]/g, '');

// ---------------------------------------------------------------------------------------------- codecs
// A codec maps the native result of one draw (bool for chance, index for sample, integer for random)
// to the contract value and back.
function chanceCodec(n, d) {
  const t = Math.min(d, Math.max(0, Math.ceil(n)));
  return {
    certain: t === 0 || t === d, fixed: t > 0,
    decode: r => !!r, encode: v => (typeof v === 'boolean' ? v : undefined),
    p: v => (v ? t / d : 1 - t / d),
  };
}
function rangeCodec(lo, hi) {
  return {
    certain: hi - lo <= 1, fixed: lo,
    decode: r => r, encode: v => (Number.isInteger(v) && v >= lo && v < hi ? v : undefined),
    p: () => 1 / (hi - lo),
  };
}
// random(d) compared as `roll < c`
function thresholdCodec(d, c) {
  const t = c === undefined ? d : Math.min(d, Math.max(0, Math.ceil(c)));
  return {
    certain: t === 0 || t === d, fixed: t > 0 ? 0 : d - 1,
    decode: r => r < t, encode: v => (v === true ? 0 : v === false ? d - 1 : undefined),
    p: v => (v ? t / d : 1 - t / d),
  };
}
// random(d) cut into buckets: values[i] for roll < cuts[i], the last value above every cut
function bucketCodec(d, cuts, values) {
  const lo = [0, ...cuts], hi = [...cuts, d];
  return {
    certain: false,
    decode: r => values[hi.findIndex(h => r < h)],
    encode: v => { const i = values.indexOf(v); return i < 0 ? undefined : lo[i]; },
    p: v => { const i = values.indexOf(v); return i < 0 ? 0 : (hi[i] - lo[i]) / d; },
  };
}
function sampleCodec(vals) {
  return {
    certain: vals.every(v => v === vals[0]), fixed: 0,
    decode: r => vals[r], encode: v => { const i = vals.indexOf(v); return i < 0 ? undefined : i; },
    p: v => vals.filter(x => x === v).length / vals.length,
  };
}

function sideOf(x) {
  if (!x) return null;
  if (x.side && x.side.id) return x.side.id;
  if (x.id === 'p1' || x.id === 'p2') return x.id;
  return null;
}

// ---------------------------------------------------------------------------------------------- controller
// Owns the draws of one battle. Modes: record (real seeded PRNG, events reported), force (answers from a
// list of events), replay (native results from an earlier run, to rebuild a battle after a failed step).
class Controller {
  constructor(seed) {
    this.real = new PRNG(Array.isArray(seed) ? seed.join(',') : seed);
    this.markers = [];
    this.mode = 'record';
    this.events = [];
    this.raws = [];
    this.replay = null;
    this.queues = null;
    this.battle = null;
    this.setupSide = null;
  }

  begin(force) {
    this.events = [];
    this.raws = [];
    if (force) {
      this.mode = 'force';
      this.queues = new Map();
      for (const ev of force) {
        const k = ev.key + '|' + (ev.side || null);
        if (!this.queues.has(k)) this.queues.set(k, []);
        this.queues.get(k).push(ev);
      }
    } else {
      this.mode = 'record';
      this.queues = null;
    }
  }

  finish() {
    if (this.mode === 'force') {
      const left = [];
      for (const q of this.queues.values()) left.push(...q);
      if (left.length) throw new TapeError(`unused events: ${JSON.stringify(left.slice(0, 3))}`);
    }
    return this.events;
  }

  realDraw(kind, args) {
    if (kind === 'chance') return this.real.randomChance(args[0], args[1]);
    if (kind === 'sample') return this.real.random(args[0].length);
    return this.real.random(args[0], args[1]);
  }

  // one draw: classify, then answer according to the mode
  draw(kind, args, site) {
    if (this.mode === 'replay') {
      if (this.replay.i >= this.replay.raws.length) throw new RigError('replay ran out of draws');
      const r = this.replay.raws[this.replay.i++];
      this.raws.push(r);
      return r;
    }
    if (!site) site = this.classify(kind, args);
    const c = site.codec;
    let r;
    if (this.mode === 'record') {
      r = this.realDraw(kind, args);
      if (!c.certain) {
        const value = c.decode(r);
        const p = c.p(value);
        this.events.push(Object.assign({}, site.ev, {value, p}));
      }
    } else if (c.certain) {
      r = c.fixed;
    } else {
      const k = site.ev.key + '|' + (site.ev.side || null);
      const q = this.queues.get(k);
      if (!q || !q.length) throw new TapeError(`missing event ${JSON.stringify(site.ev)}`);
      const ev = q.shift();
      for (const f of Object.keys(site.ev)) {
        if (f === 'key' || f === 'side' || !(f in ev)) continue;
        if (JSON.stringify(ev[f]) !== JSON.stringify(site.ev[f])) {
          throw new TapeError(`event ${JSON.stringify(ev)} does not match needed ${JSON.stringify(site.ev)}`);
        }
      }
      r = c.encode(ev.value);
      if (r === undefined) throw new TapeError(`impossible value in ${JSON.stringify(ev)}`);
      this.events.push(Object.assign({}, site.ev, {value: ev.value, p: c.p(ev.value)}));
    }
    this.raws.push(r);
    return r;
  }

  // ----- classification from the battle's state at the moment of the call
  classify(kind, args) {
    const b = this.battle;
    const top = this.markers[this.markers.length - 1];
    if (top && top.event === b.event) {
      const s = this.classifyMarker(top, kind, args);
      if (s) return s;
    }
    if (this.setupSide) {
      if (kind === 'sample') return {ev: {key: 'pick', side: this.setupSide, effect: 'gender'}, codec: sampleCodec(args[0])};
    }
    if (b.event && b.event.id && b.effect) {
      const s = this.classifyHandler(kind, args);
      if (s) return s;
    }
    return this.rawSite(kind, args);
  }

  moveFields() {
    const b = this.battle;
    const m = b.activeMove;
    return {side: sideOf(b.activePokemon), move: m ? m.id : '', hit: m && m.hit ? m.hit - 1 : 0};
  }

  classifyMarker(top, kind, args) {
    const b = this.battle;
    switch (top.fn) {
    case 'hitStepAccuracy':
      if (kind === 'chance') {
        const f = this.moveFields();
        return {ev: {key: 'accuracy', side: f.side, move: f.move, hit: 0}, codec: chanceCodec(args[0], args[1])};
      }
      break;
    case 'hitStepMoveHitLoop': {
      const f = this.moveFields();
      if (kind === 'chance') {
        return {ev: {key: 'accuracy', side: f.side, move: f.move, hit: f.hit}, codec: chanceCodec(args[0], args[1])};
      }
      if (kind === 'sample') return {ev: {key: 'hits', side: f.side, move: f.move}, codec: sampleCodec(args[0])};
      if (kind === 'random') {
        const multi = b.activeMove && b.activeMove.multihit;
        if (Array.isArray(multi) && args[0] === multi[0] && args[1] === multi[1] + 1) {
          return {ev: {key: 'hits', side: f.side, move: f.move}, codec: rangeCodec(args[0], args[1])};
        }
        // Loaded Dice: hits taken off the drawn number
        return {ev: {key: 'hits', side: f.side, move: f.move, effect: 'loadeddice'}, codec: rangeCodec(0, args[0])};
      }
      break;
    }
    case 'getDamage':
      if (kind === 'chance') {
        const f = this.moveFields();
        return {ev: {key: 'crit', side: f.side, move: f.move, hit: f.hit}, codec: chanceCodec(args[0], args[1])};
      }
      break;
    case 'randomizer':
      if (kind === 'random') {
        if (b.effect && b.effect.id === 'confusion') {
          return {ev: {key: 'roll', side: sideOf(b.effectState.target), move: 'confusion', hit: 0},
            codec: rangeCodec(0, args[0])};
        }
        const f = this.moveFields();
        return {ev: {key: 'roll', side: f.side, move: f.move, hit: f.hit}, codec: rangeCodec(0, args[0])};
      }
      break;
    case 'secondaries':
      if (kind === 'random' && top.list) {
        const sec = top.list[top.idx++];
        const moveData = top.args[3];
        const f = this.moveFields();
        const index = moveData.secondaries ? moveData.secondaries.indexOf(sec) : -1;
        return {ev: {key: 'secondary', side: f.side, move: f.move, hit: f.hit, index},
          codec: thresholdCodec(args[0], sec ? sec.chance : undefined)};
      }
      break;
    case 'selfDrops':
      if (kind === 'random') {
        const moveData = top.args[3];
        const f = this.moveFields();
        return {ev: {key: 'secondary', side: f.side, move: f.move, hit: f.hit, index: 'self'},
          codec: thresholdCodec(args[0], moveData.self ? moveData.self.chance : undefined)};
      }
      break;
    case 'addVolatile':
      if (kind === 'random' && top.status === 'partiallytrapped') {
        return {ev: {key: 'trapturns', side: sideOf(top.pokemon)}, codec: rangeCodec(args[0], args[1])};
      }
      break;
    case 'insertChoice':
      // an action inserted mid-turn among actions of equal priority: before or after the one it ties with
      if (kind === 'random' && args[1] - args[0] === 2) {
        const c = Array.isArray(top.args[0]) ? top.args[0][0] : top.args[0];
        const mine = this.tieSide(c);
        const other = this.tieSide(b.queue.list[args[0]]);
        if (mine && other && mine !== other) {
          const s0 = args[0];
          return {ev: {key: 'speedtie', side: null, effect: 'insert'}, codec: {
            certain: false, decode: x => (x === s0 ? mine : other),
            encode: v => (v === mine ? s0 : v === other ? s0 + 1 : undefined), p: () => 0.5}};
        }
      }
      break;
    case 'dragIn':
      if (kind === 'sample') {
        // the move that forces the switch (Roar, Dragon Tail...), or Red Card when the attacker is the one dragged
        const dragged = top.args[0].id;
        let effect = (b.event && b.event.id && b.effect && b.effect.id) || (b.activeMove ? b.activeMove.id : 'dragin');
        if (b.activeMove && dragged === sideOf(b.activePokemon)) effect = 'redcard';
        return {ev: {key: 'pick', side: top.args[0].id, effect}, codec: sampleCodec(args[0].map(p => this.slotOf(p)))};
      }
      break;
    }
    return null;
  }

  classifyHandler(kind, args) {
    const b = this.battle;
    const eff = b.effect;
    const id = eff.id || (b.activeMove ? b.activeMove.id : '');
    const ev = b.event.id;
    const isMove = eff.effectType === 'Move' || !eff.effectType;
    const holder = isMove ? sideOf(b.activePokemon) : (sideOf(b.effectState && b.effectState.target) || sideOf(b.event.target));
    if (kind === 'chance') {
      const codec = chanceCodec(args[0], args[1]);
      if (id === 'par' && ev === 'BeforeMove') return {ev: {key: 'fullpara', side: holder}, codec};
      if (id === 'frz' && ev === 'BeforeMove') return {ev: {key: 'thaw', side: holder}, codec};
      if (id === 'confusion' && ev === 'BeforeMove') return {ev: {key: 'confusionhit', side: holder}, codec};
      if (id === 'stall' && ev === 'StallMove') return {ev: {key: 'protect', side: holder}, codec};
      return {ev: {key: 'proc', side: holder, effect: id}, codec};
    }
    if (kind === 'sample') {
      if (id === 'slp' && ev === 'Start') return {ev: {key: 'sleepturns', side: holder}, codec: sampleCodec(args[0])};
      return {ev: {key: 'pick', side: holder, effect: id}, codec: sampleCodec(args[0].map((x, i) => this.itemValue(x, i)))};
    }
    if (kind === 'random' && args[0] !== undefined) {
      const [lo, hi] = args[1] === undefined ? [0, Math.floor(args[0])] : [Math.floor(args[0]), Math.floor(args[1])];
      const codec = rangeCodec(lo, hi);
      if (id === 'slp' && ev === 'Start') return {ev: {key: 'sleepturns', side: holder}, codec};
      if (id === 'confusion' && ev === 'Start') return {ev: {key: 'confusionturns', side: holder}, codec};
      if (id === 'lockedmove' && ev === 'Start') return {ev: {key: 'lockturns', side: holder}, codec};
      if (id === 'effectspore' && hi === 100 && lo === 0) {
        return {ev: {key: 'pick', side: holder, effect: id}, codec: bucketCodec(100, [11, 21, 30], ['slp', 'par', 'psn', ''])};
      }
      return {ev: {key: 'pick', side: holder, effect: id}, codec};
    }
    return null;
  }

  rawSite(kind, args) {
    const b = this.battle;
    const effect = (b.effect && b.effect.id) || (b.activeMove ? b.activeMove.id : '');
    const side = sideOf(b.activePokemon);
    let codec, jargs;
    if (kind === 'chance') { codec = chanceCodec(args[0], args[1]); jargs = args; }
    else if (kind === 'sample') {
      const vals = args[0].map((x, i) => this.itemValue(x, i));
      codec = rangeCodec(0, vals.length); jargs = [vals];
    } else if (args[0] === undefined) {
      codec = {certain: false, decode: r => r, encode: v => (typeof v === 'number' && v >= 0 && v < 1 ? v : undefined), p: () => null};
      jargs = [];
    } else {
      const [lo, hi] = args[1] === undefined ? [0, Math.floor(args[0])] : [Math.floor(args[0]), Math.floor(args[1])];
      codec = rangeCodec(lo, hi); jargs = args.filter(a => a !== undefined);
    }
    return {ev: {key: 'raw', side, effect, kind, args: jargs}, codec};
  }

  itemValue(x, i) {
    if (typeof x === 'string' || typeof x === 'number' || typeof x === 'boolean') return x;
    if (x instanceof Pokemon) return this.slotOf(x);
    if (x && typeof x.id === 'string') return x.id;
    return i;
  }

  slotOf(p) {
    const names = this.slotNames && this.slotNames[p.side.id];
    return names ? names.indexOf(p.name) : p.position;
  }

  // Fisher-Yates over list[start:end], one draw per position, like PRNG.shuffle. A tie of two is a
  // speedtie: between sides, the value is the side that goes first; within one side (two effects of the
  // same holder), the value is the id of the effect that goes first.
  shuffle(items, start = 0, end = items.length) {
    const tie = this.mode !== 'replay' && end - start === 2 ? this.tieSite(items[start], items[start + 1], start) : null;
    while (start < end - 1) {
      let r;
      if (this.mode === 'replay') {
        r = this.draw('random', [start, end]);
      } else if (tie) {
        r = this.draw('random', [start, end], tie);
      } else {
        const site = this.rawSite('random', [start, end]);
        site.ev.kind = 'shuffle';
        site.ev.effect = this.tieWhat(items[start], items[start + 1]) || site.ev.effect;
        r = this.draw('random', [start, end], site);
      }
      if (r !== start) [items[start], items[r]] = [items[r], items[start]];
      start++;
    }
  }

  tieSite(a, b, s0) {
    const sa = this.tieSide(a), sb = this.tieSide(b);
    let va, vb, side = null;
    if (sa && sb && sa !== sb) {
      va = sa; vb = sb;
    } else {
      va = a.effect && a.effect.id; vb = b.effect && b.effect.id;
      if (!va || !vb || va === vb) return null;
      side = sa || sb || null;
    }
    const codec = {
      certain: false,
      decode: x => (x === s0 ? va : vb), encode: v => (v === va ? s0 : v === vb ? s0 + 1 : undefined), p: () => 0.5,
    };
    const ev = {key: 'speedtie', side};
    const what = this.tieWhat(a, b);
    if (what) ev.effect = what;
    return {ev, codec};
  }

  tieSide(x) {
    if (!x) return null;
    if (x.choice) return sideOf(x.pokemon) || sideOf(x.side);
    if (x instanceof Pokemon) return sideOf(x);
    return sideOf(x.effectHolder);
  }

  // what is being ordered: nothing for actions, "pokemon" for Pokemon, the handlers' effect ids otherwise
  tieWhat(a, b) {
    if (a.choice) return null;
    if (a instanceof Pokemon) return 'pokemon';
    return [a, b].map(h => (h.effect && h.effect.id) || '?').sort().join('+');
  }
}

// PRNG facade installed as battle.prng
class ControlledPRNG {
  constructor(ctl) { this.ctl = ctl; this.startingSeed = ctl.real.startingSeed; }
  getSeed() { return this.ctl.real.getSeed(); }
  setSeed() { throw new RigError('setSeed on controlled PRNG'); }
  clone() { throw new RigError('clone of controlled PRNG'); }
  random(m, n) { return this.ctl.draw('random', [m, n]); }
  randomChance(n, d) { return this.ctl.draw('chance', [n, d]); }
  sample(items) {
    if (!items.length) throw new RangeError('Cannot sample an empty array');
    return items[this.ctl.draw('sample', [items])];
  }
  shuffle(items, start, end) { this.ctl.shuffle(items, start, end); }
}

// ---------------------------------------------------------------------------------------------- markers
// Calls that draw directly (not through an event handler) are told apart by which of these functions is
// running; a marker remembers battle.event at entry so draws in nested handlers are not mistaken for it.
const CTL = new WeakMap();

function marker(ctl, obj, name, extra) {
  const orig = obj[name];
  obj[name] = function (...args) {
    const m = {fn: name, event: ctl.battle.event, args};
    if (extra) extra(m, args);
    ctl.markers.push(m);
    try { return orig.apply(this, args); } finally { ctl.markers.pop(); }
  };
}

let protoPatched = false;
function patchPrototypes() {
  if (protoPatched) return;
  protoPatched = true;
  const orig = Pokemon.prototype.addVolatile;
  Pokemon.prototype.addVolatile = function (status, ...rest) {
    const ctl = CTL.get(this.battle);
    if (!ctl) return orig.call(this, status, ...rest);
    const id = typeof status === 'string' ? toID(status) : status && status.id;
    ctl.markers.push({fn: 'addVolatile', event: this.battle.event, status: id, pokemon: this});
    try { return orig.call(this, status, ...rest); } finally { ctl.markers.pop(); }
  };
}

function instrument(battle, ctl) {
  patchPrototypes();
  ctl.battle = battle;
  CTL.set(battle, ctl);
  battle.prng = new ControlledPRNG(ctl);
  const a = battle.actions;
  for (const fn of ['hitStepAccuracy', 'hitStepMoveHitLoop', 'getDamage', 'secondaries', 'selfDrops', 'dragIn']) {
    marker(ctl, a, fn);
  }
  marker(ctl, battle, 'randomizer');
  marker(ctl, battle.queue, 'insertChoice');
  const runEvent = battle.runEvent;
  battle.runEvent = function (eventid, target, source, effect, relayVar, onEffect, fastExit) {
    const r = runEvent.call(this, eventid, target, source, effect, relayVar, onEffect, fastExit);
    if (eventid === 'ModifySecondaries') {
      const top = ctl.markers[ctl.markers.length - 1];
      if (top && top.fn === 'secondaries') { top.list = r; top.idx = 0; }
    }
    return r;
  };
}

// ---------------------------------------------------------------------------------------------- battles
const battles = new Map();

function build(spec, ctl) {
  const battle = new Battle({formatid: FORMAT, prng: new ControlledPRNG(ctl)});
  instrument(battle, ctl);
  for (const sid of ['p1', 'p2']) {
    ctl.setupSide = sid;
    try { battle.setPlayer(sid, {name: sid, team: spec.teams[sid].map(s => Object.assign({}, s))}); } finally { ctl.setupSide = null; }
  }
  if (battle.requestState !== 'teampreview') throw new RigError('no team preview');
  const picks = {};
  for (const sid of ['p1', 'p2']) {
    const pk = spec.picks[sid];
    if (!Array.isArray(pk) || pk.length !== 3 || new Set(pk).size !== 3 || pk.some(i => !(i >= 0 && i < 6))) {
      throw new RigError(`bad picks for ${sid}`);
    }
    picks[sid] = 'team ' + pk.map(i => i + 1).join('');
  }
  if (!battle.choose('p1', picks.p1)) throw new RigError('p1 picks rejected: ' + battle.sides[0].choice.error);
  if (!battle.choose('p2', picks.p2)) throw new RigError('p2 picks rejected: ' + battle.sides[1].choice.error);
  return battle;
}

function slotNamesOf(spec) {
  const out = {};
  for (const sid of ['p1', 'p2']) {
    const team = spec.teams[sid];
    out[sid] = spec.picks[sid].map(i => {
      const s = team[i];
      return s.name || DEX.species.get(s.species).baseSpecies;
    });
  }
  return out;
}

function opNew(req) {
  if (battles.has(req.id)) throw new RigError(`battle ${req.id} exists`);
  const spec = {seed: req.seed || [1, 2, 3, 4], teams: req.teams, picks: req.picks};
  for (const sid of ['p1', 'p2']) {
    if (!spec.teams || !Array.isArray(spec.teams[sid]) || spec.teams[sid].length !== 6) throw new RigError(`${sid} needs 6 sets`);
  }
  const ctl = new Controller(spec.seed);
  ctl.begin(req.force);
  const names = slotNamesOf(spec);
  ctl.slotNames = names;
  const battle = build(spec, ctl);
  const events = ctl.finish();
  const h = {battle, ctl, spec, names, history: [{raws: ctl.raws, seed: ctl.real.getSeed()}]};
  checkNames(h);
  battles.set(req.id, h);
  const log = takeLog(h);
  return {ok: true, state: snapshot(h), options: options(h), events, log};
}

function checkNames(h) {
  for (const sid of ['p1', 'p2']) {
    const side = h.battle.sides[sid === 'p1' ? 0 : 1];
    for (const n of h.names[sid]) {
      if (side.pokemon.filter(p => p.name === n).length !== 1) throw new RigError(`${sid}: cannot place ${n} in a slot`);
    }
  }
}

// rebuild a battle from its history (after a failed step)
function rebuild(h) {
  const ctl = new Controller(h.spec.seed);
  ctl.slotNames = h.names;
  ctl.mode = 'replay';
  ctl.replay = {raws: h.history[0].raws, i: 0};
  const battle = build(h.spec, ctl);
  for (let i = 1; i < h.history.length; i++) {
    const st = h.history[i];
    ctl.replay = {raws: st.raws, i: 0};
    for (const sid of ['p1', 'p2']) if (st.sd[sid]) battle.choose(sid, st.sd[sid]);
  }
  ctl.mode = 'record';
  ctl.replay = null;
  ctl.real = new PRNG(h.history[h.history.length - 1].seed);
  h.battle = battle;
  h.ctl = ctl;
}

function holder(id) {
  const h = battles.get(id);
  if (!h) throw new RigError(`no battle ${id}`);
  return h;
}

function sideObj(h, sid) { return h.battle.sides[sid === 'p1' ? 0 : 1]; }
function monAt(h, sid, slot) {
  const n = h.names[sid][slot];
  return sideObj(h, sid).pokemon.find(p => p.name === n);
}

// contract choice -> Showdown choice
function translate(h, sid, choice, opt) {
  const m = /^(move|switch)\s+(\S+)(\s+mega)?$/.exec(String(choice).trim());
  if (!m) throw new RigError(`${sid}: bad choice ${choice}`);
  const side = sideObj(h, sid);
  if (m[1] === 'switch') {
    if (m[3]) throw new RigError(`${sid}: bad choice ${choice}`);
    const slot = Number(m[2]);
    if (!opt.switches || !opt.switches.includes(slot)) throw new RigError(`${sid}: switch ${slot} not allowed`);
    return 'switch ' + (monAt(h, sid, slot).position + 1);
  }
  if (opt.kind !== 'turn') throw new RigError(`${sid}: must switch`);
  const mega = !!m[3];
  if (mega && !opt.mega) throw new RigError(`${sid}: cannot Mega Evolve`);
  const key = /^\d+$/.test(m[2]) ? Number(m[2]) : toID(m[2]);
  let k = -1;
  const reqMoves = side.activeRequest.active[0].moves;
  if (opt.moves.includes(key)) {
    const id = typeof key === 'number' ? side.active[0].moveSlots[key].id : key;
    k = reqMoves.findIndex(x => x.id === id);
  } else if (key === 0 && opt.moves.length === 1 && typeof opt.moves[0] === 'string') {
    k = 0;
  }
  // every move disabled, some hidden: the request lists the moves and any of them is Struggle
  if (k < 0 && key === 'struggle' && opt.moves.includes('struggle')) k = 0;
  if (k < 0) throw new RigError(`${sid}: move ${m[2]} not allowed`);
  return `move ${k + 1}` + (mega ? ' mega' : '');
}

function opStep(req) {
  const h = holder(req.id);
  if (h.battle.ended) throw new RigError('battle has ended');
  const opts = options(h);
  const sd = {};
  const choices = req.choices || {};
  // refused choices: Showdown rejects them and updates the request; no side's choice is played
  const refused = {};
  for (const sid of ['p1', 'p2']) {
    const c = choices[sid];
    if (c !== undefined && c !== null && opts[sid].kind === 'turn') {
      const why = refusal(h, sid, c, opts[sid]);
      if (why) refused[sid] = why;
    }
  }
  if (Object.keys(refused).length) {
    for (const sid of Object.keys(refused)) {
      if (h.battle.choose(sid, refusedChoice(h, sid, choices[sid]))) throw new RigError(`${sid}: Showdown accepted a hidden choice`);
    }
    for (const sid of ['p1', 'p2']) {
      if (!refused[sid] && sideObj(h, sid).choice && sideObj(h, sid).choice.actions.length) h.battle.undoChoice(sid);
    }
    return {ok: true, refused, events: [], state: snapshot(h), options: options(h), log: []};
  }
  for (const sid of ['p1', 'p2']) {
    const o = opts[sid];
    const c = choices[sid];
    if (o.kind === 'turn' || o.kind === 'replace') {
      if (c === undefined || c === null) throw new RigError(`${sid} must choose`);
      sd[sid] = translate(h, sid, c, o);
    } else if (c !== undefined && c !== null) {
      throw new RigError(`${sid} has no decision`);
    }
  }
  const ctl = h.ctl;
  ctl.begin(req.force);
  const logPos = h.battle.log.length;
  try {
    for (const sid of ['p1', 'p2']) {
      if (!sd[sid]) continue;
      if (!h.battle.choose(sid, sd[sid])) throw new RigError(`${sid}: Showdown rejected ${sd[sid]}: ${sideObj(h, sid).choice.error}`);
    }
    if (!h.battle.ended && !['p1', 'p2'].some(sid => needsDecision(sideObj(h, sid)))) {
      throw new RigError('no side has a decision after the step');
    }
    const events = ctl.finish();
    h.history.push({sd, raws: ctl.raws, seed: ctl.real.getSeed()});
    const log = takeLog(h, logPos);
    return {ok: true, events, state: snapshot(h), options: options(h), log};
  } catch (e) {
    rebuild(h);
    throw e;
  }
}

function needsDecision(side) {
  const r = side.activeRequest;
  return !!(r && !r.wait && (r.forceSwitch || r.active));
}

// protocol lines since logPos, the omniscient version of split lines
function takeLog(h, from = 0) {
  const lines = h.battle.log.slice(from);
  const out = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].startsWith('|split|')) { out.push(lines[i + 1]); i += 2; continue; }
    out.push(lines[i]);
  }
  return out;
}

// ---------------------------------------------------------------------------------------------- options
function options(h) {
  const out = {};
  for (const sid of ['p1', 'p2']) out[sid] = sideOptions(h, sid);
  return out;
}

function sideOptions(h, sid) {
  const b = h.battle;
  if (b.ended) return {kind: 'done'};
  const side = sideObj(h, sid);
  const req = side.activeRequest;
  if (!req || req.wait) return {kind: 'wait'};
  const bench = [];
  for (let s = 0; s < 3; s++) {
    const p = monAt(h, sid, s);
    if (p && !p.fainted && !p.isActive) bench.push(s);
  }
  if (req.forceSwitch) {
    // Revival Blessing: the "switch" picks a fainted Pokemon to revive
    if (side.slotConditions[0] && side.slotConditions[0].revivalblessing) {
      return {kind: 'replace', switches: [0, 1, 2].filter(s => monAt(h, sid, s).fainted)};
    }
    return {kind: 'replace', switches: bench};
  }
  const p = side.active[0];
  const ra = req.active[0];
  const slots = p.moveSlots;
  const moves = [];
  // offered by the request but refused when chosen (contract section 9): Imprison's hidden disables
  const hiddenMoves = [];
  const locked = !!p.getLockedMove();
  const parallel = ra.moves.length === slots.length && ra.moves.every((x, i) => x.id === slots[i].id);
  ra.moves.forEach((x, i) => {
    if (x.disabled) return;
    const j = parallel ? i : slots.findIndex(s => s.id === x.id);
    if (j >= 0 && !locked && slots[j].disabled === 'hidden') { hiddenMoves.push(j); return; }
    moves.push(j >= 0 ? j : x.id);
  });
  // every move disabled, some of them hidden: Showdown turns any move choice into Struggle
  if (!moves.length && hiddenMoves.length) { moves.push('struggle'); hiddenMoves.length = 0; }
  const out = {kind: 'turn', moves, mega: !!ra.canMegaEvo, switches: p.trapped ? [] : bench};
  // a trap by an ability not yet revealed: the request offers the switches
  const hiddenSwitches = p.trapped === 'hidden' && !ra.trapped ? bench : [];
  if (hiddenSwitches.length || hiddenMoves.length) out.hidden = {switches: hiddenSwitches, moves: hiddenMoves};
  return out;
}

// why the game refuses a choice its request offers, or null
function refusal(h, sid, choice, opt) {
  if (!opt.hidden) return null;
  const m = /^(move|switch)\s+(\S+)/.exec(String(choice).trim());
  if (!m) return null;
  const n = Number(m[2]);
  if (m[1] === 'switch' && opt.hidden.switches.includes(n)) return 'trapped';
  if (m[1] === 'move' && opt.hidden.moves.includes(n)) return 'disabled';
  return null;
}

// Showdown's choice for a refused choice (translate() only accepts allowed ones)
function refusedChoice(h, sid, choice) {
  const m = /^(move|switch)\s+(\S+)/.exec(String(choice).trim());
  const side = sideObj(h, sid);
  if (m[1] === 'switch') return 'switch ' + (monAt(h, sid, Number(m[2])).position + 1);
  const id = side.active[0].moveSlots[Number(m[2])].id;
  const k = side.activeRequest.active[0].moves.findIndex(x => x.id === id);
  return `move ${k + 1}`;
}

// ---------------------------------------------------------------------------------------------- snapshot
// volatiles that only print a message and never affect play
const BOOKKEEPING = new Set(['chillyreception']);
const HAZARDS = new Set(['stealthrock', 'spikes', 'toxicspikes', 'stickyweb']);
// effect-state fields that affect play; everything else in Showdown's state is bookkeeping
const KEEP = ['hp', 'move', 'layers', 'counter', 'trueDuration', 'bestStat', 'multiplier', 'hitCount',
  'contactHitCount', 'numConsecutive', 'boundDivisor', 'stage', 'typeWas', 'lastMove', 'sourceEffect'];

function cleanValue(v) {
  if (v === null || v === undefined) return undefined;
  if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') return v;
  if (v && typeof v.id === 'string') return v.id;
  return undefined;
}

function volatileData(id, st) {
  const out = {};
  if (id === 'confusion') { if (st.time) out.left = st.time; }
  else if (st.duration) out.left = st.duration;
  for (const k of KEEP) {
    if (!(k in st)) continue;
    const v = cleanValue(st[k]);
    if (v === undefined) continue;
    if (k === 'sourceEffect') { if (id === 'partiallytrapped') out.move = v; continue; }
    out[k] = v;
  }
  return out;
}

function durationOf(st) { return st && st.duration ? st.duration : null; }

function snapshot(h) {
  const b = h.battle;
  const f = b.field;
  const out = {
    turn: b.turn, ended: !!b.ended,
    winner: b.ended ? (b.winner === 'p1' || b.winner === 'p2' ? b.winner : 'tie') : null,
    weather: f.weather ? {id: f.weather, left: durationOf(f.weatherState)} : {id: '', left: 0},
    terrain: f.terrain ? {id: f.terrain, left: durationOf(f.terrainState)} : {id: '', left: 0},
    rooms: {}, sides: {},
  };
  for (const [id, st] of Object.entries(f.pseudoWeather)) out.rooms[id] = durationOf(st);
  for (const sid of ['p1', 'p2']) out.sides[sid] = sideSnap(h, sid);
  return out;
}

function sideSnap(h, sid) {
  const side = sideObj(h, sid);
  const mons = [0, 1, 2].map(s => monAt(h, sid, s));
  const active = side.active[0];
  const conditions = {};
  for (const [id, st] of Object.entries(side.sideConditions)) {
    conditions[id] = HAZARDS.has(id) ? (st.layers || 1) : (st.duration || st.layers || 1);
  }
  const out = {
    active: active ? mons.indexOf(active) : null,
    megaUsed: side.pokemon.some(p => p.canMegaEvo === false || p.species.isMega),
    conditions,
  };
  const slot = side.slotConditions[0] || {};
  for (const [id, st] of Object.entries(slot)) {
    // heal() truncates the stored half of max HP
    if (id === 'wish') out.wish = {left: wishLeft(h.battle, st), hp: Math.max(1, Math.trunc(st.hp))};
    else if (id === 'futuremove') out.futuresight = {left: st.duration, move: cleanValue(st.move) || ''};
    else conditions[id] = st.duration || 1;
  }
  out.pokemon = mons.map(p => monSnap(p));
  return out;
}

// The turn whose end-of-turn step runs next: after this turn's residual (a replacement after a faint at the
// end of the turn) it is the next turn.
function nextResidualTurn(b) {
  const pending = b.queue.list.some(a => a.choice === 'residual');
  return b.turn + (b.midTurn && !pending ? 1 : 0);
}

// Wish keeps the turn it was used on, not a duration: it heals at the first residual of a turn T with
// T - 1 > startingTurn.
function wishLeft(b, st) {
  if (typeof st.startingTurn !== 'number') return st.duration || null;
  return st.startingTurn + 3 - nextResidualTurn(b);
}

// Contract 7.10: a move is disabled when it cannot be chosen now, for any reason. Taken from the side's
// request when it has one. A side that waits while the other replaces has none, and its moveSlots flags
// are those of the last turn start, so the DisableMove pass the next turn starts with is run here and
// undone. A benched Pokémon keeps stale flags in Showdown: all false.
function disabledFlags(p) {
  if (!p.isActive || p.fainted) return p.moveSlots.map(() => false);
  const req = p.side.activeRequest;
  const live = req && !req.wait && req.active && req.active[0];
  const moves = live ? req.active[0].moves : previewMoves(p);
  // the request shows Imprison's hidden disables as usable; the move slot has the truth
  const locked = live && !!p.getLockedMove();
  return p.moveSlots.map(m => (live && !locked && m.disabled === 'hidden') || !moves.some(x => x.id === m.id && !x.disabled));
}

function previewMoves(p) {
  const b = p.battle;
  const saved = p.moveSlots.map(m => [m.disabled, m.disabledSource]);
  const vols = p.volatiles, md = p.maybeDisabled, ml = p.maybeLocked, logLen = b.log.length;
  const prng = b.prng;
  p.volatiles = Object.assign({}, vols);  // choicelock may remove itself
  // handler order among equals is shuffled; the order cannot change the flags, so draw nothing
  b.prng = {random: (m, n) => (n === undefined ? 0 : m), randomChance: () => false, sample: a => a[0], shuffle: () => {}};
  try {
    p.maybeDisabled = false;
    p.maybeLocked = false;
    for (const m of p.moveSlots) { m.disabled = false; m.disabledSource = ''; }
    b.runEvent('DisableMove', p);
    for (const m of p.moveSlots) {
      const am = b.dex.getActiveMove(m.id);
      b.singleEvent('DisableMove', am, null, p);
      if (am.flags['cantusetwice'] && p.lastMove && p.lastMove.id === m.id) p.disableMove(p.lastMove.id);
    }
    return p.getMoves(p.getLockedMove(), false);
  } finally {
    b.prng = prng;
    p.moveSlots.forEach((m, i) => { [m.disabled, m.disabledSource] = saved[i]; });
    p.volatiles = vols;
    p.maybeDisabled = md;
    p.maybeLocked = ml;
    b.log.length = logLen;
  }
}

function monSnap(p) {
  const active = p.isActive;
  const status = p.fainted || p.status === 'fnt' ? '' : (p.status || '');
  const statusData = {};
  if (status === 'slp' || status === 'frz') statusData.turns = p.statusState.time;
  // the stage restarts at 0 on switch-in, so a benched Pokémon's stage is bookkeeping
  if (status === 'tox') statusData.stage = active ? p.statusState.stage : 0;
  const boosts = {};
  for (const k of ['atk', 'def', 'spa', 'spd', 'spe', 'accuracy', 'evasion']) boosts[k] = active ? p.boosts[k] : 0;
  const volatiles = {};
  if (active) {
    for (const id of Object.keys(p.volatiles).sort()) {
      if (!BOOKKEEPING.has(id)) volatiles[id] = volatileData(id, p.volatiles[id]);
    }
  }
  const st = p.storedStats;
  const dis = disabledFlags(p);
  return {
    species: p.species.id, baseSpecies: toID(p.set.species),
    // '???' is the typeless placeholder (Double Shock, Burn Up), not a type
    types: p.getTypes().filter(t => t !== '???').map(toID), ability: p.ability, baseAbility: p.baseAbility,
    item: p.item || '', hp: p.hp, maxhp: p.maxhp, fainted: !!p.fainted,
    stats: {atk: st.atk, def: st.def, spa: st.spa, spd: st.spd, spe: st.spe},
    status, statusData,
    moves: p.moveSlots.map((m, i) => ({id: m.id, pp: m.pp, maxpp: m.maxpp, disabled: dis[i]})),
    boosts, volatiles,
    lastMove: active && p.lastMove ? p.lastMove.id : '',
    activeTurns: active ? p.activeTurns : 0,
  };
}

// ---------------------------------------------------------------------------------------------- protocol
function handle(req) {
  try {
    switch (req.op) {
    case 'new': return opNew(req);
    case 'step': return opStep(req);
    case 'release':
      if (!battles.delete(req.id)) throw new RigError(`no battle ${req.id}`);
      return {ok: true};
    default: return {ok: false, error: `unknown op ${req.op}`};
    }
  } catch (e) {
    if (e instanceof TapeError) return {ok: false, error: 'tape', detail: e.message};
    if (e instanceof RigError) return {ok: false, error: e.message};
    return {ok: false, error: `${e.name}: ${e.message}`, stack: String(e.stack).split('\n').slice(0, 6).join(' | ')};
  }
}

module.exports = {handle, battles, FORMAT, SHOWDOWN_DIR, SIM};

if (require.main === module) {
  const rl = readline.createInterface({input: process.stdin, crlfDelay: Infinity});
  rl.on('line', line => {
    if (!line.trim()) return;
    let req;
    try { req = JSON.parse(line); } catch (e) { process.stdout.write(JSON.stringify({ok: false, error: 'bad json'}) + '\n'); return; }
    process.stdout.write(JSON.stringify(handle(req)) + '\n');
  });
  rl.on('close', () => process.exit(0));
}
