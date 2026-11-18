# Players spec: max damage, tower, and the shared damage estimate

Read-only reading of the Python players, written 17 Nov 2026 for the step-10 coder. Nothing here is code.
Ids: MD-n and T-n are rules (max damage, tower), PS-n are things a Battle will not have, PS-B n are
suspected bugs or accidents, PS-Q n are open decisions for the user.

Source key (all paths under `C:\Users\user\Code\pokemon_champions\`):

| Short | File |
|---|---|
| md | `pcsa\src\pcsa\players\max_damage.py` |
| ta | `pcsa\src\pcsa\players\tactics.py` (the estimate max damage uses) |
| tw | `research\pcsa-rl\rl2\opponents\tower.py` |
| sw | `research\pcsa-rl\rl2\opponents\switching.py` (optional pivot rule) |
| st | `research\pcsa-rl\rl2\stochastic.py` (`reasonable`, the validity filter tower calls) |
| ds | `pcsa\src\pcsa\players\disciplined.py` (`RECOVERY`, `is_setup`) |
| th | `pcsa\src\pcsa\players\threat.py` (the estimate tower uses) |
| dm | `pcsa\src\pcsa\battle\damage.py` (formula behind `th`) |
| op | `pcsa\src\pcsa\players\opponent.py` (assumed set, fighters, speed) |
| sd | `pcsa\src\pcsa\battle\stats.py` (stats, known ability) |
| pe | poke-env `move.py` in `pcsa\.venv\Lib\site-packages\poke_env\battle\` |

Two facts that change the brief:

1. **The two players do not share an estimate today.** Max damage scores with `ta.attack_score` (a ratio
   heuristic with no level term, no weather, no items, no burn). Tower scores with `th.hit_fraction`, which
   runs `dm.damage` (Showdown's integer formula with many modifiers) over an assumed catalog set for the
   opponent. Section 3 describes both. Step 10's "one estimate shared by both" is therefore a change for max
   damage, see PS-Q1.
2. `no_effect.py` is **not used by either player.** Its only importer is `neural_rules.py`
   (`pcsa\src\pcsa\players\neural_rules.py:19`). Neither tower nor max damage checks immunity to a status
   move, a powder move, Prankster and so on (PS-B4). The Tracker (`op.py:277`) is also unused by tower: its
   calls to `th` pass no tracker (`tw.py:122,129,134`). Only `disciplined` uses it.

---

## 1. Max damage

Class `MaxDamagePlayer` (`md.py:19`). Constant `STAB = 1.5` (`md.py:16`). Scoring is
`ta.attack_score(battle, attacker, defender, move, Scoring(stab, exact=True, form))` (`ta.py:94`),
section 3A. "Best attack" of a Pokémon against a defender = the maximum score over its non-status moves,
0.0 if it has none or the defender is None (`md.py:33-37`).

Input to every step is the legal-order list (`view.legal`, from poke-env `battle.valid_orders`,
`pcsa\src\pcsa\players\mirror.py:54-64`): move orders (a plain order and, when Mega is on offer, a second
order with `mega=True` per move) and switch orders. The order of that list is poke-env's and I did not read
it (uncertain, see PS-B3).

**Choose action** (`md.py:48-67`):

- MD-1. No legal action: raise (`md.py:49-50`). CG: cannot happen; assert.
- MD-2. `moves` = legal orders whose payload is a Move. If there is at least one move, and both actives
  exist (`md.py:55-56`): pick `max(moves, key=score)` (`md.py:62`), where
  `score = attack_score(battle, active, opponent_active, move, Scoring(1.5, True, form))` and `form` is the
  Mega form of the active (`mega_form_of`, `ta.py:33`) if that order has `mega=True`, else None
  (`md.py:58-60`). Status moves and zero-power moves score 0 (section 3A) so they are chosen only when every
  option scores 0.
- MD-3. Mega: no separate rule. A Mega order is just another candidate, scored with the Mega form's stats
  and types (`ta.py:42-48`). The docstring says "Mega Evolution when it is offered" (`md.py:3`); the code
  makes that true only because Mega stats usually score at least as high (PS-B1).
- MD-4. Tie-break: Python's `max` returns the first maximal element in `view.legal` order. No other
  tie-break exists (no PP, no random). Order unknown (PS-B3).
- MD-5. No voluntary switch: if any move is legal, switches are never looked at (`md.py:55-62`).
- MD-6. Forced switch (no move legal, switches legal, opponent active exists; `md.py:63-66`): pick
  `max(switches, key=best_attack(member, opponent_active))`, scoring with `Scoring(1.5)` = own exact stats,
  current form (no Mega consideration), HP of the member ignored, damage taken ignored. Tie: first in legal
  order.
- MD-7. Otherwise return `view.legal[0]` (`md.py:67`).
- MD-8. No Battle: `_from_screen` (`md.py:69-79`). Not needed in CG (always has a Battle).

**Team preview** (`md.py:39-46`):

- MD-9. `team` = own six in Battle order; `opponents` = `battle.teampreview_opponent_team` if non-empty, else
  `opponent_team` values (`md.py:44`). Per own mon: `score = sum over opponents of best_attack(mon, opp)`
  (`md.py:45`), with `Scoring(1.5)`, current form, defender at preview stats (species only: no ability, no
  item, no boosts, midpoint stats, section 3A). Picks = indices sorted by descending score, stable (ties
  keep the earlier team index), first `battle.max_team_size or 3` (`md.py:46`). The first pick is the lead.
  Mega is not considered at preview (a stone holder is scored in its base form).
- MD-10. If `battle is None`: random three (`md.py:41-42`). Not needed.

Constants: STAB 1.5; picks 3 (`max_team_size or 3`). Everything else is inside `attack_score` (3A).

Count: 10 rules listed, 7 that matter (MD-2, 3, 4, 5, 6, 7, 9).

---

## 2. Tower

Class `TowerPlayer(MaxDamagePlayer)` (`tw.py:37`), "after poke-dojo's Battle Tower AI" (`tw.py:1`). Uses a
`random.Random` (`tw.py:42`): every `rng.choice` is uniform. CG must use its seeded RNG.

**Team preview** T-0: identical to max damage (`tw.py:50-51`, MD-9).

**Choose action** (`tw.py:53-94`), first rule that fires wins:

- T-1. Fallback: no Battle, or either active is None -> max damage's `choose_action` (`tw.py:55-56`).
- T-2. Perish escape (`tw.py:58-60`): `switches` = legal orders that are not moves. If the active has the
  Perish-count-1 volatile (`Effect.PERISH1`) and `switches` is non-empty: uniform random switch. No
  smartness. (Intent: leave before fainting. Count 1 is the last turn before the faint, so it is right.)
- T-3. Optional pivot-switch rule (`tw.py:61-67`), only when `switching=True`. Off by default (`tw.py:40`).
  Described in section 2B. PROJECT.md step 10 leaves it out unless asked for.
- T-4. Usable moves (`tw.py:68`): `moves` = legal move orders that pass `_usable` (`tw.py:97-107`). A move
  fails if any of:
  1. status category and the active has Taunt (`tw.py:99`);
  2. `dreameater` and the opponent is not asleep (`tw.py:101`);
  3. id in `reflect lightscreen auroraveil` and that side condition is already on our side (`tw.py:103`);
  4. id in RECOVERY and own HP fraction >= 0.99 (`tw.py:105`);
  5. `st.reasonable` is False (`tw.py:107`), the four checks below.

  RECOVERY (`ds.py:31-33`): `recover roost slackoff softboiled milkdrink shoreup wish synthesis moonlight
  morningsun`. (Rest is not in it.)

  `reasonable` (`st.py:34-53`), in this order, any return ends it:
  1. id in HEALS = RECOVERY minus `wish`, plus `rest` (`st.py:20`) and own HP >= 0.99 -> False (`st.py:40`);
  2. id in FIRST_TURN `fakeout firstimpression matblock` (`st.py:25`) and the active is not on its first
     turn out -> False (`st.py:42`);
  3. id in HAZARDS (`st.py:21`): `stealthrock` max 1, `spikes` max 3, `toxicspikes` max 2, `stickyweb` max 1;
     returns True only if the opponent side lacks it, or has it with fewer layers than the max for the
     layered ones (`st.py:44-47`);
  4. id in STATUS_MOVES (`st.py:23`): `yawn thunderwave willowisp toxic poisonpowder sleeppowder spore
     hypnosis glare stunspore nuzzle`: returns True only if the opponent has no status, and (for yawn) no
     Yawn volatile (`st.py:48-49`);
  5. `is_setup(move)` (`ds.py:66-69`, definition below): returns True unless every stat the move raises is
     already +6 (`st.py:50-52`, `MAX_STAGE = 6`);
  6. otherwise True.
- T-5. Classify each usable move with `dmg = _damage(...)` = `th.hit_fraction` (exceptions give 0.0)
  (`tw.py:69-83`):
  - status category (`tw.py:73-75`): goes to `high` if `_high` is True, else `status`; never scored;
  - else, in this order, a move may land in several lists:
    a. `dmg >= opponent HP fraction and dmg > 0` -> `ko` (`tw.py:76`). `dmg` is the mean-roll, accuracy- and
       hit-weighted expected fraction of the defender's max HP (3B). Opponent HP fraction is from percent.
    b. id in SPEED_DOWN and we are not strictly faster -> `high` (`tw.py:78`);
    c. id in FIRST_TURN -> `high` (`tw.py:80`);
    d. strictly `dmg > best_dmg` (start 0.0) -> new `best` (`tw.py:82`). First one wins ties; zero-damage
       moves never become `best`.
- T-6. `_high` for a status move (`tw.py:109-116`), in order: id in SPEED_DOWN and not strictly faster ->
  True; `is_setup(move)` -> True iff we survive (`_survives`: the opponent cannot KO us, T-9); id in INFLICT
  -> True iff the opponent has no status or id in `confuseray swagger attract infestation`; else True iff id
  in SCREENS or FIRST_TURN. A move in INFLICT returns inside its branch (never reaches the last line).
- T-7. KO (`tw.py:84-86`): if `ko` non-empty: `fast` = those with priority > 0 (`priority()` = poke-env move
  priority, 0 if missing, `tw.py:24-29`); uniform random choice of `fast` if non-empty else `ko`.
  (Intent: poke-dojo's "KO with priority first".) A Mega variant and the plain variant of the same move are
  separate entries (PS-B1).
- T-8. High (`tw.py:87-88`): else if `high` non-empty: uniform random among `high + [best]` (`best` appended
  once if it exists; if it is already in `high` it has double weight).
- T-9. Status/best (`tw.py:89-91`): else `pool = status + [best]`; if non-empty, uniform random choice.
  `status` is every usable status move not in `high`, including every hazard, Protect, Substitute,
  Recover, weather moves, etc., each with the same weight as the best attack.
- T-10. Smart switch (`tw.py:92-93`): else if `switches` non-empty: weight each by the number of its
  non-status moves (all four of the bench member, known) with `opponent_active.damage_multiplier(move) > 1`
  (type chart only, `tw.py:136-142`); uniform random choice from the list in which each switch appears
  `hits` times, or uniform over all switches if every weight is 0. This is also the **forced switch** rule
  (when `moves` is empty because only switches are legal).
- T-11. Last resort: max damage's `choose_action` (`tw.py:94`).
- T-12. `_survives` (`tw.py:131-134`): `not can_ko(opponent -> us)` where `can_ko` (`th.py:109`) is True if
  any of the opponent's *known* moves has `ko_share > 0 and ko_share >= our HP fraction`, `ko_share` for a
  hit on us = max roll x the move's maximum hit count, no accuracy (`th.py:63-68`).
- T-13. `_faster` (`tw.py:126-129`): `th.outspeeds` (`th.py:120`): strictly greater effective speed (strictly
  smaller under Trick Room). Equal speed -> not faster for either side. Effective speed (`op.py:215-226`):
  base stat x item multiplier (only Choice Scarf 1.5 matters, `rules.json` `known_stat_multipliers`) x 2 for
  a revealed weather-speed ability in matching weather (swiftswim rain/primordialsea, chlorophyll
  sun/desolateland, sandrush sandstorm, slushrush snowscape/hail, `op.py:36-41`) -> stage `(2+s)/2` or
  `2/(2-s)` with integer floors (`sd.py:137-141`) -> x1.5 if quickfeet and statused, else x0.5 if paralysed
  -> x2 if Tailwind on that side.

**Move lists, as Showdown ids**

- SPEED_DOWN (`tw.py:15-16`): `icywind electroweb rocktomb bulldoze mudshot scaryface stringshot
  cottonspore glaciate lowsweep thunderwave nuzzle`.
- INFLICT (`tw.py:17-19`): `thunderwave willowisp toxic poisonpowder sleeppowder spore hypnosis glare
  stunspore yawn confuseray swagger flatter supersonic sweetkiss teeterdance attract nuzzle infestation
  whirlpool firespin sandtomb magmastorm`. Of these `nuzzle infestation whirlpool firespin sandtomb
  magmastorm` are damaging and never reach `_high` (PS-B5).
- SCREENS (`tw.py:20`): `reflect lightscreen auroraveil`.
- FIRST_TURN (`tw.py:21`, and `st.py:25`): `fakeout firstimpression matblock`.
- RECOVERY: above (T-4). HEALS: RECOVERY minus `wish` plus `rest`.
- Hazards with caps: `stealthrock 1, spikes 3, toxicspikes 2, stickyweb 1`.
- STATUS_MOVES (reasonable): above (T-4.4).
- `is_setup` (`ds.py:66-69`): `bellydrum`, or any move whose target is `self` and whose boosts dict has a
  positive sum. Not a list in the code. From poke-env's gen9 data this gives 38 status moves: `acidarmor
  agility amnesia autotomize barrier bulkup calmmind charge clangoroussoul coil cosmicpower cottonguard
  defendorder defensecurl doubleteam dragondance extremeevoboost filletaway geomancy growth harden honeclaws
  irondefense meditate minimize nastyplot noretreat quiverdance rockpolish sharpen shellsmash shelter
  shiftgear swordsdance tailglow victorydance withdraw workup` plus `bellydrum`. (I ran that predicate on
  poke-env's data, not on the Champions data; regenerate from CG's table.)
- Not in any list, so they sit in the plain `status` pool: Protect, Substitute, Roost-likes not in RECOVERY
  (`rest`, `healingwish`...), `wish` (RECOVERY), phazing, `taunt`, `encore`, `trick`, `teleport`, `haze`,
  weather and terrain setters, `perishsong`, `destinybond`, and so on, with no extra condition.

**Thresholds** (tower): KO test `dmg >= HP fraction` (mean, not min); recovery/usable HP cutoff 0.99 (>=);
Perish count 1; no other numeric constant.

### 2B. Optional pivot-switch rule (`sw`, off by default)

Applies at T-3 before any move filter, only if enabled. Constants: `BAD_HIT 0.5`, `OUR_WEAK 0.25`,
`SPARE 0.15`, `THREAT_W 0.5`, `KEEP 0.6`, `P_SWITCH 0.85`, PIVOTS = `uturn voltswitch flipturn partingshot
teleport chillyreception shedtail` (`sw.py:14-20`).

1. `their_hit, their_move = best_hit(opp -> us)` (expected fraction, `th.py:84`).
2. If we can KO them now (`can_ko(us -> them)`, mean roll expected hits): not bad, attack (`sw.py:33-34`).
3. `bad = they_ko or (their_hit >= 0.5 and our_best_hit < 0.25)` (`sw.py:35-37`). Not bad -> no switch.
4. Fires with probability 0.85, otherwise no switch (`sw.py:78`).
5. Pivot: if (we strictly outspeed them or `their_hit < our HP fraction`) and a legal PIVOTS move exists, that
   move (first in legal order) (`sw.py:41-51`).
6. Else best bench member by `(hp - taken) + 0.5 * its best hit on them`, where `taken` = their predicted
   move's expected hit on the member; members with `taken >= hp - 0.15` or `taken >= 0.6 * their_hit` are
   skipped (`sw.py:54-69`). None qualifying -> no switch.

It uses the opponent's best known move (`known_moves`, which fills unseen moves from the catalog: PS-2).

Count: tower rules T-0 to T-13 = 14 numbered rules (T-3 optional, T-12 and T-13 are helpers of T-6). Rules
that decide the action in priority order: T-1, T-2, (T-3), T-7, T-8, T-9, T-10, T-11 = 7 (plus the T-4
usable filter and T-0 preview).

---

## 3. The damage estimate

### 3A. Max damage: `attack_score` (`ta.py:94-125`)

Returns a **unitless mean score** (not a percent, not a range):

```
score = power x stab_mult x ratio x accuracy x expected_hits x type_mult
ratio = atk_stat x atk_stage_mult / max(1, def_stat x def_stage_mult)
```

- Category: physical -> atk/def, else spa/spd (status moves fall to spa/spd with power 0, score 0)
  (`ta.py:101`). `bodypress` uses the attacker's `def` as attack (`ta.py:102-103`). `foulplay` uses the
  defender's `atk` and the defender's atk stage (`ta.py:104-105`).
- Stage multiplier (`ta.py:51-53`): `(2+v)/2` for v >= 0, `2/(2-v)` for v < 0, floats, on the attack stat of
  the source and the defense stat of the defender. No clamp, no crit handling.
- Attacker stats: own exact stats (`sd.current_stats(exact=True)`), or the Mega form's when the Mega order
  is scored (`ta.py:42-48`).
- Defender stats if it is not on our team: **midpoint of the species' minimum and maximum**:
  min = 0 points with a lowering nature (x90/100), max = 32 points with a raising nature (x110/100),
  `(low + high) / 2` per stat as a float (`sd.py:80-85,125-130`, `st.py` n/a). Stat formula
  `((2 base + 31 + 2 points) x 50 // 100 + 5) x nature // 100`, HP `(... ) + 50 + 10` (`sd.py:19-26`).
- Type multiplier: poke-env `defender.damage_multiplier(move)` (type chart on the defender's current
  types), then set to 0 if the defender's *revealed* ability is a type-absorb: ground/levitate; electric/
  voltabsorb lightningrod motordrive; water/waterabsorb stormdrain dryskin, or if ground and the defender
  holds `airballoon` (`ta.py:14-20,111-116`). Nothing else (no Earth Eater, Flash Fire, Sap Sipper,
  Thick Fat, Gravity, Mold Breaker).
- STAB: `stab` (1.5) if the move's type is in the attacker's types, else 1. No Adaptability, no
  type-changing abilities.
- Power (`effective_power`, `ta.py:72-91`): `base_power`, except: `grassknot lowkick` by the defender's
  weight `<10 kg 20, <25 40, <50 60, <100 80, <200 100, else 120` (`ta.py:14,74-76`); `heavyslam heatcrash`
  by weight ratio `>=5 120, >=4 100, >=3 80, >=2 60, else 40` (defender weight floored at 0.1 kg);
  `facade` x2 if the attacker has a status; `electroball` by speed ratio `>=4 150, >=3 120, >=2 80, >=1 60,
  else 40`; `gyroball` `min(150, 25 x def_speed / our_speed + 1)`. Speeds here are raw stats, no stages.
  Moves with base power 0 that are not status (Seismic Toss, Night Shade, Super Fang, OHKO moves, Counter)
  score 0.
- Accuracy (`effective_accuracy`, `ta.py:56-69`): 1.0 if no guard on either side (but see PS-B2); `thunder
  hurricane` 1.0 in rain/primordialsea, 0.5 in sun/desolateland; `blizzard` 1.0 in snowscape/hail; else
  `move.accuracy` x `(3+s)/3` (s >= 0) or `3/(3-s)` with `s = clamp(acc_stage - eva_stage, -6, 6)`; capped
  at 1.0. A move with no accuracy check has accuracy 1.
- Expected hits: poke-env `expected_hits` (`pe` move.py:307): `triplekick/tripleaxel` 1 + 1.8 + 2.43 = 5.23;
  2-5 hit moves 5/3 + 3/2 = 3.1667 (the 35/35/15/15 average); fixed multi-hit = n; else 1.

Ignored: weather damage modifiers, burn, screens, terrain, the attacker's item and ability (Choice,
Life Orb, Huge Power, ...), the defender's item and ability beyond the immunity list, crit, rolls, the level
term and the +2. It is a ranking heuristic: it orders moves of one attacker well enough, but its numbers
cannot be compared with HP.

### 3B. Tower: `th.hit_fraction` over `dm.damage` (`th.py:75-81`, `dm.py:162-304`)

Returns the **mean damage as a fraction of the defender's max HP**, times accuracy and expected hits
(`th.py:55-60`): `(low + high) / 2 x accuracy x hits`. Its `damage()` returns `(low, high)` fractions.
`hits` = `move.expected_hits`, except `tripleaxel triplekick` count 1 + 0.9 + 0.81 = 2.71 (`th.py:46-52`).
`accuracy` = `effective_accuracy` with the attacker's assumed ability (so No Guard works here).
Returns 0 when high <= 0 (immune or non-damaging).

Formula (`dm._rolls`, `dm.py:174-195`), level 50 (`game.LEVEL = 50`, `game.py:18`):

```
power  = max(1, int(base_power x power_mod x (1.2 if -ate conversion)))
attack = int(stage(atk) x stat_mult)      defense = stage(def) [x2 Fur Coat if physical]
base   = ((2*50//5 + 2) x power x attack // max(1, defense)) // 50 + 2     (22 x power x A // D) // 50 + 2
mod    = effectiveness x modifiers x defender_modifier           (floats, no 4096 chain)
low    = (base x 85 // 100) x mod
high   = base x mod
fraction = low / hp, high / hp                                   (hp = defender max HP)
```

`stage(v, s)` = `v x (2+s) // 2` or `v x 2 // (2-s)` (integer floor, `sd.py:137-141`).

Modifiers applied:

- Power: base power from the move table unless `move_power` overrides (`op.py:204-212`): the variable-power
  moves above (`grassknot lowkick heavyslam heatcrash facade electroball gyroball`), escalating
  `tripleaxel 40`, `triplekick 20` (mean per hit), and `knockoff` x1.5 if the defender has an item (assumed
  item for the opponent).
- Attacker ability on power: `toughclaws` contact 1.3; `ironfist` punch 1.2; `strongjaw` bite 1.5;
  `sharpness` slicing 1.5; `megalauncher` pulse 1.5; `punkrock` sound 1.3; `reckless` recoil moves 1.2;
  `sheerforce` moves with a secondary 1.3; `steelworker` steel 1.5; `transistor` electric 1.3; `dragonsmaw`
  dragon 1.5; `rockypayload` rock 1.5; `waterbubble` water 2.0; `technician` base power <= 60 x1.5;
  `sandforce` in sandstorm for rock/ground/steel 1.3 (`dm.py:30-52,225-238`). First matching rule only.
  `-ate` abilities (`aerilate` flying, `pixilate` fairy, `refrigerate` ice, `galvanize` electric) convert
  Normal moves and add x1.2 (`dm.py:49-50,180-188`).
- Attack stat: Choice Band x1.5 on atk, Choice Specs x1.5 on spa; `hugepower purepower` x2 atk;
  `hustle` x1.5 atk; `guts` with a status x1.5 atk; `solarpower` x1.5 spa in sun/desolateland
  (`dm.py:259-269`). Foul Play and Body Press as in 3A.
- Defense stat: Fur Coat x2 def. Not applied: Assault Vest, Eviolite, sand/snow defense boosts.
- Effectiveness: type chart x defender ability factor per move type (`DEFENDER_ABILITY`, `dm.py:60-77`:
  thickfat fire/ice 0.5, heatproof fire 0.5, waterbubble fire 0.5, purifyingsalt ghost 0.5, levitate and
  eartheater ground 0, flashfire and wellbakedbody fire 0, waterabsorb stormdrain dryskin water 0,
  voltabsorb lightningrod motordrive electric 0, sapsipper grass 0, fluffy fire 2.0); a not-grounded
  defender (flying type, levitate, airballoon) takes 0 from ground (`dm.py:198-211`).
- Modifiers (`dm.py:285-304`): weather (sun fire 1.5 water 0.5, desolateland fire 1.5, rain water 1.5 fire
  0.5, primordialsea water 1.5); terrain 1.3 for electric, grass, psychic moves from a grounded attacker;
  Misty Terrain dragon 0.5 vs a grounded defender; STAB 1.5 (2.0 Adaptability); burn x0.5 on physical
  unless Guts; defender's screens x0.5 (Reflect or Aurora Veil on physical, Light Screen or Aurora Veil on
  special); Life Orb 1.3; Expert Belt 1.2 when super effective; matching type item 1.2 (the 18 type items in
  `dm.py:78-97`).
