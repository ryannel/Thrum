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
