# Cooked draw path: capability-selected direct fallback

9 November 2026. **Planned; nothing implemented.** This is a child design of
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md), owned by G5 (`thrum-asset-gpu`). It sits under
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2), [C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1),
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4),
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) and
[the hardware-native decision](../decisions/HARDWARE-NATIVE-ENGINE.md). Brief: [COOKED-DRAW-BRIEF.md](COOKED-DRAW-BRIEF.md).
Plan: [COOKED-DRAW-PLAN.md](COOKED-DRAW-PLAN.md). Done when: [COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json).

## Purpose and scope

The cooked scene must draw on every adapter Thrum runs on. Today it requires indirect execution. The iOS simulator's
Metal adapter lacks it, so the simulator draws no cooked content, and T2-CAPTURE M2 is blocked
([evidence](../../evidence/render/t2-capture-m2/README.md)).

The consumers are the V1 game on all three targets, T2-CAPTURE M2, which needs the cooked valley on the simulator,
and any later downlevel device.

**Commissioned:**
- a direct draw path for cooked meshes;
- selection from the capability record;
- the capability fact that selection reads;
- the iOS route attaching the cooked scene through the game's existing selection;
- proof on mac-metal, chrome-webgpu and ios-sim.

**Not commissioned:**
- changing which path Mac or Chrome selects;
- CPU or compaction culling for the fallback;
- the G3c synthetic renderer's own indirect paths (see [Adjacent gap](#adjacent-gap));
- physical-device or speed claims.

## What must hold

- **Same scene, both paths.** Where both paths can run (this Mac), the direct path's final image is byte-identical
  to the indirect path's for the same build, store and fixed-time view.
- **The indirect path is untouched.** Its output on Mac Metal and Chrome WebGPU is byte-identical before and after
  the change, and it records the same per-frame commands.
- **The simulator draws the cooked valley.** It is complete, textured, lit and shadowed. It survives
  background/foreground, device replacement and rotation, and it shows no discontinuity in the sampled frames of a
  camera pan. This is function only. Complete submitted-frame temporal proof belongs to T2-CAPTURE M2.
- **The record decides.** The path comes from adapter facts in the capability record. No platform, OS, backend or
  adapter-name test is allowed, and no private device query. Unknown facts never enable indirect execution.

Bar: where both paths run, they produce identical pixels. On the simulator, the cooked valley is as complete as on
Mac. Shipped engines' compatibility renderers work this way: Unreal's ES3.1 mobile path and Unity's GLES fallback
draw the same content with less GPU-driven work.

## Mechanism

**Selection.** At capture, `DeviceCapabilities` records the adapter's downlevel flags next to its features and
limits. The flags are known only when they come from an adapter. `SceneSurface` lends that fact the same way it lends
`features` today. At every `rebuild`, `CookedAssets` picks its draw path from the surface:
- **indirect** only when the record holds adapter-read downlevel flags that include `INDIRECT_EXECUTION`;
- **direct** otherwise, including when the flags are unknown.

