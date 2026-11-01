# Protected fixed-slot path audit

1 November 2026. This audit follows accepted input through a selected sealed fixed-slot, inline-output authority, its required finite local replica, the native callback collection and acknowledgement. It does not qualify structural spawning, tick-created variable payloads, arbitrary game-private scratch, remote queues or named devices.

The audit found three uncharged engine paths and closed them for this profile. `PreparedEdits::Many` can own a large vector even when its length fits the declared edit count; the protected authority now treats that variant as a terminal game-contract fault before any world edit. The prototype `baseline()` convenience path could allocate uncharged result and state vectors through the host's read-only authority; `try_baseline()` now returns `Unqualified` in protected mode, while `try_baseline_charged()` remains available. The host's two built-in system stages used an uncharged retained vector and allocation-based validation scratch; their ownership, dependency and conflict checks now produce an inline two-stage order with no scheduler heap allocation. The prototype and ordinary charged hosts use the same validated order.

The shared `PublicationPool` itself has one fixed Arc control allocation. `PublicationPool::control_allocation_bytes()` reports its modeled requested size, including strong and weak counters. This setup cost is explicitly outside the pool's usable owner-byte `limit_bytes()` and `used_bytes()`; cloning a pool shares that control allocation. The logical cap also excludes allocator metadata and game-private allocations. It covers the selected profile's admitted dynamic world, history, snapshot, group, replica and callback owners, not every byte of a process or of a host language runtime.

The combined test admits the maximum eight pending inputs from each of two peers, each touching a different one of 16 snapshot pages in a 512-object world. After admission, another owner reserves every remaining pool byte. The callback publishes one tick with all 16 dirty pages and results, the required replica sees every replacement, acknowledgements succeed, and final release returns pool usage to zero. Separate tests cover rejected vector edits, charged-baseline availability, inline stage order/invalid declarations and the fixed control-cost boundary. `RUSTC_WRAPPER= ./dev check` passes.

The independent review agent re-audited the selected profile after the fixes and found no remaining engine-owned dynamic allocation hole in this scope. It judged the fixed-slot owner-byte gate ready to close with the exclusions above. The broader process and game-work gates remain open.

The [release fixture](../../../crates/thrum-core/examples/i1_charged_replica_cost.rs) compared commit `b6b6d2e` with this change in A/B/B/A order, with separate Cargo target directories and distinct executable SHA-256 hashes: before `bf1a1cd89a8531967a6a1e0c52b51dcb385ac934cc0f9ccc72d9eb780499ba11`, after `d1b7a60c8ca2273492e0aa64bb3f983623ac43944aaa9263632f15557175c14d`. The raw runs are [before A](protected-gate-before-a.txt), [after A](protected-gate-after-a.txt), [after B](protected-gate-after-b.txt) and [before B](protected-gate-before-b.txt). Complete protected submit/callback/acknowledge p50 nanoseconds were:

| Workload | Before A / B | After A / B |
| --- | ---: | ---: |
| 64 objects, one change | 709 / 667 | 708 / 750 |
| 4,096 objects, one change | 1,000 / 959 | 958 / 958 |
| 64 objects, sixteen changes | 2,791 / 2,292 | 2,875 / 2,458 |
| 4,096 objects, sixteen changes | 2,833 / 2,833 | 2,750 / 2,833 |

Most ranges overlap; the 4,096-object one-change after medians were 1–42 ns below the two before medians. These samples do not establish a speed change. The local macOS/aarch64 runs use synthetic callback times, sequential modes and no named-device or allocation-counter qualification. The broader memory-plan items stay open for variable outputs, private rule work, structural growth and real multiplayer transport.
