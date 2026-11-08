# Apple tile recipe and backend dataflow (G3t)

[Boundary decision](../decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../decisions/HARDWARE-NATIVE-ENGINE.md) ·
[Executing render graph](../decisions/G3-EXECUTING-RENDER-GRAPH.md) ·
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Milestones](APPLE-TILE-PLAN.md) · [Presentation (G4)](APPLE-PRESENTATION.md)

Revision 3 for Thrum Core review, 7 November 2026. It answers the three independent reviews of revision 1
(`86ec3b6b`) and the re-reviews of revision 2 (`c507ad3d`) recorded in the plan. Interface and path names below are
proposals.

## Purpose and scope

On Apple GPUs the V1 game should draw the same valley image as the portable recipe, using the hardware the way a
tile-based GPU wants:
- attachments that die inside a pass never leave tile memory;
- no full-screen intermediate exists only to copy into the next pass;
- per-frame data reaches the GPU without a staging copy;
- the forward pass does no work the GPU can do better elsewhere.

G3t delivers that as one registry-selected configuration of the one graph portable already runs. It also defines the
prepared native-feature handoff that G4's MetalFX work consumes.

Consumers are the V1 game on macOS (M3 Max) and in the iOS shell, G4 presentation, and G3c culling, which shares
depth, instance data and indirect draws. Portable stays the running fallback and image oracle, and Chrome always runs
portable.

Commissioned:
- the recipe and registry fields;
- graph-derived memoryless and transient attachments;
- a merged scene/present graph with direct output;
- shared transient targets;
- mapped per-slot uploads;
- compact instances;
- GPU light binning;
- mirrored ranges;
- the native-segment handoff and its prepared-feature records;
- proof tooling, and traffic and scaling evidence.

Deferred:
- the complete native recorder, kept as the later performance candidate (boundary decision);
- tile shaders, imageblocks and programmable blending, which wgpu cannot express;
- Metal 4 command objects;
- Android tile;
- quantised instance rows below 80 B (only if M12 shows instance bandwidth matters);
- the cooked-placement 80 B instance layout in `thrum-asset-gpu`, which keeps its own path.

## Cues

The player sees no difference between recipes. These are the cues:
- **Same image.** Along the fixed valley route, the reference image matches portable within tolerance. That covers
  the sky, Sun disk, stars, shadows, local lights, mirrored props and the grid fixture. A recipe that drops a pass or
  reads an undefined attachment fails tolerance or validation.
- **No hitch.** The recipe is chosen before readiness. A switch between atmosphere day and star variants at dusk, or
  between capture and normal frames, selects allocations prepared at the generation build and compiles nothing.
- **Visible scaling.** From 4 to 64 local lights, and with mirrored instances present, the image stays correct and
  cost follows the cost model below. A dark frame from light-list overflow, or back faces on mirrored props, is a
  defect.

## Techniques and evidence

Research notes: `tmp/g3t-g4-plans/{code-interfaces,primary-sources,docs-survey}.md` (the code at `5045cb7d`, Apple
SDK 27.2 headers and documentation, wgpu 30.0.1 source, and the governing documents). Review reports:
`tmp/g3t-g4-plans/reviews/`.

**Three tiers, chosen in order.** Each mechanism uses the lowest tier that can express it.

1. *Public wgpu, recipe-selected.*
   - Memoryless: wgpu 30 maps `TextureUsages::TRANSIENT_ATTACHMENT` to `MTLStorageModeMemoryless`
     (`AdapterInfo::transient_saves_memory`). It requires usage exactly `RENDER_ATTACHMENT | TRANSIENT_ATTACHMENT`, 2D,
     one mip and one layer, a Clear or DontCare load, and a Discard store.
   - The rest is public wgpu too: compute light binning, compact instances, mirrored ranges, the merged graph and
     shared targets.
2. *Isolated `apple_tile` backend objects, outside any encoder.* The boundary decision assigns rings to this backend.
   The Apple upload ring is a persistently mapped `MTLStorageModeShared` buffer, imported once per generation with
   `Device::create_buffer_from_hal`. The CPU writes through its contents pointer only after the slot's previous
   submission completes. This is not an encoder splice and adds no queue.
