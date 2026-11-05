# Making an original game from an empty project

1 November 2026. Nine additional analytical journeys from three fresh Sol High agents, reconciled by the coordinator.
The earlier [first-use cases](FIRST-USE-WALKTHROUGHS.md) often began with working content or controls; this pass begins
without a scene, camera, action map, game rule or imported asset. Host/backend infrastructure can still be supplied
by the engine. Starting an original game does not require implementing a window loop or graphics backend.

These are paper traces, not working SDK examples, timed usability sessions, image acceptance or target qualification.
The [developer experience](ENGINE-EXPERIENCE.md) carries the resulting recommendation. A01–A05 remain open. No new
template format, scripting language, no-code editor, platform floor or arbitrary renderer replacement is selected.

## One project, different amounts of guidance

Distinguish new to games, new to 3D, new to this engine, and unable to program. A veteran of another engine may also be
new to Rust. Documentation must state its prerequisite knowledge and offer short task-specific explanations; neither
an experienced artist nor a shipped-game developer should be assumed to know this engine's execution model.

Guided creation adds ordinary source in small visible steps. Direct construction uses the same registration, scene
and build boundaries. Rules-first development can begin headlessly, then attach presentation to the same world.
These are approaches, not persistent project modes; the user may switch approaches without conversion. Examples are
optional references, and convenience defaults are inspectable. No choice of camera or visual style requires selecting
a genre or filling in a catalogue of every engine capability before work can begin.

An empty well-formed draft can be saved even before a selected launch is ready. Edit validity, play readiness and
export readiness are scoped checks. Typed operations still enforce schema, authority and relevant invariants; this
does not authorize malformed operations or partial runtime activation. Malformed external files remain recoverable
source with diagnostics, not an accepted executable snapshot. Exact source admission/recovery remains A02/D1 work.

Rendered launch needs an entry world/scene, a selected game view and its required products. A deliberately empty
scene or unlit style is valid; interaction and scene lights are not universal launch prerequisites. A headless entry
needs neither a camera nor a GPU. The first *interactive* milestone adds an action, behavior, state and feedback.
The editor's navigation camera, grid, selection outline and diagnostic light are clearly identified helpers, excluded
from the game unless deliberately authored. They must not conceal missing runtime content or lighting.

## A matched first interaction: newcomer and veteran

Both creators make an overhead seed-collection game from empty source. This comparison examines the same mechanism,
rather than assuming the veteran wants a more complex game. The style begins with simple graphic objects, then gains
the creator's painted assets. Picking and game-owned positions need no rigid-body dynamics unless the design asks for it.

| Step / result | First-time game developer | Industry veteran |
| --- | --- | --- |
| Create a project for one target | Prerequisite check explains compiler, content tools and which support exists; optional guidance creates inspectable setup | Pins versions and writes/selects the effective setup directly; sees all implicit defaults and dependencies |
| See an authored object in the game view | Creates a scene and seed object; learns transform/scale and camera framing; chooses explicit material/light or unlit response | Authors the scene and view, chooses material/display policy and supplies/imports geometry without unwanted game content |
| Select the seed | Names an input action and picking target, follows how input reaches the rule | Registers action mapping and a typed system with access/order; can use an ordinary library inside the rule |
| Collect it and open a gate | Adds seed count and gate state; rule validates collection and commits state; visible feedback follows | Uses the same state boundary; no private transform write or render-side rule shortcut |
| Understand a failed click | Guidance distinguishes focus consumption, missing object data, inactive behavior and an authored rejection reason | Inspects declarations, active schedule, matched objects, state/events and source revision directly |
| Add original art and share | Reviews import mappings, changes materials/light, exports a development package with its limitations | Uses scripted imports/build operations and verifies source inputs and target products; same source/override rules |

The meaningful difference is explanation and control, not separate runtime code. A library is not a plugin merely
because it is imported. A system registration is a declaration, not proof it ran. Inspect declaration → validated
plan/products → accepted source → constructed world → active systems and their results. Game-specific reasons such as
“already collected” must be authored outcomes; the engine cannot infer them from arbitrary game code.

## The nine journeys and what each exposed

