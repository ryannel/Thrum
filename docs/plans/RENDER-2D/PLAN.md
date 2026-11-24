# RENDER-2D plan

A game draws thousands of crisp sprites through a dedicated 2D scene. The done-when file owns the acceptance
criteria. [Design](DESIGN.md) · [Done-when file](DONE.json) · [Log](LOG.md) · [Module list](../ARCHITECTURE.md)

Before building, read the [module interfaces](DESIGN.md#interfaces),
[proof guidance](DESIGN.md#proof-and-builder-handoff) and
[seam rulings](../ARCHITECTURE.md#seam-rulings-for-render-2d).

## Open for the human

- Assumption: RENDER-2D and V1 can share the frame runtime and content code while they build in parallel.
- Assumption: The engine supports both pixel art and painted sprites for top-down and side-on games. The
  current scene limit is eight sheets.
- Assumption: The packed crowd proofs cover 2,000, 5,000 and 10,000 moving sprites. Engine capture and frame
  records establish the refresh result here. Panel measurement belongs to SWARM.
- Assumption: SWARM art can use linear-light blending. The output half handles SDR clipping and receives
  highlights above white for EDR.
- Assumption: Stable overlap matters more than exact height order for sprites within the sort band.
- Assumption: The lead treats a visible pulse in fine patterns during ordinary scrolling as a shipped-game
  defect. WP3 judges the original patterns at normal viewing size and speed. The owner has yet to rule on this
  specific trade-off.

## WP1 A game selects a 2D scene, and sprites reach the screen through the game's own frame with no sky, shadow or light work, on the Mac and in Chrome: done

[Review](../../../.orchestrate/evidence/RENDER-2D/WP1-review.md)

## WP2 Ten thousand sprites follow the simulation, gliding between ticks in an overlap order that holds still: done

[Review](../../../.orchestrate/evidence/RENDER-2D/WP2-review.md)

## WP3 Sprites stay crisp at every window size and pixel density, and nothing shimmers when the view scrolls

Start with WP3.12. Update the `creation-counters` inventory in `crates/thrum-render/tests/scene_2d.rs` for the
already-landed WP3 packing and frame record. Run its focused Metal check and land the test-only fix as a separate
checkpoint on main before starting any other WP3 item. The next green `long-tests` run lets WATERS WP17 and
RAY-TRACING WP3 proceed.

Then tune the [spatial filter](DESIGN.md#6-sampling-two-kinds-of-sheet) for the existing 1.5× scroll and the
full off-grid 2× transition in the Metal pan. Keep the authored patterns, camera paths and backing extents for
comparison. Reuse the camera-sweep proof on Metal and Chrome, and verify bleed neighbours in the uploaded
atlas. Retire the exact-box checks and their tracked tolerance fixtures when the replacement proofs are ready.
Close WP3 with the shared `long-tests` result covering the merged milestone (WP3.13). [WP3 items](DONE.json)

## WP4 An artist saves a sprite sheet and the running game shows the new art in one frame, on the Mac and in Chrome

Start from the lease and content publication delivered by CONTENT-LOOP WP1. Build the cooker and residency
integration under [seam ruling 2](../ARCHITECTURE.md#seam-rulings-for-render-2d).
[WP4.1–WP4.11](DONE.json)

## WP5 The reference crowd of 10,000 is drawn on the Mac and in Chrome, the sprite scene works on the iPhone simulator, and a game maker can start a 2D scene from the guide

Register `examples/crowd-2d-view` in `.config/architecture.toml`. Use SIM-2D's rule package and reference
traces from `examples/crowd-2d`. For the simulator, use the V1 scene selection in `examples/v1-game-ios`
and the runner pattern in `scripts/culling_sim.py`. Update the creator guide and maturity ledger.
[WP5.1–WP5.13](DONE.json)
