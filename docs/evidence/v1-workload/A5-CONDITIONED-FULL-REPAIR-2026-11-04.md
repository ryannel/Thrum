# A5 conditioned full-image repair, 4 November 2026

**Scope.** In the same 10,000-entity, four-peer W1 fixture, each peer now sends four retained mode-1 full visual images through a 2–4-tick, 2%-loss no-socket link. The client decodes each into private scratch and publishes all four partitions together, then returns one baseline admission ACK. The server holds visual deltas until that ACK. A nine-tick timeout resends the exact four images when an image or ACK is lost; duplicate delivery never admits a second baseline. Same-epoch interest changes and drained reconnect use the same staged path. Exact gameplay results remain in their older local diagnostic path.

| macOS 26.6.2 arm64, Rust 1.99.1, optimized fixture | Result |
| --- | ---: |
| Standard 600-tick run | 16 full images admitted; 16 sends, 0 random full drops, 4 admission ACKs; 11,136 full application bytes including envelopes |
| Standard visual datagrams | 793 sends, 10 conditioned drops, 778 deliveries; 112,640 application bytes, 4-tick maximum admitted sample age |
| Combined measured visual application bytes | 123,776 across four peers / 10 seconds, about 24.8 kbit/s per peer averaged over startup; excludes exact controller/results and transport/IP/TLS overhead |
| Drained reconnect 600-tick run | 32 full images admitted; 32 full sends, 8 admission ACKs, 22,276 full application bytes; 782 visual datagrams, 111,556 datagram bytes |
| Retained visual backing storage | 325,824 bytes across four peers, image rings and fixed link slots, excluding allocator metadata and other N1/W1 state |
| Complete tick wall p50 / p95 in standard process | 73 / 107 µs; local load prevents a speedup claim |
| Divan complete 600-tick run | 32.91 ms median across 59 samples; 156 setup/run/teardown allocations and 6.028 MB |
| Warmed 500-tick allocation assertion | zero ordinary Rust heap allocations; the previously recorded reconnect structural exception remains |

Forced-loss tests remove one initial full partition or its admission ACK after enqueue. The missing partition keeps all four client images unconfirmed; retry eventually admits them once and resumes later deltas. Malformed full payload and ACK packets are shed and counted without stopping the world tick; retry recovers. A malformed duplicate cannot poison already staged scratch, and an old full packet cannot stage a new interest generation. Existing tests cover old epoch/generation deltas, reordered/duplicate deltas and retained bases. These fault injections are deterministic tests, separate from the 600-tick seeded run, whose full-image lane happened to lose no packets.

Commands: `RUSTC_WRAPPER= cargo test -p v1-scene visual_delta --offline`; `RUSTC_WRAPPER= cargo test -p v1-scene --test conditioned_alloc --offline`; `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline` (add `-- --reconnect` for the second run); `RUSTC_WRAPPER= cargo bench -p v1-scene --bench conditioned --offline -- four_peer_600_tick_run --min-time 1 --max-time 2`. Source is the commit containing this note; the preceding local admission checkpoint was `4a5db43`.

This is a bounded local reliable-lane model. It does not authenticate sessions, use sockets or prove the platform transport. It does not encode lifecycle records, exact controller/results, or a complete downstream byte budget. The parent A5 completion gate stays open.
