# APPLE-PRESENTATION log

- 2026-11-14 M3's strengthened reference at `9c1349f2` proves bit-exact repeated
  linear means and both cut controls, but four still comparisons fail. A diagnostic
  without depth rejection improves five of six checkpoints. The next implementation
  retains rejection across the complete reprojected depth footprint and introduces
  unbiased half-float history rounding; WGSL validation passes, with target quality
  still pending. [Second attempt](../../evidence/APPLE-PRESENTATION/m3/attempt-2/run.json).

- 2026-11-14 The first M3 Metal reference run at `55ae06ee` fails the firm no-AA
  comparison at all six checkpoints, despite meeting the absolute ΔE 3 bound.
  Reference-cycle equality and both cut controls pass. Follow-up `5cffce8c`
  corrects jittered depth addressing, steady-colour blend drift and retained GPU
  storage-scale use; Clippy and shader validation pass, with target rerun pending.
  [Retained failure](../../evidence/APPLE-PRESENTATION/m3/attempt-1/run.json).

- 2026-11-14 M3 implementation checkpoint adds the portable temporal resolve, shared
  submitted history, pre-tone integration, an independent linear reference accumulator
  and the submitted game-host oracle. Focused native Clippy, shader validation and
  pinned Wasm compilation pass. Target image quality, motion, timing, full checks and
  integration remain unqualified. [Builder evidence](../../evidence/APPLE-PRESENTATION/m3/README.md).

- 2026-11-14 M3 preparation follow-up separates continuous camera edits from cuts,
  samples local lights from the same game endpoint as the view, and adds an opt-in
  reconstruction witness with an authoritative tick-60 prop replacement. The
  real Metal alternating-slot/reset execution test and both CPU fixture tests pass.
  Native/Chrome motion drivers now expose the five required scenarios; their
  output quality, the full reference oracle and integration remain unqualified.

- 2026-11-10 Revised after [the design review](../../evidence/APPLE-PRESENTATION/design-review.md) at `4bbe44a4`
  (changes requested, one Must Fix), under the task "Revise APPLE-PRESENTATION's design and plan as
  docs/evidence/APPLE-PRESENTATION/design-review.md asks".
  [Full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-1).
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
  [Full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-2).
- Reviews before the loop: [full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-3).

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
  [Full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-4).
- 2026-11-07 Revision 3 answers the G4 re-review:.
  [Full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-5).
