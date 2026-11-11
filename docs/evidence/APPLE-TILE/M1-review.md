# APPLE-TILE M1 review: The valley selects a recipe and stores only actual future inputs

- **Reviewer:** Claude Opus 5.5 (high). I am independent of the builder. Review date: 11 November 2026.
- **Commit judged:** `bffdc07f447d4ae207cdfb5ab29151119e278b63`, in a clean checkout with an empty dirty diff. From it I
  built the following:
  - release `v1-game` with `draw-probe`, plus `apple_tile_residency`;
  - `thrum-cook`, and a freshly cooked valley store;
  - the release Wasm page, the iOS simulator app and the XCUITest runner.
- **Result:** all five items pass. There are no Must Fix findings. Five Suggestions follow.
- **First review:** `fails` was 0, so there was no earlier review to follow up.

Captures and summaries are in [M1-review/](M1-review/). The bulk sources stay in my scratch folder `tmp/review-m1/`. They
are native T2 spools, 2,886 Chrome frames and simulator recordings.

## What I ran

Every GPU run held the orchestrate `gpu` lock.

| Run | Result |
|---|---|
| `cargo test -p thrum-render --lib` for the recipe, storage, transient, `metal_recipe` and `opaque_depth` tests | 6 passed |
| The same two Metal switch tests with `--features creation-counters` | 2 passed. See S1 for why I ran this. |
| `apple_tile_residency` on my cooked valley | Portable 86,130,688 B. Both generations alive 171,032,576 B. Apple tile after the old host retires 86,163,456 B. These match the builder's figures exactly. The digests also match: portable `f8fa043b9369e8dc`, Apple `efbee660684e6e92` at generation 2. [residency.json](M1-review/residency.json) |
| `scripts/apple_tile_m1.py` (native recipes) | **PASS: 38 exact comparisons against the retained M0 baselines.** They cover 30 route checkpoints (SDR and scene-linear: portable, Apple tile and a portable repeat) and 8 T2 `still` checkpoints at 1920×1080 (sun and night, SDR and linear, both recipes). Sample and exposure identities match too. Every Apple-tile run header records `Ok(AppleTile)` and digest `efbee560684e6cdf`, the same digest as the builder's at generation 1. [native-comparisons.json](M1-review/native-comparisons.json) |
| T2 `realtime` route, matched by deterministic sample | 121 samples were common to the Apple-tile and portable runs, and all 121 have identical frame SHA-256. Submit gaps had a median of 8.31 ms and a max of 12.32 ms for Apple, against 8.29 ms and 12.34 ms for portable. The authored cut is the only luminance jump, at the same place in both runs. |
| T2 `realtime` variants (day → night → day) | Both recipes have identical per-frame luminance profiles. They switch at frames 120 and 360, with no one-frame spikes or black frames. |
| Chrome T2 `fixed`, day/night × SDR/scene-linear, from this commit | **1,924 of 1,924 frames have the same hashes as the pre-change manifests.** Day SDR matches the M0 baseline from clean `e09bf32e`. Night SDR and both linear captures match the builder's `a6761604` pre-change manifests, including all 8 exposure GPU words on every linear frame. [chrome-comparison-summary.json](M1-review/chrome-comparison-summary.json) |
| Chrome T2 `realtime`, day and night | 481 and 482 complete images, with ticks monotonic. Headless Chrome ran at a median gap of about 17 ms, and no cadence claim is made. |
| D1.4 check: `python3 scripts/run_v1_showcase.py moving` | PASS |
| `cargo tree -p v1-game --target wasm32-unknown-unknown` | There is no `objc2*` or Metal crate in the 125-line tree. `objc2-metal` appears only on the Apple target. |
| Simulator (iPhone 18 Pro, iOS 27.2), running my script `tmp/review-m1/sim_review.py` | Startup, stall, hide, replace and background/foreground all passed, followed by **rotation to landscape and back** (epoch bumped, no device rebuild, actor still). The log reports `recipe=Ok(Portable); registry=f0bba45f1b65747d; unified_memory=Some(false)`, the same digest as the builder's. **On a relaunch with `THRUM_RECIPE=apple-tile`**, the app refuses with `fault=frame GPU: scene prepare: Recipe(UnsupportedAppleTile)` and draws nothing. It does not mislabel portable. [sim-report.json](M1-review/sim-report.json) |

