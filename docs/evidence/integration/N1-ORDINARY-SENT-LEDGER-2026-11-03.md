# Ordinary sent-frame ledger slice — 2026-11-03

`thrum-network::sent::OrdinaryLedger` owns exact submitted StateGroup bytes
after external game-schema and closure certification. One peer/epoch reserves a
160 KiB circular byte arena and 112 inline sequence entries. Each entry charges
its 64-byte application envelope, exact payload and 12 actual reserved arena
bytes; entry metadata and two fixed pending identity journals are separately
counted by `reserved_capacity_bytes`. The 256-result and 128-event journals
retain exact IDs while frames await client admission. A split frame has two
borrowed physical slices; no uncharged wrap padding is assumed.

Submission checks identity, structural R4 framing, sequence exhaustion, exact
frame/entry/journal space and duplicate pending identities before writing any
owned state or consuming an application sequence. The ledger privately owns
its `AuthorityRetirement`, so an exact pending receipt cannot be replaced by
another helper on retry. Results already admitted in that frontier are also
refused. Event uniqueness outside the pending journal remains an upstream
match-lifetime certification obligation. Known accepted-byte counts
advance the same immutable frame; uncertain external write disposition
quarantines the peer. A cumulative ACK is admissible only for a fully written
ordinary prefix. The ledger stages its exact result IDs in the retirement
helper before invoking the current-authority callback. Normal callback
refusal retains both frame/journal bytes and an occupied receipt for retry;
unwind quarantines both owners. Successful retirement frees the exact frame
and journal ranges. No result is released merely by watermark or group ID.

Seven focused tests pass with `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p
thrum-network --test sent`. They cover envelope content, result 8 admitted
above pending 7 then result 7 advancing the authority prefix through 9,
physical frame split across the arena boundary, byte/entry/result/event caps,
no-mutation refusal, normal callback retry, callback unwind quarantine,
known partial write, wrong epoch and sequence exhaustion. The result-gap test
proves that result 8 remains identified after its frame and journal entry are
retired. It does not claim reuse of that same physical result slot before the
gap closes. The full `RUSTC_WRAPPER= ./dev check` gate passed. Independent
read-only review found the helper-substitution fault; private ownership fixed
it, and the final review found no remaining defect in this slice.

This is an ordinary-only no-socket slice. It does not implement baseline
Begin/Chunk/End credits, out-of-order chunk staging ACKs, replacement
supersession, cross-kind sequencing, authenticated wire ACKs, historical event
dedup, W1 capture or real transport. The selected per-peer total and
four-peer CPU/allocator/RSS, mandatory burst, slow-ACK and whole-callback
targets remain unqualified. The larger N1 build-plan item stays open.

## Isolated release cost

The ignored release probe `measure_four_peer_coalesced_state_only_release_shape`
reuses four preconstructed ledger owners for 2,000 windows per process. Each
window submits, exposes, fully reads and ACKs four 32-actor state-only
partition groups per peer: 16 frames and 30,208 encoded bytes. Partition and
actor IDs are distinct, peer epochs differ, and group ticks advance by three
per window. This is the structural **post-coalescing release shape**; it does
not run the unsent queue, W1 or a game-schema validator. The byte reader uses a
rolling checksum to force all output bytes to be consumed; its work is part
of the number and is not a WSS adapter. Seven fresh process samples on the
Apple M3 Max development host with rustc 1.99.1 have a 25,728.2 ns median
per window (range 25,594.2–28,883.7 ns). All four ledgers together reserve
721,984 Rust value/arena/Vec-capacity bytes throughout the probe. See the
[raw samples](N1-ORDINARY-SENT-LEDGER-COST-SAMPLES-2026-11-03.csv).
This excludes W1 extraction, schema validation, results/events, baseline,
TLS, socket buffering, allocator-call counts, RSS and whole-callback tails.
