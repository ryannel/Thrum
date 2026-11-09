# Apple tile recipe brief (G3t)

9 November 2026. Commissioned by the [build order](../../BUILD-PLAN.md#active-gate-order-and-budgets),
under the [architecture responsibility map](../ARCHITECTURE-BRIEF.md#proposed-module-map).
This brief is the input to independent design review; it states requirements rather than selecting mechanisms.

The same reusable renderer must draw the cooked V1 valley on Apple hardware and portable WebGPU. Apple should
avoid unnecessary external attachment traffic, upload copies and repeated per-frame work while preserving the
portable image, lighting meaning, interpolation, visibility and temporal history. Chrome remains portable.
The target is this M3 Max; the iPhone simulator proves function only. Physical-device measurements stay owed.

G3t owns recipe selection in the existing generation registry, graph-derived attachment storage, scene/present
dataflow, upload transport, internal instance transport, light-list execution and the isolated Apple feature
handoff. It must integrate with the existing visibility owner rather than creating another culler. F1/platform
owns lifecycle, device and surface; V1/FrameHost owns submissions, slots, completion and retirement; W1 owns world
state; V2 owns visibility and draw membership; V3 owns light/material meaning; V4/G4 owns reconstruction and display.

Required outcomes:

- The cooked valley keeps its sky, Sun, stars, local lights, shadows and mirrored geometry through moving views,
  a camera cut, day/night variants, dropped views, slot reuse, resize and device replacement.
- Apple-specific choices come from one immutable generation registry. Unsupported or rejected optional paths
  have prepared portable fallbacks. Required resources and pipelines exist before local readiness.
- Attachment loads/stores and transient eligibility follow actual readers, including occlusion, capture and future
  reconstruction. Reduced logical traffic must not be reported as measured physical bandwidth.
- Upload writes cannot race in-flight GPU reads. Dirty changes survive skipped frames and slot reuse. Compact
  transport must retain every value that supported shader consumers use, including supplied normals and history.
- Light overflow and mirrored/degenerate rasterization preserve established renderer semantics. Cost/scaling rows
  must expose CPU fallback and validation work rather than concealing them.
- G4 receives one prepared, generation-owned native-feature boundary with graph resources, a portable fallback,
  reset, completion, errors and timing. No rival queue, raw drawable bridge or mixed raw/wgpu encoder is allowed.
- Correctness, agent visual review and lifecycle evidence are mandatory. Performance numbers are baselines and
  tripwires, never invented speed/allocation gates. Promote accelerated paths only on a measured win for their target.

The creator continues to cook the same source and use the public scene contribution seam. No new game schema,
content fork or authoring dependency belongs in shipping rendering. Extend existing crates unless a demonstrated
boundary requires otherwise; register actual dependencies and narrowly justify any unsafe backend access.

Sequence work so an offscreen game proof can proceed while PLATFORM-RUNTIME-MAC M2 waits for an unlocked display.
That does not close Mac cadence, diagnose the parked atmosphere crossing or commission the complete native recorder.
Record live/display-only qualification separately; do not replace it with callbacks or offscreen timing.

Review completion: derive the required capabilities from this brief first, then compare the design, plan and JSON
criteria against them, C04/C06/C07/C08/C12 and the actual consumers. Reject missing lifecycle/semantic mechanisms,
unresolvable milestone dependencies or proof that could pass while drawing less required work.
