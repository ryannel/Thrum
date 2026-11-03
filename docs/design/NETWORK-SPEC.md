# N1 initial protocol specification

1 November 2026. Selected design for ordinary small-session multiplayer; unimplemented and unmeasured. This
specializes [N1](modules/NETWORKING.md) and C13 rather than adding another networking framework. The first local
game still needs no transport. A different network genre can replace this protocol through the same authority and
content boundaries after its own design; these limits describe the first profile, not a permanent engine ceiling.
The [provisional outbound cadence study](research/N1-OUTBOUND-CADENCE-2026-11-03.md)
records an open construction decision: 60 Hz authority groups cannot be
collapsed into a 20 Hz latest-state sample when lifecycle or mandatory
outcomes occur, and the current I1 replica admits only one group per tick.
Its revised burst and multi-group admission proof must precede I2n runtime
promotion.

The [fresh review](SIMPLIFICATION-REVIEW.md) identified SR01–SR03/SR07. The bounded outcome, atomic group,
compatibility and analytical R4 recipes below close those design gaps for the first I2n construction slice. Runtime,
transport, device and latency qualification remain unrun. Full groups are selected for this profile; another genre
may qualify a different replication recipe without changing authority or content identity.

## Workload and transport selection

The reference profile is four players, at most 128 replicated entities per peer, 60 authoritative ticks/s and
20 state publications/s. Select WebSocket binary messages over TLS for the initial cross-platform transport.
Native hosts/clients use `tokio-tungstenite` 0.30.0 with rustls; the browser uses its WebSocket API through a
web-sys adapter. Pin resolved dependencies and TLS provider/features in the SDK lock before building. Tokio owns
network I/O only, on a bounded adapter runtime; it does not schedule simulation or mutate W1. Native messages cross
bounded channels into F1. Browser delivery queues into the same admission interface. No per-message task spawning. Configure native accept/connect before receiving application data:
`max_message_size=Some(65536)`, `max_frame_size=Some(65536)`, read buffer 8 KiB, write buffer 8 KiB,
`max_write_buffer_size=262144`; disable compression. Parser, buffer and TLS allocations are accounted in admission,
not left at library defaults. Application queue overflow cannot be repaired by an unbounded library queue.

This is a direct-connect cooperative/board/exploration baseline, not a competitive-shooter latency promise. Reliable
transport has head-of-line blocking. Native UDP/QUIC or browser WebTransport/WebRTC are not required by this profile;
adding a datagram adapter requires qualification of its real delivery semantics. WSS is required outside loopback
development. Production uses platform/maintained TLS validation, never a custom cryptographic handshake.

Initial session access uses a server-issued random 256-bit, single-use invitation token, associated with a player and
role, expiring after five minutes. Send it in the first encrypted application message, never the URL or logs. An
approved auth adapter can substitute identity credentials. Token consumption and player assignment are atomic under
N1 before game commands are enabled; reconnect obtains a new invitation through the chosen access mechanism.
No account service or automatic token distribution is implemented or required for offline/local play. The server
validates allowed browser origins where applicable; origin is not authentication. Rate limits apply before auth.
For a direct-connect game, the host starts a pinned admitted server build and creates an invitation through its
access UI/CLI. The player receives the endpoint and token through an out-of-band trusted channel, enters them in
the client's join UI, and sees loading until verified content and baseline admission complete. The token is treated
as a secret clipboard/QR/import value, not placed in an endpoint URL, browser history or telemetry. A game may
replace this invitation UX with an approved account/lobby adapter without changing N1's command or state semantics.

## Exact envelope and codec

One application message equals one binary WebSocket message. Little-endian integers; no implicit host alignment,
native pointers, platform usize, NaN, infinity or unspecified serializer format. The fixed 64-byte header is:

| Byte offset | Field |
| --- | --- |
| 0 | Four ASCII bytes `ENG1` |
| 4 | Protocol major u16, initially 1 |
| 6 | Message kind u8 |
| 7 | Flags u8, initially zero; unknown required flags reject |
| 8 | Payload length u32, excluding this header |
| 12 | Reserved u32, zero |
| 16 | MatchId, 16 bytes |
| 32 | Connection epoch u64 |
| 40 | Message sequence u64, increasing per sender/connection |
| 48 | Associated server tick u64; zero for initial Hello |
| 56 | Acknowledged application sequence u64; zero means none |

