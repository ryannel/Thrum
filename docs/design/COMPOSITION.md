# Game composition and extensions

1 November 2026. Recommended high-level design supplement to [SYSTEM-DESIGN](SYSTEM-DESIGN.md).
This closes an underdeveloped part of the earlier architecture: how a developer assembles, extends and replaces
engine features. FOUNDATION-SPEC and the other [baseline specifications](BASELINE-SPEC.md) fix the initial protocols; examples below
remain illustrative SDK syntax, not implemented interfaces.
[C01–C13](CONTRACTS.md) continue to own identity, authority, scheduling, lifetime and persistence.

**Selection clarification:** the user cited Bevy's replaceable renderer as an observation about engine architecture,
not a requirement that everything be a plugin. The required outcome is flexibility, including omitting unnecessary
features. The [scenario walkthroughs](SCENARIO-WALKTHROUGHS.md) now recommend explicit static game setup and validated
target profiles as the default. Ordinary game code registers components and systems directly. Optional reusable
packages use the same public integration rules; richer package/lifecycle mechanisms below apply only where requested.
Feature configuration, build exclusion, public extension, subsystem replacement and executable hot loading remain
different promises. [ENGINE-EXPERIENCE](ENGINE-EXPERIENCE.md) is the concise developer-facing recommendation.

