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
- 2026-11-11 [Design review](../../evidence/CULLING/design-review.md): changes requested, one Must Fix. MF1: the
  live mode switch relied on host republication, which has no Chrome entry point, takes no requested mode, and
  rebuilds the cooked contribution's pipelines at publication, where a failure detaches the cooked scene.
- 2026-11-11 Design revised. MF1: the live switch is removed, with M2.5 and M2.6 and cue C4. The brief and gate G3c
  never asked for it, and a working one is a transaction across the frame runtime, the browser host and the
  contribution seam. The design records the mechanism to cost if the owner wants it (a prepared set of modes in
  one scene); the plan lists the removal as an assumption. M2 is renamed and its exact item renumbered M2.5.
- 2026-11-11 S1 applied: M1.3 names a separate footprint witness, with a no-widening mutant or a record of the
  poses searched, and the design calls the four-ulp interpolation term an assumption and adds it as a risk.
- 2026-11-11 Exact baselines, from `BASELINE-GUARD`'s review: M2 gained `BASELINE-GUARD:M1` in its `needs`, and M2
  and M3 carry the verifier (Mac and Chrome) in place of the native-only command. M1 keeps the native matrix and
  does not wait; the plan lists that as an assumption.

- 2026-11-11 M1 implementation: logical-slot bucket history, class-matched depth twins and mesh admission.
  [Metal evidence](../../evidence/CULLING/M1.md): eight tests pass through churn, reveal, resize and fresh-device
  preparation; all three required mutations fail pixel equality, including the mesh tie with zero tolerance.
  The footprint reference search found no witness in 8,640 valid poses. Full flythrough, exact images, Check
  and integration remain unverified; no speed or new motion-quality claim.
