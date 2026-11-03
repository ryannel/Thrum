# Provisional source explicit checkpoint gate — 3 November 2026

The production-compiled but explicitly provisional
[`ProvisionalSourceStore`](../../../crates/thrum-source/src/provisional_store.rs)
now owns checkpoints on its private SQLite writer. The writer disables and
reads back automatic and last-close checkpointing and verifies the pinned
SQLite source ID and disabled data mmap. At the provisional 32 MiB WAL trigger,
one central path checks free space, obtains a non-writing `NOOP` snapshot,
plans a conditional main-file **data-write** allowance, requests `PASSIVE`
progress and then `TRUNCATE`. It refuses more staging while a reader retains
checkpoint debt. The earlier [options study](../../design/research/SOURCE-PHYSICAL-RESERVATION-OPTIONS-2026-11-03.md)
derives the narrow allowance and lists its assumptions.

For page count `N`, WAL frames `L`, backfilled frames `C` and 4096-byte pages,
the planner uses `min(N, L-C) * 4096` bytes. It charges this in addition to
the 64 MiB provisional free-space floor while retaining the WAL; it never
credits a hoped-for truncate. The writer lock spans plan and execution.
This bound is conditional on the pinned SQLite 3.53.2 implementation, valid
counters and the audited native VFS. It does **not** bound ordinary staging
SQL, sync or metadata work, APFS copy-on-write allocation, or other volume
users. No accepted-source physical reservation follows.

The checked-in test-only [native VFS observer](../../../crates/thrum-source/src/provisional_store_vfs_test.rs)
records actual main-file calls and injects a main-write error. Its unsafe
callbacks are absent from production builds. Tests cover no automatic,
`NOOP` or last-close main writes; a partially backfilled `PASSIVE`; blocked
`TRUNCATE` even when all frames are backfilled; pre- and post-checkpoint
free-space refusal; and a checkpoint I/O error. The old A lease remains
readable, B's exact committed outcome survives maintenance failure, and an
identical retry returns B without another write attempt. Independent
adversarial review reproduced four targeted tests and found no actionable
defect in this scope.

One **instrumented local release sample**, not a percentile or device
qualification, used 1000 distinct 4 KiB notes, a 12 MiB master and five
project records. Capture and candidate validation precede the timed
publication. The code-retained probe is
`provisional_store::tests::measure_checked_checkpoint_fresh_and_record_edit`;
run it with:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline --release -p thrum-source provisional_store::tests::measure_checked_checkpoint_fresh_and_record_edit -- --ignored --exact --nocapture
```

| Case | Publication | Explicit maintenance | Planned allowance | Observed main data writes | WAL after |
| --- | ---: | ---: | ---: | ---: | ---: |
| Fresh, 1006 files | 94.32225 ms | 17.813666 ms | 17,596,416 B | 17,596,416 B | 0 B |
| One-record edit | 32.272709 ms | 0.659125 ms | 258,048 B | 233,472 B | 0 B |

The independent C observer from the earlier options study also saw
zero main writes during publication. With the observer registered under the
explicit native `unix` VFS, fresh checkpoint wrote 17,596,416 B in
18.986958 ms and edit checkpoint wrote 229,376 B in 0.529916 ms. These are
separate single samples, not a speedup comparison. The temporary input
database and duplicate observer source were removed; the generators remain
in the checked-in test and scratch recipe.

The final production-source SHA-256 is
`053e11ca26bbcd7870b5501c0861311b36595e8474b76ee06e45b45e65db28eb`.
`RUSTC_WRAPPER= ./dev check` passed against the current tree after a clean
rebuild of generated `thrum-source` artifacts: **147 passed, 35 ignored**
source tests, plus repository guards, rustfmt and Clippy. An earlier run had
incorrectly reused a stale test binary (152 old tests); it is not a valid
current-source gate. The direct workspace Cargo test and Clippy
gates also passed. `git diff --check` passed.

The remaining source-admission gate needs bounded staging SQL and WAL I/O,
durable credit/bootstrap recovery, APFS physical allocation qualification,
the actual 8 GiB floor and constrained-volume fault proof. This work does
not advance the canonical RecoveryRoot, accepted-source authority or F4.
