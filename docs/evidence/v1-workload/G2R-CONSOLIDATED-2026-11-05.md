# G2R consolidated CPU evidence — 5 November 2026

Apple M3 Max, release profile, 10,000/100,000 entities with 15% movers. Reproduce the full table with `RUSTC_WRAPPER= cargo bench -p v1-game --bench g2r`; its one process runs 1,000 measured samples per case after warmup. The global counting allocator observes Rust allocations on every thread while each sample runs. Values below are microseconds (µs); allocations are total calls over the 1,000 samples. `Rayon` is the production selection policy, which runs serial below the measured crossover; `RayonForced` is a diagnostic using the resident pool.

**Qualification:** The latest full-command run below used the adaptive undo bitset and ran after the separate sea process stopped, but MediaAnalysis, DaisyDisk and app activity still occupied the Mac. Rewind-8 passed its 10k/100k p95 budgets at both window sizes with zero allocations; recorded 10k tick remained just over its strict p95 limit. G2R stays open. The immediately preceding full run without this optimization measured 100k rewind p95 1,314/1,297 µs (windows 16/64); these separate runs are directional, not controlled A/B speedups. The earlier 27.2 µs churn excursion is a **p99 tail**, not a one-in-five p95 result.

| Case (count, mode, options) | p50 µs | p95 µs | p99 µs | Allocations |
| --- | ---: | ---: | ---: | ---: |
| `tick,10000,Serial,false,false` | 19.0 | 22.9 | 25.6 | 0 |
| `tick,10000,Rayon,false,false` | 19.0 | 22.8 | 26.0 | 0 |
| `tick,10000,Serial,false,true` | 21.0 | 26.1 | 29.4 | 0 |
| `tick,10000,Rayon,false,true` | 21.0 | 25.2 | 29.6 | 0 |
| `four_peers,10000,Serial,shuffled,damagefalse` | 27.1 | 32.1 | 41.0 | 0 |
| `four_peers,10000,Rayon,shuffled,damagefalse` | 27.1 | 31.7 | 35.4 | 0 |
| `four_peers,10000,Serial,shuffled,damagetrue` | 38.7 | 44.7 | 53.5 | 0 |
| `four_peers,10000,Rayon,shuffled,damagetrue` | 38.6 | 44.8 | 54.5 | 0 |
| `churn,10000,Serial,shuffled` | 27.5 | 33.8 | 36.8 | 0 |
| `rewind8,10000,Serial,true,w16,heldfalse` | 55.7 | 65.1 | 82.2 | 0 |
| `resim8,10000,Serial,true,w16,heldfalse` | 425.0 | 481.7 | 529.9 | 0 |
| `rewind8,10000,Serial,true,w64,heldfalse` | 55.6 | 68.4 | 91.6 | 0 |
| `resim8,10000,Serial,true,w64,heldfalse` | 425.5 | 483.1 | 539.7 | 0 |
| `rewind8,10000,Serial,true,w16,heldtrue` | 31.9 | 36.7 | 41.7 | 0 |
| `resim8,10000,Serial,true,w16,heldtrue` | 173.0 | 190.1 | 228.8 | 0 |
| `tick,100000,Serial,false,false` | 179.3 | 207.4 | 245.8 | 0 |
| `tick,100000,Rayon,false,false` | 179.3 | 212.9 | 253.0 | 0 |
| `tick,100000,Serial,false,true` | 224.2 | 272.6 | 324.6 | 0 |
| `tick,100000,Rayon,false,true` | 223.9 | 264.3 | 314.0 | 0 |
| `four_peers,100000,Serial,shuffled,damagefalse` | 319.3 | 561.5 | 790.6 | 0 |
| `four_peers,100000,Rayon,shuffled,damagefalse` | 318.8 | 570.5 | 802.7 | 0 |
| `four_peers,100000,Serial,shuffled,damagetrue` | 595.3 | 1110.0 | 1388.8 | 0 |
| `four_peers,100000,Rayon,shuffled,damagetrue` | 591.1 | 1103.2 | 1350.2 | 0 |
| `churn,100000,Serial,shuffled` | 316.5 | 638.2 | 855.7 | 0 |
| `rewind8,100000,Serial,true,w16,heldfalse` | 665.0 | 969.4 | 1092.3 | 0 |
| `resim8,100000,Serial,true,w16,heldfalse` | 5063.9 | 5929.3 | 6653.7 | 0 |
| `rewind8,100000,Serial,true,w64,heldfalse` | 664.0 | 906.4 | 999.0 | 0 |
| `resim8,100000,Serial,true,w64,heldfalse` | 5076.7 | 5814.7 | 6200.5 | 0 |
| `rewind8,100000,Serial,true,w16,heldtrue` | 334.3 | 423.4 | 510.3 | 0 |
| `resim8,100000,Serial,true,w16,heldtrue` | 2089.0 | 3036.5 | 3530.6 | 0 |
| `crossover,10000,Serial` | 33.8 | 37.9 | 45.7 | 0 |
| `crossover,10000,RayonForced` | 127.9 | 241.7 | 281.8 | 32 |
| `crossover,100000,Serial` | 342.3 | 393.3 | 439.4 | 0 |
| `crossover,100000,RayonForced` | 679.0 | 787.7 | 860.2 | 32 |
| `crossover,200000,Serial` | 759.9 | 860.8 | 929.2 | 0 |
| `crossover,200000,RayonForced` | 1071.8 | 1238.5 | 1339.4 | 31 |
| `crossover,500000,Serial` | 1741.8 | 1843.1 | 2085.5 | 0 |
| `crossover,500000,RayonForced` | 2361.5 | 2615.5 | 2776.6 | 32 |

`tick,…,true` records history and publishes the changed page; `false` is an unrecorded tick. `rewind8` restores eight ticks, and `resim8` re-simulates them without intermediate publication. `w16`/`w64` are history windows; `heldtrue` retains page leases, so publication is deferred while authority continues. `churn` uses shuffled movers and one moving despawn/spawn per tick. Four peers use admitted input and optional damage. Admission occurs outside the timed interval. `crossover` compares serial against forced Rayon at 10k–500k; the forced path has pool dispatch allocations and no measured speed crossover, so the production policy remains serial for this workload. Every non-crossover case asserted zero warmed allocations.

| Budgeted path | 10k p95 budget | 100k p95 budget | Latest full-run p95 10k/100k | Result |
| --- | ---: | ---: | ---: | --- |
| Recorded tick plus page | ≤25 µs | ≤300 µs | 26.1/272.6 µs | 10k open |
| Eight-tick rewind, window 16 | ≤100 µs | ≤1,000 µs | 65.1/969.4 µs | Pass |
| Eight-tick rewind, window 64 | ≤100 µs | ≤1,000 µs | 68.4/906.4 µs | Pass |

Shuffled moving churn is a required diagnostic in the table, with no separate numeric gate. At 100k, the changed-slot bitset retains 12,504 bytes per undo (200,064 bytes at window 16; 800,256 bytes at window 64); 10k uses the original sparse visitor and retains no bitset. The position-only page path skips unrelated component reads, while mixed and structural edits use the full comparison.

Earlier integrated baseline before sparse history: recorded tick p95 61/514 µs and rewind-8 p95 81/696 µs at 10k/100k. Those were separate runs with different host load and are directional, not controlled A/B speedups. Whole-world randomized replay proof, review and Metal gate are correctness checks, not CPU-budget evidence. The host-load condition is recorded rather than filtered from the result.
