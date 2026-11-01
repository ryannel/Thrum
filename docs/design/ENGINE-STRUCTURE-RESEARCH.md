# Modern engine structure — research and design implications

Research date: 31 October 2026. Scope: public architecture documentation, current release material and a substantive
production-tools presentation. This is a comparative architecture study, not an engine benchmark, implementation audit
or selection of a host engine. The [architecture brief](ARCHITECTURE-BRIEF.md) owns our proposed design.

## Selection and evidence limits

Use different reference engines for different questions. Unreal and Decima inform integrated production and authoring;
Unity informs configurable pipelines and runtime/authoring separation; Bevy and O3DE inform composition; Godot informs
the separation of approachable scene APIs from lower-level services; Wicked and PlayCanvas inform accessible graphics
research and distribution. This is a representative comparison, not an objective ranking of the most advanced engines.
Proprietary engine internals are known only where their authors publish them.

The following records distinguish inspected material from inference. Release claims are vendor/project claims, not our
measurements. Roadmaps and experimental features do not establish production readiness. No conference recording was
watched and no engine was built or timed during this study.

## Evidence and consequences

| Source and material inspected | Published structure or direction | Consequence for our proposal |
| --- | --- | --- |
| Epic, [Game Features and Modular Gameplay](https://dev.epicgames.com/documentation/en-us/unreal-engine/game-features-and-modular-gameplay-in-unreal-engine), overview and actions | Standalone feature plugins can activate/deactivate components and data; interactions with host actors are kept narrow | Treat a feature as a package with dependencies and lifecycle, rather than scattered flags |
| Epic, [Render Dependency Graph](https://dev.epicgames.com/documentation/en-us/unreal-engine/render-dependency-graph-in-unreal-engine), introduction and listed features | Explicit resource dependencies support transient allocation, synchronization, unused-pass culling and inspection | Shared GPU scheduling and resource ownership belong below individual rendering features |
| Epic, [World Partition](https://dev.epicgames.com/documentation/en-us/unreal-engine/world-partition-in-unreal-engine), overview, streaming sources and actor references | Spatial streaming works with data layers, HLOD and authoring partitions; references can force actors to load together | Streaming must account for content relationships and consumers, not only distance or visible meshes |
| Epic, [UE 5.8 release](https://www.unrealengine.com/news/unreal-engine-5-8-is-now-available), worldbuilding, animation, rendering and creative iteration sections; 23 June 2026 | Production-ready MegaLights and Dataflow coexist with experimental Mesh Terrain, Toon Shader and MCP tooling; Lumen has a cheaper lighting mode | Compare quality, authoring, scale and maturity together. Keep advanced methods optional until their full configuration qualifies |
| Unity, [URP render graph introduction](https://docs.unity.com/en-us/engine/6000.0/manual/render-pipelines/universal-render-pipeline/customizing-urp/render-graph/introduction), resource and optimization sections | Passes declare resources; the graph removes unused work and manages temporary lifetimes. Imported persistent resources remain externally owned | Distinguish transient frame work from persistent state; hiding a pass does not automatically retire all caches |
| Unity, [Entities concepts](https://docs.unity3d.com/Packages/com.unity.entities@1.4/manual/concepts-intro.html), introduction; page identifies Entities 1.4.8 | Entity identity, component data and processing systems are distinct | Data-oriented runtime organization is a candidate, without making every subsystem use one storage scheme |
| Unity, [Unity 7 roadmap](https://investors.unity.com/news/news-details/2026/Unity-7-Roadmap-Revealed-At-Unite-Seoul/default.aspx), 20 July 2026 announcement | Planned CoreCLR work, faster iteration, public CLI/API and Surface Cache GI; an announced continuation of Unity 6 architecture | Tool access and iteration are architectural concerns. Treat the announced improvements as roadmap direction, not delivered measurements |
| Bevy, [0.19 release](https://bevy.org/news/bevy-0-19/), Next Generation Scenes and Render Graph as Systems; 19 June 2026 | Composable scenes and render passes expressed as ECS systems in render-world schedules replace the older custom graph API | Preserve explicit dependency semantics without assuming a separate bespoke graph API is always necessary |
| Bevy, [DefaultPlugins](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html), plugin inventory and Cargo-feature behaviour; inspected page identifies 0.20.1 | Runtime, rendering, camera, lighting, animation, audio, UI, assets and diagnostics are distinct plugins; compilation features control inclusion | Useful precedent for configurable bundles and separation of runtime and presentation |
| Bevy, [0.20 release](https://bevy.org/news/bevy-0-20/), Solari, mesh shaders and forthcoming tooling; 8 October 2026 | Path-traced lighting, retained scene work and mesh-shader support are progressing; Solari documents missing light types and no built-in macOS denoiser | A promising advanced renderer does not yet guarantee all lights, platforms or reconstruction paths. Evaluate complete profiles |
| O3DE, [Atom overview](https://docs.o3de.org/docs/atom-guide/what-is-atom/), Rendering pipeline | RHI, render-pipeline interface, feature processors and passes have distinct roles; only needed graphics features can be activated | Separate backend execution, feature data and pipeline composition; modules need not each become a separate GPU pass |
| O3DE, [Core Gems](https://docs.o3de.org/docs/user-guide/gems/core-gems/), introduction | Many engine functions are extensions; runtime, editor and asset-processor products use different Gems | Package authoring support with a capability while excluding editor code from runtime deployment |
| Godot, [architecture overview](https://docs.godotengine.org/en/stable/engine_details/architecture/godot_architecture_diagram.html), complete short overview; [servers](https://docs.godotengine.org/en/4.6/tutorials/performance/using_servers.html), Servers and RIDs | Scene APIs sit above rendering, physics and audio services; scene usage can be bypassed, though not compiled out via that mechanism | Friendly authoring representation and efficient runtime service access can coexist; bypassing and removing are different promises |
| Guerrilla, [Nodes and Native Code](https://www.guerrilla-games.com/read/Nodes-and-Native), 14 July 2025; [PDF](https://d3d3g8mu99pzk9.cloudfront.net/BryanKeiren/Bryan_Keiren_Nodes-and-Native.pdf), extracted speaker notes, PDF pages 24–29, 42, 48–58, 73, 78–81 | Shared graph tooling separates semantic analysis from code generation, supports reloadable native output and runtime debugging. Host systems declare scheduling dependencies. Reflection also supports serialization and editors | Reuse schemas, inspection and authoring infrastructure across domains while retaining domain-specific execution. A node editor alone is not a complete workflow |
| Wicked Engine, [official repository](https://github.com/turanszkij/WickedEngine) and [current feature inventory](https://raw.githubusercontent.com/turanszkij/WickedEngine/master/features.txt) | A public framework/editor exposes data-oriented scenes, jobs, multiple graphics backends and a broad rendering/animation/physics feature set | Useful implementation/reference candidate for individual techniques. Feature presence is not comparative quality or performance evidence |
| PlayCanvas, [engine overview](https://playcanvas.com/products/engine), backends, modules, rendering and distribution | WebGL2/WebGPU, importable modules, compressed assets and deferred loading make browser delivery a first-class concern | Public website delivery has distinct capability, startup and payload budgets; do not assume a desktop showcase will run unchanged |

The Guerrilla HTN 2026 publication listing was discovered, but direct page retrieval timed out. It is not used to
justify a particular planner algorithm here. Old unofficial Wicked documentation returned by search was excluded in
favour of the current official repository. Detailed algorithms in these engines remain for module-specific research.

## Synthesis — our interpretation

1. **Composition is a lifecycle and ownership problem.** Plugins alone do not define valid combinations, consumer
   fallbacks or what work actually disappears. Our profiles should select capabilities, implementations and budgets
   independently, then validate their compatibility.
2. **Data, work and presentation have different boundaries.** Authored scenes, simulation state and render-ready data
   need shared identity and explicit synchronization, not necessarily identical layouts. Rendering should support
   multiple views without making the primary camera the owner of world evolution.
3. **Dependencies enable optimization.** CPU tasks, GPU passes and asset preparation each benefit from declared inputs
   and outputs. They are different schedules with different lifetimes; do not collapse them into one universal graph.
4. **Whole-scene scalability extends beyond polygons.** Animation, AI, streaming, light sampling and history resources
   all need budgets. Rendering detail and simulation relevance can change independently with explicit transitions.
5. **Creation and inspection are part of the engine.** Shared schemas, reproducible import/build steps, runtime
   inspection and automation interfaces support human and programmatic workflows. A full visual scripting compiler
   is a possible later investment, not a prerequisite for useful tools.
6. **Advanced graphics needs capability-specific providers.** GPU visibility, virtualized resources, ray/path tracing
   and learned reconstruction are candidates in different profiles. Device support, motion quality, content coverage
   and actual budgets decide selection; a fashionable feature name does not.

## Potential contributions to investigate

These are engineering/research hypotheses, not claims of novelty:

- Capability profiles that explain dependencies and demonstrate the measured work saved when a feature is removed.
- Shared environmental providers whose effects remain consistent across omitted, simplified and fully visible forms.
- Quality allocation that respects view, interaction relevance, temporal stability and whole-scene budgets together.
- Small, independently usable modules delivered with authoring adapters, reproducible scenes and meaningful comparisons.

Before claiming an advance, search the specific prior art, identify the gap, define a competing baseline and measure
the complete result. Public presentation and upstream suitability should be proved through portable integration and
clear provenance rather than a large feature inventory.
