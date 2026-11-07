# V1 serial frame-host diagnostic

4 November 2026. Apple M3 Max, Metal, macOS, Rust 1.99.1 release, wgpu 30.0.1. `RUSTC_WRAPPER= cargo run -p v1-scene --release` runs 600 paced offscreen callbacks through Thrum's fixed-step clock. Each due update commits the same 10k-entity A1 simulation (1,500 moving), extracts presentation columns, queues changed instance rows, encodes both prepared HDR/display passes, submits and explicitly waits for GPU completion and timestamps. [The selected final frame](V1-FRAME-HOST-2026-11-04.png) was inspected: it is a synthetic grid, not V1 art. The second process run took 10.023 s for 600 presented frames and **601 committed ticks**; one bounded catch-up frame accounts for the extra tick. Neither run dropped presentation samples.

| Observation | First 600-frame run | Second 600-frame run |
| --- | ---: | ---: |
| CPU tick p50/p95 | 57/95 µs | 117/194 µs |
| CPU visit/extract p50/p95 | 238/322 µs | 1,000/1,384 µs |
| CPU scene map/upload p50/p95 | 185/269 µs | 419/713 µs |
| CPU encode p50/p95 | 7.5/15.8 µs | 18.0/26.5 µs |
| Host-call wall p50/p95/max | 1.79/6.89 ms; max not recorded | 3.64/15.39/25.46 ms |
| GPU forward p50/p95 | 0.099/1.875 ms | 0.098/0.583 ms |
| GPU display p50/p95 | 0.050/1.655 ms | 0.050/0.602 ms |
| Catch-up frames / host calls over 16.67 ms | not recorded | 1 / 20 |

An explicit Divan benchmark drives the same host with virtual 60 Hz deadlines and waits for each frame: 100 single-frame samples gave **1.046 ms median** with GPU timestamps and **0.866 ms** without. Its counting allocator observed median **115 allocations / 20.97 KB** with timestamps and **60 / 9.78 KB** without on the benchmark thread. These counts include synchronous Rust wgpu calls, but exclude other threads and native driver allocation; the authoritative simulation tick retains its separate zero steady-allocation assertion. Immediately after the slower paced run, host load averaged 15.5 with four `rustc` processes near a core each; that observation does not prove causality. No sustained frame-budget claim follows from these two variable runs.

The opt-in Metal host test proves an idle callback commits nothing, two due updates commit exactly twice before one latest-view draw, and nine due updates enter explicit overload without advancing the world. Strict Clippy, the full `./dev check` gate, and Wasm/iOS-simulator compile checks pass. Those compiles are not Chrome/iPhone execution. The host currently serializes GPU completion and has no platform surface, async snapshot/packet leases, device-loss recovery, cooked content, clustered lights or CSM. The ~55 additional benchmark-thread allocations per frame with timestamp queries motivate integrating resolve/readback with a future pipelined executor, while retaining per-pass timings.
