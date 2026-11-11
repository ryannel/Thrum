# Sea and cloud providers: port profile from the Broadsides Rust work

6 November 2026. **Selected direction; the owner decides port timing when Thrum is ready.** Broadsides is building a Rust sea and Rust volumetric clouds that are intended to move into Thrum. This record fixes how that work maps onto Thrum's contracts and the [hardware-native decision](HARDWARE-NATIVE-ENGINE.md), so both sides prepare for a cheap, reviewed port.

The reference revisions are committed Broadsides commits `70cf98122` (`codex/water-rust-system`) and `8b6262f13` (`codex/clouds-procedural-publish-20261104`). Broadsides is not a build dependency. Its earlier `THRUM-ALIGNMENT` mapping of the TypeScript lake lab is superseded.

## Spectral sea profile

This record adds a named `E1.water` profile. It sits beside the [prescribed sea](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river), and does not replace it or [WATER.md](../modules/WATER.md)'s inland designs.

The [water decision](E1-GPU-WATER.md) owns where water runs, body forces, replay and multiplayer sharing. In short: all water simulation runs on the GPU, the network carries causes rather than water state, and water needs no exact replay.

* **Authority.** The GPU sea is the authority: analytic long waves (at most 192 components, wavelength at least 20 m) plus FFT short-wave cascades with LEAN slope moments, evaluated once per tick into a camera-following field clipmap. Wave phase is computed in f64 and wrapped before upload. A CPU f64 evaluator of the same long-wave coefficients is the test oracle and part of the no-GPU conformance model; the GPU must match it within the declared tolerance. Bodies read the full GPU state through one batched query with asynchronous readback, one to two ticks late, with latency compensation and a declared error. Synchronous readback is forbidden.
* **Interaction.** The wake is physical water state, not a residual. A two-level dispersive (eWave) solver, about 0.25 m near and 2 m far, takes hull pressure sources once per tick; bodies feel each other's wakes through the same query, and far bodies use GPU Kelvin packets. Hull loads still use strip theory and hydrostatics, integrated by P3 on the CPU. Emitters and sources carry stable IDs with generations; capacity overflow is a counted, reported refusal rather than a log line. The game supplies hull data.
* **Whitewater.** Fresh and aged foam, aeration and slick form one transported field, fed by a deposition pass of wake and wave breaking rates, bow, stern-ribbon, cusp, wash and impact sources. Persistent GPU state advances from tick-derived time, not frame `dt`, and resets on rebuild or profile change.
* **Inland water.** Rivers, lakes and pools run WATER-DYNAMICS' selected shallow-water and generalised-heightfield methods as GPU compute, and carry their own wakes.
* **Optics.** Optics follow C08: water supplies ordered medium segments `(T,S)` and a reflection provider. Screen-space reflection is first. Ray query on desktop high is a later provider that emits hit distance.

## Volumetric cloud profile

This record selects weather-map volumetric clouds as the visible `E1.clouds` provider. ENVIRONMENT.md's projected optical-depth field stays as the portable and shadow-only representation.

Cloud presentation combines procedural weather layers with placed voxel volumes under the same E1 provider and light-transmittance contract. G5 cooks baked 3D volumes as content-addressed products, including cloud density: KTX2 BC4 on desktop, R8 or EAC slices on iPhone, and R8 for the portable recipe. Placement, product identity and history invalidation are explicit inputs; the weather field remains the source for headless queries and precipitation.

* **Authority.** The E1 cloud source is a deterministic weather field: a function of seed, preset, integrated wind travel and `t_sim`. Its outputs are coverage, genus, top height, age and column optical depth `τ`. A CPU evaluator answers point queries headless. A test asserts the GPU weather map matches it within tolerance. Wind travel is integrated per tick, never computed as mean wind × absolute time. Rain and precipitation derive from this field.
* **Light.** Thrum G3s is the sole sky/Sun/atmosphere source for both clouds and water; no Broadsides sky is ported alongside it. One cloud transmittance product with `τ` semantics feeds V3 once, as `exp(-τ)`. Shadow-only mode omits the view march. Water reflection and cloud illumination consume the same environment identity.
* **Presentation.** The march, Beer shadow map, multi-scattering octaves and sky capture are presentation. Aerial perspective is applied once. Cloud and froxel-fog `(T,S)` intervals must be ordered and disjoint.

## Ownership

The engine owns:
* the spectral evaluator, cascades, and wake and foam machinery;
* the weather operator, march, shadow, capture and history machinery.

The game owns:
* sea presets, hull and body data, and whitewater tables;
* cloud genus tables, lighting presets and the look.

None of the game-owned content appears in engine crates or tests.

## Hardware and frame-contract obligations

* **Limits are inputs, not constants.** The capability registry supplies these:
  * FFT size and workgroup memory;
  * f16 and subgroup support;
  * texture and binding caps;
  * cascade count and emitter cap.

  The WebGPU defaults (256-point f32 FFT, 16 KiB workgroup memory, 16 sampled textures) become the portable recipe, not the design.
* **Recipes.**
  * **Apple tile:** f16 arithmetic and tile-friendly passes. The pre-water full scene copy is replaced by a recipe-specific refraction source.
  * **Desktop high:** async-compute overlap for cascades, wake, foam, weather and shadow work, plus the later ray-query provider.
  * **Every recipe:** compute replaces fragment marching where measured faster.
* **Governor knobs.** Every cadence and step budget is a runtime setting that the quality governor owns: weather update rate, march refresh fraction, shadow-cascade spread, capture interval, cascade and foam rate, and resolution. Each knob declares its history invalidation, and none changes physical profiles.
* **Frame contract.**
  * Motion vectors cover short waves and wake, evaluated at the previous presentation time.
  * Foam, glints and clouds write the reactive mask.
  * Cuts, sea rebuilds, weather generations and pass toggles set reset flags.
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

## Porting policy

Port module by module only when the owner schedules it. Keep the foundation converging on the readiness above; no automatic import at G7 and no Broadsides build dependency.
* **Lift:** the `bw_water` maths and the WESL algorithms (FFT, LEAN, eWave, foam, density, lighting, phase), each through review. Re-derive the golden tests in Thrum. Record the source commit, path and licence for each lifted file.
* **Rewrite:** the `RenderModule`, `pass_list` and `TargetPool` plumbing becomes the Thrum executor. The `bw_frame` packets become the tick page and `ViewPacket`. The Bevy host glue is rewritten.
* **Never:** a wholesale import, or a Broadsides build dependency.
