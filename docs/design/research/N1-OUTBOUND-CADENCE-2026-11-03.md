# N1 outbound ownership and 60-to-20 Hz cadence — provisional construction design

3 November 2026. This proposes a resolution for a construction gap in the selected
[first network profile](../NETWORK-SPEC.md): F1 commits at 60 Hz, while each
ordinary full-state group is sampled at 20 Hz. It is a **provisional design**
for the first direct-connect R4 recipe, not a running N1 implementation or a
new wire protocol. C02 and C13 keep W1 publication, authoritative results
and replica admission as separate owners. The [private inline ownership
probe](../../evidence/i1-headless/SLOW-PEER-PROJECTION-PROBE.md) demonstrates
only a selected two-peer handoff; it does not qualify this cadence algorithm.
The [private cadence model oracle](../../evidence/integration/N1-CADENCE-MODEL-ORACLE-2026-11-03.md)
now exercises ordered mandatory units, complete-state-only coalescing and a
result-bearing unchanged controller across two three-tick windows. Its
separate reference replica lanes do not close the production multi-group
consumer or burst-cost gates below.

## Required outcome and owner boundary

F1 supplies one complete committed tick group and immutable W1 snapshot at
each successful authority barrier. N1 synchronously derives schema-checked,
peer-specific **owned** state, lifecycle and mandatory outcome/event records.
It releases every W1/callback lease before waiting on a socket, remote ACK,
content load or baseline retry. N1's per-peer state and the shared serializer
have finite entry/byte/work admission. No arbitrary `T: Clone` proves this
boundary: a game value may hide a snapshot or product lease in an `Arc`.
The selected codec must copy declared scalar/byte fields into N1-owned
records with a checked maximum size before enqueueing. W1's generic Rust
payloads and storage layout never become the wire schema.

Each admitted peer has a baseline-bound stable `groupId` for each dependency
partition, a per-partition `interestGeneration`, and an ordered
outcome/event journal, distinct from unsent replaceable state and transport
buffers. Results and mandatory events cannot be lost by state coalescing.
The controlled actor's result set and contiguous watermark are emitted with
its exact full state at the group's `commitTick`. Header ACK releases only
an atomically admitted StateGroup; a replacement BaselineAck releases exactly
its included result/event set. A successful socket write or transport-level
receipt does not release the underlying mandatory journal.
The sent-frame ledger has its own 256 KiB/128-entry/1 s pressure trigger,
with ordinary frames limited to 160 KiB/112 entries and 96 KiB/16 entries
reserved for replacement-baseline framing. At most two 32 KiB chunks are
outstanding; their independent `BaselineChunkAck` staging dispositions
replenish credits even when a rejected earlier group blocks the cumulative
header-ACK prefix. Chunk ACK releases exact frame bytes but retains a small
sequence tombstone until final disposition. The ledger is counted
simultaneously with unsent, transport, sync and mandatory journals. A valid
replacement BaselineAck may retire an earlier rejected/unadmitted frame as
**superseded**, while retiring only the exact mandatory IDs included or
explicitly expired by that baseline. A failed replacement closes the peer;
it never forges admission of rejected state.

The current I1 `Replica` is only a **one-group-per-tick** consumer: it
requires every next `commit_tick` to exceed its preceding tick and stores
one global tick. The R4 profile has four independent groups per ordinary
publication and may deliver several at the same commit tick, plus groups
with different tolerated sample ages. The I1 API cannot admit that stream
as-is. I2n needs one confirmed replica owner with an explicit multi-group
admission rule and per-group sample age (or an equivalent checked mapping),
while keeping each group atomic and ordered within its partition. Required
cross-partition dependencies must be brought into one admissible group; if
they cannot fit, that recipe is refused rather than publishing a partially
valid tick. Merely relaxing `tick > previous_tick` to
`>=` would leave the global tick falsely suggesting all entities share that
sample age. This consumer contract must be reconciled before constructing
the four-group codec or treating the private slow-peer probe as R4 proof.

## Provisional cadence rule

1. Partition one peer's admitted interest into dependency-closed groups of
   at most 32 entities and one complete 8 KiB wire message each. `groupId`
   names the stable partition from the admitted baseline; it is not a
   publication number. An authored
   relationship that needs same-barrier state must stay in one group. Interest
   changes occur only at an authority barrier and carry their own visibility
   generation; a partition boundary cannot split required first/dependent
   state to evade the group limit.
