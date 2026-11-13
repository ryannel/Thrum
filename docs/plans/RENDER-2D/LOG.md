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
