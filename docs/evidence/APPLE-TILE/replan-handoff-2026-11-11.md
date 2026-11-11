# APPLE-TILE: re-plan handoff, 11 November 2026

Moved from the module plan on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. These three sections are the planner's handoff from the scoped re-plan that answered [the second design review](design-review-2.md) and the 11 November D0.2 correction. They are unchanged, and describe the module as it stood then; M0 has since passed [its review](M0-review.md).

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
  unchanged word for word, and [the M0 record](../render/apple-tile-m0/README.md) at `03bc878a` already
  addresses each. The builder re-records them against that record and reruns the milestone checks at the claiming
  revision. A recapture is needed only if renderer, shader, capture or cooked inputs changed since `03bc878a`.
- D0.3 creation diagnostics are implemented at `be252c0d`. D0.4's [traffic probe](../render/G3T-TRAFFIC-PROBE-2026-11-08.md)
  establishes a per-encoder replay method, not a current physical-traffic gain. Missing original bulk scratch is
  not recreated or treated as proof.
- M0 is `blocked` until the lead's `unblock`; all other retained milestones are `open`, with zero `fails`. This
  revision follows a builder's scope question, not failed reviews, so no `replans` counter changes: M0/M2/M3/M12
  stay at 3, M1 at 2, M10/M11 at 1. Removed M4–M9 have no successor acceptance claim. No `*-ACCEPTED.json` is edited.

## Design review handoff

The design was approved at `2514d824` and its text has not changed, so the loop owes no design review for the
11 November D0.2 correction. That correction matches the approved review's own reading: it lists D0.1/D0.2 as
ordinary controls and D10.1/D10.2 as the native fixture and twin. For any later design revision, start from
[the revised brief](../../plans/APPLE-TILE/BRIEF.md), derive its capabilities, then read design/JSON and actual consumers.
Check each disposition above, unchanged shader/quantization assumptions, exact native copy/select feasibility,
C06 whole-encoder ordering/initialization, prepared resource/twin cost and discard/reset/loss rules. Check the G4
pre-tone seam in its consumer documents, COOKED-DRAW ownership, target capture modes, explicit window grants and
Mac critical path. Judge whether all proofs reject omitted work. No owner-only decision is required by this scoped
re-plan. Approval, unpark and milestone acceptance belong to the loop; this planner claims none of them.
