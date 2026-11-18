# Gap register

Every known place where the engine plays the game differently from Pokémon Showdown's Champions rules. Ids
never change; quote them to refer to a finding. The project document is `../PROJECT.md`.

How to read it:

- "The engine" is poke-engine, the Rust code we start from. "Showdown" is the reference for the correct rules.
- These come from agents reading both codebases side by side, checked four ways on 17 Nov
  (`../docs/investigation/audit/completeness.md`): a comparison of all data by program, a second independent
  read of every row we use, and three completeness checks. Rows we do not use were read once.
- **(gold)** marks something on the gold team. "Same cause as X" means one fix covers both.
- The full row-by-row tables with file and line are in `../docs/investigation/audit/`.

## Work list by cause

The findings below, grouped by the fault that causes them, so that one fix closes many. Each group has an id
(W1 and so on), a size, and how often it matters. This is the order of work.

- **Size:** S = a number, flag or one condition. M = a new rule or handler, up to a few dozen lines. L = a
  new mechanism or a rewrite of an existing one.
- **How often:** "every battle" for rules that apply whatever the teams are. Otherwise the share of team
  slots in our training pool that hold the move, ability or item (from `docs/investigation/coverage/`).
- **When:** "core" = fixed by the core rewrite (step 8), not one at a time. "data" = fixed by generating the
  engine's tables from Showdown (requirement G2). "gap" = worked individually in step 9.
- Not every finding id is listed in a group. Single moves and abilities with their own fault are counted in
  W27 to W29 and worked in usage order from the lists further down.

### Tier 1: every battle, fixed in the core rewrite

| Id | Cause | Findings it closes | Size | How often | When |
|---|---|---|---|---|---|
| W1 | **One outcome per turn.** The engine lists every possible outcome with a probability; a simulator has to play one. Every random event (hit or miss, crit, damage roll, side effect, number of hits, sleep and confusion length, Outrage length, trap length, speed tie) has to be drawn from a seeded source, or forced in tests. | The ten known core differences; M01-11, M04-17, M05-18, M07-13, M08-14, A01-15, C15 | L | every battle | core |
| W2 | **Exact damage.** Whole-number arithmetic with each boost at Showdown's step, for the one roll that is drawn. | F1, F2, F3, F4, F9, F16, F17, F18, SR-29 | L | every battle | core |
| W3 | **Rounding of recoil, drain and healing.** Showdown rounds; the engine cuts off. | F7, F8, T3, M01-6, M02-14, M04-12, M05-9, M05-11 | S | most battles (Roost 6.7%, Wave Crash 2.8%, Flare Blitz 2.5%, Recover 2.0%) | core |
| W4 | **Order of steps when a move is used.** The engine triggers items and abilities before it knows whether the move lands, and lets some effects through Protect. Fixing the order closes all of these at once. | I1, I2, I3, A02-3, M04-10, SP11, A01-2, M02-5, M02-8, M04-3, M05-10, M05-12, T13, T14, SR-18 | L | every battle (Focus Sash 5.9%, Protect 5.5%, Disguise 3.0%) | core |
| W5 | **End-of-turn order.** Healing before poison and burn; every per-turn effect in Showdown's order, including the ones that do nothing today. | T1, T2, C9, I4, I7, T4, T9, T11, A03-7, X3, T17, C7, C10, M03-8, M03-10 | L | every battle (Leftovers 19.1%, Sitrus Berry 4.8%) | core |
| W6 | **Accuracy.** Never-miss moves can miss (135 moves); evasion does not exist; accuracy items and abilities do nothing; weather accuracy; one-hit-KO moves; per-hit accuracy. | F5, F6, C13, M01-3, M03-13, M07-12, M04-15, M06-1, M10-1, I10, F10, M03-3, M04-1, M05-1, M02-7, SR-11 | M | every battle (Swords Dance 11.2%, Stealth Rock 10.3%, Protect 5.5% are never-miss moves) | core |
| W7 | **Legal moves each turn.** What a Pokémon may choose: Fake Out and First Impression after the first turn, locked moves, Encore, Torment, Disable, Imprison, a Choice lock after the item is gone, PP and Struggle. | T16, M01-4, M01-5, M02-16, T6, M01-7, M10-6, C2 (Disable, Torment), M08-3 (Disable), SR-26 | M | every battle (Encore 7.8%, Choice Scarf 5.5%, Outrage 3.7%, Fake Out 2.7%, First Impression 2.1%) | core |
| W8 | **Format rules.** No sleep rule in this format; the three Pokémon not brought must not count as fainted; who wins when both faint; no end-of-turn damage after the game is won; entry order by speed. | C1, M05-2, SR-22, T7, T8, T5 | S to M | every battle (Last Respects 2.3%, Supreme Overlord 1.1%) | core |

### Tier 2: wrong data, fixed by generating the tables

| Id | Cause | Findings it closes | Size | How often | When |
|---|---|---|---|---|---|
| W9 | **Mega form data.** Four wrong abilities, two wrong stats. | SP1, SP2, SP3, SP4, SP5, SP6 | S | Mega Garchomp Z 4.5% (gold), Mega Golisopod 3.0% (gold), Mega Lucario Z 2.3% | data |
| W10 | **Move data.** Power, accuracy, type, tags, crit rate, a removed side effect. 23 moves. | M01-1, M06-5, M07-4, M07-8, M04-9, M10-3, M02-4, M03-5, M03-6, M03-7, M08-10, A01-10, X4 | S | Make It Rain 2.5% (gold), Freeze-Dry 1.3%, Dire Claw 0.9% | data |
| W11 | **No Pokédex in the engine.** Types, stats and weight of every non-Mega Pokémon come from whoever calls it; Mega Evolution does not change weight. | SP14, SP12 | M | every battle | data |

### Tier 3: shared causes, worked in step 9

