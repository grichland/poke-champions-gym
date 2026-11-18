# Audit moves-06 (CSV lines 262-313)

Slice: MOVES rows 262-313 of 03-coverage-moves.csv. Rows audited: 52 of 52. NOT AUDITED: none.

Build assumption: Cargo features `champions` + `bss` only (no genN feature). Neither implies a generation, so in `choices.rs` every `if cfg!(feature="genN")` falls to its `else` arm (latest-gen values) and `genx/` is compiled (lib.rs:14). Showdown references are `data/moves.ts` (SD) and `data/mods/champions/moves.ts` (CH). Engine paths are under `poke-engine-eval/poke-engine/src`.

## Verdict counts

| verdict | n | ids |
|---|---|---|
| MATCH | 10 | glare, stormthrow, grassyterrain, barbbarrage, sandstorm, helpinghand, bite, tidyup, followme, twinbeam |
| PLAIN | 11 | breakingswipe, brutalswing, acidarmor, pounce, megakick, scaryface, seedbomb, stringshot, overdrive, headlongrush, megahorn |
| DATA | 4 | growth, aerialace, slash, meteorassault |
| LOGIC | 10 | ragingbull, howl, focuspunch, hex, perishsong, noretreat, reversal, matchagotcha, risingvoltage, electroball |
| PARTIAL | 1 | spiritshackle |
| MISSING | 9 | focusenergy, doubleteam, minimize, stompingtantrum, lastresort, fellstinger, skillswap, belch, transform |
| CORE | 7 | triplearrows, dynamicpunch, whirlpool, skyattack, firespin, snaptrap, eruption |
| UNSURE | 0 | |

## Five most important findings

1. Evasion does not exist in the engine. `StatBoosts` has no evasion field (choices.rs:19394), `check_move_hit_or_miss` uses only the attacker's accuracy stage (generate_instructions.rs:1230), and Double Team and Minimize carry an all-zero boost. Both moves are legal in Flat Rules (Evasion Moves Clause is only in `Standard`, not `Flat Rules`; rulesets.ts:19,37). Both are MISSING. Minimize's 2x damage and never-miss interaction is also absent.
2. Condition-gated moves have no gate:
   - Stomping Tantrum never doubles.
   - Last Resort always works (SD requires every other move used).
   - Belch always works (SD requires a berry eaten; Champions only removes the disable, not the `onTry`).
   - Fell Stinger never gives +3 Atk on a KO.
   - Skill Swap and Transform do nothing.
   - Focus Energy sets a volatile that no code reads.
3. Prankster handling is wrong for self and field status moves (abilities.rs:1879-1887). It calls `remove_all_effects()` on any Status move when the defender is Dark. Showdown only blocks moves that target a foe (battle-actions.ts:738, `!targets[i].isAlly(pokemon)`). A Prankster user of Howl, Growth, Acid Armor, Focus Energy, Minimize, No Retreat and similar loses the effect against a Dark opponent. Field moves keep working only because they are applied by move id elsewhere.
4. Data errors:
   - Growth is Normal; Champions makes it Grass (CH:467).
   - Slash is 70 BP; Champions makes it 80.
   - Meteor Assault is 150 BP; Champions makes it 170.
   - Aerial Ace is stored as accuracy 100; SD has `true` (never misses).
   - `apply_champions_modifiers` (choices.rs:20704-20774) covers none of these three data changes.
5. Rising Voltage uses x1.5 in Electric Terrain with no grounded-target check (choice_effects.rs:348); SD doubles BP only if the target is grounded (SD 15138-15144). Spirit Shackle has damage and 90 BP but no trap secondary (SD 17625-17630), so PARTIAL. No Retreat traps even Ghost types (state.rs:1118-1128 returns true before the Ghost check), where SD makes Ghosts immune to `trapped`.

