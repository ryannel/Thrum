# CULLING M3 independent review

Review target: `21aa3fb764d1931b88ba2d08f919cd290d7adc82`; builder ends at `0a20e02aa3ca`.

**FAIL:** M3.3 fails because disabling stars changes the measured valley default to Direct. A second Must Fix outside the items is the legacy benchmark regression described below. The other nine items pass. No item is Could Not Judge.

## Inspection list (written before viewing)

For M3.7 and M3.8: inspect whole submitted valley frames, then native-size left/right edges and ridge silhouettes through the fast yaw, entering cover and leaving cover. Look for missing geometry, transient holes, culling pops and flicker across consecutive frames. Compare fixed route checkpoints to identify persistent scene content. Inspect mixed-mesh field per-tick frames through cover and back out separately as fixed-time evidence. Assess elapsed-time cadence from capture timestamps, without treating engine submission as panel refresh. Exercise distance/fast camera and available low-sun/storm route conditions; explicitly report uncovered conditions. No project image anchors are registered; use the written Horizon Forbidden West bar of invisible culling.

Inspection outcomes and limits are recorded below.

## Basis and scope

This is an independent code and product review of the checked commit above. The review did not change implementation, tests, references or planner fields. I read the CULLING design, contracts, milestone changes (including the shader sampling-order repair and cloud test adapter), and the module's visibility, pyramid, history, bucket/arena, caster, capability selection and reference/test implementations. I traced the game and browser callers through preparation and scene attachment. No reading or judgement was delegated.

The supplied clean-checkout results for `./dev check` and M3.1, M3.2, M3.4, M3.5, M3.6 and M3.10 are accepted as instructed, after inspecting what their commands actually prove. Those results concern the files qualified by the script at `f2418fb348cd`; the review target is `21aa3fb764d1`. Historical builder captures and cost rows retain their own revision identities and are not represented as newly run checks.

## Per-item judgement

| Item | Verdict | Basis |
| --- | --- | --- |
| M3.1 | pass | The native moving-route oracle prepares separate Direct, Frustum and Occlusion hosts, asserts the selected default and resolved modes, compares all 361 submitted ticks on both native recipes, and preserves route, lighting, exposure and history identity. The default is Frustum. The supplied check passed. |
| M3.2 | pass | The browser driver exercises default and explicit modes against Direct at all five moving checkpoints, verifies submitted RGBA size and SHA and compares pixels, rather than accepting only a DOM label or screenshot. The supplied check passed. |
| M3.3 | fail | The retained paired cost rows support Frustum, and both targets report their resolved modes. However, the shipped browser page implicitly switches the same valley to Direct when stars are disabled. The live reproduction below confirms the violation of the default-selection rule. |
| M3.4 | pass | The same native route comparison selects meshes on all hosts, preserves bucket order for Direct and checks Frustum/Occlusion with Sun-caster culling on both recipes. The assertion covers every submitted route tick, not just counts. The supplied check passed. |
| M3.5 | pass | Chrome's mesh-selected route compares both culled modes to its own bucket-ordered Direct at the five checkpoints, with resolved-mode and submitted-image checks. The supplied check passed. |
| M3.6 | pass | `culling_temporal` prepares genuinely selected temporal pipelines, drives all 32 jitter phases, advances each host's history after submission, and checks exact RGBA equality for both culled modes against Direct. Its mode assertions prevent a fallback from passing as culling. The supplied check passed. |
| M3.7 | pass | Fresh 1920×1080 native fixed checkpoints and realtime fast-yaw output show no culling pop, hole or silhouette dropout in whole-frame and consecutive native-size inspection. The retained mixed-field enter/exit sequences also pass. See the inspection scope and timing limits below. |
| M3.8 | pass | Fresh 1280×720 hardware Chrome WebGPU fixed checkpoints and realtime fast-yaw output preserve geometry at both edges and the ridge. Whole-route 10 fps strips and consecutive native-size regions show no culling discontinuity. See the inspection scope and timing limits below. |
| M3.9 | pass | All G3c clauses have implementation and scoped evidence, as audited below. The ledger distinguishes implementation, correctness, inspected content and target measurements. |
| M3.10 | pass | The exact-baseline verifier covers the registered native and Chrome references and checks their retained identities without a newly relaxed image threshold. The supplied check passed. |

