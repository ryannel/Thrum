# APPLE-TILE design review

11 November 2026. Reviewed integrated revision `8d8bf889ed704befbea47ec5393ae0238c68f08c`
using `orchestrate:review-milestone` 1.7.1, “Reviewing a design”, and the Thrum engineering
review rules. **No blocking findings.** The M0/M10 scope correction remains sound, and the
subsequent document migration preserves the reviewed mechanisms and all 33 item definitions.
This approves the design; it does not independently accept implementation, images, performance
or the already recorded M0/M1 milestone results.

## Previous review and findings

The previous `design-review.md` reviewed `019f849d519ba0978f31971d55da1780888023bb` and
contained no Must Fix findings. Its three Suggestions have these dispositions:

- **Exemption from review of changed criteria:** removed from the active plan. The incorrect
  sentence survives at `docs/evidence/APPLE-TILE/replan-handoff-2026-11-11.md:38`, inside an
  explicitly historical handoff. The prior review and this review actually inspect the criteria.
  It supplies no exemption for future definition changes. The active-plan concern is resolved;
  a historical correction would still be useful.
- **D12.5 misnames the native handoff:** still present; carried below.
- **Frame-runtime maturity scope names an obsolete simulator dependency:** still present;
  carried below.

### 1. Suggestion, verified: D12.5 still misnames the native handoff

**Location:** `docs/plans/APPLE-TILE/DONE.json:420`, D12.5's `ask`;
`docs/plans/APPLE-PRESENTATION/PLAN.md:76` and `:92`.

D12.5 says “G4 M11 native handoff” must not transitively need APPLE-TILE's final cooked
simulator item. The actual native handoff is **APPLE-TILE M11 → APPLE-PRESENTATION M8**.
APPLE-PRESENTATION M11 is the final paired proof and deliberately depends on APPLE-TILE M12.
Correct the name in the question while preserving that final dependency. Otherwise a literal
reading asks the reviewer to disprove an intended dependency.

This remains advisory: the producer design, consumer plan and module list agree on the actual
sequence, and the item is judgeable using that sequence. No dependency changed in this revision.

### 2. Suggestion, verified: the frame-runtime maturity scope retains an obsolete dependency

**Location:** `docs/design/maturity.json:528`; compare
`docs/plans/APPLE-TILE/DESIGN.md:328` and the G3t scope at `docs/design/maturity.json:590`.

The frame-runtime summary still makes Mac M3 transitively dependent on T2 “Chrome M1/simulator
M2”. The design explicitly excludes T2 simulator M2 and COOKED-DRAW M1 from that chain.
Reconcile the summary with the active dependency definitions, retaining the final cooked
simulator work in APPLE-TILE M12 and T2 M2's separate submitted-motion qualification.

This is an inconsistent status summary, not a cycle in the design. Related historical claims
that APPLE-TILE remains parked/unbuilt should not override the loop's acceptance records.
No implementation or device promotion follows from correcting those descriptions.

## Request and scope

I read every entry in `docs/plans/APPLE-TILE/ACCEPTED.json` under `briefs`, oldest first.
There is one entry, dated 11 November 2026:

> Resolve APPLE-TILE M0's scope conflict: the builder blocked because done-when item D0.2 requires a native witness that the plan assigns to M10 (see the M0 blocked note and docs/evidence/render/apple-tile-m0/README.md at 03bc878a). Make M0's criteria and M10's consistent so M0 can be claimed on what it can prove, without weakening what V1 needs.

I derived the required result from this request, the project brief's V1 exit/G3t boundary and
the module brief before evaluating the design: M0 must establish the ordinary renderer's
comparison baseline without requiring an unbuilt native provider; M10 must still establish
real native composition and history behavior before G4 consumes it. The wider module must
preserve the cooked scene, lighting, mirrors, accepted history, lifecycle and portable fallback,
while deriving storage from actual readers and providing one generation-owned native boundary.
Hardware performance and physical-display qualifications remain separate from functional proof.

The review covers the complete current design and plan, all 33 done-when items, C04/C06/C07/C08/C12,
the Metal boundary and hardware-native decisions, and the relevant architecture and G4/CREATOR
consumer sections. There is no plans glossary. I read the project's review calibration and the
skill's starting graded examples. I used prompting-agents for actionable finding text, and
launched no agents.

## Revision comparison

A structured comparison against the previously reviewed revision's
`docs/design/modules/APPLE-TILE-DONE.json` found **no definition changes**: no items added or
removed, and no changes to statements, questions, inspections, commands, targets, quality bars,
attended requirements or dependencies. Status, evidence and review records were excluded from
that comparison because they record milestone execution rather than criteria.

The design's original mechanism text differs only in links and the reference to the relocated
handoff. Its appended “Sequence and dependencies” and “Proof and builder handoff” sections were
compared with the old plan: migration notices, links and removal of historical status words
change; mechanisms, sequence and proof obligations do not. The new plan is a compact work list
that points back to those sections and to DONE.json. Approval therefore does not rely merely on
an unchanged design hash or on an earlier verdict.

## Design and proof assessment

Before inspecting the design, I used this checklist: preserve the full ordinary comparison;
make the native proof achievable at its own milestone; reject omitted work and false native
production; preserve single ownership through refusal/loss/replacement; and trace downstream
consumers without a dependency cycle.

