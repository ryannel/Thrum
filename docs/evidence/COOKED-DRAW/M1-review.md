# COOKED-DRAW M1 review

Reviewed commit `50180fa0cdbda7b8ab2ec7ea09b2d34d96059654` on 2026-11-09 from the loop's clean checkout. All twelve items pass, with two advisory findings and the sampling limits recorded below.

## Inspection lists written before viewing captures

For M1.3, inspect all five views, valley noon at 0, 18 and 36 degrees, low sun, and diagnostic. Compare production I and D whole frames, full-resolution edge/horizon/object crops and unthresholded differences. Examine terrain triangle edges, entering terrain and silhouettes for missing geometry, cracks, holes or seams. Check material colour and texture, Sun illumination and shadows, metallic sphere and matte rock, mirrored diagnostic twin and edge-straddling mesh, and volume. Classify every residual using all-meshlet and full-range reference controls. Check exact D/R, independent command arguments, same-arm repeats, missing-mesh negative control, unchanged reference and restored D.

For M1.10, inspect simulator start, 36-degree end, low sun, diagnostic, landscape and restored portrait whole images and native crops of edges, horizon and objects. Check complete textured terrain, correct colour channels, metallic sphere/sky reflection, matte outward-facing rock, mirrored twin orientation and lighting, Sun/ambient direction against the corresponding Mac content, cast shadows, visible unclipped volume, correct aspect/fill after rotation, and successive held-view stability.

For M1.11, inspect every sampled pan frame including both holds and both entering frame edges. Check complete terrain and stable objects/materials without popping, flicker or holes within samples. Record sample times, presented-frame brackets, largest gap and coverage limits. Inspect sampled stills, without inferring unsampled motion or cadence.

Target edge coverage will include low sun, distance/horizon, moving camera, lifecycle and rotation where applicable. Full-refresh performance, storm weather and fast camera motion are outside these fixed functional/parity claims and receive no inferred qualification.

## Scope and execution

The inspected change is `dbf79d6b1d6eabf628f49d5aac89bba7fcb322cb..50180fa0cdbda7b8ab2ec7ea09b2d34d96059654`. BASE is the parent of the first implementation commit, verified by the harness. I reviewed the complete source diff, tests, probe/fixture/launcher scripts, configuration and documentation changes, including the adjacent console-policy documents. I traced the capability producer, surface seam, renderer contribution, native/browser/iOS consumers and generation replacement. The child module's full implementation and its callers were inspected. The production WGSL files are unchanged. Existing assertions were preserved while the GPU cases were extended to both paths. Diagnostic controls are compile-time opt-in and ordinary attachment supplies no probe.

Fresh local results live under `tmp/cooked-draw/review-50180fa0/`. `results.json` records exact command arguments, exits and durations. Each target command uses `agent_loop.py lock gpu`. This is reviewer-owned scratch with an ownership/expiry record. The loop's pre-existing scratch is untouched. These local captures corroborate the committed builder evidence, which was read after deriving the inspection list and reviewing the mechanisms.

The initial sandbox check could not enumerate Metal and failed two adapter-creation tests. The approved native run completed with 666 Rust tests in 62 result groups, zero failures, plus repository checks, formatting, Clippy and Wasm checks. The final native run under the shared GPU lock also exited 0 with 666 Rust tests in 62 groups and zero failures. Its retained `check.log` has SHA-256 `a1967e6e0949d0efe58271c5c17d096b6d4951899c86a245b16c2ff6485058b6`. Build output includes existing linker platform warnings for the ISPC objects.

## Findings

