// Generates engine/src/sim/data/generated.rs (the new resolver's static tables) and tools/rig/data/plain_moves.json
// from Showdown's Champions mod. Read-only on Showdown. Deterministic: entries sorted by id.
// usage: node tools/data/gen_sim_data.js            (Showdown from $SHOWDOWN_DIR, default below)
'use strict';
const path = require('path');
const fs = require('fs');
const {execFileSync} = require('child_process');

const SD = process.env.SHOWDOWN_DIR ||
  'C:\\Users\\user\\Code\\pokemon_champions\\pcsa\\.tools\\vendor\\pokemon-showdown';
const REPO = path.join(__dirname, '..', '..');
const OUT_RS = path.join(REPO, 'engine', 'src', 'sim', 'data', 'generated.rs');
const OUT_PLAIN = path.join(REPO, 'tools', 'rig', 'data', 'plain_moves.json');
const OUT_STAGE2 = path.join(REPO, 'tools', 'rig', 'data', 'stage2_moves.json');
const OUT_STAGE3 = path.join(REPO, 'tools', 'rig', 'data', 'stage3_moves.json');
const FORMAT = 'gen9championsbssregmc';

const {Dex, Battle} = require(path.join(SD, 'dist', 'sim'));
const dex = Dex.mod('champions');
const toID = (s) => ('' + (s || '')).toLowerCase().replace(/[^a-z0-9]+/g, '');
const commit = execFileSync('git', ['-C', SD, 'rev-parse', 'HEAD'], {encoding: 'utf8'}).trim();

const lit = (s) => JSON.stringify(String(s));
const upper = (id) => (/^[0-9]/.test(id) ? '_' : '') + id.toUpperCase();
const byId = (a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);

// ------------------------------------------------------------------------------------------------ enums
const TYPES = ['bug', 'dark', 'dragon', 'electric', 'fairy', 'fighting', 'fire', 'flying', 'ghost', 'grass',
  'ground', 'ice', 'normal', 'poison', 'psychic', 'rock', 'steel', 'water'];
const TYPE_VARIANTS = TYPES.map(t => t[0].toUpperCase() + t.slice(1)).concat(['Stellar', 'Typeless']);
function typeOf(name) {
  const id = toID(name);
  if (id === 'stellar') return 'Type::Stellar';
  if (!id || name === '???') return 'Type::Typeless';
  const i = TYPES.indexOf(id);
  if (i < 0) throw new Error('unknown type ' + name);
  return 'Type::' + TYPE_VARIANTS[i];
}
const TARGETS = ['normal', 'self', 'allAdjacent', 'adjacentAlly', 'allies', 'allAdjacentFoes', 'adjacentAllyOrSelf',
  'all', 'any', 'allySide', 'scripted', 'allyTeam', 'randomNormal', 'foeSide', 'adjacentFoe'];
const targetVariant = (t) => {
  if (!TARGETS.includes(t)) throw new Error('unknown target ' + t);
  return 'Target::' + (t === 'self' ? 'Self_' : t[0].toUpperCase() + t.slice(1));
};
const STATUSES = {'': 'Status::None', brn: 'Status::Brn', par: 'Status::Par', slp: 'Status::Slp', frz: 'Status::Frz',
  psn: 'Status::Psn', tox: 'Status::Tox'};
const statusOf = (s) => { const v = STATUSES[toID(s)]; if (!v) throw new Error('status ' + s); return v; };
const BOOSTS = ['atk', 'def', 'spa', 'spd', 'spe', 'accuracy', 'evasion'];
function boostsOf(b) {
  if (!b) return 'None';
  for (const k of Object.keys(b)) if (!BOOSTS.includes(k)) throw new Error('boost ' + k);
  return `Some([${BOOSTS.map(k => b[k] || 0).join(', ')}])`;
}
const STATS = {atk: 'Stat::Atk', def: 'Stat::Def', spa: 'Stat::Spa', spd: 'Stat::Spd', spe: 'Stat::Spe'};

// handler names on an effect object: `on*` callbacks (functions or constants, not their Order/Priority numbers)
// and the named callbacks
const CALLBACKS = ['basePowerCallback', 'damageCallback', 'beforeMoveCallback', 'beforeTurnCallback',
  'priorityChargeCallback', 'durationCallback'];
// A key ending in Priority, Order or SubOrder is an ordering number, except the handlers of the ModifyPriority
// and FractionalPriority events (a function, or Lagging Tail's constant).
const PRIORITY_EVENTS = /^on(Ally|Foe|Source|Any)?(ModifyPriority|FractionalPriority)$/;
function handlersOf(obj) {
  if (!obj) return [];
  const out = [];
  for (const k of Object.keys(obj)) {
    if (obj[k] === undefined) continue;
    if (CALLBACKS.includes(k) || (/^on[A-Z]/.test(k) && (!/(Priority|Order|SubOrder)$/.test(k) || PRIORITY_EVENTS.test(k)))) {
      out.push(k);
    }
  }
  return out.sort();
}
const strList = (a) => `&[${a.map(lit).join(', ')}]`;
// the ordering numbers of an effect's handlers (onResidualOrder, onBasePowerPriority, ...)
function numsOf(obj) {
  if (!obj) return '&[]';
  const out = [];
  for (const k of Object.keys(obj).sort()) {
    if (/^on[A-Z].*(Priority|Order|SubOrder)$/.test(k) && typeof obj[k] === 'number') {
      const v = obj[k];
      out.push(`(${lit(k)}, ${Number.isInteger(v) ? v.toFixed(1) : v})`);
    }
  }
  return `&[${out.join(', ')}]`;
}

// ------------------------------------------------------------------------------------------------ species
const species = dex.species.all().filter(s => !s.isNonstandard).slice().sort(byId);
const speciesIdx = new Map(species.map((s, i) => [s.id, i]));

