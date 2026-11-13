# SIM-2D design review

Reviewed 13 November 2026 at `081d74e86a6663988955fa4ce7bf71a305ace6b5`.
Result: **Design Changes Requested — 2 Must Fix findings.**

The design supplies the simulation part of the requested 2D multiplayer swarm game. Its scope, separation from
rendering and game policy, restricted-float approach, reference crowd and early three-target proof fit that job.
It does not yet satisfy the owner's instructions in full: it reinstates performance admission gates, and its
acceptance criteria omit the local-input/wire-input equivalence on which the deterministic profile relies.

## Must Fix findings

### MF1 — Numeric performance and allocation gates contradict the owner's decision

**Verified by document comparison.** Affects M1.8, M1.9 and M2.4;
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), lines 318–319;
[PLAN.md](../../plans/SIM-2D/PLAN.md), the final assumption and line 59;
[DONE.json](../../plans/SIM-2D/DONE.json).

M1.8 and M1.9 require the median 10,000-body tick to be below 16.7 ms. The cost command fails otherwise, and the
plan explicitly tells the builder to stop `BLOCKED`. M2.4 separately makes zero warmed allocations an acceptance
condition. Calling the time limit a tripwire does not change that these conditions hold up the build.

[Brief decision 3](../../plans/BRIEF.md#decisions) says numeric performance and allocation figures are baselines,
not gates. The [owner's 6 November decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md), lines
8–17, says to investigate without blocking other building and explicitly overrides numeric performance/allocation
admission gates. The [performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design)
also distinguishes designing storage for reuse from making allocation targets an admission condition.

The planner has identified a real concern: a simulation slower than its selected cadence cannot sustain that
cadence. It should be measured and investigated. It does not authorize overriding the owner's chosen development
sequence. The proposed hard line is listed as an assumption, but ACCEPTED.json contains only the lead's task,
“Plan module SIM-2D. It needs: SHOWCASE”; it records no owner exception.

**Resolution:** keep the cost and allocation measurements, density accounting, storage-reuse design and regression
investigation. Remove their numeric pass/fail and stop conditions from the design, plan and done-when items,
consistent with the existing owner policy. Correctness and the required target executions remain gates.

### MF2 — No acceptance item proves local inputs equal their wire-decoded copies

**Verified acceptance gap, with a numerical witness; not a claim about unbuilt implementation.** Affects the
deterministic-rules interface and M1's proofs;
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), lines 134–136;
[DONE.json](../../plans/SIM-2D/DONE.json), M1.1–M1.7 and M3.1–M3.4.

Mechanism 2 promises one engine conversion that makes a locally constructed input frame and its host-relayed
copy yield identical profile numbers. None of the items exercises those two routes or asks whether the rule
actually uses that conversion. M1.1–M1.3 replay the same trace on different targets. M1.5 covers the profile
number's operations, not this input conversion. M3's lint fixtures do not detect a missing conversion performed
inside the allowed engine API.

A convincing incomplete implementation can wrap the raw local axis as a profile number, use the same raw-axis
trace on every target, retain its digests and pass the cross-target comparisons. It still disagrees with a peer
that receives the quantized input. This is an omitted interface behaviour, not merely another floating-point
edge case.

The existing boundary makes the difference concrete:

- [InputFrame](../../../crates/thrum-core/src/sim/input.rs), lines 5–45, admits raw finite normalized `f32` axes.
- [The input-wire decision](../../design/decisions/A5-INPUT-WIRE.md), line 11, requires signed 16-bit
  quantization with ties away from zero and canonicalizes negative zero.
- [The codec](../../../crates/thrum-network/src/input_batch.rs), lines 196–221, implements this in `quantize`,
  `dequantize` and `canonical_axes`.

I independently evaluated that specified arithmetic using Python `struct.pack/unpack('<f', ...)` for binary32
rounding, binary64 multiplication by 32767, and ties-away integer rounding:

| Local axis | Local binary32 value | Wire integer | Decoded binary32 value |
|---|---:|---:|---:|
| `0.1` | 0.10000000149011612 | 3277 | 0.10000915825366974 |
| `0.5` | 0.5 | 16384 | 0.5000152587890625 |
| `-0.5` | -0.5 | -16384 | -0.5000152587890625 |

Negative zero also changes from little-endian bytes `00000080` to `00000000`. These are normal admitted inputs,
and the digest deliberately observes stored profile-number bits.

**Resolution:** include an M1 proof that the profile's actual rule-input route produces the same values and
committed state from a local frame and its real codec round trip, including inputs that quantization changes.
State how the conversion stays aligned with the existing wire authority while rule packages retain their stated
dependency boundary. This can use the existing codec without sockets or a built SESSION module. Leave the test's
implementation route to the builder.

## Suggestions

### S1 — Name which history the retained fold describes

**Verified ambiguity, not a demonstrated inability to implement replay.**
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), lines 229–231 and 342–344, and M1.1–M1.3 compare a fold of every
tick's digest with a straight run that had corrected inputs from the start. A rewinding run also visits the
superseded, uncorrected endpoints. Those endpoints should differ when the correction changes gameplay; an
append-only fold over execution order would therefore differ too.

Clarify that the comparison describes the final corrected timeline, and how superseded endpoints are excluded
or replaced. Preserve a meaningful correction in the trace. This is a clarification of an otherwise sound
straight-run-versus-replay oracle, not a request for another replay subsystem.

### S2 — Bound query arithmetic and work as clearly as position precision