| Id | Cause | Findings it closes | Size | How often | When |
|---|---|---|---|---|---|
| W12 | **Aimed at the Pokémon, its side, or the field.** The engine does not tell these apart, so abilities and rules that should only affect moves aimed at the holder also cancel field and self moves. | A01-1, M06-4, SR-16, SR-23, SR-25, M08-1, SR-13, A06-4, A01-8 | M | Good as Gold 2.5% (gold), Prankster 1.3% | gap |
| W13 | **Stat changes treated as attacks from the opponent.** Boosts are blocked by abilities that should only block drops; the abilities that react to drops are skipped or fire too often. | A06-5, M04-9, M08-2, SR-17, SR-24, M04-13, A02-8, A01-5, A01-11, A05-5 | M | Intimidate 5.4%, Stamina 3.3%, Clear Body 1.7%, Magic Bounce 1.4% | gap |
| W14 | **Substitute.** What goes through a Substitute and what does not. | M07-7, SR-19, SR-20, M03-11, M08-12, M04-4, SR-10, M04-14, M08-7, A02-5, A03-6 | M | Substitute 2.4% | gap |
| W15 | **Semi-invulnerable turns.** A Pokémon in the air or underground should still be hit by some moves, for double damage. | M08-5, SR-12 | M | Earthquake 16.0% (the attacker's side), Phantom Force 1.3% | gap |
| W16 | **Freeze.** Guaranteed thaw by the third attempt; thaw when hit by Fire moves or Scald. | T15, C8, M03-4, M07-5, M06-10 | S | Ice Beam 4.1% (the source of most freezes) | gap |
| W17 | **Screens.** Moves that break them; Defog; longer duration with Light Clay; damage on a crit. | C5, M02-3, M04-5, C6, M06-8, M02-15 | S to M | Light Clay 1.8%, Psychic Fangs 1.7%, Light Screen 1.4%, Aurora Veil 1.2%, Reflect 1.2% | gap |
| W18 | **Weather.** Snow effects check for hail; Cloud Nine and Air Lock only partly cancel weather; weather never lasts eight turns; several weather abilities are wrong. | C11, A04-1, C4 (rocks), M04-16, A01-14, A04-2, M02-10, SR-9, T10, A03-2, A03-8, A02-4 | M | Sand Stream 4.7%, Solar Beam 2.0%, Drought 1.8%, Weather Ball 1.7%, Snow Warning 1.6% | gap |
| W19 | **On the ground or not.** Terrain and Ground moves should only affect Pokémon on the ground; Roost, Smack Down, Magnet Rise, Iron Ball and Gravity change that. | F14, F15, M01-10, M02-11, M05-13, M01-9, M05-3 (Smack Down), M09-7, I9, C12 | M | Roost 6.7%, Grassy Glide 1.9%, Expanding Force 1.7%, Terrain Extender 1.3% | gap |
| W20 | **Mold Breaker.** It should get past the target's ability everywhere; the engine applies it in a few places only. | F13, A01-7, A03-4, A05-6, A06-6 | M | Mold Breaker 1.6% | gap |
| W21 | **Magic Guard.** It should block all damage that is not from a direct attack. | A02-7, A04-3, A05-8, A01-9, I5, M08-13 | S | Rough Skin 8.1% and Rocky Helmet 4.6% (the damage sources); Magic Guard itself 0.1% | gap |
| W22 | **Effects once per hit of a multi-hit move.** Contact abilities, Rocky Helmet, Multiscale, resist berries. | A01-12, A01-13, I5, I2 | M | Rough Skin 8.1%, Rocky Helmet 4.6%, Multiscale 3.9%, Triple Axel 1.7% | gap |
| W23 | **Crit chance.** Focus Energy, Scope Lens, Super Luck and the rest do nothing. | F12, M06-3 (Focus Energy), A05-11 (Merciless) | S | Scope Lens 0.1% | gap |

### Tier 4: missing mechanisms

| Id | Cause | Findings it closes | Size | How often | When |
|---|---|---|---|---|---|
| W24 | **Forced switches from items and abilities.** Emergency Exit, Wimp Out, Eject Button, Red Card. | T12, A01-4 (Emergency Exit) | L | Emergency Exit 3.0% (Golisopod before it Mega Evolves, gold) | gap |
| W25 | **Form changes.** Aegislash and King's Shield; Ditto and Transform; Castform; Mimikyu's Disguise; Mega Kangaskhan's double hit; Protean and Libero; Illusion. | SP7, M03-1, SP9, SP10, SP8, A01-3, SP13 | L | Protean 4.5%, Disguise 3.0%, Libero 1.2%, Aegislash 0.9%, Illusion 0.4%, Ditto 0.3% | gap |
| W26 | **Items the engine does not know.** 22 items. | I11, C4, M07-10 | S to M each | Light Clay 1.8%, Terrain Extender 1.3%, Quick Claw 0.8%, Damp Rock 0.6%, Smooth Rock 0.3%; the other 17 at 0.1% or less | gap |
| W27 | **Nothing switches items off.** Unnerve, Klutz, Embargo, Magic Room; a holder giving away its own Mega stone. | I13, I12 | M | Unnerve 0.2% | gap |

### Tier 5: single entries, in usage order

| Id | What | Count | When |
|---|---|---|---|
| W28 | Moves with their own fault not covered above | about 110 | gap, by usage: Sparkling Aria 3.8% (gold), Yawn 5.1%, Glaive Rush 1.5%, Curse 1.5%, Baton Pass 1.5%, Healing Wish 1.4%, Pain Split 1.4%, then the rest under 1% |
| W29 | Abilities with their own fault not covered above | about 120 | gap, by usage: Cursed Body 1.6%, Unburden 1.0%, Weak Armor 0.9%, then the rest under 1% |
| W30 | Items with their own fault not covered above | about 15 | gap, by usage: Sitrus Berry 4.8% (in W5), Shell Bell 0.1%, the terrain seeds |

### What to look at first

The gold team's own errors are in W9 (both Megas), W10 (Make It Rain), W4 (Focus Sash), W24 (Emergency
Exit) and W28 (Sparkling Aria). Tier 1 is the bulk of the work and the reason the core is rewritten and not
patched: eight causes there account for 86 of the findings and touch every battle.

Sizes are judgments from reading, not measurements. They will be corrected as the work is done.

---

## Findings

### Damage, accuracy and the numbers (F)

**F1. Damage is calculated in a different way, so it is often 1 HP off.** Showdown works in whole numbers
and applies each boost (Choice Band, Tough Claws, Life Orb) at its own step, rounding down each time. The
engine multiplies everything together as decimals and rounds once. Fixing it means rewriting the damage
calculation.

**F2. No rounding down between steps.** Part of F1.

**F3. The random damage roll is applied at the wrong point.** Every hit does between 85% and 100% of its
maximum. Showdown applies that before the same-type bonus; the engine applies it last. Part of F1.

**F4. A hit can do 0 damage.** In the game a hit that connects always does at least 1.

**F5. Evasion does nothing.** If a Pokémon raises its evasion (Double Team) or the attacker's accuracy is
lowered, moves should miss more. The engine only looks at the attacker's accuracy and ignores the target's
evasion.

**F6. Accuracy items and abilities do nothing.** Wide Lens, Zoom Lens, Bright Powder, Sand Veil (harder to
hit in sand), Snow Cloak, Tangled Feet and Gravity.

**F7. Recoil and drain are rounded the wrong way.** Recoil is the damage you take from your own move (Flare
Blitz); drain is the HP you get back (Drain Punch). 1 HP off about half the time.

**F8. Healing moves heal 1 HP too little** when max HP is an odd number (Recover, Roost).

**F9. Rocky Helmet does slightly too little.** It should take 1/6 of the attacker's HP; the engine uses
0.166.

**F10. One-hit-KO moves never work.** Fissure, Sheer Cold, Guillotine and Horn Drill do nothing.

**F11. Shell Side Arm is wrong.** This move picks physical or special, whichever does more. The engine does
not choose.

**F12. Most ways of raising the critical-hit chance are missing.** Focus Energy, Super Luck, Scope Lens and
Razor Claw do nothing. Only moves with a built-in high crit rate work.

**F13. Mold Breaker does not get past crit-blocking abilities.** Mold Breaker ignores the target's ability;
it should let crits through Battle Armor and Shell Armor.

**F14. Rising Voltage and Expanding Force are wrong.** These get stronger on Electric and Psychic Terrain.
The engine uses the wrong multiplier and does not check whether the Pokémon is on the ground (terrain does
not affect flying or levitating Pokémon).

**F15. Grassy Terrain's Earthquake weakening is incomplete.** It should also weaken Bulldoze, and only for
targets on the ground.

**F16. Flail and Reversal use approximate HP thresholds.** These get stronger as the user's HP drops; the
cut-off points are slightly off.

**F17. Gyro Ball and Electro Ball ignore speed changes.** Their power depends on speed, but the engine
ignores Choice Scarf, Tailwind and paralysis when comparing.

**F18. Technician is fragile.** It boosts moves of 60 power or less; whether the check happens before or
after other power changes depends on code order.

**F19. What already matches.** The stat formula, stat stages (+1, +2 and so on), the type chart, and
fixed-damage moves (Seismic Toss). No work needed.

**F20. Not fully verified.** Counter, Mirror Coat, Metal Burst and some special-damage moves were only
partly checked.

### How a turn runs (T)

**T1. End-of-turn effects happen in the wrong order.** In the game, healing (Leftovers) comes before poison
and burn damage. The engine does damage first, so a Pokémon at very low HP holding Leftovers faints when it
should survive. This comes up in most games.

**T2. Bad poison (Toxic) damage is slightly off,** and it keeps growing past the game's limit.

