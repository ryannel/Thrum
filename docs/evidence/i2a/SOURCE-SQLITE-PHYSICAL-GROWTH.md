# SQLite physical growth and candidate batching, narrow proof

2 November 2026. The test-only candidate-to-root helper now groups staged files of at most 4 KiB into transactions of at most 128 files and 1 MiB input. One contiguous byte buffer and one range table are reused across groups; the input buffer can hold at most 512 KiB with this file threshold. Every staged file is reread at its declared length, checked for trailing bytes and hashed before its group commits. Larger files still use the 4 MiB chunk path. The existing assembled-candidate tests pass for edits, staged drift, carried lineage, stale generation, reopened retry and a concurrent same-token winner. A read-only review found no issue. `RUSTC_WRAPPER= ./dev check` passes.

A release-mode probe on the local macOS arm64 filesystem disabled SQLite automatic checkpoints to expose append growth. It used the pinned Rust 1.99.1 build, SQLite page size 4 KiB, an empty fresh store, one 4 MiB blob and 1,000 distinct eight-byte blobs. File lengths were observed with `metadata.len()`; these are **logical file lengths**, not allocated blocks or a filesystem-wide free-space measurement.

| Point | Main file | WAL file | Logical SQLite pages |
| --- | ---: | ---: | ---: |
| Schema created | 4,096 B | 65,952 B | 9 |
| One 4 MiB blob complete | 4,096 B | 4,321,912 B | 1,035 |
| Plus 1,000 tiny blobs, batched | 4,096 B | 5,112,952 B | 1,081 |
| Reuse same complete 4 MiB blob | 4,096 B | 5,112,952 B | 1,081 |
| After successful `TRUNCATE` checkpoint | 4,427,776 B | 0 B | 1,081 |

Fresh-store matched 1,000-file runs used **844,632 B WAL batched** and **22,655,912 B WAL one file at a time**, a 26.8× WAL-length difference for this fixture. The second scene edit through full capture, validation and root proof took 3.708 ms in one release run after the batching change; this tiny fixture is not a useful claim of end-to-end speedup over the earlier 2.554–3.891 ms runs. The storage result supports batching because per-transaction page rewrites dominate the repeated tiny-file path. SQLite [describes the WAL as appended page changes and checkpoints as transfer to the main database](https://www.sqlite.org/wal.html); the observed amplification remains workload and filesystem specific.

Reproduce with `RUSTC_WRAPPER= rustup run 1.99.1 cargo test --locked --offline --release -p thrum-source sqlite_store::measure_sqlite_physical_growth -- --ignored --exact --nocapture`. Source SHA-256: `0232c231c21694114d52d408f6369099c844280f3f9568a3b6481315c39e49bd` (`crates/thrum-source/src/sqlite_store.rs`); executed release test binary SHA-256: `691f1822c367e82daf604394f1d1f895656912e78b46de90493668daf4d02ece`.

This is a **growth observation and a batched candidate-path connection**, not a physical admission reservation. Before accepted-source publication, the store still needs a prewrite check using actual volume free space, current main/WAL/temporary allocations, retained closures and unique missing candidate blobs; a per-write guard and checkpoint policy must keep the 2 GiB WAL boundary without counting a blocked checkpoint as reclaimed. The main 52 GiB page cap, 8 GiB free floor, GC pins, process-kill recovery and production admission API remain unproved. In particular, a single successful `TRUNCATE` here does not qualify checkpoint behavior with live readers or sustained admission. See the [native physical-space contract](../../design/FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc) and [SQLite checkpoint documentation](https://www.sqlite.org/pragma.html#pragma_wal_checkpoint).
