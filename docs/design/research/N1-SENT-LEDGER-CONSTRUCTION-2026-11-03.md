# N1 sent-frame ledger construction gate — candidate

3 November 2026. The selected [network protocol](../NETWORK-SPEC.md)
separates an unsent R4 payload from an assigned application sequence. The
[finite unsent queue](../../evidence/integration/N1-UNSENT-QUEUE-2026-11-03.md)
now owns copied payloads and can transfer them only after a sink reserves
exact retained bytes and ACK metadata. The [baseline ACK oracle](../../evidence/integration/N1-BASELINE-ACK-LEDGER-ORACLE-2026-11-03.md)
proves disposition arithmetic in a tree model, not physical storage. This
note records a **candidate implementation**, not a qualified ledger.

## One sequence and two frame reserves

One per-connection ledger owns the next application sequence across ordinary
StateGroups and baseline Begin/Chunk/End. The ordinary arena has 160 KiB of
fixed byte storage and at most 112 entries. A retained StateGroup charges its
64-byte envelope, exact payload and 12 actual arena bytes reserved after the
message for frame accounting. Those 12 bytes are not transmitted; entry
metadata is measured and charged separately within the per-peer reserve.
The byte arena is circular and allows one frame to span its end; this avoids
uncharged wrap padding and a false contiguous-space guarantee. Entry metadata
stores exact sequence, frame start/length, current transport write offset,
admission/rejection state and journal ranges. The frame is immutable from
submission through admission ACK, valid baseline supersession or close.
Transport writes with a **known accepted-byte offset** may resume the same
frame at that offset. An uncertain WebSocket submission preserves the frame
and sequence, stops submission and closes/quarantines the peer unless the
adapter has a separately qualified recovery guarantee. It cannot be retried
under a new number.

The 96 KiB baseline reserve has two independent fixed chunk-frame slots of
32 KiB + 76 bytes each, plus Begin/End storage and up to 16 sequence entries.
Staging `BaselineChunkAck` frees only its exact chunk frame and credit while
retaining the sequence tombstone. Two chunk credits must remain usable even
when a rejected earlier ordinary StateGroup blocks cumulative header ACK.
The immutable 256 KiB baseline image and subsequent-sync closure are separate
owners and remain charged simultaneously with both frame reserves.

## Journal and ACK transaction

The first ordinary implementation can append exact result IDs and event IDs
to fixed 256/128-entry journals in submission order; each group entry names
its exact journal range. A repeated ID is refused before submission. Cumulative
header ACK retires only the contiguous, admitted entry prefix and its exact
identities. A rejected group stops ordinary ACK progression and initiates
replacement or close. The ledger retains an ACK receipt with exact IDs until
the authority/result consumer has observed it, so a failed callback does not
silently discard identities. The authority's result-history acknowledgement
still advances only its own contiguous prefix. After a receipt for result 8
is processed while result 7 remains pending, N1 must keep a bounded
**admitted-higher** identity record even after frame/journal bytes retire.
When 7 later admits, that record lets N1 acknowledge the authority through 8
without losing the earlier client admission. It cannot infer this from a
watermark alone or discard 8's status when its frame is freed. This secondary
record needs its own 256-result capacity and exact epoch/sequence reset rule.
The ordinary proof must ACK 8 while 7 is pending, consume the receipt at
authority frontier 6, reuse retired journal slots under later submissions,
then ACK 7 and clear authority history through 8 exactly once. A reused slot
must not turn old 8 into another identity. Pending receipts, unacknowledged
journal records and admitted-higher status all count in the simultaneous
per-peer reserve and duplicate checks.

Replacement baseline admission must verify that its exact included result
set and included/expired event set covers every mandatory identity in the
superseded StateGroup interval. It retires those frames as *superseded*, never
as admitted. Chunk/header ACK alone retires no result or event. A stale or
wrong-baseline ACK does not alter the active transfer. Invalid completion of
the **active** replacement follows the selected close policy; it never
forges admission. Same-epoch baseline replacement does not reset message
sequence. Journal-slot reuse requires either a range discipline that remains
valid through replacement or a generation check; a bare index bitset can
alias recycled entries and is not selected by this note.

