# RAY-TRACING M2 review

**Pass. All eight items meet the milestone bar.** The submitted Metal frames agree on where the valley geometry stands. The publication tests put the ray change in the same submission as the image change. Both Must Fix findings from the previous review are resolved.

## Previous findings

- **Initial mesh builds run after the generation is ready — resolved.** Arena preparation now submits the resident mesh builds and waits for that submission before returning the scene. On both the first and replacement device, the lifecycle observer sees all five arena builds before the first ordinary frame. Streamed placements have their own readiness boundary. The contribution waits for the initial upload's queue-completion callback before offering that set for its first draw. Same-device visual preparation retains the existing mesh structures. [Arena preparation](../../../crates/thrum-render/src/synthetic/raytrace.rs) · [Cooked publication](../../../crates/thrum-asset-gpu/src/lib.rs) · [Lifecycle observations](M2/lifecycle.json)
- **Traced view omits the hit instance identity — resolved.** The shader selects a hue from the committed hit's instance ID and attenuates it with hit distance. The newly captured crops distinguish the small entities individually. The seam test isolates identity by placing two copies of one plane edge to edge at equal depth. It samples different colors on opposite sides of the shared edge, then moves the camera back and observes both colors darken. [Shader](../../../crates/thrum-render/src/synthetic/raytrace/view.wgsl) · [Coplanar test and samples](M2/seam.txt)

## M2.1 inspection

Before capture I listed the near ground and distant ridge, the cooked rock and sphere, and every visible entity. For each pair I compared the whole frame for missing, extra or displaced surfaces, then the native-size contours, lower edges and overlaps for disagreement where objects meet terrain.

I built the game and cooker at review commit `0f9a2854e4af`, cooked the checked-in showcase and captured fixed 1280 × 720 submissions on Apple M3 Max Metal with the Apple tile recipe and ray queries enabled. Each link shows raster on the left and primary-ray hits on the right. Both arms use the same camera, stopped world, proof population and cooked mesh rows.

| Fixed view | Whole frames | Native crop | Observation |
| --- | --- | --- | --- |
| Level, noon | [Pair](M2-review/valley-frames.png) | [Contacts](M2-review/valley-contacts.png) | The skyline peaks and saddles agree. The sphere, angular rock and eleven small entities occupy the same ground. |
| Down 13° | [Pair](M2-review/contact-frames.png) | [Contacts](M2-review/contact-contacts.png) | The downward view preserves the lower outline of both props and the staggered positions of the two nearest entities. |
| West 12°, down 8° | [Pair](M2-review/west-frames.png) | [Contacts](M2-review/west-contacts.png) | The group shifts right in both arms. The rock's broad base and the gaps between the small entities agree. |
| East 13°, down 8° | [Pair](M2-review/east-frames.png) | [Contacts](M2-review/east-contacts.png) | The group shifts left. The sphere's enlarged outline, the rock's tapered end and the distant ridge still align. |
| East 8°, down 8°, 06:30 | [Pair](M2-review/sunrise-frames.png) | [Contacts](M2-review/sunrise-contacts.png) | Long raster shadows cross the group. The low light exposes the same foreground silhouettes and ridge breaks as the traced view. |

These fixed captures cover distance, camera turns and low Sun. Storm, fast camera motion and display cadence were outside this fixed-frame inspection.

## Item verdicts

| Item | Verdict | Basis |
| --- | --- | --- |
| M2.1 | Pass | All five comparisons above agree at the scale of the whole valley and individual contacts. |
| M2.2 | Pass | An author can implement the optional callback from its documentation. It specifies every opaque placement, including offscreen ones, after uploads and before the scene build, in stable order with the tentative draw transform. The receiver specifies a row-major affine transform, bit 0 for shadow and bit 1 for ambient occlusion. The seam test offers two off-camera placements without drawing either, distinguishes the masks by ray, and checks the empty contribution. The renderer manifest has no cooked GPU dependency. [Public seam](../../../crates/thrum-render/src/contribution.rs) · [Seam test](../../../crates/thrum-render/tests/ray_instances_seam.rs) |
| M2.3 | Pass | Yes. In one host each snapshot reads submitted raster bytes, then queries that submission's structure before another scene frame. Placement and entity-row edits separately reverse the old-only and new-only hit pairs when the image changes. A skipped switch preserves the prior GPU answers, replay switches both readers, and refusal of a coupled publication preserves image and hits. [Edit test](../../../examples/v1-game/tests/ray_tracing_gpu.rs) · [Submission record](M2/edit.json) |
| M2.4 | Pass | The oracle uses the scene's active slots and configured placements. It checks IDs, translations and both mask bits, then compares complete instance records across camera turns and Sun-map settings. GPU rays hit the distant entity and placement outside the camera and all fitted Sun volumes. The producer walks the bound placements independently of caster index counts. [Caster proof](../../../examples/v1-game/tests/ray_tracing/proof/casters.rs) · [Producer](../../../crates/thrum-asset-gpu/src/draw.rs) |
| M2.5 | Pass | Both generations satisfy the readiness boundary described above. Each edit builds one affected mesh. Recovery restores the edited image and ray hits. Steady and advancing frames and same-device visual replacement add zero mesh builds. [Lifecycle test](../../../examples/v1-game/tests/ray_tracing_gpu.rs) · [Visual replacement test](../../../crates/thrum-render/tests/ray_instances_seam.rs) |
| M2.6 | Pass | Two live scenes on one ray-enabled device receive demand through their respective surfaces before buffer creation. The proof reads actual usages and BLAS presence for all three cooked meshes. Only the demanding scene adds build usage to positions and indices. The ordinary submitted bytes match the pre-M2 reference. [Device observations](M2/demand.json) · [Main reference](M2/demand-main.json) |
| M2.7 | Pass | Two runs under load measured the same 15,384,576-byte resident delta, with triangle counts and matched GPU intervals for all five mesh builds. The terrain edit added an estimated 0.733–1.061 ms in its switch submission. The investigation follows the cold outlier when build order changes and records the uncertainty for small meshes. [Measurements and investigation](M2/README.md#valley-build-costs-under-load) |
| M2.8 | Pass | The script's passing verifier uses the shared main-branch exact-images check covering the ordinary images on both Mac recipes and in Chrome. |

I used the script's successful project gate and M2.2–M2.8 checks for builder revision `564604fb7cb0`. The independent captures above are from the supplied review checkout.