I did not rerun the full `./dev check`. The builder's rebased gate covers it, and I ran every item's named check and
proof myself.

**Edge conditions run:**
- **mac-metal:**
  - Low sun: the route at solar 17.3 and the sunset still.
  - Distance: the approach checkpoint and the far ridges.
  - Fast camera: the route's close approach and the cut.
  - Full refresh: T2 realtime at a 120 Hz opportunity rate, with no cadence claim.
  - Storm: not run. The product has no storm preset, and no M1 item covers weather.
- **chrome-webgpu:** the same scenarios, plus night.
- **ios-sim:** rotation and background/foreground were both run.

## Code read

I read every changed line in `2c9d77ee` and `bffdc07f`:
- the registry: `capabilities.rs`, `recipe.rs` and `apple.rs`;
- graph storage: `graph.rs`, `storage_tests.rs`, `target_storage.rs` and its tests, the `synthetic.rs`/`forward.rs`
  day-depth selection, and the `stars/tests.rs` rewrite;
- `runtime.rs` and `host.rs` storage reports, the web/browser linear capture and `capture.mjs`/`capture_v1_browser.mjs`;
- the iOS log line, the route and window record fields, and the three scripts;
- `architecture.toml`, `maturity.json`, `DONE.json`, `PLAN.md` and `LOG.md`.

Findings from the read:
- **Selection:**
  - `resolve()` returns `AppleTile` only for an explicit request on Metal with known unified memory and known transient
    savings. Unknown facts (`None`), Chrome and Vulkan refuse a forced Apple request, and any other string is
    `InvalidOverride`. With no request it returns Portable, so the default stays portable.
  - Wasm never reads the environment.
  - `capture_device` paths have unknown unified memory, so they refuse a forced Apple request rather than mislabel it.
- **Enabled facts, not offered:** `features`/`limits` come from `device.features()`/`device.limits()`, not from the adapter's
  offers. The digest hashes adapter, enabled features, limits, downlevel, unified memory, recipe and generation. No
  display or refresh state is included; that stays with the platform.
- **Eligibility:** it is derived per compiled variant from the live accesses. Only a transient, single-layer, single-mip
  texture qualifies, with exactly one `AttachmentClear` use and no store. Any sampled, copied, storage or load use, a
  later use, an explicit store or a non-transient scope makes the texture ineligible.
  - On the real valley only depth qualifies, and only in the day variants. The star variants sample it, so it is stored.
  - Apple tile therefore adds a separate `TRANSIENT_ATTACHMENT` day depth (Memoryless in `as_hal`), and the shared depth
    stays Private for stars. `forward.rs` picks the day depth only when `!draw_stars`. The same per-frame `draw_stars`
    selects the graph (`synthetic.rs:1814`, `:2405`), so the depth and the graph cannot disagree.
- **Signs of faking:** none found.
  - The diagnostic reads (`as_hal` storage mode, `currentAllocatedSize`) are explicit report calls, never made during
    pass recording.
  - No test was deleted. The stars test was widened over both recipes.
  - No baseline was refreshed. My fresh runs reproduce the M0 and pre-change hashes.

## Inspection lists (written before looking)

**D1.3 (mac-metal, native recipes):**
1. Sun still: sky gradient, Sun disc and glow, terrain shading, ridge silhouettes.
2. Night still: stars present and occluded by terrain. Night uses the stored depth path, so this checks that it is
   unaffected.
