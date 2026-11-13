# PLATFORM-RUNTIME-MAC design review

11 November 2026. Reviewed integrated tip `3e5be4a760190434acd2655d43d7cf92c549ee8e` using the
review-milestone skill's “Reviewing a design” procedure and Thrum's engineering review rules.

No Must Fix findings or Suggestions.

## Previous round

The previous review, dated 10 November against `9d6c6f6fcdd0b7ad533cce994a01f9567ab33887`, ended
`DESIGN OK` with no blocking or advisory findings. There are no outstanding Must Fix findings to resolve.
I compared the current definitions with that revision and with the pre-reshape checkout `42fdce97`,
then read every current item and the relocated protocol. This is a review of the existing qualification
design and its reshape, not a new technique-selection or implementation review.

## Request and required outcome

I read the complete `briefs` array in [ACCEPTED.json](../../plans/PLATFORM-RUNTIME-MAC/RECORD.json)
and [the project brief](../../plans/BRIEF.md) before assessing the design. The array contains one task:
reshape the migrated plans into orchestrate 1.7.0 form, retaining milestone scope and done-when items,
moving detailed designs and review briefs to their appropriate files, shortening done milestones to
headings and review links, and repairing moved links. It does not commission a new runtime mechanism.

For this module, the project brief's G3b and the module-list entry require recovery of the same Mac game
and qualification of motion on the built-in ProMotion display. The existing contracts require one
world clock, platform-owned surface/device lifetime, renderer-owned GPU submission and history, and
separate evidence for engine images, elapsed-time motion and independently attributed presentation.
The post-Apple-tile assembly must be qualified afresh. Numeric timings remain observations; visual
hitches require investigation. Browser, iOS, G4 and physical input-to-photon qualification keep their
separate scopes.

## Reshape and mechanism assessment

The module's [plan](../../plans/PLATFORM-RUNTIME-MAC/PLAN.md) now has short outcome-named milestones.
M0/M1 consist of headings and links to retained records, which lead to their historical verdicts.
Detailed M2/M3 protocols moved into [the design](../../plans/PLATFORM-RUNTIME-MAC/DESIGN.md#m2-and-m3-qualification-protocol).
Review briefs and the console-witness commission are retained under this evidence folder and linked
from the log. The plan still links the attended runbook. The project brief and module list retain G3b
and its owning module.

The comparison against `42fdce97` found only milestone-name and relocated evidence/review-pointer
changes in `DONE.json`. No statement, inspection, reviewer question, automated check, target, quality
bar, dependency or acceptance state changed in the reshape. Compared with the previous reviewed tip,
there are also intervening builder status/evidence updates; those are not new acceptance definitions
or independently awarded milestone passes. The legacy `docs/BUILD-PLAN.md` wording remains in the
closed M0 question; its successor G3b link is in `docs/plans/BRIEF.md`. This historical wording does
not make an open item unjudgeable or warrant changing the preserved criterion.

The design's ownership and recovery sequence remain consistent with C02/C06/C07 and the shared
platform/frame-host designs: close old-device admission, abandon lost encoded work without queue
flush or history publication, retain authority and controls, then prepare and publish a fresh
validated generation. Surface and device epochs remain distinct. The same-binary control, paused
image oracle and running/paused loss rows distinguish recovery from a restart or frozen screenshot.

The relocated protocol preserves C12 and T2's capture boundaries. Actual elapsed time and ordinary
semantic controls drive unattended motion, including while authority is paused. Complete submitted
images and control/submission identities expose omissions and holds. Console observations accept
both states and transitions; denied attributed permission preflights establish the separate
capture-independence claim. The real-window tap must share the presented producer, reject stale
readback after replacement, and leave the disabled direct path free of capture work.

The cost model is inherited from T2's bounded staging, CPU buffers and writer queues, with explicit
incomplete outcomes instead of blocking the renderer. Matched ON/OFF and traced/untraced arms measure
observer costs. Only the capture-OFF arm qualifies shipping cadence, through fresh attributed swaps
and a calibrated full-interval clock join. A requested 120 Hz workload or movie timestamps cannot
stand in for those observations.

The unresolved trace-finalization mechanism remains an explicit bounded investigation. A changed
hypothesis precedes recording; the two-attempt allowance is not renewed by the reshape or M3.
Unreadable exports, ambiguous attribution or exhausted attempts leave the relevant items open.
The linked runbook accounts for both traced arms within the allowance and requires usable exports,
not merely a trace directory. This is a judgeable experiment, not an unsupported promise of success.

M2 needs T2 M0 and has no Apple-tile delivery dependency. M2.3/M2.4 precede final M2.2 qualification.
M3 follows Mac M2 and Apple-tile M3, revalidating motion, window capture and cadence on the changed
assembly. The T2 and Apple-tile dependency records preserve this ordering without a cycle.

## Acceptance-item audit

Design inspection covered retained scope, state ownership, failure ordering, clock distinctions,
complete motion coverage, permission/console handling, window-producer correspondence, stale work,
disabled observer cost, bounded investigation, attribution and post-change requalification.
No runtime image or motion inspection was performed in this design review.

| Item | Judgeability and plausible false proof excluded |
| --- | --- |
| M0.1 | Historical ownership and instrumentation review has retained records and a current G3b link. No new runtime claim is inferred from it. |
| M1.1 | Same-binary control and loss rows, queue-call audit, retained PID/window, acknowledgements and tick ordering distinguish retained authority from restart, hidden pause or old-queue work. |
| M1.2 | Complete RGBA equality plus composited post-replacement input response distinguishes intact recovery from a matching frozen endpoint. |
| M2.1 | Full elapsed-time sources, endpoint holds, yaw/submission joins and denied attributed preflights are observable after T2 M0. Fixed-time animation, static parity and missing frames cannot pass. |
| M2.2 | Fresh numeric swaps, unique app/window/display attribution, matched observer arms and a bounded-error join cover the full pan and holds. Capture-ON or historical swaps cannot qualify shipping cadence. Unavailable evidence keeps the item open. |
| M2.3 | Code tracing, real-Metal checks and a live-window pattern/replacement scenario jointly test producer correspondence, drawable lifetime, stale-readback refusal and the OFF path. Offscreen-only proof cannot pass. |
| M2.4 | The hypothesis, attempt journal, deadlines, fresh exports and reusable command make success or failure observable within M2. Tool completion without attributable swaps cannot pass. |
| M3.1 | Incorporates every M2.1 requirement on the post-Apple-tile build with fresh source/shader/content/recipe identity. An earlier verdict cannot qualify changed output. |
| M3.2 | Incorporates the complete M2.2 protocol and revalidates M2.3/M2.4 on that assembly. Earlier traces, window-function logs and renewed diagnostic retries cannot substitute. |

## Verification and limits

Read the full module design, plan and all nine acceptance items; the saved request and previous review;
the reshape diff and moved records; C02/C06/C07/C12; the shared platform and bounded-frame-host designs;
T2's relevant ownership, frame flow, cost and rollout sections; the dependency records; and the current
attended runbook. The project and skill calibration examples informed the review.

`./dev check-docs` passed: 663 Markdown files, 3,193 local links, 30 module records, crate policy and
current design hashes. `git diff --check` passed after writing this review.

This verdict assesses design and proof sufficiency, not implementation correctness, current motion
quality or panel cadence. No runtime/GPU capture, trace attempt, permission change or other agent
was launched. Only this review file was changed; no milestone verdicts were edited.

DESIGN OK
