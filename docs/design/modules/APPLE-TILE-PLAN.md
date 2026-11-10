# Apple tile recipe plan (G3t)

Design: [APPLE-TILE.md](APPLE-TILE.md) · [Brief](APPLE-TILE-BRIEF.md) ·
Done when: [APPLE-TILE-DONE.json](APPLE-TILE-DONE.json) owns criteria and proof status.
[Boundary](../decisions/G3T-METAL-BOUNDARY.md) and C04/C06/C07/C08/C12 govern.
Reviews: [design review 2 at `224b58a1`](../../evidence/APPLE-TILE/design-review-2.md): DESIGN CHANGES (7 findings); independent review of this scoped re-plan is owed to the lead/loop.
Milestone: M0 A repeatable offscreen valley proves the comparison path (open)
Now: M0 builder reran D0.3 on Metal and inspected D0.4's historical replay method. Both pass with retained evidence.
Next: retain native and Chrome pre-change T2 baselines, implement the M0 scene-linear comparison/controls and qualify the complete route.
Blocked: the runtime capture command refused its 16 GiB free-space preflight on this volume (14 GiB initially, 13 GiB during builds). D0.1/D0.2/D0.5 remain open. M2/M3/final attended rows and M12's COOKED-DRAW dependency retain their existing requirements.

## Sequence and dependencies

Retained sequence: **M0 → M1 → M2 → M3 → M10 → M11 → M12**. IDs are kept for consumers. Removed M4–M9
are deferred work, not completed milestones. One builder integrates sequentially; this task launches no agents.
M0 consumes accepted `T2-CAPTURE:M0`; M1 consumes accepted `T2-CAPTURE:M1`. G3t adds scene-linear checkpoints
through the shared tap. M2 also names delivered `PLATFORM-RUNTIME-MAC:M1`. Before M1 changes rendering, retain
Mac and Chrome same-target portable baselines using their qualified target taps and pinned source/cooked inputs.

Mac M2 qualifies the current path after T2 M0 independently. Mac M3 needs Mac M2 by module order and
`APPLE-TILE:M3`: T2 M0/M1 (both done), then G3t M0–M3, including attended D2.5/D3.5 and simulator diagnostic
function checks. **Neither COOKED-DRAW M1 nor T2 simulator M2 is on that chain.** Mac M3 reruns its complete
runtime-motion and capture-OFF built-in-panel cadence protocols plus real-window tap/tooling preflight on the changed
build. G3t never waits back on Mac M2/M3; old results qualify only their recorded builds.

G4 receives M0 proof tools, M1 eligibility, M3 pre-tone linear input/exposure binding and M11 native
provider lifetime/reset/timing. G4 reconstruction inserts before the retained SDR tone stage; G4 owns the later EDR
branch and its display semantics. M11 bounded extent fixtures and M12 fixed extents/current readers do not wait for
G4; G4 later reruns actual reconstruction-reader/scale-transition proofs. Full G4 JSON migration remains its own commission.

M12 alone needs `COOKED-DRAW:M1` for cooked simulator endpoint/function evidence. Earlier simulator rows use the
existing diagnostic scene and `simctl` interim observations, with no submitted/runtime-motion claim. This preserves
T2 rollout rather than waiting on or implementing its blocked M2. G5 owns cooked residency/draw and COOKED-DRAW's
changes to `DeviceCapabilities`, `SceneSurface` and `thrum-asset-gpu/src/draw.rs`. Extend the same registry, preserve
selected direct/indirect paths and path-independent history. No draw.rs classifier/layout/target refactor is
commissioned here; keep any mechanical execution/creation-site adaptation narrow, merge sequentially and rerun
G5's affected parity/history witnesses. G4 seam 3 handles later forward target changes in that file.

## Disposition of second-review findings

