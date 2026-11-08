# Apple tile recipe plan (G3t)

Design: [APPLE-TILE.md](APPLE-TILE.md), governed by [the G3t boundary decision](../decisions/G3T-METAL-BOUNDARY.md);
contracts [C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners). Paired plan:
[APPLE-PRESENTATION-PLAN.md](APPLE-PRESENTATION-PLAN.md).
Commission: Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).
Reviews:
- D0.3 creation-counter seam @ `5550ee34` (landed as `be252c0d`), exact diff and verdicts in
  `tmp/g3t-metal-boundary/NOTES.md`: Thrum Core approved; G5 approved the cooked hunks; G6 agreed the `SceneSurface`
  device type.
- G3t pair @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/g3t-pair.md` (with `common.md`), report
  `tmp/g3t-g4-plans/reviews/g3t-pair-report.md`: changes required (9 required changes). Answered in revision 2;
  see the Log.
- Composed frame across G3t and G4 @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/composed-frame.md`, report
  `tmp/g3t-g4-plans/reviews/composed-frame-report.md`: changes required (14 required changes). Answered in
  revision 2.
- G3t pair re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (N1–N2 required,
  N3–N7 recommended). All answered in revision 3.
- G4 pair re-review @ `c507ad3d` (interfaces only here): approve with changes; see the G4 plan.
- Composed-frame re-review @ `c507ad3d`: approve with changes; its findings fall on G4 M4 and are answered in the G4
  plan's revision 3.
- Composed frame, exact-revision recheck @ `9938b0d0` (Opus 5.5 medium), brief `tmp/g3t-g4-plans/reviews/composed-frame.md`,
  report `tmp/g3t-g4-plans/reviews/composed-frame-report.md` ("Revision 4 recheck"): approve with changes (one leftover
  always-submitted rule, three small gaps). All applied in revision 4a.
- Archive review of revision 5 @ `16c5d8f7` (Opus 5.5 medium), report `tmp/g3t-g4-plans/reviews/composed-frame-report.md`
  ("Revision 5 archive review"): changes required (sizes buffer for all runtime-sized arrays, per-pipeline cook keys,
  cache patch scope, per-device archives, sizing). All applied in revision 5a.

Milestone: M0 counters and capture probe in progress (Claude G3t M0 session) · Updated: 2026-11-08
Now: D0.3 landed (`be252c0d`). D0.4 done: Xcode replay gives per-encoder device-memory bytes on the M3 Max ([probe](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md)). Seam 2 (registry, D1.1) is approved in shape and held at `76464f23` (local branch `claude/g3t-seam2-registry`); D1.2's per-variant eligibility report is priced, with a crate-private boundary.
Next: D0.1–D0.2 on G7's moving route (seam 8, M5a on main at `bc9a338d`) through the next-submission capture flag, priced in `tmp/g3t-metal-boundary/NOTES.md`; then seam 2 and D1.2 land together with the `THRUM_RECIPE` override, rechecked on both recipes. Traffic rows report physical per-encoder bytes beside the logical model.
Blocked: the capture flag waits for Root's nod on shape. D0.1's star-switch, mirror and cut-reset checkpoints come with G7's M5b. M1 code (seam 2, D1.2) waits for M0 to close, as Thrum Core ruled.
The saved offscreen trace opens directly in Xcode. Replay stayed disabled during local device discovery, so physical traffic
remains unmeasured. Any interactive retry needs an assigned slot.

M10–M11 also wait for seam 1. Root reviews that exact seam before an experiment.
The M0 commission and file holds are recorded in `tmp/g3t-metal-boundary/OVERSEER.md`.

## Game outcomes

On this M3 Max, the V1 game runs the valley route on the Apple tile recipe:
- the reference image matches portable within tolerance;
- no validation errors, and no pipeline compiled during play;
- Chrome and a forced-portable Mac run stay correct;
- each tile mechanism is measured against portable in paired rows, or recorded as having no effect at the shipped
  configuration;
- the iOS shell selects the same recipe in the simulator, for function only;
- G4 can call a prepared, stateful native provider inside the one submission.

## Evidence conventions

Shared with G4.
- **Route.** The fixed valley route from M0, run in fixed-step mode: a fixed simulation dt per frame and a fixed
  presentation alpha. That makes reruns comparable frame for frame.
