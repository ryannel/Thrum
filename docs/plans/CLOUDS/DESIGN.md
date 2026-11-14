# Clouds: the weather, from the cloud tops to the mist on the ground, that shades and lights the world

13 November 2026, revised 14 November, judged again from first principles the same day, and revised after
[its second review](../../evidence/CLOUDS/design-review.md). [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md). C02, C04, C06, C07, C08, C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[ENVIRONMENT](../../design/modules/ENVIRONMENT.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.
[The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) records what this design selects and how
Broadsides may be reused. This design replaces the implementation target in
[CLOUDS.md](../../design/modules/CLOUDS.md); that page's ownership and transport rules stay in force and are
restated where they bind. Revised again on 14 November for the atmosphere as a whole:
[mist, fog and low cloud](#condensate-that-meets-the-ground-mist-fog-and-low-cloud) are this module's, and
[what that replaces](#judged-again-for-the-whole-atmosphere) is listed.

## The request

The lead's planning task of 13 November 2026:

> Plan the CLOUDS module (docs/plans/CLOUDS/BRIEF.md). The owner, asked whether SEA and CLOUDS planning could start
> now rather than waiting on their port timing, said (13 Nov 2026): "On 2 use the fable planner like normal but you
> can bring that across when it makes sense." Lead's reading, to record as an assumption: planning starts now, and
> the owner leaves to this plan when the Broadsides Rust work is brought across, within
> docs/design/decisions/SEA-AND-CLOUD-PORT.md (module by module, no wholesale import, Broadsides untouched).
> Milestones that depend on unfinished modules wait on them as usual.

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

The lead's reading, recorded as [assumptions](PLAN.md#open-for-the-human): the owner's last words govern; the
design of 13 November, [the brief](BRIEF.md) of 10 November and the port decision describe the Broadsides
approach and are inputs to judge, not constraints; Broadsides code, shaders and assets may be copied and reused
as is wherever this design finds them useful; Broadsides itself stays unmodified.

So this design starts from who uses a sky in Thrum and what each needs, chooses its mechanisms against that, and
only then says what Broadsides has that serves them. [Judged from first principles](#judged-from-first-principles)
gives the verdict on every choice the design of 13 November inherited: six changed, and the rest are kept for
reasons that do not rest on Broadsides having built them. [What Broadsides offers](#what-broadsides-offers) says
what may be copied.

It is not a blind study. Read for it: the design of 13 November and [its review](../../evidence/CLOUDS/design-review-1.md);
the Broadsides procedural notes at `b4d8250e6` (`THRUM-PORT`, `RUNTIME-DESIGN`, `COST-2026-11-06`, `COST-MODEL`,
`ASSET-CONTRACT`, `PROCEDURAL-COMPLETION`, `bw_core/src/weather.rs` with its data file, and for this revision
`bw_core/src/medium/scatter.rs`, `bw_render_clouds/src/phase.rs`, `cloud_lighting.ron` and the list of its cloud
shaders); the baked path at `79637f789` (the library's README, licence record and `B3-HANDOFF`, the growth proof
and its motion review, the first sections of `PIPELINE-DESIGN`); and in this repository the designs of
`EFFECT-GRAPH`, `ATMOSPHERE` (environment products), `CONTENT-LOOP` (leases) and `WATERS`, with
`atmosphere/cloud.rs`, `cloud.wgsl` and the diagnostic provider in `examples/v1-game`.

## Purpose and scope

A reusable engine sky. Thrum's first outlet is teaching demos on the owner's website, then games
([the brief](../BRIEF.md#purpose)), so the sky has four kinds of user, and each sets a requirement:

| User | Needs | What that requires of the design |
|---|---|---|
| A teaching demo, often in Chrome: a sunset, how weather changes a landscape's light, later how a cloud forms | A sky that is right as well as good-looking, composed for the shot, under a visitor's sliders, with only the parts the demo shows | Mechanisms with an oracle; light from the droplets' own optics; a cloud the demo can place; every part beyond the weather optional, at no cost when left out |
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

**What exists.** Thrum has no cloud renderer and no fog, and its haze is a constant of `ATMOSPHERE`'s air. `ATMOSPHERE` M5 applies one Sun-path optical depth on every
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
  They are the octave model above with fixed numbers, and they are this design's optical oracle for Beer
  segments, their composition and the later orders. The first order's phase has its own closed form.
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
- **Not read:** the Red Dead Redemption 2 course notes (Bauer 2019; not found to fetch), the 2015 and 2017 Nubis
  talks, and the Broadsides cloud shaders beyond their names and sizes. Nothing below rests on them.

Five reproductions were run for this design, the second for its revision, the third for the first-principles
judgement, the fourth after its second review and the fifth for mist, fog and low cloud, and are kept with their output in
[the evidence folder](../../evidence/CLOUDS/design-proof/). [Check that it can work](#check-that-it-can-work)
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
| Drift taken at the committed tick | **Change** | A 60 Hz tick is shown twice on a 120 Hz display. At a 60× time-lapse in a 40 m/s wind a near cloud would stand still for one frame and jump 37 pixels the next (part F). Travel is a function of time, so each frame takes it at the display's time, as `WATERS` does its waves. See [time](#time-drift-at-the-displays-time-evolution-on-the-tick) |
| A placed volume stands for one cell of the field | **Keep** | It is why the query, the picture and the shade agree on a shape's outline at any distance, and why a shape forms, drifts and thins like any cloud. It does not rest on the library existing |
| Shapes are placed only by the field's hash | **Change** | A demo composes its sky: a tower beside the Sun at sunset. A profile places shapes by the hash or by the game's own list of [directed shapes](#placed-volumes), which are cells whose place, shape and life the game sets |
| Shadow-only as the one reduced profile | **Change** | Shade, the visible sky, the world's light and shapes are each a capability, and a scene declares only those it uses. A sunset demo needs no library; a top-down game needs only shade, and holds a library only if it wants the shade of shapes |
| Clouds shade and light the world through `ATMOSPHERE`, never by themselves | **Keep** | One owner applies each factor once. A private sky or a second shadow term is what made Broadsides' verdicts unusable here |
| The layer ends at render extent and the engine reconstructs | **Keep** | One reconstructor sees every drawer's motion. Unreal, Frostbite and Decima all trace clouds below the render extent and hand a full-extent layer on |
| Nothing copied from Broadsides until a licence is named; stand-in shapes first | **Change** | The owner's words of 14 November. The library is the example's content from M6. See below |

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
| The trace ignores depth except to skip covered rays | A ray stops at the scene's depth and keeps two stops | Far mist lies against hillsides and water |
| "Atmospheric system" means clouds and weather; `ATMOSPHERE` is not reopened ([plan](PLAN.md#open-for-the-human), until now) | `ATMOSPHERE` gains the view volume and a haze it takes from the weather, as milestones after G3s closes | The owner's words. The V1 exit scene does not change |
| The Sun's path ends at the cloud layer's base, and a receiver below it reads one plane | It continues to the ground in a ground band, read at the receiver's own position | Land inside fog must lose its Sun, and a bank must shade the water beside it |

Kept, and now carrying more: the weather field as the one authority, the air frame, one density in m⁻¹, the
Sun's path as the one optical depth, cloud light with the droplets' phase, the layer ending at render extent,
capabilities a scene leaves out at no cost, and light and shade applied by `ATMOSPHERE` alone.

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each milestone judges fit file by file; this table says where to look. Paths are in
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
| Octave light, the density operators, the resolve's reprojection and clamp | `shaders/cloud_lighting.wesl`, `cloud_density.wesl`, `cloud_resolve.wesl`, `cloud_reconstruction.wesl`; CPU twins in `src/twin/` | Take functions where they state what this design states. The cone taps and ambient rays in the lighting are where the 289 ms went and are not copied |
| The weather operator and presets | `crates/core/bw_core/src/weather.rs`, `crates/sim/bw_world/src/weather.rs`, `cloud_weather.ron` | Reading matter. The field here is the cell process and the deck of this design |
| The packed Beer shadow, the atlas and pager, parcel and ambient caches, sky capture, the Bevy host, `module.rs` | | Not used. The Sun's path is a different product, slabs replace the atlas, and Thrum has one owner for the rest |

Broadsides shaders are WESL and Thrum's are WGSL, so a copied shader has its imports resolved into Thrum's
modules; its algorithm is left as it is. [The handoff](#proof-and-builder-handoff) says how a copy is recorded.

**What would change these selections.** M3's record shows what the trace costs. If the frame is over the refresh
interval there, M3 tries the remedies in [risk 1](#risks) that its record points at and records what each did,
and building goes on; whether the layer's extent or the light model should then change is a planner's decision
from that record, never a stop for a builder. If M5 shows the summed Sun-path product cannot carry low-Sun
relief, its near level is refined or the local samples grow, here first. If a game needs hashed shapes and
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
  never switches one on. A cell's column is a dome over a flat base at the type's condensation level, or the
  footprint of an authored shape where the cell is a [placed](#placed-volumes) one; bases are shared, sizes
  vary.
- **The deck.** Layered cloud is a sheet whose thickness follows a smooth evolving noise against a threshold set
  by coverage and type: gaps at a broken deck, none at overcast.
- **Types** are game data: a table along the type axis giving base height, depth range, liquid water content,
  droplet radius, the shares of cells and deck, and a vertical profile. The engine interpolates it. Extinction
  is physical: `σ = 3·LWC / (2·ρ_water·r_droplet)` in m⁻¹, about 0.06 m⁻¹ for 0.4 g/m³ and 10 µm.
- **Continuity.** The field is continuous in space, in weather time and in every parameter. Where cells overlap
  the column takes the larger depth and the larger profile, never a switch between cells.
- **Lattice and period are profile constants.** A preset never changes them, because scaling a lattice would move
  every cloud at once.
- **Precision.** Site indices and generations are integers. Weather time reaches the GPU as an integer generation
  count and a fraction, and positions as integer texel coordinates plus a snapped anchor, so the GPU names the
  same cells as the CPU after any travel.

**Outputs.** Coverage is the column's profile, 0 in clear air. Age is the dominant cell's life phase. A query
outside nothing is invalid; valid clear weather returns zero.

**What a query promises.** Column optical depth is the vertical integral of the density below with nothing
eroded: every noise term at its uneroded value. In the model that is exact at every point, for cells, the deck
and [placed shapes](#placed-volumes) alike. It is an upper **bound** on the column that is drawn there, because
noise only removes extinction, and it is zero exactly where the sky is clear. The field holds no average: a
query never answers for a neighbourhood, and no form of a cloud is fitted to another's mean. The averages in
this design are named where they are made: the GPU reads the field through filtered textures, so what it draws
over a point is a local average of the model's density over a texel, held to the model at stated points within
a bound that is measured first; the [coarse density](#one-density-in-m¹) takes the detail noise at its mean; and
the Sun path's [far value](#the-suns-path-one-optical-depth-product) is a mean transmittance.

### Time: drift at the display's time, evolution on the tick

The weather state steps on the committed tick, and the display shows each 60 Hz tick twice or more. Travel is a
function of time, so every presented frame takes it **at the display's time** `t = C − 1 + α` in ticks, where `C`
is the frame's committed tick and `α` its interpolation alpha (C02):
`travel(t) = travel(C) − (1 − α) · (travel(C) − travel(C − 1))`, formed on the CPU in f64 from the state.

Travel enters a frame only through the anchors that place the air frame in the world: the trace's, the mapping of
the ground field that `ATMOSPHERE` reads, the overlay's, and the motion a cloud writes. So a frame that shows the
same tick again moves every cloud and its shade by the wind's travel since the last frame, and rewrites no texel:
the weather map, the Sun's path and everything that evolves stay at the committed tick, as `EFFECT-GRAPH`'s state
rule requires. Evolution can stay there because it is slow: at 60× a cell changes its envelope by at most 0.2% of
its range in a tick (part F). On a cut, a seek or a new timeline the frame takes `travel(C)`.

### One density, in m⁻¹

Every consumer (the view, the Sun's path, the sky overlay, the CPU reference) evaluates one density function:

`σ(point) = σ_type · profile(height fraction) · shape(column profile, noise)`

The column profile thresholds a low-frequency 3D shape noise, so a cloud's interior is solid and its edge breaks
into billows; a finer detail noise erodes only the edge. Noise can remove extinction and never add it beyond the
column's bound. As a cell's envelope moves, the threshold sweeps through fixed noise, so billows appear and
dissolve as structure and nothing slides. Noise is read at a mip chosen from the sample's footprint, so a far
ray reads an average, not aliased detail. Both noise volumes are generated on the GPU at preparation from the
same integer hash on every target.

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
texel doubles or quadruples outward to the horizon. A texel holds the field's column (base, top, profile, type)
and, apart from it, two presentation values: the slot of at most one placed shape and the weight of its volume
form. Compute passes in `simulate` evaluate the same cell and deck rules as the CPU. Because the map lives in
the air frame, wind costs nothing: only evolution and the camera's movement through the air need new texels, so
the map is refreshed in parts. A coarse level of the tallest top in each region lets a ray step across empty
air.

**Age.** Each part of the map holds the field at the tick it was last written, and the map keeps that tick for
each part. The cadence is a bound on a part's age in **weather time** and in the camera's travel through the
air, never in frames: half a second of weather in the first profile, which is 2 Hz at 1× and the whole map
every frame at 60×. The Sun's path is bounded the same way, level by level. So a time-lapse refreshes more each
frame and replaces no texel with one further on in the weather than real time would.

The GPU column matches the CPU evaluator at texel centres, at the tick each part was written, within a tolerance
that M2 measures first and then asserts; that no part is older than its bound is asserted apart from it.

### The Sun's path: one optical-depth product

This section describes the product's **cloud band**. With ground condensate it continues to the ground in a
[ground band](#condensate-that-meets-the-ground-mist-fog-and-low-cloud), and the two add along the Sun's ray.

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

`L = σ · [ Σₙ wₙ · E_sun(point) · exp(−bₙ·tau_sun) · Pₙ(θ) + L_ambient ]`

- **Sun.** `E_sun` is the lent Sun light at a world position: the true Sun through the air, with no cloud factor.
  It is evaluated at the layer's base and top on each ray and interpolated by height, so a tall cloud's top
  stays lit after its base has gone into the Earth's shadow, and sunset colour comes from the sky's own
  transmittance.
- **Orders.** Four, with the retained reference's `WEIGHTS` and `BRANCH_B`. Each order reuses the same `tau_sun`,
  so multiple scattering costs no density read. `Pₙ` is the order's phase.
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
  bound M1 measures: 1,025 entries hold the integral to 1.0001 for a 50 µm droplet, and 257 to 1.002.
- **The later orders** keep the reference's Henyey–Greenstein lobes, `LIQUID_G[1..3]`: light scattered many times
  has forgotten the droplet.
- **Once a ray.** A ray has one angle `θ` to the Sun's centre, so all four phases are one evaluation a ray: a
  read of the table and three closed forms, in f32 from `sin²(θ/2)` so the forward peak keeps its digits. No
  sample pays for the phase.
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
   takes steps that lengthen with distance inside cloud, and stops when it is opaque. It yields the segment's
   scattered radiance `S`, transmittance `T` and a front distance averaged by transmittance.
2. **Resolve.** Every texel of the layer's history is written each frame: the fresh sample where one was traced,
   otherwise the previous frame's value, reprojected through the cloud's own position and wind drift and clamped
   to the fresh samples around it. The history is a ping-pong pair whose producer runs every frame. On a reset
   the layer is filled from the frame's trace alone and is sharp again within one refresh cycle (four frames in
   the first profile).
3. **Compose.** At render extent, for sky pixels: `colour = colour·T + T_air·S + (1 − T)·S_air`, where
   `(T_air, S_air)` is the lent aerial transport to the front distance. This applies aerial perspective to the
   cloud once and removes the part of the sky's own in-scatter the cloud hides.

**Not a private upscaler.** The port decision forbids a cloud path from render extent to display extent, and
lists the march's refresh fraction and resolution among its knobs. The layer ends at render extent; the engine's
reconstructor does the rest.

**The interval a ray marches.** A ray marches condensate over one interval: from where its
[far weight](#condensate-that-meets-the-ground-mist-fog-and-low-cloud) leaves zero, or where it enters the
weather's extent if that is further, to the scene's depth or the weather's far edge. In a scene with no ground
condensate, a camera below the lowest base and geometry below it, that interval is the cloud band and every ray
meets cloud before geometry or not at all, so M3 to M10 build and judge it there. The trace is written over the
general interval from M3, so that M15 changes what it is given and not how it marches.

**Geometry.** A layer texel stands for several pixels that may lie at different depths. Its ray is marched once,
to the farthest of them, and keeps the pair it had reached at the nearest of them as well: **two stops**. A pixel
takes its pair between the two by its own depth, so a ridge in front of far mist has a clean edge and no pixel
takes fog from behind the surface it shows. Where every pixel of a texel is beyond the condensate, as under a
cloud band that clears the land, the two stops are one and the compose is the depth test it was. Water writes
depth and is geometry here.

**Frame contract.** Clouds write no depth. Where a cloud holds most of a pixel it writes motion from its front
point, with wind drift, at unjittered positions; elsewhere the sky's motion stays. It writes reactive coverage
by opacity where the cloud changed beyond what motion explains. A cut, a weather seek, a profile change and a
library change mark the layer's history invalid; a knob applies the invalidation it declares.

**Segments.** The trace's segment is `(T, S)` over its interval, which begins at or beyond the handover. Nearer
than that, condensate is in `ATMOSPHERE`'s view volume, integrated jointly with the air; the two are ordered
along the ray and their weights sum to one (C08). Air inside the trace's interval is not integrated jointly: the
segment takes the air once, at its front distance. The error is bounded by the air's optical depth across the
interval, which haze makes larger for a deep bank of far mist; M15 measures it against the joint reference
before it asserts a bound.

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

A placed cell is a cell of the weather field whose cloud is an authored shape and not a dome. The field knows
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
  on the CPU and the GPU reads a texture of it; the engine never reads terrain meshes. From it the model derives
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
- **Held and drifting.** A drifting row is a function of `world point − travel` like every cloud, so a bank moves
  with the wind exactly. A held row takes its place from the surface field, which does not move, and its wisps
  from noise in the air frame: the mist stays in its valley while its wisps cross it, as valley mist does in a
  light wind.
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
- **Haze.** The weather state holds the **visual range of the clear air**, 3 to 300 km. It is aerosol, not
  condensate, and `ATMOSPHERE` owns what the air does with it
  ([ruling 8](#what-clouds-asks-of-other-modules)); the model turns the range into the aerosol extinction that
  module takes.
- **The query.** A point query also answers, for any world point at any height: the condensate's extinction in
  m⁻¹, the visual range there (3.912 over the extinction of condensate and haze together), and through the
  model's f64 reference the optical depth between any two points. A game asks whether one point can see another
  with no graphics device. Column optical depth includes the ground sources and keeps
  [what a query promises](#one-weather-field-in-the-air-frame).
- **One density, one Sun's path.** Ground sources add to the one density, in its full and coarse forms, before
  attenuation. The Sun's path gains a **ground band**: levels of camera-following windows over the surface
  field, each holding optical depth toward the Sun at a few heights above the surface, filled by marching the
  coarse density along the path direction from each node to where the ray leaves the band, plus the cloud band's
  value there. A point's `tau` is then one read: the ground band below its top, the cloud band above. A band of
  80 m columns and 9 heights holds a bank's Sun path to 0.01 RMS in transmittance where a level-layer rule is
  off by 0.04 to 0.31 and casts no shadow beside the bank (part V). While it marches, the fill also records
  where the surface field itself blocks the Sun, as a **surface visibility** kept apart from `tau`. Only medium
  samples beyond the reach of the scene's shadow cascades read it, so a ridge's shadow lies across far mist at
  dawn; no surface ever reads it, because surfaces have the scene's own shadows.
- **Near the camera: the view volume.** Fog near the camera meets the ground, water, props and the camera
  itself, and overlaps the air, which C08 requires to be integrated jointly. `ATMOSPHERE`'s view volume does
  that ([rulings 6 and 7](#what-clouds-asks-of-other-modules)). Each frame this module writes, for every cell
  of that volume, at the jittered point the volume names, three **medium coefficients**: the extinction `σ`;
  the solar kernel `σ · Σₙ wₙ · exp(−bₙ·tau_sun) · Pₙ(θ)`, which is [cloud light](#cloud-light)'s Sun term
  without the Sun; and the ambient source, cloud light's ambient term. Each is multiplied by the **near
  weight**. `ATMOSPHERE` multiplies the kernel by its Sun light at the point and by the scene's shadow there,
  adds the air's own coefficients and integrates once. This module never integrates the near field, and
  `ATMOSPHERE` never evaluates a density, a phase or a self-shadow.
- **Beyond it: the trace.** The trace marches every condensate, ground sources included, weighted by the **far
  weight**, lit by cloud light with the surface visibility on its Sun term, stopping at the scene's depth with
  [two stops](#the-visible-layer). A far bank is then drawn at the layer's grain, a texel for every four
  pixels, where the view volume's cells are sixteen pixels across.
- **The handover.** The near weight is 1 from the camera to the start of a ramp and falls to 0 at its end; the
  far weight is one minus it. The ramp is a profile value, 300 to 500 m from the camera in the first profile.
  Two integrators that each take a share of one extinction compose as ordered segments to the exact
  transmittance, and to the in-scatter within 0.002 for a ramp twice that long (part I). The distance is short
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
| **Shapes** | The library: its footprints, which the field's query reads, and its leased volumes, which the Sun's path reads for a shape's shade and the trace, with Sky, for its picture. By the field's hash or by the game | Shade |
| **Ground** | Condensate that meets the ground: the surface field and the fog table, which the field's query reads; the ground sources in the weather map; the Sun path's ground band; the medium coefficients `ATMOSPHERE`'s view volume integrates. With Sky, the trace carries the same condensate beyond the handover and the overlay includes it; without Sky it ends at the handover | Shade, and a scene that declares `ATMOSPHERE`'s view volume |

- **Headless** is Weather alone: a server answers point queries, and needs a library's footprints and never its
  volumes. With ground sources it needs the surface field and the fog table, and answers extinction, visual
  range and the optical depth between two points.
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
  place of a fragment pass or the reverse) is promoted only by a paired measured win with the same output within
  the conformance bounds.
- **Knobs.** Each is a named runtime value in `EFFECT-GRAPH`'s list with a declared invalidation: the trace's
  refresh fraction, the layer's scale, step length and sample cap, local light samples, the Sun's path cadence
  per level, weather map cadence, overlay cadence, placed range, the ground band's cadence and the medium coefficients' cadence. The handover
  distance is a profile value and not a knob, because it decides which integrator draws a thing. Extent knobs select a sub-rectangle of a
  fixed allocation, so no knob creates a resource or pipeline. None changes the weather field, the columns the
  weather map holds or what a query returns; the placed range changes only the map's presentation values, the
  slot and the volume form's weight, and through them which form of a placed cell is drawn and shaded.

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
  samples over 120 m it is 0.02. At 6° it is 0.05 to 0.08, which M5 judges by eye.
- **Cost.** A sky ray needs 17, 42 and 34 texture reads for fair, broken and overcast skies, against Broadsides'
  142 density evaluations. That is 3 to 7 million reads a frame for a 640×360 trace before local light samples.
- **K5.** Broadsides' hybrid stills and the growth track's blind review show the content reads; that one density
  function serves both on the GPU is first proved in M6.
- **K6.** Unrun. The layer's reconstruction and its look under TAAU are first seen in M3 and M9.

From `revision_proof.py`, run for the revision of 14 November against
[the first design review](../../evidence/CLOUDS/design-review-1.md)'s counterexamples:

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
  0.3 then 0.3.
- **Directed shapes and capabilities.** Unrun. Both are the placed cell and the profile already reproduced, with
  their inputs from the game; M1 and M8 prove them first.

From `second_revision_proof.py`, run after [the second review](../../evidence/CLOUDS/design-review.md), on its
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
  a hard opaque edge, are an edge placed within one column's width.

From `whole_atmosphere_proof.py`, run for the revision of 14 November for mist, fog and low cloud:

- **I, the handover.** Two integrators that each take a share of one extinction, shares summing to one, compose
  as ordered segments to the exact transmittance at any share. Their in-scatter is exact under uniform light.
  With a shadow edge or a tenfold fall of light inside a ramp 1 km long, at visual ranges of 400 m to 40 km, it
  is within 0.002 of a radiance near 1; with a 200 m ramp, within 0.0001.
- **II, pooling.** On relief of about 200 m with a datum averaged over 3 km, mist covers 19% of the ground at amount
  0, 47% at 0.4 and 89% at 1; ridges 40 m above the datum stay clear until 0.8. Depth changes by 0.14 m for an
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
- **Unrun:** the view volume's look and stability, which `ATMOSPHERE` M7 and M8 see first; the two stops at a
  geometry edge under reconstruction, first seen in M15; the far segment's air error in haze, measured in M15.

## The system as it runs

| State | Owner |
|---|---|
| The weather field, its state and step, the density function, the f64 references | `thrum-cloud-model` (this module), no GPU dependency |
| The weather map, noise, the Sun's path in both bands, the layer and its history, the overlay, placed-volume sampling, the medium coefficients it writes for the view volume | `thrum-clouds` (this module), an effect on `EFFECT-GRAPH`'s seam |
| The view volume: its grid, its history, the joint integration of air, haze and condensate near the camera, and the lent aerial operation that reads it | `ATMOSPHERE` |
| The air's haze: how much aerosol, as a visual range | The weather state; `ATMOSPHERE` owns what the air does with it |
| The surface field (ground heights and water levels) and the fog table | The game, like the type table |
| The committed weather state, with any directed shapes | The game's world; it travels in the tick page |
| Presets, the type table, looks, the library and its placement data, controls | The game (`examples/v1-game` behind its cloud selection; `examples/water` behind its own); never in engine crates or their tests |
| Sky, Sun, aerial, reflection and diffuse products; applying the cloud factor, the overlay and the haze scale | `ATMOSPHERE` |
| Stages, state commit, cadence, the knob list | `EFFECT-GRAPH` |
| Motion and reactive targets, history acceptance, reconstruction, the governor | `APPLE-PRESENTATION` |
| The residency of volume products | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's weather state and forms the travel at the display's time.
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
today. Rulings 6 to 8 are `ATMOSPHERE`'s own work, in its M7 and M8, and this module uses them as built.

1. **A provider may write the Sun-path optical depth on the GPU** (`ATMOSPHERE`, M2). Today's field is CPU bytes
   with a rigid drift, read along the true Sun, with a factor of exactly 1 below a lowest elevation and outside
   the region. A provider may instead register a texture its own `simulate` pass writes, with, each frame, its
   mapping, the path direction its columns follow, a border width and one `tau_far`. Such a field is read
   along its path direction at every Sun elevation, and a receiver's factor is `mix(exp(−tau_far), exp(−tau),
   w)` with the border weight `w` above, so it is continuous in the Sun's elevation and across the region's
   edge. The CPU-fed form states none of these and behaves exactly as now.
2. **Aerial transport to a far sky point** (`ATMOSPHERE`, M3): the lent operation beyond 32 km, as above.
3. **A sky overlay in the reflection and diffuse products** (`ATMOSPHERE`, M8). If the incident-light meter has a
   Sun term without the cloud factor, it gains it at its metering point.
4. **A haze scale on solar in-scatter** (`ATMOSPHERE`, M8).
5. **A cloud selection in two games** (`SHOWCASE`'s `examples/v1-game` from M2; `WATERS`'s `examples/water` in M11),
   off by default, leaving each game unchanged when off.

6. **The view volume takes a provider's medium coefficients** (`ATMOSPHERE` M8): extinction, a solar kernel and
   an ambient source at the sample points it gives, weighted by the provider's near weight, summed with the
   air's before integration.
7. **The Sun-path optical depth has a ground band** (`ATMOSPHERE` M8), read at the receiver's own position and
   added to the cloud band's.
8. **The air's haze is an input** (`ATMOSPHERE` M7): the aerosol's amount, which the weather state supplies.

Everything else is used as built: effect stages, histories and cadence; the sea's depth write, which the trace
treats as geometry; the temporal contribution; leases of volumes; typed controls.

## Cost account

Estimates, unmeasured, at a 2560×1440 render extent and 120 frames a second. Broadsides' figures are measured
there, not here.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Weather step | Tick, CPU | One record and at most 16 directed shapes | Microseconds | |
| Phases | Frame, once a ray; the tables at preparation | One table read and three closed forms; a table for each row of the type table, about 4 KiB each at 1,025 entries | None in the march | |
| Weather map | Half a second of weather, in parts: 2 Hz at 1× | 4 levels × 256², two textures | Under 0.1 ms a frame at 1× | 2 ms an update |
| Noise | Preparation | 128³ and 32³ | None in a frame | |
| Sun's path | At 1×, near level over 8 frames, mid 32, far 128 | 3 × 256² columns (80 m, 320 m and 1.6 km wide, the last reaching the horizon), 48, 32 and 16 slices of the coarse density, which reads the map and the shape noise, and a placed cell's slab where its volume form has weight: 6.3 M samples to fill, about 0.5 M a frame | 0.1–0.2 ms | 2.2 ms shadow |
| Ground field and far value | With the Sun's path; the far value each frame | 1024², and a mean over the outermost level's ground slice | Under 0.05 ms | |
| Time-lapse | Ages are held in weather time | At 60×, the whole map (262 k texels) and up to the Sun path's whole fill each frame | Unestimated beyond those counts; M2 records 1×, 8× and 60× | |
| Trace | Frame | 640×360 rays, three quarters sky: 3–7 M reads, up to 16 M with three local light samples | The largest term; M3 measures it | 268 ms; 33 M density evaluations |
| Resolve | Frame | 1280×720 texels | 0.1–0.2 ms | 0.9 ms |
| Compose | Frame | Sky pixels at render extent; a pass that loads scene colour | 0.2–0.4 ms and a tile pass break | |
| Sky overlay | An eighth a frame | 6 × 48² rays, coarse density | Under 0.05 ms | 15 ms capture |
| Ground sources in the weather map | With the map | The surface field and datum (two more channels a level), the fog amounts | Within the map's line | |
| Ground band of the Sun's path | With the Sun's path | 3 levels × 256² columns × 8 to 12 heights, with one visibility bit a node | As the cloud band, about a quarter of its samples | |
| Medium coefficients | Frame | One sample for each view-volume cell inside the handover (about 0.7 M of 0.9 to 1.4 M cells): the map, the shape noise, the Sun's path | Unmeasured; Frostbite's write and lighting of a like grid took 2.5 ms on a PlayStation 4. M14 records it | |
| Far condensate in the trace | Frame | The same rays, longer where mist lies between the handover and the land | Unmeasured; M15 records it beside M9's | |
| Memory | | Weather 4 MiB, noise 2, Sun's path 13, ground 2, layer history and trace 25, footprints about 1, library 27 MiB as R8 or 14 as BC4, growth track the same again; the library is held by every scene that declares Shapes, shadow-only included. With Ground: the surface field and datum about 2 MiB, the ground band about 4, the medium coefficients about 8 at 160×90×72 | About 47 MiB plus the library, and about 14 more with Ground | 62 MiB plus history |

**Records and tripwires, never gates** ([brief](../BRIEF.md), decision 3). `scripts/cloud_cost.py`, created in
M2, takes a paired record of one scene at one view, capture off: the scene with clouds and with them absent,
giving the whole frame, each cloud pass's GPU time by name, the bytes of each resource and the load on the
machine. A record trips a wire when the whole frame with clouds is over the display's refresh interval, when a
pass is about 25% or more over its last comparable record, or when a cost grows faster than the work that was
added. A tripped wire is investigated in the milestone that tripped it, and the record says what was measured
to find the cause and what was fixed or left as a follow-up in the log. Visible stutter is judged in the
`realtime` inspections.

The ambition is the display's full refresh, and whole-frame tuning is the later performance phase. **No timing
figure decides whether an item passes, whether a milestone is Done or whether the next one starts.**
`cloud_cost.py check` measures the scene again and fails only when the scene cannot be measured, when the
committed record lacks part of what a paired record holds, or when a wire the record trips has no investigation
beside it. Presented cadence on the built-in panel is measured once, attended, in M12.

## Risks

1. **The trace costs too much.** Early sign: M3's record. The wrong fix is dropping the local light samples or
   the detail noise below what the look needs without saying so. The designed ones: a sparser refresh (one texel
   in sixteen), a coarser far step, fewer local samples with a finer near level. M3 tries the ones its record
   points at and records what each did to the cost and to the look. Building goes on either way; what remains
   over the refresh interval is written up as a follow-up for the performance phase.
2. **Procedural cumulus reads as soft shelves**, as Broadsides' did. Early sign: M3's first stills. The mechanism
   (profile thresholding shape noise, flat shared bases, domes) is the shipped one; the look is tuned against
   M3's list with a fresh opinion. Three failed looks on one cue go to the log and end `BLOCKED`.
3. **The reconstructed layer ghosts or crawls.** Early sign: M3's pan. Fallback: refresh every texel every
   frame at a lower layer scale, which removes the layer's history from the picture.
4. **Low-Sun relief is lost in the summed product**, or its coarse density shades an eroded edge visibly
   differently from the cloud that is drawn. Early sign: M3's record of the two densities' Sun paths, and M5's
   low-Sun stills against the oracle. Fallback: more local samples at low Sun, or a finer near level. The same
   holds for a placed shape, whose shade edge the product places within one column's width (part H): M6's
   record and its stills of a shape's shade are the early sign.
5. **`ATMOSPHERE`'s products resist the overlay**, because their storage is relative to the Sun. Early sign:
   M8's first test. The ruling states behaviour, not storage; if the producers cannot compose it, the proposal
   goes to that module's log.
6. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M2 runs under WebGPU's default
   limits on the Mac, and M4 is the first real Chrome run.
7. **The droplet's forward peak aliases or flickers** in the part-extent trace, as a ring or a crawling aureole
   around the Sun through thin cloud. Early sign: M5's stills and pan toward the Sun. The fix is in the phase's
   evaluation (its precision, its table's spacing, or the angle taken at the texel's centre), never a return to
   the wide lobe or to a rule that loses energy; a game that wants a softer look gives its own four numbers.
8. **The handover shows**, as a ring a few hundred metres out where mist is softer on one side than the other, or
   where a low deck overhead is drawn at the volume's cell size. Early sign: M15's walk and pan. The fixes, in
   order: a longer ramp; a nearer handover; a finer grid inside it. Never a second density for the near field.
9. **Mist reads as a flat grey slab**, without the glow, the soft top and the wisps of the shipped bar. Early
   sign: M14's first stills. The look is in the fog table, the noise and the phase, and is tuned against M14's
   list with a fresh opinion; three failed looks on one cue go to the log and end `BLOCKED`.
10. **The two stops leave a halo at geometry edges** in far mist, or history ghosts where a mover crosses it.
   Early sign: M15's stills of a ridge against mist. Fallback: refresh every texel of the layer every frame
   where the two stops differ.
11. **Far mist in thick haze takes its air wrongly**, as a bank too bright or too blue against the land behind
   it. Early sign: M15's record. Fallback: the trace takes the lent aerial transport at several depths along
   its interval and not at one.
12. **The V1 game changes under the selection.** `SHOWCASE` edits the same game. The verifier and the off-parity
   check run in every milestone.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. `ios-sim` is not a target. Chrome checks need a
  browser launch outside the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
  Never substitute a Mac result.
- **Capture.** Inspected items use T2 engine capture and name their mode.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/clouds-<milestone>/`, selected evidence in
  `docs/evidence/CLOUDS/`.
- **Exact baselines.** The module adds crates and a selection that is off by default, and moves no retained
  image. Every milestone carries the verifier as its claim-time check
  ([rules](../ARCHITECTURE.md#exact-baselines)).
- **Broadsides.** Read it at the commits [What Broadsides offers](#what-broadsides-offers) names; never change
  it, and never build against it. A file may be copied or translated where it does what this design states, on
  the owner's words of 14 November. Each copied or translated file is listed in
  `docs/evidence/CLOUDS/PROVENANCE.md` with its source commit and path, goes through review like any other code,
  and meets this module's oracles and items: a verdict it passed in Broadsides is not a verdict here. Material
  that came into Broadsides from elsewhere keeps its own licence, recorded with it, and nothing GPL or
  share-alike is copied. The cloud library is read from Git LFS without changing the Broadsides checkout, and its
  licence record is copied beside it.
- **Game content stays in the game.** Presets, type tables, looks, volumes and the library never enter
  `thrum-cloud-model`, `thrum-clouds` or their tests; tests make their own inputs.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** The ledger's `E1.clouds` row is kept current from M2, with implementation, correctness, visuals
  and device apart.

## Open questions

| Question | Settled by |
|---|---|
| What does the trace cost at 2560×1440 against the refresh interval, and at which refresh fraction and local sample count? | M3 records; M12 records again at the showcase's extent on both recipes |
| How far is the coarse density's Sun path from the full density's at an eroded edge, on the GPU? | M3 records it beside the design proof's 0.001 RMS |
| What do the weather map and the Sun's path cost at 8× and at 60×, where their ages are held in weather time? | M2 records |
| What tolerance does the GPU weather map meet against the CPU field on Metal and in Chrome? | M2 and M4 measure first, then assert the recorded bound |
| How many sampled textures does the lent environment group use in the trace? | M3 counts against the record |
| Does the summed Sun-path product carry low-Sun relief with three local samples? | M5 |
| Does the droplet's forward peak hold steady in the part-extent trace? | M5 |
| How many entries does the phase table need to hold its integral and its values near the Sun? | M1 measures first, beside part G's 1,025 |
| How far is the product over a placed shape from the volume's own Sun path, at its edges and inside it, on the GPU? | M6 measures first, beside part H's figures |
| Does the incident-light meter already see cloud shade? | M8 reads the code and measures |
| Can the scene's residency take the library as a second source folder? | M6; if not, the proposal goes to `CONTENT-LOOP`'s log |
| Is an Apple-tile kernel variant a measured win? | M12 |
| How wide a datum window pools mist as the valley's relief wants? | M13 sets the profile's value on the valley's heights; M14 judges it |
| What do the ground band and the medium coefficients cost, on the Mac and in Chrome? | M14 records |
| How far is the far segment's air from a joint integration, in haze of 8, 20 and 60 km? | M15 measures first |
| Does the handover show at 300 to 500 m, and at what ramp? | M15 |
| Can `examples/water` give a surface field from its water map without a change to `WATERS`? | M11; if not, the proposal goes to `WATERS`' log |
