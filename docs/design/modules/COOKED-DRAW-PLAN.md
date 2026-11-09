# Cooked draw path plan

Design: [COOKED-DRAW.md](COOKED-DRAW.md) · [brief](COOKED-DRAW-BRIEF.md) · Done when:
[COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json). Owner: G5 (`thrum-asset-gpu`), under
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md).
Reviews: [revised design review](../../evidence/COOKED-DRAW/design-review.md) of `2394162b` is DESIGN OK. The lead owns implementation review.
Milestone: M1 Cooked scene draws on adapters without indirect execution (claimed)
Now: All twelve M1 items pass; M1 is claimed for lead review. Accepted M7 supplies oriented scene sizing.
Plain `./dev check` passes 666 Rust tests and the wrapper regression cases. Tripwire metrics pass against same-load BASE history, including quiet.
Next: The lead independently reviews this claim after required land integration.

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

## M1 Cooked scene draws on adapters without indirect execution: claimed

Work, riskiest first:
- [x] **Simulator first.** Build the record field, the surface field, selection and the direct draw. Then attach
      the cooked valley in the iOS route and run it on a fresh owned simulator, before any polishing. This settles
      whether the simulator reveals a further unsupported feature. A feature this module owns is fixed here.
      Anything owned elsewhere is reported `BLOCKED`, naming its owner (→M1.7).
- [x] Adapter-less records hold unknown downlevel facts and never select indirect. Add a test that runs
      `FrameHost::new` → cooked attach and proves the direct path is selected with no indirect-usage buffer
      (→M1.1).
- [x] The diagnostic record mask, and the cooked GPU suite run under both records in `./dev check`. Move the suite's
      indirect arm and the `cooked_edit` bench off `FrameHost::new` and onto adapter-backed `capture`, and make each
      assert its selected path. The suite covers discard, capture-no-history, device replacement, remote feed,
      material promotion and casters. The motion-history flag moves to a step both paths run (→M1.1, M1.2, M1.6).
- [x] `scripts/cooked_draw_parity.py`, built on the T2 Mac and browser taps:
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
- [x] Record `docs/evidence/render/cooked-draw/BASE`: the parent of this module's first commit in the integrated
      history. Integrate the module's commits contiguously on the main tip, so that BASE..claim holds only this
      module's changes. Re-record BASE after each rebase.
- [x] A command diff of the indirect path against BASE, the G5 tripwires, and paired, order-alternated
      indirect/direct measurements on Mac, labelled with their load class (→M1.6).
- [x] `scripts/cooked_draw_ios_sim.py`, derived from the T2 M2 probe, without its source patch. It creates and
      deletes a fresh simulator, then builds, cooks, installs and launches with the store in the app container. It
      checks the route state, requiring presented and cooked submitted frames to keep advancing. It takes `simctl`
      stills and a pan sequence, and records the pan's coverage: presented frames, samples and their times, holds,
      and the largest gap. It runs a `--lifecycle` mode (background/foreground, device replacement) and a
      `--rotation` mode, which reuses the route's existing `rotation` row with cooked content attached. It fails on
      errors and never skips (→M1.7–M1.11).
- [x] Inspect the simulator stills, including the rotated ones, and the pan sequence against the M1.10 and M1.11
      inspection lists. State the pan conclusion only within its recorded coverage, and leave proof over every
      submitted frame to T2-CAPTURE M2. Keep the lists, coverage records and crops in the evidence.
