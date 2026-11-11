# Engine maturity and evidence

Thrum repository setup is tracked separately in the ledger’s `repository_setup` record. Current design digests are checked by `./dev check-docs`; imported review hashes refer to original source bytes, as explained in [provenance](../provenance/IMPORT.md). A compilable scaffold is not an implemented engine module. The [build plan](../plans/BRIEF.md) owns current sequencing; the dated sections below are history.

The machine-readable [maturity ledger](maturity.json) tracks the **new target architecture**. It is not a claim that
the repository has no existing implementations. Prior code and demonstrations are unassessed reuse candidates until
their contracts and evidence are reconciled against this target. The [host assessment](HOST-AND-PLATFORMS.md) records
one inspected branch boundary; do not infer the rest of that branch's correctness or completeness from it.

## Independent dimensions

| Dimension | States and meaning | Promotion evidence |
| --- | --- | --- |
| Decision | open, recommended, selected, rejected, superseded | Decision owner, rationale, alternatives and conditions. User selections are distinguished from engineering recommendations. |
| Design | unscoped, scoped, high_level, detailed | Requirements mapped to actual state/operators/contracts; detailed means essential mechanisms and edge cases are specified sufficiently to implement. A scheduled experiment does not close a design gap. |
| Integration review | pending, reviewed_with_gaps, reconciled | Producer/consumer agreement, full scenario trace and finding dispositions against exact document revisions. Does not certify runtime integration. |
| Implementation | not_started_for_target, partial, feature_complete | Artifact revision and requirement coverage; compiling alone is partial evidence. Existing candidate code has a separate reconciliation field. |
| Functional validation | not_run, failing, partial, passed | Reproducible checks against independently stated requirements, including coupled behaviour and relevant failure transitions. |
| Experienced quality | not_reviewed, changes_requested, agent_reviewed, accepted, not_applicable | Reviewed build, content, camera and profile on the named target. `agent_reviewed` records automated image checks plus agent and independent-reviewer inspection; it is the evidence a gate needs. `accepted` records the owner's asynchronous steer and is never a gate condition. See [visual review and human steering](AI-DEVELOPMENT.md#visual-review-and-human-steering). |
| Platform/performance | not_run, failing, qualified, out_of_scope for each configuration | Named device/OS/backend/build/content, representative duration, resolution/settings, frame distributions, memory/startup and lifecycle results. |

There is no engine-wide completion percentage. A profile is qualified only when its required dependency closure is
qualified together, including transitions and shared resource limits. A fully designed renderer can coexist with an
unfinished animation contract; the combined profile remains unqualified. An unavailable target stays unmeasured.

## Performance design and qualification

`performance_cost_design` defaults to `not_assessed` for each owner; `modelled` means its scoped workload, scaling,
representation alternatives and verification account are recorded, while `needs_revision` identifies an invalidated
account. An owner override links its account/evidence from the existing module row. This is design readiness, separate
from the existing platform/performance dimension. A selected investment policy does not assess all modules at once.
Local kernel/count/byte evidence, integrated non-regression and measured whole-engine gains remain distinct claims.
The `performance_investment_policy` record identifies the selected process, affected scope and pending I1 work.
Earlier architecture reviews are not independent validation of this later policy or its still-unwritten module accounts.

## Ledger conventions

Stable module IDs correspond to the architecture brief. Child IDs refine a parent responsibility; they do not add
mandatory features to every game. Each row names its design artifact, requirements, consumed shared contracts,
remaining design work, planned qualification and existing-code reconciliation status. Defaults apply explicitly to
all rows unless overridden. This keeps status readable without copying a platform matrix into every record.

Design documents are not evidence of running code. Research sources support the design reasoning only. Mark the
reviewed artifact hash/revision when a review closes; any changed assumption invalidates relevant review scope. A
minor edit need not reset unrelated qualification, but its author records the affected contracts and consumers.

