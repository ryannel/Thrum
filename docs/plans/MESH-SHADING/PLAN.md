# MESH-SHADING plan

Cooked meshes culled and drawn a meshlet at a time by the Mac GPU's object and mesh stages, through wgpu's
mesh pipelines, with the path they replace as fallback and reference. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac)

Before any milestone, read [What is proved, and how hard](DESIGN.md#what-is-proved-and-how-hard),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5),
[Exact baselines](../ARCHITECTURE.md#exact-baselines) and
[the planning probe](../../evidence/RAY-TRACING/planning-probe/README.md), whose source shows the wgpu calls
that work on this Mac.

## Open for the human

- Assumption: mesh shaders are planned for cooked placements, which are the terrain and the cooked meshes placed in a scene, in the main view and in the Sun's shadow maps. The exit scene's 10,000 entities keep the path they have. Drawing them a meshlet at a time may be the larger saving, but it changes culling code that is held byte-equal to a reference, so the next plan decides it from M3's record. The owner can ask for it now.
- Assumption: nothing here is on the road to the V1 exit, and no piece earns a place on it. The path is off unless asked for, the exit's three rows are not touched, and the last milestone waits for `SHOWCASE` M8.
- Assumption: nothing turns the mesh stages on by default, on this Mac or any other. A person turns them on with one launch setting, and on a Mac whose GPU lacks them the same launch draws the existing path and says why. The last milestone records how the valley looks with them on this Mac and what they cost beside the existing path, timed while other builders work, and claims no measured win. The other choice is to make them the default on this Mac. That needs a measurement on a quiet machine, with other builders paused, and a rule for which Macs it covers.
- Assumption: the mesh stages and ray-traced Sun shadows can be turned on together, and neither module checks the pair. Each is checked alone. The other choice is one more check of the two together once both are built.
- Assumption: under the owner's bar of 17 November the picture is judged by eye in the valley, and every timing is a tripwire. Three comparisons stay automated. Culling on must equal culling off to the byte. The new path's frame outputs must match the old path's, and so must its shadow maps, because other modules read both and a fault there is silent.
- Assumption: meshlets are not tested against what hides them, and there are no levels of detail. The engine has neither for cooked meshes today.
- Assumption: these milestones take a builder only when no V1 milestone can start. Their order among the other work that is off the exit is the lead's to choose.
- Assumption: wgpu's mesh shading is marked experimental. A fault in wgpu that Thrum cannot work around stops the milestone until `APPLE-PRESENTATION` M10 brings a pinned copy of wgpu's Metal backend, and nothing is reported upstream without the owner.

## M1 On the Mac, one cooked mesh is culled and drawn a meshlet at a time by the GPU's object and mesh stages, and the picture matches the path it replaces

Rulings 1, 2, 4 and 8 are built here, ruling 8 for the view and light layouts. The indirect path is `thrum-asset-gpu`'s `cull.wgsl`, `cooked.wgsl` and `draw.rs`, and `COOKED-DRAW`'s `CookedDrawPath` selects between it and the direct path. `load.rs` stages a mesh and makes its buffers; the cooked product's meshlet table and its meshlet-vertex and meshlet-triangle sections are in `thrum-asset`'s `layout.rs` and are not uploaded today. A material reload lives in `material.rs` (`combine`, `Target`, the candidate) and is drawn and installed in `lib.rs` (`mesh_override`, `submitted`); it holds one pipeline today. `scripts/cooked_draw_parity.py` shows how two cooked paths are compared from a script, and keeps difference images. This creates `scripts/mesh_stage_proof.py`. If `RAY-TRACING` M1 has landed the launch list first, add `mesh-stages` to it.

## M2 The valley's terrain and cooked meshes draw through the object and mesh stages in the main view and the Sun's shadow maps, through edits and device loss, with the cost of both paths on record

`draw_shadow` in `draw.rs` draws a cooked mesh whole into each of the Sun's map passes, and `shadow_pipeline` there holds the caster's raster state with the reason both faces are drawn. `thrum-render`'s `synthetic/shadow.rs` owns the three maps and the caster view layout it lends as `SceneShadowSurface::view_layout`, which only the vertex stage can read today. Ruling 8 lets this milestone widen that one binding's visibility on a device that enabled the stages; nothing else there changes. Ruling 7's whole-frame GPU span is added to `timing.rs` here if `RAY-TRACING` M3 has not added it, and used as found if it has. `scripts/content_loop_proof.py` shows how an edit is driven and `scripts/cooked_draw_cost.py` how the cooked paths are timed. This creates `scripts/mesh_stage_cost.py`. `RAY-TRACING` M2 and `SCENE-VIEWS` M2 add to the same files.

## M3 A person turns the object and mesh stages on for this Mac with one launch setting, and the record shows how the valley looks and what it costs beside the path it replaces

`RAY-TRACING` M6 is the same kind of milestone, and where its launch selection or its record's layout has landed, follow it. The proving ground's valley gains one launch selection, off by default. `scripts/mesh_stage_cost.py` from M2 takes the pair, under the `gpu` lock and while other builders work; its `check` reads the retained record and measures nothing, so it passes on a busy machine. No sitting is asked for.
