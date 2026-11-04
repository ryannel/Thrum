# A3 first native extraction handoff

4 November 2026. Apple M3 Max/Metal, Rust 1.99.1, 10k W1 entities with 1.5k moving, 640×360 offscreen four-pass scene. The native V1 host can move one retained visual-page lease and its reused extractor to a one-thread Rayon pool. A bounded completion slot returns CPU columns to the host; the worker releases the world page before notifying the event loop. The macOS window now wakes on both extraction and surface acquisition, requests a drawable only after a CPU view is ready, and can submit on a zero-update callback. Browser library and diagnostic binary compile with the serial route. The window itself was not visibly run on the locked display.

| 600-frame process order | Serial 1 | Worker 1 | Worker 2 | Serial 2 |
| --- | ---: | ---: | ---: | ---: |
| Callback CPU p50/p95, ms | 0.808/1.605 | 0.276/0.526 | 0.281/0.575 | 0.692/1.198 |
| W1 tick p50, µs | 71.0 | 76.8 | 76.1 | 61.4 |
| Page copy + extraction p50, µs | 268.5 | 275.2 | 280.3 | 230.5 |
| Worker queue p50, µs | — | 8.8 | 8.8 | — |
| Ready-to-submit p50, µs | — | 9.8 | 9.5 | — |

These four process timings were collected before review repaired the busy-worker drop counter, so their zero-drop values are not usable. A post-fix worker run completed 600 frames in 1,203 callbacks over 10.040 s, counted **one** shed optional view, zero stale views and zero timing faults, with callback CPU p50/p95 0.200/0.344 ms. A serial run completed 600 frames in 602 callbacks over 10.036 s, with callback CPU p50/p95 0.649/1.001 ms. Both had zero visual age in submitted samples. The native path makes an extra wake callback for a completed view; callback distributions therefore do **not** establish a total CPU or whole-frame speedup. Tick/extraction and GPU p95 varied materially between processes. The worker queue remains one task per view; worker-thread allocations and retained peak are not yet measured. The existing A2 zero-allocation assertion covers the W1 tick and page capture, not Rayon dispatch or GPU calls.

An opt-in Metal test blocked the worker's completion callback after its first result: tick 1 submitted on a zero-update call, ticks 2 and 3 committed while the next extraction was queued, and the busy view was counted. The same tick's serial/native captures matched byte for byte. Additional Metal tests verify a same-tick explicit capture refreshes after reclaiming an unapplied worker result, a result more than two ticks old refreshes from the current committed tick before GPU submission, and a retired visual generation resets prior-pose history. A non-GPU unit test compares serial/worker columns and confirms the page lease is released before the wake. Strict repository gate and native/Wasm compile are the landing checks.

This is a first running consumer of the retained page, **not** the complete A3/F1 job system. The combined typed graph, generic system parallelism, worker/GPU packet resource ownership, allocation/retained-peak measurement, ABBA whole-frame latency under controlled load, persistent slow-worker presentation, shutdown/capture-timeout stress, visible 600-frame macOS window, Chrome runtime and iPhone run remain open.
