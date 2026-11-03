# N1 R4 baseline image — design candidate

3 November 2026. [NETWORK-SPEC](../NETWORK-SPEC.md) selects the Begin/Chunk/End
envelopes and the semantic content of an atomic baseline but does not yet
define the image's bytes. This candidate gives the first R4 profile a
deterministic image, exact producer and consumer obligations, and an
allocation bound. It is **not** a released protocol or an implemented codec;
promote its rules into NETWORK-SPEC after cross-owner review. C13 remains the
authority for atomic admission, stable identity and one committed world.

## Owners and capture barrier

The authority owns accepted input sequence and pending/resolved outcomes.
W1 owns committed state and the world tick. N1 owns per-peer interest,
assignment history, frozen sent/unsent mandatory output, baseline bytes and
post-cutoff sync output. A capture at tick `T` must obtain the authority's
peer `acceptedThrough = nextExpectedInputSequence - 1`, pending schedule
inventory, retained results/events and W1's immutable state from the **same
committed barrier**. The current core baseline group lacks the accepted-input
high-water and pending schedule inventory; a paired read-only producer is
required. Querying those facts after authority time advances would create a
false certificate. N1 copies the bounded projection before releasing the W1
publication; it does not retain a world lease while a client stages chunks.

At the barrier, N1 freezes all pre-cutoff sent and unsent mandatory IDs and
state/lifecycle units. It verifies the image against that exact union before
committing Begin. The old stores remain charged. A separate sync queue starts
from the image's new partition generations and owns every later publication.
It survives BaselineAck. The image itself is allocated at its **exact encoded
length**, not as a permanent 256 KiB peer buffer. The transport cap remains
256 KiB for future protocol-compatible game profiles.

## Image version 1, little endian

The image has one 192-byte fixed header followed by seven counted sections
in this order: partitions, assignments, accepted-but-unresolved inputs,
actors, baseline-presence records, results, events. No alignment padding,
trailing bytes or unknown fields are allowed. All counts and the total byte
length are checked with overflow-safe arithmetic before allocation or state
mutation. A changed layout changes the protocol hash.

| Header offset | Field | Bytes |
| ---: | --- | ---: |
| 0 | Magic `R4B1` | 4 |
| 4 | Format version `1`, header bytes `192` | 2 + 2 |
| 8 | Exact `totalBytes`, zero flags | 4 + 4 |
| 16 | Nonzero `baselineId` | 8 |
| 24 | MatchId | 16 |
| 40 | Nonzero connection epoch | 8 |
| 48 | Begin application sequence | 8 |
| 56 | Capture tick | 8 |
| 64 | `supersedesFrom`, `supersedesThrough` | 8 + 8 |
| 80 | Controlled input `resolvedThrough`, server `acceptedThrough` | 8 + 8 |
| 96 | Authority event cursor, controlled actor wire ID | 8 + 8 |
| 112 | Controlled partition group ID | 4 |
| 116 | Partition, actor, assignment, unresolved, result, event counts | 6 × 2 |
| 128 | Protocol hash, semantic hash | 32 + 32 |

`supersedesFrom..=supersedesThrough` names the retained pre-Begin ordinary
sequence interval; both fields are zero for an initial baseline or a
replacement without ordinary frames. Only StateGroup entries in the range
are superseded. Begin's baseline ID, hashes, capture tick, total bytes and
sequence must equal the image header. Begin's SHA-256 covers **these exact
image bytes**. The server first preflights the next sequence without
consuming it, encodes and hashes the image with that predicted Begin value,
then commits Begin as the final infallible transaction step. No other peer
submission may interleave under that owner.

| Section | Row width and bound | Canonical content |
| --- | --- | --- |
| Partitions | 16 bytes, 1–4 | `groupId:u32`, `interestGeneration:u32`, `actorCount:u16`, six zero reserved bytes; ascending unique group ID and nonzero generation. |
| Assignments | 20 bytes, ≤256 | `wireId:u64`, `groupId:u32`, full `visibilityGeneration:u32`, status `visible=1/left=2/retired=3`, three zero reserved bytes; ascending unique nonzero wire ID. |
| Accepted unresolved | 24 bytes, ≤256 | `inputSequence:u64`, `actionType:u32`, schedule kind `timed=1/nextTick=2`, zero flags/reserved, `dueTick:u64`; ascending sequence, due tick strictly after capture tick. |
| Actors | existing R4 56 bytes, ≤128 total and ≤32 per partition | Complete fixed actor records, grouped by ascending partition ID and then wire ID. |
| Baseline presence | 16 bytes, exactly one per actor | `wireId:u64`, full visibility generation `u32`, game schema ordinal `u16`, baseline-only `present=0:u8`, zero flags; same order as actors. It initializes presence without replaying a live spawn effect. |
| Results | existing R4 32 bytes, ≤256 | Full individually resolved records, ascending unique input sequence, including unresolved-gap higher IDs. |
| Events | existing R4 40 bytes, ≤128 | Full retained mandatory records, ascending unique match-scoped event ID. |

