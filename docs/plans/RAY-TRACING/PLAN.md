# RAY-TRACING plan

Ray-traced shadows and ambient occlusion on the Mac, through wgpu's ray queries, with the portable methods as
fallback and reference. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac)

Before any milestone, read [Proof](DESIGN.md#proof), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5),
[Exact baselines](../ARCHITECTURE.md#exact-baselines) and
[the planning probes](../../evidence/RAY-TRACING/planning-probe/README.md), whose source shows the wgpu calls
that work on this Mac.

## Open for the human

- Assumption: "ray tracing" is planned as shadows from the Sun and from lamps, soft Sun shadows and ambient occlusion. Reflections by ray and global illumination are not planned. Both must shade what a ray hits, the valley has nothing mirror-like, and the water mirror that reflections would replace is not built yet. Each would be a later plan on the structure built here.
- Assumption: nothing here is on the road to the V1 exit, and no piece earns a place on it. Every effect is off unless a scene asks, the exit's three rows are not touched, and the last milestone waits for `SHOWCASE` M8.
- Assumption: traced Sun shadows become the Apple recipe's default on this Mac only if they cost no more than shadow maps and are either faster or better looking on a four-point list (contact, distance, low Sun, softness). If they look better and cost more, they stay a choice a scene makes. The other choice is to turn them on by default for their look alone, in the showcase too, which changes the V1 exit.
- Assumption: ray effects exist only on the Apple tile recipe. The portable recipe stays the reference on this Mac and refuses them by name, and a Mac launched on it, which is today's default, gains nothing from promotion.
- Assumption: lamp shadows and ambient occlusion have no portable version. Chrome, and a Mac on the portable recipe, show the scene without them. A screen-space stand-in is not planned.
- Assumption: with soft shadows on, a moving shadow's body is traced in the frame it is drawn, at the cost of one more ray for each lit pixel. Its soft edge, and ambient occlusion, follow a moving caster a few frames late, and ground that has just come into view has a hard-edged shadow for one frame. The other choice is a depth prepass, a second draw of the scene's geometry every frame.
- Assumption: the structure holds everything in the scene, on screen or off, so that lamps and a low Sun are shadowed by what the camera cannot see. Its cost grows with the scene, not the view.
- Assumption: the air's shafts, fog and clouds keep reading shadow maps, so their shadows are as they are today.
- Assumption: a material's alpha does not cut a hole in a shadow. It does not today either.
- Assumption: these milestones take a builder only when no V1 milestone can start. Their order among the other work that is off the exit (water, clouds, the 2D engine, the review's fixes) is the lead's to choose.
- Assumption: wgpu's ray tracing is marked experimental. A fault in wgpu that Thrum cannot work around stops the milestone until `APPLE-PRESENTATION` M10 brings a pinned copy of wgpu's Metal backend, and nothing is reported upstream without the owner.
- Assumption: under the owner's bar of 17 November, every shadow and occlusion is judged by eye, and no ray is held to a CPU reference. Tests keep to rays whose answer is plain from how their scene was built, and to what the structure holds.

## M1 On the Mac, the moving 10,000-entity scene is held in a ray-tracing structure, rebuilt every frame at the poses that frame draws

Rulings 1, 3 and 7 are built here. Instance records are packed in `thrum-render`'s `synthetic/upload.rs` and the mesh arena is `synthetic/mesh_arena.rs`, whose live row replacement `CONTENT-LOOP` M2 built. Devices are requested in `thrum-app`'s `app/device.rs` and `sky/capture.rs`, in the proving ground's capture host and in `thrum-test-support`. If `HOST` M1 has gathered them into one recipe, extend that. If `MESH-SHADING` M1 has landed the launch list first, add `ray-query` to it. No effect exists yet, so the tests ask for the structure alone.

## M2 The whole valley is in the ray-tracing structure, terrain and cooked meshes included, on screen or off, and an edited mesh is replaced there in the frame it changes on screen

Rulings 4 and 5 are built here, and the traced view. `thrum-asset-gpu`'s `load.rs` uploads a cooked mesh's positions, attributes and indices from the loader that `lib.rs` starts with the scene's surface, and `draw.rs` holds its placements; `MESH-SHADING` and `SCENE-VIEWS` M2 add to the same files. The proving ground's `cooked_*_gpu` tests show how a cooked scene is hosted and `scripts/content_loop_proof.py` how an edit is driven. This creates `scripts/ray_tracing_proof.py` and `scripts/ray_tracing_cost.py`'s `valley` case.

## M3 The Sun and lamps cast ray-traced shadows in the valley on the Mac: right at contact and at any distance, with hard edges

Ruling 6 is built here. `sun_visibility` and `local_radiance` are in `synthetic.wgsl`, the cascades in `synthetic/shadow.rs`, and the air's shadow heights in `synthetic/air_shadow.rs`, which read the caster height map alone. `cluster_probe.wgsl` shows how a probe evaluates shader code at chosen points. The proving ground gains a selection for each request, off by default. `ATMOSPHERE` M6 reviews the sky with the map method, which this leaves as it is.

## M4 Sun shadows soften with distance from what casts them, and the Sun dims smoothly as it sets behind a ridge

The FidelityFX Shadow Denoiser's source may be ported, with its licence recorded under `docs/provenance`. `synthetic/after_opaque.rs` and `history_gpu.rs` show a pass after opaque geometry and a history that only a submitted frame commits. If `APPLE-PRESENTATION` M4 has landed, the history follows its extent transitions; if not, its builder keeps M4.7's test passing.

## M5 Props and terrain are grounded: sky light is dimmed in creases and where things stand close together

`indirect_radiance` is in `synthetic_environment.wgsl` and `synthetic_plain.wgsl`. Microsoft's real-time denoised ambient occlusion sample is the reference for the filter.

## M6 The record says whether ray-traced Sun shadows cost no more than shadow maps on this Mac and are faster or better looking, and the Apple recipe selects them by default only if they are

`docs/evidence/APPLE-TILE/M12` is the model for a paired sitting, and `MESH-SHADING` M3 uses the same two figures and the same spread. Finish every item that does not read the sitting first, then end `BLOCKED` asking the lead for it. M6.2, M6.3 and M6.5 read what the sitting retains. M6.6 is batched with other modules' attended items.
