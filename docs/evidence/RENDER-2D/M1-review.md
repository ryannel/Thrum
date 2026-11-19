# RENDER-2D M1 review

**Fail. Ten items pass. M1.3 fails on the named-refusal requirement.** All four Must Fix findings from the previous review are resolved.

## Must Fix · M1.3 · verified

**Sprite startup omits the rejected 3D content setting from its error.** On the Mac I launched the rebuilt V1 fixed checkpoint with `THRUM_SCENE=sprites THRUM_CLOUDS=sky`. It exited 1 with `capture failed: 2D scene refuses 3D content settings`. A game maker needs the rejected setting or feature named, as the milestone's quality bar requires. On the same binary, `THRUM_TRACED_VIEW=on` and `THRUM_LAMP_SHADOWS=on` each produced a refusal naming the requested feature.

The native sprite branch groups clouds with stars, meshes, route overlays and other settings under one error. In Chrome I opened the V1 page with `?scene=sprites` and added `&assets=http://127.0.0.1/cooked`, `&field` and `&diagnostic-clouds` separately. All three displayed `Startup fault: Error: 2D scene refuses 3D content settings`. The control `&culling=frustum` identified the camera culling mode and its value. The cooked-assets case matters independently because the design explicitly requires a cooked placement to be refused by name. Preserve the specific request at each launch boundary and check the diagnostic a game maker sees. I tried individual settings. Precedence among simultaneous conflicts was outside this review. [Native host](../../../examples/v1-game/src/host.rs) · [Browser startup](../../../examples/v1-game/web/index.html) · [Scene-kind contract](../../plans/RENDER-2D/DESIGN.md#1-the-scene-kind)

## Item decisions

I used the script's passing project and item checks on the files at `3a6ce80630c4`. I built the V1 game at review commit `02a06ed46672` for the Mac captures and startup probes.

| Item | Verdict | Evidence and judgment |
|---|---|---|
| M1.1 | Pass | The CPU oracle places a six-colour asymmetric 3×2 sheet by inverse integer coordinates, including four clipped copies and one central copy. It compares every SDR byte at 64×46 and 65×47, on portable and Apple tile, at scales 1, 2, 3, 4 and 6. A flip changes the expected bytes. [Exact-pixel test](../../../crates/thrum-render/tests/sprite_exact.rs) |
| M1.2 | Pass | The literal ordinary graph contains sprites, display-referred tone encode and present. Capture adds one readback. The test compares both whole lists and the ordinary completed-frame timing list. Across preparation, the HAL reports nine additional buffers, five textures and two query sets. The named storage and creation ledger account for the sprite inputs, outputs, timers and three render pipelines. The 3D control resolves TAAU and exhibits sky, stars, shadows, clusters, visibility, opaque and reconstruction passes plus depth, motion and reactive storage. [Graph and allocation test](../../../crates/thrum-render/tests/scene_2d.rs) |
| M1.3 | Fail | Engine tests cover shadow, opaque, after-opaque, temporal sampling, all three explicit culling modes and both named reconstruction choices. After refusal they check the existing scene kind, both graphs and its submitted pixels. A legal upload/simulate contribution clears a buffer, dispatches compute before sprites and receives its submission callback. The normal launch diagnostics have the defect above. [Admission and callback tests](../../../crates/thrum-render/tests/scene_2d.rs) |
| M1.4 | Pass | The original reference drivers ran against starting main `4392b652fb63`. The engine covers 64 combinations and the V1 driver prepares 11 actual game variants on each recipe, including cooked exit, valley and cloud setups. I compared the initial fixtures at `b31c96190` with the reviewed ones. Two independent changes updated graph records for CLOUDS noise textures (`8067b526e`) and separate light buffers (`0f8ef0861`). The pass sequences and transient slot assignments are preserved. The tests read checked-in expectations, and the sprite fix leaves them intact. [Engine driver](../../../crates/thrum-render/tests/support/sprite_graph_snapshot.rs) · [Game driver](../../../examples/v1-game/tests/sprite_scene_3d_control.rs) |
| M1.5 | Pass | Both Metal recipes read the ordinary submitted SDR and scene-linear taps. All 256 opaque grey codes and seven saturated colours require exact equality. Only the half-alpha RGB comparison allows one code. Red reads exactly 2.0 before encoding and clips to 255 in SDR with green and blue preserved. The two 3D tone operators and dither retain their arithmetic, including the extracted sRGB function. [Colour test](../../../crates/thrum-render/tests/sprite_colour.rs) |
| M1.6 | Pass | The real host resizes, doubles the backing extent, skips an encoded frame, refuses a mismatched drawable, captures an endpoint and publishes a replacement device. The test compares the entire next submitted image with a newly prepared host after each event on both recipes. It destroys the old device, rejects its outstanding capture and verifies generation 2. [Lifecycle test](../../../crates/thrum-render/tests/scene_2d_lifecycle.rs) |
| M1.7 | Pass | Chrome's checker reads presented SDR submissions at five scales, compares every channel with the CPU placement of the authored sheet and checks the entire graph and absent reconstructor. At scale 4 it resizes the canvas to 1121×657, destroys the actual WebGPU device and uses the restore control before testing generation 2. [Browser check](../../../scripts/check_sprites_browser.mjs) |
| M1.8 | Pass | All four reviewer captures meet the inspection list below. Each card has its full border and marks. Every 2×2 or 4×4 block is uniform and the surrounding pixels are black. [Capture receipts](M1-review/capture-record.json) |
| M1.9 | Pass | Both retained release takes identify native build `c3d176f20ee3`, the binary hash, compiler and machine load. Each has 90 completed samples per extent with separate sprite, tone and present times and all target sizes. Both valley sides use the portable release bench at 1920×1080 on the same moving route, with 1,083 paired tick/mode entries. Median changes were +2.0%, +0.5% and +0.6% for Direct, Frustum and Occlusion. Missing target bytes fail validation. Timing excursions are reported and a complete record still passes. [Metal measurements](M1/cost-mac-metal.json.gz) |
| M1.10 | Pass | Two release Wasm takes at `a68d43950340` retain build identity, distinct start times, before/after load and 90 samples per extent. The device reports 16 sampled textures per stage, 256 array layers and eight storage buffers per stage. At full backing size the sprite median crosses one timestamp step from 0 to 65,536 ns in both takes. The verifier flags both observations. Removing a load observation fails validation. [Chrome measurements](M1/cost-chrome-webgpu.json.gz) |
| M1.11 | Pass | The shared main-branch check `exact-images` covered the tested files. The item command queries that result. The sprite work changes no registered image baseline. |

## Mac inspection

Before capture I listed the whole border, black surround, orientation of both corner marks, cream arrow and aqua cross, swatch order, grey and RGB ramps, fine vertical and horizontal lines, and the checker. I inspected each whole fixed SDR submission and each native-size card crop. The gold mark opens down and right at the upper left, the arrow points up, the cross sits at the upper right, and the aqua corner closes the lower right. The ramp channels stay distinct and the finest alternating lines and checks keep hard square edges in every crop. I also checked block uniformity across the entire frame and sampled authored colours at the marks and ramps.

| Fixed output | Scale | Frame and native-size card |
|---|---:|---|
| 1280×720 | 2 | [Frame](M1-review/1280x720-2.png) · [Card](M1-review/1280x720-2-card.png) |
| 1280×720 | 4 | [Frame](M1-review/1280x720-4.png) · [Card](M1-review/1280x720-4-card.png) |
| 3456×2234 | 2 | [Frame](M1-review/3456x2234-2.png) · [Card](M1-review/3456x2234-2-card.png) |
| 3456×2234 | 4 | [Frame](M1-review/3456x2234-4.png) · [Card](M1-review/3456x2234-4-card.png) |

The fixed inspection covers the portable Metal recipe on the M3 Max. The automated pixel and lifecycle checks cover both native recipes. Low sun, storm and distance require 3D scenery. Fast-camera motion, physical-panel cadence and simulator rotation are outside this static-card milestone.

## Previous findings

- **Traced-geometry view — resolved.** The native checkpoint exits 1 and names the traced view. The new M1.3 finding concerns a different request and a different failure in the diagnostic.
- **Displaced contribution camera — resolved.** Both upload and simulate read the frame's actual centre (10, 5). Their world-to-clip matrix projects (13, 3) to pixel (38, 28), where the submitted image contains the sprite. The implementation derives the transform from the view used to upload those rows.
- **Metal target byte counts — resolved.** Removing `present_bytes` from a retained Metal row in memory made the production validator reject the record. It checks both slot sizes and the present target against the extent.
- **Chrome load observations — resolved.** Replacing one retained after-load observation with null in memory made the production validator reject the record. It requires both observations and the release build identity.