| Finding | Resolution in this re-plan | Decisive proof / boundary |
| --- | --- | --- |
| 1 CPU/GPU f32 exactness and fusion feasibility | Cut fusion, compact rows and expanded GPU ranking. Keep locked shader math mode, existing shader arithmetic/entry points and the Rgba8Unorm quantization boundary. G4 receives a pre-tone input seam, not a fused shader. Replace diagnostic blend arithmetic with exact stateful load/select/store. | D0.2 retains full-frame two-pass SDR/sRGB/letterbox controls; D3.1/D3.2 identity rebinding must be exact. D10.1 first proves a small native copy/select fixture before route integration; D10.2 readbacks/twin/negative controls are exact. Future fusion/ranking requires separate feasibility evidence and reviewed semantics, never a widened metric. |
| 2 M9 has no supported draw win / uncommissioned consolidation | Remove M9 and its shared-classifier obligation from C07/RENDERING. Preserve existing V2 policy, physical-slot order and mirror fixture. | No classifier or private bucket migration here. D0.2 omission control and retained recipe/pre-change image/order witnesses reject missing mirrors. V2/G3c owns any future consolidation. |
| 3 No actual-window observer/grant | Name OS game-window capture, tool/process/window identity and Screen Recording permission, with explicit JSON `attended` fields. Compare settled OS window frames to T2 still/fixed references of the same paused-world state; inspect content and resized letterbox geometry. Mac M2.3 keeps same-submission tap qualification. | D2.5/D3.5 and final D12.3 require actual OS-window evidence. Dynamic OS video proves functional response only. No engine-only pass or panel claim; missing setup keeps the item open. |
| 4 Chrome GPU overflow could close on CPU fallback | Expanded GPU builder is removed on both targets. Existing CPU overflow is an explicit retained outcome, never called GPU qualification. | D12.4 requires exact list/readback and pre-change Chrome still/fixed byte comparisons plus T2 realtime motion, actual path counts and omission controls. No GPU-32/64 claim remains. |
| 5 Hidden Mac critical path | Preserve the already-landed Mac M2/M3 split. Remove T2 simulator M2 from G3t M1; earlier simulator checks use diagnostic endpoint/function evidence. M12 alone consumes COOKED-DRAW M1. | Dependency table above, BUILD-PLAN and Mac plan name the full chain and attended rows. Mac M2 waits only on T2 M0; Mac M3 has no simulator cooked/capture dependency. |
| 6 Apple duplicate buffers may survive rejected pointer path | Remove M4/M5. Both recipes retain queue-ordered single buffers; no duplicate/import candidate is built or left resident. | D3.1 rejects deferred optimizations in the seam; D12.1/D12.2 reconcile actual allocations/upload bytes with the unchanged transport baseline. A future pointer candidate must include duplication in its promotion/teardown decision. |
| 7 Exactness missing in item statements | Remove D5.2/D7.3/D8.3 with their mechanisms. Every retained ordinary image item names exact same-target scene-linear and submitted SDR comparisons; native witness is exact to its own twin. | D0.2/D1.3/D1.4/D2.3/D3.1/D3.2/D3.4/D10.2/D11.2/D12.3/D12.4 and the comparison contract; no blend allowance remains. |

## Carried work and state

- D0.3 creation diagnostics remain implemented at `be252c0d` in renderer creation/runtime/capabilities and the cooked
  consumer, but its status stays open with empty evidence until its current-revision command is rerun.
- D0.4 alone retains its builder pass and [traffic-probe evidence](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md).
  It establishes a per-encoder replay method, not a current physical-traffic gain. Missing original bulk scratch is
  not recreated or treated as proof.
- M0 remains `parked`; all other retained milestone states remain `open`, with zero `fails`. Each retained definition
  revision increments `replans` once: M0/M2/M3/M12 to 3, M1 to 2, M10/M11 to 1. Removed M4–M9 have no successor
  acceptance claim. No `*-ACCEPTED.json` is edited; the loop records definitions/proof invalidation and settles unpark.

## Proof and builder handoff