**T3. Same as F7 and F8.**

**T4. Sitrus Berry is eaten too late.** It should be eaten the moment HP drops to half; the engine waits for
the end of the turn.

**T5. When both players send out a Pokémon at once, order is always player one first.** It should go by
speed. This matters for things like two weather setters (the slower one's weather stays).

**T6. A Pokémon locked into Outrage can still choose another move.** It should be forced to keep using it.

**T7. If both last Pokémon faint together, the engine always gives player two the win.** The game gives it
by who fainted last.

**T8. A player can lose after winning.** If you knock out the last opposing Pokémon, the engine still applies
end-of-turn damage, and if your last Pokémon then faints you can be given the loss.

**T9. Speed Boost triggers on the turn the Pokémon comes in.** It should start the turn after.

**T10. Weather-related abilities are wrong.** Solar Power, Dry Skin (should be hurt by sun), Ice Body and
Slush Rush (should work in snow).

**T11. Shed Skin and Hydration are wrong.** Shed Skin should cure a status one time in three; the engine
cures every time. Hydration (cures status in rain) acts after the poison damage instead of before.

**T12. Missing entirely:** Eject Button and Red Card (items that force a switch), Emergency Exit and Wimp Out
(abilities that switch out at half HP), Quick Claw and Quick Draw (chance to move first), Mycelium Might,
Stall, Beak Blast and Shell Trap.

**T13. A missed Dragon Tail or Roar still cancels the opponent's move.**

**T14. PP is used up when a move does not happen.** If a Pokémon is asleep, fully paralysed, frozen or hits
itself in confusion, the move should not lose PP.

**T15. Freeze can last forever.** In Champions a frozen Pokémon is guaranteed to thaw by its third attempt.

**T16. Fake Out can be picked after the first turn.** It only works on the turn the Pokémon comes in, and
Champions greys it out afterwards. (Same as M01-4.)

**T17. Effects that should act every turn and do nothing:** Aqua Ring and Ingrain (heal a little each turn),
Ghost-type Curse, Syrup Bomb, Octolock, Magnet Rise wearing off, Harvest (regrows a berry), Light Clay,
Disable, Attract.

**T18. Settled, no action.** A speed change in the middle of a turn cannot change move order in singles,
apart from the Mega turn, which is already a known gap.

**T19. Could not be determined.** Whether a Pokémon that faints to Stealth Rock on entry still triggers its
entry ability; Prankster against Dark types; Gorilla Tactics; Slow Start.

### Statuses, screens, weather and other lasting effects (C)

**C1. The engine refuses to put a second Pokémon to sleep.** That is a rule ("Sleep Clause") from other
formats. Champions has no such rule.

**C2. Moves that do nothing because nothing reads their effect:** King's Shield (and its marker never goes
away), Throat Chop (stops sound moves), Focus Energy, Disable, Torment, Ingrain, Aqua Ring, Octolock, Smack
Down, Ghost-type Curse.

**C3. Glaive Rush and Unburden never switch on.** Glaive Rush should make the user take double damage next
turn; Unburden doubles speed when the held item is used up.

**C4. Seven legal items are unknown to the engine:** Light Clay (longer screens), Terrain Extender, Damp
Rock, Heat Rock, Smooth Rock, Icy Rock (longer weather) and Binding Band.

**C5. Brick Break and Psychic Fangs do not remove screens.** Screens are Reflect and Light Screen, which
halve damage.

**C6. Defog removes the user's own screens.** It should remove only the opponent's.

**C7. Safeguard never ends.** It should last five turns.

**C8. Freeze does not thaw when it should.** Same as T15, and being hit by a Fire move or Scald should thaw
the target.

**C9. Same as T2.**

**C10. Encore and Taunt last the wrong number of turns in some cases,** depending on who moved first.

**C11. Snow effects are broken.** Blizzard should never miss in snow, and Slush Rush and Ice Body should
work in snow. The engine only checks for hail, which does not exist in this format.

**C12. Gravity, Magic Room, Wonder Room and Fairy Lock do not exist** in the engine.

**C13. Same as F5.**

**C14. Baton Pass passes almost nothing.** It should pass stat boosts and several effects to the Pokémon
coming in; the engine passes only Substitute and Leech Seed.

**C15. Trapping moves, confusion and Outrage have extra errors** beyond their known gaps: burn wrongly
halves confusion self-damage, and Binding Band is missing.

**C16. Low confidence.** Stockpile, Beak Blast, and Healing Wish when there is nothing to switch to.

### Items (I): all 166 audited, 27 match

**I1. (gold) Focus Sash is never used up.** It should let a full-HP Pokémon survive one knockout hit at 1 HP
and then be gone. In the engine every hit on a full-HP holder does 1 less damage and the sash stays. Sturdy
has the same fault.

**I2. Resist berries are eaten too early.** These halve one super-effective hit (Yache for Ice, Occa for
Fire, 17 in all). The engine eats the berry before checking the attack lands, so it is wasted on a miss, on
Protect, when the attacker is asleep, and even on status moves of that type (Thunder Wave into a Wacan Berry).
On a multi-hit move it halves every hit instead of the first.

**I3. Normal Gem and Shuca Berry have the same early-use fault.**

**I4. Leftovers heals after poison and burn.** Same as T1.

**I5. Rocky Helmet is wrong.** It should hurt the attacker on each hit of a multi-hit move, and not at all
if the attacker has Magic Guard.

**I6. Shell Bell cancels a move's own drain.** Using Drain Punch while holding Shell Bell loses the Drain
Punch healing.

**I7. Sitrus, Lum, Pecha and Rawst Berries trigger at the wrong moment.** Lum Berry also ignores confusion.
The holder takes one extra poison or burn hit.

**I8. Terrain seeds only trigger on entry.** Grassy Seed and the others should also trigger when terrain
appears while the holder is already out.

**I9. Iron Ball does not ground Flying types.** It should make them hittable by Ground moves.

**I10. Wide Lens gives no accuracy boost.** Same family as F6.

**I11. 22 items do nothing because the engine does not know them:** Light Clay, Terrain Extender, Quick
Claw, Damp Rock, Smooth Rock, Heat Rock, Icy Rock, Eject Button, Red Card, Scope Lens, King's Rock, Zoom
Lens, Leek, Big Root, Bright Powder, Binding Band, Focus Band, Leppa Berry, Mental Herb, Metronome, Oran
Berry, Persim Berry.

**I12. A Pokémon can give away its own Mega stone with Trick.** The move should fail. One small fix covers
all 70 stones.

**I13. Nothing ever switches items off.** Unnerve (opponent cannot eat berries), Klutz, Embargo and Magic
Room have no effect.

### Species and Mega forms (SP): all 392 audited

**SP1. (gold) Mega Garchomp Z has the wrong ability.** The engine gives it Sand Force; it should be
Levitate, so it wrongly takes Ground moves.

**SP2. (gold) Mega Golisopod has the wrong ability.** The engine gives it Emergency Exit (switches out at
half HP); it should be Tough Claws (stronger contact moves).

**SP3. Mega Absol Z has the wrong ability.** Magic Bounce instead of Sharpness.

**SP4. Mega Lucario Z cannot be loaded at all.** Its ability, Aura Guard, does not exist in the engine.

**SP5. Mega Starmie has Attack 140; it should be 100.**

**SP6. Mega Drampa has Special Defense 114; it should be 116.**

**SP7. Aegislash cannot change form.** It should switch to its attacking form when it attacks and back when
it uses King's Shield.

**SP8. Mega Kangaskhan's ability does nothing.** Parental Bond should make every move hit twice.

**SP9. Ditto cannot transform.** Neither its ability (Imposter) nor the move Transform works.

**SP10. Castform never changes form with the weather.**

