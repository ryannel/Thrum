# Physical two-credit baseline staging oracle — 2026-11-03

`crates/thrum-network/tests/baseline_physical_oracle.rs` is a **private
test-only** storage proof. The baseline image is a separate 256 KiB owner.
The stager physically reserves two exact `64 + 12 + 32,768`-byte chunk slots,
a 184-byte Begin slot and a 72-byte End slot: 65,944 arena bytes. Its inline
metadata plus the arena fit under the 96 KiB baseline-frame sublimit in the
test configuration. Its peak retained frame bytes are 65,872 (Begin plus two
maximum chunks); releasing a chunk credit frees only that exact slot and
leaves its sequence entry as a tombstone. Begin and End remain retained.

The eight-chunk fixture starts at externally supplied sequence 3, uses
sequences 4–11 for chunks and 12 for End. A third chunk refuses while both
credits are occupied without consuming sequence 6. Chunk 1 stages before
chunk 0; its slot is reused for chunk 2 while chunk 0's frame bytes remain
identical. Every image chunk has distinct bytes, so the decoded chunk-2
payload also checks the exact source offset. The duplicate ACK for chunk 1
then cannot free chunk 2's slot.
Wrong peer epoch, baseline ID, unsent index and premature ACKs change no
credit. End requires all chunks to have staged. The tests inspect the actual
encoded Begin/Chunk/End payloads and envelope sequences, a known partial
write offset, a one-byte final chunk, size/count refusal and uncertain-write
quarantine. Two focused tests pass with `RUSTC_WRAPPER= rustup run 1.99.1
cargo test -p thrum-network --test baseline_physical_oracle`.

The existing [rejected-group ACK oracle](N1-BASELINE-ACK-LEDGER-ORACLE-2026-11-03.md)
models rejected ordinary sequence 2 and mandatory identity supersession.
This new proof does **not** join those owners. Its supplied sequence numbers
are model inputs; they do not update `OrdinaryLedger`'s sequence counter.
The declared image hash is a fixture value; this oracle does not compute or
verify image SHA-256. There is no authenticated BaselineAck, unified cross-kind ACK ledger, exact
mandatory identity inclusion, W1 capture, transport or four-peer cost result.
A production integration must reserve frame and tombstone before its single
sequence commit, bind a never-aliased baseline ID to the immutable image,
retain or close on uncertain writes, and retire old ordinary entries only
after verified baseline inclusion. The larger N1 item remains unchecked.
