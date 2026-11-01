# Building a game with this engine

1 November 2026. Recommended developer experience, refined through the
[scenario walkthroughs](SCENARIO-WALKTHROUGHS.md). This is an engineering recommendation, not a working SDK, measured
usability result or user acceptance of a particular API. Exact syntax remains detailed design; [architecture decisions](ARCHITECTURE-DECISIONS.md) now select the core and library boundaries.
The [first-use review](FIRST-USE-WALKTHROUGHS.md) further specifies onboarding, external code and art workflows.
The [from-scratch review](FROM-SCRATCH-WALKTHROUGHS.md) covers empty projects, newcomers and industry veterans.

Make a game by authoring scenes, writing its rules and choosing where it runs. The engine should supply coherent
rendering, content handling, simulation services and diagnostics without making every project configure the engine's
internal machinery. Small games use a small selection; demanding games add specific capabilities and models.

## Four things to understand

| Concept | What a developer works with |
| --- | --- |
| Project | Versioned scenes, assets, game code, feature choices and named target profiles. Files are the durable project source; tools work on the same records. |
| World | A running instance of game state and its selected services. Authored scenes supply objects, behavior, lights/cameras and domains such as terrain or water; their dependencies are checked. |
| Target | Destination, content entry points, supported visual tiers and budget assumptions. It selects compatible assets and rendering, without silently changing game rules. |
| Run | An isolated execution of an admitted project snapshot for a target; play state is separate from authored files. |

Assets, components and systems retain their ordinary meanings. A **play session** is a disposable run of the project;
a **build** packages a particular accepted project revision for a target. Neither creates another authoring format.
Domain experts can inspect provider contracts, schedules and resource graphs, but first-scene creation should not
require understanding all of them.

## A first useful change

The initial programming path assumes familiarity with basic code and files, not with this engine or ECS internals.
Artist participation needs visual affordances; it is not satisfied by calling a command-line API accessible.
Neither a no-code game maker nor an AI subscription is required by this recommendation.

Offer an empty project and optional small editable examples; starting empty must be a documented path for newcomers
as well as experts. A guided path can add a scene, camera, object, light and action in understandable steps. These
produce ordinary source with a visible diff; they do not select a hidden game mode. The engine supplies host/backend
infrastructure without asking the author to implement it. Experienced users can make the same choices directly.
Examples must run unchanged on their named supported target, include an interaction, and point to its source.

Before building, a prerequisite check reports the selected engine/toolchain version, available target support and
the next missing prerequisite. Install instructions for one target must not require every mobile SDK. Starter code,
assets, docs and dependency versions belong to the same release; an unsupported platform is identified before a long
cook. SDK/toolchain distribution, supported initial hosts and actual iteration latency still need detailed decisions.

Starters are ordinary project source with named, inspectable settings. Explain where to change rules, scene content,
assets, feature selection and target settings, without requiring the beginner to edit every file category. There is
one effective game setup; the tool can show which default or explicit setting supplied each choice. Copying an example
does not create a special mode that must later be escaped, and engine updates do not silently rewrite game-owned code.
Replacing the sample scene and deleting its example rule must leave an ordinary working project. Mechanical schema
declarations can be co-located or derived with inspectable results; a tiny rule should not require hand-maintaining
several redundant manifests. Registration order and authority must not come from hidden directory scanning.

Introduce concepts through tasks: a collectible introduces object data and a rule; saving introduces durable state;
an external physics implementation introduces provider contracts. Generic lifecycle, migration and plugin packaging
do not belong in the first collectible tutorial. FOUNDATION-SPEC selects JSON source records; SDK docs must use the eventually implemented public APIs
from the release they teach, rather than plausible pseudocode presented as executable instructions.

## Starting from nothing

| Route | First meaningful result | Same underlying project |
| --- | --- | --- |
| Guided creation | Author a scene, establish its game view and add one action/rule/state/feedback path | Each addition creates inspectable ordinary source; the guide is optional |
| Direct construction | Explicitly register selected services and game data/systems, then select the entry world/view | Same validation, schedule, content and save boundaries; no privileged expert mutation path |
| Rules first | Run a world with authored initial state and explicit commands, then add presentation when useful | Headless execution does not require a camera, renderer or graphics device |

These are working approaches, not permanent project modes. Moving between them requires no project conversion.
A new project may have no scene or behavior yet. Saving a draft is different from admitting a runnable snapshot;
readiness checks name the chosen action and target. A rendered launch needs an entry scene/world, an intended view
and the required products. An empty world can deliberately render nothing; no gameplay interaction is required merely
to launch. A headless run has its own entry/state requirements. Missing camera or entry selection is explained rather
than silently supplied by the workbench. C10 selects ordinary working files and explicit immutable admission; FOUNDATION-SPEC fixes the initial native admission and browser runtime-save boundaries.

