# A5 four local actors in the V1 world, 4 November 2026

The running 10,000-entity V1 fixture now registers its last four entities as four distinct peer/actor owners. Peer 1 remains the keyboard-controlled local actor. All four pass through the same `Simulation::admit_frame` validation and fixed-tick stage; idle peers add no independent world clock. The original first 1,500 moving entities and render scene remain in place.

| Check | Result |
| --- | --- |
| Focused 10k simulation test | Four peer frames commit on one tick, with independent axis motion. An invalid peer-4 axis is refused before its valid sequence; a second peer-1 frame for the pending tick and a stale sequence on the following tick are refused. Peer 1 releases while peer 2's held motion continues. |
| Apple M3 Max / Metal host regressions | Existing committed-frame/overload and local-control hold/release tests pass with all four peers registered. |
| Chrome 156 WebGPU / Apple M3 Max, 1280 × 720 | 601 world ticks, 601 presented frames and 600 completed GPU samples in 10.021 s; zero unavailable, overload, refused ticks, timestamp clamps, shed views or report drops. CPU tick p50/p95 300/400 µs, encode 200/300 µs. Browser timer values are coarse. |
| Integrated offscreen Metal Divan, timestamps off | 100 samples: median 1.054 ms and 80 benchmark-thread Rust allocations / 14.39 KB per assembled frame. This includes simulation, extraction, upload, encode and GPU completion. There is no matched one-peer comparison, so it does not isolate the peer registration cost. |

This is a single-process authority fixture, with no network packet, delayed delivery, codec, replica or prediction. The separate test-only link conditioner is not yet wired into this running scene. Neither the browser diagnostic nor the benchmark establishes a sustained frame budget or physical-device result.
