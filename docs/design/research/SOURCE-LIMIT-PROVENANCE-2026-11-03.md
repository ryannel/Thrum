# Source disk limits: origin, value and current scope — 3 November 2026

This note traces the native source-storage numbers discussed during I2a. It is
an audit of the selected policy, not a new admission contract. The owning
[Foundation specification](../FOUNDATION-SPEC.md#4-working-files-crash-recovery-and-gc)
and [C10](../CONTRACTS.md#c10-persistence-reproducibility-and-authored-operations--p7f3t1)
remain authoritative.

## Origin and correction

The [1 November foundation handoff](../../evidence/handoffs/2026-11-01-engine-decisions/FOUNDATION-CLOSURE.md#4-checkout-mirror-crash-recovery-and-gc)
chose a 64 GiB native operating profile: 52 GiB SQLite main, 2 GiB WAL,
2 GiB managed temporary/cache and 8 GiB unallocated headroom. It required an
operation to reserve projected physical peak and leave 8 GiB actually free.
The handoff supplies the accounting intent and checkpoint behavior; it does not
derive those four values from a named project workload, a device population or
a proved APFS allocation bound. They were selected engineering defaults.

The [2 November policy decision](../../evidence/i2a/SOURCE-PHYSICAL-GUARD-DECISION.md)
and commit `cffdf01` changed those values into **configurable planning targets**.
The current Foundation specification explicitly rejects hard file-size caps or
a guarantee that SQLite or another process cannot consume more space. It
requires early estimates, live observations, backpressure, honest peak reports
and exact recovery from real space faults. The [APFS study](SOURCE-APFS-PHYSICAL-ADMISSION-OPTIONS-2026-11-03.md)
retains the old strict floor as a stronger-tier research question, not the
first-profile admission gate. Its examined Apple/SQLite mechanisms do not
establish a complete copy-on-write, metadata and recovery allocation bound.

## What each kind of rule buys

| Rule | Useful protection | Appropriate status |
| --- | --- | --- |
| Verified immutable bytes and one atomic root/outcome publication | A build sees one complete source revision; retry after an uncertain COMMIT can resolve the exact old-or-new outcome. | Correctness invariant, independent of disk size. |
| Durable pins and complete-closure checks | An old build/reader cannot lose bytes while a new source is published or collected. | Correctness invariant; collection may stay disabled until proved. |
| Finite JSON, tree, blob and transaction work | Rejects inputs that exceed the selected parser, allocation or I/O profile before unbounded work. | Versioned capability bounds; exact values need workload and cost evidence. |
| Main/WAL/temporary targets and checkpoint watermarks | Warns or defers work likely to exhaust the volume; limits normal WAL debt and surprise maintenance stalls. | Configurable operational policy, calibrated by observed peak and named workloads. |
| A guaranteed 8 GiB physically unallocated after every operation | Would protect unrelated volume work if it were enforceable. | Not an honest first-profile guarantee on a shared APFS volume; do not make source readiness depend on proving it. |

The distinction matters in implementation. An estimated peak is a useful
reason to refuse a new edit early, but it is not a reservation of filesystem
blocks. A later `SQLITE_FULL`, `ENOSPC`, sync or checkpoint error must preserve
the previously accepted revision or resolve the exact committed new one. It
must not turn into partial accepted source, silently delete a pin, or report a
failed commit as certainly absent. The user sees observed free space, estimated
and actual peak, the reason for deferral, and a retry path after recovery.

## Limits that still need scrutiny

The current source format admits up to 100,000 tree entries and separately
bounds record bytes, nodes and depth. Its first semantic profile has a
4,096-record candidate. The historical provisional SQLite store capped **all
files** at 4,096: the constant-light bridge's 4,096-record fixture had 4,098
files after project/history and was correctly refused, while 4,094 records
fit. This is a profile mismatch, not evidence that 4,094 is a useful product
limit. The [bridge proof](../../evidence/i2a/E1-CONSTANT-LIGHT-F4-LEASE-BRIDGE-2026-11-03.md)
measures that historical store at its exact checkpoint; current accepted-source
code must be evaluated separately.

Before declaring an admitted profile ready, list every numeric limit with its
owner and unit, the workload or safety model that motivated it, the cost it
bounds, the refusal shown to an author, and whether a larger profile can be
selected without changing source meaning. Align aggregate file counts with
declared record counts; measure complete source-to-build and old/new overlap,
not only a parser or SQLite helper. Keep correctness gates such as exact
publication and retained readers even if a planning threshold is changed.
