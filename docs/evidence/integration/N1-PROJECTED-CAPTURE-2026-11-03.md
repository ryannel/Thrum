# N1 bounded projected capture — 2026-11-03

`OrdinaryPeer::capture_projected` accepts one game projection callback and a
caller-owned, reusable `CaptureScratch`. The callback receives only a borrowed
`CaptureWriter` with `push(GroupInput)`. The scratch allocates four exact
6,944-byte R4 payload slots once (27,776 payload bytes); inline lengths,
partition IDs, ticks and fault state are counted separately by
`reserved_capacity_bytes`. The caller may reuse this scratch sequentially
across peers, so it need not reserve one payload arena per peer. The callback
cannot replace the scratch or clear an encoder fault through its writer.

After projection, the owner requires exactly one group for each partition in
its admitted baseline and one common commit tick. It then invokes the queue's
atomic batch preflight, including structural decode and the supplied game
validator, before any N1 queue bytes/cursors commit. The peer enters
`RequiresClose` before invoking the game hook and returns to `Ordinary` only
after the whole capture succeeds. A projector refusal, caught unwind, invalid
plan, schema refusal or ignored scratch capacity/framing error leaves the
queue unchanged and consumes no envelope sequence. Other peers can continue.

Nine focused owner tests cover successful two-partition capture, reuse of the
same scratch for a different peer, missing/duplicate partition and mixed-tick
refusal, projector error/unwind, schema refusal, and the ignored fifth push
after all four expected groups. The earlier ordinary-owner cases still check
ACK retry, exact Reject, known writes and frame pressure. A compile-fail doc
test verifies that the callback's writer cannot be replaced with a fresh
scratch. Independent read-only review found the ignored-fault and
replaceable-scratch bypasses; both were corrected and re-reviewed with no
remaining findings.

This is a structural no-socket boundary. The game hook and validator are
still supplied by the caller; tests do not establish W1 snapshot/result
provenance, manifest/dependency closure, lifecycle/event identity, a complete
four-peer mandatory burst or an actual production game recipe. The current
first-profile API deliberately emits all admitted partitions per tick; a
sparse producer and its closure rules need separate selection. No release
timing, allocation count, RSS or integrated callback measurement has run for
this new API. The earlier W1 tail numbers apply to the older fixture path.

Reproduce with `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-network
--test ordinary_peer`, `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p
thrum-network --doc`, and `RUSTC_WRAPPER= ./dev check`.

Base revision: `7bd216de63b174c4e2819106abb1a82ead11dda0`.
Formatted `crates/thrum-network/src/peer.rs` SHA-256:
`dfa7ac5c39e401a527888615e29f5571b0e49df26a0d4e5d83709118e622a75b`.
Formatted `crates/thrum-network/tests/ordinary_peer.rs` SHA-256:
`2a4c168401c3925086e4fc16bc4ec0f08ec3129083c1ba57a34db849dacb5cd7`.
