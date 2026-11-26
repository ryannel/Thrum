# Clouds: the weather, from the cloud tops to the mist on the ground, that shades and lights the world

13 November 2026, revised 14 November, judged again from first principles the same day, and revised after
[its second review](../../../.orchestrate/evidence/CLOUDS/design-review.md). [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md). C02, C04, C06, C07, C08, C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[ENVIRONMENT](../../design/modules/ENVIRONMENT.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.
[The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) records what this design selects and how
Broadsides may be reused. This design replaces the implementation target in
[CLOUDS.md](../../design/modules/CLOUDS.md); that page's ownership and transport rules stay in force and are
restated where they bind. Revised again on 14 November for the atmosphere as a whole:
[mist, fog and low cloud](#condensate-that-meets-the-ground-mist-fog-and-low-cloud) are this module's, and
[what that replaces](#judged-again-for-the-whole-atmosphere) is listed. Revised once more that day after the
review of that revision: [a stop at every pixel's depth and the air in parts](#the-visible-layer), the frames
of a ground row and the headless query, and
the yardstick its proofs quote. Revised 16 November 2026:
[drift is taken at the display phase](#time-drift-at-the-displays-time-evolution-on-the-tick), which
`EFFECT-GRAPH` WP4 provides. Revised again that day after that revision's review (finding MF1;
`git show f1076615a:docs/evidence/CLOUDS/design-review.md`): past the committed tick the travel is carried a
tick at a time by the step's own arithmetic, and the phase describes the target the clock consumes.
Revised 17 November 2026 after [WP3's handoff](../../../.orchestrate/evidence/CLOUDS/WP3-handoff.md): the Sun term of
[cloud light](#cloud-light) is a beam and a diffuse solution, in place of four fixed orders. Revised that day
for [the owner's bar](#how-the-module-is-proved): the look and the speed decide, references are diagnostics, and
near and far fog [share one Sun term](#condensate-that-meets-the-ground-mist-fog-and-low-cloud). Revised
18 November 2026 after [the review of 17 November](../../../.orchestrate/evidence/CLOUDS/design-review.md) (finding MF1):
[one boundary rule for every consumer](#cloud-light), what each integrator reads, and part XVII. Revised
that day again after WP3 stopped on repeated domes and shelves: a cell's column is
[a cluster of turrets](#one-weather-field-in-the-air-frame) and the shape noise is
[a billow field](#one-density-in-m¹), in place of one dome with a fretted skin (part XVIII). Revised that
day once more after [that revision's review](../../../.orchestrate/evidence/CLOUDS/design-review.md) (two findings): every
level of [the weather map](#the-weather-map) holds the query's own column at its texel's centre, and one
clearing factor takes a cell's coverage to zero with its thickness. Revised 18 November 2026 on the lead's
decision to park the form of a single cloud: WP3 closes on the sky at a glance and WP17 judges each cloud, from
the diagnosis in [risk 2](#risks). No mechanism and no interface changed. Revised that day after the review
of that parking (two findings; `git show 061432e11:docs/evidence/CLOUDS/design-review.md`): WP17 judges the
form in [a sky of weather alone](#risks), with no library shape in it, on both targets from both heights, and
the flaws are excused only until WP17 starts. Revised 25 November 2026 after
[WP7's diagnosis](LOG.md#wp7-diagnosis):
[holding the display's refresh](#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20) has its own
account, protocol and work packages. No mechanism of the sky and no interface changed. Revised that day
after [the review of that revision](../../../.orchestrate/evidence/CLOUDS/design-review.md) (five findings):
a lost refresh is counted from the OS-reported presented times and the window's account only places it; a
loss is attributed by the whole frame against the clouds-absent runs beside it; what those runs lose stops
nothing; the account's memory is bounded; and a frame's step is judged by its display time. Revised again
that day after [the second review](../../../.orchestrate/evidence/CLOUDS/design-review.md) (one finding): a
cloud run holds by how many refreshes it loses and by how long it stalls at once, so equal totals cannot pass
a freeze at the wind turn or give it to the window. Revised 26 November 2026 on the lead's decision to park
the weak noon relief on the broad lobes of placed clouds: WP10 closes on the light it retained, and WP21 takes
the relief up from the diagnosis in [risk 14](#risks). Revised again that day after
[the review of that parking](../../../.orchestrate/evidence/CLOUDS/design-review.md) (four findings): WP21
comes last, after WP12, and nothing waits for it; only the congestus core and the mediocris twin turrets are
excused, at noon; WP21's cost is recorded under the noon Sun, before and after; and its relief is judged by
the cues of the bar, with no starting still to beat. No mechanism and no interface changed.

## The request

The lead's planning task of 13 November 2026:

> Plan the CLOUDS module (docs/plans/CLOUDS/BRIEF.md). The owner, asked whether SEA and CLOUDS planning could start
> now rather than waiting on their port timing, said (13 Nov 2026): "On 2 use the fable planner like normal but you
> can bring that across when it makes sense." Lead's reading, to record as an assumption: planning starts now, and
> the owner leaves to this plan when the Broadsides Rust work is brought across, within
> docs/design/decisions/SEA-AND-CLOUD-PORT.md (module by module, no wholesale import, Broadsides untouched).
> Work packages that depend on unfinished modules wait on them as usual.

The owner, 14 November 2026, through the lead:

> You can fully use everything in broadsides. But we should make sure that it aligns with the approach we want.
> But yes, you can copy that code out and reuse as is if it's useful.

> We can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
> design.

> So in broadsides you will find both a procedural cloud setup as well as pre-baked clouds. The idea being that we
> could support both those two sky types as well as a hybrid as needed by the game being built.

> That's the approach we were going with in broadsides that might not be the right approach here. The agent should
> look at that work from first principles and figure out how to design the best possible atmospheric system for the
> thrum game engine. Maybe that's a flexible set of cloud capabilities built from what we did in broadsides, maybe
> it isn't, but it should start from first principles.

The owner, 14 November 2026, later the same day:

> Oh and for clouds we also need to be taking into account mist / fog. We want a plan that's able to deliver on
> top quality atmosphere.

The lead's task with it: "Revise the plans so Thrum's atmosphere reaches the quality of shipped high-end games as
a whole: sky, clouds, haze, and mist and fog (ground fog, valley mist, fog banks over water, low cloud the camera
can enter), lit consistently and changing with the weather and time of day. Settle from first principles which
module owns what across CLOUDS, ATMOSPHERE and any other affected module, keep one authority for each kind of
state".

The lead's planning task of 16 November 2026, after WP2's builder ended `BLOCKED needs_dependency`:

> CLOUDS WP2 needs something no plan provides yet. The builder wrote: The planner must settle and assign the shared
> display-time interface change before WP2.7's uneven 60× shade motion can be completed.

The owner, 17 November 2026, through the lead:

> Ok let's lower the bar.

The lead's task with it: Thrum is a game engine; the aim is the most plausible result as fast as possible, not
physical accuracy. Rendering passes when it looks as good as shipped high-end games against an agent-written
inspection list and runs fast. Physical or f64 references stop being pass/fail gates and stay as diagnostic tools.
[How the module is proved](#how-the-module-is-proved) applies it.

The lead's reading, recorded as [assumptions](PLAN.md#open-for-the-human): the owner's last words govern; the
design of 13 November, [the brief](BRIEF.md) of 10 November and the port decision describe the Broadsides
approach and are inputs to judge, not constraints; Broadsides code, shaders and assets may be copied and reused
as is wherever this design finds them useful; Broadsides itself stays unmodified.

So this design starts from who uses a sky in Thrum and what each needs, chooses its mechanisms against that, and
only then says what Broadsides has that serves them. [Judged from first principles](#judged-from-first-principles)
gives the verdict on every choice the design of 13 November inherited: six changed, and the rest are kept for
reasons that do not rest on Broadsides having built them. [What Broadsides offers](#what-broadsides-offers) says
what may be copied.

It is not a blind study. Read for it: the design of 13 November and [its review](../../../.orchestrate/evidence/CLOUDS/design-review-1.md);
the Broadsides procedural notes at `b4d8250e6` (`THRUM-PORT`, `RUNTIME-DESIGN`, `COST-2026-11-06`, `COST-MODEL`,
`ASSET-CONTRACT`, `PROCEDURAL-COMPLETION`, `bw_core/src/weather.rs` with its data file, and for this revision
`bw_core/src/medium/scatter.rs`, `bw_render_clouds/src/phase.rs`, `cloud_lighting.ron` and the list of its cloud
shaders); the baked path at `79637f789` (the library's README, licence record and `B3-HANDOFF`, the growth proof
and its motion review, the first sections of `PIPELINE-DESIGN`); and in this repository the designs of
`EFFECT-GRAPH`, `ATMOSPHERE` (environment products), `CONTENT-LOOP` (leases) and `WATERS`, with
`atmosphere/cloud.rs`, `cloud.wgsl` and the diagnostic provider in `examples/v1-game`.

## Purpose and scope

A reusable engine sky. Thrum's first outlet is teaching demos on the owner's website, then games
([the brief](../VISION.md#purpose)), so the sky has four kinds of user, and each sets a requirement:

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a sunset, how weather changes a landscape's light, later how a cloud forms | A sky that looks right, composed for the shot, under a visitor's sliders, with only the parts the demo shows | Mechanisms that follow their real inputs; light from the droplets' own optics; a cloud the demo can place; every part beyond the weather optional, at no cost when left out |
| The showcase on this Mac | Skies at the bar of shipped games, smooth at the display's refresh | The work a ray does bounded by design; drift shown at the display's time |
| A game | Weather it can read with no graphics device, the same on every device, and shade on a world that may show no sky at all | One deterministic field with a CPU query; one weather profile on every recipe; shade as a capability by itself |
| `WATERS` and `ATMOSPHERE` | Sunlight dimmed once, and clouds in ambient light and reflections | No private sky, exposure or reconstruction; one optical depth |

A game adds the cloud contribution to a scene with the [capabilities](#capabilities-shadow-only-and-headless) it
wants, keeps a weather state in its world, and gets clouds, their shade on everything the Sun lights, and their
light in the world's ambient and reflections. Consumers built now are the V1 valley and the sea scene (both behind
a selection) and `WATERS`.

**Commissioned:** cues K1 to K11 on `mac-metal` and `chrome-webgpu`, lit only by `ATMOSPHERE`'s products. K8 to
K11, added on 14 November, are mist, fog and low cloud.

**Not built**, from the brief: the ten cloud genera, cumulonimbus with anvil and rain shafts, the warm-front
sequence and high ice layers; two-layer maritime decks from large-eddy simulation; rain, lightning, moonlit cloud
and flying through cloud; a cloud-formation teaching demo. Also not built: a camera that flies (it stays 2 to
40 m above the ground, which may itself stand in cloud or above it), lamps and other local lights glowing in fog,
fog indoors or in a volume a game places by hand, precipitation fields, saving the weather state to disk, a cloud
whose density a simulation writes each tick, and a cloud held over a place against the wind. The cloud-formation demo would be the first of those two: a [directed shape](#placed-volumes) whose slab an
effect writes. The slab, its footprint and its place in the march are designed for it; the writer is not built.

**What exists.** Thrum has no cloud renderer and no fog, and its haze is a constant of `ATMOSPHERE`'s air. `ATMOSPHERE` WP5 applies one Sun-path optical depth on every
direct-Sun receiver, fed today by a CPU-made diagnostic field in `examples/v1-game`. Broadsides has both paths:
the procedural one at a median 289 ms a frame for its fair preset, and the baked one paused on its atlas reader
([brief](BRIEF.md#where-it-is-finished)). Their maths, kernels and content are a resource; the way their frame is
assembled is not.

## Cues

The cues and ranges are the brief's. Judged again, each is something one of the users above needs, so none is
dropped: K1, K2, K4 and K7 are the demo's sky, K3 is what a game and the sea need of it, and K6 is the showcase's.
K5 is restated as what the engine does (a cloud made as content stands in the sky, where the weather or the game
puts it) with the accepted library as the content that proves it; the brief had it as the port of that library.
K4's range gains the display's refresh. The bar for every inspected cue is the skies of Red Dead Redemption 2 and
Horizon Forbidden West, in words, at the display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **K1** A weather-driven sky: scattered fair cumulus with shared bases and varied sizes, a broken deck, and overcast, each recognisable and continuous to the horizon | Clear, fair, broken and overcast presets and the transitions between them; ground and deck cameras 2–40 m up; the full look arc | A tiled cloud texture or a sky dome. Rejected by parallax between near and far clouds as the camera moves 40 m, and by a point query that agrees with what is drawn overhead |
| **K2** Light that reads as cloud: bright sunlit sides grading into shadowed bases, a silver lining toward the Sun, and sunset and twilight colour taken from the sky | Noon, low Sun, backlit, sunset through twilight to night, on the one G3s Sun and sky | A lit albedo with a gradient. Rejected by one cloud seen from its Sun side and its far side, and by a tall cloud whose top is lit after its base has gone dark |
| **K3** Shade that belongs to the clouds: patches of sunlight and shade cross terrain, objects and water in step with the clouds above | One optical depth applied once per receiver; also with visible clouds off (shadow-only) | A scrolling shadow texture. Rejected by a shade edge that lies under its cloud's edge along the Sun's ray at two Sun elevations, and by identical shade with visible clouds off |
| **K4** Clouds that move and change: they drift with the wind in world space and form, grow and thin over time | 0–40 m/s wind; real time and a time-lapse control, both smooth at the display's refresh; no popping, sliding texture or periodic slots | Noise scrolled through a fixed mask. Rejected by tracking one cloud for its whole life in a time-lapse: it is born, rises, and thins away, and nothing reappears where it stood on a beat |
| **K5** Shapes made as content: a cloud cooked from a density volume stands in the same sky under the same light, where the weather or the game puts it, and one grows through staged keys | A sky of weather alone, of shapes alone and of both; the example's library, which is the twelve accepted volumes and the congestus growth track, cooked as content products; placement by the weather field and by the game | A separate pass composited over the sky. Rejected by a placed cloud partly behind a procedural one with correct occlusion both ways, and by ground shade that has the placed cloud's outline |
| **K6** Stable in motion: no shimmer, crawl or ghosting under reconstruction as the camera turns and time runs | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract | Heavy history that hides noise while the camera is still. Rejected by a fast pan, a cut and a 60× time-lapse judged frame by frame |
| **K7** Controls a visitor can use: coverage, cloud type, wind, time of day and an internal-field overlay, on Mac and in Chrome | The typed controls of `CREATOR` | A control that rebuilds the sky. Rejected by a coverage sweep in motion with no pop |
| **K8** Mist and fog on the ground: mist lies in the hollows under a soft, nearly level top that ridges and props stand out of; a bank of fog stands on water and drifts ashore; from inside, the world fades with distance | Visual range 30 m to 10 km; depth 2 to 150 m; seen from inside, across its top and from 40 m above; the valley and the water scene | A fog that thickens with height below a plane, or with distance alone. Rejected by a hollow that fills while the slope beside it at the same height stays clear, by a bank's edge that moves against the land behind it as the camera moves, and by a point query that agrees with what is drawn |
| **K9** Low cloud the camera can enter: a deck hides the hilltops, and a camera climbing a hill passes its base, stands inside it and comes out above it | A layered type's base from above the highest ground down to the ground; the camera below, inside and above, always 2 to 40 m over the ground | A fade to white by the camera's height. Rejected by a ridge that enters the deck along the deck's own base, and by shade and query that agree with it. Heaped cloud and placed shapes that meet the ground run the same code and are not inspected |
| **K10** One light: mist glows toward a low Sun and greys away from it; land inside fog loses its sunlight and its shadows by the fog that stands between it and the Sun; shafts of sunlight stand in haze and mist below ridges and broken cloud; the shadows of ridges and clouds lie across far mist; ambient light and reflections follow; at night it is dark | Noon, low Sun toward and away, dawn, twilight and night, on the one G3s Sun and sky | A fog colour. Rejected by one bank seen toward the Sun and away from it, and by an object's shadow that weakens with the depth of fog over it |
| **K11** Mist that follows the weather and the day: it forms after dusk on a clear, calm night, lies deepest at dawn, thins from its shallow edges after sunrise and is gone; cloud cover or wind prevents it; haze runs from crisp air to murk | A day at 1× to 60×; clear-air visual range 3 to 300 km; the transitions between | A whole layer faded in and out. Rejected by tracking one hollow from dusk to mid-morning: its edges go first, and under an overcast night none forms |

Finished also means, from the brief: the weather field is a deterministic function of seed, preset, integrated
wind travel and simulation time, with a CPU evaluator that answers point queries headless and a test that the GPU
map matches it; all consumers share extinction in m⁻¹; aerial perspective is applied once; cloud and fog segments
are ordered and disjoint; visible clouds off releases presentation resources and keeps shade; every cadence is a
governor knob with a declared history invalidation; a capability a scene leaves out costs it nothing; and the cost account holds at full refresh on this Mac at the
showcase extent, with texture memory recorded, by paired records and tripwires rather than a budget gate.

K6 and K7 hold over K8 to K11 as well: fog is stable as the camera moves through it, and a visitor sets mist,
moisture and haze. For K8 to K11 the bar is the valley mists and fog banks of Red Dead Redemption 2 and the low
cloud of Horizon Forbidden West.

Two ranges the brief leaves unnumbered are fixed here. **Time-lapse** runs from 1× to 60×, the rates the
Broadsides motion work judged at: an hour of weather in a minute. **"Continuous to the horizon"** means to the
geometric horizon of the cloud tops, about 200 km for a 3 km top seen from the ground.

## Research and selection

What each source was read for, and how far.

- **Nubis Evolved** ([SIGGRAPH 2022 slides](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-NubisEvolved-NoVideos.pdf),
  slide text read whole; the deck has no speaker notes). The sky is 2D fields (coverage, type, minimum and maximum
  height) turned into 3D by a vertical profile and eroded by noise: a "dimensional profile" that costs three
  texture reads and exits early in empty air. The "Scaling PS4 & PS5" slide gives the shipped settings: cloud
  target at most 960×540 and 1920×1080, 60–90 and 96–180 view samples, 6 and 10 light samples, in at most 4 ms
  and 2–3 ms. Cone-step and sphere tracing took one scene from 4.2 to 1.3 ms.
- **Nubis³** ([SIGGRAPH 2023 slides](https://advances.realtimerendering.com/s2023/Nubis%20Cubed%20(Advances%202023).pdf),
  slide text read whole). Voxel clouds store the dimensional profile, a detail type and a density scale in a
  512×512×64 grid and add detail from a 128³ noise at render time, because dense sampling cost over 30 ms.
  **Light comes from a 256×256×32 grid of density summed toward the Sun**, refreshed over 8 frames for 0.1–0.2
  ms, which cut render time by 40% and gave "long distance shadows". With a distance field and steps that grow
  with distance, cloud cost fell from 10 ms to 2.2–4 ms at 960×540.
- **How shipped clouds get their form, read on 18 November** after three looks in WP3 all read as domes.
  Nubis ([2017 slides](https://www.slideshare.net/guerrillagames/nubis-realtime-volumetric-cloudscapes-in-a-nutshell),
  slide text and notes as the page gives them) builds a cloud as
  `remap(noise · height gradient, coverage, 1, 0, 1)`. Its noise is a low-frequency "Perlin-Worley": Perlin
  noise given the round cells of inverted Worley noise, with finer Worley noise eroding the edge. Nubis
  Evolved's "Modeling Density" slides, read again for this, give
  `dimensional_profile = vertical_profile * cloud_coverage` and
  `cloud_density = saturate(cloud_noise_composite - (1.0 - dimensional_profile))`, the composite a four-channel
  128³ volume, billowy where a cloud rises and wispy where it thins. In both, the 2D fields say where cloud may
  stand and how high, with soft edges and irregular outlines, painted or generated. The lobes a viewer reads as
  the cloud are the noise's, a few to a cloud. This design kept that operator and lost what it works on: its
  column was one smooth dome a cell, and its noise a skin 200 m long on a cloud several kilometres wide.
  [Part XVIII](#check-that-it-can-work) measures the difference.
- **Frostbite** ([Hillaire 2016 course notes](https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/s2016_pbs_frostbite_sky_clouds.pdf),
  sections 5.6 to 5.10 read). Equation 17 integrates a step's scattering against its own extinction, which
  conserves energy at any step length. Equations 19–20 approximate multiple scattering as octaves with scaled
  extinction and phase. Section 5.9.1 applies aerial perspective once, at a depth averaged by transmittance
  (equation 21). Section 5.9.2 dims the air's in-scatter by the cloud layer's mean transmittance (equation 22).
  Its clouds cost 1.6 ms at 1080p on an Xbox One.
- **Unreal** ([volumetric cloud documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-cloud-component-in-unreal-engine),
  read whole). Traces at quarter resolution, reconstructs at half and upsamples to full; offers cascaded "Beer
  shadow maps" in place of marched shadows, advised for consoles; captures clouds into the sky light over nine
  time-sliced frames.
- **The retained references** in [`scripts/reference/cloud/`](../../../scripts/reference/cloud/optical_reference.py):
  Beer segments and their composition, and a four-order liquid phase model (`WEIGHTS`, `BRANCH_B`, `LIQUID_G`).
  They are the octave model above with fixed numbers. They are this design's optical oracle for Beer segments
  and their composition, and no longer for light: the four orders' weights sum to one, so they return less than
  single scattering alone, and WP3's photon transport measured what that costs (below).
- **Multiple scattering, looked at again on 17 November** after WP3 found no bright side at any density, Sun
  grid or shadow setting. WP3's builder traced photons through a level layer that scatters without absorbing
  ([its record](../../../.orchestrate/evidence/CLOUDS/WP3-handoff.md)): at an optical depth of 64 under a 45° Sun the layer
  reflects 84% of the Sun's flux and the four orders 5%. Shipped practice is Wrenninge's octaves (Frostbite's
  equations 19–20; Unreal's
  [volumetric cloud material](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-cloud-component-in-unreal-engine),
  whose page says the octaves "can result in some energy being lost" and leaves the brightness to the artist).
  At Unreal's defaults they reflect 15% in the same layer and do not change with its depth (part XII). Decima's
  clouds (Nubis) add terms an artist tunes by depth and height. Searches for a published real-time method from
  2023 to 2026 that conserves energy in cloud found none: the newer work is path tracing and learned encodings,
  which do not run in a browser's limits. What does conserve it is the atmospheric sciences' own method for
  exactly this medium: the two-stream solution with delta scaling (Joseph, Wiscombe and Weinman 1976; Meador and
  Weaver 1980), radiance taken from its source function (Toon and others 1989), the escape function of a thick
  layer, `(3/7)·(1 + 2μ)` (Chandrasekhar's H-function to first order, as cloud retrievals use it), and the
  tilted column for a Sun off the vertical (Várnai and Davies 1999). None of those papers was opened for this
  revision. The closed forms were derived again, and they rest on the photon transport of parts XII to XV, not
  on the papers.
- **Broadsides**, as what was tried. Its cost model counts 141.7 density evaluations a ray: 19.7 on the view ray,
  39.8 in a six-tap Sun cone and 82.2 in direct ambient. Lighting was 86% of the work, and removing both lighting
  terms still left 21 ms. Its accepted results are the twelve volumes, the growth track's blind motion review,
  and the rule that the true Sun, not the apparent surface light, lights a medium. Its fair preset's "flat, soft
  shelves" and four-band base look were not accepted.
- **Jendersie and d'Eon 2023** ("An Approximate Mie Scattering Function for Fog and Cloud Rendering", SIGGRAPH
  Talks). A water droplet's phase function as a Henyey–Greenstein lobe plus a Draine lobe, four parameters fitted
  to droplet diameters of 5 to 50 µm. Read through Broadsides' `bw_core/src/medium/scatter.rs` at `b4d8250e6`,
  which carries the fits and a test that reproduces the paper's parameters at 20 µm, and in the paper's
  [supplemental](https://research.nvidia.com/labs/rtr/approximate-mie/publications/approximate-mie-supplemental.pdf),
  sections 2 and 3.1: both lobes are normalised phase functions and the fit is its equations 7 to 10. The paper
  treats a point source; averaging over the Sun's disc is this design's. Parts E and G of the proofs below
  evaluate it.
- **Frostbite's volumes** ([Hillaire 2015](https://www.slideshare.net/DICEStudio/physically-based-and-unified-volumetric-rendering-in-frostbite),
  slide text read). Fog near the camera is a grid aligned to the view frustum, 8×8-pixel tiles by 64 slices:
  media are written into it as scattering, extinction and phase; one pass lights each cell from the Sun through
  the shadow cascades, the sky and local lights; samples are jittered and blended 5% new to 95% history; a last
  pass integrates along each ray with the energy-conserving step. Opaque and transparent surfaces read it at
  their depth. 2.95 ms on a PlayStation 4 at 900p. It is the method of Wronski 2014, of Unreal's volumetric fog
  and of The Last of Us Part II (abstract read: "a view space frustum voxel grid ... to properly composite with
  the rest of the world").
- **Unreal's volumetric fog** ([documentation](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-fog-in-unreal-engine),
  read whole). The same grid; 1 ms on a PlayStation 4. Its stated limits are what this design avoids: beyond the
  grid's view distance an analytic height fog takes over and "the two can't be blended to match precisely"; fog
  casts no shadow on itself or the land; and clouds are a third system.
- **Red Dead Redemption 2** (Bauer 2019, [course abstract](https://advances.realtimerendering.com/s2019/index.htm)
  only; the 232 MB deck was not fetched). "Cloud and fog rendering" are one system: "a voxelization and
  raymarching approach for scattering and transmittance, used in the main viewport, reflection maps and a sky
  irradiance probe grid". A grid near the camera and a march beyond it, sharing data, is the shape selected here.
- **Nubis³** again: its voxel clouds "allowed for some low altitude fog-like clouds that the player could fly
  through", marched from the camera with the sky's own method.
- **Koschmieder's relation**: visual range is 3.912 over extinction. Fog is condensate with a visual range under
  1 km (extinction over 0.004 m⁻¹, about 0.02 g/m³ of 8 µm droplets), mist the same condensate thinner, and haze
  is dry aerosol (part VI of the last proof).
- **Looked for, 14 November:** a successor to these methods published from 2024 to 2026. Found: a 2026 method that
  relights clouds as meshes with a learned encoding (Olajos, Doggett and Goswami), and a hybrid of sparse voxels
  and noise. Titles and abstracts only. Neither is shipped practice or runs in a browser's limits, and neither
  changes a selection below.
- **Not read:** the Red Dead Redemption 2 course notes (Bauer 2019; not found to fetch), the 2015 Nubis
  talk, and the Broadsides cloud shaders beyond their names and sizes. Nothing below rests on them.

Five reproductions were run for this design, the second for its revision, the third for the first-principles
judgement, the fourth after its second review and the fifth for mist, fog and low cloud; a sixth followed that
addition's review and a seventh, of cloud light against photon transport, WP3's handoff. They are kept with their output in
[the evidence folder](../../../.orchestrate/evidence/CLOUDS/design-proof/). [Check that it can work](#check-that-it-can-work)
reports them.

**How a march reaches the look at this frame rate, decided against the cost.**

| Candidate | Verdict |
|---|---|
| March at render extent, lighting each sample with its own Sun cone and ambient rays (Broadsides) | Heavy by design: 33 million density evaluations a frame |
| Dense voxel grid of the whole sky | Nubis³ measured over 30 ms; memory grows with the horizon |
| Cloud cards or a panorama | Fails K1's parallax and K3's shade |
| A per-cloud cached volume (the earlier cohort lab) | 13 ms for one cloud on this Mac |
| **2D weather fields with a vertical profile, noise for detail, light from one summed Sun-path product, traced at part of the render extent and reconstructed over frames** | **Selected.** Every part is shipped practice; the work a ray does is bounded by a few texture reads a sample |

### Judged from first principles

Every choice the design of 13 November took from the brief, the port decision or the Broadsides notes, judged
against the users and cues above. "Keep" means it would be chosen again with Broadsides out of the room.

| Inherited choice | Verdict | Why |
|---|---|---|
| The product is a consolidation of two paths, procedural layers and a baked library, as a hybrid | **Change** | That is the history of two Broadsides agents, not a requirement. What a sky needs is one medium: one density in m⁻¹ that every consumer reads, and one light. Where the density comes from is data. Cells, the deck and shapes are sources of the one field, and which a sky has is the game's type table and library, so a sky of weather alone, of shapes alone or of both is the same code. The engine has no sky types and no mode. See [capabilities](#capabilities-shadow-only-and-headless) |
| A deterministic weather field with a CPU f64 query is the authority | **Keep** | It is the oracle the GPU is held to, the reason Chrome and the Mac show the same sky, and what a game reads with no graphics device. A sky that exists only in GPU textures gives none of the three |
| Wind as travel integrated per tick, and every pattern in the air frame | **Keep** | Drift is then exact and costs nothing, and no texture can slide through a mask |
| 2D fields with a vertical profile and noise, not a voxel sky | **Keep** | Every commissioned camera is below the cloud base, which is where the method shipped. Nubis³ measured a dense voxel sky at over 30 ms and built voxels for flying through cloud. A shape gives a true volume where one is wanted |
| Light from one summed Sun-path product and a few local samples | **Keep** | Already a departure from Broadsides, whose cone taps and ambient rays were 86% of its work. One product also gives ground shade and long-distance shadow |
| The first scattering order's phase is the retained reference's Henyey–Greenstein lobe, g = 0.86 | **Change** | The design already takes extinction from droplet radius; the phase now comes from the same droplet. Against the droplet's own phase the lobe is three to four times too bright from 5° to 10° off the Sun, 60 times too dim inside the aureole, and two to three times too dim looking away from the Sun (parts E and G). The first is a wide glow where a silver lining should be a rim; the last is the brightness of a cloud with the Sun behind the viewer. The angle is one value a ray, so the better phase costs the march nothing. See [cloud light](#cloud-light) |
| Light scattered more than once is three more orders with fixed weights, extinction scales and lobes (the retained reference) | **Change** | A cloud is bright because it scatters without absorbing: what does not pass through a thick cloud comes back out, 58% to 92% of the Sun's flux for the layers of part XII. Fixed orders whose weights sum to one can return only what single scattering does, 4% to 22%, at any depth. The Sun's light is instead a beam, which keeps the Sun's direction, and diffuse light, which is solved as a flux that leaves nothing out. See [cloud light](#cloud-light) |
| Drift taken at the committed tick | **Change** | A 60 Hz tick is shown twice on a 120 Hz display. At a 60× time-lapse in a 40 m/s wind a near cloud would stand still for one frame and jump 37 pixels the next (part F). Travel is a function of time, so each frame takes it at the display's time. See [time](#time-drift-at-the-displays-time-evolution-on-the-tick) |
| A placed volume stands for one cell of the field | **Keep** | It is why the query, the picture and the shade agree on a shape's outline at any distance, and why a shape forms, drifts and thins like any cloud. It does not rest on the library existing |
| Shapes are placed only by the field's hash | **Change** | A demo composes its sky: a tower beside the Sun at sunset. A profile places shapes by the hash or by the game's own list of [directed shapes](#placed-volumes), which are cells whose place, shape and life the game sets |
| Shadow-only as the one reduced profile | **Change** | Shade, the visible sky, the world's light and shapes are each a capability, and a scene declares only those it uses. A sunset demo needs no library; a top-down game needs only shade, and holds a library only if it wants the shade of shapes |
| Clouds shade and light the world through `ATMOSPHERE`, never by themselves | **Keep** | One owner applies each factor once. A private sky or a second shadow term is what made Broadsides' verdicts unusable here |
| The layer ends at render extent and the engine reconstructs | **Keep** | One reconstructor sees every drawer's motion. Unreal, Frostbite and Decima all trace clouds below the render extent and hand a full-extent layer on |
| Nothing copied from Broadsides until a licence is named; stand-in shapes first | **Change** | The owner's words of 14 November. The library is the example's content from WP6. See below |

### Judged again for the whole atmosphere

The owner's words of 14 November add mist and fog and ask for one atmosphere. Judged from the same users: a
demo of dawn in a valley, a game that asks whether one point can see another, and a sea scene that meets a fog
bank all need the droplets near the ground to be the same droplets as the clouds'. Each row is an assumption
this revision replaces.

| Assumption until now | Replaced by | Why |
|---|---|---|
| Fog is a separate provider, added later, that lies below the cloud base and composes with cloud by height (this design; ENVIRONMENT's "fog is separate") | Mist, fog and low cloud are condensate of the one weather field, with sources anchored to the ground | Fog is cloud at the ground. One density gives one shade, one light, one query, and a deck that can come down onto the hills |
| The camera and all solid geometry stay below the lowest cloud base; partial immersion is not modelled ([plan](PLAN.md#open-for-the-human), until now) | The ground and the camera may stand in condensate or above it | "Low cloud the camera can enter" |
| A view ray's media are split by height: air below, cloud above | They are split by distance, at a handover: `ATMOSPHERE`'s view volume integrates air, haze and condensate jointly near the camera, and this module's trace carries condensate beyond | Near the camera fog meets geometry, water and moving props, where a grid aligned to the view is the shipped answer; far off it is a soft shape on the land, which the trace already draws at a finer grain |
| Haze under cloud is one number a frame, with no shafts | Inside the view volume the air is lit sample by sample through the cloud factor and the scene's shadows, so shafts stand in haze. The one number remains for a scene without the view volume | Shafts are what broken cloud over haze looks like |
| The trace ignores depth except to skip covered rays | A ray stops at the scene's depth and keeps a stop at each pixel's depth | Far mist lies against hillsides and water |
| "Atmospheric system" means clouds and weather; `ATMOSPHERE` is not reopened ([plan](PLAN.md#open-for-the-human), until now) | `ATMOSPHERE` gains the view volume and a haze it takes from the weather, as work packages after G3s closes | The owner's words. The V1 exit scene does not change |
| The Sun's path ends at the cloud layer's base, and a receiver below it reads one plane | It continues to the ground in a ground band, read at the receiver's own position | Land inside fog must lose its Sun, and a bank must shade the water beside it |

Kept, and now carrying more: the weather field as the one authority, the air frame, one density in m⁻¹, the
Sun's path as the one optical depth, cloud light with the droplets' phase, the layer ending at render extent,
capabilities a scene leaves out at no cost, and light and shade applied by `ATMOSPHERE` alone.

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each work package judges fit file by file; this table says where to look. Paths are in
the Broadsides repository at `b4d8250e6` unless a commit is given, and nothing there is ever changed. Of these,
the planner opened `scatter.rs`, `phase.rs`, the library's licence record and handoff, and the notes named in
[the request](#the-request); the shaders' fit is expected from their names and the notes, not read.

| Mechanism here | Where Broadsides has it | Expected fit |
|---|---|---|
| The droplet phase for a point Sun: the published fit, both lobes written to stay exact near the forward peak, the mean cosine, and the energy-conserving step | `crates/core/bw_core/src/medium/scatter.rs`, with its tests; the GPU twin `crates/gpu/bw_gpu/shaders/lib/medium.wesl` | Close, but for the Sun's disc. Its `spread` widens the lobe's squared angle and leaves the numerator alone, which removes 0.3% to 23% of the first order's energy across the droplet range (part G). That rule is not copied; [cloud light](#cloud-light) averages over the disc |
| A phase table by angle, read once a ray | `crates/render/bw_render_clouds/src/phase.rs`, `shaders/cloud_phase_table.wesl` | The form fits: a table spaced toward the forward peak, held as read-only storage with no sampler. Its contents are the widened lobe and are made again from the model's disc average |
| Integer hashes and noise | `bw_core::noise`; `shaders/cloud_noise.wesl` | May fit. The noise here must come from the same integer hash on every target |
| The twelve accepted volumes, their records and the growth track's keys | At `79637f789`: `assets/clouds/heaped-fair/` (Git LFS; density is the PNG's red channel, 128³), `factory/clouds/heaped-fair/B3-HANDOFF.md` for axes, extents, placement and hashes, and `factory/clouds/heaped-fair/review/m2-authored-growth-v2/` for the track's manifest of eleven keys. The key volumes themselves are outside Git, in the Broadsides folder under `tmp/evidence-store/cloud-route-2026-11-08/m2-authored-growth-v2/`, and `factory/clouds/heaped-fair/coherent_growth.py` makes them again | The example's library, unchanged. Its licence record comes with it |
| The shape record and its parser | `crates/core/bw_data/src/cloud_shape.rs` | Game-side, for reading the library's sidecars into the example's placement data |
| Octave light, the density operators, the resolve's reprojection and clamp | `shaders/cloud_lighting.wesl`, `cloud_density.wesl`, `cloud_resolve.wesl`, `cloud_reconstruction.wesl`; CPU twins in `src/twin/` | Take functions where they state what this design states. The octave light does not: [cloud light](#cloud-light)'s Sun term replaces it. The cone taps and ambient rays in the lighting are where the 289 ms went and are not copied |
| The weather operator and presets | `crates/core/bw_core/src/weather.rs`, `crates/sim/bw_world/src/weather.rs`, `cloud_weather.ron` | Reading matter. The field here is the cell process and the deck of this design |
| The packed Beer shadow, the atlas and pager, parcel and ambient caches, sky capture, the Bevy host, `module.rs` | | Not used. The Sun's path is a different product, slabs replace the atlas, and Thrum has one owner for the rest |

Broadsides shaders are WESL and Thrum's are WGSL, so a copied shader has its imports resolved into Thrum's
modules; its algorithm is left as it is. [The handoff](#proof-and-builder-handoff) says how a copy is recorded.

**What would change these selections.** WP3's record shows what the trace costs. If the frame is over the refresh
interval there, WP3 tries the remedies in [risk 1](#risks) that its record points at and records what each did,
and building goes on; whether the layer's extent or the light model should then change is a planner's decision
from that record, never a stop for a builder. If WP5 shows the summed Sun-path product cannot carry low-Sun
relief, its near level is refined or the local samples grow, here first. WP3's stills showed the diffuse light
wrong in a cloud of heaps in a way the box of part XV did not, and [the lit plane](#cloud-light) was corrected
for it from [the controls](../../../.orchestrate/evidence/CLOUDS/WP3-plane/README.md) (part XIX). If stills show it wrong again,
the plane's turn and the view boundary are where a planner corrects it, from the record; a gain, a second
light, fixed orders and a plane read from a sample's depths are never the correction. If a game needs hashed shapes and
directed ones in one sky, the map's texel gains a second slot; nothing else moves. If a game wants the shade of
shapes with no volume in memory, a profile that shades every placed cell through its column form is the path
already built for a lease that is not resident: a different, approximate shade (part H), and a choice a game
would declare. It is not built.

## Mechanisms

### One weather field, in the air frame

The **weather field** is the single authority. It is a pure function in `thrum-cloud-model`, evaluated in f64
with no GPU:

`field(seed, parameters, directed shapes, travel, weather time, world point) → coverage, type, base, top, age, column optical depth`

- **The weather state** is a small record the game keeps in its world and steps once a tick: the seed; the
  current parameters (coverage 0–1, a type axis 0–1 from layered to heaped, wind as a velocity at cloud height,
  a time scale for time-lapse); their targets and a response time; the **travel**, wind integrated per tick in
  f64 metres; and the **weather time**, the tick duration times the time scale, integrated the same way. Wind is
  never multiplied by absolute time. A parameter moves toward its target over the response time, so a preset
  change is a ramp. The state also holds the travel of its last tick, for [time](#time-drift-at-the-displays-time-evolution-on-the-tick),
  and, in a profile that places shapes itself, its list of [directed shapes](#placed-volumes). The state travels
  in the tick page (C02).
- **The air frame.** Every pattern is a function of `world point − travel` and weather time. So the whole field
  drifts with the wind in world space, exactly, and what changes in time is the weather and never a texture
  sliding through a mask. Noise that gives detail lives in the same frame.
- **Cells.** Heaped cloud is a set of convective cells on a lattice in the air frame. Each lattice site hosts
  one cell per generation period. A hash of the site and the generation gives the cell its birth time anywhere
  in the period, its lifetime (12 to 30 minutes in the first profile), its centre anywhere in the site, its
  size, its depth and a shape seed. A cell's **envelope** rises smoothly from zero at birth and returns to zero
  at death, and scales its radius, depth and density together: the cell forms, grows and thins. Coverage sets
  how many cells exist, through a smooth threshold on a per-cell hash, so raising coverage swells cells in and
  never switches one on. A cell's column is a cluster of turrets over a flat base at the type's condensation
  level, or the footprint of an authored shape where the cell is a [placed](#placed-volumes) one; bases are
  shared, sizes vary.
- **Turrets.** A heaped cloud is several rising heaps of different heights on one base, and its outline is
  theirs. The **turret field** is a second, finer lattice in the air frame, hash-named as the cells' is. Each
  site holds one rounded cap with its own centre, width and height: a few tall and wide, most low and narrow.
  The field is a function of the air-frame point alone. It takes no time, so a cell's envelope raises and
  lowers the turrets under it. It takes no footprint, so every reader gets the same value at the same point:
  the query, and each level of [the weather map](#the-weather-map). A cell's thickness at a point is the
  lesser of two: its **dome**, which rises from the cell's rim to its depth at its centre and is the most the
  cell can hold there, and its depth times its envelope times the turret field. What that leaves thinner than
  a least share of the cell's depth is cleared by one **clearing factor**, which rises smoothly from 0 at the
  least share to 1 a little above it. The factor multiplies the cell's thickness and its coverage alike,
  before overlapping cells are combined. So no sheet or lip stands between turrets or round a rim, and a cell
  that grows through the least share brings its coverage up from zero with its thickness. Coverage left whole
  wherever any thickness stood would switch a cell on in what a game reads: by 0.27 at one cell's centre
  however small the step, where under the factor the change shrinks with the step (part XVIII). The turret pitch, the least share and the
  factor's width are profile constants, as the lattice is. One dome a cell, the rule this replaces, gave every
  cloud the same outline at a different size ([part XVIII](#check-that-it-can-work)).
- **The deck.** Layered cloud is a sheet whose thickness follows a smooth evolving noise against a threshold set
  by coverage and type: gaps at a broken deck, none at overcast.
- **Types** are game data: a table along the type axis giving base height, depth range, liquid water content,
  droplet radius, the shares of cells and deck, and a vertical profile. The engine interpolates it. Extinction
  is physical: `σ = 3·LWC / (2·ρ_water·r_droplet)` in m⁻¹, about 0.06 m⁻¹ for 0.4 g/m³ and 10 µm.
- **Continuity.** Under cells and the deck the field is continuous in space, in weather time and in every
  parameter: its density at every point, and its column's optical depth, coverage, thickness and top. Where
  cells overlap the column takes the larger depth and the larger profile, never a switch between cells.
  Overlapping cells scale the one turret field, so a shallow cell beside a deep one is lower turrets beside
  taller ones, and never a shelf under a dome.
  [Outputs](#one-weather-field-in-the-air-frame) says what a placed shape's column does.
- **Lattice and period are profile constants.** A preset never changes them, because scaling a lattice would move
  every cloud at once.
- **Precision.** Site indices and generations are integers. Weather time reaches the GPU as an integer generation
  count and a fraction, and positions as integer texel coordinates plus a snapped anchor, so the GPU names the
  same cells as the CPU after any travel.

**Outputs.** A query outside nothing is invalid. Every valid answer is the weather's column at that point.

- **Type** is the weather's, the same at every point: the type axis as the state holds it.
- **Base and top** are heights in the world, and between them lies all of the column's condensate, with
  nothing eroded. Under cells and the deck the base is the type's condensation level, which every column
  shares, and the top is the base plus the column's **thickness**, its depth of cloud. Over a
  [placed shape](#placed-volumes) they are the lowest and the highest height of the shape's own column, and
  where a shape and weather share a column, the lower base and the higher top.
- **Coverage** is the column's profile, and **age** the dominant cell's life phase.
- **A clear column** has no thickness: its top is exactly its base, at the type's condensation level, and its
  coverage, age and column optical depth are exactly zero. A consumer asks whether a column is clear by its
  coverage or its optical depth, never by a height.

No height is ever zero to mean that there is no cloud, for three reasons. Zero is a height like any other: a
layered type's base may lie [at the ground](#condensate-that-meets-the-ground-mist-fog-and-low-cloud), and
the ground may lie below zero. The [weather map](#the-weather-map) holds base and top and is read through a
filter, so a clear texel that held zero beside a cloudy one would hang cloud below the shared base along
every cloud's edge, where a clear texel that holds its base draws a thinner cloud on the same base. And a top
that fell to zero as a cell cleared would step by the height of the base, where the cell's thickness goes to
nothing smoothly.

**What steps and what does not.** Thickness follows a cell's envelope, its rim and its turrets to zero, so under cells and
the deck the top comes down to the base as a column clears and rises from it as one forms. There the top, the
thickness, the coverage and the optical depth never step, and the base moves only as the type does. A placed
shape's extents are its geometry, and its envelope does not scale them: its base and top stand whole at its
silhouette and from the tick its envelope leaves zero, while its optical depth and its density rise from
zero. Age changes hands when another cell becomes the dominant one.

**What a query promises.** Column optical depth is the vertical integral of the density below with nothing
eroded: every noise term at its uneroded value. In the model that is exact at every point, for cells, the deck
and [placed shapes](#placed-volumes) alike. It is an upper **bound** on the column that is drawn there, because
noise only removes extinction, and it is zero exactly where the sky is clear. The field holds no average: a
query never answers for a neighbourhood, and no form of a cloud is fitted to another's mean. The averages in
this design are named where they are made: the GPU reads the field through filtered textures, so what it draws
over a point is a local average of the model's density over a texel, held to the model at stated points within
a tolerance its test states; the [coarse density](#one-density-in-m¹) takes the detail noise at its mean; and
the Sun path's [far value](#the-suns-path-one-optical-depth-product) is a mean transmittance.

### Time: drift at the display's time, evolution on the tick

The weather state steps on the committed tick, and the display shows each 60 Hz tick twice or more. Travel is a
function of time, so every presented frame takes it **at the display's time** `t = C − 1 + p` in ticks, where `C`
is the frame's committed tick and `p` its [display phase](../EFFECT-GRAPH/DESIGN.md#the-display-phase-wp4), which
`EFFECT-GRAPH` WP4 gives every effect. That time is the frame's **consumed target**, on the world's time, less
one tick. The consumed target is the time the frame's host observed, plus its lead of at most 50 ms toward the
moment the platform expects to show the frame, and it is a time on the host's clock. Display time is the
world's: by [the timing model](../EFFECT-GRAPH/DESIGN.md#the-timing-model) it holds none of the time the world's
clock was paused or overloaded, and it is the consumed target less one tick as plain numbers only on a world
that started at host time zero on tick 0 and has not stopped, as the V1 game's does. This module reads the tick
and the phase and no host time. Formed on the CPU in f64 from the state:

- at `p ≤ 1`, between the two committed ticks: `travel(t) = travel(C) − (1 − p) · (travel(C) − travel(C − 1))`;
- at `p > 1`, past the committed tick, `thrum-cloud-model` carries the travel forward a tick at a time. From the
  committed state it makes the **predicted ticks** `C + 1`, `C + 2` and on, each from the one before by the
  arithmetic of its own step: the wind and the time scale move toward the targets the committed state holds,
  over its response time, and the tick's travel is added. With `n` whole ticks and a part `f` in `p − 1`,
  `travel(t) = travel(C + n) + f · (travel(C + n + 1) − travel(C + n))`. It commits nothing. The phase is at
  most 50 ms of ticks past 1, so a frame makes at most 3 predicted ticks at 60 Hz, a few scalar operations
  each.

The next tick's travel carried across the whole of `p − 1` is not this rule. It is right in a steady wind and
wrong while a wind or a time scale is ramping, because each tick of a ramp travels a different distance. With
the wind at 10 m/s on its way to 40, a 10 s response and 60×, one display time two and a half ticks past the
committed tick is 35 m from tick `C` and 41 m from tick `C + 1`: a 6 m step with nothing changed. By predicted
ticks it is 43 m from both ([the check](../../../.orchestrate/evidence/CLOUDS/WP2-plan/display-travel.txt), part 1).

The interpolation alpha is not that time. It stops at the committed tick, and a consumed target usually lies
past it for part of every tick: at 60× in a 40 m/s wind WP2's first capture drifted 3.2 m and 36.8 m on alternate
frames where 20 m was due ([the reading](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/README.md)). With the phase, in a
steady wind and time scale, while the world's clock runs and the phase is inside its bound, equal intervals
between consumed targets are equal steps of travel, across a tick's commit as within a tick; through a ramp
the steps follow the ramp, as **What the prediction guarantees** says below. A frame whose platform target
had expired consumes the time it was observed at, and
takes its travel there. While the clock is stopped, and until the first tick after a resume, the phase is 1 and
the travel is the committed tick's.

**What the prediction guarantees.** Past the committed tick the travel is a prediction of at most 50 ms of the
world's time, which at 60× in a 40 m/s wind is 120 m.

- **While the targets stand, it is exact.** A predicted tick is made by the step's arithmetic from the state
  the step will start from, so while the control targets and the response time the committed state holds stay
  what they were, it is the tick the step then commits, bit for bit. Two frames that show one display time
  from two committed ticks therefore take the same travel, to the f64 rounding of the last interpolation: in a
  steady wind, through a wind or a time-scale ramp, and when a ramp reaches its target inside the prediction
  (the check's part 2: 33,355 pairs at 30, 60 and 240 Hz, every one exactly equal in rational arithmetic).
- **When a target changes, the correction is the change.** A game sets a target between two commits. Frames
  before the commit predicted with the old target and frames after it hold the new one. At one display time
  the travel then moves by the travel the old targets would have made less the travel the new ones make, over
  the part of the prediction past tick `C`, and by nothing else. For a wind target, with `w` the weather
  seconds in a tick and `a = 40 · w / response` the most a tick changes the wind, that is at most
  `2 · a · w · (1 + 2 + … + n + f · (n + 1))`. At 60× with a 10 s response it is 4 m half a tick past the
  committed tick, which is as far as a 120 Hz display's lead reaches, 8 m one tick past and 48 m at the bound;
  at 1× it is 13 mm at the bound. A time scale sent from 60 to 1 in a steady 40 m/s wind moves it 2 m half a
  tick past and 22 m at the bound (the check's part 4). The game's response time sets these, and a shorter one
  makes them larger.
- **Nothing else is predicted.** No state is stepped and no product is written for it.

Travel enters a frame only through the anchors that place the air frame in the world: the trace's, the mapping of
the ground field and of the ground band that `ATMOSPHERE` reads, the overlay's, and the motion a cloud writes. What
belongs to the ground takes no travel: the [surface field](#condensate-that-meets-the-ground-mist-fog-and-low-cloud)'s
texture is anchored to the world. So a frame that shows the
same tick again moves every cloud and its shade by the wind's travel since the last frame, and rewrites no texel:
the weather map, the Sun's path and everything that evolves stay at the committed tick, as `EFFECT-GRAPH`'s state
rule requires. Evolution can stay there because it is slow: at 60× a cell changes its envelope by at most 0.2% of
its range in a tick (part F). On a cut, a seek or a new timeline the frame takes `travel(C)`, as it does when the
phase is exactly 1 because the clock is not interpolating: a pause, an overload or a capture frame.

### One density, in m⁻¹

Every consumer (the view, the Sun's path, the sky overlay, the CPU reference) evaluates one density function:

`σ(point) = σ_type · profile(height fraction) · shape(column profile, noise)`

The column profile is held against the **shape noise**, a billow field in the air frame: a few octaves of
round lobes with sharp creases between them, as inverted cellular noise gives them and the Perlin-Worley
noise of shipped clouds has them. Its coarsest lobe is about half a turret
wide and each octave is two to three times finer. The threshold it sets spans the profile's whole range short
of the core, so a turret's interior stays solid and its surface moves by a lobe's size and not by a skin's:
lobes on a turret, and smaller lobes on those. A finer detail noise erodes only the edge. Noise can remove
extinction and never add it beyond the column's bound. As a cell's envelope moves, the threshold sweeps
through fixed noise, so billows appear and dissolve as structure and nothing slides.

Each noise is one tiling volume, generated on the GPU at preparation from the same integer hash on every
target, and costs a sample one read, where octaves of lobes computed at the sample would each cost a
neighbourhood of lattice sites. The volume's texels are the noise's definition: the model computes the same
texels from the hash and interpolates them as the sampler does, so the GPU and the model read one function. A
volume is read at a mip chosen from the sample's footprint, so a far ray reads an average, not aliased detail,
and keeps every lobe wider than its footprint. A volume repeats, at a period longer than a turret. The turret
field is what makes each cloud its own, so it is evaluated from the hash, in the weather map, and never
repeats.

**Full and coarse.** The function is evaluated two ways, which differ only in the detail term. The **full
density** reads the detail noise; the view march and its local light samples use it. The **coarse density**
takes the detail noise at its mean, a constant of the profile, so it erodes an edge by the average amount and
reads no detail texture; the Sun's path and the sky overlay use it. It is what the full density tends to as a
sample's footprint grows past the detail noise, and the Sun path's columns are wider than that. Nothing else
differs: over a [placed cell](#placed-volumes) both read the same forms with the same weights, the lent slab
included. The model evaluates both in f64, and each consumer is held to the reference for the density it uses.

Coefficients from different condensate add before attenuation. Two forms of the same condensate (the volume form
and the column form of one [placed cell](#placed-volumes)) are blended with weights that sum to one. Nothing is
darkened twice and no density is divided by a count.

### The weather map

The GPU reads the field through a **weather map**: a few world-anchored wrapping levels in the air frame, whose
texel doubles or quadruples outward to the horizon. A texel holds the field's column (base, top, profile, type),
a clear one with its top at its base, and, apart from it, two presentation values: the slot of at most one
placed shape and the weight of its volume form. Compute passes in `simulate` evaluate the same cell, turret and deck rules as the CPU. Because the map lives in
the air frame, wind costs nothing: only evolution and the camera's movement through the air need new texels, so
the map is refreshed in parts. A coarse level of the tallest top in each region lets a ray step across empty
air: a region is empty where its tallest top is no higher than the base.

**Every level holds the query's column.** On each level a texel holds the field's column at its own centre:
the point query's answer there, turrets and clearing included. It is never an average over the texel, and
never a column made by a rule the query does not have. So a centre the query calls clear is stored clear, a
centre it calls cloudy is stored with its thickness, and the map's check compares the two at the centre on
every level. What is drawn between centres is the filter's, as
[what a query promises](#one-weather-field-in-the-air-frame) says.

A level whose texel is wider than a turret, as the first profile's 1,600 m level is against a 1,300 m pitch,
samples turrets it cannot resolve. It holds the columns its centres land on and misses the rest, and the
filter joins what it holds into broader heaps than the near levels draw, carved by the same shape noise.
Three things make that sound ([part XVIII](#check-that-it-can-work)):

- **It keeps the sky's amount.** Over 96 km of the fair sky the far centres are cloudy at 0.131 of points
  where the field is at 0.128, and the mean thickness is 99 m in both.
- **It does not flicker.** A level moves by whole texels, so a centre is a fixed point of the air frame, and
  there the field is continuous in weather time: a far texel's column rises and falls smoothly.
- **One rule serves every texel.** A share standing in for the turrets a level cannot resolve does not: of
  the review's two far texels, one needs a share of at most 0.42 to stay clear and the other more than 0.75
  to keep any cloud.

What the sample changes is a far cloud's heaps: 0.60 of the far texels with cloud somewhere in them have a
clear centre, so the far level draws fewer and broader heaps, and they change where a cloud drifts from one
level into the next. WP3.2 judges both, and [the second risk](#risks) gives the fixes.

**The map holds nothing of the ground.** With ground condensate a texel also holds each ground row's **air
factors**, and only those: its extent and the cut of its noise. A row's
[world factors](#condensate-that-meets-the-ground-mist-fog-and-low-cloud), its height over the surface, its datum
and its shore, are read from the surface texture at the sample's own world point, and the density multiplies the
two there. So wind still costs nothing, and no texel that drifts holds a shoreline or a height.

**Age.** Each part of the map holds the field at the tick it was last written, and the map keeps that tick for
each part. The cadence is a bound on a part's age in **weather time** and in the camera's travel through the
air, never in frames: half a second of weather in the first profile, which is 2 Hz at 1× and the whole map
every frame at 60×. The Sun's path is bounded the same way, level by level. So a time-lapse refreshes more each
frame and replaces no texel with one further on in the weather than real time would.

The GPU column matches the CPU evaluator at texel centres, on every level, at the tick each part was written, within a tolerance
that WP2 measured and its test states; that no part is older than its bound is asserted apart from it.

### The Sun's path: one optical-depth product

This section describes the product's **cloud band**. With ground condensate it continues to the ground in a
[ground band](#condensate-that-meets-the-ground-mist-fog-and-low-cloud), which holds the whole path: its own
stretch of the Sun's ray, plus the cloud band's value where the ray leaves it. A receiver reads one or the
other, never their sum.

One product holds optical depth along the Sun's ray, from the top of the cloud layer down: `tau(column,
height)`, in levels of camera-following windows whose columns are Sun rays. A column is indexed by where its ray
crosses the bottom of the layer. It is filled by marching the **coarse density** along each ray, time-sliced by
tiles of columns, and stored in place. It integrates the same density in every profile and never reads the
detail noise, so its bytes do not depend on whether clouds are drawn. Over a placed cell that density is the
[shape's own](#placed-volumes), read from the lent slab.

**The path direction.** The columns follow the environment identity's true Sun, never the apparent surface
light, with one floor: their elevation is `max(Sun elevation, lowest elevation)`, a profile constant (2° in the
first profile), at the Sun's azimuth. The path direction is a continuous function of the Sun, the product is
filled at every Sun elevation, and no reader switches to anything as the Sun sinks. Below the floor the path is
a stated approximation: the optical depth of the 2° path stands for the flatter one, while the Sun's light at a
point, the phase angle and the air's own transmittance still use the true Sun. When the Sun moves, the columns
shear with it; the refresh cadence bounds the lag, and a jump larger than a bound refills the product in that
frame.

It has three readers and one meaning:

- **Ground shade.** A level's last slice is `tau` through the whole layer for a receiver below it. The slices of
  the levels are composed into one 2D field, which is registered with `ATMOSPHERE` as the Sun-path optical depth
  with its path direction ([ruling 1](#what-clouds-asks-of-other-modules)). `ATMOSPHERE` projects each receiver
  along the path direction and applies the factor once on every direct-Sun receiver: terrain, instances, cooked
  meshes, the cooked volume and the sea. `CLOUDS` never darkens a receiver.
- **Self-shadow.** A cloud sample's light is `exp(−tau)` with `tau` in two disjoint parts that add: a short local
  march of a few samples of the full density along the path direction, then the product read at the far end of
  that segment. The local part resolves lobes smaller than a column, and the detail the coarse density
  averages; the product carries everything beyond, to any distance. The sample count is a knob.
- **The sky overlay** below reads it the same way.

**The edge of the field.** The outermost level reaches the horizon of the cloud tops, so every cloud sample the
trace can reach lies inside the product; only a ground receiver can lie outside it. For those the field has a
**far value**, one number a frame: `T_far`, the mean transmittance of the outermost level's ground slice as it
stands that frame, given to `ATMOSPHERE` as `tau_far = −ln T_far`. A receiver's factor is
`mix(T_far, exp(−tau), w)`, where `w` is 1 inside the field, falls smoothly to 0 across a border inside the
field's edge, and is 0 outside. So the factor is continuous across the edge whatever lies on either side of it,
and the far value needs nothing that can become invalid. It is a mean of transmittance, not of optical depth,
because a broken sky far off lets through the light of its gaps; under a clear sky it is exactly 1 and under a
closed deck it is no brighter than the deck's thinnest column. It moves by at most one refreshed tile's share a
frame.

### Cloud light

A sample's scattered radiance, integrated over its step with Frostbite's equation 17:

`L = σ · [ E_sun(point) · (S_beam + S_diffuse) + L_ambient ]`

The Sun's light at a sample is in two parts that between them hold it all, once. The **beam** still travels
nearly as the Sun's rays do. The **diffuse** light has been scattered until it has forgotten them. Both are
closed forms of optical depths the trace already holds, so multiple scattering costs no density read. The term
was chosen because it gives a cloud its brightness with nothing tuned: against photon transport it is typically
a factor of 1.75 out on a box's faces, where the four orders it replaces are 40 times too dark (parts XII to
XVI and XIX of [the proofs](#check-that-it-can-work)). It is the starting point, not a pass mark. A game changes how its
clouds are lit through a type's droplet and its water, which are the phase and the density. A builder whose
clouds fail the inspection list may also adjust the term's constants by eye (the plane's turn, the boundary
depth, the beam's lobe), keeps one term for every consumer, and says in the evidence what changed. No item holds the shader to the equations below, to the model or to photons.

- **Sun.** `E_sun` is the lent Sun light at a world position: the true Sun through the air, with no cloud factor.
  It is evaluated at the layer's base and top on each ray and interpolated by height, so a tall cloud's top
  stays lit after its base has gone into the Earth's shadow, and sunset colour comes from the sky's own
  transmittance.
- **The first order's phase is the droplet's own**: a Henyey–Greenstein lobe plus a Draine lobe (Jendersie and
  d'Eon 2023), `(1 − w)·HG(θ, g_HG) + w·Draine(θ, g_D, α)`. Its four parameters are type data.
  `thrum-cloud-model` gives the published fit from a droplet diameter of 5 to 50 µm, so a type takes its phase
  from the droplet radius its extinction comes from, and a game that wants another look gives its own four
  numbers. This lobe is the silver lining: a rim within a few degrees of the Sun, not a glow 20° wide, and a
  brighter cloud with the Sun behind the viewer.
- **The Sun is a disc, and the phase is averaged over it.** What a sample scatters toward the eye from a Sun of
  angular radius ρ is the mean of that phase over the directions to every point of the lent Sun's disc. This is
  the phase that is rendered, and it is the whole first order's, both lobes. An average of a phase that
  integrates to 1 integrates to 1, so the disc moves light about within a degree or so of the Sun and removes
  none: across 5 to 50 µm the integral is 1.000000 (part G). Its mean cosine is the point Sun's times
  `(1 + cos ρ)/2`, a change in the sixth decimal place. At the Sun's centre it is the phase's energy inside the
  disc over the disc's solid angle, which bounds the forward peak: 1,792 sr⁻¹ for a 20 µm droplet against
  2,743 for a point Sun, and 4,325 against 18,673 at 50 µm, where the lobe is narrower than the Sun. Beyond 2°
  it is the point Sun's phase to within 2%.
- **A table, because the average has no closed form.** Widening the lobe's squared angle, the rule Broadsides
  uses, loses 0.3% to 23% of the first order's energy across the droplet range; the same lobe scaled back to unit
  energy is a lobe of lower `g`, which puts up to 70% too much light 1° to 2° from the Sun (part G). So
  `thrum-cloud-model` evaluates the average in f64, by quadrature over the disc, and gives it as a table over
  the scattering angle, spaced so the degrees nearest the Sun hold most of its entries. Each row of the type
  table has its table, made at preparation from the row's four parameters and the lent Sun's angular radius; a
  type between two rows blends their tables by weight, and a blend of phases that each integrate to 1 does too.
  The renderer reads the model's table and computes none of its own. Its size is the builder's, held to the
  bound WP1 measures: 1,025 entries hold the integral to 1.0001 for a 50 µm droplet, and 257 to 1.002.
- **What the droplet gives the Sun term.** Four numbers, which `thrum-cloud-model` works out from a type's
  phase at preparation and gives with its table: the mean cosine `g`; the **forward share** `f`, the phase's
  second Legendre moment; the broad lobe's weight `w`, which is the Draine lobe's; and that lobe's mean cosine
  `g_B`. For a 24 µm droplet they are 0.882, 0.807, 0.501 and 0.770. A type between two rows blends them by
  weight, as it blends the tables.
- **What a sample knows.** `tau_sun`, the optical depth toward the Sun, as
  [self-shadow](#the-suns-path-one-optical-depth-product) reads it. `tau_col`, the whole depth of that Sun
  column through the layer: the product's last slice at the same column, and never less than `tau_sun`.
  `tau_view`, the optical depth of the condensate a march has itself crossed before the sample, which it
  holds as its transmittance; only the view boundary reads it. `s`, the path direction, a unit vector toward
  the Sun, and `μ₀`, the sine of its elevation. `ω`, the unit vector from the sample toward the eye. `h`, the
  **plane's turn**: half the type's share of cells, so 0 for a layered type and ½ for a heaped one, and 0 for
  ground condensate. The Sun term reads no depth but these three.
- **The beam.**
  `S_beam = P₁(θ)·exp(−b₁·tau_sun) + HG(θ, g·g_B)·max(0, exp(−(1 − f)·tau_sun) − exp(−b₁·tau_sun))`, with
  `b₁ = max(w, 1 − f)`. The droplet's narrow lobe turns light by under a degree, so light that has met only
  that lobe is still the Sun's beam: it thins at the broad lobe's rate alone and scatters toward the eye with
  the droplet's own phase `P₁`. Light the broad lobe has turned once or twice is still forward. It is the
  share between that and `exp(−(1 − f)·tau_sun)`, and it scatters toward the eye through one wider
  Henyey–Greenstein lobe, whose asymmetry is the mean cosine after one broad scattering and the scattering
  toward the eye. Every lobe integrates to one, so the beam scatters exactly the light it holds. What it no
  longer holds, `1 − exp(−(1 − f)·tau_sun)` of the Sun, is the diffuse light's source. Through cloud of optical
  depth 1 to 16 this is within 0.73 to 1.26 of transport from 2° to 90° off the Sun, where the first order
  alone is 0.20 to 1.00 (part XIV).
- **The diffuse light** is the two-stream solution for a layer that scatters without absorbing, with delta
  scaling and the escape function of a thick layer, taken along the sample's Sun column:

  ```
  n   = ((1 − h)·up + h·s) / |(1 − h)·up + h·s|
  μ   = n · s
  x   = (1 − g)·μ·tau_sun        X = (1 − g)·μ·tau_col
  E   = exp(−(1 − f)·tau_sun)    E_col = exp(−(1 − f)·tau_col)
  K   = (3/7)·(1 + 2μ)
  T   = min(1, (K + (1 − K)·E_col) / (1 + ¾·X))
  I₀  = max(0, μ/(2π) · (1 − T·(1 + 1.5·x) + (2K − 1)·(1 − E)))
  I₁  = 3μ/(4π) · (T − E)
  c   = ω · n                    ℓ = (X + 4/3)/π             v = (1 − g)·tau_view
  d   = x if c > 0, else X − x
  q   = B · clamp(1 − v·|c|/d, 0, 1), and 0 where d is 0
  a   = q·exp(−(2/3)·v/ℓ)
  S_diffuse = max(0, (1 − a·ℓ/(2/3 + ℓ))·(I₀ − g·I₁·c) + g·I₀·a·(2/3)/(2/3 + ℓ))
  ```

  `T` is the share of the Sun's flux the column passes and `1 − T` the share it returns, so the column gives
  back exactly what it does not pass: 0.844 at a depth of 64 under a 45° Sun, where photons give 0.845. `I₀` is
  the diffuse light's mean radiance at the sample, falling in a straight line from the lit side to the far
  side, which is why a base is darker and never black. `I₁` is its net flow, and `−g·I₁·c` is why a face seen
  square on is brighter than one seen at a graze.
- **The lit plane.** A column knows how deep it is, not which way its surface faces, and the Sun's flux on a
  surface goes as the cosine between them. The plane is set by the type and is one direction for the whole
  cloud: its normal `n` is turned from straight up toward the Sun by the plane's turn. A layered type's plane
  is level and `μ` is `μ₀`, which is a level layer exactly, so an overcast sky darkens as the Sun sinks. A
  heaped type's plane lies halfway between up and the Sun: a heap presents its top and its Sun-facing side
  together, so at a 15° Sun it takes three times the light of a level top, and its sunlit sides stay bright
  through the day. A type between the two turns its plane by its share of cells, and the weather state ramps
  a type, so the plane never changes across a cloud or in a step. This is how shipped clouds are lit: by the
  optical depth toward the Sun, with no surface found in the volume, so a sample's light falls smoothly from
  the lit side of its Sun column to the far side whatever the cloud's shape. That much is the published
  practice. The halfway turn is not: it is Thrum's own choice among the planes
  [WP3's controls](../../../.orchestrate/evidence/CLOUDS/WP3-plane/README.md) compared on Metal (part XIX), and it is the
  constant a builder tunes by eye ([risk 12](#risks)).
  No depth sets the plane. The first design inferred it at each sample, from the cloud above the sample
  against the cloud between it and the Sun. In a cloud of heaps those two depths say where the gaps are, not
  which way a surface faces: along a base the column overhead is deep and the path to the Sun slips between
  heaps, so the plane turned to face the Sun, and across the face above it the plane turned away.
  [WP3's controls](../../../.orchestrate/evidence/CLOUDS/WP3-plane/README.md) put its bright lip under a dark stripe there, at
  2.2 to 1 in solar diffuse alone, and 1.2 to 1 with the plane held level. On a uniform box the inferred plane is the
  closer of the two (part XIX), and no sky is made of boxes.
- **The view boundary.** A column is unbounded sideways and a cloud is not. Every surface the eye sees is a
  boundary, and at a boundary diffuse light falls and flows outward. Where the view ray entered through the
  lit plane or the far one, the column's own solution already ends there, and `q` is 0: in a level layer seen
  from above or below it is 0 at every sample. Where it entered anywhere else, `q` rises to 1, and the light
  is lowered toward its value at a free boundary, over a depth `ℓ` that the column's own depth sets, with the
  outward flow toward the eye added at a mean cosine of 2/3. Without it the flanks of the three larger boxes
  of part XV were 1.4 to 3.3 times transport under the inferred plane; with it, 0.9 to 1.2. `B` is the **boundary weight**, a function of
  the sample's distance from the camera alone: 0 out to the end of
  [the handover's ramp](#condensate-that-meets-the-ground-mist-fog-and-low-cloud), rising smoothly to 1 at twice
  that distance. Every heaped cloud is beyond twice that distance and takes the whole of it. A profile without
  Ground keeps the same two distances, so a cloud's light does not change when a scene adds mist.
- **One boundary rule, for every consumer.** The trace, the medium coefficients and the sky overlay evaluate
  the term at the boundary weight of the sample's distance from the camera, and that settles three cases
  ([part XVII](#check-that-it-can-work)).
  *Nearer than the ramp's end* no sample has a view boundary, whether the camera stands in the condensate or
  outside it. A boundary at the eye is wrong for a camera in the medium, distance does not say which camera
  this is, and two integrators draw one bank there, so the term reads no view depth: it is a function of the
  sample's place, the Sun and the direction to the eye, and it is the same number whichever integrator asks. A
  bank seen from outside at that range is lit on its flank as a column is, brighter than the boundary would
  make it: by 19% in the review's bank.
  *Beyond the ramp's end* only the trace draws, so the view depth is its own march's. That march holds its far
  share of the condensate from the start of its interval and nothing nearer, so a far bank seen through mist
  near the camera takes a little more boundary than the whole ray would give it: at most 0.15 of
  [the yardstick](#how-the-module-is-proved), under 1% of its radiance, with the camera in mist or behind a
  near bank, and 0.04 with the camera inside a layer that reaches every way.
  *A bank the camera walks up to* brightens on its flank as the weight falls: by that 19% between 1,000 and
  500 m, about 0.5% in 10 m at its steepest, in the one integrator, and by nothing as it crosses the ramp.
- **How far it holds.** In a level layer the reflected share is within 0.017 of transport from a 45° Sun up
  and within 0.05 from 4° to 20° at depths of 2 and more, and low wherever it is more than 0.01 out (part XII). Radiance by direction is
  within 0.62 to 1.18 of transport at 45° and 90° from a depth of 8, and 0.47 to 1.34 at a depth of 2; the
  top of a layer under a 10° Sun, seen at a graze, is 0.56 to 0.58, and face on 1.6 to 2.7. The flux of the radiance
  the term renders is 0.91 to 1.10 of the Sun's (part XIII). Over the faces of uniform boxes 12 to 120 optical
  depths across, at a Sun of 6° to 70°, with a heaped type's plane it is 0.55 to 5.27 of transport and
  typically a factor of 1.75 out: 1.48 at Suns of 45° and 70°, and furthest out on the far side at Suns of
  15° and 6°, which it draws 1.6 to 5.3 times too bright (part XIX). The plane inferred at each sample was
  typically 1.35 out on those boxes; the four orders this replaces are typically 40 times too dark there, and
  octaves at halves 14 (part XV).
  Inside 2° of the Sun through cloud of depth 8 it is 1.5 times too bright, because the narrow lobe's own
  widening is left out.
- **Once a ray.** A ray has one angle `θ` to the Sun's centre, so both phases are one evaluation a ray: a read
  of the table and one closed form, in f32 from `sin²(θ/2)` so the forward peak keeps its digits. No sample
  pays for a phase.
- **Ambient.** Sky light from above and ground light from below, each dimmed by the diffuse transmittance of the
  cloud between: `1 / (1 + ¾(1 − g)·tau)`, the two-stream result for a layer that scatters without absorbing,
  with `tau` above and below the sample taken from the column already read. Bases darken and never go black. The
  two hemisphere irradiances are integrated once per update from the lent **sky radiance**, which never contains
  clouds, so cloud light does not feed itself.
- **Night.** With no Moon in scope, clouds are lit by the night sky alone and read as dark shapes that hide
  stars.
- **Exposure.** The lent scale, once, as the cloud is written to the scene.

### The visible layer

The cloud layer is traced at part of the render extent and reconstructed over frames, in three passes in
`EFFECT-GRAPH`'s `after-opaque` stage. They run there so the trace can skip rays the scene already covers.

1. **Trace.** Each frame, one texel in each block of the layer is marched (a 2×2 block of a half-extent layer in
   the first profile: one ray for every sixteen render pixels). A ray steps across empty air on the coarse map,
   takes steps that lengthen with distance inside cloud, and stops when it is opaque. It yields, at each stop
   below, the segment's transmittance `T`, its scattered radiance `S` already composed with the air in parts,
   and a front distance averaged by transmittance.
2. **Resolve.** Every texel of the layer's history is written each frame: the fresh sample where one was traced,
   otherwise the previous frame's value, reprojected through the cloud's own position and wind drift and clamped
   to the fresh samples around it. The history is a ping-pong pair whose producer runs every frame. On a reset
   the layer is filled from the frame's trace alone and is sharp again within one refresh cycle (four frames in
   the first profile). A stop that ends on a surface follows the stricter rule under **Carried stops** below.
3. **Compose.** At render extent: `colour = colour·T + S`. For a ray whose condensate lies in one part, which
   is every ray through one cloud, that is `colour·T + T_air·S_c + (1 − T)·S_air`, where `S_c` is the cloud's own
   scattered radiance and `(T_air, S_air)` is the lent aerial transport to the front distance. This applies
   aerial perspective to the cloud once and removes the part of the background's own in-scatter the cloud
   hides.

**Not a private upscaler.** The port decision forbids a cloud path from render extent to display extent, and
lists the march's refresh fraction and resolution among its knobs. The layer ends at render extent; the engine's
reconstructor does the rest.

**The interval a ray marches.** A ray marches condensate over one interval: from where its
[far weight](#condensate-that-meets-the-ground-mist-fog-and-low-cloud) leaves zero, or where it enters the
weather's extent if that is further, to the scene's depth or the weather's far edge. In a scene with no ground
condensate, a camera below the lowest base and geometry below it, that interval is the cloud band and every ray
meets cloud before geometry or not at all, so WP3 to WP10 build and judge it there. The trace is written over the
general interval from WP3, so that WP15 changes what it is given and not how it marches.

**Geometry: a stop at every pixel's depth.** A layer texel stands for a block of render pixels, four in the first
profile, that may lie at different depths. Its ray is marched once, to the farthest of them, and as it passes
each pixel's depth it keeps the pair it has reached there, with that distance: one **stop** for each pixel. A
stop is the whole transport along the texel's ray from the start of the interval to that pixel's own surface. It
holds everything in front of the surface and nothing behind it, wherever the condensate lies along the ray, and
nothing is interpolated between stops. A pixel composes its own stop, so a ridge in front of far mist has a clean
edge. The nearest and farthest pairs alone cannot do this: a bank just in front of a middle surface and the same
bank just behind it leave the same two pairs, and blending them gives the middle pixel a transmittance of 0.77
or 0.67 where the truth is 0.30 and 1 (part VII). The ray is the texel's and not the pixel's, under a pixel's
width to the side, as it is for the sky. In a profile without Ground every pixel of a texel is beyond the
condensate or is sky, a texel keeps one stop, and the compose is the depth test it was. Water writes depth and
is geometry here.

**Carried stops.** A stop traced in an earlier frame is carried with the surface it ended on, by the scene's
motion for that pixel; a stop that ended in the sky is carried by the cloud's own, as above. A pixel composes a
carried stop only if it was taken on the pixel's own surface: its distance agrees with that surface's distance
in the frame it was traced, within the profile's **stop tolerance**, a share of the distance. Where no carried
stop passes, at a disocclusion, at a mover's edge and after a cut, the texel is traced in that frame; a stop
that ends on a surface is never filled from a neighbour's. In fog that stands still and is even along the ray,
a carried stop is then wrong only by the fog the
camera's own travel has crossed since it was traced, at most one refresh cycle before: 0.27 m at 8 m/s, which is
0.8% of the stop's transmittance in fog of a 130 m visual range and 2% at 50 m (part VII). That figure covers
the camera's travel alone. A bank that drifts or thins across a surface that does not move changes the stop
with no change in the surface's distance, so the distance test passes it; a carried stop is then also out by
what the bank changed in one refresh cycle, which no figure here bounds. WP15's motion inspections judge it,
and tracing every texel every frame is the fallback.

**Frame contract.** Clouds write no depth. Where a cloud holds most of a pixel it writes motion from its front
point, with wind drift, at unjittered positions; elsewhere the sky's motion stays. It writes reactive coverage
by opacity where the cloud changed beyond what motion explains. A cut, a weather seek, a profile change and a
library change mark the layer's history invalid; a knob applies the invalidation it declares.

**Segments, and the air in parts.** The trace's segment is `(T, S)` over its interval, which begins at or beyond
the handover. Nearer than that, condensate is in `ATMOSPHERE`'s view volume, integrated jointly with the air;
the two are ordered along the ray and their weights sum to one (C08). The air inside the trace's interval is
`ATMOSPHERE`'s lent transport, and the trace takes it **in parts**. A part of a ray ends where the air's own
optical depth across it, read from the lent transmittance, reaches the profile's **part bound**, 0.25 in the
first profile. Each part's condensate is composed with the air at that part's own front distance,
`T_air·S_part + (1 − T_part)·S_air`, and the parts are composed in order, so each stretch of condensate meets
the air in front of it once. A part with no condensate costs nothing. In clear air a part is 15 km long, so a
ray through one cloud is one part.

One part for a whole ray is the approximation this replaces. Its error grows with the air's optical depth
between the condensate nearest the camera and the furthest: for a thin bank at 600 m with a thick one at 5 km
behind it, the near one in shadow, it is 2.6 times [the ceiling](#how-the-module-is-proved)
in haze of 8 km and 2.1 times at 20 km. With the part bound at 0.25 the same rays, a deep bank and thin mist to
the horizon are all within a quarter of the ceiling, in haze from 3 to 244 km (part X).

### The sky the world sees

A sky with clouds must light the world as one. Three small products do it, each applied by `ATMOSPHERE`
([rulings 2 to 4](#what-clouds-asks-of-other-modules)):

- **Far aerial.** The lent aerial transport stops at 32 km and clouds reach 200 km. For a point further along a
  ray that ends in the sky, transmittance is the ratio of the transmittance chart at its two ends, and in-scatter
  is the sky's radiance in that direction apportioned by optical depth. Exact at infinity, continuous at 32 km.
- **The sky overlay.** A small cube of cloud `(S, T)` around the camera, traced in parts with the coarse density.
  `ATMOSPHERE` composes it over the sky before it filters its reflection and diffuse products. So water and
  polished surfaces reflect clouds, and ground under an overcast sky is lit grey and dimmer, not by a clear blue
  dome.
- **Haze under cloud.** One number a frame scales the air's solar in-scatter: horizontal illuminance at the
  camera with clouds over the same without. It is Frostbite's equation 22 as a scalar, with no shafts and no
  colour shift. It is what the aerial product takes in a scene without `ATMOSPHERE`'s view volume. With the
  view volume the air is lit sample by sample through the cloud factor, shafts included, and takes no scalar.

### Placed volumes

A placed volume is a cooked density volume that stands for one cell.

A placed cell is a cell of the weather field whose cloud is an authored shape and not a cluster of turrets. The field knows
the shape, so the query, the picture and the shade agree on its outline at any distance and with no camera.

- **Content.** The game cooks its library as one stacked volume product: each shape is a slab of the same
  power-of-two size along one axis, and one texture serves all shapes. A growth track is a second stack whose
  slabs are keys in time. `CONTENT-LOOP` lends both through a lease, unchanged. With each shape the game gives
  its physical extent, base height, downwind axis and **footprint**.
- **The footprint** is the shape's own column: for each vertical column of its voxels, their sum and the lowest
  and highest occupied height. `thrum-cloud-model` derives it from the voxels with one function, the game keeps
  it with its placement data, and the library's footprints are an input of the field like the type table. A
  headless server needs the footprints and never the volumes.
- **Placement by the field.** A share of cells, set by the type table, are placed cells, chosen by hash, each with
  a shape the hash picks: centred in its cell, its base on the shared condensation level, turned to the wind by
  yaw, scaled uniformly. A voxel value of 1 is the type's extinction. At most one placed cell is chosen in each
  block of lattice sites, a profile constant, and its footprint is held inside the block, so no point has two
  placed shapes over it. A share of 0 is a sky of weather alone, and a share of 1 a sky whose every heaped cloud
  is a shape. None of this depends on the camera.
- **Placement by the game: directed shapes.** A profile places shapes by the field's hash or by the game, never
  both, because the weather map names one shape a texel. In a directed profile the placed share is 0 and the
  weather state holds a bounded list (16 in the first profile) of directed shapes. Each has a stable id, a centre
  in the air frame, a library shape or growth track, a yaw, a uniform scale, and two targets from 0 to 1: its
  **envelope** and, on a track, its **growth**. The state moves both toward their targets over the response time,
  as it does every parameter. A shape is added at envelope 0 and dropped when its envelope and its target are
  both 0, so it forms and thins and never switches. Its base sits on the shared condensation level, and it drifts
  with the wind like every cloud; `thrum-cloud-model` turns a world point at the current tick into the air frame.
  A directed shape is a placed cell in everything below: its column is its footprint and adds to the cells and
  deck around it. The state refuses a shape whose footprint would overlap another directed shape's, or one beyond
  the limit, and returns the refusal to the game, counted. With coverage at 0 the sky holds the directed shapes
  alone.
- **The query.** Over a placed cell the field's column is the footprint read at that point, scaled by the cell's
  envelope, and it adds to whatever other cloud is there. That is the bound of
  [what a query promises](#one-weather-field-in-the-air-frame), with the shape's true outline: zero outside
  the silhouette, the shape's own depth inside it.
- **Two forms, one march.** A placed cell is drawn in two forms of the same condensate. The **volume form** reads
  the lent slab; its uneroded column is the footprint, exactly, because a column of voxels sums to its footprint
  texel. The **column form** spreads the footprint's column over the type's vertical profile between the
  footprint's lowest and highest heights, like any weather-layer column. The weather map texel names the slot
  and the volume form's weight, which is 1 near the camera and ramps to 0 across the edge of the placed range;
  the two weights sum to one. Each form is bounded by the footprint at every point, so their blend is at every
  weight, and what is drawn over a point never depends on the camera by more than the bound allows. The same
  function feeds the Sun's path and the overlay, so a placed cloud is lit, shades the ground with its own
  outline and is reflected like any other.
- **A shape's shade is its volume's.** The Sun's path integrates the coarse density of the forms that are
  drawn, with the weights the weather map names: the lent slab wherever the volume form has weight, the column
  form elsewhere. A footprint cannot stand in for the slab there. It holds each column's sum and its lowest and
  highest heights, and not how the density lies between them, so two shapes with one footprint can differ along
  a slanting ray: in the review's pair, a transmittance of 0.02 against 0.57 at a 45° Sun, where a path made
  from their footprint gives 0.24 for both (part H). So Shade needs the volumes wherever a scene declares
  Shapes, with the trace or without it. The map writes the slot and the weight from the camera, the placed
  range and the lease, and never from whether the trace is declared; a scene with Sky off therefore integrates
  the same slab with the same weights, and its ground field is the same bytes.
- **Read at the slab's finest level.** The Sun's path samples the slab along each column's own ray, at the
  level the voxels were cooked at. A slab averaged to the column's width spreads an opaque shape's shade past
  its outline: for a cumulus of 20 m voxels under 80 m columns, the ground factor is 0.01 to 0.05 RMS from the
  volume's own with the finest level and 0.11 to 0.18 with the averaged one at the type's extinction (part H).
  Between columns the product interpolates optical depth, as it does for every cloud.
- **A cell in its column form is shaded through it**, in every profile: past the placed range, beyond the
  capacity below, and while no lease is resident. That shade has the footprint's outline straight down and its
  heights, and is 0.04 to 0.14 RMS from the volume's own on the ground under a slanting Sun (part H). It is a
  stated approximation, used where the cloud itself is drawn that way.
- **Reading a slab.** The march samples no level coarser than the one that holds one texel a slab, and holds its
  coordinate along the stacking axis half a texel of the coarser level it reads inside the slab. Up to that
  level every texel is an average of one slab alone; the next level averages neighbouring shapes, and is never
  read. A cloud too far for that level is past the placed range and drawn in its column form. Growth keys are
  read the same way, and two keys are blended by weight, never filtered along the stack.
- **Life.** The cell's envelope drives both forms' opacity and edge erosion, so the cloud forms and thins with
  its cell. A growth-track instance blends its two keys around its age, or around its growth where the game
  directs it, and its footprint is the same blend of theirs.
- **Capacity.** The number of volume-form instances is a profile limit checked against the capability record.
  Placed cells beyond it, and every placed cell while no lease is resident, are drawn and shaded in their
  column form, nearest first, and the refusal is counted and reported. The query is the same either way.

### Condensate that meets the ground: mist, fog and low cloud

Fog is cloud at the ground: the same droplets in the same field under the same light. It is not a second system.
The weather field gains sources anchored to the ground, its density and its Sun's path reach down to the ground,
and the picture is made by two integrators that share that one density and one light: `ATMOSPHERE`'s view volume
near the camera and this module's trace beyond it.

- **The surface field.** Ground condensate needs to know where the ground is. A game that declares the Ground
  capability gives the field a **surface field**, as it gives the type table: the height of the ground and, where
  there is water, the water's level, on a grid over the weather's extent in the world's frame. The model reads it
  on the CPU and the GPU reads a texture of it, anchored to the world and written once; the engine never reads
  terrain meshes. From it the model derives
  the **datum**: the surface averaged over a wide window, a profile constant (3 km in the first profile), which
  is the level that cold air pools to. A hollow is ground below the datum and a ridge is ground above it.
- **Ground sources.** A **fog table** is game data like the type table. Each row is one kind of ground
  condensate: its liquid water content and droplet radius, which give its extinction and phase exactly as a
  cloud type's do; its depth and the softness of its top; its **pooling**, from 0, a layer of even depth over the
  surface, to 1, a level top over the datum that fills hollows and leaves ridges clear; whether it is **held**
  to the ground or **drifts** in the air frame; its affinity for land or for water, which fades across a margin at the shore; and how far noise breaks it
  into banks and wisps. The first profile has three rows: valley mist (pooled, held, over land), a fog bank
  (even, drifting, over water and the shore) and steam mist (shallow, wispy, over water alone). The weather state
  holds an **amount** from 0 to 1 for each row, a parameter with a target like any other. Amount raises the
  top and lowers the threshold the row's noise is cut at, together, so mist swells in from the deepest hollows
  and the densest patches and never switches on (part II).
- **What stays with the world and what rides the wind.** A row's density at a point is a product of two kinds of
  factor, and each keeps its own frame. Its **world factors** are functions of the world point and the surface
  field and never move: the point's height over the surface there, in which the row's depth and soft top are
  measured; the datum, for a pooled row; and the affinity for land or water, with its fade across the shore
  margin. Its **air factors** are functions of `world point − travel` and weather time, like every cloud: a
  drifting row's **extent**, the pattern that says where its banks stand, and every row's noise. A **held** row
  has no extent. It stands wherever its world factors put it and only its wisps cross it, as valley mist does
  in a light wind. A **drifting** row is its extent times its world factors, so a bank moves with the wind
  exactly while its foot follows the ground it crosses and its edge stays at the shore: a bank over water comes
  up to the beach, thins across the margin and goes no further inland, at any travel. As one identity, which
  WP13.3 tests: a row under a travel, over a surface field, equals the row with no travel at `point − travel`
  over the same surface field moved by that travel, bit for bit (part VIII). A row that carried its shore with
  it would put fog 100 m inland after 200 m of travel, which is what a row made of the air-frame point alone
  does.
- **Low cloud is the deck.** A layered type's base may lie anywhere down to the ground. Where the ground stands
  above the base, the ground is in cloud. The field's density is a function of a world point and does not know
  where the camera or the ground is, so this adds no source; what it changes is the
  [interval a ray marches](#the-visible-layer).
- **The day.** The weather state holds one more integrated value, the **ground cooling**, from 0 to 1. Each tick
  it moves toward a target that is high when the Sun is down, the sky is clear and the air is calm, and it falls
  after sunrise at a rate that grows with the Sun's elevation. The step takes the Sun's elevation as an input,
  from the environment source's time, so a headless server agrees with the picture. A row marked **radiative**
  takes its amount from the ground cooling and the state's **moisture** parameter through a smooth threshold;
  any other row takes the amount the game sets. So mist forms after dusk, lies deepest at dawn and burns off,
  and a cloudy or windy night has none (part III). The constants are the profile's.
- **Haze.** The weather state holds the **visual range of the clear air**, 3 to 300 km, and the haze's
  **Ångström exponent**, 0 to 2, which says how the aerosol's extinction falls with wavelength: 0 is grey, as
  large, wet particles are, and 1 to 1.5 is dry, fine haze or smoke. Both move toward their targets like any
  parameter, and the exponent's default is 0. Haze is aerosol, not condensate, and `ATMOSPHERE` owns what the air
  does with it ([ruling 8](#what-clouds-asks-of-other-modules)); the model turns the range into the aerosol
  extinction at 550 nm that module takes, and passes the exponent as it is. The visual range is defined at
  550 nm, so the exponent does not change it or any answer of the query. Tying the exponent to the state's
  moisture is the game's choice; the model does not tie them.
- **The query.** A point query also answers, for any world point at any height: the condensate's extinction in
  m⁻¹, the visual range there (3.912 over the extinction of condensate and haze together), and through the
  model's f64 reference the optical depth between any two points, which is the integral of that extinction
  along the segment. A game asks whether one point can see another with no graphics device. The extinction is
  the [coarse density](#one-density-in-m¹) at the point, which the model evaluates in f64 with no texture.
  Cells, the deck and ground sources are functions the model holds whole, so through them the answer is that
  density exactly. A [placed shape](#placed-volumes) is content, and the answer depends on what the model was
  given of it. Given the shape's voxels, as WP1's Sun-path reference takes them, it is the volume form: the
  shape itself. Given its footprint alone, as a headless server usually is, it is the column form. That is
  exact in the column's sum, in its lowest and highest heights and in its outline, so it is zero outside the
  silhouette and above and below the shape, and it is an estimate of how the density lies between those
  heights, with no bound. Two shapes of one footprint, one dense at 15 m and the other empty there, have level
  sight lines of 0.05 and 1 through their voxels, and one answer, 0.17, from their footprint (part IX). So
  every answer says which it is: **exact**, or **from a footprint** when any part of it crossed a placed shape
  the model knows only by its footprint. A game for which sight through a placed cloud matters gives the
  voxels. Column optical depth includes the ground sources and keeps
  [what a query promises](#one-weather-field-in-the-air-frame).
- **One density, one Sun's path.** Ground sources add to the one density, in its full and coarse forms, before
  attenuation. The Sun's path gains a **ground band**: levels of camera-following windows over the surface
  field, each holding optical depth toward the Sun at a few heights above the surface, filled by marching the
  coarse density along the path direction from each node to where the ray leaves the band, plus the cloud band's
  value there. The band's columns are in the air frame and move with the display's travel, as the cloud band's
  do, because most of what they hold rides the wind: banks, and the clouds above. Its heights are measured
  over the surface at the receiver's own place, which is the world's. So what a held row adds to a node is
  carried downwind until the node is next written, by at most the travel inside the band's age bound: 20 m
  in a 40 m/s wind in the first profile, a quarter of a column. A point's `tau` is then one read: the ground
  band below its top, the cloud band above. A band of
  80 m columns and 9 heights holds a bank's Sun path to 0.01 RMS in transmittance where a level-layer rule is
  off by 0.04 to 0.31 and casts no shadow beside the bank (part V). While it marches, the fill also records
  where the surface field itself blocks the Sun, as a **surface visibility** kept apart from `tau`. Only medium
  samples beyond the reach of the scene's shadow cascades read it, so a ridge's shadow lies across far mist at
  dawn; no surface ever reads it, because surfaces have the scene's own shadows.
- **Near the camera: the view volume.** Fog near the camera meets the ground, water, props and the camera
  itself, and overlaps the air, which C08 requires to be integrated jointly. `ATMOSPHERE`'s view volume does
  that ([rulings 6 and 7](#what-clouds-asks-of-other-modules)). Each frame this module writes, for every cell
  of that volume, at the jittered point the volume names, three **medium coefficients**: the extinction `σ`;
  the solar kernel `σ · (S_beam + S_diffuse)`, which is [cloud light](#cloud-light)'s Sun term without the
  Sun; and the ambient source, cloud light's ambient term. The kernel comes from the one shader function the
  trace calls, with the inputs the trace gives a sample at that point: `tau_sun` and `tau_col`
  through one shader function for the depths at a world point, which reads the Sun's path (the ground band,
  which holds the whole path, below its top and the cloud band above); the plane's turn, which is 0 for
  ground condensate; the path direction; and the direction from the point to the camera. The boundary weight is 0 wherever the near
  weight is above 0, so the pass gives no view depth and needs none. Each is multiplied by the **near
  weight**. `ATMOSPHERE` multiplies the kernel by its Sun light at the point and by the scene's shadow there,
  adds the air's own coefficients and integrates once. This module never integrates the near field, and
  `ATMOSPHERE` never evaluates a density, a phase or a self-shadow.
- **Beyond it: the trace.** The trace marches every condensate, ground sources included, weighted by the **far
  weight**, lit by cloud light through the same two functions at the boundary weight of each sample's distance,
  so inside the ramp it evaluates the same source as the view volume, with the surface visibility on its Sun
  term, stopping at the scene's depth with
  [a stop at every pixel's depth](#the-visible-layer) and taking the air in parts. A far bank is then drawn at
  the layer's grain, a texel for every four pixels, where the view volume's cells are sixteen pixels across.
- **The handover.** The near weight is 1 from the camera to the start of a ramp and falls to 0 at its end; the
  far weight is one minus it. The ramp is a profile value, 300 to 500 m from the camera in the first profile.
  Two integrators that each take a share of one extinction compose as ordered segments to the exact
  transmittance, and to the in-scatter within 0.002 for a ramp twice that long in part I's uniform medium.
  The in-scatter's error depends on what stands in the ramp: a bank that fills a 300 to 500 m ramp is 0.011
  in 0.65 (`ATMOSPHERE`'s design review). That holds because both integrators take one source there: with a
  view boundary in the trace alone, a bank brightened by up to 19% as it crossed the ramp
  ([the review of 17 November](../../../.orchestrate/evidence/CLOUDS/design-review.md), finding MF1), which the boundary weight
  removes. With it the review's bank is the same radiance at 200, 325 and 400 m, to rounding, and over every
  case of part XVII the two integrators are within 0.17 of the yardstick of one integrator over the whole
  ray: a bank with soft edges in the ramp, a long bank across it, and the camera inside condensate. Beyond the
  ramp the boundary comes in over a distance as long as the ramp's end, in the one integrator, so nothing
  steps. WP15.13 judges one bank drawn by the view volume and by both integrators, side by side, and WP15.4 the walk
  through the ramp. The distance is short
  for three reasons: fog thick enough to hide the land is all inside it; every heaped cloud base is beyond it,
  so no cumulus is ever drawn at the volume's cell size; and a low deck overhead is featureless at that range.
- **The world's light.** The [sky overlay](#the-sky-the-world-sees) traces every condensate from the camera, so
  inside fog the land's ambient light is flat and reflections show fog. The Sun on a surface in fog is dimmed by
  the ground band through `ATMOSPHERE`'s one factor.
- **Without Sky.** A scene with Ground and no Sky has fog to the end of the ramp and none beyond, which is what a
  top-down game sees. It says so.

### Capabilities, shadow-only and headless

A scene's cloud profile names the capabilities it uses. One it leaves out declares no pass, resource or pipeline
and costs nothing; changing the set rebuilds the effect and starts a new epoch.

| Capability | What it adds | Needs |
|---|---|---|
| **Weather** | The state, its step and the point query, in `thrum-cloud-model`. No graphics device, nothing allocated | Always present |
| **Shade** | The weather map, the shape noise and the Sun's path, with the ground field `ATMOSPHERE` applies | Weather |
| **Sky** | The visible layer: the trace, the layer's history, the detail noise, the compose and the far-aerial use | Shade |
| **World light** | The sky overlay and the haze scale | Sky |
| **Shapes** | The library: its footprints, which the field's query reads (its voxels too, where a game gives them to the model), and its leased volumes, which the Sun's path reads for a shape's shade and the trace, with Sky, for its picture. By the field's hash or by the game | Shade |
| **Ground** | Condensate that meets the ground: the surface field and the fog table, which the field's query reads; the ground sources in the weather map; the Sun path's ground band; the medium coefficients `ATMOSPHERE`'s view volume integrates. With Sky, the trace carries the same condensate beyond the handover and the overlay includes it; without Sky it ends at the handover | Shade, and a scene that declares `ATMOSPHERE`'s view volume |

- **Headless** is Weather alone: a server answers point queries, and needs a library's footprints and never its
  leased volumes. With ground sources it needs the surface field and the fog table, and answers extinction,
  visual range and the optical depth between two points, each marked exact or from a footprint as
  [the query](#condensate-that-meets-the-ground-mist-fog-and-low-cloud) defines. A server that wants exact
  sight through placed shapes gives the model their voxels as plain data; it still needs no graphics device.
- **Shadow-only** is Weather and Shade, with Shapes if the sky has them. Leaving Sky out releases what only
  the picture uses: the trace, the layer and its history, the detail noise, the compose and the overlay. The
  library is not among them, because [a shape's shade is its volume's](#placed-volumes); a shadow-only scene
  with shapes holds their lease, and one without shapes holds no volume. The Sun's path integrates the coarse
  density whatever else is declared, so with the same library resident and the same camera its ground field is
  the same bytes with or without Sky and World light, under a sky whose full density the detail noise does
  erode and under placed clouds in either form.
- **A sky with no library** leaves Shapes out. No footprint, lease or slot exists, and the map's presentation
  values are not written.

### Recipes, limits and knobs

- **One weather profile on every recipe.** Lattice, period, levels and the field's meaning are the profile's,
  never a quality setting (C04). Chrome and the Mac hold the same weather.
- **Limits are inputs.** Volume and storage formats, 3D extents, sampled textures per stage and placed capacity
  are read from the capability record and refused at preparation when exceeded. Levels are array layers or
  packed regions so the trace stays inside sixteen sampled textures with the lent environment group.
- **Recipes.** Both recipes run the portable kernels first. An Apple-tile variant (f16 arithmetic, compute in
  place of a fragment pass or the reverse) is promoted only by a paired measured win with no visible difference
  in the same views.
- **Knobs.** Each is a named runtime value in `EFFECT-GRAPH`'s list with a declared invalidation: the trace's
  refresh fraction, the layer's scale, step length and sample cap, local light samples, the Sun's path cadence
  per level, weather map cadence, overlay cadence, placed range, the ground band's cadence and the medium coefficients' cadence. The handover
  distance is a profile value and not a knob, because it decides which integrator draws a thing. Extent knobs select a sub-rectangle of a
  fixed allocation, so no knob creates a resource or pipeline. None changes the weather field, the columns the
  weather map holds or what a query returns; the placed range changes only the map's presentation values, the
  slot and the volume form's weight, and through them which form of a placed cell is drawn and shaded.

<a id="numerical-acceptance-for-ground-condensate"></a>
<a id="numerical-acceptance-for-cloud-light"></a>

### How the module is proved

The owner's bar of 17 November 2026 ([the request](#the-request), and
[the project settings](../../../ORCHESTRATE.md#what-is-proved-and-how-hard)) decides what an item may ask.

- **The look decides.** Each cue is judged in the running game against its work package's inspection list, at the
  bar of the skies and mists of Red Dead Redemption 2 and Horizon Forbidden West. Cloud light, the ground band,
  the medium coefficients, the stops, the handover and the far air are all judged this way: by what a sky, a
  bank, a ridge line and a walk through the handover look like.
- **Speed is a tripwire.** The paired cost records and their wires are in [the cost account](#cost-account). A
  record passes on its check; a reviewer reads the records once, in WP12.
- **Held refresh has one judge.** Whether the player sees a new frame at every refresh while clouds drift is
  decided once, for the weather sky under the fair preset, in WP20, by
  [the wind series](#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20) and the times the OS
  reports each frame reached the display. WP12.6 records the finished sky's presented cadence and investigates
  each refresh it misses, and decides nothing. A `realtime` inspection judges where each frame puts the
  clouds, by that frame's display time. A refresh its capture skipped fails no inspected item.
- **References are diagnostics.** `thrum-cloud-model`'s f64 marches, the retained Python references and the
  proof scripts stay in the repository. A builder whose picture looks wrong compares against them to find the
  cause. No item holds a GPU product or the Sun term to one within a bound, no work package measures a bound first
  and then asserts it, and a miss against a reference neither fails an item nor ends `BLOCKED`.
- **What a game reads is tested.** The weather field is the game's: it is deterministic, and its CPU query is
  what a server reads. So the GPU's weather map and ground condensate are tested against the query, at a
  tolerance the test states: cloud and mist stand where the query has them and not where it is clear (WP2, WP4.1,
  WP6.2, WP6.9, WP7.6, WP14.2). Presence is not enough. A GPU that draws twice the query's extinction puts mist at
  every probe the query does, and a player loses sight of a thing at half the range the game works with. So
  the test of each GPU path that WP2's map test does not reach also compares the amount, at a point inside the
  condensate: through ground condensate, the optical depth along a sight line, summed from the GPU's coarse
  extinction, against the query's optical depth between the same two points (WP14.2); over a directed shape,
  the map's column optical depth against the query's (WP6.9); and for the column the GPU draws, through a placed
  cloud's two forms and their blend and through eroded cells and the deck, that it is never more than the
  query's column, which is [the bound a query promises](#one-weather-field-in-the-air-frame) (WP6.2, WP7.6).
  Each test states its allowance for the map's filtering and shows that twice the amount fails. These compare
  two answers to one question a game asks. Neither is a reference for light, and no photon is counted.
- **Contracts are tested.** The frame contract's motion, reactive coverage and history (WP3.4, WP15.7); air and
  condensate composed once (WP3.6, WP14.3); one optical depth for every receiver, with the same bytes with
  visible clouds off (WP3.8, WP6.12, WP8.9); a capability left out costing nothing; state through pauses, hitches
  and a replaced device; and no retained image moved.
- **A wrong substitute still fails.** Where a test names a plausible wrong version, such as a shore mask that
  drifts or a cloud band's value counted twice, it runs that version through the same assertion and shows it
  fail.
- **Work packages already Done stand as built.** WP1, WP2 and WP13 were proved against f64 references and the
  yardstick below, and their tests stay in `./dev check` as regression guards. A builder who changes a model on
  purpose updates its test's expected values in the same landing.

**The yardstick the proofs quote.** Where [Check that it can work](#check-that-it-can-work) and the sections
above say "the ceiling", they mean `0.05 × expected + 0.005 × F` at a probe, with `F` 1 for a transmittance and
otherwise the largest expected value among the case's probes. The planner used it to compare mechanisms, and
WP13's tests assert it. It decides nothing in a work package that is not yet Done.

### Holding the display's refresh while clouds drift (WP19 and WP20)

K4 asks that drift and time-lapse be smooth at the display's refresh. That is two claims with two proofs. WP7
held them as one item and failed four reviews on it ([the diagnosis](LOG.md#wp7-diagnosis)).

- **Where a frame puts the clouds** is this module's mechanism,
  [travel at the display's time](#time-drift-at-the-displays-time-evolution-on-the-tick). It is right when the
  travel between two submitted frames is the wind's travel over the display time between them. A frame's
  display time is its tick less one plus its display phase, and both are on its capture record. Two
  consecutive frames that were both on time are one refresh apart in display time, and in a steady wind their
  steps are equal. A frame that reached an expired platform target consumes the time it was observed at
  ([the timing contract](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)),
  so the step into it is longer and the step out of it shorter while the platform's targets stay one refresh
  apart. In [WP7's wind-turn capture](../../../.orchestrate/evidence/CLOUDS/WP7/fix/noonwall-wind-turn/runtime.json),
  submissions 519 to 521 have targets 8.333 ms apart and display times 10.181 ms and 6.486 ms apart, and the
  unequal steps there are right. WP7.4 judges this in `realtime`
  captures, and so does every later `realtime` inspection of this module. A capture is an observer and runs
  beside other work.
- **Whether the player sees a new frame at every refresh** is the whole frame's delivery: the clouds' GPU
  work, the frame host's slots, the Mac window's pacing, the compositor and the machine. WP20 judges it for the
  weather sky, in the game as a player runs it, by the times the OS reports each frame reached the display.
  WP12.6 puts the finished sky's presented cadence on record, with the governor, and investigates each refresh
  it misses. That is a record, and it decides nothing.

**Two instruments.** They answer different questions, and only the first sees the display.

- **The presented-time witness** is `PLATFORM-RUNTIME-MAC`'s
  ([its design](../PLATFORM-RUNTIME-MAC/DESIGN.md#window-output-panel-cadence-and-trace-tooling)): the time
  the OS reports each of the game's drawables was presented, from an Instruments recording of the launched
  game, joined by drawable to the game's own records. In a run's measured interval a refresh is **shown** when
  a new frame is presented in it and **lost** when none is, so a presented interval of two refresh periods is
  one lost refresh. That is what the player sees, and the witness alone counts lost refreshes. The game's own
  times cannot stand in for it. On this Mac two frames whose targets were 8.333 ms apart, and whose present
  calls were 8.239 ms apart, were presented 16.667 ms apart
  ([drawables 365 to 366](../../../.orchestrate/evidence/render/mac-m2-20261113/README.md)): by its present
  calls the game missed nothing, and the player saw one frame twice. This module uses the witness as that
  module qualified it. Where its command is extended to read the series' runs, how it takes a presented time
  stays as it is: the game process's `ca-client-presented-handler` rows, the validated conversion, the join by
  drawable ordinal, and unmatched samples kept apart from presented frames.
- **The account** is the window's own, and it is a diagnosis. It says where the game let a refresh go, and
  never that the player saw a frame. A **refresh opportunity** is one display target the display link gives the
  window, and the targets make a grid of refresh intervals. An opportunity is **served** when the present call
  of exactly one new frame is made in its interval, and the frame is the next in display time after the one
  before it. Otherwise the account says where it went: the display link gave no callback; the request reached
  the render owner after its target; no frame slot was free, which is the GPU not having retired an earlier
  frame; the drawable came too late; or encoding and submission ran past the interval. Each frame also carries
  its display target beside the time of its present call. A run whose every present call comes just after its
  own target is a refresh behind and still hands the display one new frame for each refresh: that is the
  window's latency, and nothing went unserved.

**A lost refresh has a place.** The witness's rows and the account's are joined by drawable, so each refresh
the witness counts lost has one of six places: one of the account's five, where the opportunity went unserved,
or **after the present call**, where the account served the opportunity and the frame still reached the
display a refresh or more late, as it does when its GPU work finishes past the refresh.

**What the window keeps.** A window runs until the player closes it, and at 120 Hz an hour is 432,000
opportunities, so the account's memory does not grow with the run. For the whole run it keeps counts: the
opportunities, those served, and those unserved at each place. It keeps a row for each opportunity (its
target, the time of the present call, the place) only inside a **measured interval** that a launch asks for
by its start and its length, and the length bounds the rows: the series asks for 20 seconds after the first
second, which is 2,400 rows. A game launched without one, as a player launches it, keeps no row however long it runs.
When it closes, the window writes the counts and the rows. It marks the measured interval for the recording,
and takes no GPU timestamp for any of this. GPU pass timing names the pass that was slow, and it costs cadence
(334 of 492 opportunities with it, 492 of 492 without), so it is taken in a second run, labelled a
diagnostic, that is never evidence that a refresh was shown or lost.

**What the witness needs** is its own, and is not the account's rows. It joins the game's submission and
request records to the recording by drawable, and it covers a measured interval only with a joined presented
frame at or before the start marker and another at or after the end marker. `PLATFORM-RUNTIME-MAC`'s retained
capture-off trace qualifies with its 841 joined drawables and fails coverage when given only the 488
submissions whose targets fall inside the markers. So a launch the witness records writes those records from
the frame before the start marker through the frame after the end marker. They are bounded by the interval
the launch asked for, as the rows are, and the account's rows still stop at the interval's ends.

**One count.** WP7 added to the window the longest run of consecutive targets, from a status sampled before
encoding. It is removed. It overstated: a frame that entered submission 42.57 ms after its target stayed in
the run. It understated: a still-air run presented 5,400 frames in 5,414 callbacks and reported 0.125 s,
because 752 requests found their target expired and were left out. The governor's count of display ticks
without a present stays as `APPLE-PRESENTATION` built it. The account adds a record at exit and changes no
timing policy, frame sample or report line that the governor's proof, the profiler, `scripts/host_overload.py`
or another module's check reads.

**The wind series.** Eleven runs of the release V1 game in its 1920×1080 window on the built-in 120 Hz display:
a clouds-absent run, then each of five cloud runs followed by another clouds-absent run. The five are still
air, 10 m/s and 40 m/s at 1×, 40 m/s at 60×, and 40 m/s at 1× with a 90° change of direction ten world seconds
in. Every run uses the launch settings WP7's reviews matched (the cooked valley, the Apple recipe, TAAU, the
fair preset, noon, a held camera, the world running) with no frame limit, frame-rate cap, capture, profiler or
GPU timing. Every run is recorded by the witness, which is its one observer and the same on each, and is
measured for 20 seconds after its first second. Each records the load, the thermal state, occlusion, its
account and its presented times. A series is one whole. No run is taken again by itself or chosen from
several: on one binary WP7's display receipts served between 46% and 99.7% of their opportunities, and in
both ordered series on record the early runs served theirs and the late ones did not, whatever their wind.

**A cloud run holds** by one rule with two parts. Each part is taken from the witness's presented times over
the run's 20 seconds and set against the two clouds-absent runs beside it.

- **How many.** It loses at most one refresh more than the worse of the two.
- **How long at once.** A **stall** is a run of consecutive lost refreshes, and its length is their number:
  one presented interval of n + 1 refresh periods is a stall of n. A run's longest stall is 0 when it lost
  nothing. A stall of a cloud run **stands out** when it is more than one refresh longer than the longest
  stall in either of the two. No stall of a cloud run that holds stands out.

The count cannot see a hitch, and the stall is what the player sees as one. Three 20-second runs at 120 Hz
that each lose 30 refreshes are equal by count. If the clouds-absent runs lose theirs one at a time, their
longest interval between new frames is 16.67 ms. If the cloud run loses its 30 together at the wind's turn,
the picture stands still for 258.33 ms: a stall of 30 beside stalls of 1, and the run does not hold.

That is the sky's requirement. A cloud run that does not meet both parts is the only result of a series that
rejects the sky. The record gives, for every run, the refreshes it lost, its longest stall and when in the
measured interval that stall began.

**What a clouds-absent run loses** is the game's without clouds: the window's, the frame's or the machine's.
It fails no check and stops nothing in this module. It qualifies what the series establishes, and the record
gives each cloud run's result beside what its two neighbours lost and their longest stalls. "No more than the game without clouds,
which lost 0 and 1" is a held refresh. "No more than the game without clouds, which lost 40 and 55" says only
that the clouds added nothing to a game that was already losing refreshes. Each refresh a clouds-absent run
lost is listed with its place and written to `PLATFORM-RUNTIME-MAC`'s log as a tripwire, for that module or
the performance phase ([brief](../VISION.md), decision 3).

**Not a measurement.** A series with a run missing, a run occluded, or a run whose presented times do not
cover its measured interval is a fault of the setup. It says nothing about the sky or the game and is
recorded as that; the setup is fixed and the whole series is taken again.

**Where it is taken.** Both work packages take the series on the unlocked, awake built-in display, which
[the project settings](../../../ORCHESTRATE.md#targets) call attended setup. The display is woken for each run
and not held awake between them. WP19 takes the series beside other work, under the `gpu` lock, as a record:
it fails on a series that is not a measurement or on a lost refresh with no place, and no count or stall
decides it.
WP20 takes it once with no other agent working, and there the rule decides. WP20 is the sitting this design
asks for by name under [the project settings' rule for timing](../../../ORCHESTRATE.md#targets), because
smoothness at 1× is judged in it: its done-when entry is `"alone": true` and its check holds `gpu.alone`. The
exclusive GPU lock alone is not enough. Under it, the 40 m/s run and the turn left 267 and 349 of 2,412
opportunities unserved. A review has no quiet window, so a reviewer reads the series the script took in the
sitting. A cloud run that does not hold ends the sitting: the builder ends `BLOCKED` with the series, the
places of that run's losses and each stall that stands out, and the repair is planned as work beside other
builders, as WP19's is.

**The rule is fixed before the sitting.** WP19 builds the command that applies it, and proves it there on
scripted presented times with known answers (WP19.4), the three runs above among them, so the one attended
sitting is not spent finding that the command counted wrongly.

**Whose loss it is.** The clouds-absent runs beside a cloud run are the same game on the same binary with one
thing changed. So attribution asks one question of the whole frame: what does the cloud run lose that its
neighbours do not? The rule's two parts give two answers. Every lost refresh of a cloud run is under one of
them: a refresh in a stall that stands out belongs to that stall, and every other is counted at its place.

- **A stall that stands out is the clouds'**, whatever the counts. Thirty refreshes lost together at the turn
  are not the window's because the neighbours lost thirty, one at a time, at the same place. The stall is
  listed by itself: its length, when in the measured interval it began, what the wind was doing then, and
  the places of its refreshes. It is never given to the game or the window.
- **More than its neighbours, at any place, is the clouds'**, whatever the cloud passes take by themselves.
  A frame with 7 ms of GPU work on its critical path makes an 8.33 ms refresh. With a 2 ms cloud pass added
  it takes 9 ms and does not. The cloud passes are well inside the refresh interval and the loss is still the
  clouds', because the same frame made the refresh without them. Cloud passes that fit the interval by
  themselves clear nothing.
- **No more than its neighbours is the game's own**, once the refreshes of stalls that stand out are counted
  apart. The loss is there without clouds. `PLATFORM-RUNTIME-MAC`
  owns the Mac's present and cadence path: the builder writes the loss as a proposal in that module's log, with
  the record's rows, changes nothing in how the window paces, and goes on with this module's work.
- **An excess gets a cause when it is seen again.** An excess is either of the first two: a stall that stands
  out, or more lost at a place. Beside other work, one run can lose refreshes to another builder's burst. So an
  excess is the clouds' to fix once a diagnostic run or a second series shows it again. A stall is shown again
  when the same cloud run of a second series has a stall that stands out beginning within a second of the same
  time in its measured interval, or when a diagnostic run finds cloud work taking frames past their refresh at
  that time. The time is what tells a stall the turn causes, which comes back at the turn, from a burst, which
  does not. A diagnostic run times the passes, which changes cadence, so it may not show the loss again, or may
  show it without telling its cause apart. The record then says **not reproduced** or **not distinguished**,
  with what was run. Nothing is repaired or proposed for such an excess, it ends no work package `BLOCKED`,
  and WP20's series decides whether the sky holds. That keeps the uncertainty and clears nothing: a stall that
  stands out in WP20's series stops the sky holding whether or not its cause was ever told apart.

The clouds' remedies move when work is done: a product's refresh spread over more frames, a burst bounded, a
pass left out when its inputs did not change. The cost records give one place to look first: the largest
Sun-path refill in one frame is 5 to 17 ms at 2560×1440 under load, more than a refresh interval by itself. A
remedy that lowers a sample count, an extent or a refresh fraction is [risk 1](#risks)'s and brings the stills
and sequences it changes back for inspection.

The series is under the fair preset. The broken and overcast skies cost more
([WP7.8's record](#cost-account)), and no item holds them to the refresh. WP7.8 and WP12.5 record what each
costs beside the game without clouds, with each tripped wire investigated. WP12.3 shows the governor lowering
the clouds' knobs when the scene is loaded past the refresh. WP12.6 lists and investigates each refresh the
finished sky misses on the valley's route. What remains over the refresh there is the performance phase's
(decision 3).

### Check that it can work

From `cloud_field_proof.py`, a CPU sketch with first-profile constants:

- **K1 and K7.** As coverage rises from 0 to 1, cloudy columns rise from none through 10% and 65% to a closed
  deck, with no step.
- **K4.** A column's optical depth changes by at most 0.4 a second out of 41, with no jump at a birth, a death or
  a generation boundary. Under a 40 m/s wind for ten minutes the field equals the still field shifted by the
  travel, bit for bit. Births in a 48 km square are a Poisson process (variance over mean 1.02), and the
  spectrum of births has no peak at the generation period (0.43 of its mean). An earlier draft that jittered
  births over half a period showed a peak of 4; the full-period jitter is the fix.
- **K3.** Ground shade from a product of 80 m columns is within 0.02 to 0.05 RMS in transmittance of a 5 m
  brute-force march at 60° and 20° Sun, and 0.09 at 6°.
- **K2.** The product alone is 0.06 to 0.08 RMS off for a sample inside cloud, because a lit skin 20 m deep is
  thinner than a column; this is the band Broadsides found its coarse cache could not carry. With three local
  samples over 120 m it is 0.02. At 6° it is 0.05 to 0.08, which WP5 judges by eye.
- **Cost.** A sky ray needs 17, 42 and 34 texture reads for fair, broken and overcast skies, against Broadsides'
  142 density evaluations. That is 3 to 7 million reads a frame for a 640×360 trace before local light samples.
- **K5.** Broadsides' hybrid stills and the growth track's blind review show the content reads; that one density
  function serves both on the GPU is first proved in WP6.
- **K6.** Unrun. The layer's reconstruction and its look under TAAU are first seen in WP3 and WP9.

From `revision_proof.py`, run for the revision of 14 November against
[the first design review](../../../.orchestrate/evidence/CLOUDS/design-review-1.md)'s counterexamples:

- **The Sun's path as the Sun sinks.** Beside a broken cloud bank, with the Sun going from 8° to 1°, the factor
  under open sky, in a gap and where the ray enters the bank changes by an amount that halves when the Sun's
  step halves. The rule this replaces jumped by 1.0 at 2° under open sky and in the gap, at any step.
- **The edge of the field.** A receiver leaving the field at a 60° Sun changes by an amount that halves with
  its step; the replaced rule jumped by 0.999. The far value there is a mean transmittance of 0.50, where the
  mean optical depth would have given 0.0006. Under a closed deck the factor and the far value stay below the
  thinnest column's transmittance from a 20° Sun to below the horizon.
- **Placed shapes.** For an L-shaped tower with an overhang and a hollow, the volume form's uneroded column
  equals the query to 10⁻¹⁴ at 1,500 points; the eroded volume form, the column form and their blends never
  exceed it and are zero wherever it is zero. A dome of the same mean column, the replaced rule, is exceeded at
  26% of points, by up to six times the mean. A read one mip coarser exceeds the point query at an edge, which
  is the GPU's measured bound and not the model's.
- **The coarse density.** Along 150 m edge paths, detail erosion takes the optical depth from 3.75 to between
  3.13 and 3.41; the coarse density gives 3.27 with no noise read. Over an 80 m column its transmittance is
  within 0.001 RMS of the mean of the full density's paths, against 0.014 with erosion left out.
- **Slabs.** Twelve slabs alternately empty and full, read at levels 0 to 7 with the stated hold, return their
  own value exactly; without the hold the error is half the range, and level 8 mixes every pair.

From `first_principles_proof.py`, run for the judgement of 14 November:

- **The first order's phase.** For a 20 µm droplet the phase integrates to 1 over the sphere with a mean cosine
  of 0.88, against the reference lobe's 0.86, so the energy is the same and its direction is not. Against the
  droplet, the reference lobe is 3.9 times too bright at 5° from the Sun and 3.4 times at 10°, 46 times too dim
  at half a degree, and 2.0 to 3.4 times too dim from 140° to 180°. Within 2° of the Sun those figures used
  the widened lobe the second review refuted; part G below gives them again.
- **Drift.** At 60× in a 40 m/s wind a cloud 1,500 m off moves 37 pixels a tick at 1440 rows. Taken at the tick
  it is shown as 0 then 37; at the display's time, as 18 then 18. At 1× the same wind gives 0 then 0.6 against
  0.3 then 0.3. That holds only if the frame's time passes the committed tick with its consumed target. Taken by
  alpha, WP2's capture split each tick's 40 m as 3.2 m then 36.8 m. Recomputed from that capture's rows with
  the display phase, the six frames step 19.0 to 20.8 m against 20 m due, and the spread is the record's
  ([the reading](../../../.orchestrate/evidence/EFFECT-GRAPH/WP4-plan/README.md)).
- **Drift past the committed tick.** From [`display_travel.py`](../../../.orchestrate/evidence/CLOUDS/WP2-plan/display_travel.py),
  run on 16 November after the review of the display-phase revision, in rational arithmetic with the step of
  `state.rs`. Its four parts are cited in [time](#time-drift-at-the-displays-time-evolution-on-the-tick): the
  review's ramp, where one tick's travel carried forward steps 6 m and predicted ticks step nothing; 33,355
  pairs of frames at one display time from two committed ticks, exactly equal, where one tick's travel carried
  forward is up to 77 m out; steady wind, exact at every phase to the bound; and the correction when a target
  changes, inside the limit above in every case tried. It is the model's arithmetic and not a run of a frame.
- **Directed shapes and capabilities.** Unrun. Both are the placed cell and the profile already reproduced, with
  their inputs from the game; WP1 and WP8 prove them first.

From `second_revision_proof.py`, run after [the second review](../../../.orchestrate/evidence/CLOUDS/design-review.md), on its
counterexamples and wider cases:

- **G, the phase over the Sun's disc.** At droplet diameters of 5, 10, 20, 40 and 50 µm the disc-averaged phase
  integrates to 1.000000 over the sphere, where the widened lobe gives 0.9966, 0.9811, 0.9266, 0.8109 and
  0.7677. Its mean cosine equals the closed form to six places (0.879306 at 20 µm, against 0.879310 for a point
  Sun). At the Sun's centre the quadrature equals the closed form: 99, 1,792 and 4,325 sr⁻¹ at 5, 20 and 50 µm.
  The disc changes the point Sun's phase by 12% at half a degree, 2% at 2° and 0.2% at 5° for a 20 µm droplet.
  Against it the reference lobe is 60 times too dim at half a degree, 3.8 times too bright at 5° and 3.4 times
  at 10°. A table of 1,025 entries read with linear interpolation integrates to 1.0001 and is within 0.2% at
  its midpoints; 257 entries give 1.002 and 3%; 65 give 1.04 and are too few.
- **H, a placed shape's shade.** The review's two shapes of one footprint give Sun paths of 3.9598 and 0.5657
  through their volumes at 45°, and one path of 1.4142 from their footprint. For a cumulus of 20 m voxels with
  a dense core, a shelf and a hollow, at a 45° and a 20° Sun, thin and at the type's extinction, with hard and
  with softened edges, a product of 80 m columns filled from the volume at its finest level is 0.002 to 0.05
  RMS in transmittance from the volume's own on the ground and 0.004 to 0.08 inside the cloud, which is the
  band the product holds for procedural cloud. Filled from the volume averaged to 80 m it is 0.03 to 0.18 and
  0.02 to 0.17, and from the column form 0.04 to 0.14 and 0.06 to 0.16. The largest single differences, 0.4 at
  a hard opaque edge, are an edge placed within one column's width. Two things qualify these figures. The
  script's product takes four density reads for each slice it stores, not one. And its reference is the mean
  transmittance of seven rays spread over one column's width, not the single ray through the point. So they
  show a product built with four reads a slice against the shade averaged over a column, and say nothing of a
  product built with one read a slice or of the error at a point.

From `whole_atmosphere_proof.py`, run for the revision of 14 November for mist, fog and low cloud:

- **I, the handover.** Two integrators that each take a share of one extinction, shares summing to one, compose
  as ordered segments to the exact transmittance at any share. Their in-scatter is exact under uniform light.
  With a shadow edge or a tenfold fall of light inside a ramp 1 km long, at visual ranges of 400 m to 40 km, it
  is within 0.002 of a radiance near 1; with a 200 m ramp, within 0.0001.
- **II, pooling.** On relief of about 200 m with a datum averaged over 3 km, the mist top stands over 19% of the
  ground at amount 0, 47% at 0.4 and 89% at 1; ridges 40 m above the datum stay clear until 0.8. That is depth,
  where mist may stand, and not density: part XI below composes it with the cut of the noise. Depth changes by 0.14 m for an
  amount step of 0.001 and by half that for half the step. The top's steepest slope is 12% where the ground's
  is 71%.
- **III, the day.** With one integrated value, a moist clear calm night gives no mist at 18 h, 0.39 at 20 h, full
  from 22 h to sunrise at 6 h and none by 8 h; an overcast night and a 9 m/s wind give none; dry air gives 0.05.
  The amount changes by at most 0.022 a minute.
- **IV, the view volume.** A grid of 160×90 tiles has cells 1 m across at 100 m and 5 m at 500 m. With 96
  slices and three quarters of them inside a handover at 500 m, a cell is 2 m deep at 20 m, 10 m at 100 m and
  50 m at 500 m. The cells that hold condensate are finer than any mist top or bank edge this design draws near
  the camera.
- **V, the ground band.** Through a bank 60 m deep with a 150 m visual range, a band of 80 m columns and 9
  heights is 0.006, 0.007 and 0.011 RMS in transmittance from the true Sun path at 30°, 10° and 4°. A rule that
  treats the fog over a point as a level layer is 0.04, 0.15 and 0.31, and gives the bank no shadow on the
  ground beside it.
- **VI, haze.** `ATMOSPHERE`'s fixed aerosol is a visual range of 244 km. A range of 20 km needs 42 times that
  aerosol and 3 km needs 294 times.
- **Unrun:** the view volume's look and stability, which `ATMOSPHERE` WP7 and WP8 see first; carried stops at a
  geometry edge under reconstruction, first seen in WP15.

From `ground_revision_proof.py`, run on 14 November after
[the review of the ground revision](../../../.orchestrate/evidence/CLOUDS/design-review.md), on its counterexamples:

- **VII, stops.** In one texel with surfaces at 600, 700 and 900 m, a bank at 650 to 690 m and the same bank at
  810 to 850 m leave the same pairs at 600 and 900 m. A stop at 700 m gives 0.301 and 1.000, the truth; the two
  end pairs blended give 0.767 as pairs and 0.670 as optical depth, for both banks. In a uniform slab a stop at
  the middle is the closed form. A carried stop is 0.15% to 2.1% out after four frames of a camera moving at
  1.5 to 8 m/s through fog of 130 to 50 m visual range.
- **VIII, frames.** A drifting water row over a shore and uneven ground, through 1.3 km of travel: its density
  equals its density with no travel at the point minus the travel over the surface field moved by the travel,
  bit for bit at every probe; it is zero inland of the margin and below the surface throughout, and full over
  the water. A shore mask that drifts with the row gives the row's whole extinction 100 m inland.
- **IX, the query.** The review's two shapes of one footprint: with voxels, extinctions of 0.03 and 0 at 15 m
  and level sight lines of 0.0498 and 1; from the footprint, one column form for both, 0.0176 and 0.172, whose
  whole column is the footprint's 0.5 and which is zero above the shape.
- **X, ceilings.** Measured with [the yardstick](#how-the-module-is-proved): the
  ground band with a node's allowance, the handover with a bank across a 300 to 500 m ramp, and the far air in
  one part and in parts bounded at 0.5 and 0.25, over hazes of 3, 8, 20, 60 and 244 km, two sources and four
  rays. One part is at worst 2.6 times the ceiling, a bound of 0.5 is 0.93 of it and a bound of 0.25 a quarter.
- **XI, amount zero.** With the row's noise cut at a threshold that the amount lowers from 1, mist has depth
  under its top over 19% of the ground at amount 0 and density nowhere; at 0.1 it has density over 0.4% of the
  ground and at 0.4 over 16%. So zero amount is absent mist, as WP13.1 and WP16.1 require.
- **Unrun:** the stop tolerance, and how many texels a pan through far mist traces out of turn; WP15 records
  both.

From `multiple_scattering_proof.py`, run on 17 November after
[WP3's handoff](../../../.orchestrate/evidence/CLOUDS/WP3-handoff.md). Its oracle is photon transport: exponential free paths,
no absorption, the 24 µm droplet's phase sampled exactly, a point Sun, every photon followed until it leaves.
The model in it is [cloud light](#cloud-light)'s Sun term, written apart from any engine code.

- **XII, a level layer's flux.** Over Sun elevations of 4° to 90° and optical depths of 1 to 128, 100,000
  photons a case, the column's returned share `1 − T` is within 0.017 of the photons' at 45° and above, 0.043
  at 20°, 0.050 at 4°, and 0.041 at 10° from a depth of 2 (0.073 at a depth of 1). It is low wherever it is
  out. With Eddington's own escape function in place of the thick layer's it is up to 0.11 low at 4° and 10°.
  At WP3's six cases the four fixed orders return 0.04 to 0.22 and octaves at halves 0.12 to 0.58, where
  photons return 0.53 to 0.92; from a depth of 16 to 64 neither changes by 0.04, where the photons' share
  rises by 0.14 to 0.30.
- **XIII, a level layer's radiance.** Averaged round the azimuth in five bands of exit direction, at Suns of
  10°, 45° and 90° and depths of 2, 8 and 32: the figures in [cloud light](#cloud-light). The flux of the
  radiance the term renders, which the closed form does not constrain, is 0.91 to 1.10 of the Sun's, and 0.97
  to 1.05 at a depth of 32.
- **XIV, light through thin cloud.** Under an overhead Sun at depths of 1 to 16, transmitted radiance in seven
  rings from the Sun: the term is 0.73 to 1.26 of the photons' beyond 2° and 1.06 to 1.51 inside it. Without
  the middle lobe it is 0.45 to 1.01 beyond 2°, and with the beam as the first order alone 0.20 to 1.00.
- **XV, a cloud of finite size.** Uniform boxes of 12, 48 and 100 optical depths a side and one of 120 by 120
  by 24, at Suns of 70°, 45°, 15° and 6°: the mean radiance of the Sun-facing side, a flank, the far side and
  the base toward a camera 15° below level, and the base from straight below. Over the 64 faces the term is
  0.62 to 2.35 of the photons' and typically a factor of 1.35 out. Without the view boundary it is 0.69 to
  3.25 and 1.65; with the lit plane held level, 0.09 to 3.51 and 2.31, the Sun-facing side at a 6° Sun being
  a tenth to a quarter of transport. The four fixed orders are at most 0.23 and typically 40 times too dark; octaves at
  halves at most 0.68 and typically 14. The largest excess left is the far side at a 6° Sun and the base under
  a 70° Sun, both dim faces, at about twice transport.
- **XVI, probes.** The term's value at sixteen stated inputs, and the level layer's identities: the lit plane
  is level and its cosine is the sine of the Sun's elevation, to nine places. The probes are of the plane
  inferred at each sample, as parts XV and XVII drew it; part XIX holds the plane the design now has, and
  part XVII's cases are level layers, where the two planes are one.
- **Unrun:** a droplet other than 24 µm; the disc-averaged phase in place of the point Sun's; ground
  condensate, whose depths WP14 takes from the ground band. The term in a cloud whose density is not uniform
  was unrun here and WP3's stills saw it: part XIX.

From `handover_light_proof.py`, run on 18 November against finding MF1 of
[the review of 17 November](../../../.orchestrate/evidence/CLOUDS/design-review.md), on its bank and wider cases. Each case is
one view ray drawn by the two integrators, each lighting its share with the term at the boundary weight,
against one integrator over the whole ray with the view depth counted from the camera. Errors are shares of
the yardstick, which is 5.5% of the reference radiance.

- **XVII, one Sun term across the handover.** The review's bank, 50 m along the sight line at a 50 m visual
  range with the camera outside it: the same radiance with its front at 200, 325 and 400 m, to rounding, where
  the reviewed revision was 19%, 19% and 14% out, 2.6 to 3.5 times the yardstick. With its edges tapered:
  0.17 at worst, in the ramp, which is the composition of part I and not the light. With the camera inside a
  layer that reaches every way, at visual ranges of 50 m to 5 km, a 45° and a 10° Sun and three directions:
  0.04. With the camera in mist that ends before the ramp, or behind a near bank, and the bank beyond the
  ramp: 0.15. A bank 850 m long across the ramp and the rise of the weight: 0.02. Counting the trace's view
  depth over the whole extinction it reads, in place of its share, changes none of these by 0.02. As the
  camera comes up to the review's bank its flank brightens by 19% between 1,000 and 500 m, by 0.53% in the
  steepest 10 m, and by nothing inside 450 m. Transmittance is exact throughout.
- **Unrun:** the ground band's and the map's own errors in the three depths, which are the same read for both
  integrators; the two grains, a cell sixteen pixels across against a texel of four. WP15.13's stills see both.

From `cloud_shape_proof.py`, run on 18 November after WP3 stopped on repeated domes and shelves
(`git show 6c696409396a:docs/evidence/CLOUDS/M3-handoff.md`). It makes the fair sky's cells and their density
as `build/clouds` computes them at that commit, and draws the same cells from the side in three forms
([the picture](../../../.orchestrate/evidence/CLOUDS/design-proof/cloud-shape-proof.png)). An outline is the top of a cloud's
silhouette across its width, both scaled to 1.

- **XVIII, what gives a heaped cloud its form.** Sixteen mature cells, each alone. With nothing eroded, the
  built column's outline is `(1 − x²)^0.7` for every cell, whatever its size, depth or age: one outline, which
  three looks at other pitches, depths and tapers could not change. Its 200 m noise moves that outline 0.036
  of the cloud's height from a single smooth hump, and two clouds' outlines lie 0.074 apart. The same domes
  under a billow field with a 900 m coarsest lobe: 0.062 and 0.138, and the picture is the same domes, lumpier.
  Turrets under the dome with the same billow field: 0.090 and 0.243, with 2.8 separate tops a cloud for 1.9,
  and the picture is clusters of heaps, no two alike. In a strip of sky where cells overlap, the built form
  shows the shelf, a shallow cell's dome standing out under a deep one's; with turrets the same two cells are
  low heaps beside a taller one. The drawn
  column is 0.46 to 0.51 of the query's in all three, so the bound a query promises is no looser than it was.
- **The clearing, in what a game reads.** One mature cell at its own centre, its envelope moved across the
  value where the turret under it reaches the least share. With the clearing factor on coverage and thickness
  alike, a step of ±0.02 in the envelope moves coverage 0.0098, and each halving of the step quarters that, to
  0.00015 at ±0.0025; thickness goes the same way, 11 m to 0.18 m. With coverage whole wherever thickness
  stands, the rule the review found, coverage moves 0.29 to 0.28 at every step: a switch.
  [The review's probe](../../../.orchestrate/evidence/CLOUDS/design-review/turret-query-probe.py), run against the corrected
  model, finds no jump.
- **The far level holds the query's column.** Over a 96 km square of the fair sky, the query every 100 m is
  cloudy at 0.1275 of points with a mean thickness of 99.22 m; at the 3,600 centres of 1,600 m texels, 0.1308
  and 99.23 m. Cloud lies somewhere in 0.33 of those texels, and 0.60 of them have a clear centre. The
  review's two texels are stored as the query answers at their centres: clear, and 154.75 m thick.
- **Unrun:** the engine's light on the new form, which part XIX takes up; the detail noise; a view from the
  ground; motion; the far level's clouds as drawn. WP3.1 and WP3.2 judge those in the engine. The view from the
  ground then showed what this part did not: shelves, folds and repeats on the turrets
  ([risk 2](#risks)), which WP17.1 and WP17.2 judge.

From [WP3's controls](../../../.orchestrate/evidence/CLOUDS/WP3-plane/README.md) on Metal and `lit_plane_proof.py`, both of 18
November, after WP3 stopped a second time, on a bright underside under a dark stripe toward the noon Sun
(`git show f38c7d77ad24:docs/evidence/CLOUDS/M3-handoff.md`).

- **XIX, the lit plane.** *On turrets, in the engine.* The toward-Sun still drawn one light term at a time:
  the lip over the face above it is 1.8 to 1 as built, 1.1 in the beam, 0.9 in the ambient light and 2.2 in
  solar diffuse; 2.2 still with the view boundary off, 1.7 with the flow term off, and 1.2 with the plane
  held level. Drawn as colour, the two depths the plane was inferred from are a deep column overhead and a
  short path to the Sun along the base, and the reverse across the face, so the inferred plane faced the Sun
  below and turned from it above. With one plane for the cloud the whole light is 1.1 to 1, level, halfway
  or facing the Sun. At a 15° Sun the level plane leaves every cloud flat and grey,
  and the halfway and facing planes keep bright sunlit sides over darker bases.
  *On part XV's boxes, against its photons.* Over the 64 faces the inferred plane is typically a factor of
  1.35 out, a level one 2.29, one facing the Sun 2.02 and one halfway 1.75. At Suns of 70° and 45° the three
  held planes are 1.48 to 1.52. At 15° and 6° the level plane's Sun-facing side is 0.09 to 0.49 of
  transport, and the halfway plane's is 0.64 to 1.54 with a far side at 1.6 to 5.3. A box is the inferred
  plane's best case. Halfway is the heaped type's plane because it is the nearest of the held planes on the
  boxes and loses nothing on the turrets.
- **Unrun:** a still from the built rule; a layered type; the far side of a turreted cloud at a low Sun,
  which WP5.3 judges; cost, which falls by the vertical reads the inferred plane took and WP3.9 records.

## The system as it runs

| State | Owner |
|---|---|
| The weather field, its state and step, the density function, the f64 references | `thrum-cloud-model` (this module), no GPU dependency |
| The weather map, noise, the Sun's path in both bands, the layer and its history, the overlay, placed-volume sampling, the medium coefficients it writes for the view volume | `thrum-clouds` (this module), an effect on `EFFECT-GRAPH`'s seam |
| The view volume: its grid, its history, the joint integration of air, haze and condensate near the camera, and the lent aerial operation that reads it | `ATMOSPHERE` |
| The air's haze: how much aerosol, as a visual range, and what kind, as an Ångström exponent | The weather state; `ATMOSPHERE` owns what the air does with them |
| The surface field (ground heights and water levels) and the fog table | The game, like the type table |
| The committed weather state, with any directed shapes | The game's world; it travels in the tick page |
| Presets, the type table, looks, the library and its placement data, controls | The game (`examples/v1-game` behind its cloud selection; `examples/water` behind its own); never in engine crates or their tests |
| Sky, Sun, aerial, reflection and diffuse products; applying the cloud factor, the overlay and the haze scale | `ATMOSPHERE` |
| Stages, state commit, cadence, the knob list | `EFFECT-GRAPH` |
| Motion and reactive targets, history acceptance, reconstruction, the governor | `APPLE-PRESENTATION` |
| The residency of volume products | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's weather state and forms the travel at the display's time, from the frame's display phase.
`simulate`, on a new tick only: the due part of the weather map; the
due tiles of the Sun's path in both bands and the ground field; the due part of the overlay. Each frame the
effect writes the medium coefficients at the view volume's sample points, and `ATMOSPHERE` integrates the view
volume. `ATMOSPHERE` filters its
reflection and diffuse products with the overlay. Sun passes and forward draw, with the cloud factor on every
direct-Sun term. Sky and stars draw. `after-opaque`: the sea, if the scene has one; then trace, resolve and
compose. Reconstruction, tone and present follow. On submission the effect commits its tick, anchors, refresh
phase and the history side.

**The example.** `examples/v1-game` gains a cloud selection, off by default. With it on, the game keeps a weather
state in its world, adds the cloud contribution after any sea, and shows cloud controls. With it off the game,
its world, its page and every frame are what they were. The selection replaces the diagnostic shade provider.
With Ground it also gives the field the valley's heights as a surface field and a fog table, declares
`ATMOSPHERE`'s view volume, and replaces that module's labelled test medium.

**Creator journey.** A scene adds the cloud contribution with a profile that names its capabilities, a type table
and, if it has shapes, their footprints; it steps a weather state in its world, and puts it in its page. A scene
that composes its sky adds directed shapes to that state. A scene with mist adds Ground, a surface field made
from its own terrain and water, and a fog table, and declares the view volume. Changing coverage, type, wind or time scale needs no rebuild. Changing the
profile or the library rebuilds the effect and resets its histories.

## What CLOUDS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-clouds). Rulings 1 to 5 are each built by this
module's builder in the owning module's files, with that module's items unchanged and its tests kept passing,
and each is absent from a scene with no cloud provider: the same shader text, passes, storage and bytes as
today. Rulings 6 to 8 are `ATMOSPHERE`'s own work, in its WP7 and WP8, and this module uses them as built.

1. **A provider may write the Sun-path optical depth on the GPU** (`ATMOSPHERE`, WP2). Today's field is CPU bytes
   with a rigid drift, read along the true Sun, with a factor of exactly 1 below a lowest elevation and outside
   the region. A provider may instead register a texture its own `simulate` pass writes, with, each frame, its
   mapping, the path direction its columns follow, a border width and one `tau_far`. Such a field is read
   along its path direction at every Sun elevation, and a receiver's factor is `mix(exp(−tau_far), exp(−tau),
   w)` with the border weight `w` above, so it is continuous in the Sun's elevation and across the region's
   edge. The CPU-fed form states none of these and behaves exactly as now.
2. **Aerial transport to a far sky point** (`ATMOSPHERE`, WP3): the lent operation beyond 32 km, as above.
3. **A sky overlay in the reflection and diffuse products** (`ATMOSPHERE`, WP8). If the incident-light meter has a
   Sun term without the cloud factor, it gains it at its metering point.
4. **A haze scale on solar in-scatter** (`ATMOSPHERE`, WP8). A scene that declares `ATMOSPHERE`'s view volume
   takes no haze scale.
5. **A cloud selection in two games** (`SHOWCASE`'s `examples/v1-game` from WP2; `WATERS`'s `examples/water` in WP11),
   off by default, leaving each game unchanged when off.

6. **The view volume takes a provider's medium coefficients** (`ATMOSPHERE` WP8): extinction, a solar kernel and
   an ambient source at the sample points it gives, weighted by the provider's near weight, summed with the
   air's before integration.
7. **The Sun-path optical depth has a ground band** (`ATMOSPHERE` WP8), which holds the whole path to the Sun
   and is read at the receiver's own position in place of the cloud band's value, never added to it.
8. **The air's haze is an input** (`ATMOSPHERE` WP7): the aerosol's amount and its kind, an Ångström exponent,
   which the weather state supplies.

9. **The display phase** (`EFFECT-GRAPH` WP4): where a frame's consumed target lies between the last two committed
   ticks or past the newest, bounded at 50 ms, with the frame's observation, lead, lead status and tick's due
   time on the capture record. This module reads it in WP2 and builds none of it.

Everything else is used as built: effect stages, histories and cadence; the sea's depth write, which the trace
treats as geometry; the temporal contribution; leases of volumes; typed controls; and `PLATFORM-RUNTIME-MAC`'s
presented-time witness, which WP19, WP20 and WP12.6 read on the built-in display
([holding the display's refresh](#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20)).

## Cost account

Estimates, unmeasured, at a 2560×1440 render extent and 120 frames a second. Broadsides' figures are measured
there, not here.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Weather step | Tick, CPU | One record and at most 16 directed shapes | Microseconds | |
| Phases | Frame, once a ray; the tables at preparation | One table read and one closed form; a table for each row of the type table, about 4 KiB each at 1,025 entries, with the Sun term's four numbers | None in the march | |
| Weather map | Half a second of weather, in parts: 2 Hz at 1× | 4 levels × 256², two textures | Under 0.1 ms a frame at 1× | 2 ms an update |
| Noise | Preparation | 128³ and 32³ | None in a frame | |
| Sun's path | At 1×, near level over 8 frames, mid 32, far 128 | 3 × 256² columns (80 m, 320 m and 1.6 km wide, the last reaching the horizon), 48, 32 and 16 slices of the coarse density, which reads the map and the shape noise, and a placed cell's slab where its volume form has weight: 6.3 M stored slices to fill, about 0.5 M a frame. A stored slice is one or more density reads: part H's accuracy figures took four | 0.1–0.2 ms | 2.2 ms shadow |
| Ground field and far value | With the Sun's path; the far value each frame | 1024², and a mean over the outermost level's ground slice | Under 0.05 ms | |
| Time-lapse | Ages are held in weather time | At 60×, the whole map (262 k texels) and up to the Sun path's whole fill each frame | Unestimated beyond those counts; WP2 records 1×, 8× and 60× | |
| Trace | Frame | 640×360 rays, three quarters sky: 3–7 M reads, up to 16 M with three local light samples; the Sun term adds one read of the Sun column's last slice and four exponentials a lit sample, and no density read | The largest term; WP3 measures it | 268 ms; 33 M density evaluations |
| Resolve | Frame | 1280×720 texels | 0.1–0.2 ms | 0.9 ms |
| Compose | Frame | Sky pixels at render extent; a pass that loads scene colour | 0.2–0.4 ms and a tile pass break | |
| Sky overlay | An eighth a frame | 6 × 48² rays, coarse density | Under 0.05 ms | 15 ms capture |
| Ground sources | With the map; the surface texture once | In the map, each ground row's air factors: its extent and the cut of its noise. Apart from it, one world-anchored texture of the surface field and the datum | Within the map's line, and one more read a sample for the surface | |
| Ground band of the Sun's path | With the Sun's path | 3 levels × 256² columns × 8 to 12 heights, with one visibility bit a node | As the cloud band, about a quarter of its samples | |
| Medium coefficients | Frame | One sample for each view-volume cell inside the handover (about 0.7 M of 0.9 to 1.4 M cells): the map, the shape noise, the Sun's path | Unmeasured; Frostbite's write and lighting of a like grid took 2.5 ms on a PlayStation 4. WP14 records it | |
| Far condensate in the trace | Frame | The same rays, longer where mist lies between the handover and the land; a stop for each pixel of a texel, kept as the ray passes its depth; one lent aerial read for each part of a ray that holds condensate, one for a ray through a cloud and up to tens through thin mist in thick haze; and the texels traced out of turn where no carried stop passes | Unmeasured; WP15 records it beside WP9's, with the rays traced out of turn | |
| Memory | | Weather 4 MiB, noise 2, Sun's path 13, ground 2, layer history and trace 25, footprints about 1, library 27 MiB as R8 or 14 as BC4, growth track the same again; the library is held by every scene that declares Shapes, shadow-only included. With Ground: the surface field and datum about 2 MiB, the ground band about 4, the medium coefficients about 8 at 160×90×72, and four stops a texel in the layer and its history where there was one, about 52 | About 47 MiB plus the library, and about 66 more with Ground | 62 MiB plus history |

**Records and tripwires, never gates** ([brief](../VISION.md), decision 3). `scripts/cloud_cost.py`, created in
WP2, takes a paired record of one scene at one view, capture off: the scene with clouds and with them absent,
giving the whole frame, each cloud pass's GPU time by name, the bytes of each resource and the load on the
machine. A record trips a wire when the whole frame with clouds is over the display's refresh interval, when a
pass is about 25% or more over its last comparable record, or when a cost grows faster than the work that was
added. A tripped wire is investigated in the work package that tripped it, and the record says what was measured
to find the cause and what was fixed or left as a follow-up in the log. Stutter in the clouds' own motion, a
step that does not match its frame's display time, is judged in the `realtime` inspections.

The ambition is the display's full refresh, and whole-frame tuning is the later performance phase. **No timing
figure decides whether an item passes, whether a work package is Done or whether the next one starts**, with
one exception: WP20 holds the weather sky to the display's refresh in the game as a player runs it, by the
refreshes it loses and the longest it stalls beyond the same game without clouds, both from the OS-reported
presented times
([below](#holding-the-displays-refresh-while-clouds-drift-wp19-and-wp20)). What the game loses without clouds
decides nothing there, and pass times stay tripwires there too.
`cloud_cost.py check` measures the scene again and fails only on a missing measurement or a missing
investigation. It holds the committed record and its own fresh measurement to one list: for the scene named,
a run with clouds absent and a run with clouds visible, at one view; and from the visible run, the cloud
passes by name, the work the scene's item names (rays and samples where there is a trace) and the bytes of
each resource. A record or a measurement with no clouds-visible run fails whatever its timings, and so does
a record whose tripped wire has no investigation beside it. The fresh measurement is what ties a record to
the code being checked; the record's commit is written down and decides nothing, because a record pinned to
a commit goes stale at every merge. `cloud_cost.py refuse` shows the list has teeth: it puts the committed
record, with every clouds-visible run replaced by its clouds-absent run, through the same validation and
passes only when that is turned away. WP3.9 runs it, and every later scene goes through the validation it
proved. Presented cadence on the built-in panel, by the OS-reported presented times, is attended setup: it
is judged for the weather sky in WP20's wind series and recorded for the finished sky in WP12.6.

## Risks

1. **The trace costs too much.** Early sign: WP3's record. The wrong fix is dropping the local light samples or
   the detail noise below what the look needs without saying so. The designed ones: a sparser refresh (one texel
   in sixteen), a coarser far step, fewer local samples with a finer near level. WP3 tries the ones its record
   points at and records what each did to the cost and to the look. Building goes on either way; what remains
   over the refresh interval is written up as a follow-up for the performance phase. The weather sky under the
   fair preset is the exception from WP19 on: there a refresh lost to the clouds is fixed, first by moving when
   work is done, and a remedy from this list brings the stills and sequences it changes back for inspection.
2. **Procedural cumulus reads as repeated shapes, and a single cloud shows shelves and folds.** WP3's first
   form did, through three looks: one dome a cell, so domes, lenses and shelves. Turrets and the billow field
   replace it (part XVIII). On Metal they gave a sky that reads as scattered cumulus at a glance, with separate
   clouds, varied sizes, solid interiors and parallax, and left four flaws in a cloud seen alone: stacked
   level shelves under the large clouds toward the Sun; a lip or level band under an isolated one; a hooked
   fold or squared break in the near cloud's lower edge across the Sun; and caps, wedges and cones that repeat
   in the middle distance. The lead parked the cue on 18 November 2026, after five rounds of looks. WP3.1
   judges the sky at a glance, and **WP17** judges each cloud, after the rest of the module's sky is built.

   WP17 judges the clouds that failed. By then the fair preset places library shapes among its procedural
   clouds, and at a placed share of 1 it holds no procedural heap at all, so a sky judged with shapes in it
   could pass on their heaps while the turrets kept every flaw. WP17.1 on the Mac and WP17.2 in Chrome
   therefore take the fair preset with the placed share at 0 and no directed shape, which is
   [a sky of weather alone](#placed-volumes) and the sky WP3 stopped on, from 2 m and from 40 m on both
   targets. WP17.3 then looks at the presets as they ship, shapes included.

   The flaws are excused only until WP17 starts. In a work package built before it (WP3 to WP16, and WP11) the four
   are known: they fail no inspected item, and a reviewer who sees them says so and judges the item's own
   list. From WP17 on nothing excuses them. One that shows on a procedural fair-weather cloud fails the
   inspected item whose view shows it, in WP17, in WP12 and in WP21.

   What WP3 found, in [the erosion notes](../../../.orchestrate/evidence/CLOUDS/WP3-erosion/README.md) and
   [the type-plane notes](../../../.orchestrate/evidence/CLOUDS/WP3-type-plane/README.md). At `5f84d354a3bf`
   Git stored both under `docs/evidence/CLOUDS/`. Their archives hold the opacity observer, the point march and
   the capture scripts.

   - The fold is in the density and not in the light. It is in the trace's opacity before light is composed,
     and turning the lit plane by 0.25, 0.50 and 0.75 changed its brightness and not its outline. The shelves
     stayed through those turns too.
   - The CPU point march draws the same break with no weather map and no mip: 82 px at the profile's erosion
     of 0.61, against 108 px on Metal; 80 px with the detail noise at its mean; 2 px with nothing eroded. So
     the shape noise held against the column profile cuts it, and the detail noise and the map's filtering do
     not. What makes Metal's break larger was not found.
   - On the CPU the break shrinks only below an erosion of about 0.3: 66 px at 0.30, 20 at 0.20, 8 at 0.10.
     The one Metal look at 0.20 kept the shelves, the break and the repeated caps, and smoothed the far side
     into domes and a cone. Its opacity was not read back, so those CPU figures do not describe it. Erosion
     strength alone is ruled out.
   - Also tried, each leaving shelves or repeats: a wider pitch; deeper, and then uniformly drawn, mature
     depths; a doubled shape-noise wavelength with a stronger taper; lower water with a longer base fade; a
     higher clearing threshold; coverage blended in the march.
   - Not explained: the stacked shelves toward the Sun. Opacity saturates there where near and far clouds
     overlap, so a shelf may be one cloud's own terraces, where a lobe several hundred metres wide is cut
     along a level of the column profile, or a farther cloud standing under a nearer one. Nor is the repeat
     of caps and cones explained.

   WP17 starts from causes, as [the lit plane](#cloud-light) was found: a control on the point march or the
   opacity observer that removes one flaw, before any look on Metal. The candidate fixes, none yet tried
   against a named cause: a wider spread of turret heights and widths; a coarser first octave; turrets with
   lives of their own, by the cell rule one level down. A cause that asks for a mechanism this design lacks
   ends `BLOCKED` with `needs_context` and the cause, for a planner. Never one dome a cell again, and never a form only the trace has: the query,
   the map, the Sun's path and the trace read one density, and WP17.4 holds the changed form to that. The map
   level whose texel is wider than a turret holds [a sample of them](#the-weather-map), so a far cloud's
   heaps are fewer and broader than a near one's and change where it crosses into that level. If WP3.2's crops show where that begins, the fixes in order: a
   fade between the two levels across a band at the finer one's edge; then the levels' texels and sizes,
   which are profile constants. Never a column the query does not give: no share standing in for turrets, and
   no turret field smoothed for a far level.
   Three failed looks on one cue go to the log and end `BLOCKED`.
3. **The reconstructed layer ghosts or crawls.** Early sign: WP3's pan. Fallback: refresh every texel every
   frame at a lower layer scale, which removes the layer's history from the picture.
4. **Low-Sun relief is lost in the summed product**, or its coarse density shades an eroded edge visibly
   differently from the cloud that is drawn. Early sign: WP3's and WP5's low-Sun stills. Fallback: more local samples at low Sun, or a finer near level. The same
   holds for a placed shape, whose shade edge the product places within one column's width (part H): WP6's
   record and its stills of a shape's shade are the early sign.
5. **`ATMOSPHERE`'s products resist the overlay**, because their storage is relative to the Sun. Early sign:
   WP8's first test. The ruling states behaviour, not storage; if the producers cannot compose it, the proposal
   goes to that module's log.
6. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: WP2 runs under WebGPU's default
   limits on the Mac, and WP4 is the first real Chrome run.
7. **The droplet's forward peak aliases or flickers** in the part-extent trace, as a ring or a crawling aureole
   around the Sun through thin cloud. Early sign: WP5's stills and pan toward the Sun. The fix is in the phase's
   evaluation (its precision, its table's spacing, or the angle taken at the texel's centre), never a return to
   the wide lobe or to a rule that loses energy; a game that wants a softer look gives its own four numbers.
8. **The handover shows**, as a ring a few hundred metres out where mist is softer on one side than the other, or
   where a low deck overhead is drawn at the volume's cell size. Early sign: WP15's walk and pan. The fixes, in
   order: a longer ramp; a nearer handover; a finer grid inside it. Never a second density for the near field.
   A bank that is brighter or dimmer from one integrator than the other, in WP15.13's stills, is a different
   fault: the two are not taking one source. The fix is to find the input that differs (a depth, the weight,
   the direction to the eye), never a gain on one side.
9. **Mist reads as a flat grey slab**, without the glow, the soft top and the wisps of the shipped bar. Early
   sign: WP14's first stills. The look is in the fog table, the noise and the phase, and is tuned against WP14's
   list with a fresh opinion; three failed looks on one cue go to the log and end `BLOCKED`.
10. **Carried stops ghost or cost too much.** A ghost follows a mover across far mist, or a fast pan traces so
   many texels out of turn that the frame is late. Early sign: WP15's sequences and its record of rays traced
   out of turn. The fixes, in order: a tighter stop tolerance for a ghost; for cost, a coarser layer where
   stops end on surfaces. Never a stop filled from a neighbour or blended between two depths.
11. **Far mist in thick haze takes its air wrongly**, as a bank too bright or too blue against the land behind
   it. Early sign: WP15.4's stills in thick haze. The fix is a smaller part bound, never one part for the
   ray.
12. **The type's plane lights a cloud too evenly, or the view boundary shows.** A plane held for the whole
   cloud cannot darken a far side as a plane turned away from the Sun did: on part XIX's boxes the far side
   at a Sun of 15° and lower is 1.6 to 5.3 times transport, and at noon the Sun side and the far side are
   nearer each other than photons leave them. Early sign: WP3's noon stills, as a cloud that reads flat, and
   WP5's at a low Sun, as a far side nearly as bright as the lit one. The boundary term turns on where a view
   ray enters between the lit plane and the far one, and would show as a seam along a cloud's shoulder. The
   fixes, in order: the plane's turn, by eye, for the type; the boundary depth; the type's water, which
   deepens the Sun column. The builder makes them and records what changed. If a low-Sun far side still fails
   WP5's list, draw each light term alone first, as [WP3's controls](../../../.orchestrate/evidence/CLOUDS/WP3-plane/README.md)
   did, and log which one holds it. Never a second light, a return to fixed orders, or a plane read from a
   sample's depths: that plane drew WP3's bright lip under a dark stripe on turrets, and is the likely cause
   of the level dark band on its domes before them.
13. **The V1 game changes under the selection.** `SHOWCASE` edits the same game. WP2.1's test that the game
   is unchanged with the selection off stays in `./dev check`, and the scheduled main-branch check
   `exact-images` verifies every registered image after each merge.
14. **A high Sun flattens the broad lobes of a placed cloud.** WP10 found it on Metal and in Chrome alike: at
   noon the lower right face of the congestus core, from 40 m, and the left lobe and bridge of the mediocris
   twin turrets, from 2 m, read as one even grey. The other shapes at noon, and all twelve at a 10° Sun, show
   their depth. The lead parked the cue on 26 November 2026, after three looks: at the bar of shipped games a
   high Sun that flattens a broad lobe is plausible. So the weakness is a known limitation, WP10 closes on the
   light it retained, and **WP21** takes the relief up last, after WP12. Nothing waits for it. WP12 closes the
   module on the retained light, with the weakness listed in the maturity ledger (WP12.9), and a WP21 that
   ends `BLOCKED` holds up no work package. When WP21 passes, its builder takes the weakness out of the ledger.

   The limitation is those two shapes' at noon, and no other cloud's. In every work package built before WP21
   (WP10, WP14 to WP16, WP11, WP17 and WP12), a broad lobe of the congestus core or of the mediocris twin
   turrets that reads flat at noon fails no inspected item: a reviewer who sees it says so and judges the
   item's own list. Nothing else is excused. The other ten shapes at noon, and all twelve at a 10° Sun, are
   held to their shading by the fifth line of WP10.1's list: in WP10, and again in WP21.2 and WP21.3 once the
   light has changed, so a remedy cannot flatten a face that
   [WP10's gallery](../../../.orchestrate/evidence/CLOUDS/WP10/qualification-metal/) shows shaded.

   WP21.1 on the Mac and WP21.3 in Chrome judge the two shapes, each alone in WP10's gallery view of it, from
   2 m and from 40 m, so no other cloud's relief can pass for theirs. They judge by the cues of the bar and
   nothing else: neighbouring lobes told apart by a soft shaded crease, a sheltered face darker than the crown
   above it, no mark of the remedy, and a cloud still bright and soft.
   [WP10's retained noon stills](../../../.orchestrate/evidence/CLOUDS/WP10/lighting-controls/qualified-noon.png)
   say where to look and set no bar. The faces they show as one even grey fail the first two cues as they
   stand, so the retained light cannot pass as a remedy, and a view that already meets the cues passes without
   gaining contrast.

   What WP10 found, in [its handoff](../../../.orchestrate/evidence/CLOUDS/WP10-handoff.md) and
   [its lighting controls](../../../.orchestrate/evidence/CLOUDS/WP10/lighting-controls/README.md), with the
   light it retained at `63f4e85794fe`:

   - The fault is in the Sun's diffuse light. WP10 began with curved arcs on a ridge at noon. Captures of each
     light term alone put them in the solar diffuse term, and the forward lobes and the sky's ambient light
     have no matching mark. A finer Sun march (a least step of 42 m for 66 m) and a blue-noise offset turned
     on each submission both kept the arcs, and the control with no local samples removed them.
   - The retained light gives the diffuse light the broader Sun-path column under a high Sun, which reduces
     the arcs and by itself leaves the lower lobes flat. A filtered, symmetric density probe restores part of
     the relief, and the flank away from a low Sun.
   - Ruled out: a one-sided density probe, which drew brighter relief and a chain of pale marks across the
     core; and a stronger response to local depth
     ([its patch](../../../.orchestrate/evidence/CLOUDS/WP10/lighting-controls/daylight.patch)), which
     brightened the folds of the crowns and left the lower lobes flat in three views.
   - Not tried against this cue, by WP10's record: the constants [risk 12](#risks) gives for a cloud that
     reads flat, which are the plane's turn, the boundary depth and the type's water.
   - Not shown by any control, and the planner's reading only: the Sun term reads three depths and no facing
     ([the lit plane](#cloud-light)). Under a high Sun every Sun column through a broad lobe is deep, so the
     diffuse light is nearly even across the lobe's faces, and the ambient light reads only the column above
     and below a sample. None of these says how open a point is to the lit cloud and the sky around it, which
     is what tells a crease from a crown.

   WP21 starts from the cause, as WP17 does: a control on the light terms drawn alone that brings out the
   creases of the two shapes at noon, before any look. Risk 12's constants come first. The remedy keeps one
   Sun term for every consumer and one density, keeps a placed cloud and the procedural clouds beside it in
   one light (WP21.2), and leaves the accepted voxels as they are. Shipped hero clouds add terms tuned by
   depth inside the cloud and by height ([Research and selection](#research-and-selection)). A field of that
   depth, cooked beside a shape's density or derived from it, is a product this design lacks: a cause that
   asks for it, or for any other mechanism this design lacks, ends `BLOCKED` with `needs_context` and the
   cause, for a planner. Three failed looks on one cue go to the log and end `BLOCKED`.

   WP21 comes after the finished sky, so it answers for what its light does there: WP21.5 judges the four
   presets, the mist views and WP12.7's scene again on the Apple recipe under MetalFX.

   Its cost is recorded where it runs. WP10.5's record is taken at a 10° Sun, whose vertical component, 0.17,
   is below the 0.20 where the retained light's broader column begins, so it times none of the high-Sun work.
   WP21.4 records the `relief` scene: WP10.5's view, with all twelve shapes and the track in view, on both
   recipes, at the noon Sun of WP21.1's stills and at the 10° Sun, with each arm taken at the commit WP21
   started from and again at its end, with the same tool on the same machine. The starting arm is its end
   arm's last comparable record for [the wires](#cost-account). `cloud_cost.py check` measures the end arms
   again and holds the record to that list.

## Proof and builder handoff

Add V1 game Chrome cases in the shared `scripts/check_v1_browser.mjs` while preserving existing cases. For the
water scene, WP11.5 uses `scripts/check_water_browser.mjs`.

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. `ios-sim` is not a target. Chrome checks need a
  browser launch outside the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
  Never substitute a Mac result.
- **Capture.** Inspected items use T2 engine capture and name their mode.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/clouds-<work-package>/`, selected evidence in
  `.orchestrate/evidence/CLOUDS/`.
- **Exact baselines.** The module adds crates and a selection that is off by default, and moves no retained
  image. The scheduled main-branch check `exact-images` verifies that after each merge, and no work package
  runs the verifier as an item ([rules](../ARCHITECTURE.md#exact-baselines)).
- **Broadsides.** Read it at the commits [What Broadsides offers](#what-broadsides-offers) names; never change
  it, and never build against it. A file may be copied or translated where it does what this design states, on
  the owner's words of 14 November. List each copied or translated file with its source commit and path in
  [the tracked provenance record](../../provenance/CLOUDS.md), and review it against this module's items. Material
  that came into Broadsides from elsewhere keeps its own licence, recorded with it, and nothing GPL or
  share-alike is copied. The cloud library is read from Git LFS without changing the Broadsides checkout, and its
  licence record is copied beside it.
- **Game content stays in the game.** Presets, type tables, looks, volumes and the library never enter
  `thrum-cloud-model`, `thrum-clouds` or their tests; tests make their own inputs.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** The ledger's `E1.clouds` row is kept current from WP2, with implementation, correctness, visuals
  and device apart.
- **The bar.** [How the module is proved](#how-the-module-is-proved) governs every work package not yet Done.
  A builder tunes the look against the inspection list, with a fresh opinion, and records what it changed. A
  reference is there to find a cause, never to pass or fail. Three failed looks on one cue go to the log and
  end `BLOCKED`. The form flaws [risk 2](#risks) names fail no inspected item of a work package built before WP17.
  From WP17 on they fail any inspected item whose view shows one on a procedural fair-weather cloud. A broad
  lobe of the congestus core or of the mediocris twin turrets that reads flat at noon ([risk 14](#risks))
  fails no inspected item of a work package built before WP21, and fails WP21.1 and WP21.3. On any other
  shape, or at any other Sun, nothing excuses it.

## Open questions

| Question | Settled by |
|---|---|
| What does the trace cost at 2560×1440 against the refresh interval, and at which refresh fraction and local sample count? | WP3 records; WP12 records again at the showcase's extent on both recipes |
| What do the weather map and the Sun's path cost at 8× and at 60×, where their ages are held in weather time? | WP2 records |
| What tolerance does the GPU weather map meet against the CPU field on Metal and in Chrome? | WP2 measured and asserts it; WP4 holds Chrome to the same test |
| How many sampled textures does the lent environment group use in the trace? | WP3 counts against the record |
| Does the summed Sun-path product carry low-Sun relief with three local samples? | WP5 |
| Does the Sun term give clouds their brightness and relief with nothing tuned, and if not, which constant moves? | WP3's stills, then WP5's at a low Sun; the builder records any constant it changed |
| Does a far side at a low Sun stay dark enough under a plane held for the whole cloud? | WP5's stills at a 10° and a 4° Sun |
| Do turrets that only the cell's envelope raises and lowers read as a cloud growing, or do they need lives of their own? | WP7's time-lapse |
| What draws the shelves, the fold and the repeated caps on turrets, when the lit plane and the erosion strength do not? | WP17 finds each cause by a control before any look ([risk 2](#risks)) |
| What brings out the creases of a broad lobe under a high Sun, when a finer Sun march, a broader diffuse column and density probes do not? | WP21 finds the cause by a control before any look ([risk 14](#risks)) |
| Does a far cloud read as cumulus on the map level that samples turrets it cannot resolve, and does its crossing into that level show? | WP3.2's crops |
| Does the droplet's forward peak hold steady in the part-extent trace? | WP5 |
| How many entries does the phase table need to hold its integral and its values near the Sun? | WP1 measures first, beside part G's 1,025 |
| Does the incident-light meter already see cloud shade? | WP8 reads the code and measures |
| Can the scene's residency take the library as a second source folder? | WP6; if not, the proposal goes to `CONTENT-LOOP`'s log |
| Is an Apple-tile kernel variant a measured win? | WP12 |
| Where does the game lose a refresh while clouds drift: the display link, a late request, a full frame slot, the drawable, encoding and submission, or after the present call? | WP19's first wind series |
| Which cloud work takes a frame past its refresh at 1920×1080 on the Apple recipe, through a wind turn and at 60×, that the same frame makes without clouds? | WP19's diagnostic runs, where they show the loss again |
| Does the presented-time witness cover a 20-second measured interval in each of eleven recordings? | WP19's first wind series; `PLATFORM-RUNTIME-MAC` qualified it over about four seconds |
| How wide a datum window pools mist as the valley's relief wants? | WP13 sets the profile's value on the valley's heights; WP14 judges it |
| What do the ground band and the medium coefficients cost, on the Mac and in Chrome? | WP14 records |
| Does a far bank sit in the land's haze at visual ranges of 8 and 60 km? | WP15.4's stills |
| What stop tolerance keeps a surface's own stop through a walk and rejects its neighbour's, and how many texels does a pan trace out of turn? | WP15 sets the profile's value and records the count |
| Does the handover show at 300 to 500 m, and at what ramp? | WP15 |
| Is one bank as bright from the view volume and from both integrators? | WP15.13's stills |
| Can `examples/water` give a surface field from its water map without a change to `WATERS`? | WP11; if not, the proposal goes to `WATERS`' log |

## Ground-condensate integration handoff

`ATMOSPHERE` owns the view volume, its history and the lent aerial operation; this work package writes the medium
coefficients and the ground band it reads and replaces the labelled test medium in `examples/v1-game`. A defect
in the volume itself goes to `ATMOSPHERE`'s log. WP13 built the model's solar kernel from the four fixed
orders; WP14.13 removes them, and the kernel the view volume takes is the trace's own shader function.

## Weather map and Sun-path regressions (WP18)

The native and default-limit portable wrappers share a 256-square weather-map and Sun-path campaign. Use
the smallest valid map and Sun-path grids that still cover all levels, populated and clear columns,
fractional or wrapped coordinates, cadence, rollback or rebuild, and replacement-device cases. Keep a real
GPU witness of the submitted weather and Sun-path outputs against the CPU queries on both profiles. Use
the CPU for comparisons and state transitions that need no GPU. Retain each row-age, cell identity,
optical-depth and limiting-profile assertion on the compact cases.

Run the two cases side by side on the shared GPU during the focused timing run. Select them in the workspace
test build with the default Nextest profile and worker count used for the workspace check's first attempt.
Keep the usual test groups and timeout settings. Under normal builder load, both cases must pass on that first run in
at most 9 seconds each, leaving at least 1 second before the existing 10-second timeout. Measure each case
alone for comparison with the isolated baseline in the log.
