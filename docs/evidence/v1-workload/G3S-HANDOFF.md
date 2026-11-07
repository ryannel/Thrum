# Thrum V1 handoff

Updated 7 November 2026 after Codex resumed as root. Start from remote `main` and
[the build plan](../../BUILD-PLAN.md). The older three-band baseline is `7236b38`. This file
travels with the candidate, so use its enclosing commit to identify the exact tested source.
The Codex goal remains active. Its attachment is
`/Users/ryannel/.codex/attachments/86327eba-9dfd-4eff-965e-5bb0a2134938/goal-objective.md`.

## Current checkpoint

- The browser still fetches Portable. The GPU loader resolves only among products present
  in the fetched generation, in device preference order. A Metal test with BC enabled
  exercises a manifest that advertises compressed variants without fetching them.
  [Chrome valley evidence](G7-CHROME-VALLEY-2026-11-07.md)
- Inspection of the [aerial comparison](G3S-SHADOWED-AERIAL-2026-11-07.md) supports keeping
  the terrain-shadowed aerial path. The reduced orange glow follows the ridge occlusion.
  The air map picks the strongest caster for the Sun's centre. The strongest caster can
  change across the disk, so replacing the surface march needs a separate geometry check.
  The Metal compute ordering diagnosis remains unconfirmed. Fresh Chrome stills retain
  the blue twilight gradient and warm horizon. The stepped ridge and featureless ground
  still read as diagnostic terrain.
