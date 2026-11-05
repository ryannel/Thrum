# G2R consolidated CPU evidence — 5 November 2026

Apple M3 Max, release profile, 10,000/100,000 entities with 15% movers. Reproduce the full table with `RUSTC_WRAPPER= cargo bench -p v1-game --bench g2r`; its one process runs 1,000 measured samples per case after warmup. The global counting allocator observes Rust allocations on every thread while each sample runs. Values below are microseconds (µs); allocations are total calls over the 1,000 samples. `Rayon` is the production selection policy, which runs serial below the measured crossover; `RayonForced` is a diagnostic using the resident pool.

**Qualification:** Two full-command runs overlapped substantial unrelated host activity. The current run below overlapped an unrelated windowed sea process at roughly one full CPU core plus WindowServer, DaisyDisk and app activity. Its wide p95/p99 tails cannot qualify the Mac CPU budgets. A quiet run of the immediately preceding revision measured recorded tick p95 25.2/263.5 µs and rewind-8 p95 74.1/894.4 µs (window 16) and 72.0/1004.5 µs (window 64) at 10k/100k, all with zero allocations; the 10k recorded tick and 100k window-64 rewind were narrowly above the strict limits. Keep G2R open for a controlled, fully passing run. The earlier 27.2 µs churn excursion is a **p99 tail**, not a one-in-five p95 result.

| Case (count, mode, options) | p50 µs | p95 µs | p99 µs | Allocations |
| --- | ---: | ---: | ---: | ---: |
| `tick,10000,Serial,false,false` | 18.8 | 23.7 | 43.5 | 0 |
| `tick,10000,Rayon,false,false` | 18.7 | 23.6 | 33.8 | 0 |
| `tick,10000,Serial,false,true` | 22.1 | 30.2 | 59.7 | 0 |
| `tick,10000,Rayon,false,true` | 22.2 | 29.9 | 54.9 | 0 |
| `four_peers,10000,Serial,shuffled,damagefalse` | 28.5 | 35.8 | 61.0 | 0 |
| `four_peers,10000,Rayon,shuffled,damagefalse` | 28.5 | 35.4 | 54.3 | 0 |
| `four_peers,10000,Serial,shuffled,damagetrue` | 41.7 | 52.3 | 81.5 | 0 |
| `four_peers,10000,Rayon,shuffled,damagetrue` | 41.3 | 51.2 | 85.0 | 0 |
| `churn,10000,Serial,shuffled` | 28.6 | 35.7 | 62.1 | 0 |
| `rewind8,10000,Serial,true,w16,heldfalse` | 70.8 | 92.9 | 139.2 | 0 |
| `resim8,10000,Serial,true,w16,heldfalse` | 436.2 | 521.6 | 602.8 | 0 |
| `rewind8,10000,Serial,true,w64,heldfalse` | 73.3 | 91.4 | 133.3 | 0 |
| `resim8,10000,Serial,true,w64,heldfalse` | 444.1 | 495.9 | 557.6 | 0 |
| `rewind8,10000,Serial,true,w16,heldtrue` | 33.5 | 36.9 | 52.6 | 0 |
| `resim8,10000,Serial,true,w16,heldtrue` | 182.0 | 194.9 | 259.0 | 0 |
| `tick,100000,Serial,false,false` | 189.3 | 211.6 | 252.6 | 0 |
| `tick,100000,Rayon,false,false` | 189.1 | 210.6 | 253.6 | 0 |
| `tick,100000,Serial,false,true` | 224.9 | 270.4 | 299.8 | 0 |
| `tick,100000,Rayon,false,true` | 224.5 | 261.2 | 311.7 | 0 |
| `four_peers,100000,Serial,shuffled,damagefalse` | 425.0 | 1084.6 | 1660.4 | 0 |
| `four_peers,100000,Rayon,shuffled,damagefalse` | 419.1 | 963.9 | 1361.3 | 0 |
| `four_peers,100000,Serial,shuffled,damagetrue` | 902.2 | 1579.5 | 1938.4 | 0 |
| `four_peers,100000,Rayon,shuffled,damagetrue` | 883.8 | 1552.2 | 1909.9 | 0 |
| `churn,100000,Serial,shuffled` | 436.7 | 1018.8 | 1487.7 | 0 |
| `rewind8,100000,Serial,true,w16,heldfalse` | 1326.0 | 1948.8 | 2221.4 | 0 |
| `resim8,100000,Serial,true,w16,heldfalse` | 5945.5 | 7514.3 | 8280.0 | 0 |
| `rewind8,100000,Serial,true,w64,heldfalse` | 1419.8 | 2124.9 | 3167.2 | 0 |
| `resim8,100000,Serial,true,w64,heldfalse` | 6574.2 | 10384.8 | 14461.5 | 0 |
| `rewind8,100000,Serial,true,w16,heldtrue` | 353.5 | 531.1 | 723.4 | 0 |
| `resim8,100000,Serial,true,w16,heldtrue` | 3201.5 | 6185.9 | 9334.8 | 0 |
| `crossover,10000,Serial` | 37.0 | 52.5 | 63.2 | 0 |
| `crossover,10000,RayonForced` | 152.5 | 283.0 | 391.4 | 32 |
| `crossover,100000,Serial` | 365.7 | 482.6 | 535.8 | 0 |
| `crossover,100000,RayonForced` | 731.7 | 999.6 | 1157.1 | 32 |
| `crossover,200000,Serial` | 778.8 | 1031.0 | 1134.2 | 0 |
| `crossover,200000,RayonForced` | 1209.7 | 1464.9 | 1613.9 | 31 |
| `crossover,500000,Serial` | 1979.8 | 2426.5 | 2631.0 | 0 |
| `crossover,500000,RayonForced` | 2799.0 | 3251.6 | 3429.5 | 32 |

`tick,…,true` records history and publishes the changed page; `false` is an unrecorded tick. `rewind8` restores eight ticks, and `resim8` re-simulates them without intermediate publication. `w16`/`w64` are history windows; `heldtrue` retains page leases, so publication is deferred while authority continues. `churn` uses shuffled movers and one moving despawn/spawn per tick. Four peers use admitted input and optional damage. Admission occurs outside the timed interval. `crossover` compares serial against forced Rayon at 10k–500k; the forced path has pool dispatch allocations and no measured speed crossover, so the production policy remains serial for this workload. Every non-crossover case asserted zero warmed allocations.

| Budgeted path | 10k p95 budget | 100k p95 budget | Final full-run p95 10k/100k | Result |
| --- | ---: | ---: | ---: | --- |
| Recorded tick plus page | ≤25 µs | ≤300 µs | 30.2/270.4 µs | Open under shared host load |
| Shuffled moving churn tick plus page | ≤25 µs | ≤300 µs | 35.7/1,018.8 µs | Open under shared host load |
| Eight-tick rewind, window 16 | ≤100 µs | ≤1,000 µs | 92.9/1,948.8 µs | Open under shared host load |
| Eight-tick rewind, window 64 | ≤100 µs | ≤1,000 µs | 91.4/2,124.9 µs | Open under shared host load |

Earlier integrated baseline before sparse history: recorded tick p95 61/514 µs and rewind-8 p95 81/696 µs at 10k/100k. Those were separate runs with different host load and are directional, not controlled A/B speedups. Whole-world randomized replay proof, review and Metal gate are correctness checks, not CPU-budget evidence. The host-load condition is recorded rather than filtered from the result.
