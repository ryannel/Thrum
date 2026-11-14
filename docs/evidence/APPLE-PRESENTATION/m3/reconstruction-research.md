# M3 still reconstruction diagnosis — 14 November 2026

The evidence identifies two separate issues. **Nearest-texel previous-depth rejection was destroying useful accumulation; the remaining night failure is a broad, one-code green-channel error plateau after the fixed SDR transform.** The night tie is not evidence that TAAU equals no AA, nor evidence that the reference is unstable. Half-float feedback sticking is a plausible cause of the plateau, but the retained data cannot prove it: it contains the independent linear reference means and displayed TAAU images, not TAAU's pre-tone/history values. The queued unbiased-half-rounding proof is therefore the appropriate next discriminating result. Do not change another blend constant or the acceptance rule while it runs.

This is a focused CPU/source diagnosis, not a target proof. No GPU jobs, builds, runtime edits or acceptance changes were made for it.

## Evidence and causal limits

The authorities remain [the design](../../../plans/APPLE-PRESENTATION/DESIGN.md), D3.2 in [DONE.json](../../../plans/APPLE-PRESENTATION/DONE.json), and [C07](../../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4). The required comparison is whole-frame displayed SDR CIEDE2000, p99 at most 3 and **strictly below** no AA at every checkpoint. The separate 32-offset RGBA32F linear reference and its two-cycle held-state test stay unchanged.

[Attempt 2](attempt-2/run.json), at clean source `9c1349f2`, fails four improvement comparisons despite all six independent linear cycle means agreeing bit for bit and the cut/reset controls passing. This rules out changing world/solar/celestial/exposure state between reference cycles as the cause shown by this run. It does not prove that all temporal inputs or all spatial correspondence are correct.

Two later runs are **dirty-source diagnostics**, not clean-commit passes:

- `tmp/g7-showcase/runs/reconstruction-20261114-061544/run.json`: disabling only depth rejection improves five checkpoints; night still ties. This intervention directly implicates depth rejection in the daytime failures; it cannot authorize removing required disocclusion handling.
- `tmp/g7-showcase/runs/reconstruction-20261114-061857/run.json`: the full reprojected depth footprint plus history weight 0.97 gives the following results. Because this run also changes weight, it does not isolate the footprint correction's individual benefit.

| Checkpoint | TAAU p99 | No-AA p99 | Result |
| --- | ---: | ---: | --- |
| sunrise | 0.948607 | 1.123111 | lower |
| noon | 0.986735 | 1.369814 | lower |
| sunset-clear-sun | 0.828533 | 0.888748 | lower |
| sunset-sun | 0.948346 | 1.099360 | lower |
| twilight | 0.965161 | 0.971272 | lower |
| night-upslope | 0.9245245590916956 | 0.9245245590916956 | fails strict improvement |

CPU analysis of that run's `frames/night-upslope/{reference,taau,off}-64.ppm`, using the existing `scripts/reconstruction_difference.py` Lab and ΔE functions, locates the exact night tie. The nearest-rank p99 is sorted pixel 228,096 of 230,400:

| Distribution relative to 0.9245245590916956 | TAAU pixels | No-AA pixels |
| --- | ---: | ---: |
| below | 226,923 | 227,420 |
| equal | 1,317 | 973 |
| above | 2,160 | 2,007 |

Every equal-error pixel is one of the same two RGB pairs: reference `(19,17,13)` versus candidate `(19,16,13)`, or the reverse. TAAU has 1,295 green-low pixels and 22 green-high pixels; no AA has 224 green-low and 749 green-high pixels. The TAAU plateau spans x=0..639, y=196..359. Only 177 of its plateau pixels have identical TAAU and no-AA RGB. Thus equality of p99 is a quantile plateau, not equality of images; TAAU's maximum error also improves from 8.850289 to 6.269535 while failing p99.

To move TAAU p99 strictly below this plateau, at least 1,173 currently equal-or-higher pixels must move below it, assuming no pixels move upward across it. Fixing a handful of stars cannot do that. The dominant green-low plateau is the concrete diagnostic target. These spatial counts are explanatory only; acceptance still judges every pixel.

## Source and representation boundaries

`synthetic/taau.wgsl` loads the jittered RGBA16F scene sample, dilates velocity from frontmost reversed-Z depth in 3×3, reconstructs world position, and compares expected **previous-view linear depth** with R32F history. Colour history addresses `p + velocity` on its resolved grid; stored depth addresses that position plus previous-minus-current jitter because depth remains on the jittered raster. The old nearest-depth lookup samples one raster point on a sloped surface; its depth differs under jitter even when there is no disocclusion. The footprint range correction preserves the previous-view relative-threshold comparison and addresses this representation mismatch. A range spanning separate surfaces can still admit mixed history, so motion/reveal review remains necessary.

