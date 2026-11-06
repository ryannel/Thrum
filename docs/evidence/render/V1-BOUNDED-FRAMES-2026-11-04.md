# G3b checkpoint — M3 Max / portable Metal, 6 November 2026

**Gate open.** Rust 1.99.1 / wgpu 30.0.1, 10k entities / 1.5k movers, 1080p, uncontrolled shared GPU load. Target render CPU p95 ≤300 µs and zero frame allocations remain unmet; cadence, latency and lifecycle unqualified; no iPhone result.

| Timing-mailbox comparison: render CPU, µs | p50 | p95 | p99 |
| --- | ---: | ---: | ---: |
| Channel control A1 | 563.9 | 829.0 | 1128.3 |
| Mailbox B1 | 468.7 | 797.5 | 1327.0 |
| Mailbox B2 | 558.4 | 843.0 | 1142.7 |
| Channel control A2 | 453.7 | 673.9 | 916.6 |
| Mailbox B3 | 359.2 | 583.8 | 798.6 |
| Channel control A3 | 351.2 | 625.6 | 1205.4 |
| Channel control A4 | 362.9 | 568.2 | 727.0 |
| Mailbox B4 | 391.6 | 593.6 | 792.6 |

Each offscreen run completed 200/200 frames. CPU tails move both ways; **speed and CPU non-regression are not established**. GPU p99 varies 97.511–4346.906 ms under contention. Upload, view preparation and recording dominate. All adverse rows remain in `tmp/g3b-mailbox/`.

Matched live-timing allocation fixtures (640×360, 100 warmed samples, both 120/120 submissions): channels **380→374** mailbox median/max allocations; **57,407→55,479 bytes**; world thread **0**. Earlier 376 had different completion cadence; 308 disabled timing. This supports only a sampled allocation reduction, excluding drawable/view/platform costs. Mailbox drop/reuse/concurrency tests and repeated Metal query batches pass; independent review found no lifecycle defect. No public API or resolve ordering changed.

Earlier sparse extraction reduced extraction p95 143.9/144.2→108.9/104.2 µs; whole-frame tails were mixed. The moving capture matched pre-sparse pixels at 640×360. Independent review of 12 live screenshots found no corruption or sampled flashes; grid aliasing and continuous motion remain open. Live early/late runs counted 122/107→1/1 dropped views in 240 presents; a later 1,200-present run counted eight. These counts do not qualify displayed intervals or latency.

**Metal System Trace, `ce7aa3f`, eight seconds.** After trace time 2 s, PID5128 had 525 CA wait spans (411 clusters): p50/p95/p99 0.170/0.683/0.879 ms, 132.715 ms total. Only 0.585 ms (0.44%) overlapped this app's GPU execution; other processes may use it. These are conditional span quantiles, not frame quantiles. Native encoding occurs at `CommandEncoder::finish`, after acquisition. Trace/analysis: `tmp/g3b-metal-system-trace/`.

**Parked pre-submit, `30ae560` / `codex/g3b-scene-presubmit`.** Split submissions raise timing-disabled allocations 308→335 without a demonstrated CPU win. No raw drawable bridge is selected; pinned wgpu forbids mixing raw and wgpu recording on one encoder.

Generic runtime ownership, native pre-submit, device recovery and timing qualification remain open. Premature cancellation fails instead of masking errors. Reproduce: `cargo bench -p v1-game --bench frame --locked --offline -- --g3b-frame-batch`; `cargo test -p v1-game --test frame_alloc -- --nocapture`. Live cadence needs its separate display run. Earlier evidence is in Git.
