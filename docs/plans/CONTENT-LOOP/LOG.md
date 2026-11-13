# CONTENT-LOOP log

- 2026-11-06 Before the loop: the content-addressed loop was built and run in the V1 game (`thrum-asset`,
  `thrum-cook`, `thrum-asset-gpu`), with bounded staging, tripwires and the Chrome client added by 8 November.
  [G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md);
  [evidence](../../evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md). The direct draw path is
  [COOKED-DRAW](../COOKED-DRAW/PLAN.md) M1.
- 2026-11-13 Planned from the lead's task, quoted in the design, and the brief in the module list. Three
  milestones: leases for other modules with the staging failure test (after `APPLE-PRESENTATION:M2`), cooked
  mesh rows with live replacement (after `CULLING:M3`), and the close of G5 in the running valley (after
  `APPLE-PRESENTATION:M3`).
- 2026-11-13 Planner's decisions. A lease offers a set only after the frame that recorded its last copy is
  submitted, so consumers need no order inside a frame. One residency uploads for every consumer. Row replacement
  is built in `CULLING`'s route by this module's builder under a seam ruling in the module list, with a fresh
  scene as its exact oracle and a history-reset frame at the switch. Sprite-sheet and font products, a cook-time
  volume atlas and an iPhone volume encoding are named as later asks and not built.
- 2026-11-13 Researched: Unreal's per-frame streaming caps and RDG external textures, Bevy's mesh allocator,
  Chrome 139's sliced-3D compressed textures. Read the Broadsides cloud asset contract at `b4d8250e6` for what
  `CLOUDS` will hold (twelve 128³ R8 volumes); nothing was lifted.
- 2026-11-13 Checks. The done-when file passed the loop's validator. Each new test and Chrome check was run by
  hand and fails as written, before the work exists. The loop's own items-only dry run waited on the `gpu` lock a
  builder held and was stopped. The verifier, the tripwire benches and the shipping-link scan are existing
  commands that pass today; they were parsed as shell and not run, because they need release builds, the shared
  GPU and Chrome.
- 2026-11-13 The design still needs its review; the planner could not start one.