The workbench's navigation camera, grid, selection outline and neutral inspection light are identified as authoring
helpers. They do not enter game source or exports unless explicitly authored there. A game-camera preview shows what
the game actually selects. Adding minimal game camera/light/material defaults is useful when requested; their values
and ownership remain visible. An unlit style can intentionally omit scene lights.

The first interaction teaches action mapping, registered behavior, required object data, state mutation and an
inspectable result together, with visible feedback for rendered games. Inspection can expose consumed input, a paused world, a disabled/unregistered system or a query
matching no objects. A game-specific rejection reason must be supplied by game code; the engine cannot infer the
meaning of an arbitrary algorithm. Examples and tooling must distinguish source registration from active execution.

An artist can author appearance through the visual workbench. Creating new rules still requires game code or a
specifically supported reusable behavior; no general visual rule editor has been selected. Document the programmer
handoff honestly. A veteran bringing familiar workflow concepts needs a short mapping to this engine's data, systems,
prefabs, actions, lifecycle and ownership, plus navigable APIs; experience with another engine does not imply Rust fluency.

## The ordinary loop

1. Create a project from a small, editable example or start empty. Examples expand to ordinary source; they do not
   introduce a hidden engine mode or an unchangeable game template.
2. Author world data and game rules as needed. Add scene objects/assets and presentation when needed. Choose the few
   capabilities the game needs and register ordinary components/systems directly; no plugin package is required for a rule.
3. Explicitly admit a complete working-file snapshot, validate it and play its pinned build. The engine reports missing requirements through the object or
   feature that needs them, rather than silently selecting a different physical model.
4. Inspect the running game by object, input/action, rule, light, asset or frame. The human UI and agent interface
   expose the same state and operations, with the active source/target/session identifiable.
5. Stop play. Gameplay changes are discarded by default. Explicitly applying selected authored properties back
   produces a reviewable source change with revision checks; simulated earnings, handles and transient state are not
   indiscriminately copied into the scene.
6. Build a target. Required content, schemas and shader demands are validated together; package and device evidence
   say what has actually been qualified. A successful desktop build does not certify a browser/mobile target.

Sharing source means providing the pinned engine/tools, assets and import inputs needed by another developer.
Sharing a playable development export means providing a target package and launch/hosting instructions that do not
depend on the author's workbench or warm caches. Neither implies release qualification or store submission. Offline
play, offline asset cooking and offline SDK installation are separate support claims. Required dependencies must be
available before an offline workflow is advertised.

Code or schema changes rebuild/restart the play process. Only qualified presentation fields may use a declared safe live preview; source/gameplay changes build and restart from the admitted snapshot by default. Native binary hot reload, a scripting VM and a full GUI
editor are not prerequisites for this loop. Code-iteration latency still needs measurement later.

Discarding play means discarding unsaved running state, not deleting every test artifact. An explicitly retained
private test save may be reopened after a code restart for restore/migration testing; its supported schema and lifetime
are visible and it never becomes a player save or authored scene automatically. C10 now scopes roots per authority and private session; FOUNDATION-SPEC now fixes initial retention/epoch bounds.

## Recommended simplifications

| Decision | Recommendation and reason |
| --- | --- |
| Game composition | Explicit, statically linked game setup plus dependency/content validation. Named, inspectable defaults and recipes make common cases convenient. No general provider search/optimization is required. |
| Reusable extensions | Ordinary modules/libraries with public contracts first. Add a package descriptor when distributing a reusable extension warrants it; use the same schemas, build products and integration rules. |
| Feature changes | Build/load selection by default. Support a finite set of declared visual tier changes. Replacing/removing gameplay models while running is unsupported unless specifically designed. |
| Rendering flexibility | Optional render features, material/style extension and qualified provider choices. Keep the renderer separable from world simulation for headless use. Do not promise arbitrary whole-renderer or material compatibility. |
| Physical models | The game/domain author chooses behavior and a compatible model. The engine checks requirements and validity; it does not optimize away flooding or buoyancy to meet a visual budget. |
| Game iteration | An isolated play process with a pinned source/build revision and private test-save namespace. Stop/restart gives a clear boundary and keeps editor/CLI source work outside gameplay mutations. |
| Project editing | Ordinary working files are the edit surface. A reviewable tool patch is applied through a coordinated writer, then a complete snapshot is explicitly admitted. A watcher only reports change. |
| Target products | Build the finite required content/shader set for declared content and supported visual tiers. Driver pipeline preparation may still occur at load time; unsupported or unready required work blocks activation. |