- [x] Update the G5 decision (meshlet section, capability inventory, open items), `maturity.json`, the evidence
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
- 2026-11-09 Resumed from rebased checkpoint `478c17ff` under the revised, reviewed M1.3. Added compile-time opt-in reference/omission controls, independent cooked-product coverage validation and command/resource observations. Normal builds omit these controls. M1.3 passes exact D/R and I/D/R repeats in every required view, detects each D-only terrain omission, retains R output/arguments across the defect and restores equality. All-meshlet I controls are exactly I in every view. Whole-frame/native-crop inspection classifies the residuals as partition-dependent shading with matching geometry, materials, lighting, shadows and volume. The low-level cause remains unverified.
  - Direct loading now omits unused meshlet range/bounds buffers. The material GPU suite covers both records. Device replacement covers unchanged paths and capability re-selection in both directions. The qualification feature exposes actual buffer usages and bound commands for the public adapter-less-host assertion, including a child calling the exact game attachment entry point. `./dev check` includes that observed-resource arm and passes 664 Rust test/doc-test results; final feature-enabled all-targets Clippy also passes.
  - M1.9 runs the real XCUITest rotation row on a fresh owned iPhone 18 Pro / iOS 27.2 simulator with both console-lock witnesses true. Process/window/device identities persist and cooked submissions advance at each new extent.
  - M1.10 fails its current-extent fill requirement. Observed presenter viewports are portrait `[0,972,1206,678]` inside `1206×2622` and landscape `[239,0,2144,1206]` inside `2622×1206`. **Owner dependency** is G3b iOS/render runtime support for matching scene/internal render aspect to current oriented extent on the same process, window, lease and generation. `resize_surface` updates the presenter and retains the internal source extent. This builder leaves that owning-runtime work for its owner, as build-milestone requires.
  - M1.4/M1.5 base comparisons, M1.6 cost/tripwires, the rest of M1.10 and M1.11 pan proof remain unfinished. M1.7/M1.8 stand as the owner directed. T2-CAPTURE M2 remains gated on M1 acceptance. Acceptance definitions remain unchanged. No reviewer launched.

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

- 2026-11-09 Resumed after PLATFORM-RUNTIME-IOS M7 review. M1.10's former fixed-source blocker is resolved by the owning runtime. A fresh locked held-view run covers noon start, 36-degree end, low sun, mirrored diagnostic twins/volume and both orientations. Sixteen pan samples span the start/end holds and moving view; native edge/horizon crops are inspected, with the 1.118-second largest sampling gap explicitly limiting the conclusion. Ordinary lifecycle and the existing rotation row also pass on the integrated host without held controls. The rotation harness now checks the selected authored 46-degree valley FOV instead of assuming the 65-degree diagnostic camera.
  - Production indirect BASE/current output and counts match exactly for five Mac views and four Chrome views, with both-build repeats. BASE is the integrated parent of the first implementation commit. Baseline archives receive caller/fixture/witness overlays only; production encoders/shaders are unchanged. Separate Cargo target roots avoid mixed-revision build-cache reuse.
  - Release Mac costs use four order-alternated I/D pairs, 128 GPU-complete samples per arm, and actual host load records. Direct has no contribution compute sample. These are descriptive costs, with no selection or speed promotion. The first tripwire run reported an unseeded quiet class and was not treated as a qualified comparison; genuine BASE bench runs were collected under their actual load classes before the final gate. No reviewer launched; acceptance definitions and planner needs remain unchanged.

- 2026-11-09 M1 claimed: all twelve items pass with named-target evidence. Final integrated-M7 D/R/I rerun reproduces the retained inspected payloads; Mac/Chrome BASE comparisons and both-build repeats are exact. Fresh locked ordinary simulator lifecycle/recovery/rotation and held/sampled visuals pass. All tripwire metrics pass against same-load history; three actual quiet BASE measurements per history were added without changing the encoder. Final `./dev check` passes 666 tests in 62 groups, zero failures; explicit iOS target Clippy passes. Owned bulk scratch is retired after retained-source/evidence verification. Only M1 state and item status/evidence changed in the done-when file; planner needs and review fields are preserved. No reviewer launched. Integration through land is next.

- 2026-11-09 Fixed the loop's plain Check failure. The original launcher inherited the user Cargo configuration's sccache wrapper and exited 2 with empty output. A non-quiet Cargo diagnostic reported Operation not permitted. Earlier successful checks supplied RUSTC_WRAPPER= externally. The launcher now supplies that default itself and preserves explicit caller values. The regression fixture covers unset, empty and explicit wrappers, and rejects the original launcher in the unset case. Plain ./dev check passes 666 Rust tests and all twelve repository Python tests under the GPU lock with the caller environment unmodified. Renderer and target proof remain valid. Only M1 item evidence changes in the done-when record. The correction goes through the required land gate.

- 2026-11-09 Requalified the exact M1.7 and M1.8 commands after the owner manually locked the Mac. The loop failure was the real unlocked-host precondition, confirmed by IOConsoleLocked=No. The harness now allows a bounded 120-second manual setup wait at startup and records lock observations, while missing lock evidence still fails. Seven capture tests cover immediate locked admission, a manual-lock transition and timeout refusal. Exact fresh function proof records twelve advancing observations over 22.1 seconds. Exact fresh lifecycle proof holds background counters and redraws Direct on device generation 2. Both runs have true before/after lock witnesses and deleted their owned simulators. The owner retains manual console setup through loop verification. Acceptance definitions and reviewer fields are preserved.