Total message size is at most 64 KiB. Validate header, state, length and rate before allocating decoded arrays.
Payloads are schema-generated fixed records or length-prefixed bounded arrays; length/count multiplication uses checked
arithmetic. UTF-8 strings are capped at 256 bytes unless a specific kind allows less; user chat is outside this first
protocol. Entity references are u64 wire IDs scoped by MatchId. Stable type/field IDs are F3 identifiers resolved by
the exact schema manifest, never Rust TypeId. Integers carry declared units; float fields are finite IEEE f32/f64 only
where explicitly declared. Initial messages do not use compression, avoiding decompression expansion and hidden CPU.

Kinds 1–12 are Hello, Welcome, Reject, ContentReady, BaselineBegin, BaselineChunk, BaselineEnd, BaselineAck,
InputBatch, StateGroup, Clock and Close. Reserved kinds fail this protocol version. Hello's MatchId, epoch and sequence
are zero. It declares `role` (`player` or `spectator`), `protocolHash`,
`semanticManifestHash`, exact required consumer capability IDs, invitation token and client build provenance.
Welcome assigns MatchId, a nonzero connection epoch allocated monotonically and uniquely **match-wide**, player
identity, zero or more controlled actor wire IDs,
server tick/clock sample, baseline/content-ready deadlines and the selected profile limits. A spectator has no
controlled actor and cannot send InputBatch. Content-ready is a bounded negotiation disposition after verified
download; ContentReady carries the verified required-content manifest hash and precedes baseline capture. It is not
a claim of visual GPU readiness. Everything after Welcome must
match the assigned identities. Epoch reuse is forbidden; exhaustion starts a new MatchId. This makes
`(connectionEpoch,inputSequence,kind,ordinal)` a match-unique speculative origin. No game commands appear before
BaselineAck and active admission.

The **protocol hash** covers this envelope and the exact wire action, result, event, group and baseline schemas; it
must match for this release. The **gameplay semantic-manifest hash** covers registered rules and action meanings,
authority/tick policy, required gameplay component and query meanings, physical model/domain/edge/source semantics,
and the common accepted role-specific product/projection mapping, not a particular client's visual build. It does
not hash all local F3 schemas or a whole client build. The pinned server build and physical product hashes are
provenance; a nonpredicting replica may consume the
declared confirmed projection without possessing the server solver. A locally evaluating client must prove its exact
query-product hashes map to the same semantics. Cook/admission validates that relation, never guesses similarity.
Purely visual shaders/materials and tool-only schemas may differ; changing a rule, action meaning, required collider,
physical query or its mapping changes the semantic identity and rejects an old peer. Unknown required fields/kinds
reject; there is no arbitrary extension bag or third `game hash`. A wire schema change changes the protocol hash and
starts a new match; no partial rolling upgrade is promised.

## Commands and ownership

InputBatch begins with `count:u16` and at most eight records, each
`{inputSequence:u64, actor:u64, tickHint:u64, actionType:u32,
payloadBytes:u16, payload}` with at most 256 payload bytes. ActionType indexes the negotiated manifest table mapping u32 ordinals to stable F3 action IDs; ordinal meaning never changes within a match.
Each declaration specifies held versus one-shot, `timed` versus `nextTick` scheduling, fields/units/ranges, actor
permission and maximum frequency. For `timed`, tickHint is the intended execution tick; for `nextTick`, it is the
latest authority tick observed by the sender and only bounds command age. A gameplay precondition revision belongs
in the typed payload where required.
N1 checks the envelope/ownership; P6 checks the actual game action against authoritative state.

Admit at most 120 records/s/peer with a burst of 16, at most two held-control updates per actor/tick and at most
eight queued one-shots per actor. Input sequences start at one, increase contiguously per connection and never wrap;
a gap, reuse with different bytes or exhaustion requires resynchronization. Keep a 256-result deduplication window
and a 32-tick future-input ring. A duplicate in the window receives its original disposition without execution;
below-window retries return expired and require resynchronization. For `timed` actions, accept a tickHint from server
tick minus 6 through plus 12. Late held input changes control from the next tick only; a timed one-shot more than
two ticks late rejects. Ahead-of-window input rejects rather than expanding the ring. `nextTick` board/inventory
actions with an observed tick older than 18 ticks or more than two ticks in the future reject; otherwise they execute
at the next authority tick in input-sequence order, with P6 checking current state and declared target/inventory
revisions then. No server rewind or lag compensation is selected. Missing held controls expire after six ticks and
become neutral/cancel; disconnect clears them. P6 cannot reinterpret a one-shot as repeating held input.

