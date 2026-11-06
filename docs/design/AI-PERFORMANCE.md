# AI-driven performance engineering

1 November 2026. Selected performance-development direction, extending [AI development](AI-DEVELOPMENT.md), the
[construction plan](CONSTRUCTION-PLAN.md) and C12. This is a design for future optimization campaigns, not an
implemented harness, running campaign or measured speedup. No hardware/cloud spend or overnight automation is started.
The target remains the complete custom 3D engine, with no required runtime AI service. A01–A05 now have selected architectural resolutions in [the decision record](ARCHITECTURE-DECISIONS.md); their detailed implementations remain unqualified.

The opportunity is to make the engine **cheap to understand, change and evaluate repeatedly**. Agents can explore
many reasoned algorithms, representations and target-specific implementations that would be expensive to maintain
by hand. The scarce resource becomes trustworthy evaluation and integration, as well as model/hardware time.
We should invest in those mechanisms early, while retaining a small understandable public engine.

## Performance investment from the first design

**User-selected ambition, 1 November 2026:** shocking performance, pursued deeply with mature engineering.
Zed remains an experience reference for speed and responsiveness, not an architecture choice or engine comparator.
Fast startup, immediate feedback, stable pacing, efficient memory and throughput remain ambitions alongside quality,
correctness and stability. Native uses the hardware fully; portable remains a capable fallback.

**Owner decision, 6 November 2026, 11:15:** [foundation, tripwires, then tuning](decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md).
This replaces numeric performance admission gates during feature construction, including allocation targets.

1. **Design the foundation for high performance.** Give expensive-to-reverse choices the most care: column storage,
   GPU-driven rendering, threading, memory ownership, reuse in hot paths and hardware-native recipe seams. Review
   against current best practice and the [hardware-native decision](decisions/HARDWARE-NATIVE-ENGINE.md). These
   choices do not wait for a benchmark. Prefer no allocation by design; unavoidable dependency allocations are
   recorded rather than made a reason to replace a backend during feature construction.
2. **Build with tripwires.** Investigate visible stutter, dropped frames, startup delays and hitches in running demos;
   a tracked figure more than about 25% worse than its last comparable baseline; or cost growing faster than its
   algorithm should. Keep historical measurements with source, workload, device and recipe identity. Investigate and
   fix confirmed causes while other building continues. Benchmarks inform development; their targets do not block it.
3. **Tune in a dedicated performance phase.** Once enough engine and demo content exists, profile whole frames on
   real devices, including Mac and iPhone when available, and set targets from that data. That is the AI performance
   push. Correctness, oracle tolerances, lifecycle and agent visual review remain delivery requirements now.

Specialized internals, generated code, SIMD, incremental work and accelerated recipes remain welcome when justified.
Keep public boundaries small, invariants clear, portable fallbacks working and safety checks intact. Claim measured
wins only from measurements; neither best-practice review nor a functioning implementation proves one. Count binary
size, builds, debugging, variants and regeneration as real costs. The campaign machinery below applies when a later
optimization campaign is actually commissioned, not as a prerequisite for each feature.

### Capability investment map

These are coordinated investment fronts within existing owners, not new modules or a second implementation queue.
All capabilities receive cost review as their design is commissioned, including audio/UI and networking. Sequence
follows architectural commitment and shared impact; deeper campaigns follow predicted opportunity and available proof,
not a permanent ranking of fashionable features. Candidate techniques below require scoped selection and qualification.

