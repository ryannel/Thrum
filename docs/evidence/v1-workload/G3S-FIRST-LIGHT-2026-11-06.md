# G3s first light — native valley and portable Sky

**Partial.** The native cooked valley has physical Sky, Sun and contact shadows. Chrome uses the same Sky and
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

Root and independent reviews see attached contact shadows and their changing direction. The initial terrain
self-shadowing was corrected with receiver-plane depth per PCF tap and bounded footprint tolerance. Three confirmed
false-shadow samples in low-Sun views recovered 43, 58 and 40 red codes to match `8fb8e98`. Local seam marks, hard
edges, rock speckling and magenta twilight remain. The top 32 rows of all 32 captures and held dawn, twilight and
night match `8fb8e98` byte for byte. The sole-Sun night is black. Showcase quality and continuous motion remain open.
[Receiver-plane technique](https://learn.microsoft.com/en-us/windows/win32/dxtecharts/cascaded-shadow-maps).

Chrome 156 on the M3 Max passes seven held phases and 24 samples using Wasm
`f0cbc995662d726f11cdc66da96e5650c76bdeeff6f17c057eea752285e5b6bb`. Device and live cadence evidence remain open.

**Frame history.** 10k simulation with cooked valley at 1080p and 200 threaded callbacks, offscreen Metal. Two runs
per variant in ABBA on the shared M3 Max (load 10.4–13.7). Baseline submits 198–199 frames and candidate 199–200.

| Row | 8fb8e98 | Cooked casters |
| --- | ---: | ---: |
| Render thread p95 | 1.216–4.782 ms | 1.216–2.696 ms |
| Both shadows GPU p50 | 0.077 ms | 0.096 ms |
| Forward + Sky GPU p50 | 0.379–0.380 ms | 0.383–0.384 ms |
| Scene GPU pass sum p50 / p95 | 1.631–1.634 / 2.590–2.801 ms | 1.663–1.665 / 2.662–2.828 ms |

The slice submits about 123k extra depth-only triangles in the existing two maps. Their receiver range is 512 m
beyond near for long cameras and their depth covers upstream casters. No attributable CPU win. Raw timing, captures
and input hashes are under `tmp/g3s-showcase/cooked-shadows/`. Earlier costs remain in Git.

[Environment design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains). Spectral
interpretation, higher scattering orders and absolute radiance accuracy remain open. Restricted spectral reference
data stays in the ignored cache.
