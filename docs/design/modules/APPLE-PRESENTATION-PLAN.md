# Reconstruction, display and governor plan (G4)

Design: [APPLE-PRESENTATION.md](APPLE-PRESENTATION.md), governed by [the G3t boundary decision](../decisions/G3T-METAL-BOUNDARY.md);
contracts [C04](../CONTRACTS.md#c04-feature-composition-and-quality--f2),
[C06](../CONTRACTS.md#c06-gpu-execution-and-lifetime--v1), [C07](../CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C12](../CONTRACTS.md#c12-budgets-evidence-and-acceptance--t2-with-all-owners). Paired plan:
[APPLE-TILE-PLAN.md](APPLE-TILE-PLAN.md).
Commission: Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).
Reviews:
- G4 pair @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/g4-pair.md` (with `common.md`), report
  `tmp/g3t-g4-plans/reviews/g4-pair-report.md`: changes required (14 required changes). Answered in revision 2; see
  the Log.
- Composed frame across G3t and G4 @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/composed-frame.md`, report
  `tmp/g3t-g4-plans/reviews/composed-frame-report.md`: changes required (14 required changes). Answered in
  revision 2.
- G4 pair re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (N1–N5 required,
  N6–N8 recommended). All answered in revision 3.
- Composed-frame re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (11 of 14
  answered; new N1–N9). Answered in revision 3.
- Composed frame, exact-revision recheck @ `9938b0d0` (Opus 5.5 medium), brief `tmp/g3t-g4-plans/reviews/composed-frame.md`,
  report `tmp/g3t-g4-plans/reviews/composed-frame-report.md` ("Revision 4 recheck"): approve with changes (one leftover
  always-submitted rule, three small gaps). All applied in revision 4a.
- Archive review of revision 5 @ `16c5d8f7` (Opus 5.5 medium), report `tmp/g3t-g4-plans/reviews/composed-frame-report.md`
  ("Revision 5 archive review"): changes required (sizes buffer for all runtime-sized arrays, per-pipeline cook keys,
  cache patch scope, per-device archives, sizing). All applied in revision 5a.
- Root's review of the archive revision @ `fe1749c1` (NOTES, "Root review of the archive revision at fe1749c1"): changes
  needed. Under the owner's direction its points are recorded as one-line outcomes in M10, to be designed when M10
  starts (revision 7).

Milestone: none started (plan awaiting re-review and seam rulings) · Updated: 2026-11-07
Now: revision 5 replaces the archive options with the owner's direction (since 2026-11-07).
Next: review of revision 5, Thrum Core's reading, then landing and seam rulings.
Blocked: code commissions wait for the atmosphere graph checkpoint `d2a5342` on `codex/g3s-stars-main-wip`, for interface reads only, pending the reviewed night baseline and the exact final check on main, and Thrum Core approval of the exact seams in the
design's *Seams* list. Some milestones have further waits:
- M4 and M6 wait for G3t M3.
- M8 waits for G3t M11 and G3t seam 1.
- D9.1's launch-to-ready split can be commissioned once G3t M0's counters exist, ahead of the rest of M9.
- The upstream contribution (U1) does not gate V1. It follows D9.1 and the metallib and pre-warm layer, and each
  public post waits for the owner's OK.

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

As in [G3t's conventions](APPLE-TILE-PLAN.md#evidence-conventions): fixed-step route, reference image, counters,
noise threshold and labels. Additions:
- **Reconstruction reference.** At each still route checkpoint, portable accumulates 64 jittered frames at native
  resolution with the camera held, using the M1 jitter sequence. The result is a converged supersample.
- **Reconstruction tolerance.** ΔE p99 to that reference must be at most 3 (provisional, restated after M3's first
  run). It must also be lower than the no-AA path's p99 at the same checkpoint.
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
| Reconstruction | G4 | new `reconstruction/{mod,taau,jitter,exposure,reference}.rs`, `taau.wgsl` | Present input (G3t M3) |
| Render extent and transitions | G4, with the frame-runtime and G3b owners' review (seam 5) | `runtime.rs:39`, `runtime/preparation.rs`, `render_actor.rs`, `examples/v1-game-ios/src/ios/render.rs:24` | G3t M11 provider rebuild |
| EDR present | G4 | `present.rs`, `present.wgsl` | G3t M3 present pass |
| Platform display record, EDR layer, thermal | G4, with the platform owner's review (seam 6) | `thrum-app/src/app/device.rs`, `app.rs`, `display_link.rs`; iOS `ios.rs`, `ios/render.rs` | G3b lease (untouched) |
| Governor and missed ticks | G4 (seam 7) | new `governor.rs`; `runtime.rs` (missed-tick count, value commit) | C04 |
| MetalFX provider | G4 | new `apple_tile/metalfx.rs` | G3t `apple_tile/segment.rs` |
| Pipeline preparation and archives | G4, with the asset-pipeline owner's review for the cook | metallib cook step, passthrough loader, wgpu-hal Metal `PipelineCache` patch | G3t seam 5; every pipeline site |

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
| M2 temporal inputs, history protocol | M1, G3t M1 (eligibility report), G3t M2 (shares `runtime.rs`), seams 1 and 3 | G3t M3–M9 |
| M3 TAAU at native scale | M2, **G3t M3** (TAAU output feeds the present pass) | G3t M4–M9 |
| M4 surface extent and transitions | M3, **G3t M3**, seam 5 | G3t M4–M10 |
| M5 dynamic scale | M4 | M6 |
| M6 EDR on macOS | **G3t M3**, seam 6 | M5 |
| M7 governor | M5, seam 7 | M6 |
| M8 MetalFX temporal | **G3t M11**, seam 8, M5 | M9 |
| M9 launch-to-ready split, prepared paths | D9.1 after G3t M0; the rest after M4–M8 | G3t M12 |
| M10 offline metallibs, pre-warm, local archive | D9.1 | M2–M9 (own files) |
| M11 paired proof | M1–M10, G3t M12 | none |

**Handoffs from G3t**, with their contracts in [G3t's plan](APPLE-TILE-PLAN.md#parallel-work-and-handoffs-with-g4):
- *G3t M0 → all checks.*
- *G3t M1 → M2.*
- *G3t M3 → M4 and M6.* This is the first producer/consumer handoff: the present pass reads one scene-linear input
  with exposure and H, and the surface is bound at execute time.
- *G3t M11 → M8.* The first native handoff, unlocked by G3t D10.1–D11.3.

G4 feeds back to G3t: M2's readers for G3t M12's eligibility rerun, and M5's scale steps for its two-scale rows.

M1–M3 are the early in-game work on portable through public interfaces. They need G3t's proof tooling and graph
milestones, not the boundary ruling.

## M1 Jitter, matrices and motion: pending

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

## M2 Temporal inputs and history protocol: pending

Done when:
- [ ] D2.1 With reconstruction enabled, depth and motion are stored and sampleable, and G3t's eligibility report shows
      them stored.
- [ ] D2.2 A third forward target, reactive `R8Unorm`, is written by:
  - invalid-history objects (1);
  - the cooked volume (its alpha);
  - stars and the Sun on cuts and phase edits only.

  The SceneContribution, cooked-asset and atmosphere pipelines declare it, and their tests pass.
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

Work:
- [ ] Sampleable depth and the motion reader in the graph (→D2.1).
- [ ] Reactive target across synthetic, contribution, cooked and sky pipelines, under seam 3 (→D2.2).
- [ ] Exposure resolve and p retention (→D2.3).
- [ ] `history.rs` and `runtime.rs` acceptance changes and the pipelined tests, under seam 1 (→D2.4).

## M3 TAAU at native scale in game: pending

This is the first in-game reconstruction on portable. It runs at the current 640×360 extent, after G3t M3, so
TAAU's output feeds G3t's present pass directly and nothing is reworked.

Done when:
- [ ] D3.1 TAAU runs at scale 1.0 in the V1 game on Mac (forced portable) and in Chrome on the route. The present pass
      reads its output.
- [ ] D3.2 At still checkpoints, the reference tool's converged image exists. TAAU's ΔE p99 meets the reconstruction
      tolerance, and is lower than the no-AA path's.
- [ ] D3.3 Agent review of motion sequences, watched in motion, finds no ghosting or crawl:
  - pans and translating lights;
  - a reveal behind a moving prop;
  - a spawn;
  - a held-camera star field (no shimmer);
  - a time-of-day scrub (no brightness lag).
- [ ] D3.4 A cut shows no prior history on the next frame (frame pair).
- [ ] D3.5 TAAU GPU time is recorded, as a record row against the design's estimate.

Work:
- [ ] `taau.rs` and `taau.wgsl`: dilation, YCoCg clip, luma weighting, reactive and depth rejection, linear-depth
      history, p rescale (→D3.1, D3.3).
- [ ] Registry-resolved `Reconstructor`, with TAAU as the default (→D3.1).
- [ ] `reference.rs`: the 64-frame accumulation mode (→D3.2).
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

This needs G3t M11 and seam 1.

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
- [ ] D8.6 A failed acquisition on a MetalFX frame shows the admitted outcome (submitted without present, or discarded
      with a MetalFX reset at the next encode), and an iOS background drop resets history. Both are counted in the
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

Built and proved when G4 reaches this milestone. The listed points are designed then, not now (G3t seam 5).

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
OK on each post (G3t seam 5).

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
| M2 | 2.5 |
| M3 | 1.5 |
| M4 | 2.5 |
| M5 | 1 |
| M6 | 1.5 |
| M7 | 1.5 |
| M8 | 1.5 |
| M9 | 1.5 |
| M10 | 6 |
| M11 | 1.5 |

That is **22.5 build days**, plus 1–2 days of review and seam turnaround.

**Combined date (G3t 15.5 + G4 22.5 build days, with the handoffs above).**
- *Two agents, one per plan:* about **23–24 working days** from seam approval to both paired records. G4 waits for
  G3t M3 around day 6 and for G3t M11 around day 13.
- *Four agents on disjoint files, following both parallel tables:* about **15–16 days**, with M10 on its own files.
- *Not in the date:* U1, the upstream contribution, is later and does not gate V1; its timing depends on #8526's
  review.
- *Largest date risks:* G3t M2–M3 (frame runtime), seam 1, the third forward target across three owners, and the
  render-extent change. That change raises pixel work 9× at the window and 33× full screen before the governor
  lands.

Revision 1 said 9.5–11.5 days for G4. Reviewers estimated 13–16 days.

## Log

- 2026-11-07 Revision 1 drafted for Thrum Core's G3t/G4 commission.
- 2026-11-07 Revision 2 answers the G4 pair review and the composed-frame review:
  - **History:** submission-order acceptance, with encoded frames always submitted; output-extent history keys, so
    scale steps need no reset.
  - **Motion:** sky, Sun and star motion at infinity; disocclusion through invalid-history reactive and TAAU depth
    rejection.
  - **MetalFX:** synchronous initialization; `preExposure` and the history domain under the storage scale.
  - **Forward targets:** the reactive target and mip bias named as a contributor seam.
  - **Transitions:** generation builds off the render thread, with the old generation scaled meanwhile.
  - **EDR:** one EDR surface across displays with H = 1 on SDR displays; platform-owned display record and layer
    properties off wgpu's mutex.
  - **Governor:** missed ticks from the display link, plus thermal state.
  - **Checks:** no FXAA comparison; a defined reference and tolerance; display overrides; record-only checks
    labelled.
  - **Archives:** the option-(b) consequence stated, with a conditional M10.
  - **Cross-plan:** dependencies corrected; milestones split; sizing restated.
- 2026-11-07 Revision 3 answers the G4 re-review:
  - MetalFX gets a GPU-written exposure texture, with a costed normalising fallback, instead of a CPU `preExposure`;
  - late GPU execution errors take the device-loss path;
  - the star motion target is replace, and solar change in the sky is a phase edit;
  - the TAAU depth test compares previous-view depths;
  - G4 M3 is sequenced after G3t M3;
  - ticks without a present are reworded;
  - M2 and M4 are resized;
  - effect-tier invalidation is declared;
  - the iOS polling is checked;
  - the upstream contribution is owner-gated.
- 2026-11-07 Revision 3 also answers the composed-frame re-review:
  - coalesced surface reconfigure without drain;
  - the `PreparedSynthetic` split, build cancellation and device loss during a build;
  - the governor freezes during transitions;
  - ticks without a present are labelled a proxy;
  - occlusion stops encoding;
  - an 8-bit session gains EDR through a transition;
  - a MetalFX scale-step sequence is added.
  - Its exposure-domain finding is moot: MetalFX no longer takes `preExposure`, and the exposure texture is tone
    exposure divided by p.
  - Its TAAU depth finding is answered by the previous-subrectangle addressing.
- 2026-11-07 Revision 4 answers Thrum Core's review of revision 3:
  - M1's witnesses all run through the current portable output before G3t M3, including a motion mode in the
    existing display pass;
  - the history protocol keeps the existing drop contract; MetalFX frames follow G3t seam 3, and both outcomes and
    the iOS background refusal are observable (D8.6);
  - archives are an owner ruling, with no V1 exit wording change;
  - the atmosphere checkpoint is `d2a5342`, for interface reads.
- 2026-11-07 Revision 4a applies the composed-frame recheck of `9938b0d0`: the leftover always-submitted rule is
  replaced, MetalFX history advances at encode, and D2.4 covers the portable drop and the iOS background drop.
- 2026-11-07 Revision 5 follows the owner's archive direction (overseer, 22:20):
  - the (b) rewording option is withdrawn, and V1's archive item stays as written;
  - M9 starts with a measured cold/warm split of launch-to-ready into naga, MSL compile and pipeline state creation;
  - M10 delivers offline metallibs through `PASSTHROUGH_SHADERS` and a Metal `PIPELINE_CACHE` carried as a pinned patch
    and contributed upstream under gfx-rs/wgpu#8526, with the contribution itself waiting for the owner's approval;
  - sizing is restated.
- 2026-11-07 Revision 5a applies the archive review of `16c5d8f7`: per-pipeline cook keys, `MetalLib` reflection for
  the sizes buffer, the full Metal cache patch scope and per-device archive sources. M10 becomes 5.5 days.
- 2026-11-07 Revision 5b adds the overseer's 22:30 points: parallel pipeline pre-warm (D9.4), the cook's MSL match
  check against wgpu-hal's runtime MSL, and the order measure → metallibs plus pre-warm → Metal `PIPELINE_CACHE`.
- 2026-11-07 Revision 6 moves the upstream Metal `PIPELINE_CACHE` contribution out of M10 into U1, a later item that
  does not gate V1, with the owner's rules (coordination first, Metal only on trunk, GPU-suite tests, `cargo xtask
  test`, review, owner walkthrough, owner-written posts). M10 keeps the pinned patch for V1's archives.
- 2026-11-07 Revision 7 follows the owner's direction to build the archive work when G4 reaches it: seam 5, M10 and U1
  are kept at outcome level, with the four pieces distinct and Root's fe1749c1 points as one line each, to be designed
  when M10 starts.
