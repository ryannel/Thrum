# Render, environment and terrain: adversarial architecture review

Date: 2 November 2026. Independent review commissioned from the engine brief,
then checked against the saved designs. This is paper design review: no runtime,
visual, numerical, device or user-acceptance evidence is claimed.

## Independent requirements pass, before module inspection

Inputs for this pass: README, contributor/skill instructions, the build-plan
status, and [architecture brief](../ARCHITECTURE-BRIEF.md). The build plan already
names suspected moving-shore and optics gaps; independence is therefore limited
and this is not a blind review. No module designs or implementor dogfoods had
been read when this section was first saved.

The product is a reusable 3D engine across camera styles and desktop, WebGPU and
native mobile profiles. Beautiful images alone cannot establish that effects
agree with play, editing or persistence. The following games are counterexample
generators, not added product requirements or commitments to particular solvers.

| Representative game and change | Necessary interactions and hard case |
| --- | --- |
| Isometric tactics with no drawn sky | Moving cloud shadows still alter the same sunlight used by terrain, characters and water. Removing sky/cloud imagery must actually retire its work while keeping retained illumination and gameplay queries. Two views cannot independently advance weather. |
| Third-person coastal exploration | Wind drives plants and water; rain falls onto a moving shoreline, roof, cliff and water body. A rising wave changes the receiver during an update. Rain hitting water cannot also wet the ground under it; water-retained rain must have an explicit physical meaning. |
| First-person flooded interior | The camera crosses air/water and looks through glass, spray and fog toward a submerged object. A lamp and Sun cross different media/opaque blockers. Transparent composition, receiver lighting and eye-path transport must agree about which segments have already been attenuated. |
| Village builder with terrain editing | A creator raises ground through a shoreline, changes roof coverage, replaces a plant and disables surface response. Physics, visual geometry, precipitation interception, navigation and saved surface identity must all switch to a coherent revision or reject the change. |
| Multiplayer storm/save boundary | Authority steps once while views have different fidelity. Save during rain, stop rain, suspend/reload and join late must preserve durable wetness/water/weather meaning while rebuilding only transient optical/render caches. |
| Dense forest coast at mobile quality | Plants, animated actors, shadows, clouds, water, fog and streaming contend simultaneously. Separate module averages do not bound frame tails, bandwidth, history/transient memory or edit/restart peaks. A simpler tier must retain essential cues and physical consumers. |

Derived rejection questions, independent of any proposed technique:

1. Can an implementer compute a unique precipitation receiver and state transfer
   on moving geometry without inventing a new owner, update order or fallback?
2. Can every light-to-receiver and receiver-to-eye segment identify its media,
   blockers, orientation and already-integrated transport exactly once?
3. Does each supported transparent/material combination have an honest bounded
   representation and overflow behavior, including water crossings and thin glass?
4. Does one authored edit invalidate every derived consumer by dependency, and
   can stale products fail without partially publishing another world revision?
5. Are source-off, consumer-off, headless, reload and two-view cases specified
   separately, including residual state and avoided work?
6. Is simultaneous scene cost bounded with named allocations, pass lifetimes,
   repeated per-view/per-light work, streaming and preparation overlap?

Candidate method families to challenge rather than assume: coarse authored
fields versus geometry-dependent physical queries; analytic segment integration
versus ordered event composition; bounded transparency versus arbitrary overlap;
per-frame rebuild versus incrementally invalidated products; independent module
quality knobs versus coordinated scene admission. Choosing a bounded domain may
be sound; silently substituting a different physical or visual claim is not.

## Inspection findings

Inspected after the independent pass: C02/C04–C10 in
[CONTRACTS](../CONTRACTS.md), [RENDER-SPEC](../RENDER-SPEC.md),
[PLAY-ENVIRONMENT-SPEC](../PLAY-ENVIRONMENT-SPEC.md),
[WATER-OPTICS](../modules/WATER-OPTICS.md), the three
[render](RENDER-IMPLEMENTOR-DOGFOOD-2026-11-02.md),
[environment](ENVIRONMENT-IMPLEMENTOR-DOGFOOD-2026-11-02.md) and
[terrain](TERRAIN-IMPLEMENTOR-DOGFOOD-2026-11-02.md) implementor walks,
their [readiness synthesis](RENDER-ENVIRONMENT-TERRAIN-IMPLEMENTOR-READINESS-2026-11-02.md),
the [integrated research](RENDER-ENVIRONMENT-TERRAIN-SYNTHESIS-2026-11-02.md),
the [baseline](../BASELINE-SPEC.md), relevant source-research sections and
the [maturity ledger](../maturity.json). This is not a claim of reading every
module or every cited publication. The counterexamples below are consequences
of the saved representations; no new external source would settle them.

