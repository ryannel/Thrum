# Accepted-source partial retry scale — 3 November 2026

This is a narrow F3 qualification of the production `NativeSourceStore`
retry path. It follows the [partial-prefix mechanism](SOURCE-PARTIAL-BLOB-RETRY-2026-11-03.md)
and the [space policy](../../design/research/HOLISTIC-LIMIT-POLICY-2026-11-03.md).
It does not select a disk quota, a safe physical headroom floor or a latency
target.

## Exercised path

An accepted A root and pin stay live while a deliberately shortened staged B
stream leaves 1, 16 or 256 complete four-MiB chunks. B has no keyed outcome
and A remains selected. The same accepted owner then retries B from its full
validated stage under a 40 MiB WAL stop. A rejecting SQL trigger catches
deletion **or update** of any retained B chunk. Every chunk callback copies
and verifies A's exact scene bytes through the old pin. After retry, B is the
selected root and keyed outcome. The callbacks run *between* append calls;
they do not establish concurrent pin progress while SQL is running.

A separate process test kills the actual accepted owner after its sixteenth
B chunk commits. Reopen finds A selected, no B outcome and exactly sixteen
incomplete B chunks. A new A pin reads exact bytes; a fresh full stage retries
B without deleting or rewriting that prefix. A B pin copies and authenticates
the 68 MiB master, an identical keyed retry returns the same B, and a second
reopen retains B and its outcome. The killed process leaves an orphaned
provisional stage; the test's retry stage allowance accounts for that second
copy. Automatic orphan cleanup and its temporary peak are separate work.

A normal test additionally holds a raw SQLite read transaction after seeding
sixteen B chunks. It selects an operational WAL stop at the current WAL length
plus the next chunk's nine-MiB allowance, then attempts the accepted B retry.
All sixteen retained chunks can be compared and A can be read between them,
but the missing chunk receives a `WalCheckpoint` receipt with `checkpoint_busy`
and B remains incomplete and unselected. Releasing only that raw reader lets
the same B retry finish while the A pin remains live. The post-seed stop
selection makes the checkpoint boundary deterministic; it is a test policy,
not a calibrated production threshold. A source pin alone does not hold the
SQLite snapshot that blocks this checkpoint.

A further accepted-owner test seeds sixteen B chunks and removes index zero
to force malformed-layout cleanup. A SQL trigger aborts the second deletion
after the first four-MiB row deletion has committed. The header remains
incomplete with fourteen rows, A remains selected, no B outcome exists and the
old pin still reads A. Reopen reacquires A's pin; after removing the injected
trigger, retry resumes bounded cleanup and publishes B. A B pin authenticates
the full 68 MiB master. This is an injected SQL failure at sixteen chunks,
not host-volume `ENOSPC`, and it does not measure a concurrent A read during
the cleanup loop.

## Local release observation

macOS 26.6.2, arm64, pinned Rust 1.99.1, `cargo test --release -p
thrum-source --lib measure_accepted_partial_retry_with_live_old_pin --
--ignored --nocapture`. Each row is one run on this host. `retry_ms` includes
full candidate admission, synchronous file sampling and A-pin copies. The
append timer covers retained-row lookup, WAL guard/checkpoint and any bounded
write/commit inside `append_blob_chunk`; it excludes layout scan, cleanup,
stored-blob verification, completion and final root/outcome commit. It is
compiled only for tests.

| Retained / total chunks | Retry elapsed (ms) | Max append call during retry (ms) | Sampled max main length (bytes) | Sampled max WAL length (bytes) | Logical stage copy (bytes) | Exact A reads |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 / 2 | 62.917 | 17.113 | 4,096 | 8,722,072 | 8,389,174 | 5 |
| 16 / 17 | 145.926 | 16.944 | 54,718,464 | 33,985,912 | 71,303,734 | 35 |
| 256 / 257 | 1,619.838 | 11.684 | 1,059,495,936 | 33,990,032 | 1,077,936,694 | 515 |

Main and WAL are apparent file lengths sampled only after each streamed
chunk and after the retry. Their maxima can occur at different moments;
checkpoint overlap or allocation peaks between samples are unknown. Stage
copy is one logical provisional copy, not a measured temporary high-water.
No physical APFS allocation, RSS or unallocated-container bound follows.
The seed phase's maximum append calls were 15.914, 117.133 and 82.417 ms;
these are separate from the retry values above. The 40 MiB stop is an
operational checkpoint threshold, not a maximum physical WAL allocation.

The same run also sampled Unix `st_blocks × 512` after each streamed chunk
and measured the captured B stage's file blocks after staging. These are
filesystem-reported allocated blocks at observation points, not a complete
APFS container charge or a high-water guarantee.

| Retained chunks | Max sampled main blocks (bytes) | Max sampled WAL blocks (bytes) | B stage file blocks after capture (bytes) |
| ---: | ---: | ---: | ---: |
| 1 | 4,096 | 8,724,480 | 8,400,896 |
| 16 | 54,718,464 | 33,988,608 | 71,315,456 |
| 256 | 1,073,807,360 | 34,619,392 | 1,077,948,416 |

