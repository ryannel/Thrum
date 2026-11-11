# Cooked draw path plan

Design: [COOKED-DRAW.md](COOKED-DRAW.md) · [brief](COOKED-DRAW-BRIEF.md) · Done when:
[COOKED-DRAW-DONE.json](COOKED-DRAW-DONE.json). Owner: G5 (`thrum-asset-gpu`), under
[the G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md).
Reviews: [revised design review](../../evidence/COOKED-DRAW/design-review.md) of `2394162b` is DESIGN OK. The lead owns implementation review.
Milestone: M1 Cooked scene draws on adapters without indirect execution (claimed)
Now: The console witness correction passes exact standalone M1.7/M1.8 and fresh rotation/visual runs.
All four modes retain before/after console observations and denied read-only Screen Recording preflights from
identified actual launcher parents. Accepted M7 supplies oriented scene sizing. The refreshed claim is awaiting
loop verification and independent lead review.
Next: The lead verifies the integrated claim and runs independent implementation review.
Blocked: Nothing in the builder scope.

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
  record that row's reason and continue the engine-state rows with console state recorded as a witness. The owner
  then decides whether to rescope the item as attended.

Checks: `./dev check` under `gpu` for code. This console witness revision passed `./dev check-docs` and
`git diff --check`. The definition audit preserves accepted milestones and all builder/reviewer proof fields.
Scratch goes in `tmp/cooked-draw/`, under [the storage rules](../../ASSET-LIFECYCLE.md).

## Console witness correction and verification handoff

The builder owns `scripts/cooked_draw_ios_sim.py` and the affected cases in `scripts/test_t2_capture.py`. Remove
`wait_for_console_lock`, its 120-second manual setup wait and every before/after lock state refusal, across ordinary,
`--lifecycle`, `--rotation` and `--visuals` modes. Retain actual before/after observations and raw probe output,
recording unavailable observations with a reason. Add read-only denied Screen Recording preflights before and after,
with explicit launcher/harness permission identity, following the existing ungranted-launcher attribution pattern
where needed. An unrelated denied helper is insufficient. No permission is requested or changed. Existing simulator,
content, advancement, lifecycle, cleanup and validation failures still fail their rows.

Replace the manual-lock admission/timeout tests with controls accepting locked, unlocked and changing observations
without waiting, retaining unavailable-probe reasons, and refusing missing, failed or granted permission preflights
for the independence proof. Run `./dev check` and the exact standalone M1.7 and M1.8 commands under `gpu` on fresh
owned simulators. Re-run any other mode affected by the shared harness change as required for the refreshed claim.
The loop stopped verification before review, so the existing M1.7/M1.8 pass fields and evidence are historical
claims under the superseded wording. Fresh evidence and loop verification are owed before review. Preserve prior
lock witnesses in the history. The loop records the revised definitions and clears obsolete proof. This owner scope
correction spends no failure re-plan and leaves `fails`/`replans` to their existing values. Independent review of
the revised definitions and refreshed claim is owed to the lead/loop; no other agents may be launched here.
