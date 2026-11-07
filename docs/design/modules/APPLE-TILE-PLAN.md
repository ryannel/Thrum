# Apple tile recipe plan (G3t)

Design: [APPLE-TILE.md](APPLE-TILE.md), governed by [the G3t boundary decision](../decisions/G3T-METAL-BOUNDARY.md);
contracts [C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners). Paired plan:
[APPLE-PRESENTATION-PLAN.md](APPLE-PRESENTATION-PLAN.md).
Commission: Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).
Reviews: pending.
Milestone: none started (plan awaiting review) · Updated: 2026-11-07
Now: design and plan drafted for review (since 2026-11-07).
Next: independent pair review and composed-frame review; then Thrum Core's seam rulings.
Blocked: code commissions wait for the atmosphere graph checkpoint (G3S-HANDOFF "Reconcile the culling count graph on
main") and Thrum Core approval of the exact seams in the design's *Seams* list.

## Game outcomes

On this M3 Max, the V1 game runs the valley route on the Apple tile recipe. The image matches portable within the
declared tolerance, with no validation errors and no compile during play. Chrome and a forced-portable Mac run stay
correct. Each tile mechanism is either measured against portable in paired rows, or recorded as having no effect at
the shipped configuration. The iOS shell selects the same recipe in the simulator, for function only. G4 can call a
prepared native provider inside the one submission.

## Evidence conventions

- **Route.** The fixed valley route from M0, unless a check names a showcase stop. Rows are Apple tile vs portable on
  the same M3 Max, same build, same window size.
- **Image tolerance.** Per-pixel ΔE (CIEDE2000) on the presented SDR image at the route's checkpoint frames: p99 ≤ 2
  and max ≤ 6 against the portable capture. M0 records a portable-vs-portable rerun, which must be identical. The
  threshold is provisional until that rerun shows the noise floor.
- **Timing.** GPU pass timestamps, the existing pass sum, whole-frame present intervals and upload CPU time, as p50 and
  p95 over the route.
- **Traffic.** Graph-declared bytes per attachment come from the compiled graph. Measured bandwidth comes from an
  Instruments *Metal Application* capture with the GPU counter set explicitly configured (Performance Limiters), or
  from an Xcode frame capture's memory counters. The two are recorded separately.
- **Labels.** Mac rows that stand in for iPhone are labelled **proxy**. Simulator rows are function only. Physical
  iPhone timing is owed.
- **Tripwires** (not gates): a visible hitch; a comparable regression of about 25%; scaling that departs from the
  cost account.

## Ownership, call sites and review order

| Area | Owner after approval | Files (current → proposed) | Shared call sites |
|---|---|---|---|
| Registry fields | G3t | `thrum-render/src/capabilities.rs` | `thrum-app/src/app/device.rs:82` (capture), `examples/v1-game-ios/src/ios/render.rs:204` (iOS capture), evidence rows |
| Graph eligibility, native pass kind, merged plan | G3t, with the graph owner's review | `thrum-render/src/graph.rs`, `graph_record.rs` | Atmosphere `atmosphere/graph.rs`, `synthetic.rs` scene build, `present.rs` |
| One scene/present graph, shared transients | G3t, with the frame-runtime owner's review | `thrum-render/src/runtime.rs`, `runtime/tokens.rs`, `present.rs`, `synthetic/display.rs` | iOS and macOS surface paths (G3b), capture readback |
| Upload rings | G3t | new `thrum-render/src/apple_tile/upload_ring.rs`; `synthetic/upload.rs`, `synthetic.rs:1446-1520` | `thrum-asset-gpu/src/staging.rs` (unchanged; counted) |
| Compact instance ABI | G3t, with G3c's review | `thrum-render/src/abi.rs`, `synthetic.wgsl`, `synthetic/upload.rs`, `history_gpu`, `camera_culling.rs`, `shadow.rs` | G3c culling, `thrum-asset-gpu/src/placement.rs` |
| GPU light binning | G3t | `synthetic/cluster_gpu.rs`, `synthetic/cluster.rs` (oracle) | `synthetic.rs:757`, `:976` selection |
| Mirrored ranges | G3t | `synthetic/upload.rs:280`, `synthetic.rs:2409`, `shadow.rs:273` | G3c visibility prefix |
| Native segment and providers | G3t (mechanism); G4 (providers) | new `thrum-render/src/apple_tile/{mod,segment}.rs` | G4 `reconstruction/` |
| Proof route and paired runner | Seam (G7 route owner) | `scripts/run_v1_showcase.py`, `examples/v1-game/params/` | G4 and G7 evidence |

Review order follows the dependencies:
1. Registry.
2. Graph eligibility and native pass kind.
3. Merged graph and shared transients.
4. ABI.
5. Uploads.
6. Binning and mirrors.
7. Native segment.

G4's frame contract and presentation work slot in after step 3, as its plan states.

## Parallel work and seams

| Work | Can start when | Runs in parallel with |
|---|---|---|
| M0 route | Route seam ruled | G4 M1 design-to-code prep |
| M1 registry and transients | Atmosphere graph checkpoint plus seams 2 and 3 approved | G4 M1 (portable frame contract) |
| M2 merged graph and direct output | M1 | G4 M1, which must rebase its reconstruction pass onto the merged graph |
| M3 rings and shared transients | M2 | M4, M5, G4 M2 |
| M4 compact instances | M1 and seam 4 approved; serialised with G3c edits to the same files | M3, M5 |
| M5 binning and mirrors | M1 | M3, M4 |
| M6 native segment | M2 and seam 1 approved | G4 M2–M4 |
| M7 paired record | M1–M6, and G4 M1 for the eligibility rerun | G4 M5 |

**Handoff to G4.** G4 M5 (MetalFX temporal) is the first G4 milestone that needs G3t, and it needs M6.
- *Producer:* the native segment executor and the `NativeProvider` trait.
- *Consumer:* the G4 MetalFX provider.
- *Contract:* the provider is created at closure for a device and recipe generation, with declared formats, extents
  and scale range. It encodes only into the segment's raw command buffer, from graph textures to a graph texture.
  It completes with the slot's submission. It names a prepared portable twin.
- *Unlocking proof:* M6's stand-in provider passes its done-when checks on this M3 Max.

## M0 Valley proof route: pending

Shared with G4 and G7. The owner follows Thrum Core's ruling on seam 6; the plan sizes it at half a day.

Done when:
- [ ] D0.1 One command runs the V1 game through a fixed, timed valley camera route with motion: a pan across the Sun
      side, a walk past local lights and mirrored props, and one cut. It records presented checkpoint frames on
      macOS (Metal) and in Chrome.
- [ ] D0.2 Two portable runs on the same M3 Max give identical checkpoint frames, and the run log names the recipe,
      registry digest, window size and display refresh.
- [ ] D0.3 The paired runner writes a row file with the fields listed under *Evidence conventions*. A forced-recipe
      setting selects portable or Apple tile on Mac.

Work:
- [ ] Route definition as params keyed by time: camera pose keyframes plus a cut marker, using existing showcase
      params (→D0.1).
- [ ] Checkpoint capture through the existing capture readback at fixed route times (→D0.1, D0.2).
- [ ] Determinism rerun and noise floor recorded in `docs/evidence/render/` (→D0.2).
- [ ] Row writer and forced-recipe environment setting (→D0.3).

## M1 Recipe selection and graph-derived transients: pending

This is the initial functional milestone. It needs no tuning.

Done when:
- [ ] D1.1 `DeviceCapabilities` carries `Recipe`, `NativeFeatures` (Apple) and a digest. On this M3 Max the registry
      selects Apple tile, the iOS simulator selects Apple tile, and Chrome selects portable. A forced setting selects
      portable on Mac. Unit tests cover selection from adapter facts.
- [ ] D1.2 The compiled graph reports memoryless eligibility per transient attachment. Graph tests cover:
  - depth that nothing reads after forward is eligible;
  - the star variant's depth read makes depth stored and ineligible;
  - Sun shadow cascades sampled by forward are ineligible;
  - adding a later motion or depth reader (the G4 M1 frame contract shape) flips the attachment to stored;
  - a `History` or `Persistent` resource is never eligible.
- [ ] D1.3 Under Apple tile, eligible attachments are allocated with `RENDER_ATTACHMENT | TRANSIENT_ATTACHMENT`. An
      Xcode frame capture of the valley shows memoryless storage on each one. Portable allocates them unchanged.
- [ ] D1.4 The valley route runs on both recipes within image tolerance, with no validation errors, and Chrome is
      unchanged.

Work:
- [ ] `Recipe`, `NativeFeatures` and digest in `capabilities.rs`; capture on macOS and iOS; forced override (→D1.1).
- [ ] Eligibility pass in `FrameGraph::compile`: every access within one render pass, load Clear or DontCare, store
      Discard, single mip and layer, 2D (→D1.2).
- [ ] Graph tests for the five eligibility cases (→D1.2).
- [ ] `GraphTextures` adds the transient usage when the recipe enables it and the attachment is eligible (→D1.3).
- [ ] Capture record plus route rows in `docs/evidence/render/G3T-M1-*.md` (→D1.3, D1.4).

## M2 One scene/present graph and direct output: pending

Done when:
- [ ] D2.1 Scene and present compile as one graph per frame with the surface imported late. The present pass reads
      scene HDR and writes the drawable with the tone and display encode. The `Rgba8Unorm` output target no longer
      exists in the steady path, and graph-declared bytes drop by its write and read.
- [ ] D2.2 Capture readback still works as a graph branch, and the existing surface tests pass on macOS and iOS:
      letterbox, sRGB formats, unavailable surface, resize reuse.
- [ ] D2.3 Late acquisition is unchanged: the drawable is acquired after the scene encode, as the frame tokens
      require.
- [ ] D2.4 The valley route stays within tolerance on both recipes, and Chrome is unchanged.

Work:
- [ ] Merge `PresentContext` into the scene record family, or bind both families under one compiled graph, in
      `graph_record.rs` and `runtime.rs` (→D2.1, D2.3).
- [ ] Move tone and display encode from `synthetic/display.rs` into the present pass, keeping the 640×360 render
      extent and letterbox (→D2.1).
- [ ] Keep capture readback as an optional graph branch (→D2.2).
- [ ] Surface and frame-token test runs, then route rows (→D2.2, D2.3, D2.4).

## M3 Mapped upload rings and shared transients: pending

Done when:
- [ ] D3.1 Under Apple tile, steady frames on the route make zero `queue.write_buffer` calls for instance, view, light
      and cluster data, counted in the row. Portable keeps `write_buffer`. A busy slot falls back to `write_buffer`
      for that frame and counts it.
- [ ] D3.2 One transient set serves both frame slots. The rows record the graph-declared transient bytes halving.
      A held-submission test (slot N's GPU work delayed) shows no corruption in slot N+1.
- [ ] D3.3 Upload CPU p50 and p95 are recorded for both recipes. The valley stays within tolerance.

Work:
- [ ] Per-slot `MAP_WRITE` ring in `apple_tile/upload_ring.rs`, mapped after the slot's `render_done` (→D3.1).
- [ ] Route the `update_frame` writes through a ring or a `write_buffer` sink chosen by the recipe (→D3.1).
- [ ] Move `GraphTextures` ownership from `OffscreenTarget` per slot to one set per recipe generation (→D3.2).
- [ ] Held-submission test and route rows (→D3.2, D3.3).

## M4 Compact instances: pending

Done when:
- [ ] D4.1 `INSTANCE_BYTES` is 64 on both recipes: a 3×4 affine plus ids. The previous transform comes from the GPU
      history buffer, and the normal matrix is derived in the shader.
- [ ] D4.2 The existing upload, culling, mirrored-row and motion-history tests pass, including
      `metal_motion_history_follows_submitted_frames_and_resets`.
- [ ] D4.3 The valley stays within tolerance. A 10k-instance row records instance bytes at one third of the 192 B form.

Work:
- [ ] ABI change in `abi.rs` and WGSL; shader-side normal derivation (→D4.1).
- [ ] Update `history_gpu`, `camera_culling`, `shadow` and the upload paths (→D4.1, D4.2).
- [ ] Test runs and the 10k row (→D4.2, D4.3).

## M5 GPU light binning and mirrored ranges: pending

Done when:
- [ ] D5.1 The GPU cluster build fills the full 32-per-cluster CSR for any light count, and counts overflow. A test
      matches its table to the CPU oracle at 4, 16, 32 and 64 lights.
- [ ] D5.2 Mirrored rows are drawn as their own range with the opposite cull face. All other rows keep back-face
      culling. The mirrored-row test passes, and the draw count rises by one instead of switching everything to
      cull-none.
- [ ] D5.3 A scaling row from 4 to 64 local lights, and with 0 and 1/11 mirrored rows, records GPU and CPU frame time
      on both recipes, compared against the design's cost account.

Work:
- [ ] Extend `cluster_gpu` and remove the 4-light gate for Apple tile and portable desktop (→D5.1).
- [ ] Partition the active prefix by determinant sign; two draws with set cull modes in forward and shadow (→D5.2).
- [ ] Scaling scene params and rows (→D5.3).

## M6 Native segment handoff: pending

This is proven with a labelled stand-in, not MetalFX. It needs Thrum Core's ruling on seam 1.

Done when:
- [ ] D6.1 The graph accepts a `NativeSegment` pass with declared reads and writes. The executor closes the wgpu
      encoder, records the segment in a dedicated encoder through `as_hal_mut`, and resumes. All encoders go to one
      `queue.submit`. Graph tests cover hazards, lifetime and store across the segment, and refuse a segment that
      imports the surface.
- [ ] D6.2 A **stand-in** provider performs a native `MTLBlitCommandEncoder` copy of scene HDR into a graph texture,
      which the present pass then reads. It runs for the whole route with no validation errors. Output matches the
      wgpu-copy twin bit for bit, and the provider retires only after slot completion.
- [ ] D6.3 Resize, surface replacement and a forced provider rejection each switch to the prepared portable twin with
      no pipeline creation after readiness. The pipeline-creation counter reads zero during play.
- [ ] D6.4 Evidence labels the stand-in as composition proof only, not MetalFX or tile proof.

Work:
- [ ] `PassBackend::NativeSegment` and the executor's encoder list (→D6.1).
- [ ] `apple_tile/segment.rs`: `NativeProvider` trait, trampoline, one-time wgpu initialization of native-written
      textures (→D6.1, D6.2).
- [ ] Stand-in blit provider and its wgpu twin (→D6.2).
- [ ] Pipeline-creation counter in the runtime; recovery and rejection rows (→D6.3).
- [ ] Evidence note (→D6.4).

## M7 Paired traffic and timing record: pending

This milestone is tuning, separate from the functional work above.

Done when:
- [ ] D7.1 Paired route rows (Apple tile vs portable, same M3 Max) record GPU pass times, present intervals, upload CPU,
      graph-declared bytes per attachment and measured bandwidth from a configured Instruments capture. Every row
      names the recipe and registry digest.
- [ ] D7.2 The eligibility report is rerun with G4 M1's frame contract (depth and motion readers) in place. It states
      which attachments remain memoryless at the shipped configuration.
- [ ] D7.3 Each mechanism from M1–M5 is recorded as a measured difference or as no measurable effect. No tripwire is
      open, or each open one has an assigned fix.

Work:
- [ ] Instruments template with the Performance Limiters counter set, saved under `tmp/g3t-traffic/` (→D7.1).
- [ ] Route runs, a one-mechanism-off ablation per mechanism, and rows (→D7.1, D7.3).
- [ ] Eligibility rerun (→D7.2).
- [ ] Evidence record `docs/evidence/render/G3T-PAIRED-*.md`; maturity rows go to the ledger owner (→D7.1–D7.3).

## Sizing

| Milestone | Days |
|---|---|
| M0 | 0.5 |
| M1–M6 | about 1 each |
| M7 | 1 |
| Reviews and Thrum Core seam turnaround | 1–2 |

That is about 8.5 working days of build plus 1–2 of review, so **8–10 days** for G3t. M3, M4 and M5 can overlap if
separate agents own disjoint files (see the parallel table), which brings the elapsed time to about 6–7 days. The
largest uncertainty is M2: it touches the frame runtime and both shells' surface paths.

## Log

- 2026-11-07 Plan drafted for Thrum Core's G3t/G4 commission.
