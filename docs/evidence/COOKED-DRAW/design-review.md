# COOKED-DRAW design review

9 November 2026. Reviewed integrated tip `dccdbb9b35603ee37c7e570f8298edfd2102db05` using
`orchestrate:review-milestone` (Reviewing a design) and `thrum-engineering`.
No agents launched. No milestone is being judged.

**DESIGN OK. No blocking or advisory findings.** The capability-selected full-range indexed fallback meets the
commission, preserves the existing indirect route, and has discriminating acceptance proofs. The interface-owner
agreements named in the plan remain pending; this review does not grant them.

## Requirements and inspection list

Read the brief before the design, plan and acceptance definitions. Derived these requirements from it:

- Cooked content must draw without indirect execution, selected exclusively from actual adapter facts in the
  capability registry. Unknown facts need a safe outcome.
- Supported Mac and Chrome routes must retain indirect drawing, identical output and unchanged per-frame work.
- Simulator content must be complete, textured, lit and shadowed, with function demonstrated through the profile's
  lifecycle and rotation edges. Simulator observations cannot establish device speed.
- Preserve one owner for capability facts, resources, submission and history. Identify interface changes and the
  producers and consumers that must agree to them.
- Account for repeated cost before implementation, retain measurements and existing tripwires, and introduce no
  invented performance budget.
- Supply the prerequisite for T2-CAPTURE M2 without claiming that sampled simulator imagery proves every submitted
  frame or physical display output.

Used this list to compare the complete COOKED-DRAW design, plan and all 12 done-when items with the scoped source
boundaries below. Read the existing review only after forming this assessment. The three governing COOKED-DRAW
files have no diff from its reviewed revision `2c82de2700e2db8003029ab3cb2702c0c08be86e`; this is a fresh review of
the current integrated tip, not runtime requalification.

## Mechanism and boundary assessment

**Selection and provenance.** `crates/thrum-render/src/capabilities.rs:38` captures an adapter-backed record;
`:48` also exposes adapter-less capture. `examples/v1-game/src/host.rs:344` uses the latter in the public
constructor, and `examples/v1-game/src/content.rs:66` permits cooked attachment to that host. The design explicitly
covers this real route: unknown downlevel facts select direct and never invent indirect support. Adapter-backed
records select indirect only when the flag is present. Reselection at rebuild follows the existing generation
boundary. The diagnostic mask clears the recorded flag and is explicit and evidenced. It does not add a second
selector inside the contribution.

The ordinary app and browser recovery retain adapters when capturing records
(`crates/thrum-app/src/app/device.rs:82`, `crates/thrum-app/src/web/recovery.rs:47`); the iOS route does likewise at
initial preparation and recovery (`examples/v1-game-ios/src/ios/render.rs:409`, `:550`). Migrating the indirect
cooked-test and bench arms away from adapter-less capture prevents a silent change in what they exercise.

**Draw representation and equality hypothesis.** `crates/thrum-asset-gpu/src/load.rs:601` expands meshlet indices
in meshlet order and records contiguous ranges. `draw.rs:573` binds the shared material, vertex streams, instance
and index buffer before indirect drawing. The cull shader writes zero base vertex and first instance. Drawing the
whole range with base vertex zero and one instance therefore fits the existing representation without new products,
CPU meshlet traversal or delayed visibility readback. Sun casters already use a full-range direct draw
(`draw.rs:567`).

The parity argument is appropriate within the admitted placement domain: `placement.rs:156` rejects non-rigid,
nonuniform and mirrored runtime transforms, while source-node mirror correction belongs to the cooker
(`crates/thrum-cook/src/mesh.rs:163`). The mesh pipeline back-face culls, and the indirect shader rejects frustum
bounds and normal cones. Whether that existing culler is conservative in the actual diagnostic and valley views
remains a hypothesis checked by M1.3. The explicit stop rule preserves an adverse result rather than permitting a
culler change or relaxed oracle to hide it. Equal pixels are not being claimed by this design review.

**State and execution.** `crates/thrum-render/src/synthetic.rs:758` checks `computes()` before beginning contribution
compute and calls `encode_after_compute` independently at `:766`. Thus the seam supports the proposed no-compute
fallback. Current history writes are inside `draw.rs:529`'s cull routine and must move into common work; counter
copy/clear handling at `lib.rs:752` must become path-dependent. The design explicitly requires both, along with
shared tentative/submitted state and material promotion. M1.2 tests discard, capture without history, restaging,
remote feeds, material promotion and casters under both paths. A still-image-only implementation cannot satisfy it.

