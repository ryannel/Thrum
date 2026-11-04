# A5 four-partition interest and image-pool diagnostic, 4 November 2026

**Scope.** The same running 10,000-entity/1,500-moving W1 fixture has four peers. Each now holds four sorted 32-actor visual partitions: four controlled movers and 124 static actors. A per-connection 16-candidate/16-client image history serves all four partitions, with unique connection-wide image IDs and one datagram sequence in each direction. The base is externally staged in this no-socket fixture. Exact controller/results downstream, encoded baseline/lifecycle and real transport remain open.

One quantized W1 projection per partition is shared across identical peer interests on each 20 Hz sample. An unchanged partition sends no datagram unless an earlier divergent sample may have been admitted; in that case a newer zero-record sample corrects the client and stops after the matching ACK. A same-epoch interest rebuild stages all four images before mutation, advances generation and ignores delayed old-generation packets. A drained controller reconnect changes the actor's match wire ID, resets the new connection and rebuilds the other peers' interest generation. A late-partition projection failure leaves all four visual partitions, history and sequence unchanged.

| Local macOS 26.6.2 arm64 diagnostic | Result |
| --- | ---: |
| Release standard run, 600 world ticks / 10 seconds | 796 visual sends, 12 conditioned drops, 780 deliveries, 4 in flight; 2,404 unchanged partition opportunities skipped |
| Visual downstream application bytes | 112,928 across four peers; about 142/send and 22.6 kbit/s per peer at 20 Hz; excludes transport and exact controller/results |
| ACK feedback | 780 sent, 10 drops, 764 delivered, 746 sender rebases |
| Maximum admitted visual sample age | 4 world ticks |
| Retained visual backing storage | 273,184 bytes across four peers and fixed packet slots; excludes allocator metadata, W1 and input/controller state |
| Aggregate visual projection / encode / decode CPU | 3,050 / 160 / 253 µs across 600 ticks; projection is shared once per partition, including static groups |
| Complete conditioned tick wall p50 / p95 | 74 / 104 µs in this process; local load varied between runs |
| Divan complete 600-tick run | 32.41 ms median across 61 samples; 154–155 allocations/5.975 MB for setup, run and teardown |
| Warmed ordinary ticks | zero Rust heap allocations across 500 checked ticks; structural reconnect retains its separate one-allocation exception |

Commands: `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline`; `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline -- --reconnect`; `RUSTC_WRAPPER= cargo bench -p v1-scene --bench conditioned --offline -- four_peer_600_tick_run --min-time 1 --max-time 2`; `RUSTC_WRAPPER= cargo test -p v1-scene --test conditioned_alloc --offline`. The reconnect process completed 600 ticks, rejected two delayed visual packets from retired epoch/generation and matched all controlled positions. A 120-tick adversarial test moved one actor in every partition, produced 640 state packets, forced cross-partition reorder, made over 500 rebases and recorded no unknown ACK; all four final base IDs per peer stayed distinct. Focused tests also cover lost-ACK change→return correction and quiescence, delayed ACKs beyond ring residence, and atomic refusal when the last static actor is absent.

This is a synthetic local byte/CPU diagnostic. Packet/IP/TLS overhead, active 128-actor bandwidth, full-image repair on missing base, encoded lifecycle, real session replay/authentication, exact reliable controller/result closure, visible multiplayer correction and browser/phone multiplayer remain unqualified. Earlier one-partition medians were measured under different local load; these runs do not establish a speedup.
