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
- 2026-11-14 Dependency audit ([the dependency audit](../../evidence/dependency-audit-2026-11-14/README.md)). M7 no longer waits for `PLATFORM-RUNTIME-MAC:M3` and `CREATOR:M3`,
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
  The integrated exact-image run remains the lead's gate. [Evidence](../../evidence/SHOWCASE/M5.md)
- 2026-11-18 Followed up on M5.8 and M5.9 by removing optional pass timing from the exit motion
  captures, bounding Chrome upload work away from the rendering callback, and recording native
  world overload recovery. The six refreshed Chrome stops match the retained bytes. Complete
  captures still show intermittent submission gaps under shared load.
  [Motion diagnostics](../../evidence/SHOWCASE/M5-motion-diagnostics.md)
