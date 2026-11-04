# Exact decoded-f64 quaternion and transformed-box probe

3 November 2026. Isolated executable evidence for the candidate
[R0/F3 render-source grammar](../../design/research/R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md).
This implements neither production F3 admission nor F4 transform support. The
candidate retains exact decoded-binary64 source values, the exact rational
quaternion norm tolerance and the affine image of each mesh's tight local
axis-aligned admission box, including unreferenced positions.

## Meaning, independent oracle and refusal witnesses

For XYZW quaternion `q`, let `Q=x²+y²+z²+w²`. The probe requires
`(1−1/10⁹)²≤Q≤(1+1/10⁹)²`, with exact rational constants. The exact rotation
is the right-handed column-vector matrix `R=N/Q`; the Rust code spells out
all nine standard quadratic numerators. For each world axis, the signs of
`N_ij·scale_j` select two local extrema. Six exact supports are compared against
the closed ±`10⁶` m world interval. Zero and negative scales are meaningful;
scale has no additional finite magnitude limit. Empty meshes use the origin.
Translation magnitude ≤`10⁶` and local component magnitude ≤`10³` are upstream
preconditions. The prototype does not perform a mesh-radius or position-domain
check, decode JSON or resolve source edges.

An independent Python `Fraction.from_float` oracle rotates all eight box
corners by Hamilton quaternion multiplication, then compares its extrema to
six independently selected rational supports. Both the always-exact Rust
predicate and its adaptive interval/exact result match **2,655** vectors:
**637** interval-certified and **2,018** exact fallbacks. Fixtures include
attainable world-bound equality and adjacent decoded translation steps on
both sides; neighboring decoded quaternion encodings around each exact norm
tolerance endpoint; quaternion antipodes and exactly scalar-equivalent
encodings; signed/zero scale, subnormals, maximum finite scales, cancellation
and deterministic mixed/ordinary cases. Some arithmetic stress boxes belong
to the component-bounded superset and are not claimed to be complete legal
radius-conforming source meshes.

Exact equality at either quaternion tolerance endpoint is impossible for
finite decoded-f64 components: `Q` is dyadic, while both reduced endpoint
rationals retain a factor of five in their denominator. The oracle brackets
the exact endpoints rather than pretending a rounded threshold is equality.
World-bound equality is attainable and admits. A point displaced by
`(minimum subnormal)²` beyond `10⁶` refuses even though rounded f64 addition
and multiplication erase its displacement.

The conservative rotated-empty-corner witness uses vertices `(1000,0,0)`,
`(0,1000,0)`, `(-1000,0,0)`, `q=(0,0,0.6,0.8)` and `t_y=999000`.
Every actual transformed vertex is within the world extent; the transformed
local-box corner is outside, so the authored **box** rule deliberately
refuses. Another witness includes an unreferenced `(1000,0,0)` outlier:
removing it from the summary wrongly admits the instance. Nine explicit
wrong-rule mutants are killed: strict extent, rounded world sum, vertices
instead of box, omitted unreferenced vertex, omitted scale, dropped scale
sign, omitted Q divisor, transposed rotation and rounded-hypot norm gating.
The rounded-hypot discrepancy includes exact quaternion bit patterns in the
archive.

## Certified filter and separate fixed-width proof

The standalone safe-Rust fallback stores signed magnitudes in **84 u64 limbs**
(5,376 bits, 680 bytes including sign/alignment on this host). This width is
independent of the triangle predicate's 36 limbs. Let `D=2^1074`, so every
finite binary64 is an integer divided by D. Arbitrary finite quaternion
integers have magnitude `<2^2098`; norm evaluation and comparison with the
rational tolerance fit below `2^4258`, including invalid inputs. Invalid
quaternions return before constructing rotation coefficients.

For valid quaternions, `|qI|<2^1075` and conservatively `QI<2^2150`.
Every completed matrix numerator has `|NI|≤QI`: diagonal entries partition
four squares, while off-diagonal entries use `2|ab|≤a²+b²`. Finite scales
have `|sI|<2^2098`, bounded local coordinates `|pI|<2^1084`, and translations
`|tI|<2^1094`. The exact support numerator over positive `QI·D²` is
` tI·QI·D + Σ NI_ij·sI_j·pI_j `. Each local term is below `2^5332`, their
sum below `2^5334`, and translation/extent comparison terms below `2^4318`.
Complete support and signed-sum intermediates stay below `2^5335`, leaving
41 bits within the selected magnitude width. The code checks carry/shift
capacity; the archive retains the complete bound argument. No integer
division or square root is needed.

