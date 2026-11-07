# Reconstruction, display and governor plan (G4)

Design: [APPLE-PRESENTATION.md](APPLE-PRESENTATION.md), governed by [the G3t boundary decision](../decisions/G3T-METAL-BOUNDARY.md);
contracts [C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners). Paired plan:
[APPLE-TILE-PLAN.md](APPLE-TILE-PLAN.md).
Commission: Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).
Reviews: pending.
Milestone: none started (plan awaiting review) · Updated: 2026-11-07
Now: design and plan drafted for review (since 2026-11-07).
Next: independent pair review and composed-frame review; then Thrum Core's seam rulings.
Blocked: code commissions wait for the atmosphere graph checkpoint and Thrum Core approval of the exact seams in the
design's *Seams* list. M6 also waits for G3t M6 and G3t seam 1, and M7's archive rows wait for the owner's ruling
on archives.

## Game outcomes

In the V1 game on this M3 Max:
- the valley fills the window at full backing resolution, reconstructed by MetalFX temporal on Apple tile or by TAAU
  on portable and in Chrome;
- EDR highlights appear on the built-in display within its headroom;
- the governor holds the display rate under forced load with no oscillation;
- the simulation is unchanged.

Every prepared path (cut, resize, dropped frame, display change, surface replacement, provider or archive rejection,
EDR unavailable) runs with no pipeline creation during play. The paired proof compares Apple tile + MetalFX + EDR
with portable + TAAU on the same machine. The iOS simulator shows reconstruction and SDR fallback for function only.
EDR and timing on a physical iPhone are owed.

## Evidence conventions

