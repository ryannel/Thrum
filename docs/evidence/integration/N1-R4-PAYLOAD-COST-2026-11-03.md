# Isolated R4 payload codec cost — 2026-11-03

The ignored `measure_r4_payload_cost` test in
`crates/thrum-network/tests/r4_payload.rs` measured the pinned release build
on MacBook Pro Mac15,11 (Apple M3 Max, arm64). It runs seven fresh test
processes with 100,000 ordinary iterations and 30,000 maximum-count
iterations per process. Each ordinary payload carries 32 actors (1,824
payload bytes). Each maximum payload carries 32 actors, 32 lifecycle records,
64 results and 64 events (6,944 payload bytes). IDs are distinct. The
encode loop writes caller-owned bytes; the decode loop validates framing and
result shape, then reads **every field** of every actor, lifecycle, result
and event into a checksum. The harness black-boxes encode inputs and the
destination, and the decode input; the checksum preserves decoded work.

Command: `RUSTC="$(rustup which --toolchain 1.99.1 rustc)" RUSTC_WRAPPER= rustup run 1.99.1 cargo test --release --locked --offline -p thrum-network --test r4_payload measure_r4_payload_cost -- --ignored --nocapture`.
The [14 raw case samples](N1-R4-PAYLOAD-COST-SAMPLES-2026-11-03.csv)
include elapsed nanoseconds per call and stable checksums. Codec source
SHA-256: `a4bd28277006bf19aa83a558f73d317593dffe4abd451f34848e45c384c44c1b`;
test source SHA-256:
`68e0e0a66ea842c5b0d2f66164e617672ea25effcdd94313e4569b825da453bd`.

| Payload | Encode median (range) | Decode and scan median (range) |
| --- | ---: | ---: |
| Ordinary 1,824 B | 44.9 ns (44.3–48.5) | 321.6 ns (320.4–327.8) |
| Maximum 6,944 B | 174.2 ns (167.6–188.2) | 667.8 ns (662.1–796.2) |

These are isolated native CPU timings, not an N1 callback or network rate.
They exclude source projection, result/event journaling, envelope validation,
partition admission, baseline handling, allocation-request counting, RSS,
transport and four-peer concurrency. The maximum decode range has one slower
process; no precise small-difference comparison or tail guarantee follows.
The full 0.5 ms/tick N1 service target and revised
R4 mandatory burst remain unqualified.
