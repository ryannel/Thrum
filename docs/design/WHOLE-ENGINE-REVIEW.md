# Whole-engine independent review

**Later scope qualification, 1 November 2026:** the [decision audit](ARCHITECTURE-AUDIT.md) found open boundaries and
insufficiently compared choices beyond this review's strongest coverage. The original derivation/findings below are
preserved. Its no-blocker conclusion is historical for the inspected scenarios, not current architecture-wide closure.

## Review basis and independent derivation

This section was recorded before inspecting the engine-rebuild design package. The review brief states a custom engine with no Bevy runtime host, reusable across 3D overhead, isometric, third-person and first-person games; Mac, PC, browser and native mobile store delivery; configurable capabilities, including cloud shadows when visible clouds are disabled; high performance, modern scalable visual quality and an AI-oriented development workflow. Pure 2D and MMO scale are outside scope. The target of this pass is an architecture and an ordered detailed-design/construction programme, not runtime implementation.

### Capability implications

| Requirement | Necessary architectural consequence | Unresolved input to preserve |
|---|---|---|
| Multiple 3D camera idioms | World, visibility, LOD, input targeting, audio listener, streaming and temporal history cannot assume one viewpoint or a single active camera. | Multi-camera, split-screen and VR support are not stated. |
| Overhead through first person | Near-field material/animation/occlusion and distant world scale have different error tolerance; one content format needs declared fidelity and quality fallbacks. | Specific genre, world size, crowd count and character fidelity are unknown. |
| Browser plus native desktop/mobile | Explicit platform abstraction for surfaces, input, file/storage, audio lifecycle, packaging and graphics-device capabilities; no platform can be silently treated as an export flag. | Exact OS/API minima, browser support and store rules are unknown. |
| Configurable phenomena | Capabilities have typed dependencies, lifetimes and resource/compute cost. A producer of atmospheric shadowing can remain active when a visible-cloud renderer is absent. | Which capabilities are essential per shipping profile is not specified. |
| High performance and scalable graphics | Whole-frame CPU/GPU/memory/content budgets, quality tiers, async work, visibility/streaming and degraded paths must be designed together. | Target hardware, resolution, frame rate, thermal envelope and download limits are unknown. |
| AI-oriented creation | Stable data contracts, deterministic/reproducible builds, inspectable diagnostics, validation and agent-safe change boundaries. | Whether AI must run in-game is not stated; do not infer it. |
| Custom engine | Own the runtime scheduling, world authority, render orchestration, content lifecycle and platform adapters. Reuse of libraries or tools remains possible without making Bevy the runtime host. | Host language and graphics API are not fixed by this brief. |

### Cross-system scenarios that must work

1. **Camera transition:** overhead to first person in a streamed world. Input aim, visibility, animation, sound listener, LOD, temporal render history and near-field assets change coherently without changing simulation authority or revealing unloaded content.
2. **Cloud-shadow-only profile:** time, atmosphere/lighting and shadow field advance while visible cloud geometry and its render cost are absent. Terrain, characters, water and exposure consume the same field; toggling visible clouds does not reset weather state.
3. **Native mobile interruption:** suspend during streaming and gameplay; device or surface may be lost, audio interrupted and storage writes unfinished. Resume yields a consistent world, resources and timeline.
4. **Browser device loss or graphics tier change:** transient GPU resources and temporal histories rebuild from authoritative simulation/content state; reduced quality respects memory and frame budget without deleting gameplay state.
5. **Moving character crossing a streamed boundary:** animation, root motion, collision/physics, navigation, camera collision, audio occlusion and persistence agree on transforms and origin/coordinate changes.
6. **Developer/content iteration:** an asset or rule revision enters through validated manifests and explicit migration/versioning; multiple agents can produce content without conflicting IDs or silent runtime schema drift.
7. **Frame overload:** simulation time, render interpolation, input latency, background jobs, streaming uploads and audio timing have an explicit priority and degradation policy; scaling graphic quality cannot create divergent simulation.

