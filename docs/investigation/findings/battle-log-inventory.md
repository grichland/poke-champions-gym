# Battle-log inventory against LIVE.md section 4

Read-only labelling, 17 Nov 2026. Sources: `pcsa/assets/game/battle_text.json` (3616 entries), `pcsa/src/pcsa/battle/log_patterns.py`, `log_rows.py`, `battle_text.py`. Contract: `contracts/LIVE.md` v0.1.

## What the source file is

- `battle_text.json` is Pokemon Showdown's text corpus (default.ts, moves.ts, abilities.ts, items.ts), not a capture of the phone game's log. Showdown wording differs from what the old translator reads from the phone in several places (for example the file says "was hurt by poison!", the translator matches "was hurt by its poisoning!"). Four Champions overrides are coded in `battle_text.py` (CHAMPIONS_TEXT) and are not in the file; they are listed in section 6.
- Of 3616 entries, 2910 are move, ability and item descriptions (`shortDesc` 1846, `desc` 1064): description text, not log messages. 706 entries are message templates. BT ids are the 1-based position in the file (BT-0001 to BT-3616); the key shown is `section:effect.key` (section d default, m moves, a abilities, i items).
- Whether the phone game really shows a given Showdown message is not decidable from the file. Every template is labelled by what it would carry if shown. Nothing here was checked against recorded phone battles.
- Column "old translator": the old code was run read-only on each template with placeholders filled by sample names (`parse_row`, no category label). `yes` means the row becomes an event other than `unknown`; the old event kind is given. It is a test of the pattern code, not of recorded rows.

## Counts

Category codes: A exact fit to a section 4 kind; P partial fit (the kind fits but the text lacks a required field or names it differently); B NO-KIND; C NOT-BATTLE; D1 AMBIGUOUS, the same text gives different kinds; D2 AMBIGUOUS, same kind but the effect, cause or move field differs between mechanics; E text for a mechanic Champions singles does not have (Z-Move, Dynamax, G-Max, Terastallization, Primal, Ultra Burst, ally or doubles text), added by me, unverified against the game and kept out of the other categories.

| category | messages (706) | recognised by old translator | description entries (2910) |
|---|---|---|---|
| (a) exact | 347 | 157 | 0 |
| (a~) partial | 55 | 10 | 0 |
| (b) NO-KIND | 90 | 18 | 0 |
| (c) NOT-BATTLE | 76 | 0 | 2910 |
| (d) AMBIGUOUS, kind differs | 15 | 2 | 0 |
| (d) AMBIGUOUS, field differs | 67 | 26 | 0 |
| (e) mechanic absent | 56 | 3 | 0 |
| total | 706 | 216 | 2910 |

(c) in the messages column: 76 rows = celebrate.activate, default.fullName, default.opposingParty, default.opposingPokemon, default.opposingTeam, default.party, default.pokemon, default.startBattle, default.team, deltastream.weatherName, desolateland.weatherName, hail.weatherName, happyhour.activate, makeitrain.activate, payday.activate, primordialsea.weatherName, raindance.weatherName, sandstorm.weatherName, snowscape.weatherName, sunnyday.weatherName; plus 56 `ui.*` client prompts.

## List (b): NO-KIND (battle information that fits no section 4 kind)

Nearest note is in the last column. Rows marked S5 also contradict section 5.

| id | key | message | what it carries |
|---|---|---|---|
| BT-0021 | d:default.boost0 | {POKEMON}'s {STAT} won't go any higher! | stat already at +6/-6; a boost by 0 is not "a stage changed" |
| BT-0040 | d:default.combine | The two moves have become one! It's a combined move! | pledge combination announcement |
| BT-0042 | d:default.crit | A critical hit! | S5: a critical hit is shown, contract says it never appears |
| BT-0043 | d:default.critSpread | A critical hit on {POKEMON}! | S5: a critical hit is shown, contract says it never appears |
| BT-0055 | d:default.extremelyEffective | It's extremely effective! | effectiveness message |
| BT-0056 | d:default.extremelyEffectiveSpread | It's extremely effective on {POKEMON}! | effectiveness message |
| BT-0064 | d:default.immune | It doesn't affect {POKEMON}... | "doesn't affect X": immunity, effectiveness family |
| BT-0067 | d:default.invertBoost | All stat changes on {POKEMON} were inverted! | stage signs flipped (Topsy-Turvy) |
| BT-0072 | d:default.missNoPokemon | {SOURCE}'s attack missed! | S5: names the attacker on a miss |
| BT-0073 | d:default.mostlyIneffective | It's mostly ineffective... | effectiveness message |
| BT-0074 | d:default.mostlyIneffectiveSpread | It's mostly ineffective on {POKEMON}. | effectiveness message |
| BT-0077 | d:default.ohko | It's a one-hit KO! | one-hit KO |
| BT-0085 | d:default.resisted | It's not very effective... | effectiveness message |
| BT-0086 | d:default.resistedSpread | It's not very effective on {POKEMON}. | effectiveness message |
| BT-0091 | d:default.superEffective | It's super effective! | effectiveness message |
| BT-0092 | d:default.superEffectiveSpread | It's super effective on {POKEMON}! | effectiveness message |
| BT-0094 | d:default.swapBoost | {POKEMON} switched stat changes with its target! | stage swap with target (Heart/Guard/Power Swap) |
| BT-0096 | d:default.swapDefensiveBoost | {POKEMON} switched all changes to its Defense and Sp. Def with its target! | stage swap with target (Heart/Guard/Power Swap) |
| BT-0097 | d:default.swapOffensiveBoost | {POKEMON} switched all changes to its Attack and Sp. Atk with its target! | stage swap with target (Heart/Guard/Power Swap) |
| BT-0100 | d:default.switchOut | {TRAINER} withdrew {NICKNAME}! | withdrawal row; implied by the next enter only |
| BT-0101 | d:default.switchOutOwn | {NICKNAME}, come back! | withdrawal row; implied by the next enter only |
| BT-0113 | d:default.unboost0 | {POKEMON}'s {STAT} won't go any lower! | stat already at +6/-6; a boost by 0 is not "a stage changed" |
| BT-0124 | d:deltastream.activate | The mysterious strong winds weakened the attack! | extreme weather blocks or weakens a move |
| BT-0125 | d:deltastream.block | The mysterious strong winds blow on regardless! | extreme weather blocks or weakens a move |
| BT-0129 | d:desolateland.block | The extremely harsh sunlight was not lessened at all! | extreme weather blocks or weakens a move |
| BT-0130 | d:desolateland.blockMove | The Water-type attack evaporated in the harsh sunlight! | extreme weather blocks or weakens a move |
| BT-0139 | d:electricterrain.block | {POKEMON} is protected by the Electric Terrain! | terrain blocks a status or priority move |
| BT-0152 | d:gravity.activate | {POKEMON} fell from the sky due to the gravity! | grounded by Gravity |
| BT-0159 | d:hail.upkeep | (The hail is crashing down.) | weather continues; contract weather = began or ended |
| BT-0165 | d:mistyterrain.block | {POKEMON} surrounds itself with a protective mist! | terrain blocks a status or priority move |
| BT-0170 | d:nopp.cant | {POKEMON} used {MOVE}! /   But there was no PP left for the move! | two lines: move used + "no PP left" |
| BT-0176 | d:primordialsea.block | There is no relief from this heavy rain! | extreme weather blocks or weakens a move |
| BT-0177 | d:primordialsea.blockMove | The Fire-type attack fizzled out in the heavy rain! | extreme weather blocks or weakens a move |
| BT-0186 | d:psychicterrain.block | {POKEMON} is protected by the Psychic Terrain! | terrain blocks a status or priority move |
| BT-0191 | d:raindance.upkeep | (Rain continues to fall.) | weather continues; contract weather = began or ended |
| BT-0198 | d:sandstorm.upkeep | (The sandstorm is raging.) | weather continues; contract weather = began or ended |
| BT-0208 | d:snowscape.upkeep | (The snow is blowing about!) | weather continues; contract weather = began or ended |
| BT-0210 | d:struggle.activate | {POKEMON} has no moves left! | no usable move left |
| BT-0213 | d:sunnyday.upkeep | (The sunlight is strong.) | weather continues; contract weather = began or ended |
| BT-0280 | d:unboost.fail | {POKEMON}'s {STAT} {INFLECT:STAT:s=was:p=were} not lowered! | stat drop prevented |
| BT-0281 | d:unboost.failNoStat | {POKEMON}'s stats were not lowered! | stat drop prevented |
| BT-0342 | m:aromatherapy.activate | A soothing aroma wafted through the area! | team status cure, nobody named |
| BT-0399 | m:beatup.activate | {NAME}'s attack! | names the ally whose hit is used |
| BT-0528 | m:chillyreception.prepare | {POKEMON} is preparing to tell a chillingly bad joke! | joke announced before the move |
| BT-0592 | m:courtchange.activate | {POKEMON} swapped the battle effects affecting each side of the field! | side effects moved or cleared; which not named |
| BT-0599 | m:craftyshield.block | Crafty Shield protected {POKEMON}! | S5-adjacent: a Protect-like block is shown |
| BT-0602 | m:craftyshield.start | Crafty Shield protected {TEAM}! | S5-adjacent: a Protect-like block is shown |
| BT-0666 | m:doomdesire.activate | {TARGET} took the Doom Desire attack! | delayed attack queued / lands |
| BT-0669 | m:doomdesire.start | {POKEMON} chose Doom Desire as its destiny! | delayed attack queued / lands |
| BT-0735 | m:eeriespell.activate | It reduced the PP of {TARGET}'s {MOVE} by {NUMBER}! | S5: PP of a named move is shown |
| BT-0804 | m:feint.activate | {TARGET} fell for the feint! | S5-adjacent: a protection was broken |
| BT-0810 | m:ficklebeam.activate | {POKEMON} is going all out for this attack! | power doubled |
| BT-0933 | m:futuresight.activate | {TARGET} took the Future Sight attack! | delayed attack queued / lands |
| BT-0936 | m:futuresight.start | {POKEMON} foresaw an attack! | delayed attack queued / lands |
| BT-1064 | m:grudge.activate | {POKEMON}'s {MOVE} lost all of its PP due to the grudge! | S5: PP of a named move is shown |
| BT-1070 | m:guardsplit.activate | {POKEMON} shared its guard with the target! | stat swap or average |
| BT-1104 | m:healbell.activate | A bell chimed! | team status cure, nobody named |
| BT-1215 | m:ingrain.block | {POKEMON} is anchored in place with its roots! | cannot be forced out |
| BT-1345 | m:magnitude.activate | Magnitude {NUMBER}! | magnitude level |
| BT-1355 | m:matblock.block | {MOVE} was blocked by the kicked-up mat! | S5-adjacent: a Protect-like block is shown |
| BT-1358 | m:matblock.start | {POKEMON} intends to flip up a mat and block incoming attacks! | S5-adjacent: Protect-like block |
| BT-1437 | m:mimic.start | {POKEMON} learned {MOVE}! | moveset changed |
| BT-1455 | m:mist.block | {POKEMON} is protected by the mist! | status blocked by side effect |
| BT-1549 | m:painsplit.activate | The battlers shared their pain! | both HP rows follow |
| BT-1569 | m:perishsong.start | All Pokémon that heard the song will faint in three turns! | no Pokemon named |
| BT-1616 | m:powder.activate | When the flame touched the powder on the Pokémon, it exploded! | no Pokemon named |
| BT-1625 | m:powershift.start | {POKEMON} swapped its offensive stats with its defensive stats! | stat swap or average |
| BT-1626 | m:powersplit.activate | {POKEMON} shared its power with the target! | stat swap or average |
| BT-1633 | m:powertrick.start | {POKEMON} switched its Attack and Defense! | stat swap or average |
| BT-1682 | m:pursuit.activate | ({TARGET} is being withdrawn...) | move order / interception text |
| BT-1687 | m:quash.activate | {TARGET}'s move was postponed! | move order / interception text |
| BT-1692 | m:quickguard.block | Quick Guard protected {POKEMON}! | S5-adjacent: a Protect-like block is shown |
| BT-1695 | m:quickguard.start | Quick Guard protected {TEAM}! | S5-adjacent: a Protect-like block is shown |
| BT-1794 | m:safeguard.block | {POKEMON} is protected by Safeguard! | status blocked by side effect |
| BT-1894 | m:sketch.activate | {POKEMON} sketched {MOVE}! | moveset changed |
| BT-1945 | m:snatch.activate | {POKEMON} snatched {TARGET}'s move! | move order / interception text |
| BT-1976 | m:spectralthief.clearBoost | {SOURCE} stole the target's boosted stats! | boosts stolen |
| BT-1979 | m:speedswap.activate | {POKEMON} switched Speed with its target! | stat swap |
| BT-2002 | m:spite.activate | It reduced the PP of {TARGET}'s {MOVE} by {NUMBER}! | S5: PP of a named move is shown |
| BT-2151 | m:teatime.activate | It's teatime! Everyone dug in to their Berries! | berries eaten by everyone |
| BT-2206 | m:tidyup.activate | Tidying up complete! | side effects moved or cleared; which not named |
| BT-2260 | m:uproar.block | But the uproar kept {POKEMON} awake! | sleep blocked by uproar |
| BT-2323 | m:wideguard.block | Wide Guard protected {POKEMON}! | S5-adjacent: a Protect-like block is shown |
| BT-2326 | m:wideguard.start | Wide Guard protected {TEAM}! | S5-adjacent: a Protect-like block is shown |
| BT-2525 | a:forewarn.activate | {TARGET}'s {MOVE} was revealed! | S5: an unused move of the opponent is named |
| BT-2526 | a:forewarn.activateNoTarget | {POKEMON}'s Forewarn alerted it to {MOVE}! | S5: an unused move of the opponent is named |
| BT-2585 | a:illusion.end | {POKEMON}'s illusion wore off! | illusion broken |
| BT-2678 | a:neutralizinggas.end | The effects of the neutralizing gas wore off! | ability suppression ended |
| BT-2704 | a:persistent.activate | {POKEMON} extends {MOVE} by 2 turns! | S5-adjacent: turns added |
| BT-2724 | a:powerconstruct.activate | You sense the presence of many! | no Pokemon named |

## List (d): AMBIGUOUS (same text, several mechanics)

D1: the kinds differ, so the event cannot be built from the text. D2: the kind is the same but the effect, cause or move field cannot be filled from the text alone. Some rows are also NO-KIND (stated).

| group | message | ids and mechanics | outcome |
|---|---|---|---|
| D-01 | {POKEMON} kept going and crashed! | BT-0014 d:crash.damage; BT-0379 m:axekick.damage; BT-1138 m:highjumpkick.damage; BT-1240 m:jumpkick.damage; BT-2090 m:supercellslam.damage | D2: hp side,mon,from=crash/move (same text as the four move-specific crash rows) |
| D-02 | ({EFFECT} activated!) | BT-0016 d:default.activate | D1: volatile_hit / ability / once / item, depending on {EFFECT} (generic "({EFFECT} activated!)") |
| D-03 | ({POKEMON} was hurt!) | BT-0044 d:default.damage | D2: hp side,mon,from=? ("({POKEMON} was hurt!)": cause not in text) |
| D-04 | ({EFFECT} ended!) | BT-0053 d:default.endFieldEffect | D1: field end / weather end, depending on {EFFECT} (generic) |
| D-05 | ({EFFECT} started on {POKEMON}!) | BT-0087 d:default.start | D1: volatile / once / status-like, depending on {EFFECT} (generic "({EFFECT} started on X!)") |
| D-06 | ({EFFECT} started!) | BT-0089 d:default.startFieldEffect | D1: field start / weather start, depending on {EFFECT} (generic) |
| D-07 | {POKEMON} transformed! | BT-0106 d:default.transform; BT-2914 a:terashift.transform | D1: transform vs form (target not named; same text as a:terashift.transform) |
| D-08 | {POKEMON} was hurt by poison! | BT-0182 d:psn.damage; BT-0216 d:tox.damage | D2: hp side,mon,from=psn/tox ("hurt by poison" shared by poison and bad poison) |
| D-09 | {POKEMON} was cured of its poisoning! | BT-0183 d:psn.end; BT-0217 d:tox.end | D2: cure side,mon,status=psn/tox (shared text for poison and bad poison) |
| D-10 | {POKEMON}'s {ITEM} cured its poison! | BT-0184 d:psn.endFromItem; BT-0218 d:tox.endFromItem | D2: cure side,mon,status=psn/tox (shared text for poison and bad poison) |
| D-11 | {POKEMON} is storing energy! | BT-0412 m:bide.activate; BT-0416 m:bide.start | D1: volatile vs volatile_hit (same text as m:bide.start) |
| D-12 | {POKEMON}'s attack continues! | BT-0418 m:bind.move; BT-0537 m:clamp.move; BT-0835 m:firespin.move; BT-2348 m:wrap.move | D2: (none) ("attack continues" for four trapping moves; also no kind) |
| D-13 | {POKEMON} is getting pumped! | BT-0694 m:dragoncheer.start; BT-0888 m:focusenergy.start; BT-0975 m:gmaxchistrike.start | D2: volatile side,mon,effect=? ("is getting pumped!" shared) |
| D-14 | {POKEMON} endured the hit! | BT-0765 m:endure.activate; BT-2875 a:sturdy.activate | D1: volatile_hit (Endure) / ability (Sturdy) ("endured the hit!" shared) |
| D-15 | {POKEMON} went back to {TRAINER}! | BT-0870 m:flipturn.switchOut; BT-1559 m:partingshot.switchOut; BT-2269 m:uturn.switchOut; BT-2288 m:voltswitch.switchOut | D2: (none) ("went back to TRAINER": withdrawal of four moves; also no kind) |
| D-16 | {POKEMON} became the center of attention! | BT-0897 m:followme.start; BT-1704 m:ragepowder.start; BT-2016 m:spotlight.start | D2: volatile / once effect=? ("became the center of attention!" shared (Follow Me, Rage Powder, Spotlight)) |
| D-17 | {POKEMON} was identified! | BT-0903 m:foresight.start; BT-1448 m:miracleeye.start | D2: volatile effect=? ("was identified!" shared) |
| D-18 | It broke through {TARGET}'s protection! | BT-1167 m:hyperspacefury.activate; BT-1171 m:hyperspacehole.activate; BT-1574 m:phantomforce.activate; BT-1847 m:shadowforce.activate | D2: (none) ("broke through TARGET's protection!" shared; S5-adjacent (Protect shown)) |
| D-19 | {SOURCE} took aim at {POKEMON}! | BT-1301 m:lockon.start; BT-1443 m:mindreader.start | D2: volatile effect=? ("took aim at" shared) |
| D-20 | {POKEMON} bounced the {MOVE} back! | BT-1328 m:magiccoat.move; BT-2632 a:magicbounce.move; BT-2774 a:rebound.move | D1: move bounced: once/ability/(none) ("bounced the MOVE back!": Magic Coat, Magic Bounce, Rebound) |
| D-21 | {POKEMON} protected itself! | BT-1371 m:maxguard.activate; BT-1645 m:protect.block; BT-1648 m:protect.start | D2: (none) (S5: "protected itself!" is a Protect/Max Guard activation; shared text) |
| D-22 | ({POKEMON} cut its own HP to power up its move!) | BT-1438 m:mindblown.damage; BT-2029 m:steelbeam.damage | D2: hp side,mon,from=move (same text for two moves) |
| D-23 | {POKEMON} vanished instantly! | BT-1576 m:phantomforce.prepare; BT-1849 m:shadowforce.prepare | D2: prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) |
| D-24 | {POKEMON} found one {ITEM}! | BT-1724 m:recycle.addItem; BT-2709 a:pickup.addItem | D2: item side,mon,item,what=received,from=? ("found one ITEM!" shared; Recycle vs Pickup) |
| D-25 | {POKEMON} set a shell trap! | BT-1872 m:shelltrap.prepare; BT-1874 m:shelltrap.start | D1: once vs prepare (same text as m:shelltrap.start) |
| D-26 | {POKEMON} swapped Abilities with its target! | BT-1897 m:skillswap.activate; BT-2955 a:wanderingspirit.activate | D2: (none) ("swapped Abilities with its target!" shared; no ability named) |
| D-27 | {POKEMON} absorbed light! | BT-1960 m:solarbeam.prepare; BT-1963 m:solarblade.prepare | D2: prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) |
| D-28 | {POKEMON} was hurt! | BT-1993 m:spikyshield.damage; BT-2369 a:aftermath.damage; BT-2482 a:dryskin.damage; BT-2592 a:innardsout.damage; BT-2601 a:ironbarbs.damage; BT-2792 a:roughskin.damage | D2: hp side,mon,from=? ("was hurt!" shared) |
| D-29 | {POKEMON} switched items with its target! | BT-2112 m:switcheroo.activate; BT-2232 m:trick.activate | D2: (none) ("switched items with its target!" shared; no item named) |
| D-30 | The effects of the weather disappeared. | BT-2373 a:airlock.start; BT-2428 a:cloudnine.start | D2: ability side,mon,ability=? ("effects of the weather disappeared" shared) |
| D-31 | {SOURCE} cannot use {MOVE}! | BT-2386 a:armortail.block; BT-2451 a:damp.block; BT-2460 a:dazzling.block; BT-2762 a:queenlymajesty.block | D2: ability side,mon,ability=? ("SOURCE cannot use MOVE!" shared by four abilities) |
| D-32 | Being hit by {MOVE} charged {POKEMON} with power! | BT-2494 a:electromorphosis.start; BT-2973 a:windpower.start | D2: volatile effect=charge + ability ("Being hit by MOVE charged X with power!" shared) |
| D-33 | {POKEMON} took the attack! | BT-2619 a:lightningrod.activate; BT-2870 a:stormdrain.activate | D2: ability side,mon,ability=? ("took the attack!" shared) |
| D-34 | {POKEMON}'s {STAT} was heightened! | BT-2749 a:protosynthesis.start; BT-2761 a:quarkdrive.start | D2: volatile side,mon,effect=protosynthesis/quarkdrive,detail=stat (S5: reveals the opponent's highest stat; text shared) |

Texts that are identical but resolve to one event (not ambiguous): "All stat changes were eliminated!" (BT-0037, BT-1093); "But it does not have enough HP left to make a substitute!" (BT-1861, BT-2080); "But it failed to affect {POKEMON}!" (BT-0581, BT-1110); "But nothing happened!" (BT-2007, BT-2153); "But {POKEMON} can't use the move!" (BT-0623, BT-1169); "Coins were scattered everywhere!" (BT-1348, BT-1562); "It reduced the PP of {TARGET}'s {MOVE} by {NUMBER}!" (BT-0735, BT-2002); "{POKEMON} already has a substitute!" (BT-1859, BT-2077); "{POKEMON} is already poisoned!" (BT-0181, BT-0215); "{POKEMON} is waiting for {TARGET}'s move..." (BT-0826, BT-1046, BT-2296); "{POKEMON} shattered {TEAM}'s protections!" (BT-0472, BT-1706); "{POKEMON}'s HP was restored by the Z-Power!" (BT-1412, BT-1557); "{POKEMON}'s {ITEM} lessened the damage it took!" (BT-0018, BT-0051); "{SOURCE} stole and ate its target's {ITEM}!" (BT-0484, BT-1591); "{SOURCE}'s {ABILITY} was taken over!" (BT-2728, BT-2776).


## Step 4: contract against the messages

### 4a. Section 4 kinds and fields that no message in the file can produce

The file is Showdown text, so "cannot be produced" means no template carries the value. Where the old translator got it from a row that is not in the file, that is stated (see section 5).

| item | why | ids |
|---|---|---|
| `refused` (`order`, `shown`) | No battle message. The game refusing a choice is a screen or UI event. The only refusal-like texts are Showdown client prompts (labelled NOT-BATTLE). | BT-0237, BT-0266 |
| `end.result` = `forfeit`, `unknown` | No template. The two end texts name a trainer, not own/opp, and the tie text gives a result (`tie`) that the contract does not list. Forfeit is not a log line in the file. | BT-0120, BT-0105 |
| `turn` (`n`) | A template exists ("== Turn N ==") but the old translator never matches it (no pattern, not in TEMPLATE_EVENTS); old turns come from the row grouping (`RawRow.turn`), i.e. from the capture, not from a message. | BT-0108 |
| `enter` fields `hp`, `maxhp`, `pct` | The send-out messages name the Pokemon only. HP values are in no template. | BT-0098, BT-0099 |
| `hp` fields `hp`, `maxhp`, `pct` | No template carries an HP number. Only `damagePercentage` gives a percent lost (not left), in a parenthesised Showdown form the old translator does not match. The old translator reads HP from category rows "HP Lost" / "HP Restored" of the form `X's HP: a -> b`, which are not in the file. So an `hp` event is a cause message (`from`) plus a separate HP row. | BT-0048 |
| `revive` fields `hp`/`pct` | The Revival Blessing message names the Pokemon and gives no HP. | BT-1751 |
| `ability.changed_from` | No change-ability template names the previous ability (new ability only, or the source's). The old translator got old and new from an "Ability Changed" category row (`X: old -> new`), not in the file. | BT-0036, BT-2931, BT-2668, BT-2623, BT-1774 |
| `weather.from`, `weather.of` | No weather start or end template names the move, ability or Pokemon that caused it. Drought, Drizzle, Sand Stream and Snow Warning have no message in the file (descriptions only); the cause would be a separate ability row such as [X's Ability]. | BT-0015 |
| `move.called_by` | Only two templates carry a calling move: Metronome and Nature Power. No template for Sleep Talk, Copycat, Mirror Move, Assist, Me First or Instruct. | BT-1430, BT-1499 |
| `hits.side`, `hits.mon` | Never produced, and not needed: the message names no one (as the contract already says). | BT-0063 |
| `transform.into` (side and mon) | The Transform text names the target species and no side; the generic text names no target. | BT-2229, BT-0106 |
| `mega` (`form` + `stone`) from one message | Two messages: the reacting row names mon and stone (no form), the evolved row names the Mega species (no stone). The old translator joins them. | BT-0068, BT-0069, BT-0070, BT-0107 |
| `types.types` (the resulting list) | Templates name one type (`typeChange`, `typeAdd`) or none (Burn Up, Double Shock, Roost, Conversion, Reflect Type, Mimicry). No template lists the resulting types. | BT-0110, BT-0109, BT-0111, BT-0502, BT-1783, BT-0572, BT-2646, BT-1733 |
| `boost.by` exact for set-to-maximum effects | Belly Drum and Anger Point say "maximized/maxed its Attack", no amount. | BT-0406, BT-2376 |
| `fail`'s `side`/`mon` | "But it failed!" names no one. | BT-0057 |
| `volatile_hit` (`side`,`mon`) for the confusion self-hit | "It hurt itself in its confusion!" names no one. | BT-0009 |

Section 4 kinds with at least one producing template (so they stand): `enter`, `move`, `prepare`, `cant`, `miss`, `fail` (wording), `hits`, `hp` (cause only), `faint`, `status`, `cure`, `volatile`, `volatile_end`, `volatile_hit`, `boost`, `boost_reset` (all, negative, copy), `ability`, `item` (shown, used, removed, received), `mega`, `form`, `transform`, `types`, `weather`, `field`, `side`, `once`, `revive`, `end` (win and tie only).

### 4b. Section 5 ("never appears") against the messages

| section 5 item | messages that show it | ids |
|---|---|---|
| a critical hit | "A critical hit!" and the spread form | BT-0042, BT-0043 |
| a Protect activation | Protect and Max Guard "protected itself!"; Crafty, Quick, Wide Guard and Mat Block block and start texts; Spiky Shield damage; the four "broke through the protection" texts and Feint (they reveal that a protection was up) | BT-1645, BT-1648, BT-1371, BT-0599, BT-0602, BT-1692, BT-1695, BT-2323, BT-2326, BT-1355, BT-1358, BT-1993, BT-0804, BT-1167, BT-1171, BT-1574, BT-1847 |
| who attacked on a miss | "X's attack missed!" names the attacker (contract `miss` names the avoider) | BT-0072 |
| the opponent's PP | Eerie Spell and Spite name a move and the PP removed; Grudge says all PP lost; the "no PP left" line; Leppa and Mystery Berry say PP restored for a named move (own side too, see LIVE.md 6.3) | BT-0735, BT-2002, BT-1064, BT-0170, BT-3246, BT-3312 |
| how many turns of a lock, trap or effect remain | Perish Song count and "faint in three turns" (Perish Song, Perish Body); Persistent "extends by 2 turns". LIVE.md section 4 itself lists a Perish count under `volatile`, which contradicts this sentence | BT-1566, BT-1569, BT-2703, BT-2704 |
| an unrevealed opponent item | Frisk names the item it finds. Every `{ITEM}` message and every items-section message names an item (list below) | BT-2530, BT-2531 |
| an unrevealed opponent ability | Every abilities-section message and every `{ABILITY}` message names or implies an ability (list below) | see list below |
| opponent's unused moves | Forewarn names the strongest move of the target; Mimic and Sketch name the copied move (a move already used, so not strictly unused) | BT-2525, BT-2526, BT-1437, BT-1894 |
| opponent's stats | Protosynthesis and Quark Drive name the boosted stat (the opponent's highest stat) | BT-2749, BT-2761 |

No message shows: the damage roll, a Choice Scarf (no template), the opponent's nature or points, the order of simultaneous effects, sleep turns.

Messages that name an item (`{ITEM}` or an items-section key), 69: BT-0004, BT-0006, BT-0011, BT-0017, BT-0018, BT-0019, BT-0023, BT-0026, BT-0028, BT-0045, BT-0047, BT-0050, BT-0051, BT-0068, BT-0069, BT-0084, BT-0102, BT-0115, BT-0117, BT-0118, BT-0119, BT-0146, BT-0174, BT-0184, BT-0203, BT-0218, BT-0220, BT-0361, BT-0364, BT-0411, BT-0484, BT-0582, BT-0866, BT-0889, BT-1205, BT-1252, BT-1591, BT-1609, BT-1724, BT-2530, BT-2531, BT-2557, BT-2709, BT-2896, BT-2985, BT-2997, BT-2999, BT-3028, BT-3064, BT-3078, BT-3117, BT-3119, BT-3156, BT-3158, BT-3242, BT-3246, BT-3250, BT-3305, BT-3312, BT-3354, BT-3362, BT-3375, BT-3389, BT-3400, BT-3412, BT-3427, BT-3585, BT-3587, BT-3607.

Messages that name or imply an ability (`{ABILITY}` or an abilities-section key), 107: BT-0015, BT-0036, BT-1774, BT-2369, BT-2373, BT-2376, BT-2381, BT-2386, BT-2389, BT-2393, BT-2398, BT-2400, BT-2406, BT-2409, BT-2411, BT-2428, BT-2433, BT-2434, BT-2451, BT-2458, BT-2460, BT-2471, BT-2474, BT-2482, BT-2494, BT-2495, BT-2497, BT-2499, BT-2501, BT-2507, BT-2515, BT-2518, BT-2525, BT-2526, BT-2530, BT-2531, BT-2554, BT-2556, BT-2557, BT-2567, BT-2585, BT-2592, BT-2601, BT-2619, BT-2623, BT-2626, BT-2632, BT-2646, BT-2658, BT-2668, BT-2673, BT-2678, BT-2680, BT-2688, BT-2690, BT-2703, BT-2704, BT-2709, BT-2724, BT-2727, BT-2728, BT-2737, BT-2744, BT-2745, BT-2747, BT-2749, BT-2756, BT-2757, BT-2759, BT-2761, BT-2762, BT-2765, BT-2774, BT-2776, BT-2792, BT-2808, BT-2809, BT-2829, BT-2830, BT-2834, BT-2836, BT-2856, BT-2857, BT-2867, BT-2870, BT-2875, BT-2878, BT-2882, BT-2883, BT-2889, BT-2895, BT-2896, BT-2902, BT-2907, BT-2910, BT-2914, BT-2917, BT-2931, BT-2936, BT-2940, BT-2947, BT-2950, BT-2955, BT-2973, BT-2981, BT-2982, BT-2983.

These do not break the contract if "unrevealed" means "not yet shown by the game": each such message is the game revealing it, and `item` and `ability` events exist for that. They are listed so the controller test ("nothing in section 5 ever appears") is applied to real ids.

### 4c. Messages whose kind needs a Pokemon the text does not name

Rule 2 (unknown is absent) applies, or the controller takes the Pokemon from the previous row: BT-0057, BT-0009, BT-0037, BT-1093, BT-0106, BT-0065, BT-0076, BT-0077, BT-2471, BT-2856, BT-2857, BT-2981, BT-2982, BT-1569, BT-1616, BT-2724, BT-2206.

## 5. Phone-log rows the old translator reads that are not in the file

Taken from `log_rows.py`, `log_patterns.py`, `battle_text.py`. They show what the phone game writes and the file does not hold. Not matched against any recorded battle.

- Category-labelled rows: `HP Lost` / `HP Restored` (`X's HP: a -> b`, own as points, opponent as percent), `Held Item` (`X's Item`), `Ability` (`X's Ability`), `Ability Changed` (`X: old -> new`), `Move` and `Move +N Priority` labels, `Fainted`, `Mega Evolution`. These carry the HP, item and ability values section 4 needs; no template in the file does.
- Switches: `Go! X!`, `<trainer> sent out X!`, `X, come back!`, `<trainer> withdrew X!`, `X was dragged out!`.
- Mega: `X's <stone> is reacting to <trainer>'s Omni Ring!`, `X has Mega Evolved into Y!`.
- Champions wording that replaces Showdown's (CHAMPIONS_TEXT): `{POKEMON} couldn't move because it's paralyzed!`, `{POKEMON} slept and restored its HP!`, `Toxic spikes were scattered on the ground all around {TEAM}!`, `{POKEMON} harvested {ITEM}!`; item names with a quantity ("a serving of Leftovers").
- Weather and terrain wording outside the file's: `The sandstorm is raging.`, `Rain continues to fall.`, `Snow continues to fall.`, `The sunlight is strong.` (upkeep without the parentheses of the file); terrain starts and ends written without the file's final punctuation.
- Hazard and screen wording: `Pointed stones float in the air on <side>!`, `The pointed stones disappeared from <side>!`, `A tailwind started blowing on <side>!`, `<Screen> made <side> stronger...`, `<side>'s <Screen> wore off!`, `The sticky web has disappeared...`.
- Subject sentences (SUBJECT_PATTERNS): `was hurt by its poisoning!`, `is paralyzed! It may be unable to move!`, `is fast asleep`, `protected itself!`, `had its HP restored.`, `lost some of its HP!` (Life Orb), `hung on using its X!`, `endured the hit!`, `went back to ...!`, `is already drowsy`, `has no moves left`, `was hurt!`, `must do an encore!`, `ended its encore!`.
- Item rows: `X knocked off Y's Z!`, `X stole and ate its target's Z!`, `X floats in the air with its Z!`, `Y's Air Balloon popped!`, `X can act faster than normal, thanks to its Z!`, `The <item> Gem strengthened ...!`.
- Other: `X transformed into the T type!` (Protean/Libero), `X's disguise was busted!`, `X has been afflicted with an infestation by ...!`, `It traced Y's Z!`, `The Pokemon was hit N times!`, `It's mostly ineffective...`, `It's extremely effective!`, `It doesn't affect X...`, `It hurt itself in its confusion!`.
- Screen overlays captured as rows, not log messages: `until battle ends`, `move time` (battle timer caption).

## 6. Full table: the 706 message templates

Columns: id, key (section:effect.key), message, category, kind and fields (note), old translator (yes with the old kind; no with the reason).

| id | key | message | cat | kind and fields | old |
|---|---|---|---|---|---|
| BT-0001 | d:brn.alreadyStarted | {POKEMON} is already burned! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: brn (alreadyStarted) |
| BT-0002 | d:brn.damage | {POKEMON} was hurt by its burn! | A | hp side,mon,from=brn | yes: cause |
| BT-0003 | d:brn.end | {POKEMON}'s burn was healed! | A | cure side,mon,status | no: unsupported mechanic: brn (end) |
| BT-0004 | d:brn.endFromItem | {POKEMON}'s {ITEM} healed its burn! | A | cure side,mon,status,from=item + item what=used | no: unsupported mechanic: brn (endFromItem) |
| BT-0005 | d:brn.start | {POKEMON} was burned! | A | status side,mon,status | yes: status |
| BT-0006 | d:brn.startFromItem | {POKEMON} was burned by the {ITEM}! | A | status side,mon,status,from=item + item what=shown (Flame Orb / Toxic Orb) | no: unsupported mechanic: brn (startFromItem) |
| BT-0007 | d:confusion.activate | {POKEMON} is confused! | A | volatile_hit side,mon,effect=confusion | yes: volatile_active |
| BT-0008 | d:confusion.alreadyStarted | {POKEMON} is already confused! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: confusion (alreadyStarted) |
| BT-0009 | d:confusion.damage | It hurt itself in its confusion! | P | volatile_hit effect=confusion (names no Pokemon; mon is the one from the preceding "is confused!" row) | yes: confusion_damage |
| BT-0010 | d:confusion.end | {POKEMON} snapped out of its confusion! | A | volatile_end side,mon,effect=confusion | yes: volatile_end |
| BT-0011 | d:confusion.endFromItem | {POKEMON}'s {ITEM} snapped it out of its confusion! | A | volatile_end effect=confusion + item what=used | no: unsupported mechanic: confusion (endFromItem) |
| BT-0012 | d:confusion.start | {POKEMON} became confused! | A | volatile side,mon,effect=confusion | yes: volatile |
| BT-0013 | d:confusion.startFromFatigue | {POKEMON} became confused due to fatigue! | A | volatile side,mon,effect=confusion | yes: volatile |
| BT-0014 | d:crash.damage | {POKEMON} kept going and crashed! | D2 | hp side,mon,from=crash/move (same text as the four move-specific crash rows) | yes: cause |
| BT-0015 | d:default.abilityActivation | [{POKEMON}'s {ABILITY}] | A | ability side,mon,ability | no: no_pattern |
| BT-0016 | d:default.activate | ({EFFECT} activated!) | D1 | volatile_hit / ability / once / item, depending on {EFFECT} (generic "({EFFECT} activated!)") | no: unsupported mechanic: default (activate) |
| BT-0017 | d:default.activateItem | ({POKEMON} used its {ITEM}!) | A | item side,mon,item,what=used | no: no_pattern |
| BT-0018 | d:default.activateWeaken | {POKEMON}'s {ITEM} lessened the damage it took! | A | item side,mon,item,what=used (resist berry; same text, two keys) | yes: item |
| BT-0019 | d:default.addItem | {POKEMON} obtained {ITEM}. | A | item side,mon,item,what=received | yes: item_reveal |
| BT-0020 | d:default.boost | {POKEMON}'s {STAT} rose! | A | boost side,mon,stat,by=+1/+2/+3 | yes: boost |
| BT-0021 | d:default.boost0 | {POKEMON}'s {STAT} won't go any higher! | B | (none) (stat already at +6/-6; a boost by 0 is not "a stage changed") | yes: boost |
| BT-0022 | d:default.boost2 | {POKEMON}'s {STAT} rose sharply! | A | boost side,mon,stat,by=+1/+2/+3 | yes: boost |
| BT-0023 | d:default.boost2FromItem | The {ITEM} sharply raised {POKEMON}'s {STAT}! | A | boost side,mon,stat,by + item what=used | no: unsupported mechanic: default (boost2FromItem) |
| BT-0024 | d:default.boost2FromZEffect | {POKEMON} boosted its {STAT} sharply using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (boost2FromZEffect |
| BT-0025 | d:default.boost3 | {POKEMON}'s {STAT} rose drastically! | A | boost side,mon,stat,by=+1/+2/+3 | yes: boost |
| BT-0026 | d:default.boost3FromItem | The {ITEM} drastically raised {POKEMON}'s {STAT}! | A | boost side,mon,stat,by + item what=used | no: unsupported mechanic: default (boost3FromItem) |
| BT-0027 | d:default.boost3FromZEffect | {POKEMON} boosted its {STAT} drastically using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (boost3FromZEffect |
| BT-0028 | d:default.boostFromItem | The {ITEM} raised {POKEMON}'s {STAT}! | A | boost side,mon,stat,by + item what=used | no: unsupported mechanic: default (boostFromItem) |
| BT-0029 | d:default.boostFromZEffect | {POKEMON} boosted its {STAT} using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (boostFromZEffect) |
| BT-0030 | d:default.boostMultipleFromZEffect | {POKEMON} boosted its stats using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (boostMultipleFrom |
| BT-0031 | d:default.canDynamax | {TRAINER} can dynamax now! | E | Dynamax text | no: unsupported mechanic: default (canDynamax) |
| BT-0032 | d:default.canDynamaxOwn | Dynamax Energy gathered around {TRAINER}! | E | Dynamax text | no: unsupported mechanic: default (canDynamaxOwn) |
| BT-0033 | d:default.cant | {POKEMON} cannot use {MOVE}! | P | cant side,mon,reason=<move id> (wording names the move, not the cause) | no: unsupported mechanic: Armor Tail (block) or Damp |
| BT-0034 | d:default.cantNoMove | {POKEMON} can't move! | A | cant side,mon (reason omitted) | no: unsupported mechanic: default (cantNoMove) |
| BT-0035 | d:default.center | Automatic center! | E | doubles/triples auto-choice | no: unsupported mechanic: default (center) |
| BT-0036 | d:default.changeAbility | {POKEMON} acquired {ABILITY}! | A | ability side,mon,ability | no: unsupported mechanic: default (changeAbility) |
| BT-0037 | d:default.clearAllBoost | All stat changes were eliminated! | A | boost_reset what=all (no Pokemon named in text (Haze); same text as m:haze.activate) | yes: clearallboost |
| BT-0038 | d:default.clearBoost | {POKEMON}'s stat changes were removed! | A | boost_reset side,mon,what=all | no: unsupported mechanic: default (clearBoost) |
| BT-0039 | d:default.clearBoostFromZEffect | {POKEMON} returned its decreased stats to normal using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (clearBoostFromZEf |
| BT-0040 | d:default.combine | The two moves have become one! It's a combined move! | B | (none) (pledge combination announcement) | no: unsupported mechanic: default (combine) |
| BT-0041 | d:default.copyBoost | {POKEMON} copied {TARGET}'s stat changes! | A | boost_reset side,mon,what=copy,of | yes: copyboost |
| BT-0042 | d:default.crit | A critical hit! | B | (none) (S5: a critical hit is shown, contract says it never appears) | no: unsupported mechanic: default (crit) |
| BT-0043 | d:default.critSpread | A critical hit on {POKEMON}! | B | (none) (S5: a critical hit is shown, contract says it never appears) | no: unsupported mechanic: default (critSpread) |
| BT-0044 | d:default.damage | ({POKEMON} was hurt!) | D2 | hp side,mon,from=? ("({POKEMON} was hurt!)": cause not in text) | no: no_pattern |
| BT-0045 | d:default.damageFromItem | {POKEMON} was hurt by its {ITEM}! | A | hp side,mon,from=item (+ item what=shown) | no: unsupported mechanic: default (damageFromItem) |
| BT-0046 | d:default.damageFromPartialTrapping | {POKEMON} is hurt by {MOVE}! | A | hp side,mon,from=<trapping move> | yes: cause |
| BT-0047 | d:default.damageFromPokemon | {POKEMON} was hurt by {SOURCE}'s {ITEM}! | A | hp side,mon,from=item,of=source | no: unsupported mechanic: default (damageFromPokemon |
| BT-0048 | d:default.damagePercentage | ({POKEMON} lost {PERCENTAGE}% of its health!) | P | hp side,mon (gives the percent lost, not the percent left) | no: unsupported mechanic: default (damagePercentage) |
| BT-0049 | d:default.drag | {FULLNAME} was dragged out! | A | enter side,mon,forced=true | yes: drag |
| BT-0050 | d:default.eatItem | ({POKEMON} ate its {ITEM}!) | A | item side,mon,item,what=used | no: no_pattern |
| BT-0051 | d:default.eatItemWeaken | {POKEMON}'s {ITEM} lessened the damage it took! | A | item side,mon,item,what=used (resist berry; same text, two keys) | yes: item |
| BT-0052 | d:default.end | {POKEMON} was freed from {EFFECT}! | A | volatile_end side,mon,effect={EFFECT} (generic ("freed from X")) | yes: volatile_end |
| BT-0053 | d:default.endFieldEffect | ({EFFECT} ended!) | D1 | field end / weather end, depending on {EFFECT} (generic) | no: no_pattern |
| BT-0054 | d:default.endTeamEffect | ({EFFECT} ended on {TEAM}!) | A | side side,effect={EFFECT},what=end | no: no_pattern |
| BT-0055 | d:default.extremelyEffective | It's extremely effective! | B | (none) (effectiveness message) | yes: effectiveness |
| BT-0056 | d:default.extremelyEffectiveSpread | It's extremely effective on {POKEMON}! | B | (none) (effectiveness message) | no: unsupported mechanic: default (extremelyEffectiv |
| BT-0057 | d:default.fail | But it failed! | P | fail side,mon (names no Pokemon) | yes: fail |
| BT-0058 | d:default.faint | {POKEMON} fainted! | A | faint side,mon | yes: faint |
| BT-0059 | d:default.fullName | {NICKNAME} ({SPECIES}) | C | name format fragment, not a message | no: no_pattern |
| BT-0060 | d:default.heal | {POKEMON} had its HP restored. | A | hp side,mon (restored) (cause not in text) | yes: trailing |
| BT-0061 | d:default.healFromEffect | {POKEMON} restored HP using its {EFFECT}! | A | hp side,mon,from={EFFECT} | no: unsupported mechanic: default (healFromEffect) |
| BT-0062 | d:default.healFromZEffect | {POKEMON} restored its HP using its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (healFromZEffect) |
| BT-0063 | d:default.hitCount | The Pokémon was hit {NUMBER} {INFLECT:NUMBER:s=time:p=times}! | A | hits n | yes: hitcount |
| BT-0064 | d:default.immune | It doesn't affect {POKEMON}... | B | (none) ("doesn't affect X": immunity, effectiveness family) | yes: effectiveness |
| BT-0065 | d:default.immuneNoPokemon | But it had no effect! | P | fail (names no Pokemon; wording differs) | no: unsupported mechanic: default (immuneNoPokemon) |
| BT-0066 | d:default.immuneOHKO | {POKEMON} is unaffected! | P | fail side,mon (OHKO immunity) | no: unsupported mechanic: default (immuneOHKO) |
| BT-0067 | d:default.invertBoost | All stat changes on {POKEMON} were inverted! | B | (none) (stage signs flipped (Topsy-Turvy)) | no: unsupported mechanic: default (invertBoost) |
| BT-0068 | d:default.mega | {POKEMON}'s {ITEM} is reacting to the Key Stone! | P | item what=shown side,mon,stone; mega needs form from the next row ("is reacting to the Key Stone") | yes: mega_reacting |
| BT-0069 | d:default.megaGen6 | {POKEMON}'s {ITEM} is reacting to {TRAINER}'s Mega Bracelet! | P | same as above (wording of Gen 6 / no item) | yes: mega_reacting |
| BT-0070 | d:default.megaNoItem | {POKEMON} is reacting to {TRAINER}'s Key Stone! | P | same as above (wording of Gen 6 / no item) | no: unsupported mechanic: default (megaNoItem) |
| BT-0071 | d:default.miss | {POKEMON} avoided the attack! | A | miss side,mon | yes: miss |
| BT-0072 | d:default.missNoPokemon | {SOURCE}'s attack missed! | B | (none) (S5: names the attacker on a miss) | no: unsupported mechanic: default (missNoPokemon) |
| BT-0073 | d:default.mostlyIneffective | It's mostly ineffective... | B | (none) (effectiveness message) | yes: effectiveness |
| BT-0074 | d:default.mostlyIneffectiveSpread | It's mostly ineffective on {POKEMON}. | B | (none) (effectiveness message) | no: unsupported mechanic: default (mostlyIneffective |
| BT-0075 | d:default.move | {POKEMON} used {MOVE}! | A | move side,mon,move | yes: move |
| BT-0076 | d:default.noTarget | But there was no target... | P | fail (names no Pokemon; wording differs) | no: unsupported mechanic: default (noTarget) |
| BT-0077 | d:default.ohko | It's a one-hit KO! | B | (none) (one-hit KO) | yes: trailing |
| BT-0078 | d:default.opposingParty | the opposing Pokémon | C | name format fragment, not a message | no: no_pattern |
| BT-0079 | d:default.opposingPokemon | the opposing {NICKNAME} | C | name format fragment, not a message | no: no_pattern |
| BT-0080 | d:default.opposingTeam | the opposing team | C | name format fragment, not a message | no: no_pattern |
| BT-0081 | d:default.party | your ally Pokémon | C | name format fragment, not a message | no: no_pattern |
| BT-0082 | d:default.pokemon | {NICKNAME} | C | name format fragment, not a message | no: no_pattern |
| BT-0083 | d:default.primal | {POKEMON}'s Primal Reversion! It reverted to its primal state! | E | Primal Reversion | no: unsupported mechanic: default (primal) |
| BT-0084 | d:default.removeItem | {POKEMON} lost its {ITEM}! | A | item side,mon,item,what=removed | no: no_pattern |
| BT-0085 | d:default.resisted | It's not very effective... | B | (none) (effectiveness message) | yes: effectiveness |
| BT-0086 | d:default.resistedSpread | It's not very effective on {POKEMON}. | B | (none) (effectiveness message) | no: unsupported mechanic: default (resistedSpread) |
| BT-0087 | d:default.start | ({EFFECT} started on {POKEMON}!) | D1 | volatile / once / status-like, depending on {EFFECT} (generic "({EFFECT} started on X!)") | no: unsupported mechanic: default (start) |
| BT-0088 | d:default.startBattle | Battle started between {TRAINER1} and {TRAINER2}! | C | battle start banner, no state | no: unsupported mechanic: default (startBattle) |
| BT-0089 | d:default.startFieldEffect | ({EFFECT} started!) | D1 | field start / weather start, depending on {EFFECT} (generic) | no: no_pattern |
| BT-0090 | d:default.startTeamEffect | ({EFFECT} started on {TEAM}!) | A | side side,effect={EFFECT},what=start | no: unsupported mechanic: default (start) or default |
| BT-0091 | d:default.superEffective | It's super effective! | B | (none) (effectiveness message) | yes: effectiveness |
| BT-0092 | d:default.superEffectiveSpread | It's super effective on {POKEMON}! | B | (none) (effectiveness message) | no: unsupported mechanic: default (superEffectiveSpr |
| BT-0093 | d:default.swap | {POKEMON} and {TARGET} switched places! | E | ally / triples position swap | no: unsupported mechanic: default (swap) |
| BT-0094 | d:default.swapBoost | {POKEMON} switched stat changes with its target! | B | (none) (stage swap with target (Heart/Guard/Power Swap)) | no: unsupported mechanic: default (swapBoost) |
| BT-0095 | d:default.swapCenter | {POKEMON} moved to the center! | E | ally / triples position swap | no: unsupported mechanic: default (swapCenter) |
| BT-0096 | d:default.swapDefensiveBoost | {POKEMON} switched all changes to its Defense and Sp. Def with its target! | B | (none) (stage swap with target (Heart/Guard/Power Swap)) | no: unsupported mechanic: default (swapDefensiveBoos |
| BT-0097 | d:default.swapOffensiveBoost | {POKEMON} switched all changes to its Attack and Sp. Atk with its target! | B | (none) (stage swap with target (Heart/Guard/Power Swap)) | no: unsupported mechanic: default (swapOffensiveBoos |
| BT-0098 | d:default.switchIn | {TRAINER} sent out {FULLNAME}! | A | enter side=opp,mon | yes: send_out |
| BT-0099 | d:default.switchInOwn | Go! {FULLNAME}! | A | enter side=own,mon | yes: send_out |
| BT-0100 | d:default.switchOut | {TRAINER} withdrew {NICKNAME}! | B | (none) (withdrawal row; implied by the next enter only) | yes: withdraw |
| BT-0101 | d:default.switchOutOwn | {NICKNAME}, come back! | B | (none) (withdrawal row; implied by the next enter only) | yes: withdraw |
| BT-0102 | d:default.takeItem | {POKEMON} stole {SOURCE}'s {ITEM}! | A | item side,mon,item,what=received,of=source | no: no_pattern |
| BT-0103 | d:default.team | your team | C | name format fragment, not a message | no: no_pattern |
| BT-0104 | d:default.terastallize | ({POKEMON} has Terastallized into the {TYPE}-type!) | E | Terastallization | no: unsupported mechanic: default (terastallize) |
| BT-0105 | d:default.tieBattle | Tie between {TRAINER1} and {TRAINER2}! | A | end result (win/loss/tie by trainer name) (tie not in end.result list) | no: unsupported mechanic: default (tieBattle) |
| BT-0106 | d:default.transform | {POKEMON} transformed! | D1 | transform vs form (target not named; same text as a:terashift.transform) | no: unsupported mechanic: Tera Shift (transform) or  |
| BT-0107 | d:default.transformMega | {POKEMON} has Mega Evolved into Mega {SPECIES}! | A | mega side,mon,form | yes: mega |
| BT-0108 | d:default.turn | == Turn {NUMBER} == | A | turn n | no: no_pattern |
| BT-0109 | d:default.typeAdd | {TYPE} type was added to {POKEMON}! | P | types side,mon (only the added type is named) | no: unsupported mechanic: default (typeAdd) |
| BT-0110 | d:default.typeChange | {POKEMON} transformed into the {TYPE} type! | A | types side,mon,types | yes: typechange |
| BT-0111 | d:default.typeChangeFromEffect | {POKEMON}'s {EFFECT} made it the {TYPE} type! | A | types side,mon,types | no: unsupported mechanic: default (typeChangeFromEff |
| BT-0112 | d:default.unboost | {POKEMON}'s {STAT} fell! | A | boost side,mon,stat,by=-1/-2/-3 | yes: boost |
| BT-0113 | d:default.unboost0 | {POKEMON}'s {STAT} won't go any lower! | B | (none) (stat already at +6/-6; a boost by 0 is not "a stage changed") | yes: boost |
| BT-0114 | d:default.unboost2 | {POKEMON}'s {STAT} harshly fell! | A | boost side,mon,stat,by=-1/-2/-3 | yes: boost |
| BT-0115 | d:default.unboost2FromItem | The {ITEM} harshly lowered {POKEMON}'s {STAT}! | A | boost side,mon,stat,by(neg) + item what=used | no: unsupported mechanic: default (unboost2FromItem) |
| BT-0116 | d:default.unboost3 | {POKEMON}'s {STAT} severely fell! | A | boost side,mon,stat,by=-1/-2/-3 | yes: boost |
| BT-0117 | d:default.unboost3FromItem | The {ITEM} drastically lowered {POKEMON}'s {STAT}! | A | boost side,mon,stat,by(neg) + item what=used | no: unsupported mechanic: default (unboost3FromItem) |
| BT-0118 | d:default.unboostFromItem | The {ITEM} lowered {POKEMON}'s {STAT}! | A | boost side,mon,stat,by(neg) + item what=used | no: unsupported mechanic: default (unboostFromItem) |
| BT-0119 | d:default.useGem | {ITEM} strengthened {MOVE}'s power! | A | item side,mon,item,what=used | no: unsupported mechanic: default (useGem) |
| BT-0120 | d:default.winBattle | {TRAINER} won the battle! | A | end result (win/loss/tie by trainer name) (tie not in end.result list) | no: unsupported mechanic: default (winBattle) |
| BT-0121 | d:default.zBroken | {POKEMON} couldn't fully protect itself and got hurt! | E | Z-Move text | no: unsupported mechanic: default (zBroken) |
| BT-0122 | d:default.zEffect | {POKEMON} unleashes its full-force Z-Move! | E | Z-Move text | no: unsupported mechanic: default (zEffect) |
| BT-0123 | d:default.zPower | {POKEMON} surrounded itself with its Z-Power! | E | Z-Move text | no: unsupported mechanic: default (zPower) |
| BT-0124 | d:deltastream.activate | The mysterious strong winds weakened the attack! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: deltastream (activate) |
| BT-0125 | d:deltastream.block | The mysterious strong winds blow on regardless! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: deltastream (block) |
| BT-0126 | d:deltastream.end | The mysterious strong winds have dissipated! | A | weather weather=none | no: unsupported mechanic: deltastream (end) |
| BT-0127 | d:deltastream.start | Mysterious strong winds are protecting Flying-type Pokémon! | A | weather weather | no: unsupported mechanic: deltastream (start) |
| BT-0128 | d:deltastream.weatherName | Strong Winds | C | display name of a weather | no: unsupported mechanic: deltastream (weatherName) |
| BT-0129 | d:desolateland.block | The extremely harsh sunlight was not lessened at all! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: desolateland (block) |
| BT-0130 | d:desolateland.blockMove | The Water-type attack evaporated in the harsh sunlight! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: desolateland (blockMove) |
| BT-0131 | d:desolateland.end | The extremely harsh sunlight faded. | A | weather weather=none | no: unsupported mechanic: desolateland (end) |
| BT-0132 | d:desolateland.start | The sunlight turned extremely harsh! | A | weather weather | no: unsupported mechanic: desolateland (start) |
| BT-0133 | d:desolateland.weatherName | Intense Sun | C | display name of a weather | no: unsupported mechanic: desolateland (weatherName) |
| BT-0134 | d:drain.heal | {SOURCE} had its energy drained! | A | hp side,mon (restored),from=drain,of (names the drained Pokemon) | yes: cause |
| BT-0135 | d:dynamax.block | The move was blocked by the power of Dynamax! | E | Dynamax | no: unsupported mechanic: dynamax (block) |
| BT-0136 | d:dynamax.end | ({POKEMON} returned to normal!) | E | Dynamax | no: unsupported mechanic: dynamax (end) |
| BT-0137 | d:dynamax.fail | {POKEMON} shook its head. It seems like it can't use this move... | E | Dynamax | no: unsupported mechanic: dynamax (fail) |
| BT-0138 | d:dynamax.start | ({POKEMON}'s Dynamax!) | E | Dynamax | no: unsupported mechanic: dynamax (start) |
| BT-0139 | d:electricterrain.block | {POKEMON} is protected by the Electric Terrain! | B | (none) (terrain blocks a status or priority move) | no: unsupported mechanic: electricterrain (block) |
| BT-0140 | d:electricterrain.end | The electricity disappeared from the battlefield. | A | field effect,what=end | yes: field |
| BT-0141 | d:electricterrain.start | An electric current ran across the battlefield! | A | field effect,what=start | yes: field |
| BT-0142 | d:flinch.cant | {POKEMON} flinched and couldn't move! | A | cant side,mon,reason=flinch | yes: cant |
| BT-0143 | d:frz.alreadyStarted | {POKEMON} is already frozen solid! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: frz (alreadyStarted) |
| BT-0144 | d:frz.cant | {POKEMON} is frozen solid! | A | cant side,mon,reason=frz | yes: cant |
| BT-0145 | d:frz.end | {POKEMON} thawed out! | A | cure side,mon,status | yes: cure |
| BT-0146 | d:frz.endFromItem | {POKEMON}'s {ITEM} defrosted it! | A | cure side,mon,status,from=item + item what=used | yes: cure |
| BT-0147 | d:frz.endFromMove | {POKEMON}'s {MOVE} melted the ice! | A | cure side,mon,status=frz,from=move | no: unsupported mechanic: frz (endFromMove) |
| BT-0148 | d:frz.start | {POKEMON} was frozen solid! | A | status side,mon,status | yes: status |
| BT-0149 | d:grassyterrain.end | The grass disappeared from the battlefield. | A | field effect,what=end | yes: field |
| BT-0150 | d:grassyterrain.heal | {POKEMON}'s HP was restored. | A | hp side,mon,from=grassyterrain | no: unsupported mechanic: grassyterrain (heal) |
| BT-0151 | d:grassyterrain.start | Grass grew to cover the battlefield! | A | field effect,what=start | yes: field |
| BT-0152 | d:gravity.activate | {POKEMON} fell from the sky due to the gravity! | B | (none) (grounded by Gravity) | no: unsupported mechanic: gravity (activate) |
| BT-0153 | d:gravity.cant | {POKEMON} can't use {MOVE} because of gravity! | A | cant side,mon,reason=gravity | no: unsupported mechanic: gravity (cant) |
| BT-0154 | d:gravity.end | Gravity returned to normal! | A | field effect,what=end | no: unsupported mechanic: gravity (end) |
| BT-0155 | d:gravity.start | Gravity intensified! | A | field effect,what=start | no: unsupported mechanic: gravity (start) |
| BT-0156 | d:hail.damage | {POKEMON} is buffeted by the hail! | A | hp side,mon,from=hail | no: unsupported mechanic: hail (damage) |
| BT-0157 | d:hail.end | The hail stopped. | A | weather weather=none | no: unsupported mechanic: hail (end) |
| BT-0158 | d:hail.start | It started to hail! | A | weather weather | no: unsupported mechanic: hail (start) |
| BT-0159 | d:hail.upkeep | (The hail is crashing down.) | B | (none) (weather continues; contract weather = began or ended) | no: unsupported mechanic: hail (upkeep) |
| BT-0160 | d:hail.weatherName | Hail | C | display name of a weather | no: no_pattern |
| BT-0161 | d:heal.fail | {POKEMON}'s HP is full! | P | fail side,mon ("HP is full") | yes: trailing |
| BT-0162 | d:healreplacement.activate | {POKEMON} will restore its replacement's HP using its Z-Power! | E | Z-Move | no: unsupported mechanic: healreplacement (activate) |
| BT-0163 | d:magicroom.end | Magic Room wore off, and held items' effects returned to normal! | A | field effect,what=end | no: unsupported mechanic: magicroom (end) |
| BT-0164 | d:magicroom.start | It created a bizarre area in which Pokémon's held items lose their effects! | A | field effect,what=start | no: unsupported mechanic: magicroom (start) |
| BT-0165 | d:mistyterrain.block | {POKEMON} surrounds itself with a protective mist! | B | (none) (terrain blocks a status or priority move) | no: unsupported mechanic: mistyterrain (block) |
| BT-0166 | d:mistyterrain.end | The mist disappeared from the battlefield. | A | field effect,what=end | yes: field |
| BT-0167 | d:mistyterrain.start | Mist swirled around the battlefield! | A | field effect,what=start | yes: field |
| BT-0168 | d:mudsport.end | The effects of Mud Sport have faded. | A | field effect,what=end | no: unsupported mechanic: mudsport (end) |
| BT-0169 | d:mudsport.start | Electricity's power was weakened! | A | field effect,what=start | no: unsupported mechanic: mudsport (start) |
| BT-0170 | d:nopp.cant | {POKEMON} used {MOVE}! /   But there was no PP left for the move! | B | (none) (two lines: move used + "no PP left") | yes: move |
| BT-0171 | d:par.alreadyStarted | {POKEMON} is already paralyzed! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: par (alreadyStarted) |
| BT-0172 | d:par.cant | {POKEMON} is paralyzed! It can't move! | A | cant side,mon,reason=par | yes: cant |
| BT-0173 | d:par.end | {POKEMON} was cured of paralysis! | A | cure side,mon,status | no: unsupported mechanic: par (end) |
| BT-0174 | d:par.endFromItem | {POKEMON}'s {ITEM} cured its paralysis! | A | cure side,mon,status,from=item + item what=used | no: unsupported mechanic: par (endFromItem) |
| BT-0175 | d:par.start | {POKEMON} is paralyzed, so it may be unable to move! | A | status side,mon,status | yes: status |
| BT-0176 | d:primordialsea.block | There is no relief from this heavy rain! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: primordialsea (block) |
| BT-0177 | d:primordialsea.blockMove | The Fire-type attack fizzled out in the heavy rain! | B | (none) (extreme weather blocks or weakens a move) | no: unsupported mechanic: primordialsea (blockMove) |
| BT-0178 | d:primordialsea.end | The heavy rain has lifted! | A | weather weather=none | no: unsupported mechanic: primordialsea (end) |
| BT-0179 | d:primordialsea.start | A heavy rain began to fall! | A | weather weather | no: unsupported mechanic: primordialsea (start) |
| BT-0180 | d:primordialsea.weatherName | Heavy Rain | C | display name of a weather | no: unsupported mechanic: primordialsea (weatherName |
| BT-0181 | d:psn.alreadyStarted | {POKEMON} is already poisoned! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: psn (alreadyStarted) or to |
| BT-0182 | d:psn.damage | {POKEMON} was hurt by poison! | D2 | hp side,mon,from=psn/tox ("hurt by poison" shared by poison and bad poison) | no: unsupported mechanic: psn (damage) or tox (damag |
| BT-0183 | d:psn.end | {POKEMON} was cured of its poisoning! | D2 | cure side,mon,status=psn/tox (shared text for poison and bad poison) | no: unsupported mechanic: psn (end) or tox (end) |
| BT-0184 | d:psn.endFromItem | {POKEMON}'s {ITEM} cured its poison! | D2 | cure side,mon,status=psn/tox (shared text for poison and bad poison) | no: unsupported mechanic: psn (endFromItem) or tox ( |
| BT-0185 | d:psn.start | {POKEMON} was poisoned! | A | status side,mon,status | yes: status |
| BT-0186 | d:psychicterrain.block | {POKEMON} is protected by the Psychic Terrain! | B | (none) (terrain blocks a status or priority move) | yes: trailing |
| BT-0187 | d:psychicterrain.end | The weirdness disappeared from the battlefield! | A | field effect,what=end | yes: field |
| BT-0188 | d:psychicterrain.start | The battlefield got weird! | A | field effect,what=start | yes: field |
| BT-0189 | d:raindance.end | The rain stopped. | A | weather weather=none | yes: weather |
| BT-0190 | d:raindance.start | It started to rain! | A | weather weather | yes: weather |
| BT-0191 | d:raindance.upkeep | (Rain continues to fall.) | B | (none) (weather continues; contract weather = began or ended) | no: unsupported mechanic: raindance (upkeep) |
| BT-0192 | d:raindance.weatherName | Rain | C | display name of a weather | no: no_pattern |
| BT-0193 | d:recharge.cant | {POKEMON} must recharge! | A | cant side,mon,reason=recharge | yes: cant |
| BT-0194 | d:recoil.damage | {POKEMON} was damaged by the recoil! | A | hp side,mon,from=recoil | yes: cause |
| BT-0195 | d:sandstorm.damage | {POKEMON} is buffeted by the sandstorm! | A | hp side,mon,from=sandstorm | yes: cause |
| BT-0196 | d:sandstorm.end | The sandstorm subsided. | A | weather weather=none | yes: weather |
| BT-0197 | d:sandstorm.start | A sandstorm kicked up! | A | weather weather | yes: weather |
| BT-0198 | d:sandstorm.upkeep | (The sandstorm is raging.) | B | (none) (weather continues; contract weather = began or ended) | no: unsupported mechanic: sandstorm (upkeep) |
| BT-0199 | d:sandstorm.weatherName | Sandstorm | C | display name of a weather | no: unsupported mechanic: sandstorm (weatherName) |
| BT-0200 | d:slp.alreadyStarted | {POKEMON} is already asleep! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: slp (alreadyStarted) |
| BT-0201 | d:slp.cant | {POKEMON} is fast asleep. | A | cant side,mon,reason=slp | yes: cant |
| BT-0202 | d:slp.end | {POKEMON} woke up! | A | cure side,mon,status | yes: cure |
| BT-0203 | d:slp.endFromItem | {POKEMON}'s {ITEM} woke it up! | A | cure side,mon,status,from=item + item what=used | yes: cure |
| BT-0204 | d:slp.start | {POKEMON} fell asleep! | A | status side,mon,status | yes: status |
| BT-0205 | d:slp.startFromRest | {POKEMON} slept and became healthy! | A | status side,mon,status=slp,from=rest (Champions text differs ("slept and restored its HP!")) | no: unsupported mechanic: slp (startFromRest) |
| BT-0206 | d:snowscape.end | The snow stopped. | A | weather weather=none | yes: weather |
| BT-0207 | d:snowscape.start | It started to snow! | A | weather weather | yes: weather |
| BT-0208 | d:snowscape.upkeep | (The snow is blowing about!) | B | (none) (weather continues; contract weather = began or ended) | no: unsupported mechanic: snowscape (upkeep) |
| BT-0209 | d:snowscape.weatherName | Snow | C | display name of a weather | no: no_pattern |
| BT-0210 | d:struggle.activate | {POKEMON} has no moves left! | B | (none) (no usable move left) | yes: trailing |
| BT-0211 | d:sunnyday.end | The harsh sunlight faded. | A | weather weather=none | yes: weather |
| BT-0212 | d:sunnyday.start | The sunlight turned harsh! | A | weather weather | yes: weather |
| BT-0213 | d:sunnyday.upkeep | (The sunlight is strong.) | B | (none) (weather continues; contract weather = began or ended) | no: unsupported mechanic: sunnyday (upkeep) |
| BT-0214 | d:sunnyday.weatherName | Sun | C | display name of a weather | no: no_pattern |
| BT-0215 | d:tox.alreadyStarted | {POKEMON} is already poisoned! | P | fail side,mon (text names the status; not carried) | no: unsupported mechanic: psn (alreadyStarted) or to |
| BT-0216 | d:tox.damage | {POKEMON} was hurt by poison! | D2 | hp side,mon,from=psn/tox ("hurt by poison" shared by poison and bad poison) | no: unsupported mechanic: psn (damage) or tox (damag |
| BT-0217 | d:tox.end | {POKEMON} was cured of its poisoning! | D2 | cure side,mon,status=psn/tox (shared text for poison and bad poison) | no: unsupported mechanic: psn (end) or tox (end) |
| BT-0218 | d:tox.endFromItem | {POKEMON}'s {ITEM} cured its poison! | D2 | cure side,mon,status=psn/tox (shared text for poison and bad poison) | no: unsupported mechanic: psn (endFromItem) or tox ( |
| BT-0219 | d:tox.start | {POKEMON} was badly poisoned! | A | status side,mon,status | yes: status |
| BT-0220 | d:tox.startFromItem | {POKEMON} was badly poisoned by the {ITEM}! | A | status side,mon,status,from=item + item what=shown (Flame Orb / Toxic Orb) | no: unsupported mechanic: tox (startFromItem) |
| BT-0221 | d:trapped.start | {POKEMON} can no longer escape! | A | volatile side,mon,effect=trapped | yes: volatile |
| BT-0222 | d:trickroom.end | The twisted dimensions returned to normal! | A | field effect,what=end | yes: field |
| BT-0223 | d:trickroom.start | {POKEMON} twisted the dimensions! | A | field effect,what=start | yes: field |
| BT-0224 | d:ui.actionDynamax | Dynamax and | C | client UI prompt/label | no: unsupported mechanic: ui (actionDynamax) |
| BT-0225 | d:ui.actionGigantamax | Gigantamax and | C | client UI prompt/label | no: unsupported mechanic: ui (actionGigantamax) |
| BT-0226 | d:ui.actionMegaEvolve | Mega Evolve and | C | client UI prompt/label | no: unsupported mechanic: ui (actionMegaEvolve) |
| BT-0227 | d:ui.actionMegaEvolveX | Mega Evolve (X) and | C | client UI prompt/label | no: unsupported mechanic: ui (actionMegaEvolveX) |
| BT-0228 | d:ui.actionMegaEvolveY | Mega Evolve (Y) and | C | client UI prompt/label | no: unsupported mechanic: ui (actionMegaEvolveY) |
| BT-0229 | d:ui.actionTerastallize | Terastallize ({TYPE}) and | C | client UI prompt/label | no: unsupported mechanic: ui (actionTerastallize) |
| BT-0230 | d:ui.actionUltraBurst | Ultra Burst and | C | client UI prompt/label | no: unsupported mechanic: ui (actionUltraBurst) |
| BT-0231 | d:ui.atAllySlot | at ally slot {NUMBER} | C | client UI prompt/label | no: unsupported mechanic: ui (atAllySlot) |
| BT-0232 | d:ui.atAllyTarget | at ally {TARGET} | C | client UI prompt/label | no: no_pattern |
| BT-0233 | d:ui.atSlot | at slot {NUMBER} | C | client UI prompt/label | no: no_pattern |
| BT-0234 | d:ui.atTarget | at {TARGET} | C | client UI prompt/label | no: no_pattern |
| BT-0235 | d:ui.autoChoice | Automatic choice | C | client UI prompt/label | no: unsupported mechanic: ui (autoChoice) |
| BT-0236 | d:ui.cantFling | This item cannot be used with Fling. | C | client UI prompt/label | no: unsupported mechanic: ui (cantFling) |
| BT-0237 | d:ui.cantSwitchTrapped | You're trapped and cannot switch! | C | client UI prompt/label | no: unsupported mechanic: ui (cantSwitchTrapped) |
| BT-0238 | d:ui.chooseLead | Choose lead | C | client UI prompt/label | no: unsupported mechanic: ui (chooseLead) |
| BT-0239 | d:ui.chooseSlot | Choose slot {NUMBER} | C | client UI prompt/label | no: unsupported mechanic: ui (chooseSlot) |
| BT-0240 | d:ui.flingBerry | Activates the Berry's effect on the target. | C | client UI prompt/label | no: unsupported mechanic: ui (flingBerry) |
| BT-0241 | d:ui.flingMentalHerb | Removes the effects of Attract, Disable, Encore, Heal Block, Taunt, and Torment from the target. | C | client UI prompt/label | no: unsupported mechanic: ui (flingMentalHerb) |
| BT-0242 | d:ui.flingWhiteHerb | Restores the target's negative stat stages to 0. | C | client UI prompt/label | no: unsupported mechanic: ui (flingWhiteHerb) |
| BT-0243 | d:ui.listComma | , | C | client UI prompt/label | no: no_pattern |
| BT-0244 | d:ui.lockedExplanation | (prevents switching if you're locked) | C | client UI prompt/label | no: unsupported mechanic: ui (lockedExplanation) |
| BT-0245 | d:ui.lockedIntoMove | {POKEMON} is locked into a move. | C | client UI prompt/label | no: unsupported mechanic: ui (lockedIntoMove) |
| BT-0246 | d:ui.mightBeDisabled | You might have some moves disabled, so you won't be able to cancel an attack! | C | client UI prompt/label | no: unsupported mechanic: ui (mightBeDisabled) |
| BT-0247 | d:ui.mightBeLocked | You might be locked into a move. | C | client UI prompt/label | no: unsupported mechanic: ui (mightBeLocked) |
| BT-0248 | d:ui.mightBeTrapped | You might be trapped, so you won't be able to cancel a switch! | C | client UI prompt/label | no: unsupported mechanic: ui (mightBeTrapped) |
| BT-0249 | d:ui.moveTarget | Where should {POKEMON} use {MOVE}? | C | client UI prompt/label | no: unsupported mechanic: ui (moveTarget) |
| BT-0250 | d:ui.notifyMove | Move in your battle | C | client UI prompt/label | no: unsupported mechanic: ui (notifyMove) |
| BT-0251 | d:ui.notifyMoveAgainst | Move in your battle against {OPPONENT} | C | client UI prompt/label | no: unsupported mechanic: ui (notifyMoveAgainst) |
| BT-0252 | d:ui.notifyMoveTitle | Your move! | C | client UI prompt/label | no: unsupported mechanic: ui (notifyMoveTitle) |
| BT-0253 | d:ui.notifySwitch | Switch in your battle | C | client UI prompt/label | no: unsupported mechanic: ui (notifySwitch) |
| BT-0254 | d:ui.notifySwitchAgainst | Switch in your battle against {OPPONENT} | C | client UI prompt/label | no: unsupported mechanic: ui (notifySwitchAgainst) |
| BT-0255 | d:ui.notifySwitchTitle | Your switch! | C | client UI prompt/label | no: unsupported mechanic: ui (notifySwitchTitle) |
| BT-0256 | d:ui.notifyTeam | Choose your team order in your battle | C | client UI prompt/label | no: unsupported mechanic: ui (notifyTeam) |
| BT-0257 | d:ui.notifyTeamAgainst | Choose your team order in your battle against {OPPONENT} | C | client UI prompt/label | no: unsupported mechanic: ui (notifyTeamAgainst) |
| BT-0258 | d:ui.notifyTeamTitle | Team preview! | C | client UI prompt/label | no: unsupported mechanic: ui (notifyTeamTitle) |
| BT-0259 | d:ui.replaceWho | Who will replace {POKEMON}? | C | client UI prompt/label | no: unsupported mechanic: ui (replaceWho) |
| BT-0260 | d:ui.reviveWho | Who will {POKEMON} revive? | C | client UI prompt/label | no: unsupported mechanic: ui (reviveWho) |
| BT-0261 | d:ui.tagMoves | {TAG} moves | C | client UI prompt/label | no: no_pattern |
| BT-0262 | d:ui.teamRest | What about the rest of your team? | C | client UI prompt/label | no: unsupported mechanic: ui (teamRest) |
| BT-0263 | d:ui.teamSoFar | Team so far | C | client UI prompt/label | no: unsupported mechanic: ui (teamSoFar) |
| BT-0264 | d:ui.teamStart | How will you start the battle? | C | client UI prompt/label | no: unsupported mechanic: ui (teamStart) |
| BT-0265 | d:ui.unobtainableInGen | Unobtainable in Gen {NUMBER} | C | client UI prompt/label | no: unsupported mechanic: ui (unobtainableInGen) |
| BT-0266 | d:ui.unrecognizedChoice | Unrecognized choice from server: | C | client UI prompt/label | no: unsupported mechanic: ui (unrecognizedChoice) |
| BT-0267 | d:ui.waitingOpponent | Waiting for opponent... | C | client UI prompt/label | no: unsupported mechanic: ui (waitingOpponent) |
| BT-0268 | d:ui.whatDo | What will {POKEMON} do? | C | client UI prompt/label | no: unsupported mechanic: ui (whatDo) |
| BT-0269 | d:ui.willRevive | {POKEMON} will revive {TARGET}. | C | client UI prompt/label | no: unsupported mechanic: ui (willRevive) |
| BT-0270 | d:ui.willShift | {POKEMON} will shift to the center. | C | client UI prompt/label | no: unsupported mechanic: ui (willShift) |
| BT-0271 | d:ui.willSwitch | {POKEMON} will switch to {TARGET}. | C | client UI prompt/label | no: unsupported mechanic: ui (willSwitch) |
| BT-0272 | d:ui.willUseMove | {POKEMON} will {ACTIONS}use {MOVE}{AT}. | C | client UI prompt/label | no: no_pattern |
| BT-0273 | d:ui.youPicked | You picked {POKEMON}. | C | client UI prompt/label | no: unsupported mechanic: ui (youPicked) |
| BT-0274 | d:ui.zEffectClearNegativeBoost | Restores negative stat stages to 0 | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectClearNegativeBo |
| BT-0275 | d:ui.zEffectCrit2 | Crit ratio +2 | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectCrit2) |
| BT-0276 | d:ui.zEffectCurse | Restores HP 100% if user is Ghost type, otherwise Attack +1 | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectCurse) |
| BT-0277 | d:ui.zEffectHeal | Restores HP 100% | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectHeal) |
| BT-0278 | d:ui.zEffectHealReplacement | Restores replacement's HP 100% | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectHealReplacement |
| BT-0279 | d:ui.zEffectRedirect | Redirects opposing attacks to user | C | client UI prompt/label | no: unsupported mechanic: ui (zEffectRedirect) |
| BT-0280 | d:unboost.fail | {POKEMON}'s {STAT} {INFLECT:STAT:s=was:p=were} not lowered! | B | (none) (stat drop prevented) | yes: boost_blocked |
| BT-0281 | d:unboost.failNoStat | {POKEMON}'s stats were not lowered! | B | (none) (stat drop prevented) | yes: trailing |
| BT-0282 | d:watersport.end | The effects of Water Sport have faded. | A | field effect,what=end | no: unsupported mechanic: watersport (end) |
| BT-0283 | d:watersport.start | Fire's power was weakened! | A | field effect,what=start | no: unsupported mechanic: watersport (start) |
| BT-0284 | d:wonderroom.end | Wonder Room wore off, and Defense and Sp. Def stats returned to normal! | A | field effect,what=end | no: unsupported mechanic: wonderroom (end) |
| BT-0285 | d:wonderroom.start | It created a bizarre area in which Defense and Sp. Def stats are swapped! | A | field effect,what=start | no: unsupported mechanic: wonderroom (start) |
| BT-0305 | m:afteryou.activate | {TARGET} took the kind offer! | E | ally move text | no: unsupported mechanic: After You (activate) |
| BT-0332 | m:aquaring.heal | A veil of water restored {POKEMON}'s HP! | A | hp side,mon,from=aquaring | no: unsupported mechanic: Aqua Ring (heal) |
| BT-0334 | m:aquaring.start | {POKEMON} surrounded itself with a veil of water! | A | volatile side,mon,effect=aquaring | no: unsupported mechanic: Aqua Ring (start) |
| BT-0342 | m:aromatherapy.activate | A soothing aroma wafted through the area! | B | (none) (team status cure, nobody named) | no: unsupported mechanic: Aromatherapy (activate) |
| BT-0357 | m:attract.activate | {POKEMON} is in love with {TARGET}! | A | volatile_hit side,mon,effect=attract | yes: volatile_active |
| BT-0358 | m:attract.cant | {POKEMON} is immobilized by love! | A | cant side,mon,reason=attract | yes: cant |
| BT-0360 | m:attract.end | {POKEMON} got over its infatuation! | A | volatile_end effect=attract | no: unsupported mechanic: Attract (end) |
| BT-0361 | m:attract.endFromItem | {POKEMON} cured its infatuation using its {ITEM}! | A | volatile_end effect=attract + item what=used | no: unsupported mechanic: Attract (endFromItem) |
| BT-0363 | m:attract.start | {POKEMON} fell in love! | A | volatile side,mon,effect=attract | yes: volatile |
| BT-0364 | m:attract.startFromItem | {POKEMON} fell in love because of the {ITEM}! | A | volatile effect=attract + item what=shown (Destiny Knot) | no: unsupported mechanic: Attract (startFromItem) |
| BT-0371 | m:auroraveil.end | {TEAM}'s Aurora Veil wore off! | A | side side,effect,what=end | no: unsupported mechanic: Aurora Veil (end) |
| BT-0373 | m:auroraveil.start | Aurora Veil made {TEAM} stronger against physical and special moves! | A | side side,effect,what=start | yes: hazard |
| BT-0376 | m:autotomize.start | {POKEMON} became nimble! | A | volatile side,mon,effect=autotomize | no: unsupported mechanic: Autotomize (start) |
| BT-0379 | m:axekick.damage | {POKEMON} kept going and crashed! | D2 | hp side,mon,from=crash/move (same text as d:crash.damage) | yes: cause |
| BT-0398 | m:beakblast.start | {POKEMON} started heating up its beak! | A | once side,mon,effect | yes: single |
| BT-0399 | m:beatup.activate | {NAME}'s attack! | B | (none) (names the ally whose hit is used) | no: no_pattern |
| BT-0406 | m:bellydrum.boost | {POKEMON} cut its own HP and maximized its Attack! | P | boost side,mon,stat=atk (by not given) + hp | no: unsupported mechanic: Belly Drum (boost) |
| BT-0411 | m:bestow.takeItem | {SOURCE} gave {POKEMON} its {ITEM}! | A | item side,mon,item,what=received,of | no: no_pattern |
| BT-0412 | m:bide.activate | {POKEMON} is storing energy! | D1 | volatile vs volatile_hit (same text as m:bide.start) | no: unsupported mechanic: Bide (activate) or Bide (s |
| BT-0414 | m:bide.end | {POKEMON} unleashed its energy! | A | volatile_end side,mon,effect=bide | no: unsupported mechanic: Bide (end) |
| BT-0416 | m:bide.start | {POKEMON} is storing energy! | D1 | volatile vs volatile_hit (same text as m:bide.activate) | no: unsupported mechanic: Bide (activate) or Bide (s |
| BT-0418 | m:bind.move | {POKEMON}'s attack continues! | D2 | (none) ("attack continues" for four trapping moves; also no kind) | no: unsupported mechanic: Bind (move) or Clamp (move |
| BT-0420 | m:bind.start | {POKEMON} was squeezed by {SOURCE}! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-0461 | m:bounce.prepare | {POKEMON} sprang up! | A | prepare side,mon,move | yes: prepare |
| BT-0472 | m:brickbreak.activate | {POKEMON} shattered {TEAM}'s protections! | P | side side,effect=<screen>,what=end (which screens not named) | no: unsupported mechanic: Brick Break (activate) or  |
| BT-0484 | m:bugbite.removeItem | {SOURCE} stole and ate its target's {ITEM}! | A | item side,mon,item,what=used (user ate target's item) + item of target what=removed | yes: steal_eat |
| BT-0502 | m:burnup.typeChange | {POKEMON} burned itself out! | P | types side,mon (the lost type is not named) | yes: typechange |
| BT-0514 | m:celebrate.activate | Congratulations, {TRAINER}! | C | flavour text | no: unsupported mechanic: Celebrate (activate) |
| BT-0518 | m:charge.start | {POKEMON} began charging power! | A | volatile side,mon,effect=charge | yes: volatile |
| BT-0528 | m:chillyreception.prepare | {POKEMON} is preparing to tell a chillingly bad joke! | B | (none) (joke announced before the move) | no: unsupported mechanic: Chilly Reception (prepare) |
| BT-0537 | m:clamp.move | {POKEMON}'s attack continues! | D2 | (none) ("attack continues" for four trapping moves; also no kind) | no: unsupported mechanic: Bind (move) or Clamp (move |
| BT-0539 | m:clamp.start | {SOURCE} clamped down on {POKEMON}! | A | volatile side,mon,effect=<trapping move> | no: unsupported mechanic: Clamp (start) |
| BT-0572 | m:conversion.typeChange | Converted type to {SOURCE}'s! | P | types side,mon (type not named) | no: unsupported mechanic: Conversion (typeChange) |
| BT-0581 | m:corrosivegas.fail | But it failed to affect {POKEMON}! | P | fail side,mon (wording differs) | no: unsupported mechanic: Corrosive Gas (fail) or He |
| BT-0582 | m:corrosivegas.removeItem | {SOURCE} corroded {POKEMON}'s {ITEM}! | A | item side,mon,item,what=removed,of | no: unsupported mechanic: Corrosive Gas (removeItem) |
| BT-0592 | m:courtchange.activate | {POKEMON} swapped the battle effects affecting each side of the field! | B | (none) (side effects moved or cleared; which not named) | no: unsupported mechanic: Court Change (activate) |
| BT-0599 | m:craftyshield.block | Crafty Shield protected {POKEMON}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Crafty Shield (block) |
| BT-0602 | m:craftyshield.start | Crafty Shield protected {TEAM}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Crafty Shield (block) or C |
| BT-0613 | m:curse.damage | {POKEMON} is afflicted by the curse! | A | hp side,mon,from=curse | yes: cause |
| BT-0616 | m:curse.start | {SOURCE} cut its own HP and put a curse on {POKEMON}! | A | volatile side,mon,effect=curse + hp | yes: volatile |
| BT-0623 | m:darkvoid.fail | But {POKEMON} can't use the move! | P | fail side,mon | no: unsupported mechanic: Dark Void (fail) or Hypers |
| BT-0624 | m:darkvoid.failWrongForme | But {POKEMON} can't use it the way it is now! | P | fail side,mon | no: unsupported mechanic: Dark Void (failWrongForme) |
| BT-0636 | m:destinybond.activate | {POKEMON} took its attacker down with it! | A | volatile_hit side,mon,effect=destinybond | yes: trailing |
| BT-0639 | m:destinybond.start | {POKEMON} is hoping to take its attacker down with it! | A | once side,mon,effect=destinybond | yes: single |
| BT-0646 | m:dig.prepare | {POKEMON} burrowed its way under the ground! | A | prepare side,mon,move | yes: prepare |
| BT-0650 | m:disable.cant | {POKEMON}'s {MOVE} is disabled! | A | cant side,mon,reason=disable | no: unsupported mechanic: Disable (cant) |
| BT-0652 | m:disable.end | {POKEMON}'s move is no longer disabled! | A | volatile_end effect=disable | no: unsupported mechanic: Disable (end) |
| BT-0654 | m:disable.start | {POKEMON}'s {MOVE} was disabled! | A | volatile side,mon,effect=disable,detail=<move> | yes: volatile |
| BT-0660 | m:dive.prepare | {POKEMON} hid underwater! | A | prepare side,mon,move | yes: prepare |
| BT-0666 | m:doomdesire.activate | {TARGET} took the Doom Desire attack! | B | (none) (delayed attack queued / lands) | no: unsupported mechanic: Doom Desire (activate) |
| BT-0669 | m:doomdesire.start | {POKEMON} chose Doom Desire as its destiny! | B | (none) (delayed attack queued / lands) | no: unsupported mechanic: Doom Desire (start) |
| BT-0680 | m:doubleshock.typeChange | {POKEMON} used up all its electricity! | P | types side,mon (the lost type is not named) | yes: move |
| BT-0688 | m:dragonascent.megaNoItem | {TRAINER}'s fervent wish has reached {POKEMON}! | P | mega side,mon (form not named) (Mega Rayquaza wording) | no: unsupported mechanic: Dragon Ascent (megaNoItem) |
| BT-0694 | m:dragoncheer.start | {POKEMON} is getting pumped! | D2 | volatile side,mon,effect=? ("is getting pumped!" shared) | no: unsupported mechanic: Dragon Cheer (start) or Fo |
| BT-0735 | m:eeriespell.activate | It reduced the PP of {TARGET}'s {MOVE} by {NUMBER}! | B | (none) (S5: PP of a named move is shown) | no: unsupported mechanic: Eerie Spell (activate) or  |
| BT-0743 | m:electrify.start | {POKEMON}'s moves have been electrified! | A | volatile side,mon,effect=electrify | no: unsupported mechanic: Electrify (start) |
| BT-0749 | m:electroshot.prepare | {POKEMON} absorbed electricity! | A | prepare side,mon,move | yes: prepare |
| BT-0754 | m:embargo.end | {POKEMON} can use items again! | A | volatile_end effect=embargo | no: unsupported mechanic: Embargo (end) |
| BT-0756 | m:embargo.start | {POKEMON} can't use items anymore! | A | volatile side,mon,effect=embargo | no: unsupported mechanic: Embargo (start) |
| BT-0760 | m:encore.end | {POKEMON}'s encore ended! | A | volatile_end effect=encore | no: unsupported mechanic: Encore (end) |
| BT-0762 | m:encore.start | {POKEMON} must do an encore! | A | volatile side,mon,effect=encore | yes: volatile |
| BT-0765 | m:endure.activate | {POKEMON} endured the hit! | D1 | volatile_hit (Endure) / ability (Sturdy) ("endured the hit!" shared) | yes: trailing |
| BT-0768 | m:endure.start | {POKEMON} braced itself! | A | once side,mon,effect | yes: trailing |
| BT-0791 | m:fairylock.activate | No one will be able to run away during the next turn! | P | field effect,what=start (not a terrain or room) | no: unsupported mechanic: Fairy Lock (activate) |
| BT-0804 | m:feint.activate | {TARGET} fell for the feint! | B | (none) (S5-adjacent: a protection was broken) | no: unsupported mechanic: Feint (activate) |
| BT-0810 | m:ficklebeam.activate | {POKEMON} is going all out for this attack! | B | (none) (power doubled) | no: unsupported mechanic: Fickle Beam (activate) |
| BT-0826 | m:firepledge.activate | {POKEMON} is waiting for {TARGET}'s move... | E | pledge combination with an ally | no: unsupported mechanic: Fire Pledge (activate) or  |
| BT-0827 | m:firepledge.damage | {POKEMON} was hurt by the sea of fire! | A | hp side,mon,from=firepledge | no: unsupported mechanic: Fire Pledge (damage) |
| BT-0829 | m:firepledge.end | The sea of fire around {TEAM} disappeared! | A | side side,effect,what=end | no: unsupported mechanic: Fire Pledge (end) |
| BT-0831 | m:firepledge.start | A sea of fire enveloped {TEAM}! | A | side side,effect,what=start | no: unsupported mechanic: Fire Pledge (start) |
| BT-0835 | m:firespin.move | {POKEMON}'s attack continues! | D2 | (none) ("attack continues" for four trapping moves; also no kind) | no: unsupported mechanic: Bind (move) or Clamp (move |
| BT-0837 | m:firespin.start | {POKEMON} became trapped in the fiery vortex! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-0846 | m:flameburst.damage | The bursting flame hit {POKEMON}! | E | hits the ally | no: unsupported mechanic: Flame Burst (damage) |
| BT-0866 | m:fling.removeItem | {POKEMON} flung its {ITEM}! | A | item side,mon,item,what=used | no: unsupported mechanic: Fling (removeItem) |
| BT-0870 | m:flipturn.switchOut | {POKEMON} went back to {TRAINER}! | D2 | (none) ("went back to TRAINER": withdrawal of four moves; also no kind) | yes: trailing |
| BT-0880 | m:fly.prepare | {POKEMON} flew up high! | A | prepare side,mon,move | yes: prepare |
| BT-0888 | m:focusenergy.start | {POKEMON} is getting pumped! | D2 | volatile side,mon,effect=? ("is getting pumped!" shared) | no: unsupported mechanic: Dragon Cheer (start) or Fo |
| BT-0889 | m:focusenergy.startFromItem | {POKEMON} used the {ITEM} to get pumped! | A | volatile effect=focusenergy + item what=used | yes: move |
| BT-0890 | m:focusenergy.startFromZEffect | {POKEMON} boosted its critical-hit ratio using its Z-Power! | E | Z-Move | no: unsupported mechanic: Focus Energy (startFromZEf |
| BT-0891 | m:focuspunch.cant | {POKEMON} lost its focus and couldn't move! | A | cant side,mon,reason=focuspunch | no: unsupported mechanic: Focus Punch (cant) |
| BT-0894 | m:focuspunch.start | {POKEMON} is tightening its focus! | A | once side,mon,effect | no: unsupported mechanic: Focus Punch (start) |
| BT-0897 | m:followme.start | {POKEMON} became the center of attention! | D2 | volatile / once effect=? ("became the center of attention!" shared (Follow Me, Rage Powder, Spotlight)) | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-0898 | m:followme.startFromZEffect | {POKEMON} became the center of attention! | E | Z-Move | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-0903 | m:foresight.start | {POKEMON} was identified! | D2 | volatile effect=? ("was identified!" shared) | no: unsupported mechanic: Foresight (start) or Mirac |
| BT-0911 | m:freezeshock.prepare | {POKEMON} became cloaked in a freezing light! | A | prepare side,mon,move | no: unsupported mechanic: Freeze Shock (prepare) |
| BT-0933 | m:futuresight.activate | {TARGET} took the Future Sight attack! | B | (none) (delayed attack queued / lands) | no: unsupported mechanic: Future Sight (activate) |
| BT-0936 | m:futuresight.start | {POKEMON} foresaw an attack! | B | (none) (delayed attack queued / lands) | no: unsupported mechanic: Future Sight (start) |
| BT-0939 | m:gastroacid.start | {POKEMON}'s Ability was suppressed! | A | volatile side,mon,effect=gastroacid | no: unsupported mechanic: Gastro Acid (start) |
| BT-0947 | m:geomancy.prepare | {POKEMON} is absorbing power! | A | prepare side,mon,move | no: unsupported mechanic: Geomancy (prepare) |
| BT-0967 | m:gmaxcannonade.damage | {POKEMON} is hurt by G-Max Cannonade’s vortex! | E | G-Max / Z-Move text | yes: cause |
| BT-0970 | m:gmaxcannonade.start | {PARTY} got caught in the vortex of water! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Cannonade (start) |
| BT-0975 | m:gmaxchistrike.start | {POKEMON} is getting pumped! | D2 | volatile side,mon,effect=? ("is getting pumped!" shared) | no: unsupported mechanic: Dragon Cheer (start) or Fo |
| BT-0978 | m:gmaxdepletion.activate | {TARGET}'s PP was reduced! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Depletion (activate) |
| BT-1013 | m:gmaxsteelsurge.damage | The sharp steel bit into {POKEMON}! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Steelsurge (damage) |
| BT-1015 | m:gmaxsteelsurge.end | The pieces of steel surrounding {PARTY} disappeared! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Steelsurge (end) |
| BT-1017 | m:gmaxsteelsurge.start | Sharp-pointed pieces of steel started floating around {PARTY}! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Steelsurge (start) |
| BT-1028 | m:gmaxvinelash.damage | {POKEMON} is hurt by G-Max Vine Lash’s ferocious beating! | E | G-Max / Z-Move text | yes: cause |
| BT-1031 | m:gmaxvinelash.start | {PARTY} got trapped with vines! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Vine Lash (start) |
| BT-1032 | m:gmaxvolcalith.damage | {POKEMON} is hurt by the rocks thrown out by G-Max Volcalith! | E | G-Max / Z-Move text | yes: cause |
| BT-1035 | m:gmaxvolcalith.start | {PARTY} became surrounded by rocks! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Volcalith (start) |
| BT-1038 | m:gmaxwildfire.damage | {POKEMON} is burning up within G-Max Wildfire’s flames! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Wildfire (damage) |
| BT-1041 | m:gmaxwildfire.start | {PARTY} were surrounded by fire! | E | G-Max / Z-Move text | no: unsupported mechanic: G-Max Wildfire (start) |
| BT-1046 | m:grasspledge.activate | {POKEMON} is waiting for {TARGET}'s move... | E | pledge combination with an ally | no: unsupported mechanic: Fire Pledge (activate) or  |
| BT-1048 | m:grasspledge.end | The swamp around {TEAM} disappeared! | A | side side,effect,what=end | no: unsupported mechanic: Grass Pledge (end) |
| BT-1050 | m:grasspledge.start | A swamp enveloped {TEAM}! | A | side side,effect,what=start | no: unsupported mechanic: Grass Pledge (start) |
| BT-1064 | m:grudge.activate | {POKEMON}'s {MOVE} lost all of its PP due to the grudge! | B | (none) (S5: PP of a named move is shown) | no: unsupported mechanic: Grudge (activate) |
| BT-1067 | m:grudge.start | {POKEMON} wants its target to bear a grudge! | A | volatile side,mon,effect=grudge | no: unsupported mechanic: Grudge (start) |
| BT-1070 | m:guardsplit.activate | {POKEMON} shared its guard with the target! | B | (none) (stat swap or average) | no: unsupported mechanic: Guard Split (activate) |
| BT-1087 | m:happyhour.activate | Everyone is caught up in the happy atmosphere! | C | flavour text | no: unsupported mechanic: Happy Hour (activate) |
| BT-1093 | m:haze.activate | All stat changes were eliminated! | A | boost_reset what=all (names no Pokemon; same text and same event as d:default.clearAllBoost) | yes: clearallboost |
| BT-1104 | m:healbell.activate | A bell chimed! | B | (none) (team status cure, nobody named) | no: unsupported mechanic: Heal Bell (activate) |
| BT-1107 | m:healblock.cant | {POKEMON} is prevented from healing, so it can't use {MOVE}! | A | cant side,mon,reason=healblock | no: unsupported mechanic: Heal Block (cant) |
| BT-1109 | m:healblock.end | {POKEMON} is no longer prevented from healing! | A | volatile_end effect=healblock | yes: volatile_end |
| BT-1110 | m:healblock.fail | But it failed to affect {POKEMON}! | P | fail side,mon (wording differs) | no: unsupported mechanic: Corrosive Gas (fail) or He |
| BT-1112 | m:healblock.start | {POKEMON} was prevented from healing! | A | volatile side,mon,effect=healblock | yes: volatile |
| BT-1114 | m:healingwish.heal | The healing wish came true for {POKEMON}! | A | hp side,mon,from=healingwish/lunardance | yes: full_heal |
| BT-1132 | m:helpinghand.start | {SOURCE} is ready to help {POKEMON}! | E | ally move text | no: unsupported mechanic: Helping Hand (start) |
| BT-1138 | m:highjumpkick.damage | {POKEMON} kept going and crashed! | D2 | hp side,mon,from=crash/move (same text as d:crash.damage) | yes: cause |
| BT-1167 | m:hyperspacefury.activate | It broke through {TARGET}'s protection! | D2 | (none) ("broke through TARGET's protection!" shared; S5-adjacent (Protect shown)) | no: unsupported mechanic: Hyperspace Fury (activate) |
| BT-1169 | m:hyperspacefury.fail | But {POKEMON} can't use the move! | P | fail side,mon | no: unsupported mechanic: Dark Void (fail) or Hypers |
| BT-1171 | m:hyperspacehole.activate | It broke through {TARGET}'s protection! | D2 | (none) ("broke through TARGET's protection!" shared; S5-adjacent (Protect shown)) | no: unsupported mechanic: Hyperspace Fury (activate) |
| BT-1182 | m:iceburn.prepare | {POKEMON} became cloaked in freezing air! | A | prepare side,mon,move | no: unsupported mechanic: Ice Burn (prepare) |
| BT-1200 | m:imprison.cant | {POKEMON} can't use its sealed {MOVE}! | A | cant side,mon,reason=imprison | no: unsupported mechanic: Imprison (cant) |
| BT-1203 | m:imprison.start | {POKEMON} sealed any moves its target shares with it! | A | volatile side,mon,effect=imprison | no: unsupported mechanic: Imprison (start) |
| BT-1205 | m:incinerate.removeItem | {POKEMON}'s {ITEM} was burned up! | A | item side,mon,item,what=removed | no: unsupported mechanic: Incinerate (removeItem) |
| BT-1214 | m:infestation.start | {POKEMON} has been afflicted with an infestation by {SOURCE}! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-1215 | m:ingrain.block | {POKEMON} is anchored in place with its roots! | B | (none) (cannot be forced out) | no: unsupported mechanic: Ingrain (block) |
| BT-1217 | m:ingrain.heal | {POKEMON} absorbed nutrients with its roots! | A | hp side,mon,from=ingrain | no: unsupported mechanic: Ingrain (heal) |
| BT-1219 | m:ingrain.start | {POKEMON} planted its roots! | A | volatile side,mon,effect=ingrain | no: unsupported mechanic: Ingrain (start) |
| BT-1220 | m:instruct.activate | {TARGET} followed {POKEMON}'s instructions! | E | ally move text | no: unsupported mechanic: Instruct (activate) |
| BT-1223 | m:iondeluge.activate | A deluge of ions showers the battlefield! | P | field effect,what=start (not a terrain or room) | no: unsupported mechanic: Ion Deluge (activate) |
| BT-1240 | m:jumpkick.damage | {POKEMON} kept going and crashed! | D2 | hp side,mon,from=crash/move (same text as d:crash.damage) | yes: cause |
| BT-1252 | m:knockoff.removeItem | {SOURCE} knocked off {POKEMON}'s {ITEM}! | A | item side,mon,item,what=removed,of | yes: knock_off |
| BT-1259 | m:laserfocus.start | {POKEMON} concentrated intensely! | A | volatile side,mon,effect=laserfocus | no: unsupported mechanic: Laser Focus (start) |
| BT-1277 | m:leechseed.damage | {POKEMON}'s health is sapped by Leech Seed! | A | hp side,mon,from=leechseed | yes: cause |
| BT-1279 | m:leechseed.end | {POKEMON} was freed from Leech Seed! | A | volatile_end effect=leechseed | no: unsupported mechanic: Leech Seed (end) |
| BT-1281 | m:leechseed.start | {POKEMON} was seeded! | A | volatile effect=leechseed | yes: volatile |
| BT-1292 | m:lightscreen.end | {TEAM}'s Light Screen wore off! | A | side side,effect,what=end | no: unsupported mechanic: Light Screen (end) |
| BT-1294 | m:lightscreen.start | Light Screen made {TEAM} stronger against special moves! | A | side side,effect,what=start | yes: hazard |
| BT-1301 | m:lockon.start | {SOURCE} took aim at {POKEMON}! | D2 | volatile effect=? ("took aim at" shared) | no: unsupported mechanic: Lock-On (start) or Mind Re |
| BT-1308 | m:luckychant.end | {TEAM}'s Lucky Chant wore off! | A | side side,effect,what=end | no: unsupported mechanic: Lucky Chant (end) |
| BT-1310 | m:luckychant.start | Lucky Chant shielded {TEAM} from critical hits! | A | side side,effect,what=start | no: unsupported mechanic: Lucky Chant (start) |
| BT-1316 | m:lunardance.heal | {POKEMON} became cloaked in mystical moonlight! | A | hp side,mon,from=healingwish/lunardance | no: unsupported mechanic: Lunar Dance (heal) |
| BT-1328 | m:magiccoat.move | {POKEMON} bounced the {MOVE} back! | D1 | move bounced: once/ability/(none) ("bounced the MOVE back!": Magic Coat, Magic Bounce, Rebound) | no: unsupported mechanic: Magic Bounce (move) or Mag |
| BT-1330 | m:magiccoat.start | {POKEMON} shrouded itself with Magic Coat! | A | once side,mon,effect | no: unsupported mechanic: Magic Coat (start) |
| BT-1337 | m:magmastorm.start | {POKEMON} became trapped by swirling magma! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-1342 | m:magnetrise.end | {POKEMON}'s electromagnetism wore off! | A | volatile_end effect | no: unsupported mechanic: Magnet Rise (end) |
| BT-1344 | m:magnetrise.start | {POKEMON} levitated with electromagnetism! | A | volatile side,mon,effect | no: unsupported mechanic: Magnet Rise (start) |
| BT-1345 | m:magnitude.activate | Magnitude {NUMBER}! | B | (none) (magnitude level) | no: unsupported mechanic: Magnitude (activate) |
| BT-1348 | m:makeitrain.activate | Coins were scattered everywhere! | C | flavour text | no: unsupported mechanic: Make It Rain (activate) or |
| BT-1355 | m:matblock.block | {MOVE} was blocked by the kicked-up mat! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Mat Block (block) |
| BT-1358 | m:matblock.start | {POKEMON} intends to flip up a mat and block incoming attacks! | B | (none) (S5-adjacent: Protect-like block) | no: unsupported mechanic: Mat Block (start) |
| BT-1371 | m:maxguard.activate | {POKEMON} protected itself! | D2 | (none) (S5: "protected itself!" is a Protect/Max Guard activation; shared text) | yes: trailing |
| BT-1412 | m:memento.heal | {POKEMON}'s HP was restored by the Z-Power! | E | G-Max / Z-Move text | no: unsupported mechanic: Memento (heal) or Parting  |
| BT-1425 | m:meteorbeam.prepare | {POKEMON} is overflowing with space power! | A | prepare side,mon,move | yes: prepare |
| BT-1430 | m:metronome.move | Waggling a finger let it use {MOVE}! | A | move side,mon,move,called_by=metronome (move named in text; user from the preceding row) | no: unsupported mechanic: Metronome (move) |
| BT-1437 | m:mimic.start | {POKEMON} learned {MOVE}! | B | (none) (moveset changed) | no: no_pattern |
| BT-1438 | m:mindblown.damage | ({POKEMON} cut its own HP to power up its move!) | D2 | hp side,mon,from=move (same text for two moves) | no: unsupported mechanic: Mind Blown (damage) or Ste |
| BT-1443 | m:mindreader.start | {SOURCE} took aim at {POKEMON}! | D2 | volatile effect=? ("took aim at" shared) | no: unsupported mechanic: Lock-On (start) or Mind Re |
| BT-1448 | m:miracleeye.start | {POKEMON} was identified! | D2 | volatile effect=? ("was identified!" shared) | no: unsupported mechanic: Foresight (start) or Mirac |
| BT-1455 | m:mist.block | {POKEMON} is protected by the mist! | B | (none) (status blocked by side effect) | no: unsupported mechanic: Mist (block) |
| BT-1457 | m:mist.end | {TEAM} is no longer protected by mist! | A | side side,effect,what=end | no: unsupported mechanic: Mist (end) |
| BT-1459 | m:mist.start | {TEAM} became shrouded in mist! | A | side side,effect,what=start | no: unsupported mechanic: Mist (start) |
| BT-1499 | m:naturepower.move | Nature Power turned into {MOVE}! | A | move called_by=naturepower | no: unsupported mechanic: Nature Power (move) |
| BT-1508 | m:nightmare.damage | {POKEMON} is locked in a nightmare! | A | hp side,mon,from=nightmare | no: unsupported mechanic: Nightmare (damage) |
| BT-1511 | m:nightmare.start | {POKEMON} began having a nightmare! | A | volatile effect=nightmare | no: unsupported mechanic: Nightmare (start) |
| BT-1520 | m:noretreat.start | {POKEMON} can no longer escape because it used No Retreat! | A | volatile side,mon,effect | yes: move |
| BT-1534 | m:octolock.start | {POKEMON} can no longer escape because of Octolock! | A | volatile side,mon,effect | no: unsupported mechanic: Octolock (start) |
| BT-1549 | m:painsplit.activate | The battlers shared their pain! | B | (none) (both HP rows follow) | yes: trailing |
| BT-1557 | m:partingshot.heal | {POKEMON}'s HP was restored by the Z-Power! | E | G-Max / Z-Move text | no: unsupported mechanic: Memento (heal) or Parting  |
| BT-1559 | m:partingshot.switchOut | {POKEMON} went back to {TRAINER}! | D2 | (none) ("went back to TRAINER": withdrawal of four moves; also no kind) | yes: trailing |
| BT-1562 | m:payday.activate | Coins were scattered everywhere! | C | flavour text | no: unsupported mechanic: Make It Rain (activate) or |
| BT-1566 | m:perishsong.activate | {POKEMON}'s perish count fell to {NUMBER}! | A | volatile side,mon,effect=perish,detail=N (S5: turns remaining shown, contract lists "Perish count" in section 4 but says counts never appear) | yes: volatile |
| BT-1569 | m:perishsong.start | All Pokémon that heard the song will faint in three turns! | B | (none) (no Pokemon named) | yes: field_activate |
| BT-1574 | m:phantomforce.activate | It broke through {TARGET}'s protection! | D2 | (none) ("broke through TARGET's protection!" shared; S5-adjacent (Protect shown)) | no: unsupported mechanic: Hyperspace Fury (activate) |
| BT-1576 | m:phantomforce.prepare | {POKEMON} vanished instantly! | D2 | prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) | yes: prepare |
| BT-1591 | m:pluck.removeItem | {SOURCE} stole and ate its target's {ITEM}! | A | item side,mon,item,what=used (user ate target's item) + item of target what=removed | yes: steal_eat |
| BT-1609 | m:poltergeist.activate | {POKEMON} is about to be attacked by its {ITEM}! | A | item side,mon,item,what=shown | yes: item_reveal |
| BT-1616 | m:powder.activate | When the flame touched the powder on the Pokémon, it exploded! | B | (none) (no Pokemon named) | no: unsupported mechanic: Powder (activate) |
| BT-1619 | m:powder.start | {POKEMON} is covered in powder! | A | volatile side,mon,effect=powder | no: unsupported mechanic: Powder (start) |
| BT-1625 | m:powershift.start | {POKEMON} swapped its offensive stats with its defensive stats! | B | (none) (stat swap or average) | no: unsupported mechanic: Power Shift (start) |
| BT-1626 | m:powersplit.activate | {POKEMON} shared its power with the target! | B | (none) (stat swap or average) | no: unsupported mechanic: Power Split (activate) |
| BT-1633 | m:powertrick.start | {POKEMON} switched its Attack and Defense! | B | (none) (stat swap or average) | no: unsupported mechanic: Power Trick (start) |
| BT-1645 | m:protect.block | {POKEMON} protected itself! | D2 | (none) (S5: "protected itself!" is a Protect/Max Guard activation; shared text) | yes: trailing |
| BT-1648 | m:protect.start | {POKEMON} protected itself! | D2 | (none) (S5: "protected itself!" is a Protect/Max Guard activation; shared text) | yes: trailing |
| BT-1682 | m:pursuit.activate | ({TARGET} is being withdrawn...) | B | (none) (move order / interception text) | no: unsupported mechanic: Pursuit (activate) |
| BT-1687 | m:quash.activate | {TARGET}'s move was postponed! | B | (none) (move order / interception text) | no: unsupported mechanic: Quash (activate) |
| BT-1692 | m:quickguard.block | Quick Guard protected {POKEMON}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Quick Guard (block) |
| BT-1695 | m:quickguard.start | Quick Guard protected {TEAM}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Quick Guard (block) or Qui |
| BT-1704 | m:ragepowder.start | {POKEMON} became the center of attention! | D2 | volatile / once effect=? ("became the center of attention!" shared (Follow Me, Rage Powder, Spotlight)) | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-1705 | m:ragepowder.startFromZEffect | {POKEMON} became the center of attention! | E | Z-Move | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-1706 | m:ragingbull.activate | {POKEMON} shattered {TEAM}'s protections! | P | side side,effect=<screen>,what=end (which screens not named) | no: unsupported mechanic: Brick Break (activate) or  |
| BT-1720 | m:razorwind.prepare | {POKEMON} whipped up a whirlwind! | A | prepare side,mon,move | no: unsupported mechanic: Razor Wind (prepare) |
| BT-1724 | m:recycle.addItem | {POKEMON} found one {ITEM}! | D2 | item side,mon,item,what=received,from=? ("found one ITEM!" shared; Recycle vs Pickup) | no: unsupported mechanic: Pickup (addItem) or Recycl |
| BT-1728 | m:reflect.end | {TEAM}'s Reflect wore off! | A | side side,effect,what=end | no: unsupported mechanic: Reflect (end) |
| BT-1730 | m:reflect.start | Reflect made {TEAM} stronger against physical moves! | A | side side,effect,what=start | yes: hazard |
| BT-1733 | m:reflecttype.typeChange | {POKEMON}'s type became the same as {SOURCE}'s type! | P | types side,mon (type not named) | no: unsupported mechanic: Reflect Type (typeChange) |
| BT-1751 | m:revivalblessing.heal | {POKEMON} was revived and is ready to fight again! | A | revive side,mon | yes: revive |
| BT-1774 | m:roleplay.changeAbility | {POKEMON} copied {SOURCE}'s {ABILITY} Ability! | A | ability side,mon,ability | no: unsupported mechanic: Role Play (changeAbility) |
| BT-1783 | m:roost.start | ({POKEMON} loses Flying type this turn.) | P | types side,mon (Flying removal; resulting types not named) | no: unsupported mechanic: Roost (start) |
| BT-1794 | m:safeguard.block | {POKEMON} is protected by Safeguard! | B | (none) (status blocked by side effect) | no: unsupported mechanic: Safeguard (block) |
| BT-1796 | m:safeguard.end | {TEAM} is no longer protected by Safeguard! | A | side side,effect,what=end | no: unsupported mechanic: Safeguard (end) |
| BT-1798 | m:safeguard.start | {TEAM} cloaked itself in a mystical veil! | A | side side,effect,what=start | no: unsupported mechanic: Safeguard (start) |
| BT-1799 | m:saltcure.damage | {POKEMON} is hurt by Salt Cure! | A | hp side,mon,from=saltcure | yes: cause |
| BT-1802 | m:saltcure.start | {POKEMON} is being salt cured! | A | volatile effect=saltcure | no: unsupported mechanic: Salt Cure (start) |
| BT-1811 | m:sandtomb.start | {POKEMON} became trapped by the quicksand! | A | volatile side,mon,effect=<trapping move> | no: unsupported mechanic: Sand Tomb (start) |
| BT-1847 | m:shadowforce.activate | It broke through {TARGET}'s protection! | D2 | (none) ("broke through TARGET's protection!" shared; S5-adjacent (Protect shown)) | no: unsupported mechanic: Hyperspace Fury (activate) |
| BT-1849 | m:shadowforce.prepare | {POKEMON} vanished instantly! | D2 | prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) | yes: prepare |
| BT-1859 | m:shedtail.alreadyStarted | {POKEMON} already has a substitute! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | yes: trailing |
| BT-1861 | m:shedtail.fail | But it does not have enough HP left to make a substitute! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Shed Tail (fail) or Substi |
| BT-1863 | m:shedtail.start | {POKEMON} shed its tail to create a decoy! | A | volatile side,mon,effect=substitute | yes: volatile |
| BT-1870 | m:shelltrap.cant | {POKEMON}'s shell trap didn't work! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Shell Trap (cant) |
| BT-1872 | m:shelltrap.prepare | {POKEMON} set a shell trap! | D1 | once vs prepare (same text as m:shelltrap.start) | no: unsupported mechanic: Shell Trap (prepare) or Sh |
| BT-1874 | m:shelltrap.start | {POKEMON} set a shell trap! | D1 | once vs prepare (same text as m:shelltrap.prepare) | no: unsupported mechanic: Shell Trap (prepare) or Sh |
| BT-1894 | m:sketch.activate | {POKEMON} sketched {MOVE}! | B | (none) (moveset changed) | no: unsupported mechanic: Sketch (activate) |
| BT-1897 | m:skillswap.activate | {POKEMON} swapped Abilities with its target! | D2 | (none) ("swapped Abilities with its target!" shared; no ability named) | no: unsupported mechanic: Skill Swap (activate) or W |
| BT-1903 | m:skullbash.prepare | {POKEMON} tucked in its head! | A | prepare side,mon,move | no: unsupported mechanic: Skull Bash (prepare) |
| BT-1906 | m:skyattack.prepare | {POKEMON} became cloaked in a harsh light! | A | prepare side,mon,move | no: unsupported mechanic: Sky Attack (prepare) |
| BT-1909 | m:skydrop.end | {POKEMON} was freed from the Sky Drop! | A | volatile_end effect=skydrop | no: unsupported mechanic: Sky Drop (end) |
| BT-1910 | m:skydrop.failSelect | Sky Drop won't let {POKEMON} go! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Sky Drop (failSelect) |
| BT-1911 | m:skydrop.failTooHeavy | {POKEMON} is too heavy to be lifted! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Sky Drop (failTooHeavy) |
| BT-1912 | m:skydrop.prepare | {POKEMON} took {TARGET} into the sky! | A | prepare side,mon,move | no: unsupported mechanic: Sky Drop (prepare) |
| BT-1932 | m:smackdown.start | {POKEMON} fell straight down! | A | volatile effect=smackdown | no: unsupported mechanic: Smack Down (start) |
| BT-1942 | m:snaptrap.start | {POKEMON} got trapped by a snap trap! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-1945 | m:snatch.activate | {POKEMON} snatched {TARGET}'s move! | B | (none) (move order / interception text) | no: unsupported mechanic: Snatch (activate) |
| BT-1948 | m:snatch.start | {POKEMON} is waiting for a target to make a move! | A | once side,mon,effect | no: unsupported mechanic: Snatch (start) |
| BT-1960 | m:solarbeam.prepare | {POKEMON} absorbed light! | D2 | prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) | yes: prepare |
| BT-1963 | m:solarblade.prepare | {POKEMON} absorbed light! | D2 | prepare side,mon,move=? (text shared by two moves (Phantom/Shadow Force; Solar Beam/Blade)) | yes: prepare |
| BT-1976 | m:spectralthief.clearBoost | {SOURCE} stole the target's boosted stats! | B | (none) (boosts stolen) | no: unsupported mechanic: Spectral Thief (clearBoost |
| BT-1979 | m:speedswap.activate | {POKEMON} switched Speed with its target! | B | (none) (stat swap) | no: unsupported mechanic: Speed Swap (activate) |
| BT-1988 | m:spikes.damage | {POKEMON} was hurt by the spikes! | A | hp side,mon,from=spikes/stealthrock | yes: cause |
| BT-1990 | m:spikes.end | The spikes disappeared from the ground around {TEAM}! | A | side side,effect,what=end | yes: hazard |
| BT-1992 | m:spikes.start | Spikes were scattered on the ground all around {TEAM}! | A | side side,effect,what=start | yes: hazard |
| BT-1993 | m:spikyshield.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? ("was hurt!" shared) | yes: trailing |
| BT-2002 | m:spite.activate | It reduced the PP of {TARGET}'s {MOVE} by {NUMBER}! | B | (none) (S5: PP of a named move is shown) | no: unsupported mechanic: Eerie Spell (activate) or  |
| BT-2007 | m:splash.activate | But nothing happened! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Splash (activate) or Teati |
| BT-2016 | m:spotlight.start | {POKEMON} became the center of attention! | D2 | volatile / once effect=? ("became the center of attention!" shared (Follow Me, Rage Powder, Spotlight)) | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-2017 | m:spotlight.startFromZEffect | {POKEMON} became the center of attention! | E | Z-Move | no: unsupported mechanic: Follow Me (start) or Follo |
| BT-2020 | m:stealthrock.damage | Pointed stones dug into {POKEMON}! | A | hp side,mon,from=spikes/stealthrock | yes: cause |
| BT-2022 | m:stealthrock.end | The pointed stones disappeared from around {TEAM}! | A | side side,effect,what=end | yes: hazard |
| BT-2024 | m:stealthrock.start | Pointed stones float in the air around {TEAM}! | A | side side,effect,what=start | no: unsupported mechanic: Stealth Rock (start) |
| BT-2029 | m:steelbeam.damage | ({POKEMON} cut its own HP to power up its move!) | D2 | hp side,mon,from=move (same text for two moves) | no: unsupported mechanic: Mind Blown (damage) or Ste |
| BT-2036 | m:stickyweb.activate | {POKEMON} was caught in a sticky web! | A | volatile_hit side,mon,effect=stickyweb | yes: activate |
| BT-2038 | m:stickyweb.end | The sticky web has disappeared from the ground around {TEAM}! | A | side side,effect,what=end | yes: hazard |
| BT-2040 | m:stickyweb.start | A sticky web has been laid out on the ground around {TEAM}! | A | side side,effect,what=start | yes: hazard |
| BT-2042 | m:stockpile.end | {POKEMON}'s stockpiled effect wore off! | A | volatile_end effect=stockpile | no: unsupported mechanic: Stockpile (end) |
| BT-2044 | m:stockpile.start | {POKEMON} stockpiled {NUMBER}! | A | volatile side,mon,effect=stockpile,detail=N | no: unsupported mechanic: Stockpile (start) |
| BT-2076 | m:substitute.activate | The substitute took damage for {POKEMON}! | A | volatile_hit side,mon,effect=substitute | yes: volatile_active |
| BT-2077 | m:substitute.alreadyStarted | {POKEMON} already has a substitute! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | yes: trailing |
| BT-2079 | m:substitute.end | {POKEMON}'s substitute faded! | A | volatile_end effect=substitute | yes: volatile_end |
| BT-2080 | m:substitute.fail | But it does not have enough HP left to make a substitute! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Shed Tail (fail) or Substi |
| BT-2082 | m:substitute.start | {POKEMON} put in a substitute! | A | volatile side,mon,effect=substitute | yes: volatile |
| BT-2090 | m:supercellslam.damage | {POKEMON} kept going and crashed! | D2 | hp side,mon,from=crash/move (same text as d:crash.damage) | yes: cause |
| BT-2112 | m:switcheroo.activate | {POKEMON} switched items with its target! | D2 | (none) ("switched items with its target!" shared; no item named) | yes: trailing |
| BT-2123 | m:syrupbomb.start | {POKEMON} got covered in sticky candy syrup! | A | volatile side,mon,effect | no: unsupported mechanic: Syrup Bomb (start) |
| BT-2134 | m:tailwind.end | {TEAM}'s Tailwind petered out! | A | side side,effect,what=end | yes: hazard |
| BT-2136 | m:tailwind.start | The Tailwind blew from behind {TEAM}! | A | side side,effect,what=start | yes: hazard |
| BT-2143 | m:tarshot.start | {POKEMON} became weaker to fire! | A | volatile side,mon,effect | no: unsupported mechanic: Tar Shot (start) |
| BT-2144 | m:taunt.cant | {POKEMON} can't use {MOVE} after the taunt! | A | cant side,mon,reason=taunt | yes: cant |
| BT-2146 | m:taunt.end | {POKEMON} shook off the taunt! | A | volatile_end effect=taunt | yes: volatile_end |
| BT-2148 | m:taunt.start | {POKEMON} fell for the taunt! | A | volatile effect=taunt | yes: volatile |
| BT-2151 | m:teatime.activate | It's teatime! Everyone dug in to their Berries! | B | (none) (berries eaten by everyone) | no: unsupported mechanic: Teatime (activate) |
| BT-2153 | m:teatime.fail | But nothing happened! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Splash (activate) or Teati |
| BT-2161 | m:telekinesis.end | {POKEMON} was freed from the telekinesis! | A | volatile_end effect | no: unsupported mechanic: Telekinesis (end) |
| BT-2163 | m:telekinesis.start | {POKEMON} was hurled into the air! | A | volatile side,mon,effect | no: unsupported mechanic: Telekinesis (start) |
| BT-2182 | m:throatchop.cant | The effects of Throat Chop prevent {POKEMON} from using certain moves! | A | cant side,mon,reason=throatchop | no: unsupported mechanic: Throat Chop (cant) |
| BT-2191 | m:thundercage.start | {SOURCE} trapped {POKEMON}! | A | volatile side,mon,effect=<trapping move> | no: no_pattern |
| BT-2206 | m:tidyup.activate | Tidying up complete! | B | (none) (side effects moved or cleared; which not named) | no: unsupported mechanic: Tidy Up (activate) |
| BT-2214 | m:torment.end | {POKEMON} is no longer tormented! | A | volatile_end effect=torment | no: unsupported mechanic: Torment (end) |
| BT-2216 | m:torment.start | {POKEMON} was subjected to torment! | A | volatile effect=torment | no: unsupported mechanic: Torment (start) |
| BT-2220 | m:toxicspikes.end | The poison spikes disappeared from the ground around {TEAM}! | A | side side,effect,what=end | yes: hazard |
| BT-2222 | m:toxicspikes.start | Poison spikes were scattered on the ground all around {TEAM}! | A | side side,effect,what=start | yes: hazard |
| BT-2229 | m:transform.transform | {POKEMON} transformed into {SPECIES}! | A | transform side,mon,into (names the species, not the side) | yes: transform |
| BT-2232 | m:trick.activate | {POKEMON} switched items with its target! | D2 | (none) ("switched items with its target!" shared; no item named) | yes: trailing |
| BT-2260 | m:uproar.block | But the uproar kept {POKEMON} awake! | B | (none) (sleep blocked by uproar) | no: unsupported mechanic: Uproar (block) |
| BT-2261 | m:uproar.blockSelf | But {POKEMON} can't sleep in an uproar! | P | fail (wording differs ("nothing happened", "not enough HP", ...)) | no: unsupported mechanic: Uproar (blockSelf) |
| BT-2263 | m:uproar.end | {POKEMON} calmed down. | A | volatile_end effect=uproar | no: unsupported mechanic: Uproar (end) |
| BT-2265 | m:uproar.start | {POKEMON} caused an uproar! | A | volatile effect=uproar | no: unsupported mechanic: Uproar (start) |
| BT-2266 | m:uproar.upkeep | {POKEMON} is making an uproar! | A | volatile_hit effect=uproar | no: unsupported mechanic: Uproar (upkeep) |
| BT-2269 | m:uturn.switchOut | {POKEMON} went back to {TRAINER}! | D2 | (none) ("went back to TRAINER": withdrawal of four moves; also no kind) | yes: trailing |
| BT-2288 | m:voltswitch.switchOut | {POKEMON} went back to {TRAINER}! | D2 | (none) ("went back to TRAINER": withdrawal of four moves; also no kind) | yes: trailing |
| BT-2296 | m:waterpledge.activate | {POKEMON} is waiting for {TARGET}'s move... | E | pledge combination with an ally | no: unsupported mechanic: Fire Pledge (activate) or  |
| BT-2298 | m:waterpledge.end | The rainbow on {TEAM}'s side disappeared! | A | side side,effect,what=end | no: unsupported mechanic: Water Pledge (end) |
| BT-2300 | m:waterpledge.start | A rainbow appeared in the sky on {TEAM}'s side! | A | side side,effect,what=start | no: unsupported mechanic: Water Pledge (start) |
| BT-2312 | m:weatherball.move | Breakneck Blitz turned into {MOVE} due to the weather! | E | G-Max / Z-Move text | no: unsupported mechanic: Weather Ball (move) |
| BT-2316 | m:whirlpool.start | {POKEMON} became trapped in the vortex! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-2323 | m:wideguard.block | Wide Guard protected {POKEMON}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Wide Guard (block) |
| BT-2326 | m:wideguard.start | Wide Guard protected {TEAM}! | B | (none) (S5-adjacent: a Protect-like block is shown) | no: unsupported mechanic: Wide Guard (block) or Wide |
| BT-2335 | m:wish.heal | {NICKNAME}'s wish came true! | A | hp side,mon,from=wish | yes: cause |
| BT-2348 | m:wrap.move | {POKEMON}'s attack continues! | D2 | (none) ("attack continues" for four trapping moves; also no kind) | no: unsupported mechanic: Bind (move) or Clamp (move |
| BT-2350 | m:wrap.start | {POKEMON} was wrapped by {SOURCE}! | A | volatile side,mon,effect=<trapping move> | yes: volatile |
| BT-2356 | m:yawn.start | {POKEMON} grew drowsy! | A | volatile side,mon,effect=yawn | yes: volatile |
| BT-2369 | a:aftermath.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? + ability ("was hurt!" shared; cause is one of five abilities) | yes: trailing |
| BT-2373 | a:airlock.start | The effects of the weather disappeared. | D2 | ability side,mon,ability=? ("effects of the weather disappeared" shared) | yes: trailing |
| BT-2376 | a:angerpoint.boost | {POKEMON} maxed its Attack! | P | boost side,mon,stat=atk (to +6; by not given) + ability | no: unsupported mechanic: Anger Point (boost) |
| BT-2381 | a:anticipation.activate | {POKEMON} shuddered! | A | ability side,mon,ability | no: unsupported mechanic: Anticipation (activate) |
| BT-2386 | a:armortail.block | {SOURCE} cannot use {MOVE}! | D2 | ability side,mon,ability=? ("SOURCE cannot use MOVE!" shared by four abilities) | no: unsupported mechanic: Armor Tail (block) or Damp |
| BT-2389 | a:aromaveil.block | {POKEMON} is protected by an aromatic veil! | A | ability side,mon,ability (disguise.block names no Pokemon (P)) | no: unsupported mechanic: Aroma Veil (block) |
| BT-2393 | a:asone.start | {POKEMON} has two Abilities! | A | ability side,mon,ability | no: unsupported mechanic: As One (start) |
| BT-2398 | a:aurabreak.start | {POKEMON} reversed all other Pokémon's auras! | A | ability side,mon,ability | no: unsupported mechanic: Aura Break (start) |
| BT-2400 | a:baddreams.damage | {POKEMON} is tormented! | A | hp side,mon,from=baddreams + ability | no: unsupported mechanic: Bad Dreams (damage) |
| BT-2406 | a:battlebond.activate | {POKEMON} became fully charged due to its bond with its Trainer! | A | ability side,mon,ability | no: unsupported mechanic: Battle Bond (activate) |
| BT-2409 | a:battlebond.transform | {POKEMON} became Ash-Greninja! | A | form side,mon,form[,temporary] | no: unsupported mechanic: Battle Bond (transform) |
| BT-2411 | a:beadsofruin.start | {POKEMON}'s Beads of Ruin weakened the Sp. Def of all surrounding Pokémon! | A | ability side,mon,ability | no: unsupported mechanic: Beads of Ruin (start) |
| BT-2428 | a:cloudnine.start | The effects of the weather disappeared. | D2 | ability side,mon,ability=? ("effects of the weather disappeared" shared) | yes: trailing |
| BT-2433 | a:comatose.start | {POKEMON} is drowsing! | A | ability side,mon,ability | no: unsupported mechanic: Comatose (start) |
| BT-2434 | a:commander.activate | {POKEMON} was swallowed by {TARGET} and became {TARGET}'s commander! | E | ally / Tera text | no: unsupported mechanic: Commander (activate) |
| BT-2451 | a:damp.block | {SOURCE} cannot use {MOVE}! | D2 | ability side,mon,ability=? ("SOURCE cannot use MOVE!" shared by four abilities) | no: unsupported mechanic: Armor Tail (block) or Damp |
| BT-2458 | a:darkaura.start | {POKEMON} is radiating a dark aura! | A | ability side,mon,ability | no: unsupported mechanic: Dark Aura (start) |
| BT-2460 | a:dazzling.block | {SOURCE} cannot use {MOVE}! | D2 | ability side,mon,ability=? ("SOURCE cannot use MOVE!" shared by four abilities) | no: unsupported mechanic: Armor Tail (block) or Damp |
| BT-2471 | a:disguise.block | Its disguise served it as a decoy! | P | ability side,mon,ability=disguise (names no Pokemon) | no: unsupported mechanic: Disguise (block) |
| BT-2474 | a:disguise.transform | {POKEMON}'s disguise was busted! | A | form side,mon,form[,temporary] | yes: form_change |
| BT-2482 | a:dryskin.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? + ability ("was hurt!" shared; cause is one of five abilities) | yes: trailing |
| BT-2494 | a:electromorphosis.start | Being hit by {MOVE} charged {POKEMON} with power! | D2 | volatile effect=charge + ability ("Being hit by MOVE charged X with power!" shared) | no: unsupported mechanic: Electromorphosis (start) o |
| BT-2495 | a:embodyaspectcornerstone.boost | The Cornerstone Mask worn by {POKEMON} shone brilliantly, and {POKEMON}'s Defense rose! | A | boost side,mon,stat,by=+1 + ability | yes: boost |
| BT-2497 | a:embodyaspecthearthflame.boost | The Hearthflame Mask worn by {POKEMON} shone brilliantly, and {POKEMON}'s Attack rose! | A | boost side,mon,stat,by=+1 + ability | yes: boost |
| BT-2499 | a:embodyaspectteal.boost | The Teal Mask worn by {POKEMON} shone brilliantly, and {POKEMON}'s Speed rose! | A | boost side,mon,stat,by=+1 + ability | yes: boost |
| BT-2501 | a:embodyaspectwellspring.boost | The Wellspring Mask worn by {POKEMON} shone brilliantly, and {POKEMON}'s Sp. Def rose! | A | boost side,mon,stat,by=+1 + ability | yes: boost |
| BT-2507 | a:fairyaura.start | {POKEMON} is radiating a fairy aura! | A | ability side,mon,ability | yes: ability |
| BT-2515 | a:flashfire.start | The power of {POKEMON}'s Fire-type moves rose! | A | volatile side,mon,effect=flashfire + ability | yes: ability |
| BT-2518 | a:flowerveil.block | {POKEMON} surrounded itself with a veil of petals! | A | ability side,mon,ability (disguise.block names no Pokemon (P)) | no: unsupported mechanic: Flower Veil (block) |
| BT-2525 | a:forewarn.activate | {TARGET}'s {MOVE} was revealed! | B | (none) (S5: an unused move of the opponent is named) | no: unsupported mechanic: Forewarn (activate) |
| BT-2526 | a:forewarn.activateNoTarget | {POKEMON}'s Forewarn alerted it to {MOVE}! | B | (none) (S5: an unused move of the opponent is named) | no: unsupported mechanic: Forewarn (activateNoTarget |
| BT-2530 | a:frisk.activate | {POKEMON} frisked {TARGET} and found its {ITEM}! | A | item side,mon,item,what=shown + ability (S5: names an item the opponent held unrevealed until now) | yes: ability |
| BT-2531 | a:frisk.activateNoTarget | {POKEMON} frisked its target and found one {ITEM}! | A | item side,mon,item,what=shown + ability (S5: names an item the opponent held unrevealed until now) | no: unsupported mechanic: Frisk (activateNoTarget) |
| BT-2554 | a:hadronengine.activate | {POKEMON} used the Electric Terrain to energize its futuristic engine! | A | ability side,mon,ability | yes: move |
| BT-2556 | a:hadronengine.start | {POKEMON} turned the ground into Electric Terrain, energizing its futuristic engine! | A | field effect=electricterrain,what=start + ability | no: unsupported mechanic: Hadron Engine (start) |
| BT-2557 | a:harvest.addItem | {POKEMON} harvested one {ITEM}! | A | item side,mon,item,what=received + ability (Champions text differs) | yes: item_reveal |
| BT-2567 | a:hospitality.heal | {POKEMON} drank down all the matcha that {SOURCE} made! | E | ally / Tera text | no: unsupported mechanic: Hospitality (heal) |
| BT-2585 | a:illusion.end | {POKEMON}'s illusion wore off! | B | (none) (illusion broken) | yes: illusion_end |
| BT-2592 | a:innardsout.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? + ability ("was hurt!" shared; cause is one of five abilities) | yes: trailing |
| BT-2601 | a:ironbarbs.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? + ability ("was hurt!" shared; cause is one of five abilities) | yes: trailing |
| BT-2619 | a:lightningrod.activate | {POKEMON} took the attack! | D2 | ability side,mon,ability=? ("took the attack!" shared) | no: unsupported mechanic: Lightning Rod (activate) o |
| BT-2623 | a:lingeringaroma.changeAbility | A lingering aroma clings to {TARGET}! | A | ability side,mon,ability | no: unsupported mechanic: Lingering Aroma (changeAbi |
| BT-2626 | a:liquidooze.damage | {POKEMON} sucked up the liquid ooze! | A | hp side,mon,from=liquidooze + ability | no: unsupported mechanic: Liquid Ooze (damage) |
| BT-2632 | a:magicbounce.move | {POKEMON} bounced the {MOVE} back! | D1 | move bounced: once/ability/(none) ("bounced the MOVE back!": Magic Coat, Magic Bounce, Rebound) | no: unsupported mechanic: Magic Bounce (move) or Mag |
| BT-2646 | a:mimicry.activate | {POKEMON} returned to its original type! | P | types side,mon (original types not named) | no: unsupported mechanic: Mimicry (activate) |
| BT-2658 | a:moldbreaker.start | {POKEMON} breaks the mold! | A | ability side,mon,ability | yes: ability |
| BT-2668 | a:mummy.changeAbility | {TARGET}'s Ability became Mummy! | A | ability side,mon,ability | no: unsupported mechanic: Mummy (changeAbility) |
| BT-2673 | a:naturalcure.activate | ({POKEMON} is cured by its Natural Cure!) | A | ability side,mon,ability=naturalcure + cure (status not named) | no: unsupported mechanic: Natural Cure (activate) |
| BT-2678 | a:neutralizinggas.end | The effects of the neutralizing gas wore off! | B | (none) (ability suppression ended) | no: unsupported mechanic: Neutralizing Gas (end) |
| BT-2680 | a:neutralizinggas.start | Neutralizing gas filled the area! | A | ability side,mon,ability | no: unsupported mechanic: Neutralizing Gas (start) |
| BT-2688 | a:orichalcumpulse.activate | {POKEMON} basked in the sunlight, sending its ancient pulse into a frenzy! | A | ability side,mon,ability | no: unsupported mechanic: Orichalcum Pulse (activate |
| BT-2690 | a:orichalcumpulse.start | {POKEMON} turned the sunlight harsh, sending its ancient pulse into a frenzy! | A | weather weather=sunnyday + ability | no: unsupported mechanic: Orichalcum Pulse (start) |
| BT-2703 | a:perishbody.start | Both Pokémon will faint in three turns! | A | ability side,mon,ability | no: unsupported mechanic: Perish Body (start) |
| BT-2704 | a:persistent.activate | {POKEMON} extends {MOVE} by 2 turns! | B | (none) (S5-adjacent: turns added) | no: unsupported mechanic: Persistent (activate) |
| BT-2709 | a:pickup.addItem | {POKEMON} found one {ITEM}! | D2 | item side,mon,item,what=received,from=? ("found one ITEM!" shared; Recycle vs Pickup) | no: unsupported mechanic: Pickup (addItem) or Recycl |
| BT-2724 | a:powerconstruct.activate | You sense the presence of many! | B | (none) (no Pokemon named) | no: unsupported mechanic: Power Construct (activate) |
| BT-2727 | a:powerconstruct.transform | {POKEMON} transformed into its Complete Forme! | A | form side,mon,form[,temporary] | yes: transform |
| BT-2728 | a:powerofalchemy.changeAbility | {SOURCE}'s {ABILITY} was taken over! | E | ally / Tera text | no: unsupported mechanic: Power of Alchemy (changeAb |
| BT-2737 | a:pressure.start | {POKEMON} is exerting its pressure! | A | ability side,mon,ability | yes: ability |
| BT-2744 | a:protosynthesis.activate | The harsh sunlight activated {POKEMON}'s Protosynthesis! | A | ability side,mon,ability | no: unsupported mechanic: Protosynthesis (activate) |
| BT-2745 | a:protosynthesis.activateFromItem | {POKEMON} used its Booster Energy to activate Protosynthesis! | A | ability + item what=used | yes: move |
| BT-2747 | a:protosynthesis.end | The effects of {POKEMON}'s Protosynthesis wore off! | A | volatile_end side,mon,effect | no: unsupported mechanic: Protosynthesis (end) |
| BT-2749 | a:protosynthesis.start | {POKEMON}'s {STAT} was heightened! | D2 | volatile side,mon,effect=protosynthesis/quarkdrive,detail=stat (S5: reveals the opponent's highest stat; text shared) | no: unsupported mechanic: Protosynthesis (start) or  |
| BT-2756 | a:quarkdrive.activate | The Electric Terrain activated {POKEMON}'s Quark Drive! | A | ability side,mon,ability | no: unsupported mechanic: Quark Drive (activate) |
| BT-2757 | a:quarkdrive.activateFromItem | {POKEMON} used its Booster Energy to activate its Quark Drive! | A | ability + item what=used | yes: move |
| BT-2759 | a:quarkdrive.end | The effects of {POKEMON}'s Quark Drive wore off! | A | volatile_end side,mon,effect | no: unsupported mechanic: Quark Drive (end) |
| BT-2761 | a:quarkdrive.start | {POKEMON}'s {STAT} was heightened! | D2 | volatile side,mon,effect=protosynthesis/quarkdrive,detail=stat (S5: reveals the opponent's highest stat; text shared) | no: unsupported mechanic: Protosynthesis (start) or  |
| BT-2762 | a:queenlymajesty.block | {SOURCE} cannot use {MOVE}! | D2 | ability side,mon,ability=? ("SOURCE cannot use MOVE!" shared by four abilities) | no: unsupported mechanic: Armor Tail (block) or Damp |
| BT-2765 | a:quickdraw.activate | Quick Draw made {POKEMON} move faster! | A | ability side,mon,ability | no: unsupported mechanic: Quick Draw (activate) |
| BT-2774 | a:rebound.move | {POKEMON} bounced the {MOVE} back! | D1 | move bounced: once/ability/(none) ("bounced the MOVE back!": Magic Coat, Magic Bounce, Rebound) | no: unsupported mechanic: Magic Bounce (move) or Mag |
| BT-2776 | a:receiver.changeAbility | {SOURCE}'s {ABILITY} was taken over! | E | ally / Tera text | no: unsupported mechanic: Power of Alchemy (changeAb |
| BT-2792 | a:roughskin.damage | {POKEMON} was hurt! | D2 | hp side,mon,from=? + ability ("was hurt!" shared; cause is one of five abilities) | yes: trailing |
| BT-2808 | a:schooling.transform | {POKEMON} formed a school! | A | form side,mon,form[,temporary] | no: unsupported mechanic: Schooling (transform) |
| BT-2809 | a:schooling.transformEnd | {POKEMON} stopped schooling! | A | form side,mon,form[,temporary] | no: unsupported mechanic: Schooling (transformEnd) |
| BT-2829 | a:shieldsdown.transform | Shields Down deactivated! / ({POKEMON} shielded itself.) | A | form side,mon,form[,temporary] | no: no_pattern |
| BT-2830 | a:shieldsdown.transformEnd | Shields Down activated! / ({POKEMON} stopped shielding itself.) | A | form side,mon,form[,temporary] | no: no_pattern |
| BT-2834 | a:slowstart.end | {POKEMON} finally got its act together! | A | volatile_end effect=slowstart | no: unsupported mechanic: Slow Start (end) |
| BT-2836 | a:slowstart.start | {POKEMON} can't get it going! | A | ability side,mon,ability | no: unsupported mechanic: Slow Start (start) |
| BT-2856 | a:stancechange.transform | Changed to Blade Forme! | P | form side,mon,form (no Pokemon named) | no: unsupported mechanic: Stance Change (transform) |
| BT-2857 | a:stancechange.transformEnd | Changed to Shield Forme! | P | form side,mon,form (no Pokemon named) | no: unsupported mechanic: Stance Change (transformEn |
| BT-2867 | a:stickyhold.block | {POKEMON}'s item cannot be removed! | A | ability side,mon,ability (disguise.block names no Pokemon (P)) | no: unsupported mechanic: Sticky Hold (block) |
| BT-2870 | a:stormdrain.activate | {POKEMON} took the attack! | D2 | ability side,mon,ability=? ("took the attack!" shared) | no: unsupported mechanic: Lightning Rod (activate) o |
| BT-2875 | a:sturdy.activate | {POKEMON} endured the hit! | D1 | volatile_hit (Endure) / ability (Sturdy) ("endured the hit!" shared) | yes: trailing |
| BT-2878 | a:suctioncups.block | {POKEMON} is anchored in place with its suction cups! | A | ability side,mon,ability (disguise.block names no Pokemon (P)) | no: unsupported mechanic: Suction Cups (block) |
| BT-2882 | a:supersweetsyrup.start | A supersweet aroma is wafting from the syrup covering {POKEMON}! | A | ability side,mon,ability | no: unsupported mechanic: Supersweet Syrup (start) |
| BT-2883 | a:supremeoverlord.activate | {POKEMON} gained strength from the fallen! | A | ability side,mon,ability | yes: ability |
| BT-2889 | a:sweetveil.block | {POKEMON} can't fall asleep due to a veil of sweetness! | A | ability side,mon,ability (disguise.block names no Pokemon (P)) | no: unsupported mechanic: Sweet Veil (block) |
| BT-2895 | a:swordofruin.start | {POKEMON}'s Sword of Ruin weakened the Defense of all surrounding Pokémon! | A | ability side,mon,ability | no: unsupported mechanic: Sword of Ruin (start) |
| BT-2896 | a:symbiosis.activate | {POKEMON} shared its {ITEM} with {TARGET}! | E | ally / Tera text | no: unsupported mechanic: Symbiosis (activate) |
| BT-2902 | a:tabletsofruin.start | {POKEMON}'s Tablets of Ruin weakened the Attack of all surrounding Pokémon! | A | ability side,mon,ability | no: unsupported mechanic: Tablets of Ruin (start) |
| BT-2907 | a:telepathy.block | {POKEMON} can't be hit by attacks from its ally Pokémon! | E | ally / Tera text | no: unsupported mechanic: Telepathy (block) |
| BT-2910 | a:terashell.activate | {POKEMON} made its shell gleam! It's distorting type matchups! | E | ally / Tera text | no: unsupported mechanic: Tera Shell (activate) |
| BT-2914 | a:terashift.transform | {POKEMON} transformed! | D1 | form vs transform (same text as d:default.transform) | no: unsupported mechanic: Tera Shift (transform) or  |
| BT-2917 | a:teravolt.start | {POKEMON} is radiating a bursting aura! | A | ability side,mon,ability | no: unsupported mechanic: Teravolt (start) |
| BT-2931 | a:trace.changeAbility | {POKEMON} traced {SOURCE}'s {ABILITY}! | A | ability side,mon,ability,changed_from=trace | no: no_pattern |
| BT-2936 | a:truant.cant | {POKEMON} is loafing around! | A | cant side,mon,reason=truant | no: unsupported mechanic: Truant (cant) |
| BT-2940 | a:turboblaze.start | {POKEMON} is radiating a blazing aura! | A | ability side,mon,ability | no: unsupported mechanic: Turboblaze (start) |
| BT-2947 | a:unnerve.start | {TEAM} is too nervous to eat Berries! | A | ability side,mon,ability | no: unsupported mechanic: Unnerve (start) |
| BT-2950 | a:vesselofruin.start | {POKEMON}'s Vessel of Ruin weakened the Sp. Atk of all surrounding Pokémon! | A | ability side,mon,ability | no: unsupported mechanic: Vessel of Ruin (start) |
| BT-2955 | a:wanderingspirit.activate | {POKEMON} swapped Abilities with its target! | D2 | (none) ("swapped Abilities with its target!" shared; no ability named) | no: unsupported mechanic: Skill Swap (activate) or W |
| BT-2973 | a:windpower.start | Being hit by {MOVE} charged {POKEMON} with power! | D2 | volatile effect=charge + ability ("Being hit by MOVE charged X with power!" shared) | no: unsupported mechanic: Electromorphosis (start) o |
| BT-2981 | a:zenmode.transform | Zen Mode triggered! | P | form side,mon,form (no Pokemon named) | no: unsupported mechanic: Zen Mode (transform) |
| BT-2982 | a:zenmode.transformEnd | Zen Mode ended! | P | form side,mon,form (no Pokemon named) | no: unsupported mechanic: Zen Mode (transformEnd) |
| BT-2983 | a:zerotohero.activate | {POKEMON} underwent a heroic transformation! | A | form side,mon,form[,temporary] | yes: form_change |
| BT-2985 | i:abilityshield.block | {POKEMON}'s Ability is protected by the effects of its Ability Shield! | A | item side,mon,item,what=shown | no: unsupported mechanic: Ability Shield (block) |
| BT-2997 | i:airballoon.end | {POKEMON}'s Air Balloon popped! | A | item side,mon,item,what=removed | yes: item_end |
| BT-2999 | i:airballoon.start | {POKEMON} floats in the air with its Air Balloon! | A | item side,mon,item=airballoon,what=shown | yes: item_reveal |
| BT-3028 | i:blacksludge.heal | {POKEMON} restored a little HP using its Black Sludge! | A | hp side,mon,from=item + item what=shown | no: unsupported mechanic: Black Sludge (heal) |
| BT-3064 | i:clearamulet.block | The effects of {POKEMON}'s Clear Amulet prevent its stats from being lowered! | A | item side,mon,item,what=shown | no: unsupported mechanic: Clear Amulet (block) |
| BT-3078 | i:custapberry.activate | {POKEMON} can act faster than normal, thanks to its Custap Berry! | A | item side,mon,item,what=shown/used | yes: trailing |
| BT-3117 | i:ejectbutton.end | {POKEMON} is switched out with the Eject Button! | A | item side,mon,item,what=used (+ following enter) | yes: item_end |
| BT-3119 | i:ejectpack.end | {POKEMON} is switched out by the Eject Pack! | A | item side,mon,item,what=used (+ following enter) | no: unsupported mechanic: Eject Pack (end) |
| BT-3156 | i:focusband.activate | {POKEMON} hung on using its Focus Band! | A | item side,mon,item,what=used | yes: trailing |
| BT-3158 | i:focussash.end | {POKEMON} hung on using its Focus Sash! | A | item side,mon,item,what=used | yes: trailing |
| BT-3242 | i:leftovers.heal | {POKEMON} restored a little HP using its Leftovers! | A | hp side,mon,from=item + item what=shown | no: unsupported mechanic: Leftovers (heal) |
| BT-3246 | i:leppaberry.activate | {POKEMON} restored PP to its move {MOVE} using its Leppa Berry! | P | item side,mon,item,what=used (S5/6.3: also shows PP restored for a named move (own side)) | no: unsupported mechanic: Leppa Berry (activate) |
| BT-3250 | i:lifeorb.damage | {POKEMON} lost some of its HP! | A | hp side,mon,from=item + item what=shown | yes: trailing |
| BT-3305 | i:mirrorherb.activate | {POKEMON} used its Mirror Herb to mirror its opponent's stat changes! | P | item what=used (+ boosts, not given) | yes: move |
| BT-3312 | i:mysteryberry.activate | {POKEMON} restored PP to its {MOVE} move using Mystery Berry! | P | item side,mon,item,what=used (S5/6.3: also shows PP restored for a named move (own side)) | no: unsupported mechanic: Mystery Berry (activate) |
| BT-3354 | i:powerherb.end | {POKEMON} became fully charged due to its Power Herb! | A | item side,mon,item,what=used | no: unsupported mechanic: Power Herb (end) |
| BT-3362 | i:protectivepads.block | {POKEMON} protected itself with its Protective Pads! | A | item side,mon,item,what=shown | no: unsupported mechanic: Protective Pads (block) |
| BT-3375 | i:quickclaw.activate | {POKEMON} can act faster than normal, thanks to its Quick Claw! | A | item side,mon,item,what=shown/used | yes: trailing |
| BT-3389 | i:redcard.end | {POKEMON} held up its Red Card against {TARGET}! | A | item side,mon,item,what=used (+ following enter) | yes: item_end |
| BT-3400 | i:rockyhelmet.damage | {POKEMON} was hurt by the Rocky Helmet! | A | hp side,mon,from=item + item what=shown | no: unsupported mechanic: Rocky Helmet (damage) |
| BT-3412 | i:safetygoggles.block | {POKEMON} is not affected by {MOVE} thanks to its Safety Goggles! | A | item side,mon,item,what=shown | no: unsupported mechanic: Safety Goggles (block) |
| BT-3427 | i:shellbell.heal | {POKEMON} restored a little HP using its Shell Bell! | A | hp side,mon,from=item + item what=shown | no: unsupported mechanic: Shell Bell (heal) |
| BT-3585 | i:ultranecroziumz.activate | {POKEMON} regained its true power through Ultra Burst! | E | Ultra Burst | no: unsupported mechanic: Ultranecrozium Z (activate |
| BT-3587 | i:ultranecroziumz.transform | Bright light is about to burst out of {POKEMON}! | E | Ultra Burst | no: unsupported mechanic: Ultranecrozium Z (transfor |
| BT-3607 | i:whiteherb.end | {POKEMON} returned its stats to normal using its White Herb! | A | boost_reset what=negative + item what=used | yes: clearnegativeboost |

## 7. Description entries (2910, all NOT-BATTLE, never recognised)

`shortDesc` and `desc` of every move, ability and item: reference text, not log rows. Text cut at 90 characters.

| id | key | text |
|---|---|---|
| BT-0286 | m:10000000voltthunderbolt.desc | Has a very high chance for a critical hit. |
| BT-0287 | m:10000000voltthunderbolt.shortDesc | Very high critical hit ratio. |
| BT-0288 | m:absorb.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0289 | m:absorb.shortDesc | User recovers 50% of the damage dealt. |
| BT-0290 | m:accelerock.desc | No additional effect. |
| BT-0291 | m:accelerock.shortDesc | Usually goes first. |
| BT-0292 | m:acid.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0293 | m:acid.shortDesc | 10% chance to lower the foe(s) Sp. Def by 1. |
| BT-0294 | m:acidarmor.desc | Raises the user's Defense by 2 stages. |
| BT-0295 | m:acidarmor.shortDesc | Raises the user's Defense by 2. |
| BT-0296 | m:aciddownpour.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0297 | m:acidspray.desc | Has a 100% chance to lower the target's Special Defense by 2 stages. |
| BT-0298 | m:acidspray.shortDesc | 100% chance to lower the target's Sp. Def by 2. |
| BT-0299 | m:acrobatics.shortDesc | Power doubles if the user has no held item. |
| BT-0300 | m:acupressure.desc | Raises a random stat by 2 stages as long as the stat is not already at stage 6. The use... |
| BT-0301 | m:acupressure.shortDesc | Raises a random stat of the user or an ally by 2. |
| BT-0302 | m:aerialace.shortDesc | This move does not check accuracy. |
| BT-0303 | m:aeroblast.desc | Has a higher chance for a critical hit. |
| BT-0304 | m:aeroblast.shortDesc | High critical hit ratio. |
| BT-0306 | m:afteryou.desc | The target makes its move immediately after the user this turn, no matter the priority ... |
| BT-0307 | m:afteryou.shortDesc | The target makes its move right after the user. |
| BT-0308 | m:agility.desc | Raises the user's Speed by 2 stages. |
| BT-0309 | m:agility.shortDesc | Raises the user's Speed by 2. |
| BT-0310 | m:aircutter.desc | Has a higher chance for a critical hit. |
| BT-0311 | m:aircutter.shortDesc | High critical hit ratio. Hits adjacent foes. |
| BT-0312 | m:airslash.desc | Has a 30% chance to make the target flinch. |
| BT-0313 | m:airslash.shortDesc | 30% chance to make the target flinch. |
| BT-0314 | m:alloutpummeling.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0315 | m:alluringvoice.desc | Has a 100% chance to confuse the target if it had a stat stage raised this turn. |
| BT-0316 | m:alluringvoice.shortDesc | 100% confuse target that had a stat rise this turn. |
| BT-0317 | m:allyswitch.desc | The user swaps positions with its ally. Fails if the user is the only Pokemon on its si... |
| BT-0318 | m:allyswitch.shortDesc | User and ally swap positions; using again can fail. |
| BT-0319 | m:amnesia.desc | Raises the user's Special Defense by 2 stages. |
| BT-0320 | m:amnesia.shortDesc | Raises the user's Sp. Def by 2. |
| BT-0321 | m:anchorshot.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-0322 | m:anchorshot.shortDesc | Prevents the target from switching out. |
| BT-0323 | m:ancientpower.desc | Has a 10% chance to raise the user's Attack, Defense, Special Attack, Special Defense, ... |
| BT-0324 | m:ancientpower.shortDesc | 10% chance to raise all stats by 1 (not acc/eva). |
| BT-0325 | m:appleacid.desc | Has a 100% chance to lower the target's Special Defense by 1 stage. |
| BT-0326 | m:appleacid.shortDesc | 100% chance to lower the target's Sp. Def by 1. |
| BT-0327 | m:aquacutter.desc | Has a higher chance for a critical hit. |
| BT-0328 | m:aquacutter.shortDesc | High critical hit ratio. |
| BT-0329 | m:aquajet.desc | No additional effect. |
| BT-0330 | m:aquajet.shortDesc | Usually goes first. |
| BT-0331 | m:aquaring.desc | The user has 1/16 of its maximum HP, rounded down, restored at the end of each turn whi... |
| BT-0333 | m:aquaring.shortDesc | User recovers 1/16 max HP per turn. |
| BT-0335 | m:aquastep.desc | Has a 100% chance to raise the user's Speed by 1 stage. |
| BT-0336 | m:aquastep.shortDesc | 100% chance to raise the user's Speed by 1. |
| BT-0337 | m:aquatail.shortDesc | No additional effect. |
| BT-0338 | m:armorcannon.desc | Lowers the user's Defense and Special Defense by 1 stage. |
| BT-0339 | m:armorcannon.shortDesc | Lowers the user's Defense and Sp. Def by 1. |
| BT-0340 | m:armthrust.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0341 | m:armthrust.shortDesc | Hits 2-5 times in one turn. |
| BT-0343 | m:aromatherapy.desc | Every Pokemon in the user's party is cured of its non-volatile status condition. Active... |
| BT-0344 | m:aromatherapy.shortDesc | Cures the user's party of all status conditions. |
| BT-0345 | m:aromaticmist.desc | Raises the target's Special Defense by 1 stage. Fails if there is no ally adjacent to t... |
| BT-0346 | m:aromaticmist.shortDesc | Raises an ally's Sp. Def by 1. |
| BT-0347 | m:assist.desc | A random move among those known by the user's party members is selected for use. Does n... |
| BT-0348 | m:assist.shortDesc | Uses a random move known by a team member. |
| BT-0349 | m:assurance.desc | Power doubles if the target has already taken damage this turn, other than direct damag... |
| BT-0350 | m:assurance.shortDesc | Power doubles if target was damaged this turn. |
| BT-0351 | m:astonish.desc | Has a 30% chance to make the target flinch. |
| BT-0352 | m:astonish.shortDesc | 30% chance to make the target flinch. |
| BT-0353 | m:astralbarrage.desc | No additional effect. |
| BT-0354 | m:astralbarrage.shortDesc | No additional effect. Hits adjacent foes. |
| BT-0355 | m:attackorder.desc | Has a higher chance for a critical hit. |
| BT-0356 | m:attackorder.shortDesc | High critical hit ratio. |
| BT-0359 | m:attract.desc | Causes the target to become infatuated, making it unable to attack 50% of the time. Fai... |
| BT-0362 | m:attract.shortDesc | A target of the opposite gender gets infatuated. |
| BT-0365 | m:aurasphere.shortDesc | This move does not check accuracy. |
| BT-0366 | m:aurawheel.desc | Has a 100% chance to raise the user's Speed by 1 stage. If the user is a Morpeko in Ful... |
| BT-0367 | m:aurawheel.shortDesc | Morpeko: Electric; Hangry: Dark; 100% +1 Spe. |
| BT-0368 | m:aurorabeam.desc | Has a 10% chance to lower the target's Attack by 1 stage. |
| BT-0369 | m:aurorabeam.shortDesc | 10% chance to lower the target's Attack by 1. |
| BT-0370 | m:auroraveil.desc | For 5 turns, the user and its party members take 0.5× damage from physical and special ... |
| BT-0372 | m:auroraveil.shortDesc | For 5 turns, damage to allies halved. Snow only. |
| BT-0374 | m:autotomize.desc | Raises the user's Speed by 2 stages. If the user's Speed was changed, the user's weight... |
| BT-0375 | m:autotomize.shortDesc | Raises the user's Speed by 2; user loses 100 kg. |
| BT-0377 | m:avalanche.desc | Power doubles if the user was hit by the target this turn. |
| BT-0378 | m:avalanche.shortDesc | Power doubles if user is damaged by the target. |
| BT-0380 | m:axekick.desc | Has a 30% chance to confuse the target. If this attack is not successful, the user lose... |
| BT-0381 | m:axekick.shortDesc | 30% confusion. User loses 50% max HP if miss. |
| BT-0382 | m:babydolleyes.desc | Lowers the target's Attack by 1 stage. |
| BT-0383 | m:babydolleyes.shortDesc | Lowers the target's Attack by 1. |
| BT-0384 | m:baddybad.desc | This move summons Reflect for 5 turns upon use. |
| BT-0385 | m:baddybad.shortDesc | Summons Reflect. |
| BT-0386 | m:banefulbunker.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-0387 | m:banefulbunker.shortDesc | Protects from moves. Contact: poison. |
| BT-0388 | m:barbbarrage.desc | Has a 50% chance to poison the target. Power doubles if the target is already poisoned. |
| BT-0389 | m:barbbarrage.shortDesc | 50% psn. 2× power if target already poisoned. |
| BT-0390 | m:barrage.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0391 | m:barrage.shortDesc | Hits 2-5 times in one turn. |
| BT-0392 | m:barrier.desc | Raises the user's Defense by 2 stages. |
| BT-0393 | m:barrier.shortDesc | Raises the user's Defense by 2. |
| BT-0394 | m:batonpass.desc | The user is replaced with another Pokemon in its party. The selected Pokemon has the us... |
| BT-0395 | m:batonpass.shortDesc | User switches, passing stat changes and more. |
| BT-0396 | m:beakblast.desc | If the user is hit by a contact move this turn before it can execute this move, the att... |
| BT-0397 | m:beakblast.shortDesc | Burns on contact with the user before it moves. |
| BT-0400 | m:beatup.desc | Hits one time for the user and one time for each unfainted Pokemon without a non-volati... |
| BT-0401 | m:beatup.shortDesc | All healthy allies aid in damaging the target. |
| BT-0402 | m:behemothbash.shortDesc | No additional effect. |
| BT-0403 | m:behemothblade.shortDesc | No additional effect. |
| BT-0404 | m:belch.desc | This move cannot be selected until the user eats a Berry, either by eating one that was... |
| BT-0405 | m:belch.shortDesc | Cannot be selected until the user eats a Berry. |
| BT-0407 | m:bellydrum.desc | Raises the user's Attack by 12 stages in exchange for the user losing 1/2 of its maximu... |
| BT-0408 | m:bellydrum.shortDesc | User loses 50% max HP. Maximizes Attack. |
| BT-0409 | m:bestow.desc | The target receives the user's held item. Fails if the user has no item or is holding a... |
| BT-0410 | m:bestow.shortDesc | User passes its held item to the target. |
| BT-0413 | m:bide.desc | The user spends two turns locked into this move and then, on the second turn after usin... |
| BT-0415 | m:bide.shortDesc | Waits 2 turns; deals double the damage taken. |
| BT-0417 | m:bind.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-0419 | m:bind.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-0421 | m:bite.desc | Has a 30% chance to make the target flinch. |
| BT-0422 | m:bite.shortDesc | 30% chance to make the target flinch. |
| BT-0423 | m:bitterblade.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0424 | m:bitterblade.shortDesc | User recovers 50% of the damage dealt. |
| BT-0425 | m:bittermalice.desc | Has a 100% chance to lower the target's Attack by 1 stage. |
| BT-0426 | m:bittermalice.shortDesc | 100% chance to lower the target's Attack by 1. |
| BT-0427 | m:blackholeeclipse.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0428 | m:blastburn.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-0429 | m:blastburn.shortDesc | User cannot move next turn. |
| BT-0430 | m:blazekick.desc | Has a 10% chance to burn the target and a higher chance for a critical hit. |
| BT-0431 | m:blazekick.shortDesc | High critical hit ratio. 10% chance to burn. |
| BT-0432 | m:blazingtorque.desc | Has a 30% chance to burn the target. |
| BT-0433 | m:blazingtorque.shortDesc | 30% chance to burn the target. |
| BT-0434 | m:bleakwindstorm.desc | Has a 30% chance to lower the target's Speed by 1 stage. If the weather is Heavy Rain o... |
| BT-0435 | m:bleakwindstorm.shortDesc | 30% to lower foe(s) Speed by 1. Rain: can't miss. |
| BT-0436 | m:blizzard.desc | Has a 10% chance to freeze the target. If the weather is Snow, this move does not check... |
| BT-0437 | m:blizzard.shortDesc | 10% chance to freeze foe(s). Can't miss in Snow. |
| BT-0438 | m:block.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-0439 | m:block.shortDesc | Prevents the target from switching out. |
| BT-0440 | m:bloodmoon.shortDesc | Cannot be selected the turn after it's used. |
| BT-0441 | m:bloomdoom.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0442 | m:blueflare.desc | Has a 20% chance to burn the target. |
| BT-0443 | m:blueflare.shortDesc | 20% chance to burn the target. |
| BT-0444 | m:bodypress.desc | Damage is calculated using the user's Defense stat as its Attack, including stat stage ... |
| BT-0445 | m:bodypress.shortDesc | Uses user's Def stat as Atk in damage calculation. |
| BT-0446 | m:bodyslam.desc | Has a 30% chance to paralyze the target. Damage doubles and no accuracy check is done i... |
| BT-0447 | m:bodyslam.shortDesc | 30% chance to paralyze the target. |
| BT-0448 | m:boltbeak.desc | Power doubles if the user moves before the target. |
| BT-0449 | m:boltbeak.shortDesc | Power doubles if user moves before the target. |
| BT-0450 | m:boltstrike.desc | Has a 20% chance to paralyze the target. |
| BT-0451 | m:boltstrike.shortDesc | 20% chance to paralyze the target. |
| BT-0452 | m:boneclub.desc | Has a 10% chance to make the target flinch. |
| BT-0453 | m:boneclub.shortDesc | 10% chance to make the target flinch. |
| BT-0454 | m:bonemerang.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0455 | m:bonemerang.shortDesc | Hits 2 times in one turn. |
| BT-0456 | m:bonerush.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0457 | m:bonerush.shortDesc | Hits 2-5 times in one turn. |
| BT-0458 | m:boomburst.desc | No additional effect. |
| BT-0459 | m:boomburst.shortDesc | No additional effect. Hits adjacent Pokemon. |
| BT-0460 | m:bounce.desc | Has a 30% chance to paralyze the target. This attack charges on the first turn and exec... |
| BT-0462 | m:bounce.shortDesc | Bounces turn 1. Hits turn 2. 30% paralyze. |
| BT-0463 | m:bouncybubble.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0464 | m:bouncybubble.shortDesc | User recovers 50% of the damage dealt. |
| BT-0465 | m:branchpoke.desc | No additional effect. |
| BT-0466 | m:branchpoke.shortDesc | No additional effect. |
| BT-0467 | m:bravebird.desc | If the target lost HP, the user takes recoil damage equal to 33% the HP lost by the tar... |
| BT-0468 | m:bravebird.shortDesc | Has 33% recoil. |
| BT-0469 | m:breakingswipe.desc | Has a 100% chance to lower the target's Attack by 1 stage. |
| BT-0470 | m:breakingswipe.shortDesc | 100% chance to lower the foe(s) Attack by 1. |
| BT-0471 | m:breakneckblitz.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0473 | m:brickbreak.desc | If this attack does not miss, the effects of Reflect, Light Screen, and Aurora Veil end... |
| BT-0474 | m:brickbreak.shortDesc | Destroys screens, unless the target is immune. |
| BT-0475 | m:brine.desc | Power doubles if the target has less than or equal to half of its maximum HP remaining. |
| BT-0476 | m:brine.shortDesc | Power doubles if the target's HP is 50% or less. |
| BT-0477 | m:brutalswing.desc | No additional effect. |
| BT-0478 | m:brutalswing.shortDesc | No additional effect. Hits adjacent Pokemon. |
| BT-0479 | m:bubble.desc | Has a 10% chance to lower the target's Speed by 1 stage. |
| BT-0480 | m:bubble.shortDesc | 10% chance to lower the foe(s) Speed by 1. |
| BT-0481 | m:bubblebeam.desc | Has a 10% chance to lower the target's Speed by 1 stage. |
| BT-0482 | m:bubblebeam.shortDesc | 10% chance to lower the target's Speed by 1. |
| BT-0483 | m:bugbite.desc | If this move is successful and the user has not fainted, it steals the target's held Be... |
| BT-0485 | m:bugbite.shortDesc | User steals and eats the target's Berry. |
| BT-0486 | m:bugbuzz.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0487 | m:bugbuzz.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-0488 | m:bulkup.desc | Raises the user's Attack and Defense by 1 stage. |
| BT-0489 | m:bulkup.shortDesc | Raises the user's Attack and Defense by 1. |
| BT-0490 | m:bulldoze.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-0491 | m:bulldoze.shortDesc | 100% chance lower adjacent Pkmn Speed by 1. |
| BT-0492 | m:bulletpunch.desc | No additional effect. |
| BT-0493 | m:bulletpunch.shortDesc | Usually goes first. |
| BT-0494 | m:bulletseed.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0495 | m:bulletseed.shortDesc | Hits 2-5 times in one turn. |
| BT-0496 | m:burningbulwark.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-0497 | m:burningbulwark.shortDesc | Protects from damaging attacks. Contact: burn. |
| BT-0498 | m:burningjealousy.desc | Has a 100% chance to burn the target if it had a stat stage raised this turn. |
| BT-0499 | m:burningjealousy.shortDesc | 100% burns a target that had a stat rise this turn. |
| BT-0500 | m:burnup.desc | Fails unless the user is a Fire type. If this move is successful and the user is not Te... |
| BT-0501 | m:burnup.shortDesc | User's Fire type becomes typeless; must be Fire. |
| BT-0503 | m:buzzybuzz.desc | Has a 100% chance to paralyze the foe. |
| BT-0504 | m:buzzybuzz.shortDesc | 100% chance to paralyze the foe. |
| BT-0505 | m:calmmind.desc | Raises the user's Special Attack and Special Defense by 1 stage. |
| BT-0506 | m:calmmind.shortDesc | Raises the user's Sp. Atk and Sp. Def by 1. |
| BT-0507 | m:camouflage.desc | The user's type changes based on the battle terrain. Normal type on the regular Wi-Fi t... |
| BT-0508 | m:camouflage.shortDesc | Changes user's type by terrain (default Normal). |
| BT-0509 | m:captivate.desc | Lowers the target's Special Attack by 2 stages. The target is unaffected if both the us... |
| BT-0510 | m:captivate.shortDesc | Lowers the foe(s) Sp. Atk by 2 if opposite gender. |
| BT-0511 | m:catastropika.shortDesc | No additional effect. |
| BT-0512 | m:ceaselessedge.desc | If this move is successful, it sets up a hazard on the opposing side of the field, dama... |
| BT-0513 | m:ceaselessedge.shortDesc | Sets a layer of Spikes on the opposing side. |
| BT-0515 | m:celebrate.shortDesc | No competitive use. |
| BT-0516 | m:charge.desc | Raises the user's Special Defense by 1 stage. The user's next Electric-type attack will... |
| BT-0517 | m:charge.shortDesc | +1 SpD, user's next Electric move 2× power. |
| BT-0519 | m:chargebeam.desc | Has a 70% chance to raise the user's Special Attack by 1 stage. |
| BT-0520 | m:chargebeam.shortDesc | 70% chance to raise the user's Sp. Atk by 1. |
| BT-0521 | m:charm.desc | Lowers the target's Attack by 2 stages. |
| BT-0522 | m:charm.shortDesc | Lowers the target's Attack by 2. |
| BT-0523 | m:chatter.desc | Has a 100% chance to confuse the target. |
| BT-0524 | m:chatter.shortDesc | 100% chance to confuse the target. |
| BT-0525 | m:chillingwater.desc | Has a 100% chance to lower the target's Attack by 1 stage. |
| BT-0526 | m:chillingwater.shortDesc | 100% chance to lower the target's Attack by 1. |
| BT-0527 | m:chillyreception.desc | For 5 turns, the weather becomes Snow. The user switches out even if it is trapped and ... |
| BT-0529 | m:chillyreception.shortDesc | Starts Snow. User switches out. |
| BT-0530 | m:chipaway.desc | Ignores the target's stat stage changes, including evasiveness. |
| BT-0531 | m:chipaway.shortDesc | Ignores the target's stat stage changes. |
| BT-0532 | m:chloroblast.desc | If this move is successful, the user loses 1/2 of its maximum HP, rounded up, unless th... |
| BT-0533 | m:chloroblast.shortDesc | User loses 50% max HP. |
| BT-0534 | m:circlethrow.desc | If both the user and the target have not fainted, the target is forced to switch out an... |
| BT-0535 | m:circlethrow.shortDesc | Forces the target to switch to a random ally. |
| BT-0536 | m:clamp.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-0538 | m:clamp.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-0540 | m:clangingscales.desc | Lowers the user's Defense by 1 stage. |
| BT-0541 | m:clangingscales.shortDesc | Lowers the user's Defense by 1. |
| BT-0542 | m:clangoroussoul.desc | Raises the user's Attack, Defense, Special Attack, Special Defense, and Speed by 1 stag... |
| BT-0543 | m:clangoroussoul.shortDesc | User loses 33% of its max HP. +1 to all stats. |
| BT-0544 | m:clangoroussoulblaze.desc | Raises the user's Attack, Defense, Special Attack, Special Defense, and Speed by 1 stage. |
| BT-0545 | m:clangoroussoulblaze.shortDesc | Raises the user's Atk/Def/SpAtk/SpDef/Spe by 1. |
| BT-0546 | m:clearsmog.shortDesc | Resets all of the target's stat stages to 0. |
| BT-0547 | m:closecombat.desc | Lowers the user's Defense and Special Defense by 1 stage. |
| BT-0548 | m:closecombat.shortDesc | Lowers the user's Defense and Sp. Def by 1. |
| BT-0549 | m:coaching.desc | Raises the target's Attack and Defense by 1 stage. Fails if there is no ally adjacent t... |
| BT-0550 | m:coaching.shortDesc | Raises an ally's Attack and Defense by 1. |
| BT-0551 | m:coil.desc | Raises the user's Attack, Defense, and accuracy by 1 stage. |
| BT-0552 | m:coil.shortDesc | Raises user's Attack, Defense, accuracy by 1. |
| BT-0553 | m:collisioncourse.desc | Damage is multiplied by 1.3333 if this move is super effective against the target. |
| BT-0554 | m:collisioncourse.shortDesc | Deals 1.3333× damage with supereffective hits. |
| BT-0555 | m:combattorque.desc | Has a 30% chance to paralyze the target. |
| BT-0556 | m:combattorque.shortDesc | 30% chance to paralyze the target. |
| BT-0557 | m:cometpunch.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0558 | m:cometpunch.shortDesc | Hits 2-5 times in one turn. |
| BT-0559 | m:comeuppance.desc | Deals damage to the last opposing Pokemon to hit the user with a physical or special at... |
| BT-0560 | m:comeuppance.shortDesc | If hit by an attack, returns 1.5× damage. |
| BT-0561 | m:confide.desc | Lowers the target's Special Attack by 1 stage. |
| BT-0562 | m:confide.shortDesc | Lowers the target's Sp. Atk by 1. |
| BT-0563 | m:confuseray.desc | Causes the target to become confused. |
| BT-0564 | m:confuseray.shortDesc | Confuses the target. |
| BT-0565 | m:confusion.desc | Has a 10% chance to confuse the target. |
| BT-0566 | m:confusion.shortDesc | 10% chance to confuse the target. |
| BT-0567 | m:constrict.desc | Has a 10% chance to lower the target's Speed by 1 stage. |
| BT-0568 | m:constrict.shortDesc | 10% chance to lower the target's Speed by 1. |
| BT-0569 | m:continentalcrush.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0570 | m:conversion.desc | The user's type changes to match the original type of the move in its first move slot. ... |
| BT-0571 | m:conversion.shortDesc | Changes user's type to match its first move. |
| BT-0573 | m:conversion2.desc | The user's type changes to match a type that resists or is immune to the type of the la... |
| BT-0574 | m:conversion2.shortDesc | Changes user's type to resist target's last move. |
| BT-0575 | m:copycat.desc | The user uses the last move used by any Pokemon, including itself. Fails if no move has... |
| BT-0576 | m:copycat.shortDesc | Uses the last move used in the battle. |
| BT-0577 | m:coreenforcer.desc | If the user moves after the target, the target's Ability is rendered ineffective as lon... |
| BT-0578 | m:coreenforcer.shortDesc | Nullifies the foe(s) Ability if the foe(s) move first. |
| BT-0579 | m:corkscrewcrash.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0580 | m:corrosivegas.desc | The target loses its held item. This move cannot cause Pokemon with the Sticky Hold Abi... |
| BT-0583 | m:corrosivegas.shortDesc | Removes adjacent Pokemon's held items. |
| BT-0584 | m:cosmicpower.desc | Raises the user's Defense and Special Defense by 1 stage. |
| BT-0585 | m:cosmicpower.shortDesc | Raises the user's Defense and Sp. Def by 1. |
| BT-0586 | m:cottonguard.desc | Raises the user's Defense by 3 stages. |
| BT-0587 | m:cottonguard.shortDesc | Raises the user's Defense by 3. |
| BT-0588 | m:cottonspore.desc | Lowers the target's Speed by 2 stages. |
| BT-0589 | m:cottonspore.shortDesc | Lowers the target's Speed by 2. |
| BT-0590 | m:counter.desc | Deals damage to the last opposing Pokemon to hit the user with a physical attack this t... |
| BT-0591 | m:counter.shortDesc | If hit by physical attack, returns double damage. |
| BT-0593 | m:courtchange.desc | Switches the Mist, Light Screen, Reflect, Spikes, Safeguard, Tailwind, Toxic Spikes, St... |
| BT-0594 | m:courtchange.shortDesc | Swaps user's field effects with the opposing side. |
| BT-0595 | m:covet.desc | If this attack was successful and the user has not fainted, it steals the target's held... |
| BT-0596 | m:covet.shortDesc | If the user has no item, it steals the target's. |
| BT-0597 | m:crabhammer.desc | Has a higher chance for a critical hit. |
| BT-0598 | m:crabhammer.shortDesc | High critical hit ratio. |
| BT-0600 | m:craftyshield.desc | The user and its party members are protected from non-damaging attacks made by other Po... |
| BT-0601 | m:craftyshield.shortDesc | Protects allies from Status moves this turn. |
| BT-0603 | m:crosschop.desc | Has a higher chance for a critical hit. |
| BT-0604 | m:crosschop.shortDesc | High critical hit ratio. |
| BT-0605 | m:crosspoison.desc | Has a 10% chance to poison the target and a higher chance for a critical hit. |
| BT-0606 | m:crosspoison.shortDesc | High critical hit ratio. 10% chance to poison. |
| BT-0607 | m:crunch.desc | Has a 20% chance to lower the target's Defense by 1 stage. |
| BT-0608 | m:crunch.shortDesc | 20% chance to lower the target's Defense by 1. |
| BT-0609 | m:crushclaw.desc | Has a 50% chance to lower the target's Defense by 1 stage. |
| BT-0610 | m:crushclaw.shortDesc | 50% chance to lower the target's Defense by 1. |
| BT-0611 | m:crushgrip.desc | Power is equal to 120 × (target's current HP / target's maximum HP), rounded half down,... |
| BT-0612 | m:crushgrip.shortDesc | More power the more HP the target has left. |
| BT-0614 | m:curse.desc | If the user is not a Ghost type, lowers the user's Speed by 1 stage and raises the user... |
| BT-0615 | m:curse.shortDesc | Curses if Ghost, else -1 Spe, +1 Atk, +1 Def. |
| BT-0617 | m:cut.shortDesc | No additional effect. |
| BT-0618 | m:darkestlariat.desc | Ignores the target's stat stage changes, including evasiveness. |
| BT-0619 | m:darkestlariat.shortDesc | Ignores the target's stat stage changes. |
| BT-0620 | m:darkpulse.desc | Has a 20% chance to make the target flinch. |
| BT-0621 | m:darkpulse.shortDesc | 20% chance to make the target flinch. |
| BT-0622 | m:darkvoid.desc | Causes the target to fall asleep. This move cannot be used successfully unless the user... |
| BT-0625 | m:darkvoid.shortDesc | Darkrai: Causes the foe(s) to fall asleep. |
| BT-0626 | m:dazzlinggleam.desc | No additional effect. |
| BT-0627 | m:dazzlinggleam.shortDesc | No additional effect. Hits adjacent foes. |
| BT-0628 | m:decorate.desc | Raises the target's Attack and Special Attack by 2 stages. |
| BT-0629 | m:decorate.shortDesc | Raises the target's Attack and Sp. Atk by 2. |
| BT-0630 | m:defendorder.desc | Raises the user's Defense and Special Defense by 1 stage. |
| BT-0631 | m:defendorder.shortDesc | Raises the user's Defense and Sp. Def by 1. |
| BT-0632 | m:defensecurl.desc | Raises the user's Defense by 1 stage. As long as the user remains active, the power of ... |
| BT-0633 | m:defensecurl.shortDesc | Raises the user's Defense by 1. |
| BT-0634 | m:defog.desc | Lowers the target's evasiveness by 1 stage. If this move is successful and whether or n... |
| BT-0635 | m:defog.shortDesc | -1 evasion; ends user and target hazards/terrain. |
| BT-0637 | m:destinybond.desc | Until the user's next move, if an opposing Pokemon's attack knocks the user out, that P... |
| BT-0638 | m:destinybond.shortDesc | If an opponent knocks out the user, it also faints. |
| BT-0640 | m:detect.desc | The user is protected from most attacks made by other Pokemon during this turn. This mo... |
| BT-0641 | m:detect.shortDesc | Prevents moves from affecting the user this turn. |
| BT-0642 | m:devastatingdrake.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0643 | m:diamondstorm.desc | Has a 50% chance to raise the user's Defense by 2 stages. |
| BT-0644 | m:diamondstorm.shortDesc | 50% chance to raise user's Defense by 2. |
| BT-0645 | m:dig.desc | This attack charges on the first turn and executes on the second. On the first turn, th... |
| BT-0647 | m:dig.shortDesc | Digs underground turn 1, strikes turn 2. |
| BT-0648 | m:direclaw.desc | Has a 50% chance to cause the target to either fall asleep, become poisoned, or become ... |
| BT-0649 | m:direclaw.shortDesc | 50% chance to sleep, poison, or paralyze target. |
| BT-0651 | m:disable.desc | For 4 turns, the target's last move used becomes disabled. Fails if one of the target's... |
| BT-0653 | m:disable.shortDesc | For 4 turns, disables the target's last move used. |
| BT-0655 | m:disarmingvoice.desc | This move does not check accuracy. |
| BT-0656 | m:disarmingvoice.shortDesc | This move does not check accuracy. Hits foes. |
| BT-0657 | m:discharge.desc | Has a 30% chance to paralyze the target. |
| BT-0658 | m:discharge.shortDesc | 30% chance to paralyze adjacent Pokemon. |
| BT-0659 | m:dive.desc | This attack charges on the first turn and executes on the second. On the first turn, th... |
| BT-0661 | m:dive.shortDesc | Dives underwater turn 1, strikes turn 2. |
| BT-0662 | m:dizzypunch.desc | Has a 20% chance to confuse the target. |
| BT-0663 | m:dizzypunch.shortDesc | 20% chance to confuse the target. |
| BT-0664 | m:doodle.desc | The user and its ally's Abilities change to match the target's Ability. Does not change... |
| BT-0665 | m:doodle.shortDesc | User and ally's Abilities become target's Ability. |
| BT-0667 | m:doomdesire.desc | Deals damage two turns after this move is used. At the end of that turn, the damage is ... |
| BT-0668 | m:doomdesire.shortDesc | Hits two turns after being used. |
| BT-0670 | m:doubleedge.desc | If the target lost HP, the user takes recoil damage equal to 33% the HP lost by the tar... |
| BT-0671 | m:doubleedge.shortDesc | Has 33% recoil. |
| BT-0672 | m:doublehit.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0673 | m:doublehit.shortDesc | Hits 2 times in one turn. |
| BT-0674 | m:doubleironbash.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0675 | m:doubleironbash.shortDesc | Hits twice. 30% chance to make the target flinch. |
| BT-0676 | m:doublekick.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0677 | m:doublekick.shortDesc | Hits 2 times in one turn. |
| BT-0678 | m:doubleshock.desc | Fails unless the user is an Electric type. If this move is successful and the user is n... |
| BT-0679 | m:doubleshock.shortDesc | User's Electric type: typeless; must be Electric. |
| BT-0681 | m:doubleslap.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0682 | m:doubleslap.shortDesc | Hits 2-5 times in one turn. |
| BT-0683 | m:doubleteam.desc | Raises the user's evasiveness by 1 stage. |
| BT-0684 | m:doubleteam.shortDesc | Raises the user's evasiveness by 1. |
| BT-0685 | m:dracometeor.desc | Lowers the user's Special Attack by 2 stages. |
| BT-0686 | m:dracometeor.shortDesc | Lowers the user's Sp. Atk by 2. |
| BT-0687 | m:dragonascent.desc | Lowers the user's Defense and Special Defense by 1 stage. |
| BT-0689 | m:dragonascent.shortDesc | Lowers the user's Defense and Sp. Def by 1. |
| BT-0690 | m:dragonbreath.desc | Has a 30% chance to paralyze the target. |
| BT-0691 | m:dragonbreath.shortDesc | 30% chance to paralyze the target. |
| BT-0692 | m:dragoncheer.desc | Raises the target's chance for a critical hit by 1 stage, or by 2 stages if the target ... |
| BT-0693 | m:dragoncheer.shortDesc | Ally: Crit ratio +1, or +2 if ally is Dragon type. |
| BT-0695 | m:dragonclaw.shortDesc | No additional effect. |
| BT-0696 | m:dragondance.desc | Raises the user's Attack and Speed by 1 stage. |
| BT-0697 | m:dragondance.shortDesc | Raises the user's Attack and Speed by 1. |
| BT-0698 | m:dragondarts.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0699 | m:dragondarts.shortDesc | Hits twice. Doubles: Tries to hit each foe once. |
| BT-0700 | m:dragonenergy.desc | Power is equal to (user's current HP × 150 / user's maximum HP), rounded down, but not ... |
| BT-0701 | m:dragonenergy.shortDesc | Less power as user's HP decreases. Hits foe(s). |
| BT-0702 | m:dragonhammer.shortDesc | No additional effect. |
| BT-0703 | m:dragonpulse.shortDesc | No additional effect. |
| BT-0704 | m:dragonrage.shortDesc | Deals 40 HP of damage to the target. |
| BT-0705 | m:dragonrush.desc | Has a 20% chance to make the target flinch. Damage doubles and no accuracy check is don... |
| BT-0706 | m:dragonrush.shortDesc | 20% chance to make the target flinch. |
| BT-0707 | m:dragontail.desc | If both the user and the target have not fainted, the target is forced to switch out an... |
| BT-0708 | m:dragontail.shortDesc | Forces the target to switch to a random ally. |
| BT-0709 | m:drainingkiss.desc | The user recovers 3/4 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0710 | m:drainingkiss.shortDesc | User recovers 75% of the damage dealt. |
| BT-0711 | m:drainpunch.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0712 | m:drainpunch.shortDesc | User recovers 50% of the damage dealt. |
| BT-0713 | m:dreameater.desc | The target is unaffected by this move unless it is asleep. The user recovers 1/2 the HP... |
| BT-0714 | m:dreameater.shortDesc | User gains 1/2 HP inflicted. Sleeping target only. |
| BT-0715 | m:drillpeck.shortDesc | No additional effect. |
| BT-0716 | m:drillrun.desc | Has a higher chance for a critical hit. |
| BT-0717 | m:drillrun.shortDesc | High critical hit ratio. |
| BT-0718 | m:drumbeating.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-0719 | m:drumbeating.shortDesc | 100% chance to lower the target's Speed by 1. |
| BT-0720 | m:dualchop.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0721 | m:dualchop.shortDesc | Hits 2 times in one turn. |
| BT-0722 | m:dualwingbeat.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0723 | m:dualwingbeat.shortDesc | Hits 2 times in one turn. |
| BT-0724 | m:dynamaxcannon.shortDesc | No additional effect. |
| BT-0725 | m:dynamicpunch.desc | Has a 100% chance to confuse the target. |
| BT-0726 | m:dynamicpunch.shortDesc | 100% chance to confuse the target. |
| BT-0727 | m:earthpower.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0728 | m:earthpower.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-0729 | m:earthquake.desc | Damage doubles if the target is using Dig. |
| BT-0730 | m:earthquake.shortDesc | Hits adjacent Pokemon. Double damage on Dig. |
| BT-0731 | m:echoedvoice.desc | For every consecutive turn that this move is used by at least one Pokemon, this move's ... |
| BT-0732 | m:echoedvoice.shortDesc | Power increases when used on consecutive turns. |
| BT-0733 | m:eerieimpulse.desc | Lowers the target's Special Attack by 2 stages. |
| BT-0734 | m:eerieimpulse.shortDesc | Lowers the target's Sp. Atk by 2. |
| BT-0736 | m:eeriespell.desc | If this move is successful and the user has not fainted, the target loses 3 PP from its... |
| BT-0737 | m:eeriespell.shortDesc | Removes 3 PP from the target's last move. |
| BT-0738 | m:eggbomb.shortDesc | No additional effect. |
| BT-0739 | m:electricterrain.desc | For 5 turns, the terrain becomes Electric Terrain. During the effect, the power of Elec... |
| BT-0740 | m:electricterrain.shortDesc | 5 turns. Grounded: +Electric power, can't sleep. |
| BT-0741 | m:electrify.desc | Causes the target's move to become Electric type this turn. Among effects that can chan... |
| BT-0742 | m:electrify.shortDesc | Changes the target's move to Electric this turn. |
| BT-0744 | m:electroball.desc | The power of this move depends on (user's current Speed / target's current Speed), roun... |
| BT-0745 | m:electroball.shortDesc | More power the faster the user is than the target. |
| BT-0746 | m:electrodrift.desc | Damage is multiplied by 1.3333 if this move is super effective against the target. |
| BT-0747 | m:electrodrift.shortDesc | Deals 1.3333× damage with supereffective hits. |
| BT-0748 | m:electroshot.desc | This attack charges on the first turn and executes on the second. Raises the user's Spe... |
| BT-0750 | m:electroshot.shortDesc | Raises Sp. Atk by 1, hits turn 2. Rain: no charge. |
| BT-0751 | m:electroweb.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-0752 | m:electroweb.shortDesc | 100% chance to lower the foe(s) Speed by 1. |
| BT-0753 | m:embargo.desc | For 5 turns, the target's held item has no effect. An item's effect of causing forme ch... |
| BT-0755 | m:embargo.shortDesc | For 5 turns, the target's item has no effect. |
| BT-0757 | m:ember.desc | Has a 10% chance to burn the target. |
| BT-0758 | m:ember.shortDesc | 10% chance to burn the target. |
| BT-0759 | m:encore.desc | For its next 3 turns, the target is forced to repeat its last move used. If the affecte... |
| BT-0761 | m:encore.shortDesc | Target repeats its last move for its next 3 turns. |
| BT-0763 | m:endeavor.desc | Deals damage to the target equal to (target's current HP - user's current HP). The targ... |
| BT-0764 | m:endeavor.shortDesc | Lowers the target's HP to the user's HP. |
| BT-0766 | m:endure.desc | The user will survive attacks made by other Pokemon during this turn with at least 1 HP... |
| BT-0767 | m:endure.shortDesc | User survives attacks this turn with at least 1 HP. |
| BT-0769 | m:energyball.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0770 | m:energyball.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-0771 | m:entrainment.desc | Causes the target's Ability to become the same as the user's. Fails if the target's Abi... |
| BT-0772 | m:entrainment.shortDesc | The target's Ability changes to match the user's. |
| BT-0773 | m:eruption.desc | Power is equal to (user's current HP × 150 / user's maximum HP), rounded down, but not ... |
| BT-0774 | m:eruption.shortDesc | Less power as user's HP decreases. Hits foe(s). |
| BT-0775 | m:esperwing.desc | Has a 100% chance to raise the user's Speed by 1 stage and a higher chance for a critic... |
| BT-0776 | m:esperwing.shortDesc | 100% chance to raise user Speed by 1. High crit. |
| BT-0777 | m:eternabeam.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-0778 | m:eternabeam.shortDesc | User cannot move next turn. |
| BT-0779 | m:expandingforce.desc | If the current terrain is Psychic Terrain and the user is grounded, this move hits all ... |
| BT-0780 | m:expandingforce.shortDesc | User on Psychic Terrain: 1.5× power, hits foes. |
| BT-0781 | m:explosion.desc | The user faints after using this move, even if this move fails for having no target. Th... |
| BT-0782 | m:explosion.shortDesc | Hits adjacent Pokemon. The user faints. |
| BT-0783 | m:extrasensory.desc | Has a 10% chance to make the target flinch. |
| BT-0784 | m:extrasensory.shortDesc | 10% chance to make the target flinch. |
| BT-0785 | m:extremeevoboost.desc | Raises the user's Attack, Defense, Special Attack, Special Defense, and Speed by 2 stages. |
| BT-0786 | m:extremeevoboost.shortDesc | Raises user's Atk, Def, SpA, SpD, and Spe by 2. |
| BT-0787 | m:extremespeed.desc | No additional effect. |
| BT-0788 | m:extremespeed.shortDesc | Nearly always goes first. |
| BT-0789 | m:facade.desc | Power doubles if the user is burned, paralyzed, or poisoned. The physical damage halvin... |
| BT-0790 | m:facade.shortDesc | Power doubles if user is burn/poison/paralyzed. |
| BT-0792 | m:fairylock.desc | Prevents all active Pokemon from switching next turn. A Pokemon can still switch out if... |
| BT-0793 | m:fairylock.shortDesc | Prevents all Pokemon from switching next turn. |
| BT-0794 | m:fairywind.shortDesc | No additional effect. |
| BT-0795 | m:fakeout.desc | Has a 100% chance to make the target flinch. Fails unless it is the user's first turn o... |
| BT-0796 | m:fakeout.shortDesc | Hits first. First turn out only. 100% flinch chance. |
| BT-0797 | m:faketears.desc | Lowers the target's Special Defense by 2 stages. |
| BT-0798 | m:faketears.shortDesc | Lowers the target's Sp. Def by 2. |
| BT-0799 | m:falsesurrender.shortDesc | This move does not check accuracy. |
| BT-0800 | m:falseswipe.desc | Leaves the target with at least 1 HP. |
| BT-0801 | m:falseswipe.shortDesc | Always leaves the target with at least 1 HP. |
| BT-0802 | m:featherdance.desc | Lowers the target's Attack by 2 stages. |
| BT-0803 | m:featherdance.shortDesc | Lowers the target's Attack by 2. |
| BT-0805 | m:feint.desc | If this move is successful, it breaks through the target's Baneful Bunker, Detect, King... |
| BT-0806 | m:feint.shortDesc | Nullifies Detect, Protect, and Quick/Wide Guard. |
| BT-0807 | m:feintattack.shortDesc | This move does not check accuracy. |
| BT-0808 | m:fellstinger.desc | Raises the user's Attack by 3 stages if this move knocks out the target. |
| BT-0809 | m:fellstinger.shortDesc | Raises user's Attack by 3 if this KOes the target. |
| BT-0811 | m:ficklebeam.shortDesc | Has a 30% chance this move's power is doubled. |
| BT-0812 | m:fierydance.desc | Has a 50% chance to raise the user's Special Attack by 1 stage. |
| BT-0813 | m:fierydance.shortDesc | 50% chance to raise the user's Sp. Atk by 1. |
| BT-0814 | m:fierywrath.desc | Has a 20% chance to make the target flinch. |
| BT-0815 | m:fierywrath.shortDesc | 20% chance to make the foe(s) flinch. |
| BT-0816 | m:filletaway.desc | Raises the user's Attack, Special Attack, and Speed by 2 stages in exchange for the use... |
| BT-0817 | m:filletaway.shortDesc | +2 Attack, Sp. Atk, Speed for 1/2 user's max HP. |
| BT-0818 | m:finalgambit.desc | Deals damage to the target equal to the user's current HP. If this move is successful, ... |
| BT-0819 | m:finalgambit.shortDesc | Does damage equal to the user's HP. User faints. |
| BT-0820 | m:fireblast.desc | Has a 10% chance to burn the target. |
| BT-0821 | m:fireblast.shortDesc | 10% chance to burn the target. |
| BT-0822 | m:firefang.desc | Has a 10% chance to burn the target and a 10% chance to make it flinch. |
| BT-0823 | m:firefang.shortDesc | 10% chance to burn. 10% chance to flinch. |
| BT-0824 | m:firelash.desc | Has a 100% chance to lower the target's Defense by 1 stage. |
| BT-0825 | m:firelash.shortDesc | 100% chance to lower the target's Defense by 1. |
| BT-0828 | m:firepledge.desc | If one of the user's allies chose to use Grass Pledge or Water Pledge this turn and has... |
| BT-0830 | m:firepledge.shortDesc | Use with Grass or Water Pledge for added effect. |
| BT-0832 | m:firepunch.desc | Has a 10% chance to burn the target. |
| BT-0833 | m:firepunch.shortDesc | 10% chance to burn the target. |
| BT-0834 | m:firespin.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-0836 | m:firespin.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-0838 | m:firstimpression.desc | Fails unless it is the user's first turn on the field. |
| BT-0839 | m:firstimpression.shortDesc | Nearly always goes first. First turn out only. |
| BT-0840 | m:fishiousrend.desc | Power doubles if the user moves before the target. |
| BT-0841 | m:fishiousrend.shortDesc | Power doubles if user moves before the target. |
| BT-0842 | m:fissure.desc | Deals damage to the target equal to the target's maximum HP. Ignores accuracy and evasi... |
| BT-0843 | m:fissure.shortDesc | OHKOs the target. Fails if user is a lower level. |
| BT-0844 | m:flail.desc | The power of this move is 20 if X is 33 to 48, 40 if X is 17 to 32, 80 if X is 10 to 16... |
| BT-0845 | m:flail.shortDesc | More power the less HP the user has left. |
| BT-0847 | m:flameburst.desc | If this move is successful, the target's ally loses 1/16 of its maximum HP, rounded dow... |
| BT-0848 | m:flameburst.shortDesc | Damages Pokemon next to the target as well. |
| BT-0849 | m:flamecharge.desc | Has a 100% chance to raise the user's Speed by 1 stage. |
| BT-0850 | m:flamecharge.shortDesc | 100% chance to raise the user's Speed by 1. |
| BT-0851 | m:flamethrower.desc | Has a 10% chance to burn the target. |
| BT-0852 | m:flamethrower.shortDesc | 10% chance to burn the target. |
| BT-0853 | m:flamewheel.desc | Has a 10% chance to burn the target. |
| BT-0854 | m:flamewheel.shortDesc | 10% chance to burn the target. Thaws user. |
| BT-0855 | m:flareblitz.desc | Has a 10% chance to burn the target. If the target lost HP, the user takes recoil damag... |
| BT-0856 | m:flareblitz.shortDesc | Has 33% recoil. 10% chance to burn. Thaws user. |
| BT-0857 | m:flash.desc | Lowers the target's accuracy by 1 stage. |
| BT-0858 | m:flash.shortDesc | Lowers the target's accuracy by 1. |
| BT-0859 | m:flashcannon.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0860 | m:flashcannon.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-0861 | m:flatter.desc | Raises the target's Special Attack by 1 stage and confuses it. |
| BT-0862 | m:flatter.shortDesc | Raises the target's Sp. Atk by 1 and confuses it. |
| BT-0863 | m:fleurcannon.desc | Lowers the user's Special Attack by 2 stages. |
| BT-0864 | m:fleurcannon.shortDesc | Lowers the user's Sp. Atk by 2. |
| BT-0865 | m:fling.desc | The power of this move is based on the user's held item. The held item is lost and it a... |
| BT-0867 | m:fling.shortDesc | Flings the user's item at the target. Power varies. |
| BT-0868 | m:flipturn.desc | If this move is successful and the user has not fainted, the user switches out even if ... |
| BT-0869 | m:flipturn.shortDesc | User switches out after damaging the target. |
| BT-0871 | m:floatyfall.desc | Has a 30% chance to make the target flinch. |
| BT-0872 | m:floatyfall.shortDesc | 30% chance to make the target flinch. |
| BT-0873 | m:floralhealing.desc | The target restores 1/2 of its maximum HP, rounded half up. If the terrain is Grassy Te... |
| BT-0874 | m:floralhealing.shortDesc | Heals the target by 50% of its max HP. |
| BT-0875 | m:flowershield.desc | Raises the Defense of all active Grass-type Pokemon by 1 stage. Fails if there are no a... |
| BT-0876 | m:flowershield.shortDesc | Raises Defense by 1 of all active Grass types. |
| BT-0877 | m:flowertrick.desc | This move is always a critical hit unless the target is under the effect of Lucky Chant... |
| BT-0878 | m:flowertrick.shortDesc | Always results in a critical hit; no accuracy check. |
| BT-0879 | m:fly.desc | This attack charges on the first turn and executes on the second. On the first turn, th... |
| BT-0881 | m:fly.shortDesc | Flies up on first turn, then strikes the next turn. |
| BT-0882 | m:flyingpress.desc | This move combines Flying in its type effectiveness against the target. Damage doubles ... |
| BT-0883 | m:flyingpress.shortDesc | Combines Flying in its type effectiveness. |
| BT-0884 | m:focusblast.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-0885 | m:focusblast.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-0886 | m:focusenergy.desc | Raises the user's chance for a critical hit by 2 stages. Fails if the user already has ... |
| BT-0887 | m:focusenergy.shortDesc | Raises the user's critical hit ratio by 2. |
| BT-0892 | m:focuspunch.desc | The user loses its focus and does nothing if it is hit by a damaging attack this turn b... |
| BT-0893 | m:focuspunch.shortDesc | Fails if the user takes damage before it hits. |
| BT-0895 | m:followme.desc | Until the end of the turn, all single-target attacks from the opposing side are redirec... |
| BT-0896 | m:followme.shortDesc | The foes' moves target the user on the turn used. |
| BT-0899 | m:forcepalm.desc | Has a 30% chance to paralyze the target. |
| BT-0900 | m:forcepalm.shortDesc | 30% chance to paralyze the target. |
| BT-0901 | m:foresight.desc | As long as the target remains active, its evasiveness stat stage is ignored during accu... |
| BT-0902 | m:foresight.shortDesc | Fighting, Normal hit Ghost. Evasiveness ignored. |
| BT-0904 | m:forestscurse.desc | Causes the Grass type to be added to the target, effectively making it have two or thre... |
| BT-0905 | m:forestscurse.shortDesc | Adds Grass to the target's type(s). |
| BT-0906 | m:foulplay.desc | Damage is calculated using the target's Attack stat, including stat stage changes. The ... |
| BT-0907 | m:foulplay.shortDesc | Uses target's Attack stat in damage calculation. |
| BT-0908 | m:freezedry.desc | Has a 10% chance to freeze the target. This move's type effectiveness against Water is ... |
| BT-0909 | m:freezedry.shortDesc | 10% chance to freeze. Super effective on Water. |
| BT-0910 | m:freezeshock.desc | Has a 30% chance to paralyze the target. This attack charges on the first turn and exec... |
| BT-0912 | m:freezeshock.shortDesc | Charges turn 1. Hits turn 2. 30% paralyze. |
| BT-0913 | m:freezingglare.desc | Has a 10% chance to freeze the target. |
| BT-0914 | m:freezingglare.shortDesc | 10% chance to freeze the target. |
| BT-0915 | m:freezyfrost.desc | Resets the stat stages of all active Pokemon to 0. |
| BT-0916 | m:freezyfrost.shortDesc | Eliminates all stat changes. |
| BT-0917 | m:frenzyplant.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-0918 | m:frenzyplant.shortDesc | User cannot move next turn. |
| BT-0919 | m:frostbreath.desc | This move is always a critical hit unless the target is under the effect of Lucky Chant... |
| BT-0920 | m:frostbreath.shortDesc | Always results in a critical hit. |
| BT-0921 | m:frustration.desc | Power is equal to the greater of ((255 - user's Happiness) × 2/5), rounded down, or 1. |
| BT-0922 | m:frustration.shortDesc | Max 102 power at minimum Happiness. |
| BT-0923 | m:furyattack.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0924 | m:furyattack.shortDesc | Hits 2-5 times in one turn. |
| BT-0925 | m:furycutter.desc | Power doubles with each successful hit, up to a maximum of 160 power. The power is rese... |
| BT-0926 | m:furycutter.shortDesc | Power doubles with each hit, up to 160. |
| BT-0927 | m:furyswipes.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-0928 | m:furyswipes.shortDesc | Hits 2-5 times in one turn. |
| BT-0929 | m:fusionbolt.desc | Power doubles if the last move used by any Pokemon this turn was Fusion Flare. |
| BT-0930 | m:fusionbolt.shortDesc | Power doubles if used after Fusion Flare this turn. |
| BT-0931 | m:fusionflare.desc | Power doubles if the last move used by any Pokemon this turn was Fusion Bolt. |
| BT-0932 | m:fusionflare.shortDesc | Power doubles if used after Fusion Bolt this turn. |
| BT-0934 | m:futuresight.desc | Deals damage two turns after this move is used. At the end of that turn, the damage is ... |
| BT-0935 | m:futuresight.shortDesc | Hits two turns after being used. |
| BT-0937 | m:gastroacid.desc | Causes the target's Ability to be rendered ineffective as long as it remains active. If... |
| BT-0938 | m:gastroacid.shortDesc | Nullifies the target's Ability. |
| BT-0940 | m:geargrind.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-0941 | m:geargrind.shortDesc | Hits 2 times in one turn. |
| BT-0942 | m:gearup.desc | Raises the Attack and Special Attack of Pokemon on the user's side with the Plus or Min... |
| BT-0943 | m:gearup.shortDesc | Raises Atk, Sp. Atk of allies with Plus/Minus by 1. |
| BT-0944 | m:genesissupernova.desc | If this move is successful, the terrain becomes Psychic Terrain. |
| BT-0945 | m:genesissupernova.shortDesc | Summons Psychic Terrain. |
| BT-0946 | m:geomancy.desc | Raises the user's Special Attack, Special Defense, and Speed by 2 stages. This attack c... |
| BT-0948 | m:geomancy.shortDesc | Charges, then raises SpA, SpD, Spe by 2 turn 2. |
| BT-0949 | m:gigadrain.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-0950 | m:gigadrain.shortDesc | User recovers 50% of the damage dealt. |
| BT-0951 | m:gigaimpact.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-0952 | m:gigaimpact.shortDesc | User cannot move next turn. |
| BT-0953 | m:gigatonhammer.shortDesc | Cannot be selected the turn after it's used. |
| BT-0954 | m:gigavolthavoc.shortDesc | Power is equal to the base move's Z-Power. |
| BT-0955 | m:glaciallance.desc | No additional effect. |
| BT-0956 | m:glaciallance.shortDesc | No additional effect. Hits adjacent foes. |
| BT-0957 | m:glaciate.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-0958 | m:glaciate.shortDesc | 100% chance to lower the foe(s) Speed by 1. |
| BT-0959 | m:glaiverush.desc | If this move is successful, moves targeted at the user deal double damage and do not ch... |
| BT-0960 | m:glaiverush.shortDesc | User takes sure-hit 2× damage until its next turn. |
| BT-0961 | m:glare.desc | Paralyzes the target. |
| BT-0962 | m:glare.shortDesc | Paralyzes the target. |
| BT-0963 | m:glitzyglow.desc | This move summons Light Screen for 5 turns upon use. |
| BT-0964 | m:glitzyglow.shortDesc | Summons Light Screen. |
| BT-0965 | m:gmaxbefuddle.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0966 | m:gmaxbefuddle.shortDesc | Base move affects power. Foes: slp or psn or par. |
| BT-0968 | m:gmaxcannonade.desc | Power is equal to the base move's Max Move power. If this move is successful, for 4 tur... |
| BT-0969 | m:gmaxcannonade.shortDesc | Base move affects power. Foes: -1/6 HP, 4 turns. |
| BT-0971 | m:gmaxcentiferno.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0972 | m:gmaxcentiferno.shortDesc | Base move affects power. Foes: bound 4-5 turns. |
| BT-0973 | m:gmaxchistrike.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0974 | m:gmaxchistrike.shortDesc | Base move affects power. Allies: Crit Ratio +1. |
| BT-0976 | m:gmaxcuddle.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0977 | m:gmaxcuddle.shortDesc | Base move affects power. Foes: infatuated. |
| BT-0979 | m:gmaxdepletion.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0980 | m:gmaxdepletion.shortDesc | Base move affects power. Foes: last move -2 PP. |
| BT-0981 | m:gmaxdrumsolo.desc | Power is 160 regardless of the base move's Max Move power. This move and its effects ig... |
| BT-0982 | m:gmaxdrumsolo.shortDesc | Always 160 power. Ignores Abilities. |
| BT-0983 | m:gmaxfinale.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0984 | m:gmaxfinale.shortDesc | Base move affects power. Allies: +1/6 max HP. |
| BT-0985 | m:gmaxfireball.desc | Power is 160 regardless of the base move's Max Move power. This move and its effects ig... |
| BT-0986 | m:gmaxfireball.shortDesc | Always 160 power. Ignores Abilities. |
| BT-0987 | m:gmaxfoamburst.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speed... |
| BT-0988 | m:gmaxfoamburst.shortDesc | Base move affects power. Foes: -2 Speed. |
| BT-0989 | m:gmaxgoldrush.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0990 | m:gmaxgoldrush.shortDesc | Base move affects power. Foes: confused. |
| BT-0991 | m:gmaxgravitas.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-0992 | m:gmaxgravitas.shortDesc | Base move affects power. Starts Gravity. |
| BT-0993 | m:gmaxhydrosnipe.desc | Power is 160 regardless of the base move's Max Move power. This move and its effects ig... |
| BT-0994 | m:gmaxhydrosnipe.shortDesc | Always 160 power. Ignores Abilities. |
| BT-0995 | m:gmaxmalodor.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-0996 | m:gmaxmalodor.shortDesc | Base move affects power. Foes: poisoned. |
| BT-0997 | m:gmaxmeltdown.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-0998 | m:gmaxmeltdown.shortDesc | Base move affects power. Foes: Tormented. |
| BT-0999 | m:gmaxoneblow.desc | Power is equal to the base move's Max Move power. This move bypasses all protection eff... |
| BT-1000 | m:gmaxoneblow.shortDesc | Base move affects power. Bypasses Max Guard. |
| BT-1001 | m:gmaxrapidflow.desc | Power is equal to the base move's Max Move power. This move bypasses all protection eff... |
| BT-1002 | m:gmaxrapidflow.shortDesc | Base move affects power. Bypasses Max Guard. |
| BT-1003 | m:gmaxreplenish.desc | Power is equal to the base move's Max Move power. If this move is successful, there is ... |
| BT-1004 | m:gmaxreplenish.shortDesc | Base move affects power. 50% restores Berries. |
| BT-1005 | m:gmaxresonance.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1006 | m:gmaxresonance.shortDesc | Base move affects power. Allies: Aurora Veil. |
| BT-1007 | m:gmaxsandblast.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1008 | m:gmaxsandblast.shortDesc | Base move affects power. Foes: bound 4-5 turns. |
| BT-1009 | m:gmaxsmite.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1010 | m:gmaxsmite.shortDesc | Base move affects power. Foes: confused. |
| BT-1011 | m:gmaxsnooze.desc | Power is equal to the base move's Max Move power. If this move is successful, there is ... |
| BT-1012 | m:gmaxsnooze.shortDesc | Base move affects power. Target: 50% Yawn. |
| BT-1014 | m:gmaxsteelsurge.desc | Power is equal to the base move's Max Move power. If this move is successful, it sets u... |
| BT-1016 | m:gmaxsteelsurge.shortDesc | Base move affects power. Foes: Steel hazard. |
| BT-1018 | m:gmaxstonesurge.desc | Power is equal to the base move's Max Move power. If this move is successful, it sets u... |
| BT-1019 | m:gmaxstonesurge.shortDesc | Base move affects power. Foes: Stealth Rock. |
| BT-1020 | m:gmaxstunshock.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1021 | m:gmaxstunshock.shortDesc | Base move affects power. Foes: psn or par. |
| BT-1022 | m:gmaxsweetness.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1023 | m:gmaxsweetness.shortDesc | Base move affects power. Allies: status cured. |
| BT-1024 | m:gmaxtartness.desc | Power is equal to the base move's Max Move power. If this move is successful, the evasi... |
| BT-1025 | m:gmaxtartness.shortDesc | Base move affects power. Foes: -1 evasiveness. |
| BT-1026 | m:gmaxterror.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1027 | m:gmaxterror.shortDesc | Base move affects power. Foes: trapped. |
| BT-1029 | m:gmaxvinelash.desc | Power is equal to the base move's Max Move power. If this move is successful, for 4 tur... |
| BT-1030 | m:gmaxvinelash.shortDesc | Base move affects power. Foes: -1/6 HP, 4 turns. |
| BT-1033 | m:gmaxvolcalith.desc | Power is equal to the base move's Max Move power. If this move is successful, for 4 tur... |
| BT-1034 | m:gmaxvolcalith.shortDesc | Base move affects power. Foes: -1/6 HP, 4 turns. |
| BT-1036 | m:gmaxvoltcrash.desc | Power is equal to the base move's Max Move power. If this move is successful, each Poke... |
| BT-1037 | m:gmaxvoltcrash.shortDesc | Base move affects power. Foes: paralyzed. |
| BT-1039 | m:gmaxwildfire.desc | Power is equal to the base move's Max Move power. If this move is successful, for 4 tur... |
| BT-1040 | m:gmaxwildfire.shortDesc | Base move affects power. Foes: -1/6 HP, 4 turns. |
| BT-1042 | m:gmaxwindrage.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1043 | m:gmaxwindrage.shortDesc | Base move affects power. Ends Terrain, hazards. |
| BT-1044 | m:grassknot.desc | This move's power is 20 if the target weighs less than 10 kg, 40 if less than 25 kg, 60... |
| BT-1045 | m:grassknot.shortDesc | More power the heavier the target. |
| BT-1047 | m:grasspledge.desc | If one of the user's allies chose to use Fire Pledge or Water Pledge this turn and has ... |
| BT-1049 | m:grasspledge.shortDesc | Use with Fire or Water Pledge for added effect. |
| BT-1051 | m:grasswhistle.shortDesc | Causes the target to fall asleep. |
| BT-1052 | m:grassyglide.desc | If the current terrain is Grassy Terrain and the user is grounded, this move has its pr... |
| BT-1053 | m:grassyglide.shortDesc | User on Grassy Terrain: +1 priority. |
| BT-1054 | m:grassyterrain.desc | For 5 turns, the terrain becomes Grassy Terrain. During the effect, the power of Grass-... |
| BT-1055 | m:grassyterrain.shortDesc | 5 turns. Grounded: +Grass power, +1/16 max HP. |
| BT-1056 | m:gravapple.desc | Has a 100% chance to lower the target's Defense by 1 stage. Power is multiplied by 1.5 ... |
| BT-1057 | m:gravapple.shortDesc | Target: 100% -1 Def. During Gravity: 1.5× power. |
| BT-1058 | m:gravity.desc | For 5 turns, the evasiveness of all active Pokemon is multiplied by 0.6. At the time of... |
| BT-1059 | m:gravity.shortDesc | 5 turns: no Ground immunities, 1.67× accuracy. |
| BT-1060 | m:growl.desc | Lowers the target's Attack by 1 stage. |
| BT-1061 | m:growl.shortDesc | Lowers the foe(s) Attack by 1. |
| BT-1062 | m:growth.desc | Raises the user's Attack and Special Attack by 1 stage. If the weather is Sun or Intens... |
| BT-1063 | m:growth.shortDesc | Raises user's Attack and Sp. Atk by 1; 2 in Sun. |
| BT-1065 | m:grudge.desc | Until the user's next turn, if an opposing Pokemon's attack knocks the user out, that m... |
| BT-1066 | m:grudge.shortDesc | If the user faints, the attack used loses all its PP. |
| BT-1068 | m:guardianofalola.desc | Deals damage to the target equal to 3/4 of its current HP, rounded down, but not less t... |
| BT-1069 | m:guardianofalola.shortDesc | Does damage equal to 3/4 target's current HP. |
| BT-1071 | m:guardsplit.desc | The user and the target have their Defense and Special Defense stats set to be equal to... |
| BT-1072 | m:guardsplit.shortDesc | Averages Defense and Sp. Def stats with target. |
| BT-1073 | m:guardswap.desc | The user swaps its Defense and Special Defense stat stage changes with the target. |
| BT-1074 | m:guardswap.shortDesc | Swaps Defense and Sp. Def changes with target. |
| BT-1075 | m:guillotine.desc | Deals damage to the target equal to the target's maximum HP. Ignores accuracy and evasi... |
| BT-1076 | m:guillotine.shortDesc | OHKOs the target. Fails if user is a lower level. |
| BT-1077 | m:gunkshot.desc | Has a 30% chance to poison the target. |
| BT-1078 | m:gunkshot.shortDesc | 30% chance to poison the target. |
| BT-1079 | m:gust.desc | Power doubles if the target is using Bounce, Fly, or Sky Drop, or is under the effect o... |
| BT-1080 | m:gust.shortDesc | Power doubles during Bounce, Fly, and Sky Drop. |
| BT-1081 | m:gyroball.desc | Power is equal to (25 * target's current Speed / user's current Speed) + 1, rounded dow... |
| BT-1082 | m:gyroball.shortDesc | More power the slower the user than the target. |
| BT-1083 | m:hail.desc | For 5 turns, the weather becomes Hail. At the end of each turn except the last, all act... |
| BT-1084 | m:hail.shortDesc | For 5 turns, hail crashes down. |
| BT-1085 | m:hammerarm.desc | Lowers the user's Speed by 1 stage. |
| BT-1086 | m:hammerarm.shortDesc | Lowers the user's Speed by 1. |
| BT-1088 | m:happyhour.shortDesc | No competitive use. |
| BT-1089 | m:harden.desc | Raises the user's Defense by 1 stage. |
| BT-1090 | m:harden.shortDesc | Raises the user's Defense by 1. |
| BT-1091 | m:hardpress.desc | Power is equal to 100 × (target's current HP / target's maximum HP), rounded half down,... |
| BT-1092 | m:hardpress.shortDesc | More power the more HP the target has left. |
| BT-1094 | m:haze.desc | Resets the stat stages of all active Pokemon to 0. |
| BT-1095 | m:haze.shortDesc | Eliminates all stat changes. |
| BT-1096 | m:headbutt.desc | Has a 30% chance to make the target flinch. |
| BT-1097 | m:headbutt.shortDesc | 30% chance to make the target flinch. |
| BT-1098 | m:headcharge.desc | If the target lost HP, the user takes recoil damage equal to 1/4 the HP lost by the tar... |
| BT-1099 | m:headcharge.shortDesc | Has 1/4 recoil. |
| BT-1100 | m:headlongrush.desc | Lowers the user's Defense and Special Defense by 1 stage. |
| BT-1101 | m:headlongrush.shortDesc | Lowers the user's Defense and Sp. Def by 1. |
| BT-1102 | m:headsmash.desc | If the target lost HP, the user takes recoil damage equal to 1/2 the HP lost by the tar... |
| BT-1103 | m:headsmash.shortDesc | Has 1/2 recoil. |
| BT-1105 | m:healbell.desc | Every Pokemon in the user's party is cured of its non-volatile status condition. Active... |
| BT-1106 | m:healbell.shortDesc | Cures the user's party of all status conditions. |
| BT-1108 | m:healblock.desc | For 5 turns, the target is prevented from restoring any HP as long as it remains active... |
| BT-1111 | m:healblock.shortDesc | For 5 turns, the foe(s) is prevented from healing. |
| BT-1113 | m:healingwish.desc | The user faints, and if the Pokemon brought out to replace it does not have full HP or ... |
| BT-1115 | m:healingwish.shortDesc | User faints. Next hurt Pokemon is fully healed. |
| BT-1116 | m:healorder.desc | The user restores 1/2 of its maximum HP, rounded half up. |
| BT-1117 | m:healorder.shortDesc | Heals the user by 50% of its max HP. |
| BT-1118 | m:healpulse.desc | The target restores 1/2 of its maximum HP, rounded half up. If the user has the Mega La... |
| BT-1119 | m:healpulse.shortDesc | Heals the target by 50% of its max HP. |
| BT-1120 | m:heartstamp.desc | Has a 30% chance to make the target flinch. |
| BT-1121 | m:heartstamp.shortDesc | 30% chance to make the target flinch. |
| BT-1122 | m:heartswap.desc | The user swaps all its stat stage changes with the target. |
| BT-1123 | m:heartswap.shortDesc | Swaps all stat changes with target. |
| BT-1124 | m:heatcrash.desc | The power of this move depends on (user's weight / target's weight), rounded down. Powe... |
| BT-1125 | m:heatcrash.shortDesc | More power the heavier the user than the target. |
| BT-1126 | m:heatwave.desc | Has a 10% chance to burn the target. |
| BT-1127 | m:heatwave.shortDesc | 10% chance to burn the foe(s). |
| BT-1128 | m:heavyslam.desc | The power of this move depends on (user's weight / target's weight), rounded down. Powe... |
| BT-1129 | m:heavyslam.shortDesc | More power the heavier the user than the target. |
| BT-1130 | m:helpinghand.desc | The power of the target's attack this turn is multiplied by 1.5 (this effect is stackab... |
| BT-1131 | m:helpinghand.shortDesc | One adjacent ally's move power is 1.5× this turn. |
| BT-1133 | m:hex.desc | Power doubles if the target has a non-volatile status condition. |
| BT-1134 | m:hex.shortDesc | Power doubles if the target has a status ailment. |
| BT-1135 | m:hiddenpower.desc | This move's type depends on the user's individual values (IVs), and can be any type but... |
| BT-1136 | m:hiddenpower.shortDesc | Varies in type based on the user's IVs. |
| BT-1137 | m:highhorsepower.shortDesc | No additional effect. |
| BT-1139 | m:highjumpkick.desc | If this attack is not successful, the user loses half of its maximum HP, rounded down, ... |
| BT-1140 | m:highjumpkick.shortDesc | User is hurt by 50% of its max HP if it misses. |
| BT-1141 | m:holdback.desc | Leaves the target with at least 1 HP. |
| BT-1142 | m:holdback.shortDesc | Always leaves the target with at least 1 HP. |
| BT-1143 | m:holdhands.desc | No competitive use. Fails if there is no ally adjacent to the user. |
| BT-1144 | m:holdhands.shortDesc | No competitive use. |
| BT-1145 | m:honeclaws.desc | Raises the user's Attack and accuracy by 1 stage. |
| BT-1146 | m:honeclaws.shortDesc | Raises the user's Attack and accuracy by 1. |
| BT-1147 | m:hornattack.shortDesc | No additional effect. |
| BT-1148 | m:horndrill.desc | Deals damage to the target equal to the target's maximum HP. Ignores accuracy and evasi... |
| BT-1149 | m:horndrill.shortDesc | OHKOs the target. Fails if user is a lower level. |
| BT-1150 | m:hornleech.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-1151 | m:hornleech.shortDesc | User recovers 50% of the damage dealt. |
| BT-1152 | m:howl.desc | Raises the Attack of the user and all allies 1 stage. |
| BT-1153 | m:howl.shortDesc | Raises the user's and ally's Attack by 1. |
| BT-1154 | m:hurricane.desc | Has a 30% chance to confuse the target. This move can hit a target using Bounce, Fly, o... |
| BT-1155 | m:hurricane.shortDesc | 30% chance to confuse target. Can't miss in rain. |
| BT-1156 | m:hydrocannon.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1157 | m:hydrocannon.shortDesc | User cannot move next turn. |
| BT-1158 | m:hydropump.shortDesc | No additional effect. |
| BT-1159 | m:hydrosteam.desc | If the current weather is Sun and the user is not holding Utility Umbrella, this move's... |
| BT-1160 | m:hydrosteam.shortDesc | During Sun: 1.5× damage instead of half. |
| BT-1161 | m:hydrovortex.shortDesc | Power is equal to the base move's Z-Power. |
| BT-1162 | m:hyperbeam.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1163 | m:hyperbeam.shortDesc | User cannot move next turn. |
| BT-1164 | m:hyperdrill.shortDesc | Bypasses protection without breaking it. |
| BT-1165 | m:hyperfang.desc | Has a 10% chance to make the target flinch. |
| BT-1166 | m:hyperfang.shortDesc | 10% chance to make the target flinch. |
| BT-1168 | m:hyperspacefury.desc | Lowers the user's Defense by 1 stage. This move cannot be used successfully unless the ... |
| BT-1170 | m:hyperspacefury.shortDesc | Hoopa-U: Lowers user's Def by 1; breaks protect. |
| BT-1172 | m:hyperspacehole.desc | If this move is successful, it breaks through the target's Baneful Bunker, Detect, King... |
| BT-1173 | m:hyperspacehole.shortDesc | Breaks the target's protection for this turn. |
| BT-1174 | m:hypervoice.desc | No additional effect. |
| BT-1175 | m:hypervoice.shortDesc | No additional effect. Hits adjacent foes. |
| BT-1176 | m:hypnosis.shortDesc | Causes the target to fall asleep. |
| BT-1177 | m:iceball.desc | If this move is successful, the user is locked into this move and cannot make another m... |
| BT-1178 | m:iceball.shortDesc | Power doubles with each hit. Repeats for 5 turns. |
| BT-1179 | m:icebeam.desc | Has a 10% chance to freeze the target. |
| BT-1180 | m:icebeam.shortDesc | 10% chance to freeze the target. |
| BT-1181 | m:iceburn.desc | Has a 30% chance to burn the target. This attack charges on the first turn and executes... |
| BT-1183 | m:iceburn.shortDesc | Charges turn 1. Hits turn 2. 30% burn. |
| BT-1184 | m:icefang.desc | Has a 10% chance to freeze the target and a 10% chance to make it flinch. |
| BT-1185 | m:icefang.shortDesc | 10% chance to freeze. 10% chance to flinch. |
| BT-1186 | m:icehammer.desc | Lowers the user's Speed by 1 stage. |
| BT-1187 | m:icehammer.shortDesc | Lowers the user's Speed by 1. |
| BT-1188 | m:icepunch.desc | Has a 10% chance to freeze the target. |
| BT-1189 | m:icepunch.shortDesc | 10% chance to freeze the target. |
| BT-1190 | m:iceshard.desc | No additional effect. |
| BT-1191 | m:iceshard.shortDesc | Usually goes first. |
| BT-1192 | m:icespinner.desc | Ends the effects of Electric Terrain, Grassy Terrain, Misty Terrain, and Psychic Terrain. |
| BT-1193 | m:icespinner.shortDesc | Ends the effects of terrain. |
| BT-1194 | m:iciclecrash.desc | Has a 30% chance to make the target flinch. |
| BT-1195 | m:iciclecrash.shortDesc | 30% chance to make the target flinch. |
| BT-1196 | m:iciclespear.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-1197 | m:iciclespear.shortDesc | Hits 2-5 times in one turn. |
| BT-1198 | m:icywind.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-1199 | m:icywind.shortDesc | 100% chance to lower the foe(s) Speed by 1. |
| BT-1201 | m:imprison.desc | The user prevents all opposing Pokemon from using any moves that the user also knows as... |
| BT-1202 | m:imprison.shortDesc | No foe can use any move known by the user. |
| BT-1204 | m:incinerate.desc | The target loses its held item if it is a Berry or a Gem. This move cannot cause Pokemo... |
| BT-1206 | m:incinerate.shortDesc | Destroys the foe(s) Berry/Gem. |
| BT-1207 | m:infernalparade.desc | Has a 30% chance to burn the target. Power doubles if the target has a non-volatile sta... |
| BT-1208 | m:infernalparade.shortDesc | 30% burn. 2× power if target is already statused. |
| BT-1209 | m:inferno.desc | Has a 100% chance to burn the target. |
| BT-1210 | m:inferno.shortDesc | 100% chance to burn the target. |
| BT-1211 | m:infernooverdrive.shortDesc | Power is equal to the base move's Z-Power. |
| BT-1212 | m:infestation.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-1213 | m:infestation.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-1216 | m:ingrain.desc | The user has 1/16 of its maximum HP restored at the end of each turn, but it is prevent... |
| BT-1218 | m:ingrain.shortDesc | Traps/grounds user; heals 1/16 max HP per turn. |
| BT-1221 | m:instruct.desc | The target immediately uses its last used move. Fails if the target has not made a move... |
| BT-1222 | m:instruct.shortDesc | The target immediately uses its last used move. |
| BT-1224 | m:iondeluge.desc | Causes Normal-type moves to become Electric type this turn. The effect happens after ot... |
| BT-1225 | m:iondeluge.shortDesc | Normal moves become Electric type this turn. |
| BT-1226 | m:irondefense.desc | Raises the user's Defense by 2 stages. |
| BT-1227 | m:irondefense.shortDesc | Raises the user's Defense by 2. |
| BT-1228 | m:ironhead.desc | Has a 30% chance to make the target flinch. |
| BT-1229 | m:ironhead.shortDesc | 30% chance to make the target flinch. |
| BT-1230 | m:irontail.desc | Has a 30% chance to lower the target's Defense by 1 stage. |
| BT-1231 | m:irontail.shortDesc | 30% chance to lower the target's Defense by 1. |
| BT-1232 | m:ivycudgel.desc | Has a higher chance for a critical hit. If the user is an Ogerpon, this move's type cha... |
| BT-1233 | m:ivycudgel.shortDesc | High critical hit ratio. Type depends on user's form. |
| BT-1234 | m:jawlock.desc | Prevents the user and the target from switching out. The user and the target can still ... |
| BT-1235 | m:jawlock.shortDesc | Prevents both user and target from switching out. |
| BT-1236 | m:jetpunch.desc | No additional effect. |
| BT-1237 | m:jetpunch.shortDesc | Usually goes first. |
| BT-1238 | m:judgment.desc | This move's type depends on the user's held Plate. |
| BT-1239 | m:judgment.shortDesc | Type varies based on the held Plate. |
| BT-1241 | m:jumpkick.desc | If this attack is not successful, the user loses half of its maximum HP, rounded down, ... |
| BT-1242 | m:jumpkick.shortDesc | User is hurt by 50% of its max HP if it misses. |
| BT-1243 | m:junglehealing.desc | Each Pokemon on the user's side restores 1/4 of its maximum HP, rounded half up, and ha... |
| BT-1244 | m:junglehealing.shortDesc | User and allies: healed 1/4 max HP, status cured. |
| BT-1245 | m:karatechop.desc | Has a higher chance for a critical hit. |
| BT-1246 | m:karatechop.shortDesc | High critical hit ratio. |
| BT-1247 | m:kinesis.desc | Lowers the target's accuracy by 1 stage. |
| BT-1248 | m:kinesis.shortDesc | Lowers the target's accuracy by 1. |
| BT-1249 | m:kingsshield.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-1250 | m:kingsshield.shortDesc | Protects from damaging attacks. Contact: -1 Atk. |
| BT-1251 | m:knockoff.desc | This move's power is multiplied by 1.5 if the target is holding an item, and the target... |
| BT-1253 | m:knockoff.shortDesc | 1.5× damage if foe holds an item. Removes item. |
| BT-1254 | m:kowtowcleave.shortDesc | This move does not check accuracy. |
| BT-1255 | m:landswrath.desc | No additional effect. |
| BT-1256 | m:landswrath.shortDesc | No additional effect. Hits adjacent foes. |
| BT-1257 | m:laserfocus.desc | Until the end of the next turn, the user's attacks will be critical hits. |
| BT-1258 | m:laserfocus.shortDesc | Until the end of the next turn, user's moves crit. |
| BT-1260 | m:lashout.desc | Power doubles if the user had a stat stage lowered this turn. |
| BT-1261 | m:lashout.shortDesc | 2× power if the user had a stat lowered this turn. |
| BT-1262 | m:lastresort.desc | This move fails unless the user knows this move and at least one other move, and has us... |
| BT-1263 | m:lastresort.shortDesc | Fails unless each known move has been used. |
| BT-1264 | m:lastrespects.desc | Power is equal to 50+(X × 50), where X is the total number of times any Pokemon has fai... |
| BT-1265 | m:lastrespects.shortDesc | +50 power for each time a party member fainted. |
| BT-1266 | m:lavaplume.desc | Has a 30% chance to burn the target. |
| BT-1267 | m:lavaplume.shortDesc | 30% chance to burn adjacent Pokemon. |
| BT-1268 | m:leafage.shortDesc | No additional effect. |
| BT-1269 | m:leafblade.desc | Has a higher chance for a critical hit. |
| BT-1270 | m:leafblade.shortDesc | High critical hit ratio. |
| BT-1271 | m:leafstorm.desc | Lowers the user's Special Attack by 2 stages. |
| BT-1272 | m:leafstorm.shortDesc | Lowers the user's Sp. Atk by 2. |
| BT-1273 | m:leaftornado.desc | Has a 50% chance to lower the target's accuracy by 1 stage. |
| BT-1274 | m:leaftornado.shortDesc | 50% chance to lower the target's accuracy by 1. |
| BT-1275 | m:leechlife.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-1276 | m:leechlife.shortDesc | User recovers 50% of the damage dealt. |
| BT-1278 | m:leechseed.desc | The Pokemon at the user's position steals 1/8 of the target's maximum HP, rounded down,... |
| BT-1280 | m:leechseed.shortDesc | 1/8 of target's HP is restored to user every turn. |
| BT-1282 | m:leer.desc | Lowers the target's Defense by 1 stage. |
| BT-1283 | m:leer.shortDesc | Lowers the foe(s) Defense by 1. |
| BT-1284 | m:letssnuggleforever.shortDesc | No additional effect. |
| BT-1285 | m:lick.desc | Has a 30% chance to paralyze the target. |
| BT-1286 | m:lick.shortDesc | 30% chance to paralyze the target. |
| BT-1287 | m:lifedew.desc | Each Pokemon on the user's side restores 1/4 of its maximum HP, rounded half up. |
| BT-1288 | m:lifedew.shortDesc | Heals the user and its allies by 1/4 their max HP. |
| BT-1289 | m:lightofruin.desc | If the target lost HP, the user takes recoil damage equal to 1/2 the HP lost by the tar... |
| BT-1290 | m:lightofruin.shortDesc | Has 1/2 recoil. |
| BT-1291 | m:lightscreen.desc | For 5 turns, the user and its party members take 0.5× damage from special attacks, or 0... |
| BT-1293 | m:lightscreen.shortDesc | For 5 turns, special damage to allies is halved. |
| BT-1295 | m:lightthatburnsthesky.desc | This move becomes a physical attack if the user's Attack is greater than its Special At... |
| BT-1296 | m:lightthatburnsthesky.shortDesc | Physical if user's Atk > Sp. Atk. Ignores Abilities. |
| BT-1297 | m:liquidation.desc | Has a 20% chance to lower the target's Defense by 1 stage. |
| BT-1298 | m:liquidation.shortDesc | 20% chance to lower the target's Defense by 1. |
| BT-1299 | m:lockon.desc | Until the end of the next turn, the target cannot avoid the user's moves, even if the t... |
| BT-1300 | m:lockon.shortDesc | User's next move will not miss the target. |
| BT-1302 | m:lovelykiss.shortDesc | Causes the target to fall asleep. |
| BT-1303 | m:lowkick.desc | This move's power is 20 if the target weighs less than 10 kg, 40 if less than 25 kg, 60... |
| BT-1304 | m:lowkick.shortDesc | More power the heavier the target. |
| BT-1305 | m:lowsweep.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-1306 | m:lowsweep.shortDesc | 100% chance to lower the target's Speed by 1. |
| BT-1307 | m:luckychant.desc | For 5 turns, the user and its party members cannot be struck by a critical hit. Fails i... |
| BT-1309 | m:luckychant.shortDesc | For 5 turns, shields user's party from critical hits. |
| BT-1311 | m:luminacrash.desc | Has a 100% chance to lower the target's Special Defense by 2 stages. |
| BT-1312 | m:luminacrash.shortDesc | 100% chance to lower the target's Sp. Def by 2. |
| BT-1313 | m:lunarblessing.desc | Each Pokemon on the user's side restores 1/4 of its maximum HP, rounded half up, and ha... |
| BT-1314 | m:lunarblessing.shortDesc | User and allies: healed 1/4 max HP, status cured. |
| BT-1315 | m:lunardance.desc | The user faints, and if the Pokemon brought out to replace it does not have full HP or ... |
| BT-1317 | m:lunardance.shortDesc | User faints. Next hurt Pkmn is cured, max HP/PP. |
| BT-1318 | m:lunge.desc | Has a 100% chance to lower the target's Attack by 1 stage. |
| BT-1319 | m:lunge.shortDesc | 100% chance to lower the target's Attack by 1. |
| BT-1320 | m:lusterpurge.desc | Has a 50% chance to lower the target's Special Defense by 1 stage. |
| BT-1321 | m:lusterpurge.shortDesc | 50% chance to lower the target's Sp. Def by 1. |
| BT-1322 | m:machpunch.desc | No additional effect. |
| BT-1323 | m:machpunch.shortDesc | Usually goes first. |
| BT-1324 | m:magicalleaf.shortDesc | This move does not check accuracy. |
| BT-1325 | m:magicaltorque.desc | Has a 30% chance to confuse the target. |
| BT-1326 | m:magicaltorque.shortDesc | 30% chance to confuse the target. |
| BT-1327 | m:magiccoat.desc | Until the end of the turn, the user is unaffected by certain non-damaging moves directe... |
| BT-1329 | m:magiccoat.shortDesc | Bounces back certain non-damaging moves. |
| BT-1331 | m:magicpowder.desc | Causes the target to become a Psychic type. Fails if the target is an Arceus or a Silva... |
| BT-1332 | m:magicpowder.shortDesc | Changes the target's type to Psychic. |
| BT-1333 | m:magicroom.desc | For 5 turns, the held items of all active Pokemon have no effect. An item's effect of c... |
| BT-1334 | m:magicroom.shortDesc | For 5 turns, all held items have no effect. |
| BT-1335 | m:magmastorm.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-1336 | m:magmastorm.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-1338 | m:magnetbomb.shortDesc | This move does not check accuracy. |
| BT-1339 | m:magneticflux.desc | Raises the Defense and Special Defense of Pokemon on the user's side with the Plus or M... |
| BT-1340 | m:magneticflux.shortDesc | Raises Def, Sp. Def of allies with Plus/Minus by 1. |
| BT-1341 | m:magnetrise.desc | For 5 turns, the user is immune to Ground-type attacks and the effects of Spikes, Toxic... |
| BT-1343 | m:magnetrise.shortDesc | For 5 turns, the user has immunity to Ground. |
| BT-1346 | m:magnitude.desc | The power of this move varies; 5% chances for 10 and 150 power, 10% chances for 30 and ... |
| BT-1347 | m:magnitude.shortDesc | Hits adjacent Pokemon. Power varies; 2× on Dig. |
| BT-1349 | m:makeitrain.desc | Lowers the user's Special Attack by 1 stage. |
| BT-1350 | m:makeitrain.shortDesc | Lowers the user's Sp. Atk by 1. Hits foe(s). |
| BT-1351 | m:maliciousmoonsault.desc | Damage doubles and no accuracy check is done if the target has used Minimize while active. |
| BT-1352 | m:maliciousmoonsault.shortDesc | Damage doubles if the target used Minimize. |
| BT-1353 | m:malignantchain.desc | Has a 50% chance to badly poison the target. |
| BT-1354 | m:malignantchain.shortDesc | 50% chance to badly poison the target. |
| BT-1356 | m:matblock.desc | The user and its party members are protected from damaging attacks made by other Pokemo... |
| BT-1357 | m:matblock.shortDesc | Protects allies from damaging attacks. Turn 1 only. |
| BT-1359 | m:matchagotcha.desc | Has a 20% chance to burn the target. The user recovers 1/2 the HP lost by the target, r... |
| BT-1360 | m:matchagotcha.shortDesc | 20% burn. Recovers 50% dmg dealt. Thaws foe(s). |
| BT-1361 | m:maxairstream.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speed... |
| BT-1362 | m:maxairstream.shortDesc | Base move affects power. Allies: +1 Speed. |
| BT-1363 | m:maxdarkness.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speci... |
| BT-1364 | m:maxdarkness.shortDesc | Base move affects power. Foes: -1 Sp. Def. |
| BT-1365 | m:maxflare.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1366 | m:maxflare.shortDesc | Base move affects power. Starts Sun. |
| BT-1367 | m:maxflutterby.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speci... |
| BT-1368 | m:maxflutterby.shortDesc | Base move affects power. Foes: -1 Sp. Atk. |
| BT-1369 | m:maxgeyser.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1370 | m:maxgeyser.shortDesc | Base move affects power. Starts Rain. |
| BT-1372 | m:maxguard.desc | The user is protected from nearly all attacks made by other Pokemon during this turn, i... |
| BT-1373 | m:maxguard.shortDesc | Protects user from moves & Max Moves this turn. |
| BT-1374 | m:maxhailstorm.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1375 | m:maxhailstorm.shortDesc | Base move affects power. Starts Hail. |
| BT-1376 | m:maxknuckle.desc | Boosts the user and its allies' Attack by 1 stage. BP scales with the base move's BP. T... |
| BT-1377 | m:maxknuckle.shortDesc | Base move affects power. Allies: +1 Attack. |
| BT-1378 | m:maxlightning.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1379 | m:maxlightning.shortDesc | Base move affects power. Starts Electric Terrain. |
| BT-1380 | m:maxmindstorm.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1381 | m:maxmindstorm.shortDesc | Base move affects power. Starts Psychic Terrain. |
| BT-1382 | m:maxooze.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speci... |
| BT-1383 | m:maxooze.shortDesc | Base move affects power. Allies: +1 Sp. Atk. |
| BT-1384 | m:maxovergrowth.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1385 | m:maxovergrowth.shortDesc | Base move affects power. Starts Grassy Terrain. |
| BT-1386 | m:maxphantasm.desc | Power is equal to the base move's Max Move power. If this move is successful, the Defen... |
| BT-1387 | m:maxphantasm.shortDesc | Base move affects power. Foes: -1 Defense. |
| BT-1388 | m:maxquake.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speci... |
| BT-1389 | m:maxquake.shortDesc | Base move affects power. Allies: +1 Sp. Def. |
| BT-1390 | m:maxrockfall.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1391 | m:maxrockfall.shortDesc | Base move affects power. Starts Sandstorm. |
| BT-1392 | m:maxstarfall.desc | Power is equal to the base move's Max Move power. If this move is successful, the effec... |
| BT-1393 | m:maxstarfall.shortDesc | Base move affects power. Starts Misty Terrain. |
| BT-1394 | m:maxsteelspike.desc | Power is equal to the base move's Max Move power. If this move is successful, the Defen... |
| BT-1395 | m:maxsteelspike.shortDesc | Base move affects power. Allies: +1 Defense. |
| BT-1396 | m:maxstrike.desc | Power is equal to the base move's Max Move power. If this move is successful, the Speed... |
| BT-1397 | m:maxstrike.shortDesc | Base move affects power. Foes: -1 Speed. |
| BT-1398 | m:maxwyrmwind.desc | Power is equal to the base move's Max Move power. If this move is successful, the Attac... |
| BT-1399 | m:maxwyrmwind.shortDesc | Base move affects power. Foes: -1 Attack. |
| BT-1400 | m:meanlook.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-1401 | m:meanlook.shortDesc | Prevents the target from switching out. |
| BT-1402 | m:meditate.desc | Raises the user's Attack by 1 stage. |
| BT-1403 | m:meditate.shortDesc | Raises the user's Attack by 1. |
| BT-1404 | m:mefirst.desc | The user uses the move the target chose for use this turn against it, if possible, with... |
| BT-1405 | m:mefirst.shortDesc | Copies a foe at 1.5× power. User must be faster. |
| BT-1406 | m:megadrain.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-1407 | m:megadrain.shortDesc | User recovers 50% of the damage dealt. |
| BT-1408 | m:megahorn.shortDesc | No additional effect. |
| BT-1409 | m:megakick.shortDesc | No additional effect. |
| BT-1410 | m:megapunch.shortDesc | No additional effect. |
| BT-1411 | m:memento.desc | Lowers the target's Attack and Special Attack by 2 stages. The user faints unless this ... |
| BT-1413 | m:memento.shortDesc | Lowers target's Attack, Sp. Atk by 2. User faints. |
| BT-1414 | m:menacingmoonrazemaelstrom.desc | This move and its effects ignore the Abilities of other Pokemon. |
| BT-1415 | m:menacingmoonrazemaelstrom.shortDesc | Ignores the Abilities of other Pokemon. |
| BT-1416 | m:metalburst.desc | Deals damage to the last opposing Pokemon to hit the user with a physical or special at... |
| BT-1417 | m:metalburst.shortDesc | If hit by an attack, returns 1.5× damage. |
| BT-1418 | m:metalclaw.desc | Has a 10% chance to raise the user's Attack by 1 stage. |
| BT-1419 | m:metalclaw.shortDesc | 10% chance to raise the user's Attack by 1. |
| BT-1420 | m:metalsound.desc | Lowers the target's Special Defense by 2 stages. |
| BT-1421 | m:metalsound.shortDesc | Lowers the target's Sp. Def by 2. |
| BT-1422 | m:meteorassault.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1423 | m:meteorassault.shortDesc | User cannot move next turn. |
| BT-1424 | m:meteorbeam.desc | This attack charges on the first turn and executes on the second. Raises the user's Spe... |
| BT-1426 | m:meteorbeam.shortDesc | Raises user's Sp. Atk by 1 on turn 1. Hits turn 2. |
| BT-1427 | m:meteormash.desc | Has a 20% chance to raise the user's Attack by 1 stage. |
| BT-1428 | m:meteormash.shortDesc | 20% chance to raise the user's Attack by 1. |
| BT-1429 | m:metronome.desc | A random move is selected for use, other than After You, Apple Acid, Armor Cannon, Assi... |
| BT-1431 | m:metronome.shortDesc | Picks a random move. |
| BT-1432 | m:mightycleave.shortDesc | Bypasses protection without breaking it. |
| BT-1433 | m:milkdrink.desc | The user restores 1/2 of its maximum HP, rounded half up. |
| BT-1434 | m:milkdrink.shortDesc | Heals the user by 50% of its max HP. |
| BT-1435 | m:mimic.desc | While the user remains active, this move is replaced by the last move used by the targe... |
| BT-1436 | m:mimic.shortDesc | The last move the target used replaces this one. |
| BT-1439 | m:mindblown.desc | Whether or not this move is successful and even if it would cause fainting, the user lo... |
| BT-1440 | m:mindblown.shortDesc | User loses 50% max HP. Hits adjacent Pokemon. |
| BT-1441 | m:mindreader.desc | Until the end of the next turn, the target cannot avoid the user's moves, even if the t... |
| BT-1442 | m:mindreader.shortDesc | User's next move will not miss the target. |
| BT-1444 | m:minimize.desc | Raises the user's evasiveness by 2 stages. Whether or not the user's evasiveness was ch... |
| BT-1445 | m:minimize.shortDesc | Raises the user's evasiveness by 2. |
| BT-1446 | m:miracleeye.desc | As long as the target remains active, its evasiveness stat stage is ignored during accu... |
| BT-1447 | m:miracleeye.shortDesc | Psychic hits Dark. Evasiveness ignored. |
| BT-1449 | m:mirrorcoat.desc | Deals damage to the last opposing Pokemon to hit the user with a special attack this tu... |
| BT-1450 | m:mirrorcoat.shortDesc | If hit by special attack, returns double damage. |
| BT-1451 | m:mirrormove.desc | The user uses the last move used by the target. The copied move is used against that ta... |
| BT-1452 | m:mirrormove.shortDesc | User uses the target's last used move against it. |
| BT-1453 | m:mirrorshot.desc | Has a 30% chance to lower the target's accuracy by 1 stage. |
| BT-1454 | m:mirrorshot.shortDesc | 30% chance to lower the target's accuracy by 1. |
| BT-1456 | m:mist.desc | For 5 turns, the user and its party members are protected from having their stat stages... |
| BT-1458 | m:mist.shortDesc | For 5 turns, protects user's party from stat drops. |
| BT-1460 | m:mistball.desc | Has a 50% chance to lower the target's Special Attack by 1 stage. |
| BT-1461 | m:mistball.shortDesc | 50% chance to lower the target's Sp. Atk by 1. |
| BT-1462 | m:mistyexplosion.desc | If the current terrain is Misty Terrain and the user is grounded, this move's power is ... |
| BT-1463 | m:mistyexplosion.shortDesc | User faints. User on Misty Terrain: 1.5× power. |
| BT-1464 | m:mistyterrain.desc | For 5 turns, the terrain becomes Misty Terrain. During the effect, the power of Dragon-... |
| BT-1465 | m:mistyterrain.shortDesc | 5 turns. Can't status,-Dragon power vs grounded. |
| BT-1466 | m:moonblast.desc | Has a 30% chance to lower the target's Special Attack by 1 stage. |
| BT-1467 | m:moonblast.shortDesc | 30% chance to lower the target's Sp. Atk by 1. |
| BT-1468 | m:moongeistbeam.desc | This move and its effects ignore the Abilities of other Pokemon. |
| BT-1469 | m:moongeistbeam.shortDesc | Ignores the Abilities of other Pokemon. |
| BT-1470 | m:moonlight.desc | The user restores 1/2 of its maximum HP if Strong Winds or no weather conditions are in... |
| BT-1471 | m:moonlight.shortDesc | Heals the user by a weather-dependent amount. |
| BT-1472 | m:morningsun.desc | The user restores 1/2 of its maximum HP if Strong Winds or no weather conditions are in... |
| BT-1473 | m:morningsun.shortDesc | Heals the user by a weather-dependent amount. |
| BT-1474 | m:mortalspin.desc | If this move is successful and the user has not fainted, the effects of Leech Seed and ... |
| BT-1475 | m:mortalspin.shortDesc | Poisons foes, frees user from hazards/bind/leech. |
| BT-1476 | m:mountaingale.desc | Has a 30% chance to make the target flinch. |
| BT-1477 | m:mountaingale.shortDesc | 30% chance to make the target flinch. |
| BT-1478 | m:mudbomb.desc | Has a 30% chance to lower the target's accuracy by 1 stage. |
| BT-1479 | m:mudbomb.shortDesc | 30% chance to lower the target's accuracy by 1. |
| BT-1480 | m:muddywater.desc | Has a 30% chance to lower the target's accuracy by 1 stage. |
| BT-1481 | m:muddywater.shortDesc | 30% chance to lower the foe(s) accuracy by 1. |
| BT-1482 | m:mudshot.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-1483 | m:mudshot.shortDesc | 100% chance to lower the target's Speed by 1. |
| BT-1484 | m:mudslap.desc | Has a 100% chance to lower the target's accuracy by 1 stage. |
| BT-1485 | m:mudslap.shortDesc | 100% chance to lower the target's accuracy by 1. |
| BT-1486 | m:mudsport.desc | For 5 turns, all Electric-type attacks used by any active Pokemon have their power mult... |
| BT-1487 | m:mudsport.shortDesc | For 5 turns, Electric-type attacks have 1/3 power. |
| BT-1488 | m:multiattack.desc | This move's type depends on the user's held Memory. |
| BT-1489 | m:multiattack.shortDesc | Type varies based on the held Memory. |
| BT-1490 | m:mysticalfire.desc | Has a 100% chance to lower the target's Special Attack by 1 stage. |
| BT-1491 | m:mysticalfire.shortDesc | 100% chance to lower the target's Sp. Atk by 1. |
| BT-1492 | m:mysticalpower.desc | Has a 100% chance to raise the user's Special Attack by 1 stage. |
| BT-1493 | m:mysticalpower.shortDesc | 100% chance to raise the user's Sp. Atk by 1. |
| BT-1494 | m:nastyplot.desc | Raises the user's Special Attack by 2 stages. |
| BT-1495 | m:nastyplot.shortDesc | Raises the user's Sp. Atk by 2. |
| BT-1496 | m:naturalgift.desc | The type and power of this move depend on the user's held Berry, and the Berry is lost.... |
| BT-1497 | m:naturalgift.shortDesc | Power and type depends on the user's Berry. |
| BT-1498 | m:naturepower.desc | This move calls another move for use based on the battle terrain. Tri Attack on the reg... |
| BT-1500 | m:naturepower.shortDesc | Attack depends on terrain (default Tri Attack). |
| BT-1501 | m:naturesmadness.desc | Deals damage to the target equal to half of its current HP, rounded down, but not less ... |
| BT-1502 | m:naturesmadness.shortDesc | Does damage equal to 1/2 target's current HP. |
| BT-1503 | m:needlearm.desc | Has a 30% chance to make the target flinch. |
| BT-1504 | m:needlearm.shortDesc | 30% chance to make the target flinch. |
| BT-1505 | m:neverendingnightmare.shortDesc | Power is equal to the base move's Z-Power. |
| BT-1506 | m:nightdaze.desc | Has a 40% chance to lower the target's accuracy by 1 stage. |
| BT-1507 | m:nightdaze.shortDesc | 40% chance to lower the target's accuracy by 1. |
| BT-1509 | m:nightmare.desc | Causes the target to lose 1/4 of its maximum HP, rounded down, at the end of each turn ... |
| BT-1510 | m:nightmare.shortDesc | A sleeping target is hurt by 1/4 max HP per turn. |
| BT-1512 | m:nightshade.desc | Deals damage to the target equal to the user's level. |
| BT-1513 | m:nightshade.shortDesc | Does damage equal to the user's level. |
| BT-1514 | m:nightslash.desc | Has a higher chance for a critical hit. |
| BT-1515 | m:nightslash.shortDesc | High critical hit ratio. |
| BT-1516 | m:nobleroar.desc | Lowers the target's Attack and Special Attack by 1 stage. |
| BT-1517 | m:nobleroar.shortDesc | Lowers the target's Attack and Sp. Atk by 1. |
| BT-1518 | m:noretreat.desc | Raises the user's Attack, Defense, Special Attack, Special Defense, and Speed by 1 stag... |
| BT-1519 | m:noretreat.shortDesc | Raises all stats by 1 (not acc/eva). Traps user. |
| BT-1521 | m:noxioustorque.desc | Has a 30% chance to poison the target. |
| BT-1522 | m:noxioustorque.shortDesc | 30% chance to poison the target. |
| BT-1523 | m:nuzzle.desc | Has a 100% chance to paralyze the target. |
| BT-1524 | m:nuzzle.shortDesc | 100% chance to paralyze the target. |
| BT-1525 | m:oblivionwing.desc | The user recovers 3/4 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-1526 | m:oblivionwing.shortDesc | User recovers 75% of the damage dealt. |
| BT-1527 | m:obstruct.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-1528 | m:obstruct.shortDesc | Protects from damaging attacks. Contact: -2 Def. |
| BT-1529 | m:oceanicoperetta.shortDesc | No additional effect. |
| BT-1530 | m:octazooka.desc | Has a 50% chance to lower the target's accuracy by 1 stage. |
| BT-1531 | m:octazooka.shortDesc | 50% chance to lower the target's accuracy by 1. |
| BT-1532 | m:octolock.desc | Prevents the target from switching out. At the end of each turn during effect, the targ... |
| BT-1533 | m:octolock.shortDesc | Traps target, lowers Def and SpD by 1 each turn. |
| BT-1535 | m:odorsleuth.desc | As long as the target remains active, its evasiveness stat stage is ignored during accu... |
| BT-1536 | m:odorsleuth.shortDesc | Fighting, Normal hit Ghost. Evasiveness ignored. |
| BT-1537 | m:ominouswind.desc | Has a 10% chance to raise the user's Attack, Defense, Special Attack, Special Defense, ... |
| BT-1538 | m:ominouswind.shortDesc | 10% chance to raise all stats by 1 (not acc/eva). |
| BT-1539 | m:orderup.desc | If an ally Tatsugiri has activated its Commander Ability, this move raises the user's A... |
| BT-1540 | m:orderup.shortDesc | Curly\|Droopy\|Stretchy eaten: +1 Atk\|Def\|Spe. |
| BT-1541 | m:originpulse.desc | No additional effect. |
| BT-1542 | m:originpulse.shortDesc | No additional effect. Hits adjacent foes. |
| BT-1543 | m:outrage.desc | The user spends two or three turns locked into this move and becomes confused immediate... |
| BT-1544 | m:outrage.shortDesc | Lasts 2-3 turns. Confuses the user afterwards. |
| BT-1545 | m:overdrive.desc | No additional effect. |
| BT-1546 | m:overdrive.shortDesc | No additional effect. Hits foe(s). |
| BT-1547 | m:overheat.desc | Lowers the user's Special Attack by 2 stages. |
| BT-1548 | m:overheat.shortDesc | Lowers the user's Sp. Atk by 2. |
| BT-1550 | m:painsplit.desc | The user and the target's HP become the average of their current HP, rounded down, but ... |
| BT-1551 | m:painsplit.shortDesc | Shares HP of user and target equally. |
| BT-1552 | m:paleowave.desc | Has a 20% chance to lower the target's Attack by 1 stage. |
| BT-1553 | m:paleowave.shortDesc | 20% chance to lower the target's Attack by 1. |
| BT-1554 | m:paraboliccharge.desc | The user recovers 1/2 the HP lost by the target, rounded half up. If Big Root is held b... |
| BT-1555 | m:paraboliccharge.shortDesc | User recovers 50% of the damage dealt. |
| BT-1556 | m:partingshot.desc | Lowers the target's Attack and Special Attack by 1 stage. If this move is successful, t... |
| BT-1558 | m:partingshot.shortDesc | Lowers target's Atk, Sp. Atk by 1. User switches. |
| BT-1560 | m:payback.desc | Power doubles if the user moves after the target this turn, including actions taken thr... |
| BT-1561 | m:payback.shortDesc | Power doubles if the user moves after the target. |
| BT-1563 | m:payday.desc | No additional effect. |
| BT-1564 | m:payday.shortDesc | Scatters coins. |
| BT-1565 | m:peck.shortDesc | No additional effect. |
| BT-1567 | m:perishsong.desc | Each active Pokemon receives a perish count of 4 if it doesn't already have a perish co... |
| BT-1568 | m:perishsong.shortDesc | All active Pokemon will faint in 3 turns. |
| BT-1570 | m:petalblizzard.desc | No additional effect. |
| BT-1571 | m:petalblizzard.shortDesc | No additional effect. Hits adjacent Pokemon. |
| BT-1572 | m:petaldance.desc | The user spends two or three turns locked into this move and becomes confused immediate... |
| BT-1573 | m:petaldance.shortDesc | Lasts 2-3 turns. Confuses the user afterwards. |
| BT-1575 | m:phantomforce.desc | If this move is successful, it breaks through the target's Baneful Bunker, Detect, King... |
| BT-1577 | m:phantomforce.shortDesc | Disappears turn 1. Hits turn 2. Breaks protection. |
| BT-1578 | m:photongeyser.desc | This move becomes a physical attack if the user's Attack is greater than its Special At... |
| BT-1579 | m:photongeyser.shortDesc | Physical if user's Atk > Sp. Atk. Ignores Abilities. |
| BT-1580 | m:pikapapow.desc | Power is equal to the greater of (user's Happiness × 2/5), rounded down, or 1. |
| BT-1581 | m:pikapapow.shortDesc | Max happiness: 102 power. Can't miss. |
| BT-1582 | m:pinmissile.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-1583 | m:pinmissile.shortDesc | Hits 2-5 times in one turn. |
| BT-1584 | m:plasmafists.desc | If this move is successful, causes Normal-type moves to become Electric type this turn. |
| BT-1585 | m:plasmafists.shortDesc | Normal moves become Electric type this turn. |
| BT-1586 | m:playnice.desc | Lowers the target's Attack by 1 stage. |
| BT-1587 | m:playnice.shortDesc | Lowers the target's Attack by 1. |
| BT-1588 | m:playrough.desc | Has a 10% chance to lower the target's Attack by 1 stage. |
| BT-1589 | m:playrough.shortDesc | 10% chance to lower the target's Attack by 1. |
| BT-1590 | m:pluck.desc | If this move is successful and the user has not fainted, it steals the target's held Be... |
| BT-1592 | m:pluck.shortDesc | User steals and eats the target's Berry. |
| BT-1593 | m:poisonfang.desc | Has a 50% chance to badly poison the target. |
| BT-1594 | m:poisonfang.shortDesc | 50% chance to badly poison the target. |
| BT-1595 | m:poisongas.desc | Poisons the target. |
| BT-1596 | m:poisongas.shortDesc | Poisons the foe(s). |
| BT-1597 | m:poisonjab.desc | Has a 30% chance to poison the target. |
| BT-1598 | m:poisonjab.shortDesc | 30% chance to poison the target. |
| BT-1599 | m:poisonpowder.desc | Poisons the target. |
| BT-1600 | m:poisonpowder.shortDesc | Poisons the target. |
| BT-1601 | m:poisonsting.desc | Has a 30% chance to poison the target. |
| BT-1602 | m:poisonsting.shortDesc | 30% chance to poison the target. |
| BT-1603 | m:poisontail.desc | Has a 10% chance to poison the target and a higher chance for a critical hit. |
| BT-1604 | m:poisontail.shortDesc | High critical hit ratio. 10% chance to poison. |
| BT-1605 | m:polarflare.desc | Has a 10% chance to freeze the target. This move cannot thaw a frozen target. If this m... |
| BT-1606 | m:polarflare.shortDesc | 10% chance to frz foe(s). Ramnarok transforms. |
| BT-1607 | m:pollenpuff.desc | If the target is an ally, this move restores 1/2 of its maximum HP, rounded down, inste... |
| BT-1608 | m:pollenpuff.shortDesc | If the target is an ally, heals 50% of its max HP. |
| BT-1610 | m:poltergeist.shortDesc | Fails if the target has no held item. |
| BT-1611 | m:populationbomb.desc | Hits ten times. This move checks accuracy for each hit, and the attack ends if the targ... |
| BT-1612 | m:populationbomb.shortDesc | Hits 10 times. Each hit can miss. |
| BT-1613 | m:pounce.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-1614 | m:pounce.shortDesc | 100% chance to lower the target's Speed by 1. |
| BT-1615 | m:pound.shortDesc | No additional effect. |
| BT-1617 | m:powder.desc | If the target uses a Fire-type move this turn, it is prevented from executing and the t... |
| BT-1618 | m:powder.shortDesc | If using a Fire move, target loses 1/4 max HP. |
| BT-1620 | m:powdersnow.desc | Has a 10% chance to freeze the target. |
| BT-1621 | m:powdersnow.shortDesc | 10% chance to freeze the foe(s). |
| BT-1622 | m:powergem.shortDesc | No additional effect. |
| BT-1623 | m:powershift.desc | The user swaps its Attack and Defense stats, and stat stage changes remain on their res... |
| BT-1624 | m:powershift.shortDesc | Switches user's Attack and Defense stats. |
| BT-1627 | m:powersplit.desc | The user and the target have their Attack and Special Attack stats set to be equal to t... |
| BT-1628 | m:powersplit.shortDesc | Averages Attack and Sp. Atk stats with target. |
| BT-1629 | m:powerswap.desc | The user swaps its Attack and Special Attack stat stage changes with the target. |
| BT-1630 | m:powerswap.shortDesc | Swaps Attack and Sp. Atk stat stages with target. |
| BT-1631 | m:powertrick.desc | The user swaps its Attack and Defense stats, and stat stage changes remain on their res... |
| BT-1632 | m:powertrick.shortDesc | Switches user's Attack and Defense stats. |
| BT-1634 | m:powertrip.desc | Power is equal to 20+(X × 20), where X is the user's total stat stage changes that are ... |
| BT-1635 | m:powertrip.shortDesc | + 20 power for each of the user's stat boosts. |
| BT-1636 | m:poweruppunch.desc | Has a 100% chance to raise the user's Attack by 1 stage. |
| BT-1637 | m:poweruppunch.shortDesc | 100% chance to raise the user's Attack by 1. |
| BT-1638 | m:powerwhip.shortDesc | No additional effect. |
| BT-1639 | m:precipiceblades.desc | No additional effect. |
| BT-1640 | m:precipiceblades.shortDesc | No additional effect. Hits adjacent foes. |
| BT-1641 | m:present.desc | If this move is successful, it deals damage or heals the target. 40% chance for 40 powe... |
| BT-1642 | m:present.shortDesc | 40, 80, 120 power, or heals target 1/4 max HP. |
| BT-1643 | m:prismaticlaser.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1644 | m:prismaticlaser.shortDesc | User cannot move next turn. |
| BT-1646 | m:protect.desc | The user is protected from most attacks made by other Pokemon during this turn. This mo... |
| BT-1647 | m:protect.shortDesc | Prevents moves from affecting the user this turn. |
| BT-1649 | m:psybeam.desc | Has a 10% chance to confuse the target. |
| BT-1650 | m:psybeam.shortDesc | 10% chance to confuse the target. |
| BT-1651 | m:psyblade.desc | If the current terrain is Electric Terrain, this move's power is multiplied by 1.5. |
| BT-1652 | m:psyblade.shortDesc | During Electric Terrain: 1.5× power. |
| BT-1653 | m:psychic.desc | Has a 10% chance to lower the target's Special Defense by 1 stage. |
| BT-1654 | m:psychic.shortDesc | 10% chance to lower the target's Sp. Def by 1. |
| BT-1655 | m:psychicfangs.desc | If this attack does not miss, the effects of Reflect, Light Screen, and Aurora Veil end... |
| BT-1656 | m:psychicfangs.shortDesc | Destroys screens, unless the target is immune. |
| BT-1657 | m:psychicnoise.desc | For 2 turns, the target is prevented from restoring any HP as long as it remains active... |
| BT-1658 | m:psychicnoise.shortDesc | For 2 turns, the target is prevented from healing. |
| BT-1659 | m:psychicterrain.desc | For 5 turns, the terrain becomes Psychic Terrain. During the effect, the power of Psych... |
| BT-1660 | m:psychicterrain.shortDesc | 5 turns. Grounded: +Psychic power, priority-safe. |
| BT-1661 | m:psychoboost.desc | Lowers the user's Special Attack by 2 stages. |
| BT-1662 | m:psychoboost.shortDesc | Lowers the user's Sp. Atk by 2. |
| BT-1663 | m:psychocut.desc | Has a higher chance for a critical hit. |
| BT-1664 | m:psychocut.shortDesc | High critical hit ratio. |
| BT-1665 | m:psychoshift.desc | The user's non-volatile status condition is transferred to the target, and the user is ... |
| BT-1666 | m:psychoshift.shortDesc | Transfers the user's status ailment to the target. |
| BT-1667 | m:psychup.desc | The user copies all of the target's current stat stage changes. |
| BT-1668 | m:psychup.shortDesc | Copies the target's current stat stages. |
| BT-1669 | m:psyshieldbash.desc | Has a 100% chance to raise the user's Defense by 1 stage. |
| BT-1670 | m:psyshieldbash.shortDesc | 100% chance to raise the user's Defense by 1. |
| BT-1671 | m:psyshock.desc | Deals damage to the target based on its Defense instead of Special Defense. |
| BT-1672 | m:psyshock.shortDesc | Damages target based on Defense, not Sp. Def. |
| BT-1673 | m:psystrike.desc | Deals damage to the target based on its Defense instead of Special Defense. |
| BT-1674 | m:psystrike.shortDesc | Damages target based on Defense, not Sp. Def. |
| BT-1675 | m:psywave.desc | Deals damage to the target equal to (user's level) × (X + 50) / 100, where X is a rando... |
| BT-1676 | m:psywave.shortDesc | Random damage equal to 0.5×-1.5× user's level. |
| BT-1677 | m:pulverizingpancake.shortDesc | No additional effect. |
| BT-1678 | m:punishment.desc | Power is equal to 60+(X × 20), where X is the target's total stat stage changes that ar... |
| BT-1679 | m:punishment.shortDesc | 60 power +20 for each of the target's stat boosts. |
| BT-1680 | m:purify.desc | The target is cured if it has a non-volatile status condition. If the target was cured,... |
| BT-1681 | m:purify.shortDesc | Cures target's status; heals user 1/2 max HP if so. |
| BT-1683 | m:pursuit.desc | If an opposing Pokemon switches out this turn, this move hits that Pokemon before it le... |
| BT-1684 | m:pursuit.shortDesc | If a foe is switching out, hits it at 2× power. |
| BT-1685 | m:pyroball.desc | Has a 10% chance to burn the target. |
| BT-1686 | m:pyroball.shortDesc | 10% chance to burn the target. Thaws user. |
| BT-1688 | m:quash.desc | Causes the target to take its turn after all other Pokemon this turn, no matter the pri... |
| BT-1689 | m:quash.shortDesc | Forces the target to move last this turn. |
| BT-1690 | m:quickattack.desc | No additional effect. |
| BT-1691 | m:quickattack.shortDesc | Usually goes first. |
| BT-1693 | m:quickguard.desc | The user and its party members are protected from attacks with original or altered prio... |
| BT-1694 | m:quickguard.shortDesc | Protects allies from priority attacks this turn. |
| BT-1696 | m:quiverdance.desc | Raises the user's Special Attack, Special Defense, and Speed by 1 stage. |
| BT-1697 | m:quiverdance.shortDesc | Raises the user's Sp. Atk, Sp. Def, Speed by 1. |
| BT-1698 | m:rage.desc | Once this move is successfully used, the user's Attack is raised by 1 stage every time ... |
| BT-1699 | m:rage.shortDesc | Raises the user's Attack by 1 if hit during use. |
| BT-1700 | m:ragefist.desc | Power is equal to 50+(X × 50), where X is the total number of times the user has been h... |
| BT-1701 | m:ragefist.shortDesc | +50 power for each time user was hit. Max 6 hits. |
| BT-1702 | m:ragepowder.desc | Until the end of the turn, all single-target attacks from the opposing side are redirec... |
| BT-1703 | m:ragepowder.shortDesc | The foes' moves target the user on the turn used. |
| BT-1707 | m:ragingbull.desc | If this attack does not miss, the effects of Reflect, Light Screen, and Aurora Veil end... |
| BT-1708 | m:ragingbull.shortDesc | Destroys screens. Type depends on user's form. |
| BT-1709 | m:ragingfury.desc | The user spends two or three turns locked into this move and becomes confused immediate... |
| BT-1710 | m:ragingfury.shortDesc | Lasts 2-3 turns. Confuses the user afterwards. |
| BT-1711 | m:raindance.desc | For 5 turns, the weather becomes Rain. The damage of Water-type attacks is multiplied b... |
| BT-1712 | m:raindance.shortDesc | For 5 turns, heavy rain powers Water moves. |
| BT-1713 | m:rapidspin.desc | If this move is successful and the user has not fainted, the effects of Leech Seed and ... |
| BT-1714 | m:rapidspin.shortDesc | Free user from hazards/bind/Leech Seed; +1 Spe. |
| BT-1715 | m:razorleaf.desc | Has a higher chance for a critical hit. |
| BT-1716 | m:razorleaf.shortDesc | High critical hit ratio. Hits adjacent foes. |
| BT-1717 | m:razorshell.desc | Has a 50% chance to lower the target's Defense by 1 stage. |
| BT-1718 | m:razorshell.shortDesc | 50% chance to lower the target's Defense by 1. |
| BT-1719 | m:razorwind.desc | Has a higher chance for a critical hit. This attack charges on the first turn and execu... |
| BT-1721 | m:razorwind.shortDesc | Charges, then hits foe(s) turn 2. High crit ratio. |
| BT-1722 | m:recover.desc | The user restores 1/2 of its maximum HP, rounded half up. |
| BT-1723 | m:recover.shortDesc | Heals the user by 50% of its max HP. |
| BT-1725 | m:recycle.desc | The user regains the item it last used. Fails if the user is holding an item, if the us... |
| BT-1726 | m:recycle.shortDesc | Restores the item the user last used. |
| BT-1727 | m:reflect.desc | For 5 turns, the user and its party members take 0.5× damage from physical attacks, or ... |
| BT-1729 | m:reflect.shortDesc | For 5 turns, physical damage to allies is halved. |
| BT-1731 | m:reflecttype.desc | Causes the user's types to become the same as the current types of the target. If the t... |
| BT-1732 | m:reflecttype.shortDesc | User becomes the same type as the target. |
| BT-1734 | m:refresh.desc | The user cures its burn, poison, or paralysis. Fails if the user is not burned, poisone... |
| BT-1735 | m:refresh.shortDesc | User cures its burn, poison, or paralysis. |
| BT-1736 | m:relicsong.desc | Has a 10% chance to cause the target to fall asleep. If this move is successful on at l... |
| BT-1737 | m:relicsong.shortDesc | 10% chance to sleep foe(s). Meloetta transforms. |
| BT-1738 | m:rest.desc | The user falls asleep for the next two turns and restores all of its HP, curing itself ... |
| BT-1739 | m:rest.shortDesc | User sleeps 2 turns and restores HP and status. |
| BT-1740 | m:retaliate.desc | Power doubles if one of the user's party members fainted last turn. |
| BT-1741 | m:retaliate.shortDesc | Power doubles if an ally fainted last turn. |
| BT-1742 | m:return.desc | Power is equal to the greater of (user's Happiness × 2/5), rounded down, or 1. |
| BT-1743 | m:return.shortDesc | Max 102 power at maximum Happiness. |
| BT-1744 | m:revelationdance.desc | This move's type depends on the user's primary type. If the user's primary type is type... |
| BT-1745 | m:revelationdance.shortDesc | Type varies based on the user's primary type. |
| BT-1746 | m:revenge.desc | Power doubles if the user was hit by the target this turn. |
| BT-1747 | m:revenge.shortDesc | Power doubles if user is damaged by the target. |
| BT-1748 | m:reversal.desc | The power of this move is 20 if X is 33 to 48, 40 if X is 17 to 32, 80 if X is 10 to 16... |
| BT-1749 | m:reversal.shortDesc | More power the less HP the user has left. |
| BT-1750 | m:revivalblessing.desc | A fainted party member is selected and revived with 1/2 its max HP, rounded down. Fails... |
| BT-1752 | m:revivalblessing.shortDesc | Revives a fainted Pokemon to 50% HP. |
| BT-1753 | m:risingvoltage.desc | If the current terrain is Electric Terrain and the target is grounded, this move's powe... |
| BT-1754 | m:risingvoltage.shortDesc | 2× power if target is grounded in Electric Terrain. |
| BT-1755 | m:roar.desc | The target is forced to switch out and be replaced with a random unfainted ally. Fails ... |
| BT-1756 | m:roar.shortDesc | Forces the target to switch to a random ally. |
| BT-1757 | m:roaroftime.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1758 | m:roaroftime.shortDesc | User cannot move next turn. |
| BT-1759 | m:rockblast.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-1760 | m:rockblast.shortDesc | Hits 2-5 times in one turn. |
| BT-1761 | m:rockclimb.desc | Has a 20% chance to confuse the target. |
| BT-1762 | m:rockclimb.shortDesc | 20% chance to confuse the target. |
| BT-1763 | m:rockpolish.desc | Raises the user's Speed by 2 stages. |
| BT-1764 | m:rockpolish.shortDesc | Raises the user's Speed by 2. |
| BT-1765 | m:rockslide.desc | Has a 30% chance to make the target flinch. |
| BT-1766 | m:rockslide.shortDesc | 30% chance to make the foe(s) flinch. |
| BT-1767 | m:rocksmash.desc | Has a 50% chance to lower the target's Defense by 1 stage. |
| BT-1768 | m:rocksmash.shortDesc | 50% chance to lower the target's Defense by 1. |
| BT-1769 | m:rockthrow.shortDesc | No additional effect. |
| BT-1770 | m:rocktomb.desc | Has a 100% chance to lower the target's Speed by 1 stage. |
| BT-1771 | m:rocktomb.shortDesc | 100% chance to lower the target's Speed by 1. |
| BT-1772 | m:rockwrecker.desc | If this move is successful, the user must recharge on the following turn and cannot sel... |
| BT-1773 | m:rockwrecker.shortDesc | User cannot move next turn. |
| BT-1775 | m:roleplay.desc | The user's Ability changes to match the target's Ability. Fails if the user's Ability i... |
| BT-1776 | m:roleplay.shortDesc | User replaces its Ability with the target's. |
| BT-1777 | m:rollingkick.desc | Has a 30% chance to make the target flinch. |
| BT-1778 | m:rollingkick.shortDesc | 30% chance to make the target flinch. |
| BT-1779 | m:rollout.desc | If this move is successful, the user is locked into this move and cannot make another m... |
| BT-1780 | m:rollout.shortDesc | Power doubles with each hit. Repeats for 5 turns. |
| BT-1781 | m:roost.desc | The user restores 1/2 of its maximum HP, rounded half up. If the user is not Terastalli... |
| BT-1782 | m:roost.shortDesc | Heals 50% HP. Flying-type removed 'til turn ends. |
| BT-1784 | m:rototiller.desc | Raises the Attack and Special Attack of all grounded Grass-type Pokemon on the field by... |
| BT-1785 | m:rototiller.shortDesc | Raises Atk/Sp. Atk of grounded Grass types by 1. |
| BT-1786 | m:round.desc | If there are other active Pokemon that chose this move for use this turn, those Pokemon... |
| BT-1787 | m:round.shortDesc | Power doubles if others used Round this turn. |
| BT-1788 | m:ruination.desc | Deals damage to the target equal to half of its current HP, rounded down, but not less ... |
| BT-1789 | m:ruination.shortDesc | Does damage equal to 1/2 target's current HP. |
| BT-1790 | m:sacredfire.desc | Has a 50% chance to burn the target. |
| BT-1791 | m:sacredfire.shortDesc | 50% chance to burn the target. Thaws user. |
| BT-1792 | m:sacredsword.desc | Ignores the target's stat stage changes, including evasiveness. |
| BT-1793 | m:sacredsword.shortDesc | Ignores the target's stat stage changes. |
| BT-1795 | m:safeguard.desc | For 5 turns, the user and its party members cannot have non-volatile status conditions ... |
| BT-1797 | m:safeguard.shortDesc | For 5 turns, protects user's party from status. |
| BT-1800 | m:saltcure.desc | Causes damage to the target equal to 1/8 of its maximum HP (1/4 if the target is Steel ... |
| BT-1801 | m:saltcure.shortDesc | Deals 1/8 max HP each turn; 1/4 on Steel, Water. |
| BT-1803 | m:sandattack.desc | Lowers the target's accuracy by 1 stage. |
| BT-1804 | m:sandattack.shortDesc | Lowers the target's accuracy by 1. |
| BT-1805 | m:sandsearstorm.desc | Has a 20% chance to burn the target. If the weather is Heavy Rain or Rain, this move do... |
| BT-1806 | m:sandsearstorm.shortDesc | 20% chance to burn foe(s). Can't miss in rain. |
| BT-1807 | m:sandstorm.desc | For 5 turns, the weather becomes Sandstorm. At the end of each turn except the last, al... |
| BT-1808 | m:sandstorm.shortDesc | For 5 turns, a sandstorm rages. Rock: 1.5× SpD. |
| BT-1809 | m:sandtomb.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-1810 | m:sandtomb.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-1812 | m:sappyseed.desc | This move summons Leech Seed on the foe. |
| BT-1813 | m:sappyseed.shortDesc | Summons Leech Seed. |
| BT-1814 | m:savagespinout.shortDesc | Power is equal to the base move's Z-Power. |
| BT-1815 | m:scald.desc | Has a 30% chance to burn the target. The target thaws out if it is frozen. |
| BT-1816 | m:scald.shortDesc | 30% chance to burn the target. Thaws target. |
| BT-1817 | m:scaleshot.desc | Hits two to five times. Lowers the user's Defense by 1 stage and raises the user's Spee... |
| BT-1818 | m:scaleshot.shortDesc | Hits 2-5 times. User: -1 Def, +1 Spe after last hit. |
| BT-1819 | m:scaryface.desc | Lowers the target's Speed by 2 stages. |
| BT-1820 | m:scaryface.shortDesc | Lowers the target's Speed by 2. |
| BT-1821 | m:scorchingsands.desc | Has a 30% chance to burn the target. The target thaws out if it is frozen. |
| BT-1822 | m:scorchingsands.shortDesc | 30% chance to burn the target. Thaws target. |
| BT-1823 | m:scratch.shortDesc | No additional effect. |
| BT-1824 | m:screech.desc | Lowers the target's Defense by 2 stages. |
| BT-1825 | m:screech.shortDesc | Lowers the target's Defense by 2. |
| BT-1826 | m:searingshot.desc | Has a 30% chance to burn the target. |
| BT-1827 | m:searingshot.shortDesc | 30% chance to burn adjacent Pokemon. |
| BT-1828 | m:searingsunrazesmash.desc | This move and its effects ignore the Abilities of other Pokemon. |
| BT-1829 | m:searingsunrazesmash.shortDesc | Ignores the Abilities of other Pokemon. |
| BT-1830 | m:secretpower.desc | Has a 30% chance to cause a secondary effect on the target based on the battle terrain.... |
| BT-1831 | m:secretpower.shortDesc | Effect varies with terrain. (30% paralysis chance) |
| BT-1832 | m:secretsword.desc | Deals damage to the target based on its Defense instead of Special Defense. |
| BT-1833 | m:secretsword.shortDesc | Damages target based on Defense, not Sp. Def. |
| BT-1834 | m:seedbomb.shortDesc | No additional effect. |
| BT-1835 | m:seedflare.desc | Has a 40% chance to lower the target's Special Defense by 2 stages. |
| BT-1836 | m:seedflare.shortDesc | 40% chance to lower the target's Sp. Def by 2. |
| BT-1837 | m:seismictoss.desc | Deals damage to the target equal to the user's level. |
| BT-1838 | m:seismictoss.shortDesc | Does damage equal to the user's level. |
| BT-1839 | m:selfdestruct.desc | The user faints after using this move, even if this move fails for having no target. Th... |
| BT-1840 | m:selfdestruct.shortDesc | Hits adjacent Pokemon. The user faints. |
| BT-1841 | m:shadowball.desc | Has a 20% chance to lower the target's Special Defense by 1 stage. |
| BT-1842 | m:shadowball.shortDesc | 20% chance to lower the target's Sp. Def by 1. |
| BT-1843 | m:shadowbone.desc | Has a 20% chance to lower the target's Defense by 1 stage. |
| BT-1844 | m:shadowbone.shortDesc | 20% chance to lower the target's Defense by 1. |
| BT-1845 | m:shadowclaw.desc | Has a higher chance for a critical hit. |
| BT-1846 | m:shadowclaw.shortDesc | High critical hit ratio. |
| BT-1848 | m:shadowforce.desc | If this move is successful, it breaks through the target's Baneful Bunker, Detect, King... |
| BT-1850 | m:shadowforce.shortDesc | Disappears turn 1. Hits turn 2. Breaks protection. |
| BT-1851 | m:shadowpunch.shortDesc | This move does not check accuracy. |
| BT-1852 | m:shadowsneak.desc | No additional effect. |
| BT-1853 | m:shadowsneak.shortDesc | Usually goes first. |
| BT-1854 | m:shadowstrike.desc | Has a 50% chance to lower the target's Defense by 1 stage. |
| BT-1855 | m:shadowstrike.shortDesc | 50% chance to lower the target's Defense by 1. |
| BT-1856 | m:sharpen.desc | Raises the user's Attack by 1 stage. |
| BT-1857 | m:sharpen.shortDesc | Raises the user's Attack by 1. |
| BT-1858 | m:shatteredpsyche.shortDesc | Power is equal to the base move's Z-Power. |
| BT-1860 | m:shedtail.desc | The user takes 1/2 of its maximum HP, rounded up, and creates a substitute that has 1/4... |
| BT-1862 | m:shedtail.shortDesc | User takes 1/2 its max HP to pass a substitute. |
| BT-1864 | m:sheercold.desc | Deals damage to the target equal to the target's maximum HP. Ignores accuracy and evasi... |
| BT-1865 | m:sheercold.shortDesc | OHKOs non-Ice targets. Fails if user's lower level. |
| BT-1866 | m:shellsidearm.desc | Has a 20% chance to poison the target. This move becomes a physical attack that makes c... |
| BT-1867 | m:shellsidearm.shortDesc | 20% psn. Physical+contact if it would be stronger. |
| BT-1868 | m:shellsmash.desc | Lowers the user's Defense and Special Defense by 1 stage. Raises the user's Attack, Spe... |
| BT-1869 | m:shellsmash.shortDesc | Lowers Def, SpD by 1; raises Atk, SpA, Spe by 2. |
| BT-1871 | m:shelltrap.desc | Fails unless the user is hit by a physical attack from an opponent this turn before it ... |
| BT-1873 | m:shelltrap.shortDesc | User must take physical damage before moving. |
| BT-1875 | m:shelter.desc | Raises the user's Defense by 2 stages. |
| BT-1876 | m:shelter.shortDesc | Raises the user's Defense by 2. |
| BT-1877 | m:shiftgear.desc | Raises the user's Speed by 2 stages and its Attack by 1 stage. |
| BT-1878 | m:shiftgear.shortDesc | Raises the user's Speed by 2 and Attack by 1. |
| BT-1879 | m:shockwave.shortDesc | This move does not check accuracy. |
| BT-1880 | m:shoreup.desc | The user restores 1/2 of its maximum HP, rounded half down. If the weather is Sandstorm... |
| BT-1881 | m:shoreup.shortDesc | User restores 1/2 its max HP; 2/3 in Sandstorm. |
| BT-1882 | m:signalbeam.desc | Has a 10% chance to confuse the target. |
| BT-1883 | m:signalbeam.shortDesc | 10% chance to confuse the target. |
| BT-1884 | m:silktrap.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-1885 | m:silktrap.shortDesc | Protects from damaging attacks. Contact: -1 Spe. |
| BT-1886 | m:silverwind.desc | Has a 10% chance to raise the user's Attack, Defense, Special Attack, Special Defense, ... |
| BT-1887 | m:silverwind.shortDesc | 10% chance to raise all stats by 1 (not acc/eva). |
| BT-1888 | m:simplebeam.desc | Causes the target's Ability to become Simple. Fails if the target's Ability is As One, ... |
| BT-1889 | m:simplebeam.shortDesc | The target's Ability becomes Simple. |
| BT-1890 | m:sing.shortDesc | Causes the target to fall asleep. |
| BT-1891 | m:sinisterarrowraid.shortDesc | No additional effect. |
| BT-1892 | m:sizzlyslide.desc | Has a 100% chance to burn the foe. |
| BT-1893 | m:sizzlyslide.shortDesc | 100% chance to burn the foe. |
| BT-1895 | m:sketch.desc | This move is permanently replaced by the last move used by the target. The copied move ... |
| BT-1896 | m:sketch.shortDesc | Permanently copies the last move target used. |
| BT-1898 | m:skillswap.desc | The user swaps its Ability with the target's Ability. Fails if either the user or the t... |
| BT-1899 | m:skillswap.shortDesc | The user and the target trade Abilities. |
| BT-1900 | m:skittersmack.desc | Has a 100% chance to lower the target's Special Attack by 1 stage. |
| BT-1901 | m:skittersmack.shortDesc | 100% chance to lower target's Sp. Atk by 1. |
| BT-1902 | m:skullbash.desc | This attack charges on the first turn and executes on the second. Raises the user's Def... |
| BT-1904 | m:skullbash.shortDesc | Raises user's Defense by 1 on turn 1. Hits turn 2. |
| BT-1905 | m:skyattack.desc | Has a 30% chance to make the target flinch and a higher chance for a critical hit. This... |
| BT-1907 | m:skyattack.shortDesc | Charges, then hits turn 2. 30% flinch. High crit. |
| BT-1908 | m:skydrop.desc | This attack takes the target into the air with the user on the first turn and executes ... |
| BT-1913 | m:skydrop.shortDesc | User and foe fly up turn 1. Damages on turn 2. |
| BT-1914 | m:skyuppercut.desc | This move can hit a target using Bounce, Fly, or Sky Drop, or is under the effect of Sk... |
| BT-1915 | m:skyuppercut.shortDesc | Can hit Pokemon using Bounce, Fly, or Sky Drop. |
| BT-1916 | m:slackoff.desc | The user restores 1/2 of its maximum HP, rounded half up. |
| BT-1917 | m:slackoff.shortDesc | Heals the user by 50% of its max HP. |
| BT-1918 | m:slam.shortDesc | No additional effect. |
| BT-1919 | m:slash.desc | Has a higher chance for a critical hit. |
| BT-1920 | m:slash.shortDesc | High critical hit ratio. |
| BT-1921 | m:sleeppowder.shortDesc | Causes the target to fall asleep. |
| BT-1922 | m:sleeptalk.desc | One of the user's known moves, besides this move, is selected for use at random. Fails ... |
| BT-1923 | m:sleeptalk.shortDesc | User must be asleep. Uses another known move. |
| BT-1924 | m:sludge.desc | Has a 30% chance to poison the target. |
| BT-1925 | m:sludge.shortDesc | 30% chance to poison the target. |
| BT-1926 | m:sludgebomb.desc | Has a 30% chance to poison the target. |
| BT-1927 | m:sludgebomb.shortDesc | 30% chance to poison the target. |
| BT-1928 | m:sludgewave.desc | Has a 10% chance to poison the target. |
| BT-1929 | m:sludgewave.shortDesc | 10% chance to poison adjacent Pokemon. |
| BT-1930 | m:smackdown.desc | This move can hit a target using Bounce, Fly, or Sky Drop, or is under the effect of Sk... |
| BT-1931 | m:smackdown.shortDesc | Removes the target's Ground immunity. |
| BT-1933 | m:smartstrike.shortDesc | This move does not check accuracy. |
| BT-1934 | m:smellingsalts.desc | Power doubles if the target is paralyzed. If the user has not fainted, the target is cu... |
| BT-1935 | m:smellingsalts.shortDesc | Power doubles if target is paralyzed, and cures it. |
| BT-1936 | m:smog.desc | Has a 40% chance to poison the target. |
| BT-1937 | m:smog.shortDesc | 40% chance to poison the target. |
| BT-1938 | m:smokescreen.desc | Lowers the target's accuracy by 1 stage. |
| BT-1939 | m:smokescreen.shortDesc | Lowers the target's accuracy by 1. |
| BT-1940 | m:snaptrap.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-1941 | m:snaptrap.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-1943 | m:snarl.desc | Has a 100% chance to lower the target's Special Attack by 1 stage. |
| BT-1944 | m:snarl.shortDesc | 100% chance to lower the foe(s) Sp. Atk by 1. |
| BT-1946 | m:snatch.desc | If another Pokemon uses certain non-damaging moves this turn, the user steals that move... |
| BT-1947 | m:snatch.shortDesc | User steals certain support moves to use itself. |
| BT-1949 | m:snipeshot.desc | Has a higher chance for a critical hit. This move cannot be redirected to a different t... |
| BT-1950 | m:snipeshot.shortDesc | High critical hit ratio. Cannot be redirected. |
| BT-1951 | m:snore.desc | Has a 30% chance to make the target flinch. Fails if the user is not asleep. |
| BT-1952 | m:snore.shortDesc | User must be asleep. 30% chance to flinch target. |
| BT-1953 | m:snowscape.desc | For 5 turns, the weather becomes Snow. During the effect, the Defense of Ice-type Pokem... |
| BT-1954 | m:snowscape.shortDesc | For 5 turns, snow falls. Ice: 1.5× Def. |
| BT-1955 | m:soak.desc | Causes the target to become a Water type. Fails if the target is an Arceus or a Silvall... |
| BT-1956 | m:soak.shortDesc | Changes the target's type to Water. |
| BT-1957 | m:softboiled.desc | The user restores 1/2 of its maximum HP, rounded half up. |
| BT-1958 | m:softboiled.shortDesc | Heals the user by 50% of its max HP. |
| BT-1959 | m:solarbeam.desc | This attack charges on the first turn and executes on the second. Power is halved if th... |
| BT-1961 | m:solarbeam.shortDesc | Charges turn 1. Hits turn 2. No charge in sunlight. |
| BT-1962 | m:solarblade.desc | This attack charges on the first turn and executes on the second. Power is halved if th... |
| BT-1964 | m:solarblade.shortDesc | Charges turn 1. Hits turn 2. No charge in sunlight. |
| BT-1965 | m:sonicboom.desc | Deals 20 HP of damage to the target. |
| BT-1966 | m:sonicboom.shortDesc | Always does 20 HP of damage. |
| BT-1967 | m:soulstealing7starstrike.shortDesc | No additional effect. |
| BT-1968 | m:spacialrend.desc | Has a higher chance for a critical hit. |
| BT-1969 | m:spacialrend.shortDesc | High critical hit ratio. |
| BT-1970 | m:spark.desc | Has a 30% chance to paralyze the target. |
| BT-1971 | m:spark.shortDesc | 30% chance to paralyze the target. |
| BT-1972 | m:sparklingaria.desc | If the user has not fainted, the target is cured of its burn. |
| BT-1973 | m:sparklingaria.shortDesc | The target is cured of its burn. |
| BT-1974 | m:sparklyswirl.desc | Every Pokemon in the user's party is cured of its non-volatile status condition. |
| BT-1975 | m:sparklyswirl.shortDesc | Cures the user's party of all status conditions. |
| BT-1977 | m:spectralthief.desc | The target's stat stages greater than 0 are stolen from it and applied to the user befo... |
| BT-1978 | m:spectralthief.shortDesc | Steals target's boosts before dealing damage. |
| BT-1980 | m:speedswap.desc | The user swaps its Speed stat with the target. Stat stage changes are unaffected. |
| BT-1981 | m:speedswap.shortDesc | Swaps Speed stat with target. |
| BT-1982 | m:spicyextract.desc | Raises the target's Attack by 2 stages and lowers its Defense by 2 stages. |
| BT-1983 | m:spicyextract.shortDesc | Raises target's Atk by 2 and lowers its Def by 2. |
| BT-1984 | m:spiderweb.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-1985 | m:spiderweb.shortDesc | Prevents the target from switching out. |
| BT-1986 | m:spikecannon.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-1987 | m:spikecannon.shortDesc | Hits 2-5 times in one turn. |
| BT-1989 | m:spikes.desc | Sets up a hazard on the opposing side of the field, damaging each opposing Pokemon that... |
| BT-1991 | m:spikes.shortDesc | Hurts grounded foes on switch-in. Max 3 layers. |
| BT-1994 | m:spikyshield.desc | The user is protected from most attacks made by other Pokemon during this turn, and Pok... |
| BT-1995 | m:spikyshield.shortDesc | Protects from moves. Contact: loses 1/8 max HP. |
| BT-1996 | m:spinout.desc | Lowers the user's Speed by 2 stages. |
| BT-1997 | m:spinout.shortDesc | Lowers the user's Speed by 2. |
| BT-1998 | m:spiritbreak.desc | Has a 100% chance to lower the target's Special Attack by 1 stage. |
| BT-1999 | m:spiritbreak.shortDesc | 100% chance to lower the target's Sp. Atk by 1. |
| BT-2000 | m:spiritshackle.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-2001 | m:spiritshackle.shortDesc | Prevents the target from switching out. |
| BT-2003 | m:spite.desc | Causes the target's last move used to lose 4 PP. Fails if the target has not made a mov... |
| BT-2004 | m:spite.shortDesc | Lowers the PP of the target's last move by 4. |
| BT-2005 | m:spitup.desc | Power is equal to 100 times the user's Stockpile count. Fails if the user's Stockpile c... |
| BT-2006 | m:spitup.shortDesc | More power with more uses of Stockpile. |
| BT-2008 | m:splash.shortDesc | No competitive use. |
| BT-2009 | m:splinteredstormshards.desc | Ends the effects of Electric Terrain, Grassy Terrain, Misty Terrain, and Psychic Terrain. |
| BT-2010 | m:splinteredstormshards.shortDesc | Ends the effects of terrain. |
| BT-2011 | m:splishysplash.desc | Has a 30% chance to paralyze the target. |
| BT-2012 | m:splishysplash.shortDesc | 30% chance to paralyze the target. |
| BT-2013 | m:spore.shortDesc | Causes the target to fall asleep. |
| BT-2014 | m:spotlight.desc | Until the end of the turn, all single-target attacks from opponents of the target are r... |
| BT-2015 | m:spotlight.shortDesc | Target's foes' moves are redirected to it this turn. |
| BT-2018 | m:springtidestorm.desc | Has a 30% chance to lower the target's Attack by 1 stage. |
| BT-2019 | m:springtidestorm.shortDesc | 30% chance to lower the foe(s) Attack by 1. |
| BT-2021 | m:stealthrock.desc | Sets up a hazard on the opposing side of the field, damaging each opposing Pokemon that... |
| BT-2023 | m:stealthrock.shortDesc | Hurts foes on switch-in. Factors Rock weakness. |
| BT-2025 | m:steameruption.desc | Has a 30% chance to burn the target. The target thaws out if it is frozen. |
| BT-2026 | m:steameruption.shortDesc | 30% chance to burn the target. Thaws target. |
| BT-2027 | m:steamroller.desc | Has a 30% chance to make the target flinch. Damage doubles and no accuracy check is don... |
| BT-2028 | m:steamroller.shortDesc | 30% chance to make the target flinch. |
| BT-2030 | m:steelbeam.desc | Whether or not this move is successful and even if it would cause fainting, the user lo... |
| BT-2031 | m:steelbeam.shortDesc | User loses 50% max HP. |
| BT-2032 | m:steelroller.desc | Fails if there is no terrain active. Ends the effects of Electric Terrain, Grassy Terra... |
| BT-2033 | m:steelroller.shortDesc | Fails if there is no terrain active. Ends the terrain. |
| BT-2034 | m:steelwing.desc | Has a 10% chance to raise the user's Defense by 1 stage. |
| BT-2035 | m:steelwing.shortDesc | 10% chance to raise the user's Defense by 1. |
| BT-2037 | m:stickyweb.desc | Sets up a hazard on the opposing side of the field, lowering the Speed by 1 stage of ea... |
| BT-2039 | m:stickyweb.shortDesc | Lowers Speed of grounded foes by 1 on switch-in. |
| BT-2041 | m:stockpile.desc | Raises the user's Defense and Special Defense by 1 stage. The user's Stockpile count in... |
| BT-2043 | m:stockpile.shortDesc | Raises user's Defense, Sp. Def by 1. Max 3 uses. |
| BT-2045 | m:stokedsparksurfer.desc | Has a 100% chance to paralyze the target. |
| BT-2046 | m:stokedsparksurfer.shortDesc | 100% chance to paralyze the target. |
| BT-2047 | m:stomp.desc | Has a 30% chance to make the target flinch. Damage doubles and no accuracy check is don... |
| BT-2048 | m:stomp.shortDesc | 30% chance to make the target flinch. |
| BT-2049 | m:stompingtantrum.desc | Power doubles if the user's last move on the previous turn, including moves called by o... |
| BT-2050 | m:stompingtantrum.shortDesc | Power doubles if the user's last move failed. |
| BT-2051 | m:stoneaxe.desc | If this move is successful, it sets up a hazard on the opposing side of the field, dama... |
| BT-2052 | m:stoneaxe.shortDesc | Sets Stealth Rock on the target's side. |
| BT-2053 | m:stoneedge.desc | Has a higher chance for a critical hit. |
| BT-2054 | m:stoneedge.shortDesc | High critical hit ratio. |
| BT-2055 | m:storedpower.desc | Power is equal to 20+(X × 20), where X is the user's total stat stage changes that are ... |
| BT-2056 | m:storedpower.shortDesc | + 20 power for each of the user's stat boosts. |
| BT-2057 | m:stormthrow.desc | This move is always a critical hit unless the target is under the effect of Lucky Chant... |
| BT-2058 | m:stormthrow.shortDesc | Always results in a critical hit. |
| BT-2059 | m:strangesteam.desc | Has a 20% chance to confuse the target. |
| BT-2060 | m:strangesteam.shortDesc | 20% chance to confuse the target. |
| BT-2061 | m:strength.shortDesc | No additional effect. |
| BT-2062 | m:strengthsap.desc | Lowers the target's Attack by 1 stage. The user restores its HP equal to the target's A... |
| BT-2063 | m:strengthsap.shortDesc | User heals HP=target's Atk stat. Lowers Atk by 1. |
| BT-2064 | m:stringshot.desc | Lowers the target's Speed by 2 stages. |
| BT-2065 | m:stringshot.shortDesc | Lowers the foe(s) Speed by 2. |
| BT-2066 | m:struggle.desc | Deals typeless damage to a random opposing Pokemon. If this move was successful, the us... |
| BT-2067 | m:struggle.shortDesc | User loses 1/4 of its max HP. |
| BT-2068 | m:strugglebug.desc | Has a 100% chance to lower the target's Special Attack by 1 stage. |
| BT-2069 | m:strugglebug.shortDesc | 100% chance to lower the foe(s) Sp. Atk by 1. |
| BT-2070 | m:stuffcheeks.desc | This move cannot be selected unless the user is holding a Berry. The user eats its Berr... |
| BT-2071 | m:stuffcheeks.shortDesc | Must hold Berry to use. User eats Berry, Def +2. |
| BT-2072 | m:stunspore.desc | Paralyzes the target. |
| BT-2073 | m:stunspore.shortDesc | Paralyzes the target. |
| BT-2074 | m:submission.desc | If the target lost HP, the user takes recoil damage equal to 1/4 the HP lost by the tar... |
| BT-2075 | m:submission.shortDesc | Has 1/4 recoil. |
| BT-2078 | m:substitute.desc | The user takes 1/4 of its maximum HP, rounded down, and puts it into a substitute to ta... |
| BT-2081 | m:substitute.shortDesc | User takes 1/4 its max HP to put in a substitute. |
| BT-2083 | m:subzeroslammer.shortDesc | Power is equal to the base move's Z-Power. |
| BT-2084 | m:suckerpunch.desc | Fails if the target did not select a physical attack, special attack, or Me First for u... |
| BT-2085 | m:suckerpunch.shortDesc | Usually goes first. Fails if target is not attacking. |
| BT-2086 | m:sunnyday.desc | For 5 turns, the weather becomes Sun. The damage of Fire-type attacks is multiplied by ... |
| BT-2087 | m:sunnyday.shortDesc | For 5 turns, intense sunlight powers Fire moves. |
| BT-2088 | m:sunsteelstrike.desc | This move and its effects ignore the Abilities of other Pokemon. |
| BT-2089 | m:sunsteelstrike.shortDesc | Ignores the Abilities of other Pokemon. |
| BT-2091 | m:supercellslam.desc | If this attack is not successful, the user loses half of its maximum HP, rounded down, ... |
| BT-2092 | m:supercellslam.shortDesc | User is hurt by 50% of its max HP if it misses. |
| BT-2093 | m:superfang.desc | Deals damage to the target equal to half of its current HP, rounded down, but not less ... |
| BT-2094 | m:superfang.shortDesc | Does damage equal to 1/2 target's current HP. |
| BT-2095 | m:superpower.desc | Lowers the user's Attack and Defense by 1 stage. |
| BT-2096 | m:superpower.shortDesc | Lowers the user's Attack and Defense by 1. |
| BT-2097 | m:supersonic.shortDesc | Causes the target to become confused. |
| BT-2098 | m:supersonicskystrike.shortDesc | Power is equal to the base move's Z-Power. |
| BT-2099 | m:surf.desc | Damage doubles if the target is using Dive. |
| BT-2100 | m:surf.shortDesc | Hits adjacent Pokemon. Double damage on Dive. |
| BT-2101 | m:surgingstrikes.desc | Hits three times. This move is always a critical hit unless the target is under the eff... |
| BT-2102 | m:surgingstrikes.shortDesc | Always results in a critical hit. Hits 3 times. |
| BT-2103 | m:swagger.desc | Raises the target's Attack by 2 stages and confuses it. |
| BT-2104 | m:swagger.shortDesc | Raises the target's Attack by 2 and confuses it. |
| BT-2105 | m:swallow.desc | The user restores its HP based on its Stockpile count. Restores 1/4 of its maximum HP i... |
| BT-2106 | m:swallow.shortDesc | Heals the user based on uses of Stockpile. |
| BT-2107 | m:sweetkiss.shortDesc | Causes the target to become confused. |
| BT-2108 | m:sweetscent.desc | Lowers the target's evasiveness by 2 stages. |
| BT-2109 | m:sweetscent.shortDesc | Lowers the foe(s) evasiveness by 2. |
| BT-2110 | m:swift.desc | This move does not check accuracy. |
| BT-2111 | m:swift.shortDesc | This move does not check accuracy. Hits foes. |
| BT-2113 | m:switcheroo.desc | The user swaps its held item with the target's held item. Fails if both the user and th... |
| BT-2114 | m:switcheroo.shortDesc | User switches its held item with the target's. |
| BT-2115 | m:swordsdance.desc | Raises the user's Attack by 2 stages. |
| BT-2116 | m:swordsdance.shortDesc | Raises the user's Attack by 2. |
| BT-2117 | m:synchronoise.desc | The target is immune if it does not share a type with the user. |
| BT-2118 | m:synchronoise.shortDesc | Hits adjacent Pokemon sharing the user's type. |
| BT-2119 | m:synthesis.desc | The user restores 1/2 of its maximum HP if Strong Winds or no weather conditions are in... |
| BT-2120 | m:synthesis.shortDesc | Heals the user by a weather-dependent amount. |
| BT-2121 | m:syrupbomb.desc | If this move is successful, it causes the target's Speed to be lowered by 1 stage at th... |
| BT-2122 | m:syrupbomb.shortDesc | Target's Speed is lowered by 1 stage for 3 turns. |
| BT-2124 | m:tachyoncutter.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-2125 | m:tachyoncutter.shortDesc | Hits twice. This move does not check accuracy. |
| BT-2126 | m:tackle.shortDesc | No additional effect. |
| BT-2127 | m:tailglow.desc | Raises the user's Special Attack by 3 stages. |
| BT-2128 | m:tailglow.shortDesc | Raises the user's Sp. Atk by 3. |
| BT-2129 | m:tailslap.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-2130 | m:tailslap.shortDesc | Hits 2-5 times in one turn. |
| BT-2131 | m:tailwhip.desc | Lowers the target's Defense by 1 stage. |
| BT-2132 | m:tailwhip.shortDesc | Lowers the foe(s) Defense by 1. |
| BT-2133 | m:tailwind.desc | For 4 turns, the user and its party members have their Speed doubled. Fails if this mov... |
| BT-2135 | m:tailwind.shortDesc | For 4 turns, allies' Speed is doubled. |
| BT-2137 | m:takedown.desc | If the target lost HP, the user takes recoil damage equal to 1/4 the HP lost by the tar... |
| BT-2138 | m:takedown.shortDesc | Has 1/4 recoil. |
| BT-2139 | m:takeheart.desc | The user cures its non-volatile status condition. Raises the user's Special Attack and ... |
| BT-2140 | m:takeheart.shortDesc | Cures user's status, raises Sp. Atk, Sp. Def by 1. |
| BT-2141 | m:tarshot.desc | Lowers the target's Speed by 1 stage. Until the target switches out, the effectiveness ... |
| BT-2142 | m:tarshot.shortDesc | Target gets -1 Spe and becomes weaker to Fire. |
| BT-2145 | m:taunt.desc | Prevents the target from using non-damaging moves for its next three turns. Pokemon wit... |
| BT-2147 | m:taunt.shortDesc | Target can't use status moves its next 3 turns. |
| BT-2149 | m:tearfullook.desc | Lowers the target's Attack and Special Attack by 1 stage. |
| BT-2150 | m:tearfullook.shortDesc | Lowers the target's Attack and Sp. Atk by 1. |
| BT-2152 | m:teatime.desc | All active Pokemon consume their held Berries. This effect is not prevented by substitu... |
| BT-2154 | m:teatime.shortDesc | All active Pokemon consume held Berries. |
| BT-2155 | m:technoblast.desc | This move's type depends on the user's held Drive. |
| BT-2156 | m:technoblast.shortDesc | Type varies based on the held Drive. |
| BT-2157 | m:tectonicrage.shortDesc | Power is equal to the base move's Z-Power. |
| BT-2158 | m:teeterdance.desc | Causes the target to become confused. |
| BT-2159 | m:teeterdance.shortDesc | Confuses adjacent Pokemon. |
| BT-2160 | m:telekinesis.desc | For 3 turns, the target cannot avoid any attacks made against it, other than OHKO moves... |
| BT-2162 | m:telekinesis.shortDesc | For 3 turns, target floats but moves can't miss it. |
| BT-2164 | m:teleport.desc | If this move is successful and the user has not fainted, the user switches out even if ... |
| BT-2165 | m:teleport.shortDesc | User switches out. |
| BT-2166 | m:temperflare.desc | Power doubles if the user's last move on the previous turn, including moves called by o... |
| BT-2167 | m:temperflare.shortDesc | Power doubles if the user's last move failed. |
| BT-2168 | m:terablast.desc | If the user is Terastallized, this move becomes a physical attack if the user's Attack ... |
| BT-2169 | m:terablast.shortDesc | If Terastallized: Phys. if Atk > SpA, type = Tera. |
| BT-2170 | m:terastarstorm.desc | If the user is a Terapagos in Stellar Form, this move's type becomes Stellar, hits all ... |
| BT-2171 | m:terastarstorm.shortDesc | Terapagos-Stellar: Stellar type, hits both foes. |
| BT-2172 | m:terrainpulse.desc | Power doubles if the user is grounded and a terrain is active, and this move's type cha... |
| BT-2173 | m:terrainpulse.shortDesc | User on terrain: power doubles, type varies. |
| BT-2174 | m:thief.desc | If this attack was successful and the user has not fainted, it steals the target's held... |
| BT-2175 | m:thief.shortDesc | If the user has no item, it steals the target's. |
| BT-2176 | m:thousandarrows.desc | This move can hit airborne Pokemon, which includes Flying-type Pokemon, Pokemon with th... |
| BT-2177 | m:thousandarrows.shortDesc | Grounds adjacent foes. First hit neutral on Flying. |
| BT-2178 | m:thousandwaves.desc | Prevents the target from switching out. The target can still switch out if it is holdin... |
| BT-2179 | m:thousandwaves.shortDesc | Hits adjacent foes. Prevents them from switching. |
| BT-2180 | m:thrash.desc | The user spends two or three turns locked into this move and becomes confused immediate... |
| BT-2181 | m:thrash.shortDesc | Lasts 2-3 turns. Confuses the user afterwards. |
| BT-2183 | m:throatchop.desc | For 2 turns, the target cannot use sound-based moves. |
| BT-2184 | m:throatchop.shortDesc | For 2 turns, the target cannot use sound moves. |
| BT-2185 | m:thunder.desc | Has a 30% chance to paralyze the target. This move can hit a target using Bounce, Fly, ... |
| BT-2186 | m:thunder.shortDesc | 30% chance to paralyze. Can't miss in rain. |
| BT-2187 | m:thunderbolt.desc | Has a 10% chance to paralyze the target. |
| BT-2188 | m:thunderbolt.shortDesc | 10% chance to paralyze the target. |
| BT-2189 | m:thundercage.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-2190 | m:thundercage.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-2192 | m:thunderclap.desc | Fails if the target did not select a physical attack, special attack, or Me First for u... |
| BT-2193 | m:thunderclap.shortDesc | Usually goes first. Fails if target is not attacking. |
| BT-2194 | m:thunderfang.desc | Has a 10% chance to paralyze the target and a 10% chance to make it flinch. |
| BT-2195 | m:thunderfang.shortDesc | 10% chance to paralyze. 10% chance to flinch. |
| BT-2196 | m:thunderouskick.desc | Has a 100% chance to lower the target's Defense by 1 stage. |
| BT-2197 | m:thunderouskick.shortDesc | 100% chance to lower the target's Defense by 1. |
| BT-2198 | m:thunderpunch.desc | Has a 10% chance to paralyze the target. |
| BT-2199 | m:thunderpunch.shortDesc | 10% chance to paralyze the target. |
| BT-2200 | m:thundershock.desc | Has a 10% chance to paralyze the target. |
| BT-2201 | m:thundershock.shortDesc | 10% chance to paralyze the target. |
| BT-2202 | m:thunderwave.desc | Paralyzes the target. This move does not ignore type immunity. |
| BT-2203 | m:thunderwave.shortDesc | Paralyzes the target. |
| BT-2204 | m:tickle.desc | Lowers the target's Attack and Defense by 1 stage. |
| BT-2205 | m:tickle.shortDesc | Lowers the target's Attack and Defense by 1. |
| BT-2207 | m:tidyup.desc | Raises the user's Attack and Speed by 1 stage. Removes substitutes from all active Poke... |
| BT-2208 | m:tidyup.shortDesc | User +1 Atk, Spe. Clears all substitutes/hazards. |
| BT-2209 | m:topsyturvy.desc | The target's positive stat stages become negative and vice versa. Fails if all of the t... |
| BT-2210 | m:topsyturvy.shortDesc | Inverts the target's stat stages. |
| BT-2211 | m:torchsong.desc | Has a 100% chance to raise the user's Special Attack by 1 stage. |
| BT-2212 | m:torchsong.shortDesc | 100% chance to raise the user's Sp. Atk by 1. |
| BT-2213 | m:torment.desc | Prevents the target from selecting the same move for use two turns in a row. This effec... |
| BT-2215 | m:torment.shortDesc | Target can't select the same move twice in a row. |
| BT-2217 | m:toxic.desc | Badly poisons the target. If a Poison-type Pokemon uses this move, the target cannot av... |
| BT-2218 | m:toxic.shortDesc | Badly poisons the target. Poison types can't miss. |
| BT-2219 | m:toxicspikes.desc | Sets up a hazard on the opposing side of the field, poisoning each opposing Pokemon tha... |
| BT-2221 | m:toxicspikes.shortDesc | Poisons grounded foes on switch-in. Max 2 layers. |
| BT-2223 | m:toxicthread.desc | Lowers the target's Speed by 1 stage and poisons it. |
| BT-2224 | m:toxicthread.shortDesc | Lowers the target's Speed by 1 and poisons it. |
| BT-2225 | m:trailblaze.desc | Has a 100% chance to raise the user's Speed by 1 stage. |
| BT-2226 | m:trailblaze.shortDesc | 100% chance to raise the user's Speed by 1. |
| BT-2227 | m:transform.desc | The user transforms into the target. The target's current stats, stat stages, types, mo... |
| BT-2228 | m:transform.shortDesc | Copies target's stats, moves, types, and Ability. |
| BT-2230 | m:triattack.desc | Has a 20% chance to either burn, freeze, or paralyze the target. |
| BT-2231 | m:triattack.shortDesc | 20% chance to paralyze or burn or freeze target. |
| BT-2233 | m:trick.desc | The user swaps its held item with the target's held item. Fails if both the user and th... |
| BT-2234 | m:trick.shortDesc | User switches its held item with the target's. |
| BT-2235 | m:trickortreat.desc | Causes the Ghost type to be added to the target, effectively making it have two or thre... |
| BT-2236 | m:trickortreat.shortDesc | Adds Ghost to the target's type(s). |
| BT-2237 | m:trickroom.desc | For 5 turns, the Speed of every Pokemon is recalculated for the purposes of determining... |
| BT-2238 | m:trickroom.shortDesc | Goes last. For 5 turns, turn order is reversed. |
| BT-2239 | m:triplearrows.desc | Has a 50% chance to lower the target's Defense by 1 stage, a 30% chance to make it flin... |
| BT-2240 | m:triplearrows.shortDesc | High crit. Target: 50% -1 Defense, 30% flinch. |
| BT-2241 | m:tripleaxel.desc | Hits three times. Power increases to 40 for the second hit and 60 for the third. This m... |
| BT-2242 | m:tripleaxel.shortDesc | Hits 3 times. Each hit can miss, but power rises. |
| BT-2243 | m:tripledive.desc | Hits three times. |
| BT-2244 | m:tripledive.shortDesc | Hits 3 times. |
| BT-2245 | m:triplekick.desc | Hits three times. Power increases to 20 for the second hit and 30 for the third. This m... |
| BT-2246 | m:triplekick.shortDesc | Hits 3 times. Each hit can miss, but power rises. |
| BT-2247 | m:tropkick.desc | Has a 100% chance to lower the target's Attack by 1 stage. |
| BT-2248 | m:tropkick.shortDesc | 100% chance to lower the target's Attack by 1. |
| BT-2249 | m:trumpcard.desc | The power of this move is based on the amount of PP remaining after normal PP reduction... |
| BT-2250 | m:trumpcard.shortDesc | More power the fewer PP this move has left. |
| BT-2251 | m:twinbeam.desc | Hits twice. If the first hit breaks the target's substitute, it will take damage for th... |
| BT-2252 | m:twinbeam.shortDesc | Hits 2 times in one turn. |
| BT-2253 | m:twineedle.desc | Hits twice, with each hit having a 20% chance to poison the target. If the first hit br... |
| BT-2254 | m:twineedle.shortDesc | Hits 2 times. Each hit has 20% chance to poison. |
| BT-2255 | m:twinkletackle.shortDesc | Power is equal to the base move's Z-Power. |
| BT-2256 | m:twister.desc | Has a 20% chance to make the target flinch. Power doubles if the target is using Bounce... |
| BT-2257 | m:twister.shortDesc | 20% chance to make the foe(s) flinch. |
| BT-2258 | m:upperhand.desc | Has a 100% chance to make the target flinch. Fails if the target did not select a physi... |
| BT-2259 | m:upperhand.shortDesc | 100% flinch. Fails unless target using priority attack. |
| BT-2262 | m:uproar.desc | The user spends three turns locked into this move. This move targets an opponent at ran... |
| BT-2264 | m:uproar.shortDesc | Lasts 3 turns. Active Pokemon cannot fall asleep. |
| BT-2267 | m:uturn.desc | If this move is successful and the user has not fainted, the user switches out even if ... |
| BT-2268 | m:uturn.shortDesc | User switches out after damaging the target. |
| BT-2270 | m:vacuumwave.desc | No additional effect. |
| BT-2271 | m:vacuumwave.shortDesc | Usually goes first. |
| BT-2272 | m:vcreate.desc | Lowers the user's Speed, Defense, and Special Defense by 1 stage. |
| BT-2273 | m:vcreate.shortDesc | Lowers the user's Defense, Sp. Def, Speed by 1. |
| BT-2274 | m:veeveevolley.desc | Power is equal to the greater of (user's Happiness × 2/5), rounded down, or 1. |
| BT-2275 | m:veeveevolley.shortDesc | Max happiness: 102 power. Can't miss. |
| BT-2276 | m:venomdrench.desc | Lowers the target's Attack, Special Attack, and Speed by 1 stage if the target is poiso... |
| BT-2277 | m:venomdrench.shortDesc | Lowers Atk/Sp. Atk/Speed of poisoned foes by 1. |
| BT-2278 | m:venoshock.desc | Power doubles if the target is poisoned. |
| BT-2279 | m:venoshock.shortDesc | Power doubles if the target is poisoned. |
| BT-2280 | m:victorydance.desc | Raises the user's Attack, Defense, and Speed by 1 stage. |
| BT-2281 | m:victorydance.shortDesc | Raises the user's Attack, Defense, Speed by 1. |
| BT-2282 | m:vinewhip.shortDesc | No additional effect. |
| BT-2283 | m:visegrip.shortDesc | No additional effect. |
| BT-2284 | m:vitalthrow.desc | This move does not check accuracy. |
| BT-2285 | m:vitalthrow.shortDesc | This move does not check accuracy. Goes last. |
| BT-2286 | m:voltswitch.desc | If this move is successful and the user has not fainted, the user switches out even if ... |
| BT-2287 | m:voltswitch.shortDesc | User switches out after damaging the target. |
| BT-2289 | m:volttackle.desc | Has a 10% chance to paralyze the target. If the target lost HP, the user takes recoil d... |
| BT-2290 | m:volttackle.shortDesc | Has 33% recoil. 10% chance to paralyze target. |
| BT-2291 | m:wakeupslap.desc | Power doubles if the target is asleep. If the user has not fainted, the target wakes up. |
| BT-2292 | m:wakeupslap.shortDesc | Power doubles if target is asleep, and wakes it. |
| BT-2293 | m:waterfall.desc | Has a 20% chance to make the target flinch. |
| BT-2294 | m:waterfall.shortDesc | 20% chance to make the target flinch. |
| BT-2295 | m:watergun.shortDesc | No additional effect. |
| BT-2297 | m:waterpledge.desc | If one of the user's allies chose to use Fire Pledge or Grass Pledge this turn and has ... |
| BT-2299 | m:waterpledge.shortDesc | Use with Grass or Fire Pledge for added effect. |
| BT-2301 | m:waterpulse.desc | Has a 20% chance to confuse the target. |
| BT-2302 | m:waterpulse.shortDesc | 20% chance to confuse the target. |
| BT-2303 | m:watershuriken.desc | Hits two to five times. Has a 35% chance to hit two or three times and a 15% chance to ... |
| BT-2304 | m:watershuriken.shortDesc | Usually goes first. Hits 2-5 times in one turn. |
| BT-2305 | m:watersport.desc | For 5 turns, all Fire-type attacks used by any active Pokemon have their power multipli... |
| BT-2306 | m:watersport.shortDesc | For 5 turns, Fire-type attacks have 1/3 power. |
| BT-2307 | m:waterspout.desc | Power is equal to (user's current HP × 150 / user's maximum HP), rounded down, but not ... |
| BT-2308 | m:waterspout.shortDesc | Less power as user's HP decreases. Hits foe(s). |
| BT-2309 | m:wavecrash.desc | If the target lost HP, the user takes recoil damage equal to 33% the HP lost by the tar... |
| BT-2310 | m:wavecrash.shortDesc | Has 33% recoil. |
| BT-2311 | m:weatherball.desc | Power doubles if a weather condition other than Strong Winds is active, and this move's... |
| BT-2313 | m:weatherball.shortDesc | Power doubles and type varies in each weather. |
| BT-2314 | m:whirlpool.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-2315 | m:whirlpool.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-2317 | m:whirlwind.desc | The target is forced to switch out and be replaced with a random unfainted ally. Fails ... |
| BT-2318 | m:whirlwind.shortDesc | Forces the target to switch to a random ally. |
| BT-2319 | m:wickedblow.desc | This move is always a critical hit unless the target is under the effect of Lucky Chant... |
| BT-2320 | m:wickedblow.shortDesc | Always results in a critical hit. |
| BT-2321 | m:wickedtorque.desc | Has a 10% chance to cause the target to fall asleep. |
| BT-2322 | m:wickedtorque.shortDesc | 10% chance to cause the target to fall asleep. |
| BT-2324 | m:wideguard.desc | The user and its party members are protected from moves made by other Pokemon, includin... |
| BT-2325 | m:wideguard.shortDesc | Protects allies from multi-target moves this turn. |
| BT-2327 | m:wildboltstorm.desc | Has a 20% chance to paralyze the target. If the weather is Heavy Rain or Rain, this mov... |
| BT-2328 | m:wildboltstorm.shortDesc | 20% chance to paralyze foe(s). Rain: can't miss. |
| BT-2329 | m:wildcharge.desc | If the target lost HP, the user takes recoil damage equal to 1/4 the HP lost by the tar... |
| BT-2330 | m:wildcharge.shortDesc | Has 1/4 recoil. |
| BT-2331 | m:willowisp.desc | Burns the target. |
| BT-2332 | m:willowisp.shortDesc | Burns the target. |
| BT-2333 | m:wingattack.shortDesc | No additional effect. |
| BT-2334 | m:wish.desc | At the end of the next turn, the Pokemon at the user's position has 1/2 of the user's m... |
| BT-2336 | m:wish.shortDesc | Next turn, 50% of the user's max HP is restored. |
| BT-2337 | m:withdraw.desc | Raises the user's Defense by 1 stage. |
| BT-2338 | m:withdraw.shortDesc | Raises the user's Defense by 1. |
| BT-2339 | m:wonderroom.desc | For 5 turns, all active Pokemon have their Defense and Special Defense stats swapped. S... |
| BT-2340 | m:wonderroom.shortDesc | For 5 turns, all Defense and Sp. Def stats switch. |
| BT-2341 | m:woodhammer.desc | If the target lost HP, the user takes recoil damage equal to 33% the HP lost by the tar... |
| BT-2342 | m:woodhammer.shortDesc | Has 33% recoil. |
| BT-2343 | m:workup.desc | Raises the user's Attack and Special Attack by 1 stage. |
| BT-2344 | m:workup.shortDesc | Raises the user's Attack and Sp. Atk by 1. |
| BT-2345 | m:worryseed.desc | Causes the target's Ability to become Insomnia. Fails if the target's Ability is As One... |
| BT-2346 | m:worryseed.shortDesc | The target's Ability becomes Insomnia. |
| BT-2347 | m:wrap.desc | Prevents the target from switching for four or five turns (seven turns if the user is h... |
| BT-2349 | m:wrap.shortDesc | Traps and damages the target for 4-5 turns. |
| BT-2351 | m:wringout.desc | Power is equal to 120 × (target's current HP / target's maximum HP), rounded half down,... |
| BT-2352 | m:wringout.shortDesc | More power the more HP the target has left. |
| BT-2353 | m:xscissor.shortDesc | No additional effect. |
| BT-2354 | m:yawn.desc | Causes the target to fall asleep at the end of the next turn. Fails when used if the ta... |
| BT-2355 | m:yawn.shortDesc | Puts the target to sleep after 1 turn. |
| BT-2357 | m:zapcannon.desc | Has a 100% chance to paralyze the target. |
| BT-2358 | m:zapcannon.shortDesc | 100% chance to paralyze the target. |
| BT-2359 | m:zenheadbutt.desc | Has a 20% chance to make the target flinch. |
| BT-2360 | m:zenheadbutt.shortDesc | 20% chance to make the target flinch. |
| BT-2361 | m:zingzap.desc | Has a 30% chance to make the target flinch. |
| BT-2362 | m:zingzap.shortDesc | 30% chance to make the target flinch. |
| BT-2363 | m:zippyzap.desc | Has a 100% chance to raise the user's evasion by 1 stage. |
| BT-2364 | m:zippyzap.shortDesc | Goes first. Raises user's evasion by 1. |
| BT-2365 | a:adaptability.desc | This Pokemon's moves that match one of its types have a same-type attack bonus (STAB) o... |
| BT-2366 | a:adaptability.shortDesc | This Pokemon's same-type attack bonus (STAB) is 2 instead of 1.5. |
| BT-2367 | a:aerilate.desc | This Pokemon's Normal-type moves become Flying-type moves and have their power multipli... |
| BT-2368 | a:aerilate.shortDesc | This Pokemon's Normal-type moves become Flying type and have 1.2× power. |
| BT-2370 | a:aftermath.desc | If this Pokemon is knocked out with a contact move, that move's user loses 1/4 of its m... |
| BT-2371 | a:aftermath.shortDesc | If this Pokemon is KOed with a contact move, that move's user loses 1/4 its max HP. |
| BT-2372 | a:airlock.shortDesc | While this Pokemon is active, the effects of weather conditions are disabled. |
| BT-2374 | a:analytic.desc | The power of this Pokemon's move is multiplied by 1.3 if it is the last to move in a tu... |
| BT-2375 | a:analytic.shortDesc | This Pokemon's attacks have 1.3× power if it is the last to move in a turn. |
| BT-2377 | a:angerpoint.desc | If this Pokemon, but not its substitute, is struck by a critical hit, its Attack is rai... |
| BT-2378 | a:angerpoint.shortDesc | If this Pokemon (not its substitute) takes a critical hit, its Attack is raised 12 stages. |
| BT-2379 | a:angershell.desc | When this Pokemon has more than 1/2 its maximum HP and takes damage from an attack brin... |
| BT-2380 | a:angershell.shortDesc | At 1/2 or less of this Pokemon's max HP: +1 Atk, Sp. Atk, Spe, and -1 Def, Sp. Def. |
| BT-2382 | a:anticipation.desc | On switch-in, this Pokemon is alerted if any opposing Pokemon has an attacking move wit... |
| BT-2383 | a:anticipation.shortDesc | On switch-in, this Pokemon shudders if any foe has a supereffective or OHKO move. |
| BT-2384 | a:arenatrap.desc | Prevents opposing Pokemon from choosing to switch out unless they are airborne, are hol... |
| BT-2385 | a:arenatrap.shortDesc | Prevents opposing Pokemon from choosing to switch out unless they are airborne. |
| BT-2387 | a:armortail.desc | Priority moves used by opposing Pokemon targeting this Pokemon or its allies are preven... |
| BT-2388 | a:armortail.shortDesc | This Pokemon and its allies are protected from opposing priority moves. |
| BT-2390 | a:aromaveil.desc | This Pokemon and its allies cannot become affected by Attract, Disable, Encore, Heal Bl... |
| BT-2391 | a:aromaveil.shortDesc | Protects user/allies from Attract, Disable, Encore, Heal Block, Taunt, and Torment. |
| BT-2392 | a:asone.shortDesc | See 'As One (Glastrier)' and 'As One (Spectrier)'. |
| BT-2394 | a:asoneglastrier.shortDesc | Combination of the Unnerve and Chilling Neigh Abilities. |
| BT-2395 | a:asonespectrier.shortDesc | Combination of the Unnerve and Grim Neigh Abilities. |
| BT-2396 | a:aurabreak.desc | While this Pokemon is active, the effects of the Dark Aura and Fairy Aura Abilities are... |
| BT-2397 | a:aurabreak.shortDesc | While this Pokemon is active, the Dark Aura and Fairy Aura power modifier is 0.75×. |
| BT-2399 | a:auraguard.shortDesc | This Pokemon receives 1/2 damage from contact moves. |
| BT-2401 | a:baddreams.desc | Causes opposing Pokemon to lose 1/8 of their maximum HP, rounded down, at the end of ea... |
| BT-2402 | a:baddreams.shortDesc | Causes sleeping foes to lose 1/8 of their max HP at the end of each turn. |
| BT-2403 | a:ballfetch.shortDesc | No competitive use. |
| BT-2404 | a:battery.shortDesc | This Pokemon's allies have the power of their special attacks multiplied by 1.3. |
| BT-2405 | a:battlearmor.shortDesc | This Pokemon cannot be struck by a critical hit. |
| BT-2407 | a:battlebond.desc | If this Pokemon is a Greninja, its Attack, Special Attack, and Speed are raised by 1 st... |
| BT-2408 | a:battlebond.shortDesc | After KOing a Pokemon: raises Attack, Sp. Atk, Speed by 1 stage. Once per battle. |
| BT-2410 | a:beadsofruin.shortDesc | Active Pokemon without this Ability have their Special Defense multiplied by 0.75. |
| BT-2412 | a:beastboost.desc | This Pokemon's highest stat is raised by 1 stage if it attacks and knocks out another P... |
| BT-2413 | a:beastboost.shortDesc | This Pokemon's highest stat is raised by 1 if it attacks and KOes another Pokemon. |
| BT-2414 | a:berserk.desc | When this Pokemon has more than 1/2 its maximum HP and takes damage from an attack brin... |
| BT-2415 | a:berserk.shortDesc | This Pokemon's Sp. Atk is raised by 1 when it reaches 1/2 or less of its max HP. |
| BT-2416 | a:bigpecks.shortDesc | Prevents other Pokemon from lowering this Pokemon's Defense stat stage. |
| BT-2417 | a:blaze.desc | When this Pokemon has 1/3 or less of its maximum HP, rounded down, its offensive stat i... |
| BT-2418 | a:blaze.shortDesc | At 1/3 or less of its max HP, this Pokemon's offensive stat is 1.5× with Fire attacks. |
| BT-2419 | a:bulletproof.shortDesc | This Pokemon is immune to bullet moves. |
| BT-2420 | a:cheekpouch.desc | If this Pokemon eats a held Berry, it restores 1/3 of its maximum HP, rounded down, in ... |
| BT-2421 | a:cheekpouch.shortDesc | If this Pokemon eats a Berry, it restores 1/3 of its max HP after the Berry's effect. |
| BT-2422 | a:chillingneigh.desc | This Pokemon's Attack is raised by 1 stage if it attacks and knocks out another Pokemon. |
| BT-2423 | a:chillingneigh.shortDesc | This Pokemon's Attack is raised by 1 stage if it attacks and KOes another Pokemon. |
| BT-2424 | a:chlorophyll.desc | If Sun is active, this Pokemon's Speed is doubled. This effect is prevented if this Pok... |
| BT-2425 | a:chlorophyll.shortDesc | If Sun is active, this Pokemon's Speed is doubled. |
| BT-2426 | a:clearbody.shortDesc | Prevents other Pokemon from lowering this Pokemon's stat stages. |
| BT-2427 | a:cloudnine.shortDesc | While this Pokemon is active, the effects of weather conditions are disabled. |
| BT-2429 | a:colorchange.desc | This Pokemon's type changes to match the type of the last move that hit it, unless that... |
| BT-2430 | a:colorchange.shortDesc | This Pokemon's type changes to the type of a move it's hit by, unless it has the type. |
| BT-2431 | a:comatose.desc | This Pokemon is considered to be asleep and cannot become affected by a non-volatile st... |
| BT-2432 | a:comatose.shortDesc | This Pokemon cannot be statused, and is considered to be asleep. |
| BT-2435 | a:commander.desc | If this Pokemon is a Tatsugiri and a Dondozo is an active ally, this Pokemon goes into ... |
| BT-2436 | a:commander.shortDesc | If ally is Dondozo: this Pokemon cannot act or be hit, +2 to all Dondozo's stats. |
| BT-2437 | a:competitive.desc | This Pokemon's Special Attack is raised by 2 stages for each of its stat stages that is... |
| BT-2438 | a:competitive.shortDesc | This Pokemon's Sp. Atk is raised by 2 for each of its stats that is lowered by a foe. |
| BT-2439 | a:compoundeyes.shortDesc | This Pokemon's moves have their accuracy multiplied by 1.3. |
| BT-2440 | a:contrary.shortDesc | If this Pokemon has a stat stage raised it is lowered instead, and vice versa. |
| BT-2441 | a:corrosion.shortDesc | This Pokemon can poison or badly poison a Pokemon regardless of its typing. |
| BT-2442 | a:costar.shortDesc | On switch-in, this Pokemon copies all of its ally's stat stage changes. |
| BT-2443 | a:cottondown.desc | When this Pokemon is hit by an attack, the Speed of all other Pokemon on the field is l... |
| BT-2444 | a:cottondown.shortDesc | If this Pokemon is hit, it lowers the Speed of all other Pokemon on the field 1 stage. |
| BT-2445 | a:cudchew.shortDesc | If this Pokemon eats a Berry, it will eat that Berry again at the end of the next turn. |
| BT-2446 | a:curiousmedicine.shortDesc | On switch-in, this Pokemon's allies have their stat stages reset to 0. |
| BT-2447 | a:cursedbody.desc | If this Pokemon is hit by an attack, there is a 30% chance that move gets disabled unle... |
| BT-2448 | a:cursedbody.shortDesc | If this Pokemon is hit by an attack, there is a 30% chance that move gets disabled. |
| BT-2449 | a:cutecharm.desc | There is a 30% chance a Pokemon making contact with this Pokemon will become infatuated... |
| BT-2450 | a:cutecharm.shortDesc | 30% chance of infatuating Pokemon of the opposite gender if they make contact. |
| BT-2452 | a:damp.desc | While this Pokemon is active, Explosion, Mind Blown, Misty Explosion, Self-Destruct, an... |
| BT-2453 | a:damp.shortDesc | Prevents Explosion/Mind Blown/Misty Explosion/Self-Destruct/Aftermath while active. |
| BT-2454 | a:dancer.desc | After another Pokemon uses a dance move, this Pokemon uses the same move. The copied mo... |
| BT-2455 | a:dancer.shortDesc | After another Pokemon uses a dance move, this Pokemon uses the same move. |
| BT-2456 | a:darkaura.desc | While this Pokemon is active, the power of Dark-type moves used by active Pokemon is mu... |
| BT-2457 | a:darkaura.shortDesc | While this Pokemon is active, a Dark move used by any Pokemon has 1.33× power. |
| BT-2459 | a:dauntlessshield.shortDesc | On switch-in, this Pokemon's Defense is raised by 1 stage. Once per battle. |
| BT-2461 | a:dazzling.desc | Priority moves used by opposing Pokemon targeting this Pokemon or its allies are preven... |
| BT-2462 | a:dazzling.shortDesc | This Pokemon and its allies are protected from opposing priority moves. |
| BT-2463 | a:defeatist.desc | While this Pokemon has 1/2 or less of its maximum HP, its Attack and Special Attack are... |
| BT-2464 | a:defeatist.shortDesc | While this Pokemon has 1/2 or less of its max HP, its Attack and Sp. Atk are halved. |
| BT-2465 | a:defiant.desc | This Pokemon's Attack is raised by 2 stages for each of its stat stages that is lowered... |
| BT-2466 | a:defiant.shortDesc | This Pokemon's Attack is raised by 2 for each of its stats that is lowered by a foe. |
| BT-2467 | a:deltastream.desc | On switch-in, the weather becomes Strong Winds, which removes the weaknesses of the Fly... |
| BT-2468 | a:deltastream.shortDesc | On switch-in, strong winds begin until this Ability is not active in battle. |
| BT-2469 | a:desolateland.desc | On switch-in, the weather becomes Intense Sun, which includes all the effects of Sun an... |
| BT-2470 | a:desolateland.shortDesc | On switch-in, extremely harsh sunlight begins until this Ability is not active in battle. |
| BT-2472 | a:disguise.desc | If this Pokemon is a Mimikyu, the first hit it takes in battle deals 0 neutral damage. ... |
| BT-2473 | a:disguise.shortDesc | (Mimikyu only) The first hit it takes is blocked, and it takes 1/8 HP damage instead. |
| BT-2475 | a:download.desc | On switch-in, this Pokemon's Attack or Special Attack is raised by 1 stage based on the... |
| BT-2476 | a:download.shortDesc | On switch-in, Attack or Sp. Atk is raised 1 stage based on the foes' weaker Defense. |
| BT-2477 | a:dragonize.desc | This Pokemon's Normal-type moves become Dragon-type moves and have their power multipli... |
| BT-2478 | a:dragonize.shortDesc | This Pokemon's Normal-type moves become Dragon type and have 1.2× power. |
| BT-2479 | a:dragonsmaw.shortDesc | This Pokemon's offensive stat is multiplied by 1.5 while using a Dragon-type attack. |
| BT-2480 | a:drizzle.shortDesc | On switch-in, this Pokemon summons Rain. |
| BT-2481 | a:drought.shortDesc | On switch-in, this Pokemon summons Sun. |
| BT-2483 | a:dryskin.desc | This Pokemon is immune to Water-type moves and restores 1/4 of its maximum HP, rounded ... |
| BT-2484 | a:dryskin.shortDesc | This Pokemon is healed 1/4 by Water, 1/8 by Rain; is hurt 1.25× by Fire, 1/8 by Sun. |
| BT-2485 | a:earlybird.shortDesc | This Pokemon's sleep counter drops by 2 instead of 1. |
| BT-2486 | a:eartheater.desc | This Pokemon is immune to Ground-type moves and restores 1/4 of its maximum HP, rounded... |
| BT-2487 | a:eartheater.shortDesc | This Pokemon heals 1/4 of its max HP when hit by Ground moves; Ground immunity. |
| BT-2488 | a:eelevate.desc | This Pokemon is immune to Ground-type attacks and the effects of Spikes, Toxic Spikes, ... |
| BT-2489 | a:eelevate.shortDesc | This Pokemon is immune to Ground; +1 to highest stat if it KOes another Pokemon. |
| BT-2490 | a:effectspore.desc | 30% chance a Pokemon making contact with this Pokemon will be poisoned, paralyzed, or f... |
| BT-2491 | a:effectspore.shortDesc | 30% chance of poison/paralysis/sleep on others making contact with this Pokemon. |
| BT-2492 | a:electricsurge.shortDesc | On switch-in, this Pokemon summons Electric Terrain. |
| BT-2493 | a:electromorphosis.shortDesc | This Pokemon gains the Charge effect when it takes a hit from an attack. |
| BT-2496 | a:embodyaspectcornerstone.shortDesc | On switch-in, this Pokemon's Defense is raised by 1 stage. |
| BT-2498 | a:embodyaspecthearthflame.shortDesc | On switch-in, this Pokemon's Attack is raised by 1 stage. |
| BT-2500 | a:embodyaspectteal.shortDesc | On switch-in, this Pokemon's Speed is raised by 1 stage. |
| BT-2502 | a:embodyaspectwellspring.shortDesc | On switch-in, this Pokemon's Special Defense is raised by 1 stage. |
| BT-2503 | a:emergencyexit.desc | When this Pokemon has more than 1/2 its maximum HP and takes damage bringing it to 1/2 ... |
| BT-2504 | a:emergencyexit.shortDesc | This Pokemon switches out when it reaches 1/2 or less of its maximum HP. |
| BT-2505 | a:fairyaura.desc | While this Pokemon is active, the power of Fairy-type moves used by active Pokemon is m... |
| BT-2506 | a:fairyaura.shortDesc | While this Pokemon is active, a Fairy move used by any Pokemon has 1.33× power. |
| BT-2508 | a:filter.shortDesc | This Pokemon receives 3/4 damage from supereffective attacks. |
| BT-2509 | a:firemane.shortDesc | This Pokemon's offensive stat is multiplied by 1.5 while using a Fire-type attack. |
| BT-2510 | a:flamebody.shortDesc | 30% chance a Pokemon making contact with this Pokemon will be burned. |
| BT-2511 | a:flareboost.desc | While this Pokemon is burned, the power of its special attacks is multiplied by 1.5. |
| BT-2512 | a:flareboost.shortDesc | While this Pokemon is burned, its special attacks have 1.5× power. |
| BT-2513 | a:flashfire.desc | This Pokemon is immune to Fire-type moves. The first time it is hit by a Fire-type move... |
| BT-2514 | a:flashfire.shortDesc | This Pokemon's Fire attacks do 1.5× damage if hit by one Fire move; Fire immunity. |
| BT-2516 | a:flowergift.desc | If this Pokemon is a Cherrim and Sun is active, it changes to Sunshine Form and the Att... |
| BT-2517 | a:flowergift.shortDesc | If user is Cherrim and Sun is active, it and allies' Attack and Sp. Def are 1.5×. |
| BT-2519 | a:flowerveil.desc | Grass-type Pokemon on this Pokemon's side cannot have their stat stages lowered by othe... |
| BT-2520 | a:flowerveil.shortDesc | This side's Grass types can't have stats lowered or status inflicted by other Pokemon. |
| BT-2521 | a:fluffy.desc | This Pokemon receives 1/2 damage from contact moves, but double damage from Fire moves. |
| BT-2522 | a:fluffy.shortDesc | This Pokemon takes 1/2 damage from contact moves, 2× damage from Fire moves. |
| BT-2523 | a:forecast.desc | If this Pokemon is a Castform, its type changes to the current weather condition's type... |
| BT-2524 | a:forecast.shortDesc | Castform's type changes to the current weather condition's type, except Sandstorm. |
| BT-2527 | a:forewarn.desc | On switch-in, this Pokemon is alerted to the move with the highest power, at random, kn... |
| BT-2528 | a:forewarn.shortDesc | On switch-in, this Pokemon is alerted to the foes' move with the highest power. |
| BT-2529 | a:friendguard.shortDesc | This Pokemon's allies receive 3/4 damage from other Pokemon's attacks. |
| BT-2532 | a:frisk.shortDesc | On switch-in, this Pokemon identifies the held items of all opposing Pokemon. |
| BT-2533 | a:fullmetalbody.shortDesc | Prevents other Pokemon from lowering this Pokemon's stat stages. |
| BT-2534 | a:furcoat.shortDesc | This Pokemon's Defense is doubled. |
| BT-2535 | a:galewings.shortDesc | If this Pokemon is at full HP, its Flying-type moves have their priority increased by 1. |
| BT-2536 | a:galvanize.desc | This Pokemon's Normal-type moves become Electric-type moves and have their power multip... |
| BT-2537 | a:galvanize.shortDesc | This Pokemon's Normal-type moves become Electric type and have 1.2× power. |
| BT-2538 | a:gluttony.desc | When this Pokemon is holding a Berry that usually activates with 1/4 or less of its max... |
| BT-2539 | a:gluttony.shortDesc | This Pokemon eats Berries at 1/2 max HP or less instead of their usual 1/4 max HP. |
| BT-2540 | a:goodasgold.shortDesc | This Pokemon is immune to Status moves. |
| BT-2541 | a:gooey.shortDesc | Pokemon making contact with this Pokemon have their Speed lowered by 1 stage. |
| BT-2542 | a:gorillatactics.desc | This Pokemon's Attack is multiplied by 1.5, but it can only select the first move it ex... |
| BT-2543 | a:gorillatactics.shortDesc | This Pokemon's Attack is 1.5×, but it can only select the first move it executes. |
| BT-2544 | a:grasspelt.shortDesc | If Grassy Terrain is active, this Pokemon's Defense is multiplied by 1.5. |
| BT-2545 | a:grassysurge.shortDesc | On switch-in, this Pokemon summons Grassy Terrain. |
| BT-2546 | a:grimneigh.desc | This Pokemon's Special Attack is raised by 1 stage if it attacks and knocks out another... |
| BT-2547 | a:grimneigh.shortDesc | This Pokemon's Sp. Atk is raised by 1 stage if it attacks and KOes another Pokemon. |
| BT-2548 | a:guarddog.desc | This Pokemon is immune to the effect of the Intimidate Ability and raises its Attack by... |
| BT-2549 | a:guarddog.shortDesc | Immune to Intimidate. Intimidated: +1 Attack. Cannot be forced to switch out. |
| BT-2550 | a:gulpmissile.desc | If this Pokemon is a Cramorant, it changes forme when it hits a target with Surf or use... |
| BT-2551 | a:gulpmissile.shortDesc | When hit after Surf/Dive, attacker takes 1/4 max HP and -1 Defense or paralysis. |
| BT-2552 | a:guts.desc | If this Pokemon has a non-volatile status condition, its Attack is multiplied by 1.5. T... |
| BT-2553 | a:guts.shortDesc | If this Pokemon is statused, its Attack is 1.5×; ignores burn halving physical damage. |
| BT-2555 | a:hadronengine.shortDesc | On switch-in, summons Electric Terrain. During Electric Terrain, Sp. Atk is 1.3333×. |
| BT-2558 | a:harvest.desc | If the last item this Pokemon used is a Berry, there is a 50% chance it gets restored a... |
| BT-2559 | a:harvest.shortDesc | If last item used is a Berry, 50% chance to restore it each end of turn. 100% in Sun. |
| BT-2560 | a:healer.desc | 30% chance this Pokemon's ally has its non-volatile status condition cured at the end o... |
| BT-2561 | a:healer.shortDesc | 30% chance this Pokemon's ally has its status cured at the end of each turn. |
| BT-2562 | a:heatproof.desc | If a Pokemon uses a Fire-type attack against this Pokemon, that Pokemon's offensive sta... |
| BT-2563 | a:heatproof.shortDesc | Fire damage against this Pokemon is dealt with 1/2 offensive stat; 1/2 burn damage. |
| BT-2564 | a:heavymetal.desc | This Pokemon's weight is doubled. This effect is calculated after the effect of Autotom... |
| BT-2565 | a:heavymetal.shortDesc | This Pokemon's weight is doubled. |
| BT-2566 | a:honeygather.shortDesc | No competitive use. |
| BT-2568 | a:hospitality.shortDesc | On switch-in, this Pokemon restores 1/4 of its ally's maximum HP, rounded down. |
| BT-2569 | a:hugepower.shortDesc | This Pokemon's Attack is doubled. |
| BT-2570 | a:hungerswitch.desc | If this Pokemon is a Morpeko, it changes formes between its Full Belly Mode and Hangry ... |
| BT-2571 | a:hungerswitch.shortDesc | If Morpeko, it changes between Full Belly and Hangry Mode at the end of each turn. |
| BT-2572 | a:hustle.desc | This Pokemon's Attack is multiplied by 1.5 and the accuracy of its physical attacks is ... |
| BT-2573 | a:hustle.shortDesc | This Pokemon's Attack is 1.5× and accuracy of its physical attacks is 0.8×. |
| BT-2574 | a:hydration.desc | This Pokemon has its non-volatile status condition cured at the end of each turn if Rai... |
| BT-2575 | a:hydration.shortDesc | This Pokemon has its status cured at the end of each turn if Rain is active. |
| BT-2576 | a:hypercutter.shortDesc | Prevents other Pokemon from lowering this Pokemon's Attack stat stage. |
| BT-2577 | a:icebody.desc | If Snow is active, this Pokemon restores 1/16 of its maximum HP, rounded down, at the e... |
| BT-2578 | a:icebody.shortDesc | If Snow is active, this Pokemon heals 1/16 of its max HP each turn. |
| BT-2579 | a:iceface.desc | If this Pokemon is an Eiscue, the first physical hit it takes in battle deals 0 neutral... |
| BT-2580 | a:iceface.shortDesc | If Eiscue, the first physical hit it takes deals 0 damage. Effect is restored in Snow. |
| BT-2581 | a:icescales.shortDesc | This Pokemon receives 1/2 damage from special attacks. |
| BT-2582 | a:illuminate.desc | Prevents other Pokemon from lowering this Pokemon's accuracy stat stage. This Pokemon i... |
| BT-2583 | a:illuminate.shortDesc | This Pokemon's accuracy can't be lowered by others; ignores their evasiveness stat. |
| BT-2584 | a:illusion.desc | When this Pokemon switches in, it appears as the last unfainted Pokemon in its party un... |
| BT-2586 | a:illusion.shortDesc | This Pokemon appears as the last Pokemon in the party until it takes direct damage. |
| BT-2587 | a:immunity.shortDesc | This Pokemon cannot be poisoned. Gaining this Ability while poisoned cures it. |
| BT-2588 | a:imposter.desc | On switch-in, this Pokemon Transforms into the opposing Pokemon that is facing it. If t... |
| BT-2589 | a:imposter.shortDesc | On switch-in, this Pokemon Transforms into the opposing Pokemon that is facing it. |
| BT-2590 | a:infiltrator.desc | This Pokemon's moves ignore substitutes and the opposing side's Reflect, Light Screen, ... |
| BT-2591 | a:infiltrator.shortDesc | Moves ignore substitutes and foe's Reflect/Light Screen/Safeguard/Mist/Aurora Veil. |
| BT-2593 | a:innardsout.desc | If this Pokemon is knocked out with a move, that move's user loses HP equal to the amou... |
| BT-2594 | a:innardsout.shortDesc | If this Pokemon is KOed with a move, that move's user loses an equal amount of HP. |
| BT-2595 | a:innerfocus.desc | This Pokemon cannot be made to flinch. This Pokemon is immune to the effect of the Inti... |
| BT-2596 | a:innerfocus.shortDesc | This Pokemon cannot be made to flinch. Immune to Intimidate. |
| BT-2597 | a:insomnia.shortDesc | This Pokemon cannot fall asleep. Gaining this Ability while asleep cures it. |
| BT-2598 | a:intimidate.desc | On switch-in, this Pokemon lowers the Attack of opposing Pokemon by 1 stage. Pokemon wi... |
| BT-2599 | a:intimidate.shortDesc | On switch-in, this Pokemon lowers the Attack of opponents by 1 stage. |
| BT-2600 | a:intrepidsword.shortDesc | On switch-in, this Pokemon's Attack is raised by 1 stage. Once per battle. |
| BT-2602 | a:ironbarbs.desc | Pokemon making contact with this Pokemon lose 1/8 of their maximum HP, rounded down. |
| BT-2603 | a:ironbarbs.shortDesc | Pokemon making contact with this Pokemon lose 1/8 of their max HP. |
| BT-2604 | a:ironfist.desc | This Pokemon's punch-based attacks have their power multiplied by 1.2. |
| BT-2605 | a:ironfist.shortDesc | This Pokemon's punch-based attacks have 1.2× power. Sucker Punch is not boosted. |
| BT-2606 | a:justified.shortDesc | This Pokemon's Attack is raised by 1 stage after it is damaged by a Dark-type move. |
| BT-2607 | a:keeneye.desc | Prevents other Pokemon from lowering this Pokemon's accuracy stat stage. This Pokemon i... |
| BT-2608 | a:keeneye.shortDesc | This Pokemon's accuracy can't be lowered by others; ignores their evasiveness stat. |
| BT-2609 | a:klutz.desc | This Pokemon's held item has no effect. This Pokemon cannot use Fling successfully. Mac... |
| BT-2610 | a:klutz.shortDesc | This Pokemon's held item has no effect, except Macho Brace. Fling cannot be used. |
| BT-2611 | a:leafguard.desc | If Sun is active, this Pokemon cannot become affected by a non-volatile status conditio... |
| BT-2612 | a:leafguard.shortDesc | If Sun is active, this Pokemon cannot be statused and Rest will fail for it. |
| BT-2613 | a:levitate.desc | This Pokemon is immune to Ground-type attacks and the effects of Spikes, Toxic Spikes, ... |
| BT-2614 | a:levitate.shortDesc | This Pokemon is immune to Ground; Gravity/Ingrain/Smack Down/Iron Ball nullify it. |
| BT-2615 | a:libero.desc | This Pokemon's type changes to match the type of the move it is about to use. This effe... |
| BT-2616 | a:libero.shortDesc | This Pokemon's type changes to the type of the move it is using. Once per switch-in. |
| BT-2617 | a:lightmetal.desc | This Pokemon's weight is halved, rounded down to a tenth of a kilogram. This effect is ... |
| BT-2618 | a:lightmetal.shortDesc | This Pokemon's weight is halved. |
| BT-2620 | a:lightningrod.desc | This Pokemon is immune to Electric-type moves and raises its Special Attack by 1 stage ... |
| BT-2621 | a:lightningrod.shortDesc | This Pokemon draws Electric moves to itself to raise Sp. Atk by 1; Electric immunity. |
| BT-2622 | a:limber.shortDesc | This Pokemon cannot be paralyzed. Gaining this Ability while paralyzed cures it. |
| BT-2624 | a:lingeringaroma.desc | Pokemon making contact with this Pokemon have their Ability changed to Lingering Aroma.... |
| BT-2625 | a:lingeringaroma.shortDesc | Making contact with this Pokemon has the attacker's Ability become Lingering Aroma. |
| BT-2627 | a:liquidooze.shortDesc | This Pokemon damages those draining HP from it for as much as they would heal. |
| BT-2628 | a:liquidvoice.desc | This Pokemon's sound-based moves become Water-type moves. This effect comes after other... |
| BT-2629 | a:liquidvoice.shortDesc | This Pokemon's sound-based moves become Water type. |
| BT-2630 | a:longreach.shortDesc | This Pokemon's attacks do not make contact with the target. |
| BT-2631 | a:magicbounce.desc | This Pokemon is unaffected by certain non-damaging moves directed at it and will instea... |
| BT-2633 | a:magicbounce.shortDesc | This Pokemon blocks certain Status moves and bounces them back to the user. |
| BT-2634 | a:magicguard.desc | This Pokemon can only be damaged by direct attacks. Curse and Substitute on use, Belly ... |
| BT-2635 | a:magicguard.shortDesc | This Pokemon can only be damaged by direct attacks. |
| BT-2636 | a:magician.desc | If this Pokemon has no item, it steals the item off a Pokemon it hits with an attack. D... |
| BT-2637 | a:magician.shortDesc | If this Pokemon has no item, it steals the item off a Pokemon it hits with an attack. |
| BT-2638 | a:magmaarmor.shortDesc | This Pokemon cannot be frozen. Gaining this Ability while frozen cures it. |
| BT-2639 | a:magnetpull.desc | Prevents opposing Steel-type Pokemon from choosing to switch out, unless they are holdi... |
| BT-2640 | a:magnetpull.shortDesc | Prevents opposing Steel-type Pokemon from choosing to switch out. |
| BT-2641 | a:marvelscale.shortDesc | If this Pokemon has a non-volatile status condition, its Defense is multiplied by 1.5. |
| BT-2642 | a:megalauncher.desc | This Pokemon's pulse moves have their power multiplied by 1.5. Heal Pulse restores 3/4 ... |
| BT-2643 | a:megalauncher.shortDesc | This Pokemon's pulse moves have 1.5× power. Heal Pulse heals 3/4 target's max HP. |
| BT-2644 | a:megasol.shortDesc | This Pokemon's moves are used as if the effects of Sun were active. |
| BT-2645 | a:merciless.shortDesc | This Pokemon's attacks are critical hits if the target is poisoned. |
| BT-2647 | a:mimicry.desc | This Pokemon's types change to match the active Terrain when this Pokemon acquires this... |
| BT-2648 | a:mimicry.shortDesc | This Pokemon's types change to match the Terrain. Type reverts when Terrain ends. |
| BT-2649 | a:mindseye.desc | This Pokemon can hit Ghost types with Normal- and Fighting-type moves. Prevents other P... |
| BT-2650 | a:mindseye.shortDesc | Fighting, Normal moves hit Ghost. Accuracy can't be lowered, ignores evasiveness. |
| BT-2651 | a:minus.desc | If an active ally has this Ability or the Plus Ability, this Pokemon's Special Attack i... |
| BT-2652 | a:minus.shortDesc | If an active ally has this Ability or the Plus Ability, this Pokemon's Sp. Atk is 1.5×. |
| BT-2653 | a:mirrorarmor.desc | When one of this Pokemon's stat stages would be lowered by another Pokemon, that Pokemo... |
| BT-2654 | a:mirrorarmor.shortDesc | If this Pokemon's stat stages would be lowered, the attacker's are lowered instead. |
| BT-2655 | a:mistysurge.shortDesc | On switch-in, this Pokemon summons Misty Terrain. |
| BT-2656 | a:moldbreaker.desc | This Pokemon's moves and their effects ignore certain Abilities of other Pokemon. The A... |
| BT-2657 | a:moldbreaker.shortDesc | This Pokemon's moves and their effects ignore the Abilities of other Pokemon. |
| BT-2659 | a:moody.desc | This Pokemon has a random stat, other than accuracy or evasiveness, raised by 2 stages ... |
| BT-2660 | a:moody.shortDesc | Boosts a random stat (except accuracy/evasion) +2 and another stat -1 every turn. |
| BT-2661 | a:motordrive.desc | This Pokemon is immune to Electric-type moves and raises its Speed by 1 stage when hit ... |
| BT-2662 | a:motordrive.shortDesc | This Pokemon's Speed is raised 1 stage if hit by an Electric move; Electric immunity. |
| BT-2663 | a:mountaineer.shortDesc | On switch-in, this Pokemon avoids all Rock-type attacks and Stealth Rock. |
| BT-2664 | a:moxie.desc | This Pokemon's Attack is raised by 1 stage if it attacks and knocks out another Pokemon. |
| BT-2665 | a:moxie.shortDesc | This Pokemon's Attack is raised by 1 stage if it attacks and KOes another Pokemon. |
| BT-2666 | a:multiscale.shortDesc | If this Pokemon is at full HP, damage taken from attacks is halved. |
| BT-2667 | a:multitype.shortDesc | If this Pokemon is an Arceus, its type changes to match its held Plate. |
| BT-2669 | a:mummy.desc | Pokemon making contact with this Pokemon have their Ability changed to Mummy. Does not ... |
| BT-2670 | a:mummy.shortDesc | Pokemon making contact with this Pokemon have their Ability changed to Mummy. |
| BT-2671 | a:myceliummight.desc | This Pokemon's Status moves ignore certain Abilities of other Pokemon, and go last amon... |
| BT-2672 | a:myceliummight.shortDesc | This Pokemon's Status moves go last in their priority bracket and ignore Abilities. |
| BT-2674 | a:naturalcure.shortDesc | This Pokemon has its non-volatile status condition cured when it switches out. |
| BT-2675 | a:neuroforce.desc | This Pokemon's attacks that are super effective against the target have their damage mu... |
| BT-2676 | a:neuroforce.shortDesc | This Pokemon's attacks that are super effective against the target do 1.25× damage. |
| BT-2677 | a:neutralizinggas.desc | While this Pokemon is active, Abilities have no effect. This Ability activates before h... |
| BT-2679 | a:neutralizinggas.shortDesc | While this Pokemon is active, Abilities have no effect. |
| BT-2681 | a:noability.shortDesc | Does nothing. |
| BT-2682 | a:noguard.shortDesc | Every move used by or against this Pokemon will always hit. |
| BT-2683 | a:normalize.desc | This Pokemon's moves are changed to be Normal type and have their power multiplied by 1... |
| BT-2684 | a:normalize.shortDesc | This Pokemon's moves are changed to be Normal type and have 1.2× power. |
| BT-2685 | a:oblivious.desc | This Pokemon cannot be infatuated or taunted. Gaining this Ability while infatuated or ... |
| BT-2686 | a:oblivious.shortDesc | This Pokemon cannot be infatuated or taunted. Immune to Intimidate. |
| BT-2687 | a:opportunist.shortDesc | When an opposing Pokemon has a stat stage raised, this Pokemon copies the effect. |
| BT-2689 | a:orichalcumpulse.shortDesc | On switch-in, summons Sun. During Sun, Attack is 1.3333×. |
| BT-2691 | a:overcoat.desc | This Pokemon is immune to powder moves, damage from Sandstorm, and the effects of Rage ... |
| BT-2692 | a:overcoat.shortDesc | This Pokemon is immune to powder moves, Sandstorm damage, and Effect Spore. |
| BT-2693 | a:overgrow.desc | When this Pokemon has 1/3 or less of its maximum HP, rounded down, its offensive stat i... |
| BT-2694 | a:overgrow.shortDesc | At 1/3 or less of its max HP, this Pokemon's offensive stat is 1.5× with Grass attacks. |
| BT-2695 | a:owntempo.desc | This Pokemon cannot be confused. Gaining this Ability while confused cures it. This Pok... |
| BT-2696 | a:owntempo.shortDesc | This Pokemon cannot be confused. Immune to Intimidate. |
| BT-2697 | a:parentalbond.desc | This Pokemon's damaging moves become multi-hit moves that hit twice. The second hit has... |
| BT-2698 | a:parentalbond.shortDesc | This Pokemon's damaging moves hit twice. The second hit has its damage quartered. |
| BT-2699 | a:pastelveil.desc | This Pokemon and its allies cannot be poisoned. Gaining this Ability while this Pokemon... |
| BT-2700 | a:pastelveil.shortDesc | This Pokemon and its allies cannot be poisoned. On switch-in, cures poisoned allies. |
| BT-2701 | a:perishbody.desc | Making contact with this Pokemon starts the Perish Song effect for it and the attacker.... |
| BT-2702 | a:perishbody.shortDesc | Making contact with this Pokemon starts the Perish Song effect for it and the attacker. |
| BT-2705 | a:persistent.desc | The duration of Gravity, Heal Block, Magic Room, Safeguard, Tailwind, Trick Room, and W... |
| BT-2706 | a:persistent.shortDesc | When used, Gravity/Heal Block/Safeguard/Tailwind/Room effects last 2 more turns. |
| BT-2707 | a:pickpocket.desc | If this Pokemon has no item and is hit by a contact move, it steals the attacker's item... |
| BT-2708 | a:pickpocket.shortDesc | If this Pokemon has no item and is hit by a contact move, it steals the attacker's item. |
| BT-2710 | a:pickup.desc | At the end of each turn, if this Pokemon is not holding an item and at least one adjace... |
| BT-2711 | a:pickup.shortDesc | If this Pokemon has no item, it finds one used by an adjacent Pokemon this turn. |
| BT-2712 | a:piercingdrill.shortDesc | This Pokemon's contact moves ignore a target's protection and deal 1/4 the usual damage. |
| BT-2713 | a:pixilate.desc | This Pokemon's Normal-type moves become Fairy-type moves and have their power multiplie... |
| BT-2714 | a:pixilate.shortDesc | This Pokemon's Normal-type moves become Fairy type and have 1.2× power. |
| BT-2715 | a:plus.desc | If an active ally has this Ability or the Minus Ability, this Pokemon's Special Attack ... |
| BT-2716 | a:plus.shortDesc | If an active ally has this Ability or the Minus Ability, this Pokemon's Sp. Atk is 1.5×. |
| BT-2717 | a:poisonheal.desc | If this Pokemon is poisoned, it restores 1/8 of its maximum HP, rounded down, at the en... |
| BT-2718 | a:poisonheal.shortDesc | This Pokemon is healed by 1/8 of its max HP each turn when poisoned; no HP loss. |
| BT-2719 | a:poisonpoint.shortDesc | 30% chance a Pokemon making contact with this Pokemon will be poisoned. |
| BT-2720 | a:poisonpuppeteer.desc | If this Pokemon is a Pecharunt and poisons or badly poisons a target, the target also b... |
| BT-2721 | a:poisonpuppeteer.shortDesc | Pecharunt: If this Pokemon poisons a target, the target also becomes confused. |
| BT-2722 | a:poisontouch.desc | This Pokemon's contact moves have a 30% chance of poisoning. This effect comes after a ... |
| BT-2723 | a:poisontouch.shortDesc | This Pokemon's contact moves have a 30% chance of poisoning. |
| BT-2725 | a:powerconstruct.desc | If this Pokemon is a Zygarde in its 10% or 50% Forme, it changes to Complete Forme when... |
| BT-2726 | a:powerconstruct.shortDesc | If Zygarde 10%/50%, changes to Complete if at 1/2 max HP or less at end of turn. |
| BT-2729 | a:powerofalchemy.desc | This Pokemon copies the Ability of an ally that faints. Abilities that cannot be copied... |
| BT-2730 | a:powerofalchemy.shortDesc | This Pokemon copies the Ability of an ally that faints. |
| BT-2731 | a:powerspot.desc | This Pokemon's allies have the power of their moves multiplied by 1.3. This affects Doo... |
| BT-2732 | a:powerspot.shortDesc | This Pokemon's allies have the power of their moves multiplied by 1.3. |
| BT-2733 | a:prankster.desc | This Pokemon's non-damaging moves have their priority increased by 1. Opposing Dark-typ... |
| BT-2734 | a:prankster.shortDesc | This Pokemon's Status moves have priority raised by 1, but Dark types are immune. |
| BT-2735 | a:pressure.desc | If this Pokemon is the target of an opposing Pokemon's move, that move loses one additi... |
| BT-2736 | a:pressure.shortDesc | If this Pokemon is the target of a foe's move, that move loses one additional PP. |
| BT-2738 | a:primordialsea.desc | On switch-in, the weather becomes Heavy Rain, which includes all the effects of Rain an... |
| BT-2739 | a:primordialsea.shortDesc | On switch-in, heavy rain begins until this Ability is not active in battle. |
| BT-2740 | a:prismarmor.shortDesc | This Pokemon receives 3/4 damage from supereffective attacks. |
| BT-2741 | a:propellertail.shortDesc | This Pokemon's moves cannot be redirected to a different target by any effect. |
| BT-2742 | a:protean.desc | This Pokemon's type changes to match the type of the move it is about to use. This effe... |
| BT-2743 | a:protean.shortDesc | This Pokemon's type changes to the type of the move it is using. Once per switch-in. |
| BT-2746 | a:protosynthesis.desc | If Sun is active or this Pokemon uses a held Booster Energy, this Pokemon's highest sta... |
| BT-2748 | a:protosynthesis.shortDesc | Sun active or Booster Energy used: highest stat is 1.3×, or 1.5× if Speed. |
| BT-2750 | a:psychicsurge.shortDesc | On switch-in, this Pokemon summons Psychic Terrain. |
| BT-2751 | a:punkrock.desc | This Pokemon's sound-based moves have their power multiplied by 1.3. This Pokemon takes... |
| BT-2752 | a:punkrock.shortDesc | This Pokemon receives 1/2 damage from sound moves. Its own have 1.3× power. |
| BT-2753 | a:purepower.shortDesc | This Pokemon's Attack is doubled. |
| BT-2754 | a:purifyingsalt.desc | This Pokemon cannot become affected by a non-volatile status condition or Yawn. If a Po... |
| BT-2755 | a:purifyingsalt.shortDesc | Ghost damage to this Pokemon dealt with a halved offensive stat; can't be statused. |
| BT-2758 | a:quarkdrive.desc | If Electric Terrain is active or this Pokemon uses a held Booster Energy, this Pokemon'... |
| BT-2760 | a:quarkdrive.shortDesc | Electric Terrain active or Booster Energy used: highest stat is 1.3×, or 1.5× if Speed. |
| BT-2763 | a:queenlymajesty.desc | Priority moves used by opposing Pokemon targeting this Pokemon or its allies are preven... |
| BT-2764 | a:queenlymajesty.shortDesc | This Pokemon and its allies are protected from opposing priority moves. |
| BT-2766 | a:quickdraw.shortDesc | This Pokemon has a 30% chance to move first in its priority bracket with attacking moves. |
| BT-2767 | a:quickfeet.desc | If this Pokemon has a non-volatile status condition, its Speed is multiplied by 1.5. Th... |
| BT-2768 | a:quickfeet.shortDesc | If this Pokemon is statused, its Speed is 1.5×; ignores Speed drop from paralysis. |
| BT-2769 | a:raindish.desc | If Rain is active, this Pokemon restores 1/16 of its maximum HP, rounded down, at the e... |
| BT-2770 | a:raindish.shortDesc | If Rain is active, this Pokemon heals 1/16 of its max HP each turn. |
| BT-2771 | a:rattled.desc | This Pokemon's Speed is raised by 1 stage if hit by a Bug-, Dark-, or Ghost-type attack... |
| BT-2772 | a:rattled.shortDesc | Speed is raised 1 stage if hit by a Bug-, Dark-, or Ghost-type attack, or Intimidated. |
| BT-2773 | a:rebound.desc | On switch-in, this Pokemon blocks certain status moves and instead uses the move agains... |
| BT-2775 | a:rebound.shortDesc | On switch-in, blocks certain status moves and bounces them back to the user. |
| BT-2777 | a:receiver.desc | This Pokemon copies the Ability of an ally that faints. Abilities that cannot be copied... |
| BT-2778 | a:receiver.shortDesc | This Pokemon copies the Ability of an ally that faints. |
| BT-2779 | a:reckless.desc | This Pokemon's attacks with recoil or crash damage have their power multiplied by 1.2. ... |
| BT-2780 | a:reckless.shortDesc | This Pokemon's attacks with recoil or crash damage have 1.2× power; not Struggle. |
| BT-2781 | a:refrigerate.desc | This Pokemon's Normal-type moves become Ice-type moves and have their power multiplied ... |
| BT-2782 | a:refrigerate.shortDesc | This Pokemon's Normal-type moves become Ice type and have 1.2× power. |
| BT-2783 | a:regenerator.shortDesc | This Pokemon restores 1/3 of its maximum HP, rounded down, when it switches out. |
| BT-2784 | a:ripen.desc | When this Pokemon eats certain Berries, the effects are doubled. Berries that restore H... |
| BT-2785 | a:ripen.shortDesc | When this Pokemon eats certain Berries, the effects are doubled. |
| BT-2786 | a:rivalry.desc | This Pokemon's attacks have their power multiplied by 1.25 against targets of the same ... |
| BT-2787 | a:rivalry.shortDesc | This Pokemon's attacks do 1.25× on same gender targets; 0.75× on opposite gender. |
| BT-2788 | a:rkssystem.shortDesc | If this Pokemon is a Silvally, its type changes to match its held Memory. |
| BT-2789 | a:rockhead.desc | This Pokemon does not take recoil damage, except Struggle. Does not affect Life Orb dam... |
| BT-2790 | a:rockhead.shortDesc | This Pokemon does not take recoil damage besides Struggle/Life Orb/crash damage. |
| BT-2791 | a:rockypayload.shortDesc | This Pokemon's offensive stat is multiplied by 1.5 while using a Rock-type attack. |
| BT-2793 | a:roughskin.desc | Pokemon making contact with this Pokemon lose 1/8 of their maximum HP, rounded down. |
| BT-2794 | a:roughskin.shortDesc | Pokemon making contact with this Pokemon lose 1/8 of their max HP. |
| BT-2795 | a:runaway.shortDesc | No competitive use. |
| BT-2796 | a:sandforce.desc | If Sandstorm is active, this Pokemon's Ground-, Rock-, and Steel-type attacks have thei... |
| BT-2797 | a:sandforce.shortDesc | This Pokemon's Ground/Rock/Steel attacks do 1.3× in Sandstorm; immunity to it. |
| BT-2798 | a:sandrush.desc | If Sandstorm is active, this Pokemon's Speed is doubled. This Pokemon takes no damage f... |
| BT-2799 | a:sandrush.shortDesc | If Sandstorm is active, this Pokemon's Speed is doubled; immunity to Sandstorm. |
| BT-2800 | a:sandspit.shortDesc | When this Pokemon is hit by an attack, the effect of Sandstorm begins. |
| BT-2801 | a:sandstream.shortDesc | On switch-in, this Pokemon summons Sandstorm. |
| BT-2802 | a:sandveil.desc | If Sandstorm is active, the accuracy of moves used against this Pokemon is multiplied b... |
| BT-2803 | a:sandveil.shortDesc | If Sandstorm is active, this Pokemon's evasiveness is 1.25×; immunity to Sandstorm. |
| BT-2804 | a:sapsipper.desc | This Pokemon is immune to Grass-type moves and raises its Attack by 1 stage when hit by... |
| BT-2805 | a:sapsipper.shortDesc | This Pokemon's Attack is raised 1 stage if hit by a Grass move; Grass immunity. |
| BT-2806 | a:schooling.desc | On switch-in, if this Pokemon is a Wishiwashi that is level 20 or above and has more th... |
| BT-2807 | a:schooling.shortDesc | If user is Wishiwashi, changes to School Form if it has > 1/4 max HP, else Solo Form. |
| BT-2810 | a:scrappy.desc | This Pokemon can hit Ghost types with Normal- and Fighting-type moves. This Pokemon is ... |
| BT-2811 | a:scrappy.shortDesc | Fighting, Normal moves hit Ghost. Immune to Intimidate. |
| BT-2812 | a:screencleaner.shortDesc | On switch-in, the effects of Aurora Veil, Light Screen, and Reflect end for both sides. |
| BT-2813 | a:seedsower.shortDesc | When this Pokemon is hit by an attack, the effect of Grassy Terrain begins. |
| BT-2814 | a:serenegrace.desc | This Pokemon's moves have their secondary effect chance doubled. This effect stacks wit... |
| BT-2815 | a:serenegrace.shortDesc | This Pokemon's moves have their secondary effect chance doubled. |
| BT-2816 | a:shadowshield.shortDesc | If this Pokemon is at full HP, damage taken from attacks is halved. |
| BT-2817 | a:shadowtag.desc | Prevents opposing Pokemon from choosing to switch out, unless they are holding a Shed S... |
| BT-2818 | a:shadowtag.shortDesc | Prevents foes from choosing to switch unless they also have this Ability. |
| BT-2819 | a:sharpness.shortDesc | This Pokemon's slicing moves have their power multiplied by 1.5. |
| BT-2820 | a:shedskin.desc | This Pokemon has a 33% chance to have its non-volatile status condition cured at the en... |
| BT-2821 | a:shedskin.shortDesc | This Pokemon has a 33% chance to have its status cured at the end of each turn. |
| BT-2822 | a:sheerforce.desc | This Pokemon's attacks with secondary effects have their power multiplied by 1.3, but t... |
| BT-2823 | a:sheerforce.shortDesc | This Pokemon's attacks with secondary effects have 1.3× power; nullifies the effects. |
| BT-2824 | a:shellarmor.shortDesc | This Pokemon cannot be struck by a critical hit. |
| BT-2825 | a:shielddust.desc | This Pokemon is not affected by the secondary effect of another Pokemon's attack. Attac... |
| BT-2826 | a:shielddust.shortDesc | This Pokemon is not affected by the secondary effect of another Pokemon's attack. |
| BT-2827 | a:shieldsdown.desc | If this Pokemon is a Minior, it changes to its Core forme if it has 1/2 or less of its ... |
| BT-2828 | a:shieldsdown.shortDesc | If Minior, switch-in/end of turn it changes to Core at 1/2 max HP or less, else Meteor. |
| BT-2831 | a:simple.shortDesc | When one of this Pokemon's stat stages is raised or lowered, the amount is doubled. |
| BT-2832 | a:skilllink.desc | This Pokemon's multi-hit attacks always hit the maximum number of times. Triple Kick an... |
| BT-2833 | a:skilllink.shortDesc | This Pokemon's multi-hit attacks always hit the maximum number of times. |
| BT-2835 | a:slowstart.shortDesc | On switch-in, this Pokemon's Attack and Speed are halved for 5 turns. |
| BT-2837 | a:slushrush.shortDesc | If Snow is active, this Pokemon's Speed is doubled. |
| BT-2838 | a:sniper.shortDesc | If this Pokemon strikes with a critical hit, the damage is multiplied by 1.5. |
| BT-2839 | a:snowcloak.desc | If Snow is active, the accuracy of moves used against this Pokemon is multiplied by 0.8. |
| BT-2840 | a:snowcloak.shortDesc | If Snow is active, this Pokemon's evasiveness is 1.25×. |
| BT-2841 | a:snowwarning.shortDesc | On switch-in, this Pokemon summons Snow. |
| BT-2842 | a:solarpower.desc | If Sun is active, this Pokemon's Special Attack is multiplied by 1.5 and it loses 1/8 o... |
| BT-2843 | a:solarpower.shortDesc | If Sun is active, this Pokemon's Sp. Atk is 1.5×; loses 1/8 max HP per turn. |
| BT-2844 | a:solidrock.shortDesc | This Pokemon receives 3/4 damage from supereffective attacks. |
| BT-2845 | a:soulheart.shortDesc | This Pokemon's Special Attack is raised by 1 stage when another Pokemon faints. |
| BT-2846 | a:soundproof.shortDesc | This Pokemon is immune to sound-based moves, unless it used the move. |
| BT-2847 | a:speedboost.desc | This Pokemon's Speed is raised by 1 stage at the end of each full turn it has been on t... |
| BT-2848 | a:speedboost.shortDesc | This Pokemon's Speed is raised 1 stage at the end of each full turn on the field. |
| BT-2849 | a:spicyspray.shortDesc | If this Pokemon is hit by an attack, the attacker becomes burned. |
| BT-2850 | a:stakeout.shortDesc | This Pokemon's offensive stat is doubled against a target that switched in this turn. |
| BT-2851 | a:stall.shortDesc | This Pokemon moves last among Pokemon using the same or greater priority moves. |
| BT-2852 | a:stalwart.shortDesc | This Pokemon's moves cannot be redirected to a different target by any effect. |
| BT-2853 | a:stamina.shortDesc | This Pokemon's Defense is raised by 1 stage after it is damaged by a move. |
| BT-2854 | a:stancechange.desc | If this Pokemon is an Aegislash, it changes to Blade Forme before using an attacking mo... |
| BT-2855 | a:stancechange.shortDesc | If Aegislash, changes Forme to Blade before attacks and Shield before King's Shield. |
| BT-2858 | a:static.shortDesc | 30% chance a Pokemon making contact with this Pokemon will be paralyzed. |
| BT-2859 | a:steadfast.shortDesc | If this Pokemon flinches, its Speed is raised by 1 stage. |
| BT-2860 | a:steamengine.desc | This Pokemon's Speed is raised by 6 stages after it is damaged by a Fire- or Water-type... |
| BT-2861 | a:steamengine.shortDesc | This Pokemon's Speed is raised by 6 stages after it is damaged by Fire/Water moves. |
| BT-2862 | a:steelworker.shortDesc | This Pokemon's offensive stat is multiplied by 1.5 while using a Steel-type attack. |
| BT-2863 | a:steelyspirit.desc | This Pokemon and its allies' Steel-type moves have their power multiplied by 1.5. This ... |
| BT-2864 | a:steelyspirit.shortDesc | This Pokemon and its allies' Steel-type moves have their power multiplied by 1.5. |
| BT-2865 | a:stench.desc | This Pokemon's attacks without a chance to make the target flinch gain a 10% chance to ... |
| BT-2866 | a:stench.shortDesc | This Pokemon's attacks without a chance to flinch gain a 10% chance to flinch. |
| BT-2868 | a:stickyhold.desc | This Pokemon cannot lose its held item due to another Pokemon's Ability or attack, unle... |
| BT-2869 | a:stickyhold.shortDesc | This Pokemon cannot lose its held item due to another Pokemon's Ability or attack. |
| BT-2871 | a:stormdrain.desc | This Pokemon is immune to Water-type moves and raises its Special Attack by 1 stage whe... |
| BT-2872 | a:stormdrain.shortDesc | This Pokemon draws Water moves to itself to raise Sp. Atk by 1; Water immunity. |
| BT-2873 | a:strongjaw.desc | This Pokemon's bite-based attacks have their power multiplied by 1.5. |
| BT-2874 | a:strongjaw.shortDesc | This Pokemon's bite-based attacks have 1.5× power. Bug Bite is not boosted. |
| BT-2876 | a:sturdy.desc | If this Pokemon is at full HP, it survives one hit with at least 1 HP. OHKO moves fail ... |
| BT-2877 | a:sturdy.shortDesc | If this Pokemon is at full HP, it survives one hit with at least 1 HP. Immune to OHKO. |
| BT-2879 | a:suctioncups.shortDesc | This Pokemon cannot be forced to switch out by another Pokemon's attack or item. |
| BT-2880 | a:superluck.shortDesc | This Pokemon's critical hit ratio is raised by 1 stage. |
| BT-2881 | a:supersweetsyrup.shortDesc | On switch-in, this Pokemon lowers the evasiveness of opponents 1 stage. Once per battle. |
| BT-2884 | a:supremeoverlord.desc | This Pokemon's moves have their power multiplied by 1+(X × 0.1), where X is the total n... |
| BT-2885 | a:supremeoverlord.shortDesc | This Pokemon's moves have 10% more power for each fainted ally, up to 5 allies. |
| BT-2886 | a:surgesurfer.shortDesc | If Electric Terrain is active, this Pokemon's Speed is doubled. |
| BT-2887 | a:swarm.desc | When this Pokemon has 1/3 or less of its maximum HP, rounded down, its offensive stat i... |
| BT-2888 | a:swarm.shortDesc | At 1/3 or less of its max HP, this Pokemon's offensive stat is 1.5× with Bug attacks. |
| BT-2890 | a:sweetveil.desc | This Pokemon and its allies cannot fall asleep, but those already asleep do not wake up... |
| BT-2891 | a:sweetveil.shortDesc | This Pokemon and its allies cannot fall asleep; those already asleep do not wake up. |
| BT-2892 | a:swiftswim.desc | If Rain is active, this Pokemon's Speed is doubled. This effect is prevented if this Po... |
| BT-2893 | a:swiftswim.shortDesc | If Rain is active, this Pokemon's Speed is doubled. |
| BT-2894 | a:swordofruin.shortDesc | Active Pokemon without this Ability have their Defense multiplied by 0.75. |
| BT-2897 | a:symbiosis.desc | If an ally uses its item, this Pokemon gives its item to that ally immediately. Does no... |
| BT-2898 | a:symbiosis.shortDesc | If an ally uses its item, this Pokemon gives its item to that ally immediately. |
| BT-2899 | a:synchronize.desc | If another Pokemon burns, paralyzes, poisons, or badly poisons this Pokemon, that Pokem... |
| BT-2900 | a:synchronize.shortDesc | If another Pokemon burns/poisons/paralyzes this Pokemon, it also gets that status. |
| BT-2901 | a:tabletsofruin.shortDesc | Active Pokemon without this Ability have their Attack multiplied by 0.75. |
| BT-2903 | a:tangledfeet.shortDesc | This Pokemon's evasiveness is doubled as long as it is confused. |
| BT-2904 | a:tanglinghair.shortDesc | Pokemon making contact with this Pokemon have their Speed lowered by 1 stage. |
| BT-2905 | a:technician.desc | This Pokemon's moves of 60 power or less have their power multiplied by 1.5, including ... |
| BT-2906 | a:technician.shortDesc | This Pokemon's moves of 60 power or less have 1.5× power, including Struggle. |
| BT-2908 | a:telepathy.shortDesc | This Pokemon does not take damage from attacks made by its allies. |
| BT-2909 | a:teraformzero.shortDesc | Terapagos: Terastallizing ends the effects of weather and terrain. Once per battle. |
| BT-2911 | a:terashell.desc | If this Pokemon is a Terapagos at full HP, the effectiveness of attacks against it is c... |
| BT-2912 | a:terashell.shortDesc | Terapagos: If full HP, attacks taken have 0.5× effectiveness unless naturally immune. |
| BT-2913 | a:terashift.shortDesc | If this Pokemon is a Terapagos, it transforms into its Terastal Form on entry. |
| BT-2915 | a:teravolt.desc | This Pokemon's moves and their effects ignore certain Abilities of other Pokemon. The A... |
| BT-2916 | a:teravolt.shortDesc | This Pokemon's moves and their effects ignore the Abilities of other Pokemon. |
| BT-2918 | a:thermalexchange.desc | This Pokemon's Attack is raised 1 stage after it is damaged by a Fire-type move. This P... |
| BT-2919 | a:thermalexchange.shortDesc | This Pokemon's Attack is raised by 1 when damaged by Fire moves; can't be burned. |
| BT-2920 | a:thickfat.desc | If a Pokemon uses a Fire- or Ice-type attack against this Pokemon, that Pokemon's offen... |
| BT-2921 | a:thickfat.shortDesc | Fire-/Ice-type moves against this Pokemon deal damage with a halved offensive stat. |
| BT-2922 | a:tintedlens.shortDesc | This Pokemon's attacks that are not very effective on a target deal double damage. |
| BT-2923 | a:torrent.desc | When this Pokemon has 1/3 or less of its maximum HP, rounded down, its offensive stat i... |
| BT-2924 | a:torrent.shortDesc | At 1/3 or less of its max HP, this Pokemon's offensive stat is 1.5× with Water attacks. |
| BT-2925 | a:toughclaws.shortDesc | This Pokemon's contact moves have their power multiplied by 1.3. |
| BT-2926 | a:toxicboost.desc | While this Pokemon is poisoned, the power of its physical attacks is multiplied by 1.5. |
| BT-2927 | a:toxicboost.shortDesc | While this Pokemon is poisoned, its physical attacks have 1.5× power. |
| BT-2928 | a:toxicchain.desc | This Pokemon's attacks have a 30% chance of badly poisoning. This effect comes before a... |
| BT-2929 | a:toxicchain.shortDesc | This Pokemon's attacks have a 30% chance of badly poisoning. |
| BT-2930 | a:toxicdebris.shortDesc | If this Pokemon is hit by a physical attack, Toxic Spikes are set on the opposing side. |
| BT-2932 | a:trace.desc | On switch-in, this Pokemon copies a random opposing Pokemon's Ability. Abilities that c... |
| BT-2933 | a:trace.shortDesc | On switch-in, or when it can, this Pokemon copies a random adjacent foe's Ability. |
| BT-2934 | a:transistor.shortDesc | This Pokemon's offensive stat is multiplied by 1.3 while using an Electric-type attack. |
| BT-2935 | a:triage.shortDesc | This Pokemon's healing moves have their priority increased by 3. |
| BT-2937 | a:truant.shortDesc | This Pokemon skips every other turn instead of using a move. |
| BT-2938 | a:turboblaze.desc | This Pokemon's moves and their effects ignore certain Abilities of other Pokemon. The A... |
| BT-2939 | a:turboblaze.shortDesc | This Pokemon's moves and their effects ignore the Abilities of other Pokemon. |
| BT-2941 | a:unaware.desc | This Pokemon ignores other Pokemon's Attack, Special Attack, and accuracy stat stages w... |
| BT-2942 | a:unaware.shortDesc | This Pokemon ignores other Pokemon's stat stages when taking or doing damage. |
| BT-2943 | a:unburden.desc | If this Pokemon loses its held item for any reason, its Speed is doubled as long as it ... |
| BT-2944 | a:unburden.shortDesc | Speed is doubled on held item loss; boost is lost if it switches, gets new item/Ability. |
| BT-2945 | a:unnerve.desc | While this Pokemon is active, it prevents opposing Pokemon from using their Berries. Th... |
| BT-2946 | a:unnerve.shortDesc | While this Pokemon is active, it prevents opposing Pokemon from using their Berries. |
| BT-2948 | a:unseenfist.shortDesc | This Pokemon's contact moves ignore the target's protection, except Max Guard. |
| BT-2949 | a:vesselofruin.shortDesc | Active Pokemon without this Ability have their Special Attack multiplied by 0.75. |
| BT-2951 | a:victorystar.shortDesc | This Pokemon and its allies' moves have their accuracy multiplied by 1.1. |
| BT-2952 | a:vitalspirit.shortDesc | This Pokemon cannot fall asleep. Gaining this Ability while asleep cures it. |
| BT-2953 | a:voltabsorb.desc | This Pokemon is immune to Electric-type moves and restores 1/4 of its maximum HP, round... |
| BT-2954 | a:voltabsorb.shortDesc | This Pokemon heals 1/4 of its max HP when hit by Electric moves; Electric immunity. |
| BT-2956 | a:wanderingspirit.desc | Pokemon making contact with this Pokemon have their Ability swapped with this one. Does... |
| BT-2957 | a:wanderingspirit.shortDesc | Pokemon making contact with this Pokemon have their Ability swapped with this one. |
| BT-2958 | a:waterabsorb.desc | This Pokemon is immune to Water-type moves and restores 1/4 of its maximum HP, rounded ... |
| BT-2959 | a:waterabsorb.shortDesc | This Pokemon heals 1/4 of its max HP when hit by Water moves; Water immunity. |
| BT-2960 | a:waterbubble.desc | This Pokemon's offensive stat is doubled while using a Water-type attack. If a Pokemon ... |
| BT-2961 | a:waterbubble.shortDesc | This Pokemon's Water power is 2×; it can't be burned; Fire power against it is halved. |
| BT-2962 | a:watercompaction.shortDesc | This Pokemon's Defense is raised 2 stages after it is damaged by a Water-type move. |
| BT-2963 | a:waterveil.shortDesc | This Pokemon cannot be burned. Gaining this Ability while burned cures it. |
| BT-2964 | a:weakarmor.desc | If a physical attack hits this Pokemon, its Defense is lowered by 1 stage and its Speed... |
| BT-2965 | a:weakarmor.shortDesc | If a physical attack hits this Pokemon, Defense is lowered by 1, Speed is raised by 2. |
| BT-2966 | a:wellbakedbody.desc | This Pokemon is immune to Fire-type moves and raises its Defense by 2 stages when hit b... |
| BT-2967 | a:wellbakedbody.shortDesc | This Pokemon's Defense is raised 2 stages if hit by a Fire move; Fire immunity. |
| BT-2968 | a:whitesmoke.shortDesc | Prevents other Pokemon from lowering this Pokemon's stat stages. |
| BT-2969 | a:wimpout.desc | When this Pokemon has more than 1/2 its maximum HP and takes damage bringing it to 1/2 ... |
| BT-2970 | a:wimpout.shortDesc | This Pokemon switches out when it reaches 1/2 or less of its maximum HP. |
| BT-2971 | a:windpower.desc | This Pokemon gains the Charge effect when it takes a hit from a wind move or when Tailw... |
| BT-2972 | a:windpower.shortDesc | This Pokemon gains the Charge effect when hit by a wind move or Tailwind begins. |
| BT-2974 | a:windrider.desc | This Pokemon is immune to wind moves and raises its Attack by 1 stage when hit by a win... |
| BT-2975 | a:windrider.shortDesc | Attack raised by 1 if hit by a wind move or Tailwind begins. Wind move immunity. |
| BT-2976 | a:wonderguard.shortDesc | This Pokemon can only be damaged by supereffective moves and indirect damage. |
| BT-2977 | a:wonderskin.desc | Non-damaging moves that check accuracy have their accuracy changed to 50% when used aga... |
| BT-2978 | a:wonderskin.shortDesc | Status moves with accuracy checks are 50% accurate when used on this Pokemon. |
| BT-2979 | a:zenmode.desc | If this Pokemon is a Darmanitan or Galarian Darmanitan, it changes to Zen Mode if it ha... |
| BT-2980 | a:zenmode.shortDesc | If Darmanitan, at end of turn changes Mode to Standard if > 1/2 max HP, else Zen. |
| BT-2984 | a:zerotohero.shortDesc | If this Pokemon is a Palafin in Zero Form, switching out has it change to Hero Form. |
| BT-2986 | i:abilityshield.shortDesc | Holder's Ability cannot be changed, suppressed, or ignored by any effect. |
| BT-2987 | i:abomasite.shortDesc | If held by an Abomasnow, this item allows it to Mega Evolve in battle. |
| BT-2988 | i:absolite.shortDesc | If held by an Absol, this item allows it to Mega Evolve into Mega Absol in battle. |
| BT-2989 | i:absolitez.shortDesc | If held by an Absol, this item allows it to Mega Evolve into Mega Absol Z in battle. |
| BT-2990 | i:absorbbulb.shortDesc | Raises holder's Sp. Atk by 1 stage if hit by a Water-type attack. Single use. |
| BT-2991 | i:adamantcrystal.shortDesc | If held by a Dialga, its Steel- and Dragon-type attacks have 1.2× power. |
| BT-2992 | i:adamantorb.shortDesc | If held by a Dialga, its Steel- and Dragon-type attacks have 1.2× power. |
| BT-2993 | i:adrenalineorb.shortDesc | Raises holder's Speed by 1 stage if it gets affected by Intimidate. Single use. |
| BT-2994 | i:aerodactylite.shortDesc | If held by an Aerodactyl, this item allows it to Mega Evolve in battle. |
| BT-2995 | i:aggronite.shortDesc | If held by an Aggron, this item allows it to Mega Evolve in battle. |
| BT-2996 | i:aguavberry.shortDesc | Restores 1/3 max HP at 1/4 max HP or less; confuses if -SpD Nature. Single use. |
| BT-2998 | i:airballoon.shortDesc | Holder is immune to Ground-type attacks. Pops when holder is hit. |
| BT-3000 | i:alakazite.shortDesc | If held by an Alakazam, this item allows it to Mega Evolve in battle. |
| BT-3001 | i:aloraichiumz.shortDesc | If held by an Alolan Raichu with Thunderbolt, it can use Stoked Sparksurfer. |
| BT-3002 | i:altarianite.shortDesc | If held by an Altaria, this item allows it to Mega Evolve in battle. |
| BT-3003 | i:ampharosite.shortDesc | If held by an Ampharos, this item allows it to Mega Evolve in battle. |
| BT-3004 | i:apicotberry.shortDesc | Raises holder's Sp. Def by 1 stage when at 1/4 max HP or less. Single use. |
| BT-3005 | i:armorfossil.shortDesc | Can be revived into Shieldon. |
| BT-3006 | i:aspearberry.shortDesc | Holder is cured if it is frozen. Single use. |
| BT-3007 | i:assaultvest.shortDesc | Holder's Sp. Def is 1.5×, but it can only select damaging moves. |
| BT-3008 | i:audinite.shortDesc | If held by an Audino, this item allows it to Mega Evolve in battle. |
| BT-3009 | i:auspiciousarmor.shortDesc | Evolves Charcadet into Armarouge when used. |
| BT-3010 | i:babiriberry.shortDesc | Halves damage taken from a supereffective Steel-type attack. Single use. |
| BT-3011 | i:banettite.shortDesc | If held by a Banette, this item allows it to Mega Evolve in battle. |
| BT-3012 | i:barbaracite.shortDesc | If held by a Barbaracle, this item allows it to Mega Evolve in battle. |
| BT-3013 | i:baxcalibrite.shortDesc | If held by a Baxcalibur, this item allows it to Mega Evolve in battle. |
| BT-3014 | i:beastball.shortDesc | A special Poke Ball designed to catch Ultra Beasts. |
| BT-3015 | i:beedrillite.shortDesc | If held by a Beedrill, this item allows it to Mega Evolve in battle. |
| BT-3016 | i:belueberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3017 | i:berry.shortDesc | (Gen 2) Restores 10 HP when at 1/2 max HP or less. Single use. |
| BT-3018 | i:berryjuice.shortDesc | Restores 20 HP when at 1/2 max HP or less. Single use. |
| BT-3019 | i:berrysweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3020 | i:berserkgene.shortDesc | (Gen 2) On switch-in, raises holder's Attack by 2 and confuses it. Single use. |
| BT-3021 | i:bignugget.shortDesc | A big nugget of pure gold that gives off a lustrous gleam. |
| BT-3022 | i:bigroot.shortDesc | Holder gains 1.3× HP from draining/Aqua Ring/Ingrain/Leech Seed/Strength Sap. |
| BT-3023 | i:bindingband.shortDesc | Holder's partial-trapping moves deal 1/6 max HP per turn instead of 1/8. |
| BT-3024 | i:bitterberry.shortDesc | (Gen 2) Holder is cured if it is confused. Single use. |
| BT-3025 | i:blackaugurite.shortDesc | Evolves Scyther into Kleavor when used. |
| BT-3026 | i:blackbelt.shortDesc | Holder's Fighting-type attacks have 1.2× power. |
| BT-3027 | i:blackglasses.shortDesc | Holder's Dark-type attacks have 1.2× power. |
| BT-3029 | i:blacksludge.shortDesc | Each turn, if holder is a Poison type, restores 1/16 max HP; loses 1/8 if not. |
| BT-3030 | i:blankplate.shortDesc | Changes Arceus to the Normal type when used in Legends: Arceus. |
| BT-3031 | i:blastoisinite.shortDesc | If held by a Blastoise, this item allows it to Mega Evolve in battle. |
| BT-3032 | i:blazikenite.shortDesc | If held by a Blaziken, this item allows it to Mega Evolve in battle. |
| BT-3033 | i:blueorb.shortDesc | If held by a Kyogre, this item triggers its Primal Reversion in battle. |
| BT-3034 | i:blukberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3035 | i:blunderpolicy.shortDesc | If the holder misses due to accuracy, its Speed is raised by 2 stages. Single use. |
| BT-3036 | i:boosterenergy.shortDesc | Activates the Protosynthesis or Quark Drive Abilities. Single use. |
| BT-3037 | i:bottlecap.shortDesc | Used for Hyper Training. One of a Pokemon's stats is calculated with an IV of 31. |
| BT-3038 | i:brightpowder.shortDesc | The accuracy of attacks against the holder is 0.9×. |
| BT-3039 | i:buggem.shortDesc | Holder's first successful Bug-type attack will have 1.3× power. Single use. |
| BT-3040 | i:buginiumz.shortDesc | If holder has a Bug move, this item allows it to use a Bug Z-Move. |
| BT-3041 | i:bugmemory.shortDesc | Holder's Multi-Attack is Bug type. |
| BT-3042 | i:burndrive.shortDesc | Holder's Techno Blast is Fire type. |
| BT-3043 | i:burntberry.shortDesc | (Gen 2) Holder is cured if it is frozen. Single use. |
| BT-3044 | i:cameruptite.shortDesc | If held by a Camerupt, this item allows it to Mega Evolve in battle. |
| BT-3045 | i:cellbattery.shortDesc | Raises holder's Attack by 1 if hit by an Electric-type attack. Single use. |
| BT-3046 | i:chandelurite.shortDesc | If held by a Chandelure, this item allows it to Mega Evolve in battle. |
| BT-3047 | i:charcoal.shortDesc | Holder's Fire-type attacks have 1.2× power. |
| BT-3048 | i:charizarditex.shortDesc | If held by a Charizard, this item allows it to Mega Evolve into Mega Charizard X. |
| BT-3049 | i:charizarditey.shortDesc | If held by a Charizard, this item allows it to Mega Evolve into Mega Charizard Y. |
| BT-3050 | i:chartiberry.shortDesc | Halves damage taken from a supereffective Rock-type attack. Single use. |
| BT-3051 | i:cheriberry.shortDesc | Holder cures itself if it is paralyzed. Single use. |
| BT-3052 | i:cherishball.shortDesc | A rare Poke Ball that has been crafted to commemorate an occasion. |
| BT-3053 | i:chesnaughtite.shortDesc | If held by a Chesnaught, this item allows it to Mega Evolve in battle. |
| BT-3054 | i:chestoberry.shortDesc | Holder wakes up if it is asleep. Single use. |
| BT-3055 | i:chilanberry.shortDesc | Halves damage taken from a Normal-type attack. Single use. |
| BT-3056 | i:chilldrive.shortDesc | Holder's Techno Blast is Ice type. |
| BT-3057 | i:chimechite.shortDesc | If held by a Chimecho, this item allows it to Mega Evolve in battle. |
| BT-3058 | i:chippedpot.shortDesc | Evolves Sinistea-Antique into Polteageist-Antique when used. |
| BT-3059 | i:choiceband.shortDesc | Holder's Attack is 1.5×, but it can only select the first move it executes. |
| BT-3060 | i:choicescarf.shortDesc | Holder's Speed is 1.5×, but it can only select the first move it executes. |
| BT-3061 | i:choicespecs.shortDesc | Holder's Sp. Atk is 1.5×, but it can only select the first move it executes. |
| BT-3062 | i:chopleberry.shortDesc | Halves damage taken from a supereffective Fighting-type attack. Single use. |
| BT-3063 | i:clawfossil.shortDesc | Can be revived into Anorith. |
| BT-3065 | i:clearamulet.shortDesc | Prevents other Pokemon from lowering the holder's stat stages. |
| BT-3066 | i:clefablite.shortDesc | If held by a Clefable, this item allows it to Mega Evolve in battle. |
| BT-3067 | i:cloversweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3068 | i:cobaberry.shortDesc | Halves damage taken from a supereffective Flying-type attack. Single use. |
| BT-3069 | i:colburberry.shortDesc | Halves damage taken from a supereffective Dark-type attack. Single use. |
| BT-3070 | i:cornerstonemask.shortDesc | Ogerpon-Cornerstone: 1.2× power attacks; Terastallize to gain Embody Aspect. |
| BT-3071 | i:cornnberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3072 | i:coverfossil.shortDesc | Can be revived into Tirtouga. |
| BT-3073 | i:covertcloak.desc | The holder is not affected by the secondary effect of another Pokemon's attack. Attacks... |
| BT-3074 | i:covertcloak.shortDesc | Holder is not affected by the secondary effect of another Pokemon's attack. |
| BT-3075 | i:crabominite.shortDesc | If held by a Crabominable, this item allows it to Mega Evolve in battle. |
| BT-3076 | i:crackedpot.shortDesc | Evolves Sinistea into Polteageist when used. |
| BT-3077 | i:crucibellite.shortDesc | If held by a Crucibelle, this item allows it to Mega Evolve in battle. |
| BT-3079 | i:custapberry.shortDesc | Holder moves first in its priority bracket when at 1/4 max HP or less. Single use. |
| BT-3080 | i:damprock.shortDesc | Holder's use of Rain Dance lasts 8 turns instead of 5. |
| BT-3081 | i:darkgem.shortDesc | Holder's first successful Dark-type attack will have 1.3× power. Single use. |
| BT-3082 | i:darkiniumz.shortDesc | If holder has a Dark move, this item allows it to use a Dark Z-Move. |
| BT-3083 | i:darkmemory.shortDesc | Holder's Multi-Attack is Dark type. |
| BT-3084 | i:darkranite.shortDesc | If held by a Darkrai, this item allows it to Mega Evolve in battle. |
| BT-3085 | i:dawnstone.desc | Evolves male Kirlia into Gallade and female Snorunt into Froslass when used. |
| BT-3086 | i:dawnstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3087 | i:decidiumz.shortDesc | If held by a Decidueye with Spirit Shackle, it can use Sinister Arrow Raid. |
| BT-3088 | i:deepseascale.desc | If held by a Clamperl, its Sp. Def is doubled. Evolves Clamperl into Gorebyss when traded. |
| BT-3089 | i:deepseascale.shortDesc | If held by a Clamperl, its Sp. Def is doubled. |
| BT-3090 | i:deepseatooth.desc | If held by a Clamperl, its Sp. Atk is doubled. Evolves Clamperl into Huntail when traded. |
| BT-3091 | i:deepseatooth.shortDesc | If held by a Clamperl, its Sp. Atk is doubled. |
| BT-3092 | i:delphoxite.shortDesc | If held by a Delphox, this item allows it to Mega Evolve in battle. |
| BT-3093 | i:destinyknot.shortDesc | If holder becomes infatuated, the other Pokemon also becomes infatuated. |
| BT-3094 | i:diancite.shortDesc | If held by a Diancie, this item allows it to Mega Evolve in battle. |
| BT-3095 | i:diveball.shortDesc | A Poke Ball that works especially well on Pokemon that live underwater. |
| BT-3096 | i:domefossil.shortDesc | Can be revived into Kabuto. |
| BT-3097 | i:dousedrive.shortDesc | Holder's Techno Blast is Water type. |
| BT-3098 | i:dracoplate.shortDesc | Holder's Dragon-type attacks have 1.2× power. Judgment is Dragon type. |
| BT-3099 | i:dragalgite.shortDesc | If held by a Dragalge, this item allows it to Mega Evolve in battle. |
| BT-3100 | i:dragonfang.shortDesc | Holder's Dragon-type attacks have 1.2× power. |
| BT-3101 | i:dragongem.shortDesc | Holder's first successful Dragon-type attack will have 1.3× power. Single use. |
| BT-3102 | i:dragoninite.shortDesc | If held by a Dragonite, this item allows it to Mega Evolve in battle. |
| BT-3103 | i:dragoniumz.shortDesc | If holder has a Dragon move, this item allows it to use a Dragon Z-Move. |
| BT-3104 | i:dragonmemory.shortDesc | Holder's Multi-Attack is Dragon type. |
| BT-3105 | i:dragonscale.shortDesc | Evolves Seadra into Kingdra when traded. |
| BT-3106 | i:drampanite.shortDesc | If held by a Drampa, this item allows it to Mega Evolve in battle. |
| BT-3107 | i:dreadplate.shortDesc | Holder's Dark-type attacks have 1.2× power. Judgment is Dark type. |
| BT-3108 | i:dreamball.shortDesc | A Poke Ball that makes it easier to catch wild Pokémon while they're asleep. |
| BT-3109 | i:dubiousdisc.shortDesc | Evolves Porygon2 into Porygon-Z when traded. |
| BT-3110 | i:durinberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3111 | i:duskball.shortDesc | A Poke Ball that makes it easier to catch wild Pokemon at night or in caves. |
| BT-3112 | i:duskstone.desc | Evolves Murkrow into Honchkrow, Misdreavus into Mismagius, Lampent into Chandelure, and... |
| BT-3113 | i:duskstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3114 | i:earthplate.shortDesc | Holder's Ground-type attacks have 1.2× power. Judgment is Ground type. |
| BT-3115 | i:eelektrossite.shortDesc | If held by an Eelektross, this item allows it to Mega Evolve in battle. |
| BT-3116 | i:eeviumz.shortDesc | If held by an Eevee with Last Resort, it can use Extreme Evoboost. |
| BT-3118 | i:ejectbutton.shortDesc | If holder survives a hit, it immediately switches out to a chosen ally. Single use. |
| BT-3120 | i:ejectpack.shortDesc | If the holder's stat stages are lowered, it switches to a chosen ally. Single use. |
| BT-3121 | i:electirizer.shortDesc | Evolves Electabuzz into Electivire when traded. |
| BT-3122 | i:electricgem.shortDesc | Holder's first successful Electric-type attack will have 1.3× power. Single use. |
| BT-3123 | i:electricmemory.shortDesc | Holder's Multi-Attack is Electric type. |
| BT-3124 | i:electricseed.shortDesc | If the terrain is Electric Terrain, raises holder's Defense by 1 stage. Single use. |
| BT-3125 | i:electriumz.shortDesc | If holder has an Electric move, this item allows it to use an Electric Z-Move. |
| BT-3126 | i:emboarite.shortDesc | If held by an Emboar, this item allows it to Mega Evolve in battle. |
| BT-3127 | i:enigmaberry.shortDesc | Restores 1/4 max HP after holder is hit by a supereffective move. Single use. |
| BT-3128 | i:eviolite.shortDesc | If holder's species can evolve, its Defense and Sp. Def are 1.5×. |
| BT-3129 | i:excadrite.shortDesc | If held by an Excadrill, this item allows it to Mega Evolve in battle. |
| BT-3130 | i:expertbelt.shortDesc | Holder's attacks that are super effective against the target do 1.2× damage. |
| BT-3131 | i:fairiumz.shortDesc | If holder has a Fairy move, this item allows it to use a Fairy Z-Move. |
| BT-3132 | i:fairyfeather.shortDesc | Holder's Fairy-type attacks have 1.2× power. |
| BT-3133 | i:fairygem.shortDesc | Holder's first successful Fairy-type attack will have 1.3× power. Single use. |
| BT-3134 | i:fairymemory.shortDesc | Holder's Multi-Attack is Fairy type. |
| BT-3135 | i:falinksite.shortDesc | If held by a Falinks, this item allows it to Mega Evolve in battle. |
| BT-3136 | i:fastball.shortDesc | A Poke Ball that makes it easier to catch Pokemon which are quick to run away. |
| BT-3137 | i:feraligite.shortDesc | If held by a Feraligatr, this item allows it to Mega Evolve in battle. |
| BT-3138 | i:fightinggem.shortDesc | Holder's first successful Fighting-type attack will have 1.3× power. Single use. |
| BT-3139 | i:fightingmemory.shortDesc | Holder's Multi-Attack is Fighting type. |
| BT-3140 | i:fightiniumz.shortDesc | If holder has a Fighting move, this item allows it to use a Fighting Z-Move. |
| BT-3141 | i:figyberry.shortDesc | Restores 1/3 max HP at 1/4 max HP or less; confuses if -Atk Nature. Single use. |
| BT-3142 | i:firegem.shortDesc | Holder's first successful Fire-type attack will have 1.3× power. Single use. |
| BT-3143 | i:firememory.shortDesc | Holder's Multi-Attack is Fire type. |
| BT-3144 | i:firestone.desc | Evolves Vulpix into Ninetales, Growlithe into Arcanine, Eevee into Flareon, and Pansear... |
| BT-3145 | i:firestone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3146 | i:firiumz.shortDesc | If holder has a Fire move, this item allows it to use a Fire Z-Move. |
| BT-3147 | i:fistplate.shortDesc | Holder's Fighting-type attacks have 1.2× power. Judgment is Fighting type. |
| BT-3148 | i:flameorb.shortDesc | At the end of every turn, this item attempts to burn the holder. |
| BT-3149 | i:flameplate.shortDesc | Holder's Fire-type attacks have 1.2× power. Judgment is Fire type. |
| BT-3150 | i:floatstone.shortDesc | Holder's weight is halved. |
| BT-3151 | i:floettite.shortDesc | If held by an Eternal Flower Floette, this item allows it to Mega Evolve in battle. |
| BT-3152 | i:flowersweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3153 | i:flyinggem.shortDesc | Holder's first successful Flying-type attack will have 1.3× power. Single use. |
| BT-3154 | i:flyingmemory.shortDesc | Holder's Multi-Attack is Flying type. |
| BT-3155 | i:flyiniumz.shortDesc | If holder has a Flying move, this item allows it to use a Flying Z-Move. |
| BT-3157 | i:focusband.shortDesc | Holder has a 10% chance to survive an attack that would KO it with 1 HP. |
| BT-3159 | i:focussash.shortDesc | If holder's HP is full, will survive an attack that would KO it with 1 HP. Single use. |
| BT-3160 | i:fossilizedbird.shortDesc | Can revive into Dracozolt with Fossilized Drake or Arctozolt with Fossilized Dino. |
| BT-3161 | i:fossilizeddino.shortDesc | Can revive into Arctovish with Fossilized Fish or Arctozolt with Fossilized Bird. |
| BT-3162 | i:fossilizeddrake.shortDesc | Can revive into Dracozolt with Fossilized Bird or Dracovish with Fossilized Fish. |
| BT-3163 | i:fossilizedfish.shortDesc | Can revive into Dracovish with Fossilized Drake or Arctovish with Fossilized Dino. |
| BT-3164 | i:friendball.shortDesc | A Poke Ball that makes caught Pokemon more friendly. |
| BT-3165 | i:froslassite.shortDesc | If held by a Froslass, this item allows it to Mega Evolve in battle. |
| BT-3166 | i:fullincense.shortDesc | Holder moves last in its priority bracket. |
| BT-3167 | i:galaricacuff.shortDesc | Evolves Galarian Slowpoke into Galarian Slowbro when used. |
| BT-3168 | i:galaricawreath.shortDesc | Evolves Galarian Slowpoke into Galarian Slowking when used. |
| BT-3169 | i:galladite.shortDesc | If held by a Gallade, this item allows it to Mega Evolve in battle. |
| BT-3170 | i:ganlonberry.shortDesc | Raises holder's Defense by 1 stage when at 1/4 max HP or less. Single use. |
| BT-3171 | i:garchompite.shortDesc | If held by a Garchomp, this item allows it to Mega Evolve into Mega Garchomp. |
| BT-3172 | i:garchompitez.shortDesc | If held by a Garchomp, this item allows it to Mega Evolve into Mega Garchomp Z. |
| BT-3173 | i:gardevoirite.shortDesc | If held by a Gardevoir, this item allows it to Mega Evolve in battle. |
| BT-3174 | i:gengarite.shortDesc | If held by a Gengar, this item allows it to Mega Evolve in battle. |
| BT-3175 | i:ghostgem.shortDesc | Holder's first successful Ghost-type attack will have 1.3× power. Single use. |
| BT-3176 | i:ghostiumz.shortDesc | If holder has a Ghost move, this item allows it to use a Ghost Z-Move. |
| BT-3177 | i:ghostmemory.shortDesc | Holder's Multi-Attack is Ghost type. |
| BT-3178 | i:glalitite.shortDesc | If held by a Glalie, this item allows it to Mega Evolve in battle. |
| BT-3179 | i:glimmoranite.shortDesc | If held by a Glimmora, this item allows it to Mega Evolve in battle. |
| BT-3180 | i:goldberry.shortDesc | (Gen 2) Restores 30 HP when at 1/2 max HP or less. Single use. |
| BT-3181 | i:goldbottlecap.shortDesc | Used for Hyper Training. All of a Pokemon's stats are calculated with an IV of 31. |
| BT-3182 | i:golisopite.shortDesc | If held by a Golisopod, this item allows it to Mega Evolve in battle. |
| BT-3183 | i:golurkite.shortDesc | If held by a Golurk, this item allows it to Mega Evolve in battle. |
| BT-3184 | i:grassgem.shortDesc | Holder's first successful Grass-type attack will have 1.3× power. Single use. |
| BT-3185 | i:grassiumz.shortDesc | If holder has a Grass move, this item allows it to use a Grass Z-Move. |
| BT-3186 | i:grassmemory.shortDesc | Holder's Multi-Attack is Grass type. |
| BT-3187 | i:grassyseed.shortDesc | If the terrain is Grassy Terrain, raises holder's Defense by 1 stage. Single use. |
| BT-3188 | i:greatball.shortDesc | A high-performance Ball that provides a higher catch rate than a Poke Ball. |
| BT-3189 | i:greninjite.shortDesc | If held by a Greninja, this item allows it to Mega Evolve in battle. |
| BT-3190 | i:grepaberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3191 | i:gripclaw.shortDesc | Holder's partial-trapping moves always last 7 turns. |
| BT-3192 | i:griseouscore.shortDesc | If held by a Giratina, its Ghost- and Dragon-type attacks have 1.2× power. |
| BT-3193 | i:griseousorb.shortDesc | If held by a Giratina, its Ghost- and Dragon-type attacks have 1.2× power. |
| BT-3194 | i:groundgem.shortDesc | Holder's first successful Ground-type attack will have 1.3× power. Single use. |
| BT-3195 | i:groundiumz.shortDesc | If holder has a Ground move, this item allows it to use a Ground Z-Move. |
| BT-3196 | i:groundmemory.shortDesc | Holder's Multi-Attack is Ground type. |
| BT-3197 | i:gyaradosite.shortDesc | If held by a Gyarados, this item allows it to Mega Evolve in battle. |
| BT-3198 | i:habanberry.shortDesc | Halves damage taken from a supereffective Dragon-type attack. Single use. |
| BT-3199 | i:hardstone.shortDesc | Holder's Rock-type attacks have 1.2× power. |
| BT-3200 | i:hawluchanite.shortDesc | If held by a Hawlucha, this item allows it to Mega Evolve in battle. |
| BT-3201 | i:healball.shortDesc | A remedial Poke Ball that restores the caught Pokemon's HP and status problem. |
| BT-3202 | i:hearthflamemask.shortDesc | Ogerpon-Hearthflame: 1.2× power attacks; Terastallize to gain Embody Aspect. |
| BT-3203 | i:heatranite.shortDesc | If held by a Heatran, this item allows it to Mega Evolve in battle. |
| BT-3204 | i:heatrock.shortDesc | Holder's use of Sunny Day lasts 8 turns instead of 5. |
| BT-3205 | i:heavyball.shortDesc | A Poke Ball for catching very heavy Pokemon. |
| BT-3206 | i:heavydutyboots.shortDesc | When switching in, the holder is unaffected by hazards on its side of the field. |
| BT-3207 | i:helixfossil.shortDesc | Can be revived into Omanyte. |
| BT-3208 | i:heracronite.shortDesc | If held by a Heracross, this item allows it to Mega Evolve in battle. |
| BT-3209 | i:hondewberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3210 | i:houndoominite.shortDesc | If held by a Houndoom, this item allows it to Mega Evolve in battle. |
| BT-3211 | i:iapapaberry.shortDesc | Restores 1/3 max HP at 1/4 max HP or less; confuses if -Def Nature. Single use. |
| BT-3212 | i:iceberry.shortDesc | (Gen 2) Holder is cured if it is burned. Single use. |
| BT-3213 | i:icegem.shortDesc | Holder's first successful Ice-type attack will have 1.3× power. Single use. |
| BT-3214 | i:icememory.shortDesc | Holder's Multi-Attack is Ice type. |
| BT-3215 | i:icestone.desc | Evolves Alolan Sandshrew into Alolan Sandslash, Alolan Vulpix into Alolan Ninetales, Ee... |
| BT-3216 | i:icestone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3217 | i:icicleplate.shortDesc | Holder's Ice-type attacks have 1.2× power. Judgment is Ice type. |
| BT-3218 | i:iciumz.shortDesc | If holder has an Ice move, this item allows it to use an Ice Z-Move. |
| BT-3219 | i:icyrock.shortDesc | Holder's use of Snowscape lasts 8 turns instead of 5. |
| BT-3220 | i:inciniumz.shortDesc | If held by an Incineroar with Darkest Lariat, it can use Malicious Moonsault. |
| BT-3221 | i:insectplate.shortDesc | Holder's Bug-type attacks have 1.2× power. Judgment is Bug type. |
| BT-3222 | i:ironball.shortDesc | Holder is grounded, Speed halved. If Flying type, takes neutral Ground damage. |
| BT-3223 | i:ironplate.shortDesc | Holder's Steel-type attacks have 1.2× power. Judgment is Steel type. |
| BT-3224 | i:jabocaberry.shortDesc | If holder is hit by a physical move, attacker loses 1/8 of its max HP. Single use. |
| BT-3225 | i:jawfossil.shortDesc | Can be revived into Tyrunt. |
| BT-3226 | i:kangaskhanite.shortDesc | If held by a Kangaskhan, this item allows it to Mega Evolve in battle. |
| BT-3227 | i:kasibberry.shortDesc | Halves damage taken from a supereffective Ghost-type attack. Single use. |
| BT-3228 | i:kebiaberry.shortDesc | Halves damage taken from a supereffective Poison-type attack. Single use. |
| BT-3229 | i:keeberry.shortDesc | Raises holder's Defense by 1 stage after it is hit by a physical attack. Single use. |
| BT-3230 | i:kelpsyberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3231 | i:kingsrock.desc | Holder's attacks without a chance to make the target flinch gain a 10% chance to make t... |
| BT-3232 | i:kingsrock.shortDesc | Holder's attacks without a chance to flinch gain a 10% chance to flinch. |
| BT-3233 | i:kommoniumz.shortDesc | If held by a Kommo-o with Clanging Scales, it can use Clangorous Soulblaze. |
| BT-3234 | i:laggingtail.shortDesc | Holder moves last in its priority bracket. |
| BT-3235 | i:lansatberry.shortDesc | Holder gains the Focus Energy effect when at 1/4 max HP or less. Single use. |
| BT-3236 | i:latiasite.shortDesc | If held by a Latias, this item allows it to Mega Evolve in battle. |
| BT-3237 | i:latiosite.shortDesc | If held by a Latios, this item allows it to Mega Evolve in battle. |
| BT-3238 | i:laxincense.shortDesc | The accuracy of attacks against the holder is 0.9×. |
| BT-3239 | i:leafstone.desc | Evolves Gloom into Vileplume, Weepinbell into Victreebel, Exeggcute into Exeggutor or A... |
| BT-3240 | i:leafstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3241 | i:leek.shortDesc | If held by a Farfetch’d or Sirfetch’d, its critical hit ratio is raised by 2 stages. |
| BT-3243 | i:leftovers.shortDesc | At the end of every turn, holder restores 1/16 of its max HP. |
| BT-3244 | i:legendplate.desc | When used on Arceus in Legends: Arceus, changes it to the Normal type. When Arceus uses... |
| BT-3245 | i:legendplate.shortDesc | In Legends: Arceus, Arceus's Judgment changes type to be supereffective. |
| BT-3247 | i:leppaberry.shortDesc | Restores 10 PP to the first of the holder's moves to reach 0 PP. Single use. |
| BT-3248 | i:levelball.shortDesc | A Poke Ball for catching Pokemon that are a lower level than your own. |
| BT-3249 | i:liechiberry.shortDesc | Raises holder's Attack by 1 stage when at 1/4 max HP or less. Single use. |
| BT-3251 | i:lifeorb.shortDesc | Holder's attacks do 1.3× damage, and it loses 1/10 its max HP after the attack. |
| BT-3252 | i:lightball.shortDesc | If held by a Pikachu, its Attack and Sp. Atk are doubled. |
| BT-3253 | i:lightclay.shortDesc | Holder's use of Aurora Veil, Light Screen, or Reflect lasts 8 turns instead of 5. |
| BT-3254 | i:loadeddice.desc | The holder's moves that normally hit 2 to 5 times instead hit 4 or 5 times. If the firs... |
| BT-3255 | i:loadeddice.shortDesc | Holder's moves that hit 2-5 times hit 4-5 times; Population Bomb hits 4-10 times. |
| BT-3256 | i:lopunnite.shortDesc | If held by a Lopunny, this item allows it to Mega Evolve in battle. |
| BT-3257 | i:loveball.shortDesc | Poke Ball for catching Pokemon that are the opposite gender of your Pokemon. |
| BT-3258 | i:lovesweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3259 | i:lucarionite.shortDesc | If held by a Lucario, this item allows it to Mega Evolve into Mega Lucario in battle. |
| BT-3260 | i:lucarionitez.shortDesc | If held by a Lucario, this item allows it to Mega Evolve into Mega Lucario Z in battle. |
| BT-3261 | i:luckypunch.shortDesc | If held by a Chansey, its critical hit ratio is raised by 2 stages. |
| BT-3262 | i:lumberry.shortDesc | Holder cures itself if it has a non-volatile status or is confused. Single use. |
| BT-3263 | i:luminousmoss.shortDesc | Raises holder's Sp. Def by 1 stage if hit by a Water-type attack. Single use. |
| BT-3264 | i:lunaliumz.shortDesc | Lunala or Dawn Wings Necrozma with Moongeist Beam can use a special Z-Move. |
| BT-3265 | i:lureball.shortDesc | A Poke Ball for catching Pokemon hooked by a Rod when fishing. |
| BT-3266 | i:lustrousglobe.shortDesc | If held by a Palkia, its Water- and Dragon-type attacks have 1.2× power. |
| BT-3267 | i:lustrousorb.shortDesc | If held by a Palkia, its Water- and Dragon-type attacks have 1.2× power. |
| BT-3268 | i:luxuryball.shortDesc | A comfortable Poke Ball that makes a caught wild Pokemon quickly grow friendly. |
| BT-3269 | i:lycaniumz.shortDesc | If held by a Lycanroc forme with Stone Edge, it can use Splintered Stormshards. |
| BT-3270 | i:machobrace.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3271 | i:magearnite.shortDesc | If held by a Magearna, this item allows it to Mega Evolve in battle. |
| BT-3272 | i:magmarizer.shortDesc | Evolves Magmar into Magmortar when traded. |
| BT-3273 | i:magnet.shortDesc | Holder's Electric-type attacks have 1.2× power. |
| BT-3274 | i:magoberry.shortDesc | Restores 1/3 max HP at 1/4 max HP or less; confuses if -Spe Nature. Single use. |
| BT-3275 | i:magostberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3276 | i:mail.shortDesc | Cannot be given to or taken from a Pokemon, except by Covet/Knock Off/Thief. |
| BT-3277 | i:malamarite.shortDesc | If held by a Malamar, this item allows it to Mega Evolve in battle. |
| BT-3278 | i:maliciousarmor.shortDesc | Evolves Charcadet into Ceruledge when used. |
| BT-3279 | i:manectite.shortDesc | If held by a Manectric, this item allows it to Mega Evolve in battle. |
| BT-3280 | i:marangaberry.shortDesc | Raises holder's Sp. Def by 1 stage after it is hit by a special attack. Single use. |
| BT-3281 | i:marshadiumz.shortDesc | If held by Marshadow with Spectral Thief, it can use Soul-Stealing 7-Star Strike. |
| BT-3282 | i:masterball.shortDesc | The best Ball with the ultimate performance. It will catch any wild Pokemon. |
| BT-3283 | i:masterpieceteacup.shortDesc | Evolves Poltchageist-Artisan into Sinistcha-Masterpiece when used. |
| BT-3284 | i:mawilite.shortDesc | If held by a Mawile, this item allows it to Mega Evolve in battle. |
| BT-3285 | i:meadowplate.shortDesc | Holder's Grass-type attacks have 1.2× power. Judgment is Grass type. |
| BT-3286 | i:medichamite.shortDesc | If held by a Medicham, this item allows it to Mega Evolve in battle. |
| BT-3287 | i:meganiumite.shortDesc | If held by a Meganium, this item allows it to Mega Evolve in battle. |
| BT-3288 | i:mentalherb.shortDesc | Cures holder of Attract, Disable, Encore, Heal Block, Taunt, Torment. Single use. |
| BT-3289 | i:meowsticite.shortDesc | If held by a Meowstic, this item allows it to Mega Evolve in battle. |
| BT-3290 | i:metagrossite.shortDesc | If held by a Metagross, this item allows it to Mega Evolve in battle. |
| BT-3291 | i:metalalloy.shortDesc | Evolves Duraludon into Archaludon when used. |
| BT-3292 | i:metalcoat.desc | Holder's Steel-type attacks have 1.2× power. Evolves Onix into Steelix and Scyther into... |
| BT-3293 | i:metalcoat.shortDesc | Holder's Steel-type attacks have 1.2× power. |
| BT-3294 | i:metalpowder.shortDesc | If held by a Ditto that hasn't Transformed, its Defense is doubled. |
| BT-3295 | i:metronome.shortDesc | Damage of moves used on consecutive turns is increased. Max 2× after 5 turns. |
| BT-3296 | i:mewniumz.shortDesc | If held by a Mew with Psychic, it can use Genesis Supernova. |
| BT-3297 | i:mewtwonitex.shortDesc | If held by a Mewtwo, this item allows it to Mega Evolve into Mega Mewtwo X in battle. |
| BT-3298 | i:mewtwonitey.shortDesc | If held by a Mewtwo, this item allows it to Mega Evolve into Mega Mewtwo Y in battle. |
| BT-3299 | i:micleberry.shortDesc | Holder's next move has 1.2× accuracy when at 1/4 max HP or less. Single use. |
| BT-3300 | i:mimikiumz.shortDesc | If held by a Mimikyu with Play Rough, it can use Let's Snuggle Forever. |
| BT-3301 | i:mindplate.shortDesc | Holder's Psychic-type attacks have 1.2× power. Judgment is Psychic type. |
| BT-3302 | i:mintberry.shortDesc | (Gen 2) Holder wakes up if it is asleep. Single use. |
| BT-3303 | i:miracleberry.shortDesc | (Gen 2) Holder cures itself if it is confused or has a status condition. Single use. |
| BT-3304 | i:miracleseed.shortDesc | Holder's Grass-type attacks have 1.2× power. |
| BT-3306 | i:mirrorherb.shortDesc | When an opposing Pokemon raises a stat stage, the holder copies it. Single use. |
| BT-3307 | i:mistyseed.shortDesc | If the terrain is Misty Terrain, raises holder's Sp. Def by 1 stage. Single use. |
| BT-3308 | i:moonball.shortDesc | A Poke Ball for catching Pokemon that evolve using the Moon Stone. |
| BT-3309 | i:moonstone.desc | Evolves Nidorina into Nidoqueen, Nidorino into Nidoking, Clefairy into Clefable, Jiggly... |
| BT-3310 | i:moonstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3311 | i:muscleband.shortDesc | Holder's physical attacks have 1.1× power. |
| BT-3313 | i:mysteryberry.shortDesc | (Gen 2) Restores 5 PP to the first of the holder's moves to reach 0 PP. Single use. |
| BT-3314 | i:mysticwater.shortDesc | Holder's Water-type attacks have 1.2× power. |
| BT-3315 | i:nanabberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3316 | i:nestball.shortDesc | A Poke Ball that works especially well on weaker Pokemon in the wild. |
| BT-3317 | i:netball.shortDesc | A Poke Ball that works especially well on Water- and Bug-type Pokemon. |
| BT-3318 | i:nevermeltice.shortDesc | Holder's Ice-type attacks have 1.2× power. |
| BT-3319 | i:nomelberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3320 | i:normalgem.shortDesc | Holder's first successful Normal-type attack will have 1.3× power. Single use. |
| BT-3321 | i:normaliumz.shortDesc | If holder has a Normal move, this item allows it to use a Normal Z-Move. |
| BT-3322 | i:occaberry.shortDesc | Halves damage taken from a supereffective Fire-type attack. Single use. |
| BT-3323 | i:oddincense.shortDesc | Holder's Psychic-type attacks have 1.2× power. |
| BT-3324 | i:oldamber.shortDesc | Can be revived into Aerodactyl. |
| BT-3325 | i:oranberry.shortDesc | Restores 10 HP when at 1/2 max HP or less. Single use. |
| BT-3326 | i:ovalstone.shortDesc | Evolves Happiny into Chansey when held and leveled up during the day. |
| BT-3327 | i:pamtreberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3328 | i:parkball.shortDesc | A special Poke Ball for the Pal Park. |
| BT-3329 | i:passhoberry.shortDesc | Halves damage taken from a supereffective Water-type attack. Single use. |
| BT-3330 | i:payapaberry.shortDesc | Halves damage taken from a supereffective Psychic-type attack. Single use. |
| BT-3331 | i:peatblock.shortDesc | Evolves Ursaring into Ursaluna when used during a full moon. |
| BT-3332 | i:pechaberry.shortDesc | Holder is cured if it is poisoned. Single use. |
| BT-3333 | i:persimberry.shortDesc | Holder is cured if it is confused. Single use. |
| BT-3334 | i:petayaberry.shortDesc | Raises holder's Sp. Atk by 1 stage when at 1/4 max HP or less. Single use. |
| BT-3335 | i:pidgeotite.shortDesc | If held by a Pidgeot, this item allows it to Mega Evolve in battle. |
| BT-3336 | i:pikaniumz.shortDesc | If held by a Pikachu with Volt Tackle, it can use Catastropika. |
| BT-3337 | i:pikashuniumz.shortDesc | If held by cap Pikachu with Thunderbolt, it can use 10,000,000 Volt Thunderbolt. |
| BT-3338 | i:pinapberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3339 | i:pinkbow.shortDesc | (Gen 2) Holder's Normal-type attacks have 1.1× power. |
| BT-3340 | i:pinsirite.shortDesc | If held by a Pinsir, this item allows it to Mega Evolve in battle. |
| BT-3341 | i:pixieplate.shortDesc | Holder's Fairy-type attacks have 1.2× power. Judgment is Fairy type. |
| BT-3342 | i:plumefossil.shortDesc | Can be revived into Archen. |
| BT-3343 | i:poisonbarb.shortDesc | Holder's Poison-type attacks have 1.2× power. |
| BT-3344 | i:poisongem.shortDesc | Holder's first successful Poison-type attack will have 1.3× power. Single use. |
| BT-3345 | i:poisoniumz.shortDesc | If holder has a Poison move, this item allows it to use a Poison Z-Move. |
| BT-3346 | i:poisonmemory.shortDesc | Holder's Multi-Attack is Poison type. |
| BT-3347 | i:pokeball.shortDesc | A device for catching wild Pokemon. It is designed as a capsule system. |
| BT-3348 | i:polkadotbow.shortDesc | (Gen 2) Holder's Normal-type attacks have 1.1× power. |
| BT-3349 | i:pomegberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3350 | i:poweranklet.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3351 | i:powerband.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3352 | i:powerbelt.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3353 | i:powerbracer.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3355 | i:powerherb.shortDesc | Holder's two-turn moves complete in one turn (except Sky Drop). Single use. |
| BT-3356 | i:powerlens.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3357 | i:powerweight.shortDesc | Holder's Speed is halved. The Klutz Ability does not ignore this effect. |
| BT-3358 | i:premierball.shortDesc | A rare Poke Ball that has been crafted to commemorate an event. |
| BT-3359 | i:prettyfeather.shortDesc | Though this feather is beautiful, it's just a regular feather and has no effect. |
| BT-3360 | i:primariumz.shortDesc | If held by a Primarina with Sparkling Aria, it can use Oceanic Operetta. |
| BT-3361 | i:prismscale.shortDesc | Evolves Feebas into Milotic when traded. |
| BT-3363 | i:protectivepads.shortDesc | Holder's moves are protected from adverse contact effects, except Pickpocket. |
| BT-3364 | i:protector.shortDesc | Evolves Rhydon into Rhyperior when traded. |
| BT-3365 | i:przcureberry.shortDesc | (Gen 2) Holder cures itself if it is paralyzed. Single use. |
| BT-3366 | i:psncureberry.shortDesc | (Gen 2) Holder is cured if it is poisoned. Single use. |
| BT-3367 | i:psychicgem.shortDesc | Holder's first successful Psychic-type attack will have 1.3× power. Single use. |
| BT-3368 | i:psychicmemory.shortDesc | Holder's Multi-Attack is Psychic type. |
| BT-3369 | i:psychicseed.shortDesc | If the terrain is Psychic Terrain, raises holder's Sp. Def by 1 stage. Single use. |
| BT-3370 | i:psychiumz.shortDesc | If holder has a Psychic move, this item allows it to use a Psychic Z-Move. |
| BT-3371 | i:punchingglove.shortDesc | Holder's punch-based attacks have 1.1× power and do not make contact. |
| BT-3372 | i:pyroarite.shortDesc | If held by a Pyroar, this item allows it to Mega Evolve in battle. |
| BT-3373 | i:qualotberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3374 | i:quickball.shortDesc | A Poke Ball that provides a better catch rate at the start of a wild encounter. |
| BT-3376 | i:quickclaw.shortDesc | Each turn, holder has a 20% chance to move first in its priority bracket. |
| BT-3377 | i:quickpowder.shortDesc | If held by a Ditto that hasn't Transformed, its Speed is doubled. |
| BT-3378 | i:rabutaberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3379 | i:raichunitex.shortDesc | If held by a Raichu, this item allows it to Mega Evolve into Mega Raichu X in battle. |
| BT-3380 | i:raichunitey.shortDesc | If held by a Raichu, this item allows it to Mega Evolve into Mega Raichu Y in battle. |
| BT-3381 | i:rarebone.shortDesc | No competitive use other than when used with Fling. |
| BT-3382 | i:rawstberry.shortDesc | Holder is cured if it is burned. Single use. |
| BT-3383 | i:razorclaw.desc | Holder's critical hit ratio is raised by 1 stage. Evolves Sneasel into Weavile when hel... |
| BT-3384 | i:razorclaw.shortDesc | Holder's critical hit ratio is raised by 1 stage. |
| BT-3385 | i:razorfang.desc | Holder's attacks without a chance to make the target flinch gain a 10% chance to make t... |
| BT-3386 | i:razorfang.shortDesc | Holder's attacks without a chance to flinch gain a 10% chance to flinch. |
| BT-3387 | i:razzberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3388 | i:reapercloth.shortDesc | Evolves Dusclops into Dusknoir when traded. |
| BT-3390 | i:redcard.shortDesc | If holder survives a hit, attacker is forced to switch to a random ally. Single use. |
| BT-3391 | i:redorb.shortDesc | If held by a Groudon, this item triggers its Primal Reversion in battle. |
| BT-3392 | i:repeatball.shortDesc | A Poke Ball that works well on Pokemon species that were previously caught. |
| BT-3393 | i:ribbonsweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3394 | i:rindoberry.shortDesc | Halves damage taken from a supereffective Grass-type attack. Single use. |
| BT-3395 | i:ringtarget.shortDesc | The holder's type immunities granted solely by its typing are negated. |
| BT-3396 | i:rockgem.shortDesc | Holder's first successful Rock-type attack will have 1.3× power. Single use. |
| BT-3397 | i:rockincense.shortDesc | Holder's Rock-type attacks have 1.2× power. |
| BT-3398 | i:rockiumz.shortDesc | If holder has a Rock move, this item allows it to use a Rock Z-Move. |
| BT-3399 | i:rockmemory.shortDesc | Holder's Multi-Attack is Rock type. |
| BT-3401 | i:rockyhelmet.shortDesc | If holder is hit by a contact move, the attacker loses 1/6 of its max HP. |
| BT-3402 | i:roomservice.shortDesc | If Trick Room is active, the holder's Speed is lowered by 1 stage. Single use. |
| BT-3403 | i:rootfossil.shortDesc | Can be revived into Lileep. |
| BT-3404 | i:roseincense.shortDesc | Holder's Grass-type attacks have 1.2× power. |
| BT-3405 | i:roseliberry.shortDesc | Halves damage taken from a supereffective Fairy-type attack. Single use. |
| BT-3406 | i:rowapberry.shortDesc | If holder is hit by a special move, attacker loses 1/8 of its max HP. Single use. |
| BT-3407 | i:rustedshield.shortDesc | If held by a Zamazenta, this item changes its forme to Crowned Shield. |
| BT-3408 | i:rustedsword.shortDesc | If held by a Zacian, this item changes its forme to Crowned Sword. |
| BT-3409 | i:sablenite.shortDesc | If held by a Sableye, this item allows it to Mega Evolve in battle. |
| BT-3410 | i:sachet.shortDesc | Evolves Spritzee into Aromatisse when traded. |
| BT-3411 | i:safariball.shortDesc | A special Poke Ball that is used only in the Safari Zone and Great Marsh. |
| BT-3413 | i:safetygoggles.shortDesc | Holder is immune to powder moves and damage from Sandstorm or Hail. |
| BT-3414 | i:sailfossil.shortDesc | Can be revived into Amaura. |
| BT-3415 | i:salacberry.shortDesc | Raises holder's Speed by 1 stage when at 1/4 max HP or less. Single use. |
| BT-3416 | i:salamencite.shortDesc | If held by a Salamence, this item allows it to Mega Evolve in battle. |
| BT-3417 | i:sceptilite.shortDesc | If held by a Sceptile, this item allows it to Mega Evolve in battle. |
| BT-3418 | i:scizorite.shortDesc | If held by a Scizor, this item allows it to Mega Evolve in battle. |
| BT-3419 | i:scolipite.shortDesc | If held by a Scolipede, this item allows it to Mega Evolve in battle. |
| BT-3420 | i:scopelens.shortDesc | Holder's critical hit ratio is raised by 1 stage. |
| BT-3421 | i:scovillainite.shortDesc | If held by a Scovillain, this item allows it to Mega Evolve in battle. |
| BT-3422 | i:scraftinite.shortDesc | If held by a Scrafty, this item allows it to Mega Evolve in battle. |
| BT-3423 | i:seaincense.shortDesc | Holder's Water-type attacks have 1.2× power. |
| BT-3424 | i:sharpbeak.shortDesc | Holder's Flying-type attacks have 1.2× power. |
| BT-3425 | i:sharpedonite.shortDesc | If held by a Sharpedo, this item allows it to Mega Evolve in battle. |
| BT-3426 | i:shedshell.shortDesc | Holder cannot be prevented from choosing to switch out by any effect. |
| BT-3428 | i:shellbell.shortDesc | After an attack, holder gains 1/8 of the damage in HP dealt to other Pokemon. |
| BT-3429 | i:shinystone.desc | Evolves Togetic into Togekiss, Roselia into Roserade, Minccino into Cinccino, and Floet... |
| BT-3430 | i:shinystone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3431 | i:shockdrive.shortDesc | Holder's Techno Blast is Electric type. |
| BT-3432 | i:shucaberry.shortDesc | Halves damage taken from a supereffective Ground-type attack. Single use. |
| BT-3433 | i:silkscarf.shortDesc | Holder's Normal-type attacks have 1.2× power. |
| BT-3434 | i:silverpowder.shortDesc | Holder's Bug-type attacks have 1.2× power. |
| BT-3435 | i:sitrusberry.shortDesc | Restores 1/4 max HP when at 1/2 max HP or less. Single use. |
| BT-3436 | i:skarmorite.shortDesc | If held by a Skarmory, this item allows it to Mega Evolve in battle. |
| BT-3437 | i:skullfossil.shortDesc | Can be revived into Cranidos. |
| BT-3438 | i:skyplate.shortDesc | Holder's Flying-type attacks have 1.2× power. Judgment is Flying type. |
| BT-3439 | i:slowbronite.shortDesc | If held by a Slowbro, this item allows it to Mega Evolve in battle. |
| BT-3440 | i:smoothrock.shortDesc | Holder's use of Sandstorm lasts 8 turns instead of 5. |
| BT-3441 | i:snorliumz.shortDesc | If held by a Snorlax with Giga Impact, it can use Pulverizing Pancake. |
| BT-3442 | i:snowball.shortDesc | Raises holder's Attack by 1 if hit by an Ice-type attack. Single use. |
| BT-3443 | i:softsand.shortDesc | Holder's Ground-type attacks have 1.2× power. |
| BT-3444 | i:solganiumz.shortDesc | Solgaleo or Dusk Mane Necrozma with Sunsteel Strike can use a special Z-Move. |
| BT-3445 | i:souldew.shortDesc | If held by a Latias/Latios, its Dragon- and Psychic-type moves have 1.2× power. |
| BT-3446 | i:spelltag.shortDesc | Holder's Ghost-type attacks have 1.2× power. |
| BT-3447 | i:spelonberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3448 | i:splashplate.shortDesc | Holder's Water-type attacks have 1.2× power. Judgment is Water type. |
| BT-3449 | i:spookyplate.shortDesc | Holder's Ghost-type attacks have 1.2× power. Judgment is Ghost type. |
| BT-3450 | i:sportball.shortDesc | A special Poke Ball for the Bug-Catching Contest. |
| BT-3451 | i:staraptite.shortDesc | If held by a Staraptor, this item allows it to Mega Evolve in battle. |
| BT-3452 | i:starfberry.shortDesc | Raises a random stat by 2 when at 1/4 max HP or less (not acc/eva). Single use. |
| BT-3453 | i:starminite.shortDesc | If held by a Starmie, this item allows it to Mega Evolve in battle. |
| BT-3454 | i:starsweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3455 | i:steelgem.shortDesc | Holder's first successful Steel-type attack will have 1.3× power. Single use. |
| BT-3456 | i:steeliumz.shortDesc | If holder has a Steel move, this item allows it to use a Steel Z-Move. |
| BT-3457 | i:steelixite.shortDesc | If held by a Steelix, this item allows it to Mega Evolve in battle. |
| BT-3458 | i:steelmemory.shortDesc | Holder's Multi-Attack is Steel type. |
| BT-3459 | i:stick.shortDesc | If held by a Farfetch’d, its critical hit ratio is raised by 2 stages. |
| BT-3460 | i:stickybarb.shortDesc | Each turn, holder loses 1/8 max HP. An attacker making contact can receive it. |
| BT-3461 | i:stoneplate.shortDesc | Holder's Rock-type attacks have 1.2× power. Judgment is Rock type. |
| BT-3462 | i:strangeball.shortDesc | Placeholder if caught in Poke Ball not in current game. |
| BT-3463 | i:strawberrysweet.shortDesc | Evolves Milcery into Alcremie when held and spun around. |
| BT-3464 | i:sunstone.desc | Evolves Gloom into Bellossom, Sunkern into Sunflora, Cottonee into Whimsicott, Petilil ... |
| BT-3465 | i:sunstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3466 | i:swampertite.shortDesc | If held by a Swampert, this item allows it to Mega Evolve in battle. |
| BT-3467 | i:sweetapple.shortDesc | Evolves Applin into Appletun when used. |
| BT-3468 | i:syrupyapple.shortDesc | Evolves Applin into Dipplin when used. |
| BT-3469 | i:tamatoberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3470 | i:tangaberry.shortDesc | Halves damage taken from a supereffective Bug-type attack. Single use. |
| BT-3471 | i:tapuniumz.shortDesc | If held by a Tapu with Nature's Madness, it can use Guardian of Alola. |
| BT-3472 | i:tartapple.shortDesc | Evolves Applin into Flapple when used. |
| BT-3473 | i:tatsugirinite.shortDesc | If held by a Tatsugiri, this item allows it to Mega Evolve in battle. |
| BT-3474 | i:terrainextender.shortDesc | Holder's use of Electric/Grassy/Misty/Psychic Terrain lasts 8 turns instead of 5. |
| BT-3475 | i:thickclub.shortDesc | If held by a Cubone or a Marowak, its Attack is doubled. |
| BT-3476 | i:throatspray.shortDesc | Raises holder's Special Attack by 1 stage after it uses a sound move. Single use. |
| BT-3477 | i:thunderstone.desc | Evolves Pikachu into Raichu or Alolan Raichu, Eevee into Jolteon, Eelektrik into Eelekt... |
| BT-3478 | i:thunderstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3479 | i:timerball.shortDesc | A Poke Ball that becomes better the more turns there are in a battle. |
| BT-3480 | i:toxicorb.shortDesc | At the end of every turn, this item attempts to badly poison the holder. |
| BT-3481 | i:toxicplate.shortDesc | Holder's Poison-type attacks have 1.2× power. Judgment is Poison type. |
| BT-3482 | i:tr00.shortDesc | Teaches certain Pokemon the move Swords Dance. One use. |
| BT-3483 | i:tr01.shortDesc | Teaches certain Pokemon the move Body Slam. One use. |
| BT-3484 | i:tr02.shortDesc | Teaches certain Pokemon the move Flamethrower. One use. |
| BT-3485 | i:tr03.shortDesc | Teaches certain Pokemon the move Hydro Pump. One use. |
| BT-3486 | i:tr04.shortDesc | Teaches certain Pokemon the move Surf. One use. |
| BT-3487 | i:tr05.shortDesc | Teaches certain Pokemon the move Ice Beam. One use. |
| BT-3488 | i:tr06.shortDesc | Teaches certain Pokemon the move Blizzard. One use. |
| BT-3489 | i:tr07.shortDesc | Teaches certain Pokemon the move Low Kick. One use. |
| BT-3490 | i:tr08.shortDesc | Teaches certain Pokemon the move Thunderbolt. One use. |
| BT-3491 | i:tr09.shortDesc | Teaches certain Pokemon the move Thunder. One use. |
| BT-3492 | i:tr10.shortDesc | Teaches certain Pokemon the move Earthquake. One use. |
| BT-3493 | i:tr11.shortDesc | Teaches certain Pokemon the move Psychic. One use. |
| BT-3494 | i:tr12.shortDesc | Teaches certain Pokemon the move Agility. One use. |
| BT-3495 | i:tr13.shortDesc | Teaches certain Pokemon the move Focus Energy. One use. |
| BT-3496 | i:tr14.shortDesc | Teaches certain Pokemon the move Metronome. One use. |
| BT-3497 | i:tr15.shortDesc | Teaches certain Pokemon the move Fire Blast. One use. |
| BT-3498 | i:tr16.shortDesc | Teaches certain Pokemon the move Waterfall. One use. |
| BT-3499 | i:tr17.shortDesc | Teaches certain Pokemon the move Amnesia. One use. |
| BT-3500 | i:tr18.shortDesc | Teaches certain Pokemon the move Leech Life. One use. |
| BT-3501 | i:tr19.shortDesc | Teaches certain Pokemon the move Tri Attack. One use. |
| BT-3502 | i:tr20.shortDesc | Teaches certain Pokemon the move Substitute. One use. |
| BT-3503 | i:tr21.shortDesc | Teaches certain Pokemon the move Reversal. One use. |
| BT-3504 | i:tr22.shortDesc | Teaches certain Pokemon the move Sludge Bomb. One use. |
| BT-3505 | i:tr23.shortDesc | Teaches certain Pokemon the move Spikes. One use. |
| BT-3506 | i:tr24.shortDesc | Teaches certain Pokemon the move Outrage. One use. |
| BT-3507 | i:tr25.shortDesc | Teaches certain Pokemon the move Psyshock. One use. |
| BT-3508 | i:tr26.shortDesc | Teaches certain Pokemon the move Endure. One use. |
| BT-3509 | i:tr27.shortDesc | Teaches certain Pokemon the move Sleep Talk. One use. |
| BT-3510 | i:tr28.shortDesc | Teaches certain Pokemon the move Megahorn. One use. |
| BT-3511 | i:tr29.shortDesc | Teaches certain Pokemon the move Baton Pass. One use. |
| BT-3512 | i:tr30.shortDesc | Teaches certain Pokemon the move Encore. One use. |
| BT-3513 | i:tr31.shortDesc | Teaches certain Pokemon the move Iron Tail. One use. |
| BT-3514 | i:tr32.shortDesc | Teaches certain Pokemon the move Crunch. One use. |
| BT-3515 | i:tr33.shortDesc | Teaches certain Pokemon the move Shadow Ball. One use. |
| BT-3516 | i:tr34.shortDesc | Teaches certain Pokemon the move Future Sight. One use. |
| BT-3517 | i:tr35.shortDesc | Teaches certain Pokemon the move Uproar. One use. |
| BT-3518 | i:tr36.shortDesc | Teaches certain Pokemon the move Heat Wave. One use. |
| BT-3519 | i:tr37.shortDesc | Teaches certain Pokemon the move Taunt. One use. |
| BT-3520 | i:tr38.shortDesc | Teaches certain Pokemon the move Trick. One use. |
| BT-3521 | i:tr39.shortDesc | Teaches certain Pokemon the move Superpower. One use. |
| BT-3522 | i:tr40.shortDesc | Teaches certain Pokemon the move Skill Swap. One use. |
| BT-3523 | i:tr41.shortDesc | Teaches certain Pokemon the move Blaze Kick. One use. |
| BT-3524 | i:tr42.shortDesc | Teaches certain Pokemon the move Hyper Voice. One use. |
| BT-3525 | i:tr43.shortDesc | Teaches certain Pokemon the move Overheat. One use. |
| BT-3526 | i:tr44.shortDesc | Teaches certain Pokemon the move Cosmic Power. One use. |
| BT-3527 | i:tr45.shortDesc | Teaches certain Pokemon the move Muddy Water. One use. |
| BT-3528 | i:tr46.shortDesc | Teaches certain Pokemon the move Iron Defense. One use. |
| BT-3529 | i:tr47.shortDesc | Teaches certain Pokemon the move Dragon Claw. One use. |
| BT-3530 | i:tr48.shortDesc | Teaches certain Pokemon the move Bulk Up. One use. |
| BT-3531 | i:tr49.shortDesc | Teaches certain Pokemon the move Calm Mind. One use. |
| BT-3532 | i:tr50.shortDesc | Teaches certain Pokemon the move Leaf Blade. One use. |
| BT-3533 | i:tr51.shortDesc | Teaches certain Pokemon the move Dragon Dance. One use. |
| BT-3534 | i:tr52.shortDesc | Teaches certain Pokemon the move Gyro Ball. One use. |
| BT-3535 | i:tr53.shortDesc | Teaches certain Pokemon the move Close Combat. One use. |
| BT-3536 | i:tr54.shortDesc | Teaches certain Pokemon the move Toxic Spikes. One use. |
| BT-3537 | i:tr55.shortDesc | Teaches certain Pokemon the move Flare Blitz. One use. |
| BT-3538 | i:tr56.shortDesc | Teaches certain Pokemon the move Aura Sphere. One use. |
| BT-3539 | i:tr57.shortDesc | Teaches certain Pokemon the move Poison Jab. One use. |
| BT-3540 | i:tr58.shortDesc | Teaches certain Pokemon the move Dark Pulse. One use. |
| BT-3541 | i:tr59.shortDesc | Teaches certain Pokemon the move Seed Bomb. One use. |
| BT-3542 | i:tr60.shortDesc | Teaches certain Pokemon the move X-Scissor. One use. |
| BT-3543 | i:tr61.shortDesc | Teaches certain Pokemon the move Bug Buzz. One use. |
| BT-3544 | i:tr62.shortDesc | Teaches certain Pokemon the move Dragon Pulse. One use. |
| BT-3545 | i:tr63.shortDesc | Teaches certain Pokemon the move Power Gem. One use. |
| BT-3546 | i:tr64.shortDesc | Teaches certain Pokemon the move Focus Blast. One use. |
| BT-3547 | i:tr65.shortDesc | Teaches certain Pokemon the move Energy Ball. One use. |
| BT-3548 | i:tr66.shortDesc | Teaches certain Pokemon the move Brave Bird. One use. |
| BT-3549 | i:tr67.shortDesc | Teaches certain Pokemon the move Earth Power. One use. |
| BT-3550 | i:tr68.shortDesc | Teaches certain Pokemon the move Nasty Plot. One use. |
| BT-3551 | i:tr69.shortDesc | Teaches certain Pokemon the move Zen Headbutt. One use. |
| BT-3552 | i:tr70.shortDesc | Teaches certain Pokemon the move Flash Cannon. One use. |
| BT-3553 | i:tr71.shortDesc | Teaches certain Pokemon the move Leaf Storm. One use. |
| BT-3554 | i:tr72.shortDesc | Teaches certain Pokemon the move Power Whip. One use. |
| BT-3555 | i:tr73.shortDesc | Teaches certain Pokemon the move Gunk Shot. One use. |
| BT-3556 | i:tr74.shortDesc | Teaches certain Pokemon the move Iron Head. One use. |
| BT-3557 | i:tr75.shortDesc | Teaches certain Pokemon the move Stone Edge. One use. |
| BT-3558 | i:tr76.shortDesc | Teaches certain Pokemon the move Stealth Rock. One use. |
| BT-3559 | i:tr77.shortDesc | Teaches certain Pokemon the move Grass Knot. One use. |
| BT-3560 | i:tr78.shortDesc | Teaches certain Pokemon the move Sludge Wave. One use. |
| BT-3561 | i:tr79.shortDesc | Teaches certain Pokemon the move Heavy Slam. One use. |
| BT-3562 | i:tr80.shortDesc | Teaches certain Pokemon the move Electro Ball. One use. |
| BT-3563 | i:tr81.shortDesc | Teaches certain Pokemon the move Foul Play. One use. |
| BT-3564 | i:tr82.shortDesc | Teaches certain Pokemon the move Stored Power. One use. |
| BT-3565 | i:tr83.shortDesc | Teaches certain Pokemon the move Ally Switch. One use. |
| BT-3566 | i:tr84.shortDesc | Teaches certain Pokemon the move Scald. One use. |
| BT-3567 | i:tr85.shortDesc | Teaches certain Pokemon the move Work Up. One use. |
| BT-3568 | i:tr86.shortDesc | Teaches certain Pokemon the move Wild Charge. One use. |
| BT-3569 | i:tr87.shortDesc | Teaches certain Pokemon the move Drill Run. One use. |
| BT-3570 | i:tr88.shortDesc | Teaches certain Pokemon the move Heat Crash. One use. |
| BT-3571 | i:tr89.shortDesc | Teaches certain Pokemon the move Hurricane. One use. |
| BT-3572 | i:tr90.shortDesc | Teaches certain Pokemon the move Play Rough. One use. |
| BT-3573 | i:tr91.shortDesc | Teaches certain Pokemon the move Venom Drench. One use. |
| BT-3574 | i:tr92.shortDesc | Teaches certain Pokemon the move Dazzling Gleam. One use. |
| BT-3575 | i:tr93.shortDesc | Teaches certain Pokemon the move Darkest Lariat. One use. |
| BT-3576 | i:tr94.shortDesc | Teaches certain Pokemon the move High Horsepower. One use. |
| BT-3577 | i:tr95.shortDesc | Teaches certain Pokemon the move Throat Chop. One use. |
| BT-3578 | i:tr96.shortDesc | Teaches certain Pokemon the move Pollen Puff. One use. |
| BT-3579 | i:tr97.shortDesc | Teaches certain Pokemon the move Psychic Fangs. One use. |
| BT-3580 | i:tr98.shortDesc | Teaches certain Pokemon the move Liquidation. One use. |
| BT-3581 | i:tr99.shortDesc | Teaches certain Pokemon the move Body Press. One use. |
| BT-3582 | i:twistedspoon.shortDesc | Holder's Psychic-type attacks have 1.2× power. |
| BT-3583 | i:tyranitarite.shortDesc | If held by a Tyranitar, this item allows it to Mega Evolve in battle. |
| BT-3584 | i:ultraball.shortDesc | An ultra-performance Ball that provides a higher catch rate than a Great Ball. |
| BT-3586 | i:ultranecroziumz.shortDesc | Dusk Mane/Dawn Wings Necrozma: Ultra Burst, then Z-Move w/ Photon Geyser. |
| BT-3588 | i:unremarkableteacup.shortDesc | Evolves Poltchageist into Sinistcha when used. |
| BT-3589 | i:upgrade.shortDesc | Evolves Porygon into Porygon2 when traded. |
| BT-3590 | i:utilityumbrella.desc | The holder ignores rain- and sun-based effects, including those of its Ability unless i... |
| BT-3591 | i:utilityumbrella.shortDesc | The holder ignores rain- and sun-based effects. |
| BT-3592 | i:venusaurite.shortDesc | If held by a Venusaur, this item allows it to Mega Evolve in battle. |
| BT-3593 | i:victreebelite.shortDesc | If held by a Victreebel, this item allows it to Mega Evolve in battle. |
| BT-3594 | i:vilevial.shortDesc | If held by a Venomicon, its Poison- and Flying-type attacks have 1.2× power. |
| BT-3595 | i:wacanberry.shortDesc | Halves damage taken from a supereffective Electric-type attack. Single use. |
| BT-3596 | i:watergem.shortDesc | Holder's first successful Water-type attack will have 1.3× power. Single use. |
| BT-3597 | i:wateriumz.shortDesc | If holder has a Water move, this item allows it to use a Water Z-Move. |
| BT-3598 | i:watermemory.shortDesc | Holder's Multi-Attack is Water type. |
| BT-3599 | i:waterstone.desc | Evolves Poliwhirl into Poliwrath, Shellder into Cloyster, Staryu into Starmie, Eevee in... |
| BT-3600 | i:waterstone.shortDesc | Evolves certain species of Pokemon when used. |
| BT-3601 | i:watmelberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3602 | i:waveincense.shortDesc | Holder's Water-type attacks have 1.2× power. |
| BT-3603 | i:weaknesspolicy.shortDesc | If holder is hit super effectively, raises Attack, Sp. Atk by 2 stages. Single use. |
| BT-3604 | i:wellspringmask.shortDesc | Ogerpon-Wellspring: 1.2× power attacks; Terastallize to gain Embody Aspect. |
| BT-3605 | i:wepearberry.shortDesc | Cannot be eaten by the holder. No effect when eaten with Bug Bite or Pluck. |
| BT-3606 | i:whippeddream.shortDesc | Evolves Swirlix into Slurpuff when traded. |
| BT-3608 | i:whiteherb.shortDesc | Restores all lowered stat stages to 0 when one is less than 0. Single use. |
| BT-3609 | i:widelens.shortDesc | The accuracy of attacks by the holder is 1.1×. |
| BT-3610 | i:wikiberry.shortDesc | Restores 1/3 max HP at 1/4 max HP or less; confuses if -SpA Nature. Single use. |
| BT-3611 | i:wiseglasses.shortDesc | Holder's special attacks have 1.1× power. |
| BT-3612 | i:yacheberry.shortDesc | Halves damage taken from a supereffective Ice-type attack. Single use. |
| BT-3613 | i:zapplate.shortDesc | Holder's Electric-type attacks have 1.2× power. Judgment is Electric type. |
| BT-3614 | i:zeraorite.shortDesc | If held by a Zeraora, this item allows it to Mega Evolve in battle. |
| BT-3615 | i:zoomlens.shortDesc | The accuracy of attacks by the holder is 1.2× if it moves after its target. |
| BT-3616 | i:zygardite.shortDesc | If held by a Zygarde in Complete Forme, this item allows it to Mega Evolve in battle. |
