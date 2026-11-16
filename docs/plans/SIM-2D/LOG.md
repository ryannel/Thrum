# SIM-2D log

- 2026-11-15 M1 integration resumed with all twelve retained builder results passing.
  Stopped the inherited, idle two-hour custom merge job without source changes.
  M1.11 now cites the shared main-branch exact-images check required by current policy;
  the earlier exact receipt remains historical evidence. The supported merge command
  runs the integration gate; the lead checks the merged work before review.

- 2026-11-15 M1 resumed qualification at `371450716`: fresh Mac, Chrome and simulator
  conformance, the sine control and all six focused test commands passed after the
  engine-owned publication API adaptation. Refreshed both two-repeat cost records.
  The first Mac 5,000-body row regressed 44.7%; the second regressed 5.6%, with
  concurrent target runs and recorded load supporting contention as a likely cause.
  [Evidence](../../evidence/SIM-2D/M1.md) retains every repeat and the unchanged digests.

- 2026-11-15 M1 clean-checkout recovery: the reported gate failure came from
  APPLE-PRESENTATION M3 links to evidence absent from the merged checkout. Main's
  `b247eda7b` repair identifies those builder-branch paths explicitly. All twelve
  M1 builder results are pass. Integration refreshes exact-baseline evidence after
  rebase and runs the repository gate on the combined tree.
  At `d1837acac`, documentation, Mac/Chrome/simulator conformance, the sine control
  and both cost-source checks pass. All 28 repaired-log links resolve to tracked files.

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
failures, errors or timeouts. Full milestone job `20261115-014512-10680` also passed
`./dev check`, including the V1 simulation tests, before its merge encountered conflicts.

The canonical rebase conflicted only in duplicate historical handoff-link repairs. Kept
the upstream versions of CREATOR's and CULLING's M3 handoffs and the CLOUDS log, preserving
their newer entries. The rebased source at `37134ad8` contains six upstream cloud-model
source/test changes. Both retained simulation cost records still pass their measured-source
identity checks. The simulation sources are byte-identical to the qualified M1 source.

The selected [exact-baseline receipt](../../evidence/SIM-2D/M1/exact-baselines.json) records
each proof's actual revision, covered-content key, comparisons and build identities.
Canonical integration requires the verifier on rebased covered content and `./dev check`
on the combined tree. The proof-and-merge runner refreshes that receipt before merging.

The water-model workspace addition required another manifest rebase. The workspace list
keeps the upstream water-model package and all three SIM-2D packages. Refreshed both full-trace
cost records at `bf07e76e`, with two release runs per target and recorded shared-host load.
Both records use source key `e58efdcdc45f183bcc2f6455d04dddc9d6e7d56ffd62c5aef05c3031f8ab835e`.
The Mac and Chrome artifact hashes equal their retained conformance receipts. The rebuilt
simulator binary also matches, at SHA-256 `fd393785c4318ee309ac5f0f5ed0e8dc28fef6570e8a54d54a6db2bc959c3bd1`.
The 5,000-body neighbour-scaling flags remain explained by the grid table growing fourfold;
measured index growth is 2.8–2.9 times. Clock and chronological quarter-regression flags are false.

## M1 review fixes, 16 November 2026

Expanded retained profile vectors cover all wire values and rounding boundaries on all three targets.
Preserved the previous cost record across both repeats; a regression test covers the review's slower row.
Mac, Chrome and simulator conformance reproduce the unchanged folds and expanded vectors; the sine control diverges.
Two release cost runs per target at `6ce949c39` pass source and completeness checks, with named prior comparators.
Clock and quarter-regression flags are false; the 5,000-body scaling flags remain explained by grid growth.
The six focused Rust checks pass. [Evidence and measured limits](../../evidence/SIM-2D/M1.md).

## M1 cost identity after merge, 16 November 2026

An unlinked rain-model addition changed the whole-workspace hash and invalidated both cost records.
The key now follows the runner's normal/build dependencies on all targets and their build settings.
Five focused tests and the historical merge regression pass; runtime changes still invalidate the key.
Fresh two-run release records at `42fe7d0fd` pass both cost validators and retain identical artifact hashes.
Clock and quarter-regression flags are false. The evidence records the scaling flags and replay outlier.
[Evidence, regression receipt and measured limits](../../evidence/SIM-2D/M1.md).

## M1 package feature graph, 16 November 2026

Workspace-wide metadata and lockfile dependency unions still included transport's unused futures features.
The key now hashes the package-scoped Cargo tree's active features/edges and selected package checksums.
Records retain input manifests. Seven tests and the actual clean-checkout comparison pass.
Both historical workspace/lockfile changes preserve the key; runtime source changes invalidate it.
Fresh two-run records at `8d59950fc` pass both cost validators, with unchanged native/Wasm hashes.
[Evidence and current measurements](../../evidence/SIM-2D/M1.md) retain the known scaling flags.