Use [the capture runbook](../../CAPTURE.md) and implemented T2 target adapters. Mac selects `still` or `fixed`
for exact submitted checkpoints; Chrome selects `fixed` (its adapter has no `still` switch). Both select `realtime`
for host-time elapsed engine motion. Add the linear comparison stage
through the same tap. Retain native/Chrome pre-change sources, compiler/shader/content/route/sample/exposure/format
identities and exact full-resolution images. Never silently refresh a baseline. Compare deterministic matching samples;
realtime scheduling alone does not supply byte-matched frame identities. Existing
`python3 scripts/run_v1_showcase.py moving` remains endpoint/no-history route smoke only.

Simulator uses `simctl` endpoint/function capture with recorded coverage and timing limits. No row in this plan
claims simulator exact submitted output or elapsed-time engine motion; those stay with T2 M2. M12's cooked scene
must wait on its owner COOKED-DRAW, never build a substitute. New commands are documented only when implemented.
Run GPU work under `gpu` exclusivity; no GPU or screen automation is part of this planning turn.

D2.5/D3.5/D12.3 declare **attended setup** in JSON: unlocked active Mac session and Screen Recording permission for
the chosen OS capture process. Record actual window/PID/tool and compare settled OS
window evidence to T2 `still`/`fixed` reference images at matching paused-world controls/content/extent. Inspect content
and letterbox in content-pixel coordinates; these are matched states, not the same submission. Mac M2.3 owns separate
windowed-tap integration/correspondence, which this observer does not require or qualify. Dynamic OS video with controls
proves response only. Agent runs/reviews after setup; do not automate grant changes. Missing setup leaves these rows open
while unblocked work proceeds. Mac M3 separately owns attended Instruments/panel qualification with capture OFF.

Use `tmp/g3t-<milestone>/` for scratch and selected `docs/evidence/render/` records under the storage rules. Retain
lossless sources, provenance, exact diffs and 1x views; inspect full frames and native-size crops for Sun, stars,
local lights, shadows, mirrors, cut/skip/history and replacement. Exact bytes do not replace visual review.
Performance baselines use capture OFF, paired capture ON observer-cost arms, A/A and alternating recipes. Any
promotion needs a measured complete-frame win with full fallback/validation/resident costs and comparable sustained
thermal records (at least ten minutes per recipe after warmup, a measurement method rather than an invented budget).
Physical replay counters may remain unmeasured, and physical iPhone/cadence qualifications remain separate.

<a id="m0-proof-route-counters-and-capture-probe-pending"></a>

## M0 A repeatable offscreen valley proves the comparison path: open

Outcome criteria: D0.1–D0.5 in the done-when file.

- [ ] Extend T2 linear comparison and full-resolution controls; retain pre-change sources and rerun creation diagnostics.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M1 The valley selects a recipe and stores only actual future inputs: open

Outcome criteria: D1.1–D1.5 in the done-when file.

- [ ] Add truthful registry/override, per-variant storage and ready fallback; preserve COOKED-DRAW facts/selection. Prove Mac/Chrome exact bytes and simulator diagnostic function.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M2 The existing frame lifecycle runs through one split graph: open

Outcome criteria: D2.1–D2.5 in the done-when file.

- [ ] Merge graph and late surface binding; preserve refusal/loss/background/history rules. Complete attended OS-window matched-state reference comparisons and functional recovery.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M3 G4 can replace the pre-tone linear input without changing SDR output: open

Outcome criteria: D3.1–D3.5 in the done-when file.

- [ ] Expose one colour/exposure input with exact identity substitution; keep tone/intermediate/present arithmetic. Migrate pass-name timing and prove attended window function.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M10 A stateful native segment composes inside one submission: open

Outcome criteria: D10.1–D10.4 in the done-when file.

- [ ] Start with exact same-submit copy/select fixture and twin. Then prove route/history, held lifetime, discard/reset and diagnostic simulator background race.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M11 Native providers survive replacement and expose truthful timing: open

Outcome criteria: D11.1–D11.3 in the done-when file.

- [ ] Use bounded extent fixtures and prepared twins for replacement/rejection/errors, complete generation retention and explicit clock-domain timing.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## M12 An integrated record states qualified wins and remaining targets: open

