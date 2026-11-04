# Thrum build plan

Updated 4 November 2026. This is the current status page. Git history retains prior detailed plans and experiments. [C4 contracts](design/CONTRACTS.md) own shared invariants; [the maturity ledger](design/maturity.json) separates implementation, correctness, visuals and device measurements.

## V1 milestone

One reusable engine powers one test game (`examples/v1-game`) on macOS, Chrome WebGPU and a physical iPhone. The game has 10,000 entities with 1,000–2,000 moving; registered game components and systems; serial and parallel execution; snapshot restore; cooked meshes and textures; dynamic camera, lights, clustered forward and cascaded shadows; hot reload; a named CPU/GPU profiler; native resolution and display-link interpolation. Then four peers run **the same game** over real transport through a 100 ms RTT, 20 ms jitter, 2% loss conditioner, with prediction and reconciliation at no more than 64 kbit/s per client including transport overhead. The first game uses 60 fixed ticks/s; this is not an engine ceiling.

A gate passes only when the capability runs in this game on a named target, `./dev check` is green, the independent review finds no blocking defect, and the [ledger](design/maturity.json) records the actual evidence. Repository checks alone establish no visual, device or performance claim. `./dev check` is the landing gate; CI remains manual-only by the owner's 4 November decision.

## Active gate order

- [x] **G0 — cleanup and boundary.** Retired the inactive I1 world/host/replica and R4 chain, legacy mobile receipts and I1 phone game; consolidated IDs; parked `thrum-jobs`; removed unused direct dependencies; replaced numeric stable-ID GPU indexing with a one-time lookup. This plan is a short status page, and 41 bulk evidence archives left the current Git tree. Source-store hardening and A5 in-memory protocol work are frozen. The external-consumer layout is established at `examples/v1-game`; semantic game-schema separation remains for G2. [G0 record](evidence/integration/G0-BOUNDARY-2026-11-04.md).
- [ ] **G1 — one identity and publication spine.** The world owns dense generation-checked slots. Each tick publishes one immutable page plus per-component dirty sets. Render extraction, replication and the snapshot ring consume that page. Demonstrate the path in the game with no steady-state tick allocations, including actions.
- [ ] **G2 — generic simulation.** Game-owned components, systems, rules, inputs and wire schema; engine-owned registered storage and enforced accesses. Compare hecs `PreparedQuery` in storage order with an owned archetype SoA store at 10k and 100k, with movers and statics separate, before selecting the backend. Add serial and Rayon executors with equal-result test, restore/replay for prediction, and move fixture content from engine crates into the game. Ask for a decision only if measurements do not settle storage.
- [ ] **G3 — executing renderer.** Before implementation, review a design of at most two pages covering record closures, transient pool, derived load/store, culling, automatic timestamps, compute passes, 3D/cube/array textures, epoch-skipped and time-sliced passes. Implement mesh arena, material table, texture arrays and per-frame `ViewPacket`; move `FrameHost` to an engine crate. Run cooked content in the game. The graph design gets an independent cross-module review.
- [ ] **G4 — presentation clock.** Drive presentation from the display link; interpolate local and remote entities; use Depth32Float, native resolution and tone mapping folded into present. Qualify visible game pacing on a named target.
- [ ] **G5 — content loop.** File content-addressed store with atomic manifest and stat cache; glTF meshlets and KTX2 (ASTC on iPhone, BC7 on desktop); watcher and barrier publication; dev cook server for browser and phone. Run edit-to-visible and save/restore through the game.
- [ ] **G6 — sessions and real transport.** `ServerSession` and `ClientSession` behind a link trait, one conditioner; Quinn then WebTransport, with iOS fallback selected by measurement. Compact header with piggybacked ACKs, residual encoding, priority, byte budget and non-aborting recovery. Resume ACK/rebase/retry/reconnect/lane hardening here, over real transport.
- [ ] **G7 — four rendered peers.** Run and measure the same game on macOS, Chrome WebGPU and a physical iPhone under the specified conditioner. Record tick, encode, named GPU pass, bandwidth, frame pacing, allocations, retained bytes and visible quality separately.

The game is an external engine consumer. It uses only public engine APIs; engine crates never depend on it and engine tests/benches use minimal private fixtures. Each gate reports game line count and engine-side references to game types, with a target of zero. No new standalone fixture binary lands on main. New dependencies require a reuse record and architecture registration. The game gets an unused-dependency and Wasm compile check.

## Current baseline and limits

- The fixed-step clock, checked identity/handle semantics, validate-then-apply edits, pipeline closure and temporal-history contract are retained.
- A private hecs backend and serial A1 simulation ran a 10k/1.5k local fixture. On Apple M3 Max the selected hecs tick median was 51.83 µs; an earlier provisional SoA fixture measured 20.83 µs. These are not a G2 storage decision: the 100k split workload and whole-game path remain unmeasured. [Evidence](evidence/v1-workload/A1-HECS-INTEGRATION-2026-11-04.md).
- The V1 diagnostic rendered 10k synthetic cubes with local lights and two synthetic cascades. Chrome WebGPU and macOS offscreen/window diagnostics exist; the scene still has no running cooked mesh/texture loop, and the graph is declarative. [Chrome evidence](evidence/render/V1-CHROME-WEBGPU-2026-11-04.md); 4 November independent review (`tmp/code-review-2026-11-04/THRUM-REVIEW.md`).
- Historical conditioned four-peer headless and test-only cook paths never fed the displayed game. The conditioned program is now retired from the active tree; cooked content remains a test-only path. No real transport or physical-iPhone game qualification exists.

## Parked, retained in Git history

- A5 in-memory ACK, rebase, retry, reconnect and lane hardening until G6 real transport.
- F3 space/limit/receipt diagnostics; T2 retained-root proofs; N1 credit and ledger extensions; publication-pool accounting; R0 exact-arithmetic predicates; R3 optics; moving-shore research; cloud cohort lab.
- The legacy I1 protected host, R4 chain and phone example, and the unused `thrum-jobs` package. The old implementation and the 41 evidence archives remain recoverable from Git history; the [archive index](evidence/ARCHIVE-ACCESS.md) gives exact paths and hashes. Parked means retained as reference, not active construction.
- Environment, cloud and water implementation until after G7. The design direction is Hillaire sky with froxels, weather-map volumetric clouds, FFT ocean with clipmap LOD and CPU shallow-water authority inland. Design correction is pending.

## Decision and review gates

Use [engine-contributor](../.agents/skills/engine-contributor/SKILL.md) for every code change and [engine-architecture](../.agents/skills/engine-architecture/SKILL.md) for contract changes. Authority and saves refuse atomically; wire data is dropped/counted/repaired without aborting; presentation clamps and counts; authored content validates with tolerances. Every gate gets a read-only defect-first review, including cross-module identity, time, coordinate and engine/game seams. Keep evidence to one page per slice and bulk outputs out of Git. Commit only after `./dev check` is green; publish only with exact-push authorization.
