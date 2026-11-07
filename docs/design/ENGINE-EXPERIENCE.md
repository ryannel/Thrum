# Building a game with this engine

Creator guide, updated 7 November 2026. [G6](../BUILD-PLAN.md#active-gate-order-and-budgets) built and tested its
first slice; [What G6 built](#what-g6-built) says what runs today. Everything else here is design intent, not a
working SDK or settled syntax. The
[scenario](SCENARIO-WALKTHROUGHS.md), [first-use](FIRST-USE-WALKTHROUGHS.md) and [from-scratch](FROM-SCRATCH-WALKTHROUGHS.md)
walkthroughs are the paper traces behind it; [architecture decisions](ARCHITECTURE-DECISIONS.md) select the core and
library boundaries.

## The ambition

It should be very easy to make something great. A developer or an agent writes a scene, its rules and its controls
in a little ordinary code and data. The engine supplies showcase-quality rendering, sky and light, content handling,
simulation, demo controls and diagnostics without asking the author to configure its internal machinery. Defaults
look good, a mistake explains itself, and an edit appears in the running scene. Small projects use a small selection;
demanding ones add specific capabilities and models.

G6's budgets in the build plan are the first bar:

- one command takes an empty project to a lit scene with a physically based sky, a cooked mesh, a camera and a slider;
- common mistakes produce actionable diagnostics that name the object, the cause and a safe next step;
- assets, shaders and tuned parameters hot reload into the running scene;
- text, sliders, toggles and overlays of engine fields bind to typed game parameters, natively and in the browser, so
  a teaching demo can expose its mechanism;
- a fresh agent with only the public docs and the starter builds a new small scene, such as a sunset over terrain
  with a time slider, in at most 100 lines of game code and without reading engine internals. Its friction log is
  fixed or recorded.

Every extra concept or step on this path needs a scenario that benefits from it.

## What G6 built

The [evidence](../evidence/integration/G6-CREATOR-EXPERIENCE-2026-11-07.md) records the checks. Against the
budgets above:

- **One command.** `./dev starter`, or `./dev scene <folder>` for a copy, cooks the scene's assets, watches them and
  opens a lit scene. It has the physically based sky, a cooked mesh on lit ground, a camera and sliders. The
  [starter](../../examples/starter/README.md) is 42 game lines on the sky kit (`thrum_app::sky`), with no engine
  game types. This runs on the Mac only; the browser runs the V1 game's page, not the starter.
- **Diagnostics.** Each of these names the object, the cause and a next step:
  - control declarations and parameter files, with nearest-key suggestions;
  - mesh placements;
  - scene folders, before the lockfile changes;
  - material compiles, at the hook's file, line and column;
  - a missing asset store;
  - a window the display never drew.
- **Hot reload.**
  - Asset edits are re-cooked and drawn live.
  - Parameter files and the material hook reload explicitly: `L` on the Mac; on the page, a button and `?params`.
    The material is on the development page only.
  - Code changes rebuild and restart.
  - The material hook changes surface factors only and runs on every placement. It cannot yet read controls or
    know which placement it shades.
- **Controls.** Typed sliders, toggles, text and readouts drive an AppKit panel and the browser page from one
  model. Readouts of engine fields work in both. Overlays drawn into the frame, starting with exposure zones, wait
  on the shared display shader's tone-scale seam.
- **A fresh agent.** Three runs, each by an agent that read only the creator docs:
  - a stone circle in 114 lines;
  - a sunset in 76;
  - a lighthouse with its own generated mesh and material in 74.

  The last two used only the sky kit and `Config`. Their friction was fixed or recorded; the evidence lists which.
  Live reload still needs a key press in a visible window, so an agent on a locked screen reviews through the
  offscreen capture.

## Four things to understand

| Concept | What a developer works with |
| --- | --- |
| Project | Versioned scenes, assets, game code, feature choices and named target profiles. Files are the durable project source; tools work on the same records. |
| World | A running instance of game state and its selected services. Authored scenes supply objects, behaviour, lights/cameras and domains such as terrain or sky; their dependencies are checked. |
| Target | Destination, content entry points, supported visual tiers and budget assumptions. It selects compatible assets and rendering, without silently changing game rules. |
| Run | An isolated execution of an admitted project snapshot for a target; play state is separate from authored files. |

Assets, components and systems keep their ordinary meanings. A **play session** is a disposable run of the project;
a **build** packages a particular admitted revision for a target. Neither creates another authoring format. Domain
experts can inspect provider contracts, schedules and resource graphs; a first scene does not require them.

## A first useful change

The programming path assumes basic code and files, not knowledge of this engine or ECS internals. A familiar Rust
developer and someone learning Rust need different help; language learning is named, not hidden in a tutorial. No
AI subscription is required; agents and people use the same commands and files.

Offer an empty project and small editable starters. A starter is ordinary source with named, inspectable settings,
not a hidden mode: replacing its scene and deleting its example rule leaves an ordinary working project. There is one
effective game setup, and the tool shows which default or explicit setting supplied each choice. Mechanical schema
declarations are co-located or derived, so a tiny rule does not need several hand-maintained manifests. Registration
order and authority never come from hidden directory scanning. Engine updates never silently rewrite game code.

Before building, a prerequisite check reports the engine and toolchain version, the available targets and the next
missing prerequisite. Docs, starters and dependency versions belong to the same release, and teach only implemented
public APIs. Concepts arrive through tasks: a slider introduces a typed parameter, a collectible introduces object data
and a rule, saving introduces durable state.

## Starting from nothing

| Route | First meaningful result | Same underlying project |
| --- | --- | --- |
| Guided creation | Author a scene, establish its view and add one parameter, action or rule with visible feedback | Each addition creates inspectable ordinary source; the guide is optional |
| Direct construction | Register selected services and game data/systems, then select the entry world and view | Same validation, schedule, content and save boundaries; no privileged expert path |
| Rules first | Run a world with authored initial state and explicit commands, then add presentation | Headless execution needs no camera, renderer or graphics device |

These are approaches, not project modes; moving between them needs no conversion. A rendered launch needs an entry
world, a view and the required products. A missing camera or entry is explained, not silently supplied. Workbench
helpers such as a navigation camera, grid or inspection light never enter game source or exports unless authored there.
The first interaction teaches parameter or action binding, registered behaviour, state change and an inspectable result
together. Game-specific rejection reasons come from game code; the engine cannot infer an arbitrary rule's meaning.

## The ordinary loop

1. Create a project from a small editable starter or start empty. Starters expand to ordinary source.
2. Author world data, rules, controls and presentation as needed. Choose the few capabilities the game needs and
   register ordinary components and systems directly; no plugin package is required for a rule.
3. Run: the command explicitly admits a complete working-file snapshot, validates it and plays its pinned build. Missing requirements are
   reported through the object or feature that needs them, never by silently selecting a different physical model.
4. Tune and inspect the running game by object, parameter, action, rule, light, asset or frame. Assets, shaders and
   parameters hot reload; code and schema changes rebuild and restart quickly. The human UI and agent interface
   expose the same state and operations, with the active source, target and session identified.
5. Stop. Play changes are discarded by default. Applying selected authored properties back produces a reviewable
   source change with revision checks; transient state is never copied into the scene wholesale.
6. Build a target. Content, schemas and shader demand are validated together. A desktop build does not certify the
   browser or a phone; each target reports what has been qualified.

Native binary hot reload, a scripting VM and a full GUI editor are not prerequisites for this loop. Sharing source
means the pinned engine, tools and asset inputs; a development export means target products with launch or hosting
instructions that do not depend on the author's caches. Neither implies release qualification.

## Recommended simplifications

| Decision | Recommendation and reason |
| --- | --- |
| Game composition | Explicit, statically linked game setup plus dependency/content validation. Named, inspectable defaults and recipes make common cases convenient. No general provider search. |
| Reusable extensions | Ordinary modules and libraries with public contracts first; a package descriptor only when distribution warrants it. |
| Feature changes | Build/load selection by default and a finite set of declared visual tier changes. Live replacement of gameplay models is unsupported unless designed. |
| Rendering flexibility | Optional render features, material/style extension and qualified provider choices. The renderer stays separable from simulation for headless use. |
| Physical models | The author chooses behaviour and a compatible model. The engine checks requirements; it does not optimise away a physical behaviour to meet a visual budget. |
| Game iteration | An isolated play process with a pinned revision and private test saves; hot reload for qualified assets, shaders and parameters, restart otherwise. |
| Project editing | Ordinary working files are the edit surface. Tool patches go through one coordinated writer; a watcher only reports change. |

Diagnostics show what is included and why. A named preset expands defaults transparently; alternatives that change
semantics need an explicit choice. Avoidable runtime work is removed by plan, and the saving is measured.

## Using more, bringing your own

| Intent | Shortest supported path | When more is needed |
| --- | --- | --- |
| Tune or omit a built-in feature | Edit settings or the game setup; inspect dependencies; validate and restart | Removing something still required names the consuming object and a deliberate alternative |
| Add a game rule or library | Call ordinary code from a game system; register only the data and access used | Persistence, external work or engine-owned effects use the corresponding public boundary |
| Replace a specialised implementation | Adapt one named capability contract, with declared targets and semantics | State, timing, queries, products, saves and authority need explicit mappings |
| Add a render effect or material | Use the narrowest public material or pass extension | Required inputs, render paths and targets must support it; whole-renderer replacement is not implied |

Built-ins use the same public boundary promised to an external implementation. Installing code and enabling it are
distinct; removing an opt-in identifies dependent objects rather than deleting them or changing their behaviour.

## Making it look intentional

The starter scene should look finished: physically based sky, sun and aerial perspective from G3s, sensible exposure
and tone mapping, coherent materials and a camera that frames the subject. A style recipe is ordinary content and
configuration, not another subsystem or a one-dropdown promise. Import reports what was recognised, converted or
unsupported; reimport keeps game-owned overrides and stable references, and reports ambiguous matches for repair.
Unsupported shaders or effects get a target-specific explanation and an explicit fallback choice. Rendering changes
never alter colliders, rules or physical models behind the author's back. A visual workbench for artists is later
work; artist usability stays unqualified until it exists.

## When something goes wrong

Lead with the developer's object and action, then the cause and a safe next step. For example: “Boat requires water
surface queries, but Decorative Pond only draws a surface. Choose a query-capable water model or remove Boat's
buoyancy behaviour.” The diagnostic links the source and a relevant example; technical IDs and dependency paths stay
available on demand. No automatic switch to a physically different model repairs this failure.

Make the current state clear: editing, preparing an update, running the last admitted revision, or failed. An old
play session never appears to contain newly rejected code. Human and agent diagnostics share structured causes and
source references while the human message stays short. Prerequisite, source, content, compatibility, build and device
failures need different remedies; a generic “validation failed” is insufficient. G6 measures this with the fresh-agent
friction log; the [evidence](../evidence/integration/G6-CREATOR-EXPERIENCE-2026-11-07.md#slice-4-one-command-starter-and-fresh-agent-scenes)
records three runs.

## Source and play semantics

Edit ordinary files, inspect the diff, admit a complete immutable snapshot, build and run. A tool or agent operation
proposes a patch that applies only if its base still matches; it never creates a hidden admitted change.
[C10](CONTRACTS.md#c10-persistence-reproducibility-and-authored-operations--p7f3t1) owns the invariant and the
[Foundation specification](FOUNDATION-SPEC.md#2-file-application-and-source-admission) owns application, crash,
retry and [recovery](FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc). Each play session has its own world
epoch, clocks, events and private test saves; a failed build leaves the old run visibly pinned. Authoring is native;
the browser plays and exports.

## Water and atmosphere without unnecessary generality

Atmosphere outputs are selected separately: background sky, environment light, direct transmittance and aerial
media. The V1 atmosphere is G3s; a no-sky game can still be beautifully lit. Overlapping media need a joint owner or an
explicit approximation (C08, RENDER-SPEC). Water is parked until after V1. When it returns, use one conservative water
domain where flooding or water accounting matters and distinct models where behaviour or cost justifies them, with
no default generic solver-coupling framework.

## Multiplayer without rebuilding the game around it

A single-player game uses local authority. A network-capable game uses the same typed actions and declares its
replicated state; N1 adds sessions, join/reconnect and optional bounded prediction. Local camera and UI response stay
independent of authority, and clients may choose different visual quality while gameplay semantics agree. V1 proves
this headless in G2S with rollback, amended-input replay and full-state equality; transport and rendered peers follow
V1. Turning on sockets never makes arbitrary single-player code correct.

## What should earn its place

The [AI development design](AI-DEVELOPMENT.md) applies the same loop to agents making games or contributing engine
code. Game authors use the public SDK without inheriting the engine's internal design workflow. Preserve the hard
invariants (state authority, source identity, readiness, resource lifetime and explicit approximations) inside the
engine and its tools. A simple front end must not conceal an unimplemented mechanism or pretend incompatible models
are interchangeable. G6's fresh-agent runs are a first measure; people using the engine decide whether it is
actually pleasant and fast.