### Z01 — Basic coder, first game: colorful browser garden

Create the project, add an entry scene, floor/seed, overhead camera and intentional material/light/display choices.
Bind pointer/keyboard selection, add game-owned seed count and a pickup rule, then drive the gate from accepted state.
Replace primitives with original art; water, weather, navigation and skeletal animation stay absent. Diagnose a missing
action binding or required object field, restart at a named revision, and export the selected browser target. A share
link additionally needs a supported hosting/launch handoff. **Finding:** the guide must teach the full first dependency
chain and name language prerequisites; a starter previously supplied both invisibly.

### Z02 — General programmer, new to 3D: graphic mirror puzzle

From empty setup, author an emitter, mirror, receiver and door with a camera and known coordinate/scale conventions.
Map selection/rotation actions to game-owned orientation; a pure beam computation determines receiver/door state.
Beam graphics follow that calculation. Add physical collision only if the game requires it. Import a new mirror and
review its pivot mapping; a changed source pivot must not silently invalidate an authored rotation. **Finding:** this
user needs spatial tools and ownership explanations more than basic programming lessons. Inspection distinguishes
source placement, running pose, camera visibility and any collision result.

### Z03 — Artist with limited coding: painterly touch diorama

Import original lantern/ground assets, review units/materials and compose a camera/light/display treatment. Add a tap
target, lantern/path state and sound/visual feedback through a programmer or a specifically supported reusable behavior.
The minimum workbench alone does not author a new rule. Reimport a renamed lantern node, repair the interaction mapping,
then test actual native touch/suspend and package the selected mobile target when supported. **Finding:** artist-friendly
visual editing is a valid scope; independent no-code game creation is an additional, unselected product promise.

### Z04 — Shipped-game developer changing engines: third-person key and door

Declare a desktop setup, create room/player/key/door, choose body/controller ownership and add a runtime camera, actions
and direct rules. Trying the door before collecting the key yields an authored rejection. Collecting the key updates
state; the open door's visual and collision results agree. A declared system matching no objects is diagnosable.
Replace art, add a supported toon effect, rebuild/restart and export. **Finding:** a short vocabulary bridge is useful:
object data versus executed systems, prefab versus behavior, play session versus authored scene. Familiar editor terms
must not conceal different ownership, scheduling or material support. No forced tutorial or sample removal is needed.

### Z05 — Engine specialist: own wave model and painted boat game

Choose one small boat/buoy scene and derive the model's support, units, time and integration contract before building.
Create hull/body, input, camera and render content; bind the chosen physical surface provider and separately integrate
its visual representation. An ordinary numerical helper needs no plugin descriptor, but boat-contact queries have real
authority/lifetime obligations. Inspect a surface query, body response and render phase together; reject a visual-only
surface offered as physical support. **Finding:** specialist integration must expose exact public boundaries immediately.
A03 shader readiness remains relevant; a later river join or overlapping atmosphere invokes A04/A05. This is not
permission to discover the fluid architecture by layering visual experiments.

### Z06 — Small studio with existing tools: stylized courier game

Create a target/setup and a town fragment with cart, parcel and depot. Add movement and delivery rules with one motion
owner; do not install a full world/weather stack. Use the studio's authored assets and scripted imports, retaining
masters, import settings and game-owned overrides. Coordinate an artist export with a rule edit through explicit source
admission; a renamed axle attachment causes a mapping conflict. Inspect a delivery rejection and export via the same
operations used locally and in CI. **Finding:** scriptability and a small visual workbench complement each other; neither
requires replacing the studio's DCC tools or creating a parallel engine-specific project database.

### Z07 — Solo creator with offline needs: native mobile workshop

From empty source, add a bench, camera/materials, touch target and counter rule with readable feedback. Choose which
count/queue state persists, then define private test-save and player-save behavior. Add original art and verify that
the target package contains its required initial content, shaders and fonts. Missing warm-cache-only products must be
diagnosed. Reopen after an interruption using a complete checkpoint. **Finding:** installed offline play, offline cooking
and initial SDK/dependency acquisition are distinct claims. A no-network game need not imply an airgapped development
distribution. Native lifecycle, storage and device evidence cannot be supplied by desktop pointer emulation.