| Capability / owners | What deserves deep attention | First delivery and evidence |
| --- | --- | --- |
| Runtime and world access — F1/F3/W1 | Repeated lookup, traversal, scheduling, allocation, publication/copying and synchronization; compact layouts and batched/incremental work | I1: local and replica action/state path; operation counts, bytes and allocation/latency behavior across scale and churn |
| Content and memory — F4/W2 | Import/cook/decode, cache invalidation, I/O, dependency traversal, residency, uploads and replacement peaks | I2a: bounded preload and cold/warm source-to-ready path; I2b: separately designed streaming; time to useful content, bytes, stalls and peak memory |
| Geometry and GPU execution — V1/V2/V4 | Visibility, instance/draw preparation, submission, overdraw, geometry/deformation/LOD and reconstruction; GPU-driven candidates where supported | I3a: small render path with scale workloads and preparation evidence; I3b/I4: scene complexity, image stability and end-to-end frame tails |
| Lighting/materials — V3 with E1 | Shadow/indirect/reflection work, light assignment, material specialization, shared transport and valid reuse | D2 fixes required representations before consumers; I3a baseline and later providers prove quality at cost and cost at fixed quality |
| Simulation, animation and world life — P3–P5/E1 | Body/contact queries, pose evaluation, navigation, field updates, vegetation and water/atmosphere consumers; locality, cadence and bounded approximation | D2 designs complete coupled models; I3b integrates selected providers; scale, query age, physical error, sustained cost and visual motion |
| Networking — N1 with W1/F4 | Validation, serialization, repeated group construction, copies, fan-out, queues and resync; preserve atomic outcomes and authority | I1 in-memory fixture then real I2n; application/wire bytes, committed-state age, latency tails and total client/server resource cost |
| Audio/UI/input — A1/U1/P1 | Mixing/spatial updates, text shaping/layout, retained invalidation, dispatch and input-to-feedback path | Relevant I3 slices; deadline misses, visible/audible response, allocation and correctness across focus/lifecycle changes |
| Creator/AI iteration — T1/T2/F4 | Compilation, incremental builds/cook, shader preparation, inspection, restart, tests and retained disk; trustworthy cheap evaluation | I1 establishes scoped fixtures/evidence, I2a/I3a measure real edit-to-useful-result; campaign tooling grows with consumers |

Cross-platform scaling applies to every row. Compare each recipe against portable on the same device (Apple,
Android, NVIDIA, AMD, Intel), browser startup and memory, and sustained mobile energy/thermal behavior where
supported; no desktop score certifies another profile. V1 proves on this M3 Max, Chrome and the iPhone simulator
(function only); other families follow on rented hardware after V1. Preserve
feature-off savings and game semantics. Don't add every candidate backend or technique simply because AI can write it.

### Technique register

Thrum's showcase claim rests on named techniques, not on comparisons with other engines, which depend on too many
uncontrolled factors. The register is the maturity ledger's `technique_register` list, not a new document. Each entry gives:
* the technique and its paper or production source;
* the owner and the recipe or device family;
* its implementation, correctness and visual status; measured comparisons or reference results when available;
* why it would not scale with human authors, for example per-family kernels, exhaustive variant search or
  autotuned parameters kept current by agents.

Pending and unqualified entries are allowed; they make no speed or reproduction claim. Paper reproduction still
requires reference views and an independent oracle. If a measured win disappears, retire that claim and investigate
the regression; do not automatically remove a useful functional path.

### The cost account in each owning design

Keep a compact account beside the capability's mechanism and existing proof plan; no separate mandatory report per
function. Cover the relevant workload range and uncertainty before committing layouts or public boundaries:

- Required outcome, owner/consumers, selected workload/target and quality/correctness envelope; link existing baseline
  targets or label a proposed target explicitly. Capture normal use, scale/churn and meaningful cold/transition cases.
- Work multipliers: per frame/tick/entity/view/light/peer/change, including supposedly cheap lookups and bookkeeping.
  Approximate total work by count × frequency × unit cost; track bytes, allocation and working set separately.
  Elapsed latency follows the dependency critical path and contention, not a sum of overlapping CPU/GPU timings.
- Repeated work and data movement that can be removed, shared, specialized, incrementally maintained or prepared ahead;
  state validity, invalidation, lifetime, fallback and numerical limits. Include worst-case peaks and disabled cost.
- Credible alternatives and expected savings/quality capacity, supported by math, retained research and bounds. Explain
  commitments that would be expensive to change later; distinguish selected mechanisms from candidate optimizations.
- Verification and instrumentation needed for that representation: useful reference/oracle, adversarial consumer cases,
  actual timing/byte/queue boundaries, noise and observer overhead, and target-specific validation where consequential.
- First implementation scope, further opportunity and reasoned stop/revisit conditions. Track modelled, measured and
  integrated claims separately in the existing maturity/evidence system; unknown constants remain unknown.

An existing runnable path supplies a measured baseline. For new code, choose the representation analytically and
build a correct bounded slice with its checks, then qualify predictions. Do not first entrench a knowingly unsuitable
layout merely to create something to optimize. A readable reference may remain test-only.

### Process and completion