// ------------------------------------------------------------------------------------------------ items
const items = [{id: '', name: '', fake: true}].concat(dex.items.all().filter(x => !x.isNonstandard).slice().sort(byId));
const itemIdx = new Map(items.map((x, i) => [x.id, i]));

// ------------------------------------------------------------------------------------------------ abilities
const abilityList = dex.abilities.all().filter(x => !x.isNonstandard || x.id === 'noability').slice().sort(byId);
const abilities = [{id: '', name: '', fake: true}].concat(abilityList);
const abilityIdx = new Map(abilities.map((x, i) => [x.id, i]));

// ------------------------------------------------------------------------------------------------ moves
const moves = [{id: '', name: '', fake: true}].concat(
  dex.moves.all().filter(m => !m.isNonstandard || m.id === 'struggle').slice().sort(byId));
const moveIdx = new Map(moves.map((m, i) => [m.id, i]));

// ------------------------------------------------------------------------------------------------ conditions
// Every condition id the legal data can create: the conditions data, the conditions of moves, items and
// abilities, and every id a move names as a volatile, side, slot, field, weather or terrain effect.
const condKinds = new Map();
function addCond(id, kind) {
  id = toID(id);
  if (!id) return;
  if (!condKinds.has(id)) condKinds.set(id, new Set());
  if (kind) condKinds.get(id).add(kind);
}
for (const id of Object.keys(dex.data.Conditions)) {
  const c = dex.conditions.get(id);
  addCond(id, c.effectType === 'Weather' ? 'weather' : c.effectType === 'Status' ? 'status' : null);
}
for (const m of moves) {
  if (m.fake) continue;
  if (m.condition) addCond(m.id, null);
  const effects = [m, ...(m.secondaries || []), m.self || {}, ...(m.secondaries || []).map(s => s.self || {})];
  for (const e of effects) {
    if (e.volatileStatus) addCond(e.volatileStatus, 'volatile');
    if (e.sideCondition) addCond(e.sideCondition, 'side');
    if (e.slotCondition) addCond(e.slotCondition, 'slot');
    if (e.pseudoWeather) addCond(e.pseudoWeather, 'field');
    if (e.weather) addCond(e.weather, 'weather');
    if (e.terrain) addCond(e.terrain, 'terrain');
  }
}
for (const x of items.concat(abilities)) if (!x.fake && x.condition) addCond(x.id, null);
// a two-turn move adds a volatile of its own id during its charge turn (twoturnmove onStart), condition or not
for (const m of moves) if (!m.fake && m.flags && m.flags.charge) addCond(m.id, 'volatile');
const condIds = [''].concat([...condKinds.keys()].sort());
const condIdx = new Map(condIds.map((id, i) => [id, i]));
const condOf = (id) => {
  if (!id) return 'Cond(0)';
  const i = condIdx.get(toID(id));
  if (i === undefined) throw new Error('condition ' + id);
  return `Cond(${i})`;
};

// ------------------------------------------------------------------------------------------------ move flags
const flagNames = [...new Set(moves.flatMap(m => (m.fake ? [] : Object.keys(m.flags))))].sort();
if (flagNames.length > 64) throw new Error('too many flags');

// Move fields: every key of a legal move must be listed here (consumed or deliberately ignored).
const MOVE_KEYS_IGNORED = new Set(['name', 'id', 'fullname', 'effectType', 'exists', 'num', 'gen', 'isNonstandard',
  'duration', 'noCopy', 'affectsFainted', 'sourceEffect', 'baseMoveType', 'secondary', 'hasSheerForceBoost',
  'isZ', 'isMax', 'maxMove', 'zMove', 'contestType', 'spreadHit', 'tags', 'desc', 'shortDesc']);
const MOVE_KEYS_DATA = new Set(['type', 'target', 'basePower', 'accuracy', 'critRatio', 'secondaries', 'priority',
  'category', 'overrideOffensiveStat', 'overrideOffensivePokemon', 'overrideDefensiveStat', 'overrideDefensivePokemon',
  'ignoreNegativeOffensive', 'ignorePositiveDefensive', 'ignoreOffensive', 'ignoreDefensive', 'ignoreImmunity', 'pp',
  'noPPBoosts', 'status', 'weather', 'flags', 'selfSwitch', 'ignoreAbility', 'damage', 'forceSTAB', 'volatileStatus',
  'stallingMove', 'condition', 'multihit', 'self', 'boosts', 'recoil', 'heal', 'tracksTarget', 'willCrit',
  'slotCondition', 'sideCondition', 'hasCrashDamage', 'drain', 'forceSwitch', 'selfBoost', 'callsMove',
  'ignoreEvasion', 'smartTarget', 'terrain', 'selfdestruct', 'pseudoWeather', 'breaksProtect', 'ohko', 'thawsTarget',
  'multiaccuracy', 'sleepUsable', 'mindBlownRecoil', 'struggleRecoil', 'ignoreAccuracy', 'multihitType',
  'stealsBoosts', 'nonGhostTarget', 'critModifier', 'pressureTarget', 'alwaysHit', 'chloroblastRecoil']);
const HIT_KEYS = new Set(['chance', 'boosts', 'status', 'volatileStatus', 'self', 'onHit', 'sideCondition',
  'pseudoWeather', 'dustproof', 'kingsrock']);

function pair(a) { return a ? `(${a[0]}, ${a[1]})` : '(0, 0)'; }

