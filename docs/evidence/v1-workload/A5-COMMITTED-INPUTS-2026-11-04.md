# A5 W1 committed input origins, 4 November 2026

W1 now returns a borrowed slice of exact input origins with each **successful** `Simulation::step` outcome. Its buffer is reserved to peer capacity at world setup and filled in stable peer order only at the publish barrier. A failed tick returns no outcome and retains pending frames. Same-tick actor despawn still reports an input consumed earlier in that tick. N1's four-peer conditioner compares every due handoff with the W1 origin slice before counting an accepted frame; the world remains the sole gameplay authority.

| Apple M3 Max, Rust 1.99.1 release | Observation |
| --- | --- |
| 600-tick four-peer encoded input run | 2,380 due origins matched the W1 committed-origin slices; zero due misses; four final positions matched the independent held-axis trace. Tick p50/p95 49/53 µs under one local process run, including the byte conditioner and full 10k world step. |
| Divan, 20 complete runs | Median 29.01 ms per 600-tick run, beside prior encoded-path medians 29.0/29.4 ms. The benchmark thread made 132 allocations per complete run versus 131 before this buffer; the additional setup allocation reserves the peer-capacity origin page. The existing 500-warmed-tick counting-allocator assertion remains zero. These observations do not establish a speed difference. |

Focused tests cover a rejected command followed by exact same-tick retry, no repeated origin on the next idle tick, and a pending input consumed before same-tick despawn. Strict Clippy, repository guards, formatting and the full Rust test gate are recorded by the landing commit. Review found no actionable defect after a test lint correction.

Open: W1 admission rejection needs a typed N1 result with retained delivery and application ACK; committed origins still need to travel atomically with authoritative controller state to a client. This does not yet qualify prediction, reconciliation, a state-delta codec, OS transport, browser/phone multiplayer or sustained device budgets.
