# Ray tracing: traced shadows and ambient occlusion on the Mac

19 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac) · [the brief](../VISION.md) ·
planning probes: [the API](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md),
[the ray's origin](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/OFFSET.md) and
[a rounded mesh's terminator band](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/PATCH.md).
C04, C06, C07, C08 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) and
[the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) govern.

## The request

The owner, 17 November 2026:

> Just do ray tracing and mesh shaders on mac, let's leave PC for now.

The lead's task for this plan, in part: "Plan hardware ray tracing (Metal ray queries / intersection) and mesh
shaders (Metal object and mesh stages) on the Apple tile recipe, verified on this M3 Max, under the
hardware-native decision: registry-selected accelerated paths, the portable path stays as fallback and
reference, and each path is promoted only by a measured win. Decide from first principles and current state of
the art which rendering problems in Thrum they should serve ... It must not delay V1 (SHOWCASE WP8) ... Desktop
high / PC stays out of scope; keep designs reusable for a later Vulkan/D3D12 recipe where that costs nothing."

The owner, the same day: "Ok let's lower the bar." Visual work passes on a shipped-game look and on speed. A
physical or `f64` reference is a diagnostic and never a pass mark. Proof is in proportion to risk.

The lead, 18 November 2026, after two design reviews found faults in a rule that promoted the traced path by
itself: "the traced path is an explicit, recorded opt-in for this Mac, judged by eye against the inspection
list, with a paired whole-frame timing kept as a tripwire, and a plain fallback to the portable path when its
feature is missing. Drop adapter-keyed automatic promotion and recipe-difference crediting." So this module
promotes nothing. The measured win the lead's task asks of a default is owed by whoever later makes one. The
timing the lead calls whole-frame is taken as two figures, [a timed-pass span and the frame's CPU time](#cost-account),
and the span is named for what it measures.

Mesh shaders are [MESH-SHADING](../MESH-SHADING/DESIGN.md). This design is ray tracing.

## Purpose and scope

On a Mac whose GPU traces rays, a scene can ask for the light's visibility to be found by rays against the
scene's own triangles. The player sees shadows that hold at any distance and soften as real ones do, lamps
that cast shadows, and props that sit on the ground. Everything else in the frame is unchanged.

**What the renderer does today, and where it falls short.** These are the problems rays are chosen to serve.

| Today | Shortfall |
|---|---|
| The Sun's shadow on a surface comes from two 1024² cascades with a blocker search, and beyond them from a top-down caster height map marched toward the Sun (`sun_visibility` in `synthetic.wgsl`) | Detail is bounded by a cascade's texels. A wedge of terrain at low Sun needed its own correction (`low_sun_terrain`). Bias trades acne for a gap at contact |
| Lamps (local lights) are clustered and cast no shadow | A lamp lights the far side of a wall |
| Sky light on a surface is the sky's irradiance times the material's own occlusion texture (`indirect_radiance`) | Nothing darkens where two objects meet, so props float in shade and under cloud |

| User | Needs | What that requires |
|---|---|---|
| A game or demo on the Mac | Turn each effect on for a scene, see what it resolved to, pay nothing when it is off, and still draw where rays cannot run | Scene-level requests that fall back to the map frame, a resolved report, no resource without a request |
| A person running a scene on this Mac | Turn the traced path on for the machine with one launch setting and nothing else, never have a launch fail for it, and know how it looks and what it costs as launched | A launch request that reaches the Apple tile recipe by itself and falls back, and this Mac's record of that launch |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | No shader text, pipeline, resource or byte changes there, whatever the device offers |
| A later Vulkan or D3D12 recipe | The same paths | Everything goes through wgpu's public API and WGSL; nothing names Metal |

**Commissioned**, on `mac-metal`:

1. A ray-tracing structure of every opaque thing in a scene, at the pose each frame draws it.
2. The Sun's shadow on surfaces by ray, as a method of the scene's one `sun_visibility` operation.
3. Shadows from lamps, as an effect a scene asks for.
4. Soft Sun shadows: the Sun's disc sampled by rays and filtered over time and space.
5. Ambient occlusion by ray.
6. An opt-in a person makes for a machine with one setting, a fallback to the map frame wherever rays cannot
   run, and this Mac's record: how the final build looks as that setting launches it, and a paired frame
   timing beside shadow maps.

**Not built**, with the reason:

- **A default.** No ray effect is on unless a launch and a scene ask for it, on this Mac or any other. Making
  one a default is a later decision. Under [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)
  it needs a measured win, which this module does not claim
  ([open for the human](PLAN.md#open-for-the-human)).

- **Reflections by ray, and global illumination.** Both shade what a ray hits: they need a hit's material,
  its vertex attributes and its lighting inside the tracing shader, which the visibility effects here do not.
  Reflection's consumer is water, whose mirror lookup is `WATERS` WP14 on `SCENE-VIEWS`; neither is built. No
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
- **Ray effects on the portable recipe.** The portable recipe is the reference, on this Mac too, so it has
  none. A scene that asks there draws the map frame. The opt-in reaches Apple tile by itself only where no
  recipe is named.
- **Chrome and the iPhone simulator.** Neither offers ray queries. A physical iPhone with a ray-tracing GPU
  would resolve the same paths from the same registry facts; none is available, so nothing is claimed.

## Cues

| Cue | Where it must read | Range | A plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **R1 A shadow is where the geometry puts it** | Every opaque surface the Sun or a lamp lights | Contact to the far ridge; Sun from overhead to the horizon; movers; mirrored and scaled instances; an edited mesh; a caster the camera cannot see; a caster joined into its receiver's own mesh; a room lit only by lamps, in a scene with Sun shadows off | A shadow that is right in a still and a frame late in motion, cast by last frame's mesh, or lost when its caster leaves the view. Rejected by consecutive frames around movers, by the traced view, and by rays with plain answers |
| **R2 No acne, no gap, no leak** | Receivers at grazing light, thin casters, where a prop meets the ground | Sun elevation 1° to 90°; receivers 0.5 m to 20 km from the camera; coordinates to 20 km from the scene's origin | A bias tuned on one view, or one so large that it steps over a nearby caster. Rejected by inspection across the range and by a panel close to a wall, far from the origin |
| **R3 A shadow softens with distance from its caster** | A balloon's shadow on the valley floor, a ridge's shadow at low Sun, a prop's own base, another body's shadow across a rounded prop | Caster 0.1 m to 3 km from the receiver; the Sun's disc cut by a ridge at sunset | A blur of constant width, or grain in place of softness. Rejected by frames inspected at rest and in motion |
| **R4 Things are grounded** | Under and between props, in creases of terrain, in shade and under an overcast sky | Occluders from contact to the scene's declared radius | Darkening that follows the screen and not the scene: halos at silhouettes, occlusion that vanishes when the occluder leaves the view. Rejected by turning the camera so the occluder is off screen |
| **R5 Nothing else moves** | The frame with no request; Chrome; the simulator; the portable recipe; a Mac that cannot trace | Every scene on main | A request that defaults on, a launch that fails where rays are missing, or an opt-in that draws maps on a Mac that traces because nothing chose its recipe. Rejected by the exact baselines, by counting created resources, and by launching the one setting where it can be met and where it cannot |

The bar is ray-traced shadows and occlusion as shipped high-end games show them, such as Cyberpunk 2077's
ray-traced Sun and local shadows and Metro Exodus Enhanced Edition's grounded ambient light, smooth at the
display's full refresh.

## Research and selection

Read for this design, 17 and 18 November 2026:

- **The locked wgpu reaches both features on Metal.** `wgpu-hal` 30.0.1's Metal adapter sets
  `EXPERIMENTAL_RAY_QUERY` from the device's ray-tracing support and builds acceleration structures
  (`metal/adapter.rs`, `metal/command.rs`); naga writes ray queries to MSL. wgpu's own `ray_shadows` example
  traces from a fragment stage. [The planning probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md) ran
  on this Mac: the adapter offers ray queries; a 10,000-instance structure rebuilt in about 2 ms of wall time;
  two million any-hit rays from a compute stage and from a fragment stage gave the same answers, and the
  right ones. So no raw Metal is needed, and the
  [boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md)'s rule holds: public wgpu for all it can
  express.
- **What rays are spent on in shipped games.** Shadows and ambient occlusion are the cheapest uses, because a
  ray only has to report whether it hit anything. Reflections and global illumination shade the hit. On Apple
  silicon the shipped examples that trace rays (Cyberpunk 2077 and Assassin's Creed Shadows on WP3 and WP4
  machines) run their ray-traced presets at about 30 to 50 frames a second on an M4 Max in press measurements,
  which is far from this project's bar of the display's full refresh. So this module starts with the uses that fit a
  frame at 120 Hz and leaves hit shading out.
- **One ray a pixel, filtered.** Shipped ray-traced soft shadows trace at most one jittered ray a pixel toward
  the light's disc and filter the result over time and space: AMD's
  [FidelityFX Shadow Denoiser](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/denoiser/) (MIT) and
  NVIDIA's SIGMA do this. Ambient occlusion is traced the same way at lower resolution, as Microsoft's
  real-time denoised ambient occlusion sample (MIT) does. Both run in a deferred renderer, after depth and
  before lighting. This renderer is forward, which is what
  [the soft shadow's design](#soft-sun-shadows-wp4) has to answer.
- **Apple's hidden surface removal.** A tile GPU shades each opaque pixel once whatever the draw order, so a
  ray traced inside the forward pass's fragment stage costs one ray a pixel with no depth prepass. That is why
  the hard shadow is traced inline, and why this design adds no prepass.
- **Self-intersection.** Wächter and Binder, "A Fast and Robust Method for Avoiding Self-Intersection" (Ray
  Tracing Gems, chapter 6), lift a ray's origin along the normal by an amount that scales with the point's
  coordinates, in place of a tuned bias. Their constant, 256 float steps, allows for a hit point computed at
  the end of a long ray. It is 0.25 m at 10 km from the origin, which steps over a nearby caster. A raster
  fragment's position carries far less error. [The second probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/OFFSET.md)
  measured it on this Mac: two float steps cleared every self-hit from 64 m to 20 km and down to half a degree
  of grazing light.
- **The shadow terminator.** A ray meets a coarse mesh's flat triangles, and shading reads its smooth
  normals. Where the two disagree about which way the surface faces, a whole triangle is dark that its
  shading calls lit. Hanika, "Hacking the Shadow Terminator" (Ray Tracing Gems II, chapter 4), moves the ray's
  origin onto the curved surface the vertex normals imply. That needs the vertex normals at the receiving
  point. A fragment has them. A pass that rebuilds its point from depth does not, which is what
  [the passage](#a-rounded-meshs-own-facets) has to answer.

**What was selected, and what else was weighed.**

| Decision | Selected | Weighed and not selected |
|---|---|---|
| Where the hard Sun shadow is traced | Inline, in the fragment that is being shaded, this frame | A screen-space mask pass before shading: it needs a depth prepass, a second geometry pass on a tile GPU |
| How a soft shadow is made | One ray a pixel toward a point on the disc, in a pass after opaque geometry, filtered over time and space and read by the next frame's shading, beside a centre ray traced inline this frame that overrides a settled history it contradicts | The history alone: a moving caster's whole shadow is then a frame late, and detaches from a fast mover. The same rays integrated by TAAU alone: at mid-penumbra one ray is right half the time, and a history of some twenty frames leaves about a tenth of the Sun's light as grain. A fourth forward target for demodulated Sun light, which would make the soft value current: it changes every forward pipeline in the engine. MetalFX's denoising upscaler: it needs albedo, normal and roughness targets and belongs to `APPLE-PRESENTATION`; it is what would change this selection |
| Where ambient occlusion is traced | In the same pass as the soft shadow, at half the extent | Inline in shading: one ray a pixel is grain over every shaded surface |
| How a coarse rounded mesh loses its false facets | One excused passage through the receiving surface, judged from what the ray itself reports. Inline, the fragment's reach decides it. The visibility pass has no reach, so it records a dipped ray both ways and the fragment picks the value its own passage bears out | Moving the origin onto the implied surface, as Hanika does: right for the inline ray, but the pass has no vertex normals, so its disc rays would still strike the receiver and the penumbra would be lost there. Skipping nearby hits on the receiving instance by their facing, which WP3 first built: it erased a sheet joined into the same mesh. One soft value with the passage excused at any distance: past an open rim it excused a joined panel metres away, and the panel's soft edge went hard. A forward target that carries the implied surface's height or the disc ray's raw result to the pass: it changes every forward pipeline in the engine |
| How many structures | One for the scene, rebuilt each frame | One for still geometry and one for movers: this adapter has room, but every ray would walk both. It is the named candidate if the rebuild shows as a tripwire, and WP1's record of a still scene beside a moving one says what it could save |
| What the structure holds | Every opaque thing in the scene, on screen or off | What the frame's passes draw: a lamp's ray, a Sun ray at low Sun and an occlusion ray all reach geometry that no camera or cascade volume holds |
| What turns a ray effect on | A launch and a scene that ask, with a fallback to the map frame and a record for this Mac | A rule that makes the traced Sun shadow the default where a sitting measured it no slower and better looking, keyed to the adapter. Two design reviews found that it could promote a slower frame from a sum of pass times, credit rays with a difference between two recipes, and qualify a 40-core M3 Max from a 30-core one's sitting under one adapter name. For effects a scene chooses, that machinery guards less than it costs |

## Proof

Each cue is judged by eye, in the running proving ground, against its work package's inspection list and the bar
above. Each cost is a tripwire record. Tests hold the contracts, where a failure would be silent:

- **What the structure holds.** The instance list is compared with the scene's own records on the CPU: which
  things, which mesh, which pose, which mask.
- **Rays with plain answers.** In scenes built for the purpose, a test aims rays whose answer is plain from
  how the scene was built: through the middle of a body, or past it by more than its size. It expects hit or
  miss. No ray is aimed near an edge, so no tolerance is stated and none is needed.
- **The traced view.** A diagnostic pass of this module, which the proving ground can select: primary rays
  from the camera through the scene structure, shaded by which instance each hits and how far away. Beside the
  drawn frame it shows at a glance a mesh that is missing, misplaced, stale or a frame late. It is how the
  structure is inspected, and it stays as the first tool for a shadow that looks wrong.
- **Lifecycle, selection and absence**, as the items say: a replaced device, a discarded frame, an edit, a
  launch and a request that cannot be met, a traced scene that fails to prepare, and a scene that asks for
  nothing.

No item holds a ray, a shader's visibility or an occlusion value to a CPU reference. A builder who needs one to
find a fault may write it, and says so in the evidence.

## Mechanisms

### Hardware features at launch

The registry already records the features a device generation enabled. This module adds the request.

- A launch names the hardware features it wants: `ray-query` here, `mesh-stages` for `MESH-SHADING`. It gives
  each name as **required** or **preferred**. The diagnostic route is the environment variable
  `THRUM_HARDWARE`, a comma-separated list, read as `THRUM_RECIPE` is. How the variable marks a preferred name
  is the builder's choice, and the documentation says. A host may pass the same list in code.
- One function of the registry makes the device for every host. It is given the host's way to get an adapter,
  the device request the host makes today and the launch's list, and it adds to that request the features,
  limits and wgpu experimental-features token the list needs, as `requested_timing_features` adds timing today.
- **A required name is met or the launch is refused.** A name the engine does not know, and a required name
  the adapter does not offer, are refused by name before a device is made. A test and a capture require what
  they name, so neither runs on a device other than the one it asked for.
- **A preferred name never fails a launch.** Where the adapter does not offer it, it is left out. Where the
  adapter offers it and the device request fails, the function asks once more without the preferred names,
  from a fresh adapter as wgpu's contract wants, keeping every required name and everything the host's own
  request held. That second answer, a device or an error, is the launch's. Either way the registry records
  the name as not enabled, with the reason, and [the report](#selection-fallback-and-the-opt-in) shows it.
- With no list the device is the one made today: no experimental feature, the same limits. So every exact
  baseline and the V1 exit's three rows are untouched.
- No host adds `ray-query` by itself, on any adapter. The Mac window asks for it only when its launch names
  it. Chrome has no request route and the iPhone app does not ask.

`MESH-SHADING` adds its feature to the same function and the same list. Whichever module's first work package
lands second extends what the first built.

### The structure (WP1, WP2)

Two kinds of structure, in wgpu's terms a `Blas` and a `Tlas`:

- A **mesh structure** holds one mesh's triangles in the mesh's own space. It is part of that mesh's
  residency: built when the mesh's vertices and indices are on the GPU, replaced when they are, and retired
  with them. It reads the buffers the mesh is drawn from, so no geometry is copied.
- The **scene structure** holds one instance for each opaque thing in the scene: a mesh structure, a
  transform, an identity and a mask. It is rebuilt every frame.

**Demand comes from the scene, before anything is allocated.** A scene's preparation
[resolves its requests](#selection-fallback-and-the-opt-in) first. If any resolved effect needs rays, the scene
has **ray demand**, and that one fact decides everything below. It reaches the mesh arena, which the scene's
preparation creates, and it is a field of the `SceneSurface` every contribution is prepared against. A cooked
contribution starts its loader from that surface, so it knows the demand before it makes a buffer. A scene may
also ask for the structure with no effect, which is what the traced view and a test's probe do.

- With ray demand, the arena's buffers and each cooked mesh's position and index buffers are created with the
  usage a structure build reads, and their mesh structures are built.
- Without it, none of that happens, on any device: no structure, no build pass, no usage change. That
  includes a device that enabled ray queries, and a scene prepared beside a traced one on the same device.
  The arena belongs to its scene's preparation and a cooked contribution's residency to its surface, so two
  scenes on one device share neither.

**Entities (WP1).** With ray demand, each row of the mesh arena gets a mesh structure, built from the arena's
vertex and index buffers at the row's range. The scene structure takes one instance for each active slot, with
the model matrix the vertex stage uses: `current`, with its translation mixed from `previous` by the frame's
presentation alpha. An inactive slot has no instance. So a ray meets a mover where the frame draws it.

**Cooked placements (WP2).** With ray demand, each resident cooked mesh gets a mesh structure, built in the
frame that records the mesh's last copy, after the copy and before any draw. A contribution offers its
instances each frame through one new optional method of the contribution seam: for each opaque placement it
holds in the scene, the mesh structure, the transform and the mask. `thrum-render` gains no dependency on
`thrum-asset-gpu`.

**Membership is the scene's, not a view's.** An instance is in the structure because the thing is in the
scene: an active entity, or a placement the contribution holds. No camera, cascade volume or pass's draw list
is consulted. `CULLING`'s bucket route culls Sun casters against each Sun pass's own volume, and that culling
stays where it is: a wall outside both volumes still stands between a lamp and its receiver, so it is in the
structure. Turning the camera changes no instance.

**Masks.** An instance's mask says what it does: bit 0, it casts shadows; bit 1, it occludes ambient light.
Each is a fact about the thing, and never about a pass. Nothing opaque in the engine is exempt from casting
or occluding today, so every active entity and every cooked placement sets both bits. A later contribution
may clear one for geometry of its own that should not cast or should not occlude.

Bit 0 does not follow the preparation of Sun shadow maps. The scene's `sun_shadows` switch says only whether
the Sun casts shadows, and with the map method that its cascades are prepared. A cooked contribution's shadow
pipeline and its caster index count exist only to draw those maps, and both are absent in a scene without
them. None of the three is read when a mask is set. So in a scene with Sun shadows off, a wall of entities
and a wall that is a cooked placement both stand in the structure as casters, and a lamp's ray stops at them.

**Edits (WP2).** [A content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-wp1-wp2) needs no new
kind of reader. A replacement row's mesh structure is built in the frame that records the row's last write, so
the row is ready when it is today. A replacement placement's is built in the frame that records its last copy.
The scene structure is built from the set the frame shows, so the switch frame traces the new geometry and no
frame traces a mix. A discarded frame commits nothing: the scene structure is rebuilt next frame, and a mesh
structure built in a discarded frame is built again.

**Lifetime.** Structures belong to the device generation. A replacement device builds every mesh structure
before the scene is ready, as it prepares pipelines, and no frame traces before the first scene structure is
built. The build is a pass the frame graph knows: it writes the scene structure, and every pass whose shader
reads the structure depends on it.

### Where a ray starts

Every ray this module traces from a surface starts the same way. It is one function of the `raytrace` module's
shader text.

- **The lift.** The origin is the surface point moved along the surface's normal, toward the side being lit,
  by a count of float steps: the spacing of 32-bit floats at the largest magnitude the point's position went
  through. That is its largest world coordinate, or its distance from the camera where the position was
  rebuilt from depth, and never less than the spacing at 1 km, which covers a mesh whose own coordinates are
  large. The ray starts at distance 0.
- **The count is 8.** The probe needed 2. A builder may raise it to 16 by eye if acne shows, and says so in
  the evidence. It is one constant for every ray.
- **The normal comes from the surface.** Close to the camera and far from the origin a pixel's footprint is
  smaller than a float step, and world-position derivatives become noise. A fragment uses its transformed,
  interpolated normal on the lit side. The visibility pass uses neighbouring depths in view space, where
  nearby positions keep their precision.
- **The supported separation.** A caster always blocks when it stands more than twice the lift's ceiling from
  its receiver: 32 float steps. That is 2 mm within 1 km of the origin, 3.2 cm at 10 km and 6.3 cm at 20 km.
  The factor of two leaves room for the caster's and the receiver's own rounding. Nearer than that, a caster
  thinner than the lift may be stepped over.
- **In the terminator band.** A ray in [the terminator band](#a-rounded-meshs-own-facets) of a rounded mesh
  starts the same lift on the inner side of its triangle. A caster that is another instance blocks it wherever
  it crosses the ray. A part of the receiver's own mesh is sure to block where it stands beyond the body's own
  far face, or above the surface the vertex normals imply: further from the receiving point, along its shading
  normal, than the implied surface's height over the flat triangle. That height is zero on flat and finely cut
  geometry and centimetres on a coarse prop: 7 cm at most on the review's twelve-sided, one-metre prism.
  Inside it there is one case that leaks. Where a single-sided shell ends at an open rim, a ray can pass under
  the rim and meet no far face, and the back of a sheet of the same mesh lying that close can then be taken
  for one.

### Traced shadows (WP3)

**The Sun.** The scene's existing switch says whether the Sun casts shadows at all. Where it does, the shadow
has a method: `Maps`, the cascades and height map of today, or `Traced`. Where it does not, neither method
runs: no cascade is drawn and no Sun ray is traced, as today. Lamp shadows and ambient occlusion do not
depend on that switch.

With `Traced`, the scene's composed shader holds a `sun_visibility` that traces, and the light group holds the
scene structure. The operation's callers do not change. The traced operation:

- returns 1 with no ray when the surface faces away from the Sun, which is unlit anyway;
- starts the ray [as above](#where-a-ray-starts);
- traces one any-hit ray toward the Sun's centre, against instances whose mask casts shadows, with every
  triangle opaque, and returns 0 or 1.

<a id="a-rounded-meshs-own-facets"></a>

**A rounded mesh's own facets.** On a smooth-shaded coarse mesh, a fragment's shading normal can face the Sun
while its triangle's own plane faces away. Such a fragment is in **the terminator band**. Its ray starts into
the body, under the triangle it left, and with every triangle opaque the whole triangle is dark: a facet,
where the surface the normals stand for would be lit. Only the terminator band has this fault. A ray that
leaves the outer side of its triangle has a convex patch's other triangles falling away behind it, so whatever
it hits is a caster, and it is traced as above with no exception.

A ray in the terminator band is excused **one passage** through its own surface, and nothing more:

1. It starts on the inner side of its triangle, by the same lift. A short query across the lift finds the
   receiving instance and which way round its triangle is wound.
2. The nearest thing the ray meets must be the receiving instance, crossed from the inside: wound the
   opposite way to the receiving triangle as the ray meets it. It must lie within **the reach**, the
   distance at which the ray rises through the surface the vertex normals imply. The vertex stage
   interpolates tangent-plane data without the instance translation, which gives the fragment that surface's
   height over its triangle, the **gap**. The ray rises through the implied surface's tangent plane where it
   has climbed the gap and the lift along the shading normal, and that distance is the reach. Where the gap
   is not positive, on flat and concave geometry, there is no reach.
3. From that crossing it goes on as an ordinary ray. Any triangle of any instance, wound either way, blocks
   it.

If the nearest thing is the receiving instance crossed from the inside, but beyond the reach or where there
is no reach, the passage is **refused**: the body itself is in the way, and the fragment is dark, as a plain
ray has it. If the nearest thing is anything else, another instance or a face of the receiving instance met
from outside, it blocks the ray as a caster does. If the ray meets nothing, the fragment is lit.

So the exception removes the receiving surface's own far face, once, under the implied surface. It never
removes a second hit, a hit on another instance, or a hit in open air above the implied surface. A sheet
joined into a prop's mesh and the same sheet placed as a separate instance therefore shadow the terminator
band alike, wound either way, wherever the sheet stands beyond the prop's far face or above the implied
surface. [The terminator band's supported separation](#where-a-ray-starts) states the one case left.

How a fragment learns its triangle's plane is the builder's choice. Screen derivatives of the translation-free
position it already interpolates give it without the noise of world-position derivatives. A ray too close to
the plane to call is best treated as in the terminator band. On a rounded patch its passage leaves by the
next face inside the reach, and on flat geometry the light there is already at the shading's zero. A fragment
in the terminator band costs at most two queries beyond the ordinary ray, and no other fragment costs any.

Terrain is in the structure, so the ray answers at every distance and at low Sun. The traced method does not
use the cascades, the blocker search, `horizon_visibility` or `low_sun_terrain`. It replaces the map lookup and
nothing else: with the Sun's direct light out, a `Traced` frame equals the `Maps` frame, to the rounding a
test states for two compilations of the same arithmetic.

**What is still drawn.** The two cascade passes are drawn only while something reads them. Their only readers
today are surfaces, so a scene on `Traced` draws none. The top-down caster height map is still drawn, because
the air's shadow heights are reduced from it. When a volume reader of the cascades lands (`ATMOSPHERE`'s view
volume, `WATERS`' lent Sun visibility), the cascades are drawn for it under either method.

**A hard shadow has no partial Sun.** The centre ray gives a hard edge everywhere, and at sunset a receiver's
Sun goes out when the disc's centre meets the ridge. That is WP3's stated limit. WP4 removes it.

**Lamps.** A scene may ask for lamp shadows. Then, in `local_radiance`, each lamp in the fragment's cluster
list that is in range and that the surface faces traces one any-hit ray to the lamp's position, stopping short
of it. A lamp has no size in the engine, so its shadow is hard. The count of rays a fragment may trace for
lamps is a runtime quality value whose default is the cluster list's capacity; a lamp beyond it is unshadowed
at that fragment, and counted.

The lamp origin lift uses the transformed authored normal oriented toward the lamp. Mirrored transforms
reverse raster winding while leaving the transformed normal pointed out of the lit face.

A lamp's ray tests the cast bit of [the mask](#the-structure-wp1-wp2) and nothing else. It needs no Sun shadow,
no cascade and no shadow pipeline, so a room lit only by lamps, in a scene with Sun shadows off, is shadowed
by its walls.

**Shader text.** The composed shader includes the traced operations and `enable wgpu_ray_query` only when
the scene resolves to tracing. An admitted contribution that compiles the scene's shader takes the traced
operations from it, as it takes the cloud factor.

### Soft Sun shadows (WP4)

A scene on `Traced` may turn the penumbra on. A fragment's Sun visibility then has two sources.

**This frame's centre ray.** Every fragment that faces the Sun traces it inline, exactly as in WP3. It says
whether the Sun's centre is hidden now.

**The soft history.** An engine pass, the **traced visibility pass**, runs after the frame's opaque geometry:

1. **Trace.** For each pixel that holds opaque geometry it rebuilds the position from the frame's depth and
   view record, picks a point on the Sun's disc from a sequence that differs from pixel to pixel and frame to
   frame, and traces one any-hit ray toward it, started [as above](#where-a-ray-starts). The disc's radius is
   the directional source's own. The lift's normal, rebuilt from neighbouring depths, is the triangle's own,
   so the pass knows when a ray dips below its triangle's plane. It knows neither the shading normal nor
   the gap. [The pass's passage](#the-passage-in-the-pass) says what such a ray does.
2. **Over time.** It reprojects last frame's result through the frame's motion target, rejects history whose
   depth disagrees, keeps the first two moments, and clamps history to the range of this frame's rays in the
   texel's neighbourhood. Where those rays all agree, the range is one value, so the texel takes that value
   whatever its history held. The neighbourhood is wide, some hundreds of rays, as the denoiser's is, so that
   rays in a faint shadow do not agree by chance.
3. **Over space.** Edge-stopping passes, guided by depth and normal, whose width follows the variance. Where
   every sample agrees, as at a prop's base, nothing is blurred and the edge stays sharp.

It follows the FidelityFX Shadow Denoiser, whose source may be ported. The result is a history texture: the
soft visibility, the depth it belongs to, and whether the texel is **settled**. A texel is settled lit, or
settled dark, when no ray that disagrees has reached its value, through its history or through the filter:
the value is exactly 1 or 0. Any other texel holds a **soft edge**: a penumbra, as the filter spread it.
Settled is kept apart from the stored value, so that a faint shadow whose value rounds to 1 is never taken
for full Sun and blacked out by a blocked centre ray.

The soft visibility is two values, **passed** and **stopped**, which differ only where
[a ray dipped under its own triangle](#the-passage-in-the-pass). Steps 2 and 3 treat the two alike, each has
its own settled mark, and everything this design says of a soft value holds for each.

<a id="the-passage-in-the-pass"></a>

**The passage in the pass.** A disc ray on a pixel in [the terminator band](#a-rounded-meshs-own-facets) has
to pass its own body as the centre ray does. Otherwise every disc ray strikes the receiver, the texel settles
dark, and an outside caster's soft edge never forms there. But the pass has no gap, so it has no reach, and
without one it cannot tell the body's own far face from a part of the same mesh that stands metres away:
past an open rim, the back of a panel joined into an awning's mesh is the nearest thing a ray meets, and it
is the receiving instance crossed from the inside. So the pass does not decide. It records such a ray both
ways, and [the fragment](#what-a-fragment-uses), which has the reach, chooses.

- A ray that leaves the outer side of its triangle's plane is ordinary. Both values take its answer.
- A ray that dips below the plane starts on the inner side, and a short query across the lift finds the
  receiving instance and its winding. If it meets nothing, both values are lit. If the nearest thing is
  another instance, or the receiving instance met from outside, both are dark.
- If the nearest thing is the receiving instance crossed from the inside, the crossing is **a candidate**
  for the body's far face. The ray goes on from it as an ordinary ray. **Passed** takes what it then finds:
  the candidate is excused, as a far face is. **Stopped** is dark: the candidate blocks, as a joined sheet
  does.
- The pass cannot tell a terminator-band pixel from one on a body's dark side, where the candidate lies the
  body's whole width away and the Sun's light is zero whatever the ray finds. On a pixel whose triangle
  faces away, passed is the visibility of everything but the pixel's own surface, and stopped is the
  visibility of everything, the rest of its own mesh included.
- **The dive limit** bounds that work. It is a runtime value, an angle: a ray that dips further than it
  below its triangle's plane is not traced, and its pixel stores no history and takes no part in its
  neighbours' filtering. A terminator band reaches only as far below the plane as a shading normal leans
  from its triangle, so the builder sets the limit by eye on the valley's coarsest rounded body, and the
  report states it. Beyond it a terminator-band fragment shows the centre ray's hard edge.

A pixel inside the limit whose ray dips costs two queries beyond the ordinary ray. One beyond it costs none.
The second value costs no query: it is one more channel of the history, accumulated and filtered with the
first.

<a id="what-a-fragment-uses"></a>

**What a fragment uses.** It already has last frame's clip position for its motion vector. It reads the
history there and checks the stored depth against its own. Then:

1. With no matching history, it uses the centre ray. A pixel the pass left out has none.
2. It picks the soft value its own centre ray's passage bears out. Where that passage was **granted**, the
   body's far face lies inside the reach, the disc's rays crossed the same face, and the value is
   **passed**. Where the fragment is in the terminator band and no passage was granted, because it was
   refused or because the ray's first meeting was not a candidate at all, a candidate the disc's rays met
   is not this body's far face, and the value is **stopped**. Outside the terminator band the value is
   passed.
3. Where that value is settled and the centre ray contradicts it (settled lit and the ray is blocked, or
   settled dark and the ray is clear), it uses the centre ray. Occlusion changed here after the pass ran.
4. Otherwise it uses the value it picked.

So a refused passage no longer discards the history. Where the body itself is in the way, every disc ray
met a candidate, stopped is settled dark, and the fragment is dark as the centre ray has it. Where a joined
panel's edge crosses the disc, the rays that cleared the panel are lit in stopped, and the fragment shows
that share of the Sun: the same share a separate panel gives, on a closed body or at an open rim.

**The disc straddles the plane.** A fragment whose triangle faces the Sun's centre by less than the disc's
radius reads a history in which some rays dipped, so it needs a verdict too. With the penumbra on, such a
fragment is in the terminator band: its centre ray starts on the inner side and takes the passage. On a
rounded patch the passage is granted, as it is just below the plane. On flat geometry it is refused and
the fragment is dark, where the Sun's light on it was within half a percent of zero. On concave geometry it
is refused where the faces around the fragment already hide a Sun that low.

**What is left at an open rim.** One verdict stands for all of a fragment's disc rays. Where some of them
cross the body's last face and others pass under its rim, the fragment's value is right for one group and
not the other. The strip is as narrow as the disc is across the passage, under one percent of its length,
and lies beside the leak [the supported separation](#where-a-ray-starts) already states.

**How long old light may persist.** The centre ray says whether the Sun's centre is hidden. It cannot tell
the dark half of a soft edge from a shadow's body, or the light half from full Sun. So a fragment whose
history holds a soft edge shows that value, even where the edge has since moved away. Three cases follow,
and no other exists.

- **Where the history was settled, never.** A shadow's body arriving on lit ground, and Sun returning to
  ground that was in a body, are the centre ray's in the frame they are drawn. A moving caster's shadow stays
  under it with no frame of lag, as in WP3.
- **Where a soft edge has left, for one frame.** A texel that held a soft edge, and that the edge has left
  for the shadow's body or for full Sun, shows the old soft value in the first frame after. That frame's pass
  finds every ray in the texel's neighbourhood agreeing and writes the settled value, by the pass's step 2,
  so the next frame is right. The remnant is a band as wide as the old edge, lying where the edge was a frame
  before.
- **Inside the edge where it now lies, for the settle length.** That is every texel whose neighbourhood in
  the pass's step 2 still holds rays that disagree. The filter there remembers earlier frames. Its memory is
  a runtime value in frames, the **settle length**, which the builder sets by eye and the report states. No
  remnant of an earlier position shows once the settle length and one more frame have passed, and until then the soft
  edge is displaced by no more than the shadow moved in those frames.

**What that looks like.** A wide edge moves by a small part of its width in a frame, so its one-frame
remnant lies inside the edge it has now and reads as part of it. A narrow edge that moves further in a frame
than it is wide leaves a band as narrow as itself one frame's travel behind it, for one frame at each place:
the outline of where the shadow was. Shipped ray-traced shadows trail by their filter's whole memory, in the
body as well as the edge. Whether the band reads as a second edge is judged in WP4, on a caster fast and low
enough to make it, and [risk 3](#risks) names what would remove it.

A pixel with no history, such as ground a turn has just revealed, is hard for one frame.

The history is committed only with a submitted frame and reset on a camera cut, a change of render extent and
a device replacement, by `APPLE-PRESENTATION`'s history rules. Fragments drawn after opaque geometry (water)
have no history and trace inline.

### Ambient occlusion (WP5)

A scene may ask for ambient occlusion with a radius in metres. The traced visibility pass then also traces,
at half its extent, one ray for each pixel in a cosine-weighted direction about the surface normal, no longer
than the radius, against instances whose mask occludes. It accumulates and filters the result as it does the
Sun's, in a second channel, and shading reads it the same way, upsampled against depth. Where history is
absent the value is 1.

**This is the traced method of the scene's ambient occlusion, from `ATMOSPHERE` WP14.** The owner amended
[decision 34](../ARCHITECTURE.md#owner-decisions) on 27 November 2026: "Yes, add screen-space ambient
occlusion to the default path". The request, its resolution to one method, the fragment's read and the
application described here pass to `ATMOSPHERE`
([its design](../ATMOSPHERE/DESIGN.md#ambient-occlusion-on-the-default-path-wp14)), which adds a screen-space
method for every recipe. This module keeps the rays, the filter and the channel. A scene resolves to one
method: where it asked for traced and rays run, it traces as described here and records no screen-space
pass, so traced occlusion replaces the screen-space method and never multiplies it.

The value multiplies the ambient term only: the diffuse sky light, and the reflected sky light through
Lagarde's specular occlusion. It never touches the Sun or a lamp. It multiplies the material's own occlusion
texture and does not replace it.

**Latency is accepted here.** Occlusion has no ray traced in the frame it is drawn. What a receiver shows is
last frame's filtered value, and after an occluder arrives or leaves, on screen or off, it follows within its
own settle length and one more frame. Ambient occlusion is broad and faint, and shipped ray-traced occlusion
lags in the same way. A darkening still visible after that is a trail, and fails.

**Overcast valley.** With the visible cloud sky selected, the valley composes cooked geometry and
`thrum_clouds::Shade` in its `SceneContributions` list. RAY-TRACING WP5 adds candidate preparation to `Shade`
for both traced and Maps surfaces so the production list can admit traced occlusion. The cloud provider keeps
its existing lighting, graph passes and Sun-path product. It offers no ray instances. The inspection uses
[CLOUDS WP7's overcast preset](../CLOUDS/PLAN.md#wp7-the-sky-runs-from-clear-through-fair-and-broken-to-overcast-and-back-and-its-clouds-drift-form-grow-and-thin)
with the visible sky and its shade active. At the same cloud state, camera and time, traced occlusion darkens
the base of a prop while open ground stays unchanged. Verify Apple tile and resolved occlusion in both reports.

`Shade` prepares through [the WP3 boundary](#scene-preparation-and-maps-fallback-wp3). It keeps compatible
accepted cloud resources and temporal state. If the device generation changes, stage a full replacement. On
the same generation, read the compatible weather resources while staging the bindings and resources that the
candidate's Sun-path target and sky surface need. Describe the resulting graph for host validation. Keep any
one-time GPU initialization inside the host's preparation attempt, with writes confined to candidate-owned
resources and GPU errors resolved before installation. Only commit can replace the live GPU state or the
mailbox's prepared resource handles. The mailbox keeps its weather input and publishes submitted sky products
at the existing submission boundary. A dropped candidate leaves the accepted cloud state, history and
mailbox usable. The Maps retry follows the same rule with its own surface and the same accepted state.

<a id="selection-fallback-and-promotion"></a>

### Selection, fallback and the opt-in

- **Requests.** A scene's preparation takes: the Sun's shadow method, `Maps` or `Traced` with the penumbra on
  or off; lamp shadows on or off; ambient occlusion off or a radius; and the structure alone, for the traced
  view and for tests. With none given the scene is today's: `Maps`, and no ray effect. From `ATMOSPHERE` WP14
  the occlusion request is that module's and names a method: a radius alone is the screen-space method, and
  traced occlusion is a radius that prefers traced. They sit beside the
  scene's existing shadow option, and like the culling mode they are fixed when the scene is prepared.
- **Nothing is chosen for a scene.** There is no `Auto`. The registry holds no list of measured devices, and
  nothing reads an adapter's name or its core count to turn an effect on. A ray effect runs because a launch
  and a scene asked for it.
- **Where a ray request can be met.** Only on a device generation that enabled ray queries, and only on a
  recipe that lists ray effects. The Apple tile recipe lists them. The portable recipe does not, so a launch
  on it is the reference on any Mac, with or without the feature enabled.
- **The recipe.** A named recipe resolves as it does today: `THRUM_RECIPE=portable` is portable, and
  `apple-tile` is Apple tile or the refusal it is today. With no recipe named, a launch resolves to portable
  as today unless its host states a preference. Under
  [ruling 12](../ARCHITECTURE.md#seam-rulings-for-track-5) a host may prefer Apple tile: the registry admits
  it by the facts that admit a named one (Metal, unified memory, transient storage), and where they do not
  admit it the generation resolves to portable and the registry records why. A preference is never an error
  and never overrides a named recipe. `MESH-SHADING` WP3 and this module's WP6 both use it; whichever lands
  first builds it and the other uses it as found.
- **Resolution, with a plain fallback.** One function of the registry resolves the requests against the
  generation, before anything is allocated. A ray request that cannot be met resolves to what the portable
  path has: `Traced` to `Maps`, lamp shadows to off, traced ambient occlusion to off, and from `ATMOSPHERE`
  WP14 to the screen-space method at the same radius, and the structure to none. The scene
  prepares Maps directly on the resolved recipe. A request the registry admits also needs every contribution
  to support candidate preparation. If an admitted traced candidate fails while the device is usable,
  [prepare Maps once from the inputs to the same operation](#scene-preparation-and-maps-fallback-wp3).
- **Report.** For each request: what was asked, what it resolved to and, where the two differ, why. The
  reasons are that the device did not enable ray queries, that the recipe lists no ray effect, that a
  contribution lacks candidate preparation, or that preparation failed. The report also gives the hardware
  names the launch asked for, which were enabled and why, the resolved recipe, why a preferred recipe was
  skipped where applicable, the settle lengths and the dive limit. It appears with the culling mode so
  [a run can say what it ran](../ARCHITECTURE.md#interfaces-track-5).
- **Everything is prepared.** Pipelines for the resolved methods are created on a device generation before
  the scene is ready on it, and none is created while that generation draws. A replacement device creates its
  own before its first ready frame. The method stays fixed between accepted scene preparations.
- **Quality values.** Penumbra, lamp-ray count, ambient occlusion and its resolution, the two settle
  lengths and the dive limit are runtime values declared through the contribution seam's quality knobs, each
  with the history it invalidates, for the governor.
- **The opt-in.** A person turns the traced path on for a machine by how they launch: `ray-query` as a
  [preferred](#hardware-features-at-launch) name, the Apple tile recipe, and a scene that asks for `Traced`.
  The proving ground's valley has one launch selection, off by default, that does all three: it names
  `ray-query` as preferred, it states a preference for Apple tile where the device enabled ray queries, and
  it asks the scene for `Traced` with the penumbra. It asks for no lamp shadow and no traced ambient
  occlusion, which keep the selections WP3 and WP5 gave them. The valley's own request for screen-space
  occlusion, which every launch makes from `ATMOSPHERE` WP14, stands under the opt-in. The window and the capture host read it alike. What that
  one setting gives:

  | The launch, with the selection | Recipe | The Sun's shadow |
  |---|---|---|
  | No recipe named, on a Mac that traces rays and admits Apple tile | Apple tile | `Traced` with the penumbra |
  | `THRUM_RECIPE=portable` | Portable | `Maps`, reported with the recipe as the reason |
  | No recipe named, and Apple tile is not admitted | Portable, with why it was not taken | `Maps`, as above |
  | The GPU offers no ray queries, or the device request with them fails | As with no selection | `Maps`, reported with the device as the reason |

  In the last row the selection states no preference, so the launch is the ordinary one in everything but
  its report. `THRUM_RECIPE=apple-tile` stays the strict diagnostic it is: on a Mac that does not admit Apple
  tile it is refused with the selection as it is without it, and the selection neither causes that refusal
  nor hides it. With no selection nothing here is read and the launch is today's. The opt-in is recorded in
  two places: every run's report says what was asked, enabled and resolved, and this Mac's record says what
  the opt-in was found to do here.
- **This Mac's record (WP6).** It is written for a person deciding whether to opt a machine in, and nothing in
  the engine reads it. It holds the machine as the OS reports it (chip, GPU core count, memory, OS version)
  with the registry's digest, the commit, the launch line, the look, and the paired timing. The launch line
  is the selection and nothing else: it names no recipe. The look is two inspections of the final build, each
  under the settings its capture's report gives:
  - **The opt-in as launched.** Frames captured with the selection as the launch line gives it: traced Sun
    shadows with the penumbra, no lamp shadow, and ambient occlusion by the screen-space method, as the
    valley launches it. The Sun's cues are judged at contact,
    at rest and in motion, by the lists of WP3 and WP4 that apply. This is what a person who follows the launch
    line sees, and the only inspection that would show a fault of this configuration alone, such as a soft
    history that is bound only when traced ambient occlusion is on. At the night stop it shows lamps without
    shadows, and the record says so.
  - **Every effect together.** Lamp shadows and traced ambient occlusion added through their own selections,
    judged by the lists of WP3 to WP5. The report names traced as the occlusion method, and the frame
    records no screen-space occlusion pass. The report and the pass names are the proof that one method
    replaced the other. The two looks are not compared for brightness: rays see an occluder outside the frame
    that the screen-space gather misses, and lamp shadows are on in one launch and off in the other, so a
    contact may rightly be much darker here. No sentence of the record gives this result as the opt-in's.
- **The paired timing is a tripwire.** On the final build with capture off, at 1280×720 and at 1920×1080, the
  valley's route runs from two launches. One is the opt-in launch as the record's launch line gives it, with
  no recipe named: `Traced` with the penumbra, the default quality values, lamp shadows off and ambient
  occlusion as the valley launches it, by the screen-space method. The other is the same launch without the selection and with Apple tile named, which draws
  `Maps`. So both run on the Apple tile recipe and differ in the Sun's method and in nothing else that draws:
  the same recipe, cooked draw path, reconstruction, display range, extent, route and quality values. Each
  run keeps its report of the resolved recipe, Sun shadow method and cooked draw path, which shows it. At
  each extent they run interleaved, twice each, which is eight runs. The record keeps
  [the two figures](#cost-account) of every frame, each run's medians, each pass by name and the machine's
  load. It states the difference between the methods beside the spread between a method's own two runs, and
  where the spread is the larger it says that these runs could not tell the two methods apart at that
  extent. Rows for `Traced` with hard edges, for lamp shadows, for traced ambient occlusion in place of the
  screen-space method, and for every effect together sit beside the pair. It is taken while other builders work, as every timing of this module is. A traced frame about 25% above the map frame in
  either figure, or a visible hitch, is investigated and the record says what was found. No figure fails an
  item, and none turns anything on.

  The setting also moves a launch with no recipe named from portable to Apple tile. The pair holds the
  recipe fixed because that isolates the method: against a portable launch it would also measure that
  recipe's own attachments, reconstruction and display range. The record says so beside the pair.
- **What is not claimed.** No default, and no performance qualification under C12. That asks for a
  comparison with the portable recipe, an image difference inside a declared tolerance and a sustained run,
  and it belongs with a decision to make a ray effect a default. The maturity ledger records the traced path
  as run on this Mac, not performance-qualified and not a default.
- **Why WP6 waits for the exit.** WP6 waits for `SHOWCASE` WP8 so that its inspection and its record describe
  the valley as the V1 exit leaves it. Nothing here changes the window's default, before the exit or after
  it, and the verifier's runs name no hardware feature, so no registered exact baseline moves.

### Check that it can work

- **The API path.** [The probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md) built, rebuilt and
  traced through the locked wgpu on this Mac, from both stages this design traces from.
- **Poses.** Every instance transform the vertex stage uses is on the CPU when the frame is recorded: the
  instance record's two matrices and the frame's alpha. The scene structure is built from the same values.
- **The ray's origin.** [The second probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/OFFSET.md) placed
  receivers as the raster path does and found no self-hit at 2 float steps or more, at any distance to 20 km
  and any elevation from 0.5°. At 8 steps a panel 2 cm off a wall still blocks at 20 km. At 256 steps a panel
  10 cm off a wall at 10 km does not.
- **The one passage.** [The third probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/PATCH.md)
  is a CPU calculation over smooth-shaded one-metre bodies: prisms of twelve and forty-eight sides and a
  sphere of 80 triangles, 9,500 terminator-band points in all. A plain ray lights 5% of the terminator band on
  the coarse prism and 13% on the sphere; the passage lights all of it. The body's own far face never lies
  beyond 0.49 of the reach, so the reach needs no margin. A sheet across the ray shadows the terminator band
  in every one of six cases on each body: joined into the body's mesh facing the ray, joined facing away, or
  separate, each beyond the reach and inside it past the far face. With a straight edge 5 m toward the Sun
  across the disc's centre, the pass's rule blocks half the disc rays, as the edge alone does, where plain
  rays block 93% to 97% on the two coarse bodies.
- **The two soft values.** [The fourth probe](../../../.orchestrate/evidence/RAY-TRACING/planning-probe/SOFT.md)
  is the design review's own case: the twelve-sided prism, whole and cut to an open rim at the receiving
  panel's edge, with a panel 5 m down the light, separate and joined into the mesh with its back to the ray.
  With the panel's edge 1 mm to either side of the disc's centre, a single value with no reach blocks none of
  1,024 disc rays at the open rim under the joined panel. The fragment's pick blocks 540 and 480 in all four
  arrangements. As the Sun crosses the receiving panel's plane with the joined panel over half the disc,
  the pick holds 513 of 1,024 from half a degree above the plane to four below, where it would thin to 281
  just above the plane without the straddle rule. On the whole prism with no panel, 2,323 fragments from
  0.27° above the plane to 5° below all had their passage granted, and no disc ray was blocked.
- **Penumbra range.** The Sun's disc is about 0.53° across, so a penumbra is about 0.0093 of the distance to
  the caster: 2 cm at 2 m, which is under a pixel beyond a few metres and stays sharp; 0.9 m under a balloon
  100 m up; 28 m from a ridge 3 km off. A wide penumbra changes slowly across the screen, which is the case
  the variance-guided filter is made for, and a setting Sun is the same estimate with part of the disc hidden.
- **A moving shadow.** A balloon 100 m up moving at 10 m/s shifts its shadow 8 cm in a frame at 120 Hz,
  against a penumbra 0.9 m wide: the body is traced this frame, the edge is a tenth of its width behind, and
  the edge it left a frame ago lies inside the edge it has now. A figure walking at 1.5 m/s shifts its shadow
  1.3 cm against an edge about that wide at its head's shadow: the old edge lies against the new one. A prop
  sliding along the ground at 30 m/s shifts 25 cm against a penumbra of a few centimetres: nearly every texel
  is settled, so the shadow is the centre ray's and stays at the prop's base, and the edge it left shows as a
  band a few centimetres wide, 25 cm behind, for one frame.
- **The retry at launch.** `request_device` returns an error and does not abort, and the Mac window already
  takes a fresh adapter for every device it makes, so a second request after a failed one is a call it makes
  today.
- **The timed-pass span.** `timing.rs` reads every timed pass's begin and end counters from one batch, the
  surface consumer's among them, so the span is the last end less the first begin, with no new query. A
  structure build is not a pass with timestamps of its own. It is inside the span when a timed pass that
  reads no structure is encoded before it, and WP1's record of the build's GPU time has to arrange that
  anyway. A span that held the queued transfers too would need the command buffer's own GPU start time,
  which wgpu does not give. For a tripwire the pass interval is enough when it is named as one.
- **The recipe.** `capabilities/recipe.rs` resolves the recipe from the adapter's backend, unified memory and
  transient storage and one optional name, and returns portable when none is given. A preference is one
  more optional input, read only when no name is given and admitted by the facts a named Apple tile needs.
  With no preference the function returns what it returns today, so no existing launch moves. The registry
  is made after the device, so the host knows whether ray queries were enabled before it states one.
- **Scale.** The representative scene is 10,000 entities with 1,000 to 2,000 movers. A rebuild is linear in
  instances, and the structure holds every instance whether or not it is on screen. WP1 records it at 10,000
  and at the field's 100,513, and claims nothing at the larger count.
- **What an edit costs.** A mesh structure cannot be built in parts, so an edit to a mesh costs its whole
  build in one frame. WP2 records that time against triangle count. `CONTENT-LOOP`'s promise of no late frame
  on an edit stands for the portable method; on a scene with a structure it is the recorded build.

### Scene preparation and Maps fallback (WP3)

The first submission after a successful fallback has the bytes, content and streaming progress of the
ordinary Maps submission for the same operation. Compare the same device generation and recipe, visual
revision, frame inputs, and starting raster content and upload readiness. Match the first submissions by
order. On the same device, a receiver that Maps could already draw stays drawable with the same resident
buffers and without uploading them again, even when the traced candidate needs different buffers. A new
attachment or a replacement device starts with the residency and upload budget of the ordinary Maps path
for that operation. Compare both from the start of streaming. A fallback neither warms up the scene nor
speeds up its uploads to make the comparison pass.

**Contribution admission.** `SceneContribution` keeps `passes`, `rebuild` and `encode_upload` as its required
hooks. A new object-safe hook exposes candidate preparation and defaults to unsupported. An
opted-in member prepares from a shared borrow of its accepted state and returns a candidate with the
resources and declarations it will install. Its infallible commit takes that candidate and installs it on
the member. WP3 opts the cooked contribution in for both traced and Maps candidates. WP5 opts the
[valley's cloud provider](#ambient-occlusion-wp5) in through the same hook. A `SceneContributions` list admits
candidate preparation only if every leaf opts in, including leaves in nested lists. The empty list is eligible.
Admission reads support before any traced preparation or call to `rebuild`. Detached visual and device
preparation carry that decision with their host identity. Check that identity before publication so a change
to the list cannot reuse an older admission decision.

If the device and recipe admit rays but any leaf lacks candidate preparation, the game gets the ordinary
Maps path for the whole list. Preserve the requests in the report, resolve the Sun to `Maps` and the other
ray effects to off, and report that a contribution lacks support for candidate preparation. This is an
admission result distinct from `PreparationFailed`. Ordinary Maps callers, including this direct Maps
path, keep the existing `rebuild` and `rebuild_for_attachment` hooks, their list order and their error and
recovery behaviour. A member implementing only the required hooks can still draw with Maps. The traced
preparation boundary and its retry use candidate preparation for every member.

**Host operations.** The frame host owns one prepare-and-commit boundary for any operation that attempts
traced 3D preparation and for its Maps retry. This includes `FrameHost::new` with an empty list, 3D
attachment, same-device visual preparation and publication, device replacement, and extent changes that
rebuild a traced scene or member. Initial construction includes the frame slots and optional presenter
binding in the attempt. For coupled `resize_scene_surface` and `resize_scene_surface_if`, stage any traced
rebuild for resized secondary views alongside the new slots, presenter and extent. Keep the prior grid and
contribution list live until the whole candidate passes, including the platform epoch check. A failed or
stale traced resize leaves that grid and list usable. Resizes that only refit the presenter or use the existing
extent preparation against accepted pipelines keep their target-only path.

The host uses the candidate declarations to validate the combined graph, secondary views, frame targets
and presenter bindings. Finish all fallible work before the first install, including structure and pipeline
creation and resolving GPU error scopes for validation, internal errors and out-of-memory errors on the
usable device. Commit installs the scene and full ordered contribution list together on the render owner
at the existing safe boundary.

Both attempts start from the same accepted state and use the same publication inputs. Until commit, the
live scene, list routing, raster residency and bindings, loader and upload progress, material selection,
product leases, and mesh-row publication state remain with the current owner. Only commit can replace or
invalidate them for the transition. Members can borrow compatible live resources for reading or stage
replacements. Candidate writes, lease offers and row switches stay private until the appropriate
publication boundary. A candidate for ray demand can stage incompatible draw buffers while the resident
Maps set stays live. An abandoned candidate releases its owned resources under C06. A successful Maps
commit may retire ray-only resources under the same lifetime rules and keeps normal cooked publication
semantics under C05. Each submitted frame still commits uploads and history at its existing submission
boundary.

Failure at any stage aborts the whole candidate. The host drops it and starts one Maps attempt from the
same inputs. A successful retry publishes the requested operation and reports
`PreparationFailed` for the ray requests it replaced. If the retry fails too, the host reports the error
without publishing a partial scene or detaching a member. The ordinary recovery policy still applies to
operations that resolve directly to Maps. On a lost device the accepted CPU inputs remain available for
recovery under C06.

## The system as it runs

1. **Launch.** The launch's list names `ray-query`, required or preferred. The device enables it and the
   registry records it. A preferred name that cannot be met is left out, and the registry records why. The
   recipe resolves from its name, or from the host's preference where none was named, and the registry
   records which and why.
2. **Preparation.** The scene resolves its requests before trying a candidate. With ray demand, the arena and
   cooked meshes use the structure-build usage and build their mesh structures, the shader is composed with the
   traced operations, and pipelines are created through the device's creation authority. The host commits the
   complete candidate under [the preparation boundary](#scene-preparation-and-maps-fallback-wp3).
3. **Each frame.** Uploads are reconciled. The structure pass sets one instance for each active entity and
   each placement a contribution offers, and rebuilds the scene structure. The caster height map is drawn;
   cascades only if something reads them. The forward pass shades: each Sun-facing fragment traces the centre
   ray, and with the penumbra on uses the soft history's passed or stopped value where the rules allow. After opaque geometry, the
   traced visibility pass traces, filters and writes the history the next frame reads.
4. **Submit.** Only a submitted frame commits the history.
5. **Loss or replacement.** The new generation rebuilds every structure, creates its pipelines and resets the
   history before its first frame.

## What this module provides

Defined here, listed in [the module list](../ARCHITECTURE.md#interfaces-track-5):

- **Hardware features at launch**, with `MESH-SHADING`.
- **Ray instances**: how a contribution puts its geometry in the scene structure.
- **Traced visibility**: the Sun shadow method and the lamp-shadow and ambient-occlusion requests of a scene,
  and what they resolve to.

## Shared files and exact baselines

This module builds inside state other modules own, under
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5). Its own files are a `raytrace`
module in `thrum-render` and its tests. With no request and no selection every byte on main is unchanged,
the recipe a launch resolves to included, and each work package
proves it through [the exact baselines](../ARCHITECTURE.md#exact-baselines). No baseline is added for a traced
frame: it is judged by inspection.

## Cost account

Recorded, never a gate (brief decision 3). The tripwires are the brief's: a visible hitch, a comparable figure
about 25% worse than its last record, growth that is not linear in what it should follow.

**A frame's cost** is two figures, in every record of this module from WP3 on:

- **Timed-pass span:** from the begin timestamp of the first timed pass the frame encodes to the end
  timestamp of the pass that consumes the frame's output. The time between those passes lies inside it, and
  so does the structure build, which a frame with ray demand encodes after its first timed pass. It is a
  pass interval and not the whole frame's GPU time. wgpu runs the transfers a frame queues with
  `Queue::write_buffer` before the frame's first pass, so they lie outside it: the view and instance uploads
  under both methods, the cascades' view records under `Maps`, and whatever the traced path queues for its
  own passes. Those differ between the methods, so the span can miss a GPU cost one of them pays. The CPU
  side of the same writes is inside frame CPU time. It is the span
  [ruling 7](../ARCHITECTURE.md#seam-rulings-for-track-5) adds to the frame's timing record, which WP3 adds
  if `MESH-SHADING` WP2 has not, and uses as found if it has.
- **Frame CPU time:** the frame sample's upload and encode intervals together, which run from the start of
  the frame's uploads to the return of its submission.

A sum of pass times is less than either. The runtime's scene sum leaves out the surface pass and the time
between passes, and tracing moves work into exactly those places: it adds a build before shading and removes
two cascade passes. Pass times by name explain a difference between two rows and are never the difference.

Every record that carries the span calls it the timed-pass span and says which queued transfers lie outside
it for each method, and no sentence drawn from it speaks of the whole frame's GPU time.

- **Structure memory.** One mesh structure for each mesh, not each instance, plus an instance record for each
  thing in the scene. WP1 and WP2 record resident bytes.
- **Preparation overlap.** A transition retains the live resources and one candidate. Reuse compatible
  raster buffers by reference and allocate incompatible storage only for the candidate that needs it.
  Check contribution support once per preparation. The boundary adds no work to the frame loop.
- **Each frame, CPU.** One instance write for each entity and placement in the scene.
- **Each frame, GPU.** The scene structure's rebuild, which WP1 records for a still scene, for the
  representative movers and with every instance moving. Hard Sun: one ray for each shaded opaque pixel that
  faces the Sun, and in a rounded mesh's terminator band the receiver query and the ray that goes on from the
  far face, two more queries. Soft Sun: the same, with the terminator band's two queries also paid by a
  fragment whose plane the disc straddles; and in the visibility pass one ray for each opaque pixel,
  two more queries where that ray dips below its triangle inside the dive limit, and none at all beyond the
  limit. The history holds two soft values and their moments where one would do without the terminator
  band, and the filter's passes carry both. WP4's record states the limit, the share of pixels on each side
  of it, and the history's bytes. Lamps: one ray for each
  in-range lamp a pixel faces. Ambient occlusion: one ray for each four pixels. The filter's passes.
- **Saved on `Traced`.** The two cascade passes and the blocker search, while no volume reads the cascades.

If the soft path's inline ray shows as a tripwire, the named candidate is to trace it only against instances
that moved, through a third mask bit.

## Risks

1. **wgpu's ray tracing is experimental.** A defect a builder cannot work around inside Thrum ends the
   work package `BLOCKED` with the smallest program that shows it. No pinned copy of wgpu exists before
   `APPLE-PRESENTATION` WP10, and nothing is posted upstream without the owner.
2. **The rebuild grows with the scene, not the view.** Recorded from WP1. The candidate is a second structure
   for still geometry.
3. **A soft edge trails a mover.** Inside the edge the settle length bounds it, and the edge a shadow has
   left shows for one frame. WP4 inspects both on named frames, with a caster fast and low enough that its
   shadow moves further in a frame than its edge is wide. If the edge it left reads as a second edge, the
   named candidate is a few more inline rays toward the disc's rim on a fragment whose history holds a soft
   edge: where they all agree with the centre ray, the fragment has left the edge and uses the centre ray.
   The builder may add them in WP4, as a quality value, and says so in the evidence. If the edge still cannot
   pass, WP4's builder ends `BLOCKED` with the frames.
4. **The terminator band is lit by a rule, not by the triangles.** [The passage](#a-rounded-meshs-own-facets)
   lights a terminator band the mesh's own triangles shadow, and its stated limits are a joined sheet
   inside the gap at an open rim, [the strip beside it](#what-a-fragment-uses) under soft shadows, and a
   hard edge on a terminator band beyond the dive limit. WP3 inspects the
   terminator band on the valley's rock and balloons, and WP4 under another body's soft shadow and on an
   open shell under a panel of its own mesh. If a facet
   survives on an irregular mesh because its far face lies beyond the reach, the builder may widen the reach
   by eye and says by how much, because the terminator band's supported separation grows with it.
5. **Normals rebuilt from depth are wrong at silhouettes.** The filter's edge stops bound it; WP5 inspects it.
   A terminator-band pixel the pass misreads there settles dark, and its fragment shows the centre ray's hard
   edge.
6. **Other builders are in the same files.** `SCENE-VIEWS`, `ATMOSPHERE`, `APPLE-PRESENTATION`, `HOST`,
   `REVIEW-RENDER` and `MESH-SHADING` all work in `thrum-render`, `thrum-app` or `thrum-asset-gpu`. This
   module's changes add to those files; a builder integrates on what has landed and rewrites nothing another
   work package holds.

## Proof and builder handoff

- GPU tests ask `thrum-test-support`'s gate for a device with `ray-query` required and the Apple tile recipe,
  and are skipped with a reason where the adapter offers no ray queries. On this Mac a skip fails. A capture
  for an inspected item requires the name too, so its frames cannot be the fallback's. WP6's capture of the
  opt-in is the one exception: it is launched as the record's launch line gives it, because that launch is
  what it judges. Its report is kept beside its frames, and frames whose report does not read Apple tile and
  `Traced` with the penumbra are the fallback's and are not judged.
- A case this Mac cannot show by itself is forced: an adapter without ray queries, and one whose facts do
  not admit Apple tile, through the registry's functions, and a failed device request, structure or pipeline
  through a fault the test injects.
- Tests that need only entities live in `thrum-render`'s tests. Proofs that need cooked meshes or the valley
  run in the proving ground through a script, except WP3's merge-tier guards. WP3.14 checks admission and commit
  order on the CPU through the production boundary. The reviewer traces each caller into that boundary.
  WP3.15 uses the existing `thrum-asset-gpu` integration test binary on Metal and a receiver just large enough
  to need a second upload. It checks the first submitted bytes and upload progress. Both guards run without
  opt-in Cargo features. WP3.16 requires the existing main-branch `long-tests` result covering the WP3 commit,
  including the feature-gated selection and contribution tests.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, in the
  proving ground's valley with each effect and the traced view behind a selection that is off by default.
- An item that judges motion names its frames: the frame occlusion changed, and each frame after it up to the
  settle length and one more. The reviewer is given those frames at native size.
- Every timing of this module is a tripwire taken under load, WP6's pair included: it records the load and
  fails only on a missing measurement. No sitting with other builders paused is asked for.
- Scratch goes in `tmp/ray-tracing-<work-package>/`, selected evidence in `.orchestrate/evidence/RAY-TRACING/`.

## Open questions

None that blocks a work package. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