function hitEffect(e, where) {
  for (const k of Object.keys(e)) if (!HIT_KEYS.has(k)) throw new Error(`${where}: unknown key ${k}`);
  if (e.self && e.self.self) throw new Error(`${where}: nested self`);
  const self = e.self ? hitEffect(e.self, where + '.self') : null;
  return `HitEffect { chance: ${e.chance === undefined ? 'None' : `Some(${e.chance})`}, ` +
    `boosts: ${boostsOf(e.boosts)}, status: ${statusOf(e.status || '')}, ` +
    `volatile_status: ${condOf(e.volatileStatus)}, side_condition: ${condOf(e.sideCondition)}, ` +
    `pseudo_weather: ${condOf(e.pseudoWeather)}, self_boosts: ${self ? boostsOf(e.self.boosts) : 'None'}, ` +
    `self_volatile: ${self ? condOf(e.self.volatileStatus) : 'Cond(0)'}, has_handler: ${!!(e.onHit || (e.self && e.self.onHit))} }`;
}

// Plain moves: what stage 1 plays exactly. Damaging, no secondary or self effect, no handler, standard targets,
// and only flags that change nothing without abilities, items and conditions.
const PLAIN_FLAGS = new Set(['protect', 'mirror', 'metronome', 'contact', 'bullet', 'sound', 'punch', 'bite',
  'slicing', 'pulse', 'wind', 'distance', 'nonsky', 'noassist', 'failcopycat', 'failmefirst', 'nosleeptalk',
  'failinstruct', 'failmimic', 'failencore', 'allyanim', 'noparentalbond', 'nosketch', 'mustpressure', 'heal',
  'minimize', 'bypasssub']);
const PLAIN_TARGETS = new Set(['normal', 'any', 'adjacentFoe', 'allAdjacentFoes', 'allAdjacent', 'randomNormal']);
const PLAIN_DATA_OK = new Set(['type', 'target', 'basePower', 'accuracy', 'critRatio', 'priority', 'category', 'pp',
  'noPPBoosts', 'flags', 'multihit', 'recoil', 'drain', 'willCrit', 'damage', 'ignoreImmunity',
  'ignoreNegativeOffensive', 'ignorePositiveDefensive', 'ignoreOffensive', 'ignoreDefensive', 'ignoreAbility',
  'forceSTAB', 'overrideOffensiveStat', 'overrideOffensivePokemon', 'overrideDefensiveStat', 'ignoreEvasion',
  'ignoreAccuracy']);
function plainReason(m) {
  if (m.category === 'Status') return 'status move';
  if (!PLAIN_TARGETS.has(m.target)) return 'target ' + m.target;
  if (handlersOf(m).length) return 'handlers';
  for (const f of Object.keys(m.flags)) if (!PLAIN_FLAGS.has(f)) return 'flag ' + f;
  for (const k of Object.keys(m)) {
    if (MOVE_KEYS_IGNORED.has(k) || m[k] === undefined) continue;
    if (k === 'ignoreImmunity' && m[k] === false) continue;
    if (typeof m[k] === 'boolean' && m[k] === false) continue;
    if (!PLAIN_DATA_OK.has(k)) return 'field ' + k;
  }
  if (m.multihitType) return 'multihitType';
  if (m.damage !== undefined && m.damage !== 'level') return 'damage ' + m.damage;
  if (m.basePower === 0 && m.damage !== 'level') return 'no power';
  return '';
}

// Stage-2 pool: moves whose rules need no ability, item, weather, terrain, room, side condition, slot condition
// or hazard. A move is out when its data names a field effect, or when one of its handlers (or the handlers of a
// condition it creates) refers to one, unless that reference is listed in INERT: a guard that cannot fire in a
// battle with No Ability, no items and no field effects (the resolver flags the battle when it does).
const STAGE2_CATS = {
  item: /\b(item|Item|items|takeItem|eatItem|useItem|hasItem|setItem|getItem|ignoringItem|isBerry|ateBerry)\b/,
  ability: /\b(hasAbility|getAbility|setAbility|ability|Ability|abilityState|suppressingAbility)\b/,
  weather: /\b(weather|Weather|sunnyday|raindance|effectiveWeather)\b/,
  terrain: /\b(terrain|Terrain|isTerrain)\b/,
  field: /(pseudoWeather|PseudoWeather|\bfield\.|\bgravity\b|\bGravity\b|\btrickroom\b)/,
  side: /(sideConditions|SideCondition|slotCondition|SlotCondition)/,
};
const INERT = {
  attract: {item: 'Destiny Knot message', ability: 'Cute Charm message'},
  bounce: {}, charge: {ability: 'Electromorphosis / Wind Power message'},
  disable: {ability: 'Cursed Body message'}, dive: {ability: 'Gulp Missile form change'},
  encore: {item: 'Mental Herb keeps the queued move'}, healbell: {ability: 'Soundproof / Good as Gold immunity'},
  healpulse: {ability: 'Mega Launcher amount'}, hex: {ability: 'Comatose counts as a status'},
  infernalparade: {ability: 'Comatose counts as a status'}, magnetrise: {field: 'fails under Gravity'},
  partingshot: {ability: 'Mirror Armor keeps the switch'}, rest: {ability: 'Comatose, Insomnia, Vital Spirit'},
  sleeptalk: {ability: 'Comatose'}, snore: {ability: 'Comatose'},
  smackdown: {item: 'Iron Ball', ability: 'Levitate', field: 'Gravity'},
  watershuriken: {ability: 'Battle Bond'},
  // conditions created by in-pool moves
  partiallytrapped: {item: 'Grip Claw duration, Binding Band damage'},
  confusion: {ability: 'Own Tempo-style message'}, healblock: {ability: 'Persistent duration'},
};
const DEFERRED = {
  skillswap: 'swaps abilities (Battle.skillSwap): abilities stage',
  imprison: 'disables the foe moves hidden (maybeDisabled): the request offers them, the choice is then rejected; the contract cannot express it',
};
function handlerSources(obj, pre, out) {
  if (!obj) return;
  for (const k of Object.keys(obj)) if (typeof obj[k] === 'function') out.push([pre + k, obj[k].toString()]);
}
function stage2Reason(m) {
  if (m.id === 'struggle') return 'not selectable';
  for (const k of ['sideCondition', 'slotCondition', 'weather', 'terrain', 'pseudoWeather']) {
    if (m[k]) return `field: ${k} ${toID(m[k])}`;
  }
  if (['foeSide', 'allySide'].includes(m.target)) return 'field: target ' + m.target;
  const scans = [[m.id, []]];
  handlerSources(m, '', scans[0][1]);
  handlerSources(m.condition, 'condition.', scans[0][1]);
  (m.secondaries || []).forEach((s, i) => handlerSources(s, `secondaries[${i}].`, scans[0][1]));
  handlerSources(m.self, 'self.', scans[0][1]);
  const vols = [m, m.self || {}, ...(m.secondaries || []), ...(m.secondaries || []).map(s => s.self || {})]
    .map(e => toID(e.volatileStatus)).filter(v => v && v !== m.id);
  for (const v of new Set(vols)) {
    const c = dex.conditions.get(v);
    const list = [];
    handlerSources(c, 'condition ' + v + '.', list);
    scans.push([v, list]);
  }
  for (const [owner, list] of scans) {
    for (const [name, src] of list) {
      for (const cat of Object.keys(STAGE2_CATS)) {
        const hit = src.match(STAGE2_CATS[cat]);
        if (hit && !(INERT[owner] && INERT[owner][cat])) return `${cat}: ${name} refers to ${hit[0]}`;
      }
    }
  }
  if (DEFERRED[m.id]) return 'deferred: ' + DEFERRED[m.id];
  return '';
}