An implementation evidence record should contain: module/capability IDs, claim, result, source revision, build/config,
content hashes, profile/provider/dependency versions, device/OS/API, method and duration, metrics or capture locations,
reviewer/acceptance scope, limitations and a reproducible command. Keep large traces/captures outside these notes and
retain durable selected results according to repository storage rules. No missing file may count as passing evidence.

## Update responsibilities

- Module owners update design/implementation/evidence for their scope with the change that caused it.
- The coordinator updates shared contracts, dependent review status and profile coverage; independent reports do not
  promote each other automatically.
- Tooling validates IDs, links, permitted states and dependency references. Those checks prove ledger consistency,
  not the truth of claims. Humans/agents inspect the underlying evidence for substantive promotion.
- Qualification is configuration-specific. A Mac run does not qualify Android; a spatial-AA run does not qualify a
  temporal provider; a CPU-only correctness check does not qualify GPU simulation or perceptual quality.

## Historical review sequence (C4 disposition below)

The [scenario walkthroughs](SCENARIO-WALKTHROUGHS.md) record paper task traces and a simplified recommended developer
surface. The ledger's `scenario_review` identifies that scope and artifact hashes separately from implementation,
function, visual review and platform evidence. They refine decisions; they do not pass runtime tests or close the
five audit findings by telling a plausible story. Current versus formerly reviewed design hashes remain distinct.

The subsequent [first-use pass](FIRST-USE-WALKTHROUGHS.md) has a separate `first_use_review` record: nine analytical
journeys, coordinator dispositions and cross-review of onboarding, extension and art paths. It adds owner-specific
design obligations; it does not advance implementation, experienced quality or target qualification. Prior scenario
hashes remain evidence of the earlier reviewed revisions; current document hashes are recorded separately.

The `from_scratch_review` adds nine empty-project paper journeys for newcomers, veterans and different delivery
circumstances. It records scoped readiness, helper/game separation, headless entry and handoff refinements. Earlier
scenario/first-use report hashes stay historical; latest affected hashes and owner deltas are explicit. No actual
newcomer or veteran has completed these engine workflows as part of this review.

The `ai_development_review` records the inspected current tooling, proposed engine/game instruction bundles,
enforcement/tool requirements and three analytical agent workflows. Source inspection is not execution of those
checks. Proposed skills are not installed, and no behavioral skill evaluation or target runtime qualification was
performed. Current affected design hashes are recorded without replacing historical review revisions.

The `capability_survey` records official-page research across twelve engines, coordinated owner reconciliation and
twelve inventory dispositions. It distinguishes actual recommended extensions from existing but poorly surfaced
capabilities and optional tools. Current owner hashes change; historical reviews stay intact. It adds no runtime,
platform or usability evidence and closes none of A01–A05.

The `ai_performance_design` records research and design for bounded optimization campaigns, evaluator ownership and
whole-engine promotion. No campaign, benchmark harness or speedup is implemented or measured by this record. Future
results must distinguish a local candidate from its integrated, visually reviewed and target-qualified revision.

The `performance_walkthrough_review` records four paper campaigns and coordinated review of the agent handoffs,
end-to-end completion, shared resource conflicts, outstanding work and combined-candidate claims. Numerical targets
are illustrative; no campaign was executed and no device, function or appearance result is promoted. Earlier design
hashes remain historical evidence for their actual revisions.

The [architectural decision audit](ARCHITECTURE-AUDIT.md) originally reopened five affected cross-module boundaries. The ledger's
`architecture_audit` records those findings, owners and closure conditions; affected modules are
`reviewed_with_gaps` and name their open findings. Earlier artifact hashes and reviews remain historical evidence for
their actual scope, not a claim of present architecture-wide closure. J01–J04 are comparison/workflow follow-ups,
separate from the five retained gaps. No audit finding adds runtime evidence or a new mandatory feature.

This commission develops a high-level analytically reasoned design and a structured plan for detailed module design
and construction. No runtime implementation or device benchmark is commissioned in this pass. The ledger must retain
not-run implementation/quality/platform evidence even after its design review is complete. Existing water, forest,
asset and host implementations retain their own historical evidence and are candidates for later selective reuse.

