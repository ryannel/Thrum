# APPLE-PRESENTATION log

- 2026-11-10 Revised after [the design review](../../evidence/APPLE-PRESENTATION/design-review.md) at `4bbe44a4`
  (changes requested, one Must Fix), under the task "Revise APPLE-PRESENTATION's design and plan as
  docs/evidence/APPLE-PRESENTATION/design-review.md asks".
  - Finding 1 (Must Fix): D2.5 claimed Chrome's reactive and history behaviour on the strength of the existing
    browser cases, which read neither. The design now defines the temporal-input witness, rows W1–W7, with one
    fixture for both targets and values taken from submitted frames. D2.5 is that witness in Chrome, through a new
    `temporal` case with an `ask` on what it reads. The existing suite moved to D2.7 as a named regression check.
  - Finding 2: the reconstruction reference and tolerance moved into the design and are now exact: 32 distinct
    samples (the cycle run twice only to show the state was held), the held state listed and checked, CIEDE2000
    on the submitted SDR output, and a rule for changing the provisional bound. D3.2 follows.
  - Finding 3: D2.2 is now the declaration, including the star pass's own attachments and graph accesses, and the
    new D2.6 reads positive reactive values back on the Mac (rows W1–W5).
  - Finding 4: the design and the status lines above say that an approval covers M1–M3 and seams 1–4 only, and
    name what the M4–M11 plan run must settle and have reviewed.
  - From the item notes: D3.5 records TAAU's time at M3's 640×360 extent, apart from the 1080p estimate.
  - M1's items, D2.1, D2.3, D2.4 and the other M3 items are unchanged. M2's sizing is 3.5 days.
- Assumption: the Chrome witness covers the same rows as the Mac, the volume and phase-edit rows included. The
  review required only the invalid-history rows and the history identity in Chrome; the fixture is shared, and
  Chrome is where cloud and sunset demos will run.
- Assumption: ΔE is CIEDE2000 on the submitted 8-bit SDR output, since reconstruction is judged as displayed. The
  bound stays at 3 and binds until a plan revision changes it. This replaces the earlier assumption that the
  bound is "restated after M3's first run".
- Assumption: the per-frame history identity reaches Chrome through the game page's own report, so T2's capture
  record is not changed. A builder who needs a T2 record change proposes it here first.
- 2026-11-10 Moved onto the loop from [the module list](ARCHITECTURE.md). M1–M3's done-when lists became items
  D1.1–D3.5 in the done-when file with the same meaning, each with a proof. Two items were added so Chrome is
  proved as its own target: D2.5 (the browser checker still passes with the third target) and D3.3c (the motion
  review in Chrome, split from D3.3). The done-when file is now the authority for M1–M3; the lists below remain as
  the builder's work breakdown. The module list accepts seams 1–4 as
  the design proposes them, so M1–M3 no longer wait on a separate ruling. The atmosphere checkpoint `d2a5342` is on
  main. M4–M11 stay in prose here and are migrated by the next plan run, which restates the sizing.
- Assumption: seams 1–4 of the design are accepted as written, with the G4 builder making the motion-only edit in
  the atmosphere shaders (seam 4) and the third forward target across contributors (seam 3) itself.
- Superseded on 10 November by the assumption above: M1–M3 keep the provisional reconstruction tolerance the design
  states (ΔE p99 at most 3, and below the no-AA path), restated after M3's first run as the design says.
- Status before this migration (7–9 November):
  Milestone: none started (plan awaiting re-review and seam rulings) · Updated: 2026-11-07
  Now: revision 5 replaces the archive options with the owner's direction (since 2026-11-07).
  Next: review of revision 5, Thrum Core's reading, then landing and seam rulings.
  Blocked: APPLE-TILE is parked after [two failed design reviews](../../evidence/APPLE-TILE/design-review-2.md).
  The lead has explicitly commissioned the scoped re-plan answering its seven findings; fresh independent design
  approval/unpark through the loop and delivery of each named milestone remain owed. T2 M0/M1 are done and cannot
  automatically unblock G4. M4–M9 of G3t are removed as deferred optimizations. G4 needs the retained pre-tone input
  seam and native segment, not fused SDR tone/present, compact rows, GPU overflow or shared mirrored ranges.
  COOKED-DRAW owns cooked draw.rs capability/history changes; G4 seam 3 must preserve those on integration.
  Other code commissions wait for the atmosphere graph checkpoint `d2a5342` on `codex/g3s-stars-main-wip`, for interface reads only, pending the reviewed night baseline and the exact final check on main, and Thrum Core approval of the exact seams in the
  design's *Seams* list. Some milestones have further waits:
  - M4 and M6 wait for G3t M3.
  - M8 waits for G3t M11 and independent review of its C06 whole-encoder segment.
  - D9.1's launch-to-ready split can be commissioned once G3t M0's counters exist, ahead of the rest of M9.
  - The upstream contribution (U1) does not gate V1. It follows D9.1 and the metallib and pre-warm layer, and each
    public post waits for the owner's OK.
