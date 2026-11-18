# SHOWCASE design review

**Changes requested.** The design preserves the V1 scene, its three target rows and the final visual review. Moving the M5 timing judgement to a quiet M8 sitting answers the lead's request. Four gaps in the proof plan need correction before builders and reviewers can apply it consistently.

Reviewed `e289340c01cb` on 18 November 2026 against all five planner tasks and the approved brief. The previous design review, of `555c80067c5b`, had no Must Fix findings. Its two Suggestions are accounted for below. The later M5 implementation review supplies the motion observations that prompted this revision. [Planner tasks](../../plans/SHOWCASE/RECORD.json) · [Project brief, decisions 1–4 and G7](../../plans/BRIEF.md) · [M5 review](M5-review.md)

## Findings

### MF1 · Must Fix · The named Mac recorder cannot supply M5.8's fixed sequence

M5.8 needs at least 20 seconds of consecutive world ticks in each of the flythrough and the held view. At the exit route's 60 Hz that takes 1,200 tick advances. The handoff points to `capture_v1_runtime.py --mode fixed`. Its Mac loop stops after 480 opportunities and can declare the fixed interval complete at four seconds. More importantly, `Offscreen::step_for` drives the world from elapsed host time. Increasing the opportunity count alone would still let a slow readback change the ticks submitted. A complete take from this path therefore cannot establish the coverage M5.8 requires, including the flythrough's `balloons` checkpoint at route tick 1,080. Verified by tracing the runner and its completion predicate. [M5 handoff in the plan](../../plans/SHOWCASE/PLAN.md) · [Native capture loop](../../../examples/v1-game/src/window/capture.rs) · [Offscreen clock consumption](../../../crates/thrum-app/src/offscreen.rs) · [Exit route](../../../examples/v1-game/routes/exit-flythrough.json)

There is a usable starting point. The separate native route recorder drives exactly one fresh tick per submission at a chosen phase and retains submitted pixels at named checkpoints. Identify how M5 will use or extend that path for both camera cases, retain the consecutive inspection windows, and establish the full world-time span from submission identities. For the known strip, use the reproduced tick **and phase**. The diagnostic records route ticks 202–203 as committed ticks 233–234, with alpha near one. The shipped route defaults to one half. [Native route recorder](../../../examples/v1-game/src/bin/window/route.rs) · [Fixed shadow diagnostic](M5-motion-diagnostics.json)

### MF2 · Must Fix · The specified M5 baseline refresh refuses the existing baseline

The revised plan tells the builder to inspect drift caused by the balloon shadow fix and replace the exit baseline with `--save-baseline`, keeping the thresholds. I ran `python3 scripts/run_v1_showcase.py check --exit --save-baseline` against the checked-in baseline. It exited 1 before rendering with `use --save-baseline exactly once to calibrate the exit check`. Removing the old file would select first-time calibration, which computes three thresholds from the new run. The existing thresholds are all 1.0. Neither branch implements the promised refresh. [M5 plan](../../plans/SHOWCASE/PLAN.md) · [Exit check and save guard](../../../scripts/run_v1_showcase.py) · [Checked-in exit baseline](M5-exit-baseline.json)

Provide a refresh path for the case where the reviewed shadow correction moves an exit stop. It should compare using the saved identity and thresholds, retain the three controls and record the affected grids and cause. This concerns only SHOWCASE's thresholded baseline. The registered exact comparisons have their own owner and verifier. [Baseline ownership](../../plans/ARCHITECTURE.md#exact-baselines)

### MF3 · Must Fix · M5.11 can accept a timing record from a different build

M5.11 requires six arms, records the build on each, and looks for a stopped world or a median interval above twice the retained comparison. The check reads only the launcher and two JSON files. Consider a change to the Mac frame driver after a successful recording. Those three inputs stay identical, so the cached check can still pass even if the new driver stops advancing the held world. Even a fresh invocation reads the old record. A populated build field satisfies the stated field check. The plan's stop rule requires runs of the commit being claimed. This is a verified gap in the proposed check and its declared inputs. [M5.11 and its reads](../../plans/SHOWCASE/DONE.json) · [Plan stop rules](../../plans/SHOWCASE/PLAN.md)

Tie each pair to the relevant build and route inputs at the claimed revision, and make that identity part of the check's reuse decision. Existing timing receipts already carry native binary and build-source identities, and the browser capture records the served artifacts. This can stay a cheap record check. The retained capture-on medians of 16.808 ms for the Mac route, 16.705 ms for the Mac hold and 16.7 ms for Chrome support the proposed collapse comparison. [Timing receipts](M5-motion-diagnostics.json) · [Browser capture manifest](../../../scripts/capture_v1_browser.mjs)

### MF4 · Must Fix · M7 still requires a cause that the revised timing design says the run cannot establish

