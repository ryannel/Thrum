# GAME-UI log

- 2026-11-13 Planned from the lead's task, "Plan module GAME-UI. It needs: RENDER-2D" (row 20 of the module
  list); no words of the owner's were passed on, so the design quotes the owner's page of 10 November. Five
  milestones replace the brief the module list carried, whose text is in git
  (`git show 8f038dec:docs/plans/ARCHITECTURE.md`). M1 waits on `RENDER-2D:M5`.
- 2026-11-13 Planner's decision on the text stack, left to the planner by that brief. Glyphs are rasterised when
  a font is cooked, at the sizes and whole scales a game names, with `skrifa` 0.50.0, `harfrust` 0.16.0 and
  `zeno` 0.3.3 in `thrum-cook` only (versions and licences read from the crates.io registry on 13 November; all
  MIT or Apache-2.0). A shipped game reads a table and holds no font code, so the Wasm half of the audit has
  nothing to decide. Not chosen: a glyph cache filled while the game runs, which is what Dear ImGui 1.92 and most
  general interfaces now do, because it puts a parser and a rasteriser in the shipped game and needs a sheet
  whose pages change, which `RENDER-2D`'s approved design does not offer; and distance fields, which soften small
  text. The module list's ownership row no longer names a glyph cache.
- 2026-11-13 Planner's decision on the join screen, also left by the brief. A ticket is about a hundred
  characters (`TRANSPORT`), so it is never typed: it arrives as a text drop from a link, a launch argument or a
  paste. No text field and no on-screen keyboard is built. It is an assumption in the plan.
- 2026-11-13 Planner's decision on dependencies. The brief said `RENDER-2D`'s first milestone would do. The
  interface stage and the sprite pass's second use are built in `thrum-render` files that `RENDER-2D` M2 to M5
  change, the font product stands on the sheet product of its M4, and the simulator is first reached in its M5.
  So every milestone here follows `RENDER-2D:M5`, as `SESSION` follows `SIM-2D`'s last. The module table, the
  build order and `RENDER-2D`'s paragraph in the module list say so now; `RENDER-2D`'s own files did not change.
- 2026-11-13 Planner's decision on the asks of other modules. The interface stage (`APPLE-TILE`,
  `APPLE-PRESENTATION`), the sprite pass's second use (`RENDER-2D`), the font product (`CONTENT-LOOP`) and
  input, density, insets, text drops and an interface scene in the shells and the V1 game (`CREATOR`,
  `SHOWCASE`, `T2-CAPTURE`, `PLATFORM-RUNTIME-IOS`) are built by this module's builder under four seam rulings
  in the module list, as `RENDER-2D`'s and `SESSION`'s were. No other module's design, plan or done-when file
  changed, and no contract's text changed. It is an assumption in the plan.
- 2026-11-13 Planner's decision on blending. The interface blends in linear light, through an sRGB view of the
  display image, as sprites do and as FreeType's guidance asks of text. Blending in display space would match
  paint tools and would need a second form of the sprite pass, and on an EDR surface a layer of its own. It is
  an assumption in the plan, tied to `RENDER-2D`'s.
- 2026-11-13 Planner's probe, in [the evidence folder](../../../.orchestrate/evidence/GAME-UI/planning-probe.md). One Latin face at
  four sizes is 1.6, 5.4 and 11.4 MiB at scales of 1, 2 and 3, so a font cooks to one sheet for each scale and
  only the one in use is resident. Rounding the pen leaves the gap between two glyphs a pixel out depending on
  where a line starts, so advances are rounded once, at cook, and a run is rigid. The event-ownership rule broke
  none of its four properties in 20,000 random interleavings. Direction keys reach every control and reverse in
  96 grids; a ragged row does not reverse and 4 of 2,000 random scatters strand a control, so the design claims
  rows, columns and grids only. None of it ran on a GPU.
- 2026-11-13 Planner's findings in the code. The tone stage writes encoded sRGB into an `Rgba8Unorm` image that
  capture reads and present copies, so a stage after it is in every capture. The Mac shell passes a game key
  codes only, the page keeps its own DOM listeners and the iOS shell turns touches into one gesture, so routing
  needs work in all three shells. `scripts/check_v1_browser.mjs` already sends real keys through the browser's
  debugging protocol and the iOS shell has a UI test that makes real touches; nothing can post events to a Mac
  window without a permission, so Mac proofs feed events in at the shell.
- 2026-11-13 Researched: FreeType's text-rendering guidance, Dear ImGui 1.92's release notes, Clay's README, the
  W3C's spatial navigation draft, and the crates.io registry. Not read, and marked so in the design: egui, Bevy
  and Slate on glyph caches; iOS and Chrome on pasting; the WebGPU rule for sRGB views, which M1 confirms.
