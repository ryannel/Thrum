# Ray tracing: traced shadows and ambient occlusion on the Mac

17 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac) · [the brief](../BRIEF.md) ·
[planning probe](../../evidence/RAY-TRACING/planning-probe/README.md). C04, C06, C07, C08 and C12 in
[CONTRACTS](../../design/CONTRACTS.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

## The request

The owner, 17 November 2026:

> Just do ray tracing and mesh shaders on mac, let's leave PC for now.

The lead's task for this plan, in part: "Plan hardware ray tracing (Metal ray queries / intersection) and mesh
shaders (Metal object and mesh stages) on the Apple tile recipe, verified on this M3 Max, under the
hardware-native decision: registry-selected accelerated paths, the portable path stays as fallback and
reference, and each path is promoted only by a measured win. Decide from first principles and current state of
the art which rendering problems in Thrum they should serve ... It must not delay V1 (SHOWCASE M8) ... Desktop
high / PC stays out of scope; keep designs reusable for a later Vulkan/D3D12 recipe where that costs nothing."

Mesh shaders are [MESH-SHADING](../MESH-SHADING/DESIGN.md). This design is ray tracing.

## Purpose and scope

On a Mac whose GPU traces rays, a scene can ask for the light's visibility to be found by rays against the
scene's own triangles. The player sees shadows that are right at any distance and soften as real ones do, lamps
that cast shadows, and props that sit on the ground. Everything else in the frame is unchanged.

**What the renderer does today, and where it falls short.** These are the problems rays are chosen to serve.

| Today | Shortfall |
|---|---|
| The Sun's shadow on a surface comes from two 1024² cascades with a blocker search, and beyond them from a top-down caster height map marched toward the Sun (`sun_visibility` in `synthetic.wgsl`) | Detail is bounded by a cascade's texels. A wedge of terrain at low Sun needed its own correction (`low_sun_terrain`). Bias trades acne for a gap at contact |
| Lamps (local lights) are clustered and cast no shadow | A lamp lights the far side of a wall |
| Sky light on a surface is the sky's irradiance times the material's own occlusion texture (`indirect_radiance`) | Nothing darkens where two objects meet, so props float in shade and under cloud |

| User | Needs | What that requires |
|---|---|---|
| A game or demo on the Mac | Turn each effect on for a scene, see what it resolved to, and pay nothing when it is off | Scene-level requests, a resolved report, no resource without a request |
| The showcase valley's Apple row, after the exit | The paths that measured better, selected without being asked | A promotion rule and the record it reads |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | No shader text, pipeline, resource or byte changes without a request |
| A later Vulkan or D3D12 recipe | The same paths | Everything goes through wgpu's public API and WGSL; nothing names Metal |

**Commissioned**, on `mac-metal`:

1. A ray-tracing structure of everything a scene draws as opaque geometry, at the pose each frame draws it.
2. The Sun's shadow on surfaces by ray, as a method of the scene's one `sun_visibility` operation.
3. Shadows from lamps, as an effect a scene asks for.
4. Soft Sun shadows: the Sun's disc sampled by rays and filtered over time and space.
5. Ambient occlusion by ray.
6. A paired record of each against the portable path, and promotion by the rule below.

**Not built**, with the reason:

- **Reflections by ray, and global illumination.** Both shade what a ray hits: they need a hit's material,
  its vertex attributes and its lighting inside the tracing shader, which the visibility effects here do not.
  Reflection's consumer is water, whose mirror lookup is `WATERS` M14 on `SCENE-VIEWS`; neither is built. No
  material in the showcase valley is smooth enough to mirror its surroundings. Global illumination is a
  radiance cache with state of its own. Each is a later plan on the structure this module builds
  ([open for the human](PLAN.md#open-for-the-human)).
- **Rays for volumes.** The air's Sun shadow, and any fog or cloud sample, reads the caster height map and the
  cascades as today. A volume takes many samples along each view ray, which rays cannot pay for.
- **Rays in a secondary view.** A view of `SCENE-VIEWS` keeps its own cascade and the map method, whatever the
  main view resolved. Reading the structure there is a proposal for that module's next plan.
- **Shadows cut by a material's alpha.** No shadow pass tests alpha today, so the structure treats every
  triangle as opaque and both methods agree.
- **Water, clouds and anything drawn after opaque geometry** as occluders. Clouds shade through the Sun-path
  optical depth, as today.
- **Chrome and the iPhone simulator.** Neither offers ray queries. A physical iPhone with a ray-tracing GPU
  would resolve the same paths from the same registry facts; none is available, so nothing is claimed.

## Cues

| Cue | Where it must read | Range | A plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **R1 A shadow is where the geometry puts it** | Every opaque surface the Sun or a lamp lights | Contact to the far ridge; Sun from overhead to the horizon; movers; mirrored and scaled instances; an edited mesh | A shadow that is right in a still and a frame late in motion, or cast by last frame's mesh. Rejected by an `f64` ray reference at the frame's own poses, in every frame of a scripted run |
| **R2 No acne, no gap, no leak** | Receivers at grazing light, thin casters, where a prop meets the ground | Sun elevation 1° to 90°; receivers 0.5 m to 20 km from the camera | A bias tuned on one view. Rejected by inspection across the range and by reference points placed at contact |
| **R3 A shadow softens with distance from its caster** | A balloon's shadow on the valley floor, a ridge's shadow at low Sun, a prop's own base | Caster 0.1 m to 3 km from the receiver; the Sun's disc cut by a ridge at sunset | A blur of constant width, or grain in place of softness. Rejected by penumbra widths against the Sun's angular size and by frames inspected at rest and in motion |
| **R4 Things are grounded** | Under and between props, in creases of terrain, in shade and under an overcast sky | Occluders from contact to the scene's declared radius | Darkening that follows the screen and not the scene: halos at silhouettes, occlusion that vanishes when the occluder leaves the view. Rejected by turning the camera so the occluder is off screen |
| **R5 Nothing else moves** | The frame with no request; Chrome; the simulator | Every scene on main | A request that defaults on. Rejected by the exact baselines and by counting created resources |

The bar is ray-traced shadows and occlusion as shipped high-end games show them, such as Cyberpunk 2077's
ray-traced Sun and local shadows and Metro Exodus Enhanced Edition's grounded ambient light, smooth at the
display's full refresh. R1 is held to something stricter and checkable: agreement with a ray cast on the CPU.

## Research and selection

Read for this design, 17 November 2026:

- **The locked wgpu reaches both features on Metal.** `wgpu-hal` 30.0.1's Metal adapter sets
  `EXPERIMENTAL_RAY_QUERY` from the device's ray-tracing support and builds acceleration structures
  (`metal/adapter.rs`, `metal/command.rs`); naga writes ray queries to MSL. wgpu's own `ray_shadows` example
  traces from a fragment stage. [The planning probe](../../evidence/RAY-TRACING/planning-probe/README.md) ran
  on this Mac: the adapter offers ray queries; a 10,000-instance structure rebuilt in about 2 ms of wall time;
  two million any-hit rays from a compute stage and from a fragment stage agreed with each other and with an
  `f64` reference on every ray judged. So no raw Metal is needed, and the
  [boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md)'s rule holds: public wgpu for all it can
  express.
- **What rays are spent on in shipped games.** Shadows and ambient occlusion are the cheapest uses, because a
  ray only has to report whether it hit anything. Reflections and global illumination shade the hit. On Apple
  silicon the shipped examples that trace rays (Cyberpunk 2077 and Assassin's Creed Shadows on M3 and M4
  machines) run their ray-traced presets at about 30 to 50 frames a second on an M4 Max in press measurements,
  which is far from this project's bar of the display's full refresh. So this module starts with the uses that fit a
  frame at 120 Hz and leaves hit shading out.
- **One ray a pixel, filtered.** Shipped ray-traced soft shadows trace at most one jittered ray a pixel toward
  the light's disc and filter the result over time and space: AMD's
  [FidelityFX Shadow Denoiser](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/denoiser/) (MIT) and
  NVIDIA's SIGMA do this. Ambient occlusion is traced the same way at lower resolution, as Microsoft's
  real-time denoised ambient occlusion sample (MIT) does.
- **Apple's hidden surface removal.** A tile GPU shades each opaque pixel once whatever the draw order, so a
  ray traced inside the forward pass's fragment stage costs one ray a pixel with no depth prepass. That is why
  the hard shadow is traced inline, and why this design adds no prepass.
- **Self-intersection.** Wächter and Binder, "A Fast and Robust Method for Avoiding Self-Intersection" (Ray
  Tracing Gems, chapter 6): offset the ray's origin along the geometric normal by an amount that scales with
  the point's coordinates. It replaces a tuned bias.

**What was selected, and what else was weighed.**

| Decision | Selected | Weighed and not selected |
|---|---|---|
| Where the hard Sun shadow is traced | Inline, in the fragment that is being shaded, this frame | A screen-space mask pass before shading: it needs a depth prepass, a second geometry pass on a tile GPU |
| How a soft shadow is made | One ray a pixel toward a point on the disc, in a pass after opaque geometry, filtered over time and space, and read by the next frame's shading through reprojection | The same rays integrated by TAAU alone: at mid-penumbra one ray is right half the time, and a history of some twenty frames leaves about a tenth of the Sun's light as grain. A fourth forward target for demodulated Sun light: it changes every forward pipeline in the engine. MetalFX's denoising upscaler: it needs albedo, normal and roughness targets and belongs to `APPLE-PRESENTATION`; it is what would change this selection |
| Where ambient occlusion is traced | In the same pass as the soft shadow, at half the extent | Inline in shading: one ray a pixel is grain over every shaded surface |
| How many structures | One for the scene, rebuilt each frame | One for still geometry and one for movers: this adapter has room, but every ray would walk both. It is the named candidate if the rebuild shows as a tripwire |

## The oracle

A **reference ray** is cast on the CPU in `f64` against the triangles the frame draws: each mesh's triangles,
transformed by the pose the frame gives its instance. It returns whether anything is hit within a distance, the
nearest hit's distance and which entity or placement was hit. It reads nothing back from the GPU.

GPU intersection is in 32-bit floats, so a ray that passes within a band of a triangle's edge may fall either
side. A test derives that band from the spacing of 32-bit floats at the scene's largest coordinate, does not
judge rays inside it, and records what share it skipped. Every other ray must agree exactly on hit or miss.

Hard shadows are judged by the oracle. Soft shadows and ambient occlusion are filtered estimates: their
automated proofs are the estimator's mean against the reference's fraction at points that hold still, and their
look is judged by inspection.

## Mechanisms

### Hardware features at launch

The registry already records the features a device generation enabled. This module adds the request.

- A launch names the hardware features it wants: `ray-query` here, `mesh-stages` for `MESH-SHADING`. The
  diagnostic route is the environment variable `THRUM_HARDWARE`, a comma-separated list, read as `THRUM_RECIPE`
  is. A host may pass the same list in code.
- `DeviceCapabilities` turns the list and the adapter into the features, limits and wgpu experimental-features
  token for the device request, before the device is made, as `requested_timing_features` does for timing. A
  name the engine does not know, or a feature the adapter does not offer, is refused by name. Nothing is
  dropped silently.
- With no request the device is the one made today: no experimental feature, the same limits. So every exact
  baseline and the V1 exit's three rows are untouched until [promotion](#selection-fallback-and-promotion).
- Chrome has no request route and the iPhone app does not ask.

`MESH-SHADING` adds its feature to the same function and the same list. Whichever module's first milestone
lands second extends what the first built.

### The structure (M1, M2)

Two kinds of structure, in wgpu's terms a `Blas` and a `Tlas`:

- A **mesh structure** holds one mesh's triangles in the mesh's own space. It is part of that mesh's
  residency: built when the mesh's vertices and indices are on the GPU, replaced when they are, and retired
  with them. It reads the buffers the mesh is drawn from, so no geometry is copied.
- The **scene structure** holds one instance for each thing the frame draws: a mesh structure, a transform,
  an identity and a mask. It is rebuilt every frame from what that frame draws.

**Entities (M1).** Each row of the mesh arena gets a mesh structure, built from the arena's vertex and index
buffers at the row's range. On a ray-query generation those buffers carry the usage a structure build needs.
The scene structure takes one instance for each active slot, with the model matrix the vertex stage uses:
`current`, with its translation mixed from `previous` by the frame's presentation alpha. An inactive slot has
no instance. So a ray meets a mover where the frame draws it.

**Cooked placements (M2).** Each resident cooked mesh gets a mesh structure, built from its position and
index buffers in the frame that records the mesh's last copy, after the copy and before any draw. A
contribution offers its instances for the frame through one new optional method of the contribution seam:
for each placement it draws, the mesh structure, the transform and the mask. `thrum-render` gains no
dependency on `thrum-asset-gpu`.

**Edits (M2).** [A content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) needs no new
kind of reader. A replacement row's mesh structure is built in the frame that records the row's last write, so
the row is ready when it is today. A replacement placement's is built in the frame that records its last copy.
The scene structure is built from the set the frame draws, so the switch frame traces the new geometry and no
frame traces a mix. A discarded frame commits nothing: the scene structure is rebuilt next frame, and a mesh
structure built in a discarded frame is built again.

**Masks.** An instance's mask says what it does: bit 0, it casts shadows; bit 1, it occludes ambient light.
An instance casts a shadow by ray exactly when the portable shadow passes would draw it.

**Lifetime.** Structures belong to the device generation. A replacement device builds every mesh structure
before the scene is ready, as it prepares pipelines, and no frame traces before the first scene structure is
built. The build is a pass the frame graph knows: it writes the scene structure, and every pass whose shader
reads the structure depends on it.

**When nothing asks.** A scene that requests no ray effect has no structure, no build pass and no usage
change, on any device.

### Traced shadows (M3)

**The Sun.** A scene's Sun shadow has a method: `Maps`, the cascades and height map of today, or `Traced`.
With `Traced`, the scene's composed shader holds a `sun_visibility` that traces, and the light group holds the
scene structure. The operation's callers do not change. The traced operation:

- returns 1 with no ray when the surface faces away from the Sun, which is unlit anyway;
- offsets the origin along the geometric normal, taken from the fragment's world-position derivatives, by
  Wächter and Binder's rule;
- traces one any-hit ray toward the Sun's centre, against instances whose mask casts shadows, with every
  triangle opaque, and returns 0 or 1.

Terrain is in the structure, so one ray answers at every distance and at low Sun. The traced method does not
use the cascades, the blocker search, `horizon_visibility` or `low_sun_terrain`.

**What is still drawn.** The two cascade passes are drawn only while something reads them. Their only readers
today are surfaces, so a scene on `Traced` draws none. The top-down caster height map is still drawn, because
the air's shadow heights are reduced from it. When a volume reader of the cascades lands (`ATMOSPHERE`'s view
volume, `WATERS`' lent Sun visibility), the cascades are drawn for it under either method.

**A hard shadow has no partial Sun.** The centre ray gives a hard edge everywhere, and at sunset a receiver's
Sun goes out when the disc's centre meets the ridge. That is M3's stated limit. M4 removes it.

**Lamps.** A scene may ask for lamp shadows. Then, in `local_radiance`, each lamp in the fragment's cluster
list that is in range and that the surface faces traces one any-hit ray to the lamp's position, stopping short
of it. A lamp has no size in the engine, so its shadow is hard. The count of rays a fragment may trace for
lamps is a runtime quality value whose default is the cluster list's capacity; a lamp beyond it is unshadowed
at that fragment, and counted.

**Shader text.** The traced operations and the `enable wgpu_ray_query` directive are in the composed shader
only for a scene that asked. Every contribution that compiles the scene's shader takes them from it, as it
takes the cloud factor.

### Soft Sun shadows (M4)

A scene on `Traced` may turn the penumbra on. Then an engine pass, the **traced visibility pass**, runs after
the frame's opaque geometry:

1. **Trace.** For each pixel that holds opaque geometry it rebuilds the world position from the frame's depth
   and view record and the geometric normal from neighbouring depths, picks a point on the Sun's disc from a
   sequence that differs from pixel to pixel and frame to frame, and traces one any-hit ray toward it. The
   disc's radius is the directional source's own.
2. **Over time.** It reprojects last frame's result through the frame's motion target, rejects history whose
   depth disagrees, keeps the first two moments, and clamps history to the range of the current neighbourhood,
   so a shadow that moved does not trail.
3. **Over space.** Edge-stopping passes, guided by depth and normal, whose width follows the variance. Where
   every sample agrees, as at a prop's base, nothing is blurred and the edge stays sharp.

It follows the FidelityFX Shadow Denoiser, whose source may be ported.

The result is a history texture: Sun visibility, and the depth it belongs to. **The next frame's shading reads
it.** A fragment already has last frame's clip position for its motion vector. It reads the history there,
checks the stored depth against its own, and uses the value. Where the history is absent or does not match, it
traces the hard centre ray inline, as M3 does. So the soft value is one frame late, and a pixel with no
history is hard for one frame.

The history is committed only with a submitted frame and reset on a camera cut, a change of render extent and
a device replacement, by `APPLE-PRESENTATION`'s history rules. Fragments drawn after opaque geometry (water)
have no history and trace inline.

### Ambient occlusion (M5)

A scene may ask for ambient occlusion with a radius in metres. The traced visibility pass then also traces,
at half its extent, one ray for each pixel in a cosine-weighted direction about the geometric normal, no longer
than the radius, against instances whose mask occludes. It accumulates and filters the result as it does the
Sun's, in a second channel, and shading reads it the same way, upsampled against depth. Where history is
absent the value is 1.

The value multiplies the ambient term only: the diffuse sky light, and the reflected sky light through
Lagarde's specular occlusion. It never touches the Sun or a lamp. It multiplies the material's own occlusion
texture and does not replace it.

### Selection, fallback and promotion

- **Requests.** A scene's preparation takes: the Sun shadow method (`Auto`, `Maps`, `Traced`), penumbra on or
  off, lamp shadows on or off, ambient occlusion off or a radius. They sit beside the scene's existing shadow
  option, and like the culling mode they are fixed when the scene is prepared.
- **Resolution.** `Auto` resolves to `Maps` unless the registry lists the traced method as promoted for the
  resolved recipe and the generation enabled ray queries. An explicit `Traced`, lamp shadows or ambient
  occlusion on a generation without ray queries is refused by name at preparation. A request is never
  silently downgraded.
- **Report.** What each resolved to is reported where the culling mode is, so
  [a run can say what it ran](../ARCHITECTURE.md#interfaces-track-5).
- **Everything is prepared.** Pipelines for the resolved methods exist before the scene is ready. No method is
  switched while a scene runs.
- **Quality values.** Penumbra, lamp-ray count, ambient occlusion and its resolution are runtime values
  declared through the contribution seam's quality knobs, each with the history it invalidates, for the
  governor.
- **Promotion (M6).** The traced Sun shadow with its penumbra is promoted on the Apple tile recipe only if, in
  M6's paired sitting, the valley's whole frame costs no more with it than with `Maps`, beyond the spread
  between two runs of the same configuration, in both runs. Until then the promoted list is empty and `Auto` is
  `Maps`. Lamp shadows and ambient occlusion add what the portable frame does not have, so they are never
  promoted: a scene asks for them.
- **What promotion changes.** Once a path is promoted, the Mac window asks for its hardware feature itself
  when its recipe is Apple tile. A capture and a test ask only for what they name. So the verifier's runs
  enable no feature and resolve to the reference methods, and no registered exact baseline moves at promotion.
  M6 waits for `SHOWCASE` M8, so the window's default does not change under the V1 exit either.

### Check that it can work

- **The API path.** [The probe](../../evidence/RAY-TRACING/planning-probe/README.md) built, rebuilt and
  traced through the locked wgpu on this Mac, from both stages this design traces from.
- **Poses.** Every instance transform the vertex stage uses is on the CPU when the frame is recorded: the
  instance record's two matrices and the frame's alpha. The scene structure is built from the same values.
- **Penumbra range.** The Sun's disc is about 0.53° across, so a penumbra is about 0.0093 of the distance to
  the caster: 2 cm at 2 m, which is under a pixel beyond a few metres and stays sharp; 0.9 m under a balloon
  100 m up; 28 m from a ridge 3 km off. A wide penumbra changes slowly across the screen, which is the case
  the variance-guided filter is made for, and a setting Sun is the same estimate with part of the disc hidden.
- **Scale.** The representative scene is 10,000 entities with 1,000 to 2,000 movers. A rebuild is linear in
  instances. M1 records it at 10,000 and at the field's 100,513, and claims nothing at the larger count.
- **What an edit costs.** A mesh structure cannot be built in parts, so an edit to a mesh costs its whole
  build in one frame. M2 records that time against triangle count. `CONTENT-LOOP`'s promise of no late frame
  on an edit stands for the portable method; on a scene with a structure it is the recorded build.

## The system as it runs

1. **Launch.** The host asks for `ray-query`. The device enables it and the registry records it.
2. **Preparation.** The scene resolves its requests. If any needs rays, the arena and each resident cooked
   mesh build their mesh structures, the shader is composed with the traced operations, and pipelines are
   created through the device's creation authority.
3. **Each frame.** Uploads are reconciled. The structure pass sets one instance for each drawn entity and each
   placement a contribution offers, and rebuilds the scene structure. The caster height map is drawn; cascades
   only if something reads them. The forward pass shades: each fragment reads the soft history where it is
   valid and traces inline where it is not. After opaque geometry, the traced visibility pass traces, filters
   and writes the history the next frame reads.
4. **Submit.** Only a submitted frame commits the history.
5. **Loss or replacement.** The new generation rebuilds every structure and resets the history before its
   first frame.

## What this module provides

Defined here, listed in [the module list](../ARCHITECTURE.md#interfaces-track-5):

- **Hardware features at launch**, with `MESH-SHADING`.
- **Ray instances**: how a contribution puts its geometry in the scene structure.
- **Traced visibility**: the Sun shadow method and the lamp-shadow and ambient-occlusion requests of a scene,
  and what they resolve to.

## Shared files and exact baselines

This module builds inside state other modules own, under
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5). Its own files are a `raytrace`
module in `thrum-render` and its tests. With no request every byte on main is unchanged, and each milestone
proves it through [the exact baselines](../ARCHITECTURE.md#exact-baselines). No baseline is added for a traced
frame: its proofs are the oracle and inspection.

## Cost account

Recorded, never a gate (brief decision 3). The tripwires are the brief's: a visible hitch, a comparable figure
about 25% worse than its last record, growth that is not linear in what it should follow.

- **Structure memory.** One mesh structure for each mesh, not each instance, plus an instance record for each
  drawn thing. M1 and M2 record resident bytes.
- **Each frame, CPU.** One instance write for each drawn entity and placement.
- **Each frame, GPU.** The scene structure's rebuild. Hard Sun: at most one ray for each shaded opaque pixel
  that faces the Sun. Soft Sun: one ray for each opaque pixel in the visibility pass, and inline rays only
  where history is missing. Lamps: one ray for each in-range lamp a pixel faces. Ambient occlusion: one ray
  for each four pixels. The filter's passes.
- **Saved on `Traced`.** The two cascade passes and the blocker search, while no volume reads the cascades.

## Risks

1. **wgpu's ray tracing is experimental.** A defect a builder cannot work around inside Thrum ends the
   milestone `BLOCKED` with the smallest program that shows it. No pinned copy of wgpu exists before
   `APPLE-PRESENTATION` M10, and nothing is posted upstream without the owner.
2. **The rebuild grows with the scene.** Recorded from M1. The candidate is a second structure for still
   geometry.
3. **A soft shadow trails a fast mover.** The clamp bounds it; M4 inspects it. If it fails, the penumbra stays
   off by default and the traced method is not promoted.
4. **Normals rebuilt from depth are wrong at silhouettes.** The filter's edge stops bound it; M5 inspects it.
5. **Other builders are in the same files.** `SCENE-VIEWS`, `ATMOSPHERE`, `APPLE-PRESENTATION`, `HOST`,
   `REVIEW-RENDER` and `MESH-SHADING` all work in `thrum-render`, `thrum-app` or `thrum-asset-gpu`. This
   module's changes add to those files; a builder integrates on what has landed and rewrites nothing another
   milestone holds.

## Proof and builder handoff

- GPU tests ask `thrum-test-support`'s gate for a device with the hardware request, and are skipped with a
  reason where the adapter offers no ray queries. On this Mac the feature is required, so a skip fails.
- Tests that need only entities live in `thrum-render`'s tests. Proofs that need cooked meshes or the valley
  run in the proving ground through a script, so a check names no game package.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, in the
  proving ground's valley with each effect behind a selection that is off by default.
- Timing inside M1 to M5 is a tripwire taken under load. M6 takes the paired numbers in one sitting with
  other builders paused; its builder asks the lead for it.
- Scratch goes in `tmp/ray-tracing-<milestone>/`, selected evidence in `docs/evidence/RAY-TRACING/`.

## Open questions

None that blocks a milestone. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
