# Clouds: a weather-driven sky that shades and lights the world

13 November 2026, revised 14 November and judged again from first principles the same day. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md). C02, C04, C06, C07, C08, C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[ENVIRONMENT](../../design/modules/ENVIRONMENT.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.
[The port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md) records what this design selects and how
Broadsides may be reused. This design replaces the implementation target in
[CLOUDS.md](../../design/modules/CLOUDS.md); that page's ownership and transport rules stay in force and are
restated where they bind.

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

The lead's reading, recorded as [assumptions](PLAN.md#open-for-the-human): the owner's last words govern; the
design of 13 November, [the brief](BRIEF.md) of 10 November and the port decision describe the Broadsides
approach and are inputs to judge, not constraints; Broadsides code, shaders and assets may be copied and reused
as is wherever this design finds them useful; Broadsides itself stays unmodified.

So this design starts from who uses a sky in Thrum and what each needs, chooses its mechanisms against that, and
only then says what Broadsides has that serves them. [Judged from first principles](#judged-from-first-principles)
gives the verdict on every choice the design of 13 November inherited: six changed, and the rest are kept for
reasons that do not rest on Broadsides having built them. [What Broadsides offers](#what-broadsides-offers) says
what may be copied.

It is not a blind study. Read for it: the design of 13 November and [its review](../../evidence/CLOUDS/design-review.md);
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

**Commissioned:** cues K1 to K7 on `mac-metal` and `chrome-webgpu`, lit only by `ATMOSPHERE`'s products.

**Not built**, from the brief: the ten cloud genera, cumulonimbus with anvil and rain shafts, the warm-front
sequence and high ice layers; two-layer maritime decks from large-eddy simulation; rain, lightning, moonlit cloud
and flying through cloud; a cloud-formation teaching demo. Also not built: a camera at or above the cloud base,
opaque geometry that rises into the cloud layer, light shafts in haze, precipitation fields, saving the weather
state to disk, a cloud whose density a simulation writes each tick, and a cloud held over a place against the
wind. The cloud-formation demo would be the first of those two: a [directed shape](#placed-volumes) whose slab an
effect writes. The slab, its footprint and its place in the march are designed for it; the writer is not built.

**What exists.** Thrum has no cloud renderer. `ATMOSPHERE` M5 applies one Sun-path optical depth on every
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

Finished also means, from the brief: the weather field is a deterministic function of seed, preset, integrated
wind travel and simulation time, with a CPU evaluator that answers point queries headless and a test that the GPU
map matches it; all consumers share extinction in m⁻¹; aerial perspective is applied once; cloud and fog segments
are ordered and disjoint; visible clouds off releases presentation resources and keeps shade; every cadence is a
governor knob with a declared history invalidation; a capability a scene leaves out costs it nothing; and the cost account holds at full refresh on this Mac at the
showcase extent, with texture memory recorded, by paired records and tripwires rather than a budget gate.

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
  which carries the fits and a test that reproduces the paper's parameters at 20 µm; the paper's own file could
  not be opened in this session. Part E of the proof below evaluates it.
- **Looked for, 14 November:** a successor to these methods published from 2024 to 2026. Found: a 2026 method that
  relights clouds as meshes with a learned encoding (Olajos, Doggett and Goswami), and a hybrid of sparse voxels
  and noise. Titles and abstracts only. Neither is shipped practice or runs in a browser's limits, and neither
  changes a selection below.
- **Not read:** the Red Dead Redemption 2 course notes (Bauer 2019; not found to fetch), the 2015 and 2017 Nubis
  talks, and the Broadsides cloud shaders beyond their names and sizes. Nothing below rests on them.

Three reproductions were run for this design, the second for its revision and the third for the first-principles
judgement, and are kept with their output in
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
| The first scattering order's phase is the retained reference's Henyey–Greenstein lobe, g = 0.86 | **Change** | The design already takes extinction from droplet radius; the phase now comes from the same droplet. Against the droplet's own phase the lobe is three to four times too bright from 5° to 10° off the Sun, 46 times too dim inside the aureole, and two to three times too dim looking away from the Sun (part E). The first is a wide glow where a silver lining should be a rim; the last is the brightness of a cloud with the Sun behind the viewer. The angle is one value a ray, so the better phase costs the march nothing. See [cloud light](#cloud-light) |
| Drift taken at the committed tick | **Change** | A 60 Hz tick is shown twice on a 120 Hz display. At a 60× time-lapse in a 40 m/s wind a near cloud would stand still for one frame and jump 37 pixels the next (part F). Travel is a function of time, so each frame takes it at the display's time, as `WATERS` does its waves. See [time](#time-drift-at-the-displays-time-evolution-on-the-tick) |
| A placed volume stands for one cell of the field | **Keep** | It is why the query, the picture and the shade agree on a shape's outline at any distance, and why a shape forms, drifts and thins like any cloud. It does not rest on the library existing |
| Shapes are placed only by the field's hash | **Change** | A demo composes its sky: a tower beside the Sun at sunset. A profile places shapes by the hash or by the game's own list of [directed shapes](#placed-volumes), which are cells whose place, shape and life the game sets |
| Shadow-only as the one reduced profile | **Change** | Shade, the visible sky, the world's light and shapes are each a capability, and a scene declares only those it uses. A sunset demo needs no library; a top-down game needs only shade |
| Clouds shade and light the world through `ATMOSPHERE`, never by themselves | **Keep** | One owner applies each factor once. A private sky or a second shadow term is what made Broadsides' verdicts unusable here |
| The layer ends at render extent and the engine reconstructs | **Keep** | One reconstructor sees every drawer's motion. Unreal, Frostbite and Decima all trace clouds below the render extent and hand a full-extent layer on |
| Nothing copied from Broadsides until a licence is named; stand-in shapes first | **Change** | The owner's words of 14 November. The library is the example's content from M6. See below |

### What Broadsides offers

Broadsides code, shaders and assets may be copied into this repository and used as they are, wherever they
implement a mechanism of this design as this design states it. Where they implement something else they are
reading matter. The builder of each milestone judges fit file by file; this table says where to look. Paths are in
the Broadsides repository at `b4d8250e6` unless a commit is given, and nothing there is ever changed. Of these,
the planner opened `scatter.rs`, `phase.rs`, the library's licence record and handoff, and the notes named in
[the request](#the-request); the shaders' fit is expected from their names and the notes, not read.

| Mechanism here | Where Broadsides has it | Expected fit |
|---|---|---|
| The droplet phase: the published fit, both lobes written to stay exact near the forward peak, the Sun's disc as a spread, the mean cosine, and the energy-conserving step | `crates/core/bw_core/src/medium/scatter.rs`, with its tests; the GPU twin `crates/gpu/bw_gpu/shaders/lib/medium.wesl` | Close. This is [cloud light](#cloud-light)'s first order as stated |
| Phases evaluated once and looked up by angle | `crates/render/bw_render_clouds/src/phase.rs`, `shaders/cloud_phase_table.wesl` | Useful if a table measures faster than the closed form; here the angle is one value a ray, so the closed form may be enough |
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
directed ones in one sky, the map's texel gains a second slot; nothing else moves.

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
sample's footprint grows past the detail noise, and the Sun path's columns are wider than that. The model
evaluates both in f64, and each consumer is held to the reference for the density it uses.

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

One product holds optical depth along the Sun's ray, from the top of the cloud layer down: `tau(column,
height)`, in levels of camera-following windows whose columns are Sun rays. A column is indexed by where its ray
crosses the bottom of the layer. It is filled by marching the **coarse density** along each ray, time-sliced by
tiles of columns, and stored in place. It integrates the same density in both profiles and never reads the
detail noise, so its bytes do not depend on whether clouds are drawn.

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
  d'Eon 2023), `(1 − w)·HG(θ, g_HG) + w·Draine(θ, g_D, α)`, with the Henyey–Greenstein lobe widened by the Sun's
  disc (its squared angle gains half the squared angular radius of the lent Sun), which bounds the forward peak.
  The four parameters are type data, interpolated along the type axis. `thrum-cloud-model` gives the published fit
  from a droplet diameter of 5 to 50 µm, so a table takes its phase from the droplet radius its extinction comes
  from, and a game that wants another look gives its own four numbers. This lobe is the silver lining: a rim
  within a few degrees of the Sun, not a glow 20° wide, and a brighter cloud with the Sun behind the viewer.
- **The later orders** keep the reference's Henyey–Greenstein lobes, `LIQUID_G[1..3]`: light scattered many times
  has forgotten the droplet.
- **Once a ray.** The Sun is one direction, so the angle `θ` and all four phases are one evaluation a ray, in
  f32 from `sin²(θ/2)` so the forward peak keeps its digits. No sample pays for the phase.
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

**Geometry.** The supported range is opaque geometry below the lowest cloud base and a camera below it. There,
every ray meets geometry before it could meet cloud, so the trace ignores depth except to skip covered rays, and
the compose is a depth test at render extent with a clean edge. Outside that range a pixel whose geometry is
nearer than the cloud's front distance shows no cloud; partial immersion is not modelled.

**Frame contract.** Clouds write no depth. Where a cloud holds most of a pixel it writes motion from its front
point, with wind drift, at unjittered positions; elsewhere the sky's motion stays. It writes reactive coverage
by opacity where the cloud changed beyond what motion explains. A cut, a weather seek, a profile change and a
library change mark the layer's history invalid; a knob applies the invalidation it declares.

**Segments.** The cloud segment is `(T, S)` over an interval that never starts below the profile's lowest base.
Thrum has no fog provider today; one added later lies below that height and composes ordered and disjoint (C08).
Air inside the cloud interval is not integrated jointly; its error is bounded by the air's optical depth across
the interval and is stated in the documentation.

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
  camera with clouds over the same without. It is Frostbite's equation 22 as a scalar. It makes no shafts and
  no colour shift, and says so.

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
- **Reading a slab.** The march samples no level coarser than the one that holds one texel a slab, and holds its
  coordinate along the stacking axis half a texel of the coarser level it reads inside the slab. Up to that
  level every texel is an average of one slab alone; the next level averages neighbouring shapes, and is never
  read. A cloud too far for that level is past the placed range and drawn in its column form. Growth keys are
  read the same way, and two keys are blended by weight, never filtered along the stack.
- **Life.** The cell's envelope drives both forms' opacity and edge erosion, so the cloud forms and thins with
  its cell. A growth-track instance blends its two keys around its age, or around its growth where the game
  directs it, and its footprint is the same blend of theirs.
- **Capacity.** The number of volume-form instances is a profile limit checked against the capability record.
  Placed cells beyond it, and every placed cell while no lease is resident, are drawn in their column form,
  nearest first, and the refusal is counted and reported. The query is the same either way.

### Capabilities, shadow-only and headless

A scene's cloud profile names the capabilities it uses. One it leaves out declares no pass, resource or pipeline
and costs nothing; changing the set rebuilds the effect and starts a new epoch.

| Capability | What it adds | Needs |
|---|---|---|
| **Weather** | The state, its step and the point query, in `thrum-cloud-model`. No graphics device, nothing allocated | Always present |
| **Shade** | The weather map, the shape noise and the Sun's path, with the ground field `ATMOSPHERE` applies | Weather |
| **Sky** | The visible layer: the trace, the layer's history, the detail noise, the compose and the far-aerial use | Shade |
| **World light** | The sky overlay and the haze scale | Sky |
| **Shapes** | The library's footprints, with Shade; its leased volumes, with Sky. By the field's hash or by the game | Shade |

- **Headless** is Weather alone: a server answers point queries, and needs a library's footprints and never its
  volumes.
- **Shadow-only** is Weather and Shade, with Shapes if the sky has them. The Sun's path integrates the coarse
  density whatever else is declared, so its ground field is the same bytes with or without Sky and World light,
  under a sky whose full density the detail noise does erode.
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
  per level, weather map cadence, overlay cadence, and placed range. Extent knobs select a sub-rectangle of a
  fixed allocation, so no knob creates a resource or pipeline. None changes the weather field, the columns the
  weather map holds or what a query returns; the placed range changes only the map's presentation values, the
  slot and the volume form's weight.

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
[the design review](../../evidence/CLOUDS/design-review.md)'s counterexamples:

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
  at half a degree, and 2.0 to 3.4 times too dim from 140° to 180°. Averaged over the Sun's disc the droplet's
  forward peak is 1,709 sr⁻¹, down from 2,743 for a point Sun.
- **Drift.** At 60× in a 40 m/s wind a cloud 1,500 m off moves 37 pixels a tick at 1440 rows. Taken at the tick
  it is shown as 0 then 37; at the display's time, as 18 then 18. At 1× the same wind gives 0 then 0.6 against
  0.3 then 0.3.
- **Directed shapes and capabilities.** Unrun. Both are the placed cell and the profile already reproduced, with
  their inputs from the game; M1 and M8 prove them first.

## The system as it runs

| State | Owner |
|---|---|
| The weather field, its state and step, the density function, the f64 references | `thrum-cloud-model` (this module), no GPU dependency |
| The weather map, noise, the Sun's path, the layer and its history, the overlay, placed-volume sampling | `thrum-clouds` (this module), an effect on `EFFECT-GRAPH`'s seam |
| The committed weather state, with any directed shapes | The game's world; it travels in the tick page |
| Presets, the type table, looks, the library and its placement data, controls | The game (`examples/v1-game` behind its cloud selection; `examples/water` behind its own); never in engine crates or their tests |
| Sky, Sun, aerial, reflection and diffuse products; applying the cloud factor, the overlay and the haze scale | `ATMOSPHERE` |
| Stages, state commit, cadence, the knob list | `EFFECT-GRAPH` |
| Motion and reactive targets, history acceptance, reconstruction, the governor | `APPLE-PRESENTATION` |
| The residency of volume products | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's weather state and forms the travel at the display's time.
`simulate`, on a new tick only: the due part of the weather map; the
due tiles of the Sun's path and the ground field; the due part of the overlay. `ATMOSPHERE` filters its
reflection and diffuse products with the overlay. Sun passes and forward draw, with the cloud factor on every
direct-Sun term. Sky and stars draw. `after-opaque`: the sea, if the scene has one; then trace, resolve and
compose. Reconstruction, tone and present follow. On submission the effect commits its tick, anchors, refresh
phase and the history side.

**The example.** `examples/v1-game` gains a cloud selection, off by default. With it on, the game keeps a weather
state in its world, adds the cloud contribution after any sea, and shows cloud controls. With it off the game,
its world, its page and every frame are what they were. The selection replaces the diagnostic shade provider.

**Creator journey.** A scene adds the cloud contribution with a profile that names its capabilities, a type table
and, if it has shapes, their footprints; it steps a weather state in its world, and puts it in its page. A scene
that composes its sky adds directed shapes to that state. Changing coverage, type, wind or time scale needs no rebuild. Changing the
profile or the library rebuilds the effect and resets its histories.

## What CLOUDS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-clouds). Each is built by this module's builder
in the owning module's files, with that module's items unchanged and its tests kept passing, and each is absent
from a scene with no cloud provider: the same shader text, passes, storage and bytes as today.

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

Everything else is used as built: effect stages, histories and cadence; the sea's depth write, which the trace
treats as geometry; the temporal contribution; leases of volumes; typed controls.

## Cost account

Estimates, unmeasured, at a 2560×1440 render extent and 120 frames a second. Broadsides' figures are measured
there, not here.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Weather step | Tick, CPU | One record and at most 16 directed shapes | Microseconds | |
| Phases | Frame, once a ray | Four closed forms | None in the march | |
| Weather map | Half a second of weather, in parts: 2 Hz at 1× | 4 levels × 256², two textures | Under 0.1 ms a frame at 1× | 2 ms an update |
| Noise | Preparation | 128³ and 32³ | None in a frame | |
| Sun's path | At 1×, near level over 8 frames, mid 32, far 128 | 3 × 256² columns (80 m, 320 m and 1.6 km wide, the last reaching the horizon), 48, 32 and 16 slices of the coarse density, which reads the map and the shape noise: 6.3 M samples to fill, about 0.5 M a frame | 0.1–0.2 ms | 2.2 ms shadow |
| Ground field and far value | With the Sun's path; the far value each frame | 1024², and a mean over the outermost level's ground slice | Under 0.05 ms | |
| Time-lapse | Ages are held in weather time | At 60×, the whole map (262 k texels) and up to the Sun path's whole fill each frame | Unestimated beyond those counts; M2 records 1×, 8× and 60× | |
| Trace | Frame | 640×360 rays, three quarters sky: 3–7 M reads, up to 16 M with three local light samples | The largest term; M3 measures it | 268 ms; 33 M density evaluations |
| Resolve | Frame | 1280×720 texels | 0.1–0.2 ms | 0.9 ms |
| Compose | Frame | Sky pixels at render extent; a pass that loads scene colour | 0.2–0.4 ms and a tile pass break | |
| Sky overlay | An eighth a frame | 6 × 48² rays, coarse density | Under 0.05 ms | 15 ms capture |
| Memory | | Weather 4 MiB, noise 2, Sun's path 13, ground 2, layer history and trace 25, footprints about 1, library 27 MiB as R8 or 14 as BC4, growth track the same again | About 47 MiB plus the library | 62 MiB plus history |

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
   low-Sun stills against the oracle. Fallback: more local samples at low Sun, or a finer near level.
5. **`ATMOSPHERE`'s products resist the overlay**, because their storage is relative to the Sun. Early sign:
   M8's first test. The ruling states behaviour, not storage; if the producers cannot compose it, the proposal
   goes to that module's log.
6. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M2 runs under WebGPU's default
   limits on the Mac, and M4 is the first real Chrome run.
7. **The droplet's forward peak aliases or flickers** in the part-extent trace, as a ring or a crawling aureole
   around the Sun through thin cloud. Early sign: M5's stills and pan toward the Sun. The fix is in the phase's
   evaluation (its precision, or the angle taken at the texel's centre), never a return to the wide lobe; a game
   that wants a softer look gives its own four numbers.
8. **The V1 game changes under the selection.** `SHOWCASE` edits the same game. The verifier and the off-parity
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
| Does the incident-light meter already see cloud shade? | M8 reads the code and measures |
| Can the scene's residency take the library as a second source folder? | M6; if not, the proposal goes to `CONTENT-LOOP`'s log |
| Is an Apple-tile kernel variant a measured win? | M12 |
