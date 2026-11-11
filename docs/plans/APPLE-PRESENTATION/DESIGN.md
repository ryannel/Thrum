# Reconstruction, display and quality governor (G4)

[Boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) ·
[C04](../../design/CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../../design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../../design/CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Render spec](../../design/RENDER-SPEC.md) · [Apple tile (G3t)](../APPLE-TILE/DESIGN.md) · [Milestones](PLAN.md)

Revision 3 for Thrum Core review, 7 November 2026; G3t handoffs reconciled 9 November. Revised 10 November after
[the loop's design review](../../evidence/APPLE-PRESENTATION/design-review.md) at `4bbe44a4` (changes requested); a
fresh design review of this revision is owed. It answers the three independent reviews of revision 1
(`86ec3b6b`) and the re-reviews of revision 2 (`c507ad3d`) recorded in [the log](LOG.md). Interface and path names below are
proposals.

**The owner's request.** The V1 exit in [the build plan](../BRIEF.md#goal-and-exit-rule) asks for "MetalFX,
EDR and a quality governor" and gate G4 for "No pipeline compilation during play." The 10 November planning task
that put this module on the loop says, word for word: "The owner wants work lined up continuously toward the V1
exit in docs/BUILD-PLAN.md, so builders always have a next milestone". The task for this revision says, word for
word: "Revise APPLE-PRESENTATION's design and plan as docs/evidence/APPLE-PRESENTATION/design-review.md asks".
[The module list](../ARCHITECTURE.md) places this module and accepts seams 1–4 below as proposed; M1–M3 are in
[the done-when file](DONE.json) and M4–M11 are migrated next, when seams 5–9 are settled.

**What a design approval covers.** A Design Approved result for this revision approves M1–M3 and seams 1–4, and
nothing later. Everything this page says about M4–M11 and seams 5–9 is direction for the plan run that migrates
them. That run settles, and a new design review reads, at least:
- the final EDR transform and its H = 1 identity;
- the output and exposure handoff of the normalising MetalFX fallback;
- the lifecycle of a generation build (cancellation, device loss, the switch);
- the archive mechanism.

No builder starts M4 or later on the strength of an approval of M1–M3.

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
and evidence. G4 consumes, from [APPLE-TILE](../APPLE-TILE/DESIGN.md):
- G3t's proof tooling (route, counters, reference image);
- the replaceable pre-tone scene-linear colour/exposure input, with the existing SDR tone/intermediate/present path;
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
- offline metallibs and binary archives (G4 M10);
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
  - *The star pass is its own pass.* Today it declares and attaches colour alone (`synthetic/stars.rs`). Its graph
    accesses and attachments follow its shader outputs: motion from M1, reactive from M2. A star pipeline that
    declares a target the pass does not attach, or the reverse, is a defect the M2 witness reads for.
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
    existing drop contract); a discarded MetalFX frame resets the provider before its next encode;
  - a failed submission or device loss resets history and tells the MetalFX provider to reset on its next encode;
  - MetalFX's internal history advances at encode, so a discarded or failed frame resets it at the next encode.
- The previous matrices in the view record are the last *submitted* frame's.

**The temporal-input witness (M2).** Until M3, nothing reads the reactive target, and final colour is the same
whether history advanced at submission or earlier. Screenshots of the valley therefore cannot prove M2: a target
that is declared, cleared and never written passes them. M2 is proved on each target by a witness that reads the
temporal inputs themselves, from frames the game's own frame path submitted.

- *One fixture, two targets.* One checked-in definition holds the steps and the expected values. The Mac tests and
  the Chrome case both read it, as the moving route's file already serves both targets. Each target drives the
  steps its own way. The content it needs (the cooked valley with its volume, the star field, the Sun disk) runs on
  both targets today.
- *Where values are read.* On the Mac, GPU tests read the reactive and motion targets back. In Chrome the case
  runs the built game in hardware Chrome and takes the values from frames the page submitted, through T2's Chrome
  capture. Either a reactive view beside M1's motion view in the display pass, or a readback of the attachment in
  the same submission, does that. A side render (`begin_capture`), a pipeline declaration and a CPU-side flag are
  not observations.
- *History identity.* For each submitted frame the frame host reports the identity of its history sample and of
  the sample it took as its predecessor. Both come from the state the view record's previous matrices are packed
  from, not from a separate counter. The game page's own report carries them; T2's capture record is not changed
  by this module.