### Committed action results

An envelope acknowledgement only says that an application message was dispositioned; it never means a game action
committed. Every syntactically valid sequenced InputBatch record receives one authority-owned `committed` or `rejected`
result after P6's tick/barrier decision. A 32-byte result record contains `inputSequence:u64, resolvedTick:u64,
firstEventId:u64, actionType:u32, reason:u16, eventCount:u8, disposition:u8`; a committed result has reason zero,
a rejection has no event IDs, and no immediate event uses zero as `firstEventId`. P6 allocates any immediate event IDs
for that action contiguously within MatchId; later causally related events carry the same origin without changing the
already resolved result. Admission may queue a future action but emits no successful result before commit. Protocol,
epoch or authorization failure that prevents identifying a valid sender/sequence closes or rejects the connection,
not a fabricated game result.

The controlled-actor StateGroup contains these results and its **resolvedThrough** input-sequence watermark beside
the authoritative controller state at the group's `commitTick`. The watermark advances only across contiguous
resolved sequences, never over a pending action; result records may also name individually resolved higher sequences.
For example, future timed input 7 may remain pending while nextTick input 8 commits now: watermark stays at 6, and
result 8 travels with the resulting state. The client atomically admits the group, checks every named result and
the contiguous prefix, reconciles its controller to that exact tick/state, removes **all individually resolved**
speculative inputs/effects represented in that state, then replays only unresolved inputs scheduled after commitTick.
The server resolves every input due by commitTick before publishing that controller state; an unresolved due input
blocks publication or forces explicit resync. A result's `resolvedTick` may precede a 20 Hz group's `commitTick`;
the group carries the later exact controller state that includes its disposition. The controlled-actor group carries
all newly resolved results represented by that state, up to 64; it never publishes a partial result/state closure.
If the bounded reliable queue cannot
retain all results, refuse new commands before sequencing and resynchronize or close instead of
advancing an unsupported watermark. StateGroup acknowledgement releases retained results; retain at most 256
unacknowledged results (8 KiB) per peer. On same-epoch resynchronization the new baseline includes outstanding
results individually represented in its captured controller state, including IDs above a pending gap. Reconnect
uses a new epoch and baseline, discards client pending inputs from the old
epoch and never automatically retries an uncertain one-shot. A game requiring retryable durable work supplies its
own stable operation ID and P7 result lookup. A crash-durable outcome must be committed with its authoritative
save/recovery root before the game promises retry after restart; N1 makes no cross-reconnect exactly-once promise.

Clock messages exchange echoed monotonic timestamps once per second. A client with no outgoing input sends an
ack-bearing Clock within 100 ms of admitting StateGroups, so acknowledgements do not wait for that regular
clock. Receipt of Clock alone does not trigger an immediate acknowledgement loop. Header acknowledgement covers the
highest contiguous application sequence dispositioned, including staged baseline chunks; BaselineAck separately
confirms atomic replica admission. GPU readiness is never required to acknowledge admitted replica state. Keep eight RTT samples, estimate offset from
the minimum-RTT sample and slew the client's tick estimate by at most one tick/s after initial sync. Server tick
and monotonic timestamps stay separate. Initial timed-control lead is three ticks; a game may choose a declared larger
lead within the admission window. This reduces ordinary late arrivals; it does not promise sub-RTT response or
correct asymmetric network-delay estimation. Excess jitter leads to late-action rejection or resync, not silent dt changes.

A mobile app entering background stops input and application acknowledgements, sends Close when the platform allows,
and treats any suspended socket as lost; the server clears held controls and keeps advancing for other peers. Resume
uses a newly authorized invitation, connection epoch and fresh baseline, with old-epoch pending intents discarded and
uncertain one-shots shown as uncertain unless the game offers a durable operation lookup. No background catch-up,
OS socket survival or replay of suspended input is assumed. The transport fixture must cover suspend without Close,
late old-epoch packets, resume after server save reload/new MatchId and expired invitation.

## Baselines, groups and interest

Server wire IDs monotonically increase within MatchId; reconnect rebuilds client-local maps without changing live
server IDs. A server save reload starts a new match. Visibility generations increment on re-entry; overflow starts a
new baseline/connection. **StateGroup is the only live state/lifecycle publication kind.** Its 32-byte fixed payload
header contains `groupId:u32, interestGeneration:u32, commitTick:u64, resolvedThrough:u64,
entityCount:u16, lifecycleCount:u16, outcomeCount:u16, eventCount:u16`, followed by schema-declared full entity
records, ordered 16-byte spawn/despawn/leave/re-enter records, 32-byte action results and bounded typed events.
The first-profile complete message is at most 8 KiB and at most 32 entity records, 32 lifecycle records, 64 results
and 64 events. Each lifecycle record is `wireId:u64, visibilityGeneration:u32, schemaId:u16, kind:u8, flags:u8`.
Per-game event payloads and the exact entity schema are declared in the protocol manifest. Only the controlled-actor
group carries a nonzero resolvedThrough and its matching complete controller state; other groups use zero.

One group atomically admits lifecycle changes **with** required first or dependent state at the same committed tick:
spawn/re-entry plus pose, ownership and required attachments; detach/despawn plus dependent relationship changes.
The client validates all IDs, generations, schemas, dependencies, event origins and bounded counts, then publishes
the whole group at one replica barrier or rejects all of it. Interest re-entry uses a new visibility generation and
full state. A group can pack independent entities for wire efficiency, but every dependency closure must fit in that
one group; a recipe that cannot do so fails profile admission. Independent groups may have different sample ages
only where their consumers explicitly tolerate it. No intermediate actor missing a required state is observable.

During negotiation, fetch and verify the client's required gameplay content/query closure before capturing a baseline.
This content-loading phase has a 30-second deadline, with no server snapshot history pinned for that peer. Only after
it signals content-ready does the server take a fresh baseline. If required content becomes unavailable during sync,
release the baseline/history, return to bounded content-loading, then recapture; never repeatedly retain a stale
baseline throughout a long download. Initial server matches preload their entire finite authoritative region/physical
closure before starting. They do not activate source changes or stream new authoritative regions during a match;
client-local visual/query residency can change without pausing authority. A future streamed-server profile needs its
own bounded staged commit mechanism while authoritative ticks continue; this static preload is a first-profile
admission choice, not a permanent engine restriction.

BaselineBegin specifies baseline ID, exact protocol/semantic manifest, capture tick, total bytes and SHA-256; maximum
256 KiB in eight chunks of at most 32 KiB. Stage all chunks in a reserved buffer, validate hash/schema/dependencies,
then admit once at a replica barrier and acknowledge. Its staged image may exceed an 8 KiB live StateGroup but is
published atomically only after all chunks validate. The baseline includes complete lifecycle/state groups for the
interest set, the controlled actor's committed state, its same-epoch contiguous watermark and **all outstanding
individually resolved result IDs represented in that state**, including IDs above a pending gap, plus the admitted
event cursor.
The server retains any still-pending future input in the same epoch; the client keeps its matching unresolved local
intent for later reconciliation. During sync retain at most two seconds/256 KiB of subsequent publications: coalesce replaceable full
state per group while preserving ordered lifecycle, outcomes and mandatory events. If that closure cannot be
reconstructed, abandon the baseline and capture a newer one rather than presenting a partial state. Permit two
restart attempts in ten seconds, then disconnect with a cause. A peer needing unavailable required query/collision
data stays loading up to 30 seconds;
the authority does not wait. Visual pipeline readiness remains independent.

The baseline wire framing is exact: Begin payload is `baselineId:u64, captureTick:u64, totalBytes:u32,
chunkCount:u16, reserved:u16, protocolHash:32 bytes, semanticHash:32 bytes, payloadSha256:32 bytes`
(120 bytes); Chunk payload is `baselineId:u64, chunkIndex:u32, data` (12 bytes before data); End payload is
`baselineId:u64` (8 bytes). With three 64-byte envelopes, a one-chunk baseline adds 332 bytes. BaselineAck payload
is `baselineId:u64, resolvedThrough:u64, eventCursor:u64` (24 bytes); the baseline ID binds its exact included
result-ID set, including resolved IDs above the contiguous watermark.

The first profile sends full groups, with no delta baseline or group-history machinery. Header acknowledgements for
BaselineChunk confirm validated staging only; BaselineAck names its baseline ID, resolved watermark and event cursor
and confirms atomic replica admission. The baseline ID identifies its exact included result-ID set, including any
resolved IDs above the watermark; BaselineAck releases exactly those results/events admitted by that baseline;
chunk/header acknowledgement alone never releases them. A header acknowledgement for StateGroup confirms committed
group admission, including lifecycle, results and events, not mere receipt. An invalid group triggers Reject/resync
without acknowledging that group as admitted. Unsent
replaceable **state-only** groups may coalesce before entering transport; lifecycle, results and events remain reliable
until acknowledged or included in a new complete baseline. Bytes already sent on WebSocket cannot be withdrawn.
The single ordered stream and atomic group prevent a spawn from preceding required first state. Control has queue
priority before submission, without a claim it can overtake already transmitted state.

Game events carry a MatchId-scoped event ID, commit tick, schema and optional origin
`(connectionEpoch,inputSequence,kind,ordinal)`. The client deduplicates `(MatchId,eventId)` across group replay and
uses the origin to reconcile a permitted speculative cosmetic effect. A1/V5 maintain their own presented/expired
cursors; an event acknowledgement means replica admission, never that sound was heard or particles displayed.
The R4 fixed event record is `eventId:u64, commitTick:u64, originEpoch:u64, originInputSequence:u64,
kind:u32, ordinal:u16, flags:u16` (40 bytes); absent origin uses zero epoch/sequence. Other games declare their
event payload schema and redo the size admission.
Reserve at most 128 unacknowledged 40-byte R4 events per peer and include outstanding mandatory results/events in a
same-epoch replacement baseline. On cap or retention-age pressure, stop commands and resynchronize; if a required
outcome still cannot be carried, close with an explicit cause. The baseline records its committed event cursor so
older transient cues can expire without replay. Irreversible effects and durable operations execute on authority;
the game's P7 operation ID/outcome policy covers any result that must survive a fresh connection.

Command refusal under pressure occurs before consuming a new input sequence (backpressure) or by closing the peer;
every already accepted, parseable sequenced record still receives its committed/rejected result. If the connection
closes before that result is observed, the client treats it as uncertain under the reconnect policy above.

Initial interest is an explicit game-selected set within the 128-entity cap. A later spatial policy can compute this
set but must close required dependency groups and permissions. Leaving interest does not delete server state.
Never send a secret and rely on client fog pixels to hide it. Missing water query dependencies mean unavailable,
not zero water; physical solver residency is independent of interest.

## Projection precision and correction

The reference actor projection uses a 512 m grid origin: signed i32 sector coordinates plus signed i32 millimetre
offsets, with canonical offsets in `[0,512000)`. Position rounding error is at most 0.5 mm per axis. The effective
world limit is the smaller F3 transform envelope, not the full integer representable range. Velocity uses signed
i16 centimetres/s (range ±327.67 m/s); values outside the declared game envelope reject or use another named schema,
never saturate silently. Orientation uses four normalized quaternion components quantized to signed i16 and
renormalized; reject near-zero decoded norm. Bound accepted angular error to 0.02 degrees in conformance fixtures.
Gameplay health/currency/ownership use integer/enumeration fields without lossy float quantization.
For quaternion rounding, each component error is at most δ=1/(2·32767); vector error is ≤2δ. Renormalization and
quaternion double-angle conversion give a conservative small-error angular bound below 0.01 degrees, inside the
0.02-degree admission target. The octahedral decoder's unnormalized vector has length ≥1/√3 and component-error
norm ≤√6δ; normalization gives angular error below 0.01 degrees, also inside 0.02. Conformance still checks the actual
codec, sign/fold boundaries and malformed inputs; those checks verify these specified bounds rather than discover them.

The optional initial water projection is a 96-byte record per floating body, at most four records/group; it is a confirmed
sample for that body, not an arbitrary client-accessible copy of the shallow solver. Fields, packed sequentially with
no padding, are `body:u64, domain:u64, domainGeneration:u32, sourceRevisionIndex:u32, edgeRevisionIndex:u32,
supportFlags:u32, sampleTick:u64, sectorXYZ:3×i32, sampleOffsetMm:3×i32, stageOffsetMm:i32, depthMm:u32,
normalOct:2×i16, velocityCmPerSec:3×i16, heightErrorMm:u16, velocityErrorCmPerSec:u16,
normalErrorMilliDeg:u16, supportRadiusCm:u16, maxAgeTicks:u16, reserved:u32`. Domain wire IDs and revision indexes
map to the pinned semantic manifest; an unknown mapping rejects the whole group. Stage is relative to the sector's
Y origin. For a normal `n`, divide by `|nx|+|ny|+|nz|`, take xy, and when z<0 fold xy to
`(1-|y|,1-|x|) * signNotZero(x,y)`; round each to signed i16 using scale 32767. Decode x/y by 32767,
set z=`1-|x|-|y|`, unfold x/y by the same sign rule when z<0, and normalize the resulting vector.
`signNotZero(0)=+1`; zero/nonfinite input rejects. This is the fixed octahedral normal codec for schema 1. Unavailable,
dry and valid samples are distinct supportFlags; unavailable values must not become zero-water queries.

Stage/depth quantization contributes ≤0.5 mm each, flow ≤0.5 cm/s per component and normal ≤0.02 degrees (to be
verified by codec conformance). Reported error fields include quantization, provider error and sample-age uncertainty,
rounded upward, with at most six-tick query age relative to the consumer’s requested evaluation tick.
A water StateGroup includes each body pose at `sampleTick` with its matching query record, domain/revision and
support status in the same atomic admission unit; a previously retained exact pose may instead be referenced by
identity and tick, and the whole group rejects if it is absent. A delayed presentation can use that matched historical
pair; it is not a valid current-time gameplay query. Missing correspondence rejects interpolation rather than mixing
time samples. A schema overflow or error exceeding the consumer tolerance yields unavailable,
never saturated false precision. NormalErrorMilliDeg uses millidegrees, not radians. SupportRadius describes the
provider-validated query footprint; clients cannot extrapolate a point to a whole river. The four-record state payload
is 384 bytes, comfortably below the atomic group limit. Local boat forces and volume flux remain server-owned.

Other interaction queries travel as typed actor requests and authority results; initial clients do not receive or
predict the entire 128×64 shallow grid. Visual water can use a separate admitted approximation, constrained at the
confirmed contacts/shore by declared cue/error bounds. Games requiring locally queryable flooding throughout that
field need a new declared projection/profile, not silent partial-grid replication. No generic compression may change
physical semantics.

Remote presentation interpolates with a 100 ms buffer. At most 100 ms of declared extrapolation is permitted; then
hold a stale pose and show network status. At 500 ms without usable state, suspend local predicted interaction;
after five seconds, disconnect. A missing heartbeat cannot stall server tick publication.

Initial optional prediction is **only an isolated kinematic capsule against static collision**. No dynamic body,
moving platform, water, damage/inventory effect or other predicted actor participates. Capture position, velocity,
ground/contact mode, controller state, input sequence, tick and immutable collision revision. Keep 64 ticks, at most
256 KiB, and replay at most eight ticks or 1 ms CPU per presented frame, whichever comes first. Partial replay remains
unpublished; present confirmed state until the branch catches up. If it cannot catch up within four frames or any
dependency differs, discard it and resync. This is a correctness-bounded initial provider, not full physics rollback.

Corrections below 2 mm and 0.05 degrees can retain presentation correspondence; larger changes reset affected
geometric/effect histories, with optional 100 ms visual smoothing only where it does not misrepresent collision.
Replayed cosmetic effects use `(connectionEpoch,inputSequence,kind,ordinal)` keys. Server confirmation echoes the
origin key; no repeated audio/particles. Authority alone performs irreversible state or external operations.

## R4 encoded admission fixture

Byte/rate figures here count the encoded application envelope and payload. WebSocket/TLS/TCP/IP framing and
retransmission add physical-link traffic and latency; qualification reports those separately. The application token
bucket is not a claim that these figures are the complete network-link cost.

This is an **analytical recipe**, not a measured throughput or a universal game schema. R4 has one controlled actor
and 128 interested entities per peer, packed as four full StateGroups of 32 entities at every 20 Hz publication.
Its 48-byte base actor record is `wireId:u64, sectorXYZ:3×i32, offsetXYZ:3×i32,
quaternion:4×i16, velocityXYZ:3×i16, mode:u8, flags:u8`; `health`, `inventoryRevision`, `owner` and
`visibilityGeneration` add four u16 fields for 56 bytes total. A revision/generation must never alias after wrap;
the match or visibility instance resets before exhaustion. The fixture uses a fixed 16-byte
lifecycle entry, the 32-byte result record above and a 40-byte P6 event record; its declared event schema includes
event ID, origin, commit tick and effect kind. Games needing larger or different records must redo this admission
calculation. No E1 water group or prediction is required for this first R4 fixture.

| R4 wire item per peer | Calculation | Encoded bytes |
| --- | ---: | ---: |
| Full state publication | `4 × (64 envelope + 32 group header + 32 × 56 actor)` | 7,552 |
| Steady state at 20 Hz | `7,552 × 20` | 151,040/s |
| Maximum action results at admitted rate | `120 × 32` | 3,840/s |
| Declared event cadence | `32 × 40` | 1,280/s |
| Server clock | `64 + 24`, once/s | 88/s |
| **Ordinary-cadence server → peer model** | Sum above | **156,248/s = 152.6 KiB/s** |
| Full join baseline payload | `128 × 56 actor + 128 × 16 lifecycle + 4 × 32 descriptors + 256 manifest/cursors` | 9,600 |
| Full join baseline on wire | Payload + Begin/Chunk/End framing, `332` | 9,932 |
| Maximum same-epoch replacement baseline | `9,932 + 256 × 32 results + 128 × 40 events` | 23,244 |

Both baseline sizes fit one 32 KiB chunk. The ordinary-cadence model spends
156,248 B/s against 262,144 B/s **if** results/events can share those four
20 Hz full groups. It excludes extra group headers and repeated full state
when mandatory per-tick lifecycle/result/event units must remain ordered.
The [outbound cadence study](research/N1-OUTBOUND-CADENCE-2026-11-03.md)
keeps the true worst admitted burst, per-tick CPU peak and same-tick
multi-group replica consumer open; the table is not an R4 bandwidth pass.
During resynchronization it suppresses replaceable 20 Hz state publications and coalesces the latest complete
groups; mandatory outcomes/events and lifecycle remain ordered. Under the same ordinary-cadence packing assumption,
a quarter-second resync example sends
two **maximum** replacement baselines (46,488 bytes), one four-group full state publication (7,552),
128 lifecycle entries (2,048), 46 results (1,472), 64 events (2,560) and one Clock (88): **60,208 bytes**,
below the 65,536-byte quarter-second token bucket. This example deliberately
counts some results/events both in baselines and as new traffic, but does not
include extra mandatory group framing/full state; it is not the selected
worst-case burst. The peer resumes its normal
cadence only after BaselineAck; a later content/baseline overrun paces chunks, abandons and recaptures, or disconnects
under the existing deadlines. This fixture also fits each StateGroup's 8 KiB limit: one group with 32 actor,
32 lifecycle, 64 result and 64 event records is 7,008 bytes including its envelope. It reserves no delta bases.
The two-second sync retention uses latest full state per group plus ordered lifecycle/results/events; naively saving
all 40 publications would exceed 256 KiB and is therefore not the selected algorithm. An admitted game's different
event burst, required dependency size, baseline or cadence must fit these simultaneous limits or receive a different
profile; it cannot silently drop mandatory results/events. R4 declares held-control payloads ≤8 bytes at up to
60/s, timed one-shots ≤16 bytes at up to 30/s and nextTick interactions ≤32 bytes at up to 30/s. Conservatively
charging all 120 records at the largest 32-byte payload, each record is 62 bytes (30 fixed + 32 payload), and
each may be sent promptly in its own InputBatch with a two-byte count and 64-byte envelope. With one Clock,
`120×(62+66) + 88 = 15,448 B/s`, below the 32 KiB/s client cap. A quarter-second 30-record cadence plus
16-record burst costs at most `46×128 + 88 = 5,976 B`, below the 8 KiB client burst. Packing several records
reduces bytes but is not required for action latency or admission. The wire permits up to 256 payload bytes for other games, but that maximum
at 120/s fails this profile's simultaneous bandwidth admission and cannot be advertised as R4 capacity.

For timing, 60 Hz gives 16.67 ms/tick. Three-tick timed lead covers 50 ms; a declared seven-tick lead covers about
117 ms within the `+12` admission window. At symmetric 50 ms RTT with 0/30 ms jitter, timed controls use three
ticks; at 150 ms RTT with up to 30 ms additional one-way delay, seven ticks is the candidate. `nextTick` actions
execute at the next authority tick after arrival if their observed tick is within 18 ticks and P6 preconditions
still hold. Age includes both directions of delay and up to one 20 Hz publication interval before the client
observes a tick. A symmetric 300 ms RTT already consumes about 18 ticks round-trip, before that publication delay
or jitter; nextTick actions may reject as stale. With 100 ms jitter, timed controls can exceed the seven-tick lead
and two-tick late tolerance. That case is a **degraded connection**, with explicit rejects, stale presentation
or resync. These are acceptance envelopes to measure end-to-end, not an assertion of playability or low latency.

## Resource and overload limits

| Resource | Initial per-peer limit and response |
| --- | --- |
| Unauthenticated connection | Hello within 5 s; 4 KiB receive budget; at most eight unauthenticated sockets/server |
| Application receive queue | 64 messages / 256 KiB; reject/disconnect overflow, never allocate an unbounded decoded queue |
| Outbound unsent state | 128 KiB; coalesce superseded groups; oldest pending age >250 ms triggers full resync |
| Outbound reliable control | 64 KiB; failure to reserve closes peer, never silently drops lifecycle |
| Transport send buffer | Stop submitting at 128 KiB buffered; above 256 KiB or 1 s stalled, close/resync |
| State bandwidth | 256 KiB/s server→peer, 32 KiB/s peer→server; token bucket with one quarter-second burst |
| Server serialization/admission | 0.5 ms/tick and 16 MiB aggregate N1 working-memory cap for four peers, charged inside the whole-engine budget |

Reject a selected replication recipe whose mandatory reliable cadence cannot fit; do not drop required groups to
appear fast. For temporary overload, coalesce replaceable state and resync a slow peer. If server tick service cannot
recover within the F1 bounded backlog, end the session with an overload reason; no hidden slowdown or infinite catch-up.

Reserve up to 2 MiB of engine-controlled state/queues per admitted peer plus shared serialization/projection capacity
and transport/TLS headroom inside the 16 MiB aggregate cap. This includes 256 KiB current baseline staging,
256 KiB subsequent-sync closure, 256 KiB receive, 128 KiB unsent state, 64 KiB control, 256 KiB transport-send
capacity, 8 KiB results, 5 KiB events and bounded dedup/parser metadata. There is no 512 KiB delta-base reserve.
The remaining per-peer reserve and shared aggregate headroom must cover actual codec/TLS allocations, never an
uncharged library default.
Listen hosts charge local authority and client roles separately. Refuse a connection/recipe if reservation cannot fit.
All rate/count/byte limits apply simultaneously; they are not a promise that every maximum can be exercised together.

These are engine-controlled queue limits. Browser WebSocket has no application-controlled receive backpressure and
may allocate a complete message before our handler can reject it. Native libraries enforce message/frame limits at
their transport parser; browser memory/OS buffers require actual qualification. Cooperating endpoints additionally
use baseline acknowledgements/chunk credits. A malicious server can exceed browser-internal resource assumptions;
the design does not claim a hard whole-browser memory cap from a Rust queue limit. TLS does not make a peer trustworthy.

## Acceptance and references

Before code, the state trace covers mixed accepted/rejected actions, a later sequence resolving across a pending timed
gap, duplicate and expired inputs, uncertain one-shot reconnect, durable operation lookup, spawn/first-state and
detach/dependent-state atomicity, optional water pose/sample correspondence, visual-only compatible versus collider/rule
incompatible client builds, fresh join, dropped acknowledgement, partial baseline and same-epoch result-rich resync,
interest leave/re-enter, source edit during match, slow GPU, mobile suspend/resume and exhausted bounded retention
without changing authority.
Implementation conformance then uses a controlled in-memory transport and real WSS with 50/150/300 ms RTT, 0/30/100 ms
jitter and forced stalls/disconnects. Those conditions are test inputs, not performance already demonstrated. Check
whole-game input response, state age, memory, frame tails and event correctness, not throughput alone.

Primary sources inspected 1 November 2026: [tokio-tungstenite 0.30.0](https://docs.rs/tokio-tungstenite/0.30.0/tokio_tungstenite/)
provides the native WebSocket/TLS adapter seam; the [WHATWG WebSocket standard](https://websockets.spec.whatwg.org/)
defines browser message delivery and buffered sending. [Tungstenite configuration](https://docs.rs/tungstenite/0.30.0/tungstenite/protocol/struct.WebSocketConfig.html)
provides the native parser/buffer caps, which must be set explicitly. The envelope, limits and game protocol above are our choices,
not claims supplied by those libraries. No transport, invitation service or multiplayer game was built in this pass.
