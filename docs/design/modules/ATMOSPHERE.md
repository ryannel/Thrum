# Atmosphere for the V1 outdoor scene

[Rendering contract](../RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator) ·
[Environment source](ENVIRONMENT.md) · [Milestones](ATMOSPHERE-PLAN.md) · [Done when](ATMOSPHERE-DONE.json) ·
[Module list](ARCHITECTURE.md)

**The owner's request** for the work added on 10 November (M5 and M6), word for word from that day's planning
task: "The owner wants work lined up continuously toward the V1 exit in docs/BUILD-PLAN.md, so builders always
have a next milestone, and wants water and clouds finished and consolidated ... Decide, with reasons ... where in
the build order the port's readiness conditions are met." The exit clause this module closes is "physical sky, Sun
and aerial perspective with time controls". The readiness condition it meets, from
[the port decision](../decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november), is "G3s: finish and qualify the one
shared sky, Sun, light-transmittance and environment-reflection products."

The sections below through [Frame and cost](#frame-and-cost) are the design M1–M4 were built from and are
unchanged. [Environment identity and products](#environment-identity-and-products) governs M5 and
[Closing G3s](#closing-g3s) governs M6.

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
[Apple tile M0.4](APPLE-TILE-PLAN.md#m0-proof-route-counters-and-capture-probe-pending).
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
dims direct sunlight once. `SEA` and `CLOUDS` are the consumers ([module list](ARCHITECTURE.md#interfaces)); the
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
the sky model, air profile or a LUT generation changes, and never when only time advances. This is
[C08](../CONTRACTS.md#c08-light-materials-and-transport--v3-with-e1-and-v4)'s light handoff made concrete.
The true Sun is separate because the apparent light changes quickly as the last surface light disappears, and a
medium lit from it flickers; the Broadsides cloud work found and repaired exactly this.

**Lent products, not copies.** A contribution receives a bind-group layout and a WGSL library for the products
the atmosphere already computes: the transmittance and multiple-scattering charts, the sky-view LUT, the aerial
volume, the diffuse and reflection products and the exposure scale. The library exposes four lookups: Sun
transmittance to a world position, sky radiance along a direction, incident diffuse, and aerial `(T,S)` for a
view segment. They are the functions the forward pass calls (`sun_irradiance` and the shared aerial decoder),
moved to where a contribution's own pipeline can compose them. The resources stay owned by the atmosphere and
are read-only to everyone else.

**One Sun-path optical depth.** C08 and [ENVIRONMENT](ENVIRONMENT.md) already say that a cloud source supplies
path optical depth `tau` toward the Sun and that V3 applies `exp(-tau)` at most once. A provider registers one
product: a 2D field of `tau` along the current true Sun direction for receivers below a declared plane, with its
world mapping, advection displacement, validity region and source epoch. The shared direct-Sun term multiplies
by `exp(-tau)` in the same place it reads surface transmittance, so every receiver that calls it gets the factor
once: terrain, synthetic instances, cooked meshes and the cooked volume. Sky light, emission, geometric shadow
and aerial in-scatter are not touched; shading of in-scatter by clouds is `CLOUDS`' later decision.

**Absent means absent.** Provider presence is a scene-build fact. Without a provider the forward shader is
composed without the lookup, as it is today, so there is no added texture, binding or sample. With one, the
shader variant adds one sampled texture to the light group.

**A diagnostic provider** in `examples/v1-game`, behind an opt-in control and labelled as a stand-in, supplies a
moving analytic `tau` pattern with a declared drift. It exists to prove the path; `CLOUDS` replaces it.

### Alternatives considered

- **Bake cloud attenuation into the Sun's illuminance.** One number cannot vary across the land, and C08 forbids
  pairing an already-attenuated Sun with a cloud query.
- **Draw cloud shadows into the Sun's shadow cascades.** Cascades hold binary geometric depth; soft, semi-transparent
  shade would need a second map per cascade and would be applied by a different path from the one the sea reads.
- **A 1×1 placeholder texture when no provider exists.** Simpler, but every scene would pay a sample for a
  feature it does not use, against the engine's rule that off removes the work.
- **Let each module sample the LUTs through its own private copies of the lookup code.** Two decoders drift; one
  library keeps aerial perspective applied once.

### Check that it can work

The forward pass already forms direct sunlight in one function and already lends its light group layout and
composed shader source to contributions, so the cooked mesh path is lit by the same code. The portable limit is
16 sampled textures per stage; the builder records each forward pipeline's count with the provider on, read from
the capability record, before choosing the binding. The lookups are pure functions of the existing charts, so a
GPU test can compare a contribution-side evaluation with the f64 references the M1 referees already use.

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
| The texture limit is hit in Chrome | Pipeline creation fails on the portable recipe with the provider on | Dropping another texture silently |
| The lent library exposes internals modules then depend on | A contribution reads chart layouts directly | Publishing chart formats as interface |

### Cost

With no provider: nothing. With one: one filtered 2D sample per lit fragment, and the provider's own work to
produce the field, which is its cost to account for. Record forward GPU time and whole-frame time paired on and
off on the valley, with load and order; set no budget.

### Open questions

- Whether aerial in-scatter should also be shaded by cloud. Settled in `CLOUDS`, with a joint integrator or a
  declared approximation as C08 requires.
- Whether the field needs more than one height band. The diagnostic uses one plane; `CLOUDS` may propose more
  through [the module list](ARCHITECTURE.md#interfaces).

## Closing G3s

M6. The gate closes when the valley's sky and sunlight pass a clean-context review in motion on Mac and Chrome.
It runs after TAAU is in the game (`APPLE-PRESENTATION:M3`), because reconstruction changes every edge and
gradient a reviewer judges.

**What is reviewed.** The fixed six-stop route of [the route decision](../decisions/G7-SHOWCASE-ROUTE.md) and one
continuous scrub from dawn through noon, sunset and twilight to night, with slow pans, captured from the running
game with the engine tap. The inspection list is the set of defects earlier reviews recorded: a horizontal
brightness step across the ground; a flat near ridge with a hard bright rim at low Sun; a ridge lit warm when the
Sun is behind it; missing object shadows at sunrise; shadow acne at low Sun; a hard horizon line; and banding or
steps in the sky gradient and twilight colour. The bar is the time of day in Red Dead Redemption 2 and Horizon
Forbidden West, in words.

**What keeps it honest.** The scalar and f64 references from M1 and M2 run on the shipped graph in the workspace
check, so a fix that pleases the eye cannot move radiance off the model.

**What is out.** The west-lookout shadow crossing (M3) stays a parked spike and the exact finite-disk match (M4)
stays deferred; both are named in the review as known and are not judged. Moon, weather and calibrated night
colour stay outside the profile, as the first section says.

**Risk.** Shadow defects have consumed days here before. If the review fails on one item twice, the builder stops
and reports it instead of opening a new shadow method; a new method is a re-plan with its own cost account.

