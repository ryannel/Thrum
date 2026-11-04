# Sea and cloud providers: port profile from the Broadsides Rust work

4 November 2026. **Selected design direction. Implementation follows G7.** Broadsides is building a Rust sea and Rust volumetric clouds that are intended to move into Thrum. This record fixes how that work maps onto Thrum's contracts and the [hardware-native decision](HARDWARE-NATIVE-ENGINE.md), so both sides prepare for a cheap, reviewed port.

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

* **Authority.** The E1 cloud source is a deterministic weather field: a function of seed, preset, integrated wind travel and `t_sim`. Its outputs are coverage, genus, top height, age and column optical depth `τ`. A CPU evaluator answers point queries headless. A test asserts the GPU weather map matches it within tolerance. Wind travel is integrated per tick, never computed as mean wind × absolute time. Rain and precipitation derive from this field.
* **Light.** One cloud transmittance product with `τ` semantics feeds V3 once, as `exp(-τ)`. A shadow-only mode runs without the view march, for the hidden-clouds scenario.
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

## Current state and targets

| | Measured (M3 Max, Metal) | Broadsides target |
|---|---|---|
| Sea | 22.0/22.8 ms whole-frame GPU (median/p95, 1440p); turning the water surface off saves 5.5 ms and the reflection trace 1.1 ms; other water passes are within run-to-run drift | Surface ≤1.5 ms; waves, interaction and foam ≤0.8 ms |
| Clouds | About 20 ms and 365 MiB at 1080p | ≤2 ms and ≤64 MB at 1440p |

Neither system has been measured on an iPhone or a non-Apple GPU, and neither has visual acceptance. Thrum qualifies these per recipe.

## Porting policy

Port module by module after G7; design work may start earlier.
* **Lift:** the `bw_water` maths and the WESL algorithms (FFT, LEAN, eWave, foam, density, lighting, phase), each through review. Re-derive the golden tests in Thrum. Record the source commit, path and licence for each lifted file.
* **Rewrite:** the `RenderModule`, `pass_list` and `TargetPool` plumbing becomes the Thrum executor. The `bw_frame` packets become the tick page and `ViewPacket`. The Bevy host glue is rewritten.
* **Never:** a wholesale import, or a Broadsides build dependency.
