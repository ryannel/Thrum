# Apple tile recipe and backend dataflow (G3t)

[Boundary decision](../decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../decisions/HARDWARE-NATIVE-ENGINE.md) ·
[Executing render graph](../decisions/G3-EXECUTING-RENDER-GRAPH.md) ·
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Milestones](APPLE-TILE-PLAN.md) · [Presentation (G4)](APPLE-PRESENTATION.md)

Draft for Thrum Core review, 7 November 2026. Interface and path names below are proposals.

## Purpose and scope

On Apple GPUs the V1 game should draw the same valley image as the portable recipe, using the hardware the way a
tile-based GPU wants. Attachments that die inside a pass never leave tile memory. No full-screen intermediate exists
only to copy into the next pass. Per-frame data reaches the GPU without a staging copy. The forward pass does no work
that the GPU can do better elsewhere. G3t delivers that recipe as one registry-selected configuration of the one
graph that portable already runs. It also defines the prepared native-feature handoff that G4's MetalFX, EDR and
archive work consumes.

Consumers are the V1 game on macOS (M3 Max) and the iOS shell, the G4 presentation module, and G3c culling, which
shares depth and instance data. Portable stays the running fallback and image oracle, and Chrome always runs
portable.

Commissioned: the recipe and registry fields, graph-derived memoryless and transient attachments, a merged
scene/present graph with direct output, shared transient targets, mapped per-slot upload rings, compact instances,
GPU light binning, mirrored ranges, the native-segment handoff and its prepared-feature records, and traffic and
scaling evidence. Deferred: the complete native recorder (kept as the later performance candidate, as the boundary
decision records); tile shaders, imageblocks and programmable blending, which wgpu cannot express; Metal 4 command
objects; Android tile.

## Cues

The player sees no difference between recipes. These are the cues:

- **Same image.** The valley at each showcase stop, and along the fixed camera route, matches portable within the
  declared tolerance. That covers the sky, Sun disk, stars, shadows, local lights and the grid fixture. A recipe
  that silently drops a pass, or reads an undefined attachment, shows as a tolerance failure or a validation error.
- **No hitch.** Switching recipe happens before readiness, never during play. Recovery (surface replacement, device
  loss, resize) re-closes the graph without compiling a pipeline during play.
- **Visible scaling.** With local lights from 4 to 64, and with mirrored instances present, the image stays correct
  and cost grows as the cost account predicts. A dark frame from light-list overflow, or back faces showing through
  on mirrored props, is a defect.

## Techniques and evidence

Research notes: `tmp/g3t-g4-plans/{code-interfaces,primary-sources,docs-survey}.md` (the code at `5045cb7d`, Apple
and wgpu 30.0.1 primary sources, and the governing documents).

**Three tiers, chosen in order.** Each mechanism uses the lowest tier that can express it.

1. *Public wgpu, recipe-selected.* wgpu 30 maps `TextureUsages::TRANSIENT_ATTACHMENT` to
   `MTLStorageModeMemoryless` when the adapter supports it (`AdapterInfo::transient_saves_memory`). It requires usage
   exactly `RENDER_ATTACHMENT | TRANSIENT_ATTACHMENT`, one mip and layer, a Clear or DontCare load, and a Discard
   store. That is exactly the graph's existing last-use derivation, so the graph owns eligibility and the recipe
   owns enablement. Mapped buffers (`MAPPABLE_PRIMARY_BUFFERS`, on unified memory), compute light binning, compact
   instances, mirrored ranges, the merged graph and shared targets are all public wgpu. The Apple recipe can enable
   them where portable cannot, or where portable's choice differs.
2. *Native segment (proposed boundary change, see Seams).* Some work has no wgpu path: a MetalFX scaler, a
   native pipeline from an archive, or an encoder that only Metal can record. For such work, the executor closes the
   current wgpu encoder and opens a dedicated wgpu `CommandEncoder` used **only** through `as_hal_mut`. In that
   encoder the provider records into wgpu-hal's `raw_command_buffer()`, and the executor then resumes a fresh wgpu
   encoder. All of these encoders go into the one `queue.submit` in graph order. The design never mixes raw and wgpu
   recording inside one encoder, which wgpu-core's `EncodingApi` rejects. It never writes the drawable natively and
   never uses a second queue. MetalFX's classic scalers encode into an `MTLCommandBuffer` they do not own, so they
   fit this shape.
