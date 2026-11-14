# SCENE-VIEWS plan

The scene drawn a second time from another camera, in the same frame, into a texture another module reads.
Water's mirrored scenery is its first use. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the
criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-scene-views) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: a secondary view may hold less than the main view: no sky, water, clouds or other effect drawn after opaque geometry, clear air where the main view has fog, nothing smaller than a least size, and Sun shadows from one cascade of its own with the scene's height map beyond it.
- Assumption: the module is built for water's mirror first. A request is any camera, so a probe, a portal or a monitor in the world can ask later, but nothing is built or proved for them now.
- Assumption: reflections are drawn by rasterising the scene again on every device. Ray-traced reflection on Apple devices is not planned: wgpu has it only as an experimental feature and Chrome has none.
- Assumption: the module's own proof is a new small example, `examples/mirror`, with a flat pool. The V1 exit scene is not touched.
- Assumption: the iPhone simulator is not a target, as it is not for `WATERS`.
- Assumption: cost is recorded on the Mac only, and no timing figure decides an item (brief decision 3).

## M1 A scene draws its entities a second time from another camera into a texture, lit and shadowed as the main view shows them, and a scene that asks for no view is unchanged

Rulings 1 to 4 are built here, in files other modules own: `contribution.rs` and `synthetic/{contributed,scene_graph}.rs` (`EFFECT-GRAPH`), `visibility.rs` and `synthetic/{camera_culling,mesh_route,buckets}.rs` (`CULLING`), `synthetic/{shadow,cluster,view,forward}.rs` and `synthetic.wgsl` (the forward pass), and `atmosphere/{graph.rs,aerial.wgsl}` (`ATMOSPHERE`). Their items and tests stay as they are and keep passing. `CULLING`'s Sun-caster culling in `visibility.rs` is the working model for a compaction against a volume that is not the camera's, and the Sun cascades in `shadow.rs` for a second draw of the scene. The tests make their own scenes; `examples/v1-game` is not edited. M1.1's and M1.6's bounds are measured first and then asserted.

## M2 A pool in a small scene mirrors the tower, rocks, lamp and far hill around it, cooked meshes included, whether or not the camera can see them

This builds ruling 5 in `thrum-asset-gpu` and creates `examples/mirror` (package `mirror-scene`, registered in `.config/architecture.toml`) on `thrum-app`'s sky kit, as `examples/lighthouse` is; its meshes may be copied from that example. `SkyGame::with_contribution` takes the pool's effect, which draws in `EFFECT-GRAPH`'s `after-opaque` stage. Start with M2.1's question of whether the indirect cooked path can hold a second set of culling buffers, because the answer decides what a view of cooked meshes resolves to.

## M3 The mirror pool draws in Chrome, what a view costs is on record beside the main view with the knobs that cut it, and the module is closed

`scripts/check_starter_browser.mjs` shows how a creator scene's page is launched and checked; this creates `scripts/check_mirror_browser.mjs` and `scripts/scene_view_cost.py`. `scripts/culling_cost.py` shows how paired pass timings are taken.
