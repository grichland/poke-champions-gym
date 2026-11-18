# Audit moves-09 (03-coverage-moves.csv lines 418-469)

Slice: MOVES, ids flyingpress .. rockpolish. Rows audited: 52 of 52. NOT AUDITED: none.
Build assumed: features `champions` + `bss`, no genN feature, so the engine is genx and every `cfg!(feature="gen1..gen6")` branch in choices.rs is not taken (gen9-style rows). `apply_champions_modifiers` (choices.rs:20704) is applied (choices.rs:19343).
Abbreviations: SD = `Pokemon Showdown data\moves.ts`, SDc = `data\mods\champions\moves.ts`, CH = `choices.rs`, CE = `genx\choice_effects.rs`, GI = `genx\generate_instructions.rs`, DC = `genx\damage_calc.rs`.
`A1` = new cross-cutting finding (see Notes): engine applies the attacker's accuracy stage to accuracy:true / self-target moves (SD never rolls accuracy for them).
All 52 rows have used=0.

## Summary

| verdict | count |
|---|---|
| PLAIN | 9 |
| MATCH | 7 |
| CORE | 2 |
| LOGIC | 7 |
| PARTIAL | 7 |
| MISSING | 20 |
| DATA / UNSURE | 0 |

Five most important findings:
1. Guillotine (OHKO) can never hit: CH row has base_power 0, no OHKO logic anywhere in src. `calculate_damage` returns (0,0) for bp 0 (DC:620) and `check_move_hit_or_miss` sets percent_hit=0 when damage==(0,0) (GI:1231). Accuracy 30 is irrelevant.
2. Flying Press has no Flying-type component in its effectiveness (SD onEffectiveness); the engine treats it as plain Fighting. Wrong vs Rock/Steel/Grass/Fighting/Bug/Flying/Electric/Dark... many matchups.
3. A1: never-miss and self-target moves (Milk Drink, Heal Bell, Life Dew, Rock Polish, Magnet Rise ...) get accuracy 100 x attacker accuracy stage (GI:1229-1233), so after an accuracy drop (Night Daze, Sand Attack) they fail with 25%+ probability. Evasion is not modelled at all.
4. Gyro Ball (and Electro Ball, same code) read speed from `calculate_boosted_stat` (stage only), ignoring Choice Scarf, Tailwind, paralysis, Iron Ball, weather/terrain speed abilities; SD uses `getStat('spe')` with all modifiers.
5. 20 rows have table entries but no behaviour (Gastro Acid, Octolock, Quick Guard, Power Trick/Shift, Lock-On, Psych Up, Reflect Type, Recycle, Guard/Power Swap/Split, Magic Room, Magic Powder, Forest's Curse, Instruct, Heal Pulse, Magnetic Flux, Guillotine). Volatiles GASTROACID, OCTOLOCK, POWERTRICK, POWERSHIFT, RAGEPOWDER and side condition QuickGuard exist in enums and are written, but nothing reads them.

## Table

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| flyingpress | 0 | PARTIAL | SD:5938 acc95 bp100 Phys Fighting contact protect; onEffectiveness adds Flying effectiveness to Fighting | CH:5990-6006 acc95 bp100 Phys Fighting contact protect; no FLYINGPRESS reference elsewhere in src | dual-type effectiveness (Fighting x Flying) missing; damage computed as Fighting only | M | high |
| forestscurse | 0 | MISSING | SD:6128 acc100 Status Grass protect reflectable; onHit adds Grass type (fails if already Grass) | CH:6126 same flags, no effect fields; no code | target never gains Grass type (no added-type mechanism used) | M | high |
| frenzyplant | 0 | MATCH | SD:6245 acc90 bp150 Spec Grass protect recharge; self mustrecharge | CH:6228 same; recharge flag -> MUSTRECHARGE applied in CE:695 only after a damaging hit (GI:1578) | none found | | medium |
| frostbreath | 0 | MATCH | SD:6261 acc90 bp60 Spec Ice protect; willCrit | CH:6262-6277 acc90 bp60 Spec Ice protect; guaranteed_crit CH:20495-20504; GI:2436,2459,2472 crit damage used, Battle/Shell Armor excluded | none found (crit multiplier itself is K7/other slice) | | medium |
| gastroacid | 0 | MISSING | SD:6429 acc100 Status Poison protect reflectable; volatile gastroacid suppresses ability; fails vs cantsuppress / Ability Shield | CH:6461 same flags; volatile GASTROACID applied, only reader is the enum | ability never suppressed; no cantsuppress/Ability Shield check | L | high |
| gigaimpact | 0 | MATCH | SD:6576 acc90 bp150 Phys Normal contact protect recharge | CH:6569 same; recharge via CE:695 | none found | | medium |
| gravapple | 0 | PARTIAL | SD:7734 acc100 bp80 Phys Grass protect; 100% Def-1; onBasePower x1.5 under Gravity. SDc:459 bp90 | CH:6864 bp80, 100% Def-1 on opp; CH:20720 sets bp90 (champions) | Gravity x1.5 missing; no Gravity pseudo-weather exists in genx (grep GRAVITY/gravity: none). Check if Gravity is legal in M-C before costing | M | medium |
| guardsplit | 0 | MISSING | SD:7952 acc true Psychic protect; averages Def and SpD stored stats of user and target | CH:6995 protect only, acc 100 | no stat averaging; also A1 | L | high |
| guardswap | 0 | MISSING | SD:7975 acc true Psychic protect mirror bypasssub; swaps Def and SpD stages | CH:7007 protect only, acc 100 | no boost swap; also A1 | M | high |
| guillotine | 0 | MISSING | SD:8004 acc30 Phys Normal contact protect; ohko:true (OHKO, acc 30 at equal level, Sturdy/immunity rules) | CH:7019 acc30 bp0 Phys contact protect; no ohko handling in src (grep ohko: only tests) | damage = (0,0) so percent_hit=0 (GI:1231): move never hits; needs fixed damage = target hp (+Sturdy, Ice-type irrelevant) | S | high |
| gyroball | 0 | LOGIC | SD:8050 acc100 Phys Steel contact protect bullet; bp = floor(25*targetSpe/userSpe)+1 cap 150, getStat('spe') incl. all modifiers | CH:7115 flags ok; CE:553 bp=(25*defSpd/atkSpd+1).min(150) using calculate_boosted_stat (stage only, genx\state.rs:999-1071) | speed ignores Scarf, Tailwind, paralysis, Iron Ball, Swift Swim etc (get_effective_speed GI:2535 has them, not used); bp not floored | S | high |
| hardpress | 0 | LOGIC | SD:8140 acc100 Phys Steel contact protect; bp=floor(100*hp/maxHP) (min 1) | CH:7204 same flags; CE:189 bp=100*hp/maxhp as float | bp not floored/rounded (fractional bp, up to 1 bp off) | S | medium |
| healbell | 0 | LOGIC | SD:8243 acc true sound bypasssub; cures status of all party mons except bench/active allies with Soundproof or Good as Gold (source always cured) | CH:7309 sound, target User, acc 100; CE:1410 cures every party mon | Soundproof / Good as Gold bench mons wrongly cured; A1 | S | high |
| healpulse | 0 | MISSING | SD:8402 acc true Psychic protect reflectable pulse heal; heals target 50% (75% Mega Launcher), fails at full HP; in singles target is the foe | CH:7377 flags only, no heal field, target Opponent | no healing at all | S | high |
| infernalparade | 0 | PARTIAL | SD:9545 acc100 bp60 Spec Ghost protect; 30% brn; bp x2 if target has status or Comatose. SDc:536 bp65 | CH:8776 bp60 Spec Ghost protect 30% burn; CH:20722 bp65 | status-doubling missing (bp should be 130 vs statused target); no reference elsewhere in src | S | high |
| inferno | 0 | PLAIN | SD:9565 acc50 bp100 Spec Fire protect; 100% brn | CH:8795 same | none | | high |
| instruct | 0 | MISSING | SD:9643 acc true Psychic protect bypasssub; target repeats its last move immediately (failinstruct list, pp>0) | CH:8850 protect only | no effect | L | high |
| irontail | 0 | PLAIN | SD:9741 acc75 bp100 Phys Steel contact protect; 30% Def-1 | CH:8917 same | none | | high |
| jawlock | 0 | PARTIAL | SD:9791 acc100 bp80 Phys Dark contact protect bite; onHit both user and target get trapped | CH:8959 damage, bite, contact, protect ok; no volatile | mutual trap missing (TRAPPED volatile exists, CH:1517 used by other moves, cleared on opposing switch GI:361) | M | high |
| lashout | 0 | PARTIAL | SD:10050 acc100 bp75 Phys Dark contact protect; bp x2 if user's stats were lowered this turn | CH:9237 bp75 contact protect; no reference elsewhere | doubling missing; engine has no "stats lowered this turn" tracking | M | high |
| lifedew | 0 | LOGIC | SD:10288 acc true Water heal bypasssub; heal 1/4 of baseMaxhp, rounded (BA:1265 Math.round); singles = user | CH:9592 heal 0.25 User; GI:1184 truncates `(amount*maxhp) as i16` | heal rounds down instead of nearest (e.g. maxhp 163: 41 vs 40); A1 | S | high |
| lockon | 0 | MISSING | SD:10396 acc true Normal protect; user's next move (2-turn volatile) cannot miss and ignores semi-invulnerability | CH:9687 protect only | no lock-on volatile | M | high |
| magicpowder | 0 | MISSING | SD:10740 acc100 Psychic protect reflectable powder; target type becomes Psychic | CH:10038 flags only; powder immunity for Grass handled generically GI:1594 | no type change (Overcoat/Grass immunity okay) | M | high |
| magicroom | 0 | MISSING | SD:10756 acc true Psychic; 5-turn room, items suppressed for all mons | CH:10052 flags only | no pseudo-weather, no item suppression | L | high |
| magnetrise | 0 | PARTIAL | SD:10855 acc true Electric; 5 turns, Ground immunity; fails if Smack Down/Ingrain/Gravity | CH:10155 target User, volatile MAGNETRISE; DC:290 returns 0 for Ground moves (except Thousand Arrows) | never expires (no duration code); no fail conditions; not counted in `is_grounded` (state.rs:720) so Spikes/Toxic Spikes/Sticky Web/terrains still hit; A1 | M | high |
| magneticflux | 0 | MISSING | SD:10828 acc true Electric snatch bypasssub; +1 Def/SpD to allies with Plus/Minus | CH:10143 target User, no effect; no Plus/Minus ability code in genx | no effect | S | high |
| metalsound | 0 | PLAIN | SD:11708 acc85 Steel protect reflectable sound bypasssub; SpD-2 | CH:10450 acc85 sound protect reflectable; boost SpD-2 opp | none (sound flag also drives sub bypass for damage; boosts are not sub-blocked in engine at all, see Notes) | | high |
| meteorbeam | 0 | MATCH | SD:11741 acc90 bp120 Spec Rock charge protect; turn 1 SpA+1, Power Herb skips charge | CH:10491 acc90 bp120 charge protect; CE:1029 SpA+1 on charge turn; CE:1043 Power Herb; GI:1712 charge volatile; GI:2241 volatile removed on turn 2; test_battle_mechanics.rs:13521 | none found (accuracy only checked on turn 2 in both) | | medium |
| milkdrink | 0 | LOGIC | SD:11825 acc true Normal heal snatch; 1/2 baseMaxhp rounded; SDc:643 target adjacentAllyOrSelf (self in singles) | CH:10600 heal 0.5 User; GI:1184 truncates | rounds down instead of nearest (odd maxhp: 1 HP less); A1 | S | high |
| nightdaze | 0 | PLAIN | SD:12679 acc95 bp85 Spec Dark protect; 40% acc-1. SDc:680 bp90 | CH:11319 acc95 bp85, 40% acc-1; CH:20753 bp90 | none | | high |
| nightshade | 0 | MATCH | SD:12727 acc100 Spec Ghost protect; damage = user level | CH:11362 Spec Ghost protect; CH:20509 fixed_damage = level; GI:2353 immunity check by type chart | none found (level assumed 50) | | medium |
| nobleroar | 0 | PLAIN | SD:12771 acc100 Normal protect reflectable sound bypasssub; Atk-1 SpA-1 | CH:11391 same | none | | high |
| octolock | 0 | MISSING | SD:12957 acc100 Fighting protect; trap + each end of turn Def-1 SpD-1 while source active; Ghost immune | CH:11555 volatile OCTOLOCK applied; no reader in src | no trapping, no residual drops, no Ghost immunity | M | high |
| petalblizzard | 0 | PLAIN | SD:13275 acc100 bp90 Phys Grass protect wind | CH:11921 same | none (target allAdjacent only matters in doubles) | | high |
| petaldance | 0 | CORE | SD:13288 acc100 bp120 Spec Grass contact protect dance; self lockedmove (2-3 turns random, then confusion) | CH:11975-11994 bp120 Spec Grass contact protect; LOCKEDMOVE fixed 3 turns then confusion (GI:3290-3345) | K4 only; `dance` flag absent from engine Flags and no Dancer ability code in genx (Notes) | | high |
| pluck | 0 | PARTIAL | SD:13436 acc100 bp60 Phys Flying contact protect distance; eats target's berry | CH:12145 bp60 Flying contact protect; no berry steal | berry steal+eat effect missing | M | high |
| poisonfang | 0 | PLAIN | SD:13460 acc100 bp50 Phys Poison contact protect bite; 50% tox | CH:12185 same, 50% TOXIC | none | | high |
| poisonpowder | 0 | PLAIN | SD:13509 acc75 Poison protect reflectable powder; psn | CH:12292 same | none (Grass immunity GI:1594) | | high |
| pound | 0 | PLAIN | SD:13643 acc100 bp40 Phys Normal contact protect | CH:12446 same | none | | high |
| powershift | 0 | MISSING | SD:13717 acc true Normal snatch; volatile swaps stored Atk and Def | CH:12535 volatile POWERSHIFT applied; no reader | stats never swapped | M | high |
| powersplit | 0 | MISSING | SD:13756 acc true Psychic protect; averages Atk and SpA stored stats | CH:12551 protect only | no effect; A1 | L | high |
| powerswap | 0 | MISSING | SD:13779 acc true Psychic protect bypasssub; swaps Atk and SpA stages | CH:12563 protect only | no effect; A1 | M | high |
| powertrick | 0 | MISSING | SD:13808 acc true Psychic snatch; volatile swaps stored Atk and Def | CH:12575 volatile POWERTRICK applied; no reader | stats never swapped | M | high |
| powertrip | 0 | MATCH | SD:13848 acc100 bp20 Phys Dark contact protect; bp = 20 + 20 x positive stages (all 7 stats) | CH:12591 bp20; CE:592 bp +20 per positive stage over atk/def/spa/spd/spe/acc/eva | none found | | high |
| psychup | 0 | MISSING | SD:14208 acc true Normal bypasssub; copies target stages and crit volatiles | CH:12921 no flags, no effect | no effect; A1 | M | high |
| quash | 0 | MATCH | SD:14451 acc100 Dark protect; fails in singles (activePerHalf==1) | CH:13087 protect only, no effect | none observable (fails/no-op both) | | high |
| quickguard | 0 | MISSING | SD:14486 acc true Fighting prio+3; side cond blocks priority>0 moves this turn; fails if user moves last; chain shares `stall` counter | CH:13115 prio3, QuickGuard side condition set; counter only in state.rs:611,1271; never read, never cleared | blocks nothing; condition persists forever once set | M | high |
| ragepowder | 0 | LOGIC | SD:14595 acc true Bug prio+2 powder; onTry fails in singles (activePerHalf>1 needed) | CH:13186 prio2 powder target User; volatile RAGEPOWDER applied, never read or cleared | should fail in singles; engine succeeds with inert volatile (no battle effect, state differs) | S | medium |
| ragingfury | 0 | CORE | SD:14661 acc100 bp120 Phys Fire protect; self lockedmove (2-3 turns, confuse) | CH:13219 same, LOCKEDMOVE | K4 only (no `dance` flag involved) | | high |
| recycle | 0 | MISSING | SD:14818 acc true Normal snatch; restores lastItem | CH:13384 no flags, no effect; Pokemon has no last-item field | no effect (also affects Knock Off/berry/consumed-item states) | L | high |
| reflecttype | 0 | MISSING | SD:14880 acc true Normal protect bypasssub; user copies target's types | CH:13431 protect only | no type copy | M | high |
| rockpolish | 0 | LOGIC | SD:15219 acc true Rock; Spe+2 self | CH:13697 target User, speed+2; acc 100 default | boost correct; A1 (fails if user has accuracy<0) | S | medium |

## Notes

- A1 (new, cross-cutting, GI:1229-1233 `check_move_hit_or_miss`): `percent_hit = choice.accuracy/100 * boosted_accuracy(attacker accuracy_boost)`. Showdown skips the accuracy roll for `accuracy: true` moves and self-targeted moves. In the engine such rows have accuracy 100, so any negative accuracy stage on the user (Night Daze, Sand Attack, Mud-Slap) makes Milk Drink, Heal Bell, Rock Polish, Swords Dance, Recover... fail with probability 1 - 3/(3+n). The defender's evasion stage is never read (grep evasion_boost in GI: only the boost application), so evasion boosts/drops do nothing either. Belongs to the accuracy/core slice; the fix is small (skip when target==User or accuracy flagged never-miss; add evasion).
- Status/stat moves against a Substitute: GI:1429-1445 only lets sound bypass for damage; for non-damaging boost moves `get_instructions_from_boosts` (GI:1013) has no Substitute check, so drops from non-sound moves would pass through a Substitute (Metal Sound and Noble Roar are sound and correct). Other slices should check Growl-like, Charm, Screech, etc. (Showdown blocks non-bypass status moves through Substitute).
- Dancer: Flags (CH:19436) has no `dance` flag and genx has no Abilities::DANCER code; Petal Dance (and Fiery Dance etc.) will not be copied. Check whether Dancer is in the M-C pool.
- Gyro Ball and Electro Ball share the same speed shortcut (CE:537-559); the fix is to call `get_effective_speed` (GI:2535, currently private) from CE. Neither floor() the Showdown way.
- Heal rounding: GI:1184 truncates every heal in the table (Recover, Roost, Soft-Boiled, Slack Off, Moonlight...) while Showdown rounds to nearest (battle-actions.ts:1265 `Math.round` for gen>=5). Affects all heal moves with odd or non-multiple HP.
- Meteor Beam / charge moves with Power Herb: CE:1043-1055 sets `choice.flags.charge=false` before `modify_choice` (GI:1690), so the charge branch at GI:1712 is skipped and the move hits immediately, matching Showdown. Verified order: before_move calls choice_before_move (GI:1688) then modify_choice (GI:1690).
- Heal Pulse in singles heals the foe in Showdown; low value, but listed as MISSING for completeness.
- Grav Apple: Gravity itself (SD pseudoWeather) does not exist in genx at all; if Gravity is legal in M-C it is a separate gap (move row GRAVITY exists in CH with no state).
- Champions overrides in SDc that hit this slice: gravapple bp90, infernalparade bp65, milkdrink target, nightdaze bp90, octolock/powershift isNonstandard null (now legal). The first, second and fourth are present in `apply_champions_modifiers`; the milkdrink target change has no effect in singles.
