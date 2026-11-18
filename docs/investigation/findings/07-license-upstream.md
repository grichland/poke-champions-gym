# 07 License and upstream health (poke-engine fork)

Investigation date: 16 Nov 2026. Upstream checked: github.com/pmariglia/poke-engine, main at f7b147f (11 Oct 2026).

## Summary

- **License:** poke-engine is MIT (LICENSE in the local checkout and on GitHub). A fork must keep the copyright line "Copyright (c) 2024 pmariglia" and the MIT permission notice in copies or substantial portions. Adding our own copyright line is fine.
- **foul-play is GPL-3.0.** We may read it for ideas. We must not copy code, tables or structure from it. The poke-engine checkout shows no GPL notice or foul-play-derived code in the files checked (the grep hits were README and one test file, which I did not read in full).
- **Showdown is MIT** (Copyright (c) 2011-2026 Guangcong Luo and other contributors). Generated data tables and ported mechanics must carry the Showdown notice.
- **Dependencies** checked so far are all MIT or MIT OR Apache-2.0. No copyleft found.
- **Upstream health:** active but slowing. 58 stars, 37 forks, 7 contributors (pmariglia has 894 commits, the others 1 to 5 each). Last commit to main: 11 Oct 2026. No commits on main since (none after 11 Oct found). Latest tag v0.0.48. 11 open items (4 PRs). No visible maintainer replies on the Champions-related PRs or issues.
- **Champions:** partly in upstream (mega evolutions, Champions data in v0.0.48 changelog). Open upstream PRs #36 and #31 touch Champions data. Issue #32 asks for a PyPI release with Champions megas. No visible response.
- **Recommendation:** fork from SHA **f7b147f13637ceecc1b0d1e9172ec232af4d45d0** (main head), not the tag. Pin the SHA. Keep upstream as a remote, sync by cherry-pick, keep Champions work in a feature-gated module, and send small generic bug fixes upstream as PRs.

## 1. License

**Read today:**
- Local `LICENSE` (C:\Users\user\Code\research\pcsa-rl\poke-engine-eval\poke-engine\LICENSE): MIT License, "Copyright (c) 2024 pmariglia", standard MIT text.
- Local `Cargo.toml`: `license-file = "LICENSE"` (no SPDX `license` field), version 0.0.48, authors pmariglia.
- GitHub API https://api.github.com/repos/pmariglia/poke-engine: `license.spdx_id` = MIT. Archived: false. Default branch main. Created 2020-03-22.
- PyPI https://pypi.org/pypi/poke-engine/json: latest 0.0.48 (uploaded 2026-07-19), `license` field is **null**. Minor: the Python package metadata does not state a license. The LICENSE file in the repo is the source of truth.

**What a fork must keep (MIT, general reading, not legal advice):** the copyright notice and the permission notice in all copies or substantial portions. In practice: keep the upstream LICENSE file unchanged, and add a NOTICE or header line for our additions. Do not remove the pmariglia copyright.

**Dependencies (from Cargo.lock and Cargo.toml):**

| Crate | Locked version | License (crates.io, read today) | Note |
|---|---|---|---|
| dashmap | 6.1.0 | MIT (latest 6.2.1 read: MIT) | Direct |
| clap | 4.5.4 | MIT OR Apache-2.0 (latest 4.6.7 read) | Direct |
| rand | 0.9.4 | MIT OR Apache-2.0 (latest 0.10.4 read) | Direct; locked is 0.9.4, latest is 0.10.4 |
| web-time | 1.1.0 | Not fetched (recall: MIT OR Apache-2.0) | wasm target only |
| pyo3 | 0.29.0 | Not fetched (recall: MIT OR Apache-2.0) | Only in poke-engine-py workspace member |
| Other locked transitive crates (anstyle, bitflags, proc-macro2, syn, windows-sys, wasm-bindgen, etc.) | various | Not checked individually (recall: almost all MIT OR Apache-2.0) | Need a `cargo license` run on a machine that can build |

No copyleft found in the checked set. The transitive list is unchecked.

Caveat: the crates.io figures are the latest versions, not the locked ones, so they are only a proxy.

## 2. foul-play (GPL-3.0)

**Read today:**
- https://github.com/pmariglia/foul-play/blob/master/LICENSE: "GNU GENERAL PUBLIC LICENSE / Version 3, 29 June 2007 / Copyright (C) 2007 Free Software Foundation, Inc."
- GitHub API https://api.github.com/repos/pmariglia/foul-play: SPDX GPL-3.0, not archived, pushed 2026-10-12.

**What it means for us:**
- Reading foul-play for ideas is allowed.
- Copying code, data tables, or substantial expressions from it into our fork would bring GPL-3.0 obligations into a MIT-or-proprietary project. We should not do that. Write our own implementation from the game mechanics, from Showdown (MIT), or from the games themselves.
- Re-implementing an idea in our own code is generally fine. Copying text, structure, or a near-literal translation is not.

