# First terrain final-triangle certificate — 3 November 2026

**Bounded offline proof; no engine, visual or device qualification.** This implements
one falsifiable geometry slice of the [terrain implementor recipe](../../design/research/TERRAIN-IMPLEMENTOR-DOGFOOD-2026-11-02.md)
under [C05](../../design/CONTRACTS.md#c05-assets-streaming-and-retirement--f4-with-w2-v1-and-p7),
[C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4)
and [C09](../../design/CONTRACTS.md#c09-environmental-fields-and-physical-exchange--e1-with-p3).
It extends, and preserves, the earlier [small exact-overlay falsifiers](T1-EXACT-OVERLAY-2026-11-03.md).
The source is a synthetic 64 m × 64 m heightfield at 0.25 m spacing: 257² integer
samples, 131,072 source triangles with the fixed SW–NE diagonal. Source coordinates
and heights use 0.0001 m integer units; the certificate is relative to these admitted
discrete values, not a surveyed or imagined smooth terrain. Source bytes are proof
inputs, not an F3-admitted source revision.

The selected [ten-member archive](T1-FINAL-TRIANGLE-CERTIFICATE-PROOF-2026-11-03.tar.gz)
is 312,104 bytes, SHA-256 `e113364c531c8891a94e703fff5aead294d243d985946e6f3342d9c80f954cc0`.
Its member manifest SHA-256 is `1c362448e5303a94ac9369dfdb787461a8b591ca447cc739691ebe358f931405`.
It contains separate candidate producer, independent checker, mutation runner,
serialized source/final product/neighbour record, binding, README and selected results.
The checker imports no producer code. Verify member hashes, then run `python3 checker.py`
in the extracted `terrain-certificate` directory to check the archived bytes before
regeneration. `python3 run.py` rebuilds the inputs and runs all mutations; measurements
in regenerated `results.json` will differ. All archive members were byte-verified after
packaging. A fresh extraction passed all ten member checks and reran the archived
checker successfully before regeneration. The six local evidence links/anchors passed
the repository link checker. The full repository guard was attempted; an unrelated
then-in-progress `SOURCE-REQUESTED-IO-FENCE` archive link prevented a clean run.
No third-party assets or Python packages are used.

The final product has 39,570 triangles over aligned
2×2 source-cell blocks. Boundary strips, the shelf/sill and a diagnostic saddle remain
fine. Coarse/fine interfaces use a centre fan containing each required edge midpoint.
The product retains all 66,049 f32 XYZ vertices, including unused samples; it reduces
indices, with no claimed vertex-memory saving. Each triangle is checked in its final
serialized triangulation. Positive orientation, exact per-block nonoverlap/support,
source coverage and global oriented manifold-edge/boundary checks reject omitted
faces, duplicate faces and T-junctions. Ten distinct exact XY overlay templates are
reused only for geometry; all actual block heights are evaluated. This is bounded
aligned-grid specialization, not a generic remeshing certificate.

| Achieved value | Exact result |
| --- | ---: |
| Unrounded final triangulation versus integer source | 0 m maximum absolute error |
| Actual final f32 surface minus integer source | [-1/20971520, 1/20971520] m |
| Pure f32 height rounding after enforced encoding | 1/20971520 m maximum |
| Protected shelf/sill surface error | 0 m |
| Planar-bank horizontal shore bound | 1/1048576 m |
| Shared seam | 257 exact coincident world vertices; 256 matching quarter-metre segments |

The unrestricted diagnostic first reports total final-vertex displacement, separate
from pure rounding. Pure output rounding is reported only after an independent exact
rational nearest-even binary32 oracle verifies raw Y bits at **every** source-attached
vertex, including unused vertices. The oracle searches ordered IEEE754 encodings,
compares exact distances and applies even-significand ties; manual lower/upper midpoint,
negative-symmetry and exact-crest controls also passed. All XZ coordinates are exact
quarter-metre lattice values. The main overlay reads actual final f32 numbers as exact
dyadic rationals, so conversion is already included in its signed error, not added a
second time. A 1 mm wrong-height mutation reaches the encoding refusal after passing
the 5 mm geometric gate. Contradictory units, source diagonal or water-provider
metadata also refuse explicitly.

The requested fixture vertical limit is 1/200 m (5 mm), separate from the achieved
1/20,971,520 m bound (about 4.77×10⁻⁸ m). The shoreline limit is 1/10 m. These are
local proof thresholds from the proposed recipe, not global engine guarantees.
One neighbour boundary uses local Z=0 with exact world datum Z=64; the checker compares
actual world coordinates and hashes. There is no second full region or halo closure
in this proof. The outer domain has 1,024 boundary segments and 59,867
unique manifold/boundary edges; skirts are absent.

The bank support is the full-width 64 m × 1 m strip Z∈[0,1]. Independent source
vertex checks prove `b=x/20`; affine source triangles therefore reproduce the plane.
For H∈[−1,1/2], analytic source shoreline, area and storage geometry are
`x=20H`, `A=20H+32` m² and `V=10(H+8/5)²` m³. Exact clipping/integration equals those
formulas at H=−1, 0, 1/3 and 1/2. Actual final-product contour segments cover every Z
in the strip; segment endpoints give the true maximum horizontal error at each
witness level. At H=1/3, source X=20/3 and the final contour's exact maximum deviation
is 1/5,033,166 m. The all-stage bound is 20e for exact bank vertical error e; the
source plane brackets every admitted level, and water/coordinate residuals are zero.
For every admitted H, wet-area error is at most 40e m² and storage-geometry error at
most 64e m³, from the uncertainty strip and the 1-Lipschitz depth function. Actual
final area and volume witnesses satisfy those bounds; full rational results are
retained. The shoreline claim does not extend the planar formula across the shelf.

The protected shelf is an 8 m × 8 m ROI with grid X∈[96,128], Z∈[112,144]: a 0 m
floor, 0.25 m outer rim and a 0.25 m-wide crest at exactly 0.125 m. Two source-location
seeds identify the west and east basins. Boundary is the edge of this ROI; exterior
connectivity is outside support. Wet means **b < H**, including at equality.
The complete 2,048 source/final XYZ faces coincide exactly on this ROI. Consequently
wet sets and area/volume curves are identical at every H; no derivative-root or
polynomial approximation argument is needed for this identity case. Generic changed-sill
curve certification remains open. The graph also checks every critical equality and
one representative of each adjacent open interval, whose topology is constant between
PL vertex levels 0, 1/8 and 1/4 m:

| H (m) | Wet components | West/east seeds connected | Wet area (m²) | Storage geometry (m³) |
| ---: | ---: | :---: | ---: | ---: |
| 0 | 0 | no | 0 | 0 |
| 1/16 | 2 | no | 869/16 | 10073/3072 |
| 1/8 | 2 | no | 929/16 | 5215/768 |
| 3/16 | 1 | yes | 3963/64 | 16279/1536 |
| 1/4 | 1 | yes | 1023/16 | 465/32 |

Negative/equality-outside samples −1/16 and 5/16 also pass. Lowering the two crest
columns to zero changes the H=1/16 graph from two components to one and connects the
seeds; wet area becomes 14883/256 m² and storage geometry 14641/4096 m³. The mutated
product's exact vertical error is 1/8 m. This is geometry of a prescribed stage,
not flood-arrival, conservation or solver qualification.

The runner restores the selected product after every mutation and rebinds hashes
for geometric mutations, so hash failure cannot substitute for a geometry rejection.
The wrong diagonal preserves all vertex values yet produces an exact +1/8 m error.
Duplicate replacement keeps triangle count unchanged and is rejected for overlap.

| Deliberate mutation | Observed result |
| --- | --- |
| wrong-diagonal | rejected: `vertical-error-budget` |
| omitted-triangle | rejected: `support-area` |
| duplicate-triangle | rejected: `overlapping-triangles` |
| decimated-sill | rejected: `vertical-error-budget` |
| shore-bias | rejected: `bank-shore-budget` |
| seam-displacement | rejected: `seam-world-position` |
| wrong-height-under-geometric-budget | rejected: `source-attached-f32-encoding` |
| off-grid-position | certificate-unavailable: `off-grid-XZ-conversion` |
| source-hash-mismatch | rejected: `source-hash-mismatch` |
| product-hash-mismatch | rejected: `product-hash-mismatch` |
| source-unit-binding-mismatch | rejected: `source-unit-metadata` |
| source-diagonal-binding-mismatch | rejected: `source-diagonal-metadata` |
| water-profile-binding-mismatch | rejected: `water-profile-metadata` |
| nine-triangles-in-block | rejected: `triangle-count-per-block` |

Work is bounded by fixed profile limits: exactly 16,384 aligned blocks, at most
eight candidate triangles per block and 131,072 in total, 256-bit error fractions,
and at most 2,000,000 overlay vertex evaluations. Source/product/neighbour/binding
input byte caps are 264,224 / 2,400,000 / 3,084 / 4,096. The i32 source heights are
restricted to ±2 m; XZ must decode to the exact quarter-metre grid. Binding changes
cannot raise limits. Unsupported off-grid positions return `certificate-unavailable`;
work/bit uncertainty cannot become a rounded pass. Profile/capacity, support, topology,
budget, hash, semantic or encoding violations reject. Lattice orientation/intersection
predicates have small bounded integer inputs; the bit check covers each rational
height error. Missing/malformed files fail the checker rather than supplying defaults.

The selected **single local process run**, without machine isolation or comparative
performance controls, measured 0.038 s production,
7.715 s positive certification and
61.112 s for the complete mutation campaign. The OS whole-process
peak RSS was 106,070,016 bytes (101.16 MiB),
covering Python objects, retained blobs, transient geometry and mutation/observer work.
This is separate from the packed logical resident payload: 264,196 B i32 source heights,
792,588 B f32 XYZ, 474,840 B u32 final indices and 3,084 B neighbour coordinates,
total **1,534,708 B**. Another 1,534,748 B of serialized source/product/neighbour inputs
coexists with decoded Python objects. These are logical byte accounts, not measured
allocator usage; dictionaries, Fraction objects, edge maps and old/new mutation buffers
are additional. No hard RSS admission gate is implemented.

The positive certificate evaluates 615,564 actual overlay vertices
and 1,846,692 source/final/nominal exact height dot products,
with 561 exact template polygon overlays, 118,710
directed edge visits and 66,049 raw height-encoding checks. Reference triangles are
constructed per 2×2 block; no global source×product triangle-pair search occurs.
The run used 3.15.2 on `macOS-26.6.2-arm64-arm-64bit-Mach-O`.
No startup, runtime, renderer, device, thermal, contact, navigation, quality or
cross-platform performance conclusion follows from these offline measurements.

Exact selected member identities are below; the archive member manifest supplies the
same values and covers the source/producer/checker separation.

| Member | Bytes | SHA-256 |
| --- | ---: | --- |
| `README.md` | 4958 | `5590f8560080d2c0a11281913c0136c92db530f45db398129a503d854aaf5768` |
| `binding.json` | 753 | `dfd9004082b0d561bf2204780e4c6bb77de851ea4d00cc13c6bfbd4523514262` |
| `checker.py` | 20822 | `5d95bdfb2e76f2aa0d7c12c51870d997fce54db5495885ea007920b9b858ced7` |
| `neighbour.bin` | 3084 | `7d9a0f5eb41fa2f86714618b5514e4936359f85e915b3c88e1c0c33fd7859970` |
| `producer.py` | 3845 | `0f820180f16b3168558e8d2b6d5f34f0655980a4da6a75e9e2537267eeeb72e7` |
| `product.bin` | 1267440 | `0147f886538db4e6d51733e8a9a3f73e48c04e10fb962a47cf20fe11f16df7a5` |
| `results.json` | 9929 | `9f7e1501c7ae81b05eca9ea779d5d3d8d668132ebd77b197486a74147b646f0c` |
| `run.py` | 5935 | `d4d232736a278dca3e30a5d074a2b935f134db03442064e101cd6ea022fc5f82` |
| `source.bin` | 264224 | `76493b2930052eb39f96d90595555601075987fd89518d027977cb112f7a7ebb` |

The known open work remains: full B/halo preparation; actual F3/F4 source-tree/build
binding and activation; patches/material provenance; roofs/decks/first-hit rain;
Rapier/Recast consumers and clearance; generic remeshing/topology/changed-sill bounds;
physical water and response state; runtime visuals, ordinary motion and named-device
memory/performance. This certificate does not amend canonical contracts, the build
plan or maturity status, and does not replace the true discrete source authority.
