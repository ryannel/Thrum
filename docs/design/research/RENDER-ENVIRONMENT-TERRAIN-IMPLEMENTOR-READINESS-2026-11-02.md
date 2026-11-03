# Render, environment and terrain — implementor readiness walk

2 November 2026. **Design dogfood, not a build or qualification.** This walk
uses the current [C4 contracts](../CONTRACTS.md),
[render specification](../RENDER-SPEC.md),
[play/environment baseline](../PLAY-ENVIRONMENT-SPEC.md) and
[integrated research](RENDER-ENVIRONMENT-TERRAIN-SYNTHESIS-2026-11-02.md).
The owning implementor walks are [render](RENDER-IMPLEMENTOR-DOGFOOD-2026-11-02.md),
[environment](ENVIRONMENT-IMPLEMENTOR-DOGFOOD-2026-11-02.md) and
[terrain](TERRAIN-IMPLEMENTOR-DOGFOOD-2026-11-02.md). Proposed type and product
names in those records are design notation, not existing Rust APIs or commands.
The later [gap-closure decision](RENDER-ENVIRONMENT-TERRAIN-GAP-CLOSURE-2026-11-02.md)
supersedes this walk's open-gap status for moving-shore rain and bounded regional
optics; it retains the smooth finite shore's numerical and focused-light gaps.
The later [R0 governing contract](../RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof)
also accepts a bounded first render recipe from the render walk. Its FXAA
source/port, I2a binding, code and device proof remain unfinished.

## The implementor question

For each first slice, an engineer must be able to name authored inputs, the
admitted source/build and cooked products, owning runtime state, exact operations
and order, consumer output and failure status, a small complete fixture and an
independent oracle. If they must invent a physical law, light-path partition,
resource lifetime or product identity while writing a pass, the design is not
build ready. Numeric tuning and device feasibility can remain open only with a
defined prediction, measurement and rejection rule.

## One joint slice to try to build

Use the existing R0 opaque/masked scene to prove V1–V4 independently. The E1
source paper then uses simple flat rectangles as an **analytic rain subfixture**;
those rectangles are not the terrain paper's admitted beach geometry. Cook the
integrated 64 m × 64 m beach's exposure from its actual source triangles, roof,
bridge and enclosed pond, with the same source/product hashes. Expand to the
256 m × 128 m R3 shallow-water envelope only after the smaller closure passes.
The R3 scene has a still sea reservoir mouth, one admitted small floater and two
distinct cameras (one skipped every other presentation frame). An E1 authored
track drives Sun, world wind, supported cloud optical depth and vertical rain.
The first rain mass proof uses the fixed-footprint pond behind retaining walls:
its stage remains within a range preserving first-hit receiver order. The
moving-shore geometry and level-sweep tests run with rain off. Tidal exposure
across a moving shoreline needs a later joint E1/W2/water operator. Physical
water and surface state continue while their visible passes are disabled.

| Step | Required producer → consumer handoff | Fail closed when |
| --- | --- | --- |
| 1. Source admission and cook | F3 stable geometry/patch/material/source IDs and E1 track schemas → one immutable F4 revision. F4 emits render/masked-shadow, collider, nav, water bed/capacity, exposure/response and shader/pipeline products with exact dependencies. | A required product refers to another geometry revision, an unsupported coordinate/field, unbounded error, missing neighbour halo or unknown shader tuple. |
| 2. Pre-tick activation | F4 products and P7 compatible overlay → W2 stages required physical/response closure; F1 publishes it at the pre-tick barrier. P3/P5/E1 acquire leases. | A collider, bed, nav portal or rain receiver is missing; a stale cook or partly migrated save would publish; old contact/reader leases cannot retire safely. |
| 3. Source and physical tick | F1's one time and committed source revision → E1 headless Sun/wind/cloud/rain queries and identified rain interval; P3 samples tick-`n` surface/water; E1 water/surface advance; W1 commits tick `n+1`. | A query is outside support/stale; rain reaches two first-hit receivers; a hull crosses an unsupported domain; pressure/wetness is inferred from a pixel. |
| 4. View extraction and light | W1 snapshot and F4/V1-ready products → V2 per-camera draw/visibility/current-prior geometry; V3 one light registry and source-to-receiver visibility; V1 prepared pass graph; V4 scene-linear reconstruction/display. | Two equal-profile cameras share history identity; a required shadow/medium tuple is not ready; cloud/air attenuation or Sun disk is counted twice; water interface/receiver visibility is incomplete but labelled dark. |
| 5. Transition and recovery | Source edit creates a new admitted revision and initially a new run; device loss rebuilds V1 resources from retained F4 products; P7 restores matching physical source/patch/provider state. | Presentation changes alter physical water/wetness, old GPU resources are reused before completion, or a saved patch is reattached by an ephemeral triangle/LOD index. |

