# Accepted-source selected-head audit — 3 November 2026

The format-2 provisional store checks that its selected generation is the highest committed publication. The accepted format-1 store previously checked only that some outcome matched the selected root. If a retained database had A selected again while B's later outcome remained, `NativeSourceStore::open` could report A and let a new admission proceed under that older generation.

`NativeSourceStore::open` now checks its selected root against the highest retained outcome before returning the owner. `current_root` uses the existing outcome-generation index for a descending top-two lookup. It requires the latest generation and revision to match the selected root, and refuses a second outcome at the latest generation. An empty selection requires no outcomes. This replaces the former separate existence lookup, so the common selected-root read remains a bounded index lookup rather than a scan of retained history. It adds no work to game ticks or pinned file chunks.

A public-path fixture admits A then B. After orderly close, a test mutation rewinds only the root to A while B's outcome remains; fresh open refuses `CorruptStore`. Restoring B and injecting a second outcome at B's generation also refuses. The fixture proves detection of those two inconsistent retained states. It does not prove protection from a coherent rollback of the entire database, an external noncooperating writer, OS or power loss, nor validation of every older outcome row. The accepted schema is still format 1; format-2 publication intents and explicit migration are not implemented in this slice.

Focused test and `RUSTC_WRAPPER= ./dev check` pass locally after this change.
