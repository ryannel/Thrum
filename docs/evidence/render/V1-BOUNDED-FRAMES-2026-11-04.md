# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Functional gate open.** Numeric targets retired by the 6 Nov 11:15 decision; measured history retained. Lifecycle, display and visual motion remain open. No iPhone result.

| Owner quiet row, `91ad52d` | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Full threaded callback, µs | 625.5 | 1324.0 | 2138.6 |
| Submitted render CPU, µs | 624.5 | 1318.6 | 2132.0 |
| Scene GPU pass-sum, ms | 0.695 | 1.099 | 1.236 |

Owner-run 10:50–10:52 CET, clean worktree, agents paused, builds drained; raw: `tmp/quiet-run-2026-11-06/g3b-threaded.log`.
10k/1.5k movers, 1080p, timing on; 199/200 callbacks, forecasts at 120 Hz, synthetic surface, no real presents.
Upload/encode p50 279.5/239.0 µs dominate. Later noisy ABBA history: `tmp/g3b-runtime/`; no controlled speed win.

**Native lifecycle:** ordered pause/resume drains accepted ticks; stopped wall time is excluded. Mac P retains intent;
surface loss never pauses authority. Metal checks and iOS-simulator compilation pass.
[Pause contact](V1-PAUSE-PHASES.png) / [held samples](V1-PAUSE-PHASES.mp4) are scripted, not live cadence.

**GPU replacement:** detached engine resources preserve world/controls, current camera/shadows/exposure and exact
paused twilight pixels (peak delta 0). Metal tests cover stale candidates/callbacks, resize refusal and retry without
old-device polls or fence waits. Raw `tmp/g3b-device-recovery/`; no live drawable/loss/timing claim.

**Chrome function:** build `sh examples/v1-game/web/build.sh`; Node 24
`scripts/check_v1_browser.mjs --case all --output tmp/new-browser-run`. Ten rows on Chrome 156 / ANGLE Metal M3 Max;
HTTP-200 Wasm `3ec67a791896d8cb1e7467ab42de99b58fa97ef9d154d6404672538b57cdcb3a` checked against served bytes.
Six healthy rows cover normal, pause hook, resize, overload, input and controls beyond 600 samples. Two actual
`GPUDevice.destroy()` rows cover early/late loss: old-generation GPU work stops, authority/input/pause/hooks/overload
continue. Authority-only admission shares normal accounting; accepted ticks drain before resume.

Two reconstruction rows use the real Restore button: generations 1→2→3, and a failed candidate 2 then retry 3.
A one-shot standard-canvas configure throw is caught; failed attempts release Busy and keep authority live.
Fresh adapters/capabilities, weak detached mailbox, candidate resize and real acquire preflight precede publication.
Zero canvas defers activation; paused player/tick/solar state survives; resumed held input/release works. GPU samples
restart per generation; world/platform counters remain cumulative. UI waits for the first new present call.
Review found perpetual configure retry and premature UI success; both are corrected. Root/independent review sees
enabled loss/retry, disabled preparation, restored twilight and no new corruption; grid/contrast remain provisional.
Raw four UI phases, source identity and logs: `tmp/g3b-browser-recovery/final-qualified/`; earlier loss/mutation:
`tmp/g3b-browser-loss/`. Re-destroying an old device is not stale-callback replay; Metal tests cover mailbox isolation.
Normal 600-frame shared-load encode CPU / forward GPU p50/p95: earlier 0.0/0.2 ms / 0.131/ 0.197 ms;
final 0.1/0.3 / 0.524/2.163; identical-build repeat 0.1/0.2 / 0.197/ 0.197. The broad GPU tripwire is retained;
repeat returned near the quantized baseline without a source change. Cause unattributed, not a controlled speed result.
No full-frame CPU/allocation, physical display or speed claim. Actual tabs, live Mac P/loss and iOS lifecycle remain.
Wasm Clippy runs in `./dev check`; no new expectations or policy changes. The earlier black/lost-sky recording
in `tmp/g3s-directional/` remains rejected motion evidence.

**Trace `ce7aa3f`:** CA-wait/app-GPU overlap 0.44%, not frame qualification; raw `tmp/g3b-metal-system-trace/`. Raw bridge unselected; split pre-submit parked at `30ae560`.

Earlier timing history remains in Git.
