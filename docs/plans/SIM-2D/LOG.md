# SIM-2D log

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
