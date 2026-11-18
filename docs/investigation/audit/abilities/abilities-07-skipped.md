# Audit: abilities-07-skipped (icebody)

Slice: 1 ability (Ice Body). Rows audited: 1. Verdicts: LOGIC 1.

Top findings:
1. Ice Body only heals in `Weather::HAIL`. Under the `champions` feature, Snow Warning sets `Weather::SNOW` (abilities.rs:1806-1810) and Snowscape sets SNOW (choice_effects.rs:1675). Showdown heals in `hail` and `snowscape`. In Champions, Ice Body therefore almost never works in the engine.
2. Heal Block is ignored. Showdown's Heal Block `onTryHeal` blocks the ability heal; the engine's ICEBODY arm never checks the HEALBLOCK volatile.
3. Residual order is wrong. Showdown heals in the weather residual (order 1), before Wish, Leftovers, Leech Seed and poison/burn damage. The engine heals in `ability_end_of_turn`, after the item step and after Wish and status damage.
4. Immunity to hail damage, the 1/16 amount and Cloud Nine / Air Lock are correct.

| id | used | verdict | Showdown | engine | difference | fix size | confidence |
|---|---|---|---|---|---|---|---|
| icebody | 1 | LOGIC | data/abilities.ts:1956-1969: `onWeather` heals `baseMaxhp/16` if effect is `hail` or `snowscape`; `onImmunity` hail returns false. `this.heal` truncates the amount. Weather residual is order 1 (conditions.ts:688, 724). No Champions override (grep of data/mods/champions finds none). | genx/abilities.rs:1288-1303: if `weather_is_active(HAIL)`, heals `min(maxhp/16, maxhp-hp)`. generate_instructions.rs:3003-3014: hail damage skips ICEBODY (also Ice type, Magic Guard, Overcoat). The `ability_end_of_turn` call is at :3213. | (a) SNOW is not accepted, so Snowscape and Snow Warning never trigger a heal (the HAIL-only gate is the main bug). (b) The heal ignores the HEALBLOCK volatile. (c) The heal happens late: Showdown does it first among residuals (weather, then Wish, Leftovers/items, Leech Seed, status damage, ...). The engine does it after the weather damage step, Wish and status damage, in the item/ability step. This only changes results when HP is capped or a KO would occur, for example heal-before-poison-damage at low HP, or full-HP Leftovers interactions. | S for (a): add `\|\| SNOW`. S for (b). M for (c), because it needs moving the weather-heal into the weather step. | high |

## Checked and matching

- Heal amount is `maxhp/16`, with integer division. This matches `baseMaxhp/16` truncated. Showdown's heal floors fractions and sets a minimum of 1 only when the amount is 1 or less, which is not reachable at level 50.
- No heal at full HP: the engine caps at `maxhp-hp` and skips when 0. Showdown's heal at full HP does nothing visible.
- Cloud Nine / Air Lock: `weather_is_active` (genx/state.rs:1509-1517) returns false when either active has Cloud Nine or Air Lock. Showdown's `isWeather` / `effectiveWeather` behave the same way. MATCH.
- Hail damage immunity: Ice Body is skipped at generate_instructions.rs:3010. MATCH.
- Weather end turn: the engine decrements the weather counter first (:2866-2883) and sets it to NONE at 0. Showdown ends the weather before running the residual on its last turn. Both skip the heal on the expiry turn. MATCH.
- Skipped with the engine's `hp == 0` guard (:3208). MATCH. Neutralizing Gas suppression (abilities.rs:1186) is also handled.

## Notes

- `weather_is_active` compares only the weather type and does not check `turns_remaining`. This is harmless here because expiry resets the type to NONE.
- The engine has no Ice-type Snowscape Defense bonus check in this slice, but damage_calc.rs:526 handles SNOW. That belongs to another slice.
- The heal block check at generate_instructions.rs:1645-1648 only stops moves with the `heal` flag. Other healing sources (Leftovers, Wish, Leech Seed, Ice Body, Poison Heal) are probably also not blocked. Not verified here and belongs to other slices.