Outcome criteria: D12.1–D12.6 in the done-when file.

- [ ] Rerun retained mechanisms and full workload, actual final Mac window, exact Chrome baseline, and COOKED-DRAW-dependent simulator function; record sustained costs and owed qualifications.
- [ ] Retain actual target proof and run scoped checks plus `./dev check` before claiming.

## Design review handoff

Fresh independent review is owed to the loop; no agents can be launched in this turn. Start from
[the revised brief](APPLE-TILE-BRIEF.md), derive its capabilities, then read design/JSON and actual consumers.
Check each disposition above, unchanged shader/quantization assumptions, exact native copy/select feasibility,
C06 whole-encoder ordering/initialization, prepared resource/twin cost and discard/reset/loss rules. Check the G4
pre-tone seam in its consumer documents, COOKED-DRAW ownership, target capture modes, explicit window grants and
Mac critical path. Judge whether all proofs reject omitted work. No owner-only decision is required by this scoped
re-plan. Approval, unpark and milestone acceptance belong to the loop; this planner claims none of them.

## Log

- 2026-11-10 M0 builder started from the loop's explicit commission and open JSON state. D0.3's current-revision
  Metal command passed 12 active tests, including pipeline coverage, generation clones, detached attribution and
  the unscoped play guard. D0.4's retained Xcode probe passed inspection with its original limits. Evidence is in
  [the partial M0 record](../../evidence/render/apple-tile-m0/README.md). The runtime capture command refused its
  free-space preflight before allocation. Initial space was 14 GiB against the declared 16 GiB requirement.
  The required moving-route smoke passed all 361 ticks across two native and two Chrome runs, with all five
  checkpoint repeats byte-identical within each target. Its seven Chrome refusal controls passed. This remains
  endpoint/no-history smoke. Documentation checks passed. No renderer/shader change or baseline refresh was made.
  D0.1/D0.2/D0.5 and M0 remain open.

- 2026-11-09 Lead explicitly recommissioned this scoped re-plan after two failed design reviews. Answered all seven
  second-review findings in the disposition table; cut fused present and M4–M9 instead of adding arithmetic or
  ownership prerequisites. Retained SDR shaders/quantization, exposed G4 pre-tone input, and selected an exact native
  copy/select stand-in. Updated producer/consumer documents, capture modes/attended grants and Mac critical path.
  T2 M0/M1 are delivered; earlier simulator rows use diagnostic interim function, final cooked function waits only
  on COOKED-DRAW M1. Preserved every retained milestone state and D0.4 evidence; incremented retained replan counters.
  No runtime/GPU proof, new review, commit, acceptance edit or unpark in this turn. Fresh loop design review is owed.
  Validation: `RUSTC_WRAPPER= ./dev check-docs` passed (595 Markdown files, 2595 local links, 30 module records).
  The wrapper override avoids sandbox-denied sccache; no check was bypassed. Loop schema/planner-delta and dependency
  checks passed; retained states, claims and D0.4 evidence are unchanged, and no acceptance record was touched.

- 2026-11-09 Recorded the lead's park after [design review 2](../../evidence/APPLE-TILE/design-review-2.md)
  returned DESIGN CHANGES (7 findings) at `224b58a1`, following the first failed review. Set M0 `state` to
  `parked`; later milestones remain open but cannot start through sequential ordering. Updated only the
  D2.5/D3.5/D12.3 Mac handoff references from M2 to new Mac M3 and incremented M2/M3/M12 `replans` to 2.
  Preserved other definitions/counters and D0.4 evidence: design-review failures are not failed claimed-milestone reviews. No resolution of
  the seven findings is claimed. Restart requires explicit lead recommission of a scoped re-plan and fresh
  independent design approval before build. Decoupled Mac M2 in its owning plan; new Mac M3 alone carries the
  post-G3t dependency, so T2/Mac current-path work can proceed while this module is parked.