// Stage-3 pool: the stage-2 rule with weather, terrain, rooms, side, slot and field conditions allowed. Their
// conditions' handlers (and those of the weather and terrain conditions the moves set) are scanned for item and
// ability references too; INERT3 lists the guards among them that cannot fire with No Ability and no items.
const INERT3 = {
  electricterrain: {item: 'Terrain Extender duration', ability: 'message when an ability set it'},
  grassyterrain: {item: 'Terrain Extender duration', ability: 'message when an ability set it'},
  mistyterrain: {item: 'Terrain Extender duration', ability: 'message when an ability set it'},
  psychicterrain: {item: 'Terrain Extender duration', ability: 'message when an ability set it'},
  sunnyday: {item: 'Heat Rock duration', ability: 'message and gen 5 duration when an ability set it'},
  raindance: {item: 'Damp Rock duration', ability: 'message and gen 5 duration when an ability set it'},
  sandstorm: {item: 'Smooth Rock duration', ability: 'message and gen 5 duration when an ability set it'},
  snowscape: {item: 'Icy Rock duration', ability: 'message and gen 5 duration when an ability set it'},
  chillyreception: {item: 'Icy Rock duration', ability: 'message and gen 5 duration when an ability set it'},
  trickroom: {ability: 'Persistent duration'}, gravity: {ability: 'Persistent duration'},
  wonderroom: {ability: 'Persistent duration'},
  magicroom: {ability: 'Persistent duration', item: 'held items End when it starts (no items)'},
  tailwind: {ability: 'Persistent duration'}, safeguard: {ability: 'Persistent duration'},
  reflect: {item: 'Light Clay duration'}, lightscreen: {item: 'Light Clay duration'},
  auroraveil: {item: 'Light Clay duration'},
  stealthrock: {item: 'Heavy-Duty Boots'}, spikes: {item: 'Heavy-Duty Boots'}, toxicspikes: {item: 'Heavy-Duty Boots'},
  stickyweb: {item: 'Heavy-Duty Boots'},
  magneticflux: {ability: 'Plus and Minus: with No Ability it always fails'},
};
function stage3Reason(m) {
  if (m.id === 'struggle') return 'not selectable';
  const scans = [[m.id, []]];
  handlerSources(m, '', scans[0][1]);
  handlerSources(m.condition, 'condition.', scans[0][1]);
  (m.secondaries || []).forEach((s, i) => handlerSources(s, `secondaries[${i}].`, scans[0][1]));
  handlerSources(m.self, 'self.', scans[0][1]);
  const vols = [m, m.self || {}, ...(m.secondaries || []), ...(m.secondaries || []).map(s => s.self || {})]
    .map(e => toID(e.volatileStatus)).filter(v => v && v !== m.id);
  // weather and terrain conditions live in conditions.ts, not on the move
  for (const k of ['weather', 'terrain']) if (m[k] && toID(m[k]) !== m.id) vols.push(toID(m[k]));
  for (const v of new Set(vols)) {
    const c = dex.conditions.get(v);
    const list = [];
    handlerSources(c, 'condition ' + v + '.', list);
    scans.push([v, list]);
  }
  for (const [owner, list] of scans) {
    for (const [name, src] of list) {
      for (const cat of ['item', 'ability']) {
        const hit = src.match(STAGE2_CATS[cat]);
        const inert = (INERT[owner] && INERT[owner][cat]) || (INERT3[owner] && INERT3[owner][cat]);
        if (hit && !inert) return `${cat}: ${name} refers to ${hit[0]}`;
      }
    }
  }
  if (DEFERRED[m.id]) return 'deferred: ' + DEFERRED[m.id];
  return '';
}

