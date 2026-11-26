# APPLE-PRESENTATION log

- 2026-11-14 At `9b5e5ef4`, unconditional current-source reconstruction passes
  five Metal still checkpoints but ties no AA at night in two runs; every negative
  and cut control passes. All ten Mac arms complete, but builder and a fresh Sol
  medium reader still find reveal speckling. The next bounded kernel preserves
  the raw source with usable history and adds variance clipping; target proof is pending.
  [Results and next experiment](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/variance-candidate.md).

- 2026-11-14 CPU analysis of the saved `0c3b1bc6` reference failure locates 4,298
  changed components in a narrow night terrain/horizon band. Both f32 means and
  the locator are retained; the cause remains unconfirmed and the bound is unchanged.
  [Failure evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/reference-failure/README.md).

- 2026-11-14 M3 resumed at `acc6a02b`: its traced Metal reconstruction and all ten
  Mac capture arms complete, but builder and fresh-reader native inspection still
  find reveal-edge speckling. Candidate `008fee6e` adds current-source reconstruction;
  WGSL validation, native Clippy and Wasm build pass, with device proofs queued.
  No item or integration is qualified by those compile checks.
  [Current experiment and retained target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/current-filter.md).

- 2026-11-14 M3 continuation confirms that reciprocal-only `18a20549` passes its
  Metal depth probe and cut controls but fails noon's strict p99 improvement
  (1.406131 versus no-AA 1.369814). The surface-tap experiment `ec453438` passes
  shader validation; its Metal reconstruction run waits behind SEA's nested
  GPU lock, confirmed by a stack sample of checker PID 4501. No current motion,
  timing or integration pass is claimed. All M3 items remain open and M3 remains
  `in_progress`. [Experiment evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/README.md)
  and [continuation handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/M3-handoff.md).

- 2026-11-14 The range-based candidate at `959ada11` passes all 1,966 exact
  comparisons in 939.25 seconds. The observed reveal fringe remains a failure of
  the visual bar. A bounded candidate now interpolates reciprocal view depth
  instead of accepting the whole foreground/background depth interval; shader
  validation, strict renderer Clippy and `./dev check-web` pass. A test-only Metal probe includes
  a projected plane and the former range test as a negative control. GPU and
  visual execution are pending, so D3.2, D3.4 and D3.5 are reopened alongside
  both motion items. [Hypothesis, cost and runner](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/README.md).

- 2026-11-14 Chrome's ten real-time arms pass capture/packaging on `3f896417`.
  The CPU audit verifies all 2,696 original frames. A fresh GPT-6.1 Sol medium
  opinion and the builder find a fine ragged fringe on revealed blocks and the
  moving cube, also visible on Mac. Both motion items remain open for repair;
  stable star-core traces and zero-lag scrub fits are supporting diagnostics,
  not substitutes for the visual bar. [Target observations](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/motion/chrome-review.md).