3. *Native segment (a proposed boundary amendment, seam 1; the current boundary remains in effect until Thrum Core
   rules).* Some work has no wgpu path, such as a MetalFX scaler.
   - The executor closes the current wgpu encoder and opens a dedicated wgpu `CommandEncoder` used **only** through
     `as_hal_mut`. In it the provider records into wgpu-hal's `raw_command_buffer()`. The executor then resumes a fresh
     wgpu encoder.
   - All encoders go to the one `queue.submit` in graph order.
   - wgpu-core permits a whole-Raw encoder (`record_as_hal_mut` sets `EncodingApi::Raw`; `finish` accepts it) and
     panics on mixing. The design never mixes, never writes the drawable natively and never uses a second queue.
   - MetalFX's classic scalers encode into an `MTLCommandBuffer` they do not own, so they fit this shape.
   - *Producer to segment.* Each wgpu `CommandEncoder` becomes one `MTLCommandBuffer`, and the one submit commits them
     in order on one queue. The producing encoder E1 is therefore committed before the segment's buffer N. wgpu creates
     Metal resources hazard-tracked by default, so Metal orders N's reads after E1's writes to the same resource.
     wgpu-hal's Metal texture transitions are no-ops, so no barrier is emitted or needed.
   - *Segment to wgpu.* The resuming encoder E2 is committed after N, and tracking orders E2's reads of native-written
     textures after N's writes. Command buffers retain the resources they reference only
     because wgpu-hal 30's `retain_command_buffer_references` defaults to true; the lifetime guarantee is generation
     ownership, proven by G3t D10.3, not that default.
   - *What wgpu-core cannot observe.* Native writes (its initialization state never sees them), native-only resource
     lifetimes, errors raised inside N, and the provider's internal history, which MetalFX advances at encode. Each has
     an answer below: native-written textures are cleared once and never aliased or discarded; the generation owns
     their lifetime; the segment registers a completed handler on N's raw `MTLCommandBuffer` that reads its `status` and `error` and
     raises device loss; and the executor resets the provider after any
     frame that was encoded but not executed.
   - Thrum Core reviews this exact seam, the transitions above and the G3t M10 stand-in, before any experiment starts.
4. *Complete native recorder.* This remains the later candidate.

**If seam 1 is refused:** MetalFX is unreachable until the complete recorder exists. The Apple recipe then ships
TAAU, and BUILD-PLAN's V1 "MetalFX" item becomes an owner decision. G3t M10–M11 drop out, and G4 M8 moves to after
the recorder.

**Alternatives rejected.**
- An in-encoder raw splice: wgpu forbids it, and `as_hal_mut` cannot flush pending wgpu commands.
- Raw drawable writes: wgpu does not mark the surface initialized.
- A Metal 4 queue for MetalFX 4: it is a second queue and needs event synchronisation.
- `map_async` rings as the Apple path: wgpu requires an unmap before each submit, a poll point, and a boxed callback
  per map per frame. This stays the fallback only if the raw import fails at M5.
- A literal "direct forward output" that writes display-encoded colour from forward into the drawable while HDR stays
  in tile: late acquisition forbids touching the drawable before scene encoding completes, and reconstruction needs
  stored HDR.
- wgpu `PipelineCache`: it is a stub on Metal in 30.0.1.
- Tile shaders: wgpu has no feature for them.

Shipped tile engines follow the same discipline: load only what a pass needs, store only what later passes read, and
keep single-pass depth and MSAA memoryless (Apple TBDR guidance; WWDC20 10603).

**The honest limit on memoryless, from today's graph.**

| Attachment | Day | Night (star variant) | After G4 | After G3c Hi-Z |
|---|---|---|---|---|
| Forward depth | Discarded and render-attachment-only, so eligible | Stored (star pass reads it) | Stored (reconstruction) | Stored |
| Motion | Stored only because `external_side_effect` plus `store: true` forces it (`graph.rs:663-671`); nothing reads it, so a dead store of about 8.3 MB/frame at 1080p | Same | Stored and read | Stored and read |
| Sun shadow cascades | Stored, sampled by forward | Same | Same | Same |

