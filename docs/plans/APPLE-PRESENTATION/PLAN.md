# APPLE-PRESENTATION plan

Reconstruction, display and the quality governor (gate G4), governed by
[the G3t boundary decision](../../design/decisions/G3T-METAL-BOUNDARY.md) and C04, C06, C07 and C12.
[Design](DESIGN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list and seam rulings](../ARCHITECTURE.md#interfaces) · paired plan [APPLE-TILE](../APPLE-TILE/PLAN.md)

Read the [design](DESIGN.md#m4-to-m12-settled-14-november-2026) and seams 5–9 before starting.
Build in the order below.

## Open for the owner

- Assumption: aim for about 2 s per fast test when run alone on a quiet Mac. The owner can revise this target.
- Assumption: the governor drops scale after three frames over budget, recovers at most one step per second,
  and makes at most two scale changes on an unloaded route.
- Assumption: at full window size the frame rate may dip until the governor lands in WP7. Record times for
  that interval without applying a frame-time gate.
- Assumption: WP10 pins wgpu-hal 30.0.1 for the Metal library loader. WP12 adds the archive to that copy.

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

Restore the [SDR shoulder arithmetic](DESIGN.md#the-edr-transform-and-its-h--1-identity-m6) before continuing WP6.
Match the route, cooked root and record identities in the before-and-after Chrome fixed day captures.
Keep both receipts and rerun D6.2 and D6.3 on the corrected shader. The shared `exact-images` run proves
the full matrix on the landing against the [registered APPLE-TILE baselines](../ARCHITECTURE.md#exact-baselines).

## WP6 Highlights go brighter than SDR white on the built-in XDR display

Keep the real display record alongside the forced record in the proofs so they can run with the display asleep.

## WP7 The governor holds the display rate under forced load, then recovers step by step

The aerial lookup's cadence knob is built in `ATMOSPHERE`'s files under the ruling for seam 7 if that module has
not declared one; its items do not change. `WATERS` and `CLOUDS` register their own knobs later through the same
interface, so keep it free of any module's name. Frame samples and pass timing are the frame runtime's feed
(`timing.rs`), which `CREATOR`'s profiler also reads: leave its timing-demand switch working. The generation
build's lifecycle in the frame host is WP4's; marking held frames on it is this work package's, and D4.5's test
already forces the failed build D7.9 needs.

## WP8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback

`APPLE-TILE` WP10 and WP11 left the native segment, its provider interface and the replacement rules in
`crates/thrum-render/src/apple_tile`; the scaler is a provider there. Take D8.3's noise floor first, on two
identical runs, since the exposure judgement is made against it. Build the normalising pre-pass only if the
exposure texture alone fails D8.3.

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
