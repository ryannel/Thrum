# Apple tile recipe plan (G3t)

Design: [APPLE-TILE.md](APPLE-TILE.md) · [Brief](APPLE-TILE-BRIEF.md) ·
Done when: [APPLE-TILE-DONE.json](APPLE-TILE-DONE.json) owns criteria and proof status.
[Boundary](../decisions/G3T-METAL-BOUNDARY.md) and C04/C06/C07/C08/C12 govern.
Reviews: [design review 2 at `224b58a1`](../../evidence/APPLE-TILE/design-review-2.md): DESIGN CHANGES (7 findings). Second failed design review; module parked by the lead. These findings remain unresolved.
Milestone: M0 A repeatable offscreen valley proves the comparison path (parked)
Now: 9 November parked after two failed design reviews; no implementation or new runtime proof claimed.
Next: await an explicit lead commission for a scoped re-plan addressing the seven second-review findings, then fresh independent design approval before any build.
Blocked: APPLE-TILE design is parked. T2 completion does not restart it. On recommission, M0 still needs `T2-CAPTURE:M0`; M1 additionally needs T2 Chrome M1 and simulator M2. G3t M2/M3 actual-window observer/grant findings remain unresolved. Mac M2 can qualify the current path independently; Mac M3 alone waits here for post-change requalification.

## Sequence and dependencies

The retained build order selects G3t after G3b functional work, but the module is currently parked.
The lead must explicitly recommission a scoped design re-plan and obtain independent design approval before build. One builder works
sequentially; no parallel-agent commission is implied. Each milestone follows the preceding one. M0 needs
`T2-CAPTURE:M0` for shared submitted-output/sequence capture; G3t still owns scene-linear checkpoint and recipe
comparison extensions through that capture owner. M2 also needs `PLATFORM-RUNTIME-MAC:M1` (already done)
for the loss/abandon boundary it must preserve. M1 needs `T2-CAPTURE:M1` and `T2-CAPTURE:M2` for the first
new Chrome and simulator visual proofs; keep M0's existing Chrome smoke separate. At M1 start, retain the Chrome
pre-change portable baseline before changing its renderer, using the qualified shared tap and pinned prior source.
Existing main supplies committed pages, frame host, atmosphere, cooked contributions, moving-route control and
supported visibility. M9 delivers the one shared V2 class/range mechanism and migrates the private bucket
consumer; no existing supported class ranges or future mixed-mesh qualification are assumed. The legacy raster
policy/order stays intact, including two-sided mirrors and open meshes.

G4 consumes M0 proof tools, M1 eligibility, M3 direct present input and M11 prepared native-provider lifetime/reset/
timing. M11 uses bounded extent replacement fixtures; M12 uses fixed extents and current readers. Neither waits back
on G4. G4 must later requalify actual reconstruction readers and dynamic scale transitions. Its full prose-to-JSON
migration belongs to its own planning commission; this task does not pretend its old prose is machine-readable.

Mac M2 qualifies the current surface path after T2 M0 without waiting here. New Mac M3 explicitly needs
`APPLE-TILE:M3` and reruns the full M2.1/M2.2 motion/cadence protocols with windowed-tap/tooling revalidation
on the changed path. That later milestone is blocked on this parked design, transitively including T2 M1/M2 and
G3t M0–M3. G3t D2.5/D3.5 retain their functional actual-window obligations; their observer/grant defect in the
second review is unresolved. No reverse Mac dependency is added to G3t.

## Carried work

- D0.3's historical creation seam remains implemented at `be252c0d` in `runtime/creation.rs`,
  `runtime/tests/creation_sites.rs`, `capabilities.rs` and the cooked consumer. Its claim is reset to open with empty
  evidence: rerun the diagnostic check on the integrated M0 revision and retain actual output before claiming.
  Plan prose and earlier approvals (below) do not prove the revised M0 check.
