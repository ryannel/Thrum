# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Functional gate open.** Numeric targets retired by the 6 Nov 11:15 decision; measured history retained. Lifecycle, display and visual motion remain open. No iPhone result.

| Owner quiet row, `91ad52d` | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Full threaded callback, µs | 625.5 | 1324.0 | 2138.6 |
| Submitted render CPU, µs | 624.5 | 1318.6 | 2132.0 |
| Scene GPU pass-sum, ms | 0.695 | 1.099 | 1.236 |

Owner-run 10:50–10:52 CET, clean worktree, agents paused, builds drained; raw: `tmp/quiet-run-2026-11-06/g3b-threaded.log`.
10k/1.5k movers, 1080p, timing on; 199/200 callbacks, forecasts at 120 Hz, synthetic surface, no real presents.
Upload/encode p50 279.5/239.0 µs dominate. Later shared-load ownership ABBA p95 481.4/521.4/485.1/521.3 µs;
GPU p99 1.083–19.640 ms. No controlled speed conclusion; raw `tmp/g3b-runtime/`. Older history remains in Git.

**Native lifecycle:** Core owns ordered Pause/Resume and immutable replies. Accepted ticks drain before pause;
worker-time resume excludes stopped time. Mac P ignores repeat and retains intent; surface loss never pauses authority.
Metal lifecycle pixels, full checks, Wasm and iOS-simulator compilation pass.
[Pause contact](V1-PAUSE-PHASES.png) / [held samples](V1-PAUSE-PHASES.mp4) are scripted, not live cadence.

**GPU replacement:** Engine rebuilds from retained inputs, without old-device polls/fence waits or world replacement.
Metal replacement tests cover candidates, retry, camera/shadow/exposure and old callbacks. The 10k game keeps exact
paused twilight pixels (peak delta 0), authority and controls; a running world advances without its GPU.
Review found redraw-dependent recovery and lost profiling reports; both are corrected before this checkpoint.
Raw: `tmp/g3b-device-recovery/`. No live loss/drawable/timing claim. One later recovery fixture failed its first-drive
submission assertion (cause unproved); it now permits bounded optional-view retries and fails immediately on overload.

**Chrome function:** `sh examples/v1-game/web/build.sh`, then Node 24 `scripts/check_v1_browser.mjs --case all --output tmp/new-browser-run`.
Chrome 156 / ANGLE Metal M3 Max, actual HTTP-200 Wasm hash checked. Normal, pause hook, resize, deliberate overload,
input and controls each complete ≥600 GPU samples. Controls keep RAF live while paused, edit camera/time, preserve
pause across visibility hooks, resume/re-pause and keep running beyond the profiling window (655 presents).
A temporary missing `host.pause()` mutation fails this row; source restored byte-for-byte. Review caught the former
false pass from cancelling RAF. Raw: `tmp/g3b-browser-six-qualified/`, `tmp/g3b-pause-mutant/`.
Headless function only. A later long Chrome run lost its GPU and showed black without a page fault; handling is open,
as are actual tab changes, live Mac P/loss and iOS lifecycle. Raw loss: `tmp/g3s-directional/chrome.log`.
Repeated capture on shared load showed ~16 ms tails even in the trivial blit, unlike the function batch; no controlled
regression/speed conclusion. Dense-grid aliasing persists. No new allocation or physical-display measurement.

**Trace `ce7aa3f`:** 0.44% of 132.715 ms CA wait spans overlaps this app's GPU work. Span quantiles do not qualify frames; raw: `tmp/g3b-metal-system-trace/`. Native encoding occurs at `finish` after acquisition. No raw bridge selected; mixed raw/wgpu unsupported. Split pre-submit remains parked at `30ae560` (308→335 timing-disabled allocations).

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
