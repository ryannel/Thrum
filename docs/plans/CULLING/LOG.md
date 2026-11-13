# CULLING log

- 2026-11-13 Continued M3 from `fa3cc02a` with the original forward shader.
  A diagnostic copies colour, all three Sun depth layers and the air-shadow map
  from each same submission. After two passing full routes, tick 430 differs
  at 23 Frustum colour pixels while every stored shadow map is byte-identical
  across four hosts. [Evidence](../../evidence/CULLING/M3-investigation.md#simultaneous-colour-and-shadow-resources-on-the-original-shader)
  narrows this failure to other inputs or forward evaluation; it is not a repair.
  Diagnostic production edits are restored, and M3 items remain open.

- 2026-11-13 Resumed at `2fd5cf06`. Its literal M3 checker passes native cubes,
  Chrome cubes/meshes and temporal sampling, but native meshes differ at six
  SDR pixels at tick 255. Metal API/GPU shader validation separately reproduces
  an 18-pixel scene-linear difference without a reported validation error.
  The exact verifier rejects APPLE-TILE's `portable-sdr-approach` with 4,859
  changed pixels. The explicit-gradient candidate is rejected and the shader
  restored byte for byte to its `64c81cd2` blob; no reference changed. The
  [retained evidence](../../evidence/CULLING/M3.md#explicit-gradient-candidate-rejected-at-2fd5cf06)
  separates candidate results from the unqualified restored source. Under the
  plan's exact-baseline stop rule, the [handoff](../../evidence/CULLING/M3-handoff.md)
  proposes continuing diagnosis with existing sampling and retained references.
  M3 stays `in_progress`, all builder items remain open, and no merge is claimed.

- 2026-11-13 Clean `64c81cd2` passes Chrome cubes/meshes, the full temporal
  cycle and every exact baseline (native 38 + 4, four Chrome sets of 481).
  The native item checks reproduce sparse terrain differences: cube Occlusion
  at tick 379 and independent mesh Direct at tick 180. All three same-time
  recaptures of each host converge to the common Direct bytes, with no tick
  advance or changed alpha/exposure value. [Reachable evidence](../../evidence/CULLING/M3.md)
  retains the original failures and the earlier fully passing native mesh run.
  Shader-instrumented stage comparisons pass but change compilation and do not
  repair the original. A new scoped candidate evaluates explicit UV gradients
  before Sun control flow and shares them across material samples; sampler and
  anisotropy stay unchanged. Four extra scalars live across the Sun search;
  strict target/baseline checks and measured GPU cost are pending.

- 2026-11-13 Resumed M3 from `f8255e6b`. Promoted the selected native/Chrome
  fixed and realtime inspection frames, the mixed-field readbacks and the
  independent-Direct SDR/scene-linear failure bytes. [M3 evidence](../../evidence/CULLING/M3.md)
  distinguishes historical passes from pending committed-source qualification;
  [the investigation](../../evidence/CULLING/M3-investigation.md) records the
  exact failures. A temporary depth readback finds all 361 near-shadow maps
  identical across Direct, Frustum, Occlusion and independent Direct; the other
  layers remain queued. Shadowed-air-only ablation still fails at tick 180.
  Temporary renderer edits are reverted. Repaired 27 stale evidence-link
  destinations after the plan-record migration; `./dev check-docs` passes
  777 Markdown files and 4,975 local links. No M3 item is newly marked pass.

- 2026-11-11 Planned from the owner's task, quoted in the design. Before the loop, a Claude session built Frustum,
  Occlusion on cubes, the bucket route in Direct and Frustum, and Sun-caster culling (7–8 November); its four
  evidence pages are in `docs/evidence/render/G3C-*.md`. Its design and plan were never committed. This design
  carries their mechanisms forward and replaces them; their remaining milestones became M1–M3 here.
- 2026-11-11 Planner's decisions: each entity has one visibility authority (instance culling for field entities,
  meshlet culling for cooked placements), recorded as an interface in the module list; G3c closes on the scenes
  that exist; the old native-recipes milestone is not carried. Researched: Haar and Aaltonen 2015, Karis 2021,
  Bevy's and Unity's occlusion culling.
- 2026-11-11 The heavy checks (release build, GPU, Chrome) were not dry-run by the planner: the planner's checkout
  had 23 GiB free against the 16 GiB the capture scripts require, and the GPU is shared. Each check was parsed as
  shell and the done-when file validated with the loop's own validator.
- 2026-11-11 [Design review](../../evidence/CULLING/design-review.md): changes requested, one Must Fix. MF1: the
  live mode switch relied on host republication, which has no Chrome entry point, takes no requested mode, and
  rebuilds the cooked contribution's pipelines at publication, where a failure detaches the cooked scene.
- 2026-11-11 Design revised. MF1: the live switch is removed, with M2.5 and M2.6 and cue C4. The brief and gate G3c
  never asked for it, and a working one is a transaction across the frame runtime, the browser host and the
  contribution seam. The design records the mechanism to cost if the owner wants it (a prepared set of modes in
  one scene); the plan lists the removal as an assumption. M2 is renamed and its exact item renumbered M2.5.
- 2026-11-11 S1 applied: M1.3 names a separate footprint witness, with a no-widening mutant or a record of the
  poses searched, and the design calls the four-ulp interpolation term an assumption and adds it as a risk.
- 2026-11-11 Exact baselines, from `BASELINE-GUARD`'s review: M2 gained `BASELINE-GUARD:M1` in its `needs`, and M2
  and M3 carry the verifier (Mac and Chrome) in place of the native-only command. M1 keeps the native matrix and
  does not wait; the plan lists that as an assumption.

- 2026-11-11 M1 implementation: logical-slot bucket history, class-matched depth twins and mesh admission.
  [Metal evidence](../../evidence/CULLING/M1.md): eight tests pass through churn, reveal, resize and fresh-device
  preparation; all three required mutations fail pixel equality, including the mesh tie with zero tolerance.
  The footprint reference search found no witness in 8,640 valid poses. Full flythrough, exact images, Check
  and integration remain unverified; no speed or new motion-quality claim.
- 2026-11-11 Initial release full-path check failed at tick 866 (Frustum versus Direct, two pixels).
  The unchanged rendering code subsequently passed isolated endpoint, 66-frame lead-in and full-prefix
  replay through 866; the first failure remains unexplained and retained in the evidence. Full qualification
  is being repeated with failure capture. Renderer regressions passed 242+9+8 tests. M1 remains open.
- 2026-11-11 Complete release target subsequently passed all 1,930 ticks on `bb3fe56c`; both culled modes
  equal Direct, no missing counts, Occlusion never exceeds Frustum and halves it on 1,918 ticks. The initial
  nonrepeatable two-pixel failure remains explicitly retained, not explained or erased. The real baseline
  verifier refused before capture: requires 16 GiB free, about 12 GiB available. Milestone check remains running;
  native exact proof, profile Check and integration are unqualified.

- 2026-11-12 Disk headroom recovered. Full exact verifier passed clean `53646df3` under the GPU lease:
  native matrices 38+4 comparisons, all four Chrome sets 481 records each; retained references untouched.
  [Compact proof](../../evidence/CULLING/M1.md#exact-baselines-12-november-continuation). Mandatory milestone/profile
  check and a new independent-Direct/recapture diagnostic remain queued/running; M1 stays open.

- 2026-11-12 Second tick-866 investigation: independent Direct plus both culled modes match on every prefix
  tick through900; all248 extra Direct recaptures around866 are stable. CPU support diagnostic finds no
  rounding-allowance overlap at866 and its visible count agrees with the aligned GPU row. No runtime fix
  was made. Original pixels were not captured, so the old event cannot be attributed retrospectively;
  [the evidence](../../evidence/CULLING/M1.md#second-investigation-of-tick-866) records the tested hypotheses and limits.

- 2026-11-12 M1 set to claimed as requested once every item was pass with linked Metal evidence.
  The full 1,930-tick named target, bucket witnesses/mutations and exact native matrices support the claim;
  the original unattributable observation remains explicit in the evidence. The queued aggregate check
  continues; integration will run the profile Check under the integrate lock. Lead owns the review.

- 2026-11-12 Final aggregate qualification passes all five pinned M1 commands and the profile Check;
  the raw native matrices pass 38+4 exact comparisons. Compact reports and source identity are
  retained in [M1 evidence](../../evidence/CULLING/M1.md#final-mandatory-qualification). M1 is claimed for lead review.

- 2026-11-12 Reopened M1/M1.2 after the loop's clean-copy target exited101 at integrated b2d6ad8f.
  The supplied stderr tail omits the panic, so replaying its exact executable with full output and
  failure capture. Earlier passes remain historical, not a repair for this new rejection.

- 2026-11-12 Diagnostic checkpoint cbe685ec preserves failed oracle pixels, repeat captures,
  frame context and uncaptured stderr panic summaries. The formerly rejected executable, a fresh
  build and the first current-source full-path stress trial all pass; none proves a rendering repair.
  The full-route CPU plane reference found no vertex crossing in its 25 rounding candidates.
  Investigation and limitations: [M1 evidence](../../evidence/CULLING/M1.md#clean-copy-rejection-12-november).

- 2026-11-12 Bounded repair candidate: the shadow caster position lacked WGSL's invariance
  guarantee, while the forward position already requests it. Both caster entry points now request
  invariant position arithmetic. This closes that guarantee gap; the uncaptured historical failures
  are not attributed to it. Accept only with unchanged exact baselines and fresh named-target proof.

- 2026-11-12 The broad invariance candidate moved two retained mirror-light pixels, so it is
  rejected by M1.5. Traced that valley overlay to Grid/flat casters and scoped the guarantee to the
  mesh `member_vertex` required by M1. The scoped candidate must pass a fresh exact matrix; no
  retained image or oracle changes. Broad-candidate evidence is preserved.

- 2026-11-12 Scoped mesh-caster invariance is qualified at source a935334e/key845dd21d:
  all five pinned M1 commands and ./dev check pass; the full 1,930-tick field is equal with
  1,918 halved-list ticks; exact native/surface and four Chrome baseline sets pass unchanged.
  Fresh required mutants fail at pixel equality and the restored eight-test target passes.
  M1.2 passes and M1 is claimed for lead review. [Current evidence](../../evidence/CULLING/M1.md#current-repair-and-qualification).

- 2026-11-12 Review round 1 MF1 reopens M1.4: the old target-resize/fresh-device fixture did not
  exercise a running host. Added explicit plan work and a named-target witness using the actual
  100,513-instance authored mixed field, surface presenter, host resize and device publication.
  Focused Metal execution compares all next surface frames with independently prepared Direct;
  hiding renews after replacement and authority advances while the candidate awaits publication.
  Full committed-source qualification and reachable evidence are next.

- 2026-11-12 M1.4 now passes with fresh named-target Metal evidence from daylight source f7533d44.
  All 17 tests pass and the full exact verifier passes native 38+4 and four Chrome sets of 481 records.
  The running host retains the mixed field, mode and current authority through surface resize and device
  publication, with all 12 next frames equal to independent Direct and renewed hiding. Reachable logs,
  image hashes and native-size PNGs are retained in [M1 evidence](../../evidence/CULLING/M1.md#review-round-1-repair--mf1-host-lifecycle).
  Aggregate qualification and integration continue with M1 in progress.

- 2026-11-12 Set M1 ready_for_review once all five items pass, with committed evidence for each item.
  Mandatory aggregate verification remains live and integration will run the checked merge.

- 2026-11-13 Resumed under orchestrate 2.1.0 and restored M1 to `in_progress`, preserving reviewer fields.
  Fresh rebased qualification passes all five literal M1 checks and `./dev check`; the full release field test
  passes in 1,002.80 seconds, the bucket target passes 17 tests and the native matrices pass 38+4 comparisons.
  The full verifier also passes all four Chrome sets of 481 records each, with retained references unchanged.
  [Reports, source identity and complete compressed logs](../../evidence/CULLING/M1.md#orchestrate-210-continuation)
  separate these target results from the initial sandbox adapter failure. Required mutation production files
  remain unchanged; no new speed or elapsed-time motion claim. The script owns the post-merge state and the lead owns review.

- 2026-11-13 M2 implements capability-record-only camera selection and reports requested/resolved
  modes and the reason. [Evidence](../../evidence/CULLING/M2.md) proves Metal fallback, simulator
  Direct function and the 16-view Chrome mixed-mesh oracle. Two 1,800-tick paired 1080p runs favour
  Occlusion in cover and open views, selecting it for mixed meshes; cubes retain Frustum.
  The milestone checker passes all four automated items. Clean `3cf04848` passes native 38+4 exact
  comparisons and four Chrome sets of 481 records. The checked merge runs the full repository gate;
  M2 stays `in_progress` for the script's merged-work checks and the lead's independent review.

- 2026-11-13 Resumed M2 at `7dca0b33`: the preceding merge had rebased successfully but
  refused the missing exact-baseline proof. Two rebased timing runs again favour Occlusion
  behind cover and in the open. Fresh Chrome qualification exposed a DevTools disconnect
  when reading the whole Wasm response; `84403f5a` hashes the actual instantiation input
  inside Chrome instead. All four automated M2 items now pass, including 16 exact Chrome
  views and all registered baselines (38 + 4 native, four Chrome sets of 481 records).
  [Current evidence](../../evidence/CULLING/M2.md) retains identities, reports and the
  failed transport trace. M2 remains `in_progress`; integration and lead review follow.

- 2026-11-13 Final clean-source simulator samples progress from 117 to 957 presents
  with requested Occlusion, Direct fallback and the capability reason; the inspected
  screenshot and repeated Metal selection witness are retained. Merge rebased to
  `83e9150e`, bringing only reviews/evidence and APPLE-PRESENTATION's new volume test,
  then refused the new covered-content key. Production and benchmark sources are
  unchanged; refresh the exact verifier rather than repeat unaffected timing runs.

- 2026-11-13 The rebased exact verifier passes on clean `2f91255a` in 905.27 seconds:
  native 38 + 4 comparisons and all four Chrome sets of 481 records, with retained
  references unchanged. The current proof, complete verifier log and record are
  retained in [M2 evidence](../../evidence/CULLING/M2.md). Retry the checked merge
  with M2 still `in_progress`; no production source changed in this rebase.

- 2026-11-13 The full repository gate caught two mesh-route fixture assertions that
  compared an already prepared Direct oracle with a shared scene marked Frustum.
  Assert the resolved mode against each renderer's retained preparation request;
  this still rejects silent fallback. Both formerly failing Frustum bucket tests
  pass after the correction. Only the test helper changes; refresh the covered-key
  proof and retry the repository gate before merging.

- 2026-11-13 Clean `0b520c56` passes both repaired Frustum bucket tests and the full
  exact verifier in 891.16 seconds: 38 + 4 native comparisons and four Chrome sets of
  481 records. Retained the original gate failure, focused pass and fresh exact proof
  in [M2 evidence](../../evidence/CULLING/M2.md). The checked merge retries the whole
  repository gate with M2 left `in_progress`.

- 2026-11-13 Review round 1 reopens M2.2 for `simulator_complete_scene_unchecked`.
  The iOS render owner now captures its actual fallback host at the initial committed
  endpoint and an independently prepared explicit-Direct host with the same game input.
  The named simulator check compares every RGBA byte, validates both selections and
  retains both images alongside eight progressing presentations. A development run
  matches all 12,648,528 bytes at 1206×2622. Qualify committed source and demonstrate
  rejection of an actual omitted-geometry draw before marking M2.2 pass. Corrected
  the ledger's timing medians to the two retained M2 runs.

- 2026-11-13 Clean `1ed70fa7` qualifies the review repair. The simulator matches all
  12,648,528 RGBA bytes against explicit Direct and presents eight progressing samples.
  The actual omitted-geometry draw fails by 1,158,221 pixels. All four M2 automated
  checks and iOS Clippy pass, including native 38 + 4 exact comparisons and four Chrome
  sets of 481 records. [M2 evidence](../../evidence/CULLING/M2.md) retains the reports,
  raw readbacks, stills and mutation patch. M2.2 is pass, and M2 is `in_progress`.
