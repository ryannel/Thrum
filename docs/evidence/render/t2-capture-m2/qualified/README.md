# T2 M2 simulator function qualification

Builder claim on 9 November 2026. Independent lead review is owed.

## Reproduce

```sh
python3 /Users/ryannel/.claude/plugins/cache/ryannel-skills/orchestrate/1.5.2/skills/orchestration/build/scripts/agent_loop.py lock gpu --worktree . -- python3 scripts/capture_v1_simulator.py --case all --output tmp/t2-capture-m2-review
python3 scripts/package_v1_simulator.py --session tmp/t2-capture-m2-review/01-pan
```

Python, pinned Rust/Xcode, iOS 27.2 simulator runtime, Swift, Zstd and FFmpeg are required. Missing targets fail. Build and cook precede qualification. The recorded run used `--no-build` with the frozen app and cooker whose SHA-256 values are in `report.json.gz`. Every native source matches its recorded source hash. The archived `driver-measured.py` matches the measured driver hash exactly. Subsequent driver changes preserve compact artifacts before validation/copy failures, with two added passing regression tests.

## Target and result

The owned iPhone 18 Pro simulator ran native 1206×2622 cooked Direct output through the ordinary UIKit foreground route, world lane, FrameHost, final SDR tap and shared asynchronous writer. `identity.json.gz` records the actual Apple iOS simulator GPU / Metal adapter. Root report pins host, runtime, model, binary, source, cooked products, commands, launcher and actual parent/child PIDs. LaunchServices launcher read-only `CGPreflightScreenCaptureAccess` returned false before and after. The console was unlocked through engine captures and locked naturally during the supplemental UIKit regression. No console observation gates admission or success.

Three complete pans contain 726, 728 and 734 images. Running contains 259, stall 136, surface recovery 182, background recovery 183 and device recovery 182. All 3,130 positive originals have exact coverage and zero capture omissions. `audit.json` independently decodes every original, including 87 negative-control images. Pan packaging independently checks all 726 PNG round trips and preview coverage. Running and stall packaging do the same. VFR preview PTS follows actual monotonic submissions within 0.5 microseconds, with a labelled terminal display duplicate. The injected render stall retains its 8.037415375-second gap. Engine source time establishes simulator function only.

Pending sessions cancel on replaced surface, background and injected device loss. Retired metadata, pending counts, lease/device generations and fresh sessions are retained. Background world/render counters stay fixed. Lost empty submits and recovery failures remain zero. Saturation preserves 6 of 183 submissions and reports 177 omissions. Raw slow writes preserve 81 of 302 and report 221 omissions while authority/rendering continue. Both refuse continuous evidence. Earlier compressed slow output fit its reserve and completed, so that attempt is retained as a failed control in local `series-04`, superseded by the explicit Raw control.

The existing XCUITest touches and rotation route also passes, exercising actual UIKit gestures, background-held touch clearing, stale-window input discard and recovered swipe. `04-running` uses semantic move/rest through the normal axes handler. Its controlled actor changes committed position but lies outside this portrait camera. Visible diagnostic grid actors establish inspected actor motion independently. Native UIKit/OS controls are outside the tapped engine texture. Physical iPhone and physical display qualification remain open with their owners.

## Builder inspection

`inspection-list.md` predates viewing. Full native images, pan checkpoints, true native crops, sixteen adjacent moving-grid crops and recovery images were inspected alongside source identity/timing. The sphere and rock exit left coherently during the authored yaw sweep. Terrain and sky remain complete, rows/channels are intact and opaque, and the holds are stable. Existing rock-surface striping is visible. Running-grid motion is coherent across adjacent images. Recovery images contain complete current-generation geometry. The stall retains a held image followed by fresh progress. Temporal judgment used image sequences and measured host timing. No live video perception was available. Contact-sheet black unused tiles are montage padding. `packaging.json.gz` joins every original, selected PNG and timing record. Preview files are scaled viewing aids. Native PNGs/crops preserve the original grid.

## Bounds, checks and retention

Six staging slots and 32 CPU leases share a 1 GiB pixel cap. A separate finite 256 MiB encoded reserve plus codec scratch is charged. Each writer has an 8 GiB spool cap, 1,024-record / 16-second session bounds and ten-second finalization deadline. Driver preflights 20 GiB for the complete series. `audit.json` records observed peaks. Disabled capture creates no image artifacts. Session metadata records every callback disposition, dispatched opportunity, submission and omission.

Pinned iOS Clippy, ten capture Python tests and full `./dev check` pass. The full Check uses `RUST_TEST_THREADS=1 RUSTC_WRAPPER=` because the parallel run hit existing running-world Overload(13) tests under host contention. No tests were skipped. Land repeats the full Check under the GPU lock.

Full lossless spools, exact app/cooker and failed-attempt logs remain under `tmp/t2-capture-m2-resume/` through lead review. Final sources are `final-series/`, with current app `final-v1.2-app/`. Git retains compact metadata, hashes, native selections and previews. Full spools have no durable backup. Recreate unselected PNGs by decoding the original spool before replaying `timeline.ffconcat`. Superseded bulk is eligible for ownership-scoped retirement after metadata verification. M1's 16 GB scratch was already retired in the earlier closeout.

Fresh simulator startup initially blocked in SpringBoard IconServices before engine launch. A shared CoreSimulatorService reset stopped two pre-existing booted simulators. Both pre-existing devices were verified Booted again after the passing integration Check. `simulator-restoration.json` records the restored device identities. Their earlier in-app session state could not be restored. The successful driver instead restarts only its owned simulator's IconServices worker before launching the app. This setup action is recorded with the command and every startup attempt. No pre-existing simulator data was removed.