2. On **each** committed 60 Hz tick, synchronously copy changed records and
   mandatory lifecycle/dependent state, individually resolved results,
   watermarks and events into bounded N1-owned columns. A result-bearing
   unit also copies the complete controlled actor state **even when the
   action rejected and changed nothing**; a dependent event copies its
   required state. Mandatory units
   retain that tick's `commitTick`, ID and interest generation. On the
   three-tick release boundary, also sample each ordinary full group from
   that tick's current W1 snapshot into N1-owned records. If the same group
   already has a mandatory unit at that tick, this full sample joins that
   one atomic unit rather than creating a fourth group. Every copy
   finishes before releasing its W1 snapshot and callback report. Purely
   replaceable updates need no older W1 pages.
3. Treat 20 Hz as a bounded **batch-release cadence**, not permission to
   collapse three authority barriers into one state. At each three-tick
   boundary, emit every mandatory tick unit in chronological order. Coalesce
   only intervening state-only units, retaining their latest complete
   replacement state and exact tick. Allocate contiguous **envelope message
   sequences** after coalescing and final ordered submission. The bounded
   ACK ledger binds each submitted sequence to its exact group and retained
   result/event IDs; it never retires by stable `groupId` alone. Never
   relabel a tick-1 lifecycle as tick 3 or concatenate
   changes from different ticks into one current `StateGroup`: the existing
   replica rejects duplicate entity IDs within one group.
4. A spawn with first state at tick 1, update at tick 2 and despawn with
   dependent changes at tick 3 therefore emits the tick-1 and tick-3
   mandatory units, in order. The replaceable tick-2 update may be omitted
   unless an event or declared consumer requires that exact intermediate
   state. A despawn followed by re-entry retains both visibility-generation
   transitions. An interest leave/re-entry increments only its partition's
   interest generation; a repartition requires an atomic replacement baseline. A
   same-tick spawn/despawn is represented by W1's single
   committed publication plus any independently meaningful event; N1 does
   not invent an intermediate visible entity. An event requiring that
   vanished entity's intermediate state must carry sufficient self-contained
   event data or the recipe is unrepresentable; the final W1 snapshot
   cannot reconstruct that transient state.
5. An event schema declares whether its consumer needs a named entity or
   physical sample at the event tick. If so, the event-tick mandatory unit
   contains that dependency closure. A self-contained event still retains
   its original event ID/tick when delivered later. A recipe that cannot fit
   its closure in one 8 KiB/32-entity unit fails admission.
6. Preserve one tick's complete controller/result closure. If accumulating
   until the next release would exceed 64 results, 64 events, reliable
   journal capacity or the 8 KiB message limit, select an earlier complete
   release cut, with its already copied exact state. Never split or truncate
   a controller's results to fit. The recipe must bound same-tick mandatory
   output before inputs are sequenced. If no complete unit can fit, refuse
   new commands before sequencing; already accepted work still resolves,
   and an explicit peer resync/close handles an unserviceable peer. F1 never
   retries or skips a committed tick to repair N1 capacity.

Three mandatory authority ticks can produce three wire units **per group**
in one 20 Hz release window. The R4 bandwidth and quarter-second burst
calculation in NETWORK-SPEC counts four full groups at each ordinary 20 Hz
sample and does **not** qualify these extra envelopes/full states. The
production recipe must derive a simultaneous bound on mandatory group
frequency, bytes and CPU, then either admit it under the 256 KiB/s and
65,536-byte burst profile or choose a narrower recipe. The 0.5 ms/tick N1
service target also needs a measured peak, not only an average.

| Across a normal three-tick window | Required publication |
| --- | --- |
| Existing entity updates twice | One current full state at the release boundary; no old W1 snapshot held |
| Spawn, update, despawn on distinct ticks | Retain exact tick-1 spawn and tick-3 despawn units in order; drop tick-2 update only if replaceable and independent |
| Spawn and despawn within one authority tick | Publish only the committed W1 lifecycle result plus any independently meaningful event; never fabricate an intermediate visible state |
| Sequence 7 pending, sequence 8 resolves | Carry result 8 individually with controller state; watermark stays at 6 until 7 resolves |
| Event names a transient entity | Event-tick mandatory unit with dependency state, or a self-contained event schema; never a later dangling reference |
| Slow peer misses publications | N1-owned unsent state may coalesce; mandatory journal persists until admission ACK or explicit resync/close |

