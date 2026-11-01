# I1 immutable page layout comparison

1 November 2026. The initial 32-slot page used `Vec<Option<Record<T>>>` during construction, then converted it to `Arc<[Option<Record<T>>]>`. Conversion moved the page payload into a second allocation. The selected `Arc<Vec<Option<Record<T>>>>` keeps the heap buffer and moves only the Vec header into the Arc. Root references become thin pointers. A fixed `[Option<Record<T>>; 32]` candidate was rejected after review because `Entity: Clone` has no inline-size bound and the array could exhaust a worker stack. The selected Vec keeps partial pages at their actual length.

The selected layout still has a Vec buffer and an Arc control allocation for each retained page, so it may retain more allocator metadata and cause more fragmentation than an Arc slice. It removes the payload relocation, not both allocations. On a 16,384-object world the root copies 512 thin page references instead of 512 fat references; `Snapshot::logical_bytes` excludes root and allocator overhead. The shared byte pool and precommit admission are separate unfinished work.

## Local cost evidence

`RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_cost` runs 10,000 measured authority ticks and 5,000 measured world ticks per 64/4,096/16,384-object zero, 1% and 25% churn case. It checks the replica against final authority state. Two baseline runs are [A](page-baseline-a.txt) and [B](page-baseline-b.txt); two selected-layout runs are [A](page-selected-a.txt) and [B](page-selected-b.txt). Baseline A used the working tree before the layout edit with an unused pool module; baseline B used an archive of `main` commit `e9ed268`. Both had identical `world.rs` and fixture source. They used Rust 1.99.1 release on local macOS arm64; the runs were sequential but not an isolated device benchmark. Baseline `world.rs` SHA-256 was `a83661c24b2a2e16f52472fb2a3eb9693a7d56af5df3914b1de7795607e0b6bd`; selected `world.rs` SHA-256 was `949d4cb3f5ad9d5be039c6836be747a61a1fa2a128d213ed166ba5935cd9ba92`; fixture SHA-256 was `6bc6b9975535b3ac0a2b515f0e046229f206ffe592ac366db5be3a300f685a07`.

| Publication workload | Baseline p50 | Selected p50 | Observation |
| --- | ---: | ---: | --- |
| 4,096 objects, 1% changed | 7.58–7.63 µs | 5.08–5.17 µs | The selected page path was lower in both runs. |
| 4,096 objects, 25% changed | 31.5–31.8 µs | 25.0–25.3 µs | Page copying remained a material cost. |
| 16,384 objects, 1% changed | 30.4–32.5 µs | 19.9–20.2 µs | The selected path reduced repeated page relocation work. |
| 16,384 objects, 25% changed | 136–174 µs | 104–105 µs | The spread between baseline runs shows machine noise. |

Zero-churn publication remained about 41–42 ns p50 in the fixture because it shares the existing page root. A second integrated check used the unchanged `i1_host_cost.rs` fixture (SHA-256 `8802d467bcb194d380dc3678e31d20b68df1b69681939e11f19ccaccac5baa2e`) in two [native headless callback runs](page-host-a.txt) ([second run](page-host-b.txt)). Both completed 480 scheduled callbacks, 241 authority/replica ticks and all 960 input results with no step-budget overrun. Host callback elapsed p99 was 19–22 µs and submit-to-confirmed-result p99 was 11.51–11.55 ms. This is a selected-build integration check, not a controlled before/after end-to-end speedup or device frame-time claim.

These results support the local layout selection, not a memory-residency claim. `RUSTC_WRAPPER= ./dev check` passed the existing authority/replica tests and the multi-page growth and old-snapshot isolation test. A read-only review found no defect in the final Vec layout or reservation substrate; it flagged and caused removal of the fixed-array stack-risk candidate.
