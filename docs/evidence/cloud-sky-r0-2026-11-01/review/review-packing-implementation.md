# Independent packed-field implementation review

1 November 2026. Read scalar-field.js, packing-proof.js, gpu.js, the actual independent Python reference/tests,
canonical packed-source rules and the initial packing receipt. No GPU run, renderer/integration edit or commit
was performed by this reviewer. After the first review pass, root explicitly commissioned one additional CPU
fixture in my two owned reference files; those changes and their test scope are recorded below.

Inspected source identities (SHA-256):

- scalar-field.js: `8ea5da6077d9c81c252dd860cc4a9ed1818f591257d9f6b8fd7d08e11de95f10`
- packing-proof.js: `32cc32cb2780e3f771612fc49491cfa5c5d4aa9e001324e4336c1c236776141c`
- gpu.js: `b6042156ccf0e7fa7fc702d1191075cf9db72c65b2da431f69a557115da67c55`
- revised reference_packing.py: `c7a6f2ea0e1a31a4af4882eed3c265df9b54ff05976b2a9fc12797b2b3560185`
- revised test_reference_packing.py: `03ca80a5badd28699c54e33a36108922a5ba9cc605c9420e8264b3de0343cd6e`

## Verified assembly and initial receipt scope

The implementation's boundary-space u-.5, mathematical floor before Z validation, independent z0/z1 group/channel
selection and physical group-centre sampling agree with the reference. Negative Z is rejected before integer
division, so truncation inside the nonnegative-only load is harmless. XY normalized coordinates include the one-texel
halo; default clamp-to-edge reaches the zero guard outside support rather than clamping to a cloud coefficient.
Zeroing one whole physical texel before filling valid channels correctly initializes all XY/group guards and odd
unused channels with one writer.

Each logical level is a separate texture, with ceil-halved dimensions and eight zero-extended logical parents.
It does not invoke native mipmapping on packed RGBA groups. The oracle's half rounding operates on an exact binary64
integer ratio, with known bit-pattern ties/subnormals and an independent standard-library half-codec cross-check.
The readback strips 256-byte transfer-row padding and compares tight physical payloads, including guard bytes.

The initial receipt packing-1793572603637.json reports zero byte mismatches across 12 fixtures/36 levels and 1173
queries. Its reported maximum S/Lout errors are approximately 2.0943e-5 and 3.1968e-4. These observations are bounded
results on the root's actual adapter, not portable certification or evidence of cloud appearance, optical mip validity,
full-frame cost or an actual family. The current sampler queries a selected explicit level; point optical queries must
still use level zero. The clear/cloud Jensen witness correctly rejects inferring filtered T from mean sigma.

Successful readback completion precedes normal destruction of field textures/uniforms and temporary query buffers.
readBuffer/readTexture copy mapped bytes before unmapping. The top-level per-run device destruction is a valid bounded
fallback for failed helpers, provided that destruction is guaranteed even when receipt delivery fails (finding below).

## Actionable findings and open gates

### PACK-R1 — P2: operator PASS does not yet gate S and Lout

packing-proof.js 113–117 reports maximum S/Lout errors but only gates coefficient/T and finite Lout. An incorrect
finite in-scatter term can therefore pass. The initial GPU query path also hardcodes 100 m, so the exported nine
constant slab cases, including zero-length/zero-sigma and shorter paths, are not executed by that receipt.

Root independently identified this and plans S<=0.002, Lout<=0.006 plus all nine constant slabs. Keep it open until
the revised code and receipt actually gate those quantities and all returned channels are finite. The small observed
errors in the old receipt do not repair its acceptance predicate. The affine operator must be qualified as such,
not only by its T channel.

### PACK-R2 — P2: actual-store ties/subnormals needed a discriminating fixture

The initial separate ROUND_SHADER checks pack2x16float bits, but the texture/mip fixtures lack deliberate half
midpoints and subnormal-store boundaries. Only two existing mip stores required any nonidentity rounding, so that
receipt cannot support a broad every-store/subnormal claim.

**CPU correction completed in owned files:** added rounding_store_edges-4x4x5 while preserving the original twelve
fixtures and separate codec vectors. Its 2x2x2 parent blocks force means of bits 0/1 -> 0, 1/2 -> 2,
0x2c00/0x2c01 -> 0x2c00, 0x2c01/0x2c02 -> 0x2c02 and max-subnormal/min-normal -> 0x0400. It also stores actual
minimum subnormals at logical z=4 across the group boundary and checks odd zero-extended parents. Known expected
level-one bits reject both ties-away and truncation. All 13 scoped Python tests pass (~0.18 s).

The export now contains 13 fixtures/39 levels/1273 queries and 203770 compact JSON bytes. **GPU closure remains
pending root's regeneration/rerun.** This is a finite stronger correctness witness, not exhaustive hardware support.

### PACK-R3 — P2: receipt network rejection bypasses resource cleanup

packing-proof.js 129–133 awaits the capture POST before device.destroy() and button re-enable. If fetch rejects
(e.g. proof server stops, connection resets), execution exits the finally block early: the per-run device/resources
remain alive and the only run control stays disabled. This occurs on a normal failure path of a local proof tool.

**Correction:** put device destruction/button restoration in an inner unconditional finally or finish device cleanup
before awaiting the receipt POST. Report receipt failure separately without losing the computed report or resource
retirement. Verify a deliberately rejected capture request leaves the report inspectable and another run possible.
No general recovery framework is necessary.

### PACK-R4 — P2: refetched file hashes do not by themselves identify executed modules

packing-proof.js 80–82 labels freshly fetched current files as loaded_code. Static imports already executed when
the page opened. Editing scalar-field.js while leaving that page open, then pressing Run, executes the old module
and hashes the new file. Cache-Control:no-store does not evict an existing page's in-memory ES module graph.

**Correction:** either run against immutable/versioned source bytes, or enforce and record a fresh full-page load
under a pinned no-edit capture protocol with exact pre/post source identity. Relabel arbitrary fresh fetch hashes as
served-source hashes if they are not bound to the imported module revision. A small explicit full-reload/no-edit
runner protocol is sufficient for this bounded instrument; a new module loader is not required.

**Witness:** edit a shader after page load and attempt Run; reject as a stale loaded build or require reload instead
of claiming the new hash identifies the old shader execution. This is an evidence-identity guarantee gap, not a claim
that the root's initial receipt necessarily used stale modules.

## Numeric claims and limits

The coefficient allowance 3*peak/256+2e-7 is an explicit finite acceptance envelope for interpolation; passing it on
one adapter does not establish a universal WebGPU eight-bit filtering guarantee. The separately stricter T/S/Lout
checks must remain, because coefficient error times path length can amplify optical error. Sample-output half bits in
the reference are optional output-store vectors; the current GPU writes f32 result buffers, so comparing its f32
coefficient is appropriate and does not claim a second half store occurred.

The receipt's logical_bytes field currently sums physical guarded texture allocations. Rename it physical_texture_bytes
when reporting costs; it is neither logical dense payload size nor total resident GPU memory (temporary/uniform/readback
allocations are separate). This is a reporting clarification, not a packing arithmetic defect or a performance finding.

No further centre/group/guard/odd-mip arithmetic defect was found in the inspected implementation. Resolve the finite
operator gates, new actual-store GPU fixture, cleanup failure and source-identity protocol before calling the R0
receipt complete. No family-quality, temporal, full-sky, engine binding or runtime performance claim follows.
