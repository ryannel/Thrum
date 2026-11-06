# G3s first light — open

6 November, M3 Max Metal. Storage correction: this checkpoint; actual game captures, no imported spectral data.
Numeric performance targets are retired. Correctness, spectral tolerance and visible quality remain required.

| Row | Evidence | Status |
| --- | --- | --- |
| Half-float storage/units | Actual six-angle readbacks finite; production diffuse shader integrates known πL × Sun | Scoped pass |
| Spectral radiance | Independent CPU spectral oracle absent | Open |
| Sunset/twilight | No new band/flash; scene still nearly black, grid aliases | Open |
| GPU/CPU cost | [Recorded history](../render/V1-BOUNDED-FRAMES-2026-11-04.md); no new speed claim | Tripwire |

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [1080p Sun view](g3s-first-light/sun-facing-1080p.png) · [96-frame solar sweep](g3s-first-light/solar-sweep.mp4)

**Numerical correction:** fixed power-of-two encoding (multiple 65536, sky 1024), decoded at every consumer. Equations,
grids, formats, physical Sun and camera exposure are unchanged. Actual pipelines at 45°, 0°, −3°, −6°, −9°, −12°
have no nonfinite values; encoded maxima stay far below fp16 overflow. Horizontal illuminance at −9° changes
0.296578→0.309882 lux; −12° 0.018674→0.033424 lux. Daylight illuminance is unchanged. This preserves the model's
signal; it does not validate transport against Earth. Raw source hashes/probes: `tmp/g3s-storage/`.

Noon and Sun-facing images match the control bitwise; 80/96 sweep frames are identical and remaining channel changes
are at most 1/255. Root and independent review found no new visible artifact. The precision fix does **not** solve
the dark-camera problem. A neutral target previously proved nonzero HDR light beneath black display pixels; isotropic
ambient also erases surface-normal contrast. Exposure/metering, directional lighting and the spectral oracle remain.

`./dev check` passes, including Metal and Wasm; the known-radiance GPU units regression passes. Earlier probes remain in task scratch. Exposure and visible twilight work continue; no transport multiplier was added.
