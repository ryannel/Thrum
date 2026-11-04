# Exact transformed-bound representation optimization

3 November 2026. Isolated follow-up to the retained
[84-limb probe](R0-F3-TRANSFORMED-BOUND-PROBE-2026-11-03.md), using the same
[candidate exact decoded-f64 source meaning](../../design/research/R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md).
No production predicate, source rule, build-plan status or maturity changed.

The selected exponent-normalized dyadic representation reduces the three
original 50,000-instance all-fallback families from contemporaneous baseline
medians of **1,494.59–3,752.71 ms** to **118.57–311.88 ms**, an **8.0–14.0×**
improvement depending on family. The unique-q/scale cancellation family
still costs **319.57 ms**, compared with **3,835.49 ms** baseline, without
retaining a transform cache. These are predicate-only measurements on the
named host, not complete F3 or edit-to-ready latency.

## Meaning and correctness

All variants keep the exact rational quaternion tolerance, full tight local
box including unreferenced positions, signed/zero scale, exact normalized
rotation and closed world extent. The interval filter is unchanged. Every
non-clear timed predicate family asserts 50,000 exact fallbacks.

The unchanged **2,655** archived vectors, including all **nine** wrong-rule
mutants, regenerate with the original fixture SHA-256
`a6c426b9c2981b4eaaa0a37a45c155a4157fd8065d73bc5b528c9c00a6aec8f4`.
A further **933** independent `Fraction.from_float`/Hamilton eight-corner
vectors add 900 deterministic broad-exponent cases and 33 selected samples
from three unique-transform families. All four variants' always-exact and
interval/exact results agree with the oracle on **3,588** cases; prepared
results also agree on all **3,298 valid-quaternion cases**. **1,033** cases
certify through the interval and **2,555** use exact fallback.
The new fixture SHA-256 is
`8a9177d7a0ea1156e971bf7e20752ca6f0e0cd496e43cb6b87240a4f2a04e61e`.
This finite fixture campaign supplements the width argument; it does not
exhaust the decoded-f64 domain or qualify a future compiler/target.

Three algorithmic options were compared with the archived baseline:

- Sparse fixed-denominator multiplication skips zero products and propagates
  each full 128-bit nonzero product exactly. Other arithmetic stays unchanged.
- Exponent-normalized dyadics store an odd integer magnitude, active limb
  count and power-of-two exponent. Exact addition aligns exponents; exact
  multiplication adds them. This removes work on large runs of denominator
  zeros while retaining the same fixed **84 u64 limb** capacity.
- Residual supports combine normalized dyadics with the equivalent comparison
  `sum(N*s*p) <= Q*(extent-t)` and its lower counterpart. Exact extent
  translations produce a zero comparison threshold without rounding.

The archived 5,376-bit capacity argument is retained, not replaced with a
smaller empirical width. Normalization only removes exact powers of two;
alignment never requires more bits than the corresponding original integer
scaled by `D=2^1074` per decoded factor. The same bounds, below `2^4258` for
unrestricted quaternion-norm work and `2^5335` for valid-quaternion support
work, therefore apply. Residual thresholds have at most twice the original
threshold magnitude, still far below the support bound. The archive includes
operation-level details, exponent bookkeeping bounds and checked carry/index
operations. All code uses safe Rust without external libraries or fast math.

## Fresh-process measurements

Five serial fresh optimized processes per variant and family, **200** timed
processes total, ran on `Ryans-MacBook-Pro.local`, arm64 Darwin 25.6.0 / T6031,
Rust 1.99.1 / LLVM 23.1.1 with
`--edition=2021 -C opt-level=3 -C target-cpu=native`. Python was 3.15.2.
Variant order is round-robin within each family/repeat. The inner timings
include streamed Case construction, interval evaluation and black-box
barriers. They exclude TSV/oracle work, compiling, JSON parsing and F3.
Each process measures one family. Values below are five-process medians;
all raw samples and ranges are retained.

| 50,000-instance family | Baseline | Sparse products | Normalized dyadics | Residual supports |
| --- | ---: | ---: | ---: | ---: |
| Clear interval acceptance | 8.56 ms | 8.77 ms | 8.91 ms | 8.59 ms |
| Closed-bound equality | 1661.32 ms | 417.59 ms | 118.57 ms | 135.15 ms |
| Widest exponent span, refusal | 1494.59 ms | 378.56 ms | 186.64 ms | 208.16 ms |
| Subnormal cancellation | 3752.71 ms | 520.07 ms | 311.88 ms | 293.81 ms |
| Unique translations, repeated q/scale | 1641.33 ms | 415.53 ms | 122.12 ms | 139.87 ms |
| Unique scalar q and signed scales | 1604.03 ms | 436.92 ms | 121.29 ms | 139.03 ms |
| Unique tiny q and large scales, refusal | 1564.03 ms | 364.08 ms | 186.20 ms | 221.02 ms |
| Unique tiny q and large scales, cancellation | 3835.49 ms | 549.34 ms | 319.57 ms | 310.14 ms |
| Prepared repeated q/scale, equality | 1032.44 ms | 199.05 ms | 63.45 ms | 73.09 ms |
| Preparation only, repeated q/scale | 623.23 ms | 226.97 ms | 63.17 ms | 66.63 ms |