- 2026-11-13 Checks: the done-when file and the plan file pass the loop's validators. Every check but the
  verifier was run through the loop's own `try_check` and each came back `fail`, none `error`, as it should
  before the work exists. They were run without the `gpu` lock, which a builder held: each stops before it
  reaches the GPU. The verifier items were not dry-run, because the GPU is shared.
- 2026-11-13 The design has not been reviewed. No builder starts until a design review returns Design Approved.
  The reviewer should also read the four seam rulings in the module list, which this design depends on.
- 2026-11-13 [The design review](../../../.orchestrate/evidence/GAME-UI/design-review.md) of `faa2d843` asked for changes: two
  Must Fix findings and two suggestions. This entry and the three below are the planner's answer.
- 2026-11-13 MF1, the stage had no form for the EDR output `APPLE-PRESENTATION` designs. Mechanisms 1 and 2
  and ruling 1 now have two display modes: the 8-bit mode unchanged, and a float mode in which the stage draws
  into the present pass's `Rgba16Float` target after the transform, with white at 1.0. M1.1, M1.2, M1.4, M1.5,
  M1.6 and M1.10 were written again. `APPLE-PRESENTATION`'s files did not change.
  [What was decided and why](../../../.orchestrate/evidence/GAME-UI/design-review-response.md#mf1-the-float-mode).
- 2026-11-13 MF2, a font edit could draw new glyphs at the old layout. New mechanism 11: a paint list carries a
  stamp of the sheets it was measured from, the stage draws it only with those, and the interface is a reader
  of the publication (ruling 3). M2.6, M2.8 and M2.9 were written again, M3.14 is new, and M4.2 gained a
  sentence. `CONTENT-LOOP`'s files did not change.
  [What was decided and why](../../../.orchestrate/evidence/GAME-UI/design-review-response.md#mf2-a-list-and-its-sheets-change-together).
- 2026-11-13 Suggestions. S1 taken: "element" is defined in the design, and M3.5, M3.8, M3.9, M4.10 and M3's
  quality bar now forbid one element covering another, not any two nodes overlapping, which the tree does on
  purpose. S2 taken in part: durations and repeat counts in M1.9, M2.13, M3.9, M3.10 and M4.9 became what the
  capture must cover. Sizes, scales and states stayed, because they are the range each item claims.
- 2026-11-13 Checks after the change: the done-when file and the plan file pass the loop's validators and
  `./dev check-docs` passes. The one new check, M3.14's, came back `fail` through the loop's `try_check`, as it
  should before the work exists; the other commands are unchanged from the first plan. The revised design has
  not been reviewed, and no builder starts until a design review returns Design Approved.
- 2026-11-14 Owner: "Let's move the 2d engine side of things to before v1 and decouple them. But we don't want to start working on the swarm game till v1 lands." M1 now names both milestones it uses:
  `RENDER-2D:M5` and `APPLE-PRESENTATION:M6`, whose EDR present pass the interface stage is drawn in. The
  level-up choice is worded as a modal choice of four cards in the design, the plan and eight items; what is
  built is the same. One iPhone shell for every game, engine work that `SWARM` M6 had carried under its ruling
  3, is now mechanism 12 and a new M6 here, proved with the crowd view as a second app. The revised design
  needs a design review.
- 2026-11-25 Planner, on WP1's `BLOCKED needs_dependency`: the builder's finding holds at `05437b579`. A 2D host
  has no float display mode, no float capture and no change of mode, so WP1.1, WP1.2 and WP1.5 cannot be met
  over the sprite scene. The missing part is planned as `RENDER-2D:WP6` under that module's seam ruling 1, and
  WP1 now waits for it in place of `RENDER-2D:WP5`. No item and no mechanism here changed.
- 2026-11-25 Resumed WP1 after RENDER-2D WP6 passed review. The interface uses the sprite pass after
  the 8-bit transform and inside the float present pass. The before graph snapshot came from Metal on
  `cbb05f15b`, with the float prerequisite landed. Metal and Chrome qualification and timing records
  are in the [WP1 working folder](../../../.orchestrate/evidence/GAME-UI/WP1/).
- 2026-11-25 WP1 proved authored paint and unchanged uncovered pixels on portable and Apple tile Metal in both
  display modes, plus resize and replacement on Chrome at DPR 1 and 2. The absent graph and resource census
  matched the before archive. T2 stills and moving captures kept the card fixed over the sprite field and valley.
  The two Metal timing runs flagged the absent sprite-field GPU span (+133% and +37% under load). The matching
  graph, resource census and sample minima did not isolate a cause. The valley deltas were −1.1% and +3.4%.
  [Visual review](../../../.orchestrate/evidence/GAME-UI/WP1/visual/review.md) ·
  [Timing investigation](../../../.orchestrate/evidence/GAME-UI/WP1/costs-mac-metal/investigation.md)
- 2026-11-25 WP1.3 review fix compares all 12 V1 cases in both display modes on each Metal recipe. The
  MetalFX request resolves natively on Apple tile and to TAAU on portable. Pass order, attachment storage
  and transient slots match the untouched pre-interface parent `91d2cf288`. The named renderer check also
  matches the host's live buffer count, and the opt-in allocation control still passes.
  [Before-build provenance and target runs](../../../.orchestrate/evidence/GAME-UI/WP1/review-graph-coverage.md)
- 2026-11-25 WP2 cooks Roboto Regular and an original pixel grid into validated font tables and pixel-kind sheets.
  The interface and sprite passes draw whole-pixel runs from the leased sheet. The existing residency publication
  waits for a prepared layout, and commits the matching stamp only on submission.
- WP2 proved 1×, 2× and 3× on both Metal recipes and Chrome WebGPU, including live source edits, discarded switch
  frames and last-good retention. Fixed and realtime inspections cleared the type and counting cards.
  The combined committed font textures held 3,922,212, 9,298,212 and 16,394,532 bytes at the three scales. The cook
  and layout measurements record both release runs and the machine load.
  [Proof index](../../../.orchestrate/evidence/GAME-UI/WP2/index.md)
- 2026-11-25 WP2 review fixes. A late refusal restores the committed font offer before binding and
  drops the pending publication. Font planning selects the input named in the description, preserving
  unrelated sibling products. Mac Metal proved the late refusal and exact opaque texels. Chrome WebGPU
  rejected both mixed metric and sheet references at every tested density, and both release cost runs
  recorded every cooked sheet. [Review fix evidence](../../../.orchestrate/evidence/GAME-UI/WP2/revision/index.md)
- 2026-11-25 WP2.8 review fix. The cooker publishes a font's proposed PNG bindings only with an accepted
  font product. A rejected grid rename leaves HEAD and the private grid intact across reopen. An independent
  published texture edit still lands, and a later valid rename publishes its new bindings and font together.
  The Mac Metal publication checks and all named WP2 checks passed. [Proof](../../../.orchestrate/evidence/GAME-UI/WP2/round3/index.md)
- 2026-11-26 WP2 review fix. The cooker records the last good font input binding with the manifest. A new
  sibling PNG now publishes all three profiles during a refused outline edit or grid rename. Reopening keeps
  the refused rename's old grid private and the proposed input's public hashes intact. The two valid fixture
  font products matched the previous cook byte for byte. [Proof](../../../.orchestrate/evidence/GAME-UI/WP2/round4/index.md)
- 2026-11-26 WP2 review fix. The cooker recovers the private grid of a legacy pixel font from a saved cook
  action that matches the committed product. A refused cell-size edit or rename retains the font and its
  bindings across reopen. A later independent PNG publishes all three profiles. Verified on the reviewer's
  archived store and a regression covering a previously public PNG proposed by a refused rename.
  [Target proofs](../../../.orchestrate/evidence/GAME-UI/WP2/round5/index.md)
- 2026-11-26 WP2 review fix. Opening the reviewer's store after the old cooker refused a pixel font edit
  now allows independent new PNGs to publish all three profiles. Reopens retain the font hash and keep
  the original grid private. Restoring the valid description records the binding. Unrelated stat entries
  continue to persist while the legacy font is unresolved. [Target proofs](../../../.orchestrate/evidence/GAME-UI/WP2/round6/index.md)
- 2026-11-26 WP2.9 verifier fix. The browser had already passed the font-edit comparisons when Chrome's
  teardown watchdog fired. The text check now ends capture, retires each WebGPU device and unloads its
  page before closing Chrome. Two Chrome WebGPU runs passed at DPR 1, 2 and 3 with clean shutdown.
  The fixed type cards at DPR 1 and 2 matched the inspected pixels. [Target proofs](../../../.orchestrate/evidence/GAME-UI/WP2/round7/index.md)
- 2026-11-26 WP3 adds the retained tree and the status display over the 10,000-sprite field. The scene uses
  the same tree for current and prepared sheets. Mac Metal and Chrome WebGPU matched the CPU reference,
  including first submissions after resize. Metal also compared every submission across font, art and scale
  changes. The fixed and realtime captures cover both phone orientations and the full health cycle.
  The 1× Retina labels are clear in native-size crops and harder to read in a reduced whole-frame preview.
  Two release runs on each target record the tree and frame costs.
  [Target evidence](../../../.orchestrate/evidence/GAME-UI/WP3/index.md)
- 2026-11-26 WP3.12 verifier fix. The Chrome probe waits for a presented callback before it polls GPU
  completion and reads the counters around that callback. The clean-checkout command retained two release
  runs with layout and paint costs at 100, 500 and 2,000 nodes and 32 timed status frames per run.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/costs-chrome-retry/index.md)
- 2026-11-26 WP3.7 verifier fix. Chrome reuses one tab and unloads each DPR's page after retiring its
  device. The three DPRs matched the CPU reference before and after resize and Chrome exited cleanly.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/costs-chrome-retry/chrome-fixed/run.json)
