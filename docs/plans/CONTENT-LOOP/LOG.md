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
- 2026-11-13 [Design review](../../evidence/CONTENT-LOOP/design-review.md) of `8d09e6b4`: changes requested, two
  Must Fix findings and three suggestions. Both findings were checked against the code and are right.
- 2026-11-13 MF1 resolved by narrowing the claim, not by building source admission. M3.7 promised that a "partial
  change set keeps the previous set"; the watcher and cooker cannot know where an external save ends, and the G5
  decision already records that. The design now says what a publication promises and what it does not; M3.7
  claims presentation-only reading and that an incomplete save converges on a fresh cook of the finished sources,
  with a new `thrum-cook` test. M3.8 and M3.9 name the limit. C05 is unchanged; the reading is an assumption for
  the owner.
- 2026-11-13 MF2 resolved by matching the proof to the host. M2.3 asked for a frame encoded before a row switch
  and submitted after it, which the exclusive encoded-frame token makes impossible. The item now proves a
  discarded switch frame and a newer edit superseding an unswitched replacement, and asks that every write and
  switch go through the host. The design drops "the case the builder must fence".
- 2026-11-13 Suggestions, all three taken. S1: the lease's change signal became a serial the consumer compares
  and commits in `submitted`, so a discarded or capture frame cannot lose a change (design rule 5, M1.3, M1.7,
  M1.8, the module list's lease line). S2: M3.3 says a stall this module causes at the claimed sizes fails; the
  larger-size figure is a record. S3: the design says each reader switches whole in a frame of its own, with the
  coupled cases named and a second assumption for the owner.
- 2026-11-13 The changed design needs a second review; the planner could not start one.
- 2026-11-13 [Second design review](../../evidence/CONTENT-LOOP/design-review.md) of `a0620453`: changes
  requested, one Must Fix finding (MF3) and one suggestion (S4). MF1, MF2 and S1 to S3 were found resolved. MF3
  is right: the revision let placements, leases and rows switch in frames of their own and deferred C05's one
  switch to source admission.
- 2026-11-13 MF3 resolved by meeting C05 as written, with no contract change and no owner decision. Residency now
  owns one publication per generation. Each reader prepares under its own bound without showing; the publication
  is ready when the slowest reader is; one frame then switches placements, leases and rows together, tentatively,
  and commits for all at submit. A ready publication is not superseded, which fixes the frame's generation
  before any reader records and so needs no order between contributions. A publication one reader cannot take is
  refused for every reader. Cost: one frame of latency for placements beside a lease, and the row write's frames
  for a mesh entities draw. A scene with only placements is unchanged, so no retained image moves.
- 2026-11-13 Items changed for MF3. New coupled-reader proofs M1.10 (a texture on a placement and in a lease) and
  M2.9 (a mesh placed and as a row, with a write of several frames and an arena refusal). M1.2 reworded for the
  shared switch frame; M1.5 refuses a publication for placements and leases alike. M2.3 was rewritten because a
  ready replacement can no longer be superseded: it now proves a newer edit behind a discarded switch frame, and
  supersession while the replacement is still being written. M3.1 to M3.4 run three edits, two of them to a
  product read through two paths, and M3.1 and M3.3 assert one generation per frame across readers.
- 2026-11-13 S4 taken: the design says which layer keeps the last good content, the manifest for a fault the
  cooker finds and residency's resident generation for one the running game finds. Also fixed while there: a
  lease's CPU reader takes the product the lease names, not `HEAD`; a row switch committed by a capture frame
  keeps its history reset for the next frame that advances history.
- 2026-11-13 Module list: a content-publication interface was added, the lease and entity-visibility lines and
  the M2 seam ruling now name the shared switch frame. `CULLING`'s items do not change.
- 2026-11-13 Researched for MF3: Unreal's reimport recreates the render state of every component using a mesh
  under one scope; Bevy extracts changed assets at one point per frame and defers what exceeds a byte budget.
  Checks: the done-when file passed the loop's validator, `./dev check-docs` passed, and the two test commands
  the new items use fail as written. No number was added to any item.
- 2026-11-13 The changed design needs a third review; the planner could not start one.
