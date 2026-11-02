# Dense Cu source work-mask code review

2 November 2026. Read-only review of `shaders/support.wgsl`, `shaders/dilate.wgsl`, the mask branch in `shaders/source.wgsl`, `src/scene.js` per-tick scheduling and `src/source-mask-reference.js`. Scope: dense 96×80×80 Cu pilot, 8³ work bricks, not sparse residency or family/full-sky qualification. No runtime edits.

## Verdict

**No blocker found for the Cu-only work mask under the current admitted field/control bounds.** The support pass scans every old q texel in each brick and marks any positive liquid/ice. It also marks bricks intersecting prospective emitter XZ support and the conservative Cu base band, even when old q is zero. A separate gather dilation reads the completed support buffer and writes the 27-neighbour enabled buffer. `Scene.render` performs scan → dilation → q forward/reverse/corrected passes for each source tick before moving to the next tick; those passes share the same selected old q and emitter buffer. The rest-coordinate carrier still runs over its full domain. Inactive q passes explicitly store `vec4f(0)` before velocity, sampling, source or correction, so reused forward/reverse/next textures do not retain stale values in skipped cells.

For Cu, `source.wgsl::localSource` has at most 32 emitters, each with compact XZ support `r<1.25×radius`, a smooth window ≤1, edge factor ≤1.8, moist≤1 and `min(lift,1)≤1`. Beyond `|y−1220m|=900m`, an empty cell's per-step birth is bounded by

`q_birth ≤ SΔt ≤ 32×0.065×1.8×0.25×exp[−(900/190)²] ≈ 1.7×10⁻¹⁰`,

far below half-float round-to-nonzero threshold `2⁻²⁵≈2.98×10⁻⁸`. The support pass marks any brick whose Y interval intersects `[320,2120]m`; old positive q outside remains active from the scan. Drying/loss cannot create positive q. The mask conservatively admits emitters whose current window is zero, which costs work but cannot omit a birth. Other families force every brick active; no Sc/St/Cb/Ci saving or proof is inferred.

The cited ~40m/s velocity is an observed estimate, not a hard bound. For current `Weather` control ranges and shader operations, a conservative component bound is about x≤54.7, y≤51, z≤39.6 m/s: world wind≤19.2 in x; outflow≤22; prescribed updraft≤42; curl components after generated-field scaling≤0.75, contributing≤13.5/9/13.5; base z drift≤4.1. Thus `|u|<85m/s`, and one 0.25s backtrace moves <22m. The smallest source-cell pitch is 65m vertically (horizontal pitches ~100–104m); a one-brick Chebyshev halo is eight cells wide. It covers backtrace, trilinear donor and corrected/reverse stencil reach even at the conservative bound. Any future velocity/timestep/domain scaling change must redo this bound before using the mask.

## Actual differential evidence and its limit

The retained `captures/source-mask-comparison.json` reports build `4dbb455db2f27669`, `pass:true`, 36 records across empty birth, existing source and edge birth. Every record has zero different/invalid voxels and zero maximum decoded RGBA16F difference; GPU errors are empty. The captured SHA-256 values for current `source.wgsl`, `support.wgsl`, `dilate.wgsl`, `scene.js` and `source-mask-reference.js` match the files read in this review. The test compares **all four q channels** after every step; it does not merely compare a rendered picture or total density. Its dense oracle switches the mask flag off while retaining the same source operator, which is appropriate for work-selection equivalence. The zero-initialized fixture is explicitly cleared on reuse, and the edge fixture places a prospective emitter near the X boundary.

The differential harness submits and waits after each individual stage and copies the masked result into the common next-step input. With all 36 per-step comparisons equal, that shared input does not weaken the observed chain. However, it does **not** itself stress eight ticks encoded into one production graph, three in-flight graphs, or mask scratch reuse under asynchronous timing. `Scene.render`'s ordered encoder currently recomputes support/dilation before each tick and performs all three q passes before reuse, so this is a targeted unrun scheduling test, not a source-support design blocker. The fixture also covers 12 ticks per case and a limited wind/front/lift sequence; it does not prove every admissible emitter placement, extremely long source history or future sparse pages.

## Focused follow-up checks

- Run one production-style multi-tick graph and in-flight queue case against a separate unmasked q chain, comparing full RGBA16F after each submitted graph or at controlled pauses. Include a new emitter immediately after an inactive brick and a wind reversal. This targets stale support/active and forward/reverse scratch, which the current stage-by-stage oracle does not exercise.
- Retain a numerical assertion for the half cutoff and velocity/halo bound in the proof metadata. Fail mask admission if control ranges, emitter maximum/count, source pulse rate, timestep, grid pitch or curl scale changes beyond the proven bound.
- Treat nonfinite/negative old q as an invariant failure in the support scan/diagnostic. `v.x+v.y>0` alone would classify NaN as empty and could hide a source fault. The current 36-tick oracle checks invalid output, but a production diagnostic should not silently skip invalid input.
- Measure scan + dilation + per-cell mask fetch/zero writes with all consumers and actual inactive fractions. Dispatch is still full-grid, so exact source equality does not establish a net GPU benefit or M2 viability. The observer-only build must be identified separately; the retained comparison certifies the named 4dbb build, not an assumed later build.

**Evidence status:** Cu dense work-selection equivalence passed for the retained 36-tick build; multi-tick/in-flight schedule, complete visual quality and full-device performance remain unqualified.
