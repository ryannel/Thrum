# SIM-2D design review

Reviewed 13 November 2026 at `aed1cd02c2333f4fa1ad039a448074b64dca7306`, against the previous review at
`081d74e86a6663988955fa4ce7bf71a305ace6b5`.

Result: **Design Approved — no Must Fix findings.** The revised design meets this module's part of the human's
request. Both previous Must Fix findings are resolved. Both previous Suggestions were taken and are resolved.
This approves the design and its acceptance criteria; it does not certify an implemented simulation.

## Previous Must Fix findings

### MF1 — Numeric performance and allocation gates contradict the owner's decision — resolved

**Verified by comparing the revised design, plan and items with the owner's decision.**
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), lines 387–399 and 431–432, now makes all recorded times and allocation
counts baselines. Cost-command success depends on a complete record at the claimed commit, not a threshold.
M1.8 and M1.9 say the same. M2.4 retains storage reuse and measured allocation counts with attribution, but
removes the zero-allocation admission test. [PLAN.md](../../plans/SIM-2D/PLAN.md), line 54, explicitly keeps
building when a measurement trips a tripwire; the unsupported hard-line assumption is gone.

This now follows [brief decision 3](../../plans/BRIEF.md#decisions) and the
[6 November owner decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md). The 16.7 ms period
remains useful information for SESSION and SWARM, without deciding admission. Correctness, target execution,
record completeness and investigation remain required. M2.12's rejection of an enormous empty-cell walk is an
algorithmic requirement, not a reintroduced tick-time or allocation budget.

### MF2 — No acceptance item proves local inputs equal their wire-decoded copies — resolved

**Verified against the existing input and codec boundary and the revised proofs.**
[DESIGN.md](../../plans/SIM-2D/DESIGN.md), lines 137–166, now specifies A5's conversion precisely, stores held
controls converted, and explains the dependency boundary. The rule package and core do not depend on networking.
The separate `crowd-2d-run` package can depend on both core and the existing codec; its test keeps the two
implementations of the wire rule aligned. The codec and its owning decision remain unchanged.

M1.12 requires the real public whole-batch encode/decode route, including all wire values, rounding boundaries,
endpoints and negative zero. It also requires a runtime trace slice whose changed axes actually steer players,
with local and decoded inputs reproducing retained committed-state digests. M1.1, M1.2 and M1.3 add the wire
route to the 2,000-body run on each target, alongside the full local-route runs. M1.5 closes the direct raw-float
constructor route. Together these reject the earlier plausible substitute: feeding the same unquantized trace
to every target and calling that input-wire equivalence.

I read [InputFrame](../../../crates/thrum-core/src/sim/input.rs), including axis admission, and
[input_batch.rs](../../../crates/thrum-network/src/input_batch.rs), including the public batch interfaces,
`quantize`, `dequantize` and `canonical_axes`. The specified arithmetic matches that existing implementation.
An independent [arithmetic witness](design-review/input_arithmetic.py) verified idempotence for all 65,535
admitted wire integers and adjacent binary32 inputs on both sides of all 65,534 rounding boundaries, plus the
named sample axes and signed zero. [Output](design-review/input-arithmetic.txt).

That witness checks the mathematical premise only. It neither executes the Rust codec nor replaces M1.12's
real codec and runtime proof. The conformance trace also remains a check of exercised behaviour, not a static
proof of every possible branch in an arbitrary game rule.

## Previous Suggestions

- **S1 — Name which history the retained fold describes — resolved.** Design lines 280–285 define one digest
  per tick on the final corrected timeline, replacing superseded digests before folding in tick order. M1.1
  asks the reviewer to inspect this and requires a positive count of corrections that changed a digest. Design
  lines 306–309 require corrections that steer a player differently. The straight-run oracle therefore compares
  the same timeline and cannot be satisfied by corrections that do nothing.
- **S2 — Bound query arithmetic and work as clearly as position precision — resolved.** Design lines 236–248
  replace the unlimited finite-radius claim with an explicit coordinate/radius domain, a minimum cell size,
  named refusal and a body-count-dependent fallback for wide queries. The plan discloses the finite-world
  assumption. M2.12 checks edge acceptance, out-of-domain refusal and the same ordered brute-force result for
  a query too wide to walk its empty cells. The stated domain keeps distance squares and cell coordinates
  representable; the fallback makes area-independent work feasible without changing result order.

No new Must Fix findings or Suggestions.

## Request and scope

I read both planner tasks in [ACCEPTED.json](../../plans/SIM-2D/ACCEPTED.json), oldest first, and the project
brief before comparing the design. The first task commissions SIM-2D after SHOWCASE; the second asks for the
two previous Must Fix findings to be resolved. Neither changes the human's request or overrides the performance
policy. The human's [swarm-game request](../../design/NEXT-GOAL-SWARM-GAME.md) and the module list require
reusable 2D support and a crowd that later multiplayer can share by causes.

The design supplies that simulation foundation: deterministic rules, complete committed-state comparison,
spawn/despawn and corrected replay, ordered neighbour and hit queries, enforceable named rule restrictions,
and a reusable reference crowd. Rendering, transport, session repair and the finished game's policy remain
with their named consumers. Circle bodies, the 10,000-body demonstrated range, finite coordinates and same-build
replay are explicit assumptions. The revised domain does not exclude a commissioned ordinary swarm-game case.

The restricted-float mechanism remains credible for the stated profile. Rust's
[float semantics](https://rust-lang.github.io/rfcs/3514-float-semantics.html) and
[primitive documentation](https://doc.rust-lang.org/std/primitive.f32.html) support its distinction between
specified primitive arithmetic and observable NaN-bit nondeterminism. The unchanged design refuses stored
non-finite values and keeps platform-dependent operations outside the API. Actual target agreement still has
to be earned in M1; the planner's stand-in probe is not engine qualification.

## Changed acceptance items

Every changed item was read in full, including its check and reviewer question, against the previous findings.
The remaining items and all three milestone quality bars were also read for interactions with the revision.

| Item | Assessment |
|---|---|
| M1.1 | Judgeable in M1. Preserves runtime, churn, three crowd sizes, long trace and debug proof; adds corrected-history inspection, meaningful corrections and the actual wire route. Resolves S1 and contributes to MF2. |
| M1.2 | Judgeable in M1 using the already-existing codec. Chrome must reproduce the retained values and run the wire route, not merely repeat a local raw-axis trace. |
| M1.3 | Same input proof in the simulator; appropriately proves function rather than another processor or device speed. |
| M1.5 | The allowed API and operation vectors are inspectable in M1. Excluding a general raw-float constructor supports the specified input route without depending on M3's lint work. |
| M1.8 | Requires real runtime costs and replay information from two attributed Mac runs, with tripwires recorded. No numeric acceptance condition remains. |
| M1.9 | Preserves the corresponding Chrome measurements and removes the same numeric gate. |
| M1.12 | A real codec comparison and committed-state trace test can be built in M1 without SESSION or sockets. Raw values changed by quantization must affect gameplay, so a helper-only comparison is insufficient. |
| M2.4 | Buffer reuse is reviewable and allocation sources must be measured. Replacing the automated zero-count assertion with inspection is appropriate for the owner's baseline policy. |
| M2.12 | Judgeable once M2's queries exist. The brute-force sequence and huge-area case test domain handling and bounded search work independently of target speed. |

The order remains feasible: M1 proves the profile, digest, basic circle/nearest queries and target runners;
M2 adds the remaining geometry and parallel use; M3 supplies the package guard and consumer guide. M2's existing
Crowd-baseline preservation, predicate cases and forced-parallel counts remain intact. M3's named guard classes,
second runner consumer and final target reruns are unchanged. No changed item requires a later module to exist.

## Review limits and validation

This review read the full current design, plan and done-when file, their changes since the previous review,
the previous findings, accepted tasks, project brief, relevant architecture sections, contracts C01–C03/C13,
input-wire decision, performance policy, review calibration and planner's probe/report. Scoped implementation
reads covered the input/codec boundary and its dependency direction; this was not a fresh audit of all runtime
or networking code. No work was delegated.

No conformance commands for the unbuilt module, visual inspections or device measurements were performed.
The numerical witness is explicitly separate from future implementation proof. Only this review and its
permitted evidence folder are changed; the done-when file and all planner fields are untouched. The review
is left uncommitted for the script.

Validation: `CARGO_TARGET_DIR=/private/tmp/thrum-sim2d-design-review-target ./dev check-docs` passed:
756 Markdown files, 4,725 local links, 30 module records, crate policy and current design hashes. It emitted
existing large-file responsibility notes, with no errors. `git diff --check` passed, and `git status --short`
showed only this review and its evidence folder.

DESIGN OK
