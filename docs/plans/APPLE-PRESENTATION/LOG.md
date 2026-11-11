# APPLE-PRESENTATION log

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
