# Cooked draw path plan

Design: [COOKED-DRAW.md](COOKED-DRAW.md) · [brief](COOKED-DRAW-BRIEF.md) · Done when:
[COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json). Owner: G5 (`thrum-asset-gpu`), under
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md).
Reviews: Independent review of this revised design is owed to the lead/loop; no agents launched in this planner turn.
Milestone: M1 Cooked scene draws on adapters without indirect execution (open, waiting on iOS extent)
Now: Checkpoint `b0dae34c` exposes M1.10's fixed-640×360 letterboxing failure in both orientations. G3b owns the
missing scene/surface extent transition; the checkpoint is evidence, not milestone acceptance.
Next: Lead/loop design review and acceptance of `PLATFORM-RUNTIME-IOS:M7`, then resume all outstanding M1 proofs,
including unchanged cooked rotation/stills/pan M1.9–M1.11 on the integrated host.
Blocked: Dependency `PLATFORM-RUNTIME-IOS:M7` is now in M1's `needs`. M1 returns to open so the loop sequences the
owner work before resuming G5. Existing parity/base/cost/visual proof remains subject to review and rerun.

## Ownership and agreements

- **G5** owns `thrum-asset-gpu`: path selection, the direct draw, path-independent history, stats, and the cooked
  tests and parity harness.
