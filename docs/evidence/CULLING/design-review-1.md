# CULLING design review

11 November 2026. Reviewed integrated commit `d90f66ac1638407b9821fee3868c060cd361bdca`.
Result: **Design Changes Requested — one blocking finding.** This is a design review, not runtime,
visual or performance qualification. No previous design review exists in this checkout.

## MF1 — Must Fix, verified: the live-switch transaction relies on a replacement interface that cannot deliver its stated behaviour

Affected: [DESIGN.md](../../plans/CULLING/DESIGN.md), lines 216–220 and 310;
[DONE.json](../../plans/CULLING/DONE.json), M2.5 and M2.6; C04/C06 live-change and preparation rules.

The design says the existing host republication prepares another culling mode off the frame path, keeps the old
mode presenting, and swaps without creating a pipeline on the frame path. Following that interface through the
current producer and consumers exposes a missing part of the design:

- `crates/thrum-render/src/runtime.rs:614` and `:658` compile `visual_prepare_inputs` and `publish_visual` only
  for non-Wasm targets. The claimed existing path therefore cannot implement the Chrome switch.
- `crates/thrum-render/src/runtime/visual.rs:41` accepts lighting, exposure and revision, using a private copy
  of the existing scene. There is no requested-mode input. Publication at `runtime.rs:680` replaces the prepared
  renderer but updates only the retained scene's view, not its culling selection. Merely changing the candidate
  scene would leave the host's retained selection inconsistent for a later preparation or device recovery.
- More substantially, `runtime.rs:688` calls `rebuild_contribution` synchronously during publication, after
  installing the replacement. For the ordinary cooked valley, that reaches
  `crates/thrum-asset-gpu/src/lib.rs:649`: `rebuild_inner` constructs `draw::Pipelines::new` at line 661 and
  retargets the material. Same-device residency is retained, but the pipelines are rebuilt. This work is not in
  the detached preparation. A rebuild error detaches the contribution (`runtime.rs:422`), so it can also remove
  the cooked scene after the switch was committed.

These are verified interface facts, not a measured hitch claim. An ordinary input that exposes the gap is a
mode change while flying the cooked valley, with its scene contribution attached; on Chrome even the proposed
publication entry points are absent. The open question about whether republication restages content visibly
does not settle pipeline creation, atomic failure, browser scheduling or who extends the shared host interface.
Preserving residency alone does not remove the synchronous pipeline rebuild.

**Consequence:** implementing the named reuse literally cannot provide the promised switch on both targets.
The builder must invent a consequential cross-module transaction while implementing M2. An endpoint-only
switch witness can also pass on the synthetic field while omitting the cooked contribution that triggers this
problem. M2.5's creation question must cover publication work even when the accounting calls it `Transition`.

**To resolve:** design and assign the missing host/browser/contribution extension, or choose another prepared
mode-switch mechanism. Specify how the selected mode is retained, how the old scene continues presenting during
preparation on each target, and how contribution pipelines and bindings are ready before an atomic commit.
Define failure/stale-candidate handling so the existing scene remains usable. Settle that ownership with the
frame-runtime and content interfaces and add any actual prerequisite. Make the switch proof exercise the cooked
valley on Mac and Chrome, observe submissions during preparation and the publication boundary, and reject
publication-time pipeline creation or lost contribution output. This does not require moving M1 behind M2 work.

## S1 — Suggestion, verified proof limitation: separate tolerance and footprint witnesses

Affected: [DESIGN.md](../../plans/CULLING/DESIGN.md), Depth tolerance; M1.3.

The required zero-tolerance mutant demonstrates that the mesh witness needs depth slack. It does not by itself
demonstrate that the same witness needs the x/y footprint widening, or independently establish the assumed
rasterizer interpolation allowance. Consider retaining a separate footprint-widening mutant and identifying
which witness exercises which bound. Keep the four-ulp interpolation term described as an assumption unless
stronger evidence establishes it. This sharpens an already meaningful first-milestone proof; it is not another
blocking finding or a demand for an exhaustive numerical study.

## Request and mechanism assessment

I read the complete `briefs` array in [ACCEPTED.json](../../plans/CULLING/RECORD.json) first. It contains one
task, dated 11 November, with no retry or re-plan briefs. Together with the project brief, it requires a small
first milestone available after APPLE-TILE M2, completion of G3c, and an explicit way to catch changes to another
module's exact baselines before landing.

The three-milestone scope addresses G3c: mixed-mesh occlusion first, capability selection and measured defaults
second, then the valley and gate closure. The 10,000-entity exit scene remains in scope. Deferring the final
cooked-entity assembly to SHOWCASE is explicit, with the instance/meshlet ownership and command reruns recorded
in ARCHITECTURE. The native-only optimizations and selectable-but-not-default Occlusion are explicit assumptions,
not concealed narrowing. M1's bounded extension to existing bucket compaction is a reasonable first slice.

