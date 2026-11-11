# CREATOR M1 review, round 2: the starter scene runs in Chrome from one command

11 November 2026. Reviewer: Claude Opus 5.5 (high), a fresh agent that saw none of the building. I judged commit
`42fdce97` in a clean checkout. The milestone's code is the builder's fix commit `8244a46e` on top of round 1's
`00f37d8d`. I built and ran it in Chrome 157 (new headless) on this M3 Max. The page received the hardware adapter
`vendor=apple, architecture=metal-3, isFallbackAdapter=false` in all four check runs
([r2-check-runs.json](M1-review/r2-check-runs.json)). Round-2 captures are the `r2-*` and `edge-*-r2*` files in
[M1-review/](M1-review/). Where a capture of mine was byte-identical to round 1's, I deleted my copy and link the
round-1 file, because the pixels are the same.

**Result: PASS.** Both round-1 Must Fix findings are fixed, and every item passes.

| Item | Verdict | Reason |
|---|---|---|
| M1.1 | pass | The check passed on my run. The scene case now also asserts the adapter the page itself received. |
| M1.2 | pass | The check passed. The page renders only what the model reports, and edits go through the Mac panel's setters. |
| M1.3 | pass | The check passed, and now covers refuse, then fix. My own refuse, fix, refuse again, fix cycle cleared and restored the message each time. |
| M1.4 | pass | One 40-line source with no `cfg`. It names only kit types, and the kit now owns the platform split. |
| M1.5 | pass | All six states are legible, and each message names its object, cause and next step. |

## Round 1 Must Fix findings

### F1 (M1.3): a fixed edit left a stale refusal on the page. **Fixed** (verified)

`BrowserScene::report` (`crates/thrum-app/src/sky/browser.rs:201-238`) now keeps the refusal with the generation
that stays drawn. It clears the refusal once a later generation is drawn. Transport errors are held separately.

I ran [edge-driver-r2.mjs](M1-review/edge-driver-r2.mjs) on a copy of the assets, served by `./dev starter --web`
([edge-run-r2.json](M1-review/edge-run-r2.json)):

1. Generation 1 drew ([r2-a-drawn.png](M1-review/r2-a-drawn.png)).
2. I renamed `meshes/rock.gltf` away. The page said `Generation 1 stays drawn. mesh meshes/rock …; fix the named
   asset and save it again`, and the submitted pixels were unchanged ([r2-b-refused.png](M1-review/r2-b-refused.png)).
3. I saved the texture again while the mesh was still missing. The cooker published nothing new, and the
   refusal stayed.
4. I restored the mesh. The page said only `Cooked generation 3 drawn.`, and the submitted pixels matched
   generation 1's ([r2-d-fixed.png](M1-review/r2-d-fixed.png)).
5. I renamed it away again. The refusal came back, naming generation 3 as the one that stays drawn
   ([r2-e-refused-again.png](M1-review/r2-e-refused-again.png)). Clearing doesn't hide later refusals.
6. I restored it again. The page showed `Cooked generation 5 drawn.` with the same pixels
   ([r2-f-fixed-again.png](M1-review/r2-f-fixed-again.png)).

I also started the scene with the mesh already missing. The page drew ground and sky and said `Asset load failed:
mesh meshes/rock …; fix the asset and save again.` ([r2-i-start-broken.png](M1-review/r2-i-start-broken.png)).
When I restored the mesh, generation 2 drew without a reload
([r2-j-start-broken-fixed.png](M1-review/r2-j-start-broken-fixed.png),
[edge-run-r2-start-broken.json](M1-review/edge-run-r2-start-broken.json),
[driver](M1-review/edge-driver-r2-start-broken.mjs)).

The `edit` case now restores the mesh. It asserts generation 5 is drawn, that the status has neither "stays drawn"
nor "fix the named asset", that the pixels match the pre-refusal image, and that the page and scene are the same
objects (`scripts/check_starter_browser.mjs:237-245`). On my run of the case, the page after the restore is in
[r2-edit-restored.png](M1-review/r2-edit-restored.png). The builder's
[reproduction](m1-r1/reproduction.txt) shows the case failing before the fix.

### F2 (outside the items): the starter no longer built on Linux or Windows. **Fixed** (verified at compile level)

`thrum_app::sky` is now compiled on every target (`crates/thrum-app/src/lib.rs:26`). The pieces that only the Mac
window or the page uses are gated inside the kit. On other native targets, `sky::run` prints `<name>: the native
sky window is unavailable on <os>; run ./dev scene <folder> --web in Chrome, or run the native scene on macOS` and
exits with status 1 (`sky.rs:867-880`). The starter's source has no `cfg`.

