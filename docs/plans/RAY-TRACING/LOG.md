# RAY-TRACING log

- 2026-11-17 Owner: "Just do ray tracing and mesh shaders on mac, let's leave PC for now."
- 2026-11-17 Planned from the lead's task, quoted in [the design](DESIGN.md#the-request). Six milestones: the
  structure for entities, the whole valley with edits, hard shadows from the Sun and lamps, soft Sun shadows,
  ambient occlusion, and the paired record with promotion, which waits for `SHOWCASE` M8. Reflections by ray
  and global illumination are left for a later plan. [The planning probe](../../evidence/RAY-TRACING/planning-probe/README.md)
  showed that wgpu 30.0.1 traces correctly on this Mac from compute and fragment stages. The design review is
  owed.