An external extension can register reusable behavior or a model without imposing live unload, a binary plugin ABI or
its own world/GPU authority. A named preset may expand defaults transparently, but alternatives that change semantics
require an explicit choice. Diagnostics show what is included and why. Avoidable runtime work is removed by plan;
actual binary size, memory and frame savings must be measured.

## Using more, bringing your own

| Intent | Shortest supported path | When more is needed |
| --- | --- | --- |
| Tune or omit a built-in feature | Edit ordinary settings or the game setup; inspect dependencies; validate and restart | Removing something still required reports the consuming scene/object and a deliberate alternative |
| Add a game rule or library | Call ordinary code from a game system; register only data/access actually used | Persistence, external work or engine-owned effects require the corresponding public boundary |
| Reuse a game feature | Group existing components/systems/content through ordinary code | Distribution may justify a versioned package and compatibility metadata |
| Replace a specialized implementation | Adapt one named capability contract, with declared supported targets and semantics | State, timing, queries, products, saves and authority need explicit mappings where applicable |
| Add a render effect or material | Use the narrowest public material/pass extension that supplies the intended result | Required inputs, render paths and targets must support it; whole-renderer replacement is not implied |

An integration guide should start from a working example and show the smallest change. Built-ins must use the same
public boundary promised to an external implementation; private shortcuts cannot be the only way to make the example
work. A specialist adapter needs a small reference consumer and checks for its actual contract, not a requirement to
implement every engine feature. Model compatibility and save compatibility are separate promises. Restart is the
default replacement path; existing saves are migrated only when a supported migration exists.

Installing code and enabling it in a game are distinct actions. Removing an opt-in must identify dependent source
objects and, where relevant, queued work and retained save readers; it must not delete those objects automatically or
silently change their behavior. Save compatibility is checked against the game's declared supported versions, not a
claim that every deployed player save can be inspected during development.

## Making it look intentional

Offer a restrained, attractive baseline scene as an optional example, with coherent camera, material, light and display settings. Offer
editable examples for realistic, stylized and painterly work as their supported paths exist. A style recipe is ordinary
content and configuration, not another engine subsystem or a promise that arbitrary artwork changes style through
one dropdown. Model shape, texture language, framing, materials, light and display all contribute.

The minimum visual workbench frames/selects an object, previews it through the selected runtime when available,
exposes camera/light/material fields with units, shows source versus play changes, and supports undo through the
shared operation surface. It names the actual target and labels any proxy preview or fidelity mismatch. It need not
include a full modeling tool, node editor or timeline suite.
Artist usability is unqualified until this path exists, even if the underlying CLI can perform the edits.

Import reports what was recognized, converted or unsupported: scale/orientation, textures/materials, skeleton and
animation where present. Keep masters, import settings and game-owned overrides distinguishable. Reimport preserves
stable references where correspondence is known; removed or ambiguous source nodes produce a repairable conflict,
not a guess that retargets a collider or discards a material edit. A neutral asset view helps separate import defects
from scene lighting. Target preview uses the actual supported material/pass path, or labels its fidelity difference.

Changing style can require authored material mappings and geometry changes. Unsupported shaders/effects get a
specific target/material explanation and an explicit fallback choice. Rendering changes do not change colliders,
rules or physical models behind the author's back. A no-sky example must show how light and cloud attenuation remain
selected independently, rather than implying that a sky checkbox controls all outdoor lighting.

## When something goes wrong

Lead with the developer's object and action, then the cause and a safe next step. For example: “Boat requires water
surface queries, but Decorative Pond only draws a surface. Choose a query-capable water model or remove Boat's
buoyancy behavior.” The diagnostic links the source and relevant example; technical IDs and full dependency paths
remain available on demand. No automatic switch to a physically different model repairs this failure.

Make the current state clear: editing source, preparing an update, running the last accepted revision, or failed.
An old valid play session must never appear to contain newly rejected code. Human and agent diagnostics share
structured causes and source references, while the human message stays concise. Prerequisite, source, content,
compatibility, build and device failures need different remedies; a generic “validation failed” is insufficient.

## Simple source and play semantics, precise remaining obligations

