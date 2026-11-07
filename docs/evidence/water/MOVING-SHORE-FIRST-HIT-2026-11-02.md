# Moving-shore rain first-hit: exact offline proof — 2 November 2026

This is a source-receipt proof for the [moving-shore exposure design](../../design/research/MOVING-SHORE-EXPOSURE-DESIGN-2026-11-02.md),
not a finite-water, film/coating, C09 runtime, image or device qualification.
The selected proof package ID is `thrum.e1.e2.moving-shore.first-hit-proof.v1`.
The [reproducible six-file package](../ARCHIVE-ACCESS.md#archive-inventory)
is 70,716 bytes, SHA-256
`8b680a692e5456087ed0725e8cc7ada6da3d56aa3a7c4c9e54082ee84b5be4be`.
Every archived file was compared byte-for-byte with the selected source, fixture,
result and manifest after the final replay. `proof.py` is original
dependency-free Python; the archive records exact Python 3.15.2 identity and
per-file hashes. Run `python3 -B proof.py` after extraction into an owned task
directory. Results SHA-256 is
`1dc4dba04e1ec5ab220d77c4ed6aada198e42eeae3aec7d63235c1d68d7023f0`.

## Result that changes the operator choice

An exact-rational prepared-atom, frozen-start candidate was compared with an
independently derived analytic first-hit area-time oracle. Ten valid fixtures
cover a planar rising/falling shore, identical endpoint wet masks with an
interior wet pulse, a source-rate knot/off state, roof/bridge/export ownership,
a zero-area tie, ordinary and fast 60 Hz intervals, and a separately identified
triangular response footprint. Two covers below/intersecting the bank refuse
before either candidate or oracle runs. Each receiver and each reporting window
is checked; matching total source mass alone does not pass.

Under the proposed `1e-5 kg + 0.1%` local receipt-error request, a representative
60 Hz shore advancing 0.1 mm at 10 mm/h rain is certified with one frozen
slice. A stress tick sweeping 0.1 m in 1/60 s at 100 mm/h needs 128 frozen
slices to certify the bound and **fails** the selected 16-slice profile. The
one- and two-second full-bank sweeps are stress/control intervals, not 60 Hz
workloads; their first certified tested counts are 512–2048. Identical endpoint
masks without the interior stage knot assign zero direct water rain where the
oracle assigns `1/180 kg`, despite a false endpoint-only swept bound of zero.
The correct interior envelope exposes the error.

For this restricted **piecewise-linear scalar stage, affine terrain/cover and
piecewise-linear nonnegative rain** regime, wet area of each triangle is
piecewise quadratic between vertex-height crossings. Two alternative exact
source integrators split at stage, source and vertex crossings: one uses a
rational cubic 3/8 integral, the other prepares quadratic area coefficients and
integrates their product with linear rain. Both match the independent oracle
exactly for every receiver/window in all ten fixtures. A triangular fixture
contains two nonzero quadratic area terms and eight nonzero cubic integrand
terms; deleting the quadratic terms fails its independent local oracle. This
removes the frozen-start **rain-receipt** bias in the tested scalar-stage regime
without raising the physical update cap. It says nothing about how many
water/film/coating state updates are needed across an onset or retreat.

In the fast/heavy fixture, frozen N=16 executes 34 triangle clips/102 edge
tests yet fails the local bound; the prepared coefficient path has one algebraic
piece and four receiver coefficient integrations with zero runtime clipping.
The roof/bridge/export case has 12 algebraic pieces and 84 coefficient
integrations. These are exact algorithm counters in a tiny rational fixture,
not CPU timings or a guaranteed engine speedup. Offline coefficient preparation
still clips triangles; a production f64 coefficient/error certificate, bounded
piece workload and target-device cost remain open.

The toy receipt ledger now keys delivery by source revision, stream and
**physical interval**, independent of numerical subdivision and tolerance.
Equivalent N=1→2 retries deduplicate, while different admissible aggregated
receiver masses conflict without mutation. An over-budget or over-cap candidate
refuses before the toy commit, and a valid dependency retry commits once.
This is sequential in-memory evidence, not persistence/concurrency proof.

An independent numerical reviewer first found an unchecked cover-first
assumption, absent nonzero quadratic coverage, a bypassed numerical retry gate
and undercounted source work. A second pass found the partition-dependent
receipt ID. All five findings were corrected and a final read-only recheck
reported no remaining issue within the offline scope. The exact proof was
replayed from the retained source with byte-identical result and manifest
hashes. The governing tide-only first-hit design must now choose the analytic
receipt operator and keep film/water timing, f64 error and full C09 qualification
explicitly open.
