# Audit: MOVES slice 08 (CSV lines 366-417)

Slice: `03-coverage-moves.csv` lines 366 to 417 (jetpunch ... fly). Rows audited: 52. NOT AUDITED: none.

Paths used below: SD = `pcsa\.tools\vendor\pokemon-showdown\data\moves.ts`; CH = `...\data\mods\champions\moves.ts`; ENG = `poke-engine\src\choices.rs`; CE = `src\genx\choice_effects.rs`; GI = `src\genx\generate_instructions.rs`.
Build: Cargo features `champions`+`bss` imply NO gen feature, so every `cfg!(feature = "genN")` in choices.rs takes its final `else` branch (the newest-generation values). I read only those branches.

## Counts per verdict

| verdict | n | ids |
|---|---|---|
| MATCH | 12 | icehammer, feint, appleacid, afteryou, allyswitch, blastburn, clangingscales, doublehit, dragoncheer, dragonrush, extrasensory, firelash |
| PLAIN | 7 | jetpunch, charm, babydolleyes, cottonspore, electroweb, faketears, featherdance |
| DATA | 4 | snowscape, electricterrain, block, crushclaw |
| LOGIC | 15 | aromaticmist, axekick, bind, bounce, clearsmog, coaching, confuseray, courtchange, decorate, dig, dive, facade, flail, flatter, fly |
| PARTIAL | 4 | assurance, bugbite, covet, eeriespell |
| MISSING | 7 | beatup, copycat, corrosivegas, disable, electrify, fairylock, fling |
| CORE | 3 | bonerush (K2), crosschop (K7), crosspoison (K7) |
| UNSURE | 0 | |

## Five most important findings

1. Singles-illegal ally moves give free boosts: Coaching (+1 Atk/+1 Def) and Aromatic Mist (+1 SpD) are modelled as self-boosts, Showdown makes them fail in singles (target adjacentAlly, no ally). The search will pick them.
2. Positive stat raises on the foe (Decorate +2/+2, Flatter +1 SpA) are blocked by Clear Body / White Smoke / Full Metal Body / Clear Amulet / Hyper Cutter (Atk) because `immune_to_stats_lowered_by_opponent` has no sign check (state.rs:782). Showdown only blocks drops.
3. Not implemented at all: Disable (volatile set, nothing reads it), Electrify, Fairy Lock, Copycat, Fling, Beat Up (bp 0 so it deals nothing), Corrosive Gas (item removal); partial: Covet and Bug Bite lack their item effect, Assurance lacks the double power, Eerie Spell lacks the PP drain.
4. Semi-invulnerable moves (Dig, Dive, Bounce, Fly): the engine makes every move deal 0 damage to the user (damage_calc.rs:299-307). Showdown lets Earthquake/Magnitude (Dig), Surf/Whirlpool (Dive), Gust/Twister/Thunder/Hurricane/Sky Uppercut/Smack Down/Thousand Arrows (Fly, Bounce) hit, with 2x for Earthquake, Surf, Gust, Twister. Also no sand immunity while underground/underwater.
5. Confusion application is too permissive (Confuse Ray, Flatter, Axe Kick secondary): Own Tempo, Misty Terrain and Safeguard do not block it; Clear Smog resets boosts through a Substitute; Court Change does not swap Mist/Safeguard; Facade doubles through sleep; Crush Claw lacks the Champions `slicing` flag.

