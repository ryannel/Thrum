# Selected slow-peer projection ownership probe — 3 November 2026

The [private integration fixture](../../../crates/thrum-core/tests/i1_slow_peer_projection.rs)
consumes real `HeadlessHost::local_with_peer_limit` authority publications.
For one selected inline `Counter(u64)` game, it copies the complete changed
state/lifecycle group and only the target peer's results, immediate events and
contiguous watermark into an owned typed group. The selected value types have
no `Arc`, `Snapshot` or other indirect W1 owner. The source callback report,
including each snapshot, is dropped after projection. The fast peer applies
the full committed group to a replica and acknowledges its own results.

Thirteen 17 ms callbacks commit thirteen changed ticks. The slow peer keeps
twelve projected groups while the authority's eight-live-snapshot admission
continues to advance. Its first entry preserves a committed sequence-2
result and event above a pending sequence-1 gap; the tick-10 entry preserves
sequence 1 when it resolves. Tick 3 spawns an object with its first state and
tick 4 despawns it. The fast replica finishes tick 13 with the expected two
counters and no transient object. A thirteenth slow state-only group triggers
`NeedsBaseline`; no mandatory slow result/event or lifecycle is discarded at
that transition. A new slow command is refused by the private N1 gate before
calling `HeadlessHost::submit`, leaving the authority's two unacknowledged
slow results intact. The fast peer then commits another action in a callback
that services three due ticks, and its replica reaches the authority tick.
Delayed delivery of all twelve retained peer-filtered groups into a separate
replica reaches tick 12 with the expected counters and transient object
retired; that check exercises atomic admission of the selected projection.
An independent 251 ms age case also triggers replacement-baseline state.

The diagnostic outbox preallocates twelve entry slots and accounts their
capacity plus every retained typed column's vector capacity against a
128 KiB logical byte allowance. It checks the 250 ms oldest-entry age before
copying a new group, and skips projection allocation after entry/age refusal.
The actual queue and its allocator metadata/TLS/socket buffers are unbuilt;
the test's twelve-entry cap is a probe parameter, not the selected production
limit. Per-peer projection work is proportional to changed records plus
results/events/watermarks inspected; filtering does not clone unrelated
peer outcomes into the slow queue. At four peers and the selected 20 Hz,
8 KiB/group ceiling, a single full group per peer would copy at most about
640 KiB/s of encoded payload, before baseline/interest work. That analytical
number is **not** a measured cost or a bound on arbitrary game payloads.

`RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline -p thrum-core
--test i1_slow_peer_projection` and `RUSTC_WRAPPER= ./dev check` passed locally.
The independent review agent found no remaining issue in this scoped proof.
This fixture proves ownership
and a narrow safe state-only overflow path. It does not implement N1 wire
schemas, 20 Hz batching, interest filtering, independent reliable-control
retention, full baseline replacement, remote ACK, transport or aggregate
four-peer latency/RSS qualification. A real queue must refuse/close before
losing a required lifecycle, result or event and must preserve authority
progress without retaining W1 snapshots.
