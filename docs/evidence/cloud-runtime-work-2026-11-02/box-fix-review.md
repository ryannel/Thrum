# Stratocumulus box-face fix review

2 November 2026. Read-only review of the `source.wgsl` finite-bank change in build `752b5eafbbe430ca`. Compared the before build `dc9a9efe9893ef97` and after beauty captures at the same family Sc, camera (yaw .3, pitch −.18, distance 7500 m), source/world time 0, Sun 32°, optical density scale 3, full 1280×720 view, and box bounds. The after shader/test hashes match the pinned build receipt. No source edits or new GPU session were made.

## Source mechanism

`bankRadius(x)=length(x.xz / (4000,3200))` is an authored world-space ellipse. `bankSupport=1−smoothstep(.65,1,bankRadius)` stays 1 centrally, tapers smoothly, and reaches zero at the 4.0×3.2 km ellipse. That is 1.0 km inside the x=±5 km storage faces and .8 km inside the z=±4 km faces. Its placement does not depend on the storage bounds, so enlarging/rebasing a chart does not enlarge the authored cloud bank.

Both Sc and St seeded q multiply by this support. Their ongoing `localSource` multiplies injection by the same support and uses it as the spatial flank value; decay also gains `0.08*smoothstep(.75,1,bankRadius)` per second near the edge. Thus the new initial shape, renewal and loss all favour a finite bank instead of replenishing a sheet up to the source-box face. It is deliberately a finite isolated bank, **not** proof of a global overcast St provider. Advection can carry existing q outside the authored ellipse; the added outer loss and storage margin make that a finite transient, but long-run/high-wind boundary clearance still requires watched source evidence.

## Evidence and visual verdict

- Before: the seeded Sc probe reports 158,139 occupied voxels and **13,886 occupied boundary voxels**. The before opacity-ray capture is a nearly full-width solid bank, independently showing source/opacity support at the storage edge, not merely a lighting tone.
- After: the seeded Sc probe reports 68,700 occupied voxels and **zero occupied boundary voxels**. The corresponding beauty capture shows a finite, rounded bank with clear sky at both sides. The previous conspicuous full-width rectangular wall is absent. The reduction in volume also changes composition/coverage substantially; it is not a same-morphology lighting-only adjustment.
- The build-752 GPU `finiteLayerSupport` check passes for seeded Sc and St: edge peak 0 for both, interior peaks .7949 and .5928. It checks the 8³ stage-0 source fixture, not all live-update steps, velocity histories, view silhouettes or light-cache accuracy. The full-resolution before/after capture and boundary probe provide the stronger evidence for this particular Sc seeded-state artifact.

I do not see a material remaining **axis-aligned internal face grid** in the after beauty image at this camera. Rounded lobes and a dark shallow underside remain; a single beauty view cannot decide whether subtle internal tonal bands come from the 32×24×24 point-light cache, its 32/20-step Sun/ambient rays, or the 96-step view march. A held-source direct-only/ambient-only comparison or light-cache reference at the suspicious sample positions remains the decisive test if a panel reappears. This visual fix does not qualify Sc/St final morphology, continuous bank renewal, cache error, or device cost.
