# Mesh shading: meshlets culled and drawn by the object and mesh stages on the Mac

17 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-5-ray-tracing-and-mesh-shaders-on-the-mac) · [the brief](../BRIEF.md) ·
[planning probe](../../evidence/RAY-TRACING/planning-probe/README.md). C04, C05, C06, C07 and C12 in
[CONTRACTS](../../design/CONTRACTS.md), [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)
and [the cooked draw design](../COOKED-DRAW/DESIGN.md) govern.

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
| A game or demo on the Mac | The same scene for less CPU and GPU work, and to see which path drew it | A registry-selected path, a resolved report, the indirect path prepared as the fallback |
| Chrome, the iPhone simulator, a Mac on the portable recipe | The frame they have | The indirect and direct paths untouched |
| A later Vulkan or D3D12 recipe | The same path | wgpu's public API and WGSL only |

**Commissioned**, on `mac-metal`: cooked placements drawn through the object and mesh stages in the main
view's forward pass and in the Sun's shadow passes, selected from the registry, with the indirect path as the
reference; a paired record; promotion by the rule below.

**Not built**, with the reason:

- **Entities.** The 10,000 entities of the exit scene draw whole mesh rows through `CULLING`'s bucket route,
  culled a whole instance at a time. Drawing them a meshlet at a time is where the larger saving may lie, since
  `SHOWCASE` sizes prop detail to fit the frame. It needs meshlet tables in the mesh arena and a change to the
  route whose every mode is held byte-equal to an oracle. It is planned only if this module's record shows the
  stages win on this GPU ([open for the human](PLAN.md#open-for-the-human)).
- **Occlusion culling of meshlets.** `CULLING`'s depth pyramid holds entities only, and a cooked placement is
  never an occluder in it, so there is nothing to test terrain meshlets against.
- **Levels of detail and cluster hierarchies.** The cooker makes one level.
- **Secondary views.** A view of `SCENE-VIEWS` draws cooked meshes by the path that module resolves.
- **Compressed vertex streams decoded in the mesh stage.** The cooked format is unchanged.
- **Chrome and the iPhone simulator.** Neither offers the stages.

## Cues

| Cue | Where it must read | Range | A plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **G1 The same picture** | The valley's terrain and cooked meshes, in the main view and in their shadows | Every cooked mesh in the valley; 1 to the valley's largest meshlet count; camera inside, beside and far from a mesh; noon and low Sun | A path that drops a meshlet nobody is looking at. Rejected by byte equality between the path with its cull on and with it off, on views that hold back-facing and off-screen meshlets |
| **G2 Nothing pops** | The camera in motion and across a cut | Snap turns, a mesh crossing the near plane, an edit, a device replacement | A cull tested at rest. Rejected by the same equality at every frame of a scripted path |
| **G3 The work is less** | The recorded frame | The valley's route; a mesh wholly in view, partly in view and behind the camera | Counters that fall while time does not. Rejected by paired pass and whole-frame times |
| **G4 The path is the one reported** | Mac on both recipes, with and without the launch request | Adapters with and without the stages | The indirect path passing as the mesh stages. Rejected by asserting the resolved path and by a count of mesh-stage draws that is not zero |

The bar is a shipped GPU-driven renderer's meshlet path, such as the mesh-shader paths of Alan Wake 2 and of
Unreal's Nanite fallback raster: the player never sees it work. Here that is held to something checkable:
equality with the same path drawing every meshlet.

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
- **Why not Metal's indirect command buffers.** They are Apple's own answer to GPU-written draw lists. wgpu
  does not expose them, and the [boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) keeps public
  wgpu for all it can express.

**Selected:** cull in the object stage with the rule the compute pass uses, draw in the mesh stage from the
cooked meshlet tables, and keep the indirect path prepared as fallback and reference.

## The oracle

- **Within the path.** The mesh-stage path with its cull turned off draws every meshlet. The cull only removes
  meshlets that lie outside the view or face away, so with it on, the submitted bytes are the same. That
  equality is exact, because both arms run the same shader.
- **Across paths.** The indirect path is the reference for what the scene looks like. Its vertex stage and
  the mesh stage are compiled separately, and Metal's fast-math may round them differently, so equality across
  paths is not promised. M1 measures the difference on a fixed view and states its cause; later checks hold
  runs to that record. A difference away from a triangle's edge is a defect.

## Mechanisms

### Hardware features at launch

As [RAY-TRACING defines it](../RAY-TRACING/DESIGN.md#hardware-features-at-launch): a launch that names
`mesh-stages` in `THRUM_HARDWARE`, or in code, gets a device with the feature and its limits enabled, or a
refusal by name. With no request the device is the one made today. Whichever module's first milestone lands
second extends what the first built.

Pipelines for the stages are created through the device's creation authority, which gains a third entry
beside `render` and `compute`, so the creation ledger counts them.

### Residency

On a generation that enabled the stages, a resident cooked mesh also holds on the GPU, as storage the stages
can read: its positions and shading attributes, its meshlet table, its meshlet-vertex list and its packed
meshlet triangles. The cooked product already carries all of them (`thrum-asset`'s `Meshlet`, and the
meshlet-vertex and meshlet-triangle sections), so the cooker and the product format do not change. They are
part of the mesh's residency: copied under the same upload budget, shown and retired with the mesh, and
switched by [a content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) like the rest
of it.

A generation without the stages uploads none of this.

### The draw (M1)

One draw for each placement: `draw_mesh_tasks` with enough object workgroups to cover the mesh's meshlets.

- **Object stage.** Each workgroup owns a fixed run of meshlets. It tests each against the view with
  `cull.wgsl`'s rule, the same function text compiled into both paths: the five frustum planes against the
  bounding sphere, then the normal cone against the camera. It writes the survivors' indices to the payload
  and launches one mesh workgroup for each.
- **Mesh stage.** A workgroup reads its meshlet from the payload, fetches the meshlet's vertices through the
  meshlet-vertex list, and writes for each vertex exactly what `cooked_vertex` writes: clip position,
  unjittered clip position, world position, normal, texture coordinate, the history flag and the clip delta
  for motion. It writes the meshlet's triangles from the packed list.
- **Fragment stage.** The scene's own fragment entry, unchanged. So a mesh-stage fragment is lit, shadowed and
  writes motion and reactive coverage as the indirect path's does.
- **The placement's record.** The model matrix and history flag reach the stages from the placement's uniform
  record, where the vertex path takes them as instance attributes.

The stages' module is a second compilation of the scene's composed shader with the stage entries and the
`enable wgpu_mesh_shader` directive added. The scene's own modules are not touched. On a generation that
enabled the stages, the scene's view and light group layouts are visible to the object and mesh stages as they
are to the vertex stage.

### The shadow passes (M2)

A Sun shadow pass draws a cooked mesh through the same stages with the light's view and projection: the
object stage tests the bounding sphere against the light's frustum, with no cone test, and the mesh stage
writes position only. So a cascade rasterises only the meshlets that reach it.

### Selection, fallback and promotion

- **Request.** `CookedDrawPath` gains `MeshStages` beside `Direct` and `Indirect`. A scene's preparation asks
  for `Auto` or names a path.
- **Resolution.** `Auto` is what `COOKED-DRAW` selects today unless the registry lists the mesh stages as
  promoted for the resolved recipe and the generation enabled them. An explicit `MeshStages` on a generation
  without the stages is refused by name. A mesh whose cooked limits exceed the device's mesh-stage output
  limits keeps the indirect path, counted and reported; no such mesh exists at the cooker's 64 and 124.
- **Fallback.** The indirect path's pipelines and buffers are prepared with the mesh stages', so a rejected
  pipeline falls back before the scene is ready and nothing is compiled during play.
- **Report.** The resolved path is reported where the culling mode is.
- **Promotion (M3).** The mesh stages are promoted on the Apple tile recipe only if, in M3's paired sitting on
  the valley's route at both extents, the whole frame's GPU time and the CPU time to record and submit it are
  each no higher than with the indirect path, beyond the spread between two runs of the same configuration,
  and at least one is lower by more than that spread, in both runs. Otherwise the path stays a named choice and
  the record says "no measurable effect" or "slower".
- **What promotion changes.** Once the path is promoted, the Mac window asks for the stages itself when its
  recipe is Apple tile. A capture and a test ask only for what they name. So the verifier's runs enable no
  feature and resolve to the reference paths, and no registered exact baseline moves at promotion. M3 waits
  for `SHOWCASE` M8, so the window's default does not change under the V1 exit either.

### Check that it can work

- **The API path.** [The probe](../../evidence/RAY-TRACING/planning-probe/README.md) ran object and mesh
  stages with a payload, a uniform and storage buffers on this Mac through the locked wgpu.
- **Limits.** The cooker's meshlets fit a mesh workgroup's output. An object workgroup may launch far more
  mesh workgroups than it owns meshlets. The payload holds 16,352 bytes, and a run of survivor indices is a
  few hundred.
- **The fragment contract.** The mesh stage writes the fields the scene's `VertexOutput` declares, with the
  same interpolation, so the fragment entry is shared. M1's first proof is that a mesh drawn with no culling
  matches the indirect path within the measured difference.
- **Order.** The cooked index buffer is already in meshlet order, so both paths submit triangles in the same
  order if a mesh draw rasterises its workgroups in order. Metal's documentation does not say that it does.
  Order decides a pixel only where two triangles tie in depth, and M1's measured difference shows whether any
  do.

## The system as it runs

1. **Launch.** The host asks for `mesh-stages`. The device enables them and the registry records it.
2. **Preparation.** The cooked contribution resolves its path. For `MeshStages` it compiles the stages'
   module, creates the forward and shadow pipelines and keeps the indirect path's prepared.
3. **Residency.** Each mesh's meshlet storage is copied with the mesh.
4. **Each frame.** No cull pass runs for a mesh on this path. The forward pass records one mesh draw for each
   placement; each shadow pass records one for each caster.
5. **A publication.** The new mesh's storage is prepared with it and the draw switches in the publication's
   frame.
6. **Loss or replacement.** The new generation resolves again.

## Shared files and exact baselines

This module builds in `thrum-asset-gpu` and at three places in `thrum-render` and the hosts, under
[the seam rulings for track 5](../ARCHITECTURE.md#seam-rulings-for-track-5). With no request, and on the direct
and indirect paths, every byte on main is unchanged, and each milestone proves it through
[the exact baselines](../ARCHITECTURE.md#exact-baselines). No baseline is added for the mesh-stage path: its
proofs are the two equalities above.

## Cost account

Recorded, never a gate (brief decision 3), with the brief's tripwires.

- **Memory.** On a mesh-stage generation a mesh also holds 16 bytes for each meshlet, 4 for each
  meshlet vertex and 3 for each triangle. The indirect path's index buffer and draw arguments stay, as the
  prepared fallback. M2 records resident bytes for both.
- **CPU each frame.** One draw for each placement in each pass, where the indirect path records one for each
  meshlet.
- **GPU each frame.** No cull pass. Object workgroups in proportion to meshlets; mesh workgroups in
  proportion to survivors.

## Risks

1. **wgpu's mesh shading is experimental.** A defect a builder cannot work around inside Thrum ends the
   milestone `BLOCKED` with the smallest program that shows it. Nothing is posted upstream without the owner.
2. **The stages may not be faster on this GPU.** Then the path stays unpromoted, the record says so, and
   entities are not planned. That is a result, not a failure of the module.
3. **The two paths differ by more than rounding at edges.** M1 finds the cause before anything is built on
   it.
4. **Other builders are in `thrum-asset-gpu`.** `SCENE-VIEWS` M2, `CONTENT-LOOP` M3, `RAY-TRACING` M2 and
   `REVIEW-ENGINE` work there. This module adds a path; it rewrites nothing another milestone holds.

## Proof and builder handoff

- Proofs that need cooked meshes run in the proving ground through a script, so a check names no game package.
- Visual items use T2 engine capture on `mac-metal`, `fixed` for stills and `realtime` for motion, with the
  path behind a selection that is off by default.
- Timing inside M1 and M2 is a tripwire taken under load. M3 takes the paired numbers in one sitting with
  other builders paused; its builder asks the lead for it.
- Scratch goes in `tmp/mesh-shading-<milestone>/`, selected evidence in `docs/evidence/MESH-SHADING/`.

## Open questions

None that blocks a milestone. What the owner may want decided is in [the plan](PLAN.md#open-for-the-human).
