# T2-CAPTURE design review

9 November 2026. Reviewed integrated revision `bce5a43206979da35aba1f1d432dc74f8e9da5a7` using
`orchestrate:review-milestone`, “Reviewing a design”, and Thrum's engineering review guidance.

**Verdict — DESIGN OK.** No blocking design findings. One verified advisory concerns stale status prose.
This assessment covers the current brief, design, plan and every done-when definition, including the open
simulator console-witness correction. It grants no milestone, implementation, visual or device qualification.
No agents were launched. Only this review file is changed.

## Requirements derived from the brief

The brief was read before the design and criteria. It requires the following capabilities.

1. Reproducible exact-build stills, fixed-time sequences and separately labelled elapsed-time runtime motion of
   actual engine output, with target, recipe, content and control provenance.
2. Ordinary world, input, interpolation, history and submission semantics. Preserve clock domains, repeated
   images, stalls and missing samples so slow rendering cannot become a smooth synthetic-time movie.
3. One rendering authority with bounded asynchronous GPU readback, CPU buffering, storage and finalization.
   Disabled capture adds no image copy, readback, allocation or synchronization.
4. Preserve accepted Mac/Chrome capability witnesses. Open proofs require denied, attributed read-only Screen
   Recording preflights before and after. Console state is metadata, including transitions or an explained
   unavailable observation. Its value must never cause admission, refusal or a manual-lock wait.
5. Retain the complete paused-world Mac noon pan and a separate running-world interpolation proof. Qualify
   Chrome and simulator consumers independently, with native-size images, normal-speed motion and negative controls.
6. Separate engine evidence from panel cadence, compositor/native UI, OS input/focus and physical EDR/photon
   observations. Label genuinely attended rows with their reason and preserve the historical cadence obligation.
7. Measure observer overhead and resource scaling under C12, without adding a numerical speed gate or paid tool.

## Coverage and mechanism assessment

| Requirement | Design and proof coverage | Assessment |
| --- | --- | --- |
| Exact output and provenance | Mechanism steps 2–5, completeness section, M0.1/.4/.9, M1.1, M2.1 | Final submitted output is joined to generation, frame, build, content, recipe and image digest. Endpoint still oracles retain explicit separate semantics. |
| Runtime honesty | Evidence-mode table, ordinary elapsed-time driver, host-derived PTS, M0.5–.7, M1.2, M2.1/.2 | Fixed-time smoothness, delayed map completion and encoded duplicate frames cannot count as fresh runtime samples. Unknown gaps refuse completion. |
| Ownership and bounded work | FrameHost tap, coordinator graph copy, bounded staging/CPU/encoded pools, finite output allowance and drain deadline | Consistent with C02/C06/C07/C11/C12 and TOOLS-DELIVERY. No extra device, clock, private submit or drawable lease is commissioned. |
| Console-witness correction | Completeness and rollout sections, M2 plan, M2.1 statement/ask, M2.2 ask | Denied permission proof is mandatory and attributed to the actual launcher/harness. Missing, failed or granted preflights cannot pass. Both lifecycle commands inherit the policy. |
| Target qualification | Separate M0/M1/M2 and M2 prerequisite `COOKED-DRAW:M1` | Simulator cooked drawing belongs to G5. T2 adds submitted-output capture and timing on the ordinary foreground route after that prerequisite. Simulator results remain function only. |
| Attended boundaries | Rollout table, C12, profile and plan handoff | Mac real-window tap, capture-OFF cadence and bounded trace investigation have separate owners. Engine PTS cannot qualify swaps or photons. |
| Cost and retention | Cost/failure account, M0.2/.3/.8/.9, target saturation controls | Raw bandwidth, finite pools, encoded reserve, durable flush, packaging and peak output coexistence are accounted for. New content repeats throughput qualification. |

The normal frame has a complete ownership chain. Scenario actions enter the normal semantic handler and world
lane, the render actor admits an opportunity, FrameHost produces the ordinary history-bearing output, and the
coordinator encodes its readback copy. Generation-tagged completion transfers pixels to bounded writer ownership.
Packaging verifies originals and timing before atomic completion. The paused-world output uses the ordinary
presenter/SurfaceOutput admission path without advancing authority. The simulator retains the foreground lease
and cancels pending old-generation work rather than publishing it after recovery.

No unsupported scope expansion was found. A physical-device speed claim, native UI recorder, new crate or separate
rendering authority is outside the commissioned work. The writer is explicitly qualified for measured content,
with overflow requiring an incomplete result and a rerun. Future scene-linear/EDR capture is a declared extension
with additional stage/format costs and separate qualification.

## Done-when judgeability and convincing-fake checks

The inspection questions used for this document review were whether each item names an observable result, whether
its proof reaches the required target and boundary, and whether a plausible substitute can pass while missing the
brief. These are design assessments of the definitions, not renewed verdicts on accepted milestone evidence.

