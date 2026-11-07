# Bounded R4 state/lifecycle replica slice — 2026-11-03

The registered `thrum-network::replica` owner consumes the fixed R4 payload
view after an admitted baseline. It has four partition slots, 32 actors per
partition, 128 visible actors total and 256 baseline-bound wire-ID assignments.
The assignment table retains departed IDs. Each partition has its own interest
generation and sample tick; same-tick groups in different partitions do not
imply a shared global sample age. The game supplies a mandatory manifest and
dependency-closure validator. The generic owner additionally checks lifecycle
relations, visibility generations, stable partition home, complete replacement
on interest-generation change, capacities and sample ordering.

The owner reserves actor arrays, assignment rows and one scratch array at
baseline construction. A group is built in scratch, checked, then swapped into
its partition. New assignments are inserted only after all fallible checks.
The `reserved_capacity_bytes` account covers Rust value/Vec element capacity,
not allocator bookkeeping or game-owned validator storage. The 256-ID churn
test confirms that this reserved capacity does not grow through admission and
refusal. The test does not count allocator calls or measure RSS.

Eight focused tests cover two same-tick partitions with different sample ages;
atomic stale, collision, incomplete and schema refusals; full 32-actor exchange;
leave/re-entry generation and home; terminal destruction while outside
interest; 257th assignment refusal; and explicit outcome/watermark refusal.
`RUSTC_WRAPPER= rustup run 1.99.1 cargo test -p thrum-network --test r4_replica`
passes locally. The full `RUSTC_WRAPPER= ./dev check` gate is rerun for the
checkpoint. Independent read-only review found and corrected three contract
faults: same-generation leave/re-entry, premature outcome admission and
invisible terminal destruction. The final changed lifecycle branches had no
remaining review finding.

An ignored, opt-in release test decoded four pre-encoded 32-actor groups and
admitted each to four replicas for 10,000 successive sample ticks per process.
Seven fresh process samples on the local Apple M3 Max are retained in
[the CSV](N1-R4-STATE-REPLICA-COST-SAMPLES-2026-11-03.csv). Their median is
**56,602.5 ns per four-peer/four-partition sweep** (16 group admissions and
four shared decodes). The four replica owners reserve **61,376 Rust bytes**
in total at baseline and the same amount after the sweep. The simple test
schema checks mode/ordinal/flags; it is not a game dependency-closure cost.
The probe excludes encoding, W1 capture, lifecycle/results/events,
envelopes, journals, queue/ACK work, transport, allocator-call counts and RSS.
It establishes no 0.5 ms/tick or full multiplayer performance qualification.

This slice **does not admit** any result, event or nonzero resolved watermark.
It refuses them before state mutation because a bounded progress/dedup owner
must publish outcomes with state atomically. It also does not implement
baseline staging/hash, envelope sequence, N1-owned W1 projection/journal,
coalescing, sent-frame ACK, transport or a four-peer whole-callback
measurement. The remaining N1 build-plan item stays open. The current I1
generic replica remains a separate in-memory fixture and is not silently
reinterpreted as this wire consumer.
