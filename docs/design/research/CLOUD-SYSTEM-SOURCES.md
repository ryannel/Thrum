# Cloud system evidence and analytical choices

1 November 2026. Supports [the governing cloud design](../modules/CLOUDS.md), not a running provider.
Three bounded Sol High workers independently covered requirements, evolution and transport. Parent synthesis owns
the resulting combination; it is not an assertion about private studio internals. Slides/notes and source documents
were inspected; no conference recording was watched in this commission. No bake, renderer or target benchmark ran.

## Inspected sources and consequences

| Source / inspected locator | Support and limitation | Adopted consequence |
| --- | --- | --- |
| [Guerrilla Nubis Cubed,2023](https://www.guerrilla-games.com/read/nubis-cubed), [deck](https://d3d3g8mu99pzk9.cloudfront.net/AndrewSchneider/Nubis%20Cubed.pdf), physical PDF pp.8–9,82,87–90,198 (printed slides6–7,80,85–88,193) | Prepared fluid-derived clouds, profile/detail/density fields and up-rez. The final comparison labels voxel evolution pseudomotion; it does not demonstrate the commissioned structural lifecycle. | Keep geometry/light separation and prepared detail as production evidence. Supply actual time-varying optical content; do not attribute it to that renderer. |
| [IO Interactive Smolder,SIGGRAPH2026](https://www.advances.realtimerendering.com/s2026/), [notes PDF](https://www.realtimerendering.com/advances/s2026/content/AlexanderMueller_Smolder_Siggraph26.pdf), pp.51–64,101–104,111 | Offline VDB animation, sparse bricks/min-max packing; whole-animation residency had significant memory cost. Per-frame streaming and runtime fluid evolution are future directions. Cloud lighting remains a stated improvement area. | Select finite final-density clips with bounded whole-source residency, adapting packing. Flow interpolation, history and portable limits are our additions, not Smolder guarantees. |
| [Stormscapes2020](https://computationalsciences.org/publications/haedrich-2020-stormscapes.html), [paper](https://computationalsciences.org/publications/haedrich-2020-stormscapes/haedrich-2020-stormscapes.pdf), sections4.1–4.3 and Table1 | Actual atmospheric/phase/buoyancy mechanisms for cloud formation. Published hardware/step timings do not establish portable frame-time performance. | Retain as offline/reference lineage or optional stronger provider; no compulsory live meteorology. |
| [SideFX Pyro solver](https://www.sidefx.com/docs/houdini/nodes/sop/pyrosolver.html), Fields/Density/Output; [background](https://www.sidefx.com/docs/houdini/pyro/background), simulation fields | A concrete external tool can export time-varying density/velocity. Soot density is not meteorological cloud water. | Optional master creation path with explicit fixed optical remap. Standard retained VDB/master inputs, not a proprietary runtime dependency. |
| [Epic volumetric clouds](https://dev.epicgames.com/documentation/unreal-engine/volumetric-cloud-component-in-unreal-engine), multiple scattering, shadow modes and reflection sections | Bounded approximate scattering and consumer-specific quality/shadow choices. Its platform/support statements do not qualify Thrum or promise the same visible representation on mobile. | Named effective cloud source, direct/view extinction unchanged, separately admitted reflection/mobile modes. |
| [WebGPU2026-09-15 texture tables](https://www.w3.org/TR/2026/CRD-webgpu-20260915/#plain-color-formats), [WGSL2026-10-26 pack2x16float](https://www.w3.org/TR/2026/CRD-WGSL-20261026/#pack2x16float-builtin) | Sampled scalar half does not imply portable scalar-half storage writes; format/filter features are independent. Core packing/copies allow a bounded alternative. | Pack/copy R16 current fields; filtered RGBA16 depth caches; load-only RG32 max/residual. Validate actual limits and all bindings. |

## What the synthesis adds

Final optical frames encode actual local creation/erosion and topology change. A stationary interval carrier provides
correspondence only where material residual and error allow it. The characteristic derivation in CLOUDS explains the
signed residual; it is not a meteorological conservation claim. Final detail is baked coherently, avoiding a new
runtime rest-coordinate system. Source interpolation remains C0 and can fail visual cadence; that is an explicit
content/temporal gate, not a missing formation mechanism delegated to an experiment.

The primary source uses512 m/128^3 rather than the worker's2 km/128^3 worksheet. Bytes are unchanged; voxel scale and
motion bounds are not transferable. The parent derives the midpoint carrier bound and selects a1 m fit gate plus
independent optical references. Whole encoded GPU residency is selected over decoded pair paging for the first finite
library, amortizing source evaluation into a current field shared by every view/light consumer. A generic provider
is not forced to expose clips or groups.

V3 integrates shared coefficients and direct lighting. Three extra directional cloud source terms plus coarse sky
ambient are declared approximations, not a converged radiative-transfer solution. View extinction, shadows and any
semantic transmittance query retain ordinary optical meaning. Scene-linear display, reflection boundaries, source
participation and actual prior-image times remain owned by Thrum's existing contracts.

## Alternatives rejected or bounded

- A static detailed shape with invertible sine shears and global opacity/scale cannot supply local structural development.
- A custom partial moist solver with more resolution is not selected merely because its conservation checks improve.
  Its forcing, turbulent scales and optical presentation would each still need a reasoned complete design.
- Coarse shape plus runtime noise can be excellent for an admitted regime, but dynamic morphology needs a continuous
  material-detail map and birth/reseed/history rules. We avoid inventing those to reduce source bytes.
- Positive lobe kernels minus local erosion kernels can mathematically create merges/splits with tiny content, but
  their characteristic failure is smooth inflating/cutting blobs. They remain an explicitly stylized alternative,
  not the reliable default for this user target.
- Full runtime fluid/thermodynamics adds coupling, stability, CPU/server/save and device obligations not required here.
- Fine-grained frame/network streaming can reduce large-library residency, but it adds deadlines, upload windows and
  seek/fallback obligations. The finite initial bundle does not need that infrastructure.
- FP4 packing and very coarse distant fields are optional quality/error-qualified products, not default reductions
  imposed to make a cold-start worksheet pass. The high-cap R8 source fails the original cold5s prediction; the portable
  cold recipe separately limits encoded bytes while preserving voxel detail/precision and the required cues.

## Historical reconciliation evidence

The parent had prior exposure to the earlier cloud systems. After saving the first capability target, it inspected
the production state/material map, density, depth-guide, temporal resolve and pass/resource roles at Broadsides commit
`d81f29611d90b42cf6e2d7a618dde7216ebaa16a`, branch `codex/clouds-proof`. The retain/adapt/replace decisions are in CLOUDS.
Existing static appearance, responsive Mac controls and the corrected horizon guide are useful evidence of their
scoped behaviour; old movies do not qualify the requested lifecycle. Old cost artifacts apply only to their named
builds/host and are not copied into Thrum's performance column. No code/assets have been imported and no recovery
path depends on a local Broadsides checkout.

Remaining evidence is source-content qualification, numerical optical/cache/correspondence error, ordinary and
accelerated moving quality, provider-off/resource conformance and complete target cost. No source asset, frame time,
thermal behaviour or user acceptance is marked passed by this design work.
