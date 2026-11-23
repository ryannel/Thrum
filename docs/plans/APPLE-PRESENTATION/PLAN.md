# APPLE-PRESENTATION plan

Reconstruction, display and the quality governor (gate G4), governed by
[the G3t boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07 and C12.
[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list and seam rulings](../ARCHITECTURE.md#interfaces) · paired plan [APPLE-TILE](../APPLE-TILE/PLAN.md)

For M4 and later read [M4 to M12, settled](DESIGN.md#m4-to-m12-settled-14-november-2026) and the rulings for
seams 5–9 before starting. The milestones run in the order below: M12 comes before M11.
New proof arms go in `scripts/apple_presentation.py` and new browser cases in
`scripts/check_v1_browser.mjs`. Nothing that needs a window, Chrome or the simulator runs under `./dev check`.

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the Chrome witness covers the same rows as the Mac, the volume and phase-edit rows included. The review required only the invalid-history rows and the history identity in Chrome; the fixture is shared, and Chrome is where cloud and sunset demos will run.
- Assumption: ΔE is CIEDE2000 on the submitted 8-bit SDR output, since reconstruction is judged as displayed. The bound stays at 3 and binds until a plan revision changes it. This replaces the earlier assumption that the bound is "restated after M3's first run".
- Assumption: the per-frame history identity reaches Chrome through the game page's own report, so T2's capture record is not changed. A builder who needs a T2 record change proposes it here first.
- Assumption: seams 1–4 of the design are accepted as written, with the G4 builder making the motion-only edit in the atmosphere shaders (seam 4) and the third forward target across contributors (seam 3) itself.
- Assumption: the numbers the drafts carried stay as drafted: a 100 ms resize debounce, three frames over budget before the governor drops scale, one recovery step a second, and at most two scale changes on an unloaded route. None came from you, and none is a frame-time gate.
- Assumption: from M4 the game draws at the window's full pixel size, three milestones before the governor exists (M7). At full screen the frame rate may dip in between. Times are recorded, not gated.
- Assumption: taking a new window size costs one short wait for the GPU, at the moment the picture switches and never while the edge moves, because released wgpu waits inside its surface call. Removing that wait means a patched copy of wgpu-core as well, from M4, or the engine owning the drawables beside wgpu. I chose neither.
- Assumption: on the iPhone a rotation holds the last picture, fitted whole with the background at its sides, until the first frame at the new size. Only the Mac keeps drawing through a resize, because the iPhone's surface rules refuse old-size frames and were to stay as they are.
- Assumption: a patched copy of wgpu's Metal backend (wgpu-hal 30.0.1) is kept in this repository from M10, one milestone earlier than planned, because the released backend cannot run shaders compiled ahead of time; M12 adds the archive to the same copy. Nothing is posted upstream; that stays yours (U1).

## M1 Every pixel of the valley carries correct motion under jitter, including the sky and stars: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M1-review.md)

## M2 Depth, motion, reactive and exposure are stored for reconstruction, and history advances only with submitted frames: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M2-review.md)

## M3 TAAU reconstructs the valley at native scale in the game, on the Mac and in Chrome: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M3-review.md)

## M4 The valley fills the window at any size, and keeps presenting while the window is resized

[Review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4-review.md)

## M14 The retained native valley images reflect the corrected material mip bias

`32dfd3c14` restored the designed mip bias of −1 at native width with reconstruction off. Its parent
`f582a2af1` matched all five portable SDR checkpoints. At `approach`, the correction changed 99,274 pixels
and produced the same bytes as the later failing capture. Restoring only zero bias recovered all five
byte matches with the cloud depth correction still in place.
[Attribution record](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14-attribution.json) ·
[M5 review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M5-review.md)

Refresh the affected native references under the [mip-bias ruling](../ARCHITECTURE.md#ruling-on-native-mip-bias)
before M13. Compare registered captures against a matched native spatial zero-bias control to determine
which references the correction moves. Record every replaced reference's old and new SHA-256 in both
modules' logs and qualify the refresh as described in the ruling.

## M13 A small exposure check catches a twilight brightness rebound

Use the [fixture design](DESIGN.md#twilight-exposure-regression-m13) for the cases in the [case list and baseline times](LOG.md#m13-workspace-timing-baseline).

Record each case’s disposition, any replacement coverage and its before-and-after time in the workspace
check’s default Nextest build on the Mac, excluding compilation. Measure the fast cases alone and in a
focused batch under normal builder load. The batch passes the existing ten-second limit on its first run.

## M5 Render scale steps during play with no hitch and no history reset

`pack_aux` already carries the subrectangle (M1), and TAAU is M3's. The route's scale trace is scripted here; the
governor that will drive it is M7.

## M6 Highlights go brighter than SDR white on the built-in XDR display

The SDR curve is `crates/thrum-render/src/synthetic/display/neutral_highlights.wgsl`; the design gives the EDR
form of it. The pre-tone input and its exposure are `PreToneInput` in `synthetic/display.rs`, which `APPLE-TILE`
M3 delivered. The platform side is `crates/thrum-app/src/app/device.rs`, `app.rs` and `display_link.rs`.
Engine capture's tap is `T2-CAPTURE`'s, extended here under the ruling for seam 6 to read the float target; its
8-bit records must not change. Proofs force the display record and keep the real one as a witness, so they pass
with the display asleep.

## M7 The governor holds the display rate under forced load, then recovers step by step

The aerial lookup's cadence knob is built in `ATMOSPHERE`'s files under the ruling for seam 7 if that module has
not declared one; its items do not change. `WATERS` and `CLOUDS` register their own knobs later through the same
interface, so keep it free of any module's name. Frame samples and pass timing are the frame runtime's feed
(`timing.rs`), which `CREATOR`'s profiler also reads: leave its timing-demand switch working. The generation
build's lifecycle in the frame host is M4's; marking held frames on it is this milestone's, and D4.5's test
already forces the failed build D7.9 needs.

## M8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback

`APPLE-TILE` M10 and M11 left the native segment, its provider interface and the replacement rules in
`crates/thrum-render/src/apple_tile`; the scaler is a provider there. Take D8.3's noise floor first, on two
identical runs, since the exposure judgement is made against it. Build the normalising pre-pass only if the
exposure texture alone fails D8.3.

## M9 Surface, device and display changes create nothing during play, and launch-to-ready time is on record

The creation counters are `runtime/creation.rs`, behind the `creation-counters` feature. D9.1's split is the
baseline M10 and M12 are judged against, so record it before changing how anything is prepared.

## M10 Shaders compiled ahead of time load at launch with no WGSL translation, and pipelines are prepared in parallel

wgpu is 30.0.1 from crates.io with no patch today; this milestone brings in the pinned copy of wgpu-hal, which
every module then builds against, so `./dev check`'s exact baselines must not move. The passthrough path to
change is `load_shader` and `create_compute_pipeline` in its `metal/device.rs`. The cooker is `thrum-cook`,
`CONTENT-LOOP`'s; the library cook is a new step beside it and must stay out of every shipping package, which
`scripts/g5_shipping_links.py` checks. The Metal toolchain is installed, but plain `xcrun metal` fails on this
Mac: `xcrun --toolchain Metal metal` runs it.

## M12 A warm launch compiles no pipeline: the local pipeline archive is proved by a strict fresh-process run

The pinned copy of wgpu-hal is M10's. Keep the archive patch small and apart from M10's, since the owner may
offer it upstream. Post nothing, and open no issue or pull request.

## U1 The Metal pipeline-cache change is offered upstream by the owner

Later, and not part of V1. After M12 is proved; the owner writes and approves every public post. [As drafted](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md#u1-upstream-metal-pipeline-cache-later-not-v1-gating-if-needed-pending).

## M11 Apple tile with MetalFX and EDR is compared with portable TAAU on the route, and G4 closes

`APPLE-TILE` M12's record holds the fixed-extent baseline and the eligibility rows this proof reruns with real
reconstruction readers and scale steps. The evidence page is `.orchestrate/evidence/render/G4-PAIRED-<date>.md`, and the
ledger is `docs/design/maturity.json`. Ask for the owner's look at the panel in the evidence page and in your
report, and never wait for it: G4 closes on the agent and device evidence, with the look recorded as outstanding.