## Table

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| jetpunch | 1 | PLAIN | SD:9807 Water Physical bp60 acc100 prio+1, contact protect punch | ENG:8973 same numbers, contact protect punch, priority 1 | none | | high |
| snowscape | 1 | DATA | SD:17169 accuracy true, weather snowscape 5 turns; CH:932 pp only | ENG:15567 acc 100 default; CE:1674 sets SNOW 5 turns unless SNOW already or strong winds | accuracy true is stored as 100, so GI:1230 applies the user's accuracy stage (misses at negative stage); weather part matches (no Icy Rock in Champions) | S | medium |
| icehammer | 0 | MATCH | SD:9371 Ice Physical bp100 acc90, contact protect punch, self boost spe -1 | ENG:8560 acc90 bp100 contact protect punch, boost User spe -1 | | | high |
| feint | 0 | MATCH | SD:5176 bp30 prio+2 acc100, flags mirror only (no protect), breaksProtect | ENG:5179 bp30 prio2, no protect flag so it passes Protect | does not remove the target's Protect afterwards; no observable effect in singles (target already acted) | | medium |
| appleacid | 0 | MATCH | SD:421 Grass Special bp80, CH:18 bp90; protect, 100% spd -1 | ENG:457 bp80 then ENG:20706 sets 90; protect; 100% SpD -1 secondary | | | high |
| charm | 0 | PLAIN | SD:2339 Fairy acc100 protect reflectable, atk -2 on target | ENG:2351 same, boost Opponent atk -2, reflectable protect | none | | high |
| electricterrain | 0 | DATA | SD:4500 accuracy true, terrain 5 turns (8 with Terrain Extender, not in Champions items); 1.3x Electric (5325/4096), blocks sleep/yawn for grounded | ENG:4626 acc 100; CE:1690 sets terrain 5 turns, only if different; damage_calc.rs:222 boost 1.3 under champions; GI:826 sleep immunity for grounded | accuracy true stored as 100 (same as snowscape); rest matches | S | medium |
| afteryou | 0 | MATCH | SD:195 fails in singles (`activePerHalf === 1`) | ENG:244 no effect | none observable | | high |
| allyswitch | 0 | MATCH | SD:302 prio+2, onHit fails in singles | ENG:354 prio 2, target User, no effect | none observable | | high |
| aromaticmist | 0 | LOGIC | SD:591 target adjacentAlly, spd +1; no ally in singles so it fails | ENG:614 target User, boost User SpD +1 | engine gives the user +1 SpD; Showdown fails in singles | S | high |
| assurance | 0 | PARTIAL | SD:643 bp60, doubled (120) if target.hurtThisTurn; contact protect | ENG:671 bp60 contact protect, no logic | doubling vs a target already damaged this turn missing | M | high |
| axekick | 0 | LOGIC | SD:931 acc90 bp120 contact protect, onMoveFail crash 1/2 max HP, 30% confusion; hasCrashDamage | ENG:928 acc90 bp120, crash 0.5, 30% confusion; GI:1238 and GI:1413 apply crash on miss/immune/protect | crash damage ignores Magic Guard (Showdown treats it as non-move damage, Magic Guard blocks it); confusion has the common permissiveness problem (see confuseray) | S | medium |
| babydolleyes | 0 | PLAIN | SD:951 Fairy acc100 prio+1 atk -1, protect reflectable | ENG:950 same | none | | high |
| beatup | 0 | MISSING | SD:1150 multihit = healthy non-statused team members incl. user, each hit bp 5+baseAtk/10 of that member | ENG:1114 (else branch) bp 0 default, no logic | deals no damage, no hit count, no per-ally power | L | high |
| bind | 0 | LOGIC | SD:1337 acc85 bp15 contact protect, volatile partiallytrapped (5 turns, 1/8 per turn); not applied when the hit lands on a Substitute | ENG:1242 acc85 bp15, volatile PARTIALLYTRAPPED on opponent; GI:3516 1/8 max HP residual; GI:345 cleared on switch | volatile is applied even if the hit lands on a Substitute (volatile_status_can_be_applied has no case for it); never expires is K5; Grip Claw/Binding Band not modelled | S | medium |
| blastburn | 0 | MATCH | SD:1414 Fire Special bp150 acc90, recharge, self mustrecharge | ENG:1353 acc90 bp150 protect recharge; CE:695 applies MUSTRECHARGE after damage | | | high |
| block | 0 | DATA | SD:1511 accuracy true, flags reflectable mirror (no protect), adds `trapped` (Ghost immune) | ENG:1507 acc 100, reflectable only, volatile TRAPPED; state.rs:758 not on Substitute; state.rs:1129 Ghost not trapped; GI:361 removed when trapper switches | accuracy true stored as 100; otherwise matches | S | medium |
| bonerush | 0 | CORE | SD:1674 acc90 bp25, CH:89 bp30; multihit 2-5; protect | ENG:1684 acc90 bp25 then ENG:20711 bp30; multi_hit TwoToFiveHits (choices.rs:20568) | table matches; always 3 hits is K2 | | high |
| bounce | 0 | LOGIC | SD:1703 acc85 bp85 contact charge protect, 30% par; semi-invulnerable, hit by gust/twister (2x)/skyuppercut/thunder/hurricane/smackdown/thousandarrows; fails under Gravity | ENG:1715 acc85 bp85 charge contact protect, 30% par; charge via CE:1752; damage_calc.rs:299 any move deals 0 to a bouncing target | exceptions for Gust, Twister, Sky Uppercut, Thunder, Hurricane, Smack Down, Thousand Arrows missing (Gust/Twister 2x) | M | high |
| bugbite | 0 | PARTIAL | SD:1911 bp60 contact protect; onHit steals and eats target berry | ENG:1996 bp60 contact protect, no logic | berry steal-and-eat missing | L | high |
| clangingscales | 0 | MATCH | SD:2480 Dragon Special bp110 acc100, protect sound bypasssub, selfBoost def -1 | ENG:2560 bp110 protect sound, boost User def -1 (GI:3665); sound bypasses Substitute GI:1438 | | | medium |
| clearsmog | 0 | LOGIC | SD:2554 accuracy true, bp50, onHit target.clearBoosts() (not run when a Substitute takes the hit) | ENG:2599 acc 100 default, bp50; CE:883 resets target boosts after damage, not gated by `hit_sub` | boosts are cleared through a Substitute; accuracy true stored as 100 | S | high |
| coaching | 0 | LOGIC | SD:2590 target adjacentAlly, atk+1 def+1; fails in singles | ENG:2639 target User, boost User atk +1 def +1 | engine gives +1/+1 to the user; Showdown fails in singles | S | high |
| confuseray | 0 | LOGIC | SD:2728 acc100 Ghost, confusion; Own Tempo, Misty Terrain (grounded), Safeguard block it | ENG:2774 acc100, volatile CONFUSION; state.rs:752 only Substitute blocks | Own Tempo, Misty Terrain, Safeguard do not block confusion; duration never ends is K3 | M | high |
| copycat | 0 | MISSING | SD:2849 calls the last move used (any side), fails for failcopycat moves | ENG:2890 target User, no effect | no effect at all | L | high |
| corrosivegas | 0 | MISSING | SD:2914 acc100 Poison, onHit takes the target's item (CH:149 makes it legal) | ENG:2916 acc100 protect reflectable, no logic | item removal missing (also removes Mega Stone check/Sticky Hold interplay) | M | high |
| cottonspore | 0 | PLAIN | SD:2970 Grass acc100 powder protect reflectable, spe -2 | ENG:3007 (else) acc100 powder protect reflectable, boost Opponent spe -2; GI:1594 Grass immune; abilities.rs:2890 Overcoat | Safety Goggles powder immunity not found in genx (item is in CH items.ts:814) | S | medium |
| courtchange | 0 | LOGIC | SD:3032 acc100; swaps mist, lightscreen, reflect, spikes, safeguard, tailwind, toxicspikes, stealthrock, pledges, stickyweb, auroraveil, luckychant (and G-Max) | ENG:3048 acc100; CE:1078 swaps stealthrock, spikes, toxicspikes, stickyweb, reflect, lightscreen, auroraveil, tailwind | Mist and Safeguard (both exist as engine side conditions) and Lucky Chant not swapped | S | high |
| covet | 0 | PARTIAL | SD:3099 bp60 contact protect; onAfterHit takes item if user has none | ENG:3080 bp60 contact protect, no logic (CE:856 handles only THIEF) | item steal missing | M | high |
| crosschop | 0 | CORE | SD:3174 acc80 bp100 contact protect, critRatio 2 | ENG:3153 acc80 bp100 contact protect; choices.rs:20472 in increased_crit_ratio (1/8) | table and crit stage match; crit modelled only as branch is K7 | | medium |
| crosspoison | 0 | CORE | SD:3188 bp70 contact protect slicing, 10% psn, critRatio 2 | ENG:3169 bp70 contact protect slicing, 10% poison, in increased_crit_ratio | table matches; K7 | | medium |
| crushclaw | 0 | DATA | SD:3225 acc95 bp75, 50% def -1; CH:157 adds `slicing` | ENG:3249 acc95 bp75 contact protect, 50% def -1; not in ENG:20704 | `slicing` flag missing, so Sharpness (and other slicing effects) do not apply | S | high |
| decorate | 0 | LOGIC | SD:3391 accuracy true, no protect flag, atk +2 spa +2 on target | ENG:3428 acc 100, no protect, boost Opponent atk +2 spa +2; GI:944 applies the lowering-immunity check to raises | Clear Body / White Smoke / Full Metal Body / Clear Amulet / Hyper Cutter (Atk) block the raise in the engine, not in Showdown; accuracy true stored as 100 | S | high |
| dig | 0 | LOGIC | SD:3585 acc100 bp80 contact charge protect, charge turn; invulnerable except earthquake/magnitude (2x); sand/hail immune while underground | ENG:3656 acc100 bp80 charge contact protect; damage_calc.rs:299 all moves deal 0; GI:3027 sand damage still applies | Earthquake/Magnitude must hit for 2x; no sand immunity while digging | M | high |
| disable | 0 | MISSING | SD:3648 acc100 protect reflectable bypasssub; fails if target has no last move; duration 5, blocks last-used move; CH:228 adds cantusetwice case | ENG:3712 acc100 protect reflectable, volatile DISABLE; state.rs:775 only Aroma Veil check | volatile is set but nothing disables a move (grep finds no other reader); no duration | M | high |
| dive | 0 | LOGIC | SD:3750 acc100 bp80 contact charge protect; invulnerable except surf/whirlpool (2x); sand/hail immune | ENG:3782 acc100 bp80 charge contact protect; damage_calc.rs:299 all 0 | Surf/Whirlpool must hit for 2x; no sand immunity | M | high |
| doublehit | 0 | MATCH | SD:3896 acc90 bp35 contact protect, multihit 2 | ENG:3919 acc90 bp35 contact protect; choices.rs:20571 DoubleHit | | | high |
| dragoncheer | 0 | MATCH | SD:4059 target adjacentAlly, volatile dragoncheer (crit +1/+2); fails in singles; CH:268 adds sound flag | ENG:4130 Status, target User, no volatile, stray base_power 60 | no effect in both (engine bp on a Status move is unused) | | high |
| dragonrush | 0 | MATCH | SD:4194 acc75 bp100 contact protect, 20% flinch, minimize flag | ENG:4276 acc75 bp100 contact protect, 20% flinch; state.rs:765 flinch needs first move and not Inner Focus | Minimize double-damage not modelled (Minimize itself is only an enum, state.rs:209) | | medium |
| eeriespell | 0 | PARTIAL | SD:4461 bp80 Psychic Special protect sound bypasssub; 100% secondary removes 3 PP from target's last move | ENG:4595 bp80 protect sound, no secondary | PP drain missing (engine PP is K6 anyway) | M | high |
| electrify | 0 | MISSING | SD:4559 protect; volatile electrify for 1 turn turns target's next move Electric; fails if target already moved; CH:297 makes it legal | ENG:4637 protect, volatile ELECTRIFY; state.rs:179 only an enum entry | volatile has no behaviour | M | high |
| electroweb | 0 | PLAIN | SD:4663 acc95 bp55 protect, 100% spe -1 | ENG:4697 same | none | | high |
| extrasensory | 0 | MATCH | SD:4983 acc100 bp80 protect, 10% flinch | ENG:4984 same | | | high |
| facade | 0 | LOGIC | SD:5035 bp70 contact; onBasePower x2 if status and status != slp | ENG:5019 bp70 contact protect; CE:587 doubles if status != NONE (includes sleep); damage_calc.rs:200 burn halving skipped | doubles while asleep, Showdown does not | S | high |
| fairylock | 0 | MISSING | SD:5053 pseudoWeather fairylock, duration 2, bypasssub, accuracy true: nobody can switch next turn | ENG:5034 no fields, no logic | no trapping effect | M | high |
| faketears | 0 | PLAIN | SD:5113 Dark acc100 protect reflectable, spd -2 | ENG:5081 same | none | | high |
| featherdance | 0 | PLAIN | SD:5159 Flying acc100 protect reflectable dance, atk -2 | ENG:5135 same | none | | high |
| firelash | 0 | MATCH | SD:5363 bp80, CH:373 bp90; acc100 contact protect, 100% def -1 | ENG:5441 bp80 then ENG:20715 sets 90; 100% def -1 secondary | | | high |
| flail | 0 | LOGIC | SD:5532 bp from floor(hp*48/maxhp): <2:200, <5:150, <10:100, <17:80, <33:40, else 20 | ENG:5616 bp 0; CE:125 float ratio thresholds 0.688 / 0.354 / 0.208 / 0.104 / 0.042 | thresholds are decimal approximations of 33/48, 17/48, 10/48, 5/48, 2/48; at an exact boundary (e.g. 110/160 = 0.6875) the engine gives 40 where Showdown gives 20 | S | high |
| flatter | 0 | LOGIC | SD:5700 acc100 protect reflectable, spa +1 and confusion | ENG:5808 acc100, boost Opponent spa +1, volatile CONFUSION | Clear Body / White Smoke / Full Metal Body / Clear Amulet block the +1 (sign bug, see decorate); Own Tempo, Misty Terrain, Safeguard do not block confusion; K3 | S | high |
| fling | 0 | MISSING | SD:5736 Dark Physical, bp and effect from the held item, item consumed; fails without a flingable item | ENG:5862 bp 0, no logic | no damage, no item effect | L | high |
| fly | 0 | LOGIC | SD:5897 acc95 bp90 contact charge protect; invulnerable except gust/twister (2x)/skyuppercut/thunder/hurricane/smackdown/thousandarrows | ENG:5951 acc95 bp90 charge contact protect; damage_calc.rs:299 all 0 | same exceptions as bounce missing | M | high |