**SP11. Mimikyu's Disguise breaks too early.** Disguise blocks the first hit. In the engine a move that
misses or is blocked by Protect still breaks it.

**SP12. Mega Evolution does not change weight.** Some moves depend on weight (Low Kick, Grass Knot, Heavy
Slam), and several Megas are much heavier.

**SP13. Farfetch'd and Sirfetch'd cannot hold Leek.** The engine does not know the item.

**SP14. The engine has no Pokédex.** For every non-Mega Pokémon it trusts whatever types, stats and weight
the caller gives it. Our wrapper has to supply them from Showdown's data.

**SP15. What matches.** 75 of 82 Mega forms are correct in stats, types and ability.

### Moves slice 01, the 52 most used (M01): 31 fine

**M01-1. (gold) Make It Rain is too weak and too accurate.** Power 95 and accuracy 100; it should be 120
and 95.

**M01-2. (gold) Sparkling Aria does not cure burns.** It should cure the target's burn.

**M01-3. Moves that can never miss can miss.** Swords Dance, Stealth Rock, Protect, Substitute, Aura Sphere
and others fail sometimes after the user's accuracy is lowered. Same cause as F5.

**M01-4. Fake Out can be picked after the first turn.** Same as T16.

**M01-5. Fake Out gets a second chance after a flinch.** If the user flinched on the turn it came in, the
engine allows Fake Out next turn.

**M01-6. Same as F7** (Flare Blitz, Wave Crash, Leech Life).

**M01-7. Encore works on moves it should fail against,** and does not check that the repeated move has PP
left.

**M01-8. Yawn puts a Pokémon to sleep even if it became immune meanwhile.** Yawn causes sleep a turn later;
if terrain or Safeguard appeared in between, it should fail.

**M01-9. Roost on a pure Flying type is wrong.** It should become Normal type for the turn and count as on
the ground.

**M01-10. Same as F15.**

**M01-11. Outrage always lasts the same number of turns.** It should last 2 or 3 at random. Already a known
gap.

**M01-12. Unconfirmed.** The reader says Fake Out and Encore only work if a certain setup step is called. I
have not checked this.

### Moves slice 06, rarely used (M06): 21 of 52 fine

**M06-1. Double Team and Minimize do nothing.** Same cause as F5: evasion does not exist in the engine.

**M06-2. Moves with a condition have no condition.** Stomping Tantrum (double power after a failed move)
never doubles; Last Resort (only after every other move was used) and Belch (only after eating a berry)
always work; Fell Stinger gives no Attack boost after a knockout.

**M06-3. Skill Swap, Transform and Focus Energy do nothing.**

**M06-4. Prankster is blocked too much.** Prankster gives status moves priority, and Dark types are immune
to them. The engine also blocks the user's own boosting moves (Howl, Growth) when the opponent is Dark.

**M06-5. Wrong numbers.** Growth should be Grass type, Slash power 80 (engine 70), Meteor Assault power 170
(engine 150), and Aerial Ace should never miss.

**M06-6. Spirit Shackle does not trap.** It should stop the target switching out.

**M06-7. No Retreat traps Ghost types.** Ghosts cannot be trapped.

**M06-8. Raging Bull does too much damage on a critical hit** when the opponent has a screen up.

**M06-9. Focus Punch can fail wrongly** after the opponent switches.

**M06-10. Matcha Gotcha does not thaw a frozen target.**

**M06-11. Hex ignores Comatose.** Hex doubles against a target with a status; Comatose counts as asleep.

### Moves slice 09, rarely used (M09): 18 of 52 fine

**M09-1. Flying Press is only Fighting type.** It should count as Fighting and Flying at once.

**M09-2. Moves whose effect is never read:** Gastro Acid (turns off an ability), Octolock, Power Trick,
Rage Powder, Quick Guard.

**M09-3. Missing entirely:** Lock-On, Psych Up, Reflect Type, Recycle, Guard Swap, Power Swap, Guard Split,
Power Split, Magic Room, Magic Powder, Forest's Curse, Instruct, Heal Pulse, Magnetic Flux.

**M09-4. Infernal Parade and Lash Out never double.** Infernal Parade doubles against a target with a
status; Lash Out doubles if the user's stats were lowered that turn.

**M09-5. Jaw Lock does not trap; Pluck does not eat the target's berry.**

**M09-6. Heal Bell cures Pokémon it should not.**

**M09-7. Magnet Rise never wears off, and hazards still hit.** Magnet Rise lifts the user off the ground for
five turns; Spikes should not hurt it.

**M09-8. Dancer and Gravity do not exist** in the engine.

### Moves slice 10, rarely used (M10): 8 of 46 fine

**M10-1. Same as M01-3,** for Shadow Punch, Smart Strike, Sunny Day and others.

**M10-2. Wide Guard blocks nothing; Safeguard does not block confusion.** (Safeguard never ending is C7.)

**M10-3. Syrup Bomb has the wrong numbers.** Power 90 in the engine; it should be 60 with accuracy 90. Its
speed-lowering effect is missing.

**M10-4. Missing entirely:** Role Play, Simple Beam, Worry Seed (change abilities), Trick-or-Treat, Speed
Swap, Spite, Stuff Cheeks, Teatime, Wonder Room, Spit Up, Swallow.

**M10-5. Venoshock and Round never double.**

**M10-6. Torment does not restrict moves.** It should stop the target using the same move twice in a row.

**M10-7. Snore and Sleep Talk are backwards.** Snore should only work asleep; in the engine it only works
awake.

### Abilities slice 02 (A02): 13 of 53 match

**A02-1. Piercing Drill and Unseen Fist do a quarter damage all the time.** These let contact moves hit
through Protect at quarter power. The engine applies the quarter even when the target is not protecting.

**A02-2. Leaf Guard lets a statused Pokémon be statused again** when there is no sun.

**A02-3. Sturdy has the Focus Sash fault.** Same as I1.

**A02-4. Mega Sol is half done.** It makes the user's moves act as if the sun is out. Solar Beam still
needs a charge turn, and weather-dependent accuracy ignores it.

**A02-5. Contact abilities trigger wrongly.** Gooey, Poison Point and Spicy Spray should trigger when the
holder is touched. In the engine they also trigger through a Substitute, and Poison Point is 33% instead of
30%.

**A02-6. 15 abilities do nothing:** Cute Charm, Illuminate, Electromorphosis, Illusion, Frisk, Imposter,
Mirror Armor, Unnerve, Moody, Harvest, Synchronize, Quick Draw, Early Bird, Super Luck, Parental Bond.

**A02-7. Magic Guard does not block recoil.** Magic Guard should stop all damage that is not from a direct
attack.

**A02-8. Defiant and Competitive trigger too often.** They should boost once per stat-lowering move, not
once per stat lowered.

### Abilities slice 04 (A04): 16 of 53 fine

**A04-1. Slush Rush never works.** Same as C11.

**A04-2. Air Lock and Cloud Nine only partly cancel weather.** They should switch off all weather effects.
The engine still doubles speed for Swift Swim and Chlorophyll, and still changes Weather Ball and healing
moves.

**A04-3. Bad Dreams and Aftermath ignore Magic Guard.** Bad Dreams (Mega Darkrai) hurts sleeping opponents;
Magic Guard should block it.

**A04-4. Liquid Ooze is half done.** It should hurt a Pokémon that drains it. Leech Seed still heals the
seeder.

**A04-5. Abilities that do nothing:** Sand Veil, Big Pecks, Anger Point, Anger Shell, Cheek Pouch, Dancer,
Forecast, Flower Gift.

**A04-6. Abilities that do the wrong thing:** Battery boosts the user's own moves (it should only boost a
partner, so nothing in singles); Flare Boost boosts physical moves (should be special); Wandering Spirit;
Dark Aura; Dauntless Shield triggers on every entry (should be once a battle); Beast Boost picks the wrong
stat.

