# Engine capability survey and inventory decisions

1 November 2026. Official feature pages and selected manuals for twelve engines were inspected by the coordinator
and three fresh Sol High agents. This updates the **recommended design inventory**, not implemented capabilities.
The [brief](ARCHITECTURE-BRIEF.md) remains the module map; [construction](CONSTRUCTION-PLAN.md) remains the ordered plan.
Custom host, 3D, Mac/PC, browser demos and native mobile remain the target. A01–A05 remain open. The
[build plan](../BUILD-PLAN.md) now owns V1 scope; engine comparisons inform the inventory but are not quality gates.

The strongest lesson is to describe complete creator outcomes alongside runtime responsibilities. We already cover
most broad engine systems. Several apparent omissions disappear when reading the owner designs, particularly
[UI/text/accessibility](modules/AUDIO-UI.md). Others need explicit creation, inspection and delivery paths. We should
strengthen those paths without acquiring a separate module for every feature on a competitor's website.

## What the official pages suggest

These are advertised/documented capabilities, not independently measured quality, portability or performance.
The smaller projects are active comparators; this survey does not establish an adoption or popularity ranking.
Engine-wide platform lists do not certify each feature or the editor on every platform.

| Engine and official entry points | Ideas worth carrying into our design | Limit on the inference |
| --- | --- | --- |
| [Unreal features](https://www.unrealengine.com/features) | Treat animation, world construction, content tools, gameplay authoring and delivery as a connected product alongside rendering. Keep advanced simulation and rendering selectable. | Large production tooling is precedent, not our minimum scope. Release features have individual maturity labels. |
| [Unity Engine](https://unity.com/products/unity-engine), [build profiles](https://docs.unity.com/en-us/engine/6000.3/manual/building-and-publishing/build-settings/build-profiles) | Source-controlled target configurations; creation and profiling belong in the normal developer loop. | Services and multiplayer products are separate scope decisions; editor/player/build-host support differ. |
| [Godot features](https://godotengine.org/features/) | Reusable scenes and custom resources become editable game building blocks; import and target-device debugging are first-class workflows. Offline movie capture serves demos as well as games. | Its language/platform combinations have limits; no need to adopt its node architecture, scripting language, 2D or XR scope. |
| [O3DE features](https://o3de.org/features/), [visualization tools](https://docs.o3de.org/docs/user-guide/visualization/) | Optional feature packages, text-based prefabs, automated asset processing, level blockout, animation and cinematic authoring. | Its networking and terrain scale are not our selected requirements or measured budgets. Optional Gems do not justify a universal plugin framework. |
| [Bevy](https://bevy.org/), [introduction](https://bevy.org/learn/quick-start/introduction/) | Direct composition, optional systems and render-graph organization; distinguish asset reload from game-code reload. | Remains a reference, not our host. Upstream warns of API churn and incomplete features/documentation. |
| [Fyrox feature inventory](https://fyrox-book.github.io/introduction/introduction), [1.0 release](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/) | Animation and terrain tools beside the runtime; inspect reimport and export failure cases, including dynamically loaded content. | Documented model-reload identity hazards and used-asset export limits matter as much as headline features. |
| [Flax features](https://flaxengine.com/features/), [animation](https://docs.flaxengine.com/manual/animation/index.html) | Make animation transitions, retargeting, terrain and material work understandable to creators. | [Web support](https://docs.flaxengine.com/manual/platforms/web.html) is experimental with significant language/threading constraints. Public source does not imply permissive reuse licensing. |
| [Wicked Engine](https://wickedengine.net/), [feature inventory](https://github.com/turanszkij/WickedEngine/blob/master/features.txt) | Detailed material/interchange support and specialist rendering can coexist with a small framework/editor; controller feedback deserves an owner if selected. | [Listed platforms](https://github.com/turanszkij/WickedEngine) do not establish browser or Android support. A long graphics list is not measured integrated quality. |
| [Stride features](https://www.stride3d.net/features/), [assets](https://doc.stride3d.net/latest/en/manual/get-started/assets.html) | Code-only and editor routes; show why an asset is included, included by dependency or excluded. | [Platform docs](https://doc.stride3d.net/latest/en/manual/platforms/index.html) distinguish Windows Game Studio from runtime targets; [FAQ](https://www.stride3d.net/faq/) excludes web. |
| [PlayCanvas engine](https://developer.playcanvas.com/user-manual/engine/), [Editor](https://developer.playcanvas.com/user-manual/editor/) | Lean optional systems, a short browser creation/publish loop and actual runtime inspection for agents. | [Mobile publishing](https://developer.playcanvas.com/user-manual/editor/publishing/mobile/) uses web-content wrappers; it does not qualify our native mobile architecture. |
| [Babylon.js specifications](https://www.babylonjs.com/specifications/) | Authorable materials, particles and render passes; inspectable graphics and incremental content delivery. | Separate node tools are examples of workflows, not a requirement to build several editors. Native-host listing does not certify target parity. |
| [Defold product](https://defold.com/product/), [size optimization](https://defold.com/manuals/optimization-size/), [Live Update](https://defold.com/manuals/live-update/) | Feature removal should reduce actual output; distinguish base package from on-demand content and installed content from memory residency. | Its strengths emphasize 2D; no inference of our 3D visual quality. [HTML5](https://defold.com/manuals/html5/) has hosting/threading and reload constraints. |

Independent source extractions and detailed caveats:
[major engines](../evidence/handoffs/2026-11-01-engine-capability-survey/MAJOR.md),
[smaller/native frameworks](../evidence/handoffs/2026-11-01-engine-capability-survey/MODERN.md),
[web/mobile](../evidence/handoffs/2026-11-01-engine-capability-survey/WEB-MOBILE.md).
The coordinator reconciled recommendations against the owner designs: the initial major-engine suggestion to add
semantic accessibility was already covered. Localization runtime was also present; production tooling is the delta.

## Inventory changes

“Make explicit” means existing capability, now expressed as a usable outcome. “Extend” adds recommended scope inside
an existing owner. “Optional” records a candidate without making every game depend on it. Nothing here is a claim
that tools or platform behavior have been implemented. Detailed mechanisms still belong to the named commissions.

| ID / disposition | Creator outcome and change | Existing owners / stage |
| --- | --- | --- |
| H01 — Make explicit | **Import a character and make it move well.** Bind skeleton/clips, preview transitions and root motion at game scale, edit event markers and diagnose retarget/IK errors. Provide pose/root/event traces through the shared tools; a visual graph editor is optional. Flax/Fyrox expose the gap between runtime animation support and an authoring workflow. | P4/F4/T1/T2; D1/D2 design, I3 initial path. |
| H02 — Make explicit / optional tool extension | **Build a playable place.** Start with imported geometry, including blockout made externally; for outdoor profiles, use an imported terrain or recipe, surface regions and vegetation placement. Inspect render/collision/nav representations and a revised region boundary together. O3DE/Fyrox/Flax motivate a complete workflow; in-engine blockout modeling, sculpting, spline roads and biome editors remain optional. | W2/E1/F4/P3/P5/T1; I2 content, I3 outdoor slice. |
| H03 — Make explicit | **Author an art style.** Material/light/effect controls, bounded custom shaders/passes, preview and compatibility diagnosis are capabilities. PBR, stylized/unlit and hybrid profiles use the existing rendering ownership. Babylon/Stride demonstrate tooling choices; no requirement for separate material/geometry/particle node editors. | V1/V3/V4/V5/T1; D2, I3; A03 readiness gate. |
| H04 — Extend | **Prepare a localized release.** Existing U1 already specifies message IDs, plural/select, fonts, RTL, formatting and locale switching. Add extraction/interchange, missing/obsolete-key diagnostics, expansion/pseudo-localization preview, and production/validation for locale-specific voice/art when selected locales require it. A1 already models localized audio cues; translation services are optional. | U1/F4/T1/T2; D1 formats, I3 UI path, I4 selected locales. |
| H05 — Make explicit | **Use accessible menus and text input.** Promote existing semantic tree, keyboard/controller/touch focus, IME, captions and native/browser bridge obligations into the brief. Include DOM/platform UI in capture scope or explicitly report its exclusion. Do not claim accessibility from canvas pixels or a semantic mock alone. | U1/P1/F1/T2; I3 semantics, I4 actual target bridges. |
| H06 — Extend | **Know what ships and why.** Show roots, dependencies, dynamic asset families, omitted features and byte attribution for code/assets/decoders. Dynamic loads must use declared IDs/families or an explicit external-content policy. Validate known references before export; fail undeclared runtime requests clearly. Static inspection cannot prove arbitrary code's future requests. | F4/F2/P6/T2; D1 admission design, I2 closure, I4 artifact evidence. |
| H07 — Split existing scope | **Reach first play without downloading everything.** Distinguish startup closure, on-demand packages, installed/offline readiness and in-memory residency. Define retry/integrity/version behavior and prerequisites for the selected first playable scene. Offline runtime, offline cooking and offline SDK installation remain separate promises. | F4/W2/T2; D1, I2, target-specific I4. |
| H08 — Make explicit | **Use a reproducible target configuration.** Store named development/demo/release builds in source; keep capability/quality choices distinct from build mode, signing and package metadata. Report editor OS, build host/toolchain, player target, GPU/backend and feature support separately. Browser page, native app and optional WebView shell are different delivery forms. | F1/F2/T2; D0 matrix, D1 format, I4 qualification. |
| H09 — Narrow promise | **Know what an edit will do.** Define property, imported-content, shader, game-code, schema and profile changes separately: affected world, restart need, preserved state, rollback and target availability. Baseline game-code iteration rebuilds/restarts. Replace ambiguous blanket “live editing” with declared change support. | F1/F3/F4/P7/T1/T2; D1, A01/A02 and A03 where applicable. |
| H10 — Make explicit | **Trust an import result.** Publish per-profile format/material-extension support: preserved, converted/baked, explicit fallback or rejected. Preview unsupported paths and content/shader requirements. “glTF support” is not universal extension or material fidelity. | F4/V3/V1/T1/T2; D1/D2, I2/I3 supported assets. |
| H11 — Make explicit / optional tool extension | **Compose and record a showcase.** P6 already sequences cameras, animation, audio and game commands; V5 consumes identified events for effects. Expose inspectable authored sequence state and target-view capture provenance. A timeline UI and fixed-step offline movie export are optional tools justified by demo needs, not a new simulation clock or a video decoder. | P6/P2/P4/A1/V5/T1/T2; I3 sequence/capture, later optional export. |
| H12 — Optional addition | **Give controller/mobile tactile feedback.** If selected, P1 owns feedback cues/player settings and F1 owns device output; consume P6 causes once, independently of audio. Specify capability discovery, bounded intensity/duration, cancellation and opt-out. A1 currently mentions haptics without assigning its owner. | P1/F1; D1 boundary if selected, I4 selected-device qualification. |

H04's production pipeline, H06's explicit dynamic-content rule and H12's optional output ownership are substantive
scope refinements. Most other changes expose or connect existing responsibilities. They do not require new module IDs,
a larger kernel, another world registry or a separate agent execution system.

## Keep the quality ambition; remove accidental promises

Retain selectable physics, animation, navigation, audio, UI, saves, streaming, cameras and diagnostics. Retain modern
lighting/shadows/reflections, coherent materials and atmosphere/water options, temporal stability, effects and scalable
geometry. The survey provides no reason to remove those responsibilities or abandon custom composition.

| Capability seen in competitors | Disposition for this engine |
| --- | --- |
| Hardware ray tracing, virtual geometry, sophisticated GI/upscaling, motion matching, cloth, advanced water/atmosphere | Optional implementations with declared target/cost/content requirements. Preserve simple usable providers. A feature label is not a reason to raise the portable baseline. Multiple water/air models still require A04/A05 compatibility work. |
| Visual scripting, runtime script VM, arbitrary native code reload, general graph editors | Remove any implication of baseline support. Retain code/data extension seams and selected authoring tools; add a language/editor only for an identified creator need. |
| Arbitrary renderer replacement, runtime plugin unload, every provider combination | Not promised. Preserve headless operation, feature omission, constrained rendering extensions and supported model combinations. |
| Video playback, splats/VRM, advanced terrain sculpting/roads, full cinematics suite, digital-human/ability packages | Optional later capability or game package; no baseline module commissioned. Video has separate decode/sync/caption/platform obligations. |
| Normal co-op/networking | **Since decided:** multiplayer is first-class in the architecture (E13). Replication, authority, prediction and save compatibility shape boundaries now; V1 proves fitness headless (G2S) and transport follows V1. |
| Pure 2D, MMO, XR/consoles, LiveOps/account/commerce/cloud services | Outside or unselected scope. Their presence on product pages does not justify adding them. |
| Equal feature parity on all platforms, native mobile via a WebView, zero-cost disabled features | Remove these blanket claims. Retained consumers can require shared work; qualify actual artifact size, residual jobs/memory and target-specific substitutions. |

## Coordination and qualification

D0 selected the first target/workload combinations, the networking disposition and which optional capabilities are
commissioned; the build plan now sequences them. D1 integrates content demand/delivery, localization source formats and iteration rules with
A01/A02. D2 handles render/animation/material requirements with A03. No UI proposal closes an authority or readiness gap.
Feature-qualified I3/I4 slices follow those mechanisms; optional tools must not delay the first small complete game.

For each advertised capability, track: a concrete creator action, responsible module, source data, required providers,
consumer interactions, target/build/authoring availability, omission behavior, readiness/maturity and evidence. The
[maturity ledger](maturity.json) records this survey as source research; all implementation/function/quality/platform
axes remain unchanged. A future capability showcase should demonstrate a whole workflow and final artifact on a named
target, rather than count modules or display an isolated visual effect.

The next detailed commissions should incorporate these deltas into the existing plan, not start twelve independent
feature projects. Localization shares F4 admission; terrain shares world/physics/navigation products; animation and
sequences share motion/event authority; rendering tools share V1 execution; human and AI clients share operations.