Memoryless therefore pays at V1's shipped configuration only for day-variant depth until G4 lands, and afterwards
only for views and passes with no later reader. The tile win must come from:
- load/store discipline;
- removing the 8-bit output intermediate (direct output);
- not uploading through staging copies;
- smaller instance rows.

Shared transients halve transient *memory footprint*. They are not a traffic win. The plan measures each mechanism and
assumes none. The eligibility rule treats an explicit `store` flag as ineligible, so the motion dead store stays
visible in the report until G4 gives motion a reader.

**Mechanisms per BUILD-PLAN item.**

| Item | Mechanism | Tier | Check |
|---|---|---|---|
| Memoryless/transient | Eligibility per *prepared variant* (day, star, capture). Eligible attachments get their own memoryless allocation, which has no backing memory. These are never aliased with stored ones, and every variant's allocations are created at the generation build. | 1 | Scripted storage-mode log through `Texture::as_hal`; same image |
| Direct output | One graph per generation with the surface imported late. Execution is split into a pre-acquire phase and a post-acquire phase. Tone and display encode move into the present pass, so the `Rgba8Unorm` output disappears. | 1 | Graph-declared bytes fall by the output write and read; same image |
| Shared targets | One transient set per generation instead of per slot. One queue with tracked hazards orders slot N+1 after slot N. | 1 | Transient footprint halves; held-submission test |
| Mapped per-slot upload rings | Per-slot frame buffers and bind groups on both recipes. On Apple they are backed by persistently mapped shared buffers. | 1 + 2 | Zero `write_buffer` for the per-frame classes on Apple tile; upload CPU recorded |
| Compact instances | 192 B → 80 B: current 3×4 affine (48 B), previous-simulation-sample translation (12 B), ids (16 B), padding (4 B). Today's shaders interpolate only the previous translation by `presentation_alpha` (`synthetic.wgsl:124` and three others), so nothing read is lost. The normal matrix is derived in the shader. Both recipes. | 1 | Same image; instance bytes ×0.42 |
| GPU light binning | Deterministic GPU build of the full 32-per-cluster CSR: one invocation per cluster scans lights in index order, keeps the lowest-index 32 and counts overflow | 1 | Per-cluster equality with the CPU oracle |
| Mirrored ranges | G3c's visibility compute writes two indirect argument records, normal and mirrored, drawn with opposite cull faces | 1 | Same image; +1 indirect draw |
| Prepared native features | `NativeFeatures` record and native-segment handoff (below) | 3 | Stateful stand-in inside the one submit |

**What would change these choices:**
- A measured tile-shader win on the valley would justify asking for tier 4.
- If G4 D9.1 shows pipeline state creation is negligible even cold, the archive layer still ships (V1 requires it), but
  its proof weighs less than the metallib layer's.
- A wgpu Metal 4 path would move MetalFX 4 into tier 3.

## The system as it runs

**State and owners.**
- *Platform (`thrum-app`, iOS shell).* Owns the adapter, device and generation, the surface and display link, and
  every per-display fact: headroom, refresh and backing scale. Display facts are not adapter facts and live in G4's
  display record, not in the registry.
- *Registry (`thrum-render::capabilities`).* Built once per adapter generation:
  - `recipe: Recipe::{Portable, AppleTile}`, from the backend, `transient_saves_memory`, and unified memory read
    through `Adapter::as_hal` (`hasUnifiedMemory`);
  - `native: Option<NativeFeatures>` on Apple: memoryless, Metal family, MetalFX support and scale range, binary
    archives, OS/SDK build;
  - a digest that evidence rows quote.