const moveRows = [];
const plainMoves = [];
const stage2Moves = [];
const stage2Excluded = {};
const stage3Moves = [];
const stage3Excluded = {};
for (const m of moves) {
  if (m.fake) {
    moveRows.push('    MoveData { id: "", name: "", num: 0, ty: Type::Typeless, category: Category::Status, base_power: 0, ' +
      'accuracy: ACC_ALWAYS, priority: 0, pp: 0, max_pp: 0, no_pp_boosts: true, target: Target::Normal, flags: 0, ' +
      'crit_ratio: 0, will_crit: None, multihit: (0, 0), multihit_parental_bond: false, drain: (0, 0), recoil: (0, 0), ' +
      'heal: (0, 0), damage: FixedDamage::None, ohko: Ohko::No, secondaries: &[], self_effect: None, self_boost: None, ' +
      'boosts: None, status: Status::None, volatile_status: Cond(0), side_condition: Cond(0), slot_condition: Cond(0), ' +
      'weather: Cond(0), terrain: Cond(0), pseudo_weather: Cond(0), condition: Cond(0), self_switch: SelfSwitch::No, ' +
      'force_switch: false, selfdestruct: SelfDestruct::No, ignore_immunity: false, ignore_ability: false, ' +
      'ignore_accuracy: false, ignore_evasion: false, ignore_offensive: false, ignore_defensive: false, ' +
      'ignore_negative_offensive: false, ignore_positive_defensive: false, override_offensive_stat: None, ' +
      'override_defensive_stat: None, offensive_pokemon_target: false, defensive_pokemon_source: false, ' +
      'breaks_protect: false, steals_boosts: false, thaws_target: false, multiaccuracy: false, sleep_usable: false, ' +
      'tracks_target: false, smart_target: false, force_stab: false, has_crash_damage: false, stalling_move: false, ' +
      'calls_move: false, struggle_recoil: false, mind_blown_recoil: false, non_ghost_target: None, crit_modifier: 0, ' +
      'handlers: &[], has_handlers: false },');
    continue;
  }
  for (const k of Object.keys(m)) {
    if (!MOVE_KEYS_IGNORED.has(k) && !MOVE_KEYS_DATA.has(k) && !CALLBACKS.includes(k) && !/^on[A-Z]/.test(k)) {
      throw new Error(`move ${m.id}: unknown key ${k}`);
    }
  }
  const handlers = handlersOf(m);
  const secs = (m.secondaries || []).map((s, i) => hitEffect(s, `${m.id}.secondaries[${i}]`));
  let selfEff = 'None';
  if (m.self) selfEff = `Some(${hitEffect(m.self, m.id + '.self')})`;
  const flags = Object.keys(m.flags).map(f => `flag::${upper(f)}`);
  let multihit = '(0, 0)';
  if (Array.isArray(m.multihit)) multihit = `(${m.multihit[0]}, ${m.multihit[1]})`;
  else if (m.multihit) multihit = `(${m.multihit}, ${m.multihit})`;
  let damage = 'FixedDamage::None';
  if (m.damage === 'level') damage = 'FixedDamage::Level';
  else if (typeof m.damage === 'number') damage = `FixedDamage::Value(${m.damage})`;
  let ohko = 'Ohko::No';
  if (m.ohko === true) ohko = 'Ohko::Yes';
  else if (m.ohko) ohko = `Ohko::Type(${typeOf(m.ohko)})`;
  const selfSwitch = m.selfSwitch === true ? 'SelfSwitch::Yes' : m.selfSwitch === 'copyvolatile' ? 'SelfSwitch::CopyVolatile'
    : m.selfSwitch === 'shedtail' ? 'SelfSwitch::ShedTail' : m.selfSwitch ? (() => { throw new Error('selfSwitch'); })() : 'SelfSwitch::No';
  const selfdestruct = m.selfdestruct === 'always' ? 'SelfDestruct::Always' : m.selfdestruct === 'ifHit' ? 'SelfDestruct::IfHit'
    : m.selfdestruct ? (() => { throw new Error('selfdestruct ' + m.selfdestruct); })() : 'SelfDestruct::No';
  if (m.ignoreImmunity && m.ignoreImmunity !== true) throw new Error(m.id + ': per-type ignoreImmunity');
  if (m.overrideDefensivePokemon && m.overrideDefensivePokemon !== 'source') throw new Error('overrideDefensivePokemon');
  if (m.overrideOffensivePokemon && m.overrideOffensivePokemon !== 'target') throw new Error('overrideOffensivePokemon');
  const reason = plainReason(m);
  if (!reason) plainMoves.push(m.id);
  const reason2 = stage2Reason(m);
  if (!reason2) stage2Moves.push(m.id);
  else stage2Excluded[m.id] = reason2;
  const reason3 = stage3Reason(m);
  if (!reason3) stage3Moves.push(m.id);
  else stage3Excluded[m.id] = reason3;
  const maxpp = m.noPPBoosts ? m.pp : (m.pp / 5 + 1) * 4;  // champions calculatePP
  if (!Number.isInteger(maxpp)) throw new Error('maxpp ' + m.id);
  moveRows.push(`    MoveData { id: ${lit(m.id)}, name: ${lit(m.name)}, num: ${m.num}, ty: ${typeOf(m.type)}, ` +
    `category: Category::${m.category}, base_power: ${m.basePower}, ` +
    `accuracy: ${m.accuracy === true ? 'ACC_ALWAYS' : m.accuracy}, priority: ${m.priority}, pp: ${m.pp}, max_pp: ${maxpp}, ` +
    `no_pp_boosts: ${!!m.noPPBoosts}, target: ${targetVariant(m.target)}, flags: ${flags.length ? flags.join(' | ') : '0'}, ` +
    `crit_ratio: ${m.critRatio || 0}, will_crit: ${m.willCrit === undefined ? 'None' : `Some(${m.willCrit})`}, ` +
    `multihit: ${multihit}, multihit_parental_bond: ${m.multihitType === 'parentalbond'}, drain: ${pair(m.drain)}, ` +
    `recoil: ${pair(m.recoil)}, heal: ${pair(m.heal)}, damage: ${damage}, ohko: ${ohko}, ` +
    `secondaries: &[${secs.join(', ')}], self_effect: ${selfEff}, self_boost: ${boostsOf(m.selfBoost && m.selfBoost.boosts)}, ` +
    `boosts: ${boostsOf(m.boosts)}, status: ${statusOf(m.status || '')}, volatile_status: ${condOf(m.volatileStatus)}, ` +
    `side_condition: ${condOf(m.sideCondition)}, slot_condition: ${condOf(m.slotCondition)}, weather: ${condOf(m.weather)}, ` +
    `terrain: ${condOf(m.terrain)}, pseudo_weather: ${condOf(m.pseudoWeather)}, condition: ${m.condition ? condOf(m.id) : 'Cond(0)'}, ` +
    `self_switch: ${selfSwitch}, force_switch: ${!!m.forceSwitch}, selfdestruct: ${selfdestruct}, ` +
    `ignore_immunity: ${!!m.ignoreImmunity}, ignore_ability: ${!!m.ignoreAbility}, ignore_accuracy: ${!!m.ignoreAccuracy}, ` +
    `ignore_evasion: ${!!m.ignoreEvasion}, ignore_offensive: ${!!m.ignoreOffensive}, ignore_defensive: ${!!m.ignoreDefensive}, ` +
    `ignore_negative_offensive: ${!!m.ignoreNegativeOffensive}, ignore_positive_defensive: ${!!m.ignorePositiveDefensive}, ` +
    `override_offensive_stat: ${m.overrideOffensiveStat ? `Some(${STATS[m.overrideOffensiveStat]})` : 'None'}, ` +
    `override_defensive_stat: ${m.overrideDefensiveStat ? `Some(${STATS[m.overrideDefensiveStat]})` : 'None'}, ` +
    `offensive_pokemon_target: ${m.overrideOffensivePokemon === 'target'}, defensive_pokemon_source: ${m.overrideDefensivePokemon === 'source'}, ` +
    `breaks_protect: ${!!m.breaksProtect}, steals_boosts: ${!!m.stealsBoosts}, thaws_target: ${!!m.thawsTarget}, ` +
    `multiaccuracy: ${!!m.multiaccuracy}, sleep_usable: ${!!m.sleepUsable}, tracks_target: ${!!m.tracksTarget}, ` +
    `smart_target: ${!!m.smartTarget}, force_stab: ${!!m.forceSTAB}, has_crash_damage: ${!!m.hasCrashDamage}, ` +
    `stalling_move: ${!!m.stallingMove}, calls_move: ${!!m.callsMove}, struggle_recoil: ${!!m.struggleRecoil}, ` +
    `mind_blown_recoil: ${!!(m.mindBlownRecoil || m.chloroblastRecoil)}, ` +
    `non_ghost_target: ${m.nonGhostTarget ? `Some(${targetVariant(m.nonGhostTarget)})` : 'None'}, ` +
    `crit_modifier: ${m.critModifier ? Math.round(m.critModifier * 4096) : 0}, ` +
    `handlers: ${strList(handlers)}, has_handlers: ${handlers.length > 0 || (m.secondaries || []).some(s => s.onHit || (s.self && s.self.onHit)) || !!(m.self && m.self.onHit)} },`);
}

