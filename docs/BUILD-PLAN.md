# Thrum build plan

Updated 4 November 2026. This is the current status page. Git history retains prior detailed plans and experiments. [C4 contracts](design/CONTRACTS.md) own shared invariants; [the maturity ledger](design/maturity.json) separates implementation, correctness, visuals and device measurements.

## Goal

Thrum is a modern engine that is exceptionally fast because it uses the hardware each device actually has. A capability registry selects a frame recipe per GPU family (Apple tile, Android tile, desktop high, portable), accelerated CPU, IO and network paths, and optional providers (reconstruction, ray tracing, frame interpolation, ML) through one frame contract. A quality governor holds frame, thermal and latency budgets. Portable runs everywhere as the reference and fallback; dedicated paths are promoted only by a measured win on their own hardware. [Hardware-native decision](design/decisions/HARDWARE-NATIVE-ENGINE.md).

## V1 milestone

One reusable engine powers one test game (`examples/v1-game`) on macOS and a physical iPhone, qualified against their budgets. Chrome WebGPU runs the same game at a declared reduced tier for a smoke check, with no V1 frame budget. The game has 10,000 entities with 1,000–2,000 moving; registered game components and systems; serial and parallel execution; snapshot restore; cooked meshes and textures; dynamic camera, lights, clustered forward and cascaded shadows; GPU-driven culling with indirect draws; the Apple tile recipe with memoryless attachments and MetalFX reconstruction to native output resolution; EDR output; hot reload; a named CPU/GPU profiler; display-link interpolation and a quality governor. Then four peers run **the same game** over real QUIC transport on macOS and iPhone through a 100 ms RTT, 20 ms jitter, 2% loss conditioner, with prediction and reconciliation at no more than 64 kbit/s per client including transport overhead. The first game uses 60 fixed ticks/s; this is not an engine ceiling. WebTransport follows behind the same link trait after V1.

A gate passes when it runs in this game on a named target, `./dev check` is green, review finds no blocker, and the [ledger](design/maturity.json) records evidence. Repository checks prove no visual, device or performance claim. CI is manual-only by owner decision.

## Active gate order

- [x] **G0 — cleanup and boundary.** Retired the I1/R4 stacks and phone receipts, consolidated IDs, parked `thrum-jobs` and frozen source hardening/A5 lanes. The game is the external consumer; 41 evidence archives left the current tree. [Record](evidence/integration/G0-BOUNDARY-2026-11-04.md).
- [x] **G1 — identity and publication.** Checked dense slots publish complete immutable pages with dirty sets and ordered structural changes. The V1 game renders them; the optional projection outbox reports gaps. Warmed steady ticks allocated nothing in tested paths; Metal and window diagnostics ran, while V1 budgets remain open. [Evidence](evidence/integration/G1-PUBLICATION-2026-11-04.md).
- [x] **G2 — generic simulation.** The running game owns schema, rules and pages over registered hecs storage. Atomic typed edits, bound callbacks, serial/Rayon execution, engine change stamps, bounded rolling history and replay after churn run in the one game. Review found no blocker and `./dev check` passed. [Design](design/decisions/G2-GENERIC-SIMULATION.md), [evidence](evidence/v1-workload/G2-GAME-RUNTIME-2026-11-04.md).
- [ ] **G3 — executing renderer.** Reviewed [graph design](design/decisions/G3-EXECUTING-RENDER-GRAPH.md): closures, transient/memoryless derivation, automatic timestamps, compute and time-sliced passes, 3D/cube/array resources, GPU culling/indirect draws, persistent pipelines, ray-tracing-ready mesh/material ABI, and future sea/cloud consumers. Move `FrameHost` into the engine, run cooked game content, and implement the immutable capability registry, portable and Apple tile recipes, complete C07 view contract and shading-library decision. Measure both recipes on the same iPhone.
- [ ] **G4 — presentation clock.** Drive presentation from the display link; interpolate local and remote entities; fold tone mapping into present with EDR/HDR output; reconstruct to native output resolution with MetalFX on Apple and own TAAU on portable; close the quality governor over GPU time, thermal state and missed presents. Measure input-to-photon latency and qualify visible game pacing on a named target.
- [ ] **G5 — content loop.** File content-addressed store with atomic manifest and stat cache; glTF meshlets via meshoptimizer and KTX2 (ASTC on iPhone, BC7 on desktop); BLAKE3 hashing and memory-mapped zero-copy products; watcher and barrier publication; dev cook server for browser and phone. Run edit-to-visible and save/restore through the game.
- [ ] **G6 — sessions and real transport.** `ServerSession` and `ClientSession` behind a link trait, one conditioner; Quinn on macOS and iPhone with GSO/GRO offload; QoS-classed network and simulation threads. WebTransport follows later as a web-tier adapter. Compact header with piggybacked ACKs, residual encoding, priority, byte budget and non-aborting recovery. Resume ACK/rebase/retry/reconnect/lane hardening here, over real transport.
- [ ] **G7 — four rendered peers.** Run and measure the same game on macOS and a physical iPhone under the specified conditioner, plus a Chrome reduced-tier smoke run. Record tick, encode, named GPU pass, bandwidth, frame pacing, latency, allocations, retained bytes, thermal state after ten minutes and visible quality separately, for the Apple tile recipe and for portable on the same device.

