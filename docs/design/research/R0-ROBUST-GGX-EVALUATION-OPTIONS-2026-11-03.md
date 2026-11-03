# R0 grazing evaluation without a blanket angular floor — research candidate

3 November 2026. This follows the [sharp real-number bound and scaled GGX
candidate](R0-ARITHMETIC-ENVELOPE-2026-11-03.md) and the [independent numeric
oracle](../../evidence/render/R0-GGX-NUMERIC-ORACLE-2026-11-03.md). Two
independent read-only reviews checked the algebra and its implementation
limits. This note chooses the next **experiment**, not WGSL, a source
acceptance rule, or a visual/device qualification.

## Decide what the shader can promise

The present candidate asks every contributing fragment/light pair to have
`max(ci,co)≥2^-16` and the largest component of `wi+wo` at least `2^-12`.
Those floors have no source fixture or moving-pose certificate. A runtime
evaluator could cover more grazing content, but the promise must distinguish
the authored real directions from the actual represented shader operands.
Two authored directions with opposite hemisphere signs can narrow to the
same f32 bits. A deterministic shader given those bits cannot recover which
one was authored. The existing sharp grazing family proves a tiny positive
cosine can still be bright, so treating ambiguity as zero or replacing it
with an epsilon is a material change.

The first experiment should target **finite, represented f32 inputs** with
documented upstream transform/interpolation error. Source-real conformance
still needs an authored-to-represented error certificate, a different
encoding, or an explicit unsupported/diagnostic outcome. This is a proposed
scope for proof, not a change to C08 material semantics. WGSL permits
flush-to-zero for some floating operations and even intermediate values in
bit reinterpretation; it also permits reassociation/fusion and does not
promise the rounding behavior required by a conventional IEEE-only error
filter. See the current [WGSL floating-point rules](https://www.w3.org/TR/WGSL/#floating-point-evaluation),
[reassociation](https://www.w3.org/TR/WGSL/#reassociation-and-fusion) and
[accuracy requirements](https://www.w3.org/TR/WGSL/#floating-point-accuracy).

## Fast represented-input candidate

Keep the cosine combined with height-correlated Smith. This candidate inherits
the selected material domain, including positive minimum roughness and
`F0∈[0,1]`; finite represented vectors alone do not establish admission.
For certified `ci,co>0`, let `alpha=roughness²`, `a=alpha²` and
`g(c)=sqrt(a+(1-a)c²)`. The selected real closure's factor is
`K=ci/[2*(ci*g(co)+co*g(ci))]`. Divide by only the larger cosine:

```text
if ci >= co:
    q = co / ci
    K = 0.5 / max(alpha, g(co) + q*g(ci))
else:
    q = ci / co
    K = (0.5*q) / max(alpha, q*g(co) + g(ci))
Lspec = E * D * F * K
```

The two branches are algebraically identical for positive real cosines.
Each ratio is at most one; each unguarded denominator is at least `alpha`,
so the `max` does not change the real closure. This saves one ratio division
relative to the earlier two-ratio candidate. It does **not** permit division
when computed cosines are zero or ambiguous. An ordinary branch must first
certify its signs and the positive divisor. A fast interval filter can
certify dots clearly away from zero; near-zero dots go to a robust path or
cause a complete current-frame diagnostic. Its interval includes input
quantization, products, additions, permitted reassociation/fusion and FTZ.
An exact sign alone is insufficient if a positive magnitude or ratio has
flushed away. A second-branch ratio that rounds to zero needs an absolute
radiance/display error bound before it can be dropped.

### A conservative signed-dot filter to challenge

For the explicit scalar expression
`d=x0*y0+x1*y1+x2*y2`, fix all six **already represented** f32 operands
as normal-or-zero values in `[-1,1]`. Let `D` be their exact real dot,
`N=2^-126` the smallest normal f32, and allow either adjacent rounding,
all groupings/orders of these three products and two sums, fusion, and
permitted FTZ. A conservative local-error account is:

| Effective node | Endpoint bound | Directed rounding plus FTZ error |
| --- | ---: | ---: |
| Three products | `|p|≤1` each | `2^-24+N` each |
| First pair sum | `|s|≤2` | `2^-23+N` |
| Final sum | `|d|≤3` | `2^-22+N` |

The endpoints are representable, so each adjacent rounding and flush stays
inside its row's magnitude bound. A child output flush and its parent's
input flush can be counted as one effective flush on that single-use edge.
The five nodes therefore give
`|d−D| ≤ E = 9·2^-24 + 5N < 2^-20`.
The proposed filter certifies exact represented sign only when
`d>2^-20` or `d<−2^-20`; every other value, including computed zero,
remains uncertain. `2^-20` is a simple conservative threshold, **not** a
minimum authored/view angle. In particular, rounding can give a nonzero
computed dot for an exact zero, or the opposite sign for a tiny positive
dot. `x=(a,-1,-1), y=(a,b,2^-47)` with `a=1−2^-23` and
`b=1−2^-22` has exact `D=2^-47`; a permitted downward rounding of `a²`
gives computed `d=−2^-47` in left-to-right evaluation. A positive `2^-127`
product from two normal inputs can flush to zero.

This is a **scoped mathematical filter candidate**, independently checked
against the current [WGSL accuracy](https://www.w3.org/TR/WGSL/#floating-point-accuracy),
[FTZ](https://www.w3.org/TR/WGSL/#floating-point-evaluation) and
[reassociation/fusion](https://www.w3.org/TR/WGSL/#reassociation-and-fusion)
rules. WGSL's `dot` accuracy is inherited from a component-product sum;
applying this bound to a conforming builtin is a specification-derived
consequence, not an observation about a native instruction. Alternate
implementations do not receive a larger error allowance than the inherited
accuracy contract. Backend validation and a selected shader expression are
still required before a portable R0 shader claim. The
bound excludes all upstream source, transform, interpolation and normal-map
error. If represented operands differ from authored operands by certified
component errors `|δx_i|≤a_i`, `|δy_i|≤b_i`, add a separately proved
`U≥Σ_i(|x_i|b_i+|y_i|a_i+a_i b_i)` to the threshold with outward rounding.
No such source certificate exists yet. This constant filter adds two
comparisons/branches but can send many bright grazing fragments to the
expensive path; fallback occupancy and full five-light cost are unmeasured.
The [independent finite-operation oracle](../../evidence/render/R0-SIGNED-DOT-FILTER-ORACLE-2026-11-03.md)
checks selected adjacent-rounding, FTZ and fusion schedules against exact
represented dots; it does not establish a WGSL/backend implementation.
An adversarial review identified a further gate: the `2^-20` filter certifies
**sign only**, not usable cosine magnitude. At a computed dot just above the
threshold, `E` is about 0.536 times that computed value; the exact represented
dot may be much smaller. Feeding the computed value directly into the Smith
ratio can therefore have large relative error even on the admitted side.
The fast lane needs a separate output-error certificate or an accuracy-triggered
fallback; sign certification alone cannot select the lighting value.

The existing max-component normalization of `v=wi+wo` can avoid a tiny
denominator without the arbitrary `2^-12` threshold when `v` has a
certifiably nonzero represented component. For an exactly unit `n` and
positive exact cosines,
`max(ci,co) ≤ ci+co = n·v ≤ ||n||₁ max(|v.x|,|v.y|,|v.z|) ≤ √3 hm`.
Thus a certified angular margin implies an ideal half-vector magnitude
margin. For represented values, use an actual bound on `||n||₁` and include
dot/addition error; the real inequality does not certify the computed `hm`.
Near opposition, directional error grows roughly with `1/|v|`, even if the
scaled normalization remains finite. The candidate shader formulation and
its moving-highlight quality must be checked against an independent oracle.

Filament documents the same [GGX distribution and height-correlated Smith
visibility family](https://google.github.io/filament/main/filament.html#materialsystem/specularbrdf).
Its implementation shortcuts, including absolute-value hemisphere handling
or fp16 saturation, do not select Thrum's outside-regime semantics. The
optional `|cross(n,h)|² + alpha²(n·h)²` distribution denominator avoids a
`1-(n·h)²` cancellation for unit inputs, but adds vector work; compare its
error and full five-light cost before adoption.

## Rare exact fallback candidate

For represented **normal-or-zero** f32 input components in `[-1,1]`, an
integer dyadic path could decode sign, exponent and significand and compute
the sign of a three-product dot with multiword `u32` arithmetic. Even normal
f32 components can have a `2^-149` integer lattice after exponent alignment;
product cancellation can expose a `2^-298` difference. Three products
bounded by one therefore need roughly 301 signed fixed-point bits, so ten
`u32` limbs plus proved product/carry logic are a conservative first budget,
not an optimized design. For example, with `u=2^-149` and `M=2^23`,
`((M+1)u)^2 - (Mu)((M+2)u)=u²`; every operand is a normal f32 value.
Return a normalized mantissa/exponent magnitude as well as a sign; this
allows a positive but subnormal-sized dot or ratio to be carried without
first materializing a flushed f32. A similar scaled integer sum could
support a near-opposed half vector. This adapts the filtered-predicate
principle in [Shewchuk's primary paper and code](https://www.cs.cmu.edu/~quake/robust.html),
not its floating implementation or its assumptions verbatim.

For normal-or-zero components, a concrete exact-dot diagnostic can use one
ten-`u32` signed accumulator on a `2^-298` lattice: decode each component as
`±M·2^(e−150)`, form each 48-bit significand product, and shift it by
`e_x+e_y−2`. The sum needs at most 301 signed bits for this operand range;
320 bits leave room for carries. This is an implementor candidate to test against an
independent integer oracle, not a validated WGSL implementation. The
fallback must keep exponent and significant bits through division; converting
a positive `2^-127` cosine back to ordinary f32 before division can erase a
finite bright grazing limit. Exact dot sign/magnitude also does not repair a
rounded or flushed `wi+wo`: near-opposition recovery must start from the
original represented operands. Unit-vector and `D`/Fresnel accuracy need
their own certificates.

This fallback is an unbuilt, potentially expensive candidate. WGSL permits
FTZ in bit-reinterpretation intermediates, so a float-to-integer bitcast is
not a universal recovery path for subnormal **input** components. Supporting
those inputs would need an upstream integer encoding or a separately proved
representation path; the accumulator above already covers their exponent
lattice if their bits are reliably available.
The [raw-`u32` Metal diagnostic](../../evidence/render/R0-EXACT-REPRESENTED-DOT-METAL-2026-11-03.md)
now matches an independent exact oracle for 189,803 selected records and
returns magnitude without a tiny-float conversion. Its input interface
bypasses float-to-bit recovery, and its isolated cost includes large
diagnostic writes; it does not select a fragment fallback or prove general
WGSL/backend performance.
The [single-accumulator optimization diagnostic](../../evidence/render/R0-EXACT-DOT-OPT-METAL-2026-11-03.md)
keeps that exact output and cuts the isolated 189,803-record full-output pass
from about 1.04–1.06 ms to 0.20–0.23 ms on the tested Metal device. Its direct
10.37-million-dot compact-output control still took 20.5–33.9 ms and had
substantial scaling noise. It selects a better *research kernel*, not a
common-fragment fallback or frame-cost claim.
It also cannot recover source-real signs or relative direction detail lost
before the represented operands arrived. Count fallback frequency, divergent
waves, live registers and latency in ordinary and deliberately tangent
five-light content; a rare-branch assumption is not a performance result.

## Narrow source specialization and proof order

A static planar face with constant normal has
`sign(n·(camera-P)) = sign(n·camera-d)` and similarly for a point light on
the plane `n·P=d`; direction normalization preserves the sign. A CPU
certificate per face/light/view update could avoid most fragment tests in
such a fixture. A normal cone extends this only with bounded normal-map,
tangent, interpolation and transform error. It is a specialized proof path,
not a general content restriction.
The [independent planar source-sign oracle](../../evidence/render/R0-PLANAR-SOURCE-SIGN-ORACLE-2026-11-03.md)
now checks exact decoded-binary64 plane/Sun/camera signs for one static,
identity-transform, flat-normal fixture. It also exhibits opposite source
signs that narrow to identical f32 direction words. Its proof-local F4
direction-packing bound is a separate handoff and cannot certify the shader's
normal, magnitude or general mapped/interpolated material path.

1. Extend the independent CPU oracle with both one-ratio Smith branches,
   exact dyadic represented-dot predicates and uncertainty intervals. Include
   the sharp grazing family, opposite-sign values that narrow identically,
   exact-zero/cancellation cases, half boundaries and every claimed FTZ mode.
2. Decide and record the source-to-representation error contract before
   treating represented-input success as source correctness. A filter needs a
   sound error interval under WGSL's allowed operations, not just a measured
   small error on ordinary pixels.
3. Prove each shader intermediate finite and each branch divisor nonzero
   under the selected profile, including permitted reassociation. Validate
   pinned Naga/wgpu/backends; measure fast-path cost, fallback occupancy,
   register pressure, and a deliberately all-ambiguous workload.
4. Compare moving grazing highlights, half packing, tone/exposure output and
   complete current-frame diagnostic behavior. Keep mathematical equivalence,
   source correctness, experienced visual quality and device cost separate.

No angular early exit, source-wide certificate removal, complete fallback,
physical shader safety or device performance is claimed by this note.
The [independent offline one-ratio and represented-dot
oracle](../../evidence/render/R0-ONE-RATIO-ORACLE-2026-11-03.md) checks the
candidate algebra and information-loss witnesses on bounded CPU fixtures.
It does not close the WGSL, source or device gates above.
