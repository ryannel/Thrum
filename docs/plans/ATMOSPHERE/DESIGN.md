# Atmosphere for the V1 outdoor scene

[Rendering contract](../../design/RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator) ·
[Environment source](../../design/modules/ENVIRONMENT.md) · [Work packages](PLAN.md) · [Done when](DONE.json) ·
[Module list](../ARCHITECTURE.md)

**The owner's request** for the work added on 10 November (WP5 and WP6), word for word from that day's planning
task: "The owner wants work lined up continuously toward the V1 exit in docs/BUILD-PLAN.md, so builders always
have a next work package, and wants water and clouds finished and consolidated ... Decide, with reasons ... where in
the build order the port's readiness conditions are met." The exit clause this module closes is "physical sky, Sun
and aerial perspective with time controls". The readiness condition it meets, from
[the port decision](../../design/decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november), is "G3s: finish and qualify the one
shared sky, Sun, light-transmittance and environment-reflection products."

[The view volume](#the-view-volume-haze-fog-and-light-near-the-camera), at the end, governs WP7 and WP8, added on
14 November for haze, fog and shafts of light; it is track 2 and follows G3s.
[The air at the ground](#the-air-at-the-ground) governs WP9, added the same day. WP9 is built before WP6: it
corrects the aerial product where the view meets the ground, which every scene at sea level shows.
[What the caster height map holds](#what-the-caster-height-map-holds) governs WP10, added on 18 November for a
defect `SHOWCASE` WP5 found on the populated valley. WP10 is built before WP6 and waits on nothing.
[Closing G3s](#closing-g3s) was revised on 19 November, after WP6's builder traced two of its list's entries to
the authored valley: `SHOWCASE` WP9 fixes them, and WP6 waits for it.

**The owner's bar, 17 November 2026:** "Ok let's lower the bar." Thrum is a game engine, and the aim is the most
plausible result as fast as possible, not physical accuracy. Rendering work is judged against its inspection list at
the bar of shipped games, with cost as tripwire records. An f64 reference is a diagnostic for a picture that looks wrong.
[Closing G3s](#closing-g3s) and [How the view volume is proved](#how-the-view-volume-is-proved) apply it; the
[project settings](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) hold the rule. The numerical acceptance
tables below record how WP1 to WP5 and WP9 were proved, and their tests stay in the workspace check as regression
guards.

The original design for WP1–WP4 is below through [Frame and cost](#frame-and-cost), with the remaining shadow work in
[Surface Sun visibility](#wp3-surface-sun-visibility). [Environment identity and products](#environment-identity-and-products)
governs WP5 and [Closing G3s](#closing-g3s) governs WP6. Both were revised on 10 November after
[the first design review](../../../.orchestrate/evidence/ATMOSPHERE/design-review-1.md): the numerical acceptance is written out
product by product, environment reflection is a lent and tested operation, and the limits of each lent lookup
and of the cloud field are stated. They were revised again on 11 November after
[the second review](../../../.orchestrate/evidence/ATMOSPHERE/design-review.md): the sky and aerial handoff rows H2 and H4 now
compare against an independently evaluated filtered decode, with wrong lookups that must fail, and the lent
values are stated to be free of pre-exposure. A third revision the same day follows WP5's builder measuring that
Metal returns a filtered half-float read at the stored format's precision
([entry 3](../../../.orchestrate/evidence/ATMOSPHERE/log-entries.md#entry-3)): the filtered-decode bound gains one stored step,
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

### WP3 surface Sun visibility

WP3 follows WP8 and runs one CPU trial for the sampling failure behind the in-cascade flash on the authored
west-lookout spur. The flash and the exact finite-disk match remain open in the [vision](../VISION.md#the-milestones).
The trial leaves a rerunnable reproduction and a decision on one candidate. The required game replay and
visual judgement belong to a separately reviewed correction.

#### Which inputs survive

The historical Metal probe at `ba45ff60c` compared depth and receiver state on authored spur cooked root
`32ed89be…`. At x=480, 720, 960 and 1200 on row 665 its samples stayed inside the last cascade at both
submissions 226 and 227. Blocker count stayed at six, the filter radius stayed near 53 texels, and dense
comparison still found 2.49–3.65% visibility after stock fell to zero. The separate 74–94 extinction
trajectory exposed the contour query's whole-receiver work-limit switch: 243 of 280 receivers switched at
77 and the largest step was 8.924 percentage points. The rotated-tap and spatial-resolve screens also left
visible patterns. The handoff's D32 fields, uniforms, spur source and rerun scripts are unavailable at
their named paths and have no restore entry. The later stock rerun identified the exact spur input copy
and reproduced the row-665 median and p95 steps of −50 and 72 red codes. That input copy is also missing.
[Historical handoff](../../../.orchestrate/evidence/v1-workload/G3S-HANDOFF.md#current-checkpoint) ·
[Archived D1–D3 criteria](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m3-the-west-lookouts-shadow-crosses-the-ground-without-a-flash-parked-spike)

Generate the reduced reproduction directly from the parameters below. Take the 16 odd-index taps and
four-texel bilinear depth comparison from `crates/thrum-render/src/synthetic.wgsl` at `c5e5290ba96f`.
Use a flat receiver at reverse depth 0.25, fixed 52.75-texel radius, and an interior footprint in a last
cascade. Along its vertical axis (positive opposite to the shader's UV y), set texel-centre signed
coordinates on half integers from the receiver. Centred at and below the edge are clear depth 0 and above
it are blocker depth 0.65. For an integer edge offset `e` this yields the bilinear comparison
`clamp(e + 0.5 - y, 0, 1)` at signed offset `y`. Move the footprint relative to the ridge: the crossing
samples use `e = −40, −46` texels. The extinction sweep has 21 samples, with `e = −40 − 0.75 i`
for `i = 0…20`. At each fractional offset use the same fixed depth step and translate the footprint by
the fractional part. This keeps the coordinate equivalent to the comparison formula above.
Use uniform clear and uniform blocked fields as endpoint controls.

The production taps give 12.50% then zero on the reduced crossing while dense integration gives 6.87%
then 2.70%. On the 21-sample sweep the stock filter reaches zero before the limb extinguishes. Save the
fixture recipe, stock shader identity and per-state values with the rerun command. The original spur replay
needs its mesh or cook and the submitted inputs for both trajectories. Ground E remains a separate control.
[Recovery audit and reconstruction](../../../.orchestrate/evidence/ATMOSPHERE/shadow-final-design-audit/README.md)

#### One bounded filter trial

Keep the stock cascade fit, blocker search, receiver-plane comparison, small-radius exits, overlap, far
fade and aerial evaluation. For the trial, replace only the unrestricted last-cascade disk-filter answer
with a normalized quadrature of that same bilinear comparison. Use 64 angular midpoints `θ = π(i+½)/64`
with rows at `y = −cos θ`. Take the mean at 16 equally spaced midpoints along each row's chord. Weight
the mean by `sin² θ`, which accounts for chord length and row spacing, then normalize by the sum
of the weights. This is 1,024 bilinear evaluations
and at most 4,096 depth loads per query, excluding the unchanged stock search. That is 64 times the
stock filter's 64 depth loads and is a CPU trial limit. Use the same footprint origin, radius, compare
predicate and depth field for stock, candidate and the dense same-field comparator on both trajectories.
On the reduced field, each chord has constant visibility. The dense comparator can integrate it with
32,768 uniformly spaced row midpoints, weighted by chord length. Record signed values and adjacent steps.
Whole-disk work and weights stay fixed even as only a sliver is visible. With a fixed
set of texel comparisons the bilinear field is continuous under translation and a varying radius; a
normalized weighted sum preserves that property and tends to zero as its samples lose coverage.
Re-fitting a map, changing a depth predicate or entering a stock exit can still change the inputs
abruptly, which is why the fixed trajectories and adjoining frames matter.

Park the candidate if it loses the remaining limb on the reduced crossing, reaches zero before the dense
comparison, rebounds on the reduced extinction sweep, is positive at its last state, or fails either uniform
control. Otherwise retain it as a diagnostic candidate. Either result finishes the one trial. An in-game
proposal needs the original replay and its own cost account.

#### Fixed witness and motion

The bar for an in-game correction is WP6's Red Dead Redemption 2 shadow reference: a continuously moving
edge attached to the ground, with readable terrain texture. Keep Neutral Highlights, the incident meter and
the low-light key. [Exposure decision](../../../.orchestrate/evidence/v1-workload/G3S-HANDOFF.md#exposure-audit-decision)

Before judging a correction, capture stock and candidate from the same cooked spur and starting cut/reset,
at 1920×1080 with TAAU on. Use T2 fixed submitted-frame sequences on Metal and Chrome for the
run-up and consecutive states through 223–231, with 226 and 227 explicitly joined to the archived
pose, Sun direction, simulation endpoints and alpha. Match jitter phase and the sequence of history
resets and prior submitted inputs in both arms. Use the archived 74–94 extinction sequence in the
same way. Retain raw visibility and shaded frames for the pair and adjoining frames. Require stock to show
the row-665 drop in both targets at 226→227 before comparing the candidate across x=480–1200, adjacent
rows and frames. The in-game verdict needs every fixed submission above and the realtime comparison below.
Keep T2 realtime at 1× for the 17.40–17.80 h crossing and the separate slow ±20° pan, and fixed for the
six held stops. Compare the Ground E pan separately. Review near, last and far ranges with open Sun, full occlusion, broad edge,
close terrain, distant crest, represented compact caster and overlap. Compare the cooked swap to the
fresh replacement. The correction belongs in the shared cooked and synthetic surface evaluator, with
aerial evaluation separate. Compare the first submitted image after discard and device replacement
byte for byte with a fresh render. Record paired capture-off whole-frame and shadow timings at the
same extent and settings, with run order, load and any tripped wire.

#### Exact baselines

WP3 runs its diagnostic from the working folder. The registered
`apple-tile-native`, `chrome-day-sdr`, `chrome-day-scene-linear`, `chrome-night-sdr` and
`chrome-night-scene-linear` sets launch the production valley through the shared surface evaluator.
The WP3 diagnostic is a separate CPU invocation in the working folder with no runtime call site. The
existing scheduled verifier continues to guard the registered bytes. If a later proposal changes default
surface light, its design review covers all five sets and obtains a ruling with `APPLE-TILE` naming which
sets move and who refreshes them before that work is built.
[Registry](../../../.config/exact-baselines.json) · [Exact baseline ownership](../ARCHITECTURE.md#exact-baselines)

### WP4 finite-disk shadow diagnostic

WP4 tests one source-side reuse candidate offline. Traverse source bounds against the bundle of Sun rays
swept across a receiver patch and phase interval. Reuse overlapping nodes for each receiver in that
domain, including undecided nodes. For each receiver project the admitted terrain faces, the near
complement and the detached caster along rays onto the plane one unit toward the Sun. The projected disk
has radius `tan(0.266°)`. Clip to the disk and union intervals along each solar chord before integrating.
Classify terrain as far only if every vertex is at least 64 m forward along −X from the patch's minimum X.
Keep each other triangle whole in the near complement. Compare with independent direct ray intersections
on the original triangles. The prior frozen packets agreed at finite samples while three conservative
omission proofs left moving receivers unresolved. The project bar makes this a diagnostic with no
continuous-accuracy or runtime claim.
[Historical cases](../../../.orchestrate/evidence/ATMOSPHERE/WP1-M4-records.md#m4-exact-finite-disk-shadow-reference-match-deferred-bounded-spike)

Use a versioned procedural reconstruction of the original case types. This supplies triangles, receivers,
phases and detached casters without depending on the lost spur or packet caches. In metres, use +Y up,
the Sun looking toward −X from the receiver, Sun radius 0.266°, zero azimuth and centre elevations
0.001, 0.002 and 0.003 radians. For a ridge at distance `d`, put its crest at `x=−d` and its two feet at
`x=−d±1`, with foot height −2. Extend the profile from negative to positive half-width in Z, and split
each of the two resulting quads into triangles. Use the following source cases.

| Case | Source |
|---|---|
| Open Sun | No blocker triangles |
| Broad edge | Ridge at 256 m, crest height 0.512 m, half-width 8 m |
| Distant narrow crest | Ridge at 1,024 m, crest height 3.072 m, half-width 1 m, with the ridge removed as its ablation |
| Close umbra | Ridge at 66.5 m, crest height 1 m, half-width 8 m. On this patch the rising face straddles the plane at X=−66 m. |
| Compact caster only | Triangulated 24 cm cube centred at (−160, 0.32, 0.35) |
| Overlap | Broad ridge and the same cube, considered together and in isolation |

Sample X and Z offsets −2, 0 and 2 on each 4 m receiver patch with Y=0, and also Y=0.15 for the raised
overlap patch. For the four moving receiver/phase pairs use the narrow crest with X=Y=0. Start at
Z=−1.5, −0.5, 0.5 and 1.5 m and elevation 0.001 rad. In each pair move Z by +0.5 m and elevation
to 0.002 rad. At the close centre receiver count coverage from the faces straddling the patch's
64 m plane alone as a control. Retain the fixture recipe, Sun phases and receiver positions with the
rerun command. Report the reconstructed cases under their own identities.
[Independent ray probe](../../../.orchestrate/evidence/ATMOSPHERE/shadow-final-design-audit/README.md)

Fix sample density and the candidate's work limits before running. Use the historical ceiling of 20 million
work units per case as an upper cap for the candidate. Charge a unit for each bound test, triangle visit,
chord intersection, interval comparison or merge, and coverage sample. Include preparation, receiver
queries and detached casters. Stop at the cap and record unanswered queries and spent work. Measure
both arms on the same projected disk. At common sample locations, count reference-only hits as omission
and candidate-only hits as overcoverage. Refine the
independent ray grid at the centre of each case to document convergence. The record gives the coverage,
both kinds of error, the spent work and a retain-or-park decision for the single candidate.

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
[Apple tile M0.4](../APPLE-TILE/PLAN.md#wp0-a-repeatable-offscreen-valley-proves-the-comparison-path-done).
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

WP5. Other engine modules read the one sky and Sun instead of bringing their own, and one cloud optical depth
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
lent sky, aerial or reflection value is resolved to one stored step and not to float32 precision. A step is under
0.1% of a value in the format's normal range and a larger share of the smallest values it holds. The stored
texels are already rounded to that step.

| Operation | Returns | Support, and what happens outside it |
|---|---|---|
| Sun light at a world position | RGB illuminance on a plane facing the Sun, from the true Sun through the air only; no geometric shadow, no cloud factor | Any world position. Above the 100 km top the light is unattenuated unless the ray enters the shell. Below the first chart row (6.15 m) the altitude clamps, a known approximation that reaches 1.2% in blue at sea level with the Sun on the horizon. A Sun behind the planet at that point returns exactly zero |
| Sky radiance along a direction | RGB radiance of the sky without the Sun's disk: the solar part and the night part, already summed | The active camera's observer altitude only; any direction, including the ground side below the horizon. The night part exists for 0–15 km observer altitude; outside that the result is Sun-only and the fallback is reported, as today |
| Incident diffuse on a normal | RGB irradiance from the sky, never negative | One answer per frame at the camera's location, for any normal. It does not vary with world position |
| Aerial `(T,S)` to a point | RGB transmittance and in-scattered radiance, night included, accumulated **from the active camera** to a point in its view | The active view only, addressed by the point's screen position and its distance from the camera. Beyond 32 km it returns the 32 km value. From WP9 the air goes on under the model's ground, so a point on low ground or just above the horizon takes the haze of its own line ([The air at the ground](#the-air-at-the-ground)). A segment that starts anywhere but the camera, and any second camera, is not offered in WP5; a consumer that needs one proposes it through [the module list](../ARCHITECTURE.md#interfaces) |
| Environment reflection | RGB radiance of the disk-free sky filtered to a GGX lobe around a direction at a roughness, solar and night parts decoded and summed; and the split-sum pair `(scale, bias)` for a view cosine and the same roughness | One cube per frame at the camera's location, no parallax. Roughness is the material roughness the forward pass uses, clamped to 0.045–1. View cosine 0–1. The library maps roughness to its filtered level, and between two levels returns their linear blend, so the radiance does not step as roughness changes; a contribution never names a mip |

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
are exactly today's. WP5 does not move the volume onto per-sample air transmittance; that would change every
scene with a volume and nobody asked for it.

**Where the factor is exactly 1.** A receiver at or above the plane, a read that falls outside the validity
region, a frame the provider marks invalid, and a true Sun below the provider's lowest valid elevation all get no
cloud shade: the factor is exactly 1, never a clamped edge value. The read moves further from the receiver as
the Sun lowers and has no answer at a horizontal Sun, which is why the provider declares a lowest elevation. The
provider owns continuity at these limits, by bringing `tau` to zero toward the region's edge and toward its
lowest elevation. With the Sun behind the planet the direct term is already zero. WP5 supports one plane.

**Absent means absent.** Provider presence is a scene-build fact. Without a provider the forward shader and the
cooked-volume shader are composed without the lookup, as they are today, so there is no added texture, binding or
sample. With one, the shader variant adds one sampled texture to the light group.

**A diagnostic provider** in `examples/v1-game`, behind an opt-in control and labelled as a stand-in, supplies a
moving analytic `tau` pattern with a declared drift, plane, region and lowest valid Sun elevation; that elevation is
below the low-Sun views WP5 captures. It exists to prove the path; `CLOUDS` replaces it.

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
  to the same half-float step, so the product would be no more accurate at a texel; between texels it would be
  smoother by less than a step. Whether a step shows is WP6's to judge by eye in the sky and at twilight, and a
  visible one there reopens this.
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
Each compares one product with a reference inside a stated scope. WP5 left their references, cases and bounds
unchanged and passing, and they are the whole of what was claimed for product accuracy. From WP6 they are
regression guards, and [Closing G3s](#closing-g3s) says how a deliberate change to the look gives one new
expected values.

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
independent transport reference, apart from the one row of P11. That was true before WP5, WP5 and WP6 do not change
it, and WP6 writes it in the maturity ledger. Closing it is a separate accuracy campaign, with its own plan and
cost.

**The handoff, new in WP5.** For each lent operation, a contribution with its own pipeline, using only the lent
layout and library, must return the stored product in the right units. This runs on Metal in `./dev check` and
in Chrome on hardware WebGPU. The expected values come from the same references on both; Chrome is never judged
against Metal's GPU output. That is how WP5 proved the rows. From WP6 the comparisons in them are of three kinds:
the contract, the guards made through the library, and in H5 the records. [Closing G3s](#closing-g3s) says
which is which and what a deliberate change to the look does to each.

| | Lent operation | What the contribution-side result must meet |
|---|---|---|
| H1 | Sun light at a world position | The cases and bounds of P1 and P2, against their f64 references |
| H2 | Sky radiance along a direction | Night part: the cases and bound of P3. Solar part: the filtered-decode rule below, on the live sky-view chart at the 11 view directions and six Sun elevations of the all-orders grid, and on a known field. Its three wrong lookups fail. Plus the P11 row at its bound |
| H3 | Incident diffuse on a normal | The cases and bounds of P6; with nightglow on, the added horizontal irradiance within P4's bound |
| H4 | Aerial `(T,S)` to a point | Night part: the cases and bound of P5. Solar part and transmittance: P10's fixed-texel decode at its bound, and the filtered-decode rule below, on the live volume at P10's Sun elevations and on a known field. Its three wrong lookups fail |
| H5 | Environment reflection | In WP5: the cases and bounds of P7, P8 and P9 through the lent library, with a known night term stored in the fixture and required back in absolute units, unchanged by the raw Sun's illuminance, within P7's rounding allowance. From WP6 the radiance meets the stored-cube probe below and the split-sum pair the stored-table rule below, and the wrong lookups of both fail. P9's cases, the night term and P7's agreement across the cube's edges and corners stay as they were. The differences from P7's closed forms and from P8's quadrature are recorded and decide nothing |

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

**The stored-table rule, for H5's split-sum pair, from WP6.** P8 made through the library asks whether the pair
agrees with a quadrature of the reflection model. That is a question about the producer. A table made by a
cheaper estimator fails it while the consumer returns every texel exactly: at view cosine 0.5 and roughness
0.5225 a 32-sample table stores a scale of 0.8105 where the quadrature gives 0.8198, 0.0093 apart against P8's
0.006 ([the arithmetic](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/split-sum-handoff.txt), part A). So the handoff
asks its own question of the pair: is it the one the atmosphere stored for the requested view cosine and
roughness?

- **Expected value.** The test reads back the table's stored texels, in the same run on the same target, and
  evaluates in f64 the operation's addressing and blend at the query: the view cosine over 0–1 along one axis,
  the roughness over 0.045–1 along the other, each range ending on a texel centre, and a bilinear blend of four
  texels. This evaluation is written in the test. It does not call, include or translate the lent library's
  code, and it integrates nothing.
- **Bound.** The filtered-decode bound, `C/32 + 1e-5 × M + Q` per channel, over the four texels the blend
  reads. The pair is also never negative and never sums over 1.001.
- **Cases.** The seven texel-centre nodes H5 already puts to P8, where a correct read returns one texel. One
  roughness below 0.045, which returns the table's first row. And queries a quarter, a half and three quarters
  of the way between texel centres on both axes at once, in the four cells that begin at view cosine and
  roughness (0.0625, 0.224), (0.125, 0.582), (0.75, 0.582) and (0.5625, 0.881): texels (4, 12), (8, 36),
  (48, 36) and (36, 56) of the 65 × 65 table. Every case runs at exposure scales of 1 and 2.
- **Wrong lookups that must fail.** The same harness runs three wrong lookups through the same bound, and each
  must fail at every query between texel centres: one row alone, one column alone, and the two public arguments
  exchanged, the view cosine passed as the roughness and the roughness as the view cosine, before the library
  floors and remaps the roughness. A run in which one of them passes is a failed run, on Metal and in Chrome.
  The nearest single texel is not on the list, because its errors on the two axes can cancel.
- **Why those cells.** At low roughness away from grazing the table is nearly flat along one axis, and no bound
  tells one of its rows from the next. Where the view cosine equals the roughness the exchange is the same
  query. In the four cells, on the table as the producer's equations give it, a correct read with 8-bit sampler
  weights and a half-float result uses at most 0.36 of the bound, one row or one column alone misses it by 3.2
  times or more, and the exchange by 171 times or more (parts B and C).

This rule proves that the pair is the stored one. Whether the stored table is a good split sum is P8's
question. The producer's own test keeps asking it on Metal, and through the library
[Closing G3s](#closing-g3s) makes it a record.

**The stored-cube probe, for H5's reflection radiance, from WP6.** WP5 proved the radiance against closed forms
of the lobe on a sky the fixture wrote: the sky itself at the sharpest level, and the lobe's mean at the
widest. That too is a question about the producer. The cube is a sampled estimate, and one made with fewer
samples leaves the closed form while the consumer reads every texel correctly: at the centre of a face of the
widest level, 16 samples put red 0.024 off and 8 samples 0.056, against the 0.020 the comparison allowed
([the arithmetic](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/split-sum-handoff.txt), part D). So the handoff asks
what it is. Does the contribution read the atmosphere's cube, at the face its direction names and the level
its roughness names, blending the two levels a roughness lies between, and decode it to physical units?

- **The probe.** After the producer has run, the fixture stores a known cube in the atmosphere's own cube
  texture, the one the lent layout binds, and no producer pass runs between the store and the queries. Every
  face of every level holds one colour, exact in half-float, that names it: with values between 1 and 4 at unit
  Sun, one channel steps by an eighth or more from face to face and another from level to level. The fourth
  channel holds a known night term, 1/16 in absolute units as in WP5's fixture.
- **Expected value.** The raw Sun's illuminance times the colour the fixture stored for that face and level,
  plus the night term. Between two levels it is the two colours the fixture stored for that face, blended
  linearly by the query's place between the levels. It comes from what the fixture wrote. It does not call the
  lent library, and it holds no closed form, quadrature or model.
- **Cases.** The six axis directions, each the centre of a face, with the raw Sun at 1, 0 and 2 and at exposure
  scales of 1 and 2, at thirteen roughnesses: the seven that name a level, and the six that name the midpoint
  of each pair of neighbouring levels. The library's mapping is linear, so level `l` of the seven is roughness
  `0.045 + 0.955 × l/6`, and level 2.5 is roughness 0.4429. Every query below the widest level sits 1/1024 of a
  level above the level or midpoint it names. A sampler that truncates its level fraction then still lands on
  the level, and one that keeps as few as four bits of it still weights a midpoint by a half. The widest level
  is held by the roughness clamp.
- **Bound.** The filtered-decode bound, `C/32 + 1e-5 × M + Q` per channel. At a level, `C` and `M` are taken
  over that face on the level and on the levels beside it. At a midpoint they are taken over that face on its
  two levels.
- **Wrong lookups that must fail.** The same harness runs wrong lookups through the same bound, and each must
  fail at every query it is run at with the raw Sun on. At every query: the direction negated, and the
  direction's axes rotated one place. At the queries on a level: the roughness of a neighbouring level. At the
  midpoints: the roughness of the lower level, and the roughness of the upper level, each compared with the
  midpoint's expected blend. A run in which one of them passes is a failed run, on Metal and in Chrome. With
  the raw Sun at 0 every query returns the night term alone.
- **Why the midpoints.** A library that rounded, floored or raised its level to a whole one before sampling
  returns the stored colour at every query on a level, and the forward pass's own lookup would hide it in every
  picture WP6 judges. Between levels it returns one level alone. At level 2.5 the example colours give a green
  of 2.375 with the night term, and either level alone gives 2.3125 or 2.4375.
- **What it tells apart.** With the example colours of parts E and F, a correct read whose level weight is 1/64
  off uses 0.33 of the bound at a level and at a midpoint. A neighbouring level misses it by 12 times or more,
  either level alone at a midpoint by 10 times or more, and another face by 118 times or more. Nothing else the
  layout binds holds those colours, so a contribution bound to another texture returns none of them. A raw Sun
  or exposure scale applied wrongly fails the cases that vary it, and a night term scaled by the Sun fails with
  the Sun at 0.

The probe proves which texture, face and levels were read, that two levels are blended, and the units. Whether
the cube is a good filter of the sky is the producer's question: P7 keeps guarding it on Metal, and WP6's
inspection list judges it in the picture.

**Exposure control.** At least one case of each row H1–H5 whose expected value is larger than its bound is
repeated with the lent exposure scale set to 2, and meets the same bound against the same expected value. A
library that applied the scale would return twice the value, an error larger than the bound, and fail it.

H5 varies the face, the level, the raw Sun's illuminance, the night term and a rewritten sky, so a constant,
stale, wrongly scaled or wrongly addressed reflection fails.

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
| The lent reflection reads one level where the cube blends two | An independent surface's reflected sky jumps as its roughness crosses a level, while the terrain's does not | Probing only roughnesses that name a level |
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
  camera. Neither is offered in WP5; `CLOUDS` or `SEA` proposes it the same way.

## The air at the ground

WP9, added on 14 November 2026. It is built before WP6 and waits for nothing: the sea and sunsets are seen
from sea level, and WP6 waits for TAAU.

**The request**, the lead's task of 14 November 2026, acting on
[`SCENE-VIEWS`' proposal](LOG.md): "the aerial pass stops a descending column at the model's ground, so
observers near sea level read thin haze just above the horizon, and mirror views inherit it. Sea-level scenes
(the sea, sunsets) are a showcase priority. Fix it in ATMOSPHERE's design and plan with the modern technique,
keeping SCENE-VIEWS, WATERS and CLOUDS consistent, and say where exact baselines move."

### Cue

| Cue | Where it must hold | Plausible wrong substitute, and what rejects it |
|---|---|---|
| G1 Haze deepens evenly to the horizon. Water and low ground take the haze their distance gives, and a hull, an island or a far shore just above the horizon stands in the same haze as the water under it, toward the Sun and away from it | Observers from the model's ground to 15 km; points to 32 km; any camera tilt; a Sun from high to setting; the main view, a mirror view and the view volume | Haze added by depth near the horizon, or a rule in the decoder for rows under the ground. Rejected by the read at a point against the product's own column along that point's line, at every tilt |

The bar is the sea's horizon at sunset in Sea of Thieves and the coast in Red Dead Redemption 2, in words.

### What is wrong today

The aerial volume holds, for each of 32 by 32 lines of sight, the transport from the camera to 33 distances.
The pass ends a line that descends where it meets the model's ground, the sphere at sea level, and every
layer beyond holds the value at that end. No drawn point lies on such a line past its end. But a read blends
the lines round a point, and round a point on low ground, or just above the horizon, one of them has ended
short of the point's distance. The point then reads less haze than its own line holds.

[Part IV](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/README.md) runs the pass's quadrature in f64 in a stand-in
air. In a 60° view, with points 1 to 30 km away:

| Observer above the model's ground | A point on a line 0.05° to 1° above the horizon | A point on the ground |
|---|---|---|
| 0 m (a mirror view of the sea) | up to 92% of its in-scatter missing | |
| 3 m | up to 89% | up to 72% |
| 40 m | up to 67% | up to 51% |
| 245 m (the showcase valley) | up to 35% | up to 39% |
| 1.8 km | none | up to 12% |

The loss comes and goes as the horizon falls on a row of the volume or between two. For the sea 3 km away
from 40 m it runs from 5% to 31% and back within one row of camera tilt, which on level water is a band
across the picture that moves with the camera.

### Research

Aerial perspective in current engines is a small volume aligned to the camera, as here
([Hillaire 2020](https://diglib7.eg.org/bitstream/handle/10.1111/cgf14050/v39i4pp013-022.pdf); Unreal Engine's default path;
Bevy's `aerial_view_lut`). Three treatments of the ground are published.

- **Hillaire's reference implementation**
  ([`RenderSkyRayMarching.hlsl`](https://github.com/sebh/UnrealEngineSkyAtmosphere/blob/master/Resources/RenderSkyRayMarching.hlsl),
  MIT, read for this revision) integrates each cell by itself. A cell whose far end is under the ground has
  that end lifted onto the ground, and the cell is integrated along the straight line from the camera to the
  lifted point.
- **Unreal Engine's** volume is the same author's. In 2021 he described what remains in it, "the low resolution
  aerial perspective texture can show up when you are right-on or below the planet virtual ground", and the
  remedy he thought best, "set the fog amount of voxel below the surface to the fog amount of their projected on the
  planet ground level" ([forum](https://forums.unrealengine.com/t/sky-atmosphere-fog-issue/157202)). The
  workaround he gave is to move the planet down. Unreal also offers aerial perspective marched for every pixel
  as a costlier option.
- **Bevy** (`aerial_view_lut.wgsl`, release 0.17, read for this revision) marches each line through the
  ground with no test for it. Its density follows the exponential law below zero altitude, and its Sun's light
  comes from a chart that ends at the ground.

Part IV.2 runs each rule that has a figure below on the same points:

| Rule for a line under the ground | In-scatter error in clear air, 60° view | Verdict |
|---|---|---|
| Stop at the ground (today) | 35% to 92% low | The defect |
| Move the model's ground below the scene | | Every altitude in the scene is then wrong by the offset: thinner haze everywhere and a later sunset |
| Hold: carry on, with the air and the Sun's light of the ground point above | Within 1.5% away from the Sun; 3% to 6% low toward a Sun 0.5° to 10° up | The remedy Unreal's author names. It leaves a kink at the ground in a function the read blends linearly, and it is worst in a sunset |
| Re-aim: Hillaire's reference | 1% to 4% low, and 8% to 24% high toward a low Sun | The lifted line is at another angle to the Sun and another length, and each layer is marched again from the camera |
| Dense: the density law carried under the ground, the Sun's light of the ground point above (Bevy's, in effect) | Up to 1% high for a high Sun or one behind the view; 1.5% to 6% high toward or across a low Sun | Denser air lit as if it were at the surface |
| **Continue: the air goes on under the ground, its density by its own law and the Sun's light dimmed by that air** | **Within 0.6% for a Sun 2° up and higher, within 1.7% at 0.5°** | **Selected.** The line's values are then smooth across the ground, which is what a linear blend needs |
| Weight the blend by which lines are above the ground, in the decoder | | Four loads in place of one filtered read in every reader, a second decoder for a mirror view, and nothing for a row pair that is wholly under the ground |
| March the air for every pixel (Unreal's option) | | Water, clouds and a mirror view read the lent volume at points that are not opaque pixels; it would be a second product beside the volume |

### Mechanism

**The air goes on under the model's ground.** For the aerial product the ground is not an end. A line of
sight keeps its direction and its layers at their own distances to 32 km or the top of the air, whichever
comes first. At a sample under the ground:

- the gases and the aerosol have the densities their own laws give at that depth, so the air is denser than
  at the surface, as the air of a basin below sea level is;
- the Sun's light is the light at the ground point above the sample, dimmed by the air between along the
  Sun's ray. The charts end at the ground, so the pass takes that dimming from how the charts' light changes
  with height just above the ground, or integrates it. Part IV.2's `slope` is the first: it stays within 0.4
  percentage points of the exact continuation. Which one is the builder's; [the acceptance](#numerical-acceptance-for-the-air-at-the-ground)
  decides;
- a Sun the ground hides from the ground point above stays hidden;
- the multiple-scattering source and the night source are those of the ground point above, in the denser air.
  Both are diffuse and small, and neither has the Sun's steep change with height.

Transmittance, solar in-scatter, the single-scattered luminance the shadowed decoder splits off, and the night
term all go on this way. Arithmetic far under the ground, where no read reaches, is the builder's to keep
finite: a read blends lines no more than one row apart, so the deepest sample any read takes is 32 km times
the row spacing down, 2 km at a 90° field of view, where the aerosol is 5.3 times its surface density.

**Nothing above the ground changes.** A line that never meets the ground, and every layer of a descending
line that ends before the line meets it, holds the bytes it holds today. The decoders, the lent operation's
arguments and units, the sky-view chart and the lent Sun light are untouched. The sky's own ground side still
ends at the ground, because that is where a line of sight into the planet ends.

**Every volume the pass makes.** There is one aerial pass, so the rule reaches each of its volumes with no
second copy:

- the main view's volume, in every scene, with no selection. A scene cannot ask for the old stop;
- a secondary view's volume, which [`SCENE-VIEWS`](../SCENE-VIEWS/DESIGN.md#what-a-views-air-is-held-to)
  defines as this pass's own for an observer at the mirror plane's height. A mirror of the sea stands on the
  model's ground, where every descending line is under it from the start;
- [the view volume](#the-view-volume-haze-fog-and-light-near-the-camera), whose cells take the same rule
  for the air's terms in WP7.

**A drawn point under the model's ground**, such as land below sea level, takes the air of its whole line,
the part under the ground included. `WATERS` removes the forward pass's air from a sea bed with the same
lookup that applied it ([its optics](../WATERS/DESIGN.md#optics)), so it is unaffected by what the volume
holds there.

### Check that it can work

The pass already takes a sample's altitude and its direction to the Sun from the sample's own position, and
its density law is a pair of exponentials and a tent that is zero below 10 km, so the law is defined at
negative altitude as written. The Sun's light is one more read of a chart the pass already samples. Part IV,
in f64, in single scattering, with the engine's molecules and aerosol in one band at a time:

- **IV.2, by Sun.** `Continue` is within 0.6% of a fine quadrature of the point's own line for Suns 2° up and
  higher, across observers 0 to 245 m, lines 0.05° to 1° above the horizon, points on the ground and points
  1 to 30 km away, at every camera tilt. `Hold` is 3.4% to 6.2% low toward a low Sun, where most of the
  in-scatter is the aerosol's forward peak and the aerosol's scale height is close to the height one row
  spans at 30 km.
- **IV.3, by field of view.** As a share of [the view volume's yardstick](#how-the-view-volume-is-proved):
  at most 0.08 at 30° to 60° and 0.28 at 90° for Suns 2° up and higher; 0.31 and 0.93 with the Sun 0.5° up
  dead ahead. The figures against the product's own column along the line are the same to two places: what
  remains is the blend of rows, and no part of it is the quadrature along the line.
- **IV.4, in three bands.** Against the allowance below, `continue` is at most 0.35 in red, green and blue.
  `Hold` misses it in transmittance in every case, by 1.2 to 2 times. `Dense` misses it in in-scatter with
  the Sun 2° up, by 1.2 to 2.7 times. The stop misses by 33 times and more.
- **IV.5, the view volume's rows.** On 90 rows the stop is still 78% low from 3 m, and `continue` is within
  0.1%.

Unrun: the engine's spectral integrator, its multiple scattering and its charts' own interpolation near the
ground, which WP9.1 sees first; and the look, which `WATERS` WP2 judges on a sea.

### Numerical acceptance for the air at the ground

WP9.1 compares a read with **the product's own column along the point's line**: the stored column of a texel
whose centre lies on that line, in a volume made for a camera turned so that one does. The reference is made
by the same pass, charts and layers as the read, and takes no blend of rows. So the comparison measures what
the rule decides and nothing else, and it qualifies nothing about how far the stored transport is from true
transport, which stays [unqualified](#numerical-acceptance).

- **The read.** The f64 evaluation of the lent operation on the volume read back, as
  [the filtered-decode rule](#numerical-acceptance) defines it, at a point that lies on a column of the
  volume and between two rows. H4 already holds the sampler to that evaluation, and a sampler's blend weights
  are too coarse to carry this comparison.
- **The cases.** A 60° field of view. Observers 0, 3, 50 and 245 m above the model's ground, with points 3,
  10 and 30 km away on lines 0.05° and 0.2° above the horizon. Points on the model's ground 3 km from an
  observer at 3 m, 3 and 10 km from one at 40 m, and 3, 10 and 30 km from one at 100 m. Each at eight camera
  pitches that walk the horizon across one row, and with the Sun 45° up and 60° across the view, 10° up
  behind it, 5° up and 20° off the view's heading, and 2° up and 10° off it.
- **The allowance decides.** Per channel: transmittance within `0.01 × expected + 0.001`; solar in-scatter
  within `0.025 × expected + 0.0025 × F`, where `F` is the reference column's in-scatter at 32 km; each plus
  `Q`, one stored step. The night term within 10% of the reference's, which is P5's bound. Part IV.4 puts the
  selected rule at 0.35 of this or less, and each wrong rule over it.
- **The stop must fail.** From the same volumes, the test evaluates each read again with every column held
  at its value where it met the ground. For the points 30 km away above the horizon and the ground points
  10 km away, that read misses the allowance by more than five times at one pitch or more; part IV.4's least
  is 11 times.
- **The record.** The worst share of the allowance by Sun and channel, and beside it the same reads with the
  Sun 0.5° up dead ahead and at a 90° field of view. The record decides nothing.
- **A miss is a plan change.** If a pass the builder believes follows the mechanism misses the allowance,
  the builder records where and by how much and ends `BLOCKED`. The allowance is not widened in the build.

WP9.2 pins what does not move: every texel of a line that never meets the ground, and every layer that ends
before its line meets the ground, holds the bytes the commit before WP9 stored, for the same frames.

Past the ground WP9.2 holds the stored values to what half-float storage can show. Every value is finite. A
layer's stored transmittance lies between 0 and 1 and is never higher than the layer's before it. It may be
equal: a steep line's transmittance falls below the smallest half-float within some 20 km, and two layers
that are both still falling round to the same step, zero included, so a strict fall from layer to layer
would refuse a correct column. That the air does go on is shown where storage can show it: on a line with two
or more layers past the ground, the last layer stores less than the first one past the ground. Over every
texel of the volume, from 3 m and 245 m, level and pitched down to straight down, at 60° and 90° fields of
view, that fall is 30 stored half-float steps or more, and a column held at its value at the ground fails it
([the arithmetic](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/m9-storage-rule.txt), which estimated 40, and
[the exact count](../../../.orchestrate/evidence/ATMOSPHERE/design-review-9/storage-revision-check.txt)). WP9.1 is what proves the
continuation is the right one.

P10's fixture, a camera at 245 m with rows 1.1° below the level, already holds lines that meet the ground
inside 32 km. It compares storage with a float32 run of the same integrator, so it passes unchanged and now
covers the air under the ground. No preserved referee's reference, case or bound changes.

### Exact baselines

WP9 is meant to change every scene's default output, so [rule 1](../ARCHITECTURE.md#exact-baselines) applies
as a planned move. [A reading of the valley's mesh](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/README.md) predicts
which pixels: terrain whose altitude is less than about 3% of its distance, which from the valley's cameras is
the low ground 3 to 12 km out. That is up to 0.4% of the covered pixels at the moving route's checkpoints, 4% to
5% in the sunset still and 0.1% to 0.2% in the night still. Those pixels gain haze; nothing else is predicted
to move. All five registered sets hold such pixels, so all five are expected to move, the scene-linear ones
with certainty. [The module list's ruling](../ARCHITECTURE.md#exact-baselines) says who refreshes them and
how; WP9.5 and WP9.6 carry it.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The engine's read misses the allowance where the stand-in passes | WP9.1's first run | Widening the allowance, or dropping the low Suns |
| Only the density is continued, with the Sun's light of the surface | In-scatter high toward a Sun 2° up; WP9.1 misses there | Tuning the density to hide it |
| A refresh hides an unrelated change | A reference moves at pixels the volume cannot reach: sky, near ground, a prop | Refreshing without WP9.6's images |
| Values far under the ground overflow or go to NaN | A non-finite texel in the volume read back | Clamping the stored value after the fact |
| Another builder is in `aerial.wgsl` (`SCENE-VIEWS` WP1, `CLOUDS`' rulings) | A merge that restores the stop | A second copy of the pass |

### Cost

A descending line now runs all 32 layers, where it used to skip those past the ground: at most the work the
upward lines already do, with one more chart read for each sample under the ground. The volume is 1,024
lines of 128 steps. WP9.8 records the aerial pass's GPU time paired before and after on the valley, on the Mac
and in Chrome, with tripwires and no budget ([brief](../VISION.md), decision 3).

## Closing G3s

WP6. The gate closes when the valley's sky and sunlight pass a clean-context review in motion on Mac and Chrome.
It runs after TAAU is in the game (`APPLE-PRESENTATION:WP3`), because reconstruction changes every edge and
gradient a reviewer judges.

**What is reviewed.** The fixed six-stop route of [the route decision](../../design/decisions/G7-SHOWCASE-ROUTE.md) and one
continuous scrub from dawn through noon, sunset and twilight to night, with slow pans, captured from the running
game with the engine tap. The inspection list is the set of defects earlier reviews recorded: a horizontal
brightness step across the ground; a flat near ridge with a hard bright rim at low Sun; a ridge lit warm when the
Sun is behind it; missing object shadows at sunrise; shadow acne at low Sun; a hard horizon line; and banding or
steps in the sky gradient and twilight colour. One entry is new, because the look of the reflection cube is
judged here and by no number: sky light reflected off a surface in blotches, separate spots or with a seam.
The bar is the time of day in Red Dead Redemption 2 and Horizon
Forbidden West, in words.

**The valley is `SHOWCASE`'s.** WP6's builder found two entries whose cause is the authored valley and not the
sky or the Sun ([its handoff](../../../.orchestrate/evidence/ATMOSPHERE/WP6-handoff.md)). The bright band along the lookout's rim
at the held 17.4 h stop comes from a terrain mesh too coarse there for the normals it stores. At the 6.1 h
sunrise stop the east ridge still hides the Sun from the lookout, so no prop can cast a shadow. By
[the module list's ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix) `SHOWCASE` WP9 resolves the rim in the
mesh and moves the sunrise stop to the first hour the Sun reaches the props
([its design](../SHOWCASE/DESIGN.md#the-lookouts-rim-and-the-sunrise-stop)). WP6 waits for WP9 and judges the
whole list on the changed valley, on both targets and in motion. It does not change the valley's source: a
defect it traces to the terrain is a proposal in the log.

Two entries need their meaning said, so that a correct picture passes. A brightness step is a band on ground
that is one piece. Where the lookout's ground ends and the valley floor lies 2 km beyond, a difference is a
depth edge, and it passes when it reads as the lookout's edge. A ridge lit warm with the Sun behind it is
direct sunlight on a face the terrain hides from the Sun. A crest that still sees the Sun is lit, and the
glow of the air between the camera and a ridge that stands toward the Sun is the aerial product at work.
Neither is the defect. For the crest that stays warm at the held 17.9 h stop, the builder's rays through the
terrain say which it is, and the evidence records it.

**What guards the products.** The referees P1–P11 and the handoff rows H1–H5 of
[the numerical acceptance](#numerical-acceptance) run in the workspace check on Metal, as they have since WP5,
and WP6.4 runs the handoff in Chrome. WP6.7 adds the stored-table rule and WP6.8 the stored-cube probe, on both
targets. Their comparisons are of three kinds, and WP6 treats each differently.

- **The contract: what a consumer receives.** These take their expected value from the atmosphere's own stored
  product, read back in the same run on the same target, or from a known field the fixture wrote. None takes it
  from the model of the air or of the night sky, and none from a quadrature of the reflection model. They are
  the filtered-decode rule's live arm, known-field arm and wrong lookups (H2, H4); the fixed-texel decode (H4);
  the diffuse cases against the integral of the read-back sky (H3); the exact zero with the Sun behind the
  planet (H1); and in H5 the stored-cube probe and the split-sum pair's stored-table rule with their wrong
  lookups, the known night term, the agreement across the cube's edges and corners, and the furnace and
  source-scaling cases. They prove the sampling, the addressing and the units. A deliberate change to the sky,
  the air, the cube's filter or the split-sum table moves both sides of each together, or neither side where
  the fixture wrote the field, so none needs an edit to keep passing. WP6 adds the stored-table rule and the
  probe and changes none of the other cases, bounds or wrong lookups. One that fails is a broken handoff,
  whatever the picture looks like.
- **Guards: where a product stood.** P1–P11 are regression guards: they catch a product that moved when nobody
  meant it to. Five handoff comparisons are a P row made again through the lent library, with that row's
  reference, cases and bound, and are guards too: H1 against P1 and P2, H2's night part against P3, H2's
  spectral row against P11, H3's night illuminance against P4, and H4's night part against P5. A guard that
  fails asks one question, whether the change was meant. Its difference from the model is a diagnostic and
  never a finding.
- **Records: H5 against the reflection model.** The cube and the split-sum table are sampled estimates, and
  their sample counts are a cost a builder may lower. A comparison with the reflection model made through the
  library then measures the estimator and not the handoff, as the two rules above show. So the H5 run still
  makes both, on the producer's own cube and table at the cases WP5 gave them: the radiance against P7's closed
  forms on the sky the fixture writes, and the pair against P8's quadrature. It writes the largest difference
  of each, for each channel, into the Chrome report and the Metal test's output, and passes or fails on
  neither. They are there to find the cause when a reflection looks wrong.

Every row, contract or guard, keeps [its repeat at an exposure scale of 2](#numerical-acceptance), which is
what shows that the library does not apply the scale.

**A deliberate change to the look.** A fix the inspection list needs may move a product on purpose: a twilight
colour, the gradient at the horizon, the light on a ridge. Every guard it moves past its bound then fails, on
Metal and in Chrome, while the consumer still returns the product exactly. In the same landing the builder
gives each such guard new expected values and keeps its cases, its bound and its exposure repeat:

- where the change is to an input of the model, such as a coefficient or a profile, from the row's f64
  reference run with the changed input;
- where the change departs from the model, such as a tuned curve or a tint, from the changed product on the
  producer's side: its stored values on the Mac, read back and decoded by the test's own evaluation.

An expected value never comes from the lent library's output, so a guard made through the library keeps failing
a consumer that reads the wrong resource or applies the exposure scale, as it does today. The Metal test and the
Chrome referee take the same new values. A guard made through the library in Chrome may therefore rest on a
value the Mac's producer stored: that is a guard at the row's bound and no claim that the two targets match, and
every contract comparison in Chrome still reads Chrome's own product. The log names each guard that changed, the
change to the look that moved it, and its old and new values. No review asks for a model that justifies the new
values. A change that moves a product also moves `APPLE-TILE`'s registered exact sets.
[The ruling](../ARCHITECTURE.md#ruling-on-the-valley-fix) lets WP6's builder refresh the sets that moved, in the
landing that makes the change, and WP6.9 holds the verifier and the refresh's record. So a twilight adjustment
that moves P11's blue channel by 4% lands with P11's three recorded values replaced on both targets, and H2's
live arm passes untouched because it reads the new chart. In the same way, a split-sum table or a cube filter
made with fewer samples lands with the producer's own guard, P8 or P7 on Metal, given expected values from the
product the Mac stores. H5 needs no edit: the stored-table rule and the probe read what is stored, and its
differences from the model are only recorded.

The solar sky and solar aerial transport are not qualified against an all-orders reference, and under the
owner's bar no such campaign is planned.

**Cost.** Whole-frame and atmosphere pass costs on the route are measured on the final build and, in the same
session and configuration (extent, TAAU on, load class, order alternated), on the commit WP6 started from. That
pair is the comparison. The WP1 and WP2 records predate TAAU and describe other builds and loads; they are quoted
as history and not called paired. The record is a tripwire and sets no budget
([brief](../VISION.md), decision 3). A visible hitch in the scrub, or a pass about 25% over its arm on the
starting commit, is investigated, and the finding is kept beside the record. WP6.5's check passes the record: it
fails on an arm, a named pass or the load that is missing, on a run that did not complete, on a record that
does not say whether a hitch was seen in the scrub, and on a tripped wire with no investigation, and on no
figure. WP8.9 records the paired cost of the fog bank. WP3.2 records the separate CPU shadow trial.

**Scope of the WP6 verdict.** WP6 records the west-lookout shadow crossing and finite-disk comparison as open.
WP3 and WP4 follow WP8 independently of G3s. WP6's visual exclusion covers the
moving shadow edge on the authored spur between 17.40 and 17.80 h. The held `sunset-clear-sun` stop at 17.4 h is
judged like every other stop, and the terrain lip recorded on the shipped Ground E pan is a separate finding,
judged under the brightness-step entry once `SHOWCASE` WP9 has fixed its mesh. Moon, weather and calibrated night
colour stay outside the profile, as the first section says.

**Risk.** Shadow defects have consumed days here before. If the review fails on one item twice, the builder stops
and reports it instead of opening a new shadow method; a new method is a re-plan with its own cost account.

## What the caster height map holds

WP10, added on 18 November 2026. `SHOWCASE` WP5 ended blocked on it: its builder's words were "Fix the Sun-shadow
discontinuity and the remaining frame stalls". The stalls are `SHOWCASE`'s to record; the shadow is this
module's, which owns the direct-Sun path to receivers.

### Cue

A balloon adrift over the valley shades the ground beneath it with a shadow of its own shape while it is near
the camera, and with nothing once the ground is beyond the Sun cascades. It never lays a hard dark bar across
the far ground, and no bar blinks on or off as it drifts. Ridges and mesas shade the far ground as they do
today. Range: the exit scene's 1,500 balloons, 8 to 23 m across and 65 to 295 m above the terrain, at every
hour the Sun is up, on the Mac and in Chrome.

### What is wrong today

On the Mac, on the exit flythrough, a dark stair-stepped strip appears across lit ground between committed
ticks 233 and 234 and stays. A fixed capture with every tick submitted reproduces it, so no missed frame
causes it ([the record](../../../.orchestrate/evidence/SHOWCASE/WP5-motion-diagnostics.md#reproduced-shadow-transition),
[the four ticks](../../../.orchestrate/evidence/SHOWCASE/WP5-motion-fixed-shadow-crop.png)). `SHOWCASE` WP5's reviewer had
already noted long ground shadows with "coarse stair steps and broad straight sides" at the shelf and slope
checkpoints.

The cause below is read from the code and the layout. No run has yet confirmed it, which is why WP10's builder
confirms it first.

- The Sun's three depth layers are drawn from one caster set (`synthetic/scene_graph.rs`, the passes
  `synthetic Sun shadow near`, `far` and `synthetic Sun horizon heights`). So the third, the top-down caster
  height map, holds the top of every balloon.
- Beyond the cascades' reach, which is 512 m past the near plane in a scene that sets no profile, as the exit
  scene does not, `horizon_visibility` in `synthetic.wgsl` marches that map toward the Sun as a height field:
  one height for each 32 m texel, read at the nearest texel and taken to be solid down to the ground. A
  balloon 200 m up is then a tower 32 m square and 200 m tall, and it shades a strip 32 m wide and
  200 m / tan(Sun elevation) long. The nearest-texel read is what cuts the strip's edge into stairs.
- The layer is rasterised at texel centres, which stay put in the world because the layer is laid out in
  whole texels. A balloon is narrower than a texel, so it is in the layer only while it covers a centre. Its
  drift of 0.4 to 3 cm a tick carries its edge across one, and the whole tower appears or goes in one tick.
  With each balloon taken as a disc, about 280 of the 1,500 cover a centre at a time, and one starts or stops
  about every second tick somewhere in the valley
  ([the arithmetic](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/horizon-columns.txt)). It happens with the camera
  held still too.

### Research

- A height field holds one height for each point of the ground, so it can stand for terrain and for what
  rises from it, and for nothing that has air beneath it. Height-field and horizon shadows are a terrain
  method wherever they are used
  ([Snyder and Nowrouzezahrai 2008](https://diglib.eg.org/handle/10.2312/CGF.v27i4pp1275-1283), abstract only).
- Engines that keep a cheaper shadow for long range choose its casters. Unreal's far shadow cascades draw
  only what is marked: landscape is in by default and a mesh joins through its own "Far Shadow" setting
  ([FShadowCascadeSettings](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/FShadowCascadeSettings),
  [forum, UE 5.3](https://forums.unrealengine.com/t/setting-far-shadow-property-in-c-on-static-mesh-actor-dynamically-spawned-at-runtime/1622509)).
  Small or flying things cast inside the dynamic shadow distance and not beyond it.

### Mechanism

**The caster height map is the terrain horizon: it holds the ground and what stands on it.** A scene marks a
mesh row of its entities as **aloft**. An aloft row is drawn into both Sun cascades exactly as now and is
never drawn into the height layer. A row is not aloft unless marked, so a scene that marks nothing records the
same three layers and draws the same bytes. The exit scene marks its balloon row.

So beyond the cascades an aloft caster casts no shadow. Inside them it casts as it does today, and its shadow
fades with the receiver's depth over the last tenth of the reach, as every cascade shadow already does.

| Option | Why not |
|---|---|
| Leave out whatever moved this tick | A paused balloon would grow its tower back, and a still thing that floats is as wrong as a moving one |
| Lengthen the cascades' reach until the far ground is inside them | Two 1024² maps over kilometres lose the contact shadows near the camera, and the tower is still there past the new reach |
| A second layer for the underside of floating casters | A new shadow method. By [this module's rule](#closing-g3s) that is a re-plan with its own cost account |
| Filter or soften the height read | The tower stays a tower with a soft edge, and still blinks |

What does not change:

- The fits. `caster_depth`, `caster_height` and the Sun sample uniform keep every caster, aloft or not: the
  cascades need them, and for the height march the larger bound only lets a march run a little longer. The
  shader does not change.
- Culling. An aloft row is left out of the height layer in the same way under Direct, Frustum and Occlusion,
  so [CULLING's equalities](../CULLING/DESIGN.md) hold.
- Readers of the layer. The air's shadow heights, a secondary view and `RAY-TRACING`'s traced method read the
  one layer the frame recorded, as before.

Limits, known and not built here. Only entity mesh rows can be marked. A contribution's own casters, terrain
and cooked placements, all enter the layer as today, so a placed thing that floats would still drop a tower,
a steady one. Nothing in the valley does. A balloon's shadow on ground past the reach is absent, as past any
engine's shadow distance; whether the exit scene wants a longer reach is `SHOWCASE`'s choice of profile, judged
on the finished image in its WP8.

### Check that it can work

The strip can only come from the height layer: it lies on receivers past the reach, where `sun_visibility`
returns `horizon_visibility` alone, and that function reads nothing but layer 2. With the balloon row out of
the layer, the layer holds terrain and the still props. Those do not move, and a camera move shifts the layer
by whole texels, so no texel changes from tick to tick and no far shadow can blink. The still props are a
few metres tall and the march already allows 4 m for a texel's slope, so they add little, and what they add
is steady.

**If the cause is another.** The builder first marks the balloon row and replays committed ticks 228 to 240
on the exit camera with every tick submitted. If the strip still appears, the reading above is wrong: the
builder writes what each layer holds at ticks 233 and 234 in the log and ends `BLOCKED`. It does not open
another shadow method.

### Exact baselines

No registered scene has a marked row, so no registered reference moves, and WP10.3 holds that. `SHOWCASE`'s own
exit baseline, coarse grids of its six stops, may move at a stop where a tower's strip stood; `SHOWCASE` WP5
replaces it by its own rule.

### Cost

A scene that marks a row draws that row's bucket in two Sun passes in place of three. A scene that marks
nothing pays nothing. No figure is asked for.

### How it is proved

- **WP10.1**, a test on Metal: with one caster marked aloft above the ground and one unmarked standing on it,
  the height layer read back equals that of the same scene without the marked caster, and both cascades equal
  those of the same scene with it unmarked, under each culling mode. The unmarked caster stands apart from the
  marked one and reaches higher than it. The fit keeps every caster, so a marked caster that was the highest
  thing in the scene would set the layer's depth range, and taking it out would move the stored depth of ground
  that is correctly kept. With the standing caster highest, both scenes fit the same range and the read-backs
  differ only by what the marked caster drew.
- **WP10.2**, a reviewer's inspection of the exit flythrough with every tick submitted: no strip under a
  balloon on the far ground, none appearing or vanishing, the near balloons' shadows and the ridges' far
  shadows still there.
- **WP10.3**, the exact-baseline verifier.

Chrome runs the same route code and shader. `SHOWCASE` WP5.9 looks at it there.

## The view volume: haze, fog and light near the camera

WP7 and WP8, added on 14 November 2026. They are track 2: they follow WP6, gate nothing in the V1 exit, and leave
every scene that does not ask for them as it is.

**The owner's request**, 14 November 2026: "Oh and for clouds we also need to be taking into account mist / fog.
We want a plan that's able to deliver on top quality atmosphere." The lead's task with it: "Revise the plans so
Thrum's atmosphere reaches the quality of shipped high-end games as a whole: sky, clouds, haze, and mist and fog
(ground fog, valley mist, fog banks over water, low cloud the camera can enter), lit consistently and changing
with the weather and time of day. Settle from first principles which module owns what across CLOUDS, ATMOSPHERE
and any other affected module, keep one authority for each kind of state".

### View-volume builder handoff

The aerial producer is `crates/thrum-render/src/atmosphere/aerial.wgsl` with `AERIAL_EXTENT` in `gpu.rs`; the aerosol's
constants are written into shader source by `spectrum.rs`; the forward decoders are the `synthetic_aerial*.wgsl`
files, one of which splits solar in-scatter by the surface's shadow, which the volume must not repeat. The
volume's history uses `EFFECT-GRAPH`'s state kinds, as `examples/v1-game/src/effect` does. The control and the scene's declaration go behind a selection in `examples/v1-game` that is off by
default; that game is `SHOWCASE`'s, so leave its generator, routes and exit configuration alone. `CLOUDS`
builds rulings 1 to 4 in these same files; integrate on top of what has landed, and if its haze scale is
present make sure the volume does not take it. Record WP7.1's base before changing anything.
[The look decides](#how-the-view-volume-is-proved): the f64 quadrature is there to find the cause of a
picture that looks wrong, and no item compares the volume with it. The sky is drawn from `atmosphere/sky.wgsl` and the stars in `synthetic/stars.rs`; both take the
terminal pair. `scripts/view_volume.py` is created here. The aerosol's kind makes its eight band values differ;
[the historical haze-kind check](../../../.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md) records the retired f64 probe with one Mie value per band. Part III says which grids band in thick, coloured haze. The builder chooses the grid from the stills.

WP8 extends the cloud-factor function in `crates/thrum-render/src/atmosphere/cloud.rs` and `cloud.wgsl`. Put the
test medium beside the labelled test cloud field in `examples/v1-game/src/host/scene_source/cloud.rs` until
`CLOUDS` WP14 replaces it. `CLOUDS` WP3 supplies the far aerial lookup and the [clouds design](../CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud)
defines the medium coefficients. If the ground band cannot fit a forward pipeline's WebGPU sampled-texture limit,
record the proposal in the log and end `BLOCKED`.

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
  244 km, grey in every band, with one phase. It becomes an input of two numbers, so the weather can thicken
  and thin the haze and set its kind.
- **The aerial product is clear air in a 32³ volume.** In a scene that declares the view volume it is the joint
  transport of air, haze and a provider's medium, at a finer grain near the camera. A scene that does not
  declare it keeps the 32³ product, bit for bit.
- **One plane for the Sun-path optical depth**, with a factor of exactly 1 at or above it
  ([WP5](#environment-identity-and-products), and its open question on a second height band). A provider may add
  a ground band, which a receiver inside it reads at its own position in place of the plane's value.
- **In-scatter under cloud is `CLOUDS`' scalar** (that section's first open question). Inside the view volume
  the air is lit sample by sample, through the scene's shadows and the cloud factor. The scalar stays for a
  scene without the view volume.
- **`CLOUDS`' builder builds what clouds need here** (the module list's rulings 1 to 4). Those stand. The view
  volume is larger and is judged by its own inspections, so it is this module's own work packages.
- **[RENDER-SPEC §5](../../design/RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator)'s joint froxel
  integrator** was a proposal with no owner or plan. This is it, for the Sun and the sky; its participating
  local lights are not built.

### Cues

| Cue | Where it must hold | Plausible wrong substitute, and what rejects it |
|---|---|---|
| V1 Haze: the land fades with distance into the sky's colour, over a range and in a kind the weather sets | Clear-air visual range 3 to 300 km; Ångström exponent 0 to 2; noon to twilight; Mac and Chrome | A tint by depth. Rejected by the sky and the farthest land meeting without a line at every range |
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

**The air's haze is an input, by amount and by kind.** The air profile takes two numbers for its aerosol at the
ground: its **amount**, as the extinction at 550 nm, and its **kind**, as an Ångström exponent `α` from 0 to 2.
In each of the eight bands the aerosol's extinction is `amount × (λ / 550 nm)^−α`, and its scattering is that
extinction times `3.996 / 4.4`, today's single-scattering albedo. The albedo, the phase and the height profile
are today's and do not change with either number. The
visual range is defined at 550 nm, so it depends on the amount alone. The defaults are today's values, about
244 km and `α = 0`, the grey aerosol the shaders hold now. The charts that depend on the aerosol
(transmittance, multiple scattering, the sky view, the night profile) follow a change of either number within a
lag the profile declares; at the defaults their bytes are today's. Changing either is a recomputation of
contents, like the Sun moving, and does not move the environment epoch. A weather provider supplies both;
without one a scene sets them.

What the kind does was measured before WP7 was built, with the module's own f64 reference sky in all orders of
scattering ([the haze-kind check](../../../.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md)). At a fixed visual range,
`α = 1.3` against 0 moves the sky's colour near the horizon by a CIEDE2000 difference of 4 to 6 in 20 km haze and 6 to
12 in 3 km with the Sun at 10°, and distant land by 5 to 7 in 3 km. With the Sun at 60° the sky from the
horizon to 20° up moves by 2 or less in 20 km haze and by 2.2 to 3.4 in 3 km, less than at the low Sun, and
the zenith by 3 to 5.
Grey haze at a low Sun is lit mostly by the blue sky and reads blue-grey; fine haze reddens the Sun's long path
through it and reads neutral to warm. So the kind is what turns a low Sun's murk from grey-blue to the warm haze of
dry, fine aerosol or smoke, which an amount alone cannot. It does not, in general, make dry haze bluer, as the
thesis review that raised it supposed. A type chosen from a list (continental, maritime, smoke) was rejected: it
steps where the weather moves smoothly, and it ties the albedo and the phase to the exponent without a measurement
that they show. A tint in three channels was rejected because the weather knows a particle size and not a colour,
and the air has eight bands. The albedo and the phase can join the same input later if a work package measures a need.

The kind thickens the air in the short bands: at `α = 1.3` the 429 nm band of a 3 km haze holds 1.38 times
the aerosol of the 550 nm one. On the grid part III proposes, 64 slices with 48 inside 500 m, that band's
in-scatter is 1.22 times [the yardstick](#how-the-view-volume-is-proved); 64 slices with 40 inside
500 m are at 0.54 of it there and at most 0.27 in grey haze from 244 to 3 km, and 80 slices with 48 inside
are at 0.31 and 0.16 (the check's part B). The builder chooses the grid; one that passes grey haze alone is
not enough.

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
order. The cell holds the summed extinction and the summed lit source. Beside them it holds the **clear
baseline**: the air and haze alone, with no shadow, no cloud factor and no medium.

**A cell under the model's ground.** The grid's rows descend as the aerial volume's do, and a read blends
them the same way. So a cell's point under the ground takes the air's terms by
[the rule WP9 gives a column](#the-air-at-the-ground): the densities their laws give at that depth, and the
Sun's light of the ground point above dimmed by the air between. The joint cell and the clear baseline both
take it. A provider's coefficients are the provider's at every point the volume names, and the scene's
shadow and the cloud factor are read at the point as anywhere. On 90 rows the stop would lose 78% of the
in-scatter just above the horizon from 3 m, and the rule is within 0.1% (part IV.5).

**History.** A cell's coefficients and sources are blended with their history, reprojected through the cell's
world position and the previous view, and every frame each ray is integrated from the blended cells with the
step that conserves energy. History is kept on what is integrated and not on the integrated result, so a
jittered sample at a medium's edge settles to the cell's mean: beyond a slab's face the transmittance is then
within 1.1%, against 3.7% with history on each cell's own result (part III). History advances only with
submitted frames, and a cut, a new epoch, a replaced device or a provider's reset marks it invalid, after which
the volume is filled from that frame alone and settles within a declared number of frames.

**Rays that leave the volume.** The scene draws its sky through clear, unshadowed air, and draws stars over it
as added light. Along a ray that leaves, the volume's joint pair `(T_joint, S_joint)` stands in for the near
part of that air. So the volume also integrates the clear baseline, `(T_base, S_base)`, by the same step from
the same cells: it is what the sky's radiance already holds for that stretch. At its far end the volume
carries the **terminal pair**: `R = T_joint / T_base`, the medium's transmittance, and
`ΔS = S_joint − R · S_base`. Then, for each pixel:

- the **background**, the sky's radiance with the Sun's disc where the sky's draw holds it, becomes
  `R · L + ΔS`, once;
- each **added emitter** drawn over it, the stars and a disc drawn by itself, is multiplied by `R` and takes no
  `ΔS`. The background's term already holds the medium's light for that pixel. With `R` 0.5, `ΔS` 0.5, a sky
  of 4.4 and a star of 4, the pixel is 4.7; correcting each draw by itself gives 5.2 (part II).

With no medium `R` is exactly 1. Where no medium stands on a ray and nothing shadows its samples, the joint and
baseline cells are the same numbers, so `ΔS` is exactly 0 and the sky's radiance is the clear sky's to the bit.
Where the scene's shadows or the cloud factor darken the near air, `ΔS` is negative with no medium at all: the
baseline is unshadowed and the joint source is not. That is how a shaft shows against the sky, and it is why
the terminal pair is built in WP7 with the air alone. `CLOUDS`' compose already reads the lent transport at its
cloud's front distance, so it needs no change.

For a sky-ending ray with the far aerial operation, carry that same substitution into each finite-distance
result past 32 km. If the clear-air continuation gives `(T_clear, S_clear)` at the queried distance, return
`(R·T_clear, R·S_clear + ΔS)`. The clear path starts from the volume's clear baseline, so the corrected value
at the boundary is the joint boundary value. At the sky endpoint in-scatter tends to the corrected disk-free
sky's radiance. If only the near air is shadowed, `R` is 1 and the near
scatter deficit `ΔS` persists while the same far air adds light to both rays. The clear and ground-ending
paths keep their existing support. The calculation reuses the terminal pair and the existing far approximation
with no new producer or texture.

**The ground band.** A provider may register, beside its 2D Sun-path field, a ground band: optical depth toward
the Sun at heights above the ground, with its mapping and a border width. The band holds the **whole path**:
at each node, the provider's own march from the node to where the ray leaves the band, plus the 2D field's
value there. `CLOUDS` fills it so. The one cloud-factor function therefore makes **one read**: the band, at the
receiver's own position, for a receiver inside it, and the 2D field, as before, for a receiver at or above the
band's top or outside its region. It never adds the two. A receiver in the band under an upper cloud of
optical depth ln 2, with no condensate near it, takes 0.5; adding the 2D field's value to the band's would
give 0.25 (part I). At the band's top its value is the 2D field's, so the factor is continuous there, and
across the border at its side the factor mixes from the band's to the 2D field's, as
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-clouds)'s field mixes to its far value. Every direct-Sun
receiver, the cooked volume's samples and the incident-light meter read through that function, so each takes
fog's shade once. The band may also say where the land blocks the Sun beyond the cascades; only the view
volume's cells read that, never a surface.

**A test medium** in `examples/v1-game`, behind an opt-in control and labelled as a stand-in, writes the medium
coefficients and a ground band for an analytic bank. It proves the path; `CLOUDS` WP14 replaces it.

### Check that it can work

The joint transport is the aerial integrator this module already runs, with more cells, more terms in each and a
history. Its parts are shipped practice at 1 to 3 ms on a 2013 console.
[The planner's reproduction](../../../.orchestrate/evidence/ATMOSPHERE/design-proof/README.md) is arithmetic in f64:

- **I, the ground band.** One read of a band that holds the whole path gives the true factor; a consumer that
  adds the cloud band's value again squares the upper cloud's.
- **II, a ray that leaves.** The medium's transmittance multiplies the air's, `T_joint = R · T_base`, so
  `S_joint + T_joint · L_beyond` equals `ΔS + R · (S_base + T_base · L_beyond)`. With the near air half in
  shadow and no medium, `R` is 1 and `ΔS` is −1 where the clear sky holds 2 for that stretch; held at 0, the
  sky would keep haze the land beside it has lost.
- **III, the grid as a quadrature.** Against a fine quadrature of the same air, with visual ranges from 244 km
  to 3 km, on level, climbing and descending rays to 31 km: 64 slices with 48 inside 500 m are within 0.0043
  in transmittance and 3.4% in in-scatter, at most 0.62 of [the yardstick below](#how-the-view-volume-is-proved).
  32 slices with 24 inside 500 m pass in clear air and miss the ceiling in haze of 20 km and thicker, because
  a linear blend of transmittance across a far cell is too coarse there; with 16 of the 32 beyond 500 m they
  pass. Blending optical depth in place of transmittance brings the transmittance inside the ceiling and
  leaves the in-scatter read as it was, 2.6 times its ceiling in haze of 3 km, so by itself it is not a
  remedy. A slab of fog of 30 m to 10 km visual range is within 1.1% a cell from its faces.

`CLOUDS`' design proof ([part I](../../../.orchestrate/evidence/CLOUDS/design-proof/whole-atmosphere-proof.txt)) shows that a
medium split by a near weight composes with the far part to the exact transmittance. Its in-scatter is an
approximation whose size depends on what stands in the ramp: 0.002 in that proof's uniform medium, and 0.011
in 0.65 for a bank that occupies a 300 to 500 m ramp (the design review's case). `CLOUDS` WP15.4 judges the
handover by eye, with a bank walked through the ramp. Unrun: the look, the history's steadiness and the cost, which WP7 sees first.

<a id="numerical-acceptance-for-the-view-volume"></a>

### How the view volume is proved

Under [the owner's bar](../../../ORCHESTRATE.md#what-is-proved-and-how-hard) the view volume passes on its look and
its speed.

- **The look decides.** Haze from crisp air to murk, shafts below ridges and broken cloud, and a bank of fog
  the camera walks into are judged in the running valley against WP7's and WP8's inspection lists, on the Mac and
  in Chrome.
- **Speed is a tripwire.** WP7.10's and WP8.9's paired records, as [Cost](#cost-2) says.
- **Contracts are tested.** At the defaults every chart holds the bytes it held, and a scene without the
  volume composes what it did (WP7.1, WP7.2). The volume's history follows the frame contract (WP7.5). A ray that
  leaves the volume takes its terminal pair once. With nothing in the way the sky is the clear sky's to the
  bit. On a ray whose near air is shadowed, the composed sky is the clear sky's radiance plus the `ΔS` the
  volume itself reports for that ray, and the test runs a sky that takes `ΔS` twice and one that takes none
  through the same comparison, where each fails: with a clear sky of 4 and `ΔS` of −0.25, once gives 3.75 and
  twice 3.5, and both are dimmer, so dimming alone would pass either (WP7.13). A sky and a star behind a slab
  compose once together, and the lent far lookup preserves the near correction (WP8.2). A provider's medium
  is counted once with the air: the transmittance beyond a slab is the
  air's times the slab's, and a volume that applies either twice fails (WP8.1).
- **The f64 quadrature is a diagnostic.** The [historical haze-kind measurements](../../../.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md)
  record the reference method and results. Part III gives the grids to investigate if haze bands, steps
  or sits too bright. Acceptance is based on the visual inspections.

**The yardstick the proofs quote.** Where [Check that it can work](#check-that-it-can-work-2) says "the ceiling",
it means, per channel at a probe, transmittance within `0.05 × expected + 0.005` and solar in-scatter within
`0.05 × expected + 0.005 × F`, with `F` the reference's in-scatter at 32 km on that ray. The planner used it to
choose the grid. It decides nothing in a build.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The off path is not free | A scene without the view volume composes different shader text or one more pass | Filling the volume and not reading it |
| Sunlight is shadowed twice in the air | Shafts are too dark, or a shadowed surface's haze differs from the haze beside it | Leaving the forward decoder's own split of solar in-scatter on top of the volume's |
| The history trails or flickers | A mover leaves a wake in fog; a shaft's edge crawls in a pan | A longer history |
| The night profile cannot follow a moving haze | Preparing it for a new amount takes longer than the declared lag | Freezing the haze at night without saying so. If preparation is too slow, WP7 records it and the proposal for a prepared set of amounts goes to the log |
| Chrome's sampled-texture limit | Pipeline creation fails with a ground band and a medium registered | Dropping a texture silently |
| The grid is too coarse for a fog's edge on terrain | Tiles show where fog meets a slope | Blurring the fog |
| The far cells are too thick for the haze | Far land in haze of 20 km or thicker bands, steps or sits too bright in WP7's stills | A longer history that smears it. More slices beyond the handover is the remedy. A blend of optical depth repairs the transmittance alone and leaves the in-scatter as it was, so it is no remedy by itself |
| The sky keeps haze the land has lost | A line where shadowed land meets the sky; no shaft against the sky | Holding `ΔS` at 0 when no medium is registered |

### Cost

Without the view volume: nothing. With it, each frame: one sample for each cell (0.6 to 1.4 million at the
proposed sizes) of the air, the charts, the cascades and the Sun-path field, then the integration of the joint
pair and the clear baseline; with a provider, three more reads a cell and the provider's own pass. Recorded paired, on and off, on the Mac and in
Chrome, with tripwires and no budget ([brief](../VISION.md), decision 3).

### Not built

Local lights in the air, a volume for a second camera (so [a mirror view](../ARCHITECTURE.md#scene-views)
carries clear air until its own plan says otherwise), fog indoors, and any density of this module's own.

## Atmosphere lookup regressions (WP11)

Share setup and use compact fields and readbacks for the two cases. Test lookup coordinates, expected
arithmetic and the analytic reference on the CPU where possible. Keep real Metal sampling of the
production H4 decode and diffuse irradiance shaders. The H4 witnesses must still reject each wrong lookup
and cover the live and known fields, depth interpolation and clamp. Keep the analytic direction, colour
and physical-unit witnesses and their bounds.
