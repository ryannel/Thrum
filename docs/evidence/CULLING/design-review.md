# CULLING design review

11 November 2026. Reviewed integrated commit `42ec5975715eeea889522d54f7521f2ee2d43fd1` against the previous
review at `d90f66ac1638407b9821fee3868c060cd361bdca`.

Result: **Design Approved, with one Suggestion.** The previous Must Fix is resolved. This assesses the design
and whether its milestones can prove their promises; it does not qualify implementation, images or performance.

## Previous findings

### MF1 — Resolved: the live-switch transaction relies on a replacement interface that cannot deliver its stated behaviour

The revision removes the live switch from the commissioned scope, cue C4, M2's quality bar and its switch items.
[DESIGN.md](../../plans/CULLING/DESIGN.md), lines 69–80 and 240–243, instead selects the mode at scene preparation
and retains the requested mode for resolution on a replacement device. [PLAN.md](../../plans/CULLING/PLAN.md),
line 16, explicitly lists this scope decision for the owner.

This resolves the finding by removing an unrequested feature, not by claiming the publication interface was
fixed. I read both complete planner tasks in [ACCEPTED.json](../../plans/CULLING/ACCEPTED.json), oldest first:
the original commission asks for G3c, a small first milestone after APPLE-TILE M2, and pre-landing baseline
protection; the second asks for the design findings to be addressed or a reason recorded. Neither requests a
live culling-mode control. The project brief's V1 exit and G3c also do not require it. C04 distinguishes load-time
selection from live changes and does not promise arbitrary live renderer replacement.

The replacement scope has a real path: native launch selection enters `GameSetup` in
`examples/v1-game/src/content.rs`, and `create_field_game` in `examples/v1-game/src/browser.rs` accepts the mode
before preparation. Extending browser scene setup for mixed meshes and the valley remains M2/M3 work. Device
preparation in `crates/thrum-render/src/runtime/device.rs` carries the retained scene into preparation with the
new capability record. The builder must preserve the requested selection there when adding resolution.

I rechecked the old fault boundaries: `runtime.rs` still restricts visual publication to native targets and
calls `rebuild_contribution` after installing the replacement; `runtime/visual.rs` has no requested-mode input;
`thrum-asset-gpu` still creates contribution pipelines in `rebuild_inner`. The revised culling design no longer
depends on these providing a live switch. Its possible future prepared-variant approach is explicitly something
to cost if commissioned, not an approved or proven mechanism in this review.

### S1 — Addressed: separate tolerance and footprint witnesses

M1.3 retains the required mesh tie and failing zero-depth-tolerance mutant. It adds a separate footprint witness
and failing no-widening mutant when the reference search finds one; otherwise it requires the searched poses to
be recorded. The design now explicitly says the four-ulp rasterizer interpolation allowance is an assumption,
and that a passing witness does not establish it.

This is a useful strengthening without making an unsuccessful footprint search proof of necessity. The original
depth proof has not been weakened: a cube witness or a zero-depth-tolerance mutant that still passes cannot
satisfy M1.3. No exhaustive numerical study is required by the earlier suggestion.

## S2 — Suggestion, verified limitation: M1 can land before Chrome's retained baseline is checked

