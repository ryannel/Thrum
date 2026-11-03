# Provisional source physical-space probe — 3 November 2026

The [selected proof archive](SOURCE-PHYSICAL-ENVELOPE-PROOF-2026-11-03.tar.gz)
contains the owned public-API harness, exact SQLite/VFS tool identities,
pre/post-fix raw traces, fault logs, a source patch and the detailed `REPORT.md`.
It is **57,651 bytes**, SHA-256
`b68ebb3cb70ffc66d37a97f3b8f9b29405d1ceaad105061f50cfce4be3f7532b`.
The promoted archive matches the owned source byte-for-byte; its 54 members
were enumerated/read, and the source archive's own hash manifest records its
member identities. Owned run databases and build output were removed after
verification. The current provisional store (historical source: `git show 1a6273a:crates/thrum-source/src/provisional_store.rs`)
remains a development path, not accepted-source authority.

The actual format-2 public publication path was exercised with 1,000 distinct
4 KiB files, a 12 MiB master and repeated 4 MiB edits on Apple APFS with the
linked SQLite 3.53.2. A test-only forwarding VFS counted writes, truncates
and syncs; fresh processes supplied whole-worker RSS samples. Measurements
include trace overhead and are local observations, not a proven per-operation
physical bound or an edit-latency qualification.

| Case | Observation |
| --- | --- |
| Fresh candidate | 245.394 ms instrumented; 35,380,752 B SQLite writes; peak main+WAL allocation 21,712,896 B; whole-worker peak RSS 48,316,416 B |
| Reused-tree environment edit | 31.814 ms instrumented; 251,320 B writes; exact older A lease remained readable |
| Blocked reader | Next edit refused with generation 8 selected, WAL 34,640,992 B and main 4,263,936 B |
| Checkpoint after reader drain | Main grew to 37,916,672 B while WAL was retained; main+WAL peak allocation rose from 38,907,904 to 72,560,640 B, a **33,652,736 B** overlap increase |
| 4 MiB chunk insertion | Up to 8,507,568 B observed writes including automatic checkpoint work |

The old `prepare_write` ordering checkpointed **before** checking available
space. With a test oracle reporting zero availability, it wrote 33,705,984 B
and grew main before refusing; the old selected generation remained intact.
The scoped source correction (historical source: `git show 1a6273a:crates/thrum-source/src/provisional_store.rs`)
checks free space before checkpoint work and again afterward. Replaying the
same WAL debt and zero availability produced **zero writes, syncs and
truncates**, unchanged main/WAL lengths and the old selection. Restoring
availability allowed the exact retry. Synthetic SQLite `FULL` and `SIGKILL`
cases left A readable and recovered B as pending before a successful retry.
The final source SHA-256 is
`b6da4d3f9b335d3f4037c8cdb494d5dd6bf34eef7b47f8807842cac0b551d0ec`;
`RUSTC_WRAPPER= ./dev check` and scoped source tests (**144 passed, 34
ignored**) passed. Independent review found no defect in the ordering fix.
That SHA identifies this historical probe's ordering correction. The later
[explicit checkpoint gate](SOURCE-EXPLICIT-CHECKPOINT-2026-11-03.md)
supersedes its automatic-checkpoint path and records the current source SHA.

No input-byte multiplier from these runs bounds SQLite page splits, index
rewrites, automatic checkpoints or APFS copy-on-write allocation. The 256 MiB
main cap and 64 MiB WAL check remain provisional backpressure, not a physical
peak reservation. A production admission gate needs a versioned resource/VFS
model with chargeable WAL and checkpoint debt, a cooperating reservation lock,
fault coverage and a declared external-volume-consumer assumption. The
[Foundation contract](../../design/FOUNDATION-SPEC.md)
owns that gate; no admitted source, F4 build or power-loss guarantee follows
from this probe.
