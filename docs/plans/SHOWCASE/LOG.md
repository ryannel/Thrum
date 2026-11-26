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
  design as [Limits for every milestone](DESIGN.md#limits-for-every-work-package). Not yet design-reviewed.
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
- 2026-11-19 Ran the existing exact comparisons on the populated exit. Both Metal recipes
  passed 1,351 frames and Chrome passed all six checkpoints. Two 1080p runs per Metal recipe
  favor Frustum in cover and in the open under shared load. The evidence includes default-mode
  motion on both targets. Exact baselines use the shared main check.
  [Evidence](../../../.orchestrate/evidence/SHOWCASE/M6.md)
- 2026-11-19 Diagnosed the exit Metal oracle timeout. Four prepared worlds
  used 32.8 GB in one process. The same fixed route now runs in one fresh
  process per mode, using a lossless Direct stream for byte equality and
  retaining every submitted tick and every Sun-caster timing witness.
  [Evidence](../../../.orchestrate/evidence/SHOWCASE/M6.md#metal-equality)

- 2026-11-20 M9 resolved the lookout rim with dedicated top and face normals and moved sunrise to 6.32 h. The 10,000-entity layout and seven route pins follow the changed terrain. The far crest and west ridge shadow crossing remain for ATMOSPHERE M6. [Target evidence](../../../.orchestrate/evidence/SHOWCASE/M9/README.md).
  Route baseline SHA-256 before → after, recalibrated for the new cooked roots and 6.32 h preset. Bare valley `04f30aa9b1e8750c661152a99aebac9d1597336f57612bbc0fcacbe614e9fe30` → `a169c78c3f0ebee0fb2f3d42d0e57de701fed5645a53dbfb4ff7d184943e05ee` at thresholds `[1.0, 1.0, 1.0]`. Exit scene `e7a63b2de2a200c2f0b714f8a9920c2749518c6282cb9b4f48b9446e0e5d5854` → `06d7a29f867865d2680addd79095ef9e66003b3e689809410d1cc48df2722b38` at thresholds `[1.0, 1.0, 1.0]`.
  The resolved lookout rim changes the terrain drawn by the registered sets. All five registered sets moved (apple-tile-native, chrome-day-sdr, chrome-day-scene-linear, chrome-night-sdr, chrome-night-scene-linear).
  SHA-256 before → after, all native paths relative to `examples/v1-game/tests/fixtures/exact-images/native/` and Chrome names from the registry.
  `sdr-1/approach.png` `ac493b87a796e9b173c43ec07b9e70270fccda015758cbc7a677103f70c649e2` → `1946ee6a2ac3a879850c92c0a439fbf830dd18bfdf3f2ba56c5859aef2aa9506` | `sdr-1/approach.surface-bgra8.gz` `566319b358ea4c640c6ea8718c062d5713b5c9a9469309629841d83b7fe05acf` → `0946edc7a5c34c3c38330c546b0438dec699cdc199748f96b2a1f788b98d9a9d` | `sdr-1/mirror-light.png` `f7275907cc93742fd9c07485c3dd4b8ed00e28cf1e3d000c167cabce183ac053` → `9a3f08d8fc169349b1f320f22ea4fff8ee388e2ae1acc205ec5ed2ebffd89199` | `sdr-1/mirror-light.surface-bgra8.gz` `2a800c46df666dfd1164386ca4b74e0d29644b339584fddac917ef838b8aa7e7` → `2a2bef0deb3be8bdcd41b7248b3534e037bb19377b8b89d0ef49e35892b0feb6` | `sdr-1/near-plane.png` `fb029c70df1447968575eb241ea39fe5688f54d94a379d9efd19435233ffb463` → `488266a2532964b67d7d1bf469876d20c49bba7345ec46a1fb7a251f4e57e8ed` | `sdr-1/near-plane.surface-bgra8.gz` `0234b7e577e1aaa1bddeec1475a45830b3c535aaf0662f6d66021e07b3914202` → `f451a7e8e8b9db7f54c21cae8c6f52861ac3841b9c9a3469e44dcd91176facb8` | `sdr-1/post-cut.png` `b20f6a5da46909a84ae6a49856965f424e108aebf95d52de3b6cfe09095bd242` → `956a46f9808acb3a458c11a17c795860047d0fe9f794ff43d06edfc7341d7062` | `sdr-1/post-cut.surface-bgra8.gz` `f42a2f567a4a19cf6090b5840f327da4cd53e467760eaf36d49b822e602e843b` → `6d2992b56f96ac0a188da80c446f2bcf37f9d9cad4a6dafe7e53919a8d61159f` | `sdr-1/pre-cut.png` `db59e7f5a99d9062bc7487f56306995aac1c08d4291d7d0365057b5a348f9fdd` → `8d591edd6eda944dd32b45ce4669200de77b51a3491b2afc2686ad78f79d82fc` | `sdr-1/pre-cut.surface-bgra8.gz` `10d4d20e6170a8c2d9e51bc788e753612cf221a39920971635c0033e8fbe2245` → `59eb99f2d3e91fbc9fabfed2e0c404632b4896e9dfc01e5053ff0147e2229278`
  `scene-linear-1/approach.rgba16f.gz` `5c079907b69b5e9637853a5a449b08a6d4ad23986d007718ce9a7c8d40f2b799` → `ed1453aefb0aace9e0ca0b026455ce8377db9c63dea0e803c774f830dfe133fd` | `scene-linear-1/approach.surface-bgra8.gz` `566319b358ea4c640c6ea8718c062d5713b5c9a9469309629841d83b7fe05acf` → `0946edc7a5c34c3c38330c546b0438dec699cdc199748f96b2a1f788b98d9a9d` | `scene-linear-1/mirror-light.rgba16f.gz` `42070fe46e4a1312847c09b70c2c8c388d0f752264e7c64546608c67c41376cf` → `fe6e2e48d8ecbe6e9d03c3dc2cd51ea2823721aff798f962e9272c9a6f51427f` | `scene-linear-1/mirror-light.surface-bgra8.gz` `2a800c46df666dfd1164386ca4b74e0d29644b339584fddac917ef838b8aa7e7` → `2a2bef0deb3be8bdcd41b7248b3534e037bb19377b8b89d0ef49e35892b0feb6` | `scene-linear-1/near-plane.rgba16f.gz` `6faef9402ca53c537fd4a657fe72051680c5633fd48780efb4f6a54024714544` → `6e8c044b54fc3c254122f56ebf22203559e330c85ff530a8e5cd765df08911a9` | `scene-linear-1/near-plane.surface-bgra8.gz` `0234b7e577e1aaa1bddeec1475a45830b3c535aaf0662f6d66021e07b3914202` → `f451a7e8e8b9db7f54c21cae8c6f52861ac3841b9c9a3469e44dcd91176facb8` | `scene-linear-1/post-cut.rgba16f.gz` `abc53a7e084df3834095c085dd5ac3ee87672e123dbe90901eba5ad9a1cb1b5d` → `f0caf5013bd2ca2e22b292cab16040171feea1465561245da308f4dce6fee43d` | `scene-linear-1/post-cut.surface-bgra8.gz` `f42a2f567a4a19cf6090b5840f327da4cd53e467760eaf36d49b822e602e843b` → `6d2992b56f96ac0a188da80c446f2bcf37f9d9cad4a6dafe7e53919a8d61159f` | `scene-linear-1/pre-cut.rgba16f.gz` `b264ccc9d43e92319e2dcfdea4969fc93238012f99cd93d869b1c745fcf8cc4b` → `29f987dec401a89254cb6233c85db2b0c2eb427f239a91ee7e47cfb6514615d6` | `scene-linear-1/pre-cut.surface-bgra8.gz` `10d4d20e6170a8c2d9e51bc788e753612cf221a39920971635c0033e8fbe2245` → `59eb99f2d3e91fbc9fabfed2e0c404632b4896e9dfc01e5053ff0147e2229278`
  `letterbox-bgra8-1/mirror-light-present.png` `e13fda011faecdef92268782f2ed41a8d4f3f2567a99e135df19950e7b888887` → `9418a8a0b8d942d17670a291b52d0563dbc447bccfb8cc9d699c545c56b531ca` | `letterbox-bgra8-1/mirror-light.surface-bgra8.gz` `7068d9f1d27c5ba9b216b06d06ecd4814d619049ec53b23494093e3fbfd66a16` → `1b7849615dd22c2a16782df833a689e6dbc954728950c8a3df04170c7855256a` | `letterbox-bgra8-srgb-1/mirror-light-present.png` `e13fda011faecdef92268782f2ed41a8d4f3f2567a99e135df19950e7b888887` → `9418a8a0b8d942d17670a291b52d0563dbc447bccfb8cc9d699c545c56b531ca` | `letterbox-bgra8-srgb-1/mirror-light.surface-bgra8.gz` `7068d9f1d27c5ba9b216b06d06ecd4814d619049ec53b23494093e3fbfd66a16` → `1b7849615dd22c2a16782df833a689e6dbc954728950c8a3df04170c7855256a` | `sun-sdr/checkpoint-0.png` `b5d35fcc175a3ed91820dfbbed32462e7822efca57d2f59df4a7552876e86d73` → `37951182b40582a636fba0db65d1b0a81ac169c38a4cfd91d73331b0153c6d75` | `sun-linear/checkpoint-0.rgba16f.gz` `52ef00a13e3f3d8512505394066a70e017e5105e013a3e8902b0176b191522dc` → `7e4d9e816d32e4c19221235e491222477817e69a70f451278a477f0adfdbb31b` | `night-sdr/checkpoint-0.png` `3b3db86c2447c7b40d76e4346b95bec6d11f4719d81098c874d5ebe5a6f328a4` → `c918b1ce46739396c676484e89ba846276d391d063bf9d6ff6714bc7978ef63c` | `night-linear/checkpoint-0.rgba16f.gz` `6d9adbde29658767039dae0b9103244b2976c30fc01eaf0ba42c8413100f32c2` → `c170503a4e75f023373f0d3747807703015301ba2a4e6a8bd7ccd8a68549f64d` | `night-sdr-off/checkpoint-0.png` `1a6354f4bb21f8172c5c2ea8d30e06ac58cc4d1f84a73e2865d4db6b0ae07cd6` → `310d19d3d7f228722f78634b0803ee5cadd095eeefa33f13774e63ab6cb65b71` | `night-linear-off/checkpoint-0.rgba16f.gz` `3cc0140363e49d21af9884a7bc7a1102d7ad58d8cdea89338576d2874d9ae28b` → `9f25daa08b386cb7307e35143d8ff87dfbb2f121bd2ca7b92f28f58424858dfb`
  `chrome-day-sdr` `242fa83484a8172c7fc3a750066375ef9d2adeb711e479f3d840bc9322796cda` → `19ea9419d8ebf692647702a3d76f4cbe928edbc65446ff8b1d9fc414578aaccc` | `chrome-day-scene-linear` `f8c372012c07f4d9317115308596e446ac2c1aa0684d1d1759943a958217e128` → `e14c7756bc41b6108485ea3e3d05337eb3c14a9d5b59711326e4ad94e55cd8b7` | `chrome-night-sdr` `ac51d478df8d32137a3dc18ec03eff339e6b51e581d5222eb0d4be5f6e357acb` → `ed49185b614138d97531a4d103fd4a7a3aee05087c56b3744c706c17c4100557` | `chrome-night-scene-linear` `f128068eb114b63ae9c5f89fbb6068b46f5176dee5cd87fe3d5c1415dd94404c` → `94f57fabf1be209376074776ca1cfe9db1254109ae0761382aa484c275c01c0a`

- 2026-11-20 The M9 landing moves the exact references and route and motion check inputs out of the former evidence folder under the lead's relocation ruling. The fixture audit records identical bytes across the move. The registry paths and the native comparison driver's input path follow the files. [Fixture audit](../../../.orchestrate/evidence/SHOWCASE/M9/fixture-relocation.json).

- 2026-11-20 The first post-rebase exit check recorded a 1252 × 704 Chrome backing after responsive extent support landed. The route-check pages hold the original 1280 × 720 backing and viewport used for calibration. [Resize evidence](../../../.orchestrate/evidence/SHOWCASE/M9/exit-responsive-backing-failure.json).

- 2026-11-20 Diagnosed M9 after [its review](../../../.orchestrate/evidence/SHOWCASE/M9-review.md) failed M9.6
  alone. The plan was at fault. M9.6 let the registry change only in `pin` and `refresh` and froze every
  comparison file, and was written before the lead ruled that the references leave `docs/evidence/`. The move
  kept every file's name and bytes and changed only the paths that say where the references are.
  [The module list](../ARCHITECTURE.md#ruling-on-relocated-references) now rules on a relocation, M9.6 is
  written again from its claim, and the design says which paths follow the files. No other item changed.
- 2026-11-20 Planner, from the cleanup review (L8-05): M9.7's check passed when the routes folder was missing. It now fails unless a game's route files exist, and searches them wherever the folder is. The statement and the ask are unchanged. M9 was Not Started.

- 2026-11-20 Reclaimed M9 against the amended M9.6. The Git audit confirmed all 55 reference blobs survived relocation byte-for-byte. Both logs record the 34 regenerated files, and the shared Metal and Chrome verifier passed against the same fixture hashes. [Amended M9.6 proof](../../../.orchestrate/evidence/SHOWCASE/M9/amended/M9.6.md)

- 2026-11-26 Planner, from [`APPLE-PRESENTATION`'s design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md)
  (Must Fix 1: the exit could accept WP11 as the last Apple presentation prerequisite). `APPLE-PRESENTATION` WP17,
  the display's full 120 Hz at full screen, is now among the work packages the exit rests on: WP8 depends on it,
  and WP8.12's statement and check list it. WP7 still needs only WP11. The checks of WP7.1 and WP8.12 named
  work packages by ids that no longer exist (`M12`, `M11`, `M3` and others), so neither could pass; they now use
  the `WP` ids. WP7.1's statement is unchanged. WP7 and WP8 were Not Started.
  [What the exit waits for](DESIGN.md#the-exit-record)
- 2026-11-26 Planner, from [a design review](../../../.orchestrate/evidence/SHOWCASE/design-review-8.md) (Must
  Fix 1: WP17 at full screen on the bare valley and WP8.6 on the populated exit in a window could both pass).
  WP8.6 is written again: the Apple recipe row holds the full refresh at full screen over the populated
  flythrough in two takes, with the resolved cooked root, drawable extent, requested rate and presented
  intervals. New WP8.14 keeps the portable flythrough and windowed startup. Took the suggestion on the
  exact-baseline paragraph. WP8 was Not Started.
  [The design section](DESIGN.md#the-full-refresh-at-full-screen-on-the-populated-exit)
- 2026-11-26 Planner, from [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md) (Must
  Fix 1: WP8.6 wanted an exception for a one-refresh interval a fraction of a nanosecond over 1/120 s). WP8.6
  and the design now judge by the qualified witness's classification: only a missed refresh, an interval it
  rounds to two or more refresh periods, needs its place, its evidence and no recurrence in the other take.
  Raw intervals stay in the record for diagnosis. WP8.14's item is unchanged. WP8 was Not Started.
  [The rule](DESIGN.md#the-full-refresh-at-full-screen-on-the-populated-exit)

- 2026-11-26 Planner, unblocking WP7 ([handoff](../../../.orchestrate/evidence/SHOWCASE/WP7-handoff.md)).
  The cooked root: `CONTENT-LOOP` WP6, new, reports the drawn publication's manifest hash; WP7 depends on it
  and WP7.1 lists it. The comparison: on the lead's ruling, WP7.5 is written again to hold the rows to the same
  scene and a like look, with per-stop thresholds, an uncalibrated median, a baseline per row and an
  inspection. The builder's proposal to correct MetalFX against TAAU, on `build/showcase`, is not taken up.
  [The cooked root](DESIGN.md#the-exit-configuration-one-file-three-rows-reported-and-not-echoed) ·
  [the rule](DESIGN.md#what-the-three-rows-are-held-to-wp75)
- 2026-11-26 Planner, from [the design review](../../../.orchestrate/evidence/SHOWCASE/design-review.md) (Must
  Fix 1: WP7.5's inspection asked for the Sun or Moon, the sphere and the rock at every stop, and passed a
  shadow absent from all three rows). WP7.5's inspection and statement are written again, stop by stop, from
  the route's cues, and a cue missing from all three rows fails. Took the suggestion: the median is credited
  with broad changes only. Thresholds, controls and the check are unchanged. WP7 was Not Started.
  [What each stop must show](DESIGN.md#what-each-stop-must-show)
- 2026-11-26 WP7 added the exit launch selections, preset and live-content checks, submitted-frame comparison,
  and capture-off frame records. The three-row comparison exceeded the retained thresholds. Applying the fixed
  formula to the measured pair differences would let the night diagnostic through. The existing baseline stays
  in force. [Comparison receipt](../../../.orchestrate/evidence/SHOWCASE/WP7/compare/run.json)
- 2026-11-26 WP7 proposal to `CONTENT-LOOP`. Expose the committed cooked publication's manifest hash with
  the generation already observed by the game. The resident set owns the validated `Head`, but `CookedStats`
  exposes only its drawn generation. WP7.2 to WP7.4 need this observation to distinguish two stores that both
  report generation one. A fetched head, requested root or route pin can precede actual draw admission.
- 2026-11-26 WP7 verified six presets on all three rows, visible live edits on Chrome, live publication swaps
  and stable pipeline counts on both Mac rows, and the capture-off timing records. Chrome's exit page now
  calls the web shell's profiler before each frame. Both Mac profiler proofs use 60 Hz for numeric world-tick
  rows. [WP7 evidence](../../../.orchestrate/evidence/SHOWCASE/WP7/)
- 2026-11-26 WP7 proposal to `APPLE-PRESENTATION` and the browser presentation owner. The three-row fixed
  threshold is 1.0. The Apple row and the portable Mac row differ by 5.48 at twilight in the largest grid cell.
  Holding the Apple recipe, EDR at headroom 1 and the scene fixed, changing TAAU to MetalFX produced a 5.77
  maximum there. Portable Mac and Chrome still exceed 1.0 at several stops after the resolved selections
  are verified. Trace provider and frame-history differences at those stops. The fixed calibration rule
  applied to the measured drift would admit the night diagnostic at its required separation.
  [Comparison](../../../.orchestrate/evidence/SHOWCASE/WP7/compare/run.json)
  [Selection isolation](../../../.orchestrate/evidence/SHOWCASE/WP7/compare-isolation/run.json)
- 2026-11-26 WP7 proposal to `PLATFORM-RUNTIME-MAC` and `APPLE-TILE`. Inspect submit/present and scene
  encoding where the render owner spans the missed target. The position and neighboring samples of each
  miss are retained for WP8.13. The portable sunrise tripwire found higher costs with TAAU, including tone
  encode and surface present. The remaining offscreen p95 gap and browser callback gaps are recorded under
  load for WP8.13. [Frame investigation](../../../.orchestrate/evidence/SHOWCASE/WP7/frame-investigation.md)
- 2026-11-26 WP7 added `exit --row` and ran all three rows through the six submitted stops and flythrough
  tick 1381. Each verified its resolved render selections and failed on the missing drawn cooked root.
  The portable row reached the final tick on retry after the first attempt missed the window's present count.
  [Row receipts](../../../.orchestrate/evidence/SHOWCASE/WP7/row-launches/)
- 2026-11-26 WP7 verified the six stops and complete flythrough on both Mac recipes and Chrome against
  the resolved selections and committed cooked manifest hash. The controls rejected all 13 selection omissions
  and the Apple launch check rejected a real MetalFX-to-TAAU fallback. The native launcher waits for the route
  and its timing samples, and applies every preset to the paused window through the watcher.
  [Target runs and controls](../../../.orchestrate/evidence/SHOWCASE/WP7/verified/README.md)
- 2026-11-26 WP7 applied the revised per-stop calibration. The 95th percentile / maximum bounds in route order
  are 2.596 / 6.509, 2.254 / 4.366, 2.879 / 7.692, 2.827 / 10.298, 3.317 / 16.436 and 1.000 / 1.838.
  The median is 1.0 at every stop. The exit baseline SHA-256 moved from
  `06d7a29f867865d2680addd79095ef9e66003b3e689809410d1cc48df2722b38` to
  `be3716f268f45602f09662f026edf76682b77f08bbaa31df228f9f78aad620f5`.
  [Calibration, values and stop inspection](../../../.orchestrate/evidence/SHOWCASE/WP7/calibration/inspection.md)
- 2026-11-26 WP7 verified capture-off flythrough and day-cycle records on each row. The latest Mac records
  keep the missed opportunities and named stages that crossed deadlines. Attribution between engine work
  and scheduling remains open for WP8.13. [Timing evidence](../../../.orchestrate/evidence/SHOWCASE/WP7/verified/timing-investigation.md)
- 2026-11-27 WP7 review repair. The Chrome sizing sweep retains report timing demand and supplied 500 usable rows
  with the panel profiler off. The interactive profiler still produced live readings. The noon comparison retints
  the terrain texture, and the fixed thresholds passed. A scratch `exit --compare --refresh-baseline CAUSE
  --refresh-row mac-apple --baseline PATH` refreshed only the stale noon grid and retained all thresholds.
  [Review repair evidence](../../../.orchestrate/evidence/SHOWCASE/WP7/review-fixes/README.md)
- 2026-11-27 WP7 timing repair. The Mac reports each missed deadline's full host present-call gap and all
  observed render-owner work spanning the deadline. Chrome includes long intervals across route boundaries
  and identifies page work that accounts for the delay. The real Chrome control caught both timed holds.
  Timing still unresolved under load is recorded for WP8.13.
  [Timing investigation](../../../.orchestrate/evidence/SHOWCASE/WP7/review-fixes/timing-investigation.md)
