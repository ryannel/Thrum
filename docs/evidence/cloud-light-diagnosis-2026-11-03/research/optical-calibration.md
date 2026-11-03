# Covered-Cb optical-depth calibration checkpoint

3 November 2026. This is a bounded explanation of the frozen scalar and existing renderer settings, not an accepted κ, source cook, or lighting change. The root agent owns the actual κ=.019 and later image comparison.

## Observed images and separate causes

The original f06 384-ray image in `captures/f06/*-2.png` reads as a smooth white body with a thin cap. Current daab selected-profile low and high reference images at κ=.0045, held oblique camera and the same scalar are nearly the same smooth body. The native q=.03 source-normal view reveals many curved billows on the body and separately exposes parallel cap ribbons. The root's subsequent held high-cache selected-profile frames at κ=.019 and .038 show more internal/edge contrast at .019, then mainly darkening at .038; the pear/club body and thin cap persist. Those daab frames remain in `../cloud-cb-light-reference/captures/` as `*-5.png` and `*-6.png`, with receipts pinning the κ values. These are sampled stills, not motion or full Cb acceptance. High cache alone therefore has not recovered the missing beauty detail at this held pose. The normal image establishes a structured first threshold crossing; it does not establish opacity, internal density or the rendered image response.

The 16-point high-cache Sun probe at κ=.0045 reports max absolute ordinary cached-T versus direct 512-sample T error .0088168, while direct 256 versus 512 max is .0001117. This is a surface-point subset, not a whole-image bound. The earlier 2,048-point CPU low-cache study found larger cache errors; the populations/cache resolutions differ and cannot be interchanged.

## Source values and optical lengths

All following distribution arithmetic is from the exact hash-checked R16F source, decoded as native values without artistic remap. Values count nonzero native voxels unless specified. A column is the sum of q along the full X axis times its physical 46.875 m pitch, including zeros; the quantiles are over columns with positive sum. These are *not* camera-ray, lobe-isolated, or surface-normal columns.

| Source | Peak q | Positive q p10 / median / p90 | Positive X-column ∫q ds p10 / median / p90 / p99 (q·m) |
| --- | ---: | ---: | ---: |
| Covered Cb 256³, 12 km X span | .14697 | .00405 / .10010 / .14697 | 8.6 / 362.3 / 768.1 / 851.3 |
| Qualified Cu03 256³, 2.048 km X span | .71533 | .00145 / .05472 / .12842 | .3 / 13.1 / 69.4 / 96.1 |
| Qualified Cu08 256³, 2.048 km X span | .52051 | .00163 / .05991 / .15527 | 1.1 / 26.5 / 75.0 / 117.2 |

The current prototype `src/scene.js` uses liquid κ=.0045 m⁻¹ at extinctionScale1. The selected qualified Cu03 2026-11-03 upload receipt instead records κ=.019 m⁻¹ (extinctionScale4.2222), so .0045 is an inherited prototype setting, not an independently qualified calibration for this covered Cb. The production baseline defines `σ_t=κq`; q is a normalized optical proxy, not physical water mass. Matching q peak alone does not set κ: support size, density profile and integrated extinction matter.

At covered Cb peak q, κ=.0045 gives σ=.0006614 m⁻¹ and a mean free path of about 1512 m. A 338 m representative lobe radius has τ≈.224 (T≈.80); a 200 m feature has τ≈.132 (T≈.88). At κ=.019, peak mean free path becomes about 358 m; the same radius has τ≈.944 (T≈.39). For the 330 m distance-to-density ramp, even an illustrative average q=.07 gives τ≈.104 at .0045 or .439 at .019; the real ramp varies spatially and must be integrated. This makes weak local opacity a plausible reason the threshold normal detail disappears from beauty at .0045.

The opposite risk is whole-body saturation: the covered positive X-column median/p90 τ is 1.63/3.46 at .0045 and 6.88/14.59 at .019. These full columns cross multiple features and do not measure the silhouette shell, but they show why raising κ may recover edge contrast while hiding the interior or making the body opaque. The cap's parallel ribbon anatomy is a separate source-shape failure that opacity cannot repair. The held κ=.019 comparison supports using that previously qualified Cu input calibration as the next fixed source-profile control; κ=.038 showed little morphology benefit. It does **not** select .019 as final Cb calibration or accept anatomy. Require image/column/lighting agreement across body, cap and angles before such selection. Do not infer Cb anatomy, motion or production cost acceptance from one calibration image.

Reproduction of the distributions: read each listed scalar as little-endian R16F `shape=(256,256,256)` `[Z,Y,X]`; compute quantiles over `q[q>0]`; compute `np.asarray(q,dtype='f4').sum(axis=2)*pitch_x` and quantiles over positive columns. The exact scalar paths and hashes are in `manifest.json`. No source regeneration or GPU run was performed for this calculation.
