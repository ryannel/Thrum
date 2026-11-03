# Accepted-source owner local cost — 3 November 2026

The first macOS ARM64 native accepted store now uses one cooperating process lock at open, a shared SQLite owner for leases and an indexed top-two selected-head audit. The ignored release fixture admits ten successive three-file edits through the public API. Staging and semantic validation finish before each admission timer. Each iteration then pins the selected revision, copies one scene into an I/O sink, releases the pin, and samples apparent SQLite main/WAL/logical lengths. After ten edits it measures ten fresh owner opens, each including lock acquisition, SQLite open and the selected-head check. The caller's second root assertion runs after the open timer.

Three sequential local Rust 1.99.1 optimized runs reported these min/median/max ranges in milliseconds (ten observations per run):

| Path | Run 1 | Run 2 | Run 3 |
| --- | --- | --- | --- |
| Admission | 22.408 / 29.592 / 32.251 | 24.410 / 31.560 / 34.805 | 23.408 / 33.253 / 33.845 |
| Pin, copy, release | 7.951 / 8.934 / 9.947 | 7.985 / 9.004 / 9.951 | 7.997 / 9.869 / 10.105 |
| Owner reopen | 4.663 / 4.869 / 5.072 | 4.724 / 4.888 / 5.072 | 4.792 / 4.887 / 5.736 |

All three runs sampled the same 4,096-byte main-file length, 1,124,792-byte maximum WAL length and 77,824-byte maximum logical main size after admission. Those are post-admission samples, not transient allocated-block or physical high-water. The fixture has no concurrent writer or reader, so it cannot isolate lock syscall cost, mutex wait, reader tail or source-to-build latency. Admission holds the owner mutex for its entire staged-byte and SQLite path; a concurrent pin may wait behind that work. The measurement excludes capture, semantic validation, cook, compiler, world startup, RSS and device behavior. It is a local diagnostic, not evidence of a throughput target or p99 guarantee.

A second ignored release fixture prepares and validates one 12 MiB binary master before timing. It admits B with an A lease live, then verifies the old scene and times one pinned B binary copy to an I/O sink. Three further local runs measured B admission at **85.874, 89.351 and 89.208 ms**, and pinned 12 MiB copy at **7.633, 7.676 and 7.796 ms**. Each post-admission sample showed a 4,096-byte main file, 12,986,272-byte WAL and 12,673,024 logical main bytes. This workload holds the owner mutex for the entire admission time, but it does not time a concurrent reader trying to acquire it. Uniform input bytes, warm cache, local APFS and `io::sink` make these diagnostic timings, not game or device throughput guarantees. The roughly 13 MB WAL is retained checkpoint debt at that sample, not a measured physical allocation peak.

Both focused release fixtures pass; `RUSTC_WRAPPER= ./dev check` passes after the fixture change. Concurrent reader wait and assembled peak memory remain open before changing the synchronization strategy.