### Core gaps that invalidate the broader combined claim

**A01 — A full-cell shallow bed is not the triangulated shoreline source.**
Affected owners: W2/F4 terrain, E1 shallow water/exposure, P3 and V2/V3. Location:
PLAY-ENVIRONMENT-SPEC's piecewise-constant 2 m cells with no cut/subcell shoreline,
versus the terrain walk's source-triangle bed interpolation, exact clipped wet
polygons and storage curves, and R3's shared stepped-bed/shore claim.

Counterexample: a 2 m cell on the 1:20 bank spans 0.10 m in source elevation.
Using its centre/mean bed `z_c`, `H=z_c+epsilon` gives positive HLL depth and
4 m² of full-cell water support. The triangular bank has about 2 m² wet at that
stage. At `H=z_c-epsilon` the full-cell model is dry while nearly half the source
cell is still geometrically submerged. The discrepancy is order metres at wet
onset, not the proposed millimetre conversion tolerance. Clipping only the
render mesh to the sloping bank would make rain, optical occupancy, contact and
storage disagree. Sampling the same source does not prove those representations
have the same semantics.

Closure: choose one common bed/wet-support model for the admitted shallow
fixture. A stepped source shared by terrain/collision/water is a valid bounded
choice if its scope and render relation are explicit. Retaining the triangular
bank requires a derived subcell capacity/wet-support/flux/pressure operator,
including the mapping from stored mass to stage and physical queries. A
numerical test against the candidate's own cell bed is insufficient; compare
wet area and capacity with the declared common source at onset and sill levels.
This does not invalidate the prescribed-height, rain-off triangular-bank
geometry fixture when kept separate from the finite shallow solver.

**A02 — The mean-interface underwater light path is undefined under a crest.**
Affected owners: V3, E1 water and V2 optical occupancy. Location: render dogfood's
R3 optical-A mean horizontal interface, through-water irradiance and dry-Sun
partition.

Counterexample: mean plane `H=0`, actual graph crest `eta=0.20 m`, and a bed or
hull receiver at `y=0.10 m`. The receiver is physically wet. Its refracted
direction toward an above-horizon Sun has positive Y, so the positive water ray
cannot intersect the lower mean plane. Treating it as dry adds ordinary Sun
through water; treating the failed query as dark loses valid illumination;
returning unsupported narrows the advertised graph/coastal support. A second
case has a receiver below mean but a raised bank blocking the mean ray while
the actual refracted path clears it, or the reverse. A mean-plane model is an
approximation, but its receiver domain and blocker semantics must exist first.

Closure: specify supported wet receivers relative to mean and actual interface,
the exact wet/dry direct partition and path orientation/positive lengths. Use a
selected actual-interface fallback or explicitly reject the incompatible
receiver/camera/content envelope before claiming that scene's optical-A
readiness. Then qualify the resulting approximation against a richer reference.
Changing the mean-plane equation's tuning cannot repair a nonexistent segment.

**A03 — Moving-shore rain needs both receiver partition and wet-state transfer.**
Affected owners: E1 source/water/surface response, W2 geometry and P7. The
environment and terrain walks already admit this as an open core mechanism;
their first proofs correctly keep the changing shore rain-free and finite pond
footprint fixed. This finding preserves that limitation rather than treating
it as an undiscovered defect.

Counterexample: a wave or rising stage covers half a previously raining response
cell containing film and absorbed liquid. A static first-hit table either still
deposits directly onto covered ground or double counts ground and water.
Rebuilding only the table leaves unspecified whether the old surface film is
displaced into water, remains attached while submerged, or becomes another
store. Subsequent recession can manufacture or erase mass if the choice changes.
Tick-start coverage is a possible explicit discrete approximation, but must name
its timing/error and cannot claim a time-exact moving intersection.

