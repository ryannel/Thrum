# G3s first light — native valley and portable Sky

**Partial.** The native valley uses eight-wavelength transport for Sky, Sun and aerial in-scatter. Chrome uses
the same atmosphere and shadow shaders in the diagnostic scene. [Build plan](../../plans/BRIEF.md).

| Check | Result and limit |
| --- | --- |
| Direct Sun | Metal matches CPU within `0.05T + 0.005` at 47 receiver cases repeated after translation. The 1,500 m receiver stays lit after the 245 m camera horizon. Above-shell entry, miss and planet hit pass. Near-ground RGB bounds are 0.6/0.9/1.6% with a 0.001 denominator floor. |
| Twilight M chart | 512 vs 1024 columns at 102 Sun/height cases. Both spectral banks run through the same estimator and RGB projection. |
| Storage / indirect | Scaled aerial half RGB against fp32 plus independent SH9, GGX/DFG, cube seams and energy probes. SH can err by 331% relative on faint directions. Exposure uses the direct horizontal meter. |
| Cooked shadows | Metal shows a caster 1.4 km from its receiver outside the camera frustum. Both cascades use the selected asset set after upload. Discard, capture, swap and recovery remain covered by cooked integration tests. |

![Native held phases at dawn, sunrise, noon, sunset, twilight and night](g3s-first-light/solar-contact.png)

[24 sampled views at 5 fps](g3s-first-light/solar-sweep.mp4). Native offscreen render at 1280×720 before clip encoding,
245 m camera, frozen authority and 61,312 cooked triangles. Cook `examples/v1-game/assets/showcase` with
`thrum-cook`, set `THRUM_ASSET_STORE` and `THRUM_SCENE=valley`, then use `THRUM_CAPTURE_SCENE=<newdir>`.

Review sees blue upper twilight and a warm horizon across sky, haze and reflections, closer to the CC0
Kiara photographs. The foreground remains flat and bright, and the clear Sun crosses the ridge in 97 added half-minute samples.
[The crossing](g3s-first-light/solar-crossing.mp4) exposed a 103-code foreground shadow step; the [finite-Sun penumbra](G3S-SOLAR-PENUMBRA-2026-11-07.md) replaces it with a gradual crossing, and [terrain horizon shadows](G3S-TERRAIN-HORIZON-2026-11-07.md) shade the distant valley at sunset. No new all-black frames in 32 comparisons. The sole-Sun night is black. [Kiara dusk](https://polyhaven.com/a/kiara_9_dusk).

Chrome 156 passes seven held phases and 24 samples at Wasm
`add91ac18cae0ad8a6ba885cfe4e778cea24726e0a854d219028395bc73275f8`. Live motion and showcase quality remain open.

**Frame history.** 10k entities, cooked valley at 1080p, 200 callbacks per offscreen Metal run.
ABBA on the shared M3 Max, load 10.98–11.46. Values in ms.

| Row | Three bands | Eight wavelengths |
| --- | ---: | ---: |
| Threaded callback p95 including poll | 1.062–1.113 | 1.018–1.111 |
| Atmosphere compute GPU p50 | 0.821–0.823 | 0.865–0.866 |
| Aerial GPU p50 | 0.048 | 0.069 |
| Scene GPU pass sum p50 / p95 | 1.657–1.659 / 2.708–2.804 | 1.712–1.714 / 2.885–2.933 |

Aerial trips the 25% watch line with two banks of T/M samples. Median scene cost rises ~3.3%.
Cold profile refresh and live cadence remain open. Raw logs: `tmp/g3s-showcase/q8-production/`.

**Spectrum.** Metal compared 4, 8 and 12 actual wavelengths through the same closure, with an
analytic ozone band through the existing anchors. Across 49 full-Sky samples, eight nodes differ from
5 nm integration of the same profile by at most 1.81% luminance and Δu′v′ 0.00636. Upper twilight R/G drops
from 1.82 to 0.82 before source-white normalization and exposure. Shipping Metal matches the upper-twilight
RGB oracle within 2.5% per channel and checks all eight M lanes. Earth radiance and continuous motion remain open.
[Environment design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains).
