# Audit: ITEMS slice 02 (CSV lines 58-112)

Slice: `03-coverage-items.csv` lines 58 to 112, 55 rows, all audited. NOT AUDITED: none.
Engine build: features `champions` + `bss` (no genN feature), so `src/genx/*` is compiled (lib.rs:14-16). Item code: `genx/items.rs`. Showdown: base `data/items.ts` plus `data/mods/champions/items.ts` (the override file only touches `isNonstandard` for every item in this slice; no behaviour overrides).

## Counts per verdict

| Verdict | Count |
|---|---|
| MATCH | 7 |
| MISSING | 7 |
| PARTIAL | 2 |
| DATA | 1 |
| LOGIC | 38 (34 Mega stones, 3 damage-halving berries, Pecha) |
| PLAIN / CORE / UNSURE | 0 |
| Total | 55 |

## Five most important findings

1. **Resist berries (Kebia, Chople, Colbur) are consumed at the wrong time and by the wrong moves.** `item_before_move` (items.rs:462-623) runs before the accuracy roll, before sleep/paralysis/flinch checks (generate_instructions.rs:2255 vs 2306/2363) and with no category check. The berry is therefore lost to misses, Protect, a sleeping attacker, Substitute hits, and to status moves. Example: Toxic (Poison, move_type POISON) used on a Fairy or Grass holder of Kebia strips the berry; Taunt or Nasty Plot vs a Ghost or Psychic holder of Colbur does the same. Showdown only eats on a damaging hit that is not absorbed by Substitute. A multi-hit move also gets half power on every hit, where Showdown halves only the first.
2. **Mega Drampa has Sp. Def 114 in the engine; Showdown has 116** (base_stats.rs:100 and :546; pokedex.ts:15104). This is the only stat or type or ability error among the 35 stones checked.
3. **Seven items in this slice are absent from the `Items` enum and become UNKNOWNITEM with no effect**: Scope Lens, King's Rock, Heat Rock, Red Card, Zoom Lens, Leek, Big Root.
4. **Pecha Berry cures one end-of-turn poison tick too late.** Showdown eats it the moment the holder is poisoned. The engine applies poison/toxic damage and the Toxic counter at generate_instructions.rs:3150-3199, and only then `item_end_of_turn` (:3212). The holder takes a tick the berry should have prevented.
5. **A Mega stone can be Tricked away by its own holder.** Trick/Switcheroo (choice_effects.rs:1587) tests only the target's item. Showdown fails the move if either stone is untakeable (moves.ts:19882). Knock Off, Thief and the target side of Trick are correctly blocked (state.rs:617-708, :713).

## Table