Closure: define common source/geometry/water endpoints, the swept or declared
discrete first-hit partition, exactly-once receiver batch, and the constitutive
rule for film/absorbed stores on wetting and recession. Preserve finite water
versus prescribed external reservoir semantics. Commit source, water, surface
state, transfer ledger and cursors together; save/retry must capture that whole
state. Test a wet/dry/re-wet sequence with rain on, then source off and reload.
An explicit rejection is sound for the first bounded fixture; it does not
deliver the wider combined coast interaction.

**A04 — Wider mixed optics cannot inherit completeness from optical-A.**
Affected owners: V2/V3/F4 and E1/V5 optical contributors. The proposed
TwoEventPeel orders nonrefractive alpha on the original camera ray; optical-A
excludes transparent/deforming solids, scattering water and camera traversal.
WATER-OPTICS's broader design requires source-interface ordering, media sides,
finite shells and complete proxy coverage. Those exclusions are legitimate
first-slice boundaries, but do not implement the broader claim.

Counterexample: camera refraction reaches a glass boat canopy that was not on
the original camera raster ray, then a submerged bed. Two camera event layers
cannot supply that canopy event. A rigid-only proxy that omits it cannot return
a complete bed hit, nor can sky fill that missing receiver. Likewise, a fog
sample's source ray can cross a raised wave before reaching the Sun even when
the eye-to-sample interval is air. An air-only source-ray integrator does not
become a whole-scene light partition by composing its eye segments correctly.

Closure: a supported-path matrix must enumerate eye, reflected, transmitted,
surface-direct and media-source paths; each identifies supported geometry,
medium transitions, blockers, source terms, event/visit bound and failure.
Retain an explicit UnsupportedGeometry result for glass/deformation unless a
concrete admitted transport mapping exists. Keep narrow optical-A claims apart
from broader water/fog/glass/camera-traversal readiness. Further experiments are
useful only after those producers and path rules are selected.

### Selection and integration gates, distinct from missing physical laws

**A05 — Canonical admission must choose the actual render recipe.** The render
walk correctly says it cannot silently replace RENDER-SPEC's grouped sorted
alpha with TwoEventPeel. Its auxiliary ABI revision, 12-tuple R0 inventory,
history receipt and fixed display recipe also differ from or refine the current
baseline. F2/F4/V1/V3/V4 and the architecture owner must select a coherent recipe
and update affected demands before dependent implementation. Otherwise one
implementer can build the canonical approximate path while another assumes
the research's exact two-event policy. Closure is a single canonical selection
and a producer/consumer trace, not review-file agreement.

**A06 — The combined resource schedule is not yet an admitted portable recipe.**
Affected owners: architecture/F2/F4/V1 and all pass owners. At 720p the recorded
two 16-byte event records plus nonhistory image/media inputs total 47.46 MiB,
leaving 3.74 MiB in the browser's 51.2 MiB transient planning share before
overlapping filterable display inputs or scratch. Buffer resolve avoids one
extra HDR allocation but cannot alias arbitrary WebGPU buffers with textures.
The initial browser recipe has one view; the desktop two-view recipe has its
own larger envelope, so these numbers alone do not prove a required profile
fails. A second half-dimension view would multiply those image allocations by
1.25 if concurrently resident; skipped frames still retain histories. Shading
average is not residency.

Closure: select actual per-profile formats, dimensions, resource lifetimes,
alias-compatible allocation classes, union shadow support, concurrency and
preparation/retirement overlap. Reject or serialize before allocating a peak
outside that admitted envelope. Include source/terrain immutable readers,
save capture, seven transparency geometry walks and per-sample source-ray BVH
work in the joint count/critical-path account. Device timings and physical
residency remain empirical qualification after the schedule exists. No fixed
per-module entitlement or unmeasured speed claim follows from analytical bytes.

### Cases challenged without a new defect finding

- **Overflow after display:** TwoEventPeel's proposed seventh walk sets a GPU
  overflow flag before resolve, and resolve emits the diagnostic/current
  admitted fallback before V4. CPU readback is only diagnostic. This is a
  coherent prevention mechanism on paper; implementation must preserve the
  pass hazard and cannot wait until the next displayed frame to discover it.