- D0.4 stays a builder pass from the [8 November traffic probe](../../evidence/render/G3T-TRAFFIC-PROBE-2026-11-08.md).
  It establishes a per-encoder replay method, not live/per-attachment bytes or a new timing baseline. Original bulk
  captures/screenshots are absent here; the retained transcription/limits are the available evidence.
- The old M0 never closed. M0 is now parked; all item claims are retained. Only Mac handoff references change below. Initial `fails`/`replans` were zero:
  historical conditional design reviews and missing screen slots were not failed claimed milestone reviews. M0 has
  `replans: 2` after shared-capture sequencing and the first design re-plan. M2/M3/M12 now also have
  `replans: 2` for the Mac M3 handoff correction; M1/M4–M9 retain `replans: 1`. M0 is parked and M1–M12
  remain open with `fails: 0`; unchanged M10/M11 retain their counters. D0.4 is the only
  retained pass. This is a design re-plan, not a failed claimed-milestone review.

## Proof and builder handoff

Start with existing `python3 scripts/run_v1_showcase.py moving` for route smoke proof; it currently runs native
submitted SDR and Chrome canvas captures, not scene-linear recipe comparisons. M0 consumes the T2 renderer tap
and extends proof tooling to retain linear input, full-resolution sequences and a full-resolution same-target
comparator, retaining the native pre-change baseline at M0 and Chrome baseline at M1 start before changing its
renderer. Add a durable runbook with
exact new commands when implemented; names for nonexistent commands are deliberately not used as JSON checks.
The JSON's `ask`/`inspect` proofs require actual tests/readbacks/sequences and source tracing, not a prose assertion.

Use scratch `tmp/g3t-<milestone>/`, selected evidence `docs/evidence/render/G3T-<milestone>-<date>.md`. Pin build,
shader, route and cooked identities and retain decisive artifacts under the storage rules. Unavailable tools/targets
stay open, not successful skips. Acquire the GPU lock for GPU checks; no physical replay or screen automation is part
of this planning task. Use normal-speed sequence review; physical swaps remain Mac qualification M2/M3 proof.

The design's mechanism-specific comparison table requires exact same-target pre-blend scene-linear/SDR bytes,
including pre-change portable controls. M3 preserves SDR quantization in the fused shader; native blend vs twin alone
allows 1 ULP f16/channel. Small-omission light/mirror/night-star negative controls must fail. Chrome-only route checks
now also require full-resolution baseline comparisons and source/dependency/readback proofs. G7's coarse-grid
smoke evaluator stays unchanged. Numeric performance/allocation figures remain baselines/tripwires. A safe candidate
may close unpromoted except that M8 must GPU-qualify every named overflow case; CPU fallback cannot satisfy D8.1.
Any acceleration promotion requires measured win, retained quality, complete validation/admission/fallback CPU cost,
resident fallback bytes and comparable sustained-run thermal evidence under C12.

Ownership in implementation: renderer graph/runtime/capabilities and isolated `apple_tile` changes stay in their
existing crates; update native/iOS/browser/capture consumers sequentially. V2 keeps mesh/membership/facing ownership;
V3 keeps overflow ranking; G4 keeps reconstruction/display/archives. No contract changes during a build: re-plan
producer and consumers first. Register dependencies only when actually introduced, and justify each unsafe adapter.

<a id="m0-proof-route-counters-and-capture-probe-pending"></a>

## M0 A repeatable offscreen valley proves the comparison path: parked

Outcome criteria: D0.1, D0.2, D0.3, D0.4, D0.5 in the done-when file.

- [ ] Extend existing route/capture proof without changing G7 content/evaluator; preserve and rerun creation diagnostics; retain counter limits.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M1 The valley selects a recipe and stores only actual future inputs: open

Outcome criteria: D1.1, D1.2, D1.3, D1.4, D1.5 in the done-when file.

- [ ] Add truthful registry/override and per-variant eligibility; prepare allocations; prove actual storage and fallback on named targets.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M2 The existing frame lifecycle runs through one split graph: open

