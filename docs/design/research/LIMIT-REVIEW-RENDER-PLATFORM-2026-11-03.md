# Render/platform limit review — 3 November 2026

Inspected working tree at `e3ce277492e3e43e1b48e214d26d703a4d4a421c`; unrelated dirty CI/contributor files were left alone. Bounded read-only review using engine-architecture and engine-module-design review/performance instructions. Read README, relevant BUILD-PLAN items, C04/C05/C06/C07/C08/C12, BASELINE-SPEC, RENDER-SPEC, PLATFORM-RUNTIME, holistic policy, R3 optical/resource/failure design, maturity entries, mobile gate/clear-renderer/host adapter, and mobile/browser/FXAA evidence. This is an informed challenge, not a blind architectural derivation. No renderer, browser scene or device timing was run for this review.

Recommendation: retain the policy and narrow-owner implementation order. It accurately separates meaning, supported capacity and measured targets. Do not close the following design/mechanism and integration gates by citing isolated shaders, the mobile package, or repository checks. No P0 found.

## Findings

### P1 — Specify arithmetic preconditions before promising GPU fault detection

Locations: `docs/design/research/R3-OPTICS-AND-TRANSPARENCY-DESIGN-2026-11-02.md:592–600`; `docs/design/RENDER-SPEC.md:184`, `:218–225`, `:474–475`.

The R3 state machine includes an arithmetic fault bit, while RENDER-SPEC admits unbounded scene-linear emission and says nonfinite radiance is diagnosed. Neither selects the range/precondition mechanism that makes this enforceable. WGSL's finite-math assumption permits an indeterminate result after runtime overflow/NaN/infinity; a final `x == x`, clamp, or exponent inspection after the unsafe computation is insufficient. A finite authored intensity divided by an extremely small finite `r²` can already overflow before pre-exposure is applied. A plausible finite substituted result could leave the GPU status word clear.

