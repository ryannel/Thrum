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
- Assumption: WP12 also pins wgpu-core 30.0.1, changed only so that a pipeline cache's header can be written and
  checked on Metal. Released wgpu-core does that for Vulkan alone, so the archive cannot be saved or loaded
  through wgpu without it. The lead took this route on 26 November; the owner can overturn it.
- Assumption: WP16 keeps the device safe for timestamp query sets created at the engine's one call site. Code
  that creates one with wgpu directly, in the same process and at the same moment as an engine request, is not
  protected, because that needs a change to wgpu's Metal backend. The owner can ask for that change.
- Assumption: a viewer cannot see a brightness difference under 1% of a frame's mean luminance, so D8.3 allows
  MetalFX that much. It is a little under one step of the 8-bit output at the brightness of D8.3's frames.
- Assumption: until WP17 shows 120 Hz held, a full-screen window asks for 60 Hz by itself and a smaller window
  keeps the display's full refresh. A launch setting forces either.
- Assumption: the owner's "120 upscale for full screen" covers an output scale down to 67% of the panel's size
  on each axis, since the measurement put to them pointed to about 75%. If no step from 67% up holds 120 Hz,
  WP17's builder asks before promoting 50%. The owner can allow 50% or name another floor.
- Assumption: the output scale is promoted only if it holds 120 Hz on the populated exit flythrough as well as
  on the bare valley route, so the showcase's own record does not find the value too large. The other choice
  measures the bare valley alone and leaves the populated scene to `SHOWCASE` WP8.
- Assumption: both of the Mac's recipes use the one full-screen output scale, measured on the Apple recipe, so
  the showcase's rows compare at one extent. Whether the portable recipe holds 120 Hz there is recorded and
  not claimed.
- Assumption: WP17 comes after WP11, so `SHOWCASE` WP7 does not wait for it, and the V1 exit rests on it.
  `SHOWCASE` WP8 depends on WP17 and its WP8.12 lists it.

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

## WP15 Chrome's SDR valley matches the retained frames again: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP15-review.md)

## WP6 Highlights go brighter than SDR white on the built-in XDR display: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP6-review.md)

## WP16 The long tests pass again, and running out of GPU timers is a named error, not a lost device: done

## WP7 The governor holds the display rate under forced load, then recovers step by step: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7-review.md)

## WP8 MetalFX reconstructs the valley on Apple tile, with TAAU as its prepared fallback: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-review.md)

## WP9 Surface, device and display changes create nothing during play, and launch-to-ready time is on record: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP9-review.md)

## WP10 Shaders compiled ahead of time load at launch with no WGSL translation, and pipelines are prepared in parallel: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP10-review.md)

## WP12 A warm launch compiles no pipeline: the local pipeline archive is proved by a strict fresh-process run: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-review.md)

## WP11 Apple tile with MetalFX and EDR is compared with portable TAAU on the route, and each clause of G4 is proved: done
[review](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11-review.md)

## WP18 A launch setting draws the valley below the window's pixel size and the display scales it to fill, with no hitch at the change

Read [the output scale](DESIGN.md#the-output-scale-wp18) and
[the surface across a transition](DESIGN.md#the-surface-across-a-transition-wp4) first. The registry's
full-screen value stays 100% here and the 60 Hz rule for full screen stays: WP17 changes both, after its
sitting. `ATMOSPHERE` WP12 is built beside this work package and changes the knob registry in the governor's
files, so merge one after the other. D18.5's 75% comes through the game's proof options, never the launch
setting.

## WP17 The valley holds the display's full 120 Hz at full screen on the built-in display

Read [reaching the full refresh at full screen](DESIGN.md#reaching-the-full-refresh-at-full-screen-wp17) first.
The order is part of this work package. Remove the 60 Hz rule and build D17.1, then try steps under whatever
load the host has: those takes are witnesses. Keep one on each route at every step from 67% to 100%, because
D17.5's record is the pair at the step promoted. Then end `BLOCKED` asking the lead for the full-refresh
sitting with the other builders paused. In it, try the steps from 100% down, set the registry's value, make
the four closing takes on that build, and capture D17.3's cue windows. The ledger and the evidence page come
last.

[The planner's record](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md) has four takes at
100% and the scripts that made them, which are a place to start and no part of the proof. Its exit take
stopped after 300 ticks, because the live route replay ends at 560 frames; D17.2 needs the whole flythrough.
The evidence page is `.orchestrate/evidence/render/G4-PAIRED-<date>.md`, which WP11 wrote, and the ledger is
`docs/design/maturity.json`. Ask for the owner's look at the panel there and in your report, and never wait
for it.

## U1 The owner can read, in one reviewed document, what Thrum's Metal pipeline cache would offer wgpu and what to settle there first: done
[review record](RECORD.json)

## U2 The two local wgpu cache patches pass independent code review and cache tests on Metal

Review the HAL archive and core header against the release Thrum builds. The [refreshed U1 briefing](../../../.orchestrate/evidence/APPLE-PRESENTATION/U2/upstream-notes.md)
maps the diffs. Use the independent work-package review for the code judgement.
[Local patch design](DESIGN.md#local-metal-pipeline-cache-patches-u2-and-u3)

## U3 Thrum isolates the local cache patches and flags the wgpu release that can replace them

Start at thrum-render's archive preparation and its public round-trip test. Put the removal steps in
[the pin provenance](../../provenance/APPLE-PRESENTATION.md). Exercise the release notice with a
known-positive case while the live release is still outside Thrum's control.
