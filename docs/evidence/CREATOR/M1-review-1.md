# CREATOR M1 review: the starter scene runs in Chrome from one command

11 November 2026. Reviewer: Claude Opus 5.5 (high), a fresh agent that saw none of the building. I judged commit
`00f37d8d` in a clean checkout, built it, and ran it in Chrome 157 (new headless) on this M3 Max with hardware
Metal WebGPU. The page's own adapter reported `vendor=apple, architecture=metal-3, isFallbackAdapter=false`
([edge-run.json](M1-review-1/edge-run.json)). Captures are in [M1-review-1/](M1-review-1/).

**Result: FAIL.** One item fails (M1.3), and one Must Fix lies outside the items (the starter no longer compiles
on Linux or Windows).

| Item | Verdict | Reason |
|---|---|---|
| M1.1 | pass | The check passed on my run. The question's answer is yes, with one weakness (Suggestion S3). |
| M1.2 | pass | The check passed. The page renders only what the model reports, and edits go through the Mac panel's setters. |
| M1.3 | **fail** | The check passed. But after a refused edit is fixed, the page keeps saying the old generation "stays drawn" and asks for a fix, while a newer generation is drawn (F1). |
| M1.4 | pass | The source has 40 game lines and no `cfg`. It names only kit types. |
| M1.5 | pass | All six states are legible, and each message names its object, cause and next step. |

## Must Fix

### F1 (M1.3, verified): a fixed edit leaves a stale refusal on the page for good

**Steps.** I ran `./dev starter --web` on a copy of the assets and opened the URL. Generation 1 drew. I renamed
`meshes/rock.gltf` to `rock.gltf.renamed`. The page correctly said:
`Cooked generation 1 drawn. Generation 1 stays drawn. mesh meshes/rock with texture textures/white: generation 2:
… fix the named asset and save it again` ([edge-refused.png](M1-review-1/edge-refused.png)). I then renamed the file
back. The watcher published generation 3, and the page drew it.

**What happened.** The status became `Cooked generation 3 drawn. Generation 1 stays drawn. mesh meshes/rock … fix
the named asset and save it again` and stayed that way ([edge-after-fix-stale.png](M1-review-1/edge-after-fix-stale.png),
`s5_after_fix` in [edge-run.json](M1-review-1/edge-run.json)). The page contradicts itself. It tells the creator the
fixed asset is still broken until the page is reloaded.

**Expected.** Once a later generation draws, the refusal is cleared or marked as resolved. Refuse, fix, save is the
normal creator loop, and the bar asks for function and legibility.

**Where.** `crates/thrum-app/src/sky/browser.rs:198-215`, in `BrowserScene::report`. `self.kept` is overwritten only
by a new refusal or a transport error, and nothing clears it when a newer generation draws.

**Why the check missed it.** The `edit` case stops at the refusal and never saves a fix. **Proposed sharpening:**
after the refusal, the `edit` case restores the mesh. It then asserts that a newer generation draws, that the
submitted pixels match the pre-refusal image, and that the status no longer contains "stays drawn".

### F2 (outside the items, verified at compile level): the starter no longer builds on Linux or Windows

**What changed.** Before this commit, `examples/starter` depended on `thrum-app` only on macOS. It also had a
`#[cfg(not(target_os = "macos"))] fn main() {}`, so the workspace compiled on every host. The commit removed both,
to meet M1.4. But `thrum_app::sky` is still gated `#[cfg(any(target_os = "macos", target_arch = "wasm32"))]`
(`crates/thrum-app/src/lib.rs:26-27`).

**Steps.** I ran `rustup run 1.99.1 cargo check --locked --offline -p thrum-starter --target aarch64-linux-android`
with the pinned rustc (no sccache). It failed with:

```
error[E0433]: cannot find `sky` in `thrum_app`
 --> examples/starter/src/main.rs:9:16
note: found an item that was configured out --> crates/thrum-app/src/lib.rs:27:9
```

I couldn't compile for Linux or Windows directly, because neither target is installed. Their `cfg` is the same as
Android's (neither macOS nor wasm32), so the error is the same.

**Impact.** The README documents `./dev check` and `./dev.ps1 check` on Windows. The CI matrix runs `./dev check`
(`cargo clippy --workspace`) on `ubuntu-latest` and `windows-latest`. Both now stop at the starter. Stones and the
lighthouse keep their gates, so they still compile.

**Expected.** The scene stays free of `cfg` (M1.4), and the kit owns the platform split. One fix is a
`thrum_app::sky::run` on other targets that exits with a message naming the supported targets. Another is a
target-gated module inside the kit.

