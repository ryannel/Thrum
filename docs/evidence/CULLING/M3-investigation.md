# AppleTile valley mesh oracle: retained failure

The clean `25ac378dfb0416b384533dd2f3d6d8cf40b19108` run of
`python3 scripts/run_v1_showcase.py culling --meshes --target mac-metal --no-build`
passed all 361 submitted route frames on Portable, then failed on AppleTile at
world tick 208 (route tick 28). The actual oracle test executable was
`target/release/deps/valley_culling_gpu-3358ff9ee4447fb0`, SHA-256
`2301dc2b6ed65038173e5cc69e48659bb9ff74eab90c29254f486379ff88de03`.
The run used the route's cooked root
`70d2baf823d09d85574f7cdfce7102648754544dd7d68659e9bd92dd5f30e21d`.

Exactly one of 921,600 pixels differs, at `(332, 202)` in the distant terrain:
Direct RGBA is `(177, 171, 160, 255)`; both Frustum and Occlusion are
`(176, 170, 160, 255)`. The maximum channel difference is one, and two bytes
differ. Frustum and Occlusion agree with each other. Both reported 11 drawn
slots of 10,000 at that tick; Occlusion's prepass also contained 11.

Full failure bytes, PNGs and rows remain locally under
`tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-172212/apple-tile/`.
The builder inspected the whole Direct frame; the numerical difference is on
the ridge, not a visibly missing prop. Its small size does not pass byte equality.

The original run did not retain exposure words or an independent Direct
control, so it does not establish a cause. The current oracle adds a second
independently prepared Direct host, retains exposure/submission/alpha receipts,
and can inspect the scene-linear stage. These controls preserve every route
frame and the original Direct comparison. They have not yet explained this
failure's root cause. No tolerance, retained baseline or oracle draw order has changed.

Two initial diagnostic invocations failed before rendering because their
fixture paths were relative to the workspace while Cargo starts tests in the
package directory. Corrected invocations use absolute paths; their logs are
`tmp/culling-m3/diagnostic-{sdr,linear}-fixed.log`.

## Independent Direct controls

On clean `8ce7110b0e2e8b726a9498b8839dd2854c772668`, the SDR control run reached
tick 413. Frustum, Occlusion and the first Direct host are byte-equal there;
the second Direct host differs at four pixels, by at most one SDR channel step.
Both Direct hosts report alpha bits `1056964609` and exposure words
`[981467136,1159023168,1093621713,1167819439,0,413,0,1]`.
Removing instances is therefore not necessary for an observed failure.

The separate scene-linear control reaches tick 298 and differs at 35 pixels.
Frustum, Occlusion and the second Direct host agree; the first Direct host is
the outlier. All four report alpha bits `1056964608` and exposure words
`[981467136,1163879286,1094231394,1172877893,0,298,0,1]`.
This witness precedes tone mapping and does not come from different recorded
exposure or phase. The original tick-208 failure remains individually unattributed.

The two independent Direct images from each control are now retained losslessly
under [M3/diagnostics](M3/diagnostics/direct-controls.json), with raw-byte hashes,
the exact differing coordinates and same-tick receipts. The
[comparer](M3/compare_diagnostic.py) reads the compressed raw bytes directly;
use `--pixel-bytes 8` for the scene-linear control. SDR differs at four pixels;
scene-linear differs at 35. These are witnesses of instability, not passing
culling evidence.

After rebasing onto CONTENT-LOOP M1, a clean Portable run at `7dbfb781` fails
at tick 200: Frustum differs at five pixels; both Direct hosts and Occlusion
agree. Its retained report is
`tmp/g7-showcase/runs/culling-mac-metal-meshes-20261113-183637/run.json`.
The newer lease implementation alone does not remove the fault.

The next bounded experiment temporarily disables Sun shadows in the grid
scene, retaining the same meshes, route, four hosts and exact comparison.
This asks whether the shared Sun-shadow path is needed for the mismatch. It is
an ablation, not a shipping change, and its result is not yet known.

## Continuation: Sun path ablations and direct depth readback

Prior-session local ablations retained in `tmp/culling-m3/` passed one full
Portable route with Sun shadows disabled, and one with the synthetic instance
casters omitted. Omitting cooked casters, removing receiver derivatives or
adding invariance to the cooked caster did not eliminate failures. None of
those single passes establishes that the corresponding change fixes the fault.

The continuation's initial working-tree ablation disabled shadowed air only
(`visibility = 1.0` in `synthetic_aerial_shadowed.wgsl`). It still failed at tick
180. [Its log](M3/diagnostics/no-air-shadows.log) is retained. The shipping shader
was restored; no ablation is part of the implementation.

A temporary [depth-readback patch](M3/diagnostics/shadow-map-diagnostic.patch)
copies an actual 1024×1024 Depth32Float Sun layer into the submitted SceneLinear
readback allocation, retaining all four hosts and every route frame. The copied
bytes are diagnostic depth, not scene-linear colour. With that patch, the near
map (layer 0) is identical across the four hosts for all 361 route frames.
[Identity](M3/diagnostics/shadow-map-identity.json),
[log](M3/diagnostics/shadow-map-0.log) and
[rows](M3/diagnostics/shadow-map-0-rows.jsonl.gz) pin the run. This narrows the
search; it proves neither the other layers nor downstream shading deterministic.
The far and height layer runs are queued under the shared GPU lock. The
temporary production edits have been reverted after building the diagnostic
executable, and the queued helper uses that exact binary without rebuilding.
