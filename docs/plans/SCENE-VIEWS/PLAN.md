# SCENE-VIEWS plan

The scene drawn a second time from another camera, in the same frame, into a texture another module reads.
Water's mirrored scenery is its first use. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the
criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-scene-views) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: a secondary view may hold less than the main view: no sky, water, clouds or other effect drawn after opaque geometry, clear air where the main view has fog (the reader still applies the main view's fog on its own stretch), nothing smaller than a least size, and Sun shadows from one cascade of its own with the scene's height map beyond it.
- Assumption: a secondary view's camera must stand directly above or below the main camera, at any height and facing any way, which every water mirror does. A view from anywhere else (a monitor in the world, a portal, a probe that stands elsewhere, a mirror in a tilted plane) is refused today, because hill-scale Sun shadows in a view come from the frame's one height map, which is right only there. Serving one later means a second height map drawn round that camera each frame; it would add to the module and change nothing built now.
- Assumption: air that starts at a plane is offered for a level plane only, which every water plane is. A view cut at a tilted plane is refused that option.
- Assumption: a reflection's haze is the main view's own, `ATMOSPHERE`'s aerial product for an observer standing on the water, and this module adds no rule of its own to it. Near sea level that product thins haze just above the horizon, in the main view and in a reflection alike, until `ATMOSPHERE` M9 is Done. M9 corrects it in the one aerial pass, so a reflection follows with no change here, and M1 does not wait for it.
- Assumption: reflections are drawn by rasterising the scene again on every device. Ray-traced reflection on Apple devices is not planned: wgpu has it only as an experimental feature and Chrome has none.
- Assumption: the module's own proof is a new small example, `examples/mirror`, with a flat pool. The V1 exit scene is not touched.
- Assumption: the iPhone simulator is not a target, as it is not for `WATERS`.
- Assumption: cost is recorded on the Mac only, and no timing figure decides an item (brief decision 3).

## M1 A scene draws its entities a second time from another camera into a texture, lit and shadowed as the main view shows them, and a scene that asks for no view is unchanged

Slots, requests, a view's own products and the tests of every item but M1.1 and M1.13 are on main from `82a636e10` ([handoff](../../evidence/SCENE-VIEWS/M1-handoff.md)). The handoff's question is ruled in [the design](DESIGN.md#the-camera-stands-on-the-main-cameras-vertical): a view goes on reading the frame's height layer and address record, as `shadow/secondary.rs` has it, the main view's fit is not edited, and a request off the main camera's vertical is refused when the slot is asked, from the main camera's position as the frame declares it. An existing scene-view test whose view camera stands off its main camera's vertical moves that camera and keeps its claim. `shadow/tests/scene_view_anchor.rs` stays as the measurement behind the refusal. Rulings 1 to 4 are built here, in files other modules own: `contribution.rs` and `synthetic/{contributed,scene_graph}.rs` (`EFFECT-GRAPH`), `visibility.rs` and `synthetic/{camera_culling,mesh_route,buckets}.rs` (`CULLING`), `synthetic/{shadow,cluster,view,forward}.rs` and `synthetic.wgsl` (the forward pass), and `atmosphere/{graph.rs,aerial.wgsl}` and `synthetic_aerial_{clear,shadowed}.wgsl` (`ATMOSPHERE`). Their items and tests stay as they are and keep passing. `CULLING`'s Sun-caster culling in `visibility.rs` is the working model for a compaction against a volume that is not the camera's, and the Sun cascades in `shadow.rs` for a second draw of the scene. The tests make their own scenes; `examples/v1-game` is not edited. The equality items' allowances are the design's [Bounds](DESIGN.md#proof-and-builder-handoff): a larger difference is a defect to find, or a `BLOCKED` with the measurement, never a new bound. Air from the plane runs the aerial pass for an observer at the plane's height on every line of the view's volume, those that never cross the plane included, and has the plain and the shadowed decoder address it by a fragment's distance beyond its own crossing, for a view that asks; the main view's text stays as it is. `AtmosphereFrame` refuses an observer below 0 m of altitude, and a mirror's camera can stand below sea level; the observer here is the plane's height held to 0 m, never the camera's. M1.5 and M1.12 take every reference from `ATMOSPHERE`'s pass and the main view as they stand, per [What a view's air is held to](DESIGN.md#what-a-views-air-is-held-to): `atmosphere/gpu/tests/aerial.rs` shows the volume read back, and `atmosphere/products/tests/filtered.rs` works out `Q`.

## M2 A pool in a small scene mirrors the tower, rocks, lamp and far hill around it, cooked meshes included, whether or not the camera can see them

This builds ruling 5 in `thrum-asset-gpu` and creates `examples/mirror` (package `mirror-scene`, registered in `.config/architecture.toml`) on `thrum-app`'s sky kit, as `examples/lighthouse` is; its meshes may be copied from that example. `SkyGame::with_contribution` takes the pool's effect, which draws in `EFFECT-GRAPH`'s `after-opaque` stage. Start with M2.1's question of whether the indirect cooked path can hold a second set of culling buffers, because the answer decides what a view of cooked meshes resolves to. M2.3's three reference colours are frames of the same scene from the main view with the pool's effect left out.

## M3 The mirror pool draws in Chrome, what a view costs is on record beside the main view with the knobs that cut it, and the module is closed

`scripts/check_starter_browser.mjs` shows how a creator scene's page is launched and checked; this creates `scripts/check_mirror_browser.mjs` and `scripts/scene_view_cost.py`. `scripts/culling_cost.py` shows how paired pass timings are taken.
