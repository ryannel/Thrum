# SHOWCASE log

- 2026-11-07 Before the loop, a Claude session built the local launch and the six-stop review route as milestones
  M1 to M4 under [the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md). Their plan and log are kept in
  [the route slice record](../../../.orchestrate/evidence/SHOWCASE/ROUTE-SLICE-RECORD.md).
- 2026-11-13 Planned from the lead's task (row 13 of the module list); no words of the owner's were passed on, so
  the design quotes the brief. Milestones M5 to M8 take what the route decision listed as still owed for final G7
  and the brief in the module list, which this plan replaces.
- 2026-11-13 Planner's findings. The valley's 10,000 entities are grid cubes about 240 m under the lookout, so the
  representative scene has never been seen in the showcase; populating it is M5. The tick page carries no
  orientation, so movers only translate. `CULLING` M2's measured field predicts about 5 ms of GPU time for 10,000
  mesh entities, so M5 sizes its meshes from a sweep before making them.
- 2026-11-13 Planner's decisions. The exit scene is a third game content and the valley's source root is left
  alone, so no exact baseline moves. One exit configuration file says how each row runs, and the game reports what
  it resolved. The route check gains a no-entities control. The task named the last milestone of modules 1 to 10
  and 12 as the need; M5 and M6 wait only on `CONTENT-LOOP:M2` and `CULLING:M3`, and M7 carries the rest.
- 2026-11-13 `APPLE-PRESENTATION` M4 to M11 have no done-when entries, and a `depends_on` on a milestone that does
  not exist makes the module invalid. M7 lists `APPLE-PRESENTATION:M3` and carries M7.1, a check of the record
  files that fails until M11 exists and is Done. The plan run that migrates M4 to M11 adds
  `APPLE-PRESENTATION:M11` to M7's `depends_on`.
- 2026-11-13 Researched: Horizon Zero Dawn's density-map placement (van Muijden, GDC 2017, slides summary) and
  frame-time capture practice (NVIDIA FrameView guide). Checks: the done-when file passes the loop's validator;
  every check parses as shell; the launcher, browser-case, cargo and record-file checks were run and fail as they
  should before the work exists. The verifier and the GPU checks were not dry-run, because the GPU is shared.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
- 2026-11-14 Dependency audit ([the dependency audit](../../../.orchestrate/evidence/dependency-audit-2026-11-14/README.md)). M7 no longer waits for `PLATFORM-RUNTIME-MAC:M3` and `CREATOR:M3`,
  which it builds nothing on; M8 waits for them. M7.1 asked for every milestone of modules 1 to 10 and 12,
  which had come to include `ATMOSPHERE`'s track-2 M7 and M8, and its check read `ACCEPTED.json`, which the
  script no longer writes. M7.1 now names the five milestones M7 builds on, a new M8.12 names the eleven the
  exit rests on, and both read `RECORD.json`. Both checks were run and fail, as they should until those
  milestones are Done. The design is unchanged.
- 2026-11-14 M7 now waits for `APPLE-PRESENTATION:M11`, which has a done-when entry from today, in place of the
  stand-in `APPLE-PRESENTATION:M3`. M7.1 is unchanged. The design is unchanged.
- 2026-11-18 Built M5's exit scene with 10,000 entities, 1,500 closed-loop drifts and a content-pinned
  flythrough. Verified layout regeneration, terrain contact and clearance, replay equality, an hour of
  drift, Metal culling and the Chrome pause. Inspected both targets' submitted stills and real-time motion.
  The 48-triangle tier was the lightest measured. No measured tier met both Mac recipes' interval.
  The integrated exact-image run remains the lead's gate. [Evidence](../../../.orchestrate/evidence/SHOWCASE/M5.md)
- 2026-11-18 Followed up on M5.8 and M5.9 by removing optional pass timing from the exit motion
  captures, bounding Chrome upload work away from the rendering callback, and recording native
  world overload recovery. The six refreshed Chrome stops match the retained bytes. Complete
  captures still show intermittent submission gaps under shared load.
  [Motion diagnostics](../../../.orchestrate/evidence/SHOWCASE/M5-motion-diagnostics.md)
