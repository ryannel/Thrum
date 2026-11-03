# N1-owned queue to sent-frame transfer — 2026-11-03

`OrdinaryLedger::submit_next_from_queue` is a no-socket owner-to-owner
boundary. The queue already holds copied, validator-certified R4 payload
bytes. The ledger rechecks structural framing and exact pending IDs, reserves
one physical frame, sequence entry and journal space, then commits the
application sequence. Only a successful ledger commit lets the queue release
its source slot. A refusal leaves the original payload and sequence intact.
The method checks the queue and ledger carry the same match, peer and
connection epoch before transfer. It returns the exact assigned sequence,
or `None` for an empty queue.

Two focused tests pass with `RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p
thrum-network --test queue_sent`. One captures two replaceable state-only
samples, observes their pre-submission coalescing, then captures a mandatory
result group. Transfer emits the later state sample as sequence 1 and the
result group as sequence 2; exact frames decode with their original ticks,
and one cumulative admission ACK advances the authority result prefix once.
The other test proves wrong epoch, queue/ledger peer mismatch, full 112-entry
sent capacity and a rejected peer leave queued bytes and sequence unchanged.
The generic queue
transfer's unwind poison remains the fail-closed boundary if a callback
unwinds after uncertain external mutation.

The queue validator in this fixture checks only a small local actor mode.
The path structurally decodes the payload at queue capture and again at sent
submission to extract exact result/event IDs. This is bounded repeated work.
The combined probe below includes both decodes but does not isolate the
second decode's marginal cost. Any removal of it needs a
retained exact certificate tied to immutable queued bytes and a whole-path
measurement.

## Isolated combined release cost

An ignored release probe repeats 2,000 three-tick windows in each fresh
process. Four peer-keyed queues each capture four distinct 32-actor partitions
at every tick (48 captures), coalesce replaceable state to four units per
peer, then transfer exactly 16 framed groups through their paired ledgers.
It reads and rolls a checksum over all 30,208 encoded bytes, records known
write completion and ACKs the submitted prefix. Ticks advance by three per
window, actor IDs are distinct by partition and peer epochs differ. Seven
Apple M3 Max release-process samples with rustc 1.99.1 have a **35,982.5 ns**
median (range 35,679.4–45,426.7 ns); four queue-plus-ledger owners reserve
**1,169,376 Rust bytes** throughout. See the [raw samples](N1-QUEUE-SENT-TRANSFER-COST-SAMPLES-2026-11-03.csv).

The full-byte rolling checksum is included in the CPU number. This is a
structural state-only release shape, not a qualified game trace: the small
fixture validator does not prove manifest/dependency closure. The probe
excludes W1 extraction, mandatory records, baseline/sync ownership, TLS,
socket buffering, allocator-call count, RSS and whole-callback tails.
This is **not** W1-to-R4 extraction, a complete game manifest/dependency
certificate, an authenticated peer ACK, baseline replacement, a socket
adapter or a measured four-peer whole path. The larger N1 item stays open.
