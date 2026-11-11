# APPLE-TILE M3 independent review

Reviewed commit `e2129272a95335ae94aeb680668dc553c60b61d4` in the loop's checkout on the M3 Max. No implementation changes, commits, pushes or delegated agents. Four items pass. D3.5 could not be judged because the active Mac console is locked; M3 returns to `in_progress`. No implementation defect or Must Fix outside the items was found.

## Inspection plan

[The inspection list](M3-review/inspection-plan.md) was written before inspecting images. It covers native and Chrome fixed linear/SDR controls, Sun/stars, terrain silhouettes, local lights, mirrors, shadows, cuts, exposure identity, both surface formats, letterboxing, realtime transitions, and real-window input/resize/replacement. Full frames and native-resolution details are separate from reduced 10 fps motion strips. Strips are sampled elapsed-time evidence, not watched playback or panel cadence.

## Findings

### MF1 — Attended Mac window verification unavailable

**Must Fix / verified coverage gap / D3.5 (`cant_tell`).** The review's attributed read-only preflight outside the sandbox found an existing Screen Recording grant but a locked console: [device-access preflight](M3-review/preflight-device-access.json). The earlier sandbox preflight returned false for the grant: [sandbox preflight](M3-review/preflight.json). The decisive unavailable prerequisite is the unlocked active session. Consequently this reviewer could not capture its own actual game window or exercise and observe ordinary input, resize and replacement under both recipes. Engine output cannot establish compositor presentation. The builder's earlier granted-session window evidence remains historical evidence; this does not establish a rendering defect in it.

To settle this item, rerun the specified both-recipe window protocol on the reviewed build in an unlocked active session with an existing Screen Recording grant for the observer. Record tool/version, PID/window/bounds/backing scale and clock, matched paused T2 references, moving input, resize/letterbox and replacement recovery. Do not change grants by automation. This does not require Mac M2 cadence or T2 simulator M2/COOKED-DRAW and makes no panel claim.

## Code and boundary inspection

Read the complete implementation diff in `cc578b2b` (all 28 changed code/test files), the M3 plan/ledger changes in `e2129272`, the owning design and C04/C06/C07/C08/C12, and producer/consumer context. Reviewed retained evidence summaries and audited their hashes; did not treat builder-written logs as independent execution.

The stage receives explicit colour/motion/exposure graph IDs and extent. Graph closure retains reads; tone binding checks all three reads, pipeline key and generation, and validates the output grid. Preparation builds both exposure-side bind groups. Forward, exposure resolve, history and tone continue selecting the same history write side; submission advances that owner. Resources are retained through bind groups/slots. The internal seam admits a separate prepared reconstruction texture without adding reconstruction or EDR implementation.

`synthetic_display.wgsl` and `present.wgsl` are unchanged: tone still loads source pixels, applies source-coordinate dither, stores through Rgba8Unorm quantization, then uses nearest-source presentation with the retained sRGB decode branch. No fused tone, new drawable capture, queue owner or traffic-saving claim was introduced.

`FrameSample` now carries `SceneGpuTimings`, preserving actual pass names and durations. Optional absent passes have no key; unsupported/faulted batches remain absent; executed copy/query work without counters is null. The scene sum excludes surface presentation to avoid double counting the separately exposed present duration. Native titles/benchmarks/probes, browser summaries and capture serializers use names. The iOS host only consumes completion/presentation counts and image identity, with no positional timing interpretation. CPU durations, GPU timestamp deltas and callback wall time remain distinct.

## Evidence and checks

- [Release build](M3-review/build.log), [review provenance](M3-review/review-provenance.json).
- [Renderer tests](M3-review/render-tests.log): 242 passed, including exact identity rebinding, the copy-produced reconstruction texture, foreign graph rejection and optional timing layouts.
- [Builder artifact audit](M3-review/builder-artifact-audit.json): all 279 retained hashes match; the only differences among 564 pinned source files are the two documented verification scripts. This corroborates provenance, not fresh target behavior.
- [Independent command results](M3-review/checks.json).

## Native inspection results

