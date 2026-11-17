# RAY-TRACING plan

Ray-traced shadows and ambient occlusion on the Mac, through wgpu's ray queries, with the portable methods as
fallback and reference. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5),
[Exact baselines](../ARCHITECTURE.md#exact-baselines) and
[the planning probe](../../evidence/RAY-TRACING/planning-probe/README.md), whose source shows the wgpu calls
that work on this Mac.

## Open for the human

- Assumption: "ray tracing" is planned as shadows from the Sun and from lamps, soft Sun shadows and ambient occlusion. Reflections by ray and global illumination are not planned. Both must shade what a ray hits, the valley has nothing mirror-like, and the water mirror that reflections would replace is not built yet. Each would be a later plan on the structure built here.
- Assumption: nothing here is on the road to the V1 exit, and no piece earns a place on it. Every effect is off unless a scene asks, the exit's three rows are not touched, and the last milestone waits for `SHOWCASE` M8.
- Assumption: promotion is strict. Traced Sun shadows become what the Apple recipe uses by default only if the whole frame costs no more than with shadow maps. If they look better and cost more, they stay a choice a scene makes. The other choice is to turn them on by default for their look, in the showcase too, which changes the V1 exit.
- Assumption: lamp shadows and ambient occlusion have no portable version. Chrome, and a Mac on the portable recipe, show the scene without them. A screen-space stand-in is not planned.
- Assumption: a soft shadow is one frame late, and ground that has just come into view has a hard-edged shadow for one frame. The other choice is a depth prepass, a second draw of the scene's geometry every frame.
- Assumption: the air's shafts, fog and clouds keep reading shadow maps, so their shadows are as they are today.
- Assumption: a material's alpha does not cut a hole in a shadow. It does not today either.
- Assumption: these milestones take a builder only when no V1 milestone can start. Their order among the other work that is off the exit (water, clouds, the 2D engine, the review's fixes) is the lead's to choose.
- Assumption: wgpu's ray tracing is marked experimental. A fault in wgpu that Thrum cannot work around stops the milestone until `APPLE-PRESENTATION` M10 brings a pinned copy of wgpu's Metal backend, and nothing is reported upstream without the owner.

## M1 On the Mac, a ray cast into the moving 10,000-entity scene hits what a CPU reference says it should, in every frame

Rulings 1, 3 and 7 are built here. Instance records are packed in `thrum-render`'s `synthetic/upload.rs` and the mesh arena is `synthetic/mesh_arena.rs`, whose live row replacement `CONTENT-LOOP` M2 built. Devices are requested in `thrum-app`'s `app/device.rs` and `sky/capture.rs`, in the proving ground's capture host and in `thrum-test-support`. If `HOST` M1 has gathered them into one recipe, extend that. If `MESH-SHADING` M1 has landed the launch list first, add `ray-query` to it.

## M2 The whole valley is in the ray-tracing structure, terrain and cooked meshes included, and an edited mesh is replaced there in the frame it changes on screen

Rulings 4 and 5 are built here. `thrum-asset-gpu`'s `load.rs` uploads a cooked mesh's positions, attributes and indices, and `draw.rs` holds its placements; `MESH-SHADING` and `SCENE-VIEWS` M2 add to the same files. The proving ground's `cooked_*_gpu` tests show how a cooked scene is hosted and `scripts/content_loop_proof.py` how an edit is driven. This creates `scripts/ray_tracing_proof.py` and `scripts/ray_tracing_cost.py`'s `valley` case.

## M3 The Sun and lamps cast ray-traced shadows in the valley on the Mac: right at contact and at any distance, with hard edges

Ruling 6 is built here. `sun_visibility` and `local_radiance` are in `synthetic.wgsl`, the cascades in `synthetic/shadow.rs`, and the air's shadow heights in `synthetic/air_shadow.rs`, which read the caster height map alone. `cluster_probe.wgsl` shows how a probe evaluates shader code at chosen points. The proving ground gains a selection for each request, off by default. `ATMOSPHERE` M6 reviews the sky with the map method, which this leaves as it is.

## M4 Sun shadows soften with distance from what casts them, and the Sun dims smoothly as it sets behind a ridge

Start with M4.1, because the estimator must be right before its look is judged. The FidelityFX Shadow Denoiser's source may be ported, with its licence recorded under `docs/provenance`. `synthetic/after_opaque.rs` and `history_gpu.rs` show a pass after opaque geometry and a history that only a submitted frame commits. If `APPLE-PRESENTATION` M4 has landed, the history follows its extent transitions; if not, its builder keeps M4.6's test passing.

## M5 Props and terrain are grounded: sky light is dimmed in creases and where things stand close together

`indirect_radiance` is in `synthetic_environment.wgsl` and `synthetic_plain.wgsl`. Microsoft's real-time denoised ambient occlusion sample is the reference for the filter.

## M6 The record says which ray-traced paths measured better than the portable ones on this Mac, and the Apple recipe selects those by default

`docs/evidence/APPLE-TILE/M12` is the model for a paired sitting. Finish every other item first, then end `BLOCKED` asking the lead for the sitting. M6.3 is batched with other modules' attended items.
