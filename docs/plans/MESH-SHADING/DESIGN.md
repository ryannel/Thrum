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
half of it. This design is mesh shaders: Metal's object and mesh stages, which wgpu calls the task and mesh
stages.

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
| A game or demo on the Mac | The same scene for less CPU and GPU work, and to see which path drew it | A registry-selected path, a resolved report, and the path it replaces prepared as the fallback, for the scene or for one mesh |
| A creator editing a cooked material or mesh while the scene runs | The edit, on whichever path draws the mesh | One accepted material and one publication for both paths |
| A Mac other than the one measured, or a scene whose Sun shadows are not maps | The launch and the frame it has | No automatic request and no automatic selection outside what was measured |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | The indirect and direct paths untouched |
| A later Vulkan or D3D12 recipe | The same path | wgpu's public API and WGSL only |

**Commissioned**, on `mac-metal`: cooked placements drawn through the object and mesh stages in the main
view's forward pass and in the Sun's shadow passes, selected from the registry, with the existing path as
fallback and reference; a paired record; promotion, by the rule below, for the configuration that was measured.

**Not built**, with the reason:

- **Entities.** The 10,000 entities of the exit scene draw whole mesh rows through `CULLING`'s bucket route,
  culled a whole instance at a time. Drawing them a meshlet at a time is where the larger saving may lie, since
  `SHOWCASE` sizes prop detail to fit the frame. It needs meshlet tables in the mesh arena and a change to the
  route whose every mode is held byte-equal to an oracle. It is planned only if this module's record shows the
  stages win on this GPU ([open for the human](PLAN.md#open-for-the-human)).
- **The default under the traced Sun.** `RAY-TRACING`'s traced method removes cascade passes, and part of this
  path's saving is in those passes. The default is qualified here with the Sun's shadows drawn as maps. A
  default for the two together belongs to the plan that follows both modules' records, and neither waits for
  the other.
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
| **G3 The work is less** | The recorded frame | The valley's route; a mesh wholly in view, partly in view and behind the camera | Counters that fall while time does not. Rejected by paired whole-frame GPU and CPU times |
| **G4 The path is the one reported** | Mac on both recipes, with and without the launch request | Adapters with and without the stages; a mesh the device's limits cannot bind or dispatch; a Mac that was not measured | The indirect path passing as the mesh stages, or a mesh silently left out. Rejected by asserting the resolved path for the scene and for each mesh, and by a count of mesh-stage draws that is not zero |
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
- **The speed.** Tripwire records under load in M2, which pass on their check. The qualified pair is taken
  once, in M3's sitting.

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

As [RAY-TRACING defines it](../RAY-TRACING/DESIGN.md#hardware-features-at-launch): a launch that names
`mesh-stages` in `THRUM_HARDWARE`, or in code, gets a device with the feature and its limits enabled, or a
refusal by name. With no request the device is the one made today. Whichever module's first milestone lands
second extends what the first built.

The request for `mesh-stages` also raises any ordinary limit the stage pipelines need, such as storage buffers
in one stage, and only for a launch that asks. An adapter that cannot grant them refuses the request by name.

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

So a pass rasterises only the meshlets that reach it, and what it stores is what the existing caster stores.

### Selection, fallback and promotion

- **Request.** `CookedDrawPath` gains `MeshStages` beside `Direct` and `Indirect`. A scene's preparation asks
  for `Auto` or names a path.
- **Resolution for the scene.** An explicit `MeshStages` resolves to the stages on a generation that enabled
  them, and is refused by name on one that did not. `Auto` resolves to the stages only inside a qualified
  configuration, defined below, and otherwise to what `COOKED-DRAW` selects today.
- **Resolution for each mesh.** Within a mesh-stage scene, [an ineligible mesh](#residency) draws through the
  fallback. That is reported, not refused.
- **Fallback.** The fallback is the path `COOKED-DRAW` selects for the device, which on the Mac is the
  indirect path. Its pipelines are prepared with the stages', so a stage pipeline that fails to build resolves
  the whole scene to the fallback before the scene is ready.
- **Report.** Where the culling mode is reported: the path asked for, the path the scene resolved to, how many
  meshes each path draws, and the limit that excluded each mesh that fell back.
- **Qualified configuration.** The registry holds a list of the configurations in which the stages measured
  faster. An entry names what was measured: the adapter by the name the registry records for it, the Apple
  tile recipe, and the Sun's shadows drawn as maps. A scene is inside a qualified configuration when its
  device generation's adapter and recipe match an entry, the generation enabled the stages, and the scene's
  resolved Sun shadow method is maps. Until `RAY-TRACING` M3 adds a second method, maps is the only one. The
  list is empty until M3.
- **Promotion (M3).** M3's sitting runs the valley's route on this Mac, on the Apple tile recipe, with capture
  off and the Sun's shadows as maps, at 1280×720 and at 1920×1080. At each extent it takes four runs,
  interleaved: the existing path from an ordinary launch, the mesh stages from a launch that names them, then
  both again. Two figures are read from every frame of a run:
  - **Frame GPU time:** the sum of the GPU intervals of every scene pass in the frame's timing record.
  - **Frame CPU time:** the frame sample's upload and encode intervals together, which run from the start of
    the frame's uploads to the return of its submission.

  A run's value for a figure is the median over the route's frames. At one extent, each figure then has two
  values for each path. Its spread is the larger of the two paths' differences between their own two runs. The
  mesh stages are *no higher* in a figure when their larger value is at most the existing path's smaller value
  plus the spread, *lower* when their larger value is below the existing path's smaller value by more than the
  spread, and *higher* when their smaller value is above the existing path's larger value by more than the
  spread.

  The configuration is qualified when, at both extents, both figures are no higher and at least one is lower.
  The record then says "promoted" and the list gains this Mac's entry. If any figure at either extent is
  higher, the record says "slower". Otherwise it says "no measurable effect". In those two cases the list
  stays empty and the path stays a named choice. The record keeps both figures for every frame of all eight
  runs, so a reader can recompute the conclusion.
- **What promotion changes.** For an ordinary launch, the Mac window asks the registry before it makes the
  device whether the adapter is in a qualified entry for the launch's recipe and offers the stages with the
  limits the path needs. Only then does it add `mesh-stages` to its own request. Otherwise it makes the device
  it makes today. So an automatic request is never refused, and a Mac that was not measured launches and draws
  as it does now. A launch that names `mesh-stages` itself is still enabled or refused by name. A capture and a
  test ask only for what they name, so the verifier's runs enable no feature and resolve to the reference
  paths, and no registered exact baseline moves at promotion. M3 waits for `SHOWCASE` M8, so the window's
  default does not change under the V1 exit either.

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
- **Order.** The cooked index buffer is already in meshlet order, so both paths submit triangles in the same
  order if a mesh draw rasterises its workgroups in order. Metal's documentation does not say that it does.
  Order decides a pixel only where two triangles tie in depth, and M1's comparison counts those pixels.

## The system as it runs

1. **Launch.** The host asks for `mesh-stages`, because the launch named it or because the adapter is in a
   qualified entry and offers it. The device enables the stages and the registry records it.
2. **Preparation.** The cooked contribution resolves its path. For `MeshStages` it compiles the stages'
   module, creates the forward and shadow pipelines and keeps the fallback's prepared.
3. **Residency.** Each mesh is found eligible or not when it is staged. An eligible mesh's meshlet storage is
   copied with the mesh.
4. **Each frame.** The forward pass records one mesh draw for each placement of an eligible mesh, and each of
   the Sun's map passes records one for each such caster. No cull pass runs for those meshes. A mesh that fell
   back draws as it does today.
5. **A publication.** The new mesh's eligibility is decided and its storage prepared with it, and the draw
   switches in the publication's frame.
6. **A material reload.** One candidate holds the indexed and the stage pipeline. It is drawn, installed or
   refused whole.
7. **Loss or replacement.** The new generation resolves again, and an accepted material draws once it has
   validated for both pipelines.

## Shared files and exact baselines

This module builds in `thrum-asset-gpu` and at three places in `thrum-render` and the hosts, under
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5). With no request, and on the direct
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
- **What the record counts.** Meshlets and triangles submitted and rejected in each pass, with terrain kept
  apart from the other cooked meshes. Meshlets differ in size, so the triangles are what M3's estimate for
  entities rests on.

## Risks

1. **wgpu's mesh shading is experimental.** A defect a builder cannot work around inside Thrum ends the
   milestone `BLOCKED` with the smallest program that shows it. Nothing is posted upstream without the owner.
2. **The stages may not be faster on this GPU.** Then the path stays unpromoted, the record says so, and
   entities are not planned. That is a result, not a failure of the module.
3. **The two paths differ by more than rounding.** The early sign is a pixel that differs away from an edge
   or a tie in M1's difference images. The cause is found there, before anything is built on the path.
4. **A hook validates for the indexed pipeline and not for the stage pipeline.** The reload is refused whole
   with the stage pipeline's diagnostics. If the cause is wgpu's, it is risk 1.
5. **Other builders are in `thrum-asset-gpu`.** `SCENE-VIEWS` M2, `CONTENT-LOOP` M3, `RAY-TRACING` M2 and
   `REVIEW-ENGINE` work there. This module adds a path; it rewrites nothing another milestone holds.

## Proof and builder handoff

- Proofs that need cooked meshes run in the proving ground through a script, so a check names no game package.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, with the
  path behind a selection that is off by default.
- Timing inside M1 and M2 is a tripwire taken under load. M3 takes the paired numbers in one sitting with
  other builders paused; its builder asks the lead for it. The sitting comes before the items that read its
  record, which are the decision, the look of a promoted default, the estimate for entities, the public
  documentation and the maturity row.
- Scratch goes in `tmp/mesh-shading-<milestone>/`, selected evidence in `docs/evidence/MESH-SHADING/`.

## Open questions

None that blocks a milestone. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
