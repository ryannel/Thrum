# Atmospherics technique selection and source map

2 November 2026. Current runtime selection/research support for the [cloud migration](../modules/CLOUD-MIGRATION.md).
The [production baseline](../modules/CLOUD-PRODUCTION-BASELINE.md) now locks the mechanisms. This is a selected implementation specification with qualification gates, not a claim of completed implementation or a recovered
studio architecture. The [capabilities](../modules/CLOUD-CAPABILITIES.md) retain the full commissioned experience.

## Selection

| Responsibility | Selected direction | Basis and important limit |
| --- | --- | --- |
| Clear atmosphere, horizon and aerial perspective | Hillaire's complete transmittance + multiple-scattering + sky-view + aerial-perspective LUT family; RGB aerial extinction is our extension/validated alternative to mean-alpha fast AP; a bounded direct segment integrator for spatial shadows/overlap | Author paper and MIT comparison code; Bevy's current sky/PBR/environment joins. A Sun-only transmittance LUT does not complete this system. |
| Cloud morphology and lifecycle | Native runtime sparse3D optical control, bounded advection/local source-loss and persistent IDs under live weather; no Houdini or lifecycle clips | Nubis³ establishes3D morphology/up-rez/rendering; our live optical source is an explicit adaptation. Smolder supports an optional prepared-volume provider, not the required dynamic lifecycle. New content must demonstrate local renewal and topology change. |
| Runtime density | Cooked low-frequency fields, bounded detail and conservative occupied-region traversal; projected-footprint LOD | Nubis Evolved/³; Smolder brick occupancy. Detail noise decorates the lifecycle and cannot create the missing lifecycle by assertion. |
| Direct light and cloud shadows | Receiver Beer shadows; current3D point-to-Sun visibility cache plus2local Sun samples, complete direct-ray oracle/fallback | Nubis³ slides125–127/154 disclose voxel tail caching plus2local samples; Epic documents BSM/secondary rays. Our dimensions/current cadence are adaptations; a column total cannot give arbitrary interior tau. K48 is retired from baseline. |
| Cloud multiple scattering | Two-lobe HG with liquid/ice settings, primary plus one additional MS approximation octave; current runtime voxel ambient accessibility and disk-excluded clear-sky upper-hemisphere mean | Epic single-octave game recommendation and published cloud-lighting lineage. Fitted liquid closure is an optional comparator, not a prerequisite or accepted thick/ice light model. |
| Ambient and reflections | Current same-source sky/cloud radiance generation, filtered environment/irradiance products; independent local reflected rays where parallax matters | Bevy source, Patry 2021 and RDR2's stated consumer architecture. Constant blue ambient and a perfect mirror do not prove clouded diffuse lighting or rough water. |
| Local fog / overlapping media | Integrate summed coefficients/source in overlapping intervals; compose disjoint radiance/transmittance segments once | Godot integration code and transport algebra. Use bounded volumes first; froxels become a selected dense-volume tier only if their combined cost wins. |
| View cost | Disjoint320x180far/640x360near rays, independent histories/current spatial fallback and bounded fresh samples; selective lighting-rate experiments optional | Nubis lineage and actual Takram resolve code. Fresh growth, cuts and disocclusion must receive current samples; old history cannot supply new matter. |
| Weather | Live regional fronts/controls, persistent source state and W applied once in source transport under E1/W1 | Retained research and Guerrilla's regional authoring/scheduler. Scientific meteorology is optional; persistent local growth and coherent weather remain required. |

We select these principles together. Copying a dense console texture layout, a 16-frame refresh window or a shipped
millisecond number would not establish suitability for Thrum. Format support, memory, content cadence and simultaneous
consumers are qualification conditions. Full runtime fluids, neural radiance and hardware-ray-tracing dependencies
are challengers/reference paths, not prerequisites for the browser production baseline.

## Current production adoption check — 2 November 2026

