# Cloud capabilities and coverage

1 November 2026. E1.clouds commission for Thrum; the complete system design will be [CLOUDS.md](CLOUDS.md).
This first inventory precedes reconciliation with the old cloud code. The author has prior exposure to both
Broadsides cloud implementations and their failures; this is an independent target, not a claimed blind study.
Fresh reviewers receive the original requirements and Thrum contracts before this inventory.

## Scope and requirement basis

**U**: explicit cloud-user requirement. **E**: selected Thrum engine requirement/contract. **I**: necessary implication
of delivering those requirements. **P**: proposed scoped profile or bound awaiting qualification. **O**: optional
extension, not base-delivery acceptance. Decisions, implementation and evidence are distinct. All proofs below are
planned and unrun in Thrum; the legacy lab's media and timings do not qualify this target.

The user selects production-style clouds, coherent motion, genuine growth and breakup, reduced physical requirements
where useful, responsive Mac inspection and retention of the renderer where it fits. Approaching storms/weather
progression remain stretch goals. Thrum E03/E09–E12/E14/E17/E20 and C01–C09/C12/C13 add reusable composition, portable
profiles, common authority/time, media integration, lifecycle and cost obligations. No runtime build is commissioned.
The existing attenuation-only R3 recipe remains valid; visible volumes are a separately admitted extension.

Proof groups: **Q1** content/local lifecycle; **Q2** transport/lighting; **Q3** shared projections and views;
**Q4** motion/history; **Q5** composition/headless; **Q6** authoring/loading/lifecycle; **Q7** combined target costs;
**Q8** optional approaching-weather prototype. These labels denote coverage, not a new test suite.

## Required capability inventory