- *Proof counters (`thrum-render::runtime`).* Pipeline, texture and native-object creations, each tagged with its
  thread and phase (build, transition or play). G3t M0 builds these; G3t and G4 checks both read them.
  - **Status: an opt-in diagnostic, landed for D0.3.** It lives behind the `creation-counters` renderer feature,
    which is off by default. In default builds the seam is a direct wgpu call, with no ledger, report or
    submit-path lock.
  - **Pipeline events (the guard).**
    - Each device generation has one creation authority, a `DeviceSite`. `DeviceCapabilities::capture` mints it once,
      with its own ledger, at the selected platform generation. Cloning the capabilities or rebuilding visuals shares
      that site. Re-capturing a raw `wgpu::Device` (`capture_device`, `DeviceSite::untracked`) yields an untracked
      site that reaches no ledger; it is neither an identity check nor a second ledger for the generation.
    - Every production pipeline is created through a site, `device.render(&desc)` or `.compute(&desc)`, which
      records one event on that site's own ledger after creating. Ledgers are never looked up by device identity,
      so devices of different `wgpu::Instance`s that compare equal never share events.
    - `SceneSurface::device` is that site, so contributions build on the host's generation. The cooked material
      compile thread opens `"material-reload"` or `"material-restore"` around its compile; the host opens
      `"contribution"` at attach and `"publish-device"` when it rebuilds a replacement device.
    - An event goes to the innermost build or transition region that the creating thread has open on that ledger. A
      detached preparation handle opens its region on the worker thread, so a worker's build is charged to its own
      region, and play keeps running meanwhile.
    - An event with no open region is *unscoped*. Before readiness it counts as build; after readiness it is a play
      violation, recorded with its thread and frame count.
    - Events count creations, so a pipeline created and dropped between frames is still caught.
    - Coverage tests (`runtime/tests/creation_sites.rs`, run in default builds) fail on any raw
      `create_render_pipeline` or `create_compute_pipeline` call in the production sources of `thrum-render`
      (outside the seam) or `thrum-asset-gpu`. Raw-device escapes are an exact inventory: one in `capabilities.rs`
      (`capture_device`) and two in `synthetic.rs` (the public raw-device preparation entry points); the cooked crate
      has none.
  - **Coverage limits.**
    - A pipeline built on an untracked site is not counted. Production has only the inventoried escapes; the shell's
      raw device fields and the V1 `FrameHost::new_with_setup` device parameter are separate G6 seams.
    - Native objects have no producer yet, so they report *absent*.
  - **Live-object deltas (a diagnostic, not the guard).**
    - It reads wgpu's per-device live-object counters at each region boundary, detach boundary and retirement. A
      window's net change goes to its innermost open region; an unscoped window counts as build before readiness and
      play after. The frame path is one relaxed atomic add, with no lock or counter read.
    - A window names its thread only when it was serialized: no detached handle alive, and the closing thread and
      every open region's thread were the owner. Every other window is *unattributed*, including every visual
      preparation.
    - Counts are net per window, so a create-and-drop or a deferred destruction can hide a creation. A backend with no
      counters (WebGPU) reports *uninstrumented*, not zero.
- *Graph (`thrum-render::graph`).* Compiled once per generation and variant, never per frame. It adds:
  - memoryless eligibility per variant;
  - a `PassBackend::{Wgpu, NativeSegment(ProviderId)}` declaration;
  - a surface import whose extent and view are bound at execute time.

  Execution has two phases. Phase 1 holds every pass that does not depend on the surface. Phase 2 holds the surface
  pass and any timing or readback after it. The compiler refuses other work after the surface pass. Native segments
  declare reads and writes like any pass, so hazards and lifetimes derive unchanged. A native segment cannot import
  the surface. Native-written textures are generation-persistent: never aliased, never stored with Discard, and
  retired with their generation after GPU completion.
- *Executor and runtime (`thrum-render::runtime`).* The one owner of queue, submission, completion and retirement.
  - A frame is an ordered encoder list (wgpu and native), submitted once.
  - The existing presentation-drop contract is preserved until the runtime seam is reviewed. On portable, if
    acquisition fails, views drop and pre-acquire work may still be cancelled; the existing
    `unavailable_surface_drops_only_views_and_resize_reuses_preparation` test stays as it is. Temporal history commits
    only when a frame carrying an accepted sample is successfully submitted (`history.rs`), so a cancelled frame
    leaves history at the last submitted sample.
  - A frame containing a native segment whose provider keeps internal history is the exception, because that history
    advanced at encode. The proposed runtime seam (seam 3) submits such a frame without its present phase. If Thrum
    Core refuses it, the frame is discarded and the provider resets on its next encode.
  - On iOS, background GPU submission is forbidden, so an encoded frame found after suspension is dropped and every
    history reset, native or not. This is not device loss.
  - Each outcome (submitted without present, discarded with a reset, iOS background refusal) is counted and observable
    in the proof (G3t M10–M11, G4 M8).
  - `FrameSample` timing becomes a list keyed by graph pass names instead of the fixed 11-slot array.
