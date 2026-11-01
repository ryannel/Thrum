# Independent bounded closure and native image review

2026-11-01. Follow-up to `implementation.md`. Read current full source/storage adapter, common transport, caches, reconstruction/reflection, GPU probes and validator paths; inspected three actual1280x720 PNGs directly at native dimensions. No GPU benchmark, code modification, commit, browser session, moving-video viewing or device qualification by this reviewer. Code continued changing during review: inspected source hashes are below. Final current-specialization validation was pending at review time.

```
common.wgsl 33c26db66820de91f7d2d922cd962bc76704ebcdcff1c135234d55ee061cfa65
caches.wgsl 82d56d21e63b504b13bf35c8f092f2d261d2d28bbf1033a75f128e6de9068e95
render.wgsl c9fcf9f95de2b0c248b0d4d8234a722308a12cf7b779bd58b22f2b7554c010b8
probes.wgsl 73b1d7f2313477feef119a6f3ba317629bea2db73c3af5f6411fe5fe1797a6a7
source.js 0f5d7cf85cbe13a9aa07cf3a38cdb612cb5ec1c3c54c15456bfe7785d986f441
viewer.js 749ce580f00a257f295050d23654ee2c0870a19b623ec7ab7f082293d744cea0
```

## Earlier findings

- F1 resolved in source: reflected cloud interval is now exported and checked; both main and reflected reconstruction require essentially the full bilinear weight footprint (`total>.9999`) to pass. Cloud intervals are clipped to actual opaque depth and misses normalize to zero, avoiding meaningless behind-terminal differences. This is conservative suitability, not a proof of a radiance error bound.
- F2 resolved in source: reflected full-resolution fallback checks `value.valid` and exposes failure consistently.
- F3 resolved in source: benchmark retains each measured frame's counters. Cost discussion now explicitly rejects invalid per-render-pass timestamp attribution and uses compute sentinels around the assembled graph.
- F4 addressed for bounded path semantics: `probes.wgsl` evaluates actual common light/geometry/coefficients and actual joint transport, including a mirror far ray; independent float64 validators compare those values. Main S/T/guide/HDR exports also exist. This is meaningful actual GPU consumer evidence, not only a CPU solver testing itself. The current exporter still does not directly dump reflected raster outputs/guides, so the probe reference and actual full-resolution image comparator retain distinct roles.

## Native image observations

Inspected:

- `captures/scene-1793564424106.png`: earlier production reconstruction. A visible narrow, vertically patterned streak follows reflected cloud edges; the main cloud boundary also shows slight local artifacts.
- `captures/scene-1793564427183.png`: actual full-resolution reference. Those edge streaks are absent; the cloud body and reflected view have coherent separate perspective, the sphere clips foreground correctly, and the fog's spherical footprint is visible near the reflected cloud.
- `captures/scene-1793564501175.png`: corrected production reconstruction. The conspicuous reflected streak is removed and this still appears close to the full-resolution reference. No obvious cloud leak through the sphere or horizon strip was seen in this fixture. The main cloud has a billowy opaque-looking body, a reasonably level base and differentiated lobe lighting; its mirror image shows a different surface of that body rather than simply flipping the camera image.

These are age90, default-Sun stills at one camera configuration. They do not establish ordinary motion, breakup, all source ages, low-Sun quality, upper-occluder transitions, camera entry, or user acceptance. Their capture identities predate the current bound-specialization hashes. They establish the reconstruction correction on the earlier assembly; the final optimized assembly still needs its actual image/identity and parity checks. Numerical image differences may supplement this inspection but were not independently calculated here.

## Conservative specialization derivation

The pinned shape's nonzero base nodal bounds are XYZ `[33..94,15..56,28..101]`. Authored threshold growth/erosion never adds a node outside this master support. Interpolating cooked endpoints is positive and does not add nodes either. This gives:

| Field | Nodal indices XYZ | Nonzero trilinear support in local metres XYZ |
| --- | --- | --- |
| Base128³ |33..94,15..56,28..101 |130..382,58..230,110..410 |
| Mip1 64³ |16..47,7..28,14..50 |128..384,56..232,112..408 |
| Mip2 32³ |8..23,3..14,7..25 |128..384,48..240,112..416 |

The source decoder's nodal domain `[32,12,24] <= c < [96,60,104]` safely encloses all base nodes. Explicit `pack2x16float` then unpack before RGBA16F storage preserves the intended Q1 round-to-half adapter rather than relying on a potentially different storage conversion. Actual readback parity still must qualify the compiled adapter.

The maximum-cache computed coarse cells `[7..24,2..15,5..26]` safely contain every cell whose `4*cell+[-1..4]` trilinear filter halo can touch nonzero base density (required cells only `[8..23,3..14,6..25]`). Zeroing outside therefore does not remove cloud support.

