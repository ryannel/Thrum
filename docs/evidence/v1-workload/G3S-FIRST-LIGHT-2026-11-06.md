# G3s first light — open

6 November, M3 Max Metal / Chrome 156 WebGPU. Partial function/visual checkpoint; no imported spectra or speed claim.

| Row | Evidence | Status |
| --- | --- | --- |
| Storage/camera/SDR | Six-angle LUTs, πL, meter/history, shoulder f64 within 1 byte | Scoped pass |
| Directional diffuse | Analytic/zero/colour/rotation/127000 scaling; eight actual solar/azimuth rows, 14 normals | Scoped pass |
| Projection/reference | Worst coefficient error 0.0645% M0; direct-reference refinement ≤0.0483% M0 on actual skies | Sampling pass |
| SH approximation | Worst direct error 5.225% M0; relative error up to 331% on faint directions | Accuracy open |
| Transport | 198 matched-medium rays; horizon and reflected planet boundary | Accuracy open |
| Look | Clearer twilight faces; noon silhouette, grid aliasing/pastel contrast remain | Partial |

![Six held Chrome solar phases](g3s-first-light/solar-contact.png) · [held-phase clip](g3s-first-light/solar-sweep.mp4) · [earlier native Sun](g3s-first-light/sun-facing-1080p.png)

Current contact/clip: served Wasm `fe6d60406d83f3d862df184cf8e497bb7d04f3e08d3f537842104615832f456e`,
Chrome 156.0.8078.81, ANGLE Metal M3 Max, 1252×704, world tick 2. Six held phases, **not cadence**.
Pause; scene/noon; Sunrise; Noon; Sunset; Twilight; scene view. Page presets own time/yaw/pitch;
scene view resets camera only. Clip repeats each captured phase for one second, explicit sRGB H.264;
decoded frames reviewed (grid/chroma compression remains). Code/recipe: 65° FOV, 18% rough neutral material,
one enlarged static entity. Root/independent review retain the distinct pink side/violet top/dark front at twilight.
No new broad clipping/banding; noon silhouette, grid moiré, jagged shadows and a one-pixel horizon step remain.
Sky-only upper-300-row sunrise/sunset/twilight comparison against `0ab189d`: peak delta 1 SDR byte.

[SH9 design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains) keeps
horizontal metering independent: 144-byte signed tail, 1024 additional cached LUT fetches, no new pass/readback.
Only final irradiance clamps; 64 normal probes report risk, not pixels. Flag16 cannot change gain/history.
The `3M0/32` kernel envelope is not measured accuracy. Independent midpoint refinement must pass first;
the narrow reference needed 256→512 without relaxing tolerance. Review corrected horizon sampling,
zenith/rotated Sun and solar scaling. Dark-direction SH errors remain; no whole-sky pass.
Browser normal forward+sky p50/p95 0.131/0.197 ms versus earlier 0.197/0.197: quantized, shared load, no speed win.
No new full-frame CPU/allocation or complete-atmosphere cost claim.

A sampled sunset run hit WebGPU loss/black output: **rejected as motion evidence**; loss reporting/authority now pass; reconstruction remains open.
Raw numerical/capture/failure logs: `tmp/g3s-directional/`; baseline `tmp/g3b-browser/`.

Transport uses declared bands, not measured Earth spectra/CIE. Nonlinear M layout `11cb21e` stays parked;
selected transport remains `e061671`, gains 65536/1024. A true-phase first-scattering seed was also rejected:
Sun−3° resolved-row median error worsened 14.34→17.24%; Sun0°/+1°/0° blue improved 3.11→0.25%, while
Sun−3°/+15°/180° green worsened 25.92→29.01%. Comparison holds all 198 first-order/CPU rows fixed against
retained 32768-path estimates; it is not a whole-sky tolerance pass. Raw patch/tables: `tmp/g3s-source-quality/`.
Independent continuous local-closure referee separates angular/path refinement from LUT sampling. Four cells agree within
0.25% on separate axes: angular 8→16 (fourth 16→32), path refinement at16. Its vacuum falsifier exposes inherited
self-feedback. A coupled scalar prototype fixes that boundary but remains mixed: green+25.9→+3.9%, blue−3.1→−10.1%; parked in `tmp/g3s-spatial-prototype/`.
C250 metering has no readback. NeutralHighlights preserves calibration; not standard PBR Neutral. Physical display
motion, actual tab/mobile lifecycle, spectral accuracy and final contrast remain open.
