# EFFECT-GRAPH log

- 2026-11-11 Planned from the owner's task, quoted in the design, and the brief in the module list. Three
  milestones: texture kinds in the graph (after `APPLE-TILE:M2`), persistent state and cadence through a diagnostic
  effect, and the stage after opaque (after `APPLE-TILE:M3` and `APPLE-PRESENTATION:M2`). The port's graph condition
  is now met at M3, and the module list and the `SEA` and `CLOUDS` briefs say so.
- 2026-11-11 Planner's decisions: effects extend the existing contribution seam, and a scene holds an ordered list
  of contributions; cadence is a runtime skip under one compile-time rule, not a recompiled graph; state commits
  only with a submitted frame. Researched: Unreal's RDG, Unity's render graph, WebGPU's limits and storage-texture
  access. The storage-texture count and view rules were not re-read from the specification; M1.3 records them.
- 2026-11-11 The heavy checks (release build, GPU, Chrome) were not dry-run by the planner, for lack of disk and
  because the GPU is shared. Each check was parsed as shell and the done-when file validated with the loop's own
  validator.
- 2026-11-11 [Design review](../../../.orchestrate/evidence/EFFECT-GRAPH/design-review.md): changes requested, four Must Fix. MF1:
  the tick rule reset state on a repeated tick. MF2: a skipped ping-pong writer left readers on the older side, and
  time-sliced writes had no initialisation. MF3: M3.1 removed stored depth with the refraction request. MF4: M3's
  bar promised stability under reconstruction with no reconstructor in its `needs`.
- 2026-11-11 Design revised. MF1: a state table (equal tick keeps, later advances, earlier or a new timeline
  resets), and M2.2 asserts its rows directly. MF2: each history pair has its own committed side with `previous`,
  `next` and `result` bindings and flips only when its producer ran; three cadence classes; full frames on a new
  epoch and on reset; time-sliced passes may not write a history. M2.3's witness uses two histories and a consumer.
- 2026-11-11 MF3: depth storage follows the union of its readers and the colour copy its own request; M3.1 covers
  four cases. MF4: M3's bar and name now stop at reconstruction's inputs, and reconstructed appearance is judged
  in `SEA` and `CLOUDS`; `APPLE-PRESENTATION:M3` was not added to `needs`, so M3 is still built beside TAAU.
- 2026-11-11 Suggestions applied. S1: M2.5 names the cube, volume and array-mip readbacks in Chrome. S2: M2.7 uses
  the executor's named pass timings and leaves the host's sample fields to `APPLE-TILE:M3`. S3: list order is an
  execution order only; C08's segment admission stays with `SEA`, `CLOUDS` and V3.
- 2026-11-11 Planner's additions while revising: a multi-tick advance is one execution with a count, never a
  producer run twice in a frame; a same-generation `rebuild` keeps state unless the effect's descriptors changed.
  Exact baselines, from `BASELINE-GUARD`'s review: M2 gained `BASELINE-GUARD:M1` in `needs`, M2 and M3 carry the
  verifier, and M1 keeps the native matrix and does not wait (an assumption in the plan).
- 2026-11-11 M1 implementation checkpoint `3d56bf21`: explicit 2D/volume/cube descriptors, natural or declared
  binding views, exact-dimension transient allocation and capability-record validation. Existing declarations keep
  their 2D kind. [Partial proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M1-proof.md): all 14 Metal tests and the Chrome
  capability recorder pass; pre-M1 day/night/capture compilation is pinned unchanged. Clippy and check-docs pass.
  M1.5 tried its named release/cook/native matrix command and refused before capture with less than 16 GiB free
  (about 9.4 GiB available). The full repository check refuses the missing exact-baseline proof. M1 remains open,
  unclaimed and unintegrated; free disk with build/capture headroom, then rerun exact proofs and required checks.
