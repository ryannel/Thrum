# Comparison review: revised five-family cloud model

1 November 2026. Bounded independent model/research comparison; no code, GPU work, canonical edits or commits.

## Scope and verdict

Read the complete revised `docs/design/modules/CLOUDS.md` and `CLOUD-CAPABILITIES.md`, retained source account and original commission. Inspected revision hashes before corrections:

- CLOUDS: `b2b7c7a38aed08bc305b9af87a9a74fb3f1affc7e57adc477ece9198fcaef459`.
- CLOUD-CAPABILITIES: `8c093f3f606216102f23fcbdf30c2177d561ccd34cd33ca5a42ec75ab5e5477a`.

The revised design covers the required families, weather paths and independent consumers substantially better than a universal local clip model. The status ledger correctly leaves new families and full sky unproven. **Two explicit equations do not establish their claimed property as written:** bounded erosion cannot remove cohort centres, and the proposed higher-scattering source can generate more escaping light than the incident beam. The parent supplied concrete corrections during review; both have defensible analytical resolutions under the conditions below. Additional anvil-coordinate, layer-coordinate/shear and Ci-support decisions must be explicit before the document is a complete operating model. None of this review establishes visual acceptance or device viability.

The review is a comparison with my saved independent proposal, not a fresh blind derivation. I had read the retained source index before that proposal, which exposed the former clip choice. I did not inspect existing CLOUDS or proof code until this comparison request. No new-family images, motion, source assets or executable were inspected. No conference recording was watched.

## What belongs to which proposal

My independent note recommended a layered world/event model and family-specific evolution masters behind one coefficient interface. It preferred prepared actual 3D content for difficult convection, anisotropic layers and persistent Ci cohorts; its illustrative convective flow was a prescribed streamfunction/advection/reaction producer, not the root's ellipsoid graph.

The root's new synthesis specifically selects an **evaluable authored cohort graph** as the first convective candidate, algebraic local column controls for low layers, and optical-normalized filament/emitter kernels for Ci. These are a different concrete first producer. Acknowledging that they might be replaced by a stronger prepared master is an appropriate fallback; it is not visual evidence for the chosen graph. The exact max-union threshold, radial anvil inverse, packed field layout, all-order same-depth light closure and `B_local` correction were root additions, not recommendations established by my earlier note or disclosures from the cited studios.

The low-layer normalization matches the earlier mathematical column model while explicitly changing M from mass per area to **dimensionless vertical optical depth**; that is a sound authored-proxy choice. The Ci units are also sound: `A_sigma=integral sigma dA` has units metres, and integrating a normalized 3D kernel over emission time requires an emission strength in m²/s. These successes do not close the remaining geometric/temporal operators.

## Findings

P1 means an explicit defining-property contradiction to correct before reviewed-design closure. P2 means an unresolved consequential operating choice or an overclaim needing a defined correction/qualification. A visually risky but mathematically complete candidate remains an unproven candidate, not automatically a design defect.

### CM01 — P1: bounded erosion leaves an immortal filled centre

**Location:** CLOUDS lines148–161, convective density and finite retirement.

The inspected formula is:

`rho=sigma0 smoothstep(0,e_width,m-eta(1-m)^p)`.

At every cohort centre `m=1`, the erosion multiplier is zero, so the smoothstep argument is always1 for finite eta. Increasing local drying cannot erase that centre. The formula protects the core for the entire lifetime, although the prose says protection is for birth and all density reaches exact zero at finite retirement. A terminal branch label does not itself supply the missing operator.

**Parent correction reviewed:** add a separate released erosion threshold,

```
rho = sigma0 smoothstep(0,e_width,m-eta(1-m)^p-D_i)
D_i = (1+e_width) smoothstep(t_stop+delta(q),
                            t_stop+delta(q)+duration, t)
```

This closes the defect if eta>=0, p>0, e_width>0, duration>0 and delta is bounded. At `t >= t_stop+max(delta)+duration`, D_i>=1+e_width and the argument is <=-e_width everywhere, including the centre. Spatial exposure timing delta makes this local threshold erosion rather than final-image alpha. Require the actual delta producer/allowed range in the recipe, and guard every radius component before matrix inversion.

Starting radius at zero with an explicit zero-field branch is well-defined, but its centre has a pointwise jump as positive radius first appears; integrated support/optical depth still tends to zero. If pointwise coefficient continuity is claimed, embed new filled daughters inside an existing saturated parent plateau or use a smooth local creation track. Merely starting inside the parent's nonzero support does not prove equal density there. Transport continuity is a reasonable admitted guarantee; do not call it pointwise C1 continuity.

