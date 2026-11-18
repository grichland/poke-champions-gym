// Dump the Champions-mod Dex (species, moves, items, abilities) to JSON. Read-only on Showdown.
const path = require('path');
const SD = process.argv[2];
const out = process.argv[3];
const {Dex} = require(path.join(SD, 'dist', 'sim'));
const d = Dex.mod('champions');
const rep = (k, v) => typeof v === 'function' ? '[fn]' : v;
const plain = (o) => JSON.parse(JSON.stringify(o, rep));
const species = {}, moves = {}, items = {}, abilities = {};
for (const s of d.species.all()) {
  const o = plain(s);
  delete o.learnset;
  o.tier = s.tier; o.doublesTier = s.doublesTier;
  species[s.id] = o;
}
const legalSpecies = new Set();
const learn = {};
for (const s of d.species.all()) {
  if (s.isNonstandard || s.tier === 'Illegal' || s.tier === undefined) continue;
  legalSpecies.add(s.id);
}
const legalMoves = new Set();
for (const id of legalSpecies) {
  const ld = d.species.getLearnsetData(id);
  if (ld && ld.learnset) for (const m of Object.keys(ld.learnset)) legalMoves.add(m);
}
for (const m of d.moves.all()) moves[m.id] = plain(m);
for (const i of d.items.all()) items[i.id] = plain(i);
for (const a of d.abilities.all()) abilities[a.id] = plain(a);
const legalAb = new Set();
for (const id of legalSpecies) for (const a of Object.values(d.species.get(id).abilities)) legalAb.add(d.toID(a));
const fixup = (tbl) => tbl;
const counts = {
  species_all: Object.keys(species).length,
  species_legal_tier: legalSpecies.size,
  species_no_nonstandard: Object.values(species).filter(s => !s.isNonstandard).length,
  moves_all: Object.keys(moves).length,
  moves_no_nonstandard: Object.values(moves).filter(m => !m.isNonstandard).length,
  moves_in_legal_learnsets: legalMoves.size,
  items_all: Object.keys(items).length,
  items_no_nonstandard: Object.values(items).filter(i => !i.isNonstandard).length,
  abilities_all: Object.keys(abilities).length,
  abilities_no_nonstandard: Object.values(abilities).filter(a => !a.isNonstandard).length,
  abilities_of_legal_species: legalAb.size,
};
console.log(JSON.stringify(counts, null, 1));
require('fs').writeFileSync(out, JSON.stringify({counts, legal: {species: [...legalSpecies], moves: [...legalMoves], abilities: [...legalAb]}, species, moves, items, abilities}));
