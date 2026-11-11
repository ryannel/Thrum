# Scenario walkthroughs and design simplification

1 November 2026. Ten analytical walkthroughs: seven contrasting games plus three developer circumstances. Three Sol
High agents wrote nine traces; the coordinator synthesized them and added the population/strategy stress case. These
are **paper uses of a proposed engine**, not running sample games, user testing, measured performance or implemented
commands. They test whether a developer can follow a coherent path and expose where the path still needs mechanisms.
The resulting recommendation is [ENGINE-EXPERIENCE](ENGINE-EXPERIENCE.md).

All cases begin with developer needs rather than a demand to exercise every module. They trace selection → authoring
→ behavior → play/inspect/stop → changed requirement → build → failure/recovery. New game concepts and numeric workloads
are illustrative, not additions to the engine's committed first-release scope. None requires pure 2D, MMO networking,
a runtime LLM or an arbitrary plugin ecosystem.

## Coverage at a glance

| Case | Circumstance / target | Useful engine selection | Work omitted / decision exposed |
| --- | --- | --- | --- |
| S01 Board tactics | Solo creator, short browser download, overhead camera | Picking, game rules, lit meshes, UI, cloud attenuation | No sky, weather simulation, world streaming or rigid dynamics; a static game setup is sufficient |
| S02 Workshop management | Native mobile, touch, offline saves, interruption | Input/actions, UI, bounded scene, durable rules/save, simple lighting | No outdoor stack; offline startup and pause/catch-up are explicit game decisions |
| S03 Apartment story | Writer/programmer, Mac/PC, first-person | Character collision, interactions/dialogue, lights/audio/subtitles | No terrain/water/meteorology; code/data iteration must be straightforward |
| S04 Coastal exploration | Third-person desktop showcase plus browser excerpt | Character/world, terrain, prescribed sea/river/pool, local interaction | No universal fluid solver; model joins and browser content selection need explicit meaning |
| S05 Flood management | Isometric physical gameplay, mobile visual reduction | One conservative shallow-flow domain, gates/drains, damage rules | No spectral sea or cloud volume; streaming cells cannot silently break physical conservation |
| S06 Painterly vista | Artist-led style, fixed light then sunrise/fog | Materials/style hooks, authored lighting, optional atmosphere | No full renderer replacement by default; changing sky needs real requested outputs and overlap rules |
| S07 Large-population strategy | Desktop planning game, many simple agents | Data-oriented game systems, selected navigation, batching, UI | No compulsory rigid body/skeletal animation per unit; scale is unqualified until bounded |
| S08 Programmer plus AI | Concurrent changes and branch checkout | Shared source/operations, coherent sync, pinned play/build | No automatic journal replay over a new branch or hidden collaborative database |
| S09 Artist plus programmer | Reuse character in native game/browser demo | One asset/prefab source, target cooks, target preview | No independent authoring fork per platform; missing required shader prevents target promotion |
| S10 Engine contributor | Add optional effect/model used by one game | Public typed integration, declared products, optional package | No universal plugin ABI, extra GPU owner or cost imposed on unrelated games |

## S01 — Overhead browser board tactics

The creator authors a board, tiles/tokens, win conditions, an overhead camera and a Sun/cloud-shadow field. Tile
adjacency and turn rules are ordinary game code; picking need not install a rigid-body simulation or navigation engine.
The engine validates the scene and light inputs, prepares the browser material/pass set, and starts an isolated play
session. Inspecting a token reveals its source, legal-move reason and active scene revision. Stop discards played turns.

The creator then adds a close perspective view. It shares the same world and shadow source, with its own picking,
LOD/history and camera policy. A later touch port adds a target/input mapping, not a duplicate rules implementation.
The browser build omits sky/cloud-volume/weather products. A missing masked-shadow variant fails target readiness
with the material/pass requirement shown; the prior complete build remains available. If the cloud shadow becomes a
game rule, it must be a declared world-time/turn input, not sampled render pixels. **Result:** the simple setup works
on paper; package lifecycle for the turn system buys nothing. A01–A03 still govern the unbuilt iteration/build path.

