# Reconstruction, display and quality governor (G4)

[Boundary decision](../decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../decisions/HARDWARE-NATIVE-ENGINE.md) ·
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Render spec](../RENDER-SPEC.md) · [Apple tile (G3t)](APPLE-TILE.md) · [Milestones](APPLE-PRESENTATION-PLAN.md)

Draft for Thrum Core review, 7 November 2026. Interface and path names below are proposals.

## Purpose and scope

The V1 valley should look sharp and stable at the display's full resolution, use the Mac's EDR headroom for the Sun,
sky and lights, and hold the display's frame rate as load changes. It must do this without a hitch, a compile during
play, or any change to the simulation. G4 adds:
- temporal reconstruction behind one frame contract, with MetalFX temporal on Apple and portable TAAU everywhere;
- EDR output with a prepared SDR path;
- a quality governor that changes render scale, effect quality and cadence;
- prepared native pipelines and archives through G3t's backend boundary.

Consumers are the V1 game on macOS and in Chrome, the iOS shell (function only in the simulator), G7's showcase
route and evidence. G4 consumes G3t's registry record, merged scene/present graph and native-segment handoff
([APPLE-TILE](APPLE-TILE.md)).

Commissioned:
- the C07 frame contract completed in the live game: jitter, stored and readable depth and motion, reactive mask,
  resolved exposure, wired history keys;
- portable TAAU at native and upscaled scales;
- the MetalFX temporal provider;
- EDR on macOS, with an iOS device path;
- the governor;
- archives and cold start for native pipelines;
- prepared handling of cuts, resizes, dropped frames, display changes, surface replacement and archive rejection;
- paired proof.

Deferred past V1:
- MetalFX frame interpolation and the denoised upscaler, both supported on M3 Max but outside V1's exit;
- the Metal 4 `MTL4FX` scalers, which need a second queue;
- Display P3 output primaries;
- system EDR tone mapping (`edrMetadata`).

The V1 exit's "display-link interpolation" is the existing interpolation of simulation state at the display link.
It is not MetalFX frame interpolation.

## Cues

- **Sharp and stable at full resolution.** The image covers the display's backing pixels with no letterbox band and
  no nearest-filter moiré. The G3b iOS route recorded both defects, and both come from the fixed 640×360 extent.
  Edges along the terrain silhouette, the grid fixture and thin props do not crawl when the camera is still, and do
  not smear or ghost when it pans. Stars and the Sun disk do not leave trails.
- **Correct after a cut.** The first frame after a cut or resize shows no history from before it.
- **Bright where it should be.** On an EDR display, the Sun disk, the sunset sky near the Sun and local lights go
  above SDR white within the display's current headroom. Mid-tones, the UI and SDR content are unchanged. On an SDR
  display, or with EDR unavailable, the image matches today's SDR path.
- **Holds rate.** Under a forced load the governor lowers render scale or effect cadence within a few frames, with
  no visible oscillation, and recovers slowly when the load lifts. The simulation tick and world state are identical
  with the governor on or off.
- **No hitch on change.** Changing scale or headroom, moving the window to another display, replacing the surface or
  rejecting an archive or native provider never compiles a pipeline during play.

## Techniques and evidence

Research notes: `tmp/g3t-g4-plans/{primary-sources,code-interfaces,docs-survey}.md`. The sources are Apple MetalFX
documentation, the WWDC22 10103 and WWDC25 211 sessions, CALayer, NSScreen and UIScreen EDR documentation, the
WWDC21 and WWDC22 EDR sessions, wgpu 30.0.1 source, the FSR2/FSR3 documentation, Karis 2014 and Unreal's dynamic
resolution documentation.

**One frame contract, two reconstructors.** MetalFX temporal and FSR-class TAAU take the same inputs, so one
contract feeds both:
- jittered colour in linear HDR;
- reversed-Z depth;
- unjittered motion, current→previous, in pixels;
- a 0..1 reactive mask;
- a 1×1 exposure;
- a reset flag.

C07 already specifies these fields, and Thrum's motion convention (history pixel minus current pixel, X right, Y
down, render-target pixels) is MetalFX's convention at `motionVectorScale = 1`. Apple's sample code scales by the
render size only because its vectors are in UV units. The sign is still verified on device: one WWDC slide
contradicts the documentation.

Reconstruction runs **after the forward pass, in linear space, before tone mapping**. This is the placement Apple
and AMD prescribe for temporal upscalers.

