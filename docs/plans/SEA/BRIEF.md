# Sea module brief (E1.water)

10 November 2026. Brief for the `SEA` module in [the module list](../ARCHITECTURE.md). A `plan-module` run turns it
into this folder's `DESIGN.md`, `PLAN.md` and done-when file. It finishes the Broadsides Rust sea in Thrum under
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) and [the water decision](../../design/decisions/E1-GPU-WATER.md).
It does not replace [WATER.md](../../design/modules/WATER.md), which keeps the wider inland designs.

The owner's request, from the 10 November planning task: "wants water and clouds finished and consolidated ...
Decide, with reasons, what finished means for each, whether that finishing happens in Broadsides first or as Thrum
modules, and where in the build order the port's readiness conditions are met."

## Where it is finished

In Thrum. The reasons common to sea and clouds are in
[the module list](../ARCHITECTURE.md#where-sea-and-clouds-are-finished). For water in particular:

- The Broadsides sea pass measures about 21 ms of GPU time at 1440p on this Mac, and a three-cascade control
  measured 42 ms. Published FFT oceans run in 1–4 ms
  ([survey with the Triton author](https://www.gamedeveloper.com/programming/3d-water-effects-here-s-what-s-involved-)).
  The cost is in how the Broadsides passes are assembled, so tuning looks there would tune the wrong frame.
- Water milestone M10 in Broadsides has been open since 8 November. Three spikes on 10 November each ended
  without a pass. Their results are inputs here, listed below.
- The plan the Broadsides branch still has queued (inlet and coast, rain into a stream, a lake showcase, a fleet
  in weather) is the naval game's scope; the first of those is estimated there at 8–14 days. None of it is needed
  for a finished sea.

Broadsides is read at these commits and never changed:

| Reference | Commit | What it is |
|---|---|---|
| `codex/water-rust-system` | `ae8c1ebb7` | Design, plan and handoff under `notes/water/`; last verified runtime `095b46394` |
| Accepted sea baseline | `7a59cb01b` | The owner-accepted calm and trade-wind sea (W1) |
| `claude/fable-foam` | `2e5f47184` | One foam model for all water; stochastic pattern sampling; whitecap births |
| `claude/fable-hull-release` | `cea446ae5` | A ballistic release ribbon; costed, not at the bar |
| `claude/fable-near-waterline` | `e8b45e0f0` | Gas march and attached sheet at the waterline; not at the bar |

## What finished means

A sea a visitor can change and a game can float things on, in a Thrum example scene built on the creator path,
on `mac-metal` and `chrome-webgpu`, lit only by G3s. The bar is the sea and wakes of Sea of Thieves, in words,
at the display's full refresh.

| Cue | Range it must hold over |
|---|---|
| S1 Open sea from the waterline to the horizon: wind sea over swell, no repeating tile, crests that sharpen with wind | Glassy calm to gale presets; camera 1–40 m above the water and a 1–20 km vista; noon, low Sun and twilight |
| S2 Bodies float: a small float and one hull rise, pitch and roll with the waves they sit on | One craft and eight floats; rest to 15 m/s; the GPU field read through one batched asynchronous query, one to two ticks late, never synchronously |
| S3 A wake that is water: it spreads behind a moving hull, persists after the hull stops or turns, and lifts a float it crosses | Displacement speeds to planing; turn, coast and restart; wedge angle and crest travel compared with theory at two speeds |
| S4 Foam that is material: born where waves break and where the hull meets water, carried by the water, thinning and vanishing with age | No foam on a calm sea or at rest; whitecaps scaling from fresh breeze to gale; a trail that outlives its source; no stamped or tiled pattern |
| S5 Sea optics: the sky and Sun reflect, colour deepens with depth, the Sun glitters, cloud shade dims it once | G3s sky from dawn to night; screen-space reflection with the environment product as fallback; `CLOUDS` shade when present |
| S6 Stable in motion: no shimmer, ghosting or history smear under reconstruction; cuts and rebuilds reset cleanly | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract |
| S7 Controls a visitor can use: wind, swell, time of day and a field overlay, on Mac and in Chrome | The typed controls of `CREATOR` |

Finished also means: the CPU f64 long-wave evaluator is the test oracle and the GPU matches it within a declared
tolerance; persistent state advances from tick-derived time; capacity overflow is a counted refusal; every
cadence is a governor knob with a declared history invalidation; and the cost account holds at full refresh on
this Mac at the showcase extent, with paired records and tripwires rather than a budget gate.

**Not part of finished**, with the reason:
- Chase-distance spray release from a planing hull, the attached sheet at the hull, and a visibly rising gas
  cloud. Four earlier trials and the 10 November ribbon failed the first; the waterline spike failed the other
  two. They are parked spikes that re-enter only with a reviewed, costed mechanism.
- Inland water (puddles, streams, rivers, lakes), coasts, shoaling, surf and tides. They stay with
  [WATER.md](../../design/modules/WATER.md) and the Broadsides design as later profiles of the same contracts.
- Planing trim calibrated to tow-tank data, fleets, and ray-traced reflection.
- Multiplayer water. The water decision's seams (causes on the wire, no water state) are kept; nothing is built.

## What is lifted, rewritten and left

Following the port decision's policy: lift maths and reviewed algorithms through review, re-derive the golden
tests here, record source commit, path and licence for each lifted file, and rewrite all scheduling and glue.

| From Broadsides | Into Thrum | Action |
|---|---|---|
| `bw_water` ocean maths and the f64 references | The sea's CPU authority and oracle | Lift with tolerances and tests |
| FFT, LEAN, cascade and clipmap kernels | Sea dynamics passes on `EFFECT-GRAPH` | Lift kernels; rewrite allocation, scheduling and variant selection; re-cost |
| Wake solver and hull pressure sources | Sea dynamics | Reconcile first: the decision selects a two-level dispersive solver, and the branch built a split-band variant |
| Body query ring, receipts and support rules | The body-query interface | Keep the contract; rewrite the mailbox and packing |
| Hydrostatics, strip loads, hull proxy bake | The example game, until Thrum has a physics owner | Lift as game-side code and content |
| Foam and aeration transport, the four-stock model, stochastic sampling, whitecap births | Sea whitewater | Lift the model and the two changes the foam spike passed; leave the open looks as tuning |
| Water and light composition, scene-copy refraction | C08 medium segments and a recipe-specific refraction source | Rewrite |
| Broadsides sky, exposure, temporal AA, Bevy host, frame packets | Nothing | Left behind |
| Release ribbon, gas march, attached sheet | Nothing now | Kept as references for the parked spikes |

## Interfaces and needs

Uses: the tick page (C02); the capability registry and recipes (C04); effect passes and resources
(`EFFECT-GRAPH`); the temporal contribution (`APPLE-PRESENTATION` M2–M3); the environment identity, products and
Sun-path optical depth (`ATMOSPHERE:M5`); cooked products for the foam atlas (`CONTENT-LOOP`); the scene shell and
controls on Mac and web (`CREATOR:M1`); the governor (`APPLE-PRESENTATION:M7`). Provides: the body query and the
sea's ordered medium segments and reflection input.

Proposed shape for the planner, riskiest first:
1. **CPU authority and oracle.** Long-wave evaluator, spectra, presets and the conformance harness, with no GPU.
   Needs nothing.
2. **First sea frame at cost.** One preset in a Thrum scene under G3s at full refresh, written to the frame
   contract. This settles whether the re-costed cascade design holds; it is the riskiest step. Needs
   `APPLE-TILE:M3`, `APPLE-PRESENTATION:M3`, `ATMOSPHERE:M5`, `EFFECT-GRAPH:M3`.
3. **Bodies and the query**, then **wake**, then **foam**, then **optics and the range of presets**.
4. **Recipes, knobs, Chrome and close.** Needs `APPLE-PRESENTATION:M7` and `CREATOR:M1`.

## Open decisions for the module planner

- The cost account before any representation is chosen: cascade count and size, update cadence, clipmap
  resolution, pass count and bytes moved per tick and per frame, on Apple tile and on the portable recipe, whose
  limits (256-point f32 FFT, 16 KiB workgroup memory, 16 sampled textures) come from the registry.
- Which wake operator the sea carries, decided against the cues and cost, not by which code exists.
- Where rigid-body integration lives while Thrum has no physics module. The engine side owns only the query.
- The refraction source on each recipe, replacing the full scene copy.
- How much of the foam spike's unfinished look work (feathered caps, pale bubbles under foam, trails that widen)
  is inside S4, judged against the bar.

Research depth: full module research. Start from the Broadsides design and evidence as what was tried, then
shipped and published practice for FFT cascades, wake solvers and foam at this frame rate, before selecting.

## Owner decisions

- When builders start this module ([port timing](../ARCHITECTURE.md#owner-decisions)).
- A licence for lifting Broadsides code and content into this repository.
