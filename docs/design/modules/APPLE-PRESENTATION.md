# Reconstruction, display and quality governor (G4)

[Boundary decision](../decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../decisions/HARDWARE-NATIVE-ENGINE.md) ·
[C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Render spec](../RENDER-SPEC.md) · [Apple tile (G3t)](APPLE-TILE.md) · [Milestones](APPLE-PRESENTATION-PLAN.md)

Revision 3 for Thrum Core review, 7 November 2026. It answers the three independent reviews of revision 1
(`86ec3b6b`) and the re-reviews of revision 2 (`c507ad3d`) recorded in the plan. Interface and path names below are
proposals.

## Purpose and scope

The V1 valley should:
- look sharp and stable at the display's full resolution;
- use the Mac's EDR headroom for the Sun, sky and lights;
- hold the display's frame rate as load changes;
- do all this without a hitch, a compile during play or any change to the simulation.

G4 adds:
- temporal reconstruction behind one frame contract: MetalFX temporal on Apple and portable TAAU everywhere;
- EDR output;
- a quality governor over render scale, effect quality and cadence;
- prepared generation transitions for resizes and display changes;
- native pipeline preparation through G3t's backend boundary.

Consumers are the V1 game on macOS and in Chrome, the iOS shell (function only in the simulator), G7's showcase route
and evidence. G4 consumes, from [APPLE-TILE](APPLE-TILE.md):
- G3t's proof tooling (route, counters, reference image);
- the present pass that owns tone;
- the execute-time surface binding;
- the native-segment handoff.

Commissioned:
- C07 inputs in the live game: jitter; camera, object and sky motion; disocclusion; stored reversed-Z depth; reactive
  mask; resolved exposure; history reset;
- a temporal acceptance protocol that is sound with two frames in flight;
- portable TAAU at native and reduced scale;
- the MetalFX temporal provider;
- EDR on macOS with an iOS device path;
- the governor;
- generation transitions;
- cold start;
- offline metallibs and binary archives (G3t seam 5);
- prepared handling of cuts, resizes, dropped frames, display changes, surface replacement and provider or archive
  rejection;
- paired proof.

Deferred past V1:
- MetalFX frame interpolation and the denoised upscaler (both supported on M3 Max);
- the `MTL4FX` scalers, which need a second queue;
- Display P3 output primaries;
- system EDR tone mapping (`edrMetadata`).

The V1 exit's "display-link interpolation" is the existing interpolation of simulation state at the display link, not
MetalFX frame interpolation. V1 has no in-frame UI: controls are a separate panel. So no UI composition rule is needed
here.

## Cues

- **Sharp and stable at full resolution.**
  - The image covers the surface's backing pixels with no letterbox band and no nearest-filter moiré. The G3b iOS
    route recorded both defects, and both come from the fixed 640×360 extent.
  - With the camera still, edges on the terrain silhouette, the grid fixture and thin props do not crawl.
  - Under pans they do not smear or ghost.
  - Stars and the Sun disk stay steady under jitter and leave no trails.
  - Newly revealed surfaces, behind a moving prop or after a spawn or teleport, show no ghost of what was there.
- **Correct after a cut.** The first frame after a cut or generation switch shows no history from before it.
- **Exposure changes cleanly.** While time-of-day scrubbing or entering shade changes exposure, the image shows no
  brightness lag or ghost.
- **Bright where it should be.**
  - On the EDR display, the Sun disk, the sunset sky near the Sun and local lights go above SDR white, within current
    headroom.
  - Mid-tones are unchanged.
  - With headroom at 1, or without EDR, the image matches the SDR path.
- **Holds rate.** Under forced load the governor lowers scale within a few frames, without visible oscillation, and
  recovers slowly when the load lifts. The simulation is identical with the governor on or off.
- **No hitch on change.** Changing scale or headroom never creates a texture or pipeline. Resizing the window, changing
  the backing scale, replacing the surface or rejecting a provider or archive keeps presenting frames while the new
  generation is prepared off the render thread.

## Techniques and evidence

