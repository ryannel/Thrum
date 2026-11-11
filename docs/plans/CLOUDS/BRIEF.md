# Clouds module brief (E1.clouds)

10 November 2026. Brief for the `CLOUDS` module in [the module list](../ARCHITECTURE.md). A `plan-module` run turns
it into this folder's `DESIGN.md`, which rewrites [CLOUDS.md](../../design/modules/CLOUDS.md), with a `PLAN.md` and a done-when file. It consolidates the two Broadsides cloud
paths into one Thrum provider under [the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md). The optical rules and
Python references already in [CLOUDS.md](../../design/modules/CLOUDS.md) stay in force.

The owner's request, from the 10 November planning task: "Two cloud paths, a baked library and a procedural one,
were built by separate Codex agents in Broadsides ... The owner has now asked for both to be finished and
consolidated. Decide, with reasons, what finished means for each, whether that finishing happens in Broadsides
first or as Thrum modules, and where in the build order the port's readiness conditions are met."

## Where it is finished

In Thrum, as one module. The reasons common to sea and clouds are in
[the module list](../ARCHITECTURE.md#where-sea-and-clouds-are-finished). For clouds in particular:

- **Cost.** The procedural path's own 6 November matrix records a median paired cost of 289 ms a frame for its
  fair preset and 88 ms for overcast at 1440p. Guerrilla's Nubis clouds cost about 2 ms on a PlayStation 4
  ([Nubis Evolved, SIGGRAPH 2022](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-NubisEvolved-NoVideos.pdf)).
  A march that heavy is a look reference. Its mechanism has to be re-costed before it is lifted, and a look
  approved at that cost would not survive the change.
- **Light.** Cloud appearance is mostly lighting. Broadsides lights clouds from its own sky capture and Hillaire
  implementation, and the port decision ports neither. Every sunlit side, shadowed base and sunset colour has to
  be judged again under G3s.
- **The baked path's open work is its reader.** It is paused at its milestone M8 on a combined eye-depth
  mismatch in its atlas march, with a shared atlas Broadsides main lacks. That reader is rewritten on Thrum's
  cooked products and residency whatever happens.
- **The content is already accepted.** The owner accepted twelve original cloud volumes (six fair, three
  mediocris, three congestus), and a staged congestus growth track passed independent review in motion. Content
  moves between renderers unchanged.

Broadsides is read at these commits and never changed:

| Reference | Commit | What it is |
|---|---|---|
| `codex/clouds-procedural-publish-20261104` | `b4d8250e6` | Procedural path: `notes/clouds/RUNTIME-DESIGN.md`, its plan, `THRUM-PORT.md`, `ASSET-CONTRACT.md`; M1 and M2 closed, paused before M3 |
| `codex/cloud-route-wip-handoff` | `79637f789` | Baked path: `factory/clouds/PIPELINE-DESIGN.md`, its plan and `heaped-fair/`; M1–M3 done, paused in M8 |
| `codex/cloud-bakeoff-pause-main` | `4c0f16d74` | The baked pause merged with Broadsides main |

## What finished means

One provider that makes the sky over the V1 valley and the sea scene, on `mac-metal` and `chrome-webgpu`, lit
only by G3s. The bar is the skies of Red Dead Redemption 2 and Horizon Forbidden West, in words, at the display's
full refresh.

| Cue | Range it must hold over |
|---|---|
| K1 A weather-driven sky: scattered fair cumulus with shared bases and varied sizes, a broken deck, and overcast, each recognisable and continuous to the horizon | Clear, fair, broken and overcast presets and the transitions between them; ground and deck cameras 2–40 m up; the full look arc |
| K2 Light that reads as cloud: bright sunlit sides grading into shadowed bases, a silver lining toward the Sun, and sunset and twilight colour taken from the sky | Noon, low Sun, backlit, sunset through twilight to night, on the one G3s Sun and sky |
| K3 Shade that belongs to the clouds: patches of sunlight and shade cross terrain, objects and water in step with the clouds above | One optical depth applied once per receiver; also with visible clouds off (shadow-only) |
| K4 Clouds that move and change: they drift with the wind in world space and form, grow and thin over time | 0–40 m/s wind; real time and a time-lapse control; no popping, sliding texture or periodic slots |
| K5 Placed volumes: clouds from the baked library stand in the same sky under the same light, and one grows through its staged track | The twelve accepted volumes and the congestus growth track, cooked as content products; placement by the weather field |
| K6 Stable in motion: no shimmer, crawl or ghosting under reconstruction as the camera turns and time runs | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract |
| K7 Controls a visitor can use: coverage, cloud type, wind, time of day and an internal-field overlay, on Mac and in Chrome | The typed controls of `CREATOR` |

Finished also means: the weather field is a deterministic function of seed, preset, integrated wind travel and
simulation time, with a CPU evaluator that answers point queries headless and a test that the GPU map matches it;
all consumers share extinction in m⁻¹; aerial perspective is applied once; cloud and fog segments are ordered
and disjoint; visible clouds off releases presentation resources and keeps shade; every cadence is a governor
knob with a declared history invalidation; and the cost account holds at full refresh on this Mac at the
showcase extent, with texture memory recorded, by paired records and tripwires rather than a budget gate.

**Not part of finished**, with the reason:
- The ten cloud genera, cumulonimbus with anvil and rain shafts, the warm-front sequence and high ice layers.
  They are the Broadsides game's scope ([ENVIRONMENT](../../design/modules/ENVIRONMENT.md) says so) and the procedural path has three
  genera built after its whole run.
- The baked path's two-layer maritime decks from large-eddy simulation, their rectangular layer contract and
  fine-page-miss continuity. They are open research there, and decks in finished clouds come from the weather
  layers.
- Rain, lightning, moonlit cloud and flying through cloud.
- A "physically simulated cloud formation" teaching demo. It is a later demo built on this provider.

## Consolidation

The port decision already selects the shape: "Cloud presentation combines procedural weather layers with placed
voxel volumes under the same E1 provider and light-transmittance contract." So:
- the **weather field** is the single authority: coverage, type, top height, age and column optical depth;
- **weather layers** draw the far field, decks and the general fair sky from that field;
- **placed volumes** from the baked library are hero clouds the field positions and ages;
- one march, one light evaluation and one optical-depth product serve both, with coefficients added before
  attenuation so nothing is darkened twice.

## What is lifted, rewritten and left

| From Broadsides | Into Thrum | Action |
|---|---|---|
| Weather and noise maths with the CPU point query | The weather authority | Lift with tests; rewrite the tick-page adapter |
| Density, phase, intersection and lighting shader operators | The cloud shading library | Lift after re-costing the march; replace every environment binding with G3s products |
| The packed Beer shadow and its lookup | The Sun-path optical-depth product | Lift the construction; conform it to the interface `ATMOSPHERE:M5` defines |
| The authored cloud-shape contract and the twelve volumes with their growth track | Cooked density products in `CONTENT-LOOP` and placement data in the game | Lift content and meaning; rewrite IO, cooking and residency |
| Atlas, pager, layer reader, march scheduling, caches, sky capture | The Thrum executor and `EFFECT-GRAPH`; G3s | Rewrite or drop |
| The offline optical references | Already in `scripts/reference/cloud/` | Keep as oracles |
| The four-band broad-base look, high-layer design, layer contract proposal | Nothing | Left behind |

## Interfaces and needs

Uses: the tick page (C02); the registry and recipes (C04); effect passes and resources (`EFFECT-GRAPH`); the
temporal contribution (`APPLE-PRESENTATION` M2–M3); the environment identity and products (`ATMOSPHERE:M5`);
density-volume products (`CONTENT-LOOP`); controls and the scene shell (`CREATOR`); the governor
(`APPLE-PRESENTATION:M7`). Provides: the Sun-path optical depth, replacing the diagnostic stand-in from
`ATMOSPHERE:M5`, and ordered cloud segments for V3.

Proposed shape for the planner, riskiest first:
1. **Weather authority on the CPU**, with its evaluator and tests. Needs nothing.
2. **First cloud frame at cost.** One fair preset over the valley under G3s at full refresh, written to the frame
   contract. The riskiest step: it settles whether the re-costed march reaches the look. Needs `APPLE-TILE:M3`,
   `APPLE-PRESENTATION:M3`, `ATMOSPHERE:M5`, `EFFECT-GRAPH:M1`.
3. **Shade and shadow-only**, then **light through the day**, then **placed volumes**, then **the weather range
   and motion**.
4. **Recipes, knobs, Chrome and close.** Needs `APPLE-PRESENTATION:M7`, `CREATOR:M1` and the `CONTENT-LOOP`
   density product.

## Open decisions for the module planner

- The cost account before the march is chosen: sample counts, the fraction of pixels refreshed per frame,
  shadow and weather cadence, noise and volume memory, on Apple tile and on the portable recipe. The port
  decision allows time-slicing as a governor knob and forbids a private upscaler.
- How placed volumes and weather layers share one march without a second density over the same air.
- Sun direction: G3s must give the continuous true Sun apart from the apparent surface light, which the
  Broadsides repair found necessary. `ATMOSPHERE:M5` carries it.
- Which of the procedural path's opt-in looks become defaults, judged again under G3s.

Research depth: full module research. Start from both Broadsides designs as what was tried, then current
practice for weather-map and voxel clouds at this frame rate, before selecting.

## Owner decisions

- When builders start this module ([port timing](../ARCHITECTURE.md#owner-decisions)).
- A licence for lifting Broadsides code and the cloud library into this repository. The library's licence file
  reserves it to the owner and grants no rights to third parties.
