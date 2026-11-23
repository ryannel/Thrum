# SCENE-VIEWS plan

The scene drawn a second time from another camera, in the same frame, into a texture another module reads.
Water's mirrored scenery is its first use. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the
criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Before any milestone, read [Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-scene-views) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines).

## Open for the human

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: a secondary view may hold less than the main view: no sky, water, clouds or other effect drawn after opaque geometry, clear air where the main view has fog (the reader still applies the main view's fog on its own stretch), nothing smaller than a least size, and Sun shadows from one cascade of its own with the scene's height map beyond it.
- Assumption: a secondary view's camera must stand directly above or below the main camera, at any height and facing any way, which every water mirror does. A view from anywhere else (a monitor in the world, a portal, a probe that stands elsewhere, a mirror in a tilted plane) is refused today, because hill-scale Sun shadows in a view come from the frame's one height map, which is right only there. Serving one later means a second height map drawn round that camera each frame; it would add to the module and change nothing built now.
- Assumption: every scene's hill-shadow map is now placed from the position its camera states, where it was placed from one worked back out of the camera's matrix. A reflection and the main view then always share one map, with no camera excepted. It changes the main view of a scene with no water too, but only for a camera within a few millionths of a metre of a 32 m grid line (millimetres at 20 km out), whose far shadows could step 32 m as it turned. The retained reference images are expected not to move.
- Assumption: air that starts at a plane is offered for a level plane only, which every water plane is. A view cut at a tilted plane is refused that option.
- Assumption: a reflection's haze is the main view's own, `ATMOSPHERE`'s aerial product for an observer standing on the water, and this module adds no rule of its own to it. Near sea level that product thins haze just above the horizon, in the main view and in a reflection alike, until `ATMOSPHERE` M9 is Done. M9 corrects it in the one aerial pass, so a reflection follows with no change here, and M1 does not wait for it.
- Assumption: reflections are drawn by rasterising the scene again on every device. Ray-traced reflection on Apple devices is not planned: wgpu has it only as an experimental feature and Chrome has none.
- Assumption: the module's own proof is a new small example, `examples/mirror`, with a flat pool. The V1 exit scene is not touched.
- Assumption: the iPhone simulator is not a target, as it is not for `WATERS`.
- Assumption: cost is recorded on the Mac only, and no timing figure decides an item (brief decision 3).
- Assumption: this module takes no quiet-machine sitting, and no builder is paused for it. Its cost record is the one taken while other builders worked, kept with its load, and nothing in the evidence, the author guide or the ledger is called a figure for a quiet machine. If such figures are wanted, they wait for one window batched with other modules' (the lead's task of 18 November, on the owner's bar of 17 November).

## M1 A scene draws its entities a second time from another camera into a texture, lit and shadowed as the main view shows them, and a scene that asks for no view is unchanged: done
[review](../../../.orchestrate/evidence/SCENE-VIEWS/M1-review.md)

## M2 A pool in a small scene mirrors the tower, rocks, lamp and far hill around it, cooked meshes included, whether or not the camera can see them: done
[review](../../../.orchestrate/evidence/SCENE-VIEWS/M2-review.md)

## M3 The mirror pool draws in Chrome, what a view costs is on record beside the main view with the knobs that cut it, and the module is closed

The work is on `build/scene-views` at `7a5f2c02b596`, with every item but M3.4 passed by its builder ([handoff](../../../.orchestrate/evidence/SCENE-VIEWS/M3-handoff.md), which still asks for a quiet sitting that is no longer wanted). M3.4 was planned again on 18 November ([design](DESIGN.md#cost-account), [log](LOG.md)): the record that stands is the one on main, taken at `25cfadff5bf44` beside another builder's GPU run, and no new timing run is asked for. Since that commit the milestone has changed tests, one lint attribute in `runtime/tokens.rs` and the layout of `thrum-core`'s random draw, whose results are the same, so the record is still of the view code this milestone leaves; retake it under `lock gpu` only if something a view draws changes before the merge. What is left is words: the evidence's sentence that a qualified sitting is pending, the conditions named by the endpoint table, the author guide and the ledger, and the handoff. The second field slot's excess keeps its follow-up as logged, with no quiet run promised for it. Run `./dev check` without a lock: the last session's ran under `gpu` and was stopped at the 30-minute hold limit.

`scripts/check_starter_browser.mjs` shows how a creator scene's page is launched and checked, and `scripts/culling_cost.py` how paired pass timings are taken.

## M4 A small scene catches wrong secondary-view air and shade

Use only the panels, views and frames needed for the crossing. Preserve the real GPU readback that proves
matching main and secondary air and Sun shade, and retain the wrong-stretch controls whose outputs must
differ. Check camera or coordinate arithmetic on the CPU when it needs no submission.

Record each listed test’s disposition and the coverage that replaces it if split. For each fast case,
record before and after per-test times on the Mac in the default nextest test build used by the workspace
check, excluding compilation. Measure the revised cases alone and run them as a focused batch alongside
normal builder work. Use the supplied run for the starting times where available. The batch passes the
existing ten-second limit on its first run.

These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

- 7.226 s — `thrum-render::scene_view_air_shaded_gpu plane_panels_match_the_main_camera_at_the_crossing_and_reject_wrong_stretches`
