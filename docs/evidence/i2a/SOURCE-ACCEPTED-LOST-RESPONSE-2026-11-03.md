# Accepted-source committed response loss — 3 November 2026

A successful accepted-source admission can commit its root and retry outcome before its caller receives a response. If the process then dies, the next owner must recover the exact keyed result instead of applying the same update twice.

A child process opens the accepted store, stages and validates B, calls the public `NativeSourceStore::admit`, checks that it returned generation 2, then stops before signalling a result to its parent. While it holds the owner lock, the parent receives `Busy`. The parent kills it and opens the store afresh. `outcome` for B's exact epoch/key/request hash returns generation 2, and `current_root` agrees. A public retry with the same candidate and key returns the same B even though its expected base generation is now old; a changed request hash conflicts. A historical accepted A pin still copies exact old scene bytes and the selected root stays B.

This local process-kill point is after a successful public COMMIT. The [precommit stage-kill proof](SOURCE-ACCEPTED-STAGE-KILL-2026-11-03.md) covers the other side of the transaction, while the [injected final-COMMIT I/O proof](SOURCE-NATIVE-COMMIT-IO-RECOVERY-2026-11-03.md) covers one reported post-sync error. A process killed *inside* the final transaction, real power-loss durability, filesystem replacement and complete application RecoveryRoot remain unqualified.

Focused child/parent tests and `RUSTC_WRAPPER= ./dev check` pass locally after this change.
