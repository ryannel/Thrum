# CONTENT-LOOP log

- 2026-11-13 M1 resumed at `4f47db28`. Builder inspection confirmed that `LentDiagnostic` has only
  a public lease, that native comparisons map the lease's portable identity for every mip, and that
  discard/capture cannot commit its observed serials. The coupled-reader test checks every submitted
  image and sampled buffer against all-old/all-new references in both contribution orders. The staging
  probe fails the mapped-write operation, reaching the production slot-release branch. The public
  guide documents request names, readiness, serial commit, device rebuilding and consumer ownership.
  These are source-inspection findings. The final item check at `4f47db28` completed successfully:
  job `20261113-170643-440`, nine automated passes and M1.8's documentation inspection. Native,
  fault and Chrome passes were reused by the script's source-file key from 15:55 UTC; the exact
  verifier ran fresh for 928.12 seconds and passed both native matrices and all four Chrome sets.
  Its covered-content proof key is `57788bc9592743b7f6c13f0e9dadaeccf8cf32e80baa89a6e6ea703b11ef3c83`.
  The retained target logs in the compact evidence remain the underlying proof; this entry records
  the final strengthened source's automatic verification. All ten items are now marked passed.
  Full milestone check job `20261113-172340-10345` then passed at `23f3cf2f`: ten automated
  passes including `./dev check`, plus M1.8's source inspection. The repository suite ran outside
  the sandbox under the GPU lock and also reran the lease GPU tests. Integration follows through
  the required merge command; M1's state remains `in_progress` for the lead's checks and review.

- 2026-11-13 M1 built and target-proved. The native lease and coupled-reader tests pass, including both
  profiles, all portable mips, BC4 quality, discarded recordings, capture invalidation, device loss and shared
  switches. The fault test passes through the production error branch; Chrome verifies 28,766,400 bytes after
  load, edit and recovery with one product fetched. The unique-volume bench and existing tripwires pass;
  cold first-load CPU costs remain visible. Both native exact matrices and four Chrome sets passed at
  `eceb4c9a`. [Compact evidence and limits](../../evidence/CONTENT-LOOP/M1.md). Final script checks and merge follow;
  the milestone state stays `in_progress` for the lead.

- 2026-11-13 M1 implementation checkpoint: residency now offers configured sampled products through a public
  lease, with one shared switch after the last-copy submission, retained views and per-product serials. Added
  the opt-in game diagnostic, native lifecycle/coupled-reader/fault tests, generated Chrome fetched-feed case,
  leased upload benchmark scenario and [consumer documentation](../../COOKED-LEASES.md). Native and Wasm lint
  checks and documentation checks pass. Target proofs are still running or queued under the shared GPU lock;
  no item is marked passed from compilation alone.

- 2026-11-06 Before the loop: the content-addressed loop was built and run in the V1 game (`thrum-asset`,
  `thrum-cook`, `thrum-asset-gpu`), with bounded staging, tripwires and the Chrome client added by 8 November.
  [G5 decision](../../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md);
  [evidence](../../evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md). The direct draw path is
  [COOKED-DRAW](../COOKED-DRAW/PLAN.md) M1.
- 2026-11-13 Planned from the lead's task, quoted in the design, and the brief in the module list. Three
  milestones: leases for other modules with the staging failure test (after `APPLE-PRESENTATION:M2`), cooked
  mesh rows with live replacement (after `CULLING:M3`), and the close of G5 in the running valley (after
  `APPLE-PRESENTATION:M3`).
- 2026-11-13 Planner's decisions. A lease offers a set only after the frame that recorded its last copy is
  submitted, so consumers need no order inside a frame. One residency uploads for every consumer. Row replacement
  is built in `CULLING`'s route by this module's builder under a seam ruling in the module list, with a fresh
  scene as its exact oracle and a history-reset frame at the switch. Sprite-sheet and font products, a cook-time
  volume atlas and an iPhone volume encoding are named as later asks and not built.
- 2026-11-13 Researched: Unreal's per-frame streaming caps and RDG external textures, Bevy's mesh allocator,
  Chrome 139's sliced-3D compressed textures. Read the Broadsides cloud asset contract at `b4d8250e6` for what
  `CLOUDS` will hold (twelve 128³ R8 volumes); nothing was lifted.
- 2026-11-13 Checks. The done-when file passed the loop's validator. Each new test and Chrome check was run by
  hand and fails as written, before the work exists. The loop's own items-only dry run waited on the `gpu` lock a
  builder held and was stopped. The verifier, the tripwire benches and the shipping-link scan are existing
  commands that pass today; they were parsed as shell and not run, because they need release builds, the shared
  GPU and Chrome.
- 2026-11-13 The design still needs its review; the planner could not start one.
- 2026-11-13 [Design review](../../evidence/CONTENT-LOOP/design-review.md) of `8d09e6b4`: changes requested, two
  Must Fix findings and three suggestions. Both findings were checked against the code and are right.
- 2026-11-13 MF1 resolved by narrowing the claim, not by building source admission. M3.7 promised that a "partial
  change set keeps the previous set"; the watcher and cooker cannot know where an external save ends, and the G5
  decision already records that. The design now says what a publication promises and what it does not; M3.7
  claims presentation-only reading and that an incomplete save converges on a fresh cook of the finished sources,
  with a new `thrum-cook` test. M3.8 and M3.9 name the limit. C05 is unchanged; the reading is an assumption for
  the owner.