The `storage_lifecycle_design` entry records read-only directory inventory, two Sol High design challenges and
coordinator synthesis. It adds F4/T1/T2 storage obligations and corrects contradictory local retirement guidance.
It proves neither recoverable space nor source restoration, automated collection, promotion or runtime maturity.

The `lifecycle_walkthrough_review` records four paper asset/campaign journeys and three fictional operator skill
cases, including a narrow closeout wording correction and follow-up. Two existing skill references passed structural
validation. It adds source/delivery preconditions and proportional creator/retention workflow obligations; it does
not establish Astra behavior, executable safety, cleanup, restoration or runtime/platform qualification.

## Whole-plan review baseline and parent scope

The [three-perspective review](PLAN-REVIEW.md) independently reads the full plan and cross-challenges systems,
creator and delivery findings. Its `whole_plan_review` record preserves the baseline, reports and six dispositions.
At that baseline WP01 exposed a transform representability error; WP02–WP05 recommended milestone/support/acceptance
clarifications and A01–A05 remained open. The C3 selection below records the subsequent disposition; runtime maturity is unchanged.

A row's `integration_review` describes its local recorded scope. When a module has a `parent`, read that parent's
open findings and their affected capability scope before treating the leaf as ready. In particular E1 water inherits
the relevant A04 concern, and overlapping atmosphere/cloud media the relevant A05 concern, regardless of a historical
leaf `reconciled` label. Do not infer all E1 children are blocked by every parent issue. WP06 recommends an effective
status query over these existing links; this clarification is guidance, not an implemented query or duplicated issue registry.

## Prior selection update — C3, 1 November 2026

[Architecture decisions](ARCHITECTURE-DECISIONS.md) select A01–A05 boundaries and J01–J04 choices, adopt WP01–WP06,
and add N1 as the thirtieth responsibility. Three coordinated Sol High decision studies and a subsequent adversarial
cross-review exposed/fixed source mirroring races, local versus match identity, gameplay versus visual profile pins,
role-specific physical products and authored-revision transform inputs. These are analytical checks, not runtime proof.

The ledger retains historical review hashes and baseline finding status alongside explicit current dispositions.
`current_design_sha256` identifies the now integrated file; it does not extend an older review to new text. Current
architectural findings can be resolved at their stated scope while `remaining_detailed_design` still prevents building
the dependent mechanism. The subsequent BASELINE-SPEC and its four detailed specifications now fix WP01 numerical limits, source adapters,
shader ABI/colour basis, initial water operators and N1 wire/transport profile. Runtime conformance and measured proof
remain unrun; broader family capabilities outside the admitted baseline keep separate design obligations. No implementation, quality or
platform dimension is promoted. C3 contract and current file hashes are checked separately from historical hashes.

Effective readiness is the union of the module's local obligations and applicable ancestor/shared-contract obligations,
filtered by the selected capability. This applies even if an older local review says reconciled. A04/A05 architectural
policy is now selected; detailed water/media obligations still reach their relevant E1 leaves. Do not duplicate every
parent finding into unrelated leaves. An automated runtime/tool query is not implemented by this metadata rule.


The `architecture_resolution` entry now records G01–G10 and exact current specification hashes, with N1's I2n milestone
explicitly retained. `initial_specifications` on module rows identifies the specified subset; it does not promote the
whole family to detailed status. N1's initial profile is detailed. Defaults still say target implementation not started,
functional/platform checks not run and visual quality not reviewed. Effective ancestor/shared obligations continue
to apply to every capability selection. That pass recorded closure of the named design gaps; the fresh disposition below
supersedes that closure for the findings it reopens, without relabelling measurements as done.

## Fresh simplification review — findings before reconciliation

