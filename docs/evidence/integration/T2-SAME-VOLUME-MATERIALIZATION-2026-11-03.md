# F3/F4 project temporary composition — 3 November 2026

This standalone public-API probe exercises the current F3 stage and F4
materializer under one project `.engine` tree. It tests the
[cross-owner space boundary](../../design/research/SOURCE-SPACE-RECEIPT-DESIGN-2026-11-03.md#project-temporary-bucket-across-f3-and-f4),
without changing either owner. It is a local operational-policy proof, not
a physical-space reservation or a ready-build qualification. The final run
used local main `d0d091318285433bc2c5cb5cb5d7e11b050bc215`, macOS
26.6.2 arm64 and Rust 1.99.1. The archived probe source has SHA-256
`b48a7778c6eb29f9b9c7892b607658a245f0c410ff229641fb6fadc8237d9ea3`.

## Fixture and result

An accepted A contains a 12 MiB declared opaque master and a small valid
scene. Its provisional stage is closed before the experiment; one accepted A
pin remains live. The scaled `SourceSpacePlan` selects **416 MiB main,
16 MiB WAL, 16 MiB managed temporary and 64 MiB desired observed free
headroom**. F4 uses a 16 MiB *per-tree* `BuildScratchPlan` with the same
64 MiB headroom. Both owned build-input destinations are under the project's
private `.engine` directory.

| Observation | Bytes |
| --- | ---: |
| Verified bytes copied into each F4 input | 12,583,454 |
| Two retained F4 inputs, copied bytes | 25,166,908 |
| Each input's `fs4::allocated_size` file sum | 12,595,200 |
| F3 existing-use `TemporaryScan` lower bound with both inputs | 25,268,224 |
| F3 existing charge after closing one input | 12,673,024 |
| F3 projected complete B stage with one input left | 12,644,352 |
| Selected managed-temporary/per-tree target | 16,777,216 |

Both F4 materializations succeeded and retained their A identity even though
their combined copied bytes exceeded the selected target. A tiny B scene
edit then failed F3 provisional capture with typed `TemporaryScan`; A stayed
selected and every captured file in both F4 directories still matched its A
bytes. After closing one F4 owner, B
capture failed with `TemporaryPreflight` because its **existing charge plus
complete stage projection** exceeded the same target. After closing the
second owner, B captured and admitted as generation 2. A's old pin still
returned the exact A scene. A relaxed-policy B stage taken while both F4
owners were live had the same `SourceRevision` as the finally accepted strict
stage, confirming that the operational refusal did not alter byte identity.

Filesystem availability samples before F4, with two inputs, with one and
after both closed were 74,711,224,320; 74,686,009,344; 74,698,604,544 and
74,711,203,840 bytes. They are non-atomic host-volume observations with
ample margin above the 64 MiB headroom. This fixture demonstrates the
asymmetry between F4's per-tree check and F3's project-bucket scan; it does
not test a headroom breach, simultaneous preflight race, F4 compiler output,
external private cache, actual `ENOSPC` or a composed T2 quota.
The two calls were sequential with overlapping owners. The allocated-size
figures count file data reported by `fs4`, excluding directory metadata;
the F3 scan lower bound includes other `.engine` charges and allowances.

The source is [T2-SAME-VOLUME-MATERIALIZATION-PROBE.rs](T2-SAME-VOLUME-MATERIALIZATION-PROBE.rs),
with its [manifest](T2-SAME-VOLUME-MATERIALIZATION-PROBE.toml) and
[lockfile](T2-SAME-VOLUME-MATERIALIZATION-PROBE.lock). Copy those to
`tmp/f3-f4-space-composition/src/main.rs`, `Cargo.toml` and `Cargo.lock`
respectively, then run `RUSTC_WRAPPER= cargo run --locked --offline --release`
there. The manifest's path dependencies are relative to that temporary
directory. The harness removes its project fixture on success; its generated
Cargo target is disposable.

## Disposition

Keep F3's current broad `.engine` scan as a local project-bucket observation
until T2 has a replacement aggregate account. F4's per-tree target must not
be described as an aggregate cap. T2 will need producer-owned demand and
lifetimes for F3 staging/admission, F4 materialization, compiler output,
retained candidate and external cache; count known shared allocations once
and still handle real native faults. Concurrent F4 retirement while F3 scans
also needs a safe retry or scheduling proof: current disappearance tolerance
applies to F3 staging paths, not arbitrary F4 directories. No current
mechanism promises all of those properties.
