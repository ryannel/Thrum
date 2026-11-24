# Engine architecture

What project-level planning (for the whole engine) needs to know about Thrum.

## Where the architecture lives

| What | Where |
|---|---|
| Goal, the V1 exit and its milestones | `docs/plans/VISION.md` |
| The module list and build order | `docs/plans/ARCHITECTURE.md` |
| Requirements coverage (E01–E21) | `docs/design/ARCHITECTURE-BRIEF.md` |
| Shared invariants between modules | `docs/design/CONTRACTS.md` |
| Legacy module brief, where retained | `docs/plans/<MODULE>/BRIEF.md` |

Where an older document in `docs/design/` disagrees with these, these win. Update a record in place, rather than
writing a competing version.

## A reusable engine, not one game

Thrum serves a range of games, so design from contrasting usage profiles rather than one game's needs: the V1 test
game, the kit scenes, the planned swarm game (`docs/design/NEXT-GOAL-SWARM-GAME.md`), and web demos of skies and
water. Keep
three things apart for each capability:

- **What a game needs.** A game may need a phenomenon's effects (shadows from clouds) without drawing the phenomenon.
- **How it is presented.**
- **How it is implemented.**

Profiles select capabilities, providers and quality. They don't create separate copies of shared world state.
Disabling a feature removes its work and resources while keeping what other consumers rely on. A flag that hides
output while the work continues is not modularity. Say which combinations are supported; modularity doesn't make
every combination valid.

A game composes the engine through public extension points: selecting features, registering behaviour, replacing
implementations. A folder boundary alone is not an extension point.

## Owners and interactions

Keep two maps: a module map, and an interaction map. A tree alone hides feedback loops and shared services. For each
consequential boundary, name:
- the state's owner;
- its producers and consumers;
- what they exchange;
- when it updates;
- how consistent it must be.

Walk representative play sequences across modules, including transitions, to find missing owners and duplicate
authorities before writing detailed APIs.

**Cross-cutting mechanisms need owners too, not only world state.** The 14 Nov 2026 code review traced most of the
codebase's inconsistency to mechanisms that no module owned, so each host or crate built its own:
- the device request and loss handling;
- the platform host loop;
- pause and overload policy;
- GPU test support;
- capture and readback;
- shader composition.

When a module needs one of these, name the module that owns it.

## Cost across the whole scene

Allocate time, memory, loading and content budgets across the whole scene where targets are known, and keep
estimates apart from measurements. Individually affordable modules don't make a viable engine. The account has to
include the systems that run at the same time, background work and quality transitions. Follow the owner's
performance policy: decide for performance up front, then build with tripwires.

## Proof order

Order proofs by the architectural assumption each can disprove. Each proof names:
- the decision it tests;
- a realistic workload;
- the observation that would reject it;
- what changes if it fails.

A demonstration that happens to exist doesn't select the architecture.

## Module planning inputs

Record these inputs for each module:
- its requirements and consumers;
- its contracts with neighbours;
- its budget;
- its key interactions;
- the decisions still open;
- how deep its research should go;
- when it is complete.

Include the creator's path from source to screen when that could change the module's boundary. The architecture
owner keeps the shared contracts and the whole-scene budget, even when modules are designed independently. When a
module's research finds a missing owner or contract, settle it across the affected consumers before that design is
treated as settled.

The owner decides: changes to the purpose or the V1 exit, spending, publishing, licensing or deploying, and
irreversible actions. Everything else at this level is the planner's, including pushing verified checkpoints to
`main`.
