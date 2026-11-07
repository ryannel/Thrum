# G3s — physical sky and native scene, open

7 November 2026. M3 Max Metal / Chrome 156. Function and scoped image checks pass; physical colour and showcase quality remain open.

| Check | Result and limit |
| --- | --- |
| LUT storage / SDR | Scaled half RGB; actual aerial producer vs fp32 and consumer units; πL, shoulder and history checks pass |
| Sun lookup | Production fp16 vs CPU; 20 near-ground cases: RGB error≤0.6/0.9/1.6%; first-row approximation retained |
| Twilight M chart |512 vs1024 columns; 102 Sun/height cases including chart phases: worst RGB E 0.296%, zenith 0.723%, antisolar 0.874%, local M 2.851%; same-model only |
| Diffuse / reflections | SH9 direct-LUT referee exposes truncation (5.225% M0,331% on faint directions); GGX cube/DFG independent hemisphere, seams and energy checks pass |
| Native authored scene | Same FrameHost/Sky/CAS; metre camera feeds actual altitude. Canonical G5 seam matches all 32 earlier frames byte-exactly |
| Terrain normals | Unequal-grid derivatives remove parity ribs; positions/UV/indices/placements unchanged, runtime mesh size unchanged. All 32 fixed upper-Sky ROIs remain byte-exact |
| Lifecycle | Cache/cancel/profile/altitude checks, front-facing normal regression, nonzero authored-camera GPU replacement and full `./dev check` pass |

![Native held phases: dawn, sunrise, noon; sunset, twilight, night](g3s-first-light/solar-contact.png)

[24 sampled frames, 5 fps](g3s-first-light/solar-sweep.mp4): exact canonical-seam game,1280×720 offscreen, frozen authority,
245 m camera; 61,312 cooked triangles. Eight held views plus 24 route samples; clip is a slideshow, **not live cadence**.
The sole-Sun model's night is black. Own generated glTF/texture sources and generator live under
`examples/v1-game/assets/showcase/`; cook them through `thrum-cook`, select `THRUM_SCENE=valley` with
`THRUM_ASSET_STORE`, and capture via the same game binary's `THRUM_CAPTURE_SCENE=<newdir>`.

Root and independent reviews retain the corrections. Fine vertical ribs disappear in actual GPU sunset/sequence
views; fixed ROI encoded-luma curvature0.2169→0.1654 is descriptive, not a quality tolerance. Broader slope facets,
foreground seam, flat foreground, weak grounding/contact shadows and uniformly pastel twilight remain. Four CC0
photographs were inspected as unmatched qualitative references. **Showcase readiness is rejected.**

Chrome's same-Sky route passes seven phases/24 samples at Wasm
`7476f65a9e3b7331a396dcbaea348b15d32885f7e5d24102a2af5aebbc1400ff`; prior 11 function cases remain historical.
Chrome still draws the diagnostic grid, not this native authored scene. No physical-display, continuous flicker,
iPhone or Earth-spectrum qualification follows.

**Frame history, no speed win.** ABBA, 10k simulation entities,1080p / 200 threaded callbacks; ranges are two runs.

| Change / shared load | Callback incl. GPU poll p95 before→after | Scene GPU pass-sum p50 before→after |
| --- | --- | --- |
| Sun/M lookups, load 20–21 |2.814–5.892→2.467–4.268ms |1.584–1.586→1.588ms |
| Normals only, one canonical binary, load 6–7 |0.780–1.001→0.814–1.060ms |1.582–1.590→1.578ms |

No other owned GPU work overlapped these blocks; shared work varies. Lookup atmosphere-compute p95
0.830–0.834→0.834–0.835ms. M grows 8→128 KiB and costs10,485,760 volume evaluations per profile refresh;
steady transport retains one M fetch/step. GGX costs~4.19 M fetches/changed Sky plus ~0.30 MiB products; its earlier
shared-load cost/tripwire remains in Git. Normals are O(V) offline; no extra runtime operation/buffer or allocation claim.

[Design](../../design/modules/ENVIRONMENT.md#selected-mathematical-relationships-and-domains): one Sun, shared
storage gain, SH9/GGX and separate exposure. Open physical work: spectral/band interpretation, local/isotropic
multiple scattering, SH/meter approximation and declared radiance tolerance. No empirical RGB or exposure gain
was added to hide these. Raw source identities, GPU oracles, images and paired logs: `tmp/g3s-showcase/`;
earlier reflection/transport work stays in `tmp/g3s-environment-reflections/` and Git. No GPL inputs imported.
