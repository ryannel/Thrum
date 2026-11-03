# Render, environment and terrain gap closure — 2 November 2026

**Status:** integrated design decision and adversarial disposition, not an engine
implementation, numerical qualification, visual acceptance or device measurement.
Read the [implementor readiness walk](RENDER-ENVIRONMENT-TERRAIN-IMPLEMENTOR-READINESS-2026-11-02.md),
[moving-shore design](MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md),
[R3 optics design](R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md),
[focused-light challenge](R3-FOCUSED-LIGHT-CHALLENGER-2026-11-02.md) and
[independent adversarial review](RENDER-ENVIRONMENT-TERRAIN-ADVERSARIAL-REVIEW-2026-11-02.md).
The governing ownership and invariants are [C4](../CONTRACTS.md); the selected
initial physical and render operators remain in [PLAY-ENVIRONMENT-SPEC](../PLAY-ENVIRONMENT-SPEC.md)
and [RENDER-SPEC](../RENDER-SPEC.md). The water family retains its wider ambition in
[WATER-OPTICS](../modules/WATER-OPTICS.md). No Broadsides code or assets are a
Thrum dependency.

## What the review changed

| Decision | Current status and reason |
| --- | --- |
| Rain across a moving physical shore | **Selected bounded exposure/response model:** one F1 interval, a source-triangle/rain/receiver arrangement, one first-hit owner per horizontal footprint, interval mass, explicit external or finite water receipt, film/coating/absorbed stores and a pending finite-water exchange. Prescribed spatial waves require interval classification, including wet–dry–wet events inside one tick. A work/error cap refuses rather than guessing |
| Existing shallow HLL | **Retained for matched terrace geometry:** whole 2 m cells are wet or dry. Its `A*h` storage and four-substep argument do not represent the triangulated 1:20 bank. Render or rain may not clip a smooth bank while contact/storage use this terrace as though they agree |
| Smooth finite shallow shore | **Provisional higher-quality numerical profile:** capacity `C_i(η)=∫cell max(η-b,0)dA` from the admitted fine bed, integrated face HLL flux and bed reaction, connected-sublevel certificate, bounded sliver agglomeration and eight-step cap. This is a distinct provider/product/state, inspired by but not copied from [2026 subgrid research](https://arxiv.org/html/2601.01895v1). Positivity/rest arguments and explicit refusal exist; non-rest stability, thin-overtopping accuracy, ordinary-work cap and quality remain unproved. It is not a silent upgrade of the initial solver |
| Transparent events | **Selected bounded exact extension:** retain grouped sorted alpha as explicitly approximate where that profile is chosen; use two ordered nonrefractive coverage events for the exact baseline promise, with GPU status before display. A bounded fragment append plus sort/resolve is the first implementation choice; the seven-raster-walk peel remains a comparison/reference, not an assumed fast default. Bent rays have their own regional event and visit caps |
| R3 regional optics | **Selected bounded approximation:** actual camera-side water occupancy, refraction/reflection/TIR, covered regional geometry including admitted deformation/cutouts/coverage alpha, joint media and one virtual-cover-plane Sun source model. Desktop and portable capability envelopes are distinct; portable absorption is the smaller first proof. The virtual source is complete only within its stated model and omits actual wave focusing and source redistribution |
| Focused actual-interface light | **Open full-family capability:** a conditional interval source-cell method now has a concrete conservative first-hit/receiver-bin flux producer and refusal. It is not a demonstrated 60 Hz path and does not yet produce certified filtered focused single-scatter images. A sparse photon map, screen hit or conservation of only traced packets cannot close this missing source coverage |

The first implementation target for this render-family programme is R0 after
its admitted-source/cook prerequisites, then bounded E1/terrain and an R3
region. Selecting a design does not turn a provisional physical profile or an
unmeasured optical tier into a shipping platform claim. A game requiring focused
falls/caustics, unrestricted glass, arbitrary moving shelters or interactive lake
pressure coupling needs the corresponding separate admitted capability.

## One authority and one spatial answer

F3 admits the source tree; F4 binds its admitted `SourceRevision` to actual
dependency-hashed geometry, material, exposure, capacity, collider, navigation,
render and optical products. W2 activates the required physical closure together.
E1 publishes the committed water endpoint and its **provider-specific wet support**:
full-cell for the original HLL, source-triangle-clipped partial cells for the
provisional capacity profile, and the analytic graph with its declared bed/support
for the prescribed sea. Terrain response and P3/P5 use the same physical answer.
V2/V3 may derive a checked optical shell and visual residual but cannot turn a
physically dry area into an authoritative water receiver. Optical equal-material
occupancy joins across W2 region boundaries; it is not a second fluid solver.

F4 prepares static rain/solid/bed arrangement atoms with stable source patch and
water-domain identities. E1 derives the moving cut from the committed physical
water and source interval, then emits one identified rain mass per first-hit
receiver or named export. For the prescribed wave case, analytic phase extrema
and conservative space-time Taylor bands classify wet/dry cells; ambiguous
local mass and film-sweep bounds trigger subdivision or refusal. The finite
solver uses its declared start-of-substep support and transfers newly inundated
film at the endpoint. Its dry film, submerged bound coating, absorbed mass and
pending water exchange are distinct real stores in one ledger. Pending exchange
is delivered at the next substep/tick under a bounded stage/support lag, preventing
an unspecified film→water→more film fixed-point loop. Prescribed sea exchanges
are external-reservoir entries; only finite water `V` rises from its receipts.

P3 uses X[n] traction/contact during tick n→n+1. E1 stages interval rain,
water faces, material response, pending transfers and local/global ledgers;
W1/F1 publishes the entire X[n+1] or leaves X[n] and forcing cursors untouched.
P7 captures source/build/model IDs, all liquid stores and receipt cursors at a
committed barrier. Two views and a headless host read the same physical endpoint;
graphics-off drops optional optical resources after leases/fences without
stopping retained water, rain or surface tails.

## One optical path, with named approximation

The initial alpha append orders only the first straight camera segment before
its nearest opaque or water boundary. Once refracted, V3 queries the covered
regional geometry; it does not reuse the original screen's depth/alpha records
as a first hit along a bent ray. Camera paths follow the actual admitted optical
shell, orient each medium boundary and distinguish uncovered support, ambiguous
order, exhausted visits/events and a genuinely covered miss. Masked and deformed
blockers use the same accepted source/time/coverage as depth and shadows.

For the selected regional Sun model, underwater sample-to-source rays refract at
a declared horizontal cover plane above the admitted wave/stage envelope. The
segment below it is **virtual water** and the segment above it is air. Opaque,
cutout and alpha blockers are still queried completely along those model rays;
cloud, air, fog and water terms are paid once. A physically wet crest receiver
above mean level now has a forward virtual path. The model can choose a different
air blocker and amount of water extinction from an actual-interface ray, so
actual caustics, focused shafts, self-shadowing and clear nonparallel falls are
not inferred from its complete BVH coverage. Dry source rays that cross water
are ineligible rather than receiving unattenuated ordinary Sun.

The portable absorption recipe retains actual camera refraction/occupancy and
coverage but deliberately has no water single-scattering shafts/haze. The
portable scattering expansion is provisional: the study's first-branch
example has 921,600 potential source queries/frame, requiring at least 92.9%
certified removal to meet its proposed 65,536 uncached-query admission cap.
That cap itself is not a measured 16 ms result. The focused challenger estimates
about 201 MB of logical node reads/update under an illustrative easy budget,
before its interval arithmetic and scattering image work; it is not bundled
into the regional recipe as a free caustic texture.

## Adversarial disposition and proof order

The independent reviewer derived requirements before reading the module
proposals, then supplied concrete counterexamples. A full-cell HLL wet mask on
the 1:20 source bank can move the shore by about 1 m; a wet point at y=+0.10 m
below a +0.20 m crest has no forward mean-plane Sun intersection; moving water
also needs film transfer, not just new rain weights. The owner designs now
separate the physical profiles, name the virtual light model, supply liquid
stores/transfers and enumerate regional path classes. Further review corrected
the Henyey–Greenstein scattering direction, bounded deferred film exchange,
limited merge velocity/energy loss and required analytic-wave interior-time
classification. The [review](RENDER-ENVIRONMENT-TERRAIN-ADVERSARIAL-REVIEW-2026-11-02.md)
retains the original findings and their scope; no reviewer saw a running image.

1. **Independent geometry and mass oracles:** source-triangle wet area/capacity,
   1:20 bank, sill connectivity, roof overlap, finite water/sea ledgers and
   dry→wet→dry inside one tick. A total-mass-only check must fail a wrong local
   receiver. Prove the capacity profile's non-rest/stiff-sliver behavior and
   actual 60 Hz refusal rate before calling it implementation ready.
2. **Independent optical oracles:** crest-above-mean, offscreen dry hull/bed,
   source and camera paths with a pane before/inside/behind water, cutout and
   moving actor, TIR, joint fog/water, scattering-angle sign and event/visit
   overflow before display. Compare regional virtual light against an
   actual-interface finite-Sun reference in motion, recording its visible loss.
3. **Lifecycle and whole-scene gates:** source edit/restart, save/retry/source-off,
   headless, two different view cadences, device loss and feature removal;
   charge all live/retained/transient/staging bytes and CPU/GPU work across
   shadows, alpha, media, terrain and water. The proposed portable 720p optical
   transient subtotal is 38.008 MiB before other consumers and allocator
   overhead; it is a worksheet, not a device pass.

No maturity implementation, correctness, experienced visual or platform status
is promoted by this design pass. The remaining core physical/optical selection
is the full focused surface-plus-volume light path and any wider fully coupled
lake. The capacity solver also remains a provisional numerical proof target,
with a specified refusal route rather than an inherited HLL proof.
