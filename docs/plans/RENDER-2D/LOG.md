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
- 2026-11-23 M3 diagnosis of [review 1](../../../.orchestrate/evidence/RENDER-2D/M3-review.md). M3.8 and
  M3.10 exposed a conflict in the design. Exact box coverage shifts the SDR contrast of the one-texel stripes
  and checker at 1.5× even with constant integrated scene-linear weight. Both targets show it. The lead
  assumes visible pulsing during an ordinary scroll is a defect at the owner's shipped-game bar. PLAN.md
  records that assumption. The atlas and camera findings describe two independent gaps in the proofs.
- 2026-11-23 M3.1 replaces the box-error qualification with a review of the bounded box-and-tent sampler.
  A CPU sweep integrated the original card's two reds through the kernels at 64 phases per axis. At 1.5×,
  the stripe's within-period SDR red standard deviation ranged from 52.7 to 93.7 codes with the box and
  from 30.5 to 36.7 with the proposed filter. The checker went from 39.8–78.0 to 12.9–13.5. This models a
  smaller swing and a softer pattern. The real-time and still inspections decide whether both are acceptable.
- 2026-11-23 M3.2 and M3.7 retire the retained Metal and Chrome weight thresholds. Both now check the
  submitted motion of a fixed marker under the same camera sweep, with a frozen-view control. The original
  Chrome coverage grid moved copies of the sprite while leaving the camera fixed. M3.7 keeps the DPR and
  resize checks.
- 2026-11-23 M3.4 now requires four contrasting neighbours on the same uploaded page at every tested mip.
  The source grid surrounded frame 4, while the packed shelf placed it against the left page edge. The outline
  and shared-edge checks still cover both sheet kinds.
- 2026-11-23 M3.8 and M3.10 now judge visible contrast stability and retained detail in the original stripes
  and checker at normal playback size and speed, with the original seams, outlines and capture matrix.
  M3.9 replaces the one-output-pixel transition limit with a fixed-view check for legible fine marks and
  clean edges. M3.3, M3.5, M3.6 and M3.11 retain their exact checks.
- 2026-11-23 Diagnosis validation. The documentation check passed. The item dry run exercised the existing
  native test commands for M3.2–M3.5. The window, browser and exact-image checks reached the dry run's
  45-second limit. The revised sampling and submitted-camera proofs are for the M3 build.
- 2026-11-23 Trimmed the plan to current milestones and open assumptions. The M1 handoff pointed to the
  existing shell and capture routes and introduced `scripts/render2d.py`. The M2 handoff used extraction
  and changed-row upload in the 3D instance field as precedents. The completed milestones now link to their
  reviews. The design carries cooker parity, ownership and proof guidance.
- 2026-11-23 M3.12 goes first and lands as a test-only checkpoint. The focused `creation-counters` check
  reproduced the stale 1×1 and 16-byte expectations. The inventory reports a 12×12 atlas with gutters and
  a 32-byte frame table after `67abaa0ab` and `909111165`. Keep the live generation checks and 3D control.
  This is the failing `long-tests` case blocking WATERS M17 and RAY-TRACING M3.
- 2026-11-23 [Design review](../../../.orchestrate/evidence/RENDER-2D/design-review.md) Must Fix 1.
  Mechanism 6 and M3.1 now specify a compact kernel near 2×. It changes off-grid edge weights and its
  support fits within the aligned texel at 2×. A CPU sweep of the card's reds at 64 phases per axis
  reproduced exact aligned bytes and showed stronger typical stripe and checker contrast than the box.
  The midpoint mix stays equal. M3.8 still judges whether the result holds up during the retained pan,
  and M3.3 states grid alignment explicitly for the same exact-image check.
- 2026-11-23 Took the timing suggestion. M3.1 points to the existing 1.5× Metal and Chrome before-and-after
  sprite-pass record so the reviewer can find the measured cost of the filter.
- 2026-11-23 Revision validation. The documentation check and the existing native sprite and window
  checks passed. The item dry run reproduced M3.12's inventory failure and stopped at the queue for
  the unchanged Chrome check. Visual acceptance of the compact kernel belongs to the M3 build.
