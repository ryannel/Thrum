# Thrum V1 handoff

Updated 7 November 2026 for the directional Sun penumbra WIP. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

## Current WIP

`main` at `91f524b` is the verified baseline. Branch `codex/wip-g3s-solar-penumbra`
adds a directional PCSS candidate to the same two maps, with cone padding and 16 blocker /
32 disk samples. It is unverified. Its first native capture at
`tmp/g3s-showcase/capture-20-penumbra/` reduces the pixel (640,600) 34→35 jump from 103
to 20 SDR codes. Frames 33–38 now fade through the ridge shadow. Contact appearance,
GPU cost and Chrome still need checks. Final full check is session 68807, with logs in
`tmp/g3s-showcase/solar-penumbra/final-dev-check.log`. An earlier pre-fix full check
is session 83506. Observe these sessions instead of starting another check. The GPU finite-disk/plane test passes after rendering its depth inputs. It covers two blocker
distances, tilted receivers, empty grazing maps and a narrow central caster. Independent review
found two production faults, both fixed. Empty depth stays empty when a plane reference goes
negative, and the blocker search reserves a centre sample. Run with `cargo test -p thrum-render --lib finite_sun_shadow_width`.

A separate source-triangle raycast at the real receiver (-17.5194,239.9546,15.9897) found
central ridge blockers about 13 km upstream. At frames 33–38, 512 disk directions give
visibility 0.893, 0.709, 0.498, 0.279, 0.096, 0.0. Shader shadow-off renders change by one
code at 34→35. Those results establish the finite-source shadow defect. Raw scripts, saved
partial rows and logs are under `tmp/g3s-showcase/solar-penumbra/`. Keep existing local
receiver-plane bias independent of the broad penumbra radius. The full receiver-plane
extension is an approximate parallel-surface model, and grazing receivers need the oracle.
The test uses the shipping shader body and independent ray/plane disk geometry. Its depth
fixtures must be rendered, as WebGPU forbids queue writes to Depth32Float. Updated game,
release benchmark and Wasm builds have completed. Capture final images in a new directory
and run the saved ABBA script only after both full check sessions finish using the GPU.

## Goal and status

One reusable engine powers the same single-player V1 showcase on the M3 Max's built-in 120 Hz
display and Chrome. V1 needs physical sky, Sun and aerial perspective, cooked assets, live
camera and shadows, GPU culling, Apple reconstruction and EDR, display-link interpolation,
hot reload, profiler and controls. G2S separately proves corrected-run multiplayer equality.
Keep G3s first. G3b and G2S can progress on disjoint files. G5 and G6 belong to separate Claude
sessions. Their seam notes are `tmp/g5-boundary/NOTES.md` and `tmp/g6-boundary/NOTES.md`.
Transport, sea and clouds are later. The owner chooses when the port starts.

Correctness, lifecycle, visible quality and independent review close a gate. Record numeric
baselines and investigate visible hitches or roughly 25% regressions. No physical iPhone is
available. Simulator results prove function only, and any Mac stand-ins are labelled proxy.
Commit and fast-forward verified checkpoints to `main`. Keep the owner's pending `AGENTS.md`
and `docs/ASSET-LIFECYCLE.md` edits out of the commits. Owner digests are asynchronous.

## Verified and open

- The eight-node analytic atmosphere transports solar light through Sky, M, GroundE and aerial
  in-scatter. It projects at the RGB boundaries. The disk and receiver Sun use the projected
  T plane. The CPU meter uses the same coefficients. Aerial transmission of arbitrary RGB
  materials still uses a diagonal three-band approximation.
- `env RUSTC_WRAPPER= ./dev check` passed on the real Metal host. The production upper-twilight
  sample matches the compact-profile oracle within 2.5% per channel. The test checks all eight
  M lanes alongside the separate matched-band CPU/GPU reference. Chrome's `sky-reflections`
  case passed with Wasm SHA-256 `add91ac18cae0ad8a6ba885cfe4e778cea24726e0a854d219028395bc73275f8`.
- Native review saw blue upper twilight, warm horizon, and matching haze and reflections.
  Final 32 captures are byte-identical to the independently reviewed set. The [evidence page](G3S-FIRST-LIGHT-2026-11-06.md)
  carries a contact sheet, clip and timings. The sequence is offline and sparse in solar time.
  Chrome still shows the diagnostic scene. Live cadence and calibrated Earth radiance remain open.