- **Two views:** stable distinct camera/history identities, actual accepted
  sample times, source revision retention and reset beyond the history horizon
  are already specified. One view does not advance weather or reset the other's
  history. Quality parameters remain unqualified, but no additional clock or
  history owner is needed for the bounded path.
- **Creator edit and save:** the walks pin exact source/build closure, invalidate
  dependency products, initially restart substantive physical edits, and refuse
  old-state restoration on incompatible geometry/material capacity. An explicit
  migration is later scope. Requiring arbitrary live editor migration now would
  invent a product requirement. The actual F3/F4/P7 implementation is still a
  prerequisite and a paper walk is not its evidence.
- **Source-off:** zero precipitation receipts preserve stream order while
  surface/pond/transit state continues. Headless and graphics-off remove
  optional GPU work while preserving retained physical state. No in-flight
  atmospheric rainfall is claimed, so the missing delayed-fallout model is not
  a defect of this prescribed-source fixture.
- **Transparent policy:** sorted blending is openly approximate, binary cutouts
  share cutoff semantics, and narrow optical-A rejects transparent scene solids.
  Those are coherent restrictions if admission enforces them. They must not be
  described as unrestricted glass/water completeness.

## Priority and recommendation

A01 and A02 are concrete representation/path counterexamples requiring a
canonical correction or narrower admitted scene before the combined shallow
coast optics can be called build ready. A03 and the broader part of A04 are
already acknowledged core gaps; their required experiences stay open until
actual joint mechanisms are selected. A05 and A06 are selection/integration
gates. They need concrete canonical recipes and analytic schedules first,
followed by implementation and measurements.

Shader/backend conformance, midpoint-light sampling error, mean-interface visual
quality, temporal ghosting, sustained mobile behavior, actual GPU traffic and
CPU/GPU/residency tails are empirical qualification. They cannot be certified by
this review. Rich meteorology, pollutants, Moon ephemerides, generic live source
migration, porous/oblique rain and arbitrary transparent/deforming optical
volumes are optional or broader-family work when a game actually requires them;
their absence does not invalidate R0 or the declared fixed-footprint E1 fixture.

The bounded R0, authored source and static terrain walks contain useful concrete
owners, equations, failure states and lifecycle recipes. Preserve that work,
while refusing to promote their sum into a completed general coast/environment
design. This report supplies paper counterexamples and closure conditions; the
coordinator owns canonical dispositions and review of subsequent resolutions.

## Second pass: challenge of the replacement candidates

Before reading either replacement, this reviewer challenged the commissioned
capacity-HLL representation with two independently derived cases: two
disconnected subcell bowls cannot share one stage, and merging a boundary sliver
with dry upslope cells adds no hydraulic volume or CFL relief despite zero
stage change. Those were sent to the coordinator before inspection. The root
also supplied the analytical sliver case: for `b=x/20` on a 2 m square and
`eta=-0.05+delta`, `A_w=40 delta`, `V=20 delta^2`; an outgoing wet-face flux
of order `delta^(3/2)` gives `V/Q` of order `sqrt(delta)`.

Subsequently inspected the new
[moving-shore proposal](MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md),
[regional optics proposal](R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md) and
[focused-light challenger](R3-FOCUSED-LIGHT-CHALLENGER-2026-11-02.md).
These are proposals under review, not canonical selections or running proof.

### First-pass disposition against the new proposals

