# First-use walkthroughs

1 November 2026. Three fresh Sol High agents traced nine first-use journeys across game programming, external code
and art. The coordinator reconciled them into the recommendations below and [Building a game](ENGINE-EXPERIENCE.md).
This extends the [ten earlier scenarios](SCENARIO-WALKTHROUGHS.md), focusing on the steps those stories skipped.
These are paper walkthroughs of a proposed engine, not implemented commands, observed usability or measured quality.
No new API syntax, library adoption, platform certification or full editor is claimed. Audit A01–A05 remain open.

## What the beginner knows

The programming path assumes someone who can edit/build code but is new to this engine. A familiar Rust developer
and a programmer learning Rust face different prerequisites; language learning must be identified, not hidden inside
an engine tutorial. Artists need visual import/preview/edit controls even when a programmer handles game behavior.
AI assistance is optional and uses the same project/operations; it does not excuse an incomprehensible manual path.

The first task is to make and recognize a meaningful change. The developer should be able to find the source of an
object, change a rule, understand a rejection and export one supported target before reading the architecture manual.
Specialist integration introduces deeper contracts when needed. This is progressive documentation and API design,
not permission to conceal semantic choices or defer foundational mechanisms.

## Nine journeys

### U01 — Browser tactics: change a rule before learning the engine

Choose an overhead board starter and a named browser target; check its compiler/content-tool prerequisites and actual
support status. Run its existing selection/move interaction. Follow the selected piece to its scene record and the
move action to its rule. Add a push attack: validate board adjacency, commit game-owned piece/turn state, and animate
the accepted result. Picking and board topology do not require a rigid-body simulation or navigation service.

Change one rule, admit the source revision, rebuild/restart, inspect a rejected move, stop and export. Omit water,
sky, weather, streaming and skeletal animation. If an outlined piece needs an unavailable browser material/pass
variant, name the piece, material, pass and target; offer an explicit supported material choice. Do not silently drop
the outline or present an old build as new. **Needed:** one complete action → rule → state → feedback example, a small
default selection and target preflight. Cloud shadows are a later choice, not a mandatory board prerequisite.

### U02 — Third-person exploration: add a lantern-operated bridge

Start with a small character scene whose movement, camera, input and collision already agree. A placeholder character
need not require a complex animation graph. Add an interaction that sets game-owned bridge state; the collision change
and visible opening follow that committed state through their owners. The bridge cannot become traversable solely
because its mesh moved. A quest framework, inventory, weather and ocean add no value to this rule.

Play, inspect the bridge state, stop, then move the lantern in source. Save bridge progress only if explicitly durable;
applying a selected lantern placement back must not copy the played quest state. Add a browser excerpt later and
validate its skin/material/collision requirements separately. **Needed:** source navigation for the example, an explicit
motion policy and separate labels for authored placement, session progress and saved game state.

### U03 — Mobile block puzzle: drag preview is not a committed move

Start with a room, touch actions, camera/picking, semantic buttons and grid rules. Dragging displays a candidate;
releasing submits a placement for occupancy/orientation validation. Accept or reject through one rule path. Game undo
reverses a puzzle action; editor undo reverses an authored edit. Dynamic rigid bodies, terrain and outdoor features
remain absent. A native target needs its own prerequisites and qualification; desktop pointer emulation is limited.

Try interruption/resume using a private test save, then revise a block shape. If an old saved placement is incompatible,
show the source/save mismatch and preserve the old save; choose a supported migration or a fresh test save. Visual
quality cannot alter placement rules. **Needed:** distinct drag/source/play/save status, explicit pause/checkpoint policy
and a target-specific delivery path. Store signing/submission is a separate deliberate step after local export.

### U04 — Bring a board AI library: ordinary code should stay ordinary

From U01, add a source dependency and call the search function from a game system using a supplied board snapshot.
Register the game's data and system access/order where required; library-internal search nodes need no engine schema.
A pure helper used inside an existing system needs no separate engine registration. No provider or plugin package
exists merely because a library was imported. Replay needs an explicit treatment of any nondeterministic decisions.