**The requested correction is complete.** D0.2 expressly excludes the native witness and names
D10.1/D10.2 as its owner. It still requires exact same-target portable repeats and retained
full-resolution scene-linear/submitted SDR baselines. Its question retains build/compiler,
shader, cooked-content, sample, exposure, format and H=1 identity, both surface-format branches,
non-1:1 extents and letterboxing. Phase mismatch and omitted contribution, overflowing-cluster
light, mirror and night stars must fail the comparison. M0 cannot pass on coarse image grids
or silently replaced baselines.

**Native work remains mandatory.** D10.1 makes the small same-submit copy/select fixture the
first implementation step, before route integration. D10.2 requires actual native writes,
exact output/history readbacks against the prepared twin, distinct initialized history textures,
submission-selected state and reset/replacement proof. No-op work, hidden WGSL production,
stale history/input and omitted reset must fail. D10.3/D10.4 and M11 add held-resource,
discard/failure, loss, iOS lease and replacement proofs. Ordinary runs disable the diagnostic
and retain the pre-change baseline; its altered crop never becomes the ordinary reference.

**The mechanism respects the boundary.** Separate whole raw and wgpu encoders share one ordered
coordinator submission; native work cannot touch the surface or own a queue. C06 and the design
agree on graph-declared accesses, initialization, retention and reset. I checked the installed,
locked wgpu 30.0.1 source: `wgpu/src/api/command_encoder.rs:270` forbids ending the raw command
buffer or overlapping access, and `wgpu-core/src/command/mod.rs:543` rejects mixing Raw/Wgpu
recording on one encoder. The design respects those restrictions. Actual native ordering and
copy/storage correctness remain the explicit M10 feasibility proof, not a claim established
by this document review.

Each group of items can be judged with its predecessors and stated external prerequisites:

| Items read | Judgeability and resistance to false success |
| --- | --- |
| D0.1–D0.5 | Existing cooked route and T2 Mac capture support the baseline; ordinary omission controls and creation attribution discriminate; the traffic probe is explicitly historical method evidence; Chrome endpoint smoke is not allowed to stand in for submitted motion. No native implementation is required here. |
| D1.1–D1.5 | Registry refusal tests, actual Metal storage inspection and later-reader mutations distinguish genuine eligibility from a recipe label. Paired and retained baselines catch shared regressions. Chrome uses T2 M1; simulator function needs neither cooked draw nor T2 M2. |
| D2.1–D2.5 | Graph order and phase binding, healthy refusal versus loss, stale-generation retirement, exact images, iOS lease races and actual OS-window function cover the scheduling change. Attended window setup is explicit and does not depend back on Mac M3. |
| D3.1–D3.5 | Resource/identity tracing and exact identity substitution test the pre-tone seam while retaining both shaders and the SDR intermediate. Optional-pass tests and consumer tracing discriminate pass-name timing from positional relabeling. Chrome and real-window checks remain required. |
| D10.1–D10.4 | The first native fixture and exact stateful twin precede route integration. Negative controls, held completion, discarded work and background races reject a fake native path or premature history publication. |
| D11.1–D11.3 | Bounded provider extents make replacement, preparation failure and stale completion testable before G4 live scaling. Recovery is compared with uninterrupted controls; native timing requires a real completed row and explicit clock treatment. |
| D12.1–D12.6 | Integrated paired workloads include resident twins, validation/overflow costs, thermal duration, capture-OFF timing and observer arms. Final Mac window, Chrome omission controls and cooked simulator lifecycle remain mandatory. Missing speed evidence leaves a candidate unpromoted; it does not erase functional requirements. The D12.5 naming correction is advisory as explained above. |

**Consumer order remains consistent.** G4 receives pre-tone input at APPLE-TILE M3 and native
providers at M11. Its M8 consumes the latter; its final M11 waits for APPLE-TILE M12. Mac M3
waits for APPLE-TILE M3 and its own M2, with no reverse edge. Only APPLE-TILE M12 adds the cooked
simulator prerequisite. CREATOR's profiler consumes M3 pass names; the architecture explicitly
assigns its later timing-demand switch to CREATOR M2 without changing APPLE-TILE's criteria.
The 2D scene/interface asks are explicitly future milestones when track 3 starts
(`docs/plans/ARCHITECTURE.md:434`), not omissions from this V1 commission.

The cost account preserves the SDR intermediate, upload layout and existing CPU light overflow;
it counts selected/fallback resources and generation overlap. Deferred fusion, pointer uploads,
compact rows, expanded GPU ranking and range consolidation are neither required nor claimed.
Performance promotion requires comparable whole-frame and sustained thermal evidence. This
preserves the brief's distinction between functional delivery and a measured hardware win.

## Verification and limits

`RUSTC_WRAPPER= ./dev check-docs` passed: 666 Markdown files, 3220 local links, 30 module records,
crate policy and current design hashes. It passed both before and after writing this report.
Repository checks validate documentation consistency, not GPU behavior.

No GPU, browser, simulator, OS-window or performance run was performed, and no images or motion
were judged. This is a design review, not a re-review of M0/M1 implementation. No captures were
needed. Only `docs/evidence/APPLE-TILE/design-review.md` was edited; DONE.json, acceptance records
and source files remain untouched. No commit was made; integration belongs to the loop.

Reviewed design SHA-256: `062dea2d6dd62734467d48c202f10523e348f62830d7a24ac3f2f16df1c73be5`.
Reviewed DONE.json SHA-256: `789d5573be075b79831ca5e6d6c3c7e8a6f664314c61996799db272b3c852233`.

DESIGN OK
