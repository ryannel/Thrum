# Reconstruction, display and quality governor (G4)

[Boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) ·
[Hardware-native engine](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) ·
[C04](../../design/CONTRACTS.md#c04-feature-composition-and-quality--f2) ·
[C06](../../design/CONTRACTS.md#c06-gpu-execution-and-lifetime--v1) ·
[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4) ·
[C12](../../design/CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners) ·
[Render spec](../../design/RENDER-SPEC.md) · [Apple tile (G3t)](../APPLE-TILE/DESIGN.md) · [Work packages](PLAN.md)

Revision 3 for Thrum Core review, 7 November 2026; G3t handoffs reconciled 9 November. Revised 10 November after
[the loop's design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) at `4bbe44a4` (changes requested); a
fresh design review of that revision passed on 11 November. Revised 14 November for WP4 to WP12
([settled below](#wp4-to-wp12-settled-14-november-2026)), and twice on 19 November for
[the surface across a transition](#the-surface-across-a-transition-wp4), the second time after
[its design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) asked for one answer to the drawable's size
after a failed configure and for a stated iOS transition. Revised 26 November for
[the wgpu-core pin](#pipeline-preparation-wp10-and-wp12) that WP12's archive needs, after
[the WP12 builder's handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-handoff.md), and again that day for
[offering the cache upstream](#offering-the-cache-upstream-u1-to-u3), once WP12 was Done. Revised a third time
that day for [the requested frame rate at full screen](#the-requested-frame-rate-at-full-screen-wp11), after
[the WP11 builder's handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11-handoff.md) measured that
the full-screen route misses the display's 120 Hz at the governor's floor, and a fourth time after
[that revision's design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md), which
asked that 60 Hz at full screen not be taken for the bar. Revised 27 November for
[the output scale](#the-output-scale-wp18) and [the full refresh at full screen](#reaching-the-full-refresh-at-full-screen-wp17),
after the owner answered the question
[the WP17 planner's record](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md) put, and again that
day after [that revision's design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md). It answers the three independent reviews of revision 1
(`86ec3b6b`) and the re-reviews of revision 2 (`c507ad3d`) recorded in [the log](LOG.md). Interface and path names below are
proposals.

**The owner's request.** The V1 exit in [the build plan](../VISION.md#goal-and-exit-rule) asks for "MetalFX,
EDR and a quality governor" and gate G4 for "No pipeline compilation during play." The 10 November planning task
that put this module on the loop says, word for word: "The owner wants work lined up continuously toward the V1
exit in docs/BUILD-PLAN.md, so builders always have a next work package". The task for this revision says, word for
word: "Revise APPLE-PRESENTATION's design and plan as .orchestrate/evidence/APPLE-PRESENTATION/design-review.md asks".
The task for the 14 November revision says, word for word: "APPLE-PRESENTATION WP4 to WP11 have no done-when
entries, so no builder can be given them, and SHOWCASE WP7-WP8 (the V1 exit) sit behind them [...]. Write their
done-when items and plans so they can be built, with honest dependencies. Do not change APPLE-PRESENTATION WP3,
which a builder holds."
[The module list](../ARCHITECTURE.md) places this module and settles seams 1–9 below; every work package is in
[the done-when file](DONE.json).

**The owner's answer on full screen, 27 November 2026.** Asked how V1 should run at full screen on the built-in
display, where the route cannot hold 120 Hz at the panel's pixel size, the owner said, word for word: "So the
120 upscale for full screen." At full screen V1 presents at the display's full 120 Hz, with the picture
reconstructed below the panel's pixel size and scaled up to the panel by the display.
[The output scale](#the-output-scale-wp18) is that decision's mechanism, and WP18 and WP17 build it.

**What a design approval covers.** A Design Approved result for this revision approves the whole module. WP1–WP3
are as the 11 November review approved them and are not changed. For WP4 to WP12 the review reads
[the section that settles them](#wp4-to-wp12-settled-14-november-2026), which holds the four things the earlier
approval left for it: the EDR transform and its H = 1 identity, the exposure handoff of the normalising MetalFX
fallback, the lifecycle of a generation build, and the archive mechanism. Where that section and an older
paragraph of this page disagree, that section governs. The 26 November revision changes pipeline preparation
only: the wgpu-core pin, the archive's round trip through wgpu's public pipeline cache, and, after
[its design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md), the game's check of the
saved cache data before import. The second revision of that day adds
[offering the cache upstream](#offering-the-cache-upstream-u1-to-u3) and the work packages U1 to U3. It changes
no mechanism of the engine and nothing WP1 to WP12 or WP11 build or prove. The third revision of that day
adds [the requested frame rate at full screen](#the-requested-frame-rate-at-full-screen-wp11), rewrites what
WP11 proves about the whole path in motion, and names WP17. It changes nothing WP1 to WP10 or WP12 built: a
window that is not full screen asks for the display's full refresh, as before. The fourth revision of that day
answers that revision's design review in the same two sections. It makes 60 Hz at full screen an interim rate
with the full refresh owed by WP17, judges pacing by the Mac host's refresh account, and describes the 120 Hz
retake as it was taken. It changes no mechanism but the judge of pacing. The 27 November revision adds
[the output scale](#the-output-scale-wp18), which WP18 builds, rewrites
[what WP17 does](#reaching-the-full-refresh-at-full-screen-wp17), and lets a quality knob hold one value with
the governor off and another at its first tier ([the governor's knobs](#the-governors-knobs-and-state-wp7)).
It changes nothing WP1 to WP16 built or proved: until WP17 is Done a window resolves the extent and the frame
rate it resolves today.

## Purpose and scope

The V1 valley should:
- look sharp and stable at the window's full backing resolution, and at full screen on a fast display at the
  largest [output scale](#the-output-scale-wp18) that holds the display's refresh;
- use the Mac's EDR headroom for the Sun, sky and lights;
- hold the display's frame rate as load changes;
- do all this without a hitch, a compile during play or any change to the simulation.

G4 adds:
- temporal reconstruction behind one frame contract: MetalFX temporal on Apple and portable TAAU everywhere;
- EDR output;
- a quality governor over render scale, effect quality and cadence;
- prepared generation transitions for resizes and display changes;
- an output scale for full screen on a fast display, scaled to the panel by the display's layer;
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
- offline metallibs and binary archives (G4 WP10);
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
  - At full screen at an output scale below 100%, the layer's scaling adds no band and no moiré, and thin
    props still read. Everything above holds in the frames the surface is given.
- **Correct after a cut.** The first frame after a cut or generation switch shows no history from before it.
- **Exposure changes cleanly.** While time-of-day scrubbing or entering shade changes exposure, the image shows no
  brightness lag or ghost.
- **Bright where it should be.**
  - On the EDR display, the Sun disk, the sunset sky near the Sun and local lights go above SDR white, within current
    headroom.
  - Mid-tones are unchanged.
  - With headroom at 1, or without EDR, the image matches the SDR path.
- **Holds rate.** Under forced load the governor lowers scale within a few frames, without visible oscillation, and
  recovers slowly when the load lifts. The simulation is identical with the governor on or off. At full screen
  on the built-in display the route holds the display's full 120 Hz.
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
    accesses and attachments follow its shader outputs: motion from WP1, reactive from WP2. A star pipeline that
    declares a target the pass does not attach, or the reverse, is a defect the WP2 witness reads for.
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

**The temporal-input witness (WP2).** Until WP3, nothing reads the reactive target, and final colour is the same
whether history advanced at submission or earlier. Screenshots of the valley therefore cannot prove WP2: a target
that is declared, cleared and never written passes them. WP2 is proved on each target by a witness that reads the
temporal inputs themselves, from frames the game's own frame path submitted.

- *One fixture, two targets.* One checked-in definition holds the steps and the expected values. The Mac tests and
  the Chrome case both read it, as the moving route's file already serves both targets. Each target drives the
  steps its own way. The content it needs (the cooked valley with its volume, the star field, the Sun disk) runs on
  both targets today.
- *Where values are read.* On the Mac, GPU tests read the reactive and motion targets back. In Chrome the case
  runs the built game in hardware Chrome and takes the values from frames the page submitted, through T2's Chrome
  capture. Either a reactive view beside WP1's motion view in the display pass, or a readback of the attachment in
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
| W1 | One object has no valid history: a spawn, or another of WP1's invalid-history cases that marks a single object | Reactive is 1 on the object's pixels | An inert target; invalid history left unmarked |
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

**Reconstruction reference and tolerance.** This is how a reconstructor's still image is judged, from WP3 on.
- *Reference.* At a still route checkpoint, with the world paused, a separate accumulation mode on portable renders
  the frame at native resolution once for each offset of the WP1 jitter sequence, and averages the frames in
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
  with linear filtering implemented and qualified by G4; the surface binding is execute-time (G3t WP2/WP3).
  G3t itself retains nearest-source SDR mapping and its intermediate.
- Meanwhile a preparation thread builds the new generation: targets, bind groups, providers with synchronous
  initialization, and the history keys. wgpu's `Device` is `Send + Sync`.
- Live drag-resizes are debounced: the build starts once the extent has been stable for 100 ms.
- The switch happens at a frame boundary after the build completes, and history resets there.
- Pipelines are extent-independent and never rebuilt.
- **Readiness is per device generation.** A device generation is ready when its prepared part and its first
  generation are built. "During play" in checks means the render thread's steady frames of a ready device generation.
  Creations are counted by thread and by phase: launch preparation, recovery preparation, generation build and play.
- Device loss uses the existing recovery, then the same build. Recovery preparation makes the pipelines again, as C06
  requires, and is counted as its own phase. It may run where it runs today, on the render thread, because a lost
  device has nothing to present. Play resumes only when the new device generation is ready, and compiles nothing
  after that.
- *The surface during a drag.* Today `render_actor.rs` reconfigures at every resize step, and only after every
  in-flight frame completes, so it skips frames throughout a drag. G4 changes the rule, with G3b owner review: the
  surface is left as configured while the size changes, the layer scales the picture, and the surface is
  reconfigured once, in the step that takes the generation built for the new size.
  [The surface across a transition](#the-surface-across-a-transition-wp4) gives the rule and why.
- *Off-thread build.* This needs `PreparedSynthetic` split in two:
  - an immutable prepared part (pipelines, layouts, samplers), shared behind an `Arc`, which the builder reads;
  - per-generation state (targets, per-slot frame buffers, bind groups, providers), which the builder creates.

  The new generation's frame buffers start empty and take one full upload on the first frame after the switch.
  A newer extent cancels a running build, whose result is dropped. Device loss during a build drops the build and takes
  the recovery path.
- *Governor during transitions.* The governor holds while a build runs and resumes when the build ends, whether or
  not a generation was taken. [The rule](#the-governor-across-a-build-wp4-and-wp7) is below.
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
  - WP6 verifies how the two properties interact.
- *One surface across displays.* Where the session starts on a display with potential headroom above 1, the EDR surface
  stays configured on every display. On an SDR display H is 1 and the image equals the SDR path. A display move is
  therefore an H change and needs no reconfigure. The 8-bit SDR surface is chosen only at a generation build where EDR
  is unavailable: the iOS simulator, a forced setting, or no EDR display. That costs a 2× drawable on SDR displays,
  which WP11 measures. A session that started on the 8-bit surface switches to the EDR surface through a generation
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
- *Budget.* The display interval times a target headroom of 0.85. The display interval is the interval between
  the display link's callbacks at [the requested frame rate](#the-requested-frame-rate-at-full-screen-wp11),
  which is not always the panel's refresh.
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
- WP9 takes a planning baseline of launch-to-ready. WP10 then builds and proves the pipeline preparation work: the
  pinned backend, offline metallibs and parallel pre-warm, and WP12 local archive qualification, all within V1. Their
  mechanism is [settled below](#pipeline-preparation-wp10-and-wp12).
- Contributing the cache upstream (gfx-rs/wgpu#8526) is later work that does not gate V1, under the owner's rules
  in [offering the cache upstream](#offering-the-cache-upstream-u1-to-u3).

**What would change these choices:**
- A MetalFX neural upscaler (M5 Pro/Max) would be a new provider behind the same contract.
- A wgpu Metal 4 path would allow `MTL4FX`.
- A measured MetalFX loss to TAAU on the valley would keep TAAU on Apple: the registry selects the reconstructor, and
  evidence decides.

## WP4 to WP12, settled 14 November 2026

See the [plan](PLAN.md) for the build order. WP10 cooks libraries ahead of time and prepares pipelines in
parallel. WP12 adds the archive. WP11 remains the final paired proof. The parameters below carry forward the
drafted values unless specified.

Put new native proof arms in `scripts/apple_presentation.py` and browser cases in
`scripts/check_v1_browser.mjs`. Keep window, Chrome and simulator proofs in work package or main-branch checks.

### Extent on three targets (WP4)

The render extent is the surface's backing size on the Mac, the oriented surface's on iOS and the canvas's
backing size in Chrome. From WP18 the Mac's is the backing size times
[the output scale](#the-output-scale-wp18), which is 100% wherever this section's work packages proved it. A host is still created at an extent its caller names, so offscreen runs, engine capture
and every exact baseline keep the extent they were retained at: WP4 moves no retained image. Chrome has one
thread, so there a new extent is prepared between two frames as it is today, and the page records the gap. iOS
prepares between two frames too, in its render owner, because its lease lets it present nothing meanwhile
([the surface across a transition](#the-surface-across-a-transition-wp4) says why). Only the Mac builds off the
render thread, and the lifecycle below is the Mac's.

### The lifecycle of a generation build (WP4)

A **request** is what a generation is built for: the backing extent, the display mode (EDR float or 8-bit) and
the device generation, and from WP18 [the output scale](#the-output-scale-wp18). The frame host holds the latest request, and one build at most.

| State | Entered when | What the render thread does |
|---|---|---|
| Steady | A generation is current and matches the request | Draws it |
| Debouncing | The request's extent changed during a live resize, or from WP18 its output scale changed | Draws the current generation at its own extent. The surface stays as configured, and the layer scales the picture to the window |
| Building | The extent, and from WP18 the output scale, have been stable for 100 ms, or the request changed for any other reason (backing scale, display mode, a recovered device), which starts at once | The same. The preparation thread creates the new generation's targets, frame buffers, bind groups and providers from the shared, immutable prepared part. The governor holds |
| Ready | The build finished and its request is still the latest for the current device generation | Draws the current generation until a frame boundary where the switch may be entered: no surface texture held, and no job on the preparation thread. There it reconfigures the surface for the new generation, swaps the generation in and resets history and reconstruction |
| Unmet | The build for the latest request failed, or its switch ended in a configure error, and the request has not changed | Draws the current generation at its own extent; the surface stays as configured and the layer scales the picture. No build runs, and the governor runs |
| Retiring | The swap happened | Keeps the old generation, providers included, until its frames in flight complete, as retirement works today |

- **Cancellation.** A newer request sets the running build's cancel flag, which the builder reads between
  resource groups. A cancelled build's result, and a finished one whose request is no longer the latest, is
  dropped on the preparation thread and never bound. Only then does the next build start.
- **Device loss.** A build belongs to one device generation. Loss during a build drops it; the existing recovery
  makes the device and the prepared part again and then issues a new request. A result for an old device
  generation is never taken.
- **Failure.** A build that cannot allocate leaves the current generation drawing and is reported with its
  request. The host is then in Unmet. The next change of request starts a new build; the same request is not
  retried in a loop.
- **The switch** is pointer swaps on the render thread, with one full upload of the new frame buffers in its
  first frame. No texture, view, bind group or pipeline is created on the render thread at any point of a
  Mac transition, which the creation counters prove by thread. Recovery preparation after a lost device is not a
  transition, and is counted apart. So is the surface texture each acquisition returns, with the view of it:
  those are presentation, made in every steady frame, and belong to no generation.

### The surface across a transition (WP4)

**The rule.** The surface's configured extent and format are the current generation's, at every frame. They
change only in the switch, in one step with the generation, or not at all. The host also has one answer for the
drawable's size: it acquires a drawable only while the last `Surface::configure` call on that surface succeeded.

On the Mac:

- **While the size changes, nothing touches the surface.** The drawable keeps the old generation's extent and the
  present pass stays one render pixel to one drawable pixel. Core Animation scales the drawable to the layer's
  bounds with linear filtering, in step with the window: that is the layer's default contents gravity, `resize`,
  which the layer wgpu makes through raw-window-metal leaves in place on the Mac. `resize` stretches to fill. The
  builder may set aspect fill instead if the stretch reads worse under D4.7; neither leaves a band. The host
  therefore has no scaled present path, and the presenter extent, the history key and engine capture's tap all
  stay still until the switch.
- **At the switch the surface is reconfigured once**, by one call to `Surface::configure`, with no surface texture
  held. wgpu waits inside that call for every submission on the device to finish. The wait is paid once for each
  generation taken, it is bounded by the frames in flight, and the compositor goes on showing the last drawable,
  scaled, while it lasts. A display tick it costs is recorded with that reason and the wait's duration, as a
  tripwire. The frames in flight have completed when the call returns, so the old generation retires at once.
- **A switch that cannot be entered is deferred, and nothing is called.** The host enters the switch only at a
  frame boundary where it holds no surface texture and its preparation thread runs no job. The preparation thread
  is the only other thread that submits on the game's device, and it is the host's: a build, and any later GPU
  work off the render thread, runs as a job on it, and the host starts none until the switch has returned. So no
  submission can land inside wgpu's wait, which is the one way released wgpu refuses a valid reconfigure. A
  deferral leaves `Surface::configure` uncalled and every size as it was. The ready generation is kept, the old
  one presents, and the host tries again at the next frame boundary. A deferral needs no limit of its own: it
  ends when the job does, or when a newer request makes the ready generation stale.
- **An error from the call is repaired before anything is acquired.** The switch reads the call's result through
  an error scope. It does not use the immediate retry in `configure_surface`, which exists for the stale drawable
  a lost device leaves and stays as it is for attach and recovery. On any error, public wgpu 30.0.1 has already
  cached the configuration it was asked for, whatever wgpu-core and the layer did, so the next surface texture
  would report the new size over a drawable of the old one. The host therefore takes nothing from the surface
  until it is whole: in the same step it configures the surface again with the current generation's
  configuration.
  - *The restore succeeds.* wgpu, wgpu-core and the layer agree on the old extent again. The ready generation is
    dropped unbound, the old one presents, and the host is Unmet with the error reported, as after a failed
    build. The same request is not tried again.
  - *The restore fails too.* The surface is refused. The host acquires nothing and presents nothing, and reports
    it as the render actor reports a refused configuration today. It acquires again only after a later configure
    succeeds: the next switch, or the platform's reconfigure for a new surface generation.
  - *The device was lost.* Recovery takes over, as it does for a build.
- **The record keeps these apart.** Each switch records its deferrals, its configure calls, whether the
  generation was taken, and a restore with its result. A generation taken is exactly one successful call, and a
  deferral is no call.

**iOS takes a new extent between two frames, and presents nothing in between.** The lease admits a present only
when the drawable's size is the extent UIKit last observed, and the module list's ruling on seam 5 keeps the lease
as it is. Once a rotation is observed, no old-extent frame can be shown, so a build off the owner's thread would
buy no picture: the owner would answer requests it could present nothing for, past the bound the route already
holds. The owner therefore keeps the coherent transition `PLATFORM-RUNTIME-IOS` WP7 built:

- `Resized` bumps the surface epoch and publishes the new extent, as today. A section admitted before it may
  still present once, which is the race the lease has always allowed and counted.
- At the first request under the new epoch with no frame pending, the owner prepares the new extent's generation
  in place, configures the surface inside the lease section, resets history, and only then acquires. It takes no
  drawable under the new epoch before that, so no frame is encoded to be dropped. The pre-present extent check
  stays as the backstop it is.
- In between, the compositor shows the last presented image. The Swift shell sets the Metal layer's gravity to
  `resizeAspect` when it creates the surface, which is `PLATFORM-RUNTIME-IOS`'s choice and stays: the old picture
  is fitted whole and undistorted into the animating bounds, with the view's background beside it, until the
  first frame at the new extent replaces it.
- That frame is presented within the route's existing bound of six frame replies after the resize, with at most
  one raced present and no old-extent present outside it. PID, window, lease and device generation are kept.
- A configure error ends the request with nothing acquired, as today: the owner acquires only after a configure
  for the current epoch and extent has succeeded.

So the two targets differ on purpose. The Mac goes on submitting old-generation frames through a transition; iOS
holds one image and resumes at the new extent. D4.10 observes the iOS interval in the owner's frame replies, and
the route's simulator recording across each rotation is kept beside them as a witness.

Chrome is not affected: its canvas is resized by the page and prepared between two frames.

**Why the surface is not resized as the edge moves.** Released wgpu-core 30.0.1 makes that wait in
`Device::configure_surface` on every backend, for any surface of the device and from any thread, and the wgpu
documentation calls it intended. No call through the released crates changes a drawable's size without it. A
raw `setDrawableSize` behind wgpu leaves wgpu-core and the Metal backend describing the old size, which makes two
authorities for one extent. The WP4 builder's source audit found both, and
[its handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP4-handoff.md) records them. Two designs would remove the wait:
an engine-owned drawable path beside wgpu's surface, or a patched copy of wgpu-core. Neither is taken. Each adds
an authority or a pinned crate from WP4 on, to save one wait that falls after the size has settled, and neither
shows a better picture: a build starts only when the extent has been stable for the debounce, so the old picture
is scaled through the whole drag in every design. Changing a layer's drawable size often is also its own risk:
developers report `nextDrawable` stalling for about a second after such changes, with no fix from Apple
([Apple developer forums](https://developer.apple.com/forums/thread/773229)).

### The governor across a build (WP4 and WP7)

Build work loads the GPU and the CPU, so a frame encoded while a build runs says nothing about steady cost. The
governor therefore **holds** through a build: it commits no change, and it never admits a sample or a missed tick
from a frame encoded in the hold. The frame host marks those frames, so the rule follows the frame and not the
time its sample arrives.

A build ends in one of four ways: its generation is taken, it fails, it is cancelled, or its result is dropped as
stale. A ready generation whose switch ends in a configure error counts as a failed build, and while a switch is
deferred the build has not ended. If the next build starts at once, the hold continues. Otherwise the hold ends:

- **with a switch**, after the new generation's first frame, whose sample carries the full upload and is not
  admitted;
- **with no switch** (a failed build, or one cancelled or dropped because the request returned to the current
  generation), at the next frame boundary. The current generation stays, and the governor's scale applies to it.

Either way the governor resumes with its committed scale, tier and knob values, an empty sample window and an
over-budget count of zero. So no ending of a build leaves it held: after a failed build it answers load on the
retained generation with no further change of request. WP7 builds both halves on WP4's lifecycle: the frame
host's marking of held frames, and the policy that obeys it.

### The EDR transform and its H = 1 identity (WP6)

The SDR curve is the engine's `neutral_highlights`: with x the largest channel of the exposed colour, it is the
identity up to the knee k = 0.76, and above it the peak is `1 − d²/(x + d − k)` with d = 1 − k, with the
colour drawn toward white by `g = 1/(0.15 (x − peak) + 1)`. The EDR transform uses the display's current
headroom H in place of 1 in the shoulder.

```
d    = H − k
peak = H − d² / (x + d − k)            for x > k;   the colour unchanged for x ≤ k
out  = mix(peak, colour/x · peak, 1 / (0.15 (x − peak) + 1))
```

It passes through the knee with slope 1 for every H, so nothing below the knee changes with headroom. It tends
to H and never passes it. At H = 1 the EDR curve reduces algebraically to the SDR curve. The ordinary SDR
path uses the pre-M6 shoulder arithmetic in `crates/thrum-render/src/synthetic/display/neutral_highlights.wgsl`
and retains its dithered 8-bit encode. The float EDR path uses the headroom-dependent expression. Algebraic
equivalence at H = 1 permits rounding differences that can change a quantized SDR byte. Output 1.0 is SDR white.
The float drawable is written without dither. D6.3 checks that at H = 1 a frame encoded from the float
target is within one code of the undithered 8-bit frame at every pixel.

H is the display record's current headroom, read by the platform and published through an atomic. The present
pass takes it as a uniform, so a change of H creates nothing. Both display paths take scene-linear colour
and exposure from `PreToneInput` in `synthetic/display.rs`. Under seam 6, T2-CAPTURE's engine tap records
the written float target in EDR and the existing 8-bit stage in SDR. The game can read the resolved display
mode and H. [Seam 6 ruling](../ARCHITECTURE.md#interfaces)

### The governor's knobs and state (WP7)

The governor owns one registry of **quality knobs**. A knob is a named runtime value with a short list of
prepared steps and a declared history invalidation: none, marks age, or resets its own effect. Render scale is
the governor's own knob. Any engine module registers others through `thrum-render`'s public interface without
`thrum-render` naming it, which is how `WATERS` and `CLOUDS` add theirs; WP7 registers the aerial lookup's
cadence as the first, in the atmosphere files under [seam 7's ruling](../ARCHITECTURE.md#interfaces). The policy
lowers scale first and effect knobs by tier after it, and a thermal state above nominal lowers the tier. The
governor's state, which is scale, tier, each knob's value and the reason for the last change, is in the frame
record and readable by the game.

**A knob's value with the governor off, from `ATMOSPHERE` WP12.** A knob that the governor steps declares three
values, one for each tier. WP7 required the first to be the knob's own default. From `ATMOSPHERE` WP12 a knob
may also declare an **ungoverned value** that differs from its first tier's. A host with the governor off holds
the ungoverned value, which is every offscreen run and every capture that does not enable the governor. A host
with the governor on holds the tier's value from its first frame. Such a knob is for work that costs nothing
to see and is worth spreading only when frames are presented in real time: the atmosphere's
[chart cadence](../ATMOSPHERE/DESIGN.md#the-chart-cadence-wp12) is the first, at 1 ungoverned and 4, 6 and 8 by
tier. A knob that declares no ungoverned value behaves as WP7 built it. The policy's order is unchanged: scale
first, then tiers.

### Timestamp query sets and Metal's limit (WP16)

Metal gives one process 32 live counter sample buffers, and wgpu makes one for every timestamp query set.
Measured on the M3 Max on 25 November 2026: the 33rd `makeCounterSampleBuffer` fails with "Cannot allocate
sample buffer" at every sample count from 8 to 512, and buffers held by another process do not count. The 32
are the process's, whichever wgpu device asks: two wgpu devices on this Mac share one `MTLDevice`. The
iPhone's limit is unmeasured, and nothing below depends on the number. When Metal refuses,
wgpu-core 30.0.1 loses the device (`handle_hal_error`), and reports nothing to an error scope. Every later
creation on that device is invalid, and the first sign is a panic in `create_buffer_init` that names an
unrelated buffer.

A place is taken for as long as the buffer object lives, and Metal gives it back the moment the object is
released. Measured the same day, on Metal alone: with 32 live, releasing one admits one more and not two, and
releasing two admits two. A refused creation returns an error and harms nothing, 100 times in a row. A Metal
command buffer that sampled into a buffer does not keep it alive.
[Measurement](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16-plan/README.md)

So after the engine drops a timestamp query set, what still holds its place is wgpu. wgpu-core keeps the query
set in every command encoder that recorded it, and in every submission until the device has collected that
submission's finished work. wgpu's interface shows the engine neither. A number the engine kept of its own
query sets would therefore give a place back too early, for a set an unsubmitted encoder or a running
submission still holds, and it would see nothing made outside the engine.

WP7 gave each native 3D frame slot a second timestamp batch, with a query set of its own, so a governed frame
could free its slot while its timestamps were read back. That doubled a frame host's query sets. Tests that
share a process, as every binary under `./dev long-tests` does, then passed 32 from four test threads up, and
`long-tests` failed on main at a25e1d13d. The merge tier runs each test in a process of its own and could not
see it.

Two mechanisms, both in the frame runtime's timing files:

- **One timestamp query set for each frame slot.** Only the readback waits on a map. A batch that waits holds
  the buffers it is read from and no query set, because the next frame on the slot is submitted after the
  resolve of the one before and may write the same queries. WP7's behaviour stays: a governed frame frees its
  slot while its timestamps wait, and samples are published in submission order.
- **Metal is asked first, at the one place that creates timestamp query sets.** Every timestamp query set
  `thrum-render` creates is made at one call site, the ray build probe's included
  ([ruling](../ARCHITECTURE.md#interfaces)). On the Metal backend that call site holds one lock for the whole
  process, from its first step to its last. Under it, it first creates as many counter sample buffers as the
  request asks for, directly on the device's `MTLDevice`, and releases them. If Metal refuses one, the request
  is refused with a typed timing error that names timestamp query sets as what the process has run out of, and
  wgpu is not asked, so the device is untouched and nothing of the request remains. If Metal grants them, the
  query sets are created through wgpu before the lock is released. Metal is the one authority on what is live,
  and the engine keeps no number of its own. So a request is judged against every query set live in the
  process when it asks: those on another device, those wgpu still holds for a recorded encoder or a running
  submission after their engine holder was dropped, and those made outside the call site. A place is free
  again exactly when Metal has it back. Other backends have no measured limit and are not asked.

**Who may create at once.** Between the trial and the wgpu creation, the places the trial proved are free
again. The trial's buffers cannot be handed to wgpu, because wgpu 30.0.1 has no way to make a query set from
a Metal buffer, and wgpu's own failed creation is the one that loses the device. So the lock is what closes that interval, and
it closes it only against requests that take it. Measured on Metal alone on 25 November 2026, with 30 live: a
trial of two passed, a creation that took no lock then took one of the two places, and only the first of the
two real creations succeeded. Answered in turn, the same two askers each got a whole answer: the two and then a
refusal, or the one, a refusal of the two, and room for one more.
[Measurement](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16-plan/README.md#the-interval-between-the-trial-and-the-creation)

The guarantee is therefore this, and no wider: while every timestamp query set created in the process during a
request comes through the call site, no request loses a device, and a refused request leaves nothing behind.
Requests are answered one at a time, on whichever device and thread they arrive, so two that compete for the
last places cannot both pass the trial: the first has its query sets before the second is tried. A release
needs no lock, because it can only free a place.

The call site is the one route for everything that shares a process with a request:

- the frame host's timing and the ray build probe, inside `thrum-render`;
- code outside the crate, to which `thrum-render` offers the same call site. The tests of `timestamp_budget`
  hold their query sets through it, because the tests of one binary run side by side and one test's holding
  must not fall in another's interval.

A timestamp query set made with `wgpu::Device::create_query_set` directly is outside the guarantee. Its own
creation is as unprotected as it was, and made during a request it can cost that request its device. One
exists today: `SEA`'s test harness, `crates/thrum-sea/tests/support.rs`, makes one for each frame it times.
Its test binaries prepare no frame host with timing and install no ray build probe, so they hold no request
for it to fall into, and it stays as it is. Code that comes to share a process with a timed frame host or a ray
build probe creates its timestamp query sets at the call site. Making wgpu's creation itself safe to refuse
would take a change to wgpu's Metal backend, and none is planned for this.

A refusal means one thing for each kind of asker:

- **A frame host** needs one query set for each of its frame slots, two today, and is admitted for all of them
  or refused: a refused preparation leaves no query set behind. A host that asked for timing and is refused
  fails its preparation with the typed error, as any failed timing preparation does. It is not prepared
  without timing, because a governor with no GPU time would pass as one that governs. The same preparation
  succeeds once the process has room for the whole host, so one free place does not admit a host of two slots.
- **The ray build probe** is an observer, and a refusal costs it the observation only. The probe holds no query
  set of its own. `RayBuildProbe::encode` asks for one for each mesh build it brackets, so a refusal can reach
  a probe that is already installed. Both of `encode`'s callers, the renderer for its arena meshes and the
  cooked-asset uploader, hand it the mesh build in place of recording the build themselves. A refused `encode`
  therefore records the mesh build as the caller would without a probe, with no markers around it, and
  records no sample for that mesh. The mesh is built and answers ray queries as on a device with no
  timestamps. The probe stays installed and brackets later builds when there is room.

### MetalFX exposure and the normalising fallback (WP8)

The tone pass computes the exposed colour as `stored / A`, where A is the storage scale p times the camera's
inverse gain, resolved on the GPU each frame. MetalFX is first given the stored colour and an exposure texture
of `1/A`. WP8 measures whether its history follows that through a time-of-day scrub and through a forced step of
p. If it does not, the fallback is one pre-pass that writes `n = stored · (K/A)` at render extent, with K the
constant 0.25, the storage scale's own key. Then:

- MetalFX reads n, with an exposure texture of the constant `1/K`, so its input no longer moves with p;
- its output is in the same domain, and goes to APPLE-TILE's pre-tone input together with the exposure that
  matches it, which is the constant K. The tone pass divides by the exposure it is handed, as it does today, so
  it computes the same exposed colour and no second exposure or tone step exists;
- n is a quarter of the exposed colour, and it is stored in a half-float target, whose largest finite value is
  65504. So n is exact to rounding only for exposed values up to 65504/K = 262,016, a little under 2^18. The
  pre-pass follows RENDER-SPEC's storage rule: finite arithmetic, then a clamp to the half-float range, with
  high clamps, low clamps and non-finite repairs counted for the pass. Where `K/A` is above 1 it can clamp a
  value the stored colour held, so WP8 records the counts on the route and tests a value on each side of the
  limit;
- TAAU, the reference and the portable recipe are untouched.

Which path is in use is a resolved selection the game reports. The pre-pass's cost is recorded when it is on.

### Pipeline preparation (WP10 and WP12)

**What the released backend does.** In wgpu-hal 30.0.1's Metal backend, a WGSL shader is turned into Metal
source when a pipeline is created, once for each stage of that pipeline, under options taken from the pipeline:
its layout's binding slots, its vertex layout, its topology class, its constants and its checks. The same call
yields a **stage record** that the backend keeps and uses afterwards: the translated function's name, the
workgroup size, the threadgroup memory lengths, the list of bindings whose buffer sizes the shader is passed,
and the mask of buffers it never writes. A passthrough library gets only the name and the workgroup size
(`metal/device.rs`, `load_shader` and `create_compute_pipeline`); the other three are left empty, and
wgpu-core passes the backend nothing else. So through the released backend a cooked compute shader with a
workgroup array, such as `atmosphere/diffuse.wgsl` or `visibility.wgsl`, gets no threadgroup memory, and a
shader with a runtime-sized storage array, such as the forward pass's `synthetic.wgsl`, gets no buffer sizes
for its bounds checks.

**The pinned backend (from WP10).** A copy of wgpu-hal 30.0.1 is kept in the repository, selected by a `[patch]`
entry and registered in `.config/architecture.toml`. Its difference from the release is kept beside it as
patch files a reader can review, one for each change, and the copy is exactly the release with those patches
applied. WP10 adds the first patch and WP12 the second:

1. *Stage records for passthrough (WP10).* The translation call is the only source of Metal source and stage
   records: the patch lets the cook take both from it, and nothing re-implements its options. A cooked library
   travels as one blob, the stage record of each of its functions followed by the Metal library, in the bytes
   wgpu-core already hands the backend. The pinned loader splits the blob, and pipeline creation fills the same
   fields the WGSL path fills, so the encoder binds threadgroup memory and buffer sizes through the code it
   already has. Bytes with no record section behave as in the release. The WGSL path translates and runs as in
   the release: the patch may expose what its translation call already returns, and changes nothing it does.
2. *The archive (WP12)*, below.

**The wgpu-core pin (from WP12).** wgpu-core 30.0.1 is the second pinned crate and the last: wgpu, wgpu-types
and naga stay the released crates. It is kept as the backend is: a copy that is exactly the release with one
patch file applied, under its own `[patch]` entry and registration, verified by the same pin check.

The archive cannot work without it. wgpu-core wraps every cache's data in a **cache header** that names the
adapter the data was made for, and release 30.0.1 can name only a Vulkan adapter (`src/pipeline_cache.rs`,
`adapter_key`). So on Metal the public `PipelineCache::get_data` panics while it writes the header, and
`create_pipeline_cache` refuses imported data as unsupported before the backend sees it. No change to wgpu-hal
reaches either. The first WP12 builder reproduced the panic on this Mac with a working archive behind it
([handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-handoff.md)).

The patch gives Metal an adapter key, computed in wgpu-core from the adapter information it already holds and
the same in every process on that adapter. With the key, the release's own header write and import validation
run on Metal as they do on Vulkan. The pin covers that and nothing else:

- it changes only wgpu-core's cache-header code and that code's unit tests, which gain tests of the Metal header
  beside the Vulkan ones;
- the header's layout and version, and the bytes of a Vulkan header, are as released;
- no call site, no other backend's behaviour and no public type or signature changes, which is why wgpu and
  wgpu-types need no pin.

What makes an archive stale beyond the adapter is not this patch's business. The backend's validation key,
which the archive patch supplies, and the game's own file header carry it, as the archive item below says. A
fault found anywhere else in wgpu-core is not fixed in this copy, because widening the pin is a new decision,
and [the surface's wait](#the-surface-across-a-transition-wp4) stays as designed.

The library cook belongs beside CONTENT-LOOP's cooker in `thrum-cook`. Shipping packages exclude it, as
`scripts/g5_shipping_links.py` checks.

Three techniques, each behind the registry, each kept only if WP9's and WP12's launch records show it wins:

1. **Libraries cooked ahead of time (WP10).** The unit is a pipeline stage, not a WGSL file, because the Metal
   source depends on the pipeline. For each stage of each pipeline on the manifest, the cook takes the Metal
   source and the stage record from the pinned backend's own translation, compiles the source with the Metal
   toolchain under the language version and invariance option the backend passes, and stores the blob under
   its **key**: a hash of the WGSL source, the entry point and stage, the pipeline facts the translation reads
   (bind group layout entries, vertex layout, topology class, constants, check flags), the wgpu and naga
   versions and the Metal language version. `thrum-render` computes the key from the pipeline's description
   with one function, used by the cook and at launch, and it needs no translation to do so. At preparation, on
   a match the runtime creates the module by passthrough from the blob; on any mismatch, or a blob without a
   record for the function, it builds from WGSL then and counts a fallback. That happens before readiness,
   never in play. The **manifest** is the set of pipelines the route creates, taken from the creation counters.
   The cook therefore runs on a Mac with a Metal device and the Metal toolchain, and never in a shipping
   package.
2. **Parallel preparation (WP10).** The pipelines of the manifest are created across worker threads before
   readiness. The set that is ready does not depend on the thread count.
3. **The archive (WP12).** wgpu 30.0.1 offers a pipeline cache only on Vulkan. The backend's second patch backs
   that cache with an `MTLBinaryArchive` on Metal, and the wgpu-core pin lets its data out and back in. The
   game saves and loads the archive only through wgpu's public `PipelineCache`: `get_data` to save, and
   `create_pipeline_cache` to load. The engine has no Metal persistence call of its own, so there is one
   route to keep right and one to offer upstream. In **harvest**, a separate command, pipelines
   are created with the archive attached and added to it, and the cache's data is written to a file under the
   game's own header, which carries the device, the OS build, the manifest's digest, and a digest of the whole
   byte sequence `get_data` returned.

   The game checks that file before wgpu sees any of it. wgpu's import is an unsafe call whose contract is that
   the bytes came from `get_data`, and wgpu-core's cache header cannot stand in for that check: it validates
   identity, length and a fixed marker, and holds no hash of the payload, so a payload byte changed at the same
   length passes it and would reach Metal. So every process that loads the file, the strict witness included,
   first reads the game's header and recomputes the digest over the saved cache data. The data goes to
   `create_pipeline_cache` only when the device, the OS build, the manifest's digest and the data's digest all
   match. A file that fails is set aside without being imported, and the launch prepares its pipelines the other
   way, counted by cause: stale when the device, OS build or manifest differs, corrupt when the header cannot be
   read or the data's digest differs. wgpu's own header check comes after, and a refusal there is counted the
   same way under its own cause. The digest catches damage; a file rewritten on purpose with a matching digest
   is outside V1.

   In the **strict witness**, a fresh process attaches the archive read-only and creates every pipeline with
   Metal's option that makes an archive miss an error, so no fallback can hide a miss. The archive is local to
   the device that harvested it; nothing is shipped to another device in V1.

### Offering the cache upstream (U1 to U3)

The owner may offer the Metal pipeline cache to wgpu. This is not part of V1 and nothing in V1 waits for it: U1
to U3 are built after WP11, and no work package of any module depends on them.

**What could be offered.** The two cache patches, the archive in wgpu-hal and the cache header in wgpu-core,
which together are the Metal half of gfx-rs/wgpu#8526. The cache-header patch is written as a change wgpu could
take as it stands: it names nothing of Thrum's, follows the code and the tests around it, and is a separate file
from the archive patch, so the owner can offer one or both. The archive patch holds more than the issue's
proposal asks for: beside the cache itself it carries the policy that makes a miss an error, and the repair of
a partial archive with the bundle that holds it. The stage-record patch is not part of this. Its blob keeps
that change inside wgpu-hal; typed fields on wgpu's passthrough descriptor would be its upstream form, and
would mean pinning wgpu-types as well.

**The owner's rules** (7 November 2026, [the log](LOG.md)'s revision 6), in order:

1. coordinate on #8526 first, with an owner-written comment to the contributor who proposed the design;
2. Metal only, against wgpu trunk rather than the 30.0.1 pin;
3. tests in wgpu's GPU suite: a cache round trip, rejection of corrupt or mismatched data, and a warm hit under
   fail-on-miss;
4. a full `cargo xtask test` run on this Mac;
5. an independent review;
6. an owner walkthrough before submission.

The owner writes every public description and comment, and nothing is posted without the owner's OK on each
post.

**What no agent does.** No agent posts, comments or reacts on wgpu's GitHub or its chat channels, opens or
edits an issue or a pull request there, or pushes a branch to any public remote but this repository's own. No
agent writes text for the owner to post either: no draft of a comment, an issue or a pull request description.
wgpu asks contributors to write those themselves, and its maintainers act on it, as the next paragraph shows.
Agents read wgpu's public pages and repository freely. A clone of wgpu is scratch under `tmp/`.

Each half of this rule is judged from what can establish it. Whether text was written to be posted is read from
the delivered notes. What agents did is read from the record each builder session's host keeps of the commands
and tool calls it ran, which the script's delivery history lists by work package. What an account shows upstream
is not evidence either way: the owner may post, or push to a fork of their own, at any time, and wgpu's chat
leaves nothing on GitHub.

**wgpu's side, as read on 26 November 2026.** Each of these is evidence for the shape of an offer, not a
specification, and U1 reads them again.

- The issue is open and assigned to a maintainer, and its last comment is from 20 October. The proposal of
  4 October names the same core change as the cache-header patch, a Metal case in `adapter_key` taken from the
  GPU's name with header unit tests, and an archive behind wgpu-hal's cache. A maintainer answered that it
  "looks about right" and had not then read it in detail. Its author has opened no pull request. Nothing there
  is agreed.
- The proposal leaves the OS version out of the backend's validation key on purpose. The archive patch puts
  the OS build in. That is one difference the conversation has to settle.
- An earlier pull request for the same feature, gfx-rs/wgpu#9623, was closed in June 2026 with no review, after
  a maintainer asked its author to review it before the maintainers did. On #8526 itself, on 20 October, a
  maintainer asked another commenter to write their comments themselves.
- `CONTRIBUTING.md` on trunk allows code an LLM wrote. The author who submits it answers for every line, and
  writes issue and pull request descriptions and large explanatory comment blocks by hand. It asks
  contributors to agree an approach with the maintainers before investing in a large change. The pull request
  template asks how LLMs were used.
- Trunk at `60cef2be` still gives only Vulkan an adapter key in `wgpu-core/src/pipeline_cache.rs`.

**How it proceeds.** Three work packages, each ending where only the owner can go on.

*U1, the upstream notes.* Agents write one document for the owner, kept in U1's evidence: what the owner needs
in order to write that first comment and to judge what to offer. It is written for someone who knows wgpu's
public API and has not read the patches. A reviewer checks it against its sources before the owner reads it,
because the owner will speak from it in public. It has six parts:

1. *Upstream.* The issue, and each pull request that proposes a Metal pipeline cache: author, dates, status and
   what maintainers said, with links and the day read.
2. *wgpu's rules.* What `CONTRIBUTING.md` and the pull request template say, at a named trunk commit, on change
   ownership, LLM use, designing new features and large pull requests, quoted.
3. *The patches, part by part.* For each of the two cache patches, each part of what it does, with the lines of
   the patch it rests on and one of three marks against the proposal: as proposed; proposed but done
   differently, saying how; or not in the proposal, saying why Thrum needs it and whether an upstream change
   could leave it out.
4. *Trunk.* At the same commit: whether trunk has the Metal adapter key and a Metal pipeline cache, and what
   applying each cache patch to it does.
5. *The use case.* Launch-to-ready by preparation path from WP12's launch record, with the conditions that
   record states. Nothing is measured anew.
6. *What the conversation has to settle.* Three decisions: who writes the Metal change, the proposal's author,
   the owner or both; which parts of part 3 are offered; and whether the cache header and the archive go as one
   pull request or two. Each has its options, what the patches as they stand support, and the agents'
   recommendation, marked as theirs.

*U2, the owner's answer.* On the day the owner is asked, agents read wgpu's side again and add a dated line to
each of parts 1, 2 and 4: it still holds, or what changed. That reading looks for pull requests proposing a
Metal pipeline cache opened since U1 as well as the ones part 1 lists, because a new one can appear with no
comment on the issue and no change to trunk, and it bears on who should write the change. The owner then reads
the notes and answers the loop **go ahead** or **stop**.

- **Stop** can come at any point, before a comment is posted or after. The owner's words in the log are its
  whole record, and they are all the next planner needs to drop U3.
- **Go ahead** follows a comment the owner wrote and posted on the issue themselves, once the thread has
  settled enough for them. Its record in the log links that comment and each reply the answer rests on, if
  there is one, and gives the three decisions as far as the thread settled them. How long to wait for a
  reply, and whether silence is enough to go ahead, is the owner's call.

*U3, the change.* A planner writes U3's items from U2's answer, under rules 2 to 6 above. On **stop** that
planner drops U3. Either way the pinned copies stay as V1 needs them until a released wgpu carries the cache.

U1 and U2 change nothing under `vendor/`: the pinned copies and their patch files are V1's, and WP12 proved
them as they are.

### The requested frame rate at full screen (WP11)

Added 26 November 2026, and revised that day after
[its design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md). This section is how
WP11 closed. The owner answered on 27 November 2026, and the 60 Hz rule below ends when
[WP17](#reaching-the-full-refresh-at-full-screen-wp17) is Done. The refresh account, the cue windows and what a
served interval is stay as this section defines them, and WP17 uses them. WP11's builder
measured the full-screen route on the built-in 3456×2234 display with capture off: Apple tile, MetalFX, EDR and
the governor at its 50% floor (1728×1117). That build asked the display link for 60 to 120 Hz with 120
preferred, as every window did then, and the link ran at 8.33 ms. Its fastest take spent 10.85 ms of GPU time
at the median and 14.67 ms at the 95th percentile against that interval, with a present call in 437 of 704
display opportunities. The game was asking for a rate it could not hold, so frames landed unevenly.
[The retake](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-budget-retake/README.md) holds the six
takes, the passes and the host load, which was heavy.

**The lead's decision, 26 November 2026.** WP11 commissions no further work on the frame graph. Until the route
can hold the display's full refresh at full screen, a full-screen window presents at a steady, evenly paced
60 Hz, which is how the lead reads shipped high-end Mac games to run at a display's full resolution with an
upscaler. Reaching 120 Hz there is [WP17](#reaching-the-full-refresh-at-full-screen-wp17), after WP11.

**60 Hz is an interim rate, and the bar is unchanged.** The vision names "this M3 Max and its built-in 120 Hz
ProMotion display" ([decision 1](../VISION.md#decisions)) and judges motion "smooth at the display's full
refresh" ([how it will be judged](../VISION.md#how-it-will-be-judged)). Only the owner changes either, and the
owner has not. So when WP11 ends the bar is unmet in one place, full screen on the built-in display, and the
plan keeps it owed:

- WP11 proves each clause of G4's sentence, and its items judge full screen at the interim rate.
- WP11's evidence page and the maturity ledger say the display's full refresh at full screen is owed, by WP17.
  They do not say the bar is met there, that 60 Hz is the target, or that the milestone is reached.
- WP17 ends when the full refresh is held at full screen, and on nothing less.
- The milestone, Apple reconstruction, is reached when WP17 is Done. WP11 does not reach it.

**The mechanism.** The game asks the display link for a frame rate, and the link calls it back once for each
refresh interval at that rate. That request is the **requested frame rate**.

- A full-screen window on a display that refreshes faster than 60 Hz asks for 60 Hz: minimum, maximum and
  preferred all 60. On the 120 Hz panel the system then sends a callback every 16.67 ms and shows each frame
  for two refreshes, which is even pacing. A frame drawn as fast as it can go, at whatever rate that turns out
  to be, lands on uneven refreshes.
- A window that is not full screen asks for the display's full refresh, as it does now. Nothing WP4 to WP10
  measured in a window changes.
- A launch setting forces either rate. The frame record names the requested rate and says when the setting
  forced it. The setting is new in WP11, so no take made before it is a forced-rate take.
- The platform sets the request on the main thread, where it reads the display record. Changing it creates no
  GPU object and resets no history. The display record's refresh stays the panel's.
- The governor budgets against the interval between callbacks: 16.67 ms × 0.85 = 14.17 ms at 60 Hz. It may
  settle above the 50% floor there, and that is its own decision.
- The iPhone shell and Chrome are unchanged.

The rule for full screen is provisional. It stands on the measurement above and goes when WP17 shows the full
refresh held.

**What steady means, from the capture-off record.** The engine sees its own callbacks and present calls. It
does not see when a frame reached the panel, because wgpu hides the drawable's presented time. So the claim is
about present calls, and the judge is the Mac host's **refresh account**, which already places every present
call in the refresh interval it was made in:

- A **refresh interval** is one period of the requested frame rate, ending at a display-link target time. The
  account opens one for every target a callback brought. It also opens one for every target the link sent no
  callback for, which it finds from the gap between two targets. An absent callback is therefore an interval
  on the record, and never a hole in it. The account's period is the requested rate's, not the panel's.
- An interval is **served** when exactly one present call was made inside it, by the time of the call. Which
  callback asked for the frame does not matter. A call made after its own target serves the interval it landed
  in if that one is otherwise empty, and leaves its own interval unserved.
- Every other interval is **missed**, and the account keeps its cause: the link sent no callback, the callback
  reached the render owner after its target, both frame slots were occupied, no drawable was acquired, or the
  call came outside the interval or shared it with another call.

D11.6 holds when, in each of two capture-off takes on the merge candidate, every refresh interval from the
route's first tick to its last is served. A missed interval counts whatever its cause, because the viewer sees
the same held frame whether the game or the system was late. Twelve callbacks that never came are twelve
missed intervals, and two calls in one interval after an empty one are two. With every interval served,
consecutive present calls sit in consecutive intervals, so the route needs no separate bound on the interval
between them.

Takes are made under the load other builders put on the host, as
[the settings](../../../ORCHESTRATE.md#targets) say of every timing, so a take may be repeated, and two that
hold show it was not one lucky take. Every take made on the merge candidate is kept and listed with its host
load and each missed interval's cause. The cause says where to look: occupied frame slots, a missing drawable
or a late call point at the frame's cost, and an absent or late callback at the main thread or the host. The
record also gives, without a bound, the GPU span and each pass against the interval, and when each present call
was made inside its interval.

What this proves is that the engine made one present call in every refresh interval. When a frame reached the
panel is `SHOWCASE` WP8.6's to witness, from the OS-reported presented intervals of its attended sitting
([who judges frame cadence](../ARCHITECTURE.md#who-judges-frame-cadence)), and D11.5's record says so.

**The shortfall on record.** The evidence page and the maturity ledger say that full screen does not hold the
display's full refresh, and that it is owed. The record of it is the builder's retake, described as it was
taken at `1eacafa62`: the display link asked for 60 to 120 Hz with 120 preferred and ran at 8.33 ms, EDR
headroom was forced to 2, capture was off, and the fastest take's GPU span at the governor's floor, each pass's
time and the host load are given against 8.33 ms. No launch setting forced the rate on that build, so the
record does not call it a forced-rate take. The headroom is a forced override and is labelled as one. A take
the builder makes on the merge candidate with the launch setting at 120 Hz is added beside the retake and is
the one labelled forced.

**Two changes the builder made stay.** Both are in the measured takes:

- On the EDR surface the frame runs the SDR tone encode only when something reads the SDR stage, which is the
  capture tap's SDR source or a readback. The ordinary EDR frame leaves the pass out, and the timestamp
  validator follows the graph variant the frame ran.
- The Mac window asks for copy access to the float drawable only while an EDR tap is on.

**Motion evidence at full screen.** D11.6's image claims are read from consecutive captured frames
([evidence conventions](#evidence-conventions)), and the builder's full-screen capture kept 307 of 800
submitted frames, with 493 omitted by the tap and none refused by the writer. A frame at this size is 30.9 MB,
and the capture's caller gave the tap six staging slots and 27 CPU buffers under a 1 GiB allowance, which is
about a quarter of a second of frames at 120 Hz. The capture is fixed inside
[T2's contract](../T2-CAPTURE/DESIGN.md#cost-and-failure-account), which calls for a smaller declared
workload or more throughput and forbids a smaller image or fewer sampled frames:

- The declared workload is a **cue window** for each of the four cues D11.6 names, which are the camera moving
  past the terrain silhouette and thin props, the stars, a moving prop and the time-of-day scrub: a run of
  consecutive submitted frames
  from before the cue to after it settles, at the display's pixel size and the requested frame rate. The tap
  is off between windows. A window is complete when every presented submission in it has its image, with no
  omission by the tap or the writer.
- The capture's caller chooses the staging slots, CPU buffers and byte allowance for that extent before the
  take. T2 calls these operational limits, and they are the caller's to set.
- The windowed tap keeps `omit` admission. It never holds the renderer back, and no frame is retimed.
- The builder first measures which limit binds, staging slots waiting on readback or CPU buffers still held by
  the writer, at the moments the tap reserves and collects. The earlier record cannot tell them apart.
- If that shows a slot or a buffer the tap never gives back, the fault is in `T2-CAPTURE`'s tap. The builder
  ends `BLOCKED needs_dependency` with the record, and does not change the tap's admission.

Capture adds the SDR tone encode and the copies, so a captured take may miss refresh intervals that the
capture-off take serves. That cost is shown apart and decides nothing about pacing.

### The output scale (WP18)

Added 27 November 2026. The WP17 planner measured the full-screen frame before writing any item
([the record](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md)): four capture-off takes
on the built-in display, the governor at its 50% floor in each. The frame's GPU span fits
`4.3 ms + 0.39 ms × output Mpx + 1.0 ms × drawn Mpx`. MetalFX and the EDR present pass both run at the
drawable's size, and at the panel's 7.72 Mpx they took about 3.2 ms together, which the render scale does not
touch. Against a 120 Hz interval of 8.33 ms, no render scale holds the route at the panel's pixel size.

**The owner's decision, 27 November 2026:** "So the 120 upscale for full screen." At full screen on the
built-in display V1 presents at the display's full 120 Hz, and the picture is reconstructed at an
**output scale** below the panel's pixel size and scaled to the panel by the display's layer, as shipped Mac
games run on this display. Nothing in V1 works toward 120 Hz at the panel's pixel size. The output scale
returns toward 100% as the performance phase after V1 takes work out of the frame.

**The mechanism.** The output scale is one of the five prepared steps the render scale already uses, 100, 85,
75, 67 and 50%, with the same rounding. It is part of
[the request](#the-lifecycle-of-a-generation-build-wp4): a generation is built for the backing extent, the
output scale, the display mode and the device generation, and its **output extent** is the backing extent times
the output scale.

- **Everything a generation owns is sized from the output extent.** Three sizes are kept apart:
  - *What is allocated.* The reconstruction output, the history key, the EDR and SDR targets and the drawable
    have the output extent. So do the forward attachments, which WP5 allocates once for a generation at a render
    scale of 1.0 ([dynamic resolution without reallocation](#techniques-and-evidence)). No texture is sized by
    the render scale.
  - *What is drawn.* The governor's render scale sets the forward viewport and scissor inside those
    attachments, with the view's auxiliary viewport and the reconstruction's input extent, as WP5 built them.
    That drawn subrectangle is the output extent times the render scale, by the render scale's rounding. A
    step of the render scale resizes it and creates nothing, at any output scale.
  - *What is presented.* The present pass's viewport is the whole output extent, one pixel to one drawable
    pixel.

  With a 3456×2234 backing, a 75% output scale and a 50% render scale, the forward attachments, the
  reconstruction output and the drawable are 2592×1676, the forward viewport and scissor are 1296×838, and the
  present viewport is 2592×1676. Below 100% no pass of the frame runs at the backing size, and that is the
  whole saving. The request is the one authority for the extent: nothing reads the window's backing size to
  size a target or a viewport.
- **The surface is configured at the output extent, in the switch.** Core Animation scales the drawable to the
  layer's bounds with linear filtering, which is the behaviour
  [the surface across a transition](#the-surface-across-a-transition-wp4) already relies on while a window is
  resized. At full screen the layer's bounds have the drawable's aspect, so the scaling neither stretches the
  picture nor leaves a band. The host adds no scaled present path, no second upscaler and no sharpening pass:
  each would run at the backing size and bring the cost back.
- **A change of output scale is a change of request**, so it is an ordinary generation transition. It is built
  off the render thread, the surface is reconfigured once in the switch, history and reconstruction reset
  there, and nothing is created on the render thread. The governor holds through the build as through any
  other, and afterwards its render scale applies inside the new output extent.
- **Entering or leaving full screen during play is one change of request.** The launch setting is fixed for the
  process, so a window changing mode is the only thing that changes the output scale during play, and it
  changes the backing extent with it.
  - The platform resolves the scale each time it reads the window, from the window's mode at that read, with
    the backing extent it reads beside it. It keeps no scale resolved at launch.
  - The lifecycle debounces the request as a whole: a build starts when the backing extent and the output
    scale have both been stable for the debounce. A window still animating between modes starts no build, and
    neither does a read that sees one of the two changed before the other.
  - So the host takes one generation for each mode change, from one request that names the new backing extent
    and the new scale together. A window that enters full screen is never presented by a generation at the
    full-screen backing extent and the window's 100%, and one that leaves is never presented by a generation at
    the window's backing extent and the full-screen step.
- **What resolves it.** The platform resolves the output scale with the request, on the main thread, where it
  reads the window and the display record, as it does the requested frame rate:
  - a window that is not full screen resolves 100%;
  - a full-screen window on a display that refreshes at 60 Hz or slower resolves 100%;
  - a full-screen window on a faster display resolves the capability registry's full-screen output scale for
    the device. That value is 100% until [WP17](#reaching-the-full-refresh-at-full-screen-wp17) promotes a
    lower step by measurement, under [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md);
  - a host whose scene is 2D resolves 100% in every case. It has no reconstruction, and
    [the same decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) draws it at its output's own pixel size;
  - for a 3D scene on the Mac, a launch setting forces any of the five steps, in a window or at full screen.
    A value that is not one of the five is refused by name at launch. With a 2D scene the setting forces
    nothing: the host resolves 100% and the record says not forced.

  The iPhone shell and Chrome resolve 100% and read no setting.
- **The registry's value in a proof.** WP18 lands with the registry's value at 100%, so its proofs of the
  full-screen case are given 75% another way. D18.1 gives it to the resolver. D18.5's run gives it to the game
  through its proof options, as the display proofs give a forced headroom
  ([display overrides](#evidence-conventions)). That input stands in for the registry's row and for nothing
  else: it applies only where the registry's value would, the record names it as given for a proof, and it is
  not the launch setting, so the record says not forced. Nothing a player launches sets it.
- **One value for the device.** The registry holds one full-screen output scale for this Mac. WP17 measures it
  on the Apple recipe, which is the row the claim is made for. The portable recipe on the same Mac resolves the
  same value, so that `SHOWCASE`'s rows compare at one extent, and what it holds there is recorded and not
  claimed. The value is measured on the built-in panel. On another fast display it applies unmeasured, and the
  external-display row stays owed.
- **The record.** The frame record and the display record the game reads name the backing extent, the output
  scale, the output extent and whether a launch setting forced the scale. Engine capture taps the drawable, so
  a captured frame has the output extent. A review that needs the picture at the panel's size resamples
  captured frames with a linear filter and labels the result a stand-in for the layer's scaling. No agent sees
  the panel, and engine pixels are never offered as what it showed.
- **Positions in the window.** Anything that maps a position in the window to the picture divides by the
  layer's bounds, never by the drawable's size.
- **The launch setting** is read by the game and passed in the app's configuration, as the frame-rate setting
  is. No engine library reads the environment.

**What WP18 and WP17 do not do.** They commission no change to the forward pass, MetalFX, the EDR transform or
the shadow passes, which is the performance phase's work. They add no interface drawn at the backing size: V1
has none in the frame, and an interface a later game draws over a 3D scene at a reduced output scale is scaled
with the picture until its module proposes otherwise.

### Reaching the full refresh at full screen (WP17)

Added 26 November 2026 as the work package that owes the full refresh, and rewritten on 27 November after
[the owner's decision above](#the-output-scale-wp18). WP17 reaches the display's full refresh at full screen:
it promotes an output scale by measurement and removes the 60 Hz rule.

**What it needs first.** [The output scale](#the-output-scale-wp18), which WP18 builds, and `ATMOSPHERE` WP12,
which spreads the rebuild of the sky's charts over several frames under the governor
([its design](../ATMOSPHERE/DESIGN.md#the-chart-cadence-wp12)). On the measured route those charts were rebuilt
on nearly every frame, at about 1.3 ms, the largest cut that asks nothing of the picture. The planner's fit,
with the governor at its floor, puts the route's median where every interval is served near an 80% output
scale with that cut and near 62% without it. The fit is where to start looking and decides nothing.

**What changes.**

- The registry's full-screen output scale for this Mac becomes the step the sitting below chooses.
- The 60 Hz rule goes. A full-screen window asks the display link for the display's full refresh, as a window
  that is not full screen does. The launch setting still forces 60 or 120 Hz, and the record still says when it
  did.
- Nothing else. The governor's policy, its 85% budget and its steps are as WP7 built them.

**The full-refresh sitting.** Whether every 120 Hz interval is served is a claim about the game on this Mac,
and a busy host misses intervals the game did not cause: in the planner's takes the display link sent no
callback for five of 721 intervals at a load of 4, and sent none or a late one for six of 720 at a load of 22.
So the takes that decide are made in one sitting with the other builders paused, which
[the settings](../../../ORCHESTRATE.md#targets) allow where a design asks for it by name because smoothness at
1x is judged in it. This is that sitting. The builder builds and proves everything else first, then ends
`BLOCKED` asking the lead for it. A take made while builders work is a witness and decides nothing.

- **A take** runs the capture-off release game at full screen on the built-in display: the Apple recipe,
  MetalFX, EDR, the governor on, and a requested frame rate of 120 Hz that no launch setting forced. It is
  judged by the refresh account, with interval, served and missed as
  [defined above](#the-requested-frame-rate-at-full-screen-wp11), at a 120 Hz period.
- **Two routes.** The valley route WP11 used, and the exit flythrough on the exit configuration's Apple recipe
  row, launched as `SHOWCASE` WP7 launches it, from its first tick to its last. The second is the populated
  scene the owner will see, and a value promoted on the bare valley alone could fail there.
- **A step holds** when, on each route, two takes at it serve every refresh interval from the route's first
  tick to its last, and no take made at it in the sitting misses an interval for a cause in the frame: both
  frame slots occupied, no drawable acquired, or a present call that came late or shared its interval. A take
  that misses only because the display link sent a callback late or not at all is the host's. It is kept,
  listed and made again.
- **The step chosen is the largest that holds, and every larger step is ruled out.** While steps are tried,
  the launch setting forces the step and the record says so. One take that misses an interval for a cause in
  the frame, on either route, rules its step out, by the definition above. For every step larger than the one
  chosen, the sitting's record holds such a take: 100%, 85% and 75% when 67% is chosen, and none when 100% is.
  A miss at one larger step says nothing of the others, because the governor chooses its own render scale and
  tier at each output scale, and the planner's fit, made at the governor's floor, puts those runs in no order.
  A larger step whose takes miss only for the host is tried again until it holds or misses for the frame.
- **The closing takes.** With the registry's value set to the chosen step and no launch setting at all, two
  takes on each route serve every interval. These four are what D17.2 judges, and they are made in the sitting
  on the build that merges.
- **Every take of the sitting is kept** and listed with its route, its step, whether a setting forced it, the
  host's load, the governor's trace and each missed interval's cause, so two that hold are not two chosen from
  many without that being seen.
- **If no step from 67% up holds**, the builder ends `NEEDS_CONTEXT` with the record. The owner chose from a
  measurement that pointed to about 75%, and a picture reconstructed at half the panel's size on each axis was
  not what was put to them. The lead takes it to the owner, and WP17 stays open until they answer.

What the sitting proves is that the engine made one present call in every 120 Hz refresh interval. When a frame
reached the panel is `SHOWCASE` WP8.6's to witness, from OS-reported presented intervals.

**The picture at the chosen scale.** D11.6's four cue windows are captured again, complete, and read as
[the evidence conventions](#evidence-conventions) say. The frames judged are the ones the closing takes'
configuration draws: the Apple recipe, MetalFX, EDR and the governor on, at the output scale the registry
resolves and a requested 120 Hz, with no launch setting forcing either. Each window's record names that
configuration, and gives every captured frame the render scale the governor held and the drawn extent it
gave, so a reader can tell how many pixels were drawn before reconstruction. A window captured with the
governor off, or at a forced scale, is not judged: it can draw every pixel of the output extent where the
governed game draws a quarter of them, and the two have the same size on the page. The scrub's comparison
capture is the one exception. It keeps the reference configuration D11.6 gave it, with reconstruction off, and
is read only for brightness at the same captured time. The same frames are also read resampled to the panel's
size with a linear filter, labelled a stand-in for the layer's scaling. Capture costs frames, so the governor
may hold a lower render scale in a window than in the closing takes, and these takes decide nothing about
pacing.

**The ledger and the milestone.** The maturity ledger's owed row is answered with what was held: one present
call in every 120 Hz refresh interval at full screen, at the output scale and output extent named, on the two
routes. Nothing in the ledger, the evidence page or the code still says that full screen asks for 60 Hz, and
nothing says the picture is drawn at the panel's pixel size. The milestone, Apple reconstruction, is reached
when WP17 is Done.

**The owner's observation.** No agent sees the panel, so how the scaled picture looks on it is the owner's
observation, as the look of EDR is ([below](#what-closes-g4-and-what-the-owner-observes-wp11)). WP17's evidence
page asks for it and says how to see the chosen step beside 100% with the launch setting. It gates nothing.

**The cost on record.** The pass table of the planner's record is taken again, on both routes, as a tripwire:
each pass's GPU time, the timed span against 8.33 ms, the governor's render scale and tier, and the host's
load. It sets no budget. A timing is taken while other builders work
([the settings](../../../ORCHESTRATE.md#targets)), and from the sitting until WP17 is Ready for Review they
are paused, so the record is made before the sitting, from the witness takes. For each step the sitting may
promote, 67% to 100%, the builder keeps one take on each route made while builders worked, with its pass
table. D17.5's record is the kept pair at the step the sitting then promotes. Its check reads those takes and
makes none, so it gives the same answer whenever it runs, the sitting included. The launch setting forced the
step in those takes, since the registry still held 100%, and the record says so. If the owner allows 50%, the
pair at 50% is taken when building resumes.

**The V1 exit.** The exit is reached through its milestones, and this one is reached at WP17, so the exit rests
on WP17 as it rests on WP11. `SHOWCASE` judges the exit's motion at the same bar, "smooth at the display's full
refresh" on the built-in panel ([its cue S5](../SHOWCASE/DESIGN.md#cues)), on the populated flythrough at the
output extent the game resolves. `SHOWCASE` WP7 needs only WP11. `SHOWCASE` WP8 depends on
`APPLE-PRESENTATION:WP17`, and its item WP8.12 names WP17 in its statement and its check, so an open WP17 keeps
the exit open ([`SHOWCASE`'s design](../SHOWCASE/DESIGN.md#the-exit-record)). WP17 in turn waits for WP18 and
for `ATMOSPHERE` WP12, so both are on the road to the exit.

### What closes G4, and what the owner observes (WP11)

G4 closes on evidence agents and the device produce. WP11 proves the brief's sentence clause by clause, as the
table says. The last row is the vision's bar for motion and not a clause of the sentence: the milestone is
reached when [WP17](#reaching-the-full-refresh-at-full-screen-wp17) has shown the display's full refresh held
at full screen, at [the output scale](#the-output-scale-wp18) its sitting chooses.

| The brief's clause | Proved by | What that evidence is |
|---|---|---|
| MetalFX through the native boundary | D8.1–D8.8 | Device function, records, and agents' review of motion |
| EDR | D6.1–D6.6, D6.8 | The layer's configuration, the transform, and the float values the surface was given, with agents' inspection of those captures. The real display record is a witness, and no pass depends on it |
| Binary archives | D12.1–D12.4 | A fresh-process run that is not allowed to fall back |
| The governor reacts to GPU, thermal and present evidence | D7.1–D7.9 | Policy tests and forced-load runs on the device |
| Apple tile compared with portable, with image tolerance and counters | D11.1, D11.4 | Paired rows on this Mac |
| No pipeline compilation during play | D9.2–D9.4, D11.2 | Creation counters by thread and phase |
| The whole path in motion | D11.6, D11.7 | Agents' review of realtime engine capture, and on the Mac the capture-off record of present calls in refresh intervals at [the requested frame rate](#the-requested-frame-rate-at-full-screen-wp11), 60 Hz at full screen as an interim rate |
| Smooth at the display's full refresh, at full screen | D17.2, D17.3 | Owed when WP11 ends, and recorded as owed by D11.5. Answered by WP17: the capture-off record of one present call in every 120 Hz refresh interval, at the output scale the record names, and agents' review of the cue windows at that scale. The picture is reconstructed below the panel's pixel size and scaled by the layer, by the owner's decision of 27 November |

Engine capture proves the values the surface was given, never the light the panel emits, and no agent can see
the panel. So the look of EDR on the built-in display is the owner's **observation**: experienced quality under
C12, which never gates ([brief](../VISION.md#decisions), decision 4). WP11's evidence page asks for it and says
how to see it, and the ledger carries its status, outstanding or answered in the owner's words, apart from what
was proved. No work package, G4's close included, waits for the reply. A reply that reports a defect is new work.
The physical iPhone and external-display rows stay owed in the same way.

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
- *Display (`synthetic/display.rs`, `synthetic_display.wgsl`, `present.rs`, `present.wgsl`).* G3t WP3 exposes the
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
- *Output scale change, entering or leaving full screen included:* a generation transition.
- *Refresh change:* governor budget only.
- *Requested frame rate change:* governor budget only, with an empty sample window, as after a hold.
- *Headroom change or a move to an SDR display:* H only.
- *Surface replacement:* G3b and frame-runtime recovery, then a transition if the extent changed.
- *Device loss:* recovery and a build.
- *Provider rejection:* the prepared TAAU twin.
- *Archive or metallib rejection:* a build-time compile or WGSL fallback, counted; never in play. An archive file
  the game's own check fails is rejected before wgpu is given its data.

**Cost account.** Output is 1920×1080 (the V1 window), unless stated.

| Item | Nominal cost |
|---|---|
| Pixel work at s = 1.0 vs today's 640×360 | 9× at the 1920×1080 window; 33× full screen on the 3456×2234 built-in display |
| Allocation at maximum scale | Depth 8.3 MB, HDR 16.6 MB, motion 8.3 MB, reactive 2.1 MB, TAAU linear depth 8.3 MB, history pair 2 × 16.6 MB, EDR drawable 16.6 MB (×3 swapchain) |
| Traffic at s = 0.67 (0.92 Mpx) | Attachment writes 0.44× native; reconstruction input reads about 21 B per render pixel (19 MB); history read and write 33 MB |
| Forward plus sky at s = 0.67 | About 0.85 ms if pixel-bound, from 1.94 ms native |
| Atmosphere | Unchanged: fixed-size LUTs |
| TAAU at 1080p output | Estimated 0.2–0.5 ms. MetalFX cost is unpublished. WP3 measures TAAU at its own 640×360 extent; WP5 (D5.7) measures it at the window's extent, once WP4 has given one; WP8 measures MetalFX. |

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
  counters by thread; a resize sequence; the reconfigure's wait on record at each switch.
- **The layer's scaling shows.** Early sign: a band at an edge, moiré on fine regular detail, or thin props
  that break up at full screen below 100%. Guard: the layer's bounds, gravity and filter on record (D18.3), and
  the resampled stand-in beside the captured frames (D17.3). The owner's look at the panel is an observation.
- **Something still sizes itself from the backing extent.** Early sign: a pass whose time does not fall with
  the output scale, or a picture drawn into a corner of the drawable. Guard: D18.3's extents and D18.4.
- **The output scale is resolved once and kept.** Early sign: a game launched in a window and taken to full
  screen keeps 100% and misses 120 Hz, or a window left from full screen stays soft. Guard: D18.5's mode
  change in the running game.
- **The promoted step holds the bare valley and not the showcase.** Guard: the sitting's second route.
- **The governor oscillates or hides regressions.** Guard: hysteresis plus a minimum period. Tripwires compare at a
  replayed scale trace.
- **EDR changes SDR appearance.** Guard: the H = 1 identity (D6.3) and the agents' inspection of the float
  captures (D6.8). The owner's look at the panel is an observation, and it gates nothing.
- **The pinned backend drifts from the release, or changes what the WGSL path does.** Guard: the pin check (D10.6), which
  rebuilds the copy from the release and the retained patches, and the exact baselines under `./dev check`.
- **The wgpu-core pin grows beyond the cache header.** Guard: the same pin check rebuilds that copy from its
  release and its one patch, and a reviewer reads the patch (D12.5).
- **Adding a third forward target breaks contributors.** Guard: SceneContribution and cooked-pipeline tests in G4 WP2.
- **The reactive target is declared but inert, or history advances before submission, on one target only.** Nothing
  visible changes until WP3. Guard: the temporal-input witness on the Mac and in Chrome.

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

The module list accepted seams 1–4 on 10 November and settled seams 5–9 on 14 November, in
[its rulings](../ARCHITECTURE.md#interfaces).

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
5. **Render extent and transitions.** The extent comes from the surface backing size times s, replacing 640×360.
   From WP18 the backing size is first multiplied by [the output scale](#the-output-scale-wp18), under the
   same ruling. A
   preparation thread builds generations. This fixes G3b's iOS band and moiré defects (frame runtime and G3b owners).
6. **Platform display record and EDR layer ownership.** The platform keeps a retained layer reference and sets EDR
   properties on the main thread. Headroom, refresh and backing scale are per surface (platform owner).
7. **Governor.** Location, inputs (missed ticks from the frame host, thermal state from the platform) and value
   commits inside a profile generation (C04).
8. **Native provider.** MetalFX through G3t's independently reviewed whole-encoder segment (depends on G3t WP11,
   and on G3t WP13 for [the provider interface](../APPLE-TILE/DESIGN.md#the-native-segment-and-its-providers-wp13)).
9. **Pipeline preparation and archives.** Owned by G4 WP10 and WP12 under the G3t boundary: the measured split; a
   pinned copy of wgpu-hal from WP10, whose first patch carries stage records for offline metallibs through
   passthrough and whose second (WP12) is the Metal `PIPELINE_CACHE`; from WP12 a pinned copy of wgpu-core,
   changed only in the cache header it writes and checks on Metal; upstream later, by the owner
   ([U1 to U3](#offering-the-cache-upstream-u1-to-u3)). V1's archive item stays as
   written.

## Open questions

- Does MetalFX beat TAAU in quality per millisecond on the valley at 0.67 and 0.5? Settled by WP11.
- Does tile traffic follow the drawn subrectangle at reduced scale? G4's paired proof reruns G3t WP12's fixed-extent
  baseline with real reconstruction readers and scale steps, where M0
  proved a capture method.
- Is the TAAU 1080p estimate right? WP3 records TAAU's time at its own 640×360 extent, kept apart from this
  estimate; WP5 (D5.7) records the 1080p figure, once WP4 has given the surface extent.
- Does MetalFX's history track a GPU-written exposure texture under a changing storage scale? Settled by WP8.

## Game outcomes

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape.

In the V1 game on this M3 Max:
- the valley fills the window at full backing resolution;
- MetalFX temporal reconstructs it on Apple tile, and TAAU on portable and in Chrome;
- stars, the Sun and reveals are stable;
- EDR highlights appear on the built-in XDR display within its headroom;
- the governor holds the display rate under forced load without oscillation;
- at full screen on the built-in display the game holds the display's full 120 Hz, with the picture
  reconstructed at the output scale WP17 measures and scaled to the panel by the layer. Until WP17 is Done it
  presents a steady 60 Hz there as an interim rate;
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
  recorded as sequences and judged by agents: the builder's evidence and the reviewer's verdict, with no human
  observer (owner, 18 November 2026: "agents can judge"). An agent reads frames and does not perceive playback. So
  where an item says to watch a sequence, in motion or at 1x, it reads the captured frames in their captured order
  and timing, none retimed or dropped, as runs of consecutive frames at native size across each cue. A defect
  counts when those frames show it: a copy or trail of a moving thing that stays behind it from frame to frame,
  pixels on a still feature that visibly change between consecutive frames, or brightness that trails the
  comparison capture at the same captured time. Each review says which frames it read for each cue, and that is
  its statement that it watched motion. Frame pacing is separate: where an item claims smoothness or no hitch,
  that is read from the capture's time records.
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
| History protocol | G4, with the C07 owner's review (seam 1) | `history.rs`, `runtime.rs` | The existing drop contract; G3t WP10–WP11 for native frames |
| Reconstruction | G4 | new `reconstruction/{mod,taau,jitter,exposure,reference}.rs`, `taau.wgsl` | Pre-tone linear input (G3t WP3) |
| Render extent and transitions | G4, with the frame-runtime and G3b owners' review (seam 5) | `runtime.rs:39`, `runtime/preparation.rs`, `render_actor.rs`, `examples/v1-game-ios/src/ios/render.rs:24` | G3t WP11 provider rebuild |
| Output scale | G4, under seam 5's ruling | The request and its resolution in `thrum-app/src/app/` (`app.rs`, `display_link.rs`, `render_actor.rs`), `runtime/extent.rs` and `runtime/extent_transition.rs`, the capability registry's row, the game's launch setting | Every reader of the generation's extent; engine capture's tap; `SHOWCASE`'s rows |
| EDR present | G4 | `present.rs`, `present.wgsl` | G3t WP3 pre-tone input; G4 owns EDR display math |
| Platform display record, EDR layer, thermal | G4, with the platform owner's review (seam 6) | `thrum-app/src/app/device.rs`, `app.rs`, `display_link.rs`; iOS `ios.rs`, `ios/render.rs` | G3b lease (untouched) |
| Governor and missed ticks | G4 (seam 7) | new `governor.rs`; `runtime.rs` (missed-tick count, value commit) | C04 |
| MetalFX provider | G4 | new `apple_tile/metalfx.rs` | G3t `apple_tile/segment.rs` |
| Pipeline preparation and archives | G4, with the asset-pipeline owner's review for the cook | metallib cook step, passthrough loader, the pinned wgpu-hal copy with its stage-record patch (WP10) and its Metal `PipelineCache` patch (WP12), the pinned wgpu-core copy with its cache-header patch (WP12) | G4 WP10 pipeline preparation; every pipeline site; every module's exact baselines, which the pin must not move |

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

| G4 work package | Can start when | Runs in parallel with |
|---|---|---|
| WP1 jitter, matrices, motion | G3t M0, atmosphere checkpoint, seams 2 and 4 | G3t WP1–WP3 |
| WP2 temporal inputs, history protocol | WP1, G3t WP1 (eligibility report), G3t WP2 (shares `runtime.rs`), seams 1 and 3 | G3t WP3/WP10–WP11 (sequential integration for shared files) |
| WP3 TAAU at native scale | WP2, **G3t WP3** (TAAU output feeds the retained pre-tone input) | G3t WP10–WP11 |
| WP4 surface extent and transitions | WP3, **G3t WP3**, seam 5 | G3t WP10–WP11 |
| WP5 dynamic scale | WP4 | WP6 |
| WP6 EDR on macOS | **G3t WP3**, seam 6 | WP5 |
| WP7 governor | WP5, seam 7 | WP6 |
| WP8 MetalFX temporal | **G3t WP11 and WP13**, seam 8, WP5 | WP9 |
| WP9 launch-to-ready split, prepared paths | D9.1 after G3t M0; the rest after WP4–WP8 | G3t WP12 |
| WP10 offline metallibs, pre-warm | D9.1 | WP2–WP9 (own files) |
| WP12 local archive | WP10 (the pinned backend) | none |
| WP11 paired proof | WP1–WP10 and WP12, G3t WP12 | none |
| WP18 output scale | WP11 | `ATMOSPHERE` WP12 (its own files, and the knob registry) |
| WP17 full refresh at full screen | WP18, `ATMOSPHERE` WP12, `SHOWCASE` WP7 | none: its closing takes are a sitting with the other builders paused |

The loop runs this module's work packages one at a time in [plan order](PLAN.md). The done-when file records
the outside waits: WP8 on `APPLE-TILE:WP11` and `APPLE-TILE:WP13`, WP11 on `APPLE-TILE:WP12`, and WP17 on
`ATMOSPHERE:WP12` and `SHOWCASE:WP7`.

**Handoffs from G3t**, with their contracts in [G3t's plan](../APPLE-TILE/DESIGN.md#sequence-and-dependencies):
- *G3t M0 → all checks.*
- *G3t WP1 → WP2.*
- *G3t WP3 → WP3/WP4/WP6.* One replaceable pre-tone scene-linear input carries matching exposure, extent and
  sample/generation identity. The existing SDR tone-to-Rgba8Unorm-to-present path and nearest mapping remain.
  G4 inserts TAAU/MetalFX there and owns the qualified EDR branch, headroom transform and transition filtering.
  The surface is bound at execute time; no fused-shader feasibility or traffic saving is inherited from G3t.
- *G3t WP11 and WP13 → WP8.* The first native handoff, unlocked by G3t D10.1–D11.3 and offered to a provider
  other than the witness by G3t WP13's `NativeProvider`.

G3t WP11 proves replacement with bounded provider fixtures; WP12 records current readers and fixed extents. Neither
waits for G4. G4's paired proof reruns eligibility with WP2's real reconstruction readers and traffic with WP5's actual
scale steps; these are G4 integration obligations, not backward G3t prerequisites.

WP1–WP3 are the early in-game work on portable through public interfaces. They need G3t's proof tooling and graph
work packages, not the boundary ruling.

## Twilight exposure regression (WP13)

Use the smallest scene and minimum accepted samples that carry the incident meter, exposure scale and
displayed luma through from twilight into night. Put the response arithmetic on the CPU where possible.
Keep a Metal submission witness for the actual metering and output handoff.
