# Waters: one water system for every body of water

14 November 2026, revised the same day for surf, mirrored scenery and the view from under water, again for
light under water, after the review of those revisions for
[a lake's own waves](#surf-the-shore-transform-and-the-breaker), and after the review of that for
[the highest surface under a crest](#bodies-and-the-query), which is now a cut of the sheet. Revised 17 November
2026 for the owner's bar: [the look and the speed decide](#proof-and-builder-handoff), and references are
diagnostics. Revised 25 November 2026 after the design review of that day: the cut is made
[coarse to fine](#bodies-and-the-query), so that a game gets the surface in the breakers in time to use it.
Revised again that day after WP19's builder stopped: the query is
[admitted by the tick](#bodies-and-the-query), and no longer waits for a view. Revised after that
revision's review: the carry bound covers an ordinary post, and a post is answered only from
[its own world's state](#bodies-and-the-query). Revised after the review of
[WP21](#a-surf-view-a-running-game-can-afford-wp21): delivery decides it, the distant front is held by
eye, and a held answer ends with its device.
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the wave field, `SEA`](../SEA/DESIGN.md). C02, C04, C06, C07, C08, C09 and C12 in
[CONTRACTS](../../design/CONTRACTS.md) and [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)
govern. [The water decision](../../design/decisions/E1-GPU-WATER.md), [the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md)
and the older water designs under [docs/design/modules](../../design/modules/WATER.md) are evidence this design
judged, and it replaces them as the authority for how Thrum's water works.

## The request

The owner, 14 November 2026, about the `SEA` module:

> Please make sure the sea work is actually thinking about water as a whole. So puddles, rivers, water falls,
> streams, lakes, and more. We want something that's built cohesively.

Earlier the same day, of clouds, and read by the lead as governing water too:

> The agent should look at that work from first principles and figure out how to design the best possible
> atmospheric system for the thrum game engine.

> We can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
> design.

And on 10 November, from [the module list](../ARCHITECTURE.md#the-owners-request): "wants water and clouds finished
and consolidated".

Later the same day, overturning three assumptions of this design's first version:

> Scenery reflections we must have, that's a main stay in high quality modern games. That's the standard we're
> after. Camera being able to go under water we need to support as well. Breaking waves is something we'd like
> to have as well.

So water reflects scenery the camera cannot see, the camera goes below the surface, and waves shoal and break at
a shore. The three mechanisms are [surf](#surf-the-shore-transform-and-the-breaker),
[mirror views](#mirror-views-scenery-the-camera-cannot-see) and [the view from under water](#under-water);
[judged from first principles](#judged-from-first-principles) says what each replaces.

The first version of that revision drew the view from under water without caustics, without shafts of light
and with an exposure metered from the sky. The lead's task of the same day, on the bar the owner's words set:
"The WATERS re-plan of that day left out caustics, underwater light shafts and exposure that adapts under
water, which are standard in high-end underwater views; bring the underwater view up to that bar". All three
are [light under water](#light-under-water-caustics-shafts-and-the-meter), and they are one mechanism.

Until 14 November `SEA` planned an open sea and nothing else, and inland water lived in designs no module was planned
from. This design starts again from what a game needs of water, settles one system that every body of water is
data for, and says which module builds each part. It keeps what the sea design of 13 and 14 November settled
wherever that holds for all water, and says where it does not. [Judged from first principles](#judged-from-first-principles)
gives each verdict.

## Purpose and scope

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a sunset over the sea, a lake under hills, a stream into a pond, rings from a dropped stone | Water that looks right, at the display's refresh, with only the parts the demo shows | Mechanisms with an oracle; every part beyond still open water optional, at no cost when left out |
| A game | Things that float, drift with a current and feel a wake, on any water, with the same forces whatever the camera or the device does | One physical profile on every recipe; forces from the water's own state; one query for all water |
| `CLOUDS` and `ATMOSPHERE` | A surface lit by the one sky and shaded once | No private sky, exposure or reconstruction |
| The planners of `RIVERS` and `PUDDLES` | A place for running water and rain water that needs no second surface, optics, foam or query | Every body is data for one map; the seams are proved here on stand-ins |

**Every body of water a game needs, and what carries it.** Water is four things, described under
[mechanisms](#one-water-in-four-parts): a map that says where it lies and how it moves, waves that ride on it,
material it carries, and one optics. A kind of water is a row of data for those four.

| Body | Where it lies (the map) | Waves | Whitewater | Built by |
|---|---|---|---|---|
| Open sea | A level; no bed in reach | The whole wave field | Breaking crests, hulls | `SEA` (the wave field), this module |
| Coast and beach | The sea's level meeting a rising bed, and the sand its waves wash | The field, its long waves turned, shortened and raised by the bed until they break, then held by the depth, with a remainder that runs the waterline up and back | Breaking crests and the bores they become, the shallows and the swash | This module |
| Lake, pond, pool, canal, flooded hollow | A level filling the hollow around a seed | The field at the body's fetch | Hulls; the lee shore | This module |
| River and stream | A chart: a sloping level and a current | Ripples carried by the current; roughness where it runs fast | Where the flow loses height; at rocks | The seam and a stand-in channel here; `RIVERS` |
| Waterfall | A fall joining two bodies | None: the sheet has its own streaks | The sheet as it breaks up; the plunge | `RIVERS` |
| Puddle and standing rain water | A level filling a small hollow, rising with rain | Rings from drops and feet | None | A level that rises and falls here; `PUDDLES` |
| Wet ground | No body: a wetness the ground's material reads | | | `PUDDLES`. The sheen and foam on sand the waves wash are this module's [film](#the-surface-and-the-frame-contract) |

**Commissioned in this module:** the shared system, and still open water on it: the sea, the shore, and lakes,
ponds and pools. Cues S1 to S7 of the sea brief and W1 to W12 below, on `mac-metal` and `chrome-webgpu`, lit only by
`ATMOSPHERE`'s products. The wave field is `SEA`'s, built by its WP1 and WP2 and unchanged by this design.

**Planned as later modules**, from [their briefs](../ARCHITECTURE.md#rivers-and-puddles): `RIVERS` (rivers,
streams and waterfalls) and `PUDDLES` (rain, puddles and wet ground).

**Not built by any planned module**, with the reason:

- Water that leaves the surface: spray thrown from a breaking crest, the splash where its lip lands, and a
  barrel a camera can ride inside. The breaker here is one sheet that pitches forward and may overhang. Also
  the currents surf drives: set-up of the mean level, the longshore drift and rips.
- Bulk water that moves while the game runs: a flood, a dam break, a level change that travels downstream. Flow is
  prepared once. A live solver would write the same map, so it can come later without changing a reader.
- Drops on a lens that has surfaced, and particles and bubbles drifting in the water: the engine has no
  particle owner. Light the surface throws up onto a hull's side or the underside of a bridge. Water above
  water in one column (a pool on a bridge over a river), and water in a vessel.
- The three sea cues the 10 November spikes left open (spray leaving a hull at chase distance, the attached sheet
  at the hull, a rising gas cloud), planing trim from tow-tank data, fleets, ray-traced reflection (a later
  provider for [the reflection slot](#optics) on devices that have it), multiplayer
  water, Kelvin packets for far bodies, a second set of interaction windows, and saving water state.
- Ice and snow, which are not this system's.

The seams the water decision asks for are kept: causes are plain records with stable IDs, forces never depend on
the camera, and no water state would cross a network.

**What exists.** `thrum-sea-model`, the CPU wave model, is built by `SEA` WP1. Thrum has no water on the GPU.
Broadsides has a sea whose surface pass takes 21 ms at 1440p, and lake and coast code that was never accepted:
its kernels are a resource, and its frame is not.

## Cues

S1 to S7 are the sea brief's, and each is something a user above needs. W1 to W6 are what water as a whole adds,
and W7 to W9 are the owner's of 14 November. W10 to W12 bring the view from under water to the bar the owner
set that day. The bar for the sea is the sea and wakes of Sea of Thieves; for
inland water and for mirrored scenery it is the lakes, shores and streams of Red Dead Redemption 2; for surf it
is the beaches of Horizon Forbidden West; for the view from under water it is Sea of Thieves again, and for
the light in it the shallows of Subnautica. All in
words, at the display's full refresh.

The last column says what tells each cue from its substitute. For light, wakes and foam that is something seen
in the running scene, under [the bar](#proof-and-builder-handoff); where theory or the model is named there as
a diagnostic, it finds the cause of a look that is wrong and decides no item. What a game reads, the frame
contract and what one pass hands another are still tested.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **S1** Open sea from the waterline to the horizon: wind sea over swell, no repeating tile, crests that sharpen with wind | Glassy calm to gale presets; camera 1–40 m above the water and a 1–20 km vista; noon, low Sun and twilight | One tiled FFT patch. Rejected by a 40 m view of a kilometre of sea, and by the long waves matching the CPU model at points a patch length apart |
| **S2** Bodies float: a small float and one hull rise, pitch and roll with the waves they sit on | One craft and eight floats; rest to 15 m/s; the GPU field read through one batched asynchronous query, one to two ticks late, never synchronously | Scheduled bobbing, or a CPU copy of the waves. Rejected by a float riding a wake crest that only the GPU holds, and by a count of blocking reads that stays zero |
| **S3** A wake that is water: it spreads behind a moving hull, persists after the hull stops or turns, and lifts a float it crosses | Displacement speeds to planing; turn, coast and restart; 3 m/s and 12 m/s, with theory's wedge angle and crest travel as a diagnostic | A wake decal, or a constant-speed wave equation. Rejected by arms that narrow from 3 m/s to 12 m/s with transverse crests between them, by a crest that keeps travelling after the source is removed, and by a query's wake height, which must be the wake's own state |
| **S4** Foam that is material: born where waves break and where the hull meets water, carried by the water, thinning and vanishing with age | No foam on a calm sea or at rest; whitecaps scaling from fresh breeze to gale; a trail that outlives its source; no stamped or tiled pattern | Foam drawn from the present wave shape. Rejected by a trail that stays where the water carried it after the hull has gone, and by two views a tile length apart |
| **S5** Sea optics: the sky and Sun reflect, colour deepens with depth, the Sun glitters, cloud shade dims it once | G3s sky from dawn to night; screen-space reflection with the environment product as fallback; `CLOUDS` shade when present | A private sky or exposure, or a frame that only looks right at one exposure. Rejected by the waterline seen against terrain-lit ground from dawn to night, and by a reflected panel and a bed under water whose colour over the exposure scale holds at two exposures and carries the camera's air once. The optics equation is a diagnostic |
| **S6** Stable in motion: no shimmer, ghosting or history smear under reconstruction; cuts and rebuilds reset cleanly | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract | Zero motion on a moving surface. Rejected by motion read from submitted frames against the surface's known displacement |
| **S7** Controls a visitor can use: wind, swell, time of day and a field overlay, on Mac and in Chrome | The typed controls of `CREATOR` | A control that restarts the sea. Rejected by a wind change in motion with no pop |
| **W1** Water lies where the land holds it: a level surface that ends in a clean line where the ground rises | A sea, a lake above it and a pond in one view; levels raised and lowered while the scene runs; from the shore and from 40 m up | A water plane drawn through the terrain, or a sheet joining two levels. Rejected by a hollow beyond a ridge that must stay dry, and by a top-down view classified from the CPU model |
| **W2** Shelter and shallows: a lake is calm while the sea runs, waves fade as water shallows, and on a beach the waterline runs up and back with them | Glassy calm to gale; fetches of 200 m to open sea; depths from 10 m to the waterline; a beach of 1 in 10 at a fixed stage | One wave height for all water, or a waterline pinned to the still level. Rejected by the lake's measured height against its fetch in a gale, by a wave that reaches the beach at full height, and by the waterline's travel under one regular wave |
| **W3** The ground shows through the shallows and the shore reflects in still water | Depths from the waterline to where the water type hides the bed; calm to breeze; the far shore, hills and a tree on the bank on screen | An opaque sheet with a painted edge, or a sky-only mirror. Rejected by a bed that shows through the shallows and fades with depth into the water's own colour, with the optics equation as a diagnostic, and by a reflection that must join its source at the waterline |
| **W4** Rings on any water: a dropped stone or a hull raises waves that travel at the speed the depth gives and die at the shore | Depths of 0.25 m to deep; rings and wakes, short waves and long, in one window that holds shallows and deep water; a shore and a bar of dry ground | A deep-water solver everywhere, or one transform over land and water alike. Rejected by rings that slow and bunch toward the shallows, with the depth's wave speed as a diagnostic, and by the water beyond a bar of ground, which must stay at rest |
| **W5** Running water: where the map has a current, ripples, foam and floats travel with it | Currents to 3 m/s; a channel that narrows; a hand-made chart here, a solved one in `RIVERS` | A texture sliding over still water. Rejected by a float that must be carried, by foam born at a post standing in the current, and by tracked surface detail |
| **W6** One water: the same light, foam, query and controls on every body, and no seam where two meet | A channel leaving a lake and entering the sea; a swell running; the lake's level and the tide moved through their ranges; a mouth shallow enough for the depth to hold the swell; one craft crossing from one body to another | A renderer for each kind, or a blend drawn over two surfaces that do not meet. Rejected by the surface's height either side of the join, after its depth limit, under a swell and after a stage has moved, and by one query answering across it |
| **W7** Surf: a swell meets a shore as surf. Its lines turn to face the beach, close up and rise; the largest of a set break furthest out; a crest steepens, pitches forward and breaks into a white front that runs into a connected swash | Trade wind to gale with a swell of 6 s to 14 s; beaches of 1 in 30 to 1 in 5; swell square on and 60° off the beach; a headland and a lee shore; the tide through its range; from the sand at 2 m, from 40 m up and from 1 km off | An independent shore wave train, a wave that only fades, or a bore followed by a detached wet ribbon. Follow the incoming ridges into the break zone, then watch a breaking front lose height and feed the advancing waterline. A float must rise on the crest the viewer sees break |
| **W8** Scenery the camera cannot see: water mirrors what stands round it, the far shore, hills, trees and the craft, each joined to its foot at the waterline and blurring as the water roughens | Calm to a breeze on the lake and the pond, calm to a trade wind on the sea under the island; camera 1 m to 40 m up, looking down so the source is above the frame and panning so it leaves the side; two levels in one view; dawn to night | A trace of what is on screen, or a probe at the camera. Rejected by a view whose reflected hill is wholly off screen, and by a tree's reflection that must meet its trunk at the waterline, which a probe's parallax misses |
| **W9** Under water: the waterline crosses the lens; below it the water has its colour and murk and darkens with depth, the bed and hulls show through it, and the surface overhead shows the sky through a bright window with the deep mirrored round it | Every body; the lens from 0.1 m to 20 m under; calm to gale; half in, crossing both ways, and in the swash; looking up, level and down; dawn to night | A tint over the whole frame once the camera's centre is under. Rejected by a lens half under, where each pixel is judged alone, by the window's edge at 48.6° from the surface's normal, and by a panel under water whose light falls with the length of water in front of it once and not twice |
| **W10** Caustics: under a rippled surface the Sun draws a moving net of light on the bed, on hulls and on whatever else is under water. The net is fine and sharp in the shallows and wider and softer deeper, it travels with the waves, and it shows from above through clear shallows as it does from below | Every body; receivers from 0.1 m to 16 m under; glassy calm to a trade wind; the Sun from 10° up to overhead; the lake's water and the sea's | A played texture projected down the Sun's direction. Rejected by a glassy calm, where there is no net, by a net that is sharper on a receiver in the shallows than on one in the deep, with the model's contrast as a diagnostic, by the mean light over a patch, which focusing cannot change, and by a hull's shadow, which holds no net |
| **W11** Shafts: looking up toward the Sun from under water, beams of light stand in the water round its direction. They lean with the refracted Sun, never more than 48.6° from the vertical, sweep as the waves pass, and are missing beneath a hull, where the water is darker than beside it | The lens from 0.5 m to 20 m under; a breeze to a trade wind; the Sun from 10° up to overhead; the sea and the lake | Streaks blurred outward from the Sun's place on the screen. Rejected by beams that stand with the Sun's direction out of frame, by the dark water under a hull seen from the side, and by a calm, where the water's light is even |
| **W12** The camera follows the light down: the picture stays readable from the surface to the deep, deeper water is darker by a share and never black, and crossing the surface makes no jump | The sea to 20 m under and the lake to its bed; dawn, noon and twilight; half in, and crossing both ways | An exposure metered from the sky, which leaves deep water black, or a gain taken from the picture, which pumps as pale sand comes into view. Rejected by the descent, and by the resolved exposure against the model's light at the lens |

## Research and selection

What each source was read for, and how far. The sea's sources come first, as the sea design of 13 and 14 November
read them; the sources for water as a whole follow.

- **Sea of Thieves** ([SIGGRAPH 2018 talk abstract](https://history.siggraph.org/wp-content/uploads/2022/09/2018-Talks-Ang_The-Technical-Art-of-Sea-of-Thieves.pdf),
  both pages read). Its ocean is Tessendorf's FFT. Foam is born at wave peaks and around objects that cut the
  surface, found by depth comparison in a camera-centred window, and the foam buffer is blurred with feedback so
  it disperses. Colour blends deep and sub-surface colours by view, Sun and a wave-peak mask. It has no dispersive
  wake solver. So the bar's wake is a foam trail, and S3 asks for more than the bar; S4's hull foam and dispersal
  are the bar's own.
- **eWave** ([Tessendorf 2014](https://jtessen.people.clemson.edu/reports/papers_files/ewavealgorithm.pdf), pages
  1 to 6 read). Height `h` and surface potential `φ` advance per wavenumber by an exact rotation, equations 19 and
  20: `h' = cos(ωΔt) h + (k/ω) sin(ωΔt) φ`, `φ' = cos(ωΔt) φ − (g/ω) sin(ωΔt) h`, with `ω = √(gk)`. For free propagation
  it is exact for any `Δt` and unconditionally stable, and the FFT form needs a periodic domain. Equations 21 to
  27 state the same step as a convolution in space, so the transform reaches across a whole window, land and
  water alike. The paper's exactness covers deep water and free propagation. It does not cover what this design
  adds: a source, an absorbing border, a bed and dry ground. [Interaction waves](#interaction-waves) states the
  finite-depth update and says how far each holds.
- **Whitecaps** ([Dupuy and Bruneton 2012](https://liris.cnrs.fr/Documents/Liris-5812.pdf), read whole). Breaking
  is where the Jacobian of the horizontal displacement falls below a threshold. Coverage inside a pixel is
  `½ + ½ erf((ε − μ)/(√2 σ))` from the Jacobian's filtered mean and variance, so distant whitecaps average
  correctly from mipmaps. The authors name its limit: no persistence after a wave has broken.
- **Ship wakes** (Rabaud and Moisy 2013, [abstract only](https://arxiv.org/abs/1304.2653)). The wake's bright ridge
  sits at Kelvin's 19.47° at low hull Froude number and narrows roughly as 1/Fr above about 0.5.
- **Unity's HDRP water system** ([simulation](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-water-system-simulation.html)
  and [quality](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-quality-and-performance-decisions.html)
  pages, as summarised by search; not read in full). An ocean is the sum of three periodic FFT bands, two of swell
  and one of ripples. Scripts that float things read a second copy of the simulation run on the CPU, at half or
  full resolution. It simulates the deformed plane only, with no wake solver.
- **Crest** ([ocean simulation page, 4.11](https://crest.readthedocs.io/en/4.11/user/ocean-simulation.html), read
  whole). Wave components are drawn into camera-centred levels of detail that the surface samples. Its dynamic
  waves run a separate simulation in each level, whose wave speed comes from a gravity multiplier and a Courant
  number. Foam is deposited where waves pinch and in shallows, and fades at a set rate.
- **Crest's system notes** ([4.23](https://crest.readthedocs.io/en/4.23.0/user/system-notes.html), as summarised
  by fetch). Its shape lives in "cascaded textures that are centered at the viewer". Each vertex is snapped to a
  stable grid, samples the levels and is displaced, and the data "is carried with the waves away from its rest
  position". The fragment shader combines a normal map with normals computed from the displacement texture.
  Evidence that the drawn surface reads displacement forward at the rest place.
- **Ocean lighting from geometry to BRDF** ([Bruneton, Neyret and Holzschuch 2010](https://artis.inrialpes.fr/Publications/2010/BNH10/index.php),
  abstract and a lecture summary read; the paper itself not read). Waves from kilometres to millimetres are geometry
  near the viewer, then normals, then a BRDF of the slope distribution toward the horizon, with seamless
  transitions between them. LEAN mapping (Olano and Baker 2010, from memory) keeps the mean slope and the slopes'
  second moments in mips. These are linear, so any texture filter averages them correctly, anisotropic filtering
  included.
- **Broadsides**, as what was tried. Its accepted calm and trade-wind sea (W1), its wake and query work packages WP1
  and WP2, and the foam spike's five changes with blind verdicts are evidence that mechanisms of this kind make
  their cues. Its timings are evidence of where cost went: a fused full-rate surface shader at about 2.35 ns a
  pixel, a whole-scene copy, a full-screen reflection trace of about 3 ms, and world work regrouped per draw. Four
  cascades cost it about 0.5 ms an evaluation on this Mac.
- **Not read for the sea design:** the Atlas GDC 2019 slides (over the fetch limit, twice), Crest's source, and
  Horvath 2015 on directional spectra. A search for ocean or wake work published in 2024 to 2026 found none that
  supersedes the sources above. Nothing below rests on what was not read.
- **Unreal Engine's water system** ([overview](https://dev.epicgames.com/documentation/en-us/unreal-engine/water-system-in-unreal-engine)
  and [meshing](https://dev.epicgames.com/documentation/en-us/unreal-engine/water-meshing-system-and-surface-rendering-in-unreal-engine)
  pages, as summarised by fetch). Rivers, lakes and oceans are bodies authored on the landscape, and "because all
  Water Bodies use the same mesh to render water surfaces, they blend seamlessly together"; one shading model
  serves all of them. Evidence for one surface and one optics over data that says where each body is.
- **Unity's HDRP water** ([simulation](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-water-system-simulation.html)
  and [masking](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-decals-and-masking-in-the-water-system.html)
  pages, as summarised by fetch). An ocean or lake has three wave bands, a river two and a pool one, and a mask
  texture attenuates each band separately. Evidence for giving each body a gain on each layer of one wave field.
- **Crest 5** ([water bodies](https://docs.crest.waveharmonic.com/Manual/Basics/WaterBodies.html),
  [shallows](https://docs.crest.waveharmonic.com/Manual/Simulation/ShallowsAndShorelines.html) and
  [shallow water](https://docs.crest.waveharmonic.com/Packages/ShallowWater/Manual.html) pages, as summarised by
  fetch). One global surface whose level an input raises for a lake; flow as an input; waves damped where the
  depth is under half their wavelength, from a bed height field. Its live shallow-water simulation "can be
  difficult to control and thus expectations should be tempered", and its streams "may not reach their end".
- **Horizon Forbidden West** ([Malan, SIGGRAPH 2022 course notes](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf),
  text of pages 1 to 4 and 25 to 36 read). Breaking waves, standing waves round rocks and waterfall impacts are
  offline simulations baked and instanced; "the core of the water system is baking offline sims for runtime use".
- **LightSpeed's Photon water** ([GDC 2023 session page](https://www.gdcvault.com/play/1028829/Advanced-Graphics-Summit-Open-World),
  abstract only). One system that pre-computes water data (height, velocity and foam) from physical equations and
  can update the same data while the game runs. Evidence for prepared flow and a live solver sharing one map.
- **Jeschke and Wojtan 2023** ([project page](https://visualcomputing.ist.ac.at/publications/2023/GSWSDSW/), as
  summarised by search; the older [dynamics design](../../design/modules/WATER-DYNAMICS.md) read it in full). A
  height field split into a bulk flow under the shallow-water equations and surface waves under Airy theory, the
  waves computed at a few depths and blended. This design takes the same split, with the bulk prepared. A search
  for a real-time successor published in 2024 to 2026 found none.
- **Thornton and Guza 1983**, from memory of the result and not re-read: in a saturated surf zone the
  root-mean-square wave height is about 0.42 of the depth, so the significant height is about 0.6 of it.
- **Stockdon et al. 2006**, from memory of the result and not re-read: on a natural beach the waterline's run-up
  is a set-up of the mean level plus a swash about it, each a fraction of the offshore wave height that grows
  with the beach's slope; on a slope of 1 in 10 the swash is of the order of half the wave height. Read for the
  size of the swash, which this design keeps below that figure.
- **Hasselmann et al. 1973 (JONSWAP)**, the fetch-limited growth relations, from memory and reproduced in part I.
- **Vlachos 2010**, [Water Flow in Portal 2](https://www.advances.realtimerendering.com/s2010/Vlachos-Waterflow(SIGGRAPH%202010%20Advanced%20RealTime%20Rendering%20Course).pdf),
  as summarised by search: detail is carried along a flow field by two samples half a period apart, cross-faded.
- **Lagarde 2013**, [Water drop 2b](https://seblagarde.wordpress.com/2013/01/03/water-drop-2b-dynamic-rain-and-its-effects/),
  as summarised by fetch: wet level, flood levels that rise with rain and fall slowly, puddles from a height map.
  Read for `PUDDLES`' brief.
- **The older Thrum water designs** ([commission](../../design/modules/WATER.md),
  [dynamics](../../design/modules/WATER-DYNAMICS.md), [appearance](../../design/modules/WATER-APPEARANCE.md),
  [optics](../../design/modules/WATER-OPTICS.md), [integration](../../design/modules/WATER-INTEGRATION.md)), read
  whole except the last, which was skimmed. Their coverage list (WAT-01 to WAT-10) and their rejection cases are
  carried into the cues and items here. Their mechanisms were never built or run.
- **Not read for this design:** the Far Cry 5 water talk (GDC 2018; no slides found), Uncharted 4's rapids talk,
  Fluid Flux's documentation beyond its feature list, and Barnes et al. 2014 on depression filling. Nothing in
  this module rests on them; `RIVERS` and `PUDDLES` read what their own mechanisms need.

Read for the revision of 14 November, for surf, mirrored scenery and the view from under water:

- **Horizon Forbidden West**, the same course notes, pages 4 to 18 read as extracted text. A breaking wave is
  "a single animated wave cross section": one deformation of a flat surface, baked as a texture over the place
  across the wave and an animation parameter, and applied along wavefronts. The fronts, and where along them
  the parameter passes the point of breaking, are curves artists draw; the cross section began as a Houdini
  simulation and ended as "a hand-animated curve". It stretches the surface by "a factor of 2 or 3 or more",
  so it "needs some really fine tessellation around the wavefronts". Taken from it: a breaker is one section,
  as data, driven by one parameter. Not taken: fronts drawn by hand, apart from the sea's own waves.
- **Linear wave theory over a bed**, from memory of the standard results and reproduced in parts N and O: a
  wave keeps its frequency, so its wavenumber follows `ω² = g k tanh(k d)`; its crests turn by Snell's law;
  its height follows the group velocity (Green's law in the shallows); and a single wave breaks where its
  height nears 0.78 of the depth (McCowan). Komar and Gaughan's breaker height, `0.56 H₀ (H₀/L₀)^(−1/5)`, is
  the published figure part O is held against. Ruessink et al. 2012 (title and abstract, by search) fit the
  skewness and asymmetry of nearshore waves to the Ursell number from 30,000 field records, and advise
  against their fit on slopes steeper than 1 in 30: read for the order in which a wave's shape changes, not
  for coefficients.
- **A pointwise shore transform in a game engine** ([OloEngine, pull request 1041](https://github.com/drsnuggles8/OloEngineBase/pull/1041),
  as summarised by fetch). Gerstner waves take the local depth's wavenumber, Snell's turn and Green's law at
  each point, and break by a cap on height. Its author states the limit: evaluated pointwise and not along
  rays, "individual crest positions drift", and its FFT waves do not turn at all. Evidence that phase has to
  be an integral along the wave's path, which is why the lag here is solved over the bed.
- **Live shore solvers** (the [Celeris](https://arxiv.org/abs/1611.05984) Boussinesq solver and its WebGPU
  port, a Godot shallow-water add-on and a three.js shoreline demo, each from its page or abstract). Real-time
  depth-averaged solvers shoal, break as bores and run up a beach on grids of about a thousand cells a side.
  Celeris itself is GPL and is not read as code.
- **Planar reflection in shipped engines.** [GTA V, Courrèges's frame study](https://www.adriancourreges.com/blog/2015/11/02/gta-v-graphics-study/)
  (as summarised by fetch): the scene is drawn again, in about 650 draw calls, into a 240×120 texture flipped
  to serve as the water's reflection, beside an environment cube of scenery only. [Unreal's planar
  reflections](https://dev.epicgames.com/documentation/en-us/unreal-engine/planar-reflections-in-unreal-engine)
  (as summarised by fetch): the level is drawn again from the mirrored direction; its cost follows the
  scene's complexity and not the reflection's resolution; it "can reflect off-screen objects" where
  screen-space reflection leaks at the edges; a capture is cut down by a lower resolution, a shorter view
  distance and a coarser level of detail, and widened by extra field of view where normals push samples past
  its edge. Red Dead Redemption 2 has a water-reflection quality setting apart from its screen-space one;
  no frame study of it was found.
- **Ray query in wgpu** ([tracking issue 6762](https://github.com/gfx-rs/wgpu/issues/6762) and
  [pull request 10289](https://github.com/gfx-rs/wgpu/pull/10289), as summarised by search). Acceleration
  structures and ray queries run on Metal as an experimental feature from wgpu 29, with open synchronisation
  defects, and are not in the WebGPU standard. So ray-traced reflection cannot be the portable method, and is
  a candidate for the Apple recipe later.
- **Under water in shipped systems.** [Crest](https://crest.readthedocs.io/en/4.23.0/user/underwater.html)
  (as summarised by fetch) draws a full-screen effect after transparent geometry, the surface two-sided, a
  meniscus line where the lens meets the water, and fog from the opaque depth, so anything transparent takes
  too much of it. [HDRP](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-underwater-view.html)
  (as summarised by fetch) applies an analytic absorption full-screen and draws a boundary at the water line.
  Neither page says how a pixel is found to be under water or how the underside is lit; this design derives
  both.
- **Not read for the revision:** a frame study of Red Dead Redemption 2 or Sea of Thieves, Abreu et al. 2010
  on a waveform with a given skewness and asymmetry, Jeschke and Wojtan's 2018 wavelets beyond their
  abstract, and any shipped game's surf other than Horizon's. A search for surf talks at GDC or SIGGRAPH in
  2024 and 2025 found none.

Read for the second revision of 14 November, for light under water:

- **Caustics from the simulated surface.** [HDRP](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/water-caustics-in-the-water-system.html)
  (as summarised by fetch) computes caustics from one band of its wave simulation, the ripples by default,
  into a buffer at a resolution it names, and projects them at one "virtual plane distance" that the author
  raises "until the caustics look right". They do not fall on things above the water or on transparent
  things, and they ignore current maps. The page does not say whether a shadow holds them or whether the
  water's volume shows them. Taken: the net is computed from the waves. Not taken: one plane for every depth.
- **A played texture.** [Crest 4.23](https://crest.readthedocs.io/en/4.23.0/user/water-appearance.html) (as
  summarised by fetch) projects a caustics texture with a scale, a strength, a focal depth, a depth of field
  and a distortion texture. Its underwater section has no shafts and says nothing of exposure.
- **Carrying the surface's light down.** Evan Wallace's WebGL Water (2011) and
  [his write-up of 2016](https://medium.com/@evanwallace/rendering-realtime-caustics-in-webgl-2a99a29a0b2c),
  which refused a fetch and was read as a search summary, with Renou's 2020 extension: a mesh of the surface
  is moved along its refracted rays, and brightness is the ratio of a triangle's area before to its area
  after, so light spread out dims and light gathered brightens. This is the mechanism selected, on a ladder
  of depths.
- **Shafts from the same pattern.** CryEngine 2 (Sousa, GDC 2008, by search summary): underwater god rays
  were "essentially the same procedural caustics shader, based on the sun's direction". NVIDIA's ray-traced
  water caustics (Gruen, GDC 2019, session abstract by search) trace surface caustics and volumetric
  caustics with shadows. Evidence that the net, the beams and the shadows in them are one light; its method
  is not portable.
- **Exposure under water.** No shipped system's account was found. Unreal's forums (by search summary)
  handle it with a post-process volume that narrows the auto-exposure range. Thrum's meter is a physical
  incident meter ([C07](../../design/CONTRACTS.md)), so the first-principles answer is the light at the lens.
- **From memory, not re-read:** Cox and Munk's mean square slope, `0.003 + 0.00512 U`, which part R's
  surface uses; and Petzold's phase function for sea water, whose mean cosine is about 0.92.
- **Not read for the second revision:** how Sea of Thieves or Subnautica draw their caustics and shafts (no
  talk or frame study was found), Wallace's article itself, and any measurement of caustic contrast in the
  sea. Part R's reference is this design's own ray count.

Reproductions A to G were run for the sea design and are kept with their output in
[the sea's evidence folder](../../../.orchestrate/evidence/SEA/design-proof/). Parts H to M were run for this design and are in
[this module's](../../../.orchestrate/evidence/WATERS/design-proof/): H the interaction step at depth, I shelter, J dry ground,
K joins, L the swash and M foam in the swash. Parts N to Q were run for the revision and are in
`surf_view_proof.py` there: N the shore transform, O breaking, P the mirror lookup and Q light under water. Parts R to T were run for the second revision and are in
`under_light_proof.py` there: R caustics, S shafts and T the meter. Two design reviews of 14 November are kept in
[the evidence folder](../../../.orchestrate/evidence/WATERS/), each with its own reproduction. After the first, H was run again
and J to L added. The second found a step at a shallow mouth and no carriage for foam up a beach: K now
assembles the surface after its depth limit and reproduces that review's case, and M was added.
[Check that it can work](#check-that-it-can-work) reports them.

### Judged from first principles

Every choice this design inherited, from the sea design, the water decision or the older water designs, judged
against the users and cues above. "Keep" means it would be chosen again with the earlier work out of the room.

| Inherited choice | Verdict | Why |
|---|---|---|
| A module for the open sea, with inland water left to designs nobody builds | **Change** | The sea is the one body with no bed, no shore, no current and one level. A surface, optics, foam and query built for it alone assume all four, and the rest of water is where each fails. One system is designed here, and the sea is its first body |
| The wave model and its GPU fields (`SEA` WP1 and WP2) | **Keep**, as built | Wind waves are one field for the world, with their own oracle. Every body reads them, and nothing in them depends on where water lies |
| One level, the sea's, everywhere below the horizon | **Change** | Water lies at many levels. A map says where, and the surface is drawn body by body |
| Inland water as a live generalised shallow-water solver on the GPU (the water decision and the dynamics design) | **Change**: flow is prepared once | A river is steady nearly always. A prepared flow is the same on every machine and readable by a CPU, costs nothing in the frame and exists far from the camera; live solvers are hard to keep stable and to make reach their end. The solver's split is kept: a bulk in the map, waves on it. Whether floods ever move is [open for the owner](PLAN.md#open-for-the-human) |
| Bodies read the GPU through one batched query, one to two ticks late | **Keep**, for all water | Interaction waves exist nowhere else. The answer gains the body of water, its depth and its current |
| The wake: eWave on two body-following windows with complementary bands, deep water only | **Keep**, with the depth and with regions | The step is a rotation for each wave vector on any uniform depth, once the depth is in its coefficients as well as its frequency. A bed that varies is met by a ladder of depths blended per cell, which is Jeschke and Wojtan's own device, and land by solving each connected region of water alone (parts H and J). On deep open water it is unchanged |
| A wake source that is the body's own reaction force | **Keep** | Physical, with no gain to tune. Impulses a game posts join it, for a dropped stone |
| Foam as four transported stocks, born only from booked causes | **Keep**, with more causes | The stocks and their chain are the same on any water. Shallows, a current meeting a post and, later, a river's loss of height are causes of the same kind |
| Reflection and refraction of registered bodies only; the scene is never copied | **Change** | True of an open sea, which is thick and empty. In a lake the bed is what a viewer sees in the shallows and the far shore is the reflection. The copy is kept regional: nothing on an open sea, the screen where shallow or mirror-still water fills it |
| Water reflects only what is on screen; no module plans the probe that the rest needs (this design's first version) | **Change**, on the owner's words | A lake under a hill is a mirror of the hill, and the camera looks down at it with the hill above the frame. A screen trace has nothing to read there. The scene is drawn again from the camera's mirror image, by [a new module](#what-waters-asks-of-other-modules), and the water reads that |
| The camera stays above the water (the sea brief, and this design's first version) | **Change**, on the owner's words | A camera that follows a craft, a swimmer or a wave goes under. The surface is drawn from both sides and the water between the lens and what it sees is composed as a medium |
| Waves fade at a shore and do not shoal, turn or plunge; surf is not planned (this design's first version) | **Change**, on the owner's words | The first version held that a height field cannot overturn and that shipped surf is baked. Both are true and neither stops it: the long waves are few and analytic, so each can carry a phase solved over the bed, and a displaced sheet can lean past the vertical. The depth limit and the swash are kept: they are the surf zone's own energy law |
| Under water without caustics or shafts of light, on an exposure metered from the sky (this design's first revision) | **Change**, on the bar the owner set | Each is what a viewer of a shipped game expects on going under, and leaving them out made risk 24, deep water that is black, a declared defect. The waves the surface is drawn from already hold the net: it is their focusing of the Sun |
| One sea, all of it or none; then capabilities a scene leaves out | **Keep** the capabilities | A sunset demo and a puddle need neither wake, foam nor query |
| Rigid bodies integrated by the game | **Keep** | Thrum has no physics owner, and the engine must hold no hull |
| Whitewater, optics and dynamics as separate providers with packets, charts, ray-traced regional optics and a work ledger (the older appearance, optics and integration designs) | **Not selected** | They specify a larger product than any cue here asks for, none was run, and their costs were never counted against a frame. Their rejection cases are kept as items |

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each work package judges fit file by file. Paths are in the Broadsides repository at
`ae8c1ebb7` unless a commit is given, and nothing there is ever changed. The wave kernels are listed in
[the sea's design](../SEA/DESIGN.md#what-broadsides-offers).

| Mechanism here | Where Broadsides has it | Expected fit |
|---|---|---|
| The eWave step: height and potential as one complex field, the exact turn, a source carried through the step, a damped border | `crates/render/bw_render_water/shaders/ocean_wake.wesl` and its CPU twin `crates/sim/bw_water/src/wake/grid.rs` | Close for one deep level. The two banded levels, the finite-depth update, the depth ladder, the regions and the step of several ticks are this design's |
| The batched query and its ring | `shaders/body_query.wesl`, `body_sea_query.wesl`; `crates/sim/bw_water/src/ocean/query.rs` | The kernels may fit; the mailbox and packing are written for Thrum's tick page |
| Foam stocks, deposition, transport and the stochastic pattern sampler | At `2e5f47184`: `crates/gpu/bw_gpu/shaders/lib/sea_foam.wesl`, `foam_pattern.wesl`, `whitewater_medium.wesl`; `crates/render/bw_render_water/shaders/ocean_foam.wesl`, `ocean_deposition.wesl` | The two changes the spike passed are here. Check each rule against [whitewater](#whitewater) before copying |
| The ranked foam pattern | At `2e5f47184`: `assets/textures/water-foam/ranked.png` (Git LFS) and its recipe in `factory/materials/water-foam/` | Usable as the example's pattern. It derives from Crest's foam texture under the MIT licence, and that notice comes with it |
| Hydrostatics, hull stations, a hull | `crates/sim/bw_water/src/hull/`, `body.rs`; Broadsides' ship assets | Game-side code and content for `examples/water` |
| Water optics: Fresnel, in-scatter, the microfacet Sun | Functions in `shaders/ocean_surface.wesl`, `ocean_volume.wesl` | Take functions. The pass they sit in is where the 21 ms went and is not copied |
| Lake, coast and shallow-water code; sky, exposure, temporal AA, the Bevy host, `RenderModule`, `pass_list`, `TargetPool`, frame packets | | Not used. The inland code was never accepted, and Thrum has one owner for each of the rest |

Broadsides shaders are WESL and Thrum's are WGSL, so a copied shader has its imports resolved into Thrum's
modules; its algorithm is left as it is. [The handoff](#proof-and-builder-handoff) says how a copy is recorded.

### The interaction operator, decided against the cues

| Candidate | Verdict |
|---|---|
| A wake decal or foam trail only (the bar's own) | Fails S3: nothing lifts a float |
| Constant-speed wave equation on a grid, as in Crest's dynamic waves | Fails S3. In the reproduction its ridge follows the Mach angle, 42° at 3 m/s and 17° at 6 m/s, and it has no transverse wave train |
| Steady Kelvin pattern evaluated per body | Fails S3: it has no memory, so it cannot persist through a stop or a turn |
| Wave packets or wavelets (Jeschke and Wojtan 2017, 2018) | Correct dispersion, but cost grows with packets and a rasterised field is still needed for the surface and the query. Not selected |
| Generalised shallow water with dispersive waves (Jeschke and Wojtan 2023) | Its split of a bulk flow from the waves on it is this design's, and so is its blend of a few depths. Run whole and live, it pays every tick for a bulk that does not change, and its stability on a shore was never qualified here. Not selected |
| **eWave on body-following periodic windows, two levels with complementary bands** | **Selected.** Exact dispersion at any speed on uniform depth, a few FFTs a tick, and stable at any step. Its transform is global, so by itself it knows neither a bed that varies nor land; [interaction waves](#interaction-waves) says how it takes a bed, a shore, dry ground and a current, and what each costs in accuracy |

Two levels, because one 256² window cannot both resolve a 0.5 m ripple at the hull and hold a 25 m transverse
wave: a 64 m window of 0.25 m cells does the first and a 512 m window of 2 m cells the second. Two overlapping
periodic levels can only avoid carrying one wave twice if each owns a band of wavenumbers, and the reproduction
shows what happens otherwise: two full-band levels summed are wrong by 69% of the field's RMS height, against 2.2%
with the split.

**What would change these selections.** A cost record that shows a representation to be wrong, and not merely
untuned, reopens it through a proposal in the log ([cost account](#cost-account)). A cue of `RIVERS` or `PUDDLES`
that a seam here cannot carry reopens that seam in [the module list](../ARCHITECTURE.md#water-as-one-system).
Neither stops the work packages after it.

### Surf, mirrored scenery and the lens, decided against the cues

| Cue | Candidate | Verdict |
|---|---|---|
| W7 | Waves that only fade under the depth limit | Fails: nothing turns, closes up, rises or breaks |
| W7 | A wave train drawn along the shore's distance field | Fails: it is a second sea. The crest that breaks is not the swell's, a float cannot be told which to ride, and its sets are not the wind's |
| W7 | The local depth's wavenumber and Snell's turn applied at each point (OloEngine) | Fails: phase is not an integral, so crests drift and tear |
| W7 | Baked simulations placed along drawn fronts (Horizon Forbidden West) | The look to beat. Not selected: the fronts are authored for each beach, apart from the sea state, the tide and the query, and the bake needs an offline fluid pipeline this project does not have. Its one-section breaker is taken |
| W7 | A live Boussinesq or shallow-water window fed by the wave field | Does shoal, break and run up. Not selected: a second solver whose state must agree with the analytic field at its border, a time step bound to the cell size through every hitch, no far surf outside its window, and the control problems Crest reports. It remains the route if the selected one cannot meet W7 |
| W7 | **Each long wave carries a phase lag solved over the bed. The crest takes a breaker section by its breaking index and feeds a bounded swash profile** | **Selected.** The swell's own crests turn, close up, rise and break. A localized deformation dissipates the bore and the same incoming swell moves one wet edge. CPU and GPU evaluate the same sheet |
| W8 | A screen-space trace of the scene (this design's first version) | Fails by construction when the source is off screen. Kept for water that has no mirror view |
| W8 | A cube probe drawn round the camera, with or without parallax correction | Fails the waterline: a probe is right at one point, so a reflection parts from its foot by the probe's distance from the surface point. Six faces cost more than one mirror |
| W8 | Ray query against the scene's geometry | Right in principle. Not portable: experimental on Metal in wgpu and absent from WebGPU. A later provider for the Apple recipe, promoted by a measured win |
| W8 | **A mirror view: the scene drawn again from the camera's mirror image in the water's plane, read with its depth** | **Selected.** Exact on flat water for everything above the plane, on screen or off; what Unreal and GTA V ship; one more view, whose cost its owner controls |
| W9 | A tint and fog once the camera's centre is under | Fails a lens half under |
| W9 | **A per-pixel lens test against the surface's own function, the surface drawn two-sided, and the water composed as a medium from the gather** | **Selected** |
| W10 | A played caustic texture projected along the Sun (Crest; the bar's own, as far as is known) | Fails: the same net in a calm and in a gale, a repeat, and a brightness that is not the light's |
| W10 | One map made at one depth and read at every depth (HDRP) | Fails the range. A map made at 1 m and read at 0.25 m has nearly three times the contrast of the net there, and read at 4 m and 16 m it correlates 0.30 and 0.07 with the net there (part R5) |
| W10 | A backward lookup: from the receiver, one sheet's brightness from the surface's curvature above it | Fails past the first focus, where sheets cross and the net's bright lines are. It does not conserve light (a mean of 1.30 at 1 m) and correlates 0.29 there (part R5) |
| W10 | Rays traced from the Sun through the surface (NVIDIA) | Right. Not portable |
| W10 | **The surface's light carried down its refracted rays onto a ladder of depths, each cell spread over where it lands** | **Selected.** It conserves light by construction, crossing sheets add, it is made from the waves the surface is drawn from, and it is what Wallace's and HDRP's methods do at one depth |
| W11 | Streaks blurred outward from the Sun's place on screen | Fails with the Sun out of frame, and knows nothing of a hull |
| W11 | The water as a medium in `ATMOSPHERE`'s view volume | Not selected. C08 gives water a declared boundary and this design composes the water's segments in its own pass; the volume's Sun path is straight where the water's bends at the surface; and a lens half under would put two media in one volume |
| W11 | **The beam's once-scattered light, marched through the caustic field and the scene's Sun visibility in the water's own composition** | **Selected** |
| W12 | A meter on the rendered picture | Not selected: V4's meter is a physical incident meter, and a picture meter pumps |
| W12 | **The incident meter reads the light at the lens: the game gives its camera the water's factor there** | **Selected** |

## Mechanisms

### One water in four parts

| Part | What it is | Its state | Who writes it |
|---|---|---|---|
| The map | Where water lies and how it moves in bulk: for any point, the body of water there, its level, the bed, the depth and the current | Content, prepared before the frame, with one number a body that a game moves: its stage | The game's declarations and bed; `RIVERS`' solver for charts |
| Waves | What rides on the bulk: the wind's wave field at each body's gains, carried by its current, and the interaction waves bodies and impacts raise | The field is a function of time (`SEA`); interaction waves are state | `SEA`; this module |
| Material | What the water carries: foam and submerged air | State, moved by the map's current | This module |
| Optics | One equation from the surface to the eye | None | This module |

This is the split Jeschke and Wojtan's solver makes inside one time step, bulk flow under the shallow-water
equations and waves under Airy theory, taken apart so that the bulk can be prepared. Every reader of water (a
vertex, a pixel, a force query, a foam birth) asks the map first and adds the waves at that place. A body of water
is a row of data for the four parts and never a renderer of its own, which is what keeps water one system as
kinds are added.

### The water map

**Bodies.** A game declares its water as bodies of three kinds, over a bed.

| Kind | Declared by | Its water |
|---|---|---|
| Sea | A level | Everywhere the bed lies below the level, and everywhere beyond the bed's extent. Unbounded |
| Basin | A seed point and a level | The connected hollow around the seed where the bed lies below the level. A lake, a pond, a pool, a canal, a puddle. A second hollow beyond a ridge stays dry until the level passes the ridge |
| Chart | A grid of level and current over a rectangle, with the loss of height in the flow, a footprint, and its ends: where it leaves one body and enters another | Where its level lies above the bed, inside its footprint. A river or a stream, written by `RIVERS`' solver; in this module a hand-made channel |

**The bed** is a height grid with a declared cell size, given by the game or built by `thrum-water-model` from
the triangles of the game's ground. Beyond the bed's extent the water has no bed in reach.

**One function answers for any world point**: the body there or none, its level, the bed, the depth (level less
bed), the current, and the body's gains. It is written once in `thrum-water-model` in f64 and once in WGSL, the
two are held to each other, and neither reads the camera. Where declarations overlap, a chart is taken over a
basin and a basin over the sea; two bodies of one kind that overlap are refused at preparation, by name.

**Joins: how connected bodies meet.** Taking one body over another says who owns a point. It does not make two
surfaces meet: a sea with a swell and a chart without one, or a lake raised a metre above a chart left as
prepared, would stand apart at the line between them. Still bodies never touch each other, so every join is an
end of a chart, and the join is made in the map, where every reader finds it.

- *Ends.* A chart declares a **source end** and a **mouth**: each is a stretch of its footprint's edge and the
  body it joins there. A spring has no source body, and a tributary's mouth joins another chart.
- *The mix.* Inside its footprint a chart's row (level, current, the four gains and the water type) is mixed
  toward the joined body's row by a weight. The prepared weight is 1 on the end's edge and falls smoothly, with
  no slope at either limit, to 0 a join length inside, a value of the chart (12 m for the example's channel). At
  the mouth the weight is also raised by **backwater**, measured from the receiving body's **reach**: its level
  plus the run-up a regular wave gives at the largest swash depth, which is 0.175 of the profile's margin height
  ([below](#waves-on-a-body-gains)). The weight is 1 wherever the reach stands at or above the chart's own
  level, and falls smoothly to 0 where the chart's own level is 0.25 m higher still. A rising tide therefore
  drowns the lower channel, which becomes the sea's water: the sea's level, its swell, and no current. And
  because a chart's banks stand above its own level, sand low enough for the sea's sheet to wash lies beside a
  stretch of channel that is already the sea's.
- *A reader takes nothing from a body but its row.* The level, the current, the gains and the water type are
  the row's. What follows from the gains, which is the swash depth, the wave support and so the depth limit, is
  the row's at the point too, and never a number kept for a body. The bed is the map's and the wave field is the
  world's. So the whole surface calculation has the same inputs wherever two rows agree.
- *What follows.* Where the weight is 1 the chart's row is the joined body's row. So the displaced surface after
  its depth limit, its normals, wet now, the foam's births and the query's answers are the same either side of
  the line, for any stage of either body, any sea state and any depth of water at the end. Swell enters a mouth
  and fades up the channel with the weight, and the swash on the channel's banks fades with it. The point's
  body stays the chart on its footprint.
- *Not a drawn blend.* The mix is the map's, in `thrum-water-model` and in WGSL alike, so it is the physical
  answer and the conformance model's too. Nothing is blended in the surface pass, and no reader sees a join the
  query does not (C04, C09).
- *A basin's fill stops at a chart's footprint*, so a lake does not pour down its own outlet.

**What preparation refuses.** Each body's stage has a declared range. Over those ranges, by name: a chart with a
cell on its footprint's edge whose neighbour outside is neither ground standing above the chart's level there nor
a joined body's water at weight 1; and a basin whose fill reaches the sea, another basin, the edge of the bed, or
a chart anywhere but across its end. So a chart's water is held by its banks, a lake's outlet lies where its
banks stand above its highest stage, a source never drops below its outlet's sill, and a chart reaches down to
its mouth's lowest level. A lagoon open to the sea is the sea. Two basins that would merge are refused here;
merging is [`PUDDLES`' question](#what-the-later-modules-build-on).

On a 200 m island with a channel from a lake to the sea, with the lake moved from −0.5 m to +1 m and the tide by
±1 m under a 0.5 m swell crest, taking one body over another with one stage a chart leaves steps of 0.5 m to
1.5 m between bodies and stands the channel 0.95 m over its banks at low tide. With joins the surface is the
same to four decimal places a millimetre either side of each end, and nowhere over a bank (part K).

The same holds after the depth limit, which is where a shallow end is decided. A swash depth kept once for each
body would leave a step there, because a chart's own waves hold no swell: 0.21 m at a mouth 0.25 m deep under a
0.5 m swell with no wind (the second design review's case), 0.07 m and 0.21 m at the island's mouth at mean and
low tide, and 0.013 m at a source's sill 0.05 m deep. With the swash depth taken from the row the step is none,
at every depth from 4 m to none and through both stages' ranges. Beside a mouth, on sand 2 cm above the
channel's own water, backwater measured from the sea's still level would leave the sea's sheet up to 0.10 m
from the chart's; measured from the reach it leaves none (part K).

Declared approximations of a join: the mix is linear in the weight; a chart keeps its prepared flow whatever
its source's stage, so the reach inside the source join steepens or flattens and the river below does not swell
(how a river answers its lake is `RIVERS`'); swell in a drowned mouth is held by the depth and not by the
channel's width; and the reach is one regular wave's, so where several layers crest together and climb a few
centimetres past it, a sheet's thin edge can lie beside a chart whose weight is short of 1 (0.94 or more in
part K's case), and beside a source end, which has no backwater, a lake's lap of a few centimetres can lie
beside the chart's own water.

**One water surface in a column.** The map holds one body at a point. Water above water is not planned
([plan](PLAN.md#open-for-the-human)).

**Stage.** Each body has a stage, in metres, which the game sets with the tick, inside the body's declared range,
and which is added to its level: the sea's tide, a lake's level, a puddle's fill. A stage set outside its range
is held at the range's end and counted. A basin's hollow is filled again from its seed when its stage moves, so
a shoreline travels up the ground and a neighbouring hollow floods only when the water reaches its sill. A
chart's stage raises its own level and leaves its current as prepared, a declared approximation; its ends follow
the bodies they join.

**The shore margin.** Waves wash ground above the still waterline, so the map names a body on its still water
and on its **shore margin**: the ground that rises from its waterline to the profile's margin height above its
level, reached from the still water by a path that never descends. A hollow beyond a ridge is therefore never
in it. Depth there is negative. Where two margins meet, the ground is the body's whose level is higher. A
point on a margin takes its body's row from the water it rises from, so a chart's bank inside a join carries
the weight of the channel beside it. The
margin is content, as the fill is, and moves only with a stage. Three things are then distinct, and each reader
names the one it uses: **still water** (depth above zero), **wave support** (depth above minus the swash
depth there, [below](#waves-on-a-body-gains)), and **wet now** (the surface at this moment stands above the bed).
Wet now is the physical wet support that the surface, the optics, the query and the foam's births all use (C07).

**What the map is not.** It is not what a viewer sees of a shoreline: the visible waterline is where the drawn
surface meets the scene's own depth, to the pixel, and the map's cells never show. It holds no waves.

### Waves on a body: gains

The wind's waves are one field for the world, `SEA`'s: long waves of 20 m and more, and three cascades below
([its design](../SEA/DESIGN.md#one-field-two-layers-each-wave-carried-once)). The field is generated for the
scene's most exposed body. Every body takes each of the four layers at a gain between 0 and 1, and the surface, the
query and the foam's births all read the layers through the same gains.

- **Shelter.** A body's gains come from its fetch, which the game declares, and the wind: each is the square
  root of the energy a fetch-limited spectrum has in that layer's band over the energy the field's own spectrum
  has there, from `thrum-sea-model`. Part I, for a wind of 10 m/s over a sea 2.1 m high: a pond with 200 m of
  fetch takes 0, 0, 1 and 1 of the four layers and stands 0.07 m high; a lake with 1 km takes 0, 0.25, 1, 1; with
  5 km, 0, 0.76, 1, 1; with 50 km, 0.49, 1, 1, 1. A swell, which is part of the long waves, adds nothing to
  any body's own waves but the sea's: a basin's and a chart's own gains carry what the wind raises over their
  fetch and no more, so with a swell running and no wind their long-wave gain is zero. Where a long fetch does
  raise long waves they are the field's long components at that energy, a declared approximation. A chart
  takes the sea's long waves, swell among them, only through [its mouth's join](#the-water-map).
- **Shallows and the swash.** At any point each layer is held so that its significant height is at most 0.6 of
  the depth plus the **swash depth** there, longest layer first; 0.6 is the ratio of a saturated surf zone. The
  swash depth is the profile's swash ratio (0.7 in the first profile) times the significant height the point's
  row gives the field: each layer's energy this tick at the row's gain, before the limit. It is never more than
  the margin height, and nothing in a calm. It follows from the row and is not kept for a body: over a body away
  from a join the gains are one set and it is one value, and inside a join it follows the mix, so the limit and
  the wave support meet across an end as the row does. Above the still waterline the depth is negative and the
  same expression holds, up to where it reaches zero. At 1 m of depth a 2.1 m sea is held to 0.28 of its height by the depth alone (part I).
  With the depth alone the waves would end at the still waterline, and nothing could move it: an admissible
  wave's trough never bares the bed, and the map gives no water beyond. With the swash depth they keep a
  remainder there, and the displaced surface crosses the beach along a line that travels. For one regular wave
  the waterline runs up to 0.175 of the swash depth above the level and back to 0.27 of it below: 0.26 m up and
  0.40 m down in a 2.1 m sea, which is 6.7 m along a beach of 1 in 10; 3.1 m in a 1 m sea; 0.5 m on a lake with
  1 km of fetch; and it stands still when the swash depth is zero (part L). That is the set-up and swash a surf
  zone has, in the limiter's terms, and smaller than Stockdon's figure for such a beach. By itself it is not
  surf: nothing shoals, turns or overturns until [the next section](#surf-the-shore-transform-and-the-breaker)
  adds that. The power the limit takes from the waves is reported, and it is what foam is born
  from at a shore ([whitewater](#whitewater)).
- **Roughness from the flow** is a gain `RIVERS` adds to the shortest cascades where a chart's water runs fast
  and shallow. The seam is the same four numbers.

A gain multiplies a layer's displacement, slopes and rate of change, and its square multiplies the layer's slope
and Jacobian moments. The slope a changing gain itself adds is left out: gains change over many wavelengths
except on a steep shore, where waves are already small, and inside a mouth's join, where a 0.5 m swell fading
over 12 m is left out of the shading normal by under 4° while the geometry carries it. Under the gains alone
waves do not shoal, turn or shorten as water shallows.

### Surf: the shore transform and the breaker

Surf is a capability a profile names, and it needs waves. Without it everything above stands as written: a wave
crosses a shelf unchanged and fades under the limit. With it, the long waves of a still body that has a bed,
the sea or a lake whose fetch raises long waves, are transformed as they cross the bed, and their crests take
a breaker's shape. The cascades are periodic patches that cannot turn, so the waves under 20 m are held by
the depth as before and neither turn nor shorten, a declared approximation: they are a small part of a
swell's length and die within a few metres of the waterline.

**A long wave over a bed.** A component with deep-water wave vector `k₀`, frequency `ω`, amplitude `a` and
phase `φ` becomes

`a · K(x) · cos(k₀·x + |k₀| λ(x) − ωt + φ)`,

where `λ`, the **lag** in metres, is what the bed has delayed the wave by, and `K` is the gain of its height.
It keeps its frequency. Its local wave vector is `k₀ + |k₀|∇λ`, and its horizontal displacement lies along
that.

- *The lag* solves the eikonal equation `|k₀ + |k₀|∇λ| = k(d)`, where `k tanh(k d) = |k₀|`, as the first
  arrival from the wave's **source**, where the lag is zero. Ground above the shore margin is closed to it, so
  a wave reaches the lee of an island round its sides and never through it. The depth is floored at the
  profile's lag floor, 0.5 m in the first profile, so the lag stays finite across the swash.
- *Where a wave starts.* The source depends on where the water's waves come from, and is decided for each
  connected piece of a body's water at each stage.
  - **Water that reaches the bed's edge** takes its waves across that edge: its swell and its wind sea arrive
    from outside. Its source is the bed's edge. Over a border there, an eighth of the bed's width, the depth
    the solve sees rises smoothly to deep water, so a wave meets the bed with no step in its length.
  - **Water that land encloses**, a lake in its hollow or a part of the sea cut off at that stage, has no
    path to the bed's edge, and its long waves are the ones the wind raises over its own fetch. They start
    at its own shore. A point's **sector fetch** is the longest stretch of open water upwind of it over the
    directions of the component's 45° sector. The source is the water where the sector fetch is zero: the
    waterline that faces the wave, where the wave leaves the land with the plain wave's phase. A shore the
    wave runs along has a long sector fetch and is no source, so the wave turns in toward it as it does at
    sea, and a downwind shore is never one, so nothing runs back out of a beach.
- *What is prepared.* The wave lattice has eight directions. For each, `thrum-water-model` solves the lag on
  the bed's grid at wavelengths a factor of two apart from 20 m to 640 m, and at stages a set spacing apart
  across the body's declared range, 0.5 m in the first profile. A component reads its own direction's lag,
  blended linearly in the logarithm of its wavelength and in the stage. Components longer than 640 m are not
  transformed. The lags are content, as the map is: they change only when the bed or the declared bodies do.
- *Exposure.* A component stands for a 45° sector of directions, and its energy at a point is scaled by its
  exposure there: the mean, over the sector's directions, of how much of the wave each lets arrive. For water
  that reaches the bed's edge a direction counts 1 if a straight line upwind reaches the bed's edge without
  crossing ground that stands above the body's highest stage, and 0 if not. So a windward beach takes a
  train whole and a fully sheltered lee beach laps with short waves. The sea's exposure blends back to
  ambient over the same prepared bed border, so far downwind the swell returns without a line at the edge
  of the bed. For enclosed water a direction counts the open water upwind
  along it over the body's declared fetch, and at most 1: the share of that fetch the wind has had, since a
  fetch-limited sea's energy grows in proportion to its fetch. So a lake is calm under its upwind shore,
  where the wave leaves the land with nothing in it, and carries the whole energy of its gain where the
  open water upwind is as long as the fetch the game declared.
- *Height.* `K = Ks(|k₀| d) · √exposure`, where `Ks = √(Cg₀/Cg)` is linear theory's shoaling coefficient at
  the point's depth, floored as the lag's is. The depth limit of [the last section](#waves-on-a-body-gains)
  then holds the long layer's significant height, which is now the transformed components' at the point,
  exactly as before. The swash depth is still taken from the row's gains alone, before any transform.

For an 80 m wave on a plane beach of 1 in 10, the solve on 2 m cells is within 0.34 rad of Snell's closed
form at 0°, 30° and 60° of incidence (0.69 rad on 4 m cells: the scheme is first order), and its crests in
2 m of water lie 10.9° and 19.1° off the shore's normal against Snell's 11.1° and 19.5°. Blending wavelengths
a factor of two apart misstates the local wavenumber by at most 2.0%; a factor of four would misstate it by
7.6%. Blending stages 0.5 m apart misstates it by at most 2.6% in a metre of water or more and by up to 7.9%
between the floor and a metre; a spacing of 1 m would give 11.5% and 13.8%. A component between two
wavelengths at a stage between two samples takes both blends at once, and together they misstate it by up
to 4.1% in a metre of water or more and 9.3% between the floor and a metre, at incidences from 0° to 60°. On
the grid solve's own lags at 2 m cells, against a lag solved for that wavelength and stage itself, the two
blends are 5.0% and 8.0% out, and a component given its nearest wavelength and stage with no blend is 35%
out (part W). Round an island, a wave from the
west meets the windward shore square on, the windward quarter 26° off the shore's normal, the side 45° and
the lee quarter 57°, where the untransformed wave runs 45°, 90° and 135° off and, in the lee, straight out
of the land. Its exposure at those places is 1, 1, 1, 0.47 and, in the lee, 0. Wherever the exposure is above
zero the transformed wave runs at most 60° off the shore's normal in 4 m of water, 39° in 2 m and 28° in
1 m (part N).

In a lake that land encloses, 1.4 km across and 20 m deep, no cell has a path across water to the bed's edge,
so the bed's edge alone gives it no lag at all. With its own waterline as the source every cell has one. On
the 2 m contour a 40 m wave meets the downwind shore square on, and 30°, 60° and 90° round from there it runs
11°, 20° and 33° off the shore's normal where the plain wave runs 30°, 60° and 90° off. Its exposure at
those four places is 0.95, 0.83, 0.52 and 0.19 for a declared fetch of the lake's width, and 0.03 at the
upwind shore, where the wave leaves the land. In the lake's deep water a 20 m and a 40 m wave keep the plain
wave's length within 0.9% and 1.3% and run at most 11° and 9° off its direction, bent by the shelf they left;
a run of deep water in the solve's depth at the source, four wavelengths long, brings that to 9° and 6°, and
is not taken (part U).

**A crest and its breaking index.** The transformed long waves are summed as a complex field, `Z = Σ a K e^{iθ}`, whose
real part is the plain surface. Its modulus `A` is the envelope, its argument `Θ` is the phase of the crest
that stands there, and the gradient of `Θ` gives that crest's direction of travel. All three come from sums
the evaluation already makes. The crest's **breaking index** is `b = 2A / (γ d)`, with `A` taken before the depth
limit, `d` the still depth floored as the lag's is, and `γ` the profile's breaker ratio, 0.78: the wave's
height over the height at which a single wave breaks in that depth. It is 0 in deep water, passes 1 where
that crest breaks and grows toward the waterline. The envelope rises and falls along a set, so the largest
crests pass 1 furthest out. A 1 m, 8 s wave breaks in 1.64 m of water at a height of 1.28 m, 9% under Komar
and Gaughan's 1.41 m, as linear theory is; in a 1 m sea of two trains the largest crests break in 2.3 m of
water and the smallest in 0.8 m, 46 m and 17 m off a beach of 1 in 20 (part O).

**The section.** The profile holds one **breaker section**: a table, over the phase `Θ` and the breaking index, of how
far the surface departs from the plain wave, along the crest's direction and upward, in units of the envelope
after the limit. It is the device Horizon Forbidden West bakes, driven here by the wave's own breaking index and not
by a drawn curve. The table is the profile's data, and it is held to these rules, which the model tests:

- at index 0 it adds nothing, so deep water is `SEA`'s field exactly;
- as the index rises the crest sharpens and the trough flattens, and then the front face steepens, the order
  field records give;
- near index 1 the crest is thrown forward. The throw at any index is a function of the section's height
  alone and rises with it, so the thrown curve is a shear of the unthrown one: it may overhang, and it cannot
  cross itself. A 1.4 m crest on a 25 m wave with a steepened face overhangs by 0.2 m when thrown 2.8 m and
  by 1.3 m when thrown 4.2 m (part O);
- past the profile's bore index the table has no throw and its crest and trough return to the plain heights.
  The shore profile below acts on the resulting sheet, lowering and broadening the front into the swash.

The first profile's table is made from a closed form, a phase skew for the face and a throw in proportion to
height, and is tuned against the inspected items of WP13.

**Bore decay and swash.** WP13's failed Metal view exposed a dry strip in 30 of 173 cross-shore cuts of the
f64 long-wave sheet. A representative cut had about 1 m of bared bed seaward of about 5.5 m of wet swash.
The summed trough can exceed the regular-wave limit because that limit bounds variance, while the component
amplitudes add coherently. A finer mesh or a change in opacity leaves the sheet's crossings in place. Use a
shore-local profile deformation, drawing on [Guerrilla's production breaker](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf).
[XBeach's bore and roller models](https://xbeach.readthedocs.io/en/latest/xbeach_manual.html#dissipation)
motivate letting the bore lose organized wave motion as it runs in. Thrum approximates that loss with a
bounded profile driven by the existing swell and reuses the existing foam stocks.
[WP13 handoff](../../../.orchestrate/evidence/WATERS/WP13-handoff.md)

- On a rising beach, prepare the shore-normal transects with the owning body's bed and sampled stages. Use
  each transect's still-water contour point at the current stage as the phase reference. Evaluate the same
  lagged, exposed, depth-limited long-wave sum `Zs` there for the requested time. Use this value across the
  transect, so phase varies along the shore and in time while one profile advances and retreats. Interpolate
  the reference through the tide on the same body's connected shore margin. Taper at the edge of eligible
  terrain or a body join, where the ordinary map and gains apply.
- Let `d` be signed still-water depth (positive seaward), `s` the row's swash depth and
  `c = 0.6 / (2√2)`. Set `q` to `cos(arg(Zs))` times `smoothstep(0, c·s, |Zs|)` so weak or sheltered
  arrivals drive less swash and a regular wave of the held height reaches `q = ±1`. Take `q = 0` when
  `Zs = 0` or `s = 0`. The inner profile's signed thickness is `d + c·max(d+s, 0)·q`. Evaluate it in
  world horizontal coordinates inside the existing wave support, with long-wave chop and throw faded out.
  For constant gains on a rising transect, `|q| ≤ 1` makes thickness strictly increasing with depth. At
  the two regular-wave extrema, its zero is `c·s/(1−c)` below the level and `c·s/(1+c)` above it, the
  existing draw-back and run-up (about `0.27·s` and `0.175·s`). With `s = 0` the swash stands still.
- Start blending back to the ordinary section seaward of the deepest allowed retreat, where the inner profile
  always stays wet. In the overlap, bound the negative excursion of the combined ambient geometry against
  the bed at the displaced position, with a smooth knee and positive wet thickness. Fade the floor seaward
  where the unthrown sheet's excursion bound already keeps it wet. As the bore loses throw, keep the
  cross-shore mapping ordered in the overlap so horizontal chop cannot put a second crossing inside the swash.
  Preserve the plunging shape farther out and fade any throw whose reach would intrude on the inner profile.
  Near the wet edge, taper resolved short-wave geometry so it cannot change the sign of the thickness or
  move the edge horizontally. Short-wave shading detail and the interaction waves' still-water support
  follow their existing rules.
- The section shape, bore decay and overlap width are tuned by eye for the WP13 crest, bore and swash views.
  The regular-wave extrema and the one wet edge on a rising transect are properties of the model and query.
  Use the plane-beach regular-wave case and the failed trade-wind cut to diagnose the handoff through advance
  and retreat. WP13 judges the look in submitted motion.

**Readers.** The transform, section and swash are one function, in `thrum-water-model` in f64 and in WGSL,
read by the field cache, the query and the foam's births and carriage. Use the final sheet for wet/dry,
depth, normals and motion, including in the overlap. On the GPU the lags are one array
texture and the exposures another. Where a crest overhangs, a column holds more than one surface: the query
answers the highest, by [a cut of the sheet](#bodies-and-the-query), and the surface is drawn as the
displaced sheet it is. Inside a chart's join the
transform is the joined body's, taken at the join's weight, so it follows from the row as the swash depth
does and the surface still meets across an end.

**Declared approximations.** Linear ray theory up to the break: no set-up of the mean level, no longshore or
rip current, no reflection from a steep shore and no diffraction, so energy neither gathers on a headland nor
leaks into a lee. The first arrival leaves a crease where two fronts meet behind an island, in water the
exposure has already calmed. The outer bore follows the incoming phase at the floored depth, then gives way
to the bounded swash profile. This profile is a shoreline displacement approximation. The blends misstate the
wavenumber by up to 4% in a metre of water or more and 9% between the floor and a metre, and the first-order
solve adds its own: 6% in a metre of water and 3% in two on a beach of 1 in 10 at 2 m cells, a third of that
on 1 in 30 (part W). An enclosed lake's waves start at its upwind shore with the plain wave's phase, so the
shelf they cross first bends them by up to about 11° in its deep water; their energy grows in proportion to
the open water upwind and not by a growth law of their own; and an island in a lake starts them again in
its lee. Nothing leaves the sheet as spray.

### Carriage by the current

Where the map has a current, everything on the surface travels with it.

- **State** is moved exactly: foam drifts with the current, and interaction waves are carried by their window's
  mean current as a phase shift for each wave vector inside the same step. On a beach foam is also carried by
  the rising sheet of [the swash](#whitewater).
- **The cascades and the foam pattern**, which are functions and have no state to move, are read at a place
  carried back along the current, by two samples half a period apart that cross-fade, so the detail moves
  downstream at the current's speed and its distortion is renewed before it shows. The period is a profile value.
- **Bodies** feel the current through the query, and the game's own drag carries them.

The long waves are not carried: a body with a current is a chart, and its long-wave gain is zero outside its
mouth's join. The swell a join lets in is not shifted by the outflow, a declared approximation.

### Time: content, functions and state

Water holds three kinds of thing, and time reaches them differently. **The map is content**: it changes only by
a body's stage, which is set with the tick and interpolated to the display's time as any game value is.
The shore transform's lags and exposures are content too, and its breaker section is a function of the wave
field that holds no state.
**The wave field is a function of time**, evaluated every presented frame at the display's time `t = C − 1 + α`
in ticks, where `C` is the frame's committed tick and `α` its interpolation alpha (C02); that is
[`SEA`'s](../SEA/DESIGN.md#time-the-field-is-evaluated-at-the-displays-time). Gains and carriage are applied as
the field is read, so they add no state.

**Interaction waves and foam are state.** The text below calls the interaction waves the wake, after their first source. Each is the integral of what bodies and breaking waves did, and advances on the
committed tick through `EFFECT-GRAPH`'s
[state table](../EFFECT-GRAPH/DESIGN.md#state-time-and-the-tentative-frame-wp2). The seam runs a producer once in
a frame, so each operator takes the number of ticks to advance as a parameter:

| Layer | A frame *n* ticks ahead | A full frame: the first of an epoch, a backward tick, a new timeline, a replaced device |
|---|---|---|
| Wake | One step of `min(n, 8)` ticks ([interaction waves](#interaction-waves)) | Reset to calm, and the scene's status says so |
| Foam | One step of `min(n, 8)` ticks, each operator written for a step of any length ([whitewater](#whitewater)) | Reset to none |

**The catch-up bound is 8 ticks**, 133 ms at a 60 Hz tick. Ticks beyond it are counted as dropped and reported:
the wake's waves and the foam's ageing are then that many ticks late, and nothing else changes. The bound is
where the wake's one-step error is still small (part A: 0.16% of the field's RMS height after one 8-tick step,
1.4% after one of 16).

**Two endpoints, for state only.** State cannot be evaluated between ticks, and a wake that stepped at 60 Hz
under a hull drawn at 120 Hz would judder against it. So the wake and the foam are each kept in two banks, the
endpoints: the state at the committed tick `C`, and the state at the tick committed before it, `P`. Each bank
carries the tick it holds, and the ticks commit with the frame. The surface draws the blend of the two at weight
`w = clamp((t − P) / (C − P), 0, 1)`.

| Frame | The banks once it is submitted | What the surface draws of the wake and foam |
|---|---|---|
| The same tick: a repeat frame, a pause | Unchanged | The blend at `w` |
| *n* ticks ahead, *n* ≥ 1 | The bank that held `P` now holds the new tick: `(C, C + n)` | The blend at `w`. For *n* = 1 the endpoints are one tick apart and `w = α`. For *n* > 1 the older endpoint is *n* ticks old and `w` runs from `(n − 1)/n` to 1 |
| A full frame | Both hold the frame's tick: the wake calm, no foam | That state, unblended, until the next tick. History is marked invalid |
| Discarded | Unchanged: nothing commits, under the seam's tentative rule | Nothing |
| Capture | Unchanged: the seam runs no producer and keeps the state, whatever the capture's tick | The committed endpoints at the capture's own `w`, under the wave field at the capture's `t`. A capture at another tick shows the committed wake and foam, not that tick's |

A blend is right for these two. Under a moving hull the wake is forced, and a blend of two dents a few
centimetres apart is the dent at the hull's interpolated place to second order; the free waves it misstates are
small. For a wave of amplitude `A` and frequency `ω`, endpoints one tick apart misstate it by at most
`A(ωΔt)²/8`: 0.5% for a 0.4 m wave at a 60 Hz tick (part G). Endpoints `s` ticks apart, in the one tick that
follows a hitch, misstate it by at most `A(ωΔt)²(s − 1)/2`, and never by more than `A(ωΔt + 2/s)`: at `s = 8`,
0.3% for a 20 m wave, 3% for a 2 m wave and 15% for a 0.4 m ripple (part B). Both are declared approximations of
the wake alone, on frames whose history is already invalid.

A history pair's bindings do not offer the older side on a frame whose producer was skipped, so the endpoints are
two persistent banks, each written by its own skippable producer. On a frame that advances, the producer of the
bank that holds `P` runs; on a full frame both run. `EFFECT-GRAPH` needs no change for this.

### The surface and the frame contract

**Drawn body by body, shaded as one.** The sea is drawn by a camera-centred mesh whose spacing doubles outward,
snapped to the world so vertices do not swim, closed to the horizon by a far skirt. Every other body is drawn by
tiles over its own bounds and shore margin, with spacing that follows distance. A vertex takes its body's row,
continued a little past its shore and past a chart's end at weight 1, so no triangle ever joins two levels. A
pixel is shaded only by the body the map names at its **rest place**, the place its surface point had before
the waves displaced it, which the vertices carry. Two bodies' meshes are therefore complementary parts of one
displaced sheet: where their tiles overlap there is neither a gap nor a doubled surface, and at a join, where
the map's rows agree, the two parts meet in height and slope. The ground's own depth cuts each shoreline.
Earth's curvature is not modelled; the sea covers everything below the level horizon that the ground does not.

**The sheet and the film.** Inside a body's wave support the surface is drawn over the shore margin as it is
over still water. Where it stands above the ground it is the swash, a sheet that runs up the sand and thins to
nothing at its edge, cut by the scene's depth as every waterline is. Where it stands below the ground, on sand
the waves wash, it is held on the ground and drawn as a **film**: it has no thickness, so the ground shows
through it unchanged; it reflects the sky by a weight that is 1 at the still waterline and falls to 0 at the
height a regular wave of the row's height reaches there; and the foam the sheet left lies on it. The
film is drawn with a depth bias so that it neither fights the ground nor floats over it, and it writes no scene
depth: under it the depth stays the ground's, so a reader of depth finds water only where the sheet stands. It
is a declared residual and not water: the query answers dry under it, nothing is born on it, and it has no
optical depth (C07, C09). How dark
wet sand is belongs to the ground's material, and so to `PUDDLES`.

A vertex is displaced from its level by the long waves (the field cache) and the cascades its spacing resolves,
at its body's gains, and by the interaction waves. A pixel takes its normal from the same layers down to its
footprint and its roughness from the slope covariance of everything smaller (Bruneton's geometry-to-BRDF
transition, through the LEAN moments).

**How the surface reads the wave field.** Every layer is kept at the rest place and drawn forward, as the seas
that ship are: Tessendorf's choppy FFT in Sea of Thieves, HDRP and Crest, whose camera-centred levels move each
vertex by the displacement found where it rests. Five rules settle it:

- *Forward, never inverted, for drawing.* Each level of the field cache holds the long waves' displacement in all
  three axes, its derivatives with respect to the rest place and its rate of change, evaluated at rest places
  snapped to that level's texels, as the cascades are. A vertex resting at `r` stands at its level plus the summed
  displacement at `r`, horizontal part included, so the rest grid crowds into each crest and spreads over each
  trough. A pixel reads every layer at its interpolated rest place and carries the summed slope and covariance
  through the inverse of the summed horizontal Jacobian. Finding which rest place lies under a world point is the
  query's work and the highest-surface cut's, and never the drawn surface's. On a fixed world grid a compressed
  crest turns over about a texel. At the 4 to 8 m spacings seen from 40 m up, a 53 m wind sea's crests are then
  drawn 8 to 28 times rounder than they are. The forward form draws them within 1.3 to 2.3 times their true width
  (part Z). The forward cache also costs one evaluation a texel, where the inversion iterated. The inverted
  cache took 3.6 ms a frame in WP2's captures, against the sea design's estimate of 0.2–0.3 ms.
- *Filtered over the pixel's footprint, not its longest side.* Each layer a pixel reads, the cascades and every
  cache level, is a filterable texture with a full mip chain. Its texels hold the LEAN moments: the mean slope,
  the slopes' second moments and the Jacobian's mean and square. The pixel reads it with trilinear anisotropic
  filtering, at the highest anisotropy the capability record admits, which is 16 on both first targets. The
  moments are linear, so the hardware's average over the footprint's ellipse gives the right mean and covariance
  at any anisotropy (Olano and Baker's LEAN mapping). Under a grazing view the footprint's long side is the
  distance over the height times its short side. Choosing detail by the long side turns every wave under 20 m
  into one mean slope and one variance about 600 m out from 40 m up, 60 pixels under the horizon, and every wave
  under 160 m by 2.4 km. That is the smooth belt of WP2's first build. Read over the ellipse, waves under 20 m stay resolved to about 1.7 km, and the
  wind sea to the rows where it is under a pixel tall (part Y).
- *Nothing is dropped.* Whatever a layer stops resolving joins the covariance, whether by its level, its mip or a
  fade with distance. A fade that removes resolved slope and adds no variance leaves the far sea smoother than
  the sea.
- *The far sea is its statistics, lit as such.* Where no texel resolves a wave, the pixel's light is the BRDF of
  the slope distribution (Bruneton, Neyret and Holzschuch 2010). That is the mean Fresnel over the visible slopes,
  the Sun's lobe, and the sky round the reflected direction at the covariance's roughness. A visible facet that
  reflects below the horizon sees another wave, not the ground the sky model holds there. So the environment's
  lobe is taken from the sky at and above the horizon. The far sea's tone then meets the resolved water below it
  and the sky above it with no step.
- *Crests read by their light as well as their shape.* The [optics](#optics)' thin-crest term is part of the
  surface from its first frame. In-scatter grows where the surface stands high and is compressed, lit only by
  the lent Sun and sky, as Sea of Thieves' wave-peak mask does from the same displacement. WP8 judges its glow
  against a low Sun.

The horizon these rules give is physical. From 40 m up, a wave is under a pixel tall within a few pixels of the
horizon, so the last rows are statistics and carry no resolved texture. WP2.2's "band" is something else: a strip
whose texture stops at an edge, or whose tone steps from the water below it or the sky above.

The water draws in `EFFECT-GRAPH`'s `after-opaque` stage, once for all its bodies, and writes the frame contract for its pixels:

- **Depth.** The water writes scene depth wherever its surface stands above the ground; the film writes none. Reconstruction's depth test and any later medium, such as a cloud march
  that must stop at the water, then see the water as the nearest surface. This needs
  [a ruling](#what-waters-asks-of-other-modules).
- **Motion.** From unjittered positions: where this surface point was at the previous presentation. For the
  ambient layers that is their rate of change at `t` times the interval since the previous presentation; for the
  wake it comes from its two endpoints.
- **Reactive.** Foam, glitter and the reflection of moving bodies write reactive coverage.
- **Reset.** A rebuild of the water, a change of profile or of bodies, a full frame, an advance of more than one tick and a foam pattern
  change mark the water's history invalid. A quality knob applies the invalidation it declares.

Where surf is in view the mesh is refined along the band where a crest's breaking index passes 1, because the section
stretches the sheet there. The section's throw is scaled down as the mesh's spacing grows against the wave's
length, so a breaker too far to resolve is a steep white wave and never a torn one.

The camera may stand on either side of the surface. [Under water](#under-water) says what it sees from below.

### Bodies and the query

**The query.** Once per tick the game posts sample points for every body: position, a footprint radius, the tick
the answer will be used at, and the body's ID and generation. One batch a tick answers them all from the water's own state, never from the field cache or the camera, and never one batch for each body or point. It is one compute dispatch, and the passes of [a cut](#bodies-and-the-query) where points lie in surf:

- the map: the body of water at the point, its level, bed, depth and current;
- long waves, evaluated analytically at each point for the tick it will be used at, at the body's gain, through
  the shore transform and the breaker section where the profile has surf;
- cascades, read from the frame's evaluation at the mip that matches the point's footprint, so a 6 m hull does
  not shake on ripples, at the body's gains;
- the wake, read from its windows, with **the body's own static dent added back** (below).

The wake's share of an answer is the height the wake's own state holds at that place and tick, which is the
height the surface draws there, with no gain between them. A float lifted by half a crest would still rise when
the crest came and lie still after it, so WP6 compares the two directly: a query's height against the wake's
endpoints read back, at points clear of every footprint, where no dent is added. No second wake solver is needed.

Crests sharpen because displacement is horizontal as well as vertical, so the surface above a world point is
found by inverting that displacement. Where no crest can be thrown onto the point, a fixed-point iteration on
the horizontal displacement does it, a contraction, and the oracle iterates to convergence.

**The highest surface.** Under a thrown crest the map from rest place to world place is many to one. It is
`F(u) = u + P(u) + T(u)·n(u)`: the plain displacement `P`, and the throw `T`, the section's departure along the
crest's direction `n`, both taken at the rest place `u`. The plain iteration then settles on one of the
surfaces in the column and not on the highest: on part O's section it stops in the trough under the lip at 52
of 501 points across a wave, up to 0.92 m low, with no residual to show for it (part V). Nor does a search
along the crest's direction at the point find it. In a sea of more than one train the envelope and the
direction change from place to place, so a surface arrives from beside that line, thrown further than the
envelope at the point allows: on two crossing trains such a search is wrong at 33 of 676 points, by up to
0.62 m (part X). So the surface over a point is found the way it is drawn, forward, by a **cut of the sheet**
within the point's **throw reach**.

- **The throw reach** is how far away a rest place can be and still stand over the point. It is a ceiling
  that needs no wave evaluated. The **amplitude ceiling** at a depth is the sum of the tick's long-wave
  components, each at the body's gain and at its shoaling gain in that depth, with every exposure taken as 1;
  no envelope in that depth exceeds it. The **throw ceiling** at a depth is the table's largest throw over
  every envelope up to the amplitude ceiling, each at the breaking index it has in that depth. It is zero in
  water too deep for any crest of the tick to be thrown. The **displacement ceiling** is the largest plain
  horizontal displacement those amplitudes give. A point's throw reach is the largest throw ceiling and the
  largest displacement ceiling, added, over the depths the bed has within the reach itself, at the body's
  stage and floored as the lag's is. It is found from coarse to fine: the body's whole range of depths gives
  a first reach, and the bed's lowest and highest height within that
  reach, read from a min-max pyramid of the bed that is prepared with the lags, give the next, twice in the
  first profile. Every round's reach holds every rest place that can stand over the point, because such a
  place lies inside the round before and so in a depth that round took its ceilings over. On part X's sheets
  no thrown rest place is further from its own world place than 0.92 of the reach there.
- **The cut.** The sheet is evaluated forward at the nodes of a square lattice of rest places, fixed in the
  world, of the profile's **cut cell**, 0.25 m in the first profile. Every cell within the point's throw reach
  is two triangles of the displaced sheet, and a triangle whose plan holds the point is a surface in the
  column, at the height interpolated across it. The answer is the highest. Nothing in it follows a direction
  or inverts a map: however far and whichever way a rest place was thrown, its triangle lies where the
  surface is drawn.
- **Coarse to fine.** The reach is a ceiling over every sea the tick's components could make, and in a gale
  it is many times further than any crest is thrown. On the island's gale the reach is 87.5 m; the highest
  surface's rest place is 0.6 m from its column at the median and 16.6 m at the furthest
  ([part AA](../../../.orchestrate/evidence/WATERS/WP19/query-reach-study.md)). So the cut is made in two
  passes, and only the first covers the reach. The first evaluates the sheet at the nodes of the **coarse
  lattice**, fixed in the world as the cut's is, whose **coarse cell** is a whole number of cut cells, 2 m in
  the first profile. A coarse cell is kept for the point when the box round its four displaced corners, grown
  by the cell's **landing margin**, holds the point. The margin is the cell's own. It is the profile's
  **margin floor**, 0.5 m in the first profile, added to the profile's **bend factor**, 1 in the first
  profile, times the cell's **corner spread**: the largest difference in displacement between two of its four
  corners. Where a crest is thrown, the corners of a cell are displaced by different amounts and the sheet
  between them bows out of their box, so the margin grows where the bow does; in open water the spread is
  centimetres and the margin is the floor. The second pass makes the cut above, at the cut cell, inside the
  kept cells alone, and the answer is the highest triangle it finds. The points of one body share one coarse
  pass over the union of their reaches, and a node that two of them need is evaluated once.
- **Refinement.** The highest triangle gives a rest place by the same interpolation. Newton's method on the
  assembled map moves it until it maps to the point within the profile's column tolerance, 1 mm in the first
  profile, and the answer's height, slope and velocity are the surface's there. So a point that passes into
  a throw reach sees no step beyond that tolerance, and a float does not feel the lattice. A hit that does not converge inside its
  cell, as the map's Jacobian allows at the very edge of a fold, keeps its interpolated height. In part X an
  interpolated place maps up to 0.14 m from its column, and all 16,526 refined answers land within 1 mm.

Where no depth within the reach has a throw ceiling above zero, which is everywhere but near water shallow
enough for a crest of the tick to be thrown, there is no cut and the answer is the plain iteration. The model
in f64 and the dispatch on the GPU make the same two passes on the same lattices, with the same margin rule.

What a cut can miss is a fold thinner than a cell: the edge of a lip. Its answer is then the highest surface
at a point no more than one cell away, and never the water under a lip longer than that. Part X holds it to a
cut of a lattice eight times finer, made with no reach in it, on three sheets: part V's straight section; the
design review's two trains crossing at a right angle, 0.47 m and 0.23 m; and a beach of 1 in 20 where three
trains of a 3.2 m sea shoal and turn and the section follows the breaking index. At every one of 82,619
columns, 1,062 of which hold three surfaces or more, and at cells of 0.5, 0.25 and 0.125 m, the answer lies
between the least and the greatest height the fine cut gives within one cell of the point. Of 1,200 rest
places of known height over their own world place, the 0.25 m cut answers below 28, and each of those stands
within 46 mm of a lip's edge. The review's own witness is found, to the micrometre. WP12.12 holds the model to
such a cut on such sheets, written in the test, so the GPU is compared with a model whose branch is
established. The lens test of [the view from under water](#under-water) reads the same cut, made once for the
lens: its near plane spans a cell or two, so one set of nodes serves every pixel.

What the coarse pass can miss is a part of the sheet inside one coarse cell that lands further outside the
box of that cell's corners than the cell's margin: a bend its four corners do not show. The answer is then
the highest of the surfaces the kept cells hold. A margin of one size does not hold the sheet. The design
review's column in the island's gale, at 2.0 s and world (−142.177, 8.880) m, holds three surfaces with the
highest at +1.103 m, and 2 m cells with a fixed margin of 0.5 m answer −0.806 m, the water under the lip.
[Part AB](../../../.orchestrate/evidence/WATERS/WP19/coarse-rule-study.md) measured the whole sheet: in
294,400 coarse cells over that gale's breakers, at 40 times, the sheet lies up to 1.03 m outside a 2 m cell's
corner box and further than 0.5 m in 110 cells, and it does so where the corners are spread. With the bend
factor at 1 no cell needs a floor above 0.08 m, so a cell's margin is at least 1.7 times what it needed. At
51,658 columns of ten of those times, 47,657 of them under a fold, where the throw has turned a triangle of
the cut over, the rule lost no surface and answers the
review's column at +1.103 m, where the fixed 0.5 m lost the highest surface in 91 columns, by up to 4.1 m.
At the same factor a 4 m cell needs a floor of 0.31 m and a 1 m cell none. The rule is measured on one bed
and one sea and is not a bound, so WP19.1 holds the two passes to the whole cut, the cut-cell lattice over
the whole reach, on the three production sheets, on a sheet of many trains, and on the island's gale at the
review's column and under its folds. The coarse pass is tested against the cut it replaces.

**What the cut costs.** A node is one evaluation of the long waves and the section, where a plain point costs
a few. The whole cut, the cut-cell lattice over the whole reach, is about 1,200 nodes for a 1.4 m wave thrown
4.2 m and 3,500 to 5,500 under the 3.2 m sea of part X. In the island's gale it is 495,000 to 990,000 nodes a
point. WP13's record measured 47 million nodes and 18.9 s of GPU time for one tick of eight floats in the
breakers, against 0.2 ms with surf off
([the paired record](../../../.orchestrate/evidence/WATERS/WP13/surf-cost.json)), and no game can wait for
that. So the two passes are the mechanism and not an option. On the same gale the coarse pass is about 7,800
nodes over the reach for a body. The fine pass is about 480 nodes for a point in the breakers at large, about
2,300 for a point under a fold and 5,508 at most: about 86,000 nodes a tick for the eight floats, and about
174,000 with every station under a lip (part AB). What the query must meet is its age contract, below, with
the floats in the breakers and the game running, and WP19 shows it.

The rest is the builder's, held to the model by WP13.1 and WP19.2. The nodes are independent, so the dispatch
may spread them over the GPU as the field cache does for the same sheet. A coarse cell may be skipped when
the ceilings at its own depth, taken after the depth limit, cannot carry it to the point, since those are
still ceilings. No long wave is shorter than 7.7 m, a 20 m wave over the lag's floor, so the component sums
may be taken on a coarser lattice and interpolated. The cut cell, the coarse cell, the margin floor and the
bend factor are the profile's knobs: a larger cell of either kind, or a smaller floor or factor, widens what
a cut can miss. WP12 records
what the model's cut costs, WP13 a tick with every float in the breakers, and WP19 the same tick again.

Each answer is the body of water at the point with its depth and current, the height, the slope, the surface's
velocity (the current and the waves' together) and the time derivative of height, with one support flag per
layer, and it is **valid at the use tick** its point asked for. A point that is not wet now says it is dry: ground beyond the water, and the swash between waves, where the depth it reports is negative. The dispatch brings every layer to that tick
before it sums them: the long waves are evaluated there, the height of the cascades is carried there by its
rate of change from the display time `t` of the last submitted view, and the wake's from its state tick. The
answer carries the use tick and the times it was made from. A game that uses an answer at another tick moves
the whole answer once, by its time derivative from the use tick, so no layer is extrapolated twice. For a
wave of frequency `ω` carried `Δt` the remaining error is at most `½(ωΔt)²` of its amplitude, reported per
profile.

**The carry bound** is how far a layer read from frame state may be carried, and it is the profile's: **five
ticks** in the first profile. An ordinary post needs four of them. A float at tick `C` asks for tick `C + 2`,
and its batch goes ahead of tick `C`'s view (below), so the newest cascades are the view before's. That view
committed tick `C − 1`, and its display time is `t = C − 2 + α`: a carry of `4 − α` ticks, more than three
and at most four, on every tick of a world that draws each tick. The wake's state is at tick `C − 1`, a
carry of three. The fifth tick is for an advance that completes two ticks, or one that follows a single view
the host did not submit, so that neither costs a float its short waves. At five ticks the error above is
1.1% for a 20 m wave and 11% for a 2 m wave, whose height is centimetres (part G's formula; three ticks,
the bound while the batch rode the frame, gave 0.4% and 3.8%).

**Admitted by the tick.** The batch is a [tick pass](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6) and not a
pass of the frame. The frame host asks for it in every advance that reaches the device, records it and submits
it by itself, ahead of that advance's view and whether or not a view is drawn, and the readback's map starts
when that submission is made. So a batch posted in a tick whose view is skipped, for want of a frame slot or
a drawable, is on the device in that tick. The one exception is the stage's own: at the host's bound on
outstanding tick submissions the host records no batch, counts the refusal and asks again at its next
advance ([rule 5](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6)), and the posts wait in the mailbox as they
do for any batch not yet made. As a `simulate` pass it waited for the next view the host
admitted: WP19's first running record delivered 69% of open-water answers within two ticks and none within
one ([record](../../../.orchestrate/evidence/WATERS/WP19/surf-running.json)). The water still submits
nothing itself (C06).

What the batch reads is the same in kind and may be older. The long waves, the shore transform and the cut
are evaluated at the use tick from inputs the batch stages itself, so they do not age. The cascades, the
wake's windows and the map's products on the GPU are as the last submitted frame left them, and the water
knows the time and tick each holds, because it committed them. Each is brought to the use tick from there by
the rule above, and the answer carries those times as before. Skipped views lengthen the carry. A layer
whose carry would pass the carry bound is carried to the bound and no further, and its support flag is
cleared, so a game can tell: in the first profile that is a source more than five ticks behind the use
tick, two views or more gone unsubmitted at a 60 Hz draw. Inside the bound the flag stays set, so an
ordinary answer has supported cascades at its use tick. An answer is never made from what a frame recorded and did not
submit: the batch stages its own inputs and takes the frame's state only from what the frame's passes wrote.
Where the host may not touch the device, after a loss or in the background, no batch is submitted, answers
age out and the game falls back, as below.

**Whose state.** A post carries the world endpoint of its tick, which is the world epoch, the timeline and
the tick, and the game has it from its world where it posts
([rule 3](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6)). Every answer carries its post's endpoint back.
The water's committed state has an endpoint too, the one its last submitted view committed, and the seam's
state record compares the two at each asking. Ticks are not compared, so a restore that lands on the tick
number the old timeline had reached is still another world. A post is answered only when the state exists
and its world epoch and timeline are the post's. Otherwise the water makes **no answer**: no batch for that
post, and nothing marked as made without state. That is so before a world's first submitted water view, and
after a restore, a new world or a replaced device until the first view of the changed world is submitted.
A marked answer was the other choice, and it would hold nothing a game lacks. Every layer but the long
waves is frame state, the long waves are shaped by the map's products on the device, which are frame state
as well and which a restore can change with a body's stage, and the map and the long waves are what the
fallback already gives from the game's own map.

- **What the float gets.** Until an answer of its own timeline reaches it, a float gets the fallback below,
  the conformance model's map and long waves, counted as it is when an answer ages out. The first view of a
  changed world is a [full frame](#time-content-functions-and-state): it resets the wake to calm and the foam
  to none and commits the state on the new endpoint. The asking of the next advance answers, and the
  answer arrives one to two ticks later as any does.
- **A post asked before that view** is not dropped. It waits in the mailbox as a post does for any batch
  not yet made, and the first asking after its world's view answers it with the rest, carried as any late
  question is. If it passes the maximum age first it expires and is counted.
- **The former timeline.** The drain is the gate. A game drains on the endpoint of the tick it is
  advancing, which it has from its world as it has for a post, and a drain hands the game only answers
  whose world epoch and timeline are that endpoint's, which is C02's check made where the game takes the
  result. The drain does not wait for a post to tell it the timeline changed. The island drains,
  integrates its floats from what it drained and only then posts, so the first drain of a restored
  timeline comes before that timeline's first post. A result of the former timeline published a tick
  earlier is young, and its body ID and generation can still be live. That drain hands it to no one, and
  the floats take the fallback on that tick. Age is the wrong test here: the result of a timeline
  abandoned this tick is zero or one tick old and is another world's water. The mailbox serves one world
  epoch and timeline, the one it was last drained or posted on. A drain or a post on another makes it
  drop what it holds of the former: posts no batch has taken, and results published and not yet drained.
  A batch of the former still in flight is dropped when its map completes. None of them reaches the game
  at any later drain.

Results come back through a ring of mapped buffers one to two ticks later. A result older than the declared
maximum is unavailable, and the game falls back to the conformance model in `thrum-water-model` (the map, and the long waves at the body's gain:
on a lake, level water with its current)
and counts it. A hitch ages answers the same way: at a drain that brings a body no newer result, the mailbox
hands the game the last one again, carried to that tick, until it passes the maximum age
([the held answer](#a-surf-view-a-running-game-can-afford-wp21)). No path waits on the GPU: a blocking map or poll is a defect,
and a counter proves there is none. The contract is the same where the dispatch makes cuts: with every float
of the example in the breakers of a gale, answers still reach the game one to two ticks later and none grows
older than the declared maximum. The maximum age is what a hitch may cost and is not the delivery. A query
whose answers in surf arrive later than two ticks, tick after tick, has failed the contract whether they age
out or stay inside the maximum, and so has one that holds the frame while it works, whatever its answers
([risk 32](#risks)). WP19.3 reads one to two ticks as nine answers in ten, in the breakers and in open water
beside them, and leaves the tenth to hitches. The proof is the example's eight floats, 48 stations; 2,048
points under thrown crests at once is the capacity and is not claimed.

Inputs travel with the tick page: the game puts its body records, impulses, stages and wind in its page, and the scene hands them to the water, the query's points to its tick pass and the rest in `upload`. Results return through a mailbox the game drains at the start of a tick.
Query capacity comes from the profile and the capability record. A request over capacity is refused whole, by
body in ID order, counted and reported.

**Where bodies are integrated.** In the game. The engine owns the query and the wake source; the example scene
owns hydrostatics, drag, thrust and the rigid-body step, as game code, until Thrum has a physics owner.

### Interaction waves

Interaction waves are what bodies and impacts raise on any water: a hull's wake, the rings from a dropped stone,
the lap round a float. The sea design called them the wake, after their first source, and the text below keeps
the word. They are that design's operator with five things added, so that it holds on water that has a bed, a
shore, land in it and a current. They live on still water; the swash is the ambient waves' alone.

**The update at depth.** On water of uniform depth `d`, a wave vector `k` advances by

`h' = cos(ωΔt) h + (D/ω) sin(ωΔt) φ`, `φ' = cos(ωΔt) φ − (g/ω) sin(ωΔt) h`, with `D = k tanh(kd)` and `ω = √(gD)`.

`D` is the depth's own coefficient as well as its frequency, and the pair is a rotation: its determinant is 1 for
any `Δt`. Deep water is `D = k`, which is eWave's equations 19 and 20. Putting a finite-depth `ω` into those
equations and leaving their `k/ω` is not a rotation, and grows a 2 m wave on 0.25 m of water by 9% a second
(part H). At `k = 0`, `h' = h` and `φ' = φ − gΔt h`.

**The depth ladder.** Depth varies across a window, so each level has a ladder of depths a factor of two apart,
which ends in the deep operator `D = k`: for the near level 0.25, 0.5, 1, 2 and 4 m and deep, and for the far
level 1, 2, 4, 8, 16, 32 and 64 m and deep. The deep operator serves all deeper water: 8 m is deep for the near
level's waves and 128 m for the far level's. A step transforms the state once and transforms back once for each
ladder depth that brackets a depth present, and each cell takes the blend of the two outputs that bracket its
own depth, with weights linear in the square root of depth. Below a ladder's first depth a cell blends that
output with the **rest operator** (`h' = h`, `φ' = φ − gΔt h`), linear in depth; it needs no transform, and it
slows a wave to a stop at the waterline. No depth falls outside a ladder: a window that holds 0.125 m shallows
and 10 m of water uses the rest operator, the first depth and the deep operator on the near level, and the
rest operator and the depths from 1 m to 16 m on the far level. A window over deep water has one operator, and its step is eWave's.

The blend is the selected operator, and part H measures it as such:

| Measured | Result |
|---|---|
| Each wave vector on uniform depth, from a level's first depth to deep, over the level's band, steps of 1 and 8 ticks | Phase speed within 2.1%, group speed within 4.4%; never grows; loses nothing a second at 1 tick and under 0.2% a second if every step were 8 |
| Far level between 0.25 m and its first depth, 1 m | Phase speed within 4.2%, group speed within 12%, worst for its shortest wave, 12 m |
| Height and potential fields, a 6 m packet up a beach of 1 in 10 from 6 m of water | The packet stays within 0.7 m of the true group speed's ray over 60 m, down to 1.3 m of water; the sum of squared height rises 8% as it shoals; 2.4% of its energy comes back from the shore |
| Fields, a far window holding 0.125 m shallows and 10 m of water; 30, 60 and 100 m packets across the 10 m part | Within 3.9% of the true group speed |
| Fields on rough beds with dry patches, 120 s, undamped, steps of 1 and 8 ticks | The sum of squared height never passes its start |
| The beach run in steps of 8 ticks against steps of 1 | 0.3% of the gauge's RMS height |

Below a level's first depth the speed is not claimed. There a wave is damped at a rate that rises to the
waterline, which is how it dies at a shore. The ladders and the weights are the physical profile's.

**Dry ground and regions.** A transform is a convolution over the whole window, so it carries a wave across
land, and setting the land's cells to rest afterwards does not take back what it wrote beyond them: a 2 m bar
in 4 m of water passes 3.2% of a pulse's peak, where open water passes 24% (the design review's case; part J).
So each **region** of a window, its still-water cells joined through still-water cells of that window, is solved
alone: its cells are transformed with every other cell at rest, and only its cells are kept. Between two regions
nothing passes, exactly (part J: 0). Regions are labelled from the map on the CPU when a window moves or a stage
changes; the map is content, so nothing is read back. A cell that is not still water holds no wave.

A **solve** is one region at one ladder depth. A level's step makes at most the profile's count of solves, 16 in
the first profile. Regions are admitted in order: the one that holds the named body, then the others by their
latest source. A region that does not fit is at rest, and its sources are refused, counted and reported.

Inside one region thin land shadows waves but does not stop the transform, a declared approximation. Part J
measures its size in one set of cases: a 1D section, one Gaussian pulse of 0.5 m scale raised 4 m from the
land, and the largest height beyond the land in the next 10 s. There, land with 4 m of water at its foot passes 3.2% of
the pulse's peak at 2 m wide, 1.4% at 4 m and 0.03% at 8 m; with 1 m at its foot, 0.3%; and with a beach of
1 in 4 or gentler on each side, under 0.2%, because shallow water's operators reach only as far as it is
deep. These are examples of the leak's size at a spit, a pier or a rock, and not a bound for every obstacle,
source and length of time. Two waters that land separates share nothing, exactly. That a wall reflects is not
promised.

**Current.** Each window takes the mean current of its wet cells, and its waves are carried by it exactly, as a
phase shift for each wave vector in the same step. The difference between a cell's current and the window's mean
is not carried, a declared approximation that `RIVERS` judges on running water.

**Impulses.** Beside the bodies' reaction forces, a game posts impulse records: a place, a vertical impulse, a
radius, a tick and a stable ID. Each is added as pressure over its footprint for one step. A dropped stone is
one; `RIVERS`' plunge and `PUDDLES`' drops are the same record. Records over the profile's capacity are refused
in ID order, counted and reported.

**State.** Two levels of 256², with cells of 0.25 m (a 64 m window) and 2 m (512 m), each holding `h` and `φ`.
The far level owns wavelengths above about 12 m and the near level the rest, through complementary filters on
the source. A level never holds a wave from the other's band.

**A step of *n* ticks.** For each region: transform, add half the step's impulse, apply the update for `nΔt` at
each ladder depth, add the other half, transform back and blend, and damp a border one eighth of the window wide
by its per-tick factor to the power *n*. The border absorbs what reaches the edge, so nothing wraps. The impulse is the body's pressure summed along
its path over the step, placed in Fourier space at exact positions, so a wake neither stair-steps with the cells
nor bunches where a hitch ended.

Free propagation on uniform depth is exact for any step, and the table above bounds the blend's. Two additions are approximations that grow with the step: the impulse split
around the rotation is second order in the step, and the border treats a wave that crosses it during the step as
if it had been there throughout. Part A measures them on a hull at 6 m/s against the same run stepped tick by
tick: one 8-tick step differs by 0.16% of the field's RMS height, a hitch of 8 ticks every second by 0.6%, and the
same with the pressure held at the body's last place by 3.4%, which is why the impulse follows the path. At 16
ticks the figures are 1.4% and 4.9%. Hence the catch-up bound of 8.

**Windows follow the craft, not the camera.** Each level is world-anchored and shifts by whole cells, with the
craft held ahead of centre so most of the window lies behind it. Two players looking in different directions
would get the same forces. The finished scope has one window set, around the body the game names, on whatever water it is.

**The source is the body's own reaction.** Each tick a body record gives its pose, velocity, a small map of its
wetted footprint in hull coordinates and its bounding box (both registered once), and the vertical force the
water put on it in the game's last step. The engine spreads that force over the footprint as pressure. The
amplitude is therefore the body's weight at rest and its load in motion: a physical input, with no gain to tune,
the same at any grid size (C09's reaction, returned once). Body records over the profile's source capacity are
refused in ID order, counted and reported.

**The static dent.** A pressure patch at rest settles to a dent of depth `p/ρg` under the hull: the hull's own
displacement. A body that read that dent as the water level would sink into its own hole. So the query adds
`p/ρg` of the asking body's own patch back at its points. At rest the body then reads the undisturbed level, and
in motion it still feels its own bow and stern waves, and every other body's wake unchanged.

**What the reach means.** Waves shorter than about 12 m exist only in the near window, some 30 m behind the
craft; longer ones reach about 250 m. Beyond that they fade through the border. This is the water decision's
sizing, stated as a limit ([plan](PLAN.md#open-for-the-human)).

### Whitewater

Foam is transported material, never a function of the present wave shape. It is held as four stocks, because a
viewer sees four things: white water, the lace it ages into, pale water where air is still under the surface,
and the smooth ribbon a hull leaves. The four are channels of one texture, so they cost what one would. Broadsides
arrived at the same four, and its spike passed the two rules marked below.

**Stocks**, per square metre, on two camera-following wrapping levels (0.25 m and 2 m cells): submerged air,
fresh film, aged film, and slick. Fresh and aged film are shares of the surface, and their sum is the cover,
between 0 and 1. Slick damps the short-wave roughness under it.

**Transfers and sinks.** Material moves between stocks only along one chain, each link on its own lifetime:

| From | To | Rule |
|---|---|---|
| Submerged air | Fresh film | Air that reaches the surface whitens it once, at the water type's yield, bounded as a birth is. Air that surfaces under full cover is lost |
| Fresh film | Aged film | Rupture: everything fresh film loses, aged film gains |
| Aged film | Nothing | The chain's sink |
| Slick | Nothing | Slow decay. Slick is fed only by births |

So a recipient rises with no cause running: aged film grows out of fresh film, and fresh film out of submerged
air, before either falls (part D: from fresh film alone, with lifetimes of 6 s and 20 s, aged film reaches 0.53
after 6 s). What cannot rise without a birth is the inventory, fresh plus aged film plus the yield times the air.
With the source off every stock ends at nothing. The chain is linear between births, so a step of any number of
ticks has a closed form, and one long step equals many short ones to rounding.

A step of *n* ticks is the elapsed time, once. Transport keeps what it moves, and that is all that is kept:
the sinks, the transfers and the births still act for the whole `nΔt`, one time. A patch of aged film that
drifts through a hitch arrives holding what those ticks stepped one at a time would leave it, less than it
set out with, its ageing neither frozen nor applied twice. WP7 tests a hitch that way.

**Births come only from booked causes**, as bounded rates `dc/dt = r(1 − c)` integrated exactly over the tick:

- *breaking waves*: where the Jacobian of the summed displacement falls below an onset. The onset is biased short
  of the Stokes limit (the spike found that the physical limit almost never fires on a height field), and the
  rate follows the swept-front rule, so one passage of a breaking crest leaves cover behind it;
- *wake breaking*: the same criterion on the wake's displacement;
- *hull contact*: along the footprint's edge, from the body's speed relative to the water, its current
  included, so a post standing in a current foams as a hull moving through still water does;
- *the stern ribbon*: behind the footprint, from the power the body lost to the water, which the game's own drag
  supplies in the body record;
- *the shallows and the swash*: where the depth holds a wave layer's height ([gains](#waves-on-a-body-gains)),
  at a rate in proportion to the power the limit takes, and only where the point is wet now. So foam forms
  through the band where waves die, up to the top of the swash. There is no threshold: a lake's small waves
  lose little power and make little foam, in that proportion, and a calm makes none.

**Shore births with surf.** Book each layer's loss across the depth limit on wet support as in WP7.5, using
the transformed long-wave power as the long layer's input to the limit. The short layers keep their
distributed births. Long-wave loss also feeds distributed births at its wet source cells wherever that wave
has no resolved wet breaking face nearby. This includes swash where the section has faded and waves that
stay at or below breaking index 1. Even the builder's 40 m wave of amplitude 0.12 m loses power in 0.02 m
of water, where its crest wets the bed and its breaking index is about 0.62. It can leave a light wash of
foam that rides up the sand and strands as lace on the retreat.

Where the same wave has a resolved wet front face above index 1, use the section's phase and shape to move
a share of the long-wave birth to that face as fresh film, with a share as submerged air in the wet column
under an actual thrown lip. A bore with a wet front and no lip takes only the face share. Blend the moved
share in from zero at index 1 and back to zero as the bore decays into the swash. The final sheet supplies
wet support and the advancing swash carries the stocks the bore leaves there. The birth budget still
comes from wet lost power. Use a bounded local footprint on the existing foam grid and weights that sum
to one including the share retained at the source. If the face or lip
is unresolved or dry, keep its share at the wet source. Reuse the surf samples and the existing foam step.
Each wave keeps the area-integrated birth budget WP7.5 assigns to its wet lost power before stock saturation
and ageing. Calm, omitted waves and dry columns contribute zero shore birth. The face width and air split
are visual tuning for WP13.7.

`RIVERS` books two more through the same rule: the height a chart's flow loses, and a fall's plunge.

A cause that loses no power makes no foam. That is what keeps a calm sea and a hull at rest clean.

**Transport.** Stocks move with the surface drift, which is the map's current, the wind's and, on a beach, the
rising sheet's (below). They gather where the drift converges, up to full cover, past which film is lost as
surfacing air is, and spread down their own gradient with a diffusivity that dies with the turbulence that made
them. This is the dispersal the bar gets from a feedback blur, as a physical term. Drift is followed along the
flow for the whole step and spreading is a kernel as wide as the step makes it, so a step of up to the catch-up
bound stays stable. The step keeps what it moves: what leaves one place arrives at another, so foam pushed to
the edge of the water is not lost there. Foam on ground the water has left stays where it is and ages out; on
sand the waves wash, [the film](#the-surface-and-the-frame-contract) shows it as lace until the sheet returns.
Beyond the far level the surface uses the erf coverage estimator from the cascades' Jacobian moments, so
whitecaps continue to the vista without state. A distant beach takes the cover that the same wet lost-power
birth would hold in steady state, including long-wave loss below breaking index 1. The estimator uses the
distributed source and hands over across the window blend, so a distant beach keeps its surf line.

**WP13 surf inspection.** Use the island's 1 km offshore view to judge whether the exposed beach still shows
a line of surf entirely beyond the foam windows. For the handoff, choose a separate fixed camera position
and aim it so the same active breaking front is visible both where simulated foam contributes in the coarse
window's outer blend and beyond the outer edge, where the distant estimator supplies the foam. Record where
the grid edge falls relative to the front so the reviewer can look for a seam or a sudden change in cover
at that location through the sequence. Match both views and their trade-wind settings on Chrome and portable
Metal.

**The swash carries foam.** Between the line the waterline draws back to and the line it runs up to, the water
is a sheet whose thickness, the surface less the ground, rises and falls at each place with each wave. While
it rises at a place, a line of equal thickness travels over the sand toward thinner water, at the rate of rise
divided by the slope of the thickness. At the sheet's edge that is the waterline's own velocity. The foam's
drift gains that velocity wherever the point is wet now and the sheet is rising: never more than
`√(g · swash depth)`, at full weight on the shore margin, and fading to nothing at the depth the waterline
draws back to under a regular wave of the held height. While the sheet falls the drift gains nothing: the water
drains from under the foam and leaves it on the sand. So each uprush pushes the foam in its path up the beach,
gathered toward the sheet's edge, and leaves it at the top of the swash and on the sand behind, where it ages
out. The carriage is a function of the surface model alone, its height's rate and slope and the bed's, so
`thrum-water-model` has it, and it needs no state of its own.

With no wind and no current, parcels laid between the draw-back line and the still waterline end within 0.1 m
of the run-up line in three waves, on beaches of 1 in 4 and 1 in 10 and in seas of 0.16 to 2.1 m; on a beach
of 1 in 30, where the bound holds the drift, they are 92% of the way there. Under two wave trains a parcel
ends 1 cm from the waterline's highest reach. On 0.25 m foam cells a patch laid at the still waterline
moves 3 m up a beach of 1 in 10 in three waves and keeps all its foam, in steps of 1 tick and of 8, while its cover stays under the cap; where carriage piles foam past full cover the cap is a sink, as it is everywhere. With the
drift of the current and the wind alone it does not move (part M). Declared approximations: the carriage runs
across the lines of equal thickness, so waves that arrive at an angle carry no foam along the shore; the
backwash carries none back; on a step of several ticks the sheet's drift is taken once, at the step's middle;
on the far level, whose 2 m cells are as wide as a swash, the carriage is not resolved and foam lies where it
was born; and seaward of the draw-back line nothing carries foam shoreward. A bore's white front travels
because foam is born along it, and what it leaves behind stays until the swash takes it.

**Appearance.** Coverage thresholds a ranked pattern, so thin foam is lace and thick foam closes. The pattern is
sampled by stochastic patches, the change that removed the stamped repeat. The pattern is a cooked 2D product
the game supplies through a `CONTENT-LOOP` lease, following `examples/v1-game/src/lent`. The water draws the
shore film in its own pass.

**How much of the spike's open look work is inside S4.** Judged against the bar: caps with a bright core and a
feathered edge are inside, because flat white cutouts do not read as whitecaps. Pale water over submerged air
under and behind foam is inside, because it is most of how the bar's wake reads. A trail that widens and thins is
inside, as the dispersion term. Wind streaks are outside. The parked spikes stay parked.

### Optics

The water composes C08's ordered segments itself, in one pass, in physical units, the same on every body:
`L = T_air · (R · L_reflected + (1 − R) · (S_water + T_water · L_behind)) + S_air`.

- **Air.** `ATMOSPHERE`'s aerial `(T, S)` from the camera to the surface point, applied once.
- **Reflection.** A provider slot. The environment product is always present: the filtered sky around the
  reflected direction at the roughness the slope covariance gives. Where a body of water has
  [a mirror view](#mirror-views-scenery-the-camera-cannot-see), a reflected ray reads that and then the
  environment. Where it has none, the ray is traced on screen: against registered bodies first where it finds
  one, then against the copied scene where the water is still enough to show it. Ray query is a later
  provider, for devices that have it, and is not built.
- **The Sun.** The true Sun through the air, times the cloud factor once, into a microfacet lobe whose width is
  the slope covariance. Glitter, and its path at low Sun, come from the sea state and not from a texture.
- **Water.** In-scatter from the water type's absorption and backscatter (a game table, one row a
  body of water), lit by the same Sun and
  sky. A thin crest seen against the Sun passes light through. The bed, and a body under water, are seen through
  `T_water = exp(−c·d)` per channel along the refracted path, so it tints and fades with depth.
- **Exposure.** The lent scale multiplies `L` once, as the water writes its colour, as the forward pass does.

**What lies behind and beside the water: one copy, and where it is read.** An open sea is optically thick and
empty: `L_behind` is zero except where a body is under the surface, and a reflected ray meets something other than
sky only at a body. A lake is neither. In its shallows the bed is what a viewer sees, and on a still day the far
shore is its reflection. So the water copies the scene only where a reader needs it, and reads it three ways.

*Thickness.* Where the copy holds the scene behind a water pixel, the water's thickness along the refracted view
path is measured to the copied depth. Colour then deepens with the real depth, the bed fades out where the water
type hides it, and the water thins to nothing at the waterline, which is the soft edge. Elsewhere the thickness
is the map's depth, and where there is no bed it is unbounded.

*What is copied.* Three readers, each with screen rectangles. The bodies a scene registers, each with a world box,
which the craft and floats carry in their body records. See-through water: the map's tiles whose depth is under
the water type's sight depth, where `T_water` falls below 1%. And the screen: while water still enough to mirror
is in view and has no mirror view, for the scene's reflection, and while any pixel of the lens is
[under water](#under-water). On an open sea with no body on screen and the camera above it that is nothing, which keeps the sea
design's economy on a tile GPU. Where shallow or mirror-still water fills the view it is the screen, and the copy
is then `EFFECT-GRAPH`'s refraction copy in all but name. Opaque geometry outside every rectangle is neither
reflected nor seen through the water, and takes the environment.


*The source.* Before the surface pass, a gather pass copies opaque colour and linear depth inside the union of
those rectangles. A registered body's **source rectangle** is the screen bounds of its box, padded by the
filter's reach. Nothing else is copied, and with no reader on screen the pass does not run. Scene depth cannot be sampled in the pass that writes it, which
is the second reason the gather is its own pass.

*The receiving region.* The sea pixels that can show a body are not the body's own rectangle: a reflection lies
below the waterline, outside the silhouette. Two world volumes bound them, with the body's level at height 0,
`a` the largest surface height the sea state gives this tick, `θ` the slope allowance and `ℓ` the reach (below):

- *reflection:* the body's box, extended down to `−(top + 2a + 2ℓ sin θ)` and sideways by
  `2(top + a + ℓ sin θ) sin θ`. A pixel shows a reflection exactly when its view ray, continued through the
  surface, passes through the body's mirror image in that pixel's tangent plane, and this volume holds every such
  image for a surface point within `a` of the mean level and a normal within `θ` of vertical;
- *refraction:* the slab of surface heights over the box, grown sideways by `d · tan(48.8° + θ)`, where `d` is
  how far the box reaches below the highest surface and 48.8° is the largest angle a ray can take into water.

The screen bounds of both volumes are the body's **receiving rectangle**; a volume that reaches the camera's
plane takes the whole screen. The rectangles are computed on the CPU each frame and mark a coarse screen mask of
which bodies each part of the screen may show. On a calm sea a receiving rectangle is the body and its mirror
image, two to three times the body's own; in a gale it can be most of the screen. What stays bounded there is the
work in it, not its area.

*The mapping.* Where the mask marks a body, a sea pixel tests its reflected ray and its refracted ray against
that body's box. A ray that misses the box does no more. A ray that meets it is marched across that segment
only, in screen space against the gathered depth, for a bounded number of steps. A hit reads the gathered colour
at the hit's own texel, which lies inside the source rectangle because the hit is on the body. A miss, a hit off
the screen, and a sample whose depth is not inside that body's box take the environment for a reflected ray and
the water alone for a refracted one.

*The bounded approximation.* A reflected ray is followed for at most the reach, and a pixel whose normal tilts
past the slope allowance takes the environment; each fades over its last quarter, so neither leaves an edge. The
first profile's reach is 64 m and its allowance 20°, and both are knobs. On a calm sea a body is whole in
reflection while the ray from the water to its top is no longer than the reach: a craft 3 m tall is whole out to
about 85 m from a camera 1 m up, 190 m from 6 m up, and 900 m from 40 m up. Only what the camera sees of a body
can be reflected or seen through the water, so a body's underside never shows; that is screen-space reflection's
own limit.

*The scene's reflection.* Where the water's slope variance is under the profile's mirror limit, a reflected ray
that met no registered body is marched in screen space against the copied depth, for a bounded number of steps
over the reach. A hit reads the copied radiance at the hit. A miss, a ray that leaves the screen and a hit hidden
behind nearer geometry take the environment, fading over the last quarter of the reach and toward the screen's
edge. Rougher water takes the environment alone: its reflection is a blur the environment's colour already gives.
This trace can reflect only what is on screen. It serves water that has no mirror view.

*The scene through the water.* A refracted ray that met no registered body reads the copy at its own pixel, moved
by the surface's slope in proportion to the thickness, and keeps the unmoved pixel when the moved one is nearer
than the water. The bed is then `L_behind`, by the same conversion as a body's.

**What the gather holds.** The scene target does not hold radiance. The forward pass wrote
`(T_c · L_body + S_c) · E` there, where `(T_c, S_c)` is its aerial transport from the camera to the body and `E`
the exposure scale. Used as `L_behind` or `L_reflected` as it stands, that colour would be exposed twice and
would carry the camera's air into a second air segment. The gather undoes both as it copies, with the lent
operations that wrote them:

`L_body = max(colour / E − S_c, 0) / T_c`,

where `(T_c, S_c)` is `ATMOSPHERE`'s aerial lookup at the texel's screen position and its gathered distance. It
stores the result in the scene target's pre-exposed range, and the surface pass divides the scale out as it
reads. What a ray then reads is radiance leaving the body toward the camera, unexposed and with no air on it. No
new product is asked of `ATMOSPHERE`. The rays use it so:

- *Refracted:* `L_behind = L_body · exp(−K z)`, with `z` the hit's depth below the surface and `K` the
  water type's rate for light from above, as [under water](#under-water). The forward pass lit
  the body as if it stood in air, and this is the water its light crossed on the way down, taken vertically. The
  view path's `T_water` and the water's in-scatter are the equation's own, applied once.
- *Reflected:* `L_reflected = L_body`. The equation's `T_air` and `S_air` carry the air from the camera to the
  surface point, once. The air between the water and the body is left out, because `ATMOSPHERE` lends transport
  from the camera only; that segment is no longer than the reach, over which its air passes more than 99.7% at
  every wavelength (part E).

The declared approximations, and where they hold:

- Radiance toward the camera stands in for radiance toward the water. It is exact for a matte surface and
  misplaces a highlight.
- The light's path down to a submerged body is taken vertically. The refracted Sun is never more than 48.8° from
  vertical, so the true path is at most 1.5 times as long.
- A body further than 4 km from the camera is not gathered and takes the environment. It is under a pixel tall
  there, and inside that range `T_c` stays far from zero.
- Where a scene's forward pass shadows its aerial in-scatter, the lent `S_c` is too large by at most the
  camera-to-body in-scatter; the clamp keeps the result from going negative. A texel the forward pass clamped at
  the half-float ceiling stays clamped.

**What stays tested.** The equation above is the mechanism and a diagnostic. Two properties of what one pass
hands another are contracts (C08), and a fault in either hides at the settings a scene usually runs at: a
second exposure multiplication is invisible at a scale of one, and haze left in the copy passes any test of
where the gather reads. So where each path first ships a test holds, without the equation:

- *the exposure scale once:* with only the scale changed, a water pixel's stored colour over the scale does
  not change;
- *each medium once:* what a gathered surface supplies to a water pixel is dimmed by the camera's air one
  time, seen from above (WP8), and by the water between the lens and that surface one time, seen from under
  water (WP15). The mirror view's two stretches of air are WP14's.

### Mirror views: scenery the camera cannot see

What is on screen cannot supply a reflection whose source is above the frame, past its edge or hidden behind
a nearer hill, and on still water that is most of what a viewer sees. So for water that can show scenery the
scene is drawn a second time, from the camera's mirror image in the water's plane.

- **Who draws it.** [`SCENE-VIEWS`](../ARCHITECTURE.md#scene-views), a module of its own, because a second
  view of the scene belongs to the scene's drawing and not to an effect: it needs the view's culling, lights,
  shadows and air. Water asks for a view and reads the result.
- **What water asks for.** A plane, which is a body's level with its stage; the mirrored camera; an extent,
  half the render extent each way in the first profile; and a field of view widened by the profile's margin,
  so a ray a wave has tilted still lands inside. The view holds the scene's opaque geometry above the plane,
  lit as the main view lights it, in the scene target's own encoding for that view,
  `(T_v · L_body + S_v) · E` with `(T_v, S_v)` the air between the plane and the body, with its depth and its
  coverage. It holds no sky, no water and no after-opaque effect. The sky and its clouds reach a reflection
  through the environment product, as they did.
- **Which water has one.** A plane serves every body whose level lies within the profile's plane tolerance of
  it, 1 m in the first profile, because the lookup corrects for the surface's own height: a lake and the
  channel that leaves it share one. The profile gives the count of mirror views in a frame, two in the
  first. Each frame the levels in view are ranked by the screen area of their water, and a level keeps its
  view until another's area passes it by a quarter. Water whose level has no view uses the screen-space rays.
- **The lookup.** Seen from the mirrored camera, every point of a ray reflected off flat water lies on one
  line of sight. So the reflection at a point on the plane is one read of the mirror image where that point
  projects, exactly. A surface point the waves have lifted and tilted is corrected by a fixed-point step:
  take its reflected ray as far as the mirror's depth says the last sample lay, project that point into the
  mirror and read again. Two corrections are made. A ray that leaves the image or meets no coverage takes
  the environment product.
- **Units.** The reflected path has two stretches, and each has one owner. The view's air starts at the
  plane ([`SCENE-VIEWS`](../SCENE-VIEWS/DESIGN.md#what-a-view-owns-and-what-it-shares)), so the image holds
  the body's radiance with the air between the water and the body on it, which the screen-space rays leave
  out, and none of the air between the camera and the water. The surface pass takes it as it stands:
  `L_reflected = colour / E`. The optics equation's `T_air` and `S_air` then carry the air from the camera to
  the surface point, once, as for every other term. Nothing is subtracted or divided, so fog between the
  camera and the water reaches a reflection through the lent operation while the image keeps clear air.
- **Roughness.** The view lends one level. The water filters it into mips of premultiplied radiance and
  coverage in a resource of its own, and a pixel reads the level
  its slope covariance and its ray's length cover, as it reads the environment product. A reflection
  therefore softens and stretches as the wind rises, and at its edge it blends onto the sky's. Past the
  coarsest level the scenery's share fades into the environment.
- **With the other readers.** A registered body is geometry and is in the image, so water with a view casts
  no screen-space ray for reflection. The rays through the water are unchanged.
- **The request.** [`SCENE-VIEWS`](../SCENE-VIEWS/DESIGN.md#slots-and-requests) takes a camera: the water
  gives it a rigid camera at the mirror image of the main one and the plane as its clip plane, asks for the
  view's air to start at the plane, and reads the image by projecting points through the view's own matrix, jitter
  included, so nothing is flipped. The view holds the plane at least 5 cm from its camera, so a lens at the
  waterline still has a mirror.

In a vertical section of a lake 60 m across under a bank, a tree and a hill, from cameras 2 m and 10 m up,
the lookup on flat water returns the scenery point the true ray meets. Under ripples of 2 cm and waves of
0.1 m and 0.3 m, the plain lookup with no correction is off by a median of 11% to 13% of the ray's length and
takes sky for scenery or scenery for sky on 8% to 23% of rays. With two corrections the median is 0.02% to
0.09%, the 95th percentile at most 8.4%, and at most 1.3% of rays are confused, all of them at the skyline
or the tree's top (part P).

Those figures show the correction can work, and diagnose a reflection that looks detached. They gate
nothing: a count of confused rays can tie while a post visibly leaves its foot. What is tested is the
flat-water lookup, a projection that must equal itself, on the Mac and in Chrome. Rough water is judged by
eye in WP14, on a tree and a far shore that must stay joined to their feet as the wind rises.

Declared approximations: radiance toward the mirrored camera stands in for radiance toward the water, which
is exact for a matte surface. A level further than the plane tolerance from every plane has no view. A
surface point off the plane sees scenery from a viewpoint that is off by twice its height, so the hidden side
of a near object can be wrong by that parallax, and the image's air starts at the plane and not at the lifted
point, which moves a term of about a ten-thousandth of the body's radiance by a few per cent. Whatever the view leaves out, other water among it, is not
reflected.

### Under water

The view from under water is a capability. While the whole lens stands above the highest surface within its
reach, which the CPU tests from the model each frame, none of its work is done.

- **The lens, pixel by pixel.** A pixel is under water when its own point on the near plane lies below the
  water's surface in that column: the map's body there, its level, and the waves displaced onto it, by the
  function the surface is drawn from. A lens half under is therefore split along the line where the surface
  meets it, wave by wave, and a camera in the hollow of a swell is dry. Along that line a **meniscus** a few
  pixels wide is drawn, in which the two compositions meet and the surface, seen edge on, takes its grazing
  reflection.
- **The surface from beneath.** The surface is drawn two-sided. Its underside is shaded by Fresnel's law
  from the water's side, with a refractive index of 1.333. Within 48.6° of the surface's normal a ray leaves
  the water, which is Snell's window. It reads the gather where it meets copied geometry that stands above
  the water, and otherwise the environment product in the refracted direction, with the air beyond the
  surface applied once. Outside the window all of the light is reflected. The reflected ray is traced on
  screen as it is above water, against registered bodies and the copied scene, and where it finds neither it
  takes the water's own light along the reflected path down to the map's bed. The slope covariance softens
  the window's edge. Foam dims what the window lets through and scatters the water's light back, so from
  below it reads as a pale ceiling.
- **The water between.** For every pixel under water the view path runs through water from the lens to the
  first of the underside and the copied opaque depth, a length `s`. The pixel is `T_w(s) · L_end + S_w`, with
  `T_w = exp(−c s)` in each channel from the water type. `L_end` is the underside's radiance, or a body's or
  the bed's from the gather as `L_body · exp(−K z)`, with `z` its depth below the surface, which is the rule
  the see-through ray already uses. `S_w` is the light the water scatters into the path. The water is lit
  from above, by the Sun through the surface and the cloud factor once and by the sky, and that light falls
  off as `exp(−K z)`. Along a path from depth `z₀` that climbs at sine `m` it integrates to
  `J₀ exp(−K z₀) (1 − exp(−(c − K m) s)) / (c − K m)`, which part Q holds to a numerical integral. `c`, the
  backscatter and `K` are the water type's. The exposure scale multiplies once.
- **How it is drawn.** In the water's own pass. A draw over the pixels under water writes the composition
  for those that see no surface, from the gather, and the two-sided surface then draws over it wherever it
  is nearer. The gather copies the screen while any pixel is under. Nothing is laid over the frame
  afterwards, so no pixel takes the water twice, which [WP15 tests](#optics) on a panel seen through two
  lengths of water.
- **The frame contract.** The underside writes depth, motion and reactive coverage as the upper side does,
  so a cloud march still stops at the water. Crossing the surface is not a reset: each pixel changes side by
  itself. The meniscus writes reactive coverage.

Declared approximations: in a profile without [caustics](#light-under-water-caustics-shafts-and-the-meter)
the bed is lit evenly through the surface, and without shafts the water's light is even. The exposure scale
is the camera's, and follows the light down only where the game gives its camera the water's factor.
Sunlight's path down to a body in the gather is taken vertically. Under a crest that overhangs, the highest surface decides, so inside the hollow of a pitching
wave the lens reads as under water. Anything transparent that a scene draws after the water takes none of
the water's murk.

### Light under water: caustics, shafts and the meter

Sunlight under water is not even. The waves focus it into a moving net on whatever lies below, the net
stands in the water as shafts, and all of it dims with depth, which the camera has to follow. One field
carries the first two and one number the third.

- **The beam and the rest.** Sunlight below the surface falls off as `exp(−K z)`. The share of it that has
  not yet been scattered, `D(z) = exp(−(c − K) z / μ)`, with `μ` the cosine of the refracted Sun from the
  vertical, is still a beam: waves focus it and a hull shadows it. The rest has been scattered and is even.
  So nets and shafts fade with depth by the water type alone. With `c` = 0.25 and `K` = 0.08 a metre and the Sun 30° from
  the zenith, the beam is 0.83 of the Sun's light at 1 m, 0.40 at 5 m and 0.03 at 20 m.
- **The caustic field `C`.** The beam's irradiance at a place and a depth, as a ratio to its irradiance
  under flat water. It is made each frame in `simulate`, by carrying the surface's light down. Over a
  **window** of the water's plane, each cell of a grid is moved along the Sun's rays as the surface refracts
  them there, by the slope of the function the surface is drawn from (the wave field at the body's gains
  and the interaction waves), to a level plane at one depth. It is drawn there with additive blending, and
  each cell spreads its own light over the area it lands on. Light gathered brightens, light spread out
  dims, sheets that cross add, and the total is the light that entered. One draw is one **rung**; rungs are
  a factor of √2 apart from 0.25 m to 16 m, thirteen in the first profile. Each is then blurred by its depth
  times the slope of the waves the cells do not resolve, which the wave field keeps as a variance, and by
  the Sun's disc. A rung is addressed along the Sun's mean refracted ray, so a beam keeps its place from
  rung to rung.
- **Reading it.** At a depth between two rungs the field is their mix in the logarithm of depth. Shallower
  than the first rung its departure from 1 scales with depth, so it is exactly 1 at the surface. Deeper
  than the last it is the last. Outside the window it is 1, reached across a border. The window follows the
  lens, snapped to its cells so the net never swims, and lies ahead of the lens where its view meets the
  water. The first profile has one window 18 m across with the finest cascade's cell, 3.5 cm. With the
  field goes one texture of the water's level and its beam rate `(c − K)/μ` in each column of the window,
  from the map.
- **On what lies under water.** The water registers a **Sun-path factor**, `1 + D(z) · (C − 1)`, for a
  receiver `z` below the water's level in its column. The forward pass multiplies the receiver's direct
  Sun by it, beside the cloud factor ([a ruling](#what-waters-asks-of-other-modules)). So the net is on the
  bed, a hull and a sunk float as the Sun's own light is: none inside a shadow, none on a face turned from
  the Sun, and none above the water's level, where the factor is exactly 1. The level is the map's and not
  the displaced surface, a declared approximation: under a trough a receiver below the level and above the
  water still takes the net, and under a crest one above the level takes none. WP16 judges it on a receiver
  that crosses the waterline. It reaches the gather inside the
  receiver's colour, so it shows from above through clear shallows as it does from below, and no water pass
  applies it a second time. The depth's own dimming, `exp(−K z)`, stays where the gather applies it.
- **The Sun's path bends once.** Under water the Sun's rays run along its refracted direction, for every
  reader. The field is addressed along it. A receiver under water reads its Sun visibility at the point
  where its refracted ray leaves the water, which the same ruling gives the forward pass. The shafts read
  theirs there too. So a hull's shadow on the bed, the dark water under the hull and the lean of the beams
  agree, and none of them leans past 48.6°. Both readers take that point from one function. A receiver that
  sampled the shadow along the Sun's direction above the water while the shafts sampled it at the exit point
  would each draw a dark patch, in two places. WP16 tests that the bed's shadow and the dark water over it
  meet, on the Mac and in Chrome. The dark water is known there by what the blocker does to it: just above
  the shadowed bed the shaft pass leaves the water darker than it is with the blocker taken away, since
  the marched part, below, is negative under a blocker and not zero. Where Snell's law puts the two
  patches is a diagnostic.
- **Shafts.** The water between the lens and what it sees ([under water](#under-water)) scatters `J₀`
  evenly. Of that, the part `J_s` that the Sun's light under the surface supplies is, while it is still a
  beam, scattered by the water type's phase function `p` of the angle `γ` between the view path and the
  refracted Sun, normalised so that even scattering is 1, and it carries the field and the Sun's
  visibility `V`. The source on a path is `exp(−K z) · (J₀ + J_s · D(z) · (p(γ) · C · V − 1))`. Its even
  part is the closed form the view from under water already has. With `κ = (c − K)/μ`, the part that
  depends on direction alone is a second closed form,
  `J_s (p(γ) − 1) exp(−(K + κ) z₀) (1 − exp(−(c − (K + κ) m) s)) / (c − (K + κ) m)`, which brightens the
  water round the Sun's direction. What is left, `J_s p(γ) D (C V − 1)`, is zero in a calm with nothing
  overhead and is **marched**: a bounded count of steps along the path, each read at the level of the
  field's mips that covers the stretch of the map the step crosses, so a path across the beams reads their
  mean and only a path along them resolves them. The march runs at a reduced resolution with a jittered
  offset, is joined to the full picture by depth, and writes no history of its own. `V` is the scene's Sun
  visibility, lent as one tap ([a ruling](#what-waters-asks-of-other-modules)), at the point where the
  sample's refracted ray leaves the water.
- **The marched part is signed.** The two closed forms hold the beam as if nothing blocked it. Where `V` is
  1 and the field is 1 the march adds nothing. Under a blocker `V` is 0 and the march is `−J_s p(γ) D`: it
  takes the blocked beam back out, so the water under a hull is darker than the open water beside it, by
  what the beam would have scattered there, and is not merely water without shafts. A march that wrote
  zero under a hull would leave that water as bright as open water. So what the march stores, its
  reduced-resolution target and its join to the full picture all keep the sign, and nothing clamps it at
  zero before the three parts are summed. The sum cannot go negative: `J_s` is a share of `J₀` and `D` is
  at most 1, so the source is never below `J₀ − J_s D`. With the example coefficients above, a bed 6 m
  down and the Sun 20° up (`μ` = 0.71), `D` is 0.24: beside a floating box's shadow the marched part of a
  sample at the bed is 0, and inside it −0.24 of `J_s p(γ)`.
- **The meter.** V4's exposure is a level incident meter at the camera. Under water the light at the lens
  is the light above times `f = t · exp(−K z)` in each channel, with `z` the lens's depth below the surface
  the model evaluates at the display's time and `t` the surface's transmission of light from above, mixed
  toward 1 by the share of the lens that is above water. `thrum-water-model` gives `f` for a camera, the
  game raises it to its camera's **adaptation share** and puts it in its camera request as the meter's
  medium factor ([a ruling](#what-waters-asks-of-other-modules)), and the meter multiplies the incident
  light it reads by it. A share of 1 holds mid-grey whatever the depth, 0 is the sky's meter, and between
  them deeper water is darker by the rest. The example starts at 0.75, a value to be tuned, which leaves a clear sea
  0.6 of a stop darker at 20 m and a murky lake 0.6 darker at 3 m (part T's water types). The meter's own low-light key and
  its limits apply as they do at dusk.

In a vertical section under a rippled surface, with the Sun 30° from the zenith and winds of 1, 4 and
8 m/s, the map's mean stays within 2.3% of 1 at every depth to 16 m. Its contrast is within 20% of a
reference that carries every wave down to 2 cm on a grid sixteen times finer, from 1 m down; shallower it
is between half and nine tenths of the reference's, because ripples shorter than 7 cm make a net finer than
the cells. Without the blur it is 60% to 85% over from 4 m down. Read midway between rungs √2 apart the field keeps
94% of the contrast of a map made at that depth and correlates 0.98 with it; a factor of 2 apart, 85% and
0.95. Looking up along the refracted Sun from 3 m down, the beam's scattered light varies by 14% from ray
to ray across a 6° fan, by 8% at 5° off it, 1.2% at 10° and not at all level, so shafts stand round the
Sun's direction and nowhere else. A march of 16 steps is within 1.4% of a fine integral along the beams,
and within 3% off them and across them once each step reads the mean of what it crosses, where point
samples across them are 10% off (parts R and S).

The field's own contrast does not fall with depth. It climbs to the depth where the waves focus and stays
high below it: at 4 m/s the map's is 0.31 at 0.5 m and 0.40 at 8 m (part R2). What softens with depth is the
net on a receiver, `D(z) · (C − 1)`, whose contrast there goes from 0.28 to 0.09, together with the blur. So
nothing asks the field to fade with depth, which would dim the net a second time. The field is tested for
what it must be whatever its tuning: exactly 1 under flat water, uneven under waves, changing with the
surface and from rung to rung. The softening is judged by eye on the bed and the hull in WP16.

Declared approximations: the net is no finer than the waves the wave field resolves, 7 cm, and ripples
below that only soften it. Beyond the window the bed is evenly lit. A blocker shadows the water where its
light enters, so the part of a hull below the waterline shadows nothing its waterline shape does not. A
receiver under water is shaded with the Sun's direction above the water; only where its shadow falls
follows the bend. The Moon makes no net. Seen from above the water, the water's own light stays even: the
net on the bed shows, shafts do not. The meter's depth takes the wave field and not the interaction waves.

### Capabilities, recipes, limits and knobs

- **Capabilities.** A scene's water profile names the capabilities it wants. One it leaves out declares no pass
  and no resource, and the surface's shader is prepared without its reads, so it costs nothing. Changing the set
  is a profile change.

  | Capability | What it adds | Needs |
  |---|---|---|
  | Open water, always present | The map, the surface, and the optics of sky, Sun and water | |
  | Waves | `SEA`'s wave field at each body's gains | |
  | Body query | The query dispatch, its ring and mailbox | |
  | Interaction waves | The two levels and their sources | Body records or impulses |
  | Whitewater | The foam stocks, their births and transport, and the whitecap estimator beyond them | A foam pattern; hull births need body records |
  | Surf | The shore transform's lags and exposures, the bed's min-max pyramid, and the breaker section | Waves; a bed |
  | See-through water | The gather where water is thin, and the scene through it | |
  | Scene reflection | The gather and the march where water is mirror-still | |
  | Body optics | The gather and the rays for registered bodies | Body records |
  | Scene mirror | Mirror views and their lookup | `SCENE-VIEWS` |
  | Underwater view | The lens test, the underside of the surface and the water between | See-through water |
  | Caustics | The caustic field, and the Sun-path factor on what lies under water | Waves or interaction waves |
  | Shafts | The beam's scattered light in the view from under water | Underwater view; caustics |

  Open water alone is a still pond under the sky. With waves it is the sea of a sunset demo.
- **One physical profile on every recipe.** The map and its joins, the gains, the swash ratio and margin height, component count, cascade layout and loop, the shore transform's wavelengths, stage spacing and floor, the breaker ratio and the breaker section, the interaction levels, their depth ladders and their count of solves, and query
  semantics are the profile's, never a quality setting (C04). Chrome and the Mac float the same body on the same
  sea.
- **Limits are inputs.** Transform length, workgroup memory, storage formats and counts, and sampled textures per
  stage are read from the capability record and refused at preparation when exceeded. Levels and endpoints are
  array layers so the surface pass stays inside sixteen sampled textures.
- **Recipes.** Both recipes run the portable kernels first. An Apple-tile variant (f16 working precision, a
  different transform layout) is promoted only by a paired measured win with no visible difference in the same
  views and the body query's tests still passing.
- **Knobs.** Each is a named runtime value in `EFFECT-GRAPH`'s list with a declared invalidation: surface mesh
  density; field-cache reach; the cadence of the field cache and of cascades below the profile's smallest force
  wavelength; foam cadence and level size; reflection steps, reach, slope allowance and mirror limit; the count of mirror views and their extent; the gather's resolution and the sight depth; the caustic window's size and cell, its count of rungs and its cadence; the shafts' steps and resolution. A layer evaluated less
  often than every frame is carried between evaluations by its rate of change, with its slopes held. None changes
  what a query returns.

### What the later modules build on

Nothing in this section is built here beyond the seam it names. Mirror views, the view from under water
and the light in it are the shared system's, so a river and a puddle have them with no work of their own; surf needs long waves
and a still body, so neither has it. Each module is planned by its own `plan-module`
run from [its brief](../ARCHITECTURE.md#rivers-and-puddles), with the research and reproductions its mechanisms
need. A cue of theirs that a seam cannot carry is settled in the module list, with this module.

- **`RIVERS`: running water and waterfalls.** A river is a chart. Its level, current and loss come from a steady
  shallow-water solution over the bed from declared sources, prepared once and stored, never stepped in the
  frame. It adds the flow's roughness as gains, and births of foam from the chart's loss, through the rules above.
  A waterfall is a fall record joining two bodies where the flow leaves the bed: a sheet drawn with this module's
  optics and pattern, and a plunge that enters the lower body as impulse records and births. Its solver writes
  a chart's footprint, ends and join weights with its level, current and loss, to [the rules preparation
  holds](#the-water-map). What this module proves for it: a chart is drawn, queried, carries detail, foam and
  floats, and joins a lake and the sea under a swell and through their stages (WP9). What it leaves to `RIVERS`:
  how a river's reach answers its source's stage, which here stays as prepared.
- **`PUDDLES`: rain, puddles and wet ground.** A puddle is a basin whose stage rain raises and drying lowers, in
  hollows found from the bed. Rain rings are one more short wave layer. Wet ground is not a body: it is a wetness
  the ground's material reads, written by rain and by water that has left, which needs a ruling with the forward
  pass's owner. On a shore the waves wash, this module draws the sheen and the foam as its film, and the map
  gives each body's wave support, so the ground's material can darken the same sand and the two agree on one
  support (C07). What this module proves for it: a basin of any size whose stage moves (WP4, WP10), rings on water
  0.25 m deep (WP6), and the ground seen through thin water (WP8). What it leaves to `PUDDLES`, as a question its
  planner settles here with this module: two basins may not overlap, and a stage that would merge them is
  refused, so how puddles that overflow into each other join and part again, keeping each body's identity,
  stage and the rings and foam on it, with no rebuild or reset a viewer could see.

### Check that it can work

- **S1, S2 and the oracle, and time for the wave field.** In [the sea's design](../SEA/DESIGN.md#check-that-it-can-work).
- **W1.** A level over a bed, cut by the scene's depth, is how Crest and Unreal place lakes; the hollow round a
  seed is a flood fill with a closed form on simple beds, which WP1 tests. Unrun on the GPU.
- **W2.** Part I gives the gains above, and part L the waterline's travel under the limit with a swash depth,
  against its closed form, with the pinned waterline of the limit alone beside it. That a lake reads right with
  the short layers alone, and a beach with a swash and no shoaling, is unrun on the GPU; WP4 is the first look,
  and [risk 12](#risks) covers a beach that reads wrong.
- **W4, and waves at depth.** Part H measures the selected step, the blend of ladder depths, as an operator on
  each wave vector and as fields over beds that vary, in steps of 1 and 8 ticks; [its table](#interaction-waves)
  is the envelope. Part J measures dry ground: one transform passes 3.2% of a pulse across a 2 m bar, a solve
  for each region passes nothing, and thin land inside one region passes what that section declares. Both are
  1D sections in f64, so a field constant along a straight shore; neither is a GPU measurement.
- **W6.** Part K: the map's function with joins, on a grid of an island with a lake, a channel and the sea,
  through the lake's and the tide's ranges under a swell crest, against taking one body over another. Then the
  assembled surface, the level plus each layer after its depth limit, either side of ends from 4 m deep to
  none, with the swash depth from the row against one kept for each body, and beside a mouth with backwater
  from the reach against the still level. It is the specification's arithmetic in f64; the drawn surface is
  unrun.
- **S3.** `wake_proof.py`, with a Gaussian stand-in for a 6 m hull. Transverse wavelength at 3 and 6 m/s: 5.72
  and 23.44 m against `2πU²/g` of 5.77 and 23.07 (−0.8%, +1.6%). Ridge half-angle at 3 m/s (Fr 0.39): 20.9°
  against Kelvin's 19.47°; at 6 m/s (Fr 0.78): 9.6°, narrower as Rabaud and Moisy report. With the source removed
  a tracked crest moved 6.00 m in 2 s, its phase speed, and after 10 s waves had left through the border with no
  wrap. Two banded levels against one fine level through a 60° turn: 2.2% RMS.
- **S4.** The foam spike's blind verdicts: the stochastic sampler removed the repeat; biased onset with swept
  birth gave caps at gale, fewer at fresh breeze and none on a glassy sea. Its open defects are the look work
  named above. Part M: the swash's carriage on parcels and on 0.25 m foam cells, on plane beaches under regular
  waves and two trains, against the current's and the wind's drift alone, which moves nothing. It is a 1D
  section in f64 with no births, ageing or spreading; how the carried foam looks is unrun, and WP7 is the first
  look ([risk 17](#risks)).
- **S5.** Every light term reads a product `ATMOSPHERE` WP5 lends in physical units, so a seam with terrain can
  only come from the sea's own shading. The one input that is not physical, the copied scene colour, is turned
  back with the lookup and scale that wrote it: in part E the converted gather gives the true stored colour,
  1.142, where the copy used as radiance gives 0.354 and the copy with only exposure divided out 1.001. The
  receiving rectangles are tested in part C against rays reflected and refracted on the CPU, for cameras 1 to
  40 m up, bodies 12 to 100 m off, and seas from flat to 3 m of amplitude with a 20° allowance: in 36 cases, no
  ray that met the body came from outside its receiving rectangle, and in every case some came from outside the
  body's own rectangle, 92 to 2,010 of those sampled. The design review's case, a 2 m body 20 m from a camera 2 m up, is among them.
- **S6.** Unrun. `EFFECT-GRAPH` WP3 proves motion and reactive reach reconstruction; how the sea looks after it is
  first seen in WP10.
- **Hitches.** Parts A, B and D, reported under
  [time](#time-content-functions-and-state), [interaction waves](#interaction-waves) and
  [whitewater](#whitewater).
- **W3.** Unrun. Regional copies and screen-space reflection for water are what Unreal, HDRP and Far Cry 5 ship;
  the units are the sea design's part E. WP8 measures the copy and the march where they fill the view.
- **W5.** Unrun. Two cross-faded samples carried along a flow are Portal 2's and HDRP's; the pulse they can show
  is [risk 13](#risks).
- **W7.** Part N: the lag from an eikonal solve against Snell's closed form on a plane beach, the blends
  between wavelengths and between stages, and an island, where the transformed wave meets every shore from
  the sea and the exposure calms the lee. Part O: the break point against Komar and Gaughan, sets, a thrown
  section that overhangs without crossing itself, and the swash's closed form with the transform on. All are
  f64 arithmetic on sections and small grids. Part U: a lake that land encloses, with no path to the bed's
  edge, takes its lag from its own waterline and its exposure from the fetch upwind. Part V: the plain
  iteration's misses under a lip. Part W: the two blends together against the dispersion relation and against
  the solve itself. Part X: the cut for the highest surface, on a straight section, on crossing trains and on
  a beach, against a cut eight times finer, with the throw reach held to every throw. That the section reads as a
  breaking wave is the WP13 inspection. The inner swash profile has one zero on a rising transect with
  constant gains: for `d ≥ −s`, its slope in signed depth is `1+c·q ≥ 1−c > 0`. In the overlap both profiles
  have positive thickness, so blending keeps the sign. The model cut and motion inspection cover the joined
  result ([risk 20](#risks)). The example's lake is too small to raise long waves, so WP12 and WP13
  prove it in the model and on the GPU against the model.
  [Part AA](../../../.orchestrate/evidence/WATERS/WP19/query-reach-study.md): the cut in the island's gale,
  in f64 at `6bad9e6e8`. The reach is 87.5 m and no node is displaced further than 20.2 m; of 1,600 random
  columns over the breakers 17 hold three surfaces or more, and the whole cut is 495,000 nodes a column.
  [Part AB](../../../.orchestrate/evidence/WATERS/WP19/coarse-rule-study.md): which coarse cells the cut may
  leave out, on the same gale. Part AA's fixed margin of 0.5 m loses the highest surface at the design
  review's column and at 91 of 47,657 columns under folds. A margin of 0.5 m plus the cell's corner spread
  holds every fine node of 294,400 coarse cells at 40 times with 1.7 times to spare, and loses no surface at
  51,658 columns, at about 480 fine nodes a column in the breakers at large and 2,300 under a fold. One bed
  and one sea, on the CPU: that the dispatch then meets the age contract is unrun, and WP19 is the first run.
- **W8.** Part P: the mirror lookup against exact reflected rays in a vertical section. A second draw of the
  scene is what Unreal and GTA V ship for water. `SCENE-VIEWS`' own design owes the proof that Thrum's
  renderer can draw one; nothing here runs it.
- **W9.** Part Q: Snell's window and the Fresnel split from beneath, and the scattered light's closed form
  against a numerical integral. The lens test reads the function the surface is drawn from, so the line on
  the lens and the drawn surface cannot part. The look is unrun; WP15 is the first.
- **W10.** Part R, reported under [light under water](#light-under-water-caustics-shafts-and-the-meter):
  light conserved, contrast by depth and wind against a reference, the ladder between its rungs, and the
  substitutes. One sinusoid is brightest 12% deeper than the paraxial focus its curvature gives. It is a
  vertical section in f64, so a net of lines and not of cells. That the net reads as sunlight on sand is
  unrun; WP16 is the first look.
- **W11.** Part S: where shafts stand, and the march against a fine integral. A section with one water
  type and no hull. Under a hull the marched part is the equation's own arithmetic, negative where the
  beam is blocked ([light under water](#light-under-water-caustics-shafts-and-the-meter)); WP16
  tests its place against the bed's shadow and not its size. The look, and noise under reconstruction, are
  unrun; WP16 and WP10 are the first.
- **W12.** Part T: the light at the lens by depth for three water types, and how far the picture darkens
  for each share. Arithmetic; the descent is unrun.
- **The surface's read of the field.** Part Y finds where resolved waves become statistics, for a camera 2 m
  and 40 m up under the example's 75° view: by the footprint's longest side, and through a 16x anisotropic
  footprint over full mip chains. The longest-side rule places the change about 600 m out and 61 pixels under the
  horizon from 40 m up, where WP2's first build showed a smooth belt of 50 to 70 pixels. Part Z measures how wide
  a crest's turn is drawn on one trochoid at the example's contraction of 0.85, from a fixed world grid and from
  rest places moved forward, at texels and vertex spacings of 1 to 8 m. Both are f64 arithmetic on a flat sea in
  `surface_read_proof.py`. How the sea then looks is unrun, and WP2.2 is the first look.
- **Cost.** WP2's first record measured the inverted cache. The forward cache is unmeasured, and WP2 records it.

## The system as it runs

| State | Owner |
|---|---|
| Spectrum, components, cascades and their f64 evaluators; the wave field's GPU passes | `SEA`: `thrum-sea-model`, `thrum-sea`. After `SEA`'s last work package is Done they are this module's to extend, with their tests kept passing |
| The map and its function, joins, shore margins and what preparation refuses; gains and the swash depth; the shore transform's lags and exposures and the breaker section; the labels of a window's regions; the f64 surface model and the conformance model | `thrum-water-model` (this module), no GPU dependency. An f64 reference of the interaction waves or the foam is a diagnostic a builder may add, and is not required |
| The map on the GPU, the surface from both sides, optics, the mirror lookup, the view from under water, the caustic field and its Sun-path factor, shafts, interaction waves, foam, the query ring | `thrum-water` (this module), one effect on `EFFECT-GRAPH`'s seam, which declares the wave field's passes with its own |
| Mirror views: the second draw of the scene, its culling, light, air and targets | `SCENE-VIEWS` |
| Body declarations, the bed, presets, water types, the craft and floats, hull footprints, foam pattern, body integration, controls | The example scene `examples/water` (package `water-scene`); never in engine crates or their tests |
| Stages, state commit, cadence, knob list; the tick stage and its submission | `EFFECT-GRAPH` |
| Sky, Sun, aerial, reflection product, cloud factor, the direct-Sun path a Sun-path factor joins, the lent Sun visibility | `ATMOSPHERE` |
| The exposure meter and the camera request's medium factor | `APPLE-PRESENTATION` (V4) |
| The light factor at a camera, and the caustic reference | `thrum-water-model` (this module) |
| Motion and reactive targets, history, reconstruction, the governor | `APPLE-PRESENTATION` |
| The foam pattern's residency | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's body records, impulses, stages and wind, and the
long-wave phases and loop time for the frame's `t`. `simulate`, every frame: the cascades and the field cache at
`t`, then the caustic field's rungs. `simulate`, only on a frame that advances the tick: the interaction step with its sources, region by region; foam births and
transport. `SCENE-VIEWS` draws
the mirror views the water asked for. Forward, sky and stars draw, and what lies under water takes the
Sun-path factor. `after-opaque`: the gather, where a reader needs it; the shafts' march and the composition of
pixels under water; the surface, body by body. Reconstruction, tone and present follow. On submission the
water commits its tick and each endpoint's tick. A scene without a capability runs none of its steps.

**A tick, in order.** Outside the frame, in each advance of the frame host, before that advance's view: the
query's tick pass takes the posted points, dispatches the batch against the state the last submitted frame
left and copies the answers to the ring, in a submission of its own. With nothing posted there is none.

**The example scene** is a `Game` on `thrum-app`'s shell with a ticking world of its own, as `examples/v1-game`
is. It uses `thrum-controls`, a parameter file and `./dev scene examples/water [--web]`. Its island has a beach
of about 1 in 10 facing the trade swell, a headland and a lee shore. The bed comes from the opaque terrain's
triangles. The lake sits about 300 m inland, above the sea and under a ridge, with a second hollow beside it
and a rim above its highest stage. The example has a pond and adds a tree on the lake's bank for the optics
views. It enables surf in WP13 and uses a labelled cloud-shade stand-in in WP8, following
`examples/v1-game/src/environment.rs`.

WP9's hand-authored channel is the example's stand-in for the `RIVERS` chart. It leaves the lake through an
outlet cut in ground above the lake's highest stage, quickens in a narrows, and falls below the lowest tide
at the mouth. At low tide the mouth is shallow enough for the depth limit to hold the swell. The island's
ground, craft, floats and foam pattern belong to the example, which makes them or copies suitable material
from [Broadsides](#what-broadsides-offers).

**Creator journey.** A scene adds the water contribution with a profile that names its capabilities, and declares
its bodies, each with its stage's range: a level for a sea, a seed and a level for a lake, a chart with its ends. A scene with ground gives the bed, or the triangles
to build it from. A scene that floats things puts body records in its tick page and reads answers from the
mailbox. Changing wind, swell or a stage needs no rebuild. Changing the profile or the bodies rebuilds the water
and resets interaction waves and foam.

## What WATERS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-waters), built by this module's builder after
`EFFECT-GRAPH:WP3` is Done, with `EFFECT-GRAPH`'s items unchanged and its tests kept passing:

1. An `after-opaque` pass may write scene depth (WP2).
2. An effect's own pass may read scene colour and scene depth as textures and write only the effect's targets
   (WP8).
3. Whatever the shell, the page, the scene packaging or the capture tap assume of the V1 game or of the sky
   kit's still world is made a property of the game (WP2 on the Mac, WP3 in Chrome).

4. A mirror view of the scene, drawn by [`SCENE-VIEWS`](../ARCHITECTURE.md#scene-views), a new module with its
   own design, and not by this module's builder (WP14). The interface is
   [above](#mirror-views-scenery-the-camera-cannot-see): what water asks for, and what the view holds. The
   rest of the design does not wait for it: WP14 is the only work package that needs it.

5. A Sun-path factor from the water on the forward pass's direct Sun, read at a receiver's own position
   beside the cloud factor, with the point at which a receiver under water reads its Sun visibility (WP16).
6. The scene's Sun visibility at a world point, lent with the environment products as one tap, for the
   shafts' samples (WP16).
7. A medium factor on the incident light the camera's meter reads, which the camera request carries, 1 by
   default (WP16).

8. A stage for the body query that the frame host submits every tick, whether or not a view is drawn (WP19).
   [`EFFECT-GRAPH` WP6](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6) builds it, and this module's builder
   moves the query to it.

5 and 6 are settled with `ATMOSPHERE` and 7 with `APPLE-PRESENTATION`, as
[rulings 5 to 7](../ARCHITECTURE.md#seam-rulings-for-waters). With no water provider and a factor of 1,
every scene composes the shader text and draws the bytes it does today.

`SEA` is asked for nothing. The shore transform extends `thrum-sea`'s field cache and `thrum-sea-model`
after `SEA`'s last work package, when both crates are this module's, with `SEA`'s tests kept passing. One change
reaches a `SEA` test, and WP2 makes it: [the forward cache](#the-surface-and-the-frame-contract). `SEA`'s
`ambient_gpu` test compares each cache texel with the model's evaluation at a world point, which inverts the
map, so it pins the inverted form. The cache's oracle becomes the model's forward evaluation at the texel's rest
place, held to the same 2 mm in height and 0.002 in slope. The query's oracle, its test and every other `SEA`
bound are unchanged. The cascades and the cache become filterable textures with mips, in place of storage
buffers, so the sea needs fewer storage buffers a stage.

If `EFFECT-GRAPH` WP3 as built already allows 1 or 2, nothing is added. Everything else is used as built: the
lease of one linear 2D texture, the lent environment products and cloud factor, the temporal contribution, the
typed controls and parameter files. The map's textures are this module's own resources. Nothing is asked for the
bed: the game gives it.

## Cost account

Estimates, unmeasured, at a 60 Hz tick. Broadsides' comparators are measured there, not here. The wave field's
rows are in [the sea's design](../SEA/DESIGN.md#cost-account).

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| The map | Prepared; read by every water vertex, pixel and query | Level, bed and current on each body's grid: a 1 km square at 1 m cells is about 16 MiB | One more read set in the surface pass | |
| The shore transform | Prepared; read by the field cache and the query | For each still body with long waves: 8 directions × 6 wavelengths × its stage samples, on the bed's grid, and 8 exposures; for enclosed water the source and the exposure come from rays over the bed at preparation, as the sea's exposure does. A 1 km bed at 4 m cells with a 2 m tide range is 240 layers of 250², about 30 MiB at 16 bits, and the bed's min-max pyramid about two thirds of the bed's own bytes. Solving them is CPU work at preparation, which WP12 records | In the frame, for a cache texel over a bed, a few reads for each direction and wavelength in use where there were none, and the section's table; WP13 records it | |
| The surf field cache | Frame, with surf in the profile | One sample for each node of the surface mesh and of the distant shore's lattice, about 550,000 on the island. For a node on the bed: the transform's table once for each distinct bed cell and stage its sheet reads, 56 texels a cell at a stage sample and 112 between two, whatever the number of components, then one pass over the long components at each place the sheet is summed, which gives both presentations where the stage has not moved. An interior node reads one cell and sums once. A swash node sums again at its contour, in that place's cell; a node beside a cell's edge reads the lags across it; a ring-edge node reads and sums at its two coarser nodes; a previous presentation at another stage reads its own. For a node no triangle in view uses: nothing | Unmeasured as designed; [WP21](#a-surf-view-a-running-game-can-afford-wp21) builds it so and records the reads of each kind. As WP13 built it, 93 ms to 97 ms a frame at the beach in a gale: each of 64 components read its own 12 to 24 texels, in each of two to six evaluations a node, for every node whether drawn or not | |
| Bore decay and swash | Field-cache evaluation on frame pages and direct sheet evaluation on query ticks, on supported shores with exposed long waves | Up to one extra long-wave sum at the contour per eligible sample, with reuse where samples share a contour reference at the same time and stage. The overlap adds local arithmetic and a bed lookup if the displaced bed value is unavailable | Use the existing map, surf preparation and field-cache path. Reuse derived samples for surface and foam where their time and footprint coincide. No new solver, per-frame upload or readback. WP13.12's paired run covers field-cache and query cost, including any prepared data or sample storage | |
| Interaction waves | Tick | 2 × 256². For each region, one complex transform forward and one back for each ladder depth in use: two transforms a level over deep open water, up to 16 solves a level at the profile's bound. Labelling a window's regions is CPU work on the tick, when a window moves | Under 0.2 ms over deep water; about 0.7 ms at a shore with six depths in use; more at the bound, which WP6 records with the count of regions and solves | About 3.8 ms a tick in its lake, as an interval |
| Foam | Tick | 2 × 512² × 4 stocks. The swash's drift reads the surface's rate and slope at cells on a shore, where births already read the field | Under 0.2 ms | About 2.2 ms for first deposition |
| Query | Tick | Up to 2,048 points, 48 B each. A point within reach of a crest that may be thrown costs the cut, coarse to fine: for a body, the nodes of a 2 m lattice within its points' throw reach, then for each point the nodes of the 0.25 m lattice in the coarse cells that can stand over it, where a plain point costs a few evaluations. In the island's gale that is about 7,800 nodes a body and 480 a point, 2,300 a point under a fold: about 86,000 a tick for eight floats in the breakers and 174,000 with every station under a lip, where the whole cut measured 47 million and 18.9 s | Under 0.1 ms with no point in surf; WP12 records the model's cut, WP13 a tick with every float in the breakers and WP19 that tick again with the game running; 96 KiB back | 0.03 ms |
| Gather | Frame | The rectangles its readers need: nothing on an open sea with no body, the screen where see-through or mirror-still water fills it | Proportional to the pixels copied; a whole-screen copy is one pass ended on a tile GPU | A whole-scene copy and a 3 ms trace |
| Body and scene rays | Frame, inside the surface pass | Water pixels the mask marks: two ray-against-box tests for each marked body; a bounded march where a ray meets a box or the water is mirror-still | Unmeasured; WP8 records it on a calm sea and in a gale with zero, one and nine bodies, and at the lake | |
| Mirror views | Frame, by `SCENE-VIEWS` | Up to two views of the scene's opaque geometry, each a quarter of the main view's pixels | The scene's own draw again for each view, less what its owner cuts. Unreal reports a planar reflection at up to half a frame at full detail, and GTA V ships one at 240×120. WP14 records it with no view, one and two | |
| The mirror lookup | Frame, inside the surface pass | Water pixels of a body with a view: three reads of the image and its depth | Less than the screen-space march it replaces there | |
| Under water | Frame, only while a pixel of the lens is under | The lens test for each pixel, one composition for each pixel under water, and a whole-screen gather | Of the order of the surface pass's own pixels; WP15 records it half under and wholly under | |
| The caustic field | Frame, while water with caustics is in the window | 13 rungs of an 18 m window at 3.5 cm cells; the rungs from 2 m down are drawn from coarser grids, since their blur is wider than a cell | About 1.8 million vertices and 13 small passes a frame, then a blur of each. Unmeasured; WP16 records it, and the rung count, the window and the cadence are knobs | |
| The Sun-path factor | Frame, inside the forward pass | One read of the level for every opaque fragment in a scene with caustics, and two rungs for one under water | One texture more in the forward fragment stage, counted against the capability record | |
| Shafts | Frame, only while a pixel of the lens is under | Pixels under water at half the render extent each way: 16 steps, each two rung reads and one Sun-visibility tap | Unmeasured; WP16 records it | |
| Surface | Frame | Water pixels at the render extent, about 200 k vertices for the sea and tiles for each other body in view; one read set for the map and each wave layer, two for interaction waves and foam | The largest term; WP2 measures it | 10.5 ms at 1440p after the copy |
| Memory | | Interaction, foam, the query ring, the map | About 20 MiB and the map, and the gather target, whose layout is the builder's: addressed by screen position at 2560×1440 it is about 44 MiB | |

Interaction, foam and query work lands on alternate frames at 120 Hz, so both frame kinds are recorded.

**Records and tripwires, never gates** ([brief](../VISION.md#goal-and-exit-rule), decision 3). `scripts/water_cost.py`,
created in WP2, takes a paired record of one scene at one view, capture off: the scene with the water and with it
absent, on tick frames and on repeat frames, giving the whole frame, each water pass's GPU time by name, the bytes
of each resource and the load on the machine. A record trips a wire when the whole frame is over the display's
refresh interval, when a pass is about 25% or more over its last comparable record, or when a cost grows faster
than the work that was added. A tripped wire is investigated in the work package that tripped it, and the record
says what was measured to find the cause and what was fixed or left as a follow-up in the log. Visible stutter is
judged in the `realtime` inspections.

The ambition is the display's full refresh, and whole-frame tuning is the later performance phase. **No timing
figure decides whether an item passes, whether a work package is Done or whether the next one starts.**
`water_cost.py check` measures the scene again and fails only when the scene cannot be measured, when the
committed record lacks part of what a paired record holds, or when a wire the record trips has no investigation
beside it. Presented cadence on the built-in panel is measured once, attended, in WP11.

**The query's age is a contract and not a cost figure.** A game reads it: an answer older than the declared
maximum is unavailable, and the float it was for falls back to a surface that is not the one drawn. Decision 3
is about performance figures, and what a query answers a game stays tested
([the settings](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)). So
`water_cost.py running` is a second command beside `check`. It runs a scene with its ticks following the wall
clock, the bodies posting every tick and nothing waiting on the GPU, and fails when an answer is unavailable,
when a body falls back, or when a read blocks. It reports how many ticks late the answers arrived, which
decides nothing. [WP19](#a-surf-query-a-running-game-can-use-wp19) uses it.

## Risks

1. **The surface pass costs too much.** Early sign: WP2's record, and WP4's with the map's reads and several
   bodies. The plausible wrong fix is dropping layers from the pixel shader. The designed ones: fewer texture
   reads through packing, cascades fading to moments sooner, and a coarser mesh. The work package tries the ones its
   record points at and records what each did. Building goes on either way.
2. **A 160 m repeat shows from 40 m up.** Early sign: WP2's vista capture. Fallback: a larger first patch under a
   profile the portable limits still admit, designed in `SEA` first.
3. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: WP3, the first real Chrome run. The
   map adds float textures and the surface more reads; both are counted against the capability record in WP4.
4. **Self-interaction through the query.** A body bouncing on its own wake. Early sign: WP6's craft at rest must
   settle. The dent correction is the design's answer; the wrong fix is filtering the craft out of the wake.
5. **Foam reads as grit or cutouts**, the spike's open defects. Early sign: WP7's first captures. The mechanism is
   settled; the look is tuned against the inspection list, and three failed looks on one cue go to the log.
6. **The water writes depth and a later effect disagrees.** `CLOUDS` is planned after this; the ruling fixes the
   order.
7. **The example's shell glue is larger than the water.** Early sign: WP2. A ticking entry in the sky kit is a
   proposal to `CREATOR` in the log, not built here.
8. **Rays cost too much where they fill the screen**: receiving rectangles in a gale, or a mirror-still lake.
   Early sign: WP8's record. The designed remedies are a shorter reach, fewer steps and a lower mirror limit as
   slope variance rises, since rough water blurs a reflection away, and marching at a lower resolution. The wrong
   fixes are a march over every water pixel at every roughness, and a second render of the scene.
9. **The wave field costs more every frame than the reads it saves.** `SEA`'s risk; its cadence knobs are the
   remedy.
10. **A whole-screen gather costs a pass on a tile GPU wherever shallow water fills the view.** Early sign: WP8's
    record at the lake. The designed remedies are a gather at a lower resolution with the full-resolution depth
    kept, and a sight depth that shrinks the see-through region. The wrong fix is dropping the bed from view.
11. **A reflection whose source is off screen falls to the sky**, and reads as a hole in a lake under a hill.
    Early sign: WP8's pan, which is built before the mirror view. The fade at the screen's edge and the reach
    are the remedies there; the mirror view of WP14 is the real one.
12. **A beach without surf reads as a pond's edge, or its swash as a tide.** Early sign: WP4's and WP7's captures,
    which are built before surf. The designed remedies there are the swash ratio, which sets how far the
    waterline travels, the foam born through the band, and the film's sheen. The wrong fixes are a waterline
    animated apart from the waves, which the query would not know, and foam painted along the shore. Surf, in
    WP13, is the real one.
13. **The carried detail pulses** at the carriage's period. Early sign: WP9. The designed remedies are a period
    varied from place to place and a longer one where the current is slow. The wrong fix is a single scrolled
    sample, which smears.
14. **A later module finds a seam wrong.** Early sign: `RIVERS`' or `PUDDLES`' planning. WP9 proves the chart on
    a stand-in so that the seam is tried before a solver exists.
15. **Regions and depths multiply the interaction step.** Early sign: WP6's record at the lake's shore, with its
    count of solves. The designed remedies are the profile's count of solves and, on the far level, a coarser
    ladder with its measured envelope stated. The wrong fix is one transform over land and water.
16. **The film fights the ground or floats over it.** Early sign: WP7's captures at grazing angles. The designed
    remedy is its depth bias and a bed built as [the map's](#the-water-map) is, from the highest ground in each
    cell. The wrong fix is drawing foam into the ground's material here, which is `PUDDLES`' seam.
17. **The swash gathers its foam into one hard line**, or strips the sand behind it bare. Early sign: WP7's
    captures of the beach. The designed remedies are the stocks' own spreading, births under the whole sheet,
    and the drift's bound. The wrong fixes are a pattern scrolled up the beach, and foam drawn at the waterline
    from the present wave shape.

18. **A breaking crest tears or stair-steps**, because the section stretches the sheet past what the mesh
    resolves. Early sign: WP13's captures from 40 m and from 1 km. The designed remedies are refinement along
    the band where the breaking index passes 1 and the throw's fade with the mesh's spacing. The wrong fix is taking
    the throw out everywhere. What is left of it seen close is WP20's. There a cue is diagnosed from the
    submitted frame before it is repaired, on the target and in the view and wind that show it, so a cue
    seen only in Chrome is read in Chrome. A cue can have more than one cause at the same pixels, as when
    foam shows up a branch that wrongly wins depth, and its diagnosis names each cause its frames support.
    A repair that changes which surface wins depth is held on the target where it changes the winner,
    at a cue whose diagnosis there named depth. On Metal that is a case in `surf_frame_gpu` that fails
    without the repair. That test opens Metal, so it cannot fail for a branch only the browser draws: in
    Chrome the evidence is the diagnosis's own depth reading taken again on the repaired build, at the
    same view, engine time and pixels, now at the model's nearest surface. A repair that changes the
    winner only in Chrome owes no Metal case. WP13.9's frame contract is checked on Metal either way.
19. **The lag shows**: a step in the swell where the bed begins, or a jump as the tide crosses a sampled
    stage. Early sign: WP13's tide sequence. The designed remedies are a wider border at the bed's edge and
    closer stage samples. The wrong fix is holding the lag at one stage. In a lake the same sign is a crease
    downwind of a point on its upwind shore; the designed remedy is the run of deep water at the source that
    part U measures.
20. **A steep ridge marches ashore or a dry gap opens behind the bore.** Early sign: the front stays high and
    the cross-shore cut dries and wets again before the final waterline. WP13 keeps the offshore pitch and
    tapers the bore into the [bounded swash profile](#surf-the-shore-transform-and-the-breaker). Inspect
    the bed crossing before tuning foam or mesh density. A live window remains the fallback under
    [the candidates](#surf-mirrored-scenery-and-the-lens-decided-against-the-cues) if the shared profile
    cannot deliver the cue after visual review.
21. **A mirror view costs a second scene.** Early sign: WP14's record. The designed remedies are its extent,
    a shorter view distance and coarser detail, which are `SCENE-VIEWS`' to offer, and one view in place of
    two. Ray query on the Apple recipe is the later one. The wrong fix is dropping the lake back to the
    screen-space trace.
22. **Reflections swim or ghost under reconstruction**, because the water's motion is its surface's and not
    its reflection's. Early sign: WP14's pan. The designed remedy is reactive coverage where a mirrored
    reflection is most of a pixel. The wrong fix is taking the water out of reconstruction.
23. **The line on the lens flickers, or a gap shows where the near plane cuts the surface.** Early sign:
    WP15's crossing. The designed remedies are the meniscus's width and a lens test at each pixel's own point.
    The wrong fix is a camera that is kept from touching the surface.
24. **Deep water is black**, because the exposure is metered from the sky. Early sign: WP15's views at depth,
    which are built before the meter follows the light. [The meter](#light-under-water-caustics-shafts-and-the-meter),
    in WP16, is the remedy; the wrong fix is brightening the water's own light.
25. **The caustic field costs too much**: thirteen draws of a fine grid every frame. Early sign: WP16's
    record. The designed remedies are coarser grids for the deep rungs, rungs a factor of 2 apart, a smaller
    window, and a cadence below the frame's with the field carried between. The wrong fixes are one plane
    for every depth and a played texture.
26. **The net reads as a pattern laid on the bed and not as light**: too even, too sharp at depth, or
    crawling under reconstruction. Early sign: WP16's first captures. The blur by depth and the beam's share
    are the mechanism; the look is tuned against the inspected items, and three failed looks on one cue go
    to the log.
27. **Shafts boil or band**, because the march is a few jittered steps. Early sign: WP16's stills and WP10's
    sequences. The designed remedies are more steps, each step's read of the field's mean, and a march
    resolution nearer the picture's. The wrong fix is a blur of the finished frame.
28. **The forward pass will not take another texture in Chrome.** Early sign: WP16's preparation against the
    capability record. The designed remedy is the level packed into a layer of the field's array.
29. **The picture pumps or steps as the lens crosses the surface**, because the light at the lens changes
    faster than a viewer's eye would. Early sign: WP16's crossing. The designed remedies are the mix by the
    lens's share and the adaptation share; the wrong fix is a gain taken from the picture.
30. **A device admits less anisotropy than 16.** The smooth belt then comes nearer the camera, where part Y's
    rule puts it for that anisotropy. Early sign: the capability record, and WP3's Chrome stills from 40 m up. The
    designed answer is more taps along the footprint's long side in the shader, counted against the record. The
    wrong fix is lowering the moments' variance, which makes the far sea shimmer.
31. **The swell's lines do not show before they break**, because a long, low swell is steep enough to see only
    in its last wavelength, after it has turned and closed up. Early sign: WP13's view from above shows one
    ridge at a time and the wind sea elsewhere. The remedy is the scene's and not the transform's: a swell
    short and tall enough to stand above the wind sea while it still bends, on a shelf wide enough to hold
    three ridges, seen from where their gaps can be compared
    ([the approach decision](LOG.md#wp13-approach-decision)). The wrong fixes are a live window, which is fed
    the same swell and draws the same approach, a second train of waves along the shore, and a turn or a
    height the transform does not give.

32. **The query in surf answers too late to use**, or holds the frame while it works. Early sign: WP13's
    paired record, which measured 18.9 s for one tick of eight floats in a gale's breakers; floats that
    fall back to the conformance model whenever they enter the surf; and answers that reach the game later
    than two ticks in the breakers while open water's arrive on time. A batch that waits for an admitted
    view is late whatever it costs, and [the tick stage](#bodies-and-the-query) removes that wait and
    nothing else. A batch is also late behind device work submitted ahead of it, on the one queue wgpu
    has, and no order of submission changes that: WP19's run from the sand measured it behind a 93 ms
    pass, and the remedy is that pass's cost ([risk 33](#risks)). The designed remedy for the cut's cost is
    [the coarse pass](#bodies-and-the-query), shared by a body's points; then a dispatch spread over nodes,
    ceilings taken at a cell's own depth after the depth limit, and component sums on a coarser lattice.
    The wrong fixes are a longer maximum age, a read of the field cache or of the camera's mesh, an answer
    that gives up the highest surface, a cut cell coarse enough to hide a lip, and a landing margin of one
    size, which drops the lip where the sheet bends most.
33. **The surf field cache holds the device across several ticks**, so every view with a beach in it is
    late and every query behind it. Delivery says when it is cheap enough, and no time for the view does
    (WP21.3). Early sign: WP13.12's record, 97 ms for the pass at the beach in a gale,
    which tripped the refresh-interval wire and was left; then WP19's control arm, 4% of open-water answers
    within two ticks from the sand. The designed remedies do each kind of work as often as it changes and
    leave the sheet as it is: the transform's table read once a node, both presentations from one pass over
    the components, and no evaluation of a node that nothing in view reads
    ([WP21](#a-surf-view-a-running-game-can-afford-wp21)). Beyond them, each with the surf judged again by
    eye: narrower or coarser surf rings, a sum on a coarser lattice, a cache made on alternate frames. The
    wrong fixes are a smaller extent or a far camera, which do not shrink a pass counted in nodes, a lower
    rate of views, a reordered queue and a second path for the query past the view.

## Proof and builder handoff

- **The bar.** The owner, 17 November 2026: "Ok let's lower the bar." Thrum is a game engine, and the aim is the
  most plausible result as fast as possible, not physical accuracy ([the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)).
  For every work package not yet Done, water passes on its look, judged against the work package's inspection list at
  the bar of Sea of Thieves and the shipped games each work package names, and on its speed, as cost records.
  What a game reads stays tested: the map, the long waves and the body query against `thrum-water-model`, at a
  tolerance each test states, because a float must sit on the water that is drawn and answer the same on every
  device. So do the contracts: the frame contract's depth, motion and reactive coverage; state through pauses,
  hitches and a replaced device; no wave across dry ground; a capability left out costing nothing; and what one
  part hands another, each tested as a property of the frame and not against a light or wave model: the wake a query
  answers is the wake's state (WP6), a hitch is one step of the elapsed time for wake and foam (WP6, WP7), the
  exposure scale and each medium reach a gathered surface once (WP8, WP15, WP14), and the Sun's visibility
  under water is read at one place by the bed and the shafts, which darken the water over the bed's own
  shadow (WP16). How light,
  foam, wakes, caustics and shafts are computed is not held to an f64 reference, a closed form or a bound
  measured first. The equations in the sections above are the mechanism to build and a builder's diagnostic
  when a look is wrong; a builder may tune their constants by eye and records what changed. WP1, WP2, WP3 and WP12
  stand as they were proved.
- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. `ios-sim` is not a target. Chrome checks need a
  browser launch outside the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
  Never substitute a Mac result.
- **Capture.** Inspected items use T2 engine capture in the mode they name, at 2560×1440 until the showcase's
  extent exists. An item gives the cue, the views and the range. The route, the framing and the length that show
  them are the builder's and the reviewer's to choose; frames are judged whole and at native size, and motion as
  consecutive frames at 1x. The example wires the tap as `examples/v1-game` does, under ruling 3 where the tap or
  the shell assumes the V1 game.
- **Cost records.** Each work package that adds frame work takes [the paired record](#cost-account) again, beside
  the last record for the same view. A tripped wire is investigated and written up there. It never ends a
  session `BLOCKED`.
- **Looks.** Where a work package's inspected items judge a look that is tuned, the foam of WP7, the breaker
  section of WP13 and the net and shafts of WP16 above all, tune it against those items with a fresh opinion before review. After three looks
  that fail the same cue, record them in the log and end `BLOCKED`.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/waters-<work-package>/`, selected evidence in
  `.orchestrate/evidence/WATERS/`.
- **Exact baselines.** Water adds crates and an example beside the existing scenes and moves no retained image.
  No work package runs the exact-image verifier as an item: the scheduled `exact-images` check on main and
  the reference guard in `./dev check` hold that ([the ruling](../ARCHITECTURE.md#proofs-run-once-on-main)).
- **Engine tests hold no game content.** `thrum-water` and `thrum-water-model` are tested on beds, bodies and
  pressure patches the tests make. The island, the craft and the channel are the example's.
- **Broadsides.** A resource, read at the commits in [what Broadsides offers](#what-broadsides-offers) and never
  changed. A builder may copy its code, shaders and assets as they are wherever they implement a mechanism of this
  design, and writes from this design and the published sources wherever they do not: the design decides, and a
  file is never a reason to change a mechanism. Every copied or translated file is listed in
  [the tracked provenance record](../../provenance/WATERS.md) with its source commit and path. The owner's words of 14 November cover
  what Broadsides' authors wrote. Material in Broadsides that came from elsewhere keeps its own licence, which is
  recorded with it, and none that is GPL or share-alike is copied. A copied kernel is held to this module's
  items like any other code. Scheduling, allocation and host glue are written for Thrum's seam.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** A ledger row for `E1.water` is added in WP2 and kept current, with implementation, correctness,
  visuals and device apart.

## Open questions

| Question | Settled by |
|---|---|
| What does the surface pass cost against the refresh interval, with the map and several bodies? | WP2 and WP4 record it; WP11 records it at the showcase extent on both recipes. Tuning is the later performance phase |
| Do the two endpoints of interaction waves and foam serve `EFFECT-GRAPH`'s cadence rules as built? | WP6; a failure is a proposal there |
| What do the gather and the rays cost where they fill the view? | WP8 records it |
| What tolerance does the map meet against the model on Metal and in Chrome? | WP4.1's test states it, small beside what a float or a waterline would show |
| Does a beach read as a beach with a swash and no surf, and its foam as carried by the uprush alone? | WP4 and WP7, for a profile without surf; [risks 12 and 17](#risks) |
| Does the breaker section read as a breaking wave from the sand, from above and from a kilometre off? | WP13; [risk 20](#risks) |
| What does the cut for the highest surface cost, in the model and in a tick with every float in the breakers? | WP12 and WP13 record it. It was 18.9 s a tick in the gale, and WP19 makes the cut coarse to fine and holds it to the query's age contract; the cut cell, the coarse cell, the margin floor and the bend factor are the knobs |
| What tolerance does the transformed field meet against the model on Metal and in Chrome, and what do the lags cost to prepare and to read? | WP12 records preparation; WP13.1's test states the tolerance, small beside a breaking wave's height |
| Can Thrum's renderer draw a second view of the scene, and at what cost? | `SCENE-VIEWS`' planning and its first work package; WP14 records the water's share |
| Does exposure need to know the camera is under water? | Yes: [the meter](#light-under-water-caustics-shafts-and-the-meter), in WP16 |
| What do the caustic field and the shafts cost, and how many rungs does the look need? | WP16 records it; [risk 25](#risks) |
| What adaptation share reads right from the shallows to 20 m? | WP16.7; the example's 0.75 is a starting value |
| How does a river's reach answer a change of its source's stage? | `RIVERS`' planning; here the ends follow and the reach stays as prepared |
| How do basins that overflow into each other merge and part? | `PUDDLES`' planning, with this module |
| Does one mean current a window serve interaction waves in running water? | `RIVERS`' planning, on WP9's channel |
| How many sampled textures does the surface need with the map, against sixteen? | WP4 counts against the record |
| Is an Apple-tile kernel variant a measured win? | WP11 |

## Water fixtures and full sea sweeps (WP17)

For the basin oracle, shrink the bed
around the turning, lag and exposure witnesses. For the surf oracle, keep a genuine overhang and a
multihit column in each of the three geometries, with the independent fine forward reference at the
witnesses. For the water scene and GPU water tests, use a compact beach, basin or chart, a small target
and only the submissions needed to exercise wet and dry samples, tick-page ownership, visible contribution
and foam transport. Put force, shelter and settling logic that needs no GPU in CPU tests. Keep a device
witness for the query answers consumed by the float and the wave, shoreline and film data the device
produces.

Move both portable forward-cache sweeps whole to the long tier. At light and high wind, early and a day
in, they compare every base texel and every filtered mip at production dimensions. Direct base-block
averages expose addressing, edge-block and high/low carry failures across the full hierarchy. Those
portable faults are caught by the scheduled main-branch run after merges. Mark each named long test
with an ignore reason beginning with `long:`. List `thrum-sea::it` once in `.config/long-tests.txt`
(release with overflow checks) with the fault that would reach main before the long run catches it.
Raise the `main-tier` rule version as the settings require. Run these cases in that build on the branch,
then take the covering `long-tests` record on main. Verify that the normal runner skips the moved cases
and the long runner executes each once.

## Compact Metal craft trail (WP18)

The trail test drives the wake scene through 692 submissions at a 60 Hz simulation tick to read five
samples of the submitted film stocks. Its witness is the stock left by the same 6 m/s craft on a real
Metal device after the source is removed, through 60.1 seconds of simulated time. The 32 × 24
target and 50 ms test-only tick on `build/waters` reduced the run to 182 submissions within the
foam catch-up limit and measured 1.827 s alone. Keep the five elapsed checkpoints and the existing
cover, width, peak and final fade bounds.
Keep this guard in the fast tier.

## A surf query a running game can use (WP19)

WP13 merges the cut as [the whole cut](#bodies-and-the-query): right, and 18.9 s a tick with the example's
eight floats in a gale's breakers. WP19 makes it [coarse to fine](#bodies-and-the-query) in
`thrum-water-model` and in the dispatch, and shows the age contract with the game running. It changes what
the query costs and nothing the query answers, the surface draws or the foam reads. Start from the cut WP13
merged. [Part AA](../../../.orchestrate/evidence/WATERS/WP19/query-reach-study.md) and
[part AB](../../../.orchestrate/evidence/WATERS/WP19/coarse-rule-study.md) have the figures and the source
that produced them. The coarse cell, the margin floor and the bend factor are starting values, and WP19.1
holds whatever they become. WP13 keeps the regular-wave run-up and draw-back and the highest-surface query.

- **The model first.** The two passes go into the model's cut, and the whole cut stays in its tests as the
  reference. The reference is the whole cut as a game reads it today: its highest triangle
  [refined](#bodies-and-the-query) on the sheet, or left at its interpolated height where the refinement
  does not converge. The two passes change which nodes are evaluated and nothing after the highest triangle
  is found, so the public answers are compared, and the triangle alone is not the expected height: on the
  shoaling production sheet at (−25.072, −7.435) m it is 0.4436 m and the refined answer 0.4548 m. The
  many-train sheet is the test's own: 64 long components or more over a beach it makes, steep enough to
  throw a crest further than 5 m, since a sea of many trains is where the ceiling and the throw part
  furthest. Seed its columns under lips as well as at random: in part AA one random column in a hundred
  held three surfaces. One lattice evaluated once serves the whole cut of every column, which keeps the
  test in the fast tier.
- **The island's gale** is compared in the water scene's own CPU tests, since the island's bed is the
  scene's. Its columns are the design review's, at 2.0 s and world
  (x, z) = (−142.17685937352712, 8.880178254307927) m, and columns **under a fold**: each at the centre of a
  cut-cell triangle that the throw has turned over, so that it holds three surfaces or more. Take them near
  one another, where one lattice over their reaches is the whole cut of each: about 600,000 nodes, half a
  minute on one core and a few seconds across the Mac's, which keeps this test in the fast tier too. With
  the bend factor at 0 the review's column answers 1.9 m low. If WP13 has changed the gale and that column
  no longer tells the two rules apart, find one that does as part AB's source does and add it.
- **The dispatch.** A body's points share the coarse pass. How nodes are laid out, stored and spread over
  the GPU is the builder's. The tick still has one batch for every posted point, which is what WP5.1's
  count of dispatches guards: the cut's passes are part of that batch, and if they are dispatches of their
  own the count keeps its meaning, one batch a tick whatever the number of bodies. The storage the passes
  add is counted against the capability record, and a profile without surf still creates none of it
  (WP13.2).
- **The running run.** `water_cost.py running --scene surf` takes two arms on the island in a gale: the eight
  floats held in the breakers, and the same floats held in open water beyond every throw reach, which is
  the control. Both arms are drawn from the island's sand camera, where a player in the surf stands and
  the surf's rings lie over the bed, and a station takes its answer as the island's floats do, from what
  the tick's drain hands over (the WP19 review's findings 1 and 2). The floats are held in place by the
  run, so they stay where the cut is made. An answer's lateness is counted in ticks, from the tick that
  posted its point to the tick whose drain first handed it to the game. A use of
  [a held answer](#a-surf-view-a-running-game-can-afford-wp21) is counted apart, as neither a delivery
  nor a fallback. The control shows what the machine delivers with no cut in the batch: draw at an extent at
  which nine of its answers in ten arrive within two ticks under the machine's ordinary load, and say
  which. The breakers arm must then do the same, at the same extent. A frame that outlasts two ticks with
  no water in it is a hitch, and the tenth is for hitches; all 48 stations arriving seven ticks late on
  every tick is not one, though each is inside the maximum age, and the run fails it. The run also fails
  on an answer older than the maximum age, a fallback or a blocking read, in either arm. If the control
  itself delivers fewer than nine in ten, the run measured the machine and not the query and does not
  pass: take it again, at a smaller extent if need be, and do not ask for a quiet machine. The record
  keeps the lateness of both arms, and the breakers arriving later than open water, inside the nine in
  ten, is a tripped wire to investigate.
- **The tick stage.** WP19's first candidate, `3110457e6` on `build/waters`, has the two passes in the model
  and the dispatch and builder passes for WP19.1, WP19.2 and WP19.4. Its running run failed in both arms,
  the control included, because the batch rode the frame
  ([handoff](../../../.orchestrate/evidence/WATERS/WP19-handoff.md)). Once `EFFECT-GRAPH:WP6` is Done the
  batch moves to [the tick stage](#bodies-and-the-query): its inputs, its dispatch, its copy and the start
  of its map, with the one batch a tick that WP5.1 counts. The run's skipped opportunities are then
  advances that reach the device with no view submitted, as a window's are. `advance_without_rendering` is
  the advance of a host that has lost its device, and no batch is submitted in it. The stage removes the
  wait for a view and no device time: an answer is late by what is left of the frames submitted ahead of
  it, plus the batch's own. That record's breakers arm completed a frame every 72 ms against 20 ms in open
  water, so the cut's device time is still to come down, by [risk 32](#risks)'s remedies. The cost record's
  query time is the tick pass's, read under its name from the same timing feed.
- **Carry and state on the stage.** The move changes two things an answer shows, and WP19.5 and WP19.6
  hold them on Metal with the host's views scripted. The source of the cascades is one view older than
  when the batch rode the frame, so [the carry bound](#bodies-and-the-query) is five ticks: an ordinary
  post, four ticks ahead of its source, keeps its cascade support, and a source more than five behind
  loses it. And a post is answered only from [its own world's state](#bodies-and-the-query). WP5.1's
  gale test posts before the first water view, discards that view and reads supported cascades from the
  first view submitted. On the stage both askings come before that view and find no state, so the post
  waits with no batch made, and the test's cascade comparison moves to a post answered after the view has
  seeded the state. The posts and the drains take their endpoint from the fixed update, as `EFFECT-GRAPH`
  WP6 hands it. WP19.6's restore drains on the new endpoint before it posts, as the island does, with a
  former result one tick old whose body is still live, and that drain must hand over nothing: a mailbox
  that drops the former timeline's results only when the new one posts has already moved the float by
  the old wake.
  The running run submits a view before its first measured post, as its first record did, so its count of
  fallbacks starts on a seeded timeline.
- **The cost record** is taken again as WP13.12's was, beside it.
- **After WP21.** WP19's second candidate, `ab950b5b9` on `build/waters`, failed the run from the sand in
  its control arm, and neither the cut nor the stage was the cause. The surf field cache held the device
  for 93 ms of every view, and a tick whose drain brought no result sent a float to the fallback.
  [WP21](#a-surf-view-a-running-game-can-afford-wp21) settles both and proves the control arm by itself.
  WP19 then takes the pair and the cost record again, as its items stand. A control still under nine in
  ten is not read as a view holding the device: a cheap view can sit ahead of a late drain. The run's
  record says [where each late answer waited](#where-a-late-answer-waited), in the queue, behind a pass
  or at the drain, for both arms. A pass of the water's is brought down here as WP21 brought the cache
  down, and a fault in the mailbox's drain is repaired here. WP19 ends `BLOCKED` only where the wait is
  the stage's bound or a pass another module owns, naming it and its counts.

## A surf view a running game can afford (WP21)

WP19's run from the island's sand camera failed in its control arm: 4% of open-water answers arrived
within two ticks, where the same query drawn from 2.65 km offshore delivered every one. No cut was in that
batch. The view was ahead of it. [The paired record](../../../.orchestrate/evidence/WATERS/WP19/review-fix-surf-cost.json)
at 2560 × 1440 has the surf field cache at 93 ms a frame, and WP13.12's had it at 97 ms. A view completed
in 123 ms with surf, 22 ms with surf disabled and 7 ms with no water. At 192 × 108 and at 96 × 54 a view
still completed in 113 ms and 118 ms
([the runs from the sand](../../../.orchestrate/evidence/WATERS/WP19-handoff.md)), because the pass's
work is counted in mesh nodes and not in pixels.

**Whose it is.** `WATERS`'. [The tick stage](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6) gives what it
says it gives: a tick submission goes ahead of its own view and behind whatever was submitted before it, on
the one queue wgpu has on Metal and in WebGPU. A view that holds the device for seven ticks delays every
batch behind it by up to seven, in any order of submission, and a second queue would not stop a dispatch
that is already running. So the stage is not reopened and no second path for the query is built. The
remedy is the device time ahead of the batch, and from the sand 93 ms of every view's is this pass. No
time for a view is the bar. How long a batch can wait behind a view and still be drained within two ticks
depends on when views are submitted and when the game drains: a 20 ms view submitted every other tick
leaves a 0.25 ms batch inside its two, and a cheap view does not save a result published after its drain.
The bar is delivery, which WP21.3 counts. The pass tripped the refresh-interval wire in every surf record from
WP13's first, at 22 ms, to WP19.4's, and each record answered with a sentence the tool writes for every
record in place of [an investigation](#cost-account). Under [the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design)
a confirmed cause is fixed while building goes on, and this one also breaks a contract.

**Where the work is.** The pass runs once for every node of the surface mesh and of the distant shore's
lattice, about 550,000 samples on the island. For a node on the bed it evaluates the forward sheet at this
presentation and at the previous one, again at the contour for a node in the swash, and four times more at
a ring's edge. Each evaluation sums every long component, 64 in the island's gale, and each component
reads its own lag and exposure from the shore transform's textures: 12 texels where the stage is on a
sample and 24 between two, more beside a cell's edge. That is 1,500 to 3,000 texel reads a node and of the
order of a billion a frame. [The cost account](#cost-account) planned "a few reads for each direction and
wavelength in use": the transform in one bed cell at one stage is 8 directions by 6 wavelengths, 56
texels at the cell's four corners, however many components read it. These counts are read from the source and the record's `shore_reads`;
which of them holds the time is to be measured. Three things are done more often than they change:

1. **The table is read for every component and every evaluation.** What the sheet takes from the shore
   transform depends on the bed cell it is read in and on the stage. It does not depend on the time, nor
   on the component beyond its direction and wavelength. So a read is shared by **place, footprint and
   stage**: every component, evaluation and presentation that reads the same bed cell at the same stage
   shares one read of that cell's table. That is not one table a node. The sheet at a node reads more
   than its rest cell, and each of these is a read of its own, made where the sheet makes it today:
   - *the contour.* A node in the swash takes the arriving phase from a sum at the still-water contour,
     which can lie in another bed cell. It reads that cell's table, and its own for the rest;
   - *a cell's edge.* Within a hundredth of a cell of a boundary the wave's direction is taken from the
     lag just across it, so the node reads those lags in the neighbouring cell as well;
   - *a ring's edge.* A node on a ring's outer edge is the blend of the sheet at its two coarser nodes
     (below), each with its own cell, contour and edge reads;
   - *a moved stage.* The previous presentation reads the table at its own stage where the stage has
     moved, and shares this presentation's where it has not;
   - *the distant shore's lattice.* A lattice node in view is one sum of the sheet, for this
     presentation alone, with the reads that sum makes.

   Reads that fall on the same cell and stage, from any of these, may be shared. The rest cell's table
   used in place of the contour's or the neighbour's is not a saving: it changes the arriving wave and
   its direction at the waterline and at a cell's edge, where the swash is drawn.
2. **The two presentations share nothing.** They differ by each component's phase, `ω·Δt`, and by the
   stage while a tide moves. Where the stage has not moved, one pass over the components at a place gives
   both sums there.
3. **Nodes that nothing reads are evaluated.** The cache is read by the surface draw, where a vertex reads
   its own node and four neighbours, and by the distant shore face. A node that no triangle in view uses,
   with the furthest the sheet can move a vertex allowed for, need not be evaluated in that frame. The
   distant shore's lattice is already culled so, to the nodes a pixel in view reads, and is culled no
   further.

**The order.** Measure first. With the pass's table reads switched off, then its components halved, then
its nodes halved, say where the 93 ms is, and take first the remedy the measurement names. The three
above are the designed ones, and each leaves the sheet as it is. How the table is held between its read
and the sum, how nodes are chosen and how the dispatch is laid out are the builder's, inside Chrome's
limits as on Metal.

**What does not change.** The pass writes two products, and each holds what WP13's pass put there.

- **The mesh's nodes.** An interior node holds the model's forward sheet at its rest place, as WP13.1
  holds it. A node on a ring's outer edge holds the blend that joins the ring to the coarser one outside
  it: the model's sheet at the two nearest nodes of twice the ring's spacing, one to either side of it,
  mixed by its place between them, and the sheet itself where it stands on such a node. That is so at
  both presentations. It is not the model's sheet at the node's own rest place. On a 32 m crest of 0.5 m
  amplitude the mean of the heights 4 m to either side lies 0.146 m under the crest, ten times
  WP13.1's tolerance of 0.015 m, which is why WP13.1's guard leaves those nodes out. WP21.1 compares
  them with the blend of the model.
- **The distant shore's lattice.** Beyond the near window a node in view holds the phase and the
  strength of the breaking front at its place, from the same forward sheet, and zero where nothing
  breaks or the place is dry. The surface draws the white front seen from offshore from it, blended in
  between 80 m and 140 m from the camera. This is the surf line WP13 passed at 1 km. An empty or stale
  lattice saves work and moves no mesh node, no depth and no motion, so no check of the mesh shows it,
  and in Chrome the surf case reads mesh nodes alone. WP21.6 holds it by eye on both targets.

The mesh, its rings and their spacing, the lattice, the number of components, the section, the foam and
the query, which reads no cache, stay as WP13 and WP19 left them. So the surf WP13 passed is the surf
drawn. WP13's inspection is not taken again whole. WP21.6 takes the one part of it no check reads: the
front from 1 km and across the foam handoff, beside the draw as it was before the pass changed. WP21.1
holds the sheet in the frames where a saving could show: the first frame after the camera has turned to
nodes that were not being evaluated, after it has moved, after the stage has moved and after the
spectrum has changed. WP13.1's guard keeps its 4,096 points and its tolerance, from a view that holds
them. Drawn bytes that move because sums are made in another order are a deliberate change, refreshed
under [the baseline rules](../ARCHITECTURE.md#exact-baselines).

**What is not a remedy here.** A smaller extent or a camera offshore: the pass does not shrink with the
extent, and the sand is where a player stands. A lower rate of views, a reordered queue or a longer
maximum age: the batch is behind the view however they are arranged. And [risk 33](#risks)'s three that
change the drawn sheet: other rings, a sum on a coarser lattice, a cache made on alternate frames. Those
may come with the performance phase, each with the surf judged again by eye.

<a id="where-a-late-answer-waited"></a>
**Where a late answer waited.** Delivery decides WP21.3, so a run that falls short has to say where its
late answers waited, and the run's record does. For each tick's batch it has the tick of the post, the
tick the batch was submitted or the refusal that held it back, the tick in which the result was
published to the mailbox, and the tick whose drain first handed it over. A late answer waited in one of
three places:

- **the queue**, when its batch was submitted after its post's tick: the host made no asking, or refused
  the batch at its bound on outstanding tick submissions;
- **a pass**, when its batch was submitted on its tick and its result was published after the drain two
  ticks on: device work was ahead of it, and the views' completion times and the cost record's passes
  name whose;
- **the drain**, when its result was published in time and the game took it late, or the mailbox handed
  over another in its place.

A pass of the water's is brought down here, the cache first, and a fault in the mailbox's drain is
repaired here. Where the designed remedies are spent and a pass of the water's still holds answers past
two ticks, write the split in the log and end `BLOCKED`, and a planner adds the inspection a change to
the drawn sheet needs. Where the wait is the stage's bound or a pass another module owns, end `BLOCKED`
naming it, with its counts. A run is taken again at a smaller extent only where the record shows the
extent's passes, and not the water's, ahead of the late answers.

**The held answer.** The WP19 review found a second thing that fails WP19.3 with the view cheap. Its run
with views offered at the full refresh delivered more than nine answers in ten within two ticks, and still
left 528 open-water station uses, 11 ticks in 600, with no result in that tick's drain. The mailbox hands
a result over once, and the island sends a station with none to the conformance model. So one hitch, which
the tenth is for, drops every float for a tick to a surface that is not the one drawn, and no run under
the machine's ordinary load shows none. That is not [the contract](#bodies-and-the-query), under which a
result is unavailable when it is older than the declared maximum and a hitch ages answers. The mailbox
keeps it:

- **Once a tick.** A body is handed one result for a tick, new or held, and no more. The first drain of
  a tick that has no undelivered result it can use for a live body hands the game the newest result
  already delivered for that body again. Each height is carried from its use tick to the drained tick by
  its rate of change, as a late answer's is, once and never on top of an earlier carry, and wet or dry
  is decided again from the carried height. A second drain of the same tick hands nothing for that body
  and changes no count, as the mailbox behaves today: WP5.1's guard drains a tick-3 post at tick 5,
  drains tick 5 again and gets nothing, and it stands as written. The held answer is that result at the
  first drain of tick 6, if nothing newer can be used then.
- **To the maximum age.** When the held result's post is more than the maximum age behind the drained
  tick, the mailbox lets it go and the drain hands nothing for the body. No count of the mailbox's
  changes, because the result was counted when it was first delivered. The float falls back, and the
  game counts that use unavailable as it counts any station without an answer.
- A newer result takes the held one's place when it is first delivered, at its own age.
- **Its own water's.** A held result is its world's, its body's and its device's. A drain on another
  world epoch or timeline hands it to no one, as [the former timeline](#bodies-and-the-query)'s results
  are dropped, and a changed generation ends it. So does the readback reset, which the water's rebuild
  makes when a device is replaced or the profile or the bodies change, and which already drops the
  results published and not yet drained. A rebuild resets the wake to calm, and the answer held from
  before it has the old wake in its height. World, generation and age can all still pass at that
  moment, so none of them is the test. The first drain after the reset hands nothing for the body, and
  the float is on the fallback until a result made after the rebuild is delivered. This is
  [the tick stage's rule](../EFFECT-GRAPH/DESIGN.md#the-tick-stage-wp6) that a result belongs to the
  device generation that recorded it, kept where the game takes the result.
- The age counts take each result at its first delivery, so the share within two ticks still measures
  delivery, and uses of a held result are counted apart.

The game does not change: the island takes what its drain gives it, and a predicted column more than 2 m
from the station still sends that station to the fallback.

**The proofs.** WP21.2 holds the held answer on the CPU, with results the test publishes, since the rule
is the mailbox's own and needs no device, and runs WP5.1's Metal guard beside it, unchanged. WP21.3 is
WP19.3's control arm taken by itself from the sand, `water_cost.py running --scene surf --arm open-water`,
at the extent WP19.3 will draw. It is decided by delivery and by nothing timed: of every station posted,
nine in ten first handed over within two ticks, a station never handed over counting as late, and no
station sent to the conformance model. Its record is the one
[where a late answer waited](#where-a-late-answer-waited) reads. Its corrected driver is on `build/waters`
at `ab950b5b9` and merges with this work package. WP21.4 takes the paired record again at WP13.12's view,
with the cache's time beside WP13.12's and WP19's, the nodes evaluated beside the nodes held, the table
reads of each kind above counted as the pass makes them, and the measurements of **the order** above as
the cache's investigation. No figure in it decides the item: WP21.3 is what the cost has to buy. WP21.5
runs WP13.10's Chrome case on the changed pass. WP21.6 is the distant front by eye. Its reference is the
draw before the pass changed: `fixed` stills from the revision the work package starts from, at WP13.7's
1 km view and its handoff view, taken before the pass is touched, then the same stills from the result,
on Metal and in Chrome. The breakers arm, and the pair, stay WP19's.
