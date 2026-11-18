# Ray tracing: traced shadows and ambient occlusion on the Mac

18 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac) · [the brief](../BRIEF.md) ·
planning probes: [the API](../../evidence/RAY-TRACING/planning-probe/README.md) and
[the ray's origin](../../evidence/RAY-TRACING/planning-probe/OFFSET.md). C04, C06, C07, C08 and C12 in
[CONTRACTS](../../design/CONTRACTS.md),
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) and
[the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) govern.

## The request

The owner, 17 November 2026:

> Just do ray tracing and mesh shaders on mac, let's leave PC for now.

The lead's task for this plan, in part: "Plan hardware ray tracing (Metal ray queries / intersection) and mesh
shaders (Metal object and mesh stages) on the Apple tile recipe, verified on this M3 Max, under the
hardware-native decision: registry-selected accelerated paths, the portable path stays as fallback and
reference, and each path is promoted only by a measured win. Decide from first principles and current state of
the art which rendering problems in Thrum they should serve ... It must not delay V1 (SHOWCASE M8) ... Desktop
high / PC stays out of scope; keep designs reusable for a later Vulkan/D3D12 recipe where that costs nothing."

The owner, the same day: "Ok let's lower the bar." Visual work passes on a shipped-game look and on speed. A
physical or `f64` reference is a diagnostic and never a pass mark. Proof is in proportion to risk.

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
| A game or demo on the Mac | Turn each effect on for a scene, see what it resolved to, and pay nothing when it is off | Scene-level requests, a resolved report, no resource without a request |
| The showcase valley's Apple row, after the exit | The path that earned it, selected without being asked, and a launch that never fails for it | A promotion rule, the record it reads, and a request the window makes only where it will be met |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | No shader text, pipeline, resource or byte changes there, whatever the device offers |
| A later Vulkan or D3D12 recipe | The same paths | Everything goes through wgpu's public API and WGSL; nothing names Metal |

**Commissioned**, on `mac-metal`:

1. A ray-tracing structure of every opaque thing in a scene, at the pose each frame draws it.
2. The Sun's shadow on surfaces by ray, as a method of the scene's one `sun_visibility` operation.
3. Shadows from lamps, as an effect a scene asks for.
4. Soft Sun shadows: the Sun's disc sampled by rays and filtered over time and space.
5. Ambient occlusion by ray.
6. A paired record against the portable recipe, and promotion by the rule below.

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
- **Ray effects on the portable recipe.** The portable recipe is the reference, on this Mac too, so it has
  none.
- **Chrome and the iPhone simulator.** Neither offers ray queries. A physical iPhone with a ray-tracing GPU
  would resolve the same paths from the same registry facts; none is available, so nothing is claimed.

## Cues

| Cue | Where it must read | Range | A plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **R1 A shadow is where the geometry puts it** | Every opaque surface the Sun or a lamp lights | Contact to the far ridge; Sun from overhead to the horizon; movers; mirrored and scaled instances; an edited mesh; a caster the camera cannot see | A shadow that is right in a still and a frame late in motion, cast by last frame's mesh, or lost when its caster leaves the view. Rejected by consecutive frames around movers, by the traced view, and by rays with plain answers |
| **R2 No acne, no gap, no leak** | Receivers at grazing light, thin casters, where a prop meets the ground | Sun elevation 1° to 90°; receivers 0.5 m to 20 km from the camera; coordinates to 20 km from the scene's origin | A bias tuned on one view, or one so large that it steps over a nearby caster. Rejected by inspection across the range and by a panel close to a wall, far from the origin |
| **R3 A shadow softens with distance from its caster** | A balloon's shadow on the valley floor, a ridge's shadow at low Sun, a prop's own base | Caster 0.1 m to 3 km from the receiver; the Sun's disc cut by a ridge at sunset | A blur of constant width, or grain in place of softness. Rejected by frames inspected at rest and in motion |
| **R4 Things are grounded** | Under and between props, in creases of terrain, in shade and under an overcast sky | Occluders from contact to the scene's declared radius | Darkening that follows the screen and not the scene: halos at silhouettes, occlusion that vanishes when the occluder leaves the view. Rejected by turning the camera so the occluder is off screen |
| **R5 Nothing else moves** | The frame with no request; Chrome; the simulator; the portable recipe | Every scene on main | A request that defaults on. Rejected by the exact baselines and by counting created resources |

