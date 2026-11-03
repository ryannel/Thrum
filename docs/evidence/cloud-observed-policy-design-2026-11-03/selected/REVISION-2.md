# Revision2 — all-old member state/time contract

3 November 2026. Design-only closure of the independent review finding. Read with `POLICY.md`; this revision replaces its underspecified phrase “exact current geometry/response states.” Original POLICY/README/spec/FREEZE remain byte-identical. The derivative `policy-spec-v2.json` adds the explicit capture contract and pins its original specification. No helper/source/assets/viewer changes or new result.

## Selected rule: current-pose potential, with a checked source invariant

Freeze exactly the identity set returned by the original `oldMembers()` at the crown's actual **source birth barrier t0**, after the active step/retirement pass and before installing the child. Do not use presentation time or the previous frame's capture time as t0. For every member, explicitly join by ID to the source's current/previous slot states. Require current/previous source clocks to be `t0`/`t0-.25` (except the initial equal-time constructor pair, which cannot host the first scheduled crown). The checked source programme has unique current IDs and same-epoch slot histories.

In this programme, every previous-positive identity has a matching active current identity. Proof from the frozen `SourceModel.step()`:

1. Constructor/reset creates current then copies it into previous; same-ID membership holds. Current active slots are changed only by the existing step/birth/terminal-retirement paths in the qualified producer.
2. At a step, copyPart copies each active old-current identity into the next-current buffer before integration. Integration and weather/age loss do not change its ID or active flag.
3. Retirement clears active only when both that new current and its same-ID previous state satisfy `terminalZero`. A previous-positive state (`!zero(previous,tPrev)`) implies `!terminalZero(previous,tPrev)`, so such an ID cannot retire at that step. This uses the corrected terminalZero retirement rule, not the earlier rejected seed-zero rule.
4. Birth/slot reuse chooses only an inactive current slot. By3, a slot with a previous-positive identity is not inactive; birth cannot overwrite it. Clear recovery uses both-state terminal zero and does not bypass this rule. Reset/seek constructs a fresh epoch rather than joining old previous states to new IDs.
5. Therefore the previous-only-positive branch in `oldMembers()` is unreachable at a valid synchronous birth barrier of this programme. It remains a defensive general branch in the legacy implementation. External mutation, incompatible restore, an unqualified producer or a partial async step does not inherit this proof.

The new mode **asserts this invariant before prediction/admission**, including for a scheduled birth that ultimately declines. If legacy oldMembers returns a previous-only positive ID, retain the full returned ID list and its state-source/time in an invalid-input record and fail the qualified-state assertion. Do not drop it, silently advance it, call it an occluded cue, or count a successful birth. This is an unsupported/corrupt-source-state fault, distinct from a normal prediction/observed cue failure. Offline tests must inject the case and reject it explicitly. Legacy mode remains untouched. Supporting an arbitrary prior-only producer later would require a separately specified completed-interval reconstruction; this policy does not guess one.

## Same-ID prior-positive/current-zero is included from current

When an ID exists in both arrays but current optical q is terminal-zero and previous was positive, preserve the ID and copy **current geometry and response state at t0**. Do not start from previous geometry at t0-.25, do not duplicate the ID into two pose branches, and do not union both trajectories under one implicit clock. This defines the policy's all-old counterfactual as the potential of the current source material maps at birth, with optical loss suppressed. It is not a reconstruction of the prior/current interpolated display field.

Loss suppression affects only the reference's amount evaluation: ignore age/weather E, while preserving the exact current C/u/K/Q/root/mu/tau/b/g/Aseed and ordinary future response/map updates. Retained scalar exposure extrema may be recorded as provenance but cannot alter this reference's optical amount. Ordinary Aseed continues; it is not forced to1. Thus an actually zero old cohort can have nonzero *counterfactual potential*, which is deliberately conservative for new development and is never rendered or resurrected in the actual source.

Known370s ID7 is the required example: it is current at age150, with a previous-positive same-ID state. It remains in `[9,8,3,10,6,7]`, starts the reference from current stateTime370, and continues to415 under the actual applied interval controls despite actual retirement. Its prior stateTime369.75 is provenance, not an unacknowledged reference start. No additional old identity or historical reference-only member is introduced.

## Required capture schema and checks

Each frozen member carries:

- `{epoch, cellID, localID, qualifiedID}` and source/model/asset/policy pins;
- `birthBarrierTime:t0`, `captureTick`, `captureTime:t0`, `currentStateTime:t0`, `previousStateTime:t0-.25`;
- `stateOrigin:'current'`, and `inclusion:'current-nonzero'` or `'current-with-previous-positive'` (a member may additionally record both booleans);
- `currentSlot`, exact copied current `{role,C,u,K,Qcolumns,root,s,mu,tau,b,g,Aseed,born}`;
- prior same-ID/positive flags and previous ID for audit, without retaining a second reference trajectory;
- `referenceTime:t0`, `lastAdvancedTick:captureTick`, `path:'generic-reference-continuation'`.

Every legacy returned ID must occur exactly once in the capture, every captured current ID must be active/current and match the slot/join, and no reference-only ID may enter. The actual parent must be present. Arrays are owned geometry copies and immutable capture inputs; no E views are borrowed. The .25 endpoint/time relation and epoch are checked, not inferred from an array label. Same-ID current terminal zero is allowed; a new child is not yet in the all-old set. A previous-only member records origin `previous` in the fault receipt but never becomes a valid trajectory with a fabricated t0 timestamp.

Held-cause prediction and actual reference observation use this **same normalized capture**. Prediction copies start at t0 and perform180 held-control steps. Actual references start at t0 and advance exactly once per completed applied interval, stamping referenceTime; at observation they must equal t0+45 and captureTick+180. No .25 correction is silently inserted into either path. The new-mode step order and cancellation/cleanup semantics in POLICY remain unchanged.

Reset/seek/mode interruption rules remain: no cross-epoch joining; fresh replay recaptures each actual birth. Same-cell clear recovery does not interrupt the current45s reference. An old ID can outlive its actual slot as a CPU-only counterfactual, retaining its qualified identity and current-at-birth state provenance. The clock/source physics, identity set, numerical/threshold/radius/neck gates and finite cost counts are unchanged; this revision adds only bounded provenance/join validation within the existing8-member record.

## Decisive closure checks before implementation adoption

- Default source: prove/check current-positive and current-zero/previous-positive cases yield the exact legacy returned identity set and one current-time pose per ID.
- Exact370 replay: ID7 retained, stateTime370/referenceTime370, previousStateTime369.75; aligned45s endpoint415, no duplicated pose branch.
- Inject previous-only-positive, mismatched ID/slot, stale state time, partial-step capture and cross-epoch ID reuse: explicit state-invariant rejection, no silent omission or new birth.
- Capture a same-ID current-zero pair with deliberately different previous pose: prediction/observation must use current, never a mislabeled previous pose.
- No reference advance before its birth interval, no double advance after pause, correct actual controls over all180 intervals, deadline observation before release.

These are design/test requirements, not executed helper results. They close the identity/time definition without pretending the legacy method's general previous-only branch supplies current-time geometry. The operating policy remains proposed pending independent review, implementation, replay/cost and actual optical proof.
