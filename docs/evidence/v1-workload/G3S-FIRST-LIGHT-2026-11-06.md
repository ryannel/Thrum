# G3s first light — open

6 November, M3 Max Metal. Partial functional/visual checkpoint. No imported spectral data or new speed claim.

| Row | Evidence | Status |
| --- | --- | --- |
| Storage/units | Six-angle LUT readings finite; known πL × Sun diffuse regression | Scoped pass |
| Incident camera | GPU calibration/history/cancellation/resize and adjacent-tick probes | Pass |
| SDR shoulder | GPU f64 reference within 1 byte; old default pixels bitwise exact; full `./dev check` | Pass |
| Spectral radiance | Independent all-orders CPU spectral oracle absent | Open |
| Visible/live | Readable twilight; pastel contrast/grid aliasing open; live run got 0 display-link callbacks | Partial |

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [1080p setting Sun](g3s-first-light/sun-facing-1080p.png) · [96-frame solar sweep](g3s-first-light/solar-sweep.mp4)

LUT gains 65536/1024 preserve faint radiance without changing transport: −9° horizontal light changed
0.296578→0.309882 lux; −12° 0.018674→0.033424 lux; daylight is unchanged.
The physical horizontal incident meter uses C250, authored EV0–18 and no runtime readback; fixed reciprocal bits
remain exact. One GPU resolve follows actual submitted-history sides.

The game explicitly selects `NeutralHighlights`; diagnostic `Reinhard` remains default. Khronos highlight
compression/desaturation omits its material shadow toe and is not standard PBR Neutral. No pass/resource is added;
scene radiance and exposure are unchanged. GPU ramps cover extreme gains. Clear-sky side pixels with a 255 channel fall from 36.6% to 0% at twilight; Sun-facing from 100% to 5.85%.
This is headroom, not a physical-accuracy metric. Root/independent review find smooth gradients and compact Sun/halo;
the brighter pastel look and photographic contrast stay open. Keep calibration and resolve physics before grading.

Q/E turns, R/F tilts, brackets change time. The eight-second day sweep cannot qualify real-time cadence. Source
hashes, raw frames/readings and logs: `tmp/g3s-storage/`, `tmp/g3s-exposure/`, `tmp/g3s-shoulder/`.