- 2026-11-23 [Final design review](../../../.orchestrate/evidence/RENDER-2D/design-review.md) Must Fix 1.
  The compact tent still loses checker contrast near texel-edge crossings. Mechanism 6 and M3.1 now cover
  the full off-grid 2× transition with frame-local spatial sampling. The revision removes the mandatory
  compact footprint, quarter-pixel edge mix and off-grid sample-count ceiling. It allows phase-dependent
  or neighbourhood-dependent sampling within the existing gutter. M3.3 keeps the exact aligned blocks.
  M3.8 still judges the original stripes and checker in motion at normal size and speed.
- 2026-11-23 Final design review Must Fix 2. M3.12 retains its focused Metal inventory check and lands
  first as a test-only checkpoint for WATERS M17 and RAY-TRACING M3. M3.13 requires a shared
  `long-tests` pass covering the completed M3 landing on main. The final review raised no new
  suggestions. M3.1 retains the agreed before-and-after timing record.
- 2026-11-23 Final revision validation. The documentation and done-when checks passed. The M3 item
  dry run reproduced the stale inventory expectations and reused the existing Metal passes. It stopped
  in the Chrome queue. A separate M3.13 probe reached the shared check, which reported an in-flight
  run and no covering pass. The completed M3 landing and its realtime captures supply the remaining proof.
- 2026-11-23 M3.12 updates the existing creation-counters inventory for the padded 12×12
  one-frame atlas and the 32-byte frame record. The focused test passed on portable Metal and
  Apple tile, including its live allocations, graph, pipeline and submitted-frame checks against
  the 3D control. [Focused Metal run](../../../.orchestrate/evidence/RENDER-2D/M3/M3.12-generation-inventory.log)
- 2026-11-24 M3 widens the sprite filter around 1.5×, reconstructs local texel edge crossings
  near 2×, and leaves aligned texels exact. The Metal and Chrome camera proofs drive a fixed marker
  through 256 submitted poses and reject replays of its first frame. The Metal atlas upload and
  edge tests share the same four-neighbour fixture. The 1920×1080 timing record covers both Metal
  recipes and Chrome WebGPU. [Filter and timings](../../../.orchestrate/evidence/RENDER-2D/M3/M3.1-filter-review.md)
- 2026-11-24 M3 responds to the second review with a symmetric 2× crossing, luminance-guided
  weight transfers and retained coverage alpha. The submitted two-colour edge test exercises the
  former discontinuity beside transparency on both Metal recipes. The shared pixel-filter radius
  fixes the clipped coverage estimate, and the rotated outline witness now runs at all ten named
  scales over a white panel that contains its footprint.
  [Metal edge suite](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/sprite-edges.log)
- 2026-11-24 M3 keeps the 2× weights separable and narrows the projected transition to preserve the
  original checker during simultaneous crossings. The 1920×1080 realtime trial on Apple tile and
  portable Metal kept the checker square and retained the mean through the old flash phases.
  [Metal screening](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/separable-screen/README.md)
- 2026-11-24 At merged renderer `e0aeaf6be`, all six Metal realtime takes retained both stripe
  orientations, square checks, closed ground joins and clean rotating outlines. The 2× crossings
  briefly soften without the previous dark flash. The 1.5× timing record covers both Metal recipes
  and Chrome WebGPU, with GPU timestamps, overdraw estimates and shared machine load.
  [Filter and timings](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/M3.1-merged-filter-review.md) ·
  [Metal motion](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/M3.8-merged-motion-review.md)
- 2026-11-24 Both shared checks cover `e0aeaf6be`. The merged-revision milestone check passed the
  repository gate and every target command except the AppKit window rerun, which exited while
  the graphical session was locked. The earlier passing window capture supplies M3.6 evidence.
  [Exact images](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/e0aeaf6be8acad28f2c9046e789a33ca26126dc8/main-exact-images.json) ·
  [Long tests](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/e0aeaf6be8acad28f2c9046e789a33ca26126dc8/main-long-tests.json) ·
  [Milestone check](../../../.orchestrate/evidence/RENDER-2D/M3/review-fixes/separable-milestone-check.json)
- 2026-11-24 [Design review](../../../.orchestrate/evidence/RENDER-2D/design-review.md) Must Fix 1. Mechanism 4
  specifies that a frame row goes before the first world row with a greater value on the layer. It explains the
  A, B, F inversion and the zero-band tie. WP5.11 carries the rule into public guidance. WP5.14 checks overlap
  pixels with the band on and off and on a higher layer. The completed WP2 proofs stay as recorded.
