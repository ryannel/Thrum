# Dogfooding coordinated AI optimization

1 November 2026. Four **paper campaigns**, applying [AI performance](AI-PERFORMANCE.md) to different modules and
platforms. Three Sol High agents challenged rendering, browser delivery and mobile simulation; the coordinator
derived the animation case and reconciled the shared workflow. No engine was run, no performance measurement was
taken and no optimization was accepted. Numbers below are illustrative proposed targets, not observed baselines,
selected product budgets or device certification. A01–A05 remain open.

The practical test is whether an agent team can improve a specific game outcome while preserving its other
obligations, and explain the scope of that result. “Whole engine” means the assembled game on declared workloads and
targets; it does not mean a universal speedup across every genre, scene and device.

## Shared team and campaign record

With four agent slots, use one coordinator, two implementation/hypothesis agents and one evaluator/reviewer. Human
art/game judgment remains a final review where relevant; it is not an extra permanent agent. Specialists can replace
an idle implementer in a later phase. Actual benchmark execution is tooling on leased hardware, not another agent.

| Role | Concrete work | Cannot decide alone |
| --- | --- | --- |
| Coordinator | Pins the common workload/objective and accepted baseline, assigns distinct hypotheses, reconciles consumer/resource changes, chooses candidate combinations and integration order | Cannot claim measured results from paper reasoning or aggregate away a required target failure |
| Agent A | Develops one reasoned representation/algorithm candidate in owned source/build paths, reports validity and demand changes | Cannot redefine quality, shared schemas or another owner's state implicitly |
| Agent B | Develops a complementary or competing strategy; exchanges exact assumptions with A before implementation diverges | Cannot assume A's transient/scheduling budget is available or copy an unreviewed producer format |
| Evaluator | Owns scenario/protocol/reference revisions, requests controlled measurements, diagnoses failures and independently rebuilds finalists | Cannot approve appearance by scalar score or quietly change the evaluator to admit a candidate |

Before launch, the record pins authored inputs and output requirements, each actual hardware/software configuration,
baseline code/products, quality/error criteria, allowed changes and a finite resource budget. Candidate binary/shader/
cooked hashes intentionally differ and are recorded separately. A useful initial **illustrative** allocation is two
hypothesis branches, at most eight CPU-cheap candidate revisions, then at most three expensive finalists under a fixed
device-hour cap; the real allocation follows evaluation cost and available resources. This is not a universal trial
count or authorization to schedule work. Stop/reallocate on evidence, resource limits or a changed premise.
Evaluator fixtures, thresholds and result collection are versioned outside candidate-edit authority. A separate
reviewer in the same writable tree is not an integrity boundary; actual harness isolation remains to be implemented.

Every consequential proposal carries a compact **consumer and resource delta**: changed inputs/outputs/validity,
affected owner IDs, persistent/transient bytes, CPU/GPU/transfer work, queue/freshness impact, shader/cook/build cost and
fallback conditions. These are estimates until measured. One coordinating allocation of shared resources prevents two
branches from both assuming all remaining memory/bandwidth. A contract revision returns to its owners before dependent
implementation; it is not negotiated by letting conflicting code accumulate.

## O01 — A beautiful forest on an Apple silicon Mac

**Proposed commission.** An outdoor exploration scene at a fixed 1440p output, with first-person travel and an
orthographic overview. Keep moving vegetation, shadows, a controllable character, audio/UI and region changes active.
On one inventoried Mac/OS/backend configuration, target a 25% reduction in p95 render-completion time with a selected
memory cap and no material regression in presentation deadlines or input response. Exact camera/lighting/quality
requirements, regression margins and run protocol must be fixed before execution; no Mac model is qualified here.

**Reasoned agents.** A works across F4/V2 on instance layout and conservative visibility. Its hypothesis is that
better locality and rejecting irrelevant instances can reduce traversal, upload and downstream draw work. B works
across V2/V3/E1 on shadow demand and reuse: visible receivers can depend on offscreen casters, so the useful partition
is not simply the main camera frustum. V1 owns allocation/execution. The evaluator checks moving silhouettes, shadow
coverage, view histories, cold preparation and actual full-scene timing.

**Coordination event.** A proposes shared cluster bounds and B requests extra shadow metadata. The coordinator asks
both to cost the combined representation, deformation bound updates, cook invalidation and transient lists. They agree
one owner and schema revision. If their combined list/metadata peak exceeds the campaign cap, one changes its plan
before a GPU run. A small scene can retain the simpler layout through a declared workload-valid path.

