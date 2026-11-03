# Authored source replacement: finite material, persistent weather

3 November 2026. **Provisional candidate, with one reviewed Cu pilot commissioned.** This replaces neither the selected renderer nor the public provider contracts. Complete Cb/family authoring, general source control and full-sky performance remain unfinished. [CLOUDS](CLOUDS.md) governs scope, [capabilities](CLOUD-CAPABILITIES.md) define success, and [migration](CLOUD-MIGRATION.md) remains the only active checkbox queue.

The [failed transport sources](../../evidence/cloud-front-growth-2026-11-03/README.md), [qualified static Cu ingredients](../../evidence/cloud-authored-masters-2026-11-03/README.md), [local erosion comparison](../../evidence/cloud-geometry-operators-2026-11-03/README.md) and [rejected frozen storm anatomy](../../evidence/cloud-cb-anatomy-2026-11-03/README.md) constrain this proposal. Correct transport did not supply a good storm. A good static Cu and a correct erosion operator do not supply renewal or moving breakup. A connected storm skeleton and a filled smooth column both fail appearance.

## Operating model

A persistent E1 weather cell owns a bounded population of finite-lived authored volume elements, called cohorts here. Each is newly condensed material with a stable ID, template/seed, birth time, local growth, erosion, phase and material map. The cell and its weather/storm causes persist while new billows replace old material. No complete lifecycle movie, Houdini dependency or fluid solver discovering the anatomy is required.

Finite lifetime is structural. A permanent material master under wind W_x = s y accumulates shear s t indefinitely. Clamping or reversing its map cancels wind or introduces resets. A newborn cohort therefore starts with fresh rest coordinates and zero accumulated differential wind. It inherits the parent's socket position and initial world velocity, **not** the parent's deformation, exposure or wind history. Otherwise strain would accumulate through generations.

At birth, let u_0 = v_socket − W(x_socket,t). Thereafter C_dot = W(C,t) + u. The internal velocity approaches the recipe's uplift/outflow continuously. This applies shared weather wind once. Surviving parent material keeps its own map and age. Attachment is actual geometric overlap, not a recursive constraint that moves the child through the parent forever. Optical separation changes neither ownership nor velocity; existing cohorts continue moving independently.

The anatomy is an authored producer with required data. Templates need actual detailed density, distance, coordinate calibration, bounds, sockets and admissible growth/flow ranges. Cu ingredients alone do not define rooted Cb towers, a fibrous anvil, a connected deck or head-fed cirrus. Those recipes must be supplied and visually qualified; naming a family is not a generator.

## Shared field, phase and optical ownership

For a finite zero-collared template, let rho(a) be decoded density and d(a) positive-inside distance in local metres. A cohort contributes

`q_j(x,t) = A_seed · rho(a) · smoothstep(-w,w,d(a) − M(a)·(E_age(a,t)+E_weather(a,t)))`,

where a = Phi_j_inverse(x,t), M is fixed material structure in [0.3,1], and all exposures/distances use the same local units. Outside the actual template domain, return zero. Native half/trilinear interpolation defines the point field; the imported distance is an opposite-class voxel-centre approximation, not an exact continuous isosurface distance.

A_seed only ramps a tiny buried seed before visible geometric expansion. It is one before the seed emerges; mature elements never fade by this multiplier. New billows change their own support and map, without scaling the whole formation.

Within one authored formation, m = max(q_j). Phase partitions that amount: for m > 0, eta_j = (q_j/m)^4, I = sum(eta_j I_j)/sum(eta_j), q_liquid = m(1−I), and q_ice = m I. At m = 0 return zero. Independent formations add their nonnegative coefficients once. This is an art-directed amount/overlap rule, not conserved water. Do not reapply the rejected procedural material remap to calibrated density.

Phase must be spatial. Added amount uses the declared local condensation phase, then freezing converts extant liquid. If amount grows from q0 to q1, mix the survivor phase and the added amount's phase before freezing; eroded amount does not leave an invisible ice reservoir. Cold upper material can outlive the lower body, but cannot cancel the finite lifetime. A buried child can influence weighted phase before it wins the density max; its optical transition must be continuous and qualified.

Every main, light, receiver, environment and reflected query uses the same source identity, effective point field and presentation time. A hidden main view does not stop retained consumers. Public weather causes remain CPU/E1 facts; pixels never create rain, lightning or wind authority.

## Growth, renewal and guaranteed retirement

A new element begins at a certified interior socket with positive minimum scale. During the seed ramp its entire support stays buried. Once its seed density is established, small geometric features may emerge before they are independently resolved; this requires current sampling and visual inspection. Do not require the entire child volume to stay buried until a large neck forms: review proved that condition infeasible for the chosen master.

For established interior balls, a sufficient connection is

`min(r_parent, r_child, (r_parent+r_child−separation)/2) >= h_ref`,

with both balls above the declared attachment density over the establishment interval. Tiny positive tails do not qualify. Initial containment, later neck width and visible attachment are separate checks. Intentional adverse weather can cancel establishment and cause early breakup; that outcome cannot be counted as successful wet renewal.

The age exposure is irreversible. Wet weather may admit new cohorts, but cannot rejuvenate old ones indefinitely. Choose its positive lower rate/deadline so that, by T_max,

`M_min · E_age >= max_decoded_distance + transition_width + error_margin`.