Submission is one bounded transaction: validate the exact R4 unit and
journal IDs, reserve arena bytes and an entry, reserve journal slots, then
copy the framed bytes and commit the sequence as the final infallible step.
If any check fails, the unsent queue retains its original unit and no sequence
is consumed. Once `submit_next` integrates queue and ledger, the generic
external transfer callback should be narrowed or kept an explicitly unsafe
integration boundary. A panicking transfer callback already poisons the
queue; the ledger must also close or quarantine on uncertain external writes.

## Ordered proof queue

1. Build the ordinary ring with actual split-frame storage, finite exact
   journals, admitted-higher result status and one retained ACK receipt. Prove a full ring, entry cap,
   journal cap, sequence exhaustion and no-mutation refusal. Replay one
   higher result across a pending gap and show ACK cannot silently free it.
2. Add baseline Begin/Chunk/End entries, two physical chunk credits,
   out-of-order staging ACK tombstones, exact inclusion and supersession.
   Re-run the rejected-sequence eight-chunk oracle against these owners.
3. Connect the sent owner to the unsent queue with an exact framed submission
   API; verify uncertain writes preserve sequence/bytes and cannot retry under
   a new number. Only then promote the queue's transfer contract.
4. Integrate W1 capture, game schema closure, event dedup and session epoch;
   measure the full four-peer mandatory burst, slow ACK >8 ticks, all owner
   high-water bytes, callback tails, allocator calls and RSS. The 0.5 ms/tick,
   256 KiB/s and 65,536-byte quarter-second targets remain unqualified.

The storage representation above is provisional until the ordinary and
baseline paths both pass. The protocol and whole-engine ownership contracts
remain the authority if this candidate layout proves inadequate.

## Simultaneous capacity, before a game recipe is selected

R4's independent record maxima are not an admitted simultaneous workload.
One maximum group is 6,944 payload bytes and 7,008 bytes with its envelope.
Only the controlled partition carries results. With four partitions, the
largest structurally shaped tick is therefore one 7,008-byte controlled
group plus three 4,960-byte result-free groups. Three such 60 Hz ticks make
65,664 wire bytes over one 20 Hz release, 128 bytes above the 65,536-byte
quarter-second network target even before control traffic. Their 12 queue
slots fit the 16-slot unsent owner, but a fourth unreleased tick occupies
all 16 slots. If every permitted group also carries 64 distinct events and
the controlled group carries 64 results, that three-tick window asks for
768 event IDs and 192 result IDs. The ordinary sent journals reserve only
128 events and 256 results. Game schema and dependency closure may reduce
these maxima further; this calculation alone admits no workload.

The first production game recipe must therefore state **joint** per-peer
partition, actor, result, event and lifecycle burst bounds over the cadence
and slow-ACK windows. Admission must preflight that combined work against
the queue, journals, sent-frame arena, baseline and sync closure, then pace,
recapture or explicitly close on overload. It must never claim that all R4
individual maxima can be emitted together or convert a mandatory unit into
coalescible state to make the budget fit. The owner-byte and CPU measurement
must include the old queue and sent frames, immutable 256 KiB image, baseline
frame slots, and new sync queue at the same time; the ordinary four-peer
measurement does not cover that peak.

## Integration boundary after the first physical slices

The [ordinary ledger](../../evidence/integration/N1-ORDINARY-SENT-LEDGER-2026-11-03.md)
now owns exact StateGroup frames, pending result/event IDs, one application
sequence and the result-retirement frontier. It conservatively closes after
an exact Reject. The [physical baseline oracle](../../evidence/integration/N1-BASELINE-PHYSICAL-STAGING-2026-11-03.md)
proves two chunk slots and tombstones, but its externally supplied sequence
numbers are not joined to that production owner. The next implementation
must put ordinary and baseline frame kinds under **one** sequence allocator.
It must not copy the oracle's independent counter into a second public owner.

