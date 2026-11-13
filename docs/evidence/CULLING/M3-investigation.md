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
failure. No tolerance, retained baseline or oracle draw order has changed.

Two initial diagnostic invocations failed before rendering because their
fixture paths were relative to the workspace while Cargo starts tests in the
package directory. Corrected invocations use absolute paths; their logs are
`tmp/culling-m3/diagnostic-{sdr,linear}-fixed.log`.