| Item | Judgeable proof and substitute it excludes |
| --- | --- |
| M0.1 | Trace the submitted texture through graph copy and hash, compare output/history on real Metal, exercise channel/padding and disabled-resource controls. Excludes endpoint redraw and a private submission path. |
| M0.2 | Pending-map loss, stale callbacks, saturation, write/limit errors and deadline failure with continuing world/render work and finite byte accounting. Excludes hidden blocking or successful incomplete evidence. |
| M0.3 | Actual disk writer probe, decoded original hashes, flush/drain costs and complete four-second workload. Excludes cached write acceptance, fidelity reduction and a session-sized RAM substitute. The evolved compressed-writer mechanism is explained in the design and retained accepted history. |
| M0.4 | Retained real-Metal commands with full identity, actual elapsed time, shared dispatch and paused committed tick. Excludes synthetic clocks and capture-only admission. Its locked-condition requirement belongs to accepted historical scope. |
| M0.5 | Complete 1080p paused pan, two-second transition and one-second endpoint holds, native crops and 1x host-time inspection. Excludes fixed-step footage, endpoint redraw and hidden omissions. |
| M0.6 | Moving-actor images joined to committed endpoint pairs, intermediate alpha and history, plus native and 1x inspection. Excludes endpoint-only presentation and history resets. |
| M0.7 | Separate real render/input stalls, recorded host intervals and ordinary-run controls. Excludes timestamp editing and always-enabled diagnostic hooks. |
| M0.8 | Matched on/off workload with order, load, CPU/GPU, queues, memory, bytes, disk, flush and packaging observations. Excludes calling captured timing an uncaptured baseline. |
| M0.9 | Rerunnable runbook, retention and target/attended handoffs. Excludes undocumented workflows and engine-image claims about display cadence. |
| M1.1 | Actual hardware WebGPU bundle, browser flags/scheduling, output identity and full-resolution checkpoints/1x motion. Excludes software fallback, fake visibility and display-path substitution. |
| M1.2 | Pending-readback loss, resize, saturation and stall controls with continuing authority, coverage refusal and actual-tab regression checks. Excludes headless proof replacing ordinary browser lifecycle qualification. |
| M2.1 | Real cooked simulator foreground route, native stills and 1x camera/actor motion through the shared tap/control handler, exact identity and denied attributed preflights. Excludes unrelated denied helpers, console-lock admission and simctl snapshots standing in for submitted motion. |
| M2.2 | Pending-capture background/foreground, surface/device replacement and saturation/stall rows with generation/lease traces, fresh resumed output, host PTS and incomplete-result refusal. Excludes background submit, stale publication and lifecycle counters standing in for captured pixels. |

M2.1/.2 are broad but coherent target-rollout items. Their asks require target-specific commands beyond the
workspace check, fail on unavailable required hardware and distinguish semantic actions from actual UIKit touch.
A reviewer can require acceptance controls for locked, unlocked, changing and unavailable console observations,
and refusal controls for missing, failed, granted or misattributed permission proof, directly from these definitions.
Neither permission denial alone nor a retained old simulator screenshot can close the items.

## What checks out

The shared contracts and current profile preserve the ownership and attended boundaries above. The COOKED-DRAW
criteria M1.7/.8 implement the same denied-preflight and console-witness policy, and M1.11 explicitly leaves
every-submitted-frame motion proof to T2 M2. The integrated history contains `7f70ef13`, the COOKED-DRAW M1 pass,
followed by `bce5a432`, the T2 M2 unblock. The T2 prerequisite therefore remains meaningful without a new dependency
on Mac panel qualification or APPLE-TILE.

The older review formerly at this path recorded reading a 01:59 CET locked-console probe followed by the successful
02:00 CET offscreen parity record. The design still links this section for that historical account. This review
does not independently re-establish that probe or turn it into current motion qualification. Accepted M0/M1 records
retain their historical witnesses, while open M2 explicitly uses the revised console policy.

## Findings

### A1 — Stale current-status prose obscures accepted prerequisite progress

**Advisory, verified from documents and integrated history.** `T2-CAPTURE.md` lines 3–4 describe Chrome M1 as
claimed for review, although `T2-CAPTURE-DONE.json` marks it done. `T2-CAPTURE-PLAN.md` lines 6–11 still describe
COOKED-DRAW M1 as claimed and M2 as blocked on its acceptance, although the integrated tip includes its pass and
T2's unblock. TOOLS-DELIVERY's “Commissioned capture child” also calls the plan unbuilt and independently unreviewed.

Reproduce by comparing those current-status paragraphs with the done-when records and `git log -4 --oneline` at
this revision. The expected status prose should identify accepted Mac/Chrome and COOKED-DRAW progress while keeping
T2 simulator capture unproved. Update those summaries during the next authorized documentation/build change.
The dependency, open simulator definitions and required proofs are consistent, so this prose drift does not block
design approval. Historical log entries should keep their historical meaning.

## Scope and verification

Read the brief first, then the complete T2 design, plan and JSON definitions, the profile, build-plan target guidance,
project and skill graded examples, Thrum review/design guidance, relevant C02/C06/C07/C11/C12 contracts,
TOOLS-DELIVERY ownership and COOKED-DRAW simulator criteria. Older T2 reviews were consulted after forming the
coverage assessment. Inspected the capture runbook's target and permission guidance and the integrated history.
No engine implementation audit, external API revalidation, GPU run or new image/motion inspection was performed.
Those belong to milestone verification and receive no new claim here.

`./dev check-docs` and `git diff --check` pass. Final status contains only this review file.

DESIGN OK