- 2026-11-09 Integration read of upstream `8f65fd0d`: the [T2/Mac evidence design review](../../evidence/T2-CAPTURE/design-review.md)
  requests changes to capture/windowed-tap and cadence proof. Those findings remain a separate re-plan; this Apple
  tile revision neither approves that prerequisite nor claims to resolve its ten findings. Capture milestones and
  Mac qualification stay open; the dependency order here is unchanged by the review-only upstream commit.

- 2026-11-09 Addressed [all nine findings](../../evidence/APPLE-TILE/design-review.md) from the independent
  `74be9f89` review: strengthened Chrome source/dependency/full-resolution baseline proofs; M9 delivers one V2
  range authority shared with private buckets and preserves two-sided/order semantics; per-slot storage is Apple-only
  with portable upload-byte deltas; replaced global perceptual tolerance with exact mechanism-specific comparisons
  and small-omission controls; mandatory GPU overflow qualification; sustained thermal and validation/admission CPU
  rows for promotion; required M2/M3 functional Mac windows with Mac M2 rerun after M3; explicit resident fallback
  allocation accounting and target-specific capture dependencies for M1; reset D0.3 to open pending actual
  current-revision output. Revised definitions/counters in
  one re-plan commit, kept D0.4 and all historical evidence. Independent re-review is owed; no agents launched.

- 2026-11-09 Capture ownership settled in C12/T2-CAPTURE. Added `T2-CAPTURE:M0` to M0 so G3t consumes one
  renderer-owned submitted-output path rather than building a duplicate recorder. M0 stays open with `replans: 1`
  and `fails: 0`; carried item status is unchanged. Scene-linear recipe comparison
  remains G3t work. Independent review of this handoff is owed; no implementation/evidence status changed.

- 2026-11-09 Owner requested the next module while Mac M2 waits for an unlocked display, with no agents. Selected
  G3t from the build order; converted criteria to JSON and reconciled the current consumers. Preserved D0.3/D0.4
  claims and all historical logs; did not inherit milestone completion or scratch-only approvals.
- 2026-11-09 Settled planned producer/consumer contracts before building: retain existing healthy-drop/lost-abandon
  behavior for native work with provider reset; whole raw encoder stays isolated in one submit. Preserve supplied
  normals via private 128-byte transport, scored strongest-32 light overflow and existing V2 determinant authority.
  Removed stale session holds and reverse G4 dependencies by using bounded replacement/fixed-extent proofs; updated
  G4's handoff table. C06/boundary changes need independent design review; no owner-only decision is required.
- 2026-11-09 M0 remains open. Same-native linear-image comparison and complete variants still need implementation;
  existing moving route is retained as smoke proof. Physical traffic rows may stay unmeasured without a screen slot;
  no display or physical-device qualification is promoted. No implementation or other-agent launch in this task.

### Historical reviews (earlier definitions only)

- D0.3 creation-counter seam @ `5550ee34` (landed as `be252c0d`), exact diff and verdicts in
  `tmp/g3t-metal-boundary/NOTES.md`: Thrum Core approved; G5 approved the cooked hunks; G6 agreed the `SceneSurface`
  device type.
- G3t pair @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/g3t-pair.md` (with `common.md`), report
  `tmp/g3t-g4-plans/reviews/g3t-pair-report.md`: changes required (9 required changes). Answered in revision 2;
  see the Log.
- Composed frame across G3t and G4 @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/composed-frame.md`, report
  `tmp/g3t-g4-plans/reviews/composed-frame-report.md`: changes required (14 required changes). Answered in
  revision 2.
- G3t pair re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (N1–N2 required,
  N3–N7 recommended). All answered in revision 3.
- G4 pair re-review @ `c507ad3d` (interfaces only here): approve with changes; see the G4 plan.
- Composed-frame re-review @ `c507ad3d`: approve with changes; its findings fall on G4 M4 and are answered in the G4
  plan's revision 3.
