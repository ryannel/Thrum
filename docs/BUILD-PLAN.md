# Thrum build plan

Updated 6 November 2026. Active status page; Git retains history. [Contracts](design/CONTRACTS.md) own invariants;
[maturity](design/maturity.json) separates implementation, correctness, visuals and device results.

## Purpose

Thrum is a native-first, production-grade Rust 3D engine for learning, craft and a showcase of AI-driven engineering.
Its first outlet is high-end interactive teaching demos on the owner's website, then games later. Aim for exceptional
performance and graphics that nobody recognises as homemade. Single-player fidelity leads; shared authority and rules
keep multiplayer additive. Native uses each device's hardware; Chrome runs a strong portable WebGPU recipe.

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

Build continuously; reports and owner digests are asynchronous. Visible changes need fixed cameras, a short sequence,
automated image checks, root inspection and independent agent review. Fix artifacts or record remaining look questions
with a recommendation; owner acceptance is steering, never a gate. Stop to ask only for purpose/exit/order changes,
spending, publishing/deployment/licensing or irreversible actions; continue unblocked work meanwhile. No push without
explicit authorization. Keep local verified checkpoints.

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
  Finish surface/background/device-loss recovery and simulator function. Instrument input-to-photon/present intervals;
  visible stutter, dropped frames and hitches trigger investigation. [Quiet frame figures and later comparisons](evidence/render/V1-BOUNDED-FRAMES-2026-11-04.md)
  remain baselines; wgpu frame allocation is not a gate or reason to bring native work forward.
- [ ] **G3s — physical sky, Sun and atmosphere.** Hillaire (2020) transmittance, multiple-scattering, sky-view and aerial
  LUTs; Sun disk; physical illuminance drives lights and shadows. Time controls run dawn through sunset to night.
  Half-float encoding and physical incident camera metering pass scoped Metal checks; twilight is now visible.
  Finish the SDR shoulder, independent CPU spectral reference and declared radiance tolerance. Review cameras/sweep for
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

Choose data layout, ownership, scheduling and hardware seams by best practice before entrenching APIs. Reuse hot-path
storage and keep one publication spine, one job graph and scalar references for specialized kernels. Performance claims
need measurements, but building does not wait for benchmarks. Correctness, oracle tolerance and visual review remain
blocking for their changes. Keep validate-at-admission, checked identities, pipeline closure and temporal history.
Use [engine-contributor](../.agents/skills/engine-contributor/SKILL.md), scoped checks and `./dev check`; each capability
records functional, visual and device evidence separately. Evidence stays one compact page per gate. Shrink existing
plan/ledger/evidence rather than growing process; the dated owner performance decision is the authorized new record.

## After V1 and parked work

Teaching demos follow, chosen individually: cloud formation, water and others under the [port decision](design/decisions/SEA-AND-CLOUD-PORT.md).
Transport reopens after V1: compact wire, Quinn, clock sync, relevance/prediction and four rendered peers under conditioning.
Android/desktop-high recipes and physical iPhone runs follow on suitable hardware. A dedicated AI performance phase
profiles whole frames, derives targets, and revisits retained commands, mapped uploads and complete native recording.

<a id="parked-retained-in-git-history"></a>

Retain parked work in Git: F3 limits/receipts, T2 retained-root proofs, N1 ledger/credit, publication accounting, R0 exact
predicates, R3 optics, moving shore, cohort cloud lab, water/cloud implementation and in-memory protocol hardening.
Retained-forward optimization is parked at `a4b9da8`; portable split pre-submit at `30ae560`. Neither is merged.
