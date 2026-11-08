# Atmosphere for the V1 outdoor scene

[Rendering contract](../RENDER-SPEC.md#5-initial-atmosphere-fog-and-transport-operator) ·
[Environment source](ENVIRONMENT.md) · [Milestones](ATMOSPHERE-PLAN.md)

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
  The scene's +X east, +Y up, +Z north and 45° latitude define the pole.
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

Proposed for the low-Sun terrain path, pending independent design review. Use one
solid-angle integral over the rendered Sun. At each world azimuth in its aperture,
clip terrain and other occluder spans to the same elevation chord, union the spans,
then integrate their `sin(elevation)` differences over azimuth. Duplicated coverage
is idempotent. Blend only equivalent estimates of the same occluder set. Geometry
partition, map reach and camera depth cannot omit an occluder. The point-ray
triangle oracle remains independent of the runtime integration.

The prepared terrain field keeps potentially dominant original edges over each
registered receiver chart patch and azimuth interval. The cooked terrain geometry
and placement identify the chart. An edge can be omitted only if its upper slope
bound cannot exceed a retained lower witness anywhere in the cell. A variable-range
edge witness must intersect every ray in the cell at positive range. Evaluate the
retained edges at the actual receiver and azimuth. Partition whole triangles so a
range clip cannot invent an edge inside a triangle. Subdivide loose cells, round
bounds outward and preserve wrap. The lower blocked interval requires a certified
solid terrain prefix from the common origin to every potential winner. The source
grid certificate, cooked triangle correspondence and opacity establish the surface.
Certify that the evaluated XZ origin stays in that footprint. A vertical lift uses
positional error and the adjacent faces' maximum height gradient to clear the full
local surface. Keep whole-triangle spans for boundary bands, disconnected receivers
or numerically unresolved geometry. The terrain field has no fixed receiver-relative
range partition.

Prune with a pairwise slope comparison on the true receiver triangle in a fixed
world meridian for each sector. With positive denominators, cross-multiply the edge
ratios. At each angular endpoint the resulting quadratic has six triangular Bernstein
coefficients. Their upper bounds certify dominance across the face, with vertex
intervals enclosing transform, fragment interpolation and origin lift. Every
omission needs a retained, fully supported witness. Split unresolved cells, keep
ties and competitors without a proof, and share identical sparse support lists.

Use original nearby terrain and compact caster triangles with conservative beam or
tile candidate culling. Bound the inner and outer clipped span unions over each
angular bin. Include silhouette endpoints as bin edges and refine until the summed
area enclosure meets the tolerance. The envelope handles crossings of endpoints
inside a bin. On portable, use a Sun-centred rational meridian coordinate
`q = tan(azimuth offset)` and `v = sin(elevation)`, evaluated by vector ratios and
square roots with a bounded Jacobian `dv*dq/(1+q²)`. Use the actual packed Sun
direction and charge coordinate and rounding error to the visibility tolerance.
Share candidate retrieval across receivers while retaining each receiver's coverage.

Key the prepared field by its receiver chart and transform, every covered terrain
and boundary dependency, and the caster partition and evaluator versions. Publish
the complete recipe before view extraction and keep it pinned by submitted frames
and captures. A replacement may use the existing recipe as a whole until the
enhanced recipe is ready. A missing or uncertain entry within an admitted field
requires matching geometry coverage. Certify bounds over supported receiver patches,
Sun directions and caster transforms, including offscreen contributors. Select the ordinary recipe before
submission if the generation or transform leaves that envelope. The route is a
performance witness, not an admission domain. Uncertainty cannot become clear sky
or switch methods across camera depth. Preserve the old aerial interval owner and
the graph's submitted-frame ownership. Unsubmitted frames never alter the admitted chart.

The old all-receiver geometry beam is too costly as a default. In the retained
crest diagnostic, partial receivers grow from 33,965 to 812,485 at 1080p. A
clear/full classifier alone cannot make refinement rare. The archived far-terrain
CPU witness narrows 951 potential edge entries to 34 over four bins at the difficult
4 m receiver patch. The 1,024 m crest source raises that to 84 retained entries.
Both match the triangle-section reference at the sampled rays. They do not
establish a scene-wide bound. A single-aperture Ground E census leaves 3.95 million
far edge entries and 2.75 million near triangles under the flat box-indexed layout
even with a tightened corner bound. The verified cooked faces over six fixed
sectors need 41.53 MiB after list interning, with 170 references at p95 and 581
at maximum per face-sector pair. Sampled clipped horizons agree with the
triangle reference. Materialized per-face lists at this subdivision are too
costly for one aperture. A directional sweep shares exact upstream terrain
sections along a fixed line. The sampled suffix hulls have 13 vertices at p95,
but reusing their support off-axis can miss the full disk. A global sweep
arrangement encounters 2.34 million projected vertex-order changes in six sectors.
Test shared hull bridges within fixed source blocks, with a bounded upstream join
and exact complement. Within a constant positive crossing set, endpoint tangents
bound the angular envelope. Price compact caster spans separately and verify the
narrow crest, sector seam and grazing rays before scaling to the admitted domain.
[Directional hull sweep, §§4–5](https://doi.org/10.1111/j.1467-8659.2009.01642.x).
Before implementation, choose the portable representation and admit it against
actual requested WebGPU limits, including staging and replacement overlap. Prove
the geometry prefix, the angular error after decoding and shader rounding, and
visibility continuity across chart and recipe boundaries. A candidate path with
no depth producer on Direct or unresolved overflow after submission is ineligible.
[Bounded horizon context](https://www.cg.cs.tu-bs.de/upload/publications/fritsch2025fchorizonmaps/fritsch2025fchorizonmaps.pdf) ·
[Occluder fusion](https://www-sop.inria.fr/reves/CrossmodPublic/Documents/EGSR_bmss.pdf).

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
