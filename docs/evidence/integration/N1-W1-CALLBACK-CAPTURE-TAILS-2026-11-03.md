# W1 callback plus N1 capture tails — 2026-11-03

> Status after the code-quality cleanup: the ignored timing probe described here was removed from the active test tree. This page retains its historical result; the former test source is at `git show 6493763:crates/thrum-network/tests/w1_capture.rs`. The quoted command is a historical reproduction recipe and needs a maintained benchmark before use again.

This release-only, no-socket fixture times each `HeadlessHost::on_callback`
through complete four-peer R4 projection, schema-validated atomic queue
capture and callback-report drop. It uses the actual 128-cell authority, four
32-actor partitions and the N1 queue/ordinary ledger owners from the
[W1 integration proof](N1-W1-CAPTURE-2026-11-03.md). It samples the work F1's
`observed_elapsed` omits after its return: N1 extraction and owned-byte
capture. The timer also includes the host callback itself and local fixture
assertions. All samples are synthetic 60 Hz timestamps run back-to-back on
Apple M3 Max, with hot caches and pinned Rust 1.99.1.

One-tick profiles measure 9,990 callbacks after 30 warmup ticks in each fresh
process. The idle profile submits no input; the result profile submits one
actual input for each of four peers before every tick. Queue-to-sent transfer,
complete encoded-frame checksum and exact ACK run after the timer at each
three-tick window. The result profile asserts all four controlled cells have
advanced and authority result history is empty at the end.

Two- and four-due-tick profiles stage one **timed** input per peer per intended
tick before the callback. The host reports exactly two or four committed
publications and zero queued ticks; the timer covers capture of every report
publication before drop. These profiles measure 10,000 ticks over 5,000 or
2,500 callbacks after ten warmup callbacks. Queue-to-sent transfer, full
frame-byte checksum and ACK happen after each callback, outside the timer.
That drain pattern is not the normal 20 Hz release cadence. Input submission
is outside every timed section.

Seven fresh release processes per profile give the following medians of each
process's distribution. `p99` uses nearest-rank indexing; all values are
nanoseconds **per callback**, so the due-tick rows contain more work.

| Profile | Median process mean | Median process p50 | Median process p99 |
| --- | ---: | ---: | ---: |
| One idle tick | 5,733.3 | 5,583 | 7,958 |
| One tick, four results | 6,664.7 | 6,541 | 9,250 |
| Two due ticks, eight results | 13,142.5 | 12,875 | 18,708 |
| Four due ticks, sixteen results | 26,813.6 | 26,083 | 36,750 |

The [raw samples](N1-W1-CALLBACK-CAPTURE-TAIL-SAMPLES-2026-11-03.csv) retain
each process's p99.9 and maximum as well. Isolated maxima include OS noise
and are not an admitted service guarantee. The fixture allocates its sample
vector before timing; it does not count allocator calls, RSS, GPU work, game
schema closure outside the local validator, lifecycle/events, transport/TLS,
or simultaneous render/physics/environment work. It does not prove the
whole-game 0.5 ms/tick budget or the full four-peer mandatory burst. The
current F1 report still excludes N1 capture in production; this external
timer establishes the gap for a later shared callback budget.

The measured base revision is `cd7af5adf04ac870335187ae52e9f49944fca7e4`.
The final formatted `crates/thrum-network/tests/w1_capture.rs` has SHA-256
`b4eaed5eb8de1b49224df4082394df3c211941ba9307fd53e0da9b499dde78b1`;
the measured release test executable has SHA-256
`f010d00bd6eb9d16b9904ef4f32750610a9da1805f7d15945752d8fcb2096a28`.
All 28 raw rows were sampled from this executable in separate processes after
the final fixture formatting.

Reproduce with `RUSTC_WRAPPER= rustup run 1.99.1 cargo test --release -p
thrum-network --test w1_capture <test-name> -- --ignored --nocapture`, using
`measure_w1_capture_idle_callback_tails`,
`measure_w1_capture_result_callback_tails`,
`measure_two_due_tick_capture_tails`, or
`measure_four_due_tick_capture_tails` as `<test-name>`.