**Disposition:** parent correction analytically closes finite terminal loss; canonical application and recipe witnesses were not yet verified at the inspected revision.

### CM02 — P1: the original higher-order closure is not energy bounded at low Sun

**Location:** CLOUDS lines328–340.

For the inspected closure with a=.8, b=.5, c=.5, g=.86, take a homogeneous conservative semi-infinite cloud, no air/fog/ambient, B_local approaching1 and normalized Henyey–Greenstein phases. Let mu0 be positive Sun elevation cosine to the slab normal and mu be the outgoing cosine. Ordinary view extinction and branch n give emergent radiance:

`L_n(mu,phi)=E a^n p_n(mu,phi) mu0/(mu0+b^n mu)`.

Hence upward escaping flux divided by incident horizontal flux is

`R=sum_n a^n integral_upper p_n(mu,phi) mu/(mu0+b^n mu) dOmega`.

CPU midpoint angular quadrature (512 mu intervals ×1024 azimuth intervals, four orders, no GPU) gives:

| Sun elevation | R |
| --- | ---: |
|90°|0.245667|
|30°|0.552284|
|15°|1.012706|
|5°|2.057162|
|1°|3.516604|

As elevation approaches zero, a normalized axial phase about a horizontal incident direction places half its probability in the upper hemisphere, so `R -> 0.5 sum_n(a/b)^n = 4.628`. This is an analytical counterexample, not a claim about an inspected rendered image. It demonstrates that local thin-limit suppression cannot bound thick-cloud energy in the low-Sun regime required by the proof. Retaining one ordinary tau across branches improves source/cache semantics but does not fix this effect; the reduced attenuation must be balanced by source weights.

**Parent replacement reviewed:**

`j_cloud=sigma_s E_noncloud sum_n[w_n b_n p_n exp(-b_n tau_cloud)]`,

with globally fixed nonnegative weights summing to1, `0<b_n<=1`, normalized phases and `0<=sigma_s<=sigma_t`.

For a pure-cloud branch along an incident beam,

`integral b_n sigma_s exp(-b_n tau) ds <= 1-exp(-b_n tau_total) <= 1-exp(-tau_total)`.

A weighted mixture therefore emits no more than the ordinary beam's removed power; adding ordinary unscattered transmission remains bounded by input. Ordinary view extinction can only remove escaped power. Common weights/b across all overlapping cloud constituents preserve the factorization. Spatial/family-dependent weights or different depth scalings per constituent generally do not inherit that proof and require a new bound. Other passive single-scatter media remain compatible because their ordinary-cloud attenuation is no larger than the corresponding b-scaled cloud attenuation used for each bounding branch. Additional ambient/other-light approximations need their own no-double-counting and passivity accounting.

The example weights [.7,.2,.07,.03] and b=[1,.5,.25,.125] have `sum(w*b)=.82125`. Thus the thin-limit source is about17.9% lower than the ordinary single-scatter coefficient, even as tau approaches zero. That is a **lossy effective illumination mixture**, not true primary scattering plus extra physical orders. Using the same normalized ice phase for every branch avoids extra thin-ice angular broadening. Qualify brightness and deep interior response visually; do not compensate with an unexplained albedo above1 or call it an exact physical solution.

**Disposition:** the weighted-b replacement has a sound passive direct-source bound under those constraints. Retain original golden behaviour separately and add finite/thick/grazing slab energy/reference witnesses for the new closure. Canonical replacement and visual quality remained unverified during this review.

### CM03 — P2: anvil launch-time inversion does not cover the admitted moving/sheared emitter

**Location:** CLOUDS lines170–179.

`t_emit=t-(r-r0)/v_out` is valid in a declared radial frame with monotone constant radial spread. The same paragraph admits upper drift/shear, tilt/asymmetric feeding and moving height/cap tracks. For a moving core emitter, material launched at t_emit starts at the emitter's **past** position, not its current position. A current-centred radius is generally insufficient to recover launch time.

**Correction:** explicitly select forward cohort strips/rings with conservative bounds, or define

`X(t;t_emit,theta)=X_emit(t_emit)+integral U_upper dt + radial_outflow(t,t_emit,theta)`

