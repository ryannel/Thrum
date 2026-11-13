# EFFECT-GRAPH design review

Reviewed 11 November 2026 at integrated commit `ae64c2bd9801c3d8699f4c53d00151b5b0620bbf`.
Result: **design approved — no Must Fix findings; one wording suggestion**.

This is the second design-review round, against the review at `c9dd6cd0d58617e5b269054e9b9b5a564095ced9`.
I read both tasks in [ACCEPTED.json](../../plans/EFFECT-GRAPH/RECORD.json) under `briefs`, whole and in order,
the project brief, the module-list scope and interfaces, the previous review, the complete current design and
plan, and all 21 done-when items. I compared the design, plan and criteria with the previous reviewed revision.
The commission remains a small resource milestone after APPLE-TILE M2, followed by the graph foundation for
sea and clouds, with protection against changing another module's exact baselines. It does not commission the
sea/cloud implementations or require this foundation to qualify their reconstructed appearance.

## Previous Must Fix findings

### MF1 — Repeated ticks reset persistent simulation — resolved

**Verified by design and proof inspection.** [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md), lines 151–179,
now separates equal, forward and backward ticks, changed timelines and the first frame of an epoch. Equal ticks
retain committed state exactly, including pause; backward ticks and changed timelines reset. Forward progress
has an explicit bound and dropped-tick count. Submission commits state; discard does not.

M2.2 now requires a nonempty field before repeated ticks and pause, unchanged contents afterward, and declared
reset contents after both discontinuities. Its Reviewer Question requires expectations derived independently
from the state table, so CPU/GPU agreement on the original wrong rule cannot satisfy it. M2's bar names retention
explicitly, and M2.5 carries the same expectations to Chrome. The original ordinary 120 Hz/60 Hz contradiction
is removed.

### MF2 — Cadence does not define which history result readers receive — resolved

**Verified by mechanism walkthrough and proof inspection.** [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md),
lines 181–243, now defines a committed side per pair and distinct `previous`, `next` and `result` bindings.
A downstream consumer reads `next` when its producer ran and the committed side when it skipped. Only pairs
whose producers ran flip on submission. Discard flips nothing; independently scheduled pairs retain independent
sides. Bindings for both sides are prepared before frame recording.

Walking the original counterexample through these rules: after A/step 4 advances to B/step 5 and submits, a
skipped update leaves consumers on B and commits no flip. A discarded update to A cannot publish step 6; a later
retry starts from B. The exclusive mutable host borrow in `runtime/tokens.rs` supports the claimed single pending
encoded-frame assumption, and `runtime.rs` invokes the contribution's submission callback after queue submission.
The existing graph's history scope alone does not implement this feature; the revised plan explicitly builds
the additional per-pair binding and publication protocol.

Time-sliced writes are confined to in-place persistent resources. First-epoch and reset frames cover the entire
output from seed/reset state, irrespective of cadence; a discarded full frame remains unseeded for the retry.
M2.3 requires two differently scheduled histories, readback of their consumer's output at run/skip boundaries
and after discard, full-output readback at initialization/reset, compiler refusals, and creation counters.
An always-running history plus a skipped in-place texture no longer satisfies the witness. S4 below concerns
phrasing of the item, not a missing mechanism.

### MF3 — No refraction request incorrectly requires no stored depth — resolved

**Verified against both owners.** [DESIGN.md](../../plans/EFFECT-GRAPH/DESIGN.md), “After opaque,” and M3.1 now
separate depth demand from the refraction copy. Depth storage follows the union of declared readers; no effect
adds nothing beyond other readers' needs. This agrees with C06 and APPLE-PRESENTATION M2's D2.1.

M3.1 asks for four compiled cases: no effect with reconstruction disabled, no effect with its depth input enabled,
a depth-only effect, and a refracting effect. Its moving-occluder witness still tests this frame's depth.
Removing refraction can no longer satisfy the item by wrongly removing another reader's stored depth.

### MF4 — M3 cannot prove its reconstruction quality bar — resolved

**Verified against prerequisites and consumer ownership.** M3's name, bar and M3.2 now qualify composition and
reconstruction inputs. They do not claim that a reconstructed effect is free of shimmer or ghosting.
APPLE-TILE M3 supplies the pre-tone seam and named timing schema; APPLE-PRESENTATION M2 supplies the temporal
inputs. Those are the stated prerequisites. APPLE-PRESENTATION M3's TAAU is no longer needed to judge this bar.

The design and [module list](../../plans/ARCHITECTURE.md) assign reconstructed effect appearance
to SEA/CLOUDS on the reconstructor. The remaining fixed stills judge crossings, refraction and sky/star
composition; submitted motion/reactive/reset readbacks judge the input seam. This is the explicit scope option
the previous review proposed, and it preserves the commissioned foundation.

## Previous Suggestions

- **S1 — Make the portable resource proof explicit — resolved.** M2.5 now names cube, every volume texel and
  every array mip in the Chrome readback, through loss and recovery.
- **S2 — Settle when named timing becomes available — resolved.** M2.7 and the design use the executor's existing
  named `PassTiming` values through a test or bench. They explicitly leave the host's frame-sample schema to
  APPLE-TILE M3. The latter is a prerequisite of EFFECT-GRAPH M3's shared timing proof.
