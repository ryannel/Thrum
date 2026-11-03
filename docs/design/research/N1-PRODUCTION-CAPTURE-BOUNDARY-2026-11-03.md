# N1 production capture boundary — construction candidate

3 November 2026. The [network specification](../NETWORK-SPEC.md) and
[C02/C13 contracts](../CONTRACTS.md) own behavior. The
[running W1-to-N1 fixture](../../evidence/integration/N1-W1-CAPTURE-2026-11-03.md)
shows that real host callbacks can feed four peer-owned queues without
retaining world snapshots, including a slow peer and twelve event-free
results per three-tick window. Its hardcoded `Cell` mapping is not a reusable
projection or a game manifest/dependency certificate. This note chooses the
next construction boundary to test; it does not promote N1 readiness.

## Owner and interaction

The game declares its R4 projection schema, controlled actor/partition mapping,
membership/dependency closure and optional event payload rules. F1/W1 remain
the only committed world owner. N1 takes a borrowed publication for the
duration of CPU extraction and copies validated, exact R4 units into its
per-peer owned queue before the callback report is dropped. N1 alone owns
unsent bytes, ordinary and baseline frame/sequence state, mandatory
identities and each peer's overload/sync/close state. A slow remote receipt
never extends the W1 lease. Local and remote inputs still reach one P6
authority; projection is not a second gameplay implementation.

The producer must name one snapshot ID and the corresponding committed result
group. It walks each peer's admitted baseline partitions, emits at most one
group per partition/tick, includes the complete controller state beside any
result/watermark, and proves the declared membership and dependency closure
before the queue certifies `CompletePartition`. It must not convert an unknown
schema or omitted dependent record into a state-only sample. A local
prototype can use the existing `OutboundValidator`; a reusable production
entry needs a game declaration and exact baseline identity rather than a
hardcoded fixture callback.

The C13 atomic admission unit remains one dependency-closed StateGroup. For
the first producer, use a conservative whole-peer capture preflight: validate
the planned partition set and its queue capacity before committing any units,
or enter that peer's explicit resync/close state after a partial refusal. A
temporary multi-group plan consumes separately counted scratch, not hidden
space inside the sixteen queue slots. An in-progress guard must poison/close
the peer if a game projection or schema hook unwinds after partial mutation;
it cannot later resume normal publication with only some partitions of a
tick. Failure for one peer does not roll back authority time or healthy peers.
An older mandatory unit can be retried only from complete N1-owned copies;
a fresh W1 snapshot starts a new checked baseline or closes the peer because
vanished event-time dependencies cannot be reconstructed from current state.
Neither path holds an old W1 lease.
Only the sent owner assigns an envelope sequence, after coalescing and exact
frame/journal reservation. Admission ACKs remain exact, current-match and
current-epoch callbacks; transport receipt alone retires nothing.

The selected first implementation should hold one private per-peer owner
containing queue, sent ledger and baseline/sync state. Exposing independent
replaceable queue and ledger values to a production caller would permit
same-key state substitution. The public surface should express capture,
submission, known-offset write, validated disposition and teardown; the
game-specific projection hook remains narrow. The current separate types
are useful construction slices, not proof that a public split is safe.

An [ordinary-only peer owner](../../evidence/integration/N1-ORDINARY-PEER-OWNER-2026-11-03.md)
now holds one queue and ordinary ledger under a single identity, stops after
capture fault/unwind, exact Reject or uncertain write, and keeps frame/journal
pressure retryable. It has no baseline/sync storage or game projection; its
constructor still relies on an externally admitted baseline. The production
owner remains incomplete until those states and the unified cross-kind
sequence/supersession transaction join this same boundary.

The owner now also has a [bounded complete-tick projection callback](../../evidence/integration/N1-PROJECTED-CAPTURE-2026-11-03.md).
It borrows one reusable four-group scratch, exposes only a push-only writer to
the game hook, checks the exact baseline partition set and common tick, then
atomically copies validated bytes to its unsent queue. Sticky scratch faults
and callback unwind close the peer even if the game hook suppresses an error.
The present API deliberately requires all admitted partitions at each capture;
it does not yet select a sparse producer or prove W1 snapshot/result provenance.

## Cost and alternatives

The selected fixture copies 48 group payloads and retains 16 framed messages
per ordinary three-tick state-only window. Its four queue/ledger owners reserve
1,169,376 Rust bytes; the current atomic batch path measures 54.89/76.89 µs
for state-only/result traffic on Apple M3 Max, not whole-game budgets. Per
tick work scales with peers × partitions × actors for full-state projection,
then with peers × emitted units for validation/copy/sequence and with admitted
mandatory identities for journals. A fully shared global encoded group would
save repeated encoding only where interest, generation, schema and outcome
headers are genuinely identical; each peer still needs its own admission,
capacity and ACK history. A keyed shared encoding cache is therefore a
candidate optimization, not a prerequisite or a source of semantic truth.

Compare the first production extractor against two bounded alternatives on
the same game closure: direct per-peer encode/validate/copy and encode-once
per identical partition sample with peer-specific result/watermark overlay.
Measure complete callback-plus-capture CPU tails, exact bytes, allocator
calls, retained owner bytes and RSS under four healthy peers, one slow ACK,
interest churn and mandatory bursts. A variant is worthwhile only if it
preserves exact closure and materially improves the integrated cost after
cache keys, invalidation and retained bytes are counted. The game-facing
schema API should remain the same whichever internal path wins.

## Build and rejection gates

1. Select one concrete game projection declaration with actual schema and
   dependency closure, including controlled actor, baseline partition map,
   lifecycle and event identity. Reject it if a dependent state set cannot
   fit an atomic R4 group or a declared rate cannot fit the simultaneous
   queue/journal reserve.
2. Build the private peer owner and a complete per-peer capture transaction.
   The [four-group queue preflight](../../evidence/integration/N1-ATOMIC-CAPTURE-BATCH-2026-11-03.md)
   now proves the byte/cursor slice; whole-peer producer and lifecycle state remain open.
   Prove a capacity refusal after a multi-partition plan does not leave a
   partially publishable tick, and a projection-hook unwind poisons the peer,
   while other peers and the authority advance. Account any staging scratch.
   Cross-epoch queue/ledger/baseline substitution must refuse before sequence
   assignment or mandatory retirement.
3. Join baseline Begin/Chunk/End to the same sequence owner and exact
   supersession certificate. For output at or before the baseline capture
   cutoff, the certificate must cover submitted frames and unsent mandatory
   units eligible for exact baseline inclusion before BaselineAck can retire
   them. Post-capture mandatory output in subsequent-sync closure cannot be
   represented by that frozen baseline; it remains owned and replays under
   the new mapping after admission. A rejected group must either drain already numbered
   writes safely or close; no renumbering or forged admission.
4. Run the complete four-peer mandatory burst, one slow peer over more than
   eight changed ticks, and whole callback/queue/transport resource tails.
   Record overload as an explicit resync/close outcome. The current fixture
   verifies only the selected event-free result shape with healthy immediate
   ACKs during timing; lifecycle/event maxima and WSS remain open.

This candidate leaves the [build-plan N1 item](../../BUILD-PLAN.md) open.
