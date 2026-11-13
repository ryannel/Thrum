# Thrum module list

| # | Module | What it's for | Needs |
|---|---|---|---|
| 1 | PLATFORM-RUNTIME-IOS | The game runs on the iPhone simulator, through touch, rotation, backgrounding and graphics loss | |
| 2 | PLATFORM-RUNTIME-MAC | The game recovers from graphics loss on the Mac, and its motion is proved smooth on the built-in 120 Hz display | T2-CAPTURE M0, APPLE-TILE M3 |
| 3 | T2-CAPTURE | Agents record exactly what the engine drew, as stills and motion, with nobody at the screen | PLATFORM-RUNTIME-MAC M1, COOKED-DRAW M1 |
| 4 | COOKED-DRAW | Cooked scenes draw on devices that cannot run GPU-driven draws, such as the iPhone simulator | PLATFORM-RUNTIME-IOS M7 |
| 5 | APPLE-TILE | Apple devices run their own frame recipe beside the portable one, with a native Metal step inside the frame | T2-CAPTURE M0 and M1, PLATFORM-RUNTIME-MAC M1, COOKED-DRAW M1 |
| 6 | CREATOR | A newcomer starts a scene with one command, tunes it with controls and sees where frame time goes, on Mac and in Chrome | APPLE-TILE M3 |
| 7 | APPLE-PRESENTATION | Sharp, stable images from upscaling, highlights brighter than SDR white, and a governor that holds the frame rate | APPLE-TILE M0, M2, M3, M11 and M12 |
| 8 | ATMOSPHERE | One physical sky, Sun, haze, night and stars, which every other part of the engine reads | APPLE-PRESENTATION M3 |
| 9 | BASELINE-GUARD | A change that alters another module's exact reference images is refused before it reaches main | |
| 10 | CULLING | Only what the camera can see is drawn, decided on the GPU | APPLE-TILE M2, BASELINE-GUARD M1 |
| 11 | EFFECT-GRAPH | Sea, clouds and sky run their own simulation and drawing steps inside the one frame | APPLE-TILE M2 and M3, APPLE-PRESENTATION M2, BASELINE-GUARD M1 |
| 12 | CONTENT-LOOP | Edited meshes, textures and volumes appear in the running game without a stall | APPLE-PRESENTATION M2 and M3, EFFECT-GRAPH M2, CULLING M3 |
| 13 | SHOWCASE | The V1 exit: the showcase valley on Mac and in Chrome, reviewed, with its closing record | CONTENT-LOOP M2 and CULLING M3 to start; the last milestone of modules 1 to 10 and 12 to finish |
| 14 | SEA | An open sea from waterline to horizon, with floating bodies, wakes and foam | Nothing to start; EFFECT-GRAPH M3, ATMOSPHERE M5 and CREATOR M1 for its first frame; CONTENT-LOOP M1 for foam; APPLE-PRESENTATION M3 for the whole range in motion, and its governor and MetalFX milestones to close |
| 15 | CLOUDS | A weather-driven sky from clear to overcast, with placed cloud volumes, that shades and lights the world | Nothing to start; EFFECT-GRAPH M2 and ATMOSPHERE M5 for its shade; EFFECT-GRAPH M3 for its first frame; CONTENT-LOOP M1 for placed volumes; APPLE-PRESENTATION M3 and CREATOR M1 for controls and motion; the owner's licence for the accepted library; SEA M8 for the sea; APPLE-PRESENTATION's governor and MetalFX milestones to close |
| 16 | SIM-2D | Thousands of 2D bodies step identically on every device, and find neighbours and hits cheaply | SHOWCASE M8 |
| 17 | TRANSPORT | Two processes exchange messages over real internet protocols, one of them a browser | SHOWCASE |
| 18 | RENDER-2D | Thousands of crisp sprites, drawn without paying for anything 3D | SHOWCASE M8, APPLE-TILE M12 and EFFECT-GRAPH M3 to start; CONTENT-LOOP M3 and SIM-2D M1 for its last two milestones |
| 19 | SESSION | Two to four players share one run and keep seeing the same crowd | SIM-2D M3 to start; TRANSPORT M1 for its second milestone and TRANSPORT M2 for its socket milestones |
| 20 | GAME-UI | Health bars, timers, menus and a join screen drawn in the game's own frame | RENDER-2D M5 |
| 21 | SWARM | The swarm-survival game itself: its name, rules, characters and art | SHOWCASE M8 for the owner's pick of a concept; SIM-2D M3, SESSION M1, RENDER-2D M5 and GAME-UI M5 for the game alone; SESSION M4 for a rehearsed match and SESSION M6, with TRANSPORT M2, for co-op on sockets |

Modules are in build order. Rows 1 to 13 are the road to the V1 exit, 14 and 15 are sea and clouds, and 16 to 21 are
the goal after V1. `Needs` names the milestones a module waits on in other modules; each milestone's `needs` in
its done-when file is the authority. Where each module stands is on [the status page](STATUS.md), which the
loop writes.