The [native matrix](M3-review/native-m1/manifest.json) passes all 38 retained M0 scene-linear/SDR comparisons, including portable A/A repetition and both recipes' Sun/night controls. The [surface matrix](M3-review/native-m2/manifest.json) passes all four retained Bgra8Unorm/sRGB comparisons at 1000×800. These are full-byte comparisons with unchanged reference data and matching deterministic sample/exposure identities, not two changed implementations compared only to each other. Actual surface-graph binding re-prepares the same physical colour/motion/exposure inputs; the dedicated identity and distinct-copy Metal tests also pass.

Inspection results against the prewritten list:

- **Sun and sky:** disk, horizon and terrain remain intact in the [fresh Sun frame](M3-review/native-m1/apple-tile-sun-sdr/checkpoint-0.png) and [native-pixel crop](M3-review/selected/apple-tile-sun-detail.png). No new tone clipping or displaced source coordinates were observed.
- **Stars and dark geometry:** [portable night](M3-review/native-m1/portable-night-sdr/checkpoint-0.png) and [Apple night](M3-review/native-m1/apple-tile-night-sdr/checkpoint-0.png) retain the star field, hill silhouette, mirror and rock. Dark surfaces remain dark as in the oracle.
- **Local lights, mirrors and shadows:** [mirror/light checkpoint](M3-review/selected/portable-mirror-light.png) and [post-cut checkpoint](M3-review/selected/portable-post-cut.png) agree with the retained M0 full frames viewed alongside them. No fixture or shadow caster disappeared. Existing coarse terrain/shadow appearance is inherited; this milestone preserves the scene and does not claim to finish its artistic polish, as the owning design specifies.
- **Both surface formats and non-1:1 mapping:** [portable Unorm](M3-review/selected/portable-bgra8-letterbox.png), [portable sRGB](M3-review/selected/portable-bgra8-srgb-letterbox.png), [Apple Unorm](M3-review/selected/apple-tile-bgra8-letterbox.png), and [Apple sRGB](M3-review/selected/apple-tile-bgra8-srgb-letterbox.png) preserve centered letterboxing, silhouettes and content proportions. Exact comparisons establish the source-coordinate/store-quantization semantics beyond visual inspection.
- **Cuts and changing distance:** inspected the complete eight-second portable and Apple route sequences as 10 fps strips, including the approach, camera cut and near plane. Objects move coherently through the cut; no stale old-view image or missing scene content was observed. [Portable first half](M3-review/strips/native-m1-portable-route-motion-01.png), [second half](M3-review/strips/native-m1-portable-route-motion-02.png), [Apple first half](M3-review/strips/native-m1-apple-tile-route-motion-01.png), [second half](M3-review/strips/native-m1-apple-tile-route-motion-02.png).
- **Day/night transitions and skipped work:** inspected both four-second day/night sequences and both render-stall sequences at 10 fps. Day/night switches retain geometry and introduce/remove the stars as intended. The injected approximately 250 ms render-owner pause is visible as held content, followed by an intact resumed pan. [Portable variants](M3-review/strips/native-m1-portable-variants-motion-01.png), [Apple variants](M3-review/strips/native-m1-apple-tile-variants-motion-01.png), [portable stall](M3-review/strips/native-m2-portable-render-stall-motion-01.png), [Apple stall](M3-review/strips/native-m2-apple-tile-render-stall-motion-01.png).

[Independent runtime summary](M3-review/independent-summary.json) records sample counts, history epoch changes, injected pauses and named counters. Both variants runs have 240 star-present samples and separately absent star passes. All reported samples in those runs carry named maps, including null upload/readback/query entries. Every source frame was verified by the capture packager before bulk reduction. [Strip index](M3-review/strip-index.json) links the original-time movies and sheets; I sampled sequences, not real-time playback, and do not claim panel cadence.

Profile edges exercised are low sun, night, distance/near-plane, fast camera/cut, changing variants and render stall. The native offscreen driver requests 120 Hz; this is not a full-refresh physical-display measurement. A storm fixture is not part of the retained APPLE-TILE oracle and was not exercised. Physical full-refresh and actual-window observations remain unavailable in the locked session; D3.5 stays open rather than being inferred from these captures.

The [iOS simulator app build](M3-review/ios-build.log) passes using its repository script. The earlier direct [iOS compile attempt](M3-review/ios-compile.log) used the ambient compiler and could not find the simulator standard library; the script explicitly selects the pinned compiler and succeeds. This is compilation evidence, not a new simulator lifecycle or device-speed qualification.

## Chrome inspection results

