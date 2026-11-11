# CULLING log

- 2026-11-11 Planned from the owner's task, quoted in the design. Before the loop, a Claude session built Frustum,
  Occlusion on cubes, the bucket route in Direct and Frustum, and Sun-caster culling (7–8 November); its four
  evidence pages are in `docs/evidence/render/G3C-*.md`. Its design and plan were never committed. This design
  carries their mechanisms forward and replaces them; their remaining milestones became M1–M3 here.
- 2026-11-11 Planner's decisions: each entity has one visibility authority (instance culling for field entities,
  meshlet culling for cooked placements), recorded as an interface in the module list; G3c closes on the scenes
  that exist; the old native-recipes milestone is not carried. Researched: Haar and Aaltonen 2015, Karis 2021,
  Bevy's and Unity's occlusion culling.
- 2026-11-11 The heavy checks (release build, GPU, Chrome) were not dry-run by the planner: the planner's checkout
  had 23 GiB free against the 16 GiB the capture scripts require, and the GPU is shared. Each check was parsed as
  shell and the done-when file validated with the loop's own validator.
