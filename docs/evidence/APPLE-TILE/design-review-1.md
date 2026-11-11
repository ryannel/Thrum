# APPLE-TILE scoped design review

9 November 2026. Reviewed integrated revision `2514d8240876d2ca6772698cfe6f451332fc48b4` using
`orchestrate:review-milestone`, “Reviewing a design”. **DESIGN OK: no blocking findings; two verified advisory
wording/status findings below.** This approves the scoped design for building, not implementation, visual quality,
performance, milestone completion or unpark. The loop owns those transitions.

## Scope and requirements

I read the [brief](../../design/modules/APPLE-TILE-BRIEF.md) before comparing the design and criteria. Its required
capabilities are:

1. Preserve the complete cooked scene, lighting, mirrors and temporal behavior through movement, cuts, skipped views,
   slot reuse, resize and replacement on the supported targets.
2. Select hardware paths through one generation registry, with readiness, prepared fallback and retirement under the
   existing owners.
3. Derive attachment storage from actual live readers, including capture and reconstruction, without confusing logical
   traffic, resident allocation and physical bandwidth.
4. Preserve safe queue-ordered uploads, admitted instance values/normals, existing light overflow and V2 raster/order
   semantics. This commission need not replace their representations.
5. Supply G4 with a replaceable pre-tone linear input and an isolated native feature boundary: graph resources,
   prepared twin, reset, completion, error and timing semantics under one coordinator and queue.
6. Prove correctness, lifecycle and visible behavior independently; qualify promotion with measured complete costs.
   Keep simulator function, engine motion, actual-window function, panel cadence and physical-device evidence separate.
7. Retain the creator's cook/contribution seam, preserve shared G5 work and resolve prerequisites without a reverse
   dependency on G4 or Mac cadence.

The review covers the complete [design](../../design/modules/APPLE-TILE.md),
[plan](../../design/modules/APPLE-TILE-PLAN.md) and all 33 items in the
[done-when file](../../design/modules/APPLE-TILE-DONE.json), C04/C06/C07/C08/C12, the Metal boundary and hardware-native
decisions, and relevant G4, Mac, T2 and COOKED-DRAW consumer definitions. I also inspected the current capability
record, graph recording, scene declarations, tone/present shaders and bindings, FrameHost and cancellation tokens,
the moving-route tool, capture runbook and locked wgpu 30.0.1 core/HAL sources. This is a reconciliation review with
existing code in view. I read the previous review after forming my assessment of the revised scope. No agents were
launched.

Reviewed artifact SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| APPLE-TILE-BRIEF.md | `f8b69b720d1d1729627eda41600a29eac58a866d726e8664f741116fef11a8a7` |
| APPLE-TILE.md | `d90de51acfddeb2fda00e9b763dc5b13be3d0f5ddedff520a78f24e069bc1e4f` |
| APPLE-TILE-PLAN.md | `95aa73f6bf0a748e9a55c6d743db2b758323f2c39498beab87c7b1e2c5ae8d34` |
| APPLE-TILE-DONE.json | `15f4dd380ce274beffa15ec92c18588eefdb3bf44439bde4001316146888e2e2` |

## Findings, ranked

### 1. Advisory, verified: D12.5 misnames the native handoff

**Location:** `docs/design/modules/APPLE-TILE-DONE.json:420`; compare
`docs/design/modules/APPLE-PRESENTATION-PLAN.md:118`, `:121` and `:130`.

**Evidence:** D12.5 asks to confirm that “G4 M11 native handoff” does not transitively need the final cooked-simulator
item. The consumer plan instead identifies **G3t M11 → G4 M8** as the native handoff. **G4 M11** is the final paired
proof and explicitly needs G3t M12, so that particular confirmation would be false if read literally.

**Consequence:** A later reviewer could confuse early native-boundary availability with final G4 qualification.
This is advisory because the item statement concerns simulator function, and the producer design, dependency
sequence and consumer handoff table consistently identify the intended boundary. It does not introduce an actual
cycle or place cooked simulator work on the native-handoff path.

**Resolution:** Name G3t M11/G4 M8 in that question and retain G4 M11's explicit dependency on final G3t M12. Do not
remove the final qualification dependency to make the mistaken name true.

### 2. Advisory, verified: one maturity scope retains the removed simulator dependency

**Location:** `docs/design/maturity.json:524`, the frame-runtime scope; compare its G3t scope at `:583`,
`docs/design/modules/PLATFORM-RUNTIME-MAC-DONE.json:119` and APPLE-TILE's retained `needs` entries.

**Evidence:** The frame-runtime scope still says Mac M3 is transitively blocked on T2 “Chrome M1/simulator M2”.
The revised APPLE-TILE chain needs delivered T2 M0/M1, uses diagnostic simulator function before M12 and has no
T2 simulator M2 dependency. The G3t ledger entry and active plans correctly describe the revised chain.