## Resource and critical-path account

For `P` peers, `G` dependency groups per peer, `C` changed records in one
tick and `R/E` results/events, each 60 Hz extraction inspects relevant
changed records and mandatory logs, at least `O(P(C+R+E))` before interest
lookup and dependency closure. A 20 Hz release scans or moves the latest
complete state for each admitted group plus its mandatory units. Four peers
× 128 actors × 56 encoded bytes × 20 Hz is 573,440 ordinary state-record
bytes/s before framing; this excludes additional mandatory groups. A
worst-case three full units for each of four groups every release would
triple the full-state portion and fail the current 256 KiB/s per-peer
profile. That case needs recipe refusal or a separately admitted profile,
not silent state/event loss. Actual record allocations, per-tick CPU peaks
and RSS remain unmeasured.

N1 must reserve the selected 128 KiB unsent state, 64 KiB reliable control,
256 KiB exact sent-frame ledger, 256 KiB sync closure, 256-result and
128-event retention, per-peer transport
headroom and shared serialization capacity under the existing 2 MiB/peer
and 16 MiB/four-peer planning caps. Queue limits constrain retained N1 bytes,
not allocator metadata, socket/TLS buffers or browser internals. The pending
result/event/lifecycle producer must be able to finish after an accepted
input even if another peer stalls: admission credit or a documented explicit
close path is needed before sequence consumption. No preallocated network
pool may borrow the authority's required execution reserve.

## First production storage candidate

The first no-socket owner should be admitted per peer, only when N1 is on.
This is a **candidate layout**, not a selected runtime or a measured cost:

- Reserve sixteen complete **6,944-byte R4 payload** slots (111,104 bytes)
  plus a small ordered index/free-slot table at peer admission; the complete
  64-byte envelope is added only at final submission. The 128 KiB unsent
  profile leaves 19,968 bytes for slot metadata and exact journal references,
  but the implementation must measure their actual layout/alignment and
  charge every retained allocation within the per-peer reserve. Each slot owns its encoded
  scalar bytes, partition ID, generation, tick, completeness/mandatory tags
  and exact journal-ID references. A replaceable candidate may overwrite a
  same-incarnation slot only after proving complete state and no intervening
  mandatory unit; an attempted 17th live slot closes or resynchronizes the
  peer before an application sequence is assigned. Control's 64 KiB is a
  separate reserve. This avoids per-tick `Vec` growth and moving payload slots
  merely to reorder a three-tick batch.
- Keep the baseline-bound partition table and entity assignment in compact
  sorted rows, plus explicit per-partition admitted sample tick/generation on
  the client. The first profile bounds 128 visible entities and **256 distinct
  assigned wire IDs per baseline incarnation**, counting departed IDs until
  atomic replacement. Before a 257th assignment, recapture a baseline or
  close that peer; do not let churn grow an invisible ownership map. The
  256-row table makes bounded binary lookup or sorted merge plausible;
  compare it with a direct-index/hash alternative
  under the actual four-peer change pattern before selecting. An entity that
  leaves visibility retains its partition assignment until atomic baseline
  replacement. The [composite replica oracle](../../evidence/integration/N1-MULTI-GROUP-REPLICA-ORACLE-2026-11-03.md)
  proves this semantic need using clone-backed trees; it is not this layout.
  The [isolated four-peer lookup probe](../../evidence/integration/N1-MEMBERSHIP-LOOKUP-2026-11-03.md)
  found sorted merge faster at 32/128 changed IDs and binary lookup faster
  at eight IDs; one middle-ID binary/hash lookup is similar. It does not price
  sorting, assignment maintenance or the whole callback, so the production
  sparse/dense dispatch and storage choice remain open.
- On final submission, reserve the exact framed bytes and ACK metadata in
  the 256 KiB/128-entry sent ledger **before** assigning the envelope
  sequence or freeing the unsent slot. Ordinary entries use the 160 KiB/112
  sublimit; baseline framing has the independent 96 KiB/16 reserve. A
  contiguous ordinary frame arena may suit cumulative ACK, while two
  32 KiB baseline chunk slots permit out-of-order staging ACK without arena
  compaction. Two outstanding chunk frames occupy 65,688 bytes including
  their 76-byte per-frame overhead, plus Begin/End and sequence tombstones.
  The [credit oracle](../../evidence/integration/N1-BASELINE-ACK-LEDGER-ORACLE-2026-11-03.md)
  proves counts and disposition logic only; a concrete allocator must show
  that released holes and partial writes never require more than the charged
  bytes. The immutable 256 KiB baseline image, 256 KiB sync closure,
  transport buffer and mandatory journals remain simultaneous owners.