Outcome criteria: D2.1, D2.2, D2.3, D2.4, D2.5 in the done-when file.

- [ ] Merge graph/late surface binding and preserve healthy skip, lost abandon, history and iOS lease behavior.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M3 Present consumes scene-linear colour directly: open

Outcome criteria: D3.1, D3.2, D3.3, D3.4, D3.5 in the done-when file.

- [ ] Move tone/display to present, keep prepared linear capture and update every pass-timing consumer.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M4 Apple slots receive pending changes while portable keeps queue ordering: open

Outcome criteria: D4.1, D4.2, D4.3 in the done-when file.

- [ ] Prepare Apple per-slot buffers/bind groups and reconcile all pending changes; retain portable single-buffer queue ordering and record upload-byte deltas.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M5 Apple upload transport is safe under completion and fallback: open

Outcome criteria: D5.1, D5.2, D5.3 in the done-when file.

- [ ] Implement narrowly reviewed shared-buffer import/completion rules and explicit safe fallback; measure before promotion.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M6 Only safely ordered scratch is shared between slots: open

Outcome criteria: D6.1, D6.2 in the done-when file.

- [ ] Share only proven scratch lifetimes and retain history/readback/occlusion ownership; test delayed consumers.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M7 Compact internal rows preserve supplied values and motion: open

Outcome criteria: D7.1, D7.2, D7.3, D7.4 in the done-when file.

- [ ] Pack private 128-byte rows and update every shader consumer while keeping public admission and supplied normals.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M8 GPU light lists preserve the established lighting choice: open

Outcome criteria: D8.1, D8.2, D8.3 in the done-when file.

- [ ] Extend GPU binning with established strongest-32 ranking; exact CPU oracle, visible fallback and dense-light proof.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M9 Mirrored ranges preserve existing visibility ownership: open

Outcome criteria: D9.1, D9.2, D9.3 in the done-when file.

- [ ] Deliver shared V2 ranges, migrate supported/private bucket consumers, and preserve admitted raster policy/order; validate open/back-side/interior meshes, synthetic slot 5060, shadows and ties.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M10 A stateful native segment composes inside one submission: open

Outcome criteria: D10.1, D10.2, D10.3, D10.4 in the done-when file.

- [ ] Implement reviewed whole-encoder segment and stateful twin; prove ordering, lifetime, discard/reset and iOS race.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M11 Native providers survive replacement and expose truthful timing: open

Outcome criteria: D11.1, D11.2, D11.3 in the done-when file.

- [ ] Prove generation/extent replacement and prepared rejection/recovery, then report native timing with clock limits.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## M12 An integrated record states qualified wins and remaining targets: open

Outcome criteria: D12.1, D12.2, D12.3, D12.4, D12.5, D12.6 in the done-when file.

- [ ] Rerun integrated route on both recipes/Chrome/simulator, measure fixed extents/ablations, record outstanding device/display rows.
- [ ] Record actual proof and independent verdict where required; run scoped checks and `./dev check` before claiming.

## Design review handoff

On explicit lead recommission, resolve the second review findings, then give a fresh `/review-milestone` reviewer only [APPLE-TILE-BRIEF.md](APPLE-TILE-BRIEF.md) first. It derives the required
capabilities before reading this design/JSON and actual consumers. Review the whole-encoder C06 amendment, unchanged
skip/abandon semantics, private compact layout, supplied normals, mandatory GPU overflow qualification, shared V2
range/raster authority, resident fallback cost, thermal/validation promotion rows, actual Mac window proofs and
post-M3 Mac cadence ordering. Check the lack of G4 cycles. Verify every JSON proof catches the wrong substitutes.
Record a new design verdict on Reviews;
no historical approval or self-review substitutes for that read. The module stays parked until that recommission
and approved re-plan; this task launches no agents and does not commission the outstanding G3t redesign.

## Log

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
