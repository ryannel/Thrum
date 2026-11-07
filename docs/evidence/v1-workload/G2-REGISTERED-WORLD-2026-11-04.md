# G2 registered storage and page checkpoint

4 November 2026. Apple M3 Max, Rust 1.99.1 release build, local `main` checkpoint following `6affe65`. Source: `crates/thrum-core/src/sim/registered_world.rs`, `snapshot_ring.rs`, `storage.rs`, and `examples/v1-game/benches/frame.rs`. These numbers are diagnostics for the **provisional V1 fixture**, not the completed game-owned G2 runtime or a device budget.

| Path | 10k entities / 1.5k moving | 100k / 15k moving | Scope |
| --- | ---: | ---: | --- |
| Registered private hecs + prepared write + leased page, warmed V1 game fixture | 54.29 µs median | 537.4 µs median | 100 samples × 100 iterations after 60 complete warmup ticks; four peers registered, no new frames |

The same game benchmark before the typed sparse-read cleanup measured 54.45/541.3 µs medians; these separate runs do not establish a speedup. A run before the 60-tick warmup measured 88.18 µs/1.339 ms medians with large tails, demonstrating that early page initialization must be excluded from steady-tick comparisons and reported separately. The game path still calls the fixed core fixture rules; this is **not** the requested G2 full-game tick qualification.

The live authority uses one hecs world, checked stable-ID/slot/generation mapping, a prepared storage-order motion read, and a prepared position write. The four-page `SnapshotRing` now owns the existing G1 page leases and refuses a tick when all pages are held. A review-agent found and the implementation fixed two defects: pools smaller than three pages could self-exhaust, and weak leases could make `can_capture` disagree with `Arc::get_mut`. The ring is now fixed at four pages and both selection and readiness require no strong or weak external lease. Tests cover duplicate and stale IDs, churn, rejected component bundles, retained page values, weak leases, and 10k/100k steady-tick allocation assertions. `./dev check` passed on this combined code and documentation tree; an explicit real-Metal host frame passed after the registered-world storage change.

Still open: game-owned components/actions/rules/wire schema, engine-enforced access and transaction dispatch, serial/Rayon equal results, restore/replay after churn, and integrated 10k/100k **game-owned** tick measurements. SIMD and heterogeneous core-class selection remain possible because the current tick chooses no core topology.
