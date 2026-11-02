# Stratocumulus box-face diagnosis

2 November 2026. Read-only inspection of the attached close-orbit, ground-level Stratocumulus screenshot and current `source.wgsl`, `optics.wgsl`, `light.wgsl`, `view.wgsl`, and scene bounds. No shader edits or new capture were made. The visible broad, nearly vertical rectangular planes are a real image defect; the screenshot alone cannot identify their exact world coordinates.

## Most likely producer: finite source-domain clipping

Sc/St seeded source replaces the Cu radial envelope with `q=height*coverage` over the **entire** finite box. The ongoing Sc/St source also sets broad `coverage` and `spatial=1` without an XZ falloff. For both families the scene bounds are x=`[-5000,5000]`, z=`[-4000,4000]`, y=`[400,5600]` m. Optical `rawQ` returns zero immediately outside that box but linearly samples the edge voxel *inside* it; it does not taper q or sigma to zero at the side faces. `view.integrate` starts/stops its cloud segment at `rayBox` intersections. A close low camera can therefore see an illuminated cloud slab ending on an exactly axis-aligned world plane, including the adjoining side planes. Seeded Sc/St makes this possible immediately, and local source replenishment sustains it after advection. The density-scale control multiplies sigma and makes the edge more opaque; it does not create the geometric cut.

This source/support mechanism best explains a **hard silhouette or full-thickness planar wall**. It is consequential for later world-domain/halo design: a finite test chart cannot be mistaken for a cloud-bank edge when a broad source touches its boundary. The current Cu source is less exposed because its seeded radial envelope suppresses many edge voxels.

## Other mechanisms and distinguishing signatures

| Mechanism | Code path | Expected image signature | Decisive held-source check |
| --- | --- | --- | --- |
| Point-light cache faceting | `light.wgsl` builds 32×24×24 T/A over the whole box; `cacheT`/`ambientA` linearly filter it. Cell spacing is about313×217×333 m. Full-box 32-Sun/20-ambient uniform samples can miss thin Sc support and change adjacent cache values strongly. At the box boundary, filtered edge-cache T can also disagree with the outside `directT` path. | Piecewise smooth tonal panels/creases, possibly axis aligned, while the **opacity silhouette** stays continuous. Direct and ambient views may emphasize different panels. A domain-boundary light mismatch can coincide with a source clip. | At fixed q/camera/Sun, compare beauty, direct-only and ambient-only, then bypass/cache-reference or change light-grid resolution. If a panel changes with lighting but its geometric outline remains, it is light cache error. |
| Source voxel/carrier interpolation | q is 96×80×80, and the rest carrier is half resolution; trilinear interpolation and nonlinear detail erosion can reveal voxel-sized changes in slope. | Finer ~100–200 m facets that follow material/voxel structure, not a single long box wall. | Inspect a same-world-position q/coefficient slice; move camera while holding source to see whether facets stay with voxels. The existing “optical density slice” is an XY cut at z=0, so it does not by itself inspect the exact screen-ray depth. |
| Fixed view march | `view.integrate` uses 96 midpoints over the full ray/box interval with no jitter or occupied-interval traversal. Full-resolution UI changes pixel count, **not** this step count. | Bands or popping that move with camera, ray angle or 96-step spacing; potentially stronger at grazing rays and thin layers. | Hold source/light fixed and change only view-step count or sample phase. Motion of bands with the sampling pattern identifies a march artifact. |
| Ground receiver map | Separate128² Beer field affects `ground()` only. | Rectangles/darkness on the ground, not a cloud silhouette or volume face. | Compare cloud-only region above the ground and ground shadow independently. |

## Priority isolation sequence

1. Freeze source time and camera; capture beauty, direct-only, and ambient-only at the same optical epoch/Sun, without changing density scale. Compare the *outline and exact plane position*, not only brightness. An unchanged hard wall across all three strongly favours source clipping; tonal changes with stable outline implicate cache lighting.
2. Orbit/translate slightly at fixed source and identify whether the wall remains on x=±5000 or z=±4000 m in world space. A box-coordinate wall is decisive for the source boundary. The existing z=0 optical-density slice can corroborate a nonzero q at x edges, but cannot rule out a face at another z.
3. If the outline is smooth and only internal panels remain, compare current cache against complete direct Sun at the same sample points or change cache resolution. If panels instead shift with the ray-step count/sample phase, diagnose view quadrature. No need to infer from the screenshot alone.

Neither the current full-resolution image nor higher extinction qualifies Sc morphology. Treat the screenshot as an active visual failure until a same-state isolation identifies the producer and a corrected capture removes the planar faces.