- 2026-11-14 D3.5 passes at `3f896417`: the paired 640×360 moving route on
  Metal has 361 matching-input samples per arm, TAAU resolve p50/p95
  0.046709/0.049792 ms and scene p50 2.994001 ms on versus 2.778083 ms off.
  Build, arm order and shared-host load are retained. The 0.420381 ms pixel-ratio
  heuristic is within the 1080p estimate, with no measured 1080p or cadence claim.
  [Target timing](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/README.md#moving-route-timing).

- 2026-11-14 Resumed at `8fa5592d` and retained all ten Mac motion arms, native
  source crops and independently decoded all-frame star/scrub diagnostics. The
  scrub trace's best-fit lag is zero milliseconds; sampled spawn/reveal frames
  show no persistent silhouette. Direct 1× visual observation has not occurred,
  so D3.3 stays open. Documentation checks pass; fresh route, Chrome, timing and
  integration qualification remain pending. [Inspection limits](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/motion/motion-review.md).

- 2026-11-14 D3.2 and D3.4 pass on Metal at `c3d052a4`: all six full-frame p99
  values improve on no AA and remain below 3; both linear reference cycles agree
  bit for bit, all history-disabled controls are rejected, and cut/reset equality
  plus the suppressed-reset control pass. Strict Clippy passes all native workspace
  targets after preserving the field benchmark's explicit reconstruction-off setup.
  Motion, actual moving-route timing and integration remain unqualified.
  [Target proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/README.md#reconstruction-and-cut).

- 2026-11-14 Independent GPT-6.1 Sol medium research separates the depth-rejection
  failure from night's broad one-code green-channel p99 plateau. It finds no evidence
  requiring a foundational design change and recommends evaluating the queued
  half-float correction before another runtime edit; if it fails, capture the
  pre-tone/history/validity values at plateau pixels. The reference and firm bound
  remain unchanged. [Diagnosis](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/reconstruction-research.md).

- 2026-11-14 M3's strengthened reference at `9c1349f2` proves bit-exact repeated
  linear means and both cut controls, but four still comparisons fail. A diagnostic
  without depth rejection improves five of six checkpoints. The next implementation
  retains rejection across the complete reprojected depth footprint and introduces
  unbiased half-float history rounding; WGSL validation passes, with target quality
  still pending. [Second attempt](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/attempt-2/run.json).

- 2026-11-14 The first M3 Metal reference run at `55ae06ee` fails the firm no-AA
  comparison at all six checkpoints, despite meeting the absolute ΔE 3 bound.
  Reference-cycle equality and both cut controls pass. Follow-up `5cffce8c`
  corrects jittered depth addressing, steady-colour blend drift and retained GPU
  storage-scale use; Clippy and shader validation pass, with target rerun pending.
  [Retained failure](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/attempt-1/run.json).

- 2026-11-14 M3 implementation checkpoint adds the portable temporal resolve, shared
  submitted history, pre-tone integration, an independent linear reference accumulator
  and the submitted game-host oracle. Focused native Clippy, shader validation and
  pinned Wasm compilation pass. Target image quality, motion, timing, full checks and
  integration remain unqualified. [Builder evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/README.md).

- 2026-11-14 M3 preparation follow-up separates continuous camera edits from cuts,
  samples local lights from the same game endpoint as the view, and adds an opt-in
  reconstruction witness with an authoritative tick-60 prop replacement. The
  real Metal alternating-slot/reset execution test and both CPU fixture tests pass.
  Native/Chrome motion drivers now expose the five required scenarios; their
  output quality, the full reference oracle and integration remain unqualified.

- 2026-11-10 Revised after [the design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) at `4bbe44a4`
  (changes requested, one Must Fix), under the task "Revise APPLE-PRESENTATION's design and plan as
  .orchestrate/evidence/APPLE-PRESENTATION/design-review.md asks".
  [Full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-1).
- 2026-11-10 Moved onto the loop from [the module list](../ARCHITECTURE.md). M1–M3's done-when lists became items
  D1.1–D3.5 in the done-when file with the same meaning, each with a proof. Two items were added so Chrome is
  proved as its own target: D2.5 (the browser checker still passes with the third target) and D3.3c (the motion
  review in Chrome, split from D3.3). The done-when file is now the authority for M1–M3; the lists below remain as
  the builder's work breakdown. The module list accepts seams 1–4 as
  the design proposes them, so M1–M3 no longer wait on a separate ruling. The atmosphere checkpoint `d2a5342` is on
  main. M4–M11 stay in prose here and are migrated by the next plan run, which restates the sizing.
- Superseded on 10 November by the assumption above: M1–M3 keep the provisional reconstruction tolerance the design
  states (ΔE p99 at most 3, and below the no-AA path), restated after M3's first run as the design says.
- Status before this migration (7–9 November): Milestone: none started (plan awaiting re-review and seam rulings) ·
  Updated: 2026-11-07 Now: revision 5 replaces the archive options with the owner's direction (since 2026-11-07).
  Next: review of revision 5, Thrum Core's reading, then landing and seam rulings.
  [Full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-2).
- Reviews before the loop: [full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-3).

- 2026-11-09 Scoped APPLE-TILE re-plan consumer reconciliation only: G3t M3 now supplies a pre-tone colour/exposure
  seam while retaining SDR shaders/intermediate. G4 owns its EDR branch/filtering and later arithmetic qualification.
  G3t M4–M9 are removed, leaving M10/M11 native providers and M12 integrated proof; revised dependency table only,
  not a full G4 re-plan or acceptance. Preserve concurrent COOKED-DRAW ownership in draw.rs. Independent design
  review of these handoffs is owed to the loop.

- 2026-11-09 Reconciled G3t producer handoffs only: G3t M11 uses bounded extent fixtures and M12 fixed extents,
  removing reverse G4 prerequisites. G4 owns requalification with real reconstruction readers/live scale steps.
  Native acquire refusal now has one planned outcome, discard and provider reset; healthy flush, lost abandon and
  iOS background refusal remain distinct. Independent interface review is owed; other G4 design gaps/status remain.

- 2026-11-07 Revision 1 drafted for Thrum Core's G3t/G4 commission.
- 2026-11-07 Revision 2 answers the G4 pair review and the composed-frame review:.
  [Full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-4).
- 2026-11-07 Revision 3 answers the G4 re-review:.
  [Full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-5).
- 2026-11-07 Revision 3 also answers the composed-frame re-review:.
  [Full entry](../../../.orchestrate/evidence/APPLE-PRESENTATION/log-entries.md#entry-6).
- 2026-11-07 Revision 4 answers Thrum Core's review of revision 3:
  - M1's witnesses all run through the current portable output before G3t M3, including a motion mode in the
    existing display pass;
  - the history protocol keeps the existing drop contract; MetalFX frames follow G3t seam 3, and both outcomes and
    the iOS background refusal are observable (D8.6);
  - archives are an owner ruling, with no V1 exit wording change;
  - the atmosphere checkpoint is `d2a5342`, for interface reads.
- 2026-11-07 Revision 4a applies the composed-frame recheck of `9938b0d0`: the leftover always-submitted rule is
  replaced, MetalFX history advances at encode, and D2.4 covers the portable drop and the iOS background drop.
- 2026-11-07 Revision 5 follows the owner's archive direction (overseer, 22:20):
  - the (b) rewording option is withdrawn, and V1's archive item stays as written;
  - M9 starts with a measured cold/warm split of launch-to-ready into naga, MSL compile and pipeline state creation;
  - M10 delivers offline metallibs through `PASSTHROUGH_SHADERS` and a Metal `PIPELINE_CACHE` carried as a pinned patch
    and contributed upstream under gfx-rs/wgpu#8526, with the contribution itself waiting for the owner's approval;
  - sizing is restated.
- 2026-11-07 Revision 5a applies the archive review of `16c5d8f7`: per-pipeline cook keys, `MetalLib` reflection for
  the sizes buffer, the full Metal cache patch scope and per-device archive sources. M10 becomes 5.5 days.
- 2026-11-07 Revision 5b adds the overseer's 22:30 points: parallel pipeline pre-warm (D9.4), the cook's MSL match
  check against wgpu-hal's runtime MSL, and the order measure → metallibs plus pre-warm → Metal `PIPELINE_CACHE`.
- 2026-11-07 Revision 6 moves the upstream Metal `PIPELINE_CACHE` contribution out of M10 into U1, a later item that
  does not gate V1, with the owner's rules (coordination first, Metal only on trunk, GPU-suite tests, `cargo xtask
  test`, review, owner walkthrough, owner-written posts). M10 keeps the pinned patch for V1's archives.
- 2026-11-07 Revision 7 follows the owner's direction to build the archive work when G4 reaches it: seam 5, M10 and U1
  are kept at outcome level, with the four pieces distinct and Root's fe1749c1 points as one line each, to be designed
  when M10 starts.

- 2026-11-11 Plan reshaped for orchestrate 1.7.0. Status lines and the copies of M1–M3's criteria removed (the
  done-when file owns them). `Game outcomes`, `Evidence conventions`, `Ownership, call sites and review order`
  and `Parallel work and dependencies on G3t` moved to the design. The M4–M11, U1 and sizing sections moved to
  [drafts](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md); the plan keeps a heading for each, renamed
  for what someone can see. Open assumptions moved from this log to the plan. No item or scope changed.

- 2026-11-11 M1 builder implementation and mac-metal proof completed: continuous submitted 32-phase jitter,
  unjittered geometry/infinity motion, explicit invalid correspondence and the existing-pass motion view.
  Real Metal attachment tests prove exact static zero and independent pan/solar/sidereal values. Fixed-time
  submitted valley sequences and native crops were inspected, including a real simulation mover. Repository
  gate and milestone checks are recorded separately from visual self-review; no performance or G4 closure
  is claimed. [Builder evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m1/README.md); lead review remains owed.
  Integration recheck: all 13 isolated cooked draw-probe cases pass on the identical pinned binary; the final
  gate serializes cases with explicit concurrency witnesses retained. Details are in the builder evidence.

- 2026-11-11 MF1 repair gates raster jitter on explicit temporal sampling while retaining submitted metadata.
  Metal tests prove consecutive paused colour byte-equality, advancing phases and a jittered positive control.
  Held-camera comparisons at 640×360 and 1920×1080 show stable sphere, skyline and ground edges. Current
  motion sequences were inspected. Independent re-review and performance qualification remain owed.
  [Repair evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m1-mf1/README.md).

- 2026-11-11 M2 implementation checkpoint: added the third forward reactive target, opt-in stored temporal inputs,
  GPU exposure texture and submitted-frame diagnostics; acceptance identities and output-grid keys now follow
  submitted frames. Metal attachment, exposure, populated-host history and cooked alpha proofs pass; Chrome,
  exact baselines and integration remain pending. The iOS simulator shell compiles with background cancellation
  resetting history. [Builder evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2/README.md). No M2 claim yet.

- 2026-11-11 M2 target qualification passes all 249 renderer tests on Metal and all 32 Chrome cases,
  including W1–W7 and the existing controls regression. Updated culling contribution fixtures for the
  third target and the controls check for the declared 0.05-degree step. D2.5 and D2.7 pass.
  The required checks for D2.1–D2.4 and D2.6 stop at the exact-baseline gate after the fixture repair.
  Its verifier was attempted and refused with LOW_DISK_SPACE (16 GiB required, about 10 GiB free after
  owned cache cleanup). M2 remains open and integration is blocked until a current proof can be made.
  [Target results and blocker](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-12 M2 resumed after disk recovery. Exact verification at `57590699`, rebased onto current main,
  passes 42 native comparisons and all four 481-record Chrome sets under the GPU lock (1,966 exact matches).
  References are unchanged; no device timing qualification is claimed. Documentation checks pass.
  The required milestone checker remains live; D2.1–D2.4 and D2.6 stay open until its result is available.
  [Current proof and target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-12 M2 builder verification completed on source `c506b2a6`. Resolved the main merge, retained CULLING's shared flythrough, supplied explicit graph descriptor fields and reconciled the temporal attachment snapshot. All seven item commands and the repository gate pass on Metal and hardware Chrome; exact verification passes all 1,966 comparisons. M2 is ready for independent review. Device performance qualification remains owed. [Current evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-13 M2 clean-checkout repair at `8224cda1`: default browser checks build their web bundle instead of relying on ignored output. The 3,600-second checker passes all seven items and the repository gate; hardware Chrome passes 32 cases and W1–W7, with identical Wasm and diagnostic image. Exact verification passes all 1,966 comparisons. The original watcher missed edit remains unconfirmed after 70 stress runs and a passing full gate; its code and assertions are unchanged. M2 stays `in_progress`. [Follow-up evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-13 M2 review repair `mac-volume-witness-bypasses-forward-pass`: at `56469803`, a new Mac game-host test reads W3 through the production cooked contribution, forward graph and submitted temporal diagnostic. Portable and Apple-tile each read alpha byte 161 and four uncovered zeroes. Disabling only the production reactive write makes the test fail with 0 versus 161; restoring it passes with byte-identical frames. Production code is unchanged. Strict focused Clippy and documentation checks pass; full checks and integration remain pending. [Fresh D2.6 proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2-volume/README.md).

- 2026-11-13 M2 follow-up verification at `cf0c424a`: the required checker passes all seven items and the repository gate; Chrome passes W1–W7 and 33 regression cases. The page hashes successful Wasm instantiation inputs and checks exact digest and byte length without oversized DevTools replies. Reconciled CULLING's concurrent checker change through the shared tested helper. Exact verification passes all 1,966 comparisons with references unchanged. Rebased onto main's later documentation and Mac qualification script as `e546d4ce`; refreshing its content-key proof and integration gate remains pending. M2 stays `in_progress`. [Retained proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2-volume/README.md).

- 2026-11-13 The rebased M2 checkpoint `7bcf2b02` passes all 1,966 exact-image comparisons in 889.44 seconds, under the GPU lock and without reference changes. Runtime and browser-checker source objects match the fully checked `cf0c424a`; the unrelated Mac qualification script required a new covered-content key. The required merge gate is next. [Integration proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2-volume/integration-exact.json.gz).

- 2026-11-13 M2 merged through `agent_loop.py merge` as `ff093ba7`; the repository gate passed and no upstream files came in. The post-merge milestone checker passes all seven items and the repository gate, including Chrome W1–W7 and all 33 regression cases with the same instantiated Wasm hash. Later main changes contain only CULLING plan records and STATUS.md, so they affect no M2 proof. M2 remains `in_progress` for the lead's checks and review. [Target and integration evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m2-volume/README.md).
- 2026-11-14 M4 to M12 planned, from the lead's task: "APPLE-PRESENTATION M4 to M11 have no done-when entries,
  so no builder can be given them [...]. Write their done-when items and plans so they can be built, with honest
  dependencies. Do not change APPLE-PRESENTATION M3". The drafts became items; the drafted M10 is now M10 and
  M12, and the order is M4, M5, M6, M7, M8, M9, M10, M12, M11. The design settles the EDR transform, the MetalFX
  exposure fallback, the generation build's lifecycle and the archive; the module list settles seams 5 to 9.
  Only M8 (`APPLE-TILE:M11`) and M11 (`APPLE-TILE:M12`) wait on another module. The owner's look at EDR is
  M11's last item. M1 to M3 are byte for byte as they were. The revised design needs its review; until it
  passes, the script holds the whole module, M3's next step included.

- 2026-11-14 M3 continuation: variance clipping at `0190d760` fails four Metal still checkpoints. Adaptive source at `97b0315b` and graded depth-footprint support at `e8ff99af` pass all six still comparisons and both cut controls. Exact current images, independent f32 means and held records are retained; D3.2 and D3.4 pass. Builder and fresh readers still note reveal-edge stipple, so motion remains open. A separately pinned filtered-source history-on/off diagnostic is queued; its explanation is not yet established. Native Clippy, WGSL validation, Wasm release builds and documentation checks pass; full qualification and integration remain owed. [Current target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/footprint-support.md).

- 2026-11-14 M3 route follow-up at `55b8c270`: the restored production source passes the moving route on Mac Metal and Chrome, with 361 matching tick identities and exact repeat checkpoints. Paired 640×360 resolve p50/p95 is 0.064833/0.067708 ms; its nine-times heuristic trips the 1080p estimate warning, without failing D3.5 or claiming measured 1080p cost. D3.1 and D3.5 pass with retained records. The controlled reveal shows stair steps persist without feedback and resolves several apparent trails into neighboring geometry; full production motion remains to be judged. All five paired Mac/Chrome sequences are queued under one GPU reservation. [Current route proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/current-route/README.md) · [Diagnostic](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/depth-reprojection/history-ablation/README.md#target-comparison).

- 2026-11-15 M3's complete production campaign at `b969d64e` captures all twenty arms. Fresh Sol medium readers find residual revealing-edge roughness and softened Chrome pan detail, so motion remains open. Rebased onto main, preserving native diagnostics and one target-preparation path; native compilation, strict affected Clippy and documentation checks pass. An analytic translation model motivates interpolating cubic history sampling; a quadratic-field control and full target qualification are next. Existing four passes are reopened for changed integrated source and cost. [Candidate and cost account](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/motion/history-filter.md).

- 2026-11-15 M3 continuation confirms successful native, Web and release-oracle prebuilds at `dc361d2b`, but no new target proof. After several shared reservations advance, a confirmed cross-builder lock cycle prevents GPU progress: CREATOR holds merge and waits for GPU, while CLOUDS holds GPU and waits for merge. Only this worktree's pending jobs are stopped for an evidence checkpoint; other builders are untouched. All six items remain open. The source audit and inspection checklist are preparation, not acceptance. [Diagnosis and build identities](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/continuation/lock-cycle.md) · [Refreshed handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/M3-handoff.md).

- 2026-11-15 The shared lock cycle clears by 02:48 UTC when the CREATOR process exits; CLOUDS proceeds into workspace GPU tests. M3's replacement qualification and CPU preparation are queued on unchanged renderer source. The historical diagnosis is retained, but it is no longer a reason to stop. No new target result is claimed.

- 2026-11-15 CLOUDS's subsequent model-code merge changes the blanket exact-baseline content key. M3 qualification is requeued to acquire GPU first, integrate current main with abort on conflict, rebuild and then collect proof on that source. This prevents older queued merges from invalidating the key before capture begins. The current cubic renderer is unchanged and remains unqualified. A CPU-only affine-source model identifies a possible filter-support bias, without establishing a game defect or selecting a replacement.

- 2026-11-15 M3 resumes at `e924fe69`: Metal helper probes, six still checkpoints and ten Mac motion arms passed; Chrome's spawn-off arm is incomplete with one export omission, so that campaign stopped before route timing and exact verification. Builder and a fresh Sol medium reader find no material reconstruction defect in the inspected Mac sequences, with residual small-edge roughness and a 70.871 ms spawn submission-gap caveat retained. Checkpoint `1248cac8` saves the actual inspected images and opinion. A replacement integrated-source qualification is queued under GPU; all six items remain open. The preparation documentation check fails only on old REPO-RULES links already repaired on main, which the queued rebase will import. [Recovery and current jobs](../../../.orchestrate/evidence/APPLE-PRESENTATION/M3-handoff.md).

- 2026-11-15 M3 integrates SIM-2D's main changes cleanly. At `dc7eb2c0`, documentation, strict affected native/Wasm Clippy and the official web compile checks pass. Explicit pinned tool paths correct a direct invocation that selected Homebrew Clippy's non-Wasm sysroot. The motion runner now compares source with its saved campaign commit, so earlier rebases cannot hide stale captures. GPU qualification remains pending and all six items stay open. [CPU result](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/continuation/qualification/cpu/run.json).

- 2026-11-15 Design revised after [the design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) asked for three changes. MF1: the owner's look at EDR no longer gates; D11.8 is now a recorded, non-blocking observation, and the design names the evidence G4 closes on. MF2: the pinned wgpu-hal copy moves from M12 to M10 with a first patch that carries stage records to passthrough stages (D10.2, new D10.6); M12 adds the archive patch. MF3: the governor's hold ends when a build ends, with or without a switch (D7.1, new D7.9). All three suggestions taken: readiness is per device generation (D9.2), the cost table names D5.7, and the normalising pre-pass clamps and counts (D8.3). M1 to M3 are unchanged. The design needs its review again.

- 2026-11-15 M3 at `4c0fd681`: all pinned target checks, twenty real-time motion arms and 1,966 exact baseline comparisons pass. Builder and fresh Sol medium readers find no material AA defect under ordered native-frame inspection; continuous playback perception and panel cadence remain unverified. Shared test device setup, subject-named proof scripts and consolidated scene reads satisfy current shape rules; strict native Clippy, official Web compile and documentation checks pass. Paired 640×360 resolve p50/p95 is 0.095167/0.104292 ms, with the 1080p heuristic and indicative regression recorded as tripwires. [Selected proof, limits and retained sources](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/selected/README.md).

- 2026-11-15 M3's selected commit `1c0fdbd5` repeats all three pinned commands successfully, but its canonical gate stops at two rustfmt line wraps. `2673d040` repairs only test-file whitespace; direct token comparison confirms unchanged runtime source and commands, so selected target proof remains applicable. Formatting passes; exact-key refresh and the full gate are queued under GPU. [Failure and repair record](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/integration/format-repair.json).

- 2026-11-15 M3's final GPU-reserved refresh passes all 1,966 exact comparisons at `58cd6cc1` in 943.61 seconds, with references unchanged. On the completed evidence tree `3a074d19`, D3.1, D3.2 and D3.4 pass again; the canonical M3 checker reports four passes, zero failures, including the full `./dev check`. All 265 selected artifacts match their retained hashes and sizes. Since the selected target capture, source changes remain limited to test whitespace and the native-only host-test attribute; motion and timing retain their documented coverage and limits. No incoming main files require renewed target proof. [Refreshed exact record](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/integration/format-exact.json.gz) · [Selected target proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/selected/README.md).

- 2026-11-18 Planner: [the module list's rulings on work left until after V1](../ARCHITECTURE.md#held-open-for-after-v1) touch M4 to M12 in one way. The forward pass keeps its three colour targets: guide buffers, if a later module builds them, are a prepass output at render extent under this module's extent and generation rules, never a fourth forward target, and MetalFX in M8 asks for none. No design line, item or plan note changes here.

- 2026-11-18 Planner, on the owner's answer. M3 was Blocked asking for an observer to watch the five TAAU on/off pairs at 1x on the Mac and in Chrome. The lead asked the owner whether agents can judge motion from captured frame sequences or whether they want to watch the pairs themselves. The owner: "agents can judge". D3.3 and D3.3c's inspections now say to judge from runs of consecutive captured frames in their captured order and timing, with no playback and no human observer, and to name the frames read; "Watch each at 1x" had been read by the builder and by [the review](../../../.orchestrate/evidence/APPLE-PRESENTATION/M3-review.md) as playback no agent can perceive. The design's motion-review convention says the same for every later item that asks to watch at 1x (D4.7, D5.6, D7.8, D8.8, D11.6), whose wording is unchanged, and names what counts as a defect in the frames. Statements, the quality bar and the four passed items are unchanged.

- 2026-11-18 M3. The builder passes D3.3 on Mac Metal and D3.3c on hardware Chrome WebGPU after inspecting five fresh TAAU on/off pairs per target. The Chrome exporter hit its eight-slot limit on the first take. The bounded raw spool and upload worker completed the replacement campaign with zero omissions. The three target checks, workspace gate and upload tests passed. The visual record names inspected frames, capture clocks, target differences and submission gaps. [Motion inspection and target records](../../../.orchestrate/evidence/APPLE-PRESENTATION/m3/motion-20261118/README.md).

- 2026-11-19 Planner, on M4's block. The builder ended `BLOCKED needs_dependency`: "The planner needs to select a Metal resize operation that preserves drawable ownership while GPU submissions remain in flight." Its audit is right: released wgpu-core 30.0.1 waits for the whole device queue inside `Surface::configure`, from any thread and for any surface, so the design's reconfigure "at most once a display tick" while frames are in flight cannot be built on the released crates, and a raw layer resize behind wgpu makes two extent authorities. The design now leaves the surface as configured while the size changes, lets Core Animation scale the drawable to the window, and reconfigures once in the step that takes the new generation, where the wait is paid once and recorded. A patched wgpu-core and an engine-owned drawable path were weighed and not taken. D4.4, D4.6 and D4.7 are rewritten to that rule; D4.7 now judges the window itself by OS-window capture and so needs Screen Recording access. D4.1 to D4.3 and D4.5 are unchanged. [The surface across a transition](DESIGN.md#the-surface-across-a-transition-wp4).

- 2026-11-19 Planner, on [the design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) of the morning's revision (changes requested, two Must Fix). **Must Fix 1:** the pinned wgpu 30.0.1 caches a requested configuration even when the configure fails, so a refused switch left two answers for the drawable's size, and `configure_surface` retried before the host could decide. The switch is now entered only with no surface texture held and no preparation job running, else deferred with no call; a configure error is followed by one restoring call before anything is acquired, and a failed restore leaves the surface refused until a later configure succeeds. Deferrals, calls, generations taken and restores are counted apart (D4.8 on the host, D4.9 on a real wgpu surface; D4.4 and D4.6 reworded). The module list's seam 5 ruling now says off-thread GPU work runs as a preparation-thread job. **Must Fix 2:** the iOS lease drops the old-extent frames the design told the owner to present. iOS now keeps `PLATFORM-RUNTIME-IOS` M7's transition between two frames and presents nothing in between; the compositor holds the last image under the shell's `resizeAspect`, and D4.10 observes the interval within the route's six-reply bound. Only the Mac builds off the render thread. **Suggestions taken:** D4.7 reads the window frames across each switch, and D10.6 asks that the WGSL path translate and run as in the release. [The surface across a transition](DESIGN.md#the-surface-across-a-transition-wp4).

- 2026-11-19 M4. The Mac keeps its active surface through the debounce and detached preparation, then configures once at the frame boundary. Both Metal recipes and the synchronous control exercised that rule; wgpu's own configure failure and its restore were verified against Metal's drawable. The iPhone simulator kept one window and device through both rotations, and Chrome WebGPU passed the DPR and history checks. The realtime Mac submissions were reviewed through both switches. Screen Recording access is still required to judge the OS window. [Target receipts](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4/README.md).

- 2026-11-19 M4. ScreenCaptureKit recorded the Portable game window through the sweep and both switches with the display awake. The valley filled the moving window, with one clean change at the extent switch and a steady picture at the forced scale observation. Realtime T2 repeated both switches on both Metal recipes with the valley and coordinate grid. Chrome WebGPU passed the extent and history check after the rebase. [D4.7 inspection](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4/D4.7.md) · [Chrome receipt](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4/chrome.md).

- 2026-11-20 M4 review fixes. The Metal tests verify replacement secondary views and contribution bindings, capture admission through a resize, per-thread creation calls, and first-frame history reset against a control that retains reconstruction. Both recipes present through the AppKit sweep, with two actual configure calls and a reason for each missed display interval. The synchronous control is rejected on both. [Target results and controls](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4/review-fixes.md).

- 2026-11-20 M4 missed display intervals. The receipt joins each callback to its own frame work and assigns actual presents by timestamp. Both review cases retain their late frames, and the fresh Mac sweep records two successful configuration calls and no restores on each recipe. Each synchronous control is rejected with 23 calls. [Mac records and regression replay](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4/missed-display-intervals/README.md).

- 2026-11-23 M5. Portable TAAU draws the five prepared scales inside one output grid, retaining colour and prior-grid depth. Metal proved history continuity and the independent reset, held-reference, attachment-marker and worker-allocation controls. Hardware Chrome checked the same scale trace and creation invariance. I watched all eight steps in the realtime Metal capture. The two 1080p cost runs describe shared load, with TAAU above the estimate. [Builder evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M5/README.md).

- 2026-11-23 M5 review fixes. The cloud trace stops rays using the rasterized depth rectangle. The native spatial and TAAU view records both publish −1 mip bias at native width. The Metal attachment test passes at all five widths. I watched the high-wind cloud route across all eight steps, and a matched Metal control confirms the previously clipped cloud bank persists through the return to 85%. [Attachment and layout tests](../../../.orchestrate/evidence/APPLE-PRESENTATION/M5/review-fixes/attachments.md) · [Cloud-sky inspection](../../../.orchestrate/evidence/APPLE-PRESENTATION/M5/review-fixes/README.md).

- 2026-11-23 Planning archive. The [M4–M11 drafts](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md) precede the done-when criteria.
  The M4 extent work and D4.1–D4.3 proof arms were checkpointed at `71577cd6`.
  [M4 handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/M4-handoff.md).

## M13 workspace timing baseline

- 2026-11-23 These are the cases from the passing `747909d1c` workspace check, with seconds from that run.

| Seconds | Test |
|---:|---|
| 6.131 | `thrum-render synthetic::exposure::tests::gpu::attachment::falling_twilight_radiance_does_not_brighten_the_metered_sky` |

- 2026-11-23 M13. The twilight case stays in the fast tier with both original samples and thresholds.
  It draws a single sky pixel and checks the submitted SDR luminance as well as the physical and exposed
  luminance. On M3 Max Metal the original case took 2.765 s alone. The compact case took 0.570 s alone
  and 0.142 s in the eight-case exposure batch, which passed on its first run under shared load.
  [Coverage and timings](../../../.orchestrate/evidence/APPLE-PRESENTATION/M13/README.md).

- 2026-11-23 APPLE-PRESENTATION M14 refreshed 30 native references after `32dfd3c14` restored the −1 mip bias.
  Its parent `f582a2af1` matched all five portable SDR checkpoints. The new bias changed 99,274 pixels at
  `approach`. With the cloud depth correction in place, zero bias recovered all five checkpoints and the old
  pixels for all 30 files on both Metal recipes.
  [Native mip-bias ruling](../ARCHITECTURE.md#ruling-on-native-mip-bias) ·
  [Attribution](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14-attribution.json) ·
  [Qualification](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/README.md) ·
  [Reference hashes](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/reference-hashes.md#native-references)

- 2026-11-23 The lead extended M14's refresh to all four Chrome WebGPU baselines. The restored −1 bias changed
  every decoded frame. The zero-bias control recovered all 1,924 frames, and both arms matched every identity.
  [Mip-bias ruling](../ARCHITECTURE.md#ruling-on-native-mip-bias) ·
  [Chrome control](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/chrome-bias-diagnostics.json) ·
  [Manifest hashes](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/reference-hashes.md#chrome-manifests)

- 2026-11-23 M6. The Mac configures a float drawable with a retained AppKit layer and applies current headroom
  in the prepared present pass. All seven target commands passed, including H=1 identity on both Metal recipes,
  drawable bounds through the headroom step and SDR lease continuity on the iPhone simulator. Builder and
  independent visual inspection pass the Sun, sky and four night lamp cores at H=2 and H=8.
  [Target receipts and crops](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/README.md).

- 2026-11-23 M6 review fixes. The ordinary Mac window retains its display mode under control observation. Visual
  publication seeds the new present pass from current headroom. An open default capture tap follows surface format
  and drains older images in order. The night-light fixture sits outside the public route, and the standard Mac
  command presented two frames. Metal checks reproduced the forced headroom step, the first drawable from a
  replacement visual and one capture session across both formats. The iOS preflight separately reproduces the HOST
  M4 dead-code diagnostic from `626798f4f`.
  [Target receipts](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/review-fix/README.md).

- 2026-11-23 M6 second review fixes. Drawable capture reserves against the active device generation. A stale
  tap reports omissions after device replacement and remains invalid through an extent publication until the
  caller opens a fresh session. The Mac backing-grid diagnostic selects the 8-bit path used by its exact pixel
  pattern. The Metal recovery test passed on both SDR and EDR, and the default diagnostic launch presented two
  frames at 1920×1080. [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/review-fix-2/README.md).

- 2026-11-23 Planner. `exact-images` passed on `f7abcc2a3` and first failed on its child `fcc62608e` (M6).
  Both attempts at the Chrome day SDR comparison failed at ordinal 61 with `8d7dab93…` against the retained
  `a7cde573…`. The comparison failed the same way on `fdbe70e0a` and `c8c9100d8` after the native stages passed.
  Replacing only the SDR shoulder expression on the same Wasm build recovered all 481 records after the control
  changed 147 digests with all identities matching. M15 restores the SDR arithmetic ahead of M6 using the
  existing APPLE-TILE references.
  [Attribution record](../../../.orchestrate/evidence/APPLE-PRESENTATION/M15-attribution/record.json)

- 2026-11-23 Planning archive. M1–M3 used seams 1–4 as written, with G4 making the atmosphere motion edit and
  the third forward target. Chrome covered the Mac witness rows, including volume and phase edits, and reported
  per-frame history identity through the game page. The reconstruction bound was CIEDE2000 of 3 on submitted
  8-bit SDR. M4 used a 100 ms resize debounce and waited for the GPU at the Mac surface switch. The iPhone
  held the fitted last picture with background at its sides until the first frame at the new size.

- 2026-11-23 M15 restored the pre-M6 SDR shoulder arithmetic at `ba44aac63`. The hardware Chrome fixed day
  captures shared the route, cooked root and all 481 retained record identities. The control differed in 147
  decoded RGBA8 digests, and the corrected build matched all 481, including ordinal 61. Metal D6.2 passed
  with the GPU required. D6.3 stayed within one 8-bit code at all six checkpoints on both native recipes.
  The full matrix runs through the shared `exact-images` check on main.
  [Target receipts](../../../.orchestrate/evidence/APPLE-PRESENTATION/M15/README.md).

- 2026-11-23 M6 third review fixes. SDR and EDR share drawable extent validation and the letterboxed viewport.
  Capture charges pending staging and writer-held images to the session across extent and format changes. Metal
  tests prove stale drawable recovery and capture omissions under the slot, lease and byte limits, with presentation
  continuing and capture resuming in the same session. All seven named target checks passed. The fresh Metal route's
  raw pixels and previews match the previous route byte for byte and pass the visual inspection.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/review-fix-3/README.md).

- 2026-11-24 M6. Integrated the extent and capture-budget fixes after M15. The renderer rejects stale drawable
  extents on both display paths and bounds staging and writer leases across promotion. All seven target checks
  and the Mac visual inspection passed. Chrome hardware WebGPU matched all 481 retained fixed-day SDR frames.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/after-m15/README.md).

- 2026-11-24 M6 capture review fixes. Float capture follows the configured drawable and preserves the full
  letterboxed image before and after resize. The tap now reuses the session's copy-timing objects during
  promotion and resize. Metal verified the shared byte and slot limits, capture recovery and zero timed-copy
  allocations of pipelines, bind groups or query sets on the Mac render owner.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/drawable-capture/README.md).

- 2026-11-24 M7 builds the frame host's clock-free quality governor, held-frame admission, registered aerial cadence and separate GPU and animation-frame timing paths. The scale recovery estimate projects the recorded forward and effect work onto the next prepared render area. The rest of the measured GPU time stays fixed in that estimate.
- Metal verified policy, extent failure and cancellation, external knobs, thermal recovery and identical route authority. Hardware Chrome verified both timing paths. Ordered real-time Metal crops showed continuous silhouettes and lighting across all eight scale steps. [M7 evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M7/README.md)
- M7 stays in progress. Current live Mac drawable acquisitions return `Occluded`, which leaves D7.2 and D7.3 open. The T2 time records show long intervals near load onset, which leaves D7.8 pacing open. [M7 handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/M7-handoff.md)

- 2026-11-24 M7 resumed on current main. Delayed samples feed the governor through the ordered frame timing
  collector, including Metal timestamp validation. The repository check passed 1,141 tests after the prescribed
  serial rerun of six concurrent timeouts. All three fixed Metal arms again ended at tick 540 with authority hash
  `edfe096ac1c674bf`. The unlocked 60 Hz window presented on all three arms of the shared-GPU run. Its loaded
  governor returned to the unloaded miss rate in the comparison window but did not recover its starting scale
  during the run. [Resumed target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/M7/resume/README.md)

- 2026-11-24 M7 now budgets overlapping GPU work from the timed output interval and the longest measured
  pass. The native path adds the render owner's pre-submission CPU work. At `3e7324156` Mac D7.2 met its
  settled 60 Hz comparison with 0 governed misses, 1 unloaded and 16 with the governor off under the same load.
  Scale fell at tick 236 and recovered to 100% at tick 336. D7.3 changed scale zero times. Chrome passed both
  timestamp paths. T2 proved both scale transitions visually and recorded three isolated pacing gaps away from
  the transitions. [Target records](../../../.orchestrate/evidence/APPLE-PRESENTATION/M7/resume/README.md)

- 2026-11-24 M7 qualified D7.8 in a Metal take with all 600 scheduled images verified and no missed
  opportunities. The largest submit interval from load onset through recovery was 19.17 ms. The same
  4,600-round pass as D7.2 lowered scale at tick 233 and the governor restored it at tick 332.
  The independent viewer could not locate either scale step. The latest workspace check passed 1,142 tests.
  [Motion, timing, and separate capture cost](../../../.orchestrate/evidence/APPLE-PRESENTATION/M7/resume/qualified-motion/README.md)

- 2026-11-25 WP7 gives the Chrome proof a rendered warm-up before admitting governor samples and streams
  the trace through the recovery window. On M3 Max WebGPU both timing arms started and finished at 100%
  scale. The GPU arm first dropped at tick 138 and the animation-frame arm at tick 135. The Mac verifier
  got zero presents while the console was locked. D7.2 and D7.3 still require an unlocked window for the
  merged replay. [Chrome and Metal records](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/replay/README.md)

- 2026-11-25 WP7 replayed on the unlocked Mac. The live check selects load strength using the governor-off
  control alone and compares the full settled window. At 60 Hz the governor dropped to 85% at tick 237
  and returned to 100% at tick 333. The unloaded and governed arms each missed 0 of 59 opportunities,
  against 4 of 59 with the governor off. The separate steady run changed scale zero times. Hardware Chrome
  passed both timing arms with 12,288 rounds, each returning to 100%.
  [Target records](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/unlocked/README.md)

- 2026-11-25 WP7's Chrome proof now waits for the pre-load scale to hold for one second after recovery,
  with a bounded deadline. The GPU timestamp arm uses 8,192 rounds and the animation-frame arm uses
  12,288 rounds to create sustained callback pressure. Two exact item runs passed on M3 Max WebGPU.
  [Chrome recovery traces](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/chrome-recovery/README.md)

- 2026-11-25 WP7 preserves coalesced display callbacks in the missed-present proxy and interrupts the policy
  streak when an opportunity has no render outcome. The native comparison verifies that delivery rates
  match before applying forced load. At 60 Hz the exact checks recorded zero steady scale changes and
  2/60 governed misses, 0/59 unloaded and 11/59 with the governor off under the same load.
  [Native delivery traces](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/native-delivery/README.md)

- 2026-11-25 WP7's Mac window proof holds the display awake across the live measurement, including retries.
  An unloaded baseline with more than two missed opportunities invalidates the attempt before forced load.
  The earlier sleep trace was rejected at 36/59 misses. The exact 60 Hz replay measured 0/60 unloaded,
  1/59 governed and 15/59 governor-off misses under the same load. D7.3 recorded no scale changes.
  [Display environment and target traces](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/display-environment/README.md)

- 2026-11-25 WP7 repaired the review findings. The native proof uses the ordinary 1920×1080 window and full
  display-link selection. Its 120 Hz replay recorded 0/119 settled misses with the governor and 54/120 with
  it disabled at the same load. The steady route held 100% scale. Native pressure uses the slower of the
  overlapping CPU and GPU stages. The render owner can retire an occupied slot within the same display
  opportunity. A submitted quality record takes the values committed at its frame boundary, the aerial
  lookup retains pending sky changes, and Chrome updates its budget when callback cadence changes.
  [Target records](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/review-repair/README.md)

- 2026-11-25 WP7's 1920×1080 T2 capture retained all 600 opportunities at its 60 Hz elapsed-time cadence.
  The 1× movie, unscaled crops, tap-off control, GPU copy cost and blind opinion cover the downstep and
  recovery. [Motion inspection](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/review-repair/motion-verified/README.md)

- 2026-11-25 WP7 repaired two checks that failed on main. The fixed offscreen Metal thermal proof
  gives render work headroom with an explicit 100 ms diagnostic display interval. The exact arm
  held 100% scale, stepped aerial cadence through 1, 2, 4, 2, 1, and created zero pipelines.
  The controlled Chrome refresh replay retires WebGPU work before advancing synthetic time. It
  held quality at 120→60→120 Hz, and both real callback load arms dropped and recovered.
  [Target records](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/clean-checkout/README.md)

- 2026-11-25 D7.3's failed main run counted six scale changes at 120 Hz and 61 missed render-side
  opportunities over the authored route. Three other Metal jobs overlapped that run. After an
  over-limit run, the verifier now checks delivery with the governor off on the same unloaded route.
  An invalid control permits a bounded retry. A delivering control leaves the over-limit run failed.
  The shared-lock Metal replay delivered zero scale changes at 1920×1080 and 120 Hz.
  [Steady route evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/steady-control/README.md)

- 2026-11-25 The Mac window proofs passed with the repository GPU lock held exclusively at 1920×1080
  and 120 Hz. D7.2 recorded 0/119 settled misses unloaded, 0/119 governed and 54/119 governor-off under
  the same load. D7.3 recorded zero scale changes. The lead can adopt the lock-only plan change after
  the active builder releases the module. [Target records and lock change](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/exclusive-gpu/README.md)

- 2026-11-25 The merged CLOUDS window change made GPU timing opt-in. The governor-off control still
  presented under load but carried zero GPU timestamps. The live proof now explicitly requests GPU
  timing in every arm so it can verify the same debug-load pass across the governed and disabled runs.
  [Merged replay evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/exclusive-gpu/README.md)

- 2026-11-25 A governed native frame can now release its render slot while its prepared timestamp
  batch maps. Timestamp resolution still follows render completion, and retirement validates counters
  in submission order. The full-size 60 Hz T2 take submitted all 600 opportunities with the image
  tap on and off. Both arms stepped to 85% at tick 303 and returned to 100% at 363. I inspected the
  native-pixel sequences across both changes. The blind viewer found the authored cut and could not
  identify either scale-step frame. [D7.8 evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP7/motion-repair/README.md)

  The deferred readback path applies to governed offscreen rendering. The numbered display-link
  path retains its previous admission behavior. The scoped live check passed at 120 Hz with 0/120
  settled misses unloaded, 0/119 governed and 56/119 governor-off. Its unloaded route held 100%.

- 2026-11-25 Planner: the main-branch check `long-tests` failed at a25e1d13d (last pass 5b5a0aa3e), and the
  cause is WP7's fe6716e44, which gave each native 3D frame slot a second timestamp query set. Metal allows a
  process 32, measured here: the 33rd fails, at any sample count, whatever other processes hold. wgpu then loses
  the device, and the next buffer creation panics. The binaries built at a25e1d13d fail from four test threads
  up and pass with three or fewer, run alone, so it is not other GPU work. WP16 is planned before WP7: one
  query set a slot, and a count that refuses the 33rd with a typed error.
  [Design](DESIGN.md#timestamp-query-sets-and-metals-limit-wp16)

- 2026-11-25 Planner: the WP16 design review asked for two changes. A refused timestamp request in an installed
  ray build probe now still builds the mesh, with no sample (design, ruling, WP16.2). The engine's own count is
  replaced by asking Metal first, because wgpu holds dropped query sets the engine cannot see; WP16.2 holds the 32
  on two devices and new WP16.5 proves when a place returns. The witness rows are W1 to W7 again in the design.
  [What changed and the measurement](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16-plan/README.md)

- 2026-11-25 Planner: the second WP16 design review had one Must Fix, the interval between the Metal trial and
  the wgpu creation. The guarantee is now bounded to creations that come through the one call site, which
  answers requests one at a time (design, ruling). WP16.2 also holds that nothing else calls `create_query_set`
  but `SEA`'s test harness, and new WP16.6 holds one request while a second competes for the last places.
  [What changed and the measurement](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16-plan/README.md#the-interval-between-the-trial-and-the-creation)
- 2026-11-25 WP16. One timestamp query set per slot survives readback rotation. Metal admits a whole request
  under the process lock. Exhaustion reaches a frame host as a typed timing error and a ray build probe as a
  build without a sample. All six items passed on M3 Max Metal, including five consecutive 16-thread runs of
  the four specified binaries. The governor policy, knobs and transition tests also passed.
  [WP16 evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16/README.md)
- The separate 16-thread `mesh_stage_gpu` run had five failures with `TimestampQuerySetsExhausted` during
  timing preparation. That binary's main-tier exclusion stays with MESH-SHADING.
  [Mesh-stage diagnostic](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP16/mesh-stage-gpu.log)
- 2026-11-25 Planner: WP8's block (`NativeProvider` absent) is answered by a new `APPLE-TILE` WP13, which WP8 now
  depends on. Seam 8's ruling says who builds what. WP8's items did not change, and its note no longer says WP10
  and WP11 left the interface. [Handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-handoff.md)
- 2026-11-25 Planner: WP8 stopped on D8.3 (`needs_context`), and D8.3 is rewritten; no other item and no design
  text changed. The old bound measured how MetalFX and TAAU differ on a still frame: a held-storage run reproduced
  all 17 misses. D8.3 now judges the forced step against a matched held run and the scrub against the
  un-reconstructed frame, each within 1% of mean luminance, and the builder judges the exposure texture alone first.
  [Reasons and numbers](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-plan/README.md)

- 2026-11-25 WP8 D8.7 passed on the iPhone 18 Pro simulator. Background revocation discarded one encoded
  TAAU frame with a valid predecessor. The first resumed submission reused its order with invalid history
  and completed on the same device generation.
  [Simulator receipt](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/simulator-resumed-checkpoint/reset.json)

  The standalone Mac MetalFX probe completed all five scales under Metal validation. The merged native
  lifecycle serves the copy/select fixture. The planner can assign the reusable `NativeProvider` interface
  to APPLE-TILE or place the extraction in WP8 under seam 8, preserving the frame coordinator's submission
  and completion authority.
  [Hardware probe](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/descriptor-probe.md) ·
  [WP8 handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8-handoff.md)

- 2026-11-25 WP8. MetalFX now records through APPLE-TILE WP13's native segment, with the existing TAAU pass
  prepared for rejection. D8.1, D8.2 and D8.4–D8.6 passed on Mac Metal. The earlier iPhone simulator pass
  remains recorded for D8.7. D8.3's normalized path passed the scrub and half-float boundary checks.
  The forced storage step still exceeded TAAU's mean-luminance error at 17 of 36 samples with a zero repeat
  noise floor. D8.8's realtime capture and inspection are in progress.
  [Metal evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/target-a30845335/README.md)

- 2026-11-25 WP8. The held-storage control reproduced all 17 D8.3 misses and the same worst mean-luminance
  excess. The worst sample came 15 submissions before the multiplier step. All twelve T2 realtime arms
  completed and every frame matched its writer hash. Consecutive native crops show clean trailing contours,
  reveal and spawn in the inspected windows. Continuous 1× playback review is still due.
  [Exposure control](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/fixed-storage/comparison.json) ·
  [Motion evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/motion-previews/README.md)

- 2026-11-25 WP8. The 1920×1080 exposure diagnostic still missed the exact D8.3 bound. The GPU witness
  held 12 colour channels within one half-float ULP at the matching tone input. For D8.8 the bright-star
  crop modulated 8.11% at the native output before tone, and the SDR paired crop 2.79% peak-to-peak with
  MetalFX against 0.16% with TAAU. Continuous playback review was outstanding. The rest is in the records:
  [Exposure at 1920×1080](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/exposure-1080p/comparison.json) ·
  [GPU tone-input witness](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/still-tone-input.log) ·
  [Motion record](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/motion-previews/README.md)

- 2026-11-25 WP8. The exposure texture passed the revised D8.3 on Mac Metal at every sample. The peak
  differences were 0.0707% for the matched storage-step arm and 0.4551% for the scrub. The unused
  normalising pre-pass and its counters were removed. The rerun also confirmed the five-scale native
  route and its motion controls. [Mac target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/target-exposure-texture/README.md)

- 2026-11-25 WP8. Mac Metal replays passed the still checkpoints and the prepared fallback and reset checks
  after the pre-pass removal. Both APPLE-TILE WP13 seam tests and the repository gate passed. The final
  texture-only build produced all six paired T2 sequences. The native-size bright-star crop varies by 2.979%
  with MetalFX and 0.159% with TAAU. I inspected sequential native crops and samples from 1× decoding.
  Continuous viewing remains open. [Retakes and measurements](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/texture-selected-previews/README.md)

- 2026-11-25 WP8. D8.8 passed the builder’s motion inspection on Mac Metal. I read captured consecutive
  native-size frames across all six pairs under the design’s motion-review convention. The bright-star
  luminance varied more with MetalFX, including a 2.26% single-frame step, without visible added shimmer in
  the inspected native crops. The evidence names the submissions and records the governor timing.
  [Motion inspection](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/texture-selected-previews/README.md)

- 2026-11-25 WP8. The merged D8.7 replay stalled during CoreSimulator's first-boot data migration.
  Rebooting the owned simulator with its OS data preserved reached application services. The reset
  route now starts with fresh game and XCTest runner installations by clearing both bundles after
  boot. The simulator SDK also lacks MetalFX, so simulator builds decline native scaler preparation
  and omit the framework link. [Simulator replay and diagnosis](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/D8.7-replay.md)

- 2026-11-25 WP8. The repaired build installed on the simulator. The launch control stayed on the
  Apple logo and also timed out opening Settings. Waking the display and restarting the guest's
  IconServices and SpringBoard did not bring up the Home Screen. [Launch diagnosis](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/D8.7-replay.md)

- 2026-11-25 WP8. D8.7 passed twice after the host simulator recovered, including on the rebased
  build. The second run counted one background discard and resumed submission 121 with invalid
  history and jitter phase 25. Both turns verified shutdown. [Rebased simulator receipt](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/simulator-repaired/reset.json)

- 2026-11-25 WP8. On Mac Metal, device replacement discarded one encoded frame on both Apple Tile and
  Portable TAAU. The first frame on the replacement MetalFX scaler matched an independent history reset
  byte for byte. The scaler received `setReset: true`. A temporary mutation suppressing the actual
  first-encode reset failed the test while the coordinator still counted one reset.
  [D8.6 target proof](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP8/D8.6-replacement.md)

- 2026-11-25 WP9. Mac Metal passed all five prepared paths and both recipes completed the moving route
  without shader-module or pipeline creation after readiness. The late pipeline control registered.
  Hardware Chrome completed the route with zero after-ready device calls to create shaders or pipelines.
  Two cold and two warm offscreen launch baselines record the Naga, Metal and synchronous MetalFX stages
  under shared load. [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP9/README.md)

- 2026-11-25 WP9. The clean verifier failed D9.4 because the browser check built the game but had no
  `thrum-cook` to serve the route's cooked valley. D9.4 now builds the default cooker before starting
  Chrome. [Verifier replay](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP9/verifier-replay.md)

- 2026-11-26 WP9. Prepare the native game's required contribution before publishing generation readiness.
  The launch and route witnesses use that event with zero submissions and record the first cooked draw
  separately. The prepared-path fixture now advances the same surface lease recovery used by the Mac
  render actor and presents real Metal drawables on each side of the same-extent configure.
  [Review follow-up](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP9/review-fix/README.md)

- 2026-11-26 WP10. The pinned Metal backend exports translation records for the offline cook and loads
  matching native stages during pipeline creation. The creation ledger covers every V1 route pipeline
  in the parity witness. Mac Metal verified both recipes and all five submitted stills per recipe, the
  visibility and diffuse dispatches, and rejection of stale keys. Launch records include serial WGSL,
  serial cooked libraries and parallel cooked libraries under shared load. Offline libraries are opt-in
  while WP12 measures the archive. wgpu-core still parses and validates WGSL interfaces, and its two
  device-internal validation pipelines still translate at device setup.
  [WP10 evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP10/README.md)

- 2026-11-26 WP10 review follow-up. Thrum now submits keyed Metal libraries through registry-admitted
  wgpu passthrough modules. The pinned backend observes the release's WGSL translations and restores
  stage records for passthrough. On Mac Metal, both native recipes loaded all 44 route stages, matched
  all five submitted checkpoints byte for byte, and performed zero pipeline work after readiness.
  Stale keys, reflection, launch records, the offline pin, and the Chrome shipping boundary passed.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP10/review-fix/README.md)

- 2026-11-26 WP10 lifetime follow-up. Shader and layout inputs now own their Metal stage-key
  registrations. The preparation index releases each registration with the last input owner.
  Three Mac Metal disposal rounds returned live resource counts to baseline, and reused inputs
  drew with zero WGSL translations. Both native recipes matched all five submitted checkpoints
  byte for byte, and all twelve launch runs and the Chrome shipping check passed.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP10/lifetime-fix/README.md)

- 2026-11-26 Planner: WP12 stopped on a dependency conflict. wgpu-core 30.0.1 writes and checks a pipeline
  cache's header for Vulkan alone, so on Metal the public `get_data` panics and import is refused, and no
  wgpu-hal patch reaches either. The lead decided to pin wgpu-core 30.0.1 as well, for the Metal cache header
  and nothing else, shaped so the owner could offer it for gfx-rs/wgpu#8526. The design says so, D12.1 is
  rewritten around the public `PipelineCache` round trip, and a new D12.5 holds both pins to their patches. No
  other work package changed. [Handoff](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-handoff.md)

- 2026-11-26 Planner, design review Must Fix 1 (check the saved cache bytes before import). The game's file
  header now carries a digest of the whole `get_data` byte sequence, taken at harvest and checked by every
  process before wgpu is given the data. D12.3's changed-byte case must be counted as corrupt with nothing
  imported, and it gains a reviewer question. No other item changed.
  [Design](DESIGN.md#pipeline-preparation-wp10-and-wp12) ·
  [review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md)

### CLOUDS WP9 fast-pan observation

An 85° elapsed pan under the broken preset at a 10° Sun shows short horizontal
streaks at the newly revealed right edge with portable TAAU on Mac and Chrome.
Stationary controls at the same yaw show much of the same detail above the
ridge. The moving Chrome frame still shows extra smearing on cloud edges near
the right of the crop. A nearby moving capture with reconstruction off shows
sharper, stair-stepped detail. On Metal, the lower cloud bands persist with
negligible TAAU history and in a separate take that preserves the sky's motion
under clouds. Both cloud histories reset for one frame at a cut and at the
layer profile change. [Metal controls](../../../.orchestrate/evidence/CLOUDS/WP9/metal/pan-isolation/README.md) and [Chrome controls](../../../.orchestrate/evidence/CLOUDS/WP9/chrome/pan-isolation/README.md).
- 2026-11-26 WP12. A fresh Mac Metal process hit a persisted HAL compute archive and rejected the omitted
  function and empty archive in strict mode. Public `PipelineCache::get_data` panicked in wgpu-core 30.0.1
  while writing the Metal cache header. The import validator also rejects Metal adapters. Proposed a narrow
  wgpu-core pin for Metal cache headers so WP12 can use the public cache API. The game route and launch
  comparison remain open. [Handoff and reproduction](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12-handoff.md)

- 2026-11-26 WP12. The authorized wgpu-core pin enabled public render and compute cache round trips on Mac Metal.
  The strict route hit all 32 pipelines. The empty and one-pipeline omission controls failed before readiness.
  All three fault routes completed without late creations. The combined cold and warm launch comparison
  selected the archive for the registry default when local artifacts are supplied.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12/qualification/README.md)

- 2026-11-26 WP12 review fixes. A libraries-only launch now requests Metal shader passthrough through the
  registry and prepares all 44 stages in parallel. With all products supplied, archive preparation ranked first
  in the paired launch record and remained the default. On Mac Metal, the stale, corrupt and partial archive
  routes each repaired the configured file before readiness and completed 361 ticks without late creations.
  The next fresh process strictly hit all 32 manifest pipelines in each repaired file.
  [Target evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP12/review-repair/README.md)

- 2026-11-26 Plan: U1 becomes three work packages after WP11, under the owner's rules of 7 November (revision 6
  above). U1 writes the upstream notes for the owner and a reviewer checks them. In U2 the owner posts their own
  comment on gfx-rs/wgpu#8526 and answers go ahead or stop. U3, the change on wgpu's trunk, is planned from that
  answer. The plan listed U1 before WP11; it moves behind it because revision 6 says the offer "does not gate V1
  or M11". Read that day: trunk `60cef2be` still has no Metal adapter key, the issue's last comment is from
  20 October, and gfx-rs/wgpu#9623, an earlier pull request for the feature, was closed in June with no review.

- 2026-11-26 Planner, design review Must Fix 1 (U2.2 must accept a stop before posting). U2.2 now takes the
  owner's words as the whole record of a stop, posted or not. Only a go ahead needs the link to the owner's
  comment, the replies it rests on, if any, and the three decisions. U2's name, its question to the owner and
  the design's U2 paragraph say the same.
  [Design](DESIGN.md#offering-the-cache-upstream-u1-to-u3) ·
  [review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md)

- 2026-11-26 Planner, design review Must Fix 2 (U1.4's account check cannot establish who acted). U1.4 no
  longer looks at what an account shows upstream. The notes and the landing are judged from the delivered
  files, and what agents did from the host's record of each session that built U1, which the script's delivery
  history lists. The design says why an account's activity is not evidence either way. The owner's publication
  rule is unchanged.

- 2026-11-26 Planner, design review Must Fix 3 (U2.1 can miss a new Metal cache pull request). U2.1's reading
  now covers each pull request proposing a Metal pipeline cache opened since part 1 was read, given in part 1's
  dated line as part 1 gives the others, and its reviewer searches for one the line leaves out.

- 2026-11-26 Planner, WP11 unblocked: the full-screen route misses 120 Hz at the governor's floor, and the lead
  revised the target to a steady 60 Hz there. The design gains the requested frame rate, D11.5 and D11.6 are
  written again, and reaching 120 Hz at full screen is WP17, after G4.
  [Design](DESIGN.md#the-requested-frame-rate-at-full-screen-wp11) ·
  [reasons and numbers](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11-plan/README.md)

- 2026-11-26 Planner, design review Must Fix 1 (60 Hz at full screen was treated as the bar met). 60 Hz there is
  now an interim rate. The vision's full-refresh bar stands, D11.5 has the ledger record it as owed by WP17, and
  the milestone is reached at WP17, not WP11. WP17 ends only when 120 Hz is held, and the plan no longer says
  `SHOWCASE` does not wait. The lead's ruling stands for no more frame-graph work in WP11.
  [Design](DESIGN.md#the-requested-frame-rate-at-full-screen-wp11) ·
  [detail](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11-plan/README.md)

- 2026-11-26 Planner, design review Must Fix 2 (D11.6 could pass a visible hitch). Pacing is now judged by the
  Mac host's refresh account. Every refresh interval of the route is on the record, including a target the
  link sent no callback for, and an interval is served only by exactly one present call made inside it. D11.6
  needs every interval served in each of two takes, and a missed one counts whatever its cause. The 95th
  percentile bound is gone, because with every interval served it adds nothing. D11.5 records this as present
  calls the engine made, and `SHOWCASE` WP8.6 still owns what the panel showed.

- 2026-11-26 Planner, design review Must Fix 3 (D11.5 asked for a forced-rate label the retake never had). The
  retake at `1eacafa62` is described as its build ran it: a 60 to 120 Hz request with 120 preferred, an 8.33 ms
  interval, EDR headroom forced to 2. Only a take made with WP11's new launch setting is called forced-rate,
  and D11.5's reviewer checks both.

- 2026-11-26 Proposal to `SHOWCASE`: add `APPLE-PRESENTATION:WP17` to the work packages the V1 exit rests on,
  which are WP8's `depends_on` and WP8.12. WP7 still needs only WP11. Until WP17, a full-screen run presents at
  60 Hz, which is not the display's full refresh that `SHOWCASE`'s cue S5 judges. The module list's exit table
  and its line "WP11 (the paired proof, which closes G4)" would follow. Taking WP17 out of the exit is the
  owner's decision.

- 2026-11-26 Planner, second design review, Must Fix 1 (the V1 exit could still accept WP11 as the last Apple
  presentation prerequisite). The proposal above is taken, in `SHOWCASE`'s files and the module list. `SHOWCASE`
  WP8 depends on `APPLE-PRESENTATION:WP17`. WP8.12 names WP17 in its statement and its check, and the check's
  stale `M` ids are `WP` ids: with every other prerequisite Done and WP17 open it prints
  `not Done: ['APPLE-PRESENTATION:WP17']` and fails, and with WP17 Done it passes. WP7 and its WP7.1 still need
  only WP11. The module list's row for `SHOWCASE`, its exit coverage table, its build order and its planning
  note for this module say the same. No item of this module changed.
  [Review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md)
- 2026-11-26 WP11. Alternating Metal replay, governed Chrome function and motion, and iPhone simulator
  SDR reconstruction through rotate and resume passed on their targets. MetalFX lowered the measured
  reconstruction cost and held-reference error. The paired Apple/EDR path did not show a consistent
  whole-path timing win, so the registry retains TAAU. [Promotion and follow-ups](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/promotion.md)
- 2026-11-26 WP11. D11.5 and D11.6 remain open. At 3456×2234, the governor and MetalFX with an EDR layer missed
  the 120 Hz cadence with capture off. The raw tap retained 307 of 800 presented frames. The optional panel
  observation is outstanding. [Mac pacing and capture](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-fullscreen-review/README.md)
- 2026-11-26 WP11. The ordinary float-surface graph skips the unused SDR tone pass and validates timing
  against the submitted variant. The Mac window requests drawable copy access for the explicit EDR tap.
  In the fastest capture-off trial at 3456×2234 and 50% render scale, the GPU span was 10.85 ms p50
  and 14.67 ms p95 against an 8.33 ms display interval. It made 456 present calls across 704 opportunities.
  D11.5 and D11.6 remain open. [Capture-off measurements and pass breakdown](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-budget-retake/README.md)
- 2026-11-26 WP11. The Mac now requests fixed 60 Hz for fullscreen on the built-in 120 Hz display. The
  paired replay supports keeping TAAU as the Apple Tile default. At native size, T2 retained all 380
  presented submissions across the four cue windows, with separate capture-off pacing records.
  The new forced 120 Hz control measured 11.54/15.60 ms GPU span p50/p95 at the 50% governor floor.
  [Fullscreen captures, pass timings and present intervals](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-60hz-closing/README.md)
- 2026-11-26 WP11. Capture-off takes 37 and 38 served all 361 refresh intervals of the fullscreen
  route at the unforced 60 Hz request. Their 50% governor-floor GPU spans were 11.06/12.66 ms and
  11.15/12.55 ms p50/p95. WP17 owns full-refresh fullscreen cadence and the milestone.
  [Pacing takes and host load](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/mac-60hz-closing/pacing-index.md)
- 2026-11-26 WP11. The merged D11.1 run read the idle screen flag before its asynchronous wake took effect.
  The proof now checks the wake before the timed hold, with a single display cycle for a persistent idle flag.
  The paired replay still measures the same window route.
- 2026-11-26 WP11 review repair. Replay scale now follows the exact extracted publication, and all
  2,475 routed comparison samples passed the per-submission extent check. The new proof checks repeated
  ticks and the first rendered samples after skipped ticks. EDR prepares compatible backing for surface
  variants and includes the SDR encode only when the graph reads it. Six Mac Metal renderer tests passed.
  [Paired evidence](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/review-repair-paired/README.md) ·
  [EDR graph tests](../../../.orchestrate/evidence/APPLE-PRESENTATION/WP11/review-repair-render-final.log)
- 2026-11-26 U1 records the upstream issue, contributor rules, patch comparison and WP12 launch figures in
  `.orchestrate/evidence/APPLE-PRESENTATION/U1/upstream-notes.md`. On wgpu trunk `60cef2be`, the core header
  patch applies with the crate path mapped. The archive patch rejects four hunks across dependency features,
  render pipeline creation and adapter feature reporting. The notes recommend offering the header and base
  archive first, with the division of work and the PR structure to be settled upstream by the owner.
- 2026-11-26 U1 review repair. The briefing now gives the libraries-only default as 44 stage loads across five
  threads. The D12.4 process log confirms 31 events on the main thread and 13 across the four workers.
  The publication audit also includes the Guardian auto-review child.
