# COOKED-DRAW design review

9 November 2026. Reviewed integrated revision `82fa10acbf74d4e9746cfd11170d5592fb6c3185`.
Independent design review using `orchestrate:review-milestone` and `thrum-engineering`; no agents launched.
Read the [brief](../../design/modules/COOKED-DRAW-BRIEF.md) first, then the
[design](../../design/modules/COOKED-DRAW.md), [plan](../../design/modules/COOKED-DRAW-PLAN.md) and
[done-when file](../../design/modules/COOKED-DRAW-DONE.json). No milestone is being judged.

**Verdict: DESIGN CHANGES. Three findings: one blocking, two advisory.** The direct full-range indexed draw is a
reasonable scoped fallback, and the adapter-backed production routes have a coherent selection mechanism. The
blocking exception is the proposed adapter-less record: it invents indirect support for an unrestricted public
constructor, leaving a route to the original failure. Fix that selection rule and its proof before building.

## Requirements derived from the brief

- Draw the existing cooked scene without indirect execution, using the adapter's actual registry facts rather than
  platform, backend or device-name switches.
- Retain indirect selection on supported Mac and Chrome adapters, with byte-identical before/after output and
  unchanged per-frame work.
- Prove simulator content is complete, textured, lit, shadowed and continuous during a camera pan. This proves
  function, with no simulator speed or physical-device claim.
- Preserve the contribution seam, submitted history, GPU generation/lifetime rules and evidence semantics. Name
  interface changes, their owners and the T2-CAPTURE M2 handoff.
- Account for frequent work and storage before implementation, then measure and use existing tripwires rather than
  inventing a budget.

These requirements were the inspection list for the document/source comparison. This review inspects no new images
or motion and makes no running correctness, visual or performance claim.

## Findings, worst first

### F1 — Blocking: adapter-less capture asserts an unsupported execution capability

**Verified design defect; runtime consequence inferred from the proposed selection rule, not executed.**
The design's selection section ([COOKED-DRAW.md:50](../../design/modules/COOKED-DRAW.md#mechanism)) selects indirect
whenever the record contains `INDIRECT_EXECUTION`, then says `capture_device` assumes WebGPU-compliant flags.
M1.1 explicitly requires that assumption. Marking the flags as assumed does not affect the stated selector.

The existing adapter-less entry point is not restricted to tests or to WebGPU-compliant adapters:
`examples/v1-game/src/host.rs:339` exposes `FrameHost::new(device, queue, config)`, calls
`DeviceCapabilities::capture_device` at line 344, and forwards the result to `new_with_capabilities`.
`crates/thrum-render/src/capabilities.rs:43` likewise documents loss of the adapter handle, with no WebGPU-compliance
precondition. Cooked content can then be attached through the ordinary public `attach_cooked_scene` entry point
(`examples/v1-game/src/content.rs:66`). The design's assertion that the three platform launchers use adapter-backed
capture does not close this public route.

Trace that sequence with a device whose adapter lacks indirect execution: the proposed record asserts the flag,
the cooked selector chooses indirect, and binding the set still creates the unsupported argument buffer. That is
the same failure class as the retained
[simulator prerequisite failure](../render/t2-capture-m2/README.md). The brief requires actual capability selection;
C04's single registry authority does not make invented support a hardware fact.

**Resolution:** do not enable indirect execution from unknown/assumed adapter facts. Define a conservative
adapter-less outcome, or require a verified adapter-backed record for this operation with an explicit admission
failure when it is absent. Preserve indirect selection for genuinely captured supported adapters. Revise M1.1 so a
test covers the adapter-less constructor/record and proves it cannot select unsupported indirect work; keep the
existing real-adapter and masked-record arms. A comment or provenance marker alone is insufficient.

### F2 — Advisory: sparse simulator samples cannot establish absence of one-frame holes

**Verified proof limitation.** M1.10's statement excludes one-frame holes, but its inspection permits `simctl`
screenshots at unspecified short intervals. A missing frame between screenshots is unobservable. Even decoded
simulator video needs its sampling limits recorded before it supports a temporal claim.

This is not a demand to implement T2-CAPTURE M2 inside its prerequisite. The profile deliberately permits interim
`simctl` function evidence within recorded timing limits (`ORCHESTRATE.md:34`), and reserves stronger submitted
temporal proof for T2 (`:46`). The proposed full-range draw also avoids the delayed-visibility mechanism rejected
in the alternatives table.

**Resolution:** record the pan's sample coverage, holds and gaps, and state the M1 inspection conclusion within
those limits. Keep complete submitted temporal-output qualification with T2 M2. If the absolute one-frame statement
is retained, name evidence that actually covers those frames. Otherwise a reviewer must qualify that part of the
result rather than infer unseen continuity from two clean samples.

### F3 — Advisory: the cooked simulator route has no explicit rotation row

**Verified coverage omission.** `ORCHESTRATE.md:34` names rotation and background/foreground as simulator edge
conditions. M1.8 names background/foreground and device-generation replacement; the plan's simulator work names no
rotation with cooked content attached. The existing diagnostic route has portrait/landscape/portrait checks
(`examples/v1-game-ios/ios/route.py:535`), but those do not establish the newly attached cooked camera/content
behavior.

