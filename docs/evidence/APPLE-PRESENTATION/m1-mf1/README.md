# APPLE-PRESENTATION M1 — MF1 repair

11 November 2026. Mac Metal on Apple M3 Max, portable recipe. The original review passed D1.1–D1.5 and identified MF1 outside those items. This evidence covers the repair and current M1 build. [Review](../M1-review.md)

MF1 is addressed at view selection. The game requests the continuous jitter sequence while packing an unjittered raster matrix. Explicit temporal views use `with_jitter_phase` to apply the sample. Geometry, synthetic GPU culling and cooked contributions receive the selected raster projection. Stars follow that sampling choice. Both modes retain the phase, offsets, previous submitted matrix and unjittered matrix. The 240-byte layout is preserved and RENDER-SPEC hashes are refreshed. [Layout tests](layout.log)

The regression tests read submitted GPU colour. Explicit raster jitter must change colour in the control. Spatial output must retain nonzero sample offsets and advance the phase while 35 consecutive colour comparisons remain byte-identical. The game pause test checks consecutive frames across the complete phase cycle. Recovery compares the first replacement frame with its single pre-loss reference and verifies a consecutive replacement frame exactly. The original one-code cross-device bound is retained, with all observed recovery differences zero. [Metal motion and colour tests](motion.log), [game lifecycle and recovery tests](host.log)

## Reviewer held-camera comparison

The reviewer’s held route was repeated at 640×360 and 1920×1080, with 71 consecutive submitted frames per extent. Every pair was compared. The world continues running in this route, with a moving cube and solar time advancing from 12.0127 to 12.1060 h. The paused-world test above supplies the whole-frame byte-equality proof. [Comparison tool](../../../../scripts/apple_presentation_m1_held.py), [complete measurements and crop bounds](held-summary.json)

| Measurement over 70 pairs | 640×360 | 1920×1080 |
| --- | --- | --- |
| Whole-frame pixels changing by >24 codes | 70–96, versus the reviewer’s 950–1,900 | 660–846 |
| Skyline pixels changing by >24 codes | 0 | 0 |
| Ground-patch pixels changing by >24 codes | 0 | 0 |
| Sphere upper silhouette/highlight pixels changing by >24 codes | 0 | 0 |
| Sphere-and-shadow crop pixels changing by >24 codes | up to 15, all in the shadow region | up to 134, all in the shadow region |

I inspected the reviewer’s images and the repaired whole frames, consecutive native crops and difference images. Sphere silhouettes stay fixed, the skyline and terrain edges stay fixed, and the difference images localize the continuing cube and shadow animation. [640 sphere ×4](held-640/sphere-four-consecutive-x4.png), [640 difference ×4](held-640/consecutive-difference-amplified4.png), [1080 native sphere sequence](held-1080/sphere-four-consecutive-native.png), [1080 difference ×4](held-1080/consecutive-difference-amplified4.png)

## Current M1 evidence

| Item | Current proof |
| --- | --- |
| D1.1 | Exact sequence and record-layout tests, spatial/temporal selection tests, and real submitted-phase assertions. |
| D1.2 | Static geometry, sky and stars read exactly zero motion over 35 explicit jitter phases. Moving geometry must be nonzero. The spatial colour control is checked separately. |
| D1.3 | Real Metal sky pan and solar/sidereal-only results match independent projection within 0.05 px. Small solar-step error is below 0.0001 px. |
| D1.4 | Existing invalid-history marks remain on spawn, cut, replacement and cooked swap. Both cooked draw paths share the forward motion shader. The new selection changes the raster matrix at packing. The invalid-marker tests and restored host regressions pass. |
| D1.5 | I watched motion as ordered frame sequences on the current build. Pan direction, unsaturated magnitude growth, skyline continuity, near/far parallax, stars and the moving cube remain coherent. [Day](day/preview-fixed-1x.mp4), [night](night/preview-fixed-1x.mp4), [translation](moving/preview-fixed-1x.mp4), [native crops](motion-crops.json). |

The milestone checker completed all four `./dev check` runs with 4 pass, 0 fail, 0 error and 0 timeout, using the default test concurrency. Each run held `gpu`. The retained adapter uses the original 1.7.1 checker and adds the same lock to its profile Check pass. [Checker results](checks.json), [GPU-lock adapter](check_locked.py)

Eight manifests retain binary/source/route hashes, adapter, submission identities, phases and exposure. All 812 PNG sources were decoded and verified during packaging, and movie frame counts match. The source root is the committed showcase package at `70d2baf8…e21d`. Binary SHA-256 is `f396ef6c1034ecdc5124a023b762a2ea16bc73abc925bc26371b48f9004f5af1`. [Toolchain](toolchain.txt)

The repair adds a CPU sampling choice with the existing GPU buffers and passes. Native lint and WebAssembly checks pass. Independent review remains owed. S1 saturation remains visible in fast motion views. S2–S5 retain their review follow-ups for reactive validity, volume alpha, dynamic-scale mip bias and cut capture. S6’s spatial path now shares the unjittered projection with clusters and shadows. [Advisories](../M1-review.md#suggestions)