- 2026-11-24 Must Fix 2. Mechanism 8 anchors cooked frames at the original-frame pivot through trim, scale,
  flip and rotation. Existing embedded frames keep their top-left anchor. WP4.3 compares the submitted
  positions for two trimmed animation frames and the default-centre and embedded controls. WP4.10 explains
  the conventions for artists.
- 2026-11-24 Must Fix 3. Mechanism 8 keeps frame progress during cooking, fetch and budgeted staging.
  WP4.8 and WP4.9 judge elapsed-time motion across the save and switch, using actual submission times and the
  run's pre-save motion. The existing atomic-switch proofs still cover the pictures. No timing budget was added.
- 2026-11-24 Must Fix 4. WP3.6 retains the actual-window card and first resized submission witness. Its
  unattended check can report observed AppKit geometry alone when the surface reports occlusion and supplies no
  drawable. The reviewer needs a complete presented record covering the landed path. Console state is metadata.
- 2026-11-24 Must Fix 5. WP5.4 and WP5.5 use a separate crowd cost command. Each take ties timings and
  overdraw to submissions of the packed Crowd replay, with the retained tick 120 and 240 digests checked at
  each size. WP2 keeps the V1 field cost command. This review included no Suggestions.
- 2026-11-24 Revision validation. The done-when schema, mixed-row ordering examples and retained Crowd
  checkpoints checked out. The item dry runs reached the unimplemented window option, sheet test and Chrome
  case, and crowd cost action. WP5.14 stopped in the GPU queue. The repository guard reproduced existing
  diagnostics in a clean checkout. Three stale links in this design were corrected.
- 2026-11-24 WP3.6 now distinguishes an occluded window geometry receipt from a presented card.
  The real Metal window matched the authored card at all three Retina sizes. The first resized submission
  matched the new viewport, sprite target and drawable by submission ID.
  [Window receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.6-window.json)
- Restored target evidence cleared by the plan revision, including the merged separable renderer's
  Metal motion and exact images. Verified new Metal stills at all three backing sizes.
  [Still review](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.9-fixed-stills.md)
- Fresh Chrome WebGPU captures cover both scales, scroll speeds and device pixel ratios. Each take
  retains 198 submitted images with the recorded timestamps. The merge command reproduced the migrated-plan
  repository-guard failure on clean main. Orchestrate 3.0.4 also refuses the WP3.13 command in an item check.
  [Chrome inspection](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.10-chrome-motion.md) ·
  [Merge result](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3-window-and-proof-merge.log) ·
  [Item refusal](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.13-orchestrate-refusal.json)
- The verified merge at `abffdf252` landed the AppKit proof and passed the repository gate. On that
  commit, the shared `long-tests` run passed with the feature-gated 2D generation inventory. A fresh
  Retina window run matched all three backing sizes and the first resize. The WebGPU sweep passed on
  retry after a Chrome shutdown error.
  [Long-tests receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.13-main-long-tests.json) ·
  [Window receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.6-merged-window.json) ·
  [Chrome receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.7-merged-chrome/sprites-crisp.json)
- Orchestrate 3.0.7 refuses the pinned WP3.13 command in the post-merge item checker. For the planner,
  an inspection of a covering shared-run receipt would preserve the proof without invoking the
  refused item command.
  [Orchestrate refusal](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.13-orchestrate-3.0.7-refusal.json)
- 2026-11-24 The authorized replan removed WP3.11 and WP3.13 as item gates. Restored the remaining
  11 builder results against their target evidence. The Metal inventory, sweep and edge tests passed.
  The real Retina window returned all three exact card sizes and the first resized submission. Chrome
  passed both DPR sweeps and exited cleanly after the check awaited WebGPU page unload.
  [Metal tests](../../../.orchestrate/evidence/RENDER-2D/WP3/continuation-2b2a2dc10/metal-submitted-tests.log) ·
  [Window receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/continuation-2b2a2dc10/WP3.6-window.json) ·
  [Chrome receipt](../../../.orchestrate/evidence/RENDER-2D/WP3/continuation-2b2a2dc10/chrome-after-fix/sprites-crisp.json)
- A fresh Metal T2 still capture exposed a 2D panic in the shared quality controls query. The frame host
  now queries and stages 3D controls only on the 3D renderer and retains the contribution path for either
  scene kind. The 2D regression test passed. Fresh stills matched the prior reviewed frames byte for byte.
  [2D regression and inventory](../../../.orchestrate/evidence/RENDER-2D/WP3/continuation-2b2a2dc10/scene-2d-quality-and-inventory.log) ·
  [Still review](../../../.orchestrate/evidence/RENDER-2D/WP3/continuation-2b2a2dc10/WP3.9-still-review.md)
