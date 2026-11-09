# AI-first engine and game development

1 November 2026. Recommended development product for the custom engine. Three Sol High agents examined enforcement,
skills/setup and concrete contribution workflows; the coordinator reconciled them with the existing T1/T2 design.
This is a design, not an installed SDK, implemented agent API, new Rust lint configuration or observed workflow result.
The historical Broadsides checks below are precedents only. Current Thrum rules and implemented checks live in [CONTRIBUTING](../../CONTRIBUTING.md). A01–A05 now have selected architectural resolutions in [the decision record](ARCHITECTURE-DECISIONS.md); their detailed implementations remain unqualified.

The objective is reliable contribution: an agent can discover the real public surface, make a bounded change, run the
exact changed revision, inspect the outcome and supply evidence another person can reproduce. Humans use the same
source, operations and checks. No runtime model service or AI subscription is required by the engine architecture.

## Two users of the development product

| User | Context they need | Completion evidence |
| --- | --- | --- |
| Engine contributor | Governing design, public contract, state owner, relevant producers/consumers, implementation entry points and scoped checks | Implemented behavior plus affected consumer checks; contract changes reconciled; exact evidence and remaining limitations |
| Game creator | Selected SDK version, public APIs, game-owned rules/content, selected capabilities/targets and applicable examples | The requested game interaction or content change works on the named target, with inspectable source and reproducible results |

A game creator should not need the engine's internal module-design programme to add a door. An engine contributor
changing publication or body authority must understand its consumers. Both need small relevant context, real command
help and dependable checks; neither benefits from a different agent-only mutation authority.

## What belongs in instructions, and what belongs in tools

| Layer | Responsibility | Example |
| --- | --- | --- |
| Short scoped `AGENTS.md` | Project identity, relevant paths, local commands, working constraints and links | Where game code goes; which design owns a boundary; preserve unrelated work |
| Task skill | Non-obvious judgment, workflow routing and the evidence needed for this kind of task | Decide whether a bug is local or exposes a cross-module contract error |
| Types and public APIs | Make invalid access difficult or impossible through supported interfaces | Read-only presentation snapshots; opaque resource handles; owner-controlled mutation |
| Formatter/linter/dependency checks | Repeatable mechanical conventions over a defined source/configuration scope | Public import boundaries; forbidden platform calls in authoritative code |
| Schema, setup and schedule validators | Meaning across records and selected components | Missing provider, conflicting authority, incompatible units, invalid schedule or shader demand |
| Tests, replay, captures and profiling | Check behavior and empirical limits of the actual build | Rejected stale result; saved state restored; motion looks coherent; named-device frame cost |

Instructions guide decisions; the executable layer checks rules. Deterministic checking means the same pinned inputs
and tool configuration produce the same rule result. It does not mean model decisions, arbitrary game code, floating
point across platforms or rendered pixels are thereby deterministic. State the exact scope of each claim.

