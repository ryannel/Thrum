# Parallel module start and Broadsides reconciliation — 2 November 2026

> **Evidence correction:** The Broadsides inspection below covers its historical
> TypeScript game/foundation paths at the stated checkout. Broadsides also has
> active Rust engine, cloud and water worktrees that were discovered after this
> first pass. Their design, code and qualification are assessed separately in
> the module research and cross-system synthesis. Do not treat the TypeScript
> findings or older cloud/forest reports here as a verdict on those Rust
> candidates.

## Scope and evidence level

This is the first design and read-only code reconciliation pass for the parallel
render/environment/world work proposed during I2a. The governing module designs
remain `docs/design/modules/RENDERING.md`, `ENVIRONMENT.md`, `WORLD-CONTENT.md`
and `PLAY-SYSTEMS.md`, together with `RENDER-SPEC.md`, `CONTRACTS.md` and the
water commission. This record does not select Broadsides as an implementation,
change Thrum's current I2a integration order, or claim running, visual or device
quality. The author had read Thrum's existing Broadsides provenance and water
alignment references before writing this target; this is independent of a fresh
inspection of Broadsides code, not a blind study.

## Thrum target recorded before inspecting Broadsides code

The product brief requires reusable 3D games with orthographic, isometric,
third-person and first-person views; selectable capabilities; native desktop,
WebGPU browser and mobile profiles; first-class authoritative multiplayer; and
quality suitable for public demonstration. R0 is a small opaque/masked rendered
game. R1 combines two views, characters, light, fog, physics and audio. R2 retains
gameplay meaning with cheaper presentation. R3 joins water, cloud attenuation,
air/fog, hull contact and save. These are different proof workloads, not one
ever-growing default scene.

| Target | Required owner and first bounded behavior | Critical shared boundary | Quality and cost rejection condition |
| --- | --- | --- | --- |
| Render core | V1 owns one device/submission/resource lifetime; V2 extracts committed scene per view; V3 evaluates material/light/transport; V4 reconstructs and displays. First product draws R0 from a pinned cooked revision. | C05–C08: exact shader/pipeline product, one light registry, depth and motion conventions, per-view histories; E1 supplies causes, not a second renderer. | Reject duplicated Sun attenuation, stale pipeline/product publication, unbounded per-view copies, or a feature that needs a private submit/display path. Measure full pass/submission/working-set cost and visual stability, not only shader time. |
| Shared environmental source | E1 owns time-indexed Sun/Moon, wind, cloud optical-depth and precipitation facts with units, support, interpolation and validity. Start with constant/keyframed providers and queries that run headless. | C02/C08/C09: water, cloud shadows, lighting, vegetation, surfaces and audio sample the same committed cause; visible sky/clouds can be absent. | Reject independent weather clocks or cloud pixels/particles as gameplay truth. With no retained consumer, off removes simulation/preparation/resource work; with a retained consumer, retain only its necessary source. |
| Terrain and surface | W2 owns authored region/geometry/product activation; E1 surfaces own material identity and accumulated wetness/deposit state. Start with one static shoreline/ground fixture, stable IDs and collision/render products from one source revision. | C01/C05/C09: the water bed/contact, cloud-shadow receivers, physics and render LOD reference compatible geometry and epoch; wetness consumes precipitation once. | Reject visual-only shore alignment, duplicate wetness accumulators, missing collision when a region is visible/traversable, or stale products after edit/reimport. Count cooked bytes, activation peak, CPU queries and GPU residency. |
| Water–body interface | E1 water owns domain and physical surface/flow query; P3 owns body integration and contact; one tick boundary carries an identified reaction record. The initial R3 profile logs the body's reaction against an external reservoir and does not feed it back into waves or shallow cells. | C03/C09 and water design: query support/time, hull geometry, source/edge semantics, once-only reaction and save identity. | Reject a visual displacement treated as buoyancy, a force applied twice, or an unsupported physical seam silently blended. Check numerical residual, ordinary motion and coupled CPU cost. |
| Vegetation response | Cooked plants have stable placement and bounded LOD; E1 wind drives deformation; V2/V3 receive current and valid previous geometry. Initial work is design plus a small wind/shadow fixture after source/render interfaces settle. | C07/C09: one wind field, two-view temporal sample times, shadow invalidation and optional persistent damage. | Reject cards or instances whose motion/shadow history shimmers or whose culling suppresses a required physical/visual effect. Measure alpha overdraw, shadow submission, memory and sustained device cost. |

