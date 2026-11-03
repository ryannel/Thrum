# R0 light/storage arithmetic envelope — candidate, 3 November 2026

This bounds one first-camera, fixed-light R0 proof, as required by
[RENDER-SPEC §§2, 4 and 8](../RENDER-SPEC.md) and the
[render/platform limit review](LIMIT-REVIEW-RENDER-PLATFORM-2026-11-03.md).
It is not a universal source-format cap or a claim that the R0 WGSL graph
exists. The general material closure retains roughness down to 0.045 and
scene-linear light/emission remain semantically unbounded; a *selected
profile* must prove its representation range or use a different storage
scale/provider. GPU diagnostics cannot be inferred from a final nonfinite
test: WGSL can make an overflowing runtime expression indeterminate and may
reassociate arithmetic into another overflowing order.
[WGSL floating-point evaluation](https://gpuweb.github.io/gpuweb/wgsl/#floating-point-evaluation),
[reassociation and fusion](https://gpuweb.github.io/gpuweb/wgsl/#reassociation-and-fusion).

## A source-backed falsifier for fixed pre-exposure

At the selected material minimum roughness `r=0.045`, `α=r²=0.002025`.
At normal incidence `D=1/(πα²)=77,624.7245685`. A black dielectric with
`F0=0.04`, `ω_i=ω_o=n`, under a white Sun of 100,000 lux has direct specular
`L=D·F0·E/4=77,624,724.57 cd/m²`. With the first fixture's `p=1/1024`,
`pL=75,805.40`, above RGBA16Float's largest finite value 65,504. White
metal gives `pL≈1,895,134.88`. These are ordinary finite f32 magnitudes; a
shader can overflow the target without any f32 overflow. Changing roughness
or Sun semantics globally to make that case fit would discard supported
quality. The fixed-p R0 fixture needs a narrower admitted combination or a
separately qualified storage scale.

## Sufficient per-channel recipe inequality

For an admitted material with roughness in `[0,1]`, a direct light with
channel illuminance `E≥0`, `c_i,c_o>0`, and
`g(c)=sqrt(α²+(1−α²)c²)`, combine the height-correlated Smith factor with
the cosine before division:

```text
L_spec = E · D · F · c_i / [2(c_i g(c_o) + c_o g(c_i))]
```

Since `g(c)≥α`, `0≤F≤1` and `D≤1/(πα²)`, its contribution is at most
`E/(2πα³)`. Lambert diffuse is at most `E/π`. This is deliberately
conservative but **proved for every light/view angle**, including grazing;
it is a pre-admission bound, not a BRDF replacement or per-pixel clamp. A
recipe with minimum actual material roughness `r_min`, per-channel upper
illuminances `E_j,c` for its Sun and four locals, per-channel emissive maximum
`M_c`, ambient irradiance maximum `A_c`, and storage scale `p` has the
sufficient ideal-real bound:

```text
α_min = max(0.045, r_min)²
B_c = M_c + A_c/π + Σ_j E_j,c · (1/π + 1/(2π α_min³))
p · B_c ≤ 65504, for c ∈ {R,G,B}
```

The producer validates every source float and cooked texel/factor before
upload, including f64→f32 narrowing. It computes the bound with outward
rounding or a larger explicit numerical margin. The inequality alone does
not establish f32 safety: each shader operation and every permitted
reassociation still needs an intermediate range proof, and f16 packing error
and dim-value loss need measured/analytical tolerances. A failing combination
returns a typed unsupported-profile result *before* GPU preparation; it does
not mutate source identity or silently darken a frame.

The first fixture could, for example, use white 100,000-lux Sun, four local
lights each with scalar photometric incident illuminance at most 1,000 lux,
white
1,000-lux ambient, per-channel emission at most 10,000 cd/m² and actual
material roughness at least 0.32. Even allowing every local to place all of
its unit-luminance colour in blue, the blue-channel local multiplier is at
most `1/0.0722`. The inequality then gives approximately 22,500 stored
blue units, well below 65,504. These are **illustrative fixture inputs**,
not a new SDK-wide lux, emission or roughness limit. The actual admitted R0
source must publish its measured maxima and re-evaluate the inequality.

## Runtime representation and cost path

For the bounded fixture, F4 can cook per-material reflectance/roughness and
emissive maxima; V3 can prepare per-light channel illuminance bounds from
intensity and a certified minimum lit-surface distance. V1/V4 can validate
the resulting interval at profile preparation. The normal frame then needs
no per-fragment storage-range branch or buffer readback. A wider profile may
choose a different `p`, higher-precision storage, or a guarded shader plus a
complete current-frame diagnostic path. The latter must account for status
traffic, extra passes/storage and failed-frame cost; a guard after a dangerous
operation is insufficient. Do not multiply the nine/ten R0 boot pipeline
tuples merely to avoid this bounded preflight.

The first emitter-radius contract currently specifies inverse-square
illuminance *outside* the radius and does not define an inside-radius
operator. A first static fixture can certify all reachable surfaces stay
outside; a general moving-light/geometry recipe needs a selected finite
near-field model or a different admission/refusal boundary. Bounding
`I/r²` also requires bounding the position subtraction, squared distance,
normalization and unit-colour product before division. The current R0
fixture has no saved source geometry, light distances, intensity values or
exposure value from which to certify those quantities yet.

The combined specular identity above is **real-number algebra**, not a WGSL
evaluation recipe. For example, `α=0.1024` and
`c_i=c_o=2^-126` give a denominator near `4.81e-39`: a finite ideal
grazing contribution can become `0/0` if intermediate subnormals flush.
The shader needs a scaled ratio or a proved negligible-angle exit before
forming that denominator. The [independent numeric check](../../evidence/render/R0-ARITHMETIC-BOUND-2026-11-03.md)
confirms the half-range witness and the illustrative real-number inequality;
it does not establish WGSL behaviour.

## Remaining proof before any R0 shader claim

1. Pin the admitted fixture and independently enumerate all reachable
   material/light/pose extrema, including local radius boundaries, normal
   map/tangent validity and action-driven pose changes. Resolve the
   inside-radius light behavior or certify its absence over those states.
2. Select algebra for GGX that avoids separate `G₂/(4c_i c_o)` and avoids a
   zero half-vector; prove all intermediates under WGSL reassociation and
   subnormal behavior, not just real-number output. Compare f32/half output
   with a high-precision oracle at normal, grazing, opposed and boundary
   directions; verify energy/reciprocity and visual highlight shape.
3. Bound exposure, tone denominator, colour encode/decode and FXAA viewport
   arithmetic. The fixed scale `p` and positive exposure alone do not do so.
   Check the largest admitted scene channel, near-black, half normal/subnormal
   boundary and each adjacent outside-domain value.
4. Validate generated WGSL against pinned Naga/wgpu and actual backends,
   then integrate with a current-output diagnostic or pre-admission refusal.
   Record actual shader work and ordinary moving visual quality. No claim
   about an M2/phone/browser target follows from the analytical bound.
