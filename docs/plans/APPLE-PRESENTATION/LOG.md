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

- 2026-11-19 Planner, on M4's block. The builder ended `BLOCKED needs_dependency`: "The planner needs to select a Metal resize operation that preserves drawable ownership while GPU submissions remain in flight." Its audit is right: released wgpu-core 30.0.1 waits for the whole device queue inside `Surface::configure`, from any thread and for any surface, so the design's reconfigure "at most once a display tick" while frames are in flight cannot be built on the released crates, and a raw layer resize behind wgpu makes two extent authorities. The design now leaves the surface as configured while the size changes, lets Core Animation scale the drawable to the window, and reconfigures once in the step that takes the new generation, where the wait is paid once and recorded. A patched wgpu-core and an engine-owned drawable path were weighed and not taken. D4.4, D4.6 and D4.7 are rewritten to that rule; D4.7 now judges the window itself by OS-window capture and so needs Screen Recording access. D4.1 to D4.3 and D4.5 are unchanged. [The surface across a transition](DESIGN.md#the-surface-across-a-transition-m4).

- 2026-11-19 Planner, on [the design review](../../../.orchestrate/evidence/APPLE-PRESENTATION/design-review.md) of the morning's revision (changes requested, two Must Fix). **Must Fix 1:** the pinned wgpu 30.0.1 caches a requested configuration even when the configure fails, so a refused switch left two answers for the drawable's size, and `configure_surface` retried before the host could decide. The switch is now entered only with no surface texture held and no preparation job running, else deferred with no call; a configure error is followed by one restoring call before anything is acquired, and a failed restore leaves the surface refused until a later configure succeeds. Deferrals, calls, generations taken and restores are counted apart (D4.8 on the host, D4.9 on a real wgpu surface; D4.4 and D4.6 reworded). The module list's seam 5 ruling now says off-thread GPU work runs as a preparation-thread job. **Must Fix 2:** the iOS lease drops the old-extent frames the design told the owner to present. iOS now keeps `PLATFORM-RUNTIME-IOS` M7's transition between two frames and presents nothing in between; the compositor holds the last image under the shell's `resizeAspect`, and D4.10 observes the interval within the route's six-reply bound. Only the Mac builds off the render thread. **Suggestions taken:** D4.7 reads the window frames across each switch, and D10.6 asks that the WGSL path translate and run as in the release. [The surface across a transition](DESIGN.md#the-surface-across-a-transition-m4).

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

- 2026-11-23 APPLE-PRESENTATION M14 refreshed the affected `apple-tile-native` images under the
  [native mip-bias ruling](../ARCHITECTURE.md#ruling-on-native-mip-bias). Commit `32dfd3c14` restored
  a −1 material mip bias for native spatial rendering. On Mac Metal, zero bias recovered the old
  pixels and −1 reproduced the replacement pixels on both recipes for all 30 files below.
  [Matched captures and qualification](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/README.md).
  Paths are relative to `examples/v1-game/tests/fixtures/exact-images/native/`. The SHA-256 values
  are for the retained files, old → new.

  - `letterbox-bgra8-1/mirror-light-present.png`
    `9418a8a0b8d942d17670a291b52d0563dbc447bccfb8cc9d699c545c56b531ca` → `08b0b381e68cce570a833a9992eea0c05178db243589b3666aacb8840d427e83`
  - `letterbox-bgra8-1/mirror-light.surface-bgra8.gz`
    `1b7849615dd22c2a16782df833a689e6dbc954728950c8a3df04170c7855256a` → `b887390b59751882b5d790c6c0b93e8a67b09c875bf2bfd02a37cc72a1cc9c45`
  - `letterbox-bgra8-srgb-1/mirror-light-present.png`
    `9418a8a0b8d942d17670a291b52d0563dbc447bccfb8cc9d699c545c56b531ca` → `08b0b381e68cce570a833a9992eea0c05178db243589b3666aacb8840d427e83`
  - `letterbox-bgra8-srgb-1/mirror-light.surface-bgra8.gz`
    `1b7849615dd22c2a16782df833a689e6dbc954728950c8a3df04170c7855256a` → `b887390b59751882b5d790c6c0b93e8a67b09c875bf2bfd02a37cc72a1cc9c45`
  - `night-linear-off/checkpoint-0.rgba16f.gz`
    `9f25daa08b386cb7307e35143d8ff87dfbb2f121bd2ca7b92f28f58424858dfb` → `b6f96a68d0e50416efcdd7e3cef6371065751128e3dd3ed32e1770f1122bd29c`
  - `night-linear/checkpoint-0.rgba16f.gz`
    `c170503a4e75f023373f0d3747807703015301ba2a4e6a8bd7ccd8a68549f64d` → `04cee25b73b1572fab5923e64911f98b5c794868c554586c1a9fc4b9663b4307`
  - `night-sdr-off/checkpoint-0.png`
    `310d19d3d7f228722f78634b0803ee5cadd095eeefa33f13774e63ab6cb65b71` → `7661c2462a5226491caf5575d08f39cde7d0290f66d44ed55cba50477490cae9`
  - `night-sdr/checkpoint-0.png`
    `c918b1ce46739396c676484e89ba846276d391d063bf9d6ff6714bc7978ef63c` → `16fcaaca8141bb5ac48bc63e9450e5ec9c54a972066a75eb3359da1b9fa1cfa9`
  - `scene-linear-1/approach.rgba16f.gz`
    `ed1453aefb0aace9e0ca0b026455ce8377db9c63dea0e803c774f830dfe133fd` → `e6457ff07fab60120d441e0fae3a7e60d98205c267c28487f6cf5359f4db6db5`
  - `scene-linear-1/approach.surface-bgra8.gz`
    `0946edc7a5c34c3c38330c546b0438dec699cdc199748f96b2a1f788b98d9a9d` → `2cd35faec952bdba77acdc888c28aede78898a1a96853652e3710ad1c55e0807`
  - `scene-linear-1/mirror-light.rgba16f.gz`
    `fe6e2e48d8ecbe6e9d03c3dc2cd51ea2823721aff798f962e9272c9a6f51427f` → `51f01a26fd9a10f7c656b3f4cfa451abf783a03bcbc55e18ba5dccb847d8f89b`
  - `scene-linear-1/mirror-light.surface-bgra8.gz`
    `2a2bef0deb3be8bdcd41b7248b3534e037bb19377b8b89d0ef49e35892b0feb6` → `934a6bdbd6e9c09126ccaf6bad03c13ba91d9483fb5e782b490d2981ce46eef5`
  - `scene-linear-1/near-plane.rgba16f.gz`
    `6e8c044b54fc3c254122f56ebf22203559e330c85ff530a8e5cd765df08911a9` → `fbbcdd4378c841f20183e986147e15090bfd712c62fc7821817768ed5597bcc5`
  - `scene-linear-1/near-plane.surface-bgra8.gz`
    `f451a7e8e8b9db7f54c21cae8c6f52861ac3841b9c9a3469e44dcd91176facb8` → `0b2eb247720fccedea511a8ac8418f7f05b56f7f3e71388459b248285abc7857`
  - `scene-linear-1/post-cut.rgba16f.gz`
    `f0caf5013bd2ca2e22b292cab16040171feea1465561245da308f4dce6fee43d` → `b448b7f6efe8b0c793c97d07022ab28f04f077ab2c5ba1e4d11018fd03e2d942`
  - `scene-linear-1/post-cut.surface-bgra8.gz`
    `6d2992b56f96ac0a188da80c446f2bcf37f9d9cad4a6dafe7e53919a8d61159f` → `cd59002d914d1eff1b724dd56658ef6de7409cd15a51cdbacffae73019cd2642`
  - `scene-linear-1/pre-cut.rgba16f.gz`
    `29f987dec401a89254cb6233c85db2b0c2eb427f239a91ee7e47cfb6514615d6` → `5bb15f54aabe4d76cbe0554836a63a8ae9fbcf464e7031071772f625390e0577`
  - `scene-linear-1/pre-cut.surface-bgra8.gz`
    `59eb99f2d3e91fbc9fabfed2e0c404632b4896e9dfc01e5053ff0147e2229278` → `ae18d5691db5b72eb767d80dd2daa395176961a3a3e8715df9b69bc9a7a09244`
  - `sdr-1/approach.png`
    `1946ee6a2ac3a879850c92c0a439fbf830dd18bfdf3f2ba56c5859aef2aa9506` → `809ba76fc94bb1a2b46293c5643e4d860336d44638d182af7dcbbbca989411a1`
  - `sdr-1/approach.surface-bgra8.gz`
    `0946edc7a5c34c3c38330c546b0438dec699cdc199748f96b2a1f788b98d9a9d` → `2cd35faec952bdba77acdc888c28aede78898a1a96853652e3710ad1c55e0807`
  - `sdr-1/mirror-light.png`
    `9a3f08d8fc169349b1f320f22ea4fff8ee388e2ae1acc205ec5ed2ebffd89199` → `74215dfbe907701675d76a0a9fd78bcc62867a5cc558420ba9863ac25163b9eb`
  - `sdr-1/mirror-light.surface-bgra8.gz`
    `2a2bef0deb3be8bdcd41b7248b3534e037bb19377b8b89d0ef49e35892b0feb6` → `934a6bdbd6e9c09126ccaf6bad03c13ba91d9483fb5e782b490d2981ce46eef5`
  - `sdr-1/near-plane.png`
    `488266a2532964b67d7d1bf469876d20c49bba7345ec46a1fb7a251f4e57e8ed` → `0393da6c4ee20e9cfb9921a174712689284620c803a09f0ccbf621db5a0ebfc1`
  - `sdr-1/near-plane.surface-bgra8.gz`
    `f451a7e8e8b9db7f54c21cae8c6f52861ac3841b9c9a3469e44dcd91176facb8` → `0b2eb247720fccedea511a8ac8418f7f05b56f7f3e71388459b248285abc7857`
  - `sdr-1/post-cut.png`
    `956a46f9808acb3a458c11a17c795860047d0fe9f794ff43d06edfc7341d7062` → `8b4b7ae010bec957b64079f98ca5afbfe8a3e02b952a581e2d5d908574a28bca`
  - `sdr-1/post-cut.surface-bgra8.gz`
    `6d2992b56f96ac0a188da80c446f2bcf37f9d9cad4a6dafe7e53919a8d61159f` → `cd59002d914d1eff1b724dd56658ef6de7409cd15a51cdbacffae73019cd2642`
  - `sdr-1/pre-cut.png`
    `8d591edd6eda944dd32b45ce4669200de77b51a3491b2afc2686ad78f79d82fc` → `2b5c8522e79a38c663bcfe3a6da0bbadbe3575d9179ee6e6111c558b808b14c2`
  - `sdr-1/pre-cut.surface-bgra8.gz`
    `59eb99f2d3e91fbc9fabfed2e0c404632b4896e9dfc01e5053ff0147e2229278` → `ae18d5691db5b72eb767d80dd2daa395176961a3a3e8715df9b69bc9a7a09244`
  - `sun-linear/checkpoint-0.rgba16f.gz`
    `7e4d9e816d32e4c19221235e491222477817e69a70f451278a477f0adfdbb31b` → `073c6ac95ed8952597cc97a483e399ed2898b68357dd65782ef6d0e0430e1254`
  - `sun-sdr/checkpoint-0.png`
    `37951182b40582a636fba0db65d1b0a81ac169c38a4cfd91d73331b0153c6d75` → `3a69a89679180afe8d8045b917cbf5408a1c41d9d2b7667bbfd494b263f9e005`

- 2026-11-23 The lead extended the [mip-bias ruling](../ARCHITECTURE.md#ruling-on-native-mip-bias)
  for APPLE-PRESENTATION M14 to all four Chrome WebGPU baselines. In matched captures, the designed −1 bias
  at native width changed every decoded frame. The zero-bias spatial control recovered all
  1,924 retained frames and both arms matched every registered frame identity.
  The SHA-256 pairs below give the old and new compressed manifest bytes.
  [Chrome control and capture record](../../../.orchestrate/evidence/APPLE-PRESENTATION/M14/chrome-bias-diagnostics.json).

  - `chrome-day-sdr`
    `19ea9419d8ebf692647702a3d76f4cbe928edbc65446ff8b1d9fc414578aaccc` → `19f75ffb486246f29554120f8ce8fa5b5abaa4132ff32139d114e8757b2fcbe9`
  - `chrome-day-scene-linear`
    `e14c7756bc41b6108485ea3e3d05337eb3c14a9d5b59711326e4ad94e55cd8b7` → `3d01ea3580e614c5deb4ea0bf3a2cc13e792e7c1b674c5029ad93dae9002d7f0`
  - `chrome-night-sdr`
    `ed49185b614138d97531a4d103fd4a7a3aee05087c56b3744c706c17c4100557` → `3b1acc0b7cd40e190f0df280c24d2a0dc26be84ca628101505f8df8c74959dcb`
  - `chrome-night-scene-linear`
    `94f57fabf1be209376074776ca1cfe9db1254109ae0761382aa484c275c01c0a` → `c71bee067d8607eb65319358f3babf6dff9cd66be6ca73118e40c82354ea3b9b`

- 2026-11-23 M6. The Mac configures a float drawable with a retained AppKit layer and applies current headroom
  in the prepared present pass. All seven target commands passed, including H=1 identity on both Metal recipes,
  drawable bounds through the headroom step and SDR lease continuity on the iPhone simulator. Builder and
  independent visual inspection pass the Sun, sky and four night lamp cores at H=2 and H=8.
  [Target receipts and crops](../../../.orchestrate/evidence/APPLE-PRESENTATION/M6/README.md).
