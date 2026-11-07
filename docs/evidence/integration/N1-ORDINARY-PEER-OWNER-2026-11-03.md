# N1 ordinary peer owner — 2026-11-03

The registered `thrum-network::peer::OrdinaryPeer` holds one baseline-bound
`UnsentQueue` and one `OrdinaryLedger` privately. Its operations no longer
accept a replaceable queue or sent-ledger argument. The sent ledger alone
assigns application sequences after queue coalescing and exact frame/journal
reservation. A capture refusal or caught schema-hook unwind leaves queue bytes
and cursors unchanged and requires closing this peer. Exact Reject, uncertain
write and caught authority-ACK unwind also require close. Sent frame or journal
capacity refusal retains the queued unit and permits retry after exact ACK.
An authority callback error without unwind retains its exact receipt for retry.

Six no-socket tests cover those cases, including a separate healthy peer that
continues after the failed peer's capture. The frame-capacity fixture fills all
112 ordinary entry slots, confirms the 113th unit retains its bytes and has no
assigned sequence, writes and ACKs the exact prefix, then submits that unit
under the next sequence. The result fixture checks decoded frame ticks and
sequence, stale-epoch refusal, callback retry and authority result retirement.
The tests do not authenticate the caller's ACK or verify a real transport.

The wrapper owns no new payload or frame arena: its retained-capacity account
adds only its Rust value padding/phase to the existing queue and ordinary
ledger accounts. No owner-level release timing, allocator-call count or RSS
measurement has run. The earlier W1 callback/capture measurements exercise the
separate construction owners, not this wrapper. It is an ordinary-only slice:
the game still must supply a whole-peer, schema-closed projection; baseline
image/sync closure, unified cross-kind sequence and exact supersession,
event-bearing replica admission and the complete four-peer mandatory burst
remain unbuilt. Existing split construction types remain public, so this slice
does not yet enforce the final production surface.

Review: independent read-only code review found no defect in phase, identity,
ACK or capacity handling and identified the owner-level ACK-unwind test, which
was added. Run `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-network
--test ordinary_peer` for the focused proof, and `RUSTC_WRAPPER= ./dev check`
for the repository gate.

Base revision: `fb2f4fded7a64630cc46960c2e7acb38a41130f2`. Formatted
`crates/thrum-network/src/peer.rs` SHA-256:
`d5e8763e5658721d494006459614ad16d9df68aedfda932c92ed0a7f007de201`.
Formatted `crates/thrum-network/tests/ordinary_peer.rs` SHA-256:
`13ff9ccae8dfd4581c0f1695cbff8d0824107fbc53e505bb6464a720cda2b96a`.
