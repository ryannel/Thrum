# Apple tile recipe plan (G3t)

Design: [APPLE-TILE.md](APPLE-TILE.md) · [Brief](APPLE-TILE-BRIEF.md) ·
Done when: [APPLE-TILE-DONE.json](APPLE-TILE-DONE.json) owns criteria and proof status.
[Boundary](../decisions/G3T-METAL-BOUNDARY.md) and C04/C06/C07/C08/C12 govern.
Reviews: [design review at `2514d824`](../../evidence/APPLE-TILE/design-review.md): DESIGN OK (two advisory findings). The design text is unchanged since, so that approval stands.
Milestone: M0 A repeatable offscreen valley proves the comparison path (claimed)
Now: D0.1–D0.5 pass against the retained Mac/Chrome record and corrected D0.2 scope. The current milestone gate and full repository check pass; M0 is claimed for lead review.
Next: lead review of M0.
Blocked: none for M0. M2/M3/final attended rows and M12’s COOKED-DRAW dependency retain their existing requirements.

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

- **Who owns the native witness.** M0 proves the ordinary comparison path only: portable repeats, retained pre-change
  baselines, present/letterbox controls and omission controls. The native copy/select witness and its stateful twin
  belong to M10. D10.1 makes the small same-submit fixture M10's first step, D10.2 proves it exact through route,
  reset and replacement, and D11.2/D12.3 keep it labelled and exact afterwards. No native provider is built before
  M10, because D10.1 requires independent review of C06 and the boundary first.
- **M0's proof after this revision.** Changing D0.2's statement changes M0's definitions, so the loop clears the
  status and evidence of all five M0 items when it authorises this plan. D0.1/D0.3/D0.4/D0.5 and D0.2's `ask` are
  unchanged word for word, and [the M0 record](../../evidence/render/apple-tile-m0/README.md) at `03bc878a` already
  addresses each. The builder re-records them against that record and reruns the milestone checks at the claiming
  revision. A recapture is needed only if renderer, shader, capture or cooked inputs changed since `03bc878a`.
- D0.3 creation diagnostics are implemented at `be252c0d`. D0.4's [traffic probe](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md)
  establishes a per-encoder replay method, not a current physical-traffic gain. Missing original bulk scratch is
  not recreated or treated as proof.
- M0 is `blocked` until the lead's `unblock`; all other retained milestones are `open`, with zero `fails`. This
  revision follows a builder's scope question, not failed reviews, so no `replans` counter changes: M0/M2/M3/M12
  stay at 3, M1 at 2, M10/M11 at 1. Removed M4–M9 have no successor acceptance claim. No `*-ACCEPTED.json` is edited.

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

## M0 A repeatable offscreen valley proves the comparison path: claimed

Outcome criteria: D0.1–D0.5 in the done-when file. No native witness, provider or twin is part of M0.

- [x] Extend T2 linear comparison and full-resolution controls; retain pre-change sources and rerun creation diagnostics.
- [x] Re-record D0.1–D0.5 against the retained M0 record, and replace that record's opening scope-conflict paragraph with the settled scope.
- [x] Run the milestone checks plus `./dev check` at the claiming revision, then claim.

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

The design was approved at `2514d824` and its text has not changed, so the loop owes no design review for the
11 November D0.2 correction. That correction matches the approved review's own reading: it lists D0.1/D0.2 as
ordinary controls and D10.1/D10.2 as the native fixture and twin. For any later design revision, start from
[the revised brief](APPLE-TILE-BRIEF.md), derive its capabilities, then read design/JSON and actual consumers.
Check each disposition above, unchanged shader/quantization assumptions, exact native copy/select feasibility,
C06 whole-encoder ordering/initialization, prepared resource/twin cost and discard/reset/loss rules. Check the G4
pre-tone seam in its consumer documents, COOKED-DRAW ownership, target capture modes, explicit window grants and
Mac critical path. Judge whether all proofs reject omitted work. No owner-only decision is required by this scoped
re-plan. Approval, unpark and milestone acceptance belong to the loop; this planner claims none of them.
