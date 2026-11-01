# Thrum engine design

The design programme for a flexible 3D engine on Mac, PC, browser and mobile app stores. Bevy is a research/code
reference, not the host. Existing implementations are candidates for later reconciliation, not the target.

- **Start here: [building a game](ENGINE-EXPERIENCE.md)** — project, world, target and run.
- [Brief and conversation coverage](ARCHITECTURE-BRIEF.md): E01–E20 requirements, scope and retained ambitions.
- [Integrated design](SYSTEM-DESIGN.md), [composition](COMPOSITION.md) and [selected decisions](ARCHITECTURE-DECISIONS.md): core and 30 owners.
- [C4 contracts](CONTRACTS.md): shared invariants; modules link consequences rather than duplicate protocols.
- [Baseline](BASELINE-SPEC.md): initial scope, devices and workload admission; [construction order](CONSTRUCTION-PLAN.md).
- Owning mechanisms: [foundation](FOUNDATION-SPEC.md), [render](RENDER-SPEC.md), [play/environment](PLAY-ENVIRONMENT-SPEC.md), [network](NETWORK-SPEC.md).
- [I1 headless slice](I1-HEADLESS.md): selected action/world/replica mechanism, cost account and [local evidence](../evidence/i1-headless/QUALIFICATION.md).
- [Maturity](MATURITY.md) and [ledger](maturity.json): current design versus implementation, quality and platform evidence.
- [Simplification findings and resolutions](SIMPLIFICATION-REVIEW.md): coordinated closure; [earlier review](PLAN-REVIEW.md).
- Creator acceptance: [scenarios](SCENARIO-WALKTHROUGHS.md), [first use](FIRST-USE-WALKTHROUGHS.md), [from scratch](FROM-SCRATCH-WALKTHROUGHS.md).
- [AI development](AI-DEVELOPMENT.md), [skill rationale](SKILL-WORKFLOW.md) and [programme](DESIGN-PROGRAMME.md): focused skills, checks and coordination.
- [Performance investment map](AI-PERFORMANCE.md#capability-investment-map), [design/process policy](AI-PERFORMANCE.md#performance-investment-from-the-first-design) and [campaign walkthroughs](PERFORMANCE-WALKTHROUGHS.md): deep optimization throughout construction, with trusted whole-engine evaluation.
- [Storage lifecycle](STORAGE-LIFECYCLE.md) and [journeys](LIFECYCLE-WALKTHROUGHS.md): assets, promotion, recovery and cleanup.
- Research: [modern engines](ENGINE-STRUCTURE-RESEARCH.md), [capability survey](CAPABILITY-SURVEY.md), [decision audit](ARCHITECTURE-AUDIT.md).
- [Earlier host assessment](HOST-AND-PLATFORMS.md) is historical; Bevy is not the selected host.

Module designs live in `modules/`; source/decision investigations in `research/`. The integrated design and maturity
ledger link each required responsibility to its owner, evidence, gaps and next qualification. Raw trials/captures stay
in `tmp/`; this directory contains durable decisions and compact evidence. No design status implies runtime proof.

Current repository commands and the next implementation slice are in [the build plan](../BUILD-PLAN.md).
Imported reviews retain their original scope; [provenance](../provenance/IMPORT.md) explains relocated evidence and historical hashes.