- Defender modifier (`dm.py:272-282`): `filter solidrock prismarmor` x0.75 when super effective;
  `icescales` x0.5 vs special; `fluffy` x0.5 vs contact; `punkrock` x0.5 vs sound.
- Not applied: critical hits, spread, Protect, Sturdy, Multiscale, Disguise, Wonder Guard, Mold Breaker,
  Gravity, items on the defender, two-turn moves, recoil, Parental Bond, Technician with variable power,
  Tera.
- A branch for an unknown defender spread (`spread_known False`, `dm.py:166-171`) exists but no caller sets
  it; dead code. `fighters` always builds the opponent from an assumed catalog set (`th.py:26-30`).

**What it assumes for the opponent** (`op.py:96-128,185-190`): the species' first catalog build that fits
the revealed moves, item and ability (builds seen on the live ladder first); that build's nature, 66 stat
points, item, ability (the revealed one wins) and moves (the revealed ones first, filled to four from the
build). A species with no build: nature Hardy, 11 points in every stat, revealed item, revealed moves only
(`op.py:120-123`). A revealed Mega uses its form's single ability and its required stone. `Tracker`
corrections (Scarf, power item) are not applied by tower.

**Own side** (`op.py:175-182`): exact stats in the current form, own item, own ability, own boosts, status.

