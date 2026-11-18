# Mesh shading: meshlets culled and drawn by the object and mesh stages on the Mac

17 November 2026, revised 18 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac) · [the brief](../BRIEF.md) ·
[planning probe](../../evidence/RAY-TRACING/planning-probe/README.md). C04, C05, C06, C07 and C12 in
[CONTRACTS](../../design/CONTRACTS.md), [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md),
[the cooked draw design](../COOKED-DRAW/DESIGN.md) and
[the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) govern.

## The request

The owner, 17 November 2026:

> Just do ray tracing and mesh shaders on mac, let's leave PC for now.

The lead's task is quoted in [RAY-TRACING's design](../RAY-TRACING/DESIGN.md#the-request), which is the other
half of it. It asked that each path be "promoted only by a measured win".

The lead, 18 November 2026, after two design reviews found faults in a rule that made the stages a default by
itself: "prefer the simplest promotion rule that holds: an explicit, recorded per-device opt-in with a
measured paired timing as a tripwire is acceptable; don't build general registry keying or exhaustive operand
capture this module doesn't need. Off the V1 path." So this module makes nothing a default. A person turns the
path on for a machine, and the measured win the task asks of a default is owed by whoever later makes one.

This design is mesh shaders: Metal's object and mesh stages, which wgpu calls the task and mesh stages.

## Purpose and scope

A cooked mesh is already cut into meshlets of at most 64 vertices and 124 triangles, each with a bounding
sphere and a normal cone. On a Mac whose GPU has object and mesh stages, the GPU itself decides which meshlets
to draw and draws them, inside one draw call for each placement. The player sees the same picture. What
changes is the work.

**What the renderer does today, and where it falls short.** `thrum-asset-gpu` culls a placement's meshlets in
a compute pass (`cull.wgsl`), which writes one indexed indirect draw for each meshlet with an instance count
of 1 or 0. The forward pass then calls `multi_draw_indexed_indirect` for all of them.

| Today | Shortfall |
|---|---|
| wgpu 30 has no multi-draw with a GPU-written count on Metal or WebGPU, and its Metal backend turns the call into a loop of one Metal draw for each meshlet (`wgpu-hal` 30.0.1, `metal/command.rs`) | The CPU records as many draws as the mesh has meshlets, every frame, and the GPU takes each one, including those the cull rejected |
| The Sun's shadow passes draw a cooked mesh whole (`draw_shadow`) | A cascade that sees a corner of the terrain rasterises all of it |
| Culling is a pass before the draw, with its own buffers for each view | Every further view of the scene needs its own set |

Object and mesh stages remove all three: the object stage tests meshlets and launches a mesh workgroup only
for those that pass, in whichever pass is drawing.

| User | Needs | What that requires |
|---|---|---|
| A game or demo on the Mac | The same scene for less CPU and GPU work, and to see which path drew it | A path a launch and a scene ask for, resolved from the registry and reported, with the path it replaces prepared as the fallback, for the scene or for one mesh |
| A person deciding whether to turn it on for a machine | How it looks and what it costs on a Mac that ran it | One launch selection, and this Mac's record |
| A creator editing a cooked material or mesh while the scene runs | The edit, on whichever path draws the mesh | One accepted material and one publication for both paths |
| Every launch and scene that did not ask, and a Mac whose GPU lacks the stages | The launch and the frame it has | Nothing requested or selected unasked, and a request that cannot be met draws the existing path and says why |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | The indirect and direct paths untouched |
| A later Vulkan or D3D12 recipe | The same path | wgpu's public API and WGSL only |

**Commissioned**, on `mac-metal`: cooked placements drawn through the object and mesh stages in the main
view's forward pass and in the Sun's shadow passes, resolved from the registry when a launch and a scene ask,
with the existing path as fallback and reference; one launch selection that turns the path on; and this Mac's
record of how it looks and what it costs.

**Not built**, with the reason:

- **Entities.** The 10,000 entities of the exit scene draw whole mesh rows through `CULLING`'s bucket route,
  culled a whole instance at a time. Drawing them a meshlet at a time is where the larger saving may lie, since
  `SHOWCASE` sizes prop detail to fit the frame. It needs meshlet tables in the mesh arena and a change to the
  route whose every mode is held byte-equal to an oracle. The next plan decides it from this module's record
  ([open for the human](PLAN.md#open-for-the-human)).
- **A default.** Nothing here turns the stages on unasked, on this Mac or any other. A default needs C12's
  performance qualification, taken on a quiet machine, and a rule for which machines it covers. Both belong
  with the decision to make one.
- **A check of the stages with `RAY-TRACING`'s traced Sun.** A scene may ask for both, and
  [the stages draw whichever passes it records](#selection-fallback-and-the-opt-in). This module runs the
  path with the Sun's shadows as maps and with them off. Neither module waits for the other, so the two
  together are not inspected or timed here ([open for the human](PLAN.md#open-for-the-human)).
- **Occlusion culling of meshlets.** `CULLING`'s depth pyramid holds entities only, and a cooked placement is
  never an occluder in it, so there is nothing to test terrain meshlets against.
- **Levels of detail and cluster hierarchies.** The cooker makes one level.
- **Secondary views.** A view of `SCENE-VIEWS` draws cooked meshes by the path that module resolves.
- **Compressed vertex streams decoded in the mesh stage.** The cooked format is unchanged.
- **Chrome and the iPhone simulator.** Neither offers the stages.

## Cues

| Cue | Where it must read | Range | A plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **G1 The same picture** | The valley's terrain and cooked meshes, in the main view and in their shadows | Every cooked mesh in the valley; 1 to the valley's largest meshlet count; camera inside, beside and far from a mesh; noon and low Sun | A path that drops a meshlet nobody is looking at, or a shadow caster that drops the faces turned from the Sun. Rejected by byte equality between the path with its cull on and with it off, and by the path with its cull off against the path it replaces |
| **G2 Nothing pops** | The camera in motion and across a cut | Snap turns, a mesh crossing the near plane, an edit, a device replacement | A cull tested at rest. Rejected by the same equality at every frame of a scripted path |
| **G3 The work is known** | The recorded frame | The valley's route; a mesh wholly in view, partly in view and behind the camera | Counters that fall while time does not, or a sum of pass times that falls while the frame takes longer. Rejected by a paired record of the frame's whole GPU span and its CPU time, which says so when the pair shows no difference |
| **G4 The path is the one reported** | Mac on both recipes, with and without the launch request | Adapters with and without the stages; a name required and preferred; a scene with the Sun's shadows off; a mesh the device's limits cannot bind or dispatch | The indirect path passing as the mesh stages, a fallback nobody is told of, or a mesh silently left out. Rejected by asserting the resolved path and its reason for the scene and for each mesh, and by a count of mesh-stage draws that is not zero |
| **G5 An edit arrives** | A cooked material or mesh edited while the scene runs | Accepted and refused edits, a frame that is discarded, a device replacement, a mesh that fell back | A reload reported as installed while the stage pipeline still draws the old fragment. Rejected by the edit read from pixels a stage pipeline drew, in the frame the reload reports |

The bar is a shipped GPU-driven renderer's meshlet path, such as the mesh-shader paths of Alan Wake 2 and of
Unreal's Nanite fallback raster: the player never sees it work.

## What is proved, and how hard

Under [the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) of 17 November 2026 the look is
judged by inspection, speed is a tripwire record, and exact or numerical proof stays where the bar keeps it.
This module has four proofs.

- **The cull is safe.** The path with its cull turned off draws every meshlet. The cull only removes meshlets
  that lie outside the view or face away, so with it on, the submitted bytes are the same. That equality is
  exact, because both arms run the same shader. It is the bar's "culled image against the unculled one", and
  it is held in the forward pass and in each of the Sun's maps.
- **The path draws what the path it replaces draws.** The cull's equality cannot see a fault both of its arms
  share, such as a wrong placement transform, a wrong motion delta or a one-sided shadow caster. So with its
  cull off the path is compared with the existing one in what other modules read: the frame contract's outputs
  (colour, depth, motion, reactive coverage) in the forward pass, and the depth of the Sun's maps. The two
  paths submit the same triangles and compute each vertex by one function text, so they differ at most by
  float rounding between two compilations. The test states its tolerance for each output with the reason,
  shows where the pixels it does not judge lie (a triangle's edge, a depth tie) and counts them. No bound is
  measured first and asserted later, and the difference images are kept as the diagnostic.
- **The look.** One inspection in the valley on the mesh-stage path, against the list in
  [M2's done-when item](DONE.json), with the existing path's stops beside it.
- **The speed.** Tripwire records under load, in M2 and in M3's pair, which pass on their check. Nothing is
  performance-qualified here, so no sitting with other builders paused is asked for.

## Research and selection

Read for this design, 17 November 2026:

- **The locked wgpu reaches the stages on Metal.** `wgpu-hal` 30.0.1's Metal adapter offers
  `EXPERIMENTAL_MESH_SHADER` on Apple GPUs, creates `MTLMeshRenderPipelineDescriptor` pipelines and records
  `drawMeshThreadgroups`; naga parses `enable wgpu_mesh_shader` WGSL and writes it to MSL. On this Mac
  [the planning probe](../../evidence/RAY-TRACING/planning-probe/README.md) drew 4,096 meshlets through 64
  object workgroups, rejected half in the object stage from a storage table, read vertices from a storage
  buffer in the mesh stage, and every meshlet was right. The adapter allows 256 vertices and 256 primitives
  for each mesh workgroup, which holds the cooker's 64 and 124, and 1,048,576 mesh workgroups for each object
  workgroup.
- **What the stages are for.** Apple's WWDC 2022 session on mesh shaders names GPU-driven meshlet culling as
  the primary use: the object stage decides how many mesh workgroups to launch and hands them a payload. Haar
  and Aaltonen (SIGGRAPH 2015) and Karis (Nanite, 2021) cull instances, then clusters; the stages are how
  current engines draw the survivors on hardware that has them.
- **A reason to doubt.** A 2023 Apple developer forum report found meshlet culling in the mesh pipeline on an
  M1 no faster with a depth test than without, and suspected the driver reserves output for the worst case. No
  primary source settles how the stages compare with indirect draws on a tile GPU. So this module measures
  before it widens: the cooked placements are the smallest complete use, and entities wait for the record.
- **Limits that bind a mesh, not a meshlet.** The stages read a mesh's streams as storage, where the existing
  path binds positions as a vertex buffer. wgpu's default limits allow a 256 MiB buffer and 128 MiB in one
  storage binding (`wgpu-types` 30.0.1, `limits.rs`), and they cap the object workgroups of one draw in each
  dimension and in total. So a mesh the existing path draws can be one the stages cannot bind or dispatch,
  whatever the size of its meshlets. Eligibility is therefore [decided for each mesh](#residency).
- **Why not Metal's indirect command buffers.** They are Apple's own answer to GPU-written draw lists. wgpu
  does not expose them, and the [boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) keeps public
  wgpu for all it can express.

**Selected:** cull in the object stage with the rule the compute pass uses, draw in the mesh stage from the
cooked meshlet tables, and keep the existing path prepared as fallback and reference.

## Mechanisms

### Hardware features at launch

As [RAY-TRACING defines it](../RAY-TRACING/DESIGN.md#hardware-features-at-launch): a launch names
`mesh-stages` in `THRUM_HARDWARE`, or in code, as required or preferred, and one function of the registry
makes the device for every host. A required name is enabled or the launch is refused by name. A preferred
name never fails a launch: where the adapter does not offer it, or the device request with it fails, the
launch gets the device it gets today and the registry records why the name is not enabled. With no request
the device is the one made today. Whichever module's first milestone lands second extends what the first
built.

The request for `mesh-stages` also raises any ordinary limit the stage pipelines need, such as storage buffers
in one stage, and only for a launch that asks. An adapter that cannot grant them does not offer `mesh-stages`:
a required name is refused, and a preferred one is left out.

**The launch decides the device and nothing else.** It is made before any scene exists, from the launch's
list and the adapter. No host adds `mesh-stages` unasked, on any adapter, and the registry holds no list of
machines. What a scene draws is decided when the scene is prepared, from the scene's own request
([selection](#selection-fallback-and-the-opt-in)). A device that enabled the stages and a scene that does not
draw through them hold [the bytes they hold on main](#residency).

Pipelines for the stages are created through the device's creation authority, which gains a third entry
beside `render` and `compute`, so the creation ledger counts them.

### Residency

A cooked contribution that resolved to the mesh stages keeps, for each eligible mesh, what the stages read.
The mesh's position and attribute buffers gain storage usage beside the vertex usage they have, and its
meshlet table, meshlet-vertex list and packed meshlet triangles are uploaded. The cooked product already
carries all of them (`thrum-asset`'s `Meshlet`, and the meshlet-vertex and meshlet-triangle sections), so the
cooker and the product format do not change. They are part of the mesh's residency: copied under the same
upload budget, shown and retired with the mesh, and switched by
[a content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) like the rest of it.

A contribution that resolved to any other path uploads none of this and changes no usage, on any device,
including one that enabled the stages. So the paired record's existing-path arm, and an ordinary scene on a
device that has the stages, hold the bytes they hold on main.

**Eligibility is decided for each mesh when it is staged**, before a buffer is made for it, and again for the
replacement mesh of a publication. Against the limits of the device generation, a mesh is eligible when:

- each stream the stages bind as storage (positions, attributes, meshlet bounds, meshlet table,
  meshlet-vertex list, meshlet triangles) fits one buffer and one storage binding;
- the object workgroups its meshlet count needs fit one draw, in each dimension and in total;
- an object workgroup's run of meshlets fits the mesh workgroups it may launch and the payload;
- its largest meshlet fits a mesh workgroup's output vertices and primitives.

An ineligible mesh is staged exactly as the fallback path stages it, and its placements draw through that
path. Nothing of it is bound to a stage pipeline, so a pipeline that compiled is never handed a buffer it
cannot bind. The report counts the mesh with the limit that excluded it. Its eligible neighbours draw through
the stages in the same pass. A publication that carries a mesh across the line, either way, moves that mesh
alone to the other path in the publication's frame.

### The draw (M1)

One draw for each placement of an eligible mesh: `draw_mesh_tasks` with enough object workgroups to cover the
mesh's meshlets.

- **Object stage.** Each workgroup owns a fixed run of meshlets. It tests each against the view with
  `cull.wgsl`'s rule, the same function text compiled into both paths: the five frustum planes against the
  bounding sphere, then the normal cone against the camera. It writes the survivors' indices to the payload
  and launches one mesh workgroup for each.
- **Mesh stage.** A workgroup reads its meshlet from the payload, fetches the meshlet's vertices through the
  meshlet-vertex list, and writes for each vertex exactly what `cooked_vertex` writes: clip position,
  unjittered clip position, world position, normal, texture coordinate, the history flag and the clip delta
  for motion. One function text computes those outputs in both paths, and the position keeps the scene's
  `@invariant` mark, so both paths are asked for the same clip position. It writes the meshlet's triangles
  from the packed list.
- **Fragment stage.** The scene's own fragment entry, unchanged. So a mesh-stage fragment is lit, shadowed and
  writes motion and reactive coverage as the existing path's does.
- **The placement's record.** The model matrix and history flag reach the stages from the placement's uniform
  record, where the vertex path takes them as instance attributes.

The stages' module is a second compilation of the scene's composed shader with the stage entries and the
`enable wgpu_mesh_shader` directive added. The scene's own modules are not touched. On a generation that
enabled the stages, the scene's view and light group layouts are visible to the object and mesh stages as they
are to the vertex stage.

A pass draws a mixed scene in one go, each placement by its mesh's path. The cull pass runs only for meshes
on the indirect path.

### One material for both paths (M1)

A creator changes a cooked material through `MaterialReload`. Today a request compiles the hooked scene source
into a replacement for the indexed forward pipeline off the frame, the next frame boundary draws it, and the
submission that carries it installs it. A mesh-stage contribution has a second forward pipeline, and both are
compilations of the scene's source with the one accepted hook. They change together.

- **One candidate.** A request prepares one candidate off the frame, as today: the hooked source compiled into
  the indexed pipeline and, on a mesh-stage contribution, into the stage pipeline. The candidate is eligible
  only when every pipeline in it has validated. A hook that fails in either is refused whole with its
  diagnostics, and the accepted pipelines keep drawing on both paths.
- **Drawn and installed whole.** The frame that draws a candidate draws stage-drawn meshes with its stage
  pipeline and fallen-back meshes with its indexed pipeline. The submission that carries a resident mesh
  drawn with it, on either path, installs both pipelines and promotes the hook. No submitted frame draws one
  path with one material and the other path with another.
- **A discarded frame.** A frame recorded with the candidate and never submitted installs nothing. The
  candidate stays held and the next frame draws it again, as today.
- **A rebuilt device or surface.** Both paths draw the built-in material until the accepted hook has validated
  for every pipeline again, then both draw the hook.
- **When pipelines are made.** At three times only: when a scene is prepared, when a device generation or
  surface is rebuilt, and off the frame for a reload that was asked for. Recording a frame creates none.

The shadow pipelines have no fragment stage, and a reload does not touch them.

### The shadow passes (M2)

The Sun's maps are two cascades and a top-down height layer (`synthetic/shadow.rs`), and the cooked
contribution draws its casters into each. On the mesh-stage path each of the three passes draws an eligible
mesh through the stages with the pass's own view and projection.

- **Object stage.** It tests each meshlet's bounding sphere against the planes that pass clips by, with no
  cone test. A meshlet is rejected only when every triangle in it would be clipped away.
- **Mesh stage.** It writes position only, by the expression the existing caster's vertex stage uses.
- **Raster state.** The existing caster's: both faces drawn, and the same depth format, compare and bias.
  Cooked terrain is an open heightfield. At a low Sun its Sun-facing faces are grazing slivers that rasterise
  with holes, and the faces that carry the shadow are turned from the Sun (`draw.rs`, `shadow_pipeline`). A
  one-sided caster would pass the cull's equality in both arms and still leak light.
- **The pass's view.** Each map's pass binds its own matrix at group 0, through the caster view layout that
  `synthetic/shadow.rs` makes and lends to a contribution as `SceneShadowSurface::view_layout`. Today that
  binding is visible to the vertex stage alone, so neither new stage could read it. Under
  [ruling 8](../ARCHITECTURE.md#seam-rulings-for-track-5), on a generation that enabled the stages the
  binding is also visible to the object and mesh stages. Nothing else about the layout, the uniform or the
  pass changes, and on any other generation the layout is today's. The stage shadow pipeline's layout is that
  group and a group of the contribution's own for the placement's record, so the one bind the pass makes
  serves the existing casters and the stages alike, in all three maps.
- **Which passes.** The stage shadow pipelines are made when the scene has Sun maps, as the existing caster's
  is. A scene with the Sun's shadows off makes none and draws its forward pass through the stages. The
  contribution draws into each map pass the renderer records and reads nothing about the Sun's method.

So a pass rasterises only the meshlets that reach it, and what it stores is what the existing caster stores.

<a id="selection-fallback-and-promotion"></a>
### Selection, fallback and the opt-in

- **Request.** `CookedDrawPath` gains `MeshStages` beside `Direct` and `Indirect`. A scene's preparation asks
  for `Auto` or names a path, and the path is fixed when the scene is prepared.
- **Nothing is chosen for a scene.** `Auto` is `COOKED-DRAW`'s selection between the direct and indirect
  paths, on every device, with the stages enabled or not. It never resolves to the stages. The registry holds
  no list of measured machines, and nothing reads an adapter's name or its core count. The stages draw because
  a launch and a scene asked for them.
- **Resolution for the scene.** `MeshStages` is met on a device generation that enabled the stages. On one
  that did not, it resolves to what `Auto` resolves to there, and the scene is prepared as if it had asked for
  that, so it submits the bytes of the same scene with no request. The request is not refused, and it fails
  no scene. A test that must draw through the stages requires `mesh-stages` at launch, so it cannot run on a
  device without them.
- **Resolution for each mesh.** Within a mesh-stage scene, [an ineligible mesh](#residency) draws through the
  fallback.
- **Fallback.** The fallback is the path `COOKED-DRAW` selects for the device, which on the Mac is the
  indirect path. Its pipelines are prepared with the stages', so a stage pipeline that fails to build resolves
  the whole scene to the fallback before the scene is ready.
- **The Sun's shadows.** The path does not depend on them. With maps, the forward pass and each map pass draw
  through the stages. With the Sun's shadows off, the forward pass alone does. Under `RAY-TRACING`'s traced
  method the forward pass does, and so does each map pass the renderer still records.
- **Report.** Where the culling mode is reported: the hardware names the launch gave, which were enabled and
  why any was not; the path asked for, the path the scene resolved to and, where the two differ, why; how
  many meshes each path draws; and the limit that excluded each mesh that fell back. The reasons for the
  scene are that the device did not enable the stages and that a stage pipeline failed to build. A fallback
  is never silent.
- **The opt-in.** A person turns the path on for a machine by how they launch: `mesh-stages` as a
  [preferred](#hardware-features-at-launch) name, and a scene that asks for `MeshStages`. The proving ground's
  valley has one launch selection, off by default, that does both; the recipe is chosen as it is today. On a
  Mac whose GPU lacks the stages the same launch draws the existing path and reports why. The opt-in is
  recorded in two places: every run's report says what was asked, enabled and resolved, and this Mac's record
  says what the opt-in was found to do here.
- **This Mac's record (M3).** It is written for a person deciding whether to opt a machine in, and nothing in
  the engine reads it. It holds the machine as the OS reports it (chip, GPU core count, memory, OS version)
  with the registry's digest, the commit, the launch line, the look, and the paired timing. The look is M3's
  inspection of the final build against [M2's list](DONE.json).
- **The paired timing is a tripwire.** On the final build with capture off, on the Apple tile recipe with the
  Sun's shadows as maps, at 1280×720 and at 1920×1080, the valley's route runs from an ordinary launch and
  from the opt-in launch. The two differ in the opt-in selection and in nothing else: the same recipe,
  reconstruction, display range, extent, route and Sun. At each extent they run interleaved, twice each, which
  is eight runs. Two figures are read from every frame:
  - **Frame GPU span:** from the start of the first GPU work the frame encodes to the end of the pass that
    consumes the frame's output. The time between passes and any untimed transfer lie inside it. It is the
    span [ruling 7](../ARCHITECTURE.md#seam-rulings-for-track-5) adds to the frame's timing record.
  - **Frame CPU time:** the frame sample's upload and encode intervals together, which run from the start of
    the frame's uploads to the return of its submission.

  A sum of pass times is not the frame. The runtime's scene sum leaves out the surface pass, untimed
  transfers and the time between passes, and this path removes a pass and changes where passes begin and end.
  Pass times by name explain a difference between two rows and are never the difference.

  The record keeps each run's median of both figures, the median of each pass by name,
  [the counts](#cost-account) and the machine's load. For each figure at each extent it states the difference
  between the paths beside the spread, which is the larger of the two paths' differences between their own
  two runs. Where the spread is the larger it says the pair shows no difference. It is taken while other
  builders work, as every timing of this module is. The stages about 25% above the existing path in either
  figure, or a visible hitch, is investigated and the record says what was found. No figure fails an item,
  and none turns anything on.
- **What is not claimed.** No default, and no performance qualification under C12. That asks for a
  same-device comparison with its image difference and a sustained run, on a quiet machine, and it belongs
  with a decision to make the path a default. The maturity ledger
  records the path as run on this Mac, not performance-qualified and not a default.
- **Why M3 waits for the exit.** M3 waits for `SHOWCASE` M8 so that its inspection, its record and its
  estimate for entities describe the valley as the V1 exit leaves it. Nothing here changes the window's
  default, before the exit or after it. The verifier's runs name no hardware feature and ask for no path, so
  no registered exact baseline moves.

### Check that it can work

- **The API path.** [The probe](../../evidence/RAY-TRACING/planning-probe/README.md) ran object and mesh
  stages with a payload, a uniform and storage buffers on this Mac through the locked wgpu.
- **Limits.** The cooker's meshlets fit a mesh workgroup's output. An object workgroup may launch far more
  mesh workgroups than it owns meshlets. The payload holds 16,352 bytes, and a run of survivor indices is a
  few hundred. A mesh that exceeds a storage binding or a draw's object workgroups falls back by itself. wgpu
  makes a device with limits below the adapter's when asked, so a test shows that fallback on a small mesh.
- **The streams as storage.** Positions and attributes keep their 12-byte records, which no WGSL vector array
  matches, so the mesh stage reads them as arrays of scalars. The packed triangles are read the same way.
- **The fragment contract.** The mesh stage writes the fields the scene's `VertexOutput` declares, with the
  same interpolation, so the fragment entry is shared. M1 compares the path with its cull off against the
  existing path in every frame-contract output.
- **The material.** `material.rs` builds a reload's module by appending the cooked entries to the hooked scene
  source. The hook point is in the fragment path, which the stage entries do not touch, so the stage module is
  built from the same hooked source in the same way.
- **The caster's view.** One layout serves all three maps, and each pass binds it at group 0 before the
  contribution draws (`synthetic/shadow.rs`). Widening a binding's visibility changes no pipeline that reads
  it from the vertex stage, so the existing casters draw as they do. With it the object stage has each map's
  matrix to cull by and the mesh stage has it to place vertices by.
- **The span.** `timing.rs` reads every timed pass's begin and end counters from one batch, the surface
  consumer's among them, so the span is the last end less the first begin, with no new query.
- **Order.** The cooked index buffer is already in meshlet order, so both paths submit triangles in the same
  order if a mesh draw rasterises its workgroups in order. Metal's documentation does not say that it does.
  Order decides a pixel only where two triangles tie in depth, and M1's comparison counts those pixels.

## The system as it runs

1. **Launch.** The host asks for `mesh-stages` because the launch named it, and for no other reason. The
   device enables the stages, or a preferred name is left out, and the registry records which and why.
2. **Preparation.** The cooked contribution resolves its path from the scene's request and the generation.
   For `MeshStages` it compiles the stages' module, creates the forward pipeline, and the shadow pipelines
   where the scene has Sun maps, and keeps the fallback's prepared.
3. **Residency.** Each mesh is found eligible or not when it is staged. An eligible mesh's meshlet storage is
   copied with the mesh.
4. **Each frame.** The forward pass records one mesh draw for each placement of an eligible mesh, and each of
   the Sun's map passes the renderer records takes one for each such caster. No cull pass runs for those meshes. A mesh that fell
   back draws as it does today.
5. **A publication.** The new mesh's eligibility is decided and its storage prepared with it, and the draw
   switches in the publication's frame.
6. **A material reload.** One candidate holds the indexed and the stage pipeline. It is drawn, installed or
   refused whole.
7. **Loss or replacement.** The new generation resolves again, and an accepted material draws once it has
   validated for both pipelines.

## Shared files and exact baselines

This module builds in `thrum-asset-gpu` and, in `thrum-render` and the hosts, at the places
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5) name: the launch function, the
creation ledger, the visibility of the view, light and caster view layouts, and the timing record's span. With no request, and on the direct
and indirect paths, every byte on main is unchanged, and each milestone proves it through
[the exact baselines](../ARCHITECTURE.md#exact-baselines). No baseline is added for the mesh-stage path: its
proofs are [the comparisons above](#what-is-proved-and-how-hard).

## Cost account

Recorded, never a gate (brief decision 3). The tripwires are the brief's: a visible hitch, a comparable figure
about 25% worse than its last record, growth that is not linear in what it should follow.

- **Memory.** A mesh-stage scene holds, for each eligible mesh, 16 bytes for each meshlet, 4 for each meshlet
  vertex and 3 for each triangle more. The existing path's index buffer and draw arguments stay, as the
  prepared fallback. M2 records resident bytes for both.
- **CPU each frame.** One draw for each placement in each pass, where the indirect path records one for each
  meshlet.
- **GPU each frame.** No cull pass. Object workgroups in proportion to meshlets; mesh workgroups in
  proportion to survivors.
- **A material reload.** Two pipelines compiled off the frame where there was one.
- **What the record counts.** In each pass, the meshlets and triangles submitted, and those rejected by the
  cone test and by the frustum test apart, with terrain kept apart from the other cooked meshes. Meshlets
  differ in size, so the triangles are what M3's estimate for entities rests on, and the reason is kept
  because entities already pass a frustum test for each instance.

## Risks

1. **wgpu's mesh shading is experimental.** A defect a builder cannot work around inside Thrum ends the
   milestone `BLOCKED` with the smallest program that shows it. Nothing is posted upstream without the owner.
2. **The stages may not be faster on this GPU.** Then the record says so, the path stays a choice a launch
   can make, and the next plan has its answer about entities. That is a result, not a failure of the module.
3. **The two paths differ by more than rounding.** The early sign is a pixel that differs away from an edge
   or a tie in M1's difference images. The cause is found there, before anything is built on the path.
4. **A hook validates for the indexed pipeline and not for the stage pipeline.** The reload is refused whole
   with the stage pipeline's diagnostics. If the cause is wgpu's, it is risk 1.
5. **Other builders are in `thrum-asset-gpu`.** `SCENE-VIEWS` M2, `CONTENT-LOOP` M3, `RAY-TRACING` M2 and
   `REVIEW-ENGINE` work there. This module adds a path; it rewrites nothing another milestone holds.

## Proof and builder handoff

- GPU tests ask `thrum-test-support`'s gate for a device with `mesh-stages` required, and are skipped with a
  reason where the adapter does not offer the stages. On this Mac a skip fails.
- A case this Mac cannot show by itself is forced: an adapter without the stages through the registry's
  function, small limits through a device made with them, and a failed device request or pipeline through a
  fault the test injects.
- Proofs that need cooked meshes run in the proving ground through a script, so a check names no game package.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, with the
  path behind a selection that is off by default. The capture route makes its own device through the
  registry's function. A capture for an inspected item requires `mesh-stages` and keeps its report of the
  resolved path beside its frames, so they cannot be the fallback's. What the Mac window itself resolves is
  proved from its report (M3.2), not from pictures.
- Every timing of this module is a tripwire taken under load, M3's pair included: it records the load and
  fails only on a missing measurement. No sitting with other builders paused is asked for.
- Scratch goes in `tmp/mesh-shading-<milestone>/`, selected evidence in `docs/evidence/MESH-SHADING/`.

## Open questions

None that blocks a milestone. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
