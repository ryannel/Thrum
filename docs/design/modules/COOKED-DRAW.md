# Cooked draw path: capability-selected direct fallback

9 November 2026. **M1 claimed; all twelve items pass, independent lead review owed.**
The revised Mac oracle, Mac/Chrome BASE preservation, locked simulator function/lifecycle/rotation and sampled
cooked visual proof are recorded in the [plan](COOKED-DRAW-PLAN.md) and
[evidence](../../evidence/render/cooked-draw/README.md). PLATFORM-RUNTIME-IOS M7 supplies oriented scene sizing.
This is a child design of
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md), owned by G5 (`thrum-asset-gpu`). It sits under
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2), [C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1),
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4),
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) and
[the hardware-native decision](../decisions/HARDWARE-NATIVE-ENGINE.md). Brief: [COOKED-DRAW-BRIEF.md](COOKED-DRAW-BRIEF.md).
Plan: [COOKED-DRAW-PLAN.md](COOKED-DRAW-PLAN.md). Done when: [COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json).

## Purpose and scope

The cooked scene must draw on every adapter Thrum runs on. The commissioned gap was its requirement for indirect
execution: the iOS simulator's Metal adapter lacks it, blocking T2-CAPTURE M2
([original evidence](../../evidence/render/t2-capture-m2/README.md)). The local builder checkpoint now draws the
cooked valley on the simulator, but M1 remains unaccepted and T2-CAPTURE M2 stays gated on it.

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

- **Same scene, both paths.** On this Mac, the production direct path's final submitted image is byte-identical
  to a diagnostic full-range indirect reference with the same draw partition, build, store and fixed-time view.
  The selected meshlet-indirect path also renders the same complete, correctly placed, textured, lit and shadowed
  content; cross-partition shading differences are recorded and inspected under M1.3, not required to be zero.
- **The indirect path is untouched.** Its output on Mac Metal and Chrome WebGPU is byte-identical before and after
  the change, and it records the same per-frame commands.
- **The simulator draws the cooked valley.** It is complete, textured, lit and shadowed. It survives
  background/foreground, device replacement and rotation, and it shows no discontinuity in the sampled frames of a
  camera pan. This is function only. Complete submitted-frame temporal proof belongs to T2-CAPTURE M2.
- **The record decides.** The path comes from adapter facts in the capability record. No platform, OS, backend or
  adapter-name test is allowed, and no private device query. Unknown facts never enable indirect execution.

Bar: both selected paths render the same correct scene; the direct path has exact parity against a reference
with matching draw partitions (M1.3). On the simulator, the cooked valley is as complete as on Mac. Shipped engines'
compatibility renderers work this way: Unreal's ES3.1 mobile path and Unity's GLES fallback draw the same content with
less GPU-driven work.

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
adapter, and its whole-range draw is qualified by M1.3's reference and scene checks. An adapter-less host on a capable
device pays the direct path's extra vertex work. Its stats and evidence name the path and the unknown-facts reason.

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

**Scene equivalence and draw partitioning.** The loader writes meshlet indices contiguously, in meshlet order.
A whole-range draw traverses the same primitives in the same order as the per-meshlet draws before culling.
Culling must remain conservative: a frustum-rejected sphere is outside a clip plane; a cone-rejected meshlet is
back-facing under the pipeline's back-face cull; meshoptimizer's cone test is conservative. Depth remains `Greater`.
These are geometry and visibility arguments, not a guarantee of identical shading across draw boundaries.