### 3C. What a constant-time estimate over a Battle needs

PROJECT.md D7 (line 42): "The players see their own side fully and only what the opponent has revealed;
unknowns are filled by a fixed constant-time rule." D4 (line 37): "no build catalogs, no probabilities over sets,
no damage ranges." Step 10 notes (line 212): "Unknown opposing stats use one fixed rule (neutral nature, no
stat points, no item). One constant-time damage estimate is shared by both players."

Proposed function, one per (attacker, move, defender): `estimate(battle, atk_side, atk_mon, move, def_mon) ->
{mean_hp, max_hp_hit, def_max_hp}` or percent forms of them, all integer arithmetic, no allocation, no
loops except a fixed small one over hit count:

1. Stats: for a **known** side (ours) use the stats the game shows (`SET.stats`); for the **opponent** use
   `((2 base + 31) x 50 // 100 + 5)` for each non-HP stat and `((2 base + 31) x 50 // 100 + 60)` for HP
   (neutral nature, 0 points), from the species or revealed-form table. Use the revealed form (Mega) for
   base stats and types. This gives a fixed `def_max_hp`.
2. Ability: our own, or the opponent's revealed one; the opponent's Mega form's single ability if its form
   is a revealed Mega; otherwise none (empty). Item: our own, or the opponent's revealed one that has not
   been consumed; otherwise none.
