# Handback to Codex — 7 November 2026

Claude acted as Thrum root from 6–7 November while the Codex agent was paused for quota. This is the handback.
Start from remote `main` and [the handoff](G3S-HANDOFF.md). Every item below was pushed straight to `main` as a
verified checkpoint, each with an evidence page and a maturity-ledger update. Claude writes nothing further to
Thrum after this commit.

## Landed, in order, and why

| Commits | What | Why | Evidence |
|---|---|---|---|
| `a9af1b9`…`51ec760` | Finite-Sun penumbra from `codex/wip-g3s-solar-penumbra` landed, with a receiver-bending bound, halved taps and an unpadded near map | The draft's stipple was false blocking at concave joins, found with a CPU ray probe. Padding grew the near map about threefold and faded sphere contact | [penumbra](G3S-SOLAR-PENUMBRA-2026-11-07.md) |
| `5f71e66`, `663437c` | Back-facing shading normals are lifted, not reflected | A Sun-facing slope at the floor's far edge rendered grey-blue | [penumbra](G3S-SOLAR-PENUMBRA-2026-11-07.md) |
| `ec673de` | Terrain beyond the cascades is shadowed by a top-down caster height-map horizon | Distant ridges were unshadowed | [terrain horizon](G3S-TERRAIN-HORIZON-2026-11-07.md) |
| `6dfd415` | Cooked terrain casters are two-sided | With the Sun below horizontal, culled back faces left shadow holes, causing a 100-code foreground step at solar 77→78 | [sunset casters](G3S-SUNSET-CASTERS-2026-11-07.md) |
| `72d9985`, `1b3602b`, `111115b` | `THRUM_CAPTURE_LIVE_TICKS`. Low-Sun crest speckle bounded by min(cascade, horizon) below an elevation sine of 0.1 | Live 60 Hz ticks exposed holes in the last cascade at a 0.6° Sun. The sky brightening at sunset is incident metering, not a radiance error | [live sunset](G3S-LIVE-SUNSET-2026-11-07.md) |
| `d973bb1`, `4370685` | Opt-in low-light exposure key (Krawczyk 2005), normalised at 1,000 lux | The twilight foreground read flat and as bright as noon against the CC0 Kiara dusk photographs | [low-light key](G3S-LOW-LIGHT-KEY-2026-11-07.md) |
| `ed420a5` | G7 slice 1: Chrome draws the cooked valley fetched from `thrum-cook watch --serve` | Native and web share the authored scene | [Chrome valley](G7-CHROME-VALLEY-2026-11-07.md) |
| `8abbb54` | Live asset edits in Chrome: long poll, `RemoteFeed`, reuse of unchanged products | G7 needs the asset live-edit path through the G5 seam. An edit uploaded 5,376 B, against 1.5 MB for the first load | same |
| `8d52cd6` | `--case valley-recovery`: two real device replacements with the valley attached | Recovery with cooked content had no browser coverage | same |
| `be7be9d` | Fixed triangular-PDF dither of one step at the Unorm8 display encode | The reported noon banding was 8-bit contouring | [dither](G3S-DISPLAY-DITHER-2026-11-07.md) |
| this commit's parent | Aerial in-scatter shadowed by a per-frame Sun shadow-height map | Low-Sun views glowed orange over terrain hidden from the Sun, with a hard near-floor step | [shadowed aerial](G3S-SHADOWED-AERIAL-2026-11-07.md) |

G6's controls slice landed separately from its own session (`24338a8`). Root reviewed its seams; see below.

## Verified and not verified

- **Verified.** Each landing passed `./dev check` on the real Metal host and the Chrome 156 WebGPU cases listed on
  its page. Wasm hashes are recorded there. Each also had an independent fresh-context, image-only review of
  unlabelled before/after stills, and every reviewer preferred the candidate.
  - Byte-exact native capture comparisons are on each page.
  - Raw runs are under `tmp/claude-root/g3s-shadow/` and `tmp/g3s-showcase/`. They are local and uncommitted.
- **Not verified:**
  - On-screen ProMotion presentation and cadence: that is still G3b, and the captures are offscreen.
  - Chrome frame time and startup.
  - The cost of the shadowed-aerial pass. Its time counts in the far-shadow bucket, unmeasured.
  - Calibrated Earth radiance.
  - The Mac G6 panel on screen; it was verified offscreen only because the Mac was locked.
  - iOS was not run.
- **Shared-GPU timings.** All timings recorded today were contended. They are labelled as such and are not
  baselines.

## Open, in priority order

