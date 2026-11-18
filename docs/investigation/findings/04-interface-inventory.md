# 04 - Interface inventory: everything our code reads from poke-env and Showdown

Investigator 04, 16 Nov 2026. Read-only; nothing was run. Paths: `RL` = `C:\Users\user\Code\research\pcsa-rl`,
`LL` = `C:\Users\user\Code\research\pcsa-rl-low-latency`, `PCSA` = `C:\Users\user\Code\pcsa\src\pcsa`,
`PE` = `RL\.venv\Lib\site-packages\poke_env` (0.16.1), `SD` = `C:\Users\user\Code\pcsa\.tools\vendor\pokemon-showdown`
(pinned commit c0845f33, tools.lock.json:46-50). Format id: `gen9championsbssregmc`
(`C:\Users\user\Code\pcsa\config\agent.yaml:109-110`), name "[Gen 9 Champions] BSS Reg M-C", mod `champions`
(`SD\config\formats.ts:275-280`).

## Summary (10 lines)

1. Counts: **98 attributes/methods** read (36 Battle, 37 Pokemon, 25 Move) + 6 enum classes (Field 8 members, SideCondition 10,
   Weather 8, Status 7, Effect 87, PokemonType 18); **30 request fields** read (+8 sent but never read); **129 message-type
   names** (62 poke-env parses with an effect, 54 ignored, 6 Player-level, 7 lobby); **3 monkeypatches** in `rl2/patches.py`
   (2 bug fixes + 1 plumbing) plus 4 other workarounds that live outside it.
2. Only ONE poke-env Player subclass is instantiated: `pcsa.players.arena.ArenaPlayer` (+ `AsyncArena`, `ThreadedArenaPlayer`);
   one Teambuilder subclass (`ExportTeams`); one Battle subclass (`ChampionsBattle`, used for every p2-seat decision via `replay()`).
3. The single switch point for "one config switch" is `pcsa.sim.server.ShowdownServer` (+ `pcsa.sim.engine.Engine.locate`),
   constructed once per actor in `rl/rollout.py:143-152`; clients only need `ServerConfiguration(ws://localhost:PORT/showdown/websocket, None)`.
4. The observation is NOT built from poke-env state alone: `battle._replay_data` (the raw per-seat protocol lines poke-env
   keeps) is re-read by features_log, turns, belief, labels, observed_form, choice_lock and the LL Tracker. The face must
   therefore be byte-faithful in line text and order, not only state-equivalent.
5. Champions-mod quirks that the face must copy: opponent HP is `floor(100*hp/max)||1` + `/100` with a `y/r/g` suffix at exactly
   20% and 50% (`SD\sim\pokemon.ts getHealth`); `maxChosenTeamSize` (not `maxTeamSize`) carries the pick-3; `|detailschange|`
   precedes `|-mega|`; no tera fields; `L50` always in details.
6. poke-env raises `NotImplementedError` on any `|kind|` not in its two lists (`abstract_battle.py:1191`), and `ValueError`/`KeyError`
   on name mismatches, so unknown or misspelled lines kill the whole battle (the actor then logs a failed battle).
7. Choices are sent as `/choose move <id> [mega]`, `/choose switch <1-based request slot>` (patched), `/team 123`, `/choose default`;
   `/choose move 1` for Recharge. Errors: `|error|[Invalid choice]` makes poke-env re-ask; `[Unavailable choice]` waits for an updated request.
8. Handshake to speak: `|challstr|`, reply to `/trn name,0,` with `|updateuser| name|...`, `|updatechallenges|{"challengesFrom":{id:format}}`
   on `/challenge`, accept via `/accept`, then `>battle-gen9championsbssregmc-N` rooms; `/utm packed|null` before challenge/accept; `/leave tag` after win.
9. Every actor runs 4-6 concurrent battles over ONE server with 2 ArenaPlayer websockets per lane (8-12 connections/server);
   2-16 servers per training run (configs). Battle timeout 120 s (`rl/rollout.py:34`).
10. Biggest unknowns: no recorded real wire log of a full room join found in the repos (face init order is inferred from
    server code), and exact `maxpp`/PP semantics for Champions (poke-env assumes `pp*8//5`).

---

## 1. poke-env classes we subclass or instantiate

| class | how used | where |
|---|---|---|
| `poke_env.player.Player` (subclass) | `ArenaPlayer(PokeEnvPlayer)`: overrides `teampreview`, `choose_move`, `_battle_finished_callback`; ctor args `account_configuration=AccountConfiguration(username, None)`, `server_configuration`, `battle_format`, `team=ExportTeams`, `start_listening=True`, `max_concurrent_battles=1` | `PCSA\players\arena.py:44-72` |
| `ArenaPlayer` subclasses | `AsyncArena` (awaitable choose_move/teampreview, batching) | `RL\rl2\lanes.py:91-107`; `ThreadedArenaPlayer` (to_thread) `RL\llm\play.py:43-49` |
| `poke_env.teambuilder.Teambuilder` (subclass) | `ExportTeams.yield_team()` -> `parse_showdown_team` -> `join_team` (packed team sent via `/utm`) | `PCSA\players\arena.py:31-41` |
| `Teambuilder` / `TeambuilderPokemon` (static helpers / dataclass) | `parse_showdown_team`, `parse_packed_team`, `join_team`; Champions stat points go in the `evs` field | `PCSA\battle\teams.py:60-96`, `RL\rl2\teams.py:196`, `RL\rl2\sim_oracle.py:147`, `RL\rl\teams.py:21,48` |
| `poke_env.battle.Battle` (instantiated) | p1-seat battles are created by poke-env itself (`Player._create_battle`, `PE\player\player.py:182-253`); rebuilt as `Battle(tag, "me", LOGGER, 9)` in `rl/imitate_rebuild.py:228` | -- |
| `Battle` subclass `ChampionsBattle` | adds form abilities, Mega stones, phone-HP normalisation; `replay(lines, request)` builds it for every **p2-seat decision in training** (arena.py:97, lanes.py:114) and in belief `board.py:35`. Overrides `parse_message`, `parse_request` | `PCSA\battle\adapter.py:17-120`, `PCSA\battle\state.py:278-318` |
| `AccountConfiguration` | `AccountConfiguration(username, None)`; username = `f"{name[:12]}{counter}"` (e.g. `rla1`, `rlb2`, kind names) | `arena.py:58-60`, `NAME_LIMIT=12` arena.py:30 |
| `ServerConfiguration` | `ServerConfiguration(f"ws://localhost:{port}/showdown/websocket", None)` (auth url None: never used, no password) | `PCSA\sim\server.py:24-25` |
| `poke_env.concurrency.POKE_LOOP` | only in `rl2/scripts/prof_lanes.py:30`; actors use their own `asyncio` loop (`rl/rollout.py:170`) and run poke-env coroutines on it | -- |
| `PSClient` | used indirectly: `arena.ps_client.stop_listening()` (`lanes.py:222,292`, `arena.py:184`), `ps_client.logger` | -- |
| `Player` class constants | `Player.MESSAGES_TO_IGNORE` read by `sim_oracle.feed` (`RL\rl2\sim_oracle.py:431`) | -- |
| Order classes | `SingleBattleOrder(order, mega=...)` constructed in search players (`RL\rl2\search_player.py:110-122`); `battle.valid_orders` consumed in `PCSA\players\mirror.py:57`; `choose_random_move` fallback (arena.py:100, lanes.py:117) | -- |
| Data helpers | `poke_env.data.GenData.from_gen(9)` (`moves`, `pokedex`, `type_chart`, `natures`, `UNKNOWN_ITEM`), `GenData.from_format`, `to_id_str`, `Move(id, gen=9)`, `Move.retrieve_id` | everywhere (e.g. `RL\rl\features_moves.py:36,92`, `PCSA\battle\moves.py`) |

