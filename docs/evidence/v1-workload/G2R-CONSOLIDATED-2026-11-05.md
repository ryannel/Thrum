# G2S CPU evidence — 6 November 2026

**Gate open: quiet fixed rows fail.** Owner-run M3 Max, release `91ad52d`, clean worktree, agents paused and builds drained, 10:50–10:52 CET. The two runs below retain both results; uptime/load averages and complete output are in `tmp/quiet-run-2026-11-06/g2s-qualify{,-repeat}.log`. Exit zero means measurement completed, not budget passage. Historical G2R passes used easier rows; Git retains them.

| Fixed row | Target p95 µs | Run | p50 µs | p95 µs | p99 µs | Rust allocations, all threads |
| --- | ---: | --- | ---: | ---: | ---: | ---: |
| S1 10k | ≤25 | 1 | 49.458 | 59.458 | 72.958 | 0 |
| S1 10k | ≤25 | 2 | 44.208 | 54.458 | 62.167 | 0 |
| S1 100k | ≤300 | 1 | 473.042 | 894.791 | 1207.834 | 0 |
| S1 100k | ≤300 | 2 | 473.833 | 700.167 | 955.917 | 0 |
| S2 10k | ≤100 | 1 | 243.667 | 277.250 | 290.917 | 0 |
| S2 10k | ≤100 | 2 | 240.500 | 257.916 | 266.375 | 0 |

S1 includes four peers, shuffled movers, Damage and SetHealth, tick plus publication. S2 rewinds eight, amends retained input, replays and publishes. All measured rows, including a 200-spawn burst, allocate zero. Burst p50/p95/p99: 104.084/123.958/139.375 and 103.708/115.167/123.792 µs; no separate burst CPU budget.

| S1 extra action | p95 run 1 / 2, µs |
| --- | ---: |
| 10k Command | 54.875 / 52.000 |
| 10k Damage | 55.667 / 52.083 |
| 100k Command | 849.750 / 646.166 |
| 100k Damage | 906.459 / 646.167 |

Paired extra-action gate: ≤10%. First-run 10k Damage pair rises 91.417→106.000 µs p95 (+15.95%); repeat 53.667→54.167 (+0.93%). Other paired action rows stay within 10%. Do not qualify marginal cost from the friendlier repeat. Every variant's p50/p95/p99 remains in the raw logs.

**S2 profile:** rewind p50/p95 39.042/47.959 and 39.125/42.750 µs; replay-eight including final publication 203.459/233.208 and 201.250/217.333 µs. Amendment and page lease are ~0.04 µs. Phase quantiles are not additive. Resimulation/publication dominates; small traversal changes have not met the budget.

Forced-Rayon crossover p95 serial/forced: 10k 49.750/50.917 then 50.209/48.834 µs; 100k 472.709/476.583 then 459.792/461.750 µs. Both allocate zero, but the game declares Direct/Serial systems: requesting RayonForced does not dispatch parallel jobs. These rows establish neither a parallel crossover nor “never slower”.

Reproduce: `RUSTC_WRAPPER= rustup run 1.99.1 cargo bench -p v1-game --bench g2r --locked --offline -- --g2s-qualify`. Full-world randomized replay and held-page tests establish correctness separately. CPU budgets remain unchanged; continue independent G3b/G3s work while representation changes are evaluated.
