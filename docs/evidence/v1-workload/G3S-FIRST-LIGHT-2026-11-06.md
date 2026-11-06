# G3s first light — open

6 November, M3 Max Metal. Partial functional/visual checkpoint. No imported spectral data or new speed claim.

| Row | Evidence | Status |
| --- | --- | --- |
| Storage/units | Six-angle LUT readings finite; known πL × Sun diffuse regression | Scoped pass |
| Incident camera | GPU calibration/history/cancellation/resize and adjacent-tick probes | Pass |
| SDR shoulder | GPU f64 reference within 1 byte; old default pixels bitwise exact; full `./dev check` | Pass |
| Transport | Independent f64 quadrature/all-orders CPU; 198 matched-medium Metal rays | Disagreements open |
| Visible/live | Readable twilight; pastel contrast/grid aliasing open; live run got 0 display-link callbacks | Partial |

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [1080p setting Sun](g3s-first-light/sun-facing-1080p.png) · [96-frame solar sweep](g3s-first-light/solar-sweep.mp4)

The test-only referee independently integrates declared coefficient bands, not measured Earth spectra/CIE/lux.
198 fixed rays, 32768 paths each (8 batches), match the shipping GPU sampler. 18 rows have zero samples and 41
have sparse batches: deep-night estimates remain unresolved. Single-scatter quadrature exposes horizon mixing
(at Sun 0°, green just below/above the horizon: GPU/reference 9.30/0.36). At Sun −3°, ground −15°, full GPU
radiance is 5.51e−8 versus CPU 3.15e−5 ±1.49e−6 batch SE, indicating missing indirect ground light. Units L/E0 sr⁻¹;
no accuracy pass. Analytic transport, phase/roulette/ground and refinement falsifiers pass independent review.
LUT gains 65536/1024 preserve twilight; C250 incident metering uses no readback and follows submitted history.

The game explicitly selects `NeutralHighlights`; diagnostic `Reinhard` remains default. Khronos highlight
compression/desaturation omits its material shadow toe and is not standard PBR Neutral. No pass/resource is added;
scene radiance and exposure are unchanged. GPU ramps cover extreme gains. Clear-sky side pixels with a 255 channel fall from 36.6% to 0% at twilight; Sun-facing from 100% to 5.85%.
This is headroom, not a physical-accuracy metric. Root/independent review find smooth gradients and compact Sun/halo;
the brighter pastel look and photographic contrast stay open. Keep calibration and resolve physics before grading.

Q/E turns, R/F tilts, brackets change time. The eight-second day sweep cannot qualify real-time cadence. Source
hashes, raw frames/readings and logs: `tmp/g3s-storage/`, `tmp/g3s-exposure/`, `tmp/g3s-shoulder/`, `tmp/g3s-reference/`.