### Abilities slice 06 (A06): 14 of 52 fine

**A06-1. Same as A02-1** (Unseen Fist).

**A06-2. Truant only starts after a damaging hit.** A Truant Pokémon should skip every second turn whatever
it did.

**A06-3. Toxic Boost boosts every move.** It should boost physical moves only.

**A06-4. Abilities add side effects to status moves.** A Stench user's Swords Dance can make the opponent
flinch; Will-O-Wisp on a Steam Engine Pokémon gives it +6 Speed.

**A06-5. Clear Body and White Smoke block boosts as well as drops.** They should only stop the opponent
lowering stats.

**A06-6. Mold Breaker is not checked** for status immunity, stat-drop immunity or Sticky Hold.

**A06-7. Missing entirely:** Stall, Run Away (Champions version), Wimp Out, Soul-Heart, Simple, Supersweet
Syrup, Wind Power, Ripen, Zen Mode.

### Moves slice 07, rarely used (M07): 29 of 52 match

**M07-1. Moves that do nothing:** Attract (the target may refuse to attack), Aqua Ring, Stockpile, Imprison,
Entrainment (copies the user's ability onto the target), Topsy-Turvy (reverses the target's stat changes) and
Acupressure (sharply raises a random stat).

**M07-2. Fickle Beam has the wrong effect.** It should have a 30% chance of double power. The engine instead
gives a 50% chance of raising the user's Special Attack.

**M07-3. Charge never wears off.** It should double the next Electric move only. In the engine every later
Electric move stays doubled until the user switches out.

**M07-4. Crabhammer has the wrong numbers.** Power 95 and accuracy 90; it should be 100 and 95.

**M07-5. Fire moves do not thaw a frozen target.** Same as C8. Affects Lava Plume, Heat Wave and Scorching
Sands.

**M07-6. Steel Roller never removes terrain.** That is its whole point: it destroys the terrain and fails if
there is none.

**M07-7. Sound moves are blocked by Substitute.** Sing and other sound-based status moves should go through
a Substitute.

**M07-8. Air Cutter is missing its "slicing" and "wind" tags,** so Sharpness and wind abilities do not apply
to it.

**M07-9. Clangorous Soul is slightly off.** It costs a third of max HP to raise every stat; the HP cut-off
differs for some HP values, and it should fail when no stat can rise.

**M07-10. Terrain Extender is not implemented.** Same as C4.

**M07-11. Comeuppance does 0 damage after a hit on a Substitute;** it should do 1.

**M07-12. Mean Look can miss.** Same cause as M01-3.

**M07-13. Waiting on the core rewrite:** Future Sight, Water Spout (power not rounded down) and Wrap.

### Moves slice 03, commonly used (M03): 26 of 52 fine

**M03-1. King's Shield does nothing.** It should block attacks and lower the Attack of anything that makes
contact. The engine sets a marker that nothing reads and that never goes away. Same as C2; this is
Aegislash's main move.

**M03-2. Revival Blessing does nothing.** It should bring back a fainted team member at half HP.

**M03-3. Sheer Cold never works.** Same as F10. It also lacks its special rules: Ice types are immune, and
it is less accurate when the user is not Ice type.

**M03-4. Fire moves and Scald do not thaw a frozen target.** Same as C8. Affects Overheat, Fiery Dance,
Blaze Kick, Bitter Blade and Scald.

**M03-5. Dire Claw has no side effect.** In Champions it has a 30% chance to poison, paralyse or put to
sleep. The engine also gives it a high crit rate it should not have, and it is missing the slicing tag.

**M03-6. Blizzard is missing its wind tag and its snow accuracy.** Same as C11.

**M03-7. Leech Seed is blocked by Overcoat.** It is wrongly tagged as a powder move. Double Shock is missing
its punch tag.

**M03-8. Taunt ends one action early.** It blocks two of the target's actions instead of three.

**M03-9. Rest works when it should fail.** It should fail at full HP and for Pokémon that cannot sleep
(Insomnia, Vital Spirit).

**M03-10. Psychic Noise's healing block never ends.** It should last two turns, and should block all
healing, not only healing moves.

**M03-11. Terrain Pulse ignores whether the user is on the ground.** Infestation traps through a Substitute.

**M03-12. Roar and Whirlwind ignore Ingrain.** A rooted Pokémon cannot be forced out; Ingrain does not exist
in the engine (C2).

**M03-13. Kowtow Cleave can miss.** Same cause as M01-3.

**M03-14. Undetermined: Avalanche.** It doubles if the user was hit first that turn. It may wrongly double
after the opponent switches.

### Moves slice 05, moderately used (M05): 21 of 52 match

**M05-1. Fissure never hits.** Same as F10.

**M05-2. The engine blocks a second sleep.** Same as C1: Hypnosis and Sleep Powder are refused while another
opposing Pokémon is asleep. The format has no such rule, and no rule against one-hit-KO moves either.

**M05-3. Moves with no behaviour:** Rage Fist (should get stronger each time the user is hit; stays at 50
power), Throat Chop, Burning Jealousy (should burn a target that raised its stats that turn), Gravity,
Ingrain. Smack Down sets a marker but does not bring the target to the ground.

**M05-4. Armor Cannon does not lower the user's defences.** It should lower Defense and Special Defense by
one stage each.

**M05-5. Shell Side Arm is always special.** Same as F11.

**M05-6. Beak Blast does not burn on contact.** The user heats up first; anything that touches it that turn
should be burned.

**M05-7. Rapid Spin and Mortal Spin do not free the user from Leech Seed or trapping moves.**

**M05-8. Belly Drum still costs half HP when Attack is already at maximum.** It should fail.

**M05-9. Shed Tail is wrong.** It costs half HP to leave a Substitute for the next Pokémon. The HP amounts
are rounded the wrong way, and it still costs HP when there is nothing to switch to.

**M05-10. Memento knocks the user out even when it was blocked by a Substitute.**

**M05-11. Recoil and drain rounding.** Same as F7. Affects Head Smash, Wild Charge and Horn Leech.

**M05-12. Defog is wrong in three ways.** It removes the user's own screens (C6), does not remove the
target's Safeguard and Mist, and clears hazards even when blocked by Protect. Rapid Spin and Mortal Spin
share the Protect problem.

**M05-13. Misty Explosion's terrain boost has no on-the-ground check.**

**M05-14. Damp on the user's own side does not stop Explosion-type moves.**

**M05-15. Metal Burst goes through Substitute, Sturdy and Focus Sash.** It should be treated as a normal
hit.

**M05-16. Aqua Step is missing its dance tag,** and the Dancer ability does not exist (M09-8).

**M05-17. No new item gaps.** The reader listed Safety Goggles, Terrain Extender, Damp Rock and Light Clay
as unknown to the engine. The last three are already in C4 and I11. Safety Goggles is not legal in Champions
(checked against the Showdown dump), so it needs nothing.

**M05-18. Waiting on the core rewrite:** Pin Missile, Icicle Spear, Population Bomb (number of hits), Water
Pulse (confusion), Thrash (lock length).

### Moves slice 04, moderately used (M04): 28 of 51 fine

**M04-1. Horn Drill never works.** Same as F10.

**M04-2. Soak does nothing.** It should turn the target into a pure Water type.

**M04-3. Fixed-damage moves go through Protect.** Final Gambit (the user faints and deals damage equal to
its HP), Seismic Toss, Night Shade, Super Fang and Endeavor still land when the target used Protect, Spiky
Shield or Baneful Bunker.

**M04-4. Counter and Mirror Coat work through a Substitute.** They should fail. They can also use the
damage from an earlier turn.

