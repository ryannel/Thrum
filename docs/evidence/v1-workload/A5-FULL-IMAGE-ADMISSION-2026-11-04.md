# A5 checked full visual image admission, 4 November 2026

**Claim and scope.** The running 10,000-entity, four-peer fixture now encodes each 32-actor visual partition as a schema-driven mode-1 full image and reconstructs client state from checked payload bytes at initial admission and drained reconnect. Rebaseline stages all four decoded images before replacing any peer image. The decoder rebuilds sorted wire-ID membership and all six quantized fields into scratch; malformed bytes never become confirmed state. The separate mode-0 delta and selective ACK paths remain as before. This is immediate local admission, with no reliable lane, baseline-loss retry, session replay check or lifecycle record on the wire.

| macOS 26.6.2 arm64, Rust 1.99.1, optimized diagnostic | Result |
| --- | ---: |
| Standard 600-tick run | 16 full images, 10,112 full payload bytes; 796 visual datagrams, 112,928 datagram application bytes |
| Drained reconnect 600-tick run | 32 full images, 20,228 full payload bytes; 792 visual datagrams, 112,756 datagram application bytes |
| Full image size | 632 payload bytes for each unchanged 32-actor partition; reconnect membership changes one image size |
| Warmed 500-tick allocation assertion | zero ordinary Rust heap allocations; reconnect's previously recorded one structural allocation remains |
| Standard complete tick wall p50 / p95 in this run | 51 / 67 µs; load and build state differ from the previous run, so this is not a speedup claim |

The independent tiny-schema oracle checks full-mask bytes, sorted actor-ID differences, changed membership and exact rebuild. Truncation, duplicate ID and incomplete mask refuse. The stdin parser fuzz target now invokes both mode-0 and mode-1 decoders; compilation and an empty-input smoke run passed, with no fuzz campaign claimed. The conditioned visual tests and 600-tick release runs passed, including a reconnect that retires old-epoch packets. The full-image payload is measured separately from datagram application bytes; neither total includes a reliable envelope, transport, IP or TLS overhead.

Commands: `RUSTC_WRAPPER= cargo test -p thrum-network --test state_delta --offline`; `RUSTC_WRAPPER= cargo test -p v1-scene visual_delta --offline`; `RUSTC_WRAPPER= cargo test -p v1-scene --test conditioned_alloc --offline`; `RUSTC_WRAPPER= cargo run --release -p v1-scene --bin conditioned --offline`; add `-- --reconnect` for the second run; `RUSTC_WRAPPER= cargo run -p thrum-network --example fuzz_state_delta --offline < /dev/null`. Source is the commit containing this note; the previous four-partition baseline was `f640034`.

**Open gate.** The client still receives these full-image bytes immediately inside the fixture. Next, a reliable conditioned lane must deliver and acknowledge them, survive a lost initial image, and hold mode-0 deltas until admission. Encoded lifecycle churn, exact controller/result closure, full downstream bandwidth and real transport follow. This result does not check off the complete A5 state-delta path.
