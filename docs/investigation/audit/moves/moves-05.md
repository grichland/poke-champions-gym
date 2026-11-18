# Moves audit, slice moves-05 (CSV lines 210-261)

Slice: `03-coverage-moves.csv` lines 210 to 261, 52 rows (Bitter Malice ... Snipe Shot). All 52 audited. NOT AUDITED: none.
Engine features counted: `champions` + `bss` (champions implies the gen9 branches in `choices.rs`; `apply_champions_modifiers` at choices.rs:20704).
Reference: `data/moves.ts` plus `data/mods/champions/moves.ts` (overrides in this slice: beakblast BP 120 / PP 5, snipeshot BP 85, burnup isNonstandard null, ragefist comment only), `data/mods/champions/conditions.ts` (sleep 2-3 turns, par 1/8, frz thaw 1/4), format `[Gen 9 Champions] BSS Reg M-C` = `Flat Rules` + `VGC Timer` (no Sleep Clause, no OHKO Clause).

## Summary

| Verdict | Count |
|---|---|
| MATCH | 21 |
| PLAIN | 0 |
| DATA | 1 |
| LOGIC | 14 |
| PARTIAL | 5 |
| MISSING | 6 |
| CORE | 5 |
| UNSURE | 0 |
| total | 52 |

Five most important findings:
1. Fissure never works: table row has `base_power 0` and no OHKO code, so `calculate_damage` returns `Some((0,0))` and `check_move_hit_or_miss` sets hit chance to 0 (generate_instructions.rs:1231). Same would apply to Sheer Cold / Guillotine / Horn Drill.
2. The engine enforces Sleep Clause (`has_alive_non_rested_sleeping_pkmn`, generate_instructions.rs:834-836) but Reg M-C has none. Hypnosis and Sleep Powder (and every sleep source) are wrongly blocked while another opposing mon is asleep.
3. Rage Fist, Throat Chop, Burning Jealousy, Gravity and Ingrain have no behaviour at all (only a table row or an unused enum token). Smack Down applies a `SMACKDOWN` volatile that `is_grounded()` never reads.
4. Armor Cannon lacks its self drop (Def -1, SpD -1). Shell Side Arm is always Special (Showdown picks the higher of physical/special damage).
5. Recoil and drain are truncated by the engine but `Math.round`ed by Showdown (Head Smash, Wild Charge, Horn Leech; also recoil min 1). Rapid Spin / Mortal Spin / Defog hazard clearing is keyed on move id, so it also fires when the move is blocked by Protect, and Defog removes the user's own screens.

