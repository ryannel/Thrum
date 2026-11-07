# Bounded authority result-retirement frontier — 2026-11-03

`thrum-network::retirement::AuthorityRetirement` is a server-side no-socket
helper for the gap between client admission of an individual result and the
authority's contiguous result-history acknowledgement. It binds a MatchId,
peer ID and connection epoch, reserves fixed 256-ID arrays for admitted-higher
identities, one candidate and one exact occupied receipt, and rejects an ID
outside the finite 256-sequence window. A newer connection needs a new owner.

`stage` copies one result-ID receipt only after the future sent ledger has
validated an admission ACK. It never overwrites an occupied receipt. `finish`
computes the contiguous frontier from existing admitted-higher IDs and that
receipt, then calls the authority callback **before** committing the frontier.
The callback must check the authority's *current* match/peer/epoch before
calling the core's epoch-less `acknowledge_results`. A normal refusal leaves
the exact receipt for retry if it made no authority mutation. An unwind
poisons the helper and requires peer teardown. Results above a gap move into
the independent admitted-higher set even though their frame/journal slots
may be reused.

Six focused tests cover ACK 8 while 7 is pending, independent simulated
journal-slot reuse and later ACK 7 advancing the authority frontier through
9; occupied receipt refusal; callback error/retry; callback unwind poison;
wrong match/peer/epoch and out-of-window IDs; a stale current-authority epoch;
and `u64` sequence boundary. Command: `RUSTC_WRAPPER= rustup run 1.99.1
cargo test -p thrum-network --test retirement`. The independent design
challenge required the separate admitted-higher owner and current-authority
epoch check. Independent read-only code review found no defect. The full
`RUSTC_WRAPPER= ./dev check` gate passed before the checkpoint was committed.

This helper does **not** authenticate wire ACKs, identify exact submitted
frames, implement an actual result/event journal, or prove journal-slot
reuse with a real ledger. Its callback contract is verified only by the
future host integration. No allocation-call, CPU-tail, RSS or whole-callback
measurement is claimed. The sent-frame design remains a candidate and the
larger N1 item remains unchecked.
