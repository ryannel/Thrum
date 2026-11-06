# G2S CPU evidence — 6 November 2026

**Recorded quiet baseline; old CPU targets retired at 11:15 on 6 November.** Owner-run M3 Max, release `91ad52d`, clean worktree, agents paused and builds drained, 10:50–10:52 CET. The two runs below retain both results; uptime/load averages and complete output are in `tmp/quiet-run-2026-11-06/g2s-qualify{,-repeat}.log`. Exit zero means measurement completed, not budget passage. Historical G2R passes used easier rows; Git retains them.

| Fixed row | Former p95 reference µs | Run | p50 µs | p95 µs | p99 µs | Rust allocations, all threads |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| S1 10k | ≤25 | 1 | 49.458 | 59.458 | 72.958 | 0 |
| S1 10k | ≤25 | 2 | 44.208 | 54.458 | 62.167 | 0 |
| S1 100k | ≤300 | 1 | 473.042 | 894.791 | 1207.834 | 0 |
| S1 100k | ≤300 | 2 | 473.833 | 700.167 | 955.917 | 0 |
| S2 10k | ≤100 | 1 | 243.667 | 277.250 | 290.917 | 0 |
| S2 10k | ≤100 | 2 | 240.500 | 257.916 | 266.375 | 0 |

S1 includes four peers, shuffled movers, Damage and SetHealth, tick plus publication. S2 rewinds eight, amends retained input, replays and publishes. All measured rows, including a 200-spawn burst, allocate zero. Burst p50/p95/p99: 104.084/123.958/139.375 and 103.708/115.167/123.792 µs; no separate burst CPU budget.

**Engine-owned descriptors, 6 November:** late systems can append but cannot replace/remove prior operations;
foreign permits refuse the tick. Shared candidate values stay late-bound. Serial/parallel merge refusal, final
payload/handle validation, direct rollback, recorded retry/rewind and held pages pass public-API tests; direct kernels gain no per-row check.
Shared M3 Max ABBA (other sessions active, load ~9–12), unchanged `--g2s-qualify` rows, two run p95 ranges:

| Row | Before `3f7ab60`, µs | Engine queue, µs | Warm allocations |
| --- | ---: | ---: | ---: |
| S1 10k | 47.708–48.792 | 50.291–53.167 | 0 |
| S1 100k | 683.958–716.959 | 662.583–737.041 | 0 |
| S2 10k | 280.583–319.333 | 281.834–285.791 | 0 |
| Spawn200 | 123.208–133.209 | 125.459–128.959 | 0 |

All rows retain p50/p95/p99/all-thread counts in `tmp/g2s-edit-ownership/paired-*.log`; no speed claim.
The old quiet extra-action pairs remain in their raw logs; both favorable and adverse repeats are retained.

**S2 profile:** rewind p50/p95 39.042/47.959 and 39.125/42.750 µs; replay-eight including final publication 203.459/233.208 and 201.250/217.333 µs. Amendment and page lease are ~0.04 µs. Phase quantiles are not additive. Resimulation/publication dominates; small traversal changes have not met the budget.

Forced-Rayon crossover p95 serial/forced: 10k 49.750/50.917 then 50.209/48.834 µs; 100k 472.709/476.583 then 459.792/461.750 µs. Both allocate zero, but the game declares Direct/Serial systems: requesting RayonForced does not dispatch parallel jobs. These rows establish neither a parallel crossover nor “never slower”.

Reproduce: `RUSTC_WRAPPER= rustup run 1.99.1 cargo bench -p v1-game --bench g2r --locked --offline -- --g2s-qualify`. Presence-aware hashing and 10k amended replay now match the straight corrected world; randomized/held-page proofs remain. Game-side Resolve staging and permit handling keep G2S open; tune later.
