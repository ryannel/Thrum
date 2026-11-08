# Performance as a system property

The owner's policy: make best-practice decisions for high performance up front, lay the best foundation you can, build
with tripwires, and tune later. Don't invent numeric budget gates. Record before-and-after figures, and keep the
experience intact. See [AI performance](../../../../docs/design/AI-PERFORMANCE.md) and the
[hardware-native decision](../../../../docs/design/decisions/HARDWARE-NATIVE-ENGINE.md).

## Design for cost from the start

- **Write a cost account before choosing representations.** Cover:
  - counts, frequency, scale and bytes;
  - working set and critical path;
  - transfers and waits;
  - frequently repeated cheap work as well as obvious hot spots.

  A new module doesn't need a slow implementation first to justify a fast design.
- **Treat heavy-by-design as a design defect.** If the chosen mechanism needs offline sample counts, per-frame
  full-resolution copies, redundant transforms or CPU–GPU round trips to reach the look, the design isn't finished. A
  look that exists only at many seconds per frame is a reference, not a product.
- **Specialize where it pays.** Generated code, compact layouts, precomputation, incremental work and target-specific
  paths are welcome when the benefit is real and checkable. Keep the invariants, validity, fallback and an independent
  oracle explicit. A simple public model can hide sophisticated internals.
- **Use registry-selected recipes.** Accelerated paths, a portable fallback, and promotion only by a measured win.

## Measure honestly

- **Measure an existing path before claiming a gain.** Use a representative end-to-end workload, and record the build
  identity, inputs, quality, viewport, device and backend, warm-up and timing conditions. Report tails and cadence, not
  only medians.
- **Treat a shared GPU as noisy.** Several agents share this machine's GPU. Label contended timings, repeat identical
  builds before attributing a change, and keep exclusive hardware measurement for decisions that need it.
- **Keep different kinds of time separate.** CPU stages, driver waits, GPU execution, callback cadence and displayed
  latency are different things. A wait location shows where execution blocked, not why.
- **Compare fairly.** Use paired, matched runs, and alternate their order when drift matters. Several simultaneous
  changes limit attribution. Watch long runs for growth in allocation or resources.
- **Rerun after integrating.** Isolated kernels explain attribution; only the assembled workload shows the effect.
  Keep adverse results and compact records of rejected experiments.

## Remove work before weakening fidelity

- **Exact savings first.** Look for:
  - reuse with complete invalidation;
  - invariant preparation;
  - symmetry;
  - compact access and batching;
  - shared ownership of passes and resources;
  - removing duplicate draws, uploads and copies.
- **Make fidelity reductions explicit.** For each one, state what is disabled or simplified, the work saved, the cues
  at risk and the transitions. Hiding a consumer can leave its producer fully active.
- **Rank reductions by visible loss per saving.** Rank them for the actual bottleneck, by the least visible loss per
  useful saving. Controls changed together don't establish their individual ranking.
- **Keep source-off behaviour.** A cheaper zero-input path must keep delayed emissions, persistent history and source-off
  tails, such as a wake that persists after the boat stops.
- **Keep graphics quality away from physics.** Graphics quality must never silently change physical propagation,
  history or query meaning. A simulation fidelity change needs an explicit profile and a state-transfer contract.
- **Keep fixed time on hitches.** Dropping simulation time is not a free speedup. Catch-up must be bounded and must
  recover after a hitch, not freeze.

## Sustained optimization campaigns

When one is commissioned:
- **Pin the terms.** Fix the objective, workload, quality constraints, evaluator revision and resource limits.
- **Give room to change.** Let agents change algorithms, representations, schedules and cooked products, not just
  local code.
- **Keep the evaluator separate.** Keep candidates apart from the evaluator and its thresholds. Screen cheaply, then
  rebuild and retest finalists on fresh workloads, because repeated search can select noise.
- **Keep the right evidence.** Retain the baseline and finalist sources, the evaluator and the decisive evidence under
  the [storage lifecycle](../../../../docs/ASSET-LIFECYCLE.md). Before retiring a worker, verify its adopted changes
  in their destination.

Report what improved, what regressed, what was rejected and what remains unmeasured. Native or headless results don't
certify another client.