// ------------------------------------------------------------------------------------------------ rows
const speciesRows = species.map(s => {
  const ab = ['0', '1', 'H', 'S'].map(k => {
    const id = toID(s.abilities[k]);
    if (!id) return 'AbilityId(0)';
    const i = abilityIdx.get(id);
    if (i === undefined) throw new Error(`species ${s.id}: ability ${id}`);
    return `AbilityId(${i})`;
  });
  const base = speciesIdx.get(toID(s.baseSpecies));
  const from = s.isMega ? speciesIdx.get(toID(s.changesFrom || s.baseSpecies)) : undefined;
  const stone = s.isMega ? itemIdx.get(toID(s.requiredItem)) : undefined;
  if (s.isMega && (from === undefined || stone === undefined)) throw new Error('mega ' + s.id);
  const t = s.types.map(typeOf);
  const st = s.baseStats;
  return `    SpeciesData { id: ${lit(s.id)}, name: ${lit(s.name)}, base_species: SpeciesId(${base === undefined ? speciesIdx.get(s.id) : base}), ` +
    `forme: ${lit(s.forme || '')}, types: [${t[0]}, ${t[1] || 'Type::Typeless'}], ` +
    `base_stats: [${st.hp}, ${st.atk}, ${st.def}, ${st.spa}, ${st.spd}, ${st.spe}], abilities: [${ab.join(', ')}], ` +
    `weight_hg: ${s.weighthg}, gender: ${lit(s.gender || '')}, is_mega: ${!!s.isMega}, ` +
    `mega_from: ${from === undefined ? 'None' : `Some(SpeciesId(${from}))`}, mega_stone: ${stone === undefined ? 'None' : `Some(ItemId(${stone}))`}, ` +
    `max_hp: ${s.maxHP || 0} },`;
});

const ITEM_KEYS_DATA = new Set(['name', 'id', 'fullname', 'effectType', 'exists', 'num', 'gen', 'isNonstandard',
  'duration', 'noCopy', 'affectsFainted', 'sourceEffect', 'fling', 'onDrive', 'onMemory', 'megaStone', 'zMove',
  'zMoveType', 'zMoveFrom', 'itemUser', 'isBerry', 'ignoreKlutz', 'onPlate', 'isGem', 'isPokeball', 'isPrimalOrb',
  'spritenum', 'naturalGift', 'isChoice', 'boosts', 'condition', 'desc', 'shortDesc', 'forcedForme', 'rating']);