- *The unsubmitted view.* Between two submitted frames, A and C, the fixture makes the frame path hold a view it
  does not submit. The view is abandoned at the latest point the target's frame path can abandon one: on the Mac a
  frame encoded and then cancelled on failed acquisition; in Chrome whatever the browser path really does, for
  example a canvas acquisition the checker makes fail once, as its retry case already injects one configure
  failure. The witness records the trigger and how far the view got. The camera moves after A, and the unsubmitted
  view and C share one pose P. So if history advanced early, C would see no camera change and write zero motion on
  static geometry.

| Row | Step | Observed in the submitted frame | A fault it catches |
|---|---|---|---|
| W1 | One object has no valid history: a spawn, or another of M1's invalid-history cases that marks a single object | Reactive is 1 on the object's pixels | An inert target; invalid history left unmarked |
| W2 | The next submitted frame, same object | Reactive is 0 on those pixels | Marking that never clears |
| W3 | The cooked volume in view, nothing changing | Reactive is above 0 on the volume and 0 beside it; on the Mac it equals the alpha the volume wrote | A volume that does not write the target |
| W4 | A time-of-day edit with stars in view, and one with the Sun disk in view | Reactive is above 0 on star pixels and on the disk in that frame | A star pass or sky that does not write on a phase edit |
| W5 | Steady frames before and after W4, and every other step | Star and Sun pixels read 0; a region no reactive writer covers reads 0 in every step | Stars reactive permanently; a target cleared to a nonzero value |
| W6 | Submit A, the unsubmitted view, submit C | C's predecessor is A's sample; the unsubmitted view is no frame's predecessor | History advanced before submission |
| W7 | The same | C's motion on static geometry is the camera change from A to P, which the fixture makes clearly nonzero | Previous matrices advanced before submission |

A row that cannot be observed on a target fails the witness there. It is not skipped. W6 and W7 on the Mac are the
pipelined frame-host tests of the acceptance protocol above; W1–W5 on the Mac and W1–W7 in Chrome are the witness
runs. The browser checker's existing cases stay as a regression check, and prove nothing about these rows.

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

**Reconstruction reference and tolerance.** This is how a reconstructor's still image is judged, from M3 on.
- *Reference.* At a still route checkpoint, with the world paused, a separate accumulation mode on portable renders
  the frame at native resolution once for each offset of the M1 jitter sequence, and averages the frames in
  scene-linear colour before the one SDR transform. That gives 32 distinct sub-pixel samples for each pixel, under
  a box filter. It is a deterministic 32-sample supersample. It is not claimed to be converged, and its frame count
  is not evidence that it is.
- *Held state.* The committed tick, camera, solar time and celestial phase, cooked content generation, resolved
  exposure and storage scale p are the same for every accumulated frame, and for the TAAU and no-AA frames judged
  against the reference. The mode records them and refuses a checkpoint where any differs. It runs the 32-phase
  cycle twice, 64 frames in all. The second cycle adds no new sample: the two cycles' averages must agree within
  float rounding, which shows the state was held.
- *Judged frames.* TAAU's frame is taken after the camera has been held for the same 64 frames. The no-AA frame is
  one unjittered frame in the same state.
- *Difference.* All three images pass through the same SDR transform to the submitted 8-bit sRGB output. ΔE is
  CIEDE2000, per pixel, after decoding sRGB (IEC 61966-2-1) to CIELAB with a D65 white. It is taken over the whole
  frame at the checkpoint's render extent, with no mask, crop or downscale. p99 is the 99th percentile of those
  per-pixel values.
- *Tolerance.* TAAU's p99 to the reference is at most 3, and is lower than the no-AA frame's p99 at the same
  checkpoint. The second condition is firm. The 3 is provisional, and it binds until a plan revision changes it: a
  candidate that misses it has failed. Its first recorded distribution goes to the planner, who may tighten the
  bound, or change it where the record shows the reference or the metric is at fault and not the candidate. Either
  change is a plan revision with its evidence in the log and a design review. A builder does not change it.

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
  with linear filtering implemented and qualified by G4; the surface binding is execute-time (G3t M2/M3).
  G3t itself retains nearest-source SDR mapping and its intermediate.
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
- M9 takes a planning baseline of launch-to-ready. M10 then builds and proves the pipeline preparation work: offline
  metallibs, parallel pre-warm and local archive qualification, all within V1. Their details are designed when M10
  starts.
