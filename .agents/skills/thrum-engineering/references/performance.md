# Performance

The owner's policy is to make best-practice decisions for high performance up front, lay the best foundation you
can, build with tripwires, and tune later. Use measured performance tripwires. Record before-and-after figures, and
keep the experience intact. The background is in `docs/design/AI-PERFORMANCE.md` and
`docs/design/decisions/HARDWARE-NATIVE-ENGINE.md`.

## Design for cost from the start

Write a cost account before you choose representations. The account covers counts, frequency, scale and bytes, the
working set and critical path, and every transfer and wait. Include the cheap work repeated every frame, not only the
obvious hot spots. The account goes in the owning design (`AI-PERFORMANCE.md#the-cost-account-in-each-owning-design`
shows its form). A new module doesn't need a slow first version to justify a fast design.

A mechanism that is heavy by design is a design defect. It might need offline sample counts, per-frame
full-resolution copies, redundant transforms or a CPU–GPU round trip to reach the look. Such a mechanism is not
finished, and a look that exists only at seconds per frame is a reference, not a product.

Specialised internals are welcome when their benefit is real and checkable: generated code, compact layouts,
precomputation, incremental work and target-specific paths. Keep their invariants, validity and fallback explicit,
with a comparison against the portable path that shows the output is the same to the eye, or exactly where it is
meant to be exact. Accelerated paths come from the capability registry, keep a portable fallback, and are
promoted only by a measured win. Record each named technique and its source in the technique register
(`AI-PERFORMANCE.md#technique-register`).

## Per-frame code

These costs recur in the code today. Each one came from upload or rebuild policy, not from local code:

- **Re-uploading what didn't change.** `thrum-sea` zeroes and re-uploads a 6.3 MB spectrum every frame (H16).
  Upload only what changed, and write the static part once.
- **Throwing away prepared data.** The GPU loader rebuilds meshlet indices on the CPU instead of uploading the cooked
  section (H18).
- **Rebuilding on any input change.** Every sun-direction change rebuilds the sky, diffuse light and all seven
  reflection mips (M31). Amortise the rebuild over frames, or make it incremental.
- **Readbacks to make progress.** Hosts force a full pixel readback just to advance uploads (M01).
- **Rendering when nothing changes.** Still scenes render at the full display rate forever (M58).

Reuse buffers and arenas, store iterated data as columns, and validate once. The standard `HashMap` uses SipHash.
For an internal map on a hot path, a fast non-cryptographic hasher is the better choice; register it like any
dependency.

## Measure honestly

- **Measure the existing path before claiming a gain.** Use a representative end-to-end workload. Record the build,
  inputs, quality, viewport, device, backend, warm-up and timing conditions. Report tails and cadence, not only
  medians.
- **The GPU here is shared by several agents.** Label timings taken while it was contended. Repeat identical builds
  before attributing a change. Keep exclusive runs, under the `gpu` lock, for decisions that need them.
- **Keep kinds of time apart:** CPU stages, driver waits, GPU execution, callback cadence and displayed latency. Where
  execution blocked doesn't say why it blocked.
- **Rerun after integrating.** An isolated kernel explains attribution, but only the assembled workload shows the
  effect. Keep adverse results, and compact records of rejected experiments.

## Remove work before weakening fidelity

Exact savings come first:
- reuse with complete invalidation;
- invariant preparation;
- compact access and batching;
- shared passes and resources;
- removing duplicate draws, uploads and copies.

When you do reduce fidelity, state for each reduction what is simplified, the work saved, the cues at risk and the
transitions it affects. Hiding a consumer can leave its producer fully active.

Some things never change for performance:
- **Delayed and persistent effects survive a cheaper path.** A wake still persists after the boat stops.
- **Graphics quality never changes physics.** Propagation, history and query meaning stay the same at every quality
  setting. A change to simulation fidelity needs an explicit profile and a state-transfer contract.
- **Simulation time isn't dropped to save work.** Catch-up after a hitch is bounded and recovers; it doesn't freeze.

## Optimisation campaigns

When one is commissioned:

- **Fix the terms first:** the objective, the workload, the quality constraints, the evaluator revision and the
  resource limits.
- **Let candidates change anything that matters:** algorithms, representations, schedules and cooked products.
- **Keep the evaluator and its thresholds apart from the candidates.** Screen cheaply. Rebuild and retest the
  finalists on fresh workloads, because a repeated search can select noise.
- **Keep the baseline and finalist sources and the decisive evidence**, under `docs/ASSET-LIFECYCLE.md`.

Report what improved, what regressed, what was rejected and what remains unmeasured. Native or headless results
don't certify another client.