and solve the admitted one-to-one inverse. If folding creates multiple preimages, sum the independent material contributions or reject that regime. No code should assume the simple inverse remains valid merely because the desired r branch is monotone. In the constant axisymmetric case, an emission optical-volume rate Q_sigma (m²/s) gives vertical depth `M=Q_sigma(t_emit)/(2pi r v_out)`. A general launch map needs its actual area Jacobian `|partial_temit X cross partial_theta X|`; “includes the Jacobian” should identify this quantity and inlet regularization.

Pin an emission track that ramps from cap arrival and an inlet overlapping the upper core. Partition weights summing to1 prevent duplicate contribution but do not by themselves establish a connected/timely core-to-anvil field. This is a source-coordinate/producer closure, not a demand for a moist solver.

### CM04 — P2: world forcing must be pulled back into layer coordinates exactly once

**Location:** CLOUDS lines80–91 and186–194.

Front P is defined in world coordinates, while the column formula evaluates `P(chi,t)` in a co-moving coordinate without naming the pullback. If wind is10m/s, chi=x-10t, and a world front is P(x,t)=step(x-5t), directly substituting chi moves the apparent forcing front at15m/s. The design elsewhere discusses relative source/front motion, which suggests the intended operation, but the evaluable equation should state it.

**Correction:** `M(chi,t)=max(0,P_world(F_layer(chi,t),t)-theta+...)`, or explicitly define P_layer as this cooked pullback. Use the same map for birth admission, wetting/drying and inverse inspection. A small analytic moving-front fixture can independently verify arrival/departure and source identity before visual work.

### CM05 — P2: base-height layer drift needs an admitted shear approximation

**Location:** CLOUDS shared-wind contract versus line186 and column normalization.

The shared motion section says wind is evaluated at each material's height. The low layer instead follows wind only at its base, while a0.3–1.2km Sc bank is allowed under height-linear wind. Over a long story, an upper/lower velocity difference can accumulate kilometre-scale differential motion. Fixed base-coordinate detail does not automatically follow material at other heights.

**Correction choices:** declare a within-layer constant-wind approximation with position error bound approximately `|partial_y U| H Delta_t` and admit only regimes where it is acceptable; or use a height-dependent deformation map. With height-dependent horizontal shear, M's exact unit-integral vertical-column interpretation must be re-derived: a reference material column can shear into different world columns. Do not preserve the original normalization claim without naming which column/coordinate it describes. This can be resolved as an authored approximation and a strict wind range; physical fine-scale turbulence is not required.

### CM06 — P2: Ci's stated finite compact support contradicts its Gaussian formula

**Location:** CLOUDS lines210–221 and source-bound/culling contracts.

The Gaussian transverse kernel is nonzero at every radius, whereas the source contract and paragraph call it compact. A finite hierarchy/support bound therefore needs a selected cutoff/error policy or a genuinely compact kernel. Otherwise “conservative source bounds” cannot be literal.

**Correction:** use a normalized compact polynomial/spline profile, or truncate/renormalize at k widths with a stated optical error. A2D radial Gaussian's omitted cross-sectional amount is `exp(-k²/2)`, but that mass fraction alone is not a grazing-ray transmittance bound. Bound the omitted sigma times the longest admitted relevant path, or test an independent ray integral including near-parallel views. The3D continuous-emitter variant requires its own normalization/truncation, not the2D factor copied blindly.

The line and continuous-emitter forms also need an explicit conversion if they are meant to represent the same content: emission strength divided by the launch-time-to-arclength Jacobian sets line amount. Alternatively declare them separate authoring modes with independently calibrated A_sigma; do not assume curve tessellation or emission-time resampling preserves optical amount automatically. Width/spacing rules then test convergence of the selected evaluator.

### CM07 — P2: mature low-layer renewal needs actual time-varying tracks in the selected recipe

**Location:** low-layer source formula and CF03/CF04 mature-behaviour gates.

The available operators can create renewal, but a fixed C(chi), a steady potential behind a passed front and fixed source/dry tracks produce an exactly translating frozen low layer during a hold. The current prose identifies wetting/drying and a persistent cell pattern but does not give a required mature-renewal schedule comparable to staggered Cu daughters or continuous Ci emission.

**Correction:** pin representative asynchronous compact source/drying episodes per persistent cell/region in the first source recipe, including a mature interval whose local column amounts/holes change while aggregate coverage remains roughly steady. Keep source/material IDs and cell scale stable; do not reseed C as a shortcut. St may legitimately evolve subtly, but a normal/accelerated motion witness should distinguish weak real renewal from a static translated sheet. This is a missing concrete proof input, not evidence that the column representation is incapable.