- 2026-11-11 The clean-checkout registered exact verifier was also attempted under the GPU lock and refused
  `LOW_DISK_SPACE: verifier requires 16 GiB free`, even after removing owned debug outputs. Saved its refusal
  with the M1 proof and cleaned the remaining owned build products. No integration was attempted without the
  required exact proof; M1.5 stays open.
- 2026-11-12 Resumed M1 with disk headroom and rebased onto main. Both native exact matrices pass
  (38 plus four comparisons), and all four Chrome baseline sets pass (481 records each), under the GPU lock.
  [Target proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M1-proof.md) records the source and artifact identities. All five
  items pass; M1 is claimed for review. The required milestone check passes all five named commands and `./dev check`.
- 2026-11-12 The required repository gate exposed eight existing Hi-Z consumers rejected by the new
  view validation. Their logical pass aggregates successive single-mip views in one binding slot. Corrected
  the validator to preserve that contract and keep explicit storage views single-mip; added the aggregate
  range witness and tightened the explicit-view refusal. All 242 renderer library tests and all-target Clippy
  pass. Fresh exact proofs and the complete milestone check pass before integration.
- 2026-11-12 M2 implementation checkpoint: ordered public contributions, named simulate declarations and cadence
  refusals, source-clock frames, independently committed histories and wrapping anchors, and an opt-in integer
  diagnostic with native/Chrome qualification entry points. Pinned native all-target Clippy and Wasm Clippy pass.
  CPU state-table and cadence-refusal checks passed before the Metal scene-snapshot witness was added. Target
  scripts, realtime trail inspection, exact baselines and integration remain unqualified; no M2 item is claimed.
- 2026-11-12 M2.6 and M2.7 have [final-source realtime evidence](../../../.orchestrate/evidence/EFFECT-GRAPH/M2-proof.md).
  The builder inspected the complete sequence and native-size consecutive crops: the trail persists and fades,
  stays on the ground through the wrap, and does not jump at the cadence change. Named executor times, descriptor
  bytes and load are recorded alongside the no-effect arm. Six automated item checks remain in progress under
  their GPU locks; the milestone is not yet claimed for review or integrated.
- 2026-11-12 M2 is ready for review. All six immutable item commands pass on engine source `41357d2f`
  (checked commit `e1d8fe4b`), including both native recipes, the Chrome state script and all registered exact
  baselines: 38 plus four native comparisons and 481 records in each of four Chrome sets. The separately
  GPU-locked `./dev check` passes. [Target proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M2-proof.md) includes the literal
  oracle inspection, actual device replacement, unchanged cadence pipeline count, realtime trail inspection,
  named costs and load. These verify function and captured content; no display-cadence or speedup claim is made.
  The lead starts the independent review after scripted integration; no reviewer was started by this builder.
- 2026-11-12 Scripted integration encountered newly landed APPLE-PRESENTATION M2. Rebased while preserving
  its stored temporal inputs, history and browser case, and combined both modules' named-timing exclusions.
  M2 is temporarily reopened while target checks, realtime costs and exact baselines are refreshed on the combined
  source. Earlier evidence remains historical proof of the pre-rebase source, not a claim about the combined build.
- 2026-11-12 The first rebased Chrome run caught the diagnostic shader's missing third opaque output after
  APPLE-PRESENTATION added reactive coverage. Added its reactive output and the declared invalid-motion sentinel;
  this diagnostic does not claim M3 motion correctness. Stopped the obsolete verifier before changing source.
  The failed Chrome report is retained in scratch; all final proofs will run again on the corrected source.
- 2026-11-12 The combined-source repository gate found `synthetic.rs` four lines over the size backstop.
  Moved effect-pass recording unchanged into the existing contribution module. Repository policy and native
  all-target Clippy pass after the split. Fresh target, capture and exact proofs will pin this final source.
- 2026-11-13 Requalified M2 on final branch source `ec9dd79c`: all six automated item commands and the
  separately GPU-locked `./dev check` pass. The [refreshed target proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M2-proof.md)
  retains the native/Chrome oracle results, all exact comparisons, 964 reviewed realtime images and named costs.
  All eight items pass; M2 stays `in_progress` for scripted integration and the lead's review. No M3 work was built.