**Proposed item:** `./dev check` (or `check-mobile-core`) compiles `thrum-starter` for one non-Mac native target,
for example `aarch64-linux-android`.

## Suggestions

- **S1. The cause in the asset messages is misleading** (verified). A mesh renamed away, or a broken glTF at startup,
  is reported as `malformed cooked product: asset unavailable in the requested profiles`. Nothing is malformed: the
  mesh is missing, or its source failed to import. See [edge-broken-mesh-at-start.png](M1-review-1/edge-broken-mesh-at-start.png).
  The real cause (`failed meshes/rock: source import failed: key must be a string at line 1 column 3`) appears only
  in the terminal. A malformed glTF saved while the page runs leaves the page silent, because the cooker refuses
  before publishing (`s3_malformed`). This matches the Mac, where cook failures also go to the terminal, so it is
  not a Must Fix.
- **S2. Some next steps are duplicated** (verified). A failed load reads `…; fix the asset and save again; fix the
  named asset and save again.` (`sky.rs:436` and `browser.rs:211` both append one). The missing-store message gives
  two overlapping next steps ([journey-missing-store.png](M1-review-1/journey-missing-store.png)). Both are legible, but
  each should have one.
- **S3. The hardware gate reads the wrong adapter.** The scene case tests `SystemInfo.getInfo` (the browser's GPU
  list and `featureStatus.webgpu`), not the adapter the page received. A Chrome forced to a SwiftShader WebGPU
  adapter could still list the M3 Max. The page shell's `request_adapter` doesn't refuse a fallback adapter either.
  Assert `adapter.info.isFallbackAdapter === false` and the vendor from inside the page. On my probe, that is what
  the page got.
- **S4. Web `run` ignores `Config`** (`browser.rs:442`). The page title is always "Thrum scene", not the scene's
  title.
- **S5. The starter's doc comment is Mac-only.** It mentions the panel, `L` and `material.wgsl`. On the web the
  material hook isn't loaded, and `starter.params.json` applies only with `?params=`. The README says so, but the
  source comment doesn't.
- **S6. The Mac `cooked_status` text changed.** It now has the `{names}: …; fix the asset and save again` prefix
  (`sky.rs:432-437`). The Mac live-reload check matches `content: kept`, which is a different path. I did not run
  `scripts/check_starter_live_reload.py`, which belongs to M3.3.
- **S7. Pre-existing duplication.** The group heading transform is written twice, once in `thrum-controls.js`
  (`heading`) and once in `macos.rs:387`. They give the same output today.

## What I ran

| Command or action | Result |
|---|---|
| `node scripts/check_starter_browser.mjs --case scene` | pass. 517,929 pixels changed on the Sun edit. Mesh and texture edits changed the submitted pixels. ([check-runs.json](M1-review-1/check-runs.json)) |
| `node scripts/check_starter_browser.mjs --case controls` | pass. DOM keys and labels match the model. The params refusal names `sun.elevaton` and suggests `sun.elevation`, and the valid 7° applies. |
| `node scripts/check_starter_browser.mjs --case edit` | pass. Generations 1→2→3 drew, and the refusal kept generation 3's pixels byte-identical. |
| `node scripts/check_starter_browser.mjs --case journey` | pass. |
| `./dev check-web` | pass. |
| `./dev check` | pass (exit 0) on this Mac. This includes the V1 `frame_alloc` suite. |
| My edge driver ([edge-driver.mjs](M1-review-1/edge-driver.mjs)) | A real pointer drag on the Sun slider reached the model (display `0.0°`, model 0; [edge-pointer-drag.png](M1-review-1/edge-pointer-drag.png)). Rename, then fix: **F1**. Malformed glTF while running: the page is silent (S1). Watcher killed while running: the page names the store URL and the restart step, and the last generation stays drawn ([edge-store-gone.png](M1-review-1/edge-store-gone.png)). |
| Copied scene: `cp -R examples/starter examples/revscene`, renamed and registered as the README says, then `./dev scene examples/revscene --web` | The lockfile was updated, the page was served, and generation 1 drew ([edge-copied-scene.png](M1-review-1/edge-copied-scene.png)). All temporary changes were reverted. |
| Broken mesh at startup | The page draws ground and sky, and names the mesh (S1, S2). |
| `./dev scene examples/nope --web`, `./dev starter --wbe` | Both are refused with a next step. |
| Non-Mac compile of `thrum-starter` (Android target) | **fails** (F2). |

