# G2R consolidated CPU evidence — 5 November 2026

Apple M3 Max, release profile, 10,000/100,000 entities with 15% movers. Reproduce the full table with `RUSTC_WRAPPER= cargo bench -p v1-game --bench g2r`; its one process runs 1,000 measured samples per case after warmup. The global counting allocator observes Rust allocations on every thread while each sample runs. Values below are microseconds (µs); allocations are total calls over the 1,000 samples. `Rayon` is the production selection policy, which runs serial below the measured crossover; `RayonForced` is a diagnostic using the resident pool.

**Qualification:** The complete command after the direct dirty-page path and coalesced, no-drop single-component rewind passed every G2R CPU p95 gate at both history windows, with zero warmed allocations. The preceding full run under shared host load missed the 10k rewind gate at 105 µs p95; the repeat's 10k unrecorded tick p95 of 16.2 µs versus 41.7 µs in that run documents the load change. Both runs and their tails are retained under `tmp/g2r-`. The earlier 27.2 µs churn excursion was a **p99 tail**, not a one-in-five p95 result.

A further complete rerun while the competing benchmark was paused also passed: recorded tick p95 was 23.8/225.6 µs at 10k/100k; rewind-8 p95 was 47.0/656.6 µs at window 16 and 45.6/655.3 µs at window 64. All budgeted cases had zero warmed allocations. The full p50/p95/p99 output, including churn and forced Rayon tails, is `tmp/g2r-quiet-rerun-2026-11-05.log`; this run does not replace the table below.

| Case (count, mode, options) | p50 µs | p95 µs | p99 µs | Allocations |
| --- | ---: | ---: | ---: | ---: |
| `tick,10000,Serial,false,false` | 15.2 | 16.2 | 19.3 | 0 |
| `tick,10000,Rayon,false,false` | 15.2 | 16.2 | 19.2 | 0 |
| `tick,10000,Serial,false,true` | 19.1 | 23.7 | 25.5 | 0 |
| `tick,10000,Rayon,false,true` | 19.1 | 22.8 | 24.9 | 0 |
| `four_peers,10000,Serial,shuffled,damagefalse` | 24.5 | 29.5 | 31.4 | 0 |
| `four_peers,10000,Rayon,shuffled,damagefalse` | 24.4 | 29.8 | 31.6 | 0 |
| `four_peers,10000,Serial,shuffled,damagetrue` | 43.9 | 49.0 | 54.8 | 0 |
| `four_peers,10000,Rayon,shuffled,damagetrue` | 43.7 | 48.8 | 53.2 | 0 |
| `churn,10000,Serial,shuffled` | 27.4 | 31.8 | 35.0 | 0 |
| `rewind8,10000,Serial,true,w16,heldfalse` | 37.6 | 42.9 | 48.1 | 0 |
| `resim8,10000,Serial,true,w16,heldfalse` | 424.6 | 451.1 | 465.5 | 0 |
| `rewind8,10000,Serial,true,w64,heldfalse` | 37.5 | 42.6 | 48.1 | 0 |
| `resim8,10000,Serial,true,w64,heldfalse` | 422.4 | 450.6 | 484.4 | 0 |
| `rewind8,10000,Serial,true,w16,heldtrue` | 13.2 | 13.7 | 17.0 | 0 |
| `resim8,10000,Serial,true,w16,heldtrue` | 172.8 | 185.3 | 196.4 | 0 |
| `tick,100000,Serial,false,false` | 161.9 | 177.2 | 185.9 | 0 |
| `tick,100000,Rayon,false,false` | 161.7 | 177.2 | 186.8 | 0 |
| `tick,100000,Serial,false,true` | 199.7 | 213.9 | 221.7 | 0 |
| `tick,100000,Rayon,false,true` | 199.7 | 213.0 | 221.5 | 0 |
| `four_peers,100000,Serial,shuffled,damagefalse` | 267.3 | 297.8 | 306.2 | 0 |
| `four_peers,100000,Rayon,shuffled,damagefalse` | 267.0 | 295.7 | 308.9 | 0 |
| `four_peers,100000,Serial,shuffled,damagetrue` | 432.6 | 590.6 | 826.0 | 0 |
| `four_peers,100000,Rayon,shuffled,damagetrue` | 431.7 | 597.1 | 842.8 | 0 |
| `churn,100000,Serial,shuffled` | 286.1 | 318.2 | 328.5 | 0 |
| `rewind8,100000,Serial,true,w16,heldfalse` | 374.9 | 554.0 | 672.8 | 0 |
| `resim8,100000,Serial,true,w16,heldfalse` | 4714.7 | 5991.9 | 9690.6 | 0 |
| `rewind8,100000,Serial,true,w64,heldfalse` | 546.3 | 857.6 | 944.9 | 0 |
| `resim8,100000,Serial,true,w64,heldfalse` | 5869.1 | 7924.1 | 8672.7 | 0 |
| `rewind8,100000,Serial,true,w16,heldtrue` | 140.1 | 188.4 | 246.9 | 0 |
| `resim8,100000,Serial,true,w16,heldtrue` | 2423.1 | 3563.5 | 4273.6 | 0 |
| `crossover,10000,Serial` | 34.2 | 37.5 | 41.2 | 0 |
| `crossover,10000,RayonForced` | 115.8 | 254.0 | 320.9 | 32 |
| `crossover,100000,Serial` | 345.7 | 368.3 | 411.0 | 0 |
| `crossover,100000,RayonForced` | 645.0 | 891.0 | 1018.9 | 32 |
| `crossover,200000,Serial` | 691.1 | 736.5 | 815.0 | 0 |
| `crossover,200000,RayonForced` | 1097.0 | 1402.3 | 1608.8 | 31 |
| `crossover,500000,Serial` | 1810.8 | 1917.6 | 2097.8 | 0 |
| `crossover,500000,RayonForced` | 2460.5 | 2839.8 | 3573.0 | 32 |