The bar is ray-traced shadows and occlusion as shipped high-end games show them, such as Cyberpunk 2077's
ray-traced Sun and local shadows and Metro Exodus Enhanced Edition's grounded ambient light, smooth at the
display's full refresh.

## Research and selection

Read for this design, 17 and 18 November 2026:

- **The locked wgpu reaches both features on Metal.** `wgpu-hal` 30.0.1's Metal adapter sets
  `EXPERIMENTAL_RAY_QUERY` from the device's ray-tracing support and builds acceleration structures
  (`metal/adapter.rs`, `metal/command.rs`); naga writes ray queries to MSL. wgpu's own `ray_shadows` example
  traces from a fragment stage. [The planning probe](../../evidence/RAY-TRACING/planning-probe/README.md) ran
  on this Mac: the adapter offers ray queries; a 10,000-instance structure rebuilt in about 2 ms of wall time;
  two million any-hit rays from a compute stage and from a fragment stage gave the same answers, and the
  right ones. So no raw Metal is needed, and the
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
  real-time denoised ambient occlusion sample (MIT) does. Both run in a deferred renderer, after depth and
  before lighting. This renderer is forward, which is what
  [the soft shadow's design](#soft-sun-shadows-m4) has to answer.
- **Apple's hidden surface removal.** A tile GPU shades each opaque pixel once whatever the draw order, so a
  ray traced inside the forward pass's fragment stage costs one ray a pixel with no depth prepass. That is why
  the hard shadow is traced inline, and why this design adds no prepass.
- **Self-intersection.** Wächter and Binder, "A Fast and Robust Method for Avoiding Self-Intersection" (Ray
  Tracing Gems, chapter 6), lift a ray's origin along the normal by an amount that scales with the point's
  coordinates, in place of a tuned bias. Their constant, 256 float steps, allows for a hit point computed at
  the end of a long ray. It is 0.25 m at 10 km from the origin, which steps over a nearby caster. A raster
  fragment's position carries far less error. [The second probe](../../evidence/RAY-TRACING/planning-probe/OFFSET.md)
  measured it on this Mac: two float steps cleared every self-hit from 64 m to 20 km and down to half a degree
  of grazing light.

**What was selected, and what else was weighed.**

| Decision | Selected | Weighed and not selected |
|---|---|---|
| Where the hard Sun shadow is traced | Inline, in the fragment that is being shaded, this frame | A screen-space mask pass before shading: it needs a depth prepass, a second geometry pass on a tile GPU |
| How a soft shadow is made | One ray a pixel toward a point on the disc, in a pass after opaque geometry, filtered over time and space and read by the next frame's shading, beside a centre ray traced inline this frame that overrides the history where occlusion has changed | The history alone: a moving caster's whole shadow is then a frame late, and detaches from a fast mover. The same rays integrated by TAAU alone: at mid-penumbra one ray is right half the time, and a history of some twenty frames leaves about a tenth of the Sun's light as grain. A fourth forward target for demodulated Sun light, which would make the soft value current: it changes every forward pipeline in the engine. MetalFX's denoising upscaler: it needs albedo, normal and roughness targets and belongs to `APPLE-PRESENTATION`; it is what would change this selection |
| Where ambient occlusion is traced | In the same pass as the soft shadow, at half the extent | Inline in shading: one ray a pixel is grain over every shaded surface |
| How many structures | One for the scene, rebuilt each frame | One for still geometry and one for movers: this adapter has room, but every ray would walk both. It is the named candidate if the rebuild shows as a tripwire, and M1's record of a still scene beside a moving one says what it could save |
| What the structure holds | Every opaque thing in the scene, on screen or off | What the frame's passes draw: a lamp's ray, a Sun ray at low Sun and an occlusion ray all reach geometry that no camera or cascade volume holds |

## Proof

Each cue is judged by eye, in the running proving ground, against its milestone's inspection list and the bar
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
  request that cannot be met, and a scene that asks for nothing.

No item holds a ray, a shader's visibility or an occlusion value to a CPU reference. A builder who needs one to
find a fault may write it, and says so in the evidence.

## Mechanisms

### Hardware features at launch

The registry already records the features a device generation enabled. This module adds the request.

- A launch names the hardware features it wants: `ray-query` here, `mesh-stages` for `MESH-SHADING`. The
  diagnostic route is the environment variable `THRUM_HARDWARE`, a comma-separated list, read as `THRUM_RECIPE`
  is. A host may pass the same list in code.
- One function of the registry turns the list, the launch's recipe request and the adapter into the features,
  limits and wgpu experimental-features token for the device request, before the device is made, as
  `requested_timing_features` does for timing. A name the launch gave that the engine does not know, or that
  the adapter does not offer, is refused by name. Nothing is dropped silently.
- With no list the device is the one made today: no experimental feature, the same limits. So every exact
  baseline and the V1 exit's three rows are untouched.
- **A host adds a name of its own only where it will be met.** After [promotion](#selection-fallback-and-promotion)
  the Mac window asks the same function, before it makes the device, whether the adapter and the launch's
  recipe match a qualified entry and the adapter offers ray queries. Only then does it add `ray-query`.
  Otherwise it makes the device it makes today. So a name the host added is never refused, and a Mac that was
  not measured, or one on the portable recipe, launches and draws as it does now.
- A capture and a test ask only for what they name. Chrome has no request route and the iPhone app does not
  ask.

`MESH-SHADING` adds its feature to the same function and the same list. Whichever module's first milestone
lands second extends what the first built.

### The structure (M1, M2)

Two kinds of structure, in wgpu's terms a `Blas` and a `Tlas`:

- A **mesh structure** holds one mesh's triangles in the mesh's own space. It is part of that mesh's
  residency: built when the mesh's vertices and indices are on the GPU, replaced when they are, and retired
  with them. It reads the buffers the mesh is drawn from, so no geometry is copied.
- The **scene structure** holds one instance for each opaque thing in the scene: a mesh structure, a
  transform, an identity and a mask. It is rebuilt every frame.

**Demand comes from the scene, before anything is allocated.** A scene's preparation
[resolves its requests](#selection-fallback-and-promotion) first. If any resolved effect needs rays, the scene
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

**Entities (M1).** With ray demand, each row of the mesh arena gets a mesh structure, built from the arena's
vertex and index buffers at the row's range. The scene structure takes one instance for each active slot, with
the model matrix the vertex stage uses: `current`, with its translation mixed from `previous` by the frame's
presentation alpha. An inactive slot has no instance. So a ray meets a mover where the frame draws it.

**Cooked placements (M2).** With ray demand, each resident cooked mesh gets a mesh structure, built in the
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
Bit 0 is set for what the scene declares a shadow caster before any culling: every active entity of a scene
whose shadows are on, and every placement of a cooked mesh that the contribution's shadow pass may draw.
Bit 1 is set for every instance: all opaque geometry occludes.

**Edits (M2).** [A content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) needs no new
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
- **The normal is not taken from derivatives of world position.** Close to the camera and far from the origin
  a pixel's footprint is smaller than a float step, and that derivative is noise. A fragment uses its
  interpolated normal, turned to the side it faces. The visibility pass uses neighbouring depths in view
  space, where nearby positions keep their precision.
- **The supported separation.** A caster always blocks when it stands more than twice the lift's ceiling from
  its receiver: 32 float steps. That is 2 mm within 1 km of the origin, 3.2 cm at 10 km and 6.3 cm at 20 km.
  The factor of two leaves room for the caster's and the receiver's own rounding. Nearer than that, a caster
  thinner than the lift may be stepped over.

### Traced shadows (M3)

**The Sun.** A scene's Sun shadow has a method: `Maps`, the cascades and height map of today, or `Traced`.
With `Traced`, the scene's composed shader holds a `sun_visibility` that traces, and the light group holds the
scene structure. The operation's callers do not change. The traced operation:

- returns 1 with no ray when the surface faces away from the Sun, which is unlit anyway;
- starts the ray [as above](#where-a-ray-starts);
- traces one any-hit ray toward the Sun's centre, against instances whose mask casts shadows, with every
  triangle opaque, and returns 0 or 1.

Terrain is in the structure, so one ray answers at every distance and at low Sun. The traced method does not
use the cascades, the blocker search, `horizon_visibility` or `low_sun_terrain`. It replaces the map lookup and
nothing else: with the Sun's direct light out, a `Traced` frame equals the `Maps` frame, to the rounding a
test states for two compilations of the same arithmetic.

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

A scene on `Traced` may turn the penumbra on. A fragment's Sun visibility then has two sources.

**This frame's centre ray.** Every fragment that faces the Sun traces it inline, exactly as in M3. It says
whether the Sun's centre is hidden now.

**The soft history.** An engine pass, the **traced visibility pass**, runs after the frame's opaque geometry:

1. **Trace.** For each pixel that holds opaque geometry it rebuilds the position from the frame's depth and
   view record, picks a point on the Sun's disc from a sequence that differs from pixel to pixel and frame to
   frame, and traces one any-hit ray toward it, started [as above](#where-a-ray-starts). The disc's radius is
   the directional source's own.
2. **Over time.** It reprojects last frame's result through the frame's motion target, rejects history whose
   depth disagrees, keeps the first two moments, and clamps history to the range of the current neighbourhood.
3. **Over space.** Edge-stopping passes, guided by depth and normal, whose width follows the variance. Where
   every sample agrees, as at a prop's base, nothing is blurred and the edge stays sharp.

It follows the FidelityFX Shadow Denoiser, whose source may be ported. The result is a history texture: the
soft visibility, the depth it belongs to, and whether the texel is **settled**. A texel is settled lit, or
settled dark, when every ray in its filter footprint this frame and its own history agree. Any other texel is
in a penumbra.

**What a fragment uses.** It already has last frame's clip position for its motion vector. It reads the
history there and checks the stored depth against its own. Then:

1. With no matching history, it uses the centre ray.
2. With a settled history that the centre ray contradicts (settled lit and the ray is blocked, or settled
   dark and the ray is clear), it uses the centre ray. Occlusion changed here after the pass ran.
3. Otherwise it uses the soft value.

**How long old light may persist.**

- **Outside a penumbra, never.** A shadow's body, and lit ground, are traced in the frame they are drawn. A
  moving caster's shadow arrives, leaves and stays under it with no frame of lag, as in M3.
- **Inside a penumbra, for the settle length.** The soft value a fragment reads was made a frame ago, and the
  filter there remembers earlier frames. The filter's memory is a runtime value in frames, the **settle
  length**, which the builder sets by eye and the report states. When occlusion changes at a point, no
  remnant of the old value shows there once the settle length and one more frame have passed.
- **How large the error can be.** The soft edge is displaced by no more than the shadow moved in those
  frames, and only within the penumbra's own width. A shadow that moves far in a frame against a narrow
  penumbra is nearly all settled texels, which rule 2 corrects at once. A wide penumbra moves by a small part
  of its width in a frame.

A pixel with no history, such as ground a turn has just revealed, is hard for one frame.

The history is committed only with a submitted frame and reset on a camera cut, a change of render extent and
a device replacement, by `APPLE-PRESENTATION`'s history rules. Fragments drawn after opaque geometry (water)
have no history and trace inline.

### Ambient occlusion (M5)

A scene may ask for ambient occlusion with a radius in metres. The traced visibility pass then also traces,
at half its extent, one ray for each pixel in a cosine-weighted direction about the surface normal, no longer
than the radius, against instances whose mask occludes. It accumulates and filters the result as it does the
Sun's, in a second channel, and shading reads it the same way, upsampled against depth. Where history is
absent the value is 1.

The value multiplies the ambient term only: the diffuse sky light, and the reflected sky light through
Lagarde's specular occlusion. It never touches the Sun or a lamp. It multiplies the material's own occlusion
texture and does not replace it.

**Latency is accepted here.** Occlusion has no ray traced in the frame it is drawn. What a receiver shows is
last frame's filtered value, and after an occluder arrives or leaves, on screen or off, it follows within its
own settle length and one more frame. Ambient occlusion is broad and faint, and shipped ray-traced occlusion
lags in the same way. A darkening still visible after that is a trail, and fails.

### Selection, fallback and promotion

- **Requests.** A scene's preparation takes: the Sun's shadow as `Auto`, `Maps`, or `Traced` with the
  penumbra on or off; lamp shadows on or off; ambient occlusion off or a radius; and the structure alone,
  for the traced view and for tests. They sit beside the scene's existing shadow option, and like the culling
  mode they are fixed when the scene is prepared.
- **Where a ray effect can resolve.** Only on a device generation that enabled ray queries, and only on a
  recipe that lists ray effects. The Apple tile recipe lists them. The portable recipe does not, so a launch
  on it is the reference on any Mac, with or without the feature enabled.
- **Resolution.** One function of the registry resolves the requests against the generation, before anything
  is allocated. `Auto` resolves to a qualified entry's configuration when the generation's adapter and recipe
  match the entry and the generation enabled ray queries, and otherwise to `Maps`. An explicit `Traced`, lamp
  shadows or ambient occlusion where a ray effect cannot resolve is refused by name at preparation, and the
  refusal says whether the feature or the recipe is missing. A request is never silently downgraded.
- **A promoted path that fails to prepare.** If a scene's `Auto` resolved to the traced configuration and a
  structure or a traced pipeline does not build, the scene is prepared as `Maps` before it is ready, what the
  failed attempt made is released, and the report says `Maps` and why. An explicit request that fails to
  prepare is a preparation error by name.
- **Report.** What each request resolved to, and the settle lengths, are reported where the culling mode is,
  so [a run can say what it ran](../ARCHITECTURE.md#interfaces-track-5).
- **Everything is prepared.** Pipelines for the resolved methods are created on a device generation before
  the scene is ready on it, and none is created while that generation draws. A replacement device creates its
  own before its first ready frame. No method is switched while a scene runs.
- **Quality values.** Penumbra, lamp-ray count, ambient occlusion and its resolution, and the two settle
  lengths are runtime values declared through the contribution seam's quality knobs, each with the history it
  invalidates, for the governor.
- **The qualified list.** The registry holds a list of qualified entries. An entry names what was measured:
  the adapter, by the name the registry records for it; the Apple tile recipe; and the Sun's configuration,
  which is `Traced` with the penumbra setting the sitting ran. The list is empty until M6.
- **Promotion (M6).** One configuration is the candidate: `Traced` with the penumbra on, at the default
  quality values, with lamp shadows and ambient occlusion off. M6's sitting runs the valley's route on this
  Mac, with capture off, at 1280×720 and at 1920×1080. At each extent it takes six runs, interleaved: the
  portable recipe from an ordinary launch, the Apple tile recipe with `Maps`, the candidate, then all three
  again. Two figures are read from every frame of a run:
  - **Frame GPU time:** the sum of the GPU intervals of every scene pass in the frame's timing record.
  - **Frame CPU time:** the frame sample's upload and encode intervals together.

  A run's value for a figure is the median over the route's frames. Against Apple tile `Maps`, each figure has
  two values for each path at one extent. Its spread is the larger of the two paths' differences between their
  own two runs. The candidate is *no higher* in a figure when its larger value is at most the map path's
  smaller value plus the spread, and *lower* when its larger value is below the map path's smaller value by
  more than the spread.

  **Cost.** At both extents, both figures are no higher.

  **Gain.** Either a figure is lower at an extent, or the candidate looks better: on stills paired at the
  same stops and times, the candidate passes every entry of the list below and the portable recipe's frame,
  which is the map frame, fails at least one, shown in paired crops.
  1. *Contact.* No gap and no acne where a prop meets the ground.
  2. *Distance.* Beyond the cascades' reach, props and ridges cast shadows with their own shape.
  3. *Low Sun.* Terrain shadows at a low Sun fall where the terrain puts them.
  4. *Softness.* An edge is sharp at contact and widens with the caster's distance.

  The configuration is qualified when it meets cost and gain. The record then says "promoted" and the list
  gains this Mac's entry. Otherwise it says "costs more" or "no gain" and the list stays empty. The record
  keeps both figures for every frame of all twelve runs, so a reader can recompute the conclusion. Lamp
  shadows and ambient occlusion add what the map frame does not have, so they are never promoted: a scene asks
  for them.
- **What C12 asks beside that.** The portable runs are the same-device reference for the record's times. The
  candidate's image is compared with the portable recipe's at the same stops, and the declared tolerance has
  two parts. With the Sun's direct light out, the traced and the map frame are equal, as M3 tests. With it
  on, the two may differ only where the Sun's shadow differs: at shadow edges, at contact, and where maps gave
  no shadow or a coarse one. Sky, water, cloud, ground in full Sun and ground in full shade are the same,
  judged on a difference image. The sitting ends with the candidate's route looped for ten minutes, and the
  record gives the thermal state the OS reports before and after.
- **What promotion changes.** An `Auto` scene resolves to the entry's configuration, penumbra included, on
  the measured adapter and the Apple tile recipe, and [the Mac window asks for `ray-query` itself](#hardware-features-at-launch)
  there. Nowhere else does anything change. The verifier's runs name no hardware feature, so they resolve to
  `Maps` and no registered exact baseline moves at promotion. M6 waits for `SHOWCASE` M8, so the window's
  default does not change under the V1 exit either.

### Check that it can work

- **The API path.** [The probe](../../evidence/RAY-TRACING/planning-probe/README.md) built, rebuilt and
  traced through the locked wgpu on this Mac, from both stages this design traces from.
- **Poses.** Every instance transform the vertex stage uses is on the CPU when the frame is recorded: the
  instance record's two matrices and the frame's alpha. The scene structure is built from the same values.
- **The ray's origin.** [The second probe](../../evidence/RAY-TRACING/planning-probe/OFFSET.md) placed
  receivers as the raster path does and found no self-hit at 2 float steps or more, at any distance to 20 km
  and any elevation from 0.5°. At 8 steps a panel 2 cm off a wall still blocks at 20 km. At 256 steps a panel
  10 cm off a wall at 10 km does not.
- **Penumbra range.** The Sun's disc is about 0.53° across, so a penumbra is about 0.0093 of the distance to
  the caster: 2 cm at 2 m, which is under a pixel beyond a few metres and stays sharp; 0.9 m under a balloon
  100 m up; 28 m from a ridge 3 km off. A wide penumbra changes slowly across the screen, which is the case
  the variance-guided filter is made for, and a setting Sun is the same estimate with part of the disc hidden.
- **A moving shadow.** A balloon 100 m up moving at 10 m/s shifts its shadow 8 cm in a frame at 120 Hz,
  against a penumbra 0.9 m wide: the body is traced this frame and the edge is a tenth of its width behind.
  A prop sliding along the ground at 30 m/s shifts 25 cm against a penumbra of a few centimetres: nearly
  every texel is settled, so the shadow is the centre ray's and stays at the prop's base.
- **Scale.** The representative scene is 10,000 entities with 1,000 to 2,000 movers. A rebuild is linear in
  instances, and the structure holds every instance whether or not it is on screen. M1 records it at 10,000
  and at the field's 100,513, and claims nothing at the larger count.
- **What an edit costs.** A mesh structure cannot be built in parts, so an edit to a mesh costs its whole
  build in one frame. M2 records that time against triangle count. `CONTENT-LOOP`'s promise of no late frame
  on an edit stands for the portable method; on a scene with a structure it is the recorded build.

## The system as it runs

1. **Launch.** The host's list, or the window's own request inside a qualified configuration, asks for
   `ray-query`. The device enables it and the registry records it.
2. **Preparation.** The scene resolves its requests. With ray demand, the arena and each resident cooked
   mesh are created with the build usage and build their mesh structures, the shader is composed with the
   traced operations, and pipelines are created through the device's creation authority. Without it the scene
   is prepared as it is today.
3. **Each frame.** Uploads are reconciled. The structure pass sets one instance for each active entity and
   each placement a contribution offers, and rebuilds the scene structure. The caster height map is drawn;
   cascades only if something reads them. The forward pass shades: each Sun-facing fragment traces the centre
   ray, and with the penumbra on uses the soft history where the rules allow. After opaque geometry, the
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
module in `thrum-render` and its tests. With no request every byte on main is unchanged, and each milestone
proves it through [the exact baselines](../ARCHITECTURE.md#exact-baselines). No baseline is added for a traced
frame: it is judged by inspection.

## Cost account

Recorded, never a gate (brief decision 3). The tripwires are the brief's: a visible hitch, a comparable figure
about 25% worse than its last record, growth that is not linear in what it should follow.

- **Structure memory.** One mesh structure for each mesh, not each instance, plus an instance record for each
  thing in the scene. M1 and M2 record resident bytes.
- **Each frame, CPU.** One instance write for each entity and placement in the scene.
- **Each frame, GPU.** The scene structure's rebuild, which M1 records for a still scene, for the
  representative movers and with every instance moving. Hard Sun: at most one ray for each shaded opaque
  pixel that faces the Sun. Soft Sun: that ray, and one more for each opaque pixel in the visibility pass.
  Lamps: one ray for each in-range lamp a pixel faces. Ambient occlusion: one ray for each four pixels. The
  filter's passes.
- **Saved on `Traced`.** The two cascade passes and the blocker search, while no volume reads the cascades.

If the soft path's inline ray shows as a tripwire, the named candidate is to trace it only against instances
that moved, through a third mask bit.

## Risks

1. **wgpu's ray tracing is experimental.** A defect a builder cannot work around inside Thrum ends the
   milestone `BLOCKED` with the smallest program that shows it. No pinned copy of wgpu exists before
   `APPLE-PRESENTATION` M10, and nothing is posted upstream without the owner.
2. **The rebuild grows with the scene, not the view.** Recorded from M1. The candidate is a second structure
   for still geometry.
3. **A soft edge trails a mover.** The settle length bounds it; M4 inspects it on named frames. If it cannot
   pass, the penumbra stays off by default and the traced method is not promoted.
4. **A rounded mesh shows its facets where it turns from the Sun.** A ray sees the triangles, not the
   smoothed normal, so a coarse balloon can show dark facets at its terminator. M3 inspects it. The builder may
   soften the terminator by eye and says how.
5. **Normals rebuilt from depth are wrong at silhouettes.** The filter's edge stops bound it; M5 inspects it.
6. **Other builders are in the same files.** `SCENE-VIEWS`, `ATMOSPHERE`, `APPLE-PRESENTATION`, `HOST`,
   `REVIEW-RENDER` and `MESH-SHADING` all work in `thrum-render`, `thrum-app` or `thrum-asset-gpu`. This
   module's changes add to those files; a builder integrates on what has landed and rewrites nothing another
   milestone holds.

## Proof and builder handoff

- GPU tests ask `thrum-test-support`'s gate for a device with the hardware request and the Apple tile recipe,
  and are skipped with a reason where the adapter offers no ray queries. On this Mac the feature is required,
  so a skip fails.
- Tests that need only entities live in `thrum-render`'s tests. Proofs that need cooked meshes or the valley
  run in the proving ground through a script, so a check names no game package.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, in the
  proving ground's valley with each effect and the traced view behind a selection that is off by default.
- An item that judges motion names its frames: the frame occlusion changed, and each frame after it up to the
  settle length and one more. The reviewer is given those frames at native size.
- Timing inside M1 to M5 is a tripwire taken under load. M6 takes the paired numbers in one sitting with
  other builders paused; its builder asks the lead for it.
- Scratch goes in `tmp/ray-tracing-<milestone>/`, selected evidence in `docs/evidence/RAY-TRACING/`.

## Open questions

None that blocks a milestone. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