Not used: `poke_env.environment.*` (gym wrappers), `ShowdownServerConfiguration`, `Player.ladder`, `/search`, `DoubleBattle`.
`Player.ladder` / `/search` exist in poke-env but no code path of ours calls them (grep of rl/, rl2/, PCSA\run, PCSA\players).

Other consumers of Showdown that are NOT through poke-env (a replacement must also cover or drop these):
- `PCSA\sim\validate.cjs` (team legality via Showdown's `TeamValidator`, `PCSA\sim\validate.py:13-36`), used by `RL\rl2\teams.py`, `form_sets.py:350`, `scenarios/build.py:209`, `opponents/make_pools.py:13`.
- `RL\rl2\sim_oracle.js` (search forward model; `require(PCSA_SHOWDOWN_SIM/dist/sim/battle)`, `RL\rl2\sim_oracle.js:34-36`, format created at :153). Used only by search/teacher configs (4 of ~45 configs set `search_teacher`).
- `Catalog`/`Engine.locate` need `.tools` (Node + built Showdown) to exist (`PCSA\sim\engine.py:23-36`).

## 2. Attribute inventory (what our code reads)

Counts are regex counts over variable names, so "N" is a lower bound for each row. Locations are `file:line` (abbrev: `f_rows`=`RL\rl\features_rows.py`,
`f_field`=`RL\rl\features_field.py`, `f_mov`=`RL\rl\features_moves.py`, `f1`=`RL\rl\features.py`, `f2`=`RL\rl2\features.py`,
`stats`=`PCSA\battle\stats.py`, `adapter`=`PCSA\battle\adapter.py`, `ll`=`LL\rl2\features_ll.py`, `opp`=`PCSA\players\opponent.py`).
"LL" column: Y = the low-latency observation (`LL\rl2\features_ll.py`, untracked) reads it.
Verified = I read the poke-env code; "feed" column = which protocol line or request field sets it in poke-env 0.16.1.

### 2a. Battle (36)

| attribute | read where (up to 3, + total) | LL | fed by |
|---|---|---|---|
| `turn` | f_rows.py:221, rl/rollout.py:121, rl/player.py:186 (+36) | Y (ll:364,411,685) | `\|turn\|N` (`abstract_battle.py:593`-ish `end_turn`) |
| `team` (dict `"p1: Name"` -> Pokemon) | f1:91, rl/player.py:146, rl2/matrix.py (+61) | Y (via f1._request_members) | request `side.pokemon[].ident` (`_update_team_from_request`), `\|switch\|` |
| `opponent_team` (dict; bare-species keys until first switch) | f1:120, belief/fast.py:1113 (+48) | Y (ll:306) | `\|switch\|/\|drag\|`, `\|poke\|` creates preview entries keyed by species (quirks.md row "Before the first switch") |
| `active_pokemon` | max_damage.py:54, belief/fast.py:1463 (+54) | -- (uses `team` order) | `\|switch\|`, request `active` flag |
| `opponent_active_pokemon` | f1:121,144, f_field (+65) | Y (ll:307,418) | `\|switch\|`/`\|drag\|` |
| `player_role` (read AND written: `battle.player_role = "p1"` in `state.py:221`, `board.py:38`) | rl/rollout.py:121, rl/player.py:72,232 (+27) | Y (ll:417) | `\|player\|` (username compare) and request `side.pokemon[0].ident[:2]` (`battle.py:99-100`) |
| `opponent_role` | rl2/lines.py:222,538, matrix2.py:371 (+3) | -- | derived |
| `fields` (dict Field->turn) | f_rows.py:275, f_field.py:83-85 (+27) | Y (ll:370,557) | `\|-fieldstart\|`/`\|-fieldend\|` |
| `weather` (dict Weather->turn) | f_mov.py:64, f_field.py:77, endgame.py:64 (+9) | Y (ll:369,412) | `\|-weather\|` |
| `side_conditions`, `opponent_side_conditions` | f_field.py:157-158, f_rows.py:260 (+13/+17) | Y (ll:378,646) | `\|-sidestart\|`/`\|-sideend\|` (stackable: Spikes layers) |
| `used_mega_evolve`, `opponent_used_mega_evolve` | f_rows.py:232-233, f_field.py:65-66 (+8/+9) | Y (ll:366,419) | `\|-mega\|` |
| `can_mega_evolve` | f_field.py:65, heuristic.py:104 | Y (ll:366) | request `active[0].canMegaEvo` |
| `trapped`, `maybe_trapped` | f_field.py:64 | Y (ll:365) | request `active[0].trapped` / `.maybeTrapped` |
| `force_switch` | f_field.py:64 | Y (ll:365) | request `forceSwitch[0]` |
| `last_request` (dict) | f1:90, f2:268, belief/fast.py:121,1366, arena.py:81,96, lanes.py:113,128 (+13) | Y (ll:413,684) | last `\|request\|` JSON (poke-env stores it raw, `battle.py:~88`) |
| `_last_request` (private, read+written by search) | rl2/search_player.py:67-101, sim_oracle.py:235 | -- | same |
| `max_team_size` | rl/player.py:146, rl2/player.py:173,334, clone.py:396 (+12) | -- | request `maxTeamSize` (poke-env), overwritten from `maxChosenTeamSize` by our code (see 6.4) |
| `_max_team_size` (written) | arena.py:83, lanes.py:130,148, scenarios/source.py:135 | -- | -- |
| `teampreview_opponent_team` | f1:129, endgame.py:178, max_damage.py:44 (+8) | Y (ll:315) | `\|poke\|p2\|details\|` (`_register_teampreview_pokemon`, only for the other role) |
| `battle_tag` | rl/player.py:187, best_matchup.py:80 (+27) | -- | room name in first line `>battle-...` |
| `_replay_data` (list of split lines, all lines that reached `parse_message`) | f_log.py:189, rl/player.py:74, belief/fast.py:661, arena.py:95, state.py:157,182 (+25) | Y (ll:300,678,691) | every line except Player-level ignores (see 4.5) |
| `won`, `lost`, `finished` | rl/rollout.py:118, arena.py:164, search_sim.py:122 | -- | `\|win\|name` (`won_by`, compares to `player_username`), `\|tie\|` |
| `valid_orders` | mirror.py:57 | -- | derived from request (`battle.py:272-298`) |
| `available_moves`, `available_switches` | heuristic.py:93,120 (+7) | -- | request `active[0].moves[]` (non-disabled) and `side.pokemon[]` |
| `is_grounded(mon)` | no_effect.py:81,130, neural_rules.py:233 | -- | fields + mon item/ability/types |
| `player_username`, `opponent_username` | sim_oracle.py:414-415 | -- | `\|player\|` |
| `get_pokemon(ident, ...)` | sim_oracle.py:252 | -- | -- |
| `parse_message`, `parse_request` (called directly) | state.py:278-309, board.py:139, search_sim, sim_oracle.feed:431-470, patches.py:36-51 | -- | -- |
| `won_by`, `tied` (called directly) | sim_oracle.py:455-458 | -- | -- |
| `rules`, `players`, `rating`, `team_size` | not read by our code (poke-env fills `rules` from `\|rule\|`, `_team_size` from `\|teamsize\|`; `_team_size` is enforced: `get_pokemon` raises `ValueError` when exceeded, `abstract_battle.py:312-317`) | | |

### 2b. Pokemon (37)

| attribute | read where | LL | fed by |
|---|---|---|---|
| `species` | stats.py:94-98, f1:126 (+162) | Y | `details` of request/switch; Mega form NOT reflected (poke-env keeps base species; our `observed_form` rescans the protocol, `PCSA\battle\moves.py:25-57`) |
| `name` | f1:107, state.py:26 (+237) | Y | ident (`p1: Name`) |
| `fainted` | matrix.py:105 (+63) | Y | `\|faint\|`, condition `0 fnt` |
| `current_hp_fraction`, `current_hp`, `max_hp` | f_rows.py:164, threat.py:99, ll:455-457,592 | Y | request `condition` `hp/max [status]` (own); `\|switch\|/-damage/-heal/-sethp\|` hp field (opp as `x/100`) |
| `stats` (dict hp..spe) | encore_switch.py:284, state.py:26, ll:331,454 | Y (ours only) | request `side.pokemon[].stats` + `hp` set from max_hp (`pokemon.py:546-547`) |
| `base_stats` | f_rows.py:185,252, stats.py:101-129 | -- | static dex via species |
| `status`, `status_counter` | f_rows.py:204-206, rl2/lines.py:556, ll:349,460-463 | Y | condition suffix, `\|-status\|`, `\|-curestatus\|`; counter incremented in `Pokemon.moved/cant_move` |
| `boosts` (dict, `accuracy`/`evasion` included) | f_mov.py:65, stats.py:150, ll:348,464 | Y | `\|-boost\|/-unboost/-setboost/-clear*/-swapboost/-invertboost/-copyboost\|` |
| `effects` (dict Effect->counter) | f_rows.py:209, no_effect.py:71, f_field.py:141, ll:469 | Y | `\|-start\|/-end/-activate/-singleturn/-singlemove\|` |
| `moves` (dict id->Move, insertion order matters) | f2:106,133, threat.py:36, ll:487-489 | Y | request `side.pokemon[].moves` (own, in request order); `\|move\|` reveal (opp) |
| `item` (`"unknown_item"`, `""`, id, `None`) | stats.py:147, encore_switch.py:287, matrix.py:117, ll:323 | Y | request `item`; `\|-item\|/-enditem\|`, `[from] item:`; `UNKNOWN_ITEM` default for opp |
| `ability`, `base_ability`, `temporary_ability`, `forme_change_ability`, `possible_abilities` | stats.py:159, encore_switch.py:239-241, adapter.py:98-136, regenerator.py:30 | Y (`ability`) | request `baseAbility` (REQUIRED key) / `ability`; `\|-ability\|`; dex |
| `types`, `type_1`, `type_2`, `_temporary_types` | f_rows.py:174-177, no_effect.py:64, ll:442-447 | Y | dex + `\|-start\|...typechange` |
| `first_turn` | f_rows.py:214, f_field.py:73, no_effect.py:142, ll:476 | Y | `switch_in` / `end_turn` counters |
| `protect_counter`, `preparing`, `must_recharge` | f_rows.py:215-218, ll:475-477 | Y | `Pokemon.moved`, `\|-prepare\|`, `\|-mustrecharge\|` |
| `last_move` | f_field.py:112, state.py:187 | -- | `\|move\|` |
| `active` | encore_switch.py:80-91, state.py:289 (+18) | Y | request flag; `\|switch\|` |
| `transformed` | adapter.py:33,95,112 | -- | `\|-transform\|` |
| `level` | PCSA\battle\teams.py:86 (export text) | -- | details `L50` |
| `damage_multiplier(type)` | matrix.py:123, no_effect.py:96, tactics.py:111 (+9) | -- | dex |
| mutators called by `state.normalize_fainted_request` | `faint()`, `_active=False`, `clear_boosts()`, `_max_hp` (`state.py:287-293`) | -- | -- |
| private: `_moves._transform_moves`, `base_moves` | adapter.py:35-40 | -- | `\|-transform\|` |

### 2c. Move (25) - almost all static dex lookups, not protocol-fed

Constructed with `Move(move_id, gen=9)` (`f_mov.py:36,92`, `opp.py:76`, `ll:177`). Read: `id`, `type`, `category`, `base_power`, `accuracy`,
`priority`, `expected_hits`, `boosts`, `self_boost`, `target`, `flags`, `status`, `secondary`, `self_switch`, `heal`, `drain`, `recoil`,
`n_hit`, `entry`, `volatile_status`, `side_condition`, `pseudo_weather`, `is_protect_move`, plus **dynamic**: `max_pp`, `current_pp`
(`f_rows`/`f1.py pp_fraction` `f_mov.py:149-152`; `ll:504-505`). `current_pp` is decremented by poke-env on every `|move|` of the mon
(`Move.use`, `move.py:123-130`, Pressure adds 1) and `max_pp = entry["pp"]*8//5` (`move.py:461`): this is poke-env's data, not Showdown's `maxpp`.
LL reads the request's `pp`/`maxpp`/`disabled` for our active instead (`ll:413-414,500-507`).

### 2d. Enums

- `Field` (8): TRICK_ROOM(16 uses), PSYCHIC/MISTY/ELECTRIC/GRASSY_TERRAIN, GRAVITY, WONDER_ROOM, MAGIC_ROOM. Field strings parsed from `-fieldstart|move: Trick Room`, `Electric Terrain` (`PE\battle\field.py:33-52`).
- `SideCondition` (10): TAILWIND, SPIKES, TOXIC_SPIKES, STEALTH_ROCK, REFLECT, LIGHT_SCREEN, AURORA_VEIL, STICKY_WEB, SAFEGUARD, MIST (`f_field.py:30-42`; `SideCondition.from_showdown_message` maps `move: Name`).
- `Weather` (8): SUNNYDAY, DESOLATELAND, RAINDANCE, PRIMORDIALSEA, SANDSTORM, SNOWSCAPE, HAIL, DELTASTREAM (`f_field.py:15-26`).
- `Status` (7): BRN FRZ PAR PSN SLP TOX FNT.
- `Effect` (87 + 3 derived, `f_rows.py:35-123`: CONFUSION ... IMPRISON; the 90-wide volatile block is a layout constant) plus `Effect.ENCORE/TAUNT/YAWN/...` in rules.
- `PokemonType` (all but THREE_QUESTION_MARKS and STELLAR, `f_mov.py:17`), `MoveCategory`, `Move.target.name` (`f_log.py`).
- Unknown enum strings only warn (UNKNOWN member) - they do not raise (`effect.py:245`, `field.py:48`, `side_condition.py:55`, `weather.py:36`).

## 3. Request JSON: fields relied on

Source of truth for what Showdown emits: `SD\sim\battle.ts:1405-1470` (getRequests), `SD\sim\pokemon.ts:1104-1210` (move/switch data),
`SD\sim\side.ts:540-570`, `SD\server\room-battle.ts:796-810` (server adds `rqid`, forwards as `|request|JSON`).

| field | consumer (file:line) | note |
|---|---|---|
| `wait` | poke-env `battle.py:~64` -> `_wait`; `Player._handle_battle_request` skips choosing (`player.py:~400`); `search_sim.py:318,387` | other seat gets `{"wait":true,"side":{...}}` while one side must switch |
| `forceSwitch` (list, `[0]` read) | `battle.py:90`; `f2.py:268`, `ll:684`, `search_sim.py:322` | `[true]`; `{forceSwitch, side}` (no `active`) |
| `teamPreview` | `battle.py:94-98` -> `battle.teampreview` -> `Player.teampreview` | |
| `maxTeamSize` | `battle.py:97` (default = number of mons) | Showdown does NOT send it |
| `maxChosenTeamSize` | `arena.py:81`, `lanes.py:128,146`, `scenarios/source.py:134` copy into `battle._max_team_size` | Showdown sends it (`battle.ts:1431`) = 3 here. Without it `max_team_size` = 6 and `all_picks(6, 6)` blows up |
| `side` | `battle.py:67`, everywhere | `{name, id, pokemon[]}`; `name`,`id` used by our mirror regex only (`mirror.py:15`) |
| `side.pokemon[].ident` | `battle.py:99-100` (role = `ident[:2]`), `f1:91`, `patches.py:20-21`, `_update_team_from_request` | format `"p1: Name"` (no `a`) |
| `.details` | `pokemon.py:730` | `Species[, L50][, F/M]`; Mega after evolve: `Garchomp-Mega-Z, L50, F`; no `tera:` in Champions |
| `.condition` | `pokemon.py:724`, `state.py:111` | `hp/max`, `hp/max par`, `0 fnt`; exact HP for own side |
| `.active` | `pokemon.py:712`, `battle.py` active detection | true for the first N (singles: 1) |
| `.stats` | `pokemon.py:736-738`; `stats.our_stats`, `ll:331,454` | `{atk,def,spa,spd,spe}` only (hp from condition) |
| `.moves` (list of move ids) | `pokemon.py:733-734` `_add_move`; order = our action slot order | `hiddenpower...`, `return102` special ids exist in Showdown (poke-env handles) |
| `.baseAbility` | `pokemon.py:715-716` (hard `KeyError` if missing) | |
| `.ability` | `pokemon.py:717-720` (optional; differs => temporary ability) | present because gen >= 7 (`pokemon.ts:1195`) |
| `.item` | `pokemon.py:728` (hard key) | `""` when none/consumed |
| `.reviving` | `battle.py:~80` (`side.get("pokemon")` any) | gen9 field, false |
| `active[0].moves[].id` | `pokemon.py:804` (available moves; `disabled` = skip) | `recharge`, `struggle` special ids (`move.py:17`) |
| `active[0].moves[].disabled` | `pokemon.py:804`, `ll:506`, `search_sim.py:328`, `rl/imitate_rebuild.py:251` | bool |
| `active[0].moves[].pp`, `.maxpp` | `ll:501-503` (LL only), `rl2/scripts/pp_check.py:93-119` | poke-env does not read them unless `strict_battle_tracking` (default False) |
| `active[0].moves[].move`, `.target` | not read (only poke-env strict mode) | |
| `active[0].trapped`, `.maybeTrapped` | `battle.py:108,125` | only when a switch exists (`pokemon.ts:1128-1140`) |
| `active[0].canMegaEvo` | `battle.py:117`, `search_sim.py:326` | `canMegaEvoX/Y` also exist in Showdown but are ignored |
| `active[0].canZMove/canDynamax/canTerastallize` | `battle.py:119-123` | always absent in Champions |
| `rqid`, `noCancel`, `update` | not read (poke-env 0.16.1 only keeps them inside `last_request`) | server adds `rqid`; `update:true` on `[Unavailable choice]` re-emits |

Count read: 30 (7 top-level, 7 active-level incl. 2 in moves[], 4 moves[], 10 side.pokemon[], `side.name/id` 2). Emitted but unread: `rqid`, `noCancel`, `update`,
`maxChosenTeamSize` is read; `canMegaEvoX/Y`, `pokeball`, `commanding`, `maxMoves`, `moves[].move/.target`.

Synthetic requests our own code builds (live path, not server): `PCSA\battle\state.py:49-141` and `PCSA\rl_training\preview_leads.py:64`
(`rqid: 1`, `maxTeamSize: 3`). Same keys; useful as a minimal-valid-request reference.

## 4. Protocol messages

### 4.1 Lobby / handshake a fake server must speak (poke-env `PSClient`, `PE\ps_client\ps_client.py`)

Client side verified; server side from `SD` code.

1. Client opens `ws://localhost:PORT/showdown/websocket` (`ps_client.py:246-262`, options: `max_queue=None`, open timeout 10 s, ping 20/20 s).
2. Server -> `|challstr|4|<hex>` (`:177`; only `[2]`,`[3]` used when a password exists, so any value works).
3. Client -> `|/trn <name>,0,` (`:299`; empty assertion, no `--no-security` auth needed since the server runs with `--no-security`, `sim/server.py:73`).
4. Server -> `|updateuser| <name>|1|<avatar>|{...}`; client sets logged_in only if `[2]` == `" "+username` or `" "+username+"@!"` (`:180-186`). A `Guest ` name is tolerated; any other name logs a warning but continues.
5. Client -> `|/utm <packed team>` or `|/utm null` (`:328-332`) then challenger -> `|/challenge <opponentId>, gen9championsbssregmc` (`:130`); acceptor -> `|/utm ...` then `|/accept <challengerId>` (`:125`).
6. Challenged client must receive `|updatechallenges|{"challengesFrom":{"<id>":"gen9championsbssregmc"},"challengeTo":null}` (`player.py:431-441`; `"updatechallenges" in split[1]`, `ps_client.py:195-197`). The `|pm|..|/challenge ...` path also works (`ps_client.py:205-208`, `player.py:422-429`) but `updatechallenges` is what the real server sends.
7. Server creates room `battle-gen9championsbssregmc-N` and sends both players `>battle-gen9championsbssregmc-N\n|init|battle\n|title|A vs. B\n...`. **Room id format is load-bearing**: `_create_battle` does `">battle-gen9championsbssregmc-709".split("-")`, checks `format in (parts[1], parts[1]+"bo3")`, and `battle_tag = "-".join(parts)[1:]` (`player.py:191-193`). A different id raises `ShowdownException` (`:254`). Also `|updatesearch|{...}` (ignored, `ps_client.py:198`).
8. After `|win|name` / `|tie|` the client sends `|/leave battle-tag` (`player.py:383`), server may answer `|deinit` (poke-env drops the per-room lock when the message contains `|deinit`, `ps_client.py:175-176`).
9. Other server -> client lobby lines poke-env knows: `|popup|...` (logs warning; our `Popups` handler treats any warning containing "popup" as a failed battle, `RL\rl\rollout_battle.py:30-41,55-60`), `|nametaken|` (raises), `|pm|`, `|updatesearch|`. Any other lobby line (`|formats|`, `|updateuser|` for others, `|queryresponse|`) logs "Unhandled message" at WARNING (harmless unless the text contains "popup").
10. Never sent by our players: `/search`, `/avatar` (avatar None), `/timer on` (`start_timer_on_battle_start=False` default), `/acceptopenteamsheets` (only if "vgc" in format), `/confirmready` (bo3 only), `/forfeit` (no code path), `/cancel`, `/undo`. Verified with rg over PE and our code.

Battle-room commands sent, as `"battle-tag|<msg>"` (`send_message`, `ps_client.py:307-327`; `player.py:420`):
`/choose move <id>[ mega]`, `/choose switch <N|name>`, `/team <digits>`, `/choose default`, `/choose move 1` (recharge; `battle_order.py:46-48`). No `|rqid` suffix is ever appended. `ForfeitBattleOrder` = `/forfeit` exists but is unused by us.

Win/tie/error handling (`player.py:370-398`): `|win|name` -> `battle.won_by(name)` compares to `player_username` (the exact `/trn` name); `|tie` -> `tied()`; `|error|[Unavailable choice]...` -> `_trying_again.set()` then waits for an updated request; `|error|[Invalid choice]...` -> immediately re-calls `choose_move` with `maybe_default_order=True` (1/1000 chance of `/choose default`, so a face that returns Invalid for a legal choice produces an infinite loop); other `|error|` -> logged at level 25 ("Error message received"), counted into `run_match` errors (`arena.py:154-158`); `|bigerror|` -> warning.

### 4.2 Per-room message types poke-env 0.16.1 parses with an effect (62)

All in `AbstractBattle.parse_message` (`PE\battle\abstract_battle.py:562-1191`); anything not here and not in the ignore sets raises
`NotImplementedError` (`:1191`). "Reg M-C" = my judgement whether BSS singles in the champions mod can emit it; "us" = our code depends on the line text.

| type | line | poke-env effect | Reg M-C | us (text/effect) |
|---|---|---|---|---|
| switch, drag | 571 | `switch()`: `get_pokemon(ident, details)`, set_hp_status | yes | heavily (features_log, turns, observed_form, belief, choice_lock) |
| -damage | 574 | set hp; `[from] item:` + `[of]` reveal item; `[from] ability:` | yes | heavily; HP text `x/100[y/g/r]` for opp |
| move | 579 | `moved()`, PP use, Pressure, `[from] move:` override (patch 1), `[miss]/[still]/[notarget]/[spread]` suffixes, `[from] lockedmove` | yes | heavily |
| cant | 725ish | `cant_move()` | yes | belief, state.acted_since_entry |
| turn | ~730 | `end_turn(int)` | yes | everywhere |
| -heal, -sethp | | hp | yes | features_log, turns |
| -boost, -unboost, -setboost, -clearboost, -clearallboost, -clearnegativeboost, -clearpositiveboost, -copyboost, -swapboost, -invertboost | | boosts | yes | boosts feature; belief |
| -weather | 755 | `_weather={W: turn}` (`none` clears) | yes | features, belief |
| faint | | `faint()` | yes | turns, features_log |
| -ability, -endability | | ability / temp ability | yes | `known_ability` reads the raw line (`stats.py:160-165`) |
| -start, -end, -activate, -singleturn, -singlemove | 795 | effects (many special cases: Skill Swap, Dancer, Mummy, Symbiosis, Leppa, Trick, Mimic) | yes | belief, effects feature |
| -status, -curestatus, -cureteam | | status | yes | |
| rule | | `rules.append` | yes (Species/Item/Nickname Clause lines) | no |
| -fieldstart, -fieldend | | fields (`-fieldend` without start KeyErrors in some paths; our `replay()` dedupes, `state.py:300-310`) | yes | |
| -sidestart, -sideend, -swapsideconditions | | side_conditions | yes | |
| -item, -enditem | 946 | item set/cleared; 6-part form handles Frisk/Pickpocket/Magician/Thief/Covet else `ValueError("Unhandled item message")` | yes | belief evidence, turns |
| -mega | 1004 | `used_mega_evolve`, `mega_evolve(event[3])` (arg is species, poke-env calls it stone) | yes | `observed_form`, belief |
| detailschange, -formechange | 940 | `forme_change(species)` | yes | observed_form, LL tracker |
| -transform | | transform | yes (Ditto) | adapter, patch 2 context |
| -prepare, -mustrecharge | | preparing / must_recharge | yes | feature columns |
| -primal, -zpower, -terastallize | | flags | no (Champions mod) | no |
| -immune (4 parts, `[from] ability:` sets ability) | | | yes | belief (`-immune`) |
| clearpoke, poke, teamsize, start, gen, tier, player, title, inactive, raw, replace, swap, message, -message | | team size, preview team, role, format (`tier` -> `_format`), ratings (`raw` "X's rating: NNNN"), illusion end (`replace`), `swap`(doubles only, logs warning) | clearpoke/poke/teamsize/start/gen/tier/player/title/rule/message: yes; replace: Zoroark only; swap: no; raw: only rated; inactive: only with timer | `player` (role), `poke` (preview), `teamsize` (size guard) |

Counted: 62 distinct names (drag counted separately from switch).

### 4.3 Ignored by `AbstractBattle` (49) - still stored in `_replay_data`

`-anim -block -burst -center -combine -crit -fail -fieldactivate -hint -hitcount -miss -notarget -nothing -ohko -resisted -supereffective -waiting -zbroken J L askreg badge c chat crit debug deinit gametype hidelines html immune inactiveoff init j join l leave n name rated resisted sentchoice split supereffective teampreview upkeep uhtml zbroken ""` (`abstract_battle.py:19-69`). Judgement: Reg M-C singles can emit `-crit -miss -fail -hitcount -supereffective -resisted -notarget -nothing -ohko -hint -block -anim -fieldactivate -waiting(no, doubles) upkeep teampreview gametype init j/l/n/c (room chat/join) html/uhtml`; it cannot emit `-burst -zbroken -combine -center` (Ultra Burst, Z, doubles-only). Note `-immune` (hyphen) is handled with an effect (4.2), `immune` (no hyphen) is the ignored one.

**Our code reads several of these from the raw lines:** `-crit` (belief `hits.py:120`, `rl2.player.crit_reported=True` "the simulator prints -crit lines", `RL\rl2\player.py:110`), `-miss`, `-immune`, `-fail`, `upkeep` (belief `hits.py:75,237`), `-hitcount`, `-supereffective`/`-resisted` (not found read).

### 4.4 Player-level handling (`PE\player\player.py:295-398`)

- Ignored entirely, never reach the battle or `_replay_data`: `t:` `expire` `uhtmlchange` `tempnotify` `tempnotifyoff` (5; `player.py:49,328`). Timestamps `|t:|123` are therefore invisible to everything we do.
- `|request|JSON` (`:330`): `orjson.loads`, `battle.parse_request`, then choose. An empty payload is skipped.
- `|showteam|` (open team sheets): not emitted by Reg M-C BSS (no Open Team Sheets rule in `Flat Rules`); not needed.
- `|win|name`, `|tie`, `|error|`, `|bigerror|` as above.
- A line with a single element (no `|`) is treated as chat text (`:316-323`).
- Everything else -> `battle.parse_message`.

### 4.5 `_replay_data` contents (what our features see)

Every split line passed to `parse_message`, including blanks (`['', '']`), `init`, `title`, `j`, `gametype`, `player`, `teamsize`, `gen`, `tier`, `rule`, `clearpoke`, `poke`, `teampreview`, `start`, all battle lines; plus a synthetic `['', 'win', name]`/`['', 'tie']` appended by `won_by`/`tied` (`abstract_battle.py:~1400`). NOT included: `request`, `error`, `t:`, `expire`, ... (see 4.4).

### 4.6 Which protocol kinds our code parses itself (from `_replay_data`)

`RL\rl\features_log.py:158-193` (turn, switch, drag, replace, move, -damage, -heal, -sethp, -mega, faint, `[from]` tag test);
`RL\rl2\turns.py:67-93` (turn, move, switch, -mega, -damage/-heal/-sethp, faint, -item, -enditem, -ability, `[from] item:`/`[from] ability:`);
`PCSA\battle\moves.py:35-57` (-mega, switch, drag, replace, detailschange, -formechange);
`stats.py:160-165` (-ability); `state.py:160-190` choice_lock / acted_since_entry (switch, drag, move, cant, `[from]`);
`research\pcsa-belief\belief\fast.py:1079-1134`, `hits.py:72-141,224-305`, `evidence.py:40-121` (turn, upkeep, move, switch, drag, replace, -damage, -heal, -sethp, -crit, -miss, -immune, -fail, cant, -activate, -item, -enditem, -ability, -end, -start, detailschange, -formechange, -mega, -fieldstart, -fieldend, `[of] `, `[from] ability:`, `[from] item:`, `[notarget]`);
`RL\rl2\lines.py` (+matrix) (-boost, -status, -weather, -fieldstart/-end, -sidestart/-sideend tailwind, -cureteam, -clearallboost, -start/-end speed volatiles);
`RL\rl2\labels.py` (turns.read); `LL\rl2\features_ll.py:225-298` Tracker (turn, move, switch, drag, replace, detailschange, -formechange, -mega).
~35 distinct kinds.

## 5. Action indexing

Network head: `ACTIONS = 11` (`RL\rl\spec.py`; `rl/features.py:22` `MOVE_ACTIONS, MEGA_AT, SWITCH_AT = 4, 4, 8`).

| index | meaning | built / mapped at |
|---|---|---|
| 0-3 | move slot j (no Mega) | `rl/features.py:178-197 action_index`: `min(3, a.action.move)` |
| 4-7 | move slot j + Mega (`/choose move <id> mega`) | `MEGA_AT + slot` |
| 8-10 | switch to bench member k = position in `_request_members(battle)` minus the active | `rl/features.py:88-95,188-195` |
| (team preview) | `/team 123...`: pick tuple of 0-based indices into `battle.team` order (= request `side.pokemon` order), lead first | `rl2/player.py:173,334` (`preview.all_picks(len(team), max_team_size)`), `arena.py:95`, `lanes.py:137,160` |

Details:
- **Move slot j** = position of the move id in `list(battle.active_pokemon.moves)` (`PCSA\players\mirror.py:57-62`), i.e. insertion order of `Pokemon.moves`. For our own mon that is the request `side.pokemon[i].moves` order (`pokemon.py:733-734`), so index 0-3 = request move order. `_slot_moves` (`f1.py:63-75`, reused by `ll:487`) places each legal move at its slot.
- `valid_orders` (`battle.py:272-298`): switches (if not trapped), moves, and for `can_mega_evolve` the same moves again with `mega=True`. Message strings: `/choose move {id}` (+` mega`), `/choose switch {name}`; recharge `/choose move 1`; Struggle/Recharge appear as `Move` objects not in `mon.moves` (slot falls back to 0).
- **Switch is rewritten** by `rl2.patches.SingleBattleOrder.message` to `/choose switch {slot}` where slot = 1-based index of the mon's name in `request.side.pokemon` (`patches.py:18-22,64-70`); falls back to the name if no request/no unique match. So the face must accept numeric switch targets and the request's `side.pokemon` ORDER is the contract (after a team pick it is pick order, actives first: `SD\sim\battle.ts:2803-2810`).
- Team preview: `Showdown picks` -> `side.pokemon` is cut to the chosen N in chosen order, so in battle requests slot 1 = lead.
- p2 seat: the observation is built from a mirrored (p1-view) ChampionsBattle (`mirror.as_p1`, `arena.py:96-98`, `lanes.py:112-114`), but the ORDER sent is built from the original seat's battle objects (`chosen.order`), so names/slots refer to the seat's real request.
- Low-latency (`features_ll.py`): same mask and indexing (`ll:675-676` calls `f1.action_index`, `_slot_moves` at ll:487), the only difference is that the active's move state comes from the request (`ll:413-414`).

## 6. `rl2/patches.py` workarounds

Installed once per actor process from `rl2.player._teach_wrapped_kinds` (`RL\rl2\player.py:443`); records repairs to `<run>/repairs.jsonl` (`patches.py:75-90`).

| # | patch | bug fixed | what the face must emit/accept | where |
|---|---|---|---|---|
| 1 | `AbstractBattle.parse_message` wrapper: on `KeyError` in a `\|move\|...\|[from] move: X` line where X is not in `mon.moves`, adds X to the mon and re-parses | poke-env `overridden = mon.moves[Move.retrieve_id(overridden_move)]` KeyError (`abstract_battle.py:~705`) when a move is called by a move the mon has not revealed. Test case: `\|move\|p2a: Gyarados\|Round\|p1a: Ditto\|[from] move: Round` (`RL\rl2\tests\test_v2.py:116-127`) | Keep emitting Showdown's exact `[from] move: Name` tag; do not "fix" it server-side or the patch becomes a no-op (harmless) | `patches.py:29-51` |
| 2 | `SingleBattleOrder.message` property: switch orders sent as `/choose switch <slot>` instead of `/choose switch <name>` | a Transformed Ditto's name (and other duplicate/shadowed names) makes Showdown pick the wrong target when sent by name (test `test_a_switch_is_sent_by_slot_so_a_transformed_ditto_cannot_shadow_it`, `test_v2.py:204-225`) | Must accept `/choose switch N` with N = 1-based index in `request.side.pokemon`; (and still accept names for the unpatched paths) | `patches.py:53-72` |
| 3 | `Player._handle_battle_request` wrapper that stores `battle.last_request` in a `ContextVar` | plumbing for #2 (message is built later in the same asyncio task) | none (needs `battle.last_request` to be the request being answered) | `patches.py:58-62` |

Workarounds outside `patches.py` that also constrain the face:
- **max team size**: `ArenaPlayer.teampreview` / `_preview` / `_uniform_preview` / `forced_preview` copy `last_request["maxChosenTeamSize"]` into `battle._max_team_size` because poke-env 0.16.1 reads `maxTeamSize` (`arena.py:81-83`, `lanes.py:127-130,145-148,134-135`).
- **`replay()` dedupe of `-fieldend`** and rewriting `[notarget]` target to `""` (`state.py:296-312`); `ChampionsBattle._normalize` drops a `-fieldend` for a field never started (`adapter.py:59-66`).
- **p2 mirror**: `mirror.mirror_protocol` regex swaps `p1`<->`p2` where followed by `a-d:`, `:`, `|` or end; `mirror_request` swaps `"p1"` / `"p1: ..."` JSON strings (`mirror.py:13-30`). Requires the usual `p1a: Name`, `p1: Name` shapes.
- **`|player|` renaming** (`me`/`opponent`, p1 line first) because poke-env takes the first `|player|` line as its own side when neither name is ours (quirks.md, "poke-env takes the first |player| line").
- **popup watcher** (`rollout_battle.Popups`): aborts a battle on a warning containing "popup".

## 7. How training launches and connects today

- **Server processes**: `ShowdownServer(engine, None, log_path)` per actor (`RL\rl\rollout.py:143-152`; `PCSA\sim\server.py:46-124`): `node pokemon-showdown start --skip-build --no-security <port>`, cwd = Showdown dir, port = free port chosen via bind(0). Ready when stdout contains `now listening on` (`server.py:105-106`) and the port accepts a TCP connect. Kill: `taskkill /PID <pid> /T /F` on Windows (`server.py:112-124`, `rollout.py:_reap`). `Rollout.info()` exposes `port` and `node_pid` (`rollout.py:166`).
- **Lean mode**: `PCSA_PS_SUBPROCESSES=0` (default in `start_actors`, `rollout.py:343-358`) appends a block to `Showdown/config/config.js` (`ensure_lean_server_config`, `rollout.py:307-334`) so each server runs in one process (no 9 helpers). A face does not need this.
- **Scale**: `actors` 2-16 per config (most 4, 10, 16; league run 5; `configs\*.yaml`), each a Ray actor with its own server; `lanes: 6` concurrent battles per actor (`RL\rl2\lanes.py`); `Lane.arena(key)` caches one `AsyncArena` per key per lane (seat A and B at least, so >= 12 websocket connections per server, more with extra scripted keys). Preview-playout workers of the live player also start a server + ArenaPlayers (`PCSA\players\preview_worker.py:111-137`).
- **Format**: `Engine.locate(config.agent.sim.format)` -> `gen9championsbssregmc` (`PCSA\sim\engine.py:20-30`, `agent.yaml:109`); `Engine` also requires the pinned Showdown build + `.pcsa-built` marker; `engine.format` is passed to every ArenaPlayer as `battle_format`.
- **Teams**: strings (Showdown export text) per job (`Job.team_a/team_b`, `rl/rollout.py:60-70`); `arena.teams.exports, arena.teams.history = [team], []` per battle (`lanes.py:~265`, `rollout.py:~210`); `ExportTeams.yield_team` parses and `join_team`s to packed text, sent by poke-env with `/utm` before `/challenge`/`/accept`. Champions stat points ride in the EVs field (`PCSA\battle\teams.py:60-72`, team.txt shows `EVs: 1 HP / 32 SpA / 1 SpD / 32 Spe`, Level 50). Teams are validated beforehand by `validate.cjs`, not by the server path we use.
- **Starting a battle**: `first.battle_against(second, n_battles=1)` (`RL\rl\rollout_battle.py:55-65`, `PE\player\player.py:595-640`): first challenges (sits p1; the `index % 2` job parity alternates seats, `lanes.py:~290`), second `accept_challenges`. Wait for result with timeout 120 s (`BATTLE_TIMEOUT_S`, `rollout.py:34`) then `only_battle(arena)` requires exactly one finished battle (`rollout_battle.py:68-72`); `battle.forget` clears `arena.battles`.
- **Results**: `battle.won/lost/turn/player_role`, `battle._replay_data` joined as `Episode.protocol` (`rollout_battle.py:75-80`, `rollout.py:117-125`).
- **One config switch**: the places to swap are `Rollout.__init__` (`rollout.py:143-152`: `Engine`, `ShowdownServer`), `ArenaPlayer.__init__` (`server` arg only), `PCSA\sim\server.py` (`ShowdownServer.configuration/port/process.pid/output/start/stop`), `Engine.locate`, `ensure_lean_server_config` (skip), and the standalone harnesses `PCSA\run\sim_command.py:74-78`, `RL\llm\play.py:317`, `RL\llm_jetson\play.py:103`, `preview_worker.py:127`.

## 8. Easy-to-miss things a replacement must reproduce

1. **Per-seat views and HP text.** Server sends each player a channel-filtered log (`SD\server\sockets.ts:261`, `extractChannelMessages`); `|split|` never reaches clients. Own side: `hp/maxhp[ status]`; other side: `pct/100[y|g|r][ status]` with `pct = floor(100*hp/maxhp) || 1` in the champions mod (`SD\sim\pokemon.ts getHealth`), `y/r` at exactly 20 (`r` if hp*5<=maxhp), `g/y` at exactly 50. **Our `_fraction` helpers do `float("100y")` -> ValueError -> None**, i.e. at exactly 20% and 50% the opponent HP update is silently dropped in `features_log.py:93-101` and `turns.py:49-58`; reproducing the suffix reproduces that behaviour (poke-env itself strips non-digits, `pokemon.py:542`). `0 fnt` for fainted.
2. **Details strings.** `Species[, L50][, M|F][, shiny]`, `Species-Forme`, Mega as `Garchomp-Mega-Z`; level omitted at 100; shiny only in own request. Preview lines mask some formes with `-*` (Greninja, Urshifu, Gourgeist, Xerneas...: `SD\data\rulesets.ts:643-652`). `identifies_as` renames the preview entry on first switch, so switch details must resolve to the same species as the earlier `|poke|` line.
3. **Idents and nicknames.** Protocol idents `p1a: Name`, request idents `p1: Name`; name = nickname or species name. poke-env builds teams from `ident`; `ValueError` if a side exceeds `|teamsize|` (`abstract_battle.py:312-317`). Our teams carry nicknames only when `name != species` (`PCSA\battle\teams.py:67`).
4. **Order of lines is data.** `|move|` (+`[from]`, `[still]`, `[notarget]`, `[miss]`, `[spread]`), then effects, `|-damage|` with `[from]`/`[of]`, `|-crit|` BEFORE the damage line (belief `hits.py:120-122`), `|upkeep|` before `|turn|`, switches at turn start before moves, replacement `|switch|` after `|faint|` (turns.py treats a switch after a faint as a replacement via `side.fainted`, `turns.py:100-102`). `|detailschange|` precedes `|-mega|` in Showdown (`SD\sim\pokemon.ts:1464-1486`): observed_form and the LL tracker assume either order but `ChampionsBattle` inserts its own `detailschange` before `-mega` for phone logs (`adapter.py:75-79`), so a doubled change is possible on the p2 path.
5. **`|-crit|` presence.** `Player2.crit_reported = True` for simulator data (`rl2\player.py:110`); the belief uses it to rule out crits. Also needed: `-miss`, `-immune`, `-fail`, `-hitcount`, `-activate` text.
6. **Blank lines and timestamps.** Showdown writes `|` (empty) lines and `|t:|N` before turns; `|t:|` never reaches battle data, blank lines do (harmless; `turns.py` and `features_log` skip kinds they do not know).
7. **Inactivity timer.** `Timer`/`VGC Timer` rules exist in the format but the timer only runs after `/timer on` or `Config.forcetimer` (`SD\server\room-battle.ts:607`, `config.js:381` false), and we never send `/timer on`. So no `|inactive|` / `|inactiveoff|` lines in training; poke-env tolerates them (`abstract_battle.py:1086-1090`).
8. **Format rules emitted as `|rule|` lines.** Flat Rules (Species, Item = 1, Nickname Clause, Adjust Level = 50, Picked Team Size = Auto = 3, Cancel Mod, VGC Timer; `SD\data\rulesets.ts:39-43` and champions mod `rulesets.ts:34-39`). Request side is cut to 3 after `/team` (`SD battle.ts:2803`) and `|teamsize|pN|3` goes before `|start` (`battle.ts:2723-2729`); the `|teampreview|3` line precedes the preview request (`battle.ts:1431`).
9. **Request/log ordering.** In the sim stream `update` (log) is sent first, then each side's request (`SD\sim\battle.ts sendUpdates`); the server forwards the log as one room message and the request as a separate `|request|` message with `rqid` inserted (`room-battle.ts:796-810`). poke-env decides on the `|request|` message; `battle.turn` must already include the new `|turn|`, so the face must send the log chunk BEFORE the request. A `{"wait":true}` request makes poke-env skip choosing (`_waiting`).
10. **Request-side consistency.** Choice lock (Choice items, Encore, locked moves return only that move with no pp: `pokemon.ts getMoves` lockedMove branch), disabled flags (Taunt/AV/PP 0), `trapped`/`maybeTrapped` only when a switch is available, `Struggle` row `{"move":"Struggle","id":"struggle","target":"randomNormal","disabled":false}`, `Recharge` row `{"move":"Recharge","id":"recharge"}` (no pp). These are consumed through `available_moves_from_request`.
11. **`[from]`/`[of]` tags** drive item/ability reveals in poke-env (`_check_damage_message_for_item/ability`, `_check_heal_message_for_*`, `-item`, `-activate` special cases) and belief evidence (`evidence.py:40-45`: `[of] ` and `[from] ability: `). Parts must be separate pipe fields exactly as in Showdown (`len(split_message) == 6` checks).
12. **`-mega` shape**: `|-mega|p1a: Garchomp|Garchomp|Garchompite Z` (species, then stone; champions mod: `SD\data\mods\champions\scripts.ts:91`, `sim\pokemon.ts:1486`). `PCSA\battle\game.mega_form_for_stone(parts[3], parts[4])` expects (species, stone) in that order (`moves.py:39-40`, `features_ll.py:292`).
13. **Request `details` after Mega** must reflect the new forme or `ChampionsBattle.current_request` rewrites it from the protocol (`adapter.py:121-141`, solves stats from points); a face must give consistent request stats for the Mega form.
14. **Rating/`|raw|`**: unrated challenges emit no rating `|raw|`; `|player|pN|name|avatar|` has 6 parts when printed by the sim (`SD battle.ts:3313`, rating `''`). `_player_username` comparison uses the `/trn` name verbatim.
15. **Battle count semantics.** `max_concurrent_battles=1` per ArenaPlayer: a second battle room appearing for a player blocks on `_battle_count_queue`; rooms must be released with `|win` + client `/leave`.
16. **Ties.** `|tie` -> `won=None`: `_settle` leaves the episode result None unless it adds the terminal reward (`rl/rollout.py:118-122`).

## Low-latency observation extras (LL)

Reads beyond the base features (all in `LL\rl2\features_ll.py`): Battle `last_request` (`active[0].moves[].id/pp/maxpp/disabled`, `forceSwitch`; :413-414,684), `Pokemon.stats`, `max_hp`, `current_hp` for our mons; `mon._temporary_types` (:442); `mon.moves` as a dict (`.items()`/`.values()` order matters: :487-489); `Move.max_pp/current_pp` for non-active (:504-505); Tracker over `_replay_data` (:225-300). It claims "every input is a direct read of the poke-env Battle / request" (module docstring :1-6) - so the face must still build real poke-env Battles unless LL is rewritten. No new protocol kinds beyond the base set.

## Open questions / not verified

- No recorded real Showdown wire log of a full room join was found in the repos; the init sequence (`|init|battle`, `|title|`, `|j|`, `|player|`, `|gametype|`, `|gen|`, `|tier|`, `|rule|`, `|clearpoke|`, `|poke|`, `|teampreview|3`, request) is taken from `SD\sim\battle.ts` and `SD\server\room-battle.ts`, not captured. `Episode.protocol` in `RL\runs\*` holds per-seat protocol text and would be a golden file, but I did not open run artifacts (pickled `.pt`).
- poke-env's `Move.max_pp = pp*8//5` versus Champions' `maxpp`: whether the champions mod reports `pp*8/5`, or `fallback_pp_by_gen9_base_pp` (`PCSA\battle\moves.py:75-84` suggests Champions PP differs from gen 9). Training features use poke-env's value; LL uses the request's. Not compared.
- Exact Champions-mod emission of `|-crit|`, `-hitcount`, `-miss`, `|-activate|` variants was not enumerated (only the base sim). The list in 4.2/4.3 is "can occur" by judgement, not exhaustively derived.
- `rl2/lines.py`, `matrix.py`, `matrix2.py`, `search_*` and `belief/*` were scanned by grep for attribute and kind reads, not read line by line; counts are lower bounds. `research\pcsa-belief` is outside the stated sources but is imported at runtime (`rl2/belief_features.py:46-61`).
- How a bo3-default format reacts to `/challenge <id>, gen9championsbssregmc` without `bo3` (evidence: sim tag `battle-gen9championsbssregmc-709` from `pcsa\output\sim\20261108-160750-...\results.json`, so single battle); `format_is_bestof` is False.
- poke-env's `|showteam|`, `|inactive|`, bo3 `>game-` rooms are not exercised by training; not checked for the face.
- Which of the 4 `search_teacher` configs is still used in training (the search oracle runs Showdown's JS in-process, outside the face).
- Behavior of `Illusion`/`Zoroark` and `Commander` paths in poke-env were not checked against Reg M-C legality (probably absent in the pool).
- The memory note "Showdown 16 servers ~1,100 dec/s" and "~1.6 GB per server" come from the user's memory index/config comment (`configs\*.yaml` "~1.6 GB each"), not measured here.