**Does poke-engine contain GPL-derived code or data?**
- The local checkout has no GPL notice file. A grep for `foul-play|foul_play|showdown|GPL` (excluding target and .git) hit only README.md and `tests/test_battle_mechanics.rs`. I did not read the test file in full. Next step: read it and confirm it has no copied foul-play code.
- README.md mentions Showdown only as a comparison ("nowhere near as complete or robust as the PokemonShowdown battle engine"), not as a credit or source.
- I did not verify the history of upstream commits for copied foul-play code. That would need a full diff review.

## 3. Pokémon Showdown license

**Read today:** https://github.com/smogon/pokemon-showdown/blob/master/LICENSE: "The MIT License (MIT)", copyright line "Copyright (c) 2011-2026 Guangcong Luo and other contributors http://pokemonshowdown.com/".

**Implication:** Generating data tables from Showdown's data files and porting mechanics from its source are allowed under MIT. Keep the Showdown copyright and MIT notice with any copied data or substantial code. Note that Showdown's data files in `data/` may be under the same MIT license (not checked file by file).

## 4. Upstream health

**Repo metrics (read today, GitHub API):**
- Stars 58, forks 37, open issues 11 (includes PRs), created 2020-03-22, last push 2026-10-15 (see note below).
- Contributors (API, read today): pmariglia 894, jpf4e 5, Henauxg 3, kelvinfkr 2, SPGesus 2, larry-the-table-guy 1, pulkitahuja0 1.

