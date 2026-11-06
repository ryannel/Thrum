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

**Chrome function:** build with `sh examples/v1-game/web/build.sh`, then Node 24
`scripts/check_v1_browser.mjs --case all --output tmp/new-browser-run`. Chrome156 / ANGLE Metal M3 Max;
actual HTTP-200 Wasm identity checked. Six healthy rows cover normal, pause hook, resize, overload, input and
controls beyond 600 samples. Two standard `GPUDevice.destroy()` rows cover early loss and loss after 600 samples.
Once the running loop observes loss, it makes no configure/acquire/poll/encode/submit/completion calls. The same world and input
advance; pause holds, hidden gaps exclude time, resume and overload remain explicit. Present/sample counters stop.
The first diagnostic persists in HTML; status writes once, healthy qualification refuses any graphics fault.
Engine authority-only admission shares the normal body/skip accounting. Resume drains accepted work through it.
Actual Metal pending-work destruction, invalid time and partial failed-tick retry pass. A missing callback-flag
mutation fails; source is restored byte-for-byte. Review found no remaining scoped defect.
Raw: `tmp/g3b-browser-loss/`. Earlier long-run loss/black recording: `tmp/g3s-directional/`; rejected as motion evidence.
No reconstruction, actual-tab, physical display or speed claim. Mac P/loss and iOS lifecycle remain open.
Wasm Clippy now runs in `./dev check`: four latent findings→0. Existing ownership-transfer expectations apply
on both targets; two narrow Wasm expectations preserve the native fallible/threaded facade. No policy change.

**Trace `ce7aa3f`:** CA-wait/app-GPU overlap 0.44%, not frame qualification; raw `tmp/g3b-metal-system-trace/`. Raw bridge unselected; split pre-submit parked at `30ae560`.

Reproduce: `rustup run 1.99.1 cargo bench -p v1-game --bench frame -- --g3b-threaded-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Earlier evidence remains in Git.