3. *Complete native recorder.* Every pass is recorded natively. This remains the later performance candidate, with
   the requirements the boundary decision lists.

**Alternatives rejected.**
- An in-encoder raw splice: wgpu forbids it, and `as_hal_mut` cannot flush pending wgpu commands.
- Raw drawable writes: they do not mark wgpu's surface initialized.
- A Metal 4 queue for MetalFX 4: it is a second queue and needs event synchronisation.
- wgpu `PipelineCache`: it is a no-op stub on Metal in 30.0.1.
- Tile shaders: wgpu has no feature for them.

Shipped tile engines use the same discipline: load only what the pass needs, store only what later passes read, and
keep depth and MSAA memoryless (Apple TBDR guidance and WWDC20 *Optimize Metal apps and games with GPU counters*).

**The honest limit on memoryless.** At the shipped V1 configuration, depth is read after the forward pass by G4's
reconstruction and by G3c's Hi-Z pyramid, and motion and scene colour are read by reconstruction. These must be
stored. Memoryless therefore applies to:
- depth on views without reconstruction or occlusion, such as reflection and capture views;
- depth and colour on a stencil-only or MSAA pass;
- the intermediates that a later merge keeps in tile.

The V1 tile win is mainly:
- load/store discipline: dontCare loads and discarded dead stores, which the graph already derives;
- removing the 8-bit output intermediate through direct output;
- shared transients, which halve per-slot transient memory;
- upload and instance bandwidth.

The plan measures each of these. None is assumed.

**Mechanisms per BUILD-PLAN item.**