- *`apple_tile` backend (`thrum-render/src/apple_tile/`, `cfg(target_vendor = "apple")`).* Owns:
  - the shared-storage rings;
  - the segment trampoline (`as_hal_mut` → `raw_command_buffer`; `Texture::as_hal` → `raw_handle`);
  - segment timing, read at completion from each segment command buffer's `GPUStartTime`/`GPUEndTime`;
  - native provider objects keyed by generation, extents, formats and scale range.

  It owns no queue, surface or history.

**Per-slot frame buffers and the Apple ring.** Today the instance table, directional, local-light, cluster and
cluster-bounds buffers are single buffers that both slots share, ordered by `queue.write_buffer`.
- Both recipes move these to per-slot buffers, with per-slot bind groups prepared at the generation build for
  forward, shadow, visibility, history and atmosphere consumers.
- Instance dirty rows go to the slot being written as the union of the rows dirtied since that slot's last write:
  this frame's and the previous frame's.
- Portable fills the slot buffers with `write_buffer`.
- Apple tile writes the same bytes straight into the shared buffer once the slot's `render_done` is set. A slot still
  busy at encode time takes `write_buffer` for that frame, and the fallback is counted.
- Memory cost is one extra copy per buffer: at 10k instances × 80 B, about 0.8 MB.

**Prepared native-feature handoff (consumed by G4 M8).**
- *Generation.* Providers are created at the generation build for one device generation and recipe generation, with
  declared input and output formats, required texture usages, extents and scale range. The build runs before
  readiness, or during a transition off the render thread (G4 owns transitions).
- *Lifetime.* A provider object and every native-touched texture live until their generation retires after GPU
  completion. wgpu-core does not track textures touched only by raw encoding, so the generation, not the command
  buffer, is what keeps them alive until retirement.
- *Encode.* `encode(raw_cmd_buf, inputs, outputs, params)`, only inside a native segment, from graph textures to graph
  textures, never the drawable.
- *State and reset.* A provider may keep internal history. It must reset when told, and the executor tells it after
  any generation change or any frame whose submission failed.
- *Initialization.* A native-written texture is cleared once through wgpu at creation, and is never aliased or
  discarded afterwards, so wgpu-core's lazy initialization never overwrites native output.
- *Completion.* The provider completes with the slot's submission. Raw work cannot publish history or retire resources
  before that.
- *Acquisition.* Unchanged: phase 2 alone touches the drawable.
- *Fallback.* Each provider names a portable twin prepared at the same build. Unsupported hardware, creation failure or
  a forced rejection selects the twin without compiling. A command-buffer error takes the device-loss path.
- *Timing.* Each segment reports GPU start and end from its own command buffer, merged into `FrameSample` under the
  segment's pass name.

