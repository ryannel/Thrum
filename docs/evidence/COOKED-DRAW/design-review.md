# COOKED-DRAW design review

9 November 2026. Reviewed integrated tip `2394162b498fd498f93e11ffa54d4ba471430595` with
`orchestrate:review-milestone` and `thrum-engineering`, following the design-review procedure.
The review covers the complete brief, design, plan and twelve done-when items, including the revised M1.3 oracle.

**DESIGN OK. Zero blocking findings and zero advisory findings.** The revised oracle can qualify the production
direct encoder while preserving the commissioned indirect output and work. M1 implementation and target
qualification remain outstanding. Interface-owner agreements retain the obligations recorded in the plan.

## Requirements derived before reading the design

- Draw cooked content on adapters lacking indirect execution, selecting solely from actual capability-registry
  facts. Handle unknown facts conservatively and reselect for a replacement device generation.
- Keep indirect selection on supported Mac and Chrome routes, with byte-identical before/after output and
  unchanged per-frame work.
- Render complete, textured, lit and shadowed cooked content on the simulator. Cover background/foreground,
  replacement and rotation, and keep simulator conclusions within function evidence.
- Preserve the contribution seam, single state authorities, C06 GPU lifetime, C07 motion history and C12 evidence
  meaning. Identify changed interfaces, producers, consumers and required agreements.
- Account for repeated work, storage, transfers and waits before building. Record measurements and existing
  tripwires without inventing a budget.
- Deliver the prerequisite for T2-CAPTURE M2 and keep its submitted-frame temporal proof with that consumer.

These requirements formed the inspection list for the design comparison. I formed the mechanism and proof
assessment before reading the previous design review. The previous review applies to the earlier oracle.

## Mechanism, scope and boundary assessment

The record-backed selector and whole-range direct draw satisfy the brief. Adapter-backed capture supplies known
flags. Adapter-less `capture_device` supplies unknown facts and selects direct, covering the public
`FrameHost::new` route. `SceneSurface` carries the fact to `CookedAssets`, which reselects at rebuild. The mask clears
the registry flag through explicit diagnostic opt-in. M1.1 checks all three provenance cases and device reselection.
Mac, browser and iOS preparation retain adapters for capture. The existing constructor and platform boundaries
support these additions. Relevant source includes `crates/thrum-render/src/capabilities.rs`,
`examples/v1-game/src/host.rs` and `examples/v1-game-ios/src/ios/render.rs`.

The existing loader expands meshlet triangles into contiguous global index ranges. A full-range indexed draw with
base vertex zero and one instance fits that representation. The existing forward draw binds the shared material,
vertex streams, placement instance and index buffer. Sun casters already draw full ranges. The contribution executor
checks `computes()` before beginning compute and calls `encode_after_compute` independently. This supports omitting
cull dispatches and readbacks while retaining shared upload, history and submission handling. The design explicitly
moves history work out of culling and requires both-path tests for discard, capture without history, recovery,
remote feeds, material promotion and casters. Relevant source includes `crates/thrum-asset-gpu/src/load.rs`,
`draw.rs`, `lib.rs` and `crates/thrum-render/src/synthetic.rs`.

The capability and surface additions retain C04's registry authority. The common tentative/submitted protocol
retains C06 and C07 meaning. The design's interface table names render/registry, G5, V1 game and iOS-shell ownership
and affected consumers. The plan requires agreement requests before building and explicitly permits building against
those proposals, with objections returning for replanning. The separate synthetic-renderer indirect gap is explicitly
assigned to G3c. The scoped fallback can proceed with that deferral.

The cost account covers per-placement commands and writes, meshlet dispatches and argument storage, count readback,
all-index vertex work and representative valley scale. One direct draw per placement avoids CPU meshlet traversal
and delayed GPU visibility feedback. The diagnostic reference's O(P) arguments and commands are confined to proof
runs. M1.6 preserves the production indirect command/resource audit and existing tripwires, alongside paired,
order-alternated Mac measurements with load labels. The simulator-first order exposes further unsupported features
before completing the comparison harness.

The iOS route uses the existing cooked-content selection. The planned state witnesses extend lifecycle and device
replacement checks to actual cooked drawing. The existing `examples/v1-game-ios/ios/route.py` rotation row supplies
the portrait/landscape transitions. Sampled-pan coverage includes sample times, holds and maximum gap, with its
conclusion limited to those samples. T2-CAPTURE M2 retains the stronger temporal proof, and its done-when file
contains `needs: ["COOKED-DRAW:M1"]`.

## Revised parity oracle

The brief requires exact indirect before/after output. The earlier additional direct-versus-meshlet byte-equality
premise has contrary checkpoint evidence. I inspected the historical JSON and reproducer source in Git object
`9c230ff4f54294078bb59cffb851e8ec6c68c83e`, verified all thirteen selected artifact hashes, and independently decoded
the retained production PNGs with the repository's PNG reader. The 1920×1080 RGBA images differ at 1,506 pixels,
with maximum channel delta 18. The retain-all control's recorded raw-output hash matches the decoded indirect image,
and the full-range-indirect control's recorded raw-output hash matches the decoded direct image. Those checks
corroborate the retained comparison records. Fresh execution and perceptual assessment remain M1 work.

