# Thrum V1 handoff

Updated 7 November 2026 for the directional Sun penumbra WIP. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

## Current WIP

`main` at `91f524b` is the verified baseline. Work is saved on
`codex/wip-g3s-solar-penumbra`. Keep this branch off main until its final checks, cost
comparison and visual review pass.

The current model uses the nearest strictly upstream blocker, with 16 search and 32 disk
samples. It bounds broad receiver-plane extrapolation to half that blocker gap. Grazing
support and sparse blocker selection conservatively underestimate softness. The private
192-byte uniform contains the world-height support used by both map padding and search,
plus numeric surface tolerance. The comparison sampler has been removed.

Capture24 (`tmp/g3s-showcase/capture-24-penumbra-nearest/`) resolves the major defects
from rejected candidates. Full-plane mean PCSS leaked warm triangles near the horizon;
local-only correction caused widespread triangle acne; unconstrained global padding made
noon contacts coarse. Independent review sees a gradual ridge transition and restored noon
contacts in24, but a stronger narrow stippled seam in solar24–28 still needs attention.

A source review then found downstream depth samples could zero the nearest gap. The current
code requires positive axial separation in search and treats downstream depths as visible
in filtering. Its shipping GPU oracle passes, including a mixed downstream/upstream fixture.
It also covers two blocker distances, tilted planes, an empty grazing map and a narrow
central caster. Depth fixtures must be rendered, as WebGPU forbids writes to Depth32Float.

The off-camera umbra test retains its darkness assertion with a 32 m roof 1.4 km upstream.
The former 1.6 m roof is smaller than the finite Sun's ~13 m footprint and cannot form that
umbra. The corrected scoped test passed. CPU upload tests check height and tolerance units.

**Runs to observe.** Full check session37906 writes `conservative-dev-check.log`; it predates
the final upstream-depth guard, so repeat full verification after it finishes. Capture
session19038 builds and records the latest source into `capture-25-penumbra-upstream`,
then retains exact PNGs. Both logs and all raw records live in
`tmp/g3s-showcase/solar-penumbra/`. Do not restart these handles merely because output is
quiet. Once terminal, review25 against baseline18 and rejected23 at all131 frames, especially
sequence02/21, solar24–28 and66–71. Rebuild the release benchmark and Wasm before fresh Chrome
and ABBA runs. The saved before executable is the exact verified q8 baseline; retain it.

The shadow-off counterfactual reduces pixel(640,600) frame34→35 from103 to one SDR code.
Source-triangle rays at receiver(-17.5194,239.9546,15.9897) find ridge blockers about13 km
upstream. With512 rays across the Sun disk, frames33–38 give visibility
0.893,0.709,0.498,0.279,0.096,0.0. The first PCSS candidate reduced the code step to20.
Re-measure the final model before carrying that number forward. No owner decision is owed.

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