- **Reference image.** The scene-linear image that feeds the present pass: scene HDR, or reconstruction output once G4
  lands.
  - The prepared capture variant reads it back at route checkpoints.
  - It is converted with the fixed SDR tone transform at headroom H = 1, then compared by per-pixel ΔE (CIEDE2000)
    against the portable run.
  - The drawable itself is not read back. It stays `framebufferOnly`, so capture never perturbs the presented path.
- **Image tolerance.** Provisional p99 ≤ 2 and max ≤ 6. M0's rerun sets the noise floor, and the threshold is
  restated from it.
- **Timing.** GPU pass and native-segment times, the pass sum, display-link present intervals and upload CPU time.
  Each is reported as p50 and p95.
- **Traffic.** Graph-declared bytes per attachment come from the compiled graph. Measured bytes use only a capture
  method M0 proved. Otherwise the row says *unmeasured*.
- **Counters.** Pipeline, texture and native-object creations, by thread and phase (build, transition, play), from
  M0.
- **"No measurable effect".** A difference within twice the p50 spread of M0's portable reruns.
- **Labels.** Mac rows that stand in for iPhone are labelled **proxy**. Simulator rows are function only. Physical
  iPhone timing is owed.
- **Tripwires** (not gates): a visible hitch; a comparable regression of about 25%; departure from the design's cost
  models.

## Ownership, call sites and review order

| Area | Owner after approval | Files | Shared call sites |
|---|---|---|---|
| Proof counters, capture probe | G3t | `thrum-render/src/runtime/creation.rs` (counters); `tmp/g3t-traffic/` (probe) | All G3t and G4 checks |
| Route and fixed-step mode | Seam 8 (G7 route owner) | `scripts/run_v1_showcase.py`, `examples/v1-game/params/`, the V1 game clock | G4, G7 |
| Registry fields | G3t | `thrum-render/src/capabilities.rs` | `thrum-app/src/app/device.rs:82`, `examples/v1-game-ios/src/ios/render.rs:204` |
| Graph eligibility, variants, native pass kind, split execution | G3t, with the graph owner's review | `thrum-render/src/graph.rs`, `graph_record.rs` | `atmosphere/graph.rs`, `synthetic.rs` scene build, `present.rs` |
| Merged graph, direct output, timing schema | G3t, with the frame-runtime and G3b owners' review | `runtime.rs`, `runtime/tokens.rs`, `present.rs`, `synthetic/display.rs`, `timing.rs` | macOS `render_actor.rs`, iOS `ios/render.rs`, capture readback |
| Per-slot frame buffers | G3t, with the atmosphere and G3c owners' review | `synthetic.rs:1446-1520`, `synthetic/upload.rs`, bind-group builders in forward, shadow, `visibility.rs`, `history_gpu.rs`, `atmosphere/gpu.rs` | `thrum-asset-gpu/src/staging.rs` (unchanged, counted) |
| Apple ring | G3t | new `apple_tile/{mod,ring}.rs` | Per-slot frame buffers |
| Shared transients | G3t | `runtime/preparation.rs`, `synthetic.rs` `OffscreenTarget` | Frame slots |
| Compact instances | G3t, with G3c and G3b interpolation review | `abi.rs`, `synthetic.wgsl`, `synthetic/shadow.wgsl`, `history_gpu.wgsl`, `visibility.wgsl`, `synthetic/upload.rs`, `thrum-app/src/sky.rs`, `tests/contracts.rs` | G3c |
| GPU light binning | G3t | `synthetic/cluster_gpu.rs`, `synthetic/cluster.rs` (oracle) | `synthetic.rs:757`, `:976` |
| Mirrored ranges | G3t, with G3c | `visibility.rs`, `visibility.wgsl`, `synthetic/camera_culling.rs`, `shadow.rs` | G3c indirect draws |
| Native segment, providers | G3t (mechanism), G4 (MetalFX provider) | new `apple_tile/segment.rs` | G4 `apple_tile/metalfx.rs` |

Review order:
1. Counters.
2. Registry.
3. Graph eligibility and variants.
4. Split execution and merged graph.
5. Per-slot buffers.
6. Ring.
7. ABI.
8. Binning and mirrors.
9. Native segment.

G4 work that touches the same files lands after the matching G3t step (see below).

## Parallel work and handoffs with G4

