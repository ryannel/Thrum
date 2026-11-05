# Thrum build plan

Updated 5 November 2026. This is the active status page. Git retains earlier plans. [Contracts](design/CONTRACTS.md) own shared invariants; [maturity](design/maturity.json) records implementation, correctness, visuals and device results separately.

## Goal and exit rule

Make one frame fast end to end, then widen. One reusable engine powers `examples/v1-game` with 10,000 entities and 1,000–2,000 movers on an Apple Silicon Mac and a physical iPhone; Chrome WebGPU runs the portable smoke tier. The game uses cooked meshes/textures, live camera/lights/shadows, GPU culling, reconstruction, EDR, display-link interpolation, hot reload, saves and a Tracy-backed profiler. Four rendered peers then run over Quinn through 100 ms RTT, 20 ms jitter and 2% loss at no more than 64 kbit/s per client. The fixed 60 Hz game tick is not an engine ceiling.

**A gate passes only when its numeric budgets pass** on the named target, the same game runs through public engine APIs, `./dev check` is green, and defect-first review finds no blocker. Record p50/p95 and the hot-path profile. V1 exits on [BASELINE-SPEC](design/BASELINE-SPEC.md) p95/p99 over 600 frames and ten minutes, zero pipeline hitches after first launch, and Apple tile at least 10% faster than portable on the same iPhone within image tolerance. Stop with numbers if a budget proves unreachable. CI remains manual-only by owner decision.

## Fix first — regression test for each

- [x] Dispatch the newest committed page whenever the native extract worker becomes idle; do not silently miss tick N+1 while a view is ready.
- [x] Never freeze a moving instance outside the prepared shadow envelope; make shadows degrade without dropping its transform.
- [x] A busy publication page drops a presentation frame; it does not end or stall authority.
- [x] Remove the SQLite authoring link from the iOS game by moving product types into a small runtime content crate.
- [x] Use reversed-Z `Depth32Float` throughout the scene and shadows.
- [x] Preserve canonical published order and byte-identical published game state across restore; publish sorted dirty slots.

## Active gate order and budgets

- [x] **G0/G1 — boundary and publication foundation.** Prior correctness work remains; its speed is requalified in G2R. [G0](evidence/integration/G0-BOUNDARY-2026-11-04.md), [G1](evidence/integration/G1-PUBLICATION-2026-11-04.md).
- [x] **G2R — simulation speed and replay.** Owned columns, checked handles, typed batches, sparse undo and changed-page journals run in-game. **Mac budgets:** tick+page p95 ≤25/300 µs and eight-tick rewind+eligible page p95 ≤100 µs/1 ms at 10k/100k, zero warmed allocations. M3 Max release p95 before→after: recorded tick+page 61/514→21/213 µs; static churn tick 35/257→24.9/216 µs in a 10k/2.5k-sample run; rewind 81/696→73/703 µs. Four-peer serial/Rayon replay, stale-page catch-up, held controls and zero-allocation churn pass. Held pages defer views; ticks continue. [Storage decision](design/decisions/G2-STORAGE-SELECTION.md).
- [ ] **G3a — executing graph and live scene.** Portable graph: typed resource pool, closure recording, topology/culling, derived load/store and timestamps; delete hand encoding. Run camera, clusters and cascades per frame; use dynamic view offsets, back-face culling, indexed draws, compact instances and GPU-retained previous transforms. **Budgets:** camera/light/shadow changes without re-preparation; 10k CPU encode ≤0.3 ms; named GPU passes visible in-game. M3 Max 10k p95 CPU upload+encode through submit 348→280 µs; indexed cubes reduced GPU pass sum 0.381→0.293–0.299 ms in two quiet runs, with device-clock variation. Topology and draw compaction remain.
- [ ] **G3b — frame loop, then engine host.** Display-link `advance(target_time) → extract → encode_scene → late present(drawable)` with interpolation, at most two frames in flight, off-main simulation at user-interactive QoS and one overload/occlusion/suspend policy. Move `FrameHost` into the engine only after this redesign. **Budgets:** 120 Hz ProMotion present-interval p99 within 1 ms of display period, measured input-to-photon latency, zero steady main-thread allocations on submit and dispatch.
- [ ] **G3c — GPU-driven culling.** Compute frustum and Hi-Z, one `draw_indexed_indirect` per mesh/pipeline bucket with GPU-written counts, stable mesh offsets and compute-readable material table. **Budget:** full-visibility 10k GPU time no worse than without culling, with time falling with visibility.
- [ ] **G4 — Apple tile and reconstruction.** First decide the Metal escape-hatch boundary. Direct forward tone/output when no post pass; otherwise transient intermediates. MetalFX to native resolution, `Rgba16Float` EDR/CAMetalLayer, MTLBinaryArchive, own portable TAAU and a GPU/thermal/present quality governor. **Budget:** V1 Apple tile beats portable by ≥10% GPU time or bandwidth on the same iPhone, with image in tolerance; use GPU timestamps, not wait-inclusive wall benches.
- [ ] **G5 — replace content authoring stack.** Replace `thrum-source`/`thrum-cook` with a small runtime product crate and about 3k lines of stat cache, BLAKE3 file/action CAS, aligned mmap products, glTF meshlets, KTX2 ASTC/BC7, atomic manifest, watcher and dev server. Retain only needed path/JSON/record validation; supersede the SQLite design mandates. **Budgets:** no-change project scan ≤50 ms; one-texture edit visible ≤500 ms without GPU stall; shipping targets link no authoring crates.
- [ ] **G6 — redesign wire, then QUIC.** ≤12-byte packet header with piggybacked ACK bits, relevance/priority under a token bucket, dead-reckoning residuals, repaired unreliable membership, compact input batches, clock sync, remote interpolation and owned-entity prediction. Preserve strict codecs/fuzzing; then add Quinn GSO/GRO and sessions under one link trait/conditioner. **Budgets:** ≤64 kbit/s per client including overhead at 1.5k movers under the conditioner; iPhone reconciliation ≤0.5 ms/snapshot.
- [ ] **G7 — four rendered peers.** Qualify macOS and physical iPhone Apple tile plus portable, and Chrome portable smoke, over 600 frames and ten minutes against V1 exits.

## Working constraints

Profile before optimising. Each slice reports a before/after hot-path number and net lines per crate; a slower budgeted path stays off main. Zero steady tick/frame allocations are tested. One publication spine, one job graph with serial and core-class-aware execution, and SIMD chosen from the capability registry with a scalar reference. No recipe lands before portable runs through the graph. The game owns schema, rules, scene and assets; engine crates never import game types. Report game lines and engine-side game-type references at each gate. No new fixture binary.

Do not grow design, maturity, evidence or provenance files until V1; correct or shrink them. Evidence is at most one page per gate. Keep the fixed-step clock, checked identities, validate-then-apply authority, pipeline closure, temporal history and strict codecs. Use [engine-contributor](../.agents/skills/engine-contributor/SKILL.md), `./dev check` and a read-only review-agent pass at each gate. Local commits are checkpoints; push only with exact authorization.

## Parked, retained in Git history

Park A5 lane hardening until G6. F3 limits/receipts, T2 retained-root proofs, N1 ledger extensions, publication-pool accounting, R0 exact predicates, R3 optics, moving-shore research and environment/water/cloud implementation stay parked. [Hardware-native](design/decisions/HARDWARE-NATIVE-ENGINE.md) and [sea/cloud port](design/decisions/SEA-AND-CLOUD-PORT.md) govern post-V1 work.