Three new Sol High agents independently challenged the frozen 42-file plan and cross-reviewed their results.
[SIMPLIFICATION-REVIEW](SIMPLIFICATION-REVIEW.md) preserves server authority and the early I2n milestone but reopens
G08 for action results, atomic lifecycle/state admission and compatibility-hash scope (SR01–SR03). G09 also needs the
concrete R4 byte/cadence/latency recipe and portable live-transition admission proof (SR05/SR07). These are design
obligations, not merely future measurements. N1 remains detailed but `reviewed_with_gaps`; its earlier “none remaining”
claim is superseded. Affected consumers inherit these shared obligations without duplicating them into every module.

G01/G02 remain selected with explicit simplification challenges. File-first authoring, restart-first edits, native-only
initial authoring, a smaller first game and optional delta/prediction/water recipes are recommended, not silently
substituted across canonical protocols. `simplification_review` retains reviewed file/report hashes and dispositions;
`architecture_resolution` points to the current open design work. All runtime/quality/platform evidence is unchanged.

## Reconciled selection — C4, 1 November 2026

The user selected the reductions and required all affected contracts/modules to agree before construction. Three
Sol High owners reconciled source, multiplayer and delivery, then cross-challenged the integrated protocols. C4 keeps
30 responsibilities and the same E01–E20 product scope. [Simplification dispositions](SIMPLIFICATION-REVIEW.md) record
SR01–SR08 closure for the initial scope; [conversation coverage](ARCHITECTURE-BRIEF.md#conversation-coverage-and-retained-scope)
links each substantive request to its governing design.

G08 now has committed action outcomes, individual/prefix resolution, atomic lifecycle/state, exact gameplay/wire
compatibility, bounded full-group replication and explicit reconnect/suspend behavior. G09 contains an encoded R4
recipe including result-rich resync and concrete portable boot/transition estimates. These are analytical admission
and failure mechanisms, not measured performance. G01/G02 now select file-first native admission, restart-first edits
and checkpoint/drain/retire/load recovery for bounded initial worlds; full browser authoring and live whole-world
replacement are not baseline prerequisites. C4 placed I2n alongside the small game; since 5 November 2026 the
[build plan](../plans/BRIEF.md) parks transport until after V1 and G2S proves multiplayer fitness headless.

The `reconciliation` ledger record retains the prior C3 evidence and exact C4 hashes, affected owners, requirements,
selected finding dispositions and validation scope. N1's initial design is detailed and reconciled; broader family
rows retain their scoped detailed-design obligations (for example advanced camera, effects, audio/UI, environmental
providers and seamless streaming). No row is promoted to implemented, tested, visually reviewed or device-qualified.
Implement only the admitted subset after its dependency/API/toolchain checks; additional claims require their own
complete mechanism before construction. Paper closure does not certify the full future engine.

## I1 headless slice — scoped local evidence through 2 November 2026

The [I1 design](I1-HEADLESS.md) governs the bounded executable authority/replica path. A game-owned
collect/open rule uses the same validation and commitment in standalone and match roles. The
[running evidence](../evidence/i1-headless/QUALIFICATION.md) includes correctness tests and
local release-mode cost fixtures at 64, 4,096 and 16,384 objects. Separate records cover the
[clock](../evidence/i1-headless/CLOCK.md), [native headless callback](../evidence/i1-headless/HOST.md)
and [two controlled peers](../evidence/i1-headless/TWO-PEER.md). Later selected fixed-slot and
bounded structural hosts charge their engine-owned publication, pending and result storage through
a shared pool; the [combined scope record](../evidence/i1-headless/FOUNDATION-STATUS-2026-11-02.md)
separates those local proofs from general game allocations and device behavior. The proposed
universal process-memory cap was [retired](../evidence/i1-headless/MEMORY-DIRECTION-2026-11-02.md);
allocation, RSS and pressure response still need representative measurement. F1, F3, W1, P6 and N1
remain `partial` for implementation and functional validation in their bounded scopes; F1/W1/P6/N1
have `modelled` I1 cost accounts. Existing review hashes and whole-module design stages are
unchanged. Named platform performance remains `not_run`; real transport, production source/save,
broader scheduling and visible phone presentation remain separate work. This evidence does not
promote the whole engine or I1 package to feature-complete.