Research notes: `tmp/g3t-g4-plans/{primary-sources,code-interfaces,docs-survey}.md`. They cover:
- Apple MetalFX documentation and SDK 27.2 headers;
- WWDC22 10103 and WWDC25 211;
- CALayer, NSScreen and UIScreen EDR documentation and WWDC21/22 EDR sessions;
- wgpu 30.0.1 source;
- FSR2/FSR3, Karis 2014 and Unreal dynamic resolution.

Review reports: `tmp/g3t-g4-plans/reviews/`.

**One frame contract, two reconstructors.** MetalFX temporal and FSR-class TAAU take the same inputs:
- jittered linear HDR colour;
- reversed-Z depth;
- unjittered current→previous motion in render pixels;
- a 0..1 reactive mask;
- a 1×1 exposure;
- a reset flag.

Thrum's C07 motion convention (history pixel minus current pixel, X right, Y down, render pixels;
`synthetic.wgsl:561-564`) is MetalFX's convention at `motionVectorScale = 1` (`MTLFXTemporalScaler.h`). Reconstruction
runs after the forward pass, in linear space, before tone. That is Apple's and AMD's placement for temporal
upscalers.

**Motion everywhere, including the sky.**
- Objects already write motion from current and previous interpolated transforms. Motion moves from
  `@builtin(position)` (which becomes jittered) to an unjittered current clip position.
- The sky, Sun disk and stars currently write zero motion (`atmosphere/sky.wgsl:32`). They gain *motion at infinity*,
  following ATMOSPHERE's rule: "reproject with retained view and celestial phase or mark star history reactive on cuts
  and phase edits".
  - *Sky gradient:* camera-only motion. The view direction is projected with the previous unjittered view-projection.
    Its change with solar phase is not rigid motion, so a phase edit, such as a time-of-day scrub step, marks it
    reactive, and steady solar motion is left to history clipping.
  - *Sun disk:* camera motion plus the retained solar phase delta, which is rigid for the disk.
  - *Stars:* camera motion plus the retained sidereal phase delta. The star pipeline blends additively on colour only.
    Its motion target uses blending disabled (replace), so star pixels carry their own motion, not the sum with the
    sky's (seam 4).
- Stars and the Sun are reactive only on cuts and phase edits, never permanently.

**Disocclusion.**
- *Invalid history.* Where an object's history is invalid (spawned, teleported, identity changed;
  `synthetic.wgsl:556-558`), forward writes reactive 1 instead of relying on a fabricated zero vector, as C07
  requires.
- *TAAU.* It also rejects history by depth, comparing like with like:
  - it keeps the previous frame's linear view depth as a render-scale `R32Float` history, together with the previous
    render subrectangle;
  - it transforms each current pixel's world position into the *previous* view and compares that previous-view depth
    with the stored depth sampled at the reprojected location (addressed through the previous subrectangle), under a
    relative threshold.

  A moving camera therefore does not cause false rejection on near geometry. FSR2 reaches the same result by
  reconstructing previous depth into the current grid.
- *MetalFX.* It does its own disocclusion from depth and motion, and receives the same reactive mask.

**Temporal acceptance with two frames in flight.**
- Today `history.rs` clears acceptance at every submit and accepts on provider confirmation.
- RENDER-SPEC:403-409 resets history whenever acceptance is unknown when the next frame is encoded. With
  `IN_FLIGHT = 2` that is most frames.
- G4 proposes *submission-order acceptance* (seam 1):
  - on one queue, frame N+1's reconstruction executes after frame N's, so the history N writes is the history N+1
    reads, whether or not the CPU has observed N's completion;
  - a frame's history sample is accepted when its submission succeeds;
  - on portable, a dropped presentation may cancel the frame, and history stays at the last submitted sample (the
    existing drop contract); a frame carrying a MetalFX segment follows G3t seam 3;
  - a failed submission or device loss resets history and tells the MetalFX provider to reset on its next encode;
  - MetalFX's internal history advances at encode, so a discarded or failed frame resets it at the next encode.