**Steps.** I used the pinned rustup 1.99.1 toolchain. Homebrew's rustc is first on `PATH` and has no Android
std. I set a stub C compiler for `blake3`'s build script, because this check doesn't link. Then I ran
`cargo clippy --locked --offline -p thrum-app -p thrum-starter --all-targets --target aarch64-linux-android -- -D warnings`.
It finished clean, with no dead-code or unused warnings. Android has the same `cfg` as Linux and Windows here
(neither macOS nor wasm32), so the CI clippy that failed in round 1 now gets past these crates. I could not run
the workspace-wide clippy for Android, because `astcenc-sys` needs a real C++ cross compiler.
`./dev check-mobile-core` passes and now checks `thrum-starter` for `aarch64-apple-ios-sim`.

## Must Fix

None.

## Suggestions

- **S1. The asset-refusal cause is still misleading** (verified, still open from round 1). A missing mesh is
  reported as `malformed cooked product: asset unavailable in the requested profiles`
  ([r2-b-refused.png](M1-review/r2-b-refused.png), [r2-i-start-broken.png](M1-review/r2-i-start-broken.png)).
  Nothing is malformed. A malformed glTF saved while the page runs still leaves the page silent. The status stayed
  `Cooked generation 5 drawn.` ([r2-g-malformed.png](M1-review/r2-g-malformed.png)), and the cause appears only in
  the terminal. The plan records this as a follow-up.
- **S8. The non-Mac compile of the starter isn't in `./dev check`.** It runs only in `./dev check-mobile-core`,
  for the iOS simulator. `./dev check` on a Mac would not catch F2 coming back. Only the manual CI on Ubuntu and
  Windows would. Adding the iOS-simulator `cargo check -p thrum-starter` (a few seconds) to `./dev check` keeps
  the guard local. `./dev help` still describes `check-mobile-core` without the starter probe, although
  `CONTRIBUTING.md` was updated.
- **S9. Status spacing.** `report` always formats `"{content}. {refusal} {transport}"`. With no refusal, the status
  ends in two spaces (`Cooked generation 1 drawn.  `). After the watcher stops, it reads `Cooked generation 5
  drawn.  asset store http://…: fetch request did not complete; check the store URL and restart the scene command`
  ([r2-h-store-gone.png](M1-review/r2-h-store-gone.png)). The message names the store, the cause and the next
  step, but its sentence starts in lower case after a double space. Join only the parts that are present.
- **S10. The edit check stops at one refuse and fix.** It doesn't refuse again after the fix. A clear that also
  suppressed later refusals would pass it. My step 5 above covered this by hand.
- **S7. Pre-existing duplication** (still open). The group-heading transform is written in both
  `thrum-controls.js` and `macos.rs`.

## A note for the loop, not the milestone