The proposed record and surface additions preserve C04's single capability authority and C06/C07 ownership and
history semantics. The design names render/registry, G5, game and iOS-shell owners and affected consumers. The plan
requires agreement requests before building, permits building against the proposals, and returns objections for
replanning. The synthetic renderer's separate unselected indirect paths are explicitly deferred to G3c rather than
silently presented as fixed by this module.

**Simulator and capture.** Existing `attach_cooked_scene` selects valley or diagnostic content from the named
environment variables and leaves unset content alone. Attaching through it before handing the host to the iOS
render owner is consistent with the existing seam. `examples/v1-game-ios/ios/route.py:535` supplies the specified
landscape/portrait row. The new cooked generation, path and submitted-count witnesses extend lifecycle and rotation
proof to the attached content. Sampled pan claims are explicitly limited by times, holds and maximum sample gap.
T2-CAPTURE M2 retains exact submitted-frame temporal proof, and its done-when file contains
`needs: ["COOKED-DRAW:M1"]`.

**Cost and scope.** The account identifies per-placement uniforms, argument storage, dispatches, command counts,
visibility readback and all-index vertex work, with representative valley counts. It removes CPU/GPU round trips
rather than adding them to compensate for absent indirect execution. Both paths retain shared fragment, shadow and
volume work. Paired, order-alternated Mac measurements and the existing G5 tripwires are appropriate for this
functional fallback; the design does not use them to promote a different selector or claim simulator speed. The
simulator-first order exposes another unsupported feature before investing in the complete parity harness.

## Acceptance proof assessment

These assess the planned proofs, not implementation verdicts. The proposed commands are implementation deliverables.

| Item | Judgeability and convincing-fake coverage |
| --- | --- |
| M1.1 | Adapter-backed, masked and unknown-fact cases, public-constructor attachment and generation reselection are judgeable. Source tracing rejects platform switches, assumed support and private probes; resource/command inspection rejects indirect work hidden behind a direct label. |
| M1.2 | Both suite arms assert their selected path and path-specific counts. Stateful discard, history, recovery, feed, material and caster cases reject a happy-path-only fallback. Allocation and recording inspection cover the no-indirect/no-compute claim. |
| M1.3 | One build/store, varied views, actual path records and deterministic rerun controls support an exact final-output oracle. A mismatch or nondeterministic control cannot pass. |
| M1.4 | Pinned pre-implementation base, shared cooked store, identical views, visible counts and repeat control can establish Mac before/after parity. Reviewer must check that separate base and claim binaries actually execute. |
| M1.5 | Real hardware WebGPU, same-session base/claim runs, path records and repeat controls can establish Chrome parity. A browser endpoint returning the same cached output for both builds would fail source/bundle identity review. |
| M1.6 | Indirect command/write/resource comparison complements pixel equality. An adapter-backed bench must retain indirect selection and same-load history. Paired timing records must identify load and order; no compute pass on direct must be reported as absent. |
| M1.7 | Fresh simulator, lock witnesses, drawn generation, advancing presented/cooked submitted counts and fail-closed tooling reject a loaded-but-undrawn or stalled scene. |
| M1.8 | Lifecycle and device replacement require resumed current-generation drawing and reject background submissions and stale publication. |
| M1.9 | Current extents, surface epochs, retained identities and advancing cooked frames make both orientation transitions checkable. Any attended gesture exception remains separate from locked engine-state rows. |
| M1.10 | Native-size named views and crops separately inspect geometry, textures, facing, material response, lighting, shadows, volume and rotated aspect. Counters alone cannot satisfy this proof. |
| M1.11 | Sample times, gaps, holds and edge-entry samples support only the stated sampled continuity conclusion. Every-submitted-frame or cadence claims remain outside this proof. |
| M1.12 | Decision, maturity ledger, evidence and T2 handoff can be checked against the implementation and actual target records, keeping maturity dimensions and storage closeout separate. |

## Validation and limits

Read project and skill calibration examples, relevant C04/C06/C07/C12 clauses, the hardware-native and G5 decisions,
the retained T2 M2 blocker and dependency, and scoped capability, content, contribution, draw, loader, placement,
material, cooker and iOS-route code. This was not a whole-engine source review. No images or motion were judged,
no GPU command was executed, and no implementation, visual or performance result is established.

The done-when JSON parses with 12 unique open item IDs. `./dev check-docs` passes with `RUSTC_WRAPPER=`
and the installed Rust 1.99.1 compiler explicitly selected (the inherited-wrapper invocation exited 2 without
diagnostics). `git diff --check` passes. Only this review file is changed; acceptance definitions and milestone
state remain untouched.

DESIGN OK
