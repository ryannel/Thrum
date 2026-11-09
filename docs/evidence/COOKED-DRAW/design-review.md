# COOKED-DRAW design review

9 November 2026. Reviewed integrated revision `2c82de2700e2db8003029ab3cb2702c0c08be86e`.
Independent design review using `orchestrate:review-milestone` and `thrum-engineering`. No agents launched.
Read the [brief](../../design/modules/COOKED-DRAW-BRIEF.md) before the
[design](../../design/modules/COOKED-DRAW.md), [plan](../../design/modules/COOKED-DRAW-PLAN.md) and
[done-when file](../../design/modules/COOKED-DRAW-DONE.json). Subsequently read the previous review and compared
its reviewed revision, `82fa10ac`, with this revision. No milestone is being judged.

**DESIGN OK. No blocking or advisory findings.** The revised selection rule closes the unrestricted adapter-less
host route, and the simulator proofs now have explicit sampling limits and cooked rotation coverage. The design
can proceed through its stated owner-agreement process to implementation. Those agreements remain pending.

## Requirements derived from the brief

The following inspection list governed the document and source comparison.

- Render existing cooked content without indirect execution, selecting from actual adapter facts held by the
  capability registry. Unknown facts must have a safe outcome.
- Keep indirect execution selected on the supported Mac and Chrome routes, preserving output and per-frame work.
- Establish simulator function through complete textured, lit and shadowed content, with the profile's rotation
  and background/foreground edges. Keep simulator observations separate from device performance claims.
- Preserve the contribution seam, submitted motion history, generation-bound resource lifetime and evidence
  meaning. Identify changed interfaces, their producers and consumers, and required owner agreements.
- Account for repeated work and memory before implementation, measure under recorded conditions, and use the
  existing tripwires without inventing a budget.
- Deliver the prerequisite that lets T2-CAPTURE M2 resume, while retaining its responsibility for stronger
  submitted-frame capture proof.

This review inspected documents and scoped source boundaries. It inspected no new images or motion and establishes
no implementation, visual or performance result.

## Resolution of the previous findings

All three earlier findings are resolved in the design and acceptance definitions. These are verified document
changes, with their implementation still to be proved.

| Earlier finding | Resolution in this revision |
| --- | --- |
| F1, blocking, adapter-less capture asserted unsupported indirect execution | `COOKED-DRAW.md:51–73` permits indirect only from adapter-read flags and makes unknown facts select direct. M1.1 explicitly tests `FrameHost::new` through cooked attachment, including absence of indirect resources and commands. M1.2 and M1.6 migrate the indirect suite and bench arms to adapter-backed capture and require their selected path to be recorded. |
| F2, advisory, sparse simulator samples could not establish absence of one-frame holes | `COOKED-DRAW.md:133–143` and M1.11 constrain the conclusion to sampled frames and require presented-frame counts, sample times, holds and the largest gap. Complete submitted-frame temporal proof remains with T2 M2. M1.7 and M1.8 also require presented and cooked submitted counters to advance. |
| F3, advisory, cooked rotation coverage was absent | M1.9 now requires portrait → landscape → portrait through the existing XCUITest route, preserving process, window and device identity while presenting at each current extent. M1.10 inspects the rotated content. The plan separates any attended gesture reason from the locked engine-state rows. |

## Mechanism and composition

The existing public constructor is unrestricted. `examples/v1-game/src/host.rs:339` builds through
`DeviceCapabilities::capture_device`, and `examples/v1-game/src/content.rs:66` accepts that host for cooked
attachment. The new conservative rule therefore covers a real entry point. It also explicitly acknowledges the
extra vertex work paid by adapter-less callers on capable hardware. The ordinary Mac, Chrome and iOS platform
routes retain adapters when capturing their records, so the design preserves supported-device indirect selection.
The device-generation rebuild is the right place to reselect the path.

The proposed full-range draw fits the existing representation. `crates/thrum-asset-gpu/src/load.rs:601` expands
meshlet indices contiguously and records each range in that order. `draw.rs:573` binds the material, both vertex
streams, instance and index buffer before indirect drawing. A full-range indexed draw with zero base vertex and
one instance can reuse those inputs. `cull.wgsl` writes zero base vertex and first instance for the indirect
commands. Existing Sun casters already draw the full range directly at `draw.rs:567`.

The pixel-parity argument depends on conservative frustum and cone rejection, the mesh pipeline's back-face
culling (`material.rs:322`) and the admitted positive uniform-scale placement policy (`placement.rs`). The design
backs that argument with exact comparisons, including mirrored source geometry and an edge-straddling mesh.
Its stop rule preserves a failed oracle if the existing indirect culler rejects visible geometry. Byte equality
remains an M1 hypothesis to test on the real target.

The existing seam can skip compute while retaining the surrounding contribution work.
`crates/thrum-render/src/synthetic.rs:756` consults `computes()` before beginning the pass, then calls
`encode_after_compute` independently. The implementation must maintain history in the common path and condition
counter/readback handling on the selected path. The design and M1.2 require both. In the current source,
`crates/thrum-asset-gpu/src/lib.rs:666–861` separately tracks tentative upload/draw state and commits it in
`submitted`, including capture-without-history and material promotion. The planned two-path suite covers these
stateful cases rather than relying on still-image equality.