| ID / basis | Observable outcome | Responsible owner and important consumers | Design obligation; planned proof |
| --- | --- | --- | --- |
| CL01 U/E | A game selects clear, attenuation-only or visible-cloud capability combinations explicitly. | F2 composition; E1 cloud source; V3/V4 rendering | Dependencies, supported substitutes and rejection domains; Q5. |
| CL02 U/P | Clouds have convincing body and coherent silhouette from useful viewpoints; the proposed first regime is billowy cumulus-like content with a shaded base. | E1 optical geometry; V3 light; V4 display | Representation must retain required spatial scales and optical contrast, not just encode water mass; Q1/Q2. |
| CL03 U | Clouds visibly develop: local structures form, change connectivity, detach and erode. Rising billows are the proposed first regime; condensation is not a thermodynamic claim. | E1 evolution/content producer; all projections | Actual structural change and local creation/destruction. Global scale/fade or invertible warp alone cannot qualify it; Q1. |
| CL04 U/E | Cloud bodies and fine detail drift coherently with the selected wind at cloud height. | E1 weather/wind and cloud trajectory | A constant/authored wind is valid; shared frame/time convention, integrated transport, no double moving-frame offset or independent noise scrolling; Q1/Q4. |
| CL05 U/E | Normal-speed motion and accelerated development both have plausible pacing and stable identity. | F1 time; E1 source; authoring preview | Weather/illumination clocks are explicit mappings from committed time; quality never dilates simulation to hide cost; Q1/Q4/Q7. |
| CL06 I/E | Camera changes, culling and origin rebases do not reseed or move a cloud in world space. | W1 identity; E1 source; V2 extraction | Stable source identity/seed, world support, phase and source revision; Q3/Q4. |
| CL07 E/I/P | Supported illumination changes the same cloud and its transmitted light correctly; the first extension selects Sun plus a conditional Moon. | E1 celestial facts; V3 lighting | Directional source paths, daylight/source-off and angular validity; an authored Sun-only field cannot claim arbitrary directions; Q2/Q3. |
| CL08 U/E | Ground, vegetation, water and volumetric receivers see cloud attenuation from the same place/time/source. | E1 optical field; V3 direct lighting/media | Defined source-ray optical-depth query or typed unavailable result; no duplicate cloud attenuation; Q2/Q3. |
| CL09 E/I | A visible cloud, air and overlapping fog have correct depth order and one declared transport composition. | V3 integrator; E1 cloud/air/fog coefficient producers | Joint extinction/incident lighting; independent overlapping whole-ray composites are invalid; Q2. |
| CL10 U/I | Opaque geometry clips the actual cloud path without a horizon band or a transparent leak. | V3 path integrator; V2 depth; V4 reconstruction | Conservative volume support and cloud-relative occluder metadata; foreground, interior, far-ground and sky cases; Q2/Q4. |
| CL11 U/E | Reflections and environment illumination describe the current cloud geometry, not the main-camera image. | V3 reflection/environment consumers; E1 source | Independent query viewpoints, supported update age and source/path validity; water surface remains its owning module's responsibility; Q3. |
| CL12 E/I | Main, secondary, shadow and reflection views can coexist, including off-camera occluders. | V1 resources; V2 views; E1 shared source; V3 | Shared world products and bounded view-local history; no per-view source authority/evolution; derived copies must have justified validity and accounted cost; Q3/Q7. |
| CL13 E/I | History follows retained material where correspondence exists and rejects births, deaths and missing prior support. | E1 flow/correspondence; V3 effect history; V4 reconstruction | Evaluate the actual previous image's time, not automatically previous tick/frame. Topology change is not a rigid vector; Q4. |
| CL14 E/I | Lighting uses explicit optical units and a declared approximation with no source-off emission. | V3 closure; E1 extinction/phase inputs; V4 display | Nonnegative extinction, Beer view transport, normalized phase, labelled higher-scatter source approximation. One source off removes its term; darkness requires all direct/ambient/emission inputs zero; Q2. |
| CL15 E/I | Sky/background and cloud-derived illumination do not count baked cloud imagery or the Sun twice. | F4 sky assets; V3 light/environment | Compatible terminal background, source metadata and cache invalidation; Q2/Q3/Q6. |
| CL16 E | A game can omit main-view clouds while independently retaining attenuation or reflected/environment cloud appearance; only unused consumers retire. | F2 selection; E1 source; V1/V3 resources | Name retained-consumer combinations and shared-media history ownership. Explicit clear returns one; source absence is not silent clear; Q5/Q7. |
| CL17 E | A headless world supplies any required cloud/gameplay facts without reading GPU textures. | E1 CPU semantic provider; P6/N1 consumers | Gameplay queries are opt-in typed capabilities with declared projection/error; decorative cloud content does not imply rain, heat or concealment authority; Q5. |
| CL18 E/I | Source and camera data share committed identity/time while presentation can run at another cadence. | F1/W1 publication; E1 state; V2 packet | Immutable source revisions and bounded provider leases; no per-view stepping or unbounded old snapshots; Q4/Q6. |
| CL19 I/E | A concrete producer supplies versioned evolution, optical geometry and valid correspondence or explicit invalidity in declared frames/time/support. | F4 importer/cook; E1 authoring/source | Provider-specific data/authoring contract and reproducibility. Prepared clips, when selected, owe explicit channels/timestamps; no per-frame normalization masquerading as evolution; Q1/Q6. |
| CL20 E/I | Data activation and time discontinuities publish complete compatible source samples with bounded residency and explicit readiness. | F4/W2 products; E1 sampler; V1 uploads | Prepared sequences additionally specify bracketing, boundaries and any paging; evaluable providers need no invented sequence API. Stale-result rejection and bounded staging; Q4/Q6/Q7. |
| CL21 E/I | The selected finite source scale has explicit overlap, support and culling rules. | E1 spatial source set; V3 integral; V2 bounds | Sum constituents once and bound cost. Instance/group counts belong to selected profiles, not universal provider APIs. A bounded proof cannot certify regional scale; Q2/Q3/Q7. |
| CL22 E/U | Quality profiles preserve identity, source time and meaningful cues while removing real work/memory. | F2 profiles; E1 semantic source; V1/V3/V4 | Target-cooked representations and resource ledger, no silent loss of required shadows/gameplay meaning; Q5/Q7. |
| CL23 E/I | A supported content/profile replacement, source retirement, device loss or suspend/resume leaves no stale cloud/shadow/reflection/history. | F1/F2/F4/V1; E1 and effect owners | Default restart/stop-load for substantive changes; explicit retirement and reconstruction for admitted presentation changes; Q6. |
| CL24 E/I | Save/replay/network restoration retains the source facts the game actually needs. | P7/N1; E1 source; F4 pinned content | Persist authority/phase/source references, not GPU caches; cosmetic state can be derived or reset only under a declared cue policy; Q5/Q6. |
| CL25 U/E | Authoring and diagnosis make causes inspectable; Mac orbit/pan/pinch remain usable. | T1/T2 tooling; P1 input; E1 source | Proposed preset authoring, required inspectable causes, preview ordinary and accelerated motion, expose support/provenance/history validity without creating another weather authority; Q1/Q6. |
| CL26 E/U | Startup, memory, sustained cadence and input latency fit the assembled target recipe. | C12/T2 all owners | Cost account for source preparation, bandwidth, per-view/light/sample work, repeated updates and transition peaks; actual artifacts/devices required; Q7. |
| CL27 E/I | A game can register, replace or omit cloud providers through Thrum's ordinary composition model. | F2 public capability/schema/system declarations | Define required outputs, units/regimes, product types and removal dependencies; no Bevy host or local plugin framework; Q5/Q6. |
| CL28 E/I | Selected source content and preparation can be recovered and legally delivered. | F4 asset pipeline; T1 authoring; project source | Stable package IDs, hashes, licenses, recoverable masters/recipes, runtime payload distinction and bounded retained storage; Q6/Q7. |

## Optional capabilities and exclusions

| ID / basis | Outcome and boundary |
| --- | --- |
| CO01 O/U | A distant authored weather system can approach, increase coverage and change type coherently. Design source/time/control seams now; implement only after CL02–CL05 and coupled base proof pass; Q8. |
| CO02 O/E | Precipitation, storm gameplay, thunder or physical irradiance/thermal effects consume explicit E1 weather facts; they are not inferred from a dark cloud pixel. Their physical producers are separate commissions. |
| CO03 O/P | Flight, planetary/orbital sky, precipitation-bearing volumes and additional cloud families are separate regimes. The proposed full-volume base admits ordinary perspective first/third-person and orthographic ground/exterior views, including ground-accessible cloud entry, camera altitude0–1536m and path≤4km. A distinct distant/mobile recipe may exclude entry only through explicit game/camera/content admission, never a silent fallback. |
| CO04 O/P | Native high-tier lighting, ray tracing, sparse compressed brick acceleration or original research providers compete at typed seams after conformance and whole-target benefit. |

No live moist-fluid/LES solver, meteorological conservation, raw-Mie calibration or exact all-orders scattering is
required. No universal cloud genera programme, planetary simulation, automatic cross-provider state migration, paid
compute, asset import or runtime rewrite is authorized by this design commission. The baseline attenuation-only
recipe does not become dependent on the proposed volume extension. Exact numeric content capacities and extra
feature combinations must be selected with a cost account, not fabricated as measured support.