**Verified missing detail; failure of a future implementation is only suspected.**
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), N4 and mechanism 5, promises radii up to any finite size and queries
over hashed cells without a declared arena. The proof examples reach several cells and positions at 16,384 units.
Finite binary32 values alone do not ensure finite squared distances, valid cell-coordinate conversion, or a
practical cell walk for a query much larger than the occupied region.

State the supported radius/cell-coordinate domain and large-query behaviour, or explain how the implementation
keeps the broader promise. A bounded scan of occupied data is one possible treatment; the design need not commit
to that implementation. This does not block the ordinary crowd range already commissioned.

## Request and scope assessment

I read ACCEPTED.json and the project brief before the design. There is one recorded planner task, quoted above,
and no prior design review. The human's request is the original swarm survivor, 2D engine support and several
players over the network in [the next-goal page](../../design/NEXT-GOAL-SWARM-GAME.md). Thousands of bodies,
2–4 players and the target mix are explicitly defaults. I also read the original SIM-2D module brief preserved at
`7a3694b8:docs/plans/ARCHITECTURE.md`, then the current module responsibilities and consumer interfaces.

For this module that means deterministic simulation with spawn/despawn and replay, complete peer-comparison
state, reusable neighbour/hit queries, enforceable rule restrictions, and a crowd later modules can consume.
It does not mean building sockets, sprites, UI or the finished game here. Those deferrals fit the module map.
The circle-body, finite-world and same-build assumptions are visible in the plan rather than silently narrowing
the human's request. The new query crate and game-owned reference crowd preserve the reusable-engine boundary.

The restricted-float decision is credible for the named scope. I checked the cited
[Rust float semantics](https://rust-lang.github.io/rfcs/3514-float-semantics.html) and
[primitive documentation](https://doc.rust-lang.org/std/primitive.f32.html): the selected arithmetic guarantee,
NaN-bit caveat and nondeterministic signed-zero choice in primitive minimum/maximum support the wrapper's
restrictions. This supports attempting M1; it does not prove the engine deterministic. The planner's discarded
stand-in probe is reported evidence only, and M1 correctly requires the registered runtime on all three targets.

The centre-cell grid, oversize list, deterministic query order, brute-force candidate oracle and separate
predicate examples form a plausible mechanism. I also checked the cited
[uniform-grid source](https://developer.download.nvidia.com/assets/cuda/files/particles.pdf). Nothing in this
review qualifies integrated query cost or runtime correctness.

## Done-when review

Every item was read, including its check and reviewer question. The milestone sequence is feasible: M1 builds
the profile, digest, basic queries and target runners; M2 extends queries and proves parallel use; M3 enforces
package restrictions and documents the consumer route. None requires later rendering or transport work.

| Items | Assessment |
|---|---|
| M1.1–M1.4 | Real target executions, runtime/structural/history inspection and a divergent control reject a private simulation stand-in or same-process comparison. Add the omitted input-boundary proof in MF2; clarify the fold as S1. |
| M1.5–M1.7 | Operation inspection, runtime refusal and per-kind authority-state perturbations are judgeable in M1 and materially stronger than a positions-only hash. |
| M1.8–M1.9 | Cost and replay records are useful to SESSION; numeric admission clauses conflict with owner policy (MF1). |
| M1.10–M1.11 | The retained slice, unchanged V1 tests and registered exact-baseline verifier are judgeable regression protections. |
| M2.1–M2.3 | Whole-sequence comparison, independent expected geometry and an endpoint-miss/swept-hit case distinguish real queries from merely equal peer mistakes. S2 concerns the broader advertised domain. |
| M2.4 | Reuse is a sound design objective; zero allocations as an admission condition conflicts with owner policy (MF1). |
| M2.5–M2.8 | Preserved Crowd digests, exercised Mixed cases and positive forced-parallel dispatch counts can be judged using M1 and M2 alone. |
| M2.9–M2.11 | Comparable density-aware costs and the baseline verifier are judgeable; regression explanations do not impose the M1 hard threshold. |
| M3.1–M3.2 | One-violation controls and inspection of registry discovery and check integration can prove the explicitly enumerated guard classes. They do not prove arbitrary rule code deterministic, and the design appropriately supplies a separate conformance route. |
| M3.3–M3.4 | The second runner consumer and executable guide can be inspected before SWARM exists. A fresh guide-only consumer would sharpen the usability claim, but is not needed to unblock this design. |
| M3.5–M3.9 | Closing target reruns, separated maturity claims and exact-baseline verification are judgeable without later modules. |

## Review limits and validation

This is an independent design review, not a milestone verdict. I read the full SIM-2D design, plan and done-when
file, the accepted task, relevant brief and architecture sections, contracts C01–C03/C13, G2 and input-wire
decisions, performance policy, review calibration and planning-probe report. Scoped code reads covered the existing
input/codec boundary, runtime authority hash, registered-world metadata capture and history interface; I did not
review the entire engine. No work was delegated.

I did not run the unbuilt conformance commands, recreate the planner's probe, inspect visual output or measure
device speed. No visual claim is made by this module. Only this review and its evidence folder are changed;
DONE.json and all planner files are untouched.

Validation: `CARGO_TARGET_DIR=/private/tmp/thrum-sim2d-design-review-target ./dev check-docs` passed:
756 Markdown files, 4,719 local links, 30 module records, crate policy and current design hashes. It emitted
existing large-file responsibility notes, with no errors. `git diff --check` was clean, and `git status --short`
showed only the new review file. The review is left uncommitted for the script.

DESIGN CHANGES: 2 must_fix findings
