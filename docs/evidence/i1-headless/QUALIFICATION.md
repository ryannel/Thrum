# I1 headless slice: running evidence and limits

1 November 2026. This is local CPU/headless evidence for the bounded [I1 design](../../design/I1-HEADLESS.md), not a whole-engine, multiplayer transport or device qualification. Base checkout was `a45541c3759468a1a555e96278bc4fe4944db945` plus the I1 files in this change. The selected source SHA-256 values at measurement were `world.rs` `00903dd53690c8c07f12ec14d783e19ffed6d668a1712e8158c984cfcaed1201`, `authority.rs` `22cb3714d74534a1ad85b288c8c105498253ff24825b77e4cd2e2c6d0ed183a1`, `replica.rs` `85fd4afcfa9926e88a698c5b1b17daafef076d0b7bbcf13809666cc3ee991d55`, and fixture `i1_cost.rs` `6bc6b9975535b3ac0a2b515f0e046229f206ffe592ac366db5be3a300f685a07`. `rustc 1.99.1` release profile, `macos/aarch64`, macOS 26.6.2 build 25G83. Hardware model/counters and thermal state were unavailable in this sandbox. No other benchmark job was run concurrently by this task.

## Correctness

`RUSTC_WRAPPER= ./dev check` runs the game-owned integration tests and the separate [clock cases](CLOCK.md). The fixture loads an actor, collectible and door; standalone and match authorities run the same `Collect`/`Open` rule. It observes two committed state changes and two distinct event IDs at one tick, with a replica receiving the complete resulting state. It checks unauthorized and old-epoch admission, pending and resolved duplicates, a timed gap with an individually resolved higher sequence, late and stale observation rejection, new-epoch reconnect without replay, immutable old snapshots, stale slot handles, malformed group all-or-nothing admission, a false contiguous watermark, and bounded snapshot backpressure before action mutation. A fresh same-epoch baseline carries outstanding results above the pending gap and their event identities; the replica later advances the contiguous prefix only when the missing result arrives. These tests do not exercise real WSS, serializer bytes, schema/content compatibility, durable results or device lifecycle.

## Cost fixture and observations

Run `RUSTC_WRAPPER= rustup run 1.99.1 cargo run --release --locked --offline -p thrum-core --example i1_cost`. The optimized binary completes one typed action/tick through admission, authority, replica and result acknowledgement for 10,000 measured ticks after 1,000 warmup ticks. A separate world/replica fixture uses 64, 4,096 and 16,384 `Counter` objects, with 0%, 1% and 25% spatially spread changes for 5,000 measured ticks after 500 warmup ticks. It consumes prior snapshot state and compares every final replica object with authority state. Stage timing uses `Instant`; work counts report page references, copied pages/objects and full changed records. `logical_snapshot_bytes` includes page entries but excludes allocator, map, root-pointer, queue and process overhead. No allocation counter or RSS measurement was made. Inputs are synthetic and short compared with a sustained device run.

Two same-binary selected runs are retained in [run A](verified-a.txt) and [run B](verified-b.txt). Times below are microseconds; ranges span the two run blocks, not confidence intervals.

| Workload/stage | p50 | p99 | Count/byte observation |
| --- | ---: | ---: | --- |
| 64, one action, admission through replica/ack | 0.67–0.96 | 0.83–1.08 | 32 objects copied in one page, one full group object, 4 KiB logical snapshot |
| 4,096, idle publication | 0.042–0.084 | 0.084 | No page roots or objects copied; 256 KiB logical snapshot retained by reference |
| 4,096, 1% publication | 6.9–7.1 | 9.4–12.5 | 41 dirty pages, 1,312 objects and 41 group objects per tick |
| 16,384, 1% publication | 29.9–30.1 | 40.0–40.1 | 164 dirty pages, 5,248 objects and 164 group objects; 1 MiB logical snapshot |
| 16,384, 25% publication | 123.3–124.7 | 158.8–210.6 | All 512 pages/16,384 objects copied; 4,096 full group objects |
| 16,384, 25% replica admission | 252.2–255.9 | 303.4–386.2 | Validates then applies 4,096 full records; no wire encoding |

The 0–42 ns measurements approach the clock's observed resolution, so they support only the no-copy work count, not a precise nanosecond speed claim. The same-binary authority p50 ranged 0.67–0.96 µs; p99 and maxima also varied. The 16,384/25% replica p99 ranged 0.303–0.386 ms, so tail improvement requires more controlled measurement before promotion. No GPU, frame callback, transport wait, queue age or sustained thermal behavior is included.

## Representation choice and rejected alternatives

The immutable root shares all pages on an unchanged tick; a changed tick copies `ceil(N/32)` page references and each dirty 32-object page. Short page-size trials on the same fixture used 16, 32 and 64 entries. At 16,384/1%, object clones per tick were 2,624 / **5,248** / 10,496 respectively; at 25% all variants copied 16,384 objects. Short observed 1%-churn publication p50 was roughly 22–26 / **31–33** / 40–41 µs, while 25%-churn p50 was roughly 140–148 / **125** / 115 µs. Thirty-two is the selected compromise for this slice; these short blocks did not isolate compilation/thermal effects and are not a general page-size optimum. The immutable root removed reference-vector copies on idle ticks by count; its measured submicrosecond latency is noise-limited.

The first long replica baseline used an ordered `BTreeMap` for confirmed object lookup: [baseline A](qualified-a.txt), [baseline B](qualified-b.txt). The one-change `HashMap` trial is retained as [trial A](hash-replica-a.txt) and [trial B](hash-replica-b.txt): at 16,384/25%, replica p50 moved from 0.479–0.482 ms to 0.238–0.240 ms, and at 4,096/1% from 2.63–2.96 µs to 1.46–1.58 µs. The final selected variant also validates that each contiguous result watermark is supported by admitted individual results. Its 16,384/25% p50 was 0.252–0.256 ms; p99 varied 0.303–0.386 ms. The one-action integrated p50 ranges overlap or drift, so no small-game latency gain is claimed. A trial changing the temporary duplicate-ID validation set to `HashSet` worsened the 4,096/1% p50 (2.25–2.50 µs versus 1.46–1.58 µs in its contemporary control) and was reverted. These comparisons are local stage results, not an engine-wide speedup.

## Remaining I1 and dependency limits

The current authority bounds pending inputs (8), retained results (256) and live snapshot tick bundles (8), returning backpressure before a blocked tick mutates state. It does **not** yet admit a whole-process snapshot byte budget, memory-reserve concurrent consumers, encode/batch 20 Hz R4 StateGroups, implement reliable result acknowledgement/resync over transport, or run mobile shells. Direct `World::publish` is a lower-level operation whose caller must manage retained snapshots. The game rule fixture is a tested example, not a general component scheduler, source admission or persistence system. I2n still owns actual WSS and four-peer fault/latency qualification. The named Mac/Windows/browser/iOS/Android platform performance entries remain `not_run`.

The bounded host-clock sub-slice has its own [correctness and cost record](CLOCK.md). It does not close the stage scheduler, real callback budget, whole-process memory or device obligations above.

A subsequent read-only review and the [review-fix evidence](REVIEW-RESOLUTION.md) cover event/result consistency, out-of-order acknowledgements, public adapter types and constant-time idle-page detection. The original measurements above remain historical to their stated source hashes.

The later [two-player in-memory match record](TWO-PEER.md) extends this fixture to independent controlled peers and a slow replica. It does not supply real transport or device qualification.

The [native headless host record](HOST.md) closes the initial clock/callback box with validated authority/replica stage order, actual monotonic callbacks and measured overrun handling. It does not qualify a renderer, phone or online host.
