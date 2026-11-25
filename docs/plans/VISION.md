# Vision and scope document for Thrum

This is the owner's page: what Thrum is for, the V1 exit and how the owner wants to work. It was folded from
`docs/BUILD-PLAN.md` on 11 November 2026, when the plans took the orchestrate 1.7.0 shape. The build plan's wording is
kept wherever it said what is wanted. Its paragraphs on progress toward each milestone are gone, because
[the status page](STATUS.md) reports that now; `git show 42fdce97:docs/BUILD-PLAN.md` has the last copy.
[The module list](ARCHITECTURE.md) plans the work. [Contracts](../design/CONTRACTS.md) own invariants, and
[maturity](../design/maturity.json) keeps implementation, correctness, visuals and device results apart.

Only the owner changes the purpose or the V1 exit. No word-for-word request from the owner is on file for the
project as a whole, so nothing below is quoted as the owner's unless it names its source.

## The request

<a id="purpose"></a>

> Thrum is a native-first production Rust 3D engine and AI engineering showcase: exceptional performance and
> professional graphics for teaching demos on the owner's website, then games. Single-player fidelity leads;
> shared rules keep multiplayer additive. Native uses each device; Chrome runs portable WebGPU at showcase quality.

That is the build plan's purpose, as it stood on 10 November 2026. Later statements that changed or added to it:

- **6 November 2026.** The owner replaced numeric performance thresholds with three rules: design for speed up front,
  build with tripwires, tune later in a dedicated phase
  ([the decision](../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md)).