During module design, compare performance-conscious representations before committing the API. During implementation,
build the selected representation and its relevant verification together. Small routine changes need only their
actual impact, not a whole-engine campaign. Agent commissions carry the objective, affected owners, cost model,
allowed scope, immutable evaluation basis and resource bounds; the coordinator owns the integrated result.

Screen opportunities widely. Apply deep work where frequency, scale, latency, memory or capacity make it worthwhile,
including cumulative small savings across hot paths. A small gain below full-frame timing noise can retain scoped
kernel/count/byte evidence and integrated non-regression; do not claim an unmeasured whole-engine speedup or reject
it solely because it is individually small. Assess the combined candidate with consumers and realistic contention.

A working implementation or an already-passed budget does not automatically end a commissioned performance effort.
Continue through the meaningful opportunities within its scope and resources. Stop at the agreed outcome/resource
boundary, when remaining gain is unsupported or below uncertainty, or when further variants cost more than their
useful benefit. Record why and what would reopen the decision. No endless search or spending is implied.

Functional delivery requires preserved semantics, stability, lifecycle and required visuals. Performance claims need
evidence, but benchmarks are tripwires during feature construction. Later campaigns use the deeper workflow below.

## What changes in the architecture

The engine already proposes profiles, ownership, traces, replay, cooked products and maturity records. Those are
necessary but insufficient for autonomous optimization. Add an executable performance commission, trusted evaluators,
isolated candidate builds, controlled measurement jobs and a promotion path tied to exact integrated artifacts.
These live in T2/F4 and contributor tooling; they do not form a new runtime subsystem or require every function to
be a dynamically replaceable plugin. Normal static compilation and ordinary branches can compare implementations.

Stable boundaries specify required semantics, not frozen internal layouts. A data structure, algorithm, kernel or
schedule can change substantially inside its responsibility. A promising cross-module representation change gets a
coordinated design commission with producer/consumer ownership and migration implications, rather than a local patch
that quietly breaks the contract. The external behavior can remain stable while several internals change together.

## Two legitimate objectives

1. **Reduce cost at a fixed experience.** Keep the selected workload, semantics, target, quality requirements and
   numerical envelope, then reduce relevant time, memory, loading, energy or artifact size.
2. **Improve the experience at a fixed cost.** Increase accepted detail, scale, responsiveness or visual quality within
   the selected budget. Report the changed quality/capacity explicitly; this is not an equivalent-work speedup.

Correctness, lifecycle and quality floors are eligibility gates, not penalties that a large timing win can outweigh.
After eligibility, compare the useful alternatives across frame tails, memory, startup, energy and target coverage.
Retain a small set of non-dominated choices per device/profile where no single implementation wins everywhere. Do not conceal a severe
mobile regression in a weighted desktop average. A target-specific variant is legitimate only with declared dispatch,
qualification and a portable fallback; variant count, binary size and compilation cost are also budgets.

## An executable performance commission

A campaign pins the following as a versioned record; exact schema/commands belong to D1. This describes required data,
not a claim that a new CLI or file format exists.

| Part | Required contents |
| --- | --- |
| Objective | User-visible outcome, primary metric, absolute target, acceptable regression envelope and minimum useful gain; baseline may already meet the target |
| Scope | Owned code/content/build settings, relevant contracts and consumers; allowed local transformations and separately coordinated architectural alternatives |
| Workloads | Versioned scenes/input traces, scale distributions, initial state, cameras, resolution, visible and offscreen consumers, transition cases and supported feature combinations |
| Targets | Named device/OS/browser/backend, build/compiler flags, target profile and availability; missing hardware yields unqualified evidence |
| Semantics and quality | Event/query/save obligations, numerical accuracy/stability envelope, accepted visual/motion cues, latency/freshness limits and allowed approximations |
| Measurement | Timing boundaries, cold/warm/sustained protocols, repetitions and comparison method, observer overhead, memory scope, exclusions and noise policy |
| Authority | Candidate source area; separately pinned evaluator, references, thresholds and workload generators; revision rules for legitimate evaluator corrections |
| Resources | Wall time, model/token or monetary budget, hardware hours, concurrency, storage retention and stop/review conditions |
| Result | Candidate ancestry, exact source/build/loaded product IDs, correctness/quality results, raw measurements, uncertainty, failures, target coverage and integration disposition |

