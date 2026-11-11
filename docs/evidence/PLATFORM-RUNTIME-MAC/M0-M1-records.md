# PLATFORM-RUNTIME-MAC M0–M1: records

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. M0 and M1 were closed before the loop kept reviews as files; their verdicts are in [the log](../../plans/PLATFORM-RUNTIME-MAC/LOG.md) and their briefs in [review briefs](review-briefs.md). Sections are unchanged.

## M0 Reviewed Mac qualification plan: done

M0.1 passed: independent review verified runtime ownership, the two game outcomes and instrumentation limits.
[Verdict record](../../plans/PLATFORM-RUNTIME-MAC/LOG.md) · [brief](review-briefs.md#design-and-plan-review-brief).
The [build plan](../../plans/BRIEF.md#active-gate-order-and-budgets) links this plan.

## M1 The same Mac game resumes after graphics loss: done

Owner review: [Ground E route, noon tile](../v1-workload/g7-showcase/route-ground-e.png) (proposed, pending).
Use the checked-in noon preset and the same-build uninterrupted run as the exact image control. The reference supplies the expected intact composition.

M1.1 and M1.2 passed: held-drawable loss recovered in the same PID/window with continuing or paused authority,
no old-queue calls after destroy, identical paused pixels and visible input response after replacement.
[Raw events, audit and independent review](../render/mac-m1-recovery/README.md).

Work:

- [x] Give an encoded frame an explicit lost-device abandon path. The Mac actor checks observed loss before and after drawable acquisition, including acquisition refusal. Abandoning releases the encoder and timing reservation without flushing the queue or committing view history. Healthy skip still flushes once. A real Metal test checks the queue call boundary. Replacing abandonment with ordinary drop produces an extra attempted submit and fails the test. [Focused evidence](../render/V1-BOUNDED-FRAMES-2026-11-04.md#mac-lost-encoded-frame).
- [x] Build a one-shot diagnostic destroy after acquisition and before view creation or submit. Observe the normal generation-tagged callback, abandon the encoded frame and release the drawable through its owner. Ordinary builds carry no diagnostic hooks. [Witness build and events](../render/mac-m1-recovery/README.md).
- [x] Run the detached Metal and paused readback preflights, then the cooked valley in the same diagnostic build with the arm disabled and with both loss modes. The arm-disabled row supplies the uninterrupted control with identical inputs.
- [x] Record device, surface, session and world identity, the acknowledged endpoint, queue calls, cooked generation, and the first restored submission. Retain the motion excerpt and independent review alongside the paired image results.
