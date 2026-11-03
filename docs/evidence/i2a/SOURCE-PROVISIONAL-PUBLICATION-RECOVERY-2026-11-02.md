# Provisional source publication recovery — 2 November 2026

The later [physical-space ordering probe and correction](SOURCE-PHYSICAL-ENVELOPE-2026-11-03.md)
updates the current source hash and source-test count; the recovery protocol
described below is unchanged.

The production-compiled format-2 provisional store (historical source: `git show 1a6273a:crates/thrum-source/src/provisional_store.rs`)
records an exact keyed **pending publication intent before blob staging**.
Its final SQLite transaction checks the expected selection and complete
closure, then advances the selected revision, writes the keyed outcome and
marks that intent committed together. Pending work never auto-selects or
rewrites working files. Reopen and retry can distinguish `Missing`,
`PendingNeedsRetry` and `Committed(exact selection)` without reading a
`StagedCandidate`. This is a development snapshot path, **not** F3 accepted
source, the canonical `RecoveryRoot`, or F4 ready-build authority.

## Identity and interruption proof

The new 162-byte `ProvisionalSelection` token carries fixed format/version,
store and project identities, base generation/revision and an encoding
checksum. It can be persisted before publication and decoded by a fresh
application process. The checksum detects accidental change; it is not
authentication. The status query binds that token, key and target revision to
the exact store request and checks committed intent/outcome and bounded
manifest metadata closure. A committed historical result may be returned
without changing the currently selected root; a pending key does not become
committed because another key selected identical bytes.

Four real child-process kill boundaries were exercised: after durable intent,
after complete blobs but before the final transaction, after all final SQL
writes but before commit, and after successful commit but before
acknowledgement. The first three leave the old selection and a retryable
pending intent; the last recovers the exact committed selection. A separate
fresh-process case drops the original Rust selections and candidate, persists
only token/key/target bytes, dirties the checkout, then recovers the committed
answer through public APIs. A known acknowledgement error after `COMMIT` has
returned success may recover by exact key. If `COMMIT` itself reports an error,
publication returns that error; the caller can reopen and query rather than
assuming a same-connection read certifies durability.

An adversarial review found that checking only the selected row's own
intent/outcome allowed a valid old selection row to be restored after a later
commit. The store now validates a contiguous retained committed history on
open (at most 1,025 rows in this profile) and checks the selected generation
against an indexed committed high-water row on selected reads, leases and
publication barriers. Regression restores A's valid row after B: open,
selection, a new lease, status and C publication refuse; an already pinned A
reader remains valid. Restoring B permits the next generation. The bounded
history and index path are checked by tests. This detects inconsistent rows in
the retained database; a coherent rollback of the entire database requires
an external trusted anchor and is not claimed solved here.

Format 2 explicitly refuses a format-1 database without migration. Tests also
cover wrong/corrupt intent rows, stale bases, pending-key capacity, malformed
or cross-store tokens and final-transaction faults. Final source identity is
repository HEAD `05184410ccdee3494c9558822f98306c4081f277` plus
uncommitted `provisional_store.rs` SHA-256
`f020c580204611c5c855477ff420ccffb373bbf52150cb19cbb5ab724a9e2a53`.
`RUSTC_WRAPPER= ./dev check` passed repository policy, formatting, Clippy and
workspace tests; the source library's scoped replay reported **143 passed,
34 ignored**. The prior [verified-lease and batch evidence](SOURCE-PROVISIONAL-VERIFIED-LEASE-2026-11-02.md)
records the small-file cost, corruption checks and source read behavior.

## Unfinished authority and cost gates

The extra selected-read check is one indexed generation high-water seek; open
performs one bounded retained-history scan. There is no integrated
edit-to-ready or process-memory measurement for this format-2 path. The
provisional profile still uses a heuristic main/WAL/free-space backpressure,
not a proven physical peak reservation. The full canonical RecoveryRoot and
journal/dedup epoch, working-file application recovery, pending-intent
retirement, abandoned-owner recovery, safe GC, power-loss qualification and
admitted-source interpretation pins remain open. No F4 cooker, runnable game
or source editor follows from this result.