- The [sunset comparison](G3S-SHADOWED-AERIAL-2026-11-07.md#mac-cost-shared-load) measured a 29%
  higher scene GPU median under very heavy shared load. Reusing one horizon march in the fade
  band preserved all 191 native frames and 34 Chrome images. Its separate ABBA shows no clear
  whole-frame speedup. Rerun and hashes are in `tmp/handback-review/horizon-fade/`.
  The resumable comparison is under `tmp/handback-review/aerial-cost/`. From the repository
  root, run `sh tmp/handback-review/aerial-cost/build-pair.sh` and then
  `python3 tmp/handback-review/aerial-cost/run-pair.py` with access to Metal. The script uses
  exact commits `9b4bea6` and `2db5039`, the cooked valley store and a 17.8 h solar start.
  It retains partial attempts and skips completed slots on restart. When reusing a Cargo target
  for Git archives, touch the archived Rust files before building and verify the executable
  hashes differ. Cargo accepted the archived timestamps on the first attempt and reused the
  baseline binary for both commits.
- Metal reproduced and verified the below-horizontal horizon fix. The analytic plateau
  and near/last blocker-window oracles now pass, with identical valley images on Metal and
  Chrome. [Terrain horizon evidence](G3S-TERRAIN-HORIZON-2026-11-07.md)
- Chrome's cooked edit and replacement probe found no missed next animation callback at
  the measured swaps. Retain the current loader for the valley and carry the tripwire
  forward for heavier authored edits. [Chrome measurements](G7-CHROME-VALLEY-2026-11-07.md)
- G6's re-split controls and macOS runner commits are approved. The native starter and
  cooked material shader reload are separate reviews. ExposureZones can use the
  existing pre-tone exposed luminance seam. See `tmp/g6-boundary/NOTES.md`.
- The G6 stone circle now reports Metal timestamps with zero synthetic instances.
  Skip the empty transform-history pass and match timestamps against each submitted
  slot's layout. In two interleaved sets of four 120-frame live runs, all baseline
  windows report GPU timing unavailable and all candidate windows report it. The
  1600×900 Metal captures are byte-identical. In the repeat after the workspace
  check, shared load was 25–27.2 and render-worker p50/p95 was 1.40–1.52/3.84–4.94 ms
  before and 1.44–1.49/4.99–7.02 ms after. The tails are unresolved under this load.
  All-clear diagnostic render passes can still lack Metal counters. Rerun
  `python3 tmp/handback-review/empty-timing/run-pair.py <fresh-prefix>` on Metal.
  Raw logs, input hashes and the G6 fixture are in the same directory.
- CPU rays through the G6 stone scene classify 130 of 140 strong streak samples
  fully blocked, three partially blocked and seven clear. The Metal map leaks more
  than 0.25 visibility at 115 of those pixels. The global bias stays. Smoother
  authored geometry reduces the marks in a held view and changes the stone bounds.
  The exact capture, 64-ray comparison and asset variants are in
  `tmp/handback-review/empty-timing/ray-truth/`. G6 owns its starter art.
- The same morning capture has coherent arcs in the stored fp16 sky, before SDR
  quantization. Manual bilinear and double LUT height move the arcs. The green
  radiance p99 change is 0.027% and 0.098% respectively over the sampled sky.
  Neither probe is selected. Scripts and curvature images are in
  `tmp/handback-review/empty-timing/sky-bands/`. Resume after the ultra-review fixes.
- The ultra-review atmosphere fixes keep the five fixed Metal valley phases
  byte-identical. The two-bank Sun query is shared by full and single aerial
  scattering. In the repeated 1080p ABBA at shared load 25–29, scene GPU p50
  was 6.20–6.36 → 6.27–6.68 ms and aerial p50 was 0.096–0.098 → 0.098–0.124 ms.
  The second candidate sample had elevated tone, forward and aerial tails.
  No throughput claim. Raw logs and hashes are in
  `tmp/ultrareview-2026-11-07/atmosphere/`.
- The ultra-review frame and clock fixes are in the
  [bounded-frame evidence](../render/V1-BOUNDED-FRAMES-2026-11-04.md).
  The all-clear Metal timing issue remains separate.
- The cooked valley probe retained the existing sky filter. Vertical cubic
  reconstruction changed morning curvature p99 from 0.157% to 0.079%, but
  changed the sampled SDR sky by at most one code. Baseline midnight was
  entirely black. Capture and scoring scripts, matched frames and source hash
  are in `tmp/g3s-showcase/sky-contours/`.
- Continue [night milestone M1](../../design/modules/ATMOSPHERE-PLAN.md) with
  the finite-shell reference in `tmp/g3s-showcase/night-design/`. Run
  `cc -O3 mixed.c -lm -o mixed && ./mixed 2` in that directory. The program
  gives absolute luminance, ground irradiance and finite-path in-scatter.
  Rebuild the 24 KiB profile prototype with `cc -O3 profile.c -lm -o profile`,
  then run `./profile earth-night-rg16f.bin > profile.csv` and
  `python3 compare-profile.py`. The CPU-packed comparison passes. Next bind
  it in the atmosphere graph and verify the GPU consumers.

## Exposure audit decision

The audit is `tmp/research-2026-11-07/EXPOSURE-AUDIT.md`. G3s owns a global SDR tone-scale
comparison now. Keep the incident meter and low-light key. Compare a scene-referred
toe and shoulder against the current valley and the dawn, noon and dusk references.
Select mid-grey placement with the curve and keep the output path extensible to EDR.
Use [Polyphony's GT7 reference](https://s3.amazonaws.com/gran-turismo.com/pdi_publications/s2025_PBS_Physically_Based_Tone_Mapping_GT7.pdf)
as a candidate. The audit's performance figures are estimates.

The first Metal capture comparison retains the current SDR curve. The per-channel
scalar GT7 probe omits the reference's colour handling and washes out warm sunset
colour. The shadow-only luminance probe preserves metered grey and the existing
highlight path. It moves the twilight floor median from 67.30 to 64.22 code units
and leaves the sampled sunset sky median at 237.47. The foreground still looks
flat next to the CC0 Kiara dawn, noon, sunset and dusk photographs. These are
uncalibrated visual comparisons. Try the full colour-aware output transform against
the unchanged scene, then test terrain and material contrast separately. The
meters, low-light key and shipping display shader are unchanged.

The three binaries, ten matched stills per binary, `compare.py`, `compare-all.json`,
`contact-all.png` and input hashes are in `tmp/handback-review/tone-scale/`. Run
`sh tmp/handback-review/tone-scale/build-pair.sh` from the root to fill missing
binaries. For a fresh capture set `THRUM_ASSET_STORE` to the absolute path of
`tmp/g3s-showcase/cooked`, `THRUM_SCENE=valley` and `THRUM_CAPTURE_SCENE` to a new
output directory, then run `v1-game-neutral`, `v1-game-scalar` or `v1-game-anchored`
from the study directory on Metal. These captures establish visual output only.

- **Night and EV floor remain open with G3s.** The reviewed
  [night milestones](../../design/modules/ATMOSPHERE-PLAN.md) pair the key and
  storage correction with moonless light and then stars. The current renderer
  remains Sun-only. Lunar illumination and mesopic colour remain open.
- **G3t/G4 own HDR.** G3t owns storage and the native boundary. G4 owns the shared display
  mapping and SDR/EDR comparison, including history scale and dynamic headroom.
- **G4 camera work owns occlusion-aware and local exposure.** Keep the open-sky incident
  mode for the valley. Compare a view- or position-dependent meter when there is a shaded
  consumer. Reassess local exposure on the backlit route after the global curve. Include
  the later cloud provider at the camera-light seam.

## Claude root session (6–7 November)

**Handback:** see [the Codex handback](CODEX-HANDBACK-2026-11-07.md) for everything landed, what is verified,
open items in priority order, the G6 boundary state and owner questions. Codex resumes as root.

The Claude session covered the Codex quota pause. Its oversight log is
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
  Chrome now also draws the cooked valley (G7). Live cadence and calibrated Earth radiance remain open.
- The foreground read flat and bright against Kiara dusk until the low-light key below. Some low-Sun contact marks and
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
- The image review flagged a stair-step far ridge. In the 1280×720 west-facing sunset still,
  its edge tracks the source mesh within 0.48 px at p95. Doubling mesh density on both axes
  moves the projected edge by at most 0.74 px. Keep the mesh for this view and evaluate
  edge reconstruction in G4. Raw projection and measured rows are in `tmp/handback-review/ridge/`.
- In the same view the near-floor strip starts where depth jumps from 1.68 km to 59 m.
  The near pixels at the strip keep their RGB values with aerial colour bypassed. They
  cover a Sun-facing slope in the authored ground. Keep the aerial path. The paired sample
  and geometry probe are in `tmp/handback-review/ridge/`.
- The 100-code foreground step at solar 77→78 was a shadow leak from back-face-culled cooked
  casters with the Sun below horizontal. It is fixed ([evidence](G3S-SUNSET-CASTERS-2026-11-07.md)).
- Live cadence: `THRUM_CAPTURE_LIVE_TICKS` captures consecutive 60 Hz ticks. It exposed crest speckle at a
  0.6° Sun, now bounded by the height-map horizon below a 0.1 elevation sine
  ([evidence](G3S-LIVE-SUNSET-2026-11-07.md)). This is offscreen; on-screen ProMotion presentation is still G3b.
- Flat, bright twilight foreground: the incident meter now darkens below 1,000 lux on a
  Krawczyk key curve (about 2 stops at 10 lux, daylight unchanged)
  ([evidence](G3S-LOW-LIGHT-KEY-2026-11-07.md)). Earth radiance is still uncalibrated.
- Distant terrain shadows now come from a caster height-map horizon
  ([evidence](G3S-TERRAIN-HORIZON-2026-11-07.md)). Its +24% pass-sum is the next cost
  item; a per-frame horizon-angle map is the scalable form.
- Noon sky banding was 8-bit contouring; the display now dithers
  ([evidence](G3S-DISPLAY-DITHER-2026-11-07.md)).
- Low-Sun in-scatter over terrain hidden from the Sun is now shadowed
  ([evidence and shared-load measurements](G3S-SHADOWED-AERIAL-2026-11-07.md)).
- The penumbra's +18% pass-sum. Gathered fetches and a lit early-out are untried.

**G7.** Chrome now draws the cooked valley fetched from `thrum-cook watch --serve`
([evidence](G7-CHROME-VALLEY-2026-11-07.md); `check_v1_browser.mjs --case valley`). Its long poll swaps in edited
publications live, uploading only changed products. Device recovery with the valley attached
is a browser case (`--case valley-recovery`). The typed controls and parameter files are live.
G7 still owes the shared demo route and its startup, pacing and edit-hitch measurements.

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
