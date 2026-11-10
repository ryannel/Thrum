# Thrum build plan

Updated 10 November 2026. Active status page; Git retains history. [Contracts](design/CONTRACTS.md) own invariants;
[maturity](design/maturity.json) separates implementation, correctness, visuals and device results.

## Purpose

Thrum is a native-first production Rust 3D engine and AI engineering showcase: exceptional performance and
professional graphics for teaching demos on the owner's website, then games. Single-player fidelity leads;
shared rules keep multiplayer additive. Native uses each device; Chrome runs portable WebGPU at showcase quality.

## Goal and exit rule

Make one frame fast and beautiful end to end, then widen. **V1 exit:** one reusable engine powers `examples/v1-game`,
a single-player showcase on this M3 Max and its built-in 120 Hz ProMotion display, and the same scene in Chrome.
It has physical sky, Sun and aerial perspective with time controls; cooked meshes/textures; live camera, lights and
cascaded shadows; GPU culling; MetalFX, EDR and a quality governor; display-link interpolation; hot reload, profiler and
native/browser demo UI. Keep 10,000 entities and 1,000–2,000 movers as the representative scene. Headless corrected-run
replay proves multiplayer fitness; transport stays after V1. A fresh agent builds a small scene from public docs alone.
The initial 60 Hz game clock is configurable, not an engine ceiling.

A gate closes when its outcome runs in the game on the named target, correctness/lifecycle and required oracle
checks pass, `./dev check` is green, and independent code/evidence and agent visual review pass.
**Numeric performance/allocation figures are baselines, not gates**, per the
[11:15 owner decision](design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md). Design for performance up front;
investigate visible stutter/hitches, about 25% comparable regression or unexpected scaling. Tune whole frames later.

Mac is a real target. The iPhone 18 Pro simulator proves build/link, lifecycle, input, surface and display-link function,
never speed; fail if it cannot boot. No physical iPhone is available: do not ask or wait. Label Mac stand-ins **proxy**
and never record them as iPhone results. At V1 exit report outstanding physical-device runs. CI remains manual-only.

## Steering and visual review