**Commits on main (API https://api.github.com/repos/pmariglia/poke-engine/commits?per_page=100):**
- The first 25 are readable (newest f7b147f, 11 Oct 2026, back to 4909360f "cargo fmt", 30 Aug 2026). The response was truncated, so the true total is unknown. Over the last 3 months (since mid-Aug), at least 25 commits. Last 12 months: at least 25, total unknown.
- Commit timing: burst 10 to 11 Oct; several in Sep (Champions mega evolutions e98de3f7, 6 Sep; Spikes fix; Ice Face fix); Aug (wasm for mcts, 31 Aug).
- Commits since f7b147f: **none found.** Compare API (f7b147f...main) returned "identical, ahead 0, behind 0". Commits API with `since=2026-10-11T23:59:59Z` returned an empty list. So the local checkout is current with main as of today.
- Note: `pushed_at` says 15 Oct 2026, but no commit exists after 11 Oct. Possibly a non-commit push (tags, PR refs, or a rebased branch). Unverified.

**Tags (https://api.github.com/repos/pmariglia/poke-engine/tags):** 30 tags, v0.0.19 through v0.0.48. Latest v0.0.48 at bcf13823abc162a608e187b26bbf683f759f385e, commit dated 19 Jul 2026 (API read today).

**Releases:** The releases endpoint returned an empty list. Tags are used, not GitHub releases. PyPI latest is 0.0.48 (19 Jul 2026).

**Changes after the tag v0.0.48:** The Aug to Oct commits (about 25+) come after the tag and are not in the tag. These include Champions mega evolution data (e98de3f7, 6 Sep), Spikes per-layer fix, Ice Face fix, Flail base power fix, Moldbreaker/unaware/iceface fixes, deltastream (b2419e42, 10 Oct), sniper ability (f7b147f, 11 Oct).

**Changelog (local CHANGELOG.md, read today):** v0.0.48 (2026-07-19) lists Champions items: Reg M-B Champions data and abilities, Champions base stat calc for stat points, mega evolving, Champions thaw/paralysis/sleep changes, unseenfist and piercingdrill. This is from the changelog only.

**Gen 9 and BSS in Cargo.toml (local, read today):** features `champions`, `bss = ["champions"]`, `terastallization = ["gen9"]`, and gen1 to gen9 feature flags. README (read today): "Generations 4 through 8 are available." The README and the feature flags disagree about gen 9. Treat gen 9 as partial.

**Open items (API read today, https://api.github.com/repos/pmariglia/poke-engine/issues?state=open):**

| # | Type | Title (short) | Opened | Notes |
|---|---|---|---|---|
| 41 | PR | Locking moves last 2 or 3 turns (Lompta) | 7 Nov | Newest |
| 40 | Issue | Black Sludge damage on non-Poison holders | 30 Oct | |
| 39 | Issue | End-of-turn order: status damage before Leftovers and Leech Seed | 30 Oct | |
| 38 | Issue | Rest succeeds at full HP (Showdown fails it) | 30 Oct | |
| 37 | Issue | Items enum full (u8): how to add items | 26 Oct | Design question |
| 36 | PR | Fix Champions move data, Mega Evolution data, Aura Guard (MSalman5230) | 24 Oct | Champions, updated 24 Oct |
| 33 | Issue | Gen 1 move effects missing | 15 Oct | |
| 32 | Issue | Release request: Champions megas on main not in any PyPI release | 11 Oct | Champions |
| 31 | PR | Speed ties; Champions Dire Claw: ex (haydenchiu) | 11 Oct | Champions |
| 29 | PR | Offer Struggle for exhausted moves (owlsowo) | 6 Oct | |
| 6 | Issue | Destiny Bond mutual KO | 4 May | Old |

Closed PR #35 is a duplicate of #36. Closed PR #30 (Transform/Imposter) merged or closed on 6 Oct.

**Responsiveness:** Comment arrays for issue #32 and PR #36 were empty (read today). No maintainer reply is visible on any Champions-related item. Last maintainer commit 11 Oct, about five weeks ago. Other contributors' PRs (#29, #31, #36, #41) have no visible merge.

**Health verdict:** Maintained but slow; bursts of activity; no maintainer response to open Champions requests. Expect no upstream merges on our timeline.

## 5. Contribution policy

- CONTRIBUTING.md at https://github.com/pmariglia/poke-engine/blob/main/CONTRIBUTING.md: **404 (not found)** read today.
- Local `.github/` contains only `workflows/publish.yml` and `workflows/tests.yml`. No PR template, no issue template.
- No CLA or DCO requirement found.
- No stated policy on Champions patches found. The README has no contribution section (read today).
- Signs of acceptance: pmariglia merged contributor PRs before (e.g. #26 by kelvinfkr, #19 by pulkitahuja0, #20 by SPGesus) and merged a kelvinfkr commit with "feat: add missing Champions mega evolutions" (Sep 2026). So outside contributions do get merged, but the maintainer has not replied to the most recent Champions PRs.

## 6. Forks and other projects

- Searches (WebSearch, read today) found no public fork of poke-engine that does Champions rules, and no gym-style or vectorised wrapper over poke-engine. This is not exhaustive; GitHub code search was not available.
- Unrelated Rust RL projects appeared in results (gymnasia, gmgn vectorised envs, RLGym Rust). These are not connected to poke-engine.
- **mikumiku37 (Smogon thread, read today):** https://www.smogon.com/forums/threads/3789199/ — posts by Shao (2 Oct 2026 onward) say:
  - mikumiku37 reached #1 on the Gen 9 Champions VGC 2026 Reg M-C ladder at 1857 Elo (Oct 2 post, quoted in search snippet and confirmed by thread fetch).
  - "The simulator is a Rust engine doing ~7k random battles/s on a single core." (Shao, 7 Oct 2026).
  - Open-sourcing: "not planning on open sourcing the battle engine or the weights" (Shao, 2 Oct). Other posters asked the author not to open source it.
  - "A technical writeup is also coming up soon." (2 Oct). A request for the writeup on 28 Oct got no reply in the fetched thread.
  - Verdict: there is a public self-description of a Rust Champions engine with no source and no writeup. Its code is not public, so we cannot read it. Do not assume it shares code with poke-engine. The search snippet's link to a different Smogon post (Wolfe_Alt6767, Rust Champions engine) is unverified and may be another bot.

## 7. Recommendation

**Fork point:** Fork from **f7b147f13637ceecc1b0d1e9172ec232af4d45d0** (main, 11 Oct 2026). Reasons:
- It is the newest upstream code, including Champions mega data (6 Sep) and the Oct bug fixes.
- The tag v0.0.48 (commit bcf1382, 19 Jul) is older and misses the Aug to Oct commits.
- Pin the SHA in our repo. Do not track main blindly.

**Rebase policy (proposed):**
1. Keep `upstream` as a git remote. Do not rebase the fork onto upstream main automatically.
2. Put Champions and any gym/vector work behind a feature flag or in new modules, so a sync touches few upstream lines.
3. Track each fork-only change as a small, labelled commit or patch file (for example `patches/NNN-description.patch`) so it can be reapplied after a sync.
4. Sync upstream only when needed (a bug fix we want or a new tag). Use cherry-pick for specific upstream commits; a merge of upstream main only at a planned point.
5. Send generic bug fixes (e.g. Rest at full HP, Black Sludge, Leftovers order, locking moves) upstream as small PRs. Expect no merge; keep the fork patch regardless.
6. Keep the MIT LICENSE file and the pmariglia copyright unchanged. Add our copyright line in a NOTICE file.
7. Do not port code from foul-play. Port mechanics from Showdown (MIT) with its copyright notice, or write from first principles.

## Not verified

- Full commit count and history beyond the first 25 commits (API response truncated).
- Why `pushed_at` is 15 Oct when no commit after 11 Oct exists.
- The releases endpoint returned an empty list; there may be GitHub releases not shown.
- Whether all data files in poke-engine derive from Showdown and carry its notice. Only the top-level LICENSE was read.
- Full read of `tests/test_battle_mechanics.rs` and a full history diff for foul-play-derived code.
- License of each transitive crate in Cargo.lock; web-time and pyo3 licenses are recall only.
- Locked versions of dashmap, clap and rand vs their latest versions: crates.io pages read were the latest versions; the locked versions' license fields were assumed the same.
- mikumiku37 engine: no public code, writeup, or verifiable details beyond the Smogon thread posts. The "Wolfe_Alt6767" Rust engine post was not fetched.
- Other forks: only a web search, no GitHub code or fork-list search.
- Gen 9 status: conflicting signals (README says gens 4 to 8; Cargo features include gen9 and terastallization). Not resolved.
- Legal interpretation of MIT and GPL obligations is general reading, not legal advice.
