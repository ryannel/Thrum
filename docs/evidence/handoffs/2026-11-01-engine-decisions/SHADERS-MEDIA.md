# Shader demand, renderer scope and overlapping media — decision handoff

1 November 2026. Analytical design decision for audit A03, A05 and J02. This is an
integration handoff, not an implemented or device-qualified engine. It refines
the existing [C04–C08](../../../design/CONTRACTS.md), [F2/F4/T1/T2](../../../design/modules/TOOLS-DELIVERY.md),
[V1/V3](../../../design/modules/RENDERING.md) and [E1](../../../design/modules/ENVIRONMENT.md)
owners. It neither creates a second asset database nor changes the source/cook,
manifest, world, or GPU authorities. A game with no renderer has no shader demand.

## Decision A03 — finite cooked demand, device preparation at runtime

**Select a finite demand closure for each named target build and its admitted
profile plans.** F2 resolves the explicit static game setup, target/backend
limits, scene/content roots, supported visual tiers, pass providers and permitted
live transitions. F4 extends its existing hashed product DAG with shader modules
and pipeline *descriptors* for the exact reachable demand tuples. V1 validates
and prepares the corresponding executable GPU pipelines on the actual device.
An F4 cooked product is not a promise that the device has compiled its pipeline.

A demand tuple names: selected material closure and its version; material
feature/texture/alpha mode; geometry and deformation interface (including
skin/morph/instancing where used); render path/pass role (for example depth,
masked shadow, forward opaque/transparent, velocity, reflection); selected
light/medium/effect provider contract; render target/depth formats, sample count
and layout; target feature tier/backend; and compile-time specialization values.
Only reachable tuples are included. The builder walks actual scene/prefab
instances and declared dynamic asset families through pass providers. It also
includes replacement and fallback tuples required by each admitted transition.
It reports an inclusion reason and dependency chain for every tuple. This is a
set union over selected plans, not a Cartesian expansion of all material,
geometry, pass and platform options.

The demand declaration is a consequence of ordinary source registration and
content references, not a parallel author-authored shader inventory. A pass
provider declares what interfaces and pass roles it demands; a material declares
its supported closures/roles; geometry declares its vertex/deformation layout;
an imported material has an explicit preserved/converted/fallback/rejected
mapping. F2 rejects a required tuple with no compatible implementation or
target capability. F4 then strips only shader products proven outside the
closure, and T2 records why included or excluded. Unreachable optional packages
and their shaders leave the build if build-excluded; compiled but load-disabled
code may remain in the binary under the existing C04 distinction.

**Identity and caching.** The immutable F4 shader product key hashes canonical
source/IR bytes, all include/dependency hashes, material/pass/geometry schema
and layout versions, compiler/translation/toolchain versions, target API/feature
tier and compile-time specialization. Its manifest links logical demands to
verified product hashes as C05 already requires. A separate V1 runtime pipeline
key hashes the F4 product hash plus the complete concrete pipeline descriptor,
backend/adapter feature set and device generation. In-process reuse is allowed
only for an equal key; device loss invalidates handles and reconstructs them
from retained cooked inputs. Any native driver cache is an optional accelerator,
versioned/device-scoped and disposable, never product authority. No universal
cross-API binary or reliable warm driver cache is assumed.

**Readiness is two-dimensional.** T1/T2 report source validity, target cook
closure, package/download/offline availability and actual per-device pipeline
preparedness separately. A required tuple for the first playable scene must
be packaged or fetched and V1-prepared before that scene/view becomes playable.
Later regions may enter a named loading/blocked state while required pipelines
prepare, then activate under the same C05/W2 generation rules. An admitted live
profile switch reserves old/new overlap, prepares all newly required products
and pipelines, commits one profile generation before extraction, invalidates
affected histories, then retires old resources after leases/fences. Failure or
device loss during preparation leaves the previous plan active. There is no
silent use of a stripped variant or GPU compilation stall hidden in the first
draw. WebGPU explicitly recommends async pipeline creation because nominally
immediate pipeline creation may stall later on the queue; async completion still
must be checked for errors and device loss. [WebGPU specification](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createrenderpipelineasync).