### Z08 — Artist/programmer pair: polished browser portfolio scene

Choose browser first and author an entry scene, lantern, camera, input/hit target and toggle state; add a controller only
if walking is needed. The artist shapes assets/light/material/display while the programmer supplies the rule. Separate
workbench-only helpers from the game view. Admit coordinated source changes, preserve overrides on reimport and expose
unsupported browser material/pass combinations. **Finding:** a source handoff must rebuild from declared masters/tools;
a recipient gets a playable package without authoring tools. Identify the packaged revision so a cached older demo is
not mistaken for a new change. A polished desktop capture does not qualify browser appearance.

### Z09 — Veteran simulation team: headless strategy before artwork

Create an initial board/topology, two unit records, explicit command input and one move rule. Choose fixed-tick/turn
policy within the engine's declared scheduling model; no unconstrained custom clock is implied. Run accepted/rejected
commands and inspect state in CI with zero GPU/window/shader initialization. Then attach meshes, camera, picking and
lighting to those same identities and rules. **Finding:** asset-first startup must not be a hidden prerequisite for world
execution. Later browser reduction cannot silently change unit count or game rules; explicitly choose a bounded scenario
if required. Scale and deterministic envelopes remain declared, unmeasured limits rather than architectural guarantees.

## Changes to carry into detailed design

| Decision or obligation | Existing owners / stage | What this pass does not resolve |
| --- | --- | --- |
| Guided empty creation and direct setup create the same ordinary source; optional examples | F2/F3/T1, D1; I2 authoring and I3 interaction | Exact files, public syntax and generation/override conflict rules |
| Draft validity is distinct from selected play/export readiness | F2/F3/F4/W1/T1/T2, A02/D1 | Source transactions, invalid external files, revision/recovery protocol |
| Workbench helpers and exported game objects are distinct | T1/P2/V2–V4, D1–D2/I3 | Exact helper storage, camera/view APIs and visual UI layout |
| Source registration can be traced to actual execution | F1/F2/F3/W1/P1/P6/T1, D1/I1–I3 | Debug instrumentation and bounded overhead; game reasons remain authored |
| Headless-first has its own sufficient entry requirements | F1/F2/W1/T2, D1/I1 | Exact command runner and turn policy API; no server/network promise |
| Source sharing, development export and qualified release are separate handoffs | F4/T2, D1 and relevant I4 slice | SDK delivery, packaging/hosting adapters and named-device support |
| Artist workflow includes an explicit behavior-authoring handoff | T1/P6 and game author, I3 | No-code/visual scripting is unselected; no automatic design generation |

Convenience may create named ordinary defaults, but must not silently choose a physical model, add a second state
owner or make a recipe's order override system scheduling. A callable game helper requires no generic extension
framework. A custom renderer/physics host embedding is not inferred from specialist needs; use the narrow supported
extension first and evaluate broader replacement only if a concrete requirement remains unmet.

## Evidence required later

Qualify a from-empty path independently of the starter path. Record every authored object, explicit setting, tool-only
helper, registered behavior, product prerequisite and undocumented intervention. Use the same small game for a newcomer
and veteran to expose guidance versus control problems, then exercise the art and headless paths separately.

G6's fresh agent, building a new small scene from public docs and the starter, is the first running from-empty check.
Headless, import and export paths follow as their tools ship. A development export needs its packaging slice, not
completion of every target or visual review gate.
Such an export reports its unqualified claims. Release qualification adds its actual content/device/lifecycle/quality
evidence. Test a clean development checkout separately from a nondeveloper recipient's packaged game; missing caches,
source masters and prerequisites must be visible. No elapsed-time or ease-of-use result is assigned by this paper pass.

Full traces: [newcomers](../evidence/handoffs/2026-11-01-engine-from-scratch/NEWCOMERS.md),
[veterans](../evidence/handoffs/2026-11-01-engine-from-scratch/VETERANS.md),
[circumstances](../evidence/handoffs/2026-11-01-engine-from-scratch/CIRCUMSTANCES.md).