## S02 — Offline mobile workshop

The creator authors machines, placement sockets, recipe tables and a semantic touch UI in one room. Purchases and
production queues are durable game state; decorative motion is presentation. The game chooses pause-on-suspend or a
bounded offline progression rule explicitly. Initial content/fonts/pipelines must be available offline; optional skins
may load later under a declared fallback.

The developer tests with a session-local copy of a save, changes a recipe, inspects queue/currency state and stops
without altering a real player's slot. A browser demo reuses rules but needs its own storage/input/product policy.
On suspension during upload/save, completed checkpoints remain valid and stale GPU work cannot publish after resume;
storage exhaustion reports an unfinished save rather than success. These are designed obligations, not observed
device results. **Result:** save, input and lifecycle complexity earns its place; weather, streaming and live plugin
replacement do not. Thermal pressure may reduce presentation; it cannot secretly slow or alter authoritative rules.

## S03 — First-person apartment story

The team authors a room, collision, interactables, dialogue/subtitles, lights/audio and checkpoint data. A photograph
interaction changes a story flag and emits identified sound/UI events. A window can show an authored image with
declared light inputs; the room does not require a physical sky or outdoor environment family.

The writer edits text/placement, plays, inspects branch state, and stops. Explicit apply-back selects an authored
placement, never elapsed story progress. A code/schema edit rebuilds/restarts; a failed save migration preserves the
old test save. Adding an overhead capture camera does not move the audio listener or duplicate simulation. A branch
checkout while an editor change is pending stops source admission until sync/conflict resolution, while a pinned
player can remain on its old revision. **Result:** plain game code, validated references and source/play boundaries
provide value; a quest plugin framework, full atmosphere and native hot code loading are unnecessary for this case.

## S04 — Coastal exploration, desktop and browser

The creator places a player, shore, collectible, prescribed sea, authored river flow and still pool. Initial gameplay
needs wading/contact, not flooding or ocean mass accounting. Those selected quantities have explicit validity. The
collectible is a game-owned state/rule, not a reusable package requirement. Playing collects it only in session state.

Extending the river to the sea forces a meaningful choice: a visual-only seam, or a prescribed/open boundary in which
sea level drives the river and outflow has a named sink. Contact-query ownership is single-valued through the seam.
Adding a tide gate or upstream ponding exceeds prescribed-flow support and requires a different physical model;
the tool must not disguise that as a graphics-quality change. Dynamic falls/volumetric contact remain unsupported
unless explicitly designed; decorative falls can be a declared visual effect.

The browser excerpt cooks a bounded scene and cheaper visual products while preserving collision/collectible/contact
semantics. Missing collision blocks traversal; unsupported material/pass demand blocks the new build. **Result:**
different water models are useful; generic arbitrary model coupling and live replacement are not necessary to start.
A04 remains open until the domain-edge contract is reconciled with the water/play/content owners.

## S05 — Flood management with water as a rule

The creator authors elevations, inflows, gates, drains and threatened buildings. Water depth/flow and damage are
authoritative. Begin with one conservative shallow-flow domain, whose data may span many streamed cells. A gate rule
changes a declared boundary/operator at the world step; visual water draws the resulting field. The model must choose
valid resolution, stability and error bounds before implementation can claim flood behavior.

Adding a coastal outlet can use a prescribed tide boundary with recorded exchange into an external reservoir;
this does not claim that the ocean responds. Do not introduce a second solver merely because another streaming cell
loads. Cells required by the active physical domain stay available, or a specifically designed boundary/offscreen
policy governs them. An unloaded cell cannot silently become a dry wall or drain that changes the puzzle.

The mobile target lowers foam, reflection, shadow and mesh expense. If the original physical model cannot fit, the
target remains unqualified or the author chooses a distinct rules/model variant with save/outcome consequences.
Suspend saves field state and source clocks; it does not reconstruct physical water from a cosmetic texture.
**Result:** one coupled model is simpler where conservation is central; multiple models remain an option for genuinely
different needs. A graphics setting must never choose the physics model on the player's behalf.