- The 60 Hz extraction path should walk W1's changed IDs and committed
  result/event lists once per peer, lookup selected partition membership,
  copy schema-declared scalars directly into owned bounded records, and
  release every W1 lease before any send/ACK work. At the 20 Hz boundary it
  samples full current state. No generic `T: Clone` or retained `Arc` proves
  ownership; the codec's fixed schema and byte-count checks must do so.
  `HeadlessHost::on_callback` currently returns its publication list **after**
  its internal clock/tick timer stops. The first integration fixture must
  time callback plus synchronous N1 capture as one whole update path and
  include that capture in F1's budget account. A later synchronous
  publication sink may bring capture inside the host callback; a host
  `observed_elapsed` reading alone cannot qualify N1 cost.

The first implementation gate is one fixed-schema no-socket slice with a
production partition consumer and owned journal. Measure one/two/four peers,
idle and maximum selected changed sets, a three-tick mandatory burst,
slow ACK and failed admission. Record encoded bytes, slot/ledger high-water,
allocation counts, callback tails and RSS while W1 snapshot lease count
returns to baseline. Compare the sorted-membership and contiguous-ledger
candidates before promoting this layout; do not preallocate per-peer network
storage for a networking-off standalone game.

## Minimum executable oracle and rejection gates

- Use one selected fixed-schema game with two and four peers, a three-tick
  release window, a full dependency group and a controlled actor. Compare each
  emitted group with an independently stepped authoritative state at its
  stated `commitTick`, then admit it to a replica. Exercise spawn/update/
  despawn on separate ticks and within one tick, interest exit/re-entry,
  dependent detach, delayed result gaps and event references. Include two
  independent groups at the same commit tick and different permitted sample
  ages; the current I1 replica must first gain or be replaced by a checked
  N1 admission path that represents both cases without a second authority.
- Exhaust every count/byte boundary (32 entities/lifecycle, 64 results/
  events, 8 KiB message, 128 KiB unsent state, 64 KiB reliable control,
  160 KiB/112-entry ordinary sent-frame sublimit, 96 KiB/16-entry baseline
  reserve, 256 results and 128 events). A rejected group must leave all prior peer
  state/ACK credits intact. An accepted input's result must either be
  delivered in a complete group/baseline or have an explicit closed-peer
  uncertain outcome; it cannot vanish behind state coalescing.
- Hold one peer without ACK through more than eight changed authority ticks
  while the others continue. Inspect live W1 lease count, old page retention,
  N1 queue/journal bytes, per-tick callback time and allocation/RSS tails.
  Inject slow socket writes, invalid ACK, baseline restart, disconnect and
  guard against old-epoch acknowledgement. The test-only inline probe is
  insufficient for this general path.
- Exercise two partitions at the same `commitTick`, state-only coalescing
  before envelope sequencing, a stale interest generation, an ACK for an
  unsent or unrelated group and a same-epoch replacement baseline. The
  replacement binds a new partition table, superseded submitted sequence
  interval and exact included outcome/event IDs without resetting the
  application sequence or releasing an unrepresented mandatory record.
- Reject StateGroup sequence S, then send a maximum eight-chunk 256 KiB
  replacement baseline while S blocks cumulative ACK. Two independent
  chunk credits must complete staging without exceeding the sent-frame
  ledger or forging S admission. A failed BaselineAck closes; a valid one
  supersedes S and retires only included mandatory identities.
- Measure the selected four-peer whole callback with ordinary 20 Hz state,
  three-unit mandatory bursts and maximum admitted result/event bursts. A
  sampled 0.5 ms/tick average cannot conceal an over-budget publication
  tick or unbounded accumulation. Keep native, browser and device evidence
  separate.

The first production slice should implement the schema-checked owned
projection/journal and a bounded no-socket consumer together with a replica
oracle. Only after that interaction is coherent should the selected WSS
adapter take ownership of already encoded N1 messages. This keeps socket
backpressure out of W1 and avoids inventing a second game-state authority.
