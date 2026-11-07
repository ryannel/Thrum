# N1 three-tick cadence model oracle — 3 November 2026

The [private Rust test](../HISTORICAL-SOURCE.md)
exercises the [provisional outbound cadence rule](../../design/research/N1-OUTBOUND-CADENCE-2026-11-03.md)
against actual W1 publications and the existing `Replica` validation logic.
It is a selected inline model, not N1 runtime code, a wire codec or a
four-player performance result.

One `World<u64>` has two dependency partitions. Over ticks 1–3 the first
updates one controller on each tick; the second spawns a transient entity
with state 7, updates it to 8 and despawns it. The private model stages
committed tick groups without retaining snapshots, coalesces the first
partition's three **complete** state-only replacements to tick 3, and emits
the second partition's tick-1, tick-2 and tick-3 units in order. The two
tick-3 groups share a commit tick and receive distinct contiguous model
envelope sequences; their partition identities stay stable.
The test applies each partition to an independent reference `Replica` lane,
checks the transient after each mandatory transition and compares final
state with W1. A separate counterexample stages partial updates to
different IDs; both units must survive because the later update is not a
complete replacement of the partition.
At tick 3 the mandatory despawn unit also merges the snapshot-derived full
sample for the same partition and tick. The test requires one resulting
unit containing both the despawn and the unchanged static entity; a second
sample unit or a missing static record fails.

In ticks 4–6, a rejection-only sequence-8 result is paired with the
unchanged full controller state at tick 4 while sequence 7 remains pending;
its watermark stays at 6. Sequence 7 commits with event ID 1 at tick 5,
raising the watermark to 8. Tick 6 carries the ordinary full sample. The
reference replica admits all three units, their individually resolved
results, event link and watermarks. An empty unit for the other partition
does not produce a wire group. The two release windows emit four units each,
with model envelope sequences 1–8; the final boundary sample is within its
tick-6 unit.
For every result-bearing unit, the test independently reads the controller
from the committed W1 snapshot at that tick and requires exactly one
matching full controller update. Removing the unchanged controller record
would now fail even though an older replica value happens to match.

The model's `complete_state` marker is supplied by the fixture; no schema
validator or production interest/dependency proof supplies it yet. The two
reference replica lanes validate each partition independently because the
current I1 replica refuses a second group with the same `commitTick`; they
do **not** implement one N1 confirmed replica with per-group sample ages.
The fixture also does not cover interest exit/re-entry, dependent detach,
same-tick spawn/despawn, 64/65-result and 8 KiB boundaries, reliable queue
overflow, remote ACK, four-peer burst, CPU tails, allocator counts or RSS.
Those remain in the build plan's unchecked production gate.

The focused command `RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked
--offline -p thrum-core --test n1_cadence_oracle` passed both tests locally.
The review agent found and then verified fixes for the boundary-merge and
controller-state assertions; its final scoped review had no findings.
The source is small enough to inspect, but the test's owned `Vec` columns
and hand-supplied completeness are not a bounded production serialization
contract. No latency or memory conclusion follows from this correctness
oracle.
