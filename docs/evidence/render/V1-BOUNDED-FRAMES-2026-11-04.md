# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Gate open:** 10k/1080p render CPU p95 ≤300 µs and zero frame-thread allocations unmet. Real 120 Hz ten-minute cadence, latency and device recovery unqualified; no iPhone result.

| Threaded offscreen render CPU, µs | p50 | p95 | p99 | Submitted/callbacks |
| --- | ---: | ---: | ---: | ---: |
| Game host A1 | 379.1 | 481.4 | 1062.8 | 199/200 |
| Engine host B1 | 378.5 | 521.4 | 1535.1 | 199/200 |
| Engine host B2 | 373.1 | 485.1 | 1099.3 | 199/200 |
| Game host A2 | 381.0 | 521.3 | 1055.5 | 188/200 |

Control `91ad52d`, candidate this checkpoint; Rust 1.99.1/wgpu 30.0.1, 1.5k movers, timing enabled, uncontrolled shared CPU/GPU load. Forecasts at 120 Hz, synthetic surface texture, no drawable or real presents. Full callbacks including poll p95 A1/B1/B2/A2: 484.3/522.1/486.1/522.2 µs. B1 has an adverse CPU tail; **no speed/non-regression qualification**. GPU p99 1.083–19.640 ms. Raw logs/identities: `tmp/g3b-runtime/`.

Separate unpaced inline ABBA, each 200/200: render p95 606.5/472.2/373.5/524.5 µs; GPU p99 0.992–1957.056 ms. Candidate B2 upload/encode p50 206.0/102.8 µs dominate; this is not the threaded or display row.

Threaded allocator A1/B/A2, 100 warmed successful samples each: render median **374 allocations / 55,371 B** throughout; maxima 380/380/382. World **0/0** after wake five. Each submitted 120 frames but needed 215/1154/204 callbacks: no throughput parity. Rust counts exclude drawable/Objective-C costs. Separate inline 600-frame phase medians/maxima: advance 0/0, scene encode 93/102, submit 233/234; submit bytes 25,313/25,353.

Core owns clock/lane; generic thrum-render `FrameHost` owns two slots, tokens, completion and history. Game retains schema, scene mapping and camera policy. Review found retry dirty-loss and mapping-reuse defects; fixed with GPU regressions, plus uniform source-identity checks. Full `./dev check`, Wasm and iOS simulator compile pass. Independent review found no remaining defect. Fixed initial/moving pixels match control bitwise; this proves two stills, not sequence quality. Grid aliasing persists.

**Earlier trace `ce7aa3f`, eight seconds:** 525 CA wait spans after time 2 s, p99 0.879 ms, total 132.715 ms; only 0.585 ms (0.44%) overlaps this app's GPU execution. Span quantiles are not frame quantiles; raw: `tmp/g3b-metal-system-trace/`. Native encoding occurs at `finish` after acquisition. No raw bridge selected; mixed raw/wgpu unsupported. Split pre-submit remains parked at `30ae560` (308→335 timing-disabled allocations).

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