Action: distinguish checked capacity/status operations from mathematical domain safety. Derive admitted ranges for inputs **and intermediates**, test divisor/discriminant/normalization preconditions before evaluating them, and use algebra/scaling that stays within those ranges. Validate externally supplied float bit patterns at CPU boundaries. Make the half-float/pre-exposure envelope explicit for alpha records as well as render targets; positive `p` alone does not prove a suitable dynamic range. Reserve an explicit unsupported/error route when the admitted range cannot be guaranteed. This is a design gap before the affected shaders are built, not a claim that existing clear rendering has this bug. [WGSL floating-point evaluation](https://gpuweb.github.io/gpuweb/wgsl/#floating-point-evaluation).

Proof: an independent f64/interval oracle generates the smallest admitted light radius, largest finite intensity/emission, critical-angle Snell cases, zero-length normals and near-zero integration extinction. Include values immediately outside each range. Assert outside cases set failure **before** the prohibited arithmetic; assert supported outputs and half-float packing meet the declared error. A test that only observes one backend propagating NaNs does not prove portability.

### P2 — A prepared fallback pipeline is not an executable current-frame fallback

Locations: R3 study `:598–605`; `docs/design/research/HOLISTIC-LIMIT-POLICY-2026-11-03.md:68–76`, `:170–175`.

The current-view gate may choose a whole fallback after resolving the failing primary path. Preparation establishes pipeline/product readiness, but not a current fallback image. The graph does not choose whether the fallback is rendered speculatively, executed after status using GPU-controlled work, or deferred through a visible loading transition. Those alternatives have materially different peak storage, traffic and worst failure-frame time. A fallback may require lighting/media inputs that failed in the primary, so readiness alone cannot establish eligibility.

Action: the first proof should select the already permitted diagnostic/loading surface. It needs a tiny independent path, GPU status selection and no failed-image history write. Treat richer same-frame fallback as a later named graph: enumerate its current inputs, failure independence, execution mechanism, last uses and failed-primary-plus-fallback work. Shader selection can choose an existing image; it cannot invoke an unencoded render graph. GPU-written indirect arguments are one possible mechanism, not a requirement. [WebGPU indirect draws](https://gpuweb.github.io/gpuweb/#dom-gpurendercommandsmixin-drawindirect).

Proof: force the final optical batch to exhaust its source cap, prohibit CPU status readback until several later frames, and verify the same submitted frame is diagnostic or a complete current fallback. Compare GPU image/status/history data, not producer logs. Repeat with fallback dependency missing and device generation changed. Measure failure-frame cost separately.

### P2 — The GPU lifetime account still lacks a selected cross-frame reuse schedule

Locations: `docs/design/CONTRACTS.md:184–197`; PLATFORM-RUNTIME `:91–95`; R3 study `:670–736`; holistic policy `:105–109`, `:135`, `:170`.

The policy correctly names this as unfinished. The R3 38.008 MiB is one resolve live set, while the host allows two frames in flight. Two private copies would cost approximately 76.016 MiB before other targets, exceeding the browser's 51.2 MiB transient share. This is **not** proof two frames require two copies: a single-queue sequential scratch arena can legally reuse resources after their actual final GPU use. The missing selection is which allocations are queue-serialized, per-frame rings, retained outputs, histories or fence-held old generations. Multiplying every row by two is as wrong as charging only one frame without a schedule.

Action: publish one concrete R0 allocation/lifetime schedule first, then extend it to R3. Use allocation identity, size/alignment class, generation, first/last GPU use, retained CPU reasons and submission completion token. Separate staging, live and retirement claims without allocating the same backing memory twice. Cap queued submissions explicitly; swapchain latency configuration is not a demonstrated complete resource-credit mechanism. Reconcile F4 decode/upload and V1 transition reservations at the point both become live, with a rollback route if actual allocation fails.

Proof: hold frame N completion, submit N+1, request resize/profile/BVH replacement, then cancel and restore. Verify no early reuse, premature release, double charge or reserve reuse. Log the simultaneous lifetime high-water and compare a single scratch arena against per-frame targets. Logical credits must remain distinct from device/process measurements. Native pressure estimates are advisory: [Vulkan memory budget](https://docs.vulkan.org/spec/latest/chapters/memory.html) and [Metal recommended working set](https://developer.apple.com/documentation/metal/mtldevice/recommendedmaxworkingsetsize?changes=la).

### P2 — Resolve output extent before treating portable recipes as finite resource plans

Locations: BASELINE-SPEC `:44–46`; holistic policy `:114–116`; `examples/i1-mobile-game/src/render.rs:80–101`.

This known open decision affects more than capture metadata. Fixed 720p internal pixels leave output-sized AA/UI/encoding/surface storage and resize overlap unconstrained. The mobile clear shell uses the complete physical window size, which is valid for its explicitly tiny proof and supplies no 720p/output-cap enforcement for a future scene. CSS size multiplied by DPR also does not uniquely specify the browser's device-pixel box at fractional scale.

Action: select internal extent, output extent, aspect/letterbox policy, per-dimension and pixel-count caps, integer rounding, zero-size behavior, capture extent and resize transaction. Prefer the observed device-pixel content box where available; specify and label the fallback. Resolve limits against the **requested device**, not only adapter capabilities. Update surface/output allocations atomically and retain old generations until completion. The first recipe can simply use a capped fixed output extent and CSS scale it, avoiding a dynamic-resolution framework. [W3C Resize Observer](https://www.w3.org/TR/resize-observer/#resizeobserverentry), [WebGPU device limits](https://gpuweb.github.io/gpuweb/#dom-gpudevicedescriptor-requiredlimits).

Proof: 1.25/1.5/2/3 DPR, fractional CSS sizes, portrait/landscape, hidden zero-size canvas, enormous viewport and repeated resize with two outstanding submissions. Assert exact output/capture dimensions and high-water bounds; review native crops and moving thin geometry separately.

### P2 — Distinguish GPU acceptance from CPU submission/history bookkeeping

Locations: RENDER-SPEC `:427–435`; R3 study `:587–605`; `docs/design/modules/RENDERING.md:82`.

R0 records the submitted sample; the wider temporal contract requires the sample actually retained, and R3 discovers validity on GPU. A masked history texture write alone cannot update the CPU's previous-pose/exposure/sample metadata correctly. If CPU advances it on submit while GPU rejects N, N+1 can read history N−1 with matrices or deformation from N. This is a future temporal extension gap; the selected spatial-only first proof correctly avoids it.

Action: explicitly keep submitted metadata separate from accepted-history metadata. Choose either GPU-owned accepted metadata/history selection with bounded retained geometry, delayed asynchronous acceptance with bounded CPU leases, or reset history on uncertainty. The simplest first temporal slice can reset instead of retaining ambiguous history. Queue completion is a lifetime event and does not certify presentation or semantic acceptance. [WebGPU queue completion](https://gpuweb.github.io/gpuweb/#dom-gpuqueue-onsubmittedworkdone).

Proof: accept N−1, reject N late on GPU, accept N+1 with camera motion and changed pre-exposure; delay readback, skip another view, then restore device. The retained image, previous matrices, source epoch and exposure must identify the same accepted sample. Never infer this from global frame parity.

## Sound elements and limits of present evidence

- C04/C05/C12 preserve semantic floors and owner authority, separate source validity from target readiness, count shared allocations once, and retain every reader/fence reason. Requiring a universal RSS ceiling would weaken the design by claiming something its allocator and platforms cannot guarantee.
- The R3 append path checks records before stores, rejects a third required event, separates pass synchronization from atomic ordering, and accounts for overdraw beyond its two stored slots (`R3 study:230–250`, `:654–661`). Keep checked CPU products/prefixes and GPU counter headroom; integer arithmetic wrapping is not detection. [WGSL integer types](https://gpuweb.github.io/gpuweb/wgsl/#integer-types).
- Full geometry/source-path status, terminal lookahead, source-query caps, and portable absorption are coherent supported-domain decisions. Sorted/weighted transparency and water scattering removal are explicit quality alternatives, not equivalent fallbacks. Do not require the ambitious focused-light provider before proving R0.
- BASELINE-SPEC `:174–178` already requires sustained runs, tails, startup and lifecycle traces; these are useful gates. Add matched failure/transition windows, input-to-visible/state-age measurements and thermal/low-power state to each concrete qualification workload. [Apple's game-performance talk](https://developer.apple.com/videos/play/wwdc2018/612/) explicitly advocates sustained thermal-aware workload selection and profiling individual passes; it supports this methodology, not Thrum's numeric budgets.
- `docs/evidence/i1-headless/MOBILE-APP-SHELL.md` reports black iOS simulator output, unlaunched Android packaging, and no lost-device recovery. `ClearRenderer` submits/presents a surface clear and calls that `Presented`; it does not observe scanout. Keep the distinction when building the frame evaluator. The browser walkthrough is explicitly invented paper data. None of these proves target support or aggregate policy enforcement.
- The 1080p FXAA timestamp proof isolates vertex-start→fragment-end on M3 Max and explicitly excludes attachment load/store, other passes and displayed latency. Its delayed serial completion capture is evidence for its finite method, not a shipping loop or M2/mobile/browser budget. The rejection of invalid query captures and separation of output/readback memory peaks are strong evidence practices to preserve.

## First executable package

Build only the R0 owner path with its real F4 consumer. Freeze a resolved recipe including exact output, requested device limits, concrete pipeline closure, allocation/lifetime schedule and independent diagnostic path. Exercise: normal input→world→image; rejected light/instance/list; failed pipeline/allocation; stale source/device callbacks; held submission during replacement; device loss; resize; optional consumer removal. Verify complete output and retirement before doing representative cold/sustained timing. Then extend the same evaluator with the R3 late-overflow and temporal-rejection cases above. This is simpler and more discriminating than building an engine-wide limits registry or testing each unbuilt owner in isolation.

External primary sources were fetched on 3 November 2026; WebGPU/WGSL editor drafts identify 30 October 2026. Their availability does not establish that pinned wgpu/Naga/browser versions implement every newly specified feature. No external secondary tutorial or unsupported browser compatibility assumption is used as evidence.
