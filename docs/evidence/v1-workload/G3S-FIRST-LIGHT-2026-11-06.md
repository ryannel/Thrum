# G3s first light — open

6 November, M3 Max Metal / Chrome 156 WebGPU. Partial function/visual checkpoint; no imported spectra or speed claim.

| Row | Evidence | Status |
| --- | --- | --- |
| Storage/units | Six-angle finite LUTs; πL × Sun diffuse regression | Scoped pass |
| Camera/SDR | GPU meter/history/cancellation/resize; f64 shoulder within 1 byte | Pass |
| Transport | 198 matched-medium rays; horizon and reflected planet boundary | Accuracy open |
| Chrome controls | Solar presets/sliders, camera, live-render pause; six function rows | Pass, headless |
| Look | Readable twilight; neutral subject reveals flat ambient; grid moiré/pastel contrast | Open |

![Six held Chrome solar phases](g3s-first-light/solar-contact.png) · [held-phase clip](g3s-first-light/solar-sweep.mp4) · [earlier native 1080p Sun](g3s-first-light/sun-facing-1080p.png)

Current contact/clip: actual served Wasm `0816025e4d84f6bd602bda124cfe5a5dbb7c56866d0b217879ef842b98affa86`,
Chrome 156.0.8078.81, ANGLE Metal M3 Max, 1252×704, world tick 4. Six held phases, **not motion/cadence**.
Pause, then scene/noon; Sunrise; Noon; Sunset; Twilight; scene view. Presets encode time/yaw/pitch in the page;
scene view resets camera only. The clip repeats each captured phase for one second; decoded frames reviewed.
Source: game scene/control code; 65° FOV, 18% rough neutral material and one enlarged existing static entity.
Root/independent review finds flat faces, grid aliasing and a one-pixel horizon step; none is showcase quality.
PNG checks: no pure-black pixels; twilight has 7.1% clipped-channel pixels. Explicit-sRGB H.264 adds grid chroma
artifacts (sky-only mean byte error <0.62). Keep calibration; address directional diffuse before contrast grading.
Actual tab visibility, device loss and physical display intervals are unqualified.

The CPU referee integrates declared coefficient bands, not measured Earth spectra/CIE/lux. All 198 first-order
values stay exact. At Sun 0°, green single-scatter below/above curved horizon changed 9.30/0.36→1.04/0.99.
Metal two-colour tests prohibit cross-half filtering at 0/2/100 km. Tangent/pole/zenith/azimuth and ABI checks pass.
Nonlinear M sampling `11cb21e` is parked: one sky error grew 3.11%→23.00%; retain `e061671`. No whole-sky pass.
GroundE adds reflected sky at the actual planet hit: 1 KiB logical texels, one cooperative profile kernel, no new
ordinary-frame kernel. πL/zero/scaling/terminator/cache tests pass; 144 sky-side full values stay exact.
Ground median sampled-MC disagreement at Sun 45°/0° changed 6.98%/92.09%→0.24%/3.56%; −3°/−6° remains
16.71%/22.16%, with adverse horizon rows (−6° blue +188.68%). 32768 paths/ray leave 18 zero/41 sparse unresolved.

LUT gains 65536/1024 preserve twilight. C250 metering has no readback and follows submitted history.
Game `NeutralHighlights` uses Khronos highlight compression/desaturation, omitting its material toe; not standard
PBR Neutral. No new pass; calibration is unchanged. Earlier twilight side/Sun-facing clipped-channel fractions fell
36.6%→0% / 100%→5.85%: headroom, not physical accuracy. Spectral accuracy/model error remain open.
Native Q/E turns, R/F tilts, brackets edit time; Chrome exposes sliders/presets. Raw: `tmp/g3s-{storage,exposure,shoulder,reference,horizon,ground}/`, `tmp/g3b-browser/`, `tmp/g3b-browser-six-qualified/`.
