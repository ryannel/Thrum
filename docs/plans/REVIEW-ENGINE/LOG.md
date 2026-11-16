# REVIEW-ENGINE log

- 2026-11-16 M3 uploads the mapped index section, derives meshlet ranges by prefix sum, writes
  whole Pod drawing records and measures the engine loader in the cooker test and tripwire.
  On `3408034a5`, the mapped Metal readback, unchanged asset-GPU tests, shader reflection and
  edit-to-visible assertions pass; `82709e784` adds actual renderer-material reflection.
  Terrain's owned index/range arrays fall from 711,784 to 5,224 bytes; paired preparation
  medians are 95.000 and 14.458 us, under shared load, excluding staging and driver memory.
  [M3 evidence](../../evidence/REVIEW-ENGINE/M3/README.md) keeps the old tripwire failures and
  three replacement seeds. Exact images use the shared main-branch check.

- 2026-11-16 M2 integration resolves the lockfile conflict with TEST-SUPPORT and adapts only the
  three new GPU tests to its optional harness admission. No assertion or bound changes. Focused
  `cargo clippy --locked --offline -p thrum-sea --all-targets -- -D warnings` passes on the Mac.
  The earlier merge failed compilation at `dfd9ffc62`; target proof remains in
  [M2 evidence](../../evidence/REVIEW-ENGINE/M2/README.md). The normal merge runs the workspace gate.

- 2026-11-16 M2 gives the sea typed resource/pass/group/pipeline keys, whole Pod shader records and
  reflection checks, a persistent mode allocation, and a transition into retained spectrum storage.
  On `977d3b0a2`, all twelve sea tests pass on Metal, including every unchanged SEA test; all twelve
  model tests pass too. The paired debug upload medians are 38,669.500 and 1,683.166 microseconds;
  unchanged-frame queue bytes fall from 6,330,544 to 39,088, with declared buffer bytes unchanged.
  These are contended upload observations, not integrated frame timings. Exact images use the shared
  main-branch check. [The evidence](../../evidence/REVIEW-ENGINE/M2/README.md) records identities and limits.
  The full workspace gate passes at `77256baef` after the first check's 600-second deadline.

- 2026-11-16 M1 closes the watcher's event-only wait: periodic stat-cache reconciliation recovers silent
  changes without altering the original test's assertions or deadline. On `1d57d92d6`, 12 cooker unit tests
  and four server/watcher tests pass without TMPDIR; ten empty-environment repeats pass too. Restoring the
  old wait makes the new regression fail at generation 1. [The recovery evidence](../../evidence/REVIEW-ENGINE/M1/watcher-recovery/README.md)
  records the confirmed liveness defect; the OS event-delay cause remains unconfirmed. The full gate and
  normal merge follow; M1's five builder passes and `in_progress` status are retained.

- 2026-11-15 On `96f26c347`, the refreshed simulator route, M1.1, both rewind cases, 42 unchanged game
  simulation tests and all 1,966 native/Chrome exact comparisons pass. Verifier time is 947.88 seconds.
  Target proof is retained at `588c2185e`; all four retained simulator images match the inspected originals,
  and the baseline gate accepts the current content key. [The evidence](../../evidence/REVIEW-ENGINE/M1/README.md)
  names source and binary. The full project gate continues before normal merge under the same GPU lease.

- 2026-11-15 The live batch acquires GPU and rebases to `96f26c347`. M1.1, both rewind cases and all 42
  simulation tests pass. The refreshed simulator report passes with ASTC, 29 buffers, pause/resume tick
  1946 and recovery to generation 2. Builder inspection confirms the cooked scene in all four new views.
  The report and images are in `tmp/review-engine/simulator-batch-1794765248104734000` until the batch retains
  them. Exact baselines are running under the same lease; no verifier, full-gate or merge pass is claimed yet.

- 2026-11-15 The continuation rebases cleanly onto CREATOR's main at `bcdc75f4a`. The standard iOS build,
  native verifier build and Web build pass on `48f92ecc4`; these prove compilation only. The queued batch
  must refresh simulator and Mac qualification for the changed sky facade. CREATOR and APPLE-TILE release
  GPU; SIM-2D's workspace gate is active. M1 product code and assertions are unchanged, and the live batch
  remains `20261115-162847-73863`. [The handoff](../../evidence/REVIEW-ENGINE/M1-handoff.md) names the current wait.

- 2026-11-15 The queued batch acquired GPU but failed before proof on an obsolete APPLE-PRESENTATION
  log-link repair during rebase. Both that repair and its reversal resolve to the current upstream log;
  the rebase completes at `fc31fa95b`. Only WATERS' model changed upstream since the qualified source,
  leaving iOS dependencies unchanged. Replacement batch `20261115-162847-73863` is queued for proof and
  normal merge; [the handoff](../../evidence/REVIEW-ENGINE/M1-handoff.md) names its process.

- 2026-11-15 The continuation at `8a92e3275` confirmed the existing device/publication code inspection and
  inspected all four retained simulator views. It claims no new target run. The proof-and-merge batch
  `20261115-144609-30539` remains queued behind active GPU work; the refreshed
  [handoff](../../evidence/REVIEW-ENGINE/M1-handoff.md) names the live process and next completion checks.
  Product code, assertions and M1's five passed builder results are unchanged.

- 2026-11-15 The queued merge-only retry is replaced before it starts: APPLE-TILE's merge batch now holds
  GPU and may change main. The replacement keeps rebase, affected proof, full gate and normal merge under
  one lease, so another stale-proof refusal can be handled without releasing the resource. No running
  proof or other builder's process was stopped. The handoff and build-loop record name the replacement.

- 2026-11-15 The refreshed full gate passes, but its repeat inside merge at `15f0e49c` times out waiting for
  cooker watcher generation 2. The exact failed binary passes all four tests in ten consecutive runs;
  [the diagnostic record](../../evidence/REVIEW-ENGINE/M1/watcher-gate.json) preserves both results without
  claiming a cause. Production code and test assertions/bounds are unchanged for the merge retry. The temporary
  APPLE-PRESENTATION log-link repair is removed now that its evidence is present on main.

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
