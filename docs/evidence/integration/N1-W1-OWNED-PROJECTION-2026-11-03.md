# Real W1 capture through N1 peer owners — 2026-11-03

The new no-socket fixture consumes real `HeadlessHost` authority
publications from a 128-cell world. It extracts four 32-actor partitions once
per tick, then uses one reusable `CaptureScratch` to encode the complete
partition set for each of four `OrdinaryPeer` owners. The game's local
`PartitionSchema` checks the test mapping. The callback report is dropped
before submission, writes or remote ACK. Over nine changed ticks, one peer
withholds ACK beyond eight retained ordinary frames while authority time and
the other peers continue. The exact match/peer/epoch ACK eventually retires
its result and frames. A separate injected shared-extraction unwind proves
the test coordinator closes all affected peers if that outer game projection
is caught; per-peer capture itself has its own fault guard.

The [raw paired samples](N1-W1-OWNED-PROJECTION-COST-SAMPLES-2026-11-03.csv)
compare the existing split queue/ledger fixture and the new owned fixture in
one release test binary. Each fresh process measures 10,000 three-tick
windows after ten warmup windows. Pair order alternates to reduce systematic
drift. Synthetic 60 Hz callback timestamps run back-to-back on Apple M3 Max
with Rust 1.99.1. `Instant` elapsed time covers actual input submission,
host callbacks, local projection and capture, queue-to-sent transfer, exact
framed-byte checksum and authority result ACK. Setup and final assertions
are outside the timer. Every measured profile asserts final world state,
empty queue/ledger history and exact frame/byte totals.

| Three-tick four-peer profile | Split median ns/window | Owned median ns/window | Median paired reduction |
| --- | ---: | ---: | ---: |
| State-only | 52,882.6 | 50,133.2 | 5.81% |
| Four input results per tick | 73,448.9 | 69,752.1 | 5.03% |

All seven pairs in each profile measured a lower owned time. Both variants
write 16 frames / 30,208 bytes per state-only window or 24 frames / 45,696
bytes per result window, with identical full-byte checksums. This observes
the **complete local fixture paths**; it does not attribute the difference
solely to the owner wrapper. The new path replaces repeatedly initialized
stack payload buffers with a shared heap scratch and adds partition/fault
checks. Process scheduling and unrelated load may affect the times.

The four owned queue/ledger values plus one scratch report 1,197,392 Rust
value/arena/Vec-capacity bytes. The split construction values report
1,169,376 bytes without their transient projection buffers. Those reserve
figures are not peak stack use, allocator metadata or RSS. The fixture has
no sockets/TLS, real 20 Hz pacing, full game manifest/dependency closure,
event/lifecycle burst, baseline replacement, simultaneous game systems or
whole-game callback-tail qualification. The selected projection remains in
an integration test, not a reusable production game declaration.

Reproduce the functional tests with `RUSTC_WRAPPER= rustup run 1.99.1 cargo
test -p thrum-network --test w1_capture`. For each release profile, run
`RUSTC_WRAPPER= rustup run 1.99.1 cargo test --release -p thrum-network
--test w1_capture <name> -- --ignored --exact --nocapture`, where `<name>` is
`measure_real_w1_to_n1_state_only_path`,
`measure_owned_w1_to_n1_state_only_path`,
`measure_real_w1_to_n1_result_burst_path`, or
`measure_owned_w1_to_n1_result_path`. Independent read-only review found no
semantic or benchmark-boundary defect and confirmed the comparison scope.

Base revision: `eec59d4d48f0f8e81ccafbf2b7edfb1bcdf2cfa2`. Formatted
`crates/thrum-network/tests/w1_capture.rs` SHA-256:
`65e8d3bf36040b78bef12e8141e51df7821eb44b45c6f05045e7b8c7b6257855`.
Measured release test executable SHA-256:
`1f49b8bd44f7ff1b5472c827a0ef72bc4910842a6bd8b987d4ac45c5cb830745`.
