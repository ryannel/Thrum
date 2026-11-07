# Retained fixed-slot world storage

1 November 2026. The protected fixed-slot host now includes the world's retained slot vector, dirty-page flag vector and stable-ID index in its activation transaction. These buffers existed before activation; the transaction reserves their observed capacities and transfers a charge to the world before selecting the protected profile. It also covers the journal and free-list allocation and old/new free-buffer overlap from the [journal proof](FIXED-SLOT-JOURNAL.md). A failed reservation installs nothing and leaves activation retryable.

The stable-ID table now uses the workspace's pinned hashbrown 0.17.1 with `RandomState`, matching the charged replica's measured-allocation approach. The world charges `by_id.allocation_size()` rather than deriving bytes from logical entry count; this includes its control bytes and spare buckets. The map is never inserted into in the no-spawn protected mode. Despawns assert its allocation size is unchanged, and each protected publication checks the slot and dirty capacities and map allocation against activation. The prototype keeps its normal spawn, reuse and prepared-batch semantics; its temporary validation maps remain separate standard-library maps.

The logical charge for the retained buffers is `slots.capacity × size_of(Slot<T>) + dirty.capacity × size_of(bool) + by_id.allocation_size()`. The free-list and journal charges are separate, so no buffer is counted twice. Observed vector capacity may include allocator-supplied surplus; charging it is conservative. The hashbrown allocation size excludes allocator metadata, as does the rest of the publication pool. Inline slot records are covered here; any immutable payload owners retain their separate charges. The charge outlives the buffers' final use and releases with the world.

Tests cover an empty world, spare vector capacity, an allocated but empty stable-ID table, a reservation one byte short of the complete activation charge, successful retry, exact retained byte delta and final release. The existing fixed-slot tests cover all-object despawn, terminal generations and full free-list occupancy. The complete `RUSTC_WRAPPER= ./dev check` gate passes. The review agent found no scoped defect.

For a regression check, the release fixture ran in A/B/B/A order against commit `739a731` and the changed tree, with 1,000 warmups followed by 10,000 one-change or 3,000 sixteen-change measured callbacks per mode. The variants used separate Cargo target directories and distinct verified executable SHA-256 hashes: before `329fdb581b6b8624f1eb50bf198d6893a9866a650f8daf806d21c4e7ffcf8c13`, after `bf1a1cd89a8531967a6a1e0c52b51dcb385ac934cc0f9ccc72d9eb780499ba11`. The runs are [before A](retained-world-before-a.txt), [after A](retained-world-after-a.txt), [after B](retained-world-after-b.txt) and [before B](retained-world-before-b.txt). Complete submit/callback/acknowledge p50 nanoseconds were:

| Workload | Before A / B | After A / B |
| --- | ---: | ---: |
| 64 objects, prototype one change | 750 / 750 | 750 / 709 |
| 4,096 objects, prototype one change | 1,041 / 1,083 | 1,125 / 1,083 |
| 64 objects, ordinary charged one change | 708 / 709 | 750 / 709 |
| 4,096 objects, ordinary charged one change | 1,000 / 1,084 | 1,042 / 1,083 |
| 64 objects, protected one change | 666 / 667 | 708 / 708 |
| 4,096 objects, protected one change | 959 / 1,041 | 1,042 / 1,000 |
| 64 objects, ordinary charged sixteen changes | 2,500 / 2,625 | 2,708 / 2,667 |
| 4,096 objects, ordinary charged sixteen changes | 3,000 / 3,000 | 3,083 / 3,166 |
| 64 objects, protected sixteen changes | 2,334 / 2,500 | 2,542 / 2,459 |
| 4,096 objects, protected sixteen changes | 2,833 / 2,959 | 2,958 / 2,959 |

Most ranges overlap, while the protected 64-object one-change median was 41–42 ns higher in these two after runs. This is a possible small cost, not a qualified device regression or an established backend-specific cause. The paired runs constrain the broader regression concern from changing the prototype's map backend, but are local macOS/aarch64 samples with sequential modes, synthetic callback times and no allocation counter or named-device qualification. The complete memory gate still needs a bound on game-private rule work, tick-created variable payloads and structural-growth profiles.
