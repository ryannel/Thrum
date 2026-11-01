# Reusable engine — architecture brief

Status: commissioned whole-engine research and design programme, 31 October 2026. Custom host selected;
C4 reconciles the initial design on 1 November; implementation and numerical/device qualification remain unfinished.
This is the canonical brief for the new whole-engine design discussion. Product decisions remain in
[DESIGN](../provenance/CONTEXT.md#source-779e25e8668e); this brief develops their architectural consequences.

## Purpose and confirmed requirements

Build a flexible engine suitable for a variety of games. Features can be selected to fit each game. The engine should
reach a quality worth demonstrating on the user's personal website and potentially contributing to open-source game
engines. It should use modern, cutting-edge work where useful and leave room for original advances supported by evidence.

| ID | Confirmed requirement | Architectural consequence to investigate |
| --- | --- | --- |
| E01 | Serve a variety of games rather than only the current game | Derive capabilities from contrasting usage profiles; separate reusable services from game policy and authored content |
| E02 | Turn features on or off to fit the game | Explicit dependencies, supported combinations, resource ownership and configuration lifecycle |
| E03 | A top-down game may need beautiful light and cloud shadows without visible clouds or sunsets | Separate visible sky presentation from lighting/occlusion services; retain useful providers without requiring the visible renderer |
| E04 | Quality suitable for public demos and possible open-source engine contributions | Demonstrable integrated quality, reproducible workloads, maintainable interfaces and contribution-ready provenance |
| E05 | Modern methods and possible forward advances | Research established and emerging alternatives; distinguish proven production paths from experimental hypotheses |
| E06 | Start with the whole engine, independently of existing water implementation | Select responsibilities and contracts before reconciling old code; water is one consumer and contributor among others |
| E07 | Focus on 3D across top-down, isometric, third-person and first-person cameras and different game styles | One world can support different view/control profiles; perspective and orthographic requirements need explicit support |
| E08 | Pure 2D games and massive multiplayer are outside current scope | Retain UI/text for 3D games; future ordinary multiplayer is first-class under E13, without MMO scope |
| E09 | Target modern desktops including Macs, scalable quality, and a browser profile for demos | Native and browser delivery share world/content semantics but qualify different GPU providers, budgets and cooked assets |
| E10 | Build our own engine; abandon Bevy as the host, learn from its open-source design | Own composition/runtime integration; retain source provenance for any adapted code. Reuse other suitable libraries deliberately rather than rebuilding every commodity |
| E11 | Include mobile app stores in the initial platform ambition | Design native mobile lifecycle, touch, thermal/memory limits and packaging from the start; iOS and Android are the working interpretation |
| E12 | Build for AI development, with deliberate architecture and explicit maturity tracking | Machine-readable contracts/status, inspectable and replayable operations, bounded agent commissions, reproducible evidence and human visual acceptance |
| E13 | Future multiplayer must be a first-class citizen to avoid rebuilding later | Design actor intents, world authority, scoped IDs, replication, content compatibility and correction now; optional runtime networking and local-first construction |
| E14 | Derive complete designs from research, mathematics and underlying systems before building | Investigate academic/production sources; derive operators and coupling, use later measurements to test stated predictions rather than discover architecture through feature layering |
| E15 | Coordinate recursive agent research actively so modules do not duplicate or contradict one another | One contract owner, bounded decisions/commissions, shared context and budgets, cross-review and sequential integration; detailed decomposition stops when the commissioned decision is resolved |
| E16 | Simple first-use and expert workflows across game styles, imported assets and custom modules | Empty-project and starter paths, ordinary typed code/files, same human/AI tools, meaningful edit/play/export and art/reimport acceptance |
| E17 | Enable ambitious AI optimization across modules and platforms | Concrete outcome targets, trusted independent evaluation, broad candidate freedom, whole-engine regression/quality constraints, integrated promotion and retained evidence |
| E18 | Prevent scattered assets and runaway disk usage | Separate disposable work from selected source/runtime assets; explicit promotion, exact provenance, bounded build storage, recoverable masters and owner-scoped cleanup |
| E19 | Keep the engine elegant and understand its real maturity | Small public model, one owner per mechanism, deterministic convention checks, separate selected/design/implementation/quality/platform evidence and no speculative universal frameworks |
| E20 | Allow different water, atmosphere and other feature implementations where their use warrants it | Capability/provider separation, explicit validity and compatibility, shared causes/state ownership, mathematically defined heterogeneous boundaries; no automatic physical seam blending |

E03 is a user-provided example of composability, not a selected cloud algorithm or mandatory first demonstration.
Desktop, browser and mobile-store profiles are selected. [BASELINE-SPEC](BASELINE-SPEC.md) now chooses reference
devices, environments, workloads and numerical targets; certified minimum support remains unqualified. Bevy is excluded
as host. Rust/wgpu and direct library versions are selected; full dependency locks and target proof accompany implementation. Possible
contributions do not select a license, repository or publication action.

## Conversation coverage and retained scope

This maps the substantive requests through the simplification reconciliation. A linked design is coverage, not
working software. The smaller first SDK changes sequencing and admitted combinations; it does not reduce the intended
engine to one game, one camera, one art style, a water demo or a browser-only product.

| Requests retained | Governing record and acceptance boundary |
| --- | --- |
| E01/E06/E07: reusable 3D, whole engine first, contrasting cameras and games | This brief's responsibility map; [system design](SYSTEM-DESIGN.md); [scenario walkthroughs](SCENARIO-WALKTHROUGHS.md) |
| E02/E03/E20: modular features, bring a provider, no visible sky with retained light/shadows, multiple physical models | [Composition](COMPOSITION.md), C04/C08/C09, [environment](modules/ENVIRONMENT.md); static selection and qualified replacements, no arbitrary universal renderer/live-swap promise |
| E04/E05/E10/E14: modern research, visual ambition, selective open-source ideas/reuse, no Bevy host | [Engine research](ENGINE-STRUCTURE-RESEARCH.md), [capability survey](CAPABILITY-SURVEY.md), [decisions](ARCHITECTURE-DECISIONS.md), [programme](DESIGN-PROGRAMME.md); primary papers/production sources and math before construction, explicit novelty/provenance limits |
| E08/E13: ordinary multiplayer first-class, no initial MMO or pure-2D engine | C13 and [NETWORK-SPEC](NETWORK-SPEC.md); same local/server rules, real early I2n, semantic-compatible heterogeneous clients, explicit result/reconnect/replica behavior |
| E09/E11: Mac/PC, scalable browser demos, native mobile stores | [Baseline](BASELINE-SPEC.md), [runtime](modules/PLATFORM-RUNTIME.md), [construction](CONSTRUCTION-PLAN.md); native authoring first is independent of player-platform support, device/store evidence still required |
| E12/E15/E19: AI engine and game contribution, clear skills, deterministic checks, active coordination | [AI development](AI-DEVELOPMENT.md), [skill rationale](SKILL-WORKFLOW.md), [programme](DESIGN-PROGRAMME.md), [maturity](MATURITY.md); focused Astra-aligned guidance, scoped checks, public commands and reviewed combined revisions |
| E16/E19: teachable core, newcomer/veteran, from scratch/custom code/art styles and artists | [Creator guide](ENGINE-EXPERIENCE.md), [first use](FIRST-USE-WALKTHROUGHS.md), [empty-project journeys](FROM-SCRATCH-WALKTHROUGHS.md); project/world/target/run, ordinary files, source admission, restart, import/reimport and recipient export |
| E17: ambitious optimization that improves the engine, not only a benchmark | [AI performance](AI-PERFORMANCE.md), [campaign walkthroughs](PERFORMANCE-WALKTHROUGHS.md); independent evaluator, simultaneous consumer/resource/quality budgets, fair baseline and integrated evidence |
| E18: generated asset iteration/promotion, campaign cleanup, bounded conversations/evidence | [Storage lifecycle](STORAGE-LIFECYCLE.md), [lifecycle journeys](LIFECYCLE-WALKTHROUGHS.md), [asset contract](../ASSET-LIFECYCLE.md); retain selected masters/recipes and decisive evidence, never equate ignored local files with backup or delete chats/other work implicitly |
| E14/E19: finish cohesive design before building, challenge contradictions and reduce complexity | [Simplification dispositions](SIMPLIFICATION-REVIEW.md), C4 and the five initial specifications; initial scope is designed, broader feature claims need their own detailed recipe and runtime proof |

No new product requirement is inferred from an engine's highlight page. Optional advanced algorithms, runtime LLMs,
browser authoring, universal hot reload, competitive-shooter rollback and seamless server-world expansion are not
first-SDK promises. Their absence does not remove the shared boundaries required to add a justified provider later.

## Selected architectural direction

The [decision record](ARCHITECTURE-DECISIONS.md) selects the structural baseline; precise APIs and qualified implementations remain separate:

- Capabilities declare what they provide and consume. A game profile selects compatible providers and quality settings.
- Explicit game setup and entity components make composition available to game developers, with optional packages
  where reuse warrants them. [Scenario walkthroughs](SCENARIO-WALKTHROUGHS.md) recommend this simpler baseline;
  [composition](COMPOSITION.md) specifies extension/compatibility rules and the limits of supported live changes.
- Presentation can be absent while a cheaper or independent provider supplies effects required by retained consumers.
- Off should remove avoidable scheduling, allocations and preparation; shared services remain only for active consumers.
- Build-time, load-time and live selection may have different guarantees. We need not promise arbitrary live toggles.
- Whole-engine quality includes controls, simulation, animation, world life, audio, authoring, persistence and delivery,
  as well as rendering. Their inclusion and fidelity follow the supported game range.
- Experimental techniques can compete behind stable contracts without forcing all games to inherit their risk or cost.

## Performance ambition and development economics

The user selects shocking performance as an engine-wide ambition: invest deeply in expensive operations and small
costs repeated frequently or at scale, including the development loop. E05/E12/E17/E19 therefore favor efficient
execution, a clear public model and verifiable internals. AI can absorb substantial internal implementation and
maintenance complexity when the benefit is justified; genericity or ease of hand-editing is not the default winner.
Application stability, correctness and required quality remain integral to that ambition.

[The performance investment policy](AI-PERFORMANCE.md#performance-investment-from-the-first-design) owns the capability
map, module cost accounts and promotion rules. Performance design starts before representations are entrenched and
continues through the construction sequence. I5 is later capability expansion, not the beginning of serious optimization.
This selects a development strategy, not a measured advantage, specific algorithm or new runtime safety exception.

## Current scope and next stage

The broad 3D camera/platform range and future multiplayer requirement are selected. [The baseline register](BASELINE-SPEC.md)
now records G01–G10 decisions for source/recovery, world/transforms, shaders/light/media, play/water, networking and
reference targets. [The construction plan](CONSTRUCTION-PLAN.md) includes the I2n multiplayer vertical path alongside
I3a's small rendered game. Larger worlds, richer fluids/media and additional device families retain their explicit
extension/qualification obligations; no architecture decision or numerical target is a running support claim.

## Research basis

The [modern-engine study](ENGINE-STRUCTURE-RESEARCH.md) compares public Unreal, Unity, Bevy, O3DE, Godot, Decima,
Wicked and PlayCanvas material. It records exact sources, inspected sections and maturity limits. The following
structure is our synthesis, not a claim that any reference engine uses this precise decomposition.

The subsequent [twelve-engine capability survey](CAPABILITY-SURVEY.md) compares official highlight pages with the
detailed owners. Its H01–H12 dispositions make creation/delivery workflows explicit, extend localization production
and dynamic-content admission, and record optional haptics. They refine the recommendation without adding kernel
modules or implying implementation. Broad labels below must be read with those concrete outcomes and target limits.

The [host and platform assessment](HOST-AND-PLATFORMS.md) records the earlier alternatives and actual integration.
The user's subsequent decision selects a custom host. The [design programme](DESIGN-PROGRAMME.md) coordinates
recursive research, integration and qualification without inheriting the current implementation as the target.

## Desktop and browser consequences

- Establish a high-quality portable raster/compute baseline for native desktop, including Macs. Qualify advanced
  ray-tracing, geometry and reconstruction providers by actual device/API support; they are optional enhancements.
- Select Apple silicon and the M2 reference candidate initially; Intel Mac coverage remains an additional qualification profile.
- Treat browser demos as a separately qualified WebGPU product profile with their own startup, memory, download and
  scene budgets. WebGL2 fallback is an additional scope decision, not an automatic promise.
- Native mobile profiles must account for suspend/resume, interrupted audio/input, thermal pressure, memory limits,
  touch/UI, store packaging and mobile GPU bandwidth. They share content sources and contracts, not identical budgets.
- Keep authoritative game state and required interactions consistent across quality settings. Simplify appearance and
  presentation work independently; any gameplay-affecting simulation reduction needs an explicit game contract.
- Cook content for the target, load asynchronously, and specify shader preparation and asset residency. Do not make
  runtime features depend on local filesystem access or native threading being available in the browser.
- Own one whole-scene budget, including animation, AI, physics, streaming and temporal resources. Quality changes need
  explicit resource retirement and history invalidation. Merely lowering render resolution is not a scaling design.
- Distinguish host-engine restrictions from graphics-library restrictions and hardware/browser restrictions. Dropping
  Bevy while retaining wgpu does not remove wgpu's API limits; native API work is a separate cost and decision.

## Proposed module map

These are responsibility boundaries, not a demand for a new crate, plugin, process or GPU pass for every row. Several
can be supplied by a host engine or library. The custom work should concentrate on missing capabilities and valuable
advances. Use direct explicit contracts where possible; no universal service container or event bus is selected.

### Foundation and world data

| ID / module | Owns and provides | Boundary |
| --- | --- | --- |
| F1 Runtime and platform | Application lifecycle, platform I/O, jobs, simulation/presentation time, allocation and capability discovery | No gameplay or environment policy; GPU backend lives behind rendering execution |
| F2 Feature composition | Validated profiles, dependencies, provider selection, startup/shutdown and configuration changes | Resolve supported combinations; do not scatter ownership across booleans |
| F3 Identity and schemas | Stable object/asset identifiers, units, data schemas, reflection and versioning conventions | Common meaning, not a single universal storage layout |
| F4 Asset pipeline and residency | Import, preparation/cooking, dependency hashes, compression, loading, cache and residency | Editable sources and runtime payloads have distinct lifetimes; renderer consumes handles |
| W1 Scene and world | Entity/component state, transforms, hierarchy, scene instances, spatial membership and lifecycle | Canonical identity and object lifetime; specialised systems own their state and derived indexes |
| W2 World construction and streaming | Regions, additive content, procedural recipes, terrain construction and streaming relevance | Authoring placement and runtime activation are distinct; render, physics and navigation consume geometry products |

### Play, simulation and persistence

| ID / module | Owns and provides | Boundary |
| --- | --- | --- |
| P1 Input and actions | Devices, action maps, rebinding, focus, input recording and accessibility settings; optional feedback cues | Produces intent without assuming a player controller or camera style; selected haptics use F1 device output |
| P2 Cameras and views | Orthographic/perspective projection, rigs, blending, obstruction handling, picking and view descriptions | Shares the world across gameplay, editor, reflections and captures; does not advance world simulation |
| P3 Physics and scene queries | Collision, rigid bodies, constraints, triggers, query service and optional specialised solvers | One authority for a body's solved motion; gameplay and animation submit requests/constraints |
| P4 Animation and characters | Clips, blending, skeletons, morphs, retargeting, IK, pose evaluation and locomotion integration, with authoring preview/diagnosis | Pose and root-motion contracts join physics; advanced matching, cloth and facial systems are optional providers |
| P5 Navigation and agents | Paths, traversal, perception and decision-execution interfaces | World facts and navigation are reusable; behaviours, species and tactics are game content/policy |
| P6 Behaviour and interaction | Reusable state/action execution, interaction queries and sequencing; optional script/graph integration | Combat, quests, inventory rules and economy are game packages; no baseline runtime VM or visual scripting promise |
| N1 Network sessions and replication | Optional server/replica sessions, typed actor intent, coherent state projection, bounded transport and correction | Server/standalone commits gameplay; client visuals may differ; prediction requires complete state/dependency capture; [selected design](modules/NETWORKING.md) |
| P7 Persistence and replay | Save/load, schema migration, persistent world deltas and diagnostic input/snapshot playback | Modules expose durable state; transient rendering caches are rebuilt. Cross-platform deterministic replay is a separate requirement |

### Presentation and environment

| ID / module | Owns and provides | Boundary |
| --- | --- | --- |
| V1 Graphics execution | GPU resources, pipeline/shader preparation, pass dependencies, synchronization and device recovery | Backend capability is explicit; choose host facilities before designing another graphics abstraction |
| V2 Render scene and visibility | Render-ready instances, deformation inputs, culling, LOD, batching and per-view extraction | Reads committed state; can retain representations optimized differently from scene/physics data |
| V3 Materials and illumination | Material models, lights, shadows, indirect light, reflections and their provider contracts | Internally separate material description, lighting and visibility providers; sky imagery is not a prerequisite for light |
| V4 Image reconstruction and display | Antialiasing, temporal histories, optional denoising/upscaling, exposure, colour management and output | Histories belong to their view and algorithm; profile switches explicitly preserve or invalidate them |
| E1 Environmental modules | Independently selectable terrain surfaces, vegetation, weather/wind, atmosphere/sky/clouds, water and surface response | Shared environmental inputs with separate producers; decompose further below rather than creating one monolith |
| V5 Effects | Particles, decals, trails, impacts and authored or simulated local effects | Consumes typed causes from gameplay/environment; does not invent a competing source of physical truth |
| A1 Audio | Spatial emitters/listeners, buses, mixing, ambience/music control and optional acoustics | Shares source identity/timing; sound logic does not depend on visible particle count or camera frustum |
| U1 UI and text | Layout, shaping/fonts, focus/picking, IME, semantic accessibility and localized content with production tooling | Native/browser bridges and selected locales require qualification; needed for 3D games despite pure 2D exclusion |

### Creation and delivery

| ID / module | Owns and provides | Boundary |
| --- | --- | --- |
| T1 Authoring and automation | Inspector, scene/prefab editing, undo/redo, content/animation/style preview, declared edit/restart support, extensible controls and CLI/API | Shared schemas and operations support human and programmatic use; editor-only dependencies stay out of the game |
| T2 Diagnostics and delivery | CPU/GPU/memory attribution, captures, debug views, regression scenes, source-stored build configurations, package inclusion/size diagnosis and export provenance | Separate authoring/build/player support and startup/offline/residency promises; qualify profiles and reproducible demos |

### Environmental decomposition

- **Weather and wind:** authored or simulated environmental fields and transitions. Consumers may use constant inputs
  when dynamic weather is absent. Seasonal/climate simulation is not automatically required.
- **Atmosphere and sky:** aerial transport, environmental illumination and visible sky are distinct capabilities.
- **Clouds:** cloud representation, sunlight attenuation/shadow queries and visible cloud rendering are separable.
  A lightweight authored provider can supply attenuation without volumetric cloud geometry.
- **Terrain and surfaces:** surface identity, material regions and optional wetness/snow/deformation responses. W2
  handles construction and activation; this module handles changing surface properties and their consumer contracts.
- **Vegetation:** placement/asset support, growth only if required, deformation and interaction. Wind and lighting
  come from shared providers; vegetation does not own a second weather model.
- **Water:** water regions, selected dynamics, contact/force queries and appearance adapters. Body integration remains
  with P3; light transport uses V3/V4. Sea, lake, river and specialised fluids need not share one numerical solver.

This decomposition identifies responsibilities only. Deep module research must decide mechanisms, budgets and which
submodules genuinely deserve independent packaging. Caustics, for example, cross water geometry, illumination and
receiver visibility; naming a caustics box does not solve that contract.

## Interactions that determine the boundaries

| Interaction | Ownership and handoff to design |
| --- | --- |
| Player or AI moves an animated body | Input/agent produces intent → movement policy requests displacement → physics resolves motion → animation builds the pose; root motion and ragdoll overrides have explicit authority |
| A client joins and corrects predicted play | N1 validates semantic content/protocol and actor intents → authority commits → replica admits coherent baseline → optional bounded replay → affected visual/audio history reconciles; client loading never pauses server |
| The same world is seen from two cameras | World step commits once → per-view extraction and visibility → lighting/rendering → view-specific histories/display. Offscreen casters, reflections and audible events may still matter |
| Weather changes a landscape | One time-indexed environment source → wind/precipitation → selected vegetation, water, surface-response and audio consumers; lighting uses the same selected sun/attenuation facts |
| A region streams in or out | W2 requests activation → F4 satisfies asset dependencies → physics/navigation/render/audio representations attach → P7 retains required durable state on retirement |
| A material or asset changes live | T1 records an editable operation → F4 rebuilds affected products → consumers receive versioned replacements → stale derived resources/histories are invalidated |
| A profile disables a feature | F2 checks retained consumers and substitutes an allowed provider → affected work/resources retire at safe boundaries → persistence and history rules apply; unresolved dependencies reject the configuration |

The spatial representations for rendering, physics and navigation may differ. Stable IDs, transforms, units, timestamps
and explicit tolerances join them; forcing every consumer into one geometry structure is not the goal.

## Capability profiles — proposal

A profile has three independent dimensions: **what exists**, **which provider supplies it**, and **quality/work budget**.
Camera choice influences that policy but is not itself a quality tier. Top-down water may still need sky reflections;
removing the sky draw does not necessarily remove environmental lighting or every sky evaluation.

| Illustrative profile | Retained capabilities | Optional or substituted work |
| --- | --- | --- |
| Top-down/isometric | Excellent materials/light/shadows, overhead views, picking, appropriate animation and scene detail | Cloud-shadow field without visible clouds; selected atmosphere contribution; simplified distant content |
| Third-person exterior | Character motion, camera obstruction, near detail, terrain/vegetation, spatial audio and visible horizon | Selected dynamic sky/weather and quality providers according to budget |
| First-person interior | Close materials, animation/hands as needed, dense local lighting, acoustics and portals/visibility | Outdoor environment absent or supplying only window illumination/reflections |
| Technical showcase | Reproducible camera route, comparison controls, measurements and selected advanced capabilities | Experimental providers explicitly identified; showcase settings do not become defaults for every game |

These profiles are proof workloads, not four forked engines. The default game package may omit unwanted modules at
build time; load-time selection chooses providers; only explicitly supported settings are promised as live toggles.

## Design and construction order

The [integrated system design](SYSTEM-DESIGN.md), [shared contracts](CONTRACTS.md) and
[construction plan](CONSTRUCTION-PLAN.md) now develop this responsibility map into a cohesive recommendation.

1. Establish supported profiles and representative scene/device constraints; provisional numerical envelopes live
   only in the runtime design until target hardware and workload are selected.
2. Derive the complete operating model from mathematical/domain knowledge, primary evidence and constraints:
   representations, actual producers/operators, approximations, ownership, timing, lifetime and cost/error behaviour.
3. Research coupled module families together, with whole-system context. Reconcile their contracts continuously and
   trace complete normal/adverse scenarios before accepting their design. Do not use experiments to fill missing models.
4. Complete detailed mechanism/API choices required by each construction package. Reconcile existing code and reusable
   libraries against that target; do not preserve the rejected host through hidden dependencies.
5. Build integrated increments governed by the design. Subsequent measurements check explicit predictions and empirical
   limits. Failed predictions revise the model/implementation; they do not justify unrelated feature layering.

Potential distinctive work includes environmental providers that remain coherent across profiles, measurable feature
removal, and quality allocation across views and systems. These are research questions, not claims of original invention.

## Method and status

Use [engine architecture](../../.agents/skills/engine-architecture/SKILL.md) to maintain this brief and commission module
designs; use [module design](../../.agents/skills/engine-module-design/SKILL.md) for research, synthesis and proof.
Keep requirements, selection, implementation, evidence and user acceptance separate. Existing research may later supply
candidates; existing demonstrations do not select the architecture. The author has prior exposure to water research
and repository rules, so this is a fresh requirements exercise, not a claim of blind technical independence.

Current stage: 3D and desktop/Mac/browser/mobile scope recorded; custom host selected; coordinated research/design
produced an integrated high-level recommendation and independently challenged contracts. Bevy remains a reference
and source candidate, not the selected runtime. The [maturity ledger](MATURITY.md) keeps detailed mechanism choices,
minimum devices/budgets, implementation, visual acceptance and platform qualification explicitly unfinished.
