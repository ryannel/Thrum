# A5 first confirmed client consumer, 4 November 2026

The 10k-entity W1 world remains the sole gameplay authority. After each successful tick, it exposes a copied controller projection for each active peer. The four-peer diagnostic pairs each projection with that tick's exact committed input origin and sends complete groups through a bounded, ordered, two-to-four-tick in-memory return lane. Each client holds its own confirmed state and admits contiguous ticks, matching bindings, finite controller values and strictly advancing origins. This is a lossless local consumer proof, not an ACK or state-delta transport.

| Apple M3 Max, Rust 1.99.1 release | Observation |
| --- | --- |
| 600-tick four-peer run | 2,384 controller groups delivered and 16 in flight at stop; all four clients reached tick 596 while authority reached 600. Maximum return-group age was four ticks. Every delivered controller position matched an independent tick-indexed held-axis trace; delivered plus in-flight committed origins matched all 2,380 accepted frames. Authority tick p50/p95 was 47/51 µs in this one local process run, including input bytes, 10k world step and result delivery. |
| Divan, 20 complete runs | Median 35.24 ms per 600-tick run; 134 benchmark-thread Rust allocations per complete run, including setup and teardown. The counting-allocator assertion remains zero across 500 warmed ticks. Run-to-run timing varies; these observations do not establish a speed difference. |

Focused tests reject skipped and repeated ticks, changed bindings, nonfinite or out-of-range values, and misbound or repeated committed origins without changing client state. Core tests verify committed controller values after input and the invalidated binding after same-tick despawn. The 600-tick run and burst-loss case pass; strict Clippy passes. An independent read-only review found no actionable defect and identified the expected next gates: reconnect baselines, state deltas, result/ACK retention and prediction.

Open: the return lane has no reverse loss, retry or socket. It copies complete controller values and has no client prediction or reconciliation. Browser and phone multiplayer, visible acceptance and sustained bandwidth/device budgets remain unqualified.