| Item | Mechanism | Tier | Check |
|---|---|---|---|
| Memoryless/transient attachments | The graph marks a texture eligible when every access lies inside one render pass, with Clear/DontCare load and Discard store; the recipe adds `TRANSIENT_ATTACHMENT` | 1 | Capture shows memoryless storage; same image |
| Direct forward output | The scene and present plans merge into one graph. Tone encode moves into the present pass, which reads scene HDR (or G4's reconstructed output) and writes the drawable after late acquisition, so the `Rgba8Unorm` output target disappears. Capture readback stays a graph branch. | 1 | Graph-declared traffic falls by one output write and read; same image |
| Shared targets | Transient sets live per recipe generation, not per frame slot. One queue with tracked hazards orders slot N+1 after slot N's use. | 1 | Transient bytes halve; no validation error; same image |
| Mapped per-slot upload rings | Each frame slot owns a `MAP_WRITE` ring (storage-usable via `MAPPABLE_PRIMARY_BUFFERS` on unified memory). It is mapped after the slot's previous submission completes (`render_done`), written, and unmapped before submit. Portable keeps `queue.write_buffer`. | 1 | Zero steady `write_buffer` calls on Apple tile; upload CPU time recorded |
| Compact instances | The ABI moves from 192 B to 64 B rows: a 3×4 affine (48 B) plus ids (16 B). The previous transform comes from the existing GPU history buffer; the normal transform is derived in the shader. Both recipes share the ABI. | 1 | Same image; instance bytes ÷3 |
| GPU light binning | `cluster_gpu` extends from 1–4 lights to the full 32-per-cluster CSR. Overflow is counted. The CPU table stays as the oracle. | 1 | Equal tables on both paths; 4→64 light scaling recorded |
| Mirrored ranges | Rows are partitioned by determinant sign into two ranges, drawn with back-face and front-face culling, instead of switching every row to cull-none | 1 | Same image with mirrored props; draw count +1, not cull-none for all |
| Prepared native features, archives | `NativeFeatures` registry record and native-segment provider handoff (below) | 2 | The G4 provider encodes inside the one submit |

**What would change these choices:**
- A measured tile win from tile shaders on the valley would justify asking for tier 3.
- A wgpu upstream Metal `PipelineCache` would replace the archive handoff.
- A future Metal 4 path in wgpu would move MetalFX 4 into tier 2.

## The system as it runs

**State and owners.**
- *Platform (`thrum-app`, iOS shell).* Owns the adapter and device generation, the surface and display link. It
  builds `DeviceCapabilities` once per generation.
- *Registry (`thrum-render::capabilities`, proposed fields).* Adds:
  - `recipe: Recipe::{Portable, AppleTile}`, selected before closure from the backend, `transient_saves_memory`
    and the unified-memory family;
  - `native: Option<NativeFeatures>` on Apple: memoryless support, Metal family, MetalFX spatial and temporal
    support with scale range, binary-archive support, OS/SDK build, and potential EDR headroom when known;
  - a digest of the record.

  Passes never probe privately.
- *Graph (`thrum-render::graph`).* Adds two things to its compile:
  - memoryless eligibility per transient;
  - a `PassBackend::{Wgpu, NativeSegment(ProviderId)}` declaration.

  Native segments declare reads and writes like any pass, so hazards, lifetimes and stores derive unchanged. A
  native segment cannot import the surface.
- *Executor and runtime (`thrum-render::runtime`).* Still the one owner of queue, submission, completion and
  retirement. It now records a frame as an ordered list of encoders, wgpu or native, and submits it once.
- *`apple_tile` backend (proposed `thrum-render/src/apple_tile/`, `cfg(target_vendor = "apple")`).* Owns:
  - upload-ring policy;
  - the native-segment trampoline (`as_hal_mut`, then `raw_command_buffer`, then `Texture::as_hal` →
    `raw_handle`);
  - native provider objects keyed by device generation, extents, formats and scale range.

  It owns no queue, surface or history.

**The prepared native-feature handoff (consumed by G4).**
- *Generation.* Providers are created at graph closure, for one device generation and recipe generation, before
  readiness. Examples are a MetalFX temporal scaler with `inputContentMinScale/MaxScale` covering the governor's
  range.
- *Lifetime.* A provider object lives until its generation retires after GPU completion. Resize, device loss and
  surface replacement close a new generation and prepare new providers before switching.
- *Encode.* `encode(raw_cmd_buf, inputs: &[NativeTextureRef], outputs, params)`, called only inside a native
  segment. Inputs and outputs are graph textures, never the drawable.
- *Initialization.* The graph clears a native-written texture once through wgpu when it is created, so wgpu-core's
  lazy initialization never overwrites native output.
- *Completion.* The provider completes with the slot's submission (`on_submitted_work_done`). Raw work cannot publish
  history or retire resources before that, as the boundary decision requires.
- *Acquisition.* Unchanged. The drawable is acquired after scene and native work are encoded, and only the wgpu
  present pass touches it.
- *Fallback.* Each native provider names a portable twin prepared at the same closure. Unsupported hardware,
  creation failure or a forced rejection selects the twin without compiling. A command-buffer error during play takes
  the device-loss path.

*Archives.* A native pipeline built through tier 2 may come from an `MTLBinaryArchive` keyed by adapter, driver,
OS/SDK, recipe, shader and ABI revision. A miss compiles at closure, never in play. wgpu's own pipelines cannot use
archives in wgpu 30, so for V1 their cold-start compile is recorded rather than archived (owner decision; see Seams).

**A normal frame (Apple tile, after G3t).** The steps run in order:
1. Encode stage, on the render owner. The slot's ring is mapped once its previous use has completed. Dirty instance
   rows, view, light and cluster inputs are written, then the ring is unmapped.
2. Graph encoders, in order:
   - atmosphere compute;
   - optional contribution;
   - Sun shadow cascades (stored, sampled by forward);
   - air shadow;
   - GPU light binning;
   - camera visibility (G3c);
   - forward opaque plus sky (depth stored only when a later reader exists, otherwise memoryless; scene HDR and
     motion stored when read);
   - instance history;
   - then, under G4, a native segment for MetalFX or the portable reconstruction pass.
3. Late acquire.
4. Present pass: tone and display encode into the drawable.
5. One submit, then completion retires the slot.

Valley baseline at 1080p offscreen on this M3 Max: GPU pass sum p50 3.86 ms (forward and sky 1.94, atmosphere 1.24,
shadow 0.50); upload p50 628 µs; encode p50 1.63 ms. These are host-shared baselines, not budgets.

**Cost account (nominal, 1920×1080, before compression).**

| Attachment | Bytes |
|---|---|
| Depth D32 | 8.3 MB |
| Scene `Rgba16Float` | 16.6 MB |
| Motion `Rg16Float` | 8.3 MB |
| `Rgba8` output, removed by direct output | 8.3 MB written + 8.3 MB read |

Memoryless depth saves 8.3 MB of store only where depth has no later reader. Shared transients save one set of
per-slot transients: about 33 MB at 1080p.

Instance upload:

| Rows | 192 B rows | 64 B rows |
|---|---|---|
| 10k | 1.92 MB | 0.64 MB |
| Valley (1,500) | 288 KB | 96 KB |

Measured traffic needs Instruments' Performance Limiters counter set or Xcode capture counters; public
`MTLDevice.counterSets` exposes only timestamps on this M3 Max. Graph-declared bytes and measured device bandwidth
are recorded separately.

## Risks

- **Memoryless buys little at the shipped configuration.** The early sign is that reconstruction and Hi-Z leave no
  eligible depth.
  - The plausible wrong substitute is to claim a tile win from a memoryless flag on a texture that is never used at
    the shipped setting.
  - Guard: report eligibility per attachment at the shipped configuration.
- **Native segment and wgpu tracking disagree.**
  - Lazy initialization may clear native output.
  - Usage transitions may be missing.
  - Early sign: validation errors, a black or stale upscaled image.
  - Guard: the one-time wgpu clear at creation; a stand-in segment proof before MetalFX.
- **Shared transients race across slots.** Early sign: flicker under a stall. Guard: tracked-hazard textures only;
  test with a held submission.
- **The mapped ring stalls.** `map_async` completes late when the slot is busy. Early sign: map wait time. Fallback:
  `write_buffer` for that frame, counted.
- **Compact instances lose precision or break the history.** Early sign: shimmer or a motion-vector mismatch.
  Guard: image tolerance plus motion tests already in `history_gpu`.
- **Two copies of light binning.** Guard: the CPU table remains the oracle and tests compare tables.

## Performance

Following the 6 November decision, there are no admission gates. The tripwires are a visible hitch, about a 25%
comparable regression, and unexpected scaling in lights, instances, mirrors or resolution. Every row names the
recipe and registry digest. Rows are paired Apple tile and portable runs on the same M3 Max, with GPU pass
timestamps, whole-frame present intervals, upload CPU time, graph-declared bytes, measured bandwidth from a
configured Instruments capture, and image tolerance. Mac rows that stand in for the iPhone are labelled proxy. The
iPhone simulator is function only. Physical iPhone timing is owed.

## Seams for Thrum Core (proposals)

1. **Boundary amendment.** Admit the tier-2 *whole-encoder native segment* inside the one submission as defined
   above. The decision currently excludes "partial raw splice", and this needs an explicit ruling.
2. **Registry.** `Recipe`, `NativeFeatures` and the digest in `DeviceCapabilities`, built by the platform per
   generation.
3. **Graph.** Memoryless eligibility, `PassBackend::NativeSegment`, one merged scene/present graph, transient sets
   per recipe generation.
4. **ABI.** 64 B instance rows for both recipes (`abi.rs`, shaders, G3c consumers).
5. **Archives.** For wgpu-built pipelines, choose (a) contribute or patch a Metal `PipelineCache` in wgpu, or (b)
   accept closure-time preparation plus the OS shader cache with recorded cold-start time. The plan assumes (b) until
   the owner rules.
6. **Route.** A fixed valley camera route with motion, shared with G4 and G7. The G7 presets are paused today.

## Open questions

- Does `MAPPABLE_PRIMARY_BUFFERS` with per-frame `map_async` beat `write_buffer` on this M3 Max? Settled by M3's
  paired rows. If not, the raw shared-storage ring through `create_buffer_from_hal` becomes a tier-2 candidate.
- Which attachments are memoryless-eligible once G4 and G3c land? Settled by the eligibility report in M1, rerun
  at M6.
- Is the native segment's wgpu-core tracking sound with no barrier calls on Metal? Settled by the M5 stand-in proof.
