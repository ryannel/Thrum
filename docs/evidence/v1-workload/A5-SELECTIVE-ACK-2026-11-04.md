# A5 first selective state ACK, 4 November 2026

**Scope.** The running four-peer, 10,000-entity/1,500-moving local fixture now sends each peer's one 32-moving-actor partition through a lossy downstream datagram lane and returns checked selective image ACKs through a separately conditioned upstream lane. `InputBatch` and `StateAck` share one monotonic client datagram envelope sequence per peer and epoch. No OS transport or complete 128-interest-per-client session is claimed.

Mode-0 deltas still encode every changed field against exactly one immutable base. Each sent candidate is retained in a 16-image sender ring; the receiver reconstructs in scratch from an exact client base, retains admitted images and sends a 24-byte ACK inside the 64-byte envelope. A newer retained image **and** tick advance the sender base. Older ACKs cannot regress it. An ACK for an evicted candidate is counted and ignored without mutation. The receiver protects the currently referenced base outside its 16-image ring. A same-peer datagram arriving after a newer sample cannot replace it. On reconnect, epoch, image bases, candidate rings and the client datagram sequence reset together; old-epoch packets are discarded.

| Local macOS 26.6.2 arm64 diagnostic | Result |
| --- | ---: |
| 600 world ticks / 10 seconds, four peers | 800 visual sends, 12 conditioned drops, 781 visual deliveries, 7 in flight |
| ACK feedback | 781 sent, 10 conditioned drops, 768 delivered, 3 in flight; 750 sender rebases |
| Visual application bytes | 224,000 downstream total; 280/publication; 44.8 kbit/s per peer at 20 Hz |
| ACK application bytes | 68,728 upstream total; 88/ACK including envelope; excludes IP/transport |
| Visual sample age at delivery | maximum 4 ticks |
| Visual path retained backing storage | 182,112 bytes across four peers and fixed packet slots, excluding allocator metadata and world/input/controller state |
| Complete conditioned tick wall p50 / p95 | 78 / 121 µs; process-local diagnostic under variable load |
| Divan complete 600-tick run | 35.37 ms median across 56 samples; 149–150 allocations/5.884 MB for setup, run and teardown |
| Warmed ordinary ticks | zero Rust heap allocations across 500 checked ticks; reconnect's structural allocation remains separate |

Evidence: `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline`; `RUSTC_WRAPPER= cargo bench -p v1-scene --bench conditioned --offline -- four_peer_600_tick_run --min-time 1 --max-time 2`; `RUSTC_WRAPPER= cargo test -p v1-scene --test conditioned_alloc --offline`. Golden ACK bytes and truncated/extended/zero-identity refusals pass in `state_ack` tests. Forced older ACK, evicted candidate, duplicate/reordered state and more than one ring of lost ACKs pass. The parser fuzz target compiles; no fuzz campaign has run. A read-only review found and resolved two issues: ACK envelope sequence ownership and evicted-candidate handling.

The different fixed-base diagnostic's 42.15 ms Divan median was measured under different local load, so these runs do not establish a speedup. The current fixed rings are a fixture bound, not a qualified network latency envelope. Full-image repair, actual session replay/rate/authentication admission, four partitions/128 interested actors per client, exact controller/result delivery, lifecycle churn, total downstream budget and named-device multiplayer remain open.
