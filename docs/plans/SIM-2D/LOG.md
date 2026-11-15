# SIM-2D log

- 2026-11-14 M1 implementation. Added the binary32 profile number and vector, Philox2x32-10,
  portable state encoding, registered-component finite validation, and the committed world digest.
  The reference crowd uses registered edits and eight-tick engine history. Straight corrected Mac
  traces retain the timeline folds and checkpoints for 2,000, 5,000 and 10,000 bodies. The runner
  uses the production input codec and separate Mac, Chrome Wasm and simulator executables.
- 2026-11-14 Verified before the first checkpoint. Core and crowd target tests, exhaustive input
  conversion boundaries, metadata mutation coverage, Clippy and documentation checks passed.
  Chrome's platform-sine control diverged at tick 1. Full target qualification, costs and exact
  image verification follow at the source checkpoint.
- 2026-11-14 The documentation gate exposed ten dangling links in older CREATOR, CULLING and
  CLOUDS handoffs. Replaced the absent working-tree links with historical checkpoint/path references,
  preserving their evidence claims. This repair lets the repository documentation gate run.
- 2026-11-15 M1 API audit fixed default composite finite validation and canonicalized NaN
  encoding, preventing a diagnostic digest from observing a NaN payload. The registered-rule
  refusal test uses a nested component and the arithmetic tests exercise distinct NaN encodings.
- 2026-11-15 Full target qualification and two full-trace cost runs passed at `aa8dd9cb`.
  [M1 evidence](../../evidence/SIM-2D/M1.md) retains receipts, tests and costs. The retained folds
  stay `87496bd96b8c5be4`, `290bd104cb7a0409`, `8f66295acaf3bd0c`. Table-size growth explains
  the 5,000-body neighbour-scaling tripwires. Shared-host load is recorded with both repeats.
  Eight-tick replay at 10,000 costs 69 ms on the Mac and 73–86 ms in Chrome.

- 2026-11-13 Planned from the lead's task (row 16 of the module list); no words of the owner's were passed on, so
  the design quotes the owner's page of 10 November. Three milestones replace the brief the module list carried,
  whose text is in git (`git show 7a3694b8:docs/plans/ARCHITECTURE.md`). M1 waits on `SHOWCASE:M8`.
- 2026-11-13 Planner's decisions, both left to the planner by that brief. Rules compute in `f32` held to exactly
  rounded operations behind one engine-owned number type, not fixed point: Rust and WebAssembly both promise those
  operations bit for bit apart from a NaN's bits, and Box2D 3.1 and Rapier ship on the same footing. The queries
  go in a new crate, `thrum-sim2d`, beside `thrum-core`.
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/SIM-2D/planning-probe.md). A throwaway crowd
  of 2,000, 5,000 and 10,000 in those operations gave equal digests over 3,600 ticks on the Mac, the iPhone
  simulator and Wasm in Node and Chrome 157. The platform's `sin` and `powf` diverged between the Mac and Wasm
  but not between the Mac and the simulator, so the Wasm row is the one that discriminates and is where the
  control runs. `f32::max` and `min` never diverged in a run, so that class is refused by the check, not left
  to a trace.
- 2026-11-13 Planner's decision from the probe: query results come back in the grid's order (oversize, then cell
  row, column, slot), not slot order, which cost 25% to 37% more per tick. The cell size therefore becomes a
  constant of the rule set. The probe's tick at 10,000 packed bodies was 5.0 ms on the Mac and 5.6 ms in Chrome,
  so an eight-tick whole-world replay would be about 40 ms; M1 records the engine's own figure for `SESSION`.
- 2026-11-13 Planner's findings in the code. The V1 game's state hash goes through `std::hash`, which hashes
  `usize` at the machine's width and process type identities, so it cannot be compared across targets; the digest
  is a new function with its own encoding. `WorldRead` lets a rule read change ticks, so the digest covers them.
  The guard requires every package to inherit the workspace lints, so the profile is enforced by a per-package
  Clippy configuration, which the probe confirmed can refuse float methods by name.
