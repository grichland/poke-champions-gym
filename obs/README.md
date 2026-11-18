# obs

The observation: `obs::observe(&Battle, ReqKind, &mut ObsBuffers)` fills the model's input arrays from a
Battle (what one player knows), laid out as in `features_proposed.csv`. That file is a copy of
`pcsa-former/features_proposed.csv` (17 Nov 2026, sha256 `1ac2d02d...acac34`); `src/layout.rs` is generated
from it by `tools/obs/gen_layout.py`, and `src/learnsets.rs` by `tools/obs/gen_learnsets.py` from Showdown's
Champions learnsets. Rerun both after the CSV or the game data changes.

## Shapes

One observation is seven plain arrays, each contiguous in C order (`ObsBuffers` is `#[repr(C)]`). For a
batch, `observe_into(b, kind, ObsOut { .. }, true)` writes the same arrays into caller-provided flat
slices, so a binding can pass slices of numpy or torch arrays of shape `[N, ...]` without copying per
element.

| Array | Shape | dtype | Contents |
|---|---|---|---|
| `rows` | [12, 300] | f32 | Pokémon rows: 0..6 ours, 6..12 theirs (order below) |
| `row_ids` | [12, 10] | i32 | per row: species, item, ability, 4 moves, 3 possible abilities |
| `field` | [66] | f32 | field vector |
| `last` | [17] | f32 | last completed turn |
| `last_ids` | [4] | i32 | our move, our switch-in species, their move, their switch-in species |
| `mask` | [11] | u8 | legal actions (1 = legal): moves 0-3, the same with Mega 4-7, switch to picks 0-2 |
| `opp_prior` | [13] | f32 | how available each opponent action is, from the Battle (`obs::opp_prior`): row 6's move slots 0-3, the same with Mega 4-7, switch to rows 7-11 |

Row order (`obs::row_order`): ours in pick order. Rows 0..3 are the three brought, in the order of the
picks (lead = row 0), rows 3..6 the three not brought, in team order; at a preview request (no picks yet)
all six in team order. The active Pokémon is marked only by its is_active column (F3-9), not by its row.
So switch action 8 + p is row p, and move actions 0-3 (and 4-7 with Mega) are the move slots of whichever
of rows 0..3 is active. Theirs: row 6 their active, then the ones seen, then the unseen, each in preview
order. Empty rows are all zeros. Matchup, damage and move-slot features are computed for each row against
the opposing active, whatever its row.

Column offsets: `obs::layout` (one constant per feature id, e.g. `layout::F3_1`, and `FEATURES` with
block, offset, size, encoding and range). Move-slot features sit at `MOVE_BASE + MOVE_STRIDE * slot + offset`
(154 + 36 * slot).

Id vocabularies (`obs::vocab`), every one with 0 reserved:

| Vocabulary | Size | Numbering |
|---|---|---|
| species | 393 | 0 pad / unknown; `SpeciesId(i)` is i + 1 (392 species and forms, Megas included) |
| moves | 516 | 0 empty / none / unknown; `MoveId(i)` is i (515 moves) |
| items | 168 | 0 unknown; 1 known to hold nothing; `ItemId(i)` is i + 1 (166 items) |
| abilities | 319 | 0 unknown / pad; `AbilityId(i)` is i (318 abilities, "No Ability" included) |

The learnset table (M2-1) is fixed, not part of each observation: `obs::learnset_table(&mut [u8])` fills a
[393, 516] u8 table (species id x move id, 1 = can learn).
