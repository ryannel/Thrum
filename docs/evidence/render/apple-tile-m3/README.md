# APPLE-TILE M3 evidence

M3 prepares a graph-bound pre-tone colour/motion/exposure input and the existing SDR stage. Default input is the
forward composition; G4 can supply its prepared reconstruction texture through the same crate-internal stage.
The tone shader, Rgba8Unorm intermediate, present shader, source-coordinate dither and nearest-source mapping
are unchanged. No native provider, EDR arithmetic, tone fusion or traffic saving is claimed.

The proof build is clean source `7ffff46fcda5fa8cc04cc6cb38e4a7152a080d0c`, release binary SHA-256
`be33c9c3ef2a111e41b4fad7adc64adbb9859690e64485cb9685760b79daf9e9`, Rust 1.99.1 on this M3 Max.
[Provenance](provenance.json) pins compiler, all source files and binary. The [artifact audit](artifact-audit.json)
rechecks all retained native comparison files and the qualified binary hash. The
[integration audit](integration-source-audit.json) confirms engine/game/shader/build inputs are unchanged after rebase. All target runs used the shared GPU lock. The required check uses a [Cargo runner](checks/gpu-test-runner.py)
to lock native test executables, while the named moving check takes its declared lock. After long
shared-resource waits between executables, the final check uses one [check-scoped lease](checks/profile-gpu-hold.py)
through the same lock command for native test executables; it ends with the check process.

| Item | Evidence |
| --- | --- |
| D3.1 | [Stage and binding](../../../../crates/thrum-render/src/synthetic/display.rs) retain graph resource, generation, extent and exposure-side identity. All three shader inputs are declared reads. Metal identity rebinding preserves exact scene-linear/SDR bytes; a distinct wgpu-copy-produced texture has a copy→tone hazard and identical SDR pixels; foreign graph bindings are refused. [Tests](checks/input-tests.log.gz). |
| D3.2 | [Native matrix](native-m1.json.gz): 38/38 exact M0 route/repeat/Sun/night SDR and linear comparisons. [Surface matrix](native-m2.json.gz): 4/4 exact Bgra8Unorm/sRGB non-1:1 letterbox comparisons. [Direct recipes](paired.json): 10/10 exact with matching sample/exposure identity. No tolerance. [Motion](motion-summary.json) verifies every source, route history 1→2 and injected skipped-view recovery. |
| D3.3 | FrameSample retains named graph timestamps; [consumer audit](timing-consumers.txt), optional-layout tests and [actual day/night samples](timing-schema-sample.json). All 480 samples have counters; stars are present on 240 and absent on 240. Untimed transfers/query resolution are null, unsupported/faulted batches are None. The raw map includes present; `SceneGpuTimings::sum` excludes it, while `gpu_present_ns` mirrors its measured value. CPU/GPU/callback clocks remain separate. |
| D3.4 | [Chrome](chrome/report.json): 1,924/1,924 exact pre-change day/night SDR/linear frames, including GPU exposure identity on linear comparisons. Separate realtime captures are complete. Resize changes presenter generation 1→2 at 961×541, retaining engine extent/content. The [named moving command](checks/moving-run.json.gz) passes as endpoint smoke only. |
| D3.5 | [Fresh MF1 window rerun](window-rerun/README.md) on the exact reviewer binary closes the locked-session coverage gap. Historical [OS window proof](window/README.md): granted Screen Recording, PID/build/recipe-attributed matched-state stills, ordinary controls, CUA resize/restoration and generation 2→3 replacement with accepted pan under both recipes. |

The builder inspected full frames, native details and cut-adjacent sources for Sun, stars, shadows, mirror/local
lights and scene geometry, plus native/Chrome contact sheets. Chrome's saturated midnight light fixture is inherited
and byte-identical to its retained baseline. IINA 1.5.0 playback at 1x reviewed both recipe route, injected-stall and prepared day/night sequences, Chrome day/night realtime, and OS control/recovery recordings; [review record](playback-review.json). Independent lead review is owed.

Main's newly registered [exact-baseline gate](checks/integration-exact/proof.json) passed on the integrated code content: 38 route plus four surface comparisons and 1,924 Chrome records. Its [record](checks/integration-exact/record.json) pins the content key, reference hashes, compiler and candidate artifacts. Native binary, JavaScript and Wasm hashes exactly match the original M3 proof artifacts, preserving the motion/window review's build identity.

Preparation retains graph reads/resource IDs and prepares exposure-side bind groups before readiness. Execution
adds bounded binding/extent checks; completed reports own bounded name/value lists and transfer labels. GPU textures,
tone/intermediate/present arithmetic and submission ownership are retained. Added CPU/report cost, physical traffic,
performance, panel cadence and physical iPhone behavior are unmeasured.

Reproduce with `scripts/apple_tile_m1.py` and `scripts/apple_tile_m2.py`, each with `--binary`, `--store` and a fresh
`--output`; build using `cargo build --locked --offline --release -p v1-game --features draw-probe,thrum-render/creation-counters`.
Chrome uses `scripts/capture_v1_browser.mjs`, `scripts/apple_tile_m1_compare_browser.py` and
`scripts/package_v1_browser.py`; the manifests retain exact arguments and artifacts. [Repository check](checks/dev-check.log.gz),
[Clippy](checks/final-clippy.log.gz) and [simulator build](checks/ios-build.log.gz) and [simulator diagnostic lifecycle](ios/report.json) pass. The required milestone command passes D3.4 and the full repository gate: [check results](checks/milestone-check.json). The [earlier timeout](checks/milestone-check-timeout.json) records a GPU-lock wait; a [30-minute attempt](checks/milestone-check-contended.json) also expired behind the shared GPU queue, and an [in-sandbox attempt](checks/milestone-check-sandbox.json) exposed no Metal adapter. The successful rerun uses actual Metal access and one check-scoped GPU lease for native test executables; the moving check takes its own declared lock.

The successful milestone check covers these byte-identical native/browser artifacts. After the upstream verification-policy update, the registered exact proof above passed; final integration reruns the updated full repository Check through `agent_loop.py land`. A [duplicate queued check](checks/duplicate-check-cancelled.json) was stopped before it acquired the GPU, without representing it as a pass.

Selected lossless images, original-time movies, manifests and checks are retained here. Full lossless spools,
Chrome records and qualified binary remain in owned `tmp/g3t-m3` through lead review; no remote backup is claimed.
