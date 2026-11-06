# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Gate open.** Rust 1.99.1, wgpu 30.0.1, 10k entities / 1.5k movers, 1920×1080. Three GPU agents and Blender shared this Mac. These short diagnostics do not qualify the ≤0.3 ms frame CPU, zero allocations, ten-minute presentation-interval, latency or lifecycle rows. No iPhone timing is recorded.

| Sparse extraction A/B: complete render-frame wall, µs | p50 | p95 | p99 | Extract p95 |
| --- | ---: | ---: | ---: | ---: |
| Full control A1 | 744.5 | 1331.8 | 1677.5 | 144.2 |
| Sparse B1 | 638.8 | 1257.4 | 1446.7 | 104.2 |
| Full control A2 | 764.2 | 1330.2 | 1455.6 | 143.9 |
| Sparse B2 | 689.2 | 1325.0 | 1677.4 | 108.9 |

Each offscreen run submitted 200/200 frames and uploaded 300k rows / 57.6 MB. One earlier post-fix pair was 2.7% slower for the whole frame. Extraction improves consistently; frame tails remain load-sensitive and over budget. Time Profiler identified full-row extraction and scene application. Upload, view preparation and command recording now dominate.

Live early/late A/B: 240 presents per run, 241–242 display callbacks; early counted 122/107 dropped views, late 1/1. Worker wall p95 was 0.767/0.766 ms early and 1.268/1.777 ms late; early excludes its acquisition thread. A later breakdown measured worker/acquire/upload+encode p95 2.165/1.424/0.780 ms. The final 1,200-frame capture run had 1,223 callbacks, eight drops and p95 2.830/1.132/2.023 ms respectively. These counts and elapsed measures establish no presentation-interval or latency result.

The warmed extractor asserts zero allocations. Loaded threaded fixtures still report 308 median render-callback allocations and zero world-thread allocations; collection outside that fixture is not covered. The moving-scene capture matched the pre-sparse build byte for byte at 640×360. Independent review of 12 live screenshots found no corruption or sampled flashes; continuous motion remains unqualified.

An independent reviewer found a capacity-order hole in sparse extraction; staging plus full reconstruction fixes it. A second review found resize-during-acquire and blocked-close regressions; surface generations and nonblocking explicit close address them. Scripted live lifecycle exercise timed out in macOS UI automation, so resize, occlusion, loss and blocked-acquire recovery remain open. Scene commands are encoded before acquisition but still submitted with presentation. `FrameHost` remains game-owned.

Reproduce with `cargo bench -p v1-game --bench frame --locked --offline -- --g3b-frame-batch`, `cargo test -p v1-game --test frame_alloc -- --nocapture`, and `THRUM_V1_FRAMES=240 cargo run -p v1-game --release --locked --offline`. Raw logs remain in `tmp/g3b-sparse-final-*` and `tmp/g3b-late-surface-*`. Earlier evidence is in Git.