`tick,…,true` records history and publishes the changed page; `false` is an unrecorded tick. `rewind8` restores eight ticks, and `resim8` re-simulates them without intermediate publication. `w16`/`w64` are history windows; `heldtrue` retains page leases, so publication is deferred while authority continues. `churn` uses shuffled movers and one moving despawn/spawn per tick. Four peers use admitted input and optional damage. Admission occurs outside the timed interval. `crossover` compares serial against forced Rayon at 10k–500k; the forced path has pool dispatch allocations and no measured speed crossover, so the production policy remains serial for this workload. Every non-crossover case asserted zero warmed allocations.

| Budgeted path | 10k p95 budget | 100k p95 budget | Complete-run p95 10k/100k | Result |
| --- | ---: | ---: | ---: | --- |
| Recorded tick plus page | ≤25 µs | ≤300 µs | 23.7/213.9 µs | Pass |
| Eight-tick rewind, window 16 | ≤100 µs | ≤1,000 µs | 42.9/554.0 µs | Pass |
| Eight-tick rewind, window 64 | ≤100 µs | ≤1,000 µs | 42.6/857.6 µs | Pass |

Shuffled moving churn is a required diagnostic in the table, with no separate numeric gate. At 100k, the changed-slot bitset retains 12,504 bytes per undo (200,064 bytes at window 16; 800,256 bytes at window 64); 10k uses the original sparse visitor and retains no bitset. The position-only page path appends physically ordered dirty slots directly; mixed and structural edits retain the bitset comparison. Coalesced rewind applies each repeated no-drop component preimage once using reused world scratch; structural, mixed-component and drop-sensitive histories keep reverse-order restore. In paired focus runs, 100k rewind window-16 median moved from 572.5 to 420.5 µs (old→new) and 713.8 to 533.6 µs (new→old); window-64 moved 540.5→477.4 and 714.9→532.7 µs. The paired p95 tails varied with shared load, so these median comparisons are directional.

Earlier integrated baseline before sparse history: recorded tick p95 61/514 µs and rewind-8 p95 81/696 µs at 10k/100k. Those were separate runs with different host load and are directional, not controlled A/B speedups. Whole-world randomized replay proof, review and Metal gate are correctness checks, not CPU-budget evidence. The host-load condition is recorded rather than filtered from the result.
