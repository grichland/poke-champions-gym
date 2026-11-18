# Abilities audit 05 (CSV lines 214-266: friendguard .. rkssystem)

Slice: 03-coverage-abilities.csv rows 214-266. Rows audited: 53 of 53. NOT AUDITED: none.
Features: `champions` + `bss` (bss implies champions; champions-gated arms read as compiled, WEATHER_ABILITY_TURNS = 5, snow not hail). The `used` column is 0 for every row of this slice.
Showdown refs are all `data\abilities.ts` (base); the Champions mod `data\mods\champions\abilities.ts` overrides none of these 53 ids. Engine refs are `genx\abilities.rs` (abbr. A) unless stated; GI = `genx\generate_instructions.rs`, ST = `genx\state.rs`.

## Summary

Counts: MATCH 20, PLAIN 1, DATA 2, LOGIC 17, PARTIAL 5, MISSING 8, CORE 0, UNSURE 0.

Five most important findings:
1. Pickpocket is implemented backwards: the Pickpocket holder steals from the foe when it attacks (shares Magician's arm, A:750); in Showdown the holder steals the attacker's item when hit by a contact move.
2. Lingering Aroma overwrites the attacker's ability with Mummy (A:817-826), not Lingering Aroma.
3. Intrepid Sword boosts Attack on every switch-in (A:1505); Showdown only once per Pokemon per battle (`swordBoost`, pokemon.ts:501).
4. Protosynthesis / Quark Drive only react to switch-in and natural weather/terrain expiry (GI:2774-2852): weather or terrain changed while the holder is on the field is ignored, and Booster Energy activation is not tracked (`fromBooster`), so it is dropped when sun/terrain ends.
5. Own Tempo has no confusion immunity (ST:752 only checks Substitute); Guard Dog does not turn Intimidate into +1 Atk; Mind's Eye has no accuracy-drop immunity; Klutz, Merciless, Opportunist, Pickup are absent.
Also notable: Full Metal Body does not stop Sticky Web (GI:469-479 passes the switching side as both source and target); Immunity/Pastel Veil/Own Tempo ignore Mold Breaker; Galvanize/Normalize lack Showdown's type-change exceptions and Normalize lacks its 1.2x.

## Table

| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| friendguard | 0 | MATCH | abilities.ts:1534 onAnyModifyDamage x0.75 for allies of holder | A:442 only in mold-breaker list; no effect | doubles-only (no ally in singles), no effect either side | | high |
| fullmetalbody | 0 | LOGIC | :1559 onTryBoost removes foe-caused negative boosts; not breakable | ST:782-796 blocks opponent-caused drops; not in mold-breaker list (correct); GI:928-947 | Sticky Web drop (GI:469-479) passes switching side as source and target, so FMB (and Clear Body/White Smoke/Clear Amulet) does not block it; Showdown source is the foe so it is blocked | S | medium |
| galvanize | 0 | LOGIC | :1598 Normal->Electric except judgment/multiattack/naturalgift/revelationdance/technoblast/terrainpulse/weatherball (and Tera Blast tera); x4915/4096 | A:1933 any Normal move -> Electric, BP x1.2 | no exception list. modify_choice (GI:1690) runs first so Weather Ball in clear weather (still Normal) is wrongly converted; Revelation Dance / Judgment / Terrain Pulse etc. likewise | S | medium |
| gorillatactics | 0 | MATCH | :1655 Atk x1.5; locks first used move (onModifyMove), blocks others | A:2154 Physical BP x1.5; A:696 disables other moves via choice-lock helper | lock/boost equivalent. caveat: engine boosts BP so Body Press (uses Def, not Atk) is boosted wrongly | | medium |
| grasspelt | 0 | LOGIC | :1698 Def x1.5 in Grassy Terrain; breakable | A:2713 BP /1.5 for Physical moves in Grassy Terrain; breakable list ok | Psyshock/Psystrike/Secret Sword (Special, use Def) are reduced in Showdown, not in engine (category check) | S | medium |
| grimneigh | 0 | MATCH | :1717 onSourceAfterFaint SpA +length when KO by move | A:786 +1 SpA when damage>0 and target hp==0 | equal for singles | | high |
| guarddog | 0 | PARTIAL | :1728 blocks drag-out; Intimidate becomes +1 Atk; breakable | drag block GI:3669-3678 (ability != GUARDDOG); A:390 in breakable list; Intimidate arm A:1613 has no Guard Dog case | Intimidate lowers Atk instead of +1; drag block ignores Mold Breaker | M | high |
| gulpmissile | 0 | LOGIC | :1746 Surf (and Dive onTryMove) sets form by hp<=1/2; on damaging hit attacker takes baseMaxHp/4, then -1 Def (Gulping) or par (Gorging); formeChange back | A:618 form on Surf/Dive use; A:828 on hit: maxhp/4 damage, -1 Def or para via secondary; A:1060 revert on switch-out | form set when move is used, not when it hits (miss/protect); damage not blocked by Magic Guard (Showdown: ability damage is indirect) | S | medium |
| hadronengine | 0 | DATA | :1783 sets Electric Terrain on start; SpA x5461/4096 in Electric Terrain | A:1681 sets terrain 5 turns; A:1917 Special BP x1.33 | multiplier 1.33 vs 1.33325, applied to BP not stat (equivalent). terrain duration has no Terrain Extender support (none in src) | S | medium |
| honeygather | 0 | PLAIN | :1869 no battle effect | A:232 enum only | none | | high |
| hospitality | 0 | MATCH | :1875 heals adjacent allies 1/4 | A:338 enum only | doubles-only, no effect in singles | | high |
| hustle | 0 | LOGIC | :1909 Atk x1.5 stat; Physical accuracy x3277/4096 when accuracy is a number | A:2268 Physical BP x1.5, accuracy x0.80 | never-miss physical moves (Aerial Ace, Feint Attack, Shadow Punch, ...) are stored with accuracy 100 (choices.rs:212 etc.) so Hustle makes them miss 20%; Body Press boosted wrongly | S | medium |
| iceface | 0 | LOGIC | :1970 Eiscue blocks first Physical hit (damage 0, crit/effectiveness 0), becomes Noice; back to Eiscue on switch-in AND on weather change into hail/snow; not through Substitute; breakable | A:552 turns any Physical move into Status (no damage, secondaries kept) and busts form; A:1425 re-forms only on switch-in with hail/snow | no Substitute check; no immunity/miss check; multi-hit absorbs all hits (K2 aside) not just first; no re-form when snow starts mid-battle (e.g. foe Snow Warning) | M | medium |
| icescales | 0 | MATCH | :2027 Special damage x0.5, breakable | A:2817 Special BP x0.5; breakable list A:462 | equal | | high |
| immunity | 0 | LOGIC | :2097 blocks psn/tox, cures on update; breakable | GI:858 poison/toxic blocked; A:405 breakable list | immune_to_status (GI:776) never consults Mold Breaker/Teravolt/Turboblaze so Immunity still blocks; no cure if poisoned before gaining ability | M | medium |
| intrepidsword | 0 | LOGIC | :2214 +1 Atk only once per Pokemon per battle (swordBoost, set false only in constructor pokemon.ts:501) | A:1505 +1 Atk on every switch-in | repeats on re-entry | M | high |
| ironbarbs | 0 | LOGIC | :2225 contact: attacker takes baseMaxHp/8 (ability damage) | A:971 shared with Rough Skin: maxhp/8 when damage>0 and contact | no Magic Guard check on the attacker (ability damage is blocked by Magic Guard) | S | medium |
| klutz | 0 | MISSING | :2279 holder's item is ignored (ignoringItem) | A:125 enum only; no item gating anywhere | Klutz holder still uses Choice items, Leftovers, Life Orb, etc. | M | high |
| lightmetal | 0 | MATCH | :2335 weight trunc(hg/2); breakable | choice_effects.rs:44 trunc to 0.1 kg, min 0.1; A:444 breakable list | equal | | medium |
| lingeringaroma | 0 | LOGIC | :2388 contact: attacker's ability becomes Lingering Aroma (skip cantsuppress / already same) | A:817 shared with Mummy: sets attacker ability to MUMMY | wrong ability written; no cantsuppress (Multitype, Disguise, Ice Face, Gulp Missile, Schooling, ...) or already-same guard | S | high |
| liquidvoice | 0 | MATCH | :2417 sound moves become Water, no boost | A:2036 sound -> Water | equal (Mega/dynamax clause irrelevant) | | high |
| longreach | 0 | MATCH | :2429 removes contact flag | A:2014 contact=false | equal | | high |
| magician | 0 | LOGIC | :2478 after move: steals item from first hit target; not if source.switchFlag (U-turn/Volt Switch), Fling, status moves | A:750 steals when damage>0, foe item removable, own item empty | engine steals after a pivot move (Showdown does not) and nothing excludes Fling | S | medium |
| magnetpull | 0 | MATCH | :2517 traps adjacent Steel types (Ghost exempt, Shed Shell frees) | ST:1143 Steel trapped; Ghost/Shed Shell return false first (ST:1129) | equal; ignores Neutralizing Gas on holder | | medium |
| merciless | 0 | MISSING | :2573 crit ratio 5 (guaranteed crit) vs poisoned/toxic target | A enum only | no forced crit; engine crit modelling is itself K7 | M | high |
| mindseye | 0 | PARTIAL | :2620 immune to accuracy drops, ignores evasion, ignores Normal/Fighting immunity of Ghost; breakable | damage_calc.rs:348 Ghost immunity bypass for Normal/Fighting; A:393 breakable list | accuracy-drop immunity missing (ST:804 only checks KEENEYE). evasion is not modelled anywhere in GI:1229 (note) | S | high |
| minus | 0 | MATCH | :2644 SpA x1.5 with Plus/Minus ally | A enum only | doubles-only | | high |
| mistysurge | 0 | MATCH | :2681 sets Misty Terrain | A:1583 sets terrain 5 turns if not already | equal (no Terrain Extender item support, see Notes) | | medium |
| multitype | 0 | MISSING | :2773 Arceus type from plate (conditions.ts) | A enum only | no plate typing; not expected to matter unless Arceus is in the roster | M | low |
| myceliummight | 0 | PARTIAL | :2795 status moves -0.1 fractional priority and ignore target ability | A:376 ignores target ability for Status moves | no fractional priority: status move still wins speed order inside same priority | M | high |
| neuroforce | 0 | MATCH | :2895 damage x5120/4096 on super-effective | A:1958 BP x1.25 when effectiveness>1 | equal | | medium |
| neutralizinggas | 0 | PARTIAL | :2906 suppresses all non-cantsuppress abilities, ends Slow Start, restarts abilities on leave, Ability Shield | many fns return early if foe has NG (A:547,717,1056,1186,1408,1869,2327); GI:3167 | no restart of foe switch-in abilities when NG leaves; cantsuppress abilities (Gulp Missile, Ice Face, Disguise) wrongly suppressed; speed/stat/boost/status abilities (Quick Feet, Contrary, Clear Body, Immunity, Levitate) not suppressed; no Ability Shield; Slow Start not cleared | L | medium |
| normalize | 0 | LOGIC | :2987 all moves Normal except hiddenpower/judgment/multiattack/naturalgift/revelationdance/struggle/technoblast/terrainpulse/weatherball; x4915/4096 | A:2086 move_type=NORMAL, no boost | missing 1.2x BP; missing exception list | S | high |
| opportunist | 0 | MISSING | :3042 copies foe's stat boosts afterwards | A enum only | no copy | M | high |
| orichalcumpulse | 0 | DATA | :3089 sets sun; Atk x5461/4096 in sun/desolate (effectiveWeather) | A:1539 sets sun 5 turns; A:1924 Physical BP x1.33 in sun/harsh sun (Cloud Nine aware) | 1.33 vs 1.33325; Body Press boosted wrongly; no Heat Rock support | S | medium |
| overcoat | 0 | MATCH | :3109 immune to sand/hail damage and powder; breakable | GI:3009,3031 weather damage skipped; A:2889 powder remove_all_effects + accuracy 0; A:455 breakable list | equal (Grass type powder immunity at GI:1594) | | medium |
| owntempo | 0 | PARTIAL | :3145 confusion immune/cured, Intimidate immune; breakable | A:1615 Intimidate immunity; A:419 breakable list | confusion immunity missing (ST:752 checks Substitute only), so Swagger/Confuse Ray/Dynamic Punch confuse it | S | high |
| pastelveil | 0 | LOGIC | :3190 poison/toxic immune, cures own poison at start/update; breakable | GI:858 poison/toxic blocked; A:465 breakable list | Mold Breaker not honoured (immune_to_status has no attacker ability input); no cure of existing poison | M | medium |
| perishbody | 0 | LOGIC | :3228 contact: if attacker has no perishsong volatile, both get perishsong | A:1011 contact+damage: both sides without any perish volatile and not Soundproof get PERISH4 | engine excludes Soundproof (Showdown does not); Showdown does nothing if the attacker already has the count, engine still gives it to the holder | S | medium |
| pickpocket | 0 | LOGIC | :3240 when holder is hit by a contact move and holds nothing, it steals the attacker's item | A:750 Pickpocket in attacker arm: holder steals from foe when it attacks | direction reversed; holder-hit path absent | M | high |
| pickup | 0 | MISSING | :3263 residual: picks up item an adjacent Pokemon used this turn | A enum only | no pickup | M | high |
| plus | 0 | MATCH | :3318 SpA x1.5 with ally | A enum only | doubles-only | | high |
| poisonpuppeteer | 0 | MISSING | :3358 Pecharunt: poison/toxic by its moves also confuses target | A enum only | no effect | S | high |
| powerconstruct | 0 | MISSING | :3386 Zygarde to Complete form under 1/2 hp at residual | A enum only | no forme change | M | high |
| powerofalchemy | 0 | MATCH | :3401 copies fainted ally's ability | A enum only | doubles-only | | high |
| powerspot | 0 | MATCH | :3413 ally BP x5325/4096 | A enum only | doubles-only | | high |
| primordialsea | 0 | MATCH | :3451 heavy rain, cannot be replaced except by other strong weather, ends on holder leave (unless another holder) | A:1718 sets HEAVYRAIN (-1 turns); A:1121 cleared on switch-out | equal for singles; edge: other holder on the field, Neutralizing Gas end | | medium |
| prismarmor | 0 | MATCH | :3475 damage x0.75 when super-effective; not breakable | A:2694 BP x0.75 if effectiveness>1; not in breakable list | equal | | high |
| propellertail | 0 | MATCH | :3487 tracksTarget (ignores redirection) | A enum only | no redirection in singles | | high |
| protosynthesis | 0 | LOGIC | :3514 volatile when sunnyday (not desolate) on start and every weather change; Booster Energy volatile persists (fromBooster); best stat boost 1.3/1.5 | A:1441 on switch-in; GI:2774 removal only at natural sun expiry; damage_calc.rs:276,311 x1.3; GI:2575 spe x1.5 | no reaction to weather changes while active (new sun from foe does not grant, rain/desolate does not remove); Booster-activated state not remembered (removed when sun ends) | L | medium |
| quarkdrive | 0 | LOGIC | :3651 same, Electric Terrain | A:1453; GI:2814 removal only at natural terrain expiry | terrain change while active ignored (both directions); no fromBooster tracking | L | medium |
| quickfeet | 0 | MATCH | :3749 Spe x1.5 with any status; paralysis halving skipped | GI:2564 x1.5 with status; GI:2595 no paralysis drop | equal | | high |
| rkssystem | 0 | MISSING | :3900 Silvally type from memory (conditions.ts:875) | A enum only | no memory typing; only matters if Silvally is in the roster | M | low |

## Notes

- Systemic: Atk/SpA abilities are implemented as base-power multipliers (Hustle, Gorilla Tactics, Orichalcum, Hadron). Showdown boosts the stat, so Body Press (Def stat) and Psyshock (target Def) behave differently; Foul Play matches.
- Systemic: `immune_to_status` (GI:776), `immune_to_stats_lowered_by_opponent` (ST:782) and the drag block (GI:3669) never test `ignores_target_ability`, so Mold Breaker/Teravolt/Turboblaze/Sunsteel Strike do not bypass Immunity, Pastel Veil, Own Tempo, Clear Body, Hyper Cutter, Guard Dog (all breakable in Showdown). Only damage-time effects in `ability_modify_attack_against` respect it.
- Engine belongs to other slices: accuracy ignores target evasion boosts completely (GI:1229); Sticky Web is treated as self-inflicted (GI:469); Rough Skin/Aftermath/Innards Out do not check Magic Guard; Gluttony-like never-miss moves have accuracy 100 not "always hits".
- Terrain/weather durations from abilities are fixed at 5; no Terrain Extender / Heat Rock / Damp Rock / Smooth Rock / Icy Rock handling found in `src` (grep). Not verified whether the Champions item list includes them.
- `legal` is 1 for all 53 rows in the CSV; roster membership was not re-verified. Doubles-only abilities (Friend Guard, Hospitality, Minus, Plus, Power Spot, Power of Alchemy, Propeller Tail) are no-ops in singles, so MATCH.
- Mummy and Wandering Spirit share Lingering Aroma's arm (A:817); Wandering Spirit should swap abilities, not copy Mummy (other slice).