**Archives.** V1 has no tier-3 pipeline that can use an `MTLBinaryArchive`: classic MetalFX scalers take no archive,
and wgpu 30 cannot build pipelines from native objects or from a Metal cache. Seam 5 closes that gap for
wgpu pipelines in two layers, measured first. Offline metallibs through wgpu 30's passthrough remove naga and the runtime
MSL compile. A Metal `PIPELINE_CACHE` over `MTLBinaryArchive`, carried as a pinned wgpu patch, removes cold
pipeline-state compiles. Contributing that cache upstream (gfx-rs/wgpu#8526) is a later item and does not gate V1. V1's archive item stays as written.

**A normal frame (Apple tile, after G3t).** The steps run in order:
1. Encode stage on the render owner. The slot's shared buffers are written once its previous use has completed.
2. Phase 1 encoders:
   - atmosphere compute;
   - optional contribution;
   - Sun shadow cascades (stored);
   - air shadow;
   - GPU binning;
   - camera visibility, which writes normal and mirrored indirect records;
   - forward opaque plus sky: depth memoryless in the day variant without readers, stored otherwise;
   - instance history;
   - under G4, reconstruction (native segment or TAAU).
3. Late acquire.
4. Phase 2: the present pass encodes tone and display into the drawable.
5. One submit; completion retires the slot.

If acquisition fails, the existing drop contract applies; a frame carrying a stateful native segment follows seam 3.

Valley baseline at 1080p offscreen on this M3 Max: GPU pass sum p50 3.86 ms (forward and sky 1.94, atmosphere 1.24,
shadow 0.50); upload p50 628 µs; encode p50 1.63 ms. These are host-shared baselines, not budgets.

**Cost account and models (nominal, 1920×1080).**

*Attachments, before compression.*

| Attachment | Bytes |
|---|---|
| Depth D32 | 8.3 MB |
| Scene `Rgba16Float` | 16.6 MB |
| Motion | 8.3 MB |
| `Rgba8` output, removed | 8.3 MB written + 8.3 MB read |

Memoryless day depth saves an 8.3 MB store. Shared transients save about 33 MB of footprint.

*Instances.*

| Rows | 192 B rows | 80 B rows |
|---|---|---|
| 10k | 1.92 MB | 0.80 MB |
| Valley (1,500/frame) | 288 KB | 120 KB |

*Lights.*
- The GPU build tests 9,216 clusters × L lights, so cost is linear in L: about 0.6 M tests at L = 64. It is expected
  to be well under 0.1 ms on M3 Max, a model to be measured.
- Forward shading is linear in the mean lights per covered cluster, capped at 32.
- CPU frame time should stay within noise from 4 to 64 lights on the GPU build. On the CPU build it is linear in L.

*Mirrors.* One extra indirect draw and no change in shaded pixels.

*Traffic.* Graph-declared bytes are recorded per attachment. Measured device bandwidth needs a counter capture that
M0 must first prove possible: the commission's probe found only `GPUTimestamp` in `MTLDevice.counterSets`, and only
`RT Unit Active` from an Instruments run with the Performance Limiters counter set. If no byte counter can be
captured, rows record graph-declared bytes labelled *unmeasured*, and GPU time is the measured signal.

## Risks

- **Memoryless buys little at the shipped configuration.** The early sign is the per-variant table above.
  - The plausible wrong substitute is to claim a tile win from a flag on an attachment nobody uses at the shipped
    setting.
  - Guard: report eligibility per variant at the shipped configuration.
- **Native segment and wgpu tracking disagree.**
  - Lazy initialization may clear native output.
  - A native-only texture may be freed in flight.
  - Guard: the native texture rules above, plus a *stateful* stand-in that writes through storage, proven before
    MetalFX.
- **The raw ring breaks wgpu's assumptions.** `create_buffer_from_hal` import, lazy-init state and usage flags are
  unproven.
  - Early sign: validation errors or zero-filled rows.
  - Fallback: `map_async` rings, with their costs recorded.
- **Shared transients race across slots.** Early sign: flicker under a stall. Guard: a held-submission test.
- **Variant allocations multiply.** Prepared variants: atmosphere day/star × capture × reconstructor twin.
  - Memoryless allocations cost no memory. Stored allocations are shared across variants by descriptor.
  - Guard: report allocation bytes per variant at the generation build.
- **The merged graph's split execution hides ordering bugs.** Guard: compile-time refusal of non-surface work after the
  surface pass; the existing frame-token tests.

## Performance

Following the 6 November decision, there are no admission gates. The tripwires are:
- a visible hitch;
- about a 25% comparable regression;
- scaling that departs from the models above.

Every row names the recipe and registry digest. Rows are paired Apple tile and portable runs on the same M3 Max over
the M0 route, in its fixed-step mode, and record:
- GPU pass and segment times;
- display-link present intervals;
- upload CPU time;
- graph-declared bytes;
- measured bandwidth where M0 proved a capture;
- image tolerance on the reference image.

Mac rows standing in for the iPhone are labelled proxy. The iPhone simulator is function only. Physical iPhone timing
is owed. Whole-frame present intervals on the Mac also depend on G3b's open live Mac presentation row.

## Seams for Thrum Core (proposals)

1. **Boundary amendment (proposed).** Admit the tier-3 *whole-encoder native segment* inside the one submission, as
   defined above, including its producer and consumer transitions and what wgpu-core cannot observe. The current
   boundary remains in effect; Thrum Core reviews this exact seam before an experiment starts. The decision currently excludes a "partial raw splice". If this is refused, see *If seam 1 is refused*.
2. **Registry.** `Recipe`, `NativeFeatures` and the digest in `DeviceCapabilities`. Per-display facts go to the
   platform's display record, not the registry.
3. **Graph.**
   - Per-variant memoryless eligibility.
   - `PassBackend::NativeSegment`.
   - Native-written textures as generation-persistent.
   - One graph per generation with split execution and an execute-time surface binding.
   - The `FrameSample` timing schema keyed by pass name.
   - Submit-without-present only for frames carrying a stateful native segment; the portable presentation-drop
     contract is unchanged.
4. **Per-slot frame buffers.** For both recipes, with per-slot bind groups in forward, shadow, visibility, history and
   atmosphere (atmosphere and G3c owners to review).
5. **Pipeline preparation and archives (V1's "binary archives" item stays as written).** Built when G4 reaches M10,
   then proved; details are designed when the milestone starts. Today wgpu 30 translates WGSL with naga and compiles
   MSL at every pipeline creation on every launch, and its Metal `PipelineCache` is a stub. Four distinct pieces:
   - *Offline metallibs* loaded through wgpu 30's `PASSTHROUGH_SHADERS`, which skip naga and the runtime MSL compile.
     Passthrough loses wgpu-hal's per-pipeline reflection, so a pinned wgpu patch carries reflection from the cook.
   - *Parallel pre-warm*: closure-time preparation creates pipelines across worker threads.
   - *Local archive qualification*: a Metal `PIPELINE_CACHE` over `MTLBinaryArchive` in a pinned wgpu patch, with a
     shipped archive proved by a strict fail-on-miss witness. This stays within V1.
   - *Later upstream contribution* under gfx-rs/wgpu#8526, not V1-gating, only if needed and only as a change we can
     stand behind. wgpu's CONTRIBUTING allows LLM-written code but not fully agentic PRs, so the owner writes every
     public description and comment, and nothing is posted without the owner's OK on each post.
6. **Compact instance ABI.** 80 B rows for both recipes: `abi.rs`, `synthetic.wgsl`, `shadow.wgsl`,
   `history_gpu.wgsl`, `visibility.wgsl`, `thrum-app/src/sky.rs` and `tests/contracts.rs`. Reviewed by G3c and the
   G3b interpolation owner.
7. **Mirrored indirect records.** G3c's visibility compute emits normal and mirrored indirect argument records.
8. **Route.** A fixed valley route with motion and a fixed-step mode (fixed dt, fixed presentation alpha), shared with
   G4 and G7. The G7 presets are paused today.
9. **Dependencies.** Register `wgpu-hal` (Metal), `objc2-metal` at wgpu-hal's version (0.3.2) and `objc2-metal-fx` for
   `thrum-render` in `.config/architecture.toml` when M5 and M10 need them.
10. **Stale text.** `decisions/G3-EXECUTING-RENDER-GRAPH.md:23` still requires a ≥10% win over portable, which the 6
    November performance decision retired.

## Open questions

- Can any device byte counter be captured on this M3 Max: an Xcode GPU frame capture with memory counters, or
  another explicitly selected counter set? Settled by M0. If not, traffic stays graph-declared and labelled
  unmeasured.
- Does the raw shared-storage ring import cleanly, and does it beat `write_buffer` in upload CPU time? Settled by M5.
- Is the native segment sound with wgpu-core tracking for a stateful, storage-writing provider? Settled by M10.
