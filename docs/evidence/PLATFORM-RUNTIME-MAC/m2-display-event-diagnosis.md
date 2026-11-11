# Mac M2 missing display events: planner diagnosis, 11 November 2026

Read-only diagnosis for the scoped re-plan of Mac M2. No recording was made. Scratch copies of the two retained
11 November traces (`tmp/mac-m2/final-window-20261110/{on,off}-traced/pan.trace` in the `mac-m2` worktree) were
exported with `xcrun xctrace export --input <trace> --xpath '/trace-toc/run/data/table[...]'` and the rows counted.
The originals were not opened. [The builder's evidence](../render/mac-m2-20261111/README.md) holds the runs.

## What the traces show

| Finding | Capture OFF trace | Capture ON trace |
| --- | --- | --- |
| Recording mode in the TOC | `Windowed (5 seconds)` | `Windowed (5 seconds)` |
| Kernel-sourced rows span | 5.659–8.626 s | 5.721–8.629 s |
| `MacM2Whole` markers | 4.465–8.566 s | 4.462–8.562 s |
| `MacM2Pan` begins | 5.465 s | 5.462 s |
| System-wide `kdebug` rows, all tables | 7,735,401 | not counted |
| `kdebug` codes `0x85,0x80` (kernel, WindowServer, game) | 252,025 | 224,483 |
| `kdebug` codes `0x34,0x11` `0x34,0x0f` | 0 | 0 |
| `display-surface-swap`, `display-surface-queue`, `display-vsyncs-interval`, `displayed-surfaces-interval` | 0 | 0 |
| `ca-client-presented-handler`, game PID | 349 rows, drawables 496–844, none missing | 343 rows, drawables 505–847, none missing |
| Presented intervals of one refresh (8.328–8.339 ms) | 346 | 341 |
| Presented intervals of two refreshes (16.665–16.667 ms) | 2: drawables 526→527, 544→545 | 1: drawables 558→559 |

Other capture-OFF counts: `cpu-state` 564,583; `context-switch` 164,857; `metal-gpu-intervals` 26,676;
`metal-command-buffer-completed` 12,628; `ca-client-present-request` 348; `device-display-info` 1 (Built-In
Display 1, 3456×2234, built-in, 120 Hz maximum); `metal-known-compositor-process` 1 (WindowServer). Kernel tables
with zero rows besides the display source: codes `0x85,0x2`, `0x85,0x12`, `0x85,0xc2` and `0x85,0xAF`.

## Diagnosis

1. **Authorization is not the cause.** Each trace holds system-wide kernel events from the kernel, WindowServer
   and unrelated processes. A recording refused kernel tracing could not contain them. `DevToolsSecurity` governs
   the debugger-attach authorization prompt; nothing in Apple's documentation ties it to display events. Enabling
   Developer mode is not part of the fix.
2. **One kernel source is silent.** The system-wide table for codes `0x34,0x11` and `0x34,0x0f` has zero rows
   in both traces while neighbouring graphics tables hold hundreds of thousands. Every display table is empty
   with it. Why macOS 26.6.2 with Instruments 27.2 emits none of these events on this Mac is not established, and
   the planner found no documented switch for it.
3. **The recordings were windowed, so they could not have covered the pan anyway.** The Game Performance template
   records as a ring buffer. Under this event volume it kept about 2.9 seconds, which starts after the pan begins
   and drops the whole initial hold. A populated swap table would still have failed the full-interval criterion.
4. **A working, app-attributed presented-time stream is already in both traces.** `ca-client-presented-handler`
   carries one row per game drawable with a presented time on the display's refresh grid. In the capture-OFF trace
   all 349 rows join by ordinal to the game's submission IDs 496–844, with present-call-to-handler lag of
   21–39 ms. The two-refresh interval at drawable 545 coincides with the game's own 15.47 ms gap between present
   calls 544 and 545. The one at drawable 527 has regular present calls around it, so it is a miss the game's own
   log cannot see. Handler callback times jitter between 3.9 and 16.7 ms; the presented-time column does not.

Apple documents the underlying value, `MTLDrawable.presentedTime`, as "the host time, in seconds, when the
drawable was displayed onscreen", and `0.0` for a dropped frame.

## Export quirks the builder must handle

- The `at-time` column is labelled "Requested Time" and is not in trace time. Consecutive differences multiplied
  by 3/125 give nanoseconds; the absolute value needs an offset. The conversion has to be derived and validated
  against the same-process markers, with its error stated.
- The `delta` column stays near 8.33 ms across the two-refresh intervals, so it is not the presented interval.
- `status` is 2 on every row seen. Other values are unobserved and must be reported, not assumed presented.

## Retained tables

[Capture OFF](m2-presented-off-traced.tsv.gz) and [capture ON](m2-presented-on-traced.tsv.gz): drawable ID,
handler trace time, status, raw `at-time` and `delta` for every `ca-client-presented-handler` row. The local
traces have no durable backup, so these are the durable known-answer rows.

## Not pursued

`metalperftrace(1)` appears in the macOS 26.6 SDK manual pages as an always-on per-layer frame-timing recorder,
but no binary is installed on this Mac. It is not a known-working source.

## Sources

- [Analyzing the performance of your Metal app](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app)
- [MTLDrawable.presentedTime](https://developer.apple.com/documentation/metal/mtldrawable/presentedtime)
- [xctrace(1)](https://keith.github.io/xcode-man-pages/xctrace.1.html) and `xcrun xctrace help record`
- [DevToolsSecurity(8)](https://keith.github.io/xcode-man-pages/DevToolsSecurity.8.html)
