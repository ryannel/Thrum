# Finite N1 pre-submission queue — 2026-11-03

`thrum-network::outbound::UnsentQueue` owns sixteen fixed 6,944-byte R4
payload slots (111,104 payload bytes) in one allocation plus inline metadata
and chronological slot order. It is created only for an admitted networking
peer. Bound to one match/peer/epoch, each queue reserves 111,832 Rust bytes on
the local target, below the 128 KiB unsent profile by 19,240 bytes; allocator overhead,
per-peer transport and other N1 owners are excluded. No sequence is assigned
while a payload is unsent.

At capture, fixed R4 framing, baseline-bound partition, increasing per-
partition tick, global nondecreasing authority barrier, interest generation,
watermark and a mandatory game-schema/dependency validator run before bytes
or cursors change. The validator explicitly certifies a **complete current
partition** or **partial** state. Only a complete, state-only replacement may
overwrite the last replaceable unit of the same partition incarnation; that
slot moves to the chronological tail. Partial actor updates, lifecycle,
results, events, generation changes and watermark advancement remain ordered
mandatory units. A generation change requires certified complete state.
The 17th live slot refuses without consuming the new tick. The peer owner
must resynchronize or close when it cannot retain mandatory output.

Seven focused tests cover owned copying, cross-partition coalescing order,
partial A/B state preservation, global tick regression, mandatory barrier,
17th-unit refusal, retryable no-mutation sink error and poisoned queue after
an external transfer callback unwinds. Command: `RUSTC_WRAPPER= rustup run
1.99.1 cargo test -p thrum-network --test outbound`. Independent read-only
review found and corrected three faults: partial-state coalescing, global
time inversion and uncertain callback-unwind retry. The final transfer fix
had no further review finding. `RUSTC_WRAPPER= ./dev check` passes for the
checkpoint.

An ignored release probe captures four 88-byte state-only groups for each of
four peers over three ticks, coalesces to four groups per peer, then transfers
those sixteen groups to a no-op sink while reading every outbound byte. It
repeats 2,000 windows per process. Seven Apple M3 Max process samples are in
[the CSV](N1-UNSENT-QUEUE-COST-SAMPLES-2026-11-03.csv); after binding every
queue to its peer and epoch, seven fresh release processes have a median of
**1,939.0 ns per four-peer three-tick window** and four queues reserve
447,328 Rust owner bytes. This is a narrow in-process queue cost,
not an ordinary network workload: it excludes W1 extraction, full 32-actor
groups, mandatory results/events, schema dependency work, envelope, sent
ledger, ACK, sockets, callback tails and RSS. Sixteen slots can exhaust in
four ticks if four mandatory partitions produce one unit each tick; the
selected recipe must bound that burst or close/resync without loss.

`transfer_next` removes a unit only after a successful external callback.
Normal callback refusal is retryable **only if the callback made no external
mutation**. If it unwinds after an uncertain publication, the queue is
poisoned and capture/peek/transfer refuse until peer teardown. The
[ordinary queue-to-sent integration](N1-QUEUE-SENT-TRANSFER-2026-11-03.md)
now reserves framed bytes and ACK metadata before sequence assignment and
successful transfer. A queue is bound to one match, peer and connection epoch;
the integration refuses a mismatched ledger. This queue cost probe excludes
the sent owner, journal identity and remote admission ACK.
