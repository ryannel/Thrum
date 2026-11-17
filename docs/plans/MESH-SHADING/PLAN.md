# MESH-SHADING plan

Cooked meshes culled and drawn a meshlet at a time by the Mac GPU's object and mesh stages, through wgpu's
mesh pipelines, with the indirect path as fallback and reference. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5),
[Exact baselines](../ARCHITECTURE.md#exact-baselines) and
[the planning probe](../../evidence/RAY-TRACING/planning-probe/README.md), whose source shows the wgpu calls
that work on this Mac.

## Open for the human

- Assumption: mesh shaders are planned for cooked placements, which are the terrain and the cooked meshes placed in a scene, in the main view and in the Sun's shadow maps. The exit scene's 10,000 entities keep the path they have. Drawing them a meshlet at a time may be the larger saving, but it changes culling code that is held byte-equal to a reference, so it is planned only if M3's record shows the stages are faster on this GPU. The owner can ask for it now.
- Assumption: nothing here is on the road to the V1 exit, and no piece earns a place on it. The path is off unless asked for, the exit's three rows are not touched, and the last milestone waits for `SHOWCASE` M8.
- Assumption: promotion is strict. The stages become what the Apple recipe uses by default only if they measure faster. If they do not, the path stays in the engine as a named choice and the record says so.
- Assumption: meshlets are not tested against what hides them, and there are no levels of detail. The engine has neither for cooked meshes today.
- Assumption: these milestones take a builder only when no V1 milestone can start. Their order among the other work that is off the exit is the lead's to choose.
- Assumption: wgpu's mesh shading is marked experimental. A fault in wgpu that Thrum cannot work around stops the milestone until `APPLE-PRESENTATION` M10 brings a pinned copy of wgpu's Metal backend, and nothing is reported upstream without the owner.

## M1 On the Mac, one cooked mesh is culled and drawn a meshlet at a time by the GPU's object and mesh stages, and the picture matches the path it replaces

Rulings 1, 2, 4 and 8 are built here. The indirect path is `thrum-asset-gpu`'s `cull.wgsl`, `cooked.wgsl` and `draw.rs`, and `COOKED-DRAW`'s `CookedDrawPath` selects between it and the direct path. `load.rs` uploads a mesh; the cooked product's meshlet table and its meshlet-vertex and meshlet-triangle sections are in `thrum-asset`'s `layout.rs` and are not uploaded today. `scripts/cooked_draw_parity.py` shows how two cooked paths are compared from a script. This creates `scripts/mesh_stage_proof.py`. Do M1.3 before M1.4, because its record is the bound M1.4 holds. If `RAY-TRACING` M1 has landed the launch list first, add `mesh-stages` to it.

## M2 The valley's terrain and cooked meshes draw through the object and mesh stages in the main view and the Sun's shadow maps, through edits and device loss, with the cost of both paths on record

`draw_shadow` in `draw.rs` draws a cooked mesh whole into each shadow pass. `scripts/content_loop_proof.py` shows how an edit is driven and `scripts/cooked_draw_cost.py` how the cooked paths are timed. This creates `scripts/mesh_stage_cost.py`. `RAY-TRACING` M2 and `SCENE-VIEWS` M2 add to the same files.

## M3 The record says whether the object and mesh stages measured faster on this Mac, and the Apple recipe selects them by default only if they did

`docs/evidence/APPLE-TILE/M12` is the model for a paired sitting. Finish every other item first, then end `BLOCKED` asking the lead for the sitting.