Each pass declares requirements; the registry closes a recipe and its portable fallback before drawing. Stay on wgpu with isolated native escape hatches. A direct native backend needs evidence that the escape hatch dominates. Chrome uses portable without limiting native recipes. [Hardware policy](design/decisions/HARDWARE-NATIVE-ENGINE.md).

The game consumes public engine APIs; engine crates never import it. Each gate reports game lines and engine-side game-type references (target zero). No new fixture binary lands. Dependencies need reuse and architecture records; the game gets unused-dependency and Wasm checks.

## Current baseline and limits

- The fixed-step clock, checked identity/handle semantics, validate-then-apply edits, pipeline closure and temporal-history contract are retained.
- G1's incremental 10k tick plus page was 76.16 µs median; four pages reserved 10.0 MB, projection 4.1 MB. Two Metal window runs showed tick CPU p50/p95 104.625/193.167 and 103.666/129.833 µs. [Evidence](evidence/integration/G1-PUBLICATION-2026-11-04.md).
- The matched hecs/custom SoA motion kernels were 2.42–2.50/1.65 µs at 10k and 26.74–27.62/19.08–19.24 µs at 100k. Current **game-owned** tick plus page medians are 138.1 µs/1.394 ms serial and 324.2 µs/1.656 ms Rayon at 10k/100k. Tick plus rolling delta is 192.3 µs/2.060 ms. The earlier direct-write serial medians were 76.87/774.8 µs; checked edits and stamps add cost. [Comparison](evidence/v1-workload/G2-STORAGE-COMPARISON-2026-11-04.md), [qualification](evidence/v1-workload/G2-GAME-RUNTIME-2026-11-04.md).
- Chrome WebGPU and macOS render diagnostics draw 10k synthetic cubes; the graph is declarative and cooked content remains test-only. No real transport or physical-iPhone game qualification exists. [Chrome evidence](evidence/render/V1-CHROME-WEBGPU-2026-11-04.md).

## After V1: hardware track

Each item lands as an optional provider or recipe, promoted by measurement on named devices: Android tile (Adreno, Mali, ADPF); desktop high (NVIDIA, AMD, Intel) with visibility buffer, mesh shaders, async compute and Reflex/Anti-Lag; DLSS, FSR and XeSS reconstruction and frame generation; hardware ray query for shadows, AO and reflections with a radiance cache and denoising; VRS and rasterisation rate maps; tensor/neural-accelerator inference; MTLIO and DirectStorage readers.

## Parked, retained in Git history

- A5 in-memory ACK, rebase, retry, reconnect and lane hardening until G6 real transport.
- F3 space/limit/receipt diagnostics; T2 retained-root proofs; N1 credit and ledger extensions; publication-pool accounting; R0 exact-arithmetic predicates; R3 optics; moving-shore research; cloud cohort lab.
- The legacy I1 protected host, R4 chain and phone example, and the unused `thrum-jobs` package. The old implementation and the 41 evidence archives remain recoverable from Git history; the [archive index](evidence/ARCHIVE-ACCESS.md) gives exact paths and hashes. Parked means retained as reference, not active construction.
- Environment, cloud and water implementation until after G7. The [sea and cloud port decision](design/decisions/SEA-AND-CLOUD-PORT.md) selects a CPU f64 spectral sea with a GPU FFT/wake/foam residual, weather-map volumetric clouds over the E1 `τ` source, Hillaire sky with froxels and CPU shallow-water authority inland. The Broadsides Rust sea and clouds port module by module after G7.

## Decision and review gates

Use [engine-contributor](../.agents/skills/engine-contributor/SKILL.md) for every code change and [engine-architecture](../.agents/skills/engine-architecture/SKILL.md) for contract changes. Authority and saves refuse atomically; wire data is dropped/counted/repaired without aborting; presentation clamps and counts; authored content validates with tolerances. Every gate gets a read-only defect-first review, including cross-module identity, time, coordinate and engine/game seams. Keep evidence to one page per slice and bulk outputs out of Git. Commit only after `./dev check` is green; publish only with exact-push authorization.