The everyday path is **edit ordinary files → inspect the diff → explicitly admit a complete immutable snapshot → build → run**. A well-formed draft can be saved before it is runnable for a target. A workbench/AI operation proposes a patch with a semantic diff; the coordinated writer applies it to working files only if its admitted base and affected paths still match. It does not create a hidden accepted change. Direct file edits use the same explicit admission. The [Foundation specification](FOUNDATION-SPEC.md#2-file-application-and-source-admission) owns the application, crash, retry, retention and recovery protocol; [C10](CONTRACTS.md#c10-persistence-reproducibility-and-authored-operations--p7f3t1) owns the invariant. `accepted` means the exact captured bytes have a durable admitted root under the native adapter's declared guarantee, not that cooking, play or saving completed.

A watcher only reports working changes. An immutable Git tree can bound a capture; a dirty multi-file set needs a declared finished set and verified captured bytes. A noncooperating writer can race a cooperative window, so uncertain captures are reported as conflicts. Interrupted application leaves visible partial files and an unaccepted intent for review; recovery never replays writes over unknown bytes. A crash after admission recovers the exact pinned snapshot/outcome even if files later change. Git diff is useful for review, while the internal immutable snapshot supplies exact build/recovery bytes and root/outcome atomicity. This extra native source copy is bounded by [Foundation's disk admission](FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc), and retained source masters still need the project's declared durable handoff.

Each play session has its own world epoch, clocks, events, providers and private test saves. A source/gameplay change builds and restarts a disposable run by default; a failed build leaves the old run visibly pinned. Initial gameplay may preload finite content or use an explicit stopped loading boundary. Qualified presentation fields may preview live under a declared compatibility and activation rule. A broader live-streaming transaction requires its own scoped state-transfer and capacity design. Apply-back selects schema-supported authored fields from the pinned play base, proposes a reviewed file patch against the current admitted source, and never copies earned/solver state or replica changes indiscriminately. A retained private test save stays separate from player/server saves and requires its exact compatible closure or migration.

Initial authoring is native. Browser play/export can use runtime product caches and player saves with explicit persistence/export policy; a full browser source editor is a later separately specified feature. Source handoff includes pinned tools and required masters, and development export includes target products and launch instructions. Neither internal cache nor Git metadata alone replaces those inputs.

## Water and atmosphere without unnecessary generality

Use one conservative physical water domain where the game needs connected flooding or water accounting, even if
streaming divides its data into cells. Use distinct models where the different behavior or cost justifies them.
A coastal exploration scene may use prescribed sea waves and authored river flow with a declared visual/prescribed
boundary; it does not claim a closed hydrological simulation. A later conservative model-to-model connection needs
an explicit supported adapter. There is no default generic solver-coupling framework and no fabricated height sample
for an unsupported falling/volumetric region.

Atmosphere selection is similarly explicit: background, environment light, direct transmittance and local/aerial media
are separate outputs. A no-sky game can still have beautiful illumination. Compatible overlapping coefficients may
be jointly integrated; a preintegrated ray segment is used within its declared support/composition policy. Independently
preintegrated overlapping media are rejected without a joint owner or explicit approximation. C08/E1/V3 now select these rules, including joint incident lighting for overlaps. RENDER-SPEC now selects the initial joint air/fog approximation and numerical targets; more advanced providers need
a separately complete design before being advertised.

## Multiplayer without rebuilding the game around it

Choose local authority for a single-player game. A network-capable game uses the same typed actor actions and
explicitly declares replicated state; N1 adds server/replica sessions, join/reconnect and optional bounded prediction.
Local camera/UI response remains independent of authority. Different clients may select different visual quality,
while gameplay content/query semantics must agree. A browser peer can receive confirmed physical projections without
running the server's full water solver. Local play requires no online service, and multiplayer support never means
that arbitrary single-player code becomes correct merely by turning on sockets.

## What should earn its place

The [AI development design](AI-DEVELOPMENT.md) applies this same loop to agents contributing engine code or making
games. Version-matched skills guide the task; types, linters and validators enforce explicit rules. Bounded structured
queries expose real schemas, dependencies and runtime state, while checks/captures identify the exact tested revision.
Engine contributors receive relevant contracts/consumers; game authors use the public SDK without inheriting the
engine's entire internal design workflow. These are proposed tools and instruction bundles, not installed capabilities.

Every common developer task should have a short path and a useful failure explanation. Extra concepts need a scenario
that benefits from them. Preserve the hard invariants—state authority, source identity, readiness, resource lifetime
and explicit approximations—inside the engine and its tools. A simple front end must not conceal an unimplemented
mechanism or pretend incompatible models are interchangeable.

The value is the combination: coherent scenes and rules, high-quality optional presentation, reusable models, reliable
iteration and target delivery, and the same inspectable workflow for humans and agents. The walkthroughs support
this shape analytically; running the examples will eventually determine whether it is actually pleasant and fast.
