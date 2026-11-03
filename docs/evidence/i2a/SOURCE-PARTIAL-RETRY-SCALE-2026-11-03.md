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
| 1 / 2 | 51.017 | 12.416 | 4,096 | 8,722,072 | 8,389,174 | 5 |
| 16 / 17 | 143.016 | 13.038 | 54,718,464 | 33,985,912 | 71,303,734 | 35 |
| 256 / 257 | 1,352.527 | 11.418 | 1,059,495,936 | 33,990,032 | 1,077,936,694 | 515 |

Main and WAL are apparent file lengths sampled only after each streamed
chunk and after the retry. Their maxima can occur at different moments;
checkpoint overlap or allocation peaks between samples are unknown. Stage
copy is one logical provisional copy, not a measured temporary high-water.
No physical APFS allocation, RSS or unallocated-container bound follows.
The seed phase's maximum append calls were 10.870, 75.045 and 72.426 ms;
these are separate from the retry values above. The 40 MiB stop is an
operational checkpoint threshold, not a maximum physical WAL allocation.

The normal accepted-owner crash test and this opt-in scale test passed.
An independent adversarial review corrected the child fixture lifetime,
the kill deadline, the update oracle and the seed/retry timer scope before
this checkpoint.

## Still open for the full F3 gate

The retained A pin does not itself block a SQLite checkpoint. A separate
reader-held checkpoint refusal and retry, interrupted malformed-layout
cleanup at scale, actual native `ENOSPC` after a successful guard check,
whole-phase SQL maxima, temporary growth and higher-fidelity main/WAL
allocation sampling remain unproved. The parent F3 build-plan item stays
unchecked.