The absolute section-bound calculation is `192 + 4×16 + 256×20 +
256×24 + 128×56 + 128×16 + 256×32 + 128×40 = 34,048` bytes. This is a
conservative sum of independent maxima, not an admitted simultaneous game
workload. The fixed R4 grammar needs at most two 32 KiB chunks; its
event-free subset is at most **28,928 bytes and one chunk**. Two-chunk R4
parsing is structural-only until event-bearing admission exists. The image
must not gain arbitrary padding to exercise the eight-chunk transport limit.
A separate semantically validated larger-profile fixture must stress eight
chunks in the actual frame owner.

## Admission rules across owners

Every visible assignment has exactly one matching actor and presence row,
including group ID and full generation; no left/retired row has either.
Presence rows are baseline initialization, not authority creation events.
A replacement begins a fresh ≤256-row assignment incarnation. It may carry
selected left/retired rows; omitting old invisible rows reclaims capacity.
Within the new incarnation, live `Spawn` means first admission of a wire ID
to that incarnation, even if the match-scoped ID was seen before an atomic
replacement. Live `Reenter` requires a carried `left` row. Neither kind may
be treated as a gameplay creation effect; that requires an identified event.
The authority still never reuses a match-scoped wire ID. Reused group IDs
must advance generation exactly once; wrap requires a new connection.

`resolvedThrough ≤ acceptedThrough`; the gap between them fits the 256-input
window. Every server-accepted sequence above the prefix through
`acceptedThrough` appears exactly once as either a result or an accepted
unresolved row. Every result newly covered when the client's old resolved
prefix advances to the image prefix must have its full outcome in the image,
unless N1 has already recorded exact client admission for that identity.
The image may repeat an already admitted outcome idempotently. A scalar
watermark never substitutes for an unseen committed/rejected result. The
producer refuses a pending input whose due tick is at or before capture
tick. Every result's resolved tick is at most the capture tick. A newly
learned result for an intent that was unresolved at the client's last
admitted controller sample must resolve strictly after that sample tick;
an earlier or equal tick would contradict the previously confirmed state.

The client checks `acceptedThrough` against its highest locally allocated
input sequence and refuses regression below its previously admitted
same-epoch accepted high-water. It also refuses a regressing resolved prefix
or known-resolved higher ID, and a capture tick older than any replaced
partition's sample. It matches results and unresolved rows to retained same-epoch
intents, keeps its own next input sequence and every locally allocated but
server-unreceived intent above `acceptedThrough`, and resends missing input
batches under their **original** sequences. It never constructs a shorter
client sequence owner from the server inventory. A new connection epoch
instead discards old pending input under the separate reconnect policy.

The event cursor is the authority's highest allocated event ID at capture,
not proof that every lower ID reached this peer. The first completion path
requires exact inclusion of every still-mandatory pre-cutoff event; no
transient expiry is inferred from the scalar cursor. Immediate events named
by included results must be present with matching origin, range and commit
tick under the game validator. Until the client has a bounded event owner,
only an event-free selected game with cursor zero may complete BaselineAck;
event-bearing images can be structurally parsed but cannot be admitted.

The client builds a candidate replica, progress and event state from the
checked image, validates the game manifest/dependencies and compares local
pending intents before changing the confirmed state. Any late failure keeps
the old replica/progress/event state and emits no BaselineAck. The server
compares the successful ACK to the image's exact ID, hash-bound certificate,
resolved prefix, event cursor and included mandatory sets; an authority
callback error retains one exact receipt for retry. Only after that callback
succeeds may it retire the frozen old output and expose post-cutoff sync.

## Chunk and rejection gates

For `totalBytes > 0`, `chunkCount = ceil(totalBytes / 32,768)` and chunks are
numbered zero through `chunkCount−1`. Every chunk except the last has exactly
32,768 data bytes; the last has the remaining 1–32,768 bytes. The receiver
rejects a wrong count, index, length, duplicate with different bytes, hash,
reserved field or cross-wired Begin/image identity before baseline admission.
Staging ACK frees one physical chunk slot and keeps its sequence tombstone;
it does not retire any result/event or supersede an ordinary frame.

Proof must cover malformed counts/order/lengths and exact image bytes;
missing assignment/presence rows; stale generations; prefix 6 with pending
7, resolved 8 and locally allocated but server-unreceived 9; omission of
newly covered results 1–6; a forged accepted high-water beyond the client's
last allocation or a rollback below the last admitted accepted high-water;
future-dated results, newly learned results no later than the prior controller
sample, a regressing higher result or old partition sample; a missing
mandatory event below the cursor; failed authority ACK retry; a caught
unwind; and post-cutoff output surviving successful BaselineAck. A physical
eight-chunk transfer, structural two-chunk R4 parsing and a semantic
event-free one-chunk R4 baseline are distinct
tests. The whole N1 production gate remains open until producer, codec,
consumer and peer owner run together with real W1 output and measured cost.
