# Native store disk-fault and usage follow-up

2 November 2026. **Production API fault tests and local sampled usage; disk qualification remains open.** This follows the [accepted-source boundary](SOURCE-NATIVE-STORE.md) and the [configurable planning decision](SOURCE-PHYSICAL-GUARD-DECISION.md). The store remains append-only and does not set a production SQLite `max_page_count` cap.

`RUSTC_WRAPPER= CARGO_TARGET_DIR=/Users/ryannel/Workspace/thrum/target ./dev check` passed after this follow-up: 115 `thrum-source` tests passed, with 37 optional measurement fixtures ignored.

The public `NativeSourceStore::admit` path was tested with a genuine SQLite `SQLITE_FULL` during a candidate blob write. The test uses `PRAGMA max_page_count` solely as fault injection after an older source is accepted. The failed call reports `SpacePressure`; current root and retry outcome stay at the old accepted state. Reopening with the injection gone verifies both old values and admits the same staged request. A second test pre-stages complete candidate bytes, then injects a one-megabyte outcome-trigger write inside the final transaction under the same test-only page limit. The root update rolls back with the outcome, and reopening plus retry succeeds after dropping the trigger. The existing test-only process-kill cases remain separate evidence; neither case simulates host-volume exhaustion or power loss.

A production-API fixture also holds an SQLite reader transaction while admitting a 12 MiB asset under a 12 MiB WAL planning stop. After one 4 MiB chunk, the next guarded write defers; the old accepted root and absent retry outcome remain observable. Draining the reader permits the same candidate to retry and commit. Three local debug runs each observed a 4,272,472-byte WAL at refusal, then an 8,470,528-byte main file and a 4,297,192-byte WAL after recovery. These are boundary observations for one candidate, not a long-running workload or a true high-water trace.

`NativeSourceStore::space_usage` is an optional tooling query, outside simulation callbacks. It returns apparent SQLite main and WAL file lengths, logical page count bytes, reusable freelist bytes and observed available volume bytes. These values are read sequentially and are not an atomic view. Freelist bytes are reusable **inside** SQLite and are not reclaimed volume space. File length is not allocated blocks; an application can sample before/after work and report pressure, but this query cannot reserve blocks or prove a transient peak. The first bounded store can retain old revisions even when the plan refuses a new admission.

## Local measurements

Rust 1.99.1 optimized `thrum-source` test binary on local macOS arm64. Three sequential runs of ten successive small three-file edits each. Stage capture and semantic validation completed before each admission timer; usage was sampled after each admission and outside both timers. The sample excludes cook, world startup, concurrent external volume users and process RSS.

| Run | Admission min/median/max ms | Pin and verified scene copy min/median/max ms | Largest sampled main / WAL / logical main bytes | Lowest sampled available bytes |
| --- | --- | --- | --- | --- |
| 1 | 24.467 / 32.558 / 34.829 | 7.895 / 9.046 / 9.947 | 4,096 / 1,124,792 / 77,824 | 108,496,158,720 |
| 2 | 22.323 / 33.434 / 35.084 | 7.923 / 8.975 / 9.967 | 4,096 / 1,124,792 / 77,824 | 108,496,150,528 |
| 3 | 21.035 / 32.518 / 40.350 | 8.977 / 9.906 / 9.982 | 4,096 / 1,124,792 / 77,824 | 108,496,150,528 |

The WAL holds most of this tiny-store sample before checkpoint; a 4,096-byte main-file length does not imply that only 4,096 bytes of accepted source were retained. The largest sampled lengths are post-admission observations, not peak allocated blocks or a 99th-percentile latency. Other local workloads were not controlled.

## Remaining work

Measure the production path under prolonged reader-blocked checkpointing, larger new/reused source overlap and actual host-volume free-space change; record main/WAL/managed-temporary allocated blocks and observed high-water without assuming the planning estimate is a hard cap. Test host I/O faults and platform VFS/power-loss conditions separately. Collector deletion and transitive owner pins remain a later gate. See the [build plan](../../BUILD-PLAN.md).
