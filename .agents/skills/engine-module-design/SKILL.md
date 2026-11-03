---
name: engine-module-design
description: Research, design and prove new or substantially redesigned game-engine modules against requirements and shared contracts.
---

# Engine Module Design

Design the complete required module, then build and prove it in increments governed by that design. Select foundational
representations against all required behaviours and external contracts, not the first attractive demonstration.

## Start at the right scope

Use the engine architecture brief as the module's commission: game requirements, responsibility, neighbouring systems,
budget, consequential interactions and design maturity sought. If these are unresolved, clarify the relevant boundary
using [engine-architecture](../engine-architecture/SKILL.md). A bounded module task needs only its relevant engine context;
an ordinary bug fix does not require redesigning the engine. Discovery and research alone do not commission a build.

For reusable engines, design against supported capability profiles and combinations. Separate optional presentation
from services other consumers retain. Specify dependencies, provider substitutions, disabled cost and configuration
lifecycle; a feature flag that hides output while retaining all work is not a complete modularity contract.
When a module is extensible or pluggable, define its developer-facing registration, component/system contribution,
compatibility and removal contract. Show how a game uses or replaces it without importing private implementation or
creating another state authority; inherit the engine's composition model rather than inventing a local plugin framework.

Choose the target independently of existing implementation, using research, production practice and domain knowledge.
Then reconcile existing code with that target. Previously encountered code is not a reason to claim blind independence.

## Load the relevant workflow

- **Research/design:** [blue-sky research](references/blue-sky.md); use [review](references/review.md) to challenge the proposal.
- **Compare or replace existing code:** [reconciliation](references/reconciliation.md), using the selected target.
- **Prototype/build:** [implementation](references/implementation.md), carrying the design into a complete interaction.
- **Representation or integration faults:** [system diagnosis](references/system-diagnosis.md).
- **Cost/scaling decisions, runtime budgets, quality tiers or optimization:** [performance](references/performance.md), during design and implementation.

Read only relevant references. Research alone does not commission implementation. When several stages are commissioned,
continue through running, inspection and correction without adding approval stops.

## Keep one governing design

Maintain a canonical module design linking requirements, selected mechanisms, external contracts, decisions and proof
status. Itemise required capabilities and interactions before choosing foundational representations; distinguish user
requirements and necessary implications from assumptions, options and exclusions. Keep decision status (proposed,
provisional, selected, superseded) separate from implementation and validation. Experiments and evidence link back to
this design rather than becoming competing targets.

Select a complete operating model, including content and authoring. Explain how its drivers, state and operators
produce each defining behaviour. Complete gaps in publications with explicit, testable deductions; a capability label
or governing equation is not a missing producer. Give shared causes clear owners and consumers appropriate fidelity.
Cohesion does not require a universal solver. Use mathematical and systems knowledge to derive representations,
coupling, approximation regimes, invariants and cost/error bounds before building. Work through representative
interactions across the full required scope without inventing missing core mechanisms or interfaces. An experiment
must test a stated prediction or an uncertainty analysis cannot resolve; it must not substitute for a missing design.
Keep such gaps explicitly unfinished. Numerical/perceptual/hardware qualification may remain unrun after a reasoned
design is complete; analytical confidence does not turn those measurements into passed evidence.

For a substantial build-ready design, walk representative developer and content journeys from the saved plan. Resolve
consequential choices the implementer would otherwise have to invent; keep unrun qualification visible.

Include a cost account when selecting the module's representation: workload multipliers, repeated work/data movement,
working set, critical path, alternatives and verification. Investigate frequent cheap operations as well as expensive
ones. AI can absorb substantial internal complexity when specialization is worthwhile and checkable; hand-editing
convenience alone is not a selection rule. Use [performance](references/performance.md) for the relevant design or
measurement stage; a new module does not need an existing implementation to justify its cost-conscious design.

Derive the proof/build queue from architectural risk and dependency. Each slice names the requirement or decision it
tests, affected contracts and rejection conditions. Prove the least-established consequential coupling early. Reduce
content count or world extent without removing the interaction that could invalidate the model. Successful local
experiments qualify only what they exercise; they do not automatically promote their architecture.

When evidence contradicts a structural assumption or a new requirement changes representation needs, revise the
governing design and affected engine contracts before extending dependent code. Do not preserve a failing foundation
through successive local patches. Use independent capability discovery and design challenge when available and
authorized; [review](references/review.md) describes the scope. Preserve user acceptance as distinct from engineering proof.

Keep research support, implementation correctness, experienced quality and runtime viability distinct. An honestly
labelled failure remains unfinished. Scale investigation to decisions it can change. Correct the responsible model
or contract; retain qualified parts, but give sunk effort and small diffs no preference over a sound replacement.

For a coordinated module programme, follow the project's [design programme](../../../docs/design/DESIGN-PROGRAMME.md):
return contract changes and unresolved dependencies to the architecture owner, bound any child study by a decision it
can change, and report the achieved evidence level. Do not claim another module's conclusion as this module's proof.
