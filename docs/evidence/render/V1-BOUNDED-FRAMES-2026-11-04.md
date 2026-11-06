# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Functional gate open.** Numeric CPU/allocation/cadence targets retired by the 6 Nov 11:15 owner decision. Preserve measured history; lifecycle, displayed function and visual motion remain open. No iPhone result.

| Owner quiet row, `91ad52d` | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Full threaded callback, µs | 625.5 | 1324.0 | 2138.6 |
| Submitted render CPU, µs | 624.5 | 1318.6 | 2132.0 |
| Scene GPU pass-sum, ms | 0.695 | 1.099 | 1.236 |

Owner-run 10:50–10:52 CET, clean worktree, agents paused, builds drained; uptime retained in `tmp/quiet-run-2026-11-06/g3b-threaded.log`. M3 Max, portable Metal, 10k/1.5k movers, 1080p, timing enabled. Submitted 199/200 callbacks; forecasts at 120 Hz, synthetic surface texture, no drawable or real presents. Upload/encode p50 279.5/239.0 µs dominate; their p95 values 546.0/521.3 µs are not additive. This missed the former CPU target; it is now a recorded baseline.

**Newer ownership checkpoint `8bb3979`, shared load:** threaded offscreen ABBA control/candidate/candidate/control render p50/p95/p99 µs: 379.1/481.4/1062.8; 378.5/521.4/1535.1; 373.1/485.1/1099.3; 381.0/521.3/1055.5. Submitted 199/199/199/188 of 200; GPU p99 1.083–19.640 ms. B1 has an adverse CPU tail; no speed/non-regression qualification. These do not replace the quiet `91ad52d` row. Raw logs/identities: `tmp/g3b-runtime/`.

**Native lifecycle checkpoint:** Core owns one ordered Pause/Resume slot and immutable replies. It drains accepted
ticks before acknowledging pause; worker-time resume excludes the stopped interval. Mac P ignores repeat and retains
pending intent. Surface loss never pauses authority implicitly. Core backlog/failure/refusal/shutdown/recovery tests,
10k game adapter tests, full `./dev check`, Wasm and iOS simulator compilation pass. One old 55 ms scheduling assumption
was replaced with observed tick progress; control assertions remain unchanged.

The actual Metal presentation path to a 640×360 offscreen surface proxy has pixel-identical paused frames, alpha=1,
a responsive camera and same-world resume. Root/independent review finds no new corruption; dense-grid aliasing
remains. [Contact](V1-PAUSE-PHASES.png): paused, camera while paused, resumed endpoint.
[Four held phase samples](V1-PAUSE-PHASES.mp4) are scripted captures, not live cadence or keyboard qualification.
Live P, device recovery, iOS worker/lifecycle and Chrome runtime remain open. Older allocation/inline ABBA details stay
in Git and `tmp/g3b-runtime/`; no new CPU/allocation measurement is claimed.

**Trace `ce7aa3f`:** 0.44% of 132.715 ms CA wait spans overlaps this app's GPU work. Span quantiles do not qualify frames; raw: `tmp/g3b-metal-system-trace/`. Native encoding occurs at `finish` after acquisition. No raw bridge selected; mixed raw/wgpu unsupported. Split pre-submit remains parked at `30ae560` (308→335 timing-disabled allocations).

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
