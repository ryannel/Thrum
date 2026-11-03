# Marginal source-space preflight — 3 November 2026

This checkpoint narrows the first source planning heuristic. It preserves the
configurable 52/2/2/8 GiB defaults as policy, the accepted root/outcome/pin
protocol, live checks and real SQLite/OS error handling. It does not claim a
physical allocation bound or a coordinated same-volume reservation.

## Phase ownership

Standalone provisional capture knows the declared tree sizes but has no
authenticated staged hashes or accepted-store connection. It now charges the
full rounded staging copy, existing managed temporary use and observed free
headroom. Main-file size and WAL stop are not new staging allocations, so they
cannot independently refuse this copy. The writer lock, unsafe-path scan,
checked arithmetic, per-write space probes and final temporary scan remain.

Accepted admission has the staged hashes. It deduplicates equal hashes,
charges missing or incomplete blobs plus metadata against observed retained
main size, and keeps existing uncheckpointed main growth as separate debt.
Projected new WAL growth is `min(2 × projected main growth, remaining WAL
window)` with checked arithmetic. A larger unused stop does not raise that
work estimate when neither window is reached. The WAL stop and per-transaction
allowances still act as live backpressure; a successful estimate does not
guarantee later allocation or publication. Incomplete blobs are conservatively
priced as full restages even when a retry can reuse a verified prefix.

These two preflights cannot be added blindly as a proved physical peak. A
future integrated authoring operation must compose stage, admission,
materialization/compiler and concurrent job claims on the actual volume,
then report estimate class and observed high-water separately.

## Proof and limits

Focused tests show a tiny stage succeeds beside a source main file and WAL
larger than artificially low capture targets, while the temporary-target
refusal remains. A narrow and a very wide unused WAL stop yield the same
staging projection. The accepted preflight accepts a tiny candidate with a
one-TiB unused stop and has deterministic equality, capped-window and
overflow cases for its WAL-work calculation. Existing tests cover accepted
main/WAL/free-space refusals before any blob write, once-only hash reuse,
checkpoint debt, pinned readers, `SQLITE_FULL` and exact root/outcome recovery.
`RUSTC_WRAPPER= ./dev check` passed after design-hash refresh: repository
checks, 107 Python tests, workspace Clippy and Rust tests, including 191
passing `thrum-source` unit tests (46 opt-in tests ignored).

This is a logical cost correction, not a measured performance or peak result.
No claim is made for a calibrated WAL write multiplier, partial-prefix work
projection, large master or many-file end-to-end cost, real `ENOSPC`, OS
allocation behavior, diagnostic receipt or concurrent build reservation.