Drying adds nonnegative exposure. Retirement follows actual zero contribution, never a lifetime cutoff that clips visible material. Dominated cohorts remain stored until genuinely zero because they may become visible again when a newer maximum erodes. The population bound is ceil(max_birth_rate · T_max) plus explicit establishment overlap. Exceeding it is overload, not silent suppression of required renewal.

## Material motion and family extension

The general candidate uses a monotone vertical map, positive horizontal scales and height-dependent wind displacement. The inverse, footprint Jacobian and prospective support must be explicit. Integrate differential wind at each label's actual evolving height. A finite age bounds history; it does not by itself prove acceptable strain under every wind.

The Cu pilot uses a simpler exact affine-height representation described below. Cb would require broad rising material, upper slowing/spreading and spatial freezing of that same material. Keeping the cap as the cohort's upper subset avoids a duplicated detached disk and ownership transfer. The exact cap map, directional anatomy and runtime programme are **not selected yet**. A flattened rounded Cu crown fails even if the phase arithmetic passes.

Sc/St require overlapping bank/sheet content with controlled seams and local holes. Ci requires head-fed directional tail anatomy and differential loss/fall/shear. They share source and optical contracts, not necessarily one shape generator. New authoring work follows published whole-volume shape/detail operations; static success still does not prove its runtime deformation.

## Sampling and cost baseline

Direct instanced sparse-template evaluation is a complete reference/default sampling path. Conservative world-brick candidate lists identify all contributing cohorts; evaluate their native point fields and formation max before transport. A global final-q volume or an unspecified adaptive up-rez scheme is unnecessary for mathematical completeness. A final-field cache can be a later measured optimization.

Static storage can use RG16FLOAT density/distance, 8³ cores and 10³ guarded slots. Core admission includes the one-texel dilation of positive density; merely padding nonzero cores loses cross-core trilinear support. Preserve exact addressing and outside-domain zero. Actual 256³ counts are 2,464 admitted cores for 03 and 2,927 for 08: useful slot payloads of 9,856,000 and 11,708,000 bytes, plus 131,072-byte indices each. These are measured data counts, not rounded physical atlas residency or GPU timings. Future dense sheets may be much less sparse.

MAX hierarchies provide conservative skip bounds, never average material. `max(filtered parts)` does not equal `filter(max parts)`. Direct native queries followed by ray/pixel quadrature avoid that false identity; convergence and reconstruction remain rendering gates. Any coarse field must average the already combined field and compare with the direct reference.

Count candidates per brick separately from cohorts per cell. Lists must be complete, with exact overflow traversal or explicit unavailable/over-budget status. A naive 1280×720×96 march with eight candidates would exceed 700 million cohort evaluations before light/reflection work. Occupancy, interval skipping, production view resolutions, shared light caches and actual overlap distributions are essential. The method can still fail cost admission.

Control lattice pitch follows mapped material-edge lengths, including shear. Account for CPU updates, uploads, current/prior states, template atlases, indices, light caches and transient swaps. Full mixed-sky population/control dimensions and the 96 MiB source pool remain open. No per-cloud dense 3D allocation or budget pass follows from one ingredient. The optional 20 MiB velocity cache remains a separate flow-provider experiment; this analytic producer allocates none when unused.

## Reviewed Cu interaction pilot

The [retained commission and review dispositions](../../evidence/cloud-cohort-design-2026-11-03/README.md) pin the implementation. This is a liquid Cu interaction, not a full source selection:

- One cell, at most eight preallocated cohorts, one shared 03/256 density/distance pair. No full old PDE/rest allocation or per-view source step.
- Tiny seed size 32 m; four growth-seconds of density startup, then geometric expansion through growth time 100 s. Growth time integrates a two-second wet/lift response. Replacements arrive at 130-second intervals while regional forcing permits them.
- Target scale 0.65–1; b = 4·target_scale·growth enters every map, inverse, wind integral and bound. New rest and differential-wind state start at birth; socket position/initial velocity are inherited correctly.
- Mean wind magnitude 0–12 m/s and height-gradient magnitude at most 0.002/s. Fixed 17³ exposure controls meet the 200 m pitch screen: the maximum mapped material edge is at most 2,523.7 m, giving 157.7 m node spacing.
- Age exposure is `endpoint·smoothstep(200,360,age)` in source seconds. Weather loss is one local metre per second times the persistent trailing dry profile. Wet weather does not erase accumulated exposure. The actual .04 attachment mask and all native guard vertices are independently checked.
- The dry front starts at source 420 s, with width 800 m and default speed 50 m/s. Its value inhibits admissions at the moving condensation region and growth at each cohort centre, while local material loss samples the full mapped lattice.
- One fixed quarter-second source clock and actual prior/current parameter interpolation feed every consumer. Presentation lags the **committed** clock by at most one interval; requested-clock backlog and display latency are separate. Rate 16 cannot discard state updates silently.

Actual native birth/growth, multiple replacement intervals, changed wind/front forcing and moving source-off remnants must be inspected, with normal and accelerated recordings. Source opacity/material diagnostics, independent reflected rays, receiver shadows, CPU work, uploads, resources and actual cadence accompany the beauty views. No alpha scaffold, periodic clone stack, frozen mature outline, reset, clipping or dissolve-only shutdown qualifies. Current implementation and appearance are unproved.
