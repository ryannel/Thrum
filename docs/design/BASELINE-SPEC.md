# Baseline decisions and closure register

1 November 2026. The user asked to itemise and resolve the remaining design gaps, including the multiplayer rework.
This document chooses a concrete first specification and orders its implementation. It distinguishes **selected and
specified** from **built and qualified**. Unmeasured hardware support, appearance and timings cannot be closed by
writing a specification. The larger engine can grow beyond this baseline without rewriting its authority contracts.

**Current review disposition:** the [fresh simplification review](SIMPLIFICATION-REVIEW.md) prompted the smaller
first-game and portable-transition scope below. G08's protocol and R4 admission recipe are owned by the
[network specification](NETWORK-SPEC.md); its exact readiness follows that specification. G01/G02 retain the
selected identity and authority contracts with the narrower first-run path in [Foundation](FOUNDATION-SPEC.md).
The table records specifications, not built or qualified features.

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
| G08 Multiplayer protocol and limits | Four-player initial profile, server authority, explicit binary WSS protocol, coherent baselines, optional complete kinematic prediction | [Network specification](NETWORK-SPEC.md), C13, N1 and all consumers |
| G09 Devices/workloads/budgets | Named reference candidates, finite workloads, shared budgets and analytical browser/transition admission below; R4 encoded recipe in Network | This document, the single [frame/residency budget table](modules/PLATFORM-RUNTIME.md#whole-scene-budget-proposal--all-unmeasured), and [R4 network fixture](NETWORK-SPEC.md#r4-encoded-admission-fixture) |
| G10 Mobile shell/delivery baseline | winit host adapters with one Android GameActivity entry; iOS native app; bundled initial content; platform release gates | Host selection below, F1/T2 |

The initial scope is deliberately finite. Unsupported conservative falls, arbitrary rollback, arbitrary renderer
replacement and binary hot reload are excluded combinations with diagnostics, not hidden missing implementations of
advertised behavior. The selected operators must meet their declared domains; widening those domains requires a new
complete design. Nothing here makes existing Broadsides implementations automatically conform.

## Reference targets

These are **chosen qualification candidates**, not discovered available hardware, purchased equipment or certified
minimum requirements. The specified workloads must run on them before publishing support. If a required operation
is unavailable, reject that configuration or revise the explicit target decision; do not silently change game semantics.

| Profile | Candidate and initial environment | Output/renderer target |
| --- | --- | --- |
| Mac desktop | Apple M2 Mac, 16 GiB memory, macOS 15 or newer; Metal through wgpu | 1920×1080 internal/output, 60 Hz |
| Windows desktop | Windows 11, RTX 3060, 16 GiB system memory; DX12 first | 1920×1080, 60 Hz |
| Browser demo | The M2 reference Mac, Chrome stable qualification snapshot, WebGPU, no cross-origin-isolated threading requirement | 1280×720 internal, 30 Hz floor; scale to CSS viewport with explicit DPR/output cap |
| iOS native | iPhone 13, iOS 18 or newer, native Metal app | 1280×720-equivalent internal pixel budget, orientation-aware output, sustained 30 Hz |
| Android native | Pixel 7, Android 14 or newer, native Vulkan app | Same internal pixel budget and sustained 30 Hz |

Record the exact OS patch, driver, browser build, adapter features/limits and device SKU in each result. “Stable” is
resolved once into the qualification artifact; a later browser update is a new configuration, not an invisible moving
benchmark. Safari/Firefox, integrated Windows GPUs, Intel Macs, Linux and additional phones are additional qualification
profiles, not implied by these candidates. No minimum OS support is claimed merely because a dependency lists it.

Use queried WebGPU-compatible limits for the portable renderer; reject missing required features before loading the
game. Advanced native techniques are separate recipes. WebGL2 fallback is not part of this baseline. Desktop authoring
starts on Mac; Windows authoring follows the same source/CLI contracts but needs its own adapter qualification. Mac
build hosts produce iOS archives with Xcode; Android uses a pinned Gradle/NDK toolchain. Browser and desktop artifacts
use their declared target toolchains. No claim of universal cross-compilation or store acceptance follows.

## Workloads that constrain the whole engine

Counts below are authored workload limits for reproducible fixtures, not capacity claims or algorithm benchmarks.
All use metres and a 60 Hz authoritative tick, independent of 30/60 Hz presentation. Default lighting/material settings
come from RENDER-SPEC; error/physics limits come from FOUNDATION-SPEC and PLAY-ENVIRONMENT-SPEC.

| Workload | Concrete content and transitions | Purpose |
| --- | --- | --- |
| R0 Small game | 64 entities in one bounded preloaded gameplay region; one orthographic or perspective camera, one directional and four local lights, opaque/masked materials, one action/rule/state/result, private save, source edit/restart and one development export | I3a first useful game from ordinary working files through one immutable admitted snapshot and pinned local authority; omit animation, water, media, nav, transport and audio cleanly when unused. An in-memory authority/replica fixture exercises the same action and state contracts early. |
| R1 Integrated desktop | Two 512 m regions, 4096 static instances, 32 skinned characters ×64 joints, 128 active rigid bodies, 16 nav agents, 128 local lights, one Sun, one bounded fog region, 32 spatial voices and focusable text/UI | I3b interactions, streaming, reimport, source/save, two cameras; secondary view half width/height every other frame with valid/reset histories |
| R2 Browser/mobile | Same authored semantics with 1024 visible static instances, 8 skinned characters, 32 bodies, 8 nav agents, 32 local lights, one view and 16 voices; reductions only in authored non-gameplay population/visual detail | First portable recipe preloads a bounded gameplay closure and uses restart or visible stop/load for substantive replacement; cold-start, offline boot, player save and suspend/resume are required. Live authoritative region streaming is a separate wider recipe with its own peak transfer proof. |
| R3 Coastal model | A prescribed sea, one 128×64 shallow grid, at most 16 incident waves and four floating bodies; reservoir water exchange plus visibly blocked mixed-model hull crossing; fog overlap and cloud-attenuation-only view | C09 flux/query/save and C08 transport integration; optional workload, not R0's prerequisite |
| R4 Multiplayer | Four players, 128 replicated entities/peer, 60 Hz authority/20 Hz publication, one browser client, one native client, one headless authority; join/interest/reconnect/content mismatch/correction | N1 finite preloaded server baseline and whole-game latency/memory/event correctness, independently of visual tier. The [encoded fixture, full-group choice and playable/degraded network cases](NETWORK-SPEC.md#r4-encoded-admission-fixture) are its admission recipe; aggregate counts alone confer no throughput claim. |

R2 can be a bounded demo of R1 rather than a promise that dropping gameplay actors preserves the identical game. A
full cross-platform multiplayer game must use the same gameplay population/rules across clients; use visual culling,
LOD and server query projections to reduce its client cost. The R4 profile with R2 rendering cannot silently simulate
fewer relevant actors. Geometry fixtures specify 250k visible triangles in R0, 2M in R1 and 500k in R2 including shadow
submissions reported separately; imported content exceeding those planned loads is a different workload. Transparency
is capped at two admitted layers and 25% main-view coverage for the baseline fixture, with deliberate stress cases
reported separately. These are content constraints, not permission to discard a required transparent layer at runtime.

## Budget allocation and failure rules

The platform document remains the sole numeric whole-frame/residency envelope: desktop CPU 5 ms/GPU 10 ms at 60 Hz,
browser CPU 8 ms/GPU 16 ms and mobile CPU 7 ms/GPU 16 ms at 30 Hz; 3 GiB/512 MiB/1 GiB respective memory planning targets, not universal runtime hard limits.
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

For the first portable R2 recipe, the 512 MiB browser envelope gives 76.8 MiB world/solver/snapshot, 102.4 MiB
decoded content, 179.2 MiB persistent GPU products, 51.2 MiB transient GPU targets, 51.2 MiB staging/overlap and
51.2 MiB reserve. The four steady categories sum to **409.6 MiB**; staging and reserve are additional concurrent
allowances, not a promise that a second full world fits. A copy of a world using its entire 76.8 MiB share exceeds
the 51.2 MiB overlap allowance by 25.6 MiB before provider/product replacement, readers and scratch. The matching
1 GiB mobile shares are 153.6/204.8/358.4/102.4 MiB steady, 102.4 MiB staging and 102.4 MiB reserve; a full
153.6 MiB world copy likewise exceeds staging by 51.2 MiB. First R0 and portable R2 therefore preload their
authoritative gameplay closure and restart or enter a visible stop/load before substantive source/region replacement.
The stopped run releases old world, provider and product leases before admission of the new closure; it cannot claim
an uninterrupted simulation. A later live authoritative streaming recipe must name `M_live + M_staged_W1 +
M_provider_replacements + M_old/new_products + M_readers + M_decode/upload_scratch + headroom` with shared allocations
counted once, simultaneous CPU/GPU peaks and a specific 30/60 Hz transition workload. Its provider state-transfer
rules and numerical gate are separate from the first portable admission. Client visual residency or N1 interest
changes do not prove server gameplay-region streaming.

Cold browser startup uses a fixed 50 Mbit/s payload throughput, 40 ms RTT and cold caches for the initial ≤25 MiB
download/≤5 s interaction design target. 25 MiB alone consumes about 4.2 s of that payload rate, leaving less than
0.8 s for nonoverlapped fetch/setup/decode/pipeline work; the content build must reduce startup bytes or overlap work
to meet it. Report transfer and first-play completion separately; a blank responsive shell is not first play.
Native local-SSD startup retains the ≤1 s interactive-shell/≤5 s reference-scene targets. Mobile's initial content
slice is ≤40 MiB, distinct from total signed package size. R0 offline boot requires a complete verified installed
bundle; it is a separate case from the cold-network worksheet below.

### Browser first-play admission worksheet — estimates, not measurements

The boot closure is the exact target-specific code, manifest, gameplay data, image/geometry/audio products, shader
products and pipelines required before the first intended input produces visible feedback. Later optional regions and
unused providers are excluded only if the game can complete that interaction without them; a required collider,
font, rule, action or masked shadow role cannot be deferred to make the count appear small. These are planning limits
for the two named fixtures, not SDK format ceilings. Compressed transfer estimates are deliberately below 25 MiB
because 25 MiB leaves too little nonoverlapped work at the stated link rate. A target build must report its actual
closure and reject this reference recipe or revise it before export when admission fails.

| Browser boot item | R0 planning cap | R2 preloaded planning cap | Dependency and preparation assumption |
| --- | ---: | ---: | --- |
| Compressed Wasm/JS/host glue | 4 MiB | 5 MiB | Fetch after HTML; instantiate after bytes arrive; exact feature build and compression are unknown. |
| Compressed gameplay/geometry/material/texture/UI products | 3 MiB | 9 MiB | Manifest identifies hashes; fetch can overlap code; CPU decode and GPU upload follow each product. Includes all first-play collision and UI dependencies. |
| Compressed WGSL/descriptor and boot metadata | 1 MiB | 2 MiB | Fetch with products; browser compilation and pipeline creation follow device acquisition and validated descriptors. |
| **Total cold payload** | **8 MiB** | **16 MiB** | Ideal transfer at 50 Mbit/s is **1.34 s** / **2.68 s**; both are under the 25 MiB ceiling. |
| Distinct required device-ready pipeline tuples | **≤16** | **≤48** | Count exact material × geometry × pass × format/layout/override tuples; report depth, shadow, forward, velocity and UI roles separately. No first-draw compilation. |
| Maximum concurrent CPU decode/instantiate scratch | **24 MiB** | **40 MiB** | Charged with decoded products and download buffers to the 51.2 MiB staging share; measure allocator/browser overhead separately. |
| Maximum concurrent GPU upload/prepare scratch | **16 MiB** | **32 MiB** | Charged within the same 51.2 MiB staging share if simultaneous with CPU scratch; schedule peaks so combined staging stays ≤51.2 MiB. |

These CPU/GPU row maxima are separate phase limits, not additive concurrent reservations. The combined R2
decode/upload/prepare peak is capped at **48 MiB** in the staging share; serialize or chunk work when the two
individual peaks would overlap. Count retained browser responses, Wasm heap growth and driver-visible staging in
the measured peak. A stopped portable transition uses the same staging limit after old leases retire.

The first pipeline demand inventory budgets **R0 10 boot tuples** (2 depth, 2 shadow, 2 forward,
tone, spatial AA, final output and one UI pass when the authored fixture uses it)
and **R2 40 tuples** (6 depth, 6 shadow, 10 forward, 6 velocity, 4 transparency, 4 UI/post, 4 joint-media).
The spatial-only R0 fixture does not boot two unused velocity tuples; adding a temporal provider or an
admitted transition would add its reachable velocity and reconstruction roles to the closure. These are
placeholders for an admitted authored closure, with 6/8 tuples of headroom against the respective first
recipe caps; an actual material/geometry/pass combination can consume several tuples. A target report must enumerate
each concrete descriptor key and its role, not infer readiness from this arithmetic. The wider [Rendering](RENDER-SPEC.md#3-shader-closure-keys-and-readiness)
512-total/64-boot/128-live-switch controls remain reference-profile gates, not SDK-wide format limits.

Plan three request/response dependency turns (HTML/manifest, device-independent products, final readiness) at
**0.12 s** RTT cost. After the slowest required byte arrives, the provisional *nonoverlapped critical path* allowance
is **1.5 s for R0** and **1.8 s for R2** for Wasm instantiation, validation/decode, required GPU upload, all boot
pipeline creation, world construction, first tick and response. Fetch, device acquisition and product preparation
may overlap only when the dependency DAG permits it; do not subtract imagined overlap from a measured path. Simple
planning sums are 1.34 + 0.12 + 1.5 = **2.96 s** (R0) and 2.68 + 0.12 + 1.8 = **4.60 s** (R2), leaving
2.04/0.40 s against the 5 s design target for unmodelled browser/compositor overhead. The R2 margin is intentionally
tight: neither these allowances nor the bounded tuple inventory guarantees browser WGSL/pipeline completion time;
actual build bytes, cold pipeline time and browser memory must be measured before a support claim. The 16/48
tuple and scratch estimates are subordinate to the demand/readiness rules in [Rendering](RENDER-SPEC.md#3-shader-closure-keys-and-readiness),
not a second shader codec. The [performance walkthrough](PERFORMANCE-WALKTHROUGHS.md) supplies the capture and
unfinished-work checks. Browser playback uses verified runtime cache policy and separate player-save durability;
native source authoring is sufficient for the first SDK, and offline playback is a separately qualified claim.

Qualification uses a fixed five-minute warmup and 25-minute run, ten cold launches and separate ten-cycle lifecycle
traces. Record p50/p95/p99 intervals, CPU/GPU tails, peak memory and all unfinished/queued work. Acceptance requires
p95 presented interval ≤1.05×frame period, p99 ≤1.5×period, no unreported stall >100 ms outside declared loading,
and no continuing queue/memory growth. Stable display cadence alone cannot hide stale simulation or missing visual
work. Mobile thermal warnings, background loss and sustained throttling are recorded with device state; a cool burst
does not qualify the profile. Visual/motion and numerical acceptance are independent gates.

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

Early I1 shells verify boot content, tick, surface recreation and input on both named phones. Full I4 tests cover
text composition, accessibility, audio interruptions, thermal behavior and packaging. Backend/library incompatibility
is diagnosed at these qualification gates, not treated as evidence that game ownership should change.

Primary boundary evidence: [winit Android](https://docs.rs/winit/0.30.13/winit/platform/android/index.html) documents the
single matching activity-glue requirement and GameActivity option; [winit iOS](https://docs.rs/winit/0.30.13/winit/platform/ios/index.html)
documents native application integration and lifecycle. The selected devices, workloads and numerical budgets are
our engineering targets, not measured results or support claims from those sources.