The R0 render path should not wait for full water, clouds, terrain streaming or
vegetation. Their designs can proceed against the shared contracts; their
integrated implementations wait for the relevant I2a and V1/V2 handoffs. The
initial whole-engine frame and memory envelopes in `BASELINE-SPEC.md` are
proposals, not certified budgets. A module cannot independently spend the whole
render or staging allowance. Preserve a small public registration/profile model;
specialized private layouts and prepared products are welcome when they have
clear invariants and measurable benefit.

## Broadsides code inspection and disposition

Inspected `/Users/ryannel/Workspace/Broadsides` at HEAD
`02dc688556263b34e74d4a468328d8e25e031690` (2 November 2026), plus its
working tree. The inspected `game/src/gpu`, `game/src/sim/water`, `game/src/sim/world`
and `labs/game/src/foundation` paths were clean at inspection. The larger tree
contains unrelated modified/untracked work, including water-lab changes. The
untracked `notes/research/sea-physics/FOUNDATION-CODE-REVIEW.md` supplied leads;
specific findings below were checked against source. No Broadsides runtime was
run or image reviewed in this pass. Its older visual/performance reports remain
dated evidence, not a current Thrum result.

| Area and inspected producer → consumer path | Fit to Thrum target | Disposition |
| --- | --- | --- |
| `game/src/gpu/Renderer.ts` → `core/module.ts` → `rendererOpaque.ts`: one renderer owns pass order, encoder and submission; modules contribute stages. `setOn` skips inactive module callbacks and water-only copies. | The single submission owner and conditional water work fit V1/C06. The stage contract and `OceanFrame` carry ocean, exposure and presentation decisions together; a disabled module is skipped but most of its previously allocated resources remain. | **Retain the ownership principle; redesign the API.** V1 receives versioned render packets and validated pass/resource declarations. F2 distinguishes build omission, load-time provider selection and a live review switch; each has its own retirement semantics. Do not port the stage interface as Thrum's public plugin model. |
| `gpu/sky/SkyLight.ts` → `Renderer.LightInputs` → scene, sky, water and post: finite Sun/Moon light, atmospheric tables and an incident meter. `gpu/sky/CloudAdvection.ts` integrates wind over time; `skyLayout.ts` defines cloud/shadow resources. | Valuable formulas, fixtures and failure examples. `SkyLight` also adapts exposure on CPU while `post/AutoExposure.ts` owns another adaptation path. The ocean renderer includes planetary/sea-specific paths beyond R0/R1. Broadsides' own 27 October code-review lead reports several competing visibility, air and light paths; this pass directly checked the CPU/GPU exposure chain and separate ground sources. | **Re-research and redesign the assembled light/media/display path against C08.** Keep V3 transport, E1 source facts and V4 exposure/display as distinct owners. R0 uses constant ambient plus direct light; R1's selected joint single-scatter air/fog is the portable first operator. Finite-disc or richer sky techniques compete later at a named quality/cost seam, with one contribution per physical light path. |
| `sim/world/WorldSim.ts`/`Weather.ts` → live sea versus `sim/water/weather/weatherState.ts`/`windColumn.ts` → foundation `lighting.ts`/`engine.ts`: a seeded changing game-weather state and a separate pure held weather-column model feed different scenes. | The pure, unit-bearing wind/air calculation and deterministic sampled-time idea are useful. These are two game/lab contexts, not a shared Thrum environmental authority. The foundation assembles wind, cloud cover, haze, Sun and surface air from settings for each render frame. | **Redesign E1 source schema and sampling.** One committed input/source revision and time maps to typed fields. A constant/keyframed source is the first provider. A later evolving weather provider can replace it at a declared load/transition boundary without changing water, vegetation or lighting query meaning. |
| `gpu/scene/terrain.ts` → terrain mesh/standing positions; `labs/game/src/foundation/clearingStream.ts` selects a different stream-refined height; `gpu/scene/farStand.ts` still samples the base terrain. `clearingContact.ts` selects a few fixed object blockers. | A pure height function and matched mesh grid are useful for a bounded heightfield provider. They do not establish one authored terrain/collider/nav/water-bed product. The source check confirms that base and stream-refined height paths coexist. | **Restructure around W2/F4 product closure.** Give render, collision, water bed, placements and shadow/air support compatible geometry revision and stated approximation error. A heightfield is one provider; caves/interiors use other geometry. Do not import lab terrain or generated assets as the world model. |
| `gpu/scene/SceneModule.ts` → `vegetation.ts`/`vegetation.wgsl` → colour, shadow and motion; `CanopyLight.ts` prepares crown visibility. Stable vegetation keys and one deformation function across passes are strong local ideas. | Wind response is a periodic 8 s visual bend; it is not growth, breakage or a general physical wind field. The source-to-canopy/ground/air visibility agreement is incomplete in the dated forest review. Per-frame `SceneModule.update` builds several placement arrays, while `instances.ts` maps/copies/group-buckets all placements and `SceneMotion.ts` makes a fresh map and pose slices. | **Retain the deformation-coherence rule and stable identity; redesign preparation and visibility.** Cook placement/LOD/asset tables, keep persistent instance storage with change lists and per-view visibility, evaluate prior deformation at each retained sample time, and let one declared geometry/visibility product serve affected consumers. Benchmark before choosing exact CPU/GPU split. |
| `sim/water/ocean/OceanQuery.ts` → `WaterSim.ts` → strip-body stations: preallocated typed arrays, batched phasors and explicit convergence/error support; `WaterSim.resetWakeHistory` documents a GPU/CPU wake mismatch after one-body teleport. | Excellent comparator and optimization candidate for a prescribed sea. It is sea/hull-specific and does not supply the selected finite shallow domain, P3 body authority, complete source/save closure or Thrum's bounded heterogeneous edges. | **Retain equations, fixtures and cost hypotheses as reference; implement against Thrum's water/P3 design only after the shared query/force/identity boundary is closed.** No direct TS port or production claim. |
| `gpu/scene/loadSceneAssets.ts` → synchronous `SceneModule.uploadAssets`: network fetch/decode, fallback boxes, then GPU upload. | Useful failure visibility and asset parsing examples. It lacks Thrum's admitted exact source/build identity and atomic required-product activation. | **Replace the lifecycle.** F4 cooks/pins immutable products; W2 admits required collision/game products before a region opens; V1 prepares pipelines/uploads and publishes optional visual products against their version. Required missing content refuses rather than silently changing gameplay. |