Required means missing it would change an admitted material, silhouette,
depth/occlusion, shadow, velocity or promised visual behavior, or block a
selected pass. It fails build/load/transition readiness with the source and
tuple cause. Optional means a *declared* presentation reduction with its own
cooked, prepared fallback pipeline and semantic/appearance limit: e.g. a
nonessential effect may be absent; a separately approved simpler material may
substitute. A masked shadow cannot silently become opaque or disappear. A
placeholder never supplies absent collision/gameplay content. F2 validates
fallbacks against the selected content/profile before they can be used; T1
labels preview fidelity and T2 records actual activation. Unity documents the
production risk of stripping a shader variant that runtime selection later
needs; O3DE documents a shader-asset build path, supporting demand/cook
traceability rather than any automatic completeness claim.
[Unity shader stripping](https://docs.unity.com/en-us/engine/6000.5/manual/materials-and-shaders/shaders/shader-troubleshooting/shader-reducing/shader-variants/shader-variant-stripping),
[O3DE shader build](https://www.docs.o3de.org/docs/atom-guide/dev-guide/shaders/shader-build-pipeline/).

**Dynamic content is bounded.** A dynamic lookup root declares stable asset
IDs, a finite family with explicit material/geometry/pass envelope, or a
separately admitted external-content policy. An unknown future code lookup
cannot be found by static scanning and is not a reason to include every
variant. At runtime an undeclared request is rejected with a content/demand
diagnostic. External content is first validated and cooked against the
target/profile and its full shader closure; it cannot inject arbitrary raw
shaders or material flags around F2/F4. Admission can prepare a new immutable
manifest generation and stage required V1 pipelines before the content becomes
active. Installed/offline availability is a separate declared delivery claim.
Untrusted executable shader/mod support is outside the baseline.

**Bound variant growth.** Use numeric uniforms, resources or a bounded dynamic
branch for values that change per draw/frame without changing bindings, vertex
interface, pass semantics or target capability. Compile-time specialization is
reserved for dimensions that alter layout, pass contract, materially remove
work, or are proven to benefit under a named target budget. Share equivalent
material feature sets; choose authored finite tiers; cap and report tuple count,
cooked bytes, prepared pipeline count and peak preparation time. If the cap is
exceeded, reject the build with the dominant axes and require an explicit
material/pass/tier reduction. Do not silently drop variants or assume dynamic
branches are free. Target-specific specialization and native caching remain
empirical tuning after this semantic closure.

**Alternative disposition.** Pure runtime specialization from arbitrary
materials has a smaller initial cook, but cannot certify offline or first-play
readiness and can shift compilation stalls into play. Making every possible
variant in advance explodes combinations and includes impossible tuples. A
single dynamic uber-shader can reduce pipeline count, but cannot represent
different binding/vertex/pass interfaces and may retain avoidable cost. The
selected finite closure plus narrow dynamic dimensions has an inspectable
build and transition guarantee. It admits controlled on-demand content via the
same F4/V1 transaction rather than promising universal advance preparation.

**Analytical rejection cases.** (1) A browser build omits visible sky but uses
masked foliage with shadow and velocity passes: masked shadow/velocity demands
remain; the sky-only tuples are stripped. (2) An allowed live switch enables a
new shadow provider: its descriptors and dependencies are in the build closure;
the switch waits for actual device readiness and fails without changing the
active plan if preparation fails. (3) A streamed imported character material
arrives with an unsupported texture/alpha extension: explicit conversion or an
approved fallback is required before activation; no random existing variant
may draw it. (4) Device loss after a warm run invalidates V1 handles, retains
F4 products, and blocks required draw until preparation for the new device
generation completes. (5) A dynamic name constructs an undeclared material ID:
the runtime gives an unknown-root error; it cannot make target promotion green
by relying on developer cache residue.

## J02 — scoped renderer extension promise

Select the existing separable headless/runtime boundary, optional render
features and material/style closures, typed V2 extraction, V3 light/media
inputs and pass hooks submitted through V1. A game may omit the renderer or
visible sky; a selected backend adapter may replace the GPU API implementation
while preserving V1's semantic contract. A pass/material extension must declare
its resource edges, output units, layer/depth behavior, target limits, shader
demands and history invalidation. It receives scoped typed inputs, not a second
device or queue. The baseline remains the provisional depth-prepared clustered
forward renderer with spatial fallback. This allows the S06 painterly style
through material/light/display policy where those contracts suffice.

Arbitrary whole-renderer replacement, automatic material compatibility across
unrelated pipelines and live renderer swaps are **not** public promises. A
materially different deferred/tile pipeline is a separately selected provider
only when a named workload/target justifies it; it must specify a complete
material/light/media/depth/velocity/display mapping and A03 demand closure.
If its mapping cannot preserve requested content semantics, F2 rejects that
profile. A native backend's extra features are capability-gated, not quietly
imposed on browser/mobile. This avoids a framework for a requirement absent
from the current brief while keeping the verified extension seams. Relevant
official precedents distinguish custom pipelines from graphics drivers:
[Unity SRP](https://docs.unity.com/en-us/engine/6000.0/manual/render-pipelines/scriptable-render-pipeline-introduction),
[Godot rendering architecture](https://docs.godotengine.org/en/4.7/engine_details/architecture/internal_rendering_architecture.html).

## Decision A05 — medium representation admission

Let distance `s` increase from camera toward an opaque surface or the terminal
background, along one specified ray. A medium has extinction `σ_t(s) >= 0`
in m⁻¹ and source `j(s,ω_o)` in radiance per metre. For a segment `[a,b]`,
`T(a,b) = exp(-∫[a,b] σ_t(u)du)` and
`S(a,b) = ∫[a,b] T(a,s) j(s,ω_o)ds`.
Then `L(a) = S(a,b) + T(a,b)L(b)`. Ordered *disjoint* near/far
segments compose as `(T_n T_f, S_n + T_n S_f)`. A sky is boundary `L(b)`;
an opaque surface truncates the path; transparent/water interfaces need their
declared depth and boundary rule. This is the existing C08 scene-linear
operator, with V4 only receiving its completed image.

For overlapping media `i`, the joint integrator uses
`σ_total(s)=Σ_i σ_t,i(s)` and a jointly defined `j_total(s)` before
integrating. Under a stated single-scattering closure,
`j_total = Σ_i [σ_s,i(s) ∫ p_i(ω_in→ω_o) L_in(s,ω_in)dω_in + e_i(s)]`;
the `L_in` for direct lights must use the **total** relevant extinction,
occlusion/cloud visibility and light registry on each source ray, once. Thus
independently computed in-scattering terms generally cannot be simply added
even when their view-ray extinction coefficients are available. Multiple
scattering, ambient, emission, phase-function and spectral/colour reductions
must be declared as part of the joint closure or as explicit approximation.
The provider cannot treat visual sky radiance as physical source irradiance
without a specified mapping. These relationships follow radiative transfer;
[Hillaire's atmosphere method](https://sebh.github.io/publications/egsr2020.pdf)
is a production candidate for a specified atmosphere, not proof of arbitrary
fog overlap interoperability.

**Admit exactly one of these representation modes per ray support/quantity:**

1. **Ordered nonoverlapping partition.** E1/V5 provides segment bounds and
   already integrated `(T,S)` for mutually disjoint intervals in the relevant
   ray domain. V3 orders and composes them. Segment validity includes ray/view
   range, time, wavelength/colour basis and source-light assumptions.
2. **Joint coefficient/source integration.** Each overlapping provider exposes
   bounded support and compatible `σ_t`, `σ_s`/phase/emission or equivalent
   source-evaluation inputs. One selected V3 integrator evaluates the total
   coefficients and shared lighting closure over the overlap, producing a
   single `(T,S)` for that interval. E1 owns medium identity/source fields;
   V3 owns light accounting, integration and depth order; V1 executes the
   declared passes.
3. **Jointly precomposed provider.** One selected owner consumes all overlapping
   source media and light dependencies, jointly integrates them and returns
   the combined segment pair with support and validity. Its constituents are
   masked from any second independent integration over the same interval.

An **explicit approximation** is separately admissible for a named profile:
homogeneous-mixture reconstruction, froxel sampling, ordered screen layers, or
mask/partition of one medium's support. It declares the modeled domain, which
terms are omitted (e.g. cross-attenuation of lighting, multiple scattering),
error/visual acceptance policy and fallback. It must never be labeled exact
coefficient combination. F2 rejects independently preintegrated overlapping
pairs with no joint owner or chosen approximate policy. Approximation is a
profile/content choice; an automatic visual quality reduction cannot silently
change the selected physical/gameplay environmental source.

**Analytical rejection case.** In a common interval, medium B emits a narrow
source at the center and has standalone pair `(1,b)`. Medium A absorbs with
total optical depth `τ`, standalone pair `(e^-τ,0)`. Concentrate most of A's
absorption before B, then after B, preserving positive A support throughout
and the same total `τ`. Both providers' standalone pairs stay identical;
the combined B contribution changes from about `e^-τ b` to about `b`.
No operator over only those two pairs can recover both answers. For disjoint
ordered slabs the pair operator above does recover the correct result. A
second test uses a local Sun-lit fog volume overlapping global atmosphere:
the fog's source must change when atmospheric/cloud attenuation on the Sun
path changes, even if the fog's local coefficients remain fixed. A third ray
crosses air, a transparent water interface and underwater medium; the interval
partition and optical boundary must not integrate air behind the opaque depth
or reuse an inapplicable underwater pair.

**Output selection and support.** F2 validates distinct consumers: terminal
sky/background radiance, direct Sun/source transmittance, environment/ambient
radiance, aerial perspective, local media, cloud directional visibility and
optional visible clouds. An HDRI or gradient can supply a background (and
separately calibrated illumination) without claiming path extinction; a
precomputed atmosphere has declared geometry/altitude/time/parameter validity;
a dynamic provider has its update and target cost. One scene may use distinct
background and lighting providers by an intentional authored coupling policy.
No-visible-sky views can still require direct transmittance, aerial perspective
and cloud shadows. A provider/output request outside altitude, distance,
direction, time or parameter support is rejected or uses a declared fallback
with its limited claim; no silent clamping or stale LUT. Local/far fog boundaries
must be partitioned or jointly integrated. [Unity HDRP sky choices](https://docs.unity.cn/Packages/com.unity.render-pipelines.high-definition%4017.1/manual/sky.html)
and [ambient-light separation](https://docs.unity.cn/Packages/com.unity.render-pipelines.high-definition%4017.1/manual/ambient-lighting-configure.html)
illustrate the distinct outputs, not a selected engine implementation.

**Residence and lifetime.** Authoritative weather/medium source facts and any
gameplay query stay E1/W1 CPU/headless-compatible under C02/C09. GPU LUTs,
froxel grids, cloud volumes and volume histories are derived presentation
products under F4/V1 leases and E1/V3 effect-history ownership. A profile can
use CPU analytical, precomputed or GPU-integrated rendering within its declared
support; a GPU-only visual residual cannot silently become authoritative
fog/heat/visibility state. If a gameplay query later requires GPU-derived
fields, its latency, readback, headless and restore semantics require a
separate decision. F2 validates one medium identity/support owner, V3 prevents
double extinction/source accounting, V1 validates pass/resource edges, and
V4 owns reconstruction/display only. Medium/provider or light-model changes
invalidate affected effect and view histories through C07; cooked LUTs carry
the source, model, light/spectral-basis and validity dependencies in F4's
existing DAG.

**Future multiplayer boundary.** A future N1 authoritative simulation
snapshot/input contract carries the world and E1 fields that affect gameplay,
including their model/schema and semantic content compatibility. It does not
serialize V2 render packets, per-view media segments, shader/pipeline handles,
GPU LUTs or temporal histories as network authority. Each peer may cook/load a
different target-specific visual profile and GPU products for the same
compatible gameplay content; byte-identical render payloads are not required.
F2/F4 distinguish a gameplay-relevant content/model mismatch, which blocks
joining or requires an agreed migration, from a local visual-product or
device-readiness failure, which blocks that peer's view/play readiness under
its declared policy. Local fog/cloud approximations cannot alter the
authority-owned weather, visibility or physical medium field consumed by game
rules. This extends C02/C04/C05/C09 without turning renderer snapshots into
the network-state schema.

## Contract edits and qualification still needed

| Owning record | Exact correction to integrate |
| --- | --- |
| C04 / F2 and COMPOSITION | Add finite shader demand/transition closure and medium representation compatibility to plan validation; keep static setup, optional packages and build/load/live distinction. |
| Future N1 / C02/C04/C05/C09 | Keep authoritative gameplay content/model compatibility separate from per-peer visual profile and cooked GPU byte identity; network snapshots carry E1 gameplay fields, not render packets/pipelines. |
| C05 / F4 and W2 | Add shader modules/descriptors to existing hashed cook/manifest/required-product transaction; distinguish cooked/package from V1 device-ready; preserve old generation on failure. |
| C06 / V1 | Own asynchronous or off-frame pipeline preparation, concrete device key, error/device-loss handling, submission and retirement; no parallel shader authority. |
| C08 / V3 | Replace unconditional `preintegrated pair` equivalence for overlap with the three admission modes above; require shared-light joint source treatment, interval/depth order and declared approximations. |
| C07 / V2/V4/E1/V3 | Propagate material/provider/profile epochs and invalidation to image and effect histories; preserve actual view sample times. |
| E1 and V3/V5 | E1 declares medium identity, support, coefficient/source or precomposed pair mode and environmental light facts; V3 integrates; V5 submits typed effects only. Fix E1 and V3 wording that currently offers a preintegrated pair as generally equivalent to local coefficients. |
| T1/T2 / WORLD-CONTENT | Preview and build report exact demand inclusion, missing tuples, fallback fidelity, offline/first-play/device readiness and source cause. Dynamic roots and imported-material mappings feed the same F4 graph. |
| CONSTRUCTION-PLAN D1/D2/I2/I3/I4 | D1 owns source/product schema and dynamic roots; D2 owns tuple and transport compatibility; I2 cooks chosen products; I3 prepares first rendered scene; I4 qualifies named targets and transitions. No dependent rendered path claims closure earlier. |

The **mechanisms above are selected analytically** for the shared-contract
stage. D1/D2 must still specify concrete material/pass schema, supported
source language and compiler/translation path, exact descriptor hashing and
error diagnostics, layout limits, actual dynamic-branch policy, medium spectral
basis, phase/lighting closure, ray partition/intersection conventions and
chosen approximations. Target device floors and budgets are product inputs.
Implementation must then measure pipeline counts, cook/first-play and switch
latency, compile failures, memory peaks, volume sample/error bounds, cross-view
history stability and representative mobile/browser sustained behavior. Visual
review must compare whole scenes and moving rays, including the analytic cases
above. No existing runtime/build, target performance, shader compatibility or
media image is certified by this memo.

## Cross-review against N1, sessions/source and water domains

Read [N1 networking](../../../design/modules/NETWORKING.md),
[sessions/source](SESSIONS-SOURCE.md) and [water domains](WATER-DOMAINS.md)
after the decisions above. N1's authority, purpose-specific replication
projection and opt-in prediction fit this handoff. The following narrow
clarifications prevent the three designs from making incompatible promises:

1. **Peer compatibility is a semantic projection, not `BuildRevision`
   equality.** The source handoff's `BuildRevision` hashes target profile and
   products, so two peers with different render backends must normally have
   different build hashes. N1 negotiation should compare a separately derived
   protocol/game-rule/replication-schema and gameplay-content/model lock from
   each peer's admitted source/build, with an explicit compatibility rule.
   Only gameplay-relevant collision, queries, water model/edge, rule and
   authoritative E1 data enter that check. Texture, shader, LUT and GPU-cache
   bytes remain local unless a particular game rule genuinely depends on an
   authored value they encode. Never infer semantic compatibility merely from
   a common asset ID or seed.
2. **Network admission and local view readiness are separate gates.** N1 may
   synchronize/maintain a coherent replica while V1 prepares required local
   pipelines. The peer remains locally loading until its selected playable
   view has cooked, delivered and device-ready required tuples, or an already
   declared acceptable fallback. The server does not pause; N1's bounded
   queue/baseline policy applies during the wait. If the wait exceeds that
   bound, resynchronize or disconnect rather than retaining an old world
   snapshot. A shader failure cannot silently replace masked depth/shadow or
   render a misleading view; it also does not turn an otherwise compatible
   gameplay manifest into a different network schema.
3. **Prediction requires the physical dependency closure, not visual
   products.** A client that predicts boat/water contact needs the chosen
   E1 domain/edge state, forcing/source revisions, query support and P3
   coupling relevant to every replayed tick, with content generations aligned
   to N1's confirmed snapshot. A local high-detail ripple or fog LUT is never
   a replay input. If the sea/river boundary or dependent body state cannot be
   restored within the prediction budget, N1 discards prediction and presents
   confirmed/interpolated state as its draft already specifies. A client that
   does not predict that interaction need not run the server's complete water
   solver merely to display replicated results; required client queries must
   still be declared and content-ready.
4. **Correction has an E1/V3 history consequence.** N1's confirmed world tick,
   environment/source revision and relevant content/domain epoch must reach
   render extraction with the corrected sample. V2/V3/E1 reset or explicitly
   reproject medium, cloud, reflection and image histories when correction
   invalidates their support, light inputs or geometry; V4 alone cannot clear
   all effect caches. A small pose correction may use the declared smoothing
   policy, but no history may blend across an incompatible water boundary or
   changed media/light source as if it were continuous. A client's visual
   atmosphere approximation may differ by profile while using the same
   authority-owned gameplay weather/visibility facts. If cloud/fog visibility
   affects rules, its authoritative value/model belongs in the gameplay
   compatibility and replication contract; visual-only density need not.
5. **A source/build restart is not a live network-content mutation.** A01/A02
   pin an isolated play session to one admitted source/build. A remote peer
   joining it must negotiate that session's gameplay lock; a later local
   source sync or shader cook does not change the server session. A deliberate
   server gameplay-content update needs N1's new compatible session/epoch or a
   separately specified versioned migration/publication protocol. Local-only
   presentation tier switches can use the A03/F2 transaction without changing
   network authority or the server save root.

These are contract wording and projection/readiness gates for N1, F2/F4,
V1/V3, E1 and C02/C04/C05/C07/C09. They add no required transport, renderer,
prediction provider or water solver. The networking draft's detailed wire,
budget and provider qualifications remain later work.
