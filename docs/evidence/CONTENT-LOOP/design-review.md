# CONTENT-LOOP design review

13 November 2026. Reviewed commit `953c0423aa6fe50104b048464fca27be37e45ae1`, against the previous
review of `a06204535dde8dc3e87278c399a1ebe4215fc6c7`.

**Design approved, with one nonblocking suggestion. No Must Fix findings remain.** The revised design answers
the human-approved brief: cooked content reaches the running Mac and Chrome scene through bounded loading,
visible edits are judged for stalls and reconstruction defects, and shipping remains separate from authoring.
It now meets C05's cross-reader publication requirement instead of deferring it.

## Previous findings

### MF3 — Cross-reader publication is deferred against C05: resolved

The revised [publication mechanism](../../plans/CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2)
(lines 220–269) gives residency ownership of one publication across placements, leases and cooked mesh rows.
Each reader prepares without displaying the replacement. Readiness waits for the slowest reader; the selected
generation is fixed before extraction; all readers switch in the same submitted frame. A ready publication
cannot be superseded, a discarded switch is retried, and a refusal by one reader retains the previous generation
for every reader. The [module list's interfaces](../../plans/ARCHITECTURE.md#interfaces) and row-replacement
ruling now carry the same obligation. C05 itself is unchanged.

The previous counterexamples are addressed directly. A placement sharing a texture with a lease waits until
the lease can use it. A placed mesh sharing its product with an entity row waits for the row's bounded write,
even when that takes several additional frames. M1.10 and M2.9 compare actual outputs throughout those
intervals, including discarded switch frames; the latter also tests refusal of a publication that changes
another product. Independent early switching would fail these proofs. M3 now shows the coupled readers together
under reconstruction on both targets, rather than demonstrating only separate paths.

The ordering is implementable under the existing host protocol. The
[encoded-frame token](../../../crates/thrum-render/src/runtime/tokens.rs) holds the host exclusively while a
frame is unsubmitted. [The host](../../../crates/thrum-render/src/runtime.rs) calls `submitted` after queue
submission, including `submitted(false)` for capture, and the
[contribution list](../../../crates/thrum-render/src/contribution/list.rs) forwards it to every contribution.
Consequently every callback has finished before the next frame begins. Readiness established at that boundary
can select one generation independent of contribution order. The current residency's last-copy-frame swap and
unconditional newest-pending replacement must change for coupled scenes; the design and plan explicitly assign
that work to M1, with row participation added in M2. Approval does not imply those changes already exist.

- **MF1 — Partial-source-save guarantee has no mechanism: remains resolved.** The design still distinguishes a
  valid intermediate preview from a detectable bad save. M3.7 retains presentation-only use and final convergence
  against a fresh cook. The existing G5 preview exception remains explicit; this revision does not claim source
  admission or an external-save transaction boundary.
- **MF2 — The required old-frame-after-switch proof contradicts the host protocol: remains resolved.** M2.3
  continues to use the exclusive host token. It now distinguishes a newer edit arriving behind a ready,
  discarded switch from one superseding a replacement still being prepared. Neither requires an earlier
  unsubmitted frame to be overtaken.
- **S1 — Lease invalidation across an abandoned consumer frame: remains resolved.** M1.3 and M1.8 retain the
  serial comparison until a submitted frame advances history. The revised row-history rule also keeps a reset
  pending when a capture commits the switch.
- **S2 — Chrome stall question and quality bar: remains resolved.** M3.3 still fails content-induced stalls at
  the claimed valley and twelve-volume fixture sizes.
- **S3 — Publication granularity: resolved by the shared publication above.** The former explanation of
  independent switches has been replaced with a common switch and corresponding proofs.
- **S4 — Distinguish cooker publication from runtime last-good retention: resolved.** Design lines 253–260 and
  289–293 now distinguish retaining an asset's previous manifest entry after a cooker fault from retaining the
  runtime's resident generation after a runtime refusal. M1.5 tests the latter across placements and leases.

## Changed items and proof assessment

All 29 items and the three milestone bars were read. Each of the nine changed or added items was compared with
its previous wording and the prior findings. These assess planned proofs, not completed implementation.

| Item | Assessment at its milestone boundary |
|---|---|
| M1.2 | Judgeable in M1, before cooked rows exist. It distinguishes recording the last copy from submitting it, and retains the discard and superseded-upload checks. |
| M1.5 | Judgeable. Runtime refusal now preserves both placements and leases. The production-branch fault-injection question remains; an isolated probe that merely returns its own error cannot pass. |
| M1.10 | Judgeable with the M1 diagnostic consumer. Submitted placement pixels and consumer output must match all-old or all-new references throughout the edit, in both contribution orders and after a discarded switch. Independent switching cannot pass. |
| M2.3 | Judgeable under the production host protocol. The ready replacement must survive discard and a newer edit, while supersession during preparation must leave the current geometry intact. The old-space lifetime obligation remains. |
| M2.9 | Judgeable after M2 adds row replacement. The required interval with a completed residency upload but unfinished row write exercises the previous failure. Whole-frame references, shadows and refusal of another changed product make the shared switch observable. |
| M3.1 | Judgeable after reconstruction is available. The three edits retain mesh, texture and volume coverage while coupling two reader pairs. Per-reader generation records supplement the unchanged capture-off timing and tripwire requirements. |
| M3.2 | Judgeable. Both readers of a shared product must be visible together in realtime Mac capture under TAAU. The no-mix, no-flash and no-ghost criteria remain. |
| M3.3 | Judgeable. Chrome must show the same coupled edits and one generation across readers, with changed-product fetches and the existing causal no-stall requirement. |
| M3.4 | Judgeable. Chrome realtime inspection now includes both readers in view under reconstruction, so correct generation labels alone cannot substitute for correct output. |

The unchanged items retain meaningful checks for texels and profiles, retained resource identity, upload bounds,
device recovery, fresh-scene row equality, public usability, bad saves, shipping isolation and G5 closure.
No further Must Fix was found. A diagnostic lease consumer makes M1 independent of unfinished SEA and CLOUDS.
`EFFECT-GRAPH:M2` is already `done`, so M1 can start when `APPLE-PRESENTATION:M2` passes. M2's route work waits
for `CULLING:M3`; M3's reconstructed proof waits for `APPLE-PRESENTATION:M3`. No dependency cycle was introduced.

## Suggestion

### S5 — Name the portable product identity for a CPU reader

**Suggestion · verified wording ambiguity, not a demonstrated implementation defect.** Design lines 166–168
now correctly pin CPU reads to the lease's publication instead of current `HEAD`, but say to read the portable
product using the product hash the lease names. The lease identity at lines 143–145 also describes the selected
GPU profile. These need not be the same product: the
[volume cooker](../../../crates/thrum-cook/src/texture.rs) (lines 100–128) produces BC4 for desktop and R8 for
portable, and [manifest bindings](../../../crates/thrum-asset/src/manifest.rs) are keyed by name and profile.
Mapping the selected BC4 hash does not yield portable decoded voxel bytes.

Clarify that a CPU reader receives the portable variant's hash from the same selected publication, or can resolve
it through that publication's retained manifest. The existing manifest already supplies the necessary binding;
this is a clarification of the public identity description, not a reason to redesign the publication mechanism
or hold up the GPU lease milestone. Do not resolve the portable variant through a newer `HEAD`.

## Request and review limits

I read all three complete tasks under `briefs` in
[ACCEPTED.json](../../plans/CONTENT-LOOP/ACCEPTED.json), the human-approved
[project brief](../../plans/BRIEF.md), and the module-list entry before assessing the revision. The planning
tasks are lead/script instructions; they do not override the human's purpose or exit. Leases, live cooked rows
and the G5 integration support that request. Later sprite-sheet/font products and sea/cloud-specific choices
remain named future work. The new whole-publication refusal policy is explicitly listed as an assumption;
it preserves last-good presentation and does not change the V1 exit.

This was a document and scoped boundary-code review. I read the full design, plan and done-when file, their
revision diff, C05 and its recorded G5 exception, the affected architecture interfaces, and the submission,
contribution, residency and product-identity code cited above. I read the planner's log after forming the
assessment. Thrum's review calibration and the skill's starting examples were read; there is no plans glossary
or owner-graded verdict yet. No other agents were started.

The new Unreal analogy is consistent with Epic's documented
[render-state recreation scope](https://dev.epicgames.com/documentation/unreal-engine/API/Runtime/Engine/FStaticMeshComponentRecreateRend-).
The linked Bevy documentation was unavailable through the web reader. Neither analogy establishes Thrum's
correctness or absence of stalls; the local protocol and planned target proofs support this verdict.

No runtime, visual-quality or device-performance result is claimed. No capture or GPU run was needed. Only this
review file was changed; no milestone verdict or planner-owned file was edited. `./dev check-docs` passed
(741 Markdown files, 4,530 local links, 30 module records, crate policy and current design hashes).
`git diff --check` passed.

DESIGN OK
