# PLATFORM-RUNTIME-MAC plan

Qualifies the Mac frame runtime on the M3 Max and its built-in 120 Hz display (gate G3b). [Design](DESIGN.md), under
[the shared platform runtime](../../design/modules/PLATFORM-RUNTIME.md#g3b-world-and-presentation-lifecycle--selected-partial-implementation)
and [the bounded frame host](../../design/decisions/V1-BOUNDED-FRAME-HOST.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md) · siblings [iOS](../PLATFORM-RUNTIME-IOS/PLAN.md) and
[web](../PLATFORM-RUNTIME-WEB/PLAN.md)

## Open for the owner
- Assumption: the OS-reported presented time of each game drawable is an acceptable witness for built-in ProMotion
  cadence, in place of the Instruments display-swap table, which is empty on this Mac. It is not the kernel's
  swap record or photons. Overturning it means finding why the display source is silent before WP2 can close.

## WP0 The Mac qualification plan passes independent review: done
[record and review](../../../.orchestrate/evidence/PLATFORM-RUNTIME-MAC/WP0-M1-records.md#m0-reviewed-mac-qualification-plan-done)

## WP1 The same Mac game resumes after graphics loss: done
[record and review](../../../.orchestrate/evidence/PLATFORM-RUNTIME-MAC/WP0-M1-records.md#m1-the-same-mac-game-resumes-after-graphics-loss-done)

## WP2 The cooked valley presents smoothly on built-in ProMotion

The [13 November current-build proof](../../../.orchestrate/evidence/render/mac-m2-20261113/README.md) supplies fresh motion,
window replacement and matched cadence arms from the merged `7a791b20` source.

Needs `T2-CAPTURE:WP0` only; no item waits on `APPLE-TILE`. Each item's full protocol is in
[the design](DESIGN.md#wp2-and-wp3-qualification-protocol), and the done-when file owns the criteria. Cadence is
judged from the OS-reported presented time of each game drawable, because both 11 November traces had empty
display tables for a cause that is not authorization
([diagnosis](../../../.orchestrate/evidence/PLATFORM-RUNTIME-MAC/m2-display-event-diagnosis.md)).
[The attended runbook](../../../.orchestrate/evidence/render/mac-m2-presented-20261111/RUNBOOK.md) holds the tested collection commands; its
trace steps are replaced by WP2.4 and WP2.5. For the attended items the owner unlocks the Mac and wakes the
built-in panel, and nothing else. Verdicts on `0d53d3aa` qualify only that build
([history](../../../.orchestrate/evidence/render/mac-m2-pan/README.md)).

- [x] WP2.1, unattended: current-build runtime content motion over the two-second 0°–36° noon pan and both
      one-second holds, with denied Screen Recording preflights and console state recorded as a witness only.
- [x] WP2.3, attended setup: the shared T2 tap on the real CAMetalLayer window, with output correspondence,
      lease, replacement, stale-generation refusal and capture-OFF direct-path proofs.
- [x] WP2.4, unattended: one reusable command that turns a trace's `ca-client-presented-handler` rows into
      presented intervals per drawable. On the retained 11 November tables it finds the three known two-refresh
      intervals and joins drawable 545's to the game's own present-call gap. No recording is needed.
- [x] WP2.5, attended setup: a fresh recording of the release game whose retained rows span both `MacM2Whole`
      markers, with every submission joined, the time conversion validated against the markers and the display
      tables' row counts recorded.
- [x] WP2.2, attended setup: after WP2.3, WP2.4 and WP2.5, three matched arms on WP2.1's build, with the normal
      AppKit slider action: capture ON untraced, capture ON traced, capture OFF traced. The OFF arm gives the
      cadence result, with each interval over one refresh period listed and investigated.
- [x] Callback target, opportunity, acquisition and submit, GPU timestamps, engine image time, movie PTS and
      OS-reported presented time recorded as separate measurements, with unmatched samples.

Stop rules:
- Six trace recordings are granted across WP2.5 and WP2.2, each at most 16 seconds with at most 75 seconds of
  finalization, under the `gpu` lock. An WP2.5 recording that meets every OFF-arm condition may serve as that arm.
  When the six are used without both items passing, stop with them open; more recordings need a scoped re-plan.
- A recording that is windowed short of the markers changes the recording configuration, not the witness. Do not
  repeat an unchanged configuration.
- Empty Instruments display tables are recorded and do not block. Do not change Developer mode or any other
  security setting to chase them.
- A locked or sleeping display, an uncovered interval or ambiguous attribution leaves WP2.5 or WP2.2 open. It does
  not block unattended visual work.
- WP2 closes only when all five items pass independent review. WP2.3, WP2.4 and WP2.5 pass before WP2.2's final run.

## WP3 After the Apple tile change, the cooked valley still moves and presents smoothly on built-in ProMotion

Needs `APPLE-TILE:WP3`, and Mac WP2 by module order. It repeats the whole qualification on the changed surface and
present path; [the design](DESIGN.md#wp2-and-wp3-qualification-protocol) has the protocol and the dependency chain.

- [ ] WP3.1, unattended: the complete WP2.1 protocol on the integrated release build, with fresh build, shader,
      content and recipe identity.
- [ ] WP3.2, attended: WP2.3's windowed-tap proofs revalidated, WP2's qualified recording configuration and witness
      command rerun, and all three WP2.2 arms on WP3.1's build.

Stop rules:
- A tooling regression needs a scoped re-plan; this work package grants one recording per traced arm and no
  diagnostic recordings.
- Window logs that only show function cannot pass. Both items need a fresh independent review.
