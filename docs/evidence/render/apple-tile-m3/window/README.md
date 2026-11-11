# M3 actual Mac window function

The existing Screen Recording grant was observed, never changed: [preflight](preflight.json).
The observer is macOS `screencapture` (OS-bundled tool; macOS version/build pinned by the native manifests),
plus read-only CGWindow enumeration. [Tool/version](observer-tool.json) pins the actual OS build and tool hash;
[observer sources](reproduction/window.py) retain the protocol (scratch paths are local replay inputs). All runs held `agent_loop.py lock gpu`.
`observer.json` retains PID/window ID, global bounds, build/recipe, paused controls and clock observations.
OS PNGs include the 64-backing-pixel title bar; the 1920×1080 game content starts at (0,64), scale 2.
Full desktop movie bounds include the game window; the window bounds in the observer identify its content.
The proof uses the M3 binary pinned in the parent provenance record.

For each recipe, `sun.png`, `night.png`, `noon.png` were inspected against
`references/<recipe>-<state>/checkpoint-0.png`. References use matching paused world controls,
scene content, camera and 1920×1080 extent; they are separate submissions, not compositor byte oracles.
Sun/sky, stars, terrain, cube arrangement, mirror, local light and shadows agree in settled states.
The night light is the existing saturated diagnostic light. OS scaling/colour and title bars are not compared exactly.

`<recipe>/control-response.mov` records ordinary parameter controls changing Sun/night/noon at host time.
`resize.json` and `<recipe>/{before,resized,restored}.{png,json}` retain the actual CUA drag and restored view,
with loaded executable path/hash and actual process recipe. At 1520×880 content, the 16:9 scene occupies
1520×855 pixels with 12/13 black rows; geometry is intact and the original view returns under the same PID.
The test-host app retains its historical Thrum M2 name, but its observed executable is the M3 proof binary.

`<recipe>-recovery/window-response.mov` is four seconds of OS window observation. `timeline.json`
aligns recorder start, generation 2→3 replacement and accepted camera pan inside that interval;
`surface.tsv.gz`, `controls.tsv.gz`, `clock-anchor.tsv.gz`, game/recorder logs and `observer.json` retain attribution.
`post-pan.png` is the original movie at 3.7 seconds and corroborates the moved intact scene.
All media were inspected at 1x. Replacement preserves the paused world and controls, then accepts the pan.
These are functional window observations, not measured display swaps, physical cadence or input-to-photon latency.
Mac M2.3 owns separate window-tap byte correspondence; Mac M3 must requalify the changed build's cadence.