- The previous matrices in the view record are the last *submitted* frame's.

**History keys and scale.**
- `HistoryKey.extent` is the *output* extent. Reconstruction converts render grids to the output grid, which is
  exactly the provider conversion C07 allows. So a governor scale change does not reset history.
- Governor commits are value changes inside a profile generation and do not bump the profile epoch.
- The jitter is one continuous 32-phase Halton(2,3) sequence, which covers scales down to 0.5 (`ceil(8·2²) = 32`).
  Scale changes do not restart it.
- A cut on the same frame as a scale change resets history and commits the scale. The two compose.

**Exposure and the history domain.**
- Scene HDR is stored multiplied by a per-frame storage scale p (`exposure.wgsl:52-76`), which moves with time of day.
- *TAAU* works in the storage domain. It retains p with each history sample, as C07 requires, and rescales history by
  p_current/p_previous on read.
- *MetalFX:* p is resolved on the GPU in the same frame, and the CPU learns it only 1–2 frames later, with no readback
  allowed by C07. So per-frame `preExposure` cannot be set from the CPU, and stays 1.
  - Instead, the GPU exposure resolve writes the 1×1 `exposureTexture` as the tone exposure divided by p. Colour times
    exposure is then the tone-mapper input on every frame, which is the luminance domain MetalFX weights with.
  - D8.3's time-scrub sequence checks whether MetalFX's internal history tracks this.
  - If it lags, a costed fallback runs: a normalising pre-pass divides colour by p into the reconstruction input. It
    reads 8 B and writes 8 B per render pixel, about 15 MB at s = 0.67 and an estimated 0.05–0.1 ms on M3 Max.

**Portable TAAU.** A compute pass following Karis 2014 and the FSR2 inputs:
- front-most 3×3 velocity dilation;
- YCoCg neighbourhood clipping;
- 1/(1+luma) weighting;
- history weight lowered by reactive and depth rejection;
- an output-resolution `Rgba16Float` history pair in `History` scope.

It is the fallback, the image oracle for MetalFX, and Chrome's reconstructor. There is no FXAA in the code: the
current display shader is a pre-AA diagnostic. So TAAU at scale 1.0 becomes the AA of the admitted temporal profile
(seam 2).

**MetalFX temporal (Apple).** The provider runs inside G3t's native segment.
- Classic `MTLFXTemporalScaler`, encoded into the segment's `raw_command_buffer()`.
- Descriptor:
  - colour and output `Rgba16Float`, depth `Depth32Float` with `isDepthReversed`, motion `Rg16Float`, reactive
    `R8Unorm` (`reactiveMaskTextureFormat`);
  - `isInputContentPropertiesEnabled`, with min and max scale covering the governor's 0.5–1.0 (scale ratio 1–2, inside
    M3 Max's supported 1.0–3.0);
  - `isReactiveMaskTextureEnabled`;
  - **`requiresSynchronousInitialization = YES`**, so MetalFX does not compile a faster upscaler in the background
    after creation.
- Graph allocation uses the provider's declared texture usages: shader-read inputs; an output with storage, render
  attachment and sampling.
- Per frame it sets `inputContentWidth/Height`, the jitter offset (pixel centre minus sample, ±0.5; the Y sign is
  verified on device), `motionVectorScale` 1, and `reset`. The exposure texture is GPU-written each frame.
- `isJitteredMotionVectorsEnabled` stays off, because Thrum's motion is unjittered.
- The scaler is created per output extent in a generation build.

**Mip bias.** wgpu samplers have no LOD bias. Material sampling uses `textureSampleBias` with a per-view bias of
`log2(render/output) − 1` from the view record. That covers synthetic materials and cooked materials in
`thrum-asset-gpu` (seam 3).

**Dynamic resolution without reallocation.**
- Render-extent attachments are allocated at maximum scale (1.0 of output) per generation.
- The governor's scale s sets:
  - the viewport and scissor;
  - the view's auxiliary viewport (`pack_aux`), which motion, sky and aerial lookups use;
  - the TAAU input extent;
  - MetalFX's input content size.
