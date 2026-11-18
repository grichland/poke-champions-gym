# Second read: ITEMS slice b1 (csv lines 2-63, leftovers .. mysticwater)

SUMMARY: 62 rows audited (first id leftovers, last id mysticwater confirmed). MATCH 38, LOGIC 11, DATA 5, MISSING 7, PARTIAL 1, CORE 0, UNSURE 0, PLAIN 0. Mega-stone rows are MATCH when form/types/stats/ability agree; global stone issues N1/N2 (see Notes) are not repeated per row.

Paths: SD = pcsa/.tools/vendor/pokemon-showdown/data (items.ts, pokedex.ts, mods/champions/items.ts). EN = research/pcsa-rl/poke-engine-eval/poke-engine/src/genx (items.rs, generate_instructions.rs [gi.rs], base_stats.rs, state.rs). Build = champions+bss, no genN feature, so non-gen4/gen5 branches count.

| id | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | confidence |
|---|---|---|---|---|---|
| leftovers | LOGIC | items.ts:3350 residual order 5 sub 4, heal baseMaxhp/16 (blocked by Heal Block) | items.rs:972 maxhp/16 in item_end_of_turn; gi.rs:3117-3212 status damage runs BEFORE items; leech seed gi.rs:3217 after | order: SD Leftovers (5) is before leech seed (8), poison (9), burn (10); engine does poison/burn first, so a low-HP holder faints before healing. No Heal Block check | high |
| lifeorb | LOGIC | items.ts:3422 onModifyDamage x5324/4096; onAfterMoveSecondarySelf 10% baseMaxhp to user for non-Status; skipped by Sheer Force and Magic Guard | items.rs:1194 BP x1.3, adds Heal(-0.1) secondary (not Magic Guard) | Sheer Force: ability_modify_attack_being_used (abilities.rs:2233) clears secondaries first, then item code (gi.rs:1692-1695) re-adds LO recoil, so Sheer Force + LO takes recoil; SD does not. Also BP-stage vs damage-stage multiplier (K1-ish rounding) | medium |
| focussash | LOGIC | items.ts:2281 onDamage: only if hp==maxhp AND damage>=hp from a Move; useItem consumes; leaves 1 HP | gi.rs:1491-1497 at full HP subtracts 1 from damage_dealt for ANY damage (no damage>=hp test); item never removed (only Items::FOCUSSASH ref) | (1) non-lethal hits at full HP deal 1 less; (2) sash not consumed, works again after healing to full | high |
| choicescarf | LOGIC | items.ts:995 onModifySpe x1.5; choicelock volatile added on move, removed if item no longer choice | gi.rs:2591 speed x1.5 OK; items.rs:793 disables other 3 moves on use (item_before_move); re-enable only on switch/taunt-end (state.rs:1467, gi.rs:2204) | speed matches. Lock is stored as disabled flags: after Trick/Switcheroo/Knock Off removes the scarf, moves stay disabled (SD drops lock). Trick scarf user stuck on Trick | medium |
| sitrusberry | LOGIC | items.ts:5768 onUpdate at hp<=1/2 (any HP change, immediately); heal baseMaxhp/4 | items.rs:754-762 before holder's own move at hp<=1/4 (<=1/2 only with Gluttony); items.rs:915 end of turn at <=1/2; heal maxhp/4 OK | timing: SD eats right after the damaging event; engine only at holder's next move (at <=1/4 only) or end of turn. A holder hit to 30-50% and moving second is not healed first. No Heal Block check | high |
| rockyhelmet | LOGIC | items.ts:5319 onDamagingHit contact: damage attacker baseMaxhp/6 (Magic Guard blocks; not through Substitute; per hit) | items.rs:1076 contact -> secondary Heal(-0.166) on User, applied once per move even if hit_sub (gi.rs:1072 skips only Opponent-target secondaries) | (a) fires through Substitute; (b) ignores attacker Magic Guard; (c) 0.166 vs 1/6 (maxhp divisible by 6 loses 1 HP); (d) once per move not per hit (K2-adjacent) | high |
| garchompitez | DATA | pokedex.ts:8183 Garchomp-Mega-Z Dragon, 108/130/85/141/85/151, ability Levitate | base_stats.rs:656-661 Dragon/-, same stats, ability SANDFORCE | ability Levitate vs Sand Force (Levitate = Ground immunity) | high |
| golisopite | DATA | pokedex.ts:14531 Golisopod-Mega Bug/Steel 75/150/175/70/120/40, ability Tough Claws | base_stats.rs:632-637 same types/stats, ability EMERGENCYEXIT | ability Tough Claws vs Emergency Exit | high |
| salamencite | MATCH | pokedex.ts:6935 Dragon/Flying 95/145/130/120/90/120 Aerilate | base_stats.rs:350-355 identical | none | high |
| lucarionitez | DATA | pokedex.ts:8258 Lucario-Mega-Z Fighting/Steel 70/100/70/164/70/151, ability Aura Guard | base_stats.rs:662-667 same types/stats, ability ADAPTABILITY (copy of Lucario-Mega) | ability Aura Guard vs Adaptability; AURAGUARD absent from Abilities enum (abilities.rs) so fix is M not S | high |
| dragoninite | MATCH | pokedex.ts:3365 Dragon/Flying 91/124/115/145/125/100 Multiscale | base_stats.rs:416-421 identical | none | high |
| lightclay | MISSING | items.ts:3462 + moves.ts:846,10330,14852 hasItem('lightclay') -> screens 8 turns | not in Items enum (items.rs:25-282); parses to UNKNOWNITEM; screen durations fixed | holder's Reflect/Light Screen/Veil lasts 5 not 8 | high |
| charizarditey | MATCH | pokedex.ts:138 Fire/Flying 78/104/78/159/115/100 Drought | base_stats.rs:146-151 identical | none | high |
| metagrossite | MATCH | pokedex.ts:6993 Steel/Psychic 80/145/150/105/110/110 Tough Claws | base_stats.rs:356-361 identical | none | high |
| charizarditex | MATCH | pokedex.ts:123 Fire/Dragon 78/130/111/130/85/100 Tough Claws | base_stats.rs:140-145 identical | none | high |
| terrainextender | MISSING | items.ts:6307 + moves.ts:4514,7690,12167,14106 hasItem('terrainextender') -> terrain 8 turns | not in Items enum; UNKNOWNITEM; terrain 5 turns always | terrain set by holder lasts 5 not 8 | high |
| blazikenite | MATCH | pokedex.ts:5117 Fire/Fighting 80/160/80/130/80/100 Speed Boost | base_stats.rs:266-271 identical | none | high |
| airballoon | LOGIC | items.ts:185 airborne; onDamagingHit pops; onAfterSubDamage pops when a move damages the Substitute | items.rs:1029 Ground move BP=0 (Thousand Arrows exempt); other non-status Opponent moves add RemoveItem secondary, skipped when hit_sub (gi.rs:1072); state.rs:727 not grounded | balloon not popped when the hit lands on a Substitute (SD pops); pop is once per move not first hit (K2-adjacent); Gravity/Smack Down/Ingrain grounding not modelled | medium |
| baxcalibrite | MATCH | pokedex.ts:19058 Dragon/Ice 115/175/117/105/101/87 Thermal Exchange | base_stats.rs:650-655 identical | none | high |
| staraptite | MATCH | pokedex.ts:7438 Fighting/Flying 85/140/100/60/90/110 Contrary | base_stats.rs:560-565 identical | none | high |
| raichunitey | MATCH | pokedex.ts:852 Raichu-Mega-Y Electric 60/100/55/160/80/130 No Guard | base_stats.rs:620-625 Electric/-, identical stats, NOGUARD | none | high |
| glimmoranite | MATCH | pokedex.ts:18589 Rock/Poison 83/90/105/150/96/101 Adaptability | base_stats.rs:482-487 identical | none | high |
| blackglasses | MATCH | items.ts:529 onBasePower Dark x4915/4096 (1.2) | items.rs:1155 Dark BP x1.2 | none beyond float vs 4915/4096 rounding | high |
| greninjite | MATCH | pokedex.ts:12200 Water/Dark 72/125/77/133/81/142 Protean | base_stats.rs:458-463 identical | none | high |
| clefablite | MATCH | pokedex.ts:1045 Fairy/Flying 95/80/93/135/110/70 Magic Bounce | base_stats.rs:422-427 identical | none | high |
| quickclaw | MISSING | items.ts:5013 onFractionalPriority: if priority<=0, 20% chance to move first (not Status with Mycelium Might) | not in Items enum; UNKNOWNITEM; no priority branch (modify_choice_priority gi.rs:2603 has no item) | 20% move-first branch absent (K10: branch with p=0.2 missing) | high |
| gyaradosite | MATCH | pokedex.ts:2965 Water/Dark 95/155/109/70/130/81 Mold Breaker | base_stats.rs:200-205 identical | none | high |
| normalgem | LOGIC | items.ts:4342 onSourceTryPrimaryHit: not Status, type Normal, then useItem + gem volatile x5325/4096 (conditions.ts:467) | items.rs:625,330 power_up_gem BP x1.3 (non-gen5), consumed in item_before_move for any category | (a) consumed by Status Normal moves (Swords Dance); SD excludes Status; (b) consumed before accuracy/Protect/immunity so a miss or Protect wastes it; SD consumes only after the move gets through | high |
| shucaberry | LOGIC | items.ts:5714 onSourceModifyDamage: Ground, typeMod>0, not when hitting Substitute, eat then x0.5 | items.rs:590,304 BP/2 and consume in item_before_move when move_type Ground and effectiveness >1 | consumed before accuracy/Protect/Substitute check, so miss/Protect/Sub wastes it; also eaten by Status Ground moves (Sand Attack). Damage effect itself matches (BP/2 vs dmg x0.5 rounding only) | high |
| swampertite | MATCH | pokedex.ts:5176 Water/Ground 100/150/110/95/110/70 Swift Swim | base_stats.rs:272-277 identical | none | high |
| starminite | DATA | pokedex.ts:2747 Starmie-Mega Water/Psychic 60/100/105/130/105/120 Huge Power | base_stats.rs:500-505 same types/ability, Atk 140 | base Atk 140 vs 100 (40 higher; large damage error) | high |
| delphoxite | MATCH | pokedex.ts:12108 Fire/Psychic 75/69/72/159/125/134 Levitate | base_stats.rs:452-457 identical | none | high |
| damprock | MISSING | items.ts:1273 + conditions.ts:485 rain 8 turns if holder set it | not in Items enum; UNKNOWNITEM; weather turns fixed 5 | holder-set rain lasts 5 not 8 | high |
| tyranitarite | MATCH | pokedex.ts:4961 Rock/Dark 100/164/150/95/120/71 Sand Stream | base_stats.rs:254-259 identical | none | high |
| gengarite | MATCH | pokedex.ts:2224 Ghost/Poison 60/65/80/170/95/130 Shadow Tag | base_stats.rs:182-187 identical | none | high |
| lopunnite | MATCH | pokedex.ts:7932 Normal/Fighting 65/136/94/54/96/135 Scrappy | base_stats.rs:374-379 identical | none | high |
| blastoisinite | MATCH | pokedex.ts:214 Water 79/103/120/135/115/78 Mega Launcher | base_stats.rs:152-157 identical | none | high |
| lumberry | LOGIC | items.ts:3555 onAfterSetStatus + onUpdate: eat at once on any status OR confusion; cureStatus + remove confusion | gi.rs:886 blocks a move-applied major status and consumes; items.rs:751 pre-move, items.rs:912 end of turn; no confusion handling (LUMBERRY refs only these) | (a) confusion never cured; (b) statuses from non-move paths (hazards, ability contact, Orbs) cured only at holder's next move or end of turn, after the poison/burn tick; SD cures immediately | medium |
| nevermeltice | MATCH | items.ts:4314 Ice x4915/4096 | items.rs:1236 Ice BP x1.2 | none (float rounding only) | high |
| lucarionite | MATCH | pokedex.ts:8243 Fighting/Steel 70/145/88/140/70/112 Adaptability | base_stats.rs:386-391 identical | none | high |
| venusaurite | MATCH | pokedex.ts:47 Grass/Poison 80/100/123/122/120/80 Thick Fat | base_stats.rs:134-139 identical | none | high |
| banettite | MATCH | pokedex.ts:6612 Ghost 64/165/75/93/83/75 Prankster | base_stats.rs:332-337 Ghost/-, identical | none | high |
| gardevoirite | MATCH | pokedex.ts:5512 Psychic/Fairy 68/85/65/165/135/100 Pixilate | base_stats.rs:278-283 identical | none | high |
| scizorite | MATCH | pokedex.ts:4400 Bug/Steel 70/150/140/65/100/75 Technician | base_stats.rs:236-241 identical | none | high |
| meganiumite | MATCH | pokedex.ts:3482 Grass/Fairy 80/92/115/143/115/80 Mega Sol | base_stats.rs:428-433 identical | none | high |
| smoothrock | MISSING | items.ts:5844 + conditions.ts:637 sandstorm 8 turns if holder set it | not in Items enum; UNKNOWNITEM; weather turns fixed 5 | holder-set sand lasts 5 not 8 | high |
| excadrite | MATCH | pokedex.ts:9927 Ground/Steel 110/165/100/65/65/103 Piercing Drill | base_stats.rs:536-541 identical | none | high |
| absolitez | DATA | pokedex.ts:6722 Absol-Mega-Z Dark/Ghost 65/154/60/75/60/151 ability Sharpness | base_stats.rs:644-649 same types/stats, ability MAGICBOUNCE | ability Sharpness vs Magic Bounce (SHARPNESS exists, abilities.rs:1902) | high |
| mawilite | MATCH | pokedex.ts:5815 Steel/Fairy 50/105/125/55/95/50 Huge Power | base_stats.rs:290-295 identical | none | high |
| slowbronite | MATCH | pokedex.ts:1963 Water/Psychic 95/75/180/130/80/30 Shell Armor | base_stats.rs:176-181 identical | none | high |
| whiteherb | PARTIAL | items.ts:7682 triggers on switch-in, after any move, after Mega, and at residual (order 29); consumes, zeroes all negative boosts | gi.rs:3819-3844 after_move_finish only, both sides, uses reset_negative_boosts (state.rs:849, incl. acc/eva), consumes | engine misses switch-in (e.g. Intimidate on entry; item_on_switch_in items.rs:807 has no WHITEHERB), after-Mega and residual triggers | medium |
| victreebelite | MATCH | pokedex.ts:1732 Grass/Poison 80/125/85/135/95/70 Innards Out | base_stats.rs:494-499 identical | none | high |
| scovillainite | MATCH | pokedex.ts:18329 Grass/Fire 65/138/85/138/85/75 Spicy Spray | base_stats.rs:476-481 identical | none | high |
| heracronite | MATCH | pokedex.ts:4438 Bug/Fighting 80/185/115/40/105/75 Skill Link | base_stats.rs:242-247 identical | none | high |
| shellbell | LOGIC | items.ts:5674 after move: heal floor(totalDamage/8) once, extra on top of any drain | items.rs:1272 sets choice.drain = Some(0.125) on EVERY move; applied per hit in gi.rs:1549 | (a) overwrites move's own drain (Drain Punch/Giga Drain/Horn Leech heal 1/8 not 1/2 + 1/8); (b) per-hit floor vs total/8; (c) sub damage counted | high |
| ejectbutton | MISSING | items.ts:1692 + mods/champions/items.ts:262 onAfterMoveSecondary: holder hit by non-Status move, switches out, item used | not in Items enum; UNKNOWNITEM; no behaviour | forced switch on being hit absent | high |
| spelltag | MATCH | items.ts:5926 Ghost x4915/4096 | items.rs:1290 Ghost BP x1.2 | none (float rounding only) | high |
| scopelens | MISSING | items.ts:5578 onModifyCritRatio +1 stage | not in Items enum; UNKNOWNITEM | crit stage +1 absent | high |
| eelektrossite | MATCH | pokedex.ts:11157 Electric 85/145/80/135/90/80 Eelevate | base_stats.rs:578-583 Electric/-, identical | none | high |
| dragalgite | MATCH | pokedex.ts:12999 Poison/Dragon 65/85/105/132/163/44 Regenerator | base_stats.rs:602-607 identical | none | high |
| galladite | MATCH | pokedex.ts:8661 Psychic/Fighting 68/165/95/65/115/110 Inner Focus | base_stats.rs:398-403 identical | none | high |
| mysticwater | MATCH | items.ts:4272 Water x4915/4096 | items.rs:1231 Water BP x1.2 | none (float rounding only) | high |

