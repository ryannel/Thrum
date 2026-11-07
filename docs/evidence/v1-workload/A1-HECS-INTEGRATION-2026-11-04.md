# A1 private hecs integration

4 November 2026. Apple M3 Max, arm64 macOS, Rust 1.99.1, Divan 0.1.21 release profile. This is a local CPU diagnostic, not a complete V1 device budget. The private backend is hecs 0.11.2; public stable IDs and checked handle generations remain engine owned. The same input, simulate, resolve and publish stages now read and publish typed hecs components. A borrowed view still cannot cross a world mutation. The public component scan visits one hecs query; snapshot copying shares that path. Position publication uses one mutable query instead of a lookup per moving entity.

| 10k entities, 1.5k moving | Median | Fastest–slowest, 100 samples |
| --- | ---: | ---: |
| Serial component tick | 51.83 µs | 44.54–62.99 µs |
| Visit all typed components | 51.93 µs | 49.87–61.08 µs |
| Four full-record snapshots | 222.8 µs | 204.5–305.2 µs |

The earlier provisional fixed-column path measured 20.83 µs per tick and 150.0 µs for four full copies on this host. These are separate diagnostic runs, not a controlled speedup comparison. The first unoptimized hecs tick in this branch measured 70.62 µs; batched position publication measured 48.66 µs in one run and 51.83 µs in the final run. The selected backend costs more than the fixed-column candidate while supporting sparse archetypes and arbitrary typed components. A2 must replace full-record consumer copies with change-based extraction.

Ten focused simulation tests pass, including invalid, duplicate and stale input, atomic rejected ticks, checked reuse, four sparse component archetypes, 1,000 churn pairs during 600 ticks and 32,000 unique-ID replacements. Counting-allocator assertions observed **zero allocations after representative warmup** for the churn plus four-copy path and the long replacement path. The first use of hecs structural/query storage allocates, so a static-tick-only warmup is insufficient. `cargo check -p thrum-core --target wasm32-unknown-unknown` passes with the pinned rustup compiler; browser runtime was not run. The complete repository gate passes.

Open: deterministic 8–16 tick restore/replay in the production adapter, F1 worker execution, delta publication, complete four-peer game, Chrome and iPhone runtime checks, and whole-engine frame/tick budgets. No claim is made for those gates.
