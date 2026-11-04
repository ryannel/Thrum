# Hardware-native engine: capability registry, frame recipes and frame contract

4 November 2026. **Selected direction; implementation lands per gate.** Thrum's goal is a modern engine that is exceptionally fast because it uses the hardware each device actually has. A portable path is the correctness floor and the fallback, not the design target. Designing to a lowest common denominator, then leaving vendor and platform hardware unused, is a defect. This record amends how C04, C06, C07 and C12 apply; it does not change authority, identity or tick semantics.

## Capability registry

The platform runtime builds one immutable `DeviceCapabilities` record per adapter generation, before pipeline closure. It records:
* GPU vendor, architecture family, driver and backend;
* wgpu features and limits, plus backend-native features reached through the isolated escape hatch;
* tile or immediate-mode rendering, unified or discrete memory, and whether memoryless attachments are available;
* CPU features (NEON, AVX2, AVX-512), performance and efficiency core counts;
* display refresh range, VRR, EDR/HDR headroom and colour space;
* thermal and power APIs, storage IO APIs and ML or tensor hardware.

Every accelerated path selects from this record. No pass, system or shader queries the device independently. Profiles, evidence and bug reports carry the record's digest.

## Frame recipes

A recipe is a graph configuration chosen per GPU family: pass organisation, attachment storage, drawing and culling path, reconstruction, and shader variants. Quality tiers scale within a recipe; they do not choose between recipes.

| Recipe | Hardware | Shape |
|---|---|---|
| Apple tile | Metal on iPhone, iPad, Apple Silicon Mac | Memoryless depth and intermediates, merged passes, programmable blending through the escape hatch where measured, MetalFX reconstruction, EDR output |
| Android tile | Vulkan on Adreno and Mali | Subpass-shaped load/store discipline, transient attachments, ASTC, ADPF thermal feedback |
| Desktop high | Vulkan/D3D12 on NVIDIA, AMD, Intel | GPU-driven visibility buffer, mesh shaders where available, async compute, ray query, vendor reconstruction and latency APIs |
| Portable | WebGPU and every native device | Forward+ with GPU culling and indirect draws within WebGPU limits; own temporal reconstruction |

The portable recipe builds and runs on every native device. It is the reference for image comparison and the fallback when a dedicated path fails pipeline closure. A dedicated recipe or path is promoted only with a measured win on its own hardware. The win is either at least 10% lower GPU frame time or bandwidth, a lower p99 or latency, or a capability portable cannot express. Its image must also stay within the declared tolerance against portable. A path whose win disappears is removed. The web tier runs portable and never limits a native recipe.

## Frame contract

Each view publishes one frame contract, so reconstruction, interpolation and denoising providers plug in without renderer rewrites. It contains:
* jittered and unjittered projection, with current and previous view-projection;
* motion vectors in the C07 convention;
* reversed-Z Depth32Float depth;
* exposure;
* a reactive and transparency mask;
* camera cut and history reset flags;
* optional guide buffers: albedo, normals, roughness and hit distance.

Providers include Thrum's own TAAU, MetalFX spatial and temporal, DLSS, FSR, XeSS, frame interpolation, and neural or vendor denoisers. Each is an optional provider selected by the registry, with own TAAU as the portable fallback. Rendering below output resolution and reconstructing is the default for heavy views on every recipe. Native resolution remains a quality setting.

## Quality governor

A closed loop holds the frame within budget. Its inputs are GPU pass time, CPU tick and encode time, missed presents, thermal state, power mode and link quality. Its outputs are render scale (feeding reconstruction), per-effect quality and update cadence for time-sliced passes. It applies hysteresis and records every change in the profiler. It never changes simulation rate, authority or replicated state.

## Hardware use by layer

* **GPU, V1:**
  * GPU-driven frustum and occlusion culling with indirect draws;
  * meshlets from meshoptimizer, with mesh shaders as a variant;
  * f16 shading arithmetic where supported;
  * executor-derived memoryless and transient attachments;
  * mapped instance rings on unified memory;
  * per-adapter persisted pipeline caches;
  * timestamps on every pass;
  * display-link pacing, EDR/HDR output and MetalFX on Apple.
* **GPU, after V1:**
  * visibility buffer on desktop high;
  * VRS and rasterisation rate maps;
  * hardware ray query for shadows, AO and reflections, with SER/OMM where present;
  * a radiance cache for GI;
  * vendor reconstruction and frame generation;
  * Reflex and Anti-Lag;
  * async compute;
  * tensor and neural-accelerator inference.
* **CPU:**
  * Simulation and encode run on performance cores at user-interactive QoS. Cook, streaming and compression run on efficiency cores at utility QoS.
  * SIMD kernels are selected by registry with a scalar reference and an equality test.
  * Work-stealing jobs respect core classes.
* **IO and content:**
  * BLAKE3 content hashing;
  * zero-copy memory-mapped cooked products;
  * KTX2 with ASTC or BC7 by registry;
  * MTLIO and DirectStorage readers after V1, behind the same reader trait.
* **Network:** Quinn with GSO/GRO segmentation offload, plus schema-driven residual coding.
* **Tooling:**
  * Tracy capture of CPU and GPU zones;
  * hot patching of game code in development;
  * pipeline and shader caches shared by the dev cook server.

## Shading and backend

Authored WGSL remains current. G3 includes a time-boxed decision on a shading library. The choice is between WGSL with naga specialization and Slang compiled to MSL, SPIR-V and WGSL through wgpu passthrough, and it is decided by variant count, specialization and native feature reach. Stay on wgpu. Raw backend calls live in one module per backend behind the graph, each with a portable fallback. A direct Metal or Vulkan backend requires measurements showing the escape hatch dominates.

## Ray-tracing readiness constraints on G3

The bindless material table must be readable from compute and hit shaders, and the mesh arena must give stable device addresses. G1 dirty sets drive acceleration-structure refit and rebuild. Temporal history must be reusable by denoisers. Ray tracing is not V1 scope; these constraints prevent a later rewrite.

## Verification

Each recipe has named devices:
* **V1:** Apple tile on a physical iPhone and an Apple Silicon Mac; portable on both, plus Chrome as a smoke run.
* **After V1:** Android tile on Adreno and Mali, and desktop high on NVIDIA, AMD and Intel, as devices are acquired.

Evidence records frame time per recipe against portable on the same device, image difference, pipeline-closure hitches, and memory and thermal state after ten minutes. An unmeasured recipe is implemented, not qualified. Repository checks prove none of these.

## Sequencing

| Gate | Hardware work |
|---|---|
| G3 | The registry, the portable and Apple tile recipes, the frame contract fields, executor-derived attachments, pipeline caches, timestamps and indirect GPU culling. The ray-tracing readiness constraints. The shading-library decision. |
| G4 | Display-link pacing, EDR/HDR output, dynamic resolution with MetalFX and own TAAU, the governor, and input-to-photon latency measurement. |
| G5 | Meshlets, KTX2 ASTC/BC7, BLAKE3, memory-mapped products and dev-server cache sharing. |
| G6 | Quinn GSO/GRO, residual coding, and QoS-classed network and simulation threads. |
| After G7 | Android tile, desktop high, vendor providers, ray tracing, GI, frame generation and ML inference, each promoted by measurement. |