3. Run `dm._rolls`' integer steps (with the modifiers of 3B, minus any the user drops) giving `base`, `mod`;
   `low = (base x 85 // 100) x mod`, `high = base x mod`. In CG use the engine's own integer chain for the
   same terms where it exists; the estimate must not call the roll sampler.
4. Return `mean = (low + high) / 2` per hit, the maximum `high` for the KO-against-us check, and
   `def_max_hp`. Multiply by hits and accuracy only in the callers (the tower KO test uses mean x accuracy x
   expected hits against the opponent, and max roll x max hits, no accuracy, against us: `th.py:63-68`).
5. Comparison with HP: opponent current HP in points = `percent x def_max_hp / 100`; or work in percent
   of max HP throughout so the opponent's percent needs no max HP. Our own current HP percent comes from the
   Battle (rule 8, LIVE.md) and our max HP is exact from `SET.stats`.
6. Cache per (attacker, move, defender, field) as the observation builder will; the players may call the
   same function (PROJECT.md line 254).

**Inputs it reads, to be checked against the Battle:**

| Input | From | Battle has it? |
|---|---|---|
| Attacker species/form, types, level 50 | own: SET and `mega`/`form`/`types` events; opp: preview species, `mega`, `form`, `transform`, `types` | yes |
| Own stats, boosts, status, item, ability | SET stats; `boost`; `status`; `item` events | yes (Mega stats: PS-5) |
| Opponent base stats | species table | yes (table) |
| Opponent boosts, status, types, form | events | yes (accuracy and evasion stages: PS-10) |
| Opponent item, ability | revealed only | yes, as revealed |
| Move: type, category, power, accuracy, priority, flags (contact, punch, bite, sound, pulse, slicing, bullet), target, boosts, multihit, recoil, secondary | move table | yes if the generated table carries them (PS-11) |
| Defender species weight | species table | verify (PS-11) |
| Weather, terrain, rooms, Gravity | `weather`, `field` events | yes |
| Defender side's screens | `side` events | opposing: yes; own side's: wording unrecorded (PS-7) |
| Attacker burn, defender grounded | status; types, items, ability, Gravity, `volatile` (Magnet Rise, Smack Down) | yes |
| Expected hits, accuracy | table | yes |

