# G3s first light — open

6 November, M3 Max Metal. Shaders after `04d0d7b`, diagnostics at `784102d`; `./dev check` passed. **Not qualified:** spectral accuracy, atmosphere GPU budget and twilight scene lighting remain open. Display timing belongs to G3b.

![Fixed solar cameras](g3s-first-light/solar-contact.png) · [Sun-facing 1080p still](g3s-first-light/sun-facing-1080p.png) · [96-sample solar sweep](g3s-first-light/solar-sweep.mp4)

The game selects Sun time; Hillaire-style LUTs light sky and surfaces. Independent review of 96 frames found no reversal at six sky points; dark geometry and grid moiré remain.

**Storage isolation:** unchanged equations/grid/transmittance, paired pre-store GPU f32 and raw/scaled-half readbacks at −3°, −6°, −9°. At −9°, changing only sky storage raises GPU-sampled horizontal illuminance 0.296578→0.308025 lux (+3.86%); at −6°, +0.120%; at −3°, unchanged. Canonical stores match, and repeated −6° sidecars match exactly. Independent review corrected half-format counters on a f32 comparison. This isolates representation loss, not physical accuracy; CPU reference interpolation and host summation differ from the production reduction. No encoding change was promoted. Raw data, gains, source hashes and fixed-exposure brackets remain in `tmp/g3s-precision/`. Next: separate exposure from transport; do not conceal the latter with an ambient multiplier.

| Row | Target | Current observation | Status |
| --- | ---: | --- | --- |
| Sky + LUT + aerial GPU, 1080p | p95 ≤0.3 ms | Separate complete row unavailable; atmosphere compute p50/p95 0.257/2.510 to 0.403/3.960 ms in two loaded runs | Open |
| Whole game render-thread CPU, 1080p | G3b p95 ≤0.3 ms | Candidate A/B/A p95 1.219/1.361/1.206 ms; fixed-exposure control is B | Open; offscreen, shared load |
| Twilight and sweep | visible, no banding/flicker | Visible sky gradient; no sampled flash; dark scene remains | Partial agent review |
| Spectral sky radiance | declared oracle tolerance | Oracle not yet built/qualified | Open |

Shared GPU/Blender load; interleaved logs: `tmp/g3s-exposure-{a1,b1,a2}.log`. GPU variation prevents a performance claim.
