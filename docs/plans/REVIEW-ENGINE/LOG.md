# REVIEW-ENGINE log

- 2026-11-15 M1 integrated SIM-2D's new consumers. The runtime retains its world digest and profile validation;
  the new core fixture, crowd example and cost runner use the engine page and callback-free rewind API, with
  assertions unchanged. At `56c14e94`, focused core rewind/digest/refusal tests, crowd conformance/input-route
  tests and focused Clippy pass. The crowd runner also compiles for Wasm and iOS simulator with explicit pinned
  rustc. Documentation checks pass. Mac target checks, refreshed simulator proof and integration remain queued;
  [the handoff](../../evidence/REVIEW-ENGINE/M1-handoff.md) names the live batch. Its full project gate runs once
  after the retained target proof is committed, then it merges before releasing the GPU lease.

- 2026-11-15 M1 target proof resumed. The ordinary iPhone 18 Pro simulator run passed start, background/foreground,
  device recovery and both rotations at `3e7b9650`. The retained report names the actual ASTC profile and 29 storage
  buffers per stage. Builder inspection confirmed the cooked scene in the selected views. M1.2 is now passed,
  with proof committed at `2394f61d`. Documentation checks pass. M1.1, M1.5, the full gate and integration remain
  pending. A CREATOR/CLOUDS merge/GPU lock cycle delayed the queued checks and cleared when CREATOR's merge
  process exited. CLOUDS' workspace tests are now progressing. The live proof job and process identities are in the refreshed
  [handoff](../../evidence/REVIEW-ENGINE/M1-handoff.md).

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