1. **Cost of the new passes.**
   - Measure the shadowed-aerial pass and the 8-interval aerial loop: ABBA against `be7be9d`, with the
     `q8-production` runner.
   - Then replace the receivers' per-pixel 96-step horizon march with lookups in the same Sun shadow-height map.
     That march is the +24% terrain-horizon item, and doing this removes duplicated work.
2. **Compute after render on Metal (`G3S-SHADOWED-AERIAL`, Determinism).**
   - A compute pass that read a depth layer drawn earlier in the frame produced nondeterministic pixels. The same
     work as a render pass is deterministic.
   - Reproduce it minimally in `thrum-render` and decide whether it is a wgpu Metal ordering bug or a missing
     declaration in our graph.
   - Audit other compute passes that read just-rendered targets. The exposure resolve reads the scene; its tests
     are deterministic today.
3. **Sky light shafts.** Sky-view pixels are not shadowed by terrain. Apply the same visibility on the sky path.
4. **Look items left from review.**
   - A faint light strip just below the near-floor silhouette at low Sun.
   - Hard shading terminators on lit ridges at sunset.
   - A staircase far-ridge silhouette (terrain geometry).
   - Score any shadow finding with `tmp/g3s-showcase/probe-rays.py` before changing anything.
5. **Sky-to-ground balance.** This is a camera choice. A sky-weighted meter or a stronger key would move toward the
   Kiara photographs. It is untuned and needs owner taste.
6. **G7.** The demo panel composes G6's typed controls. Next are the Chrome demo page (`?assets` plus `?params`)
   and measurements.
7. **The penumbra's +18% pass-sum.** Gathered fetches and a lit early-out are untried.

## G6 boundary state

G6 is a separate Claude session in worktree `claude/g6-creator-experience`. The full record is
`tmp/g6-boundary/NOTES.md`. Root approved:
- **`thrum-controls`**, the typed model, AppKit panel and DOM builder, plus game bindings in `src/controls.rs`.
  - `sky.solar_time` goes through the checked setter.
  - `world.paused` goes through the existing user-pause path (`PauseIntent::request` plus `reconcile_pause`).
  - These landed as `24338a8`, with `./dev check` and six Chrome cases passing.
- **Overlays**, phased: readouts first. A GPU overlay slot is proposed in NOTES before any graph change. It
  composites in encoded display space and stays undithered, or applies the same dither.
- **`thrum-app` extraction**, with conditions:
  - one behaviour-preserving commit after the controls slice;
  - G6 messages root before it starts and before it pushes;
  - showcase, cooked, content, capture, scene_source, camera and sky setup stay in v1-game;
  - hooks only, with no game types;
  - narrow dependencies in `.config/architecture.toml`;
  - root reviews the diff;
  - not started at handback. Codex inherits the review.
- **Parameter files in the game layer**, approved with three asks:
  1. `?params` works alongside `?assets`.
  2. The params case asserts `#status` stays clean.
  3. A Mac apply while paused holds the world tick.
  - Reload is explicit: `L` on the Mac, a button in Chrome. No G5 crate or watcher change.
  - Landed as `9b4bea6` with all three asks met (`params` case, six cases and `./dev check` passing).
- **Held:** `ProductKind::Params` as a fourth G5 product kind, because G5 extensions are parked. See the owner
  questions.

## Owner questions

- **Parameter files through G5.** Should they ride the G5 watcher and dev server as a fourth product kind?
  Recommendation: yes, once G5 is un-parked. It reuses the one watcher, server and atomic publication.
- **Sky-to-ground exposure balance.** Keep incident metering with the low-light key, or move toward the
  sky-exposed photographs?
- **Disk space.** It ran to 100% full on 7 November and later recovered. Bulk captures under `tmp/` should be
  pruned under the storage rules.

## What I'd do differently from the earlier approach, and why

- **Get an independent image review before each push, not only at gates.** Every look change today went to a
  fresh-context, image-only reviewer with unlabelled pairs. That caught a crease line from an interim fixed-slope
  tolerance that the numbers missed.
- **Measure truth before tuning shadows.** The CPU ray probe (`probe-rays.py`) settled the penumbra stipple and the
  sunset step: one was a false blocker, the other a caster hole. Neither was a filter-width problem. Before
  changing any shadow parameter, score against ray truth.
- **Use consecutive live ticks, not sparse solar samples.** `THRUM_CAPTURE_LIVE_TICKS` found crest speckle that the
  97-sample crossing hid. Sparse samples also made a real ridge-shadow crossing look like a pop in Chrome.
- **Fix causes instead of compensating.** The bright twilight was a metering cause, not a colour gain. Sunset sky
  brightening was explained as incident metering rather than tuned away. The noon banding was quantisation, not
  the sky model.
- **Prefer render passes for reads of fresh depth on Metal** until the ordering question in open item 2 is
  settled.
