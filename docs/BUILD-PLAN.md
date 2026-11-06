# Thrum build plan

Updated 6 November 2026. Active status page; Git retains history. [Contracts](design/CONTRACTS.md) own invariants;
[maturity](design/maturity.json) separates implementation, correctness, visuals and device results.

## Purpose

Thrum is a native-first production Rust 3D engine and AI engineering showcase: exceptional performance and
professional graphics, first for high-end teaching demos on the owner's website, then games. Single-player fidelity
leads; shared authority/rules keep multiplayer additive. Native uses each device; Chrome runs portable WebGPU at
showcase quality.

## Goal and exit rule

Make one frame fast and beautiful end to end, then widen. **V1 exit:** one reusable engine powers `examples/v1-game`,
a single-player showcase on this M3 Max and its built-in 120 Hz ProMotion display, and the same scene in Chrome.
It has physical sky, Sun and aerial perspective with time controls; cooked meshes/textures; live camera, lights and
cascaded shadows; GPU culling; MetalFX, EDR and a quality governor; display-link interpolation; hot reload, profiler and
native/browser demo UI. Keep 10,000 entities and 1,000–2,000 movers as the representative scene. Headless corrected-run
replay proves multiplayer fitness; transport stays after V1. A fresh agent builds a small scene from public docs alone.
The initial 60 Hz game clock is configurable, not an engine ceiling.

A gate closes when its functional outcome runs in the game through public engine APIs, correctness and lifecycle
checks pass, `./dev check` is green, required oracle tolerance and agent visual review pass, and an independent reviewer
has tried to break the code and evidence. **Numeric performance and allocation figures are recorded baselines, not
admission gates**, following the [11:15 owner decision](design/decisions/PERFORMANCE-PHILOSOPHY-2026-11-06.md).
Design performance foundations up front; investigate visible stutter/hitches, about 25% regression from a comparable
baseline, or unexpected scaling. Tune later from whole-frame profiles and set targets then.

Mac is a real target. The iPhone 18 Pro simulator proves build/link, lifecycle, input, surface and display-link function,
never speed; fail if it cannot boot. No physical iPhone is available: do not ask or wait. Label Mac stand-ins **proxy**
and never record them as iPhone results. At V1 exit report outstanding physical-device runs. CI remains manual-only.

## Steering and visual review

Keep building; owner reports/digests are asynchronous. Visible changes need fixed cameras, a sequence, image checks
and independent agent review. Fix or record open look questions with a recommendation; acceptance never blocks.
Ask only for purpose/exit/order changes, spending, publishing/deployment/licensing or irreversible actions; continue
unblocked work. Local verified commits are useful; no push without authorization.

<a id="active-gate-order-and-budgets"></a>

## Active work and gates

**Now: finish G3s sky, Sun, sunset and twilight.** Functional G3b and G2S correctness can proceed on disjoint owners.
The native Metal recorder stays in G3t. The later dedicated performance phase owns numeric tuning.

- [x] **G0 — integrity and baselines.** Real Metal regressions fail without a device; no successful skips. One baseline
  command records runnable rows and absent consumers. Dead source/cook/mobile crates are removed and shipping trees
  exclude SQLite/authoring. [Evidence](evidence/v1-workload/G0-INTEGRITY-2026-11-05.md).
- [ ] **G2S — simulation correctness and multiplayer fitness.** Typed shared/mutable queries, validated admission,
  trusted in-place rules under sparse undo, engine-owned publication with dirty unions across gaps/rewinds/held pages,
  adjacent previous/current endpoints, retained-input amendment and corrected replay. Require complete authority-state
  equality against the straight corrected run and structural churn/refusal recovery. The complete presence-aware 10k corrected-run proof now passes. Remaining:
  staged resolve/edit/permit ownership. Requested Rayon currently runs serial game systems and proves no parallel fitness.
  [Recorded quiet S1/S2 figures](evidence/v1-workload/G2R-CONSOLIDATED-2026-11-05.md) are history; old CPU/allocation
  limits are retired. Do not close on a partial hash or an easier row.
- [x] **G3a — executing graph and live scene.** Render/Compute/Copy closures execute in dependency order with culling,
  derived attachment ops, live camera/lights/cascades, reversed-Z `Depth32Float`, indexed draws and submitted transforms.
