# A1 storage spike: first measured pass

3 November 2026, Apple M3 Max, arm64 macOS, Rust 1.99.1 release. [Selected source, exact lockfile and raw local measurements](world-storage-spike/README.md) are retained in Git. Exact libraries: hecs 0.11.2, bevy_ecs 0.20.1 with `std` only, Divan 0.1.21. Three adapters passed 600 deterministic ticks against the same 10k-ID oracle, sparse ownership changes, 100 spawn/despawn pairs/s, 1% changed-position detection, and a 16-tick snapshot/rewind/replay. All three compiled for `wasm32-unknown-unknown`; that is a compile check, not a browser runtime measurement. Each 16-tick full-row ring occupies 3.84 MB before private-store overhead.

| Private store operation | Custom fixed-column SoA | hecs | bevy_ecs |
| --- | ---: | ---: | ---: |
| Tick p50, 1,500 indexed movers + churn | 0.42 µs | 1.96–2.0 µs | 4.33–4.42 µs |
| Tick p99, three runs | 0.54–0.58 µs | 2.46–3.04 µs | 4.92–8.13 µs |
| Four full peer copies p50 | 42–44 µs | 57–58 µs | 53–58 µs |
| 1% changed extraction, Divan median | 0.68 µs | 0.78 µs | 3.08 µs |
| Toggle 100 sparse components, Divan median | 0.013 µs | 8.54 µs | 7.79 µs |
| Full 10k restore, Divan median | 8.71 µs | 388 µs | 417 µs |
| Allocation calls in 600 ticks after 600 warm ticks | 0 | 0 | 0 |

The p50/p99 rows use three serial local runs, each with 600 warm and 1,200 sampled ticks; they remain local observations, not a regression gate. The other rows are a separate 0.25 s Divan run. Hecs uses a prepared mutable view and Bevy a cached mutable query for indexed motion; both library stores query optional ownership with the other snapshot fields. Initial unprepared/per-entity versions were discarded because they measured adapter mistakes. The custom store uses direct fixed-ID column indexes, and its optional ownership column does **not** implement generic archetype transitions; its sparse-toggle result is therefore a lower bound. The library stores use private entity handles cached by stable ID; they rebuild private handles on full restore. Every snapshot currently copies whole records, even for four peers, so these numbers do not qualify N1's delta transport or production rollback. Bevy's changed row uses its native `Changed<Position>` filter and its standalone adapter rotates removal trackers each tick; the others use explicit tick stamps. Heap counts are asserted for all three in a process-local allocator test, not full-process engine CI.

**Decision:** use hecs 0.11.2 as the private A1 backend. A scoped two-thread test simultaneously mutated disjoint Position and Health components through shared-world queries, supporting F1's graph direction; the actual F1 worker executor is still unbuilt. `cargo tree -e normal` found 4 unique package nodes for hecs versus 60 for the minimal Bevy profile (including each root). The custom result is a lower-bound specialization, not a complete ECS. The production stable-ID/checked-handle adapter, end-to-end A1 tick, delta publication and named-device results still require integration measurements. The read-only reviewer's two Bevy tracker-maintenance findings were fixed before the retained measurements; final review found no remaining issue.
