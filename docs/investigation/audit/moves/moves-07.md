# Audit moves-07 (CSV lines 314-365)

Slice: MOVES, 52 rows (lava plume .. air cutter). Rows audited: 52. NOT AUDITED: none.

Build read: Cargo `champions` + `bss` (bss implies champions), no gen feature, so runtime `cfg!(gen1..gen5)` is false and the "else" (gen6+) table branches are the live ones; `genx` module is compiled. `apply_champions_modifiers` (choices.rs:20704) applies.

Counts: MATCH 29, LOGIC 10, CORE 3, DATA 3, MISSING 7, PLAIN 0, PARTIAL 0, UNSURE 0.

Top findings:
1. Attract (volatile token only, no "50% cannot move" and no gender), Aqua Ring (no 1/16 end-of-turn heal), Stockpile, Imprison, Entrainment, Topsy-Turvy, Acupressure: no behaviour at all (MISSING).
2. Fickle Beam is wrong: engine gives a 50% self +1 SpA; Showdown is a 30% chance of doubling base power, no stat change (choices.rs:5258).
3. Charge: volatile is never consumed after the next Electric move, so every later Electric move stays doubled (genx/damage_calc.rs:273; no removal anywhere).
4. Crabhammer: engine BP 95 (champions override line 20712) and accuracy 90; Showdown BP 100, accuracy 95 (champions override).
5. Fire-type hit does not thaw a frozen target (Lava Plume, Heat Wave, Scorching Sands thawsTarget); Steel Roller never clears terrain; Sing does not bypass Substitute; Air Cutter lacks slicing and wind flags; Clangorous Soul threshold and no fail when stats maxed; Misty Terrain has no Terrain Extender (8 turns).

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| lavaplume | 1 | LOGIC | moves.ts:10108 Fire spec 80bp 100acc, 30% brn; conditions.ts:117 frz onDamagingHit: Fire damaging move cures target freeze | choices.rs:9301 80bp, 30% burn, protect; no thaw-on-hit code anywhere in genx (only user thaw list choice_effects.rs:23) | frozen target hit by Fire move stays frozen | M | high |
| lightofruin | 1 | MATCH | moves.ts:10301 90acc 140bp Fairy, recoil 1/2 of damage; champions moves.ts:576 only isNonstandard | choices.rs:9607 90acc 140bp Fairy, recoil 0.5 | | | high |
| heatwave | 1 | LOGIC | moves.ts:8519 Fire 95bp 90acc, 10% brn, wind; same Fire thaw rule conditions.ts:117 | choices.rs:7463 (gen6+ branch) 95bp 90acc wind, 10% burn; no thaw-on-hit | frozen target not thawed by Fire hit | M | high |
| mistyterrain | 1 | LOGIC | moves.ts:12153 terrain 5 turns, 8 with Terrain Extender; blocks status and confusion for grounded, halves Dragon | choice_effects.rs:1732 sets 5 turns always; damage_calc.rs:245 Dragon halve; generate_instructions.rs:797 blocks status for grounded; no TERRAINEXTENDER anywhere in src | no 8-turn duration with Terrain Extender (item may or may not be legal; Champions items.ts has no entry); confusion-block under Misty Terrain not found in engine | S | medium |
| scorchingsands | 1 | LOGIC | moves.ts:15813 Ground 70bp 100acc, 30% brn, defrost flag (user thaws) + thawsTarget; conditions.ts:112 cures target freeze | choices.rs:14349 same numbers; user thaw via CHOICE_THAWS_USER choice_effects.rs:32 | thawsTarget (and Fire-type hit thaw) not implemented, frozen target stays frozen | M | high |
| aurawheel | 1 | MATCH | moves.ts:775 Electric 110bp, 100% self spe+1; onTry fails if not Morpeko; Hangry becomes Dark | choices.rs:791 same; choice_effects.rs:142 Hangry -> Dark | non-Morpeko failure not coded (cannot occur with legal sets) | | medium |
| toxicthread | 1 | MATCH | moves.ts:19785 psn + spe -1; champions moves.ts:1069 spe -2 | choices.rs:17947 poison + boost opp; champions modifier choices.rs:20761 speed -2 | | | high |
| shelter | 1 | MATCH | moves.ts:16311 self def +2, Steel | choices.rs:14801 user def +2 | | | high |
| mountaingale | 1 | MATCH | moves.ts:12364 100bp 85acc Ice, 30% flinch; champions moves.ts:672 bp 120 | choices.rs:10941 100bp 85acc 30% flinch; champions modifier choices.rs:20752 bp 120 | | | high |
| tropkick | 1 | MATCH | moves.ts:20053 Grass 70bp, 100% atk -1; champions moves.ts:1088 bp 85 | choices.rs:18181 70bp, 100% atk -1; modifier choices.rs:20773 bp 85 | | | high |
| chargebeam | 1 | MATCH | moves.ts:2318 90acc 50bp, 70% self spa+1 | choices.rs:2323 same | | | high |
| supercellslam | 1 | MATCH | moves.ts:18441 95acc 100bp Electric, contact, crash 1/2 max HP on failure | choices.rs:16579 same, crash 0.5; applied on miss (generate_instructions.rs:1238) and on zero damage (1412) | not checked: Magic Guard / Protect-block crash edge cases | | medium |
| lowkick | 1 | MATCH | moves.ts:10444 weight BP 20/40/60/80/100/120 at 10/25/50/100/200 kg | choices.rs:9737 bp 0; choice_effects.rs:641 same thresholds, heavy/light metal handled (37-47) | Float Stone halving not modelled (no FLOATSTONE in genx); legality in Champions unknown | | medium |
| hammerarm | 1 | MATCH | moves.ts:8088 100bp 90acc, punch, self spe -1 | choices.rs:7138 same, punch flag | | | high |
| steelroller | 1 | LOGIC | moves.ts:17889 130bp; onTry fails if no terrain; onHit/onAfterSubDamage clears terrain | choices.rs:16101 130bp; choice_effects.rs:361 bp 0 if no terrain; terrain never cleared (only ICESPINNER clears, choice_effects.rs:889) | terrain not removed after use; the bp0 "fail" path counts as a miss (triggers miss effects) | S | high |
| futuresight | 1 | CORE | moves.ts:6394 120bp Psychic, slot condition hits at end of 2nd turn after use, uses user's stats | choices.rs:6444 120bp; choice_effects.rs:994 sets 3-turn timer; generate_instructions.rs:3051 hit at end of turn, damage_calc.rs:693 uses SpA of original mon, 0.925 x max | K1 only; also ignores weather/terrain, applies Light Screen only (checked via Weather::NONE args); source-ability effects (Infiltrator, Normalize) not seen | | medium |
| payback | 1 | MATCH | moves.ts:13183 50bp, x2 if target already moved and not newly switched | choices.rs:11862 50bp; choice_effects.rs:581 x2 if not first_move and opp not switching | | | medium |
| drillpeck | 1 | MATCH | moves.ts:4270 80bp 100acc Flying contact | choices.rs:4390 same | | | high |
| steelwing | 1 | MATCH | moves.ts:17910 70bp 90acc, 10% self def+1 | choices.rs:16116 same | | | high |
| muddywater | 1 | MATCH | moves.ts:12400 90bp 85acc, 30% acc -1 | choices.rs:11022 (gen6+ branch) same | | | high |
| accelerock | 1 | MATCH | moves.ts:34 40bp priority +1 contact | choices.rs:36 same | | | high |
| comeuppance | 1 | LOGIC | moves.ts:2681 returns 1.5x last damage taken from a foe this turn (floored, or 1 if 0), fails if none; a hit absorbed by Substitute records 0 so deals 1 | choices.rs:2735 bp 0; choice_effects.rs:1365 3/2 x damage dealt if not first mover, skipped if hit substitute | substitute-absorbed hit: SD does 1 damage, engine 0; otherwise same | S | medium |
| saltcure | 1 | MATCH | moves.ts:15628 40bp Rock, 100% saltcure volatile; champions moves.ts:842 residual 1/8 (Water/Steel 1/4 halved? i.e. 1/16 normal, 1/8 Water/Steel) | choices.rs:14085; generate_instructions.rs:3533 SALT_CURE_DAMAGE_DIVISOR 16 under champions (line 147), halved for Water/Steel | | | medium |
| heatcrash | 1 | MATCH | moves.ts:8479 BP 40/60/80/100/120 by weight ratio 2/3/4/5 | choices.rs:7421; choice_effects.rs:662 ratio cutoffs 0.5/0.3335/0.2501/0.2001 (equivalent except tiny slivers) | Float Stone not modelled | | medium |
| superfang | 1 | MATCH | moves.ts:18457 90acc, half target HP min 1 | choices.rs:16596 90acc; choices.rs:20510 (hp/2).max(1) | | | high |
| sing | 1 | LOGIC | moves.ts:16501 55acc sleep, sound, bypasssub | choices.rs:14953 55acc sleep, sound; generate_instructions.rs:799 immune_to_status blocks any status vs Substitute, only damage path checks sound (line 1438) | Sing sleep is blocked by Substitute in engine, bypasses it in Showdown (applies to all sound status moves) | S | medium |
| pollenpuff | 1 | MATCH | moves.ts:13559 90bp Bug bullet; ally branch heals 50% | choices.rs:12369 90bp bullet | ally-heal branch irrelevant in singles | | high |
| chillingwater | 1 | MATCH | moves.ts:2377 50bp, 100% atk -1 | choices.rs:2422 same | | | high |
| stockpile | 1 | MISSING | moves.ts:17956 stacks up to 3: +1 Def/SpD each, layers tracked, undone on end | choices.rs:16160 applies STOCKPILE volatile only; token never read (no genx hit besides enum state.rs:254) | no boosts, no layer count, no cap of 3 | M | high |
| tickle | 1 | MATCH | moves.ts:19603 atk -1 def -1 | choices.rs:17784 same | | | high |
| aquaring | 1 | MISSING | moves.ts:466 volatile, heals 1/16 max HP each residual | choices.rs:513 applies AQUARING volatile; no residual code (only enum state.rs:164 and a unit test) | no end-of-turn heal | M | high |
| attract | 1 | MISSING | moves.ts:706 infatuation: gender check, 50% cannot move, ends when source leaves | choices.rs:734 applies ATTRACT volatile; only state.rs:777 (Aroma Veil immunity); no immobilise roll, no gender | volatile does nothing | M | high |
| crabhammer | 1 | DATA | moves.ts:3129 100bp 90acc critRatio 2; champions moves.ts:153 accuracy 95 | choices.rs:3118 100bp 90acc; champions modifier choices.rs:20712 sets bp 95 (acc stays 90); high crit ok choices.rs:20471 | bp 95 vs 100; accuracy 90 vs 95 (the champions modifier is wrong, accuracy line missing) | S | high |
| entrainment | 1 | MISSING | moves.ts:4862 target copies user's ability with fail rules | choices.rs:4869 plain row, no effect (no code; instruction ChangeAbility exists) | no ability change | M | high |
| ficklebeam | 1 | LOGIC | moves.ts:5221 80bp Dragon, 30% chance base power x2, no stat effect | choices.rs:5247 80bp with 50% secondary self SpA +1 | wrong effect: invented stat boost, missing 30% double-power branch | M | high |
| clangoroussoul | 1 | LOGIC | moves.ts:2498; champions moves.ts:121 accuracy true; fails if hp <= 33% maxhp (float) or all boosts maxed; pays floor(33% maxhp), +1 all five | choice_effects.rs:199 requires hp > maxhp/3 (integer), pays 0.33; no fail when stats maxed | threshold differs for some max HP (e.g. 150: hp 50 works in engine, fails in SD); HP paid even if no stat can rise | S | medium |
| lunge | 1 | MATCH | moves.ts:10609 80bp, 100% atk -1 | choices.rs:9886 same | | | high |
| snarl | 1 | MATCH | moves.ts:17081 55bp 95acc sound, 100% spa -1 | choices.rs:15459 same | | | high |
| eerieimpulse | 1 | MATCH | moves.ts:4444 spa -2 | choices.rs:4570 same | | | high |
| aquatail | 1 | MATCH | moves.ts:511 90bp 90acc | choices.rs:556 same | | | high |
| topsyturvy | 1 | MISSING | moves.ts:19651 inverts all target stat boosts, fails if none | choices.rs:17831 plain row, no effect | no inversion | M | high |
| circlethrow | 1 | MATCH | moves.ts:2451 60bp 90acc priority -6, forceSwitch | choices.rs:2494 same, drag flag handled generate_instructions.rs:3669 | | | medium |
| meanlook | 1 | DATA | moves.ts:11494 accuracy true (never misses), adds trapped | choices.rs:10263 accuracy 100 default; trapped logic state.rs:758,1118 (Ghost and Shed Shell exempt, sub blocks) correct | never-miss is encoded as 100, and check_move_hit_or_miss (generate_instructions.rs:1230) scales by accuracy stage, so it can miss after accuracy drops | S | medium |
| wrap | 1 | CORE | moves.ts:21083 90acc 15bp, partiallytrapped, 5-6 turns, 1/8 per turn; conditions.ts:222 | choices.rs:19157 90acc 15bp; generate_instructions.rs:3516 1/8 per turn (line 142), removed when user switches (345) | K5 only; Binding Band (1/6) and Grip Claw duration not modelled | | medium |
| thunderfang | 1 | MATCH | moves.ts:19513 65bp 95acc bite, 10% par + 10% flinch | choices.rs:17642 same | | | high |
| waterspout | 1 | CORE | moves.ts:20657 150 x hp/maxhp, floored, min 1 | choices.rs:18740 150bp; choice_effects.rs:620 multiplies by float ratio | fractional BP not floored (K1-type rounding) | | medium |
| psyshieldbash | 1 | MATCH | moves.ts:14241 70bp, 100% self def+1; champions moves.ts:785 bp 90 | choices.rs:12929 70bp, def+1; modifier choices.rs:20754 bp 90 | | | high |
| mudslap | 1 | MATCH | moves.ts:12438 20bp 100acc, 100% acc -1 | choices.rs:11077 same | | | high |
| acupressure | 1 | MISSING | moves.ts:137 random stat below +6 gets +2, fail if none | choices.rs:199 target User, no effect | no effect | M | high |
| charge | 1 | LOGIC | moves.ts:2264 spd +1, next Electric move x2; removed after any Electric move used or aborted | choices.rs:2296 spd +1 and CHARGE volatile; damage_calc.rs:273 x2 for Electric; no removal code | Charge never consumed: all later Electric moves doubled until switch | M | high |
| imprison | 1 | MISSING | moves.ts:9491 foe cannot use any move the user knows | choices.rs:8721 applies IMPRISON volatile only; never read | no move restriction | M | high |
| aircutter | 1 | DATA | moves.ts:236 60bp 95acc critRatio 2, flags slicing + wind | choices.rs:301 (gen6+ branch) 60bp 95acc, flags protect only; crit ok choices.rs:20467 | slicing and wind flags missing (Sharpness, wind abilities and Tailwind-wind interactions) | S | high |

## Notes

- Never-miss moves (accuracy true in Showdown) are encoded as accuracy 100.0 with the user's accuracy stage applied (generate_instructions.rs:1230), so they can miss after an accuracy drop; engine also ignores target evasion there. Belongs to a general accuracy slice. Mean Look is the only never-miss opponent-target move in this slice.
- Fire-type damaging hit curing freeze (conditions.ts:117) is absent from the engine entirely; user-thaw list CHOICE_THAWS_USER (choice_effects.rs:23) has only 10 moves versus every defrost-flag move in Showdown (other slices).
- immune_to_status (generate_instructions.rs:776) has no sound exception for Substitute, affects all sound status moves.
- Float Stone is not present in genx; Low Kick and Heat Crash weights ignore it.
- Terrain Extender has no handler in engine (src grep), also affects the other terrain moves.
- Engine Future Sight uses the generate_instructions.rs:4511 helper and a separate damage function; weather and terrain are ignored (Weather::NONE passed).
- Aura Wheel type change is keyed to PokemonName::MORPEKOHANGRY, Hunger Switch form change must exist elsewhere.
