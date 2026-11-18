// Damage of one hit computed by Showdown, for the new resolver's damage test (engine/tests/sim_damage.rs).
// Random legal species (Megas included) with No Ability and no item, random natures, stat points, stat
// stages and burn, every plain move, a forced crit and roll; Showdown's getDamage gives the expected value.
// usage: node tools/data/damage_fixture.js [out.tsv]   (Showdown from $SHOWDOWN_DIR, default below)
'use strict';
const path = require('path');
const fs = require('fs');

const SD = process.env.SHOWDOWN_DIR ||
  'C:\\Users\\user\\Code\\pokemon_champions\\pcsa\\.tools\\vendor\\pokemon-showdown';
const REPO = path.join(__dirname, '..', '..');
const OUT = process.argv[2] || path.join(REPO, 'engine', 'tests', 'data', 'sim_damage.tsv');
const FORMAT = 'gen9championsbssregmc';
const {Dex, Battle} = require(path.join(SD, 'dist', 'sim'));
const dex = Dex.mod('champions');
const plain = JSON.parse(fs.readFileSync(path.join(REPO, 'tools', 'rig', 'data', 'plain_moves.json'), 'utf8')).moves;

let seed = 20261117;
const rnd = (n) => { seed = (seed * 1103515245 + 12345) % 2147483648; return Math.floor(seed / 65536) % n; };
const pick = (a) => a[rnd(a.length)];
const species = dex.species.all().filter(s => !s.isNonstandard).map(s => s.name).sort();
const natures = dex.natures.all().map(n => n.name).sort();
const KEYS = ['hp', 'atk', 'def', 'spa', 'spd', 'spe'];
const BOOSTS = ['atk', 'def', 'spa', 'spd', 'spe'];

function set() {
  const evs = {};
  for (const k of KEYS) evs[k] = rnd(4) === 0 ? 0 : rnd(33);
  return {species: pick(species), item: '', ability: 'No Ability', moves: ['tackle'], nature: pick(natures),
    evs, level: 50, gender: 'M'};
}

const rows = ['# attacker\tnature\tevs\tboosts\tstatus\tdefender\tnature\tevs\tboosts\tmove\tcrit\troll\tdamage'];
const BATTLES = 120;
for (let b = 0; b < BATTLES; b++) {
  const battle = new Battle({formatid: FORMAT, seed: [b + 1, 2, 3, 4]});
  const teams = [[], []];
  for (const t of teams) for (let i = 0; i < 6; i++) t.push(set());
  battle.setPlayer('p1', {name: 'p1', team: teams[0]});
  battle.setPlayer('p2', {name: 'p2', team: teams[1]});
  battle.choose('p1', 'team 123');
  battle.choose('p2', 'team 123');
  const [a, d] = [battle.sides[0].active[0], battle.sides[1].active[0]];
  for (let c = 0; c < 40; c++) {
    const [src, tgt] = rnd(2) ? [a, d] : [d, a];
    for (const p of [src, tgt]) {
      for (const k of BOOSTS) p.boosts[k] = rnd(3) === 0 ? rnd(13) - 6 : 0;
    }
    src.status = rnd(5) === 0 ? 'brn' : '';
    const moveid = pick(plain);
    const crit = rnd(4) === 0;
    const roll = rnd(16);
    const saved = battle.prng;
    battle.prng = {
      random: (m, n) => (m === 16 && n === undefined ? roll : (() => { throw new Error('unexpected random'); })()),
      randomChance: () => crit,
      sample: () => { throw new Error('unexpected sample'); },
      shuffle: () => {},
    };
    const move = battle.dex.getActiveMove(moveid);
    move.hit = 1;
    battle.setActiveMove(move, src, tgt);
    let dmg = battle.actions.getDamage(src, tgt, move, true);
    battle.clearActiveMove();
    battle.prng = saved;
    if (dmg === false) dmg = 'fail';
    if (dmg === undefined) dmg = 'none';
    const row = (p) => [p.species.id, toId(p.set.nature), KEYS.map(k => p.set.evs[k]).join(','), BOOSTS.map(k => p.boosts[k]).join(',')];
    rows.push([...row(src), src.status || '-', ...row(tgt), moveid, crit ? 1 : 0, roll, dmg].join('\t'));
  }
  battle.destroy();
}
function toId(s) { return String(s).toLowerCase().replace(/[^a-z0-9]/g, ''); }
fs.mkdirSync(path.dirname(OUT), {recursive: true});
fs.writeFileSync(OUT, rows.join('\n') + '\n');
console.log(`${rows.length - 1} cases -> ${OUT}`);