**Portable TAAU.** The portable reconstructor is a compute pass following Karis 2014 and the FSR2 inputs. It uses:
- Halton(2,3) jitter with `ceil(8·s²)` phases, where s is the per-axis upscale ratio, never (0,0);
- front-most 3×3 velocity dilation;
- YCoCg neighbourhood clipping;
- luminance weighting by 1/(1+luma);
- a history weight lowered by the reactive mask;
- an output-resolution `Rgba16Float` history pair in `History` scope, keyed by C07's `HistoryKey`.

It is the fallback and the image oracle for MetalFX, and it runs in Chrome. At scale 1.0 it replaces FXAA as the
anti-aliasing for the admitted temporal profile (a RENDER-SPEC change; see Seams).

**MetalFX temporal (Apple).** The provider is implemented under G3t's native segment.
- It uses the classic `MTLFXTemporalScaler`, encoded into wgpu-hal's `raw_command_buffer()`. Classic scalers do not
  need to own the command buffer.
- The descriptor:
  - colour `Rgba16Float`, depth `Depth32Float` with `isDepthReversed`, motion `Rg16Float`, output `Rgba16Float`;
  - `isInputContentPropertiesEnabled`, with min and max scale covering the governor's range (M3 Max supports 1.0–3.0;
    Apple advises a maximum of 2× or less);
  - `isReactiveMaskTextureEnabled`;
  - an `exposureTexture`.
- Per frame it sets `inputContentWidth/Height`, the jitter offset (pixel centre minus sample, ±0.5), the motion scale
  and `reset`.
- The scaler is created at closure for each output extent and device generation, never in play.
- The provider uses MetalFX's own minimum texture usages, probed on M3 Max: shaderRead on inputs; shaderRead,
  shaderWrite and renderTarget on the output.
- The SDK 27 option `isJitteredMotionVectorsEnabled` is left off, because Thrum's motion is already unjittered.

**Mip bias.** wgpu samplers have no LOD-bias field. Material sampling uses `textureSampleBias` with a per-view bias
of `log2(render/output) − 1`, carried in the view record. The bias changes with scale and needs no new sampler.

**Dynamic resolution without reallocation.** Render-extent attachments are allocated at the maximum render scale
(1.0 of output) for the generation. The governor's scale sets a viewport and scissor subrectangle, the TAAU input
extent and MetalFX's `inputContentWidth/Height`. A scale change therefore never creates a texture or a pipeline. On
Apple tile, memory traffic still follows the drawn subrectangle, because tile load and store cover only touched
tiles. That claim is to be measured, not assumed.

**EDR.** EDR goes through public wgpu wherever wgpu supports it.
- The surface is configured as `Rgba16Float` with `SurfaceColorSpace::ExtendedSrgbLinear`. This keeps RENDER-SPEC's
  Rec.709/D65 working primaries. wgpu then sets `wantsExtendedDynamicRangeContent` and the layer colour space.
- The tone transform maps exposed scene-linear colour to [0, H], where H is the display's current headroom and
  1.0 is SDR reference white:
  - below a shoulder, it equals the SDR curve in linear terms;
  - highlights compress toward H instead of 1.0;
  - the UI and SDR content stay at or below 1.0.

  With H = 1 the transform reduces exactly to the current `NeutralHighlights` SDR curve, which gives a testable
  identity.
- `contentsHeadroom` (OS 26) declares the headroom the content needs, to save power. The platform sets it on the
  main thread through `Surface::as_hal`.
- *Headroom source.* On macOS the platform reads `NSScreen` current and potential headroom on the main thread (wgpu's
  `display_hdr_info` is main-thread only), on screen-parameter changes and once per display-link tick. It publishes
  H to the render owner through an atomic. On iOS there is no notification, so the shell polls
  `UIScreen.currentEDRHeadroom` per display-link tick.
- *Fallback.* EDR is enabled only when the potential headroom is at least 1.5, which is Apple's threshold. Otherwise
  the prepared SDR surface and tone path run. Both pipelines are prepared at closure.
- EDR is treated as device-only on iOS, because Simulator EDR is unverified.
- *Rejected:* `edrMetadata` system tone mapping, which costs significant memory and GPU, and PQ/HLG 10-bit output,
  which suits video rather than a game using headroom.