- 2026-11-25 WP3.6's deadline receipt now identifies stale surface epochs skipped before acquisition
  and counts actual drawable acquisition attempts. Geometry-only evidence requires every acquisition
  to report occlusion. Device loss, configuration errors, timeouts and any other acquire result still
  fail the check. The unattended run passed all three sizes and resize on Metal.
  [Window evidence](../../../.orchestrate/evidence/RENDER-2D/WP3/WP3.6-unattended/README.md)
- 2026-11-25 WP3.8 staggers the narrow near-2× colour transition along each sprite edge and
  redistributes weight between diagonals where the two shoulders overlap. The slow Metal checker
  held red-channel contrast between 107.7 and 111.5 versus a low of 56.3 in the review take.
  Both Metal recipes passed the submitted continuity and exact alignment checks. All seven
  automated item checks passed, including the three presented Mac windows and first resize.
  [Motion](../../../.orchestrate/evidence/RENDER-2D/WP3/metal-contrast/motion.md) ·
  [Item checks](../../../.orchestrate/evidence/RENDER-2D/WP3/metal-contrast/automated-checks.json)
- 2026-11-25 WP3.7 now unconfigures each sprite canvas and waits for the WebGPU device's
  destruction before navigating away. The exact Chrome check passed both DPRs in a fresh
  checkout, recorded eight device retirements and exited with code 0.
  [Chrome shutdown evidence](../../../.orchestrate/evidence/RENDER-2D/WP3/chrome-shutdown/README.md)
- 2026-11-25 WP3.8 reconstructs the 2× colour phase from the sprite projection at the output pixel
  centre. Straight edges cross together, and the blend retains the dominant texel at an exact tie.
  The six Metal takes show straight stripes, joined ground tiles and clean rotating outlines.
  Each 2× repeat retained 570 frames. Eight isolated crossing blends held for 7.6–8.8 ms.
  All seven automated item checks passed. [Metal motion](../../../.orchestrate/evidence/RENDER-2D/WP3/tie-crossings/motion.md) ·
  [Sampling, timing and checks](../../../.orchestrate/evidence/RENDER-2D/WP3/tie-crossings/sampling.md)
- 2026-11-25 WP3.8 narrows the simultaneous colour blend near axis-aligned 2× and computes the
  phase relative to the nearest edge from the split pivot. Both Metal recipes preserve straight
  stripes and distinct checks through the submitted pans. An additional fast take contains one
  8.24 ms dip versus the repeated fading in review. Refreshed Chrome motion, fixed Metal stills
  and the real-window resize on the repair build.
  [Metal motion](../../../.orchestrate/evidence/RENDER-2D/WP3/phase-precision/motion.md) ·
  [Sampling and timings](../../../.orchestrate/evidence/RENDER-2D/WP3/phase-precision/sampling.md) ·
  [Chrome motion](../../../.orchestrate/evidence/RENDER-2D/WP3/phase-precision/chrome-motion.md)

## WP4 Cooked sprite sheets

- 2026-11-25 Built the sheet product, its validated page array and frame table, and named handles
  connected through the content lease. Authored folders use `sheet.json`, with row-major grid suffixes.
  Cooked rows preserve their original-frame pivot through trim, and the separate embedded upload keeps
  its top-left anchor. The cooker builds mips beside the atlas before encoding and shares the format's
  mip and gutter constants with the renderer.
  [Author guide](../../SPRITE-SHEETS.md)
- The cooker and hostile input checks passed on Mac. The Metal tests passed for discarded
  recordings, pivot placement, repacking, name removal and restoration, five last-good refusals,
  and device replacement. The native and Chrome realtime captures verified whole switches with
  advancing ticks and views. Chrome fetched the changed sheet once and reused the unrelated product.
  [Package checks](../../../.orchestrate/evidence/RENDER-2D/WP4/check-9615a6717.json) ·
  [Metal tests](../../../.orchestrate/evidence/RENDER-2D/WP4/metal-tests.md) ·
  [Mac motion](../../../.orchestrate/evidence/RENDER-2D/WP4/mac/inspection.md) ·
  [Chrome motion](../../../.orchestrate/evidence/RENDER-2D/WP4/chrome/inspection.md)
