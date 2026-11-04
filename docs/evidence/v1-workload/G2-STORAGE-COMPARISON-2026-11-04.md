# G2 storage-order tick comparison

4 November 2026. Apple M3 Max, macOS 26.6.2, Rust 1.99.1 release, hecs 0.11.2, Divan 0.1.21. Baseline `43a900f`. [Retained benchmark source](g2-storage-comparison/Cargo.toml) and lockfile are a separate measurement crate, not a game executable or engine dependency. Run `RUSTC_WRAPPER= cargo test --manifest-path docs/evidence/v1-workload/g2-storage-comparison/Cargo.toml --offline` and `RUSTC_WRAPPER= cargo bench --manifest-path docs/evidence/v1-workload/g2-storage-comparison/Cargo.toml --offline --bench storage -- --min-time 0.5 --max-time 2`.

Both candidates hold 15% movers in a separate archetype, f64 XYZ position and velocity, stable identity, health and visual data. Every 60 Hz tick integrates all movers, edits 1% of static positions through stable identity, and replaces static entities at 1% of population per second (100/s or 1,000/s). Hecs uses `PreparedQuery::query_mut` in storage order for motion, and private entity lookup for sparse edits/churn. The custom store has two typed SoA archetypes, stable-ID-to-row lookup and swap-remove repair. After 600 ticks at each size, every position, changed count and aggregate generation/data checksum match. This is an isolated kernel; it does not include G1 publication, game rules, workers, transport, render extraction or a device frame.

| Entities / movers | hecs tick median, run 1 / 2 | custom SoA tick median, run 1 / 2 | hecs extra time |
| --- | ---: | ---: | ---: |
| 10,000 / 1,500 | 2.416 / 2.499 µs | 1.645 / 1.645 µs | 0.771–0.854 µs |
| 100,000 / 15,000 | 26.74 / 27.62 µs | 19.24 / 19.08 µs | 7.50–8.54 µs |

Divan's hecs allocation summary reports zero in the maximum sampled tick, but sub-byte mean allocation accounting appears over the long run; this is **not** a zero-allocation guarantee. The custom benchmark output did not show per-sample allocation rows. Candidate setup and all retained memory were not measured. The custom candidate handles two known archetypes and churn but does not implement arbitrary typed component registration, general archetype transitions, borrow enforcement or restore. The earlier [A0 spike](A1-STORAGE-SPIKE-2026-11-03.md) covers broader hecs/Bevy restore and Wasm feasibility, but used indexed mover access and cannot substitute for this storage-order comparison. Integrated 10k/100k cost and tail latency remain G2 qualification work.