| Finding | Subsequent mechanism and current review disposition |
| --- | --- |
| A01: incompatible finite-bank storage/support | New proposal explicitly separates actual terraced `V=A h` source from a different source-triangle capacity profile. This repairs the representation conflation. Capacity inversion plus connected-sublevel admission supplies a concrete candidate for a smooth finite shore; its numerical stability, hydraulics, quadrature, mixing and practical refusal rate remain consequential unrun proof. Do not call the original HLL qualified on that bank. |
| A02: wet point above mean plane | RegionalVirtual's cover plane above every admitted wet receiver supplies a positive forward source path. This closes the missing-segment defect **within the named virtual-source model**. Optical-A must retain its narrower positive-intersection restriction. Actual source redistribution, focusing, reciprocal reflection/transmission and clear falls remain undelivered by the virtual model; their exclusion is explicit. |
| A03: moving rain and wet-state transfer | Static atoms, source/response/water identities, start-of-subinterval first-hit allocation, conservative dry-film/bound-coating transitions and named finite-water debit supply the previously absent operating model. Prescribed monotone scalar-stage allocation has an explicit local time-split bound. Read A08 below before treating the complete finite-water/material endpoint as closed. Wave-driven shore contact also needs a certified temporal/spatial operator beyond scalar tide cuts. |
| A04: path completeness | Regional geometry now includes current deformed opaque/cutout/coverage-alpha participants and bounded actual camera-tree traversal; bent paths do not reuse the primary raster alpha list. Its source-path matrix rejects real-interface-crossing dry paths and incompatible falls instead of declaring them complete. This is coherent for the named narrower source model. Transparent dielectric solids remain unsupported; the focused challenger separately keeps its full scattered-image producer open. |
| A05: actual recipe | The optics candidate now chooses fixed per-pixel alpha append instead of silently relying on the earlier seven-walk proposal. It preserves sorted alpha as another labelled policy and retains peel as an oracle. Canonical acceptance and exact reachable tuple/ABI reconciliation remain coordinator work. |
| A06: combined targets | The new worksheet separates alpha and first-interface storage, ray batches, medium pairs, final output and status. It explicitly refuses two concurrent 720p portable views and charges both desktop views. This closes the earlier missing-schedule/accounting choice at design level for the listed optical allocations. Current/prior vertex products, all scene consumers, upload/retirement overlap and device residency/time still need whole-profile admission/qualification. |

### A07 — Regional single-scatter equation reverses the HG angle

Affected owner: V3, with E1 phase metadata and independent reference consumers.
Regional optics §5 gives `p(dot(w_light,w_to_camera))` while declaring both
directions point **away** from the sample. The incoming propagation direction
is `-w_light`; the outgoing direction is `w_to_camera`. For standard HG with
positive forward-asymmetry parameter, the propagation-angle cosine must be
`-dot(w_light,w_to_camera)`.

Concrete case: Sun and camera on the same side of a sample give dot=+1.
That is physical backscatter, but the written formula gives HG's forward peak.
Putting camera and Sun on opposite sides should give the forward peak and is
instead darkened. For `g=0.8`, the HG forward/back ratio is
`((1+g)/(1-g))^3=729`; this is not a small convention error. Rayleigh's even
phase hides the defect. Saying the reference will fix the sign leaves the
saved equation contradictory.

Closure: define propagation and toward-source/observer directions explicitly,
correct the equation or declare and consistently map a reversed HG convention,
and add independent same-side/opposite-side analytic phase tests. The focused
challenger must use the same convention when its single-scatter extension is
designed. No device measurement is needed to resolve the sign.

### A08 — Finite water and coating need a final consistent wet endpoint

Affected owners: E1 finite water, terrain response and W1 transaction. The
moving-shore sequence derives an endpoint wet set after flux, transfers the
newly inundated excess film/coating into free-water V, then rechecks support.
That positive volume changes stage and can inundate additional film. Refill
withdrawal can similarly lower the stage before flux. A consistency check can
reject a candidate, but it does not specify how a valid final endpoint is
produced in the ordinary coupled case.

Counterexample: incoming finite flow wets a first strip of sloping bank carrying
dry film above the coating cap. Released excess raises stage, wets a second
strip and releases more film. Using only the first sweep leaves some liquid
classified as exposed film underneath final water; repeatedly sweeping requires
a selected convergence/termination and mass-transfer rule. A globally correct
mass sum passes either wrong support classification. A smaller dt alone does
not define the missing final-state operator.

Closure: specify a bounded monotone/fixed-point endpoint solve with an actual
existence/uniqueness or refusal domain, or select a conservative delayed
exchange inventory with explicit support timing/error. Recompute/validate
coating classification and finite-water stage together before publication.
Demonstrate a high-film bank advance, water-starved coating refill, ebb and
retry/save without an unresolved support change. This is a core coupling
question, separate from testing the HLL pressure algebra.

### A09 — Agglomeration cannot choose erased flow by an undefined adjective

Affected owner: E1 capacity-HLL and F2 admitted physical profile. Equal stage
and the 0.5 mm stage limit do not bound velocity mixing. Two equal-volume,
equal-stage neighbors with opposing velocities `+u` and `-u` merge into rest.
All resolved kinetic energy is removed while volume and integrated momentum
remain exact. The proposal correctly reports the nonnegative energy loss, but
its instruction to reject a merge that would “hide materially different flow”
has no numerical admission rule.