The source inspection supports a **new Thrum integration design**, with selective
reuse of mathematical ideas and tests, not a wholesale renderer or forest port.
Broadsides' 25 October forest review reported roughly 845–918 MiB of *tracked*
GPU allocations for that specific clearing/courtyard configuration and no
representative 60 fps certification. That is not directly comparable to Thrum's
R2 browser 512 MiB *total* planning envelope, but it rejects any assumption
that the current forest stack can simply be enabled in the portable profile.
The 27 October cloud issue brief records unresolved gold plates and black
layers in its then-current renderer. Neither a numerical safety scan nor an
attractive still closes those visual issues; this pass has not established their
status in another checkout.

## Cost model and quality gate for the next design increment

At R1's authored 4,096 static instances, one complete per-frame placement walk
at 60 Hz visits 245,760 placements per second. The inspected scene path makes
several such passes and allocates intermediate objects; two views and shadows
add independent visibility and submission work. This count is a workload
multiplier, **not a measured Broadsides frame time**. The Thrum candidate should
separate stable cooked placement/asset data, changed world transforms and
view-specific visibility, with persistent scratch and explicit dirty/version
tracking. A reference full rebuild remains a correctness oracle. Measure
incremental wins against edit churn, teleport, two cameras and changing LOD;
specialization is retained only with equivalent image/query semantics and
whole-frame evidence.

