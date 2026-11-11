# Apple tile recipe brief (G3t)

9 November 2026. Scoped re-plan explicitly recommissioned by the lead after two failed design reviews, under the [build order](../BRIEF.md#active-gate-order-and-budgets)
and the [architecture responsibility map](../../design/ARCHITECTURE-BRIEF.md#proposed-module-map).
This brief is the input to independent design review; it states requirements rather than selecting mechanisms.

The same reusable renderer must draw the cooked V1 valley on Apple hardware and portable WebGPU. Apple should
derive attachment storage from actual readers and supply G4's native boundary while preserving the
portable image, lighting meaning, interpolation, visibility and temporal history. Chrome remains portable.
The target is this M3 Max; the iPhone simulator proves function only. Physical-device measurements stay owed.

G3t owns recipe selection in the existing generation registry, graph-derived attachment storage, scene/present
dataflow, a replaceable pre-tone scene-linear input and the isolated Apple feature handoff. Upload/instance
transport, expanded light-list execution, scratch sharing and mirrored-range consolidation are deferred optimizations,
not prerequisites of this scoped G4/Mac M3 commission. Existing semantics must survive unchanged.
It integrates with the existing visibility owner rather than creating another culler. F1/platform
owns lifecycle, device and surface; V1/FrameHost owns submissions, slots, completion and retirement; W1 owns world
state; V2 owns visibility and draw membership; V3 owns light/material meaning; V4/G4 owns reconstruction and display.

Required outcomes:

- The cooked valley keeps its sky, Sun, stars, local lights, shadows and mirrored geometry through moving views,
  a camera cut, day/night variants, dropped views, slot reuse, resize and device replacement.
- Apple-specific choices come from one immutable generation registry. Unsupported or rejected optional paths
  have prepared portable fallbacks. Required resources and pipelines exist before local readiness.
- Attachment loads/stores and transient eligibility follow actual readers, including occlusion, capture and future
  reconstruction. Reduced logical traffic must not be reported as measured physical bandwidth.
- Upload writes cannot race in-flight GPU reads. Dirty changes survive skipped frames and slot reuse. Retain all
  admitted values, supplied normals and history; this commission requires no transport/layout rewrite.
- Light overflow and mirrored/degenerate rasterization preserve established renderer semantics. Cost/scaling rows
  must expose CPU fallback and validation work rather than concealing them.
- G4 receives one prepared, generation-owned native-feature boundary with graph resources, a portable fallback,
  reset, completion, errors and timing. No rival queue, raw drawable bridge or mixed raw/wgpu encoder is allowed.
- Correctness, agent visual review and lifecycle evidence are mandatory. Performance numbers are baselines and
  tripwires, never invented speed/allocation gates. Promote accelerated paths only on a measured win for their target.

The creator continues to cook the same source and use the public scene contribution seam. No new game schema,
content fork or authoring dependency belongs in shipping rendering. Extend existing crates unless a demonstrated
boundary requires otherwise; register actual dependencies and narrowly justify any unsafe backend access.

Sequence shared capture first: APPLE-TILE M0 needs `T2-CAPTURE:M0`, then adds its own linear recipe comparison
through that owner. Offscreen work proceeds independently of PLATFORM-RUNTIME-MAC M2's attended panel cadence.
T2 Mac M0 and Chrome M1 are done. Earlier simulator diagnostic function can use labelled interim simctl
observations; final cooked simulator function waits for COOKED-DRAW M1. Stronger simulator submitted/runtime-motion
proof stays with T2 M2 and does not gate the G4 native boundary or Mac M3.
That does not close Mac cadence, diagnose the parked atmosphere crossing or commission the complete native recorder.
Record live/display-only qualification separately; do not replace it with callbacks or offscreen timing.

Review completion: derive the required capabilities from this brief first, then compare the design, plan and JSON
criteria against them, C04/C06/C07/C08/C12 and the actual consumers. Reject missing lifecycle/semantic mechanisms,
unresolvable milestone dependencies or proof that could pass while drawing less required work.