The new shared-machine rule explicitly applies through M7. Yet M7.9 and M7.10 still require a cause for **every** missed opportunity or long callback. The retained Chrome take gives the concrete case. All 1,450 submitted images arrived, two intervals lasted 121.8 and 176.1 ms, other builders were active, and the cause of each delay remains unresolved. The timeline establishes the gap and can explain the displacement between images. It cannot by itself apportion the delay between the host and the engine. Under the written M7 items an honest investigation can still block M7, or a reviewer could accept shared load as proof of the cause. [Shared-machine rule](../../plans/SHOWCASE/DESIGN.md#motion-on-a-shared-machine) · [M7.9 and M7.10](../../plans/SHOWCASE/DONE.json) · [Motion handoff](M5-handoff.md)

Allow an investigated interval whose cause remains unresolved under the recorded load to be carried to M8. Keep its duration, evidence and investigation in the frame record. This preserves G7's hitch reporting, the escalation of identified engine defects, and M8's review of visible stutter at 1× with the other builders paused. [Project timing policy](../../../ORCHESTRATE.md) · [M8 closing criteria](../../plans/SHOWCASE/DONE.json)

## Previous suggestions

- **S1 · Suggestion, still open.** M7.5 requires a shared comparison encoding. The Apple presentation design now specifies the EDR transform and its identity at headroom one. SHOWCASE still leaves the comparison stage and treatment of headroom unspecified. Name them when calibrating the three-row metric and retain the Apple row's separate EDR evidence. [Apple EDR transform](../../plans/APPLE-PRESENTATION/DESIGN.md#the-edr-transform-and-its-h--1-identity-m6) · [M7.5](../../plans/SHOWCASE/DONE.json)
- **S2 · Suggestion, still open.** The M5 diagnostics establish experience with one route and held take. M8 still asks for a full day cycle on each row. Give the builder the intended extent, duration, peak storage allowance and excerpts to retain for those sessions. T2 already calls for a separate allowance beyond its four-second SDR case. [T2 storage policy](../../plans/T2-CAPTURE/DESIGN.md) · [SHOWCASE proof handoff](../../plans/SHOWCASE/DESIGN.md#proof-and-builder-handoff)

## Assessment of the revised milestones

| Change | Judgement against the request |
|---|---|
| M5 bar, dependencies and M5.8 | Judging per-tick content here and real-time smoothness at M8 preserves both obligations. The new dependency has a concrete output. ATMOSPHERE M10 keeps a marked balloon in the Sun cascades while excluding it from the far caster height map. Its GPU comparisons and on-route inspection cover the risk of simply removing every balloon shadow. M10 is planned and still open at this revision. MF1 and MF2 concern how SHOWCASE consumes its result. |
| M5.9 and M5.11 | The six paired stops and Chrome's own consecutive flythrough submissions can expose missing props and jumps. Read the source-tick identities for the inspected sequence. The project has already observed a Chrome fixed callback defer a due tick under load. Real-time capture-on and capture-off arms keep that timing visible. MF3 concerns which build those arms qualify. |
| M7 bar, dependencies and M7.1 | M7 still requires the actual scene and resolved settings on each row. M7.1 reads both milestone status and the completion record for its five inputs. CULLING M3 is already required through M5. APPLE-PRESENTATION M11 now exists in the plan and follows M12. The Mac panel qualification and the fresh creator run are consumed at M8. MF4 concerns the separate M7 frame-record items. |
| M8 bar, dependencies and M8.12 | M8.12 checks eleven named prerequisites, including G3s closure. M10 is already required through M5. M8.2, M8.3 and M8.5 judge elapsed motion on all three rows, and M8.6–M8.7 distinguish the Mac presented-time witness from Chrome's callback clock. M8.8–M8.10 retain the clause-by-clause closing record, digest and device work owed. |

I ran the M7.1 and M8.12 record checks. Both refused the three outstanding inputs at this revision, APPLE-PRESENTATION M11, ATMOSPHERE M6 and CONTENT-LOOP M3. This is the expected result for those checks today. The dependency split preserves the representative 10,000-entity scene with 1,500 movers, shared controls and the complete Apple presentation features at exit. [Milestone criteria](../../plans/SHOWCASE/DONE.json) · [Aloft-caster ruling](../../plans/ARCHITECTURE.md#seam-ruling-for-atmosphere-m10) · [Chrome fixed-clock ruling](../../plans/ARCHITECTURE.md#a-fixed-capture-that-stalls)

**Suggestion, verified.** Bring the handoff's remaining historical instructions up to date. It still describes presentation M11 as unplanned, gives M8's item range only through M8.11, and calls for pre-landing verification. The current project ruling already assigns exact-image proof to the shared main-branch check. [SHOWCASE design](../../plans/SHOWCASE/DESIGN.md) · [SHOWCASE plan](../../plans/SHOWCASE/PLAN.md) · [Governing proof rule](../../plans/ARCHITECTURE.md#proofs-run-once-on-main)

This review judges the design and the feasibility of its proofs. I traced the capture and comparison paths and checked retained records. I made no new device or visual qualification.

DESIGN CHANGES: 4 must_fix findings