The iOS host captures its adapter at preparation and recovery
(`examples/v1-game-ios/src/ios/render.rs:409`, `:550`). Cooked attachment before moving the host into its threaded
owner fits the existing construction. `examples/v1-game-ios/ios/route.py:535` provides the named rotation row,
and its resize checks already witness current extent, surface epoch and retained process/window identity.
The new cooked witnesses and captures extend that row to the newly attached content.

The cost account identifies the relevant exchange. Direct drawing removes meshlet cull dispatches, argument
buffers, cull uniforms and visibility readbacks, and encodes one draw per placement while processing all mesh
indices. It retains the shared lighting, volume and shadow work. The representative valley counts and paired,
order-alternated Mac measurements are suitable for this scoped fallback. M1.6 also requires an indirect command,
write and resource comparison, so equal pixels alone cannot conceal extra work or a bench that changed paths.
There is no performance-based selector change commissioned here.

The contracts table identifies the capability record, surface lending, stats and iOS attachment changes and their
owners. Approval of this design does not supply those owners' agreements. The plan requires the requests before
building, permits work against the proposals, and sends objections back for replanning. The deferred synthetic
indirect-culling gap belongs to G3c and is explicitly recorded. T2-CAPTURE M2's
`needs: ["COOKED-DRAW:M1"]` is present in its done-when file.

## Acceptance proof assessment

These are assessments of planned proof judgeability and coverage. The commands to be implemented are deliverables,
and their current absence is consistent with this unimplemented design.

| Item | Assessment |
| --- | --- |
| M1.1 | Judgeable. Real-adapter, masked and unknown-fact arms, public-constructor attachment, generation reselection and source inspection reject platform switches and invented support. The reviewer must trace the flags from capture through the surface to allocation and recording. |
| M1.2 | Judgeable. Both selected paths must run the cooked GPU suite, with path-specific counts and discard, history, recovery, remote-feed, material and caster cases. Allocation and command inspection reject a nominal direct path that still creates indirect resources or dispatches culling. |
| M1.3 | Judgeable. Separate direct and indirect outputs from one build/store, recorded selections, varied views and same-path reruns provide an exact oracle. A nondeterministic control invalidates the comparison under the plan's stop rule. |
| M1.4 | Judgeable. A pinned pre-implementation base, one cooked store, unchanged visible counts and repeat controls can establish Mac before/after parity. The reviewer must verify baseline identity and the isolation of the claimed module changes. |
| M1.5 | Judgeable. Real hardware WebGPU, selected-path records, matched fixed-time views and same-session rerun controls can establish Chrome before/after parity. Separate base and claim bundles must actually execute. |
| M1.6 | Judgeable. The source comparison covers indirect commands, writes and resources. The adapter-backed bench must assert its path, use the existing same-load tripwires and retain labelled paired measurements. A direct path with no contribution compute pass must report that absence accurately. |
| M1.7 | Judgeable. The fresh simulator, locked-console witnesses, drawn generation and advancing presented/cooked submitted counts reject a loaded-but-undrawn scene or stalled route. Missing target, tool or lock evidence fails the command. |
| M1.8 | Judgeable. Background/foreground and device replacement must restore current-generation cooked drawing and advancing counters, with no forbidden submissions or old-generation publication. Reusing the existing lifecycle witnesses is appropriate. |
| M1.9 | Judgeable. Orientation, extent, epoch and identity witnesses accompany advancing cooked frames after both transitions. Any attended reason is confined to the gesture row and cannot substitute for M1.7 or M1.8. |
| M1.10 | Judgeable. Named native-size views and crops test geometry, material response, facing, lighting, shadows, volume presence and rotated aspect separately. Held-view pairs add a repeat observation. Successful present counters alone cannot satisfy this inspection. |
| M1.11 | Judgeable within its explicitly recorded sampling. Frame-edge entry, pan samples and holds test observed completeness, with times and gaps limiting the conclusion. It does not certify unsampled frames or cadence. |
| M1.12 | Judgeable. The decision record, ledger and handoff can be compared against actual code and target evidence, with separate maturity dimensions and scratch closeout. |

The simulator-first work order resolves remaining compatibility uncertainty before the parity harness and evidence
polish. Its stop rules name a further unsupported feature, nondeterministic parity control or capture access
requirement as outcomes to investigate or route. These are bounded implementation risks with discriminating proof,
and do not require another mechanism to be invented before this design can be built.

## Validation and limits

Read the project and skill graded examples, relevant C04/C06/C07/C12 clauses, G5 and hardware-native decisions,
retained T2 M2 blocker and dependency, and the scoped capability, host, content, contribution, loader, draw, cull,
placement, material, simulator and cooked-test boundaries discussed above. This was a scoped boundary read of
existing code and the complete COOKED-DRAW design, plan and 12 acceptance items. Historical simulator failure was
read from its retained record. Runtime proof remains due in M1.

The done-when JSON parses with 12 uniquely identified open items. `./dev check-docs` passes with
`RUSTC_WRAPPER=` and the installed Rust 1.99.1 compiler explicitly selected. It reports 595 Markdown files,
2,597 local links, 30 module records, crate policy and current design hashes checked. `git diff --check` passes.
Only this review file is changed. No milestone state or acceptance definition is edited.

DESIGN OK