const itemRows = items.map(x => {
  if (x.fake) return '    ItemData { id: "", name: "", num: 0, mega_stone: &[], is_berry: false, is_choice: false, ignore_klutz: false, fling_power: 0, handlers: &[], nums: &[] },';
  for (const k of Object.keys(x)) {
    if (!ITEM_KEYS_DATA.has(k) && !/^on[A-Z]/.test(k)) throw new Error(`item ${x.id}: unknown key ${k}`);
  }
  // onDrive/onMemory/onPlate are type names, not handlers
  const handlers = handlersOf(x).filter(h => !['onDrive', 'onMemory', 'onPlate'].includes(h));
  const megas = [];
  if (x.megaStone) {
    for (const [from, to] of Object.entries(x.megaStone).sort()) {
      const a = speciesIdx.get(toID(from)), b = speciesIdx.get(toID(to));
      if (a === undefined || b === undefined) continue;  // the other form is not legal
      megas.push(`(SpeciesId(${a}), SpeciesId(${b}))`);
    }
  }
  return `    ItemData { id: ${lit(x.id)}, name: ${lit(x.name)}, num: ${x.num}, mega_stone: &[${megas.join(', ')}], ` +
    `is_berry: ${!!x.isBerry}, is_choice: ${!!x.isChoice}, ignore_klutz: ${!!x.ignoreKlutz}, ` +
    `fling_power: ${(x.fling && x.fling.basePower) || 0}, handlers: ${strList(handlers)}, nums: ${numsOf(x)} },`;
});

const abilityRows = abilities.map(x => {
  if (x.fake) return '    AbilityData { id: "", name: "", num: 0, flags: 0, suppress_weather: false, handlers: &[], nums: &[] },';
  const AFLAGS = ['breakable', 'cantsuppress', 'failroleplay', 'failskillswap', 'noentrain', 'noreceiver', 'notrace', 'notransform'];
  for (const f of Object.keys(x.flags)) if (!AFLAGS.includes(f)) throw new Error(`ability ${x.id}: flag ${f}`);
  const flags = Object.keys(x.flags).map(f => `aflag::${upper(f)}`);
  return `    AbilityData { id: ${lit(x.id)}, name: ${lit(x.name)}, num: ${x.num}, flags: ${flags.length ? flags.join(' | ') : '0'}, ` +
    `suppress_weather: ${!!x.suppressWeather}, handlers: ${strList(handlersOf(x))}, nums: ${numsOf(x)} },`;
});

const condRows = condIds.map(id => {
  if (!id) return '    ConditionData { id: "", kind: 0, effect_type: "", duration: 0, no_copy: false, handlers: &[], nums: &[] },';
  const c = dex.conditions.getByID(id);
  const kinds = [...condKinds.get(id)].sort();
  const kindExpr = kinds.length ? kinds.map(k => `ckind::${k.toUpperCase()}`).join(' | ') : '0';
  return `    ConditionData { id: ${lit(id)}, kind: ${kindExpr}, effect_type: ${lit(c && c.exists ? c.effectType : '')}, ` +
    `duration: ${(c && c.duration) || 0}, no_copy: ${!!(c && c.exists && c.noCopy)}, ` +
    `handlers: ${strList(c && c.exists ? handlersOf(c) : [])}, nums: ${numsOf(c && c.exists ? c : null)} },`;
});

// type chart: TYPE_CHART[attacking][defending] = 0 neutral, 1 super effective, -1 resisted, IMMUNE
const chart = TYPES.map(atk => TYPES.map(def => {
  const t = dex.types.get(def).damageTaken[TYPE_VARIANTS[TYPES.indexOf(atk)]];
  return t === 1 ? 1 : t === 2 ? -1 : t === 3 ? 'IMMUNE' : 0;
}));

const natures = dex.natures.all().slice().sort(byId);
const statIdx = {'': 'None', atk: 'Some(Stat::Atk)', def: 'Some(Stat::Def)', spa: 'Some(Stat::Spa)', spd: 'Some(Stat::Spd)', spe: 'Some(Stat::Spe)'};
const natureRows = natures.map(n => `    NatureData { id: ${lit(n.id)}, name: ${lit(n.name)}, plus: ${statIdx[n.plus || '']}, minus: ${statIdx[n.minus || '']} },`);

// ------------------------------------------------------------------------------------------------ constants
function consts(mod, ty, list) {
  const lines = list.map((x, i) => (x ? `    pub const ${upper(x)}: ${ty} = ${ty}(${i});` : `    pub const NONE: ${ty} = ${ty}(0);`));
  return `pub mod ${mod} {\n    use super::*;\n${lines.join('\n')}\n}\n`;
}

