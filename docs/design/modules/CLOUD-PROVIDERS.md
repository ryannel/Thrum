# Cloud and atmosphere provider composition

2 November 2026. Written contract targeting [F2 composition](../COMPOSITION.md) and
[C04/C08/C09](../CONTRACTS.md); no public Rust API or plugin runtime is implemented here.
The [selected baseline](CLOUD-PRODUCTION-BASELINE.md) is one replaceable provider recipe, not the public contract.

## Authorities and independently replaceable roles

E1/W1 owns committed weather causes/world time/shared wind. A selected weather provider may use authored fields,
procedural regional evolution or a later physical model, but declares one law/state owner per domain. The cloud source
consumes those causes and owns its private optical evolution. Clear-air, local-medium and cloud-light providers are
separate roles. V3 joins their coefficient/light inputs and owns transport; V4 owns final reconstruction/display.
Replacing source technique does not replace weather, invent another clock or duplicate wind.

Use existing typed F2 registration from game code or a plugin package. Declare owner/namespace, capability/schema/
provider versions, required and optional inputs, supplied outputs, supported domains/families/control response,
platform features, content/dependency hashes, state codec/replay level, actual cost/resources and build/load/live
support. F2 validates an explicit finite recipe; no automatic provider search, runtime WGSL function pointers,
universal binary ABI or per-event shader compilation. GPU implementations bind selected finite shader modules/
layouts at pipeline creation; the common semantics remain the same even when physical layouts differ.

## Representation-neutral handoff

| Input/output | Required meaning |
| --- | --- |
| WeatherFrame | World/epoch/tick/time/domain/revision; wind(m/s), fronts and declared normalized authored controls or physical units. Stable region/event/cause IDs and forcing history. Renderer pixels never supply authority. |
| SourceSnapshot | Immutable sample time, source/provider/recipe revision, stable region/group/lineage IDs, conservative support/error/domain and source-product leases. No public VDB, frame-pair, brick or private solver requirement. |
| Coefficient query | Point/segment in world metres, time, footprint/quality/material convention; nonnegative extinction/scattering(m^-1), normalized phase, conservative bound and validity/error. V3 can jointly sample overlapping media. |
| Optical-path query | Named directional endpoints/boundary and media/source set/time; dimensionless tau/T plus fidelity/error. A filtered radiance shadow cannot claim exact semantic visibility. |
| Correspondence | Actual retained prior source time/position, confidence and birth/loss invalidity. Unsupported mapping rejects history; no guessed previous frame. |
| Source preparation demand | Union of actual main/reflected/environment/light domains, including offscreen casters; required quality/error and deadline. Private preparation may be analytic, sampled, cached or streamed. |
| Storm causes | Identified support/time and explicitly authored rain flux(kg m^-2 s^-1) or potential, lightning event energy/light parameters where supported. Potential is not flux; no cause inferred from image darkness. |

Clear is valid sigma=0/T=1. Missing, stale, outside-domain and capacity failure are distinct unavailable statuses.
Real independent constituents add once. Parts/LODs representing the same condensate partition/substitute once.
Only disjoint or jointly-owned C08 segment operators may be preintegrated; a completed cloud sky image is insufficient
for arbitrary fog overlap or independent reflected geometry. Each retained consumer gets its own query domain/history.

## Two concrete selectable recipes

| Configuration | Selection and validation |
| --- | --- |
| `runtime-volume-sky` (commissioned default) | E1 regional live weather/wind + native3D source + Hillaire air + local fog + voxel/Beer cloud light + independent views. Requires CF01–05/CW01–04, local renewal/loss, live wind/front response, self/receiver/air/fog shadows and independent reflected appearance in the declared camera domain. A static playback provider cannot satisfy these response capabilities. |
| `shadow-only-world` (substitution witness) | Same E1 weather/wind/cause frame + independently implemented runtime2D authored coverage/column-tau provider + Sun attenuation receiver adapter; no visible cloud, self-light, main cloud march or cloud reflection promise. Suitable for the corresponding top-down profile only. Retained exact physics/visibility queries must be declared separately; it cannot be selected for the full profile. |

P0/P3a implement just enough adapters and finite setup validation to prove the second recipe can be selected without
importing the3D source's layout. Use a known analytic attenuation footprint and the shared clock/wind as oracle.
Select invalid combinations deliberately and get a useful error before activation. No competing full sky renderer
needs to be built merely to demonstrate replacement. Optional prepared3D/VDB, procedural weather-map or physical
sources can later implement the same contract; none is selected by this document or silently substituted after failure.

## Lifecycle, state and off cost

Build omission removes unused source code/assets. Load-time choice validates dependencies, prepares content/pipelines,
and activates at the world/extraction barrier. Replacing unrelated running providers requires stop/load/new epoch and
validated save/source compatibility, or rejects the save. No automatic conversion between arbitrary private states.
An explicitly admitted visual tier switch can prepare old/new products, atomically publish, invalidate affected
histories and retire by CPU leases/GPU fences; count peak overlap. Seamless hot swap is optional.

Weather and cloud-state updates happen once per world/source step independent of view count. Optical q may live on
GPU as derived visual state; E1 CPU causes, wind, rain/lightning and physical response never need
its pixels/readback. Headless gameplay visibility is separately authored; headless physical cloud attenuation needs
an explicit CPU/analytic optical provider with declared accuracy, never weather potential or cosmetic GPU readback. The source advertises cosmetic determinism/recovery separately from semantic replay. Save/reset records source codec/version and recipe/seed/forcing. Default reset-optics explicitly starts a new optical
epoch after discarded all-off state/device loss/incompatible seek; preserve semantic weather/event IDs, report
unavailable until initialized, invalidate histories and claim no uninterrupted material continuity across reset.
Continuous-optics retains source updates when needed, requires completed checkpoints<=60source-seconds and a bounded
<=60s forcing ring; missed capacity reports unavailable/rejects continuity. Count async checkpoint staging/durable and
load replay work. No cross-platform bitwise GPU promise. Each SourceSnapshot records its actual WeatherFrame revision,
source tick and presentation interpolation interval; FrameOptics/leases bind exactly those revisions, never just the
latest world tick label.

Main draw off retires only main passes/history while shadow/environment/reflection demands remain. No optical demand under reset-optics
retires source visual updates and GPU pages/light/history after readers finish; continuous-optics retains its declared
source update/recovery cost; demanded E1 weather/causes continue.
Re-enable follows the selected reset or bounded-checkpoint policy; never pretend a frozen source stayed current. Headless facts require no graphics device. If every cause/optical consumer is removed, its provider's avoidable
updates/storage retire. Culling cannot erase rain/lightning events needed elsewhere. Quality may reduce visual detail,
but cannot silently change gameplay wind/flux/visibility laws.

## Proof obligations

P0/P3a demonstrate registration, full versus shadow-only recipe selection/rejection and no layout leakage. P4a
changes wind/drying while main is hidden: receiver/environment/mirror and cause IDs respond at the same source time.
P6 uses two views, rain/lightning stubs, replay/checkpoint recovery and all-off/restore. P8 repeats against actual
Thrum F2/F4/E1/V1–V4 APIs and real water/surface/effect owners; the standalone adapters are not public API maturity.