## Default decision and G3c audit

I independently recomputed the four cost summaries from the retained compressed raw rows in [qualification-396132a1](M3/qualification-396132a1/cost.json), using the summarizer's pairing and warmup rules. All summaries matched. Each run contains 1,085 rows, with 361 measured triplets after warmup; the cover/open split is 53/308. The nonempty Occlusion prepass is asserted. The rows record host load and are labelled contended, rather than claimed as isolated hardware performance.

Median Occlusion minus Frustum whole-GPU milliseconds are:

| Recipe/run | Cover | Open |
| --- | ---: | ---: |
| Portable 1 | +0.052754 | -0.0729995 |
| Portable 2 | +0.109583 | -0.020583 |
| Apple tile 1 | +0.064170 | -0.1614775 |
| Apple tile 2 | -0.087210 | +0.0205605 |

Occlusion does not win both regimes in both runs. Keeping Frustum is the design's required decision. Explicit Direct remains a legitimate oracle request; the finding concerns an implicit switch caused by an unrelated visual control.

For M3.9, `visibility.wgsl` performs compute frustum classification and compaction, and `visibility/occlusion.wgsl` tests conservatively against the reverse-Z min pyramid. History is keyed by logical slot, with distinct previous/final lists and lifecycle invalidation; bucket members do not become history identities. Compute writes the indexed indirect arguments, including instance counts. `mesh_route.rs` consumes one indexed indirect draw per nonempty mesh/pipeline bucket, with matching material/raster class and depth twin. `mesh_arena.rs` owns immutable vertex/index offsets; camera and caster compaction change instance membership without repacking meshes. Caster visibility uses light volumes independently of the camera list.

The existing material binding now admits compute access. The production-buffer compute test reads its material values through a real compute dispatch; this is not a duplicate table or a permission-only assertion. Mixed-bucket, lifecycle and jitter tests provide visibility/image equivalence, while the moving-route proofs connect it to the game. [The G3c cost record](../render/G3C-CAMERA-CULLING-2026-11-07.md), M2 paired mesh-field records and M3 paired valley records cover full and partial visibility as diagnostic tripwires. No timing threshold is silently substituted for correctness.

The G3c row in [the maturity ledger](../../design/maturity.json) records implementation separately from correctness, inspected visuals, device measurements and scope limits. Simulator function is not physical-phone performance. Submitted engine output is not panel cadence. The unresolved attribution of earlier sporadic native differences is disclosed in [the investigation](M3-investigation.md); the strict current checks and unchanged retained references, rather than a tolerance, establish this review's current correctness result. I found no weakened comparison or fabricated GPU result in this path.

## Inspection observations

Retained mixed-field fixed readbacks: inspected [whole tick 301](M3/field-fixed/whole-301.png), [whole tick 426](M3/field-fixed/whole-426.png), and the native-size [enter](M3/field-fixed/enter-1x.png) / [exit](M3/field-fixed/exit-1x.png) strips, ticks 301–306 and 421–426. Props, wall edges and their emerging silhouettes stay continuous across those consecutive frames. No isolated missing prop or hole was visible. These are 640×360 fixed-time readbacks from the builder's recorded revision, not elapsed-time field motion. Their correspondence record retains source hashes and Frustum/Occlusion equality. I sampled stills; I did not watch a video.

### Fresh valley captures — M3.7 and M3.8

Built the native game/cooker and browser WASM from the review checkout and ran the checked-in routes through T2 submitted-image capture under the shared GPU lock. Native fixed checkpoints use the supported offscreen moving-route driver (`THRUM_CAPTURE_SCENE`/`THRUM_CAPTURE_ROUTE`); the general runtime driver only accepts realtime for a moving route. That initial refusal was a reviewer command mismatch, not a failed product proof. The capture recipe and completed run are retained as [driver](M3-review/capture_review.py) and [log](M3-review/capture-final.log).