- 2026-11-18 Re-planned M5 for the lead's task, which answers its `needs_dependency` block. The shadow strip at committed
  ticks 233 to 234 is `ATMOSPHERE`'s, planned there as M10, and M5 now waits for `ATMOSPHERE:M10`. The lead, on the
  whole-frame gaps: "treat them as a tripwire record labelled with load, not a gate. Rewrite M5.8/M5.9 so they don't block
  on host scheduling noise." M5.8 and M5.9 are written again to judge motion tick by tick from `fixed` captures of every
  tick, and M5.11 is new: the real-time takes as a tripwire record. The design gains
  [Motion on a shared machine](DESIGN.md#motion-on-a-shared-machine). Changing two items clears M5's proof, so its
  builder claims M5.1 to M5.7 and M5.10 again from the evidence the first review passed. Not yet design-reviewed.
- 2026-11-18 Revised for [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md) of `e289340c01cb`, 4 Must Fix.
  MF1: M5.8's Mac sequence comes from the native route recorder, extended to keep windows of consecutive ticks,
  with the strip judged at phase 0.9999 too; M5.9 reads Chrome's source ticks. MF2: the exit baseline is replaced
  by a refresh that keeps identity and thresholds (M5.5), since `--save-baseline` refuses an existing file.
  MF3: M5.11's record carries a digest of the build and route inputs, which the check recomputes and `reads`
  lists. MF4: M7.9 and M7.10 may list a late frame as unresolved under load, and the new M8.13 answers it.
- 2026-11-18 Same revision, the review's Suggestions. Taken: the three-row comparison's encoding and headroom are
  named (S1); stale text is gone (M11 unplanned, M8's item range, the verifier before `land`, per-run sandbox
  approvals). Taken in part: day-cycle storage is an open question M8's builder answers with T2's storage record
  (S2), because no sweep exists to measure. The plan's step lists became notes and its stop rules moved to the
  design as [Limits for every milestone](DESIGN.md#limits-for-every-milestone). Not yet design-reviewed.
- 2026-11-18 Revised for the second round of [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md), of
  `571cd10a31d6`, 1 Must Fix. MF3: M5.11's digest and `reads` now also cover `scripts/`, whole, and `.cargo/`, so a
  change to a capture script alone (`4ffb33052`, the receiver's limit) makes the old record fail and its pass is
  not reused. Suggestion taken: M5.9 discounts only the size of the step after a deferred Chrome tick, and still
  judges a mover or prop missing or changed in those frames. Not yet design-reviewed.
- 2026-11-18 Rechecked exit layout, authoritative replay, grounded scatter, cooked-row counts and Chrome pause.
  Fixed Metal and Chrome takes showed continuous drift in the inspected windows, and the old tick-234 strip was
  absent at both phases. Refreshed the exit grids for ATMOSPHERE M10's terrain-horizon shadow correction with the
  calibrated identity and thresholds. Baseline SHA-256 changed from
  `b7ef2ce595cafc0379e326b2b71def0f3ab8992055552dd9d85e2c4dd2676d8f` to
  `e7a63b2de2a200c2f0b714f8a9920c2749518c6282cb9b4f48b9446e0e5d5854`.
  The six real-time arms are recorded under load. [Evidence](../../../.orchestrate/evidence/SHOWCASE/M5.md)
- 2026-11-18 Retook all six real-time arms from clean revision `6ad433e05` with MESH-SHADING
  and WATERS integrated. The same build reproduced all retained fixed route images and both
  sets of six stops byte for byte. Exit parity passed.
  [Evidence](../../../.orchestrate/evidence/SHOWCASE/M5.md)
- 2026-11-18 Restored the authored half tick after the Chrome fixed capture's opening hold.
  The recorder checks the submitted phase across the active route. The retake kept 1,351
  half-tick route frames with no deferrals, reproduced both sets of stops, and passed parity.
  Retook all six real-time arms on the same clean build. [Evidence](../../../.orchestrate/evidence/SHOWCASE/M5.md#chrome-fixed-motion)
- 2026-11-19 Added M9, between M6 and M7, for `ATMOSPHERE` M6's block: "Assign the shipped valley fix and authorize the
  affected APPLE-TILE exact-reference refresh." Read from the generator, not from a run: the mesh has one 20 m strip across
  the lookout's rim whose far normals turn 33 degrees to the west, which is the bright band at sunset; and at 6.1 h the east
  ridge hides the Sun from the props ([proof](../../../.orchestrate/evidence/SHOWCASE/design-proof/lookout-rim.md)). M9 resolves the rim in
  the mesh, moves the sunrise stop's hour, and refreshes every baseline the terrain moves, under
  [the ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix). No item of M5 to M8 changed. The design has not been reviewed
  since this change.
- 2026-11-19 Design review of M9 asked for four changes; all resolved, in M9 and one question of M8.8. M9.8 and M9.9
  run M6's two equality commands on the changed exit root, and M8.8 asks that the exit record cite receipts of the root
  that ships. Each diagnostic control is drawn at its stop's preset hour, the sunrise one at the revised hour. The bare
  valley's canonical check fails unless it compared its baseline. M9.6 names the refresh it expects, and the "only
  the pin" rule moved to the six route files, as M9.7. Took both suggestions: M9.4 and M9.5 hold `chrome`, and
  the motion overview says Chrome can defer a tick.
- 2026-11-19 Second design review of M9 asked for one change: M9.7 listed six route files, and M6 ships a seventh,
  `exit-fast-yaw.json`, pinned to the exit root. M9.7 now covers every route file that names an old root at M9's
  starting commit, by rule, with a check that no route file names either old root. M9.8's question about M6's
  receipts no longer assumes they are in git, since evidence moved to the working folder that day.
