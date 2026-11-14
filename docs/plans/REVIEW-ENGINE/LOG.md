# REVIEW-ENGINE log

- 2026-11-15 M1 implementation: the iPhone's preparation and recovery call one Mac-testable request with content
  features and adapter limits. Core now has only the engine page; game capture and rewind patch callbacks are
  removed, retaining deferred final-tick undo and previous poses. Verified on the Mac: core tests, the two new
  deferred rewind/cold-capture byte comparisons, 42 unchanged V1 simulation tests and focused Clippy. The iOS
  target compiles. GPU/simulator and exact-baseline proof remain pending; initial integration was refused for
  absent baseline proof, and documentation checks report existing missing links in other modules. Details and
  source identity are in [M1 evidence](../../evidence/REVIEW-ENGINE/M1/README.md).

- 2026-11-14 Planned from the owner's request to plan and drive the code review's fixes. It takes the findings in
  crates where no builder is at work, so it starts at once. Checked while planning: no planned module's design
  (`SIM-2D`, `SESSION`, `SWARM`) uses a game-authored page, so H14 removes it. H18 is fixed with no change to the
  cooked format. M71 and the mip filter's defects are parked, with reasons in the design.
- 2026-11-15 Revised after [the first design review](../../evidence/REVIEW-ENGINE/design-review.md) (one Must
  Fix). M3.3 no longer runs the rule engine, which `REPO-RULES` M1 builds: it searches the cooker's manifest and
  sources, so M3 keeps no dependency and can follow M2 at once. The design says which deferred-publication state
  the engine keeps when the game-authored capture goes.