## S06 — Painterly vista that later needs sunrise

The artist authors palette/material response, geometry, lights and a lantern interaction. A fixed HDRI/authored light
and simple air can serve the initial scene. Material/style hooks and declared render effects are tried before any
full pipeline replacement. One scene-linear/display convention still governs the result; style does not require each
object to apply another tone mapper. The artist plays, toggles the lantern, captures a named view, and stops without
changing its authored initial state.

A moving sunrise with aerial colour changes requires outputs the fixed image cannot provide. Select a dynamic
atmosphere model and declare its light/background/aerial outputs. Valley fog overlapping that atmosphere needs joint
integration of compatible coefficients, a joint preintegrating owner, or a declared approximate policy; independent
whole-ray pairs alone cannot promise correct overlap. Unsupported exact composition fails validation. A browser
version may select a fixed-time scene with baked appearance as a distinct authored excerpt; it must not claim the same
dynamic sunrise. **Result:** change the relevant model/output, not necessarily the renderer. A03/A05 and real visual
qualification remain obligations.

## S07 — Strategy with many simple agents

This coordinator-added stress case asks for thousands of simple units, as a workload to investigate rather than a
promised engine capacity. The developer authors unit records, selectable formations, terrain/path topology and game
rules. A unit can be a lightweight transform/game component with game-owned movement and selected navigation;
it need not become a dynamic rigid body, fully skinned character or per-object update callback. Physical units still
obey P3 authority; nonphysical movement must have one declared W1/game owner rather than bypassing a body solver.

Play exercises one move order, inspection explains path/blocked state, and a headless run checks the same rules.
Adding a close camera may increase visual/animation detail without changing unit identity or combat outcomes.
Demanding a mobile build requires a workload/cost comparison; lower visual quality cannot guarantee a CPU-heavy
simulation fits. Path request quotas and queues need a game-visible overload policy; stale paths cannot cross retired
terrain. **Result:** optional services and specialized stores are valuable; capacity is unresolved. This case tests
whether the public game surface permits efficient behavior without forcing a full character stack or claiming scale
from architecture diagrams. It does not select a crowd algorithm or add an MMO requirement.

## S08 — Programmer and AI change a project together

Each works on scoped code/source changes or through the same validated operation surface. A running player remains
pinned while code is rebuilt. Before accepting externally edited source, the developer declares the change set ready
and invokes sync/check. Snapshot admission requires that explicit completion boundary and validation; changed files
or branch identity during capture reject the candidate for retry. The tool reports schema/reference conflicts. File-watch
events only mark dirty state; silence is not proof that a checkout or multi-file save is complete.

When a pending tool edit meets another branch, preserve its base and patch but never replay it over different source
silently. One cooperating source writer or explicit external-edit phase is needed; uncontrolled concurrent writes
cannot be made atomic by revision labels alone. Tool recovery inspects current working files, retains the last admitted snapshot and reports a pending recovery
patch/conflict without applying it. A play restart reads only the selected admitted build. Play saves remain session-local. **Result:** files
plus explicit sync are the selected default. FOUNDATION-SPEC now supplies the exact file-application/admission
and crash recovery mechanism; implementation and behavioral qualification remain unrun. No database-first collaboration service is justified by this scenario.

## S09 — Artist and programmer reuse a character across targets

The artist delivers one character/skeleton/material/clip source package; the programmer defines its prefab, controller
and interactions. Target cooks derive different texture/geometry products while maintaining stable identities and
required skeleton/contact semantics. Native and browser preview each name the actual target, camera and revision.

Playing moves the body without rewriting its source pose. Applying an authored placement back produces a selected
diff; a concurrent prefab change causes a revision conflict. If the browser cannot render a required skin/material/pass
combination, report that dependency and withhold that build rather than silently showing a bind pose or missing
collision. The author may choose a compatible material or explicitly narrower content variant. **Result:** shared
authoring with real target validation is more valuable than separate platform content trees; A01/A03 enable that path.