Notation: SD = Showdown. X1 = Klutz / Embargo / Magic Room not modelled (see Notes); it applies to every item that has an effect and is not repeated per row. "K1-rounding" = engine multiplies base power by 1.2 where SD uses 4915/4096 inside its integer chain.

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| spelltag | 1 | MATCH | items.ts:5926 Ghost moves x4915/4096 base power; champions items.ts:874 only sets isNonstandard null | items.rs:1290 Ghost BP x1.2 | none (K1-rounding, X1) | | high |
| scopelens | 1 | MISSING | items.ts:5578 crit stage +1 (1/24 -> 1/8) | not in `Items` enum (items.rs:25-282), parses to UNKNOWNITEM; crit chance only by move (generate_instructions.rs:2436-2465) | item has no effect; crit chance never raised by item | M | high |
| eelektrossite | 1 | LOGIC | items.ts:1669-1676 megaStone Eelektross -> Eelektross-Mega; onTakeItem false for holder. Mega: Electric, 85/145/80/135/90/80, Eelevate (pokedex.ts:11159) | base_stats.rs:578 ELEKTROSS->EELEKTROSSMEGA, Electric/Typeless, same stats, EELEVATE; item_is_permanent state.rs:692 | stone/form/stats/ability correct, protected from Knock Off/Thief/target-Trick. User's own stone can be Tricked away (see M1) | S | high |
| dragalgite | 1 | LOGIC | items.ts:1485 Dragalge -> Dragalge-Mega. Poison/Dragon, 65/85/105/132/163/44, Regenerator (pokedex.ts:13001) | base_stats.rs:602 same data; state.rs:691 | data correct; M1 only | S | high |
| galladite | 1 | LOGIC | items.ts:2389 Gallade -> Gallade-Mega. Psychic/Fighting, 68/165/95/65/115/110, Inner Focus (pokedex.ts:8663) | base_stats.rs:398 same data; state.rs:661 | data correct; M1 only | S | high |
| mysticwater | 1 | MATCH | items.ts:4272 Water moves x4915/4096 | items.rs:1231 Water BP x1.2 | none (K1-rounding, X1) | | high |
| pyroarite | 1 | LOGIC | items.ts:4985 Pyroar -> Pyroar-Mega. Fire/Normal, 86/88/92/129/86/126, Fire Mane (pokedex.ts:12524) | base_stats.rs:584 same data; state.rs:700 | data correct; M1 only | S | high |
| kingsrock | 1 | MISSING | items.ts:3221 every non-Status move gets a 10% flinch secondary unless it already has a flinch secondary | not in enum -> UNKNOWNITEM | no extra 10% flinch | M | high |
| glalitite | 1 | LOGIC | items.ts:2511 Glalie -> Glalie-Mega. Ice, 80/120/80/120/80/100, Refrigerate (pokedex.ts:6779) | base_stats.rs:344 same data; state.rs:652 | data correct; M1 only | S | high |
| heatrock | 1 | MISSING | items.ts:2853 no handler; conditions.ts:555 Sunny Day lasts 8 turns when the setter holds it (also via Drought, durationCallback with source) | not in enum; SUNNYDAY sets 5 turns (choice_effects.rs:1610-1624); Drought sets WEATHER_ABILITY_TURNS=5 (abilities.rs:34) | Sun lasts 5 instead of 8 for a Heat Rock holder (move and Drought) | M | high |
| absolite | 1 | LOGIC | items.ts:37 Absol -> Absol-Mega. Dark, 65/150/60/115/60/115, Magic Bounce (pokedex.ts:6710) | base_stats.rs:338 same data; state.rs:651 | data correct; M1 only | S | high |
| kangaskhanite | 1 | LOGIC | items.ts:3132 Kangaskhan -> Kangaskhan-Mega. Normal, 105/125/100/60/100/100, Parental Bond (pokedex.ts:2653) | base_stats.rs:188 same data; state.rs:626 | data correct; M1 only | S | high |
| beedrillite | 1 | LOGIC | items.ts:425 Beedrill -> Beedrill-Mega. Bug/Poison, 65/150/40/15/80/145, Adaptability (pokedex.ts:343) | base_stats.rs:158 same data; state.rs:621 | data correct; M1 only | S | high |
| redcard | 1 | MISSING | items.ts:5170 after a damaging hit on holder (holder and attacker alive, attacker has a bench mon): item used up, attacker force-switched to a random bench mon | not in enum; the drag mechanism exists (Choice flags.drag, generate_instructions.rs:1708) but nothing triggers it from an item | holder's Red Card does nothing | M | high |
| sceptilite | 1 | LOGIC | items.ts:5545 Sceptile -> Sceptile-Mega. Grass/Dragon, 70/110/75/145/85/145, Lightning Rod (pokedex.ts:5060) | base_stats.rs:260 same data; state.rs:638 | data correct; M1 only | S | high |
| altarianite | 1 | LOGIC | items.ts:241 Altaria -> Altaria-Mega. Dragon/Fairy, 75/110/110/110/105/80, Pixilate (pokedex.ts:6301) | base_stats.rs:326 same data; state.rs:649 | data correct; M1 only | S | high |
| zoomlens | 1 | MISSING | items.ts:7849 accuracy x4915/4096 if the target has already moved this turn (`!queue.willMove(target)`) | not in enum | no accuracy boost | M | high |
| feraligite | 1 | LOGIC | items.ts:2006 Feraligatr -> Feraligatr-Mega. Water/Dragon, 85/160/125/89/93/78, Dragonize (pokedex.ts:3604) | base_stats.rs:434 same data (arm keyed FERALIGATR); state.rs:667 | data correct; M1 only | S | high |
| leek | 1 | MISSING | items.ts:3334 Farfetch'd/Sirfetch'd crit stage +2; champions items.ts:514 makes it legal (isNonstandard null) | not in enum -> UNKNOWNITEM | no crit boost for Farfetch'd/Sirfetch'd | M | high |
| kebiaberry | 1 | LOGIC | items.ts:3165 on a damaging Poison hit that is super effective and not absorbed by Substitute: `eatItem()` and damage x0.5 | items.rs:542 + 304-325 `damage_reduction_berry`: if move_type Poison and type chart > 1.0, item removed and BP /2. Runs in `item_before_move` (generate_instructions.rs:1687, called at :2255) | (a) no category check: Toxic, Poison Gas etc. strip it; (b) consumed before accuracy, Protect, sleep/para/flinch gates; (c) no Substitute check; (d) multi-hit halves every hit, SD only the first; (e) no Unnerve (ability not implemented anywhere); X1 | M | high |
| aerodactylite | 1 | LOGIC | items.ts:138 Aerodactyl -> Aerodactyl-Mega. Rock/Flying, 80/135/85/70/95/150, Tough Claws (pokedex.ts:3192) | base_stats.rs:206 same data; state.rs:629 | data correct; M1 only | S | high |
| pinsirite | 1 | LOGIC | items.ts:4631 Pinsir -> Pinsir-Mega. Bug/Flying, 65/155/120/65/90/105, Aerilate (pokedex.ts:2870) | base_stats.rs:194 same data; state.rs:627 | data correct; M1 only | S | high |
| chesnaughtite | 1 | LOGIC | items.ts:880 Chesnaught -> Chesnaught-Mega. Grass/Fighting, 88/137/172/74/115/44, Bulletproof (pokedex.ts:12050) | base_stats.rs:446 same data; state.rs:669 | data correct; M1 only | S | high |
| pechaberry | 1 | LOGIC | items.ts:4521 onUpdate: eaten as soon as holder is poisoned or toxic (also on switch-in); cures status | items.rs:772-777 (before holder's own move) and :927-932 (end of turn); `status_cure_berry` items.rs:418 | eaten late: poison/toxic damage and Toxic counter increment run first (generate_instructions.rs:3150-3199, berry at :3212), so holder takes a tick SD would have prevented; a holder that does not move before end of turn also takes it. Berry eligible only when status already set at one of those two points. X1, no Unnerve | M | high |
| fairyfeather | 1 | MATCH | items.ts:1939 Fairy moves x4915/4096 | items.rs:1189 Fairy BP x1.2 | none (K1-rounding, X1) | | high |
| froslassite | 1 | LOGIC | items.ts:2348 Froslass -> Froslass-Mega. Ice/Ghost, 70/80/70/140/100/120, Snow Warning (pokedex.ts:8723) | base_stats.rs:530 same data; state.rs:682 | data correct; M1 only | S | high |
| aggronite | 1 | LOGIC | items.ts:150 Aggron -> Aggron-Mega. Steel, 70/140/230/60/80/50, Filter (pokedex.ts:5872) | base_stats.rs:296 same data; state.rs:644 | data correct; M1 only | S | high |
| blackbelt | 1 | MATCH | items.ts:514 Fighting moves x4915/4096 | items.rs:1150 Fighting BP x1.2 | none (K1-rounding, X1) | | high |
| sablenite | 1 | LOGIC | items.ts:5456 Sableye -> Sableye-Mega. Dark/Ghost, 50/85/125/85/115/20, Magic Bounce (pokedex.ts:5790) | base_stats.rs:284 same data; state.rs:642 | data correct; M1 only | S | high |
| drampanite | 1 | DATA | items.ts:1575 Drampa -> Drampa-Mega. Normal/Dragon, 78/85/110/160/**116**/36, Berserk (pokedex.ts:15100-15105) | base_stats.rs:542-547 and table :100 give Sp. Def **114**; types, ability, stone mapping correct; state.rs:684 | Mega Drampa Sp. Def 114 vs 116 (two places). Also M1 | S | high |
| sharpedonite | 1 | LOGIC | items.ts:5648 Sharpedo -> Sharpedo-Mega. Water/Dark, 70/140/70/110/65/105, Strong Jaw (pokedex.ts:6083) | base_stats.rs:314 same data; state.rs:647 | data correct; M1 only | S | high |
| alakazite | 1 | LOGIC | items.ts:218 Alakazam -> Alakazam-Mega. Psychic, 55/50/65/175/105/150, Trace (pokedex.ts:1618) | base_stats.rs:170 same data; state.rs:623 | data correct; M1 only | S | high |
| falinksite | 1 | LOGIC | items.ts:1987 Falinks -> Falinks-Mega. Fighting, 65/135/135/70/65/100, Defiant (pokedex.ts:16862) | base_stats.rs:608 same data; state.rs:693 | data correct; M1 only | S | high |
| pidgeotite | 1 | LOGIC | items.ts:4584 Pidgeot -> Pidgeot-Mega. Normal/Flying, 83/80/80/135/80/121, No Guard (pokedex.ts:398) | base_stats.rs:164 same data; state.rs:622 | data correct; M1 only | S | high |
| abomasite | 1 | LOGIC | items.ts:25 Abomasnow -> Abomasnow-Mega. Grass/Ice, 90/132/105/132/105/30, Snow Warning (pokedex.ts:8427) | base_stats.rs:392 same data; state.rs:660 | data correct; M1 only | S | high |
| chopleberry | 1 | LOGIC | items.ts:1042 damaging Fighting hit, super effective, not absorbed by Substitute: eat, damage x0.5 | items.rs:472 + 304-325, same function as Kebia | same (a)-(e) as kebiaberry; also Detect/Bulk Up (Fighting status) used against a Normal/Rock/Dark/Steel/Ice holder strips it | M | high |
| emboarite | 1 | LOGIC | items.ts:1849 Emboar -> Emboar-Mega. Fire/Fighting, 110/148/75/110/110/75, Mold Breaker (pokedex.ts:9499) | base_stats.rs:440 same data; state.rs:668 | data correct; M1 only | S | high |
| barbaracite | 1 | LOGIC | items.ts:394 Barbaracle -> Barbaracle-Mega. Rock/Fighting, 72/140/130/64/106/88, Tough Claws (pokedex.ts:12959) | base_stats.rs:596 same data; state.rs:688 | data correct; M1 only | S | high |
| lightball | 1 | MATCH | items.ts:3439 Pikachu (any forme, by baseSpecies): Atk and SpA x2 | items.rs:1541 BP x2 if `is_pikachu_variant` (pokemon.rs:1481, all Pikachu formes) | none for damaging moves (see Notes N3 for two minor edge cases); X1 | | medium |
| audinite | 1 | LOGIC | items.ts:337 Audino -> Audino-Mega. Normal/Fairy, 103/60/126/80/126/50, Healer (pokedex.ts:9957) | base_stats.rs:404 same data; state.rs:662 | data correct; M1 only | S | high |
| dragonfang | 1 | MATCH | items.ts:1494 Dragon moves x4915/4096 | items.rs:1175 Dragon BP x1.2 (shared arm with Dragon Scale, see Notes N4) | none for Dragon Fang (K1-rounding, X1) | | high |
| skarmorite | 1 | LOGIC | items.ts:5793 Skarmory -> Skarmory-Mega. Steel/Flying, 65/140/110/40/100/110, Stalwart (pokedex.ts:4652) | base_stats.rs:512 same data; state.rs:680 | data correct; M1 only | S | high |
| psychicseed | 1 | PARTIAL | items.ts:4927 onStart: used if Psychic Terrain is up (+1 SpD); onTerrainChange: used when terrain later becomes Psychic; consumed even if the boost cannot apply | items.rs:879-899 only `item_on_switch_in` (called generate_instructions.rs:571 and :4032) | missing: terrain set after holder is already out (own or opposing terrain move/ability, e.g. Psychic Surge) never triggers it; item kept if +SpD is at +6 (apply_boost_instruction returns false, items.rs:888); X1 | M | high |
| colburberry | 1 | LOGIC | items.ts:1145 damaging Dark hit, super effective, not absorbed by Substitute: eat, damage x0.5 | items.rs:518 + 304-325 | same (a)-(e) as kebiaberry; Taunt/Torment/Nasty Plot (Dark status) vs Psychic/Ghost holder strips it | M | high |
| bigroot | 1 | MISSING | items.ts:482 drain, Leech Seed, Ingrain, Aqua Ring, Strength Sap healing x5324/4096 | not in enum -> UNKNOWNITEM | holder's draining/seeding heals are not boosted | M | high |
| electricseed | 1 | PARTIAL | items.ts:1811 onStart if Electric Terrain (+1 Def); onTerrainChange when Electric Terrain starts; consumed even if boost cannot apply | items.rs:816-836 only on switch-in | same as psychicseed: no terrain-change trigger, kept at +6 Def, X1 | M | high |
| floettite | 1 | LOGIC | items.ts:2201-2213 megaStone `Floette-Eternal` -> Floette-Mega; takeable unless holder is that species or the Mega. Fairy, 74/85/87/155/148/102, Fairy Aura (pokedex.ts:12589) | base_stats.rs:488 FLOETTEETERNAL->FLOETTEMEGA, Fairy/Typeless, same stats; state.rs:676 | data correct (only Eternal Floette is accepted, same as SD); M1 only | S | high |
| steelixite | 1 | LOGIC | items.ts:6095 Steelix -> Steelix-Mega. Steel/Ground, 75/125/230/55/95/30, Sand Force (pokedex.ts:4318) | base_stats.rs:230 same data; state.rs:633 | data correct; M1 only | S | high |
| scraftinite | 1 | LOGIC | items.ts:5602-5613 Scrafty -> Scrafty-Mega. Dark/Fighting, 65/130/135/55/135/68, Intimidate (pokedex.ts:10478) | base_stats.rs:572 same data; state.rs:704 | data correct; M1 only | S | high |
| chandelurite | 1 | LOGIC | items.ts:776 Chandelure -> Chandelure-Mega. Ghost/Fire, 60/75/110/175/110/90, Infiltrator (pokedex.ts:11241) | base_stats.rs:554 same data; state.rs:686 | data correct; M1 only | S | high |
| medichamite | 1 | LOGIC | items.ts:3886 Medicham -> Medicham-Mega. Fighting/Psychic, 60/100/85/80/85/100, Pure Power (pokedex.ts:5913) | base_stats.rs:302 same data; state.rs:645 | data correct; M1 only | S | high |
| ampharosite | 1 | LOGIC | items.ts:253 Ampharos -> Ampharos-Mega. Electric/Dragon, 90/95/105/165/110/45, Mold Breaker (pokedex.ts:3911) | base_stats.rs:224 same data; state.rs:632 | data correct; M1 only | S | high |
| houndoominite | 1 | LOGIC | items.ts:2916 Houndoom -> Houndoom-Mega. Dark/Fire, 75/90/90/140/90/115, Solar Power (pokedex.ts:4694) | base_stats.rs:248 same data; state.rs:636 | data correct; M1 only | S | high |
| cameruptite | 1 | LOGIC | items.ts:747 Camerupt -> Camerupt-Mega. Fire/Ground, 70/120/100/145/105/20, Sheer Force (pokedex.ts:6149) | base_stats.rs:320 same data; state.rs:648 | data correct; M1 only | S | high |
| miracleseed | 1 | MATCH | items.ts:4153 Grass moves x4915/4096 | items.rs:1295 Grass BP x1.2 | none (K1-rounding, X1) | | high |

M1 (shared by all 34 Mega-stone LOGIC rows, plus Drampanite): Trick/Switcheroo checks only the target (choice_effects.rs:1587 `!defender.item_can_be_removed()`), not the user. Showdown (moves.ts:19873-19888) fails the move when the user's own stone cannot be taken. Fix: add `|| !attacker.item_can_be_removed()` (one condition).

## Notes

**Mega stone checks done for all 35 stones in the slice.** Each stone maps to the Showdown Mega species (all Showdown stones have a single base-species key; Floettite's key is Floette-Eternal, and the engine accepts only FLOETTEETERNAL, matching). Types, base stats and the Mega ability were compared against `pokedex.ts` (there is no champions pokedex override). The only mismatch is Drampa Sp. Def (114 vs 116). `Items::*` mega stones are all in `item_is_permanent` (state.rs:617-708), so Knock Off (choice_effects.rs:841), Thief (:859), Knock Off power bonus (:518), Pickpocket-style abilities (abilities.rs:753) and the target side of Trick are blocked for any holder. This is slightly broader than Showdown (a holder of the wrong species could lose it in SD), which cannot arise in legal teams. The engine keeps the stone after Mega Evolution and the protection does not depend on `id`, so a Mega-evolved holder stays protected, as in SD. K8 (Mega-turn speed uses pre-Mega speed) applies to every stone but was not repeated per row.

**X1 (cross-cutting, every item with an effect):** Klutz, Embargo and Magic Room are not modelled. `KLUTZ` exists only as an enum value (abilities.rs:125), `PokemonVolatileStatus::EMBARGO` (state.rs:181) is never read, and Magic Room has a move row but no field state. Showdown's `ignoringItem()` suppresses item effects in all three. Needs one helper checked in `items.rs`. Fix size M for the whole set. Not counted as a row verdict.

**X2 (cross-cutting, berries):** Unnerve (blocks the opposing side's berries), Ripen, Cheek Pouch and Harvest are not implemented for berries (only `GLUTTONY` is read, items.rs:755). Unburden's volatile is read in speed (generate_instructions.rs:2557) but nothing in items.rs sets it when an item is consumed. Belongs to the ability slices.

**N3 (Light Ball):** SD doubles the Atk/SpA stat, the engine doubles base power. Equal for ordinary damage. Two edge differences: a Pikachu using Body Press is not doubled in SD (Def-based) but is in the engine; confusion self-hit damage is doubled in SD but not in the engine. Both negligible.

**N4 (Dragon Scale, outside slice):** items.rs:1175 gives DRAGONSCALE the same x1.2 Dragon boost as Dragon Fang. In Showdown (gen 9) Dragon Scale has no battle effect and champions items.ts:226 marks it Past. Wrong but not legal in the format.

**N5 (Knock Off bonus and Sticky Hold, outside slice):** the Knock Off x1.5 uses `item_is_permanent` (choice_effects.rs:518) and so still applies against a Sticky Hold holder; Showdown's boost runs the TakeItem event, which Sticky Hold blocks during Knock Off. Medium confidence, other slice.

**N6 (type-boost rounding):** all type-boost items here use float x1.2 on base power; Showdown uses chainModify(4915/4096) on integer base power (K1 family). Not reported per row.

**N7 (resist-berry halving):** engine halves base power (items.rs:323) where Showdown halves final damage after type effectiveness, STAB and burn (K1 family); the multi-hit and fixed-damage consequences are under finding (d) and are real differences. Fixed-damage moves of the berry's type vs a weak holder also consume the berry in the engine (before_move has no fixed-damage exemption) but are not reduced.

**N8 (Big Root, King's Rock, Zoom Lens, Red Card):** all four need hooks the engine does not have yet (drain/Leech Seed heal multiplier, secondary append in `modify_choice`, accuracy order check using `first_move`, post-hit forced switch). They are not blocked by the K1-K10 core items except Zoom Lens and King's Rock, whose correctness also depends on K10 (flinch/accuracy branches) being modelled per branch.