**Resolution:** name a cooked-content rotation row, including current-extent output and retained content after the
transition, or explicitly identify the existing route command that will run it with cooked content. Preserve any
attended UI-automation reason separately from the required locked engine-state rows. This is advisory because the
profile already obliges a milestone reviewer to exercise that edge; the omission need not change the draw design.

## Mechanism, scope and composition

The inspected source supports the main proposed mechanism:

- `crates/thrum-asset-gpu/src/load.rs:601` expands indices in meshlet order into one contiguous buffer and records
  their ranges. Drawing its full range preserves primitive order relative to the retained indirect draws.
- `draw.rs:590` binds the mesh's material, vertex/instance buffers and index buffer before multi-draw. The proposed
  direct call can reuse those inputs. Existing Sun casters already use a full-range, zero-base-vertex direct draw
  (`draw.rs:557`). Keeping shadow visibility independent of camera meshlet culling is correct.
- `cull.wgsl` writes zero base vertex and first instance. The placement admission rule allows positive uniform
  scale and rigid rotation (`placement.rs`), which matters to the cone/bounding-sphere parity argument. The claim
  of identical pixels remains a hypothesis for M1.3, not a result of this review.
- `crates/thrum-render/src/synthetic.rs:756` can skip `encode_compute` while still calling `encode_after_compute`.
  Therefore moving history maintenance to a shared step is feasible without a new contribution callback. The
  implementation must also make the readback/counter logic conditional, as M1.2 already requires.
- The iOS route captures its adapter on initial preparation and recovery (`ios/render.rs:409`, `:550`). Attaching
  cooked content before moving the host into its threaded owner fits the existing route. M1.8 appropriately tests
  that the contribution survives reconstruction and lease transitions.

The cost account identifies the meaningful trade: remove per-meshlet cull/argument/readback work and encode one
draw per placement, in exchange for processing every mesh index. The valley counts, existing tripwires and paired
Mac measurements are appropriate for this commission. No reason was found to require a second CPU culler,
per-frame index compaction or delayed readback before the fallback can be built. There is no invented simulator
performance gate, and no proposal to change supported-device path selection after merely observing timings.

The contracts table names producers and consumers for the capability/surface additions, stats and iOS attachment.
The required owner agreements are still pending; design approval would not constitute those agreements. The
explicitly deferred synthetic culled modes are an adjacent G3c gap, not evidence that the cooked fallback must own
another renderer. T2 M2's `needs: COOKED-DRAW:M1` is present, and stronger capture proof remains downstream.

## Done-when judgeability and resistance to substitutes

These are assessments of planned proofs, not milestone verdicts. The new parity and simulator commands are planned
deliverables; their absence before implementation is not a failure.

| Item | Assessment |
| --- | --- |
| M1.1 | Revise for F1. The adapter-backed selection, explicit mask and device-generation tests otherwise reject a simulator-name switch. |
| M1.2 | Judgeable by the two-record GPU suite and source trace. It covers discard, history, recovery, remote content, material changes and casters rather than just drawing once. |
| M1.3 | Judgeable. Matched build/store, multiple views, diagnostic geometry and same-path reruns are a strong parity oracle. Audit that each arm actually selects its named path and compares separate outputs. |
| M1.4 | Judgeable. A pinned base, shared cooked store, unchanged cull counts and rerun control can reject a changed indirect image. |
| M1.5 | Judgeable on real Chrome WebGPU, with the adapter and selected path recorded. Browser/session identity and separate base/claim bundles must survive the harness. |
| M1.6 | Judgeable. The command/write/resource diff adds evidence that pixel equality alone cannot provide; tripwires and labelled paired timings cover cost without promoting a new selector. |
| M1.7 | Judgeable if the locked target is available. Require advancing presented-frame witnesses with a genuinely drawn cooked generation, not only successful loading or a selected-path label. Missing target or lock evidence cannot pass. |
| M1.8 | Judgeable for its stated background and device-replacement rows. Add the rotation coverage noted in F3. |
| M1.9 | Judgeable. The specified native-size views/crops separately test geometry, textures, facing, lighting, shadows and volume presence; a successful present counter alone cannot pass. |
| M1.10 | Judgeable as sampled function evidence, with the temporal limitation in F2. Sparse samples cannot prove the absolute one-frame clause. |
| M1.11 | Judgeable by comparing code and per-target evidence with the ledger, G5 record and handoff. Its separate maturity dimensions prevent functional success from becoming a speed claim. |

## Validation and limits

Read the project and skill graded examples, relevant C04/C06/C07/C12 clauses, G5 and hardware-native decisions,
the retained simulator blocker, contribution/draw/load/cull/placement code, capability and host constructors,
iOS preparation/recovery, relevant cooked GPU tests and simulator route checks. This is a scoped boundary read,
not a claim to have read the whole renderer or independently reproduced the historical simulator fault.

The done-when JSON parses and contains all 11 M1 items. `./dev check-docs` passes with `RUSTC_WRAPPER=` and the
installed Rust 1.99.1 compiler explicitly selected. The first unadjusted invocation failed because the configured
`sccache` compiler probe was denied by the sandbox; no source or configuration was changed to rerun it. GPU,
browser and simulator checks were not run for this unimplemented design. Only this review file is changed.

DESIGN CHANGES: 3 findings
