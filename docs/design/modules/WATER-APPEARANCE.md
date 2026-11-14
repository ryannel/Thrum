# Causal whitewater and breaking presentation

**14 November 2026: retained as research.** Thrum's water is now designed as one system in
[WATERS' design](../../plans/WATERS/DESIGN.md), which judged this document as evidence and is the authority for
how water works. Nothing below is commissioned.

1 November 2026. **Selected production approximation, unimplemented and uncalibrated.** Refines WAT-02/04/08
under the [water commission](WATER.md), [dynamics](WATER-DYNAMICS.md) and [optical transport](WATER-OPTICS.md).
Published precedents, derived accounting and authored coefficients have different status. Predictive entrainment,
dense plumes and contact-qualified overturning water are not claimed.

## One cause, explicit population quantities

E1 owns causal source records and coarse population state; V5 prepares optional fine samples; V3 owns lighting.
A cause includes world/domain/source revisions, tick/substep interval, event and parent-transfer identity, actual
support, surface reference, incident/receiver velocity and normal, mixed/delivered volume, and work **with its kind**.
Distinguish dissipated work, incident energy and an appearance-only proxy. Sampling one event again cannot birth twice.

| Cause | Actual producer and admissible birth budget |
| --- | --- |
| Hull/contact | P3/E1 wet support and surface-connected relative drag loss `max(0,-F_drag dot v_rel) dt`; reversible buoyancy work is not automatic foam |
| Fall/impact | First intercepted actual transit volume/momentum and the remaining arrival-energy allocation; no second wave impulse |
| Propulsion | Motor/thrust/disk support and admitted ventilation/transom air access; not boat world speed or arbitrary submerged propeller air |
| Breaking | Tracked crest plus the explicit calibrated budget below; not fabricated turbulence or automatically measured solver dissipation |
| Rain/small contact | Actual delivered deposit; water enters storage even when the event is too small to create whitewater |

Bubble packets carry equivalent gas volume/size, foam carries projected bubble area/size, and physical spray carries
actual liquid volume. Optical spray tracers are a separate declared class and cannot later credit water or wetness.
At dilute gas fraction at most 0.02, fluid density/volume remain the water provider's approximation; gas populations
do not silently add a second liquid/mixture volume solve. Its O(gas-fraction) omission needs qualification.

## Crest tracking and authored breaking closure

On a filtered physical graph with declared wavelength support, locate directional ridges
`g=d dot grad(eta)=0`, `d^T Hessian(eta)d<0`. Use supported source direction or a nondegenerate curvature direction.
The ridge normal velocity is `c_n=-(partial_t g) grad(g)/|grad(g)|²`, or a translation fit on a surrounding
nonzero-slope patch. Dividing `eta_t` by zero crest slope is invalid. Match tracks with a 0.25 lambda gate, stable
ancestry on split/merge and at least 0.1 period persistence; degeneracy returns unavailable rather than a guessed front.

Let H be adjacent-trough-to-crest height, lambda crest spacing and S=pi H/lambda. Initial **authored** onset is
`S>=0.35 OR H/depth>=0.78`, persisting 0.1 period; release requires `S<=0.25 AND H/depth<=0.60`.
Require lambda at least 8 cells and H at least 0.05 m. These are starting profile values, not universal or measured thresholds.
The [surface-velocity onset criterion](https://faculty.washington.edu/jmt3rd/Publications/Derakhti_etal_2020_breakingonset.pdf)
is not adopted on the false assumption that depth-average discharge is surface velocity.

For crest length L and admitted injection depth d, use
`Delta V_mix=L d |(c_n-u_surface) dot n| dt` and
`E_proxy=epsilon_break (rho g H²/8) L |(c_n-u_surface) dot n| dt gate`, initially epsilon_break=0.1.
This is an appearance-budget closure, not removed physical wave energy. If an actual breaker-loss operator is later
selected, replace this proxy rather than spending both. Crest phase drives front position; water motion carries foam.

## Birth, surface transfer and death

For a source's available allocation E, initial gas yield is
`Delta V_air=min(alpha Delta V_mix, epsilon_air E/(3sigma/r32+rho g d), max(0,phi_max V_support-V_air))`.
Here `r32=sum(n r³)/sum(n r²)`, so the denominator combines new interface energy and buoyancy work per air volume.
Initial authored alpha values are 0.02 for breaks, 0.05 for impacts and 0.01 for surface contact; epsilon_air=0.1,
phi_max=0.02. Store which budget is proxy versus actual; no pressure-work diagnostic becomes turbulence by naming it.
Use rho=1000 kg/m³, mu=0.001 Pa s, sigma=0.072 N/m, gas density 1.2 kg/m³ and radii 0.5–2 mm in 3–4 bins.

Partition admitted gas volume 25% directly to foam and 75% to bubbles initially. With flattening factor1,
`V_air,foam=0.25 Delta V_air`, `A_foam=3 V_air,foam/(4 r32)`; the other 75% remains in bubbles. It is gas volume, not
particle count or alpha, being partitioned. Initial injection depths
are `min(0.5depth,max(0.02m,0.25H))` for fronts and `min(0.5depth,1m,0.25v_n²/g)` for impacts. Reduce unsupported yield;
do not inject below an admitted receiver. Source energy and support saturation limit birth before sampling it.

Bubble advection immediately uses committed/staged post-deposit `U=Q/h` and the column carrier defined in
[dynamics](WATER-DYNAMICS.md), plus terminal rise, including inside source supports. Incoming momentum sets the
admitted birth depth/spread; no resolved downward jet or entraining plume is claimed. At volume-influx supports the
fluid surface-limit normal speed excludes the inlet flux s, while geometrically attached foam uses the separate
attachment closure. No bubble queries an undefined local velocity. Select the depth-uniform column field with the
inlet `-s` limit, not volumetric redistribution; geometrically attached foam is separately modelled. From
[Wretborn et al. 2022, equation 5](https://alexey.stomakhin.com/research/siggraph2022_whitewater.pdf), select the one-way balance
`a=6pi mu r`, `b=0.5pi rho r²`, `B=(rho-rho_air)g(4pi r³/3)`, `w_terminal=2B/(a+sqrt(a²+4bB))`.
It gives about 0.10–0.22 m/s over the selected radius range. This adopts neither their complete bubble nor foam solver.
Use swept first upward interface crossing to transfer the same gas/size population to foam, respecting branch identity.
Initial gas dissolution `dV_air/dt=-V_air/8s` names an unrepresented/dissolved gas sink, never a liquid-water sink.

Transport uses fixed-step midpoint integration on the declared staged carrier, with bounded subdivisions; initial
guards are `|U|dt/dx<=0.5`, `|div U|dt<=0.25` and `|s|dt/h<=0.1` on bubble support. Require
`h>=max(h_dry,4r_max)` and `r<=d_injection<=h-r`, otherwise reduce bubble birth or explicitly allocate it to surface
foam. Sweep bank/bed/solid contacts, project remaining motion tangentially while preserving gas, and process the
first upward surface crossing. Drying vents unsupported gas through a recorded sink; it never removes water.
Missing support or exhausted admitted transport work rejects the candidate instead of skipping movement. Advance
new packets only over their lived interval from actual birth time; integrate continuous births with bounded temporal
quadrature. Giving end-of-step births a whole step of transport would change age/rise with the solver cadence.

For graph interfaces, publish `u_attach=(U_s,eta_t+U_s dot grad(eta))`; at source supports this is a modelled foam
attachment velocity, distinct from the fluid limit with its `-s` term. For a moving chart P(q,t),
`q_dot=(J^T J)^-1 J^T (u_lift-P_t)` and define modelled chart material velocity
`u_chart=P_t+Projection_tangent(u_lift-P_t)`. This preserves attachment without inventing physical 3D curl flow.
Chart speed, crest speed, physical carrier and modelled chart carrier remain distinct. Reject singular charts or stale
correspondence; never choose a nearest surface on the other side of a barrel.

Foam packets carry projected area, size bin, chart/branch coordinates, tangent footprint, age/seed and ancestry.
Reconstruct `coverage=1-exp(-sum A_i K_i)` with kernels normalized in actual surface area. Advect/stretch footprints,
split stretched packets and merge only compatible local packets preserving area/first moments. Initial area decay
`dA/dt=-A/4s` vents the corresponding gas fraction; it creates no new bubbles or liquid sink. Initial diffusion is
`D=0.0005m²/s+0.01ell²||sym grad_surface(u)||`; it is a strain-based dispersion closure, not turbulent diffusion truth.
Banks require actual no-flux/deposition handling. Dry stacked foam and neighbour cohesion are unselected richer models.

## Motor wash without speed-based foam

For an ideal non-reversing actuator disk, incoming axial relative speed U>=0, thrust T>=0 and area A give
`v_i=0.5[sqrt(U²+2T/(rho A))-U]`, `Q=A(U+v_i)`, `P_h=T(U+v_i)=TU+Tv_i`.
This follows [actuator-disk momentum theory](https://web.mit.edu/16.unified/www/SPRING/propulsion/notes/node86.html).
Q is swept existing water, not a source. Wash power is retained kinetic energy, not automatically dissipated air-birth work.

P3 applies thrust once; E1 applies one equal-opposite wet momentum deposition
`Delta Q_f=-T e w_f dt/(rho A_f)`, componentwise on staggered faces with their dual areas, `w_f>=0`, `sum w_f=1`,
using [dynamics' total-flux scatter](WATER-DYNAMICS.md). Match source first moments or record unresolved torque;
do not write that value directly to q_bulk. There is no depth source. The resulting stored flow carries
the population. It is a depth-averaged reaction closure, not a rotor jet or new current painted only under particles.
Compare actual staged fluid/body work with the admitted motor budget. Any signed mismatch belongs to the explicit
external drive ledger; if power-limited behavior is advertised, jointly reduce/reject thrust and reaction, never clamp
only the fluid. Reverse flow, cavitation and arbitrary rotor geometry are outside this initial disk profile.

For a ventilated surface wash, use the same yield law with swept mixed volume and an explicit air-access factor.
If only ideal wash energy is known, the optical yield is a labelled appearance proxy; do not also debit that retained
kinetic energy as real dissipation. A richer joint motor/fluid/air energy partition is a distinct physical promise.

## Breaking shape and finite splash

Select prepared parametric cross-sections swept along the tracked front, following the production role disclosed by
[Horizon Forbidden West, 2022 speaker notes](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-Water-Malan.pdf).
Preparation supplies positions, derivatives, material map, source mask, inner branch and return-to-base mapping;
runtime supplies the actual front identity, phase, orientation and strength. This is shape machinery, not physics.
Initial collapse time `4sqrt(H/g)` and final 20% return blend are authored timing. Join positions/tangents to the base
and replace its covered surface rather than overlaying a second intact sheet. Nonoverlap/stitch support is required.

The selected curl is **noncontact presentation**. Its full deformation envelope must stay outside protected hull,
interaction and camera traversal support over its lifetime. Otherwise do not admit that chart. Do not spawn a large
curl and later hide it suddenly when a boat approaches. Outside protected support, optical error is separately declared;
inside interaction support the small residual target is initially 2 cm, not measured acceptance. Physical contact with
a curl, occupied barrel traversal and arbitrary intersecting breakers remain open capabilities.

At a real airborne hit, `Delta V_in=Delta V_receiver+Delta V_spray`. Spray retained in transit inherits ancestry and
later makes its own first-hit transfer. Spend one arrival budget:
`E_wave+E_spray+E_air+E_unrepresented<=E_available`; a spray candidate includes kinetic and interface energy
`0.5rho Delta V_spray v_launch²+3sigma Delta V_spray/r32`. Initial spray allocation is at most 10%.
The normal momentum deficit includes retained outgoing spray; the wave/bed reaction cannot ignore that momentum.
Commit all births, deposits, impulses and sound/wetness events only with the world endpoint.

## State, off-path and performance

Maintain bounded coarse, world-owned structure-of-arrays packets; fine view samples are derived. Initial workload
hypothesis: 16,384 appearance packets, approximately 1–1.25 MiB, with separately reserved physical transit. Fine samples
of 250k at 32–48 bytes add 8–12 MiB before staging/history. Neither count is measured capacity. Population updates scale
with packet count × field queries × due steps; sorting/shading/history also multiply by views and coverage.

Drawing off retains coarse tails when continuation is promised; fully disabling the appearance provider is a declared
lifecycle transition with dependent consumers checked. Saves retain coarse state and consumed-event cursors, not fine
GPU samples. Source-off ends births while existing waves, flight, bubbles and foam continue. Physical transit uses
source admission/backpressure, never a particle-count deletion policy.

Birth saturation and packet overflow must have an admitted policy: merge compatible moments or retain them in a
reserved coarse field/source store. If that closure cannot fit, report saturation/backpressure instead of silently
losing already-born material. Graphics tiers change fine sampling, not the coarse birth/area/gas totals of the same
physical/appearance profile. Richer collective bubble plumes require real 3D mixture dynamics; current 2025 immersed-bubble,
PF-FLIP and sharp-interface results are research/reference alternatives, not missing inputs this model may invent.
