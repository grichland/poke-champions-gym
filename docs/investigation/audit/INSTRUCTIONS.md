# Audit instructions (read fully before starting)

## Purpose

We are about to build a Pokémon Champions singles training engine from the Rust library poke-engine. Before any
code is written, the owner wants every difference between poke-engine and the real rules found up front, by
reading, not discovered later by failing tests. You audit one slice. Your rows go into the project's gap register,
so a missed difference is a bug that ships, and a false "MATCH" is worse than an honest "UNSURE".

## The two sources

- **The reference (correct by definition):** Pokémon Showdown, mod `champions`, format
  `[Gen 9 Champions] BSS Reg M-C` (singles, bring 3 of 6, level 50, Megas, no Tera).
  Path: `C:\Users\user\Code\pokemon_champions\pcsa\.tools\vendor\pokemon-showdown`
  - Base data and behaviour: `data\moves.ts`, `data\abilities.ts`, `data\items.ts`, `data\conditions.ts`,
    `data\pokedex.ts`, and the simulator in `sim\` (`battle.ts`, `battle-actions.ts`, `pokemon.ts`, `side.ts`,
    `field.ts`, `battle-queue.ts`).
  - Champions overrides (these win over the base): `data\mods\champions\` (`moves.ts`, `abilities.ts`, `items.ts`,
    `conditions.ts`, `scripts.ts`, `pokedex.ts` if present). An entry with `inherit: true` changes only the listed
    fields. Always check both the base entry and the override.
- **The engine under audit:** poke-engine, commit f7b147f.
  Path: `C:\Users\user\Code\pokemon_champions\research\pcsa-rl\poke-engine-eval\poke-engine\src`
  - `choices.rs` (move table, about 20,700 lines; `apply_champions_modifiers` near the end changes move data for
    Champions), `genx\abilities.rs`, `genx\items.rs`, `genx\generate_instructions.rs` (turn resolution),
    `genx\damage_calc.rs`, `genx\choice_effects.rs`, `genx\state.rs`, `genx\base_stats.rs` (Mega data),
    `state.rs`, `pokemon.rs`, `instruction.rs`.
  - It is built with the Cargo features `champions` and `bss`. Read `Cargo.toml` once to see which generation
    feature those imply, and only count code that is compiled under those features (`#[cfg(feature = ...)]`).
  - Identifiers are upper-case without punctuation: `Choices::MAKEITRAIN`, `Abilities::TOUGHCLAWS`,
    `Items::LIGHTCLAY`, `PokemonVolatileStatus::...`. Grep the identifier across all of `src`, not just one file:
    an effect is often implemented far from the data table.
  - A move's table row may reference a `volatile_status`, `side_condition`, `status`, `boost`, `secondaries`,
    `drain`, `recoil`, `heal`, `crash` field. A token existing in an enum does not mean it has behaviour: follow
    it to the code that reads it.

## Rules

- Read-only. Tools: Read and Grep only. Do NOT run python, node, cargo or any program: this machine must not run
  compute. Do not use the shell to process files.
- The only file you may create is your output file (named in your task).
- Never open `.env` files. Never stop a process.
- Work through EVERY row of your slice. Do not sample. Do not skip rows because they look simple. If you run short
  of budget, write what you have and list the unfinished ids at the top under "NOT AUDITED".

## Known core differences (do not re-report these as new findings)

The engine was built for search and will have its core rewritten. These are already known. When a row depends on
one, give verdict `CORE` and name which, then still audit everything else about the row.

- K1 damage roll: engine uses 0.925 x max (float); Showdown uses 16 integer rolls with truncation at each step.
- K2 multi-hit count fixed (2-5 hit moves hit 3 times; 5 with Skill Link; 4 with Loaded Dice).
- K3 confusion never expires; K4 lock-in moves (Outrage) fixed length; K5 partial trapping never expires.
- K6 PP only decremented below 10; no Struggle.
- K7 crit only modelled as a branch in some cases.
- K8 Mega-turn speed uses pre-Mega speed.
- K9 engine has no hidden-information model (not relevant to rules).
- K10 the engine enumerates outcome branches instead of sampling: fine, as long as each branch and its
  probability is right. A wrong probability or a missing branch IS a finding.

## What to check for each row

Moves: type, category, base power, accuracy (including moves that never miss), priority, PP, target, flags that
change behaviour (contact, sound, punch, bite, slicing, bullet, powder, wind, protect, bypasses Substitute),
secondary effects and their chances, self effects, stat changes, status, volatile statuses and their durations,
side or field effects and durations, recoil, drain, healing, crash damage, variable power, fixed damage, multi-hit,
charge and recharge turns, switching effects, conditions for failing, immunities, interaction handlers
(`onTry`, `onHit`, `onModifyMove`, `onBasePower`, `onAfterHit` and so on), and every Champions override field.

Abilities and items: every event handler in Showdown's entry (`onStart`, `onModifyAtk`, `onBasePower`,
`onDamagingHit`, `onResidual`, `onTryHit`, `onSwitchOut`, and so on), each with its exact multiplier, chance,
condition and timing, against the engine's code for the same thing. For items also: consumption, Knock Off
interaction, when a berry triggers, Mega stones.

## Verdicts

| Verdict | Meaning |
|---|---|
| MATCH | You read both sides and they agree on everything checked, apart from the known core differences |
| PLAIN | No behaviour beyond table numbers, and the numbers agree |
| DATA | A number, type, flag, chance or duration differs |
| LOGIC | Behaviour is implemented but differs (wrong condition, timing, multiplier, missing case) |
| PARTIAL | Some of its effects are implemented and some are missing |
| MISSING | Not implemented: absent from the engine, or present in an enum or table with no behaviour |
| CORE | Correct only once a known core difference (K1-K10) is fixed; nothing else wrong |
| UNSURE | You could not determine it; say what you would need |

## Output file format

Start with a summary: slice, rows audited, count per verdict, and the five most important findings.

Then one table, one row per id, in the same order as your slice:

`| id | used | verdict | Showdown (file:line, what it does) | engine (file:line, what it does) | difference | fix size | confidence |`

- `used`: the `used` column from the CSV (1 if our data uses it).
- `difference`: exact and concrete ("accuracy 95 vs 100", "no thaw on use", "heals 1/16 after poison damage
  instead of before"). Empty for MATCH and PLAIN.
- `fix size`: S (a number or flag), M (a new arm or condition, under about 30 lines), L (new mechanism).
- `confidence`: high / medium / low.

Keep cells short. Then a section "Notes" for anything that does not fit a row, including engine behaviour you
noticed that is wrong but belongs to another slice.

Finally reply to the manager with: counts per verdict, the ten most important findings, and any ids not audited.
Under 250 words.