Closure: define an actual velocity-dispersion/energy-loss limit and its relation
to allowed hydraulic truncation/cue error, including cumulative merge loss and
repeated merge/split over a tick. A loss ledger reports the decision; it does
not select which flowing states may collapse. The optional aggregation can
remain a conditional acceleration with explicit refusal when no compatible
neighbor exists. Eight substeps and a fully wet mouth apron honestly narrow
the sliver case; they do not establish stability or useful coverage for every
advancing bore. If ordinary required motion repeatedly refuses, the numerical
profile must change rather than skipping physical time.

### Pressure/source and focused-light challenges without a new defect

The proposed capacity momentum cancellation has the correct algebraic sign:
for reconstructed constant stage, bed reaction equals the sum of own-face
hydrostatic pressure integrals. Adding the conventional side correction
`Pi_i-Pi_i*` and that bed reaction yields net face term `Phi_m-Pi_i*`.
At common-stage rest its cached shared pressure branch cancels exactly. This
is a coherent paper lake-rest construction. It is not a nonlinear stability,
moving-front accuracy or convergence proof; donor volume positivity does not
establish those other properties. Pointwise face HLL plus interval quadrature
still needs independent non-rest and thin-overtopping qualification.

The focused challenger does not conceal its remaining work. It proposes
conservative finite source-cell geometry bounds and receiver-bin flux intervals,
with refusal at work limits and separate model versus arithmetic error. It
explicitly says surface-bin conservation does not produce a filtered
single-scatter camera image, and does not certify arbitrary folded-water
beauty or real-time performance. Thus it is a restricted challenger/reference
candidate, not a closure claim for the full focused surface-plus-media family.
The virtual cover plane is a valid distinct approximation only within its
declared eligible source domain. Neither more samples nor source bookkeeping
recovers the actual caustic redistribution that model deliberately omits.

### A10 — Scalar tide clipping must not stand in for physical wave contact

Affected owners: E1 prescribed sea/source, terrain response, P3 and V2/V3.
The first replacement text used scalar-stage affine cuts while PLAY's sea is
mean tide plus up to sixteen spatial sinusoidal waves. On a 1:20 bank even a
0.05 m physical wave amplitude moves contact about 1 m horizontally; using a
mean-tide rain/coating mask would disagree with P3 and optical occupancy far
beyond the 0.10 m shore target. A patch can wet and dry inside one tick while
both endpoint masks remain dry. An endpoint-only rain bound can report zero
error even though real first-hit and coating history changed.

**Resolution verified in the later owner revision:** tide-only clipping is now
a separate fast path allowed only with zero waves or a certificate that the
entire wave envelope leaves receiver ordering unchanged. The wave-contact
branch uses the actual PLAY component/phase evaluator. It interval-encloses
cosine extrema over each spatial/time slab, then constructs affine Taylor
upper/lower bands with second-order phase remainder. Ambiguous space-time
volume bounds local rain assignment; possibly wet union versus observed swept
true area bounds missed film transfer; rate times uncertain wet-area-time
bounds refill/absorption/evaporation effects. Source geometry and exchange
timelines remain CPU/headless. Local error, topology and work caps refuse the
whole candidate rather than advancing only the source or a cosmetic mask.
The later deterministic recipe clips each triangle-times-time prism by the
centre Taylor plane, partitions water/solid with the same cut, and integrates
linear rain from polyhedron volume and first time moment; upper/lower Taylor
planes enclose its local assignment error. This replaces the frozen-start
`J*area` estimate for the wave branch while retaining a separately bounded
chronological material split. An implementer is not left choosing arbitrary
weights inside the uncertain band.

This is a concrete conditional mechanism, not evidence every sixteen-wave sea
fits sixteen time slices or useful CPU cost. Tangent contact on a broad flat
shelf may produce loose possible-wet bounds and refusal even at small rain
error; tiny rain mass alone cannot justify ignoring a finite film sweep. The
saved operator explicitly includes that distinction. Qualification must report
refusal rates on ordinary wave advance/retreat and exact protected-sill cases.

### Verified disposition after owner corrections

