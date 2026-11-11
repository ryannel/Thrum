# Reconstruction, display and governor plan (G4)

Design: [APPLE-PRESENTATION.md](APPLE-PRESENTATION.md), governed by [the G3t boundary decision](../decisions/G3T-METAL-BOUNDARY.md);
contracts [C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners). Paired plan:
[APPLE-TILE-PLAN.md](APPLE-TILE-PLAN.md).
Commission: Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).
Done when: [APPLE-PRESENTATION-DONE.json](APPLE-PRESENTATION-DONE.json) owns M1–M3. M4–M11 below are prose until
they are migrated. Module list and seam rulings: [ARCHITECTURE.md](ARCHITECTURE.md#interfaces).
Reviews: design review of M1–M3 at `4bbe44a4`: changes requested, one Must Fix (D2.5) · [review](../../evidence/APPLE-PRESENTATION/design-review.md). The design and items were revised the same day; a fresh design review is owed before M1 starts. Earlier reviews are in the log.
Milestone: M1 Jitter, matrices and motion (waiting on `APPLE-TILE:M0`) · Updated: 2026-11-10 (revised after the design review)
Now: M1–M3 revised after the design review; nothing built (since 2026-11-10)
Next: design review of the revision, which can approve M1–M3 and seams 1–4 only; M1 when `APPLE-TILE:M0` is Done; then a plan run migrates M4–M11, settles seams 5–9 and gets its own design review.
Blocked: M1 waits on `APPLE-TILE:M0`, M2 on `APPLE-TILE:M2`, M3 on `APPLE-TILE:M3`. Later waits are in the
[dependency table](#parallel-work-and-dependencies-on-g3t).

## Game outcomes

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

As in [G3t's conventions](APPLE-TILE-PLAN.md#proof-and-builder-handoff): fixed-step route, reference image, counters,
noise threshold and labels. Additions:
- **Reconstruction reference and tolerance.** [The design](APPLE-PRESENTATION.md#techniques-and-evidence) defines
  both, under *Reconstruction reference and tolerance*: a 32-sample reference averaged in scene-linear colour with
  the scene, time and exposure held and checked; CIEDE2000 on the submitted SDR output over the whole frame; p99 at
  most 3 (provisional, changed only by a plan revision) and lower than the no-AA frame's.
- **Temporal-input witness.** [The design](APPLE-PRESENTATION.md#techniques-and-evidence) defines rows W1–W7 under
  *The temporal-input witness*: one fixture read by the Mac tests and the Chrome case, with values taken from
  submitted frames.
- **Motion review.** Pans, translating lights, reveals behind a moving prop, a spawn, a time-of-day scrub and a cut,
  recorded as sequences and judged by agent review. Each review states that it watched motion.
- **Display overrides.** The only display is the built-in XDR panel (3456×2234, potential headroom 16). Rows for SDR
  displays, refresh changes and display moves use forced overrides of the display record, labelled as such. A
  physical external-display row is owed.
- **Sim identity.** World-state hash and tick count at route end, with the governor on and off.

## Ownership, call sites and review order

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

**Handoffs from G3t**, with their contracts in [G3t's plan](APPLE-TILE-PLAN.md#sequence-and-dependencies):
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

## M1 Jitter, matrices and motion: waiting on APPLE-TILE M0

Every M1 witness runs through the current portable output, offscreen, before G3t M3: the jitter-sequence and
record-layout unit tests (D1.1); motion-texture readback (it has `COPY_SRC`) for the static-scene zero-motion test
(D1.2) and the sky pan test (D1.3); and a motion mode in the existing `synthetic/display.rs` display pass for the route
review (D1.5). The first G4 witnesses that need G3t M3 are TAAU (M3) and the extent change (M4).

Done when:
- [ ] D1.1 The view record carries:
  - the jitter offset from a continuous 32-phase Halton(2,3) sequence;
  - jittered and unjittered matrices;
  - the previous submitted unjittered matrices;
  - the mip bias;
  - the subrectangle.

  A test checks the sequence, that (0,0) never occurs, and the record layout under the RENDER-SPEC change.
- [ ] D1.2 Motion uses unjittered clip positions: a static scene under jitter gives zero motion (test).
- [ ] D1.3 The sky, Sun disk and stars write motion at infinity, including the celestial phase delta. A pan test
      matches the expected sky motion within 0.05 px.
- [ ] D1.4 Objects with invalid history are marked, ready for M2's reactive target, instead of writing a fabricated
      zero vector.
- [ ] D1.5 A motion mode in the existing display pass shows motion on the valley route. Agent review of a pan sequence, watched in motion, finds
      object and sky motion in the expected direction.

Work:
- [ ] View-record fields and jitter sequence, with `pack_aux` carrying the subrectangle (→D1.1).
- [ ] Unjittered clip motion in `synthetic.wgsl` (→D1.2).
- [ ] Sky and star motion at infinity, delivered by the atmosphere owner under seam 4 (→D1.3).
- [ ] Invalid-history flag (→D1.4).
- [ ] Motion mode in `synthetic/display.rs` and the review sequence (→D1.5).

## M2 Temporal inputs and history protocol: waiting on APPLE-TILE M2

Done when:
- [ ] D2.1 With reconstruction enabled, depth and motion are stored and sampleable, and G3t's eligibility report shows
      them stored.
- [ ] D2.2 A third forward target, reactive `R8Unorm`, is declared by every forward drawer: synthetic,
      SceneContribution, cooked direct and indirect, the sky, and the separate star pass, whose attachments and
      graph accesses match its shader outputs. Their tests pass. It is written by:
  - invalid-history objects (1);
  - the cooked volume (its alpha);
  - stars and the Sun on cuts and phase edits only.
- [ ] D2.3 Each frame resolves a 1×1 `R16Float` exposure texture, and the history sample retains the storage scale p
      (test).
- [ ] D2.4 Submission-order acceptance:
  - a pipelined test with two frames in flight and a held submission shows the history sample advancing at each
    successful submission;
  - the previous matrices are the last submitted frame's;
  - a forced submission failure resets history;
  - a frame cancelled on failed acquisition leaves the history sample at the last submitted frame, and an iOS
    background drop resets history, both counted in the proof log;
  - `HistoryKey` uses the output extent.
- [ ] D2.5 In Chrome, on the game's own frame path, witness rows W1–W7 hold (`check_v1_browser.mjs --case
      temporal`).
- [ ] D2.6 On the Mac, witness rows W1–W5 hold, read back from the reactive target (W6 and W7 on the Mac are
      D2.4's pipelined tests).
- [ ] D2.7 The browser checker's existing cases still pass with the third target declared. Regression only.

Work:
- [ ] Sampleable depth and the motion reader in the graph (→D2.1).
- [ ] Reactive target across synthetic, contribution, cooked, sky and star-pass pipelines, with the star pass's
      attachments and graph accesses, under seam 3 (→D2.2).
- [ ] Exposure resolve and p retention (→D2.3).
- [ ] `history.rs` and `runtime.rs` acceptance changes and the pipelined tests, under seam 1 (→D2.4).
- [ ] The witness fixture definition, read by both targets, and the Mac GPU tests for W1–W5 (→D2.6).
- [ ] The frame host's per-frame history identity in the game page's report; a reactive view or attachment
      readback that reaches Chrome's submitted frames; the `temporal` case with its unsubmitted view (→D2.5).
- [ ] The existing browser cases rerun (→D2.7).

## M3 TAAU at native scale in game: waiting on APPLE-TILE M3

This is the first in-game reconstruction on portable. It runs at the current 640×360 extent, after G3t M3, so
TAAU's linear output feeds G3t's pre-tone input; the existing SDR transform still runs once.

Done when:
- [ ] D3.1 TAAU runs at scale 1.0 in the V1 game on Mac (forced portable) and in Chrome on the route. The present pass
      reads its output.
- [ ] D3.2 At still checkpoints, the design's 32-sample reference exists, with its held state checked. TAAU's ΔE
      p99 meets the reconstruction tolerance, and is lower than the no-AA frame's.
- [ ] D3.3 Agent review of motion sequences, watched in motion, finds no ghosting or crawl:
  - pans and translating lights;
  - a reveal behind a moving prop;
  - a spawn;
  - a held-camera star field (no shimmer);
  - a time-of-day scrub (no brightness lag).
- [ ] D3.4 A cut shows no prior history on the next frame (frame pair).
- [ ] D3.5 TAAU GPU time is recorded at M3's own 640×360 extent, kept apart from the design's 1080p estimate.

Work:
- [ ] `taau.rs` and `taau.wgsl`: dilation, YCoCg clip, luma weighting, reactive and depth rejection, linear-depth
      history, p rescale (→D3.1, D3.3).
- [ ] Registry-resolved `Reconstructor`, with TAAU as the default (→D3.1).
- [ ] `reference.rs`: the accumulation mode (the 32-phase cycle run twice, the two averages compared) and the
      CIEDE2000 comparison (→D3.2).
- [ ] Reset wiring (→D3.4).
- [ ] Timing row (→D3.5).

## M4 Surface-derived extent and generation transitions: pending

Done when:
- [ ] D4.1 The render extent derives from the surface backing size, with no letterbox band. On the iOS simulator the
      G3b route's portrait band and moiré are gone (function only).
- [ ] D4.2 During a live window drag-resize, frames keep presenting: the old generation is scaled into the new
      drawable.
  - The surface reconfigures at most once per tick, without waiting for GPU drain.
  - The new generation builds on the preparation thread and switches after the debounce.
  - A newer extent cancels a running build.
  - Counters show no texture or pipeline creation on the render thread.
  - Render-thread frame time shows no hitch beyond the tripwire.
- [ ] D4.3 A forced backing-scale change on the display record takes the same transition. History resets exactly at
      the switch.

Work:
- [ ] Surface-derived extent on macOS and iOS, removing the 640×360 default and the letterbox tests' fixed extent
      (→D4.1).
- [ ] Split `PreparedSynthetic` into a shared prepared part and per-generation state (→D4.2).
- [ ] Preparation thread, debounce, cancellation and boundary switch in `runtime/preparation.rs` (→D4.2).
- [ ] Coalesced reconfigure without drain in `render_actor.rs` and the iOS render owner, with G3b owner review
      (→D4.2).
- [ ] Old-generation scaling in the present pass (→D4.2).
- [ ] Forced backing-scale override and history-reset check (→D4.3).

## M5 Dynamic render scale: pending

Done when:
- [ ] D5.1 The scale steps through 1.0, 0.85, 0.75, 0.67, 0.5 on the route with TAAU. ΔE to the reference is recorded
      per step, and history does not reset on a step (frame-pair check).
- [ ] D5.2 Scale changes create no textures or pipelines (counters), and show no visible hitch in the recorded
      sequence.
- [ ] D5.3 Forward-plus-sky and TAAU GPU time follow drawn area as the cost account predicts, and atmosphere time stays
      flat. Departures are tripwires.

Work:
- [ ] Subrectangle viewport and scissor, TAAU input extent, mip bias from scale (→D5.1, D5.2).
- [ ] Scale-sweep rows (→D5.3).

## M6 EDR on macOS: pending

Done when:
- [ ] D6.1 On the built-in XDR display the surface runs `Rgba16Float` extended-linear.
  - The platform sets `preferredDynamicRange` and `contentsHeadroom` on the main thread through its own layer
    reference, without touching wgpu's layer mutex.
  - The capture variant shows the Sun disk, sunset sky and local lights above 1.0, capped at current headroom.
- [ ] D6.2 With H forced to 1, the EDR output equals the SDR path within encoding precision (test).
- [ ] D6.3 A forced SDR display record (potential headroom 1) keeps the EDR surface with H = 1, needs no reconfigure,
      and creates no pipeline. A session started with EDR unavailable selects the prepared 8-bit surface at its
      generation build.
- [ ] D6.4 A brightness change, or a forced headroom change, reaches H within one display tick, with no hitch.
- [ ] D6.5 The owner's visual check on the XDR display is requested and recorded, and the agent's review of captures
      is recorded. The iOS simulator runs the SDR path for function. A forced headroom value on the iOS display record
      reaches the render owner, exercising the polling path.

Work:
- [ ] Platform display record, retained layer reference, main-thread EDR properties and NSScreen reads (→D6.1, D6.4).
- [ ] iOS `UIScreen` polling into the record, kept away from the lease (→D6.5).
- [ ] Present EDR transform with the H = 1 identity, and the 8-bit twin (→D6.1–D6.3).
- [ ] Overrides, captures and the owner check request (→D6.3–D6.5).

## M7 Quality governor: pending

Done when:
- [ ] D7.1 The frame host counts display ticks without a present. A forced GPU load on the route (debug-load pass)
      makes the governor drop scale within its panic window, after which those ticks fall to their unloaded rate. Recovery is
      stepwise, at most one step per minimum period. The trace is recorded.
- [ ] D7.2 With no forced load at 120 Hz, the route shows at most 2 scale changes.
- [ ] D7.3 World-state hash and tick count are identical with the governor on and off.
- [ ] D7.4 A forced thermal-state change lowers the effect tier. Aerial LUT cadence steps switch without pipeline
      creation.

Work:
- [ ] Missed-tick accounting in the frame host (→D7.1).
- [ ] `governor.rs` policy and value commits (→D7.1, D7.2).
- [ ] Debug-load pass and sim-identity check (→D7.1, D7.3).
- [ ] Thermal input from the platform, and cadence steps (→D7.4).

## M8 MetalFX temporal: pending

This needs G3t M11 and independent review of its C06 whole-encoder segment.

Done when:
- [ ] D8.1 On Apple tile the MetalFX provider runs inside the native segment on the route, at scales 1.0 to 0.5.
  - It is created with `requiresSynchronousInitialization = YES` in generation builds.
  - Native-object creations appear only in build or transition phases.
  - No validation errors occur.
- [ ] D8.2 A translating-object test confirms the motion sign and scale and the jitter Y sign against TAAU. The exposure
      tool shows a constant mid-grey checkerboard.
- [ ] D8.3 A time-of-day scrub with the GPU-written exposure texture shows no brightness lag beyond TAAU's. If it
      does, the costed normalising pre-pass is applied, its GPU time recorded, and the check rerun.
- [ ] D8.4 At still checkpoints, ΔE to the reference is recorded next to TAAU's at each scale. Agent review of the M3
      motion set finds no ghosting beyond TAAU's, including a sequence across a governor scale step.
- [ ] D8.5 A forced provider rejection selects the prepared TAAU twin with no pipeline creation.
- [ ] D8.6 A failed acquisition discards the MetalFX frame with a provider reset at the next encode under G3t/C06;
      an iOS background drop resets history. Both are counted in the
      proof log next to portable's drop.

Work:
- [ ] `apple_tile/metalfx.rs`: descriptor, usages, per-frame properties, encode on the segment buffer, and reset
      wiring (→D8.1).
- [ ] Translating-object scene and exposure-tool run (→D8.2).
- [ ] Scrub sequence and fallback (→D8.3).
- [ ] Comparison rows and sequences (→D8.4).
- [ ] Rejection to the twin (→D8.5).

## M9 Launch-to-ready split and prepared paths: pending

Done when:
- [ ] D9.1 A planning baseline of launch-to-ready on this M3 Max, cold against warm, split into naga, MSL compile and
      pipeline state creation. It can run as soon as G3t M0's counters exist.
- [ ] D9.2 Rows each show the specified behaviour with zero render-thread creations in play:
  - surface replacement;
  - device-generation change;
  - forced refresh change, which changes only the governor budget;
  - forced display move, which changes only H;
  - resize during MetalFX.
- [ ] D9.3 Cold and warm launch-to-ready times are recorded, including synchronous MetalFX creation and wgpu pipeline
      preparation, with the post-readiness compile counter at zero.

Work:
- [ ] Baseline timing (→D9.1).
- [ ] Scenario switches in the route runner (→D9.2).
- [ ] Launch timing with cleared and warm OS caches (→D9.3).

## M10 Offline metallibs, pre-warm and the local archive: pending

Built and proved when G4 reaches this milestone. The listed points are designed then, not now (the G4 pipeline preparation boundary).

Done when:
- [ ] D10.1 *Offline metallibs:* route pipelines load through passthrough with zero WGSL fallbacks in a parity run,
      matching their WGSL images.
  - Reflection includes dynamic threadgroup allocation lengths and binding indices per entry point, proved by a real
    threadgroup-using dispatch and a missing-reflection refusal.
- [ ] D10.2 *Parallel pre-warm:* closure-time preparation creates pipelines across worker threads.
- [ ] D10.3 *Local archive qualification:* a shipped archive satisfies a strict warm witness.
  - The witness is a fresh process that uses the archive read-only (no prefill, write-back or fallback) over a manifest
    of required pipelines, and fails on an empty or incomplete archive, kept separate from harvest.
  - Stale-key, same-length corrupted payload and valid partial archive cases are each exercised, counted apart from
    device errors, with no compile in play.
- [ ] D10.4 Launch-to-ready is recorded on the final pipeline manifest for WGSL serial, metallib plus pre-warm, and the
      shipped archive, under stated cache conditions. This is an outcome record, not the archive correctness gate.

## U1 Upstream Metal pipeline cache (later, not V1-gating, if needed): pending

After M10 is proved. The owner writes every public description and comment, and nothing is posted without the owner's
OK on each post (the G4 pipeline preparation boundary).

Done when:
- [ ] U1.1 A change we can stand behind is coordinated on gfx-rs/wgpu#8526 and, with the owner's walkthrough and
      approval, submitted by the owner.

## M11 Paired proof run: pending

Done when:
- [ ] D11.1 Paired route rows compare Apple tile + MetalFX + EDR with portable + TAAU (SDR at H = 1) on this M3 Max,
      with a replayed scale trace. They record:
  - GPU pass and segment times;
  - missed ticks and present intervals;
  - the governor trace;
  - counters;
  - reference-image tolerance;
  - graph-declared and measured (or *unmeasured*) traffic at two scales;
  - the recipe and registry digest.
- [ ] D11.2 The Chrome portable run on the route passes functional review.
- [ ] D11.3 The iOS simulator runs reconstruction and the SDR path for function. The evidence states what iPhone EDR
      and timing are owed.
- [ ] D11.4 The MetalFX-vs-TAAU result is recorded as promotion evidence for the Apple reconstructor. Every open
      tripwire has an assigned fix.

Work:
- [ ] Route runs with a replayed scale trace (→D11.1).
- [ ] Chrome run (→D11.2).
- [ ] Simulator run (→D11.3).
- [ ] Evidence record `docs/evidence/render/G4-PAIRED-*.md`; maturity rows go to the ledger owner (→D11.1–D11.4).

## Sizing

| Milestone | Days |
|---|---|
| M1 | 1.5 |
| M2 | 3.5 |
| M3 | 1.5 |
| M4 | 2.5 |
| M5 | 1 |
| M6 | 1.5 |
| M7 | 1.5 |
| M8 | 1.5 |
| M9 | 1.5 |
| M10 | 6 |
| M11 | 1.5 |

That is **23.5 build days**, plus 1–2 days of review and seam turnaround. M2 grew by a day on 10 November for the
temporal-input witness on both targets.

**Combined schedule.** The earlier G3t 15.5-day / G4 22.5-day combined estimates predate the scoped re-plan and
are withdrawn. G4 can build against the named handoffs after delivery and approval; M4–M9 of G3t are not gates.
Fresh sizing belongs to G4's own re-plan. G3t M2/M3 runtime work, G4's history/third-target seams, extent changes and
native boundary remain risks; no delivery date follows from a documentation change. U1 remains outside V1 and
requires the existing owner authorization for each public post.

Revision 1 said 9.5–11.5 days for G4. Reviewers estimated 13–16 days.
