# G3s first light — native valley and portable Sky

**Partial.** The native game uses real cooked meshes, a metre-scale camera and the same Sky as Chrome's diagnostic
scene. G3s still needs physical colour and showcase review. See [the build plan](../../BUILD-PLAN.md).

| Check | Result and limit |
| --- | --- |
| Direct Sun | Metal matches the CPU receiver oracle within `0.05T + 0.005`: 47 cases repeated after translation. A 1,500 m receiver stays lit after the 245 m camera horizon; rays above the shell enter it, miss it or hit the planet. Separate near-ground RGB bounds remain 0.6/0.9/1.6%. |
| Twilight M chart | 512 vs 1024 columns, 102 Sun/height cases: worst RGB E 0.296%, zenith 0.723%, antisolar 0.874%, local M 2.851%; same estimator only. |
| Storage / indirect | Scaled aerial half RGB against fp32; independent SH9, GGX/DFG, cube seams and energy probes. SH truncation can reach 331% relative on faint directions; exposure retains its direct horizontal meter. |
| Native lifecycle | Cache, cancellation, front-facing normal, camera recovery and cooked upload/history checks pass. |

![Native held phases: dawn, sunrise, noon; sunset, twilight, night](g3s-first-light/solar-contact.png)

[24 sampled views at 5 fps](g3s-first-light/solar-sweep.mp4): same FrameHost, Sky and cooked valley, 1280×720 offscreen
before clip encoding, frozen authority, 245 m camera, 61,312 cooked triangles. Cook `examples/v1-game/assets/showcase`
with `thrum-cook`, set `THRUM_ASSET_STORE` and `THRUM_SCENE=valley`, then run the game with
`THRUM_CAPTURE_SCENE=<newdir>`. The clip samples the route; live cadence is unqualified.

Root and independent reviews retain the source normals and receiver lighting. Compared with `a5f29b7`, all 32
sky strips (top 32 rows) and held dawn/twilight/night match byte for byte; sunrise/sunset differ by up to 10 codes.
Upper slopes brighten subtly. No new visible ribs or seams; broad facets, foreground seam, weak contact grounding
and magenta twilight remain. Sole-Sun night is black. Four CC0 photos informed qualitative review; they are not a
matched oracle. **Showcase readiness is rejected.**

Chrome 156 on the M3 Max passes seven Sky phases and 24 samples at Wasm
`cc458c034faadd5ab7d5ce9e4079519219243be8d7e4daf5fba5af7612927f08`. It still draws the diagnostic scene.
No physical display, continuous flicker, iPhone or Earth-spectrum qualification follows.

**Frame history.** Same 10k simulation, cooked valley, 1080p / 200 threaded callbacks, offscreen Metal. Repeat ABBA
on the shared M3 Max (load 9.0–9.3), all four runs submit 200/200. Ranges are two runs per variant.

| Row | Before | Receiver Sun |
| --- | ---: | ---: |
| Render thread p95 | 1.312–1.395 ms | 1.231–1.306 ms |
| Forward + Sky GPU p50 | 0.356–0.358 ms | 0.378–0.379 ms |
| Scene GPU pass sum p50 / p95 | 1.586–1.588 / 2.774–2.800 ms | 1.627–1.631 / 2.799–2.823 ms |

One candidate in the first ABBA (load 9.3–9.7) reached 7.205 ms scene GPU p95 and submitted 189/200; unchanged
shadow, cluster and atmosphere passes spiked too. The repeat did not reproduce it; cause unattributed. Receiver
lighting adds one T lookup per sunlit fragment with the resident LUT/sampler, no new pass or image. No CPU speed
or allocation claim. Earlier chart, aerial and GGX costs remain in Git.

[Design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains): one Sun, shared
storage gain, SH9/GGX and separate exposure. Spectral interpretation, local/isotropic scattering closure and
absolute radiance accuracy remain open. No empirical RGB gains or restricted reference data added. Exact input
hashes, paired runs, pixel comparisons and referees: `tmp/g3s-showcase/receiver-sun/`.