**M04-5. Brick Break does not break screens.** Same as C5.

**M04-6. Temper Flare never doubles.** It should double if the user's previous move failed.

**M04-7. Alluring Voice never confuses.** It should confuse a target that raised its stats that turn.

**M04-8. Parting Shot switches out even when it did nothing.** If both stat drops fail, the user should
stay in.

**M04-9. Swagger has accuracy 90; it should be 85.** Its Attack boost is also wrongly blocked by Clear
Body and similar abilities (same cause as A06-5).

**M04-10. Endure takes 1 HP off every hit.** Same fault as Focus Sash (I1).

**M04-11. Spiky Shield, Baneful Bunker and Endure do not fail when the user moves last.** Only Protect has
that check.

**M04-12. Drain rounding.** Same as F7. Affects Drain Punch, Draining Kiss and Parabolic Charge.

**M04-13. Sticky Web's speed drop ignores abilities that react to stat drops.** Clear Body should block it;
Defiant should trigger from it.

**M04-14. Strength Sap heals through a Substitute.** It should fail.

**M04-15. Thunder can miss in rain.** It should never miss in rain; the engine gives it accuracy 100, which
still fails after an accuracy drop (same cause as M01-3).

**M04-16. Weather always lasts five turns.** The weather-extending rocks do not exist (C4).

**M04-17. Waiting on the core rewrite:** Water Shuriken, Rock Blast, Bullet Seed (number of hits).

### Rows the audit skipped (found by a program check of every slice file against the row lists)

**X1. Hyper Beam was not audited** in slice 04 (row 209; the reader wrongly reported that the list ended at
row 208). Audited since: it matches. The recharge turn is applied after a hit, a knockout or a hit on a
Substitute, and not after a miss, an immunity or Protect, as in Showdown. It needs a regression test when the
core is rewritten, because the Protect case works by a side route in today's code.

**X2. Zing Zap was not audited** in slice 10 (row 516, the last row; the slice was given rows 470 to 515).
Audited since: it matches (power 80, 30% flinch, no flinch when the user moves second).

### Moves slice 02, commonly used (M02): 25 of 52 fine

**M02-1. Curse is wrong for Ghost types.** For a Ghost, Curse costs half the user's HP and puts a curse on
the opponent that takes a quarter of its HP every turn. The engine gives every user the non-Ghost version
(Attack and Defense up, Speed down).

**M02-2. Glaive Rush has no drawback.** Same as C3: the user should take double damage until its next turn.

**M02-3. Psychic Fangs does not break screens.** Same as C5.

**M02-4. Freeze-Dry can still freeze.** Champions removed its 10% freeze chance; the engine keeps it.

**M02-5. Pain Split ignores Protect and works even when it misses.** Pain Split averages both Pokémon's HP.

**M02-6. Healing Wish knocks the user out with nothing to switch to.** It should fail.

**M02-7. Triple Axel hits three times too often.** Each of its three hits should check accuracy separately
(73% for all three); the engine checks once (90%).

**M02-8. Steel Beam has no penalty on a miss.** It always costs the user half its HP, even when it misses
or is blocked.

**M02-9. Baton Pass drops effects it should pass.** Same as C14.

**M02-10. Weather moves ignore weather-cancelling abilities.** Solar Beam, Weather Ball, Moonlight and
Morning Sun act as if the weather is there when Cloud Nine has cancelled it. Solar Beam also ignores Mega
Sol (A02-4).

**M02-11. Grassy Glide and Expanding Force skip the user-on-the-ground check.** Same family as F14.

**M02-12. Heavy-Duty Boots stop a Poison type absorbing Toxic Spikes.** A Poison type on the ground should
remove them on entry even with Boots.

**M02-13. Dragon Tail ignores Ingrain.** Same as M03-12.

**M02-14. Rounding of heals, drain and recoil.** Same as F7 and F8.

**M02-15. Screens always last five turns.** Light Clay is missing (C4).

**M02-16. First Impression stays selectable after its first turn.** Same kind of fault as Fake Out (T16).

### Moves slice 08, rarely used (M08): 19 of 52 fine

**M08-1. Coaching and Aromatic Mist boost the user.** These only help a partner in doubles and should fail
in singles. In the engine they are free stat boosts.

**M08-2. Stat raises on the opponent are blocked by Clear Body.** Decorate and Flatter raise the target's
stats; abilities that stop stat drops wrongly stop them too. Same cause as A06-5.

**M08-3. Moves that do nothing:** Disable (C2), Electrify, Fairy Lock, Copycat, Fling, Beat Up (deals no
damage) and Corrosive Gas.

**M08-4. Moves missing part of their effect:** Covet does not steal the item, Bug Bite does not eat the
target's berry, Assurance never doubles, Eerie Spell does not remove PP.

**M08-5. A Pokémon in the air or underground cannot be hit by anything.** During Dig, Dive, Bounce and Fly
the user dodges most moves, but some should still hit, a few for double damage: Earthquake hits Dig, Surf
hits Dive, Thunder and Hurricane hit Fly. The engine blocks everything. It also skips sandstorm immunity
while underground or underwater.

**M08-6. Confusion gets through its blockers.** Own Tempo, Misty Terrain and Safeguard should prevent
confusion. Affects Confuse Ray, Flatter and Axe Kick.

**M08-7. Clear Smog resets stat changes through a Substitute.**

**M08-8. Court Change does not swap Mist or Safeguard.** It swaps the effects on each side of the field.

**M08-9. Facade doubles while the user is asleep.** It should double only for burn, poison and paralysis.

**M08-10. Crush Claw is missing its slicing tag** (a Champions change).

**M08-11. Flail's power thresholds.** Same as F16.

**M08-12. Bind traps through a Substitute.** Same as M03-11 for Infestation.

**M08-13. Axe Kick's crash damage ignores Magic Guard.** Medium confidence.

**M08-14. Waiting on the core rewrite:** Bone Rush (number of hits), Cross Chop and Cross Poison (crit
rate).

### Abilities slice 05 (A05): 21 of 53 fine

**A05-1. Pickpocket is backwards.** It should steal the item of a Pokémon that touches the holder. In the
engine the holder steals when it attacks.

**A05-2. Lingering Aroma spreads the wrong ability.** A Pokémon that touches the holder should get Lingering
Aroma; the engine gives it Mummy.

**A05-3. Intrepid Sword triggers on every entry.** It should raise Attack once per battle (same kind of
fault as Dauntless Shield in A04-6).

**A05-4. Protosynthesis and Quark Drive miss changes while the holder is out.** These boost the holder's
best stat in sun or on Electric Terrain. They should switch on and off when the sun or terrain starts or
ends, and a boost from Booster Energy should stay; the engine only checks on entry.

**A05-5. Immunities that are missing:** Own Tempo does not prevent confusion (M08-6), Guard Dog does not
turn Intimidate into an Attack boost, Mind's Eye does not prevent accuracy drops, Full Metal Body does not
block Sticky Web's speed drop.

**A05-6. Mold Breaker does not get past Immunity, Pastel Veil or Own Tempo.** Same family as A06-6 and F13.

**A05-7. Galvanize and Normalize are incomplete.** These change the type of the user's moves. Some moves
should be left alone, Normalize should add a 20% boost, and Weather Ball is converted wrongly in clear
weather.

**A05-8. Iron Barbs and Gulp Missile ignore Magic Guard.** Same family as A02-7 and A04-3.

**A05-9. Magician steals after pivot moves** (U-turn, Volt Switch); it should not. Perish Body wrongly
spares Soundproof Pokémon.

**A05-10. Mycelium Might does not make status moves go last.** Neutralizing Gas is only partly modelled. Ice
Face does not re-form when snow starts mid-battle.