**Governor.** The policy follows Unreal's dynamic resolution, the only documented production governor of the
sources:
- *Budget:* the display interval times a target headroom (default 0.85).
- *Inputs:* resolved GPU frame time from timestamps, missed presents, render-owner CPU time and thermal state.
- *Hysteresis:* a change threshold, a minimum change period and a history window.
- *Panic:* a drop after N consecutive over-budget frames, with gradual recovery.

The outputs are a render scale in prepared steps within the scaler's range, an effect quality tier within the
recipe, and effect cadence, such as aerial-perspective LUT updates every N frames under C07's governor-set
cadence.

Every output step is prepared at closure, and a governor change is a C04 profile transaction that commits a
generation of values rather than pipelines. The governor never reads or writes simulation state or rate.

**Archives and cold start.** G4's own native pipelines are the MetalFX scaler and any provider pipelines. These are
created at closure. Where a provider builds `MTLRenderPipelineState`s, it uses an `MTLBinaryArchive` keyed as the
boundary decision requires and treats a miss as a closure-time compile. `failOnBinaryArchiveMiss` is used in tests to
prove the archive hit. wgpu 30's `PipelineCache` is a Metal stub, so wgpu-built pipelines are not archived in V1.
Their cold and warm closure time is recorded, along with a zero count of pipeline creations after readiness. The
plan assumes this until the owner rules on G3t seam 5.

**What would change these choices:**
- MetalFX's neural upscaler, M5 Pro/Max only, would be a new provider behind the same contract.
- A wgpu Metal 4 path would allow `MTL4FX`.
- A measured MetalFX loss to TAAU on the valley would keep TAAU on Apple. The registry recipe selects the
  reconstructor, and the evidence decides promotion.

## The system as it runs

**State and owners.**
- *Platform (`thrum-app/src/app/device.rs`, iOS shell).* Owns:
  - the surface format and colour-space choice;
  - CALayer EDR properties, on the main thread;
  - headroom sampling;
  - the thermal state.

  It publishes headroom and thermal state to the render owner through atomics and never waits.
- *Frame contract (`thrum-render`: `synthetic/view.rs`, `synthetic.wgsl`, `history.rs`, `runtime.rs`).* Owns:
  - the jitter sequence and phase;
  - the jittered and unjittered matrices;
  - the mip bias;
  - a `HistoryKey` per view, reset on cut, resize, device, profile or content epoch;
  - the `ViewHistory` accepted sample.
- *Reconstruction (proposed `thrum-render/src/reconstruction/`).* Owns:
  - the `Reconstructor` choice resolved from the registry;
  - the TAAU passes and its history pair;
  - the exposure 1×1 texture resolve.

  The MetalFX provider lives in `apple_tile/metalfx.rs` and implements G3t's `NativeProvider`.
- *Present and display (`present.rs`, `present.wgsl`).* After G3t M2 the present pass owns tone and display encode.
  G4 adds the EDR transform and the `Rgba16Float` surface path.
- *Governor (proposed `thrum-render/src/governor.rs`).* A pure policy over `FrameSample`s, platform thermal state and
  the display interval. It emits profile transactions that the frame host commits between frames.

**A normal frame (Apple tile, MetalFX, EDR).** The steps run in order:
1. The governor's committed scale s and cadence apply. The view record carries the jitter for phase k, the jittered
   and unjittered matrices, the previous unjittered matrix from the last accepted history sample, and the mip bias.
2. The forward pass draws into the s-sized subrectangle. Depth is stored (read by reconstruction and G3c Hi-Z),
   along with HDR, motion and the reactive mask: the Sun disk, stars and any alpha-blended effect write their
   reactivity.
3. The exposure resolve writes the 1×1 `R16Float` texture from the metered exposure sample.
4. A native segment runs the MetalFX temporal scaler, writing output-extent HDR. On portable or as the fallback,
   the TAAU compute pass runs instead.
5. Late acquire.
6. The present pass tone-maps to the current headroom H and writes the `Rgba16Float` extended-linear drawable, or the
   SDR drawable on the fallback.
7. One submit. Completion advances the accepted history sample.

**Edge cases, each a prepared path.**
- *Cut:* the reset flag goes to TAAU or MetalFX and clears the history epoch.
- *Resize or display change:* the frame host closes a new generation with new extents, scalers and history keys,
  prepared before switching. A refresh-rate change only changes the governor budget. A headroom change only changes
  H.
- *Dropped or unaccepted frame:* history does not advance (C07). The next frame's previous matrix is the accepted
  sample's, so motion stays correct.