The final BaselineAck transaction is wider than the sent ring. A baseline
certificate must bind the immutable image hash and unique baseline ID to the
Begin sequence, capture tick, `supersedesThrough`, exact included result IDs,
event IDs and permitted transient-event expiry. At the capture barrier the
owner freezes the **pre-cutoff** mandatory union of already submitted ordinary
frames and unsent queue units. The game-schema/image validator must prove
that the frozen image includes each exact result and required event in this
union, with the corresponding state/lifecycle closure. A valid ACK may retire
only that certified pre-cutoff set. A conservative first path requires exact
event inclusion and closes when transient expiry cannot be proved. Wire
framing and a hash of opaque image bytes alone cannot establish this semantic
certificate.

Output committed **after** the baseline capture tick belongs to a distinct
bounded subsequent-sync queue. It survives BaselineAck and is submitted under
the replacement partition map and advanced interest generations after the
ACK transaction succeeds. It cannot be covered by the frozen image or retired
by that ACK. The old unsent queue and sent frames, immutable baseline image,
and new sync queue therefore coexist. Clearing the old queue at Begin or ACK,
or reusing it for post-cutoff capture before old mandatory identities are
certified, would lose accepted output. Sync overflow follows an explicit
abandon-and-recapture or close decision; it never drops mandatory units.

For the first replacement path, require every already assigned ordinary frame
to be fully written and no occupied ACK receipt before Begin. Otherwise close
this peer under the current policy. This avoids an unqualified ordered-drain
path while preserving the old frames and their exact application sequences.
Begin/Chunk/End then consume the **same** sequence counter as StateGroup. A
staged chunk frees only its frame slot and credit; its sequence tombstone stays
until final disposition. Ordinary cumulative ACK must inspect entry kinds
and explicit disposition, not equate numeric sequence distance with the
number of ordinary entries. A rejected StateGroup remains rejected even when
a later chunk stages successfully.

One final ACK receipt stays occupied across callback refusal. With the
current Authority's match/peer/epoch checked inside the callback, commit the
contiguous result prefix first; only then may the N1 owner mark old ordinary
sequences *superseded*, retire the exact included mandatory identities,
release baseline frames/tombstones, advance the cross-kind disposition prefix
through End, and resume ordinary submission at the next unused sequence.
Callback unwind or uncertain transport submission closes/quarantines the
whole peer owner. A wrong, stale or invalid BaselineAck changes no owner state;
invalid completion of the active replacement follows the selected close
policy rather than pretending the rejected frame was admitted.

The first integrated rejection proof must hold a sent rejected result and an
unsent mandatory result at the cutoff, progress eight chunks through two
credits with an out-of-order staging ACK, and capture another mandatory unit
after the cutoff. Reject wrong/stale/premature BaselineAck and omitted result
or event identities without retirement. Fail and retry the authority callback
with the exact occupied receipt; caught unwind requires close. A successful
ACK retires only the certified old union, preserves the new sync unit and
advances the result frontier through an earlier admitted-higher gap exactly
once. Exercise an unwritten assigned frame, sequence exhaustion and sync
capacity refusal as explicit close/recapture outcomes. The existing physical
oracle proves frame storage and staging only; semantic completion also needs
a selected image encoder, validator and consumer to check actual image
contents against the certificate.

If a Reject arrives while later already assigned frames remain unwritten,
the current conservative ledger requires close. A production replacement
path may explicitly drain those same numbered frames before Begin, keeping
their bytes and journal identities charged; it cannot assign fresh numbers
or claim their admission. This ordered-drain policy must be proved with the
actual transport adapter before it changes the conservative boundary.
