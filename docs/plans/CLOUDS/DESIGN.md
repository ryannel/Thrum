# Clouds: a weather-driven sky that shades and lights the world

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [brief](BRIEF.md) ·
[module list](../ARCHITECTURE.md). C02, C04, C06, C07, C08, C09 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md), [ENVIRONMENT](../../design/modules/ENVIRONMENT.md)
and [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern. This design replaces
the implementation target in [CLOUDS.md](../../design/modules/CLOUDS.md); that page's ownership and transport
rules stay in force and are restated where they bind.

## The request

The lead's planning task of 13 November 2026:

> Plan the CLOUDS module (docs/plans/CLOUDS/BRIEF.md). The owner, asked whether SEA and CLOUDS planning could start
> now rather than waiting on their port timing, said (13 Nov 2026): "On 2 use the fable planner like normal but you
> can bring that across when it makes sense." Lead's reading, to record as an assumption: planning starts now, and
> the owner leaves to this plan when the Broadsides Rust work is brought across, within
> docs/design/decisions/SEA-AND-CLOUD-PORT.md (module by module, no wholesale import, Broadsides untouched).
> Milestones that depend on unfinished modules wait on them as usual.

The owner's request of 10 November, from [the brief](BRIEF.md): "Two cloud paths, a baked library and a procedural
one ... The owner has now asked for both to be finished and consolidated." The brief fixes where clouds are
finished (in Thrum, as one module), what finished means (cues K1 to K7 below, in its words), what is not part of
finished, the shape of the consolidation, and the commits Broadsides is read at. It leaves four decisions to this
design, settled in [Mechanisms](#mechanisms): the cost account before the march is chosen, how placed volumes and
weather layers share one march, the Sun direction clouds are lit from, and which of the procedural path's opt-in
looks become defaults.

This is not a blind study. Read for it: the Broadsides procedural notes at `b4d8250e6` (`THRUM-PORT`,
`RUNTIME-DESIGN`, `COST-2026-11-06`, `COST-MODEL`, `ASSET-CONTRACT`, `PROCEDURAL-COMPLETION`, and
`bw_core/src/weather.rs` with its data file), the baked path at `79637f789` (the library's README, licence and
`B3-HANDOFF`, the growth proof and its motion review, the first sections of `PIPELINE-DESIGN`), and in this
repository the designs of `EFFECT-GRAPH`, `ATMOSPHERE` (environment products), `CONTENT-LOOP` (leases), `SEA`,
with `atmosphere/cloud.rs`, `cloud.wgsl` and the diagnostic provider in `examples/v1-game`.

## Purpose and scope

A reusable engine sky: a game adds the cloud contribution to a scene, keeps a weather state in its world, and
gets clouds, their shade on everything the Sun lights, and their light in the world's ambient and reflections.
Consumers are the V1 valley and the sea scene (both behind a selection), later teaching demos, and `SEA`.

**Commissioned:** cues K1 to K7 on `mac-metal` and `chrome-webgpu`, lit only by `ATMOSPHERE`'s products.

**Not built**, from the brief: the ten cloud genera, cumulonimbus with anvil and rain shafts, the warm-front
sequence and high ice layers; two-layer maritime decks from large-eddy simulation; rain, lightning, moonlit cloud
and flying through cloud; a cloud-formation teaching demo. Also not built: a camera at or above the cloud base,
opaque geometry that rises into the cloud layer, light shafts in haze, precipitation fields, and saving the
weather state to disk.

**What exists.** Thrum has no cloud renderer. `ATMOSPHERE` M5 applies one Sun-path optical depth on every
direct-Sun receiver, fed today by a CPU-made diagnostic field in `examples/v1-game`. Broadsides has both paths:
the procedural one at a median 289 ms a frame for its fair preset, and the baked one paused on its atlas reader,
which is why both are references and not products ([brief](BRIEF.md#where-it-is-finished)).

## Cues

The cue and range columns are the brief's, unchanged. The bar for every inspected cue is the skies of Red Dead
Redemption 2 and Horizon Forbidden West, in words, at the display's full refresh.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **K1** A weather-driven sky: scattered fair cumulus with shared bases and varied sizes, a broken deck, and overcast, each recognisable and continuous to the horizon | Clear, fair, broken and overcast presets and the transitions between them; ground and deck cameras 2–40 m up; the full look arc | A tiled cloud texture or a sky dome. Rejected by parallax between near and far clouds as the camera moves 40 m, and by a point query that agrees with what is drawn overhead |
| **K2** Light that reads as cloud: bright sunlit sides grading into shadowed bases, a silver lining toward the Sun, and sunset and twilight colour taken from the sky | Noon, low Sun, backlit, sunset through twilight to night, on the one G3s Sun and sky | A lit albedo with a gradient. Rejected by one cloud seen from its Sun side and its far side, and by a tall cloud whose top is lit after its base has gone dark |
| **K3** Shade that belongs to the clouds: patches of sunlight and shade cross terrain, objects and water in step with the clouds above | One optical depth applied once per receiver; also with visible clouds off (shadow-only) | A scrolling shadow texture. Rejected by a shade edge that lies under its cloud's edge along the Sun's ray at two Sun elevations, and by identical shade with visible clouds off |
| **K4** Clouds that move and change: they drift with the wind in world space and form, grow and thin over time | 0–40 m/s wind; real time and a time-lapse control; no popping, sliding texture or periodic slots | Noise scrolled through a fixed mask. Rejected by tracking one cloud for its whole life in a time-lapse: it is born, rises, and thins away, and nothing reappears where it stood on a beat |
| **K5** Placed volumes: clouds from the baked library stand in the same sky under the same light, and one grows through its staged track | The twelve accepted volumes and the congestus growth track, cooked as content products; placement by the weather field | A separate pass composited over the sky. Rejected by a placed cloud partly behind a procedural one with correct occlusion both ways, and by ground shade that has the placed cloud's outline |
| **K6** Stable in motion: no shimmer, crawl or ghosting under reconstruction as the camera turns and time runs | TAAU on portable and Chrome, MetalFX on Apple tile; motion, reactive and reset written per the frame contract | Heavy history that hides noise while the camera is still. Rejected by a fast pan, a cut and a 60× time-lapse judged frame by frame |
| **K7** Controls a visitor can use: coverage, cloud type, wind, time of day and an internal-field overlay, on Mac and in Chrome | The typed controls of `CREATOR` | A control that rebuilds the sky. Rejected by a coverage sweep in motion with no pop |

Finished also means, from the brief: the weather field is a deterministic function of seed, preset, integrated
wind travel and simulation time, with a CPU evaluator that answers point queries headless and a test that the GPU
map matches it; all consumers share extinction in m⁻¹; aerial perspective is applied once; cloud and fog segments
are ordered and disjoint; visible clouds off releases presentation resources and keeps shade; every cadence is a
governor knob with a declared history invalidation; and the cost account holds at full refresh on this Mac at the
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
  They are the octave model above with fixed numbers, and they are this design's optical oracle.
- **Broadsides**, as what was tried. Its cost model counts 141.7 density evaluations a ray: 19.7 on the view ray,
  39.8 in a six-tap Sun cone and 82.2 in direct ambient. Lighting was 86% of the work, and removing both lighting
  terms still left 21 ms. Its accepted results are the twelve volumes, the growth track's blind motion review,
  and the rule that the true Sun, not the apparent surface light, lights a medium. Its fair preset's "flat, soft
  shelves" and four-band base look were not accepted.
- **Not read:** the Red Dead Redemption 2 course notes (Bauer 2019; not found to fetch), the 2015 and 2017 Nubis
  talks, and the Broadsides shaders. Nothing below rests on them.

One reproduction was run for this design and is kept with its output in
[the evidence folder](../../evidence/CLOUDS/design-proof/). [Check that it can work](#check-that-it-can-work)
reports it.

**How a march reaches the look at this frame rate, decided against the cost.**

| Candidate | Verdict |
|---|---|
| March at render extent, lighting each sample with its own Sun cone and ambient rays (Broadsides) | Heavy by design: 33 million density evaluations a frame |
| Dense voxel grid of the whole sky | Nubis³ measured over 30 ms; memory grows with the horizon |
| Cloud cards or a panorama | Fails K1's parallax and K3's shade |
| A per-cloud cached volume (the earlier cohort lab) | 13 ms for one cloud on this Mac |
| **2D weather fields with a vertical profile, noise for detail, light from one summed Sun-path product, traced at part of the render extent and reconstructed over frames** | **Selected.** Every part is shipped practice; the work a ray does is bounded by a few texture reads a sample |

**What would change these selections.** If M3's record shows the trace cannot hold the refresh after the designed
remedies in [risk 1](#risks), the layer's extent and the light model are re-planned before M5. If M5 shows the
summed Sun-path product cannot carry low-Sun relief, its near level is refined or the local samples grow, here
first.

## Mechanisms

### One weather field, in the air frame

The **weather field** is the single authority. It is a pure function in `thrum-cloud-model`, evaluated in f64
with no GPU:

`field(seed, parameters, travel, weather time, world point) → coverage, type, base, top, age, column optical depth`

- **The weather state** is a small record the game keeps in its world and steps once a tick: the seed; the
  current parameters (coverage 0–1, a type axis 0–1 from layered to heaped, wind as a velocity at cloud height,
  a time scale for time-lapse); their targets and a response time; the **travel**, wind integrated per tick in
  f64 metres; and the **weather time**, the tick duration times the time scale, integrated the same way. Wind is
  never multiplied by absolute time. A parameter moves toward its target over the response time, so a preset
  change is a ramp. The state travels in the tick page (C02).
- **The air frame.** Every pattern is a function of `world point − travel` and weather time. So the whole field
  drifts with the wind in world space, exactly, and what changes in time is the weather and never a texture
  sliding through a mask. Noise that gives detail lives in the same frame.
- **Cells.** Heaped cloud is a set of convective cells on a lattice in the air frame. Each lattice site hosts
  one cell per generation period. A hash of the site and the generation gives the cell its birth time anywhere
  in the period, its lifetime (12 to 30 minutes in the first profile), its centre anywhere in the site, its
  size, its depth and a shape seed. A cell's **envelope** rises smoothly from zero at birth and returns to zero
  at death, and scales its radius, depth and density together: the cell forms, grows and thins. Coverage sets
  how many cells exist, through a smooth threshold on a per-cell hash, so raising coverage swells cells in and
  never switches one on. A cell's column is a dome over a flat base at the type's condensation level; bases are
  shared, sizes vary.
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

**Outputs.** Coverage is the column's profile, 0 in clear air. Age is the dominant cell's life phase. Column
optical depth is the vertical integral of the density below, before detail erosion: it bounds what is drawn. A
query outside nothing is invalid; valid clear weather returns zero.

### One density, in m⁻¹

Every consumer (the view, the Sun's path, the sky overlay, the CPU reference) evaluates one density function:

`σ(point) = σ_type · profile(height fraction) · shape(column profile, noise)`

The column profile thresholds a low-frequency 3D shape noise, so a cloud's interior is solid and its edge breaks
into billows; a finer detail noise erodes only the edge. Noise can remove extinction and never add it beyond the
column's bound. As a cell's envelope moves, the threshold sweeps through fixed noise, so billows appear and
dissolve as structure and nothing slides. Noise is read at a mip chosen from the sample's footprint, so a far
ray reads an average, not aliased detail. Both noise volumes are generated on the GPU at preparation from the
same integer hash on every target.

Coefficients from different condensate add before attenuation. Two forms of the same condensate (a cell and the
placed volume that stands for it) are blended with weights that sum to one. Nothing is darkened twice and no
density is divided by a count.

### The weather map

The GPU reads the field through a **weather map**: a few world-anchored wrapping levels in the air frame, whose
texel doubles or quadruples outward to the horizon. A texel holds the column: base, top, profile, type, and the
slot and weight of at most one placed volume. Compute passes in `simulate` evaluate the same cell and deck rules
as the CPU. Because the map lives in the air frame, wind costs nothing: only evolution and the camera's movement
through the air need new texels, so the map is refreshed in parts at a cadence of about 2 Hz. A coarse level of
the tallest top in each region lets a ray step across empty air.

The GPU map matches the CPU evaluator at texel centres within a tolerance that M2 measures first and then
asserts.

### The Sun's path: one optical-depth product

One product holds optical depth along the **true Sun's** ray, from the top of the cloud layer down:
`tau(column, height)`, in levels of camera-following windows whose columns are Sun rays. A column is indexed by
where its ray crosses the bottom of the layer. It is filled by marching the density function along each ray,
time-sliced by tiles of columns, and stored in place.

It has three readers and one meaning:

- **Ground shade.** A level's last slice is `tau` through the whole layer for a receiver below it. The slices of
  the levels are composed into one 2D field, which is registered with `ATMOSPHERE` as the Sun-path optical depth
  ([ruling 1](#what-clouds-asks-of-other-modules)). `ATMOSPHERE` applies `exp(−tau)` once on every direct-Sun
  receiver: terrain, instances, cooked meshes, the cooked volume and the sea. `CLOUDS` never darkens a receiver.
- **Self-shadow.** A cloud sample's light is `exp(−tau)` with `tau` in two disjoint parts that add: a short local
  march of a few samples of the full density toward the Sun, then the product read at the far end of that
  segment. The local part resolves lobes smaller than a column; the product carries everything beyond, to any
  distance. The sample count is a knob.
- **The sky overlay** below reads it the same way.

The Sun's direction is the environment identity's true Sun, never the apparent surface light. When it moves, the
columns shear with it; the refresh cadence bounds the lag, and a jump larger than a bound refills the product in
that frame. The product is valid down to a lowest Sun elevation the profile declares (2° in the first profile).
Below it, and outside the field's region, a receiver gets `exp(−tau_far)`, where `tau_far` is one number a frame:
the mean slant optical depth of the far level. So an overcast sunset stays dark, and a clear one is exactly
unshaded.

### Cloud light

A sample's scattered radiance, integrated over its step with Frostbite's equation 17:

`L = σ · [ Σₙ wₙ · E_sun(point) · exp(−bₙ·tau_sun) · HG(θ, gₙ) + L_ambient ]`

- **Sun.** `E_sun` is the lent Sun light at a world position: the true Sun through the air, with no cloud factor.
  It is evaluated at the layer's base and top on each ray and interpolated by height, so a tall cloud's top
  stays lit after its base has gone into the Earth's shadow, and sunset colour comes from the sky's own
  transmittance.
- **Orders.** Four, with the retained reference's `WEIGHTS`, `BRANCH_B` and `LIQUID_G`. Each order reuses the same
  `tau_sun`, so multiple scattering costs no density read. The forward lobe gives the silver lining.
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

- **Content.** The game cooks its library as one stacked volume product: each shape is a slab of the same size
  along one axis, so every mip stays inside its slab and one texture serves all shapes. A growth track is a
  second stack whose slabs are keys in time. `CONTENT-LOOP` lends both through a lease, unchanged. With each
  shape the game gives its physical extent, base height, downwind axis and mean column sum.
- **Placement.** A share of cells, set by the type table, are hero cells, chosen by hash. Within a placed range
  of the camera a hero cell is drawn from a volume the hash picks: centred on the cell, its base on the shared
  condensation level, turned to the wind by yaw, scaled uniformly. Its extinction scale is set so its mean
  column optical depth equals the cell's, so the CPU query stays true on average.
- **One march.** The weather map texel names the slot. The density function reads the volume there in place of
  that cell's own column, with a weight that ramps across the edge of the placed range; the two forms' weights
  sum to one. Other cells' cloud at that texel adds. The same function feeds the Sun's path and the overlay, so
  a placed cloud is lit, shades the ground and is reflected like any other.
- **Life.** The cell's envelope drives the volume's opacity and edge erosion, so it forms and thins with its
  cell. A growth-track instance blends the two keys around its age.
- **Capacity.** The number of placed instances is a profile limit checked against the capability record. Cells
  beyond it stay procedural, nearest first, and the refusal is counted and reported.

### Shadow-only and headless

- **Headless.** `thrum-cloud-model` answers point queries with no graphics device and allocates nothing.
- **Shadow-only** is a profile: the weather map, the shape noise and the Sun's path are built, and the trace,
  the layer history, the detail noise, the overlay and the far-aerial use are not declared. Its ground field is
  the same bytes as with visible clouds on. Changing between the two rebuilds the effect and starts a new epoch.

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
  fixed allocation, so no knob creates a resource or pipeline. None changes the weather field or what a query
  returns.

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
- **K5.** Unrun here. Broadsides' hybrid stills and the growth track's blind review show the content reads; that
  one density function serves both is first proved in M6.
- **K6.** Unrun. The layer's reconstruction and its look under TAAU are first seen in M3 and M9.

## The system as it runs

| State | Owner |
|---|---|
| The weather field, its state and step, the density function, the f64 references | `thrum-cloud-model` (this module), no GPU dependency |
| The weather map, noise, the Sun's path, the layer and its history, the overlay, placed-volume sampling | `thrum-clouds` (this module), an effect on `EFFECT-GRAPH`'s seam |
| The committed weather state | The game's world; it travels in the tick page |
| Presets, the type table, looks, the library and its placement data, controls | The game (`examples/v1-game` behind its cloud selection; `examples/sea` behind its own); never in engine crates or their tests |
| Sky, Sun, aerial, reflection and diffuse products; applying the cloud factor, the overlay and the haze scale | `ATMOSPHERE` |
| Stages, state commit, cadence, the knob list | `EFFECT-GRAPH` |
| Motion and reactive targets, history acceptance, reconstruction, the governor | `APPLE-PRESENTATION` |
| The residency of volume products | `CONTENT-LOOP` |
| The shell, typed controls, parameter files | `CREATOR` |

**A frame, in order.** `upload` takes the page's weather state. `simulate`: the due part of the weather map; the
due tiles of the Sun's path and the ground field; the due part of the overlay. `ATMOSPHERE` filters its
reflection and diffuse products with the overlay. Sun passes and forward draw, with the cloud factor on every
direct-Sun term. Sky and stars draw. `after-opaque`: the sea, if the scene has one; then trace, resolve and
compose. Reconstruction, tone and present follow. On submission the effect commits its tick, anchors, refresh
phase and the history side.

**The example.** `examples/v1-game` gains a cloud selection, off by default. With it on, the game keeps a weather
state in its world, adds the cloud contribution after any sea, and shows cloud controls. With it off the game,
its world, its page and every frame are what they were. The selection replaces the diagnostic shade provider.

**Creator journey.** A scene adds the cloud contribution with a profile and a type table, steps a weather state
in its world, and puts it in its page. Changing coverage, type, wind or time scale needs no rebuild. Changing the
profile or the library rebuilds the effect and resets its histories.

## What CLOUDS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-clouds). Each is built by this module's builder
in the owning module's files, with that module's items unchanged and its tests kept passing, and each is absent
from a scene with no cloud provider: the same shader text, passes, storage and bytes as today.

1. **A provider may write the Sun-path optical depth on the GPU** (`ATMOSPHERE`, M2). Today's field is CPU bytes
   with a rigid drift. A provider may instead register a texture its own `simulate` pass writes, with its mapping
   updated each frame, and one `tau_far` used below the lowest valid elevation and outside the region. The
   CPU-fed form keeps `tau_far` at zero and behaves exactly as now.
2. **Aerial transport to a far sky point** (`ATMOSPHERE`, M3): the lent operation beyond 32 km, as above.
3. **A sky overlay in the reflection and diffuse products** (`ATMOSPHERE`, M8). If the incident-light meter has a
   Sun term without the cloud factor, it gains it at its metering point.
4. **A haze scale on solar in-scatter** (`ATMOSPHERE`, M8).
5. **A cloud selection in two games** (`SHOWCASE`'s `examples/v1-game` from M2; `SEA`'s `examples/sea` in M11),
   off by default, leaving each game unchanged when off.

Everything else is used as built: effect stages, histories and cadence; the sea's depth write, which the trace
treats as geometry; the temporal contribution; leases of volumes; typed controls.

## Cost account

Estimates, unmeasured, at a 2560×1440 render extent and 120 frames a second. Broadsides' figures are measured
there, not here.

| Work | Rate | Size | Estimate | Broadsides |
|---|---|---|---|---|
| Weather step | Tick, CPU | One record | Microseconds | |
| Weather map | About 2 Hz, in parts | 4 levels × 256², two textures | Under 0.1 ms a frame | 2 ms an update |
| Noise | Preparation | 128³ and 32³ | None in a frame | |
| Sun's path | Near level over 8 frames, mid 32, far 128 | 3 × 256² columns, 48, 32 and 16 slices: 6.3 M samples to fill, about 0.5 M a frame | 0.1–0.2 ms | 2.2 ms shadow |
| Ground field | With the Sun's path | 1024² | Under 0.05 ms | |
| Trace | Frame | 640×360 rays, three quarters sky: 3–7 M reads, up to 16 M with three local light samples | The largest term; M3 measures it | 268 ms; 33 M density evaluations |
| Resolve | Frame | 1280×720 texels | 0.1–0.2 ms | 0.9 ms |
| Compose | Frame | Sky pixels at render extent; a pass that loads scene colour | 0.2–0.4 ms and a tile pass break | |
| Sky overlay | An eighth a frame | 6 × 48² rays, coarse density | Under 0.05 ms | 15 ms capture |
| Memory | | Weather 4 MiB, noise 2, Sun's path 13, ground 2, layer history and trace 25, library 27 MiB as R8 or 14 as BC4, growth track the same again | About 46 MiB plus the library | 62 MiB plus history |

**Tripwires, never gates:** each pass's GPU time by name, bytes per resource, and a paired record with clouds
absent. The owner's figure is the display's full refresh. An automated check fails when the valley's whole frame
exceeds the refresh interval with clouds on where it holds with clouds off, or when a cloud pass grows about 25%
over its recorded baseline. Presented cadence on the built-in panel is measured once, attended, in M12.

## Risks

1. **The trace costs too much.** Early sign: M3's record. The wrong fix is dropping the local light samples or
   the detail noise below what the look needs without saying so. The designed ones, in order: a sparser refresh
   (one texel in sixteen), a coarser far step, fewer local samples with a finer near level. If those do not hold
   the refresh, the design is re-planned before M5.
2. **Procedural cumulus reads as soft shelves**, as Broadsides' did. Early sign: M3's first stills. The mechanism
   (profile thresholding shape noise, flat shared bases, domes) is the shipped one; the look is tuned against
   M3's list with a fresh opinion. Three failed looks on one cue go to the log and end `BLOCKED`.
3. **The reconstructed layer ghosts or crawls.** Early sign: M3's pan. Fallback: refresh every texel every
   frame at a lower layer scale, which removes the layer's history from the picture.
4. **Low-Sun relief is lost in the summed product.** Early sign: M5's low-Sun stills against the oracle.
   Fallback: more local samples at low Sun, or a finer near level.
5. **`ATMOSPHERE`'s products resist the overlay**, because their storage is relative to the Sun. Early sign:
   M8's first test. The ruling states behaviour, not storage; if the producers cannot compose it, the proposal
   goes to that module's log.
6. **Chrome refuses a format, count or binding the Mac accepts.** Early sign: M2 runs under WebGPU's default
   limits on the Mac, and M4 is the first real Chrome run.
7. **The library's licence does not arrive.** Then M10 waits, and with it M11 and M12. Everything a visitor sees
   except the accepted shapes is finished by M9.
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
- **Broadsides.** Read it at the brief's commits; never change it. **Nothing is copied or translated from it,
  code or content, until the owner has named the licence** ([plan](PLAN.md#open-for-the-human)). Until then
  write from the published sources and this design. Once it is named, a lifted file goes through review, its
  tests are re-derived here, and `docs/evidence/CLOUDS/PROVENANCE.md` records its source commit, path and
  licence.
- **Game content stays in the game.** Presets, type tables, looks and volumes never enter `thrum-cloud-model`,
  `thrum-clouds` or their tests; tests make their own inputs.
- **Dependencies.** New crates and any third-party dependency are registered in `.config/architecture.toml`.
- **Maturity.** The ledger's `E1.clouds` row is kept current from M2, with implementation, correctness, visuals
  and device apart.

## Open questions

| Question | Settled by |
|---|---|
| Does the trace hold the refresh at 2560×1440, and at which refresh fraction and local sample count? | M3 measures; M12 confirms at the showcase's extent on both recipes |
| What tolerance does the GPU weather map meet against the CPU field on Metal and in Chrome? | M2 and M4 measure first, then assert the recorded bound |
| How many sampled textures does the lent environment group use in the trace? | M3 counts against the record |
| Does the summed Sun-path product carry low-Sun relief with three local samples? | M5 |
| Does the incident-light meter already see cloud shade? | M8 reads the code and measures |
| Can the scene's residency take the library as a second source folder? | M6; if not, the proposal goes to `CONTENT-LOOP`'s log |
| Is an Apple-tile kernel variant a measured win? | M12 |