## Table

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| bittermalice | 1 | MATCH | moves.ts:1381 Special Ghost 75 BP, acc 100, 100% Atk -1 on target | choices.rs:1327 same, secondary Boost atk -1 100% Opponent | | | high |
| memento | 1 | LOGIC | moves.ts:11622 acc 100, target Atk -2 SpA -2, `selfdestruct: ifHit` (user faints only if move hits; a Substitute makes a status move fail) | choices.rs:10380 boost opp -2/-2, `heal` user -1.0 (faint); choice_effects/run_move applies heal after hit check | Engine heal runs even when opponent's Substitute blocked the stat drops (boost blocked via `immune_to_stats_lowered_by_opponent`, heal not gated), so user faints; Showdown fails the move with no faint | M | medium |
| thrash | 1 | CORE | moves.ts:19369 120 BP Normal contact, self `lockedmove` (2-3 turns, then confusion) | choices.rs:17455 (non-gen1-4 branch) 120 BP, LOCKEDMOVE; generate_instructions.rs:3290-3345 fixed 3 turns then CONFUSION | Only K4 (fixed lock length) and K3 (confusion never expires); table row correct | | high |
| ragefist | 1 | MISSING | moves.ts:14580 BP callback `min(350, 50 + 50*timesAttacked)`; champions mod: counter reset in clearVolatile; punch flag | choices.rs:13168 flat 50 BP, no counter anywhere (only table row + enum) | No hits-taken counter, power stays 50; needs per-mon counter (+hits taken, reset on switch-out in Champions) | M | high |
| zenheadbutt | 1 | MATCH | moves.ts:21182 80 BP acc 90 contact, 20% flinch | choices.rs:19271 same, 20% FLINCH secondary; flinch honoured in `cannot_use_move` (generate_instructions.rs:1640) | | | high |
| coil | 1 | MATCH | moves.ts:2606 Atk +1 Def +1 Acc +1, self | choices.rs:2662 same boosts, target User | | | high |
| sleeppowder | 1 | LOGIC | moves.ts:16847 acc 75, status slp, powder; Champions slp 2 or 3 turns (conditions.ts:11-29); no Sleep Clause in Reg M-C | choices.rs:15187 acc 75, SLEEP, powder; wake 0/1/2 via generate_instructions.rs:171-179 (1/3 then 100%, uses 0.333); powder immunity Grass (:1594) and Overcoat (abilities.rs:2889) | Sleep Clause applied (`has_alive_non_rested_sleeping_pkmn`, :834-836) but the format has none; Safety Goggles powder immunity absent (no SAFETYGOGGLES in genx); wake chance 0.333 not 1/3; Mold Breaker not considered for Insomnia/Sweet Veil | S | high |
| throatchop | 1 | MISSING | moves.ts:19385 80 BP; secondary 100% adds `throatchop` volatile (2 turns): sound moves disabled / fail | choices.rs:17475 80 BP, applies PokemonVolatileStatus::THROATCHOP to Opponent | THROATCHOP appears only in the enum (genx/state.rs:260); nothing reads it, no duration, sound moves still usable | M | high |
| headsmash | 1 | LOGIC | moves.ts:8229 150 BP acc 80, recoil [1,2] = `max(1, round(dmg/2))` of total damage (battle-actions.ts:1435-1441) | choices.rs:7289 150/80 recoil 0.5; generate_instructions.rs:1566-1567 `(dmg*0.5) as i16` truncates | Recoil truncated not rounded (odd damage: 1 HP less), no minimum 1 | S | high |
| pinmissile | 1 | CORE | moves.ts:13369 25 BP acc 95, multihit 2-5 (35/35/15/15) | choices.rs:12059 same data; multi_hit TwoToFiveHits (:20582) | K2 fixed 3 hits (5 Skill Link, 4 Loaded Dice); data correct | | high |
| spiritbreak | 1 | MATCH | moves.ts:17598 75 BP contact, 100% SpA -1 | choices.rs:15865 same | | | high |
| boomburst | 1 | MATCH | moves.ts:1690 140 BP sound, bypasssub, allAdjacent | choices.rs:1699 140 BP sound; sound bypasses Substitute (generate_instructions.rs:1438) | | | high |
| iciclespear | 1 | CORE | moves.ts:9456 25 BP acc 100, multihit 2-5 | choices.rs:8678 same; TwoToFiveHits | K2 only | | high |
| tailwind | 1 | MATCH | moves.ts:18871 side condition 4 turns (incl. use turn), speed x2 | choices.rs:17077 side_condition Tailwind; TAILWIND_DURATION 4 (generate_instructions.rs:151, 653), decrement at end of turn (:2989), speed x2 (:2585) | | | high |
| gigatonhammer | 1 | MATCH | moves.ts:6592 160 BP acc 100, `cantusetwice` | choices.rs:6583 160 BP; move disabled when last used (genx/state.rs:507-512) | | | medium |
| cottonguard | 1 | MATCH | moves.ts:2953 Def +3 self | choices.rs:2951 Def +3 | | | high |
| waterpulse | 1 | CORE | moves.ts:20585 60 BP pulse, 20% confusion (2-5 turns) | choices.rs:18668 60 BP pulse, 20% CONFUSION | K3 (confusion never expires); data correct | | high |
| ancientpower | 1 | MATCH | moves.ts:396 60 BP, 10% self +1 all five stats | choices.rs:430 same, secondary target User | | | high |
| smackdown | 1 | PARTIAL | moves.ts:16950 50 BP; `smackdown` volatile: grounds Flying/Levitate target (Ground hits Flying, hazards, terrain), cancels Fly/Bounce/Magnet Rise/Telekinesis | choices.rs:15297 50 BP, applies SMACKDOWN to Opponent | Volatile is set but `is_grounded()` (genx/state.rs:720) never reads it: Ground moves still hit-immune vs Flying, terrains/hazards still skip; only abilities.rs:2239 mentions it (Sheer Force exclusion) | M | high |
| discharge | 1 | MATCH | moves.ts:3733 80 BP, 30% par | choices.rs:3744 same; Electric par immunity (generate_instructions.rs:846-849) | | | high |
| wildcharge | 1 | LOGIC | moves.ts:20869 90 BP contact, recoil [1,4] rounded min 1 | choices.rs:18939 90 BP recoil 0.25 | Recoil truncation vs `Math.round`, no min 1 (same as Head Smash) | S | high |
| fissure | 1 | MISSING | moves.ts:5516 acc 30, `ohko: true` (no OHKO Clause in format), nonsky | choices.rs:5601 acc 30, Physical, base_power 0, no OHKO code (no FISSURE in genx) | `calculate_damage` returns (0,0) for BP 0 and `check_move_hit_or_miss` forces hit chance 0 (generate_instructions.rs:1231): move never hits. Needs OHKO damage = target HP, 30% (level rule), Sturdy/Sash handling | M | high |
| dualwingbeat | 1 | MATCH | moves.ts:4331 40 BP acc 90 contact, multihit 2 | choices.rs:4463 same; DoubleHit = 2 hits (exact) | | | high |
| shellsidearm | 1 | LOGIC | moves.ts:16210 90 BP Special Poison; onModifyMove picks Physical (and contact) when physical damage calc > special (coin flip if equal); 20% psn | choices.rs:14744 90 BP always Special, no contact, 20% poison | Category never switches to Physical; no contact flag when physical | M | high |
| shedtail | 1 | LOGIC | moves.ts:16157 fails if cannot switch, sub exists, or hp <= ceil(maxhp/2); cost ceil(maxhp/2); sub HP floor(maxhp/4); switch passes sub, not boosts | choices.rs:14716 empty row; choice_effects.rs:1501-1545 cost `maxhp/2` (floor), needs `hp > maxhp/2`; pivot only if >1 alive (generate_instructions.rs:3686) | Odd maxhp: cost 1 HP too low and hp threshold off by 1; with no replacement the engine still pays HP and makes the Sub, Showdown fails the move | S | medium |
| metalburst | 1 | LOGIC | moves.ts:11657 returns 1.5x last damage taken this turn (floor), fails if not damaged this turn, normal hit rules | choices.rs:10406 empty row; choice_effects.rs:1365-1383 `(damage*3)/2` min HP, only if user moves second and last hit did not hit Substitute | Damage is subtracted as raw HP: bypasses defender's own Substitute, Sturdy/Sash, Disguise-type protection; otherwise equal | M | medium |
| burnup | 1 | MATCH | moves.ts:2092 130 BP Fire, fails if user not Fire, removes Fire type after use, defrost; champions: legal | choices.rs:2171 130 BP; choice_effects.rs:116 fails if not Fire; :765 removes Fire type (TYPELESS slot) | Minor: CHOICE_THAWS_USER lists Burn Up for any user (Showdown does not thaw a non-Fire user) | | medium |
| lowsweep | 1 | MATCH | moves.ts:10484 65 BP contact, 100% Spe -1 | choices.rs:9786 (gen6+ branch) 65 BP, Spe -1 100% | | | high |
| rapidspin | 1 | PARTIAL | moves.ts:14691 50 BP contact; after hit (and after Sub damage) removes Leech Seed, hazards (spikes, tspikes, rocks, web) on user side, ends partial trap; 100% self Spe +1 | choices.rs:13247 50 BP, self Spe +1 100%; choice_effects.rs:1240-1289 clears rocks/spikes/tspikes/web only | Leech Seed and partial-trap removal missing; clear is keyed on move id so it also runs when the move is Protected (Protect only strips choice fields, generate_instructions.rs:1744) | M | medium |
| trailblaze | 1 | MATCH | moves.ts:19803 50 BP contact, 100% self Spe +1 | choices.rs:17975 same | | | high |
| psychicterrain | 1 | MATCH | moves.ts:14092 5 turns (8 with Terrain Extender), blocks priority>0.1 moves vs grounded targets, Psychic x1.3 for grounded user | choices.rs:12834 empty; choice_effects.rs:1718 5 turns; block in generate_instructions.rs:1615-1620; boost 1.3 damage_calc.rs:222-255 | Terrain Extender 8 turns not modelled (no TERRAINEXTENDER in genx; item slice) | | medium |
| amnesia | 1 | MATCH | moves.ts:359 SpD +2 self | choices.rs:391 (non-gen1 branch) SpD +2 | | | high |
| armorcannon | 1 | PARTIAL | moves.ts:524 120 BP Special Fire, self Def -1 SpD -1 | choices.rs:572 120 BP Fire, no boost field | Self stat drops missing | S | high |
| darkestlariat | 1 | MATCH | moves.ts:3323 85 BP contact, ignores target Def boosts and evasion | choices.rs:3333 85 BP; damage_calc.rs:394-399 ignores defender boosts | Evasion is not modelled anywhere in hit chance (engine-wide), so no effect here | | high |
| populationbomb | 1 | CORE | moves.ts:13610 20 BP acc 90, multihit 10 with multiaccuracy (Skill Link ignores checks, Loaded Dice 4-10) | choices.rs:12399 20/90 slicing contact; generate_instructions.rs:2399-2406 fixed 6 hits (9 Wide Lens) | K2/K10 (hit count approximated, per-hit accuracy not branched); Skill Link / Loaded Dice ignored for this move | | high |
| selfdestruct | 1 | LOGIC | moves.ts:16002 200 BP, `selfdestruct: always` (faints even on miss/protect/immune), Damp on any mon blocks | choices.rs:14535 200 BP; choice_effects.rs:1007-1018 user HP to 0 before hit check unless defender has Damp | Faint timing correct; Damp only checked on the defender (abilities.rs:2859), a Damp user (own mon) is not checked | S | medium |
| aquastep | 1 | DATA | moves.ts:490 80 BP contact, `dance` flag, 100% self Spe +1 | choices.rs:529 same numbers; Flags struct has no `dance` | `dance` flag not representable (Flags in choices.rs:19436 has none; Dancer ability only an enum token); numbers match | L | medium |
| beakblast | 1 | PARTIAL | moves.ts:1119 100 BP (Champions 120, PP 5), priority -3, bullet; while charging (volatile `beakblast`, 1 turn) a contact hit burns the attacker | choices.rs:1077 priority -3 bullet; apply_champions_modifiers choices.rs:20708 sets 120 | No `beakblast` volatile: attackers making contact are not burned | M | high |
| explosion | 1 | LOGIC | moves.ts:4969 250 BP, `selfdestruct: always`, Damp blocks | choices.rs:4968 250 BP; same special-effect faint as Self-Destruct | Same Damp-on-user gap as Self-Destruct | S | medium |
| hornleech | 1 | LOGIC | moves.ts:8998 75 BP contact heal, drain [1,2] = `round(dmg/2)` (battle.ts:2188) | choices.rs:8165 75 BP drain 0.5; generate_instructions.rs:1549-1551 truncates | Drain truncated, Showdown rounds half up (odd damage heals 1 less) | S | high |
| ingrain | 1 | MISSING | moves.ts:9611 volatile: heal 1/16 max HP each residual, trapped, grounded, cannot be dragged | choices.rs:8831 applies INGRAIN to user | INGRAIN only in enum (genx/state.rs:199); no heal, no trap, no grounding, no drag block | M | high |
| acidspray | 1 | MATCH | moves.ts:98 40 BP bullet, 100% SpD -2 | choices.rs:157 same | | | high |
| burningjealousy | 1 | MISSING | moves.ts:2071 70 BP Special Fire; 100% burn if target's stats were raised this turn | choices.rs:2157 70 BP Fire, no secondary | No "stats raised this turn" tracking and no burn | M | high |
| bulldoze | 1 | MATCH | moves.ts:1972 60 BP, 100% Spe -1 | choices.rs:2060 same | | | high |
| mistyexplosion | 1 | LOGIC | moves.ts:12134 100 BP Special Fairy, selfdestruct always, x1.5 BP in Misty Terrain only if user grounded | choices.rs:10821 100 BP; choice_effects.rs:284-287 x1.5 in Misty Terrain with no grounded check; faint at :1007 | Boost applies to an airborne user; plus Damp-on-user gap | S | high |
| gravity | 1 | MISSING | moves.ts:7757 pseudo-weather 5 turns: accuracy x1.667, grounds everyone, cancels Fly/Bounce/Magnet Rise/Telekinesis, disables gravity-flag moves | choices.rs:6887 empty row, no handler (no GRAVITY in genx) | Whole effect absent; `is_grounded()` ignores it | L | high |
| bellydrum | 1 | LOGIC | moves.ts:1216 fails if hp <= maxhp/2 or Atk already +6; pays floor(maxhp/2), Atk +12 (clamped, Contrary reverses) | choices.rs:1173 empty row; choice_effects.rs:1302-1321 pays HP when hp > maxhp/2 and sets Atk to +6 | Atk already +6: engine still pays half HP (Showdown fails, no HP loss); Contrary ignored | S | medium |
| defog | 1 | LOGIC | moves.ts:3447 target Evasion -1 (unless Sub); removes reflect, light screen, aurora veil, safeguard, mist and hazards from TARGET side; hazards only from user side; clears terrain; bypasssub, protect-blocked | choices.rs:3499 empty boost row; choice_effects.rs:1119-1160 clears terrain and rocks/spikes/tspikes/web/reflect/light screen/aurora veil on BOTH sides | Removes user's own screens (wrong); misses Safeguard and Mist on target; keyed on move id so also fires under Protect; evasion drop not modelled | M | medium |
| mortalspin | 1 | PARTIAL | moves.ts:12316 30 BP contact; after hit removes Leech Seed, user-side hazards, partial trap; 100% poison on foes | choices.rs:10920 30 BP, 100% POISON; hazard clear shared with Rapid Spin (choice_effects.rs:1240) | Leech Seed / partial-trap removal missing; clears even when Protected | M | medium |
| hypnosis | 1 | LOGIC | moves.ts:9223 acc 60, status slp (2-3 turns in Champions); no Sleep Clause | choices.rs:8428 acc 60, SLEEP; wake schedule matches (generate_instructions.rs:171-179) | Same Sleep Clause gap as Sleep Powder | S | high |
| raindance | 1 | MATCH | moves.ts:14676 rain 5 turns (Damp Rock 8) | choices.rs:13234; choice_effects.rs:1626 5 turns, blocked only by Delta Stream | Damp Rock not modelled (no DAMPROCK in genx; item slice); primal weathers banned in format | | medium |
| snipeshot | 1 | MATCH | moves.ts:17134 80 BP (Champions mod 85), critRatio 2 | choices.rs:15505 80 BP; apply_champions_modifiers :20758 sets 85; increased_crit_ratio includes SNIPESHOT (:20488) | | | high |

