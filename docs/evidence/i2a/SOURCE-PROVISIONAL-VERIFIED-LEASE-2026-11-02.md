# Provisional native source selection and verified lease — 2 November 2026

This records the first verified-lease and tiny-batch slice. The later
[format-2 publication recovery proof](SOURCE-PROVISIONAL-PUBLICATION-RECOVERY-2026-11-02.md)
adds durable intent, public restart tokens and selected-history integrity;
its source hash and test count supersede the ones below.

The production-compiled provisional store (historical source: `git show 1a6273a:crates/thrum-source/src/provisional_store.rs`)
is a **development path**, not an admitted-source authority or a ready-build
capability. It accepts only a previously F3-validated `StagedCandidate`,
rehashes every staged file before copying, writes verified immutable SQLite
blobs and a canonical manifest, and commits a generation plus keyed retry
outcome together. One short transaction compares the current selection and
registers a durable read pin. A cloneable `VerifiedSourceLease` shares that pin,
retains A after publication of B, and checks complete metadata, bounded chunk
layout, SHA-256, record ID and type before returning record bytes. No SQL read
transaction spans consumer work. Destructive collection is disabled; a failed
final pin release retains its durable row.

The deliberately smaller profile is at most 4,096 files, 16 MiB per file,
64 MiB total source bytes, a 4 MiB manifest, 1,024 outcomes and 1,024 pins,
with a verified 256 MiB SQLite main-page cap. Its 64 MiB WAL check and volume
free-space check are **backpressure**, not a certified physical-peak
reservation. A source selection or lease from this API cannot satisfy F4's
authenticated admitted-source precondition.

## Checked interaction and adversarial correction

The focused tests cover two constant-light source revisions, mutable checkout
drift, exact A reads after B selection, reopened idempotent and conflicting
retries, stale/foreign project or store selection refusal, shared last-owner
pin release, corrupt/missing/oversized BLOB or TEXT chunks, failed lease
acquisition cleanup, abandoned-pin capacity and foreign/unsafe store paths.
The first adversarial review found that an A retry could accept a corrupted
outcome row redirected to valid B. Retry now checks the stored generation and
revision against the publication requested by the original selection and
candidate; regression mutates both fields and leaves current B unchanged.
The second review found that a corrupt oversized chunk could be materialized
before its length was checked. A metadata query now checks BLOB type and length
before fetching payload, using a guarded `CASE` so a corrupt TEXT value does
not run `length()` first. Application buffers stay bounded by the declared
chunk/record limits; this does not prove a process-wide SQLite RSS ceiling.
Independent re-review found no remaining actionable defect in that scope.
Follow-on tests exercise 128-file/1 MiB tiny batches, duplicate and reused
blobs, a failed batch that preserves the prior selection, staged drift of a
reused file, and retry after a batch failure.
An independent adversarial review of the final batch path found no actionable
regression within the documented provisional ownership model. It checked
rollback, reuse drift, duplicate/empty/incomplete blobs and bounded inputs;
physical quota and WAL qualification remain open.

`RUSTC_WRAPPER= ./dev check` passed repository policy, rustfmt, Clippy and the
workspace tests after tiny-file batching. A separate locked/offline
`thrum-source --lib --quiet` replay reported **131 passed, 34 ignored**.
Source identity is repository HEAD
`05184410ccdee3494c9558822f98306c4081f277` plus uncommitted
`provisional_store.rs` SHA-256
`6f4385a56c723e9157cbe55bb0eb3ceb9fb4ef5afe1376af28ccf0d36e79b68c`;
the F3 header change in `environment_json.rs` is
`81c74636bedd08aaf7b2eeaf44d78fba779febc75446bca267298b792f695ced`.
The linked `libsqlite3-sys 0.38.2` bundled archive reported SQLite **3.53.2**,
source ID `d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`;
the local archive SHA-256 was
`5d46f705c3637d63659a8a7f90dd3d34c66e6ea71fe55d849bcc106ef2fcc962`.
This is one local runtime identity observation, not a deployment matrix.

## Performance and authority limits

Publication remains O(the candidate's staged source bytes) because it must
catch post-F3 stage drift. Immutable complete storage is reused by metadata
without a second whole-tree byte sweep; each leased record is hash-checked
when read. Files at most 4 KiB now share one transaction for at most 128 files
and 1 MiB of input; fresh blobs are read and hash-checked inside that
transaction before completion. Reused complete blobs still get a staged-byte
hash check, without packing into the batch or rehashing their stored bytes.
Larger files keep the separate chunk path. The 4,096-file all-tiny case
requires at least 32 batch transactions plus manifest and selection, versus
12,288 per-file blob transactions plus manifest and selection in the previous
path. This is a transaction-count bound, not a physical-space or elapsed-time
guarantee.

An isolated release probe with 1,000 ordinary 4 KiB files and five source
records measured fresh publication at **3,019 transactions / 226.184 ms**
individually versus **12 / 74.032 ms** batched. The small edit measured
**7 / 31.792 ms** versus **5 / 28.712 ms**. After normal SQLite
auto-checkpoint, final WAL lengths were **4,140,632 B** individually and
**4,775,112 B** batched. These are single-run local observations; the batch
WAL was larger, and neither a general speedup nor a worst-case disk bound
follows. Combined source peak memory and edit-to-ready latency remain open.

The [native admitted-source lease contract](../../design/FOUNDATION-SPEC.md#native-admitted-source-lease-for-the-first-f4-cook)
still requires physical peak reservation, truthful durability/recovery root,
schema/definition interpretation pins, safe abandoned-owner handling and
validated accepted publication. No F4 cooker, runnable build, save/restore,
source editor or visual/device proof follows from this slice.
