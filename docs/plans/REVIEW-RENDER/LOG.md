# REVIEW-RENDER log

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes. Every milestone waits
  for the five milestones other builders hold in `thrum-render` (`ATMOSPHERE` M9, `APPLE-TILE` M10,
  `APPLE-PRESENTATION` M3, `EFFECT-GRAPH` M3, `CULLING` M3). Decided for H09: two buffers with one writer each,
  not one record with named offsets. M24, M31 and theme T7 are parked, with reasons in the design.
- 2026-11-15 Revised after [the first design review](../../evidence/REVIEW-RENDER/design-review.md) (two Must
  Fix). Timing: the retained Metal diagnosis of 3 November outranks the code review's M40, by the lead's task, so
  queries are still resolved in a later submission after render completion; M5 removes the allocations and proves
  that a published time is valid and belongs to its frame. Lights: the constant ambient moves to the CPU's
  directional record, so a photometric scene with no atmosphere keeps its ambient light and each buffer still has
  one writer.