- No texture or pipeline is created on a scale change. Atmosphere LUTs are fixed-size and do not scale; only draws
  into the subrectangle do.

**Generation transitions (resize, backing-scale change, device loss).**
- A new extent needs new targets and a new MetalFX scaler, so it cannot be free.
- The frame host keeps presenting the old generation. The present pass scales its output into the resized drawable
  with linear filtering; the surface binding is execute-time (G3t M3).
- Meanwhile a preparation thread builds the new generation: targets, bind groups, providers with synchronous
  initialization, and the history keys. wgpu's `Device` is `Send + Sync`.
- Live drag-resizes are debounced: the build starts once the extent has been stable for 100 ms.
- The switch happens at a frame boundary after the build completes, and history resets there.
- Pipelines are extent-independent and never rebuilt.
- "During play" in checks means the render thread's steady frames. Transition-phase creations are counted separately
  by thread.
- Device loss uses the existing recovery, then the same build.
- *Surface reconfigure during a drag.* Today `render_actor.rs:244-257` reconfigures only after every in-flight frame
  completes, so it skips frames on each resize step. G4 changes the rule, with G3b owner review:
  - reconfigure when no `SurfaceTexture` is held, not when GPU work is pending;
  - coalesce to at most one reconfigure per display tick.

  In-flight GPU work that targets an already-presented drawable is unaffected on Metal.
- *Off-thread build.* This needs `PreparedSynthetic` split in two:
  - an immutable prepared part (pipelines, layouts, samplers), shared behind an `Arc`, which the builder reads;
  - per-generation state (targets, per-slot frame buffers, bind groups, providers), which the builder creates.

  The new generation's frame buffers start empty and take one full upload on the first frame after the switch.
  A newer extent cancels a running build, whose result is dropped. Device loss during a build drops the build and takes
  the recovery path.
- *Governor during transitions.* The governor freezes from build start to the first frame after the switch, and
  discards timing samples that span the switch.
- *Occlusion.* A fully occluded window stops the display link and so stops encoding. Presentation drop covers only
  transient acquisition failures, so no GPU time goes to frames nobody sees.

**EDR.**
- *Surface.* `Rgba16Float` with `SurfaceColorSpace::ExtendedSrgbLinear`, which keeps RENDER-SPEC's Rec.709/D65 working
  primaries.
  - The render owner configures it through wgpu, as today. wgpu sets the colour space and its deprecated
    `wantsExtendedDynamicRangeContent`.
  - The platform separately sets `preferredDynamicRange` (the OS 26 replacement) and `contentsHeadroom` on the main
    thread, through its own retained `CAMetalLayer` reference. It never goes through `Surface::as_hal`, which takes
    the layer mutex that `nextDrawable` holds.
  - M6 verifies how the two properties interact.
- *One surface across displays.* Where the session starts on a display with potential headroom above 1, the EDR surface
  stays configured on every display. On an SDR display H is 1 and the image equals the SDR path. A display move is
  therefore an H change and needs no reconfigure. The 8-bit SDR surface is chosen only at a generation build where EDR
  is unavailable: the iOS simulator, a forced setting, or no EDR display. That costs a 2× drawable on SDR displays,
  which M11 measures. A session that started on the 8-bit surface switches to the EDR surface through a generation
  transition when the window reaches a display with potential headroom above 1.
- *Tone.* The present pass maps exposed scene-linear colour to [0, H], where 1.0 is SDR reference white.
  - Below a shoulder it equals the SDR `NeutralHighlights` curve in linear terms.
  - Highlights compress toward H.
  - At H = 1 it reduces exactly to the SDR curve: a testable identity (seam 2).
- *Headroom source.* The platform owns a per-surface display record: current and potential headroom, refresh and
  backing scale.
  - On macOS it reads `NSScreen` directly on the main thread, on screen-parameter notifications and each display-link
    tick.
  - On iOS it polls `UIScreen.currentEDRHeadroom` per tick; there is no notification. This never touches the G3b
    surface lease.
  - The record goes to the render owner through atomics.
  - The registry holds no headroom.