---

## 4. What each player reads, against LIVE.md

Everything below is read at decision time from `battle` (and the legal list).

| Read | Python | In a Battle? |
|---|---|---|
| Own six: species, moves, stats, item, ability, HP, status, boosts | `battle.team` | Yes: `battle_start` SET (stats always; points and nature only when read) |
| Own HP fraction | `current_hp_fraction` | Yes, as percent (LIVE rule 8) |
| Opponent species, types, form, HP percent, status, boosts | `opponent_active_pokemon` | Yes |
| Opponent preview six | `teampreview_opponent_team` | Yes (`opp_preview`, null where unread: skip nulls) |
| Opponent revealed moves | `mon.moves` | Yes (`move` events) |
| Opponent revealed item, ability | `mon.item`, `known_ability` (from raw `-ability` lines, `sd.py:156-166`) | Yes (`item`, `ability`, `ability_change`, `mega_stone`) |
| Own effects: Taunt, Perish1 | `mine.effects` | Yes: `volatile` with Perish `detail` N; Taunt listed (LIVE section 4) |
| Opp effect: Yawn | `Effect.YAWN in theirs.effects` | **Unconfirmed** (PS-8) |
| Own side conditions, opponent side conditions with layer counts | `battle.side_conditions` | Partly (PS-6, PS-7) |
| `mine.first_turn` | poke-env | Needs the turns-on-field counter (PS-9) |
| Weather, fields (Trick Room, terrains, Gravity) | `battle.weather`, `battle.fields` | Yes |
| Tailwind per side | `side_conditions` | Yes (`side` event) |
| Speed stage, paralysis, quickfeet | boosts, status | Yes |
| Bench: HP, moves, types | `battle.team` | Yes (own) |
| Legal orders incl. Mega variants and switches | `battle.valid_orders` | Computed by CG from the Battle (PROJECT.md "Decided", line 238); not a Battle field to read. Whether Mega orders are separate is a CG interface choice (PS-12) |

