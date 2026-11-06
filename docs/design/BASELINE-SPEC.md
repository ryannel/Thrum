# Baseline decisions and closure register

1 November 2026, re-scoped 5 November. This register chose a concrete first specification for each design gap and
the engine-wide workloads and budgets. **V1 qualifies against the [build plan](../BUILD-PLAN.md#goal-and-exit-rule)
targets, gate budgets and exit, not against the candidates, workloads and envelopes below**; those apply after V1 and
to the designs that cite them. The register distinguishes **selected and specified** from **built and qualified**:
unmeasured hardware support, appearance and timings cannot be closed by writing a specification. The larger engine
can grow beyond this baseline without rewriting its authority contracts.

**Review disposition:** the [simplification review](SIMPLIFICATION-REVIEW.md) prompted the smaller first-game and
portable-transition scope below. G08's protocol and R4 admission belong to the [network specification](NETWORK-SPEC.md),
whose transport track is parked until after V1; G2S proves the C13 authority, rollback and replay contracts headless.
G01/G02 retain the selected identity and authority contracts with the narrower first-run path in
[Foundation](FOUNDATION-SPEC.md). The table records specifications, not built or qualified features.

## Design gaps and their owning specifications

| Gap | Selected outcome | Owning contract |
| --- | --- | --- |
| G01 Source format/admission/recovery | Human-readable JSON working files, reviewed file application followed by explicit immutable exact-byte snapshot admission, content hashes and atomic authority root/outcome | [Foundation specification](FOUNDATION-SPEC.md), C10, F3/F4/T1/P7 |
| G02 World identity/access/transactions | Public epoch/slot/generation identities; private hecs; declared typed stage access; fully staged structural publication | [Foundation specification](FOUNDATION-SPEC.md), C01–C03, F1/W1 |
| G03 Transform numerical semantics | Finite f64 authored envelope, TRS representability/error test against admitted source, explicit f32 consumer envelope | [Foundation specification](FOUNDATION-SPEC.md), C01, W1/P3/V2 |
| G04 Shader/compiler/pipeline convention | WGSL and wgpu/Naga, fixed binding ABI, finite reachable pipeline closure and local preparation | [Rendering specification](RENDER-SPEC.md), C04–C06, F4/V1/V3 |
| G05 Colour/light/media closure | Linear Rec.709/D65 photometric rendering, separate thermal units, jointly lit single-scatter media with explicit approximations | [Rendering specification](RENDER-SPEC.md), C08, V3/E1/V4 |
| G06 Physics/animation/navigation | Narrow selected libraries/owned algorithms and complete authority/pose/query bridges | [Play/environment specification](PLAY-ENVIRONMENT-SPEC.md), C02/C03/C09, P3–P5 |
| G07 Admitted water operators | Prescribed sea and finite shallow-domain operators, explicit reservoir exchange, bounded one-way body interaction and crossing rules | [Play/environment specification](PLAY-ENVIRONMENT-SPEC.md), C09, E1/P3/W2/P7 |
| G08 Multiplayer protocol and limits | Server authority, typed intents, validation at admission and committed outcomes; V1 proves rollback and amended-input replay headless (G2S). Wire and transport follow V1 under the [A0 N1 decision](decisions/A0-N1-TRANSPORT-CODEC.md) | [Network specification](NETWORK-SPEC.md), C13, N1 and all consumers |
| G09 Devices/workloads/budgets | V1 targets in the build plan; post-V1 reference candidates, finite workloads, shared budgets and browser admission below | This document, the single [frame/residency budget table](modules/PLATFORM-RUNTIME.md#whole-scene-budget-proposal--all-unmeasured), and [R4 network fixture](NETWORK-SPEC.md#r4-encoded-admission-fixture) |
| G10 Mobile shell/delivery baseline | winit host adapters with one Android GameActivity entry; iOS native app; bundled initial content; platform release gates | Host selection below, F1/T2 |

The initial scope is deliberately finite. Unsupported conservative falls, arbitrary rollback, arbitrary renderer
replacement and binary hot reload are excluded combinations with diagnostics, not hidden missing implementations of
advertised behavior. The selected operators must meet their declared domains; widening those domains requires a new
complete design. Nothing here makes existing Broadsides implementations automatically conform.

## Reference targets

**V1 proof targets** are this M3 Max with its built-in 120 Hz ProMotion display, which qualifies native recipes and
timing; the iPhone 18 Pro simulator, which proves function only; and Chrome WebGPU on the same Mac, which runs the
portable recipe at showcase quality because website demos run there. The build plan owns their budgets, proxy labels
and device-pending rule. Emulate and simulate first; the owner does no manual device testing.

The rows below are **post-V1 qualification candidates**, run on rented cloud machines or devices when a recipe needs
them; renting is a spending decision for the owner. They are not purchased equipment or certified minimum
requirements. The specified workloads must run on a candidate before support is published there. If a required
operation is unavailable, reject that configuration or revise the target decision; do not silently change game semantics.

| Profile | Post-V1 candidate | Recipe it qualifies |
| --- | --- | --- |
| Older Mac | Apple M2 Mac, 16 GiB memory, macOS 15 or newer | Apple tile and portable recipes below the M3 Max |
| Windows desktop | Windows 11, RTX 3060, 16 GiB system memory; DX12 first | Desktop-high recipe |
| iOS device | iPhone 13 or newer, iOS 18 or newer, native Metal app | Physical timing for the simulator-qualified iOS path |
| Android native | Pixel 7, Android 14 or newer, native Vulkan app | Android tile-GPU recipe |
| Browser | Chrome stable WebGPU on these desktops; Safari and Firefox are further profiles | Portable recipe at showcase quality, with its own budgets and cooked assets |

Each recipe sets its output and frame targets when it is commissioned. The earlier 60 Hz desktop and sustained 30 Hz
mobile/browser rows survive only as floors for the budget shares below, not as goals. Record the exact OS patch,
driver, browser build, adapter features/limits and device SKU in each result; a browser update is a new configuration,
not an invisible moving benchmark. Integrated Windows GPUs, Intel Macs, Linux and additional phones are further
profiles. No minimum OS support is claimed merely because a dependency lists it.

Use queried WebGPU-compatible limits for the portable renderer; reject missing required features before loading the
game. Advanced native techniques are separate recipes, and the browser never limits them. WebGL2 fallback is not part
of this baseline. Desktop authoring starts on Mac; Windows authoring follows the same source/CLI contracts but needs its
own adapter qualification. Mac build hosts produce iOS archives with Xcode; Android uses a pinned Gradle/NDK toolchain.
No claim of universal cross-compilation or store acceptance follows.

## Workloads that constrain the whole engine

V1's workload is the build plan's single-player scene: 10,000 entities with 1,000–2,000 movers, the G2S S1/S2 rows and
the G7 route. The fixtures below were the first planned workloads and remain the post-V1 references. Counts are
authored limits for reproducible fixtures, not capacity claims. All use metres and a 60 Hz authoritative tick,
independent of presentation rate.

| Workload | Concrete content | Status and purpose |
| --- | --- | --- |
| R0 Small game | 64 entities in one preloaded region; one camera, one directional and four local lights, opaque/masked materials, one action/rule/state/result, private save, edit/restart and one export | Superseded as a V1 target by the build-plan scene; its render contract is [RENDER-SPEC section 8](RENDER-SPEC.md#8-r0-first-rendered-game-contract-and-proof) |
| R1 Integrated desktop | Two 512 m regions, 4096 static instances, 32 skinned characters ×64 joints, 128 rigid bodies, 16 nav agents, 128 local lights, one Sun, one fog region, 32 spatial voices and focusable UI | Post-V1 interactions, streaming, reimport and two cameras with valid/reset histories |
| R2 Browser/mobile | R1 semantics with 1024 visible instances, 8 characters, 32 bodies, 8 agents, 32 local lights, one view and 16 voices; reductions only in non-gameplay detail | Post-V1 portable recipe: preloaded closure, restart or visible stop/load for replacement, cold start, offline boot, save and suspend/resume |
| R3 Coastal model | Prescribed sea, one 128×64 shallow grid, ≤16 incident waves, four floating bodies, reservoir exchange and blocked mixed-model hull crossing | Parked with water work until after V1 |
| R4 Multiplayer | Four players, 128 replicated entities/peer, 60 Hz authority, one browser and one native client, one headless authority; join, interest, reconnect, content mismatch and correction | Follows V1 with the network track under its [admission recipe](NETWORK-SPEC.md#r4-encoded-admission-fixture); G2S first proves its authority and replay contracts headless |

A cross-platform multiplayer game uses the same gameplay population and rules on every client; visual culling, LOD
and server query projections reduce client cost, never relevant actors. Geometry fixtures specify 250k visible
triangles in R0, 2M in R1 and 500k in R2, with shadow submissions reported separately. Transparency is capped at two
admitted layers and 25% main-view coverage for these fixtures; that is a content constraint, not permission to
discard a required layer at runtime.

## Budget allocation and failure rules

V1 gates use the build plan's budgets. For post-V1 profiles the platform document remains the sole numeric
whole-frame/residency envelope: desktop CPU 5 ms/GPU 10 ms at 60 Hz, browser CPU 8 ms/GPU 16 ms and mobile CPU
7 ms/GPU 16 ms at 30 Hz; 3 GiB/512 MiB/1 GiB respective memory planning targets, not universal runtime hard limits.
CPU includes simulation ticks due that callback, extraction, UI, audio submission, networking and mandatory platform
work. GPU includes every active view and shadow/media pass. Parallel work is counted in both work and critical-path
reports; CPU+GPU arithmetic is not a proof of frame time.

Assign at most 30% of the CPU critical-path envelope to due simulation ticks, 40% to extraction/render submission,
10% to input/UI/audio submission, 10% to bounded optional prepare and 10% reserve. Network simulation work is inside
the simulation allocation. When a 30 Hz callback owes two ticks, both share its 30%, rather than receiving two complete
frame budgets. A provider unable to meet these allocations rejects that recipe or requires a deliberate rebalance,
not invisible time-step changes. Required source/content/pipeline preparation outside the bound presents loading.

Resident-memory planning shares are world/solver/snapshot state 15%, decoded CPU content 20%, persistent GPU products
35%, transient GPU targets 10%, staging/replacement overlap 10% and reserve 10%. Count shared/unified allocations once,
report estimates separately from process/physical measurements and reserve the aggregate across all worlds/views.
Subsystem budgets may trade within a declared recipe; no two owners can both spend the same reserve. Full off-world
source activation or shader-profile replacement must fit its peak, otherwise perform an explicit stop/load/restart.

For the portable R2 recipe the 512 MiB browser envelope gives 76.8 MiB world, 102.4 MiB decoded content, 179.2 MiB
GPU products, 51.2 MiB transient targets, 51.2 MiB staging and 51.2 MiB reserve; the 1 GiB mobile shares scale alike.
A full world copy exceeds the staging share, so R0 and portable R2 preload their gameplay closure and restart or enter
a visible stop/load before substantive replacement, releasing old leases first; they cannot claim an uninterrupted
simulation. A later live streaming recipe must name its complete old/new, provider, reader and scratch peak, with
shared allocations counted once, and its own transition workload. Client residency or N1 interest changes do not
prove server region streaming.

Cold browser startup targets a ≤25 MiB initial download and ≤5 s to first play at a fixed 50 Mbit/s and 40 ms RTT with
cold caches; 25 MiB alone takes about 4.2 s at that rate, so the build must cut startup bytes or overlap work. Report
transfer and first play separately; a blank responsive shell is not first play. Native local-SSD startup keeps the
≤1 s interactive-shell/≤5 s reference-scene targets. Mobile's initial content slice is ≤40 MiB, distinct from signed
package size; offline boot requires a complete verified installed bundle.

### Browser first-play admission worksheet — estimates, not measurements

The boot closure is the exact code, manifest, gameplay data, products, shaders and pipelines needed before the first
intended input produces visible feedback; a required collider, font, rule or masked shadow role cannot be deferred to
shrink it. R0/R2 planning caps are a compressed payload of 8/16 MiB (code 4/5, products 3/9, shaders and metadata
1/2), ≤16/≤48 distinct device-ready pipeline tuples with no first-draw compilation, and CPU/GPU preparation scratch of
24/40 and 16/32 MiB inside the staging share, serialized so the combined R2 peak stays ≤48 MiB. The first inventory
budgets R0 at 11 boot tuples (2 depth, 2 shadow, 2 forward, exposure resolve, tone, spatial AA, output and optional UI) and R2 at 40.
With three dependency turns (0.12 s) and nonoverlapped critical-path allowances of 1.5 s (R0) and 1.8 s (R2), simple
sums are 2.96 s and 4.60 s against 5 s. These are estimates: a target build reports its actual closure, bytes, cold
pipeline time and browser memory, and [Rendering](RENDER-SPEC.md#3-shader-closure-keys-and-readiness) owns demand and
readiness. Its 512-total/64-boot/128-live-switch controls remain reference-profile gates, not SDK-wide format limits.

V1 gate runs follow the build plan's rows and durations. Post-V1 profile qualification uses a fixed five-minute warmup
and 25-minute run, ten cold launches and separate ten-cycle lifecycle traces. Record p50/p95/p99 intervals, CPU/GPU
tails, peak memory and all unfinished/queued work. Acceptance requires p95 presented interval ≤1.05×frame period, p99
≤1.5×period, no unreported stall >100 ms outside declared loading, and no continuing queue/memory growth. Stable
display cadence alone cannot hide stale simulation or missing visual work. Mobile thermal warnings, background loss
and sustained throttling are recorded with device state; a cool burst does not qualify the profile. Visual/motion
acceptance is [agent visual review](AI-DEVELOPMENT.md#visual-review-and-human-steering), independent of numerical acceptance.

## Concrete host and delivery selection

Select Rust 1.99.1 as the initial compiler candidate, pinned by exact rust-toolchain release when implementation
starts; matching rustfmt/clippy accompany it. A resolved Cargo.lock, dependency checksums/features and native SDK/NDK
versions belong to every reproducible build. A pinned choice is not evidence that the full dependency set compiles.
[The official Rust release](https://blog.rust-lang.org/2026/10/15/Rust-1.99.1/) establishes version availability.

Select winit 0.30.13 for the initial native host boundary, wgpu 30.0.1 for GPU execution and explicit browser JS
integration. Android selects winit's `android-game-activity` route with one Gradle GameActivity shell. Consume
android-activity through winit's matching re-export, avoiding duplicate glue versions and competing entry points.
iOS builds the Rust runtime as a static library in an Xcode app; the main entry starts the winit event loop and creates
UI/surfaces on resume. Any custom application delegate forwards to the same F1 lifecycle owner; it is not a second
simulation loop. UI text/accessibility extensions operate through the platform main-thread bridge. Browser uses the
page's canvas/lifecycle with asynchronous initialization and a single-thread baseline.

Initial native mobile content is bundled in the app, with optional later HTTPS hash-addressed products through F4.
Do not require Apple on-demand-resource APIs or Play Asset Delivery for this first small slice. Larger store-managed
packs can be a later byte-source adapter. Preserve package hash, SDK/toolchain lock, manifest and dependency notices.
Development export is distinct from release signing, privacy declarations and store submission; those policy-specific
checks are performed against the actual release toolchain when a release exists. No external submission is authorized
or performed by writing this design.

V1 verifies iOS boot, tick, surface recreation and input on the iPhone 18 Pro simulator. Physical phones, Android and
full I4 coverage of text composition, accessibility, audio interruptions, thermal behavior and packaging follow on
rented devices after V1. Backend/library incompatibility is diagnosed at these qualification gates, not treated as
evidence that game ownership should change.

Primary boundary evidence: [winit Android](https://docs.rs/winit/0.30.13/winit/platform/android/index.html) documents the
single matching activity-glue requirement and GameActivity option; [winit iOS](https://docs.rs/winit/0.30.13/winit/platform/ios/index.html)
documents native application integration and lifecycle. The selected devices, workloads and numerical budgets are
our engineering targets, not measured results or support claims from those sources.
