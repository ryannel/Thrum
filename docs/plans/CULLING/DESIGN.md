# Culling: only what the camera can see is drawn (G3c)

11 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../VISION.md). C04, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md), the
[graph decision](../../design/decisions/G3-EXECUTING-RENDER-GRAPH.md) and the
[hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

## The request

The owner's planning task of 11 November 2026, word for word:

> Plan the modules CULLING and EFFECT-GRAPH from docs/plans/ARCHITECTURE.md: write each one's DESIGN, PLAN and
> done-when file. Both depend on APPLE-TILE WP2, which is in its second review now, so builders can start them once
> it passes. Everything else is waiting on APPLE-TILE WP2 or WP3, so these are the next work. Keep each first
> milestone small enough to start right after APPLE-TILE WP2 lands. Also: APPLE-PRESENTATION WP1's jitter change broke
> APPLE-TILE's exact baseline comparisons, and nothing caught it before it merged
> (.orchestrate/evidence/APPLE-TILE/WP2-review.md MF1). Say in ARCHITECTURE.md or the affected designs how a module's change
> that alters another module's exact baselines gets caught before landing.

What the module must deliver comes from [the brief](../VISION.md):

- the V1 exit (decision 1) lists "GPU culling" and says "Keep 10,000 entities and 1,000–2,000 movers as the
  representative scene";
- gate G3c: "Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per mesh and pipeline
  bucket, stable mesh offsets and compute-readable materials. Validate visibility and image equivalence; record
  full and partial-visibility cost and unexpected growth as tripwires";
- decision 3: numeric performance figures are baselines, not gates.

This design reconciles work that already exists; it is not a blind study. Read for it: the four G3c evidence pages
([camera culling](../../../.orchestrate/evidence/render/G3C-CAMERA-CULLING-2026-11-07.md),
[field Frustum](../../../.orchestrate/evidence/render/G3C-FIELD-FRUSTUM-2026-11-07.md),
[the occlusion experiment](../../../.orchestrate/evidence/render/G3C-OCCLUSION-EXPERIMENT-2026-11-07.md),
[shipped Occlusion](../../../.orchestrate/evidence/render/G3C-OCCLUSION-M3A-2026-11-07.md)), the earlier session's uncommitted
design and plan, and `crates/thrum-render/src/visibility*` and `synthetic/{camera_culling,mesh_route,buckets}.rs`
on main. This file replaces that earlier design as the governing one. For the revision after
[the design review](../../../.orchestrate/evidence/CULLING/design-review.md), also read: `runtime.rs` (`visual_prepare_inputs`,
`publish_visual`, `rebuild_contribution`), `runtime/visual.rs` and `rebuild_inner` in `thrum-asset-gpu`.

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
   meshes, and the field default decided from paired rows.
3. The valley's 10,000 entities culled by default and equal to Direct in every pixel, on Mac and Chrome, and the
   gate closed. That proof needs Direct itself to be one image, so WP3 also makes the forward pass draw
   [one image under load](#m3-one-image-under-load).

**Not built here:** LOD selection; several cameras; meshlet culling of field meshes; a visibility buffer; Metal
indirect command buffers; GPU-written draw *counts* (`MULTI_DRAW_INDIRECT_COUNT` has no Metal or WebGPU support in
wgpu 30); and the native-only culling recipes (subgroup compaction, first-instance draws), which stay candidates for
the performance phase the brief sets after the exit.

**A live mode switch is not built either.** Neither the V1 exit nor gate G3c asks for one, and the mode is chosen
when a scene is prepared, at launch or page load. The first version of this design promised a switch through the
host's scene republication. The design review showed that path cannot deliver it: the publication entry points
are compiled only for native targets, so Chrome has none; preparation takes no requested mode, and publication
leaves the host's retained scene on the old one; and publication rebuilds the cooked contribution's pipelines
synchronously, where a failure detaches the cooked scene after the switch has been committed. A switch that works
is a transaction across the frame runtime, the browser host and the contribution seam, which this module does not
own and the gate does not need. If the owner asks for one, it is planned as its own milestone with those owners.
The mechanism to cost first is preparing a requested set of modes inside one prepared scene, as the day and night
graphs are prepared today, so a switch selects a prepared variant and no republication, pipeline rebuild or new
browser entry point is involved.

## Cues

| Cue | Where it must read | Range | Plausible wrong substitute, and what rejects it |
|---|---|---|---|
| **C1 Nothing visible disappears or pops** | The game camera in motion, Mac and Chrome | 1 to 100,513 instances; 0–100% in view; snap turns, cuts, resize, device replacement, dropped frames, slot churn; moving and mirrored instances; near-plane crossings | A test that passes at sampled frames or on scenes without ties. Rejected by byte equality with the oracle at every tick of a scripted path, plus tie witnesses |
| **C2 Occluders hide the work behind them** | Flying behind walls and props in the mixed-mesh field | One wall to full cover; stepping out of cover | Counters that fall while the forward cost does not. Rejected by paired per-pass GPU time on meshes with real vertex work |
| **C3 The mode is right for the device and the scene** | Mac both recipes, Chrome, the iOS simulator | Adapters with and without indirect execution; cheap cubes to meshes of a few thousand triangles | A culled mode that silently runs Direct, making every equality test vacuous. Rejected by asserting the resolved mode and a nonzero hidden count |

The bar is a shipped GPU-driven renderer, where culling is invisible: the player never sees it work. Here that is
held to something stricter and checkable, byte equality with drawing everything. The bar is about which geometry
each frame holds. How fast or how evenly frames arrive is [not judged in this module](#frame-cadence).

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
  [the experiment's record](../../../.orchestrate/evidence/render/G3C-OCCLUSION-EXPERIMENT-2026-11-07.md).
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
exact form, and GPU tests pin the shader's slack to the reference at its boundary.

The bound has two parts, and each needs its own witness:

- **Depth slack.** On cubes a zero-tolerance mutant still passes, so the witness that needs the slack is a mesh
  face lying on its own bounds, flush with a prepass occluder at a rounded far pose (WP1).
- **Footprint widening.** The depth witness says nothing about it. Its witness is a slot that shows only in a
  texel column the unwidened rectangle would leave out. The reference searches poses for one. If it finds one, a
  mutant without the widening must lose pixels there. If it finds none, the evidence records the poses searched,
  and the widening stays as derived and unfalsified on this device.

The rasterizer's four-ulp interpolation term is an assumption. No test here establishes it: the depth witness
passing with the full ε is consistent with it and does not prove it.

### Built: the bucket route and Sun-caster culling

A scene may give each logical slot a mesh from a fixed per-preparation arena (`SyntheticMeshes`). Each (mesh row,
pipeline class) bucket owns a segment of a member list, aligned to 256 slots so its draw binds the list at a
CPU-known dynamic offset with `first_instance` 0, as WebGPU requires. The class (ordinary, mirrored, two-sided)
comes from the upload's determinant. The compaction runs once over the concatenated segments and writes one
indirect record per bucket. Sun casters are culled per Sun pass against that pass's own raster volume, never
against the camera.

### WP1: Occlusion on the bucket route

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
  list; zero depth tolerance on the mesh tie witness; and no footprint widening on the footprint witness, where
  the search finds one.

**Check that it can work.** The equality argument above does not depend on the mesh, only on prepass parity and a
conservative test. Parity holds per bucket by sharing the pipeline state. The test is conservative for any mesh
inside its local bounds, which admission computes from the vertices. Where it pays is known in outline. The
earlier session's harness (its logs are not committed, so this is a prediction for WP2 to test, not evidence)
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

### WP2: the mode follows the device and the measurements

- **Capability fallback.** Culled modes draw with `draw_indexed_indirect` and compute. The requested mode resolves
  from the capability record, as [COOKED-DRAW](../COOKED-DRAW/DESIGN.md) does: a record without adapter-read
  indirect execution, or with unknown facts, resolves to Direct. No pass probes the device. The resolved mode and
  the reason are reported beside the requested one, and every equality test asserts the resolved mode, so a
  fallback can never pass as culling.
- **Default by measurement.** A scene's default is Frustum. It becomes Occlusion only when paired whole-GPU rows on
  that scene favour it in two separate runs, behind cover and in the open, at 1920×1080. Scenes fully in view may
  keep Direct. The rows and the reason are recorded either way.
- **The mode is fixed per prepared scene.** It is requested at launch (`THRUM_CULLING` natively, the page's
  culling argument in Chrome) and resolved once, at preparation. Device replacement prepares the same requested
  mode again and resolves it against the new generation's record. Direct stays requestable everywhere, as the
  oracle a creator can launch.

### WP3: the valley's entities

The valley keeps its content. Its 10,000 entities move from Direct to the default the rule above gives, on the cube
route, where Frustum and Occlusion are byte-equal to Direct, so culling changes no retained image. An opt-in mesh selection
for the same entities (as `THRUM_FIELD_MESHES` does for the field) runs the bucket route with its Sun-caster
culling against bucket-ordered Direct, which is the configuration `SHOWCASE` adopts.

**Under temporal sampling.** The prepass, both culls and the forward pass read one view record, so a jittered
projection moves them together and the proof is unchanged. Motion and reactive outputs come from the forward pass,
whose colour, motion and reactive arithmetic is untouched. WP3 proves equality with temporal sampling selected.

**Compute-readable materials.** The gate asks that compute can read the material table. The one existing material
buffer gains compute visibility; no second table is made.

### WP3: one image under load

Added 14 November 2026, after WP3's builder stopped on [the module list's](../ARCHITECTURE.md#exact-baselines)
rule 1. Its evidence is on branch `build/culling`, under `.orchestrate/evidence/CULLING/WP3/` (`pinned-probes-d70c1a27/`,
`triangle-merging/`, `sample-dependency/`, `sample-frequency/`, `qualification-a5ab2143/`), and reaches main
when that branch merges.

**The defect.** On Metal the valley drawn Direct is not one image. With other work on the GPU, two hosts that
draw the same tick Direct differ at 4 to 152 pixels, and they still do with instance culling, Sun-caster culling
and cooked-meshlet culling all absent. At the differing pixels the primitive, the depth and the UV agree and the
UV derivatives do not. The reported cause is triangle merging: the driver merges adjacent triangles, and a
fragment at a shared edge can then take its implicit derivatives from the neighbouring triangle's values. The
source is [Khronos Vulkan-Portability issue 54](https://github.com/KhronosGroup/Vulkan-Portability/issues/54),
where MoltenVK's maintainer, Bill Hollings, reports to Apple that Metal "is not disabling 'triangle merging'
optimizations within the driver, when texture sampling with implicit LOD". The thread holds no reply from Apple,
so this is an implementer's report and not a guarantee about the driver. Here, in four retained 2×2 quads, one
host's derivatives equal the differences across two primitives exactly, which supports that cause in those
occurrences. Which edges merge changes under load. So no oracle drawn by the same shader can be relied on, and
every exact comparison of the valley is exposed: this module's, and `APPLE-TILE`'s retained references, which
hold merged derivatives.

**The mechanism.** Every opaque forward fragment entry that takes an implicit derivative, through a
`textureSample` or through `dpdx` or `dpdy` of an interpolated value, writes an all-ones `sample_mask` output.
By the reports on that issue the driver does not merge triangles under a fragment shader that writes coverage, so
each fragment's derivatives come from its own triangle. The value covers every sample, so coverage, depth and the
shader's arithmetic are as they were; only derivatives at shared edges change, and with them the mip level and
the Sun receiver's slope there. There is one shader source for every backend and recipe: no Metal-only variant
and no selection. Volume entries, and passes whose fragment stage takes no derivative, stay as they are. This is
the workaround Steven Noonan reports on that issue from Darwinia, over ANGLE's Metal backend and over its Vulkan
backend on MoltenVK. It has to be a written output: he recalls a declared but unused one being optimised away.
Those reports are why this mechanism is chosen. That it works here is WP3.11's to prove.

**Why not another way.** Each of these was built and is retained on the branch. Explicit gradients for the
material samples moved 4,859 reference pixels and the hosts still differed. Sample-frequency interpolation
failed WP3.4's own check by one pixel. Making the Sun search depend on the samples failed under load. A depth output
on opaque fragments was stable over four loaded routes, and at the `portable-sdr-approach` checkpoint its image
is byte-equal to the coverage output's, 8,303 pixels from the retained reference. Two different outputs give
the same bytes there, which is what both stopping the merge would produce; it is an inference from one
checkpoint, not a proof that the image is the unmerged one. The depth output is not chosen,
because it moves the depth test after the shader for every opaque fragment. A selection that kept today's output
as the default is not chosen either: every scene that did not ask would draw an image that varies, and the exact
tests would prove a configuration nobody ships.

**What moves.** The default output of every scene with textured or shadowed opaque geometry, at pixels whose
2×2 quad spans more than one triangle. `APPLE-TILE`'s registered references therefore move, and
[the module list's ruling on WP3](../ARCHITECTURE.md#ruling-on-culling-m3) says who refreshes them. Once refreshed,
they guard the pipelines they are drawn with, which are Direct's: a build whose Direct pipelines lost the
output's effect draws the old bytes there and fails the verifier. They are never drawn Frustum, Occlusion or
with meshes selected, so they say nothing about those pipelines.

**Proof that it is one image.** A passing route proves little, because builds with no repair in them have
passed 8, 16 and 24 complete routes in a row. So the claim rests on a controlled experiment, and the experiment
has to be made on the build it speaks for. Direct and the culled modes prepare different pipelines: the vertex
entry differs, and the bucket route adds a pipeline for each class. A rebase can change one of them through a
pipeline descriptor, a dependency or the toolchain while the shader text and every image drawn Direct stay as
they were. Three proofs carry the claim. Each is one bounded run, and none waits on another.

1. **The campaign, once, on main.** After the merge, on the Mac, a build of a commit of main that holds the
   coverage output and a control that differs from it only by leaving the output out take turns, one complete
   route each, with the same recipe, source and entity route, four hosts (Direct, Frustum, Occlusion and a
   second Direct), two counted render workers and the full exact comparison at every tick. The turns go round
   the eight combinations of the two recipes, the two sources (SDR and scene-linear) and the two entity
   routes: cubes, as WP3.1 draws the valley, and selected meshes, as WP3.4 does. Those are the forward pipelines
   this milestone qualifies. The campaign ends when the control has differed in five routes and both builds
   have run every combination. The build with the output must have run at least as many routes as the control
   and differed in none. If the output did nothing, five differences would all fall to the control with a
   chance of 1 in 32; five is the planner's choice of how strong the proof is. A campaign whose control has
   not differed five times after sixteen pairs is stopped. It proves nothing, and is reported, not shortened.
   Sixteen is the planner's choice too: twice round the combinations, about an hour and a half at the 2.75
   minutes a route took on the branch. The builder takes such a campaign once more at another hour, and after
   a second ends `BLOCKED` with both records. The record names the commit both binaries were built from,
   their hashes, the control's source difference, the toolchain, the route, the cooked valley, the load and
   each route's result with its combination, so a reader sees where the control showed the load at work.
2. **The references, on main.** Rule 2's `verify --full` passed on the branch against the regenerated
   references, and the main-branch check `exact-images` draws them again on main. Where the output stops
   working, a build draws merged derivatives, the image the old references hold. At `portable-sdr-approach`
   alone the two images are 8,303 pixels apart, and on the branch all thirty native files and all four Chrome
   manifests moved. So a loss on Direct's pipelines fails that check on every run, at any load. That is
   WP3.10's check. It draws Direct only, so it does not stand in for the campaign.
3. **This module's comparisons, on the merged tip.** WP3.1 and WP3.4 compare separately prepared hosts at every
   tick of the route on both recipes, and WP3.4's check is where this module first met the variation. The
   script runs both on main's tip while other builders load the GPU. One passing run of each is weak evidence
   that the image is one, by this section's first sentence. They are there to catch a mode that differs from
   its oracle.

<a id="what-a-campaign-stands-for"></a>

**What the campaign stands for.** The commit it names, which is a commit of main that holds the landing and
every change this milestone has merged to what the game is built from: `crates/`, `examples/v1-game/`,
`.cargo/` and the workspace manifest, lockfile and toolchain file. So it is taken after the merge and never
before it, and the rebase inside `merge` cannot come between the experiment and the build. If the milestone
later merges another such change, a fix after a review for example, the campaign is taken once more after it.
It is not taken again because main moved, and no record relates it path by path to a later commit. What other
modules merge afterwards is theirs, and for it there is a standing stop, not a standing loaded proof:
[the ruling on a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) ends
`BLOCKED` any module whose exact comparison, the verifier's or its own, varies along triangle edges on
content that holds the output.
Equal shader sources are never offered as proof of anything.

A campaign passed on the branch at `cfecd0c7`: meshes selected, six pairs, the control differing in five
routes and the build with the output in none. It is why this mechanism lands. It ran before the rebase and
without the cube route, so it is not WP3.11's proof.

The campaign is a measurement and nothing else: one job that does not fetch, rebase, verify or merge, which
the builder starts itself once it has read `merge`'s result. It holds the `gpu` lock around whole pairs. A
campaign too long for the lock hold limit is taken as whole pairs in separate holds, and the record adds them
up; a hold that is stopped loses only its unfinished pair. If the build with the output differs once, the
correction does not hold on the merged build: keep both payloads and end `BLOCKED` with the record.

<a id="qualified-before-it-lands"></a>
<a id="how-it-lands"></a>

**How it lands.** The coverage output, the regenerated references and their pins reach main in one merge, as
[rule 2's planned refresh](../ARCHITECTURE.md#proofs-run-once-on-main). The branch is proved once and merged.
Nothing is proved again because main moved, and no record of the order of these steps is owed to anyone.

On the branch, once each:

- **References.** Each reference that moved is regenerated from the build that holds the output, with
  `APPLE-TILE`'s scripts, and committed. The references they replace must be what main drew: a passing
  verifier record at the **starting commit**, the commit of main the branch regenerated on top of, says so.
  The main-branch check's pass at that commit is such a record and costs no run.
- **WP3.12's difference images**, between the references replaced and the new ones, show only what that item
  allows. The builder judges them by the item's own list, and the reviewer judges them again at review.
- **Rule 2's one `verify --full`** passes on the branch's build against the regenerated references (proof 2).
- **Pins.** The [reference guard](../BASELINE-GUARD/DESIGN.md#the-reference-guard) in `./dev check` refuses a
  moved reference until the registry records it. A commit changes, in `.config/exact-baselines.json`, the
  `pin` and the `refresh` of each baseline whose reference moved, and nothing else.
  `python3 scripts/exact_baselines.py guard` prints each pin from the committed files. The refresh names the
  anchor `ruling-on-culling-m3`, the module `CULLING` and the milestone `WP3`. Only the guard reads those two
  fields, so this commit changes nothing that was drawn or compared.

Then the builder runs `merge`, by itself. It rebases, runs `./dev check` with the reference guard, and pushes.
The script's run on the merged tip is the proof of record, and WP3.10's check passes from the main-branch
check's run there. On main, once: the campaign (proof 1), on a build of
[the commit it stands for](#what-a-campaign-stands-for).

**What a rebase repeats.** `APPLE-TILE` WP10 and `ATMOSPHERE` WP9 also land covered changes, so the builder
integrates on top of what has landed.

- If the registered reference files are as they were, nothing is repeated: not the verifier, not WP3.12's
  images. The campaign follows the merge in any case.
- If only a pin changes, because main changed a comparison file, the builder copies the pins the guard prints
  and keeps its refresh.
- If a registered reference changes or conflicts, another landing moved an image. Main's new tip becomes the
  starting commit. The builder regenerates each moved reference from the rebased build, makes WP3.12's images
  again, and runs `verify --full` once on that build. Each of the two refreshes records its own old and new
  hashes, as [the module list](../ARCHITECTURE.md#ruling-on-culling-m3) orders.

**When it fails.** Before the merge: if a reference moved where WP3.12 allows none, nothing is merged, and
the builder ends `BLOCKED` with the payloads. After the merge: if the main-branch check fails at this module's
commit, rule 2's course applies. The builder fixes it as its next task, or the lead reverts the merge, which
restores the shader, the references and the pins together. If the build with the output differs in the
campaign, or a failure's every difference lies along shared triangle edges, the correction does not hold on
that target: the builder ends `BLOCKED` with the record, and a planner settles what main keeps, as
[the ruling on a frame that varies under load](../ARCHITECTURE.md#a-frame-that-varies-under-load) says.

<a id="no-landing-pipeline"></a>

**No landing pipeline.** The builder writes no job that waits for, asserts or follows another job, a merge or
a verifier run, and installs no Git hook, merge-queue helper or guard of its own around `merge`. Each proof
above is one command, started once with the script's `background` and collected with `wait`. The builder
reads the result and decides the next step itself. This milestone's first build enforced an order (qualify,
then push, with the campaign standing path by path for the commit pushed and the commit claimed) through a
chain of such jobs. A job audit of 16 November 2026 traced about 30% of all the project's failed background
jobs and five failed GPU holds to that chain: every follow-up failed when the job before it did, and each
queued for the lock to prove again what the run on main proves. The order and the rule it enforced are
withdrawn, not automated better.

**Cost.** A coverage write can cost fragment time: merged triangles shaded fewer helper fragments at shared
edges, and the driver may test depth later. Nobody has measured it here. WP3.13 records it under
[Performance](#performance), as paired rows of the claimed shader and the same control, taken once and not
again for a rebase. The figure is a
recorded cost. It is not a condition of landing the correction, and no result of it ends the milestone
([the brief](../VISION.md), decision 3;
[the owner's performance decision](../../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md)). If the rows
cross a tripwire, the builder investigates within WP3: which passes carry the cost, from the per-pass rows, and
what in the output causes it. The finding is recorded with a disposition, one of two: the cause is corrected in
WP3, or the cost is carried, with the reason and what the performance phase should try, as a proposal in the log
for a planner. Building continues either way. A cheaper form of the output moves the references again, so it
lands [as above](#how-it-lands), and the campaign is taken again on main after it. Correctness and the
milestone's quality bar remain requirements whatever the rows say: WP3.7 and WP3.8 judge the captures for popping,
flicker and missing geometry. They do not judge how fast or how evenly the frames arrive, and no item of this
module does ([Frame cadence](#frame-cadence)).

**Chrome.** The campaign runs on the Mac only. Chrome's image varies too, by
[the module list's reading](../ARCHITECTURE.md#a-frame-that-varies-under-load) of two Chrome records, which is
an inference from where the pixels lie. The same shader source reaches Chrome, where WP3.2, WP3.5 and the
main-branch check over the four Chrome sets stand as the proof, and WP3.10 records which sets moved.

## The system as it runs

| State | Owner |
|---|---|
| Visibility pipelines, the visible-last bits, the mode resolution | This module (`thrum-render::visibility`, `synthetic/camera_culling.rs`) |
| Bucket membership and the mesh arena | This module (`synthetic/{buckets,mesh_route,mesh_arena}.rs`), maintained by the synthetic upload |
| Per-slot visible lists, indirect records, the prepass depth and pyramid | The frame slot's target, declared in the graph |
| Graph semantics, the split graph, attachment storage | `APPLE-TILE` |
| The view record, jitter, motion and reactive targets | `APPLE-PRESENTATION` |
| Frame cadence of a culled scene: the frame records, the presented intervals and the verdict on smoothness | `SHOWCASE`, on the Mac present path `PLATFORM-RUNTIME-MAC` qualified, with `APPLE-PRESENTATION`'s governor ([Frame cadence](#frame-cadence)) |
| Cooked placements and meshlet culling | `CONTENT-LOOP` (`thrum-asset-gpu`) |
| Scene content, routes and the controls | The game |

**An Occlusion frame on the bucket route**, 100,000 slots, with the costs measured on cubes on a shared GPU:
clear the next bits (12.5 KB); prepass list (0.02–0.14 ms); depth prepass per bucket (0.02–0.34 ms, more with
vertex work); pyramid (0.08–0.09 ms at 720p, one dispatch per level); final cull (0.03–0.10 ms); forward over the
final list; then history, exposure and display as before. Extra memory per render target is the prepass depth, the
pyramid and a second list: about 13.8 MB per host at 720p and 27.8 MB at 1080p, logical sizes.

**Transitions.** A camera cut or a slot swap leaves stale bits, which cost one frame of prepass efficiency. Resize
reallocates the pyramid and keeps the bits. Device replacement rebuilds everything and clears the bits. A dropped
submission leaves the bits as of the last executed final cull. A capture does not advance them. The mode changes
only when a scene is prepared.

## Exact baselines

[The module list](../ARCHITECTURE.md#exact-baselines) says how a change that alters another module's exact images
is caught. For this module:

- It must not move `APPLE-TILE`'s registered baselines, on the Mac or in Chrome, with one exception that the
  module list rules: WP3's [coverage output](#m3-one-image-under-load). The main-branch check `exact-images`
  runs the verifier on main after each merge, and WP2 and WP3 carry `verify` as their claim-time check, which
  passes from that run. No image run is asked before a merge, except WP3's one run for its planned refresh.
- WP1 was built before the verifier existed and carried the native matrix as an automated check.
- Its own exact comparisons are relative (each mode against its oracle in the same run), so they hold no retained
  bytes. Another module's builder must not loosen them.
- WP3 changes the valley's default mode. That is allowed only because the culled image is byte-equal to Direct,
  which WP3.1 and WP3.2 prove.
- WP3's coverage output is the one change of this module that moves retained images. Under the ruling, its
  builder regenerates each reference that moved, with `APPLE-TILE`'s own scripts, in the landing that adds the
  output, which lands [as rule 2's planned refresh](#how-it-lands). In the registry,
  which is `BASELINE-GUARD`'s, the landing changes the `pin` and the `refresh` of each baseline whose reference
  moved. Every other field of the registry, the comparisons, their sampling and every script stay as they are,
  and the references stay `APPLE-TILE`'s. WP3.10 holds the refresh and WP3.12 has a reviewer see what moved. No
  other change of WP3 may move a reference.

## Risks

1. **False culling at a silhouette or tie on real meshes.** Early sign: the mesh tie witness or the flythrough
   differs from the oracle. Fallback: raster-proxy occlusion; until then Occlusion stays off by default.
2. **Occlusion costs more than it saves.** Apple's hidden-surface removal already discards hidden fragments, so
   only vertex and binning work is saved. Early sign: WP2's paired rows. Outcome: Frustum stays the default.
3. **A silent fallback hides a failure.** Mitigation: resolved-mode and hidden-count assertions in every witness.
4. **Shared files move under the builder.** `scene_graph.rs` and `synthetic.rs` are also edited by `APPLE-TILE` WP3,
   `APPLE-PRESENTATION` WP2 and `EFFECT-GRAPH`. Integrate sequentially. The script's run on the merged tip is the
   proof after a rebase, and the builder reruns only the check it says failed.
6. **The interpolation allowance is wrong on some rasterizer.** It is assumed, not measured. Early sign: the mesh
   tie witness loses pixels with the full ε. Fallback: as risk 1.
5. **wgpu's indirect-call validation adds untimed GPU work.** It stays on in shipped builds and is bounded once
   with a bench-only comparison.
7. **The coverage output is not the whole cause, or stops working.** The evidence for it is bounded: four quads
   and eight loaded routes. A later compiler, dependency or pipeline change could also fold the output away.
   Early sign: the build with the output differs in WP3.11's campaign on main, or the main-branch check draws
   the old bytes, which shows a loss on Direct's pipelines only. Outcome: the builder ends `BLOCKED` with the
   payloads, or [rule 2's course](#how-it-lands) applies, with a revert that restores the references. After
   WP3, a loss confined to a culled or bucket pipeline has no standing loaded proof: it shows as an exact
   comparison that varies under load, which stops the module that meets it.
8. **The coverage output costs more than expected.** Early sign: WP3.13's paired rows cross a tripwire. Outcome:
   the cause is investigated and given a disposition within WP3, as the Cost paragraph of
   [the mechanism](#m3-one-image-under-load) says. The figure alone never stops the milestone.

## Performance

Costs are recorded, never gated ([performance rules](../../../.agents/skills/thrum-engineering/references/performance.md)).
Paired, interleaved rows through the frame host's pass-name timing; p50 and p95; two separate runs; the load
recorded and every row labelled contended, because the GPU is shared. Tripwires: about 25% paired regression
against the previous checkpoint at the same configuration; per-instance cost that stops being flat from 10,000
slots to the prepare limit; per-bucket cost that grows from 1 to 64 buckets; allocation growth per encode. A
crossed tripwire starts an investigation, whose cause and disposition are recorded with the rows. It is not a
pass mark, and no item fails on the figure. No quiet-GPU figure exists, and none is claimed.

The coverage output's cost is recorded the same way on the valley's moving route: the claimed shader paired with
a control that leaves the output out, per pass and for the whole frame, on both native recipes.

## Frame cadence

This module is not judged on frame cadence. No item of it is about how fast or how evenly frames arrive, and none
needs an awake panel, a capture-off run or a sitting with the other builders paused. Settled on 17 November 2026,
with [the module list](../ARCHITECTURE.md#who-judges-frame-cadence).

- **What WP3.7 and WP3.8 judge.** Which geometry each submitted frame holds: nothing visible pops in or out,
  flickers or is missing at a frame edge or at an occluder's silhouette. They use a `realtime` capture because the
  game then draws at uneven intervals, with larger camera steps between frames and the occasional dropped
  submission, which the fixed-tick routes of WP3.1 to WP3.5 never do. The capture is read as a sequence of images.
  Its submission intervals are not a measurement of cadence: it is taken while other builders work, with the
  capture observer on, and Chrome's recorder runs headless on a 60 Hz timer. A long interval between two frames
  is a larger camera step, a harder case for the item and no failure of it.
- **Who judges cadence.** `SHOWCASE`, on the exit scene, which is this valley populated and drawn in the default
  mode this module's rule sets. Its cue S5 and its items for the capture-off whole-frame record with each late
  frame's cause, the OS-reported presented intervals on the built-in panel, and the frame callbacks of a visible
  Chrome window (M7.9, M8.6 and M8.7 when this was written) carry "smooth at the display's full refresh".
  `PLATFORM-RUNTIME-MAC` qualified the Mac's present path on the cooked valley, and `APPLE-PRESENTATION`'s
  governor is what holds the rate when a frame costs too much.
- **Why not here.** Gate G3c asks for visibility, image equivalence and costs "as tripwires", and says nothing of
  pacing. Cadence belongs to the whole frame, on a display, with capture off and every module's passes in it. A
  figure for culling alone would need the attended panel and the paused builders this plan declines, and
  `SHOWCASE` would take it again on the scene that ships. What culling adds to a frame is in WP3.3's and WP3.13's
  paired rows, per pass, at p50 and p95.
- **What a reviewer does with an uneven capture.** Nothing against WP3.7 or WP3.8, and the target's full-refresh
  edge condition is not run for them. A stall the reviewer traces to this module's own code, such as a culling
  pass's time in the frame's rows or a resource it creates mid-run, is a defect to report like any other. An
  interval in a loaded capture with no traced cause is an observation.
- **How a cadence problem comes back.** When `SHOWCASE`'s record names a culling pass as the cause of a late
  frame, it writes a proposal in this module's log and a planner settles it, as its design already says of entity
  level of detail.

## Proof and builder handoff

- **Targets.** `mac-metal` for every milestone, on both recipes where a recipe is named. `chrome-webgpu` through
  `scripts/check_v1_browser.mjs`, as the existing field cases run. `ios-sim` for function only: the simulator has
  no indirect execution, so it proves the fallback, never a culled mode.
- **Capture.** Equality is proved in-process by reading back each mode's target in one run. WP3.7 and WP3.8 use
  T2 engine capture and name its modes: `fixed` for stills, `realtime` for the pan, which is read as a sequence of
  submitted frames and [never as a timing](#frame-cadence).
- **GPU.** Each GPU check holds one lock: `gpu` on the Mac, `chrome` for a run that starts Chrome. Scratch goes in `tmp/culling-<milestone>/`; selected evidence in
  `.orchestrate/evidence/CULLING/`.
- **Once, then `merge`.** Each proof is one run on the branch; the script's run on the merged tip is the proof
  of record, and [no job of the builder's](#no-landing-pipeline) chains proofs, guards the merge or follows it.
  WP3.11's campaign is the one proof the builder takes on main, [after the merge](#what-a-campaign-stands-for).
- **Maturity.** The `G3c_gpu_culling` row of [the ledger](../../design/maturity.json) is updated at each milestone,
  with implementation, correctness, visuals and device results kept apart.

## Open questions

| Question | Settled by |
|---|---|
| Does the derived ε hold on a mesh face lying on its bounds? | WP1's tie witness and its zero-tolerance mutant |
| Does any pose need the footprint widening? | WP1's reference search, and the no-widening mutant if it finds one |
| Does Occlusion win on realistic meshes at 1080p on this Mac? | WP2's paired rows |
| Which of the five registered reference sets does the coverage output move? | WP3.10's record |
| What does the coverage output cost on the valley? | WP3.13's paired rows, and its investigation if a tripwire is crossed |
