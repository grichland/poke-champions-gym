Second read, MOVES lines 262-313 (focusenergy..eruption), 52 rows: MISSING 8, MATCH 17, PLAIN 5, DATA 4, LOGIC 11, CORE 4, PARTIAL 3, UNSURE 0.

Key: SD = pokemon-showdown data/moves.ts (SDC = data/mods/champions/moves.ts); CH = engine src/choices.rs; CE = src/genx/choice_effects.rs; GI = src/genx/generate_instructions.rs; DC = src/genx/damage_calc.rs; ST = src/genx/state.rs. Engine built with features champions+bss only; no genN feature, so every `else` branch of the cfg!(gen..) chains in CH is the live one. apply_champions_modifiers (CH:20704) only touches Snap Trap type and Spirit Shackle BP from this slice. The engine Choice has no PP, no bypasssub flag, no critRatio field, no evasion in StatBoosts (CH:19394).

| id | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | confidence |
|---|---|---|---|---|---|
| focusenergy | MISSING | SD:5974 volatile focusenergy, onModifyCritRatio +2 (SD:5995) | CH:6037 sets FOCUSENERGY volatile; token only in ST:187 enum; crit rate in GI:2432-2465 never reads it | crit stage +2 never applied | high |
| glare | MATCH | SD:6682 acc 100, par, protect/reflectable/mirror | CH:6714 (else branch) acc 100 default, PARALYZE on opponent, protect+reflectable | none found | medium |
| growth | DATA | SD:7863 Normal, +1 atk/spa, +2/+2 in sun; SDC:467 type Grass | CH:6952 type NORMAL, +1/+1; CE:248 +2/+2 if SUN weather | type Normal vs Grass (Champions override not in apply_champions_modifiers); Mega Sol user (sunny via effectiveWeather) not handled in CE:254 | high |
| breakingswipe | MATCH | SD:1789 60 BP Dragon contact, 100% atk -1 on foe | CH:1783 same, secondary 100% atk -1 opponent (CH:1795) | none | high |
| brutalswing | PLAIN | SD:1859 60 BP Dark contact, no effect | CH:1839 60 BP Dark contact protect | none | high |
| acidarmor | MATCH | SD:66 self def +2 | CH:134 target User boost def +2 (CH:143) | none | high |
| ragingbull | LOGIC | SD:14630 90 BP; onTryHit removes reflect/lightscreen/auroraveil; type by Tauros form | CH:13201 90 BP; CE:162 doubles BP if reflect/aurora_veil up, sets type by form; CE:803 removes the 3 screens | BP x2 hack also doubles crit_damage, but screens do not halve crit damage (DC:657-670 halve only `damage`): crit branch 2x too high with screen up | medium |
| triplearrows | MATCH | SD:19977 90 BP Fighting, crit 2, 50% def -1, 30% flinch | CH:18100 90 BP, secondaries 50% def -1 and 30% flinch; CH:20491 in increased_crit_ratio | none | high |
| pounce | MATCH | SD:13624 50 BP Bug contact, 100% spe -1 | CH:12416 same, secondary 100% spe -1 (CH:12428) | none | high |
| howl | MATCH | SD:9012 self atk +1, sound; SDC:507 adds bypasssub | CH:8182 sound, boost atk +1 User | bypasssub flag absent but self-target so no effect | high |
| doubleteam | MISSING | SD:3988 self evasion +1 | CH:4001 boost struct all 0 (no evasion field, CH:19394); GI:1230 accuracy ignores target evasion | no evasion boost applied, evasion not used in hit check | high |
| stormthrow | MATCH | SD:18126 60 BP Fighting contact, willCrit; SDC:973 only nonstandard | CH:16282 (else) 60 BP contact; CH:20500 in guaranteed_crit; GI:2436 crit 1.0 | none | medium |
| dynamicpunch | CORE | SD:4357 acc 50, 100 BP, punch, 100% confusion | CH:4493 acc 50, 100 BP, punch, 100% CONFUSION secondary | confusion duration is K3 (never expires); otherwise equal | medium |
| focuspunch | MATCH | SD:6004 150 BP, prio -3, loses focus if hit by damaging move (SD:6029) | CH:6052 150 BP prio -3 punch; CE:528 remove_all_effects if foe dealt damage>0 not into substitute; ST:1539 enables damage_dealt tracking | none found; SD flinch immunity of volatile moot | medium |
| grassyterrain | LOGIC | SD:7676 5 turns, grounded +1.3x Grass, halves earthquake/bulldoze/magnitude on grounded target, 1/16 heal grounded | CE:1704 5 turns; DC:236 1.3x Grass, only EARTHQUAKE halved (DC:239); GI:2907 heal 1/16 grounded | Bulldoze (and Magnitude) not halved; no defender grounded/semi-invulnerable check | high |
| hex | LOGIC | SD:8610 65 BP, x2 if target status or Comatose | CH:7536 65 BP; CE:268 x2 if status != NONE | Comatose target not counted | low |
| minimize | MISSING | SD:11919 evasion +2; volatile: x2 damage from minimize-flag moves and they never miss | CH:10652 boost all 0, sets MINIMIZE volatile; token only ST:209 | no evasion boost; volatile never read | high |
| barbbarrage | MATCH | SD:1038 60 BP, x2 if target psn/tox, 50% psn | CH:1006 60 BP, 50% POISON; CE:604 x2 if POISON/TOXIC | none | high |
| megakick | PLAIN | SD:11596 acc 75, 120 BP | CH:10346 acc 75, 120 BP | none | high |
| stompingtantrum | MISSING | SD:18045 75 BP, x2 if user's previous move failed | CH:16196 75 BP only; no hit in genx | doubling condition absent (no last-move-result tracking) | high |
| sandstorm | PARTIAL | SD:15695 weather Sandstorm, wind; SDC:859 pp 5; conditions.ts:632 8 turns with Smooth Rock | CH:14175 wind; CE:1642 sets SAND 5 turns, fails if already sand | no SMOOTHROCK reference in genx (always 5 turns); PP not in engine table; Smooth Rock legality in Champions not checked | low |
| scaryface | MATCH | SD:15796 acc 100, spe -2 foe | CH:14323 (else) acc 100, boost opp speed -2, protect+reflectable | none | high |
| seedbomb | PLAIN | SD:15955 80 BP Grass bullet | CH:14463 80 BP bullet protect | none | high |
| whirlpool | LOGIC | SD:20742 acc 85, 35 BP, partiallytrapped; SD:3783 hits Dive target for x2; conditions.ts:222 5-6 turns, 1/8 | CH:18809 acc 85, 35 BP, PARTIALLYTRAPPED; GI:3516 1/8 per turn (GI:142); DC:303 returns 0 vs DIVE | no double damage vs Dive (engine 0 damage); trap never expires (K5); no Binding Band/Grip Claw | medium |
| aerialace | DATA | SD:168 accuracy true (never misses) | CH:211 accuracy default 100; GI:1230 min(acc*accuracy-stage) so attacker acc drop makes it miss | accuracy 100 vs always-hit | high |
| lastresort | MISSING | SD:10068 140 BP, onTry fails unless 2+ moves and all others used | CH:9270 140 BP contact only | fail condition absent, always usable | high |
| perishsong | MATCH | SD:13230 all actives get perishsong, duration 4, faint at end; sound, bypasssub | CH:11906 sound; CE:1547 PERISH4 on each active unless Soundproof/already; GI:3405-3490 counts down, faints from PERISH1 | none found | medium |
| stringshot | MATCH | SD:18190 acc 95, spe -2 foe | CH:16348 acc 95, boost opp spe -2 | none | high |
| overdrive | PLAIN | SD:13095 80 BP Electric sound, bypasssub | CH:11682 80 BP sound; GI:1438 sound skips substitute | none | high |
| fellstinger | PARTIAL | SD:5205 50 BP, +3 atk if it KOs (onAfterMoveSecondarySelf) | CH:5230 (else) 50 BP contact; no Choices::FELLSTINGER in genx | +3 atk on KO not implemented | high |
| skillswap | MISSING | SD:16586 swaps abilities (onHit skillSwap) | CH:15003 protect flag only; no genx reference | no ability swap | high |
| noretreat | LOGIC | SD:12789 all stats +1; fails if already has it; SDC:692 volatile adds 'trapped' (Ghost-immune, return not false) | CH:11413 all +1 + NORETREAT; CE:318 no boost if already; ST:1120-1127 trapped() true for NORETREAT before Ghost/Shed Shell test | engine traps Ghost-type and Shed Shell holder (items.ts:5657 shedshell clears trapped) | medium |
| skyattack | MATCH | SD:16647 acc 90, 140 BP, charge, crit 2, 30% flinch | CH:15083 acc 90, 140 BP, charge, 30% flinch; CH:20486 high crit; CE:1765 charge volatile | none | medium |
| slash | DATA | SDC:907 BP 80 (base 70), crit 2, slicing | CH:15170 BP 70 slicing; CH:20487 high crit | BP 70 vs 80 | high |
| reversal | LOGIC | SD:15074 BP by floor(hp*48/maxhp): <2 200, <5 150, <10 100, <17 80, <33 40, else 20 | CH:13562 physical; CE:125 float hp ratio cuts 0.688/0.354/0.208/0.104/0.042 | cut-offs rounded vs exact 33/48=0.6875, 17/48, 10/48, 5/48, 2/48; exact-boundary HP gives wrong bracket (e.g. 110/160 gives 40 not 20) | medium |
| firespin | CORE | SD:5462 acc 85, 35 BP, partiallytrapped | CH:5548 acc 85, 35 BP Special Fire, PARTIALLYTRAPPED; GI:3516 1/8 | trap never expires (K5); otherwise equal | medium |
| meteorassault | DATA | SDC:630 BP 170 (base 150), recharge | CH:10473 BP 150 recharge | BP 150 vs 170 | high |
| matchagotcha | LOGIC | SD:11029 acc 90, 80 BP, drain 1/2 (Math.round, battle.ts:2188), 20% brn, thaws user and target | CH:10241 acc 90, 80 BP, 20% BURN, drain 0.5; CE:33 thaws user; GI:1550 drain truncated | drain truncates, SD rounds half up; no target thaw code found (low) | medium |
| helpinghand | LOGIC | SD:8576 prio 5; onTryHit fails with no ally (singles) | CH:7499 prio 5, sets HELPINGHAND on user; token never read (ST:196) | engine does nothing but does not report failure | low |
| bite | MATCH | SD:1351 60 BP Dark bite contact, 30% flinch | CH:1285 (else) Dark 60 BP bite, 30% FLINCH | none | high |
| headlongrush | MATCH | SD:8211 120 BP Ground punch, self def -1 spd -1 | CH:7262 same, boost User def -1 spd -1 | none | high |
| belch | MISSING | SD:1197 120 BP, fails unless user ate a berry (onTry) | CH:1158 acc 90, 120 BP, no condition | berry condition absent | high |
| snaptrap | CORE | SDC:923 Steel; SD:17067 35 BP contact, partiallytrapped | CH:15440 35 BP; CH:20757 type STEEL; PARTIALLYTRAPPED | trap never expires (K5); otherwise equal | medium |
| risingvoltage | LOGIC | SD:15134 70 BP, x2 if electric terrain and target grounded | CH:13589 70 BP; CE:348 x1.5 if electric terrain | 1.5x vs 2x; target-grounded not checked | high |
| megahorn | PLAIN | SD:11583 acc 85, 120 BP Bug contact | CH:10330 same | none | high |
| tidyup | MATCH | SD:19621 +1 atk spe; removes subs and spikes/tspikes/rocks/web both sides | CH:17808 boost atk/spe +1; CE:1161 clears 4 hazards both sides, substitutes both sides | boost always applied (SD fails only if nothing happened); irrelevant otherwise | medium |
| followme | LOGIC | SD:6042 prio 2; onTry fails unless 2+ actives per side (singles) | CH:6069 prio 2, sets FOLLOWME on user; token never read (ST:188) | engine move silently does nothing instead of failing (affects fail-flag users like Stomping Tantrum) | low |
| twinbeam | MATCH | SD:20117 40 BP Psychic, multihit 2 | CH:18222 40 BP; CH:20590 DoubleHit | none | medium |
| electroball | LOGIC | SD:4591 BP by floor(user spe/target spe) via getStat (incl. scarf, paralysis, tailwind) | CH:4652; CE:537 uses calculate_boosted_stat (ST:999,1055: stage only) | speed items/paralysis/Tailwind not in ratio | medium |
| spiritshackle | PARTIAL | SD:17616 80 BP (SDC:953 90), 100% traps target | CH:15891 BP 80, CH:20759 BP 90; no secondaries/volatile | trapping effect missing (damage and BP fine) | high |
| transform | MISSING | SD:19824 onHit transformInto | CH:18002 no effect | no transform | high |
| eruption | CORE | SD:4891 150 BP x hp/maxhp (floored via clampIntRange) | CH:4882 150 BP; CE:620 BP *= hp ratio (float, no floor) | BP not truncated: part of K1 integer-step truncation | medium |

Notes
- Row count: 52 (first focusenergy line 262, last eruption line 313).
- Engine accuracy (GI:1230) ignores target evasion entirely; affects Double Team, Minimize and any accuracy test.
- No Smooth Rock / Terrain Extender / Grip Claw / Binding Band handling found in genx; legality in Champions not verified.
- Damage drain uses truncation (GI:1550) while SD rounds (battle.ts:2188); applies to every drain move.
