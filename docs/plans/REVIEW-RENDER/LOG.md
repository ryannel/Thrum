# REVIEW-RENDER log

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes. Every milestone waits
  for the five milestones other builders hold in `thrum-render` (`ATMOSPHERE` M9, `APPLE-TILE` M10,
  `APPLE-PRESENTATION` M3, `EFFECT-GRAPH` M3, `CULLING` M3). Decided for H09: two buffers with one writer each,
  not one record with named offsets. M24, M31 and theme T7 are parked, with reasons in the design.
- 2026-11-15 Revised after [the first design review](../../../.orchestrate/evidence/REVIEW-RENDER/design-review.md) (two Must
  Fix). Timing: the retained Metal diagnosis of 3 November outranks the code review's M40, by the lead's task, so
  queries are still resolved in a later submission after render completion; M5 removes the allocations and proves
  that a published time is valid and belongs to its frame. Lights: the constant ambient moves to the CPU's
  directional record, so a photometric scene with no atmosphere keeps its ambient light and each buffer still has
  one writer.
- 2026-11-18 Planner, from the owner's 17 November decision to leave dynamic GI until after V1 if nothing forces
  rework: M1 gains M1.5. `ambient_irradiance` and `indirect_radiance` take the lit point's world position and
  pixel, in the landing that already rewrites the first, with no value changed (mechanism 1 in the design; C08;
  [the decision](../../design/decisions/POST-V1-FOUNDATIONS.md)). The atmosphere's lent sky products keep their
  signatures. M1.1 to M1.4 are unchanged. The design review is owed.
- 2026-11-18 Planner, after [the design review](../../../.orchestrate/evidence/REVIEW-RENDER/design-review.md) of M1.5 (one Must
  Fix): the cooked volume is a medium, so it reads the ambient source and is not a caller of the surface query. The
  design had it pass the world position of its fragment, which is the box's back face and none of the march
  samples it lights. Mechanism 1 now splits `ambient_irradiance(n)` into `ambient_source(direction)`, the source
  read C08 gives a medium, and the surface query by position and pixel; the volume keeps its one uniform-sky read
  and its pixels. M1.5 is rewritten to match, and module-list ruling 4 and the decision say the same. Suggestion
  taken: the plan no longer tells builders to run the verifier before landing, since exact images are proved once
  on main. The design review is owed.
- 2026-11-18 Built M1. The CPU and atmosphere now each write a whole light record, and surface encode
  faults reach the drive and completed frame. Surface lighting takes both positions and the cooked
  volume reads the ambient source once. Verified the three pre-split pixel fixtures byte for byte,
  exercised all three surface fault kinds, and ran the cooked game draw on Metal. The registered
  baselines use the shared main-branch proof. [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M1.md).
- 2026-11-19 Addressed the M1 review findings. Sprite recording now leaves surface timing admission
  to the presenter. Shade device limits reach the V1 constructor as GPU errors. Restored the
  owning modules' source fingerprints and graph fixtures and projected only the light split in
  their comparisons. Verified the fault paths and historical controls on Metal.
  [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M1.md).
- 2026-11-19 Preserved typed attachment errors across the ordered contribution list.
  On the restricted Metal device, direct and grouped Shade both returned the Weather
  map device limit as a GPU error. Secondary-view regressions passed.
  [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M1/grouped-shade.md).
- 2026-11-19 Built M2. The renderer reflects its production WGSL against 31 host records and gives each shared
  declaration one source file. The scene, visibility and atmosphere write typed records, and the build generates
  shared shader constants from Rust. The Mac tests rejected a type change and a copied declaration and exercised
  the changed renderer paths on Metal. Registered images use the shared main-branch proof.
  [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M2/README.md).
- 2026-11-20 Addressed the M2 review. Cluster vacancies use whole records, and allocations and binding spans
  use record sizes. The generated WGSL covers dispatch dimensions, display and exposure selectors, and the Sun
  disc tangent. The frozen SDR shader is byte-identical to the pre-M2 file and compiles independently.
  Verified the scoped renderer tests on Mac, including the retained SDR pixels and scene source fingerprints.
  [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M2/README.md).
- 2026-11-20 Addressed the second M2 review. The Earth profile supplies both density paths. Rust owns the night
  chart limit, Rec.709 weights, exposure thresholds, sRGB transfer values, FP16 limit and visibility word width.
  On Metal the transmittance, night, exposure and SDR tests passed, including the retained display and scene
  source comparisons. [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M2/README.md#m24-lighting-and-display-parameters).
- 2026-11-20 Addressed the third M2 review. The sky LUT shader derives its half sizes and zenith
  row from the generated Rust texture height. The production shader regression passed on Mac.
  A source probe at height 110 derived row 109, and the shipping height still derives row 107.
  [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M2/README.md#m24-sky-lut-zenith-row).
- 2026-11-21 Built M3. Every renderer pass is declared by a typed `PassKey` that supplies its name, timing
  bucket, pipeline role and recorder. Timing buckets sum by category, and an atmosphere stage's bind group and graph
  reads come from one chart table. Verified on Metal: 336 compiled graphs pass `pass_keys`, `timing_categories` and
  `atmosphere_stage_table`, and the full thrum-render suite passed (470 tests). A scratch key failed to compile, and
  a keyless pass failed `pass_keys`. Found and fixed: a secondary view's aerial pass did not declare its
  multiple-scattering read. The graph goldens passed unchanged. Evidence: the M3 items in DONE.json.
- 2026-11-21 Addressed the M3 review. The frame host no longer compiles a graph to count timestamp slots: a
  3D slot reads its target's compiled graphs, and timing recovery reuses the slot's capacity. On Metal a stored read
  took 3 ns, against 47 µs per compile. `pass_keys` checks that the target's count equals the compiled count. The 2D
  binder now lists every key. Verified: the three M3 checks and `scene_2d` passed on Metal.
- 2026-11-23 Built M4. Renderer and effect graph failures carry typed causes through the frame host.
  The instance uploader distinguishes invalid transforms from lighting errors, and scene admission errors
  identify the limits they checked. Verified 547 tests on Metal with the GPU required, plus the separate
  public error contract test and string-error check on the M3 Max. The registered image comparison is
  assigned to the shared main-branch run. [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M4/README.md).
- 2026-11-23 Addressed Must Fix 1 of the M6 design review. M6.1 selects the stage registry cases by their
  shared prefix and fails on an empty selection. The design names the CPU and prepared-capacity coverage, and
  the item asks the reviewer to confirm that every replacement case ran.
  [Design review](../../../.orchestrate/evidence/REVIEW-RENDER/design-review.md).
- 2026-11-23 Built M6. The CPU checks all fourteen stage keys, and one prepared Metal scene verifies chart
  reads, roles, timing buckets and timestamp capacity for SDR and scene-linear targets. Both cases passed
  the focused run on the first attempt in 0.010 s and 0.354 s, down from the original test's 9.200 s.
  They passed alone in 0.008 s and 0.039 s. [Evidence](../../../.orchestrate/evidence/REVIEW-RENDER/M6/README.md).