OpenAI's [Astra guidance](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra) supports
concise, discriminating skills, progressive detail and explicit completion boundaries rather than blanket reading or
long prescribed itineraries. [Skill discovery](https://learn.chatgpt.com/docs/build-skills) uses names/descriptions
before loading full instructions. This supports a few useful task routes, not a skill for each engine subsystem.

## Historical precedents and their limits

The original Broadsides TypeScript game had checks in [package.json](../provenance/CONTEXT.md#source-7ae45ad102ea),
[eslint.config.js](../provenance/CONTEXT.md#source-a32a0887ed9d) and [.agents/lint](../provenance/CONTEXT.md#source-ee4a7de0054a). Layer/public-surface rules,
typed adapter ownership checks, GPU boundaries, formatting, type checks, cycle checks and a
[lint-debt guard](../provenance/CONTEXT.md#source-651ec3a94927) are useful precedents. Their presence was inspected;
this pass did not rerun the game suite or qualify their completeness.

The [adapter rule](../provenance/CONTEXT.md#source-389df4109fce) recognizes specified constructions and stepping calls;
it is not a proof that arbitrary code cannot mutate simulation. The [changed-test selector](../provenance/CONTEXT.md#source-2e04d0d1f474)
uses a static import approximation, not a complete content/shader/target impact graph. Preserve these distinctions in
agent output. Do not transplant Broadsides' TypeScript naming, numeric-literal or size limits wholesale into Rust.

The original Broadsides root instruction to read three skills for every task is broader than a future engine/SDK setup needs.
For the new implementation, use scoped instructions and task-relevant references. Keep the existing game's foundation
queue and TS rules with its current code. Repo-local engine skills already take precedence over same-named personal
copies in that repository; SDK distribution needs equally unambiguous version/provenance. This design changes no active policy.

## How work runs

Work follows one proven pattern: a planner writes a high-level design and milestones, a generator builds them, an
automated oracle answers most questions, and a clean-context evaluator judges what the oracle can't. The skills are
the generic orchestrate suite (the `orchestration` domain of `ryannel/skills`), configured for Thrum by
[ORCHESTRATE.md](../../ORCHESTRATE.md), with engine rules in
[thrum-engineering](../../.agents/skills/thrum-engineering/SKILL.md). They are named by level:

| Level | Skill | Produces | Model and effort |
| --- | --- | --- | --- |
| Project and module | `/plan-module` | Module map, gate order, [contracts](CONTRACTS.md); then each module's design, plan and JSON done-when file | Claude Opus 5.5, high |
| Milestone | `/build-milestone` | The next milestone, verified and pushed | Codex `gpt-6.1-sol` or Claude Opus 5.5, medium |
| Milestone | `/review-milestone` | A verdict from a clean context | Claude Opus 5.5, medium |
| Unattended | `/build` | Keeps N builders busy on ready milestones, on Codex, Claude Code or another host | Lead, builders and reviewers at medium |

There are two speeds:
- **One chat.** The owner drives a single chat in Claude or Codex with `build-milestone`, and can move a stream
  between them at a milestone boundary. The plan, the done-when file and the plan's log are the whole handover.
- **Unattended.** A lead on any host runs `build`, told how many builders on which hosts. Builders, reviewers and
  planners run through the suite's `agent_loop.py`, which names the model and effort on every launch, starts a fresh
  session per milestone, holds milestones whose `needs` aren't done, and reruns every check before it accepts a claim.
  Reviews run on the other host where two are available. Escalation
  after a failed review is by count: the same session, then a fresh session, then a re-plan, then park.

Every launch names its model and reasoning effort, so no default ever chooses one. Changing a role's model or effort
is the owner's decision. [`repo-structure`](../../.agents/skills/repo-structure/SKILL.md) covers file placement.
Proposed future routes (`game-authoring` for SDK games, `runtime-diagnosis`) wait until their APIs and commands exist.

The engine repository carries its own routes. A standalone SDK game project gets the relevant game and diagnosis
routes and its own project conventions; engine integration invariants stay fixed, and game-owned instructions aren't
overwritten on an SDK update.

Keep purpose, important decisions and routing in `SKILL.md`; put larger task recipes in references and repeated
mechanical work in tested scripts. Link to the real checker instead of duplicating its entire rule list. Describe
routine safe local work so agents can finish it without repeated approval pauses; publication/signing remain explicit
separate operations. Use normal project permissions, not a skill-created permission system.

[AGENTS discovery](https://learn.chatgpt.com/docs/agent-configuration/agents-md) depends on instruction scope and the
working directory. Verify what a fresh session actually receives; do not assume a nested instruction file is loaded
merely because a tool edits a file beneath it. Keep portable guidance accessible to other agent clients as well.

## Enforce a few consequential rules well

For the recommended Rust implementation, use the compiler and narrow APIs first, with pinned formatting and a selected
lint set. Clippy supports configured [lint levels](https://doc.rust-lang.org/clippy/configuration.html) and
[disallowed methods/types](https://doc.rust-lang.org/clippy/lint_configuration.html); these are useful ingredients,
not an architecture proof. [Cargo metadata](https://doc.rust-lang.org/stable/cargo/commands/cargo-metadata.html)
provides machine-readable dependency information. Evaluate the selected target/features and dependency kinds; a name
search in a manifest is not the effective dependency graph.

| Invariant | Enforcement proposed | Important limit |
| --- | --- | --- |
| Game code uses public engine surfaces; simulation does not depend on presentation | Rust visibility/crate boundaries plus resolved dependency checks for supported target/feature configurations | Transitive dependencies, generated bindings, FFI and tooling exceptions need explicit treatment |
| Authoritative logic obtains time/randomness through declared inputs | Narrow clock/RNG APIs plus scoped symbol-aware forbidden-call checks | Cannot alone prove ordering, numerical determinism or purity of external libraries |
| Each authoritative state has a named owner; integration crosses controlled boundaries | Owner-controlled types/handles, mutation APIs and dev assertions; P3 body solve, W1 committed world, V1 GPU execution/lifetime remain distinct | Interior mutation, unsafe/native code and inaccurate declarations require additional review/tests |
| Systems declare valid access and causal order | Registration and effective schedule validation; controlled access APIs | Legitimately ordered multiple writers are allowed; reject ambiguous ownership/order, not all writes |
| Content and selected features are compatible | Schema/reference/unit validation; provider, save and required content/shader closure | A03–A05 mechanisms must be settled for the advertised combinations |
| Saves and products refer to durable identities | Serialization/schema validation and restore/round-trip fixtures; runtime handles kept private | Passing one fixture does not establish every migration or semantic compatibility |
| Work remains attributable | Build/session/source/product identity in diagnostics and evidence | Declared configuration is not proof that the changed implementation actually ran |

Keep policy near its owning checker, with stable rule IDs, scope, rationale, repair guidance and positive/rejected
fixtures. Check symbol/dependency meaning where needed; regular expressions cannot reliably enforce Rust semantics.
Use small justified exceptions for native/unsafe/numeric boundary code rather than blanket bans or broad exemptions.
Arbitrary function-length or parameter-count rules should not force awkward abstractions into numerical kernels.

Changing a rule, suppression, expected image, threshold or fixture is visible as such in the change report. Fixing a
failure and changing its acceptance condition are different claims. Agents must not silently weaken gates to report
success. `./dev check` on the integrated revision is the gate; CI is manual-only by owner decision and local hooks are convenience.
Scoped selection should include source, schema, content, shader, configuration and deleted dependency edges, with a
conservative broader check when impact is unknown. A zero-test selection or unavailable GPU is not a behavioral pass.

Static structure, resolved configuration and observed execution remain distinct. Useful tools explain all three,
including why a feature is present, whether it activated, and which work it actually performed.

## The development interface agents need

Use ordinary file editing and language tooling for code. Use typed scene/asset operations where stable IDs, revisions
and semantic edits help. Both feed the same canonical source and readiness path. Start with a reliable local CLI and
structured output; a workbench, local API or MCP adapter can call that same implementation. Transport does not create
new authority, and an arbitrary remote code-evaluation endpoint is not needed.

These are operation requirements, not invented command names:

| Operation family | Required result |
| --- | --- |
| Discover | Engine/SDK version, supported targets, actual prerequisites, public schemas/APIs, units/defaults, example compatibility and tool capabilities |
| Inspect | Bounded queries by object, action, system, asset, provider, job or frame; source links and declared/effective/active provenance |
| Change | Typed edit with expected revision, reviewable diff and dependency impact; code edits admitted as a coherent external change set |
| Check/build | Exact candidate source and selected target, stages/results and products; pin before cook, report readiness before activation/export |
| Run/control | Named isolated session, stop/restart, semantic action injection, input recording where supported, private test-save selection |
| Observe | Authoritative state/events, queries, diagnostics, screenshots/clips and traces linked to the actual run and presentation sample |
| Track jobs | Operation ID, progress, terminal outcome, cancellation and reconnect/retry semantics; stale results cannot publish |

Reports need a schema version, source/build/configuration identity, target, check scope and an explicit result such as
passed, failed, skipped, unavailable or inconclusive. Diagnostics carry rule/error ID, location or stable object ID,
cause chain and repair options. Large logs/captures are artifacts linked from a compact result, not unbounded context.
T1/T2 and C10 select the shared path. C4 applies ordinary file changes before immutable snapshot admission and uses restart-first play; FOUNDATION-SPEC owns recovery/adapters. Typed tool requests retain the same preconditions as human edits. Version-matched CLI/API commands and their behavioral qualification remain implementation work.
Dry-run cost/impact estimates must be labeled estimates, not execution or performance proof.

Agents need visual and motion feedback as well as text. A capture records the actual target or labeled proxy, camera,
exposure, content, source/build, world snapshot and presentation sample/history identifiers. A video or filmstrip plus
input/state trace can expose an animation or water failure a still misses. Headless state checks do not establish
image quality, and no visual review substitutes for device measurement.

## Visual review and human steering

Agents can read images, so visual review is part of agent work. The owner steers direction and taste without becoming
a checkpoint, and a report never pauses building.

| Layer | Who and when | Blocking? |
| --- | --- | --- |
| Automated image checks | Every change with visible output: fixed cameras and a short sequence on the named target. Flag NaN, black, clipped, banded and flickering output. Where a portable recipe or ground-truth oracle exists, bound the difference with a perceptual metric such as FLIP; that is a correctness oracle. Golden captures are regression tripwires. These catch broken output; they don't judge the look. | Yes, for that change |
| Inspection list | The planner writes each visual milestone's inspection list in its done-when file: observable qualities checked in the running game, with the bar set in words by shipped high-end games of the same kind. There are no target images to match. The generator checks its own captures against the list. | Shapes the milestone |
| Evaluator | A clean-context `/review-milestone` agent inspects frame sequences and contact sheets captured offscreen from the exact build on the named target, judges each item against the list and bar, and records `agent_reviewed`. | Yes, the milestone closes on its pass |
| Owner steer | The daily report carries new inspection lists, verdicts and contact sheets. The owner adjusts a bar or overturns a verdict; an overturned verdict becomes a graded example for the evaluator. | Never |

Stop and ask the owner only to change purpose or the V1 exit; to spend money, such as renting cloud
machines or devices; to publish, choose a licence or deploy; or for destructive or irreversible actions. Verified
checkpoints are pushed to main without force. While a question waits, continue other unblocked work. Visual quality
claims come only from the named target or a labelled proxy, never from a description of the intended result.

## Ambitious performance improvement

[The selected performance policy](AI-PERFORMANCE.md#performance-investment-from-the-first-design) also governs ordinary
capability design: consider expensive and frequent small work before fixing representations, and accept worthwhile
AI-maintained specialization with explicit invariants and relevant verification. Keep game-facing semantics clear;
agent implementation effort is a resource we can spend on deeply efficient internals. A functioning slice or a met
budget need not end its scoped performance commission.

The [AI performance design](AI-PERFORMANCE.md) extends these tools into bounded optimization campaigns. Agents can
change algorithms, representations, scheduling, content preparation and target specialization against an executable
commission. T2 owns trusted evaluator identity, isolated candidate runs, hardware scheduling and structured evidence;
module owners supply semantic/error/quality obligations. plan-engine retains whole-scene budgets and cross-module
integration. Candidate code cannot weaken its own acceptance criteria. Finalists need independent integrated runs;
same-quality cost reduction and better-quality-within-budget are distinct claims. This is proposed infrastructure,
not a running optimizer or permission to consume unbounded compute. Use existing performance/diagnosis routing until
real campaigns justify a dedicated skill with implemented commands.

## Concrete work loops

**Illustrative pointer-driven game creation.** An agent checks the SDK/target, creates ordinary empty-project source, adds a seed action/count/gate
rule and runs a headless semantic-action fixture. It then exercises actual pointer input, focus, picking and feedback
in the rendered game. Injecting the action headlessly does not prove the pointer route. The result includes source,
the rule outcome and a game-camera capture; failed new code cannot be presented as tested in an older session.
Other games exercise their actual chosen input and output path; this fixture is not a mandatory game recipe.

**Visual or motion repair.** For a boat apparently below water and passing through a dock, capture the failing input
and view. Inspect source scale/pivot, running body/collider, water query time, visual displacement/material and actual
product revisions. Choose the responsible owner from evidence. Apply the correction, rerun the relevant state repro
and inspect motion from the same camera. Do not fix a physical fault by moving only its render mesh.

**Engine contribution.** For a provider or schema change, identify its public contract and affected body, render,
content, save and target consumers before editing. Implement within the agreed boundary, compile/lint, validate the
effective setup and run a consumer fixture. A boundary change returns to `plan-engine` and the affected owners. Report
which integration and target checks actually ran; a provider compiling alone is not a completed game integration.

These loops use analytical designs and tests of known obligations. They do not authorize discovering the architecture
by accumulating experiments or treating every local bug as a reason to redesign the engine.

## Setup, parallel work and durable handoff

Provide a pinned toolchain/dependency setup and one discoverable prerequisite/check entry point, scoped to the requested
target. Pair SDK APIs, skills, examples, source schemas and tools by release. Compile executable documentation examples
against that release. A game author needs the SDK/public surface; an engine contributor needs its implementation and
consumer fixtures. Native mobile SDKs and signing are not prerequisites for every desktop/headless edit.

Generated schemas/API indexes come from the real type/registration sources where possible and have drift checks.
Architecture rationale and game intent remain authored documents; generation cannot infer them. Use the current
maturity ledger for evidence/status and derive build/dependency views from actual metadata, without introducing a
second competing module registry. Do not assume every responsibility maps to one crate.

For parallel changes, the fan-out lead assigns milestones to streams, each in its own worktree; code is shared, and each
stream rebases on main and passes `./dev check` before pushing. Shared contract changes have one coordinating owner,
`plan-engine`, and a producer/consumer impact note before dependent implementation diverges. Worker/build outputs are
task-isolated; shared caches require content keys and concurrency-safe publication. Isolated branches do not
themselves prove integration.

A compact handoff records intent, owned paths, base and tested revision including dirty state, changed contracts,
affected consumers, checks and their exact inputs/results, evidence links/hashes and remaining gaps. The tool emits
facts; the agent explains reasoning. A resumed agent should find current state in these artifacts, not reconstruct it
from a long chat. Failed, skipped or outdated evidence cannot promote maturity automatically.

## Delivery order and behavioral qualification

The [build plan](../BUILD-PLAN.md) owns current sequencing; G6 delivers the first game-authoring loop. The stages below
describe what the tooling needs, not an order of work.

| Stage | Deliver the smallest useful part | Completion boundary |
| --- | --- | --- |
| D1/I1 contributor foundation | Scoped instructions, pinned tools, public boundaries, compiler/format/lint/dependency checks and headless rule fixture | Clean checkout can make and verify one owned code change; no GUI or every-platform prerequisite |
| D1/I2 source/tool foundation | Schema discovery, draft readiness, source/operation identity, bounded inspection and structured check reports | A01/A02 closed for advertised mutations; retries/conflicts cannot claim untested code active |
| I3 game loop | Empty project interaction, actual input route, state inspection, capture and restart | A03 shader demand/readiness resolved in D1–D2 before the supported cooked render path; write `game-authoring` against real commands |
| Relevant I3/I4 slices | Provider fixture, target products, clean source handoff and development export | Qualified scope recorded; A04/A05 resolved for advertised water-domain and overlapping-media combinations |

Compiler/lint/docs work can start before runtime tools exist. A01/A02 gate claims about source/session mutations, not
all useful contributor tooling. Introduce `runtime-diagnosis` when inspect/replay is operational. Start with existing
tooling before building custom compiler plugins or a large orchestration service.

Evaluate the skills by fresh-agent tasks on pinned fixtures; G6's fresh agent building a new small scene from public docs
is the first. Later tasks cover ordinary bug repair, new game from empty source, external
library use, a cross-boundary schema change, a visual/runtime mismatch and a clean handoff. Include routing cases where
a typo does not trigger architecture research and a model change does require shared review. Record completed behavior,
API invention, unrelated edits, weakened checks, recovery quality, unnecessary context and elapsed time when runnable.
Do not grade only wording, headings or invocation. This follows OpenAI's
[behavioral skill evaluation guidance](https://developers.openai.com/blog/eval-skills), with correctness checks and
judgment separated. Skill syntax validation is useful but does not establish workflow reliability.

Full analyses: [enforcement](../evidence/handoffs/2026-11-01-engine-ai-workflows/ENFORCEMENT.md),
[skills/setup](../evidence/handoffs/2026-11-01-engine-ai-workflows/SKILLS-SETUP.md),
[game workflows](../evidence/handoffs/2026-11-01-engine-ai-workflows/GAME-WORKFLOWS.md).

The [lifecycle skill walkthrough](LIFECYCLE-WALKTHROUGHS.md#skill-support-now-and-later) refines existing Broadsides
storage/performance references now. Future SDK task skills should invoke implemented package inspection, publication
and closeout operations, with one source of truth for each record. Skills express selection and retention judgment;
actual hash/closure/precondition checks and storage admission belong in tested tools. Do not require novice creators
to understand collector leases or load the whole engine programme to deliver a prop.