## Notes

- N1 (all mega stones): engine item_is_permanent (state.rs:617-708) returns true for every holder; SD (items.ts onTakeItem) only blocks removal when the holder is the matching species. Mismatched holders can lose the stone in SD, not in engine.
- N2 (all mega stones): Trick/Switcheroo (choice_effects.rs:1587) only checks the TARGET item removable. SD fails Trick when the USER holds its own mega stone. Engine lets the stone holder swap it away (then mega_evolve panics if it was queued).
- N3: mega form is applied by mega_evolve (gi.rs:3916-3986): swaps id, types, ability, stats; item stays. Matches SD formeChange (champions scripts.ts:56-121) except K8 and the ability errors above.
- N4: secondaries run once per move (gi.rs:3660 comment). SD per-hit items (Rocky Helmet, Air Balloon) trigger per hit; Life Orb and Shell Bell once per move.
- N5: Covert Cloak on defender skips all secondaries including user-side Life Orb recoil and Rocky Helmet (gi.rs:3794); not in Champions legal pool anyway.
- N6: Champions mods/champions/items.ts marks Choice Band/Specs, Assault Vest, Eviolite, Black Sludge, Flame/Toxic Orb, Weakness Policy etc. as Past (illegal); none of those are in this slice. Items in this slice all legal (isNonstandard not Past).