const out = [];
out.push(`// Generated by tools/data/gen_sim_data.js from Pokemon Showdown (mod champions). Do not edit.
#![allow(clippy::all)]
use super::types::*;

pub const SHOWDOWN_COMMIT: &str = ${lit(commit)};
pub const FORMAT: &str = ${lit(FORMAT)};

/// Moves stage 1 plays exactly (see tools/data/gen_sim_data.js, \`plainReason\`).
pub static PLAIN_MOVES: [MoveId; ${plainMoves.length}] = [${plainMoves.map(id => `MoveId(${moveIdx.get(id)})`).join(', ')}];

/// Moves of the stage-2 pool (see tools/data/gen_sim_data.js, \`stage2Reason\`; tools/rig/data/stage2_moves.json).
pub static STAGE2_MOVES: [MoveId; ${stage2Moves.length}] = [${stage2Moves.map(id => `MoveId(${moveIdx.get(id)})`).join(', ')}];

/// Moves of the stage-3 pool (see tools/data/gen_sim_data.js, \`stage3Reason\`; tools/rig/data/stage3_moves.json).
pub static STAGE3_MOVES: [MoveId; ${stage3Moves.length}] = [${stage3Moves.map(id => `MoveId(${moveIdx.get(id)})`).join(', ')}];

pub mod flag {
${flagNames.map((f, i) => `    pub const ${upper(f)}: u64 = 1 << ${i};`).join('\n')}
    pub const NAMES: [&str; ${flagNames.length}] = [${flagNames.map(lit).join(', ')}];
}

pub static TYPE_CHART: [[i8; 18]; 18] = [
${chart.map(r => `    [${r.join(', ')}],`).join('\n')}
];

pub static NATURES: [NatureData; ${natures.length}] = [
${natureRows.join('\n')}
];

pub static SPECIES: [SpeciesData; ${species.length}] = [
${speciesRows.join('\n')}
];

pub static MOVES: [MoveData; ${moves.length}] = [
${moveRows.join('\n')}
];

pub static ITEMS: [ItemData; ${items.length}] = [
${itemRows.join('\n')}
];

pub static ABILITIES: [AbilityData; ${abilities.length}] = [
${abilityRows.join('\n')}
];

pub static CONDITIONS: [ConditionData; ${condIds.length}] = [
${condRows.join('\n')}
];
`);
out.push(consts('species', 'SpeciesId', species.map(s => s.id)));
out.push(consts('moves', 'MoveId', moves.map(m => m.id)));
out.push(consts('items', 'ItemId', items.map(x => x.id)));
out.push(consts('abilities', 'AbilityId', abilities.map(x => x.id)));
out.push(consts('conds', 'Cond', condIds));

fs.mkdirSync(path.dirname(OUT_RS), {recursive: true});
fs.writeFileSync(OUT_RS, out.join('\n'));
fs.writeFileSync(OUT_PLAIN, JSON.stringify({
  showdown: commit, generator: 'tools/data/gen_sim_data.js', count: plainMoves.length, moves: plainMoves,
}, null, 1) + '\n');
const byReason = {};
for (const [id, r] of Object.entries(stage2Excluded)) {
  const k = r.split(':')[0];
  byReason[k] = (byReason[k] || 0) + 1;
}
fs.writeFileSync(OUT_STAGE2, JSON.stringify({
  showdown: commit, generator: 'tools/data/gen_sim_data.js',
  rule: 'legal moves with No Ability, no item, no weather, terrain, room, side or slot condition; ' +
    'references in INERT are guards that cannot fire in such a battle',
  count: stage2Moves.length, moves: stage2Moves, excluded_count: Object.keys(stage2Excluded).length,
  excluded_by_reason: byReason, excluded: stage2Excluded, inert: INERT,
}, null, 1) + '\n');
const byReason3 = {};
for (const [id, r] of Object.entries(stage3Excluded)) {
  const k = r.split(':')[0];
  byReason3[k] = (byReason3[k] || 0) + 1;
}
const added3 = stage3Moves.filter(id => !stage2Moves.includes(id));
// Moves of the stage-2 pool that the field changes: types the weathers and terrains boost or weaken, priority
// (Psychic Terrain, Quick Guard), spread (Wide Guard), statuses and confusion (Safeguard, the terrains),
// grounding, Gravity's moves, switching (hazards), Protect and Substitute (blocking the removers), stats the
// rooms and Tailwind change, semi-invulnerable turns (weather damage).
const INTERACT_IDS = new Set(['earthquake', 'bulldoze', 'roost', 'smackdown', 'ingrain', 'magnetrise', 'protect',
  'detect', 'substitute', 'leechseed', 'bodypress', 'psyshock', 'electroball', 'gyroball', 'dig', 'dive', 'fly',
  'bounce', 'rest', 'yawn', 'feint', 'highjumpkick', 'batonpass', 'shedtail', 'partingshot', 'sleeptalk',
  'healblock', 'endure', 'trickortreat', 'forestscurse', 'soak', 'reflecttype', 'transform', 'copycat',
  'wrap', 'firespin', 'whirlpool', 'sandtomb', 'infestation', 'bind', 'snaptrap', 'phantomforce', 'metalburst',
  'counter', 'mirrorcoat', 'outrage', 'thrash', 'petaldance', 'ragingfury', 'uproar', 'focuspunch']);
const INTERACT_TYPES = new Set(['Fire', 'Water', 'Electric', 'Grass', 'Psychic', 'Dragon']);
const interact3 = stage2Moves.filter(id => {
  const m = dex.moves.get(id);
  if (INTERACT_IDS.has(id)) return true;
  if (m.priority > 0) return true;
  if (['allAdjacent', 'allAdjacentFoes'].includes(m.target)) return true;
  if (m.flags.gravity) return true;
  if (m.selfSwitch || m.forceSwitch) return true;
  if (m.category === 'Status' && (m.status || m.volatileStatus === 'confusion')) return true;
  if (m.category !== 'Status' && INTERACT_TYPES.has(m.type) && (m.basePower >= 60 || m.basePowerCallback)) return true;
  return false;
});
fs.writeFileSync(OUT_STAGE3, JSON.stringify({
  showdown: commit, generator: 'tools/data/gen_sim_data.js',
  rule: 'legal moves with No Ability and no item; weather, terrain, rooms, side and slot conditions allowed; ' +
    'references in INERT and INERT3 are guards that cannot fire in such a battle',
  count: stage3Moves.length, moves: stage3Moves, added_count: added3.length, added: added3,
  interacting_count: interact3.length, interacting: interact3,
  excluded_count: Object.keys(stage3Excluded).length, excluded_by_reason: byReason3, excluded: stage3Excluded,
  inert: INERT, inert3: INERT3,
}, null, 1) + '\n');
console.log(JSON.stringify({species: species.length, moves: moves.length - 1, items: items.length - 1,
  abilities: abilities.length - 1, conditions: condIds.length - 1, flags: flagNames.length, plain: plainMoves.length,
  stage2: stage2Moves.length, stage3: stage3Moves.length}));