**Consequence:** Status readers can infer an obsolete blocker. This is advisory because the executable dependency
records and owning plans agree; no capability is falsely promoted and no prerequisite is missing from the build
sequence.

**Resolution:** Align that frame-runtime scope sentence with the current chain, leaving final cooked simulator function
under COOKED-DRAW M1 and stronger simulator capture under T2 M2. This review changes neither record.

## Mechanism and boundary assessment

**Scope is now proportionate to the brief.** Deferring fused present, pointer uploads/duplicates, scratch sharing,
compact rows, expanded GPU ranking and mirror-range consolidation removes the unsupported exactness premises and
uncommissioned classifier refactor. It does not omit a required capability: existing upload, instance, lighting and
visibility semantics remain mandatory. No deferred optimization is credited as delivered or as a saving.

**Storage has a usable decision rule.** Eligibility is per compiled variant, excludes explicit stores and later readers,
and respects attachment-only usage restrictions. The public
[wgpu 30.0.1 transient-attachment contract](https://docs.rs/wgpu/30.0.1/wgpu/struct.TextureUsages.html#associatedconstant.TRANSIENT_ATTACHMENT)
requires clear/dont-care and discard, consistent with the design. The locked Metal allocator selects Memoryless for
supported transient textures (`wgpu-hal-30.0.1/src/metal/device.rs:561`). The plan requires actual storage-mode
inspection and mutation tests rather than inferring success from a recipe label. Stored star/capture variants and
old/new generations are counted separately; a resident-memory win is not presumed.

**The scene/output split preserves a concrete frame path.** Current scene declarations already express forward colour,
exposure, tone output and readback; current SurfacePresenter separately samples the encoded Rgba8 intermediate.
M2 joins those declarations and partitions only at the surface dependency. M3 changes the bound linear source,
carrying exposure/sample/extent identity, while retaining `synthetic_display.wgsl`, Rgba8Unorm quantization and both
`present.wgsl` format branches. This is compatible with G4's documented TAAU/MetalFX insertion and later EDR branch.
It no longer requires reproducing cross-shader arithmetic or hardware quantization in a fused shader.

**Normal and adverse execution compose with existing owners.** A committed sample is reconciled and encoded before
late acquisition; successful submission alone accepts image history. A healthy refused drawable discards scene work
and flushes pending queue writes; observed loss abandons without that queue call. These are distinct existing token
paths in `crates/thrum-render/src/runtime/tokens.rs`, not a newly invented submit-without-present policy. Native
provider reset joins discard, cut, failure and incompatible generation replacement. The iOS foreground lease still
authorizes submission, and old-generation completion cannot publish live state.

**The native segment is a plausible, bounded handoff, with proof still owed.** The public
[raw-encoder safety contract](https://docs.rs/wgpu/30.0.1/wgpu/struct.CommandEncoder.html#method.as_hal_mut) keeps
command-buffer finishing with wgpu. Locked core `command/mod.rs` also rejects mixing Raw/Wgpu encoding on one
encoder. A dedicated whole raw encoder between finished ordinary encoders addresses both constraints. Declared
resources, wgpu initialization, tracked same-queue resources and generation retention supply the intended ordering
and lifetime mechanism. The first M10 step must qualify it on the locked stack before route integration or MetalFX.
This review does not claim that experiment has run.

The replacement witness avoids the prior floating-point feasibility premise: scene/history regions use copies,
while only a reserved binary-exact tile uses integer selection and storage writes. Distinct preinitialized histories,
submission-selected state, exact twin readbacks and explicit no-op/stale/reset controls can discriminate native work
from a concealed portable producer. M10/M11 must still demonstrate the actual command path, initialization, held-slot
retention, command-buffer errors, replacement and reset. The witness is composition proof, not a MetalFX or speed
qualification.

## Criteria and proof assessment

All retained items have an identifiable observation or code/contract question. The following is an assessment of their
**definitions**, not milestone verdicts or a claim that their checks have passed.

| Items | Judgeability and convincing-fake protection |
| --- | --- |
| D0.1, D0.2 | Real cooked route and full-resolution pre-change linear/SDR controls, deterministic identity and separate realtime motion; phase, contribution, weak-light, mirror and star omissions must produce nonzero exact diffs. The unchanged two-pass SDR/sRGB/letterbox control makes later comparisons concrete. |
| D0.3, D0.4 | A runnable creation-counter check and production-coverage question distinguish first-use creation from net live counts. The replay-probe question is explicitly historical, per-encoder and method-only, with absent scratch and measurement limits disclosed. |
| D0.5 | Chrome endpoint smoke is labelled as such; a retained same-target submitted baseline is separately required before renderer changes. It cannot qualify temporal motion through coarse cross-target agreement. |
| D1.1, D1.2 | Selection/refusal/unknown-fact cases, graph-reader mutations and actual Metal storage inspection expose false Apple labels, unsupported force overrides and transient resources with later readers. |
| D1.3, D1.4, D1.5 | Native paired/pre-change bytes, Chrome's own exact baseline and wasm dependency boundary, plus simulator actual-fact/function observations cover distinct target claims. Prepared-variant switches and resident totals expose lazy creation and hidden fallback allocations. |
| D2.1, D2.2 | Graph partition/order questions and separate healthy-refusal, loss and successful-submit witnesses reject early acquisition, accidental scene submission and lost-device flushes. |
| D2.3, D2.4, D2.5 | Exact native checkpoints and separate motion, simulator background/lease recovery, and required OS-window observations cover rendering, lifecycle and composited function separately. The attended grant and matched-state reference protocol are explicit. |
| D3.1, D3.2 | Trace the replaceable colour/exposure/identity binding and all graph edges; exact identity rebinding against both SDR surface branches and non-1:1 letterboxing rejects tone duplication, quantization changes and source-coordinate dither errors. Actual alternate-input binding must be established by the D3.1 code trace; identity pixels alone are insufficient. |
| D3.3, D3.4, D3.5 | Optional-pass timing/schema tracing covers every platform consumer; Chrome exact checkpoints/motion and fresh actual-window observations prevent Mac-only or offscreen-only closure. |
| D10.1, D10.2 | Whole-encoder API separation, an early exact same-submit fixture and the stateful native/twin route witness expose no-op native work, hidden WGSL production and stale source/history. Failure returns to design review. |
| D10.3, D10.4 | Held submissions, acquire refusal, failed submission, replacement/loss and iOS background races cover completion ownership and reset rather than only the successful path. |
| D11.1, D11.2, D11.3 | Bounded replacement fixtures, rejected preparation/provider, command errors, exact recovery controls and completion-tagged native timing expose premature retirement, stale generation publication and invented/cross-clock timing. |
| D12.1, D12.2 | Complete CPU/GPU/validation/fallback and unique resident costs, capture-OFF baselines, observer arms, A/A and alternating runs, representative scaling and sustained thermal records prevent promotion on omitted work or hidden duplicates. An inconclusive candidate can remain unpromoted. |
| D12.3, D12.4 | Final native/window and Chrome pre-change comparisons retain exact ordinary bytes, omission controls and normal-speed motion. Existing CPU light overflow must be counted honestly; it does not satisfy a new GPU-overflow claim. |
| D12.5, D12.6 | COOKED-DRAW-owned simulator function and final checked-revision/integration questions preserve the downstream owner and separate maturity dimensions. D12.5's handoff name has advisory finding 1; its function requirement remains judgeable. |

## Dependencies, earlier findings and limits

The retained order is M0 → M1 → M2 → M3 → M10 → M11 → M12. M0 needs delivered T2 Mac M0; M1 needs delivered T2
Chrome M1; M2 names the delivered Mac M1 loss boundary. No retained milestone needs T2 simulator M2. Only M12
needs COOKED-DRAW M1 for cooked simulator function. Mac M2 remains independent; Mac M3 needs Mac M2 and G3t M3,
including the attended M2/M3 window proofs and diagnostic simulator rows. G3t does not wait back on Mac M2/M3.
G4 reconstruction consumes M3, and its native provider consumes G3t M11 without waiting for G3t M12. G4's final
paired proof deliberately still consumes M12. I found no cycle in these relevant dependency paths.

The seven findings in [review 2](design-review-2.md) are substantively answered: unsupported arithmetic mechanisms
and M9 are removed; actual-window observer/grants are named; CPU overflow is an explicit retained outcome; the Mac
critical path is split; duplicate-upload work is deferred with its pointer candidate; and exactness is stated for
retained ordinary image items. The two advisory inconsistencies above do not reverse those decisions.

I ran `RUSTC_WRAPPER= ./dev check-docs`; repository checks passed before writing this report, including 595 Markdown
files, 2595 local links, 30 module records, crate policy and current design hashes. The final report is checked again
before handoff. The wrapper override avoids the configured external compiler cache; it bypasses no repository check.
No GPU, browser, simulator, OS-window, motion, Instruments or performance experiment was run for this design review;
no images or videos were judged. Future milestone reviewers must execute the specified target proofs and preserve
unavailable results rather than inheriting a behavioral pass from this document. Only this requested review file is
changed; definitions, acceptance records, milestone states and source code are untouched.

DESIGN OK
