# A1 storage spike: first measured pass

3 November 2026, Apple M3 Max, arm64 macOS, Rust 1.99.1 release. Isolated source and lockfile: `tmp/world-storage-spike/` (local ignored scratch until selection). Exact libraries: hecs 0.11.2, bevy_ecs 0.20.1 with `std` only, Divan 0.1.21. Three adapters passed 600 deterministic ticks against the same 10k-ID oracle, sparse ownership changes, 100 spawn/despawn pairs/s, 1% changed-position detection, and a 16-tick snapshot/rewind/replay. All three compiled for `wasm32-unknown-unknown`; that is a compile check, not a browser runtime measurement. Each 16-tick full-row ring occupies 3.84 MB before private-store overhead.

| Private store operation | Custom fixed-column SoA | hecs | bevy_ecs |
| --- | ---: | ---: | ---: |
| Tick p50, 1,500 indexed movers + churn | 0.375 µs | 14.6–15.5 µs | 9.8–10.3 µs |
| Tick p99, three runs | 0.54–1.25 µs | 22–118 µs | 13–32 µs |
| Four full peer copies p50 | 42–44 µs | 272–277 µs | 157–165 µs |
| 1% changed extraction, Divan median | 0.71 µs | 1.70 µs | 3.42 µs |
| Toggle 100 sparse components, Divan median | 0.014 µs | 8.79 µs | 7.54 µs |
| Full 10k restore, Divan median | 8.7 µs | 395 µs | 418 µs |
| Allocation calls in 600 ticks after 600 warm ticks | 0 | 0 | 0 |

The p50/p99 rows use three serial local runs, each with 600 warm and 1,200 sampled ticks. One run suffered large tail outliers, so p99 is observational only. The other rows are a separate 0.25 s Divan run. The custom store uses direct fixed-ID column indexes, and its optional ownership column does **not** implement generic archetype transitions; its sparse-toggle result is therefore a lower bound. The library stores use private entity handles cached by stable ID; they rebuild private handles on full restore. Every snapshot currently copies whole records, even for four peers, so these numbers do not qualify N1's delta transport or production rollback. Bevy's changed row uses its native `Changed<Position>` filter and its standalone adapter rotates removal trackers each tick; the others use explicit tick stamps. Heap counts use a process-local allocator test, not full-process engine CI.

No representation is selected yet. Day 3 must test arbitrary component add/remove and scheduler interaction, account for code/dependency cost and the stable-ID/checked-handle adapter, then compare end-to-end A1 tick plus publication. The custom result is a lower-bound specialization, not a complete ECS; the hecs p99 spread makes a single-run regression threshold unsafe. Production A1 remains separate and unmodified by this experiment.
