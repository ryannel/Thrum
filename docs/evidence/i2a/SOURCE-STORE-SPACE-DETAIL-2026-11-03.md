# Accepted-store space-pressure detail — 3 November 2026

This checkpoint implements the accepted-store portion of the
[phase-specific receipt design](../../design/research/SOURCE-SPACE-RECEIPT-DESIGN-2026-11-03.md).
`SourceStoreError::space_pressure()` returns a fixed-size `StoreSpacePressure`
when a local main target, WAL stop/checkpoint guard or filesystem-availability
check deferred work. `SourceStoreErrorKind::SpacePressure` remains the broad
category. A real SQLite `DiskFull` fault still has that broad kind but has no
preflight receipt; it is an operation fault with the existing exact recovery
rules. No selected threshold, source byte identity, accepted store format or
root/outcome transaction changed.

Candidate preflight distinguishes accounted current main bytes from estimated
marginal main growth, observed WAL length from its stop target, and projected
main/WAL growth plus checkpoint debt from sampled filesystem availability and
selected headroom. The shared WAL guard separately reports a too-large next
allowance before stat, observed WAL after a blocked checkpoint, and live
availability. It can serve pin/maintenance work as well as admission; these
variants do not falsely label every guard call candidate preflight. The
existing hash-reuse and incomplete-blob pricing still own the estimate.

The production availability decisions reuse their existing filesystem
queries. Injected-reader tests assert exact sample values, equality, one byte
short and checked overflow before I/O for both preflight and live checks.
Existing main/WAL refusal, blocked-reader checkpoint, old-root/retry and
`DiskFull` regressions still pass. A public `DiskFull` path reports no typed
estimate, while a public blocked-WAL admission returns one. No additional
normal-path candidate scan, SQLite query, filesystem stat or allocation was
added for the detail.
`RUSTC_WRAPPER= ./dev check` passed after design-hash refresh: repository
checks, Python tests, workspace Clippy and Rust tests, including 204 passing
`thrum-source` unit tests (46 opt-in tests ignored).

These receipts are non-atomic local estimates and observations, never APFS
block reservations or an 8 GiB physical-floor guarantee. Actual filesystem
high-water and ENOSPC timing, cross-process same-volume job coordination,
F4 materialization/compiler demand and full source-to-ready-build cost remain
open. A successful preflight can be superseded before the next write; live
checks, exact root/outcome recovery and retained old pins remain authoritative.