- **S3 — Do not imply that list order establishes medium validity — resolved.** The after-opaque section now
  distinguishes execution order from C08's admission and transport rules. SEA/CLOUDS and V3 retain the obligation
  to provide disjoint segments or an explicit joint/approximate composition policy.

## New findings

### S4 — Distinguish tentative results from committed results in M2.3

**Suggestion; verified wording ambiguity.** Location: [DONE.json](../../plans/EFFECT-GRAPH/DONE.json), M2.3,
line 114. The sentence calls every consumer value the “newest submitted result,” then correctly includes a
producer's tentative output. Its “committed side again after a frame whose update was discarded” also leaves
implicit whether the next frame skips or retries the producer.

The design's four-frame example settles the mechanism: discard preserves B, but when frame 4 retries the update,
its downstream consumer reads the freshly written A. Clarify that a post-discard skip reads committed B, while
a post-discard retry computes from B and exposes tentative A. Use “result for this frame” for the encompassing
term. This is a wording refinement: the explicit binding rules and existing run/skip/discard witness already
make the intended proof judgeable, so it does not hold up the build.

## Changed criteria and prerequisite assessment

“Judgeable” means the planned proof can be assessed at its milestone, not that implementation has passed.

| Changed definition | Assessment against the previous review |
|---|---|
| M1.5 | Judgeable. The native matrix and release-build command are unchanged. The claim now accurately names Mac-only coverage. The temporary Chrome deferral is explicit in the design, plan assumptions and module list. |
| M2 bar / needs | Judgeable. Retention on repeated ticks and pause resolves MF1. The new BASELINE-GUARD M1 prerequisite supplies the verifier needed by M2.8. |
| M2.2 | Judgeable. Nonempty retention, independent expectations and explicit backward/timeline resets address MF1; discard and anchor checks support MF2. |
| M2.3 | Judgeable under the design's binding rules; S4 sharpens wording. Two histories, consumer readback, initialization/reset coverage and refusal checks address MF2. |
| M2.5 | Judgeable. Same state expectations plus explicit portable resource readbacks resolve the browser part of MF1/MF2 and S1. |
| M2.7 | Judgeable with existing executor timings; S2 is resolved without depending on the later host-schema migration. |
| M2.8 | Judgeable after BASELINE-GUARD M1. The new command covers registered Mac and Chrome baselines, rather than only the native matrix. |
| M3 bar / needs | Judgeable. The bar now matches the supplied inputs and composition witnesses; MF4 is resolved. The APPLE-TILE M3 and APPLE-PRESENTATION M2 prerequisites are unchanged. |
| M3.1 | Judgeable. Four demand cases and the current-frame occluder witness resolve MF3. |
| M3.2 | Judgeable as submitted motion/reactive/reset input proof, explicitly excluding reconstructed output; MF4 is resolved. |
| M3.8 | Judgeable. The verifier is available through the preceding M2's prerequisite. It retains the Mac/Chrome baseline requirement. |

The unchanged items remain judgeable in this sequence: M1.1–M1.4 cover resource allocation/readback, refusals,
capabilities and unchanged compiled graphs; M2.1 supplies the external consumer beside the cooked contribution;
M2.4 tests generation replacement; M2.6 inspects elapsed-time trail persistence and wrapping. M3.3 proves only
declared async eligibility, M3.4 proves knobs and skipped-pass timing after its provider, M3.5–M3.6 inspect
composition on the named targets, and M3.7 records public documentation, paired cost and separate maturity facts.
The revised bar no longer requires these still inspections to establish temporal reconstruction quality.

## Scope, boundaries and evidence

M1 remains small enough to begin after APPLE-TILE M2. The extension uses the existing contribution seam and one
executor, without a new rendering framework or private submission path. Its multi-tick update rule is explicitly
bounded; a future consumer requiring globally synchronized substeps must reopen the design as its risk section
says. Preserving same-generation effect state is distinguished from device replacement and descriptor changes.
No new performance threshold has been made an acceptance gate.

The exact-baseline change strengthens later milestones: BASELINE-GUARD M1's planned verifier builds fresh native
and Wasm outputs and compares all registered native and Chrome references. M2 now waits for it, and M3 follows
M2. M1 retains the disclosed interim limitation: it can land before that gate, with native matrices rerun before
landing and Chrome first rechecked by the guard's builder. This review does not certify that the future gate
exists or that M1 proves Chrome baseline preservation. That limitation was already disclosed in the previous
review and is now explicitly listed as an assumption for the owner.

Boundary inspection covered C02/C04/C06/C07/C08/C12; `contribution.rs`; the history scope and history-pair test in
`graph.rs`; `runtime/tokens.rs`; ordinary submission and endpoint-capture paths in `runtime.rs`; the contribution
frame in `synthetic/contributed.rs`; `PassTiming` and host timing collection; and the relevant APPLE-TILE,
APPLE-PRESENTATION and BASELINE-GUARD criteria. This is a design review informed by existing code, not a blind
technique study. I used the project review calibration and the skill's starting graded examples; no plans glossary
was present. No other agents were launched.

No implementation, visual quality or device performance result is claimed. I did not run the proposed GPU/browser
milestone checks or make captures. Only this review file was changed; DONE.json and all other project records
remain untouched.

Validation: `./dev check-docs` passed (699 Markdown files, 3,651 local links, 30 module records, crate policy
and current design hashes). `git diff --check` passed. The sole changed path is this review.

DESIGN OK