At 256 chunks, sampled main allocated blocks exceeded its sampled apparent
length. The maxima are independent observations and must not be added as if
they occurred at the same instant. Directory metadata, other provisional
stages, SQLite internal transients, concurrent F4 work and APFS snapshot or
clone accounting remain outside this table.

An additional opt-in release probe seeded 256 four-MiB B chunks, removed
index zero and retried the malformed B through the accepted owner. Cleanup
deleted the remaining **255 four-MiB rows**, one per transaction, before
restaging B. Its test-only timer starts after each row-selection query and
covers the WAL guard/checkpoint and DELETE transaction through commit: 255
measured slices totaled 1,033.982 ms, with a 5.428 ms maximum. The whole
retry took 6,244.751 ms, including cleanup, restaging, verification, 257
synchronous exact A-pin reads and B publication. The post-retry apparent
main/WAL lengths were 1,076,318,208 / 21,267,472 bytes. This is one local
host run, not a latency promise. It does not measure the SELECT work preceding
each deletion, an allocation high-water or a concurrent A read during the
cleanup loop.

The normal accepted-owner crash, reader-blocked WAL and cleanup tests, and
both opt-in scale tests, passed.
An independent adversarial review corrected the child fixture lifetime,
the kill deadline, the update oracle and the seed/retry timer scope before
this checkpoint.

## Native out-of-space after admission preflight

An opt-in test used a task-owned 256 MiB APFS disk image for the accepted
SQLite database while its validated source stage stayed on the host volume.
The test-only hook runs after the actual `append_blob_chunk` live WAL/space
guard and before the first B-chunk SQL transaction. It filled the image until
a native `ENOSPC` write, attempted to release rollback/unlink room by
truncating the filler, and then let SQLite continue. In the clean run, the
filler completed 248 one-MiB writes; observed available space was
266,792,960 bytes before B admission and 5,701,632 bytes after failed
admission. SQLite reported `SQLITE_FULL`, not a planning receipt. After
removing the filler, the old A pin still returned exact bytes, A remained
selected and B had no keyed outcome. A fresh accepted owner observed the
same A/none state and admitted B with its exact keyed outcome. The test
removed its database subdirectory after completion; a test-only unwind
guard also clears the pending hook and attempts to remove the filler on failure.

This is a real native write fault after one successful live guard, with the
fault created by a test-only competing writer. It does not prove a physical
free-space reservation, other APFS allocations, a cleanup/final-COMMIT space
fault, power-loss recovery, or cross-volume staging behavior. The test is
opt-in because it requires a separately mounted bounded volume.

## Cleanup pressure after a competing native out-of-space write

A separate opt-in test used an exclusive task-owned **384 MiB APFS** image
for the accepted SQLite database; the validated A/B source stages stayed on
the host volume. It seeded the first 64 MiB—sixteen four-MiB chunks—of a
68 MiB B master,
removed index zero to make the incomplete prefix malformed, and truncated
the WAL. The ordinary cleanup's descending index-15 deletion committed.
After the **index-14 live guard** succeeded, a test-only hook wrote four-KiB
filler blocks until its own filesystem write returned native errno 28. The
filler completed 300,972 KiB of whole writes; the image's observed available
space fell from 316,866,560 to 8,523,776 bytes. The selected SQLite
`secure_delete` setting was 0; the test did not change it.

The index-14 deletion **also committed** with the filler present. The next
bounded slice, index 13, returned typed `AvailableLive` pressure with a
9,437,184-byte incoming allowance, zero checkpoint debt and 8,523,776
bytes observed available. B's header remained `state=0` with exactly indices
1–13 retained; A stayed selected and B had no keyed outcome. A's old pin
returned exact scene bytes **before filler removal**. After removing the
filler, a fresh owner found the same incomplete B rows, reacquired A's pin,
completed B cleanup/retry and authenticated the full 68 MiB master through
B's pin. A second reopen found B's exact selected root and keyed outcome.

This is native `ENOSPC` in the **competing filler**, followed by successful
ordinary cleanup SQL and a live *planning refusal*. It does not establish
`SQLITE_FULL` in a cleanup DELETE, a physical free-space floor, concurrent
A reads during the cleanup loop, process-kill recovery or a latency result.
The test-only cleanup timer includes filler execution and was not used for
cost evidence. The run requires its own mounted image; it should not run
concurrently with the separate append-fault probe on the same volume.
After mounting an exclusive 384 MiB APFS image, run the ignored test with
`THRUM_TEST_ENOSPC_VOLUME_DIR` set to its mountpoint:

```sh
RUSTC_WRAPPER= rustup run 1.99.1 cargo test --offline --locked -p thrum-source measure_accepted_cleanup_pressure_after_native_enospc -- --ignored --nocapture --test-threads=1
```

## Still open for the full F3 gate

An interrupted hundreds-chunk cleanup, native-volume-induced SQLite
`SQLITE_FULL` during ordinary cleanup and final publication,
whole-phase SQL maxima, temporary growth and higher-fidelity main/WAL
allocation sampling remain unproved. The parent F3 build-plan item stays
unchecked.