## Table

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| focusenergy | 1 | MISSING | moves.ts:5974-6003 volatile `focusenergy`; `onModifyCritRatio` +2 stages; fails to start if dragoncheer | choices.rs:6037 sets volatile FOCUSENERGY; enum only at state.rs:187; no reader. Crit chance uses only `increased_crit_ratio`/`guaranteed_crit` (generate_instructions.rs:2432-2465) | Focus Energy has no effect on crit chance (should be stage +2 = 1/2). Also Prankster bug (finding 3) | M | high |
| glare | 1 | MATCH | moves.ts:6682 acc 100, status par, protect/reflectable, target normal | choices.rs:6713-6730 else arm: acc 100, PARALYZE, protect+reflectable; Electric/Limber immunity generate_instructions.rs:846 | none found | | high |
| growth | 1 | DATA | CH:467 type Grass; SD 7863-7884 +1 Atk +1 SpA, sun (incl. Mega Sol via effectiveWeather, pokemon.ts:2214) makes +2/+2; Utility Umbrella removes sun | choices.rs:6951-6975 type NORMAL, +1/+1; choice_effects.rs:248-267 +2/+2 only if `weather_is_active(SUN)` | Type Normal vs Grass (STAB, Grass-type damage calc, Grassy/other type-keyed code). Also Mega Sol user should get +2/+2 with no sun; Utility Umbrella not considered | S (type); M (Mega Sol) | high |
| breakingswipe | 1 | PLAIN | moves.ts:1789 Dragon phys 60 BP acc 100, contact, 100% Atk -1 | choices.rs:1784-1809 same | none | | high |
| brutalswing | 1 | PLAIN | moves.ts:1859 Dark phys 60 BP, contact, target allAdjacent | choices.rs:1840-1853 same (singles identical) | none | | high |
| acidarmor | 1 | PLAIN | moves.ts:66 Poison status, +2 Def, accuracy true, self | choices.rs:135-156 +2 Def, target User, accuracy default 100 | accuracy true vs 100: see Note N1 (accuracy drop makes self moves miss) | S (N1) | high |
| ragingbull | 1 | LOGIC | moves.ts:14630 90 BP Normal contact; `onTryHit` removes Reflect, Light Screen, Aurora Veil before damage (before accuracy); type by Paldean Tauros form | choices.rs:13202 90 BP; choice_effects.rs:162-183 doubles BP if target has Reflect or Aurora Veil, type by form; choice_effects.rs:803-840 removes all three screens after damage (hit only) | Damage is right on a normal roll (x2 BP cancels the screen x0.5). On a crit the engine crit damage ignores screens (damage_calc.rs:657-670), so BP x2 gives 2x too much. Infiltrator attacker also gets 2x. Screens are not removed if the move misses (acc 100 so rare) | M | medium |
| triplearrows | 1 | CORE | moves.ts:19977 90 BP Fighting phys, crit +1, 50% Def -1, 30% flinch, no contact | choices.rs:18101-18132 same; high-crit list choices.rs:20491 | Crit +1 stage only modelled as a branch in some paths (K7). Nothing else wrong | | high |
| pounce | 1 | PLAIN | moves.ts:13624 Bug phys 50 BP contact 100% Spe -1 | choices.rs:12417-12442 same | none | | high |
| howl | 1 | LOGIC | CH:507 flags snatch, sound, bypasssub; SD 9012 +1 Atk, target allies (user in singles); Soundproof only blocks when target is not the source | choices.rs:8183-8205 +1 Atk, target User, sound true; abilities.rs:2356-2361 Soundproof defender calls `remove_all_effects` on any sound move | Howl is cancelled when the opponent has Soundproof, although Howl only targets the user. Also Prankster bug (finding 3) | S | medium |
| doubleteam | 1 | MISSING | moves.ts:3988 evasion +1, accuracy true, self | choices.rs:4002-4023 boost struct all zeros (no evasion field); evasion never used in hit chance (generate_instructions.rs:1230) | Move does nothing. Evasion boost cannot be represented. Legal in this format | L | high |
| stormthrow | 1 | MATCH | moves.ts:18126 60 BP Fighting contact, `willCrit` (CH:973 only isNonstandard) | choices.rs:16283-16297 else arm 60 BP contact; `guaranteed_crit` choices.rs:20500; handled in all damage paths generate_instructions.rs:2436,2459,2472 | none (Battle/Shell Armor handled) | | medium |
| dynamicpunch | 1 | CORE | moves.ts:4357 100 BP acc 50, punch+contact, 100% confusion | choices.rs:4494-4514 same | Confusion never expires (K3). Nothing else wrong | | high |
| focuspunch | 1 | LOGIC | moves.ts:6004-6041 priority -3, 150 BP; fails if hit by a damaging move this turn (not through Substitute); user immune to flinch while charging | choices.rs:6052; choice_effects.rs:528-536 removes all effects if opponent `damage_dealt` >0, Phys/Spec, not hit_substitute; flag set when move present state.rs:1533-1544 | Opponent `damage_dealt` is reset only when the opponent runs a move (generate_instructions.rs:2048-2059; Switch returns first). If the opponent switches this turn, last turn's damage persists and Focus Punch fails wrongly (also Avalanche). Flinch immunity during charge not modelled | M | medium |
| grassyterrain | 1 | MATCH | moves.ts:7676 5 turns (8 with Terrain Extender), fails if already active, +1.3x Grass, 1/16 heal, halves EQ | choice_effects.rs:1704-1717 sets terrain 5 turns, fails if already Grassy; effects damage_calc.rs:236, generate_instructions.rs:2907 | Terrain Extender (8 turns) not modelled (no TERRAINEXTENDER anywhere in src); item slice. Move itself matches | | medium |
| hex | 1 | LOGIC | moves.ts:8610 65 BP, x2 if target has status or Comatose | choices.rs:7537-7551 65 BP; choice_effects.rs:268-272 x2 if status != NONE | Comatose target does not double Hex. (Comatose exists in engine, generate_instructions.rs:790) | S | medium |
| minimize | 1 | MISSING | moves.ts:11919 evasion +2, volatile `minimize` (2x damage and never-miss for minimize-flag moves) | choices.rs:10653-10678 boost all zeros, volatile MINIMIZE set; MINIMIZE only in the enum (state.rs) with no reader | No evasion, no 2x damage, no accuracy bypass. Legal in this format | L | high |
| barbbarrage | 1 | MATCH | moves.ts:1038 60 BP, x2 if target psn/tox, 50% poison | choices.rs:1007-1024 60 BP 50% poison; choice_effects.rs:604-611 x2 on POISON/TOXIC | none | | high |
| megakick | 1 | PLAIN | moves.ts:11596 Normal phys 120 BP acc 75 contact | choices.rs:10347-10361 same | none | | high |
| stompingtantrum | 1 | MISSING | moves.ts:18045 75 BP, x2 if user's last move failed (`moveLastTurnResult === false`) | choices.rs:16197-16210 plain 75 BP; no handler | No doubling. Needs per-pokemon last-move-result state | L | high |
| sandstorm | 1 | MATCH | moves.ts:15695 weather Sand 5 turns, fails if active; CH:859 PP 5 | choice_effects.rs:1642-1657 sets SAND 5 turns, not if already or strong winds; choices.rs:14176 wind flag | Smooth Rock (8 turns) not modelled (item slice). PP not in `choices.rs` (see N2) | | medium |
| scaryface | 1 | PLAIN | moves.ts:15796 acc 100, Spe -2, reflectable | choices.rs:14324-14348 else arm acc 100, Spe -2 | none | | high |
| seedbomb | 1 | PLAIN | moves.ts:15955 80 BP Grass bullet | choices.rs:14463-14477 same | none | | high |
| whirlpool | 1 | CORE | moves.ts:20742 35 BP acc 85 Water special, partiallytrapped | choices.rs:18810-18828 same, PARTIALLYTRAPPED; 1/8 damage generate_instructions.rs:3516-3532; Ghost exempt state.rs:1129; removed on opponent switch | Trap never expires (K5); Binding Band 1/6 and Grip Claw not modelled | | high |
| aerialace | 1 | DATA | moves.ts:168 60 BP, accuracy true (never misses), contact, slicing | choices.rs:211-226 60 BP contact slicing, accuracy 100 | Accuracy 100 vs never-miss: a lowered accuracy stage on the user makes it miss (generate_instructions.rs:1230); see N1 | S | high |
| lastresort | 1 | MISSING | moves.ts:10068 140 BP; `onTry` fails unless user knows >=2 moves and has used all others | choices.rs:9269-9284 else arm 140 BP, no condition | Always usable. Needs per-move "used" tracking | L | high |
| perishsong | 1 | LOGIC | moves.ts:13230 all active get volatile (4 turns), faints at end of 3rd following turn; sound, bypasssub; Soundproof only blocks when target is not source | choice_effects.rs:1547-1578 gives PERISH4 to each active with hp>0 and ability != SOUNDPROOF; countdown generate_instructions.rs:3405-3487 (faint at the right time) | A Soundproof user does not get the counter on itself (SD: only foes' Soundproof blocks). Rest is correct | S | medium |
| stringshot | 1 | PLAIN | moves.ts:18190 Bug acc 95 Spe -2 | choices.rs:16348-16372 same | none | | high |
| overdrive | 1 | PLAIN | moves.ts:13095 80 BP Electric special, sound, bypasssub | choices.rs:11683-11696 same; sound bypasses Substitute generate_instructions.rs:1438 | none | | high |
| fellstinger | 1 | MISSING | moves.ts:5205 50 BP; `onAfterMoveSecondarySelf` +3 Atk if target fainted | choices.rs:5230-5245 else arm 50 BP; no FELLSTINGER in genx code | No +3 Atk on KO | M | high |
| skillswap | 1 | MISSING | moves.ts:16586 `skillSwap(source,target)` swaps abilities, fails on unswappable abilities | choices.rs:15003-15014 table row only, protect flag; no handler (a `ChangeAbility` instruction exists, instruction.rs:59) | Move does nothing | M | high |
| noretreat | 1 | LOGIC | moves.ts:12789 +1 all five stats, fails if already active; CH:692 condition adds `trapped`, which Ghost types are immune to (typechart.ts:204), Shed Shell frees | choices.rs:11414-11439 +1 x5 and volatile NORETREAT; choice_effects.rs:318-325 drops boost if already active (fail equivalent); state.rs:1118-1128 NORETREAT returns trapped=true before the Ghost/Shed Shell check | Ghost-type user is trapped in the engine but not in SD; Shed Shell does not free it. Also Prankster bug (finding 3) | S | medium |
| skyattack | 1 | CORE | moves.ts:16647 140 BP acc 90 Flying, charge turn, crit +1, 30% flinch | choices.rs:15084-15103 same, charge flag; choice_effects.rs:1765 volatile SKYATTACK; crit list choices.rs:20486 | Crit +1 only modelled as branch in some paths (K7). Nothing else wrong | | medium |
| slash | 1 | DATA | CH:907 80 BP; SD 16833 Normal contact slicing crit +1 | choices.rs:15171-15185 70 BP | 70 vs 80. Not in `apply_champions_modifiers`. Crit +1 and slicing correct (choices.rs:20487) | S | high |
| reversal | 1 | LOGIC | moves.ts:15074 ratio=floor(hp*48/maxhp): <2 200, <5 150, <10 100, <17 80, <33 40, else 20 | choice_effects.rs:125-141 thresholds on hp/maxhp: 0.688/0.354/0.208/0.104/0.042 | Cut-offs rounded (exact 33/48=0.6875, 17/48=0.3542, 10/48=0.2083, 5/48=0.1042, 2/48=0.0417). Wrong BP for HP fractions inside the gaps, e.g. exactly 0.6875 gives 40 instead of 20 | S | high |
| firespin | 1 | CORE | moves.ts:5462 35 BP acc 85 Fire special, partiallytrapped | choices.rs:5549-5568 same | K5 trap never expires. Nothing else wrong | | high |
| meteorassault | 1 | DATA | CH:630 170 BP; SD 11725 Fighting, recharge, protect, mirror (no contact) | choices.rs:10473-10487 150 BP, protect+recharge | 150 vs 170. Not in `apply_champions_modifiers`. Flags/recharge ok | S | high |
| matchagotcha | 1 | LOGIC | moves.ts:11029 80 BP acc 90, drain 1/2, 20% burn, defrost + `thawsTarget` | choices.rs:10241-10262 same; thaws user choice_effects.rs:33,979 | Frozen target is not thawed by the hit (engine has no target-thaw logic anywhere; same gap for all Fire moves). Drain truncates instead of SD rounding (generate_instructions.rs:1550) | M | medium |
| helpinghand | 1 | MATCH | moves.ts:8576 priority 5, target adjacentAlly; fails in singles (no ally) | choices.rs:7499-7515 priority 5, sets volatile HELPINGHAND which nothing reads | Behaviourally a no-op in both. Also Prankster bug (finding 3) cancels the volatile only | | medium |
| bite | 1 | MATCH | moves.ts:1351 60 BP Dark contact bite, 30% flinch | choices.rs:1286-1307 else arm: Dark, 30% flinch, bite+contact | none | | high |
| headlongrush | 1 | PLAIN | moves.ts:8211 120 BP Ground, punch, contact, self Def -1 SpD -1 | choices.rs:7262-7288 same | none | | high |
| belch | 1 | MISSING | moves.ts:1197 120 BP acc 90 Poison; `onTry` fails unless user ate a berry; CH:52 removes only `onDisableMove` | choices.rs:1158-1172 plain 120 BP acc 90 | Belch always works. No "ate berry" state | L | high |
| snaptrap | 1 | CORE | moves.ts:17067 35 BP Grass, partiallytrapped; CH:923 type Steel | choices.rs:15441-15458 35 BP + trap; choices.rs:20757 sets STEEL | K5 trap never expires. Type override present | | high |
| risingvoltage | 1 | LOGIC | moves.ts:15134 70 BP; x2 if Electric Terrain and target grounded | choices.rs:13590; choice_effects.rs:348-352 x1.5 if Electric Terrain, no groundedness check | x1.5 vs x2, and applies to a non-grounded target | S | high |
| megahorn | 1 | PLAIN | moves.ts:11583 120 BP acc 85 Bug contact | choices.rs:10330-10344 same | none | | high |
| tidyup | 1 | MATCH | moves.ts:19621 removes Substitutes (all), spikes/tspikes/rocks/web on all sides, +1 Atk +1 Spe | choices.rs:17808-17830 +1 Atk +1 Spe; choice_effects.rs:1161-1230 clears 4 hazards and both Substitutes | none | | medium |
| followme | 1 | MATCH | moves.ts:6042 priority 2; `onTry` fails in singles | choices.rs:6069-6085 priority 2, volatile FOLLOWME, never read (state.rs:188) | No-op in both (no redirect in singles). Failure not recorded (affects Stomping Tantrum only) | | medium |
| twinbeam | 1 | MATCH | moves.ts:20117 40 BP Psychic, multihit 2, acc 100 | choices.rs:18222-18235 40 BP; `DoubleHit` choices.rs:20590 | none (fixed 2 hits, not K2) | | high |
| electroball | 1 | LOGIC | moves.ts:4591 BP by floor(spe/target spe): 40/60/80/120/150 (cap 4); `getStat` includes all speed modifiers | choices.rs:4652; choice_effects.rs:537-552 same BP table but on `calculate_boosted_stat(Speed)` (boost stages only, state.rs:999) | Ignores Choice Scarf, Tailwind, paralysis, ability and weather speed modifiers (the turn-order code has them, generate_instructions.rs:2535-2600) | M | medium |
| spiritshackle | 1 | PARTIAL | CH:953 90 BP; SD 17616 Ghost phys, 100% secondary adds `trapped` (Ghost immune) | choices.rs:15891-15904 80 BP Ghost phys, no secondaries; BP 90 set at choices.rs:20759 | 90 BP applied; trap missing (secondary `VolatileStatus(TRAPPED)` 100% would reuse the existing TRAPPED handling, state.rs:1136, generate_instructions.rs:361) | S | high |
| transform | 1 | MISSING | moves.ts:19824 `transformInto(target)` | choices.rs:18002-18012 table row only; no handler | Move does nothing (no copied stats/moves/ability/types) | L | high |
| eruption | 1 | CORE | moves.ts:4891 150 BP x hp/maxhp, floored to integer, min 1 | choices.rs:4882-4895 150 BP; choice_effects.rs:620-624 BP x hp/maxhp as float | No per-step truncation of BP (same family as K1). Nothing else wrong | | medium |

## Notes

N1. "Accuracy true" is not representable. `Choice.accuracy` defaults to 100 and `check_move_hit_or_miss` multiplies by the attacker's accuracy stage with `.min(1.0)` (generate_instructions.rs:1229-1230) for every move, including self-targeted ones. A negative accuracy stage on the user would make self buffs (Acid Armor, Howl, Growth, No Retreat, Tidy Up) and never-miss moves (Aerial Ace) miss with probability 1-3/(3+n). Rare in practice (needs an accuracy-lowering move used on the user), but it affects every `accuracy: true` row. Fix is one rule: skip the roll when the move is `target == User` or has an explicit never-miss flag.

N2. PP does not live in `choices.rs` (`Choice` has no PP field). Champions PP changes (Sandstorm 5, others) must come from the data feeding `Pokemon.moves`. Not checked in this slice. Engine PP is only decremented below 10 (K6).

N3. Prankster (abilities.rs:1879-1887) blocks every Status move against a Dark defender, including self-targeted and field moves. Showdown blocks only foe-targeted moves. This hits rows: focusenergy, growth, acidarmor, howl, doubleteam, minimize, noretreat, helpinghand, followme. Field moves (Sandstorm, Grassy Terrain, Perish Song, Tidy Up) are unaffected only because their effects are applied by move id in `choice_special_effect`. Needs a target check.

N4. Frozen-target thaw is missing engine-wide (only the user thawing list, choice_effects.rs:23-34). SD thaws a frozen target hit by any damaging Fire move or a `thawsTarget` move. This belongs to the Fire moves and status slices.

N5. `damage_dealt` stays stale across turns when the opponent's last action was a switch (generate_instructions.rs:2048-2059). It also affects Avalanche, Counter-type moves and Focus Punch; reported under focuspunch only.

N6. Several tables in `choices.rs` are chosen at compile time by `cfg!(feature = "genN")` inside `moves.insert`. With only `champions`+`bss`, the `else` arms are used, and I confirmed the else value matched Showdown for glare, growth (apart from type), hex, stormthrow, bite, firespin, whirlpool, scaryface, lastresort, fellstinger. If the build ever adds a gen feature, these change silently.

N7. Mega Sol: SD's `effectiveWeather` makes Growth (and any Move effect) act as sun for a Mega Sol user (pokemon.ts:2214-2218). The engine honours Mega Sol for Morning Sun and Weather Ball only. The code comment at choice_effects.rs:249-253 claims Growth "should fail" with Mega Sol, which does not match this Showdown source.
