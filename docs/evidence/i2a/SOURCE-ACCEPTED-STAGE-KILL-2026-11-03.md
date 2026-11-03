# Accepted-source kill after verified stage — 3 November 2026

The accepted admission pipeline writes complete, SHA-256-verified file and manifest blobs before its final root/outcome transaction. A process interruption at that boundary must leave the earlier root selected. Complete unselected blobs may remain append-only; they cannot become an accepted source without a final committed outcome.

A child process opens the accepted store under its cooperating owner lock and runs the same internal candidate admission pipeline used by `NativeSourceStore::admit`. A deterministic test hook stops it immediately after verified B files and manifest, before the final root transaction. The parent observes that another opener is `Busy`, kills the child, and opens the store afresh. It finds A still selected, no B keyed outcome and B's manifest blob retained complete. It registers an A lease, uses the public `admit` method to retry B from a fresh validated stage, then reads A's exact old scene through the live lease. Reopen sees B and its exact keyed outcome.

The hook is a no-op in the production public call and adds no work to game ticks. The test exercises one local process-kill point after verified staging. It does not qualify a kill inside or just after final COMMIT, a lost acknowledgement, coherent database rollback, actual full-volume failure or power-loss durability. The latter cases remain separate gates; the accepted store is still append-only without destructive collection.

Focused child/parent tests and `RUSTC_WRAPPER= ./dev check` pass locally after this change.