- Reviews before the loop:
  - G3t handoff reconciliation, 9 November: independent review owed for the changed native drop/reset and dependency
    interfaces below; earlier approvals do not approve these changes. Full G4 plan migration remains owed when G4 is planned.
  - G4 pair @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/g4-pair.md` (with `common.md`), report
    `tmp/g3t-g4-plans/reviews/g4-pair-report.md`: changes required (14 required changes). Answered in revision 2; see
    the Log.
  - Composed frame across G3t and G4 @ `86ec3b6b`, brief `tmp/g3t-g4-plans/reviews/composed-frame.md`, report
    `tmp/g3t-g4-plans/reviews/composed-frame-report.md`: changes required (14 required changes). Answered in
    revision 2.
  - G4 pair re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (N1–N5 required,
    N6–N8 recommended). All answered in revision 3.
  - Composed-frame re-review @ `c507ad3d`, same brief, report section *Revision 2*: approve with changes (11 of 14
    answered; new N1–N9). Answered in revision 3.
  - Composed frame, exact-revision recheck @ `9938b0d0` (Opus 5.5 medium), brief `tmp/g3t-g4-plans/reviews/composed-frame.md`,
    report `tmp/g3t-g4-plans/reviews/composed-frame-report.md` ("Revision 4 recheck"): approve with changes (one leftover
    always-submitted rule, three small gaps). All applied in revision 4a.
  - Archive review of revision 5 @ `16c5d8f7` (Opus 5.5 medium), report `tmp/g3t-g4-plans/reviews/composed-frame-report.md`
    ("Revision 5 archive review"): changes required (sizes buffer for all runtime-sized arrays, per-pipeline cook keys,
    cache patch scope, per-device archives, sizing). All applied in revision 5a.
  - Root's review of the archive revision @ `fe1749c1` (NOTES, "Root review of the archive revision at fe1749c1"): changes
    needed. Under the owner's direction its points are recorded as one-line outcomes in M10, to be designed when M10
    starts (revision 7).

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
- 2026-11-07 Revision 2 answers the G4 pair review and the composed-frame review:
  - **History:** submission-order acceptance, with encoded frames always submitted; output-extent history keys, so
    scale steps need no reset.
  - **Motion:** sky, Sun and star motion at infinity; disocclusion through invalid-history reactive and TAAU depth
    rejection.
  - **MetalFX:** synchronous initialization; `preExposure` and the history domain under the storage scale.
  - **Forward targets:** the reactive target and mip bias named as a contributor seam.
  - **Transitions:** generation builds off the render thread, with the old generation scaled meanwhile.
  - **EDR:** one EDR surface across displays with H = 1 on SDR displays; platform-owned display record and layer
    properties off wgpu's mutex.
  - **Governor:** missed ticks from the display link, plus thermal state.
  - **Checks:** no FXAA comparison; a defined reference and tolerance; display overrides; record-only checks
    labelled.
  - **Archives:** the option-(b) consequence stated, with a conditional M10.
  - **Cross-plan:** dependencies corrected; milestones split; sizing restated.
- 2026-11-07 Revision 3 answers the G4 re-review:
  - MetalFX gets a GPU-written exposure texture, with a costed normalising fallback, instead of a CPU `preExposure`;
  - late GPU execution errors take the device-loss path;
  - the star motion target is replace, and solar change in the sky is a phase edit;
  - the TAAU depth test compares previous-view depths;
  - G4 M3 is sequenced after G3t M3;
  - ticks without a present are reworded;
  - M2 and M4 are resized;
  - effect-tier invalidation is declared;
  - the iOS polling is checked;
  - the upstream contribution is owner-gated.
- 2026-11-07 Revision 3 also answers the composed-frame re-review:
  - coalesced surface reconfigure without drain;
  - the `PreparedSynthetic` split, build cancellation and device loss during a build;
  - the governor freezes during transitions;
  - ticks without a present are labelled a proxy;
  - occlusion stops encoding;
  - an 8-bit session gains EDR through a transition;
  - a MetalFX scale-step sequence is added.
  - Its exposure-domain finding is moot: MetalFX no longer takes `preExposure`, and the exposure texture is tone
    exposure divided by p.
  - Its TAAU depth finding is answered by the previous-subrectangle addressing.
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