Owner reports are asynchronous and owner review is one batched window a day. Work runs as described in
[how work runs](design/AI-DEVELOPMENT.md#how-work-runs). Visual milestones are judged in the running game against an
inspection list whose bar is set by shipped high-end games, and close on a clean-context evaluator's pass; acceptance
never blocks. Ask only for purpose/exit changes, spending, publishing/deployment/licensing or irreversible actions;
continue unblocked work. Verified checkpoints are pushed to main (owner, 6 Nov proof of concept).

<a id="active-gate-order-and-budgets"></a>

## Active work and gates

**Road to the exit.** [The module list](design/modules/ARCHITECTURE.md) plans all remaining work as loop modules:
what each owns, the build order, the interfaces between them and a brief for each. Ready to build on 10 November:
`PLATFORM-RUNTIME-MAC` M2, `APPLE-TILE` M0 (the owner unparked it that day), `CREATOR` M1
([G6 close](design/modules/CREATOR-PLAN.md)) and `ATMOSPHERE` M5 ([G3s](design/modules/ATMOSPHERE-PLAN.md)).
Waiting on `APPLE-TILE`: `APPLE-PRESENTATION` M1–M3 ([G4](design/modules/APPLE-PRESENTATION-PLAN.md)). Plans still
owed, none of them startable yet: G4 M4–M11, G3c, the G5 close, G7, and the sea and cloud track. Sea and clouds
are planned as two Thrum modules ([SEA](design/modules/SEA-BRIEF.md), [CLOUDS](design/modules/CLOUDS-BRIEF.md)) that
never gate the exit; when their builders start is the owner's decision. The paragraphs below are each gate's own
status and may trail the loop's record.

Mac M2.1 has fresh current-release runtime motion with 480 exact originals, complete pan/holds and attributed denied
before/after launcher/driver preflights. M2.3 has actual-window pattern/lease/replacement/OFF builder proof on that
same binary. [Current evidence and the one-command runbook](evidence/render/mac-m2-20261110/README.md) retain the
untraced arms. M2.4 trace tooling and M2.2 capture-OFF cadence remain open for attended unlocked active built-in-panel/
Instruments setup. Zero of the two new diagnostic trace attempts were used. Fresh independent swaps and calibrated
full-pan alignment are owed. Engine images and repository checks have their separate proof scopes.

**[T2 unattended capture M0 and M1](design/modules/T2-CAPTURE-PLAN.md) are done.** Metal and Chrome now have
qualified submitted-output and elapsed-time engine capture. [Mac evidence](evidence/render/t2-capture-m0/README.md)
and [Chrome evidence](evidence/render/t2-capture-m1/README.md) include provenance and bounded failure controls.
T2 simulator M2 is blocked on [COOKED-DRAW M1](design/modules/COOKED-DRAW-PLAN.md), owned by G5.
COOKED-DRAW M1 in turn needs [PLATFORM-RUNTIME-IOS M7](design/modules/PLATFORM-RUNTIME-IOS-PLAN.md): G3b owns
matching the scene/internal render size to the current oriented surface. M7 uses the diagnostic scene without a
cooked dependency; after its acceptance G5 reruns cooked rotation and visual proof with M1.10 unchanged.
G3b Mac M2 qualifies the current surface path after T2 M0 only. New Mac M3 needs Mac M2 and G3t M3 for fresh
post-change motion/cadence: T2 M0/M1 (done) → G3t M0–M3, including attended actual-window checks and simulator
diagnostic function, with no simulator cooked/capture dependency. Mac panel cadence needs **attended setup**
(unlocked active panel and any Instruments authorization). Mac M2.3 owns real-window tap qualification, M2.4 bounded
trace-finalization diagnosis; M2.2 qualifies cadence with capture OFF and separate capture-ON observer/content arms.
Engine pixels need no Screen Recording grant; G3t's actual OS-window observer separately requires that grant.

APPLE-TILE remains parked after two failed design reviews. The lead has now commissioned the scoped re-plan
answering the seven findings; fresh independent design approval and unpark through the loop remain owed. T2
completion alone cannot restart it. G3t keeps M0–M3/M10–M12 and removes M4–M9 optional optimization definitions.
G3t has no reverse Mac cadence dependency. G3s sunset crossing stays parked; G2S is closed. The complete native
Metal recorder remains a later performance candidate; the dedicated performance phase owns numeric tuning.

- [x] **G0 — integrity and baselines.** Real Metal regressions fail without a device; no successful skips. One baseline
  command records runnable rows and absent consumers. Dead source/cook/mobile crates are removed and shipping trees
  exclude SQLite/authoring. [Evidence](evidence/v1-workload/G0-INTEGRITY-2026-11-05.md).
- [x] **G2S — simulation correctness and multiplayer fitness.** Typed shared/mutable queries, validated admission,
  trusted in-place rules under sparse undo, engine-owned publication with dirty unions across gaps/rewinds/held pages,
  adjacent previous/current endpoints, retained-input amendment and corrected replay. Require complete authority-state
  equality against the straight corrected run and structural churn/refusal recovery. Presence-aware 10k corrected replay
  and engine-owned append-only edit descriptors pass. Resolve stages privately under its own permits; forced Rayon
  runs the chunked mover kernel and warm page capture concurrently. It matches the straight corrected 100k run's complete
  authority hash and page after churn and kernel/Resolve refusal, and publishes byte-identical pages in every mode. The
  quiet ABBA found no crossover, so requested Rayon is the measured serial fallback, and RayonForced is the tested parallel
  path. Root approved the closeout after an independent review on 7 November.
  [Parallel, Resolve and quiet ABBA evidence](evidence/v1-workload/G2S-PARALLEL-RESOLVE-2026-11-07.md).
  [Recorded quiet S1/S2 figures](evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) are history; old CPU/allocation
  limits are retired. Do not close on a partial hash or an easier row.
- [x] **G3a — executing graph and live scene.** Render/Compute/Copy closures execute in dependency order with culling,
  derived attachment ops, live camera/lights/cascades, reversed-Z `Depth32Float`, indexed draws and submitted transforms.
- [ ] **G3b — functional engine frame runtime.** Display-link targets drive Mac/iOS/browser presentation.
  Mac runs world/render work off its main thread. GPU interpolation, late acquisition and a two-frame limit
  are shared.
  Core owns clock/lane; thrum-render owns `FrameHost`, slots, tokens, completion and history without game schema.
  Metal pause/replacement, Mac overload drain/resume and ten Chrome function rows pass; browser reconstruction retains
  authority/input. Actual Chrome tab transitions pass. The iOS simulator runs world and rendering off UIKit.
  Real touch moves the committed player. Background/foreground and surface replacement recover under the
  epoch-checked lease, including a raced present. [Simulator evidence](evidence/render/G3B-IOS-SIM-ROUTE-2026-11-07.md).
  Simulator rotation preserves the committed player, and an injected device loss recovers in the same process with
  world and controls intact. [Rotation and recovery evidence](evidence/render/G3B-IOS-SIM-ROTATION-LOSS-2026-11-08.md).
  iOS M7 is claimed for review with native oriented scene targets, correct projection and full-surface presentation,
  including recovery refit. [Extent evidence](evidence/render/ios-extent/README.md). COOKED-DRAW M1 retains its M7
  acceptance gate and unchanged M1.9–M1.11 reruns.
  A held-drawable Metal destroy recovers in the same Mac window with world and controls intact.
  [Live Mac evidence](evidence/render/mac-m1-recovery/README.md). Physical iPhone loss and Mac display cadence
  remain open. Mac M2 needs only `T2-CAPTURE:M0` and qualifies the current surface path; its unattended runtime-image
  and attended panel-swap proofs remain distinct and both are required. No M2 tooling item waits on APPLE-TILE.
  New Mac M3 needs `APPLE-TILE:M3` and reruns both protocols plus windowed-tap/tooling preflight on the changed path.
  M3 is blocked on parked APPLE-TILE: delivered T2 Mac M0/Chrome M1 then sequential G3t M0–M3 and their
  attended OS-window proofs. Neither simulator cooked drawing nor T2 simulator M2 gates it; M2 can close first. M2.3 has builder real-window tap proof; M2.4 resumes the failed Instruments
  finalization investigation after design re-review/prerequisites, with a changed hypothesis and at most two new
  diagnostic attempts. M2.2 remains blocked on those proofs and active-panel/tool setup; an unreadable trace cannot
  close it. Its capture-OFF traced arm qualifies cadence; capture-ON arms show content/observer cost. Record
  latency/present intervals and investigate visible hitches. [Mac qualification plan](design/modules/PLATFORM-RUNTIME-MAC-PLAN.md).
  [Frame history](evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md) and wgpu
  allocations are baselines; native recording stays G3t.
- [ ] **G3s — physical sky, Sun and atmosphere.** Hillaire (2020) transmittance, multiple-scattering, sky-view/aerial
  LUTs and Sun disk; physical illuminance drives lights and shadows. Time controls span dawn through night.
  Scaled storage, receiver Sun lookup, twilight interpolation and SH9/GGX have scoped Metal referees. Chrome
  shows twilight. The native cooked valley uses the same Sky, metre-scale camera and cooked Sun casters.
  Eight-wavelength transport yields blue twilight in the valley. Radiance accuracy, shadow polish and live motion remain.
  [Night milestones](design/modules/ATMOSPHERE-PLAN.md) qualified the moonless valley and stars on Metal and Chrome.
  The [west-lookout crossing](design/modules/ATMOSPHERE-PLAN.md#m3-sunset-crossing) is parked at 0/3. The stock graph ships and the frozen motion and controls stay available for diagnosis.
  [Partial evidence](evidence/v1-workload/G3S-FIRST-LIGHT-2026-11-06.md). Follow the Broadsides reading map and PD/CC0 rules.
- [ ] **G3t — tile dataflow and Metal boundary.** One registry, graph-derived attachment storage, one split
  scene/output graph, G4 pre-tone colour/exposure input and a prepared generation-owned native segment/twin.
  Retain existing SDR shaders/intermediate, queue-ordered uploads, 192-byte rows, CPU light overflow and V2 raster/order.
  Defer fused present, pointer rings/slot duplication, scratch sharing, compact rows, expanded GPU binning and
  mirror-range consolidation. No raw/wgpu encoder splice or rival queue; the complete native recorder stays later
  under [the boundary record](design/decisions/G3T-METAL-BOUNDARY.md).
  [Design](design/modules/APPLE-TILE.md), [plan](design/modules/APPLE-TILE-PLAN.md) and
  [criteria](design/modules/APPLE-TILE-DONE.json) answer the [seven second-review findings](evidence/APPLE-TILE/design-review-2.md).
  **Lead recommission received; scoped re-plan awaits fresh independent design approval/unpark through the loop.**
  M0 remains parked; no new implementation or qualification is claimed. Retained IDs are M0–M3/M10–M12.
  T2 M0/M1 are delivered; final M12 cooked simulator function alone needs COOKED-DRAW M1. Preserve G5's registry,
  direct/indirect path and shared draw.rs edits; no classifier/transport refactor is commissioned here.
  D2.5/D3.5/final window proof requires attended unlocked setup and Screen Recording permission for OS window
  capture; engine output alone cannot prove the window. Mac M3 owns fresh post-G3t-M3 motion/capture-OFF cadence.
  Record logical bytes, complete costs and any measured wins honestly; physical replay/device rows remain unqualified
  without their own proof. Follow [T2 capture modes and attended boundaries](design/modules/T2-CAPTURE.md#rollout-and-attended-boundaries).
- [ ] **G3c — GPU-driven culling.** Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per
  mesh/pipeline bucket, stable mesh offsets and compute-readable materials. Validate visibility and image equivalence;
  record full/partial-visibility cost and unexpected growth as tripwires.
- [ ] **G4 — Apple reconstruction and governor.** MetalFX, EDR and binary archives through the native boundary;
  governor reacts to GPU/thermal/present evidence. Compare Apple tile and portable on this Mac with image tolerance and
  counters. No pipeline compilation during play.
  G4 handoffs are blocked on parked APPLE-TILE and delivery of its named milestones. Physical iPhone comparison remains pending.
- [ ] **G5 — content-addressed loop.** Stat cache, BLAKE3 CAS, aligned mmap products, glTF meshlets, KTX2 ASTC/BC7,
  atomic manifest, watcher and dev server. Keep only runtime validation the game needs; shipping never links authoring.
  Prove visible asset edits without GPU stalls; track scan/edit-to-visible latency and startup/hitch tripwires.
  Native and Chrome draw and update cooked assets through the reviewed `SceneContribution` seam. Physical-device runs remain;
  GC/leases and layered EAC stay parked.
  ([design](design/decisions/G5-CONTENT-ADDRESSED-LOOP.md),
  [evidence](evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md)).
- [ ] **G6 — creator experience and demo controls.** Native/browser text, sliders, toggles and field overlays bound to
  typed parameters. One command produces a lit scene, sky, cooked mesh, camera and slider; diagnostics and hot reload
  support asset/shader/parameter edits. A fresh agent uses only public docs/starter, builds a small scene without reading
  internals, and records friction. Keep that scene to about 100 game lines, report game lines and zero engine game
  types. See [creator guide](design/ENGINE-EXPERIENCE.md).
- [ ] **G7 — V1 showcase.** Assemble the exit scene on Mac Apple tile/portable and Chrome portable. The fixed route
  passes correctness and agent visual review; record whole frames and visible stutter/startup/hitches. Native and web
  share scene/controls. Close with visual digest, functional/platform status and device runs owed.

## Working constraints

Choose layouts, ownership, scheduling and hardware seams by best practice; reuse hot storage and one publication
spine/job graph, with scalar references. Preserve admission validation, checked IDs, pipeline closure and history.
Follow [ORCHESTRATE.md](../ORCHESTRATE.md) and `./dev check`. Measure wins; keep
function/visual/device evidence separate, one compact page per gate. Shrink plan/ledger/evidence.

## After V1 and parked work

**Next goal after V1:** [a swarm-survival test game](design/NEXT-GOAL-SWARM-GAME.md), with 2D support and online
multiplayer built up towards it. Owner, 10 November 2026. It is planned as
[track 3 of the module list](design/modules/ARCHITECTURE.md#track-3-the-swarm-survival-game-after-v1): `SIM-2D`
(a crowd that steps identically on every target), `RENDER-2D`, `GAME-UI`, `TRANSPORT`, `SESSION` and `SWARM` (the
game). Each has a brief there, with its interfaces, the build order after the exit and the owner's decisions. None
has a done-when file, so none can start before the exit.

Teaching demos follow, chosen individually: cloud formation, water and others under the [port decision](design/decisions/SEA-AND-CLOUD-PORT.md).
The sea and clouds they build on are planned in [the module list](design/modules/ARCHITECTURE.md#where-sea-and-clouds-are-finished).
Transport reopens after V1 as `TRANSPORT` and `SESSION` in track 3: Quinn and WebTransport, clock sync, prediction
and four rendered peers under conditioning. The compact wire carries the baseline and repair; relevance waits for
a game whose state must travel.
Android/desktop-high recipes and physical iPhone runs follow on suitable hardware. A dedicated AI performance phase
profiles whole frames, derives targets, and revisits retained commands, mapped uploads and complete native recording.

<a id="parked-retained-in-git-history"></a>

Retain parked work in Git: F3 limits/receipts, T2 retained-root proofs, N1 ledger/credit, publication accounting, R0 exact
predicates, R3 optics, moving shore, cohort cloud lab, sea/cloud port pending owner timing and in-memory protocol hardening.
Retained-forward is parked at `a4b9da8`; split pre-submit at `30ae560`; nonlinear M-LUT layout at `11cb21e`
(`codex/g3s-multiple-layout`, mixed oracle accuracy). None is merged; the selected transport remains `e061671`.
