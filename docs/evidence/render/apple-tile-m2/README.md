# APPLE-TILE M2 evidence

M2 implements one prepared scene/surface graph per host day/night and capture variant. The existing host encodes its
pre-acquire prefix, then supplies an execute-time surface binding for the final present pass. Both recipes retain
the tone shader, Rgba8Unorm intermediate, present shader, formats, draw order and quantization boundary.
No native segment, replacement tone input, optimization or performance promotion is included. The standalone
SurfacePresenter utility remains available; FrameHost uses the combined graph rather than that utility executor.

| Item | Builder result |
| --- | --- |
| D2.1 | Graph/executor phase tests reject an unrelated post-acquire scene and a scene depending on the surface. Missing late binding stops phase two without preventing phase one. Real Metal tests inspect both recipes' merged plans and retained output texture identity through surface resize. |
| D2.2 | FrameHost/token, actual Metal staging-submit-attempt, history/cut, surface bytes, resize and replacement regressions pass. Healthy skip flushes staging; lost-device abandon performs no extra queue call. Existing Mac loss checks remain before and after acquisition. |
| D2.3 | **Open.** Exact M0/paired comparisons, source coverage, native frames/crops and contact sequences pass. Required 1x playback inspection is incomplete: Computer Use rejected IINA access as not approved. The final-build movies and host-time identities are retained. |
| D2.4 | The iPhone 18 Pro simulator diagnostic route passes hidden/replaced surfaces, stalled replacement, raced presentation and foreground/background lease recovery. Endpoint images and queue/lifecycle logs prove function only. |
| D2.5 | **Open.** The proposed shell-launched OS observer's read-only Screen Recording preflight is denied. The console was unlocked. No grant was requested or changed, and no OS game-window capture was started. |

The initial native matrix passes 38 exact M0 comparisons (30 route/repeat and eight Sun/night SDR/linear).
The final checkpoint probe and surface controls pin the final diagnostic binary and retain exact sample/exposure
identity. [Initial matrix](initial-native.json.gz), [final checkpoints](final-checkpoints.json.gz),
[surface controls and motion](surface-and-skip.json.gz), [checks](checks).
The surface controls retain the complete 1000×800 non-1:1 letterbox images for both Bgra8Unorm and sRGB branches.

[Motion records](motion-summary.json) distinguish elapsed-time engine motion from fixed checkpoint equality.
The portable/Apple routes retain 964/963 submitted images and history epochs 1→2. The deliberate render-owner interruption
skips 30 opportunities on each recipe; observed image gaps are 259.14 / 260.13 ms and then recover.
Every source hash verifies, with zero ring/writer omissions. These are injected-gap observations, not cadence or speed
qualification. [Selected lossless frames, original-time 1x movies and records](motion).

The builder inspected full frames, native details, contact sheets and cut-adjacent samples for Sun, sky/stars,
terrain, shadows, mirror/local lights and recovery. [Playback refusal](playback-review.json) keeps the separate 1x
inspection open. The inherited valley presentation and local-light fixture remain unchanged.
Independent lead review is owed. [Simulator records](ios), [window permission preflight](window-preflight.json).

Preparation recompiles and binds the small variant plans; checked texture-slot/descriptor/storage correspondence
reuses existing scene allocations across surface-only resize. Frame execution walks prepared recorder slices,
with no new per-frame graph lookup or preparation. Upload/storage/resident resources remain the M1 arrangement.
Performance, physical traffic, panel cadence and physical iPhone behavior are unmeasured.

Reproduce the full retained matrix with `scripts/apple_tile_m1.py`; `scripts/apple_tile_m2.py` adds letterbox and
realtime cut/stall proof. Both take `--binary`, `--store` and a fresh `--output`. Build with
`cargo build --locked --offline --release -p v1-game --features draw-probe,thrum-render/creation-counters`.
Build the simulator with `examples/v1-game-ios/ios/build-simulator.sh`, then run
`scripts/apple_tile_m1_sim.py --output <fresh-directory>`. All GPU runs use the repository `gpu` lock.
Bulk lossless sources and qualified binaries remain in owned `tmp/g3t-m2` through lead review; no remote backup is
claimed. [Selected-file hashes](retained-files.json) permit verification before scratch closeout.

M2 remains open. D2.3 needs approved video playback access and completed 1x inspection. D2.5 needs the owner's
attended OS-observer setup, followed by both-recipe window/input, resize/replacement and matched-state reference
observations. Engine images cannot pass D2.5. The milestone check and complete repository gate pass.