The conventions are the same as [G3t's](APPLE-TILE-PLAN.md#evidence-conventions): the M0 route, paired rows, ΔE
tolerance, separate traffic records and proxy labels. Additions:
- **Reconstruction reference.** A native-resolution, 8×-supersampled still at each route checkpoint, with the camera
  held for 64 frames so that history converges, rendered on portable. Moving checkpoints compare against TAAU at
  1.0, and ghosting is judged on recorded sequences by agent review.
- **Counters.** Pipeline creations and texture creations after readiness, from G3t M6's counter extended to textures.
- **Sim identity.** The world-state hash and tick count at the end of the route, recorded with the governor on and
  off.

## Ownership, call sites and review order

| Area | Owner after approval | Files (current → proposed) | Shared call sites |
|---|---|---|---|
| Frame contract | G4, with the C07 and atmosphere owners' review | `synthetic/view.rs`, `synthetic.wgsl`, `synthetic.rs` (forward targets), `history.rs`, `runtime.rs` (`HistoryKey` wiring) | Atmosphere star pass (shared jitter), G3c Hi-Z (depth reader), G3t M2 merged graph |
| Render extent from surface | G4, with the frame-runtime and G3b owners' review | `runtime.rs:39`, `examples/v1-game-ios/src/ios/render.rs:24`, `present.rs` letterbox | macOS and iOS surface configuration |
| Reconstruction | G4 | new `thrum-render/src/reconstruction/{mod,taau,jitter,exposure}.rs` and `taau.wgsl` | G3t's merged graph (present reads its output) |
| MetalFX provider | G4 | new `thrum-render/src/apple_tile/metalfx.rs` | G3t `apple_tile/segment.rs` `NativeProvider` |
| EDR present | G4 | `present.rs`, `present.wgsl`, tone code moved there by G3t M2 | RENDER-SPEC tone section |
| Platform EDR and thermal | G4, with the platform owner's review | `thrum-app/src/app/device.rs`, `app.rs`, `render_actor.rs`; iOS `ios.rs` and `ios/render.rs` | Surface configuration call sites `app.rs:446`, `render_actor.rs:255`/`:375` |
| Governor | G4 | new `thrum-render/src/governor.rs`; `FrameHost` profile commit in `runtime.rs` | C04 profile transactions |
| Archives | G4 | new `thrum-render/src/apple_tile/archive.rs` | G3t pipeline-creation counter |

Review order:
1. Frame contract.
2. Render extent.
3. TAAU.
4. Dynamic scale.
5. EDR present and platform.
6. Governor.
7. MetalFX provider.
8. Archives.

Steps 1–4 follow G3t's merged-graph review (G3t step 3) when that has landed. Otherwise they land on the current
tone pass and G3t M2 rebases them.

## Parallel work and dependencies on G3t

| Work | Can start when | Runs in parallel with |
|---|---|---|
| M1 frame contract | Atmosphere graph checkpoint plus seams 1 and 3 approved | G3t M1–M2 |
| M2 TAAU at 1.0 | M1; G3t M2 preferred first | G3t M3–M5 |
| M3 dynamic scale | M2 | G3t M3–M6, G4 M4 |
| M4 EDR on Mac | Seams 2 and 4 approved; G3t M2 (present owns tone) | M2, M3 |
| M5 governor | M3 and seam 5 approved | M4, G3t M6 |
| M6 MetalFX temporal | **G3t M6 closed** plus G3t seam 1 approved, and M3 | M7 |
| M7 prepared paths, archives, cold start | M4–M6; archive rows wait for the owner's ruling | G3t M7 |
| M8 paired proof | M1–M7 and G3t M7 | none |

**First G4 milestone needing G3t's handoff: M6.**
- *Producer:* G3t M6's native segment and `NativeProvider`.
- *Consumer:* the G4 MetalFX provider.
- *Contract:* as stated in [G3t's plan](APPLE-TILE-PLAN.md#parallel-work-and-seams): created at closure per
  generation; encodes only graph textures into a graph texture inside the segment's raw command buffer; completes
  with the slot's submission; names a prepared portable twin (here TAAU).
- *Unlocking proof:* G3t D6.1–D6.3 pass on this M3 Max with the stand-in provider.

M1–M5 run on portable through public interfaces and need nothing from G3t beyond the shared files, so the early
in-game milestone does not wait on the boundary ruling.

## M1 Frame contract in the live game: pending

Done when:
- [ ] D1.1 The render extent derives from the surface backing size: the drawn area matches the surface aspect with no
      letterbox band. On the iOS simulator the G3b route's portrait band and nearest-filter moiré are gone (function
      only).
- [ ] D1.2 The view record carries a Halton(2,3) jitter offset, jittered and unjittered matrices, the previous
      unjittered matrix of the last accepted sample, and the mip bias. A test checks the phase sequence, that (0,0) is
      never produced, and that motion stays unjittered: a static scene gives zero motion under jitter.
- [ ] D1.3 With reconstruction enabled, depth and motion are stored and sampleable, and G3t's eligibility report
      shows them as stored. A reactive `R8Unorm` target is written by the Sun disk and star passes. A 1×1 `R16Float`
      exposure texture is resolved from the metered sample each frame.
- [ ] D1.4 `HistoryKey` and `ViewHistory` are wired in the runtime. Tests cover a cut, a resize, a device-generation
      change and a dropped frame: the accepted sample does not advance, and the next frame's previous matrix is that
      sample's.
- [ ] D1.5 A debug view shows motion, the reactive mask and depth on the valley route. Agent review of the motion
      view in a pan matches the expected direction.

Work:
- [ ] Surface-derived render extent on macOS and iOS; remove the fixed 640×360 default (→D1.1).
- [ ] Jitter sequence and view-record fields; WGSL use of jittered and unjittered matrices; `textureSampleBias`
      plumbing (→D1.2).
- [ ] Forward-target changes: sampleable depth, reactive target, exposure resolve pass (→D1.3).
- [ ] `HistoryKey`/`ViewHistory` wiring in `runtime.rs` and its tests (→D1.4).
- [ ] Debug view selector and review sequence (→D1.5).

## M2 TAAU at native scale in game: pending

This is the early in-game milestone on portable, through public interfaces.

Done when:
- [ ] D2.1 TAAU at scale 1.0 runs in the V1 game on Mac (forced portable) and in Chrome on the valley route. The
      present pass reads its output.
- [ ] D2.2 At still checkpoints, ΔE against the 8× reference is within tolerance, and lower than the FXAA path's.
- [ ] D2.3 A pan sequence and a translating-light sequence pass agent review with no ghosting or crawl on the terrain
      silhouette, the grid and thin props. Stars and the Sun disk leave no trail.
- [ ] D2.4 A cut and a resize show no prior history on the next frame (captured frame pair).
- [ ] D2.5 The TAAU GPU time at the window's extent is recorded and compared with the design's estimate.

Work:
- [ ] `reconstruction/taau.rs` and `taau.wgsl`: dilation, YCoCg clip, luminance weighting, reactive weighting, history
      pair (→D2.1, D2.3).
- [ ] Registry-resolved `Reconstructor` with TAAU as the default (→D2.1).
- [ ] Reference capture mode and checkpoint comparison (→D2.2).
- [ ] Reset wiring from the frame contract (→D2.4).
- [ ] Timing row (→D2.5).

## M3 Dynamic render scale: pending

Done when:
- [ ] D3.1 The render scale steps through 1.0, 0.85, 0.75, 0.67, 0.5 on the route with TAAU, and ΔE at still
      checkpoints is recorded for each step.
- [ ] D3.2 Scale changes during play create no textures and no pipelines (counters read zero), and show no visible
      hitch in the recorded sequence.
- [ ] D3.3 Forward and atmosphere GPU time scale with drawn area as the cost account predicts. Any departure is
      recorded as a tripwire.

Work:
- [ ] Allocate at maximum scale; draw into a viewport and scissor subrectangle; pass the TAAU input extent (→D3.1,
      D3.2).
- [ ] Phase count from the scale; mip bias from the scale (→D3.1).
- [ ] Texture-creation counter alongside G3t's pipeline counter (→D3.2).
- [ ] Scale-sweep rows (→D3.3).

## M4 EDR on macOS: pending

Done when:
- [ ] D4.1 On the M3 Max built-in display (potential headroom 16) the surface runs `Rgba16Float` extended-linear. The
      Sun disk, sunset sky and local lights exceed 1.0 in the drawable, capped at the current headroom. The SDR UI
      stays at or below 1.0.
- [ ] D4.2 With H forced to 1, the EDR path's output equals the SDR path's within encoding precision (test).
- [ ] D4.3 On a display or setting with potential headroom below 1.5, the prepared SDR surface and tone path run with
      no pipeline creation.
- [ ] D4.4 A headroom change (brightness change, or a window move between displays) changes H within one display
      tick with no hitch.
- [ ] D4.5 The owner's visual check on the M3 Max display is requested and recorded. The agent's own review of
      tone-mapped captures is recorded. The iOS simulator runs the SDR fallback for function.

Work:
- [ ] Platform: colour-space choice, `contentsHeadroom` through `Surface::as_hal` on the main thread, headroom sampling
      and atomic, on macOS and the iOS shell (→D4.1, D4.4, D4.5).
- [ ] Present: `Rgba16Float` path and headroom tone transform with the H = 1 identity (→D4.1, D4.2).
- [ ] Prepared SDR twin and selection (→D4.3).
- [ ] Captures, headroom-change row, owner check request (→D4.4, D4.5).

## M5 Quality governor: pending

Done when:
- [ ] D5.1 On the route with a forced GPU load (a debug-load pass), the governor lowers scale within its panic window,
      and missed presents stop. When the load lifts it recovers stepwise, with no more than one change per minimum
      period. The scale trace is recorded.
- [ ] D5.2 With no forced load at 120 Hz, the scale makes no more changes than the hysteresis allows over the route.
- [ ] D5.3 The world-state hash and tick count are identical with the governor on and off.
- [ ] D5.4 Effect-cadence steps (aerial LUT cadence) are prepared and switchable without pipeline creation.

Work:
- [ ] `governor.rs` policy: budget, history window, change threshold, minimum period, panic, thermal input (→D5.1,
      D5.2).
- [ ] Profile transaction commit in `FrameHost` (→D5.1, D5.4).
- [ ] Debug-load pass and sim-identity check in the runner (→D5.1, D5.3).
- [ ] Cadence step for the aerial LUT (→D5.4).

## M6 MetalFX temporal: pending

This needs G3t M6.

Done when:
- [ ] D6.1 On Apple tile the MetalFX temporal provider runs inside the native segment on the route at scales 1.0 to
      0.5. The scaler is created at closure (none in play), and no validation errors occur.
- [ ] D6.2 A translating-object test confirms the motion sign and scale against TAAU. The exposure debug tool shows a
      constant mid-grey checkerboard.
- [ ] D6.3 At still checkpoints, ΔE against the 8× reference is recorded next to TAAU's at each scale. Agent review
      of pan sequences finds no ghosting beyond TAAU's.
- [ ] D6.4 A forced provider rejection switches to the prepared TAAU twin with no pipeline creation.

Work:
- [ ] `apple_tile/metalfx.rs`: descriptor, per-frame properties, `encode(commandBuffer:)` on the segment buffer
      (→D6.1).
- [ ] Translating-object scene and the exposure-tool run (→D6.2).
- [ ] Comparison rows and sequences (→D6.3).
- [ ] Rejection wiring to the TAAU twin (→D6.4).

## M7 Prepared paths, archives and cold start: pending

Done when:
- [ ] D7.1 Rows for a window move between displays (refresh and headroom change), a resize, a surface replacement
      and a device-generation change each show a new generation prepared before switching, the correct history reset,
      and zero pipeline and texture creations during play.
- [ ] D7.2 Cold and warm launch-to-ready times are recorded, including MetalFX scaler creation and wgpu pipeline
      preparation.
- [ ] D7.3 Archive rows follow the owner's ruling on archives.
  - Under option (b), the rows record that wgpu pipelines are prepared at closure with the OS shader cache, and that
    the post-readiness compile counter reads zero.
  - Under (a), or for native provider pipelines, an archive hit is proven with `failOnBinaryArchiveMiss`, and a
    stale or rejected archive compiles at closure, not in play.

Work:
- [ ] Display-change and replacement scenarios in the runner (→D7.1).
- [ ] Launch-to-ready timing for cold runs (cache cleared) and warm runs (→D7.2).
- [ ] `apple_tile/archive.rs` for native pipelines, or the recorded option-(b) result (→D7.3).

## M8 Paired proof run: pending

Done when:
- [ ] D8.1 The paired route rows (Apple tile + MetalFX + EDR vs portable + TAAU + SDR, same M3 Max, same scale trace
      replayed) record GPU times, present intervals, missed presents, governor trace, counters and image tolerance
      against the reference. Each row names the recipe and registry digest.
- [ ] D8.2 The Chrome portable run on the same route passes functional review.
- [ ] D8.3 The iOS simulator runs reconstruction and the SDR fallback for function. The evidence states that iPhone
      EDR and timing are owed.
- [ ] D8.4 No tripwire is open, or each open one has an assigned fix. The MetalFX-vs-TAAU result is recorded as the
      promotion evidence for the Apple recipe's reconstructor.

Work:
- [ ] Route runs with a replayed scale trace (→D8.1).
- [ ] Chrome run (→D8.2).
- [ ] Simulator run (→D8.3).
- [ ] Evidence record `docs/evidence/render/G4-PAIRED-*.md`; maturity rows go to the ledger owner (→D8.1–D8.4).

## Sizing

| Milestone | Days |
|---|---|
| M1, M2, M3, M5, M6, M8 | about 1 each |
| M4 (two platforms, main-thread layer work and an owner check) | 1.5 |
| M7 | 1 |
| Reviews and seam turnaround | 1–2 |

That is about **9.5–11.5 days** for G4. M4 can run beside M2–M3, and M6 must wait for G3t M6. With G3t and G4 run by
separate agents and the parallel tables followed, the combined elapsed time is about **11–13 working days** from
seam approval to both paired records. The largest risks to that date are G3t M2 (frame runtime), the native-segment
ruling, and the render-extent change, which raises pixel work 4–9× before the governor lands.

## Log

- 2026-11-07 Plan drafted for Thrum Core's G3t/G4 commission.