## S10 — Contributor adds an optional effect or model

The contributor implements a public material/effect or environmental query contract, declares required inputs and
products, and supplies a bounded example. One game opts in; another does not. Ordinary modules and typed registration
are enough within a repository. A versioned package becomes useful when distributing/reusing the extension, but it
uses existing authority, build and readiness mechanisms instead of adding a plugin-specific world or GPU device.

If a browser target lacks a required depth/history input or shader variant, the opted-in game's check reports the
unsatisfied requirement. The unrelated game has no new recurring work/products; actual binary savings need a build
measurement. A changed gameplay model rebuilds/restarts unless migration was explicitly designed. **Result:** useful
extensibility survives simplifying the default setup; arbitrary runtime unload and full renderer replacement add
complexity without solving this contributor's task.

## What changed in the recommendation

| Pressure from the cases | Polished recommendation | Kept internally |
| --- | --- | --- |
| Simple rules were carrying package ceremony | Direct game components/systems in explicit static setup; packages optional | Schema, access, authority and dependency validation |
| Every capability could imply a search/lifecycle framework | Named chosen providers and finite target tiers; no general optimizer or default gameplay live removal | Validity, absent-feature behavior and controlled visual transitions |
| Preview/rebuild behavior was undefined | Isolated play process, private test saves, rebuild/restart baseline, selected apply-back | Session identity, source pin, cancellation, save/source separation |
| Source files and journal could compete | Canonical files, explicit coherent sync, guarded recovery patches | Revision/conflict checks, immutable snapshots and atomic accepted publication |
| Porting could silently change a game | Targets change eligible products/presentation; behavioral variants are explicit | Content/shader readiness, target support and provenance |
| Different water models implied a generic coupling layer | One domain where conservation is needed; explicit compatible edges where models differ | Queries, validity, state/time/flux ownership and declared sources/sinks |
| “Atmosphere quality” hid different outputs | Select required sky/light/media outputs and compatible representations | Single light ownership, support/integration rules and history invalidation |

These are reasoned recommendations, not implementation results. A01–A05 remain tracked until the affected contracts
and mechanisms are reconciled. J01/J02/J04 now have a concrete recommended baseline rather than an unbounded choice
list. J03's standalone-library evaluation remains detailed design; no library was adopted by these walkthroughs.

## Next genuine dogfood

The first running dogfood is G6 in the [build plan](../plans/BRIEF.md#active-gate-order-and-budgets): a fresh agent
with only public docs builds a new small scene and logs its friction. After that, use S01 for the smallest complete create/change/play/build path, S05 to expose
gameplay-versus-visual quality mistakes, and S09 to expose source/target/asset/shader problems. Use S06 when alternative
media integration is implemented. Those are complementary running checks, not permission to discover missing
architectural mechanisms by trial. Record task completion, unexpected concepts, actionable errors, restart latency,
output quality and real target costs. Fresh-agent runs and, later, people using the engine establish usability; these
paper traces cannot establish that it feels simple or produces beautiful results.

Full agent traces: [small games](../evidence/handoffs/2026-11-01-engine-dogfood/SMALL-GAMES.md),
[model-heavy games](../evidence/handoffs/2026-11-01-engine-dogfood/MODEL-GAMES.md),
[developer circumstances](../evidence/handoffs/2026-11-01-engine-dogfood/DEVELOPER-LOOP.md).

Independent follow-up readers challenged the synthesized developer loop, model cases and revised composition/tooling
recommendation. Corrections made snapshot admission conditional, prevented recovery wording from implying cross-branch
overwrite, separated process isolation from save/side-effect enforcement, and scoped package/removal/recovery rules.
Documentation links, anchors, whitespace, skill structure and ledger identities were checked. These are document
checks and analytical reviews, not new runtime or observed usability evidence.
