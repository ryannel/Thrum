# Final bounded render code review

2 November 2026. Read-only inspection of pilot build `a65113f850bee519` and `captures/gpu-reference-checks-a65113f850bee519-820e7fb0-1.json`. Current SHA-256 hashes for the changed optics, view, GPU reference and scene files match the receipt. No source edits, new GPU run, art verdict or budget acceptance were made.

## Beer transport

`beer(j, sigma, ds)` uses `T=exp(-sigma*ds)` and the segment source integral `j * (1-T)/sigma`. For each RGB channel with `x=sigma*ds<.001`, it substitutes `j*ds*(1-x/2+x²/6)`, the correct Taylor expansion through second order. At vacuum (`sigma=0`) this returns `j*ds`; both expressions passed to WGSL `select` remain finite because the ordinary denominator is clamped to `1e-20`. `join(a,b)=(a.S+a.T*b.S,a.T*b.T)` preserves front-to-back composition. Nonnegative sigma and ds are the admitted domain.

The pinned Apple Metal 3 GPU receipt reports passing checks for constant optical depth, its Beer transmittance, homogeneous source integral, weak/vacuum source limit, two-half versus full-segment composition, clear-air transmittance, phase normalization, and bounded interior source examples. It reports zero GPU errors. Its weak test uses sigma `[0,1e-9,1e-6]` over 2 m; it does not characterize switching error exactly at `sigma*ds=.001`, inhomogeneous segment quadrature, cloud cache accuracy, or full image transport. The prior misleading `analyticBeer` label has been replaced by separate checks matching what is asserted.

## Analytic shadow-only provider

`analyticShadowT(x)` projects a receiver below y=2200 m along the current sample-to-Sun direction to one caster on that thin plane. It evaluates a wave there using integrated wind displacement, moistening, drying and the same moving front coordinate supplied by Weather. Points on or above the plane return visibility 1. Within the admitted UI Sun range (elevation at least 3 degrees), `max(sun.y,.03)` equals the actual positive Sun y component, so the projection is consistent with that direction.

For shadow-only mode (`p.domain.w` between .5 and 1.5), `directT` returns this common visibility and its `.25` second-octave power; `cacheT` delegates directly to `directT` without sampling an absent volume light cache; `receiverT` calls the same helper. Fog scattering in `rayAir` queries `cacheT` whenever fog is present. The reflected upward sky path also uses `rayAir`, so it sees the same analytic fog attenuation. Clear mode returns unit cloud visibility. No volume source is allocated for the shadow-only/clear provider beyond one-voxel placeholders, and the provider switch retires the previous scene/device before constructing the next.

The GPU analytic-provider check compares one point with an independent CPU expression, a second point that projects to the identical thin-plane caster, and one above-plane point expected to be clear. It passes within `5e-5`. It does **not** test moving-front responses, low-Sun projection, fog image coupling, or reflected-image equivalence; those remain staged visual/interaction gates.

## Receipt and resource lifecycle

`compareSourceMask` now reuses its stored result only when both `scene.epoch` and `scene.time` match. A same-Scene reset increments the epoch; source advancement changes time. Its comparison textures/buffers are retained in `scene.maskFixture` after the first observation and are included in the GPU logical-allocation ledger. A source-less provider cannot invoke that fixture and receives no full velocity field; switching providers disposes the old GPU device. The previous stale-result-after-reset finding is closed for the source mask receipt.

## Explicit limits

Outside the 3D q domain, `rayAir` still does not multiply *air scattering* by cloud or geometry shadow visibility. Its fog scattering does use `cacheT`. The fixed sky mean is not SH9 or a clouded current environment. The dense pilot's source morphology, occupancy-aware light error, complete joint air/fog/cloud path, independent reflection proof, temporal reconstruction and normal/64x cost budgets are not established by these unit receipts. Keep those acceptance gates unqualified.
