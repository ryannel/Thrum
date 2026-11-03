# Charged read-only baseline

1 November 2026. This is the first product integration of the selected shared-column group owner. It applies to a read-only baseline from a sealed, pool-charged authority. Live tick output, replica admission and host delivery still use the prototype path.

`Authority::try_baseline_charged` counts all full-state changes, retained results and events, and peer watermarks. It checks the four column sizes and reserves their combined requested bytes before allocating a column or copying a value. A nonempty column has a fixed fill limit, a private vector and one pool charge behind a shared owner. Cloning the group copies its header and four owner handles; the final reader releases each column charge. An empty column allocates nothing. The prototype `baseline` remains available for unqualified callers.

The two-peer correctness fixture compares every charged baseline column and header with the prototype after a committed event and a game rejection. It proves capacity refusal leaves pool usage unchanged, a copied group retains the charge, and the last group reader releases it after the authority drops. Closed state and outcome values have already passed the authority's pool and size admission, so copying their inline parts shares charged payload handles. Event sorting is in place. This proof does not charge replica maps or callback storage.

The [release fixture](../../../crates/thrum-core/examples/i1_baseline_cost.rs) builds two-peer baselines with 64 and 4,096 fixed-inline records sharing a 512-byte charged payload. It runs 20 warmups and 100 samples, then measures 64 clone-and-drop operations per sample. Two local macOS/aarch64 runs gave these p50 values in nanoseconds; each clone entry is the **whole 64-operation batch**.

| Records | Prototype build | Charged build | Prototype clone batch | Charged clone batch |
| ---: | ---: | ---: | ---: | ---: |
| 64, run 1 / 2 | 459 / 542 | 500 / 542 | 16,917 / 16,625 | 292 / 292 |
| 4,096, run 1 / 2 | 18,583 / 18,417 | 26,917 / 26,959 | 1,089,125 / 1,083,167 | 333 / 292 |

The charged 4,096-record build is slower in these local runs. It counts records and sets up two nonempty charged owners (changes and watermarks), but this fixture does not isolate the cause of the difference. Its clone cost is nearly independent of the record count, and the current deep clone copies every record handle. These timings do not establish whole-tick throughput or a device budget. The pool accounts for engine-requested logical bytes, including vector slots and shared owner headers; it does not count allocator metadata, allocator-supplied slack or game-private memory.

Next: reserve the same columns before a live tick's first edit, account for overlapping authority and required replica storage, and measure the complete host path. The parent whole-process byte gate stays open.
