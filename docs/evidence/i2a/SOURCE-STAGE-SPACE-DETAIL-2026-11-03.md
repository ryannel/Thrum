# Stage space-pressure detail — 3 November 2026

This checkpoint implements the stage portion of the
[phase-specific receipt design](../../design/research/SOURCE-SPACE-RECEIPT-DESIGN-2026-11-03.md).
It preserves the selected `SourceSpacePlan`, complete-copy stage estimate,
per-write checks, early-stop temporary scan and provisional capture result.
`StageError::space_pressure()` returns a fixed-size `StageSpacePressure` only
when one of those stage-local policy or availability checks refuses. No source
bytes, `SourceRevision`, accepted root or product key include this detail.

The detail distinguishes a lower-bound existing/final managed-temporary scan
from a complete preflight charge plus projected stage copy. Scan charges use
rounded `max(allocated_size, file length)` for files plus metadata allowances;
they are not observed physical allocation totals. It separately names a raw
copy-projection overrun, preflight available-space sample and before/after
write samples. The latter reuse the existing filesystem queries. Availability
is a non-atomic filesystem reading, not an APFS reservation or a guarantee
that later writes retain the requested headroom.

Focused tests exercise an early scan refusal, a complete temporary preflight
refusal, an available-space preflight, before/after-write refusals, raw copy
projection overrun and a final-scan lower bound. The production availability
decision helpers are tested with exact injected samples: equality passes,
one byte short refuses with that sample, and checked overflow happens before
the reader is called. Existing stage tests still preserve an older stage
across capture drift and refusal. No extra normal-path scan, filesystem query
or allocation was added for the detail.
`RUSTC_WRAPPER= ./dev check` passed after the design-hash refresh: repository
checks, 112 Python tests, workspace Clippy and Rust tests, including 201
passing `thrum-source` unit tests (46 opt-in tests ignored).

Accepted-store main/WAL/free-space refusals still expose only their broad
error kind. T2/F4 per-volume composition, actual ENOSPC/high-water trials,
physical allocation behavior and a supported whole-source/build profile
remain open. This stage-local diagnostic does not strengthen the OS space
guarantee or make a refused candidate accepted.