For every candidate, report CPU tick, extraction, prepare and submission
distributions; GPU per-pass and complete-frame span; bytes uploaded; pipeline
count/readiness; resident and transition-peak bytes; and off-state residual
work. Use R0, R1/R2 and the optional R3 interaction separately. Hold exposure,
camera, source revision and weather when comparing images; inspect motion,
shadows, fog/cloud boundaries and day/night transitions at native resolution.
The selected whole-engine envelopes in `BASELINE-SPEC.md` constrain the
assembled recipe. A single fast shader or passing test is insufficient.

Primary research checked for this pass: [Hillaire 2020](https://sebh.github.io/publications/egsr2020.pdf)
demonstrates a scalable sky/atmosphere method over a broader camera domain than
Thrum's initial R1 operator; [Bruneton's documented 2017 implementation](https://ebruneton.github.io/precomputed_atmospheric_scattering/index.html)
shows why atmosphere equations, texture mappings and tests must be inspected
together. Neither paper proves Broadsides' assembled cloud/forest result or
selects a full planetary path for Thrum. The [WebGPU specification](https://gpuweb.github.io/gpuweb/#dom-gpudevice-createrenderpipelineasync)
warns that immediate pipeline creation can stall queue work, supporting V1's
selected prepare-before-first-use gate. These sources corroborate the existing
Thrum specifications; no new algorithm is selected by this brief check.

## Dependency-ordered next proof and build queue

1. **R0 render closure after I2a admission:** pin one source/build revision;
   cook/validate a finite WGSL/material/pipeline closure; prepare on the target;
   draw one camera, opaque/masked material, Sun and local lights; save/edit/restart.
   Reject stale or unavailable products and graphics work in the headless recipe.
2. **Shared environment cause before visible-cloud implementation:** define
   versioned constant/keyframed Sun, wind, cloud optical depth and rain fields
   with headless point/flux queries. Show one Sun/cloud factor on ground, water
   and vegetation across two views while visible clouds are off. Reject any
   second clock or duplicated attenuation. Close producer/consumer details in
   C08/C09 before dependent code.
3. **One shoreline geometry/source proof:** cook one authored ground/bed and
   immutable collider/render/shadow products. Move camera and a body over the
   shore; edit/restart; verify height/contact/visual errors and old-product
   retirement. Reject a geometry representation that works only for pixels.
4. **R3 water/body/media coupling:** sample one declared water domain at tick n,
   integrate a body once through P3, log the identified external-reservoir reaction, and render
   water/fog/cloud shadows from the same state. Reject unsupported edge modes,
   lost/double forces, depth or transport inconsistencies, and unbounded peak
   memory. This precedes broad water/forest content production.
5. **Vegetation and two-view history:** shared wind and stable plant IDs drive
   colour, depth, shadow and prior deformation at each view's actual retained
   sample time. Compare static and moving views, cuts, LOD transitions and wind
   changes. Reject shimmer, stale shadow, fabricated velocity and unmeasured
   disabled costs.

This is a **scoped self-review**, not independent module review. It covers the
named code paths and dated reports, not all Broadsides source or a running
visual inspection. Before a substantial Thrum implementation, each owning
module design needs a deeper decision check and a producer/consumer walkthrough;
the larger interactive water/cloud/forest ambitions stay open until their
specific mechanisms and integrated quality qualify.