| G3t milestone | Can start when | Runs in parallel with |
|---|---|---|
| M0 route, counters, probe | Counters and probe: nothing, commissioned separately. Route: seam 8, reviewed with G7 | G4 M1 |
| M1 registry and memoryless | M0, atmosphere checkpoint, seams 2 and 3 | G4 M1 |
| M2 split-execution merged graph | M1 | G4 M1 |
| M3 direct output and timing schema | M2 | G4 M2, which rebases its `runtime.rs` wiring onto M2 |
| M4 per-slot frame buffers | M1, seam 4 | M6–M9, G4 M2–M3 |
| M5 Apple ring | M4, seam 9 | M6–M9 |
| M6 shared transients | M3 | M4–M5, M7–M9 |
| M7 compact instances | M1, seam 6; after G3c's edits to the same shaders | M4–M6, M8 |
| M8 GPU light binning | M1 | M4–M7 |
| M9 mirrored ranges | M7 (same rows), seam 7 | M8 |
| M10 native segment, stand-in | M3, seam 1 | G4 M4–M7 |
| M11 native recovery, rejection, timing | M10 and G4 M4 (real extent transitions) | G4 M5–M7 |
| M12 paired record | M1–M11, G4 M2 (readers in place) and G4 M5 (scale) | G4 M8 |

**Handoffs from G3t to G4.**
- *M0 → every G4 check.* Route, fixed-step mode, counters and the traffic capture method (or the *unmeasured*
  label).
- *M1 → G4 M2.* The per-variant eligibility report, which shows depth and motion becoming stored once G4's readers
  exist.
