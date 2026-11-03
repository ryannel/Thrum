# First coupled air/Sun proof profile

2 November2026. Bounded experiment under the [complete cloud design](CLOUDS.md), its finite ambient DAG and C08/C12.
This selects a Sun-only air attenuation helper before extending the renderer. It does not declare the full atmosphere,
fog, cloud family or engine complete. Independent proposal review and actual GPU qualification precede adoption.

## Held calibration and product

Use the existing authored optical-reference profile: Earth radius6,371,000m, air top100,000m; Rayleigh RGB sea-level
coefficients[5.802,13.558,33.1]e-6/m, scale8000m; aerosol scattering3.996e-6/m, **extinction4.4e-6/m**, scale1200m, HGg=.8;
ozone off. Therefore aerosol absorption is.404e-6/m. This differs from a publication's4.4e-6 absorption label and must
not silently inherit its provenance. Top-of-atmosphere irradiance is the light input; do not attenuate an already
attenuated ground irradiance again. Scattering coefficients/phase belong to sources, extinction to transmittance.

One256mu×64height LUT, endpoints included. Cook512midpoint samples per path in float64; runtime payloadRGBA16F
[T_R,T_G,T_B,1], little-endian128KiB, with profile/mapping/cooker/table SHA pins. A48B uniform carries geometric and
optical profile values. Exact producer/decoded bytes and immutable-resource/staging overlap count. Upload/cook is not
per-frame work: the table depends on atmosphere profile, not cloud age, camera, weather IDs or Sun angle. Sun/Moon
can share it with distinct incident directions. No new quality-dimension sweep is commissioned.

The finite CPU comparison covers16,384knots and1153visible off-grid cases plus42planet-blocked cases. Maximum knot
quadrature error is about2.1e-5; half-table bilinear lookup error about.000462. These support an experiment, not a
uniform certificate. Hardware filtering, local-world float32 mapping, horizon/ground branches, exact loaded-artifact
identity, preparation and query cost remain actual GPU gates. Portable half filtering is the first candidate; retain
manual/reference lookup in the inspector. First finite GPU acceptance is total absolute RGBT error<=.0015against the independent fine reference for admitted probes, plus exact uploaded half-byte parity. Reserve .0005for table/quadrature/storage,.0005for filtering and.0005for mapping/numerics as an experimental allocation, not a uniform certificate. Report each component and rejected/ambiguous inputs separately; do not relax it after seeing GPU results.

## Stable coordinates and explicit validity

For local height h∈[0,A], mu∈[-1,1], let r=R+h, rho=sqrt(h(2R+h)), H=sqrt(A(2R+A)), delta=A-h, C=delta(2R+A+h), b=r*mu.
Unoccluded distance to top is sqrt(b²+C)-b; for outward mu>=0 use C/(sqrt(b²+C)+b). At top/outward returnT=1 explicitly.
Map x_r=rho/H, x_mu=(d-delta)/(rho+H-delta). In the outward branch evaluate
`d-delta=2*r*delta*(1-mu)/(sqrt(b²+C)+b+delta)` to avoid nearby-distance subtraction. Texture centres are
`(.5+x*(size-1))/size`; endpoints are not cell boundaries.

Inverse rho=H*x_r, h=rho²/(sqrt(R²+rho²)+R), d=delta+x_mu*(rho+H-delta),
mu=(C-d²)/(2*r*d). Explicit d=0 selects outward1. The final x_mu row stores the visible grazing LIMIT, not opaque
planet visibility. Planet visibility precedes lookup: ground h=0/mu<=0 is blocked; otherwise mu<=-rho/r is blocked,
including tangent contact. Never useabs(mu), a clamped horizon lookup or a zero ground root to fabricate clear Sun.

API separates `heightMu -> {RGBT,valid}` from optional world-point conversion. Valid planet-blocked returns zeroT;
unsupported/nonfinite/out-of-domain input returns an explicit invalid result, not clear/black light masquerading as
valid. World conversion keeps local height via
`h=(x²+z²+y*(2R+y))/(sqrt(x²+z²+(R+y)²)+R)` in the declared Earth-tangent frame. Preserve normalized radial mu and
qualify the float32 range/roundoff policy explicitly; naive sqrt(...)-R loses sub.25m height near Earth. Wrapper dot
roundoff and known geometry-boundary height are different from accepting arbitrary out-of-domain primitive inputs.
Above-atmosphere origins need a separate entry trace; this helper cannot simply clamp them clear.
The first world wrapper admits |x|,|z|<=100km and0<=y<=20km in the declared tangent frame. Here its height numerator
is a sum of nonnegative terms. It covers the held cloud/near receiver fixture, whose receiver ray distance is capped
at80km. It does not silently admit global far surfaces or full atmosphere-ray sample positions. The height/mu primitive
still covers the whole LUT domain for conformance. Broader/inward world geometry is a separate GPU qualification gate.