- **The render owner (Thrum Core, V1 graphics execution; implements F2's C04 registry)** must agree to two additive
  changes, recorded in the design's contracts table:
  - `DeviceCapabilities` records the adapter's downlevel flags;
  - `SceneSurface` lends them.

  Neither changes any signature that call sites use.
- **The V1 game and G3b iOS shell owners** must agree that the iOS route attaches cooked content through
  `attach_cooked_scene`.
- **G3b / PLATFORM-RUNTIME-IOS M7** owns current-oriented-extent scene sizing, including shared host/prepared-device
  changes. M1 needs its acceptance. Diagnostic-scene M7 proof does not accept M1.10; G5 reruns cooked visuals after
  integration. G4 reconstruction/governor work is not needed for this baseline correction.
- **G4 seam 3** shares `draw.rs`. There is no sequencing dependency. Merge with care.

Ask for these agreements before the builder starts. The builder may build against them as proposed. If an owner
objects, the change returns here as a re-plan. The builder never redesigns another owner's interface inside this
milestone.

## M1 Cooked scene draws on adapters without indirect execution: open after re-plan

Work, riskiest first:
- [ ] **Simulator first.** Build the record field, the surface field, selection and the direct draw. Then attach
      the cooked valley in the iOS route and run it on a fresh owned simulator, before any polishing. This settles
      whether the simulator reveals a further unsupported feature. A feature this module owns is fixed here.
      Anything owned elsewhere is reported `BLOCKED`, naming its owner (→M1.7).
- [ ] Adapter-less records hold unknown downlevel facts and never select indirect. Add a test that runs
      `FrameHost::new` → cooked attach and proves the direct path is selected with no indirect-usage buffer
      (→M1.1).
- [ ] The diagnostic record mask, and the cooked GPU suite run under both records in `./dev check`. Move the suite's
      indirect arm and the `cooked_edit` bench off `FrameHost::new` and onto adapter-backed `capture`, and make each
      assert its selected path. The suite covers discard, capture-no-history, device replacement, remote feed,
      material promotion and casters. The motion-history flag moves to a step both paths run (→M1.1, M1.2, M1.6).
- [ ] `scripts/cooked_draw_parity.py`, built on the T2 Mac and browser taps:
  - the `paths` arm on one build: production masked-direct D against opt-in diagnostic full-range indirect R,
    byte for byte at matching draw partitions in every existing view. Derive R's arguments independently from cooked
    ranges/placements and validate/record them. Keep selected production meshlet indirect I as the third arm (→M1.3);
  - same-arm byte-identical repeats of D, R and I for every view, and a D-only visibly missing-mesh negative control
    that must fail exact comparison while R is unchanged, followed by restored D/R equality (→M1.3);
  - I/D unthresholded difference maps and metrics, whole-frame and native-resolution inspection for identical scene
    content, geometry, materials, lighting/shadows and volume, including terrain edges, silhouettes, mirrored and
    edge-straddling meshes. Classify cross-partition residuals with diagnostic controls where needed; edge location
    is not a waiver and the checkpoint's numbers are not thresholds (→M1.3);
  - the `indirect` arm, base against claim, on mac-metal and chrome-webgpu (→M1.4, M1.5).

  Exact comparisons use decoded final submitted bytes. Every arm records selected path, mask, diagnostic mode,
  commands and source/content/submission identity; no shader source patch between arms. Controls are probe-only and
  cannot change selection, production indirect work or output, production direct resources, or cost runs. Cook the
  store once, and use it for both builds. The existing checkpoint command is incomplete and must be updated; its
  original failure is not a new-oracle pass.
- [ ] Record `docs/evidence/render/cooked-draw/BASE`: the parent of this module's first commit in the integrated
      history. Integrate the module's commits contiguously on the main tip, so that BASE..claim holds only this
      module's changes. Re-record BASE after each rebase.
- [ ] A command diff of the indirect path against BASE, the G5 tripwires, and paired, order-alternated
      indirect/direct measurements on Mac, labelled with their load class (→M1.6).
- [ ] `scripts/cooked_draw_ios_sim.py`, derived from the T2 M2 probe, without its source patch. It creates and
      deletes a fresh simulator, then builds, cooks, installs and launches with the store in the app container. It
      checks the route state, requiring presented and cooked submitted frames to keep advancing. It takes `simctl`
      stills and a pan sequence, and records the pan's coverage: presented frames, samples and their times, holds,
      and the largest gap. It runs a `--lifecycle` mode (background/foreground, device replacement) and a
      `--rotation` mode, which reuses the route's existing `rotation` row with cooked content attached. It fails on
      errors and never skips (→M1.7–M1.11).
- [ ] Inspect the simulator stills, including the rotated ones, and the pan sequence against the M1.10 and M1.11
      inspection lists. State the pan conclusion only within its recorded coverage, and leave proof over every
      submitted frame to T2-CAPTURE M2. Keep the lists, coverage records and crops in the evidence.
- [ ] Update the G5 decision (meshlet section, capability inventory, open items), `maturity.json`, the evidence
      README and the T2 handoff. Close out scratch (→M1.12).

Stop rules:
- A failed parity arm whose difference shows the direct path drawing geometry the indirect cull rejected is an
  existing indirect cull defect. Report `BLOCKED` with the diff. Do not touch the indirect path or waive scene
  correctness. A shading-only byte difference does not establish a culling defect.
- A non-deterministic same-path rerun invalidates that arm. Find out why before you claim anything.
- Any D/R byte mismatch fails M1.3; no tolerance or edge exclusion is allowed. Failure to detect the D-only defect,
  to restore equality, or to classify a scene discrepancy also prevents the claim. Do not force parity with per-meshlet
  direct draws or a shader change. Exact derivative/ordering diagnosis is not required unless a failed check or
  unclassified discrepancy demands further investigation.
- If a `simctl` capture or the XCUITest rotation gesture turns out to need an unlocked console or an OS grant,
  record that row's reason and continue the locked engine-state rows. The owner then decides whether to rescope the
  item as attended.

Checks: `./dev check` under `gpu` for code. This planning revision passed `RUSTC_WRAPPER= ./dev check-docs`
(the local sccache wrapper cannot run in this sandbox) and `git diff --check`; all eleven other done-when items
were verified unchanged. Scratch goes in
`tmp/cooked-draw/`, under [the storage rules](../../ASSET-LIFECYCLE.md).

## Log

- 2026-11-09 Replanned dependency after builder checkpoint `b0dae34c` blocked M1.10. Its README and
  `rotation/layout-failure.json` show the source stays 640×360 while surface/presented extents rotate. G3b now owns
  PLATFORM-RUNTIME-IOS M7 for coherent scene, internal target, projection and surface extent changes. Added
  `needs: ["PLATFORM-RUNTIME-IOS:M7"]`, changed blocked to open for dependency sequencing and removed the obsolete
  free-text blocker. All item definitions, including M1.10, are unchanged; fails/replans are unchanged because this
  routes missing owner work, not a four-review failure. No new proof or acceptance claimed. The enclosing loop
  records the definitions and invalidates obsolete proof; fresh design review remains owed. No owner decision.

- 2026-11-09 Re-planned M1.3 at the owner's request after blocked checkpoint `9c230ff4`. The first valley view's
  1,506 differing pixels / max delta 18 persisted with all meshlets kept; one full-range indirect command matched
  direct exactly. This isolates draw partitioning for that view, not the low-level cause. Primitive-order equivalence
  did not justify the old cross-partition exact-byte requirement.
  - Selected exact production-direct versus probe-only full-range indirect parity at matching partitions, preserving
    every named view, final submitted-output tap and exact same-arm repeats. Added independent reference argument
    validation and a D-only visibly missing-mesh negative control with restored equality to catch a real draw defect.
    Selected meshlet indirect remains captured and inspected with full differences, without a fitted threshold or
    ignored edges. Scene geometry/culling defects and visible quality failures still block.
  - Cause identification is not a prerequisite; derivative/quad or ordering explanations remain unverified. A
    partition-invariant shader change is not worthwhile or commissioned: it risks changing supported indirect output,
    material semantics and cost. Indirect selection, production output/work, M1.1/M1.2 and M1.4–M1.12 are unchanged.
  - No owner decision is required within this scope. Changing selection or preserved indirect pixels would require a
    separate decision. Independent design review is owed to the lead/loop; the owner forbids launching other agents.
    M1 returns to open, fails stays 0, replans becomes 1; the loop records new definitions and invalidates old proof.
    No `*-ACCEPTED.json` edited. Documentation and diff checks passed; no GPU or new-oracle pass is claimed.

- 2026-11-09 Builder checkpoint, following the fresh DESIGN OK and building against the proposed additive interfaces as permitted above. The capability record lends known/unknown downlevel flags; unknown selects direct. Direct draws allocate no cull/indirect/count/readback resources; history writes run on both paths. Cooked GPU cases run on adapter-backed and masked records; the bench retains an adapter-backed indirect record. The ordinary iOS selection attaches cooked content and exposes drawn-device generation. The initial simulator feature risk is cleared; no additional unsupported feature surfaced. M1.7/M1.8 pass on the final source: locked-console advancing counters, background hold/resume and a drawn-device witness on generation 2.
  - M1.3 is blocked: 1,506 final submitted pixels differ in the noon valley view, despite exact same-path repeats. Scratch controls disprove a rejected-meshlet explanation and isolate command partitioning. **Proposal to planner:** investigate partition-invariant output, or explicitly reconsider the whole-range strategy or exact oracle. Do not alter the production indirect path or weaken equality within this build.
  - Remaining views, base/Chrome parity, cost/tripwire work, rotation/pan and full visual qualification are unfinished. The parity command refuses incomplete modes. T2-CAPTURE M2 remains gated on acceptance. See the evidence README for actual checks and simulator records. `./dev check` passes (651 tests, including all 13 cooked GPU cases on both paths); documentation checks pass. No reviewer launched, acceptance definitions unchanged, no milestone claim.

- 2026-11-09 Answered the [design review](../../evidence/COOKED-DRAW/design-review.md).
  - F1 (blocking): adapter-less `capture_device` records now hold unknown downlevel facts, and unknown never selects
    indirect. The public `FrameHost::new` → `attach_cooked_scene` route draws through the always-valid direct path.
    Refusing the attach was rejected. M1.1 now requires a test of that route showing no indirect-usage buffer. The
    indirect test arm and the `cooked_edit` bench move to adapter-backed records and record their path (M1.2, M1.6).
  - F2: the simulator pan (M1.11) now claims continuity only within its recorded sample coverage. Proof over every
    submitted frame stays with T2-CAPTURE M2. M1.7 and M1.8 require advancing presented and cooked submitted-frame
    counts.
  - F3: new M1.9 is the cooked rotation row, reusing the route's existing XCUITest `rotation` row. M1.10 adds rotated
    stills. Any attended gesture reason stays separate from the locked engine-state rows.

  Items were renumbered (stills M1.10, pan M1.11, docs M1.12). No proof existed, `fails` and `replans` stay 0, and a
  fresh design review is owed.

- 2026-11-09 Planned, at the owner's direction through the lead, after T2-CAPTURE M2 was blocked
  ([evidence](../../evidence/render/t2-capture-m2/README.md)). The design selects a direct full-range indexed draw,
  chosen from adapter downlevel flags in the capability record. It rejects CPU culling, compaction culling, delayed
  readback, per-meshlet direct draws and a simulator override, on cost and complexity grounds. T2-CAPTURE M2 now
  `needs` `COOKED-DRAW:M1`. The design names an adjacent gap for G3c: synthetic culled modes use indirect draws
  without selection. No implementation, and no agents launched. Independent design review is owed.