Edge conditions for chrome-webgpu: I ran low sun (3° in the scene check, about 0° by pointer drag). Storm, distance
and fast camera have no controls in the starter, so they don't apply. Full refresh and cadence are outside this
milestone, which covers function and legibility only, so I didn't judge them.

## Questions (`ask`)

- **M1.1.** Does the scene case run the documented command and drive real hardware Chrome? Yes. It spawns
  `./dev starter --web`, reads `SCENE_URL`, and launches the installed Chrome. Would it fail on a software adapter?
  Only through the browser-level GPU info (S3). An undrawn cooked generation or a fallback scene fails it, because
  the case also edits the mesh scale and the texture and asserts that the submitted pixels change. A slider that
  changes state but not pixels fails it too: the edit goes through the DOM `input` event, and the case compares
  submitted images with a threshold of 1,000 pixels.
- **M1.2.** Is anything declared separately for the web? No control, group, range or refusal text is.
  `thrum-controls.js` builds every widget from `describe_json`. `BrowserScene::edit_control` calls `lookup`,
  `edit_display` and `edit_value`, the same setters as `MacPanel::apply_edits` (`macos.rs:223-246`). `?params`
  calls `Controls::apply_file`, and the page only prefixes the refusals with the file name. The one web-only string,
  "browser control input needs a number, boolean or string", can't be reached from the widgets.
- **M1.4.** Is there any `cfg`, web-only item or engine game type in the scene's source? No.
  `examples/starter/src/main.rs` has no `cfg` and no web items. It names only the kit's facade (`sky::run`,
  `Config`, `SkyGame`, `Controls`, `Camera`, `MeshPlacement`, `CookedMaterial`, `Display`). I counted 40 non-blank,
  non-comment lines, from one file built for both targets. The kit's web runner (`sky/browser.rs`, `web/scene.js`,
  `web/index.html`) holds all the bindings. Removing the `cfg` caused F2.

## M1.5 inspection

I wrote this list before looking. For each state, is the scene or the message readable, and does every message name
its object, cause and next step?

| State | Capture | Judgment |
|---|---|---|
| Loading (network throttled) | [journey-loading.png](M1-review-1/journey-loading.png) | Readable. It names what is loading (the WebGPU renderer and cooked assets), says to keep the page open, and says what to do if loading fails. |
| Drawn | [scene-drawn-canvas.png](M1-review-1/scene-drawn-canvas.png) (submitted output), [journey-tab-return.png](M1-review-1/journey-tab-return.png) (DOM) | Lit ground, sky and the cooked rock. "Cooked generation 1 drawn." Groups and values are readable. |
| Missing store | [journey-missing-store.png](M1-review-1/journey-missing-store.png) | Names the store URL and the cause (the fetch did not complete), then gives restart steps. The steps are redundant (S2), but legible. |
| Misspelt `?params` key | [journey-typo.png](M1-review-1/journey-typo.png), [controls-params.png](M1-review-1/controls-params.png) | Names the file and the key, and suggests the right key. The scene keeps drawing, and valid keys apply. |
| After a tab switch | [journey-tab-return.png](M1-review-1/journey-tab-return.png) | Real hidden and visible transitions. The submitted pixels after the return were identical to before the switch. |
| 480 px wide | [journey-narrow.png](M1-review-1/journey-narrow.png) | The canvas, status and controls fit with no horizontal overflow. "Exposure zones" wraps to two lines, which is still readable. |

Also seen (not on the list): the Sun edit at low sun ([scene-sun-edit-canvas.png](M1-review-1/scene-sun-edit-canvas.png))
and the builder's refusal state on my run ([edit-refused.png](M1-review-1/edit-refused.png)). The stale message after a
fix is F1, under M1.3.

## Code read

I read every changed line of `00f37d8d`: `thrum-app` (`config.rs` move, `lib.rs`, `sky.rs` gating,
`sky/browser.rs`, `web/index.html`, `web/scene.js`), `examples/starter`, `scripts/scene_web.py`,
`scripts/check_starter_browser.mjs`, `tools/xtask`, the architecture registration, and the docs, maturity and plan
edits.

I found no hard-coded results, canned data, test-only branches or weakened tests. The `game` and
`__creatorReloadWitness` globals on `window` are a debugging and check surface only.

The builder's `./dev check` record shows one unexplained V1 `frame_alloc` failure, which passed on reruns. My own
`./dev check` passed in full, so I could not reproduce that failure. It runs only on the Mac, so it doesn't show F2.
