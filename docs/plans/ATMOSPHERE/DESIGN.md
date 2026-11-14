# Atmosphere for the V1 outdoor scene

[Rendering contract](../../design/RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator) ·
[Environment source](../../design/modules/ENVIRONMENT.md) · [Milestones](PLAN.md) · [Done when](DONE.json) ·
[Module list](../ARCHITECTURE.md)

**The owner's request** for the work added on 10 November (M5 and M6), word for word from that day's planning
task: "The owner wants work lined up continuously toward the V1 exit in docs/BUILD-PLAN.md, so builders always
have a next milestone, and wants water and clouds finished and consolidated ... Decide, with reasons ... where in
the build order the port's readiness conditions are met." The exit clause this module closes is "physical sky, Sun
and aerial perspective with time controls". The readiness condition it meets, from
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november), is "G3s: finish and qualify the one
shared sky, Sun, light-transmittance and environment-reflection products."

[The view volume](#the-view-volume-haze-fog-and-light-near-the-camera), at the end, governs M7 and M8, added on
14 November for haze, fog and shafts of light; it is track 2 and follows G3s.

The sections below through [Frame and cost](#frame-and-cost) are the design M1–M4 were built from and are
unchanged. [Environment identity and products](#environment-identity-and-products) governs M5 and
[Closing G3s](#closing-g3s) governs M6. Both were revised on 10 November after
[the first design review](../../evidence/ATMOSPHERE/design-review-1.md): the numerical acceptance is written out
product by product, environment reflection is a lent and tested operation, and the limits of each lent lookup
and of the cloud field are stated. They were revised again on 11 November after
[the second review](../../evidence/ATMOSPHERE/design-review.md): the sky and aerial handoff rows H2 and H4 now
compare against an independently evaluated filtered decode, with wrong lookups that must fail, and the lent
values are stated to be free of pre-exposure. A third revision the same day follows M5's builder measuring that
Metal returns a filtered half-float read at the stored format's precision
([entry 3](../../evidence/ATMOSPHERE/log-entries.md#entry-3)): the filtered-decode bound gains one stored step,
the known field's steps are sized against it, and each lookup stays one hardware-filtered read.

The V1 valley uses one Earth sky from noon through astronomical night. The shipped
Hillaire transport owns sunlight, the sky-view and aerial LUTs, and the shared diffuse
and reflection products. The night extension covers a clear, moonless rural sky and
an exposed foreground over 0–15 km observer altitude. The scene stays recognizably
dark while the ridge separates from the sky and the same light reaches diffuse rock
and specular metal. In a sweep from Sun −6° through −18°, the last solar colour
recedes continuously. Stars emerge against the darker background, move with the
celestial frame and disappear behind terrain. Earth light pollution, weather-driven
airglow variations and calibrated night colour are outside this profile.

## Light and storage

- Use the observed dark-site zenith scale of about 0.00017 cd/m². Approximate unresolved
  background as a neutral diffuse component and airglow as an axisymmetric emitting
  uniform spherical shell from 85 to 95 km. The benchmark split at ground zenith is 40% diffuse and 60% shell.
  Divide the positive ray length inside the shell by its 10 km thickness. The
  finite thickness removes the thin-shell tangent singularity.
  Use direct plus first scattering in the engine's spherical Earth medium at
  550 nm, with the existing Rayleigh and Henyey–Greenstein phase functions. For each altitude and outgoing cosine, integrate visible incoming
  shell and isotropic boundary light against the Rayleigh and aerosol phase
  functions. Prepare a small RG16Float profile (start at 64 altitude × 96 angular
  nodes) with separate gains for terminal luminance and local in-scatter per
  metre. Use the split tangent chart for terminal luminance and the full cosine
  domain for local source. Normalize both incident components at ground zenith
  before mixing. At 1.8 km the thin-shell scalar reference peaks near +10°. At that altitude's curved horizon, first
  scatter is more than 99% of the sky-side total. The absolute photometry is
  an Earth-like reference profile. Multiple night scattering and spectrum
  remain unqualified. [Noll et al. §4.1](https://arxiv.org/pdf/1205.2003)
  and [Crumey §1.3](https://arxiv.org/pdf/1405.4209) give the shape and observed scale.
- During sky-view compute store the absolute night luminance in the available alpha
  channel with independent fp16 gain. Keep the Sun-relative RGB convention.
  Sky-view compute samples the prepared terminal luminance once per texel.
  The night sky side receives shell emission. The solar RGB path retains its
  100 km endpoint. The night ground side ends at
  a 10% Lambert boundary lit by the profile's prepared ground horizontal
  integral, independent of the downstream camera meter. Finite aerial
  segments add cumulative first-scatter using the same local source, initially
  one midpoint per depth interval. Store it in the unused transmission alpha.
  Surface RGB extinction stays unchanged. The shared aerial colour decoder
  adds night in-scatter once after the solar shadow split, for synthetic, cooked
  mesh and cooked volume consumers. Qualify the local source to the shell;
  any high-altitude cutoff needs a bound for grazing paths. Outside the declared
  0–15 km observer range, report a Sun-only fallback for the night provider. Integrate RGB and
  alpha with the same weights in the existing diffuse and cubemap consumers. Decode
  them once into scene-linear Rec.709 before lighting and metering. The shell
  emits beyond the valley. The in-scatter uses the distributed night background,
  without sampling the direct Sun shadow.
- V4 measures incident lux with the existing GPU resolve, including the night term.
  Apply the owner-selected key to actual measured lux before clamping the resulting
  EV, with a V1 interval of −12 to +18 EV100. Apply a positive numerical guard
  for the logarithm and report the true lower clamp. Use the current exposure
  sample to raise fp16 storage continuously in dim scenes. The initial rule is
  `P = clamp(camera_gain / 4, P_day, 64)`. Fixed and invalid meter paths retain
  the prepared scale. Select the submitted exposure side once during encoding
  and use the same resource for resolve, sky, shared forward view, cooked mesh
  and volume, and display. Resolve precedes those draws in the graph. Adaptive
  atmosphere frames clear to black. Update timestamp interpretation to accept
  the new execution order. Lighting LUTs and depth stay in their own units.
  Temporal consumers retain exposure with the actual accepted image, per C07.
- Stars use compact immutable direction and V-band flux records. Multiply top-of-atmosphere
  flux by directional atmosphere transmission once. An instanced additive draw
  reads stored opaque depth before display mapping on the render grid. Test the centre against
  the planet and opaque depth and reject covered fragment samples. A normalized
  Gaussian with initial angular σ = 0.10° and a 0.7-pixel minimum spreads the
  flux over render pixels. Integrate the normalized footprint over each pixel and
  divide by its solid angle to recover scene radiance, then multiply by the
  current P before blending. Use the same jittered projection as opaque depth.
  Verify the displayed magnitude hierarchy after tone mapping as well as raw flux.
  The scene's +X east, +Y up, −Z north and −45° latitude define the pole.
  Require a proper equatorial-to-world rotation. The north celestial pole lies
  below the northern horizon and fixed stars move west as hour angle increases.
  Zero hour angle uses the Sun's noon meridian with an authored right ascension
  at transit. One sidereal turn per artificial game day deliberately repeats the
  demo; the same interpolated phase and edit drive both outputs. Keep star
  orientation outside atmospheric LUT keys. Reproject with retained view and
  celestial phase or mark star history reactive on cuts and phase edits.
  Admit catalogue data only with confirmed public-domain or CC0 rights and
  URL, licence basis, date and SHA-256. The USNO navigational stars can prove
  placement before a wider catalogue is verified. This first star composite is
  scoped to the opaque valley, whose config has no transparent volume. Volume
  transmittance in front of a star belongs to the later ordered compositor.
- Moon remains a separate named directional emitter. When commissioned it adds
  phase-dependent direct light and scattering through the same atmosphere products,
  with separate visibility. It does not replace the Sun's direction, the nightglow
  source or the meter. Exact lunar geometry, mesopic appearance and a measured
  Earth night spectrum remain open. [Jones et al. §3](https://arxiv.org/pdf/1310.7030)
  and [USGS ROLO](https://www.usgs.gov/publications/spectral-irradiance-moon) inform that extension.

### M3 surface Sun visibility

M3 uses the established combination of stable cascades, overlapping
handoffs and directional percentage-closer soft shadows (PCSS).
Guerrilla shipped stable cascades, and Godot and Unreal expose split
blending and a far fade. Thrum already snaps the cascades and blends
the supported overlap. The archived row-665 receivers stay inside the
last map in both frames, so changing split width cannot fix their
flash. Keep the projection and handoffs fixed for this slice.
[Guerrilla stable cascades](https://www.guerrilla-games.com/media/News/Files/GDC09_Valient_Rendering_Technology_Of_Killzone_2_Extended_Presenter_Notes.pdf) ·
[Godot split controls](https://docs.godotengine.org/en/stable/classes/class_directionallight3d.html) ·
[Unreal directional transitions](https://dev.epicgames.com/documentation/en-us/unreal-engine/directional-lights-in-unreal-engine).

The stored field retains a 2.49–3.65% limb after stock reaches zero.
The archived receivers hold blocker count and filter radius across
the abrupt step. The matched 32-tap screen keeps the eight blocker
probes, physical Sun radius, receiver-plane comparison and small
radius exits. Rotation applies only to the unrestricted last call.
It substantially reduces the coherent flash, but produces visible
penumbra crosshatching on Metal and Chrome. Keep the stock shader.
[Unity directional PCSS](https://raw.githubusercontent.com/Unity-Technologies/Graphics/master/Packages/com.unity.render-pipelines.core/ShaderLibrary/Shadow/PCSS.hlsl) ·
[Matched screen evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint).

The next design under review filters the scalar Sun visibility in
the current frame before opaque lighting. The production precedents
are URP's screen-space shadow mask and FidelityFX's spatial,
receiver-guided denoising. Keep the raw receiver arithmetic and
test a small spatial footprint against the original penumbra and
geometry boundaries. The existing Occlusion prepass omits cooked
geometry and cannot supply the receiver winners. G4 history stays
downstream. Surface motion does not describe the moving cast edge.
New graph work needs a costed proof and a review with G3t and G5.
[Unity URP screen shadows](https://docs.unity.com/en-us/engine/7000.0/manual/lighting-overview/lighting/shadows-in-urp/renderer-feature-screen-space-shadows) ·
[FidelityFX shadow denoiser](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/denoiser/).

Park 16-ray SMRT and contour-area fallback. The first is too
coarsely quantized for the saved limb, and the second switches
243 receivers at once on the broader trajectory. A single
unresolved 32² region can cover about 12% of the disk. Regional
accounting by itself does not remove that risk. Filament's
filterable moments are a production option but lose the
receiver-relative depth correlation and can leak at overlap.
The measured failure is in the current filter and does not yet
establish the need for a novel coverage model.
[Saved-field evidence](../../evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint) ·
[Filterable shadow-map limits](https://developer.nvidia.com/gpugems/gpugems3/part-ii-light-and-shadows/chapter-8-summed-area-variance-shadow-maps).

M4 retains the independent finite-disk reference. Clip terrain and
detached spans to the same solar chord and union them before integrating
solid angle. The projected circle-area approximation has a normalized
solid-angle error below 0.0033 percentage points for the actual Sun.
Original-triangle rays remain the referee. Frozen original-edge packets
match the finite samples, while three conservative omission proofs
leave moving-receiver admission unresolved. Keep those witnesses and
a predeclared work limit for the bounded follow-up. Exact matching does not
gate the M3 visual outcome.

## Frame and cost

Environment supplies the existing timed Sun and the authored observer transform.
Atmosphere owns geometry, night profile and the one disk-free radiance product.
V4 owns exposure; scene shading reads the resolved scale and display decodes it once.
The 192×108 sky and existing reflection textures stay their current size.
The 24 KiB candidate profile is prepared once per supported air profile.
The sky producer adds one filtered read per LUT texel. The aerial producer
adds one per depth interval, without a new sky-sized attachment or CPU–GPU
synchronization. Compare whether a
night contribution below stored precision can skip that work. Prepare graph variants with and without stars. The star variant
adds a depth read, depth retention and colour attachment load/store. The
day variant retains today's depth discard where no other consumer needs it.
Select it only on a profile range where the brightest star is shown to be
subvisible. Record logical attachment area, named GPU intervals, full-frame cost and motion on Mac and Chrome.
Physical tile traffic at night and through the transition is queued with
[Apple tile M0.4](../APPLE-TILE/PLAN.md#m0-proof-route-counters-and-capture-probe-pending).
Investigate visible hitches
and comparable regressions over the build plan tripwire.

The isotropic and unattenuated-shell controls gave 0.54 and 0.77 millilux
horizontal. These are storage probes. At that range the fixed 1/1024 scale
quantizes a 10% Lambert surface to zero, and the revised scale also puts a
1% Lambert probe above the fp16 normal floor. The finite-shell reference with the mixed background yields 0.671 millilux
at ground level. At 1.8 km, it predicts roughly 1.24, 11.8 and 31.5 μcd/m²
of first-scatter along horizontal 1, 10 and 30 km paths. Doubling the
quadrature changed sampled radiance by less than 0.065%. These are scalar
model checks. Packed GPU transport and visible night remain open.

## Environment identity and products

M5. Other engine modules read the one sky and Sun instead of bringing their own, and one cloud optical depth
dims direct sunlight once. `SEA` and `CLOUDS` are the consumers ([module list](../ARCHITECTURE.md#interfaces)); the
forward pass and cooked draws already are.

### Cues

| Cue | Where it must hold | Bar |
|---|---|---|
| E1 Anything a module draws is lit by the same Sun and sky as the terrain beside it, at every time of day | A contribution's own pipeline, on Metal and in Chrome, noon through night | No visible seam in brightness or colour between a module's surface and the scene's |
| E2 Cloud shade crosses the land: the patch dims direct sunlight only, moves in world space and agrees on every lit surface | The V1 valley at noon and low Sun; terrain, synthetic instances, cooked meshes and the cooked volume | Cloud shade in Red Dead Redemption 2 or Horizon Forbidden West: shaded ground stays sky-lit and blue-leaning, never black |
| E3 With no cloud source, nothing changes | Every scene today | Identical pixels and identical per-frame work |

### Mechanism

**One identity.** Each frame the atmosphere publishes one environment identity and lends it to every scene
contribution: the source epoch and simulation time; the **true Sun** as geometric receiver-to-source direction,
raw perpendicular illuminance, photometric colour and angular radius, with no air, cloud or terrain attenuation
applied; and, kept apart, the apparent surface light the forward pass uses today. The epoch changes exactly when
the sky model, the air profile or a LUT generation changes. A LUT generation is a replacement of the resources
or of the model behind them: a new device, a changed chart size or format. Recomputing a chart's contents as
time advances is not a generation, so a time scrub never moves the epoch. This is
[C08](../../design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4)'s light handoff made concrete.
The true Sun is separate because the apparent light changes quickly as the last surface light disappears, and a
medium lit from it flickers; the Broadsides cloud work found and repaired exactly this.

**Lent products, not copies.** A contribution receives a bind-group layout and a WGSL library for the products
the atmosphere already computes: the transmittance and multiple-scattering charts, the sky-view LUT, the aerial
volume, the diffuse and reflection products and the exposure scale. The library exposes five operations. They
are the functions the forward pass calls (`sun_irradiance`, the shared aerial decoder, `ambient_irradiance` and
the reflection read inside `indirect_radiance`), moved to where a contribution's own pipeline can compose them.
The resources stay owned by the atmosphere and are read-only to everyone else. Chart sizes, mip counts, storage
gains and channel packing stay private: a contribution passes physical arguments and gets decoded scene-linear
values in physical units, with no pre-exposure applied. Transmittance and the split-sum pair are dimensionless.
The exposure scale is lent beside them and the library never applies it. A contribution multiplies its finished
colour by that scale once, as it writes to the scene target, after it has combined its lighting and composed
aerial `(T,S)`. The forward pass does the same in the last line of its shading.

Each chart lookup is one hardware-filtered read per chart layer of a half-float chart, as the forward pass reads
it today. A GPU may return a filtered half-float read at the stored format's precision, and the M3 Max does. So a
lent sky, aerial or reflection value is resolved to one stored step, at most 0.1% of the value, and not to
float32 precision. The stored texels are already rounded to that step.

| Operation | Returns | Support, and what happens outside it |
|---|---|---|
| Sun light at a world position | RGB illuminance on a plane facing the Sun, from the true Sun through the air only; no geometric shadow, no cloud factor | Any world position. Above the 100 km top the light is unattenuated unless the ray enters the shell. Below the first chart row (6.15 m) the altitude clamps, a known approximation that reaches 1.2% in blue at sea level with the Sun on the horizon. A Sun behind the planet at that point returns exactly zero |
| Sky radiance along a direction | RGB radiance of the sky without the Sun's disk: the solar part and the night part, already summed | The active camera's observer altitude only; any direction, including the ground side below the horizon. The night part exists for 0–15 km observer altitude; outside that the result is Sun-only and the fallback is reported, as today |
| Incident diffuse on a normal | RGB irradiance from the sky, never negative | One answer per frame at the camera's location, for any normal. It does not vary with world position |
| Aerial `(T,S)` to a point | RGB transmittance and in-scattered radiance, night included, accumulated **from the active camera** to a point in its view | The active view only, addressed by the point's screen position and its distance from the camera. Beyond 32 km it returns the 32 km value. A segment that starts anywhere but the camera, and any second camera, is not offered in M5; a consumer that needs one proposes it through [the module list](../ARCHITECTURE.md#interfaces) |
| Environment reflection | RGB radiance of the disk-free sky filtered to a GGX lobe around a direction at a roughness, solar and night parts decoded and summed; and the split-sum pair `(scale, bias)` for a view cosine and the same roughness | One cube per frame at the camera's location, no parallax. Roughness is the material roughness the forward pass uses, clamped to 0.045–1. View cosine 0–1. The library maps roughness to its filtered level; a contribution never names a mip |

The reflection operation is the one `SEA` uses as the fallback under its screen-space reflection. It returns the
filtered radiance and the pair separately, so a consumer can apply its own Fresnel term.

**One Sun-path optical depth.** C08 and [ENVIRONMENT](../../design/modules/ENVIRONMENT.md) already say that a cloud source supplies
path optical depth `tau` toward the Sun and that V3 applies `exp(-tau)` at most once. A provider registers one
product: a 2D field of `tau` along the current true Sun direction for receivers below a declared plane, with its
world mapping, advection displacement, validity region, lowest valid Sun elevation and source epoch. A receiver
reads the field where its own ray toward the true Sun crosses the plane, so receivers at different heights under
one cloud agree. One cloud-factor function does that read and returns `exp(-tau)`. The shared direct-Sun term
calls it in the same place it reads surface transmittance, so terrain, synthetic instances and cooked meshes get
the factor once. Sky light, emission, geometric shadow and aerial in-scatter are not touched; shading of
in-scatter by clouds is `CLOUDS`' later decision.

The cooked volume is the one receiver that does not go through the direct-Sun term today: its march lights each
sample from the frame's directional light. With a provider, the march multiplies that Sun term by the same
cloud-factor function at each lit march sample's world position. Without one, the volume's shader and lighting
are exactly today's. M5 does not move the volume onto per-sample air transmittance; that would change every
scene with a volume and nobody asked for it.

**Where the factor is exactly 1.** A receiver at or above the plane, a read that falls outside the validity
region, a frame the provider marks invalid, and a true Sun below the provider's lowest valid elevation all get no
cloud shade: the factor is exactly 1, never a clamped edge value. The read moves further from the receiver as
the Sun lowers and has no answer at a horizontal Sun, which is why the provider declares a lowest elevation. The
provider owns continuity at these limits, by bringing `tau` to zero toward the region's edge and toward its
lowest elevation. With the Sun behind the planet the direct term is already zero. M5 supports one plane.

**Absent means absent.** Provider presence is a scene-build fact. Without a provider the forward shader and the
cooked-volume shader are composed without the lookup, as they are today, so there is no added texture, binding or
sample. With one, the shader variant adds one sampled texture to the light group.

**A diagnostic provider** in `examples/v1-game`, behind an opt-in control and labelled as a stand-in, supplies a
moving analytic `tau` pattern with a declared drift, plane, region and lowest valid Sun elevation; that elevation is
below the low-Sun views M5 captures. It exists to prove the path; `CLOUDS` replaces it.

### Alternatives considered

- **Bake cloud attenuation into the Sun's illuminance.** One number cannot vary across the land, and C08 forbids
  pairing an already-attenuated Sun with a cloud query.
- **Draw cloud shadows into the Sun's shadow cascades.** Cascades hold binary geometric depth; soft, semi-transparent
  shade would need a second map per cascade and would be applied by a different path from the one the sea reads.
- **A 1×1 placeholder texture when no provider exists.** Simpler, but every scene would pay a sample for a
  feature it does not use, against the engine's rule that off removes the work.
- **Let each module sample the LUTs through its own private copies of the lookup code.** Two decoders drift; one
  library keeps aerial perspective applied once.
- **Blend exact texels in the shader, to get float32 results.** Four texel loads replace the sky's one filtered
  read, and sixteen replace the aerial volume's four, on every sky pixel and lit fragment. The forward pass shares
  these functions, so its shader text and every scene's image would change. The stored texels are already rounded
  to the same half-float step, so the product would be no more accurate, only smoother below 0.1% of the value.
  M6 judges sky and twilight banding by eye, and a visible step there reopens this.
- **Float32 charts.** Filtering a 32-bit float texture is an optional WebGPU feature (`float32-filterable`), so
  the portable recipe could not rely on it, and the charts' memory and bandwidth would double.

### Check that it can work

The forward pass already forms direct sunlight in one function and already lends its light group layout and
composed shader source to contributions, so the cooked mesh path is lit by the same code. The portable limit is
16 sampled textures per stage; the builder records each forward pipeline's count with the provider on, read from
the capability record, before choosing the binding. The lookups are pure functions of the existing charts, so a
GPU test can run a contribution-side evaluation through the cases of the referees that already exist. Which
referee, which cases and which bound each operation must meet is the table below.

### Numerical acceptance

Two different things are proved, and they are kept apart.

**Preserved referees.** These tests already run in `./dev check` on Metal against the production producers.
Each compares one product with a reference inside a stated scope. M5 and M6 must leave their references, cases
and bounds unchanged and passing. They are the whole of what is claimed for product accuracy.

| | Product and quantity | Reference | Cases | Bound |
|---|---|---|---|---|
| P1 | Sun transmittance chart, RGB, dimensionless | f64 direct-light quadrature | 0, 0.1, 2 and 6.15 m; Sun at 0, 0.5, 1, 2 and 5° | Relative error at most 0.6, 0.9 and 1.6% in R, G and B, with the denominator floored at 0.001 |
| P2 | Sun light at a world position, as illuminance over the raw illuminance and colour | f64 receiver transmission | Seven receivers from 0 to 5,000 m and ±20 km at a 245 m camera, Sun at −2, −0.6, 0, 1 and 30°; three receivers at 100–130 km, Sun at −13, −10.07, −1 and 5°; two camera origins | Absolute error at most 0.05 × expected + 0.005; exactly zero behind the local horizon |
| P3 | Night sky luminance, cd/m² | f64 night profile | 0, 1.8 and 15 km; zenith, peak and both split-horizon rows | Relative error at most 10% |
| P4 | Night horizontal illuminance, lux | f64 night profile | 0, 1.8 and 15 km | Relative error at most 5% |
| P5 | Night aerial in-scatter, cd/m² | f64 shell and diffuse paths | Horizontal 1, 10 and 30 km paths at 1.8 km | Relative error at most 10% |
| P6 | Incident diffuse: nine SH coefficients and irradiance on 14 normals, per unit Sun | f64 hemisphere integral of the read-back sky, refined until it moves under 0.25% of the sky's zeroth moment M0 | Sun at 90, 45 (two azimuths), 0, −3, −6, −9 and −12° | Coefficients within 1% of M0; irradiance within the SH truncation bound of 3/32 M0 plus the projection error |
| P7 | Reflection cube, radiance on a direction-coded sky with values in 1–2 | Closed form at the sharpest level and at the widest level's axes | Two Sun azimuths with the sky rewritten and refiltered between them; face centres, every edge and every corner; all levels | Absolute error under 1/1024 + 0.019; under 0.01 across an edge or corner |
| P8 | Split-sum pair | Independent Gauss–Legendre hemisphere quadrature | Six view-cosine and roughness nodes and the two analytic limits | Absolute error under 0.006; the pair never sums over 1.001 |
| P9 | Reflection as a consumer composes it | White furnace and source scaling | Base colour, metalness, three roughnesses and five view cosines; raw Sun at 1, 0 and 2 | At most 1.002 × source + 1e-6; exactly zero facing away or with no source; doubling within 2e-6 |
| P10 | Aerial storage and decode, solar part | A float32 run of the same integrator, and a fixed-texel decode | Sun at 2, 0, −2, −4, −6 and −9° | Within 0.1% plus half a storage step; decode within 1e-5 relative |
| P11 | One upper-twilight spectral sky row, radiance per unit Sun | The recorded eight-node spectral value | Sun −5.08°, view 20° up, 95.1° from the Sun, 245 m | Relative error at most 2.5% per channel |

P6 and P10 take their reference from the atmosphere's own sky or integrator, so they qualify the projection and
the storage, not the sky. The all-orders solar comparison (198 rows against quadrature and Monte Carlo) stays in
the check as a **report**. It has no bound, and passing it qualifies nothing.

**What stays unqualified.** No test bounds the solar sky radiance or the solar aerial `(T,S)` against an
independent transport reference, apart from the one row of P11. That was true before M5, M5 and M6 do not change
it, and M6 writes it in the maturity ledger. Closing it is a separate accuracy campaign, with its own plan and
cost.

**The handoff, new in M5.** For each lent operation, a contribution with its own pipeline, using only the lent
layout and library, must return the stored product in the right units. This runs on Metal in `./dev check` and
in Chrome on hardware WebGPU. The expected values come from the same references on both; Chrome is never judged
against Metal's GPU output.

| | Lent operation | What the contribution-side result must meet |
|---|---|---|
| H1 | Sun light at a world position | The cases and bounds of P1 and P2, against their f64 references |
| H2 | Sky radiance along a direction | Night part: the cases and bound of P3. Solar part: the filtered-decode rule below, on the live sky-view chart at the 11 view directions and six Sun elevations of the all-orders grid, and on a known field. Its three wrong lookups fail. Plus the P11 row at its bound |
| H3 | Incident diffuse on a normal | The cases and bounds of P6; with nightglow on, the added horizontal irradiance within P4's bound |
| H4 | Aerial `(T,S)` to a point | Night part: the cases and bound of P5. Solar part and transmittance: P10's fixed-texel decode at its bound, and the filtered-decode rule below, on the live volume at P10's Sun elevations and on a known field. Its three wrong lookups fail |
| H5 | Environment reflection | The cases and bounds of P7, P8 and P9 through the lent library. The same run also stores a known night term in the fixture and requires it back in absolute units, unchanged by the raw Sun's illuminance, within P7's rounding allowance |

**The filtered-decode rule, for H2 and H4.** Lying between the neighbouring texels is not enough: a lookup that
returns one depth slice instead of blending two stays in that range at every query. So each result is compared
with the one value a correct lookup returns at that point.

- **Expected value.** The test reads back the stored texels and evaluates, in f64 on the CPU, the operation's
  addressing and blend at the query, then decodes to physical units. For the sky this is the CPU model's
  direction-to-chart mapping and a bilinear blend of four texels of one horizon half, with coordinates held to
  the edge texel centres as the production read holds them. For the aerial volume it is a bilinear blend in the
  point's screen position inside a depth slice, then a linear blend by distance between the two adjacent slices
  at their square-root spacing, held at 32 km. This evaluation is written in the test. It does not call, include
  or translate the lent library's code.
- **Bound.** Per channel, the result is within `C/32 + 1e-5 × M + Q` of the expected value. `C` is the largest
  minus the smallest of the decoded texels the blend reads there, and `M` is the largest of them. `C/32` allows
  each sampler axis a weight error of 1/64, four times the 1/256 step of the 8-bit weights GPU samplers commonly
  keep. `1e-5 × M` is P10's decode allowance for float32 arithmetic. `Q` is one stored step: the spacing of
  half-float values at the largest stored magnitude among the texels the blend reads, taken per stored channel
  and carried through the same decode scale as the value. Where a decode sums two stored channels, as the solar
  part and the night term are summed, each adds its own step.
- **Why `Q`.** The read-back texels are exact, but the filter's result is not. On the M3 Max, Metal returns a
  filtered `RGBA16Float` read rounded to the nearest half-float. Across 257 positions between two texels, a pair
  far apart returns 257 distinct values inside the old bound, and a pair one stored step apart returns only
  those two values. The live sky chart missed the old bound by this rounding and by nothing else: addressing
  agreed with the f64 evaluation to 0.000007 texels. WebGPU and Metal do not specify the precision or rounding
  of a filtered result, so `Q` is a whole step, which covers a filter that rounds to nearest and one that
  truncates. The Metal run keeps this 257-position measurement and prints, for both pairs, the count of distinct
  values and the largest error in stored steps. That is a record of the target; the bound is the gate.
- **A further miss.** If a correct lookup misses this bound on a target, the builder records that target's
  measured weight and result resolution and brings it back as a plan change. The bound is not widened during
  the build.
- **Live arm.** The production chart or volume, at the row's named cases. It shows that the lookup reads the
  right resource with the right gain and mapping on real contents. For aerial `S` the fixture path has no
  geometric shadow, so the result equals the stored in-scatter.
- **Known-field arm.** The fixture writes a field through the lent layout, as P10's fixed-texel fixture does.
  Its value differs between neighbouring texels by a comparable step along every blended axis (both chart axes
  for the sky; both screen axes and depth for the volume) and differs between channels. Around every query that
  step is at least 32 stored steps on each blended axis and in each channel, so `Q` cannot hide a wrong lookup;
  the wrong lookups below are the proof. A plain ramp loses that ratio away from the chart's origin, because the
  stored step grows with the value; a repeating pattern keeps it. Queries sit a quarter,
  a half and three quarters of the way between texel centres on every axis at once. For the sky they cover
  both halves of the chart, the row beside the horizon split and two Sun azimuths. For the volume they cover
  the first, a middle and the last pair of slices, and one point beyond 32 km that must return the 32 km value.
- **Wrong lookups that must fail.** The same harness runs deliberately wrong lookups through the same bound on
  the known field, and each must fail at every query that sits between texel centres. Sky: the nearest single texel, one row
  alone, and one column alone. Aerial: the lower slice alone, the upper slice alone, and the nearest
  single texel in screen. A run in which one of them passes is a failed run, on Metal and in Chrome.

The rule proves the handoff, the addressing and the units. It does not prove the accuracy of what is stored.

**Exposure control.** At least one case of each row H1–H5 is repeated with the lent exposure scale set to a
value other than 1, and meets the same bound against the same physical reference. A library that applied the
scale would fail it.

H5 varies direction, roughness (the sharpest and widest levels have different closed forms), the raw Sun's
illuminance, the night term and a rewritten sky, so a constant, stale or wrongly scaled reflection fails.

The Chrome run needs a page that drives the production producers with the referees' frames, as
[the cooked-draw browser check](../../../scripts/cooked_draw_browser.mjs) drives its own page. The builder
arranges it.

### State and owners

| State | Owner |
|---|---|
| Sky, transmittance, aerial, diffuse and reflection products; the environment identity | `ATMOSPHERE` |
| The Sun's timed direction and raw illuminance | The environment source (E1), as today |
| Exposure and storage scale | V4 (`APPLE-PRESENTATION`), lent alongside |
| The optical-depth field's contents | Its provider (`CLOUDS`; the game's diagnostic until then) |
| Applying `exp(-tau)` on the direct-Sun path | `ATMOSPHERE` (V3 transport) |

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| Shade applied twice or to the wrong term | Shaded ground goes black, or shade deepens inside geometric shadow | Darkening final colour instead of direct irradiance |
| The off path is not free | The forward shader text or pipeline count changes with no provider | A placeholder texture |
| Receivers disagree | A cooked mesh stays bright inside a patch that darkens the terrain under it | Patching one receiver's shader separately |
| The cooked volume is skipped, or reads the field at the wrong point | The volume stays bright under a patch, or its shade ignores where its samples are | Reading the field once at the volume's centre or at the fragment |
| A lent sky or aerial lookup steps where the product blends | An independent surface shows bands in depth or across the sky that the terrain beside it does not | Accepting any value between the neighbouring texels |
| A contribution pre-exposes twice, or not at all | A module's surface is brighter or darker than the scene by the exposure scale, and the gap changes as the meter moves | Applying the scale inside a lent lookup |
| The reflection handoff is wrong while the forward pass is right | The first independent consumer shows a sky of the wrong brightness, or copies mip and gain rules to fix it | Testing only the forward fragment path |
| The texture limit is hit in Chrome | Pipeline creation fails on the portable recipe with the provider on | Dropping another texture silently |
| The lent library exposes internals modules then depend on | A contribution reads chart layouts directly | Publishing chart formats as interface |

### Cost

With no provider: nothing. With one: one filtered 2D sample per lit surface fragment; one per lit march sample
in a cooked volume, so a volume pays its step count and not one read per pixel; and the provider's own work to
produce the field, which is its cost to account for. Record forward GPU time and whole-frame time paired on and
off on the valley, with load and order, on Mac and in Chrome; set no budget. The valley has no volume, so its
figures say nothing about one; a volume's added cost is its lit step count times one read, and is measured when a
scene with a volume is on the route.

### Open questions

- Whether aerial in-scatter should also be shaded by cloud. Settled in `CLOUDS`, with a joint integrator or a
  declared approximation as C08 requires.
- Whether the field needs more than one height band. The diagnostic uses one plane; `CLOUDS` may propose more
  through [the module list](../ARCHITECTURE.md#interfaces).
- Whether a consumer needs aerial transport for a segment that does not start at the camera, or for a second
  camera. Neither is offered in M5; `CLOUDS` or `SEA` proposes it the same way.

## Closing G3s

M6. The gate closes when the valley's sky and sunlight pass a clean-context review in motion on Mac and Chrome.
It runs after TAAU is in the game (`APPLE-PRESENTATION:M3`), because reconstruction changes every edge and
gradient a reviewer judges.

**What is reviewed.** The fixed six-stop route of [the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md) and one
continuous scrub from dawn through noon, sunset and twilight to night, with slow pans, captured from the running
game with the engine tap. The inspection list is the set of defects earlier reviews recorded: a horizontal
brightness step across the ground; a flat near ridge with a hard bright rim at low Sun; a ridge lit warm when the
Sun is behind it; missing object shadows at sunrise; shadow acne at low Sun; a hard horizon line; and banding or
steps in the sky gradient and twilight colour. The bar is the time of day in Red Dead Redemption 2 and Horizon
Forbidden West, in words.

**What keeps it honest.** The preserved referees P1–P11 and the handoff rows H1–H5 of
[the numerical acceptance](#numerical-acceptance) still pass on the final build: all of them on Metal in the
workspace check, and H1–H5 in Chrome. No reference, case or bound may change in M6 to make a picture pass, so a
fix that pleases the eye cannot move a qualified product off its model. This is preservation. It does not newly
qualify the solar sky or solar aerial transport against an all-orders reference; the ledger names that as open.
Twilight colour is covered numerically by the one row of P11 and otherwise judged by eye in the scrub.

**Cost.** Whole-frame and atmosphere pass costs on the route are measured on the final build and, in the same
session and configuration (extent, TAAU on, load class, order alternated), on the commit M6 started from. That
pair is the comparison. The M1 and M2 records predate TAAU and describe other builds and loads; they are quoted
as history and not called paired.

**What is out.** The west-lookout shadow crossing (M3) stays a parked spike and the exact finite-disk match (M4)
stays deferred; both are named in the review as known and are not judged. The exclusion is narrow: it covers the
moving shadow edge on the authored spur between 17.40 and 17.80 h. The held `sunset-clear-sun` stop at 17.4 h is
judged like every other stop, and the terrain lip recorded on the shipped Ground E pan is a separate finding,
judged under the brightness-step entry. Moon, weather and calibrated night colour stay outside the profile, as
the first section says.

**Risk.** Shadow defects have consumed days here before. If the review fails on one item twice, the builder stops
and reports it instead of opening a new shadow method; a new method is a re-plan with its own cost account.

## The view volume: haze, fog and light near the camera

M7 and M8, added on 14 November 2026. They are track 2: they follow M6, gate nothing in the V1 exit, and leave
every scene that does not ask for them as it is.

**The owner's request**, 14 November 2026: "Oh and for clouds we also need to be taking into account mist / fog.
We want a plan that's able to deliver on top quality atmosphere." The lead's task with it: "Revise the plans so
Thrum's atmosphere reaches the quality of shipped high-end games as a whole: sky, clouds, haze, and mist and fog
(ground fog, valley mist, fog banks over water, low cloud the camera can enter), lit consistently and changing
with the weather and time of day. Settle from first principles which module owns what across CLOUDS, ATMOSPHERE
and any other affected module, keep one authority for each kind of state".

### Who owns what

The atmosphere is one medium with two kinds of state, and each has one owner.

| State | Owner |
|---|---|
| What is in the air and where: cloud, fog and mist as one condensate, and how much haze there is | The weather, in `CLOUDS` ([its design](../CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud)) |
| What light does on its way to the eye and to a surface: the air's own optics, the charts, the sky, and the transport from the camera to any point | This module |
| The far condensate's picture, beyond the handover | `CLOUDS`' trace, composed through this module's lent transport |

So this module never evaluates a density, a droplet phase or a cloud's self-shadow, and `CLOUDS` never
integrates the air or lights a surface. Fog near the camera overlaps the air, which
[C08](../../design/CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4) requires to be integrated
jointly by one owner; the aerial product is already that owner's, so the joint integrator is this module's.

### What this replaces

- **The air's profile is fixed.** The aerosol is a constant written into the shaders, a visual range of about
  244 km. It becomes an input, so the weather can thicken and thin the haze.
- **The aerial product is clear air in a 32³ volume.** In a scene that declares the view volume it is the joint
  transport of air, haze and a provider's medium, at a finer grain near the camera. A scene that does not
  declare it keeps the 32³ product, bit for bit.
- **One plane for the Sun-path optical depth**, with a factor of exactly 1 at or above it
  ([M5](#environment-identity-and-products), and its open question on a second height band). A provider may add
  a ground band read at the receiver's own position.
- **In-scatter under cloud is `CLOUDS`' scalar** (that section's first open question). Inside the view volume
  the air is lit sample by sample, through the scene's shadows and the cloud factor. The scalar stays for a
  scene without the view volume.
- **`CLOUDS`' builder builds what clouds need here** (the module list's rulings 1 to 4). Those stand. The view
  volume is larger and is judged by its own inspections, so it is this module's own milestones.
- **[RENDER-SPEC §5](../../design/RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator)'s joint froxel
  integrator** was a proposal with no owner or plan. This is it, for the Sun and the sky; its participating
  local lights are not built.

### Cues

| Cue | Where it must hold | Plausible wrong substitute, and what rejects it |
|---|---|---|
| V1 Haze: the land fades with distance into the sky's colour, over a range the weather sets | Clear-air visual range 3 to 300 km; noon to twilight; Mac and Chrome | A tint by depth. Rejected by the sky and the farthest land meeting without a line at every range |
| V2 Shafts: sunlight stands in the haze below ridges, props and broken cloud | Low Sun and noon; visual range 8 to 60 km; within the shadow cascades and under a registered Sun-path optical depth | A screen-space streak from the Sun's disc. Rejected by shafts that stay when the Sun is off screen and that meet the lit ground beneath them |
| V3 A medium near the camera: the land fades into fog by depth, surfaces inside lose their sunlight by the fog between them and the Sun, the sky, the Sun's disc and stars dim through it, and the camera passes in and out | A provider's medium of visual range 30 m to 10 km, from the camera to the provider's handover | A fog composed over the finished frame by depth. Rejected by a slab integrated jointly with the air against the reference, and by a shadow that weakens inside the fog |
| V4 Steady: nothing crawls, trails or flickers as the camera pans, walks or the Sun moves, and a cut is whole in its first frame | Pan, walk, time scrub, cut; a mover through the fog's edge | Heavy history that hides noise while the camera is still. Rejected frame by frame in the walk and behind the mover |
| V5 Absent means absent | Every scene that declares no view volume | A volume that is filled and not read. Rejected by identical shader text, passes, resources and frames |

The bar is haze, shafts and near fog in Red Dead Redemption 2 and The Last of Us Part II, in words.

### Research

Fog near the camera in shipped engines is a grid aligned to the view frustum
(Wronski 2014; [Hillaire 2015](https://www.slideshare.net/DICEStudio/physically-based-and-unified-volumetric-rendering-in-frostbite),
slide text read; [Unreal's volumetric fog](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-fog-in-unreal-engine),
read whole; The Last of Us Part II, abstract read). Frostbite's is 8×8-pixel tiles by 64 slices: media are
written into it as extinction, scattering and phase, a pass lights each cell from the Sun through the shadow
cascades and from the sky, samples are jittered and blended 5% new to 95% history, and a last pass integrates
along the ray with a step that conserves energy. It cost 2.95 ms on a PlayStation 4 at 900p; Unreal's costs 1 ms
there. Unreal states the limits this design avoids: its grid hands over to an analytic fog that "can't be
blended to match precisely", and its fog casts no shadow. Red Dead Redemption 2 (Bauer 2019, abstract only)
joins "cloud and fog rendering" in one "voxelization and raymarching approach". Hillaire 2020's aerial volume,
which this module already has, is the same grid at 32³ with the air alone in it.

| Candidate | Verdict |
|---|---|
| Keep the 32³ aerial volume and add an analytic height fog on top | Two media integrated apart over the same air, which C08 rejects; no shafts; fog that cannot pool, drift or take a bank's shape |
| A second grid for fog, composed with the aerial volume | The same overlap, and two lookups on every surface |
| Fog marched for each pixel after the opaque pass | Transparent surfaces and water cannot read it at their depth, and its history breaks at every moving edge near the camera |
| **One grid: the aerial product itself, integrating the air, the haze and a provider's medium coefficients jointly, lit through the scene's shadows, with history** | **Selected.** Every reader of the lent aerial operation gets fog and haze once, with no change of its own |

### Mechanism

**A scene declares it.** The view volume is a scene-build fact, like the Sun-path provider. Without it the
aerial product, the shaders, the passes and the frames are today's. With it the aerial product is the view
volume, and the lent "aerial `(T,S)` to a point" keeps its arguments, units and support: any reader gets the
joint transport.

**The air's haze is an input.** The air profile takes the aerosol's amount at the ground, with today's value as
the default. The charts that depend on it (transmittance, multiple scattering, the sky view, the night profile)
follow a change within a lag the profile declares; at the default their bytes are today's. Changing the amount
is a recomputation of contents, like the Sun moving, and does not move the environment epoch. A weather provider
supplies the amount; without one a scene sets it.

**The grid.** Cells aligned to the view frustum: tiles of pixels across, slices in distance. Most slices lie
inside the provider's handover distance, where condensate is; the rest carry air and haze to 32 km. The sizes
are recipe values and knobs (RENDER-SPEC proposes 160×90×64 on desktop and 128×72×32 in the browser, and
`CLOUDS`' proof part IV gives the cell sizes).

**A cell.** Each frame every cell is sampled at one jittered point. There the pass takes the air's own
coefficients; a registered provider's **medium coefficients**, read from a texture the provider writes at the
same points (extinction, a solar kernel and an ambient source, already weighted by the provider's near weight);
the Sun's light at the point, from the transmittance chart; the scene's shadow, from the cascades inside their
range; and the Sun-path optical depth. The air's single-scattered sunlight is multiplied by the shadow and by
the cloud factor at the point. The provider's solar kernel is multiplied by the Sun's light and the shadow
only, because the provider has already put its own optical depth into the kernel, once for each scattering
order. Coefficients are summed, then the cell is integrated with the step that conserves energy.

**History.** A cell's value is blended with its history, reprojected through the cell's world position and the
previous view. History advances only with submitted frames, and a cut, a new epoch, a replaced device or a
provider's reset marks it invalid, after which the volume is filled from that frame alone and settles within a
declared number of frames.

**Rays that leave the volume.** Beside the joint pair, the volume carries at its far end the provider's
transmittance `R` and the in-scatter the medium added over clear air, `ΔS`. The sky, the Sun's disc and the
stars, which are drawn through clear air, become `R·L + ΔS`. With no medium `R` is 1 and `ΔS` is 0, exactly.
`CLOUDS`' compose already reads the lent transport at its cloud's front distance, so it needs no change.

**The ground band.** A provider may register, beside its 2D Sun-path field, a ground band: optical depth toward
the Sun at heights above the ground, read at a receiver's own position and added to the 2D field's value. The
one cloud-factor function reads both, so every direct-Sun receiver, the cooked volume's samples and the
incident-light meter take fog's shade once. The band may also say where the land blocks the Sun beyond the
cascades; only the view volume's cells read that, never a surface.

**A test medium** in `examples/v1-game`, behind an opt-in control and labelled as a stand-in, writes the medium
coefficients and a ground band for an analytic bank. It proves the path; `CLOUDS` M14 replaces it.

### Check that it can work

The joint transport is the aerial integrator this module already runs, with more cells, more terms in each and a
history. Its parts are shipped practice at 1 to 3 ms on a 2013 console. `CLOUDS`' design proof
([part I](../../evidence/CLOUDS/design-proof/whole-atmosphere-proof.txt)) shows that a medium split by a near
weight composes with the far part to the exact transmittance. The identity for a ray that leaves holds because
the medium's transmittance multiplies the air's: `T_joint = R·T_air`, so `S_joint + T_joint·L_beyond` equals
`ΔS + R·(S_air + T_air·L_beyond)` with `ΔS = S_joint − R·S_air`. Unrun: the look, the history's steadiness and
the cost, which M7 sees first.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The off path is not free | A scene without the view volume composes different shader text or one more pass | Filling the volume and not reading it |
| Sunlight is shadowed twice in the air | Shafts are too dark, or a shadowed surface's haze differs from the haze beside it | Leaving the forward decoder's own split of solar in-scatter on top of the volume's |
| The history trails or flickers | A mover leaves a wake in fog; a shaft's edge crawls in a pan | A longer history |
| The night profile cannot follow a moving haze | Preparing it for a new amount takes longer than the declared lag | Freezing the haze at night without saying so. If preparation is too slow, M7 records it and the proposal for a prepared set of amounts goes to the log |
| Chrome's sampled-texture limit | Pipeline creation fails with a ground band and a medium registered | Dropping a texture silently |
| The grid is too coarse for a fog's edge on terrain | Tiles show where fog meets a slope | Blurring the fog |

### Cost

Without the view volume: nothing. With it, each frame: one sample for each cell (0.6 to 1.4 million at the
proposed sizes) of the air, the charts, the cascades and the Sun-path field, then the integration; with a
provider, three more reads a cell and the provider's own pass. Recorded paired, on and off, on the Mac and in
Chrome, with tripwires and no budget ([brief](../BRIEF.md), decision 3).

### Not built

Local lights in the air, a volume for a second camera (so [a mirror view](../ARCHITECTURE.md#scene-views)
carries clear air until its own plan says otherwise), fog indoors, and any density of this module's own.
