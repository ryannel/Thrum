---
name: engine-architecture
description: Design whole-engine capabilities, module boundaries and shared contracts for one game or reusable game families.
---

# Engine Architecture

Design the engine around its intended users, supported games and the work needed to create them. Establish the whole
system before commissioning deep module designs. Module boundaries are hypotheses to improve through research and
proof, not a copy of the existing folders or a list of fashionable engine features.

## Establish the engine brief

First distinguish a single-game engine from a reusable engine or module library. For a reusable engine, establish its
intended range of games, capability combinations, quality ambitions, developer experience and delivery goals. Derive
contrasting usage profiles to expose shared needs and legitimate differences; do not force the user to select one
game or narrative moment. For a game-specific engine, representative player experiences can establish the brief.
Discuss unresolved choices in small rounds and keep the brief provisional; do not invent answers to finish it.

Distinguish confirmed requirements, necessary implications, proposals, open questions and exclusions. Use existing
product decisions as context, with their dates and scope; do not silently promote old experiments or revoke accepted
choices. For a fresh design, derive responsibilities before inspecting old technical plans or implementation. Prior
exposure is a limitation to disclose, not a reason to claim a blind study.

Discover requirements across the whole game: player control and camera, world scale and continuity, interactions,
characters and world life, rules and progression, presentation and sound, persistence, content creation and delivery.
These are prompts for discovery, not mandatory modules. Ask about platforms, devices, input and accessibility,
networking, authoring workload and performance where they change architectural decisions. Keep unknowns explicit.

For reusable engines, distinguish a capability from its visual presentation and from its implementation. A consumer
may need a phenomenon's effects without drawing the phenomenon itself. Profiles select capabilities, providers and
quality; they do not create independent copies of shared world state. Define supported combinations, dependencies,
fallbacks and unsupported configurations. Disabling a feature should remove its avoidable work and resources while
preserving retained consumers. Separate build-time inclusion, load-time configuration and live toggles where their
cost, state migration or lifecycle differs. Modularity does not imply every arbitrary combination is valid.

Make developer composition explicit: how games select features, assemble entities from data, add behaviour and replace
implementations through public extension points. Distinguish responsibility modules, plugin packages, components,
systems and providers where those concepts apply. Folder boundaries alone do not establish pluggability; account for
registration, compatibility, content requirements and removal without bypassing shared authorities.

## Derive responsibilities and interactions

Maintain one architecture brief in the project's durable design area. Link product decisions to their canonical
record rather than maintaining competing versions. Give important requirements stable references when useful.

Map required experiences and usage profiles to responsibilities, then propose modules around coherent ownership and change boundaries. Maintain
both a module map and an interaction map: a tree alone hides feedback and shared services. Distinguish reusable
engine capabilities, game-specific policies and authored content. Identify what the chosen host engine already
provides once the experience requirements are established; building a custom replacement needs a reason.

For consequential boundaries, identify state owners, producers and consumers, exchanged quantities/events, lifecycle
and scheduling, persistence and acceptable consistency. Walk representative play sequences across systems, including
important transitions. Find missing owners, duplicate authorities and circular assumptions before detailed APIs.
Choose contract detail proportional to its downstream consequences; avoid empty frameworks and speculative APIs.

For performance-led engines, map investment to expensive operations, frequent small work, data movement and waits
across runtime and creator workflows. Derive scaling and critical-path consequences before committing representations.
AI-maintained specialization may justify harder internals when semantics, invariants and gains are verifiable; public
simplicity does not require uniform internal layouts. Sequence capability investments through construction rather
than postponing them to a final optimization phase. Keep prospective advantage separate from measured evidence.

Allocate whole-scene time, memory, loading and content budgets where targets are known. Distinguish estimates from
measurements. Include simultaneous systems, background work and quality transitions; individually affordable
modules do not establish a viable engine. Unknown hardware remains an open constraint, not invented certification.

## Develop and maintain the architecture

Research decisions that could change system boundaries or feasibility using primary production and academic sources.
Compare credible architectural alternatives, including a simpler composition, rather than only mapping engine feature
lists to modules or rejecting extremes. Distinguish evidence that a candidate is internally coherent from evidence
for choosing it. Include developer creation/change workflows and heterogeneous model boundaries where they can expose
choices that ordinary play/frame scenarios miss; do not turn every surveyed engine capability into a requirement.
When scenario dogfooding is commissioned, walk concrete creator actions from initial setup through changed requirements,
iteration, delivery and a realistic failure. Compare what a simple and a demanding game actually need; use the traces
to remove unnecessary developer concepts and machinery while retaining internal correctness. Label paper walkthroughs
separately from running tools, observed usability and measured quality; a plausible story does not close a missing mechanism.
For ambitious quality or innovation goals, distinguish established production methods, emerging research and original
hypotheses. Derive consequential choices from domain models, mathematical relationships, system invariants and known
constraints before proposing experiments. Explain assumptions, approximations, coupled behaviour, complexity and
error/cost bounds where they determine the architecture. A missing core mechanism is a design gap; a prototype plan
does not close it. Compare complete results with relevant current references; novelty alone is not quality.
Use measurements to test explicit predictions or uncertainties analysis cannot settle, not to discover the architecture
by layering attractive results. Public demos need representative running
results and reproducible measurements; upstream contributions also need clear integration boundaries and licensing.
Delegate detailed technique selection to module design once its responsibility and neighbouring contracts are clear.
Keep selection status (proposed, provisional, selected, superseded) separate from implementation and proof status.

For each module commission, supply the requirement set, external contracts, budget, key interactions, open decisions,
research depth and completion criteria. Include the relevant creator journey and content source when either could
change the module boundary or its proof. Use [engine-module-design](../engine-module-design/SKILL.md) for research,
synthesis and proof. Child research may expose a missing owner or contract; integrate that finding across affected
consumers before treating the module design as settled. The architecture owner retains the shared contract and
whole-scene budget, even when module studies run independently. For this project's coordinated research programme,
use the [design programme](../../../docs/design/DESIGN-PROGRAMME.md).

Order proofs by the architectural assumptions they can disprove. Each proof names the decision, realistic workload,
observable rejection condition and consequences of failure. A demonstration does not select the engine architecture
by existing. Reconcile old implementations only after choosing the target; retain what fits without favouring sunk
effort. Revise the target when evidence or the user's requirements change.

An exploratory brief is complete for its stage when confirmed needs and consequential questions are clear. An
architecture is ready to commission modules when required experiences have owners, important interactions and shared
budgets are accounted for, and consequential structural choices have reasoned mechanisms. Remaining empirical
uncertainties have bounded qualification; unresolved core mechanisms remain design gaps. It need not preselect every algorithm.
Record unqualified combinations and deferred decisions; a completed design phase does not certify implementation,
performance or visual quality. Continue through the commissioned stage without an invented approval pause.