**Tempting false win.** Main-view culling becomes faster but removes trees whose shadows remain visible; or the
shadow benchmark improves because lighting is frozen while the real scene's moving Sun invalidates the cache.
The evaluator rejects the lost cue/invalid validity domain. Another candidate preserves visuals but adds a prepass
and memory traffic that lengthen full-frame completion. That is a local technique result, not target attainment.

**Revised path and decision.** Use separate receiver/caster relevance with shared bounds only where their validity
matches; charge update work and retained buffers once. Compare each candidate and their combination in ordinary
packaged runs, with concurrent animation, streaming, UI and audio. Check turning/teleporting cameras, wind changes,
fresh lighting, region transitions and a held-out indoor/sparse scene. An accepted outcome needs whole-scene time or
the declared resource benefit, quality evidence and bounded startup/peak cost. A Mac-specific win can select a Mac
path with fallback; it cannot certify Windows/browser/mobile. A03 gates the cooked shader path; A05 matters only if
the proposed change touches the unresolved overlapping-media mechanism.

## O02 — Many animated characters on Windows

**Proposed commission.** A third-person action hub with 120 characters, several skeleton/clip families, an independent
map view, interaction hitboxes, root-motion movement and footstep audio. The fixed workload includes entering a crowd,
selecting a nearby character, an offscreen interaction, a camera cut and ragdoll recovery. Target 30% less p95
animation/extraction critical-path cost and a measurable reduction in action-to-visible-response delay on one named
Windows CPU/GPU/OS/backend build. This is a workload proposal, not an engine limit or a measured bottleneck.

**Reasoned agents.** A owns P4/F4 sampling and blending: batch compatible data, reduce redundant decoding and use
layout/SIMD opportunities. Its work model includes characters, joints and clip layers, plus grouping/cache overhead.
B owns P4/V2 presentation extraction and V1 upload/skinning coordination. It seeks valid reuse across passes/views
and compact transfers. They preserve P3's achieved root motion, P6 event identity and A1 consumption; the GPU render
pose does not become authoritative hitbox or locomotion state.

**Coordination event.** B wants to evaluate poses only for visible characters and asks A for a shared cache. A reports
that root requests and hitboxes still need tick-time data, and different clips/phases/masks/IK inputs cannot share a
pose merely because skeletons match. They agree exact cache keys, lifetimes and invalidation, with gameplay-required
evaluation retained and presentation histories indexed by each actual retained sample. The map can have an older
history than the main view. The coordinator reserves old/new palette bytes and accounts for both views.

**Tempting false win.** A render-only clip viewer shows a large improvement from dropping offscreen evaluation. In the
game, a character misses its attack window or footstep, root motion changes, or returning to view produces invalid
motion vectors. That candidate fails correctness/quality. Another candidate compresses palettes enough to save
upload bytes, but decoding delays P3 on the critical path; fewer bytes alone does not meet this latency commission.

**Revised path and decision.** Batch valid sampling work and reuse only data whose complete inputs/time match; retain
required tick-time state and explicit history invalidation. Test the whole hub, including physics, cloth only if
selected, skinning, shadows, audio, input and background loads. A held-out case varies skeleton proportions, clip
wrap/transition timing, blend layers and character count; a single-character game checks fixed overhead. Observe
input through action admission, solved body and presented response. If CPU time falls but GPU/present waits keep
response unchanged, record a scoped CPU saving, not the claimed responsiveness success. Any useful secondary memory/
power result must be predeclared or evaluated under a separately revised commission. No universal bitwise pose
equivalence is assumed beyond the selected numerical contract. A03 gates affected cooked shader variants.

## O03 — A browser demo that becomes playable quickly

**Proposed commission.** A compact exploration demo loads from a cold cache, gives the player control, then crosses two
regions while audio/UI remain responsive. On a pinned browser/device/hosting configuration and controlled connection,
target a 30% reduction in navigation-to-first-successful-game-action-and-feedback time, plus fewer streaming deadline
misses. Bound initial bytes, total session transfer, memory and the longest required-content wait. Record cold/warm
and offline-supported states separately. A displayed loading screen or a movable camera alone is not first play.

**Reasoned agents.** A owns F4/T2 packaging: boot closure, chunk boundaries, compression and request dependencies.
B owns F4/W2/F1 runtime preparation: decode batches, upload pacing, cancellation and activation readiness. V1 validates
upload/resource lifetime and P3/P5 validate colliders/nav. A's hypothesis concerns critical dependency depth/bytes;
B's concerns preparation contention and latency, including temporary old/new products.

**Coordination event.** A's smaller package requires a larger decode buffer and B wants several chunks in flight.
The coordinator combines those peaks with the running scene before accepting either design. They settle an explicit
first-play dependency set and staged availability: required world/interaction/shader products must be ready, while
declared optional distant visuals may follow. The same source/project is built with candidate-specific products.

