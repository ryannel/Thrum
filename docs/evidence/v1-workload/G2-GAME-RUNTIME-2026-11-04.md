# G2 running-game checkpoint

4 November 2026. Apple M3 Max, Rust 1.99.1, release build. This is the actual `examples/v1-game` authority with 10k/1.5k or 100k/15k movers, four registered peers, game-owned fields and rules, engine-owned hecs storage, checked identity and leased publication. The tick bench warms 60 ticks, then uses `cargo bench -p v1-game --bench frame game_tick_and_publication -- --min-time 1 --sample-count 40`. It admits no new frame in the timed loop; the four-peer, failure and churn paths are tested separately. These are local diagnostics, not V1 device budgets.

| Complete game tick + page | 10k | 100k |
| --- | ---: | ---: |
| Serial median | 76.87 µs | 774.8 µs |
| Rayon mover jobs median | 198.2 µs | 1.062 ms |

Serial is the current default. A counting-allocator test observed zero allocations in 100 warmed serial 10k ticks. Divan observed occasional Rayon scheduler allocation, around 0.015 allocations and 24 bytes averaged per measured tick, alongside slower medians. The tick has no hard-coded core topology; the platform can select a suitable executor later. A four-peer 10k test compares full endpoint rows, accepted origins, dirty positions and structural events after a rejected edit, churn and restore/replay. A separate replay test captures after slot reuse and compares the later endpoint. The explicit native Metal host test `committed_ticks_drive_latest_gpu_frame_and_overload_is_explicit` passed after the game-boundary migration.

The column snapshot experiment uses one typed vector per registered component, with explicit snapshot-copy registration for non-Copy types. The bench measures full-world captures, not yet a rolling delta or owned subset. At 10k/100k, full capture medians were 93.81/951.1 µs with 16 allocations and 1.41/14.08 MB per capture. Restore medians were 1.531/19.27 ms with 14.02/133.7 MB allocated per restore, largely rebuilding hecs and four complete pages. Those costs make per-tick full-world prediction capture inappropriate; a change-driven rolling representation remains to be designed and qualified.

The fixed core world, page and controller fixture code has been removed. Production engine declarations of V1 position, velocity, health, visual, ownership and action types: **zero**. `./dev check` passed at local branch checkpoint `f8f746e`; scoped game tests and workspace Clippy passed after the column snapshot change. **G2 remains open.** Independent review found that generic `WorldCommit` can partially mutate before a later checked edit fails, and the engine does not yet bind callbacks and edit targets directly to compiled schedule execution. Address both before declaring G2 complete; do not interpret the running-game test as a general transaction proof.