- The foreground reads flat and bright against Kiara dusk. Some low-Sun contact marks and
  terrain joins remain. The original 17.9 h Sun-facing captures point into a ridge. Added clear views show the disk, and midnight
  is black under the current sole-Sun model. Isotropic multiple scattering and exposure are
  still approximations. Do not tune a colour gain to compensate for incorrect sky irradiance.
- G3b live ProMotion presentation and recovery are open. G2S still needs game-side Resolve
  staging and permit handling. The older quiet runs and tripwire baselines are linked from
  the build plan. There is no pending owner decision for the next atmosphere step.

## Pick up here

The capture route now adds clear Sun stills and 97 half-minute solar samples. Metal captures are at
`tmp/g3s-showcase/capture-18-solar-crossing/`. Full `./dev check` passed. Independent review
saw a stable roughly 9-pixel disk, then progressive ridge occlusion across frames 33–38,
disappearing in frame 39. Sky samples changed by at most 1–2 SDR codes per step. The [selected
crossing clip](g3s-first-light/solar-crossing.mp4) runs at 24 sampled frames/s.

**Next defect.** Foreground pixel (640,600) jumps by 103 SDR codes in frame 34→35 while part
of the disk remains visible. Best diagnosis is the terrain shadow from a point Sun with a
small PCF footprint. Test a finite solar angular-radius penumbra using blocker separation,
and check receiver-plane bias, cascade transitions, cost and contact attachment together.
Keep the same host Sun/time and the fixed valley camera as the comparison. Separate disk/halo,
Earth-shadow stepping, shadow-map contact and meter/exposure defects before tuning. Inspect
physical radiance and displayed colour independently. Use the checked CC0 Kiara photos
under `tmp/g3s-showcase/reference/` for look only. The provenance JSON is beside them.
The dense spectral probe is under `tmp/g3s-showcase/spectral-transport/`. It includes a
separate restricted reference cache. Never commit the cache or derived measured values.

Rerun from the repository root on the Mac with Metal access:

```sh
env RUSTC_WRAPPER= ./dev check
env RUSTC_WRAPPER= rustup run 1.99.1 cargo run --locked --offline -p thrum-cook -- cook examples/v1-game/assets/showcase tmp/g3s-showcase/cooked
env RUSTC_WRAPPER= rustup run 1.99.1 cargo build --locked --offline -p v1-game --bin v1-game
env THRUM_ASSET_STORE=tmp/g3s-showcase/cooked THRUM_SCENE=valley THRUM_CAPTURE_SCENE=tmp/g3s-showcase/capture-fresh target/debug/v1-game
env RUSTC_WRAPPER= sh examples/v1-game/web/build.sh
node scripts/check_v1_browser.mjs --case sky-reflections --output tmp/g3s-showcase/chrome-fresh
```

Use new capture and Chrome directories per run. `tmp/g3s-showcase/retain-capture.py` verifies
byte-exact PNG recovery and builds contact sheets. The selected clip is 24 samples at 5 fps.
Do not present it as live motion. The ordinary sandbox may provide no Metal adapter. Use
the permitted device run. Use the pinned Cargo and clear `RUSTC_WRAPPER` if the host cache
wrapper fails. Cargo builds in worktrees sharing one target should run serially. Do not
overlap owned GPU measurements and tests.

Raw logs, build inputs, ABBA runner and per-run hashes are in `tmp/g3s-showcase/q8-production/`.
The benchmark is the existing `cargo bench --locked --offline -p v1-game --bench frame --no-run`
artifact run with `--g3b-threaded-batch`, the same CAS and `THRUM_SCENE=valley`. Build baseline
`7236b38` and candidate separately, retain each executable before replacing it, then alternate
A/B/B/A with uptime logged before and after each run. A completed `paired/runs.json` row has the
binary hash and exit status. Keep completed rows, use a fresh directory for any missing slot,
and append its observation instead of repeating finished work. The campaign completed before this checkpoint was pushed. All current capture, observer and check sessions are terminal. No run needs to be resumed. The old live Metal
System Trace scratch remains local. Review it before any raw drawable bridge.