3. Mirror and local light: mirrored sphere, cube cluster, local-light pool and shadows.
4. Pre-cut and post-cut: no history smear across the cut.
5. Near plane and approach: correct depth ordering, no depth garbage. A wrong memoryless store or load would show here.
6. 1x route motion: no flicker, popping or shimmering shadows, with frames equal to portable at matched samples.
7. 1x variant switch: day↔night with no black, garbage or one-frame frames, and no new textures or pipelines.
8. Storage report: Memoryless versus Private under both recipes, and resident totals reconciled.

**D1.4 (chrome-webgpu):**
1. Full-frame day and night fixed frames, SDR and linear, byte-identical to pre-change.
2. Sun, stars, geometry, shadows, mirror and local lights present.
3. 1x realtime: continuous, no stalls or flashes.

**D1.5 (ios-sim):**
1. The diagnostic scene draws at startup and after each lifecycle step.
2. The log reports the recipe, digest and actual unified memory.
3. Background/foreground and rotation keep the lease, the world and the device.
4. A forced unsupported recipe is refused, not mislabelled.

Each entry was judged against the bar: the same complete cooked valley as portable, stable at normal speed.
- **D1.3:**
  - Entries 1–5 pass. See [night still](M1-review/native-apple-tile-night-1080p.png): stars stop at the ridge line and
    the sphere and rock are present. See [mirror/light checkpoint](M1-review/native-apple-tile-mirror-light.png): the
    mirror, cubes, pool and shadows are intact. All of these are byte-identical to the M0 portable baselines.
  - Entry 6 passes, and so does entry 7; see the [variant contact sheet](M1-review/native-apple-tile-variants-contact.png).
    The no-creation part of entry 7 holds only with counters enabled (S1).
  - Entry 8 passes. The Apple day variants show `RENDER_ATTACHMENT | TRANSIENT_ATTACHMENT` → `Memoryless`, and the star
    and portable depth show `Private`. Per slot, the unique count is 6 textures and 33.1 MB nominal for portable, and
    7 textures and 36.8 MB for Apple; fallback-only is 0.
  - **Reconciliation:** the driver totals show the memoryless day depth costs about 16 KB per slot in resident memory,
    so Apple tile has **no resident saving**. The design expects exactly this when stars keep a stored depth. The builder
    reports it honestly and makes no traffic-to-memory claim.
- **D1.4:** entries 1–3 pass. See [night fixed frame 270](M1-review/chrome-night-fixed-270.png) and the
  [realtime night contact](M1-review/chrome-realtime-night-contact.png). The night local-light pool is blown out, but it
  is byte-identical to pre-change (S4).
- **D1.5:** entries 1–4 pass. See [landscape after rotation](M1-review/sim-landscape.png) and the
  [forced Apple tile refused](M1-review/sim-forced-apple-tile-refused.png).

## Item results

- **D1.1 pass.**
  - The selection tests cover every backend × unified-memory × transient combination (eligible, ineligible and
    unknown), plus default, forced portable, forced Apple and invalid input.
  - The Metal test proves `capture_device` (unknown facts) refuses forced Apple.
  - Enabled features and limits are separate from offered ones, and display facts stay with the platform.
  - The digests repeat across the builder's runs and mine on both Mac and simulator.
- **D1.2 pass.**
  - The mutation tests change only the affected variant for added readers (sampled, copy, load), an explicit store and
    history/persistent/imported scopes. Layer and mip limits are enforced.
  - `as_hal` storage logs show Memoryless versus Private under both recipes.
  - Unique allocations and bytes, fallback-only resources (0) and the old/new overlap are reported and reproduced.
  - No stored twin exists for an override: the stored depth exists because stars read it.
  - A whole-recipe change builds a complete second host before the old one is dropped.
  - The allocation-level caveat is in S2.
- **D1.3 pass.** Both recipes produce exact SDR and scene-linear route and still bytes against the paired and pre-change
  portable controls. Matched realtime samples are identical. The variant switch creates no texture or pipeline; this
  is proved with counters on (S1). The visual inspection passes.
- **D1.4 pass.** The named check passes. The Wasm tree excludes the native Apple crates. All 1,924 fixed frames across
  day/night × SDR/linear match pre-change, with identity and exposure words matching. Realtime motion is intact.