Colour then rescales accepted history by current/previous storage p, clamps in YCoCg against the current 3×3 range, applies 1/(1+luma) weighting, and stores the result into RGBA16F output/history. `taau.rs` binds a distinct RGBA32F ping-pong accumulation resource for `reference.wgsl`; that shader sums native jittered scene samples without TAAU clipping/rejection and emits the mean through the same RGBA16F pre-tone seam. The test independently computes ordered f32 means from submitted RGB16F scene captures and checks both cycles. This is not feedback from TAAU being mistaken for its reference.

`synthetic_display.wgsl` reads one pre-tone texel and applies matching resolved exposure, the selected tone operator, sRGB encoding and fixed per-pixel triangular dither to Rgba8Unorm. Its selectors come from retained exposure; night records selector 1. The plateau is therefore after both half-float quantisation and the established dithered SDR quantisation. Fixed dither does not make a linear bias harmless: it decides which pixels cross each 8-bit threshold. Do not remove or retune that shared transform to conceal the error.

The 1,295 green-low pixels have median reference stored RGB approximately `(0.00329697, 0.00273699, 0.00203425)`. Their green values lie in the half-float exponent interval with ULP `2^-19 ≈ 1.90735e-6`. For a simplified unclipped EMA with new-sample coefficient 0.03, deterministic round-to-nearest can suppress updates whenever the input-history difference is below about `0.5 ULP / 0.03 ≈ 3.18e-5` (16.7 ULP). This demonstrates a real accumulation precision mechanism, not a measured TAAU error magnitude. The actual kernel has clipping, changing luminance weights, depth/reactive rejection and finite warmup. A 0.97 recurrence also retains approximately 14.2% initial weight after 64 updates; it is not the reference's uniform 32-sample box mean.

## Published methods and recommendation

[Karis's UE4 SIGGRAPH presentation](https://de45xmedrsdbp.cloudfront.net/Resources/files/TemporalAA_small-59732822.pdf) describes Halton jitter, exponential accumulation, pre-postprocess luminance weighting, velocity dilation and YCoCg history restriction (slides 12–19, 24–33). It also explicitly identifies mixed sample depths and clamp-induced flickering (28, 41–44). These mechanisms are tradeoffs rather than a promise to reproduce an arithmetic box mean. In particular, the mandated luminance weighting and clipping mean even infinite-precision accumulation is not mathematically identical to this reference.

[AMD's FSR2 documentation](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-temporal/#reproject-accumulate) separates reconstructed depth/disocclusion, reactive handling, history rectification and accumulation. It uses an RGBA16F history pair and additional thin-feature locks/luminance stability data. Its filter explicitly accounts for jittered input sample positions. This supports preserving the existing frame seam and half-float pair while correcting validity/precision internally; half-float storage itself does not require replacing the design. It also cautions against expecting a minimal EMA to inherit a shipped reconstructor's stability merely from sharing several techniques.

**Recommendation:** retain the footprint-aware previous-view depth comparison and evaluate the already queued unbiased-half-history-rounding correction before selecting any further runtime change. It directly addresses a representation defect compatible with the required RGBA16F pair, keeps exactly representable history exact, and leaves the sole SDR transform intact. Its success is unproved. The current seed repeats with jitter phase, so its threshold is deterministic and correlated over repeated cycles; the usual expectation argument for stochastic rounding is not a guarantee of convergence for this exact 64-frame schedule.

If that proof still fails at night, the specific missing evidence is submitted pre-tone TAAU RGB and the corresponding incoming history/current RGB at representative green-low plateau pixels, with depth acceptance, reactive value, clipping result and effective blend coefficient. Compare those to the retained f32 reference mean and the relevant half-float neighbours. This distinguishes persistent rounding bias from biased luminance weighting, clipping/rejection impulses and finite warmup. One narrowly scoped diagnostic capture can answer it; another parameter sweep cannot. The retained final SDR images alone cannot choose between these causes.

**Can the existing mechanism meet the comparison?** Nothing in these records proves a foundational change is necessary. Five strict improvements already occur in a compatible diagnostic, and RGBA16F history is used by published reconstruction systems. Nothing yet proves all six can pass with the current implementation either. The binding failure is a quantised night plateau, and only a clean target result can establish strict improvement. Neither the reference nor its threshold should change on the evidence here.
