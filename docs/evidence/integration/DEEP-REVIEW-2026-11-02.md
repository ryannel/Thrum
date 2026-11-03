# Deep holistic review of the merged foundation

2 November 2026. Three bounded, independent read-only reviews examined the merged `main` foundation at `fc591f8`: (1) core clock, authority, world, host, replica and multiplayer contracts; (2) native source capture and test-store correctness against C10; (3) plan, maturity, package boundaries, target checks and developer journey. The coordinator checked the live clean-copy GitHub CI matrix and integrated the fixes. This review is evidence about these paths, not a claim that every future engine subsystem is implemented or that historical benchmarks were rerun.

## Findings and disposition

| Priority | Finding | Disposition |
| --- | --- | --- |
| P1 | [CI](../../../.github/workflows/ci.yml) ran offline Cargo before fetching locked dependencies. All three clean GitHub runners stopped before tests on missing `hashbrown`. | Added pinned `cargo fetch --locked` before `./dev check`; the next Linux and macOS jobs passed. |
| P1 | [Repository checks](../../../scripts/repo_check.py) decoded UTF-8 Markdown with the Windows default `cp1252` encoding, then emitted platform-specific backslashes in a tested manifest diagnostic. | Explicit UTF-8 reads/writes in the checker and [design hash tool](../../../scripts/design_hashes.py); normalized repository-relative diagnostics to `/`. |
| P2 | [Source inventory](../../../crates/thrum-source/src/inventory.rs) excluded only exact `.git`, `.engine` and `target` names. A case-fold alias such as `.ENGINE` could enter a candidate on a case-sensitive filesystem and overlap a private/generated root on a case-insensitive one. | Reject reserved-root aliases before inventory; preserve exact exclusions. A focused regression test passes, and staging uses the same inventory gate. The case comparison avoids a per-entry folded-string allocation. |
| P2 | [Maturity prose](../../design/MATURITY.md) and two [ledger](../../design/maturity.json) summaries described early I1 proofs as current behavior after selected pool-charged hosts were built. The contributor instructions also omitted the bounded host. | Updated the scoped description and current digests without promoting broad implementation, device or visual status. The universal process-memory cap remains retired. |
| P2 | Unix-only mode setup left a mutable directory builder on Windows, where strict Clippy rejected it. | Split Unix private-mode creation from the other-platform directory creation path in [source budget code](../../../crates/thrum-source/src/budget.rs). |

The core-runtime reviewer found no qualifying introduced defect in the selected clock, authority, publication, replica or two-peer paths. Its scoped Rust suite passed 141 tests with six development cost fixtures ignored. The source reviewer found no further qualifying defect after the alias fix; its suite passed 102 tests with 33 ignored fixtures. A second-pass read-only review of the CI, checker and source-path fixes found no new issue. These results do not prove absent code is safe or make test-only mechanisms production APIs.

## Whole-path assessment

The selected local/in-memory headless path is internally coherent and ready to be a consumer of source/run work. Its 60 Hz default world update cadence is independent of presentation refresh. The [sustained cost record](../i1-headless/SUSTAINED-HOST-COST.md) advances 30,000 callbacks on a synthetic clock with a small rule and local Apple Silicon; it does not qualify a representative game, network, saved restart, named device or whole-engine frame ceiling.

Source capture, F3 validation and SQLite proofs have substantial negative-case coverage, but the store remains `cfg(test)` and no production code can accept a source revision. The [256-descriptor probe](../i2a/SOURCE-256-CANDIDATE-COST.md) exercises one game record and one required edge, without all-type distribution, maximum simultaneous edges or combined editor/store peak. The broader [editable game and save](../../BUILD-PLAN.md#after-the-foundation), real transport, visible phone shell and rendered game gates remain unchecked. Distinct authoring source, ready build, run and private save roots in C10 remain the shared authority model.

The next complete milestone is one bounded editable game through coordinated file application, durable accepted-source admission, a pinned cook/load, local run, private save and process restart/restore. Exercise failed validation/cook, same-token retry, stale generation, interruption, corrupt or incompatible save, old-reader retention and final release in that path. Measure its edit-to-ready time, allocations/RSS and sustained callback tails on a named machine before generalizing source or performance claims. Reuse the same rules when real transport joins the first rendered game.

## Verification record

`RUSTC_WRAPPER= ./dev check` passed after the code fixes; `./dev check` includes repository checks, Python tests, formatting, Clippy and workspace Rust tests. [GitHub CI run 41463201029](https://github.com/ryannel/Thrum/actions/runs/41463201029) passed on Linux, macOS and Windows at code checkpoint `5590cf6`, including the portable-core Wasm compile on Linux. The local review did not run a Windows compiler, browser runtime, phone app or device; CI compilation and tests do not qualify those runtimes or their performance.
