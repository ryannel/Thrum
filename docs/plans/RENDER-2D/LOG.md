# RENDER-2D log

- 2026-11-13 Planned from the lead's task, "Plan module RENDER-2D. It needs: SHOWCASE, APPLE-TILE, CONTENT-LOOP"
  (row 18 of the module list); no words of the owner's were passed on, so the design quotes the owner's page of
  10 November. Five milestones replace the brief the module list carried, whose text is in git
  (`git show 54ab55bb:docs/plans/ARCHITECTURE.md`). M1 waits on `SHOWCASE:M8`, `APPLE-TILE:M12` and
  `EFFECT-GRAPH:M3`; M4 on `CONTENT-LOOP:M3`; M5 on `SIM-2D:M1`.
- 2026-11-13 Planner's decisions, all four left to the planner by that brief. Order is sorted rows on the CPU with
  a band against jitter, not depth. No reconstruction: a 2D scene is drawn at the surface's pixel size. Values
  above white are kept and handed to the output half; SDR clips, and the EDR look is not judged here. Text shares
  the path as rows: a glyph is a frame, and `GAME-UI` runs the pass at its own stage.
- 2026-11-13 Planner's decision on the asks of other modules. The module list said each ask "becomes a milestone
  of that module", and `CONTENT-LOOP`'s design says the same of the sheet product. The owner's 10 November task
  also said "Don't change V1 modules' plans", and builders are on those modules now. So the three asks this module
  needs are built by its own builder after V1, under seam rulings in the module list, as `CREATOR` M2 and
  `CONTENT-LOOP` M2 were. No V1 module's design, plan or done-when file changed. It is an assumption in the plan.
- 2026-11-13 Planner's probe, in [the evidence folder](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md). Coverage
  sampling equals the exact box filter at every scale of 1 or more, to rounding error, while nearest sampling
  makes a texel 2 or 3 pixels wide by turns at a scale of 2.37. Ordering 10,000 rows costs 71 µs natively. All
  256 sRGB codes survive the half-float target. None of it ran on a GPU; M1 and M3 repeat it there.
- 2026-11-13 Planner's findings in the code. `FrameHost` holds one prepared 3D scene and its scene source returns
  a 3D view, so choosing a scene kind is a change to the host, not a flag. Both tone operators alter colours
  below white and the 8-bit output is dithered, so authored sprite colours need a display-referred transfer. The
  contribution seam's `rebuild` is given the 3D forward pass's layouts, so residency needs a leases-only form in
  a 2D scene. The iOS shell is tied to the V1 game, so the simulator is reached through a scene selection there.
- 2026-11-13 Researched: Bevy's sprite renderer (`bevy_sprite_render` 0.18.1, which breaks a batch at each image
  change), Godot's batching guide, Hargreaves on premultiplied alpha, d7samurai's antialiased point sampling, and
  WebGPU's limits table. No primary source was found for how Celeste, Vampire Survivors or Halls of Torment draw.
- 2026-11-13 Checks: the done-when file passes the loop's validator; every check was run by hand except the
  verifier and each fails, as it should before the work exists. The verifier items were not dry-run, because the
  GPU is shared.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
  The reviewer should also read the seam rulings in the module list, which this design depends on.
- 2026-11-13 [Design review](../../../.orchestrate/evidence/RENDER-2D/design-review.md): changes requested, two Must Fix findings
  and two Suggestions. The planner resolved all four in the entries below.
- 2026-11-13 Must Fix `sort-band-does-not-prevent-reversals`. The band held around each row's own sort value is
  gone: two rows that travel together use it up and then swap freely, as the review's trace shows. The order is
  now one list kept from tick to tick, in which two near rows exchange only when one has passed the other by more
  than the band. The design states what that guarantees, with the argument, and what it does not: rows that are
  not neighbours in the list can be further out of order through a chain, and no total order avoids that without
  exchanging a pair that has not passed. M2.2 now claims the exact order alone, M2.13 is new and claims the rule,
  M5.3 measures the crowd and no longer rests the guarantee on a trace, and M5.6 also asks that a clearly lower
  body is in front.
- 2026-11-13 Planner's probe 4, in [the evidence folder](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md#4-does-a-kept-order-with-a-band-stop-reversals-and-what-does-it-cost).
  The rule held on the review's pair and on 144,000 ticks of random motion, where the old band failed 2,805 times.
  It first put the band between every two rows; in a packed crowd of 10,000 that left 1.2% of overlapping pairs
  more than a band out of order, growing with the crowd, so the rule applies the band to near rows only (0.04%).
  Researched: no published source for hysteresis in sprite order was found; shipped advice stops at a tie-break.
- 2026-11-13 Must Fix `c07-no-reconstruction-exception-unresolved`. C07 and the hardware-native frame contract now
  name two forms of the frame contract, full for every view of a 3D scene and no reconstruction for a view of a 2D
  scene, with a provider asked for a 2D scene refused by name. Seam ruling 1 in the module list states it, with
  the frame runtime as producer and `APPLE-PRESENTATION`'s registry and governor as consumer. No 3D view's
  obligations changed. M1.2 now also checks that no reconstructor is resolved, and M1.3 the refusal. The current
  design hashes in the maturity ledger were refreshed for the changed contract file.
- 2026-11-13 Both Suggestions taken, because each removes a wording a reviewer could fail a correct build on.
  M3.1 retains a pixel figure and a weight figure, measured apart, and M3.2 and M3.7 test the weight. M1.2 names
  the scene with no contribution, and M1.3 and mechanism 1 say what a contribution adds.