- Composed frame, exact-revision recheck @ `9938b0d0` (Opus 5.5 medium), brief `tmp/g3t-g4-plans/reviews/composed-frame.md`,
  report `tmp/g3t-g4-plans/reviews/composed-frame-report.md` ("Revision 4 recheck"): approve with changes (one leftover
  always-submitted rule, three small gaps). All applied in revision 4a.
- Archive review of revision 5 @ `16c5d8f7` (Opus 5.5 medium), report `tmp/g3t-g4-plans/reviews/composed-frame-report.md`
  ("Revision 5 archive review"): changes required (sizes buffer for all runtime-sized arrays, per-pipeline cook keys,
  cache patch scope, per-device archives, sizing). All applied in revision 5a.


### Earlier plan log


- 2026-11-07 Revision 1 drafted for Thrum Core's G3t/G4 commission.
- 2026-11-07 Revision 2 answers the G3t pair review and the composed-frame review:
  - **Proof tooling:** a capture probe and counters moved to M0, with an *unmeasured* label as the fallback, and a
    fixed-step route with a defined reference image.
  - **Instances:** 80 B rows keep the previous translation that interpolation reads; every consumer is listed.
  - **Uploads:** per-slot buffers and bind groups specified; the Apple ring is a raw shared-storage import, with
    `map_async` as the fallback.
  - **Graph:** per-variant memoryless allocations; split execution with an execute-time surface; encoded frames always
    submitted.
  - **Native segment:** texture rules, a stateful storage-writing stand-in and segment timing added; the consequence of
    refusing seam 1 is stated.
  - **Cross-plan:** numbering corrected; the M3 → G4 handoff added; the extent change is sequenced after M3.
  - **Lights and mirrors:** a deterministic GPU binning oracle, cost models, and mirrors through G3c's indirect
    records.
  - **Sizing:** restated.
- 2026-11-07 Revision 3 answers the G3t re-review:
  - the surface test is rewritten for submit-without-present, with G3b owner review;
  - submit-without-present runs on iOS only in the foreground;
  - the lifetime reason is corrected;
  - the ring preconditions are stated;
  - instance rows are 80 B;
  - the stand-in tolerance is stated;
  - the tone-encode guard test is named.
- 2026-11-07 Revision 4 answers Thrum Core's review of revision 3:
  - M0's counters and capture probe are commissionable separately; the route stays a G7-reviewed seam;
  - the native segment is marked a proposed amendment, with producer and consumer transitions, Metal's tracking
    guarantees and what wgpu-core cannot observe;
  - the stand-in also covers a discarded encoded frame and the iOS background race (D10.5);
  - the portable presentation-drop contract is restored in M2; submit-without-present is limited to frames carrying a
    stateful native segment, behind seam 3;
  - archives are put to the owner as (a) against (b), with no V1 exit wording change;
  - the atmosphere checkpoint is `d2a5342`, for interface reads.
- 2026-11-07 Revision 4a applies the composed-frame recheck of `9938b0d0`: the lifetime guarantee is generation
  ownership, not wgpu-hal's retained-references default, and N's completed handler raises device loss.
- 2026-11-07 Revision 5 rewrites seam 5 to the owner's direction: measure first, then offline metallibs through
  passthrough and an upstream Metal `PIPELINE_CACHE` (gfx-rs/wgpu#8526). The rewording option is withdrawn.
- 2026-11-07 Revision 5a applies the archive review of `16c5d8f7` to seam 5: the cook is per pipeline, the patch carries
  `MetalLib` reflection, the validation key, a Metal `adapter_key` and the archive-miss option, and archives come from
  each shipped device.
- 2026-11-07 Revision 5b adds parallel pre-warm, the cook's MSL match check and the measure → metallib plus pre-warm →
  `PIPELINE_CACHE` order to seam 5.
- 2026-11-07 Revision 6 makes seam 5's upstream contribution a later, non-V1-gating item under the owner's rules. The
  pinned patch delivers V1's archives.
- 2026-11-07 Revision 7 cuts seam 5 to outcome level under the owner's direction; details are designed when G4 M10
  starts.
