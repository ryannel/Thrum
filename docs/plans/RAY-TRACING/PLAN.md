# RAY-TRACING plan

Ray-traced shadows and ambient occlusion on the Mac, through wgpu's ray queries, with the portable methods as
fallback and reference. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac)

Before any milestone, read [Proof](DESIGN.md#proof), [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5),
[Exact baselines](../ARCHITECTURE.md#exact-baselines) and
[the planning probes](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md), whose source shows the wgpu calls
that work on this Mac.

## Open for the human

- Assumption: "ray tracing" is planned as shadows from the Sun and from lamps, soft Sun shadows and ambient occlusion. Reflections by ray and global illumination are not planned. Both must shade what a ray hits, the valley has nothing mirror-like, and the water mirror that reflections would replace is not built yet. Each would be a later plan on the structure built here.
- Assumption: nothing here is on the road to the V1 exit, and no piece earns a place on it. Every effect is off unless a scene asks, the exit's three rows are not touched, and the last milestone waits for `SHOWCASE` WP8.
- Assumption: nothing ray-traced is on by default, on this Mac or any other. A person turns it on with one launch setting, and on a machine that cannot trace rays the same launch draws shadow maps and says why. That setting gives traced, soft Sun shadows; lamp shadows and ambient occlusion are separate settings. The last milestone records how the valley looks as that setting launches it on this Mac, and with every effect on, and what it costs beside shadow maps, timed while other builders work, and claims no measured win. The other choice is to make traced Sun shadows the default on this Mac, in the showcase too. That changes the V1 exit and needs the measured win.
- Assumption: ray effects exist only on the Apple tile recipe, so the one setting also moves a launch that names no recipe from the portable recipe, today's default, to Apple tile, on a Mac that traces rays and supports that recipe. The portable recipe stays the reference on this Mac: a launch that names it draws shadow maps with the setting on, and reports why.
- Assumption: lamp shadows and ambient occlusion have no portable version. Chrome, and a Mac on the portable recipe, show the scene without them. A screen-space stand-in is not planned.
- Assumption: with soft shadows on, a moving shadow's body is traced in the frame it is drawn, at the cost of one more ray for each lit pixel. Its soft edge, and ambient occlusion, follow a moving caster a few frames late, the edge a fast shadow has just left shows for one more frame, and ground that has just come into view has a hard-edged shadow for one frame. The other choice is a depth prepass, a second draw of the scene's geometry every frame.
- Assumption: the structure holds everything in the scene, on screen or off, so that lamps and a low Sun are shadowed by what the camera cannot see. Its cost grows with the scene, not the view.
- Assumption: the air's shafts, fog and clouds keep reading shadow maps, so their shadows are as they are today.
- Assumption: a material's alpha does not cut a hole in a shadow. It does not today either.
- Assumption: a coarse rounded prop is lit wherever its smooth shading faces the Sun, so it shows no dark facets where its flat triangles turn away first. The cost is one narrow case: at the open rim of a single-sided shell, a thin part of the same mesh lying within the few centimetres between the flat triangles and the smooth surface they stand for may cast no shadow on that strip.
- Assumption: these milestones take a builder only when no V1 milestone can start. Their order among the other work that is off the exit (water, clouds, the 2D engine, the review's fixes) is the lead's to choose.
- Assumption: wgpu's ray tracing is marked experimental. A fault in wgpu that Thrum cannot work around stops the milestone until `APPLE-PRESENTATION` WP10 brings a pinned copy of wgpu's Metal backend, and nothing is reported upstream without the owner.
- Assumption: under the owner's bar of 17 November, every shadow and occlusion is judged by eye, and no ray is held to a CPU reference. Tests keep to rays whose answer is plain from how their scene was built, and to what the structure holds.

## WP1 On the Mac, the moving 10,000-entity scene is held in a ray-tracing structure, rebuilt every frame at the poses that frame draws: done

[review](../../../.orchestrate/evidence/RAY-TRACING/WP1-review.md)

## WP2 The whole valley is in the ray-tracing structure, terrain and cooked meshes included, on screen or off, and an edited mesh is replaced there in the frame it changes on screen: done

[review](../../../.orchestrate/evidence/RAY-TRACING/WP2-review.md)

## WP3 The Sun and lamps cast ray-traced shadows in the valley on the Mac: right at contact and at any distance, with hard edges

The shadow shaders, proving-ground selections, timed-pass span and [terminator passage](DESIGN.md#a-rounded-meshs-own-facets)
are built. `thrum-render/src/runtime/preparation.rs` owns 3D preparation, and `contribution/list.rs` dispatches
to members. The constructor binds its presenter in `runtime.rs`. The detached visual, device and extent
operations are in `runtime/visual.rs`, `runtime/device.rs` and `runtime/extent.rs`. The detached extent job
already stages secondary view bindings. `CookedAssets::rebuild_inner` currently validates one member and
then installs its live state. [The retained probe](../../../.orchestrate/evidence/RAY-TRACING/WP3-review/c7cbfaf/composed-retry.diff)
shows the two-member failure. `ObserveView` and `Fault` in the existing `ray_tracing_gpu` proof need the
opt-in to reach their traced cases. Keep the CPU guard in `thrum-render`'s library tests and the Metal
guard in `thrum-asset-gpu`'s `it` binary. WP5 owns the cloud provider's opt-in.

## WP4 Sun shadows soften with distance from what casts them, and the Sun dims smoothly as it sets behind a ridge

The FidelityFX Shadow Denoiser's source may be ported, with its licence recorded under `docs/provenance`. `synthetic/after_opaque.rs` and `history_gpu.rs` show a pass after opaque geometry and a history that only a submitted frame commits. If `APPLE-PRESENTATION` WP4 has landed, the history follows its extent transitions; if not, its builder keeps WP4.7's test passing. WP4.4 needs a caster the valley may not have, low and fast enough that its shadow outruns its own edge: add one to the proving ground behind a selection that is off by default. For WP4.11, [WP3's hit identities](../../../.orchestrate/evidence/RAY-TRACING/WP3/review-fixes.md#sun-stills) name three balloons that take another balloon's shadow from over 600 m at the fixed low-Sun stops; if none of those edges crosses a terminator band, place a caster the same way. Its open shell and panel are new: add them to the proving ground behind a selection that is off by default, as one mesh and as two. [The fourth probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/SOFT.md) has the shell, the panel and both soft values on the CPU.

## WP5 Props and terrain are grounded: sky light is dimmed in creases and where things stand close together

`indirect_radiance` is in `synthetic_environment.wgsl` and `synthetic_plain.wgsl`. Microsoft's real-time denoised ambient occlusion sample is the reference for the filter.
The production list is assembled by `examples/v1-game/src/clouds.rs`. Its `Shade::rebuild_for_attachment`
can take the accepted weather map before validation, and cloud setup currently submits noise work directly.
Use [the candidate preparation boundary](DESIGN.md#scene-preparation-and-maps-fallback-m3) for this opt-in.

## WP6 A person turns ray-traced shadows on for this Mac with one launch setting, the record shows how the valley looks with them and what they cost beside shadow maps, and where rays cannot run the same launch draws shadow maps

Nothing is promoted and no host asks for `ray-query` unasked: [the design](DESIGN.md#selection-fallback-and-the-opt-in) says what the opt-in is and what the record holds. `MESH-SHADING` WP3 is the same kind of milestone, and where its launch selection or its record's layout has landed, follow it. Ruling 12's recipe preference is built here, in `capabilities.rs` and `capabilities/recipe.rs`, unless that milestone has landed it; `recipe::resolve` returns portable for every launch that names no recipe today, and still must where no preference is stated. The pair's `Maps` arm names `THRUM_RECIPE=apple-tile` and the traced arm names none. `.orchestrate/evidence/APPLE-TILE/WP12` shows how paired, interleaved runs are retained, and `scripts/recipe_cost.py` how a route is timed with capture off. The pair is taken under the `gpu` lock while other builders work, so ask for no quiet sitting. `ray_tracing_cost.py check` reads the retained record and measures nothing, so it passes on a busy machine. WP6.3 and WP6.4 share one capture session. WP6.5 is batched with other modules' attended items.
