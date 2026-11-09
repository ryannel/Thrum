# COOKED-DRAW M1 proof checkpoint

9 November 2026. M1 remains in progress. M1.1, M1.2, revised M1.3 and M1.9 now have proof alongside the standing
M1.7/M1.8 results. The remaining acceptance work includes a G3b/render-owner dependency.

## Current-extent blocker

M1.10 requires the rotated scene to fill the current extent. The fresh locked-console rotation run presents new
extents on the same process, window and device, with Direct selected and cooked submissions advancing. Its images
retain the fixed 640×360 scene aspect. Portrait uses viewport `[0,972,1206,678]` inside `1206×2622`; landscape uses
`[239,0,2144,1206]` inside `2622×1206`. Native screenshots show the corresponding borders. G3b iOS/render runtime
must supply matching scene/internal render aspect through orientation changes. `resize_surface` updates the
presenter while retaining its source extent. The build-milestone skill returns missing owning-module work to that
owner. [Layout/source record](rotation/layout-failure.json), [rotation states and commands](rotation/function.json.gz),
[landscape](rotation/12-landscape.png), [portrait](rotation/13-portrait.png).

## Revised Mac oracle

The opt-in `draw-probe` feature derives R arguments independently from validated cooked products and placement
order. D executes the real masked-record direct encoder. I executes selected production meshlet indirect. All five
views have exact D/R equality, exact I/D/R repeats, detected D-only terrain omissions, unchanged post-defect R
arguments/output and restored D/R equality. Commands, actual resource usages, source/binary/store hashes and
submitted-frame identities are retained. Ordinary builds omit the probe feature. [Run record](paths/report.json.gz).

| I/D comparison | Changed pixels | Maximum channel delta |
|---|---:|---:|
| Noon 0° | 1506 | 18 |
| Noon 18° | 2225 | 41 |
| Noon 36° | 2128 | 38 |
| Low sun | 1504 | 50 |
| Diagnostic | 139 | 56 |

These observations supply no threshold. Retain-all-meshlets controls reproduce I exactly in every view. Whole-frame,
native-crop and unthresholded-difference inspection finds matching geometry, silhouettes, texture/material detail,
lighting/shadows and volume. Plain/mirrored diagnostic twins face outward; terrain crosses the left frustum edge.
Residuals follow draw partitioning across the controls, without a derivative/ordering cause claim or an unclassified
scene discrepancy. Each view folder retains I, D, omitted-mesh, difference and native-crop images.
[Inspection assessment](paths/inspection.json).

## Correctness and remaining proof

`./dev check` passes 664 tests. Both cooked and material GPU suites exercise adapter-backed and masked records.
Replacement covers unchanged paths and capability changes both ways. The observed-resource arm also runs the
actual public `FrameHost::new` → `attach_cooked_scene` route in a child and proves unknown facts select Direct,
zero INDIRECT buffers, zero cull/count/readback resources and core indexed commands. Direct loading omits unused
meshlet bounds/ranges. [Check record](check.json). Standing locked simulator function/lifecycle proof is retained.
[Function/lifecycle record](ios-function.json.gz).

M1.4/M1.5 Mac/Chrome base comparisons, M1.6 cost/tripwires, complete simulator stills/pan and M1.12 remain open.
The incomplete base-comparison command refuses that mode. Chrome flags/selection and comparative cost remain
unqualified for this change. Simulator results establish function. T2-CAPTURE M2 remains gated on M1 acceptance.

Selected originals and compact metadata follow the repo-structure retention rules. Scratch in `tmp/cooked-draw/`
retains reconstructible stores/captures until owner disposition and the next builder's inspection. Source generation
lives in `scripts/cooked_draw_fixture.py`; [BASE](BASE) records the pre-implementation parent after rebase, pending
before/after proof. [Selected evidence hashes](selected-hashes.json). The older `paths-failure.json.gz` and scratch
shader controls retain historical evidence of the superseded oracle.