- *Surface replacement or device loss:* G3b and frame-runtime recovery, then a new generation.
- *Native provider or archive rejection:* the prepared TAAU twin, or the closure-time compile.
- *EDR unavailable:* the prepared SDR path.

**Cost account (nominal).** Output 1920×1080; render scale 0.67, so 1280×720 (0.92 Mpx).

| Item | Nominal cost |
|---|---|
| Render attachments at s = 0.67 | 0.44× native area: depth 3.7 MB, HDR 7.4 MB, motion 3.7 MB, reactive `R8Unorm` 0.9 MB |
| Reconstruction input reads | about 17 B per render pixel: 15.7 MB |
| History read and write at output | 16.6 + 16.6 MB |
| EDR drawable | doubles from 8.3 to 16.6 MB at 1080p |
| Forward plus sky at s = 0.67 | 0.44 × 1.94 ms ≈ 0.85 ms, if pixel-bound |
| TAAU at 1080p output | an estimate of 0.2–0.5 ms; MetalFX cost is unpublished, and both are to be measured |

With the forward-plus-sky saving against the reconstruction estimate, the expected net GPU saving at s = 0.67 is
about 0.5–0.9 ms on the valley. At s = 1.0 (native AA), reconstruction is a pure cost of the same 0.2–0.5 ms.

The fixed 640×360 extent today is 0.23 Mpx. Moving to the surface's backing extent at s = 1.0 is roughly a 4–9×
increase in pixel work, which is why the governor and upscaling arrive before the paired proof.

## Risks

- **Motion sign or scale is wrong for MetalFX.** Early sign: ghosting that doubles under pans. Guard: a
  translating-object test that compares MetalFX output with TAAU and the native reference; an exposure-tool
  checkerboard check.
- **Jitter leaks into stable content.** The star field, the Sun disk and the UI could shimmer. Guard: stars share
  opaque jitter as ATMOSPHERE requires, the UI is drawn after reconstruction, and the reactive mask covers the Sun
  disk.
- **Wrong exposure input causes flicker.** Guard: the exposure texture is the same sample tone mapping uses for the
  frame; MetalFX's exposure debug tool.
- **The governor oscillates or hides a regression.** Guard: hysteresis plus a minimum change period. Evidence rows
  record the scale trace, and tripwires compare at a fixed scale.
- **EDR changes SDR appearance.** Guard: the H = 1 identity test; an owner visual check on the M3 Max display.
- **Reallocation creeps in.** Guard: the pipeline- and texture-creation counters stay at zero after readiness across
  the scale, headroom and display-change rows.

## Performance

Performance follows the same policy as G3t: no admission gates, tripwires only, and paired Apple tile + MetalFX + EDR
vs portable + TAAU (SDR) on the same M3 Max, with Chrome as the portable functional reference. Rows record:
- GPU pass times, including reconstruction;
- whole-frame present intervals;
- the governor's scale trace;
- missed presents;
- pipeline and texture creations after readiness;
- image tolerance against a native-resolution reference.

Mac rows standing in for iPhone are labelled proxy. iPhone timing is owed.

## Seams for Thrum Core (proposals)

1. **C07 fields.** Jitter phase and offsets, separate jittered and unjittered matrices, the reactive mask and the
   exposure texture as named frame-contract outputs.
2. **RENDER-SPEC.** An admitted temporal AA profile, TAAU at 1.0 replacing FXAA under it, and the EDR headroom
   transform with its H = 1 identity. The spec currently leaves EDR headroom and nits policy unspecified.
3. **Render extent.** The extent comes from the surface's backing size times the governor's scale, replacing the fixed
   640×360 (frame runtime; this also fixes the G3b iOS band and moiré defects).
4. **Present and platform.** The `Rgba16Float` extended-linear surface path, layer properties set on the main thread,
   and headroom and thermal atomics.
5. **Governor.** Location and interface: a C04 profile transaction, committed by the frame host.
6. **Native provider.** MetalFX through G3t's native segment, which depends on G3t seam 1.
7. **Archives.** Shared with G3t seam 5.

## Open questions

- Does MetalFX beat TAAU in quality per millisecond on the valley at 0.67 and 0.5? Settled by M8.
- Does tile traffic follow the drawn subrectangle at reduced scale? Settled by G3t M7's capture at two scales.
- Is the 1080p TAAU estimate right? Settled by M2's timing row.
