# Second read: ITEMS slice b2 (CSV lines 64-125, ids pyroarite..passhoberry)

Summary: 62 rows audited. MATCH 44, LOGIC 7, PARTIAL 3, MISSING 8, DATA 0, CORE 0, UNSURE 0, PLAIN 0. First id pyroarite, last id passhoberry confirmed; 62 rows counted.

Abbreviations. SD = Showdown. SDi = `pokemon-showdown/data/items.ts`, SDp = `pokemon-showdown/data/pokedex.ts` (no champions pokedex override; `mods/champions/items.ts` only flips isNonstandard, none of this slice is flagged "Past"; Leek is made legal, Eject Button is the only handler override). ENG = `poke-engine-eval/poke-engine/src/genx/`. BS = ENG base_stats.rs. GI = ENG generate_instructions.rs. Engine has no feature gate that changes items under champions/bss (Cargo: `bss = ["champions"]`, no gen feature), so items.rs is compiled with the non-gen4/5/6 branches.

| id | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | confidence |
|---|---|---|---|---|---|
| pyroarite | MATCH | SDp:12524 Pyroar-Mega Fire/Normal 86/88/92/129/86/126 Fire Mane; SDi:4982 megaStone Pyroar, onTakeItem blocks removal from matching holder | BS:584 same types/stats/ability FIREMANE; state.rs:700 stone permanent; GI:3916 mega_evolve (forme, stats, ability, base_ability, types, ability on-switch-in) | none beyond K8 (speed order). Permanence is unconditional in engine (state.rs:617-708), SD only for matching holder: irrelevant for legal teams | high |
| kingsrock | MISSING | SDi:3221 onModifyMove (priority -1): non-Status moves get +10% flinch secondary unless a flinch secondary already exists | not in Items enum (items.rs:22-283), parses to UNKNOWNITEM; no behaviour | whole item missing: no 10% flinch on damaging moves | high |
| glalitite | MATCH | SDp:6779 Glalie-Mega Ice 80/120/80/120/80/100 Refrigerate | BS:344 same | none beyond K8 | high |
| heatrock | MISSING | SDi:2853 no handler on the item; Sunny Day duration 8 turns when holder sets it (moves.ts) | not in Items enum -> UNKNOWNITEM | weather extension absent | high |
| absolite | MATCH | SDp:6710 Absol-Mega Dark 65/150/60/115/60/115 Magic Bounce | BS:338 same | none beyond K8 | high |
| kangaskhanite | MATCH | SDp:2653 Kangaskhan-Mega Normal 105/125/100/60/100/100 Parental Bond | BS:188 same | none beyond K8 | high |
| beedrillite | MATCH | SDp:343 Beedrill-Mega Bug/Poison 65/150/40/15/80/145 Adaptability | BS:158 same | none beyond K8 | high |
| redcard | MISSING | SDi:5170 onAfterMoveSecondary: holder hit by a non-Status move from a different mon, both alive, attacker can switch -> item used up, attacker forced out (consumed even if attacker cannot be dragged) | not in Items enum -> UNKNOWNITEM | whole item missing (forced switch + consume) | high |
| sceptilite | MATCH | SDp:5060 Sceptile-Mega Grass/Dragon 70/110/75/145/85/145 Lightning Rod | BS:260 same | none beyond K8 | high |
| altarianite | MATCH | SDp:6301 Altaria-Mega Dragon/Fairy 75/110/110/110/105/80 Pixilate | BS:326 same | none beyond K8 | high |
| zoomlens | MISSING | SDi:7849 onSourceModifyAccuracy x4915/4096 (1.2) if target has not yet moved this turn (`!queue.willMove(target)`) | not in Items enum; engine accuracy (GI:1229) uses only move accuracy and attacker accuracy boost | item and its accuracy hook missing | high |
| feraligite | MATCH | SDp:3604 Feraligatr-Mega Water/Dragon 85/160/125/89/93/78 Dragonize | BS:434 same (ability Dragonize implemented abilities.rs:2202) | none beyond K8 | high |
| leek | MISSING | SDi:3334 +2 crit stage for Farfetch'd/Sirfetch'd (champions items.ts:514 makes it legal) | not in Items enum -> UNKNOWNITEM; engine has no crit-stage item hook | item missing | high |
| kebiaberry | LOGIC | SDi:3165 onSourceModifyDamage: Poison move, typeMod>0, not if target behind Substitute (non-bypass), eatItem then x0.5 on final damage; only on a damaging hit that connects; first hit only on multi-hit (item gone) | items.rs:542 -> damage_reduction_berry items.rs:304-325: if move_type==Poison and type eff >1: item consumed, choice.base_power /= 2. Called from item_before_move (GI:1687) | (1) runs in before_move, before sleep/par/freeze/flinch checks (GI:2306), accuracy (GI:2363), Protect (GI:1727) and Substitute: berry eaten on a miss, a full-para/asleep attacker, a protected or substituted target. (2) Status moves of the type also eat it. (3) Halves base power for all hits of a multi-hit move (SD halves only the first hit). (4) Type read before type-changing abilities/moves. (5) Unnerve/As One/Klutz/Embargo/Magic Room not modelled (Unnerve is enum token only). Right type, 0.5, SE-only, 1 use are correct | high |
| aerodactylite | MATCH | SDp:3192 Aerodactyl-Mega Rock/Flying 80/135/85/70/95/150 Tough Claws | BS:206 same | none beyond K8 | high |
| pinsirite | MATCH | SDp:2870 Pinsir-Mega Bug/Flying 65/155/120/65/90/105 Aerilate | BS:194 same | none beyond K8 | high |
| chesnaughtite | MATCH | SDp:12050 Chesnaught-Mega Grass/Fighting 88/137/172/74/115/44 Bulletproof | BS:446 same | none beyond K8 | high |
| pechaberry | LOGIC | SDi:4521 onUpdate: if psn/tox -> eatItem immediately, cureStatus (so cured the moment poisoned, before any residual damage) | items.rs:772-777 (before holder's move) and items.rs:927-932 (item_end_of_turn) -> status_cure_berry items.rs:418 | engine cures only at holder's next move or end-of-turn item step, which runs AFTER poison/toxic damage (GI:3128-3199 then GI:3212): holder takes one tick SD never applies; no cure on switch-in onto Toxic Spikes before damage; toxic counter not handled at inflict. Consume + cure itself correct | high |
| fairyfeather | MATCH | SDi:1939 onBasePower Fairy x4915/4096 | items.rs:1189 Fairy bp x1.2 | rounding only (BP chain vs float) | high |
| froslassite | MATCH | SDp:8723 Froslass-Mega Ice/Ghost 70/80/70/140/100/120 Snow Warning | BS:530 same | none beyond K8 | high |
| aggronite | MATCH | SDp:5872 Aggron-Mega Steel 70/140/230/60/80/50 Filter | BS:296 same | none beyond K8 | high |
| blackbelt | MATCH | SDi:514 onBasePower Fighting x4915/4096 | items.rs:1150 Fighting bp x1.2 | rounding only | high |
| sablenite | MATCH | SDp:5790 Sableye-Mega Dark/Ghost 50/85/125/85/115/20 Magic Bounce | BS:284 same | none beyond K8 | high |
| drampanite | MATCH | SDp:15100 Drampa-Mega Normal/Dragon 78/85/110/160/114/36 Berserk | BS:542 same (Berserk abilities.rs:956; Champions override of Berserk trigger is the abilities slice) | none for the item | high |
| sharpedonite | MATCH | SDp:6083 Sharpedo-Mega Water/Dark 70/140/70/110/65/105 Strong Jaw | BS:314 same | none beyond K8 | high |
| alakazite | MATCH | SDp:1618 Alakazam-Mega Psychic 55/50/65/175/105/150 Trace | BS:170 same | none beyond K8 | high |
| falinksite | MATCH | SDp:16862 Falinks-Mega Fighting 65/135/135/70/65/100 Defiant | BS:608 same | none beyond K8 | high |
| pidgeotite | MATCH | SDp:398 Pidgeot-Mega Normal/Flying 83/80/80/135/80/121 No Guard | BS:164 same | none beyond K8 | high |
| abomasite | MATCH | SDp:8427 Abomasnow-Mega Grass/Ice 90/132/105/132/105/30 Snow Warning | BS:392 same | none beyond K8 | high |
| chopleberry | LOGIC | SDi:1042 same pattern as Kebia, type Fighting | items.rs:472 damage_reduction_berry Fighting | same five differences as kebiaberry | high |
| emboarite | MATCH | SDp:9499 Emboar-Mega Fire/Fighting 110/148/75/110/110/75 Mold Breaker | BS:440 same | none beyond K8 | high |
| barbaracite | MATCH | SDp:12959 Barbaracle-Mega Rock/Fighting 72/140/130/64/106/88 Tough Claws | BS:596 same | none beyond K8 | high |
| lightball | LOGIC | SDi:3439 onModifyAtk and onModifySpA x2 stat for Pikachu (baseSpecies Pikachu, all formes) | items.rs:1541 doubles move base_power if is_pikachu_variant() | stat vs BP: wrong for Foul Play (uses target Atk) and Body Press (uses Def), which engine doubles; SD also doubles Atk for confusion self-hit, engine does not; rounding. Normal attacks equal | medium |
| audinite | MATCH | SDp:9957 Audino-Mega Normal/Fairy 103/60/126/80/126/50 Healer | BS:404 same | none beyond K8 | high |
| dragonfang | MATCH | SDi:1494 onBasePower Dragon x4915/4096 | items.rs:1175 Dragon bp x1.2 | rounding only | high |
| skarmorite | MATCH | SDp:4652 Skarmory-Mega Steel/Flying 65/140/110/40/100/110 Stalwart | BS:512 same; STALWART is token only (no behaviour) but is a doubles redirection ability, no effect in singles | none in singles | high |
| psychicseed | PARTIAL | SDi:4927 onStart (switch-in, priority -1) AND onTerrainChange: if Psychic Terrain -> useItem, +1 SpD | items.rs:879-899 item_on_switch_in (GI:571, GI:4032) only: +1 SpD, consumed only if boost applied | missing onTerrainChange trigger: holder already in play when Psychic Terrain starts (own/opp Psychic Surge, move) never uses the seed; also not run at Mega evolve (mega_evolve calls ability_on_switch_in only). Seed consumed even at +6 in SD (low confidence) | high |
| colburberry | LOGIC | SDi:1145 same pattern, type Dark | items.rs:518 damage_reduction_berry Dark | same five differences as kebiaberry | high |
| bigroot | MISSING | SDi:482 onTryHeal x5324/4096 (1.3) for drain, leechseed, ingrain, aquaring, strengthsap | not in Items enum -> UNKNOWNITEM; no drain boost | 1.3x on drain/Leech Seed heal missing | high |
| electricseed | PARTIAL | SDi:1811 onStart + onTerrainChange: Electric Terrain -> useItem, +1 Def | items.rs:816-836 switch-in only | same as psychicseed (no terrain-change trigger; Raichu-Mega-X Electric Surge on Mega evolve will not fire a held seed) | high |
| floettite | MATCH | SDp:12589 Floette-Mega Fairy 74/85/87/155/148/102 Fairy Aura; SDi:2201 megaStone Floette-Eternal | BS:488 FLOETTEETERNAL -> FLOETTEMEGA same | none beyond K8 | high |
| steelixite | MATCH | SDp:4318 Steelix-Mega Steel/Ground 75/125/230/55/95/30 Sand Force | BS:230 same | none beyond K8 | high |
| scraftinite | MATCH | SDp:10478 Scrafty-Mega Dark/Fighting 65/130/135/55/135/68 Intimidate | BS:572 same | none beyond K8 | high |
| chandelurite | MATCH | SDp:11241 Chandelure-Mega Ghost/Fire 60/75/110/175/110/90 Infiltrator | BS:554 same | none beyond K8 | high |
| medichamite | MATCH | SDp:5913 Medicham-Mega Fighting/Psychic 60/100/85/80/85/100 Pure Power | BS:302 same | none beyond K8 | high |
| ampharosite | MATCH | SDp:3911 Ampharos-Mega Electric/Dragon 90/95/105/165/110/45 Mold Breaker | BS:224 same | none beyond K8 | high |
| houndoominite | MATCH | SDp:4694 Houndoom-Mega Dark/Fire 75/90/90/140/90/115 Solar Power | BS:248 same | none beyond K8 | high |
| cameruptite | MATCH | SDp:6149 Camerupt-Mega Fire/Ground 70/120/100/145/105/20 Sheer Force | BS:320 same | none beyond K8 | high |
| miracleseed | MATCH | SDi:4153 onBasePower Grass x4915/4096 | items.rs:1295 Grass bp x1.2 | rounding only | high |
| widelens | MISSING | SDi:7737 onSourceModifyAccuracy x4505/4096 (1.1) on every move with numeric accuracy | GI:2401 only sets Population Bomb hit_count 9 vs 6; no accuracy change (GI:1229 ignores items) | the real effect (1.1x accuracy for all moves) absent; the Pop Bomb special case is an invented approximation (SD rolls accuracy per hit) | high |
| expertbelt | MATCH | SDi:1913 onModifyDamage x4915/4096 if typeMod>0 | items.rs:1180 bp x1.2 if type_effectiveness>1.0 | final-damage stage vs BP (rounding); ignores Delta Stream/Tera edge cases | medium |
| chestoberry | MATCH | SDi:889 onUpdate: if slp -> eat, cure (cured as soon as asleep, incl. Rest, Yawn) | GI:893-899 sleep inflicted -> berry consumed, status not applied; items.rs:769 and :924 cure leftover sleep before holder's move and at end of turn | same net result (sleep cured immediately, berry gone). Slightly different path for Rest/Yawn (cure delayed to next move check, but before the sleep check, so holder still acts) | medium |
| magnet | MATCH | SDi:3707 onBasePower Electric x4915/4096 | items.rs:1395 Electric bp x1.2 | rounding only | high |
| scolipite | MATCH | SDp:10156 Scolipede-Mega Bug/Poison 60/140/149/75/99/62 Shell Armor | BS:566 same | none beyond K8 | high |
| ironball | PARTIAL | SDi:3062 onModifySpe x0.5; isGrounded true (pokemon.ts:2174, beats Flying/Levitate/Air Balloon); onEffectiveness: Ground vs Flying holder becomes neutral (0) | GI:2590 speed x0.5 (correct); state.rs:721 is_grounded true | Ground-type move vs Flying-type or Levitate holder still treated immune: type_effectiveness_modifier (damage_calc.rs:104) and Levitate arm (abilities.rs:2630) never read IRONBALL. Hazards/terrain/Arena Trap grounding correct | high |
| hawluchanite | MATCH | SDp:13151 Hawlucha-Mega Fighting/Flying 78/137/100/74/93/118 No Guard | BS:506 same | none beyond K8 | high |
| malamarite | MATCH | SDp:12917 Malamar-Mega Dark/Psychic 86/102/88/98/120/88 Contrary | BS:590 same | none beyond K8 | high |
| cobaberry | LOGIC | SDi:1121 same pattern, type Flying | items.rs:510 damage_reduction_berry Flying | same five differences as kebiaberry | high |
| metalcoat | MATCH | SDi:3979 onBasePower Steel x4915/4096 | items.rs:1221 Steel bp x1.2 | rounding only | high |
| brightpowder | MISSING | SDi:671 onModifyAccuracy x3686/4096 (0.9) against holder | not in Items enum -> UNKNOWNITEM; no evasion-side item hook | item missing | high |
| sharpbeak | MATCH | SDi:5630 onBasePower Flying x4915/4096 | items.rs:1267 Flying bp x1.2 | rounding only | high |
| passhoberry | LOGIC | SDi:4467 same pattern, type Water | items.rs:558 damage_reduction_berry Water | same five differences as kebiaberry | high |

## Notes

- Damage-reduction berries (Kebia, Chople, Coba, Colbur, Passho, same code for all 18): item_before_move is called at the top of before_move (GI:1687), before generate_instructions_from_existing_status_conditions (GI:2306), check_move_hit_or_miss (GI:2363), Protect handling (GI:1727) and any Substitute check. The defender's berry is therefore consumed and the move halved when the attacker is fully paralysed, asleep, frozen, misses, or is blocked by Protect/Substitute, and also by Status moves of the berry type (e.g. Will-O-Wisp vs a Grass holder of Occa). SD only consumes inside onSourceModifyDamage on a damaging hit that reaches the target.
- Same hook also fires the attacker's own Lum/Sitrus/Petaya/status-cure berries before the attacker's sleep/par check, and the Choice item lock-in before accuracy.
- Berries in general in this engine ignore Unnerve, As One, Klutz, Embargo, Magic Room (tokens exist, no behaviour).
- End-of-turn order: engine runs status damage (GI:3117) before item_end_of_turn (GI:3212). In SD Leftovers/Black Sludge residual order 5 runs before burn/poison (order 9-10), and Pecha/Rawst/etc. never let the tick happen at all. Belongs to the Leftovers/Black Sludge rows but affects Pecha here.
- Mega stones: all 35 stones in this slice have an engine arm with identical form id, types, ability and base stats to SD pokedex (checked field by field). Stones are unremovable in engine for any holder (state.rs:617-708); SD blocks removal only when the holder's species matches. Legal teams cannot hit the difference. One mega per team enforced (state.rs:1097). K8 applies to Mega-turn speed.
- Skarmory-Mega ability Stalwart and Healer (Audino-Mega) have no engine behaviour; Stalwart has no singles effect, Healer is ally-only. Champions overrides of Berserk/Healer/Regenerator are the abilities slice.
- Past-flag check: none of the 62 ids is "Past" in champions items.ts, so every one is legal in the format; Leek is explicitly re-enabled (champions items.ts:514).
- Type-boost items (1.2) are applied to base power in the engine and to the BP chain in SD, same stage; ok within K1-style rounding.