This table supersedes the open wording of A07–A09 above while preserving the
original defects and their counterexamples for provenance.

| Finding | Verified later text and current disposition |
| --- | --- |
| A07 phase sign | Regional §5 now defines toward-source and toward-camera directions, `mu=dot(-omega_light,omega_camera)` and standard HG. Its oracle has `g=0.6`, opposite-side/source-side ratio 64 and a continuous angular sweep. **Closed at design level.** |
| A08 endpoint feedback | Inundated excess now enters a real pending water-exchange inventory, not current free-water V. Delivery is next substep/next tick, at most one tick residence, with saved identities and inclusion in total liquid. Its hypothetical immediate-delivery stage shift is bounded to 0.5 mm and wet-set displacement to 0.01 m with protected topology unchanged. Source-updated coating withdrawal uses frozen contact support and an explicit endpoint remap with the same shift bounds. This selects a finite lagged exchange operator rather than leaving a recursive solve implicit. **Closed at design level within those rejection bounds.** High-film onset can be outside this profile and is not silently solved by conservation alone. |
| A09 flow erased by merge | Each wet constituent must satisfy `abs(u_i-u_G)<=0.02 m/s`; per-merge and cumulative tick kinetic-energy loss have explicit `1e-3` reference-energy bounds. Opposite fast currents can no longer merge merely because stages agree. **Closed at design level.** Hydraulic accuracy, sufficient compatible neighbors and useful eight-step/refusal coverage remain unrun. |
| A10 physical waves | Actual analytic-wave interval/Taylor classification and local material-history bounds now separate the scalar fast path. **Closed as a conditional design mechanism**, with useful-domain/numerical/runtime proof unrun. |

The revised optics text also distinguishes physical terrace full-cell support
from capacity-triangle clipped support and requires the corresponding exact
cut in optical shells, rain and physical queries. Its boundary-band camera
initialization now refines the physical/shell predicate or fails; perspective
pinhole jitter is not falsely treated as putting the eye on both sides. These
are useful cross-owner consistency corrections.

Performance remains an explicit limit rather than a passed claim. The portable
scattering expansion now caps uncached source visibility queries at 65,536;
its stated 921,600-query first-branch example therefore requires at least 92.9%
proved empty-prism removal, with secondary branches potentially requiring more.
A dock or moving hull can invalidate that easy-envelope prediction. The new
`RegionalAbsorption-P` first proof removes water scattering and limits each
camera path to one interface/three tree nodes; it is an authored narrower
visual profile, not a device secretly dropping scattering. Air/fog keeps its
own declared operator and joint budget. Neither profile is a measured 16 ms
result. The allocation worksheet is a logical optical subtotal; complete scene
consumer, deformation, staging, old/new, browser/runtime and physical residency
charges remain mandatory before device admission.

The fixed two-slot alpha append now has explicit primitive-key/count preflight,
zero/opaque/interface rejection before atomics, finite tile coverage, unique
record ownership and a later-pass sort. A third event fails through the GPU
gate before display/history, independently of CPU readback. Regional bent rays
query the full current proxy instead of reusing the primary camera list.
Desktop's initial raster segment remains capped at two alpha events even when
later tree paths admit four; that restriction is now explicit. These are
coherent finite operators on paper. Adapter storage conformance, extreme
overdraw and whole-scene cost remain unrun.

**Current recommendation:** no additional conceptual blocker found in the
latest declared bounded rain/material and regional virtual-optics operators.
The architecture owner must still accept/reconcile the proposals and preserve
their real admission exclusions. Capacity-HLL numerical stability, non-rest
overtopping accuracy, quadrature tightness, coating-lag/merge quality and
ordinary-motion refusal frequency need their stated independent executable
proof before implementation-ready or visually successful claims. Full
actual-interface focused surface-plus-media transport remains open; the
restricted interval surface-flux challenger does not close that family.

The coordinator's later
[gap-closure synthesis](RENDER-ENVIRONMENT-TERRAIN-GAP-CLOSURE-2026-11-02.md)
was checked against the owner reports and these dispositions. Its physical
profile separation, provisional capacity status, conditional wave exposure,
named virtual-source losses, portable scattering query cap and unmeasured
optical subtotal accurately preserve the reviewed scope. This is a synthesis
consistency check, not independent implementation or canonical conformance
evidence.