- 2026-11-13 The revised design has not been reviewed. No builder starts until a design review returns Design
  Approved. The reviewer should also read C07's new paragraph and seam ruling 1, which the design depends on.
- 2026-11-14 Owner: "Let's move the 2d engine side of things to before v1 and decouple them. But we don't want to start working on the swarm game till v1 lands." M1 no longer waits for `SHOWCASE:M8` or
  `APPLE-TILE:M12`; it waits for `EFFECT-GRAPH:M3` (the after-opaque stage it refuses) and
  `APPLE-PRESENTATION:M3` (the reconstruction a 2D scene declares none of). M4 waits for `CONTENT-LOOP:M1`, not
  M3: a sheet uses the lease and the publication, both M1's. No item changed. The design now says the rulings
  are built on what the owning modules have landed, and that the iPhone app for a second game is `GAME-UI` M6.
  It needs a design review.
- 2026-11-19 M1 adds a sprite scene half to the existing frame host and selects it from the V1 game on Mac and web.
  Both Metal recipes passed submitted-pixel, resource, admission and lifecycle checks. The V1 3D graph control
  matched its starting snapshots. Chrome matched the CPU rasterizer before and after resize and device replacement.
  The four fixed Metal stills and two release timing takes per target are in the [M1 evidence](../../../.orchestrate/evidence/RENDER-2D/M1.md).
  Chrome's sprite median crossed one timestamp step from 1080p to full backing size. The record flags both takes.
- 2026-11-19 M1 review follow-up. The native sprite launch refuses traced geometry and explicit 3D shadows.
  On Metal, upload and simulate projected the known world point to the submitted sprite pixel with a displaced view.
  The cost checks accepted both retained target records and rejected missing build, storage and load observations.
  Timing excursions are still reported without failing a complete record. [Evidence](../../../.orchestrate/evidence/RENDER-2D/M1.md#review-follow-up)
- 2026-11-19 M1 named-refusal follow-up. Sprite startup identifies each rejected content option on Mac and
  Chrome. A Mac release capture reproduced the silent acceptance of explicit shadow maps and the rebuilt
  launch refused that request by name. All 19 Mac startup probes refused before submission, and the Metal
  scene admission and contribution checks passed. [Evidence](../../../.orchestrate/evidence/RENDER-2D/M1/launch-refusals.md)
- 2026-11-19 M1's native sprite launch refuses enabled native witness selections by name. Release checkpoints
  rejected all eight selections before submission on both Metal recipes, and the launch regression and Metal
  scene checks passed. [Evidence](../../../.orchestrate/evidence/RENDER-2D/M1/launch-refusals.md)
- 2026-11-19 M2 maps V1 tick publications to slot-indexed sprite poses and keeps overlap order across ticks.
  The sprite pass uses eight sheet slots and one draw, with frame rows, tint, flash and additive blending.
  Metal readback checks passed for pose and animation, exact and retained order, changed-row uploads and the
  CPU blend oracle. The V1 field drew 10,000 world rows and two frame rows with one draw and no drops on
  Metal and Chrome. The 22-second Chrome realtime capture showed the cyan sprite staying in front of the
  purple sprite through the crossing, with fractional-alpha motion and zero world-clock overloads.
  The [M2 evidence](../../../.orchestrate/evidence/RENDER-2D/M2/README.md) includes both realtime sequences,
  two release cost takes per target, and tripwires observed under load.
- 2026-11-20 M2.9 now extracts adjacent poses from the tick page when Chrome misses a publication.
  Metal readback verified the pose pair and the committed-pose fallbacks after rewind and slot reuse.
  Chrome replayed the original crossing and verified a 22-second realtime take against hashes saved
  before the browser spool write. The [evidence](../../../.orchestrate/evidence/RENDER-2D/M2/chrome-realtime-adjacent/inspection.md)
  includes the crossing, elapsed-time playback and the download-integrity control.
- 2026-11-23 M3 adds coverage filtering, linear-light box mips with per-frame gutters, and tile edge ownership
  to the one sprite pass. Both Metal recipes preserved authored SDR bytes at whole-number scales. In the retained
  scene-linear sweep, the largest colour error was 0.00298 and the largest single-texel area error was 0.01363
  output pixels. Both sampling controls had greater colour error at each fractional scale.
- 2026-11-23 M3's window proof matched the aligned card at all three Retina sizes and at the first presented
  extent after resize. Chrome matched both CSS grids before and after resize at DPR 1 and 2 and measured
  the same 0.01363-pixel weight bound. The original engine taps verified six Metal and eight Chrome motion
  takes. The [M3 evidence](../../../.orchestrate/evidence/RENDER-2D/M3/README.md) includes the full frames,
  native-size crops, playback and source submission hashes.
- 2026-11-23 M3.11 passed in the shared exact-image run on main at `d1ef5f918792`, which includes
  the M3 source at `e764c0ea7f34`. The verifier passed both Metal matrices and all four Chrome captures
  after APPLE-PRESENTATION M14 refreshed the references for the corrected spatial mip bias.
  [Coverage receipt](../../../.orchestrate/evidence/RENDER-2D/M3/exact-images-covered.json) ·
  [Verifier record](../../../.orchestrate/evidence/RENDER-2D/M3/exact-images-d1ef5f918-record.json)
