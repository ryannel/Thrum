# APPLE-PRESENTATION: log entries in full

Entries of [the module log](../../plans/APPLE-PRESENTATION/LOG.md) that ran past a few lines, moved here unchanged on 11 November 2026 when the plans took the orchestrate 1.7.0 shape. The log keeps each one's opening and a link.

<a id="entry-1"></a>

## Entry 1: 2026-11-10

- 2026-11-10 Revised after [the design review](design-review.md) at `4bbe44a4`
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

<a id="entry-2"></a>

## Entry 2: undated

- Status before this migration (7–9 November):
  Milestone: none started (plan awaiting re-review and seam rulings) · Updated: 2026-11-07
  Now: revision 5 replaces the archive options with the owner's direction (since 2026-11-07).
  Next: review of revision 5, Thrum Core's reading, then landing and seam rulings.
  Blocked: APPLE-TILE is parked after [two failed design reviews](../APPLE-TILE/design-review-2.md).
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

<a id="entry-3"></a>

## Entry 3: undated

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

<a id="entry-4"></a>

## Entry 4: 2026-11-07

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

<a id="entry-5"></a>

## Entry 5: 2026-11-07

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

<a id="entry-6"></a>

## Entry 6: 2026-11-07

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
