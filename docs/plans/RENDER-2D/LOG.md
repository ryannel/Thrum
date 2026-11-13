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
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/RENDER-2D/planning-probe.md). Coverage
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
- 2026-11-13 [Design review](../../evidence/RENDER-2D/design-review.md): changes requested, two Must Fix findings
  and two Suggestions. The planner resolved all four in the entries below.
- 2026-11-13 Must Fix `sort-band-does-not-prevent-reversals`. The band held around each row's own sort value is
  gone: two rows that travel together use it up and then swap freely, as the review's trace shows. The order is
  now one list kept from tick to tick, in which two near rows exchange only when one has passed the other by more
  than the band. The design states what that guarantees, with the argument, and what it does not: rows that are
  not neighbours in the list can be further out of order through a chain, and no total order avoids that without
  exchanging a pair that has not passed. M2.2 now claims the exact order alone, M2.13 is new and claims the rule,
  M5.3 measures the crowd and no longer rests the guarantee on a trace, and M5.6 also asks that a clearly lower
  body is in front.
- 2026-11-13 Planner's probe 4, in [the evidence folder](../../evidence/RENDER-2D/planning-probe.md#4-does-a-kept-order-with-a-band-stop-reversals-and-what-does-it-cost).
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
