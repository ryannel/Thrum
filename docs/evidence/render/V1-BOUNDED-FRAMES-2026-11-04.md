# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Gate open.** Rust 1.99.1 / wgpu 30.0.1, 10k entities / 1.5k movers, 1080p, uncontrolled shared CPU/GPU load. Render CPU p95 ≤300 µs and zero frame allocations remain unmet; displayed cadence, latency and device recovery unqualified; no iPhone result.

| Clock ownership transfer: render CPU, µs | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Game-clock control A1 | 410.9 | 571.3 | 1028.3 |
| Engine clock B1 | 399.9 | 779.3 | 1333.1 |
| Engine clock B2 | 413.3 | 605.4 | 1092.2 |
| Game-clock control A2 | 412.0 | 643.0 | 862.8 |

Each run completed 200/200 offscreen frames. Control `e7095ae`; candidate is this checkpoint's source. Raw output and executable identities: `tmp/g3b-clock/checkpoint-*`. Candidate tails worsen in B1; **CPU non-regression is unproven**. Tick p50 A1/B1/B2/A2 is 56.0/63.2/63.0/58.5 µs, also adverse. Upload and recording dominate; GPU p99 spans 183.959–1478.035 ms. This is a local ownership checkpoint, not performance promotion.

Clock admission, catch-up, failure retention, pause/resume and native handoff now share `thrum-core::ClockedWorld<G>`. Game rules and projection policy stay in the game. Eleven clock tests plus game admission and held-page Metal tests cover failure/recovery; full `./dev check` (Metal and Wasm) passes. Independent code review found no defect; frame-host transfer remains open.

Current threaded fixture, timing enabled, 100 warmed submitted samples at 640×360: render allocations median/max **374/380**, bytes **55,371/55,955**; world **0/0** after wake five. It needed 565 callbacks for 120 submissions and coalesced 125 displays. Timing-mailbox comparison previously matched 120/120 callbacks: channels 380→374 allocations, 57,407→55,479 bytes. These count Rust allocations only and exclude drawable/platform costs; unequal completion cadence does not establish allocation non-regression.

Earlier sparse extraction reduced extraction p95 143.9/144.2→108.9/104.2 µs; whole-frame tails were mixed. A moving capture matched pre-sparse pixels at 640×360. Independent review of 12 live screenshots found no corruption or sampled flashes; grid aliasing and continuous motion remain open. Counts of missed views do not qualify displayed intervals or latency.

**Metal System Trace, `ce7aa3f`, eight seconds.** After trace time 2 s, PID5128 had 525 CA wait spans: p50/p95/p99 0.170/0.683/0.879 ms, 132.715 ms total. Only 0.585 ms (0.44%) overlapped this app's GPU execution; other processes may use it. Span quantiles are not frame quantiles. Native encoding occurs at `finish`, after acquisition. Raw trace/analysis: `tmp/g3b-metal-system-trace/`.

Pre-submit `30ae560` stays parked: timing-disabled allocations 308→335, no CPU win. No raw drawable bridge selected; pinned wgpu forbids mixed raw/wgpu recording. Reproduce CPU: `cargo bench -p v1-game --bench frame -- --g3b-frame-batch`; allocations: `cargo test -p v1-game --test frame_alloc -- --nocapture`. Live cadence needs a separate display run. Earlier evidence remains in Git.