The specification supports the design's caution about derivatives. WGSL describes approximate quad-based
derivatives and gives derivative builtins no finite accuracy bound. WebGPU defines indexed-indirect drawing through
buffer-supplied arguments. These facts support testing equivalent command arguments at matching partitions.
Attributing the particular Metal discrepancy to derivatives remains an inference requiring further evidence.
[WGSL derivatives](https://www.w3.org/TR/WGSL/#derivatives),
[WGSL accuracy](https://www.w3.org/TR/WGSL/#floating-point-accuracy),
[WebGPU indexed indirect drawing](https://www.w3.org/TR/webgpu/#dom-gpurendercommandsmixin-drawindexedindirect).

M1.3 now has three distinguishable roles. D executes the production direct encoder through the masked record.
R emits full-range indirect commands derived independently from cooked ranges and placement records, with validated
coverage. I executes ordinary selected meshlet culling and drawing. D/R equality remains exact in every original
view, with exact same-arm repeats for all three arms. Independent argument validation and command/submission
witnesses address a reference that repeats faulty direct arguments. The D-only missing-mesh control must break
comparison while R stays unchanged, and restoration must recover exact equality. This can expose a comparator that
reads one arm twice or ignores the altered submitted output.

I/D scene correctness also remains a gate. Full images and unthresholded differences receive whole-frame and
native-resolution inspection across terrain edges, silhouettes, entering geometry, mirror handling, textures,
lighting, shadows and volume. Geometry/culling defects, objectionable shading seams and unclassified scene
discrepancies block acceptance. The checkpoint's pixel counts supply historical observations only. Further
investigation is required when these checks fail or a discrepancy remains unclassified. This supplies a bounded
functional proof without requiring the precise low-level cause before building the replacement harness.

M1.4 and M1.5 still compare selected production indirect output against the pre-implementation base byte for byte.
M1.6 still audits its work. Probe controls must remain opt-in and excluded from those comparisons and timings.
The design therefore preserves the brief's indirect invariance while revising the unsupported cross-partition oracle.

## Acceptance proof assessment

The table assesses judgeability and plausible-fake coverage of the planned proofs. Milestone verdicts remain for a
later review of the implementation and running targets.

| Item | Assessment |
| --- | --- |
| M1.1 | Judgeable through adapter-backed, masked and unknown-fact tests, public-host attachment, generation reselection and source tracing. Resource/command inspection can reject an indirect path hidden behind a direct label. |
| M1.2 | Both suite arms assert their selection and path-specific counts. Stateful cases and allocation/recording review can reject a fallback that only draws a correct still. |
| M1.3 | Exact D/R comparisons, all original views, independent argument coverage, arm identities, repeats, a D-only defect and restoration provide discriminating automated proof. Explicit I/D inspection and residual classification retain the scene-quality gate. |
| M1.4 | The pinned pre-implementation base, shared store, original views, visible counts and repeat control permit exact Mac regression comparison. Separate base and claim binaries and their submitted outputs must be traced during implementation review. |
| M1.5 | Real hardware WebGPU, one Chrome session, base/claim identity, path records and repeat controls permit Chrome regression comparison. Source/bundle identity review can reject cached or substituted output. |
| M1.6 | Command, dispatch, resource and queue-write comparison complements image equality. Adapter-backed bench selection, same-load tripwires and labelled paired measurements make the work/cost claim judgeable. |
| M1.7 | Fresh simulator, lock witnesses, absent indirect capability, actual drawn generation and advancing presented/cooked counters reject undrawn or stalled content. Missing tools and errors fail closed. |
| M1.8 | Current-generation drawing and resumed counters after lifecycle and replacement, with background/lost-queue witnesses, can reject stale publication and lifecycle violations. |
| M1.9 | Both rotations have current extents, surface epochs, retained process/window/lease/device identities and advancing cooked frames. Attended exceptions remain explicit for that row. |
| M1.10 | Named views and native-size crops inspect geometry, textures, facing, materials, Sun shadows, volume and rotated aspect separately. Complete imagery is required alongside counters. |
| M1.11 | Timed samples, edge-entry views, holds and gaps make the stated sampled-continuity conclusion judgeable. Submitted-frame temporal qualification stays assigned to T2 M2. |
| M1.12 | Decision, maturity, evidence and handoff records can be reconciled with actual target results and scratch disposition while retaining separate implementation, correctness, visual and device status. |

Parsed twelve unique item IDs with proofs. A semantic comparison against `1ab6c7fb` confirms that M1.3 is the only
item whose statement or proof definitions changed. Historical evidence/status fields remain provisional inputs for
future qualification against the claimed assembly.

## Validation and scope

Read the project and skill calibration examples, relevant C04/C06/C07/C12 clauses, the hardware-native and G5
decisions, the T2 M2 prerequisite record and dependency, and the scoped source boundaries described above.
The historical checkpoint inspection covered selected records and reproducers. Whole-checkpoint implementation
review, fresh GPU/simulator execution, image perception and motion inspection remain outside this design verdict.
The revised oracle, negative control and remaining target proofs still require implementation and qualification.
No agents were launched.

`./dev check-docs` passes with `RUSTC_WRAPPER=` and the pinned Rust 1.99.1 compiler explicitly selected.
Its existing source-size notes remain informational. Build outputs used an owned target directory under
`/private/tmp`. `git diff --check` passes. The working-tree change is limited to this review file.

DESIGN OK