1. **Advisory, verified. Existing Sun shadow aliasing remains visible.** The fresh `paths/low-sun/I/output.png` shows horizontal stair-stepping on distant terrain shadow edges. The same feature is present in D and R, and the fresh `indirect-mac/low-sun/base/output.png` is byte-identical to the current I capture. The diagnostic scene also retains its existing hard volume-box boundary. Geometry remains complete and the fallback preserves these shadow/volume appearances. This is inherited presentation debt in the shared shadow/diagnostic-volume consumers. The compatibility result here qualifies preserved content, materials, illumination and shadow membership. A later shadow/volume quality milestone should inspect these specific regions against its own appearance bar.
2. **Advisory, verified. A few prose summaries lag the refreshed evidence.** `docs/design/decisions/G5-CONTENT-ADDRESSED-LOOP.md:232` still reports the earlier 1.118-second maximum sampling gap, while the refreshed committed visual report and evidence README give 1.125629 seconds. `docs/design/modules/COOKED-DRAW.md:344` still says the revised oracle awaits qualification, and the plan's review line names the earlier design revision. The primary reports, maturity entry and current claim identify the refreshed proof and retain its sampled-only limits. Refresh those historical summary values/phrases during the next authorized documentation update. These discrepancies do not change the recorded coverage or infer an extra capability.

No blocking code, contract, data-loss, security or fabricated-work finding was found in the reviewed diff. The selection has no platform/name/backend special case, and the real target runs exercise computed output.

## Item results and boundary answers

### M1.1 — pass

`DeviceCapabilities::capture` reads `adapter.get_downlevel_capabilities().flags` at `crates/thrum-render/src/capabilities.rs:39`. `capture_device` supplies `None` at line 56. Its untracked provenance never grants a downlevel fact. `SceneSurface` lends that optional field, and `CookedDrawPath::select` requires known `INDIRECT_EXECUTION`. Loader, pipelines, readback and stats derive from that surface at rebuild. Mac device creation, browser creation/recovery and iOS creation/recovery all use adapter-backed capture. The only production-record mask API removes support, and its callers are explicit tests or the qualification binary.

Both ordinary and draw-probe GPU suites passed. The public-host test constructs `FrameHost::new`, and the feature-enabled child uses the real `attach_cooked_scene` route. It observes unknown facts, Direct, zero INDIRECT buffers/cull/count/readback resources and full-range indexed commands. Unit cases cover supported, missing and unknown facts. Device replacement tests cover retained selection and both changes of selection on a new generation. Fresh I/D reports separately record original facts, mask and selected path.

### M1.2 — pass

`crates/thrum-asset-gpu/src/draw.rs:689` binds the common pipeline/material/vertex/instance/index resources before selecting the command. Direct issues `draw_indexed(0..mesh.index_count, 0, 0..1)`. Direct pipelines omit cull pipeline/layout, bound cull uniform, arguments and visible counter. Loading omits GPU range/bounds buffers and their uploads. `Live.readback` is absent. The graph's imported tokens describe ordering and allocate no physical INDIRECT buffer. `computes()` suppresses the contribution dispatch. Fresh production D witnesses have resource observation `[0, false]` and one core indexed command per placement.

History writes moved into common upload recording before either draw path. They derive from committed continuation/reset/material state, and discarded recordings are corrected by the next recording. The source audit preserves the indirect arm's writes and values. Both-path cooked/material suites cover capture without history, discarded uploads, resets, replacement, remote fetched generations and edits, deferred/failed/promoted material hooks and off-camera Sun casters. Direct visible counts remain `None`.

### M1.3 — pass

Fresh `paths` exited 0 for all five named views. Each view has exact D/R equality, exact I/D/R same-arm repeats, a visibly missing forward terrain placement in the D-only omission, unchanged R pixels and arguments after that defect, and exact restored D/R equality. I uses the selected production meshlet encoder. D uses the actual direct encoder with a record mask. R independently opens and validates cooked products, derives complete contiguous index coverage per placement, checks base vertex zero/one instance and validates the total against resident coverage. It shares shaders, bindings, placement order and state. The reference and omission controls remain absent from ordinary builds. BASE comparisons build without draw-probe, cost records explicitly request production, and simulator routes use ordinary attachment.

Whole I/D frames and native crops show matching complete terrain, entering frame edges, horizons, object silhouettes, textures, reflection/matte response, Sun direction and cast shadows. The diagnostic edge mesh, outward-facing twins and translucent volume remain present. Full-resolution maximum-residual crops were also inspected, including an 8× gain supplement beside the unmodified differences. The omission montage visibly removes the terrain in each valley and the left placed diagnostic mesh.