- **D1.5 pass.** The simulator registry reports actual Metal facts (`unified_memory=Some(false)`) and runs Portable. The
  diagnostic scene draws through all lifecycle steps and rotation. An unsupported forced Apple request is refused.
  This is function-only evidence, as the item scopes it.

## Findings

No Must Fix findings.

**S1. The variant-switch "no texture or pipeline creation" assertions are vacuous in the ordinary gate.**
*Suggestion, verified.*
- **The defect:** `stars/tests.rs` compares `device.get_internal_counters().hal.textures/render_pipelines/compute_pipelines`
  before and after a variant switch. Without wgpu's `counters` feature, `InternalCounter::read()` always returns 0
  (`wgpu-types-30.0.1/src/counters.rs:36-41`). Only `thrum-render/creation-counters` enables that feature, and neither
  `./dev check` nor any workspace crate turns it on.
- **Impact:** the assertion passes even if a switch allocates. I reran the test with `--features creation-counters`
  and it still passes, so the claim is true today. A regression would go unnoticed, though.
- **Fix:** gate those asserts on `cfg(feature = "creation-counters")` and run that test with the feature in the
  gate. Or assert a non-zero texture count before the switch, so a disabled counter fails loudly.

**S2. Graph textures are allocated from the capture (readback) variant.**
*Suggestion, verified by reading; no current input reaches it.*
- **The defect:** `OffscreenTarget::with_capture_source` calls `GraphTextures::new(..., &night.unwrap_or(&day).readback, recipe)`
  (`synthetic.rs` near line 2247). `prepare_day_depth` also requires `day.readback` eligibility. Today capture reads
  only scene and output, so nothing changes.
- **Impact:** a future capture-only depth reader would turn the *normal* day depth back to stored. The design says
  capture variants must not change the normal path.
- **Fix:** derive the normal-path allocation from the non-capture variants, and give a capture-only reader its own
  stored allocation in a capture-enabled generation.

**S3. Chrome scene-linear manifests mislabel their encoding.** *Suggestion, verified.*
- **The defect:** `scripts/capture_v1_browser.mjs:176` always writes
  `encoding = {stage: 'final SDR engine output…', channel_order: 'RGBA8', row_stride: width*4}`. It does so even for
  `--source scene-linear`, whose frames are RGBA16Float with stride width×8. The builder's
  `chrome-current-night-linear/manifest.json.gz` and my own linear runs carry the wrong label.
- **Impact:** the pixel bytes and comparisons are correct. This is the same kind of metadata error M0's
  `linear-stride-erratum.json` corrected on native.
- **Fix:** derive the encoding fields from `args.source`.

**S4. The midnight route's local-light pool saturates to flat white in SDR.**
*Suggestion, verified; it predates M1 and is outside its scope.*
- **What I saw:** Chrome night fixed frames 000000 and 000270 and the realtime night sequence show the pool, nearby cubes
  and the sphere's lit side clipped to white. See [frame 270](M1-review/chrome-night-fixed-270.png) and the
  [contact sheet](M1-review/chrome-realtime-night-contact.png). This would not pass in a shipped game.
- **Why it is not an M1 defect:** it is byte-identical to the pre-change build, and M1's own items require exactly
  those bytes.
- **Where it belongs:** route it to the module that owns local-light authoring or exposure for night scenes.

**S5. The storage report's "nominal bytes" counts the memoryless day depth at full size.** *Suggestion.*
- **What happens:** the README table (Apple 14 textures / 73.6 MB against portable 12 / 66.2 MB) reads as if Apple tile
  costs 7 MB more. The driver total shows about 32 KB more.
- **Fix:** label memoryless allocations separately, as a resident estimate of about 0 next to the logical size, so
  later readers don't misread the table.

**Item strength:** D1.5's selection proof is weak on the simulator. With portable as the default, the default run would
select portable whatever the facts are. The forced-Apple refusal I ran is what shows the facts are actually consulted.
A later revision of the item could name that refusal explicitly.

PASS M1
