# A2 live instance-update diagnostic

Recorded at commit `a7876fd`; the current `v1-scene` command now runs the later paced frame host.

4 November 2026. Apple M3 Max, Metal, macOS, Rust 1.99.1 release, wgpu 30.0.1. `RUSTC_WRAPPER= cargo run -p v1-scene --release` prepares the 10k-instance HDR/display scene from a committed A1 simulation frame at tick 60, then advances the 1,500 movers to tick 120. It updates the same prepared GPU instance table and target, submits another frame and compares readback: **20,963 pixels changed**. The image test checks a real Metal draw, not just CPU table bytes. A separate opt-in Metal test exercises a sparse one-cube write, a full fallback with mixed valid/invalid rows, duplicate-slot refusal and retention of the prior valid draw.

| Local observation | Result |
| --- | ---: |
| Earlier whole-table repack/compare, then sparse 288 KB write | 615–686 µs CPU across two process runs |
| Changed-slot pack/compare and one 288 KB queue write | 183, 239 and 400 µs CPU across three process runs |
| Changed rows / queue writes / payload | 1,500 / 1 / 288,000 bytes |
| Real Metal pixels changed between submitted frames | 20,963 |
| CPU storage for prepared 10k table | 1.92 MB packed baseline + 1.92 MB staging + 40 KB slot capacity |

The CPU upload interval includes validation, ABI packing, comparison and `Queue::write_buffer`; it excludes `Queue::submit`, GPU transfer and presentation. These isolated process samples are noisy and do not establish a p95 or a frame budget. The path submits one later view after 60 simulation ticks, not a continuously paced frame stream. The producer uses sorted changed slots, the updater coalesces adjacent slots, and more than 64 runs or over half the table changing triggers a full write. CPU vectors reserve their maximum capacity during preparation, though wgpu/driver staging allocation is not covered by a zero-allocation claim.

Three focused CPU tests cover run grouping, baseline row application and shared packing validation. The opt-in real-device test verifies changed pixels, malformed-row degradation and duplicate-slot refusal. Strict crate Clippy and the full `./dev check` gate pass. Worker `SnapshotLease`, owned `RenderPacket`, submission/fence retirement, paced CPU/GPU timing, Chrome/iPhone execution, clustered lights and CSM remain open.