The fast filter encloses each binary64 operation by one outward step;
positive division encloses reciprocal then multiplication. Nonfinite
intermediate interval ambiguity widens the enclosure and defers. Exact
source values remain authoritative. The argument assumes IEEE
round-to-nearest binary64 with gradual underflow and no fast-math
reassociation. The named optimized Rust build passed these vectors; future
compiler/target qualification remains separate.

## Bounded repeated work on the named host

The regenerable source-shaped **instance-array fragment** has 50,000
instances, **10,800,015 bytes** and **850,002** recursively checked JSON value
nodes, within the common 16 MiB/one-million-value limits. It is not a closed
render record: mesh/material closure, cameras, lights and production parsing
are absent. SHA-256:
`5e32752fccd2d3aecdab91dad9462ca065eed42655c77d4980005e609e2f1798`.

Five fresh optimized Rust processes ran serially on
`Ryans-MacBook-Pro.local`, arm64 Darwin 25.6.0 / T6031,
Rust 1.99.1 / LLVM 23.1.1, `--edition=2021 -C opt-level=3 -C target-cpu=native`.
The oracle used Python 3.15.2. Inner times exclude TSV loading, JSON and
Python work; the three adversarial families each force every predicate to
fallback. They are selected stress families, not a proved universal runtime
maximum.

| Predicate-only family, 50,000 instances | Five-process median | Range |
| --- | ---: | ---: |
| Ordinary clear interval acceptance | 9.23 ms | 8.74–9.59 ms |
| Closed-bound equality, accepted | 1,538.84 ms | 1,487.46–1,573.03 ms |
| Widest exponent-span box, refused | 1,471.36 ms | 1,416.16–1,513.52 ms |
| Subnormal cancellation, accepted | 3,611.02 ms | 3,511.78–3,730.97 ms |
| Unique translations, repeated q/scale, uncached | 1,560.65 ms | 1,504.10–1,613.22 ms |
| Same unique translations, one prepared q/scale entry | 1,066.86 ms | 1,036.71–1,075.79 ms |

The repeated-q/scale experiment prepares Q and nine exact scaled coefficients
once, then varies translation per instance while keeping the equality
fallback. The 6,800-byte prepared entry reduces that selected loop's median
by about **32%**; one-time preparation is outside its timed loop. This is a
bounded reuse opportunity, not a reason to retain one such entry for every
unique instance: 50,000 distinct entries would alone consume 340 MB. Source
keys, cache admission/eviction and charged allocation would need an owner.

Mesh summary reuse separately scans 20,000 local vertices once and reuses its
six f64 extrema for 10,000 instances: median **23.58 µs**, versus **121.59 ms**
for rescanning all vertices per instance. These loops time summary work only,
without transformed predicates. Compact source summaries need 48 value bytes
per mesh, before identity/index/account overhead. The shared-summary algorithm
is `O(vertices + instances)` before exact-arithmetic work.

The cumulative maximum RSS among the five Rust children was **3,637,248
bytes** (3.47 MiB). It includes TSV/cases, predicate scratch and the prepared
entry; the 10.8 MB JSON was **not loaded** in those processes. This is neither
strict allocator charging nor combined F3 peak memory.

The performance consequence is concrete: an ordinary fast-path result does
not bound hostile but valid exact-work latency. This unoptimized fixed-width
prototype spends roughly 1.5–3.6 seconds on selected 50,000-instance fallback
batches before any F3 parsing or closure. Mesh summaries and bounded repeated
transform preparation help, but do not close the unique-transform/adversarial
cost. Production integration needs operation-aware filtering/representation
work, complete charged-allocation and combined admission measurements, and
an operational pressure policy distinct from invalid source. No edit-to-ready
latency target or new source count cap follows from these measurements.

## Retained reproduction and remaining scope

The [selected source, fixtures, width argument and raw samples](../ARCHIVE-ACCESS.md#archive-inventory)
include a deterministic generator, verifier, serial measurement harness and
SHA-256 manifest. The generated JSON and executable are omitted and rebuilt
from source. An extracted archive was regenerated and compiled independently;
the 2,655 vectors and payload hash reproduced. Archive SHA-256:
`79a662ef80b6ead42a4fcdbca2b0449f79610f1999b4083efdf2d9864898208c`.
Fixture TSV SHA-256:
`a6c426b9c2981b4eaaa0a37a45c155a4157fd8065d73bc5b528c9c00a6aec8f4`.

No production crate, source schema registration, admitted-source lease, F4
representation certificate, W1 runtime pose, renderer or device was exercised.
Strict decoder plus identity/edge summaries, near-limit complete-record peak,
old/new pin overlap, true F3 stage/pin latency and operational pressure/retry
behavior remain unmeasured. This probe supplies arithmetic and isolated cost
evidence for that work and makes no maturity promotion.