### CM08 — P2: B_local's thin-limit statement needs its exact scope if retained

**Location:** CLOUDS lines334–337.

For uniform dilution sigma=epsilon sigma_ref at fixed finite ell_opt, `B_local=O(epsilon)` and added nth-order source is O(epsilon^(n+1)); the claimed limit is correct in that regime. It does not prove a geometrically thin anisotropic source is treated correctly when ell_opt is an unspecified “cohort scale”, nor that equivalent source decompositions yield the same light. Max-union dominance changes between cohorts of different sizes can change ell/B while sigma remains continuous.

**Correction:** if the parent adopts the passive weighted-b replacement, this finding becomes superseded rather than a requirement to keep B. If retained elsewhere, define an anisotropic, source-partition/LOD-stable optical-length query and its mixture rule; qualify both geometric-thinning and density-dilution witnesses. “Camera independent” is necessary, not sufficient for a unique/invariant local closure.

### CM09 — P2: retained research account still presents superseded selections as current

**Location:** `docs/design/research/CLOUD-SYSTEM-SOURCES.md` heading, “What the synthesis adds” and alternatives.

The retained source account says it supports governing CLOUDS, selects a finite final-density library and avoids inventing runtime material-detail maps; it calls lobe kernels a stylized alternative rather than the reliable default. Revised CLOUDS selects a lobe/cohort candidate and new material/detail/light machinery. The new document appropriately acknowledges its risk, but the undated/current-looking research account could be mistaken for evidence supporting that selection.

**Correction:** label the old account as the historical local-clip decision/source record and link it to CLOUD-LOCAL-CLIP. Append the new candidates/selection reasons, actual primary-paper locators and unresolved visual claims. Explicitly attribute graph, column, Ci kernel, B/weighted-light and curved-world combinations to the present synthesis. Production decks demonstrate useful components; none inspected here establishes this complete authored lifecycle system.

## Research and reference comparison

The earlier independent note contains the substantive primary-source inspection and exact links. The consequential distinctions remain:

- Webanck et al.2018 §§4–5 and supplement disclose family-specific procedural fields and wind-aware cover morphing. Their shrinking/fading/noise animation does not establish the new daughter-billow, anvil-transfer or mature-renewal claims.
- Goswami/Neyret2017 §§5–6 gives actual parcel entrainment/detrainment and amplification, but its conclusion leaves billowing as future work. It cannot certify the root's sphere-cohort appearance.
- Stormscapes2020 §6.1/Fig.12 supports cap-driven vertical development/outflow as a reference; §7.1 explicitly excludes cirrus/ice. It does not validate an authored radial inverse or the Ci source.
- Vimont2020 §§4–6 supports layered source reasoning; Table4 and §§7.3–7.4 do not establish a game-frame runtime. Its upsampling/animation is distinct from the root's optical column closure.
- Thunderscapes arXiv v1 §§4–5 identifies stronger hydrometeor/electrification producers, not obligations to build them for the authored profile.
- WMO's family remarks support the required observable distinctions: cap-limited Cu/Cb spreading, fibrous upper Cb/anvil, fragment-to-sheet St, and Ci heads that can vanish before trails. They are observational criteria, not algorithms.
- Epic's volumetric-cloud documentation and retained Nubis/Smolder slide inspection support approximate transport/resource practices. They do not disclose the exact B_local or proposed passive-mixture closure. Use actual higher-sample/oracle and visual references for the new closure.

No source index, correct equation, plausible still or cross-review agreement establishes the five-family output. Keep the ledger's unproven status until actual native and normal/accelerated motion passes.

## Recommended disposition

Correct CM01/CM02 and make CM03–CM06's coordinate/support choices explicit before closing D0. Pin CM07's actual mature tracks in the first isolated recipe. Supersede or narrow CM08 and reconcile CM09. Preserve the valid parts: family-specific source dimensionality, normalized optical columns, explicit Ci units, source-group distinction, common world clock, independent consumers and honest unproven/fail statuses. Then prove the candidate graph without temporal concealment and retain prepared final-density content as a qualified alternative at the same seam.

This review does not require scientific meteorology, a mandatory live PDE or exact full multiple scattering. It requires that the chosen authored operations actually have the declared finite/continuous support, shared coordinates, source accounting and bounded-light properties. Perceived cloud behaviour remains an empirical gate.
