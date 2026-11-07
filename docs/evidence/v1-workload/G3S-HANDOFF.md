# Thrum V1 handoff

Updated 7 November 2026 for the finite-Sun penumbra landing by Claude, root while Codex is paused. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

## Claude root session (7–12 November)

The Codex agent is paused for quota until 12 November. A Claude session acts as root
meanwhile and writes a handback for Codex before then. Its oversight log is
`tmp/code-review-2026-11-05/overnight/WATCH-LOG.md`.

The finite-Sun penumbra from `codex/wip-g3s-solar-penumbra` landed on `main` with two
follow-ups. See [the penumbra evidence](G3S-SOLAR-PENUMBRA-2026-11-07.md).

- **Stipple cause.** A CPU ray probe over the source terrain reproduced Codex's
  0.893…0 visibility. It showed that the remaining stippled patches were false: a
  concave join 0.6 m upstream of a planar receiver reads as a blocker at grazing Sun.
  Depth within 0.9·tan(Sun elevation above the receiver) × the tap offset, capped at
  slope 0.25, now counts as the receiver's own surface. A new shipping GPU oracle
  fixture fails without it.
- **Taps.** Reduced to 8 search and 16 disk taps after truth scoring.
- **Near map unpadded.** The draft's finite-Sun cone guard grew the near map about
  threefold at low Sun and faded sphere contact. Only the last cascade is padded now.
  Blockers whose support leaves the near map are evaluated there, combined by minimum.
- **Cost.** Shared-GPU ABBA pass-sum p50 is 1.71 → 2.02 ms.
- **Terrain horizon.** Receivers beyond the cascades march a top-down caster height
  map toward the Sun (third shadow layer, 32 m texels over 32 km). Ray-truth error past
  500 m is at most 0.06 mean.
- **Floor-edge seam.** A Sun-facing slope at the floor's far edge rendered grey-blue
  because the back-facing normal repair reflected it across V's plane. It now lifts
  the normal just onto V's visible side.
- **Review.** Independent image-only review ranks the candidate above main and the
  draft. A second reviewer caught crease lines left by an interim fixed-slope
  tolerance. After the elevation bound, ray-truth false samples are zero on the
  scored solar frames.

The probe, scores and bench logs are in `tmp/g3s-showcase/solar-penumbra-claude/`. The final
capture is `tmp/g3s-showcase/capture-29-terrain-horizon/`.

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

**Next defects.** The 103-code foreground jump is fixed. Open, in order:
- The final image-only review passed. Its most visible flaw is a staircase far-ridge
  silhouette, which is terrain geometry rather than shadowing. Score any shadow finding
  with the ray probe before changing anything.
- The 100-code foreground step at solar 77→78 was a shadow leak from back-face-culled cooked
  casters with the Sun below horizontal. It is fixed ([evidence](G3S-SUNSET-CASTERS-2026-11-07.md)).
- Live cadence: `THRUM_CAPTURE_LIVE_TICKS` captures consecutive 60 Hz ticks. It exposed crest speckle at a
  0.6° Sun, now bounded by the height-map horizon below a 0.1 elevation sine
  ([evidence](G3S-LIVE-SUNSET-2026-11-07.md)). This is offscreen; on-screen ProMotion presentation is still G3b.
- Distant terrain shadows now come from a caster height-map horizon
  ([evidence](G3S-TERRAIN-HORIZON-2026-11-07.md)). Its +24% pass-sum is the next cost
  item; a per-frame horizon-angle map is the scalable form.
- The penumbra's +18% pass-sum. Gathered fetches and a lit early-out are untried.

Keep the same host Sun/time and the fixed valley camera as the comparison. Inspect
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
