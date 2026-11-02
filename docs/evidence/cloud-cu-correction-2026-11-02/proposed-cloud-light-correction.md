# Proposed source-only cloud-light correction

2 November 2026. Bounded design/reference study. Inspected the actual native `cu-1793573494117.png` and its capture metadata, current Cu source/cache/transport shaders and the retained CPU optical oracle. Consulted primary production documents below. Performed only small CPU analytic/angular-quadrature calculations; no GPU, candidate implementation, foreign code, canonical edit or commit. Preserve accepted Q1 and the rejected first passive-profile image/recipe as separate golden/negative witnesses.

## Diagnosis of this capture

The metadata fixes source age650s, variant0/seed1729, cloud base900m, support1216×544×922m, camera(1168,1453,2301)m and target(−4,1172,50)m. Sun elevation is30 degrees, direction approximately(−.7794,.5,.3775). The central view/Sun cosine is **−.02995**, a91.72-degree scattering angle. This is side illumination, not the strong forward silver-lining configuration.

At that angle HG(g=.86) is only.008645 sr^-1. The current fixed passive mixture contributes.012426 sr^-1 at Sun depth0,.002161 at depth3 and.0002154 at depth10. With90000 red-channel lux, the last two correspond to source functions of about194 and19.4 cd/m² before ambient. The external boundary is strongly blue ([6000,9000,15000] cd/m²), and the five-axis ambient cache attenuates that boundary through the whole cloud. Opaque regions lose the warm directional source while retaining some cool sky; a blue-grey body is an expected outcome. This is an analytic attribution, not a claim that every captured pixel has those depths.

The source coefficient reduction is substantial: the passive primary is.7 instead of1, and its later amplitudes are.1,.0175,.00375. It constrains integrated generated scattering to at most one incident beam. Physical multiple scattering can count the same surviving photon at many scattering events; its generated-event integral can exceed incident power while escaped power remains bounded. The passive rule is sufficient for a power bound but unnecessarily restrictive for high-albedo cloud appearance. Its measured semi-infinite upward flux ratio at5 degrees is~.2117, compared with~2.057 for the rejected unnormalized a=.8/b=.5 proposal. Neither number establishes a convincing cloud.

The smooth ellipsoidal body is separately a density/geometry rejection. A better source closure can reveal gradients and crevices; it cannot create missing billow silhouettes or turn a smooth union into convincing structure. Keep density and light revisions independently inspectable.

## What the primary production sources support