- [Epic UE5.8 cloud documentation](https://dev.epicgames.com/documentation/unreal-engine/volumetric-cloud-component-in-unreal-engine?lang=en-US)
  was rechecked for volume ray marching, BSM/secondary shadow paths, one MS octave, sky capture and coupled lighting.
  These are established major-engine features, not evidence of our exact pipeline or browser timings.
- [Guerrilla Nubis³](https://www.guerrilla-games.com/read/nubis-cubed) describes the move to voxel clouds in *Horizon
  Forbidden West: Burning Shores*, prepared fluid/model morphology and sampling/light acceleration. This supports
  prepared3D source architecture; it does not publish our five-family evolving library.
- [Smolder2026 speaker notes](https://www.realtimerendering.com/advances/s2026/content/AlexanderMueller_Smolder_Siggraph26.pdf)
  support shipped baked animated-volume production. Its whole-animation-memory limitation and future-streaming note
  are retained; our bounded frame-window streamer is conventional texture IO, not a claimed Smolder feature.
- Optional authoring reference, **not a dependency**: [Houdini22.0.459 production announcement](https://www.sidefx.com/forum/topic/104614/) records the1October2026
  release; the previous selected toolchain is superseded. [Cloud modeling](https://www.sidefx.com/docs/houdini/model/cloud.html) and
  [sparse Pyro](https://www.sidefx.com/docs/houdini/nodes/sop/pyrosolver.html) supply concrete authoring operations.
  No installation/licence or full-episode cache is required by the current native generator.
- [Epic animated SVT documentation](https://dev.epicgames.com/documentation/unreal-engine/sparse-volume-textures-in-unreal-engine?lang=en-US)
  provides adjacent-frame/interpolation/page/prefetch API precedent, but the product is explicitly **Experimental**.
  Do not call it shipped adoption proof or import its implementation. Our runtime is a selected independent portable
  volume implementation; its IO/quality/device admission is unrun.

“Modern through2026” means selected methods with current production support, plus clearly labelled adaptations.
New neural/Gaussian methods, hardware-specific light sharing and custom transport compression are optional; newest
publication date alone does not establish adoption. Accessible pinned code below teaches pass ordering/math/resources,
with source-specific rights and portability limits. No private Decima/UE runtime was accessed.

## Visual-first selection and dynamic source adaptation

[The new selection review](../../evidence/cloud-dynamic-revision-2026-11-02/visual-selection.md) uses primary independent
reviews of Burning Shores for its3D cloud appearance, including reviewer text rather than only developer marketing.
It does not claim universal consensus or that visual praise proves an unpublished runtime weather model. Full DF
video motion was not watched here. The selected default retains genuine3D morphology/rendering economies rather than
calling an older2.5D weather-map method equivalent to that close-cloud result. Compare cheaper methods only within
an explicitly accepted distance/quality domain and complete CPU/GPU/memory scope.

Root re-read retained Nubis³ speaker notes slides123–129,147–154 and171–174 on2November: decoupled Sun samples stored
in a256x256x32 voxel grid after the first2local samples; sky-direction accumulated density for ambient; console test
cloud costs2.2–4ms at960x540 with named optimizations. Those are **their** source/view/workload, not a Thrum prediction.
Our point caches use complete source-to-boundary tails, current generation and independent consumers, with measured
error/fallback and full update cost. No8-frame amortization transfers automatically to fast-changing weather.

The native source uses bounded MacCormack semi-Lagrangian scalar advection plus exact positive local reaction and
authored internal velocities/phase-class transfer. [Selle et al., author paper](https://faculty.cc.gatech.edu/~jarek/papers/maccormack.pdf)
supports the advection/limiter, not cloud art quality. Root inspected the author-hosted abstract and limiter/reversion
passages after the original Stanford locator returned404. Genus rules and the reduced E1 control law are explicit Thrum
choices; [source alternatives/reconciliation](../../evidence/cloud-dynamic-revision-2026-11-02/source-review.md) retains
why a full thermodynamic/parcel model is optional. No whole-publication combination is claimed production-proven.

## Primary research and material actually inspected

These locators distinguish substantive study from discovery. No full conference recording was watched in this review.
The 2024 video observations are selected stills; the tutorial below was located but its transcript was unavailable.

| Primary material | Inspected scope and design consequence |
| --- | --- |
| [Hillaire, EGSR 2020 paper](https://sebh.github.io/publications/egsr2020.pdf), §§3–7; [2020 shading slides](https://blog.selfshadow.com/publications/s2020-shading-course/hillaire/s2020_pbs_hillaire_slides.pdf), slides25–44 | Complete compact clear-air LUTs, source closure, limits, cloud BSM and reconstruction tradeoffs. Higher-order clear-air scattering assumes approximate isotropy; it is not local cloud/fog multiple scattering. |
| [Nubis 2015 deck](https://advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf), slides38–46,92 | Weather/type maps, authoring and interleaving. Historical baseline, not the complete modern target. |
| [Nubis 2017 notes](https://advances.realtimerendering.com/s2017/Nubis%20-%20Authoring%20Realtime%20Volumetric%20Cloudscapes%20with%20the%20Decima%20Engine%20-%20Final%20.pdf), physical pp39–50,53–72,94–102 | Regional weather scheduling and cumulative optimizations. Physical p63 explicitly limits ordinary evolution to detail rather than formation; weather-state blending is not individual growth. |
| [Nubis Evolved 2022](https://advances.realtimerendering.com/s2022/SIGGRAPH2022-Advances-NubisEvolved-NoVideos.pdf), physical pp18–80,88–132,135–184 | Separate sky 2.5D, near envelope/SDF and storm VFX paths. Different regimes justify different representations; vortex animation is not a five-family source model. |
| [Nubis³ 2023](https://advances.realtimerendering.com/s2023/Nubis%20Cubed%20%28Advances%202023%29.pdf), modeling/sampling/lighting/rendering sections and physical p198 | Prepared 3D morphology, detail up-rez, distance acceleration and light fields. Pseudomotion remains distinct from structural evolution. |
| [Schneider 2024 author talk/abstract/video](https://www.guerrilla-games.com/read/synthesizing-realistic-clouds-for-video-games), abstract pp1–2; video stills02:00/08:00/14:00/20:00/26:00/32:00 | Observed modeling/detail/lighting/cost illustrations. Audio and continuous motion were not inspected; do not cite this as a watched lifecycle demonstration. |
| [Smolder, SIGGRAPH 2026 notes](https://www.realtimerendering.com/advances/s2026/content/AlexanderMueller_Smolder_Siggraph26.pdf), physical pp14,22,51–64,80,89–100,101–104,111 | Offline volume animation, sparse occupancy/packing and transmittance-adaptive lighting-rate sharing; whole-image downsampling failed its TAA while per-pixel jittered shared lighting worked in dense smoke. Whole-animation memory and cloud-specific lighting remain limits. Our streaming/interpolation and composition are additions. |
| [Patry, 2021](https://advances.realtimerendering.com/s2021/jpatry_advances2021.pdf), slides10,83,87,94–101 | Shared sky/lighting product and amortized updates. A 60-frame sky refresh is not an adopted fast-weather latency. |
| [RDR2, 2019 course](https://advances.realtimerendering.com/s2019/index.htm) | Abstract only: shared atmospheric products for views, reflections and irradiance. Large PPT not substantively inspected; exact implementation/cost unknown. |
| [Procedural Cloudscapes, 2018](https://perso.liris.cnrs.fr/egalin/Articles/2018-cloudscapes.pdf), §§4–6 and supplement; [Stormscapes, 2020](https://computationalsciences.org/publications/haedrich-2020-stormscapes/haedrich-2020-stormscapes.pdf), §§4/6/7 | Controlled family morphology/transport and warm-cloud dynamics respectively. Use as authoring/reference evidence; neither establishes our library or browser budget. Ice needs its own source. |
| [SideFX cloud modeling](https://www.sidefx.com/docs/houdini/model/cloud.html), curve/polygon/VDB examples; [Pyro fields/export](https://www.sidefx.com/docs/houdini/nodes/sop/pyrosolver.html) | Concrete modeling/cook operations and time-varying field export. Smoke density needs a fixed optical remap; noise evolution alone does not meet lifecycle acceptance. |
| [JangaFX cloud tutorial](https://www.youtube.com/watch?v=JI2A6qjrS0s), official chapter description; [VDB export documentation](https://docs.jangafx.com/embergen/pages/getting_started.html); [free VDBs](https://jangafx.com/software/embergen/download/free-vdb-animations) | Tutorial located, transcript unavailable; not watched. Export workflow docs inspected. CC0 cloud pack is ten variations, not verified lifecycle sequences; no asset downloaded or selected. |
| [Advanced Real-Time Volume Graphics, 2026](https://dvr-course.github.io/), extended abstract and sample listing | Relevant traversal/multi-volume challengers. Full notes/samples not run; CUDA/OptiX paths are not a portable WebGPU foundation. |

Smolder PDF SHA256: `1db757c81568181f812a279d97f42fb2ce691b2018f5111e407ef1cb72316268`.
Root re-read the retained primary speaker notes and inspected the indirection-grid figure, avoiding another large
download. Its useful contribution is a production data/scheduling precedent; it does not certify our cloud lighting.
No third-party deck, video, source tree or asset library is vendored by this plan.

## Accessible reference code

Pinned revisions were checked on 2 November 2026. Study code for pass order, resource ownership and limitations;
reimplement the selected contracts rather than importing an engine wholesale. Not built or benchmarked here.

| Project / exact revision | Files/functions actually inspected; role and license |
| --- | --- |
| [Hillaire author app,183ead5bdacc701b3b626347a680a2f3cd3d4fbd](https://github.com/sebh/UnrealEngineSkyAtmosphere/tree/183ead5bdacc701b3b626347a680a2f3cd3d4fbd) | `Resources/RenderSkyRayMarching.hlsl`: `IntegrateScatteredLuminance`24, `NewMultiScattCS`418, `RenderTransmittanceLutPS`547, `SkyViewLutPS`581, `RenderCameraVolumePS`645. MIT comparison app, not UE runtime source; shader has Epic copyright and author README records release permission. Review applicable file notices before any copying. Root rechecked raw locators. |
| [Bruneton,34f14e745cff948f4ca3157d1b62a445ffa7286f](https://github.com/ebruneton/precomputed_atmospheric_scattering/tree/34f14e745cff948f4ca3157d1b62a445ffa7286f) | `atmosphere/functions.glsl`: `GetSkyRadiance`, `GetSkyRadianceToPoint`, `GetSunAndSkyIrradiance`; `model.cc` precompute/order/allocation. BSD3; independent mathematical/reference path. |
| [Bevy,ae6152152d6fb8308ad6038314b48ef295e80fbc](https://github.com/bevyengine/bevy/tree/ae6152152d6fb8308ad6038314b48ef295e80fbc) | `crates/bevy_pbr/src/atmosphere/{node.rs,resources.rs,aerial_view_lut.wesl,render_sky.wesl,environment.rs,environment.wesl}` and PBR light join. MIT/Apache2. Root confirmed RGB versus mean-alpha fallback in `render_sky.wesl`; do not assume dual-source blending. |
| [Takram,b012ad06d858fc035d88aacfd73f092f93c994e4](https://github.com/takram-design-engineering/three-geospatial/tree/b012ad06d858fc035d88aacfd73f092f93c994e4) | `packages/clouds/src/CloudsEffect.ts::update`454; `shaders/clouds.frag::marchClouds`472; `shadow.frag::marchClouds`37; `cloudsResolve.frag::temporalUpscale`63. MIT. Root checked resolve source. Current main's cloud path remains GLSL/WebGL; WebGPU rewrite is in progress, not a ready dependency. |
| [Muth author renderer](https://github.com/Qendolin/webigeo-clouds) (`cloud-rendering` branch) and [preprocessor](https://github.com/Qendolin/webigeo-clouds-server), discovered2November2026 | Public repositories identified from the university page; renderer declares GPL-3.0. Commit/raw-function retrieval was unavailable in this bounded check, so code algorithms are **not inspected/pinned/adopted**. The thesis supplies the traversal/LOD comparison; code import needs a separate source/license/portability check. |
| [Godot,95c0fa5fe8a019adc29d074c16b64469b04571f7](https://github.com/godotengine/godot/tree/95c0fa5fe8a019adc29d074c16b64469b04571f7) | `servers/rendering/renderer_rd/shaders/environment/volumetric_fog_process.glsl`: `MODE_DENSITY`297–813, `MODE_FOG`825–858. MIT. Useful injection/integration/reprojection; Forward+ cost/support constraints remain. |
| [O3DE,106fe7a0103f83dae3e579af214cced8b89d79a3](https://github.com/o3de/o3de/tree/106fe7a0103f83dae3e579af214cced8b89d79a3/Gems/SkyAtmosphere) | SkyAtmosphere pass/feature processor and `SkyAtmosphereCommon.azsli`. MIT/Apache2 with file exceptions. Dirty scheduling useful; inspected shader is single-scatter, not complete Hillaire multiple scattering. |

Takram documents sparse ghosting/disocclusion and depth-overlap aerial artifacts. These are regression cases, not
features to copy. Its WebGPU status was checked against the current official README, not a third-party package fork.
Unity HDRP public documentation is comparative material; its Companion-licensed source was excluded from the
competitive engine study. Public visibility is not automatically an import license.

## Recent alternatives and decision watchlist

| Primary 2025/2026 work | Actual access and disposition |
| --- | --- |
| [Environmental Volumetric Neural Shading, HPG2026](https://fileadmin.cs.lth.se/graphics/research/papers/2026/environmental_volumetric_neural_shading_of_clouds_for_real_time_rendering/); [source ecf6f94fa59c59f8532af1ef100455fa32f396a1](https://github.com/LUGGPublic/evns-clouds-rtr/tree/ecf6f94fa59c59f8532af1ef100455fa32f396a1) | Author abstract and training/model/runtime shaders inspected; paper PDF unavailable to reader. Fixed prepared mesh-asset shading, NVIDIA cooperative-vector shader dependency, no repository license file located. Research challenger for bounded assets, not weather/lifecycle foundation or admitted browser dependency. |
| [Dynamic volumetric cloud modeling with edge details refinement,2025](https://doi.org/10.1016/j.cag.2025.104456) | Publisher preview only; full text paywalled, code not located. Coordinate-driven detail flow supports investigating edge motion; no demonstrated replacement for our lifecycle/weather owner. |
| [DDCL accepted manuscript,2025](https://www.thomasbashfordrogers.com/Papers/DDCL.pdf) | First review used indexed excerpts; second reviewer inspected accepted manuscript §§3.1–3.3/4.1/4.4/4.6. Image-space sky illumination, reported3s/frame and separately added unattenuated obscured Sun exclude our runtime foundation. Image-space interpolation failures do not prove optical-coefficient interpolation fails. |
| [Image Based Whole Sky Cloud Volume Generation,2026](https://doi.org/10.1109/TVCG.2025.3641982) | Abstract only; potential offline captured asset reconstruction, not verified dynamic source or runtime path. |
| [Muth, WebGPU meteorological rendering thesis, March2026](https://www.cg.tuwien.ac.at/research/publications/2026/muth-2026-clouds/muth-2026-clouds-thesis.pdf), §§3.3–3.4/4.2–4.3 | Substantive second-review PDF study: tiled compressed hierarchy, conservative occupancy/coarse-fine traversal and anisotropic LOD; stated1.14GiB atlas and weak small-Cu/fog detail exclude direct adoption. RTX5070Ti/native Dawn timings do not qualify M2/browser; exact pass scope is recorded in second review. Compare traversal/LOD only with same source and thin-gap tests. |
| [WebGPU September2026 draft](https://www.w3.org/TR/2026/CRD-webgpu-20260915/) | Feature/format tables inspected. BC/ASTC sliced-3D support is optional; query actual adapter features. Portable uncompressed/packed format remains required before a compressed specialist path is admitted. |

Recheck this watchlist at representation selection, before major implementation, and before target admission. Record
new source version/date, inspected scope, affected decision and whether it changes the plan. No monitoring automation
or recurring task has been installed. Prefer a complete proven fit over a newer title.

## Deductions and unresolved qualification

Our combination of live reduced optical-control evolution, footprint-aware source/light products, ordered overlap and
independent histories is a synthesis. The disjoint transfer algebra is established; RGB aerial extension, dynamic source/genus recipes and current voxel-light error/admission
are our explicit adaptations. Specialized far summaries are optional. Their quality/cost must be demonstrated by the migration POCs. Publications leave some authoring inputs and runtime
choices private. We do not present deductions as knowledge of their internals.

The largest unproved mechanism is the native live3D source producing convincing morphology/renewal/breakup under
new runtime forcing at viable cost. Published advection or a named voxel renderer does not prove those cues. P1/P4a
therefore exercise actual source creation and coupled images/motion before broad variants or engine integration.


## Second-review consequences

The four fresh reviews evaluate applicability to2026 rather than treating newest publication date as a selection rule.
No inspected successor establishes a better complete five-family lifecycle/browser system. The earlier second review retained provisional source/conditional light comparisons. Those selection instructions
are now superseded by the locked production baseline. Actual P1 renewal/bytes proof remains required. Smolder89–100
selective-light sharing and specialist WebGPU occupancy/LOD remain optional optimization references; neither author's memory, hardware timing,
async-compute benefit or TAA setup transfers automatically. The new POC/performance contracts make the combined
quality/cost decision before expanding content. Research support is distinct from source, appearance or budget passes.


The Muth timing is not used as a budget premise: its university abstract calls the reported figure cloud-pass cost,
while the second reader describes the thesis benchmark as native-Dawn whole-frame scope. That pass-scope difference
was not independently resolved in this bounded review; no numerical performance transfer is admitted. The comparative
value is its documented tile/occupancy/LOD design and explicit memory/detail limitations, not that timing number.