Inspected all five fixed checkpoints on each target: approach, mirror-light, pre-cut, post-cut and near-plane. The [native overview](M3-review/native-fixed-route/overview.png) is alphabetic by checkpoint name; the [Chrome overview](M3-review/chrome-fixed/overview.png) is in tick order. Both are downscaled whole-frame overviews; individual full-size checkpoint PNGs and capture identity records are retained alongside them. The near-plane view legitimately fills the frame with the approaching blue cube; it is not a missing background or a dropped foreground face.

For elapsed-time output, I inspected all eighty 10 fps overview samples across each approximately eight-second capture: [native first half](M3-review/native-realtime/motion-01.png), [native second half](M3-review/native-realtime/motion-02.png), [Chrome first half](M3-review/chrome-realtime/motion-01.png), [Chrome second half](M3-review/chrome-realtime/motion-02.png). These whole-frame strips include the initial hold, departure, rapid reversal, return, authored cut, approach to cover and final hold. I also inspected native whole frames near ticks 285, 375 and 420, Chrome's whole frame near 420, and six consecutive submitted frames at each of those three windows on each target. The latter strips preserve original pixel scale: left edge, ridge/silhouette region, right edge, in columns; successive frames run top to bottom. Exact source IDs, ticks, crop coordinates and hashes are in [the selection record](M3-review/selection.json).

| Inspection cue | Mac | Chrome |
| --- | --- | --- |
| Fast yaw, including objects entering/leaving frame | Pass: the sphere enters the left crop continuously; the ridge and cast shadows traverse the frame without a hole. | Pass: sphere, props and rock cross the edge continuously through the six-frame yaw strip. |
| Ridge and occluder silhouettes | Pass: silhouettes remain filled; no detached or disappearing patch in pre-cut or post-cut regions. | Pass: the ridge, rock and sphere boundaries remain continuous. |
| Entering/leaving cover and near plane | Pass: moving props remain present; the large foreground cube progressively covers the view. Retained field strips show normal emergence around walls. | Pass: foreground coverage grows continuously, without a lost face or sudden background reveal. |
| Cut and holds | Pass: the one large view change occurs at the route's authored cut; held endpoints stay intact. | Pass: same authored transition and stable endpoints. |

Native-size strips: [Mac yaw](M3-review/native-realtime/crops-285-1x.png), [Mac pre-cut](M3-review/native-realtime/crops-375-1x.png), [Mac post-cut](M3-review/native-realtime/crops-420-1x.png); [Chrome yaw](M3-review/chrome-realtime/crops-285-1x.png), [Chrome pre-cut](M3-review/chrome-realtime/crops-375-1x.png), [Chrome post-cut](M3-review/chrome-realtime/crops-420-1x.png). Existing stair-stepped terrain shadows are visible, but I saw no culling-induced flicker or disappearance. The exact-oracle results separately establish equivalence; the visual judgement is not inferred from those results alone.

I sampled stills and strips, rather than watching video playback. The original-time [native preview](M3-review/native-realtime/preview-1x.mp4) and [Chrome preview](M3-review/chrome-realtime/preview-1x.mp4) are retained. Lossless packaging verified 964 native realtime images, 482 Chrome realtime images and 481 Chrome fixed images; the native fixed driver retained its five submitted checkpoints. The realtime captures report no native ring/writer omissions and no Chrome renderer omissions. Native submission gaps were median 8.293 ms, p95 11.264 ms, maximum 12.516 ms; Chrome gaps were 16.9/20.2/22.8 ms. These are capture-ON engine observations, not capture-OFF performance measurements or physical display swaps. Fixed capture's wall time is not realtime evidence.

Edge coverage: low sun (route starts at 17.3 solar hours), distant ridge, fast yaw/reversal, a camera cut and close/near-plane geometry on both targets; mixed field through cover via the specifically requested retained readbacks. The route and available controls do not expose a storm scenario, so storm was not exercised. No attended physical-panel cadence qualification was performed or inferred. The visual verdicts judge the named T2 culling-continuity proofs against the invisible-culling bar; they do not certify panel refresh or general storm rendering.