- Contributing the cache upstream (gfx-rs/wgpu#8526) is a later item that does not gate V1, under the owner's per-post authorization rules in G4 M10/U1.

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
- *Display (`synthetic/display.rs`, `synthetic_display.wgsl`, `present.rs`, `present.wgsl`).* G3t M3 exposes the
  pre-tone linear colour/exposure input and retains the current two-pass SDR arithmetic/quantization. G4 feeds
  reconstruction into that input, and adds/qualifies its EDR branch and transition-time linear scaling. No G3t
  fusion or cross-shader bit-equivalence is assumed. Ordinary SDR keeps the retained branch; a later fused SDR
  replacement needs separate feasibility evidence and reviewed semantics, not a widened image tolerance.
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
  is discarded and MetalFX resets at the next encode, under the reconciled G3t/C06 handoff. On iOS in the background
  the frame is dropped and history resets. Each outcome is
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
- **The reactive target is declared but inert, or history advances before submission, on one target only.** Nothing
  visible changes until M3. Guard: the temporal-input witness on the Mac and in Chrome.

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
   presentation-drop contract stays unchanged: TAAU history commits at successful submission of an accepted sample;
   discarded native frames reset the provider before reuse under G3t/C06. Provider reset on failure or generation
   change. `HistoryKey.extent` is
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
     cooked-asset pipelines (`thrum-asset-gpu/src/draw.rs:239-249`, `material.rs:333`), the atmosphere sky
     pipeline, and the separate star pass with its own attachments and graph accesses.
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
8. **Native provider.** MetalFX through G3t's independently reviewed whole-encoder segment (depends on G3t M11).
9. **Pipeline preparation and archives.** Owned by G4 M10 under the G3t boundary: the measured split, offline metallibs through
   passthrough, and the Metal `PIPELINE_CACHE` in a pinned patch; upstream later. V1's archive item stays as written.

## Open questions

- Does MetalFX beat TAAU in quality per millisecond on the valley at 0.67 and 0.5? Settled by M11.
- Does tile traffic follow the drawn subrectangle at reduced scale? G4's paired proof reruns G3t M12's fixed-extent
  baseline with real reconstruction readers and scale steps, where M0
  proved a capture method.
- Is the TAAU 1080p estimate right? M3 records TAAU's time at its own 640×360 extent, kept apart from this
  estimate; the 1080p figure is settled when M4 gives the surface extent.
- Does MetalFX's history track a GPU-written exposure texture under a changing storage scale? Settled by M8.

## Game outcomes

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

In the V1 game on this M3 Max:
- the valley fills the window at full backing resolution;
- MetalFX temporal reconstructs it on Apple tile, and TAAU on portable and in Chrome;
- stars, the Sun and reveals are stable;
- EDR highlights appear on the built-in XDR display within its headroom;
- the governor holds the display rate under forced load without oscillation;
- the simulation is unchanged;
- scale, headroom and cut changes create nothing during play;
- resizes and backing-scale changes keep presenting while the new generation builds off the render thread.

The paired proof compares Apple tile + MetalFX + EDR with portable + TAAU. The iOS simulator shows reconstruction and
the SDR path for function only. iPhone EDR and timing are owed.

## Evidence conventions

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

As in [G3t's conventions](../APPLE-TILE/DESIGN.md#proof-and-builder-handoff): fixed-step route, reference image, counters,
noise threshold and labels. Additions:
- **Reconstruction reference and tolerance.** [The design](DESIGN.md#techniques-and-evidence) defines
  both, under *Reconstruction reference and tolerance*: a 32-sample reference averaged in scene-linear colour with
  the scene, time and exposure held and checked; CIEDE2000 on the submitted SDR output over the whole frame; p99 at
  most 3 (provisional, changed only by a plan revision) and lower than the no-AA frame's.
- **Temporal-input witness.** [The design](DESIGN.md#techniques-and-evidence) defines rows W1–W7 under
  *The temporal-input witness*: one fixture read by the Mac tests and the Chrome case, with values taken from
  submitted frames.
- **Motion review.** Pans, translating lights, reveals behind a moving prop, a spawn, a time-of-day scrub and a cut,
  recorded as sequences and judged by agent review. Each review states that it watched motion.
- **Display overrides.** The only display is the built-in XDR panel (3456×2234, potential headroom 16). Rows for SDR
  displays, refresh changes and display moves use forced overrides of the display record, labelled as such. A
  physical external-display row is owed.
- **Sim identity.** World-state hash and tick count at route end, with the governor on and off.

## Ownership, call sites and review order

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

| Area | Owner after approval | Files | Shared call sites |
|---|---|---|---|
| Jitter, matrices, motion, subrectangle | G4, with the C07 and RENDER-SPEC owners' review | `synthetic/view.rs`, `synthetic.wgsl`, `synthetic.rs` (`pack_aux`) | G3c visibility (view record), atmosphere |
| Sky and star motion at infinity | Atmosphere owner, by G4 request (seam 4) | `atmosphere/sky.wgsl`, star pass | Atmosphere plan |
| Third forward target and mip bias | G4, with the G5, asset and atmosphere owners' review (seam 3) | `synthetic.rs` forward targets, `contribution.rs:56`, `thrum-asset-gpu/src/draw.rs`, `material.rs`, `atmosphere/gpu.rs` | All forward contributors |
| History protocol | G4, with the C07 owner's review (seam 1) | `history.rs`, `runtime.rs` | The existing drop contract; G3t M10–M11 for native frames |
| Reconstruction | G4 | new `reconstruction/{mod,taau,jitter,exposure,reference}.rs`, `taau.wgsl` | Pre-tone linear input (G3t M3) |
| Render extent and transitions | G4, with the frame-runtime and G3b owners' review (seam 5) | `runtime.rs:39`, `runtime/preparation.rs`, `render_actor.rs`, `examples/v1-game-ios/src/ios/render.rs:24` | G3t M11 provider rebuild |
| EDR present | G4 | `present.rs`, `present.wgsl` | G3t M3 pre-tone input; G4 owns EDR display math |
| Platform display record, EDR layer, thermal | G4, with the platform owner's review (seam 6) | `thrum-app/src/app/device.rs`, `app.rs`, `display_link.rs`; iOS `ios.rs`, `ios/render.rs` | G3b lease (untouched) |
| Governor and missed ticks | G4 (seam 7) | new `governor.rs`; `runtime.rs` (missed-tick count, value commit) | C04 |
| MetalFX provider | G4 | new `apple_tile/metalfx.rs` | G3t `apple_tile/segment.rs` |
| Pipeline preparation and archives | G4, with the asset-pipeline owner's review for the cook | metallib cook step, passthrough loader, wgpu-hal Metal `PipelineCache` patch | G4 M10 pipeline preparation; every pipeline site |

Review order:
1. History protocol.
2. Jitter, matrices and motion.
3. Forward contributors.
4. TAAU.
5. Extent and transitions.
6. Scale.
7. EDR and display record.
8. Governor.
9. MetalFX.
10. Offline metallibs and archives.

## Parallel work and dependencies on G3t

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

| G4 milestone | Can start when | Runs in parallel with |
|---|---|---|
| M1 jitter, matrices, motion | G3t M0, atmosphere checkpoint, seams 2 and 4 | G3t M1–M3 |
| M2 temporal inputs, history protocol | M1, G3t M1 (eligibility report), G3t M2 (shares `runtime.rs`), seams 1 and 3 | G3t M3/M10–M11 (sequential integration for shared files) |
| M3 TAAU at native scale | M2, **G3t M3** (TAAU output feeds the retained pre-tone input) | G3t M10–M11 |
| M4 surface extent and transitions | M3, **G3t M3**, seam 5 | G3t M10–M11 |
| M5 dynamic scale | M4 | M6 |
| M6 EDR on macOS | **G3t M3**, seam 6 | M5 |
| M7 governor | M5, seam 7 | M6 |
| M8 MetalFX temporal | **G3t M11**, seam 8, M5 | M9 |
| M9 launch-to-ready split, prepared paths | D9.1 after G3t M0; the rest after M4–M8 | G3t M12 |
| M10 offline metallibs, pre-warm, local archive | D9.1 | M2–M9 (own files) |
| M11 paired proof | M1–M10, G3t M12 | none |

**Handoffs from G3t**, with their contracts in [G3t's plan](../APPLE-TILE/DESIGN.md#sequence-and-dependencies):
- *G3t M0 → all checks.*
- *G3t M1 → M2.*
- *G3t M3 → M3/M4/M6.* One replaceable pre-tone scene-linear input carries matching exposure, extent and
  sample/generation identity. The existing SDR tone-to-Rgba8Unorm-to-present path and nearest mapping remain.
  G4 inserts TAAU/MetalFX there and owns the qualified EDR branch, headroom transform and transition filtering.
  The surface is bound at execute time; no fused-shader feasibility or traffic saving is inherited from G3t.
- *G3t M11 → M8.* The first native handoff, unlocked by G3t D10.1–D11.3.

G3t M11 proves replacement with bounded provider fixtures; M12 records current readers and fixed extents. Neither
waits for G4. G4's paired proof reruns eligibility with M2's real reconstruction readers and traffic with M5's actual
scale steps; these are G4 integration obligations, not backward G3t prerequisites.

M1–M3 are the early in-game work on portable through public interfaces. They need G3t's proof tooling and graph
milestones, not the boundary ruling.
