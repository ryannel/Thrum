# A0 decision: component storage for the V1 world

3 November 2026. **Status: three-day measured selection, not a final library commitment.** This reopens the private hecs 0.11.2 selection in [ARCHITECTURE-DECISIONS](../ARCHITECTURE-DECISIONS.md) because the existing opaque-record world has no simulation query or component change ticks. C4's one world authority, stable IDs, checked external handles, fixed-step time and atomic validated edits remain binding. The ECS entity identifier is private and never enters a save, source asset or packet.

## Candidate and workload

Compare [hecs](https://docs.rs/hecs/latest/hecs/), [bevy_ecs](https://docs.rs/bevy_ecs/latest/bevy_ecs/) as a standalone library, and a minimal custom archetype/SoA store. Use the same deterministic 10,000-entity fixture with 1,500 continuously moving entities, 100 spawn/despawn pairs per second, transform/velocity/health/ownership/visual components, and a 4-peer extraction. Include sparse components, archetype transitions, a 1% changed-transform tick, and rollback of 8–16 ticks. Run native release and Wasm serial builds. The custom candidate is a comparison, not advance permission to grow a generic framework.

The spike has three days of bounded work: day 1 adapters and correctness; day 2 native/Wasm timings and allocation counts; day 3 rollback, change-detection and parallel-query assessment. A single adapter boundary must expose spawn/despawn, typed query, component change tick, snapshot/restore and validated command apply. The candidate must preserve stable-ID mapping and generation checks. Compare total tick p50/p99, allocations, bytes copied for rollback, extraction cost, query throughput, build size and the amount of adapter code. Record per-candidate exact versions and one compact timing table. A missing capability is a finding, not a zero-cost assumption.

## Selection rule

Prefer the simplest candidate that sustains the V1 workload and permits zero steady-tick allocations after warmup. Bevy ECS has integrated change detection and scheduling, but [change-filtered queries still scan matched archetypes](https://docs.rs/bevy_ecs/latest/bevy_ecs/change_detection/index.html); measure the 1% change case instead of assuming O(changes). Hecs is smaller, but its [change tracker](https://docs.rs/hecs/latest/hecs/struct.ChangeTracker.html) clones/compares visited values and it has no integrated scheduler; measure the extra machinery. A custom SoA may win the narrow path but must account for archetype transitions, rollback and maintenance cost. The selected store remains private behind W1; a public game component API is chosen after the spike.

Reject any candidate that cannot compile in the selected Wasm profile, cannot restore a deterministic tick with the same stable IDs, or requires whole-record cloning for each moving component. A clean correctness pass is required before comparing speed. Do not replace the I1 authority until the new path ports its invalid, late and duplicate input regressions. If none qualifies, narrow the custom adapter and remeasure; do not lock in the old opaque `T` path by default.

## First measured pass; selection remains open

The [3 November spike](../../evidence/v1-workload/A1-STORAGE-SPIKE-2026-11-03.md) passes a deterministic 16-tick replay and Wasm compile for all three, and measures the narrow fixed-column path substantially faster than the generic stores on indexed motion and full copies. Its `Option<Ownership>` is **not** a general archetype transition, and its direct fixture index omits the production stable-ID and checked-handle layer. The next comparison must exercise dynamic component add/remove and an integrated A1 schedule before this record can select storage. Do not treat full-row copy as the N1 transport design.
