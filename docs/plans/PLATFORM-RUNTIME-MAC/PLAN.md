# PLATFORM-RUNTIME-MAC plan

Qualifies the Mac frame runtime on the M3 Max and its built-in 120 Hz display (gate G3b). [Design](DESIGN.md), under
[the shared platform runtime](../../design/modules/PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [the bounded frame host](../../design/decisions/V1-BOUNDED-FRAME-HOST.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · siblings [iOS](../PLATFORM-RUNTIME-IOS/PLAN.md) and
[web](../PLATFORM-RUNTIME-WEB/PLAN.md)

## M0 The Mac qualification plan passes independent review: done
[record and review](../../evidence/PLATFORM-RUNTIME-MAC/M0-M1-records.md#m0-reviewed-mac-qualification-plan-done)

## M1 The same Mac game resumes after graphics loss: done
[record and review](../../evidence/PLATFORM-RUNTIME-MAC/M0-M1-records.md#m1-the-same-mac-game-resumes-after-graphics-loss-done)

## M2 The cooked valley presents smoothly on built-in ProMotion

Needs `T2-CAPTURE:M0` only; no item waits on `APPLE-TILE`. Each item's full protocol is in
[the design](DESIGN.md#m2-and-m3-qualification-protocol), and the done-when file owns the criteria.
[The attended runbook](../../evidence/render/mac-m2-20261110/RUNBOOK.md) runs M2.4 and M2.2 in one sitting: the
owner arranges the unlocked active built-in panel and any Instruments authorization, and the builder collects
and reviews. Verdicts on `0d53d3aa` qualify only that build
([history](../../evidence/render/mac-m2-pan/README.md)).

- [x] M2.1, unattended: current-build runtime content motion over the two-second 0°–36° noon pan and both
      one-second holds, with denied Screen Recording preflights and console state recorded as a witness only.
- [ ] M2.4, trace tooling: a changed, falsifiable hypothesis from the retained failed bundles, then at most two
      new diagnostic recordings under the 9 November commission. Success is a fresh readable TOC and
      `displayed-surfaces-interval` export with real app and window swaps, plus a reusable command.
- [x] M2.3, attended setup: the shared T2 tap on the real CAMetalLayer window, with output correspondence,
      lease, replacement, stale-generation refusal and capture-OFF direct-path proofs.
- [ ] M2.2, attended setup: after M2.3 and M2.4, three matched arms on M2.1's build, with the normal AppKit
      slider action: capture ON untraced, capture ON traced, capture OFF traced.
- [ ] For the OFF traced arm, a fresh readable Game Performance trace with one app-labelled swap stream for
      the game's window on the built-in display, joined to the full pan and both holds.
- [ ] Callback target, opportunity, acquisition and submit, GPU timestamps, engine image time, movie PTS, VSync
      and app-attributed swaps recorded as separate measurements, with unmatched samples.

Stop rules:
- No justified hypothesis, unavailable authorization or the two attempts used up leaves M2.4 and M2.2 open.
  Record the cause and continue unblocked work; more attempts need a scoped re-plan.
- A locked display, an unreadable export or ambiguous swap attribution leaves M2.2 open. It does not block
  unattended visual work.
- M2 closes only when all four items pass independent review. M2.3 and M2.4 pass before M2.2's final run.

## M3 After the Apple tile change, the cooked valley still moves and presents smoothly on built-in ProMotion

Needs `APPLE-TILE:M3`, and Mac M2 by module order. It repeats the whole qualification on the changed surface and
present path; [the design](DESIGN.md#m2-and-m3-qualification-protocol) has the protocol and the dependency chain.

- [ ] M3.1, unattended: the complete M2.1 protocol on the integrated release build, with fresh build, shader,
      content and recipe identity.
- [ ] M3.2, attended: M2.3's windowed-tap proofs revalidated, M2.4's qualified trace command rerun, and all
      three M2.2 arms on M3.1's build.

Stop rules:
- A tooling regression needs a scoped re-plan; this milestone grants no new diagnostic attempts.
- Window logs that only show function cannot pass. Both items need a fresh independent review.
