# G3s first light — open

6 November, M3 Max Metal. Production shaders unchanged; diagnostics at `784102d`/`91ad52d`. No spectral reference data imported. These probes select neither exposure nor transport.

| Row | Target | Observation | Status |
| --- | ---: | --- | --- |
| Sky + LUT + aerial GPU, 1080p | Recorded figure; tripwire | Complete row unavailable; loaded compute p50/p95 0.257/2.510–0.403/3.960 ms | Open |
| Spectral radiance | declared oracle tolerance | Independent spectral oracle absent | Open |
| Twilight and sweep | visible, no banding/flicker | Dark geometry, quantization and grid moiré | Partial review |

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [1080p](g3s-first-light/sun-facing-1080p.png) · [96-sample sweep](g3s-first-light/solar-sweep.mp4)

Game Sun samples drive Hillaire-style LUTs and lights. Review of 96 frames found no reversal at six sky points. Shared GPU/Blender load prevents a speed claim; raw exposure A/B logs remain `tmp/g3s-exposure-{a1,b1,a2}.log`. Frame CPU belongs to [G3b evidence](../render/V1-BOUNDED-FRAMES-2026-11-04.md).

**Half storage:** fixed equations/grid, paired pre-store f32/raw/scaled-half. At −9°, sky storage alone changes sampled illuminance 0.296578→0.308025 lux (+3.86%); −6° +0.120%; −3° unchanged. Canonical controls and repeated sidecars match. Same-equation precision isolation, not physical accuracy; `tmp/g3s-precision/`.

**Neutral target:** 18% rough nonmetal, no local lights/shadows. Corrected front-face HDR Y at −3°/−6°/−9°: 9.8875/0.44967/0.016948 cd/m². Exposure brackets preserve HDR bitwise; COPY_SRC instrumentation preserves displayed pixels. Nonzero −9° half subnormals become black after display mapping. Isotropic ambient erases normal contrast; exposure cannot restore it. Review excluded the original mislocated cube ROI and finite-ground edge; `tmp/g3s-neutral/`.

Current-LUT CPU luminance quadrature makes SH9 worth revisiting, but at −9° it still overstates upward irradiance by 26.8% and leaks downward. No technique selected: RGB, receiver-normal error, matched visuals, cost and transport accuracy remain open.