The required `python3 scripts/run_v1_showcase.py moving` passes: [command log](M3-review/moving.log), [result](M3-review/moving-run.json). The complete `./dev check` also passes, including the cooked-assets GPU suite: [check log](M3-review/dev-check.log). These checks supplement the target observations.

Fresh Chrome fixed day/night captures each pass 481 exact records for both scene-linear and submitted SDR, totaling 1,924 exact deterministic records against the retained pre-change Chrome controls. Comparison checks include full decoded image hashes, sample identity and linear exposure words. [Comparison summary](M3-review/chrome-comparisons.json), [execution log](M3-review/chrome.log). Browser pixels are compared to their own baseline, not Metal's.

Against the inspection list, full-resolution day and night checkpoints retain the terrain silhouette, mirror, cubes, rock, shadows, stars and local light. [Day mirror/light](M3-review/chrome-day-sdr/selected/mirror-light.png), [night mirror/light](M3-review/chrome-night-sdr/selected/mirror-light.png). The night local-light disk and lit foreground remain strongly saturated, exactly as in the retained Chrome oracle; this is inherited appearance, not a new tone-seam regression. The complete realtime day/night sequences were inspected as 10 fps elapsed-time strips: [day first half](M3-review/strips/chrome-day-01.png), [day second half](M3-review/strips/chrome-day-02.png), [night first half](M3-review/strips/chrome-night-01.png), [night second half](M3-review/strips/chrome-night-02.png). The approach, camera cut and near-plane progression preserve the fixtures with no stale view after the cut. Original-time [day movie](M3-review/chrome-day-realtime/preview-1x.mp4) and [night movie](M3-review/chrome-night-realtime/preview-1x.mp4) are retained. No slowed playback was used to infer smoothness or panel cadence.

Each realtime run records 482 submitted images. The standard resize run records 481, with presenter generation 1→2 at 961×541; a second observer run adds an external 1000×800 canvas resize and reaches generation 3 while retaining a 1280×720 scene. [Runtime summary](M3-review/chrome-runtime-summary.json). I inspected the actual Chrome composed [1000×800 canvas screenshot](M3-review/chrome-canvas/presented-canvas.png) alongside its [last engine source](M3-review/chrome-canvas/selected/last.png): content proportions and centered top/bottom bars are intact. [Observer identity, bounds, backing scale and clock](M3-review/chrome-canvas/canvas-observation.json). This is a CDP observation of headless Chrome composition, not an OS-window observation. The separate native exact matrix exercises both Unorm/sRGB format branches; Chrome uses its selected supported surface path. Browser timing serialization preserves named maps and nullable unsupported counters without positional relabeling.

## Verdicts

| Item | Verdict | Basis |
| --- | --- | --- |
| D3.1 | pass | Graph/input ownership traced; prepared identity and copy-input tests; unchanged shaders and SDR intermediate. |
| D3.2 | pass | 38 native exact comparisons, four surface comparisons, both recipes' full-frame and realtime inspection. |
| D3.3 | pass | Optional timing tests, named runtime maps, all producer/consumer boundaries and iOS build checked. |
| D3.4 | pass | Required moving command, 1,924 exact Chrome records, day/night motion and composed resize inspection. |
| D3.5 | cant_tell | Locked active console prevents the required independent both-recipe Mac-window observations (MF1). |

Final record validation: `./dev check-docs` passes ([log](M3-review/check-docs.log)); all review links resolve, `git diff --check` passes, and a structural comparison confirms only the permitted M3 verdict/review/state/findings fields changed. Counts and builder status/evidence are untouched.

## Evidence retention

All source records were verified before reducing this review's bulk outputs. Complete identities, hashes, manifests and logs remain, together with selected lossless full-resolution images, selected scene-linear sources, elapsed-time movies and inspection strips. [Native spool selection](M3-review/bulk-selection.json), [readback selection](M3-review/readback-selection.json), and per-browser-run `selection-retention.json` record the reductions. [Reproducible browser build/cache inventory](M3-review/browser-build-retention.json) records removed local launchers, copied sources, cooked stores and caches. The reviewed repository inputs remain unchanged; no remote backup is claimed. Review-only scripts in this folder record the commands and observation procedure. The initial canvas observer invocation had a misplaced shebang and did not launch; the corrected observer completed, as its retained log and capture show.

FAIL M3: 0 fail, 1 cant_tell, 0 outside the items