- 2026-11-13 MF2 resolved by matching the proof to the host. M2.3 asked for a frame encoded before a row switch
  and submitted after it, which the exclusive encoded-frame token makes impossible. The item now proves a
  discarded switch frame and a newer edit superseding an unswitched replacement, and asks that every write and
  switch go through the host. The design drops "the case the builder must fence".
- 2026-11-13 Suggestions, all three taken. S1: the lease's change signal became a serial the consumer compares
  and commits in `submitted`, so a discarded or capture frame cannot lose a change (design rule 5, M1.3, M1.7,
  M1.8, the module list's lease line). S2: M3.3 says a stall this module causes at the claimed sizes fails; the
  larger-size figure is a record. S3: the design says each reader switches whole in a frame of its own, with the
  coupled cases named and a second assumption for the owner.
- 2026-11-13 The changed design needs a second review; the planner could not start one.
- 2026-11-13 [Second design review](../../evidence/CONTENT-LOOP/design-review.md) of `a0620453`: changes
  requested, one Must Fix finding (MF3) and one suggestion (S4). MF1, MF2 and S1 to S3 were found resolved. MF3
  is right: the revision let placements, leases and rows switch in frames of their own and deferred C05's one
  switch to source admission.
- 2026-11-13 MF3 resolved by meeting C05 as written, with no contract change and no owner decision. Residency now
  owns one publication per generation. Each reader prepares under its own bound without showing; the publication
  is ready when the slowest reader is; one frame then switches placements, leases and rows together, tentatively,
  and commits for all at submit. A ready publication is not superseded, which fixes the frame's generation
  before any reader records and so needs no order between contributions. A publication one reader cannot take is
  refused for every reader. Cost: one frame of latency for placements beside a lease, and the row write's frames
  for a mesh entities draw. A scene with only placements is unchanged, so no retained image moves.
- 2026-11-13 Items changed for MF3. New coupled-reader proofs M1.10 (a texture on a placement and in a lease) and
  M2.9 (a mesh placed and as a row, with a write of several frames and an arena refusal). M1.2 reworded for the
  shared switch frame; M1.5 refuses a publication for placements and leases alike. M2.3 was rewritten because a
  ready replacement can no longer be superseded: it now proves a newer edit behind a discarded switch frame, and
  supersession while the replacement is still being written. M3.1 to M3.4 run three edits, two of them to a
  product read through two paths, and M3.1 and M3.3 assert one generation per frame across readers.
- 2026-11-13 S4 taken: the design says which layer keeps the last good content, the manifest for a fault the
  cooker finds and residency's resident generation for one the running game finds. Also fixed while there: a
  lease's CPU reader takes the product the lease names, not `HEAD`; a row switch committed by a capture frame
  keeps its history reset for the next frame that advances history.
- 2026-11-13 Module list: a content-publication interface was added, the lease and entity-visibility lines and
  the M2 seam ruling now name the shared switch frame. `CULLING`'s items do not change.
- 2026-11-13 Researched for MF3: Unreal's reimport recreates the render state of every component using a mesh
  under one scope; Bevy extracts changed assets at one point per frame and defers what exceeds a byte budget.
  Checks: the done-when file passed the loop's validator, `./dev check-docs` passed, and the two test commands
  the new items use fail as written. No number was added to any item.
- 2026-11-13 The changed design needs a third review; the planner could not start one.
- 2026-11-13 Fixed M1.3's overlapping-edit reuse defect from the [M1 review](../../evidence/CONTENT-LOOP/M1-review.md)
  in `f3099f6d`. Upload plans retain their resident HEAD; plans prepared behind a protected ready publication
  are replanned after its switch, before recording copies. Native and fetched preparation share the rule.
  Verified on this Mac's Metal device: all three lease tests passed, including both profiles, both contribution
  orders and native/fetched overlapping edits; Clippy passed. The full exact verifier passed both native
  matrices and all four Chrome sets (977.48 seconds). [Compact proof](../../evidence/CONTENT-LOOP/M1.md#overlapping-edit-repair).
- 2026-11-13 The repository gate exposed 27 historical evidence links left pointing to `ACCEPTED.json`
  after the plan migration. Retargeted only those links to `RECORD.json`, which owns the cited briefs and
  human feedback. `./dev check-docs` passed (774 Markdown files, 4,955 links); review prose and verdicts are unchanged.
- 2026-11-13 The overlap repair and migration links merged as `ea58db51`, after the repository gate passed.
  Reviewing M1.5's full refusal statement then found that cached textures skipped use-dimension validation.
  `011521f8` centralizes that validation for both cached and newly mapped textures. The stronger Metal fault
  test passed: a manifest assertion proves the invalid volume request reuses the placement's resident 2D
  product, and the whole publication is refused. Clippy passed. [Proof](../../evidence/CONTENT-LOOP/M1.md#overlapping-edit-repair).
- 2026-11-13 Complete candidate `fa1077748324f157223291c3c0ae7f1e7db07796`: the script's M1 items-only
  check passed all nine automated items on Metal and hardware Chrome WebGPU. Native lease, cached-product
  refusal, device replacement, coupled-reader and fetched-store checks passed. Both native baseline matrices
  and all four Chrome sets passed in 908.23 seconds, with covered-content key
  `044f2d2a852963ef343c302e9e1087434b43755e7e829186c861390f13affbc3`.
  The public guide and generic lease surface still satisfy M1.8; no interface or retained image changed.