| View | Changed I/D pixels | Maximum channel delta | Assessment |
| --- | ---: | ---: | --- |
| Noon 0° | 1,506 | 18 | Sparse ground/triangle and object-edge shading residuals. Same geometry, reflection, texture and shadows. |
| Noon 18° | 2,225 | 41 | Sparse ground and rock-edge residuals. Entering terrain and silhouettes agree. |
| Noon 36° | 2,128 | 38 | Sparse ground and rock-edge residuals. Sphere has correctly exited, terrain remains complete. |
| Low Sun | 1,504 | 50 | Sparse ground/edge shading residuals. Cast-shadow shape and lighting agree, including the inherited aliasing in finding 1. |
| Diagnostic | 139 | 56 | Sparse checker-surface edge residuals. Plain/mirrored facing, left boundary and volume agree. |

For every view, retaining all meshlets is exactly I and the full-range indirect reference is exactly D. This classifies the residuals as dependent on draw partitioning. The controls reject a visibility/culling explanation for these captures. The lower-level derivative/ordering cause remains unproved. No count/delta tolerance or ignored-pixel mask was used. No additional visible seam, hole, geometry discrepancy or unclassified difference was found.

### M1.4 — pass

The exact native indirect check exited 0. Separate BASE/current build roots used one newly cooked store with caller/fixture/witness overlays only. BASE/current and both same-build reruns are byte-identical in all five views. The visible/total counts match respectively at `171/680`, `170/680`, `153/680`, `171/680` and `227/258`. Current reports select Indirect and carry adapter-read support. Production shaders and the indirect cull/draw implementation are preserved.

### M1.5 — pass

The exact browser indirect check exited 0 in one owned Chrome `156.0.8078.81` session. Hardware WebGPU was enabled on Apple M3 Max/Metal. Noon 0°/18°/36° and low Sun match BASE/current and both-build repeats exactly, including visible counts. The current first adapter-backed record contains `INDIRECT_EXECUTION` and CookedStats selects Indirect. The T2 submitted-output export records frame metadata. The launcher reports denied Screen Recording preflights and clean browser exit.

### M1.6 — pass

`sh scripts/check_g5_tripwires.sh` exited 0 without recording a new baseline. Every metric passed its same-load history, and the bench asserts/reports adapter-backed Indirect. Source comparison against BASE retains one cull dispatch per placement with `ceil(meshlets/64)` workgroups, the same 160-byte uniform writes, conditional four-byte history writes, argument and buffer usages, multi-draw commands, count copy/clear and asynchronous readback. History writes now occur in common upload recording, before the same submission, with unchanged values/counts.

I read the retained raw `cost/paired.json.gz` and baseline-history records and checked their source identity against this checkout. Renderer/native/browser sources match the recorded proof. The one aggregate source difference is an iOS helper comment, outside those builds. Four release valley pairs alternate I/D order at 1920×1080, with 128 completed samples per arm, explicit actual quiet-load labels, commands and resource witnesses. The recorded per-run medians are forward I 1.240–1.244 ms and D 1.283–1.330 ms, contribution I 0.01025–0.01048 ms and absent on D, encode I 0.270–0.289 ms and D 0.149–0.188 ms. Forward already includes contribution work. These remain descriptive Mac observations with no path promotion or simulator timing claim.

### M1.7 — pass

The fresh standalone command exited 0 on owned simulator `EF869A81-48BC-4F6E-8BC0-2C5BF2E81876`, iPhone 18 Pro/iOS 27.2, host macOS 26.6.2. Twelve samples span 22.024695 seconds. Presented/cooked counts advance from 120/120 to 1440/1440. The adapter-backed record lacks `INDIRECT_EXECUTION` and `BASE_VERTEX`, selects Direct, draws cooked generation 1 on device generation 1, and has empty cooked errors. The command builds, cooks, installs, runs and deletes its simulator.