**Tempting false win.** Download completion arrives sooner, but synchronous decoding stalls input; or first frame
arrives early because collision, controls or required interaction feedback are still missing. Another prefetch strategy wins the fixed route
by downloading unseen regions and regresses memory or total transfer. The evaluator measures the actual interaction,
the subsequent journey and remaining work, rather than selecting the earliest convenient timestamp.

**Unbounded wait case.** A changing connection has no guaranteed maximum readiness time. Prefetch lookahead cannot
promise seamless traversal through a disconnected region. The game must choose safe boundary/loading behavior,
timeout/retry and pause policy; blocked duration remains part of the result. A candidate cannot improve active-frame
statistics by moving slow intervals into an unreported loading category. Where browser audio needs a gesture, test
its response after the action and report that state explicitly rather than requiring pre-gesture sound for readiness.

**Revised path and decision.** Prepare the real startup closure, schedule bounded subsequent work and activate only
complete required region products. Take an alternate route, change direction, disconnect where offline play is
claimed, cancel/retry loading and revisit earlier regions. Record meaningful readiness milestones, byte totals,
queue age, memory peak and the end of the measurement window's unfinished mandatory work. A fast timed interval
cannot export its debt to the following interval. A fresh browser profile and actual exported package must reproduce
the result; a developer cache is not delivery evidence. Browser success does not certify native mobile packaging.
A01/A02 gate live-edit/preview and affected external source-change workflows; a static pinned package comparison
can run without claiming those gaps closed. A03 gates required shader readiness, not just download closure.

## O04 — Sustained mobile tactics with many moving agents

**Proposed commission.** A native top-down tactics game with 200 agents, interactive obstacles, path commands,
selection/UI, audio, animation and shadows, on one inventoried iPhone and one Android configuration. Target improved
sustained frame deadline compliance after a declared long play window while preserving command-to-motion latency,
path freshness, collision policy, fixed simulation time and memory bounds. A 30 fps / 20-minute workload is an
illustrative starting specification, not a selected platform floor or proof of thermal steady state.

**Reasoned agents.** A owns P5 representation and request scheduling: spatial neighborhood queries, route reuse under
exact topology/agent constraints and bounded path search work. B owns P3/P5/F1 data flow and task layout, preserving
the actual solve and committed query epochs. Their hypotheses concern repeated neighbor/path work and cache/scheduler
overhead. The evaluator also watches UI input, animation, audio, backlog and power/thermal metadata where available.

**Coordination event.** A requests a shared neighborhood index while B changes its publication time. The coordinator
requires consumer-compatible query meaning and age, generation validation and update/retirement cost. The index may
support both consumers when their semantics match, but P5 still produces intent and P3 owns solved bodies. Two
independently helpful caches cannot both reserve the same available memory. Share or simplify after joint cost analysis.

A concrete interaction to reject is A retaining an old P3 query snapshot while B prepares its replacement and W2
stages a new cell. The three generations together can exceed admission even if each isolated branch fits. Cancellation
is not permission to reclaim a page still in use. Bound reader leases, pending route dependencies and replacement
overlap; safe invalidation or declining one candidate is preferable to silently reclaiming live data. Also distinguish
the selected simulation rate from display rate: two world ticks per image would charge both ticks to that image's
CPU path. This walkthrough selects neither a universal tick rate nor a fixed two-tick budget.

**Tempting false win.** Navigation milliseconds fall by postponing path requests, but commands respond late and the
queue grows; skipping collision steps makes a crowd benchmark fast while bodies pass through obstacles. Both fail.
A genuinely efficient CPU branch can also increase memory traffic enough that the complete mobile run loses its
sustained benefit. A short cool-device burst does not resolve that question.

**Revised path and decision.** Remove repeated work within explicit validity domains, batch compatible requests and
retain age/deadline/fairness bounds. Close/reopen doors, issue burst commands, mix agent sizes, stream a boundary,
save/restore and suspend/resume at several points. Measure sustained delivered frames and input-to-motion together
with completed/deferred/cancelled work and the oldest pending request; verify arrivals and collision outcomes within
the specified game/numerical contract. Compare each device separately with matched run/thermal conditions. If one
layout wins on iPhone but loses on Android, retain a finite qualified variant/fallback or reject a universal default;
averaging the two cannot satisfy either device's requirements. Missing device evidence stays unqualified.

## Demonstrating whole-engine improvement

Each case uses the following qualification path; these are prospective checks, not completed tests.

