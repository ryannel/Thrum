# APPLE-PRESENTATION plan

Reconstruction, display and the quality governor (gate G4), governed by
[the G3t boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07 and C12.
[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list and seam rulings](../ARCHITECTURE.md#interfaces) · paired plan [APPLE-TILE](../APPLE-TILE/PLAN.md)

Read the [design](DESIGN.md#wp4-to-wp12-settled-14-november-2026) and seams 5–9 before starting.
Build in the order below.

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the governor drops scale after three frames over budget, recovers at most one step per second,
  and makes at most two scale changes on an unloaded route.
- Assumption: at full window size the frame rate may dip until the governor lands in WP7. Record times for
  that interval without applying a frame-time gate.
- Assumption: WP10 pins wgpu-hal 30.0.1 for the Metal library loader. WP12 adds the archive to that copy.
- Assumption: WP16 keeps the device safe for timestamp query sets created at the engine's one call site. Code
  that creates one with wgpu directly, in the same process and at the same moment as an engine request, is not
  protected, because that needs a change to wgpu's Metal backend. The owner can ask for that change.
- Assumption: a viewer cannot see a brightness difference under 1% of a frame's mean luminance, so D8.3 allows
  MetalFX that much. It is a little under one step of the 8-bit output at the brightness of D8.3's frames.

## WP1 Every pixel of the valley carries correct motion under jitter, including the sky and stars: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP1-review.md)

## WP2 Depth, motion, reactive and exposure are stored for reconstruction, and history advances only with submitted frames: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP2-review.md)

## WP3 TAAU reconstructs the valley at native scale in the game, on the Mac and in Chrome: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP3-review.md)

## WP4 The valley fills the window at any size, and keeps presenting while the window is resized: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP4-review.md)

## WP14 The retained native valley images reflect the corrected material mip bias: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP14-review.md)

## WP13 A small exposure check catches a twilight brightness rebound: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP13-review.md)

## WP5 Render scale steps during play with no hitch and no history reset: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP5-review.md)

## WP15 Chrome's SDR valley matches the retained frames again

Restore the [SDR shoulder arithmetic](DESIGN.md#the-edr-transform-and-its-h--1-identity-wp6) before continuing WP6.
Match the route, cooked root and record identities in the before-and-after Chrome fixed day captures.
Keep both receipts and rerun D6.2 and D6.3 on the corrected shader. The shared `exact-images` run proves
the full matrix on the landing against the [registered APPLE-TILE baselines](../ARCHITECTURE.md#exact-baselines).

## WP6 Highlights go brighter than SDR white on the built-in XDR display

Keep the real display record alongside the forced record in the proofs so they can run with the display asleep.

## WP16 The long tests pass again, and running out of GPU timers is a named error, not a lost device

The main-branch check `long-tests` has failed since a25e1d13d, and this is its fix:
[the design](DESIGN.md#timestamp-query-sets-and-metals-limit-wp16) gives the cause, the two mechanisms and what a
refusal means for a frame host and for the ray build probe. The
fault needs tests that share a process. `./dev check` runs each test in a process of its own and passes with the
fault in. `cargo test -p v1-game --features draw-probe,thrum-render/creation-counters --test ray_tracing_gpu`
fails from four test threads up, and passes with three.

`./dev long-tests` runs every test of `timestamp_budget` in one process, side by side, and WP16.4 does the same.
So the two tests that fill the process to 32 must not leave the binary's other tests refused, or each other.
The guarantee holds [while every creation during a request comes through the call
site](DESIGN.md#timestamp-query-sets-and-metals-limit-wp16), so the tests hold their query sets through it and
`thrum-render` offers it outside the crate. WP16.6 needs a test to hold a request between its Metal trial and its
wgpu creation: keep that hold out of builds without `creation-counters`. `SEA`'s harness in
`crates/thrum-sea/tests/support.rs` is not yours to change.
WP7's items do not change and its tests keep passing: the governed frame still frees its slot while its
timestamps are read back. In `RAY-TRACING`'s `build_probe.rs` change only how `encode` asks for its query set
and what it records when refused, under [the ruling](../ARCHITECTURE.md#interfaces). The test-only cap in
`timestamp_permit.rs` stays unless WP16.4 passes without it. `mesh_stage_gpu`'s not-run line in
`.config/long-tests.txt` is `MESH-SHADING`'s to lift: record in the log whether it passes whole after this
work package, and leave the line.

## WP7 The governor holds the display rate under forced load, then recovers step by step

The aerial lookup's cadence knob is built in `ATMOSPHERE`'s files under the ruling for seam 7 if that module has
not declared one; its items do not change. `WATERS` and `CLOUDS` register their own knobs later through the same
interface, so keep it free of any module's name. Frame samples and pass timing are the frame runtime's feed
(`timing.rs`), which `CREATOR`'s profiler also reads: leave its timing-demand switch working. The generation
build's lifecycle in the frame host is WP4's; marking held frames on it is this work package's, and D4.5's test
already forces the failed build D7.9 needs.

## WP8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback

`APPLE-TILE` WP13 leaves the native segment and its provider interface, `NativeProvider`, in
`crates/thrum-render/src/apple_tile`, as [its design](../APPLE-TILE/DESIGN.md#the-native-segment-and-its-providers-wp13)
describes; the scaler is a provider there and TAAU is its twin. WP10 and WP11 had built that lifecycle only
inside the copy/select witness, which is why this work package stopped on 25 November. The segment keeps the
candidate, the reset decision, the encoder split, completion and the counters, so D8.5 and D8.6 drive it and
add none of their own. Where MetalFX needs something the interface lacks, change it in `APPLE-TILE`'s files
under [the ruling for seam 8](../ARCHITECTURE.md#interfaces) and keep that module's tests passing.

Carry on from `build/apple-presentation` at `203ba8269`, which holds the provider, the TAAU twin, the pre-pass
and the builder's passes for every item but D8.3 and D8.8; the
[handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-handoff.md) says what was tried. D8.3 changed on
25 November and no other item did. It used to hold MetalFX to TAAU's distance from the un-reconstructed frame
with no tolerance, which failed on differences between the two reconstructors of 0.0003 in mean luminance that
were the same with the storage scale held. It now asks only for what a viewer could see: the forced step is
judged against a matched run with the scale held, the scrub against the un-reconstructed frame, both within 1%.
Two identical runs differed by exactly zero on this Mac, so the bound carries no noise term. TAAU is not judged
by D8.3.

The old bound is what switched the pre-pass on, so judge the exposure texture alone against the new one first.
Its first run sat within 0.51% of the un-reconstructed frame through the scrub and 0.13% through the step, so
expect it to pass. If it does, it is the path: take the pre-pass out of the product with its counters and
tests, because nothing would select it, and take again the evidence of the other items that the pre-pass was on
for, D8.8's recordings among them. Keep the pre-pass only if the exposure texture alone fails D8.3.

## WP9 Surface, device and display changes create nothing during play, and launch-to-ready time is on record

The creation counters are `runtime/creation.rs`, behind the `creation-counters` feature. D9.1's split is the
baseline WP10 and WP12 are judged against, so record it before changing how anything is prepared.

## WP10 Shaders compiled ahead of time load at launch with no WGSL translation, and pipelines are prepared in parallel

Use `xcrun --toolchain Metal metal` on the Mac. The shared `exact-images` run checks the pinned backend
against the retained images.

## WP12 A warm launch compiles no pipeline: the local pipeline archive is proved by a strict fresh-process run

The pinned copy of wgpu-hal is WP10's. Keep the archive patch small and apart from WP10's, since the owner may
offer it upstream. Post nothing, and open no issue or pull request.

## U1 The Metal pipeline-cache change is offered upstream by the owner

Later, and not part of V1. After WP12 is proved; the owner writes and approves every public post. [As drafted](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP4-M11-milestone-drafts.md#u1-upstream-metal-pipeline-cache-later-not-v1-gating-if-needed-pending).

## WP11 Apple tile with MetalFX and EDR is compared with portable TAAU on the route, and G4 closes

`APPLE-TILE` WP12's record holds the fixed-extent baseline and the eligibility rows this proof reruns with real
reconstruction readers and scale steps. The evidence page is `.orchestrate/evidence/render/G4-PAIRED-<date>.md`, and the
ledger is `docs/design/maturity.json`. Ask for the owner's look at the panel in the evidence page and in your
report, and never wait for it: G4 closes on the agent and device evidence, with the look recorded as outstanding.
