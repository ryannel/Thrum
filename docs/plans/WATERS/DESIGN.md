# Waters: one water system for every body of water

14 November 2026, revised the same day for surf, mirrored scenery and the view from under water.
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

Until 14 November `SEA` planned an open sea and nothing else, and inland water lived in designs no module was planned
from. This design starts again from what a game needs of water, settles one system that every body of water is
data for, and says which module builds each part. It keeps what the sea design of 13 and 14 November settled
wherever that holds for all water, and says where it does not. [Judged from first principles](#judged-from-first-principles)
gives each verdict.

## Purpose and scope

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a sunset over the sea, a lake under hills, a stream into a pond, rings from a dropped stone | Water that is right as well as good-looking, at the display's refresh, with only the parts the demo shows | Mechanisms with an oracle; every part beyond still open water optional, at no cost when left out |
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
ponds and pools. Cues S1 to S7 of the sea brief and W1 to W9 below, on `mac-metal` and `chrome-webgpu`, lit only by
`ATMOSPHERE`'s products. The wave field is `SEA`'s, built by its M1 and M2 and unchanged by this design.

**Planned as later modules**, from [their briefs](../ARCHITECTURE.md#rivers-and-puddles): `RIVERS` (rivers,
streams and waterfalls) and `PUDDLES` (rain, puddles and wet ground).

**Not built by any planned module**, with the reason:

- Water that leaves the surface: spray thrown from a breaking crest, the splash where its lip lands, and a
  barrel a camera can ride inside. The breaker here is one sheet that pitches forward and may overhang. Also
  the currents surf drives: set-up of the mean level, the longshore drift and rips.
- Bulk water that moves while the game runs: a flood, a dam break, a level change that travels downstream. Flow is
  prepared once. A live solver would write the same map, so it can come later without changing a reader.
- Caustics and shafts of light under water, drops on a lens that has surfaced, water above water in one column
  (a pool on a bridge over a river), and water in a vessel.
- The three sea cues the 10 November spikes left open (spray leaving a hull at chase distance, the attached sheet
  at the hull, a rising gas cloud), planing trim from tow-tank data, fleets, ray-traced reflection (a later
  provider for [the reflection slot](#optics) on devices that have it), multiplayer
  water, Kelvin packets for far bodies, a second set of interaction windows, and saving water state.
- Ice and snow, which are not this system's.

The seams the water decision asks for are kept: causes are plain records with stable IDs, forces never depend on
the camera, and no water state would cross a network.

**What exists.** `thrum-sea-model`, the CPU wave model, is built by `SEA` M1. Thrum has no water on the GPU.
Broadsides has a sea whose surface pass takes 21 ms at 1440p, and lake and coast code that was never accepted:
its kernels are a resource, and its frame is not.

## Cues

S1 to S7 are the sea brief's, and each is something a user above needs. W1 to W6 are what water as a whole adds,
and W7 to W9 are the owner's of 14 November. The bar for the sea is the sea and wakes of Sea of Thieves; for
inland water and for mirrored scenery it is the lakes, shores and streams of Red Dead Redemption 2; for surf it
is the beaches of Horizon Forbidden West; for the view from under water it is Sea of Thieves again. All in
words, at the display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **S1** Open sea from the waterline to the horizon: wind sea over swell, no repeating tile, crests that sharpen with wind | Glassy calm to gale presets; camera 1–40 m above the water and a 1–20 km vista; noon, low Sun and twilight | One tiled FFT patch. Rejected by a 40 m view of a kilometre of sea, and by the long waves matching the CPU model at points a patch length apart |
| **S2** Bodies float: a small float and one hull rise, pitch and roll with the waves they sit on | One craft and eight floats; rest to 15 m/s; the GPU field read through one batched asynchronous query, one to two ticks late, never synchronously | Scheduled bobbing, or a CPU copy of the waves. Rejected by a float riding a wake crest that only the GPU holds, and by a count of blocking reads that stays zero |
| **S3** A wake that is water: it spreads behind a moving hull, persists after the hull stops or turns, and lifts a float it crosses | Displacement speeds to planing; turn, coast and restart; wedge angle and crest travel compared with theory at two speeds | A wake decal, or a constant-speed wave equation. Rejected by the transverse wavelength at two speeds and by a crest that keeps travelling after the source is removed |
| **S4** Foam that is material: born where waves break and where the hull meets water, carried by the water, thinning and vanishing with age | No foam on a calm sea or at rest; whitecaps scaling from fresh breeze to gale; a trail that outlives its source; no stamped or tiled pattern | Foam drawn from the present wave shape. Rejected by a trail that stays where the water carried it after the hull has gone, and by two views a tile length apart |
| **S5** Sea optics: the sky and Sun reflect, colour deepens with depth, the Sun glitters, cloud shade dims it once | G3s sky from dawn to night; screen-space reflection with the environment product as fallback; `CLOUDS` shade when present | A private sky or exposure, or a frame that only looks right at one exposure. Rejected by a seam test against terrain-lit geometry at the waterline at four times of day, and by a body's reflection and submerged part held to the optics equation at two exposures |
| **S6** Stable in motion: no shimmer, ghosting or history smear under reconstruction; cuts and rebuilds reset cleanly | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract | Zero motion on a moving surface. Rejected by motion read from submitted frames against the surface's known displacement |
| **S7** Controls a visitor can use: wind, swell, time of day and a field overlay, on Mac and in Chrome | The typed controls of `CREATOR` | A control that restarts the sea. Rejected by a wind change in motion with no pop |
| **W1** Water lies where the land holds it: a level surface that ends in a clean line where the ground rises | A sea, a lake above it and a pond in one view; levels raised and lowered while the scene runs; from the shore and from 40 m up | A water plane drawn through the terrain, or a sheet joining two levels. Rejected by a hollow beyond a ridge that must stay dry, and by a top-down view classified from the CPU model |
| **W2** Shelter and shallows: a lake is calm while the sea runs, waves fade as water shallows, and on a beach the waterline runs up and back with them | Glassy calm to gale; fetches of 200 m to open sea; depths from 10 m to the waterline; a beach of 1 in 10 at a fixed stage | One wave height for all water, or a waterline pinned to the still level. Rejected by the lake's measured height against its fetch in a gale, by a wave that reaches the beach at full height, and by the waterline's travel under one regular wave |
| **W3** The ground shows through the shallows and the shore reflects in still water | Depths from the waterline to where the water type hides the bed; calm to breeze; the far shore, hills and a tree on the bank on screen | An opaque sheet with a painted edge, or a sky-only mirror. Rejected by a bed of known radiance under known depths, and by a reflection that must join its source at the waterline |
| **W4** Rings on any water: a dropped stone or a hull raises waves that travel at the speed the depth gives and die at the shore | Depths of 0.25 m to deep; rings and wakes, short waves and long, in one window that holds shallows and deep water; a shore and a bar of dry ground | A deep-water solver everywhere, or one transform over land and water alike. Rejected by a wave's speed across the range of depths, and by the water beyond a bar of ground, which must stay at rest |
| **W5** Running water: where the map has a current, ripples, foam and floats travel with it | Currents to 3 m/s; a channel that narrows; a hand-made chart here, a solved one in `RIVERS` | A texture sliding over still water. Rejected by a float that must be carried, by foam born at a post standing in the current, and by tracked surface detail |
| **W6** One water: the same light, foam, query and controls on every body, and no seam where two meet | A channel leaving a lake and entering the sea; a swell running; the lake's level and the tide moved through their ranges; a mouth shallow enough for the depth to hold the swell; one craft crossing from one body to another | A renderer for each kind, or a blend drawn over two surfaces that do not meet. Rejected by the surface's height either side of the join, after its depth limit, under a swell and after a stage has moved, and by one query answering across it |
| **W7** Surf: a swell meets a shore as surf. Its lines turn to face the beach, close up and rise; the largest of a set break furthest out; a crest steepens, pitches forward and breaks into a white front that runs in to the swash | Trade wind to gale with a swell of 6 s to 14 s; beaches of 1 in 30 to 1 in 5; swell square on and 60° off the beach; a headland and a lee shore; the tide through its range; from the sand at 2 m, from 40 m up and from 1 km off | A wave train drawn along the shore, apart from the sea's own waves, or a wave that only fades. Rejected by one swell crest followed from deep water to the sand, its spacing and height against wave theory at three depths, and by a float that must rise on the crest the viewer sees break |
| **W8** Scenery the camera cannot see: water mirrors what stands round it, the far shore, hills, trees and the craft, each joined to its foot at the waterline and blurring as the water roughens | Calm to a breeze on the lake and the pond, calm to a trade wind on the sea under the island; camera 1 m to 40 m up, looking down so the source is above the frame and panning so it leaves the side; two levels in one view; dawn to night | A trace of what is on screen, or a probe at the camera. Rejected by a view whose reflected hill is wholly off screen, and by a tree's reflection that must meet its trunk at the waterline, which a probe's parallax misses |
| **W9** Under water: the waterline crosses the lens; below it the water has its colour and murk and darkens with depth, the bed and hulls show through it, and the surface overhead shows the sky through a bright window with the deep mirrored round it | Every body; the lens from 0.1 m to 20 m under; calm to gale; half in, crossing both ways, and in the swash; looking up, level and down; dawn to night | A tint over the whole frame once the camera's centre is under. Rejected by a lens half under, where each pixel is judged alone, by the window's edge at 48.6° from the surface's normal, and by a panel of known radiance seen through known lengths of water |

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
- **Broadsides**, as what was tried. Its accepted calm and trade-wind sea (W1), its wake and query milestones M1
  and M2, and the foam spike's five changes with blind verdicts are evidence that mechanisms of this kind make
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

Reproductions A to G were run for the sea design and are kept with their output in
[the sea's evidence folder](../../evidence/SEA/design-proof/). Parts H to M were run for this design and are in
[this module's](../../evidence/WATERS/design-proof/): H the interaction step at depth, I shelter, J dry ground,
K joins, L the swash and M foam in the swash. Parts N to Q were run for the revision and are in
`surf_view_proof.py` there: N the shore transform, O breaking, P the mirror lookup and Q light under water. Two design reviews of 14 November are kept in
[the evidence folder](../../evidence/WATERS/), each with its own reproduction. After the first, H was run again
and J to L added. The second found a step at a shallow mouth and no carriage for foam up a beach: K now
assembles the surface after its depth limit and reproduces that review's case, and M was added.
[Check that it can work](#check-that-it-can-work) reports them.

### Judged from first principles

Every choice this design inherited, from the sea design, the water decision or the older water designs, judged
against the users and cues above. "Keep" means it would be chosen again with the earlier work out of the room.

| Inherited choice | Verdict | Why |
|---|---|---|
| A module for the open sea, with inland water left to designs nobody builds | **Change** | The sea is the one body with no bed, no shore, no current and one level. A surface, optics, foam and query built for it alone assume all four, and the rest of water is where each fails. One system is designed here, and the sea is its first body |
| The wave model and its GPU fields (`SEA` M1 and M2) | **Keep**, as built | Wind waves are one field for the world, with their own oracle. Every body reads them, and nothing in them depends on where water lies |
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
| One sea, all of it or none; then capabilities a scene leaves out | **Keep** the capabilities | A sunset demo and a puddle need neither wake, foam nor query |
| Rigid bodies integrated by the game | **Keep** | Thrum has no physics owner, and the engine must hold no hull |
| Whitewater, optics and dynamics as separate providers with packets, charts, ray-traced regional optics and a work ledger (the older appearance, optics and integration designs) | **Not selected** | They specify a larger product than any cue here asks for, none was run, and their costs were never counted against a frame. Their rejection cases are kept as items |

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each milestone judges fit file by file. Paths are in the Broadsides repository at
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
Neither stops the milestones after it.

### Surf, mirrored scenery and the lens, decided against the cues

| Cue | Candidate | Verdict |
|---|---|---|
| W7 | Waves that only fade under the depth limit | Fails: nothing turns, closes up, rises or breaks |
| W7 | A wave train drawn along the shore's distance field | Fails: it is a second sea. The crest that breaks is not the swell's, a float cannot be told which to ride, and its sets are not the wind's |
| W7 | The local depth's wavenumber and Snell's turn applied at each point (OloEngine) | Fails: phase is not an integral, so crests drift and tear |
| W7 | Baked simulations placed along drawn fronts (Horizon Forbidden West) | The look to beat. Not selected: the fronts are authored for each beach, apart from the sea state, the tide and the query, and the bake needs an offline fluid pipeline this project does not have. Its one-section breaker is taken |
| W7 | A live Boussinesq or shallow-water window fed by the wave field | Does shoal, break and run up. Not selected: a second solver whose state must agree with the analytic field at its border, a time step bound to the cell size through every hitch, no far surf outside its window, and the control problems Crest reports. It remains the route if the selected one cannot meet W7 |
| W7 | **Each long wave carries a phase lag solved over the bed, and a crest takes a breaker section by its breaking index** | **Selected.** The swell's own crests turn, close up, rise and break; a CPU evaluates it exactly; it costs texture reads in the frame; and the depth limit, the swash and their proofs stand |
| W8 | A screen-space trace of the scene (this design's first version) | Fails by construction when the source is off screen. Kept for water that has no mirror view |
| W8 | A cube probe drawn round the camera, with or without parallax correction | Fails the waterline: a probe is right at one point, so a reflection parts from its foot by the probe's distance from the surface point. Six faces cost more than one mirror |
| W8 | Ray query against the scene's geometry | Right in principle. Not portable: experimental on Metal in wgpu and absent from WebGPU. A later provider for the Apple recipe, promoted by a measured win |
| W8 | **A mirror view: the scene drawn again from the camera's mirror image in the water's plane, read with its depth** | **Selected.** Exact on flat water for everything above the plane, on screen or off; what Unreal and GTA V ship; one more view, whose cost its owner controls |
| W9 | A tint and fog once the camera's centre is under | Fails a lens half under |
| W9 | **A per-pixel lens test against the surface's own function, the surface drawn two-sided, and the water composed as a medium from the gather** | **Selected** |

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
  arrival from the bed's edge, where the lag is zero. Ground above the shore margin is closed to it, so a
  wave reaches the lee of an island round its sides and never through it. The depth is floored at the
  profile's lag floor, 0.5 m in the first profile, so the lag stays finite across the swash. Over a border at
  the bed's edge, an eighth of its width, the depth the solve sees rises smoothly to deep water, so a wave
  meets the bed with no step in its length.
- *What is prepared.* The wave lattice has eight directions. For each, `thrum-water-model` solves the lag on
  the bed's grid at wavelengths a factor of two apart from 20 m to 640 m, and at stages a set spacing apart
  across the body's declared range, 0.5 m in the first profile. A component reads its own direction's lag,
  blended linearly in the logarithm of its wavelength and in the stage. Components longer than 640 m are not
  transformed. The lags are content, as the map is: they change only when the bed or the declared bodies do.
- *Exposure.* A component stands for a 45° sector of directions. Its exposure at a point is the share of
  that sector's directions along which a straight line upwind reaches the bed's edge without crossing ground
  that stands above the body's highest stage, and the component's energy is scaled by it. So a windward
  beach takes a train whole, a lee beach takes none of it and laps with the short waves alone, and far
  downwind of an island the shadow closes as the sector opens.
- *Height.* `K = Ks(|k₀| d) · √exposure`, where `Ks = √(Cg₀/Cg)` is linear theory's shoaling coefficient at
  the point's depth, floored as the lag's is. The depth limit of [the last section](#waves-on-a-body-gains)
  then holds the long layer's significant height, which is now the transformed components' at the point,
  exactly as before. The swash depth is still taken from the row's gains alone, before any transform.

For an 80 m wave on a plane beach of 1 in 10, the solve on 2 m cells is within 0.34 rad of Snell's closed
form at 0°, 30° and 60° of incidence (0.69 rad on 4 m cells: the scheme is first order), and its crests in
2 m of water lie 10.9° and 19.1° off the shore's normal against Snell's 11.1° and 19.5°. Blending wavelengths
a factor of two apart misstates the local wavenumber by at most 2.0%; a factor of four would misstate it by
7.6%. Blending stages 0.5 m apart misstates it by at most 2.6% in a metre of water or more and by up to 7.9%
between the floor and a metre; a spacing of 1 m would give 11.5% and 13.8%. Round an island, a wave from the
west meets the windward shore square on, the windward quarter 26° off the shore's normal, the side 45° and
the lee quarter 57°, where the untransformed wave runs 45°, 90° and 135° off and, in the lee, straight out
of the land. Its exposure at those places is 1, 1, 1, 0.47 and, in the lee, 0. Wherever the exposure is above
zero the transformed wave runs at most 60° off the shore's normal in 4 m of water, 39° in 2 m and 28° in
1 m (part N).

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
- past the profile's bore index there is no throw, and the crest and trough stand at the plain wave's
  heights: what is left is a bore with a steep front;
- every departure fades to nothing between half a swash depth of still water and the depth the waterline
  draws back to. In the swash the surface is therefore the limit's remainder as first built, and the swash's
  closed form holds with surf on: on part L's beach the waterline's highest and lowest reach are the same
  with the transform as without it (part O).

The first profile's table is made from a closed form, a phase skew for the face and a throw in proportion to
height, and is tuned against the inspected items of M13.

**Readers.** The transform and the section are one function, in `thrum-water-model` in f64 and in WGSL, read
by the field cache, the query, the foam's births and the conformance model. On the GPU the lags are one array
texture and the exposures another. Where a crest overhangs, a column holds more than one surface: the query
answers the highest, and the surface is drawn as the displaced sheet it is. Inside a chart's join the
transform is the joined body's, taken at the join's weight, so it follows from the row as the swash depth
does and the surface still meets across an end.

**Declared approximations.** Linear ray theory up to the break: no set-up of the mean level, no longshore or
rip current, no reflection from a steep shore and no diffraction, so energy neither gathers on a headland nor
leaks into a lee. The first arrival leaves a crease where two fronts meet behind an island, in water the
exposure has already calmed. A bore travels at the linear speed of the floored depth. In less than a metre of
water the wavenumber is off by up to 8%. Nothing leaves the sheet as spray.

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
[state table](../EFFECT-GRAPH/DESIGN.md#state-time-and-the-tentative-frame-m2). The seam runs a producer once in
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
the answer will be used at, and the body's ID and generation. One compute dispatch answers them all from the water's own state, never from the field cache or the camera:

- the map: the body of water at the point, its level, bed, depth and current;
- long waves, evaluated analytically at each point for the tick it will be used at, at the body's gain, through
  the shore transform and the breaker section where the profile has surf;
- cascades, read from the frame's evaluation at the mip that matches the point's footprint, so a 6 m hull does
  not shake on ripples, at the body's gains;
- the wake, read from its windows, with **the body's own static dent added back** (below).

Crests sharpen because displacement is horizontal as well as vertical, so the surface above a world point is
found by fixed-point iteration on the horizontal displacement; the oracle iterates to convergence. Under a
breaking crest that overhangs, the answer is the highest surface in the column.

Each answer is the body of water at the point with its depth and current, the height, the slope, the surface's
velocity (the current and the waves' together) and the time derivative of height, with one support flag per
layer, and it is **valid at the use tick** its point asked for. A point that is not wet now says it is dry: ground beyond the water, and the swash between waves, where the depth it reports is negative. The dispatch brings every layer to that tick
before it sums them: the long waves are evaluated there, the height of the cascades is carried there from the
frame's `t` by its rate of change, and the wake's from its state tick. The answer carries the use tick and the
times it was made from. A game that uses an answer at another tick moves the whole answer once, by its time
derivative from the use tick, so no layer is extrapolated twice. For a wave of frequency `ω` carried `Δt` the
remaining error is at most `½(ωΔt)²` of its amplitude, reported per profile. The cascades are carried at most
three ticks: 0.4% for a 20 m wave and 3.8% for a 2 m wave, whose height is centimetres (part G).

Results come back through a ring of mapped buffers one to two ticks later. A result older than the declared
maximum is unavailable, and the game falls back to the conformance model in `thrum-water-model` (the map, and the long waves at the body's gain:
on a lake, level water with its current)
and counts it. A hitch ages answers the same way. No path waits on the GPU: a blocking map or poll is a defect,
and a counter proves there is none.

Inputs travel with the tick page: the game puts its body records, impulses, stages and wind in its page, and the scene hands them to the water in `upload`. Results return through a mailbox the game drains at the start of a tick.
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
  lose little power and make little foam, in that proportion, and a calm makes none. With surf, the long
  layer's share of that birth is laid where a crest spends the power: along the front face of a crest past
  index 1, by the section's phase, with a part of it as submerged air under the lip. The total is unchanged.

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
whitecaps continue to the vista without state. A shore's foam continues the same way: beyond the far level it
is the cover the limit's power would hold in a steady state, so a distant beach keeps its surf line.

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
moves 3 m up a beach of 1 in 10 in three waves and keeps all its foam, in steps of 1 tick and of 8. With the
drift of the current and the wind alone it does not move (part M). Declared approximations: the carriage runs
across the lines of equal thickness, so waves that arrive at an angle carry no foam along the shore; the
backwash carries none back; on a step of several ticks the sheet's drift is taken once, at the step's middle;
on the far level, whose 2 m cells are as wide as a swash, the carriage is not resolved and foam lies where it
was born; and seaward of the draw-back line nothing carries foam shoreward. A bore's white front travels
because foam is born along it, and what it leaves behind stays until the swash takes it.

**Appearance.** Coverage thresholds a ranked pattern, so thin foam is lace and thick foam closes. The pattern is
sampled by stochastic patches, the change that removed the stamped repeat. The pattern is a cooked 2D product
the game supplies through a `CONTENT-LOOP` lease.

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

- *Refracted:* `L_behind = L_body · exp(−c·z)`, with `z` the hit's depth below the surface. The forward pass lit
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
  `(T_v · L_body + S_v) · E` with `(T_v, S_v)` the air from the mirrored camera, with its depth and its
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
- **Units.** The mirrored camera's line of sight to a body is the reflected path unfolded, so the image
  already carries the air of the whole path. The surface pass takes off the part the optics equation applies
  itself: `L_reflected = max(colour / E − S_cs, 0) / T_cs`, where `(T_cs, S_cs)` is the lent aerial lookup
  from the camera to the surface point. What remains is the body's radiance with the air between the body and
  the water on it, which the screen-space rays leave out.
- **Roughness.** The image carries mips of premultiplied radiance and coverage, and a pixel reads the level
  its slope covariance and its ray's length cover, as it reads the environment product. A reflection
  therefore softens and stretches as the wind rises, and at its edge it blends onto the sky's. Past the
  coarsest level the scenery's share fades into the environment.
- **With the other readers.** A registered body is geometry and is in the image, so water with a view casts
  no screen-space ray for reflection. The rays through the water are unchanged.

In a vertical section of a lake 60 m across under a bank, a tree and a hill, from cameras 2 m and 10 m up,
the lookup on flat water returns the scenery point the true ray meets. Under ripples of 2 cm and waves of
0.1 m and 0.3 m, the plain lookup with no correction is off by a median of 11% to 13% of the ray's length and
takes sky for scenery or scenery for sky on 8% to 23% of rays. With two corrections the median is 0.02% to
0.09%, the 95th percentile at most 8.4%, and at most 1.3% of rays are confused, all of them at the skyline
or the tree's top (part P).

Declared approximations: radiance toward the mirrored camera stands in for radiance toward the water, which
is exact for a matte surface. A level further than the plane tolerance from every plane has no view. A
surface point off the plane sees scenery from a viewpoint that is off by twice its height, so the hidden side
of a near object can be wrong by that parallax. Whatever the view leaves out, other water among it, is not
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
  afterwards, so no pixel takes the water twice.
- **The frame contract.** The underside writes depth, motion and reactive coverage as the upper side does,
  so a cloud march still stops at the water. Crossing the surface is not a reset: each pixel changes side by
  itself. The meniscus writes reactive coverage.

Declared approximations: no caustics and no shafts of light, so the bed is lit evenly through the surface.
The exposure scale is the sky's, so deep water is dark and stays dark. Sunlight's path down is taken
vertically. Under a crest that overhangs, the highest surface decides, so inside the hollow of a pitching
wave the lens reads as under water. Anything transparent that a scene draws after the water takes none of
the water's murk.

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
  | Surf | The shore transform's lags and exposures, and the breaker section | Waves; a bed |
  | See-through water | The gather where water is thin, and the scene through it | |
  | Scene reflection | The gather and the march where water is mirror-still | |
  | Body optics | The gather and the rays for registered bodies | Body records |
  | Scene mirror | Mirror views and their lookup | `SCENE-VIEWS` |
  | Underwater view | The lens test, the underside of the surface and the water between | See-through water |

  Open water alone is a still pond under the sky. With waves it is the sea of a sunset demo.
- **One physical profile on every recipe.** The map and its joins, the gains, the swash ratio and margin height, component count, cascade layout and loop, the shore transform's wavelengths, stage spacing and floor, the breaker ratio and the breaker section, the interaction levels, their depth ladders and their count of solves, and query
  semantics are the profile's, never a quality setting (C04). Chrome and the Mac float the same body on the same
  sea.
- **Limits are inputs.** Transform length, workgroup memory, storage formats and counts, and sampled textures per
  stage are read from the capability record and refused at preparation when exceeded. Levels and endpoints are
  array layers so the surface pass stays inside sixteen sampled textures.
- **Recipes.** Both recipes run the portable kernels first. An Apple-tile variant (f16 working precision, a
  different transform layout) is promoted only by a paired measured win with the oracle still inside tolerance.
- **Knobs.** Each is a named runtime value in `EFFECT-GRAPH`'s list with a declared invalidation: surface mesh
  density; field-cache reach; the cadence of the field cache and of cascades below the profile's smallest force
  wavelength; foam cadence and level size; reflection steps, reach, slope allowance and mirror limit; the count of mirror views and their extent; the gather's resolution and the sight depth. A layer evaluated less
  often than every frame is carried between evaluations by its rate of change, with its slopes held. None changes
  what a query returns.

### What the later modules build on

Nothing in this section is built here beyond the seam it names. Mirror views and the view from under water
are the shared system's, so a river and a puddle have them with no work of their own; surf needs long waves
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
  floats, and joins a lake and the sea under a swell and through their stages (M9). What it leaves to `RIVERS`:
  how a river's reach answers its source's stage, which here stays as prepared.
- **`PUDDLES`: rain, puddles and wet ground.** A puddle is a basin whose stage rain raises and drying lowers, in
  hollows found from the bed. Rain rings are one more short wave layer. Wet ground is not a body: it is a wetness
  the ground's material reads, written by rain and by water that has left, which needs a ruling with the forward
  pass's owner. On a shore the waves wash, this module draws the sheen and the foam as its film, and the map
  gives each body's wave support, so the ground's material can darken the same sand and the two agree on one
  support (C07). What this module proves for it: a basin of any size whose stage moves (M4, M10), rings on water
  0.25 m deep (M6), and the ground seen through thin water (M8). What it leaves to `PUDDLES`, as a question its
  planner settles here with this module: two basins may not overlap, and a stage that would merge them is
  refused, so how puddles that overflow into each other join and part again, keeping each body's identity,
  stage and the rings and foam on it, with no rebuild or reset a viewer could see.

### Check that it can work

- **S1, S2 and the oracle, and time for the wave field.** In [the sea's design](../SEA/DESIGN.md#check-that-it-can-work).
- **W1.** A level over a bed, cut by the scene's depth, is how Crest and Unreal place lakes; the hollow round a
  seed is a flood fill with a closed form on simple beds, which M1 tests. Unrun on the GPU.
- **W2.** Part I gives the gains above, and part L the waterline's travel under the limit with a swash depth,
  against its closed form, with the pinned waterline of the limit alone beside it. That a lake reads right with
  the short layers alone, and a beach with a swash and no shoaling, is unrun on the GPU; M4 is the first look,
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
  section in f64 with no births, ageing or spreading; how the carried foam looks is unrun, and M7 is the first
  look ([risk 17](#risks)).
- **S5.** Every light term reads a product `ATMOSPHERE` M5 lends in physical units, so a seam with terrain can
  only come from the sea's own shading. The one input that is not physical, the copied scene colour, is turned
  back with the lookup and scale that wrote it: in part E the converted gather gives the true stored colour,
  1.142, where the copy used as radiance gives 0.354 and the copy with only exposure divided out 1.001. The
  receiving rectangles are tested in part C against rays reflected and refracted on the CPU, for cameras 1 to
  40 m up, bodies 12 to 100 m off, and seas from flat to 3 m of amplitude with a 20° allowance: in 36 cases, no
  ray that met the body came from outside its receiving rectangle, and in every case some came from outside the
  body's own rectangle, 92 to 2,010 of those sampled. The design review's case, a 2 m body 20 m from a camera 2 m up, is among them.
- **S6.** Unrun. `EFFECT-GRAPH` M3 proves motion and reactive reach reconstruction; how the sea looks after it is
  first seen in M10.
- **Hitches.** Parts A, B and D, reported under
  [time](#time-content-functions-and-state), [interaction waves](#interaction-waves) and
  [whitewater](#whitewater).
- **W3.** Unrun. Regional copies and screen-space reflection for water are what Unreal, HDRP and Far Cry 5 ship;
  the units are the sea design's part E. M8 measures the copy and the march where they fill the view.
- **W5.** Unrun. Two cross-faded samples carried along a flow are Portal 2's and HDRP's; the pulse they can show
  is [risk 13](#risks).
- **W7.** Part N: the lag from an eikonal solve against Snell's closed form on a plane beach, the blends
  between wavelengths and between stages, and an island, where the transformed wave meets every shore from
  the sea and the exposure calms the lee. Part O: the break point against Komar and Gaughan, sets, a thrown
  section that overhangs without crossing itself, and the swash's closed form with the transform on. All are
  f64 arithmetic on sections and small grids. That the section reads as a breaking wave and not as a
  marching ridge is unrun; M13 is the first look ([risk 20](#risks)).
- **W8.** Part P: the mirror lookup against exact reflected rays in a vertical section. A second draw of the
  scene is what Unreal and GTA V ship for water. `SCENE-VIEWS`' own design owes the proof that Thrum's
  renderer can draw one; nothing here runs it.
- **W9.** Part Q: Snell's window and the Fresnel split from beneath, and the scattered light's closed form
  against a numerical integral. The lens test reads the function the surface is drawn from, so the line on
  the lens and the drawn surface cannot part. The look is unrun; M15 is the first.
- **Cost.** Unrun. M2 is the first measurement.

## The system as it runs

| State | Owner |
|---|---|
| Spectrum, components, cascades and their f64 evaluators; the wave field's GPU passes | `SEA`: `thrum-sea-model`, `thrum-sea`. After `SEA`'s last milestone is Done they are this module's to extend, with their tests kept passing |
| The map and its function, joins, shore margins and what preparation refuses; gains and the swash depth; the shore transform's lags and exposures and the breaker section; the labels of a window's regions; the f64 surface model, the interaction and foam references, the conformance model | `thrum-water-model` (this module), no GPU dependency |
| The map on the GPU, the surface from both sides, optics, the mirror lookup, the view from under water, interaction waves, foam, the query ring | `thrum-water` (this module), one effect on `EFFECT-GRAPH`'s seam, which declares the wave field's passes with its own |
| Mirror views: the second draw of the scene, its culling, light, air and targets | `SCENE-VIEWS` |
| Body declarations, the bed, presets, water types, the craft and floats, hull footprints, foam pattern, body integration, controls | The example scene `examples/water` (package `water-scene`); never in engine crates or their tests |
| Stages, state commit, cadence, knob list | `EFFECT-GRAPH` |
| Sky, Sun, aerial, reflection product, cloud factor | `ATMOSPHERE` |
| Motion and reactive targets, history, reconstruction, the governor | `APPLE-PRESENTATION` |
| The foam pattern's residency | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's body records, query points, impulses, stages and wind, and the
long-wave phases and loop time for the frame's `t`. `simulate`, every frame: the cascades and the field cache at
`t`. `simulate`, only on a frame that advances the tick: the interaction step with its sources, region by region; foam births and
transport; the query dispatch and its copy to the ring. Forward, sky and stars draw, and `SCENE-VIEWS` draws
the mirror views the water asked for. `after-opaque`: the gather, where a reader needs it; the composition of
pixels under water; the surface, body by body. Reconstruction, tone and present follow. On submission the
water commits its tick and each endpoint's tick. A scene without a capability runs none of its steps.

**The example scene** is a `Game` on `thrum-app`'s shell with a ticking world of its own, as `examples/v1-game`
is. It uses `thrum-controls`, a parameter file and `./dev scene examples/water [--web]`. Its world is an island
the example makes: open sea, a beach open to the swell, a headland and a lee shore, a lake in the hills under
a ridge with a tree on its bank, a pond, and a channel from the lake to the sea whose chart is made by hand and
labelled a stand-in. Its ground, craft, floats and foam pattern are the example's
content: made here, or copied from Broadsides where [that serves](#what-broadsides-offers).

**Creator journey.** A scene adds the water contribution with a profile that names its capabilities, and declares
its bodies, each with its stage's range: a level for a sea, a seed and a level for a lake, a chart with its ends. A scene with ground gives the bed, or the triangles
to build it from. A scene that floats things puts body records in its tick page and reads answers from the
mailbox. Changing wind, swell or a stage needs no rebuild. Changing the profile or the bodies rebuilds the water
and resets interaction waves and foam.

## What WATERS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-waters), built by this module's builder after
`EFFECT-GRAPH:M3` is Done, with `EFFECT-GRAPH`'s items unchanged and its tests kept passing:

1. An `after-opaque` pass may write scene depth (M2).
2. An effect's own pass may read scene colour and scene depth as textures and write only the effect's targets
   (M8).
3. Whatever the shell, the page, the scene packaging or the capture tap assume of the V1 game or of the sky
   kit's still world is made a property of the game (M2 on the Mac, M3 in Chrome).

4. A mirror view of the scene, drawn by [`SCENE-VIEWS`](../ARCHITECTURE.md#scene-views), a new module with its
   own design, and not by this module's builder (M14). The interface is
   [above](#mirror-views-scenery-the-camera-cannot-see): what water asks for, and what the view holds. The
   rest of the design does not wait for it: M14 is the only milestone that needs it.

`SEA` is asked for nothing. The shore transform extends `thrum-sea`'s field cache and `thrum-sea-model`
after `SEA`'s last milestone, when both crates are this module's, with `SEA`'s tests kept passing.

If `EFFECT-GRAPH` M3 as built already allows 1 or 2, nothing is added. Everything else is used as built: the
lease of one linear 2D texture, the lent environment products and cloud factor, the temporal contribution, the
typed controls and parameter files. The map's textures are this module's own resources. Nothing is asked for the
bed: the game gives it.

## Cost account

Estimates, unmeasured, at a 60 Hz tick. Broadsides' comparators are measured there, not here. The wave field's
rows are in [the sea's design](../SEA/DESIGN.md#cost-account).

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| The map | Prepared; read by every water vertex, pixel and query | Level, bed and current on each body's grid: a 1 km square at 1 m cells is about 16 MiB | One more read set in the surface pass | |
| The shore transform | Prepared; read by the field cache and the query | For each still body with long waves: 8 directions × 6 wavelengths × its stage samples, on the bed's grid, and 8 exposures. A 1 km bed at 4 m cells with a 2 m tide range is 240 layers of 250², about 30 MiB at 16 bits. Solving them is CPU work at preparation, which M12 records | In the frame, for a cache texel over a bed, a few reads for each direction and wavelength in use where there were none, and the section's table; M13 records it | |
| Interaction waves | Tick | 2 × 256². For each region, one complex transform forward and one back for each ladder depth in use: two transforms a level over deep open water, up to 16 solves a level at the profile's bound. Labelling a window's regions is CPU work on the tick, when a window moves | Under 0.2 ms over deep water; about 0.7 ms at a shore with six depths in use; more at the bound, which M6 records with the count of regions and solves | About 3.8 ms a tick in its lake, as an interval |
| Foam | Tick | 2 × 512² × 4 stocks. The swash's drift reads the surface's rate and slope at cells on a shore, where births already read the field | Under 0.2 ms | About 2.2 ms for first deposition |
| Query | Tick | Up to 2,048 points, 48 B each | Under 0.1 ms; 96 KiB back | 0.03 ms |
| Gather | Frame | The rectangles its readers need: nothing on an open sea with no body, the screen where see-through or mirror-still water fills it | Proportional to the pixels copied; a whole-screen copy is one pass ended on a tile GPU | A whole-scene copy and a 3 ms trace |
| Body and scene rays | Frame, inside the surface pass | Water pixels the mask marks: two ray-against-box tests for each marked body; a bounded march where a ray meets a box or the water is mirror-still | Unmeasured; M8 records it on a calm sea and in a gale with zero, one and nine bodies, and at the lake | |
| Mirror views | Frame, by `SCENE-VIEWS` | Up to two views of the scene's opaque geometry, each a quarter of the main view's pixels | The scene's own draw again for each view, less what its owner cuts. Unreal reports a planar reflection at up to half a frame at full detail, and GTA V ships one at 240×120. M14 records it with no view, one and two | |
| The mirror lookup | Frame, inside the surface pass | Water pixels of a body with a view: three reads of the image and its depth | Less than the screen-space march it replaces there | |
| Under water | Frame, only while a pixel of the lens is under | The lens test for each pixel, one composition for each pixel under water, and a whole-screen gather | Of the order of the surface pass's own pixels; M15 records it half under and wholly under | |
| Surface | Frame | Water pixels at the render extent, about 200 k vertices for the sea and tiles for each other body in view; one read set for the map and each wave layer, two for interaction waves and foam | The largest term; M2 measures it | 10.5 ms at 1440p after the copy |
| Memory | | Interaction, foam, the query ring, the map | About 20 MiB and the map, and the gather target, whose layout is the builder's: addressed by screen position at 2560×1440 it is about 44 MiB | |

Interaction, foam and query work lands on alternate frames at 120 Hz, so both frame kinds are recorded.

**Records and tripwires, never gates** ([brief](../BRIEF.md#goal-and-exit-rule), decision 3). `scripts/water_cost.py`,
created in M2, takes a paired record of one scene at one view, capture off: the scene with the water and with it
absent, on tick frames and on repeat frames, giving the whole frame, each water pass's GPU time by name, the bytes
of each resource and the load on the machine. A record trips a wire when the whole frame is over the display's
refresh interval, when a pass is about 25% or more over its last comparable record, or when a cost grows faster
than the work that was added. A tripped wire is investigated in the milestone that tripped it, and the record
says what was measured to find the cause and what was fixed or left as a follow-up in the log. Visible stutter is
judged in the `realtime` inspections.

The ambition is the display's full refresh, and whole-frame tuning is the later performance phase. **No timing
figure decides whether an item passes, whether a milestone is Done or whether the next one starts.**
`water_cost.py check` measures the scene again and fails only when the scene cannot be measured, when the
committed record lacks part of what a paired record holds, or when a wire the record trips has no investigation
beside it. Presented cadence on the built-in panel is measured once, attended, in M11.

## Risks

1. **The surface pass costs too much.** Early sign: M2's record, and M4's with the map's reads and several
   bodies. The plausible wrong fix is dropping layers from the pixel shader. The designed ones: fewer texture
   reads through packing, cascades fading to moments sooner, and a coarser mesh. The milestone tries the ones its
   record points at and records what each did. Building goes on either way.
2. **A 160 m repeat shows from 40 m up.** Early sign: M2's vista capture. Fallback: a larger first patch under a
   profile the portable limits still admit, designed in `SEA` first.
3. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M3, the first real Chrome run. The
   map adds float textures and the surface more reads; both are counted against the capability record in M4.
4. **Self-interaction through the query.** A body bouncing on its own wake. Early sign: M6's craft at rest must
   settle. The dent correction is the design's answer; the wrong fix is filtering the craft out of the wake.
5. **Foam reads as grit or cutouts**, the spike's open defects. Early sign: M7's first captures. The mechanism is
   settled; the look is tuned against the inspection list, and three failed looks on one cue go to the log.
6. **The water writes depth and a later effect disagrees.** `CLOUDS` is planned after this; the ruling fixes the
   order.
7. **The example's shell glue is larger than the water.** Early sign: M2. A ticking entry in the sky kit is a
   proposal to `CREATOR` in the log, not built here.
8. **Rays cost too much where they fill the screen**: receiving rectangles in a gale, or a mirror-still lake.
   Early sign: M8's record. The designed remedies are a shorter reach, fewer steps and a lower mirror limit as
   slope variance rises, since rough water blurs a reflection away, and marching at a lower resolution. The wrong
   fixes are a march over every water pixel at every roughness, and a second render of the scene.
9. **The wave field costs more every frame than the reads it saves.** `SEA`'s risk; its cadence knobs are the
   remedy.
10. **A whole-screen gather costs a pass on a tile GPU wherever shallow water fills the view.** Early sign: M8's
    record at the lake. The designed remedies are a gather at a lower resolution with the full-resolution depth
    kept, and a sight depth that shrinks the see-through region. The wrong fix is dropping the bed from view.
11. **A reflection whose source is off screen falls to the sky**, and reads as a hole in a lake under a hill.
    Early sign: M8's pan, which is built before the mirror view. The fade at the screen's edge and the reach
    are the remedies there; the mirror view of M14 is the real one.
12. **A beach without surf reads as a pond's edge, or its swash as a tide.** Early sign: M4's and M7's captures,
    which are built before surf. The designed remedies there are the swash ratio, which sets how far the
    waterline travels, the foam born through the band, and the film's sheen. The wrong fixes are a waterline
    animated apart from the waves, which the query would not know, and foam painted along the shore. Surf, in
    M13, is the real one.
13. **The carried detail pulses** at the carriage's period. Early sign: M9. The designed remedies are a period
    varied from place to place and a longer one where the current is slow. The wrong fix is a single scrolled
    sample, which smears.
14. **A later module finds a seam wrong.** Early sign: `RIVERS`' or `PUDDLES`' planning. M9 proves the chart on
    a stand-in so that the seam is tried before a solver exists.
15. **Regions and depths multiply the interaction step.** Early sign: M6's record at the lake's shore, with its
    count of solves. The designed remedies are the profile's count of solves and, on the far level, a coarser
    ladder with its measured envelope stated. The wrong fix is one transform over land and water.
16. **The film fights the ground or floats over it.** Early sign: M7's captures at grazing angles. The designed
    remedy is its depth bias and a bed built as [the map's](#the-water-map) is, from the highest ground in each
    cell. The wrong fix is drawing foam into the ground's material here, which is `PUDDLES`' seam.
17. **The swash gathers its foam into one hard line**, or strips the sand behind it bare. Early sign: M7's
    captures of the beach. The designed remedies are the stocks' own spreading, births under the whole sheet,
    and the drift's bound. The wrong fixes are a pattern scrolled up the beach, and foam drawn at the waterline
    from the present wave shape.

18. **A breaking crest tears or stair-steps**, because the section stretches the sheet past what the mesh
    resolves. Early sign: M13's captures from 40 m and from 1 km. The designed remedies are refinement along
    the band where the breaking index passes 1 and the throw's fade with the mesh's spacing. The wrong fix is taking
    the throw out everywhere.
19. **The lag shows**: a step in the swell where the bed begins, or a jump as the tide crosses a sampled
    stage. Early sign: M13's tide sequence. The designed remedies are a wider border at the bed's edge and
    closer stage samples. The wrong fix is holding the lag at one stage.
20. **Surf reads as ridges marching ashore and not as waves breaking.** Early sign: M13's first captures. The
    mechanism gives the place, the timing and the set; the look is the section's table, tuned against the
    inspected items, and three failed looks on one cue go to the log. If the table cannot reach it, a section
    baked from a simulation is the same data, and a live window is the route named under
    [the candidates](#surf-mirrored-scenery-and-the-lens-decided-against-the-cues). The wrong fix is a second
    wave system drawn along the shore.
21. **A mirror view costs a second scene.** Early sign: M14's record. The designed remedies are its extent,
    a shorter view distance and coarser detail, which are `SCENE-VIEWS`' to offer, and one view in place of
    two. Ray query on the Apple recipe is the later one. The wrong fix is dropping the lake back to the
    screen-space trace.
22. **Reflections swim or ghost under reconstruction**, because the water's motion is its surface's and not
    its reflection's. Early sign: M14's pan. The designed remedy is reactive coverage where a mirrored
    reflection is most of a pixel. The wrong fix is taking the water out of reconstruction.
23. **The line on the lens flickers, or a gap shows where the near plane cuts the surface.** Early sign:
    M15's crossing. The designed remedies are the meniscus's width and a lens test at each pixel's own point.
    The wrong fix is a camera that is kept from touching the surface.
24. **Deep water is black**, because the exposure is metered from the sky. Early sign: M15's views at depth.
    It is declared; a meter that knows the camera is under water is a proposal to `ATMOSPHERE` in the log.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. `ios-sim` is not a target. Chrome checks need a
  browser launch outside the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
  Never substitute a Mac result.
- **Capture.** Inspected items use T2 engine capture in the mode they name, at 2560×1440 until the showcase's
  extent exists. An item gives the cue, the views and the range. The route, the framing and the length that show
  them are the builder's and the reviewer's to choose; frames are judged whole and at native size, and motion as
  consecutive frames at 1x. The example wires the tap as `examples/v1-game` does, under ruling 3 where the tap or
  the shell assumes the V1 game.
- **Cost records.** Each milestone that adds frame work takes [the paired record](#cost-account) again, beside
  the last record for the same view. A tripped wire is investigated and written up there. It never ends a
  session `BLOCKED`.
- **Looks.** Where a milestone's inspected items judge a look that is tuned, the foam of M7 and the breaker
  section of M13 above all, tune it against those items with a fresh opinion before review. After three looks
  that fail the same cue, record them in the log and end `BLOCKED`.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/waters-<milestone>/`, selected evidence in
  `docs/evidence/WATERS/`.
- **Exact baselines.** Water adds crates and an example beside the existing scenes and moves no retained image.
  Every milestone carries the verifier as its claim-time check ([rules](../ARCHITECTURE.md#exact-baselines)).
- **Engine tests hold no game content.** `thrum-water` and `thrum-water-model` are tested on beds, bodies and
  pressure patches the tests make. The island, the craft and the channel are the example's.
- **Broadsides.** A resource, read at the commits in [what Broadsides offers](#what-broadsides-offers) and never
  changed. A builder may copy its code, shaders and assets as they are wherever they implement a mechanism of this
  design, and writes from this design and the published sources wherever they do not: the design decides, and a
  file is never a reason to change a mechanism. Every copied or translated file is listed in
  `docs/evidence/WATERS/PROVENANCE.md` with its source commit and path. The owner's words of 14 November cover
  what Broadsides' authors wrote. Material in Broadsides that came from elsewhere keeps its own licence, which is
  recorded with it, and none that is GPL or share-alike is copied. A copied kernel is held to this module's
  oracles and items like any other code. Scheduling, allocation and host glue are written for Thrum's seam.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** A ledger row for `E1.water` is added in M2 and kept current, with implementation, correctness,
  visuals and device apart.

## Open questions

| Question | Settled by |
|---|---|
| What does the surface pass cost against the refresh interval, with the map and several bodies? | M2 and M4 record it; M11 records it at the showcase extent on both recipes. Tuning is the later performance phase |
| Do the two endpoints of interaction waves and foam serve `EFFECT-GRAPH`'s cadence rules as built? | M6; a failure is a proposal there |
| What do the gather and the rays cost where they fill the view? | M8 records it |
| What tolerance do the map, the interaction waves and the foam meet against their f64 references on Metal and in Chrome? | M4, M6 and M7 measure first, then assert the recorded bound |
| Does a beach read as a beach with a swash and no surf, and its foam as carried by the uprush alone? | M4 and M7, for a profile without surf; [risks 12 and 17](#risks) |
| Does the breaker section read as a breaking wave from the sand, from above and from a kilometre off? | M13; [risk 20](#risks) |
| What tolerance does the transformed field meet against the model on Metal and in Chrome, and what do the lags cost to prepare and to read? | M12 records preparation; M13 measures first, then asserts the recorded bound |
| Can Thrum's renderer draw a second view of the scene, and at what cost? | `SCENE-VIEWS`' planning and its first milestone; M14 records the water's share |
| Does exposure need to know the camera is under water? | M15; [risk 24](#risks) |
| How does a river's reach answer a change of its source's stage? | `RIVERS`' planning; here the ends follow and the reach stays as prepared |
| How do basins that overflow into each other merge and part? | `PUDDLES`' planning, with this module |
| Does one mean current a window serve interaction waves in running water? | `RIVERS`' planning, on M9's channel |
| How many sampled textures does the surface need with the map, against sixteen? | M4 counts against the record |
| Is an Apple-tile kernel variant a measured win? | M11 |