- **6 November 2026.** Verified checkpoints are pushed to main, as a proof of concept.
- **10 November 2026.** Sea and clouds are to be finished and consolidated as Thrum modules. The V1 exit does not depend on them
  ([the module list](ARCHITECTURE.md#where-sea-and-clouds-are-finished)).
- **10 November 2026.** The next goal after V1 is [a swarm-survival test game](../design/NEXT-GOAL-SWARM-GAME.md),
  with 2D support and online multiplayer built up towards it. From that page: "A swarm survivor", "2D support in
  the engine" and "Multiplayer. Several players play together over the network."
- **15 November 2026.** On transport's timing, the owner: "it can come before V1 if we have space for it, but it's
  not urgent". Decision 1 read "transport stays after V1" until then. Transport is still no part of the V1 exit;
  [the module list](ARCHITECTURE.md#transports-timing) builds it when a builder slot is free.
- **25 November 2026.** The owner asked for a game SDK: "a high end, intuitive, expressive, readable, SDK that game
  developers can use to build new games. this should be what V1 uses ideally and what we will use to build the swarm
  game. It's VITAL that we get this right as it's the entire interface that developers and coding agents will use to
  build a variety of games. It's the front of the product we're building. So it needs to be as intuitive, well
  documented and easy to use as possible." Their decisions: "V1 first, Rust only, I'll approve the design." The V1
  exit does not wait for the SDK; the showcase moves onto it afterwards, and the swarm game is built on it.

## Why

A reusable engine that shows what AI-driven engineering can do: performance that uses each device's hardware fully,
graphics at the level of shipped high-end games, and a foundation that demos and then games are built on. The
first outlet is teaching demos on the owner's website.

The planner's summary of the build plan's purpose and the project settings' quality bar. Not confirmed by the
owner.

## Who it is for, and what they will be able to do

- **The owner** runs the V1 showcase on this Mac and in Chrome, and publishes teaching demos built on the engine.
- **A creator**, human or agent, starts a small scene from the public docs alone, tunes it with named controls
  and sees where frame time goes.
- **Game makers**, later: the engine holds no game types, and a game links only what it uses.

The planner's reading of the V1 exit and milestone G6. Not confirmed by the owner.

## Decisions

Each is carried from the build plan unless it names another source.

<a id="goal-and-exit-rule"></a>

1. **The goal and the V1 exit.** Make one frame fast and beautiful end to end, then widen. **V1 exit:** one
   reusable engine powers `examples/v1-game`, a single-player showcase on this M3 Max and its built-in 120 Hz
   ProMotion display, and the same scene in Chrome. It has physical sky, Sun and aerial perspective with time
   controls; cooked meshes/textures; live camera, lights and cascaded shadows; GPU culling; MetalFX, EDR and a
   quality governor; display-link interpolation; hot reload, profiler and native/browser demo UI. Keep 10,000
   entities and 1,000–2,000 movers as the representative scene. Headless corrected-run replay proves multiplayer
   fitness; transport is not part of the exit, and "can come before V1 if we have space for it, but it's not
   urgent" (the owner, 15 November 2026). A fresh agent builds a small scene from public docs alone. The initial
   60 Hz game clock is configurable, not an engine ceiling.
2. **When a milestone is reached.** Its outcome runs in the game on the named target, correctness, lifecycle and required
   oracle checks pass, `./dev check` is green, and independent code, evidence and agent visual review pass.
3. **Numeric performance and allocation figures serve as baselines.** The owner, 6 November 2026
   ([the decision](../design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md)). Design for performance up front;
   investigate visible stutter or hitches, about 25% comparable regression or unexpected scaling. Tune whole frames
   later.
4. **Visual work is judged in the running game** against an inspection list whose bar is set by shipped high-end
   games, and closes on a clean-context reviewer's pass. Acceptance never blocks other work.
5. **The order of milestones within the V1 exit is the agents' call.** From the project settings.
6. **Sea and clouds are finished as Thrum modules and are not part of the V1 exit.** The module list's decision,
   10 November 2026, from the owner's request that both be finished and consolidated. Adding them to the exit is
   the owner's call.
7. **After V1 comes the swarm-survival game**, with 2D and online multiplayer. The owner, 10 November 2026
   ([the page](../design/NEXT-GOAL-SWARM-GAME.md)); planned as
   [track 3 of the module list](ARCHITECTURE.md#track-3-the-swarm-survival-game-after-v1).
8. **Working constraints.** Choose layouts, ownership, scheduling and hardware seams by best practice; reuse hot
   storage and one publication spine and job graph, with scalar references. Preserve admission validation,
   checked IDs, pipeline closure and history. Measure wins; keep function, visual and device evidence separate,
   one compact page per milestone. Shrink plan, ledger and evidence.

<a id="active-gate-order-and-budgets"></a>

## The milestones

The V1 exit is the release milestone reached through the milestones below. G0, G2S and G3a were reached
before the build began. For the others, [the status page](STATUS.md) reports progress in the modules named.

- **Integrity and baselines** (`milestone/integrity-and-baselines`, was G0). Real Metal regressions fail without a device; no successful skips. One baseline
  command records runnable rows and absent consumers. Shipping trees exclude SQLite and authoring.
  [Evidence](../../.orchestrate/evidence/v1-workload/G0-INTEGRITY-2026-11-05.md).
- **Simulation correctness** (`milestone/simulation-correctness`, was G2S). Typed shared and mutable queries, validated admission,
  trusted in-place rules under sparse undo, engine-owned publication, retained-input amendment and corrected
  replay, with complete authority-state equality against the straight corrected run. Never closed on a partial
  hash or an easier row. [Evidence](../../.orchestrate/evidence/v1-workload/G2S-PARALLEL-RESOLVE-2026-11-07.md);
  [recorded figures](../../.orchestrate/evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) are history.
- **Live scene** (`milestone/live-scene`, was G3a). Render, compute and copy closures execute in dependency order with
  culling, derived attachment ops, live camera, lights and cascades, reversed-Z `Depth32Float`, indexed draws and
  submitted transforms.
- **Frame runtime** (`milestone/frame-runtime`, was G3b). Display-link targets drive Mac, iOS and browser presentation; Mac runs
  world and render work off its main thread; GPU interpolation, late acquisition and a two-frame limit are shared.
  Modules: `PLATFORM-RUNTIME-MAC`, `PLATFORM-RUNTIME-IOS`, and the web shell.
  [Frame history](../../.orchestrate/evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md) and wgpu allocations are baselines.
- **Sky and atmosphere** (`milestone/sky-and-atmosphere`, was G3s). Hillaire (2020) transmittance, multiple scattering, sky-view and aerial
  LUTs and Sun disk; physical illuminance drives lights and shadows; time controls span dawn through night. Follow
  the Broadsides reading map and the PD and CC0 rules. Module: `ATMOSPHERE`.
  The TAAU route proof on Metal and Chrome records sunrise shadows, the lookout depth edge and the aerial glow at
  sunset. The west-lookout crossing and exact disk match remain open. [WP6 builder evidence](../../.orchestrate/evidence/ATMOSPHERE/WP6/README.md)
- **Tile dataflow** (`milestone/tile-dataflow`, was G3t). One registry, graph-derived attachment storage, one split scene and
  output graph, G4's pre-tone colour and exposure input, and a prepared generation-owned native segment with its
  twin, under [the boundary record](../design/decisions/G3T-METAL-BOUNDARY.md). No raw or wgpu encoder splice and
  no rival queue; the complete native recorder stays later. Module: `APPLE-TILE`.
- **GPU culling** (`milestone/gpu-culling`, was G3c). Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per
  mesh and pipeline bucket, stable mesh offsets and compute-readable materials. Validate visibility and image
  equivalence; record full and partial-visibility cost and unexpected growth as tripwires. Module: `CULLING`.
- **Apple reconstruction** (`milestone/apple-reconstruction`, was G4). MetalFX, EDR and binary archives through the native boundary; the
  governor reacts to GPU, thermal and present evidence. Compare Apple tile and portable on this Mac with image
  tolerance and counters. No pipeline compilation during play. Module: `APPLE-PRESENTATION`.
- **Content-addressed loop** (`milestone/content-addressed-loop`, was G5). Stat cache, BLAKE3 CAS, aligned mmap products, glTF meshlets, KTX2 ASTC and BC7,
  atomic manifest, watcher and dev server. Keep only the runtime validation the game needs; shipping never links
  authoring. Prove visible asset edits without GPU stalls. Modules: `COOKED-DRAW`, `CONTENT-LOOP`
  ([design](../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md),
  [evidence](../../.orchestrate/evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md)).
- **Creator experience** (`milestone/creator-experience`, was G6). Native and browser text, sliders, toggles and field overlays bound
  to typed parameters. One command produces a lit scene, sky, cooked mesh, camera and slider; diagnostics and hot
  reload support asset, shader and parameter edits. A fresh agent uses only public docs and the starter, builds a
  small scene without reading internals, and records friction. Keep that scene to about 100 game lines, report
  game lines and zero engine game types. See [the creator guide](../design/ENGINE-EXPERIENCE.md). Module: `CREATOR`.
  [WP1](../../.orchestrate/evidence/CREATOR/WP1.md) proves the shared Mac/Chrome scene route and live assets;
  [WP2](../../.orchestrate/evidence/CREATOR/WP2.md) and its [independent review](../../.orchestrate/evidence/CREATOR/WP2-review.md)
  prove the profiler. [WP3](../../.orchestrate/evidence/CREATOR/WP3.md) records the lead-commissioned fresh scene on Mac and
  Chrome, the documentation check, native mistakes and the attended window journey.
  The clipping and shadow-cascade overlays and parameter files as a cooked product remain unbuilt and outside
  the scope of G6.
- **V1 showcase** (`milestone/v1-showcase`, was G7). Assemble the exit scene on Mac Apple tile and portable and on Chrome portable. The fixed
  route passes correctness and agent visual review; record whole frames and visible stutter, startup and hitches.
  Native and web share scene and controls. Close with the visual digest, functional and platform status and the
  device runs owed. Module: `SHOWCASE`.

Engine capture for all of them is `T2-CAPTURE`.

## Not part of this

- **Before the exit:** transport and sockets, which the exit does not need and which may be built before it when
  there is space (the owner, 15 November 2026); sea and clouds; physical iPhone qualification; Android and
  desktop-high recipes; numeric performance tuning, which belongs to a dedicated AI performance phase that profiles
  whole frames, derives targets, and revisits retained commands, mapped uploads and complete native recording.
- **After the game:** teaching demos, chosen one at a time: cloud formation, water and others under
  [the port decision](../design/decisions/SEA-AND-CLOUD-PORT.md).

<a id="parked-retained-in-git-history"></a>

Parked work is retained in Git: F3 limits and receipts, T2 retained-root proofs, N1 ledger and credit, publication
accounting, R0 exact predicates, R3 optics, moving shore, cohort cloud lab, and in-memory protocol hardening.
Retained-forward is parked at `a4b9da8`; split pre-submit at `30ae560`; nonlinear M-LUT layout at `11cb21e`
(`codex/g3s-multiple-layout`, mixed oracle accuracy). None is merged; the selected transport remains `e061671`.

## Must not change

- The purpose and the V1 exit, except by the owner.
- [The contracts](../design/CONTRACTS.md), except with their producers and consumers.
- Broadsides: it is read, never modified, and nothing is imported from it wholesale.
- No GPL or share-alike code or data.

## How it will be judged

Decision 2 defines when a milestone is reached. Visual work is judged as decision 4 says: in the running game, in words, against
an inspection list, never against target images. The bar is a shipped high-end game of the kind, such as sea and
wakes as in Sea of Thieves and skies as in Red Dead Redemption 2, smooth at the display's full refresh. At the V1
exit the outstanding physical-device runs are reported. The owner's own judgements are batched: owner review is
one window a day, and reports to the owner are asynchronous.

## Where it runs

- **The Mac is a real target:** this M3 Max, Metal, its built-in 120 Hz ProMotion display.
- **Chrome** runs the portable WebGPU recipe at showcase quality.
- **The iPhone 18 Pro simulator** proves build and link, lifecycle, input, surface and display-link function, never
  speed; a run fails if it cannot boot. No physical iPhone is available: do not ask or wait. Label Mac stand-ins
  **proxy** and never record them as iPhone results.
- CI remains manual-only, so `./dev check` must pass before a merge.

## How the owner wants to work

Work runs as [how work runs](../design/AI-DEVELOPMENT.md#how-work-runs) and [the project settings](../../ORCHESTRATE.md)
describe: one Codex builder unless the owner says otherwise, reviews on the other host, and verified checkpoints
pushed to main. Agents keep building and review visual output themselves. Ask the owner only for changes to the
purpose or the exit, spending, publishing, deploying, licensing or irreversible actions, and continue unblocked
work meanwhile.

## Still open

- **When sea and cloud builders start.** Not decided yet. Until the owner says, `SEA` and `CLOUDS` are planned no
  further than their module briefs.
- **The licence for code and data lifted from Broadsides.** Not decided yet; nothing is lifted until it is.
- **The purpose line still says "3D engine"**, and the 10 November goal adds 2D. Rewording it is the owner's.
- **What finishes the goal after V1.** Not decided yet; the module list holds a proposal.
- The rest of the owner's open decisions are listed in [the module list](ARCHITECTURE.md#owner-decisions).