- 2026-11-13 Researched: Box2D's determinism post (Catto, 2024), Rapier's determinism guide, Rust's `f32`
  documentation and RFC 3514, WebAssembly's nondeterminism page, Photon Quantum's fixed-point manual, Green's
  uniform-grid paper (2010) and the counter-based generators of Salmon et al. (2011). No source was found for how
  Vampire Survivors indexes its crowd.
- 2026-11-13 Checks: the done-when file passes the loop's validator; every check parses as shell and fails, as
  it should before the work exists. The verifier items were not dry-run, because the GPU is shared.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
- 2026-11-13 Design review: Design Changes Requested, two Must Fix findings
  ([review](../../evidence/SIM-2D/design-review.md)). The design, plan and done-when file were changed to answer it.
- 2026-11-13 MF1, performance gates removed. The 16.7 ms line no longer fails the cost command or stops a builder,
  and M2.4 no longer requires zero warmed allocations: brief decision 3 and the owner's decision of 6 November make
  such figures baselines. Times and allocation counts are still recorded, tripwires are flagged in the record and
  investigated while building goes on, and the index is still designed to reuse its storage. The plan's
  assumption that proposed the hard line is deleted, having no owner exception behind it.
- 2026-11-13 MF2, input conversion specified and proved. A rule gets an axis as the value the input wire's
  receiver decodes (A5's rounding rule), through one conversion in `thrum-core`. Planner's check: all 65,535 wire
  integers re-encode to themselves, so a local frame and its decoded copy convert to the same bits. New item
  M1.12 compares the conversion with `thrum-network`'s real encode and decode in `./dev check`, and the
  conformance run gains a wire route on every target (M1.1–M1.3). The codec is not edited; the test lives in the
  crowd's runner package, `crowd-2d-run`.
- 2026-11-13 Suggestions, both taken. S1: the fold is defined over the corrected timeline, one digest per tick as
  last committed, and the run counts rewinds whose replay changed a digest. S2: "any finite radius" is replaced by
  a query domain of 2²⁴ units with a cell size of at least 2⁻⁴, refusal outside it, and query work bounded by the
  body count (new item M2.12). Both were edges a milestone reviewer would otherwise have tried.
- 2026-11-13 The plan's ticked step lists became short notes, as the plan-module skill now asks. The changed and
  new checks parse and fail, as they should before the work exists. The revised design has not been reviewed.
- 2026-11-14 Owner: "Let's move the 2d engine side of things to before v1 and decouple them. But we don't want to start working on the swarm game till v1 lands." M1 no longer waits for `SHOWCASE:M8` and waits for
  nothing. The assumption that held it is deleted. No item and no word of the design changed, so its approval
  stands.

## M1 verification

M1.11 passed on `b7fd7d5301bf3ea3bffed9084cc0d508acb12ed6` on this Mac and in Chrome.
The verifier passed 38 native M1 comparisons, four native M2 comparisons and all 481 records
in each of Chrome's day SDR, day scene-linear, night SDR and night scene-linear sets.
Every build, cook and comparison command exited zero. The run took 922.214 seconds.
The covered-content key was `9e786b91d631ad087e813df1b346739747da9a26e0f7cadc558145a5d2bbce2b`.
The native binary SHA-256 was `25b90d1de2473c2dab2af2626b02c93fca3adca7d04eb9a5722a1b98e13c1e85`,
the Wasm SHA-256 was `7d3419e2de3271340331d1342f5a8b9a17f10f295bfff19227acc58f0ad8ad68`,
and the fresh cooked root was `70d2baf823d09d85574f7cdfce7102648754544dd7d68659e9bd92dd5f30e21d`.
The source checkout was clean, using Rust 1.99.1 and macOS 26.6.2 (25G83).

Item-check job `20261115-002057-18054` exited zero with all 11 commands passed and zero
failures, errors or timeouts. The continuation job `20261115-014512-10680` is queued under
the GPU lock to run the full milestone check and then merge. Its result is pending.

The continuation holds the GPU lock while it checks and merges, and relies on cached item
checks for the unchanged files outside `docs/plans/`. Keep those files unchanged until it
exits: invalidating the M1.11 cache would make its nested check wait for its own GPU lock.
Changes to this log and to the done-when results preserve that cache. Observe the continuation's
merge result and affected upstream files before reporting completion.