## Notes

- Systemic: Showdown `accuracy: true` has no engine representation (default `accuracy: 100.0`). GI:1230 multiplies by the user's accuracy-stage multiplier, so a user at a negative accuracy stage can "miss" with Snowscape, Electric Terrain, Block, Clear Smog, Decorate and so on. Cheap fix: use a large accuracy (for example 1000.0) for every `accuracy: true` move, because `.min(1.0)` caps it. This applies to all such moves in the whole table, not just this slice.
- `choice.boost` for moves that target the opponent goes through `immune_to_stats_lowered_by_opponent` regardless of the sign of the boost (GI:944, state.rs:782). It affects every move that raises a foe's stats (Decorate, Flatter, Swagger, and similar).
- Confusion (state.rs:752) lacks Own Tempo, Misty Terrain and Safeguard checks. Belongs to the ability/field slices but hits Confuse Ray, Flatter and the confusion secondaries (Axe Kick).
- Mist (a side condition in the engine) is not consulted by stat-lowering moves (Charm, Fake Tears, Feather Dance, Cotton Spore). I did not verify this exhaustively; check in the side-condition slice.
- Sand damage (GI:3027) does not exempt a Pokemon that is Digging or Diving.
- Weather and terrain duration items (Icy Rock, Terrain Extender, and so on) are absent from the engine; I found neither in the Champions override files, so I treated them as out of scope.
- The `flags` struct has no `bypasssub`, `mirror`, `gravity`, `minimize`, `snatch`, `allyanim`; only sound is hard-wired to bypass Substitute (GI:1438). Moves that bypass Substitute without the sound flag (Disable, Decorate is not one, Fairy Lock, Coaching, Dragon Cheer, Aromatic Mist) are not covered; irrelevant for the ones that do nothing in the engine.
- Axe Kick's Magic Guard finding is from reading Magic Guard's `effect.effectType !== 'Move'` rule against the `damage(..., conditions.get('Axe Kick'))` call; I did not open abilities.ts for the exact text, hence medium confidence.
- The CSV line 417 is Fly; the slice ends there (52 rows).
