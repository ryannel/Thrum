# RAY-TRACING log

- 2026-11-17 Owner: "Just do ray tracing and mesh shaders on mac, let's leave PC for now."
- 2026-11-17 Planned from the lead's task, quoted in [the design](DESIGN.md#the-request). Six milestones: the
  structure for entities, the whole valley with edits, hard shadows from the Sun and lamps, soft Sun shadows,
  ambient occlusion, and the paired record with promotion, which waits for `SHOWCASE` M8. Reflections by ray
  and global illumination are left for a later plan. [The planning probe](../../evidence/RAY-TRACING/planning-probe/README.md)
  showed that wgpu 30.0.1 traces correctly on this Mac from compute and fragment stages. The design review is
  owed.

- 2026-11-17 Human: "Ok let's lower the bar." The most plausible result as fast as possible, not physical accuracy; references become
  diagnostics. Plan: dropped M4.1 and M5.1, which held the soft-shadow and occlusion estimators to reference fractions; their look
  is judged by inspection. The ray oracle of M1 to M3 stays, because whether a ray is blocked is a fact about geometry. Five cost
  records pass on their check alone.
- 2026-11-18 [The design review](../../evidence/RAY-TRACING/design-review.md) asked for changes: seven Must Fix,
  resolved in the design and items. (1) Promotion needs cost no higher and a gain, beside portable runs, a
  difference image and a ten-minute run (M6.1 to M6.5). (2) The window asks for `ray-query` only in a qualified
  configuration, and the portable recipe refuses ray effects (M3.6, M6.4). (3) A centre ray traced each frame
  overrides a settled history, and a soft edge's latency is a stated settle length (M4.4, M4.7, M5.5).
- 2026-11-18 The same review, continued. (4) A ray's origin is lifted 8 float steps, not 256, after
  [a second probe](../../evidence/RAY-TRACING/planning-probe/OFFSET.md) (M3.1). (5) Membership is the scene's,
  before any culling (M2.2, M2.4, M3.4). (6) Ray demand is the scene's and reaches the arena and cooked
  residency before allocation (M1.5, M2.6). (7) Pipeline creation is counted for each generation (M3.7). The
  suggestion was taken: M1.6 records a still scene and every entity moving beside the 1,500 movers. Items were
  renumbered. The design review is owed.
- 2026-11-18 Human, 17 November: "Ok let's lower the bar." Applied in full. The CPU ray reference no longer
  decides any item (old M1.2, M1.3, M2.1, M2.3, M3.1, M3.4). Tests aim rays whose answer is plain from how the
  scene was built and compare what the structure holds with the scene's records; the valley's structure is
  inspected in a traced view (M2.1). A builder may still write the reference as a diagnostic.
