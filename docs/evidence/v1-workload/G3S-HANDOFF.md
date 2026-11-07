# Thrum V1 handoff

Updated 7 November 2026 for the solar-crossing capture extension. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

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
  terrain joins remain. The fixed Sun-facing valley captures point into a ridge, and midnight
  is black under the current sole-Sun model. Isotropic multiple scattering and exposure are
  still approximations. Do not tune a colour gain to compensate for incorrect sky irradiance.
- G3b live ProMotion presentation and recovery are open. G2S still needs game-side Resolve
  staging and permit handling. The older quiet runs and tripwire baselines are linked from
  the build plan. There is no pending owner decision for the next atmosphere step.

## Pick up here

The current WIP adds clear Sun stills and 97 half-minute solar samples in the existing game capture route.
Native Metal output is at `tmp/g3s-showcase/capture-18-solar-crossing/`. The clear Sun is visible
at 17.4 h, then passes behind the valley ridge. Treat the capture addition as unverified until
its full check exits successfully. The active check is tool session 52173, writing
`tmp/g3s-showcase/solar-crossing/dev-check.log`. The disk observer is session 22333 with
`disk.json` and `disk.log` in that directory. Captures have completed and the PNG retention
pass has its own log there. Follow up with independent visual review of the new sequence. Keep the
same host Sun/time and the fixed valley camera as the comparison. Separate disk/halo,
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
and append its observation instead of repeating finished work. The current campaign finishes
before this checkpoint is pushed. No GPU recording needs to be resumed. The old live Metal
System Trace scratch remains local. Review it before any raw drawable bridge.