At checkpoint `9c230ff4`, Apple M3 Max / Metal produced 1,506 differing final pixels (maximum channel delta 18)
along terrain triangle edges. Keeping every meshlet left the selected indirect image unchanged, while one full-range
indirect command per mesh matched direct exactly. Same-path repeats were exact. The
retained controls (the builder's checkpoint `9c230ff4`, `docs/evidence/render/cooked-draw/README.md`, integrated with M1) establish partition dependence for that view,
not a culling defect or the low-level cause. Pixel-quad derivatives and draw-boundary ordering remain hypotheses.

Research checked 9 November 2026: [WGSL §§15.6.2 and 15.7.4.1](https://www.w3.org/TR/WGSL/#derivatives) describe
quad-based approximate derivatives and give derivative functions no finite error bound;
[WebGPU §17.2.1](https://www.w3.org/TR/webgpu/#dom-gpurendercommandsmixin-drawindexedindirect) defines indexed
indirect drawing from buffer-supplied arguments. These support matching draw partitions for an execution-path
oracle; they do not establish this Metal discrepancy's cause. Equal primitive order alone is insufficient evidence
for byte equality across different partitions.

**M1.3 parity oracle (selected).** Keep the whole-range production direct strategy and every existing required view:
valley noon pan at 0°, 18° and 36°, a low-sun valley view, and the diagnostic scene with its volume, mirrored node
and edge-straddling mesh. On one build and cooked store, capture final submitted output through the T2 Mac tap:

- **D — production direct:** select it only through the opt-in indirect-masked capability record. Exercise the actual
  production direct encoder, with one whole-range `draw_indexed` per placed mesh, unchanged shaders and bindings.
- **R — diagnostic full-range indirect reference:** an explicitly enabled, separately labelled probe mode on the
  capable Mac emits one whole-range indexed indirect command per placed mesh, with no visibility rejection. Use the
  same pipelines, shaders, buffers, bindings, placement order and state as D. Build reference arguments independently
  from the cooked mesh ranges and placement records, never by copying D's encoded arguments or image. Record each
  placement's index count/range, base vertex 0 and single instance, and validate coverage against the cooked product.
  R uses indirect commands only as a test oracle; it is neither the selected production indirect path nor a new recipe.
- **I — selected production indirect:** unmasked capability record, ordinary meshlet cull and draws, with no override.
  Retain its image and actual selected-path witness for every view alongside D and R.

Require **D = R byte for byte for every view**, with byte-identical same-arm reruns of D, R and I. Use identical
extent, fixed world/time, camera, lighting, warmup and capture conditions. Record source/binary and store hashes,
adapter facts, capability mask, selected path, diagnostic mode and submitted-frame identity. Equality is a measured
same-device qualification, not a cross-device or cross-driver determinism promise. A D/R mismatch or unstable repeat
fails closed and must be investigated; do not introduce tolerances or fall back to visual-only acceptance.

Prove the oracle detects a real direct-path defect: in an explicitly enabled negative-control run, omit a visibly
covered placed mesh from D only, leaving R and its arguments unchanged. The exact comparison must fail with differing
submitted bytes. Restore D and require exact equality again. Retain the defect, command witness and comparison result;
a harness that compares one arm to itself or derives both arms from faulty D commands must not pass.

For I versus D, retain full images, unthresholded difference maps, differing-pixel counts and maximum channel deltas
for every view. Inspect whole frames and native-resolution crops of terrain triangle edges, silhouettes, entering
geometry, the mirrored and edge-straddling meshes, material/texture detail, Sun shadows and volume. Require the same
complete geometry, placement, materials, lighting and shadows, without visible cracks, holes or objectionable shading
seams. Cross-partition residuals need a recorded assessment and partition-control evidence (retain-all meshlets and
full-range indirect) where attribution is uncertain. Edge location alone does not exonerate a defect. Neither 1,506
pixels nor delta 18 is a tolerance, and no mask may hide edge pixels. A geometry/culling discrepancy, visible quality
failure or unclassified scene discrepancy blocks the claim. No byte-equality gate is imposed between I and D, or
between different meshlet-visibility partitions; diagnostic controls are evidence, not new production behaviour.

The reference and negative controls must be opt-in probe facilities, absent from ordinary selection and per-frame
work. Do not patch production WGSL between arms or leave an override enabled for M1.4–M1.6. Review the probe's command
witness and independence from D. The production direct path still allocates no indirect/cull/readback resources.
M1.4/M1.5 still compare **production I before versus after**, byte for byte; M1.6 still audits its unchanged commands,
resources and writes. The reference cannot satisfy any of those items or substitute for simulator qualification.

**Cause investigation and shader decision.** Identifying the exact derivative, scheduling or ordering cause is not
a prerequisite for this re-plan or M1 acceptance: the retained controls refute the original oracle's premise, and
the replacement checks production direct execution at matching partitions. This is not a declaration that the driver
is correct or that every edge difference is harmless. Investigate further if the replacement oracle fails, if scene
inspection fails or cannot classify a discrepancy, or if validation indicates undefined shader behaviour.

A partition-invariant shared shader change is **not commissioned or worthwhile for M1**. It would broaden work into
material/derivative semantics, may change filtering, quality and cost, and would threaten M1.4/M1.5's unchanged indirect
pixels. A direct-only shader would break the shared-pipeline requirement in M1.2. Reconsider only for a demonstrated
visible defect or a separately commissioned need for partition invariance, with cause isolation and quality/cost proof;
any change to the preserved indirect output or selection requires a separate scope decision.

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
path. The engine-state rows do not depend on the XCUITest gesture. If the gesture needs an unlocked
console, only the rotation row is recorded with that reason.

Console state is recorded before and after every simulator mode as witness metadata. Locked, unlocked and changed
states are accepted, and unavailable observations retain their reason. No manual-lock wait or lock state refusal
belongs in these checks. M1.7/M1.8 prove independence from desktop capture with denied read-only Screen Recording
preflights before and after under the identified launcher/harness permission identity. Missing, failed or granted
preflights leave that proof incomplete. The route continues to use engine state and simulator output; no grant is
requested. The builder replaces the lock admission in `scripts/cooked_draw_ios_sim.py` and updates the obsolete
manual-lock regression cases in `scripts/test_t2_capture.py`.

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
| One direct draw per meshlet | Hundreds of draws per mesh and no culling benefit. Matching the production indirect partition does not justify that fallback cost. |
| Whole-range direct versus selected meshlet indirect, with a pixel-count threshold or ignored edge mask | Conflates command execution with partition-dependent shading; a fitted threshold can conceal a real draw defect. M1.3 instead uses exact equality at matching partitions plus scene inspection. |
| Change the shared shader to force partition invariance | Low-level cause is unverified, and changing the supported indirect output violates the commissioned invariance. M1.3 needs a valid oracle, not a new shading model. |
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
| Fragment work | Visible geometry; partition-dependent helper/shading work | Same visible geometry; helper/shading work may differ |
| Memory | 20 B × M arguments, 160 B uniform, 4 B counter and readback, per placement | None of these |

**Tripwires and measurements:**
- **Indirect path.** It is unchanged by construction. Byte-identical output and a reviewed command diff prove it,
  and the existing G5 tripwires (`scripts/check_g5_tripwires.sh`) watch it.
- **Direct path on this Mac.** M1 records paired, order-alternated measurements against the indirect path: GPU
  forward and contribution time, and render-thread encode time. Each is labelled with its load class. No speed claim
  follows for either path.
- **Simulator.** Its timing is never recorded as speed.

The diagnostic reference adds O(P) 20-byte indexed-indirect argument records and O(P) commands per captured
reference frame, plus submitted-output readback already used by the harness. Repeats and negative controls multiply
offline proof work only. These controls add no ordinary-frame allocations, writes, dispatches or commands and are
excluded from M1.6's production-path timings.

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
- **Scene comparison shows direct drawing visible geometry the indirect cull rejected.** That would be an
  existing cull defect in the indirect path. Record it and report `BLOCKED` for a plan decision. Never alter the
  indirect path silently or waive the scene-correctness check. A shading-only byte difference does not establish this.
- **A reference repeats the direct defect.** Copying D's emitted ranges into R, comparing the same capture twice or
  leaving a diagnostic override enabled could manufacture parity. Independent cooked-range validation, arm/command
  witnesses, the D-only missing-mesh negative control and restored equality reject these substitutes.
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

## Code account and proof limits

**Original design inspection (before checkpoint `9c230ff4`; retained as rationale):**
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
- Production direct matches the full-range indirect reference exactly across all M1.3 views. The original
  cross-partition exactness assumption is refuted by the checkpoint; the revised oracle is not yet qualified.
- The simulator draws the rest of the cooked pipelines (ASTC or portable textures, 3D volume, casters) without
  further gaps.
- `simctl` screenshots of a headless simulator work without desktop capture, with denied Screen Recording
  preflights and observed console state. Recorded locked runs remain capability witnesses.
- The XCUITest orientation gesture may need an unlocked console. If it does, only the rotation row is attended.

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