Pin the common authored workload, inputs, output requirements, target and acceptance contract for comparison.
Each candidate intentionally has different code/binary/shader and possibly cooked-product hashes; record and verify
those separately. Requiring identical cooked bytes would prohibit legitimate content/layout optimizations.

**Illustrative commission, not selected engine budgets:** reduce the p95 GPU critical-path time of a fixed 1440p
outdoor reference scene from a measured 12 ms baseline toward 8 ms on one explicitly named Mac configuration.
Keep the selected visible-detail, shadow, motion and gameplay requirements; cap peak memory and p99 regression;
qualify moving cameras, disocclusion, region changes and cold shader preparation separately. Check Windows/browser/mobile
consumers before advertising broader benefit. An unresolved measurement or quality threshold must be specified before
a campaign can award itself success. Changing final output resolution would be a different commission; internal resolution/reconstruction can vary if the
commission permits it and the same required output quality survives.

## Freedom large enough to produce substantial gains

| Search level | Examples worth considering when attribution supports them | Obligations retained |
| --- | --- | --- |
| Algorithm and representation | Spatial hierarchy, compaction method, light assignment, incremental dependency propagation, reusable visibility results | Completeness, validity/invalidation, query age, overflow/fallback and worst-case work bounds |
| Data and scheduling | Dense/sparse choices, SoA/AoSoA layouts, SIMD traversal, batch sizes, task partitioning, pass fusion and async overlap | Ownership, ordering, memory lifetime, cancellation, CPU/GPU contention and critical path |
| Numerical implementation | Equivalent algebra, lookup/preintegration, bounded approximation, mixed precision, shader workgroup/register strategy | Declared domain and error propagation, long-run stability and perceptual/motion quality; no silent change of physical model |
| Content and compilation | Mesh/texture/layout preparation, compression/decode tradeoffs, shader specialization, selected feature elimination, justified compiler/PGO settings | Provenance, format semantics, build/cook time, startup latency, total peak memory and content readiness |
| Game/target specialization | Fixed material families, known skeleton layouts, target limits, finite profile-specific paths | Source-declared assumptions, compile/cook validation, explicit fallback and complete shader/product demand under A03 |

Bounded specialization can be a major advantage: optimize a real game's selected features rather than paying for every
possible engine combination in every path. It must not hardcode benchmark answers, camera tracks or content IDs to skip
required work. A shipping game may legitimately use a static scene or baked light if its declared product requirements
permit that; the portable dynamic profile cannot inherit its score. No new provider-search framework is required.

Each hypothesis explains the governing work/cost model before building. For example, reducing memory traffic matters
when bandwidth constrains the actual stage; fusion may save reads but increase register pressure or serialized work.
Use lower bounds and estimates to prioritize. Amdahl's `1 / ((1-f) + f/s)` illustrates the ceiling for an accelerated
fraction `f` sped up by `s`, with unaffected fraction `1-f`; it is not a substitute for measuring a CPU/GPU dependency graph with overlapping queues.
Measurements decide compiler/hardware contingencies that analysis cannot settle. Repeated evaluation is appropriate
inside a defined problem; it does not replace the analytical design of the engine or justify random feature layering.

## Evaluation that can sustain a long search

**Trusted semantics.** Keep a readable reference path or independent mathematical oracle where practical, plus
invariants, boundary/randomized cases and consumer scenarios. Test integer/event/identity obligations exactly when
applicable. Floating-point reordering, approximation and changed models are different claims; numerical tolerances
must reflect units, scale, accumulated error and relevant consumers. Differential agreement is not proof when reference
and candidate share a bug. A slower reference can remain test-only without duplicating a production engine.