The prefix integration box `[112,32,96]..[400,256,432]` contains the union of all three mip supports with margins. Skipping source samples wholly outside that box leaves ordinary cumulative depth unchanged. Writing the previous cumulative value for skipped prefix cells also preserves valid prefix endpoints.

Ambient is sampled by actual transport only where base density is positive. At those points its32³ trilinear interpolation can address nodes X7..24,Y3..14,Z6..26. The computed node domain `[112,32,80]..[400,256,432]` includes centres X120..392,Y40..248,Z88..424, therefore all required taps. Zero outside is safe for this cloud source consumer, but the resulting product is **not** a valid incident-ambient query everywhere in the512m cube. Arbitrary diagnostic calls there may intentionally see zero. Keep that validity boundary explicit.

The base/traversal support box remains `[128,48,96]..[384,240,416]`, safely enclosing base density; ambient axis rays integrate this conservative box at4m steps. All specialization proofs depend on this exact source and current filter/mip definitions.

## Remaining concrete issues and limits

1. **Admission must enforce the content-specific bound certificate.** At the inspected revision `PreparedSource.create` verifies binary bytes against the loaded manifest's own SHA, but accepts any schema-compatible manifest. Hardcoded decoder/max/prefix/ambient bounds would silently clip another self-consistent source installed at the same URL. Require the certified selected product hash (`999cd89480371d02dca2f7553b938e5b93e23f0b31ca9b306ae60beb62e325bd`) or an independently validated matching bound descriptor before applying these optimizations. Recording the loaded hash is provenance, not admission enforcement.
2. **Ambient validation must respect the new queried domain.** Existing `validate_cache.py` also tests external cube corners `[8,8,8]` and `[504,504,504]`. Those points are intentionally outside the optimized ambient validity domain, so comparing their published zeros to physically nonzero sky would create a false error. Separate unused/unavailable diagnostics from the maximum over valid cloud-consumer nodes; include boundary interpolation taps. Do not hide true in-domain error by dropping arbitrary failing nodes.
3. **Independent-consumer probe options need to follow actual reflection semantics.** `exportQueries.add` currently gives every query `cloudOn=state.main_cloud`, including mirror-secondary. Actual reflection always uses cloudOn=true. In the main-off/reflection-retained fixture, the separate secondary probe would therefore not represent the actual reflection consumer. Also its displayed-pixel direction helper uses tan0.6 while overview uses0.55. Use latched actual camera parameters and reflected query policy before claiming these modes from probes. These do not invalidate the default inspected fixture.
4. **Cache approximation error remains quantitatively material.** In actual `probe-1793564416246`, independent validation reports source/mip parity, view S relative discrepancy<=1.294%, T absolute discrepancy<=1.642e-6, composed discrepancy<=0.620%, and mirror full discrepancy<=0.0753% when comparing with the same actual light/ambient caches. That verifies bounded closure/view/foreground composition well. It does not remove physical light-cache error: dense primary tau3.338 versus direct2.748, receiver3.725 versus3.125, with up to45.1% relative direct-incident discrepancy; sampled world max absolute T error0.02859 and tau error0.6001. Report absolute and relative effects and a declared acceptance floor. This is a cache approximation limit, not evidence of double attenuation or invalid joint composition. No predeclared passed physical Q2 floor is established by that report.
5. **Final specialized artifacts need their own numerical and visual verification.** `probe-1793564729432` has current source/common/cache/render identities; its validation was pending. Earlier4416246 results cannot certify bound-skipped products without comparison. At least one actual final capture plus source/mip/cache/consumer checks should accompany those source changes. Existing stills cannot be relabelled final optimized output.

## Performance disposition

The author reports improved assembled sentinel-bracket timing around12ms median/18ms p95 after specialization. This reviewer did not run or independently certify that measurement. Even if confirmed, it is above the proposed5ms cloud GPU allocation and has tails beyond the complete16ms browser envelope before integrating a game. A faster source kernel or correct bounds cannot turn it into target admission. Current larger128³ RGBA16F prefix and RGBA16F density mips also differ materially from the original64³/scalar planning ledger and need the actual logical resource account; browser peak remains unmeasured. Preserve that distinction in final claims.

## Recommendation

The bounded coupled-lighting model and inspected default assembled still are credible; the earlier four source/evidence findings have substantive resolutions. Enforce the bound admission pin, repair remaining query/validator domain semantics, and retain final-specialization evidence. Keep physical cache accuracy, moving-output coverage, complete Thrum integration and device/frame-budget qualification explicitly open. No new structural transport contradiction was found. This review does not establish full Q2/Q3 completion or user acceptance.
