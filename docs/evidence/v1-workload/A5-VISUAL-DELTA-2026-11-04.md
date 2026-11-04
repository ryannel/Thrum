# A5 first running visual delta, 4 November 2026

**Scope.** One fixed-baseline partition of 32 moving actors per peer, four peers, the existing 10,000-entity/1,500-moving W1 game and fixed-seed 2–4-tick/2%-loss in-memory link. The same run retains encoded upstream input, four independently confirmed controllers and prediction. This is a codec and local conditioning diagnostic, not the selected 128-interest-per-client protocol or a device/network budget pass.

The game quantizes three positions to signed 21-bit millimetres and three velocities to signed 20-bit units of 1/16,384 m/s. Nonfinite or out-of-envelope values refuse. N1 encodes game-declared fields as checked LSB-first masks and bit values into caller-owned buffers; one delta reconstructs from its immutable fixed base in scratch. The receiver checks match/epoch/kind/sample age, compares the decoded image with an independently projected W1 sample and only then publishes it. Reordered, duplicate and old-epoch packets do not replace the latest image. Golden bytes, every truncation, noncanonical varints, out-of-range fields and the stdin parser fuzz target cover the first mode-0 codec. No fuzz campaign has run.

| Local macOS arm64 diagnostic | Result |
| --- | ---: |
| Release run, 600 world ticks / 10 s, four peers | 800 visual publications; 8 conditioned drops; 787 deliveries; 5 in flight |
| Visual application bytes | 224,000 total; 280/publication; 44.8 kbit/s per peer at 20 Hz |
| Visual sample age at delivery | maximum 4 world ticks |
| Aggregate visual encode / decode CPU | 2,810 / 441 µs across the run (about 3.51 / 0.56 µs per encoded/delivered packet) |
| Complete conditioned tick wall p50 / p95 | 56 / 73 µs, including W1, upstream input, controller return/prediction and visual delta |
| Divan complete 600-tick run | 42.15 ms median across 48 samples; 140 allocations/5.74 MB for setup, run and teardown |
| Warmed ordinary ticks | zero Rust heap allocations across 500 checked ticks; the structural reconnect retains its separately qualified one-allocation exception |

Commands: `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline`; `RUSTC_WRAPPER= cargo bench -p v1-scene --bench conditioned --offline -- four_peer_600_tick_run --min-time 1 --max-time 2`; `RUSTC_WRAPPER= cargo test -p v1-scene --test conditioned_alloc --offline`; `RUSTC_WRAPPER= cargo test -p thrum-network --test state_delta --offline`. Local host reported macOS 26.6.2/arm64. The packet byte figure is application framing only and excludes UDP/TLS/QUIC, ACK and exact controller/result downstream bytes. The four-actor analytical example in the decision is a different workload and has not been measured. A controlled before/after speed comparison, 128 interests per peer, selective ACK rebasing, reliable repair, lifecycle churn, browser/phone execution and visual acceptance remain open.
