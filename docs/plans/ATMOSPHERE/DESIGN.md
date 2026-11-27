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
[The chart cadence](#the-chart-cadence-wp12) governs WP12, added on 27 November for `APPLE-PRESENTATION` WP17,
which holds the display's full 120 Hz at full screen and waits for it.
[The Sun's cascades round the camera](#the-suns-cascades-round-the-camera-wp13) governs WP13, added the same
day for the owner's three reports of Sun shadows that flicker, float and stop at a line.
[Ambient occlusion on the default path](#ambient-occlusion-on-the-default-path-wp14) governs WP14, added the
same day when the owner amended [decision 34](../ARCHITECTURE.md#owner-decisions): "Yes, add screen-space
ambient occlusion to the default path".

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
engine's shadow distance. From WP13 the default reach is longer and the cascades are
[a nested set round the camera](#the-suns-cascades-round-the-camera-wp13), and "both cascades" in this section
reads as every cascade that holds the caster at its size.

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

## The chart cadence (WP12)

Added 27 November 2026. **The owner's words that day,** on full screen on the built-in display: "So the 120
upscale for full screen." V1 holds the display's full 120 Hz there with the picture reconstructed below the
panel's pixel size ([`APPLE-PRESENTATION`'s design](../APPLE-PRESENTATION/DESIGN.md#the-output-scale-wp18)).
The sky's charts are the largest cost in that frame that asks nothing of the picture, and
`APPLE-PRESENTATION` WP17 waits for this work package.

**Revised the same day, after the first build.** The builder's checkpoint `286a34491` on `build/atmosphere`
spread every refresh over the knob's full value. On Metal it held 4 and cut the charts' GPU time at the median
from 1.18 to 0.31 ms. In Chrome at 8, with the Sun scrubbed 1.5° per submission through twilight, the sky
visibly caught up at each refresh: the cited sky patch stepped 27.0 SDR codes in one frame against 11.3 with
the governor off. A spread that long is right for the game's clock and wrong for a scrub, so the knob is now
the longest spread allowed and [the spread follows the motion](#the-spread-follows-the-motion). The
checkpoint's scheduler, device probes, trace and route timing stay.
[Builder's handoff](../../../.orchestrate/evidence/ATMOSPHERE/WP12-handoff.md) ·
[evidence](../../../.orchestrate/evidence/ATMOSPHERE/WP12/README.md)

### Cues

| Cue | Where it must hold | Bar |
|---|---|---|
| C1 The sky, the sky's light on the ground and the sky's reflection follow the Sun and the camera with no step, flicker or flash while their charts are rebuilt over several frames | The game's clock through the sharpest twilight, a time-of-day scrub entered from that clock and left to it, a climb from the valley floor to the lookout's height and a jump of the hour, with the knob at 4 in the Mac window at 120 Hz and at 8 in Chrome at 60 Hz. Chrome's scrub is the fastest planned, 90° of Sun a second. In Chrome, where the oldest reflection level trails longest, the reflection is judged on a surface that reads level 6 alone | Time of day as in Red Dead Redemption 2 or Horizon Forbidden West, which is [WP6's bar](#closing-g3s): colour that moves continuously, and no frame where the sky can be seen to update. Against the governor-off take at the same Sun and camera pose, no frame steps to catch up |
| C2 With the governor off, nothing changes | Every offscreen run, every capture that does not enable the governor, every exact baseline taken without it | Identical work in each frame and identical pixels |

### What is wrong today

Three products follow the Sun and the camera's altitude: the sky view, the diffuse irradiance and the
reflection cube's seven levels. A change in Sun direction or camera altitude invalidates all nine stages.
At fixed direction and altitude, a change in solar illuminance or colour invalidates diffuse. An allocated
overlay also makes diffuse and all seven reflection levels due whenever its registered graph producer writes,
or on every submission if it has no registered graph producer. The running clock or a climb
therefore rebuilds all nine each moving frame. In the WP17 planner's full-screen takes they ran in
434 to 444 of every 438 to 470 frames. Their median times sum to about 1.3 ms of an 8.33 ms interval,
including about 0.1 ms for the sky view, 0.3 ms for diffuse and 0.8 ms for the seven reflection levels. [Full-screen measurements](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md)

The code review's finding M31 is the same fault, and
[`REVIEW-RENDER`](../REVIEW-RENDER/DESIGN.md) left it for the governor's cadence controls.

### Research

Hillaire's method rebuilds the sky view each frame because at his size it is cheap, and that holds here: the
sky view is a tenth of a millisecond. The cost is the filtering that follows it. Unreal Engine's real-time sky
light capture has the same shape, a sky captured and filtered into a mip chain for reflections and into
diffuse light, and by default it spreads the capture and the filtering over several frames
(`r.SkyLight.RealTimeReflectionCapture.TimeSlice`). That is shipped practice for exactly these products, and it
is the mechanism below.

Shipped engines also set how often the sky's lighting is rebuilt by how fast it changes, not by a fixed count
of frames. Unity's HDRP rebuilds the sky's ambient and reflection lighting when a sky property changes, or on a
period the game sets that goes down to every frame. Unreal's capture over several frames is reported to show
when a scene's lighting changes sharply, which is what the first build saw in Chrome. A game's clock moves the
Sun a few hundredths of a degree a frame, and a scrub moves it a degree or more, so one fixed spread cannot
serve both. [HDRP sky update mode](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.1/manual/hdri-sky-volume-override-reference.html) ·
[Unreal sky lights](https://dev.epicgames.com/documentation/unreal-engine/sky-lights-in-unreal-engine) ·
[Unreal forum report](https://forums.unrealengine.com/t/disabling-lumen-by-setting-global-illumination-quality-to-low-or-medium-during-cut-scene-breaks-lighting/2587589/12)

### Mechanism

**The Sun-following charts** are the sky view, the diffuse irradiance and the reflection cube's levels 0 to 6.
Nothing else changes: transmittance, multiple scattering and the split-sum table are rebuilt only when the air
changes, the ground chart and the view volume keep their rules, and the aerial lookup keeps its own cadence
knob.

**Cadence 1 uses the existing product keys.** Empty charts, changed profile epoch, aerosol or night selection
rebuild the nine. Sun direction and altitude are the sky and reflection keys, with diffuse also dependent
on the sky and solar illuminance and colour. A camera cut with identical valid keys adds no stages. With
no overlay, or a registered overlay producer that skips the write, the keys alone determine the due set.
A registered overlay write adds diffuse and reflection levels 0–6. An allocated overlay with no registered
graph producer adds those same eight stages on every submission because a contribution can upload it
directly through the queue. This also refreshes cloudy reflections, whose stored radiance includes solar
illuminance, on the submission that changes the light. Under a registered producer that skips its write,
these keys leave a cloudy reflection at the old solar light until the producer next writes. Cadence 1 keeps
that, because C2 keeps every governor-off frame as it is, and
[the governed keys](#governed-keys) do not. The encoder runs the union in its existing order.
The current diffuse, reflection and haze producers run together as today.

**A governed refresh** visits the due stages in dependency order, sky view, diffuse irradiance, then reflection
levels 0 to 6. On continuous Sun or altitude motion all nine are due.

<a id="governed-keys"></a>**Under the governor a stage is due when what it stores has changed.** The sky view
stores the sky for one Sun direction and altitude. Diffuse stores that sky lit by the solar illuminance and
colour. A clear reflection level stores separate solar and night terms and is lit when it is read, so the
light is not its key. A cloudy level stores radiance with the transmitted clear-sky light already in it, and
both lookups return that radiance as stored, so the light is its key. The scheduler cannot see a level's
encoding without a readback. So in a scene with an allocated overlay, a change in solar illuminance or colour
makes reflection levels 0 to 6 due with diffuse, whether or not the overlay's producer wrote. With no overlay
it makes diffuse alone due. The light is part of [the measure](#the-spread-follows-the-motion), so a halved
illuminance refreshes those eight on the submission that halves it.

- The atmosphere takes one snapshot of the chart inputs at the first submission of the refresh, including Sun
  direction, solar illuminance and colour, altitude and sky mapping. Later stages that read the sky view use
  its matching mapping. The live Sun, aerial inputs and producer-owned cloud overlay retain their own timing.
  Diffuse and each reflection level compose the overlay published by the time their stage runs.
- A refresh has a **spread**, the number of consecutive submissions it takes: from 1 to the knob's value N,
  chosen [by the motion](#the-spread-follows-the-motion). Distribute the nine stages over the S submissions
  of a spread S. When due, the sky view runs first, on the first frame. Each ordinary frame encodes at most
  ⌈9/S⌉ stages (9 at 1, 5 at 2, 3 at 3 or 4, 2 from 5 to 8). Within this bound the builder may balance their GPU costs. Skip stages
  whose keys and overlay are current. Submit commits the cursor, product keys and observed overlay writes. An
  abandoned frame leaves the same stages due.
- While the inputs keep moving, refreshes follow one another. Preserve changes arriving after a product's
  turn for the next refresh, including overlay writes at a stationary Sun. After the last change, finish the
  current refresh and any outstanding products in the following one, then idle. On steady motion at spread S
  the sky view trails by at most S − 1 frames and the oldest reflection level by 2S − 2, so by at most
  2N − 2 at the knob's value. An unchanged input remains current while idle.
- On first use, a new generation, an actual camera cut, a profile, aerosol or night-selection change, or a
  Sun step greater than 2° from the last submission, abandon the spread and produce all nine for this frame
  before readers run. For both hosts, carry the *reason* for a view discontinuity to chart scheduling. A
  continuous time-of-day edit alone is not such a reason, even when the temporal renderer invalidates
  correspondence for the phase edit: its speed chooses the spread. A coincident camera cut still wins.
  Measure the Sun step between submissions, so a jump of the hour updates immediately through the same
  control used to scrub.
- A longer knob value starts with the next refresh. On recovery to a shorter governed value with outstanding
  or changed inputs, cancel the old schedule and run one whole refresh on the first submission reporting
  the shorter value. Resume spreading from the next submission. Already-current, idle products need no
  work. This one recovery frame costs up to nine stages and requires no new resources or history clear.
  In the 6-to-4 example, frames 0–5 use frame 0's snapshot, frame 6 publishes all nine from frame 6,
  and frames 7–10 spread the next refresh. The oldest inputs on frame 8 are two frames old. The new tier's
  bound holds immediately, including if the change arrives partway through the old refresh.

<a id="the-spread-follows-the-motion"></a>**The spread follows the motion.** The knob's value is the longest
spread a refresh may take. The atmosphere's scheduler chooses each refresh's spread from how fast the chart
inputs are changing, so a chart trails the sky by a bounded amount of change and not by a fixed count of
frames. The registry, the governor's policy and the knob's values are untouched: the frame record still
reports the tier's value, and the chart trace reports the spread each submission ran under and the change
that chose it.

- *The measure.* The change between two records of chart inputs is counted in **tolerances**. It is the
  largest of four ratios: the angle between the two Sun directions over the Sun tolerance, the altitude
  difference over the altitude tolerance, and the relative differences in solar illuminance and in any solar
  colour channel over the light tolerance. A relative difference is the difference over the larger of the
  two values. It is 0 between equal values, two zeros included, and 1 from or to zero, which is far past
  any light tolerance and takes a spread of 1. The scheduler already holds each of these for its keys. Where
  the Sun tolerance varies with the Sun's elevation, a measure uses the value at the later of its two
  records, which is always the submission being scheduled. The overlay is not measured. Its producer owns how fast it changes, and
  [the module list's ruling](../ARCHITECTURE.md#seam-ruling-for-atmosphere-wp12) on a cloud's reflection
  stands.
- *Choosing.* On the submission that starts a refresh, take the change c from the previous submission's
  inputs, however long the interval between the two was. The spread is the largest whole number S from 1 to
  the knob's value with S × c ≤ 1. A change over one tolerance has no such S and takes a spread of 1, as
  every change over half a tolerance does. Inputs that have not moved since the previous submission take the
  knob's value. A refresh of spread 1 runs its due stages on that submission from that submission's inputs,
  as the governor-off path does on a moving frame. The scheduler chose it, so the trace records it as a
  refresh of spread 1 with its change, and not as a cut. The stages a submission may encode follow the
  spread chosen, ⌈9/S⌉, and not the knob. With the knob at 4 and the Sun tolerance at 0.2°, a refresh that
  starts four ticks of the game's clock after the submission before it, 0.08° on, takes two submissions of
  up to five stages. One that starts eleven ticks after it, 0.22° or 1.1 tolerances on, takes one submission
  of nine.
- *Cutting short.* On a later submission of a spread refresh, if the inputs have moved more than one
  tolerance from the refresh's snapshot, abandon the refresh and produce all nine from this submission's
  inputs before readers run, as the whole updates above do, with motion named as the reason in the trace.
  A refresh is never cut for a change inside one tolerance of its snapshot. Its first submission takes the
  snapshot and so is never a cut: a large change there chooses a spread of 1.
- *Uneven submissions.* The measure is the change in the inputs and not their speed, so a long interval
  between submissions counts as the motion it carried. At a steady speed and an even pace a refresh ends at
  most (S − 1)/S of a tolerance from its snapshot and is not cut. An interval long enough to carry the rest
  of the tolerance cuts it: with the knob at 8 and the Sun tolerance at 0.2°, six clock steps of 0.02° and
  then one of 0.10°, a submission four ticks late, end 0.22° from the snapshot. That is one whole update
  after a hitch, and it is what makes the bound below hold at any pace. A refresh after a shortened or cut
  one chooses from its own first submission's change, so one long interval shortens or cuts one refresh and
  the spread returns with the next. The trace counts both.
- *What it bounds.* The bound is counted in each tolerance's **widest value**, which is the one value the
  altitude and light tolerances hold and the value the Sun tolerance holds with the Sun 10° or more above
  the horizon. While refreshes are spread, successive snapshots of one chart differ by at most one widest
  tolerance at an even pace and one and a half at any pace, and that is the size of the step when the chart
  is replaced. On every governed submission, each chart was built from inputs within three widest tolerances
  of that frame's in what the chart stores, at any pace and at any hour. The chart being rewritten is within
  one tolerance of its snapshot, or the refresh is cut. One not yet rewritten comes from the refresh before,
  whose snapshot lies at most a tolerance and one submission's change earlier, and that change is at most
  half a tolerance whenever the running spread is 2 or more. Each of those three is counted in the tolerance
  in force when it was measured, which is never wider than the widest, so they sum to at most two and a
  half widest tolerances. The knob's bound in submissions still holds.
- *Where the Sun tolerance tightens,* the bound is not three of the tighter value, and no borrower may read
  it so. With the knob at 8, eight submissions of the game's clock under a 0.2° tolerance leave reflection
  level 6 holding the first one's sky. If the next submission is in a band tuned to 0.05°, its 0.02° step
  chooses a spread of 2 and writes five stages, and level 6 is 0.16° old on that submission: 0.8 of the
  widest tolerance and 3.2 of the tight one. The schedule already replaces it on the following submission,
  and a whole update there would spend nine stages to hide a chart inside the tolerance it was built under.
  A refresh still running when the tolerance tightens is measured against the tighter value and is cut once
  it is more than that from its snapshot. The step when a chart is replaced follows the tolerance its last
  snapshot was taken under, so a Sun tolerance that narrows gradually with elevation keeps each step near
  the value in force, and one that drops at an elevation leaves one refresh's step at the wider value there.
- *The tolerances* start at 0.2° of Sun, 10 m of altitude and 1% of light. In the retained Chrome take the
  governor-off sky patch moves 11.3 SDR codes for 1.5° of Sun near 17.7 solar hours, close to the most it
  moves in that take, so 0.2° is a step of about 1.5 codes when a chart is replaced, the size an 8-bit
  picture of a steadily changing sky already steps by. 10 m is under 1% of the aerosol's 1,200 m scale
  height. The builder tunes the three by eye against WP12.4 and WP12.5 and records the values kept. The
  altitude and light tolerances each hold one value. The Sun tolerance holds one value, its widest, wherever
  the Sun is 10° or more above the horizon. Below that it may tighten toward the horizon, where the sky
  changes most for each degree, and it is nowhere wider. So the game's clock through the sharpest twilight
  may take a shorter spread than the knob's value, down to every submission, if the pictures there need it.
  The scheduler reads the tuning as data, so WP12.1 can run it under a tuning that tightens whether or not
  the kept one does. Three anchors hold whatever the tuning, and WP12.1 checks them. With the Sun 10° or
  more above the horizon, the game's clock at 60 Hz, 0.02° of Sun a submission in a five-minute day, takes
  the knob's value at every tier. A scrub of 1.5° a submission rebuilds on every submission at every tier
  and every hour. A refresh that such a scrub interrupts is cut on the scrub's first submission, which
  keeps the widest Sun tolerance under 1.5°.
- *Where the saving is owed.* The sustained saving is owed where the light changes slowly, which is most
  of play. The Sun is 10° or more above the horizon from about 7 to 17 solar hours of the valley's day, and
  WP12.3's route runs there, from 16 hours. Through the sharpest twilight the bar is the picture alone, and
  a tuning that pays the governor-off cost for those minutes passes. The starting values spread the game's
  clock at every hour, 0.16° over eight submissions at 60 Hz, and they stand if the pictures support them.
  Widening a tolerance at midday is not built, because only a time-lapse there would gain.

**Reflection handoff.** An integer level and its encoding become readable together before forward shading
and the lent lookup run. In a scene with an overlay, classify the current cube on the GPU after each overlay
write, reusing the diffuse producer's transparent-cube test. Each scheduled level uses that classification
and the live cube when filtering, and records its own encoding. A clear level retains separate solar and
night terms. A cloudy level stores filtered physical radiance divided by 32. Both lookups may use the current
single filtered sample when the bracketing levels share an encoding. For mixed encodings they sample each
integer level, decode it to physical radiance with its own tag, then interpolate at the requested roughness.
At cadence 1 the existing diffuse flag is authoritative for the whole cube because the overlay write makes
all levels due together. Governed sampling uses the per-level tags, initialised from the resident cube's
encoding if the governor starts after preparation. The spread needs seven encoding tags and uses the
existing reflection cube without a CPU readback.

**Haze between diffuse updates.** In a governed overlay scene without the view volume, publish the haze scale
after this frame's cloud field and overlay are ready and before the forward and lent aerial decoders read it.
Give the per-frame scalar its own writer/storage so a later diffuse write cannot overwrite it. Keep the
existing horizontal light calculation. Sample the clear and overlaid sky on the same hemisphere nodes with
the available sky view's mapping, using current solar illuminance and colour. Include the current open
direct illuminance and that frame's cloud Sun factor at the camera. Scale by the clamped shaded/open ratio
with the existing clear and near-zero identity rules. The horizontal reduction can share results on diffuse
frames with matching inputs and needs no spherical-harmonic fit on intervening frames. Apply the scale
once to solar in-scatter. Transmission and night retain their values. The view volume lights the air
itself. Scenes without an overlay incur neither cloud classification nor haze work.

**The knob's values.** With the governor off the knob holds 1. Under the governor it holds 4 at the first
tier, 6 at the second and 8 at the third. The registry lets a knob hold one value ungoverned and another at
the first tier from this work package on
([`APPLE-PRESENTATION`'s design](../APPLE-PRESENTATION/DESIGN.md#the-governors-knobs-and-state-wp7)); the
aerial knob does not use it and is unchanged.

- *Why 4 at the first tier.* The governor lowers render scale before it lowers a tier. A cadence that waited
  for the second tier would be reached only with the picture already at half scale, and the cheapest saving
  would come last. At 4 the charts complete a refresh 30 times a second at 120 Hz and 15 times at 60 Hz.
- *Why the value is a ceiling.* On the game's clock the oldest level trails the Sun by 0.12° at 4 and 0.28°
  at 8 at 60 Hz, inside the bound above. Held through the fastest planned scrub (90°/s), the same spreads
  would trail it by 9° and 21°, and the first build showed the second as a step in the sky. So a scrub
  shortens the spread, to every submission at that speed, and pays the governor-off cost while it lasts.
- *Why 1 with the governor off.* Spreading the work helps only where frames are presented against a display's
  interval. Everywhere else it would move pixels for nothing: every offscreen run, test and exact baseline
  taken without the governor stays as it is, to the byte.

**What a consumer may assume.** The true Sun and direct sunlight follow this frame. Chart scheduling moves
no environment epoch and leaves aerial transport on its existing path. The lent sky radiance, diffuse and
reflection products are whole on every submission. At tiers 1, 2 and 3 the maximum age of differing
atmosphere inputs on continuous motion is respectively 6, 10 and 14 submissions, including on tier changes,
and those inputs are within three [widest tolerances](#the-spread-follows-the-motion) of this frame's
however fast the Sun, its light or the camera moves, however the submissions are paced and whatever the
hour. At the starting values that is 0.6° of Sun, 30 m of altitude and 3% of light, and the builder records
the values kept. The figure does not shrink where the Sun tolerance tightens toward the horizon: the
tightening is for the picture, and a borrower relies on the widest value at every hour. That covers what
each product stores, the solar light in a cloudy reflection level included.
Live overlay samples can be newer than the atmosphere snapshot. Both reflection lookups return radiance in
the same units throughout the handoff. [Environment products](../ARCHITECTURE.md#interfaces)

**Exact baselines.** The checkout-local reference guard checks protected files and registry pins. The scheduled
main-branch verifier compares pixels. Ungoverned captures retain their references. If a governed moving
capture needs a refresh, its registered owner follows the applicable ruling and records the set and capture
in this landing. [Reference guard and refresh procedure](../../../CONTRIBUTING.md#exact-image-baselines-and-the-reference-guard)

### Alternatives considered

- **Rebuild all nine every Nth frame**, as the aerial knob does for its one stage. It lowers the mean and leaves
  every Nth frame at the full cost, and a held refresh is decided by the worst frames.
- **Skip a rebuild until the Sun or the camera has moved by a tolerance.** Every chart would then be replaced
  in one frame once the tolerance is reached, which leaves the worst frame at the full cost. The mechanism
  uses the same tolerance to choose how long a refresh may be spread, which keeps the worst frame low on the
  game's clock.
- **One fixed spread for each tier, whatever the motion.** The first build. It passed on the game's clock
  and on the Mac's scrub at 4, and stepped the sky by 27.0 SDR codes in one frame at 8 in Chrome's fastest
  scrub, against 11.3 with the governor off.
- **Fade from the old charts to the new over a few frames.** It hides the step and adds lag: on the retained
  Chrome frames a four-frame fade lowered the largest step from 27.0 to 11.4 codes and raised the mean error
  against the governor-off sky from 8.9 to 11.4, and an eight-frame fade raised the worst error from 23.8 to
  36.0. It also needs a second sky view, diffuse product and cube, and two samples of each in every reader
  while it fades.
  [The builder's calculation](../../../.orchestrate/evidence/ATMOSPHERE/WP12/chrome-catch-up-experiment.log)
- **Spread the work with the governor off as well.** One behaviour everywhere, at the price of moving every
  exact image and fixed capture that has motion in it, where no frame is presented against a display.
- **A second set of charts, swapped in when a refresh completes.** Every reader would see one age. It costs a
  second sky view, diffuse product and cube, and a rebinding in every consumer's group at each swap, to hide mixed ages.
  C1 tests whether mixed ages are visible.
- **Smaller charts.** That changes the picture, which nobody asked for.

### Check that it can work

Each stage is already its own graph pass with a validity bit, and a stage whose bit is set returns before it
encodes, so a stage that is not due costs no pass and no timestamp today. The aerial knob shows the path from
a registered knob to a per-frame decision in `atmosphere/gpu.rs`. For scheduling, the seven
reflection levels share one validity bit and need one each. Also, the nine stages read one uniform record that
is rewritten every frame, so the refresh's inputs need a record of their own that stays still while it runs.

The first build did both, and its scheduler in `atmosphere/gpu/chart_schedule.rs` already holds what the
chosen spread needs. Its input record carries the Sun direction, altitude, solar illuminance and colour of
the last submission and of the refresh's snapshot. It measures the Sun step between submissions for the 2°
jump. It divides the nine stages over any spread, not only 4, 6 and 8, and reports each submission's spread
and position to the trace. Its recovery path is the whole update a cut needs.

### How it is proved

WP12.1 exercises the schedule, the governed keys, the chosen spread and its floor of 1, the cut, uneven
steps and the recovery bound on submitted CPU frames. Its steps are written as fractions of a widest
tolerance, so tuning the tolerances moves no case but the three anchors, which are in degrees. Its
tightening cases run under a test tuning whose Sun tolerance drops to a quarter at an elevation, once with
the drop between two refreshes and once inside one, and its bound follows the source of every readable
stage, the oldest reflection level included, in widest tolerances. WP12.2 uses small Metal probes, and moves its Sun
slowly enough to keep the spread it tests. With the overlay held fixed, compare a moving, spread refresh to a
cadence-1 host at the first frame's atmosphere inputs. With the Sun held fixed, change the overlay after
diffuse runs and compare integer and fractional reflection lookups to the corresponding clear and cloudy
control levels. With a cloudy overlay whose producer has stopped writing, halve the solar illuminance and
compare every reflection level to controls at the old and the new light. Probe the aerial result while a
cloud shadow crosses the camera between diffuse updates.

WP12.3 pairs the valley takes on this Mac. Read the identities of the GPU passes that executed in each
submission, and each refresh's spread and the change that chose it. Every refresh's spread must be the one
its change gives, which is 1 for every change over half a tolerance, however far over, and every submission
of a refresh of spread S must run at most ⌈9/S⌉ stages. A refresh that starts after a long interval, more
than a tolerance on, passes as a refresh of spread 1 and is counted as one, not as a motion cut. The saving
is sustained when complete four-submission refreshes cover at least three quarters of the submissions of
the route's moving segment. A healthy take is far above that, since at the starting tolerance a
refresh there shortens only after an interval of more than 2.5 ticks. A tuning that spreads nothing is at zero. Mark initialization,
discontinuities, tier recovery and motion cuts separately in the same trace, count the refreshes at each
spread from 1 to 4, and keep the cost of all of them in the whole-take figures. The p50 and p95 of summed chart time
are tripwires.

WP12.4 and WP12.5 judge C1 in paired realtime captures, governed against governor off. Match Sun and
camera pose on the same material. Attach the chosen spread and the executed stages to submitted frame IDs,
so the images establish which schedule produced them. Each target judges the cases the mechanism
separates. On the game's clock through the sharpest twilight, at whatever spread the tuning chooses there,
no frame may step more than the governor-off take does at the same pose. Through a scrub, with the clock
running before and after it, the spread shortens and lengthens under the same bar. The Mac holds the knob at
4, its first tier. Chrome holds it at 8, the longest spread at the slower display, and scrubs at 1.5° per
submitted frame, which is the take that failed: the first build's
[retained frames](../../../.orchestrate/evidence/ATMOSPHERE/WP12/README.md) from submissions 150 to 165 are
the example of the fault. The two scrubs are entered differently. The Mac's follows two complete
four-submission refreshes, which shows the spreading sustained up to the scrub, and so starts a new
refresh. Chrome's first scrub submission arrives partway through an eight-submission refresh, as a player's
may on any submission. The trace must show that submission run all nine stages as a motion cut, and the
frame must show the governor-off take's sky, shade and reflection at that pose, so a renderer that reported
the cut and still read the interrupted refresh's charts fails. At 16.9 solar hours the Sun is 11.6° above
the horizon, so the first anchor puts the clock before that scrub at eight submissions a refresh whatever
the tuning. Chrome also frames a surface of roughness 1, which reads reflection level 6 alone,
and judges it where refreshes take eight submissions: there level 6 is the oldest chart a reader sees, up
to fourteen submissions behind, and the first build's takes left it unjudged. WP12.6 checks the protected
references locally.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The worst frame is not lowered | The 95th percentile of the summed chart time stays where the median was | Reporting the mean alone |
| A later stage reads this frame's inputs | A seam in the reflected sky, or reflection and sky that disagree in azimuth, while the Sun moves | Testing a still Sun |
| The sky steps where it changes fastest | At twilight in Chrome, the whole sky's brightness changes at each refresh and holds between | Judging a noon scrub only, or a scrub only, where nothing is spread |
| The tolerance is tuned until nothing is spread | With the Sun high, the game's clock rebuilds all nine on every frame, and the route's chart time returns to the governor-off figure | Passing the pictures with the saving gone |
| A scrub that starts mid-refresh is noticed late | The first scrub frames show the pre-scrub sky over newly lit ground | Starting every scrub take from a still Sun or on a refresh boundary, or passing the first scrub frame on the trace's label alone |
| A borrower's bound moves with the hour | Where the Sun tolerance tightens, a chart built under the wider value is more than three of the tighter one behind | Promising three of the tolerance in force, or a whole update at every tightening |
| A long interval before a refresh is misreported | A refresh that starts more than a tolerance on fails the route check, or is counted as a motion cut | Capping the recorded change at one tolerance |
| One long frame costs more than one refresh | After a single late submission on the game's clock, a second refresh is shortened or cut | Widening the tolerance until a hitch fits inside it, or cutting on elapsed time and not on the change |
| A cloudy reflection keeps the old light | After a light edit under cloud whose producer is not writing, the ground dims and its reflections do not | Testing the edit under a clear sky, or with the overlay rewritten on the same submission |
| The oldest reflection level is never looked at | The inspected surface is smooth and reads level 0, or level 6 is shown only as a line in the trace | Passing the reflection on the trace's word that level 6 ran |
| A jump shows the old sky | Up to six frames of the old hour's sky over newly lit ground after the hour is set | Lowering the cadence's value to hide it |
| The governor-off path changed | An exact image taken without the governor moves | Refreshing it |

### Cost

On ordinary moving frames at the first tier, nine stages that summed to about 1.3 ms become at most three
per submission. The first build measured 0.31 ms at the median and 0.55 ms at the 95th percentile on the Mac
route at 4, against 1.18 and 1.61 ms with the governor off. While the sky's inputs move faster than half a
tolerance a submission, as in a scrub, the charts cost what they cost with the governor off, and choosing
the spread is a few comparisons on the CPU. A long interval costs one shortened refresh or one whole
update. Keep the full-refresh frames and the shortened refreshes in the cost record. In an overlay scene a
light-only edit under the governor makes eight stages due where cadence 1 makes one. In overlay scenes account separately for
classification (at most one scan of the 48² × 6 overlay per write), horizontal haze reduction (256 sky samples
when due), and the second reflection sample at a mixed-encoding boundary. The clear valley incurs none of
these overlay costs.

## The Sun's cascades round the camera (WP13)

WP13, added on 27 November 2026 for three reports the owner made that day from the V1 showcase on the Mac:

1. "I'm seeing some shadow flicker at times, for example when you fly towards this beige ballon, or the shadows
   from the rocks flickering in the scene with the squares."
2. "The shadows are only drawn for objects that are near by which means there is a very clear line where the
   shadows suddenly appear. It's a bit weird, but maybe it's correct for how games like this need to work"
3. "The shadow flicker is very obvious here", of the cubes beside the sphere. The lead added that the cubes show
   no shadow or contact darkening of their own and read as floating.

The planner reproduced all three with realtime captures and found the causes from the frames
([the record](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/README.md), with its routes, frames and
the owner's screenshots). This section replaces the two fitted cascades of the original design with a nested set
round the camera. [The caster height map](#what-the-caster-height-map-holds) and the march that reads it are unchanged.

### Cues

- **Shadows are steady.** With the game's clock running, an edge slides across the ground as the Sun moves and
  bends over what it falls on. It never steps, crawls, flickers or drops out and comes back, with the camera
  still, flying or turning. A shadow may change continuously: with the Sun, with a caster that moves, with what
  it crosses, and by fading with distance.
- **Shadows meet what casts them.** A cube, a rock or a balloon's basket standing on sunlit ground has its shadow
  start at its base, at every hour the Sun is up.
- **No line where shadows end.** Looking across the valley, shadows thin out with distance by the size of what
  casts them and are gone before anyone can say where. Ridges and mesas shade the far ground as they do today.

Range: the exit scene's 8,500 still props (0.25 m cactus trunks to 7 m boulders) and 1,500 balloons 65 to 295 m
up, and the bare valley's sphere, rock and cubes, from the ground to the flythrough's height, at every hour the
Sun is up, on the game's clock, which turns the Sun 1.2° a second (a day in 300 s), on the Mac and in Chrome.

### What is wrong today

All of it follows from two 1024² maps fitted to the view (`synthetic/shadow.rs`) and from how the shader reads
them (`synthetic.wgsl`). With the showcase camera (46° tall, near plane 0.1 m) the fit splits the 512 m reach at
38 m: the near map's texels are 7 to 8 cm and the far map's about 1 m.

| Report | What the frames show | Cause |
|---|---|---|
| 1 and 3, flicker | With the camera and the Sun both held, consecutive frames are identical. With the camera flying or turning and the Sun held, shadows keep their shape. With the camera held and the clock running, a rock's shadow changes outline every frame, a cactus's shadow comes and goes, and the sphere's shadow edge is a staircase whose steps move every frame | The maps are snapped to whole texels for a moving camera, which works. Nothing can hold them as the Sun turns: each frame draws every caster onto a grid at a new angle. That shows because a texel is tens of pixels on screen (26 in the sphere's staircase at 1,280 × 720), an edge whose penumbra is under half a texel is read with one 2×2 comparison, and a caster thinner than a texel is drawn only when it covers a texel's centre |
| 3, floating | A lit strip rings each sunlit cube's base. At a low Sun the first cube-length of shadow is pale speckle | The receiver's depth offset is sized by the texel: half a texel's depth step, which grows as 1 / tan(Sun elevation), plus a fixed 6 cm. That is 8 cm along the ray at 35°, 27 cm at 10°, against a 36 cm cube, and about 1 m in the far map |
| 3, floating in shade | A cube inside the sphere's shadow has no contact cue | Nothing on the default path darkens where objects meet. Not this work package's: [WP14](#ambient-occlusion-on-the-default-path-wp14) builds it |
| 2, the line | Props cast shadows up to a line across the ground and none beyond | Past the 512 m reach a receiver reads only the caster height map, whose 32 m texels hold no prop, and the hand-over is a 51 m band |

### Research

- **Stable cascades.** Snapping a cascade to whole texels stops edges crawling under camera motion, at some cost
  in resolution; it does nothing for a light that turns
  ([Pettineo, A Sampling of Shadow Techniques](https://therealmjp.github.io/posts/shadow-maps), after Valient).
  Thrum already snaps.
- **A turning Sun defeats caching.** Engines that cache shadow depth (Insomniac's scrolled and cached cascades,
  CRYENGINE's cached far cascades, Unreal's virtual shadow map pages) rely on a light that holds still
  ([Acton, Day and Hastings 2012](https://advances.realtimerendering.com/s2012/insomniac/Acton-CSM_Scrolling%28Siggraph2012%29.pdf),
  [CRYENGINE cached shadows](https://cryengine.com/docs/static/engines/cryengine-3/categories/1114113/pages/21267738)).
  Fortnite's team names a continuously moving Sun among what undercut caching for them
  ([Virtual Shadow Maps in Fortnite Battle Royale Chapter 4](https://www.unrealengine.com/tech-blog/virtual-shadow-maps-in-fortnite-battle-royale-chapter-4),
  read through a search summary: the page refused the planner's fetch).
  So with a day of 300 s the maps are redrawn every frame, and stability has to come from texel density and
  filtering.
- **Density near one texel to a pixel, in levels round the camera.** Unreal's virtual shadow maps give a
  directional light a clipmap of expanding ranges around the camera, every level the same resolution and
  twice the radius of the one before
  ([Virtual Shadow Maps](https://dev.epicgames.com/documentation/unreal-engine/virtual-shadow-maps-in-unreal-engine)).
  The layout does not depend on where the camera looks.
- **A filter several texels wide from few taps.** Bilinear comparison taps weighted as one tent give a 5×5
  texel kernel from nine reads ([Castaño, Shadow Mapping Summary](https://www.ludicon.com/castano/blog/?p=901),
  from The Witness).
- **Transitions and the far end.** Shipped engines blend neighbouring cascades over a fraction of each and fade
  the last one out over a band (Unreal's Cascade Transition Fraction and Distance Fadeout Fraction,
  [Directional Lights](https://dev.epicgames.com/documentation/unreal-engine/directional-lights-in-unreal-engine);
  HDRP's cascade borders, [Shadows override](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@17.0/manual/reference-shadows-volume-override.html)),
  and stop drawing a caster into a shadow map once it is too small there (Unreal's
  [`r.Shadow.RadiusThreshold`](https://indxzero.github.io/ue544cvarwiki/articles/r.shadow.radiusthreshold)).

### Mechanism

**The Sun cascades become a nested set of square maps centred on the camera.** Each has the same number of
texels and serves twice the distance of the one inside it. The planner's starting set is nine cascades of 2048²,
with radii from 8 m to 2,048 m and so texels from about 0.8 cm to 2 m. What binds is the six rules below.

Three words are used throughout. A receiver's **distance** is the straight-line distance from the eye of the
view being drawn to the receiver: the main camera's position, or a secondary view's own. A cascade's **radius**
is the largest distance it serves. A caster's [**cast width**](../GLOSSARY.md#cast-width) is the diameter of
the largest ball that fits inside its mesh, times the instance's smallest axis scale.

1. **Layout.** A cascade is a square in the Sun's frame, centred on the eye and laid out in whole texels of its
   own, as each map is today, so camera travel moves it a texel at a time. Its half-width is its radius plus a
   guard band, the same number of texels in every cascade (16 to start). A ball of a cascade's radius round the
   eye is a disc of that radius in the Sun's frame at any Sun elevation, so the square holds every receiver the
   cascade serves, with the guard band to spare for the search and the filter. A cascade's place depends on the
   eye's position, the Sun's direction and the casters' depth range, and on nothing else: turning the camera or
   changing its field of view changes no cascade. Every cascade keeps every caster up-Sun of its square in its
   depth range. The near map's partition, its bounded blocker range and the last cascade's padding for the
   Sun's cone go.
2. **Which cascade a receiver reads is decided by its distance, and by nothing else.** It reads the finest
   cascade whose radius is at least its distance. Across the outer quarter of each radius (a starting value) the
   answer blends into the next cascade's, so no line marks where one ends. Whether a point falls inside a
   cascade's square does not choose it. The square is only what the map covers, and at a low Sun it covers
   ground far past the radius along the Sun's bearing: at 5° a point 500 m out lies 44 m from the centre of the
   Sun's frame. Surfaces, the view volume's samples and the lent Sun lookup all take the cascade, the blend and
   the fade below from this one function of distance. Because the choice ignores where the camera looks, turning
   the camera changes no receiver's answer.
3. **A blocker is found and filtered at the scale of its penumbra.** The Sun's disc gives a blocker a penumbra
   whose radius on the receiver is the blocker's distance along the ray times 0.00465 (`SUN_ANGULAR_RADIUS`). A
   cascade can find a blocker only within its search, a few taps over at most eight texels (a starting value),
   and that is all its guard band holds. So the read has two parts, split at the distance along the ray where
   the penumbra's radius is eight texels of the receiver's cascade, about 14 m in the finest:
   - **Near blockers**, inside that distance, are found and filtered in the receiver's own cascade. Contact
     shadows, and the shadows of props the size rule keeps out of coarse cascades, come from here.
   - **Far blockers**, beyond it, cast a penumbra wider than the receiver's cascade can search, and their
     outline can lie wholly outside its square: a balloon 260 m up at a 10° Sun is 1,500 m along the ray and
     shades ground 7 m to the side of its outline. They are found by a second search, in the finest cascade
     whose eight texels span the widest penumbra any caster can throw on that receiver, which the casters'
     depth range gives. The first search also reports a far blocker that stands directly over the receiver.
     The nearest far blocker found sets the cascade the far part is filtered in: the one where its penumbra is
     four to eight texels in radius. That part counts far blockers only, so no blocker is counted twice.
   - The receiver's answer is the darker of the two parts. Where the receiver's own cascade already spans the
     widest penumbra, there is one part and one read.

   So a soft balloon shadow and a rock's sharp contact shadow are both present on the same ground, each read
   at its own scale. Shadows still sharpen toward contact and soften with the blocker's distance. A caster the
   size rule keeps out of the second search's cascade is found as a far blocker only where it stands over the
   receiver, so the outermost part of a small caster's wide penumbra is trimmed. That is the faint tip of a long
   thin shadow at a low Sun, and it is [not built](#not-built-by-wp13).
4. **The filter is never narrower than about two and a half texels** (a starting value), in either part, so an
   edge is a ramp and never the texel staircase.
5. **Bias is counted in texels of the cascade read.** Every term of the receiver's offset scales with that
   cascade's texel, and none is a fixed length in metres. Near the camera the lit strip at a base shrinks from
   centimetres to millimetres with the texel.
6. **A cascade holds only casters it can hold steady, by their cast width.** A caster whose cast width is under
   about one and a half texels of a cascade (a starting value, never under √2) is left out of that cascade and
   of every coarser one. The measure is the caster's thickest part and not its bounds: a cactus's bounds are
   its arms' span, but its shadow is as wide as its trunk. A disc at least √2 texels across covers a texel's
   centre wherever the grid lies, and a ball inside the mesh shows as a disc of its diameter from every Sun
   direction, so a caster a cascade holds always draws into it. The cast width is measured once from a mesh's
   triangles, when the mesh is cooked or registered, and does not change with the Sun, so no caster enters or
   leaves a cascade as the Sun turns. An estimate may fall short of the true ball, which only shortens a
   shadow, and never exceeds it. A sheet, which has no inside, takes the largest disc that fits on it. The
   shadow of a caster ends across the blend band of the last cascade that holds it: there it is still at
   least one and a half texels wide, and it fades with the receiver's distance before the next cascade would
   lose it. Parts of a mesh thinner than its thickest part can lose texels in that last cascade; the filter's
   width makes that a change of tone inside a soft shadow. If WP13.4 shows such parts breaking up, the builder
   raises the threshold. Terrain and a contribution's casters, which give no mesh for each caster, are in every
   cascade. An aloft row is held to the same rule and is still never in the caster height map.

**Reach.** The reach is the coarsest cascade's radius, a distance from the eye: 2,048 m to start, where it was
512 m of view depth. Over the outer two fifths of the reach (a starting value) the answer blends into the
caster height map's alone, as the last tenth of the reach does today. Past the reach a receiver reads only the
height map, whatever the coarsest cascade's square covers there: only the ground and what stands on it cast,
at the height map's 32 m scale, as now. The low-Sun rule, that the height map's horizon may only darken a
cascade's answer when the Sun is within about 6° of the horizon, stays.

So shadows end by size. With the starting set a cactus's or a scrub's shadow ends between about 100 and 250 m
by the width of its trunk or its stems, a standing rock's or a small boulder's between about 400 m and the reach, and
a large boulder's and a balloon's fade out with the reach. No distance is the end for everything, which is
what made the line.

**What the builder tunes.** One ratio is fixed: each cascade's radius is twice the next finer one's, which the
profile's meaning and WP13.1 hold. The builder tunes the rest by eye against WP13's lists and by the cost
record: the count, the map size, the reach, the guard band, the blend and fade fractions, the least filter
width, the search's eight texels and the size threshold down to √2.

**A scene's profile.** `SyntheticView::with_sun_shadow_profile(count, reach)` keeps its meaning and admits up to
the engine's full count: `count` cascades, the coarsest of radius `reach`, each inner one half the next. One
cascade is a single cascade of that radius, and none leaves the height map alone, as today. A view that sets
no profile gets the full set to the default reach. The reach was a view depth and is now a distance from the
eye. The two agree on the view's axis and differ off it, where a receiver at the same depth is farther away.

| What a neighbour relies on | What it sees |
|---|---|
| `SCENE-VIEWS`: a view's Sun shadows come from one cascade of its own, inside the view's shadow reach, with the height map beyond, and its picture equals the ordinary main draw at the same camera and profile | The same. Both sides lay the cascade out by rule 1 and choose by rule 2, each from its own eye, so a secondary view's 128 m cascade shades nothing past 128 m from that view's eye. Its tests keep passing; one that places a receiver off the axis between the old depth and the new distance is restated at the same claim |
| `CULLING`: the caster lists of each Sun pass draw what the unculled route draws | The same for more passes. The size rule applies alike under Direct, Frustum and Occlusion, as the aloft mark does, and reads a cast width kept with each mesh's bounds |
| `RAY-TRACING`: a scene that asked for the traced method records no Sun map pass, and `sun_visibility` is the one operation it replaces | Unchanged |
| `WATERS` WP16, when it builds the lent Sun visibility: one tap of the cascades and the height map at a world point | The tap reads the cascade rule 2 picks for the point's distance from the eye of the view the effect is drawn in, with the same blend and fade, and the height map alone past the reach |
| The view volume, the air's shadow heights and a contribution's `draw_shadow` | A view-volume sample is a receiver: its distance from the eye picks its cascade and its fade. The height map's readers and the contribution interface are unchanged: a contribution is asked to draw into each cascade |

### Alternatives considered

| Option | Why not |
|---|---|
| Keep cascades fitted to the view and add two | A fitted cascade moves when the camera turns. At this field of view its bounding sphere is as wide as the camera-centred square (514 m against 512 m), so the fit buys no resolution |
| Hold the Sun still for the maps and step it every so often | At 1.2° a second each step is a visible jump of every shadow, and fading between two directions doubles the maps |
| Cache still casters' depth and redraw only what moved | Void each frame the Sun turns, which on the game's clock is every frame (the research above) |
| Virtual shadow maps | Left until after V1 by the owner on 17 November ([the module list](../ARCHITECTURE.md#held-open-for-after-v1-cluster-level-of-detail-dynamic-gi-and-virtual-shadow-maps)). The nested set is the same layout at a fixed, small size and lays no foundation against it |
| Ray-traced Sun shadows | `RAY-TRACING`'s, an opt-in on the Mac alone. The exit asks for cascaded shadows on the Mac and in Chrome |
| A finer caster height map for far props | It does not turn with the Sun, so it could carry prop shadows past the reach without shimmer. It holds only what stands on the ground and would be a second way to shade the same receivers. Kept as the next step if WP13.5 still finds a line |
| Draw the coarse cascades every few frames | Under a Sun this fast a balloon's shadow crosses the ground at about 20 m a second, so a cascade drawn four frames ago is visibly behind. Left out; see [Cost](#cost-of-wp13) |

### Check that it can work

- **Texels against pixels.** A pixel covers 0.57 to 0.79 mm for each metre of distance at 1,497 and 1,080 rows
  (full screen at 67%, and the window). A cascade of the starting set has texels of about `r / 1008` for its
  radius `r` and serves receivers from three eighths of `r` out to `r`, so a texel there is about 1.3 to 4.6
  pixels. The finest is read from the eye out: its 0.8 cm texel is about ten pixels at one metre, where the
  near map's is a hundred. At 40 m today's far texel is 31 to 44 pixels. With a filter two and a half texels
  wide, what redrawing at a new Sun angle can change is under a texel of edge position spread across the ramp,
  where today it is a whole step of the staircase.
- **Nothing thinner than the grid, by the thickest part and not the bounds.** The design review projected the
  exit's cactus of layout entry 1548 (scale 0.926, 16 h) onto 0.5 m texels: its bounds span 1.86 by 4.06
  texels, which passes a rule on bounds, and its triangles cover four texel centres at one place of the grid
  and none at another. Its trunk, the thickest part, is about 0.3 m, 0.6 of a texel, so rule 6 leaves it out
  of that cascade and of the 0.25 m one, and keeps it in the 0.125 m cascade, where the trunk is over two
  texels. The frames' blinking cactus shadow is this trunk in 1 m texels.
- **The base.** With every term in texels, the offset along the ray in the finest cascade at 10° of Sun is
  about 2.3 cm for the depth step, where the near map's is 27 cm.
- **The reach is a distance.** A secondary view with one 128 m cascade at a 5° Sun: ground 500 m from its eye
  along the Sun's bearing lies 44 m from the centre of that cascade's square, inside it, and a prop standing
  there is drawn into the map. Rule 2 does not read the map for that ground, because it is 500 m away, so it
  gets the height map alone, as the profile promises.
- **A balloon's penumbra reaches ground its outline does not.** Near the camera the finest cascade's square is
  about 8 m in half-width, and a balloon's shadow edge is soft over 7 m to each side. The second search runs
  in the 1,024 m cascade, whose eight texels span 8 m, and whose square holds every balloon within a kilometre
  of the eye in the Sun's frame. A cactus beside the same receiver is not in that cascade and is found by the
  first search, in the finest.
- **The far end.** With the starting values a caster's shadow ends where its thickest part is about two to seven
  pixels wide on screen, across a band a quarter of a radius deep, and each size ends at its own distance.
- **If it does not work.** If WP13.4 or WP13.6 still shows crawl with the starting set tuned, the cause is not
  texel density and the builder ends `BLOCKED` with frames. It does not hold the Sun still for the maps or add
  a pass that smooths shadows over time.

### Exact baselines for WP13

WP13 is meant to change the default output of every scene with a Sun shadow, so
[rule 1](../ARCHITECTURE.md#exact-baselines) applies as a planned move, and
[the module list's ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp13) says which sets move and who refreshes
them. It is not built behind a selection, because the defects are in the one Sun visibility every scene reads.
The pixels that move are those a cascade shadows or nearly shadows: shadow edges, the ground at the base of the
sphere, the rock and the cubes, and receivers between 512 m and the new reach that a prop or a ridge now shades
through a cascade. The sky, the stars, the air's haze and unshadowed ground far from an edge are predicted not
to move.

<a id="cost-of-wp13"></a>

### Cost

Today the Sun's two cascades, the height map and the air's shadow pass take 0.56 to 0.76 ms of GPU time at the
median on the Mac, of a frame that `APPLE-PRESENTATION` WP17 is holding to 120 Hz at full screen
([its measurements](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md)). By the planner's
estimate from the layout generator's rules, which is not a count, the three map passes draw about 0.65 million
triangles a frame on the exit scene: the 62,000-triangle terrain three times, about 3,000 props and balloons
in the far map and all 8,500 still props in the height map.

The starting set draws the terrain nine times and each prop into the few cascades that hold it at its size:
by the same estimate about 1.35 million triangles with the height map as it is, twice today's, and four times
the texels in each map. Three more things change with it. The Sun's caster cull, a compute pass ahead of the
maps (`PassKey::CasterCull`), prepares a list for nine cascades where it prepared two and applies the size
rule. The forward pass's Sun read changes: a receiver near the camera makes two searches and two filtered
reads where it made one or two, and a receiver in a blend band reads two cascades. So the change is expected
to cost more than today, by an amount nobody has measured.

WP13.8 records it, paired before and after. The record counts the Sun's work as every cascade's pass, the
caster height map, the air's shadow pass and the caster cull, and takes the forward opaque pass beside it.
Its Apple row is the workload the owner named: full screen on the built-in display at the 67% output scale,
which is the owner's floor and the step WP17 is trying, with MetalFX, EDR and the governor on, and with the
governor's render scale and the drawn extent recorded through each take. A window would understate the
forward pass. No figure is a budget. If the Sun's summed work, the caster cull alone or the forward pass comes
out more than about a quarter over its time before, the builder pulls these levers in order, and records what
each bought:

1. Fewer, wider cascades, while WP13's lists still pass.
2. Skip a contribution's draw into a cascade its `shadow_bounds` do not overlap, and redraw the caster height
   map only on a frame when something in it could have changed (it scrolled a texel, or a caster that is not
   aloft moved or changed), where the builder can show the skipped frame's map would be the same bytes.
3. A cheaper read where the receiver is far from any edge: no second search where the first finds nothing
   and the second's cascade is empty over the receiver, and one read in place of two.

Drawing the coarse cascades less often than every frame is not one of them. It needs a schedule that follows
how fast each cascade's shadows move, which on this clock is fast, and a governor knob. If the record after the
three levers is still over, the builder says so in its handoff and the lead takes it to a planner.

<a id="wp13-hand-off-to-wp17"></a>**The hand-off to WP17.** The verdict on 120 Hz at full screen is
`APPLE-PRESENTATION` WP17's, from its closing sitting, and that sitting qualifies the frame it was taken on.
WP17 does not wait for WP13 in the plan, so the sitting can come first. WP13's record therefore ends with the
whole frame's timed span on the full-screen row before and after, the 120 Hz intervals served and missed in
each take, and the commit of WP17's closing sitting if one has been taken. It then says which of three holds.
The sitting has not been taken, and is then taken on a frame with this change. Or it was taken on a commit
without this change and the span after is no higher than before, beyond the spread between the record's own
repeats: it stands. Or it was taken without this change and the span is higher: WP17's sitting is to be
taken again, and the builder's handoff asks the lead for it in those words. WP13.8 fails without that
section.

The maps take about 150 MB where they took 12 MB. If the workspace's GPU tests slow or run short of memory, the
builder gives their small scenes a smaller profile through the existing call. It does not shrink the default.

### How WP13 is proved

- **WP13.1**, tests on the CPU, for the layout, the cast width and which cascade holds which caster.
- **WP13.2**, small tests on Metal, for what only the drawn maps and the shader's read can show: the choice by
  distance, the blends, the reach, and near and far blockers together.
- **WP13.3**, a reviewer's question: each rendered behaviour the two-cascade tests guarded and the design keeps
  still has a test of the production code.
- **WP13.4 to WP13.6**, a reviewer's inspection of each of the owner's three views on the Mac, with the clock
  running, against the take from before.
- **WP13.7**, the same lists in Chrome, on the same routes.
- **WP13.8**, the cost record and the hand-off to WP17.
- **WP13.9 and WP13.10**, the planned refresh of the exact baselines and a look at what moved.
- **WP13.11**, the maturity ledger.

What a moving shadow may do. The inspections judge four faults at the size the take plays at: crawl along an
edge, a shadow that drops out and comes back, a jump in sharpness or darkness, and a shadow that comes away
from the foot of its caster. A shadow that changes continuously is not at fault: it slides with the Sun,
moves with a drifting balloon, bends over a rock it crosses, is hidden by what stands in front of it, and
fades with distance under rule 6 and the reach. In the exit scene the 1,500 balloons drift whether or not
the Sun is held, so a claim that shadows stay put is made of the still props there, and of everything at the
valley's cubes, where nothing moves.

Where the offline reference stands. The half-disc test in `synthetic/shadow/tests/disk.rs` traces f64 rays
through triangles in test code and never runs the shader. It checks WP4's diagnostic and protects no rendered
behaviour. WP13 keeps it as that, and holds the shader to no fraction it gives: the exact finite-disk match
stays open. What the shader does with soft shadows and hand-overs is proved on the device, by WP13.2 and the
inspections.

The routes are in [the diagnosis folder](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/routes/), in
the working folder every worktree shares. Each is written for a 1280 × 720 game grid and cuts from the authored
camera to its pose. A route's `start.hold_solar` holds the Sun, and without it the
game's clock runs. The Mac takes a route with `scripts/capture_v1_runtime.py --route` and Chrome takes the
same file with `scripts/capture_v1_browser.mjs --route-file`. Both keep every submitted image with its tick,
eye, heading and solar hour, so a Mac frame and a Chrome frame are at a matching pose when those records
agree.

| Route | Scene | What it is for |
|---|---|---|
| `balloons.json` | exit | The owner's first view: 15 s toward the beige and red balloons from 16 h, looking down on rocks and cacti. Mac and Chrome |
| `e2-held-sunruns.json` | exit | That view with the camera held, so only the Sun and the balloons move |
| `e4-turn-sunheld.json` | exit | That view turning in place with the Sun held |
| `valley-floor.json` | exit | The owner's second view: down to the valley floor and level across it, the ground running out to the ridges |
| `c1-held-sunruns.json` | valley, `proof` overlay | The owner's third view from 15 h, camera held above the cubes beside the sphere |
| `cubes.json` | valley, `proof` overlay | Round the cubes from 17.3 h to sunset. Mac and Chrome |
| `c2-moving-sunheld.json` | valley, `proof` overlay | Moving round the cubes with the Sun held |

A route names the cooked root it was cut for. If the root has moved, the builder cuts the route again at the
same poses and says so.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| The staircase is traded for blur | Contact shadows of the cubes are soft smears wider than the cubes' edges are sharp | Passing "no steps" on a filter many texels wide |
| Shimmer hidden by the reconstructor | The takes with reconstruction off still crawl and the MetalFX take does not | Judging the flicker points with reconstruction on |
| The size rule draws its own line | All shadows of one kind of prop stop at one distance in a band a reviewer can point to | Lowering the threshold until thin casters blink again |
| A thin caster passes by its bounds | A cactus's or a scrub's shadow comes and goes at about 150 to 400 m | Measuring the size rule by bounds, or by a box round the mesh |
| The gap at the base returns at a low Sun | At 17.3 h a cube's shadow starts a visible way from its base | Judging at 15 h only |
| A blend band shows | A faint ring round the camera where shadows change sharpness or strength | Judging stills only: fly through it |
| A square is read past its radius | At a low Sun, props far along the Sun's bearing cast through a fine cascade, and their shadows change as the Sun's bearing swings | Choosing a receiver's cascade by which square holds it |
| A balloon's shadow has a hard outline or a cut edge near the camera | The soft edge stops at a line about 8 m from the camera, or shows a sharp core | One read in the receiver's own cascade for every blocker |
| A long shadow's tip is cut | At a low Sun a cactus's shadow ends in a step where it should fade out | Widening the first search until it is a few taps over tens of texels |
| Cost is carried by the governor | The exit's Apple row holds its rate only because the render scale dropped | Recording the frame rate and not the Sun passes' time and the scale |
| Far terrain shadows move | Ridge shadows past the reach differ from before | Refreshing the baselines without WP13.10's look |

<a id="not-built-by-wp13"></a>

### Not built

- **Contact darkening in shade.** A cube inside another shadow gets no cue from the Sun. That cue is ambient
  occlusion, which [WP14](#ambient-occlusion-on-the-default-path-wp14) builds on the default path after the
  owner amended [decision 34](../ARCHITECTURE.md#owner-decisions). WP13 judges grounding in sunlight.
- **Screen-space contact shadows** for the last millimetres at a base. They need the opaque depth before
  shading, which the forward pass does not have.
- **Shadows of props past the reach**, and a cadence for the coarse cascades (above).
- **The outermost penumbra of a small caster far up the ray.** Rule 3 trims it where the caster is too small
  for the second search's cascade. The shadow there is already mostly light.
- **A sheet edge-on to the Sun.** Its shadow is a line thinner than any texel and may shimmer. The showcase has
  no sheet that casts.
- **The moving edge on the authored west-lookout spur** between 17.40 and 17.80 h, which stays open under G3s
  with [WP3's record](#wp3-surface-sun-visibility). WP13 replaces the filter that record studied and does not
  judge the spur.

## Ambient occlusion on the default path (WP14)

WP14, added on 27 November 2026. [The diagnosis](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/README.md#the-causes)
of the owner's shadow reports found, as cause 7, that in shade nothing darkens where an object meets the ground:
the default path had no ambient occlusion, because [owner decision 34](../ARCHITECTURE.md#owner-decisions)
gave it to ray tracing alone and planned no screen-space stand-in. Asked whether to revisit that decision so the
default path gets cheap screen-space ambient occlusion, the owner said:

> Yes, add screen-space ambient occlusion to the default path

Decision 34 as amended: screen-space ambient occlusion is on the default path, on the Mac and in Chrome.
Ray-traced occlusion from `RAY-TRACING` stays the higher tier where rays run, and replaces the screen-space
method there. The two never stack. Lamp shadows are still traced only.

WP14 is built after WP13. Both edit the forward shader and the light group, both move the same exact baselines,
and WP14's lists are judged at the cubes, where WP13 first has to hold the sphere's shadow still.

### Cues

- **Grounded in shade.** A thing standing on ground the Sun does not reach, inside another shadow, in a ridge's
  shade or after sunset, has a soft darkening where it meets the ground, and reads as standing on it. Creases
  where things touch are darker than the faces beside them.
- **Sunlight keeps its look.** The darkening is the sky light's alone. Sunlit ground, flat faces and the sky
  look as they do without it, and nothing looks dirty or outlined.
- **Steady.** Flying, turning and beside drifting balloons, the darkening stays on what it belongs to: no dark
  or light outline round a silhouette, no grain or shimmer, no trail.

Range: the bare valley's sphere, rock and cubes (0.36 m) and the exit scene's 8,500 still props, 0.25 m cactus
trunks to 7 m boulders, with 1,500 balloons adrift, from the ground to the flythrough's height, at every hour,
at render extents from 1280 × 720 to the full screen's 2,316 × 1,497 under the governor, on both of the Mac's
recipes and in Chrome.

### Who owns it

**The scene's ambient occlusion is `ATMOSPHERE`'s**: the request a scene makes, the one method each scene
resolves to, the value a fragment reads and how the light step applies it, the screen-space method with its
history, and its quality value. `RAY-TRACING` owns the traced method: its rays, its filter and its channel of
the traced visibility history ([its design](../RAY-TRACING/DESIGN.md#ambient-occlusion-wp5)).

The reason is the one that already divides the Sun. Ambient occlusion is how much of the sky's light reaches a
surface, as a Sun shadow is how much of the Sun's does. This module owns both sources and the Sun's visibility
at a receiver, with the cascades as its portable method and `RAY-TRACING`'s rays as the traced one. Occlusion
gets the same cut: one operation, a portable method here and a traced method there. It also owns the routes,
the takes from before and the grounding lists of WP13, which WP14 is judged beside.

| Other owner considered | Why not |
|---|---|
| `EFFECT-GRAPH`, as an effect | It owns how a module declares passes and state, and no effect. Occlusion is not a contribution a scene lists: the forward shader's light step reads it for every surface |
| `RAY-TRACING` | Nothing of its is a default, it is built only when no V1 work package can start, and it exists only on the Apple tile recipe. Work that is on by default in Chrome cannot live under those rules |
| `APPLE-PRESENTATION` | It owns how a history is keyed, accepted and reset, and the governor. Occlusion follows those rules and registers a quality value, as the aerial lookup and the charts do |
| A new module | One work package, in files this module and the forward shader already hold, with no state another module would build on |

### Research

- **Ground-truth ambient occlusion (GTAO).** Activision's method for Call of Duty searches the depth buffer for
  the horizon in a few slices round the view direction and integrates the open arc with a cosine weight, which
  matches a ray-traced reference where older horizon methods over-darken. It is computed at half resolution,
  filtered over space and time, and cost 0.5 ms on a PS4 at 1080p
  ([Jiménez, Wu, Pesce and Jarabo 2016](https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf)).
  Unreal and Unity's HDRP offer it as their screen-space method.
- **An open implementation.** Intel's [XeGTAO](https://github.com/GameTechDev/XeGTAO) (MIT) adds a depth
  mip chain so far taps stay cheap, noise that varies over space and time, an edge-aware filter and normals
  taken from depth, and is tuned against a ray-traced reference. Intel says it supersedes ASSAO. Its source may
  be ported.
- **Visibility bitmask.** A depth buffer read as a solid height field makes a thin thing shade everything
  behind it, which is the dark halo of screen-space occlusion. Keeping the occluded sectors of each slice as a
  bitmask, with a thickness for each depth sample, lets light pass behind a thin thing at the same cost
  ([Therrien, Levesque and Gilet 2023](https://arxiv.org/abs/2301.11376)). Bevy replaced its GTAO with it in
  0.15, in WGSL on wgpu, for "thin geometry" ([release notes](https://bevy.org/news/bevy-0-15/)); that source
  (MIT or Apache-2.0) may be ported too.
- **Occlusion read by a forward shader.** Forward renderers that apply occlusion to indirect light alone need
  it before they shade. Godot's and Unity's forward paths get it from a depth prepass
  ([the decision](../../design/decisions/POST-V1-FOUNDATIONS.md)). Apple's GPUs resolve which surface is
  visible before they shade ([Harness Apple GPUs with Metal](https://developer.apple.com/videos/play/wwdc2020/10602/)),
  so a prepass saves no shading there and adds a draw of every mesh.
- **What Thrum already has.** `RAY-TRACING` WP5 runs its occlusion pass after the frame's opaque geometry and
  the next frame's forward pass reads the result by reprojection, checked against depth. That read passed
  review on this Mac at contact, at rest and in motion (the `RAY_TRACING_WP5` row of
  [the maturity ledger](../../design/maturity.json)).

### Mechanism

**One value at a fragment, from one method a scene resolves when it is prepared.**

- **The request.** A scene asks for ambient occlusion as off, or as a radius in metres with a preferred
  method, screen-space or traced. It is fixed when the scene is prepared, like the Sun's shadow method. A scene
  that gives none gets none: it prepares no occlusion pass, texture or binding and draws the bytes it drew
  (C04).
- **Resolution.** One function resolves it before anything is allocated. A radius resolves to the
  **screen-space** method on every recipe and device. A preference for **traced** resolves to traced where
  `RAY-TRACING`'s admission passes, and otherwise to screen-space at the same radius with the admission's
  reason. A scene resolved to one method declares no pass, texture or pipeline of the other. Where the forward
  stage would bind more sampled textures than the device allows, or the device lacks a format the pass
  needs, occlusion resolves off with that reason and the scene is not refused (C08).
- **The showcase asks.** The valley, with or without the proof overlay, and the exit scene ask for the
  screen-space method at the game's radius, 1 m to start, in every launch: the Mac's two recipes, Chrome and
  the iPhone simulator. A proving build's launch value can turn it off, set the radius or prefer traced.
  `RAY-TRACING`'s launches that ask for traced occlusion resolve as they do today where rays run.
- **Report.** The report that names the Sun's shadow method names occlusion too: what was asked, the method
  resolved and why they differ.

**The screen-space method** is GTAO's horizon search with the visibility bitmask, at half the render extent
each way, accumulated over frames. It is a pass of the scene's own, declared in the one graph after the
frame's opaque geometry, where the traced visibility pass sits. It reads the frame's depth, view record and
motion target, which every view of a 3D scene publishes (C07), and nothing else of the frame:

1. **Depth.** It reduces the frame's depth to a half-extent linear depth with a short mip chain. A tap far
   from its pixel reads a coarse mip, so the cost does not grow with the radius.
2. **Normal.** It rebuilds each pixel's normal from neighbouring depths, taking the neighbours on the pixel's
   own surface. That is the triangle's normal, which is what occlusion wants. No guide buffer and no prepass
   is built.
3. **Gather.** In a few slices round the view direction it steps outward to the radius, in metres at the
   pixel's depth, up to a cap in pixels. Each tap marks the sectors of the slice it covers in a bitmask, from
   its own depth to a constant thickness behind it. The open sectors, weighted by the cosine to the normal,
   are the visibility. The slice directions and the step offsets differ from pixel to pixel and from frame to
   frame. The sky's pixels neither receive nor occlude. A tap outside the frame marks nothing.
4. **Over time.** It reprojects last frame's result through the motion target, rejects history whose depth
   disagrees, clamps what is left to the range of this frame's neighbourhood and blends. Its memory in frames
   is the **settle length**, a runtime value the builder sets by eye and the report states.
5. **Over space.** One edge-stopping filter, guided by depth and the rebuilt normal, so darkening does not
   leak across a silhouette.

The result is **the occlusion history**: at half extent, the visibility and the depth it belongs to. The
traced method's filtered result has the same two values, so shading reads either without knowing which
method wrote it.

**What a fragment reads.** The forward pass reads last frame's occlusion history at the fragment's previous
position, which it already has for its motion vector: the four nearest half-extent texels, each weighted by
how well its stored depth matches the fragment's. That is the bilateral upsample. With no matching history
the value is 1. This is the read `RAY-TRACING` WP5 built, and from WP14 both methods use the one function.
Fragments drawn after opaque geometry, such as water, read nothing and are as they were.

**What it multiplies.** Indirect light only (C08): the diffuse sky light, and the reflected sky light through
Lagarde's specular occlusion. It multiplies the material's own occlusion and does not replace it. It never
touches the Sun's light, a lamp's or emitted light. This is the application `RAY-TRACING` WP5 built, in the
atmosphere-lit and the fixed-light variants of the shader, and both methods use the one function.

**Latency is accepted, as it is for traced occlusion.** A surface shows the value its place had a frame ago.
After a thing arrives or leaves, its darkening follows within the settle length and one frame. Occlusion is
broad and faint and the display is at 120 Hz. A darkening still visible after that is a trail, and fails.

**History.** The occlusion history is committed only with a submitted frame, and reset on a camera cut and a
replaced device, by `APPLE-PRESENTATION`'s history rules. Its textures are sized once for a generation, from
the output extent, as the forward attachments are, and the pass fills half the drawn rectangle. A step of the
render scale does not reset it: the pass and the fragment's read address the history by the drawn extent it
was written at, so the first frame at a new scale reads the frame before it and nothing is created. The
governor steps the scale during play, and a reset there would blink every contact in view. The traced
method's history resets at a step today and keeps doing so.

**Quality.** One quality value, the gather's slices and steps, declared through the contribution seam's
quality knobs with three tier values and no history invalidation, for the governor. The policy's order is
unchanged: scale first, then tiers. No tier turns occlusion off.

| What a neighbour relies on | What it sees |
|---|---|
| `RAY-TRACING`: a scene that asked for traced occlusion, where rays run, traces it and draws what it drew | The same. The request now names the method, and such a scene declares no screen-space pass. Where its admission fails, the scene shows screen-space occlusion where it showed none |
| `RAY-TRACING`: with soft Sun shadows and traced occlusion both on, one history texture carries both | Unchanged |
| A scene that asks for no occlusion, which is every scene but the showcase's | Its bytes, passes and bindings are today's. `SCENE-VIEWS`', `CULLING`'s, `WATERS`' and `CLOUDS`' tests are of such scenes |
| `SCENE-VIEWS`: a secondary view's picture | A view has no history key and reads no occlusion, in any scene. A mirror shows the scene without it |
| `APPLE-PRESENTATION`: history rules, the knob registry, `SceneSurface`'s three colour formats | Followed as found. The forward pass gains one sampled texture and one uniform in the light group, and no colour target |
| The frame contract's guide buffers (C07) | Not selected. If a later module builds the prepass, the pass can take its normals and the read does not change |

### Alternatives considered

| Option | Why not |
|---|---|
| A depth prepass, with occlusion computed and read in the same frame | A second draw of 10,000 entities and every cooked mesh in every frame, on a frame with no time to spare at full screen, and on Apple's GPUs it saves no shading. One frame of latency on this term already passed review for the traced method |
| Multiply the finished colour by occlusion after shading | It darkens sunlit and emissive surfaces as much as shaded ones, which is the dirty look of early screen-space occlusion, and C08 gives occlusion indirect light alone |
| Plain GTAO horizons, as XeGTAO ships | A cactus trunk or a post would darken the ground metres behind it. The bitmask costs the same and is the published fix |
| Hemisphere-kernel SSAO, HBAO, ASSAO | Older. They are not cosine-weighted, over-darken creases and halo more. Intel retired ASSAO for XeGTAO |
| Full render extent | Four times the gather for a term with no fine detail. The depth-weighted read restores the edges |
| No accumulation over frames, more taps in each | Several times the gather for the same grain |
| Ray-traced occlusion everywhere | Chrome has no ray queries, and the traced path is an opt-in on this Mac |
| Screen-space indirect light, or bent normals for the sky's direction | A further step on the same gather. Global illumination is left until after V1 |

### Check that it can work

- **Depth after opaque geometry is there.** The traced visibility pass and the late effects read it today, on
  both recipes.
- **The read exists and has been judged.** The forward read, its depth weights and the application are in the
  tree and passed `RAY-TRACING` WP5's review on Metal. In Chrome the same WGSL is one more sampled texture in
  the forward stage: by the count `synthetic/preparation.rs` makes, the valley's fullest composition is 15 of
  WebGPU's 16 with it.
- **Texels against things.** A pixel covers 1.18 mm for each metre of distance at 720 rows, so a half-extent
  texel covers 2.4 mm. At 3 m a 0.36 m cube is about 50 texels and a 1 m radius is capped. At 100 m a texel is
  24 cm: a 1 m rock is four texels and still gets a contact. By about 400 m the radius is a texel and
  occlusion fades to nothing, where a prop is a few pixels and its shadow is thinning out too (WP13).
- **The halo.** Plain horizons treat a cactus trunk as solid to any depth behind it, so ground within the
  radius behind the trunk loses the whole arc above the trunk's front. With a thickness of some tens of
  centimetres, only the sectors between the trunk's front and that depth behind it are marked: from ground
  half a metre behind a 0.25 m trunk, a thin slice of the arc. Ground further behind than the radius is out
  of reach of either, so a dark outline there can only be the filter or the read leaking across the
  silhouette, which their depth weights are there to stop.
- **If it does not work.** If a halo or shimmer survives tuning the thickness, the counts and the settle
  length, the builder ends `BLOCKED` with frames. It does not add a prepass, blur the result until contacts
  go soft, or multiply the finished colour.

### Exact baselines for WP14

WP14 is meant to change the showcase's default output, so [rule 1](../ARCHITECTURE.md#exact-baselines) applies
as a planned move and [the module list's ruling](../ARCHITECTURE.md#ruling-on-atmosphere-wp14) says which sets
move and who refreshes them. All five registered sets are captures of the valley as the game launches it, by
day and by night, and all five are expected to move: sky light reaches the ground at every hour. The pixels
that move are on surfaces within about the radius of another surface: the foot of the sphere, the rock and the
cubes, and creases in the terrain. The sky, the stars and the bars do not move. Scenes of other modules ask
for no occlusion, so no other exact test is expected to move.

<a id="cost-of-wp14"></a>

### Cost

Nobody has measured it. The paper's 0.5 ms is for a gather of 0.5 million pixels, half of 1080p each way, on
a PS4. Here the gather is 0.5 to 0.9 million pixels at the full screen's render extents, on a much faster GPU,
so the pass is expected to be a few tenths of a millisecond on this Mac, and the forward pass gains four texel
reads a fragment. It lands on a frame with no time to spare: at full screen at the panel's pixel size
`APPLE-PRESENTATION` WP17 measured a timed span of about 10 ms against 8.33 ms, with the governor at its floor
([its measurements](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP17-plan/README.md)), and that work
package draws full screen at an output scale of 67% to hold the rate. So the cost is
recorded where that work package will read it (WP14.6): paired on and off on one commit, on the Apple row at
the full screen's output extent with the governor on, on the portable row and in Chrome. No figure is a budget.

The record's tripwires are the pass over about 0.5 ms on an Apple row, the forward pass more than about a
tenth over its time with occlusion off, and a governor that settles on a lower render scale with occlusion on.
When one trips, the builder pulls these levers in order and records what each bought:

1. Fewer slices and steps at the first tier, with a longer settle length, while WP14's lists still pass.
2. A shorter mip chain or a narrower spatial filter.
3. The gather at a quarter of the render extent each way, with the history and the read kept at half.

The occlusion history, its previous copy and the depth chain are about 20 bytes for each half-extent texel:
17 MB at the full screen's output extent and 5 MB at 1280 × 720.

### How WP14 is proved

- **WP14.1**, tests on the CPU, for the request, its resolution, the composed shader and the quality value,
  with a reviewer's question on the one read and the one application.
- **WP14.2**, small tests on Metal, for what only the drawn depth, the pass and the forward read can show.
- **WP14.3**, a reviewer's inspection of stills on the Mac: grounded in shade, and sunlight unchanged.
- **WP14.4**, the same takes in motion: no halo, shimmer, trail or blink.
- **WP14.5**, both lists in Chrome.
- **WP14.6**, the cost record.
- **WP14.7 and WP14.8**, the planned refresh of the exact baselines and a look at what moved.
- **WP14.9**, the maturity ledger and the creator's page.

Every inspection sets a take beside the same route with occlusion turned off, which is the "before". The
routes are WP13's, in [the diagnosis folder](../../../.orchestrate/evidence/ATMOSPHERE/WP13-diagnosis/routes/),
and three copies that differ from their originals only in `start`, which the builder keeps in
`.orchestrate/evidence/ATMOSPHERE/WP14/routes/`:

| Route | Scene | What it is for |
|---|---|---|
| `c2-moving-sunheld.json` | valley, `proof` overlay | Round the cubes with the Sun held at 15.3 h: cubes inside the sphere's shadow, and cubes in sunlight beside them |
| `c2-dusk.json`, a copy with the Sun held 4° below the horizon (18.3 h as planned) | valley, `proof` overlay | The cubes, the sphere and the rock lit by the sky alone |
| `valley-floor-dusk.json`, a copy held at the same hour | exit | Down to the valley floor and level across it at dusk: every prop is in shade |
| `valley-floor-low-sun.json`, a copy held at 17.3 h | exit | The same flight with long shadows: props inside a ridge's or a mesa's shadow beside props in sunlight |
| `balloons.json` | exit | Flying at 17 m/s over rocks and cacti toward the balloons, clock running from 16 h: sunlight, and motion |
| `valley-floor.json` | exit | The descent and level flight with the clock running |
| `e4-turn-sunheld.json` | exit | Turning in place: the frame's edges |

If 18.3 h does not put the Sun about 4° down in the scene, the builder sets the hour that does and says so.

### Risks

| Risk | Early sign | Wrong substitute to refuse |
|---|---|---|
| Darkening that is everywhere | Sunlit ground and flat walls are greyer than with occlusion off | Passing "grounded" on a picture that is darker all over |
| A halo traded for no contact | Thin props have no darkening at their base | Lowering the thickness until cacti float again, or raising it until they outline |
| Shimmer hidden by the reconstructor | The takes with reconstruction off sparkle at contacts and the MetalFX take does not | Judging grain only with reconstruction on |
| Blur in place of accumulation | Contacts are wide grey smudges with no dark core | Widening the spatial filter to hide grain |
| A trail read as softness | A darkening lingers where a balloon's basket or the camera's parallax has moved on | Judging stills only |
| A blink at a scale step | Contacts flash on the Apple row when the governor moves | Judging only takes in which the scale never stepped |
| Cost carried by the governor | The Apple row keeps its rate because the render scale fell | Recording the frame rate and not the pass times and the scale |
| A second kind of occlusion | A scene on the traced method also runs the screen-space pass, or the two are multiplied | Judging the traced launch by eye alone: WP14.2 counts the passes |

<a id="not-built-by-wp14"></a>

### Not built

- **Occluders off screen.** A thing outside the frame, or hidden behind another, darkens nothing. The traced
  method is the tier that lifts this, and WP14's lists do not judge it.
- **Occlusion in a secondary view.** A mirror shows the scene without it.
- **Lamp shadows without rays**, the other half of decision 34, which stays as it was.
- **Screen-space contact shadows** for the Sun, which need depth before shading.
- **Coloured or bounced occlusion**: the multi-bounce tint, bent normals and screen-space indirect light.
- **Occlusion as the default of every scene.** A scene asks. Whether the starter scene and a game made with
  the SDK ask by default is `CREATOR`'s and `GAME-SDK`'s to plan.