| Check | What it rules out |
| --- | --- |
| Same authored task and required experience; verified active candidate artifacts | Easier scene, stale binary/shader, altered output/physics and warm-cache substitution |
| Reference/invariant/consumer checks and affected visual motion review | Fast incorrect work, stale queries, dropped events, bad temporal history or lost visual cues |
| Same-binary noise control, paired candidate comparison and independent final runs | Measurement drift, instrumentation artifacts and selection of a lucky trial |
| Full application under simultaneous consumers, including transitions | An isolated kernel/pass win that worsens another critical path, resource peak or startup |
| Work completion, freshness and end-window debt | Smaller timed work achieved by postponing obligations or accumulating queues |
| Fresh content/input distributions and a contrasting simple game | Route-specific fitting, pathological fallback and disproportionate fixed overhead |
| Actual supported target rows with an explicit unavailable state | Desktop gains presented as browser/mobile proof or averaged-away regressions |
| Sequential integration on the exact combined revision | Two separately qualified changes assumed to compose successfully |

The coordinator compares `B0`, `B0+A`, `B0+B` and `B0+A+B` when A/B are compatible and interaction matters; alternative
representations are compared as alternatives, not forcibly combined. All rows record exact artifact identities and
the same comparison contract. If A and B change a shared contract, prepare a coherent combined candidate before any
claim about their interaction. Controlled ablation is diagnostic, not an excuse to run invalid mixtures.

An accepted report names the actual improvement, constraints preserved, affected workload/target range and remaining
limits. Distinguish **local cost saving**, **integrated target attainment**, **accepted quality/capacity increase**, and
**qualified target-specific variant**. CPU headroom or package savings can be valuable without an FPS increase; claim
that value explicitly. Global performance qualification remains configuration-specific in [maturity](MATURITY.md).

## Refinements exposed by the walkthroughs

1. The commission needs a **meaningful completion event**, not merely a convenient timer: a responded-to action,
   complete required region, solved motion or delivered frame. Intermediate milestones remain diagnostic.
2. Candidate handoffs need the **consumer/resource delta** above, including shared bandwidth, transition peak and
   shifted work. Interface signatures alone do not coordinate independently affordable modules.
3. Reports need **remaining work and freshness** at window boundaries. Counting only executed work rewards deferral.
4. Promotion needs an explicit **claim category and scope**, allowing honest local savings and target-specific wins
   without pretending they meet a broader objective or automatically rejecting useful narrower results.
5. Integration needs **combined-candidate provenance and compatibility**, plus targeted ablation when meaningful.
6. Qualification scope follows affected consumers and selected targets. A scoped Mac improvement need not wait for
   every future platform, but cannot be advertised as a portable default without that coverage.

The narratives expose mechanisms to finish, not findings already closed by the stories:

| Before executing | Remaining detailed mechanism / owning commission |
| --- | --- |
| O01 visibility/shadow reuse | Computable conservative deformation/influence bounds, cache validity/age and shared resource lifetime/peak layout; E1/V2/V3/V1/F4, D2; A03 for cooked variants |
| O02 animation reuse | Complete pose/cache keys, tick versus presentation consumers, bounded per-view sample retention and invalidation; P4/P3/V2/V4/F4, D1/D2 |
| O03 first play/streaming | Executable first-play dependency/event definition, game-owned network-tail/loading policy and exact package/manifest/cache rollout; F1/F4/W2/T2/P1, D1; A01/A02 only for affected source/session/edit workflows and A03 for shaders |
| O04 query/path optimization | Game-level route deadline, pending acknowledgment, safe movement/fairness, query lease horizon and stale/cancelled result retirement across cell replacement; P3/P5/F1/W2/F4, D1/D2 |
| All cases | Named configurations, usable baseline, accepted quality/error envelopes, evaluator implementation and calibrated comparison protocol; D0 and relevant I1–I4 slices |

Fixed immutable build comparisons can begin on a runnable slice without claiming live-edit/session mechanisms have
been solved. Conversely, a workflow that changes source during play must meet its A01/A02 obligations. No campaign
can tune away the missing meaning of a field, query or readiness state.

These extend the existing T2 commission/result design and coordinator handoff, not the runtime module map. No new
mandatory plugin system or universal benchmark service follows, and appearance still needs agent visual review rather than a scalar score. Independent inputs:
[rendering](../evidence/handoffs/2026-11-01-engine-performance-walkthroughs/RENDER.md),
[browser](../evidence/handoffs/2026-11-01-engine-performance-walkthroughs/BROWSER.md),
[mobile](../evidence/handoffs/2026-11-01-engine-performance-walkthroughs/MOBILE.md).