The core occlusion mechanism is credible. It renders current transforms of a historical subset into depth,
tests every active instance conservatively, and draws survivors in the oracle's bucket and member order. Old
visibility bits select occluders rather than vetoing newly visible instances. Logical-slot history and a depth
twin for each raster class address real bucket-layout and facing hazards. The proof explicitly depends on
prepass parity and conservative bounds; M1 is where the additional mesh evidence must establish those conditions.
I do not treat the existing cube evidence as proof for the new meshes.

The instance-to-cluster progression has precedent in
[Haar and Aaltonen's primary presentation](https://advances.realtimerendering.com/s2015/aaltonenhaar_siggraph2015_combined_final_footer_220dpi.pdf).
The distinction between GPU-written instance counts and GPU-written numbers of draws is also justified:
[wgpu 30's feature documentation](https://docs.rs/wgpu-types/30.0.0/wgpu_types/struct.Features.html#associatedconstant.MULTI_DRAW_INDIRECT_COUNT)
lists the latter feature for DX12 and Vulkan, not Metal or WebGPU. Neither source proves Thrum's numerical bounds
or performance.

The exact-baseline response is substantive: ARCHITECTURE defines owner-controlled comparisons, the BASELINE-GUARD
land check, and full-matrix checks tied to the claimed revision. Every CULLING milestone carries the M0 matrix.
The architecture explicitly acknowledges the interim period before the guard is built and orders it first.
The pre-land full-matrix obligation remains necessary during that period; the guard's planned existence is not
evidence that it already protects main. No baseline refresh is authorized by this CULLING plan.

## Done-when item review

These are assessments of whether the proposed proofs can judge the work when built, not milestone verdicts.
New test files and script cases are expected deliverables; their absence before implementation is not a defect.

| Item | Assessment |
|---|---|
| M1.1 | Judgeable. Per-frame Direct comparison, resolved-mode/hidden-count assertions and the two wrong-prepass mutants reject meaningful substitutes. |
| M1.2 | Judgeable. Full path through the first cut and a nonvacuous hidden-count condition exercise the game workload. |
| M1.3 | Judgeable first falsification of the mesh tolerance; S1 sharpens what each part proves. |
| M1.4 | Judgeable. Resize and device replacement must occur with actual hidden geometry. |
| M1.5 | Judgeable exact-baseline command; release build and fresh cook are explicit. |
| M2.1 | Judgeable. Tracing the registry through prepared pipelines prevents an independent device probe or silent fallback from passing. |
| M2.2 | Judgeable on the simulator as function only; a fallback report alone cannot establish that the complete scene draws. |
| M2.3 | Judgeable for its stated 16 Chrome views, with same-target oracle and actual occlusion. It does not claim continuous path equality. |
| M2.4 | Judgeable from paired runs and code selection. Nonempty prepass and hidden counts reject the previously withdrawn benchmark's failure mode. |
| M2.5 | Blocked by MF1: the publication/contribution transaction is not designed to meet this statement. |
| M2.6 | Blocked by MF1: the supposed existing publication API is unavailable on this target, and endpoints alone miss the shared preparation promise. |
| M2.7 | Judgeable exact-baseline command, as M1.5. |
| M3.1 | Judgeable once implemented: the native command must compare every submitted route frame for both recipes. |
| M3.2 | Judgeable checkpoint proof on Chrome; distinct from every-frame native equality. |
| M3.3 | Judgeable from the recorded paired rows and actual reported default, using the design's two-run rule. |
| M3.4 | Judgeable mixed-mesh route against bucket-ordered Direct, including Sun-caster behaviour. |
| M3.5 | Judgeable Chrome counterpart over its stated checkpoints. |
| M3.6 | Judgeable over the existing temporal-sampling sequence; no completed temporal reconstructor is necessary to test culling under jitter. |
| M3.7 | Judgeable with the named T2 modes, edge/silhouette inspection and separate fixed-time field frames. |
| M3.8 | Judgeable with Chrome T2 realtime and fixed output; ordinary canvas endpoints cannot substitute for motion. |
| M3.9 | Judgeable closing code/evidence audit. It explicitly covers stable mesh offsets and compute-readable materials, which image equality alone would miss. |
| M3.10 | Judgeable exact-baseline command, as M1.5. |

## Review scope and verification

Read the complete CULLING design, plan and done-when file; the project brief, relevant module-list ownership,
interface and baseline sections; C04/C06/C07 and the project review calibration. Inspected the existing culling,
mesh-route, occlusion shader, visual preparation/publication, cooked contribution rebuild, browser host and
timing boundaries. Consulted the prior G3c occlusion evidence as historical evidence, not as a current test run.
No other agents were launched.

No game run, GPU benchmark, screenshot or motion capture was needed to establish the interface finding. None
was performed, and this review makes no current image, cadence or device-performance claim. Only this review
file is changed; DONE.json remains untouched. `./dev check-docs` passed (695 Markdown files, 3,591 local links,
30 module records, crate policy and design hashes); `git diff --check` reported no errors.

DESIGN CHANGES: 1 blocking findings
