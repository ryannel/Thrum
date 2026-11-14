# Sea and clouds: Thrum's designs decide, and Broadsides is a resource

6 November 2026, revised 14 November 2026. **Selected direction.** Broadsides built a Rust sea and Rust volumetric
clouds. This record was written as the profile for porting them. On 14 November the owner changed its standing:

> We can fully reuse anything from broadsides, but it's there as a resource where it's useful, it shouldn't dictate
> design.

> The agent should look at that work from first principles and figure out how to design the best possible
> atmospheric system for the thrum game engine. Maybe that's a flexible set of cloud capabilities built from what we
> did in broadsides, maybe it isn't, but it should start from first principles.

So each module's design starts from what Thrum needs, chooses its mechanisms against that, and then takes from
Broadsides whatever serves them. What this record says of how Broadsides did something is evidence for those
designs and never a constraint on them. It still fixes what both modules owe Thrum's contracts and the
[hardware-native decision](HARDWARE-NATIVE-ENGINE.md), below.

- **The sea** was judged this way on 14 November, and later that day water was designed as one system for every
  body of water. Its methods are [WATERS' design](../../plans/WATERS/DESIGN.md), with the wave field in
  [SEA's](../../plans/SEA/DESIGN.md). The section below summarises the sea as it stood that morning; where it
  differs from `WATERS`' design, that design holds.
- **The clouds** were judged this way on 14 November. Their methods are
  [CLOUDS' design](../../plans/CLOUDS/DESIGN.md), and the section below summarises them.

Broadsides is read at the commits the module designs name. It is not a build dependency and is never modified. Its
earlier `THRUM-ALIGNMENT` mapping of the TypeScript lake lab is superseded.

## The sea

This record adds a named `E1.water` profile. It sits beside the [prescribed sea](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river),
and does not replace it or [WATER.md](../modules/WATER.md)'s inland designs.

The [water decision](E1-GPU-WATER.md) owns where water runs, body forces, replay and multiplayer sharing. In short:
all water simulation runs on the GPU, the network carries causes rather than water state, and water needs no exact
replay. [WATERS' design](../../plans/WATERS/DESIGN.md) owns the methods, and [SEA's](../../plans/SEA/DESIGN.md) the wave
field's, with [the verdict on each choice](../../plans/WATERS/DESIGN.md#judged-from-first-principles) this record
used to make:

* **Authority.** The GPU sea is the authority: analytic long waves (at most 192 components, wavelength at least
  20 m) plus FFT short-wave cascades with LEAN slope moments. Both are functions of time, **evaluated every
  presented frame at the display's time** and holding nothing between frames; this replaces "evaluated once per
  tick". Long-wave phase is computed in f64 and wrapped before upload, and cascade phase is a whole number of turns
  in a declared loop. A CPU f64 evaluator of the same long-wave coefficients is the test oracle and part of the
  no-GPU conformance model; the GPU must match it within the declared tolerance. Bodies read the GPU through one
  batched query with asynchronous readback, one to two ticks late, with latency compensation and a declared error.
  Synchronous readback is forbidden.
* **Interaction.** The wake is physical water state, not a residual. A two-level dispersive (eWave) solver, about
  0.25 m near and 2 m far, each level owning a band of wavenumbers, takes each body's reaction as a pressure source
  once per tick; bodies feel each other's wakes through the same query. The wake steps on the committed tick and is
  drawn as a blend of its last two ticks. Far-body Kelvin packets are not built. The game integrates its bodies
  until Thrum has a physics owner. Emitters and sources carry stable IDs with generations; capacity overflow is a
  counted, reported refusal rather than a log line. The game supplies hull data.
* **Whitewater.** Fresh and aged foam, submerged air and slick are four transported stocks in one field, born only
  from booked causes as bounded rates. Persistent GPU state advances from tick-derived time, not frame `dt`, and
  resets on rebuild or profile change.
* **Capabilities.** The query, the wake, whitewater and body optics are each left out by a scene at no cost. The
  open sea alone is a complete sea.
* **Inland water.** Lakes, ponds and shores are `WATERS`'; rivers, waterfalls and puddles are `RIVERS`' and
  `PUDDLES`', on the same system ([module list](../../plans/ARCHITECTURE.md#water-as-one-system)). A river's flow is
  prepared once and not simulated live, which sets WATER-DYNAMICS' generalised-heightfield selection aside.
* **Optics.** Optics follow C08: water supplies ordered medium segments `(T,S)` and a reflection provider. The sea
  reflects and refracts the bodies a scene registers, from a gather of their own screen rectangles, with the
  environment product everywhere else and no whole-scene copy on any recipe. Ray query on desktop high is a later
  provider that emits hit distance.

## The clouds

This record selects a weather-driven volumetric sky as the visible `E1.clouds` provider. ENVIRONMENT.md's projected optical-depth field stays as the meaning of cloud shade, and is what a scene with shade alone builds.

[CLOUDS' design](../../plans/CLOUDS/DESIGN.md) owns the methods, with
[the verdict on each choice](../../plans/CLOUDS/DESIGN.md#judged-from-first-principles) this record used to make:

* **One medium, not two paths.** Broadsides built a procedural sky and a baked library and meant to offer each, and a hybrid, as sky types. Thrum has one density in m⁻¹ that every consumer reads and one light. Convective cells, a layered deck and shapes cooked from density volumes are sources of that one field, and which a sky has is the game's data: weather alone, shapes alone or both run the same code. G5 cooks the volumes as content-addressed products: KTX2 BC4 on desktop, R8 or EAC slices on iPhone, and R8 for the portable recipe.
* **Authority.** The E1 cloud source is a deterministic weather field: a function of seed, parameters, integrated wind travel and weather time, with any shapes the game directs. Its outputs are coverage, type, base, top, age and column optical depth `τ`. A CPU evaluator answers point queries headless. A test asserts the GPU weather map matches it within a measured tolerance. Wind travel is integrated per tick, never computed as mean wind × absolute time, and is shown at the display's time each frame. Rain and precipitation would derive from this field.
* **Light.** Thrum G3s is the sole sky/Sun/atmosphere source for both clouds and water; no Broadsides sky is ported alongside it. One Sun-path optical-depth product with `τ` semantics feeds V3 once, as `exp(-τ)`, and the same product is the clouds' own self-shadow. The first scattering order takes its phase from the droplets (Henyey–Greenstein plus Draine); later orders are octaves that reuse the same `τ`. Clouds reach the world's ambient light and reflections through `ATMOSPHERE`'s products, never by a capture of their own. Water reflection and cloud illumination consume the same environment identity.
* **Presentation.** The visible layer is traced at part of the render extent and reconstructed over frames into a layer at render extent. Aerial perspective is applied once. Cloud and froxel-fog `(T,S)` intervals must be ordered and disjoint.
* **Capabilities.** The weather query, shade, the visible sky, the world's light from clouds, and shapes are each left out by a scene at no cost. Shade alone omits the view march. A profile places shapes by the field's hash or by the game.

## Ownership

The engine owns:
* the spectral evaluator, cascades, and wake and foam machinery;
* the weather field, march, Sun-path product, sky overlay and history machinery.

The game owns:
* sea presets, hull and body data, and whitewater tables;
* cloud type tables with their droplet optics, presets, the shape library and the look.

None of the game-owned content appears in engine crates or tests.

## Hardware and frame-contract obligations

* **Limits are inputs, not constants.** The capability registry supplies these:
  * FFT size and workgroup memory;
  * f16 and subgroup support;
  * texture and binding caps;
  * cascade count and emitter cap.

  The WebGPU defaults (256-point f32 FFT, 16 KiB workgroup memory, 16 sampled textures) become the portable recipe, not the design.
* **Recipes.**
  * **Apple tile:** f16 arithmetic and tile-friendly passes. Water copies the scene only where it is thin enough to see through or still enough to mirror, and an open sea copies nothing ([WATERS](../../plans/WATERS/DESIGN.md#optics)).
  * **Desktop high:** async-compute overlap for cascades, wake, foam, weather and shadow work, plus the later ray-query provider.
  * **Every recipe:** compute replaces fragment marching where measured faster.
* **Governor knobs.** Every cadence and step budget is a runtime setting that the quality governor owns: weather update rate, march refresh fraction, Sun-path and sky-overlay cadence, cascade and foam rate, and resolution. Each knob declares its history invalidation, and none changes physical profiles.
* **Frame contract.**
  * Motion vectors cover short waves and wake, evaluated at the previous presentation time.
  * Foam, glints and clouds write the reactive mask.
  * Cuts, sea rebuilds, weather seeks, cloud profile and library changes, and pass toggles set reset flags.
  * Cloud and water effects output at render resolution. V4's reconstructor upscales; there is no private upscaler.
* **Effect history.** History keys include view, resolution and epoch. History uses ping-pong rather than per-frame full copies, and world-anchored wrapping where the effect is spatial.
* **Evidence.** Restore per-pass GPU timestamps, and measure on iPhone and Mac for each recipe against portable.

## G3 consumers

The G3 graph design must support:
* compute passes before dependent fragment passes;
* storage-texture arrays with mips, plus 3D and persistent noise textures;
* ping-pong and wrapping histories;
* time-sliced and epoch-skipped passes;
* an async-compute-eligible pass declaration;
* WESL-style shader modules, or their equivalent in the shading-library decision.

## Readiness, 6 November

Thrum is **not yet ready for the port**. Executing Render/Compute/Copy closures, dependency order, 2D-array textures,
ping-pong history, automatic timestamps and immutable device capabilities exist. These seams still need consumers:
* G3s: finish and qualify the one shared sky, Sun, light-transmittance and environment-reflection products.
* Graph: implement 3D/cube resources and cadence; async-compute eligibility remains a design seam, not overlap proof.
* Frame/V4: effects must supply their own motion/reactivity and reset epochs; reconstruction remains G4 work.
* G5: cook/map the named mesh/texture/3D-volume products into the small runtime; cloud volumes are not loaded yet.
* Recipes/governor: resource/quality selection and history-safe cadence still need G3t/G4 integration and measurements.

Historical Broadsides Mac observations were about 22 ms whole-frame sea at 1440p and 20 ms / 365 MiB clouds at 1080p.
They qualify no Thrum recipe, physical iPhone result or visual acceptance. Re-measure each reviewed module here.

## Planning, 10 November

The owner asked for both to be finished and consolidated. [The module list](../../plans/ARCHITECTURE.md) plans them
as two Thrum modules, `SEA` and `CLOUDS`, finished here and not in Broadsides, and names the milestone that meets
each readiness condition above ([table](../../plans/ARCHITECTURE.md#where-the-ports-readiness-conditions-are-met)).
Their briefs are [SEA-BRIEF](../../plans/SEA/BRIEF.md) and [CLOUDS-BRIEF](../../plans/CLOUDS/BRIEF.md). When builders
start them is still the owner's decision, and the reference commits in those briefs supersede the two above.

## First principles, 14 November

The owner's words at the top of this record. `SEA` was judged on them the same day
([log](../../plans/SEA/LOG.md)): the layering, the query, the wake, the foam stocks and the optics are kept for
reasons its design gives, and two things changed. The ambient sea is evaluated at the display's time each frame in
place of tick copies blended in the surface pass, and every part beyond the open sea is a capability a scene can
leave out. Later the same day the owner asked that the sea work think about water as a whole, and `SEA` was
narrowed to the wave field with the rest designed for every body of water in `WATERS`
([log](../../plans/WATERS/LOG.md)).

`CLOUDS` was judged on them the same day ([log](../../plans/CLOUDS/LOG.md)). Kept, for reasons its design gives:
the deterministic weather field with its CPU query, the air frame, 2D fields with a vertical profile, one Sun-path
product for shade and self-shadow, a placed volume standing for a cell of the field, light and shade through
`ATMOSPHERE` alone, and a layer that ends at render extent. Changed: the two Broadsides paths and their hybrid
are one medium whose sources are data; the first scattering order takes the droplets' own phase; drift is shown
at the display's time; a game can place shapes itself; and every part beyond the weather is a capability a scene
can leave out. The accepted library of twelve volumes and its growth track is the example's content.

## Reuse policy

Broadsides code, shaders and assets may be copied into Thrum and used as they are, wherever they implement a
mechanism the module's design has chosen. The design decides; a file that exists is never a reason to choose a
mechanism. There is no build dependency on Broadsides and it is never modified.

* **Copy as is, where it fits:** maths, kernels, shaders and assets that do what the design states. Each module's
  design names where to look. Record the source commit and path of every copied or translated file in the module's
  provenance file.
* **Licence.** The owner's words of 14 November cover what Broadsides' authors wrote. Material that came into
  Broadsides from elsewhere keeps its own licence, recorded with it, and nothing GPL or share-alike is copied.
* **Hold to Thrum's proof.** Copied code meets the module's oracles and done-when items like any other. A verdict
  passed in Broadsides, under its sky, exposure and reconstruction, is not a verdict here.
* **Write for Thrum:** scheduling, allocation, pass lists, target pools, frame packets and host glue. Thrum has one
  owner for each: the executor, the tick page and `ViewPacket`, the one sky, the frame contract, the capability
  registry and cooked products.
