# Sea and cloud providers: port profile from the Broadsides Rust work

4 November 2026. **Selected design direction. Implementation follows G7.** Broadsides is building a Rust sea and Rust volumetric clouds that are intended to move into Thrum. This record fixes how that work maps onto Thrum's contracts and the [hardware-native decision](HARDWARE-NATIVE-ENGINE.md), so both sides prepare for a cheap, reviewed port.

The reference revisions are committed Broadsides commits `70cf98122` (`codex/water-rust-system`) and `8b6262f13` (`codex/clouds-procedural-publish-20261104`). Broadsides is not a build dependency. Its earlier `THRUM-ALIGNMENT` mapping of the TypeScript lake lab is superseded.

## Spectral sea profile

This record adds a named `E1.water` profile. It sits beside the [prescribed sea](../PLAY-ENVIRONMENT-SPEC.md#e1-analytic-sea-and-shallow-river), and does not replace it or [WATER.md](../modules/WATER.md)'s inland designs.

* **Authority.** A CPU f64 analytic long-wave sum is the authority:
  * at most 192 components, with wavelength of at least 20 m;
  * height, horizontal (choppy) displacement, orbital velocity and dynamic pressure;
  * evaluated at committed tick time `t_sim`.
  
  A displaced surface point is `x₀ + D(x₀, t)`. Height and contact queries solve for `x₀` with a declared iteration bound and error. The GPU evaluates the same coefficient set for drawing, and the two must agree within the declared tolerance under test. Gameplay, buoyancy and P3 forces use CPU queries only. GPU readback never enters the force path.
* **Interaction.** Hull loads use strip theory and hydrostatics. Wake crossing uses CPU Kelvin kernels on ticks. Emitters and sources carry stable IDs with generations; capacity overflow is a counted, reported refusal rather than a log line. Hull strip theory and wake crossing form an optional vessel package. The game supplies hull data.
* **Presentation residual.** GPU work below the 20 m cut-off is a bounded presentation residual that gameplay never reads. It covers FFT short-wave cascades with LEAN slope moments, the eWave wake grid, foam and material transport, and whitewater. Its height and slope error bounds against the authority are declared. Cascades are periodic in time, so they are evaluated from presentation time. Persistent GPU state (wake, foam) advances from tick-derived time, not frame `dt`. It resets on correction, rebuild or profile change.
* **Inland water.** It stays on WATER.md's CPU shallow-water and generalised-heightfield authority. A GPU finite-volume river is not adopted as authority.
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
| Sea | 34.4/37.8 ms whole frame (median/p95, 1440p); water's own cost is unmeasured | Surface ≤1.5 ms; waves, interaction and foam ≤0.8 ms |
| Clouds | About 20 ms and 365 MiB at 1080p | ≤2 ms and ≤64 MB at 1440p |

Neither system has been measured on an iPhone or a non-Apple GPU, and neither has visual acceptance. Thrum qualifies these per recipe.

## Porting policy

Port module by module after G7; design work may start earlier.
* **Lift:** the `bw_water` maths and the WESL algorithms (FFT, LEAN, eWave, foam, density, lighting, phase), each through review. Re-derive the golden tests in Thrum. Record the source commit, path and licence for each lifted file.
* **Rewrite:** the `RenderModule`, `pass_list` and `TargetPool` plumbing becomes the Thrum executor. The `bw_frame` packets become the tick page and `ViewPacket`. The Bevy host glue is rewritten.
* **Never:** a wholesale import, or a Broadsides build dependency.