The minimal proof is not one attractive frame. It includes a headless tick oracle,
mass/flux and shoreline/contact comparisons, an ordinary moving two-camera
render with source changes, masked shadow/opacity and history validity, plus
source-off, missing-product, failed-cook, save/restart and device-loss cases.
Whole-scene CPU, GPU and peak-residency accounting uses the selected R0/R1/R2/R3
workloads; none has been measured in Thrum.

### One build binding across the handoffs

F3 admits exact project bytes and one `SourceRevision S`. F4's build manifest
maps that admitted revision and each stable `GeometryId` to a canonical geometry
revision `G`, then to the actual dependency-hashed collider, bed, nav, render,
shadow, patch-area and rain-exposure products. Product content may be reused
after an unrelated source edit; W2 activation still records the particular
S/build binding and refuses a mixed physical closure. Terrain's source patch
identity and local location survive render LOD and atlas changes. P3, P5 and
water read the admitted physical product; V2 reads its checked render relation;
E1 deposits rain through the derived first-hit partition and the surface/water
receiver identities. A flat XZ height lookup is insufficient under the bridge.

F1's integer tick defines the shared time. E1 evaluates phase tracks and the
interval rain mass once per tick. Its point light/wind/cloud values carry source,
model, support and sample identity; V3 consumes the named light source and applies
cloud attenuation once on the direct path. Water, materials, vegetation, P3 and
audio consume supported fields without owning another weather clock. V2 extracts
the committed physical snapshots for each view; V4 history is keyed by camera
identity/generation and the image actually accepted for that view. Disabled
presentation removes its resources without erasing headless physical state.

The first static rain operator partitions projected roof, ground and enclosed
pond coverage into one first-hit receiver or an explicit export sink. The pond
stage is valid only inside its declared interval and below the retaining wall.
If a wet footprint or first-hit order changes, the tick blocks before mass
deposition. The moving shoreline needs a separate conservative exposure
update/transfer operator; the static fixture must not be sold as its proof.

## Readiness audit at the time of the paper walks

| Scope | Current design readiness | Work still needed before claiming the scope build ready |
| --- | --- | --- |
| R0 first rendered game | Canonical WGSL, ABI, material/light closure and finite pipeline demand are selected. The render walk now proposes an exact packet/pass/resource/display recipe and failure states. E1 and V3 walks agree on typed RGB ambient irradiance with one Lambert evaluation. | Accept the selected recipe into canonical design, then implement/qualify the first camera, masked shadow, material/light and display path. |
| Bounded authored E1 source | The environment walk now supplies F3 schema, finite support, product/field definitions, analytic phase and rain operators, absent-field behavior and headless proof cases. | Reconcile with the real F3/F4 envelope and terrain receiver cook; implement the fixed-footprint fixture and independently check phase, mass, save and off-state behavior. It does not supply a general weather simulation. |
| First shoreline/roof region | The terrain walk now supplies source/product DAG, correspondence, discrete error/seam/shore/sill checks, activation and save traces, and cost bounds. | Accept its first source/product schema and error gate in canonical module design; implement cooker/checker, P3/P5/bed/exposure closure and run failure/visual tests. Dynamic wet-front exposure and general nav/mesh certification remain open. |
| R1/R2 assembly | A proposed view identity/history, resource lifetime, light-list, shadow and off-state protocol makes its interfaces inspectable. | Canonical owner must select it; prove ordinary motion quality and sustained CPU/GPU/peak residency on named devices with all consumers active. Seven transparent geometry walks and regional ray queries require measured viability. |
| R3 mixed optics and interactive lake | Render walk proposes an intentionally narrow optical-A: above-water camera, covered rigid/cutout regional geometry, shared surface, absorption-only water and a named mean-interface Sun approximation. The selected R3 physical shallow model is separate. | Decide whether to admit exact two-event mode beside canonical sorted transparency; qualify optional-feature-free event identity, overflow and complete regional query. Wider scattering, focused caustics, deforming/transparent coverage, underwater traversal and GPU-authoritative pressure lake still need core design and joint contracts. |

**Answer at the time of this walk:** before this exercise, the research supplied direction and
proof queues but not enough detail for an implementer to build every handoff.
The three paper walks now make bounded R0, authored E1 and first terrain work
commissionable once their proposed choices are accepted into the owning designs.
They do not make full R3 optics, moving-shore rain or the wider coupled lake
build ready. No code, visual acceptance or device performance has been proved by
these documents or documentation checks.