For the first float32 predicate, ground h=0/mu<=0 is explicitly blocked and outward mu>=0 is visible. On an inward
ray, treat |mu-mu_h|<=1e-6 as numerically ambiguous and return unsupported, rather than pretending a rounded horizon
tie certifies the real predicate. Outside that band apply the declared mask; compare it against the independent
reference at actual probe inputs. This is a conservative candidate admission band, not a universal WGSL error proof.
The mathematical tangent is opaque; its uncertain float32 representation is not silently declared clear or black.
World-wrapper cosine roundoff may be constrained to[-1,1] only after unit-direction validation and an explicitly
small rounding allowance; arbitrary out-of-domain height/mu primitive inputs remain unsupported.


The half table has110zero blue nodes on long grazing paths. Segment ratios can therefore be0/0; even a float32 table
can give Tsegment>1 after interpolation. **No finite-segment transmittance ratio API is admitted.** Integrate finite
view-air coefficients directly. Do not hide the failed ratio with a[0,1]clamp. Stable local coefficients use the
Cartesian rationalized height; any path-form density evaluation must avoid subtracting nearly equal squared radii.
For an outward path use rho²+s*(2b+s); for an admitted inward grazing path a closest-approach form may be more stable.
Those float32 regimes require their own checks rather than borrowing the float64 cook result.

## Complete coupling order and bounded slices

3 November2026: the next complete graph uses the [selected detailed joint-media design](../../evidence/cloud-joint-media-design-2026-11-03/PREFLIGHT.md).
This supersedes the earlier512-anchor/24-direction A0 proposal for that graph. The held Sun-air-only calibration above
and its evidence remain unchanged. The complete graph explicitly selects the Hillaire reference's aerosol absorption
and ozone profile, rather than silently reusing the older minimal extinction calibration; products carry separate hashes.

1. One immutable media generation binds the presented source/weather/Sun/fog time, profile, optical mask, view demands
   and device epoch. Owned source products and fenced GPU resources keep current and retained consumers consistent.
2. Direct air and fog sources use ordinary cloud Sun visibility even outside local cloud support. Apply air/fog/geometry
   Sun factors once; cloud MS modifies source terms, never view extinction. Offscreen casters remain in the complete query.
3. Four-LUT air provides transmittance, clear multiple scattering, sky view and per-view aerial perspective. A pristine
   upper-hemisphere mean supplies cloud ambient and the declared fog fill; clouded environment never feeds its own producer.
4. Sum air, cloud and fog RGB coefficients/source before one Beer transfer. AP is eligible only for one complete clear,
   unshadowed camera-origin-to-terminal ray. Mixed rays use disjoint joint-media/shadow spans and bounded reanchored direct
   clear-air gaps. No AP prefix subtraction/division, clear-air overlay or unqualified transmittance-ratio segment API.
5. BSM/point-tail/direct visibility covers all admitted source bounds. Main, reflected and environment rays have independent
   origins, intervals and terminals. Apply the surface near leg and Sun disk once; environment excludes the direct disk.
6. Current environment capture produces SH9 and rough reflection products without reading its own output. Main-hidden
   execution retains requested source/light/environment/reflection consumers. Feature-off savings follow actual demands.
7. Four-template mixed pipelines bind only their inputs, fitting16 sampled textures; complete-clear AP/Sky paths omit native
   cloud bindings. The detailed resource/work ledger includes all products, retained generations, uploads and explicit overflow.

The [maintained CPU fog reference](../../../scripts/cloud_atmosphere_proof/README.md) is implemented and independently
reviewed, with12 focused tests passing. Its analytic ellipsoid integral and stated binary64/host-transcendental assumptions
are a CPU correctness checkpoint only. WGSL/f32, four-LUT production, joint rays, environment and whole-graph qualification
remain unbuilt. AP prefix certificates are optional future optimization, not a delivery prerequisite. The single
[migration queue](CLOUD-MIGRATION.md#current-assembly-checkpoint) owns integration and visual/cost proof.

## Checks and budgets

Probe actual GPU knots and held off-grid height/mu/world points against independent Gauss/f64 results; include top,
ground, inward visible rays, below-horizon planet block, lowSun5/15/30/75, local-height cancellation, malformed profile
and exact all-off. Pin shader/table/source identities and do not compare a different loaded host. Inspect native cloud,
receiver and independent reflection at low/high/front/back Sun, main-hidden and normal/accelerated source motion.
The existing new-source/lifecycle and full-sky gates remain separate.

Measure preparation, per-query and assembled workload.128KiB is a declared table size, not process residency; count
CPU/staging and old/new overlap. Continue comparing browser cloud5ms and full-frame16ms/512MiB, desktop3ms/10ms, and
named target devices; this M3-class standalone diagnostic certifies none. Prior native budget failures remain visible.
Feature-off resource retirement and final engine registration are later runtime gates, not claims from a fixture flag.
