# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Functional gate open.** Numeric CPU/allocation/cadence targets retired by the 6 Nov 11:15 owner decision. Preserve measured history; lifecycle, displayed function and visual motion remain open. No iPhone result.

| Owner quiet row, `91ad52d` | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Full threaded callback, µs | 625.5 | 1324.0 | 2138.6 |
| Submitted render CPU, µs | 624.5 | 1318.6 | 2132.0 |
| Scene GPU pass-sum, ms | 0.695 | 1.099 | 1.236 |

Owner-run 10:50–10:52 CET, clean worktree, agents paused, builds drained; uptime retained in `tmp/quiet-run-2026-11-06/g3b-threaded.log`. M3 Max, portable Metal, 10k/1.5k movers, 1080p, timing enabled. Submitted 199/200 callbacks; forecasts at 120 Hz, synthetic surface texture, no drawable or real presents. Upload/encode p50 279.5/239.0 µs dominate; their p95 values 546.0/521.3 µs are not additive. This missed the former CPU target; it is now a recorded baseline.

**Newer ownership checkpoint `8bb3979`, shared load:** threaded offscreen ABBA control/candidate/candidate/control render p50/p95/p99 µs: 379.1/481.4/1062.8; 378.5/521.4/1535.1; 373.1/485.1/1099.3; 381.0/521.3/1055.5. Submitted 199/199/199/188 of 200; GPU p99 1.083–19.640 ms. B1 has an adverse CPU tail; no speed/non-regression qualification. These do not replace the quiet `91ad52d` row. Raw logs/identities: `tmp/g3b-runtime/`.

Separate inline ABBA p95 606.5/472.2/373.5/524.5 µs, 200/200 each. Threaded allocator A1/B/A2, 100 warmed successful samples: median **374 allocations / 55,371 B** throughout, maxima 380/380/382; world **0/0** after wake five. Each submitted 120 but needed 215/1154/204 callbacks: no throughput parity. Rust counts exclude drawable/Objective-C costs. Inline 600-frame phase median/max allocations: advance 0/0, scene 93/102, submit 233/234.

Core owns clock/lane; thrum-render owns two-slot frame execution. Game owns schema, mapping and camera policy. Retry/mapping review defects have GPU regressions; source identity is checked on all targets. Full `./dev check`, Wasm and iOS simulator compile pass. Review found no remaining defect. Initial/moving stills match bitwise; sequence quality and grid aliasing remain open.

**Trace `ce7aa3f`:** 0.44% of 132.715 ms CA wait spans overlaps this app's GPU work. Span quantiles do not qualify frames; raw: `tmp/g3b-metal-system-trace/`. Native encoding occurs at `finish` after acquisition. No raw bridge selected; mixed raw/wgpu unsupported. Split pre-submit remains parked at `30ae560` (308→335 timing-disabled allocations).

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
