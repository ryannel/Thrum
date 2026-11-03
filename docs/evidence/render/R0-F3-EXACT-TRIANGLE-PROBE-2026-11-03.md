# Exact decoded-f64 triangle admission probe

3 November 2026. This is isolated evidence for the candidate
[R0/F3 render-source grammar](../../design/research/R0-F3-RENDER-SOURCE-GRAMMAR-2026-11-03.md),
not an implemented F3 schema, accepted source record, F4 product or renderer.
The proposed F3 rule is: three distinct mesh indices and at least one
nonzero component of the exact cross product
`(P1−P0)×(P2−P0)` over the decoded binary64 vertex values. Either winding
is valid. F4 separately decides whether a valid source triangle can be
represented and shaded under the selected error envelope.

## Method and independent check

Every finite binary64 coordinate is an exact dyadic `m·2^e`. The standalone
Rust probe shifts the nine coordinates of a triangle to one common exponent,
subtracts and multiplies signed fixed-width integers, and tests all three
cross-component signs exactly. The existing Foundation local-radius bound
puts each mesh coordinate within ±`10^3` m. Even at the minimum subnormal
exponent, a difference is under `2^1085` scaled integer units and a
cross-component magnitude under `2^2171`; 36 little-endian u64 limbs
(2304 bits) are sufficient for this **bounded source regime**. The prototype
asserts on exceeded capacity. Widening the authored geometry bound requires
a fresh width proof.

The fast path uses outward binary64 intervals. Each subtraction and
multiplication rounds its lower result one representable number downward
and upper result one upward; if any cross-component interval excludes zero,
the triangle is certainly nondegenerate. Ambiguous intervals go to exact
integer arithmetic. This argument assumes finite inputs within the bounded
regime, IEEE round-to-nearest operations with gradual underflow, and no
compiler transformation that invalidates the outward steps. The production
path still needs a target/compiler check. This is a narrower cross-component
test, **not** a 3D orient4 predicate or a copy of the published code.
[Shewchuk's adaptive-predicate work](https://www.cs.cmu.edu/~quake/robust.html)
motivates doing only the precision needed for a certified sign;
[CGAL's exact-predicate guidance](https://www.cgal.org/FAQ.html) supports
keeping source topology decisions separate from rounded constructions.

An independent Python `Fraction.from_float` oracle produced 8,009 triples,
including reversed winding, coincident/collinear cases, subnormal area,
near-cancelled products, signed zero, local-bound extrema and deterministic
random mixtures. The Rust exact signs matched all 8,009 three-component
oracle sign vectors. The interval filter certified 4,737 valid triples and
deferred 3,272; it had no false acceptance. A naive rounded-f64 cross-product
sign vector differed from the rational oracle on 3,064 triples and declared
517 exact-nonzero triples zero. In particular, two minimum-subnormal edges
have exact area component `2^-2148` but rounded binary64 multiplication
returns zero. The prototype does not yet exercise a production index parser;
distinct-index checking remains a separate, inexpensive F3 rule.

## Bounded work and measurements

A deterministic JSON **mesh-value payload** contains 39,000 independent
triangles, matching UV pairs and distinct triangle indices. Its byte length
is **16,658,929** (118,287 below the 16 MiB record limit) and its recursively
checked value-node count is **975,004** (24,996 below the one-million-node
limit). Python parsed the complete payload and checked the first/last
indices and decoded pathological vertices. Its SHA-256 is
`37ae2f61936d3ddc8782389d99aa2213bbb66d38db41ee64ff9dd918e6be1568`.
It is schema-shaped mesh content, not a full render record; long decimal
spellings intentionally stress source bytes without changing decoded f64
values. The complete payload is regenerable and omitted from retained evidence.

All 39,000 triangles use a decoded nonzero triple for which the interval
filter refuses and the exact fallback runs. Five independent optimized Rust
processes measured predicate-only times of **26.53–28.58 ms**, median
**27.47 ms**, for that 39,000-triangle batch. A clear-area filter-only batch
of the same size measured **0.70–0.79 ms**, median **0.76 ms**. One longer
3,900,000-triangle run took 2.735 s inside the fallback loop and reported
1,867,776 bytes maximum process RSS via macOS `/usr/bin/time -l`. That RSS
belongs to the streaming fixture reader and predicate executable; the
16.66 MB JSON was **not** loaded into that process. This is an isolated
host measurement on arm64 Darwin 25.6.0 / T6031, Rust 1.99.1, LLVM 23.1.3,
`--edition=2021 -C opt-level=3 -C target-cpu=native`. The Python oracle used
Python 3.15.2.

The selected [source, oracle fixtures and raw results](R0-F3-EXACT-TRIANGLE-PROBE-2026-11-03.tar.gz)
have archive SHA-256 `56976128f6051ee76945d3e7170f8fc37e0c70da06183683d27b511925ce9cff`. The archive includes the
generator and verifier; regenerating the omitted JSON must reproduce the
payload hash above. The fixture TSV SHA-256 is
`377ad1a10b561c95a77d3e40cb2d43642d9a05e7873a1cda28dee0a9e4f91b09`.

## What this does not establish

The experiment does not measure F3's strict JSON decoder, embedded UUID
index/closure, typed mesh storage, old/new pin overlap, F4 reconstruction,
whole-build latency or any device cost. It does not prove the interval code
under all future compilers, targets or altered source envelopes. Before F3
adopts the schema, integrate the predicate with the one versioned typed
decoder, test index validity and refusal paths, measure parser and exact
scratch peak on a complete near-limit record, and repeat the all-fallback
case through the actual admitted-source path. Do not turn the 27 ms isolated
number into a source-admission latency promise.
