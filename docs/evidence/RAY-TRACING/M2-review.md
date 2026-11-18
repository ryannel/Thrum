# RAY-TRACING M2 review

**Result: FAIL.** Seven items pass. M2.5 fails at the scene-ready boundary. The traced view also misses an explicit diagnostic requirement in the design.

## Findings

1. **M2.5 — Must Fix, verified. Initial mesh builds run after the generation is ready.** On first load the renderer allocates the arena structures and sets `initial_ray_builds = true` (`mesh_arena.rs:239`). The frame's structure pass later consumes the pending builds (`synthetic/raytrace.rs:235`). The host has already called `creations.ready()` (`runtime.rs:819`). Device replacement has the same ordering. In `runtime/device.rs:241` the host marks the replacement ready before returning from `publish_device`. The lifecycle test then installs a fresh observer, submits frames until the cooked publication draws, and **requires that observer to see five arena mesh builds** (`examples/v1-game/tests/ray_tracing_gpu.rs:134–142`). This is direct evidence of late builds even for the already-resident arena rows. A correct first image and zero steady-state rebuilds both pass today. Move initial resident builds into preparation and observe the ready boundary on both generations. The first ordinary frame should require zero initial mesh builds. The saved Metal run and its production path establish the order without repeating the check.
2. **Outside the items — Must Fix, verified. Traced view omits the hit instance identity.** The design specifies shading by which instance the ray hits and how far away it is. The fragment shader reads the hit kind and distance, estimates a normal from nearby hit positions and applies a single blue-grey tint. It never reads the hit's instance identity. Two adjoining coplanar placements with the same orientation therefore have no identity boundary in this view. The native crops show the common shading on the sphere, rock and small entities. I verified the omission in the shader and inspected the valley. I did not run a separate coplanar fixture. A proposed acceptance item is that two adjoining coplanar opaque placements at the same depth can be distinguished by hit identity in the traced view, with their shared edge and distance cue preserved. [Design](../../plans/RAY-TRACING/DESIGN.md#proof) · [Fragment shader](../../../crates/thrum-render/src/synthetic/raytrace/view.wgsl)

## M2.1 inspection list

At four fixed valley viewpoints, compare the submitted drawn and traced frames. Follow the far ridgeline, the near terrain silhouette and openings onto the sky. Locate each cooked mesh and the entities in both views. At native resolution compare their outlines, occlusion and points of contact with terrain. Look for missing, extra or displaced surfaces in either view.

## Item verdicts

| Item | Verdict | Evidence and judgement |
| --- | --- | --- |
| M2.1 | Pass | The four fixed pairs below show the same near ground, distant ridge peaks and valleys, sphere on the left, irregular rock on the right, and eleven smaller entities between them. The downward and turned views preserve the lower contours, spacing and overlaps at native size. I found no missing or displaced geometry. The separate identity finding concerns what the diagnostic can tell the user about a hit. |
| M2.2 | Pass | Yes. The optional callback documents the recording phase, tentative switch, generation, stable order and off-screen membership. Its receiver documents the row-major affine transform and both mask bits. The list forwards the call to each contribution. The public-seam test offers the same BLAS twice without drawing, verifies shadow-mask inclusion and exclusion by ray, and compares the empty contribution with the control. The renderer has no dependency on the cooked GPU crate. |
| M2.3 | Pass | Yes. Each snapshot reads the submitted raster and then queries that frame's TLAS before another game frame records. The old-only and new-only rays reverse separately for the placement and entity edits when the raster bytes change. The skipped recording leaves the old GPU answers in place, the replay switches both readers, and the oversized coupled publication leaves both unchanged. |
| M2.4 | Pass | Yes. The expected set comes from active scene slots and authored placements. The check compares identities, translations and masks, then compares complete instance records after a turn and across Sun-map settings. Shadow rays hit the entity and placement on opposite sides of the camera, with their boxes outside all three fitted Sun volumes when maps are on. The cooked contribution iterates its bound meshes directly. |
| M2.5 | Fail | The lifecycle check establishes successful recovery, one rebuild for the edit, and steady frames without mesh builds. It also observes initial builds after readiness on both generations. See finding 1. |
| M2.6 | Pass | Two live hosts share the Metal device. The loader receives demand from its own scene surface before allocating buffers. The test reads actual buffer usages and BLAS presence for terrain, sphere and rock. Only the demanding scene adds build input usage to positions and indices. The ordinary scene's submitted bytes match the retained pre-change SHA-256. |
| M2.7 | Pass | The two contended runs record machine load, a matched 15,384,576-byte resident delta, triangle counts and GPU brackets for five mesh structures. The 58,880-triangle terrain edit added an estimated 0.737 and 0.905 ms in its switch submission. The repeat investigated the cold terrain build. The matched controls explain why the small meshes' isolated costs are unresolved. The raw intervals and signed differences are retained. [Cost record and investigation](M2/README.md#valley-build-costs-under-load) |
| M2.8 | Pass | The saved verifier result names a passing shared exact-images run on `f8828d5f3698`. It covers the registered ordinary images for both Mac recipes and Chrome. No registered baseline changed in the milestone. |

## Independent Metal captures

I built the game and cooker from `76f2a88b1e3e`, cooked the checked-in showcase and captured the submitted SDR output on Apple M3 Max, Metal, Apple tile, with ray queries enabled. Each pair puts the drawn image on the left and the traced image on the right. I inspected reduced whole-frame pairs and unscaled contact crops. The linked whole frames retain all 1280 × 720 pixels per arm.

| Fixed camera | Whole frames | Native contacts |
| --- | --- | --- |
| Level, noon | [Frames](M2-review/valley-frames.png) | [Crop](M2-review/valley-contacts.png) |
| Down 13° | [Frames](M2-review/contact-frames.png) | [Crop](M2-review/contact-contacts.png) |
| West 12°, down 8° | [Frames](M2-review/west-frames.png) | [Crop](M2-review/west-contacts.png) |
| East 13°, down 8° | [Frames](M2-review/east-frames.png) | [Crop](M2-review/east-contacts.png) |
| Level, sunrise at 06:06 | [Frames](M2-review/sunrise-frames.png) | [Crop](M2-review/sunrise-contacts.png) |

The sunrise pair changes the raster lighting substantially while preserving the same ridge and foreground silhouettes. Distance, low Sun, and turned cameras were covered. Storm, fast-camera motion and panel refresh were untested in these fixed geometry inspections. I used the script's saved results for the project gate and M2.2–M2.8 on `f8828d5f3698`. Only orchestration records changed between that checkout and the review checkout.
