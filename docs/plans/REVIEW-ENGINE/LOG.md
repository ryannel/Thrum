# REVIEW-ENGINE log

- 2026-11-15 M1 integrated APPLE-PRESENTATION's landing. The normal merge refused the stale baseline key,
  so the batch refreshed simulator proof and all approved checks under its existing GPU lease. On `1d60e7b9`,
  the inspected simulator route, Mac request, both rewind tests, 42 unchanged game simulation tests and all
  1,966 exact comparisons pass; verifier time is 939.78 seconds. Proof is retained at `bde949ae`, including
  the newly inspected simulator views. [The evidence](../../evidence/REVIEW-ENGINE/M1/README.md) follows the
  retained records; the full gate and normal merge are running again on the integrated source.

- 2026-11-15 M1's final batch reproduced all approved checks on `d2139ca2`: the Mac request and both rewind
  tests pass, all 42 unchanged simulation tests pass, and all 1,966 exact native/Chrome comparisons pass in
  949.94 seconds. Proof is retained at `e26b9199`; the simulator remains applicable because every iOS dependency
  is unchanged from its inspected build. [The compact evidence](../../evidence/REVIEW-ENGINE/M1/README.md)
  names each item and its target records. The full gate and normal merge continue under the same GPU lease.

- 2026-11-15 M1's approved target checks all pass at `bd40a689`: the Mac request gives ASTC and adapter limits,
  both deferred rewind cases and all 42 unchanged game simulation tests pass, and every native/Chrome exact
  baseline matches. The refreshed simulator route and all four inspected views pass on that source, with ASTC
  and 29 storage buffers. [The retained proof](../../evidence/REVIEW-ENGINE/M1/README.md) names source and binary.
  M1.1 and M1.5 now have builder result `pass`; M1 stays `in_progress`. The first full gate failed on upstream
  APPLE-PRESENTATION log links to unmerged evidence; those paths are preserved as branch references for retry.

- 2026-11-15 M1 rebased onto REPO-RULES' new shape checks. Ordinary imports distinguish calls to the iPhone
  wrapper from the two underlying wgpu requests; `shape_rules.py tighten` records those two existing sites in
  their extracted file, with no increase in allowance. The shape checks pass. The full documentation check
  currently reports missing links from upstream APPLE-PRESENTATION and STATUS plan edits. Target proof stays
  queued behind ATMOSPHERE's GPU lease.

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