- The public sprite page now follows the watched sheet store and refuses 3D selections.
  The Chrome check verified the guide URL, a save on the ordinary animation loop and an independent
  T2 realtime switch, with only the changed product fetched in each run.
  [Chrome proof](../../../.orchestrate/evidence/RENDER-2D/WP4/review-repair/chrome/inspection.md)
- The public ordering time now includes the cooked world's retained-order update and the frame-row
  merge. A fresh Metal take verified both live switches and recorded ordering on changed and
  repeated source ticks. An independently authored guide example cooked, and its field configuration
  drew all three submitted art generations.
  [Metal motion and ordering](../../../.orchestrate/evidence/RENDER-2D/WP4/review-repair/mac/inspection.md) ·
  [Guide proof](../../../.orchestrate/evidence/RENDER-2D/WP4/review-repair/guide/inspection.md)

## WP5 Crowd view, simulator and creator guide

- 2026-11-25 Added the Crowd view on the shared window and browser shells. It consumes immutable
  SIM-2D body components and their checked previous poses. All retained checkpoints and timeline folds
  matched on Metal and Chrome at 2,000, 5,000 and 10,000 bodies with one sprite draw per tick.
  The independent overlap sweep selected the 0.06 band and verified all 9,014 near-pair rank changes.
- The packed release takes matched both timed checkpoints. One full-size Metal take crossed the
  120 Hz tripwire under shared load. Metal and Chrome captures retained over twenty seconds in each
  view. The violet projectile and gold hit flash follow the retained Crowd events. Crops and the
  worst geometric ordering pair accompany the inspection.
- The V1 sprite card matched the CPU texels on the iPhone 18 Pro simulator in portrait and landscape.
  Drawing resumed after rotation and backgrounding. The Metal checks cover mixed-row ordering and
  independent sprite passes in one submission. A fresh reader exercised the creator guide and the
  revised walkthrough produced a viewable submitted still.
  [Target evidence](../../../.orchestrate/evidence/RENDER-2D/WP5/README.md)
- The post-merge simulator check timed out reading the host process table after boot. A bounded
  retry preserves the ancestor lock check. Regression tests cover both recovery and a repeated
  timeout. The unchanged simulator command passed with a minimal environment and verified shutdown.
  [Simulator rerun](../../../.orchestrate/evidence/RENDER-2D/WP5/ios-admission.log)
- The creator guide now lists the import, manifest and workspace edits for a renamed copy.
  A fresh `my-crowd` built and ran on Metal. Tick-360 images with and without the guide's
  frame row differ only where the violet mark projects. The copied Wasm library generated
  the import named by its page. [Renamed game proof](../../../.orchestrate/evidence/RENDER-2D/WP5/guide-renamed/inspection.md)
- 2026-11-25 Planner: WP6 added, because `GAME-UI:WP1` ended `BLOCKED needs_dependency`
  ([handoff](../../../.orchestrate/evidence/GAME-UI/WP1-handoff.md)). Mechanism 7 promised that a value above white
  reaches an EDR display, but the EDR surface (`APPLE-PRESENTATION:WP6`) landed after WP1 and serves a 3D host
  only: a 2D frame is 8-bit codes decoded onto the float drawable, the EDR tap refuses a 2D host, and so does
  the change of mode. The planner read the code at `05437b579` and confirmed all three.
- 2026-11-25 Planner's decisions for WP6. It is built here under seam ruling 1, extended, and not as a work
  package of `APPLE-PRESENTATION`: the transfer is this design's, that module's queue holds the `long-tests` fix,
  and a changed design there would hold that fix for a design review. The float form clips at the headroom and
  has no highlight curve (an assumption in the plan). A float-mode generation holds no 8-bit display image.
  The checks select tests by `float_mode` in existing binaries; each failed for want of a test when written.
  The changed design needs a design review.
- 2026-11-25 Planner: the design review of WP6 asked for three changes, all settled in mechanism 7 and WP6's
  items. A 2D game makes the game contract's EDR declaration (WP6.7). A named 8-bit capture of a float-mode host
  is served (WP6.6, WP6.8). The clip is at the greatest half-float not above the headroom (WP6.1, WP6.3).
  [Review](../../../.orchestrate/evidence/RENDER-2D/design-review.md) ·
  [What changed](../../../.orchestrate/evidence/RENDER-2D/design-revision-2026-11-25.md)