Gaps and their simplest rule-preserving replacement:

- **PS-1. Opponent set from a catalog** (`th.fighters` -> `op.assumed`, `op.py:96-128`; `default_catalog`
  `op.py:67`): nature, 66 points, item, ability and unrevealed moves from build lists. D4 forbids it.
  Replace with: neutral nature, 0 points, no item, ability only if revealed (Mega form's single ability
  counts), as in 3C. Affects tower's damage (`hit_fraction`), `_survives`, `_faster`.
- **PS-2. Opponent's unrevealed moves** (`th.known_moves`, `th.py:33-38`, used by `_survives` via `can_ko`
  and by the switching rule): moves filled to four from the build. Replace with the revealed moves only. If
  none of them is damaging, the opponent cannot KO us, so setup counts as surviving (PS-Q2).
- **PS-3. Opponent stats in max damage:** midpoint of the min and max spread (`sd.py:125-130`). Replace with
  the same neutral/0-point rule as everything else (PS-Q1: this changes max damage's ranking slightly).
- **PS-4. Opponent speed:** catalog set including an assumed Choice Scarf (`op.py:215`), used by `_faster`
  and the Trick Room reversal. Replace with the neutral/0-point speed, no item; a Scarf is never revealed
  (LIVE section 6). Revealed weather-speed abilities and Tailwind stay.
- **PS-5. Own Mega stats:** `sd.our_stats` solves stat points and nature from the shown stats to compute
  Mega-form stats (`sd.py:29-53,106-122`). The Battle has `points` and `nature` only when they were read
  (LIVE section 2). Replacement: use them when present; otherwise recompute each stat from the shown stat
  with a neutral-nature inversion (`p = smallest points giving the shown stat`) and take Mega stat from
  that. Needed only for scoring a Mega order and for stats after Mega Evolution; after the `mega` event
  the game shows the new stats (verify whether the live SET refreshes them: unknown).
- **PS-6. Spikes and Toxic Spikes layers:** `st.py:44-47` reads `opponent_side_conditions[cond]` as a layer
  count. LIVE's `side` event has `start/end/acted` and no count; the builder must count repeated `start`
  events. Check that it does.
- **PS-7. Own side's conditions** (own Reflect, Light Screen, Aurora Veil for T-4.3; Tailwind for own speed):
  LIVE section 7.2 lists "our own side's screens, Tailwind and Safeguard on our side" as wording never
  recorded. If the Battle misses them, the "screen already up" check never fires and own Tailwind is not seen.
- **PS-8. Yawn volatile on the opponent** (`st.py:49`): LIVE's `volatile` list does not name it. If absent,
  Yawn can be repeated while the opponent is not yet asleep.
- **PS-9. `first_turn`** (Fake Out, First Impression, Mat Block; `st.py:42`): needs the Battle's
  turns-on-field counter (PROJECT.md line 252). Rule: first turn = entered this turn or the previous
  turn change, exactly as the engine would; confirm the definition with the builder.
- **PS-10. Accuracy and evasion stages:** read from `boosts` (`ta.py:68`). LIVE's `boost.stat` does not
  list which stats exist. Confirm `accuracy` and `evasion` are included, else treat as 0.
- **PS-11. Move and species data fields:** `base_power`, `priority`, `accuracy`, `n_hit`, `flags`, `target`,
  `boosts`, `recoil`, `secondary`, `weightkg`, Mega ability and stone. Python takes them from poke-env gen9
  data and `pcsa` assets, not Champions data. CG's generated tables must carry them; the coder checks each.
  The `is_setup` list above is from poke-env, not Champions.
- **PS-12. Mega orders:** Python's legal list holds a plain and a Mega order for each move. CG's action set
  is "4 moves, 4 moves with Mega, 3 switches" (PROJECT.md line 259). The spec below treats them the same
  way; confirm the order the coder produces them in (affects ties, PS-B3).
- **PS-13. Opponent ability under Mold Breaker, Neutralizing Gas, Gravity and the like:** not modelled in
  Python either; no gap, just do not add it.

Not a gap: opponent HP (percent in Python too, `current_hp_fraction`); the Tracker (unused by tower);
`no_effect` (unused).

---

## 5. Suspected bugs and accidents

- **PS-B1. Tower and Mega.** The legal list holds a Mega and a plain order for each move. Tower's `best`
  uses `dmg` with the current form, so both variants score the same, and strict `>` keeps whichever comes
  first (the plain one in Python's likely order), so Mega is chosen only by the uniform draws in T-7 to T-9,
  where both variants sit in the same list. Evident intent: use Mega when offered (max damage's docstring,
  `md.py:3`, and the `mega_first` helper in `ds.py:72-78` exists for exactly this and is not applied by
  tower). Max damage: the Mega order wins only because Mega stats score at least as high; it is not forced.
  Recommendation: always take the Mega variant when offered, as the docstring says (PS-Q3).
- **PS-B2. Attacker ability discarded in `attack_score`.** `attacker_profile` returns the Mega form's
  ability (`ta.py:97`), then `ability = known_ability(battle, defender)` overwrites it (`ta.py:112`), and
  that defender ability is passed to `effective_accuracy` as the attacker's (`ta.py:122`). Effect: the No
  Guard test is `noguard in (defender ability, defender ability)`, so the attacker's No Guard is ignored.
  Intent: pass the attacker's ability (as `th` does with `fa.ability`).
- **PS-B3. Ties and all-zero scores.** `max(moves, key=score)` returns the first maximal order in poke-env's
  order; when every move scores 0 (all immune, or only status moves) it returns the first order, which can
  be a status move or a Mega order. Tower's `best` also stays None when every damaging move is 0. Intent:
  any rule is fine, but fix one: lowest move slot, Mega variant first when offered. Order of poke-env
  `valid_orders` not read (uncertain).
- **PS-B4. No immunity checks for status moves.** Thunder Wave on a Ground type, Will-O-Wisp on Fire,
  Toxic on Poison/Steel, powder on Grass, Spore on Overcoat, Sleep Powder on Sap Sipper, status into
  Misty Terrain, Prankster into Dark, Good as Gold, Magic Bounce: tower picks them as `high` (T-6) because
  `reasonable` checks only the opponent's status. `no_effect.why` (`no_effect.py:138`) was written for this
  and is only used by the neural player. Intent of poke-dojo's rule: do not use a status move that cannot
  work. The simplest preserving fix: type-chart based immunity for status moves from the Battle only
  (Thunder Wave vs Ground, Will-O-Wisp vs Fire, Toxic vs Poison/Steel, powder vs Grass) (PS-Q4).
- **PS-B5. Dead and misfiled entries.** INFLICT contains damaging moves (`nuzzle infestation whirlpool
  firespin sandtomb magmastorm`) that never reach `_high`, which is only called for status moves (`tw.py:73-75`).
  `nuzzle` is also in `STATUS_MOVES` of `reasonable`, so a damaging move is dropped whenever the
  opponent is statused (`st.py:23,48-49`), which cuts the best attack. Intent: probably "do not inflict
  twice"; it should not remove Nuzzle's damage.
- **PS-B6. Repeated confusion and Attract.** `confuseray swagger flatter supersonic sweetkiss teeterdance
  attract` are `high` while the opponent has no major status (and `confuseray swagger attract infestation`
  always), with no check for existing confusion or infatuation (`tw.py:115`). Intent: apply a condition
  once.
- **PS-B7. Status pool is unfiltered.** T-9 gives every other usable status move (Substitute at low HP,
  Protect, phazing, `teleport`, `trick`, a second weather move, `haze`, `taunt` on an already taunted foe,
  `destinybond`, `perishsong` that the user cannot survive...) the same weight as the best attack. This
  matches a "random among reasonable" reading of poke-dojo, but is probably not the intent of a "Battle
  Tower AI" and is extremely random. Copy it as is unless the user asks otherwise; test it so it is not
  changed by accident (PS-Q5).
- **PS-B8. `best` double weight.** `high + [best]` can contain the same order twice (a speed-down damaging
  move that is also best), doubling its weight (`tw.py:88`). Probably an accident; keep unless asked.
- **PS-B9. Priority and KO use an expectation, not a roll.** `dmg` includes accuracy and the mean roll
  (85-100 average), so "KO" means expected damage >= current HP: a 70% move needs a 143% mean to count.
  Against the rule of poke-dojo (which used the true damage range) this is conservative. Keep: it is what the
  code does and no min/max rule exists in D4 ("no damage ranges").
- **PS-B10. Perish escape is random.** A random switch, not the safest (T-2). Intent from poke-dojo
  unknown; keep.
- **PS-B11. Smart switch ignores abilities and immunity.** It counts type-chart super-effective moves only,
  not damage or whether the switch-in survives (T-10). Keep.
- **PS-B12. Max damage's preview ignores Mega** and treats a stone holder as its base form (MD-9). Keep.
- **PS-B13. `_hits` in `th` and poke-env's `expected_hits` disagree for the escalating moves** (2.71 vs
  5.23); in `ta` the move is scored at base power 20 x 5.23, in `th` at the mean power. Max damage is
  ranking only, so this is cosmetic, but a shared estimate removes it.

---

## 6. Test list

Max damage:

1. Four attacks, one clearly higher estimate: picks it.
2. Two moves with equal estimate: picks the lower slot (the tie rule the coder fixes, PS-B3).
3. All moves immune to the foe (Ground vs Flying): picks by the fixed zero rule, never a switch.
4. Status move only plus attacks: attack even when the status move comes first.
5. Mega offered: picks the Mega variant of the best move.
6. Mega offered and the plain order scores higher (contrived): documented behaviour either way.
7. Never switches while any move is legal, even in a hopeless matchup.
8. Forced switch: picks the bench member with the highest best-attack estimate against the foe; ignores
   HP and the damage it would take.
9. Forced switch tie: first in the fixed order.
10. Preview: three highest summed best-attack scores over the six opponent species, lead first, ties keep
    the lower team index.
11. Preview with null opponent entries: skipped.
12. Opponent revealed Levitate: Ground move scores 0; unrevealed ability: scores normally.
13. Defender holds revealed Air Balloon: Ground scores 0.
14. Boosted foe defense lowers the estimate; own attack stage raises it.
15. No Guard on the attacker makes a 70% move score as 100% (fixes PS-B2).
16. Thunder in rain counts accuracy 1; in sun 0.5; Blizzard in snow 1.
17. Multi-hit 2-5 counts 3.1667 hits; Triple Axel 5.23 (or the shared rule).
18. Foul Play uses the foe's Attack.
19. Grass Knot by foe weight bands; Facade doubled when statused.

Tower, in priority order:

20. Perish count 1 with a legal switch: a switch is chosen, uniformly among the legal ones.
21. Perish count 1 and trapped (no switch): continues to the move rules.
22. A KO among the options: a KO is chosen even if a status move is available.
23. Two KOs, one with priority > 0: the priority one is chosen; none with priority: uniform among KOs.
24. KO means mean expected damage (accuracy x hits) >= opponent HP fraction.
25. Damage 0 never counts as a KO.
26. Slower than the foe, holding a damaging SPEED_DOWN move: it is in the `high` list.
27. Faster than the foe: SPEED_DOWN is not forced into `high`.
28. Equal speed counts as not faster; Trick Room reverses the comparison.
29. Fake Out on the first turn out: in `high`; second turn: dropped.
30. Setup move when the foe cannot KO us: in `high`; when the foe's revealed move can KO: in `status`
    (not `high`) or absent per T-6.
31. Setup move with every raised stat at +6: dropped.
32. Foe has no revealed damaging move: setup counts as safe (PS-Q2).
33. Thunder Wave on an un-statused foe, slower than us: `status`; slower than the foe: `high`.
34. Thunder Wave on a statused foe: dropped.
35. Toxic on a statused foe: dropped; Confuse Ray on a statused foe: `high` (and PS-B6 behaviour).
36. Taunted: every status move is dropped.
37. Dream Eater on an awake foe: dropped.
38. Reflect already up: dropped; Aurora Veil already up: dropped.
39. Recovery move at HP >= 0.99: dropped; at 0.98: kept in `status`.
40. Stealth Rock when the foe's side has it: dropped; Spikes with 3 layers: dropped; Spikes with 2: kept.
41. Yawn on a foe that has Yawn: dropped.
42. No KO, no `high`: uniform among status moves plus the best attack.
43. `high` non-empty: uniform among `high` plus best (best weighted twice when also in `high`).
44. Only status moves, none usable, switches available: smart switch.
45. Smart switch weights by the number of super-effective damaging moves; all zero: uniform.
46. Forced switch (only switches legal): smart switch.
47. Preview identical to max damage's for the same Battle.
48. Seeded RNG: same seed, same sequence of choices.
49. (Optional rule) Bad matchup with a pivot move and we are faster: pivot chosen with probability 0.85.
50. (Optional rule) We can KO now: no switch.

Estimate:

51. Known values: one Showdown-checked case per modifier (STAB, type, weather, burn, screen, Life Orb,
    Choice Band, Technician, Adaptability, terrain) against the 16-roll max and min.
52. Unknown opposing stats: the estimate uses neutral nature, 0 points, no item, and equals a hand-computed value.
53. Revealed Mega form changes the opponent's base stats and ability.
54. Defender consumed its item: no item effect.
55. Estimate of a status move is 0; of a type-immune move is 0.
56. Estimate takes only a Battle: a test that the signature has no State.

---

## 7. Open decisions (for the user)

- **PS-Q1.** Max damage today ranks with a lighter formula than tower uses. One shared integer estimate
  (3C) is what step 10 says. Using it for max damage adds burn, weather, screens, Life Orb, Choice items and
  ability effects to max damage's ranking and makes its results differ slightly from the Python opponent.
  Recommendation: shared estimate; say so in the README so past max-damage results are not compared.
- **PS-Q2.** Tower's setup test when the opponent has no revealed damaging move: treat as safe (my default).
- **PS-Q3.** Mega: always evolve when offered (docstring) for both players, or copy the accident (PS-B1).
- **PS-Q4.** Whether to add type-chart status immunities (PS-B4) or copy tower's lack of them.
- **PS-Q5.** Whether to keep tower's unfiltered status pool (PS-B7).
- **PS-Q6.** Tie-break among equal estimates (PS-B3): lowest move slot, Mega first.
- **PS-Q7.** Return form of the estimate: HP points plus max HP, or percent of max HP.

---

## 8. What I did not read

- `valid_orders` in poke-env (the order of legal actions, so tie behaviour is unconfirmed), and
  `pcsa\src\pcsa\battle\moves.py` (`base_data`, `observed_form`), `game.py` beyond `LEVEL`, `is_mega`,
  `mega_ability`, `mega_form_for_stone`, and `pcsa\src\pcsa\battle\builds.py` (catalog selection).
- poke-dojo's own source (no network): the "after poke-dojo" intent is inferred from comments only.
- `pilots.py`, `clone.py`, `make_pools.py`, `best_matchup.py`, `heuristic.py`, `neural_rules.py` bodies.
- The Champions data and the CG tables; all move/species facts above are from poke-env gen9 data.
- Whether Pokémon Champions' real formulas differ from `dm.py`; I did not compare `dm.py` with
  `audit/mechanics-formulas.md`.