- 2026-11-07 Revision 3 also answers the composed-frame re-review:.
  [Full entry](../../evidence/APPLE-PRESENTATION/log-entries.md#entry-6).
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
  [drafts](../../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md); the plan keeps a heading for each, renamed
  for what someone can see. Open assumptions moved from this log to the plan. No item or scope changed.

- 2026-11-11 M1 builder implementation and mac-metal proof completed: continuous submitted 32-phase jitter,
  unjittered geometry/infinity motion, explicit invalid correspondence and the existing-pass motion view.
  Real Metal attachment tests prove exact static zero and independent pan/solar/sidereal values. Fixed-time
  submitted valley sequences and native crops were inspected, including a real simulation mover. Repository
  gate and milestone checks are recorded separately from visual self-review; no performance or G4 closure
  is claimed. [Builder evidence](../../evidence/APPLE-PRESENTATION/m1/README.md); lead review remains owed.
  Integration recheck: the first land gate stopped at the final cooked draw-probe check; the identical pinned
  binary passed all 13 isolated cases without changes. The final land gate serializes test cases, with every
  assertion and explicit concurrency witness enabled; details are retained with the builder evidence.

- 2026-11-11 MF1 repair gates raster jitter on explicit temporal sampling while retaining submitted metadata.
  Metal tests prove consecutive paused colour byte-equality, advancing phases and a jittered positive control.
  Held-camera comparisons at 640×360 and 1920×1080 show stable sphere, skyline and ground edges. Current
  motion sequences were inspected. Independent re-review and performance qualification remain owed.
  [Repair evidence](../../evidence/APPLE-PRESENTATION/m1-mf1/README.md).

- 2026-11-11 M2 implementation checkpoint: added the third forward reactive target, opt-in stored temporal inputs,
  GPU exposure texture and submitted-frame diagnostics; acceptance identities and output-grid keys now follow
  submitted frames. Metal attachment, exposure, populated-host history and cooked alpha proofs pass; Chrome,
  exact baselines and integration remain pending. The iOS simulator shell compiles with background cancellation
  resetting history. [Builder evidence](../../evidence/APPLE-PRESENTATION/m2/README.md). No M2 claim yet.

- 2026-11-11 M2 target qualification passes all 249 renderer tests on Metal and all 32 Chrome cases,
  including W1–W7 and the existing controls regression. Updated culling contribution fixtures for the
  third target and the controls check for the declared 0.05-degree step. D2.5 and D2.7 pass.
  The required checks for D2.1–D2.4 and D2.6 stop at the exact-baseline gate after the fixture repair.
  Its verifier was attempted and refused with LOW_DISK_SPACE (16 GiB required, about 10 GiB free after
  owned cache cleanup). M2 remains open and integration is blocked until a current proof can be made.
  [Target results and blocker](../../evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-12 M2 resumed after disk recovery. Exact verification at `57590699`, rebased onto current main,
  passes 42 native comparisons and all four 481-record Chrome sets under the GPU lock (1,966 exact matches).
  References are unchanged; no device timing qualification is claimed. Documentation checks pass.
  The required milestone checker remains live; D2.1–D2.4 and D2.6 stay open until its result is available.
  [Current proof and target evidence](../../evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-12 M2 builder verification completed on source `c506b2a6`. Resolved the main merge, retained CULLING's shared flythrough, supplied explicit graph descriptor fields and reconciled the temporal attachment snapshot. All seven item commands and the repository gate pass on Metal and hardware Chrome; exact verification passes all 1,966 comparisons. M2 is ready for independent review. Device performance qualification remains owed. [Current evidence](../../evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-13 M2 clean-checkout repair at `8224cda1`: default browser checks build their web bundle instead of relying on ignored output. The 3,600-second checker passes all seven items and the repository gate; hardware Chrome passes 32 cases and W1–W7, with identical Wasm and diagnostic image. Exact verification passes all 1,966 comparisons. The original watcher missed edit remains unconfirmed after 70 stress runs and a passing full gate; its code and assertions are unchanged. M2 stays `in_progress`. [Follow-up evidence](../../evidence/APPLE-PRESENTATION/m2/README.md).

- 2026-11-13 M2 review repair `mac-volume-witness-bypasses-forward-pass`: at `56469803`, a new Mac game-host test reads W3 through the production cooked contribution, forward graph and submitted temporal diagnostic. Portable and Apple-tile each read alpha byte 161 and four uncovered zeroes. Disabling only the production reactive write makes the test fail with 0 versus 161; restoring it passes with byte-identical frames. Production code is unchanged. Strict focused Clippy and documentation checks pass; full checks and integration remain pending. [Fresh D2.6 proof](../../evidence/APPLE-PRESENTATION/m2-volume/README.md).

- 2026-11-13 M2 follow-up verification at `cf0c424a`: the required checker passes all seven items and the repository gate; Chrome passes W1–W7 and 33 regression cases. The page hashes successful Wasm instantiation inputs and checks exact digest and byte length without oversized DevTools replies. Reconciled CULLING's concurrent checker change through the shared tested helper. Exact verification passes all 1,966 comparisons with references unchanged. Rebased onto main's later documentation and Mac qualification script as `e546d4ce`; refreshing its content-key proof and integration gate remains pending. M2 stays `in_progress`. [Retained proof](../../evidence/APPLE-PRESENTATION/m2-volume/README.md).

- 2026-11-13 The rebased M2 checkpoint `7bcf2b02` passes all 1,966 exact-image comparisons in 889.44 seconds, under the GPU lock and without reference changes. Runtime and browser-checker source objects match the fully checked `cf0c424a`; the unrelated Mac qualification script required a new covered-content key. The required merge gate is next. [Integration proof](../../evidence/APPLE-PRESENTATION/m2-volume/integration-exact.json.gz).

- 2026-11-13 M2 merged through `agent_loop.py merge` as `ff093ba7`; the repository gate passed and no upstream files came in. The post-merge milestone checker passes all seven items and the repository gate, including Chrome W1–W7 and all 33 regression cases with the same instantiated Wasm hash. Later main changes contain only CULLING plan records and STATUS.md, so they affect no M2 proof. M2 remains `in_progress` for the lead's checks and review. [Target and integration evidence](../../evidence/APPLE-PRESENTATION/m2-volume/README.md).