- [ ] **G3b — functional engine frame runtime.** Display-link targets drive Mac/iOS/browser presentation; world/render
  work is off Apple's main thread, at most two frames are in flight, interpolation is on GPU and acquisition is late.
  Core owns clock/lane; thrum-render owns `FrameHost`, slots, tokens, completion and history without game schema.
  Metal pause/replacement and ten Chrome function rows pass. Browser loss/retry reconstructs graphics with authority/input retained; actual tabs, live Mac P/loss and simulator lifecycle remain. Instrument latency/present intervals;
  visible stutter, dropped frames and hitches trigger investigation. [Quiet frame figures and later comparisons](evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md)
  remain baselines; wgpu frame allocation is not a gate or reason to bring native work forward.
- [ ] **G3s — physical sky, Sun and atmosphere.** Hillaire (2020) transmittance, multiple-scattering, sky-view and aerial
  LUTs; Sun disk; physical illuminance drives lights and shadows. Time controls run dawn through sunset to night.
  Scaled half storage, metering and smooth SDR highlights pass Metal checks; Chrome time/camera controls show twilight.
  SH9 now lights faces by normal; the direct cosine referee bounds its approximation. Remaining:
  multiple-scattering/spectral error, declared radiance tolerance, motion and contrast. Review cameras/sweep for
  colour, banding and flicker. [First-light evidence](evidence/v1-workload/G3S-FIRST-LIGHT-2026-11-06.md) is partial;
  record complete atmosphere cost when available, without a numeric admission target. Follow the Broadsides reading
  map and public-domain/CC0 provenance rules; no code/assets or GPL-derived values are imported.
- [ ] **G3t — tile dataflow and Metal boundary.** Direct forward output, derived memoryless/transient attachments,
  mapped upload ring, compact instances, GPU light binning, mirrored ranges, shared targets and one scene/present graph.
  One registry-selected boundary supports native archives, MetalFX and EDR with portable fallback. The complete Metal
  recorder stays a later performance candidate in the [boundary record](design/decisions/G3T-METAL-BOUNDARY.md).
  Record attachment traffic, full GPU work, upload behavior and light/mirror scaling; investigate regressions, not old
  numeric targets. No partial raw/wgpu encoder splice or rival queue.
- [ ] **G3c — GPU-driven culling.** Compute frustum and Hi-Z, GPU-written indirect counts, one indexed indirect draw per
  mesh/pipeline bucket, stable mesh offsets and compute-readable materials. Validate visibility and image equivalence;
  record full/partial-visibility cost and unexpected growth as tripwires.
- [ ] **G4 — Apple reconstruction and governor.** MetalFX, EDR and binary archives through the native boundary;
  governor reacts to GPU/thermal/present evidence. Compare Apple tile and portable on this Mac with image tolerance and
  counters. Record the measured difference without a required percentage win. No pipeline compilation during play.
  Physical iPhone comparison remains pending.
- [ ] **G5 — content-addressed loop.** Stat cache, BLAKE3 CAS, aligned mmap products, glTF meshlets, KTX2 ASTC/BC7,
  atomic manifest, watcher and dev server. Keep only runtime validation the game needs; shipping never links authoring.
  Prove visible asset edits without GPU stalls; track scan/edit-to-visible latency and startup/hitch tripwires.
- [ ] **G6 — creator experience and demo controls.** Native/browser text, sliders, toggles and field overlays bound to
  typed parameters. One command produces a lit scene, sky, cooked mesh, camera and slider; diagnostics and hot reload
  support asset/shader/parameter edits. A fresh agent uses only public docs/starter, builds a small scene without reading
  internals, and records friction. Keep that scene to about 100 game lines, report game lines and zero engine game
  types. See [creator guide](design/ENGINE-EXPERIENCE.md). UI cost is recorded, not an admission target.
- [ ] **G7 — V1 showcase.** Assemble the exit scene on Mac Apple tile/portable and Chrome portable. The fixed route
  passes correctness and agent visual review; record whole frames and visible stutter/startup/hitches. Native and web
  share scene/controls; performance comparisons remain historical until the dedicated push. Close with visual digest,
  functional/platform status and physical-device runs owed.

## Working constraints

Choose layout, ownership, scheduling and hardware seams by best practice. Reuse hot storage; keep one publication
spine/job graph and scalar kernel references. Measure claimed wins; building does not wait for benchmarks.
Correctness/oracle/visual checks remain blocking. Preserve admission validation, checked IDs, pipeline closure and
history. Use [engine-contributor](../.agents/skills/engine-contributor/SKILL.md), scoped checks and `./dev check`.
Record function/visual/device evidence separately; shrink existing plan/ledger/evidence. One compact evidence page
per gate; the dated performance decision is the authorized new record.

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