- *M3 → G4 M4 and M6.* Present owns tone and display encode, and the surface is bound at execute time.
  - Contract: the present pass reads one scene-linear input (scene HDR, or G4's reconstruction output) plus the
    frame's exposure and headroom H. It writes the drawable in the surface format chosen at the generation build.
  - Unlocking proof: D3.1–D3.3.
  - G4 M4 (render extent from the surface) waits for M3, so G3t M2–M3 keep 640×360 and its letterbox tests until
    then.
- *M11 → G4 M8 (MetalFX), the first G4 milestone needing the native handoff.*
  - Producer: the native segment executor and the `NativeProvider` trait.
  - Consumer: G4's MetalFX provider.
  - Contract: created at the generation build with declared formats, texture usages, extents and scale range; encodes
    only graph textures into graph textures inside the segment's raw command buffer; may keep internal history and
    resets when told; completes with the slot's submission; reports segment timing; names a prepared portable twin.
  - Unlocking proof: D10.1–D10.4 and D11.1–D11.3 on this M3 Max.

**Handoffs from G4 to G3t.** G4 M2 adds the depth and motion readers that M12 reruns eligibility with. G4 M5's scale
steps give M12 its two-scale traffic rows.

## M0 Proof route, counters and capture probe: pending

M0 has two independent parts. The counters and capture probe (D0.3, D0.4) can be commissioned on their own, with no
dependency on the shared graph or the route. The route (D0.1, D0.2) waits for seam 8, a G7-reviewed seam.

Done when:
- [ ] D0.1 One command runs the V1 game through a fixed valley route in fixed-step mode: a pan across the Sun side, a
      walk past local lights and mirrored props, a dusk crossing into the star variant, and one cut. It writes
      reference-image checkpoints on macOS (Metal) and in Chrome.
- [ ] D0.2 Two portable runs on this M3 Max give identical checkpoint images. The log names the recipe, window size and
      display refresh. The noise floor of timing is recorded.
- [x] D0.3 Pipeline, texture and native-object creation counters report counts by thread and phase. A test fails if a
      pipeline is created after readiness.
- [x] D0.4 A capture probe on the offscreen valley either produces a per-encoder byte counter for one attachment (an
      Xcode GPU frame capture through `MTLCaptureManager`, or another explicitly selected counter set) or records that
      none is available. The plans' traffic rows then use that method or the *unmeasured* label.

Work:
- [ ] Route keyframes with a cut marker, built from the existing showcase params (→D0.1).
- [ ] Fixed-step route clock with fixed presentation alpha, under seam 8 (→D0.1, D0.2).
- [ ] Capture-variant readback at route checkpoints, and a determinism rerun (→D0.1, D0.2).
- [x] Creation counters in the runtime (opt-in; the generation-owned pipeline seam), and the
      after-readiness test (→D0.3).
- [x] Capture probe saved under `tmp/g3t-traffic/`, with the result in `docs/evidence/render/G3T-TRAFFIC-PROBE-*.md`
      (→D0.4). Result: Xcode's per-encoder device-memory bytes, in
      [G3T-TRAFFIC-PROBE-2026-11-08](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md).
      Open: a single sample per trace, so about ±1 MiB per encoder is noise; the replay noise floor is measured in the
      later tuning phase.

## M1 Recipe selection and graph-derived memoryless: pending

This is the initial functional milestone. It needs no tuning.

Done when:
- [ ] D1.1 `DeviceCapabilities` carries `Recipe`, `NativeFeatures` (Apple) and a digest.
  - Selection: Apple tile on this M3 Max and in the iOS simulator, portable in Chrome.
  - A forced setting selects portable on Mac.
  - Unit tests cover selection from adapter facts.
- [ ] D1.2 The compiled graph reports memoryless eligibility per prepared variant. Graph tests cover:
  - day depth is eligible;
  - star-variant depth is stored;
  - Sun cascades are stored;
  - an explicit `store` (today's motion) is ineligible;
  - an added later reader flips an attachment to stored;
  - `History` and `Persistent` resources are never eligible;
  - the capture variant does not change eligibility in the normal variant.
- [ ] D1.3 Under Apple tile, a scripted log reads each eligible attachment's storage mode through `Texture::as_hal` and
      shows `Memoryless`. Stored attachments show `Private`. Portable is unchanged. A dusk crossing on the route
      creates no textures (counter).
- [ ] D1.4 The valley route runs on both recipes within tolerance with no validation errors. Chrome is unchanged.

Work:
- [ ] `Recipe`, `NativeFeatures`, digest and unified-memory query; capture on macOS and iOS; forced override (→D1.1).
- [ ] Per-variant eligibility in `FrameGraph::compile` and the graph tests (→D1.2).
- [ ] Separate memoryless allocations per variant, created at the generation build (→D1.3).
- [ ] Storage-mode log and route rows in `docs/evidence/render/G3T-M1-*.md` (→D1.3, D1.4).

## M2 One graph with split execution: pending

Done when:
- [ ] D2.1 Scene and present compile once per generation as one graph with an execute-time surface binding. Execution
      runs phase 1 before acquisition and phase 2 after. The compiler refuses a non-surface pass after the surface
      pass (test).
- [ ] D2.2 A resize at the current fixed render extent rebinds the surface without recompiling the scene. The
      surface tests pass on macOS and iOS: `surface_pass_letterboxes_and_preserves_sdr_bytes_across_srgb_formats`,
      and `unavailable_surface_drops_only_views_and_resize_reuses_preparation`, unchanged: the presentation-drop
      contract is preserved.
- [ ] D2.3 A failed acquisition follows the existing drop contract under the merged graph: views drop, pre-acquire
      work may be cancelled, and history stays at the last submitted sample (frame-token tests). On iOS, a frame
      found after the app leaves the foreground is dropped with a history reset (test). The valley
      stays within tolerance on both recipes.

Work:
- [ ] Bind both record families under one compiled graph with phase partitioning (→D2.1).
- [ ] Execute-time surface binding and the resize path (→D2.2).
- [ ] The existing drop contract under split execution in the macOS render actor and the iOS render owner (→D2.3).
- [ ] Test runs and route rows (→D2.2, D2.3).

## M3 Direct output and timing schema: pending

Done when:
- [ ] D3.1 The present pass reads scene HDR and writes the drawable with tone and display encode. The `Rgba8Unorm`
      output target is gone from the steady graph, and graph-declared bytes drop by its write and read.
- [ ] D3.2 The capture variant still produces reference images.
      `surface_pass_letterboxes_and_preserves_sdr_bytes_across_srgb_formats` passes on macOS and iOS as the guard
      for the moved tone and display encode.
- [ ] D3.3 `FrameSample` carries GPU times keyed by pass name, and the timing tests pass. The valley stays within
      tolerance on both recipes, and Chrome is unchanged.

Work:
- [ ] Move tone and display encode from `synthetic/display.rs` into the present pass (→D3.1).
- [ ] A capture branch reading the scene-linear input (→D3.2).
- [ ] Timing schema in `timing.rs` and `runtime.rs`, with consumers updated (→D3.3).
- [ ] Route rows (→D3.1, D3.3).

## M4 Per-slot frame buffers: pending

Done when:
- [ ] D4.1 On both recipes, the instance, directional, local-light, cluster and cluster-bounds buffers exist per slot,
      with per-slot bind groups built at the generation build. No bind group is created during play (counter).
- [ ] D4.2 Instance dirty rows reach each slot's copy. `active_prefix_matches_fresh_scene_through_churn_and_slot_reuse`
      and `changed_slot_updates_pixels_and_invalid_row_keeps_prior_draw` pass. A new churn test alternates slots with
      different dirty sets.
- [ ] D4.3 The valley stays within tolerance on both recipes. Upload CPU p50 and p95 are recorded against M0's.

Work:
- [ ] Per-slot buffer sets and bind groups in forward, shadow, visibility, history and atmosphere (→D4.1).
- [ ] Dirty-union propagation in `synthetic/upload.rs` and the churn test (→D4.2).
- [ ] Route rows (→D4.3).

## M5 Apple shared-storage ring: pending

Done when:
- [ ] D5.1 On Apple tile, the per-slot buffers are persistently mapped shared `MTLBuffer`s, imported with
      `create_buffer_from_hal`. Steady route frames make zero `write_buffer` calls for the M4 classes, and the counted
      busy-slot fallback is reported. Portable still uses `write_buffer`.
- [ ] D5.2 The M4 tests pass on Apple tile, and the valley is within tolerance.
- [ ] D5.3 Upload CPU p50 and p95 are recorded on both recipes.
  - If the raw import fails validation or loses to `write_buffer`, the `map_async` fallback's result is recorded
    instead.
  - The design's open question is answered either way.

Work:
- [ ] `apple_tile/ring.rs`: shared buffer creation on the same `MTLDevice`, import, contents-pointer writes after
      `render_done` (→D5.1). Preconditions: shared storage, and wgpu usages including `COPY_DST` for the
      `write_buffer` fallback. wgpu treats imported buffers as initialized and does not check usage flags against
      the raw buffer, so the ring code asserts these itself.
- [ ] Busy-slot fallback and counter (→D5.1).
- [ ] Test and route runs (→D5.2, D5.3).

## M6 Shared transients: pending

Done when:
- [ ] D6.1 One transient set per generation serves both slots. Graph-declared transient footprint halves in the rows.
- [ ] D6.2 A held-submission test (slot N's GPU work delayed) shows no corruption in slot N+1. The valley stays within
      tolerance.

Work:
- [ ] Move `GraphTextures` ownership from `OffscreenTarget` per slot to the generation (→D6.1).
- [ ] Held-submission test and route rows (→D6.2).

## M7 Compact instances: pending

Done when:
- [ ] D7.1 `INSTANCE_BYTES` is 80 on both recipes: a current 3×4 affine, the previous simulation sample's translation,
      ids and padding. Shaders derive the normal matrix. Display-link interpolation of translation by
      `presentation_alpha` is unchanged.
- [ ] D7.2 Upload, culling, mirrored-row and motion-history tests pass, including
      `metal_motion_history_uses_interpolated_display_samples` and
      `metal_motion_history_follows_submitted_frames_and_resets`.
- [ ] D7.3 The valley stays within tolerance. A 10k-instance row records instance bytes at 0.42 of the 192 B form.

Work:
- [ ] ABI in `abi.rs`, WGSL in the four shaders, `sky.rs` and contract tests (→D7.1).
- [ ] Update the upload, history, culling and shadow paths (→D7.1, D7.2).
- [ ] Test runs and the 10k row (→D7.2, D7.3).

## M8 GPU light binning: pending

Done when:
- [ ] D8.1 The GPU cluster build fills the 32-per-cluster CSR for any light count, keeping the lowest-index lights on
      overflow and counting the overflow. A test checks per-cluster set equality with the CPU oracle, under the same
      rule, at 4, 16, 32 and 64 lights.
- [ ] D8.2 A 4→64-light route row on both recipes records GPU binning time, forward time and CPU frame time against
      the design's light model. CPU frame time stays within noise from 4 to 64 lights on the GPU build.

Work:
- [ ] One invocation per cluster in index order in `cluster_gpu`; remove the 4-light gate; oracle rule in
      `cluster.rs` (→D8.1).
- [ ] Light-scaling params and rows (→D8.2).

## M9 Mirrored ranges: pending

Done when:
- [ ] D9.1 Visibility compute writes normal and mirrored indirect records. Forward and shadow draw them with opposite
      cull faces, and no draw switches to cull-none.
- [ ] D9.2 `mirrored_row_uses_two_sided_fallback_and_keeps_near_surface` is replaced by a test with the same visual
      assertion on the mirrored range. The valley with mirrored props stays within tolerance.
- [ ] D9.3 A row with 0 and 1/11 mirrored instances records +1 indirect draw and GPU time against the model.

Work:
- [ ] Two indirect records in `visibility.wgsl` and `visibility.rs`; draws in `camera_culling.rs` and `shadow.rs`
      (→D9.1).
- [ ] Test replacement and route rows (→D9.2, D9.3).

## M10 Native segment with a stateful stand-in: pending

This is proven with a labelled stand-in, not MetalFX. It needs seam 1, and Thrum Core reviews the exact seam (the
design's tier-3 transitions and this stand-in) before the milestone starts.

Done when:
- [ ] D10.1 The graph accepts a `NativeSegment` pass with declared reads and writes. The executor closes the wgpu
      encoder, records the segment in a dedicated `as_hal_mut` encoder and resumes, with all encoders in one submit.
      Graph tests cover hazards across the segment, refusal of a surface import, and native-written textures being
      unaliased and never discarded.
- [ ] D10.2 A **stand-in** provider is a native compute kernel. It blends scene HDR with an internal history texture,
      writes through storage usage into a graph texture that the present pass reads, and resets on command. It runs
      for the whole route with no validation errors. It matches its wgpu compute twin within 1 ULP of `f16` per
      channel: the two are compiled by different front ends (MSL source vs naga), so bit equality is not required.
- [ ] D10.3 A held-submission test shows no native texture is freed or reused before slot completion.
- [ ] D10.4 Evidence labels the stand-in as composition proof only, not MetalFX or tile proof.
- [ ] D10.5 The stand-in covers, each with a test and a counter in the proof log:
  - same-submit visible output;
  - history across frames;
  - provider replacement;
  - a discarded encoded frame, which resets the provider on its next encode, or, if Thrum Core admits seam 3's runtime
    change, a native frame submitted without present;
  - the iOS background race: a frame encoded before suspension is dropped and history resets.

Work:
- [ ] `PassBackend::NativeSegment`, the encoder list, and the native texture rules in graph and executor (→D10.1).
- [ ] `apple_tile/segment.rs`: `NativeProvider` trait, trampoline, one-time wgpu initialization (→D10.1, D10.2).
- [ ] Stand-in provider and its wgpu twin (→D10.2).
- [ ] Held-submission test and evidence note (→D10.3, D10.4).
- [ ] Discard, reset and background-race tests with outcome counters (→D10.5).

## M11 Native recovery, rejection and timing: pending

Done when:
- [ ] D11.1 With G4 M4's real extent transitions in place, a resize builds a new provider in the transition off the
      render thread. The pipeline counter reads zero in play, and native-object creations appear only in build or
      transition phases.
- [ ] D11.2 A forced provider rejection and a surface replacement select the prepared wgpu twin without pipeline
      creation. A submission failure resets provider history on the next frame (test).
- [ ] D11.3 Segment GPU start and end times appear in `FrameSample` under the segment's pass name, and pass a sanity
      check: inside the frame's GPU span, and non-zero.

Work:
- [ ] Provider rebuild in G4's transition path (→D11.1).
- [ ] Rejection, replacement and failure-reset wiring (→D11.2).
- [ ] Segment timing from the raw command buffer's GPU times (→D11.3).

## M12 Paired traffic and timing record: pending

This milestone is tuning, separate from the functional work above.

Done when:
- [ ] D12.1 Paired route rows (Apple tile vs portable, this M3 Max) record:
  - GPU pass and segment times;
  - present intervals;
  - upload CPU;
  - graph-declared bytes;
  - measured bytes by the M0 method, or *unmeasured*;
  - the recipe and registry digest.

  The rows cover two render scales (1.0 and 0.67, from G4 M5).
- [ ] D12.2 The eligibility report is rerun with G4 M2's readers in place, per variant.
- [ ] D12.3 For each mechanism from M1 and M3–M9, a one-off ablation row shows a measured difference or *no measurable
      effect* (the M0 threshold). Every open tripwire has an assigned fix.

Work:
- [ ] Route runs at two scales with the M0 capture method (→D12.1).
- [ ] Eligibility rerun (→D12.2).
- [ ] Ablation switches and rows (→D12.3).
- [ ] Evidence record `docs/evidence/render/G3T-PAIRED-*.md`; maturity rows go to the ledger owner (→D12.1–D12.3).

## Sizing

| Milestone | Days |
|---|---|
| M0 | 1 |
| M1 | 1.5 |
| M2 | 1.5 |
| M3 | 1 |
| M4 | 1 |
| M5 | 1 |
| M6 | 1 |
| M7 | 1.5 |
| M8 | 1 |
| M9 | 1 |
| M10 | 1.5 |
| M11 | 1 |
| M12 | 1.5 |

That is **15.5 build days**, plus 1–2 days of review and seam turnaround, for G3t alone.
- With two agents splitting M4–M9 by the parallel table, the elapsed time is about **11–12 days**.
- M2 and M3 carry the most uncertainty: the frame runtime, both shells and active G3b code.
- The total revises revision 1's 8.5 days. Reviewers judged that figure optimistic and estimated 11–13 build days.
  This plan adds the proof tooling, the per-slot buffer redesign and the native recovery work that reviewers found
  missing.

## Log

- 2026-11-07 Revision 1 drafted for Thrum Core's G3t/G4 commission.
- 2026-11-07 Revision 2 answers the G3t pair review and the composed-frame review:
  - **Proof tooling:** a capture probe and counters moved to M0, with an *unmeasured* label as the fallback, and a
    fixed-step route with a defined reference image.
  - **Instances:** 80 B rows keep the previous translation that interpolation reads; every consumer is listed.
  - **Uploads:** per-slot buffers and bind groups specified; the Apple ring is a raw shared-storage import, with
    `map_async` as the fallback.
  - **Graph:** per-variant memoryless allocations; split execution with an execute-time surface; encoded frames always
    submitted.
  - **Native segment:** texture rules, a stateful storage-writing stand-in and segment timing added; the consequence of
    refusing seam 1 is stated.
  - **Cross-plan:** numbering corrected; the M3 → G4 handoff added; the extent change is sequenced after M3.
  - **Lights and mirrors:** a deterministic GPU binning oracle, cost models, and mirrors through G3c's indirect
    records.
  - **Sizing:** restated.
- 2026-11-07 Revision 3 answers the G3t re-review:
  - the surface test is rewritten for submit-without-present, with G3b owner review;
  - submit-without-present runs on iOS only in the foreground;
  - the lifetime reason is corrected;
  - the ring preconditions are stated;
  - instance rows are 80 B;
  - the stand-in tolerance is stated;
  - the tone-encode guard test is named.
- 2026-11-07 Revision 4 answers Thrum Core's review of revision 3:
  - M0's counters and capture probe are commissionable separately; the route stays a G7-reviewed seam;
  - the native segment is marked a proposed amendment, with producer and consumer transitions, Metal's tracking
    guarantees and what wgpu-core cannot observe;
  - the stand-in also covers a discarded encoded frame and the iOS background race (D10.5);
  - the portable presentation-drop contract is restored in M2; submit-without-present is limited to frames carrying a
    stateful native segment, behind seam 3;
  - archives are put to the owner as (a) against (b), with no V1 exit wording change;
  - the atmosphere checkpoint is `d2a5342`, for interface reads.
- 2026-11-07 Revision 4a applies the composed-frame recheck of `9938b0d0`: the lifetime guarantee is generation
  ownership, not wgpu-hal's retained-references default, and N's completed handler raises device loss.
- 2026-11-07 Revision 5 rewrites seam 5 to the owner's direction: measure first, then offline metallibs through
  passthrough and an upstream Metal `PIPELINE_CACHE` (gfx-rs/wgpu#8526). The rewording option is withdrawn.
- 2026-11-07 Revision 5a applies the archive review of `16c5d8f7` to seam 5: the cook is per pipeline, the patch carries
  `MetalLib` reflection, the validation key, a Metal `adapter_key` and the archive-miss option, and archives come from
  each shipped device.
- 2026-11-07 Revision 5b adds parallel pre-warm, the cook's MSL match check and the measure → metallib plus pre-warm →
  `PIPELINE_CACHE` order to seam 5.
- 2026-11-07 Revision 6 makes seam 5's upstream contribution a later, non-V1-gating item under the owner's rules. The
  pinned patch delivers V1's archives.
- 2026-11-07 Revision 7 cuts seam 5 to outcome level under the owner's direction; details are designed when G4 M10
  starts.