Run one accepted and one rejected action. A missing piece field names the object and consumer; an unsupported Wasm
dependency names the library and target. Remove/rewrite the call to uninstall it, preserving any still-required game
data/readers. **Needed:** a direct dependency-to-game-code example and no packaging ceremony for local behavior.
Engine-owned effects and asynchronous work still use their public authority/lifecycle boundaries.

### U05 — Bring navigation or physics: a real adapter has a real contract

Begin from a working scene and replace one selected implementation. A board graph search may remain ordinary game
code; replacing the engine navigation service requires mapping path requests, content, agent classes, generations,
errors and lifetime. Replacing physics additionally maps solved motion, queries/contacts and durable state. Keep one
motion owner; a navigation adapter returns guidance rather than writing body transforms. Stop/rebuild/restart is the
baseline, with explicit save compatibility. No binary plugin ABI or live solver swap is needed.

Use a bounded navigation fixture first: two cells, one agent, one portal and a changed obstacle/cell that makes a
pending path stale. Check unavailable/stale results, not identical route quality across algorithms. Remove the binding
while a path-seeking actor remains: name that actor and the missing service, then rebind/restart. Unsupported joints,
agent classes or targets must fail before use. **Needed:** a concise public contract, reference adapter/consumer and
checks for claimed capabilities. Built-ins and outsiders use the same boundary for those claims. This is specialist
work after the relevant contracts, not a prerequisite for making the first board game.

### U06 — Bring water or atmosphere: choose the output before the model

A custom water material can be a rendering extension; buoyancy/contact queries require a physical surface provider.
A sky image, environment illumination, direct sunlight attenuation and aerial media are separate outputs. Start with
one domain and known consumers, not an adapter claiming every fluid or atmosphere regime. Declare support, time,
units, products and targets; inspect a point/query or ray with its supplying model.

A river/sea connection needs a declared visual, prescribed or conservative boundary. Unsupported falling water cannot
invent a surface-height answer. Overlapping fog and atmosphere require compatible integration inputs or an explicit
approximation, not two opaque whole-ray effects assumed exact. Browser mismatch names the unsupported input/product;
removing the model identifies boats, media or saves that still require it. **Needed:** output-oriented documentation
and exact model support. A03–A05 block publishing these as supported integration recipes until mechanisms reconcile.

### U07 — Realistic room: import should lead to a legible scene

Import room geometry/materials and review scale, orientation, textures and unsupported data. Imported cameras/lights
must be identified as accepted scene data or retained source data; an interchange file is not automatically a playable
room with collision. Choose an editable interior lighting/camera/display recipe, frame the room and inspect actual
target rendering. Add a simple walk/interaction path when required; keep outdoor modules absent.

Adjust lighting/materials and compare the asset in a neutral view. Reimport a changed mesh/texture with separate
imported values and game-authored overrides. Missing or ambiguous node correspondence requires repair before accepting
affected references. An optional missing texture may use a declared fallback; required collision or material support
cannot disappear. **Needed:** import review, stable mapping/conflicts and a small visual workbench. These prioritize
existing F4/W1/T1/R7 obligations rather than adding a second editor asset database.

### U08 — Toon character: replacing the sample must be a supported task

Import the creator's mesh, skeleton, material slots and clips; expose scale, root-motion interpretation, attachments and
required mappings. Replace the starter character while preserving explicit controller/collider and animation ownership.
Preview at gameplay distance as well as close up. Toon style combines assets, material response, lights, shadows and
display; outlines/transparency/skinned shadows have explicit supported passes.

Reimport a changed skeleton or renamed clip. Identify affected attachment, animation or prefab references rather than
silently producing a bind pose or dropping an accessory. A missing browser skinned-shadow variant needs a material/pass
repair or an explicit narrower target. **Needed:** a sample-replacement guide, visible import-to-prefab mappings and
reversible style edits. One master asset can have target products; it need not become two independent authored trees.

### U09 — Painterly overhead game: no sky should be an understandable choice

