# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Gate open.** Rust 1.99.1 / wgpu 30.0.1, 10k entities / 1.5k movers, 1080p, shared GPU/Blender load. CPU, allocation, cadence, latency and lifecycle budgets remain unqualified; no iPhone result.

| Sparse extraction A/B: complete render-frame wall, µs | p50 | p95 | p99 | Extract p95 |
| --- | ---: | ---: | ---: | ---: |
| Full control A1 | 744.5 | 1331.8 | 1677.5 | 144.2 |
| Sparse B1 | 638.8 | 1257.4 | 1446.7 | 104.2 |
| Full control A2 | 764.2 | 1330.2 | 1455.6 | 143.9 |
| Sparse B2 | 689.2 | 1325.0 | 1677.4 | 108.9 |

Each offscreen run submitted 200/200 frames and uploaded 300k rows / 57.6 MB. One earlier pair was 2.7% slower for the whole frame. Extraction improves consistently; tails remain load-sensitive. Upload, view preparation and recording now dominate.

Live early/late A/B: 240 presents per run, 241–242 display callbacks; early counted 122/107 dropped views, late 1/1. Worker wall p95 was 0.767/0.766 ms early and 1.268/1.777 ms late; early excludes its acquisition thread. A later breakdown measured worker/acquire/upload+encode p95 2.165/1.424/0.780 ms. The final 1,200-frame capture run had 1,223 callbacks, eight drops and p95 2.830/1.132/2.023 ms respectively. These counts and elapsed measures establish no presentation-interval or latency result.

The warmed extractor asserts zero allocations. With live timestamp features, 100 warmed submitted callbacks report 376 median / 386 maximum allocations, versus 308 with timing disabled; world-thread allocations remain zero. This excludes drawable/view/platform costs. The moving capture matched pre-sparse pixels at 640×360. Independent review of 12 live screenshots found no corruption or sampled flashes; continuous motion remains unqualified.

**Metal System Trace, `ce7aa3f`, eight seconds.** After trace time 2 s, PID5128 had 525 CA wait spans (411 clusters): duration p50/p95/p99 0.170/0.683/0.879 ms, 132.715 ms total. Only 0.585 ms (0.44%) overlapped this app’s GPU execution; other processes may use the GPU. These are conditional span quantiles, not frame quantiles. Pinned wgpu defers native encoding to `CommandEncoder::finish`, currently after acquisition. Trace and XML analysis stay in `tmp/g3b-metal-system-trace/`.

**Rejected pre-submit, `30ae560` / `codex/g3b-scene-presubmit`.** Lifecycle tests and `./dev check` pass, but split submissions raise timing-disabled allocations 308→335. Paired render CPU p95 controls 395/483 µs versus split 545/477 µs show no clear win; failed live runs provide no overlap evidence. Independent review recommended parking it. No raw drawable bridge is selected.

Review fixed capacity ordering, stale resize and blocked close. Premature cancellation now fails instead of masking errors. Generic ownership, native pre-submit, device recovery and timing qualification remain open.

Reproduce: `cargo bench -p v1-game --bench frame --locked --offline -- --g3b-frame-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`; `THRUM_V1_FRAMES=240 cargo run -p v1-game --release --locked --offline`. Earlier evidence is in Git.