- Epic describes transmittance octaves, artist controls over contribution/occlusion/eccentricity, and a source-side cloud luminance compensation. This supports a bounded appearance approximation; it does not establish physical conservation or our chosen parameters. [Unreal cloud documentation, Light Multiple Scattering and Directional Light Interactions](https://dev.epicgames.com/documentation/en-us/unreal-engine/volumetric-clouds?application_version=4.27).
- Wrenninge's contrast approximation uses a source amplitude, phase and reduced extinction per bank. His2015 work distinguishes inexpensive contrast control from actual spatial diffusion, which can require many bounce events. Our octave notation below relabels his coefficient roles; it is not his full path tracer. [Art-Directable Multiple Volumetric Scattering, Eq1](https://history.siggraph.org/wp-content/uploads/2022/10/2015-Talks-Wrenninge_Art-Directable-Multiple-Volumetric-Scattering.pdf).
- Patry reports depth-dependent cloud HG behavior from forward g≈.85 toward backscatter g≈−.15, with simulation-derived backscatter compensation. This is evidence that a fixed forward phase alone is inadequate. His segment-transmittance depth estimator should not be copied literally into our camera/sample-independent source. [Real-Time Samurai Cinema, Atmospheric Lighting / Clouds](https://advances.realtimerendering.com/s2021/jpatry_advances2021/index.html#/96/0/8).
- Schneider/Vos explain a directional, depth-dependent powder effect and its limited viewing regime. It is a later appearance cue, not a substitute for missing diffuse body light. [Horizon cloud talk, pp59–68](https://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf).

The50MB Frostbite2016 PDF exceeded the web reader limit; I do not claim to have inspected its full formula. The proposal below is our explicit synthesis, not a studio implementation claim.

## Recommended first comparison: exact primary plus one fitted isotropic bank

Keep ordinary extinction, geometry, air/fog, Sun registry, all-source optical depth and Beer composition unchanged. Change only the cloud source:

`j_c(x,v) = sigma_s(x) { E_noncloud(x) [ p0(v·sun) exp(-tau_all(x)) + A(T_body(x),mu0) /(4pi) exp(-kappa tau_all(x)) ] + L_external(x) }`.

Select **kappa=.25** for this first liquid-Cu comparison. `p0` is the existing normalized HG(.86); the physical primary coefficient returns to1. The extra term is an isotropic effective redistribution bank, not another cloud, emitter or display multiplier. `E_noncloud` includes geometry and air/fog once and excludes cloud attenuation. Both exponents use the same **ordinary all-cloud** Sun depth. Primary receiver shadows still use exp(−tau_all), never reduced kappa depth.

This is a compact contrast/redistribution approximation. Its reduced depth does not model where a random-walk photon travelled, and its slab calibration cannot prove energy conservation for arbitrary finite/mixed geometry. The user's production-appearance goal permits that labelled approximation. Do not silently describe it as a physical multiple-scattering solver.

### Canonical world-column descriptor and light angle — LIGHT-R1 correction

Each independently owned condensate group has a source/recipe-defined, world-space orthonormal patch frame
`(a(t),e1(t),e2(t),up(t))`. Its origins/axes follow the same committed source trajectory and declared owner-up; they are
not chosen from the camera, a visible density normal, cohort dominance or current LOD. For the first unrotated Cu
fixture, `up=(0,1,0)`, horizontal axes are world X/Z, and a(t) follows the existing source anchor. In SKY-G a selected
patch may use radial up at its anchor; one common rigid rotation transforms its frame, light and coefficients together.
The patch's fixed physical-metre extent covers the owner's admitted lifecycle support. This parallel owner-up column is
a declared finite-patch geometric descriptor, not a globally radial column at every point of a spherical cloud.

Define the **canonical** descriptor from the complete transformed world coefficient query:

`T_body(u,v,t)=integral_s sigma_t,owner_world(a(t)+u*e1(t)+v*e2(t)+s*up(t),t) ds`.

The integral spans that line's entire finite owner support, uses world-length ds in metres and includes the admitted
source deformation/shear. The canonical sigma query already owns any material/world conversion; no extra guessed
Jacobian rescales extinction during this line integral. Its result is dimensionless optical depth.

Cu cohorts are max-unioned into **one owner field first**; do not sum thickness per lobe. **Sc/St M is a reference
material-column optical amount and is not generally this world-column integral.** With height-dependent F_h, a fixed
world column intersects different material columns. Integrate the transformed sigma query along the declared up axis;
reuse M directly only in an explicitly checked aligned, no-height-shear regime where equality holds. A scalar frame
conversion cannot establish it. A reference-column descriptor is a different possible approximation and would need
its own calibration/profile, rather than being silently substituted for this T_body.

Two LODs are one field. Genuine independent groups add ordinary extinction, but each group's source bank uses its own
T_body: a remote thick deck cannot make an ice filament's local descriptor thick. In a liquid/ice condensate, the
single owner's extinction and column are not duplicated by its scattering split; see explicit phase dispatch below.
All-source Sun tau still includes every group along the light path.

The column describes low-cloud bodies/layers, not universal diffusion distance. A narrow tower, tilted anvil, filament
or detached remnant above a parent can have the same column value and different lateral escape. Those are finite-cloud
appearance witnesses, not evidence that a missing phase or a changed column definition may be hidden in a fit.

Use `mu0=abs(sun·up(t))` for the symmetric reference slab fit. Its illuminated face changes sign with the local plane;
actual direction, primary phase, planet visibility and spherical light paths remain unchanged. Never use abs to
illuminate a shadowed source. At mu0=0 use the analytic grazing fit limit. Source grouping/frame/time/table revision
are world/recipe facts, independent of view, ray step, cache placement and camera LOD.

### Explicit first profile and whole-model dispatch — LIGHT-R2 correction

The first new bank is **liquid-Cu only**: primary normalized HG(.86), scattering albedo1, kappa=.25 and rho*=.8,
with a separately keyed fitted A table. Primary coefficient remains1.

The whole-model seam has an explicit initial ice path: normalized HG(.75), albedo1, **exact ordinary primary and
A_ice=0**. It consumes the same ordinary all-source tau and existing external ambient; no liquid table or phase is
applied to ice. Thus the first experiment has a complete defined ice fallback, while claiming no qualified ice
multiple-scattering approximation. The identical cooker parameterized by HG(.75) defines a later ice-bank candidate;
its distinct phase/albedo/table key must pass its own scalar/native thin/thick witnesses before activation.

For liquid and ice scattering weights sigma_s,L and sigma_s,I, evaluate

`j_c = E_noncloud { exp(-tau_all)[sigma_s,L*p_L + sigma_s,I*p_I] + exp(-kappa*tau_all)/(4pi)[sigma_s,L*A_L + sigma_s,I*A_I] } + (sigma_s,L+sigma_s,I)*L_external`.

The recipe owns the split; `sigma_s,L+sigma_s,I<=sigma_t` and ordinary sigma_t is added once. Initially A_I=0; a valid
liquid table can be used only for its declared albedo/phase. Missing or incompatible requested liquid products return
unavailable/error or use an explicitly selected primary-only comparison profile; they do not silently borrow the
ice/liquid table or fabricate clear sky. Mixed-source appearance/energy is not certified by a pure-liquid slab fit.
Changing ice fraction changes scattering weights under the source clock, not extinction ownership or event identity.
A primary-only ice owner needs no new A table/body-column product unless another retained bank consumer requires that owner descriptor; avoid scheduling that unused preparation. This preserves the intended exact thin primary separately for liquid and ice; it is not a claim that CF05 has passed.

## Complete coefficient producer and escaped-flux cap

The first liquid table uses albedo1 and HG(.86). Select an **authored opaque illuminated-face reflectance target rho*=.8**. This is a contrast/brightness target for an inspection profile, not a measured water-cloud albedo, nor a rule that conservative semi-infinite physical clouds reflect only80%. More physical/reference or art-approved targets can replace it through a newly cooked profile. Do not adjust it per camera to make a screenshot pass.

For a homogeneous slab of vertical optical thickness T and illumination cosine mu0, compute linear escaping-flux functions in units of incident horizontal beam flux:

- F0(T,mu0): unscattered transmission plus top and bottom escaped **ordinary single-scatter** flux.
- F1(T,mu0,kappa): top plus bottom escaped flux for unit isotropic reduced-depth bank, using ordinary view extinction.
- R0∞(mu0): illuminated-face semi-infinite ordinary single-scatter reflectance.
- R1∞(mu0,kappa): its unit-bank counterpart.

The upgoing contribution for arbitrary normalized phase p and source exponent b is

`U_b,p(T)=int_upper p(theta) mu/(mu0+b*mu) [1-exp(-T*(1/mu+b/mu0))] dOmega`,

where mu is outgoing normal cosine and `cos(theta)=sqrt(1-mu²)sqrt(1-mu0²)cos(phi)-mu*mu0`. The bottom expression uses the plus sign in cos(theta) and the formal-solution factor

`D_b,p(T)=int_lower p(theta)/mu0 * [exp(-b*T/mu0)-exp(-T/mu)]/(1/mu-b/mu0) dOmega`.

Use its continuous equal-slope limit, expm1 and factored exponentials; do not divide by a near-zero slope. Then `F0=exp(-T/mu0)+U_1,p0+D_1,p0`, `F1=U_kappa,iso+D_kappa,iso`. These are one-dimensional depth solutions with angular quadrature, not a runtime volumetric solver.

The isotropic semi-infinite result is analytic:

`R1∞=.5[1/kappa - mu0/kappa² log(1+kappa/mu0)]`.

Compute

`A∞=max(0,(rho* - R0∞)/R1∞)`,

`G(T)=(1-exp(-T))²`,

`A(T,mu0)=min[ G(T) A∞(mu0), max(0,1-F0(T,mu0))/F1(T,mu0) ]`.

At T=0 set A=0. At grazing, use continuous limits of these flux expressions; R0∞→.5 and R1∞→1/(2kappa), giving A∞→.15 for this profile. The finite-slab second argument is essential: fitting opaque reflectance alone exceeded total escaped power by~4.8% at30 degrees/T≈2.45 and~38% at75 degrees/T≈2.0 in the checked analytic samples. The cap budgets **escaped**, not generated, power. It permits repeated-bounce-sized generated source while preventing that specific reference slab from emitting more than its incident flux.

For T below1e−4, A=0 is an admissible numeric shortcut: the omitted bank source function is bounded by120000×3.724×T²/(4pi)<0.00036cd/m²/channel. This avoids cancellation in1−F0 near clear without changing first-order cloud transport. Negative residuals are recorded against convergence error; a nonfinite/invalid reference solve fails the cook. Do not disguise a failed solve with arbitrary coefficient clipping.

### Numerical table/cook, portable lookup and evidence level

Start with64 mu0 nodes including analytic0/1 boundaries and128 **explicit nonuniform physical T nodes**, including0 and
logarithmically placed1e−4…1000 samples. Above1000 use a separately bounded semi-infinite branch; kappa*T>=250 separates
its tail. Below1e−4 use the stated zero-bank numeric policy. Store A in R32float, initially32KiB before axis buffers;
R32float is not assumed filterable or a legal float-filtered binding on portable WebGPU.

Bind sampled R32float textures as unfilterable-float and use textureLoad; the lookup needs no filtering sampler. Body storage uses the portable write-only r32float storage role. Use manual bilinear interpolation. Locate the actual enclosing T_i,T_(i+1) in the stored axis, then
`fT=(T-T_i)/(T_(i+1)-T_i)`; similarly compute fMu from the actual stored mu0 nodes. A log locator may accelerate interval
search, but **log(T) is not the interpolation fraction** when the table is defined linear in physical T. The four
R32 values and nonnegative weights define the lookup. Axis lookup, sample-centre convention, endpoint behavior,
analytic branches, dimensions and table/axis generations are explicit hashed inputs. No hardware linear filtering or
assumed uniformly spaced T coordinate is hidden in the caller.

Precompute azimuth-integrated HG marginals per mu0 row, then integrate finite-T factors over outgoing cosine. This
reuses phase work across T. Compare128/256 outgoing-cosine and512/1024 azimuth quadrature; record convergence and
independent off-grid T/mu0 residuals, finite-cap kink behavior, grazing/equal-slope stability and escaped-flux error.
Start from these finite empirical checks rather than claiming that doubled quadrature certifies a uniform bound.

**The first native appearance comparison may use an empirically checked table.** A uniform deltaA<=2.5e−5 or exact
finite-slab energy certificate is not a user requirement or a gate on that experiment. Record its actual coefficient/
escaped-flux errors and label any uniform certificate unproven. A regular256² table may miss such a tight target at
the min/cap kink without making an honest art comparison invalid. Optional refinement has an explicit256²R32float
capacity (256KiB); failure to meet a future numeric target remains evidence, not permission to drop the required
appearance witness or demand a full solver. If a later numerical commission needs uniform certification, interval
bounds/remainders and conservative interpolation can be designed/qualified then. A possible subsequent smooth F0/F1/
A-infinity lookup with the final min evaluated at the query avoids interpolating the kink; it is not selected work now.

Hard validity/source bounds **do** apply immediately: every table value/lookup is finite, nonnegative and less than
3.724, with admitted axes/phase/albedo and valid generation. Validate rather than masking an invalid cook. Convex manual
interpolation preserves this finite coefficient range even when the reference-slab cap has interpolation error. Report
that energy error honestly; it does not compromise the separately derived global radiance bound.

Hash profile ID, kappa, target, albedo, normalized primary phase/parameters, complete axis values and interpolation
semantics, quadrature/fit version, precision/error policy and table bytes. Ice HG(.75) has its own key even with equal
kappa/target/albedo. All views use the same admitted source time, table and world-column generation. No frame-by-frame
fit or screenshot-specific normalization is selected.

Checked **analytic candidates**, not native image results:

| Sun elevation | A∞ for rho*=.8 | A at T=2 after gate/cap | Total escaped at T=2 |
| --- | ---: | ---: | ---: |
|5 degrees|.58125|.43457|.71016|
|15 degrees|1.22425|.91530|.85234|
|30 degrees|2.04441|1.46500|1.00000|
|75 degrees|3.57175|1.67535|1.00000|

At30 degrees/T_body=5, A≈2.01695. For the capture's side angle and tau_sun=3, that gives source factor≈.0763 sr^-1 versus current.00216: approximately35× more neutral Sun-driven body light. This is a prediction at a specified optical query, **not** a measured image brightness gain. It could be visually excessive in some geometry and needs the witnesses below.

## Production octave comparison and alternative

The usual contrast-style candidate has `sum_n a^n p(g*c^n) exp(-b^n*tau)` with primary1, rather than fixed w*b. With a=b=c=.5 and four terms, the capture side-angle factor is~4.83× current at tau0,9.56× at tau3 and20.1× at tau10. Its semi-infinite upward flux ratio is~.915 at5 degrees,.443 at15 and.105 at75; at2 degrees it is~1.314. Generated power greater than one is not by itself a failure, but escaped power greater than one diagnoses that reference approximation. Unconditional added banks also change the leading thin-cloud source; they need an explicit thin regime and further calibration.

A general multi-octave fit is viable, but has more correlated parameters and does not inherently solve spatial diffusion. The recommended one-bank model is the smaller first experiment: fit one A using complete top/bottom escape, preserve exact thin primary and isolate whether missing neutral diffuse source explains this image. If it cannot preserve gradients/undersides across geometry, do not keep adding banks indefinitely.

A credible next representation is a shared coarse incident-radiance cache with a **fixed small number of source iterations**: direct cloud source seeds a transport sweep; its angular mean produces the next isotropic cloud source; repeat2–4 times, then final main/reflected/probe queries read the same sum. No final probe feeds its own source. This supplies actual limited spatial redistribution and cross-layer exchange, but needs grid/transfer/opacity filtering and work/error admission. It is a separately costed fallback, not a mandatory full fluid or high-bounce runtime solver.

## Cost, bounds, dependencies and next witness

Runtime adds one A lookup, one scalar body-column lookup and one reduced-depth exponential/isotropic term; it removes three extra HG evaluations and two exponentials relative to the current four-bank mixture. Deep light queries need primary and kappa=.25 terms separately when footprint-filtered; the existing typed ordinary-tau and per-branch filtering semantics suffice. No new public engine API is necessary.

The first body cache is a128² **R32float2D compute-storage** product:64KiB before guards, manually bilinearly sampled
with textureLoad (core float32 bindings are not assumed filterable). A one-texel physical halo increases the actual
allocation; bounds and world-to-column coordinate/centre conventions are carried in the product. A128³ fixed-source
field reduction costs about2.1M coefficient visits per rebuild, reusable with source preparation but not free.

The cache **approximates** the canonical transformed-sigma line integral; neither current field samples nor camera-
selected LOD redefine T_body. The first source patch/frame/extent is fixed by the admitted owner recipe and time.
Compare cached T_body and resulting A against direct integration of the same transformed canonical query, including
height shear, source growth/retirement, map edges, rotation and interpolation. Record body/A/radiance error separately;
refine/direct or explicitly fail the requested cache quality if needed. Do not silently reuse reference M for sheared
layers or change descriptor when the camera moves.

Multiple ages/group poses do not share evaluated columns merely because they share a recipe. One generation joins
source coefficients, world frame/columns, phase-keyed table, light products and consumers; views cannot see a new
column with old geometry or a stale table. Main-hidden retained reflection/environment consumers still request the
products. CPU/GPU leases, new/old overlap, staging and preparation all count. Sc/St may reuse M only in the explicitly
checked no-shear/aligned equality regime; other cases integrate the transformed coefficient field.

For this kappa/target, R1∞ is smallest at mu0=1, so `A<=rho*/R1∞(1)<3.724`. With phase peak<=8, directional irradiance<=120000/channel and L_external<=960000, the new source ratio is below~1.956e6cd/m²/channel. Reserve **2e6** as a conservative bound if selected; update the source exit bound instead of silently retaining1.92e6. The2e9 solar terminal still dominates the composed maximum, so the existing shared2^-16 pre-exposure can remain within its stated numeric domain. All-light-off gives zero source; ordinary density/T persist.

Keep the finite DAG: shared coefficients/body columns/ordinary light depth → direct-only external air/fog ambient → corrected cloud source → final joint main/reflection/clouded-environment products. Do not restore blue by a per-cloud display colour, or brighten receiver direct light using reduced kappa shadows. Overcast surface diffuse light must come from the final clouded field; the current constant receiver term is still only a fixture.

Discriminating next witness, with density recipe and global display/exposure frozen. The narrow comparison uses
liquid-Cu plus the explicit primary-only ice path; the complete family commission remains unchanged:

1. Independent finite scalar/table checks: finite and semi-infinite slabs at2/5/15/30/75/90 degrees, T from clear to opaque, top+bottom+unscattered flux, thin leading order, source-off, grazing/equal-slope and actual LUT midpoint/cap error. Finite values/axes/phase keys/source bounds are required; a uniform escaped-energy certificate is not required before the labelled native appearance comparison.
2. Render the exact rejected camera/age with old passive, exact-primary-only and proposed source, then normal/front/backlit/underside angles. Hold Sun/sky units and display fixed; inspect raw scene-linear direct/ambient contributions and source-depth/column diagnostics. Brightness may improve while smooth-body rejection remains.
3. Use one finite puff, a thin slab/ribbon, a wide thick deck, narrow tower and two overlapping layers. Include a height-sheared layer where reference M differs from the world column, plus a common rigid-rotation/frame invariance check. Use ice HG(.75)/A_I=0 and a liquid/ice overlap to check dispatch; the separate ice-bank cook remains unqualified. Predict: thin liquid/ice retains exact primary; thick Cu gains neutral diffuse body light and identifiable dark gradients; low Sun retains warm rims without reference-slab escape blowup; overcast becomes brighter/less saturated inside while underside and transmitted diffuse light remain constrained. A false uniformly white puff, flattened contrast, light leak through unrelated layers or a bright thin ice bar rejects the candidate.
4. Check ordinary receiver tau/shadows unchanged, actual independent mirror source agreement, main-hidden/reflection-only behavior and whole-source wind/motion at1×/16×/64×. Measure incremental cache/table and full graph work after the appearance witnesses, retaining all adverse costs.

This revised boundary proposal supplies explicit corrections for LIGHT-R1/R2, awaiting the independent reviewer’s verification and actual qualification. It is a concrete first liquid-Cu comparison profile with a complete declared primary-only ice path, not an accepted whole-family lighting replacement. Its finite-slab escaped-flux discipline is a useful guardrail, not a reinstated exact-science objective or an arbitrary-geometry energy certificate.