- 2026-11-26 WP3.12 cold-start fix. A Chrome canvas can present before the status sheets are resident.
  The probe tracks retirement of each presented frame and collects costs once a timed status frame is
  available. The clean-checkout command passed both release runs, with 32 timed status frames in each.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/costs-chrome-retry/final/summary.json)
- 2026-11-26 WP3 review fixes. Capped shares release slack even at extreme weight ratios. Generated trees
  cover both logical endpoints and fixed-size text changes repaint without relayout. Mac Retina scale-one
  capture uses larger cooked type at display density two. Every Chrome cost sample covers an actual
  one-tick update, and the Metal switch test compares changing values on every submission. All ten
  automated checks and the 18 fixed Metal retakes passed.
  [Review fix evidence](../../../.orchestrate/evidence/GAME-UI/WP3/review-fixes/index.md)
- 2026-11-26 WP3 second review fixes. Fitting stacks measure their cross-axis alignment. Hidden text edits
  keep the visible paint list and appear when the subtree is shown. The status scene selects compact sizes
  when the safe area is too short for the large panel. Mac layout tests and the 97,435-case sweep passed.
  Fixed Metal captures show all three counts in the short safe area and preserve the larger Retina text.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/second-review-fixes/index.md)
- 2026-11-26 WP3.3 review fix. Hidden anchor, visibility, tint and bar edits keep the visible layout and
  paint list cached. Revealing the subtree applies the retained edits in one layout and matches a fresh
  tree. The layout properties suite passed on macOS, and the Metal comparison confirmed idle
  submissions upload zero interface bytes.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/hidden-edits/index.md)
