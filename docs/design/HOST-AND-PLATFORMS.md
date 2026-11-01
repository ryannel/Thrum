# Host engine and platform assessment

31 October 2026. Status: earlier comparison, superseded on host selection by the user's decision to build a custom
engine without Bevy as host. Retained for the actual integration inventory, costs and alternatives; recommendations
to keep Bevy provisional below describe the earlier assessment, not the current direction.
The [architecture brief](ARCHITECTURE-BRIEF.md) owns requirements and module responsibilities.

## Confirmed scope and decision

Modern desktops including Macs, scalable quality and browser demos are required. The user asks whether Bevy is
helping and explicitly permits considering abandonment. Evaluate it on future whole-engine requirements, not the
existing water implementation. No minimum Mac generation, desktop OS coverage or browser matrix is selected.

Separate three decisions: application/runtime services, rendering architecture, and GPU API abstraction. Replacing
one does not require replacing all three. Source access and permission to replace a plugin do not establish that
replacement will be cheap.

## What the existing branch actually reuses

Read-only inspection of `engine/rebuild` at `49a6bc78a2385a18f5d57c8fb15a5b4fa7c30cc5`, not a running-build audit:

- `Cargo.toml` pins Bevy 0.20.0 with selective features and wgpu 30. Its stated boundary is an application framework;
  it does not enable the normal Bevy PBR/animation feature bundle. Audio is separately supplied through Kira.
- `crates/host/bw_engine/src/plugins.rs` supplies Bevy plugin groups and app/headless configurations. This is real
  application scheduling and lifecycle reuse, not merely a dependency with no callers.
- `crates/host/bw_engine/src/render_host.rs` creates a custom camera render schedule instead of `Core3d`. It passes
  Bevy's GPU device, queue and main view texture to our renderer, draws egui, then uses Bevy's output blit. It also
  handles render-world extraction and camera bookkeeping; the engine computes its own projection.
- `crates/gpu/bw_gpu/Cargo.toml` depends directly on wgpu and engine crates, not Bevy. The rendering boundary already
  gives us substantial independence from Bevy's normal 3D pipeline.

Inference: the current arrangement captures fewer benefits than adopting Bevy's normal content/rendering stack.
It still saves host work. There is integration glue and overlapping view bookkeeping, but no measured evidence here
that Bevy is a performance bottleneck. Removing it is not automatically a graphics improvement. These findings
describe the inspected committed branch; other worktrees and uncommitted work were not audited.

## Current public evidence

Sources inspected on the date above; documentation claims are not local measurements.

