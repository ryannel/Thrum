# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Functional gate open.** Numeric CPU/allocation/cadence targets retired by the 6 Nov 11:15 owner decision. Preserve measured history; lifecycle, displayed function and visual motion remain open. No iPhone result.

| Owner quiet row, `91ad52d` | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Full threaded callback, µs | 625.5 | 1324.0 | 2138.6 |
| Submitted render CPU, µs | 624.5 | 1318.6 | 2132.0 |
| Scene GPU pass-sum, ms | 0.695 | 1.099 | 1.236 |

Owner-run 10:50–10:52 CET, clean worktree, agents paused, builds drained; uptime retained in `tmp/quiet-run-2026-11-06/g3b-threaded.log`. M3 Max, portable Metal, 10k/1.5k movers, 1080p, timing enabled. Submitted 199/200 callbacks; forecasts at 120 Hz, synthetic surface texture, no drawable or real presents. Upload/encode p50 279.5/239.0 µs dominate; their p95 values 546.0/521.3 µs are not additive. This missed the former CPU target; it is now a recorded baseline.

**Ownership checkpoint `8bb3979`, shared load:** offscreen ABBA render p95 481.4/521.4/485.1/521.3 µs;
GPU p99 1.083–19.640 ms. B1 has an adverse CPU tail; no speed qualification. These do not replace the quiet row.
Full quantiles, counts and identities: `tmp/g3b-runtime/` and Git.

**Native lifecycle:** Core owns ordered Pause/Resume and immutable replies. Accepted ticks drain before pause;
worker-time resume excludes stopped time. Mac P ignores repeat and retains intent; surface loss never pauses authority.
Full `./dev check` (Metal included), submitted lifecycle pixels, Wasm and iOS-simulator compilation pass.
[Pause contact](V1-PAUSE-PHASES.png) / [held samples](V1-PAUSE-PHASES.mp4) are scripted, not live cadence.

**GPU replacement:** Engine rebuilds from retained inputs, without old-device polls/fence waits or world replacement.
Real Metal destruction/replacement tests cover stale/foreign/cooked candidates, retry, camera/shadow refresh, fallback
exposure and old callback isolation. The 10k game keeps exact paused twilight pixels (peak SDR delta 0), same authority
and held controls; a running world kept advancing without its GPU. Installation precedes incremental-upload telemetry.
Review found redraw-dependent recovery and lost profiling reports; both are corrected before this checkpoint.
Raw logs/captures/contact/held clip: `tmp/g3b-device-recovery/`. No live device-loss, drawable or timing claim.

Live P/loss, iOS worker/lifecycle and Chrome runtime remain open. Dense-grid aliasing persists; no new corruption in
the scoped captures. No new CPU/allocation measurement. Earlier histories remain in Git and named raw directories.

**Trace `ce7aa3f`:** 0.44% of 132.715 ms CA wait spans overlaps this app's GPU work. Span quantiles do not qualify frames; raw: `tmp/g3b-metal-system-trace/`. Native encoding occurs at `finish` after acquisition. No raw bridge selected; mixed raw/wgpu unsupported. Split pre-submit remains parked at `30ae560` (308→335 timing-disabled allocations).

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