## Notes

- Sleep Clause: the format `[Gen 9 Champions] BSS Reg M-C` inherits only `Flat Rules` (rulesets.ts:37-43) and `VGC Timer`; neither includes Sleep Clause Mod or OHKO Clause. If the Champions game itself differs from Showdown, the owner should decide; the reference says no clause.
- Sleep timing matches Champions: engine `chance_to_wake_up` gives 0 / 0.333 / 1.0 on successive move attempts, equal to Showdown `startTime = sample([2,3,3])`. The constant 0.333 should be 1/3 (probabilities no longer sum exactly).
- Protect handling (`remove_effects_for_protect`, choices.rs:20614) only clears fields of the Choice. Anything keyed on `move_id` in `choice_hazard_clear` / `choice_special_effect` still runs (Rapid Spin, Mortal Spin, Defog, and for protect-flagged self-faint moves it is correct only for Explosion-type moves). Worth checking the other hazard-clearing moves (Tidy Up, Court Change).
- Engine-wide: evasion boosts are not used in hit chance (`boosted_accuracy` takes only accuracy boost), `dance` flag and Dancer do not exist, no `bypasssub` flag (only `sound` is special-cased), Safety Goggles absent, Terrain Extender / Damp Rock / Light Clay absent.
- Recoil/drain: Showdown (battle-actions.ts:1440, battle.ts:2188) uses `Math.round`; recoil min 1; recoil is applied once on total multi-hit damage. Engine truncates (generate_instructions.rs:1549, 1566). Affects every recoil/drain move, not just the three rows here.
- Sub-hit statuses: engine applies statuses/boosts through `immune_to_status` / `immune_to_stats_lowered_by_opponent`, but `heal`-based self-faint (Memento) is not gated by Substitute.
- `immune_to_status` ignores Mold Breaker-type ability bypass for Insomnia, Sweet Veil, Vital Spirit (another slice).
- PP values (Beak Blast PP 5, etc.) are not in the move table at all; PP comes from the team data, so PP overrides cannot be checked here.
