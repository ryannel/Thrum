# APPLE-PRESENTATION M4–M11: milestones as drafted, awaiting migration

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. M4–M11 and U1 have no entry in the done-when file. The plan run that migrates them, named in [the module list](../../plans/ARCHITECTURE.md#apple-presentation-m4m11-g4), turns each `Done when` list below into items as written and settles seams 5–9. Until then no builder starts one. The sections are unchanged; the commission was Thrum Core's G3t/G4 design and milestone plans (`tmp/g3t-metal-boundary/NOTES.md`).

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
