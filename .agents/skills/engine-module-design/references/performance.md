# Performance as a system property

Use during architecture selection, quality design, optimization and final qualification. Preserve the experience;
measure the actual bottleneck instead of treating a faster calculation as a faster application.

## Establish cost and evidence

Identify the irreducible shared/source floor and relevant compute, submission, GPU, bandwidth, allocation, resident
memory and preparation costs. Attribute work to owners and update frequency. State actual product budgets when known;
unknown hardware does not prevent selecting viable reductions, but it prevents device-specific certification.

Measure a representative end-to-end workload before optimizing. Retain build/loaded-artifact identity, inputs, quality,
viewport/DPR, device/backend, visibility, warmup and timing conditions. Record time-series cadence and tails as well as
central values; periodic failures can disappear in a short median. Include sustained use and consequential transitions.
State sampling windows, overwritten records and exclusions; keep observer work bounded and avoid captures/probes that
contend with the workload during timing. Avoid concurrent benchmark jobs when they would confound the comparison.

Distinguish CPU elapsed stages, driver waits, GPU execution, callback cadence and displayed latency. A wait location
is evidence about where execution blocked, not its ultimate cause. Label logical resource estimates separately from
measured residency. Synthetic CPU slowdown does not emulate a low-end GPU/device. Capability fallbacks, resource
restoration and high-tier costs need their own scope of qualification.

## Remove work before weakening fidelity

Profile, form a concrete hypothesis and compare it against the retained baseline. Useful opportunities include exact
reuse with complete invalidation, invariant preparation, transform/symmetry reuse, compact data access, batching,
shared pass/resource ownership and removing duplicate draws or uploads. Select them from the measured dependency
path; an extra full-screen intermediate can reduce submissions while increasing memory or fill cost.

Prefer a clear local data structure or helper to a general framework unless several real consumers justify it.
Hoisting, specialization and reuse need explicit validity/lifetime conditions, including reset, changing inputs and
resource restoration. A cheaper zero-input path must preserve delayed emissions, persistent history and source-off
behaviour. Check numerical equivalence with appropriate identities, independent references or tolerances; label
intentional approximation separately from floating-point reordering.

Measure useful isolated kernels to understand attribution, then integrate and rerun the assembled workload. Repeat
paired comparisons under matched conditions; alternate/reverse order when scheduling or thermal drift matters.
Preserve adverse measurements used in comparisons and compact records of rejected experiments; this does not require retaining every rejected build or full trace. Multiple simultaneous changes limit attribution. Keep gains only
when the relevant measured cost improves while the required experience is preserved; a faster microbenchmark can
regress frame pacing or another tier.

## Rank reductions by saved work and perceptual cost

Separate exact optimizations from fidelity changes. For each proposed reduction, name what is disabled, replaced,
simplified or updated less often; the actual work/residency saved; important cues at risk; dependencies; and transition
or preparation requirements. Hiding a consumer may leave its producer and buffers fully active.

Rank reductions for the actual bottleneck and intended experience. Compare held inputs and moving use to find the
least visible loss per useful saving. Memory pressure, fill cost and CPU cost can require different orders. If controls
are changed together, do not claim their individual visual/cost ranking was measured. Keep an initial ordering a
hypothesis until qualified, and revise it when hardware or scenes change the bottleneck.

Preserve the experience floor, meaningful interactions, physical clock, persistent identity/history and downstream
query meaning. Graphics quality must not silently change physical propagation or erase tails. A simulation/world
fidelity change needs an explicit profile and state-transfer contract. Specify which controls change live, which
need rebuilding, and how detail moves between resolved, authored and statistical representations without duplication
or discontinuity. Use [system diagnosis](system-diagnosis.md) for consequential reconstruction handoffs.

## Sustained agent optimization

When a sustained optimization campaign is commissioned, pin its objective, workload/target, semantic and quality
constraints, evaluator revision and resource limits. Give agents room to change algorithms, representations, schedules
and cooked products, not just local code. Coordinate consequential contract changes with affected owners. A cost/error
model narrows credible alternatives; repeated measurement tests compiler/hardware contingencies within that design.

Keep candidate edits separate from the evaluator, references and thresholds. Track candidate ancestry and failures;
screen cheaply, then independently rebuild/retest finalists on fresh workloads and the integrated product. Repeated
search can select noise or overfit known cases. Keep hardware measurement jobs exclusive on contended resources.
Separate equivalent-experience cost reductions from accepted quality/capacity improvements at a fixed budget.
Stop within the commission's limits and retain the best verified result, with tradeoffs and unqualified targets explicit.
Apply the repository's [storage lifecycle](../../../../docs/ASSET-LIFECYCLE.md): baseline/finalist source,
evaluator and decisive evidence survive; owned dominated bulk can retire at checkpoints. Keep exact unpublished
source deltas and unique inputs needed for the selected result, not merely a commit label. Before a worker is retired,
verify its adopted changes and evidence in their destination; local speed does not establish integrated qualification.
For this engine's planned tooling and ownership, see [AI performance](../../../../docs/design/AI-PERFORMANCE.md).

## Requalify after integration

Compare the final loaded build with the retained baseline, ordinary visuals/motion and required lifecycle/fallback
paths. Report percentiles, maxima, duration, workload and memory scope rather than a single speedup. Match simulation
elapsed time to intended physical time and expose discarded/deferred work; dropping simulation is not a free speedup.
Investigate tails and long-run allocation/resource growth when observations warrant it.

Review optimization correctness and claims independently when available, including whether caching, batching, depth/
composition changes or lower sampling introduced a new artifact. State what improved, what overlapped or regressed,
what was rejected and what remains unmeasured. Native/headless results do not certify a different client or user tab.
Deliver a maintainable selected path and reproducible compact evidence, retaining only useful rebuild inputs and
comparisons under repository storage rules.
