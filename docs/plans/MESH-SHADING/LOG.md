# MESH-SHADING log

- 2026-11-17 Owner: "Just do ray tracing and mesh shaders on mac, let's leave PC for now."
- 2026-11-17 Planned from the lead's task, quoted in [RAY-TRACING's design](../RAY-TRACING/DESIGN.md#the-request).
  Three milestones: one cooked mesh through the object and mesh stages, the valley's cooked meshes in the main
  view and the Sun's shadow maps, and the paired record with promotion, which waits for `SHOWCASE` M8. Entities
  are left until that record shows whether the stages are faster on this GPU.
  [The planning probe](../../evidence/RAY-TRACING/planning-probe/README.md) showed that wgpu 30.0.1 runs both
  stages on this Mac. The design review is owed.