**A05-11. Abilities that do nothing:** Klutz (the holder's item has no effect), Merciless (always crits a
poisoned target), Opportunist (copies the opponent's stat boosts), Pickup, Poison Puppeteer, Power Construct,
Multitype, RKS System.

### Abilities slice 03 (A03): 21 of 52 match

**A03-1. Sheer Force breaks the holder's status moves.** Sheer Force removes the side effects of attacks in
exchange for power. The engine removes the effect of every move that applies a lasting condition, so the
holder's Protect, Taunt, Encore, Leech Seed, Confuse Ray and Focus Energy do nothing.

**A03-2. Solar Power is wrong three ways.** It should boost special moves only, in sun and in harsh sun, and
cost an eighth of max HP each turn. The engine also boosts physical moves, gives nothing in harsh sun, and
takes no HP at full health (T10).

**A03-3. Guts boosts special moves.** It should boost Attack only.

**A03-4. Mold Breaker (and Teravolt, Turboblaze) never gets past immunity abilities.** Affects Insomnia,
Magma Armor, Purifying Salt, Sweet Veil, Hyper Cutter and Aroma Veil. Same family as A06-6, A05-6, F13.

**A03-5. Abilities that do nothing:** Snow Cloak (F6), Tangled Feet (F6), Mimicry (type follows the
terrain), Cud Chew (eats its berry a second time), Flower Veil, and Rivalry (the engine has no gender).

**A03-6. Effect Spore has the wrong chances.** It should be 11% sleep, 9% poison, 10% paralysis; the engine
rolls three separate 9% chances. It also ignores powder immunity and triggers through a Substitute.

**A03-7. Shed Skin always cures; Hydration and Shed Skin cure after the poison or burn damage.** Same as
T11.

**A03-8. Sand Rush and Sand Force holders still take sandstorm damage.** They should be immune. Sand Rush
also ignores Cloud Nine (A04-2).

**A03-9. Armor Tail does not block priority status moves.** It should block every priority move aimed at
the holder, including Prankster's.

**A03-10. Smaller faults:** Dry Skin takes no sun damage (T10); Damp only works on the defender's side
(M05-14); Gluttony only affects Sitrus Berry; Stakeout (double damage to a Pokémon that just switched in)
misses replacements after a faint; Dragonize has no list of moves it should leave alone.

### Abilities rows the audit skipped

**X3. Ice Body was not audited** in slice 03 (row 160; the reader wrongly reported that the list ended at
row 159). Audited since. Ice Body heals a sixteenth of max HP each turn in snow. Three faults: it never
heals in Champions, because the engine only checks for hail (C11); Heal Block does not stop it; and it heals
after poison and burn damage where Showdown heals before (T1). The amount, the full-HP case and Cloud Nine
are right.

### Abilities slice 01, the most used (A01): 18 of 53 match

**A01-1. Prankster is blocked far too much.** Against a Dark-type opponent the engine cancels every status
move a Prankster user makes, including ones aimed at itself or the field: Protect, Recover, Swords Dance,
Tailwind, Reflect, weather and hazards. Only moves aimed at the Dark type should fail. Wider than M06-4.

**A01-2. Disguise breaks too early.** Same as SP11, plus: it also removes the attack's side effects and
blocks every hit of a multi-hit move.

**A01-3. Protean and Libero are wrong.** These change the user's type to match the move it uses. A
two-type holder is not changed when one type already matches, and the change happens even when the user is
asleep or flinches.

**A01-4. Abilities that do nothing:** Emergency Exit (T12), Cursed Body (may disable a move that hits the
holder), Aura Guard (SP4), Stance Change (SP7), Steadfast (Speed up on a flinch), Unburden (C3).

**A01-5. Magic Bounce does not reflect stat-lowering moves.** It should bounce status moves back at the
user.

**A01-6. Speed Boost triggers on the entry turn.** Same as T9.

**A01-7. Mold Breaker is ignored in many places.** Sturdy, status immunities, stat-drop immunities, flinch
immunity, Contrary, Battle Armor and Shell Armor. Same family as A03-4, A05-6, A06-6, F13.

**A01-8. Thermal Exchange and Justified trigger on status moves.** They should raise Attack only when hit
by a damaging Fire move (Thermal Exchange) or Dark move (Justified).

**A01-9. Rough Skin ignores Magic Guard.** Same family as A02-7, A04-3, A05-8.

**A01-10. Missing move tags.** Double Shock has no punch tag (Iron Fist does not boost it); Crush Claw and
Dire Claw have no slicing tag (Sharpness does not boost them). Same as M03-5, M03-7, M08-10.

**A01-11. Intimidate ignores the abilities that react to it.** Guard Dog (A05-5), Mirror Armor (bounces the
drop back) and Rattled (Speed up).

**A01-12. Multiscale halves every hit of a multi-hit move.** It should halve only while the holder is at
full HP, so only the first hit.

**A01-13. Contact abilities roll once per move, not once per hit.** Static, Flame Body and similar, and
Weak Armor, should get a chance on each hit of a multi-hit move.

**A01-14. Weather and terrain abilities always give five turns.** The extending items do not exist (C4).

**A01-15. Waiting on the core rewrite:** Pressure (extra PP use).

### Found by the data comparison by program (step 3), not by the readers

**X4. Dragon Cheer is a 60-power attack in the engine.** In the game it is a doubles support move that
raises a partner's crit chance and does nothing useful in singles. The engine gives it power 60, no sound
tag and no effect. The slice 08 reader marked it as matching. This is the one data error out of 23 moves
with data errors that the first read missed.

### Added or settled by the second read (step 4a)

**SR-9. Synthesis has the same weather faults as Moonlight and Morning Sun (M02-10).** The first reader
marked Synthesis as matching. It ignores Cloud Nine and Air Lock cancelling the weather, and heals the wrong
amount in harsh sun.

**SR-10. Avalanche is settled (was M03-14, undetermined).** It doubles if the user was hit first that turn.
The engine also doubles when the opponent only hit the user's Substitute, which should not count.

**SR-11. Defog does not lower the target's evasion.** It should lower it by one stage. Adds to M05-12; it
has no effect until evasion exists at all (F5).

**SR-12. Earthquake and Surf are part of M08-5.** They should hit a Pokémon using Dig or Dive for double
damage. The first readers of those two rows marked them as fine.

**SR-13. Doubles-only moves do not count as failed.** Helping Hand and Follow Me do nothing in singles in
both engines, but Showdown records them as failed moves and the engine does not. That only matters for moves
that react to a failure (Stomping Tantrum, Temper Flare), which the engine does not handle yet (M06-2,
M04-6).

**SR-16. Type-absorbing abilities cancel moves that are not aimed at the holder.** Water Absorb, Storm
Drain, Dry Skin, Volt Absorb, Lightning Rod, Motor Drive, Sap Sipper, Flash Fire and Earth Eater should only
absorb a move aimed at the holder. The engine checks the move's type and nothing else (I read the code:
`abilities.rs` Lightning Rod, Earth Eater and Water Absorb arms). So the opponent's Spikes (a Ground move) is
absorbed by Earth Eater and heals it, Rain Dance heals a Water Absorb holder and sets no rain, and Sunny Day
gives a Flash Fire holder its boost. The first readers marked these abilities as matching. Medium
confidence on exactly which field and self-targeted moves reach this code.

**SR-17. Absorb abilities give no boost behind a Substitute.** Lightning Rod and Sap Sipper should raise the
holder's stat when they absorb a move. The engine treats the boost as coming from the opponent, so a
Substitute or Clear Amulet on the holder blocks it. Same cause as A06-5.

**SR-18. Eerie Impulse fails against Ground types.** Status moves ignore type immunity (Thunder Wave is the
exception). The engine blocks every Electric move aimed at a Ground type, status moves included (I read the
code: `move_has_no_effect`). For the same reason Supercell Slam does not hurt its user when it fails against
a Ground type, where it should take half its max HP.

