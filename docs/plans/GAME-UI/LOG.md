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
- 2026-11-13 Planner's probe, in [the evidence folder](../../evidence/GAME-UI/planning-probe.md). One Latin face at
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
