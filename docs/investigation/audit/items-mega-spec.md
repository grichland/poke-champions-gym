# Items and Mega Evolution: specification for the new resolver

Read-only spec for the coding stage that adds all 166 legal items and Mega Evolution to `engine/src/sim`. Written 17 Nov 2026 from Showdown commit `c0845f33` (the same commit as `SHOWDOWN_COMMIT` in `engine/src/sim/data/generated.rs:5`). Format `gen9championsbssregmc` (`generated.rs:6`; Showdown `config/formats.ts:275-279`: mod `champions`, ruleset Flat Rules + VGC Timer).

## 0. Conventions and sources

- **SD** = `C:\Users\user\Code\pokemon_champions\pcsa\.tools\vendor\pokemon-showdown`. Paths below are relative to it. `items.ts` = `data/items.ts`; `moves.ts` = `data/moves.ts`; `ch/items.ts`, `ch/scripts.ts`, `ch/abilities.ts` = `data/mods/champions/...`.
- "Line" numbers are Showdown line numbers at that commit. `block` = the first line of the item's object in `items.ts`.
- **CG audit** = `docs/investigation/audit/` (`items/items-01..03.md`, `second-read/items-b1,b2.md`, `species.md`, `mechanics-turn.md`). **Register** = `gaps/INDEX.md` (ids I*, SP*, SR*, N*, W*, F*, T*, C*, M*). **Usage** = `docs/investigation/coverage/03-coverage-items.csv` (pool share = share of team slots in the 488-roster training pool; live = share of 1,919 observed live builds).
- "Draw" = a call that consumes the random stream in Showdown (`randomChance`, `random`, `sample`, `shuffle`). The outcome tape must have one entry per draw, in Showdown's call order.
- Engine side: `engine/src/sim/hooks.rs` (Hook enum), `events.rs` (Ev enum, `Holder`), `residual.rs` (has an item holder with sub-order 8), `battle.rs` (`run_mega_evo` at line 1056, queue and re-sort), `state.rs` (`Mon`).
- Not legal means: not in the 166-row `ITEMS` table of `generated.rs:1021-1188` (that table equals Showdown's `Dex.mod('champions')` items with no `isNonstandard`, per `docs/investigation/coverage/03-coverage.md` section 1).

### 0.1 Counts

| group | items |
|---|---|
| Mega stones | 81 (they give 82 Mega forms: Meowsticite gives Meowstic-M-Mega and Meowstic-F-Mega) |
| Type-boost items | 18 |
| Damage and stat multipliers (Life Orb, Expert Belt, Muscle Band, Wise Glasses, Light Ball, Metronome) | 6 |
| Gem (Normal Gem only) | 1 |
| Resist berries (17 type berries + Chilan) | 18 |
| Status-cure berries (Cheri, Chesto, Pecha, Rawst, Aspear, Persim, Lum) | 7 |
| HP and PP berries (Sitrus, Oran, Leppa) | 3 |
| Survival (Focus Sash, Focus Band) | 2 |
| Residual and healing (Leftovers, Shell Bell, Big Root) | 3 |
| Contact and hit reactors (Rocky Helmet, Air Balloon) | 2 |
| Switch-forcing (Red Card, Eject Button) | 2 |
| Herbs (White Herb, Mental Herb) | 2 |
| Speed, priority, grounding (Choice Scarf, Quick Claw, Iron Ball) | 3 |
| Accuracy, crit, flinch (Wide Lens, Zoom Lens, Bright Powder, Scope Lens, Leek, King's Rock) | 6 |
| Duration extenders and trapping (Damp, Heat, Smooth, Icy Rock, Light Clay, Terrain Extender, Binding Band) | 7 |
| Terrain seeds | 4 |
| Trap immunity (Shed Shell) | 1 |
| **Total** | **166** (81 + 85) |

28 of the 166 are berries (`is_berry`, `generated.rs`). Exactly one item is a Choice item: **Choice Scarf** (`is_choice` true only there).

### 0.2 Items that are NOT legal (do not implement, do not assume)

Checked against the 166-item id list. None of these is in the table: Choice Band, Choice Specs, Assault Vest, Eviolite, Weakness Policy, Black Sludge, Flame Orb, Toxic Orb, Booster Energy, Covert Cloak, Clear Amulet, Safety Goggles, Heavy-Duty Boots, Throat Spray, Loaded Dice, Eject Pack, **Power Herb**, Mirror Herb, Utility Umbrella, Protective Pads, Punching Glove, Ability Shield, Room Service, Lagging Tail, Sticky Barb, Absorb Bulb, Cell Battery, Blunder Policy, Adrenaline Orb, Snowball, Berry Juice, Razor Claw, Razor Fang, Grip Claw, Float Stone, Destiny Knot, Macho Brace, Lucky Punch, Thick Club, Stick, Metal Powder, Quick Powder, Amulet Coin, Everstone, Dragon Scale (the engine still gives it a Dragon boost, `items-02.md` N4: wrong), all plates, all Z-crystals, all gems except Normal Gem, all pinch berries (Figy, Wiki, Mago, Aguav, Iapapa, Salac, Liechi, Petaya, Apicot, Ganlon, Starf, Lansat, Micle, Custap, Jaboca, Rowap, Kee, Maranga, Enigma), Orbs (Adamant, Lustrous, Griseous). Showdown's `ch/items.ts` marks them `isNonstandard: "Past"` (the champions mod flips 83 items to `null`, all other base items stay Past or Future). The register's F12 and I11 mention Razor Claw and Metronome-style entries as if legal: Razor Claw is not legal; **Metronome (item) is legal** (`metronome`, num 277). Note the *move* Metronome is a different thing.

Consequences for implementers: there is no Choice Band/Specs lock to build (only Scarf), no Assault Vest, no Eviolite, no Weakness Policy, no Booster Energy, no Toxic/Flame Orb, no Power Herb (so no two-turn-move skip item), no Eject Pack, no Utility Umbrella (weather effects always apply), no Safety Goggles (powder and weather damage are never blocked by an item), no Heavy-Duty Boots (hazards always hit), no Covert Cloak and no Shield Dust-style item (nothing blocks King's Rock's flinch except abilities), no Protective Pads (contact items always make contact), no Ability Shield.

The only item overridden with behaviour in the Champions mod is **Eject Button** (`ch/items.ts:262-278`); everything else in `ch/items.ts` is an `isNonstandard` flip (verified by grep: the only `on...` handler in the 1018-line file is at line 264).

## 1. The 166 items

### 1.1 Machinery every item shares (read this before the tables)

1. **Where an item's handler runs.** Showdown calls an item's `onX` through `runEvent` (collected for the holder in `findEventHandlers`, `sim/battle.ts:1060+`) or `singleEvent` (item as the effect). `onStart` of an item is run as part of the `SwitchIn` event when the item has no `onSwitchIn`/`onAnySwitchIn` (`getCallback`, `battle.ts:1028-1038`). This is why the seeds' `onSwitchInPriority: -1` (items.ts:2613, 4933, ...) applies to their `onStart`.
2. **Sort key of an item handler** (`resolvePriority`, `battle.ts:957-1022`): `order` (the `onXOrder` number; none means last), then `priority` (`onXPriority`, higher first), then the holder's speed (`pokemon.speed`, higher first), then `subOrder` (default for an Item effect = **8**, `battle.ts:976`; ability 7, condition 2, side condition 4, field/weather 5), then `effectOrder` (only for SwitchIn and RedirectTarget). Ties in all keys are broken with `prng.shuffle` (`speedSort`, `battle.ts:434-475`; one draw per element of the tied block, `prng.ts:146-158`). `Invulnerability`, `TryHit`, `DamagingHit`, `EntryHazard` are sorted left to right instead (`compareLeftToRightOrder`, `battle.ts:426`, selected at `battle.ts:796-797`), so a tie never draws there. The residual sub-order of Leftovers is `onResidualSubOrder: 4` (items.ts:3357), not 8.
3. **Suppression.** `runEvent` skips an item's handler when `holder.ignoringItem()` unless the event is `Start`, `SwitchIn` or `TakeItem` (`battle.ts:881-889`). `singleEvent` skips unless the event is `Start`, `TakeItem` or `SetAbility` (`battle.ts:614-617`). `ignoringItem()` (`pokemon.ts:900-907`) is true when: the Pokémon is not active (gen >= 5), it has the `embargo` volatile (the move Embargo is not legal, so never), Magic Room is up (`magicroom` pseudo-weather; the move is legal), or the holder's ability is Klutz and the item is not `ignoreKlutz` (**no legal item has `ignoreKlutz`**, `generated.rs`: all `ignore_klutz: false`). `hasItem()` (`pokemon.ts:1916-1923`) includes `!ignoringItem()`. Raw `pokemon.item` is used, without the suppression test, by: Acrobatics, Poltergeist, Covet, Thief, Magician, Recycle, `lastItem` bookkeeping, and `Pokemon.isGrounded` uses `ignoringItem()` (`pokemon.ts:2173`). When Magic Room starts, every active Pokémon gets `singleEvent('End', item)` (moves.ts:10781-10783, Magic Room `onFieldStart`).
4. **Primitives** (`pokemon.ts`): `eatItem(force?)` line 1789, `useItem()` line 1832, `takeItem(source?)` line 1872, `setItem()` line 1889, `clearItem()` line 1925.
   - `eatItem`: needs `item`, `hp > 0` and `isActive`. Events in order: `runEvent('UseItem')`; unless `force`, `runEvent('TryEatItem')` (the item's own `onTryEatItem`, plus **Unnerve**: `onFoeTryEatItem` returns false while the foe's Unnerve is active, abilities.ts:5260-5276; Berserk's `onTryEatItem`, Ripen's); then `singleEvent('Eat')` (the item's `onEat`), `runEvent('EatItem')` (Cheek Pouch heal 1/3, Cud Chew), clears `item`, sets `lastItem`, `usedItemThisTurn`, `ateBerry = true`, then `runEvent('AfterUseItem')` (Unburden: abilities.ts:5237-5258).
   - `useItem`: needs `item`, `hp > 0` unless the item is a gem, and `isActive`. `runEvent('UseItem')`; applies `item.boosts` through `battle.boost` (seeds; the item is consumed even if the boost cannot apply, because the result of the boost is not tested); `singleEvent('Use')` (White Herb `onUse`); clears the item, sets `lastItem`, `usedItemThisTurn`; `runEvent('AfterUseItem')`. Does NOT set `ateBerry`.
   - `takeItem`: `runEvent('TakeItem')` (collects `onTakeItem` of the item, of the holder's ability (Sticky Hold) and others); if truthy, clears the item and runs `singleEvent('End')` and `runEvent('AfterTakeItem')` (Unburden, abilities.ts:5242). It does not set `lastItem` (so Knock Off, Thief, Trick leave nothing for Recycle). Sticky Hold blocks only when `source !== holder` or the active move is Knock Off (abilities.ts:4623-4631).
   - `setItem`: needs `hp` and `isActive`; sets the new item, `singleEvent('End', old)`, `singleEvent('Start', new)` (so Air Balloon's and Metronome's `onStart` run on receipt, and a Choice lock is reset: items.ts:1001-1006).
5. **Update event.** Berries and herbs with `onUpdate` fire when `eachEvent('Update')` runs: after every action (`battle.ts:2925-2926`, gen >= 5, skipped for the `start` action and before an `instaswitch`), after each hit of a multi-hit move (`ch/scripts.ts:536`), once more after the hit loop before `AfterMoveSecondary` (`ch/scripts.ts:573`), after a Pursuit activation (`battle-actions.ts:67`), after the Weather event (`battle.ts:479`, gen >= 7). `eachEvent` sorts the active Pokémon by speed with `speedSort`: **a speed tie between the two actives draws a shuffle every time Update runs**, whether or not any handler fires. The number of Update calls must therefore match Showdown even in battles with no items.
6. **Arithmetic.** `chainModify([n, 4096])` multiplies the event's running modifier: `modifier = ((modifier*4096 trunc * n + 2048) >> 12) / 4096`, and `n = trunc(num*4096/den)` (`battle.ts:2337-2346`); the final value is `modify(value, modifier)` = `trunc((trunc(value*m) + 2048 - 1) / 4096)` (`battle.ts:2348-2358`, applied when the event ends, `battle.ts:939`). `Ctx::chain_modify` in `events.rs` already does this. Fractions: Showdown's `heal(x)`: `if (x && x <= 1) x = 1; x = trunc(x)`, then `TryHeal` event (a number relay, so Big Root's chain applies at the end of the event), then no-op at full HP (`battle.ts:2277-2285`). `damage(x)`: `clampIntRange(x, 1)` (floor, min 1) before and after the `Damage` event (`battle.ts:2130` and `2145`). Drain is `Math.round(damage * n/d)`, recoil `Math.round` (`battle.ts:2188`, `battle-actions.ts:1435`). `maxhp/16` etc. below are `baseMaxhp / 16` as a float passed to `heal` or `damage`, so they truncate with a minimum of 1.
7. **Damage chain for items.** Champions overrides `modifyDamage` (`ch/scripts.ts:196-312`): base +2, spread/Parental Bond, weather, crit x1.5, `randomizer` (the 85..100 roll; a draw), STAB, type effectiveness (doubling or halving with `tr`), burn x0.5, then `runEvent('ModifyDamage')` (final modifiers: Life Orb, Expert Belt, Metronome, the target's `onSourceModifyDamage` resist berries and Multiscale-like abilities), then the Protect-bypass quarter, then `tr(…, 16)`. Item `onBasePower` run in `BasePower` (`battle-actions.ts:1704`), `onModifyAtk/SpA` in `battle-actions.ts:1762`.
8. **After a damaging move**, in this order (`battle-actions.ts`, Champions override of the hit loop `ch/scripts.ts:426-585`): per hit `DamagingHit` (Rocky Helmet order 2, `battle.ts:797`) and `Update`; after the loop recoil (`applyRecoilDamage`, `ch/scripts.ts:551-553`), `Update`, then **`AfterMoveSecondary`** (`afterMoveSecondaryEvent`, `ch/scripts.ts:575`; Eject Button priority 2, Red Card, Pickpocket, Berserk...), then `EmergencyExit`; then back in `useMove`, `AfterMoveSecondarySelf` (Life Orb priority 0, Shell Bell priority -1; `battle-actions.ts:600-606`), then `AfterMove` (White Herb `onAnyAfterMove`, `battle-actions.ts:368-369`). **The Champions override of the hit loop calls `afterMoveSecondaryEvent` without the base game's `!suppressingSecondaries()` test** (compare base `battle-actions.ts:1064`), so Sheer Force does not suppress Eject Button, Red Card or Pickpocket in this mod; it still suppresses `AfterMoveSecondarySelf` (Life Orb, Shell Bell: `battle-actions.ts:600`). Unsure whether that is intended by the mod authors; it is what the code does.
9. **Draws that items add** (the tape must carry them, with these exact conditions):
   - Quick Claw: `randomChance(1,5)` once per move action when the queue is built, from `runEvent('FractionalPriority')` (`battle-queue.ts:249`, items.ts:5015-5021). Skipped when the move is Status and the holder has Mycelium Might (items.ts:5016) or when the relay priority is above 0 (never: the relay starts at 0).
   - Focus Band: `randomChance(1,10)` on **every** `Damage` event of the holder, evaluated before `damage >= hp` and before the Move test (items.ts:2272). So it draws for poison, burn, sand, hazards, recoil, Life Orb, Rocky Helmet damage and for non-lethal hits.
   - King's Rock: pushes a `{chance: 10, volatileStatus: 'flinch'}` secondary, so `secondaries()` draws `random(100)` for it on each hit (`battle-actions.ts:1399`: the roll is drawn for every secondary, even one with no chance).
   - Resist berries, Sitrus, etc.: no draw.
   - Red Card: after the forced switch flag is set, `dragIn` draws `sample(possibleSwitches)` (`battle.ts getRandomSwitchable`; `prng.ts:128-139`) **even when there is only one candidate** (`random(1)` still advances the generator).
   - Eject Button: no draw (the replacement is the player's choice).
   - Speed ties from `Update`, `queue.sort`, `SwitchIn` (see 1.1 item 2 and 5).
   - Metronome (item), Leftovers, Shell Bell, Life Orb, Light Ball and the type boosters: no draw.

### 1.2 Per-item tables

Columns: use rank is by pool share (rank 1 = Leftovers; "pool 0" = no slot in the 2,886-slot pool; items used only by the live list or the gold team still show their live share); live share; `gold` = on the gold team. "handlers with order numbers" is `ITEMS[...]`.handlers and `.nums` in `generated.rs` (Showdown's own handler names; `nums` are the `onXOrder/Priority/SubOrder` values). Fling BP is Showdown's `item.fling.basePower` (section 2).

#### Type-boost (x4915/4096 on base power, onBasePowerPriority 15) (18)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Black Belt (`blackbelt`) | #84 / 0.05 / 0.05  | items.ts:514 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Black Glasses (`blackglasses`) | #23 / 0.94 / 0.83  | items.ts:529 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Charcoal (`charcoal`) | pool 0 / 0.00 / 0.00  | items.ts:785 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Dragon Fang (`dragonfang`) | #97 / 0.02 / 0.05  | items.ts:1494 | onBasePower ; onBasePowerPriority=15.0 | 70 |
| Fairy Feather (`fairyfeather`) | #81 / 0.06 / 0.05  | items.ts:1939 | onBasePower ; onBasePowerPriority=15.0 | 10 |
| Hard Stone (`hardstone`) | pool 0 / 0.00 / 0.00  | items.ts:2786 | onBasePower ; onBasePowerPriority=15.0 | 100 |
| Magnet (`magnet`) | pool 0 / 0.00 / 0.00  | items.ts:3707 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Metal Coat (`metalcoat`) | pool 0 / 0.00 / 0.00  | items.ts:3979 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Miracle Seed (`miracleseed`) | pool 0 / 0.00 / 0.00  | items.ts:4153 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Mystic Water (`mysticwater`) | #62 / 0.10 / 0.10  | items.ts:4272 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Never-Melt Ice (`nevermeltice`) | #39 / 0.48 / 0.31  | items.ts:4314 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Poison Barb (`poisonbarb`) | pool 0 / 0.00 / 0.00  | items.ts:4670 | onBasePower ; onBasePowerPriority=15.0 | 70 |
| Sharp Beak (`sharpbeak`) | pool 0 / 0.00 / 0.00  | items.ts:5630 | onBasePower ; onBasePowerPriority=15.0 | 50 |
| Silk Scarf (`silkscarf`) | pool 0 / 0.00 / 0.00  | items.ts:5738 | onBasePower ; onBasePowerPriority=15.0 | 10 |
| Silver Powder (`silverpowder`) | pool 0 / 0.00 / 0.00  | items.ts:5753 | onBasePower ; onBasePowerPriority=15.0 | 10 |
| Soft Sand (`softsand`) | pool 0 / 0.00 / 0.00  | items.ts:5881 | onBasePower ; onBasePowerPriority=15.0 | 10 |
| Spell Tag (`spelltag`) | #57 / 0.15 / 0.10  | items.ts:5926 | onBasePower ; onBasePowerPriority=15.0 | 30 |
| Twisted Spoon (`twistedspoon`) | pool 0 / 0.00 / 0.00  | items.ts:7401 | onBasePower ; onBasePowerPriority=15.0 | 30 |

#### Damage and stat multipliers (6)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Life Orb (`lifeorb`) | #2 / 12.42 / 12.82  | items.ts:3422 | onAfterMoveSecondarySelf, onModifyDamage | 30 |
| Expert Belt (`expertbelt`) | pool 0 / 0.00 / 0.00  | items.ts:1913 | onModifyDamage | 10 |
| Muscle Band (`muscleband`) | pool 0 / 0.00 / 0.00  | items.ts:4257 | onBasePower ; onBasePowerPriority=16.0 | 10 |
| Wise Glasses (`wiseglasses`) | pool 0 / 0.00 / 0.00  | items.ts:7778 | onBasePower ; onBasePowerPriority=16.0 | 10 |
| Light Ball (`lightball`) | #96 / 0.03 / 0.00  | items.ts:3439 | onModifyAtk, onModifySpA ; onModifyAtkPriority=1.0, onModifySpAPriority=1.0 | 30 |
| Metronome (`metronome`) | pool 0 / 0.00 / 0.00  | items.ts:4011 | onStart | 30 |

#### Gem (1)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Normal Gem (`normalgem`) | #28 / 0.72 / 0.57  | items.ts:4342 | onSourceTryPrimaryHit | cannot be flung |

#### Resist berries (halve one super-effective hit) (18)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Occa Berry (`occaberry`) | pool 0 / 0.00 / 0.00  | items.ts:4365 | onEat, onSourceModifyDamage | 10 |
| Passho Berry (`passhoberry`) | pool 0 / 0.00 / 0.00  | items.ts:4467 | onEat, onSourceModifyDamage | 10 |
| Wacan Berry (`wacanberry`) | pool 0 / 0.00 / 0.00  | items.ts:7517 | onEat, onSourceModifyDamage | 10 |
| Rindo Berry (`rindoberry`) | pool 0 / 0.00 / 0.00  | items.ts:5227 | onEat, onSourceModifyDamage | 10 |
| Yache Berry (`yacheberry`) | pool 0 / 0.00 / 0.00  | items.ts:7793 | onEat, onSourceModifyDamage | 10 |
| Chople Berry (`chopleberry`) | #94 / 0.03 / 0.00  | items.ts:1042 | onEat, onSourceModifyDamage | 10 |
| Kebia Berry (`kebiaberry`) | #76 / 0.07 / 0.05  | items.ts:3165 | onEat, onSourceModifyDamage | 10 |
| Shuca Berry (`shucaberry`) | #29 / 0.70 / 0.57  | items.ts:5714 | onEat, onSourceModifyDamage | 10 |
| Coba Berry (`cobaberry`) | pool 0 / 0.00 / 0.00  | items.ts:1121 | onEat, onSourceModifyDamage | 10 |
| Payapa Berry (`payapaberry`) | pool 0 / 0.00 / 0.00  | items.ts:4491 | onEat, onSourceModifyDamage | 10 |
| Tanga Berry (`tangaberry`) | pool 0 / 0.00 / 0.00  | items.ts:6246 | onEat, onSourceModifyDamage | 10 |
| Charti Berry (`chartiberry`) | pool 0 / 0.00 / 0.00  | items.ts:824 | onEat, onSourceModifyDamage | 10 |
| Kasib Berry (`kasibberry`) | pool 0 / 0.00 / 0.00  | items.ts:3141 | onEat, onSourceModifyDamage | 10 |
| Haban Berry (`habanberry`) | pool 0 / 0.00 / 0.00  | items.ts:2762 | onEat, onSourceModifyDamage | 10 |
| Babiri Berry (`babiriberry`) | pool 0 / 0.00 / 0.00  | items.ts:355 | onEat, onSourceModifyDamage | 10 |
| Chilan Berry (`chilanberry`) | pool 0 / 0.00 / 0.00  | items.ts:910 | onEat, onSourceModifyDamage | 10 |
| Colbur Berry (`colburberry`) | #100 / 0.02 / 0.00  | items.ts:1145 | onEat, onSourceModifyDamage | 10 |
| Roseli Berry (`roseliberry`) | pool 0 / 0.00 / 0.00  | items.ts:5384 | onEat, onSourceModifyDamage | 10 |

#### Status-cure berries (7)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Cheri Berry (`cheriberry`) | pool 0 / 0.00 / 0.00  | items.ts:848 | onEat, onUpdate | 10 |
| Chesto Berry (`chestoberry`) | pool 0 / 0.00 / 0.00  | items.ts:889 | onEat, onUpdate | 10 |
| Pecha Berry (`pechaberry`) | #79 / 0.07 / 0.00  | items.ts:4521 | onEat, onUpdate | 10 |
| Rawst Berry (`rawstberry`) | pool 0 / 0.00 / 0.00  | items.ts:5092 | onEat, onUpdate | 10 |
| Aspear Berry (`aspearberry`) | pool 0 / 0.00 / 0.00  | items.ts:292 | onEat, onUpdate | 10 |
| Persim Berry (`persimberry`) | pool 0 / 0.00 / 0.00  | items.ts:4542 | onEat, onUpdate | 10 |
| Lum Berry (`lumberry`) | #38 / 0.48 / 0.31  | items.ts:3555 | onAfterSetStatus, onEat, onUpdate ; onAfterSetStatusPriority=-1.0 | 10 |

#### HP and PP berries (3)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Sitrus Berry (`sitrusberry`) | #5 / 4.80 / 4.64  | items.ts:5768 | onEat, onTryEatItem, onUpdate | 10 |
| Oran Berry (`oranberry`) | pool 0 / 0.00 / 0.00  | items.ts:4415 | onEat, onTryEatItem, onUpdate | 10 |
| Leppa Berry (`leppaberry`) | pool 0 / 0.00 / 0.00  | items.ts:3370 | onEat, onUpdate | 10 |

#### Survival (2)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Focus Sash (`focussash`) | #3 / 5.87 / 6.20 gold | items.ts:2281 | onDamage ; onDamagePriority=-40.0 | 10 |
| Focus Band (`focusband`) | pool 0 / 0.00 / 0.00  | items.ts:2265 | onDamage ; onDamagePriority=-40.0 | 10 |

#### Residual and healing (3)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Leftovers (`leftovers`) | #1 / 19.14 / 18.08 gold | items.ts:3350 | onResidual ; onResidualOrder=5.0, onResidualSubOrder=4.0 | 10 |
| Shell Bell (`shellbell`) | #55 / 0.15 / 0.26  | items.ts:5674 | onAfterMoveSecondarySelf ; onAfterMoveSecondarySelfPriority=-1.0 | 30 |
| Big Root (`bigroot`) | #99 / 0.02 / 0.00  | items.ts:482 | onTryHeal ; onTryHealPriority=1.0 | 10 |

#### Contact and hit reactors (2)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Rocky Helmet (`rockyhelmet`) | #6 / 4.56 / 4.59  | items.ts:5319 | onDamagingHit ; onDamagingHitOrder=2.0 | 60 |
| Air Balloon (`airballoon`) | #18 / 1.22 / 0.83 gold | items.ts:185 | onAfterSubDamage, onDamagingHit, onStart | 10 |

#### Switch-forcing (2)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Red Card (`redcard`) | #72 / 0.07 / 0.10  | items.ts:5170 | onAfterMoveSecondary | 10 |
| Eject Button (`ejectbutton`) | #56 / 0.15 / 0.31  | items.ts:1692 | onAfterMoveSecondary ; onAfterMoveSecondaryPriority=2.0 | 30 |

#### Herbs (2)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| White Herb (`whiteherb`) | #51 / 0.23 / 0.42  | items.ts:7682 | onAnyAfterMega, onAnyAfterMove, onAnySwitchIn, onResidual, onStart, onUse ; onAnySwitchInPriority=-2.0, onResidualOrder=29.0 | 10 |
| Mental Herb (`mentalherb`) | pool 0 / 0.00 / 0.00  | items.ts:3907 | onUpdate | 10 |

#### Speed, priority, grounding (3)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Choice Scarf (`choicescarf`) | #4 / 5.45 / 6.15 gold | items.ts:995 | onModifyMove, onModifySpe, onStart | 10 |
| Quick Claw (`quickclaw`) | #26 / 0.82 / 0.78  | items.ts:5013 | onFractionalPriority ; onFractionalPriorityPriority=-2.0 | 80 |
| Iron Ball (`ironball`) | pool 0 / 0.00 / 0.00  | items.ts:3062 | onEffectiveness, onModifySpe | 130 |

#### Accuracy, crit, flinch (6)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Wide Lens (`widelens`) | pool 0 / 0.00 / 0.00  | items.ts:7737 | onSourceModifyAccuracy ; onSourceModifyAccuracyPriority=-2.0 | 10 |
| Zoom Lens (`zoomlens`) | #74 / 0.07 / 0.10  | items.ts:7849 | onSourceModifyAccuracy ; onSourceModifyAccuracyPriority=-2.0 | 10 |
| Bright Powder (`brightpowder`) | pool 0 / 0.00 / 0.00  | items.ts:671 | onModifyAccuracy ; onModifyAccuracyPriority=-2.0 | 10 |
| Scope Lens (`scopelens`) | #58 / 0.14 / 0.31  | items.ts:5578 | onModifyCritRatio | 30 |
| Leek (`leek`) | #77 / 0.07 / 0.05  | items.ts:3334 | onModifyCritRatio | 60 |
| King's Rock (`kingsrock`) | #64 / 0.10 / 0.05  | items.ts:3221 | onModifyMove ; onModifyMovePriority=-1.0 | 30 |

#### Duration extenders and trapping (7)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Damp Rock (`damprock`) | #33 / 0.60 / 0.83  | items.ts:1273 | (none) | 60 |
| Heat Rock (`heatrock`) | #66 / 0.09 / 0.16  | items.ts:2853 | (none) | 60 |
| Smooth Rock (`smoothrock`) | #46 / 0.30 / 0.10  | items.ts:5844 | (none) | 10 |
| Icy Rock (`icyrock`) | pool 0 / 0.00 / 0.00  | items.ts:3022 | (none) | 40 |
| Light Clay (`lightclay`) | #12 / 1.82 / 1.93  | items.ts:3462 | (none) | 30 |
| Terrain Extender (`terrainextender`) | #16 / 1.31 / 1.62  | items.ts:6307 | (none) | 60 |
| Binding Band (`bindingband`) | pool 0 / 0.00 / 0.00  | items.ts:498 | (none) | 30 |

#### Terrain seeds (4)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Electric Seed (`electricseed`) | #101 / 0.02 / 0.00  | items.ts:1811 | onStart, onTerrainChange ; onSwitchInPriority=-1.0 | 10 |
| Grassy Seed (`grassyseed`) | pool 0 / 0.00 / 0.00  | items.ts:2607 | onStart, onTerrainChange ; onSwitchInPriority=-1.0 | 10 |
| Misty Seed (`mistyseed`) | pool 0 / 0.00 / 0.00  | items.ts:4218 | onStart, onTerrainChange ; onSwitchInPriority=-1.0 | 10 |
| Psychic Seed (`psychicseed`) | #102 / 0.02 / 0.00  | items.ts:4927 | onStart, onTerrainChange ; onSwitchInPriority=-1.0 | 10 |

#### Trap immunity (1)

| item | use rank / pool% / live% | Showdown block | handlers with order numbers | Fling BP |
|---|---|---|---|---|
| Shed Shell (`shedshell`) | pool 0 / 0.00 / 0.00  | items.ts:5657 | onMaybeTrapPokemon, onTrapPokemon ; onMaybeTrapPokemonPriority=-10.0, onTrapPokemonPriority=-10.0 | 10 |

### 1.3 Rules by group (exact numbers, consumption, draws, Champions overrides)

No item except Eject Button differs between base Gen 9 and Champions (`ch/items.ts`). So every rule below is base `items.ts`. Champions differences that touch items come from `ch/scripts.ts` (Mega, damage formula, hit loop, speed) and `ch/abilities.ts` (Emergency Exit, Berserk, Healer, Unseen Fist, Run Away, Natural Cure...).

**Type-boost items (18).** `onBasePowerPriority: 15`; when `move.type === <type>` (the move's type after type-changing abilities: Pixilate etc. run in `ModifyType`, earlier) `chainModify([4915, 4096])` on **base power** (the BasePower event, `battle-actions.ts:1704`), e.g. Black Glasses items.ts:535-540. Types: Black Belt Fighting, Black Glasses Dark, Charcoal Fire, Dragon Fang Dragon, Fairy Feather Fairy, Hard Stone Rock, Magnet Electric, Metal Coat Steel, Miracle Seed Grass, Mystic Water Water, Never-Melt Ice Ice, Poison Barb Poison, Sharp Beak Flying, Silk Scarf Normal, Silver Powder Bug, Soft Sand Ground, Spell Tag Ghost, Twisted Spoon Psychic. Not consumed. No draw. Poison Barb's Fling also poisons (items.ts:4673-4676). The engine's `x1.2` float is wrong by rounding only (register F1); implement the 4096 chain.

**Muscle Band, Wise Glasses.** `onBasePowerPriority: 16`, Physical (Muscle Band) or Special (Wise Glasses) `chainModify([4505, 4096])` (items.ts:4263-4268, 7784-7789). Priority 16 means they run before the type boosters (15); the chain is multiplicative so order matters only through rounding of the chained modifier.

**Expert Belt.** `onModifyDamage` (no priority number), `typeMod > 0` of the hit (`target.getMoveHitData(move).typeMod`, set in `modifyDamage` just before the ModifyDamage event) gives `chainModify([4915, 4096])` (items.ts:1919-1923). It is in the final-damage chain, not base power (the engine's BP x1.2 is wrong, `items-03.md`).

**Life Orb.** `onModifyDamage` `chainModify([5324, 4096])` (items.ts:3428-3430). `onAfterMoveSecondarySelf(source, target, move)`: when `source !== target`, the move is not Status and `!source.forceSwitchFlag`, the holder takes `baseMaxhp/10` (truncate, min 1) as Item damage (items.ts:3431-3435). Runs inside `useMove` after the move has not failed (`battle-actions.ts:600-606`), once per move (not per hit), **not** when the holder's Sheer Force suppressed secondaries (`battle-actions.ts:600`), not for `futuremove`. Magic Guard blocks it (damage effect is an Item). No draw. It is the Damage event, so a holder's Focus Band draws for it.

**Light Ball.** `onModifyAtk` and `onModifySpA` priority 1: `baseSpecies.baseSpecies === 'Pikachu'` gives `chainModify(2)` on the stat (items.ts:3446-3457). It doubles the stat, not the power, so Foul Play (uses the target's Atk) and Body Press (Def) are not doubled and confusion self-damage is (the register's SR-29 documents the engine doing it wrongly). Fling: BP 30 and paralysis (items.ts:3442-3445).

**Metronome (item).** `onStart` adds a `metronome` volatile; volatile `onTryMovePriority: -2` counts consecutive uses: same move as last and `pokemon.moveLastTurnResult` true -> count+1; if a `twoturnmove` volatile is active: count = 1 when the move changed else +1; else 0; moves that call moves (`callsMove`) are skipped. `onModifyDamage` multiplies by `[4096, 4915, 5734, 6553, 7372, 8192][min(count, 5)] / 4096` (items.ts:4017-4051). Removed (volatile deleted) when the holder no longer has the item (items.ts:4027-4030). Used 0 in pool; implement because it is legal.

**Normal Gem.** `onSourceTryPrimaryHit(target, source, move)`: not when `target === source`, not for Status moves, not for `pledgecombo`; when `move.type === 'Normal'` and `source.useItem()` succeeds, adds the `gem` volatile to the user (items.ts:4346-4351), and the volatile gives `onBasePower` priority 14 `chainModify([5325, 4096])` for that move (`conditions.ts:467-474`, duration 1, `affectsFainted`). Consumed at TryPrimaryHit time, i.e. only when the move reaches the target and survives Protect/accuracy/type immunity (TryPrimaryHit is after `TryHit`, accuracy and the immunity step, `battle-actions.ts:1199`), so Protect, a miss or Normal-immune Ghost do not consume it; a Substitute hit does consume (the event comes before the Substitute handler? **unsure**: Substitute's own handler is `onTryPrimaryHit` in `moves.ts:18364` region; the order between the item's `onSourceTryPrimaryHit` and the Substitute's `onTryPrimaryHit` is by the sort keys above, not read). Thief, Covet and Magician check `source.volatiles['gem']` before stealing (moves.ts:3109, 19308; abilities.ts:2478-2485).

**Resist berries (18).** `onSourceModifyDamage(damage, source, target, move)`: when `move.type === T` and `target.getMoveHitData(move).typeMod > 0` (Chilan: any Normal move, no effectiveness test, items.ts:918-929), and the target is not behind a Substitute (`hitSub = target.volatiles['substitute'] && !move.flags['bypasssub'] && !(move.infiltrates && gen >= 6)`; if `hitSub` return nothing), then `target.eatItem()`; if the berry was eaten, `chainModify(0.5)` (= 2048/4096 on the final damage), items.ts:5722-5733 (Shuca; the 17 others are the same text). `onEat() {}` does nothing else. Types: Occa Fire, Passho Water, Wacan Electric, Rindo Grass, Yache Ice, Chople Fighting, Kebia Poison, Shuca Ground, Coba Flying, Payapa Psychic, Tanga Bug, Charti Rock, Kasib Ghost, Haban Dragon, Babiri Steel, Colbur Dark, Roseli Fairy, Chilan Normal. Consumed only on a damaging hit that reaches the target's HP (not Status moves: the event is ModifyDamage, which only runs inside `modifyDamage`); the berry is eaten on the **first** hit of a multi-hit move and later hits are full damage (it is gone). Fixed-damage moves do not run `modifyDamage`, so the berry is not eaten. It is blocked by **Unnerve** on the foe (`onFoeTryEatItem`, abilities.ts:5272) and doubled by **Ripen** (abilities.ts:3843-3873: `onEatItem` records `berryWeaken`; then a further `chainModify(0.5)` in `onSourceModifyDamage` priority -1 -> 1/4). Eating is subject to `UseItem` and `TryEatItem` events. Typeless gate: the effectiveness test uses `typeMod` of the final move type against the holder's current types (not the move's base type). Cheek Pouch heals 1/3 on `EatItem`. No draw. Register I2 and I3 are the engine's early-consumption faults (the berry must be consumed inside the damage step, not before accuracy).

**Status-cure berries (7).** All use `onUpdate` and `onEat` (the cure) except Lum:
- Cheri: `status === 'par'` -> `eatItem()`; `onEat` cures if still par (items.ts:856-865). Chesto: `slp` (889-909). Pecha: `psn` or `tox` (4529-4538). Rawst: `brn` (5100-5109). Aspear: `frz` (300-309). Persim: `volatiles['confusion']` -> eat; `onEat` removes confusion (4550-4557).
- Lum: `onAfterSetStatusPriority: -1`, `onAfterSetStatus(status, pokemon)` -> `eatItem()` (items.ts:3563-3566, so it triggers **at once** when any status is set: by a move, an ability such as Static, or Toxic Spikes); `onUpdate` -> eat if `pokemon.status || volatiles['confusion']` (3567-3571); `onEat` cures the status and removes confusion (3572-3575). Confusion is cured at the next Update, not at the instant it is applied (no `AfterSetStatus` for volatiles).
- Eaten only at Update points (after every action, after each hit, ...), subject to Unnerve. Cured status is removed before end-of-turn residual, so Pecha/Rawst prevent the same turn's poison/burn tick if an Update ran after the status was applied (every action ends with Update, so poison from a move is cured before the residual action). Sleep: Chesto cures `slp` at the next Update, which for a sleeping holder normally means before it ever fails to move (Update runs after the move that put it to sleep).
- Register I7, N6: the engine cures late and ignores confusion.

**Sitrus.** `onUpdate`: `hp <= maxhp/2` (float compare, so exactly half qualifies) -> `eatItem()`; `onTryEatItem`: `runEvent('TryHeal', pokemon, null, effect, baseMaxhp/4)` must be truthy (Heal Block, Psychic Noise's block, return false); `onEat`: `heal(baseMaxhp/4)` (truncate, min 1; items.ts:5776-5786). **Gluttony does not matter** (it only affects the pinch berries, which are not legal) so Sitrus always triggers at 1/2; the engine's Gluttony arm and "1/4 before the holder's own move" rule are wrong. Eaten immediately after the damaging event through the next Update (after each hit of a multi-hit move, `ch/scripts.ts:536`).
**Oran.** Same trigger (`hp <= maxhp/2`); `onTryEatItem` `TryHeal` with 10; heals fixed `10` (items.ts:4423-4433). Not used in the pool.
**Leppa.** `onUpdate`: any move slot with `pp === 0` and `hp > 0` -> eat; `onEat` picks the first slot with `pp === 0` (else the first with `pp < maxpp`) and adds 10 PP (20 with Ripen), capped at maxpp (items.ts:3378-3391). Eaten after the move that took the last PP resolves (next Update). Not used in the pool.

**Focus Sash.** `onDamagePriority: -40`; `onDamage(damage, target, source, effect)`: if `target.hp === target.maxhp && damage >= target.hp && effect.effectType === 'Move'` and `target.useItem()` succeeds, return `target.hp - 1` (items.ts:2287-2294). Works in the Damage event after Sturdy (`onDamagePriority: -30`, abilities.ts: Sturdy block, same condition; higher priority runs first, so a Sturdy holder keeps the Sash: Sturdy returns `hp-1`, then the Sash sees `damage < hp`); consumed; per-hit for multi-hit: only the first hit can be saved (HP is then below max). Does nothing against residual damage (effect is not a Move). **No draw.** Register I1: the engine reduces every full-HP hit by 1 and never consumes.
**Focus Band.** `onDamagePriority: -40`; `if (randomChance(1,10) && damage >= target.hp && effect.effectType === 'Move')` return `target.hp - 1`, item **not consumed** (items.ts:2271-2277). Draw on every Damage event as described in 1.1 item 9. Used 0 in pool.

**Leftovers.** `onResidualOrder: 5`, `onResidualSubOrder: 4`, `heal(baseMaxhp/16)` (items.ts:3356-3360). Fails (no message) at full HP; blocked by Heal Block via `TryHeal`; Ripen announces. Runs before Leech Seed (8), poison (9), burn (10) in the residual action (`mechanics-turn.md`, register T1). Relative order to weather residual (1), Future Sight, Wish (4) is by `order`, then speed ties shuffled.
**Shell Bell.** `onAfterMoveSecondarySelfPriority: -1`; `if (move.totalDamage && !pokemon.forceSwitchFlag) heal(move.totalDamage / 8, pokemon)` (items.ts:5680-5685). `totalDamage` is the sum over hits and targets of damage actually dealt to HP (Substitute damage counts 0: `damage[i] = md === true || !md ? 0 : md`, `ch/scripts.ts:526-534`). One heal per move, truncated, min 1. Does not replace the move's own drain (register I6).
**Big Root.** `onTryHealPriority: 1`; `onTryHeal(damage, target, source, effect)`: if `effect.id` is one of `drain`, `leechseed`, `ingrain`, `aquaring`, `strengthsap` -> `chainModify([5324, 4096])` of the heal amount (items.ts:488-494). The amount is the already rounded drain (`Math.round`, `battle.ts:2188`), then x5324/4096 and truncated by `modify`.

**Rocky Helmet.** `onDamagingHitOrder: 2` (so after abilities with order 1: Rough Skin, Iron Barbs, ... abilities.ts:79, 1189, 2142, 2226, 3940, 5513): if `checkMoveMakesContact(move, source, target)`, `damage(source.baseMaxhp/6, source, target)` (items.ts:5325-5330). Per hit of a multi-hit move; not on a Substitute hit (DamagingHit needs damage to the Pokémon); Magic Guard blocks; Long Reach and (not legal) Protective Pads make `checkMoveMakesContact` false. Interacts with Focus Band (draw) and Focus Sash (effect is not a Move, no).
**Air Balloon.** `onStart`: if `!target.ignoringItem() && !field.getPseudoWeather('gravity')` show the balloon (items.ts:191-194). The holder is airborne through `isGrounded` (`pokemon.ts:2169-2180`: `item !== 'airballoon'` is the last test, after Gravity, Ingrain, Smack Down, Iron Ball, Flying type, Levitate/Eelevate, Magnet Rise, Telekinesis). `onDamagingHit` pops it: `item = ''`, clears item state, `runEvent('AfterUseItem')` (Unburden) with no `useItem` (items.ts:197-202), so it does not set `lastItem`... **not read**: whether the pop sets `lastItem`; the code shown sets `target.item = ''` directly. `onAfterSubDamage` pops it when a move hits the holder's Substitute (items.ts:203-211; the event is run from `moves.ts:18364-18365`). A Ground move that is blocked by the balloon never "hits", so it does not pop. No draw.

**Red Card.** `onAfterMoveSecondary(target, source, move)`: when `source !== target`, both alive, move not Status; `source.isActive && canSwitch(source.side) && !source.forceSwitchFlag && !target.forceSwitchFlag`; `target.useItem(source)` (consumes, announces); then `runEvent('DragOut', source, target, move)` (Suction Cups, Guard Dog... block) and if truthy `source.forceSwitchFlag = true` (items.ts:5176-5188). No priority number, so it sorts with the other `AfterMoveSecondary` handlers by speed. The drag happens in `runAction`'s phazing step after the action (`battle.ts:2864-2871`) via `dragIn` (random replacement: draw, see 1.1 item 9). Not for Status moves; not through a Substitute hit (AfterMoveSecondary targets are the Pokémon hit; **unsure** whether a Substitute hit counts: the event runs for `targetsCopy.filter(val => !!val)` regardless).
**Eject Button.** Champions version (`ch/items.ts:262-278`, base `items.ts:1698-1713`): `onAfterMoveSecondaryPriority: 2`; for a damaging, non-`futuremove` move with `source !== target`, `target.hp > 0`: return if `!canSwitch(target.side)`, `target.forceSwitchFlag`, `target.beingCalledBack`, `target.isSkyDropped()`, the holder is commanding/commanded, or any active Pokémon has `switchFlag === true`; set `target.switchFlag = true`; if `target.useItem()` fails reset it. **The Champions override does not clear `source.switchFlag`** (the base game clears the attacker's pending pivot switch: `source.switchFlag = false` at items.ts:1708; Champions removes this line), so when Eject Button fires after U-turn the attacker's own switch is still pending. Needs the player's replacement choice (a request, no draw). Priority 2 so it runs before Red Card and most others.

**White Herb.** `onStart(pokemon)`: if any boost is below 0, `useItem()`, then `onUse` (`setBoost` back to 0 for the recorded stats; items.ts:7703-7733). The same function is called from `onAnySwitchIn` (priority -2), `onAnyAfterMega`, `onAnyAfterMove` and `onResidual` (order 29) for the holder. So it fires: on entering, when anyone switches in, after any Pokémon's move (Intimidate drop, Close Combat's own drop, Parting Shot), after any Mega Evolution (Mega Scrafty/Manectric Intimidate drops the holder's Attack and the herb fires at `AfterMega`), at end of turn. It resets only negative stats to 0 and does not touch positive ones; the herb is consumed through `useItem` so `lastItem` is set (Recycle works) and Unburden triggers. Fling: BP 10 with the same effect on the target (items.ts:7685-7702).
**Mental Herb.** `onUpdate`: if the holder has any of `attract, taunt, encore, torment, disable, healblock` (checked in that order), `useItem()` then removes **all six** volatiles (items.ts:3927-3941). Fling gives the same cure to the target (3910-3926).

**Choice Scarf.** Only Choice item. `onModifySpe`: `chainModify(1.5)` unless Dynamax (items.ts:1010-1013). `onModifyMove` adds the `choicelock` volatile (items.ts:1007-1009, run in `useMove`, `battle-actions.ts:503`), whose `onStart` records the active move id (it returns false and does not lock when the move has `hasBounced` set or came from Snatch; `conditions.ts:327-331`); `onBeforeMove` fails the move and removes the volatile when the item is gone; `onDisableMove` disables every other move slot, and removes the lock when the holder no longer has a Choice item or no longer has that move (`conditions.ts:332-361`). `onStart` of the item removes a stale lock and re-adds none (items.ts:1001-1006). So the lock appears when the move is **used** (not when selected), after sleep/flinch/paralysis checks pass; it ends on switching out (volatiles cleared) or loss of the item (register SR-26, N1).
**Quick Claw.** See 1.1 item 9; `onFractionalPriority` returns `0.1` when the roll succeeds (priority <= 0, `items.ts:5017-5020`), added to the action priority as a fraction (`battle.ts:2699`): it beats every other priority-0 move regardless of speed and loses to +1 priority. Two Quick Claws: both 0.1, so speed decides.
**Iron Ball.** `onEffectiveness`: Ground moves hit a Flying-type holder as neutral unless Ingrain, Smack Down or Gravity (items.ts:3068-3072); `onModifySpe` x0.5 (3074-3076); `isGrounded` returns true before the Flying/Levitate tests (`pokemon.ts:2174`). (The engine does not lift Levitate immunity: `items-03.md`.)

**Accuracy and crit.**
- Wide Lens: `onSourceModifyAccuracyPriority: -2`, `chainModify([4505, 4096])` if accuracy is a number (items.ts:7743-7748).
- Zoom Lens: same priority, `chainModify([4915, 4096])` when `!this.queue.willMove(target)` i.e. the target has already moved this turn (7855-7860).
- Bright Powder: `onModifyAccuracyPriority: -2`, `chainModify([3686, 4096])` against the holder (677-682).
- Scope Lens: `onModifyCritRatio` +1 (5584-5586). Leek: +2 for Farfetch'd/Sirfetch'd (`baseSpecies.baseSpecies`, items.ts:3340-3344); legal in Champions by `ch/items.ts:514`. Crit chance table: `critMult = [0, 24, 8, 2, 1]` for stage 0..4 (`battle-actions.ts:1678-1695`, gen 7+): stage 1 = 1/24, 2 = 1/8, 3 = 1/2, 4 = always. Draw `randomChance(1, critMult[stage])` per hit (not drawn when stage is 0 = never crits unless `willCrit`; note `critRatio` of 0 skips the draw: `if (critRatio) ...`, `battle-actions.ts:1693-1695`).
- King's Rock: `onModifyMovePriority: -1`; for non-Status moves, appends `{chance: 10, volatileStatus: 'flinch'}` unless a flinch secondary exists (items.ts:3228-3240). Order of ModifyMove handlers: Sheer Force (priority 0) deletes secondaries first, **then** King's Rock re-adds the flinch; Serene Grace (priority -2) runs after and doubles it to 20 (abilities.ts Serene Grace, `onModifyMovePriority: -2`). Draw: one `random(100)` per damaging hit as in 1.1 item 9.

**Duration extenders** (no handler of their own; the setter's item is read when the effect starts):
- Damp Rock (`conditions.ts:485`), Heat Rock (:555), Smooth Rock (:637): 8 turns instead of 5 for rain, sun, sand. **Icy Rock (:675 hail, :705 snow): 8 turns** (CG's `items-03.md` says Icy Rock has no effect; that is wrong, the source has two `hasItem('icyrock')` tests). The weather's setter is the Pokémon that used the move or has the ability: `durationCallback(source, source, sourceEffect)` (`field.ts:78-80`), so Drought/Drizzle/Sand Stream/Snow Warning (and Mega abilities of the same name, section 3) with the matching rock also give 8 turns. `source.hasItem()` is subject to Klutz/Magic Room at that moment. Extreme weathers (Desolate Land, Primordial Sea) have no duration.
- Light Clay: Reflect (`moves.ts:14852`), Light Screen (`moves.ts:10330`), Aurora Veil (`moves.ts:846`): 8 turns instead of 5 (`durationCallback(target, source, effect)` tests `source.hasItem('lightclay')`).
- Terrain Extender: Electric (`moves.ts:4514`), Grassy (`moves.ts:7690`), Misty (`moves.ts:12167`), Psychic Terrain (`moves.ts:14106`): 8 turns instead of 5. Also applies when the terrain is set by an ability (Electric Surge etc. pass the holder as `source`, `field.ts:140-150`).
- Binding Band: the partial-trap volatile stores `boundDivisor = source.hasItem('bindingband') ? 6 : 8` (`conditions.ts:231`); the trap's residual damage is `baseMaxhp / boundDivisor` (Grip Claw is not legal, so the duration stays `random(5, 7)`, `conditions.ts:225-227`: one draw when the trap starts).

**Terrain seeds (4).** `onSwitchInPriority: -1`, `onStart(pokemon)`: if `!pokemon.ignoringItem() && field.isTerrain(<terrain>)` then `pokemon.useItem()`; `onTerrainChange(pokemon)`: if the terrain is now that terrain, `useItem()` (items.ts:2613-2623 Grassy; Psychic 4933-4943; Electric and Misty the same shape). `boosts`: Grassy and Electric +1 Def, Psychic and Misty +1 SpD, applied by `useItem` -> `battle.boost` and **consumed even if the stat is already +6**. `TerrainChange` runs via `eachEvent` (not prefixed, `battle.ts:1066`), so a seed holder that is already out reacts when the terrain starts mid-battle; the engine only does switch-in (register I8).

**Shed Shell.** `onTrapPokemonPriority -10` sets `trapped = false` and `onMaybeTrapPokemonPriority -10` sets `maybeTrapped = false` (items.ts:5663-5670). It runs after the trappers (ability Shadow Tag, Arena Trap, Magnet Pull and the trap volatiles use priority 0 by default), so it overrides them. It is not read to override Ingrain/No Retreat/Octolock by the audit (`items-03.md` says engine checks LOCKEDMOVE/NORETREAT first and Showdown's -10 handler would free a No Retreat user): the volatiles `noretreat`, `octolock` and `partiallytrapped` set `pokemon.tryTrap()` in `onTrapPokemon` with default priority; **not read in detail**.


### 1.4 Mega stones (81 items, 82 Mega forms)

Rules shared by every stone (items.ts block of each stone, e.g. Garchompite Z items.ts:2430-2441):

- `megaStone: { "<base species name>": "<Mega form name>" }` and `itemUser`. Meowsticite maps both `Meowstic` and `Meowstic-F`; Floettite maps `Floette-Eternal` (not plain Floette); Charizardite X/Y and Raichunite X/Y give distinct forms; the Z stones (Absolite Z, Garchompite Z, Lucarionite Z) give the `-Mega-Z` forms.
- `onTakeItem(item, holder)` returns `false` (the stone cannot be taken) when the holder is of the matching species: `!item.megaStone?.[holder.baseSpecies.baseSpecies]` (single-form stones) or, for Meowsticite and Floettite, `!item.megaStone || (!item.megaStone[holder.baseSpecies.name] && !Object.values(item.megaStone).includes(holder.baseSpecies.name))` (items.ts:3953-3956, 2206-2209). `baseSpecies` becomes the Mega form after Mega Evolution (`ch/scripts.ts:68`), so the check keeps matching afterwards (`Meowstic-M-Mega` is in the values list). A stone on a holder of another species can be taken (cannot arise on legal teams; the engine's blanket protection is a harmless superset, register N1 of `second-read/items-b1.md`).
- Because of `onTakeItem`: Knock Off, Thief, Covet, Pickpocket, Magician, Trick and Switcheroo (both directions), Bug Bite/Pluck (not berries anyway), Corrosive Gas and Fling all fail or skip for the stone (section 2). Knock Off's x1.5 is skipped too (`singleEvent('TakeItem', item, ...)`, moves.ts:9972).
- Nothing else: no battle handler. The stone stays held after Mega Evolution (the item is not consumed), so a Mega Pokémon has the unremovable stone for the rest of the game and never has "no item" (matters for Acrobatics, Unburden, Poltergeist, Recycle).
- Fling base power 80 for every stone (`dex-items.ts:146-149`) but Fling always fails for the matching holder.
- Item Clause = 1 (Flat Rules, `ch/rulesets.ts:38`): a team never holds two of the same item, so at most one holder of each stone.

Table columns: usage as in 1.2; "Mega ability (slot 0)" is the ability the Pokémon gets (`species.abilities['0']`, `ch/scripts.ts:110`); types shown without the second type when the Mega is mono-type (the engine's `Typeless`); weights in hectograms (`weight_hg`), which change on Mega Evolution through `setSpecies` (`pokemon.ts:1414`). HP is identical before and after for all 82 forms (compared in `generated.rs`), so `updateMaxHp` never changes HP.

| stone | rank / pool% / live% | items.ts | base species -> Mega (pokedex.ts line of Mega) | Mega types, base stats HP/Atk/Def/SpA/SpD/Spe | Mega ability (slot 0) | weight base -> Mega (hg) |
|---|---|---|---|---|---|---|
| Abomasite (`abomasite`) | #88 / 0.03 / 0.05  | 22 | Abomasnow -> Abomasnow-Mega (pokedex.ts:8427) | Grass/Ice; 90,132,105,132,105,30 | snowwarning | 1355 -> 1850 |
| Absolite (`absolite`) | #67 / 0.09 / 0.05  | 34 | Absol -> Absol-Mega (pokedex.ts:6710) | Dark; 65,150,60,115,60,115 | magicbounce | 470 -> 490 |
| Absolite Z (`absolitez`) | #48 / 0.25 / 0.21  | 46 | Absol -> Absol-Mega-Z (pokedex.ts:6724) | Dark/Ghost; 65,154,60,75,60,151 | sharpness | 470 -> 490 |
| Aerodactylite (`aerodactylite`) | #75 / 0.07 / 0.05  | 135 | Aerodactyl -> Aerodactyl-Mega (pokedex.ts:3192) | Rock/Flying; 80,135,85,70,95,150 | toughclaws | 590 -> 790 |
| Aggronite (`aggronite`) | #83 / 0.05 / 0.00  | 147 | Aggron -> Aggron-Mega (pokedex.ts:5872) | Steel; 70,140,230,60,80,50 | filter | 3600 -> 3950 |
| Alakazite (`alakazite`) | #89 / 0.03 / 0.05  | 215 | Alakazam -> Alakazam-Mega (pokedex.ts:1618) | Psychic; 55,50,65,175,105,150 | trace | 480 -> 480 |
| Altarianite (`altarianite`) | #70 / 0.07 / 0.10  | 238 | Altaria -> Altaria-Mega (pokedex.ts:6301) | Dragon/Fairy; 75,110,110,110,105,80 | pixilate | 206 -> 206 |
| Ampharosite (`ampharosite`) | pool 0 / 0.00 / 0.05  | 250 | Ampharos -> Ampharos-Mega (pokedex.ts:3911) | Electric/Dragon; 90,95,105,165,110,45 | moldbreaker | 615 -> 615 |
| Audinite (`audinite`) | #92 / 0.03 / 0.00  | 334 | Audino -> Audino-Mega (pokedex.ts:9957) | Normal/Fairy; 103,60,126,80,126,50 | healer | 310 -> 320 |
| Banettite (`banettite`) | #42 / 0.34 / 0.26  | 379 | Banette -> Banette-Mega (pokedex.ts:6614) | Ghost; 64,165,75,93,83,75 | prankster | 125 -> 130 |
| Barbaracite (`barbaracite`) | #93 / 0.03 / 0.00  | 391 | Barbaracle -> Barbaracle-Mega (pokedex.ts:12959) | Rock/Fighting; 72,140,130,64,106,88 | toughclaws | 960 -> 1000 |
| Baxcalibrite (`baxcalibrite`) | #19 / 1.07 / 0.99  | 403 | Baxcalibur -> Baxcalibur-Mega (pokedex.ts:19060) | Dragon/Ice; 115,175,117,105,101,87 | thermalexchange | 2100 -> 3150 |
| Beedrillite (`beedrillite`) | #69 / 0.07 / 0.16  | 422 | Beedrill -> Beedrill-Mega (pokedex.ts:343) | Bug/Poison; 65,150,40,15,80,145 | adaptability | 295 -> 405 |
| Blastoisinite (`blastoisinite`) | #37 / 0.51 / 0.26  | 568 | Blastoise -> Blastoise-Mega (pokedex.ts:216) | Water; 79,103,120,135,115,78 | megalauncher | 855 -> 1011 |
| Blazikenite (`blazikenite`) | #17 / 1.27 / 1.72  | 580 | Blaziken -> Blaziken-Mega (pokedex.ts:5119) | Fire/Fighting; 80,160,80,130,80,100 | speedboost | 520 -> 520 |
| Cameruptite (`cameruptite`) | pool 0 / 0.00 / 0.05  | 744 | Camerupt -> Camerupt-Mega (pokedex.ts:6149) | Fire/Ground; 70,120,100,145,105,20 | sheerforce | 2200 -> 3205 |
| Chandelurite (`chandelurite`) | pool 0 / 0.00 / 0.10  | 773 | Chandelure -> Chandelure-Mega (pokedex.ts:11241) | Ghost/Fire; 60,75,110,175,110,90 | infiltrator | 343 -> 696 |
| Charizardite X (`charizarditex`) | #15 / 1.35 / 1.51  | 800 | Charizard -> Charizard-Mega-X (pokedex.ts:125) | Fire/Dragon; 78,130,111,130,85,100 | toughclaws | 905 -> 1105 |
| Charizardite Y (`charizarditey`) | #13 / 1.73 / 2.19  | 812 | Charizard -> Charizard-Mega-Y (pokedex.ts:140) | Fire/Flying; 78,104,78,159,115,100 | drought | 905 -> 1005 |
| Chesnaughtite (`chesnaughtite`) | #78 / 0.07 / 0.00  | 877 | Chesnaught -> Chesnaught-Mega (pokedex.ts:12050) | Grass/Fighting; 88,137,172,74,115,44 | bulletproof | 900 -> 900 |
| Chimechite (`chimechite`) | pool 0 / 0.00 / 0.00  | 950 | Chimecho -> Chimecho-Mega (pokedex.ts:6682) | Psychic/Steel; 75,50,110,135,120,65 | levitate | 10 -> 80 |
| Clefablite (`clefablite`) | #25 / 0.83 / 0.52  | 1100 | Clefable -> Clefable-Mega (pokedex.ts:1047) | Fairy/Flying; 95,80,93,135,110,70 | magicbounce | 400 -> 423 |
| Crabominite (`crabominite`) | pool 0 / 0.00 / 0.00  | 1226 | Crabominable -> Crabominable-Mega (pokedex.ts:13979) | Fighting/Ice; 97,157,122,62,107,33 | ironfist | 1800 -> 2528 |
| Delphoxite (`delphoxite`) | #32 / 0.65 / 0.89  | 1390 | Delphox -> Delphox-Mega (pokedex.ts:12110) | Fire/Psychic; 75,69,72,159,125,134 | levitate | 390 -> 390 |
| Dragalgite (`dragalgite`) | #60 / 0.12 / 0.21  | 1482 | Dragalge -> Dragalge-Mega (pokedex.ts:13001) | Poison/Dragon; 65,85,105,132,163,44 | regenerator | 815 -> 1003 |
| Dragoninite (`dragoninite`) | #11 / 1.92 / 1.88  | 1523 | Dragonite -> Dragonite-Mega (pokedex.ts:3367) | Dragon/Flying; 91,124,115,145,125,100 | multiscale | 2100 -> 2900 |
| Drampanite (`drampanite`) | #86 / 0.03 / 0.10  | 1572 | Drampa -> Drampa-Mega (pokedex.ts:15100) | Normal/Dragon; 78,85,110,160,116,36 | berserk | 1850 -> 2405 |
| Eelektrossite (`eelektrossite`) | #59 / 0.14 / 0.26  | 1669 | Eelektross -> Eelektross-Mega (pokedex.ts:11159) | Electric; 85,145,80,135,90,80 | eelevate | 805 -> 1800 |
| Emboarite (`emboarite`) | #95 / 0.03 / 0.00  | 1846 | Emboar -> Emboar-Mega (pokedex.ts:9499) | Fire/Fighting; 110,148,75,110,110,75 | moldbreaker | 1500 -> 1803 |
| Excadrite (`excadrite`) | #47 / 0.30 / 0.31  | 1901 | Excadrill -> Excadrill-Mega (pokedex.ts:9929) | Ground/Steel; 110,165,100,65,65,103 | piercingdrill | 404 -> 600 |
| Falinksite (`falinksite`) | #90 / 0.03 / 0.05  | 1984 | Falinks -> Falinks-Mega (pokedex.ts:16862) | Fighting; 65,135,135,70,65,100 | defiant | 620 -> 990 |
| Feraligite (`feraligite`) | #71 / 0.07 / 0.10  | 2003 | Feraligatr -> Feraligatr-Mega (pokedex.ts:3604) | Water/Dragon; 85,160,125,89,93,78 | dragonize | 888 -> 1088 |
| Floettite (`floettite`) | pool 0 / 0.00 / 0.36  | 2201 | Floette-Eternal -> Floette-Mega (pokedex.ts:12589) | Fairy; 74,85,87,155,148,102 | fairyaura | 9 -> 1008 |
| Froslassite (`froslassite`) | #82 / 0.06 / 0.00  | 2345 | Froslass -> Froslass-Mega (pokedex.ts:8723) | Ice/Ghost; 70,80,70,140,100,120 | snowwarning | 266 -> 296 |
| Galladite (`galladite`) | #61 / 0.10 / 0.16  | 2386 | Gallade -> Gallade-Mega (pokedex.ts:8663) | Psychic/Fighting; 68,165,95,65,115,110 | innerfocus | 520 -> 564 |
| Garchompite (`garchompite`) | pool 0 / 0.00 / 0.00  | 2418 | Garchomp -> Garchomp-Mega (pokedex.ts:8171) | Dragon/Ground; 108,170,115,120,95,92 | sandforce | 950 -> 950 |
| Garchompite Z (`garchompitez`) | #7 / 4.48 / 3.75 gold | 2430 | Garchomp -> Garchomp-Mega-Z (pokedex.ts:8185) | Dragon; 108,130,85,141,85,151 | levitate | 950 -> 990 |
| Gardevoirite (`gardevoirite`) | #43 / 0.34 / 0.31  | 2442 | Gardevoir -> Gardevoir-Mega (pokedex.ts:5514) | Psychic/Fairy; 68,85,65,165,135,100 | pixilate | 484 -> 484 |
| Gengarite (`gengarite`) | #35 / 0.54 / 0.42  | 2454 | Gengar -> Gengar-Mega (pokedex.ts:2226) | Ghost/Poison; 60,65,80,170,95,130 | shadowtag | 405 -> 405 |
| Glalitite (`glalitite`) | #63 / 0.10 / 0.05  | 2508 | Glalie -> Glalie-Mega (pokedex.ts:6779) | Ice; 80,120,80,120,80,100 | refrigerate | 2565 -> 3502 |
| Glimmoranite (`glimmoranite`) | #22 / 1.02 / 0.73  | 2520 | Glimmora -> Glimmora-Mega (pokedex.ts:18591) | Rock/Poison; 83,90,105,150,96,101 | adaptability | 450 -> 770 |
| Golisopite (`golisopite`) | #8 / 3.02 / 3.13 gold | 2541 | Golisopod -> Golisopod-Mega (pokedex.ts:14533) | Bug/Steel; 75,150,175,70,120,40 | toughclaws | 1080 -> 1480 |
| Golurkite (`golurkite`) | pool 0 / 0.00 / 0.00  | 2553 | Golurk -> Golurk-Mega (pokedex.ts:11449) | Ground/Ghost; 89,159,105,70,105,55 | unseenfist | 3300 -> 3300 |
| Greninjite (`greninjite`) | #24 / 0.83 / 0.63  | 2637 | Greninja -> Greninja-Mega (pokedex.ts:12202) | Water/Dark; 72,125,77,133,81,142 | protean | 400 -> 400 |
| Gyaradosite (`gyaradosite`) | #27 / 0.74 / 0.99  | 2750 | Gyarados -> Gyarados-Mega (pokedex.ts:2967) | Water/Dark; 95,155,109,70,130,81 | moldbreaker | 2350 -> 3050 |
| Hawluchanite (`hawluchanite`) | pool 0 / 0.00 / 0.00  | 2801 | Hawlucha -> Hawlucha-Mega (pokedex.ts:13151) | Fighting/Flying; 78,137,100,74,93,118 | noguard | 215 -> 250 |
| Heracronite (`heracronite`) | #54 / 0.17 / 0.10  | 2889 | Heracross -> Heracross-Mega (pokedex.ts:4440) | Bug/Fighting; 80,185,115,40,105,75 | skilllink | 540 -> 625 |
| Houndoominite (`houndoominite`) | pool 0 / 0.00 / 0.05  | 2913 | Houndoom -> Houndoom-Mega (pokedex.ts:4694) | Dark/Fire; 75,90,90,140,90,115 | solarpower | 350 -> 495 |
| Kangaskhanite (`kangaskhanite`) | #68 / 0.07 / 0.21  | 3129 | Kangaskhan -> Kangaskhan-Mega (pokedex.ts:2653) | Normal; 105,125,100,60,100,100 | parentalbond | 800 -> 1000 |
| Lopunnite (`lopunnite`) | #36 / 0.51 / 0.73  | 3487 | Lopunny -> Lopunny-Mega (pokedex.ts:7934) | Normal/Fighting; 65,136,94,54,96,135 | scrappy | 333 -> 283 |
| Lucarionite (`lucarionite`) | #40 / 0.41 / 0.26  | 3515 | Lucario -> Lucario-Mega (pokedex.ts:8245) | Fighting/Steel; 70,145,88,140,70,112 | adaptability | 540 -> 575 |
| Lucarionite Z (`lucarionitez`) | #10 / 2.33 / 1.82  | 3527 | Lucario -> Lucario-Mega-Z (pokedex.ts:8260) | Fighting/Steel; 70,100,70,164,70,151 | auraguard | 540 -> 494 |
| Malamarite (`malamarite`) | pool 0 / 0.00 / 0.00  | 3772 | Malamar -> Malamar-Mega (pokedex.ts:12917) | Dark/Psychic; 86,102,88,98,120,88 | contrary | 470 -> 698 |
| Manectite (`manectite`) | pool 0 / 0.00 / 0.00  | 3793 | Manectric -> Manectric-Mega (pokedex.ts:5954) | Electric; 70,75,80,135,80,135 | intimidate | 402 -> 440 |
| Mawilite (`mawilite`) | #49 / 0.24 / 0.36  | 3851 | Mawile -> Mawile-Mega (pokedex.ts:5817) | Steel/Fairy; 50,105,125,55,95,50 | hugepower | 115 -> 235 |
| Medichamite (`medichamite`) | pool 0 / 0.00 / 0.10  | 3883 | Medicham -> Medicham-Mega (pokedex.ts:5913) | Fighting/Psychic; 60,100,85,80,85,100 | purepower | 315 -> 315 |
| Meganiumite (`meganiumite`) | #45 / 0.31 / 0.36  | 3895 | Meganium -> Meganium-Mega (pokedex.ts:3484) | Grass/Fairy; 80,92,115,143,115,80 | megasol | 1005 -> 2010 |
| Meowsticite (`meowsticite`) | pool 0 / 0.00 / 0.00  | 3945 | Meowstic -> Meowstic-M-Mega (pokedex.ts:12745) | Psychic; 74,48,76,143,101,124 | trace | 85 -> 101 |
| Meowsticite (`meowsticite`) | pool 0 / 0.00 / 0.00  | 3945 | Meowstic-F -> Meowstic-F-Mega (pokedex.ts:12762) | Psychic; 74,48,76,143,101,124 | trace | 85 -> 101 |
| Metagrossite (`metagrossite`) | #14 / 1.71 / 1.09  | 3961 | Metagross -> Metagross-Mega (pokedex.ts:6995) | Steel/Psychic; 80,145,150,105,110,110 | toughclaws | 5500 -> 9429 |
| Pidgeotite (`pidgeotite`) | #91 / 0.03 / 0.05  | 4581 | Pidgeot -> Pidgeot-Mega (pokedex.ts:398) | Normal/Flying; 83,80,80,135,80,121 | noguard | 395 -> 505 |
| Pinsirite (`pinsirite`) | #80 / 0.07 / 0.00  | 4628 | Pinsir -> Pinsir-Mega (pokedex.ts:2870) | Bug/Flying; 65,155,120,65,90,105 | aerilate | 550 -> 590 |
| Pyroarite (`pyroarite`) | #65 / 0.10 / 0.05  | 4982 | Pyroar -> Pyroar-Mega (pokedex.ts:12524) | Fire/Normal; 86,88,92,129,86,126 | firemane | 815 -> 933 |
| Raichunite X (`raichunitex`) | pool 0 / 0.00 / 0.00  | 5059 | Raichu -> Raichu-Mega-X (pokedex.ts:839) | Electric; 60,135,95,90,95,110 | electricsurge | 300 -> 380 |
| Raichunite Y (`raichunitey`) | #21 / 1.06 / 0.57  | 5071 | Raichu -> Raichu-Mega-Y (pokedex.ts:854) | Electric; 60,100,55,160,80,130 | noguard | 300 -> 260 |
| Sablenite (`sablenite`) | #85 / 0.04 / 0.05  | 5453 | Sableye -> Sableye-Mega (pokedex.ts:5790) | Dark/Ghost; 50,85,125,85,115,20 | magicbounce | 110 -> 1610 |
| Salamencite (`salamencite`) | #9 / 2.40 / 2.34  | 5530 | Salamence -> Salamence-Mega (pokedex.ts:6937) | Dragon/Flying; 95,145,130,120,90,120 | aerilate | 1026 -> 1126 |
| Sceptilite (`sceptilite`) | #73 / 0.07 / 0.10  | 5542 | Sceptile -> Sceptile-Mega (pokedex.ts:5060) | Grass/Dragon; 70,110,75,145,85,145 | lightningrod | 522 -> 552 |
| Scizorite (`scizorite`) | #44 / 0.33 / 0.16  | 5554 | Scizor -> Scizor-Mega (pokedex.ts:4402) | Bug/Steel; 70,150,140,65,100,75 | technician | 1180 -> 1250 |
| Scolipite (`scolipite`) | pool 0 / 0.00 / 0.00  | 5566 | Scolipede -> Scolipede-Mega (pokedex.ts:10156) | Bug/Poison; 60,140,149,75,99,62 | shellarmor | 2005 -> 2305 |
| Scovillainite (`scovillainite`) | #53 / 0.19 / 0.21  | 5590 | Scovillain -> Scovillain-Mega (pokedex.ts:18331) | Grass/Fire; 65,138,85,138,85,75 | spicyspray | 150 -> 220 |
| Scraftinite (`scraftinite`) | pool 0 / 0.00 / 0.10  | 5602 | Scrafty -> Scrafty-Mega (pokedex.ts:10478) | Dark/Fighting; 65,130,135,55,135,68 | intimidate | 300 -> 310 |
| Sharpedonite (`sharpedonite`) | #87 / 0.03 / 0.10  | 5645 | Sharpedo -> Sharpedo-Mega (pokedex.ts:6083) | Water/Dark; 70,140,70,110,65,105 | strongjaw | 888 -> 1303 |
| Skarmorite (`skarmorite`) | #98 / 0.02 / 0.05  | 5790 | Skarmory -> Skarmory-Mega (pokedex.ts:4652) | Steel/Flying; 65,140,110,40,100,110 | stalwart | 505 -> 404 |
| Slowbronite (`slowbronite`) | #50 / 0.24 / 0.10  | 5832 | Slowbro -> Slowbro-Mega (pokedex.ts:1965) | Water/Psychic; 95,75,180,130,80,30 | shellarmor | 785 -> 1200 |
| Staraptite (`staraptite`) | #20 / 1.06 / 1.25  | 6001 | Staraptor -> Staraptor-Mega (pokedex.ts:7440) | Fighting/Flying; 85,140,100,60,90,110 | contrary | 249 -> 500 |
| Starminite (`starminite`) | #31 / 0.69 / 1.20  | 6045 | Starmie -> Starmie-Mega (pokedex.ts:2749) | Water/Psychic; 60,100,105,130,105,120 | hugepower | 800 -> 800 |
| Steelixite (`steelixite`) | pool 0 / 0.00 / 0.16  | 6092 | Steelix -> Steelix-Mega (pokedex.ts:4318) | Steel/Ground; 75,125,230,55,95,30 | sandforce | 4000 -> 7400 |
| Swampertite (`swampertite`) | #30 / 0.69 / 0.99  | 6204 | Swampert -> Swampert-Mega (pokedex.ts:5178) | Water/Ground; 100,150,110,95,110,70 | swiftswim | 819 -> 1020 |
| Tyranitarite (`tyranitarite`) | #34 / 0.56 / 0.26  | 7416 | Tyranitar -> Tyranitar-Mega (pokedex.ts:4963) | Rock/Dark; 100,164,150,95,120,71 | sandstream | 2020 -> 2550 |
| Venusaurite (`venusaurite`) | #41 / 0.38 / 0.47  | 7493 | Venusaur -> Venusaur-Mega (pokedex.ts:49) | Grass/Poison; 80,100,123,122,120,80 | thickfat | 1000 -> 1555 |
| Victreebelite (`victreebelite`) | #52 / 0.21 / 0.16  | 7505 | Victreebel -> Victreebel-Mega (pokedex.ts:1734) | Grass/Poison; 80,125,85,135,95,70 | innardsout | 155 -> 1255 |

## 2. Item-dependent moves

Thirteen of the fourteen moves in the brief are legal here (Gastro Acid is skipped as asked), plus Trick and Switcheroo. All are in the `MOVES` table of `generated.rs`. Corrosive Gas is legal only because the Champions mod flips it (`ch/moves.ts:149-152`, `isNonstandard: null`). **Embargo, Natural Gift, Bestow, Incinerate, Judgment/Multi-Attack/Techno Blast with plates or memories do not exist or are not item-driven here** (Embargo, Natural Gift and Bestow are absent from the move table; Magic Room is present). Line numbers are `data/moves.ts` unless noted.

Common facts: a Pokémon "has an item" in these moves by raw `pokemon.item` (not `hasItem`), except where stated, so Klutz, Magic Room and Embargo do **not** stop the checks written with `.item`. `takeItem()` runs the `TakeItem` event, which Sticky Hold blocks (`source !== holder` or Knock Off) and the Mega stone of its own species blocks (section 1.4). The Gem volatile exists only for Normal Gem (`source.volatiles['gem']` is truthy for the rest of the move that consumed the gem).

| move | lines | rule |
|---|---|---|
| **Acrobatics** | 117-136 | Flying, Physical, base 55, contact, `distance`, target `any`. `basePowerCallback`: `!pokemon.item` -> `move.basePower * 2` = 110, else 55. Raw `item`: a Klutz holder with an item does not double; a holder whose item was consumed or knocked off does. A Mega-evolved Pokémon always holds its stone (no double). |
| **Belch** | 1197-1215 | Poison, Special, base 120. `onTry(source)` returns `source.ateBerry`; if false the move fails (no PP rule change). `ateBerry` is set to true in `eatItem` (`pokemon.ts:1825`) and by Bug Bite/Pluck/Fling berries (`if (item.onEat) source.ateBerry = true`). It is never reset (initialised at `pokemon.ts:450`, not cleared on switch). **Champions override `onDisableMove: undefined`** (`ch/moves.ts:52-55`): the base game disables Belch in the request when `ateBerry` is false; here Belch stays selectable and fails in `onTry`. A Belch that fails in `onTry` has already spent its PP (`deductPP` at `battle-actions.ts:339`, before the `Try` event at `battle-actions.ts:652`). |
| **Bug Bite** | 1911-1934 | Bug, Physical, base 60, contact. `onHit(target, source, move)`: `item = target.getItem()`; if `source.hp && item.isBerry && target.takeItem(source)` the user **eats** the target's berry: `singleEvent('Eat', item, target.itemState, source, source, move)`; if that returned truthy, `runEvent('EatItem', source, ...)`; `if (item.id === 'leppaberry') target.staleness = 'external'`; `if (item.onEat) source.ateBerry = true` (lines 1920-1930). Bypasses `UseItem`, `TryEatItem` and Unnerve (the berry's own `onEat` runs directly, e.g. Sitrus heals the user `baseMaxhp/4` even above the 50% trigger; Lum cures the user; resist berries do nothing). Fails silently on Sticky Hold (takeItem false), a Substitute (onHit does not run on a Substitute hit), a non-berry item. `onHit` is the move's own handler, run in the Hit step after damage. |
| **Pluck** | 13436-13459 | Flying, Physical, base 60, contact, `distance`. Identical `onHit` to Bug Bite (lines 13445-13455). |
| **Corrosive Gas** | 2914-2934 | Poison Status, `protect`, `reflectable`, `mirror`, `allyanim`, target `allAdjacent`. `onHit(target, source)`: `item = target.takeItem(source)`; truthy -> `-enditem` message, else fail message. The taken item is destroyed (not given to the user, `lastItem` unchanged because `takeItem` does not set it). Blocked by Sticky Hold and by a Mega stone of the holder's species; accuracy applies (`accuracy` field, not item related). In singles `allAdjacent` is just the foe. |
| **Covet** | 3099-3128 | Normal, Physical, base 60, contact, `failmefirst`. `onAfterHit(target, source, move)`: return if `source.item` or `source.volatiles['gem']`; `yourItem = target.takeItem(source)`; return if falsy; then `singleEvent('TakeItem', yourItem, target.itemState, source, target, move, yourItem)` (checks the item can go to the user: the stone clause) and `source.setItem(yourItem)`; if either fails, `target.item = yourItem.id` (**restored, bypassing setItem**); else messages. The user must have **no** item (raw `item`). `setItem` runs `End` on the old and `Start` on the new item (Air Balloon message, Metronome volatile). A Mega Pokémon (holds its stone) can never use Covet's steal. The Champions hit loop runs `AfterHit` even when the user fainted (`ch/scripts.ts:313`), but `setItem` returns false for a fainted user, so the stolen item is put back on the target. |
| **Thief** | 19298-19326 | Dark, Physical, base 60, contact, `failmefirst`. Same as Covet (lines 19307-19325) with a silent message. Same rules. |
| **Knock Off** | 9961-9986 | Dark, Physical, base 65, contact. `onBasePower(basePower, source, target, move)`: `item = target.getItem()`; `if (!singleEvent('TakeItem', item, target.itemState, target, target, move, item)) return;` then `if (item.id) return chainModify(1.5)` (lines 9970-9976). So x1.5 applies when the target holds any item that could be taken **by the item's own `onTakeItem`**: only Mega stones of the holder's species fail; **Sticky Hold does not stop the boost** (it is an ability event, `singleEvent` only reads the item). CG's `items-02.md` N5 says the opposite; the code says the boost applies. `onAfterHit(target, source)`: `item = target.takeItem()` (no source argument, so `source = target`), message; Sticky Hold blocks it because the active move is Knock Off (abilities.ts:4627). `takeItem` does not set `lastItem` (Recycle cannot get it back). Runs after damage; not after a Substitute hit. The item is not given to the user. The boost uses raw `getItem()` (Klutz holder still loses it and still gets hit for x1.5). |
| **Poltergeist** | 13592-13609 | Ghost, Physical, base 110. `onTry(source, target)`: `return !!target.item` (raw item: Klutz/Magic Room holders still count; a Mega stone counts). Fails otherwise (the move is lost, PP used). `onTryHit` only a message. No item effect. |
| **Recycle** | 14818-14838 | Normal Status, target `self`, `snatch`. `onHit(pokemon, source, move)`: `if (pokemon.item || !pokemon.lastItem) return false;` else `item = pokemon.lastItem; pokemon.lastItem = ''; setItem(item, source, move)`. `lastItem` is set by `eatItem` and `useItem` only (berries eaten, herbs, gems, seeds, Sash/Red Card/Eject Button used), **not** by `takeItem` (Knock Off, Thief, Trick) and not by Air Balloon's pop. Persists across switches for the same Pokémon (`battle-actions.ts:148` copies only in gen <= 4). The restored item runs `onStart` (Air Balloon re-inflates; a berry may be eaten at the next Update). |
| **Stuff Cheeks** | 18248-18269 | Normal Status, self. `onTry(source)`: `source.getItem().isBerry` must be true (uses `getItem`, so a berry under Klutz still passes). `onHit(pokemon)`: `if (!this.boost({def: 2})) return null;` (fails with no PP refund when Defense cannot rise: +6 or Clear Body-type block) then `pokemon.eatItem(true)` (**force**: skips `TryEatItem`, so Sitrus at full HP is eaten and its heal does nothing, but Unnerve does not stop it (`UseItem` still runs)). **Champions: `onDisableMove: undefined`** (`ch/moves.ts:989-992`): selectable without a berry, fails in `onTry`. |
| **Teatime** | 19031-19061 | Normal Status, target `all` (field), `bypasssub`. `onHitField`: for every active Pokémon: if `runEvent('Invulnerability') === false` print a miss; else if `runEvent('TryHit', pokemon, source, move)` is truthy **and** `pokemon.getItem().isBerry` add to targets; if no targets, fail (`NOT_FAIL`); else for each, `pokemon.eatItem(true)` in the order built (`getAllActive()`: side order). The foe is included (a foe's berry is eaten); Protect/Substitute-style `TryHit` blockers exclude a Pokémon. Force-eat: no `TryEatItem` (Unnerve irrelevant), `UseItem` still runs. |
| **Trick / Switcheroo** | 19861-19915 / 18640-18690 | Psychic / Dark Status, `protect`, `mirror`, `allyanim`, `noassist`, `failcopycat`. `onTryImmunity(target)`: `return !target.hasAbility('stickyhold')` (the foe's Sticky Hold = the move fails before `onHit`; **the user's own Sticky Hold does not matter**). `onHit(target, source, move)`: `yourItem = target.takeItem(source)`; `myItem = source.takeItem()`; if either is `false` (blocked: a Mega stone of the holder's species on either Pokémon; Sticky Hold already tested) **or** neither has an item, restore both (`target.item = yourItem.id`, `source.item = myItem.id`) and fail. Then `singleEvent('TakeItem', myItem, source.itemState, target, source, move, myItem)` and the same for `yourItem` with `(source, target)`: the item's `onTakeItem` is asked with the **receiving** Pokémon as holder (the stone clause for a non-matching receiver); if either returns false, restore both and fail. Success: `target.setItem(myItem)` (or `-enditem` if the user had none) then `source.setItem(yourItem)`. `setItem` runs End/Start of both items (re-inflates an Air Balloon, adds Metronome volatile, resets a Choice lock via the Scarf's `onStart`). `setItem` records "external" staleness for Leppa only (cosmetic). **The user's own Mega stone makes Trick fail** (`source.takeItem()` returns false; CG register I12, `items-01.md` M1). Choice Scarf given away: the holder's `choicelock` is removed at the next `onDisableMove`/`onBeforeMove` (the lock test `!pokemon.getItem().isChoice`). A user with no item and a target with an item: succeeds and the user receives it (the target loses it). The Pokémon that receives a Choice Scarf and moves: `onModifyMove` adds the lock at its next move. |
| **Fling** | 5736-5815 | Dark, Physical, `protect`, `mirror`, `allyanim`, `noparentalbond`, base 0 then set. `onPrepareHit(target, source, move)`: (1) `source.ignoringItem(true)` -> fail (Klutz always, Magic Room, Embargo; `isFling` uses `hasAbility('klutz')` ignoring `ignoreKlutz`, `pokemon.ts:904-905`); (2) `singleEvent('TakeItem', item, source.itemState, source, source, move, item)` false -> fail (**Mega stone of the holder's species**); (3) `!item.fling` -> fail (**Normal Gem has no fling data**); (4) `move.basePower = item.fling.basePower`; (5) berry: `move.onHit` eats it on the target (`singleEvent('Eat', item, source.itemState, foe, source, move)` then `EatItem` if truthy, `if (item.onEat) foe.ateBerry = true`; Cud Chew of the user triggers an `EatItem` on the user) else if `item.fling.effect` set `move.onHit = effect` (White Herb, Mental Herb: they cure the **target**) else add a secondary `{status}` (Light Ball: par; Poison Barb: psn) or `{volatileStatus}` (King's Rock: flinch), which draws `random(100)` like any secondary and always applies (no chance); (6) `source.addVolatile('fling')`: the volatile's `onUpdate` (next Update) removes the item from the user (`setItem('')`, `lastItem = item.id`, `usedItemThisTurn = true`, `AfterUseItem`). **The item is removed whether or not the move hits** if `onPrepareHit` ran (a miss, Protect, a Ghost-immune Normal... `PrepareHit` is before accuracy): the volatile is added in `onPrepareHit`, so a Fling at a Protect user still loses the item. Fling does not run `takeItem`, so Sticky Hold of either side is not consulted, and Unburden fires through `AfterUseItem`. Magician excludes Fling (abilities.ts:2481). |

**Fling base power per legal item** (`generated.rs` `fling_power`, which equals `item.fling.basePower` after the constructor defaults at `dex-items.ts:146-149`): 130 Iron Ball; 100 Hard Stone; 80 Quick Claw and every Mega stone (stones fail anyway); 70 Dragon Fang, Poison Barb (poison); 60 Damp Rock, Heat Rock, Leek, Rocky Helmet, Terrain Extender; 50 Sharp Beak; 40 Icy Rock; 30 Binding Band, Black Belt, Black Glasses, Charcoal, Eject Button, King's Rock (flinch), Life Orb, Light Ball (paralysis), Light Clay, Magnet, Metal Coat, Metronome, Miracle Seed, Mystic Water, Never-Melt Ice, Scope Lens, Shell Bell, Spell Tag, Twisted Spoon; 10 every berry (all 28), Air Balloon, Big Root, Bright Powder, Choice Scarf, Electric/Grassy/Misty/Psychic Seed, Expert Belt, Fairy Feather, Focus Band, Focus Sash, Leftovers, Mental Herb, Muscle Band, Red Card, Shed Shell, Silk Scarf, Silver Powder, Smooth Rock, Soft Sand, White Herb, Wide Lens, Wise Glasses, Zoom Lens; **no fling (fails): Normal Gem**. Berries fed through Fling: the effect is the berry's own `onEat` on the target (Sitrus heals the target 1/4, a status berry cures it, a resist berry does nothing, Leppa restores PP).

**Interaction table (items x abilities/field), from the code above:**

| interaction | result | cite |
|---|---|---|
| Sticky Hold target vs Knock Off | item stays; the x1.5 power still applies | abilities.ts:4623-4631 (needs `source !== pokemon` or Knock Off); moves.ts:9972 |
| Sticky Hold target vs Thief, Covet, Bug Bite, Pluck, Corrosive Gas, Magician, Pickpocket | `takeItem(source)` false: nothing happens (Bug Bite/Pluck deal damage only) | abilities.ts:4627 |
| Sticky Hold target vs Trick, Switcheroo | move fails in `onTryImmunity` (whole move, no item change) | moves.ts:19863 |
| User with Sticky Hold using Trick | not blocked (`source.takeItem()` has `source === holder`) | abilities.ts:4627-4628 |
| Klutz holder | no item effects (all legal items lack `ignoreKlutz`); Fling fails; Acrobatics does not double with an item; Knock Off still removes it; Trick/Thief still move it; a Klutz holder that receives a Choice Scarf is not locked (`ignoringItem`) | pokemon.ts:900-907, items.ts:1007; `abilities.ts` Klutz block 2279-2290 runs `End` on the item at switch-in |
| Unburden | triggers on `AfterUseItem` (own item used, eaten, Fling, Air Balloon pop) and on `TakeItem` (knocked, tricked, stolen); volatile gives x2 Speed while `!pokemon.item`; ends when an item is received | abilities.ts:5237-5258 |
| Magic Room (5 turns) | every item stops acting (`ignoringItem`); `End` is run on each active item at start; `hasItem` false; **Fling fails, Poltergeist still works, Acrobatics reads raw item**; Light Clay and the rocks read `hasItem` at set time so they stop working; seeds/herbs/berries do nothing (their `onUpdate`/events are skipped) | moves.ts:10756-10790; pokemon.ts:900 |
| Embargo | not legal (move absent) | - |
| Unnerve (foe) | the holder cannot eat berries (`TryEatItem` false): resist berries, Sitrus, status berries, Oran, Leppa do nothing and stay held; force-eat (`eatItem(true)`: Stuff Cheeks, Teatime) skips the test; Bug Bite/Pluck/Fling ignore it | abilities.ts:5260-5276; pokemon.ts:1799 |
| Gluttony | no legal item reads it | abilities.ts:1619-1626 |
| Cheek Pouch | heals `baseMaxhp/3` on every `EatItem` (also for forced eats) | abilities.ts:492-496 |
| Cud Chew | re-eats the berry at the end of the next turn (`onResidualOrder 28`, subOrder 2) unless it came from Bug Bite/Pluck | abilities.ts:742-770 |
| Harvest | restores `lastItem` if it is a berry: in sun always, otherwise `randomChance(1,2)` (draw) at residual order 28/2 | abilities.ts:1801-1814 |
| Ripen | doubles berry heals (`onTryHeal`), boost amounts and the resist-berry effect (a second halving) | abilities.ts:3843-3873 |
| Symbiosis | gives the holder's item to an ally (doubles only) | abilities.ts:4839 |


## 3. Mega Evolution in the Champions mod

The mod is `gen: 9` (`ch/scripts.ts:2`), so the Gen 8+ queue rules apply. Everything below is from `ch/scripts.ts` (overrides), `sim/battle-actions.ts`, `sim/battle.ts`, `sim/battle-queue.ts`, `sim/side.ts`, `sim/pokemon.ts`.

### 3.1 When Mega Evolution is offered and legal

1. **Eligibility is fixed when the Pokémon object is built.** `Pokemon` constructor: `this.canMegaEvo = this.battle.actions.canMegaEvo(this)` (`pokemon.ts:511`). The Champions override (`ch/scripts.ts:182-194`) returns `item.megaStone?.[pokemon.baseSpecies.name]` (the Rayquaza branch needs gen <= 7 or the past/future tags; ignore it, there is no Rayquaza). So: the held stone must list the Pokémon's `baseSpecies.name` (Meowstic-F and Floette-Eternal are separate keys). It is **never recomputed** when the item changes (`pokemon.ts` has no recomputation; only Power Construct and Mega Rayquaza Clause touch it, `abilities.ts:3393`, `rulesets.ts:1567`). Since a stone cannot leave its own species and a stone on another species is never legal, "built with the stone" and "holds the stone" coincide on legal teams.
2. **One Mega per battle per side.** `Side.chooseMove` rejects `mega` when `this.choice.mega` is already set this turn (`side.ts:780-782`), and `runMegaEvo` clears `canMegaEvo` on **every** Pokémon of the side after one evolves (`battle-actions.ts:1958-1964`), including benched ones and ones that later return. A Mega form never reverts, even on fainting or switching (`ch/scripts.ts:55-56`, `formeChange` sets `baseSpecies` = the Mega form).
3. **Not offered** (the `canMegaEvo` field is left out of the move request) when: the Pokémon has a locked move or is forced to Struggle/recharge (`getMoveRequestData`: `if (!lockedMove) { if (this.canMegaEvo) data.canMegaEvo = true ...`, `pokemon.ts:1158-1161`; `lockedMove` includes `lockedmove` (Outrage, Thrash, Petal Dance), `twoturnmove` mid-way, `mustrecharge`, Struggle); when every move is disabled or Struggle (`updateDisabledRequest`, `side.ts:865-870`); when it is not the move-choice request (a switch choice cannot carry `mega`: `chooseMove` is the only place that reads `event === 'mega'`, `side.ts:767-785`). Register N12 (engine offers Mega to an Outrage-locked Pokémon). `options.rs:158-181` already covers locked, all-disabled and Struggle.
4. **Choice syntax** is `move <slot> mega`: Mega is a flag of a move choice, not an action of its own; the Mega Pokémon must also use a move that turn. A move choice with `mega` that is rejected later (the move is disabled) cancels both.

### 3.2 When it happens in the turn

1. `BattleQueue.resolveAction` turns a move choice that has `mega` into **two** actions: `megaEvo` (order **104**) followed by the move (order 200); `battle-queue.ts:211-216` (unshifted so Mega comes before the move), orders at `battle-queue.ts:184-186`. Other orders: `team` 1, `start` 2, `instaswitch` 3, `beforeTurn` 4, `beforeTurnMove` 5, `revivalblessing` 6, `runSwitch` 101, `switch` 103, **`megaEvo` 104**, `runDynamax` 105, `terastallize` 106, `priorityChargeMove` 107, `shift` 200, move 200, `residual` 300. So **all switches (103) and their entry effects (101) happen first, then every Mega Evolution, then all moves.** Not in the move order: a Mega Pokémon whose opponent switches in the same turn evolves after the opposing Pokémon has entered. (`Pursuit` has a special early Mega, `moves.ts:14411-14424`; Pursuit is not legal here: absent from the move table.)
2. **Order among the two Megas** (both sides evolve in one turn): `comparePriority` (`battle.ts:409-417`): order, then priority (0 for non-move actions), then speed high first, then random tie via `speedSort`'s shuffle (`battle.ts:434-475`). The speed is `action.pokemon.getActionSpeed()`, **the Champions override** (`ch/scripts.ts:46-54`): `getStat('spe', false, false)` (boosts, paralysis, Scarf, Tailwind all included), negated under Trick Room with no 16-bit underflow. It is computed at queue-build time, before any Mega has evolved, and **not** re-sorted between the two Mega actions (the re-sort below needs the next action to be a move).
3. **Speed on the Mega turn: Showdown uses the new (post-Mega) Speed for the move order.** After every action `runAction` ends with (`battle.ts:2975-2984` (the `if` is at 2977)): `if (this.gen >= 8 && nextAction && ['move','runDynamax'].includes(nextAction.choice)) { this.updateSpeed(); for (const queueAction of this.queue.list) if (queueAction.pokemon) this.getActionSpeed(queueAction); this.queue.sort(); }`. When the last Mega action finishes and the next queued action is a move, every queued action's `speed` **and** `priority` are recomputed (`getActionSpeed`, `battle.ts:2672-2717`, which re-runs `ModifyPriority`: Prankster, Gale Wings, Triage, Grassy Glide, and re-adds the stored `fractionalPriority`, which is **not** re-rolled) and the queue is re-sorted with the shuffle on exact ties. Consequences: a Mega that gains Speed (Garchomp 169 -> 223 in the gold team) outspeeds what it did not outspeed before; **Mega Banette's new Prankster applies to its status move the same turn** (its move's priority is recomputed after the evolution); a Mega whose ability changes a priority modifier or a speed modifier (Swift Swim in rain, Sand Force n/a) uses it. CG register N13 and `mechanics-turn.md` Q11/Q12 are this: the old engine used the pre-Mega speed; the new resolver's re-sort (`battle.rs:625-633`, runs `update_speed` and `sort_queue` when the next action is a Move) matches. Q12: two Megas at equal speed are a coin flip (`prng.shuffle` draw), under Trick Room speeds are negated. The same re-sort also happens after **every** later action while the next action is a move, but in singles only two move actions exist, so it matters only after the Mega/switch actions and after pivots.
4. The move action itself keeps `action.mega` truthy; the evolution is not repeated.
5. **Action text:** `-mega` is printed, and `moveThisTurnResult = true` is set so Mega Evolution counts as the Pokémon's action for **Truant** (`ch/scripts.ts:91-92`). `battle.ts:1691` copies it into `moveLastTurnResult` at the next turn start. Check that `run_mega_evo` (`battle.rs:1056-1070`) sets `move_this_turn_result`.
6. If the Mega user later cannot move (asleep, flinched, fully paralysed, confused self-hit) it has still evolved; the evolution cannot be flinched because it precedes all moves. If it faints before moving it is irrelevant (Mega actions come first; only Pursuit or a hazard cannot hit before them).

### 3.3 What `runMegaEvo` changes (`battle-actions.ts:1952-1970`, `ch/scripts.ts:56-121`)

`runMegaEvo(pokemon)`: `speciesid = pokemon.canMegaEvo`; `pokemon.formeChange(speciesid, pokemon.getItem(), true)`; then `canMegaEvo = false` on all side Pokémon (`battle-actions.ts:1958-1964`); then `runEvent('AfterMega', pokemon)` (`battle-actions.ts:1968`; `onAnyAfterMega` of White Herb, items.ts:7720, Opportunist abilities.ts:3060, rulesets). `formeChange(species, source=item, isPermanent=true, abilitySlot='0')`:

| what | Showdown | cite |
|---|---|---|
| species and `baseSpecies` | `setSpecies(Mega)`, then `this.baseSpecies = rawSpecies`: the Mega form is permanent | `ch/scripts.ts:59, 68` |
| types | `setType(species.types, true)` (resets the types to the Mega's; clears Roost's `???`-style changes and Forest's Curse via `addedType = species.addedType`) | `pokemon.ts:1413-1416` |
| weight | `weighthg = species.weighthg` (the Mega's own weight, e.g. Charizard 905 -> X 1105 / Y 1005; table in 1.4) | `pokemon.ts:1417` |
| stats | `spreadModify(species.baseStats, this.set)` -> new `storedStats` for atk/def/spa/spd/spe with the set's stat points and nature through the Champions `statModify` (`ch/scripts.ts:10-40`: HP `base + sp + 75`, others `base + sp + 20`, nature `tr(tr(stat*110,16)/100)` or `*90`); `baseStoredStats` too | `pokemon.ts:1419-1439`; `ch/scripts.ts:24-38` |
| HP | `updateMaxHp()`: recomputes `baseMaxhp`; only changes if the Mega's base HP differs (none of the 82 do: checked), then current HP is shifted by the difference | `pokemon.ts:1519-1527`; `ch/scripts.ts:73` |
| ability | `ability = species.abilities['0']` (slot 0 of the Mega, always one ability); `setAbility(ability, null, null, true)`: `singleEvent('End', oldAbility)` then new `abilityState` then `singleEvent('Start', newAbility)` **immediately**; `baseAbility` = the Mega ability, so it persists after a switch | `ch/scripts.ts:110-114`; `pokemon.ts:1929-1960` |
| item | unchanged; the stone stays held and unremovable | n/a |
| boosts, status, volatiles, moves, PP, current HP, `lastMove`, Substitute | unchanged (no `clearVolatile`) | not touched |
| turn flags | `moveThisTurnResult = true`; `canTerastallize = null` | `ch/scripts.ts:78, 92` |
| speed cache | `this.speed = storedStats.spe` in `setSpecies`; recomputed by `updateSpeed()` before the re-sort | `pokemon.ts:1439`, `battle.ts:2976` |

The new ability's **Start** runs inside the Mega action, so effects appear before any move: Drought (Charizard-Mega-Y) / Sand Stream (Tyranitar-Mega) / Snow Warning (Abomasnow-Mega, Froslass-Mega) / Electric Surge (Raichu-Mega-X) set weather or terrain with the Mega as the source (so Heat Rock and friends extend them, section 1.3); Intimidate (Manectric-Mega, Scrafty-Mega) lowers the foe's Attack at once (a foe's White Herb then fires at `AfterMega`; a foe with Defiant/Competitive/Guard Dog/Mirror Armor reacts); Trace (Alakazam-Mega, Meowstic-M/F-Mega; also `onUpdate`) copies the foe's ability; Fairy Aura (Floette-Mega); Mold Breaker (Ampharos, Emboar, Gyarados-Mega) only prints its message at Start; Thermal Exchange (Baxcalibur-Mega) has `onUpdate`. The handlers of the **old** ability get `End` (Emergency Exit has none; Intimidate none; Slow Start etc. irrelevant).

### 3.4 Once per battle, once per side; Mega is not a switch

One Mega Evolution per side per battle (3.1 item 2). A Mega Pokémon that is switched out and back keeps its Mega form, ability slot 0 and stone. Revival Blessing (legal) revives a fainted Mega Pokémon as the Mega form (formes are not reverted on faint, `ch/scripts.ts:55`).

### 3.5 Every legal Mega, with stone, base species, Mega ability

See the stone table in 1.4 (81 stones, 82 forms; types, Mega base stats, ability and weight columns). Champions-specific facts:
- The Mega abilities **new to Champions** (`ch/abilities.ts`, flipped to legal): Aura Guard (Lucario-Mega-Z), Dragonize (Feraligatr-Mega), Eelevate (Eelektross-Mega), Fire Mane (Pyroar-Mega), Mega Sol (Meganium-Mega), Piercing Drill (Excadrill-Mega), Spicy Spray (Scovillain-Mega). All other Mega abilities are base Gen 9 abilities. Champions also overrides Unseen Fist (Golurk-Mega: Protect-bypass for contact moves only, quarter damage; `ch/abilities.ts:91-100`), Healer (Audino-Mega; ally-only so no singles effect), Regenerator (Dragalge-Mega: heals `baseMaxhp/3` on switch-out), Natural Cure (Altaria... base, not Mega), Berserk (Drampa-Mega: trigger rule change), Wimp Out/Emergency Exit (the pre-Mega Golisopod ability).
- Three Z Megas exist only in Champions: **Absol-Mega-Z** (Absolite Z; Dark/Ghost 65/154/60/75/60/151, Sharpness), **Garchomp-Mega-Z** (Garchompite Z; pure Dragon 108/130/85/141/85/151, Levitate), **Lucario-Mega-Z** (Lucarionite Z; Fighting/Steel 70/100/70/164/70/151, Aura Guard). Charizard X/Y and Raichu X/Y each have two stones. Meowstic-M and -F use one stone. Floette's stone requires Floette-Eternal.
- The engine's `generated.rs` Mega rows already carry the corrected values for the register's SP1-SP6 (checked in this session: Garchomp-Mega-Z Levitate, Golisopod-Mega Tough Claws, Absol-Mega-Z Sharpness, Lucario-Mega-Z Aura Guard present in the ability table, Starmie-Mega Atk 100, Drampa-Mega SpD 116) and carry weights (SP12).

### 3.6 The gold team's Megas in detail

Team: `research/pcsa-rl/runs/team-gold/team.txt` (read-only). Level 50, stat points as listed.

**Garchomp @ Garchompite Z**, ability Rough Skin, Timid, 2 HP / 32 SpA / 32 Spe, moves Draco Meteor, Earth Power, Power Gem, Flamethrower.
- Pre-Mega: Dragon/Ground, 108/130/95/80/85/102 base (the engine's species table supplies these), weight 950 hg, ability Rough Skin (set), contact damage 1/8 (`onDamagingHitOrder: 1`).
- Mega-Z: **pure Dragon** (no Ground, so Earth Power and Ground STAB are lost; Earth Power is still its own Ground-type move at base 90), 108/130/85/141/85/151, weight 990 hg, ability **Levitate** (slot 0; Rough Skin is gone: no contact punishment after evolving) -> immune to Ground moves through `isGrounded() === null` (`pokemon.ts:2169-2180`, immunity message `pokemon.ts:2282-2284`) unless grounded by Gravity, Smack Down, Ingrain or an Iron Ball. Register **SP1**: the old engine gave Sand Force (wrongly took Ground moves).
- Stat example (computed by hand from `ch/scripts.ts:10-40`, not by running Showdown): Speed pre-Mega `tr(tr((102+32+20)*110,16)/100)` = `tr(16940/100)` = **169**; Mega `tr(tr((151+32+20)*110,16)/100)` = `tr(22330/100)` = **223**. SpA Mega `141+32+20` = 193 (Timid does not touch SpA). HP `108+2+75` = 185 both. Atk Mega `tr(tr((130+0+20)*90,16)/100)` = `tr(13500/100)` = 135 (Timid lowers Attack).
- Mega speed matters: after evolving it moves with 223, so on the Mega turn it outspeeds a Speed-200 foe it would not at 169 (register N13). Under Trick Room both stats are negated for ordering.
- Against Gholdengo's Air Balloon (foe's team mirror) etc. see the tests in section 6.

**Golisopod @ Golisopite**, ability Emergency Exit, Adamant, 32 HP / 32 Atk / 2 SpD, moves Leech Life, First Impression, Sucker Punch, Iron Head.
- Pre-Mega: Bug/Water 75/125/140/60/90/40, weight 1080 hg, **Emergency Exit** (Champions override `ch/abilities.ts:26-34`: when its HP falls from above half to half or below, and it can switch, set `switchFlag`; `onEmergencyExit` events run after damage, after recoil and after residual).
- Mega: Bug/Steel 75/150/175/70/120/40, weight 1480 hg, ability **Tough Claws** (slot 0): contact moves x5325/4096 on base power (`onBasePower`, the register's SP2; the old engine gave Emergency Exit, a spurious switch-out and no x1.3). Mega Golisopod does not switch at half HP because Emergency Exit is gone.
- Example: Atk Mega `tr(tr((150+32+20)*110,16)/100)` = `tr(22220/100)` = **222**; pre-Mega `125+32+20` = 177 -> `tr(19470/100)` = 194. HP `75+32+75` = 182. SpD Mega `120+2+20` = 142. Speed `40+0+20` = 60 (Adamant is neutral on Speed) before and after.
- Turn-one play: the Mega action precedes the move; **First Impression** (priority +2, works only on the first turn out: `activeMoveActions` is unchanged by the Mega action) and **Sucker Punch** both still work after Mega Evolving (Mega is not a move action). Priority is 2 and 1 respectively so Mega Golisopod's lower Speed does not matter against non-priority foes.
- Interaction: Golisopod's stone is also protected from Knock Off/Thief/Trick for the rest of the game; if Golisopod uses Trick-like moves the holder cannot (it has none).


## 4. Hooks and events the items and Mega Evolution need

Checked against `engine/src/sim/hooks.rs` (the `hooks!` list at lines 22-42 and `Hook::of_handler` at 55-84), `events.rs` (the `events!` list at lines 41-50, `Pre`, `Holder` at 233-247, `table()` at 103-140) and `state.rs` (`Mon` at 106-200). "present" means the name exists; it says nothing about whether a handler is implemented.

### 4.1 Showdown events and callbacks that legal items use (from the 85 non-stone `handlers` lists, plus the volatiles items create)

| Showdown handler / event | used by | `Hook` in hooks.rs | `Ev` in events.rs | comment |
|---|---|---|---|---|
| `onBasePower` (priority 14, 15, 16) | 18 type boosters, Muscle Band, Wise Glasses, Gem volatile | present (`BasePower`) | present | priorities 14/15/16 must be honoured (`nums`) |
| `onModifyDamage` (own) and `onSourceModifyDamage` | Life Orb, Expert Belt, Metronome / 18 resist berries | `ModifyDamage` (one hook for both) | `ModifyDamage` + `Pre::Source` | distinguish the own handler from the Source handler: the berries run on the **target**'s item with the attacker as event source |
| `onModifyAtk`, `onModifySpA` (priority 1) | Light Ball | present | present | |
| `onModifySpe` | Choice Scarf, Iron Ball | present | present | |
| `onModifyAccuracy`, `onSourceModifyAccuracy` (priority -2) | Bright Powder / Wide Lens, Zoom Lens | `ModifyAccuracy` | present | Zoom Lens needs `queue.willMove(target)` |
| `onModifyCritRatio` | Scope Lens, Leek | present | present | |
| `onModifyMove` (priority -1) | King's Rock; Choice Scarf (adds lock) | present | present | King's Rock appends a secondary |
| `onFractionalPriority` (priority -2) | Quick Claw | present | present | needs the item as a handler holder in the action builder (`battle.rs:375-385` already calls `FractionalPriority`) |
| `onEffectiveness` | Iron Ball | present | present | |
| `onDamage` (priority -40) | Focus Sash, Focus Band | `Damage` | present | Focus Band draws on every Damage event |
| `onTryHeal` (priority 1) | Big Root | present | present | relay-value chain on a heal amount |
| `onDamagingHit` (order 2) | Rocky Helmet, Air Balloon | present | present | left-to-right sort, not speed |
| `onAfterMoveSecondary` (priority 2 for Eject Button) | Eject Button, Red Card | present | present | |
| `onAfterMoveSecondarySelf` (priority -1 for Shell Bell) | Life Orb, Shell Bell | present | present | |
| `onTryPrimaryHit` (Source) | Normal Gem | present | present | |
| **`onAfterSubDamage`** | Air Balloon | **mapped to `TryPrimaryHit` (wrong place)** | **missing** | needs its own event run after a Substitute absorbs a move (`moves.ts:18364-18365`) |
| `onUpdate` | 7 status berries, Sitrus, Oran, Leppa, Mental Herb, Lum | present | present | `Update` runs per action and per hit (section 1.1 item 5) |
| `onStart` (via the SwitchIn event) | Air Balloon, Metronome, seeds, White Herb | `Start`, plus the `holder_mask_of` SwitchIn bit | via `SwitchIn` | also run by `setItem` (Trick, Covet, Thief, Recycle, Harvest) with `singleEvent('Start')`: an item-acquisition path is needed |
| `onSwitchInPriority: -1`, `onAnySwitchIn` (priority -2) | seeds / White Herb | present | present | |
| **`onTerrainChange`** | 4 seeds | **mapped to `Residual`** | present (`TerrainChange`) | wrong mask only; the event is run via `eachEvent` with no prefixes (`events.rs:355-357` already lists it) |
| **`onAfterSetStatus`** (priority -1) | Lum Berry | **mapped to `Immunity`** | present (`AfterSetStatus`) | needs its own hook; it runs after the status is set |
| **`onTryEatItem`, `onEat`, `onUse`, `onTakeItem`, `onEatItem`, `onUseItem`, `onAfterUseItem`, `onAfterTakeItem`** | berries, White Herb, seeds, stones (`onTakeItem`), Unburden, Cheek Pouch, Cud Chew, Unnerve | **all fall into `Other`** | `TakeItem`, `TryEatItem`, `EatItem`, `AfterUseItem` present; **`UseItem`, `AfterTakeItem` missing**; `Eat` and `Use` are single events (no Ev needed) | the item-use pipeline of section 1.1 item 4 needs hooks for these |
| `onResidual` (order 5/sub 4; order 29) | Leftovers; White Herb | `Residual` | not in `Ev` (`residual.rs` has `Holder::Item` with sub-order 8, `residual.rs:109`) | give Leftovers `order 5, subOrder 4`, not 8 (use the `nums` in `generated.rs`) |
| `onMaybeTrapPokemon`, `onTrapPokemon` (priority -10) | Shed Shell | present | present | |
| `onAnyAfterMega`, `onAnyAfterMove` | White Herb | `AfterMega`, `AfterMove` | `AfterMega`, `AfterMove` + `Pre::Any` | `run_mega_evo` only calls `run_field_hook(Hook::AfterMega)` (`battle.rs:1069`), which only notes the effect: the real event must be run |
| `onDragOut` check by Red Card | Red Card | `DragOut` present | present | |
| `durationCallback` reads `source.hasItem()` | Damp/Heat/Smooth/Icy Rock, Light Clay, Terrain Extender; Binding Band via `boundDivisor` in `onStart` | `of_handler` maps `durationCallback` to `Start` | n/a | not an event: a `hasItem(source, id)` helper (with `ignoringItem`) at the six set points (weather, terrain, three screens, partial trap) |
| item volatiles: `choicelock`, `metronome`, `gem`, `fling` | Choice Scarf, Metronome, Normal Gem, Fling | not in `IMPLEMENTED_CONDS` (`hooks.rs:105-122`) | n/a | `unburden` is already implemented; the four above are not |

### 4.2 Structural gaps that are not hook names

1. `events::Holder` has no `Item` variant (`events.rs:233-247`); `table()` builds sort keys for conditions and abilities only (`events.rs:103-140`, `ab_index`). Items need a third index range (an `ITEMS` table of `handlers`/`nums` already exists in `generated.rs`), `Holder::Item(side)`, the default sub-order 8 (`battle.ts:976`), and the `ignoringItem` suppression in the handler collector (`battle.ts:881-889`, `614-617`).
2. `Mon` has no `last_item`, `ate_berry`, `used_item_this_turn`, or per-item state (Metronome's last move and count, Choice lock move, White Herb temporary boosts). `Pokemon.isGrounded` and `ignoringItem` helpers (Klutz, Magic Room) must read `item` through one function.
3. Items can change at run time (Trick, Switcheroo, Covet, Thief, Knock Off, Recycle, Harvest, eating): every change must go through `takeItem`/`setItem`/`useItem`/`eatItem`-equivalents so that End/Start, `TakeItem` (stone and Sticky Hold), `AfterUseItem` (Unburden) and the staleness bookkeeping all fire.
4. `Update` must keep running at the exact points listed in 1.1 item 5 (shuffle draws on speed ties).
5. Mega: `run_mega_evo` (`battle.rs:1056-1070`) already sets species, `base_species`, HP recompute, ability, clears `can_mega` and sets `mega_spent`. Missing or to check: `move_this_turn_result = true` (Truant), `-mega` is cosmetic, running the `AfterMega` event for real (White Herb), `Hook::ModifySpecies` is only noted. The post-Mega re-sort exists (`battle.rs:625-633`).

### 4.3 Hooks believed missing, summary

`AfterSubDamage` (Air Balloon), `UseItem`, `AfterTakeItem`, a distinct `AfterSetStatus` hook (currently collapsed into `Immunity`), a distinct `TerrainChange` hook (currently `Residual`), item-use pipeline hooks for `TryEatItem`/`TakeItem`/`EatItem`/`AfterUseItem` (currently `Other`), `Holder::Item` in `events.rs`, the four item volatile conditions, and the item-acquisition `Start` path.

## 5. Known traps

### 5.1 Register findings about items and Megas, with the behaviour to implement

| id | fault in the old engine | correct behaviour (cite) |
|---|---|---|
| I1 (gold), A02-3, M04-10 | Focus Sash/Sturdy/Endure subtract 1 from every full-HP hit; sash never used up | `onDamage` priority -40: only when `hp === maxhp && damage >= hp && effect is a Move`, then `useItem()` and return `hp - 1`; one hit only (items.ts:2287-2294). Sturdy (priority -30) runs first and keeps the sash |
| I2, I3, N2 (items) | resist berries, Normal Gem, Shuca eaten before accuracy, Protect, on Status moves, on every hit | inside the damage step only (`ModifyDamage`), damaging moves, Substitute hits excluded, first hit only; Normal Gem at `TryPrimaryHit` for non-Status moves (items.ts:5722-5733, 4346-4351) |
| I4, T1, W5 | Leftovers after poison/burn | residual order 5/4, before Leech Seed 8, poison 9, burn 10 (items.ts:3356-3357) |
| I5, F9, W21, W22 | Rocky Helmet 0.166, once per move, ignores Magic Guard | `baseMaxhp/6` truncated, per hit, `DamagingHit` order 2, Magic Guard blocks (items.ts:5325-5330) |
| I6 | Shell Bell replaces the move's drain | independent heal `totalDamage/8` after the move (items.ts:5680-5685) |
| I7, T4 | Sitrus/Lum/Pecha/Rawst timing; Lum ignores confusion | all at `Update` (immediately after damage / after each action); Lum also on `AfterSetStatus`; Lum and Persim cure confusion; Sitrus at 1/2 always (items.ts:5776, 3563-3575) |
| I8 | seeds only on entry | also on `TerrainChange`; consumed even when the stat is capped (items.ts:2613-2623; `battle.ts:1066` event list) |
| I9 | Iron Ball does not lift Flying/Levitate immunity | `isGrounded` true first (`pokemon.ts:2174`) and `onEffectiveness` returns 0 for Ground vs Flying (items.ts:3068-3072) |
| I10, F6 | accuracy items do nothing | Wide Lens 4505/4096, Zoom Lens 4915/4096 (target already moved), Bright Powder 3686/4096 on accuracy, priority -2 |
| I11 / C4 / W26 | 22 items unknown to the engine: Light Clay, Terrain Extender, Quick Claw, Damp/Heat/Smooth/Icy Rock, Eject Button, Red Card, Scope Lens, King's Rock, Zoom Lens, Leek, Big Root, Bright Powder, Binding Band, Focus Band, Leppa, Mental Herb, Metronome, Oran, Persim | the new `ITEMS` table has all 166; implement each per 1.3 |
| I12, M1 (items-01) | a Mega stone holder can Trick its own stone away | Trick/Switcheroo fail when either `takeItem` is false (moves.ts:19873-19888; section 2) |
| I13, W27, A02-6, A05-11 | Unnerve, Klutz, Embargo, Magic Room do nothing | `ignoringItem` (Klutz, Magic Room; Embargo not legal) and Unnerve's `TryEatItem` block (section 1.1 items 3-4) |
| W24, T12 | forced switches from Eject Button, Red Card (and Emergency Exit) | section 1.3; Eject Button Champions override; Red Card draws a random replacement |
| T17, M07-10, C4, W17, W18, W19 | Light Clay, Terrain Extender, rocks do nothing | 8 turns when the setter holds the item at set time (section 1.3) |
| F12, W23 | Scope Lens, Leek do nothing | crit stage +1, +2 for Farfetch'd/Sirfetch'd (items.ts:5584, 3340) |
| M08-4, M09-5, M09-3, M10-4, M06-2, M08-3 | Covet, Bug Bite, Pluck, Recycle, Stuff Cheeks, Teatime, Belch, Fling, Corrosive Gas wrong or missing | section 2 |
| SR-26, N1 (items-01) | Choice lock outlives the Choice item | the volatile removes itself when the holder has no Choice item (`conditions.ts:333-336` and `348-351`) |
| SR-29 | Light Ball doubles the move's power | doubles Atk and SpA (items.ts:3446-3457) |
| N1 (register, found by the rig) | the move counts as used (and locks a Choice item) before the Pokémon can move | the lock is added in `onModifyMove`, which runs in `useMove` after BeforeMove (sleep, flinch, full paralysis, confusion) passed (`battle-actions.ts:503`) |
| SP1 (gold) | Mega Garchomp Z has Sand Force | Levitate (pokedex.ts:8185); fixed in `generated.rs` |
| SP2 (gold) | Mega Golisopod has Emergency Exit | Tough Claws (pokedex.ts:14533); fixed in `generated.rs` |
| SP3, SP4, SP5, SP6 | Absol Z Magic Bounce; Lucario Z Aura Guard missing; Starmie Atk 140; Drampa SpD 114 | Sharpness; Aura Guard (`auraguard` exists in the new table); Atk 100; SpD 116; all four are correct in `generated.rs` |
| SP8 | Mega Kangaskhan's Parental Bond has no effect | `onPrepareHit` + `onSourceModifySecondaries` (hits twice, second hit 0.25x: `ch/scripts.ts:208-212`) - ability slice, but it starts working at Mega |
| SP12, W11 | Mega Evolution does not change weight | `weighthg` = the Mega's (1.4 table); Low Kick/Grass Knot/Heavy Slam/Heat Crash read it (the new `set_species` sets `weight_hg`) |
| N12 | Mega offered to an Outrage-locked Pokémon | not offered when locked, struggling, recharging or all moves disabled (3.1 item 3) |
| N13, Q11, Q12, K8 | Mega-turn move order uses pre-Mega speed | the queue re-sorts with post-Mega speed and priority (3.2 item 3); two Megas tie-break with a shuffle draw |
| W8 | two Pokémon entering at once order | by speed (not an item rule; White Herb `onAnySwitchIn` priority -2) |

### 5.2 New traps found in this read (ids IM1-IM18; CLAUDE.md asks every finding for an id)

- **IM1. Icy Rock is not inert.** It extends Hail and Snow to 8 turns (`conditions.ts:675` and `705`). `items-03.md` marked it "none in Showdown data read, low confidence" and the register lists Icy Rock as one of the seven unknown legal items (C4) correctly but with no behaviour.
- **IM2. Knock Off x1.5 is not stopped by Sticky Hold.** The power test calls `singleEvent('TakeItem', item, ...)` which only reads the item's own `onTakeItem` (moves.ts:9972); Sticky Hold is an ability handler reached only through `takeItem()`'s `runEvent`. `items-02.md` N5 says the opposite. Only a Mega stone of the holder's species removes the boost.
- **IM3. Focus Band's roll happens on every Damage event** of the holder, lethal or not, any source (items.ts:2272), before the lethality test. Put the draw there or every following draw shifts.
- **IM4. Sheer Force does not suppress `AfterMoveSecondary` in this mod** (`ch/scripts.ts:575` has no `suppressingSecondaries()` test, the base game does at `battle-actions.ts:1064`), but it does suppress `AfterMoveSecondarySelf` (Life Orb recoil, Shell Bell, `battle-actions.ts:600`). Eject Button, Red Card and Pickpocket therefore fire after a Sheer Force move. Likely unintended by the mod's authors; implement as written.
- **IM5. Champions' Eject Button does not clear the attacker's pending switch** (`ch/items.ts:262-278` vs `items.ts:1708`). After a pivot move (U-turn) that hits an Eject Button holder, both Pokémon switch (the attacker's `switchFlag` stays; **unsure** how the two requests are ordered; `battle.ts` `switches` handling after `runAction`, not read).
- **IM6. Sitrus ignores Gluttony** in Champions' legal set; always at 1/2. The engine's Gluttony arm does not apply.
- **IM7. Modify-move order for King's Rock**: Sheer Force (0) deletes secondaries, then King's Rock (-1) adds a flinch secondary, then Serene Grace (-2) doubles it. The flinch appears even for a Sheer Force holder with King's Rock.
- **IM8. Quick Claw's roll is at queue-build time**, once per move action, before the turn runs, and it is not re-rolled by the post-Mega re-sort or by any re-sort.
- **IM9. Champions removes the request-level disable for Belch and Stuff Cheeks** (`ch/moves.ts:52-55`, `989-992`): the move is always selectable; failure comes from `onTry`, and it costs PP.
- **IM10. Fling removes the item after `PrepareHit` whether or not the move hits** (volatile `fling` in `onUpdate`); a Fling into Protect still loses the item. Berry Fling makes the target eat it (its `onEat` runs on the target).
- **IM11. The post-Mega re-sort re-runs `ModifyPriority`**, so Mega Banette's Prankster boosts its status move on the Mega turn.
- **IM12. `canMegaEvo` is fixed at construction**; a Pokémon that receives a stone later (possible only on an illegal team) cannot Mega Evolve.
- **IM13. A seed is consumed even when its boost is capped** (`useItem` does not test the boost result), and the stat boost goes through `battle.boost` (Clear Body etc. apply; Contrary flips it).
- **IM14. The weather rocks, Light Clay and Terrain Extender read the setter's item at the moment the effect starts** through `hasItem`, so Klutz and Magic Room switch them off, and abilities (Drought etc., including Mega abilities) count with the ability holder as setter.
- **IM15. Red Card draws a `sample` over the possible replacements even with a single candidate** (`random(1)` still advances the generator).
- **IM16. Every `Update` with a speed tie between the two actives draws one shuffle**, regardless of items. Items add Update-driven effects, not draws.
- **IM17. Normal Gem uses `onSourceTryPrimaryHit`**, i.e. at the target-reaching step, not at move start; the gem condition lasts the rest of that move (`conditions.ts:467-474`, duration 1).
- **IM18. Light Ball checks `baseSpecies.baseSpecies === 'Pikachu'`** (items.ts:3448, 3454): every Pikachu form qualifies, no other species (Raichu does not).


## 6. Test list: rare interactions that random play will seldom reach

One line each: setup -> expected. IDs T-I* = items, T-M* = Mega. "Expected" is from the cited Showdown code; none was run against a live Showdown in this session (read-only task), so the first run of each against the Showdown oracle (`tools/showdown-oracle`) is itself the check.

Items
- T-I1. Full-HP Focus Sash holder, 3-hit move: hit 1 leaves 1 HP and the sash is gone; hit 2 kills (items.ts:2287-2294).
- T-I2. Sturdy holder that also holds Focus Sash: Sturdy triggers, sash stays (priority -30 before -40).
- T-I3. Focus Band holder takes poison damage while at 60% HP: the `randomChance(1,10)` is drawn although nothing is at risk (items.ts:2272).
- T-I4. Sitrus Berry holder hit from 60% to 45% by the first hit of a 2-hit move: eaten after hit 1 (`Update` after each hit, `ch/scripts.ts:536`), hit 2 hits the healed Pokémon.
- T-I5. Sitrus under Heal Block: not eaten, stays held (`TryEatItem` fails).
- T-I6. Sitrus at exactly half HP (even maxhp): eaten (`hp <= maxhp/2`); at half+1: not.
- T-I7. Lum Berry holder hit by a Thunder Wave: cured at once through `AfterSetStatus`; confused by Confuse Ray: cured at the next Update (not at the same instant).
- T-I8. Pecha Berry holder badly poisoned by Toxic: cured at the Update after the move, no residual tick that turn.
- T-I9. Resist berry (Occa) vs a 2-hit Fire move: first hit halved and berry gone, second hit full.
- T-I10. Occa Berry vs Will-O-Wisp, vs a missed Fire move, vs a Fire move into Protect, vs a fixed-damage Fire move: berry stays in all four.
- T-I11. Occa Berry against a Foe with Unnerve: not eaten, full damage.
- T-I12. Chilan Berry vs a Normal move that is neutral: halved (no super-effective test).
- T-I13. Normal Gem user uses Swords Dance, then a Normal attack that misses, then one that hits: gem is eaten only on the hit and the hit is x5325/4096 on base power (conditions.ts:467).
- T-I14. Normal Gem user hits a Ghost: immune, gem is not consumed (before TryPrimaryHit).
- T-I15. Life Orb + Sheer Force holder using a move with a secondary: the x5324/4096 final modifier still applies but no recoil is taken (`battle-actions.ts:600`); Life Orb user whose move fails or misses: no recoil.
- T-I16. Life Orb user at 5 HP of 161 max: recoil `tr(161/10)` = 16 kills it after dealing damage; check double-faint winner rule.
- T-I17. Shell Bell on a 2-hit move: one heal `totalDamage/8`, truncated, min 1; Shell Bell with Drain Punch: both heals.
- T-I18. Leftovers holder poisoned, 160 max HP, at 15 HP: heals +10 first (residual order 5), then poison -20 (order 9): ends at 5 HP and survives (the old engine took poison first and fainted).
- T-I19. Rocky Helmet vs a 3-hit contact move: three procs of `tr(maxhp/6)`; vs a Substitute hit: none; vs an attacker with Magic Guard: none.
- T-I20. Air Balloon holder hit by Earth Power: immune, balloon stays; hit by a Water move: balloon pops, damage is dealt; hit through its Substitute: balloon pops (items.ts:203-211).
- T-I21. Air Balloon holder: Gravity up before it enters: no balloon announcement (items.ts:192) and it is grounded; Gravity set while it is out: `isGrounded` is true at once (`pokemon.ts:2170`) and Ground moves hit it.
- T-I22. Red Card holder hit by a move: attacker is forced out to a random bench Pokémon (draw even with one bench Pokémon); Red Card vs Suction Cups: item consumed, no switch (`DragOut` false).
- T-I23. Eject Button holder hit by U-turn: both sides switch (Champions override leaves the attacker's flag; check the request order).
- T-I24. Eject Button holder KOed: no switch (`target.hp` test).
- T-I25. White Herb holder hit by Intimidate on entry, and by Mega Intimidate (Scrafty, Manectric) later: herb fires at `AfterMega`; White Herb after its own Close Combat: fires after the move (`AfterMove`), before end of turn.
- T-I26. White Herb with a positive and a negative stat: only the negative one resets.
- T-I27. Mental Herb with Taunt and Encore at once: both removed, plus Attract/Torment/Disable/Heal Block.
- T-I28. Choice Scarf holder uses Trick, then switches: new holder is locked into its first move; the old holder's lock is removed (`isChoice` false).
- T-I29. Choice Scarf holder asleep tries to move, then wakes: no lock until a move is actually used.
- T-I30. Quick Claw holder and an opposing +1 priority move: the Claw never beats +1; with two Quick Claws, speed decides after both roll.
- T-I31. Quick Claw roll draw happens once per queued move action, also when the Pokémon then flinches or is asleep.
- T-I32. Iron Ball on a Flying/Levitate holder: grounded, takes Ground damage as neutral, Speed x0.5.
- T-I33. Wide Lens vs a 90% move: accuracy 90 -> `modify(90, 4505/4096)` = `tr((90*4505 + 2047)/4096)` = 99.
- T-I34. Zoom Lens when the foe has not moved yet: no boost; foe moved first: boost.
- T-I35. King's Rock with a Sheer Force holder and a move with a secondary: flinch roll still exists; with Serene Grace: 20%.
- T-I36. King's Rock on a move that already has a flinch secondary: no second roll.
- T-I37. Scope Lens on a stage-1 crit move (e.g. Slash): stage 2 gives 1/8; Scope Lens on stage 3 is clamped at stage 4 (always crits).
- T-I38. Terrain seed holder switches in under the matching terrain: used at entry (priority -1 after other SwitchIn handlers); terrain set later by the foe: used at `TerrainChange`; seed at +6 Def: still consumed.
- T-I39. Heat Rock Drought holder (Charizard-Mega-Y evolving): 8 turns of sun; Heat Rock under Magic Room: 5.
- T-I40. Light Clay holder uses Reflect, then is Knocked Off: the screen keeps its 8 turns (duration fixed at set time).
- T-I41. Terrain Extender holder with Electric Surge (ability) switching in: 8 turns.
- T-I42. Binding Band holder uses Wrap: trap damage 1/6 (`boundDivisor` 6) and duration `random(5,7)` (draw).
- T-I43. Big Root holder drains with Drain Punch: heal = `tr(round(dmg/2) * 5324/4096)`; Big Root with Leech Seed and Strength Sap.
- T-I44. Shed Shell holder vs Shadow Tag, partial trap and a Fairy-lock-style trap: can switch (priority -10 after the trappers).
- T-I45. Muscle Band and a Technician move: ordering of 4505/4096 (priority 16) and Technician's x1.5 (base power chain).
- T-I46. Metronome (item): same move 6 times in a row: x1.0, 1.2, 1.4, 1.6, 1.8, 2.0; a failed move resets the count; Trick-ing the item away removes the volatile.
- T-I47. Light Ball Pikachu: Atk and SpA doubled; Foul Play not doubled; confusion self-hit doubled.
- T-I48. Eating through Bug Bite: target's Sitrus heals the attacker `tr(attackerMaxHp/4)` even at full HP (no heal) and sets `ateBerry` (then Belch works).
- T-I49. Pluck on a Sticky Hold target: damage only, berry stays.
- T-I50. Knock Off vs Sticky Hold: x1.5 power applies, item stays; vs a stone of its own species: x1 and stays; vs a Mega-evolved holder: x1.
- T-I51. Knock Off, then Recycle: nothing to recycle (no `lastItem`); eat a berry, then Recycle: berry returns.
- T-I52. Trick: user has Choice Scarf, target has a Mega stone of its species: fails; target has Leftovers: swap, scarf lock on the target at its next move.
- T-I53. Trick user holding its own stone: fails (I12). Switcheroo vs Sticky Hold: fails before swapping.
- T-I54. Thief and Covet with the user holding Normal Gem consumed this move (`gem` volatile): no steal.
- T-I55. Fling Poison Barb / Light Ball / King's Rock / White Herb / Sitrus / Mega stone / Normal Gem: poison, paralysis, flinch roll, cure, heal, fails, fails; Fling into Protect: item gone.
- T-I56. Acrobatics with a consumed berry: 110 power; with a Mega stone: 55.
- T-I57. Belch before eating anything: fails and costs PP (Champions leaves it selectable); after Bug Bite on a berry: works.
- T-I58. Stuff Cheeks at +6 Defense: fails, berry kept; with a Sitrus at full HP: berry eaten, no heal.
- T-I59. Teatime with berries on both sides: both eat (force), Unnerve irrelevant.
- T-I60. Poltergeist vs a target with Klutz and an item: works; vs an empty-handed target: fails.
- T-I61. Corrosive Gas on a Mega stone holder of its species: fails.
- T-I62. Magic Room: Leftovers stops healing, Choice Scarf stops locking and boosting, berries stay uneaten, Light Clay already up keeps its duration.
- T-I63. Klutz holder with Leftovers: no heal; Knock Off on it still removes the item.
- T-I64. Unburden holder using Fling / eating a berry: Speed doubles while empty.
- T-I65. Harvest after eating Sitrus: restored with probability 1/2 (draw) or always in sun; Cud Chew replays the eat the turn after.
- T-I66. Item Clause: no two identical items on one team; a stone on a non-matching species is never generated (the team generator must not produce one).

Mega Evolution
- T-M1. Garchomp Mega-Z (Timid, 32 Spe) vs a 200-Speed foe: before the Mega the foe moves first; with `Mega` flag Garchomp moves first on the same turn (post-Mega Speed 223 vs 169).
- T-M2. Both sides Mega Evolve with equal Speed: the order of the two evolutions and the two moves each comes from a shuffle draw; Trick Room reverses.
- T-M3. Banette Mega Evolves and uses Thunder Wave/Will-O-Wisp: Prankster priority +1 applies the same turn, before a priority-0 foe move.
- T-M4. Charizard-Mega-Y evolves: Drought starts at once, before any move; opposing Pokémon with Swift Swim/Chlorophyll see it on the same turn.
- T-M5. Tyranitar-Mega with Smooth Rock: sand lasts 8 turns.
- T-M6. Scrafty-Mega / Manectric-Mega Intimidate: foe's Attack -1 before the foe moves; foe with White Herb restores at `AfterMega`; foe with Defiant gets +2.
- T-M7. Alakazam-Mega / Meowstic-Mega Trace: copies the foe's ability at the Mega action (`onStart`, plus `onUpdate`).
- T-M8. Pokémon with Truant Mega Evolving (no Truant Mega exists): `moveThisTurnResult = true` counts as the action (engine check only).
- T-M9. A Mega-eligible Pokémon locked into Outrage: no Mega choice offered (N12); after the lock ends it is offered again.
- T-M10. Mega user with all moves disabled or no PP (Struggle): no Mega choice.
- T-M11. After one Pokémon has Mega Evolved, the benched Pokémon with a stone: no Mega choice, even after switching in.
- T-M12. Mega user is Taunted/Encored/Disabled before the turn: Mega still possible with a legal move; the Mega Evolution happens even if the move then fails (asleep, flinch, full para: the Mega action precedes every move).
- T-M13. Mega Pokémon switches out and back: still Mega, ability slot 0, stone held, `base_ability` the Mega ability.
- T-M14. Mega Pokémon faints and is revived by Revival Blessing: returns as the Mega form.
- T-M15. Knock Off, Thief, Trick, Covet, Corrosive Gas, Fling on a Mega-evolved holder: all fail or skip; Knock Off damage x1.
- T-M16. Mega-evolved Pokémon uses Acrobatics: 55 power (holds its stone); Poltergeist against it works.
- T-M17. Gold team: Garchomp Mega-Z takes an Earth Power from the foe after evolving: immune (Levitate); before evolving: Ground typing (neutral/weak per type chart); after evolving it is pure Dragon: Ice 2x, Dragon 2x, Fairy 2x.
- T-M18. Gold team: Golisopod Mega + First Impression turn one: works; Emergency Exit before Mega at 51% -> hit to 49%: switch-out request; after Mega the same hit: no switch.
- T-M19. Gold team: Golisopod holds 100% HP and gets hit by a Rocky Helmet holder: Tough Claws x5325/4096 on contact moves from the Mega action on.
- T-M20. Mega with an HP-changing item or recoil on the same turn: HP is unchanged by the evolution (all 82 forms share base HP).
- T-M21. Weight: Heavy Slam / Low Kick used on a Pokémon before and after Mega Evolution (Steelix 4000 -> 7400 hg, Chimecho 10 -> 80 hg).
- T-M22. Mega Evolution under Neutralizing Gas / Gastro Acid on the user: old ability End then new ability Start still run (`isFromFormeChange` true skips the suppress check).
- T-M23. The Mega action when the opponent switches in the same turn: Mega after the switch-in entry effects (hazards, abilities), before any move.
- T-M24. Mega + priority move: priority bracket first, then post-Mega speed within the bracket.

## 7. What was not read, and what is uncertain

Not read (or only skimmed), so the coder must not rely on this spec for them:
- `data/moves.ts` condition handlers of `healblock`, `partiallytrapped` residual damage, `substitute` ordering relative to item handlers at `TryPrimaryHit`, `curse`, `leechseed`, `ingrain`, `aquaring`, `strengthsap`; the exact `TryHeal` priority of Heal Block against Big Root.
- `sim/battle.ts` `commitChoices`/`Side.choose*` queue-building order (which side's `FractionalPriority` draws first), `switchIn` entry order for items (`runSwitch` item `onStart` with `-1`/`-2` priorities relative to abilities), `resolveFaints`, `checkWin`, how two simultaneous `switchFlag` requests are ordered (Eject Button + U-turn, IM5).
- The hazards (`stealthrock`, `spikes`, `toxicspikes`, `stickyweb`) and Levitate/Air Balloon/Iron Ball interaction with `Spikes` damage (`isGrounded` is cited; the hazard handlers are not).
- Abilities that read items: Magician (read), Pickpocket (read), Symbiosis, Klutz, Unnerve (read), Gluttony, Harvest, Cud Chew, Cheek Pouch, Ripen (read); the rest (Sticky Hold read). Others that matter at Mega time (Intimidate's per-target checks: Clear Body, Hyper Cutter, Inner Focus, Own Tempo, Oblivious, Scrappy, Guard Dog, Mirror Armor, Rattled, Defiant, Competitive) were not read.
- `data/conditions.ts` weather `onFieldResidual`/`onWeather` handlers, `field.ts` `setTerrain` beyond lines 140-150, `sim/battle.ts` Embargo handling (not legal).
- All the engine files other than `hooks.rs`, `events.rs` (partly), `state.rs` (the `Mon` struct), `battle.rs` (Mega and queue parts), `options.rs` (Mega offering), `residual.rs` (item holder lines), `data/generated.rs` (item and species tables).
- CG audit docs: read `items-01..03.md` and the notes of `second-read/items-b1,b2.md`, `species.md` (Mega section), `mechanics-turn.md` (Q10-Q12, L17-L18), `data-diff/09-data-diff.md` (Mega and item sections) and the whole register `gaps/INDEX.md`. Not read: the other `audit/` files (`mechanics-formulas.md`, `mechanics-conditions.md`, abilities and moves slices, `completeness.md`) beyond keyword searches, and `docs/investigation/coverage/03-coverage.md` beyond its method section.
- Not run: nothing was executed against Showdown. All stat examples (section 3.6) are hand-computed from the formula in `ch/scripts.ts:10-40`.

Uncertain, flagged in place: whether Normal Gem's `TryPrimaryHit` runs before the Substitute's own handler (1.3); whether a Substitute hit triggers Red Card or Eject Button (they run on `targetsCopy` regardless); whether Air Balloon's pop sets `lastItem` (the code sets `item = ''` directly, so it appears not); the exact `moveResult` truth value for Life Orb when the move hits only a Substitute (`useMove` `battle-actions.ts:580-606`); the Shed Shell versus No Retreat/Octolock ordering.

## 8. Ten places most likely to be implemented wrongly

1. **Post-Mega speed and priority re-sort** (3.2 item 3): easy to keep the queue-time order (that is what the old engine did, N13) or to forget that `ModifyPriority` is re-run (Mega Banette Prankster).
2. **Focus Band's draw on every Damage event** (IM3): a draw tied to lethality shifts the whole tape.
3. **Resist berries and Normal Gem consumed inside the damage step**, first hit only (I2, I3): the engine consumed them early and on Status moves.
4. **Sitrus/Lum/Pecha/Rawst at `Update`** with the right number of `Update` calls, and the speed-tie shuffle that each call draws (IM16).
5. **Trick/Switcheroo/Thief/Covet/Knock Off through one `takeItem`/`setItem` pipeline** with the stone clause, Sticky Hold rules (blocks Knock Off removal but not its x1.5) and the `singleEvent('TakeItem')` second check (IM2).
6. **Eject Button (Champions override) and Red Card** ordering, the missing `source.switchFlag = false`, and the unconditional `sample` draw (IM5, IM15).
7. **Weather rocks, Light Clay, Terrain Extender**: Icy Rock exists (IM1), and the setter-holds-item rule includes abilities and Mega abilities (IM14).
8. **Fling**: the item leaves at the next Update even if the move misses or is protected (IM10), the effect is on the target, and Mega stones/Normal Gem make it fail.
9. **King's Rock with Sheer Force and Serene Grace** (IM7), and a `random(100)` draw for each secondary of each hit.
10. **Leftovers' residual order** (5, sub-order 4) versus the item sub-order 8, and White Herb's five trigger points (including `AfterMega` and `onAnySwitchIn` priority -2).