`rebuild` already runs for every device generation, so a replacement device re-selects. Texture-profile selection
already works this way (G5's "Profile selection" row).

**Adapter-less records are conservative.** `DeviceCapabilities::capture_device` has no adapter to read, and wgpu 30
exposes downlevel flags only on `Adapter`. So its record says the downlevel facts are unknown. It never assumes them
and never invents support. The public route `FrameHost::new` → `capture_device` → `attach_cooked_scene` therefore
selects the direct path on every device. The direct path uses only core indexed drawing, so it is valid on any
adapter, and where both paths run it gives the same pixels (M1.3). An adapter-less host on a capable device pays the
direct path's extra vertex work. Its stats and evidence name the path and the unknown-facts reason.

Refusing to attach cooked content to an adapter-less host was the alternative. It was rejected: it gains nothing over
an always-valid path, and it would break the existing tests and benches that use `FrameHost::new`.

Code that needs the indirect path has to build its record with the adapter-backed `capture`. That covers the Mac
app, Chrome, the iOS route, the indirect arm of the cooked GPU suite, and the `cooked_edit` tripwire bench. The bench
and the cull-count tests use `FrameHost::new` today, so they move to `new_with_capabilities` with `capture`. Otherwise
they would silently measure or test the direct path.

A diagnostic mask can clear `INDIRECT_EXECUTION` in a captured record. It must be opted into explicitly, it is never
on by default, and evidence records it whenever it is used. The mask lets this Mac exercise the direct path through
the same selection code. The mask acts on the record, never inside `thrum-asset-gpu`.

**Indirect path (unchanged).** It keeps G5's meshlet path:
- a compute pass culls each meshlet's bounding sphere against the frustum and its normal cone against the camera;
- the pass writes `DrawIndexedIndirect` arguments;
- the forward pass calls `multi_draw_indexed_indirect`;
- a visible count is read back without blocking.

**Direct path.** It runs no meshlet cull. For each placed mesh it issues one `draw_indexed` over the mesh's whole
index range, with base vertex 0 and one instance. It uses the same pipeline, vertex buffers, instance record,
material bind group and index buffer as the indirect path. It creates no `INDIRECT`-usage buffer and no cull
uniform. It records no visible counter and no readback. `computes()` returns false, so the declared
`contribution compute` pass is never begun. The seam already allows this.

The draw needs only core indexed drawing. It uses no `BASE_VERTEX`, no `INDIRECT_FIRST_INSTANCE` and no compute.
The simulator reports neither `BASE_VERTEX` nor `INDIRECT_EXECUTION`.

**Why the images match.** The loader writes each meshlet's indices contiguously, in meshlet order. So one draw over
the whole range rasterizes the same primitives in the same order as the per-meshlet draws.

Culling is conservative:
- a frustum-rejected meshlet's bounding sphere is outside a clip plane, so it yields no fragments;
- a cone-rejected meshlet is back-facing, and the mesh pipeline culls back faces;
- meshoptimizer's cone test is conservative.

So the same fragments are written, in the same order, under depth `Greater`. The direct path costs extra vertex
work, but the fragment work and the pixels are the same.

**Path-independent state.** Both paths share these in one place:
- the per-placement motion-history flag (C07), which today is written inside the cull step;
- the tentative/submitted protocol;
- residency and staging;
- material reload;
- Sun casters (already a full-range direct draw);
- the volume (already a direct draw).

`CookedStats` gains the selected draw path. `visible_meshlets` stays `None` on the direct path, because no cull ran.

**iOS route.** The simulator route attaches cooked content through the game's existing `attach_cooked_scene`
selection (`THRUM_ASSET_STORE`, `THRUM_SCENE`). With those unset, the route is unchanged. The route reports these in
the state file it already writes:
- the selected path;
- the drawn generation;
- the cooked submitted-frame count;
- the cooked errors.

**Simulator coverage.** The simulator command reuses the route's existing rows, with cooked content attached:
- background/foreground;
- device-generation replacement;
- portrait → landscape → portrait rotation, which is the `rotation` row in `examples/v1-game-ios/ios/route.py`,
  driven by its XCUITest orientation gesture.

After each row, the route must present at the current extent with the cooked set still drawn through the direct
path. The locked-console engine-state rows do not depend on the XCUITest gesture. If the gesture needs an unlocked
console, only the rotation row is recorded with that reason.

**What the simulator pan can show.** Before T2-CAPTURE M2, simulator frames come only from `simctl` screenshots or a
`simctl` recording. Both are sampled, and neither is tied to submitted frames. The evidence records the following:
- the route's presented-frame count over the pan;
- the samples taken, with their times;
- the holds;
- the largest gap between samples.

The M1 conclusion covers only the sampled frames. It can show that content is complete in every sample, that the
frame edges fill as terrain enters, and that no sample is missing geometry. It cannot rule out a defect confined to
unsampled frames. Proof over every submitted frame, with exact frame and time identity, stays with T2-CAPTURE M2. The
direct path has no delayed visibility, so no mechanism in it predicts one-frame holes.

## Alternatives rejected

| Option | Why not |
|---|---|
| CPU meshlet cull with coalesced direct draws over runs of kept meshlets | It saves vertex work for culled meshlets. But it adds O(meshlets) render-thread work every frame (G5's account keeps the render thread near zero), and a second cull implementation that must match the GPU one. It also needs CPU-side bounds for remote feeds, and one draw per run. The saving matters only on real downlevel GPUs, and none is a V1 target. Promote it only if such a target appears and measurement shows the direct path is vertex-bound. |
| GPU cull, compacted into a per-frame index buffer, then a direct draw of the maximum count | Without indirect execution the count stays CPU-known, so culled meshlets become degenerate triangles that still cost vertex work. It writes about 0.7 MB of indices per frame for the showcase terrain, and needs an index copy per frame in flight. It also adds a compute-to-index hazard. More cost and complexity, and no clear saving. |
| GPU cull with the visibility read back to the CPU and drawn a frame later | Geometry entering the view would be missing for a frame, which shows as popping. It also adds map synchronization. This is incorrect, not merely slower. |
| One direct draw per meshlet | Hundreds of draws per mesh and no culling benefit. Strictly worse than one draw. |
| Making the simulator report indirect support (patching wgpu or overriding by platform) | It breaks capability selection. Apple's DTS describes the simulator device as [Apple-family-2-like](https://developer.apple.com/forums/thread/776140), and Apple's [feature tables](https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf) put indirect arguments at Apple3. Real downlevel devices would still lack a fallback. |

## Cost account

All counts are per frame. P is the number of placed meshes, M the number of meshlets per mesh, and I the number of
indices per mesh. The showcase valley has three meshes: terrain (58,880 triangles, 176,640 indices, about 480 or more
meshlets at 124 triangles each), sphere (2,208 triangles) and rock (224 triangles).

| Work | Indirect (unchanged) | Direct |
|---|---|---|
| Render-thread writes | One 160-byte cull uniform per mesh; history flag on change | History flag on change only |
| Compute | One cull dispatch per mesh, M threads; visible-count copy and clear | None |
| Encoded draws | One multi-draw per mesh. wgpu-hal 30 Metal expands it into M indirect draw commands; the WebGPU backend also loops | One `draw_indexed` per mesh |
| Vertex work | Indices of kept meshlets | All I indices, about 61k triangles for the valley. At most one extra Sun-cascade caster pass, which already draws full ranges per cascade |
| Fragment work | Visible fragments | The same fragments |
| Memory | 20 B × M arguments, 160 B uniform, 4 B counter and readback, per placement | None of these |

**Tripwires and measurements:**
- **Indirect path.** It is unchanged by construction. Byte-identical output and a reviewed command diff prove it,
  and the existing G5 tripwires (`scripts/check_g5_tripwires.sh`) watch it.
- **Direct path on this Mac.** M1 records paired, order-alternated measurements against the indirect path: GPU
  forward and contribution time, and render-thread encode time. Each is labelled with its load class. No speed claim
  follows for either path.
- **Simulator.** Its timing is never recorded as speed.

Neither path has a cost target. Only the profile tripwires apply: visible hitches, about 25% comparable
regression and unexpected scaling.

**Observation for the owner.** On Metal, the selected indirect path issues one indirect draw command per meshlet,
about 480 for the valley terrain. Nobody has measured whether this beats one direct draw on Apple GPUs. M1's paired
figures inform that question, but changing the selection needs a measured win and a separate decision.

## Contracts, interfaces and agreements

This work changes no contract semantics. The `SceneContribution` seam keeps its declared passes, recording versus
publication rules and single lighting authority. Contribution graph tokens are imported and virtual: `graph.rs`
allocates no buffers, so the declared `IndirectRead` token creates nothing on a downlevel device. C07 motion history
and C06 lifetime keep their meaning on both paths.

| Change | Producer | Consumers | Agreement needed |
|---|---|---|---|
| `DeviceCapabilities` records adapter downlevel flags. `capture_device` records them as unknown, and unknown never selects indirect. | `thrum-render` `capabilities.rs` (V1 graphics execution; implements F2's C04 registry) | Call sites need no signature change: `thrum-app` device creation, web recovery, sky capture, `v1-game-ios` render, `asset-loop`, benches. Readers: `thrum-asset-gpu` now, and later G3c culling | Render/registry owner (Thrum Core) |
| `SceneSurface` lends the downlevel fact | `thrum-render` contribution seam (V1) | `thrum-asset-gpu` | Render owner |
| C04 wording, proposed: "features, limits and downlevel execution facts" in the registry list | C04 owner (F2) | All registry readers | C04 owner; the text only clarifies the existing rule |
| `CookedStats` gains the draw path. `visible_meshlets` is documented as `None` on the direct path | `thrum-asset-gpu` | V1 cooked tests, the [G6 creator note](../decisions/G6-CREATOR-EXPERIENCE.md), T2 evidence | G5 (this module) |
| The iOS route attaches cooked content via `attach_cooked_scene` | V1 game, iOS route (`examples/v1-game-ios`) | T2-CAPTURE M2 | V1 game and G3b iOS shell owners |

**Overlap with G4 seam 3.** [APPLE-PRESENTATION-PLAN](APPLE-PRESENTATION-PLAN.md) seam 3 adds a reactive forward
target and a mip bias to the pipelines in `draw.rs` and `material.rs`. Both cooked paths share one mesh pipeline, so
G4 changes it once. G4's "invalid-history objects" reactive write reads the history flag, which this design keeps
identical on both paths. Neither work blocks the other. The only overlap is the shared file, which needs a merge.
G4 M2 has not started.

**T2-CAPTURE dependency.** T2-CAPTURE M2 `needs` `COOKED-DRAW:M1`. M1 delivers the iOS route with the cooked valley
on the direct path, through background/foreground, device replacement and rotation. M2's capture items then run on
that route unchanged. M2 also adds the submitted-frame temporal proof that M1's sampled pan cannot give.

## Risks

- **The simulator reveals another unsupported feature.** Its feature set resembles Apple family 2. Early sign: a
  different validation error once indirect is gone. The response: run the simulator route first, before polishing
  (see the plan). A feature this module owns is handled here. Anything owned elsewhere is reported `BLOCKED` to its
  owner.
- **Parity fails because the direct path draws visible geometry the indirect cull rejected.** That would be an
  existing cull defect in the indirect path. Record it and report `BLOCKED` for a plan decision. Never alter the
  indirect path silently, and never loosen the comparison.
- **Plausible wrong substitute: a simulator-only switch.** For example, `cfg(target_os = "ios")` or an adapter-name
  match. It would pass the simulator rows. The selection test and review questions reject it, and so does the
  masked-record arm on Mac, which must pick the direct path with no platform change.
- **Plausible wrong substitute: assumed capability.** A record that claims indirect support it never read from an
  adapter would pass every adapter-backed row, then fail on the first adapter-less host on a downlevel device. M1.1's
  adapter-less test rejects it: that route must select the direct path and create no indirect-usage buffer.
- **A test or bench silently changes path.** A cull-count test or the `cooked_edit` bench still built through
  `FrameHost::new` would quietly measure or test the direct path. M1.2 and M1.6 require each arm to record its
  selected path.
- **Plausible wrong substitute: a direct path missing state.** A direct path that draws but skips the history flag,
  discard handling or remote feeds could also look right in stills. Running the whole cooked GPU suite under the
  masked record rejects it.

## Checked in code and assumed

**Checked in code:**
- The draw-argument buffer and `multi_draw_indexed_indirect` call (`draw.rs`).
- The cull shader writes `first_instance` and `base_vertex` as 0 (`cull.wgsl`).
- Meshlet index ranges are contiguous and ordered (`load.rs`, `MeshletTables`).
- The mesh pipeline culls back faces (`material.rs`).
- Sun casters already draw full ranges directly.
- `computes()` false skips the compute pass, while `encode_after_compute` still runs (`synthetic.rs`).
- Contribution tokens are imported, and `graph.rs` creates no buffers.
- `DeviceCapabilities` records no downlevel flags today.
- `FrameHost::new` (`examples/v1-game/src/host.rs`) is public and calls `capture_device`. Its callers include the
  cooked GPU tests and the `cooked_edit` bench, and `attach_cooked_scene` accepts any host.
- The iOS route's `rotation` row uses an XCUITest orientation gesture through `xcodebuild`.
- The iOS route uses `capture` and does not attach cooked content.
- wgpu 30: `Adapter::get_downlevel_capabilities` is the only downlevel query. The WebGPU backend reports all flags.
  Metal sets `INDIRECT_EXECUTION` from `iOS_GPUFamily3_v1`, `macOS_GPUFamily1_v1` or `tvOS_GPUFamily2_v1`, and
  expands multi-draw into one indirect draw per entry.
- The simulator's recorded flags lack `INDIRECT_EXECUTION` and `BASE_VERTEX`, and include `COMPUTE_SHADERS` and
  `VERTEX_STORAGE`.

**Assumed, and settled by M1:**
- The M3 Max record reports `INDIRECT_EXECUTION`.
- Forced-direct output is byte-identical to indirect output.
- The simulator draws the rest of the cooked pipelines (ASTC or portable textures, 3D volume, casters) without
  further gaps.
- `simctl` screenshots of a headless simulator work while the console is locked, with no OS grant.
- The XCUITest orientation gesture works on a locked console. If it doesn't, only the rotation row is attended.

## Adjacent gap

This gap is not in scope, and the lead should route it to G3c. The synthetic renderer's camera culling modes
(`Frustum` and `Occlusion`), its culled shadow casters and its bucket route use `draw_indexed_indirect` with no
capability selection (`camera_culling.rs`, `shadow.rs`, `mesh_route.rs`). The default `CullMode::Direct` keeps the
iOS route working today, but selecting a culled mode on a downlevel adapter would fail the same way. G3c should select
from the same record fact.

## Open questions

- Should Chrome also run a forced-direct arm, to prove the direct path under WebGPU's validation? It is not
  commissioned, because Chrome always selects indirect.
- Should the Mac selection change after M1's paired figures? That needs a separate decision and a measured win. This
  module does not change it.