The precedents differ. [Bevy](https://bevy.org/learn/quick-start/getting-started/plugins/) explicitly packages engine
features as plugins. [Unity SRP](https://docs.unity.com/en-us/engine/6000.0/manual/render-pipelines/scriptable-render-pipeline-introduction)
supports custom render pipelines over its rendering facilities.
[Unreal](https://dev.epicgames.com/documentation/unreal-engine/graphics-programming-overview-for-unreal-engine)
puts rendering in a separate module reached through interfaces, with dependencies back into Engine; that alone does
not demonstrate drop-in renderer replacement.
[Godot](https://docs.godotengine.org/en/4.7/engine_details/architecture/internal_rendering_architecture.html)
separates rendering methods and graphics drivers, with RenderingDevice for modern backends. These sources support
deliberate boundaries, not one identical packaging model. A no-sky game can use an ordinary optional render feature;
the stronger ability to replace the whole renderer needs its own justification and compatibility contract.

## The developer model

Build games through an explicit game setup: select capabilities and meaningful model choices, assemble entities from
data components, and register ordinary game systems against public contracts. Statically linked modules are sufficient
for the baseline. Reusable packages use those same extension interfaces when packaging adds value. A module is a
responsibility boundary; a plugin is an optional installable unit; a crate is a code/build boundary.
They need not map one-to-one. In particular, the broad E1 environment family must not become an indivisible plugin.

The [from-scratch pass](FROM-SCRATCH-WALKTHROUGHS.md) adds guided empty creation, direct setup and headless-first
construction as approaches over this same model, not permanent project modes. Convenience additions emit ordinary
inspectable source; no recipe order bypasses access/schedule validation. Authoring helpers are separate from game
camera/light/content. A valid draft need not be runnable: F2 evaluates the requirements of the selected launch/build,
including a headless entry with no renderer dependency. Draft source admission and runtime activation retain their
separate obligations under C10: tool changes appear in ordinary working files before immutable snapshot admission;
cook/play read only that admitted tree. Proposals and interrupted file applications never activate themselves.
[FOUNDATION-SPEC](FOUNDATION-SPEC.md) owns the protocol; substantive changes restart a disposable run.

The [first-use walkthroughs](FIRST-USE-WALKTHROUGHS.md) define the teaching order: configure a working example, add
ordinary game code or a library, then introduce specialist provider contracts only when needed. Local helpers need
no separate registration; systems declare actual data/access/order through the public surface. Mechanical schema
declarations should be co-located or derived with inspectable expansion, without hidden scans choosing authority or
schedule. A starter must graduate into an ordinary project after its sample scene/rule is deleted. Reusable extension
guides use a minimal working consumer and conformance checks for the capabilities/targets actually claimed; they do
not require a universal plugin ABI or promise migration between arbitrary private provider states.

| Concept | What it means | Example |
| --- | --- | --- |
| Shared runtime | Common identity/schema, scheduling, lifecycle and typed registration | No mandatory renderer, physics, audio, weather or editor; may reuse libraries |
| Optional feature package | Versioned reusable capability and its integration when packaging is useful | Physics adapter, custom effect or shared game feature; not required for every rule |
| Entity component | Typed data attached to an entity; identity and schema owned through F3/W1 | Transform, collider reference, animation state, health, interactable |
| System | Work over declared components/resources, with explicit access and causal order | Damage processing, animation evaluation, camera follow |
| Provider | A concrete implementation of a capability contract | Physics adapter, shadow technique, authored or simulated wind field |
| Profile | A game's selected capabilities, provider choices, platform constraints and budgets | Overhead browser demo or desktop third-person game |
| Prefab/recipe | Reusable authored entity/component composition and references | Character with body, collider, animation, health and sound emitter |

Components describe objects; they do not each own an update loop. Systems process batches of relevant data. ECS
storage is suitable for composition and queries, while physics, animation, navigation and GPU implementations keep
their specialized stores behind their owners. An ECS component may reference a derived representation; it must not
create a second authoritative pose or an independent lifetime. W1 selects hecs 0.11.2 as private ordinary-component storage behind owned identity/schema/access contracts; no backend
handle is a public extension API. Neither an object inheritance hierarchy nor making every GPU resource an entity is required.

N1 is an optional capability with first-class authority semantics: a standalone world commits locally; networked
rules submit typed actor intent to the server and consume coherent replica projections. Game code must declare
replicated fields/commands and permitted speculative effects; adding a transport does not make arbitrary local rules
multiplayer-safe. Networking-off performs no replication work. Materials, providers and render passes can extend the
baseline; arbitrary full/live renderer replacement remains outside its compatibility promise.

## Packaging and dependencies

The application composition root explicitly selects feature modules, optional packages and target profiles. Shared
infrastructure depends on contracts, not concrete feature implementations. Modules depend on small public contracts
and declared prerequisites; they cannot import a neighbour's private solver or cache. Game code depends on engine contracts; engine packages
never depend on a particular game's rules. Editor integrations and cook-time extensions are separate build products
from runtime support, even when distributed in one plugin package.

Examples of packages and their public seams:

| Feature packages | Extension surface | Shared authority retained |
| --- | --- | --- |
| Rendering, lighting, materials, optional sky/cloud drawing | Render extraction, material closures, light/media inputs and declared GPU passes | V1 device/submission; V3 transport composition; V4 reconstruction |
| Weather fields, cloud attenuation, terrain, vegetation, water | Typed fields, queries, disturbances and render producers | E1 field owners; W1 world; P3 body motion |
| Physics, locomotion, animation, navigation | Body/query contracts, motion requests/results, pose providers, navigation queries | One transform/motion authority and explicit tick boundaries |
| Audio, UI, input, camera rigs | Source events, semantic UI, action maps, view descriptions | Audio listener policy, input consumption and per-view histories |
| Inventory, combat, quests or a game's rules | Custom component schemas, systems, commands and prefab recipes | Game-owned state; public engine commands for engine-owned state |
| Importers, inspectors, diagnostics and packaging adapters | Cook recipes, schema inspection, semantic operations and evidence hooks | F4 product graph; T1 operations; T2 build/evidence |

A plugin can contain several provider candidates without activating them all. One capability can have several
implementations, but only one selected owner per scope unless its contract defines a combiner. Replacement means
implementing a particular contract, not arbitrary substitution: two physics solvers may support different joints,
precision or determinism envelopes, and their private state need not be migratable. F2 rejects incompatible content
and profiles before activation. More specialized capabilities remain explicit extensions rather than being squeezed
into a lowest-common-denominator interface or exposed through untyped backend escape hatches.

## Declaration, validation and activation

Direct game registration declares its actual schemas, systems/access and content requirements without a separate
package descriptor. F2 validates explicit bindings and named defaults; it does not search for a cheaper physical
model. If an independently reusable package is useful, its descriptor declares stable ID/version, compatible contracts,
provided capabilities, component/resource schemas, systems, operation handlers, product recipes and extension points.
Each registration has a namespace and owner. Required/optional dependencies, conflicts, platform limits and
build/load/live support are explicit. Plugin instance scope (such as process/world/view) is distinct from provider
binding scope. Each binding names its capability, owning world and applicable region/view/domain key; tick/frame
and generation identify output validity, not a fresh authority. Multiple regional providers or view tiers may coexist
under distinct keys. Overlapping domains require a declared selection/combining rule; ambiguous coverage is rejected.
Two different owners cannot register the same identity.
Reusable plugin groups are named presets that expand into ordinary declarations, not privileged hidden behaviour.

Registration only declares the configured application; it does not start worker threads, create GPU devices, mutate worlds or
subscribe live callbacks. F2 validates the profile and content requirement closure. F3 validates schema/version
compatibility; F1 validates schedules; F4 validates required product closure. The resulting lock records exact package,
contract, schema and provider versions plus configuration and content references. Inspecting the lock explains why
each plugin is present, which consumers retain it and which work is excluded. Enumeration order is not update order.

After validation, prepare selected instances off-world under resource admission. Commit them through the existing
F1/W1/W2 barriers. Gameplay-affecting changes publish before a tick; presentation-only changes publish before
extraction under the matching generations. Failed preparation leaves the previous plan active. Registered but unused
features have no running systems, instances, cook products or recurring work; compiled code may still occupy binary
space unless excluded by the build. The manifest and source schema catalog can remain available to authoring tools.

Systems declare component/resource reads and writes, execution domain, semantic phase, explicit causal dependencies
and command-publication boundary. Conflicting accesses prohibit concurrent execution; a declared causal edge or
owner-defined reduction determines meaningful order. Reject ambiguous conflicting writes rather than letting plugin
installation order choose gameplay. Structural entity changes go through the world owner's command/barrier protocol.
Typed events carry source/tick/generation and declared delivery phase; callbacks cannot mutate committed state or
introduce an implicit cross-module solver. GPU pass order remains V1's resource graph, not the CPU schedule.

Extensions receive typed scoped access, not an unrestricted global service locator. Public render hooks submit work
to V1; they cannot create a second GPU queue. New editor panels and agent tools submit the same T1 operations as other
clients. Custom components declare persistence/migration and inspection rules, so they are not invisible blobs to AI
development tools. Native compiled plugins are trusted application code; access declarations are an engineering
contract, not a security sandbox for downloaded mods. Untrusted executable extension support is outside this baseline.

## Components, scenes and feature removal

Build/load configuration and rebuild/restart are the baseline. Live component edits and presentation-tier changes
require a declared supported path. The removal rules below constrain optional live gameplay removal; implementing
generic live removal, executable hot reload or migration is not a prerequisite for a game that never requests it.

A prefab declares component schema IDs/versions and required capabilities. Adding a component also validates its
dependencies: adding a rigid-body component requires a compatible physics provider and collision products. It does
not silently enable a default solver. Component removal checks relations and dependent components, then retires the
owner's derived state at the appropriate boundary. Data-only game components need no special engine feature plugin
beyond their schema and systems; lightweight custom gameplay should not require writing a backend.

A component requiring a specialized representation declares its projection/attachment owner through that subsystem's
public extension contract. Create/change/remove requests follow validate -> prepare derived state/products -> commit
component and required representation at one boundary -> retire old generation after leases. A third-party physics
component uses P3's declared attachment/query interfaces, not private solver storage. Its schema owner declares save
codecs/migrations; P3 owns the derived body's lifetime and C03 motion rules. Unsupported projections fail admission.

Content loading is a second admission check: a valid application profile does not make every possible scene valid.
Required unknown components or unavailable gameplay capabilities fail with a dependency explanation. Authoring may
preserve unknown serialized records losslessly for later recovery; that does not make them runnable. Optional
presentation data may be omitted from a target cook only under a declared omission policy. Saved game state retains
the required schema/provider/profile information through C10; disabling a feature does not silently discard its state.

| Change | Baseline guarantee |
| --- | --- |
| Exclude a plugin from a build | Remove its avoidable code/dependencies and products; validate all retained content/profiles |
| Select a compiled plugin/provider while loading | Resolve, prepare and activate a compatible plan before dependent content runs |
| Add/remove entity components | Validate invariants, publish commands at a world barrier and retire derived resources safely |
| Toggle presentation or change quality live | Only declared, prevalidated transitions; prepare replacements, invalidate history and release leases/fences |
| Disable a gameplay plugin or replace its provider | Only with explicit state migration/quiescence and game consent; otherwise requires world restart or is rejected |
| Load a prefab/asset while running | Content/schema operation; does not load new executable code |
| Unload/reload native plugin machine code | Not a baseline promise; compiled plugins stay linked for process lifetime |

For supported disable transitions, first enumerate the transitive active dependent closure: component relations,
specialized stores, product leases, queued work and affected operation handlers, plus retained content/save restore
requirements. Stage rebindings/migrations for every retained live consumer and recheck the closure generation at commit;
a new dependent joins preparation or forces reprepare. If any consumer has no valid replacement/removal policy, reject
the transition or require an explicit world restart. Do not wait indefinitely for dependencies to disappear.

Block new affected work, settle or explicitly cancel pending operations, and publish the validated active change as
one profile/world generation at the appropriate barrier. Retirement follows CPU leases/GPU fences. Retained saves
must keep a compatible restore closure/schema reader or have a validated migration; never delete their restore path.
C10 still separates active publication from durable RecoveryRoot/checkpoint publication: a live toggle is not a saved
checkpoint, and recovery may restore the prior compatible tuple until the new one is durable. Do not unregister schema
readers still needed by retained saves or content. Executable hot reload, asset reload and live feature selection are separate
capabilities. This provides a portable initial composition model without requiring a cross-platform binary plugin ABI.

The [cloud consumer matrix](modules/CLOUDS.md#responsibility-and-public-composition) concretizes one provider family:
main cloud drawing can be off while attenuation or a reflected cloud remains. Bind retained consumers explicitly;
retire only unused products. RC-one/RC-four resource capacities are load-time selections, not promises of zero-cost
live resizing or arbitrary provider migration.

## Two games, one engine

Illustrative recipe, with exact names/formats deliberately not selected. These are richer compositions, not mandatory
starter defaults: a board can use direct picking/topology without rigid-body queries, and omit audio/cloud attenuation.

```text
Overhead browser demo
  select: world/content, input, orthographic cameras, rendering/lighting,
          rigid-body queries, UI, audio, game rules
  require: Sun illumination, cloud attenuation
  bind cloud attenuation: authored moving transmittance field
  omit: visible sky, cloud-volume rendering, dynamic weather simulation
  compose actor: Transform + Collider + Health + Interactable + MeshRef + SoundEmitter

Third-person desktop game
  select: the relevant common features plus character locomotion, animation,
          navigation, perspective cameras, terrain, vegetation and weather
  bind cloud attenuation: compatible weather-driven field
  add: visible sky/clouds and higher-quality lighting providers
  compose actor: Transform + CharacterBody + Collider + AnimationState + Health + SoundEmitter
```

The first recipe still has a producer for cloud shadows: an authored field evaluated at declared world time, not a
reference to missing cloud geometry. Wind/weather simulation is unnecessary when that provider has no such dependency.
The second shares its weather fields with retained consumers. Turning off visible clouds removes their drawing and
private resources, not the field consumed by lighting. A headless profile can retain world/gameplay/physics and omit
rendering/audio/UI; dependencies must demonstrate this instead of relying on a hidden window or graphics device.

## Research basis and acceptance

One game-owned `Combat` feature illustrates direct registration; it needs no independent package or removal protocol
unless reuse/live removal is requested. It declares `Health` and `DamageIntent` schemas,
a damage system, inspection metadata, commands and persistence policy. A character prefab requires those schemas;
loading it without Combat rejects that requirement rather than dropping health. During a tick, the damage system
reads admitted intents and the declared health state, emits a health update through its scheduled W1 access, and
publishes stable damage/death events for downstream gameplay/audio. It cannot directly move the P3 body. P7 captures
Health through its schema codec at the save barrier; transient intents have an explicit consume/checkpoint policy.
Disabling Combat finds health-bearing entities, combat-dependent AI/UI handlers, queued intents and retained saves.
The default is rejection while these dependencies remain. A game can declare a transition to noncombat actors that
migrates state, removes/rebinds every live dependent, settles queued intents and preserves save restoration under C10.
F2/W1 validate and commit that complete change; turning off the damage system alone is not a successful removal.

Bevy makes both engine features and games plugins and provides grouped defaults/minimal selections. Retain that
compositional idea while adding our explicit contract resolution, authority and lifecycle rules; these additions are
our recommendation, not a claim about Bevy guarantees. [Official Bevy plugin guide](https://bevy.org/learn/quick-start/getting-started/plugins/).
Cargo features select optional compilation/dependencies and unify additively. They cannot alone enforce our exclusive
runtime provider ownership; the application profile resolver and resolved build manifest have separate jobs.
[Cargo feature reference](https://doc.rust-lang.org/cargo/reference/features.html).

D1 specifies direct schema/access registration, schedule validation, target/profile locks and content compatibility
before dependent construction. Package compatibility, extension conformance, migration and live-removal protocols
are required only for the reusable packages/transitions actually commissioned. Analyze duplicate providers, missing
component requirements, unavailable target features and stale asynchronous work; add live-dependent removal cases
only for supported removal. Admission rejects invalid combinations before mutation; unrelated modules must not
acquire private implementation dependencies. Implementation tests and target measurements then qualify these
predictions. The API sketch and paper traces do not establish implementation, binary size or performance.
