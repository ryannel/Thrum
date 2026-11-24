# T2-CAPTURE plan

Engine capture for every visual milestone. [Design](DESIGN.md) · [brief](BRIEF.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md)

## Open for the human
- Assumption: the motion arms that fail are not on a fixed clock, as the request assumed. They step on real elapsed time, so waiting for the ring means fewer frames, drawn later, and every frame that is drawn is kept. That is safe here because those arms compare no pixels. The other choice takes the motion arms out of the main-branch check, which also ends the dependence on load but stops running motion on main.
- Assumption: only the motion arms of the exact-images check wait for the ring. Every other motion capture still drops a frame when the ring is full, because a capture that holds the renderer back cannot show how smoothly the game runs. A module whose capture only needs every frame can choose to wait.
- Assumption: only the Mac capture gains exit 75 for an incomplete runtime record. Chrome's captures in the check run on a fixed clock and already wait for each frame.
- Assumption: a validated Chrome capture can pass after TERM or KILL during requested shutdown if all owned processes are confirmed gone, cleanup succeeds, and the launcher records both denied permission preflights.

## M0 Mac engine images and runtime motion without a desktop capture grant: done
[review](../../../.orchestrate/evidence/T2-CAPTURE/M0-review.md)

## M1 Chrome WebGPU images and runtime motion without desktop capture: done
[review](../../../.orchestrate/evidence/T2-CAPTURE/M1-review.md)

## M2 iOS simulator engine motion without desktop capture: done
[review](../../../.orchestrate/evidence/T2-CAPTURE/M2-review.md)

## M3 A motion capture on a busy Mac keeps every frame it drew, and one that falls short is tried again, not blamed on a commit

Read [a full ring, and a run that could not judge](DESIGN.md#a-full-ring-and-a-run-that-could-not-judge) and
[the ruling](../ARCHITECTURE.md#ruling-on-a-capture-that-drops-frames) first. Start with M3.1's `omit` control
under load, because the design rests on its cause: if frames are lost while staging slots are free, end `BLOCKED`
with that record.

The opportunity loop and the completeness rule are in `examples/v1-game/src/window/capture.rs`, the elapsed
clock and the `Offscreen` driver in `crates/thrum-app/src/offscreen/`, and the tap in
`crates/thrum-render/src/runtime/submitted_capture.rs`. `scripts/capture_v1_runtime.py` turns the runtime record
into the manifest and the exit status, and `run_t2` in `scripts/apple_tile_m0.py` is how the matrices call it.
Many scripts under `scripts/` and `.orchestrate/evidence/` call that wrapper or `run_t2`; none may start reading an
incomplete capture as a pass. The run that failed was pruned from the shared scratch, so
[the handoff](../../../.orchestrate/evidence/REVIEW-ENGINE/M3-handoff.md) holds its numbers and the load has to be made again.

`APPLE-TILE` owns the matrices and `BASELINE-GUARD` owns the verifier, its scratch rule and the registry. The
ruling lists the only changes allowed there. Those files are pinned, so the reference guard in `./dev check`
fails until the registry holds the pins it prints and the refresh the ruling names.

M3.5 passes from the shared run after the merge: set it `pass` with the evidence `main-branch check
exact-images` and do not run `verify --full` yourself. The two proof commands load the GPU on purpose, so run
each under the `gpu` lock and no longer than it needs.

## M4 A completed Chrome capture survives slow shutdown and leaves no browser or profile

Follow [owned Chrome shutdown after capture](DESIGN.md#owned-chrome-shutdown-after-capture). Start with the
LaunchServices path shared by `scripts/capture_v1_browser.mjs` and `scripts/capture_browser_launcher.swift`.
Coordinate the change with the cleanup helper in `scripts/check_v1_browser.mjs` and its direct-child and
LaunchServices callers. Drive the shutdown cases deterministically with stand-in processes so the proof
doesn't depend on recreating the machine load. The shared `exact-images` run on main supplies M4.3.