`./dev check` fails at this commit in its first step, `scripts/repo_check.py`. It reports 540 missing local links
and anchors, and eight missing designs under `modules/`. Every one comes from `42fdce97` ("orchestrate: move the
plans into a folder per module, under docs/plans"), the loop's own commit after the claim. At the builder's claim
commit `f5a40b27`, the same guard passes ("Repository checks passed: 629 Markdown files, 3002 local links, 30
module records"). I ran every later step of `./dev check` separately (table below). The links need fixing on
`main`, but the milestone didn't cause them.

## What I ran

| Command or action | Result |
|---|---|
| `node scripts/check_starter_browser.mjs --case scene` | pass. The page received the hardware adapter, and its title is "Thrum starter". The Sun edit changed 517,929 pixels. Mesh, texture, refusal and restore pass. |
| `node scripts/check_starter_browser.mjs --case controls` | pass. The params refusal names `sun.elevaton` and suggests `sun.elevation`. Azimuth 9999 is refused with the range. |
| `node scripts/check_starter_browser.mjs --case edit` | pass. Generations 1→2→3 drew, 4 was refused with 3 kept, and 5 restored the pixels with the message cleared. |
| `node scripts/check_starter_browser.mjs --case journey` | pass. |
| [edge-driver-r2.mjs](M1-review/edge-driver-r2.mjs) | F1 is fixed through two refuse and fix cycles. The 480 px refusal fits with no horizontal overflow ([r2-e-refused-480.png](M1-review/r2-e-refused-480.png)). Malformed glTF: the page is silent (S1). Watcher stopped: the page names the store and the restart step, and generation 5 stays drawn (S9). |
| [edge-driver-r2-start-broken.mjs](M1-review/edge-driver-r2-start-broken.mjs) | Starting with a broken mesh is reported, and fixing it draws without a reload. |
| Android clippy of `thrum-app` and `thrum-starter`, all targets, `-D warnings` | pass (F2). |
| `./dev check-mobile-core` | pass, including the new starter probe for the iOS simulator. |
| `./dev check-web` | pass. |
| `./dev check` | fails at `repo_check.py` on the loop's plan move (see the note above). The same guard passes at `f5a40b27`. |
| Remaining `./dev check` steps run one by one: four Python unittest suites, `cargo fmt --check`, workspace clippy `-D warnings`, `v1-game` draw-probe clippy, `cargo test --workspace`, and the draw-probe `cooked_assets_gpu` test | pass. Every suite reports 0 failed in `cargo test --workspace`, and the draw-probe GPU test passes 13 tests. |

Edge conditions for chrome-webgpu: I ran low sun in the scene check. Storm, distance and fast camera have no
controls in the starter, so they don't apply. Full refresh and cadence are outside this milestone, which covers
function and legibility only.

## Questions (`ask`)

- **M1.1.** Does the scene case run the documented command, drive real hardware Chrome, and fail on the listed
  fakes? Yes. It spawns `./dev starter --web`, reads `SCENE_URL`, and launches the installed Chrome. It now wraps
  `GPU.prototype.requestAdapter` before the page loads. It records the adapter the page actually received, and
  fails unless that adapter is non-fallback Apple `metal-3` (`check_starter_browser.mjs:119-150`). This closes
  round 1's S3. An undrawn cooked generation or a fallback scene fails it, because the case edits the mesh and the
  texture and asserts the submitted pixels change. A slider that changes state but not pixels fails the
  1,000-pixel threshold on submitted images.
- **M1.2.** Is anything declared separately for the web? No, and this round didn't change it. `thrum-controls.js`
  builds every widget from `describe_json`. `edit_control` uses `lookup`, `edit_display` and `edit_value`, the Mac
  panel's setters. `?params` goes through `Controls::apply_file`. The new `title()` reads the scene's
  `Config.window_title`, so the page title is no longer a web-only string.
- **M1.4.** Is there any `cfg`, web-only item or engine game type in the scene's source? No.
  `examples/starter/src/main.rs` is one file with 40 non-blank, non-comment lines. It names only the kit's facade
  (`sky::run`, `Config`, `SkyGame`, `Controls`, `Camera`, `MeshPlacement`, `CookedMaterial`, `Display`). Mac and
  web build the same file, so the game-line count is the same. The doc comment now describes both targets (round
  1's S5). The kit's web runner (`sky/browser.rs`, `web/scene.js`, `web/index.html`) holds all the bindings. The
  kit, not the scene, now holds the native-platform split (F2).

## M1.5 inspection

I wrote this list before looking. For each state, is the scene or the message readable, and does every message name
its object, cause and next step?

| State | Capture | Judgment |
|---|---|---|
| Loading | [journey-loading.png](M1-review/journey-loading.png) (my capture, identical to round 1's) | Readable. It says what is loading, to keep the page open, and what to do if loading fails. |
| Drawn | [r2-a-drawn.png](M1-review/r2-a-drawn.png) (DOM), [scene-drawn-canvas.png](M1-review/scene-drawn-canvas.png) (submitted output, identical) | Lit ground, sky and the cooked rock, under `Cooked generation 1 drawn.` Groups and values are readable. |
| Missing store | [r2-journey-missing-store.png](M1-review/r2-journey-missing-store.png) | `Scene could not start: asset store http://127.0.0.1:1: fetch request did not complete; check that the scene command is still serving its cooked asset store, fix the reported cause and rerun it.` Object, cause and one next step. Round 1's duplicate (S2) is gone. |
| Misspelt `?params` key | [journey-typo.png](M1-review/journey-typo.png) (identical) | Names the file and the key, and suggests the right key. The scene keeps drawing. |
| After a tab switch | [journey-tab-return.png](M1-review/journey-tab-return.png) (identical) | Real hidden and visible transitions (`tabVisibility [true,false]`). The submitted pixels after the return equal the drawn image. |
| 480 px wide | [journey-narrow.png](M1-review/journey-narrow.png) (identical), [r2-e-refused-480.png](M1-review/r2-e-refused-480.png) | Canvas, status and controls fit with no horizontal overflow, including the long refusal message. |

## Code read

I read every changed line of `8244a46e`: `crates/thrum-app/Cargo.toml` (shared dependencies made unconditional,
with the `metal` and `webgpu` features still per target), `lib.rs`, `sky.rs` (the gating of Mac- and web-only
fields and functions, and the native `run` fallback), `sky/browser.rs` (refusal lifetime, transport error,
`title`), `web/scene.js`, `examples/starter/src/main.rs`, `scripts/check_starter_browser.mjs`,
`tools/xtask/src/main.rs`, `CONTRIBUTING.md`, the maturity row, and the plan and log edits. I also read `f5a40b27`,
the loop's claim, and the CREATOR part of `42fdce97`, the plan move.

I found no hard-coded results, canned data, test-only branches, weakened or deleted tests, or silenced warnings.
The new `#[expect(clippy::print_stderr)]` on the native `run` gives its reason, like the kit's other reporters.
The maturity row claims an iOS-simulator compile only, and says Linux and Windows host execution was not run,
which matches what I found.
