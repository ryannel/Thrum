# G3s first light — native valley and portable Sky

**Partial.** The native valley has physical Sky, Sun and contact shadows. Chrome uses the same Sky and
shadow shader in the diagnostic scene. [Build plan](../../BUILD-PLAN.md).

| Check | Result and limit |
| --- | --- |
| Direct Sun | Metal matches CPU within `0.05T + 0.005` at 47 receiver cases repeated after translation. The 1,500 m receiver stays lit after the 245 m camera horizon. Above-shell entry, miss and planet hit pass. Near-ground RGB bounds are 0.6/0.9/1.6%. |
| Twilight M chart | 512 vs 1024 columns at 102 Sun/height cases. Worst RGB E 0.296%, zenith 0.723%, antisolar 0.874%, local M 2.851% with the same estimator. |
| Storage / indirect | Scaled aerial half RGB against fp32 plus independent SH9, GGX/DFG, cube seams and energy probes. SH can err by 331% relative on faint directions. Exposure uses the direct horizontal meter. |
| Cooked shadows | Metal shows a caster 1.4 km from its receiver outside the camera frustum. Both cascades use the selected asset set after upload. Discard, capture, swap and recovery remain covered by cooked integration tests. |

![Native held phases at dawn, sunrise, noon, sunset, twilight and night](g3s-first-light/solar-contact.png)

[24 sampled views at 5 fps](g3s-first-light/solar-sweep.mp4). Native offscreen render at 1280×720 before clip encoding,
245 m camera, frozen authority and 61,312 cooked triangles. Cook `examples/v1-game/assets/showcase` with
`thrum-cook`, set `THRUM_ASSET_STORE` and `THRUM_SCENE=valley`, then use `THRUM_CAPTURE_SCENE=<newdir>`.

Review sees attached contact shadows and the expected direction changes. Receiver-plane PCF recovered three
false-shadow samples to their baseline RGB. Local seam marks and hard edges remain. All 32 sky strips and held
dawn, twilight and night match the previous build. The sole-Sun night is black. [PCF reference](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/cascaded-shadow-maps).

Chrome 156 passes seven held phases and 24 samples at Wasm
`f0cbc995662d726f11cdc66da96e5650c76bdeeff6f17c057eea752285e5b6bb`. Live motion and showcase quality remain open.

**Frame history.** 10k simulation with cooked valley at 1080p and 200 threaded callbacks, offscreen Metal. Two runs
per variant in ABBA on the shared M3 Max (load 10.4–13.7). Baseline submits 198–199 frames and candidate 199–200.

| Row | 8fb8e98 | Cooked casters |
| --- | ---: | ---: |
| Render thread p95 | 1.216–4.782 ms | 1.216–2.696 ms |
| Both shadows GPU p50 | 0.077 ms | 0.096 ms |
| Forward + Sky GPU p50 | 0.379–0.380 ms | 0.383–0.384 ms |
| Scene GPU pass sum p50 / p95 | 1.631–1.634 / 2.590–2.801 ms | 1.663–1.665 / 2.662–2.828 ms |

About 123k extra depth-only triangles reuse the two maps. Receiver reach is 512 m for long views. Caster depth
includes upstream meshes. CPU attribution is inconclusive. Raw inputs, captures and logs: `tmp/g3s-showcase/cooked-shadows/`.

**Spectral candidate.** Metal transported 4, 8 and 12 actual wavelengths through the same closure, with an
analytic ozone band through the existing anchors. Across 49 full-Sky samples, eight nodes differ from
5 nm integration of the same profile by at most 1.81% luminance and Δu′v′ 0.00636. Upper twilight R/G drops
from 1.82 to 0.82 before exposure. Integration, scattering closure and Earth colour qualification remain open.
Restricted reference data and reproducible probe inputs stay under `tmp/g3s-showcase/spectral-transport/`.
[Environment design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains).