- EDR is device-only on iOS, because Simulator EDR is unverified.

**Governor.** The policy follows Unreal's dynamic resolution.
- *Budget.* The display interval times a target headroom of 0.85.
- *Inputs:*
  - resolved GPU time, from pass timestamps plus segment times;
  - display ticks without a present: the frame host counts each display-link tick on which no `present()` call was
    made. That catches render-side stalls and slot backpressure. It does not catch compositor-side drops, because
    wgpu hides the drawable's presented time; that gap is recorded;
  - display-link present intervals;
  - render-owner CPU time;
  - thermal state from `ProcessInfo.thermalState` notifications, read by the platform.
- In Chrome the inputs are GPU timestamps where available and otherwise `requestAnimationFrame` intervals.
- *Hysteresis.* A change threshold, a minimum change period (default 1 s) and a history window.
- *Panic.* A drop after 3 consecutive over-budget frames, with stepwise recovery.
- *Outputs.* A scale from prepared steps {1.0, 0.85, 0.75, 0.67, 0.5}, an effect tier within the recipe, and effect
  cadence, such as aerial LUT updates every N frames.
- *Invalidation per output:*
  - scale changes invalidate nothing (output-extent history keys);
  - a cadence change keeps effect histories but marks their age;
  - a tier change resets the history of each effect whose algorithm the tier changes, as C07 requires, and no
    others.
- Every output is prepared, and none touches simulation state or rate. Timing samples arrive about 2 frames late, and
  the governor's window absorbs that. The tick count is a render-side proxy for missed presentation, and evidence labels
  it so.

**Archives and cold start.**
- G4's native objects are MetalFX scalers. They take no binary archive, and their cost is synchronous creation in
  generation builds.
- M9 first measures launch-to-ready, cold against warm, split into naga translation, MSL compile and pipeline state
  creation (G3t seam 5).
- M10 then delivers both layers of seam 5:
  - offline metallibs loaded through `PASSTHROUGH_SHADERS`, with a counted WGSL fallback;
  - Metal `PIPELINE_CACHE` over `MTLBinaryArchive`, carried as a pinned wgpu patch.
- M10 proves an archive hit with `failOnBinaryArchiveMiss`, and proves that a stale archive or a mismatched metallib
  is handled at build, not in play.
