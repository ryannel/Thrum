# SIM-3D log

- 2026-11-27 Planned from the lead's task, after the owner's "I approve the sdk plan" took the game SDK's scope
  answer 1. Five work packages, WP1 and WP2 with items. Parry as a dependency was weighed and not chosen.
  [What was chosen and why](../../../.orchestrate/evidence/SIM-3D/planning-2026-11-27.md), with
  [the planning probe](../../../.orchestrate/evidence/SIM-3D/planning-probe.md).
- 2026-11-27 Left for a planner of the whole project: the module list still says `SIM-3D` is "to be planned",
  in its planning input and its list of assumptions, and `GAME-SDK` WP16's `depends_on` is not yet written.
  The work package that delivers the move is `SIM-3D:WP4`. Nothing in the module list's interface row needed
  to change.
- 2026-11-28 [Design review 1](../../../.orchestrate/evidence/SIM-3D/design-review-1.md) asked for changes, with
  eight Must Fix findings. The design and the items of WP1 and WP2 were revised for all eight, with new items
  WP1.10, WP1.11 and WP2.9, and the Suggestion was taken.
  [What changed for each finding](../../../.orchestrate/evidence/SIM-3D/design-revision-2026-11-28.md).
- 2026-11-28 For `GAME-SDK`'s planner, when WP16 gets its items: the rise and descent of a climbed step and a
  snap are legs of no duration; a move's contacts are what it began inside and what took part of its motion;
  a move of more than 4,096 units is refused. [The handoff](DESIGN.md#interfaces) lists them.
- 2026-11-28 Left for a planner of the whole project: the module list's interface row for `SIM-3D` names a
  contact's normal, the mover's place and the motion left, and not yet the depth and the legs, which the
  SDK's own design already asks for.
- 2026-11-28 [Design review 2](../../../.orchestrate/evidence/SIM-3D/design-review.md) asked for changes, with
  six Must Fix findings, all settled in the design and in WP1.4, WP1.10, WP1.11, WP2.3, WP2.6, WP2.8 and
  WP2.9. The Suggestion was taken. It was the last round, so the planner checked each fix against its finding.
  [What changed for each finding, and the check](../../../.orchestrate/evidence/SIM-3D/design-revision-2-2026-11-28.md).
- 2026-11-28 For `GAME-SDK`'s planner, when WP16 gets its items, in place of the entry above: no leg of a
  move's path has length and no time, so `approach_speed` is defined on a rise, a descent and a snap; a path
  box is cut back to the query domain; a ray's `depth` comes from the depth test.
  [The handoff](DESIGN.md#interfaces) lists them.
- 2026-11-28 WP1 built the pair tests on one closest-feature routine and a separating-axis depth test.
  The Mac sweep measurement selected a 1/128-unit contact tolerance and a 1/256-unit half gap.
  Hand-worked Mac cases and 12,471 retained vectors passed on the Mac (development and release),
  Chrome Wasm and the iPhone simulator (function). The changed-word control named its differing
  vector. [Item proofs and measurement](../../../.orchestrate/evidence/SIM-3D/WP1/).
- 2026-11-28 WP1 review fixes preserve rounding residuals in the domain, collinearity and ray predicates.
  The sweep fixtures now finish clear of thin targets and cover opposing full-domain motions.
  The revised Mac measurement selects a 1/64-unit contact tolerance and a 1/128-unit half gap.
  Mac development and release, Chrome Wasm and the iPhone simulator reproduce all 13,288 retained
  answers. [Review fixes and target proof](../../../.orchestrate/evidence/SIM-3D/WP1/review-fixes/README.md).
- 2026-11-28 WP1's second review added exact turned-cuboid vertex and face contacts, finite solid ray
  endpoints and short controls, and half-gap motion-length boundaries. The pair test preserves small
  closest-feature displacements and rejects witnesses that cross a separating plane. The ray and
  sweep guards compare their limits before rounding them to travel distances. The Mac release corpus was regenerated
  because these corrections change some contact words and add hand-worked cases.
  [Review fixes and target proofs](../../../.orchestrate/evidence/SIM-3D/WP1/review-2/README.md).
- 2026-11-28 Revised WP1.2–WP1.4 after the third WP1 review. The [named boundary cases](DESIGN.md#wp1-boundary-cases)
  cover tangent pairs, grazing rays, finite endpoints and turned-face normals.
  [Decision for each finding](../../../.orchestrate/evidence/SIM-3D/WP1-boundary-revision-2026-11-28.md).
