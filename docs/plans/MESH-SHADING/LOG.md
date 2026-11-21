# MESH-SHADING log

- 2026-11-17 Owner: "Just do ray tracing and mesh shaders on mac, let's leave PC for now."
- 2026-11-17 Planned from the lead's task, quoted in [RAY-TRACING's design](../RAY-TRACING/DESIGN.md#the-request).
  Three milestones: one cooked mesh through the object and mesh stages, the valley's cooked meshes in the main
  view and the Sun's shadow maps, and the paired record with promotion, which waits for `SHOWCASE` M8. Entities
  are left until that record shows whether the stages are faster on this GPU.
  [The planning probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md) showed that wgpu 30.0.1 runs both
  stages on this Mac. The design review is owed.
- 2026-11-18 [The design review](../../../.orchestrate/evidence/MESH-SHADING/design-review.md) asked for changes: five Must Fix.
  Resolved in the design and items. (1) The default is keyed to the measured adapter, the Apple recipe and map
  shadows, and the window's own request is never refused (M3.3). (2) A material reload is one candidate for the
  indexed and the stage pipeline (M1.7; M1.5 allows its off-frame preparation). (3) Each Sun map is compared
  with the existing caster at a low Sun (M2.3). (4) Eligibility is decided for each mesh at staging and at
  publication, by storage-binding and dispatch limits (M1.6, M2.5). (5) The promotion rule names its two
  figures, the median and the spread, and the sitting precedes the items that read it (M3.1, M3.2). The
  suggestion was taken: triangles rejected, terrain apart (M2.7, M3.5). Items were renumbered.
- 2026-11-18 Human, 17 November: "Ok let's lower the bar." Applied here. The measured parity record and the item
  held to it (old M1.3, M1.4) became one contract comparison at a tolerance the test states (M1.3). The cost
  record's reviewer question was dropped (M2.7), and the final-build inspection runs only if the path is
  promoted (M3.4). Meshlet storage follows the scene's resolved path, not the device. The design review is owed.
- 2026-11-18 [The second design review](../../../.orchestrate/evidence/MESH-SHADING/design-review.md) asked for changes: four
  Must Fix. The lead: "prefer the simplest promotion rule that holds: an explicit, recorded per-device opt-in
  with a measured paired timing as a tripwire is acceptable". Nothing is promoted now: the stages are an
  opt-in, and the pair is a tripwire. M2 gained M2.8 and M3's items were rewritten.
  [What changed, finding by finding](../../../.orchestrate/evidence/MESH-SHADING/design-review-2-resolution.md). The design
  review is owed.
- 2026-11-18 [The third design review](../../../.orchestrate/evidence/MESH-SHADING/design-review.md) asked for changes: two
  Must Fix. The lead: "take the smallest fix that removes each defect: an explicit portable recipe must stay on
  the portable path; for the frame timing, either measure the whole command-buffer GPU span or label the number
  as the pass interval it is." `MeshStages` is met only on Apple tile, and the GPU figure is named the
  timed-pass span. [What changed, finding by finding](../../../.orchestrate/evidence/MESH-SHADING/design-review-3-resolution.md).
  The design review is owed.
- 2026-11-18 The M1 builder verified opt-in Apple tile forward mesh stages on Apple M3 Max Metal. The proof
  exercised admission, culling, the submitted frame contract, fallback, material installation and upload budgets.
  The shared `exact-images` check runs after integration. [Metal results and images](../../../.orchestrate/evidence/MESH-SHADING/M1.md).
- 2026-11-18 M1 review fixes passed on M3 Max Metal. The GPU gate shares registry creation with the native hosts,
  the cull compares all six planes with nonzero crossing and rejection witnesses, and a prepared secondary
  slot starts cooked compute only in the frame that requests it. [Repair evidence](../../../.orchestrate/evidence/MESH-SHADING/M1.md).
- 2026-11-18 M2 runs the valley and all three Sun maps through opt-in mesh stages on M3 Max Metal.
  The forward binding uses the existing packed view through a stage-only group to work around wgpu 30
  Metal's mixed-stage binding defect. Target proofs cover culling, shadow depth, publication and device
  replacement. The paired timing records load and the repeated CPU tripwire investigation.
  [Target results and captures](../../../.orchestrate/evidence/MESH-SHADING/M2.md).
- 2026-11-18 M2 review fixes add a grazing-Sun valley case on Apple M3 Max Metal. Production
  casters match, and the one-sided and wrong-transform controls fail. The paired cost record
  also counts resident cooked draw resources and reports recorded Sun-map draws.
  [Target results](../../../.orchestrate/evidence/MESH-SHADING/M2.md).
- 2026-11-21 Handed over by the planner that diagnosed `DEAD-CODE` M2, for this module's next planner. `.config/long-tests.txt` lists the game's `mesh_stage_gpu` as not-run with this module as its owner (`GATE-TIME` M6, at `8d3180963`). Run whole under `cargo test`, 4 to 6 of its 10 `metal::` tests fail, a different set each run: a buffer or pipeline "is invalid", or `WrongSubmissionIndex` in `support.rs` `read_buffer`. With `--test-threads=1` all pass, so the tests share state, and the binary runs in no tier. `DEAD-CODE` M2.5 and M6.5 run it one test at a time, only to show their own edits broke nothing.
  No milestone here owns the fix yet. The line moves to a feature line when the shared state has been named and fixed and the binary passes whole under `cargo test`; one pass is weak evidence, since the failing set changed each run. The builder that fixes it moves the line, and this log records the cause. A pass after `GATE-TIME` M8 is not that evidence: `./dev long-tests` keeps `cargo test`, and a pass with each test in a process of its own would only show the state is shared inside one process.