### Review tests derived before inspecting the package

- Every shared quantity has one authority, with clear derived/read-only consumers and update order: world transforms, simulation clock, camera and listener poses, animation/root motion, environment state, visibility, asset residency and persistent state.
- Module boundaries describe producers, consumers, data precision, update cadence, lifetime, failure/rebuild behaviour and transactional transitions where one capability changes several systems.
- The architecture chooses foundational representations through domain and mathematical reasoning; experiments test predicted bounds or uncertainties. Unmeasured quality and performance remain honest qualifications.
- Quality profiles compose supported capability sets and provider choices; unsupported combinations are explicit. No profile obtains a second world state by accident.
- The design programme orders shared contracts before module implementation and includes cross-system integration proofs with rejection conditions.

## Findings

### W1 — Committed world snapshot readership has no lifetime rule

**Original severity:** high for cross-system integration. **Status:** resolved at high-level contract scope after coordinator revision; implementation remains unbuilt. **Affected:** [C02](CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world), [C05](CONTRACTS.md#c05-assets-streaming-and-retirement--f4-with-w2-v1-and-p7), [C07](CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4), [platform/runtime](modules/PLATFORM-RUNTIME.md), [world/content](modules/WORLD-CONTENT.md), [rendering](modules/RENDERING.md), and D1/D2 in [construction plan](CONSTRUCTION-PACKAGES.md).

C02 promises immutable committed state and interpolation between adjacent snapshots; the renderer calls it an immutable `WorldSnapshot`, two views use it, and the host may have two frames in flight. W2 concurrently replaces and retires region state. The documents define F4 product leases and GPU fences but not who owns a snapshot read lease, how W1 prevents reuse/mutation of world/solver/animation stores while extraction reads them, when each view releases its read, or how many committed epochs may remain live. Merely tagging a result with an epoch detects staleness after the read; it does not make a racing read safe.

**Counterexample:** the first view starts extracting tick `n` while the host admits a region replacement and tick `n+1`. W1 reuses a table slot or P3/P4 changes a specialised store. A second view then extracts `n` using a mixture of old transform and new material/collider-derived bounds, or the first view reads a retired region. F4/GPU leases protect product bytes and submitted buffers, but they do not pin W1's committed values or the producer state needed for presentation-time animation. On a single-threaded host this can be avoided by a strict extraction barrier; the design currently does not require that either.

**Correction condition:** C02/C05/C07 should select a publication/read rule: for example, synchronous extraction from a stable committed pair before the next mutation, or versioned immutable snapshots with bounded reader leases/epochs. Name which owner pins W1, P3/P4/E1 and region-derived inputs through both views' extraction, the release point, the memory charge/retirement rule, and what a delayed/skipped view does when its required source epoch is no longer retained. The exact copy-on-write layout is detailed design, but the mutual-exclusion or lifetime invariant is a shared architectural contract. A two-view extract concurrent with tick, region replacement and asset retirement is the rejection scenario.

**Closure check:** C02 now assigns F1 a bounded count/byte publication pool and W1 an immutable read-model bundle whose pages cannot alias mutable solver/world arrays. Registered readers take `SnapshotLease` and relevant CPU product/provider leases. V2 copies into an owned `RenderPacket` before releasing the world lease; GPU fences govern packet/resource retirement separately. C05 forbids reclaiming on cancellation until the reader releases its lease. C07 bounds historical deformation/effect retention and resets a skipped view outside the valid horizon. Optional unsubmitted views may be cancelled/deferred; a required reader backpressures publication or enters the explicit overload state. C10 gives save serialization its own bounded immutable durable-state capture, avoiding a mistaken assumption that a render snapshot is a complete save. The platform, world and rendering module notes and integration review I11 repeat the same ownership. These rules prevent the counterexample without preselecting a page layout or pool size. D1 must still specify the exact storage/lease API and a concurrency stress must test it; neither is high-level selection work.

### Items that remain properly open

The documents mark minimum devices and browser coverage, source/runtime formats, physics/audio/text library audits, exact material and environment closures, numeric whole-scene costs, running visual quality and store packaging as unselected or unqualified. These need the D0–D2 detailed commissions and later measurements; they are not evidence that a duplicate authority or incompatible module boundary has already been selected. The recommended Rust/wgpu baseline likewise remains a recommendation with explicit platform gates, while the custom host and no-Bevy-runtime constraint are user decisions. No arbitrary live feature combination is promised.

## Scope and conclusion

I reviewed the brief, integrated system design, C01–C12, design programme, construction plan, maturity model and ledger, all seven module-family designs, integration dispositions and the content/temporal research reviews. I used the independent derivation above to challenge the normal tick, two cameras, moving character/streaming boundary, hidden-cloud shadow profile, edit/save/recovery, mobile interruption, GPU loss and quality-switch paths. This is a design-document review; I did not inspect or run the target runtime, test a device, or judge rendered visual quality. Primary-source claims were used at the scope stated in the source ledgers; this review's finding follows from the design's own concurrent-reader obligations and requires no external empirical claim.

The high-level package is **coherent enough to commission detailed modules**. No unresolved high-level architectural blocker remained after W1's contract revision. Its strongest evidenced choices are one body/world authority, required-product pre-tick admission, separate content/CPU/GPU graphs, environmental effects independent of visible presentation, source-to-save recovery ancestry, bounded immutable snapshot readership, and per-view temporal sample identity. Those choices are reasoned at an architectural level and do not certify the proposed renderer's quality, cross-platform performance, or implementation. The machine ledger should mark this document-level integration review as reconciled for the reviewed revisions while retaining not-run implementation, quality and platform evidence.

**Reviewed technical revision scope:** SHA-256 prefixes on 1 November 2026: `CONTRACTS.md` `920a00b5d6db`; `modules/PLATFORM-RUNTIME.md` `d0d80e788cf6`; `modules/WORLD-CONTENT.md` `9f388e1f2ff8`; `modules/RENDERING.md` `23eb7fceface`; `INTEGRATION-REVIEW.md` `e7924e0395f7`. A changed owner/lifetime assumption requires renewed review of the affected path; prose/navigation edits outside this scope do not imply runtime evidence.

### Narrow environment delta check, 1 November 2026

I rechecked the later C08/C09 and environment source/tick revision against C02/C03 and the P3/P4/P6 play schedule, without repeating the full survey. The new rule has W1 own authored light identity/descriptors, E1 own celestial/environmental drivers and headless irradiance/thermal provision, and V3 own rendered transport. This preserves one source of physical light/heat facts while allowing a renderer to approximate image transport; GPU pixels cannot become the thermal authority. Surface state `X[n]` supplies contact/traction during the physical solve. E1 integrates tick-`n` forcing and publishes `X[n+1]` at the world commit; identified contact deposition enters the next surface update once. That order neither gives P3 a partly updated surface nor creates a same-step P3↔E1 callback loop. The explicitly declared one-tick lag and stronger coupled-solver option agree with C03's causal boundary. Named receiving regions or exported sinks close runoff bookkeeping at the interface. No new high-level conflict was found. Detailed response coefficients, exposure/thermal approximation, runoff transfer and acceptable lag remain D2/module design and later qualification.

**Delta revision scope:** SHA-256 prefixes: `CONTRACTS.md` `7c9503bbc567`; `modules/ENVIRONMENT.md` `19d6e802c436`; `modules/PLAY-SYSTEMS.md` `e814dfd6ba4e` (schedule checked, unchanged by this delta); `INTEGRATION-REVIEW.md` `e1cb8c645ed4`. These supersede the earlier contract/integration hash prefixes above for the final reviewed state; the platform/world/render snapshot hashes remain as recorded.