- 2026-11-13 Resolved the integration rebase by retaining the upstream browser checker's default build alongside
  both modules' cases. Chrome state/recovery and all exact baselines pass again on `c9ebc788`; the
  [source diff and proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M2-proof.md) identify the unaffected native/capture evidence.
  Upstream production renderer and diagnostic code did not change. M2 remains `in_progress` for the scripted gate.
- 2026-11-15 Planner, after M3's builder ended `BLOCKED needs_dependency` and named `APPLE-TILE` (handoff on
  `build/effect-graph-m3` at `baeb6d1f`). Read the two retained frames: the 12,637 pixels of Chrome night
  scene-linear record 1 lie along the far hills' triangle edges, the load variation `CULLING` M3 corrects, and
  `APPLE-TILE` M10 met the same without M3 ([reading](../../../.orchestrate/evidence/EFFECT-GRAPH/M3-replan/README.md)). So
  `APPLE-TILE` owes nothing and no dependency was added. [The module list](../ARCHITECTURE.md#a-frame-that-varies-under-load)
  gained a ruling on setting such a run aside, the design a paragraph, and M3 the item M3.9. M3.1 to M3.8 are
  unchanged. The plan file took the current shape, without task lists. No GPU check was run for this revision.
- 2026-11-15 Builder resumed the revised plan on the failed run's exact content key. The unchanged verifier
  on `f5b26a9e` passed both native matrices and all Chrome day SDR records, then its day scene-linear capture
  stopped at the visibility guard before comparison ([record](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/capture-interruption/README.md)).
  Independently regenerated and inspected the original night-linear difference: its identities, matching bytes
  and triangle-edge pattern meet tests 1–3 ([inspection](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/night-linear-failure/builder-inspection.md)).
  Test 4 still needs a complete same-content verifier pass; no failed image run is set aside and all nine
  builder flags remain open. Native/Wasm preparation, all-target Clippy and documentation checks pass.
- 2026-11-15 Audit found and validated a complete unchanged `f5b26a9e` verifier pass on the original
  failed content key, completing that night-frame record. The retry on `bd3468d5` failed a new Chrome day
  SDR image at ordinal 25. All 481 identities and hashes verify; only 528 opaque hill-edge pixels differ,
  at most two byte values, with matching lossless control bytes held. [The run audit](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/variation-runs.md)
  retains both records and inspections. The new failure needs a later complete same-content pass; no
  reference or comparator changed, no new failed run is set aside, and all nine flags stay open.
- 2026-11-15 The unchanged verifier on `58a6a0bb` passed both native matrices and all four 481-record
  Chrome sets on the original content key, completing the day SDR ordinal-25 ruling as well as the night
  record. [The audit](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/variation-runs.md) retains both failed runs and their
  lossless controls; the new complete pass's comparisons, identities, digests and hashes were validated.
  Integration-base checks, fixed native crossings, paired costs and the full gate remain outstanding.
  Resolved failure bulk was released after byte-for-byte retained-root verification.
- 2026-11-15 Integration preflight against main's new shape rules found M3's public string errors,
  duplicated device requests and milestone-coded capture script. Added typed quality/diagnostic errors,
  reused the existing diagnostic request sites, moved shared native support to normal modules and renamed
  the capture tool by purpose. Two CPU tests prove typed refusals preserve valid staged values.
  Proposed for HOST: own the shared device-request recipe; until it lands these witnesses reuse their
  existing example request policy. Fresh target and exact qualification remains required after integration.
  Pinned native/Wasm Clippy and both CPU tests pass; preparation source hashes and output are retained.
- 2026-11-15 Rebased M3 onto main, preserving both maturity rows and current upstream handoffs/logs.
  Planner-owned DONE fields match main. Tightened only the relocated native-request allowance and two
  removed path attributes. Main's historical APPLE-PRESENTATION log links named unmerged artifacts;
  verified them on its owner branch and kept their paths inline until that evidence lands.
  Fresh combined-source checks, captures, costs and the normal merge gate remain required.
  Combined-source workspace all-target and V1 Wasm Clippy pass, with the compile revision retained.
- 2026-11-15 Independently regenerated both failed-frame differences, inspected whole images and densest
  native-size crops beside failed frames, and revalidated all 481 identities per failure, retained
  source/control hashes and both complete same-content pass archives. Promoted the denser night crop
  at (114, 230). M3.9's builder proof passes; [the audit](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/variation-runs.md)
  retains both failed records. Documentation checks pass. Fresh integration target checks and merge
  remain pending while the existing campaign waits for the shared GPU; no target result is inferred.
- 2026-11-15 Privately rebased onto main `3042de93c`, integrating APPLE-PRESENTATION's TAAU path.
  Kept upstream capture/pre-tone helpers and reconstruction capture controls. Late effects now precede
  TAAU as well as native consumers. The graph witness selects actual reconstruction on/off, and the
  late compute witness checks its final epilogue before TAAU. Native workspace all-target Clippy,
  rustfmt and documentation checks pass. Fresh target proofs remain queued in the existing campaign.
  Release native/Web builds and pinned native/Wasm Clippy pass at `7aea52c74`, with source/build
  hashes and repaired preparation failures in [the record](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/reconstruction-integration-preparation.json).
- 2026-11-15 The first combined-source M3.1 target run reached late compute readback and failed
  because TAAU requires the input diagnostic to be selected explicitly. The fixture now selects
  `DisplayOverlay::TemporalInputs` when it needs that reader, retaining actual TAAU and ordering checks.
  Pinned native/Wasm Clippy pass. [The retained failure](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/reconstruction-witness-failure.md)
  records the target result. Fresh qualification remains required for M3.1–M3.8.
- 2026-11-15 Fresh target qualification at `8134e6973` passed every M3 check, the affected
  M2 Chrome recovery consumer, the complete unchanged exact verifier and the official full milestone
  check including `./dev check`. Inspected all five fixed whole frames and native-size crossings.
  Retained twelve paired cost arms with load; transfer-only cost and reconstructed motion are unclaimed.
  [Target proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M3-proof.md) separates checks, visual judgment and costs.
  All builder items pass; normal integration follows this evidence checkpoint, with review owned by the lead.
- 2026-11-15 The later official merge gate at `99e9fb125` failed the cooker's generation-2
  watcher deadline. The exact workspace binary then passed three focused reruns and all four suite
  tests unchanged. [Diagnosis](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/watcher-retry/record.json) retains the
  failed gate and reruns; the cause is unconfirmed. No cooker source changed. Target proofs remain
  valid because integration brought only plan/status changes. A fresh normal merge gate is required.
- 2026-11-15 Resumed from the retained target proof. Cancelled the old GPU-queued merge wrapper
  before it started; current project rules require the official merge without an outer device lock.
  M3.8 now cites the shared main-branch exact-images check, as those rules require. Historical full
  verifier passes and both variation audits remain retained. No engine or fixture source changed.
- 2026-11-16 The script's clean-checkout M3.5 run passed slab readback but lost one HTTP
  frame upload. Added bounded replay of the same submitted bytes and strict receiver deduplication.
  Four transport regression tests, 17 T2 tests and documentation checks pass. The affected Chrome
  check passes all 127 fixed frames; its selected slab frame matches the retained image exactly.
  [The failed and passing records](../../../.orchestrate/evidence/EFFECT-GRAPH/M3/upload-recovery/README.md) retain source identities;
  the socket reset's origin is unestablished. Normal checked integration follows this correction.
- 2026-11-16 Planner: M4 added for the display phase, on the lead's task after `CLOUDS` M2 ended
  `BLOCKED needs_dependency` ("settle and assign the shared display-time interface change"). Alpha stops at the
  committed tick, so under a display lead `C − 1 + α` is not the display's time. Each frame now gives effects the
  unclamped phase; alpha and authority do not change. `HOST` was the builder's other candidate and waits on seven
  milestones. [Design](DESIGN.md#the-display-phase-wp4). M3's builder note and stop rules left the plan with M3 Done.
- 2026-11-16 Planner: M4 revised after its design review's one Must Fix (MF5), with `CLOUDS`. The phase
  describes the consumed target, the observation plus the lead, so alpha stays the phase held to 1 in every
  frame and an expired target has one meaning. `Offscreen::step_for` reads the clock once; the window hosts are
  left as they are. A prediction is carried a tick at a time. M4.1 to M4.4 rewritten; the record gains the
  observation, the lead's status, the platform target and whether the clock interpolated.
  [Check](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-plan/consumed-target.txt). S5 taken; S4 left, since M2 is Done.
- 2026-11-16 Implemented the phase and one-read offscreen observation; normal Mac target scripts pass,
  including 144 effect frames and 32 realtime records. Native/Wasm Clippy and iOS compilation pass.
  A new submitted-frame counterexample at `5a8f26002` fails M4.2: after render preemption the world
  supplies tick 2 for observation 42.417 us, with alpha/phase zero and interpolation true, a 33.291 ms error.
  Proposal: the planner must settle observation-coherent endpoint selection or a revised consumption policy;
  either changes more existing frame behavior than the current design permits. No merge was attempted.
  [Handoff and reproduction](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-handoff.md) separate passing scope from the failure.
- 2026-11-16 Resumed M4 after the owner's instruction to carry on. The lane now enforces the design's
  existing `t_C <= observation` premise, retaining its previous pair if a preempted poll sees a future endpoint.
  Sampling formulas, consumed target and authority are unchanged; no history or clock read is added.
  Four core integration tests and both Mac GPU checks pass, including the old counterexample and a real
  completion-callback stall inside the offscreen driver. All 32 captured frames satisfy the target identity.
  [Updated proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-proof.md) retains source identities and the historical failure.
  Exact images use the shared main-branch run; visual prediction quality and physical cadence remain unclaimed.
- 2026-11-16 Planner: M4 re-planned on the lead's task after its first review failed M4.2 and the builder ended
  `BLOCKED needs_context` ("Decide the policy, update the design and done-when file so they agree, and name
  the affected consumers"). "No frame without a lead changes" could not hold beside M4.1 and M4.2. Policy: a
  frame shows a tick due by its observation, and the bar says which frames change. The built lane's one held
  pair stalls in a run of delayed frames, so it also holds the refused pair. Bar, M4.2 and M4.4 rewritten;
  M4.6 added. [Design](DESIGN.md#the-frames-pair), with who meets a delayed frame ·
  [probe and the choice not taken](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-replan/README.md).
- 2026-11-16 The resumed builder reproduced the review's zero-lead selection change with the actual
  clock on `8a72a0c0b`: tick 2/alpha 0 becomes tick 0/alpha 0.06 after a delayed observation.
  Keeping tick 2 requires phase -1.94 to satisfy M4.2, contradicting M4.1's nonnegative phase.
  Proposal for the planner: retain causal endpoint selection and explicitly settle its zero-lead
  compatibility exception, affected threaded consumers and regression proof in the design and criteria.
  No engine or criteria changed; M4.2 stays open. [Reproduction](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-compatibility.md)
  and [handoff](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-handoff.md) retain the conflict; no new GPU claim or merge.
  The CPU probe and documentation checks pass; owned probe scratch was removed.
- 2026-11-16 Implemented the revised pair rule: the lane retains the refused clock/page pair, so repeated
  delayed polls progress while authority continues and completed ticks are reported once. Updated C02,
  public effect documentation and the separate implementation/correctness ledger claims.
  Five core tests and both Mac Metal frame checks pass at `fbb4115cc`, including 12 delayed polls,
  144 effect frames and 32 realtime frames with no dropped opportunity. The capture includes an expired
  target and a 45 ms render-owner delay. [Proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-proof.md) retains the results.
  Cost is at most one additional leased page; physical cadence and consumer prediction remain unqualified.
- 2026-11-17 Planner: M4 re-planned on the lead's task after its second review failed M4.2 on a resumed frame
  and the builder ended `BLOCKED needs_context` ("write the whole timing model for M4 (origin, phase, lead,
  delayed observation, pause) as one consistent set"). The plan had written display time as host time; it is
  world time. [The timing model](DESIGN.md#the-timing-model) states spans, the hold, the pair, the phase,
  display time and each bounded step at a span's edge together, and a frame's record gains its tick's due time.
  C02 rewritten with it; `CLOUDS`' design corrected in three places, its items unchanged. M4.2 to M4.4
  rewritten; M4.7 to M4.10 added: spans, the lane through a resume, resumed frames, the sky world's stand-in.
  [Check on the repository's clock and lane](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-timing/README.md).
- 2026-11-17 Confirmed the second review's M4.2 origin failure on six submitted Mac Metal frames at
  `8ea0cd27d`, through the public V1 pause/resume path and the actual effect time. Leads 0, 10 and 50 ms
  give a 1.000 s identity error; a partial-tick pause adds 8 ms discarded by pause. Alpha, phase and
  authority preserve their existing semantics. No implementation or planner/reviewer field changed.
  Proposal: the planner must define a clock-origin mapping, preserve raw target records, and revise the
  identity and its record semantics together. [Evidence](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-resume/README.md)
  and [handoff](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-handoff.md) retain the contradiction and affected producers.
- 2026-11-17 Resumed at `605cc0329` and checked origin/main `a590c33b3`; M4's design, plan, criteria
  and C02 have no intervening planner changes. The retained actual-clock probe again confirms a 1 s
  M4.2 identity error after resume for leads 0, 10 and 50 ms. No GPU qualification was repeated.
  No implementation or criteria changed; M4.2 remains open pending the time-origin contract decision.
  [Rewritten handoff](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-handoff.md) records the reproduction and next action.
- 2026-11-17 Implemented the reviewed timing model: clock-sample due times now reach every world adapter,
  completed frame and capture record; the ticking sky states its own phase with bitwise unchanged alpha.
  The clock's phase, alpha, authority and refused-pair selection are unchanged. Three span tests, five
  phase tests, the sky regression and all three Mac Metal checks pass. Submitted lifecycle arms retain
  22 omitted delayed opportunities and validate 24 submitted frames per threaded arm; capture records
  include a stop, resume and origin change. [Proof and identities](../../../.orchestrate/evidence/EFFECT-GRAPH/M4-proof.md).
  Numerical Mac correctness is separate from visuals, device speed, consumer prediction and panel cadence.
- 2026-11-23 M5.1 steps the original three gap and pause cases through the resident world lane on simulated
  time. The Mac nextest run verified the original 2 µs origin checks, the hold before the first resumed tick
  and the delayed observation in both spans. The test took 0.053 s alone and 0.058 s in the first focused batch
  (11 passes), against 6.213 s in the plan's workspace run.
  [Timing and coverage](../../../.orchestrate/evidence/EFFECT-GRAPH/M5/M5.1.md).
- 2026-11-24 M5.1 now maps the worker's resume and advance samples and the caller's bracket through
  the same clock offset. The reviewer's 25 ms probe failed at the acknowledgment identity, with
  resumed tick 3 and paused tick 2. On the Mac the revised check took 0.018 s alone and 0.017 s in
  the first focused batch, which passed all 11 tests.
  [Review fix and proof](../../../.orchestrate/evidence/EFFECT-GRAPH/M5/M5.1-review-fix.md).