For normalized dyadics, equality ranges **116.04–123.49 ms**, widest-span
refusal **178.28–230.90 ms**, subnormal cancellation **302.75–332.89 ms**,
and unique cancellation **314.89–324.07 ms**. Baseline cancellation ranges
**3,600.05–4,228.67 ms**. An isolated sparse widest-span sample took
**833.76 ms**, versus its **378.56 ms** median; that outlier is retained.
The residual variant's cancellation medians are slightly lower, but their
ranges overlap the normalized variant. It is slower on equality and widest
span, so these fixtures do not justify selecting it as a universal replacement.
Clear-filter medians differ by less than 0.4 ms per 50,000 instances, with
overlapping ranges; no ordinary-filter speedup is claimed.

The three unique families vary both quaternion encoding and scale for each
instance. Scalar-q identity rotations force exact equality; tiny-q/large-scale
families force wide-span refusal or inward subnormal cancellation. Their
streamed source-shaped instance fragments are **13,376,561**, **15,716,863**
and **16,616,863 bytes**, respectively, with **850,002 JSON value nodes**
each. All fit the common 16 MiB/one-million-node fragment limits. The
cancellation point is within the legal local-radius ball; the widest box is
from the original component-bounded stress superset and is not claimed to
be a radius-conforming mesh. None is a complete closed render record.
Fragment hashes and deterministic reconstruction are retained. Only eleven
selected indices per unique family receive the independent corner oracle;
all 50,000 timed results are classified by the exact predicates, and are
not claimed to have independent per-instance oracle coverage.

## Preparation and memory account

One prepared normalized q/scale entry uses **6,880 bytes**, versus **6,800**
for the baseline: exponent/active-length metadata adds eight bytes per exact
integer. Preparation is outside the `prepared` loop; its **63.45 ms** median
includes interval filtering and all per-instance support arithmetic. A
separate timed loop constructs/discards 50,000 repeated-identity entries in
**63.17 ms** median. This measures that selected preparation workload, not
an upper bound for arbitrary unique transforms. No heap/cache allocation or
lookup/key/eviction cost is hidden inside a claimed cache speedup.

Retaining 50,000 full normalized entries would use **344,000,000 bytes**
(328.06 MiB) before keys, indexing, allocator/account metadata or mesh data.
The baseline equivalent is 340,000,000 bytes. Thus normalization improves
compute cost without making an unbounded per-instance prepared cache
reasonable. A production owner may retain a bounded, charged entry for
proven transform reuse; unique transforms must use transient scratch or a
separately proved compact representation. Shared mesh extrema remain six
f64 values (**48 value bytes**) per mesh, with separate identity/account
cost. This experiment adds no source count cap or validity restriction.

Per-process `wait4` maximum RSS across all timed children was **1,802,240
bytes** (1.72 MiB); normalized children reached **1,753,088 bytes**. Processes
stream one Case at a time and keep fixed predicate scratch plus one prepared
entry. They load neither TSV cases nor JSON. These figures therefore cannot
be compared as a full-memory improvement against the previous TSV-loading
probe, and are neither charged allocation nor combined F3 peak bounds.

## Selected recommendation and retained reproduction

Use the unchanged certified interval filter with the **exponent-normalized
84-limb exact fallback** as the next production predicate candidate. It
materially reduces selected hostile exact work even when every q/scale is
unique. Preserve checked capacity and the existing exact norm/support graph;
review the bounded exponent representation as a private implementation.
Bound prepared-transform reuse by the host's charged work owner and measure
actual key, allocation and eviction cost before enabling it. The sparse and
residual variants remain small retained comparisons; residual support may
be useful after additional workload-directed qualification.

The [selected archive](../ARCHIVE-ACCESS.md#archive-inventory) contains
the exact baseline, standalone source generators and four Rust variants,
independent oracle, selected fixtures, raw samples, width argument and
SHA-256 manifest. Executables and reconstructible JSON are omitted. An
extracted archive regenerated both fixture hashes, rebuilt all four variants
and reproduced all 3,588 oracle matches per variant. Archive SHA-256:
`a87b75baf2e7a93a8a54115b15cb2b6e5605d07fcbb5e201ee174ee7dd16a35f`.

Production F3 dispatch, the shared strict typed decoder, complete record
closure, authenticated pin/F4 handoff, charged allocation, old/new pin overlap,
near-limit combined peak, true stage/pin latency and operational pressure/retry
behavior are outside this experiment. No renderer, target device or integrated
engine workload ran. The observed ~0.32 s selected unique fallback batch is
still meaningful admission work before parsing/closure. It is not a universal
runtime maximum, a selected host budget, or a maturity promotion.