**Image and motion quality.** Test temporal behavior, coverage, disocclusion, thin geometry, silhouettes, lighting,
material response and history reset, as relevant to the change. Scalar image similarity alone cannot certify beauty
or correct motion. Use matched views/exposure/history, selected cue checks and
[agent visual review](AI-DEVELOPMENT.md#visual-review-and-human-steering) of stills and sequences, including an
independent reviewer agent, for affected appearance. Agents reject failures, shortlist and promote candidates on that
evidence; owner feedback steers later work and never gates promotion. Purely internal changes may reuse applicable
review only with recorded evidence that its scope survives.

**Independent evaluation.** Candidate edits do not alter the evaluator, its measurement clocks/protocol, references, acceptance thresholds,
fixtures or results used to judge that candidate. Keep evaluator execution and result collection outside the candidate's
editable area; use bounded test workers with resource limits. A separate agent in the same writable tree is useful
review but not an integrity boundary. If the evaluator is wrong, change/version it separately and rerun baseline and
candidates; do not quietly move the goalposts. Use evaluator-side launch/artifact checks alongside runtime active IDs; candidate self-report alone is insufficient.
Record compiler inputs and actually loaded binaries/shaders to detect stale-build wins, omitted output and accidental
benchmark-only paths. F1 clock/scheduler internals may themselves change while preserving the time contract.

**Generalization and repeated selection.** Use fast disclosed development cases, fresh randomized/varied validation
inputs and a reserved final workload set. Reserve sets cannot remain independent if repeatedly exposed through scores;
record evaluations and refresh the qualification set after adaptive use. An ordinary local hidden seed is an overfit
check, not a security boundary. Retain unsuccessful results and search history. Independently rebuild and retest finalists
against the pinned original and current best; thousands of noisy trials otherwise select lucky measurements.

**Fair execution.** Benchmark optimized production configurations against a credible current baseline, including
applicable compiler/library optimizations. Consume results and test invalid/tail sizes so dead work elimination or
friendly inputs do not manufacture a win. Match required output semantics and numerical/quality envelopes; record internal precision and work reductions. Distinguish CPU submission, GPU completion,
present cadence, input-to-display latency and sustained throughput. Record deadline misses, queue age/backlog and
completion counts so deferred work is not mistaken for removed work. Instrumentation helps diagnosis; final timing uses
bounded, documented overhead. Capture/profiler-heavy runs can be separate from timing runs.

**Actual hardware.** Record warmup and separate cold compilation/loading from steady state; preserve adverse transitions
and cache invalidation. Use interleaved baseline/candidate runs under comparable conditions and independent run blocks;
frames within a trace are correlated, not thousands of independent samples. Include same-binary A/A controls to measure harness noise and drift. Predeclare practical improvement/noise and
regression criteria; inconclusive is a valid result. Record power/thermal/background load and sustained mobile behavior.
One measurement lease controls the contended machine/device resources, including competing CPU builds, disk I/O
and GPU work; many agents can reason/build on other resources while timing jobs queue. Browser timestamp restrictions or unavailable counters produce explicit evidence limits. Never
substitute a desktop GPU result for a Mac, browser or phone result.

## Coordinated campaigns and promotion

One coordinator owns the whole-scene objective, shared resource budget and accepted baseline. It assigns distinct
hypotheses with shared context, a cost prediction, owner/consumer scope and resource limit. Agents work in isolated
source/build directories and publish candidate records to one append-only campaign history. Reuse already tested
ideas; retain useful diversity rather than assigning many workers the same local tuning problem.

Cheap schema/compile/correctness checks reject invalid candidates before expensive GPU/device runs. Survivors progress
through focused measurements, coupled scenarios, end-to-end workloads and qualification targets. Small meaningful
failures stop a candidate early; a failed narrow tactic need not end a still-promising different hypothesis. Reallocate
work according to expected application benefit per evaluation cost, not the largest isolated kernel speedup.

Local improvements are proposals until integrated. Shared caches, bandwidth, task granularity and resource peaks mean
two independent wins can conflict. Integrate sequentially, test the combined revision and use targeted ablation when
attribution is unclear. Do not multiply local speedups into an engine-wide claim. Retain the prior known-good path and
a concrete revert/dispatch choice. Promotion includes source clarity, licenses/provenance, maintenance ownership,
bounded variants, error/quality evidence and exact target scope; generated code is still maintained code.

Stop on the commissioned budget, verified target attainment, diminishing expected useful improvement, inadequate
measurement, an unresolved semantic/quality decision or an invalidated premise. A blocked candidate cannot enlarge its
scope, relax quality or purchase more compute itself. Return the best **verified** result, unresolved tradeoffs and
evidence; a disappointing honest result is more valuable than a false speedup. Automatic resumable work would require
an explicitly scheduled campaign with these limits; this design starts none.

## First substantial campaign candidates

These are proposed commissions once the relevant executable paths exist, not three simultaneous implementation tasks.
The coordinator chooses the one with the largest supported whole-product opportunity.

| Campaign | Coupled freedoms | Required preserved outcomes |
| --- | --- | --- |
| Geometry, visibility and shadows | Instance layout, hierarchy, mesh/vegetation representation, shadow caster work, upload batches | Required coverage/silhouettes, stable moving views, contact/shadow cues, separate authoritative collision/nav and bounded memory |
| Materials and execution | Finite variants versus branches, light bins, pass fusion, transient allocation and target specialization | Material/lighting meaning and temporal quality; A03 closed for demand, A05 before overlapping-media changes |
| Streaming and background work | Region packaging, compression/decode, preparation order, upload batches, caching and eviction | Required collision/nav/content readiness, startup and p99 latency, bounded queues/peaks, cancellation/suspend/save integrity |

Changing the primitive-operation count is often the whole point. Verify equivalent required behavior and cues rather
than forcing identical draw/triangle/job counts. Keep enough workload counters to detect missing work and explain the
saving; legitimate culling, fusion or reuse should reduce them. A changed quality tier is separately labeled.

## Ownership, tooling and skill shape

The [four paper campaigns](PERFORMANCE-WALKTHROUGHS.md) exercise Mac rendering, Windows animation, browser delivery
and native mobile simulation. They refine the commission and promotion protocol without supplying benchmark results.
Pin a meaningful completion event (delivered response, required region readiness or solved motion), plus intermediate
milestones. Candidate handoffs include consumer/resource deltas: validity, peak/retained bytes, shared bandwidth/work,
freshness and shifted cost. Report completed, pending and cancelled work and oldest outstanding obligations at each
window boundary; a shorter timed interval cannot hide deferred debt or unreported loading waits.

When compatible changes interact, compare baseline, each candidate and their coherent combination; targeted ablation
cannot run an invalid mix of representations. Record exact combined provenance. Distinguish local resource savings,
integrated target attainment, accepted quality/capacity gains and qualified target-specific variants. Useful narrower
results can be retained with their honest scope, without pretending they satisfy a broader commissioned objective.
Requalification follows affected consumers and selected target coverage rather than blocking every scoped win on all
future platforms. Provider-specific bounds, cache lifetime, first-play and route-deadline mechanisms remain detailed
design tasks identified by the walkthroughs, not questions an optimizer may settle by changing a score.

| Existing owner | Addition needed |
| --- | --- |
| T2 | Versioned commission/evaluator identity, candidate registry, benchmark runner, hardware leases, raw/summary results, promotion report and regression tracking |
| F1/V1 | Bounded CPU/GPU timing hooks, lifecycle/device metadata, source-to-job/pass attribution and truthful capability limits; instrumentation can be excluded from shipping profiles |
| F4/F2 | Exact cooked/shader/configuration lineage, isolated content-addressed builds, finite variants and declared specialization assumptions; reuse A03's demand mechanism |
| Module owners | Cost models, domain/error/freshness obligations, reference or invariant checks, conformance scenarios and valid implementation freedoms |
| Coordinator | Whole-scene budgets, cross-module changes, target/quality tradeoffs and sequential integration; gameplay/art intent remains owned by the game/user |

Expose these through the same local CLI and structured jobs used by people and agents. An optional remote worker
adapter can add hardware capacity later; do not build a large service before one local campaign works. No paid model,
CUDA-only runtime or online self-modifying engine is required. Search runs during development/cooking; selected
implementations ship as ordinary pinned artifacts. Runtime adaptive quality remains a separate bounded F2 policy.

Follow Astra's concise-skill approach: keep discovery, judgment and completion criteria in existing performance/
diagnosis guidance; executable targets and evaluator mechanics belong in data/tools. A dedicated optimization skill
is justified later only if repeated campaigns need distinct routing, and must use actual commands. Do not place the
whole protocol in every agent's mandatory context. Mechanical rules cannot prove numerical equivalence or beauty.

## Delivery order and evidence

The [build plan](../BUILD-PLAN.md) owns current sequencing; these stages describe how campaign tooling grows.

- **D0:** select first representative workload/device and the cost-versus-quality objective; identify honest budgets.
- **D1/I1:** specify campaign/result records and trusted evaluation boundary; implement one local CPU/headless path,
  source identity, isolated builds and reference checks where an authorized implementation exists.
- **D2/I2/I3:** add selected content/GPU/coupled workload paths, numerical and visual obligations, A03-compatible shader
  identity, and one modest baseline-to-candidate run before automating broader search.
- **I4:** independent finalist runs on actual selected devices, sustained/lifecycle qualification and regression tracking;
  add worker capacity only when measurement queue cost justifies it. A04/A05 still gate their dependent model claims.
- **Later campaigns:** broaden algorithmic/architectural freedom and target specialization based on proven evaluator
  coverage; retain the whole-scene coordinator and bounded budgets.

Start with optimization-friendly construction and evaluation hooks. Ambitious campaigns need a correct executable
baseline and meaningful workload; no need to wait for the entire engine, and no basis yet to promise a speedup.
This future-engine proposal does not change Broadsides' current test charter or add benchmark gates to its fast suite.
The existing maturity ledger records the design; later evidence will distinguish local candidate, integrated result,
agent visual review and per-target performance. More trials do not themselves increase maturity.

## Research basis and limits

- [AlphaEvolve overview](https://deepmind.google/blog/alphaevolve-a-gemini-powered-coding-agent-for-designing-advanced-algorithms/),
  opened 1 November 2026: model proposals, automated evaluators and retained candidate programs support iterative
  algorithm discovery. This motivates the development workflow; its reported gains are not predictions for our engine.
- [KernelBench](https://arxiv.org/abs/2502.10517), opened 1 November 2026: evaluates both correctness and speed of generated
  GPU kernels, and reports benefits from execution/profiling feedback. ML/CUDA workloads do not establish wgpu game
  rendering, Mac/mobile portability or accepted image quality.
- [KernelBench-Verified](https://arxiv.org/abs/2607.16241), opened 1 November 2026: reports that realistic baselines,
  broader correctness distributions and memory accounting can overturn apparent improvements. This motivates separate
  evaluator ownership, fresh qualification inputs and multi-resource evidence, not a blanket claim that optimization fails.

These sources support the opportunity and evaluation risks. The proposed engine-specific architecture is our synthesis.

Additional primary measurement references, inspected by the measurement agent on 1 November 2026:
[WebGPU specification](https://www.w3.org/TR/webgpu/) (timestamp feature/limits),
[PIX timing captures](https://learn.microsoft.com/en-us/windows/win32/direct3dtools/pix/articles/timing-captures/pix-timing-captures)
(live CPU/GPU overlap),
[Apple GPU counter analysis](https://developer.apple.com/documentation/xcode/analyzing-apple-gpu-performance-using-counter-statistics)
(profiling can alter pass execution), and [Android game optimization](https://developer.android.com/games/optimize)
(sustained device/thermal conditions). Availability of these tools does not implement our cross-platform harness.

Independent design inputs: [measurement](../evidence/handoffs/2026-11-01-engine-ai-performance/MEASUREMENT.md) and
[search scope](../evidence/handoffs/2026-11-01-engine-ai-performance/SEARCH.md). Coordinator synthesis distinguishes
proposal search from independent qualification and preserves source/evaluator and whole-engine ownership.

## Storage across a campaign

The [storage lifecycle](STORAGE-LIFECYCLE.md) adds peak disk reservations, live free-space monitoring and bounded
reusable worker roots before bulk search. Keep compact proposal histories and enough raw evidence/inputs for the
baseline, current best, finalists and decisive failures. Release only owned dominated bulk after evaluation and
publication leases end; mark expired artifacts unavailable. Hashes alone cannot reproduce deleted evidence.
Verified worker resets preserve candidate/evaluator isolation. I1 admission precedes autonomous bulk search.

The [campaign closeout walkthrough](LIFECYCLE-WALKTHROUGHS.md#l04--finish-an-ai-optimization-campaign-without-retaining-every-attempt)
distinguishes full selected rebuild/evidence sets from historical summaries. Verify adopted dirty patches and unique
generated inputs before removing a worker. A retained comparison or minimized open-failure reproducer is a legitimate
retention disposition; cleanup need not wait for that separate decision. Final integrated qualification still needs
its own completed run and evidence, within the authorized campaign budget.

## Admitted candidate handoff

A campaign candidate pins C4's immutable admitted source, including dirty code and generated selected inputs. Handoff
exports that exact manifest and payloads or verifies a matching VCS/LFS closure; a later clean checkout does not prove
it contains the evaluated candidate. F4/T2 validate recipient rebuild inputs before releasing local retention. Ordinary
candidate edits use files, admission and restart. This changes no evaluator-isolation, quality or whole-engine
promotion requirement, and no automatic publication/cleanup action is implied by a performance result.