**SR-19. Salt Cure lands through a Substitute.** Its lasting damage effect should be blocked when the hit
lands on a Substitute. Same family as M03-11 and M08-12.

**SR-20. Sound moves' stat drops are blocked by a Substitute.** Snarl's damage goes through a Substitute
(sound moves do), but its Special Attack drop is wrongly blocked. Same family as M07-7.

**SR-21. Limber and Water Bubble do not cure a status the holder already has.** They should cure paralysis
(Limber) or a burn (Water Bubble) gained before the ability was active, for example after Skill Swap. Rare.

**SR-22. The three Pokémon not brought count as fainted.** At team preview the engine marks the three
unselected Pokémon as fainted (I read the code: `TeamPreviewFaintIndex`), and its count of fainted teammates
counts every team slot at 0 HP. So Last Respects (power 50 plus 50 for each fainted teammate) starts the
battle at 200 power where it should be 50, and Supreme Overlord (10% more power per fainted teammate) starts
at 30% where it should be nothing. Both first readers and one second reader marked these as matching, with a
note that it depended on how the team is stored. Last Respects is among the 60 most-used moves.

**SR-23. Good as Gold blocks field moves.** It should only block status moves aimed at the holder. The
engine also blocks moves like Haze and Perish Song that affect the whole field. Low confidence.

**SR-24. Stamina's Defense boost is blocked by Clear Amulet.** Same cause as A06-5.

**SR-25. Queenly Majesty blocks hazards set with priority.** It should block priority moves aimed at the
holder only. The engine also blocks a Prankster user's Stealth Rock or Spikes, which are aimed at the
holder's side of the field. Same family as SR-16 and SR-23: the engine does not tell "aimed at the Pokémon"
from "aimed at its side or the field".

**SR-26. A Choice lock outlives the Choice item.** Choice Scarf, Band and Specs lock the holder into one
move. If the item is then removed (Knock Off) or swapped away (Trick), the lock should end. The engine keeps
the other moves disabled until the Pokémon switches out, so a Scarf user that Tricks its Scarf away is stuck
using Trick. The first reader marked Choice Scarf as matching.

**SR-29. Light Ball doubles the wrong thing for two moves.** It should double Pikachu's Attack and Special
Attack. The engine doubles the power of Pikachu's moves instead, which is the same for normal attacks but
wrong for Foul Play (uses the target's Attack) and Body Press (uses Defense). Part of F1.

### From the four unanswered upstream pull requests (read 17 Nov, `docs/investigation/upstream-prs/`)

None is taken as a patch: three change code that the core rewrite replaces, and the fourth changes data that
will be generated from Showdown. They are kept as references.

**X5. On a speed tie, the first mover's boosts are applied twice** (pull request 31, not verified by us). When
two Pokémon have the same speed the engine works out both orders, and the second order starts from choices
the first already changed. Tough Claws or Life Orb's 1.3 is applied twice, and the other side is treated as
having moved second (no flinch, Sucker Punch failing). The readers did not find this. Closed by W1.

**X6. After Outrage ends, the other moves may stay disabled** (pull request 41, not verified by us). The
pull request re-enables them, which suggests the engine leaves them disabled. Conflicts with T6, which says
a locked Pokémon can still choose other moves. To be settled when W7 is built.

**X7. Pull request 36 (Champions data) fixes only things already in this register:** Make It Rain,
Crabhammer, Syrup Bomb, Swagger, Slash, Meteor Assault, Growth, the four Mega abilities, the two Mega stats
and Aura Guard. Its list of unimplemented moves (Heal Pulse, Magic Powder, Revival Blessing, Simple Beam,
Soak, Speed Swap, Transform, Worry Seed) is also already here. It names Metronome too, which is not legal in
Champions.

**X8. Pull request 29 (Struggle)** adds Struggle when no move has PP. Reference for W7.

### Found by the first run against Showdown (step 7), missed by the audit

500 random battles, 17 Nov (`tools/rig/results/2026-11-17-first-run.md` and `-triage.md`). Of the 81
distinct causes behind that run's differences, 69 were already in this register and these 12 were not. Step
ids such as 1:5:14 reproduce with `tools/rig/driver.py --seed 1 --start 5 --battles 1`.

**N1. A move counts as used before the Pokémon is able to move.** The engine records the last move, and
starts a Choice lock, before it checks sleep, freeze, paralysis, flinching and confusion. A Pokémon that was
asleep all turn is then treated as having used the move: it can be Encored into it and is locked into it by
a Choice item. Confirmed in the engine code (`generate_instructions.rs`, the last move is set before the
checks). 237 steps, the largest single miss. Belongs to W4. Example 1:5:14.

**N2. Outrage locks the user even when it failed.** If the target is immune or the move is disabled, the
user should not be locked in. 8 steps. W7. Example 1:14:12.

**N3. PP is charged twice for two-turn and locked moves.** The second turn of Solar Beam, and each further
turn of Outrage, should not cost PP again. 17 steps. W7. Example 1:39:11.

**N4. PP goes wrong in two edge cases.** With Pressure it can drop to -1; and a move used after the opponent
has already fainted costs no PP when it should. 5 steps. W7. Examples 1:386:41, 1:88:6.

**N5. A failed Protect keeps its counter.** Using Protect twice in a row may fail; when it fails the chance
should reset. 3 steps. W4. Example 1:282:54.

**N6. Pain Split can raise HP above the maximum.** 2 steps. W28. Example 1:110:0.

**N7. Pain Split works against Good as Gold.** Good as Gold should block it. 1 step. W12. Example 1:183:6.

**N8. Spicy Spray burns the user of a status move.** It should only burn a Pokémon whose attack dealt
damage. 2 steps; not confirmed in the engine code. W12 (same family as A06-4). Example 1:79:11.

**N9. After the last opposing Pokémon faints mid-turn, the winner's waiting move still runs.** The battle
should end at once. 10 steps. W8 (same family as T8). Example 1:136:25.

**N10. Trace copies Disguise.** Trace should not be able to copy it. 2 steps. W29. Example 1:218:2.

**N11. Double Shock leaves the user Electric type.** The user should lose its Electric type after using it.
1 step. W28. Example 1:480:6.

**N12. Mega Evolution is offered to a Pokémon locked into Outrage.** 3 decisions. W7. Example 1:155:7.

**N13. Move order on the turn a Pokémon Mega Evolves** had no id of its own (it was half a sentence in T18).
The engine uses the speed from before the Mega Evolution. Known core difference, W1. Examples 1:207:7,
1:21:10.

Not settled by the run (16 steps): the engine seems to play the opponent's waiting move after a pivot move
in 7 steps; Parting Shot and Strength Sap stat stages; No Retreat's trapped state; five Make It Rain damage
values outside Showdown's range.

**N14. A recharge turn is played during the opponent's replacement.** After Hyper Beam the user must spend
its next turn recharging. When the opponent's Pokémon faints and it picks a replacement, the engine treats
that replacement step as the recharge turn, so the user is free to attack on the following turn. Found when
a suspected rig fault was traced into the engine (`state.rs` gives the waiting side an empty choice, which
the engine plays as the recharge). W4. Example 1:393:3. Corrects the confidence on X1 (Hyper Beam), which
the audit marked as matching.

**Probability check (second run).** Where the engine has an outcome exactly equal to Showdown's, its
probability was compared with Showdown's, damage rolls aside: 1,973 of 2,352 steps agree and 379 do not.
Most of the 379 are outcomes the engine merges into one; 7 steps where Showdown had no random event at all
have an engine probability of 0.667, which is unexplained. Closed by W1, when every event is drawn singly.