The ordinary iOS route invokes the existing `attach_cooked_scene`, whose unset-store path returns without attaching. Permission preflights are read-only calls in the actual LaunchServices parent. Before/after values are both denied, with matching bundle identity and real parent/child PIDs 85895/85896. Console observations happened to be locked/locked and were recorded without setup or refusal. Source inspection and regression cases accept unlocked, changed and unavailable observations and reject missing/failed/granted/unattributed preflights. Source/content/binary identities are retained. No grant request is present.

### M1.8 — pass

The separate fresh lifecycle command exited 0 on owned simulator `0F98CF5A-76EB-4085-A0C2-5F0FC2B03A68` and deleted it. Background snapshots hold presented/cooked counts at 1549/1549 and generation 1. Foreground resumes at 1678/1678. Device replacement redraws Direct on current generation 2 with cooked generation 1 retained and counters at 1797/1797. Lost empty submissions and recovery failures remain zero, errors remain empty, and existing route witnesses check revocation and recovery. Its own attributed permission endpoints are denied and its console observations are locked/locked metadata.

### M1.9 — pass

The separate rotation command exited 0 on owned simulator `4AEB7BE2-3F42-4D1A-AC66-B1714297EE51` and deleted it. The actual XCUITest orientation/swipe route goes portrait 1206×2622 → landscape 2622×1206 → portrait 1206×2622. PID 87635, device generation 1, one installed window and zero replacements persist. Surface epochs advance 2 → 3 → 4, with cooked submitted counts 1560 → 2419 → 3678 and Direct/drawn generation 1 throughout. Each extent proof has matching attachments, drawable, source, destination, full viewport and approximately 46° vertical FOV. Cooked errors are empty, and old-extent/stale presentation witnesses pass. Its own preflights are denied and the console remained locked. This run required no additional setup or grant.

### M1.10 — pass

The fresh `--visuals` command exited 0 on owned simulator `192B689C-A4F5-42CB-8857-7A5B5CEFC403`, then shut down and deleted it. All 32 image observations retain Direct, drawn generation 1 and empty cooked errors. Its own actual-launcher permission preflights are denied, and locked/locked console observations remain witnesses. I inspected all six whole native-source screenshots, displayed at reduced size, and their original-resolution crops. The native crop coordinates are retained in `review-held-comparison.json`.

| Inspection entry | Result |
| --- | --- |
| 1. Terrain to horizon and edges | Complete in start, end, low Sun, landscape and restored portrait, including each entering edge. No missing meshlet patches, cracks or holes. |
| 2. Ground material and channels | Brown/grey textured ground retains its base colour and fine texture. No default-colour surface or channel inversion. |
| 3. Objects and mirrored facing | Sphere shows the sky band and Sun reflection, rock has matte response and outward faces. Diagnostic checker twins face outward and share the expected lighting. |
| 4. Sun and ambient | Noon sky/ambient and low-Sun warmer side illumination agree with the corresponding Mac content. Held rotations preserve the selected noon lighting. |
| 5. Sun casters | Sphere and rock cast onto the terrain. Low Sun lengthens their shadows in the expected direction. Existing shadow aliasing is finding 1. |
| 6. Diagnostic volume | Translucent lit volume is visible on the right and continues through the expected diagnostic region. Its inherited hard-box/depth limitation remains. |
| 7. Rotated extent and aspect | Landscape and restored portrait fill their current extent. Projection/viewport witnesses agree with the surface, objects preserve their aspect and cooked geometry remains present. |
| 8. Held stability | Exact decoded pair comparisons and component inspection find stable scene pixels, with compositor differences classified below. No scene z-fighting or flicker was found. |

The noon-start, diagnostic and restored-portrait pairs are entirely byte-identical after decoding. Noon-end differs at 3,200 clock pixels within x=172–318/y=77–116. Low Sun differs at 29,200 Dynamic Island pixels within x=460–745/y=41–152 and 16 corner pixels within x=0–3/y=0–3. Landscape differs at 29,200 Dynamic Island pixels within x=41–152/y=460–745, 1,716 clock pixels within x=1588–1735/y=77–116 and 16 corner pixels within x=0–3/y=1202–1205. I viewed the landscape pair's left-side crop directly and confirmed the appearing black compositor pill. `review-held-components.json` retains every connected component. These comparisons counted every differing pixel, and the classifications describe the actual UI regions. Scene pixels outside those compositor changes are exact.