Affected: [DESIGN.md](../../plans/CULLING/DESIGN.md), lines 287–293;
[PLAN.md](../../plans/CULLING/PLAN.md), line 17; M1.5;
[ARCHITECTURE.md](../../plans/ARCHITECTURE.md#exact-baselines).

M1 retains the previous release-build/fresh-cook native matrix and now states its target coverage accurately.
It does not run the Chrome retained comparisons. If M1 lands before BASELINE-GUARD M1, a Chrome-only regression
can therefore reach main before the guard builder's first verifier run finds it. Intending to preserve default
output is not evidence that it was preserved. The manual native rerun before `land` also does not yet provide
the guard's protection against a covered-path change introduced by `land`'s own rebase.

Prefer having BASELINE-GUARD land before CULLING M1 lands, while allowing CULLING implementation to start
immediately. Keep the interim limitation explicit if that ordering is unavailable; do not report M1.5 as proof
of unchanged Chrome output or of an already enforced pre-push gate.

This is advisory, not a new blocker: the native check is unchanged from the reviewed first slice; the interim
ordering and Chrome omission are now explicit assumptions for the owner, rather than hidden narrowing; the
commission also asks that M1 be available immediately after APPLE-TILE M2. The design supplies the requested
permanent pre-landing mechanism, and M2 cannot start without it. Approval does not establish that the temporary
gap is covered or that the owner has separately confirmed the assumption.

## Revised items and milestone boundary

I read every item in the current [DONE.json](../../plans/CULLING/DONE.json), and its complete diff from the
previous review. These are design assessments, not stored milestone verdicts.

| Changed item or boundary | Assessment against the previous review |
|---|---|
| M1.3 | Judgeable. S1 is addressed as above. The test and reviewer question distinguish depth slack from footprint widening and retain a nonvacuous depth mutant. |
| M1.5 | Judgeable for the stated Mac scope. The release/fresh-cook command is unchanged. The added Chrome exclusion is honest and carries the S2 limitation. |
| M2 name and bar | Coherent with MF1's resolution. Device fallback, complete drawing and measured selection remain required; no unimplemented live-switch promise remains in the bar. |
| M2 `needs` | `BASELINE-GUARD:M1` supplies the new verifier before M2 starts. M3 inherits that prerequisite through the module's sequential milestone order; no duplicate dependency is needed. |
| M2.5 | The identifier now names the baseline check, not the failed switch requirement. The verifier covers both native matrices and four Chrome retained sets at the claimed content. Its prerequisite's M1.1–M1.4 require fresh builds, complete comparisons and refusal after a covered-content rebase, so this is a concrete dependency rather than a nonexistent command assumed to work. |
| M2.6 removed | The Chrome live-switch proof is intentionally removed with its unrequested feature. It does not remove Chrome mixed-mesh equality, which remains M2.3. |
| M2.7 removed | The old native-only baseline check is superseded by M2.5, whose verifier includes that matrix. This is not loss of the native baseline obligation. |
| M3.10 | Judgeable after BASELINE-GUARD M1 and CULLING M2. Replacing the native-only command with the full verifier strengthens coverage of the default-mode change. |

Although M2.5 and M3.10 carry `target: mac-metal`, their statements and command require Chrome too. A native-only
pass cannot satisfy either item. BASELINE-GUARD's design explicitly builds Wasm and compares all four Chrome
sets; it is not a Mac substitute for browser proof. The verifier remains future work at this reviewed tip.

The unchanged items remain judgeable with their predecessors built:

| Items | Retained proof and assessment |
|---|---|
| M1.1, M1.2, M1.4 | Per-frame bucket-ordered equality, actual occlusion, wrong-prepass/list mutants, layout churn, resize and device replacement still reject an unchanged Direct path or stale-history substitute. |
| M2.1–M2.4 | Capability-to-pipeline tracing, simulator complete-scene fallback, same-target Chrome equality and nonempty-prepass paired measurements still cover selection and defaults. The simulator remains function only. |
| M3.1–M3.6 | Native route equality, Chrome checkpoints, measured defaults, opt-in bucket/Sun-caster coverage and jitter-cycle equality remain required. Chrome checkpoint equality is not being promoted to every-frame motion proof. |
| M3.7, M3.8 | T2 fixed and realtime output remains specified on each target for silhouette, edge and motion inspection. No visual verdict is made here. |
| M3.9 | The closing audit still names every G3c clause, including stable mesh offsets and compute-readable materials, and keeps maturity dimensions separate. |

## Request, mechanism and scope

The retained three-step delivery covers G3c: mixed-mesh occlusion; capability resolution and measured field
defaults; then the valley's 10,000 entities and gate closure. No revision drops compute frustum/Hi-Z, GPU-written
instance counts, bucket draws, stable mesh offsets, compute-readable materials or full/partial-visibility cost
records. Existing explicit assumptions about SHOWCASE's final assembly and native-only optimization candidates
are unchanged.

The core mechanism is also unchanged: current transforms of a historical subset produce the prepass, every
active instance is conservatively tested, logical-slot history survives bucket churn, and the final draw keeps
the oracle's bucket/member order. Depth twins preserve each raster class. The revised witness requirements
test the numerical assumptions more precisely; they do not replace the conservative test with a weaker
visibility rule. New GPU and browser tests are deliverables of the named milestones, not missing prerequisites
merely because they are unbuilt today.

## Review scope and verification

Read both accepted briefs, the project brief, the previous review, all current CULLING design/plan/items and
their revision diff, then the module log. Read the relevant architecture ownership and baseline rules,
BASELINE-GUARD's design and items, C04/C06/C07/C12, project review calibration and the skill's graded examples.
Inspected the launch/browser selection, culling preparation, retained device preparation, visual publication
and cooked contribution rebuild boundaries described above. This was a revision review with existing code
visible, not a blind technique study. No other agents were launched.

No GPU run, browser run, benchmark or visual capture was needed for these design findings; none was performed.
The earlier runtime evidence is not asserted to qualify this tip. Only this review file changed; DONE.json and
the plans remain untouched. `./dev check-docs` passed (698 Markdown files, 3,647 local links, 30 module records,
crate policy and current design hashes). `git diff --check` passed.

DESIGN OK
