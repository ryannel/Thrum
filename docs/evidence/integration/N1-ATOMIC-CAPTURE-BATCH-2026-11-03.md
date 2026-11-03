# Atomic N1 pre-submission capture batch — 2026-11-03

`UnsentQueue::capture_batch` preflights up to four chronological R4 groups
against a copy of its small inline metadata before it writes any payload
bytes. A capacity, framing or game-schema refusal leaves the queue's bytes,
order, partition cursors and submission sequence unchanged. A schema-hook
unwind also precedes all live mutation. The implementation copies no 111 KiB
queue arena; only the metadata candidate is copied. A successful preflight
copies each exact payload into its selected slot and commits the candidate
metadata. Repeated coalescing destinations use the last group's exact bytes
and length. The existing single-group `capture` continues to plan directly
against live metadata without a full metadata copy.

Focused tests cover capacity failure after a would-be coalesce, then successful
retry; framing refusal in the second group; actual schema refusal; hook
unwind; empty and over-four batches; and repeated same-slot state-only groups
with a shorter final payload. The real W1 fixture now submits each peer's
four partitions as one batch and still advances nine changed ticks with one
slow peer. The full `./dev check` passes. Independent read-only review found
no actionable alias or atomicity defect after the edge cases were added.

This is a queue-level transaction, not yet a production whole-peer owner.
The game producer still must prove that its planned partition set is complete
and dependency closed. On a production peer failure after any external side
effect, the owner must resync or close; baseline and sent frames still need one
private identity/sequence owner. The W1 fixture stages four maximum-size R4
payload buffers (4 × 6,944 = **27,776 bytes**) on its stack for one peer at a
time before batch capture. That temporary scratch is separate from the
1,169,376 Rust bytes reserved by four queue/ledger owners; neither number is
process RSS.

## Same-length release control

The prior single-group fixture at commit `07137af` was run in an isolated
temporary worktree with only `WINDOWS` changed from 2,000 to 10,000. The
current batch fixture used the same 10,000 measured windows and ten warmup
windows. Seven fresh Apple M3 Max processes were run for each version and
workload with pinned Rust 1.99.1. The full encoded frame-byte checksum and
frame/byte assertions match between versions. See the [raw samples]
(N1-ATOMIC-CAPTURE-BATCH-COST-SAMPLES-2026-11-03.csv).

| Three-tick four-peer workload | Single-group reference median | Atomic batch median | Observed difference |
| --- | ---: | ---: | ---: |
| Idle state-only, 16 frames / 30,208 bytes | 54,428.4 ns | 54,892.9 ns | +464.5 ns (+0.85%) |
| One event-free result per peer per tick, 24 frames / 45,696 bytes | 75,912.9 ns | 76,887.7 ns | +974.8 ns (+1.28%) |

These are sequential fresh-process observations, not a paired randomized
causal estimate. They show the selected atomic path remains in the same cost
range; they do not establish a universal slowdown, whole-game frame rate or
transport throughput. The result workload invokes real authority retirement
with immediate healthy ACKs. Both workloads exclude lifecycle/event closure,
baseline/sync, socket/TLS, allocator-call and RSS measurement.