This is simulator function qualification. Physical-device quality, speed and cadence remain unmeasured.

### M1.11 — pass

I inspected all sixteen fresh samples chronologically as whole-frame sheets and original-resolution left-edge, right-edge and horizon crops. Start holds contain the sphere and rock, subsequent sampled views move them toward the edge and out of view, and the end holds retain complete terrain. Every sample has the correct textured ground and horizon, without a missing patch, sampled geometry pop or scene flicker. The changing Dynamic Island and clock belong to the simulator compositor. I sampled stills and frame strips, without watching or claiming continuous unsampled motion.

This fresh capture has substantially sparser coverage than the retained builder run. Sixteen screenshot calls span 94.537553 seconds from first begin to last end. The largest interval between sample starts is **14.723334 seconds**. Bracketing state records cover presented counts 1559–2273 for the full pan/hold observation, and 1679–2039 around the screenshot samples. Those counters are coarse state witnesses, with repeated observations between publications. They provide no exact screenshot-to-submission join. The camera control specifies a two-second start hold and eight-second 0°–36° pan. The recorded end hold is at least 96.99301 seconds. Screenshot call times and before/after states remain in the fresh `ios-visuals/report.json`.

| Sample | Start relative to first sample, seconds | Presented witness before/after |
| --- | ---: | --- |
| pan-00 | 0.000000 | 1679/1679 |
| pan-01 | 1.111406 | 1679/1679 |
| pan-02 | 2.245357 | 1799/1799 |
| pan-03 | 3.374943 | 1799/1799 |
| pan-04 | 4.490012 | 1919/1919 |
| pan-05 | 8.456493 | 1919/1919 |
| pan-06 | 13.497583 | 1919/1919 |
| pan-07 | 25.007875 | 1919/1919 |
| pan-08 | 36.718149 | 1919/1919 |
| pan-09 | 51.441483 | 1919/1919 |
| pan-10 | 56.036324 | 1919/1919 |
| pan-11 | 60.179521 | 2039/2039 |
| pan-12 | 72.679559 | 2039/2039 |
| pan-13 | 76.797249 | 2039/2039 |
| pan-14 | 82.385090 | 2039/2039 |
| pan-15 | 87.609674 | 2039/2039 |

The inspected samples support content completeness only at those observations. The long gaps leave intervening frames unqualified. This item explicitly scopes its conclusion to recorded samples and imposes no sampling-rate/cadence threshold. T2-CAPTURE M2 retains every-submitted-frame temporal proof. The retained builder report separately records its sixteen samples and 1.125629-second largest gap, with the same limited conclusion.

### M1.12 — pass

The G5 decision, maturity entry and evidence README identify both paths, conservative unknown-fact selection, Mac/Chrome adapter-backed Indirect and simulator Direct, explicit diagnostic masks, exact oracle and preservation results, cost conditions and their limits. Implementation, functional checks, builder visual observations and device measurements remain distinct. No path promotion, simulator timing or physical-device result is claimed. Finding 2 identifies minor historical prose that should be refreshed, while the current primary records contain the correct values and limits.

The T2 handoff remains conditional on this milestone's acceptance and leaves submitted-frame/time identity and every-frame capture proof with T2 M2. Builder scratch has an explicit closeout inventory, retained roots and expiry conditions. The reviewer scratch is separately owned and may be discarded after this review is integrated. The loop's prior outputs were preserved. Only the review and this milestone's reviewer fields/state are changed for integration.

## Final verification and verdict

Every required check was run from the claimed checkout. `./dev check` passed under the shared GPU lock. The fresh paths, native indirect, Chrome indirect, tripwires, standalone simulator function, lifecycle, rotation and visual commands all exited 0. No source fix was made. Review output consists of this file and each M1 item's `verdict`/`review` fields plus M1 `state`. All twelve item verdicts are `pass`, state is `done`, and `fails` remains 0. The two findings are advisory. There are zero blocking findings outside the items.

PASS M1
