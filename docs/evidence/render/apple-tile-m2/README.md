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
| D2.3 | Exact M0/paired comparisons and complete source accounting pass. Visible-session IINA 1.5.0 playback at 1x was inspected for both recipes' route/cut and skipped-view sequences; scene content and recovery remain intact. [Review](playback-review.json). |
| D2.4 | The iPhone 18 Pro simulator diagnostic route passes hidden/replaced surfaces, stalled replacement, raced presentation and foreground/background lease recovery. Endpoint images and queue/lifecycle logs prove function only. |
| D2.5 | Both recipes have settled OS Sun/night/noon captures against matched T2 references, ordinary control-response movies, deliberate resize/restoration with correct letterbox, and same-PID surface-replacement movies/logs. [Window records](window/summary.json), [resize](window/resize.json). |

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

The builder inspected full frames, native details, contact sheets, cut-adjacent sources and IINA 1x playback for
Sun, stars, terrain, shadows, mirror/local lights, cut and recovery. The inherited valley presentation remains.
The earlier [denied preflight](window-preflight.json) and playback refusal are history; the attended
[passing preflight](window/preflight-after.json) and [visible postflight](window/visible-postflight.json) record no automated grant change. Independent lead review is owed.
[Simulator records](ios) prove diagnostic function only.

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

Owner setup/direction is recorded verbatim in [ACCEPTED.json](../../../plans/APPLE-TILE/ACCEPTED.json),
`milestones.M2.owner_answers`; the latest D2.5 entry is authoritative. [Selected record](window/summary.json)
preserves those exact entries. Computer Use succeeded in this visible Codex session.
OS observations use `/usr/sbin/screencapture` on macOS 26.6.2/25G83, with PID/window/bounds, scale 2, host clock and
final binary hash retained. At 1520×880 content, the unchanged 1920×1080 scene fits 1520×855 with 12/13 black rows;
both windows restore to 1920×1080. Sun/night/noon OS images match T2 content at the same paused controls, not the
same submission or exact compositor bytes. Short host-time recovery videos supplement the retained replacement
records: both span surface generation 2→3 and native AppKit yaw response, with all lifecycle rows presented.
Earlier failed recorder attempts remain labelled failures; no replacement video is inferred from engine images.
M2 is claimed for lead review. The [milestone check](checks/visible-milestone-check.json) and full repository gate
pass; no panel-cadence, speed or physical-device promotion.
