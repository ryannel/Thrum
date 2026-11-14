# Interactive water dynamics

**14 November 2026: retained as research.** Thrum's water is now designed as one system in
[WATERS' design](../../plans/WATERS/DESIGN.md), which judged this document as evidence and is the authority for
how water works. Nothing below is commissioned.

1 November 2026. **Selected engineering recommendation for a wider, bounded provider; unimplemented and
unqualified.** This is D2-water B beneath the [water commission](WATER.md), not a replacement for the smaller
[initial sea/shallow specification](../PLAY-ENVIRONMENT-SPEC.md). Relationships labelled derived or authored below
are Thrum's adaptations, not claims about private engines or guarantees inherited from a paper.
Per the [water decision](../decisions/E1-GPU-WATER.md), the methods below run as GPU compute and are the water
authority; bodies read them through an asynchronous batched query. Exact replay is not required.

## Representations and selection

| Role | Selected mechanism | Reason and real limit |
| --- | --- | --- |
| Prescribed far water / cheap pond | Retain the initial analytic/prescribed provider | Low-cost headless queries; no claim of finite storage or interactive wake feedback |
| Connected interactive graph water | Complete generalized-heightfield bulk/dispersive/transport method, GPU authority | One graph state supports current, wakes, finite storage and shore response; not overturning, 3D vortices or arbitrary thin obstacles |
| Floating-body disturbance | Geometry-derived, reference-immersion pressure surrogate, plus geometric P3 buoyancy/drag | Produces queryable waves with modest state; explicitly nonreciprocal and not exact rigid-hull exclusion |
| Detached flow | Finite discharge intervals and ballistic transit, earliest receiver contact and accounted return | A genuine donor/transit/receiver connection; no general 3D splash or waterfall-volume solver |
| Thin rain/puddles | Separate connected-triangle volume/head flow, below | Basin filling and exchange over preconnected wet edges; advancing runoff over dry slopes remains unselected |

The interactive domain covers the whole swept hull, receiving pool and relevant shore closure. No automatic moving
patch, hidden periodic recycling or traversable analytic/HLL/generalized-heightfield seam is selected. Same-provider
connections use native faces. A prescribed exterior is a named reservoir with signed volume/work accounting;
cross-model traversal stays blocked until a separately derived operator exists. This is a deliberate bounded first
interactive product, not completion of open-world or all-regime water.

## Published graph-water core

Select [Jeschke/Wojtan 2023](https://research-explorer.ista.ac.at/download/14240/14725/2023_ACMToG_Jeschke.pdf),
§§3–5 and appendices, with the [author corrections](https://visualcomputing.ist.ac.at/publications/2023/GSWSDSW/).
Keep its depth-aware bulk/surface decomposition, native bulk operator, dispersive update, residual-height/discharge
transport and recombination together. It is not a shallow solver with an independent ripple overlay.
Author corrections to equations 13/17/19–21/27 are mandatory: gradient constraint, subtracting divergence, signed-flow
treatment and corrected dispersion. The published boat example does not supply our body operator.

Stored staggered discharge is not full 3D velocity. Total continuity flux includes
`Q = q_bulk + q_surface + h_residual u_bulk`. Publish common-time height/derivatives and this flux, with actual
integrator phases retained privately. Numerical boundaries, unresolved bed/thin-wall leakage and discarded fine
vorticity remain limitations. Replacing the paper's velocity clamp with admitted substeps/failure is an adaptation
requiring separate qualification. Follow the complete cited operators rather than reconstructing them from this summary.

Do not change this storage law to add subgrid puddle volume or replace the bulk scheme while retaining the source's
claims. Fixed topology, transform/filter plans, depth-bin symbols and boundary products are prepared once where
valid; state-dependent coefficients are repartitioned explicitly. One shared solver serves all bodies and views.

## Body source: an independent reference, not a feedback controller

F4 cooks nonoverlapping body volume elements, exterior drag geometry and a compact nonnegative pressure kernel
`K_h`, with `∫K_h dA=1`, justified width and preserved moments. P3 owns body pose/velocity and integration. E1 owns
pressure/wave state. The source uses a reference interface independent of the interaction response:

`d_ref,h(x) = submerged hull-column depth against eta_ref`,
`pi_h = K_h * d_ref,h`, `p_h = rho g pi_h`, `∫p_h dA = rho g V_ref,h`.

`eta_ref` is a reference mean stage plus an independently prescribed incident field when selected. It is **not**
`eta_total + pi_h`. Using that self-corrected force surface to generate its own pressure introduces an unintended
integral pressure controller. `V_ref` and the actual force-query submerged volume are distinct quantities in waves.

For finite water, derive the reference mean `H_ref` from actual liquid inventory, fixed bed and reference body poses:

`V_liquid = sum_i A_i max(0, H_ref - z_i - sum_h pi_h(H_ref,pose_h))`.

Use a bracketed scalar solve; require `dV_liquid/dH=sum_active A_i[1-partial_H sum pi_h,i]` to retain a positive
margin over the whole bracket. Waterplane area below half the **active wet area** is a sufficient bound only for wholly
wet, normalized pressure supports; dry banks cannot inflate that area. Update it after actual inventory/pose changes
without overwriting physical water height. At calm admission,
solve body equilibrium and this datum together. The liquid volume remains unchanged by adding a hull; the mean rises
through excluded-equivalent pressure volume. Runtime spawn/removal changes the source and continuity response;
it does not reset the whole surface or debit liquid a second time.

Dynamic inventory normalization is explicitly a **quasi-static, nonlocal reference approximation**: a remote inflow
can change another hull's reference pressure before its physical waves arrive. At fixed current pose isolate
`Delta pi_datum=pi(H_new,pose_current)-pi(H_old,pose_current)`, and bound its force effect by
`|Delta F_buoyancy,h|<=rho g A_max,h ||Delta pi_datum||_infinity`, where cooked A_max bounds the horizontal hull
section over the entire affected load-height interval. Buoyancy torque is bounded by R_max times that value.
For **total** force/torque, evaluate P3 loads counterfactually at identical current pose, velocity and physical fluid
sample while varying only the datum; hold incident phase fixed. Check both each-stage change and deviation from the
admission datum against authored force/torque/source-validity tolerances.
Reject updates outside that tolerance; do not silently low-pass away required inventory or claim exact finite-speed
load causality. Local reference estimation is a separate unselected improvement. The physical wave operator still
propagates its disturbances; the reference correction is not one of those waves.

The pressure surrogate is motivated by [Bao et al. 2026, §2.2](https://dcwan.sjtu.edu.cn/userfiles/1-s2_0-S0029801826018147-main.pdf).
It is not exact hull pressure. [Yao et al. 2024, §§2,5–8](https://www.cambridge.org/core/journals/journal-of-fluid-mechanics/article/influence-of-static-versus-dynamic-pressure-distribution-strategies-for-modelling-nonlinear-waves-generated-by-ships/61331ACF6FD86F38DD4AFD291462257F)
shows why static surrogate shape/volume can drift during motion; their dynamic-pressure enforcement is not silently
included here. Compare effective indentation volume/shape to the admitted geometry and reject outside the profile.

## Pressure-inclusive rest state and sampled force

The native gradient-constrained filter is nonlinear; a symbol L alone cannot make its pressure split linear. Select
an explicit frozen-coefficient substage: derive equation 13 face coefficients from the current **physical depth** with
the corrected gradient constraint, then freeze face averaging, wet masks, compatible ghosts and the fictitious-time
pass schedule. `L=product[I+delta_T Div(alpha D)]` uses that identical schedule for eta, pi and discharge, or evaluates
`r=eta+pi` directly. Each pass preserves constants. Never recompute coefficients independently from pressure or a
partly filtered input. This is a Thrum adaptation, not the unmodified paper's stability guarantee.

Derive bulk force potential from `L(eta+pi)` and dispersive force potential from `(I-L)(eta+pi)`. Physical filtered
depth remains `L eta-bed`, with physical residual/continuity unchanged; pressure is not liquid storage. Thus
`eta+pi=H`, zero flow is a pressure-balanced rest state under identical support/masks/coefficient conventions.
A changing split requires conservative state repartition, not a new seed. This exact rest argument applies to a
fully wet admitted domain. Dry cells have eta=bed, not H; freezing masks alone does not establish constant extension
through a global dispersive transform. Native shoreline rest/leakage therefore needs its own numerical qualification;
no exact wet/dry well-balance is inherited. Positivity/stability of the frozen-stage adaptation is also unqualified.

For a frozen Airy mode, the forced-oscillator displacement contains `-pi(1-cos(omega dt))`, and its velocity contains
`-omega pi sin(omega dt)`. A sine-only impulse appended to an unforced update misses static indentation. Derive the
staggered displacement/discharge updates from this shifted equilibrium, applying high-pass pressure once.

Body `h` samples `eta_load,h = eta_total + pi_h,applied`, using the **pressure actually represented by that committed
endpoint**, not a newly recomputed footprint. This subtracts an instantaneous equilibrium particular solution;
it does not measure or remove the actual transient self-field. Own radiated history and other bodies' waves remain,
with a transient load bias that must satisfy the admitted source-rate/force-error bounds. Clip transformed P3 volume
elements against the load surface, compute Archimedes
force/centroid and flow-relative drag, and apply their torques once. Hull identity never selects a second body solver.

Source generation uses `eta_ref`; force sampling uses `eta_load`. The separation is an authored production closure,
not a reciprocal no-penetration or added-mass solve. Record body force/torque impulse, pressure work
`W_p=-sum p_i Delta eta_i A_i`, drag loss `-F_drag dot v_rel dt`, and the unresolved external exchange separately.
Administrative ledger closure does not establish physical energy conservation.

## Flow, contact and order

The physical query returns its domain/epoch/tick, graph height, derivatives, represented support/error, depth and
effective horizontal material carrier `U=Q/h`, with dry handling. Select the modelled depth-uniform column carrier
`w(y)=U dot grad(z)-(y-z) div(U)`; this is not resolved vertical circulation. For surface influx s in m/s,
`h_t+div(hU)=s`, hence its surface limit is `w_fluid=eta_t+U dot grad(eta)-s`. Injected material may cross that moving
boundary. A geometrically attached foam carrier instead uses `w_attach=eta_t+U dot grad(eta)` and is explicitly a
presentation closure on source supports. Propulsor momentum changes Q, not s. Optional orbital products
must use the actual wave depth/bin convention and expose their separate approximation/cost.

Inside one declared E1/P3 substep: query the staged common endpoint; derive reference pressure and loads; form
provisional **fluid and P3 trajectories** with those sampled loads; find earliest transit/solid/water events against
those same candidates; rollback and recompute all affected candidates to that event, apply the transfer, then advance
the remaining interval. P3 remains the sole body integrator. Validate all candidates.
Publish only the complete world tick. Applied pressure identity and sampling lag are part of the endpoint. No public
consumer sees intermediate substeps. The [integration design](WATER-INTEGRATION.md) owns candidate, lease and failure rules.

Uniform-current versus translated-source equivalence requires both residual height and discharge advection. Test
it in constant depth with matched forcing and frames. Do not extend it to shear, strong vorticity or arbitrary bed
variation. Fixed rocks use the resolved bed/bank topology; a fixed hull surrogate uses the same pressure law and
relative-flow facts. Foam does not supply a missing standing-wave operator.

All delivered horizontal momentum is scattered onto the native staggered faces using their component dual-area
measure, not an ambiguous cell average. Nonnegative weights preserve impulse; first moments locate its application
point or an explicit unresolved moment reaction is recorded. At frozen face depths, a requested total-flux change
`delta Q` can be represented by `delta q_bulk=(h_bar/h_total) delta Q`, with q_surface fixed, because
`Q=q_bulk+q_surface+h_residual q_bulk/h_bar`. This preserves the intended impulse, unlike changing q_bulk by delta Q.
Repartition with the same admitted filter/time phase before transport, preserving h and total Q. Dry or singular
faces reject the source. Propulsor and impact deposits use this same operator, not private current fields.

## Finite fall, moving receiver and continuous arrival

Release one cached, positivity-limited donor-face volume over its actual release-time interval. A transit cohort
stores that interval, volume/flow, position, velocity, ancestry and remaining budget. Its paths satisfy
`x(a,t)=x0(a)+v0(a)(t-a)+0.5 g_vector(t-a)^2`. Connected tube cross-section is
`A=Q_release(a)/|partial_a x(a,t)|`; sheet thickness is `A/width`. The familiar `Q/|v|` follows only for stationary
release whose trajectory depends on age. Detect vanishing release-coordinate Jacobian, overlaps or folds; switch to
admitted finite transit geometry or report unsupported, never render infinite thickness from a singular division.

Find the first CPU solid or staged moving-water hit with swept support and bracketed intersection. Shorten and
recompute a provisional substep to that event, batch ties by stable IDs, deposit, then advance the remainder.
Bound refinements/events; exhaustion rejects the candidate, never drops liquid or substitutes an old surface.
At solids, actual liquid goes to the selected surface-storage/drainage owner; no automatic optical splash credits water.

At water, `Delta V = Delta V_receiver + Delta V_spray`. Credit only the first term now; retained spray stays airborne.
Cache incident momentum `M_in=integral rho Q_in v_in dt` and retained outgoing spray momentum
`M_spray=sum rho Delta V_spray v_spray`. Credit `(M_in-M_spray)_horizontal` through nonnegative unit-sum wet receiver
weights. Define unresolved normal arrival excess
`J_n=max(0,-[M_in-M_spray-rho Delta V_receiver u_receiver] dot n)` against the sampled receiver carrier.
The admitted wave fraction and remaining normal/bed reaction partition this excess; upward spray can increase it.
One ancestry/consumption record drives geometry, sound, wetness and whitewater.

Crucially, cohorts are intervals, not 60 Hz impact impulses. Map release time `a` to first arrival `t_hit(a)`.
Integrate volume/momentum over the preimage of each receiving substep; where invertible,
`Q_arrival(t)=sum Q_release(a)/|dt_hit/da|`. Otherwise integrate the finite intervals directly. A steady jet on a
steady receiver gives delayed steady flow and mean traction `p_mean=(admitted normal excess rate) K`, not a new ring
each tick. Without retained spray or receiver normal motion its unscaled value is `rho Q_arrival v_normal K`.
Changing flow, location or source onset naturally radiates waves. Source-off stops release, not transit or old waves.

For a genuinely isolated impact, use the same residual J_n (reducing to `rho Delta V max(0,-v_relative dot n)` without
retained spray), and `p=alpha J_n K(x) f(t)`, with
unit-integral spatial/temporal kernels. In constant-depth linear theory its wave-energy estimate is
`E_wave=alpha^2 J_n^2 <K,G K>/(2rho)`, `G(k)=k tanh(kH)`. Use the solver's discrete response for a discrete claim.
Bound `alpha` by the remaining arrival budget after credited flow, spray and air work. Never apply this impulse in
addition to the same cohort's continuous mean traction. The pressure adds no second volume.

Optional unresolved impact fluctuations require a separate, bounded zero-mean traction closure, below; steady
height oscillation or a reset ring is not the default substitute. [Whitewater](WATER-APPEARANCE.md) shares the one
arrival-energy allocation and does not apply another physical wave impulse.

### Optional continuous-impact fluctuation closure

Select a calibrated traction closure, not an inferred turbulence solver. With the same discrete unit-area footprint K,
use `psi_j=K[cos(k_j dot (x-c))-mu_j]`, `mu_j=sum A_i K_i cos(k_j dot (x_i-c))`, so each mode has zero spatial mean.
Use fixed source/world phases, not cohort age: `p_0=A(e) sum beta_j psi_j cos(omega_j t+phi_j)`, with dimensionless
beta and `A(e)=A_0 sqrt(max(0,e)/P_ref)`. Filtered arrived power e and P_ref are W; A_0 is N and the pressure is Pa.
Choose A_0/P_ref as explicit calibrated source-profile parameters, not universal turbulence constants. Start with four modes,
`lambda>=8dx`, `omega dt<=pi/4`; amplitudes/frequencies and footprint are authored physical-profile parameters.
The envelope e filters actual arrived available-power history with normalized causal
`k_T(s)=30s²(T-s)²/T^5`, `0<=s<=T`. Its finite smooth support has spectral sidebands; it is not exactly bandlimited.
The source keeps the corresponding finite arrived-energy store. No envelope spends future arrivals.

For an actual candidate gain a, mean plus fluctuation traction is `p=p_mean+a p_0`. Choose
`0<=a<=min(1,min[p_mean/(-p_0) where p_0<0])`, preserving nonnegative total pressure and integrated vertical graph
traction. Sloped-surface vector/moment changes remain in the actual receiver/bed reaction ledger.
The same frozen pressure-response substage gives a cheap work predictor. If `Delta eta(a)=Delta eta_0+a delta`,
`delta=R_eta[p_0]`, then its pressure-work increment is `c_1 a+c_2 a²`, with
`c_1=-<p_0,Delta eta_0>-<p_mean,delta>` and `c_2=-<p_0,delta>`; brackets include cell area.
The cross term matters. R is the actual bulk-gradient plus forced dispersive update with frozen depths, symbols,
decomposition, masks, face depths and transport predictor; use its corrected staggered/time-centred symbols and zero
response for k=0. It costs one extra stencil/batched response transform, not a nonlinear solve for every scalar gain.

Solve the scalar feasible quadratic interval against remaining arrived work, then check **actual** staged work and
state constraints after nonlinear transport. Permit at most two reduced-gain recomputations, then use the validated
mean-only candidate (`a=0`). Required mean failure rejects the whole candidate. Debit positive mean and incremental
work once; negative work is not new arrival energy. The quadratic is a substage predictor, not a proof that the full
generalized-heightfield step is linear. Limiting is recorded as source/model stress; tune admitted amplitudes so normal
operation needs no limiter pumping. Source-off leaves old fluid waves; this optional traction cannot sustain itself.

## Thin rain and puddle provider

Select a bounded friction-dominated, gravity-driven **storage/exchange** approximation, independently of the interactive
graph solver. Prepare gentle-slope connected terrain triangles with a well-centred orthogonal Delaunay/Voronoi-compatible
dual; reject skew/nonorthogonal support rather than inherit continuum accuracy from a two-point graph. No cell may hide
a sill or disconnected basin. Each node stores actual liquid `V_i` and exact piecewise-planar hypsometry
`V_i(eta)=∫triangle max(0,eta-z(x)) dA`, using **horizontal projected area**. Invert it monotonically. The same clipped
geometry supplies wet area without whole-cell switches. Edge length and dual distance below use the horizontal metric;
the gentle-slope thin-film reduction omits O(slope²) metric terms.

Across a shared terrain edge, with dual length `ell`, use the derived lubrication conductance
`C_ij = g/(3 nu ell) ∫edge max(0,max(eta_i,eta_j)-z(s))^3 ds`, `Q_ij=C_ij(eta_i-eta_j)`.
`C` is symmetric/nonnegative in m²/s; `Q` is m³/s. Sills are the actual connected wet intervals of that edge.
Rain, evaporation, infiltration and exports have named finite volume/mass stores and actual fluxes.

Admit inter-node exchange only when the receiver's wet support already touches the shared incoming wet edge. Otherwise
water credited to a hydrostatic triangle can appear in its low corner across a dry slope: exact volume/energy balance
would still produce the detached patch the brief rejects. Basin rain filling is supported; general overflow advancing
over dry slopes needs an intra-cell film/transit representation and remains **structurally unselected WAT-06**.
Do not silently block that required experience and call general runoff complete.

Use deterministic edge-colour sweeps, recomputing heads per edge. For a higher-head donor, transfer
`delta=min(dt C Delta eta, V_donor, delta_equal_head, delta_close_wet_edge)`.
Solve the equal-head cap from the two nodes' combined hypsometry; the closing cap stops at the donor's shared-edge
sill. Cache one transfer with opposite signs. Nonsharing edges in one prepared colour may execute in parallel;
alternate colour order deterministically. This is a first-order split graph scheme with declared bias/error, not HLL.

It preserves nonnegative volume. For `E=rho g sum_i ∫_0^Vi eta_i(v) dv`, each edge transfer stops before heads reverse,
so `Delta E<=0` for the unforced step; donor limiting alone would not prove that. Rain/sinks/friction work are separate.
Accuracy admission limits per-substep head/volume changes and requires bounded substeps; large stable timesteps are
not a claim of correct runoff speed. Optical smoothing is a bounded residual; wet geometry still comes from storage.
Because adjacent triangle heads may differ, the raw storage surface is not a globally smooth sheet. Cooking/admission
must bound resolved head jumps and render reconstruction position/wet-area error; refine terrain support or reject the
profile when those bounds fail. A smoothed normal cannot conceal an unqualified step or alter stored volume.

Admit only `h/L<<1`, `U h²/(nu L)<<1`, viscous relaxation `h²/(3nu)<<forcing time`, and gravity-dominant resolved scale.
For water `nu≈1e-6 m²/s`, a 1 cm layer relaxes over about 33 s: this model cannot claim fast deep runoff. Capillary
pinning/droplets, inertial flooding, floating bodies and general steep flow require another admitted provider.
The [lubrication reference](https://www.princeton.edu/~stonelab/Publications/pdfs/From%20Howard/PhysicsFluids/SnoeijerLeGrandLimatStoneEggersPhysFluids07.pdf)
supplies lineage; this discrete graph/limiter is our derived reduction. The [October 2026 subgrid SWE preprint](https://arxiv.org/html/2601.01895v2),
§4.2.2, explicitly warns about unresolved barriers and steep-slope redistribution; it is not adopted as a magic puddle fix.

## Admission, cost and open capabilities

Initial interactive bounds are hypotheses: `lambda>=8dx`, pressure length/beam at least `4dx`, wholly wet pressure
support, head `pi<=0.2 depth`, nonoverlapping pressure supports, residual `ka<=0.2` outside declared breaker support,
hull depth-Froude at most 0.5, resolved smooth bed/thick obstacles and measured surrogate shape/volume error within
the admitted tolerance. No boundary-clipped renormalization may silently change a moving source. Smaller regions or
more efficient kernels may qualify; no cell count or speed has been certified.

Count `substeps × [cells × filter passes + depth bins × cells log(cells) + advection + hull elements + event recomputes]`,
plus source solves, pressure footprints and staged/leased products. GPU execution uses asynchronous batched query readback;
it does not make global transforms/filter iterations free. Queries, field preparation and solver steps are shared
across views/bodies. Whole-scene budgets, not this document, decide admissible workload.

Exact open boundaries, moving-patch transport, traversable heterogeneous joins, thin-wall wave exclusion, dense
3D turbulence and rigid-body no-penetration are unselected wider capabilities. Numerical/perceptual/device proofs
must reject rest drift, source feedback, lost volume, current-frame mismatch, fake crossings, source-off erasure,
surrogate distortion, leakage and unacceptable cost. These are qualification obligations, not already-passed results.