- Contributing the cache upstream (gfx-rs/wgpu#8526) is a later item that does not gate V1, under the owner's rules in
  G3t seam 5.

**What would change these choices:**
- A MetalFX neural upscaler (M5 Pro/Max) would be a new provider behind the same contract.
- A wgpu Metal 4 path would allow `MTL4FX`.
- A measured MetalFX loss to TAAU on the valley would keep TAAU on Apple: the registry selects the reconstructor, and
  evidence decides.

## The system as it runs

**State and owners.**
- *Platform (`thrum-app/src/app/device.rs`, `app.rs`, iOS shell).* Owns:
  - the surface format and colour-space choice per generation build;
  - its retained layer reference and the EDR layer properties (main thread);
  - the display record;
  - thermal state.

  It publishes through atomics and never waits.
- *Frame contract (`synthetic/view.rs`, `synthetic.wgsl`, `history.rs`, `runtime.rs`).* Owns:
  - the jitter index;
  - jittered and unjittered matrices;
  - the previous submitted matrices;
  - the mip bias and subrectangle;
  - `HistoryKey` (output extent) and submission-order acceptance;
  - the retained storage scale p.
- *Forward contributors (synthetic, atmosphere sky and stars, `SceneContribution`, `thrum-asset-gpu`).* Write colour,
  motion and reactive. The colour-target count rises from two to three (seam 3).
- *Reconstruction (`thrum-render/src/reconstruction/`).* Owns:
  - the registry-resolved `Reconstructor`;
  - TAAU passes and histories (colour pair, linear depth);
  - the 1×1 exposure resolve.

  The MetalFX provider is `apple_tile/metalfx.rs`, implementing G3t's `NativeProvider`.
- *Present (`present.rs`, `present.wgsl`; tone moved there by G3t M3).* Adds the EDR transform and the transition-time
  linear scaling.
- *Generation builder (`runtime/preparation.rs`, proposed thread).* Builds new generations off the render thread. The
  frame host switches at a boundary.
- *Governor (`thrum-render/src/governor.rs`).* A pure policy over `FrameSample`s, missed ticks and the display record.
  It commits values between frames.

**A normal frame (Apple tile, MetalFX, EDR, s = 0.67).** The steps run in order:
1. The governor's committed s applies. The view record carries jitter phase k, both matrices, the previous submitted
   matrices, the mip bias and the subrectangle.
2. Forward draws into the subrectangle. Depth, HDR, motion and reactive are stored, because reconstruction and G3c
   read them. The sky writes motion at infinity.
3. The exposure resolve writes the 1×1 texture, and p is retained.
4. The native segment runs MetalFX into output-extent HDR. On portable, TAAU runs instead.
5. Late acquire.
6. Phase 2: present tone-maps to H into the `Rgba16Float` drawable.
7. One submit. Submission success accepts the history sample.

**Edge cases.**
- *Cut:* reset to the reconstructor and a new correspondence epoch.
- *Scale change:* values only, no reset.
- *Dropped presentation:* on portable the existing drop contract holds. The frame may be cancelled, and TAAU history
  stays at the last submitted sample because it commits at successful submission. A frame carrying a MetalFX segment
  is submitted without present if Thrum Core admits G3t seam 3's runtime change; otherwise it is discarded and MetalFX
  resets at the next encode. On iOS in the background the frame is dropped and history resets. Each outcome is
  counted in the proof.
- *Resize or backing-scale change:* a generation transition.
- *Refresh change:* governor budget only.
- *Headroom change or a move to an SDR display:* H only.
- *Surface replacement:* G3b and frame-runtime recovery, then a transition if the extent changed.
- *Device loss:* recovery and a build.
- *Provider rejection:* the prepared TAAU twin.
- *Archive or metallib rejection:* a build-time compile or WGSL fallback, counted; never in play.

**Cost account.** Output is 1920×1080 (the V1 window), unless stated.

| Item | Nominal cost |
|---|---|
| Pixel work at s = 1.0 vs today's 640×360 | 9× at the 1920×1080 window; 33× full screen on the 3456×2234 built-in display |
| Allocation at maximum scale | Depth 8.3 MB, HDR 16.6 MB, motion 8.3 MB, reactive 2.1 MB, TAAU linear depth 8.3 MB, history pair 2 × 16.6 MB, EDR drawable 16.6 MB (×3 swapchain) |
| Traffic at s = 0.67 (0.92 Mpx) | Attachment writes 0.44× native; reconstruction input reads about 21 B per render pixel (19 MB); history read and write 33 MB |
| Forward plus sky at s = 0.67 | About 0.85 ms if pixel-bound, from 1.94 ms native |
| Atmosphere | Unchanged: fixed-size LUTs |
| TAAU at 1080p output | Estimated 0.2–0.5 ms. MetalFX cost is unpublished. Both are measured (M3, M8). |

With the forward saving against the reconstruction estimate, the expected net GPU saving at s = 0.67 is about
0.4–0.9 ms on the valley. At s = 1.0, reconstruction is pure cost. Full-screen play at s = 1.0 is about 3.6× the
pixels of the window, which is why the governor exists before the paired proof.

## Risks

- **Motion sign or scale is wrong for MetalFX.** Early sign: ghosting that doubles under pans. Guard: a
  translating-object test against TAAU and the reference; the exposure-tool checkerboard; a jitter-sign check.
- **Sky motion at infinity is wrong.** Early sign: stars smear or shimmer under pans. Guard: a held-camera star
  sequence and a pan sequence.
- **MetalFX history does not track the GPU-written exposure.** Early sign: lag under a time scrub. Fallback: the
  costed normalising pre-pass.
- **Transitions stall the render thread.** Early sign: render-thread frame time spikes during a resize. Guard:
  counters by thread; a resize sequence.
- **The governor oscillates or hides regressions.** Guard: hysteresis plus a minimum period. Tripwires compare at a
  replayed scale trace.
- **EDR changes SDR appearance.** Guard: the H = 1 identity; the owner's visual check.
- **Adding a third forward target breaks contributors.** Guard: SceneContribution and cooked-pipeline tests in G4 M2.

## Performance

No admission gates. Tripwires only. Paired runs are Apple tile + MetalFX + EDR vs portable + TAAU on the same M3 Max
over G3t's route, with a replayed scale trace, plus Chrome as the portable functional reference. Rows record:
- GPU pass and segment times;
- missed ticks and present intervals;
- the governor trace;
- creation counters by thread and phase;
- reference-image tolerance;
- graph-declared and (where M0 proved it) measured traffic at two scales.

Mac rows standing in for iPhone are labelled proxy. iPhone EDR and timing are owed.

## Seams for Thrum Core (proposals)

1. **Temporal acceptance (RENDER-SPEC:403-409, `history.rs`, C07).** Submission-order acceptance on one queue. The
   presentation-drop contract is unchanged until the runtime seam is reviewed: TAAU history commits at successful
   submission of an accepted sample, and native frames follow G3t seam 3. Provider reset on failure or generation change. `HistoryKey.extent` is
   the output extent. Governor value commits do not bump the profile epoch.
   - A GPU execution error surfaces only at completion, after up to `IN_FLIGHT = 2` later frames may have read the
     affected history. Such an error takes the device-loss path, which rebuilds the generation and resets every
     history.
   - The contamination is bounded to those frames and accepted explicitly.
2. **RENDER-SPEC.**
   - An admitted temporal AA profile, with TAAU at 1.0 as its AA.
   - The EDR headroom transform and its H = 1 identity. EDR nits policy is currently unspecified.
   - The view-record ABI change: RENDER-SPEC:410 fixes the 160-byte `ViewRecord`; jittered and unjittered matrices,
     the jitter offset, the mip bias and the subrectangle grow it.
3. **Forward contributors.**
   - A third forward colour target (`R8Unorm` reactive): `SceneContribution::color_formats` (`contribution.rs:56`), the
     cooked-asset pipelines (`thrum-asset-gpu/src/draw.rs:239-249`, `material.rs:333`) and the atmosphere sky
     pipeline.
   - Cooked materials adopt `textureSampleBias`.
   - G5, asset and atmosphere owners review.
4. **Sky motion at infinity.** `atmosphere/sky.wgsl` and the star pass write motion with the retained celestial and
   solar phase (atmosphere owner).
5. **Render extent and transitions.** The extent comes from the surface backing size times s, replacing 640×360. A
   preparation thread builds generations. This fixes G3b's iOS band and moiré defects (frame runtime and G3b owners).
6. **Platform display record and EDR layer ownership.** The platform keeps a retained layer reference and sets EDR
   properties on the main thread. Headroom, refresh and backing scale are per surface (platform owner).
7. **Governor.** Location, inputs (missed ticks from the frame host, thermal state from the platform) and value
   commits inside a profile generation (C04).
8. **Native provider.** MetalFX through G3t's native segment (depends on G3t seam 1).
9. **Pipeline preparation and archives.** Shared with G3t seam 5: the measured split, offline metallibs through
   passthrough, and the Metal `PIPELINE_CACHE` in a pinned patch; upstream later. V1's archive item stays as written.

## Open questions

- Does MetalFX beat TAAU in quality per millisecond on the valley at 0.67 and 0.5? Settled by M11.
- Does tile traffic follow the drawn subrectangle at reduced scale? Settled by G3t M12's two-scale rows, where M0
  proved a capture method.
- Is the TAAU 1080p estimate right? Settled by M3.
- Does MetalFX's history track a GPU-written exposure texture under a changing storage scale? Settled by M8.