Start with a board/character interaction, painted assets, a deliberate camera and an editable lighting/style recipe.
Choose background, direct light, fill and optional cloud attenuation separately. Omitting visible sky/clouds must not
require debugging implicit lighting dependencies. Change palette/surface response and shadow settings through the
same source operations, holding camera/exposure when comparing appearance.

Inspect the source of illumination and attenuation. Removing a retained attenuation provider reports the consumer;
choose a constant visibility or remove the effect explicitly. Adding dynamic sunrise/fog introduces the corresponding
light/media choices and A05 constraints. **Needed:** a no-sky example and light/material provenance. A painterly result
may require different geometry, textures or effects; a universal style dropdown is not the design.

## Reconciled changes and owners

| Pressure exposed | Recommendation | Owning design / timing |
| --- | --- | --- |
| First project starts after unexplained setup | One target preflight, release-matched docs/examples, named baseline and a complete interaction | T2/F1/F2, D0–D1 definition; first public starter |
| Too many concepts and files for a rule | One effective setup; direct code path; co-locate or derive mechanical declarations with inspectable expansion | F2/F3/W1/T1, D1; no hidden scan determines order |
| Demo works but cannot become the user's game | Replace/delete all sample content and rule using the same setup/build path | T1/T2 and play owners, I3 starter qualification |
| CLI completeness is mistaken for artist usability | Minimal target-aware visual workbench; F3/F4 stable import mapping, T1 override/conflict operations | F3/F4/W1/T1/V3/V4; I2 import/operations, I3 visual path |
| Extensibility sounds equally easy at every level | Ordinary library → game system → specialist adapter; narrow examples and claimed support | F2 and service owners; D1 boundaries, fixtures as service ships |
| Style sounds like a renderer toggle | Editable content/material/light/display recipes with supported passes and explicit mappings | F4/V2–V4/T1, D2 semantics and I3 visual examples |
| Generic validation errors strand beginners | Object/action first, source link, cause, active revision and deliberate repair options | F2/T1/T2 plus producer; first starter, A01–A03 mechanisms |
| Portability and removal hide consequences | Target-specific compile/content/device status; dependent content/save diagnosis before change | F2/F4/P7/T2; first applicable target/feature, no silent migration |

These refine priorities and public affordances. They do not add a mandatory marketplace, universal package manager,
scripting VM, live binary replacement, arbitrary renderer swap or full GUI editor. Do not require every specialist
example before a small starter can ship. Equally, do not advertise an art workflow or target before its actual path is
usable. A minimum workbench is a constrained client of existing operations, not another source of project truth.

## Definition of a credible first-use path

G6 in the [build plan](../plans/BRIEF.md#active-gate-order-and-budgets) runs the first slice of this path with a fresh
agent and public docs. As targets and tools ship, qualify one complete path before multiplying templates:

1. On a clean supported host, discover prerequisites and run the named starter using version-matched instructions.
2. Find the source of an object and action. Change appearance and one rule through public documented surfaces.
3. Inspect accepted/rejected actions; distinguish authored state, private play progress and any durable test save.
4. Replace the sample scene and remove its example rule. Build the user's content without sample-only helpers.
5. Recover from one realistic dependency/target failure without guessing or losing the accepted work.
6. Stop/restart after code changes and export one named target; confirm exactly which revision and products ran.

An extension path separately adds a plain library or optional feature. The art path imports, visually adjusts and
reimports an asset without silently losing overrides; the specialist path changes one provider while retaining shared
authority. Explicitly retained private test saves may survive a restart; unsaved play state is discarded. Their
retention policy remains A01 work. Record steps needing undocumented knowledge,
user-edited files, error recovery, task completion and actual cold/warm edit-to-play latency when tools exist. No
arbitrary success time or usability score is assigned now. Clean target exports, device tests and
[agent visual review](AI-DEVELOPMENT.md#visual-review-and-human-steering) remain distinct evidence. These checks validate reasoned workflows; they do not substitute for missing architecture.

Full traces: [games](../evidence/handoffs/2026-11-01-engine-first-use/GAMES.md),
[extensions](../evidence/handoffs/2026-11-01-engine-first-use/EXTENSIONS.md),
[art](../evidence/handoffs/2026-11-01-engine-first-use/ART.md).
