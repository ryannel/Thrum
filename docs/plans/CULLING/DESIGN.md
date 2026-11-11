# Culling: only what the camera can see is drawn (G3c)

11 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C04, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), the
[graph decision](../../design/decisions/G3-EXECUTING-RENDER-GRAPH.md) and the
[hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

## The request

The owner's planning task of 11 November 2026, word for word:

> Plan the modules CULLING and EFFECT-GRAPH from docs/plans/ARCHITECTURE.md: write each one's DESIGN, PLAN and
> done-when file. Both depend on APPLE-TILE M2, which is in its second review now, so builders can start them once
> it passes. Everything else is waiting on APPLE-TILE M2 or M3, so these are the next work. Keep each first
> milestone small enough to start right after APPLE-TILE M2 lands. Also: APPLE-PRESENTATION M1's jitter change broke
> APPLE-TILE's exact baseline comparisons, and nothing caught it before it merged
> (docs/evidence/APPLE-TILE/M2-review.md MF1). Say in ARCHITECTURE.md or the affected designs how a module's change
> that alters another module's exact baselines gets caught before landing.

What the module must deliver comes from [the brief](../BRIEF.md):

- the V1 exit (decision 1) lists "GPU culling" and says "Keep 10,000 entities and 1,000–2,000 movers as the
  representative scene";
- gate G3c: "Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per mesh and pipeline
  bucket, stable mesh offsets and compute-readable materials. Validate visibility and image equivalence; record
  full and partial-visibility cost and unexpected growth as tripwires";
- decision 3: numeric performance figures are baselines, not gates.

This design reconciles work that already exists; it is not a blind study. Read for it: the four G3c evidence pages
([camera culling](../../evidence/render/G3C-CAMERA-CULLING-2026-11-07.md),
[field Frustum](../../evidence/render/G3C-FIELD-FRUSTUM-2026-11-07.md),
[the occlusion experiment](../../evidence/render/G3C-OCCLUSION-EXPERIMENT-2026-11-07.md),
[shipped Occlusion](../../evidence/render/G3C-OCCLUSION-M3A-2026-11-07.md)), the earlier session's uncommitted
design and plan, and `crates/thrum-render/src/visibility*` and `synthetic/{camera_culling,mesh_route,buckets}.rs`
on main. This file replaces that earlier design as the governing one.

## Purpose and scope

The camera's opaque draw cost follows what the player can see, not what exists, and nothing visible is ever
removed. Consumers: the V1 game's entity field and valley, `SHOWCASE`'s exit scene, and any scene that sets a
culling mode.

**Built and on main** (kept; the code and the evidence pages are the authority):

- `CullMode::Direct`, `Frustum` and `Occlusion`, chosen at launch, with read-back draw counts that never hold a
  frame.
- Frustum: one compute compaction in slot order and one indexed indirect draw. Byte-equal to Direct over the
  100,513-instance field flythrough, natively and in Chrome.
- Occlusion on the cube route: a depth-only prepass of last frame's visible set, a min depth pyramid, a final cull
  of every slot and one slot-ordered final draw. Byte-equal to Direct, with a derived depth tolerance and a widened
  footprint.
- The bucket route for scenes that select meshes: one indexed indirect draw per (mesh row, pipeline class), in
  Direct and Frustum, with GPU-written instance counts.
- Sun-caster culling per light volume on the bucket route.

**Commissioned here**, in this order:

1. Occlusion on the bucket route, so it works on realistic meshes, with the mesh tie witness the evidence owes.
2. The mode follows the device and the measurements: a capability-selected fallback to Direct, Chrome on mixed
   meshes, the field default decided from paired rows, and a live switch.
3. The valley's 10,000 entities culled by default with no pixel changed, on Mac and Chrome, and the gate closed.

**Not built here:** LOD selection; several cameras; meshlet culling of field meshes; a visibility buffer; Metal
indirect command buffers; GPU-written draw *counts* (`MULTI_DRAW_INDIRECT_COUNT` has no Metal or WebGPU support in
wgpu 30); and the native-only culling recipes (subgroup compaction, first-instance draws), which stay candidates for
the performance phase the brief sets after the exit.

## Cues

| Cue | Where it must read | Range | Plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **C1 Nothing visible disappears or pops** | The game camera in motion, Mac and Chrome | 1 to 100,513 instances; 0–100% in view; snap turns, cuts, resize, device replacement, dropped frames, slot churn; moving and mirrored instances; near-plane crossings | A test that passes at sampled frames or on scenes without ties. Rejected by byte equality with the oracle at every tick of a scripted path, plus tie witnesses |
| **C2 Occluders hide the work behind them** | Flying behind walls and props in the mixed-mesh field | One wall to full cover; stepping out of cover | Counters that fall while the forward cost does not. Rejected by paired per-pass GPU time on meshes with real vertex work |
| **C3 The mode is right for the device and the scene** | Mac both recipes, Chrome, the iOS simulator | Adapters with and without indirect execution; cheap cubes to meshes of a few thousand triangles | A culled mode that silently runs Direct, making every equality test vacuous. Rejected by asserting the resolved mode and a nonzero hidden count |
| **C4 A creator can flip the mode and see no change** | The game's controls on Mac and in Chrome | Any mode to any mode, while moving | A switch that stalls or compiles in the frame. Rejected by creation counters and frame records across the switch |

The bar is a shipped GPU-driven renderer, where culling is invisible: the player never sees it work. Here that is
held to something stricter and checkable, byte equality with drawing everything.

## The oracle

**Direct** draws every active slot in slot order with no culling resources. With opaque writes and a strict
`Greater` depth test, a pixel belongs to the first fragment in draw order with the greatest depth.

- **Frustum** keeps draw order, so it is byte-equal to Direct.
- **Occlusion** draws its final list in slot order, so it is Direct's sequence minus slots that win no pixel, and
  is byte-equal to Direct.
- **The bucket route** changes order across buckets by construction. Its oracle is **bucket-ordered Direct**: each
  bucket's members in logical-slot order, buckets in key order (mesh row, then class), each with its class
  pipeline. Frustum and Occlusion on the bucket route are byte-equal to it. It never replaces Direct for scenes
  that select no meshes.

Chrome modes are compared with Chrome's own Direct through the page's readback. Native-to-Chrome agreement is not
claimed.

## Research and selection

- **Two-phase occlusion from last frame's visible set.** Haar and Aaltonen's
  [GPU-Driven Rendering Pipelines](https://advances.realtimerendering.com/s2015/aaltonenhaar_siggraph2015_combined_final_footer_220dpi.pdf)
  (SIGGRAPH 2015) cull instances, then clusters, with a two-phase occlusion test; Karis's
  [Nanite](https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf) (2021) keeps
  the same shape and rejects depth reprojection as not conservative. Bevy's experimental occlusion culling draws
  last frame's visible meshes in an early prepass, builds a min pyramid and tests every bounding box; its docs say
  it is off by default because the pyramid and tests cost time, and to enable it after measuring
  ([docs mirror](https://doc.qu1x.dev/bevy_trackball/bevy/render/occlusion_culling/struct.OcclusionCulling.html);
  I read the search summary of those pages on 11 November, not the source). Unity 6 tests instance bounds against
  depth pyramids from the camera
  ([manual](https://docs.unity3d.com/6000.2/Documentation/Manual/urp/gpu-culling.html)).
- **What Thrum selected and why.** The visible-last-frame bit picks occluders, as in those systems. The colour
  draw is moved after the test and kept in slot order, because the split forward pass of the first experiment
  handed equal-depth pixels to the wrong surface (3,216 pixels in the tie witness). The alternatives costed then
  (a slot-ID target, a depth bias, reversed late order) are in
  [the experiment's record](../../evidence/render/G3C-OCCLUSION-EXPERIMENT-2026-11-07.md).
- **Instance, then meshlet.** Both published chains cull whole instances first and clusters second. Thrum has both
  stages but in different modules; [one authority per entity](#one-visibility-authority-per-entity) settles which
  one answers for each entity.
- **What would change the selection.** If the pyramid cannot be made conservative on some mesh, the fallback is
  raster-proxy occlusion (bounds drawn against the prepass depth with early tests). If Occlusion never wins on any
  shipped scene, it stays selectable and off by default.

## Mechanisms

### Built: Frustum

A conservative world box (`|basis|·extent` around the interpolated translation the vertex shader uses) against the
six reverse-Z clip planes of the frame's `view_proj`. Compaction is a per-workgroup count, a one-workgroup offset
scan and a mask-only scatter, which keeps order.

### Built: Occlusion

1. **Prepass list** (compute): slots whose visible-last bit is set and that pass the frustum test, in order.
2. **Depth prepass:** that list drawn depth-only into a private stored `Depth32Float` target, with the forward
   vertex entry, primitive state, index buffer, view group and extent. `@invariant` on the position output keeps
   the two pipelines' depth identical.
3. **Pyramid:** a min reduction (the farthest depth under reverse-Z) of the prepass depth.
4. **Final cull** (compute): every active slot gets the frustum test, then the pyramid test. Survivors are
   compacted in order, and the same pass writes the next visible-last bits.
5. **Forward:** unchanged, drawing the final list into cleared depth.

**Why it equals the oracle.** The prepass draws only geometry the oracle draws, at this frame's transforms, so at
every pixel the oracle's final depth is at least the prepass depth. A slot is dropped only when its nearest depth
is behind the pyramid's minimum over its footprint by more than ε, so it is strictly behind the final depth
everywhere it could cover and wins nothing. Equality counts as visible, so ties are never dropped. The bits only
choose occluders: a stale bit costs time, never a pixel.

<a id="depth-tolerance"></a>

**Depth tolerance.** ε is derived, not tuned: `ε = 2γ16·(Z + |nearest|·W)/w + 10 ulp(nearest)`, with
`γ16 = 16u/(1 − 16u)`, `u = 2⁻²⁴`, and `Z`, `W` the clip-row magnitudes over every world coordinate either path
forms. It bounds the rounding difference between the vertex shader's clip position and the cull's, plus the two
divides, the rasterizer's interpolation and the compare. The cull also widens each slot's texel rectangle by the
x/y rounding bound before flooring. `visibility/occlusion.wgsl` and its mirror `occlusion_reference.rs` hold the
exact form, and GPU tests pin the shader's slack to the reference at its boundary. The rasterizer's four-ulp
interpolation term is assumed; on cubes a zero-tolerance mutant still passes, so the witness that needs the bound
is a mesh face lying on its own bounds at a rounded far pose (M1).

### Built: the bucket route and Sun-caster culling

A scene may give each logical slot a mesh from a fixed per-preparation arena (`SyntheticMeshes`). Each (mesh row,
pipeline class) bucket owns a segment of a member list, aligned to 256 slots so its draw binds the list at a
CPU-known dynamic offset with `first_instance` 0, as WebGPU requires. The class (ordinary, mirrored, two-sided)
comes from the upload's determinant. The compaction runs once over the concatenated segments and writes one
indirect record per bucket. Sun casters are culled per Sun pass against that pass's own raster volume, never
against the camera.

### M1: Occlusion on the bucket route

Occlusion is refused today for scenes that select meshes (`admit_meshes` returns `SceneCapacity`), because the
visible-last bit sat at the member's list index, which a layout change hands to another slot.

- **History follows the logical slot.** One bit per logical slot (`InstanceRecord.ids[0]`), both sides sized to
  capacity. The prepass list reads the bit of each listed member. The final cull sets the next side's bit for each
  member it keeps, and the frame clears the whole next side first. Bucket layout and physical rows never move a
  bit.
- **A depth-only twin per bucket pipeline.** Each class's prepass pipeline shares its forward pipeline's module,
  vertex entry, primitive state and depth state. Matching the class is what keeps the proof above true: a
  two-sided prepass over a back-culled open mesh would write depth the final draw never writes.
- **Unchanged:** the pyramid, the test and its ε, the four pass names, and the final draw, which is the Frustum
  bucket draw over the final list. Member order inside a bucket is kept, so equal-depth winners match the oracle.
- **Mutants that must fail:** a two-sided prepass on a back-culled open mesh; the final draw reading the prepass
  list; zero tolerance on the mesh tie witness.

**Check that it can work.** The equality argument above does not depend on the mesh, only on prepass parity and a
conservative test. Parity holds per bucket by sharing the pipeline state. The test is conservative for any mesh
inside its local bounds, which admission computes from the vertices. Where it pays is known in outline. The
earlier session's harness (its logs are not committed, so this is a prediction for M2 to test, not evidence)
measured about 22,000 frustum-passing 1,024-triangle spheres at 7.2 ms of forward time at 720p, and 0.8–1.4 ms in
this prepass shape. The committed rows show the other end: on 12-triangle cubes Occlusion costs about 0.3 ms more
than Frustum at 1080p. So it is a per-scene choice.

### One visibility authority per entity

Two visibility systems exist: instance culling here, and per-meshlet culling of cooked placements in
`thrum-asset-gpu`. The ruling, recorded in [the module list](../ARCHITECTURE.md#interfaces):

- An **entity** (a row of the tick page drawn through the instance field) is culled per instance by this module
  and draws a whole mesh row. Its mesh comes from the scene's mesh selection, whoever produced the vertices.
- A **cooked placement** (terrain, a hero mesh) is culled per meshlet by `thrum-asset-gpu`. It is never an
  occluder for instance culling, which only makes the instance test more conservative.
- Nothing is in both. Chaining instance culling into meshlet culling for instanced cooked meshes is not built for
  V1; a few thousand triangles per prop do not need it.

`CONTENT-LOOP` supplies cooked meshes as mesh rows for the exit scene. This module does not wait for that: it
proves the valley's entities on the meshes that exist, and `SHOWCASE` reruns the same commands on its scene.

### M2: the mode follows the device and the measurements

- **Capability fallback.** Culled modes draw with `draw_indexed_indirect` and compute. The requested mode resolves
  from the capability record, as [COOKED-DRAW](../COOKED-DRAW/DESIGN.md) does: a record without adapter-read
  indirect execution, or with unknown facts, resolves to Direct. No pass probes the device. The resolved mode and
  the reason are reported beside the requested one, and every equality test asserts the resolved mode, so a
  fallback can never pass as culling.
- **Default by measurement.** A scene's default is Frustum. It becomes Occlusion only when paired whole-GPU rows on
  that scene favour it in two separate runs, behind cover and in the open, at 1920×1080. Scenes fully in view may
  keep Direct. The rows and the reason are recorded either way.
- **Live switch.** The mode is fixed per prepared scene. A typed control requests the same scene prepared with
  another mode through the host's existing scene republication, off the frame path. Frames keep presenting the old
  mode until the new one is ready, the swap happens at a frame boundary, the visible-last bits start empty and
  the old resources retire after their GPU work completes. No pipeline is created on the frame path. Direct is the
  oracle a creator can flip to.

### M3: the valley's entities

The valley keeps its content. Its 10,000 entities move from Direct to the default the rule above gives, on the cube
route, where Frustum and Occlusion are byte-equal to Direct, so no retained image changes. An opt-in mesh selection
for the same entities (as `THRUM_FIELD_MESHES` does for the field) runs the bucket route with its Sun-caster
culling against bucket-ordered Direct, which is the configuration `SHOWCASE` adopts.

**Under temporal sampling.** The prepass, both culls and the forward pass read one view record, so a jittered
projection moves them together and the proof is unchanged. Motion and reactive outputs come from the forward pass,
which is untouched. M3 proves equality with temporal sampling selected.

**Compute-readable materials.** The gate asks that compute can read the material table. The one existing material
buffer gains compute visibility; no second table is made.

## The system as it runs

| State | Owner |
|---|---|
| Visibility pipelines, the visible-last bits, the mode resolution | This module (`thrum-render::visibility`, `synthetic/camera_culling.rs`) |
| Bucket membership and the mesh arena | This module (`synthetic/{buckets,mesh_route,mesh_arena}.rs`), maintained by the synthetic upload |
| Per-slot visible lists, indirect records, the prepass depth and pyramid | The frame slot's target, declared in the graph |
| Graph semantics, the split graph, attachment storage | `APPLE-TILE` |
| The view record, jitter, motion and reactive targets | `APPLE-PRESENTATION` |
| Cooked placements and meshlet culling | `CONTENT-LOOP` (`thrum-asset-gpu`) |
| Scene content, routes and the controls | The game |

**An Occlusion frame on the bucket route**, 100,000 slots, with the costs measured on cubes on a shared GPU:
clear the next bits (12.5 KB); prepass list (0.02–0.14 ms); depth prepass per bucket (0.02–0.34 ms, more with
vertex work); pyramid (0.08–0.09 ms at 720p, one dispatch per level); final cull (0.03–0.10 ms); forward over the
final list; then history, exposure and display as before. Extra memory per render target is the prepass depth, the
pyramid and a second list: about 13.8 MB per host at 720p and 27.8 MB at 1080p, logical sizes.

**Transitions.** A camera cut or a slot swap leaves stale bits, which cost one frame of prepass efficiency. Resize
reallocates the pyramid and keeps the bits. Device replacement rebuilds everything and clears the bits. A dropped
submission leaves the bits as of the last executed final cull. A capture does not advance them. A mode switch is a
new prepared scene.

## Exact baselines

[The module list](../ARCHITECTURE.md#exact-baselines) says how a change that alters another module's exact images
is caught. For this module:

- It must not move `APPLE-TILE`'s M0 baseline. Every milestone reruns that baseline's full matrix at the claimed
  commit as an automated check.
- Its own exact comparisons are relative (each mode against its oracle in the same run), so they hold no retained
  bytes. Another module's builder must not loosen them.
- M3 changes a default. It is allowed only because the new default is byte-equal to the old one; the guard in the
  land gate then keeps checking that for free.

## Risks

1. **False culling at a silhouette or tie on real meshes.** Early sign: the mesh tie witness or the flythrough
   differs from the oracle. Fallback: raster-proxy occlusion; until then Occlusion stays off by default.
2. **Occlusion costs more than it saves.** Apple's hidden-surface removal already discards hidden fragments, so
   only vertex and binning work is saved. Early sign: M2's paired rows. Outcome: Frustum stays the default.
3. **A silent fallback hides a failure.** Mitigation: resolved-mode and hidden-count assertions in every witness.
4. **Shared files move under the builder.** `scene_graph.rs` and `synthetic.rs` are also edited by `APPLE-TILE` M3,
   `APPLE-PRESENTATION` M2 and `EFFECT-GRAPH`. Integrate sequentially and rerun the exact checks after every rebase.
5. **wgpu's indirect-call validation adds untimed GPU work.** It stays on in shipped builds and is bounded once
   with a bench-only comparison.

## Performance

Costs are recorded, never gated ([performance rules](../../../.agents/skills/thrum-engineering/references/performance.md)).
Paired, interleaved rows through the frame host's pass-name timing; p50 and p95; two separate runs; the load
recorded and every row labelled contended, because the GPU is shared. Tripwires: about 25% paired regression
against the previous checkpoint at the same configuration; per-instance cost that stops being flat from 10,000
slots to the prepare limit; per-bucket cost that grows from 1 to 64 buckets; allocation growth per encode; a
visible hitch at a mode switch. No quiet-GPU figure exists, and none is claimed.

## Proof and builder handoff

- **Targets.** `mac-metal` for every milestone, on both recipes where a recipe is named. `chrome-webgpu` through
  `scripts/check_v1_browser.mjs`, as the existing field cases run. `ios-sim` for function only: the simulator has
  no indirect execution, so it proves the fallback, never a culled mode.
- **Capture.** Equality is proved in-process by reading back each mode's target in one run. The one inspected item
  (M3) uses T2 engine capture and names its mode: `fixed` for stills, `realtime` for the pan.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/culling-<milestone>/`; selected evidence in
  `docs/evidence/CULLING/`.
- **Maturity.** The `G3c_gpu_culling` row of [the ledger](../../design/maturity.json) is updated at each milestone,
  with implementation, correctness, visuals and device results kept apart.

## Open questions

| Question | Settled by |
|---|---|
| Does the derived ε hold on a mesh face lying on its bounds? | M1's tie witness and its zero-tolerance mutant |
| Does Occlusion win on realistic meshes at 1080p on this Mac? | M2's paired rows |
| Does republication for a live switch restage cooked content visibly? | M2's switch test; if it does, the switch keeps the contribution's residency and the finding goes to `CONTENT-LOOP` |
