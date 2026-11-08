# Thrum build plan

Updated 7 November 2026. Active status page; Git retains history. [Contracts](design/CONTRACTS.md) own invariants;
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

**Now: qualify the G3b Mac frame runtime.** The G3s sunset crossing is parked for fresh-context diagnosis; G2S is closed.
The native Metal recorder stays in G3t. The later dedicated performance phase owns numeric tuning.

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
  A held-drawable Metal destroy recovers in the same Mac window with world and controls intact.
  [Live Mac evidence](evidence/render/mac-m1-recovery/README.md). Physical iPhone loss and Mac display cadence
  remain open. Record latency/present intervals and investigate visible hitches. [Mac qualification plan](design/modules/PLATFORM-RUNTIME-MAC-PLAN.md).
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
- [ ] **G3t — tile dataflow and Metal boundary.** Direct forward output, derived memoryless/transient attachments,
  mapped upload ring, compact instances, GPU light binning, mirrored ranges, shared targets and one scene/present graph.
  One registry-selected boundary supports native archives, MetalFX and EDR with portable fallback. The complete Metal
  recorder stays a later performance candidate in the [boundary record](design/decisions/G3T-METAL-BOUNDARY.md).
  Record attachment traffic, full GPU work, uploads and light/mirror scaling. No raw/wgpu encoder splice or rival queue.
- [ ] **G3c — GPU-driven culling.** Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per
  mesh/pipeline bucket, stable mesh offsets and compute-readable materials. Validate visibility and image equivalence;
  record full/partial-visibility cost and unexpected growth as tripwires.
- [ ] **G4 — Apple reconstruction and governor.** MetalFX, EDR and binary archives through the native boundary;
  governor reacts to GPU/thermal/present evidence. Compare Apple tile and portable on this Mac with image tolerance and
  counters. No pipeline compilation during play.
  Physical iPhone comparison remains pending.
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

Teaching demos follow, chosen individually: cloud formation, water and others under the [port decision](design/decisions/SEA-AND-CLOUD-PORT.md).
Transport reopens after V1: compact wire, Quinn, clock sync, relevance/prediction and four rendered peers under conditioning.
Android/desktop-high recipes and physical iPhone runs follow on suitable hardware. A dedicated AI performance phase
profiles whole frames, derives targets, and revisits retained commands, mapped uploads and complete native recording.

<a id="parked-retained-in-git-history"></a>

Retain parked work in Git: F3 limits/receipts, T2 retained-root proofs, N1 ledger/credit, publication accounting, R0 exact
predicates, R3 optics, moving shore, cohort cloud lab, sea/cloud port pending owner timing and in-memory protocol hardening.
Retained-forward is parked at `a4b9da8`; split pre-submit at `30ae560`; nonlinear M-LUT layout at `11cb21e`
(`codex/g3s-multiple-layout`, mixed oracle accuracy). None is merged; the selected transport remains `e061671`.