## Findings

### M3.3 — star visibility changes the valley's default culling mode

**Must Fix; verified in source and the running Chrome product.** M3.3's default-selection statement does not hold for the documented `?assets=…&stars=off` valley configuration. `examples/v1-game/web/index.html:63` passes `cooked && stars !== "off"` as `valley_stars`. `examples/v1-game/src/browser.rs:130` passes that same value to `valley_culling` as its scene discriminator. At lines 344–349, a false discriminator selects Direct for an omitted/default culling request. `attach_cooked_valley` later installs the valley contribution and camera but does not change the prepared culling mode. Thus disabling the star catalogue also disables the measured Frustum default for the same 10,000-entity valley, without the required scene measurements or an explicit culling request.

Keep scene/default selection independent of star visibility, preserve explicit mode requests, and cover the shipped page with stars on and off. The existing route check does not catch this because `create_route_game` always passes `true` as the valley discriminator. The native scene-selection path is independent of stars. The [live reproduction](M3-review/stars-default.json), run from this checkout on hardware Chrome WebGPU, records the shipped page after 600 samples for each case: ordinary valley → `Frustum / Supported`; `stars=off` → `Direct / RequestedDirect`; `stars=off&culling=frustum` → `Frustum / Supported`. The third case rules out an unavailable indirect capability as the reason. The [reproduction driver](M3-review/stars-default.mjs) does not patch the page or its game exports. Device replacement and other star/culling combinations were not exercised in this reproduction.

### Outside the items — legacy culling cost benchmarks silently time Direct twice

**Must Fix; verified by source trace and the current GPU benchmark run.** The final-module audit found a regression in the existing performance tripwires. `crates/thrum-render/benches/camera_culling.rs:171` prepares both labelled arms using `PreparedSynthetic::prepare`; `crates/thrum-render/benches/camera_culling_encode.rs:43` uses the same route. That API calls `DeviceCapabilities::capture_device`, which records no adapter-read downlevel facts. Since the M2 capability-selection change (`da88e3d5`), `CullingSelection::resolve` turns the supposed Frustum arm into Direct with `UnknownCapabilities`. Neither benchmark asserts its resolved mode. The GPU benchmark even substitutes zero for a missing camera-visibility pass, so it can successfully print a misleading Direct→Frustum comparison; the allocation benchmark similarly measures Direct in both labelled arms.

Use the already available adapter-generation capability record for preparation and assert the resolved mode in both benchmarks before measuring. Do not remove the conservative fallback from production. The newer field and valley paired benchmarks already use the capability-aware path and mode assertions; their recorded default decision remains valid. Historical pre-M2 cost rows are not retroactively declared false, but rerunning the original tripwire on this final module no longer measures what it says.

Proposed item that would catch this: “The full/partial-visibility GPU and encode/allocation tripwires assert and report the actual resolved mode; on a supported adapter their Direct and Frustum arms execute those distinct modes.” Keep timings diagnostic, with no numeric performance gate. The [fresh benchmark log](M3-review/camera-culling-bench.log) corroborates the trace: `cargo bench --locked --offline -p thrum-render --bench camera_culling` succeeds on the M3 Max, labels B as Frustum, and prints `visibility ms 0.000 (0.000)` for all six full/half/5% rows at 10k/100k. The missing-pass-to-zero substitution is in its `sample` helper; zero is not evidence of a free cull. The encode benchmark was source-audited, not executed in this review.

## Closeout

`./dev check-docs` passed (828 Markdown files, 5,847 local links, 30 module records). All selected evidence hashes were verified after copying. Approximately 7.1 GB of this review's temporary capture/store/profile output was removed after the owned jobs exited; selected images, original-time previews, compressed capture records, the reproduction and benchmark logs remain in `M3-review/`. Existing builder evidence and other workers' scratch/build trees were left intact.

Only this review, its evidence folder and M3's permitted verdict/review/status/findings fields were changed. The changes are uncommitted for the review script.

FAIL M3: 1 fail, 0 could_not_judge, 1 outside the items