| Source | Relevant evidence | Selection consequence |
| --- | --- | --- |
| [Bevy DefaultPlugins](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html), page identifies 0.20.1 | Optional plugins include time/tasks, input/windowing, assets/scenes, rendering, animation, audio and UI | Reuse could extend substantially beyond our current host, provided the data/rendering paths integrate |
| [Bevy plugin development](https://bevy.org/learn/quick-start/plugin-development/), architecture and licensing | Built-in functionality can be replaced with plugins; permissive MIT/Apache licensing and ecosystem guidance support contributions | A custom provider need not imply a fork, but integration must be demonstrated |
| [Bevy 0.20 release](https://bevy.org/news/bevy-0-20/), Solari and What's Next | Solari runs on macOS but has no built-in Mac denoiser; some light types are still forthcoming. Editor-related scene assets and remote inspection remain future work | Neither advanced lighting parity nor a complete integrated authoring workflow should be assumed |
| [Bevy 0.19–0.20 migration guide](https://bevy.org/learn/migration-guides/0-19-to-0-20/), overview and change inventory | Breaking changes span rendering, scenes, ECS and UI | Count upgrade work and third-party compatibility as recurring costs; pin a qualified release |
| [wgpu overview](https://wgpu.rs/) and [feature API](https://docs.rs/wgpu/latest/wgpu/struct.Features.html), observed version 30.0.1 | Native Metal/Vulkan/DX12 and browser WebGPU/WebGL2 paths; feature documentation distinguishes portable and native extensions | A custom wgpu host retains the graphics abstraction's restrictions. Platform support does not imply feature parity |
| [Apple Metal capability tables](https://developer.apple.com/metal/capabilities/), feature/family matrix | Ray tracing, mesh shading and reconstruction support vary by API and GPU family | Detect and qualify capabilities; “Mac” is not one high-end feature tier |
| [Godot web export](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html), requirements and threading | Browser export uses WebAssembly/WebGL2, with distinct threading/export constraints | A different full engine also has platform compromises; it needs the same profile review |

## Candidate architectures

| Candidate | Value | Cost or rejection reason |
| --- | --- | --- |
| Use more of Bevy, extend its normal renderer | Broad reuse of content, animation, materials, scene and runtime services; plausible plugin/upstream path | Reject if our required lighting/material/world contracts demand pervasive patches or duplicated representations that cannot meet budgets |
| Bevy runtime with a custom renderer | Reuse scheduling, lifecycle, platform and tooling integrations while owning rendering representations | We supply rendering/content bridges; animation, materials and asset features do not transfer into our renderer automatically. Reject if integration and upgrades cost more than the retained services save |
| Custom host on wgpu with selected libraries, optionally Bevy ECS alone | Control lifecycle and renderer; reuse graphics abstraction and proven lower-level libraries | We integrate surfaces, events, input, async assets, timing, tooling, packaging and recovery. Independent ECS reuse does not deliver a complete runtime. Reject if this infrastructure displaces the distinctive engine work |
| Another full engine | Candidate when integrated authoring and game production dominate | Requires a fresh extension/platform/content audit. This study has not established a superior replacement; Godot's web profile illustrates one tradeoff |

Direct Metal plus Vulkan/DX12 backends are a further graphics decision, not the default consequence of rejecting
Bevy. Pursue only when a required, measured technique cannot be delivered through the chosen abstraction and the
benefit warrants separate implementations, shader paths and validation. Browser limitations still remain.

## Recommended reuse boundary — conditional on host selection

- Reuse commodity foundations: scheduling/ECS, platform event handling, asset formats/decoders, established physics
  and navigation libraries, audio, text/UI and GPU abstraction where they fit. Audit compatibility before selecting
  packages. Do not commission new solvers or importers merely to own them.
- Own the engine's capability/profile policy, state and lifecycle contracts, environmental coordination, quality
  budgets, game-facing APIs, reference scenes and quality evidence. Own specialist algorithms where research shows
  that existing providers cannot deliver the intended result.
- For content and characters, assess the complete route: authored scene → import → animation/pose → geometry and
  materials → lighting/shadows → culling/streaming → authoring feedback. Counting an animation plugin is insufficient
  if its output cannot feed the selected renderer without substantial new work.
- Keep independently useful algorithms, durable content schemas and required simulation contracts separable from
  the host. Use the chosen host directly elsewhere; do not build a speculative wrapper for every API or promise
  effortless engine replacement.

## Platform design

Propose a portable native raster/compute baseline with optional advanced providers. Recommend initial Apple-silicon
qualification, while leaving exact generations and Intel coverage open. Qualify browser WebGPU separately, including
download/startup, memory, async loading, shader preparation and input/audio lifecycle. WebGL2 is optional extra scope.
The browser can use reduced scene extent, cheaper representations and different lighting providers while retaining
shared world semantics and content sources. Desktop quality need not be capped by browser limits.

Profiles must select compatible providers, not just dial sample counts. No retained consumer may depend on a disabled
provider without an explicit substitute. Gameplay-affecting quality changes require separate acceptance. GPU effects,
animation, simulation and background work share an overall budget. Native-only features remain optional until the
entire supported configuration, including reconstruction and content coverage, is qualified.

## How to select without restarting an engine project

Recommendation: keep Bevy provisional and make the strongest comparison between deeper reuse and a smaller host.
Do not let the present hybrid survive by inertia, and do not maintain several complete engines to make the decision.

First define the workload and targets: an animated character, instanced environment, local lights and shadows,
transparent material, multiple camera styles, one distinctive environment provider, content loading and a minimal
edit/inspect loop. This exposes whole-engine integration better than a water-only or empty-window benchmark.

Then use source/API inspection to eliminate impossible candidates and prove only the uncertain boundaries. Compare
equivalent content and output quality on a representative Mac, another desktop GPU and the browser target. Record:

- Required capabilities achieved and visual/temporal defects, not just frame rate.
- CPU/GPU frame distributions, memory, startup/download and shader stalls against agreed budgets.
- Import/edit/debug iteration, amount of integration code and repeated data conversion.
- Dependence on private APIs, upgrade/fork burden and missing authoring work.
- Whether an optional provider actually retires its avoidable work and resources.

Reject Bevy if a mandatory contract cannot be implemented through maintainable public integration, if measured host
overhead fails the agreed budget when a fair smaller-host comparison passes, or if replacement and ongoing ownership
are credibly cheaper than integration/upgrades. Retain it if it supports the target without those costs and lets us
spend more effort on distinctive capabilities. A single missing rendering feature calls for provider evaluation
before concluding that the entire runtime must go.

These are proposed decision criteria. No comparative builds, timings, migration-cost estimates or host selection
were completed in this design discussion.
