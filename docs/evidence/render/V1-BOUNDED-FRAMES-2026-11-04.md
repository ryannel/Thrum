# V1 bounded frame host — Apple M3 Max / Metal, 4 November 2026

**Scope.** Rust 1.99.1, wgpu 30.0.1, `cargo run -p v1-scene --release`. The same 10,000-entity, 1,500-moving, four-local-light, two-Sun-cascade synthetic scene runs through the fixed clock, CPU extraction, sparse instance upload and four GPU passes. Three prepared offscreen targets hold in-flight frames. The callback polls without waiting; timestamp resolution and collection happen later. No surface, Chrome/iPhone execution, cooked asset or visual-acceptance claim follows. The final 640×360 image had 85,260 non-background pixels in each run.

| 600 completed frames per optimized process | Run 1 | Run 2 |
| --- | ---: | ---: |
| Elapsed / committed ticks | 10.041 s / 602 | 10.035 s / 602 |
| Callback CPU p50 / p95 / max | 0.708 / 1.151 / 4.385 ms | 0.661 / 2.409 / 11.846 ms |
| Callbacks over 16.67 ms / dropped views / dropped reports | 0 / 0 / 0 | 0 / 0 / 0 |
| Timestamp faults / dropped presentation | 0 / 0 | 0 / 0 |
| CPU tick p50 / p95 | 53 / 113 µs | 55 / 126 µs |
| CPU extract p50 / p95 | 234 / 343 µs | 231 / 1,073 µs |
| CPU map/upload p50 / p95 | 176 / 243 µs | 168 / 424 µs |
| CPU encode p50 / p95 | 8 / 14 µs | 7 / 20 µs |
| GPU near/far Sun p50 | 0.061 / 0.058 ms | 0.061 / 0.058 ms |
| GPU forward/display p50 | 0.163 / 0.081 ms | 0.162 / 0.079 ms |

Both runs uploaded 172.8 MB of changed instance payload. Full callback timing includes nonblocking completion polling and all due callbacks; the second process had a 1,073 µs extraction p95 under variable local load. GPU pass p95 values were near/far 0.064/0.061 and 0.069/0.064 ms; forward/display 0.169/0.086 and 0.170/0.089 ms. A 12-sample Divan assembled-frame benchmark that explicitly waits after submission measured medians of 1.248 ms with timestamp queries and 1.138 ms without, with 141/80 benchmark-thread Rust allocations. Its individual samples reached 12.38/11.6 ms under variable local load. It remains a wait-inclusive benchmark; the callback distribution above measures the nonblocking path. The earlier synchronous host had variable wall tails, so these are not a controlled speedup ratio or a qualified device frame budget.

The opt-in Metal integration test covers idle callback, two committed ticks before a latest-view draw, in-order deferred pass reports, timestamp-enabled and disabled devices, visible capture and explicit clock overload without a skipped/repeated tick. A focused unit test covers late oldest-report eviction; an opt-in Metal unit test shows capture refreshes a newer committed world after an unsent view. Strict workspace checks, Wasm/iOS-simulator compiles and read-only review passed. The bounded ring and drop counters are implemented; sustained GPU backpressure, surface replacement/device loss and snapshot/packet worker leases remain open.
