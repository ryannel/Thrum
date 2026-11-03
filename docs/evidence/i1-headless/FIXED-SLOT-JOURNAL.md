# Charged fixed-slot change journal

> Status after the code-quality cleanup: the I1 timing examples cited here were removed from the active tree. Their historical measurements and source remain in Git; quoted example commands are no longer current commands.

1 November 2026. The selected protected, inline-output host now activates a charged change journal before accepting inputs. Its fixed-slot game declaration has zero spawns and at most two edits per action. The original `BTreeMap` journal remains for prototype and ordinary charged worlds, preserving their unrestricted lifecycle behavior.

Activation requires a clean charged publication and the authority's pool. It reserves one mark per existing slot, a touched `(StableId, slot)` vector bounded by `min(slot count, maximum peers × 8 pending inputs × maximum edits per action)`, and a free-slot buffer large enough for every existing slot. If the old free buffer is replaced, the reservation also covers its overlap with the new buffer. A failed or foreign-pool activation leaves the world unchanged. Charges remain with the world until its final owner drops. The protected authority activates this representation before setting its execution-credit profile; a failure prevents host construction.

The first touch of a slot appends its stable ID and slot once. Repeated replacements update its mark; a later despawn converts it to a despawn without losing the stable ID. Recording costs O(1) per edit and allocates no tree node. Publication sorts only the C touched IDs in O(C log C), emits the same stable-ID ordered full changes expected by the charged replica, resets marks, and reuses the buffers. The existing free list never exceeds slot high-water when spawns are forbidden. Generation-exhausted slots remain retired. The snapshot and group output reservations remain separate future-input credits.

Focused tests cover full competing-pool refusal and retry at activation, foreign-pool refusal, pre-existing free slots, repeated replacement, an invalid second edit leaving the journal untouched, update then despawn, reverse-ID touches, full free-list occupancy, an exhausted generation, two consecutive publications, old snapshot isolation and final charge release. A protected host test updates and then despawns an object under full competing pool use; the required replica sees both results. The review agent found no defect in the scoped design or implementation. `RUSTC_WRAPPER= ./dev check` passes.

The `release fixture` (historical `git show 84d7673:crates/thrum-core/examples/i1_charged_replica_cost.rs`) runs 1,000 warmups and 3,000 measured two-peer callbacks with 16 distinct inline replacements per tick, followed by acknowledgements. The ordinary charged path keeps the tree; the protected path uses the journal and execution credits. Two sequential macOS/aarch64 runs gave these complete-path p50 nanoseconds:

| Objects | Ordinary charged, runs 1 / 2 | Protected, runs 1 / 2 |
| ---: | ---: | ---: |
| 64 | 2,750 / 2,667 | 2,417 / 2,458 |
| 4,096 | 2,750 / 2,791 | 2,792 / 2,708 |

The one-change protected callback median was 750 ns at 64 objects in both runs, versus 792 / 833 ns ordinary; at 4,096 objects, protected was 958 / 1,000 ns versus 958 / 1,000 ns ordinary. The 64-object many-change difference is encouraging, while the 4,096-object ranges overlap. These modes run sequentially, include different admission work, and were measured on one local machine; they do not establish a device-level speedup or isolate the journal alone. Slot storage, dirty flags and the existing stable-ID hash table are unchanged and still outside the complete mutable-world charge. Game-private callback scratch and tick-created variable payloads also remain open.