- 2026-11-26 WP3.12 verifier fix. The cost check gives Chrome time to drain after the status page
  unloads and records the owned process group's exit. It accepts the logged teardown watchdog after
  a complete take only when shutdown was requested and all owned processes have exited. Both Chrome
  WebGPU release runs passed with 32 timed status updates each and normal browser exits.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP3/chrome-cost-shutdown/index.md)
- 2026-11-26 WP4. The interface routes complete key, pointer and touch contacts against the last
  submitted layout. The V1 choice suspends and re-acquires the game's held input, and the scripted
  join scene accepts whole tickets from the shells. Eight automated checks passed, including real
  device replacement on both targets. The fixed stills, 1× motion take and two release measurements
  per target are retained in the [WP4 proof](../../../.orchestrate/evidence/GAME-UI/WP4/index.md).
- 2026-11-26 WP4 review fixes. Disarming suspends game contacts and returns those that worked no control.
  Touch motion and release hide focus styling. The Mac route records the player position from the first
  choice submission onward, and the T2 take shows opening, focus moves and closing on the next submission.
  Chrome recorded renderer-thread CPU batches from actual routed browser events and confirmed focus and
  press increased paint-list work without new trees or layout passes. Both release targets and the
  workspace check passed. [Target proof](../../../.orchestrate/evidence/GAME-UI/WP4/repair/index.md)
- 2026-11-26 WP4.6 merged-check fix. Each scripted Mac launch wakes the display and retries once
  on the zero-callback startup failure. The existing always-on-top mode keeps the proof window
  visible, and the shell filters unrelated desk input while the script runs. The release window
  retained Guard through Metal device generation 1 to 2 and completed both pasteboard calls.
  [Target proof](../../../.orchestrate/evidence/GAME-UI/WP4/drops-display/index.md)
- 2026-11-26 WP5 connected the V1 status display, choice and join screen to UIKit's view geometry, touch
  stream and opened URLs. The iPhone 18 Pro simulator proved scale-3 status pixels and safe areas in two
  orientations, choice and focus retention across backgrounding, one activation per card, and held-stick
  handoff and a whole invite ticket. The builder inspected the six paired stills. On Mac Metal the
  creator guide's example and window commands ran, and the dependency and maturity audits passed.
  [Proof index](../../../.orchestrate/evidence/GAME-UI/WP5/index.md)