This is the project plan the orchestrate loop builds from: the modules, what each owns, the build order, the
interfaces between them and a brief for each module not yet planned. First written 10 November 2026.
[The brief](BRIEF.md) owns the purpose and the V1 exit. [The architecture brief](../design/ARCHITECTURE-BRIEF.md)
owns the engine-wide responsibility map (F1–T2) and [CONTRACTS](../design/CONTRACTS.md) owns shared invariants. A
module here is a unit of build ownership: one builder can work on it without touching the others, and each kind of
state has exactly one module in charge of it. The goal after V1, a swarm-survival game with 2D and online
multiplayer, is [track 3](#track-3-the-swarm-survival-game-after-v1). It was added the same day from a second
request, quoted there.

## The owner's request

Quoted word for word from the 10 November planning task:

> Plan the rest of Thrum's road to delivering the engine, as a project-level plan that brings water and clouds into
> it.
>
> Purpose. The owner wants work lined up continuously toward the V1 exit in docs/BUILD-PLAN.md, so builders always
> have a next milestone, and wants water and clouds finished and consolidated. Today only PLATFORM-RUNTIME-MAC M2
> and APPLE-TILE (G3t) are planned and open; everything after them is unplanned.
>
> Water and clouds are built in the Broadsides repository at /Users/ryannel/Workspace/Broadsides, not here. Water is
> on branch codex/water-rust-system, with notes/water/DESIGN.md, DESIGN-PLAN.md and HANDOFF.md. Three spikes on 10
> November left results on branches claude/fable-hull-release, claude/fable-near-waterline and claude/fable-foam
> (notes/water/FABLE-*.md on each). Two cloud paths, a baked library and a procedural one, were built by separate
> Codex agents in Broadsides; find their branches and notes there. docs/design/decisions/SEA-AND-CLOUD-PORT.md
> governs bringing them into Thrum: it says Thrum wasn't ready on 6 November and that the owner schedules porting.
> The owner has now asked for both to be finished and consolidated. Decide, with reasons, what finished means for
> each, whether that finishing happens in Broadsides first or as Thrum modules, and where in the build order the
> port's readiness conditions are met.
>
> Done means: the project plan (module map, build order, contracts and a brief per module) covers the remaining work
> to the V1 exit plus water and clouds; each module that can start next has its design, plan and done-when file, so
> a builder can start it; and the build plan's active section points to it. List anything that needs an owner
> decision, such as port timing or spending, in your result rather than deciding it.
>
> Boundaries. Don't change the definitions of PLATFORM-RUNTIME-MAC M2 or APPLE-TILE, because builders are on them.
> Read Broadsides but don't change it.

## Three tracks

**Track 1, the V1 exit.** Unchanged. Its gates from here are G3b, G3s, G3t, G3c, G4, G5, G6 and G7; G0, G2S and G3a
closed before the loop. Nothing in track 2 gates it.

**Track 2, sea and clouds.** Two engine modules, `SEA` and `CLOUDS`, built in Thrum on the foundation track 1
lays. They are finished here, not in Broadsides; [the reasons](#where-sea-and-clouds-are-finished) are below.
Track 2 adds one foundation module, `EFFECT-GRAPH`, and asks track 1 for three things it was already going to
build. Both are planned and start under their plans' assumptions on the owner's words of 13 November
([SEA](SEA/PLAN.md#open-for-the-human), [CLOUDS](CLOUDS/PLAN.md#open-for-the-human)).

**Track 3, the goal after V1.** An original swarm-survival game that proves 2D and online multiplayer, set by the
owner in [the brief](../design/NEXT-GOAL-SWARM-GAME.md). Six new modules, none of which starts before the exit or gates
it. They are listed in [their own section](#track-3-the-swarm-survival-game-after-v1), so the tables below stay
the road to the exit.

## Modules and what each owns

| Module | Gate | Owns (state and code) | Design or brief |
|---|---|---|---|
| `PLATFORM-RUNTIME-IOS` | G3b | The iOS shell: the UIKit lifecycle, touch, the render owner and its surface lease, rotation, device-loss recovery, and the scene extent that follows the oriented surface | [Design](PLATFORM-RUNTIME-IOS/DESIGN.md) |
| `PLATFORM-RUNTIME-MAC` | G3b | Qualification of the Mac surface, present and cadence path | [Design](PLATFORM-RUNTIME-MAC/DESIGN.md) |
| `T2-CAPTURE` | T2 | Engine capture: the submitted-output tap, its readback and writer, the capture modes and their evidence | [Design](T2-CAPTURE/DESIGN.md) |
| `COOKED-DRAW` | G5 | The direct cooked draw path and the selection between it and the indirect path | [Design](COOKED-DRAW/DESIGN.md) |
| `APPLE-TILE` | G3t | Recipe selection from the capability registry, the frame lifecycle and its one split scene/output graph, the pre-tone scene-linear input, the native Metal segment | [Design](APPLE-TILE/DESIGN.md) |
| `APPLE-PRESENTATION` | G4 | The view record (jitter, matrices), motion and reactive targets, history keys and acceptance, reconstruction, render extent and generation transitions, exposure retention, EDR present, the display record, the quality governor, pipeline preparation | [Design](APPLE-PRESENTATION/DESIGN.md); [brief for M4–M11](#apple-presentation-m4m11-g4) |
| `ATMOSPHERE` | G3s | The one sky, Sun, aerial perspective, night and stars; the environment identity; the direct-Sun path to receivers | [Design](ATMOSPHERE/DESIGN.md) |
| `BASELINE-GUARD` | T2 | The exact-baseline registry, the one-command verifier, its proofs and the proof gate in `./dev check`. Retained references stay with each baseline's owner | [Design](BASELINE-GUARD/DESIGN.md) |
| `CULLING` | G3c | Instance visibility lists, occlusion history, the culling mode and its resolution, the mesh-bucket route and Sun-caster culling | [Design](CULLING/DESIGN.md) |
| `CONTENT-LOOP` | G5 | Cooked products, the manifest, runtime mapping and GPU residency of cooked meshes, textures and volumes; leases of resident products to other modules; the geometry of cooked mesh rows | [Design](CONTENT-LOOP/DESIGN.md) |
| `CREATOR` | G6 | The typed controls model and its views, parameter files, the app runner, sky kit and starter, the creator docs, the profile window and profiler rows | [Design](CREATOR/DESIGN.md) |
| `SHOWCASE` | G7 | The exit scene in `examples/v1-game` (its content, layout and generator, prop sources and mover rule), the routes, the exit configuration, the launcher `scripts/run_v1_showcase.py`, the V1 exit record | [Design](SHOWCASE/DESIGN.md) |
| `EFFECT-GRAPH` | track 2 | Graph resource kinds and cadence that simulated effects need; how an engine module declares its own passes; the list of contributions a scene holds | [Design](EFFECT-GRAPH/DESIGN.md) |
| `SEA` | track 2 | The sea model and its f64 oracle (`thrum-sea-model`); sea state on the GPU (long-wave field, spectral cascades, wake levels, foam stocks), the body-query ring, the sea surface and its optics (`thrum-sea`); the sea example scene (`examples/sea`) | [Design](SEA/DESIGN.md); [brief](SEA/BRIEF.md) |
| `CLOUDS` | track 2 | The weather field, its state's step and the f64 references (`thrum-cloud-model`); the weather map, cloud density and noise, the Sun-path optical-depth product, the cloud layer and its history, the sky overlay, placed-volume sampling (`thrum-clouds`); the cloud selection in `examples/v1-game` and `examples/sea` | [Design](CLOUDS/DESIGN.md); [brief](CLOUDS/BRIEF.md) |

State that no module above owns stays where it is: committed world state and the tick page in `thrum-core`
(G1 and G2S), input and replay in `thrum-network` and core, and game content (the valley, presets, hull data,
cloud genus tables, looks) in `examples/` and never in engine crates. The browser shell in `thrum_app::web` was
built before the loop and has [a design](PLATFORM-RUNTIME-WEB/DESIGN.md) and no done-when file; it joins the table
above as `PLATFORM-RUNTIME-WEB` when work on it is next planned.
[Track 3's modules](#modules-and-what-each-owns-track-3) are in their own table.

## Coverage of the V1 exit

| Exit clause | Closed by |
|---|---|
| One engine powers `examples/v1-game` on this M3 Max and its 120 Hz display, and the same scene in Chrome | `SHOWCASE` M7 and M8; `PLATFORM-RUNTIME-MAC` M2 and M3 |
| Physical sky, Sun and aerial perspective with time controls | `ATMOSPHERE` M6 |
| Cooked meshes and textures; hot reload | `CONTENT-LOOP` M3; `CREATOR` |
| Live camera, lights and cascaded shadows | G3a, closed before the loop; shadow quality on the route in `ATMOSPHERE` M6 |
| GPU culling | `CULLING` M3 |
| MetalFX, EDR and a quality governor; no pipeline compilation during play | `APPLE-TILE` M10–M12; `APPLE-PRESENTATION` M4–M11 |
| Display-link interpolation | G3b's function rows, closed before the loop; Mac cadence in `PLATFORM-RUNTIME-MAC` |
| Profiler and native/browser demo UI | `CREATOR` M1 and M2 |
| 10,000 entities and 1,000–2,000 movers as the representative scene | `SHOWCASE` M5 and M6, with `CULLING` and `CONTENT-LOOP` |
| Headless corrected-run replay | G2S, closed before the loop |
| A fresh agent builds a small scene from public docs alone | `CREATOR` M3 |
| Outstanding physical-device runs reported | `SHOWCASE` M8 |

## Build order

Riskiest and most depended-on first. A milestone's `needs` in its done-when file is the authority; this is the
reasoning behind those entries. The project settings allow one Codex builder, so the lanes below say what is safe to run beside `APPLE-TILE` if the owner allows a second builder, and what a lone builder picks up while `APPLE-TILE` or the Mac qualification waits on an attended session.

1. **First.** `PLATFORM-RUNTIME-MAC` M2. `APPLE-TILE` M0, then M1–M3. `APPLE-TILE` is first because everything
   after it reads its graph, its recipe selection or its pre-tone input.
2. **Beside them, in other files.** `CREATOR` M1 (`thrum-app`, `thrum-controls`, `examples/starter`, docs).
   `ATMOSPHERE` M5 (`thrum-render/src/atmosphere`, the forward light group, the contribution seam).
3. **After `APPLE-TILE:M0`.** `APPLE-PRESENTATION` M1 (jitter, matrices and motion), which runs on the current
   portable output and needs only M0's proof tools.
4. **After `APPLE-TILE:M2`.** `APPLE-PRESENTATION` M2 (temporal inputs and history; it shares `runtime.rs`).
   `CULLING` M1 (Occlusion on mixed meshes) and `EFFECT-GRAPH` M1 (3D, cube and mipped-array textures in the graph),
   which both edit the graph and scene files M2 moves. `BASELINE-GUARD` M1 waits on nothing and is built first of
   these. `CULLING` M1 and `EFFECT-GRAPH` M1 do not wait for it, by an assumption in each plan; their later
   milestones do, so the proof gate is in place before most of their renderer changes land.
5. **After `APPLE-TILE:M3`.** `APPLE-PRESENTATION` M3 (TAAU in the game). `PLATFORM-RUNTIME-MAC` M3. `CREATOR` M2
   (the profiler), which reads pass-name timing and adds the timing-demand switch in `runtime.rs`; it merges one
   after the other with the two beside it.
6. **After TAAU.** `ATMOSPHERE` M6, because antialiasing changes every edge and gradient the sky review judges.
   `APPLE-PRESENTATION` M4–M7 (extent, scale, EDR, governor). `APPLE-TILE` M10–M11, then `APPLE-PRESENTATION` M8
   (MetalFX), M9 and M10. `CONTENT-LOOP` M3 and `CREATOR` M3 close at any point here. `CONTENT-LOOP` M1
   (leases) starts earlier, after `APPLE-PRESENTATION:M2`, and its M2 (cooked mesh rows) after `CULLING:M3`.
7. **The exit.** `SHOWCASE` M5 and M6 (the populated valley and its culling) start once `CONTENT-LOOP:M2` is
   Done, beside the work of step 6. `APPLE-TILE` M12 and `APPLE-PRESENTATION` M11 (paired proof) come next, then
   `SHOWCASE` M7 and M8.
8. **Track 2.** `SEA` and `CLOUDS` each start with a CPU authority and oracle milestone that needs nothing from the
   renderer. Their GPU milestones start where [the readiness conditions](#where-the-ports-readiness-conditions-are-met)
   are met. Their closing milestones need the governor. `SEA` M1 (the CPU model) and M2 (GPU state, on
   `EFFECT-GRAPH:M2`) can run beside track 1 in files of their own; its first frame, M3, follows `EFFECT-GRAPH:M3`.
   `CLOUDS` M1 (the CPU weather) and M2 (shade with no cloud drawn, on `EFFECT-GRAPH:M2` and `ATMOSPHERE:M5`) can
   run the same way; its first frame, M3, follows `EFFECT-GRAPH:M3`.

**The order in which the remaining modules are planned**, each a `plan-module` run from its brief below:
`APPLE-PRESENTATION` M4–M11 (migration of [the drafted milestones](../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md)).
`CONTENT-LOOP`, `SHOWCASE`, `SEA` and `CLOUDS` are planned. Each waits on `APPLE-TILE`, on a milestone
that itself waits on `APPLE-TILE`, or on the owner's port timing, so each is planned before a builder could reach it.

## Where the port's readiness conditions are met

[The port decision](../design/decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november) lists five conditions. Each is met at
a named milestone.

| Condition (6 November) | Met by |
|---|---|
| G3s: one shared sky, Sun, light-transmittance and environment-reflection products | `ATMOSPHERE:M5` (identity, lent products, the one cloud optical-depth path) |
| Graph: 3D and cube resources, cadence; async-compute eligibility as a declared seam | `EFFECT-GRAPH:M3` (kinds in M1, state and cadence in M2, the stage after opaque and the declared seam in M3) |
| Frame and V4: effects supply motion, reactivity and reset epochs; reconstruction | `APPLE-PRESENTATION:M2` (third forward target, history protocol) and `M3` (TAAU) |
| G5: cook and map 3D-volume products; cloud volumes loaded | `CONTENT-LOOP:M1` (leased density volumes, R8 portable and BC4 desktop) |
| Recipes and governor: resource and quality selection, history-safe cadence | `APPLE-TILE:M1` (recipe selection), `APPLE-PRESENTATION:M7` (governor), `M8` for the Apple reconstructor |

So a first sea or cloud frame can be built in Thrum once `APPLE-TILE:M3`, `APPLE-PRESENTATION:M3`,
`ATMOSPHERE:M5` and `EFFECT-GRAPH:M3` are Done. That point falls at step 5–6 of the build order, before G7. The
ports close after `APPLE-PRESENTATION:M7`. Demo scenes in Chrome also need `CREATOR:M1`.

`SEA`'s first frame, its M3, waits only on `EFFECT-GRAPH:M3`, which itself waits on `APPLE-TILE:M3`. How the sea
looks under TAAU is its M9, after `APPLE-PRESENTATION:M3`. `CLOUDS` is the same: its first frame is its M3, on
`EFFECT-GRAPH:M3`, and its look under TAAU is its M9.

`EFFECT-GRAPH:M3` proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs. How
an effect looks after reconstruction (shimmer, ghosting, a cut) is accepted in `SEA`'s and `CLOUDS`' own
milestones, on `APPLE-PRESENTATION:M3`'s reconstructor.

## Interfaces

Existing contracts are named by number. New interfaces are named here and defined in the design of the module
that provides them; they are provisional until that module's design review passes. A module does not change an
interface by itself: a builder writes a proposal in its module's log and a planner settles it here.

| Interface | Provider → consumers | Defined in |
|---|---|---|
| Tick page, committed and previous endpoints, tick-derived time | core → every simulated effect | C02 |
| Capability registry, recipes, quality values | `APPLE-TILE` (selection), `APPLE-PRESENTATION` (governor) → all GPU modules | C04, [hardware-native decision](../design/decisions/HARDWARE-NATIVE-ENGINE.md) |
| Graph declaration, GPU lifetime, device generations | `APPLE-TILE` → all GPU modules | C06, [graph decision](../design/decisions/G3-EXECUTING-RENDER-GRAPH.md) |
| **Effect passes and resources**: a contribution declares its own compute and render passes at five named frame stages, with 3D, cube and mipped storage-array textures, persistent, ping-pong and world-anchored wrapping state stepped by the tick and committed only with a submitted frame, a runtime cadence, and quality knobs. A scene holds an ordered list of contributions | `EFFECT-GRAPH` → `SEA`, `CLOUDS`, `ATMOSPHERE`; `RENDER-2D` later | [EFFECT-GRAPH](EFFECT-GRAPH/DESIGN.md#mechanisms) |
| **Entity visibility**: an entity drawn through the instance field is culled per instance and draws a whole mesh row from the scene's mesh selection; a cooked placement is culled per meshlet; nothing is in both. Cooked meshes reach entities as mesh rows, and an edited mesh replaces its row's geometry while the game runs, switching in the frame the content publication names | `CULLING` (the route and its culling), `CONTENT-LOOP:M2` (cooked meshes as rows, by the ruling below) → `SHOWCASE` | [CULLING](CULLING/DESIGN.md#one-visibility-authority-per-entity), [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#cooked-mesh-rows-m2) |
| Pre-tone scene-linear input with matching exposure, extent and sample identity | `APPLE-TILE:M3` → `APPLE-PRESENTATION` | [APPLE-TILE](APPLE-TILE/DESIGN.md) |
| **Temporal contribution**: every drawer writes motion from unjittered positions, marks invalid history, writes the reactive target, and honours the frame's reset flag | `APPLE-PRESENTATION` M1–M2 → `ATMOSPHERE`, `CONTENT-LOOP`, `CULLING`, `SEA`, `CLOUDS` | C07, [APPLE-PRESENTATION](APPLE-PRESENTATION/DESIGN.md#seams-for-thrum-core-proposals) seams 1–4 |
| **Environment identity and products**: source epoch and time, the true Sun (direction, raw illuminance, radius), and read-only sky, transmittance, aerial and reflection products with their WGSL lookups | `ATMOSPHERE:M5` → forward contributors, `SEA`, `CLOUDS` | [ATMOSPHERE](ATMOSPHERE/DESIGN.md#environment-identity-and-products), C08 |
| **Sun-path optical depth**: one provider's `tau` toward the Sun; V3 applies `exp(-tau)` once on each direct-Sun receiver. The field is CPU bytes or, by [ruling 1](#seam-rulings-for-clouds), a texture the provider writes on the GPU, with a far value for receivers below its lowest Sun elevation or outside its region | `CLOUDS` from its M2 (a diagnostic stand-in in `ATMOSPHERE:M5`) → `ATMOSPHERE` receivers, `SEA` | C08, [ENVIRONMENT](../design/modules/ENVIRONMENT.md), [ATMOSPHERE](ATMOSPHERE/DESIGN.md#environment-identity-and-products), [CLOUDS](CLOUDS/DESIGN.md#the-suns-path-one-optical-depth-product) |
| **Sky overlay and haze scale**: a cube of cloud `(S,T)` around the camera, which `ATMOSPHERE` composes over the sky before it filters its reflection and diffuse products, and one number a frame that scales the air's solar in-scatter. So ambient light, reflections and haze follow the clouds, and the sea reflects them through the environment reflection product | `CLOUDS:M8` → `ATMOSPHERE`'s products, and through them the forward pass and `SEA` | C08, [CLOUDS](CLOUDS/DESIGN.md#the-sky-the-world-sees), [rulings 3 and 4](#seam-rulings-for-clouds) |
| Ordered medium segments `(T,S)` and the reflection provider. The cloud segment never starts below its profile's lowest base and is composed with the air once ([CLOUDS](CLOUDS/DESIGN.md#the-visible-layer)). The sea composes its own air, surface and water segments in its one pass and **writes scene depth**, so a medium drawn after it stops at the water; `SEA` precedes `CLOUDS` in a scene's contribution list | `SEA`, `CLOUDS` → V3 composition in `ATMOSPHERE` and the forward pass | C08, [SEA](SEA/DESIGN.md#optics) |
| Cooked products and placements | `CONTENT-LOOP` → games, `CREATOR`'s sky kit | C05, [G5 decision](../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md) |
| **Content publication**: one cooked generation changes through every reader a scene has (the placement set, each lease, each cooked mesh row) in one submitted frame. The scene's residency owns it: each reader prepares under its own bound without showing, the publication is ready when the slowest reader is, a ready publication is not superseded, and one a reader cannot take is refused for all. This is C05's qualified visual preview for cooked products; C05 is unchanged | `CONTENT-LOOP:M1` (placements and leases), `CONTENT-LOOP:M2` (rows) → `CULLING`'s route, lease consumers (`SEA`, `CLOUDS`), games | C05, [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) |
| **Leases**: an engine module names a set of cooked textures and volumes; the scene's one residency uploads them and lends read-only views, as a whole set, only after the frame that recorded the last copy is submitted, in the frame the content publication switches placements and rows. Each product carries identity and a change serial; a consumer keeps a change pending until a submitted frame that advances history has used it. Placement and meaning are the consumer's inputs | `CONTENT-LOOP:M1` → `CLOUDS` (density volumes), `SEA` (foam texture); `RENDER-2D` and `GAME-UI` later | [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#leases-m1) |
| **Body query**: one batched asynchronous GPU query, one to two ticks late, with support and age. The game posts points with a footprint, a use tick and a body ID and generation in its tick page, and drains answers (height, slope, surface velocity, time derivative, evaluation tick, a support flag per layer) from a mailbox; nothing waits on the GPU, and a request over capacity is a counted refusal | `SEA` → games and P3 | [E1 water decision](../design/decisions/E1-GPU-WATER.md), C09, [SEA](SEA/DESIGN.md#bodies-and-the-query) |
| **Quality knobs**: each cadence and step budget is a runtime value with a declared history invalidation. None changes a physical profile or what a query returns | `SEA`, `CLOUDS`, `ATMOSPHERE` → the governor in `APPLE-PRESENTATION:M7` | C04, port decision, [SEA](SEA/DESIGN.md#recipes-limits-and-knobs) |
| Typed controls, parameter files, the scene shell on Mac and web | `CREATOR` → `SHOWCASE`, demo scenes for `SEA` and `CLOUDS` | C11, [G6 decision](../design/decisions/G6-CREATOR-EXPERIENCE.md), [CREATOR](CREATOR/DESIGN.md) |
| **Frame timing feed**: completed frame samples with pass-name GPU timing, and one timing-demand switch on the frame host (on by default; off writes no timestamps and the sample says GPU time was not requested) | The frame runtime (`APPLE-TILE:M3` for pass names; the switch by the ruling below) → `CREATOR`'s profiler, frames reports, Mac qualification, capture | C12, [CREATOR](CREATOR/DESIGN.md#the-frame-timing-feed) |
| Engine capture modes and evidence | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE/DESIGN.md) |
| **Resolved selections**: each selectable path tells the game what it resolved to on this device (recipe, reconstructor, display mode, governor state, culling mode), so a run can report what it ran and a fallback cannot pass as the feature | `APPLE-TILE`, `APPLE-PRESENTATION`, `CULLING` → `SHOWCASE`'s exit report | C04, [SHOWCASE](SHOWCASE/DESIGN.md#the-exit-configuration-one-file-three-rows-reported-and-not-echoed) |
| **Exact baselines**: retained references, their owner, the one-command verifier, and the proof gate in `./dev check` that `land` runs before it pushes | Each baseline's owner (references), `BASELINE-GUARD` (registry, verifier, gate) → every module that edits the covered paths | [Exact baselines](#exact-baselines) |

**Seam rulings for `APPLE-PRESENTATION` M1–M3.** Its design proposed nine seams to Thrum Core. Under the loop the
module list settles them. Seams 1–4 are accepted as written there:
- seam 1 (submission-order history acceptance, `history.rs` and `runtime.rs`) is built in M2, after
  `APPLE-TILE:M2` has moved `runtime.rs`;
- seam 2 (the view-record change in RENDER-SPEC) is built in M1, and the builder refreshes the design hashes;
- seam 3 (the third forward target) is built in M2 across `contribution.rs`, `thrum-asset-gpu` and the sky
  pipeline, keeping COOKED-DRAW's path selection and history rules;
- seam 4 (sky and star motion at infinity) is built in M1 by the `APPLE-PRESENTATION` builder inside the
  atmosphere shaders, for motion output only.

Seams 5–9 are settled when M4–M11 are migrated.

**Seam ruling for `CREATOR` M2.** The profiler needs the frame host to stop timing frames while it is off, which
no interface offered. The timing-demand switch and the "not requested" reason are built in `thrum-render`'s
runtime and timing files by the `CREATOR` M2 builder, after `APPLE-TILE:M3` is Done, with the behaviour
[CREATOR](CREATOR/DESIGN.md#the-frame-timing-feed) defines. `APPLE-TILE`'s items do not change. Once built, the switch
belongs to the frame runtime: later work there (`APPLE-TILE` M10–M12, `APPLE-PRESENTATION`, `EFFECT-GRAPH`) keeps
its tests passing, and any timing it adds, a native segment's included, is not gathered while the switch is off.

**Seam ruling for `CONTENT-LOOP` M2.** Replacing a mesh row's geometry in a prepared scene needs a change to the
mesh arena and the bucket route, which `CULLING` owns and which no interface offered. It is built in
`thrum-render`'s `synthetic/{mesh_arena,mesh_route,camera_culling,shadow}.rs` and the frame host by the
`CONTENT-LOOP` M2 builder, after `CULLING:M3` is Done, with the behaviour
[CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#cooked-mesh-rows-m2) defines. The route writes a replacement, reports the
write done, and switches the row only in the frame residency's
[publication](CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) names; `thrum-render` gains no dependency
on `thrum-asset-gpu`. `CULLING`'s items and its exact comparisons do not change. Once built, row replacement
belongs to the route: later work there keeps its tests passing.

### Seam rulings for SEA

The sea draws after opaque geometry, and two things it needs there are not in `EFFECT-GRAPH`'s after-opaque
stage as designed. Both are built in `contribution.rs` and `synthetic/{contributed,scene_graph}.rs` by the `SEA`
builder, after `EFFECT-GRAPH:M3` is Done, with the behaviour [SEA](SEA/DESIGN.md#the-surface-and-the-frame-contract)
defines. `EFFECT-GRAPH`'s items do not change and its tests keep passing: a scene with no effect that uses them
compiles to the same passes and storage as before. If M3 as built already allows either, nothing is added. A
third ruling covers the sea's example scene, the first game after the V1 game to run on the shell with a world
that ticks.

1. **An after-opaque pass may write scene depth** (`SEA` M3). The stage let an effect read depth and write
   colour, motion and reactive coverage. A sea that left depth alone would hand reconstruction the depth of
   whatever lies behind the water, and a cloud march drawn later would run through it. Stored depth still follows
   the union of its readers.
2. **An effect's own pass may read scene colour and scene depth as textures and write only that effect's
   targets** (`SEA` M8). The sea gathers the scene behind its surface only inside the screen rectangles of
   bodies, in place of the shared whole-scene refraction copy, which it never requests. Depth cannot be sampled
   in the pass that writes it, so the gather is a pass of its own.

3. **The window, the page and the capture tap for a second ticking game** (`SEA` M3 on the Mac, M4 in Chrome;
   `CREATOR`'s and `PLATFORM-RUNTIME-WEB`'s shell in `thrum-app`, `T2-CAPTURE`'s tap and the scene packaging
   scripts). `examples/sea` is a game on the shell with its own ticking world and the engine's sky. Whatever the
   shell, the page, the packaging or the tap still assume of the V1 game, or of the sky kit's world that never
   ticks, is made a property of the game. The V1 game, the starter and the creator scenes build, run and draw as
   they did, no item of an owning module changes, and no engine crate gains a type or file of the sea scene.
   `CREATOR` M2 and M3 edit the same crate; the `SEA` builder integrates on top of what has landed.
   [`SWARM`'s ruling 2](#track-3-the-swarm-survival-game-after-v1) then covers only what a 2D game with no sky
   still needs.

Once built, rulings 1 and 2 belong to `EFFECT-GRAPH`'s seam and ruling 3 to the shell's owners: later work there
keeps their tests passing, and `CLOUDS` uses the same accesses and adds none.

### Seam rulings for CLOUDS

Clouds shade and light the world through `ATMOSPHERE`, and four things they need are not in its lent interface
as `ATMOSPHERE:M5` built it. Each is built in `thrum-render`'s atmosphere files by the `CLOUDS` builder, with the
behaviour [CLOUDS](CLOUDS/DESIGN.md#what-clouds-asks-of-other-modules) defines. `ATMOSPHERE`'s items, tests,
bounds and references do not change and keep passing. A scene with no cloud provider composes the same shader
text and draws the same bytes as today, which is that module's cue E3. `ATMOSPHERE` M6 edits the same files;
whichever builder lands second integrates on top. A fifth ruling covers the two games clouds are shown in.

1. **A provider may write the Sun-path optical depth on the GPU** (`CLOUDS` M2). M5's field is CPU bytes with a
   rigid drift. A provider may instead register a 2D texture that its own `simulate` pass writes, with its plane,
   region, displacement, lowest Sun elevation and validity given each frame, and one `tau_far`. A receiver below
   the lowest elevation or outside the region gets `exp(-tau_far)`. A provider that sets none has `tau_far` zero,
   and its receivers get exactly 1 there, as now. A receiver at or above the plane, and an invalid frame, still
   get exactly 1.
2. **Aerial transport to a far sky point** (`CLOUDS` M3). The lent aerial operation stops at 32 km. For a point
   beyond it on a ray that ends in the sky, it returns the transmittance chart's ratio between the two ends, and
   an in-scatter that is the sky's radiance in that direction apportioned by optical depth: a stated
   approximation, continuous at 32 km and exact at infinity. At 32 km and nearer the operation is unchanged.
3. **A sky overlay in the reflection and diffuse products** (`CLOUDS` M8). A provider may register a small cube
   of `(S,T)` in physical units. The reflection and diffuse producers then filter a sky whose radiance in each
   direction is `T·L + S`. The lent sky radiance stays cloud-free, and the provider lights itself from that. If
   the incident-light meter has a Sun term without the cloud factor, it gains it at its metering point.
4. **A haze scale** (`CLOUDS` M8). A provider may give one number a frame that multiplies the solar part of the
   aerial in-scatter. Transmittance and the night term are untouched. It is the stated approximation
   `ATMOSPHERE`'s design left to `CLOUDS` for in-scatter under cloud: no shafts and no colour shift.
5. **A cloud selection in two games** (`CLOUDS` M2 in `SHOWCASE`'s `examples/v1-game`, M11 in `SEA`'s
   `examples/sea`). Each game gains a selection that is off by default. With it off, the game's world, tick page,
   compiled graph and frames are what they were, and no item of the owning module changes. With it on, the game
   keeps a weather state in its world, adds the cloud contribution after any sea, and the labelled stand-in
   shade provider is not used. `CLOUDS` does not edit the showcase's generator, routes or exit configuration.
   Putting clouds in the V1 exit scene by default would change the exit, which is the owner's.

Once built, rulings 1 to 4 belong to `ATMOSPHERE`: later work there keeps their tests passing. `CONTENT-LOOP`'s
anticipated cook-time atlas is not asked for: a game stacks its shapes into one `TVOL` source, whose slabs keep
their mips apart.

Track 3's interfaces are in [their own table](#interfaces-track-3).

## Exact baselines

An **exact baseline** is a set of retained bytes that a proof compares rendered output with, byte for byte. Each
has one owner. On 11 November `APPLE-PRESENTATION` M1 applied jitter to every game view, which moved every
rasterised pixel of `APPLE-TILE`'s M0 baseline. The builder changed two exact tests in `./dev check` to absorb it,
and the change merged. `APPLE-TILE` M2 was then claimed on evidence from a binary built before its rebase
([APPLE-TILE M2 review](../evidence/APPLE-TILE/M2-review.md), MF1;
[APPLE-PRESENTATION M1 review](../evidence/APPLE-PRESENTATION/M1-review-1.md), MF1). Three rules close those holes.

**1. An exact comparison is its owner's interface.** A builder never changes another module's retained
references, its comparison, its tolerance or its sampling (accepting the closest of several frames, for example),
nor an exact test of output another module owns, to make its own change pass. The registry, the verifier and the
gate of rule 2 are `BASELINE-GUARD`'s in the same way. When its change fails one, it stops, writes the proposal in
its log and ends `BLOCKED`; a planner settles it here with the owner, as for any interface. A change that needs
different output is built behind a selection whose default leaves every registered image exact, which is how MF1
was fixed: jitter is gated on explicit temporal sampling. When a milestone is meant to change the default output,
its planner names here, before it is built, which baselines move and which milestone of the owner refreshes them.
A refresh records the cause and the old and new hashes in both modules' logs. Nobody refreshes a baseline
silently. A loop builder pushes a covered-path change only through `land` with its check on, never
`land --no-check`. Any other thread that pushes one runs `./dev check` on the exact commit it pushes.

**2. Before the push: a proof for exactly the content being pushed.** `land` rebases, runs `./dev check` and
pushes under one lock, without handing control back in between, so `./dev check` is the only thing between any
change and main, a fix with no plan included. A full comparison needs a release build, a cook and Chrome, which
`./dev check` cannot run. So [BASELINE-GUARD](BASELINE-GUARD/DESIGN.md) builds two parts:

- a **verifier**, one command that draws fresh output on the Mac and in Chrome at a commit and compares it with
  every registered baseline. On a full pass it writes a **proof** that names a digest of the covered paths at that
  commit;
- a **gate** in `./dev check`, which passes a commit that changes no covered path beyond main, and refuses any
  other commit unless a proof names its exact covered content.

A builder runs the verifier before `land`. If `land`'s rebase brings in another covered-path change, the content
and its digest change, the gate fails and nothing is pushed; the builder runs the verifier on the rebased commit
and lands again. So the full matrix, Chrome and night included, has always passed on the content that reaches
main. Every module's landings are covered from the day the gate exists, with no item added to any plan. Until
`BASELINE-GUARD` M1 is Done, rule 3 is the only automatic catch, and it acts after the push.

**3. At the claim: the verifier again, as a planner-written check.** A builder cannot change an item's `check`,
and the loop reruns every `check` on a clean copy of main before it records a milestone. So each milestone that
edits covered paths, in a module planned from now on, carries an item whose `check` is the verifier. It is the
backstop for a skipped gate or a forged proof. Every exact or visual proof records the revision, clean-diff hash
and binary hash it ran on, and that revision is the commit claimed. A reviewer who finds a baseline, the gate or
an exact test changed by a module that does not own it, with no ruling here, records a Must Fix.

| Baseline | Owner | What it pins | Compared by | Covered paths |
|---|---|---|---|---|
| `docs/evidence/render/apple-tile-m0/native-baseline/` | `APPLE-TILE` | On the Mac, for both recipes: submitted SDR and scene-linear bytes at the moving route's checkpoints, Sun and night stills at 1080p, and letterbox bytes in two surface formats | `scripts/apple_tile_m1.py` and `scripts/apple_tile_m2.py`, on a release build of the commit and a fresh cook of the showcase valley | `crates/`, `examples/v1-game/`, `scripts/`, `tools/`, `.cargo/`, `.config/exact-baselines.json`, the workspace manifest, lockfile and toolchain file, and all registered retained files |
| Four Chrome manifests: `docs/evidence/render/apple-tile-m0/chrome-baseline/prechange-chrome-fixed-manifest.json.gz` (day, submitted SDR), and under `docs/evidence/render/apple-tile-m1/chrome/` the `manifest.json.gz` of `chrome-prechange-linear-3` (day, scene-linear), `chrome-prechange-night-sdr` and `chrome-prechange-night-linear` | `APPLE-TILE` | Chrome's portable output on the moving route: for each of 481 records, the identity fields and the SHA-256 of the frame's decoded bytes | A fresh Wasm build and `fixed` capture of the commit, compared record by record with the retained digests by `scripts/apple_tile_m1_compare_browser.py` | The same |
| `docs/evidence/v1-workload/g7-showcase-baseline.json` | `SHOWCASE` | The six route stops on both targets, within thresholds, not exactly. Not in the verifier | `python3 scripts/run_v1_showcase.py check` | The same |
| The exit baselines under `docs/evidence/SHOWCASE/`, created by `SHOWCASE` M5 and extended to one per row in M7 | `SHOWCASE` | The six stops of the exit scene, within thresholds, not exactly. Not in the verifier | `python3 scripts/run_v1_showcase.py check --exit` | The same |

The verifier, `python3 scripts/exact_baselines.py verify`, runs the first two rows; it exists once
`BASELINE-GUARD` M1 is Done.

**Ruling on the Chrome reference (with `APPLE-TILE`).** The complete reference captures, 481 frames each, were
kept only in the scratch of the builder that made them, so a clean checkout could not run the byte comparison.
The retained manifests are the reference: each holds every record's identity fields and the SHA-256 of its decoded
bytes. A fresh frame whose decoded bytes hash to the retained digest is taken as byte-equal. `BASELINE-GUARD` M1
adds this reference-by-digest mode to `scripts/apple_tile_m1_compare_browser.py`; the script's byte mode, the
retained files and `APPLE-TILE`'s items do not change. If a set does not match on untouched main, the builder
ends `BLOCKED` and `APPLE-TILE` refreshes it under rule 1.

Exact tests that hold no retained references (a culled mode against Direct, the cooked direct path against its
reference, paused frames against each other) come under rule 1: they belong to the module that owns the output.

`CULLING` M1 and `EFFECT-GRAPH` M1 may be built before the gate exists. Each carries the native matrix as its
rule-3 item, and its builder reruns that item after every rebase and before `land`; Chrome is first rechecked
when `BASELINE-GUARD`'s builder runs the verifier on main. Their later milestones wait for `BASELINE-GUARD` M1 and
carry the verifier. Milestones planned before this rule (`APPLE-PRESENTATION` M2 and M3, `ATMOSPHERE` M6, `CREATOR`
M2, `APPLE-TILE` M3) need no new item: the gate covers their landings once it exists. `SHOWCASE` moves no exact
baseline: its exit scene is new content beside the grid and the field, and the valley's source folder and cooked
root stay as they are ([design](SHOWCASE/DESIGN.md#exact-baselines)). `APPLE-PRESENTATION` M3 puts
TAAU in the game: if that changes the default output, its planner settles the refresh under rule 1 before it is
built.


## Where sea and clouds are finished

**Decision: both are finished as Thrum modules. No further finishing happens in Broadsides.** The Broadsides
branches are frozen references at the commits the briefs pin. Reasons:

1. **Cost.** Thrum's bar is the display's full refresh. Broadsides presents the stock sea at about 21 frames a
   second at 1440p, with or without clouds, and its sea pass alone takes about 21 ms of GPU time. Its fair-weather
   cloud preset added about 289 ms a frame in a paired run on 6 November. Looks finished at those costs are
   references, not products
   ([performance rules](../../.agents/skills/thrum-engineering/references/performance.md)). Shipped engines
   reach comparable cues in a few milliseconds, so the mechanisms have to be re-costed before they are lifted.
2. **Light.** G3s is the only sky. Broadsides clouds and water are lit by their own sky, exposure and temporal
   path, and none of that is ported. Every visual verdict passed there has to be earned again under Thrum's light,
   reconstruction and display.
3. **Plumbing.** The port decision already says the Broadsides pass lists, target pools, frame packets and host
   glue are rewritten. The open Broadsides work is mostly in that plumbing: the baked path is paused on a reader
   defect in its atlas march, and the water branch's unfinished waterline assembly on a build error in its render
   module.
4. **Scope.** The open Broadsides milestones serve a naval game: spray leaving a planing hull at an 18 m chase,
   a rising gas cloud at the waterline, ten cloud genera, a squall and a warm front. Thrum's purpose is teaching
   demos and a reusable engine. Three spikes on 10 November tried the hardest water cues again and none reads at
   the shipped-game bar yet.
5. **What does transfer is already finished enough to lift.** The CPU maths and its tests, the GPU kernels, the
   accepted sea baseline, the first two water milestones, the accepted cloud library of twelve volumes and its
   growth track, and the weather field. The briefs list each with its commit.

**What finished means** is defined in each brief as a short list of cues with ranges, judged in a Thrum scene on
`mac-metal` and `chrome-webgpu`: [SEA](SEA/BRIEF.md#what-finished-means), [CLOUDS](CLOUDS/BRIEF.md#what-finished-means).
In one line each:
- **Sea:** an open sea from waterline to horizon across calm to gale, bodies that float on it through one
  asynchronous query, a wake that persists and moves another body, transported foam from breaking waves and hull
  contact, and sea optics through the shared light contract, at full refresh with governor knobs.
- **Clouds:** one provider that draws a weather-driven sky from clear to overcast and placed cloud volumes from
  the baked library with the same march and light, shades the world through one optical depth, lights correctly
  from noon to sunset under G3s, and runs shadow-only, at full refresh with governor knobs.

**Consolidation** means one module per provider. The two cloud paths become one `CLOUDS` module in the shape the
port decision already selected: the deterministic weather field is the authority and draws layers, and the baked
library supplies placed volumes under the same optical contract.

## Module briefs

Briefs for the modules not yet planned. Modules with a design are linked from the table above.

### APPLE-PRESENTATION M4–M11 (G4)

Migrate [the drafted milestones](../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md) into the done-when file as written: extent
and generation transitions, dynamic scale, EDR, the governor, MetalFX, launch-to-ready, offline metallibs and the
paired proof. Settle seams 5–9 here with the frame-runtime, platform and asset owners. `needs`: M4 and M6 on
`APPLE-TILE:M3`, M8 on `APPLE-TILE:M11`, M11 on `APPLE-TILE:M12`. The governor (M7) must accept the quality-knob
interface above, so `SEA` and `CLOUDS` can register knobs without a second governor. Research depth: none new;
restate the sizing after M3's first measurements. The same run adds `APPLE-PRESENTATION:M11` to the `depends_on`
of `SHOWCASE` M7, which until then lists `APPLE-PRESENTATION:M3` and carries a check of the record files that fails
while M11 is missing. M4 to M11 also give the game the resolved reconstructor, display mode and governor state, for the
resolved-selections interface above. The same run adds the governor's and MetalFX's milestones (M7 and M8 in the
drafts) to the `depends_on` of `SEA` M10, which until then carries a check of the record file that fails while
either is missing; if the migration renumbers them, `SEA` M10.1's check is rewritten with it. `CLOUDS` M12 and
its M12.1 are treated the same way.

### CULLING (G3c)

Planned on 11 November 2026: [design](CULLING/DESIGN.md), [plan](CULLING/PLAN.md). Its planner settled the open
decision this brief carried: [each entity has one visibility authority](CULLING/DESIGN.md#one-visibility-authority-per-entity).
G3c closes on the scenes that exist, the mixed-mesh field and the valley as it stands, so it does not wait for
`SHOWCASE`.

### CONTENT-LOOP (G5)

Planned on 13 November 2026: [design](CONTENT-LOOP/DESIGN.md), [plan](CONTENT-LOOP/PLAN.md). The first sea or
cloud frame that samples a cooked product needs its M1; `SHOWCASE` needs its last milestone, M3. Parked and
staying parked: CAS garbage collection and leases on the store, layered EAC, parameter files as a product kind.

### SHOWCASE (G7)

Planned on 13 November 2026: [design](SHOWCASE/DESIGN.md), [plan](SHOWCASE/PLAN.md). Its milestones are M5 to M8;
M1 to M4 were the route slice, built before the loop under
[the route decision](../design/decisions/G7-SHOWCASE-ROUTE.md) and kept in
[its record](../evidence/SHOWCASE/ROUTE-SLICE-RECORD.md). M5 and M6 need `CONTENT-LOOP:M2` and `CULLING:M3`; M7 and
M8 need the last milestone of every other track 1 module. Track 3 starts when M8 is Done.

### EFFECT-GRAPH (track 2 foundation)

Planned on 11 November 2026: [design](EFFECT-GRAPH/DESIGN.md), [plan](EFFECT-GRAPH/PLAN.md). The first sea or
cloud frame needs its last milestone, M3.

### SEA and CLOUDS

`SEA` was planned on 13 November 2026: [design](SEA/DESIGN.md), [plan](SEA/PLAN.md), from [its brief](SEA/BRIEF.md).
`CLOUDS` was planned the same day: [design](CLOUDS/DESIGN.md), [plan](CLOUDS/PLAN.md), from
[its brief](CLOUDS/BRIEF.md). It takes from `SEA`'s design that the sea writes scene depth and comes first in a
scene's contribution list, and it puts clouds into the environment reflection product the sea reads.

## Track 3: the swarm-survival game, after V1

Added 10 November 2026. [The brief](../design/NEXT-GOAL-SWARM-GAME.md) is the owner's page; this section turns it into
modules. Nothing here starts before the V1 exit, changes a V1 module's plan or gates the exit.

### The owner's request (track 3)

Quoted word for word from the 10 November planning task:

> Add the goal after V1 to the module list in docs/design/modules/ARCHITECTURE.md: the original swarm-survival
> test game in docs/design/NEXT-GOAL-SWARM-GAME.md, with the 2D support and online multiplayer it needs. Done when
> ARCHITECTURE.md names the new modules (2D rendering, multiplayer/networking, the game and anything else the brief
> needs), their owners, interfaces with existing modules, build order after the V1 exit, and a brief for each; the
> BUILD-PLAN 'After V1' section points at them. Don't change V1 modules' plans or the V1 exit; list any decision
> that belongs to the owner. No done-when files yet: this work starts after V1.

And from the brief, what the owner asked for:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

The brief's three defaults are kept as written: cooperative online play for 2 to 4 players; top-down 2D with
sprites on Mac, Chrome and the iOS simulator; thousands of enemies and projectiles on screen at once.

### What the game needs from the engine

One run of the game, walked across the engine, gives the modules.

| The run needs | The engine has | So track 3 adds |
|---|---|---|
| Thousands of bodies that chase, crowd each other and get hit, every tick | The registered simulation, with rewind and replay proven at 10,000 entities (G2S). No spatial queries | `SIM-2D` |
| The same crowd on two to four machines | Headless multiplayer fitness, and a parked wire design sized for 128 visible actors a peer | `SIM-2D` and `SESSION` |
| Players on separate machines, one of them a browser | No sockets. Transport was parked for after V1 | `TRANSPORT` |
| Thousands of sprites, layered and crisp, with no sky or Sun | A 3D forward scene only | `RENDER-2D` |
| A health bar, a timer, a level-up choice and a join screen on all three targets | Demo panels in AppKit and the DOM; no text drawn in the frame, nothing on iOS | `GAME-UI` |
| The game: its name, rules, characters and art | `examples/v1-game`, which is a different game | `SWARM` |

### Mechanisms settled here

Three choices decide where the lines between modules fall, so they are settled at this level. Each is provisional
until the design review of the module that owns it passes.

**1. The crowd is shared by causes, not by state.** Every peer steps the same registered rules from the same
baseline and the same admitted input frames. The host stays the only authority (C13): it admits inputs, commits
ticks and sends each peer the frames it admitted. In steady play no per-enemy state travels. Each peer sends a
digest of its committed state, and a peer whose digest differs is repaired with a full image.

- *Why not replicate the crowd.* The parked wire design packs a moving actor into about 18 bytes and proposes 64
  kbit/s down to each client at 20 Hz ([A0](../design/decisions/A0-N1-TRANSPORT-CODEC.md),
  [A5](../design/decisions/A5-STATE-DELTA-SESSION.md)). A 2D pose could shrink to about 6 bytes. 3,000 movers × 6 bytes ×
  20 Hz is 2.9 Mbit/s for each client, 45 times that target, and a crowd that chases the players changes every
  tick, so deltas against a baseline save little. Vampire Survivors' own online co-op met the same wall with
  hundreds of enemies: each client simulates the crowd locally and takes position corrections at 1 Hz, because
  full sync for each enemy "likely wouldn't scale"
  ([coherence case study](https://coherence.io/blog/tradecraft/vampire-survivors-online-coop-case-study)).
- *What travels.* An input frame is about 31 bytes for each player and tick
  ([input wire](../design/decisions/A5-INPUT-WIRE.md)), which does not grow with the crowd. A peer that joins gets a
  baseline and catches up from the inputs that followed, as Factorio's joining clients do
  ([Friday Facts 149](https://www.factorio.com/blog/post/fff-149)).
- *What it rests on.* The game's rules must give the same bytes on Mac arm64, Chrome's wasm32 and the iOS
  simulator. C13 promises no bit identity across platforms, so the rules earn it: integers or fixed point, or
  floats held to add, subtract, multiply, divide and square root with non-finite values refused; no platform maths
  library; no order taken from hashing, threads or pointer width; random streams keyed by seed, tick and entity.
  `SIM-2D` owns that profile and its first milestone tries to break it on all three targets.
- *What keeps a miss from spreading.* The digest catches a divergence that lasts to the next digest tick, at the
  interval `SESSION` sets, and
  repair is the reliable full image A5 already selects. A client that changes its rules only desyncs itself.
- *Range.* 2 to 4 peers and the brief's thousands. A full-world replay does not fit a frame at that size, so the
  crowd is shown at the confirmed tick and only the player's own character is predicted, which needs no crowd
  replay. [`SESSION`'s planner chose this](SESSION/DESIGN.md#11-the-predicted-controller) from `SIM-2D`'s probe;
  its M4 sets `SIM-2D`'s measured replay beside it.
- *If the profile cannot be held.* The fallback is Vampire Survivors' own method: local simulation with corrections
  through the acknowledged deltas of A5, at a rate the byte budget allows.
- *Precedent in Thrum.* C13 already shares water by causes, and [NETWORKING](../design/modules/NETWORKING.md) leaves room for a game
  to "supply a qualified specialist protocol". The acknowledged-delta design stays selected for games whose state
  must travel. This game uses it for the baseline and for repair.

**2. 2D is its own scene half, not sprites in the 3D forward pass.** A profile selects a 2D scene whose passes run
through the one graph executor and write the same pre-tone scene-linear input. Exposure, tone, EDR, the governor,
present and capture are shared from that point. A 2D game then carries no sky tables, cascades or light clusters
(E02), and a 3D game carries no sprite path. Bevy
([`core_2d`](https://docs.rs/bevy/latest/bevy/core_pipeline/core_2d/index.html)), Godot and Unity's URP each keep
a 2D path separate from 3D on one backend. Drawing sprites inside a lit 3D scene is not part of this goal.

**3. A native process hosts and browsers join.** A browser cannot accept connections, and WebTransport gives it a
client only. So the authority is a headless host or a Mac player who also hosts, both on the one rules path of
C13. Chrome and the iOS simulator join as clients.

<a id="modules-and-what-each-owns-track-3"></a>

### Modules and what each owns (track 3)

The responsibility column names the row of [the architecture brief](../design/ARCHITECTURE-BRIEF.md#proposed-module-map)
that the module builds. The code home is where the planner is expected to put it; the planner confirms it and
registers any new crate in `.config/architecture.toml`. Each module is built by the profile's builder.

| Module | Responsibility | Owns (state and code) | Expected code home |
|---|---|---|---|
| `SIM-2D` | W1 and P3, for crowds | The deterministic rules profile and the check that enforces it; the digest of committed state; the 2D spatial index and its overlap, nearest and neighbour queries; the reference crowd and its retained traces | `thrum-core` for the profile and the digest, a new crate `thrum-sim2d` for the queries, `examples/crowd-2d` for the reference crowd ([design](SIM-2D/DESIGN.md)) |
| `RENDER-2D` | V2 and V3, for 2D | The 2D scene half and its pass; the 2D view record (a centre and a scale in output pixels per world unit); sprite rows extracted from the tick page and given each frame; draw order; sprite sampling and blending; sheet slots and frame tables on the GPU; the crowd view | `thrum-render`, and `examples/crowd-2d-view` for the crowd view ([design](RENDER-2D/DESIGN.md)) |
| `GAME-UI` | U1 | Text and interface drawn in the frame: the interface tree and its layout, focus and scopes, text runs, the paint list; which of the interface and the game gets each key, pointer and touch event. Glyphs are rasterised when a font is cooked, so it keeps no glyph cache | A new crate `thrum-ui`, and the interface scene in `examples/v1-game` ([design](GAME-UI/DESIGN.md)) |
| `TRANSPORT` | N1 adapters, F1 network I/O | Connections and their state: native QUIC, the browser's WebTransport, the in-memory conditioned link, a host's TLS identity and its join ticket. It holds no game state and gives no meaning to a message | A new crate `thrum-transport`, so a game that is not networked links no sockets, and `examples/link-proof` for the proof programs and the relay ([design](TRANSPORT/DESIGN.md)) |
| `SESSION` | N1 | The match, its roster and each connection's epoch; the handshake and the rule-set identity; the input lead and the playout clock; input staging, commands and the set stream; the baseline and join; digest exchange and repair; the predicted controller; retained results | `thrum-network`, and `examples/match-proof` for the proof programs and the scripted players ([design](SESSION/DESIGN.md)) |
| `SWARM` | Game policy and content | The game: its name, premise, characters, art, rules, waves, upgrades and screens; the schemas and rule set it declares to `SESSION`; its sprite and font sources; its traces and their retained digests; the record that closes track 3 | `examples/swarm/`, never an engine crate: the rule package `swarm-rules`, the game `swarm`, the iOS app `swarm-ios` and the proof package `swarm-proof` ([design](SWARM/DESIGN.md)) |

State no module above owns stays where it is: the tick, committed world state, rewind and the tick page in
`thrum-core` (G1 and G2S); cooked products in `CONTENT-LOOP`; the graph, the split and the output half in
`APPLE-TILE` and `APPLE-PRESENTATION`; the demo panels and the app runner in `CREATOR`.

### What track 3 asks of existing modules

Each ask is new work inside state that an existing module owns. It becomes a milestone of that module, numbered
after its V1 milestones and added by that module's planner when track 3 starts, unless a seam ruling below gives
it to the asking module's builder. None is added now, so no V1 plan changes.

| Owner | Ask | For |
|---|---|---|
| `CONTENT-LOOP` | A sprite-sheet product (pages, frames, pivots), cooked, mapped and hot-reloaded like meshes and textures. Built in `RENDER-2D:M4` under [ruling 2](#seam-rulings-for-render-2d) | `RENDER-2D` |
| `CONTENT-LOOP` | A font product: a table of metrics and one sheet for each scale, in the sheet form `RENDER-2D` defines; a publication that waits for the interface's new layout, so text and sheets change in one frame. Built in `GAME-UI:M2` under [ruling 3](#seam-rulings-for-game-ui) | `GAME-UI` |
| `APPLE-TILE` | A game selects the 2D scene half in place of the 3D one, and the output half gains a display-referred transfer. Built in `RENDER-2D:M1` under [ruling 1](#seam-rulings-for-render-2d) | `RENDER-2D` |
| `APPLE-TILE`, with `APPLE-PRESENTATION` | The output half gains an interface stage at output extent, after reconstruction and the display transform, in the 8-bit display image and in the EDR present pass's float target. Built in `GAME-UI:M1` under [ruling 1](#seam-rulings-for-game-ui) | `GAME-UI` |
| `RENDER-2D` | The sprite pass runs a second time in a frame, at the interface stage, with its own sheets and rows. Built in `GAME-UI:M1` under [ruling 2](#seam-rulings-for-game-ui) | `GAME-UI` |
| `CREATOR`, `SHOWCASE` and `PLATFORM-RUNTIME-IOS` | The shells give a game pointer and touch events as well as keys, the display's density, the safe-area insets, text drops and the clipboard; the V1 game gains an interface scene. Built in `GAME-UI` M1 to M5 under [ruling 4](#seam-rulings-for-game-ui) | `GAME-UI` |
| `APPLE-PRESENTATION` | The governor and reconstruction accept a scene that declares no temporal reconstruction, so sprites stay crisp. Built in `RENDER-2D:M1` under [ruling 1](#seam-rulings-for-render-2d) | `RENDER-2D` |
| `CREATOR`, with `PLATFORM-RUNTIME-IOS` and `PLATFORM-RUNTIME-WEB` | An app runner on all three targets for a game that brings no sky kit. The iOS shell lives in `examples/v1-game-ios` today and serves one game. Built in `SWARM:M2` for the window and the page and `SWARM:M6` for iOS, under [rulings 2 and 3](#seam-rulings-for-swarm) | `SWARM` |
| Repository guard | `.config/architecture.toml` names one external game; it must admit a second. Built in `SWARM:M2` under [ruling 1](#seam-rulings-for-swarm) | `SWARM` |
| Core (W1) and `SIM-2D` | A world image: the bytes the digest covers, with a way back into a fresh runtime in another process. Built in `SESSION:M1` under [ruling 1](#seam-rulings-for-session) | `SESSION` |
| `SIM-2D` | The reference crowd's player movement as one function its own rule calls, so a predictor can call it too. Built in `SESSION:M4` under [ruling 2](#seam-rulings-for-session) | `SESSION` |
| Core (F1 and W1), with `CREATOR` | A world lane that commits ticks when the session says, in place of its own clock, for a host's and a client's copy of a match. Built in `SWARM:M7` under [ruling 4](#seam-rulings-for-swarm) | `SWARM` |

`RENDER-2D`'s own sprite pass is declared in the one graph by `thrum-render`, as the 3D scene's forward pass is.
An engine module outside `thrum-render` attaches to a 2D scene through `EFFECT-GRAPH`'s seam, at the upload and
simulate stages; a 2D scene has no shadow, opaque or after-opaque stage. A second way to declare passes is not
built.

<a id="seam-rulings-for-render-2d"></a>

**Seam rulings for `RENDER-2D`.** `RENDER-2D` needs three changes inside state other modules own, which no
interface offered. Each is built by the `RENDER-2D` builder of the milestone named, after the owning modules'
last V1 milestones are Done, with the behaviour [RENDER-2D](RENDER-2D/DESIGN.md#mechanisms) defines. No item of
the owning module changes, its exact comparisons are not loosened, and a scene that asks for none of it compiles
to the same passes and draws the same bytes. Once built, each belongs to its owner: later work there keeps its
tests passing. The rulings were written while V1 was being built, so that no V1 plan changed; `CONTENT-LOOP`'s
design says each ask becomes a milestone there, and ruling 2 replaces that for the sheet product.

1. **The frame runtime, in `RENDER-2D:M1`** (`APPLE-TILE` and `APPLE-PRESENTATION`; `thrum-render`'s frame host,
   scene graph and display files). A frame host is created for one
   [scene kind](RENDER-2D/DESIGN.md#1-the-scene-kind), 2D or 3D, and the 3D kind is the default. A 2D scene binds
   its own colour as the pre-tone input with a fixed exposure, selects a
   [display-referred transfer](RENDER-2D/DESIGN.md#7-colour) with no tone curve and no dither, and declares no
   reconstruction: its extent is the surface's, with no jitter, no history and no scale for the governor.
   C07 and [the hardware-native frame contract](../design/decisions/HARDWARE-NATIVE-ENGINE.md#frame-contract) carry
   this as the contract's second form: a view of a 2D scene publishes one unjittered projection, exposure and
   output extent and none of the provider inputs, the registry resolves no provider for it, and a request that
   names one is refused by name at preparation. Every view of a 3D scene still publishes the full form, so no
   provider, history rule or governor behaviour of `APPLE-PRESENTATION` changes. The declaration is the frame
   runtime's and the refusal is `APPLE-PRESENTATION`'s; both are built in `RENDER-2D:M1`.
2. **Content, in `RENDER-2D:M4`** (`CONTENT-LOOP` and `EFFECT-GRAPH`; `thrum-cook`, `thrum-asset`,
   `thrum-asset-gpu` and the contribution list). The [sheet product](RENDER-2D/DESIGN.md#8-sheets) is a new
   product kind; its pages are lent as an array texture under the lease's rules, and they switch with its frame
   table under the content publication. In a 2D scene residency runs with leases only, and a contribution that
   declares a shadow, opaque or after-opaque stage is refused when it is attached.
3. **The V1 game and its shells, in `RENDER-2D:M1` and, for the simulator, `M5`** (`SHOWCASE`, `T2-CAPTURE` and
   `PLATFORM-RUNTIME-IOS`; `examples/v1-game`, `examples/v1-game-ios` and the capture and check scripts). A
   `sprites` scene selection, off by default, as the other modules' diagnostics are. It is never part of the
   showcase.

<a id="seam-rulings-for-session"></a>

**Seam rulings for `SESSION`.** `SESSION` needs two changes inside state other modules own, which no interface
offered. Each is built by the `SESSION` builder of the milestone named, after `SIM-2D`'s last milestone is Done,
with the behaviour [SESSION](SESSION/DESIGN.md#mechanisms) defines. No item of `SIM-2D` changes, and its two
retained traces keep their retained digests: neither ruling refreshes an exact baseline, and a builder who finds
it cannot hold them stops and ends `BLOCKED`. Once built, each belongs to its owner: later work there keeps its
tests passing.

1. **The world image, in `SESSION:M1`** (core and `SIM-2D`; `thrum-core`'s `sim` module and the component types
   of `examples/crowd-2d`). The state encoding gains its decoding half, and a registered runtime gains a way to
   write [a world image](SESSION/DESIGN.md#7-the-world-image-and-the-baseline) and to be built from one in
   another process. The image carries exactly what the digest covers. A type that supplies one half of the
   encoding without the other does not compile into a rule set, so every later rule package supplies both.
2. **The reference crowd in a match, in `SESSION:M4`** (`SIM-2D`; `examples/crowd-2d`). The player's movement
   becomes one function in the rule package, called by the crowd's own rule and by
   [the predicted controller](SESSION/DESIGN.md#11-the-predicted-controller). It is a change of shape only: the
   rule package still passes `SIM-2D`'s guard with nothing suppressed.

<a id="seam-rulings-for-game-ui"></a>

**Seam rulings for `GAME-UI`.** `GAME-UI` needs four changes inside state other modules own, which no interface
offered. Each is built by the `GAME-UI` builder of the milestone named, after `RENDER-2D`'s last milestone and
the owning modules' last V1 milestones are Done, with the behaviour [GAME-UI](GAME-UI/DESIGN.md#mechanisms)
defines. No item of the owning module changes and its exact comparisons are not loosened. A host prepared with
no interface compiles to the same passes and draws the same bytes, and a game that uses none of it links none of
it. Once built, each belongs to its owner: later work there keeps its tests passing.

1. **The interface stage, in `GAME-UI:M1`** (`APPLE-TILE` and `APPLE-PRESENTATION`; `thrum-render`'s frame
   host, scene graph and display files). A host may be prepared with an interface, for either scene kind. It then
   has [one more stage](GAME-UI/DESIGN.md#1-the-interface-stage) after reconstruction and the display transform
   and before the frame is captured or shown, which draws a paint list into the target the transform wrote, in
   linear light, with interface white at SDR white. With the 8-bit display image that is a pass between the
   transform and present. With `APPLE-PRESENTATION`'s EDR surface it is in the present pass, after the
   transform's draw into the `Rgba16Float` target, where 1.0 is SDR white. It changes no pixel that no row
   covers, and its pipelines for both are made when the host is prepared.
2. **The sprite pass's second use, in `GAME-UI:M1`** (`RENDER-2D`; `thrum-render`'s sprite files).
   [The pass](RENDER-2D/DESIGN.md#interfaces) runs once more in a frame with the display image as its target,
   a view in output pixels, its own set of up to eight sheets and the paint list as its rows. The scene's use of
   the pass is unchanged.
3. **The font product, in `GAME-UI:M2`** (`CONTENT-LOOP`; `thrum-cook`, `thrum-asset` and `thrum-asset-gpu`).
   [A font](GAME-UI/DESIGN.md#4-fonts) is a new product kind: a table and one pixel-kind sheet product for each
   scale, leased and switched as sheets are. The interface is one more reader of a publication that changes a
   sheet it uses: it [lays out again beside what is drawn](GAME-UI/DESIGN.md#11-a-list-and-its-sheets-change-together),
   the publication is not ready until the render owner holds that layout's paint list, and a table the
   interface cannot take refuses the publication whole. `CONTENT-LOOP`'s design says each ask becomes a milestone there;
   this ruling replaces that for the font product, as `RENDER-2D`'s ruling 2 did for sheets. The cooker gains
   `skrifa`, `harfrust` and `zeno`; no shipping crate does.
4. **The shells and the V1 game, in `GAME-UI` M1 to M5** (`CREATOR`'s `thrum-app` window and page; `SHOWCASE`'s
   `examples/v1-game` and its page; `T2-CAPTURE`'s scripts; `PLATFORM-RUNTIME-IOS`'s `examples/v1-game-ios`).
   Each shell gives every key, pointer and touch event to
   [the interface first](GAME-UI/DESIGN.md#8-who-gets-an-event), reports the output's extent, the display's
   density and the safe-area insets, and delivers text drops; the Mac shell also writes the clipboard. The
   translation sits behind a feature of `thrum-app`. An `interface` scene selection is added to the V1 game, off
   by default and never part of the showcase.

<a id="seam-rulings-for-swarm"></a>

**Seam rulings for `SWARM`.** `SWARM` needs four changes inside state other modules own, which no interface
offered. Each is built by the `SWARM` builder of the milestone named, after the owning modules' last milestones
and `GAME-UI`'s are Done, with the behaviour [SWARM](SWARM/DESIGN.md#mechanisms) defines. No item of an owning
module changes and its exact comparisons are not loosened. The V1 game, the starter and the creator scenes
build, run and draw as they did. Once built, each belongs to its owner: later work there keeps its tests
passing. No engine crate gains a type, string or file of either game.

1. **The repository guard, in `SWARM:M2`** (`scripts/repo_check.py` and `.config/architecture.toml`). The guard
   admits more than one external game. No engine crate may depend on any game, and a game may not depend on
   another game. The V1 game's boundary is as it was, and `SIM-2D`'s rule-package guard is unchanged.
2. **The window and the page for a second game, in `SWARM:M2`** (`CREATOR` and `PLATFORM-RUNTIME-WEB`;
   `thrum-app`'s window and page, with `T2-CAPTURE`'s and the packaging scripts). `RENDER-2D:M5` runs a second
   program on both and `GAME-UI` gives them events, density, insets and text drops. Whatever they still assume
   of the V1 game or the sky kit when a game with a 2D scene, an interface and no sky kit runs, is capturable
   and is packaged for Chrome, is made a property of the game.
3. **The iOS shell for a second game, in `SWARM:M6`** (`PLATFORM-RUNTIME-IOS`, with what `COOKED-DRAW`,
   `RENDER-2D` and `GAME-UI` added; `examples/v1-game-ios` and `thrum-app`). The UIKit lifecycle, touch
   delivery, the render owner and its surface lease, rotation, device-loss recovery and `GAME-UI`'s reports of
   extent, density and insets move into `thrum-app`, generic over a game as the Mac shell is.
   `examples/v1-game-ios` becomes its first user and passes every simulator check those modules left,
   unchanged; `swarm-ios` is its second. Neither game keeps a copy of that policy.
4. **A driven world, in `SWARM:M7`** (core F1 and W1, with `CREATOR`; `thrum-core`'s world lane and clocked
   world, the frame host's world observer in `thrum-render`, and `thrum-app`). A world may be
   [driven](SWARM/DESIGN.md#7-who-admits-the-frames-alone-and-in-a-match): its driver names how many ticks to
   commit at each advance, none, one or several, and where the presentation stands between the last two. A
   driven world raises no overload for ticks nobody asked for. A world on the fixed clock behaves as it did,
   and its tests pass unchanged. The engine side names no session and no link: the driver is the game's, which
   asks `SESSION`.

<a id="interfaces-track-3"></a>

### Interfaces (track 3)

New interfaces are defined in the design of the module that provides them and are provisional until its design
review passes. Changes to a numbered contract are settled with its producers and consumers, as the brief requires.

| Interface | Provider → consumers | Defined in |
|---|---|---|
| Tick page, committed and previous endpoints, rewind and replay | core → `SIM-2D`, `RENDER-2D`, `SESSION` | C02, [G2 decision](../design/decisions/G2-GENERIC-SIMULATION.md); unchanged |
| **Deterministic rules profile**: what a rule may compute with and in what order, so one input trace gives one result on every target. Rules live in a rule package that `./dev check` guards, and one conformance command runs any rule package's trace on the three targets | `SIM-2D` → `SWARM` (its rules), `SESSION` | [SIM-2D](SIM-2D/DESIGN.md#mechanisms); C02 |
| **World digest**: one value for each committed tick over all authority state, computed on request. A baseline must restore everything it covers | `SIM-2D` → `SESSION` | [SIM-2D](SIM-2D/DESIGN.md#4-the-world-digest) |
| **2D queries**: overlap, nearest and neighbours, with results in a fixed order that depends on the index's cell size, which is a constant of the rule set and never a quality setting | `SIM-2D` → `SWARM` | [SIM-2D](SIM-2D/DESIGN.md#5-the-index-and-its-queries); C03 |
| **Sprite rows**: a game's read-model components map to sheet frame, layer, sort value, tint, flash and transform, interpolated between tick endpoints on the GPU; rows the game gives each frame are drawn among them. One total order: layer, sort value, stable identity, kept from tick to tick so that two rows near each other exchange places only when one has passed the other by more than the scene's band | `RENDER-2D` → `SWARM`, any 2D game | [RENDER-2D](RENDER-2D/DESIGN.md#3-sprite-rows); C07 |
| **Sheets**: a folder of images cooks to pages and a frame table; a sheet is pixel or smooth; a game names frames by name, and a scene shows at most eight sheets. A glyph is a frame, so a font cooks to sheets | `RENDER-2D` → `SWARM`, `GAME-UI` | [RENDER-2D](RENDER-2D/DESIGN.md#8-sheets); C05 |
| **The sprite pass at another stage**: the pass takes its target, view and rows as inputs, so a second instance draws a paint list at the interface stage | `RENDER-2D` → `GAME-UI`, wired by [ruling 2](#seam-rulings-for-game-ui) | [RENDER-2D](RENDER-2D/DESIGN.md#interfaces) |
| **2D scene half**: a host created for the 2D scene kind compiles only that scene's pass and the output half's, writing the pre-tone scene-linear input at the surface's pixel extent | `APPLE-TILE` → `RENDER-2D`, by [ruling 1](#seam-rulings-for-render-2d) | [RENDER-2D](RENDER-2D/DESIGN.md#1-the-scene-kind); [APPLE-TILE](APPLE-TILE/DESIGN.md), C06, C07's no-reconstruction form |
| Sprite-sheet and font products. A **font product** is a table of metrics and one pixel-kind sheet for each scale, rasterised when it is cooked at the sizes a game names | `CONTENT-LOOP` → `RENDER-2D`, `GAME-UI`, games. The sheet product by [ruling 2](#seam-rulings-for-render-2d), the font product by [ruling 3](#seam-rulings-for-game-ui) | [RENDER-2D](RENDER-2D/DESIGN.md#8-sheets); [GAME-UI](GAME-UI/DESIGN.md#4-fonts); C05 |
| **Interface stage**: a paint list drawn at output extent into the target the display transform wrote (the 8-bit display image, or the EDR present pass's float target), before the frame is captured or shown, blended in linear light with interface white at SDR white. A host without one is unchanged | `APPLE-TILE` and `APPLE-PRESENTATION` → `GAME-UI`, by [ruling 1](#seam-rulings-for-game-ui) | [GAME-UI](GAME-UI/DESIGN.md#1-the-interface-stage); C06, C07 |
| **Event routing**: every key, pointer and touch event goes to the interface first, which consumes it or passes it on; each press has one owner until its release; the game is told when its input is interrupted and what is still held afterwards; text drops and the clipboard | shells, by [ruling 4](#seam-rulings-for-game-ui) → `GAME-UI` → the game | [GAME-UI](GAME-UI/DESIGN.md#8-who-gets-an-event); C11 |
| **Interface tree**: nodes a game builds in code (stacks, overlays, text, images, fills, panels, bars, buttons, scopes), laid out in points for the output's extent, scale and safe area; text runs from a font product; focus; activations handed to the game | `GAME-UI` → `SWARM`, any game | [GAME-UI](GAME-UI/DESIGN.md#interfaces) |
| **Message link**: whole messages on a reliable ordered lane (up to 65,536 bytes) and a datagram lane (up to 1,000 bytes), backpressure, lifecycle events with a reason, and counters, the same on every adapter and on the in-memory conditioned link. A reliable message arrives or the link closes, and a close can cut off the last ones sent. A datagram's wait is bounded in the link's own queue only, so one may arrive late and `SESSION` rejects old ones by content. A host's **join ticket** is its address and its certificate's hash | `TRANSPORT` → `SESSION`; the ticket also to `SWARM`, which shows it in `GAME-UI`'s nodes | [TRANSPORT](TRANSPORT/DESIGN.md#1-the-link); [NETWORKING](../design/modules/NETWORKING.md#transport-and-overload), A0 |
| **World image**: a world's whole authority state as bytes, the same bytes the digest is taken over, restored into a fresh runtime on any target | core and `SIM-2D` → `SESSION`, by [ruling 1](#seam-rulings-for-session) | [SESSION](SESSION/DESIGN.md#7-the-world-image-and-the-baseline); C02 |
| **Session**: a host and a client driven by time: join and leave, roster events for each player slot, the host's admitted set for each tick, how many ticks to commit now, the predicted controller and its corrections, and commands that happen once in order. A game says which of its actions are commands | `SESSION` → `SWARM` | [SESSION](SESSION/DESIGN.md#interfaces); C13, A0 and the A5 decisions |
| **Connection state**: what a screen can show for this peer and for each slot, with the reason for a refusal or a close | `SESSION` → `SWARM`, which shows it in `GAME-UI`'s nodes; `GAME-UI` links neither | [SESSION](SESSION/DESIGN.md#9-negotiation-and-leaving) |
| **Rule set in the gameplay manifest**: a match names its deterministic rule set and build, by an identity derived from their sources, and refuses a peer that differs. A proposed addition to C13, written into the contract by `SESSION:M6` | `SESSION`, with `SIM-2D`, core and the game | [SESSION](SESSION/DESIGN.md#interfaces) |
| Quality knobs | `RENDER-2D` → the governor | C04; the interface above, unchanged. `RENDER-2D` registers none: a 2D scene has no scale, cadence or step budget to lower |
| The app runner and typed controls | `CREATOR` → `SWARM` | C11, [CREATOR](CREATOR/DESIGN.md) |
| Engine capture, one for each peer process | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE/DESIGN.md); unchanged |

### Build order after the V1 exit

Track 3 starts when `SHOWCASE` is Done. Whether it runs before, after or beside track 2 is the owner's decision.
Each module needs a `plan-module` run from its brief and a design review that returns Design Approved before a
builder starts it. The modules are planned in the order below.

1. **The two proofs that can overturn the plan.** `SIM-2D`, first milestone: one input trace gives equal digests
   on Mac, in Chrome and on the iOS simulator at the brief's crowd size, with tick, rewind and replay costs
   recorded. `TRANSPORT`, first milestone: one native host holds a Mac client, a Chrome client and an iOS-simulator
   client over real sockets under the A0 conditioner. They share no files with each other or with the renderer.
2. **Sprites.** `RENDER-2D`, which builds its asks of `APPLE-TILE` and `CONTENT-LOOP` itself under
   [the seam rulings](#seam-rulings-for-render-2d). Its first four milestones can run beside step 1; its last
   draws `SIM-2D`'s crowd and waits for `SIM-2D`'s first milestone.
3. **A match without pictures.** `SESSION`, after `SIM-2D`'s last milestone, since its first milestone works in
   the same files. Its matches on the in-memory conditioned link follow `TRANSPORT`'s first milestone, which
   delivers that link, and its socket milestones follow `TRANSPORT`'s second.
4. **Interface.** `GAME-UI`, after `RENDER-2D`'s last milestone, since its first two milestones work in the
   same files. It builds the interface-stage and font-product asks itself under
   [the seam rulings](#seam-rulings-for-game-ui).
5. **The game.** `SWARM` opens with the owner's choice of concept, which only `SWARM` waits on. Then a
   single-player slice and the full run after `SIM-2D`, `RENDER-2D` and `GAME-UI`, a rehearsed match after
   `SESSION`'s fourth milestone, co-op on sockets after its last, and the close. It builds its asks of the
   guard, the shells and the world lane itself under [the seam rulings](#seam-rulings-for-swarm).

A lone builder takes them in that order: `SIM-2D`, `TRANSPORT`, `RENDER-2D`, `SESSION`, `GAME-UI`, `SWARM`.
`SWARM`'s options milestone is built first of all, so the owner's answer is in before any art is made.

**Reaching the targets.** `TRANSPORT`, `SESSION` and `SWARM` open local sockets and drive Chrome, which Codex's
`workspace-write` sandbox does not allow without a request for each run. Their planners settle how a builder and a
reviewer reach those targets; [the choice](#owner-decisions) is the owner's. `TRANSPORT` puts every socket proof
behind [one proof command](TRANSPORT/DESIGN.md#proof-and-builder-handoff) and keeps sockets out of `./dev check`.
`SESSION` does the same with [one match command](SESSION/DESIGN.md#proof-and-builder-handoff), through
`TRANSPORT`'s relay. `SWARM` puts every proof that needs a window, Chrome, the simulator or a socket behind
[one command of its own](SWARM/DESIGN.md#proof-and-builder-handoff), which reuses `SESSION`'s match harness and
the relay.

### What finished means (track 3)

Proposed, for the owner to change: two to four players, on any mix of Mac, Chrome and the iOS simulator, join one
host and play a whole run of an original swarm survivor together, with thousands of enemies and projectiles on
screen, smooth at the display's full refresh on the Mac, and the peers still agree under the A0 conditioner. The
engine modules hold no game types, and a second small 2D scene builds on them without `SWARM`.

### Module briefs (track 3)

A `plan-module` run turns each brief into a design, a plan and a done-when file. Costs are recorded as tripwires;
no budget below is a gate. The quality bar is a shipped game, named for each milestone by the planner.

#### SIM-2D

Planned on 13 November 2026: [design](SIM-2D/DESIGN.md), [plan](SIM-2D/PLAN.md). Its planner settled the two
choices this brief carried: rules compute in `f32` held to exactly rounded operations, not fixed point, and the
queries sit beside `thrum-core` in a new crate. Three milestones; `SESSION` needs M3, because its first milestone works in
the same files, and quotes M1's record of tick, rewind and replay costs; `SWARM` needs M3. If M1's three targets cannot be made equal, the
builder stops and [mechanism 1](#mechanisms-settled-here) is replanned.

#### RENDER-2D

Planned on 13 November 2026: [design](RENDER-2D/DESIGN.md), [plan](RENDER-2D/PLAN.md). Its planner settled the
four choices this brief carried: order is sorted rows on the CPU, with a band that stops jitter from reordering a
packed crowd; a 2D scene has no reconstruction and is drawn at the surface's own pixel size; values above white
are kept for the output half, and how they look on an EDR panel is not judged; and text shares the path as rows,
a glyph being a frame on a sheet. Five milestones. `GAME-UI` and `SWARM` need M5. The asks of `APPLE-TILE`, `APPLE-PRESENTATION` and `CONTENT-LOOP` are built in M1 and M4
under [the seam rulings](#seam-rulings-for-render-2d). Three findings bind later planners: sprites blend in
linear light, so soft art looks lighter over dark than in a paint tool; a scene shows at most eight sheets; and
no iOS app for a second game exists yet, so the crowd view is drawn on the Mac and in Chrome and the simulator
is reached through the V1 game's shell.

#### GAME-UI

Planned on 13 November 2026: [design](GAME-UI/DESIGN.md), [plan](GAME-UI/PLAN.md). Its planner settled the two
choices this brief carried: glyphs are rasterised when a font is cooked, with `skrifa`, `harfrust` and `zeno` in
the cooker only, so a shipped game holds no font code; and a join ticket is never typed, but arrives as a text
drop from a link, a launch argument or a paste. [A probe](../evidence/GAME-UI/planning-probe.md) sized a cooked
font and checked the rules for spacing, event ownership and direction keys. Five milestones, all after
`RENDER-2D:M5`, and not after its first milestone as this brief first said, because the interface stage and the
sprite pass's second use are built in files `RENDER-2D`'s builder works in. The asks of `APPLE-TILE`,
`RENDER-2D`, `CONTENT-LOOP` and the shells are built in M1 to M5 under
[the seam rulings](#seam-rulings-for-game-ui). `SWARM` needs M4 for its screens on the Mac and in Chrome and M5
for the simulator. Five findings bind `SWARM`'s planner: text exists only at the sizes and scales a font was
cooked at, and a size that was not cooked is refused; the interface blends in linear light, so a half-transparent
panel is lighter than in a paint tool; whether a choice pauses the run is the game's, and the interface only
says what was activated; there is no text field, scrolling list or transition system; and the app runner
`SWARM` gets must deliver events, density, insets and text drops as the V1 game's shells now do.

#### TRANSPORT

Planned on 13 November 2026: [design](TRANSPORT/DESIGN.md), [plan](TRANSPORT/PLAN.md). Its planner settled the
three choices this brief carried: Quinn with `web-transport-quinn` on the host and a hand-written binding of the
browser's `WebTransport`; one I/O thread that meets the world lane only at bounded queues; and no WebSocket
fallback. [A probe](../evidence/TRANSPORT/planning-probe.md) reached one host from the Mac, Chrome and the
simulator with datagrams, so the fallback this brief named, input on the reliable stream for Chrome, is held in
reserve and not built. Two milestones; `SESSION` needs M1 for the in-memory conditioned link and M2 before its socket
milestones. M1 also delivers the three socket adapters and a cost record under each congestion controller. Four findings bind `SESSION`: a join's baseline
should flow from the host down; a frozen browser page keeps its link open, so silent input is `SESSION`'s to time
out; a datagram can arrive long after it was sent, so old input and state are rejected by content; and a close can
cut off the last reliable messages, so one that must arrive is answered before the link is closed.

#### SESSION

Planned on 13 November 2026: [design](SESSION/DESIGN.md), [plan](SESSION/PLAN.md). Its planner settled the three
choices this brief carried: the crowd is drawn at the confirmed tick and only a player's own character is
predicted; digests are compared twice a second to start, a setting of the match; and a modal choice is a command
on the reliable lane, admitted at one tick and sent to every peer in that tick's set.
[A timing model](../evidence/SESSION/planning-probe.md) settled two more: the host's admitted input travels as
datagrams that repeat what a client has not acknowledged, and a client sizes its input lead from what the host
reports. Six milestones. `SWARM` needs M5 for co-op and can rehearse on the in-memory link from M4. The world
image and one function of the reference crowd are built in M1 and M4 under
[the seam rulings](#seam-rulings-for-session). Four findings bind `SWARM`'s planner: a client's world must step
when the session says, which the app runner's world lane does not yet allow; a hero the crowd can push is
corrected every tick it is pushed; the crowd is drawn behind the hero by the lead plus the confirmed delay; and
a peer too slow for the match is dropped, since one match has one crowd.

#### SWARM

Planned on 13 November 2026: [design](SWARM/DESIGN.md), [plan](SWARM/PLAN.md). Its planner settled the co-op
rules this brief carried: one team level from shared pick-ups, every player choosing their own upgrades at the
same time between waves, a fallen hero back at the next wave, and no choice that pauses the run. It read
"upgrades between waves" as Brotato's shape, with nothing paused inside a wave, and the brief's "thousands" as
at least 2,000 enemies and projectiles in one view, which makes a common enemy about a fiftieth of the screen's
height. Eight milestones: the owner's pick of a concept (after `SHOWCASE:M8`), a single-player slice that first
measures how large a crowd Chrome can hold, the whole run with its screens, the finished content and look, a
rehearsed match (after `SESSION:M4`), the iPhone simulator, co-op on sockets (after `SESSION:M6`), and the
close with the owner's play and the record that ends track 3. The three asks this brief named, and a fourth for
the window and the page, are built in M2, M6 and M7 under [the seam rulings](#seam-rulings-for-swarm).

## Owner decisions

Nothing below is decided by this plan.

1. **Port timing.** [The water decision](../design/decisions/E1-GPU-WATER.md) says implementation follows G7, and the
   port decision says the owner schedules it. The readiness conditions are met before G7, so the owner can start
   `SEA` and `CLOUDS` builders at their CPU milestones now, at their GPU milestones at step 5–6, or after G7.
   On 13 November the owner said planning can start and "you can bring that across when it makes sense".
   `SEA` is planned on the reading that its plan decides when; that reading is
   [an assumption](SEA/PLAN.md#open-for-the-human) the owner can overturn. `CLOUDS` is planned on the same
   reading, under [the same assumption](CLOUDS/PLAN.md#open-for-the-human).
2. **Licence.** Broadsides code is marked `UNLICENSED` and its cloud library carries an owner-only licence.
   Thrum's demos are meant to have public source. Lifting either into this repository needs the owner's licence
   for it first. `SEA`'s plan is written so that it can be built without lifting, and nothing is copied until the
   owner names the licence. `CLOUDS` can be built without lifting code, but its cue K5 names the twelve accepted
   volumes and their growth track: its M10 waits for the licence, and holds its sea milestone and its close.
3. **The V1 exit stays as written.** Sea and clouds are not added to it. Adding them is the owner's call.
4. **Builders.** The profile says one Codex builder. Steps 2–4 of the build order hold work that is safe beside
   `APPLE-TILE` if the owner allows a second.
5. **Broadsides threads.** This plan asks nothing more of them. Whether they stay paused is the owner's call.
6. **Spending.** None is requested. Physical-device runs stay owed and are reported at the exit.

For [track 3](#track-3-the-swarm-survival-game-after-v1):

7. **When track 3 starts.** The brief says after V1, and this plan holds every track 3 module until `SHOWCASE` is
   Done. Whether the game goes before, after or beside sea and clouds is the owner's order to set.
8. **The game's identity.** Its name, premise, characters and look are the owner's taste. `SWARM`'s first
   milestone lays out options and waits for the owner's pick. Its last waits for the owner to play the game.
9. **Art and licence.** The plan assumes art made in this repository, original or PD and CC0. A paid generation
   service is spending. The licence the game and its art are published under is the owner's.
10. **A public host.** Play across the internet needs a machine others can reach, a domain and a certificate,
    which is spending and deploying. Until the owner asks for one, "online" is proved between separate processes
    on this Mac over real QUIC and WebTransport under the A0 conditioner. Putting the game on the website is
    publishing.
11. **Sockets and Chrome under the Codex sandbox.** `TRANSPORT`, `SESSION` and `SWARM` need both for most checks.
    The choices are a request for each run as today, `Codex sandbox: danger-full-access`, or a Claude builder for
    those modules. `TRANSPORT`'s and `SESSION`'s plans assume a request for each run.
12. **Sound.** The brief does not ask for it and no audio module is planned. A1 stays a design.
13. **The purpose line.** The brief still says "3D engine" and requirement E08 of the architecture brief
    still puts pure 2D outside scope. The 10 November goal overrides both; rewording them is the owner's page.
14. **What finishes the goal.** [The proposal](#what-finished-means-track-3) stands in for an exit the owner has
    not written.
15. **Builders.** Steps 1 and 2 of track 3's order hold three lanes that share no files.

## Assumptions for modules not yet planned

Each stands until the owner overturns it, and moves to its module's plan, under `Open for the owner`, when that module is planned.

- The sea's assumption has moved to [SEA's plan](SEA/PLAN.md#open-for-the-human).
- The clouds' assumption has moved to [CLOUDS's plan](CLOUDS/PLAN.md#open-for-the-human).
- The track 3 assumptions have moved to their modules' plans: the readings of "2D support in the engine" to
  [RENDER-2D](RENDER-2D/PLAN.md), [SIM-2D](SIM-2D/PLAN.md) and [GAME-UI](GAME-UI/PLAN.md), "no gamepad" to
  GAME-UI's and [SWARM](SWARM/PLAN.md)'s, and "no sound" and "the simulator proves function, never speed" to
  SWARM's.

## Log

- 2026-11-10 First module list. Planned from the owner's request above. `CREATOR`, `ATMOSPHERE` M5–M6 and
  `APPLE-PRESENTATION` M1–M3 gained done-when files; the other modules gained briefs. Assumptions that belong to a planned module are in that module's plan, under `Open for the owner`; those for modules not yet planned are above.
- 2026-11-11 `CREATOR` revised after its design review. Its profiler could not be built as a reader of the
  existing timing interfaces, so the frame timing feed and its seam ruling were added above, and `CREATOR` M2
  moved from step 2 to step 5 of the build order, behind `APPLE-TILE:M3`. No other module's plan changed.
- 2026-11-10 Track 3 added from the owner's second request, quoted in its section: six modules with briefs, the
  asks of existing modules, interfaces and a build order after the exit. No V1 module's plan, no done-when file
  and no part of the exit changed. The brief's three defaults are unchanged. Researched: Vampire Survivors' online
  co-op, Factorio's join, WebTransport's browser state and the 2D paths of Bevy, Godot and Unity. Its readings are listed above and stand until the owner overturns them.
- 2026-11-11 Reshaped for orchestrate 1.7.0. The module table now opens the page, in build order and without
  status; the status column, the readiness table's state column and the paragraphs on what was ready or owed are
  gone, since [the status page](STATUS.md) reports them. The capability designs the migration had carried into the
  plans folder went back to `docs/design/modules/`, and the build plan was folded into [the brief](BRIEF.md). No
  module's ownership, interface, build order or done-when item changed.
- 2026-11-11 `CULLING` and `EFFECT-GRAPH` planned, each with a design, plan and done-when file, from the owner's
  request quoted in their designs. `EFFECT-GRAPH` has three milestones, so the port's graph condition moved from its
  M1 to its M3. [Exact baselines](#exact-baselines) added after `APPLE-TILE` M2's review found that another module's
  change had moved its baseline unnoticed: three rules, a registry, and a new one-milestone module,
  `BASELINE-GUARD`, inserted at row 9 (later rows renumbered). The entity-visibility interface settles the open
  decision between `CULLING` and `CONTENT-LOOP`. No other module's done-when item changed.
- 2026-11-11 `BASELINE-GUARD`, `CULLING` and `EFFECT-GRAPH` revised after their design reviews.
  [Exact baselines](#exact-baselines): rule 2 is now a proof gate in `./dev check`, keyed to the exact covered
  content, because `land` cannot hand control back between its rebase and its push; rule 3 is the backstop; the
  Chrome baseline is four retained digest manifests, by a ruling with `APPLE-TILE`. `CULLING` M2 and `EFFECT-GRAPH`
  M2 gained `BASELINE-GUARD:M1` in their `needs`. `CULLING` no longer builds a live mode switch. No other module's
  done-when item changed.
- 2026-11-13 `CONTENT-LOOP` planned, with a design, plan and done-when file, from the lead's task quoted in its
  design. Three milestones: leases (after `APPLE-PRESENTATION:M2`), cooked mesh rows with live replacement (after
  `CULLING:M3`) and the close of G5 (after `APPLE-PRESENTATION:M3`). The leases interface and the seam ruling for
  row replacement were added above; `SEA` and `CLOUDS` now name `CONTENT-LOOP` M1, and the port's G5 condition is
  met there. No other module's done-when item changed.
- 2026-11-13 `CONTENT-LOOP` revised after its second design review (MF3). The content-publication interface was
  added: placements, leases and cooked mesh rows switch in one frame, as C05's qualified visual preview asks.
  The leases and entity-visibility lines and the M2 seam ruling name that frame. C05 is unchanged, and no other
  module's done-when item changed.
- 2026-11-13 `SHOWCASE` planned, with a design, plan and done-when file, from the lead's task. Four milestones, M5
  to M8: the populated valley (after `CONTENT-LOOP:M2` and `CULLING:M3`), its culling, the three rows from one
  exit configuration, and the review with the V1 exit record (after every other track 1 module's last milestone).
  The resolved-selections interface and the exit baselines' row were added above. No exact baseline moves and no
  other module's done-when item changed. `APPLE-PRESENTATION` M4 to M11 are still to be planned, and the exit
  waits on them.
- 2026-11-13 `SIM-2D` planned, the first module of track 3, with a design, plan and done-when file, from the
  lead's task. Three milestones: the three-target proof at 10,000 bodies (after `SHOWCASE:M8`), the queries
  against a brute-force search, and the check that refuses a rule which breaks the profile. Its three interfaces
  now link to its design. No contract, exact baseline or other module's done-when item changed.
- 2026-11-13 `TRANSPORT` planned, the second module of track 3, with a design, plan and done-when file, from the
  lead's task. Two milestones: one host holding Mac, Chrome and simulator clients through A0's conditioner (after
  `SHOWCASE:M8`), and the hostile, stalled and frozen peers with the close. The message-link interface now links to
  its design and names its limits and the join ticket. Two assumptions moved to its plan. No contract, exact
  baseline or other module's done-when item changed.
- 2026-11-13 `TRANSPORT`'s design review asked for changes, and the message-link row and `TRANSPORT`'s entry now
  say two things the design had left unclear: a close can cut off the last reliable messages sent, and a
  datagram's wait is bounded in the link's own queue only, so `SESSION` rejects old ones by content. No contract,
  exact baseline or other module's done-when item changed.
- 2026-11-13 `RENDER-2D` planned, the third module of track 3, with a design, plan and done-when file, from the
  lead's task. Five milestones: a 2D scene through the game's frame with nothing 3D (after `SHOWCASE:M8`,
  `APPLE-TILE:M12` and `EFFECT-GRAPH:M3`), ten thousand rows that glide in a stable order, crisp sampling at
  every window size, cooked sheets with live edits (after `CONTENT-LOOP:M3`), and the reference crowd with the
  simulator and the close (after `SIM-2D:M1`). Its asks of `APPLE-TILE`, `APPLE-PRESENTATION` and `CONTENT-LOOP`
  are settled as [seam rulings](#seam-rulings-for-render-2d) that its own builder builds after V1, so no V1
  module's design, plan or done-when item changed. The sprite-rows, sheets and 2D-scene-half interfaces now link
  to its design. No contract or exact baseline changed.
- 2026-11-13 `RENDER-2D`'s design review asked for two changes, and both reach this page. C07 required every view
  to publish motion, depth, a reactive mask and jitter, which a 2D scene under ruling 1 never makes; C07 and the
  hardware-native frame contract now name two forms, full for every view of a 3D scene and no reconstruction for a
  view of a 2D scene, with the refusal of a provider named for one. Ruling 1 states it. No 3D view's obligations
  changed, so no V1 module's design, plan or done-when item changed. The sprite-rows interface now says the order
  is kept from tick to tick with a band between near rows, replacing a band on each row's own value that the
  review showed could not hold.
- 2026-11-13 `SESSION` planned, with a design, plan and done-when file, from the lead's task quoted in its design.
  Six milestones: the world image (after `SIM-2D:M3`), the steady match and its faults on the in-memory link
  (after `TRANSPORT:M1`), the predicted controller, the match on sockets (after `TRANSPORT:M2`) and the close,
  which writes the rule-set addition into C13. The seam rulings for `SESSION`, the world-image and
  connection-state interfaces and three asks were added above, one of them open for `SWARM`'s planner. Two
  assumptions moved to [SESSION's plan](SESSION/PLAN.md). No other module's done-when item changed.
- 2026-11-13 `SESSION`'s design review asked for four changes; one reaches this page. The line on what keeps a
  miss from spreading now says the digest catches a divergence that lasts to the next digest tick, since a digest
  is taken over one committed endpoint. No interface, seam ruling or other module's done-when item changed.
- 2026-11-13 `SWARM` planned, the last module of track 3, with a design, plan and done-when file, from the
  lead's task. Eight milestones: the owner's pick of a concept (after `SHOWCASE:M8`), a slice alone at the
  brief's crowd size (after `SIM-2D:M3`, `SESSION:M1`, `RENDER-2D:M5` and `GAME-UI:M5`), the whole run, the
  finished content, a rehearsed match (after `SESSION:M4`), the simulator, co-op on sockets (after
  `SESSION:M6`) and the close. The seam rulings for `SWARM` settle the three asks its brief carried and a
  fourth, the window and the page for a game with no sky kit; the ask of core that was open for this planner is
  ruling 4. The last two assumptions for unplanned modules moved to [SWARM's plan](SWARM/PLAN.md). No contract,
  exact baseline or other module's done-when item changed.
- 2026-11-13 `SEA` planned, with a design, plan and done-when file, from the lead's task and
  [its brief](SEA/BRIEF.md), on the owner's words of 13 November quoted in its design. Ten milestones: the CPU
  model (needs nothing), GPU state (on `EFFECT-GRAPH:M2`), the first frame at cost (after `EFFECT-GRAPH:M3`),
  Chrome, bodies and the query, the wake, foam, optics, the whole range in motion (after
  `APPLE-PRESENTATION:M3`) and the close, which waits for the governor and MetalFX. The body-query, medium-segment
  and quality-knob rows now link to its design, and [three seam rulings](#seam-rulings-for-sea) settle its asks
  of `EFFECT-GRAPH` and the shell. The sea's assumption moved to [its plan](SEA/PLAN.md). No contract, exact
  baseline or other module's done-when item changed.
- 2026-11-13 `CLOUDS` planned, with a design, plan and done-when file, from the lead's task and
  [its brief](CLOUDS/BRIEF.md), on the owner's words of 13 November quoted in its design. Twelve milestones: the
  CPU weather (needs nothing), shade with no cloud drawn (on `EFFECT-GRAPH:M2` and `ATMOSPHERE:M5`), the first
  frame at cost (after `EFFECT-GRAPH:M3`), Chrome, light through the day, placed volumes with stand-in shapes
  (on `CONTENT-LOOP:M1`), the weather range, the world lit by clouds, controls and motion (after
  `APPLE-PRESENTATION:M3`), the accepted library (after the owner's licence), the sea (after `SEA:M8`) and the
  close, which waits for the governor and MetalFX. The Sun-path row now names the GPU-written field, a row for
  the sky overlay and haze scale was added, and [five seam rulings](#seam-rulings-for-clouds) settle its asks of
  `ATMOSPHERE` and of the two games it is shown in. The clouds' assumption moved to [its plan](CLOUDS/PLAN.md).
  No contract, exact baseline or other module's done-when item changed.
