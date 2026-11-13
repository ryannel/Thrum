# SHOWCASE log

- 2026-11-07 Before the loop, a Claude session built the local launch and the six-stop review route as milestones
  M1 to M4 under [the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md). Their plan and log are kept in
  [the route slice record](../../evidence/SHOWCASE/ROUTE-SLICE-RECORD.md).
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
