# EFFECT-GRAPH log

- 2026-11-11 Planned from the owner's task, quoted in the design, and the brief in the module list. Three
  milestones: texture kinds in the graph (after `APPLE-TILE:M2`), persistent state and cadence through a diagnostic
  effect, and the stage after opaque (after `APPLE-TILE:M3` and `APPLE-PRESENTATION:M2`). The port's graph condition
  is now met at M3, and the module list and the `SEA` and `CLOUDS` briefs say so.
- 2026-11-11 Planner's decisions: effects extend the existing contribution seam, and a scene holds an ordered list
  of contributions; cadence is a runtime skip under one compile-time rule, not a recompiled graph; state commits
  only with a submitted frame. Researched: Unreal's RDG, Unity's render graph, WebGPU's limits and storage-texture
  access. The storage-texture count and view rules were not re-read from the specification; M1.3 records them.
- 2026-11-11 The heavy checks (release build, GPU, Chrome) were not dry-run by the planner, for lack of disk and
  because the GPU is shared. Each check was parsed as shell and the done-when file validated with the loop's own
  validator.
