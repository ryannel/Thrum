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
| 8 | ATMOSPHERE | One physical sky, Sun, haze, night and stars, which every other part of the engine reads, and the air near the camera, where haze, fog and shafts of light are drawn | CONTENT-LOOP M4 for its M9 to pass the full check; APPLE-PRESENTATION M3 and SHOWCASE M9 for its M6 |
| 9 | BASELINE-GUARD | A change that alters another module's exact reference images is found on main within about two hours and traced to the commit that made it | |
| 10 | CULLING | Only what the camera can see is drawn, decided on the GPU | APPLE-TILE M2, BASELINE-GUARD M1 |
| 11 | EFFECT-GRAPH | Sea, clouds and sky run their own simulation and drawing steps inside the one frame | APPLE-TILE M2 and M3, APPLE-PRESENTATION M2, BASELINE-GUARD M1 |
| 12 | CONTENT-LOOP | Edited meshes, textures and volumes appear in the running game without a stall | Nothing for its M4; APPLE-PRESENTATION M2 and M3, EFFECT-GRAPH M2, CULLING M3 |
| 13 | SHOWCASE | The V1 exit: the showcase valley on Mac and in Chrome, reviewed, with its closing record | CONTENT-LOOP M2 and CULLING M3 to start; ATMOSPHERE M10 for the populated valley's motion review; APPLE-TILE M12, APPLE-PRESENTATION's last milestone, ATMOSPHERE M6 (which closes G3s) and CONTENT-LOOP M3 for the three rows from one command; PLATFORM-RUNTIME-MAC M3 and CREATOR M3 as well to close |
| 14 | SEA | The wind's waves, from glassy calm to gale: one wave field that every body of water reads | Nothing to start; EFFECT-GRAPH M2 for the field on the GPU |
| 15 | WATERS | Sea, shore, lakes and ponds as one water: drawn, lit, floated on, stirred and foaming, breaking on its beaches, mirroring the land, seen from beneath and lit there by the Sun's net and shafts, with a place made for rivers and puddles | SEA M1 to start; SEA M2, EFFECT-GRAPH M3, ATMOSPHERE M5 and M9 and CREATOR M1 for its first frame; CONTENT-LOOP M1 for foam; APPLE-PRESENTATION M3 for the whole range in motion; SCENE-VIEWS M3 for mirrored scenery; APPLE-PRESENTATION's governor and MetalFX milestones to close |
| 16 | SCENE-VIEWS | The scene is drawn a second time from another viewpoint, so water can mirror hills and trees the camera cannot see | CULLING M1, ATMOSPHERE M5 and EFFECT-GRAPH M3 |
| 17 | CLOUDS | The weather: a sky from clear to overcast with placed cloud volumes, and mist, fog and low cloud on the ground, which shade and light the world | Nothing to start; EFFECT-GRAPH M2 and M4 and ATMOSPHERE M5 for its shade; EFFECT-GRAPH M3 for its first frame; CONTENT-LOOP M1 for placed volumes, and DEAD-TOOLS M5 for the record of the library it copies; APPLE-PRESENTATION M3 and CREATOR M1 for controls and motion; ATMOSPHERE M8 for mist and fog; WATERS M8 for the sea; APPLE-PRESENTATION's governor and MetalFX milestones to close |
| 18 | RIVERS | Rivers, streams and waterfalls that run down the land, swell with the lake that feeds them, carry what floats and foam where they fall | WATERS M1 for its flow on the CPU; WATERS M9 for its first running stream; WATERS M10 and M16 to close |
| 19 | PUDDLES | Rain wets the ground and fills its hollows with puddles that ripple, reflect the sky and dry out | WATERS M1 for its puddles and wet ground on the CPU; WATERS M8 for its first puddle; SHOWCASE M5 for the valley; WATERS M10, M14 and M16 to close |
| 20 | SIM-2D | Thousands of 2D bodies step identically on every device, and find neighbours and hits cheaply | |
| 21 | TRANSPORT | Two processes exchange messages over real internet protocols, one of them a browser | |
| 22 | RENDER-2D | Thousands of crisp sprites, drawn without paying for anything 3D | EFFECT-GRAPH M3 and APPLE-PRESENTATION M3 to start; SIM-2D M1 for its last milestone |
| 23 | SESSION | Two to four players share one run and keep seeing the same crowd, and a windowed game is stepped by its match | SIM-2D M3 to start; TRANSPORT M1 for its second milestone and TRANSPORT M2 for its socket milestones; RENDER-2D M5 for its last |
| 24 | GAME-UI | Health bars, timers, menus and a join screen drawn in the game's own frame, and one iPhone shell for every game | RENDER-2D M5 and APPLE-PRESENTATION M6 |
| 25 | SWARM | The swarm-survival game itself: its name, rules, characters and art | SHOWCASE M8 to start, with the owner's pick of a concept; SIM-2D M3, SESSION M1, RENDER-2D M5 and GAME-SPLIT M4 for the first slice; GAME-UI M4 for the whole run alone and GAME-UI M6 for the iPhone simulator; SESSION M4 for a rehearsed match and SESSION M7 for co-op on sockets |
| 26 | REPO-RULES | `./dev check` refuses the code shapes the code review found, bulk media leaves the checkout, and names and layout are put right once | Nothing to start; every other track 4 module's fixes, and track 6's, for the rename and the tidy-up |
| 27 | REVIEW-ENGINE | The review's findings in the simulation, the sea, cooked assets and the cooker are fixed, and the iPhone app gets the textures and buffers its hardware has | Nothing to start; REPO-RULES M1 for its errors |
| 28 | TEST-SUPPORT | Engine tests get a GPU device and shared fixtures from one place, and a test that needs a GPU says so and is skipped with a reason where there is none | REPO-RULES M1 to start; HOST M1, GAME-SPLIT M2, ATMOSPHERE M9, APPLE-TILE M10, APPLE-PRESENTATION M3, EFFECT-GRAPH M3 and CULLING M3 for the renderer's tests; GATE-TIME M2 and M6, DEAD-CODE M2 and REVIEW-RENDER M2 for adopting the test kit; nothing for the small target and the block on writing a test |
| 29 | REVIEW-RENDER | The review's findings in the renderer are fixed with no pixel changed: one writer for each light buffer, GPU records held by test, typed pass names, typed errors | ATMOSPHERE M9, APPLE-TILE M10, APPLE-PRESENTATION M3, EFFECT-GRAPH M3 and CULLING M3 to start; REPO-RULES M1 for its errors |
| 30 | HOST | The Mac window, the browser page, the iPhone app, captures and tests get their device and their fault, pause and recovery behaviour from one place, and a game is hosted through one contract | CREATOR M3, APPLE-TILE M10, APPLE-PRESENTATION M3, EFFECT-GRAPH M3, CULLING M3, REVIEW-ENGINE M1 and REPO-RULES M1 |
| 31 | GAME-SPLIT | The showcase game moves to its own repository and uses only the engine's public API, and the engine proves itself without it | REPO-RULES M1 and TEST-SUPPORT M1 to start; REVIEW-ENGINE M3, CULLING M3, EFFECT-GRAPH M3 and APPLE-PRESENTATION M3 for the boundary; HOST M4 to cut the game free; SHOWCASE M8 for the move; CLOUDS M12, GAME-UI M6, PUDDLES M6, RENDER-2D M5, ATMOSPHERE M8, CONTENT-LOOP M2, SESSION M6 and TRANSPORT M2 for the rename |
| 32 | RAY-TRACING | On the Mac, the Sun and lamps cast ray-traced shadows that are right at contact and soften with distance, and props are grounded by ray-traced ambient occlusion; a person turns them on with a launch setting, and where rays cannot run the scene draws shadow maps as today | CULLING M3 and CONTENT-LOOP M2 to start; ATMOSPHERE M5, APPLE-PRESENTATION M3 and SHOWCASE M5 for shadows; EFFECT-GRAPH M3 and APPLE-PRESENTATION M2 for soft shadows; SHOWCASE M8 to close |
| 33 | MESH-SHADING | On the Mac, cooked meshes are culled and drawn a meshlet at a time by the GPU's object and mesh stages, with no draw call for each meshlet, turned on by a launch setting and never by default | CONTENT-LOOP M2 and COOKED-DRAW M1 to start; SHOWCASE M8 to close |
| 34 | GATE-TIME | A merge's check takes about five minutes where it took fifty: it runs only the fast tests its change can reach, side by side and each once, and the long tests and the whole workspace run on main after the merge | Nothing to start; TEST-SUPPORT M2 for the runs that happen twice; TEST-SUPPORT M6 for the guard rules; WATERS M7 for the water scene's slow tests; REVIEW-RENDER M2 for the reference tests; DEAD-TOOLS M2 for the contributor guide |
| 35 | DEAD-TOOLS | Scripts, benches and records that nothing runs or reads are gone and the check refuses new ones, no tool writes into `docs/evidence`, and each module's record of what it copied is tracked in Git | Nothing to start; TEST-SUPPORT M2 for its last milestone |
| 36 | DEAD-CODE | Engine code that nothing calls is gone, and a scene contribution no longer writes empty methods | GATE-TIME M1 to start; GATE-TIME M6 for the contribution's defaults; WATERS M7 for the water scene's settings; HOST M3 for the game's projection; REVIEW-RENDER M3 and GATE-TIME M6 for the two helpers |

Modules are in build order. Rows 1 to 13 are the road to the V1 exit, 14 to 19 are water and clouds, 20 to 24
are the 2D and online engine work, which is built beside V1, `TRANSPORT` last and only when a builder slot is free, and 25 is the swarm game, which starts after V1. Rows 26 to 31 are [track 4](#track-4-the-code-review-and-the-games-separation), the code review's fixes and the game's separation, built beside the rest. Rows 32 and 33 are [track 5](#track-5-ray-tracing-and-mesh-shaders-on-the-mac), ray tracing and mesh shaders on the Mac, built beside the rest and never ahead of a V1 milestone. Rows 34 to 36 are [track 6](#track-6-the-cleanup-of-20-november), the cleanup of 20 November, built beside the rest by Claude builders. `Needs` names the milestones a module waits on in other modules; each milestone's `needs` in
its done-when file is the authority. Where each module stands is on [the status page](STATUS.md), which the
loop writes.

This is the project plan the orchestrate loop builds from: the modules, what each owns, the build order, the
interfaces between them and a brief for each module not yet planned. First written 10 November 2026.
[The brief](BRIEF.md) owns the purpose and the V1 exit. [The architecture brief](../design/ARCHITECTURE-BRIEF.md)
owns the engine-wide responsibility map (F1–T2) and [CONTRACTS](../design/CONTRACTS.md) owns shared invariants. A
module here is a unit of build ownership: one builder can work on it without touching the others, and each kind of
state has exactly one module in charge of it. A swarm-survival game with 2D and online multiplayer is
[track 3](#track-3-the-swarm-survival-game-after-v1), added the same day from a second request, quoted there.
Its engine modules are built beside V1 and the game after it, by
[the owner's decision of 14 November](#the-split-of-14-november-2026).

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

**Track 2, water and clouds.** Water is one system built by four engine modules, `SEA`, `WATERS`, `RIVERS` and
`PUDDLES` ([water as one system](#water-as-one-system)), and `CLOUDS` is one, all built in Thrum on the foundation
track 1 lays. They are finished here, not in Broadsides; [the reasons](#where-sea-and-clouds-are-finished) are below.
Track 2 adds two foundation modules, `EFFECT-GRAPH` and, for water's mirrored scenery, `SCENE-VIEWS`
([design](SCENE-VIEWS/DESIGN.md)), and asks track 1 for three things it was already going to build. `SEA`, `WATERS` and `CLOUDS` are planned and start under their plans' assumptions on the owner's words of 13
November ([SEA](SEA/PLAN.md#open-for-the-human), [WATERS](WATERS/PLAN.md#open-for-the-human),
[CLOUDS](CLOUDS/PLAN.md#open-for-the-human)); `RIVERS` and `PUDDLES` are planned too ([RIVERS](RIVERS/PLAN.md#open-for-the-human), [PUDDLES](PUDDLES/PLAN.md#open-for-the-human)).

**Track 3, 2D and online play, then the swarm game.** An original swarm-survival game that proves 2D and online
multiplayer, set by the owner in [the brief](../design/NEXT-GOAL-SWARM-GAME.md). Six new modules. Five are
reusable engine work, built beside V1 since [14 November](#the-split-of-14-november-2026); the sixth, `SWARM`,
is the game and starts when the exit is reached. None gates the exit. They are listed in
[their own section](#track-3-the-swarm-survival-game-after-v1), so the tables below stay the road to the exit.

## Modules and what each owns

| Module | Gate | Owns (state and code) | Design or brief |
|---|---|---|---|
| `PLATFORM-RUNTIME-IOS` | G3b | The iOS shell: the UIKit lifecycle, touch, the render owner and its surface lease, rotation, device-loss recovery, and the scene extent that follows the oriented surface | [Design](PLATFORM-RUNTIME-IOS/DESIGN.md) |
| `PLATFORM-RUNTIME-MAC` | G3b | Qualification of the Mac surface, present and cadence path | [Design](PLATFORM-RUNTIME-MAC/DESIGN.md) |
| `T2-CAPTURE` | T2 | Engine capture: the submitted-output tap, its readback and writer, the capture modes and their evidence | [Design](T2-CAPTURE/DESIGN.md) |
| `COOKED-DRAW` | G5 | The direct cooked draw path and the selection between it and the indirect path | [Design](COOKED-DRAW/DESIGN.md) |
| `APPLE-TILE` | G3t | Recipe selection from the capability registry, the frame lifecycle and its one split scene/output graph, the pre-tone scene-linear input, the native Metal segment | [Design](APPLE-TILE/DESIGN.md) |
| `APPLE-PRESENTATION` | G4 | The view record (jitter, matrices), motion and reactive targets, history keys and acceptance, reconstruction, render extent and generation transitions, exposure retention, EDR present, the display record, the quality governor, pipeline preparation | [Design](APPLE-PRESENTATION/DESIGN.md); [brief for M4–M11](#apple-presentation-m4m11-g4) |
| `ATMOSPHERE` | G3s; track 2 from M7 | The one sky, Sun, aerial perspective, night and stars; the environment identity; the direct-Sun path to receivers; the air's optics for a haze the weather sets; the view volume, which integrates air, haze and a provider's medium jointly near the camera, with its history | [Design](ATMOSPHERE/DESIGN.md) |
| `BASELINE-GUARD` | T2 | The exact-baseline registry, the verifier that the main-branch check `exact-images` runs, and the reference guard in `./dev check`. Retained references stay with each baseline's owner | [Design](BASELINE-GUARD/DESIGN.md) |
| `CULLING` | G3c | Instance visibility lists, occlusion history, the culling mode and its resolution, the mesh-bucket route and Sun-caster culling | [Design](CULLING/DESIGN.md) |
| `CONTENT-LOOP` | G5 | Cooked products, the manifest, runtime mapping and GPU residency of cooked meshes, textures and volumes; leases of resident products to other modules; the geometry of cooked mesh rows | [Design](CONTENT-LOOP/DESIGN.md) |
| `CREATOR` | G6 | The typed controls model and its views, parameter files, the app runner, sky kit and starter, the creator docs, the profile window and profiler rows | [Design](CREATOR/DESIGN.md) |
| `SHOWCASE` | G7 | The valley's source in `examples/v1-game/assets/showcase` and the six stop presets, by [ruling](#ruling-on-the-valley-fix); the exit scene in `examples/v1-game` (its content, layout and generator, prop sources and mover rule), the routes, the exit configuration, the launcher `scripts/run_v1_showcase.py`, the V1 exit record | [Design](SHOWCASE/DESIGN.md) |
| `EFFECT-GRAPH` | track 2 | Graph resource kinds and cadence that simulated effects need; how an engine module declares its own passes; the list of contributions a scene holds | [Design](EFFECT-GRAPH/DESIGN.md) |
| `SEA` | track 2 | The wave model and its f64 oracle (`thrum-sea-model`); the wave field on the GPU: long-wave phases, spectral cascades and the field cache, evaluated each frame (`thrum-sea`). After its last milestone both crates are `WATERS`' to extend | [Design](SEA/DESIGN.md); [brief](SEA/BRIEF.md) |
| `WATERS` | track 2 | The water map and its one sampling function, gains, the shore transform and the breaker section, the f64 surface model, the interaction, foam and caustic references and the light factor at a camera (`thrum-water-model`); the map on the GPU, the surface from both sides and its frame contract, optics and the gather, the mirror lookup, the view from under water, the caustic field and its Sun-path factor, shafts, interaction waves, foam stocks, the body-query ring (`thrum-water`); the water example scene (`examples/water`) | [Design](WATERS/DESIGN.md) |
| `SCENE-VIEWS` | track 2 | Secondary views of the scene: slots and requests, a view's record, its pass and fragment entry, its targets, and which contributions draw in it; a view's culling, Sun cascades, cluster table and aerial volume, built in their owners' files by [ruling](#seam-rulings-for-scene-views); the mirror example (`examples/mirror`) | [Design](SCENE-VIEWS/DESIGN.md) |
| `RIVERS` | track 2 | The flow solver, its brink rule and energy account, and the river file's writer (`thrum-river-model`); a river's flow samples and gauge, brink ends, fall records and the sheet's reference, the plunge's births and impulses, the flow bands and their gains, as files of its own and [ruled changes](#seam-rulings-for-rivers) in `thrum-water-model`, `thrum-water` and `thrum-sea`; the river selection and its ground in `examples/water` | [Design](RIVERS/DESIGN.md) |
| `PUDDLES` | track 2 | The tree of hollows, the fill step and its account, the canopy grid and exposure, the ground's stores and the wetness function, the ring response, and the rain state a game keeps (`thrum-rain-model`); the hollows kind in the map, the rain bands, the fine interaction level, the film over standing rain water and the wetness provider, as files of its own and [ruled changes](#seam-rulings-for-puddles) in `thrum-water-model`, `thrum-water`, `thrum-sea` and the forward shader; the rain selection and its ground in `examples/water` and `examples/v1-game` | [Design](PUDDLES/DESIGN.md) |
| `CLOUDS` | track 2 | The weather field, its state's step and the f64 references (`thrum-cloud-model`): every condensate from the cloud tops to mist, fog and low cloud on the ground, and how much haze the air holds; the weather map, cloud density and noise, the Sun-path optical-depth product in its cloud and ground bands, the cloud layer and its history, the sky overlay, placed-volume sampling, the medium coefficients it writes for the view volume (`thrum-clouds`); the cloud selection in `examples/v1-game` and `examples/water` | [Design](CLOUDS/DESIGN.md); [brief](CLOUDS/BRIEF.md) |

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
| Live camera, lights and cascaded shadows | G3a, closed before the loop; shadow quality on the route in `ATMOSPHERE` M6; the far ground under the exit scene's balloons in `ATMOSPHERE` M10 |
| GPU culling | `CULLING` M3 |
| MetalFX, EDR and a quality governor; no pipeline compilation during play | `APPLE-TILE` M10–M12; `APPLE-PRESENTATION` M4–M12 |
| Display-link interpolation | G3b's function rows, closed before the loop; Mac cadence in `PLATFORM-RUNTIME-MAC` |
| Profiler and native/browser demo UI | `CREATOR` M1 and M2 |
| 10,000 entities and 1,000–2,000 movers as the representative scene | `SHOWCASE` M5 and M6, with `CULLING` and `CONTENT-LOOP` |
| Headless corrected-run replay | G2S, closed before the loop |
| A fresh agent builds a small scene from public docs alone | `CREATOR` M3 |
| Outstanding physical-device runs reported | `SHOWCASE` M8 |

<a id="who-judges-frame-cadence"></a>

**Who judges frame cadence on the culled valley (with `CULLING` and `SHOWCASE`), 17 November 2026.** `CULLING`
M3's review left M3.7 and M3.8 unjudged for "full-refresh smoothness", from submission intervals in a loaded
capture, and its builder asked for a paused-builder sitting
([review](../../.orchestrate/evidence/CULLING/M3-review.md)). The brief's bar, "smooth at the display's full refresh", has one
judge for the showcase, and it is not `CULLING`.

- **`SHOWCASE` judges it**, on the exit scene, which is the valley populated and drawn in `CULLING`'s default
  mode: [cue S5](SHOWCASE/DESIGN.md#cues), with the capture-off whole-frame record that names each late frame's
  cause (M7.9), the OS-reported presented intervals on the built-in panel (M8.6) and a visible Chrome window's
  frame callbacks (M8.7). No item of `SHOWCASE` changes.
- **`PLATFORM-RUNTIME-MAC`** owns the Mac's surface, present and cadence path, qualified on the cooked valley in
  its M2 and M3. **`APPLE-PRESENTATION`** owns the governor, which holds the rate when a frame costs too much.
- **`CULLING`** owns what culling adds to a frame, recorded as paired rows and tripwires, as gate G3c words it.
  Its items judge which geometry each submitted frame holds, and none judges cadence or takes a sitting
  ([its design](CULLING/DESIGN.md#frame-cadence)). When `SHOWCASE`'s record names a culling pass as the cause of
  a late frame, that is a proposal in `CULLING`'s log.
- **Between the two.** Until `SHOWCASE` M7, no cadence figure exists for the valley with culling on, and none is
  claimed. The intervals of an engine capture taken under load, with its observer on, are not one.

## Build order

Riskiest and most depended-on first. A milestone's `needs` in its done-when file is the authority; this is the
reasoning behind those entries. The project settings allow one Codex builder, so the lanes below say what is safe to run beside `APPLE-TILE` if the owner allows a second builder, and what a lone builder picks up while `APPLE-TILE` or the Mac qualification waits on an attended session.

1. **First.** `PLATFORM-RUNTIME-MAC` M2. `APPLE-TILE` M0, then M1–M3. `APPLE-TILE` is first because everything
   after it reads its graph, its recipe selection or its pre-tone input.
2. **Beside them, in other files.** `CREATOR` M1 (`thrum-app`, `thrum-controls`, `examples/starter`, docs).
   `ATMOSPHERE` M5 (`thrum-render/src/atmosphere`, the forward light group, the contribution seam), then its
   M9 (the aerial pass at the model's ground), which waits for nothing and which `WATERS` M2 waits for.
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
   (MetalFX), M9, M10 and M12 (prepared paths, cooked shader libraries, the pipeline archive). `CONTENT-LOOP` M3 and `CREATOR` M3 close at any point here. `CONTENT-LOOP` M1
   (leases) starts earlier, after `APPLE-PRESENTATION:M2`, and its M2 (cooked mesh rows) after `CULLING:M3`.
7. **The exit.** `SHOWCASE` M5 and M6 (the populated valley and its culling) start once `CONTENT-LOOP:M2` is
   Done, beside the work of step 6. `APPLE-TILE` M12 and `APPLE-PRESENTATION` M11 (paired proof) come next, then
   `SHOWCASE` M7 and M8.
8. **Track 2.** `SEA`, `WATERS` and `CLOUDS` each start with a CPU milestone that needs nothing from the renderer:
   `SEA` M1 (the wave model), `WATERS` M1 (where water lies, after `SEA:M1`) and `CLOUDS` M1 (the weather). `SEA`
   M2 (the wave field on the GPU, on `EFFECT-GRAPH:M2`) and `CLOUDS` M2 (shade with no cloud drawn, on
   `EFFECT-GRAPH:M2` and `ATMOSPHERE:M5`) can run beside track 1 in files of their own. The first frames, `WATERS`
   M2 and `CLOUDS` M3, follow `EFFECT-GRAPH:M3`, where [the readiness conditions](#where-the-ports-readiness-conditions-are-met)
   are met. Their closing milestones need the governor. Water is built in the order
   [water as one system](#water-as-one-system) gives: the wave field, then still water with the shared system,
   then running water, then rain. `RIVERS`' flow solver is CPU work in files of its own and can run beside
   `WATERS` once `WATERS:M1` is Done, and so can `PUDDLES`' two CPU milestones. `SCENE-VIEWS` is renderer work that only `WATERS` M14 waits for, on its M3, so it
   can be built at any point after `CULLING:M1`, `ATMOSPHERE:M5` and `EFFECT-GRAPH:M3`. It builds beside
   `CULLING` M3 in that module's files and integrates on top of what has landed.
9. **The atmosphere as a whole** ([below](#the-atmosphere-as-a-whole)). `CLOUDS` M13 (mist, fog and low cloud
   on the CPU) follows its M1 and needs nothing. `ATMOSPHERE` M7 (the view volume, haze and shafts) and M8 (a
   test fog) follow its M6, so they wait for TAAU and for G3s to close, in `thrum-render`'s atmosphere files.
   `CLOUDS` M14 to M16 (mist near the camera, far mist and low cloud, mist through a day) follow its M10 and
   `ATMOSPHERE:M8`.

**Every module is planned.** `APPLE-PRESENTATION` M4 to M12 were the last, on 14 November 2026.

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

Water's first frame, `WATERS` M2, waits only on `EFFECT-GRAPH:M3`, which itself waits on `APPLE-TILE:M3`, and on
`SEA:M2`. How water looks under TAAU is its M10, after `APPLE-PRESENTATION:M3`. `CLOUDS` is the same: its first frame is its M3, on
`EFFECT-GRAPH:M3`, and its look under TAAU is its M9.

`EFFECT-GRAPH:M3` proves that an effect's motion, reactive coverage and reset reach reconstruction's inputs. How
an effect looks after reconstruction (shimmer, ghosting, a cut) is accepted in `WATERS`' and `CLOUDS`' own
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
| **Effect passes and resources**: a contribution declares its own compute and render passes at five named frame stages, with 3D, cube and mipped storage-array textures, persistent, ping-pong and world-anchored wrapping state stepped by the tick and committed only with a submitted frame, a runtime cadence, and quality knobs. A scene holds an ordered list of contributions | `EFFECT-GRAPH` → `SEA`, `WATERS`, `CLOUDS`, `ATMOSPHERE`; `RENDER-2D` later | [EFFECT-GRAPH](EFFECT-GRAPH/DESIGN.md#mechanisms) |
| **Display phase**: with the tick, the tick duration and the interpolation alpha, each frame gives every effect where its consumed target lies in ticks after the previous committed tick, with no stop at the committed one and at most 50 ms past it. The consumed target is the time the frame's host observed plus its bounded lead toward the platform's target, and is where alpha is taken today. A frame's display time is its tick less one plus the phase. It is a time of the world's, not of the host's clock: the world time of the tick before the frame's, plus the host time from the frame's tick falling due to the consumed target. So it holds none of the time a clock was paused or overloaded, equal intervals between consumed targets are equal steps of it while the clock runs, and an interpolating frame's record carries its tick's due time. An effect is given no host time. Alpha is the phase held to 1 and is the same function as before; sampling never advances authority; the tick a frame shows was due at or before its observation, so a world on its own thread shows a delayed frame an earlier pair; a quantity shown past the committed tick is a presentation-only prediction from committed state, carried a tick at a time by its own step | The world's clock in core and the frame host, built in `EFFECT-GRAPH:M4` → `CLOUDS` from its M2; `SEA`, `WATERS`, `RENDER-2D` and games when their owners plan it | C02, [EFFECT-GRAPH](EFFECT-GRAPH/DESIGN.md#the-timing-model), [the ruling below](#seam-ruling-for-effect-graph-m4) |
| **Entity visibility**: an entity drawn through the instance field is culled per instance and draws a whole mesh row from the scene's mesh selection; a cooked placement is culled per meshlet; nothing is in both. Cooked meshes reach entities as mesh rows, and an edited mesh replaces its row's geometry while the game runs, switching in the frame the content publication names | `CULLING` (the route and its culling), `CONTENT-LOOP:M2` (cooked meshes as rows, by the ruling below) → `SHOWCASE` | [CULLING](CULLING/DESIGN.md#one-visibility-authority-per-entity), [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#cooked-mesh-rows-m2) |
| Pre-tone scene-linear input with matching exposure, extent and sample identity | `APPLE-TILE:M3` → `APPLE-PRESENTATION` | [APPLE-TILE](APPLE-TILE/DESIGN.md) |
| **Temporal contribution**: every drawer writes motion from unjittered positions, marks invalid history, writes the reactive target, and honours the frame's reset flag | `APPLE-PRESENTATION` M1–M2 → `ATMOSPHERE`, `CONTENT-LOOP`, `CULLING`, `WATERS`, `CLOUDS` | C07, [APPLE-PRESENTATION](APPLE-PRESENTATION/DESIGN.md#seams-for-thrum-core-proposals) seams 1–4 |
| **Environment identity and products**: source epoch and time, the true Sun (direction, raw illuminance, radius), and read-only sky, transmittance, aerial and reflection products with their WGSL lookups | `ATMOSPHERE:M5` → forward contributors, `WATERS`, `CLOUDS` | [ATMOSPHERE](ATMOSPHERE/DESIGN.md#environment-identity-and-products), C08 |
| **Sun-path optical depth**: one provider's `tau` toward the Sun; V3 applies `exp(-tau)` once on each direct-Sun receiver. A provider may add a **ground band** that holds the whole path to the Sun: a receiver inside it reads the band at its own position in place of the field, so land inside fog loses its Sun and an upper cloud is counted once ([ruling 7](#seam-rulings-for-clouds)). The field is CPU bytes or, by [ruling 1](#seam-rulings-for-clouds), a texture the provider writes on the GPU, read along a path direction the provider keeps continuous at every Sun elevation, and fading across a border to a far value for receivers outside its region | `CLOUDS` from its M2, the ground band from its M14 (diagnostic stand-ins in `ATMOSPHERE:M5` and `M8`) → `ATMOSPHERE` receivers and its view volume, `WATERS` | C08, [ENVIRONMENT](../design/modules/ENVIRONMENT.md), [ATMOSPHERE](ATMOSPHERE/DESIGN.md#environment-identity-and-products), [CLOUDS](CLOUDS/DESIGN.md#the-suns-path-one-optical-depth-product) |
| **Sky overlay and haze scale**: a cube of cloud `(S,T)` around the camera, which `ATMOSPHERE` composes over the sky before it filters its reflection and diffuse products, and, in a scene without the view volume, one number a frame that scales the air's solar in-scatter. So ambient light, reflections and haze follow the clouds, and the sea reflects them through the environment reflection product | `CLOUDS:M8` → `ATMOSPHERE`'s products, and through them the forward pass and `WATERS` | C08, [CLOUDS](CLOUDS/DESIGN.md#the-sky-the-world-sees), [rulings 3 and 4](#seam-rulings-for-clouds) |
| Ordered medium segments `(T,S)` and the reflection provider. Condensate is split along a view ray by distance, at a handover: nearer, it is in `ATMOSPHERE`'s view volume, integrated jointly with the air; beyond, it is the trace's segment, which stops at the scene's depth and is composed with the air once ([CLOUDS](CLOUDS/DESIGN.md#the-visible-layer)). The two shares sum to one. Water composes its own air, surface and water segments in its one pass and **writes scene depth**, so a medium drawn after it stops at the water; `WATERS` precedes `CLOUDS` in a scene's contribution list | `WATERS`, `CLOUDS` → V3 composition in `ATMOSPHERE` and the forward pass | C08, [WATERS](WATERS/DESIGN.md#optics) |
| **View volume and medium coefficients**: a scene may declare the view volume, and then the lent aerial `(T,S)` to a point is the joint transport of air, haze and a provider's medium near the camera, with the same arguments and units. A provider writes three coefficients (extinction, a solar kernel that already holds its own optical depth and phase, an ambient source) at the sample points the volume names, weighted by its near weight; `ATMOSPHERE` multiplies the kernel by its Sun light and the scene's shadow, sums with the air and integrates once. A ray that leaves the volume takes its terminal pair once: the sky behind it becomes `R·L + ΔS`, and stars and a disc drawn by itself are multiplied by `R` alone. A scene that declares none is unchanged | `ATMOSPHERE:M7` (the volume, with air and haze), `ATMOSPHERE:M8` (a medium) → every reader of the lent aerial operation; `CLOUDS:M14` provides the medium | C08, [ATMOSPHERE](ATMOSPHERE/DESIGN.md#the-view-volume-haze-fog-and-light-near-the-camera), [CLOUDS](CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud), [rulings 6 to 8](#seam-rulings-for-clouds) |
| **The air's haze**: the aerosol's amount, as the clear air's visual range, and its kind, as an Ångström exponent. The weather state holds both; `ATMOSPHERE` takes them as inputs of its air profile and recomputes its charts, with today's constant and an exponent of 0 as the defaults | `CLOUDS:M13` (the range), `CLOUDS:M16` (the exponent), `ATMOSPHERE:M7` (the inputs) → `ATMOSPHERE`'s charts | [ATMOSPHERE](ATMOSPHERE/DESIGN.md#the-view-volume-haze-fog-and-light-near-the-camera) |
| **Surface field**: the height of the ground and the level of any water, on a grid, which a game gives the weather as data so that mist can pool in hollows and fog can keep to water. The engine never reads terrain meshes or the water map for it | games → `CLOUDS` | [CLOUDS](CLOUDS/DESIGN.md#condensate-that-meets-the-ground-mist-fog-and-low-cloud) |
| Cooked products and placements | `CONTENT-LOOP` → games, `CREATOR`'s sky kit | C05, [G5 decision](../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md) |
| **Content publication**: one cooked generation changes through every reader a scene has (the placement set, each lease, each cooked mesh row) in one submitted frame. The scene's residency owns it: each reader prepares under its own bound without showing, the publication is ready when the slowest reader is, a ready publication is not superseded, and one a reader cannot take is refused for all. This is C05's qualified visual preview for cooked products; C05 is unchanged | `CONTENT-LOOP:M1` (placements and leases), `CONTENT-LOOP:M2` (rows) → `CULLING`'s route, lease consumers (`SEA`, `CLOUDS`), games | C05, [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) |
| **Leases**: an engine module names a set of cooked textures and volumes; the scene's one residency uploads them and lends read-only views, as a whole set, only after the frame that recorded the last copy is submitted, in the frame the content publication switches placements and rows. Each product carries identity and a change serial; a consumer keeps a change pending until a submitted frame that advances history has used it. Placement and meaning are the consumer's inputs | `CONTENT-LOOP:M1` → `CLOUDS` (density volumes), `SEA` (foam texture); `RENDER-2D` and `GAME-UI` later | [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#leases-m1) |
| **Body query**: one batched asynchronous GPU query for all water, one to two ticks late, with support and age. The game posts points with a footprint, a use tick and a body ID and generation in its tick page, and drains answers (the body of water with its depth and current, or dry ground; height, slope, surface velocity and time derivative, valid at the use tick, with that tick, the times they were made from and a support flag per layer) from a mailbox; nothing waits on the GPU, and a request over capacity is a counted refusal | `WATERS` → games and P3 | C09, [WATERS](WATERS/DESIGN.md#bodies-and-the-query) |
| **Wave field**: the wind's waves as analytic long-wave components and spectral cascades with their slope and Jacobian moments, evaluated every frame at the display's time and keeping nothing between frames; an f64 evaluator of the same waves; the spectrum's energy in each layer's band for a given fetch | `SEA` → `WATERS` | [SEA](SEA/DESIGN.md#the-ambient-sea) |
| **Water map**: a game declares bodies of water (a sea by its level, a basin by a seed and a level, a chart by grids of level, current and loss with a footprint, a source end and a mouth, and, by [ruling 1 for `PUDDLES`](#seam-rulings-for-puddles), hollows by a rectangle with its own bed, a label for each cell and a level for each hollow) over a bed; one function, the same on the CPU and the GPU and independent of the camera, gives the body, level, bed, depth, current and wave gains at any point; a stage, inside a declared range, moves a body's level. Bodies meet only at a chart's ends, where the chart's row is mixed to the joined body's, so level, current and waves are continuous across a join for every reader; preparation refuses a chart its banks do not hold and a basin that spills or merges. Each body also has a shore margin its waves wash, with its wave support, and, where a profile has surf, a shore transform that turns, shortens and raises its long waves over the bed. A chart is what a flow solver writes, and a hollows body is where puddles stand | `WATERS` → games, `RIVERS`, `PUDDLES` | [WATERS](WATERS/DESIGN.md#the-water-map), [PUDDLES](PUDDLES/DESIGN.md#hollows-the-tree-and-the-point-test) |
| **Secondary view** (water's mirror view): a contribution declares slots when the scene is prepared and asks each frame for a view from a rigid camera it names on the main camera's vertical (its two horizontal coordinates, at any height, facing any way; any other is refused by name), with an optional clip plane, from which its air may start when the plane is level, and a least size. The view holds the scene's opaque geometry beyond the clip plane, entities and cooked placements, shaded by the scene's own shader with visibility, Sun cascades, local-light clusters and air made again for that view, in the scene target's encoding for it (radiance through the view's air, pre-exposed; for a mirror that air starts at the plane, and the reader applies the air from its own camera to the plane itself), with coverage and depth, at the frame's jitter phase, and no sky, water, fog, after-opaque effect or history. The asking contribution reads its own slot's colour, depth and record in its own passes. A scene that declares none compiles and draws as before, but for where [ruling 3](#seam-rulings-for-scene-views) lays its height map out | `SCENE-VIEWS:M3` (entities from its M1, cooked placements from its M2) → `WATERS:M14`; `PUDDLES` and `RIVERS` through `WATERS` | C07, [SCENE-VIEWS](SCENE-VIEWS/DESIGN.md#slots-and-requests), [WATERS](WATERS/DESIGN.md#mirror-views-scenery-the-camera-cannot-see) |
| **Sun-path factor**: a water provider's factor on a receiver's direct Sun, read at the receiver's own position beside the cloud factor: `1 + D (C − 1)`, the beam's share times the caustic field's departure from even light, and exactly 1 at or above the water's level and outside the provider's window. With it goes the point at which a receiver under water reads its Sun visibility, where its refracted Sun ray leaves the water. This is C08's caustics, an illumination contribution whose owner is `WATERS`, applied once by V3's direct-Sun path | `WATERS:M16` → `ATMOSPHERE`'s direct-Sun receivers in the forward pass, and every contribution that compiles the scene's shader | C08, [WATERS](WATERS/DESIGN.md#light-under-water-caustics-shafts-and-the-meter), [ruling 5](#seam-rulings-for-waters) |
| **Aloft casters**: a scene marks a mesh row of its entities as aloft. The row casts into the Sun cascades as before and is left out of the top-down caster height map, which holds the ground and what stands on it, so beyond the cascades it casts no shadow. A row is not aloft unless marked, and a scene that marks none draws the same bytes | `ATMOSPHERE:M10` → `SHOWCASE:M5`; any scene with entities in the air | [ATMOSPHERE](ATMOSPHERE/DESIGN.md#what-the-caster-height-map-holds), [the ruling](#seam-ruling-for-atmosphere-m10) |
| **Lent Sun visibility**: with the environment products, the scene's Sun visibility at a world point as one tap of its cascades and height map, for an effect's own volume samples; 1 in a scene with no shadows | `ATMOSPHERE` (built by `WATERS:M16`, [ruling 6](#seam-rulings-for-waters)) → `WATERS`' shafts | C08, [WATERS](WATERS/DESIGN.md#light-under-water-caustics-shafts-and-the-meter) |
| **Meter medium factor**: a camera request may carry three numbers that multiply the incident light the camera's meter reads, for a camera inside a medium; 1 by default, and with 1 the resolved exposure sample is unchanged to the bit. `thrum-water-model` gives the factor for a camera under water, and the game sets it | `APPLE-PRESENTATION` (V4; built by `WATERS:M16`, [ruling 7](#seam-rulings-for-waters)) → games | C07, [WATERS](WATERS/DESIGN.md#light-under-water-caustics-shafts-and-the-meter) |
| **River**: everything one steady solve of the map's bed writes, as a stored file with a key: one chart for each reach, each holding one or more flow samples (level, current, loss and flow gains) and the gauge level of each; brink ends where the ground drops away; and a fall record for each fall. A chart's loss is the power that breaks the surface, in watts a square metre. A game declares a river's rectangle, sources, bodies and flow samples; a build step or a native game's preparation solves it, and a page only loads it | `RIVERS:M1` (one sample), `RIVERS:M2` (falls), `RIVERS:M3` (samples and gains) → games, and `WATERS`' map through [rulings 1 to 3](#seam-rulings-for-rivers) | [RIVERS](RIVERS/DESIGN.md#what-a-solve-writes) |
| **Fall record**: along a fall's lip, for each flow sample, the water's path with the time it takes, its discharge for each metre of lip and its launch speed; the reach above and the body below; stable IDs for its segments; and so the fall's power, which a later mist may read | `RIVERS:M2` → `WATERS`' pass (the sheet, by ruling 7), games | [RIVERS](RIVERS/DESIGN.md#the-falls-sheet) |
| **Impulse records**: a place, a vertical impulse, a radius, a tick and a stable ID, which raise interaction waves on any water; refused over capacity in ID order and counted. A footfall in a puddle is one; a raindrop is not, since rain's rings are [wave layers](PUDDLES/DESIGN.md#rain-rings) | games, `RIVERS` (a fall's plunge) → `WATERS` | [WATERS](WATERS/DESIGN.md#interaction-waves) |
| **Rain record**: the rain's flux and the drying rate, in kilograms a square metre a second, uniform over a scene and held over a tick, which a game sets in its tick page, and the flux of each tick of the last second, which the rain state keeps and the rain bands read; with it a game's shelters, each a set of triangles with the share of rain it lets through, as prepared content, up to two in a column. `CLOUDS` may set the record once it has rain | games → `PUDDLES`, and through it `WATERS`' rain bands | C09, [PUDDLES](PUDDLES/DESIGN.md#rain-and-what-it-reaches) |
| **Ground wetness**: the pores' fill and the surface water with its cover at a world point, one function in f64 and in WGSL of prepared fields (exposure, slope, relief) and the rain state a game steps, reading no camera. A forward surface reads it before it is lit, composed as the cloud factor is; with no provider a scene compiles the shader text and draws the bytes it does today | `PUDDLES:M2` (the function), `PUDDLES:M5` (the provider) → the scene's shader and the cooked material, `WATERS`' film, games | C09, [PUDDLES](PUDDLES/DESIGN.md#wet-ground), [ruling 5](#seam-rulings-for-puddles) |
| **Quality knobs**: each cadence and step budget is a runtime value with a declared history invalidation. None changes a physical profile or what a query returns | `WATERS`, `CLOUDS`, `ATMOSPHERE` → the governor in `APPLE-PRESENTATION:M7` | C04, port decision, [WATERS](WATERS/DESIGN.md#capabilities-recipes-limits-and-knobs) |
| Typed controls, parameter files, the scene shell on Mac and web | `CREATOR` → `SHOWCASE`, demo scenes for `WATERS` and `CLOUDS` | C11, [G6 decision](../design/decisions/G6-CREATOR-EXPERIENCE.md), [CREATOR](CREATOR/DESIGN.md) |
| **Frame timing feed**: completed frame samples with pass-name GPU timing, and one timing-demand switch on the frame host (on by default; off writes no timestamps and the sample says GPU time was not requested) | The frame runtime (`APPLE-TILE:M3` for pass names; the switch by the ruling below) → `CREATOR`'s profiler, frames reports, Mac qualification, capture | C12, [CREATOR](CREATOR/DESIGN.md#the-frame-timing-feed) |
| Engine capture modes and evidence, and a realtime capture's admission policy: `omit` by default, `wait` for a sequence that must keep every submitted frame | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE/DESIGN.md) |
| **Resolved selections**: each selectable path tells the game what it resolved to on this device (recipe, reconstructor, display mode, governor state, culling mode), so a run can report what it ran and a fallback cannot pass as the feature | `APPLE-TILE`, `APPLE-PRESENTATION`, `CULLING` → `SHOWCASE`'s exit report | C04, [SHOWCASE](SHOWCASE/DESIGN.md#the-exit-configuration-one-file-three-rows-reported-and-not-echoed) |
| **Exact baselines**: retained references, their owner, and the verifier that one run on main shares between every landing | Each baseline's owner (references), `BASELINE-GUARD` (registry, verifier, reference guard) → every module that edits the covered paths | [Exact baselines](#exact-baselines) |

**Seam rulings for `APPLE-PRESENTATION` M1–M3.** Its design proposed nine seams to Thrum Core. Under the loop the
module list settles them. Seams 1–4 are accepted as written there:
- seam 1 (submission-order history acceptance, `history.rs` and `runtime.rs`) is built in M2, after
  `APPLE-TILE:M2` has moved `runtime.rs`;
- seam 2 (the view-record change in RENDER-SPEC) is built in M1, and the builder refreshes the design hashes;
- seam 3 (the third forward target) is built in M2 across `contribution.rs`, `thrum-asset-gpu` and the sky
  pipeline, keeping COOKED-DRAW's path selection and history rules;
- seam 4 (sky and star motion at infinity) is built in M1 by the `APPLE-PRESENTATION` builder inside the
  atmosphere shaders, for motion output only.

Seams 5–9 were settled on 14 November 2026, when M4 to M12 were planned. Each is built by the
`APPLE-PRESENTATION` builder of the milestone named, inside state another module owns, with the behaviour
[its design](APPLE-PRESENTATION/DESIGN.md#m4-to-m12-settled-14-november-2026) defines. No item of the owning module
changes and no exact comparison is loosened. Once built, each belongs to its owner: later work there keeps its
tests passing.
- seam 5 (render extent and generation transitions; `PLATFORM-RUNTIME-MAC`'s render actor, the frame runtime,
  and `PLATFORM-RUNTIME-IOS`'s render owner) is built in M4. The surface lease on iOS is not changed, and iOS keeps
  preparing a new extent in its render owner between two frames. GPU work a module runs off the render thread on
  the game's device runs as a job on the frame host's preparation thread, so that a surface switch can exclude
  every other submitter. The Mac's cadence
  qualification (`PLATFORM-RUNTIME-MAC` M3) stands for the path it measured, and `SHOWCASE` M8's attended record
  measures the path as it ships;
- seam 6 (the platform display record, the EDR layer properties, and engine capture's tap reading the float
  target; `thrum-app`'s platform files and `T2-CAPTURE`'s tap) is built in M6. T2's records in the 8-bit mode
  do not change;
- seam 7 (the governor: missed ticks from the frame host, thermal state from the platform, value commits inside
  a profile generation, and the knob registry) is built in M7. The aerial lookup's cadence knob is built in
  `ATMOSPHERE`'s files by the M7 builder if that module has not declared one;
- seam 8 (MetalFX as a provider in `APPLE-TILE`'s native segment) is built in M8, after `APPLE-TILE:M11`, under
  [the boundary decision](../design/decisions/G3T-METAL-BOUNDARY.md);
- seam 9 (pipeline preparation: the library cook as a step beside `CONTENT-LOOP`'s cooker, the passthrough
  loader, and a pinned copy of wgpu's Metal backend) is built in M10 and M12. The pinned copy arrives in M10,
  where it carries stage records to shaders compiled ahead of time, and M12 adds the archive to it. From M10
  every module builds against that copy, and its patches leave the WGSL path and every exact baseline as they
  are. No shipping package links the cook, and nothing is posted upstream.

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

<a id="seam-ruling-for-atmosphere-m10"></a>
**Seam ruling for `ATMOSPHERE` M10.** `SHOWCASE` M5 found that a balloon in the air drops a dark strip on the
ground beyond the Sun cascades, which blinks on and off as it drifts: the top-down caster height map holds its
top, and the march reads that as a tower. The Sun's visibility at a receiver is `ATMOSPHERE`'s, so which casters
that map holds is too. The aloft mark is built by the `ATMOSPHERE` M10 builder in `thrum-render`'s scene mesh
table and Sun passes (`synthetic/{scene_source,scene_graph,shadow}.rs`), in the bucket route's caster draws
(`synthetic/mesh_route.rs`, `CULLING`'s) and, as one mark on the balloon row, in
`examples/v1-game/src/exit_scene.rs` (`SHOWCASE`'s), with the behaviour
[ATMOSPHERE](ATMOSPHERE/DESIGN.md#what-the-caster-height-map-holds) defines. `CULLING`'s items and exact
comparisons do not change, and an aloft row is left out the same way in every culling mode. `SHOWCASE`'s layout,
generator, routes and launcher do not change. No registered exact baseline moves. Once built, the
membership belongs to the Sun path and the mark to the exit scene.

<a id="seam-ruling-for-effect-graph-m4"></a>
**Seam ruling for `EFFECT-GRAPH` M4.** `CLOUDS` M2 found that the interpolation alpha is not the display's time:
it stops at the committed tick, and under a display lead the target is past that tick for part of every tick, so
drift taken by alpha takes a short step and a long one in turn. No interface offered the target's place. The
display phase is built by the `EFFECT-GRAPH` M4 builder in `thrum-core`'s clock, world lane and clocked world,
the frame host's world observer, effect time and frame sample in `thrum-render`, `thrum-app`'s sky world and
offscreen runner, the V1 game's world wrappers and its capture time record, with the behaviour
[EFFECT-GRAPH](EFFECT-GRAPH/DESIGN.md#the-display-phase-m4) defines. The phase describes the target the clock
consumes, the host's observed time plus its lead, so alpha is the phase held to 1 in every frame. It adds to
each file and changes two things. The offscreen runner measures its lead from the clock read it gives the world,
where it read the clock twice, so a capture consumes the platform's target exactly
([one read of the clock](EFFECT-GRAPH/DESIGN.md#one-read-of-the-clock), revised 16 November after the design
review). And a world on its own thread shows a delayed frame, one that polls after a tick due later than its
observation was committed, an earlier pair where it took the newest: the Mac window, Mac captures and the
iPhone app meet such frames, and the browser never does
([the frame's pair](EFFECT-GRAPH/DESIGN.md#the-frames-pair), settled 16 November after M4's first review, with
what each reader sees). Alpha as a function, the lead's 50 ms bound and its statuses, the Mac's render actor, the iPhone's
render owner, the clock's authority and every exact baseline stay as they are, and no item of `CREATOR`, the
platform modules, `T2-CAPTURE` or `SHOWCASE` changes. M4's second review found that the rule had been written
with display time on the host's clock, which fails after any pause, and
[the timing model](EFFECT-GRAPH/DESIGN.md#the-timing-model) (17 November) restates every rule about a frame's
time as one set, with C02. It changes no phase, alpha or pair that a frame is given. It adds one fact to each
producer: the clock sample states the host time its tick fell due, every implementer of the frame host's world
observer passes it on, and the frame's sample and the capture time record carry it, so `docs/CAPTURE.md`
gains a field and loses none. `thrum-app`'s sky world with a tick duration, which `examples/water` runs,
states its own clock's phase where it reported 1; its alpha and so the sea's images do not move. Readers
change nothing: `CLOUDS` reads the same phase, its design now says that display time is the world's, and its
items stand; readers of alpha are untouched. Once built, the phase belongs with
the clock and the frame host: `HOST` M2 to M5, `SESSION` M7 and later work there keep its tests passing, and a
[driven world](SESSION/DESIGN.md#12-a-driven-world) states its phase and its tick's due time from its driver's
position as a clocked world does from its clock. `HOST` was the other candidate and is not the owner: what a frame tells an effect
about time is `EFFECT-GRAPH`'s interface, and `HOST` waits on seven milestones. Readers are separate. `CLOUDS`
M2 takes its drift at the phase. `SEA`'s and `WATERS`' wave time, `RENDER-2D`'s sprites and a game's camera and
entities still read alpha and are uneven under a lead by the same arithmetic; each is for its owner's next plan,
the camera and entities with `HOST` M4's contract.

<a id="seam-rulings-for-sea"></a>

### Seam rulings for WATERS

Water draws after opaque geometry, and two things it needs there are not in `EFFECT-GRAPH`'s after-opaque
stage as designed. Both are built in `contribution.rs` and `synthetic/{contributed,scene_graph}.rs` by the `WATERS`
builder, after `EFFECT-GRAPH:M3` is Done, with the behaviour [WATERS](WATERS/DESIGN.md#the-surface-and-the-frame-contract)
defines. `EFFECT-GRAPH`'s items do not change and its tests keep passing: a scene with no effect that uses them
compiles to the same passes and storage as before. If M3 as built already allows either, nothing is added. A
third ruling covers the water example scene, the first game after the V1 game to run on the shell with a world
that ticks.

1. **An after-opaque pass may write scene depth** (`WATERS` M2). The stage let an effect read depth and write
   colour, motion and reactive coverage. Water that left depth alone would hand reconstruction the depth of
   whatever lies behind the water, and a cloud march drawn later would run through it. Stored depth still follows
   the union of its readers.
2. **An effect's own pass may read scene colour and scene depth as textures and write only that effect's
   targets** (`WATERS` M8). Water copies opaque colour and depth only inside the screen rectangles its readers
   need: the bodies a scene registers, water thin enough to see through, and the screen while water still enough
   to mirror is in view. On an open sea with no body in view that is nothing. It does not request the shared
   refraction copy. Depth cannot be sampled
   in the pass that writes it, so the gather is a pass of its own.

3. **The window, the page and the capture tap for a second ticking game** (`WATERS` M2 on the Mac, M3 in Chrome;
   `CREATOR`'s and `PLATFORM-RUNTIME-WEB`'s shell in `thrum-app`, `T2-CAPTURE`'s tap and the scene packaging
   scripts). `examples/water` is a game on the shell with its own ticking world and the engine's sky. Whatever the
   shell, the page, the packaging or the tap still assume of the V1 game, or of the sky kit's world that never
   ticks, is made a property of the game. The V1 game, the starter and the creator scenes build, run and draw as
   they did, no item of an owning module changes, and no engine crate gains a type or file of the water scene.
   `CREATOR` M2 and M3 edit the same crate; the `WATERS` builder integrates on top of what has landed.
   [`SWARM`'s ruling 2](#track-3-the-swarm-survival-game-after-v1) then covers only what a 2D game with no sky
   still needs.

4. **A mirror view is `SCENE-VIEWS`' to draw** (`WATERS` M14). Water mirrors scenery the camera cannot see by
   reading a second draw of the scene, from the camera's mirror image. That draw needs the scene's culling,
   lights, shadows and air for another view, which no effect may reach, so it is a module of its own with the
   [secondary-view interface](#interfaces) and not work the `WATERS` builder does in the renderer's files. Every
   other `WATERS` milestone is built without it.

Three more, for [light under water](WATERS/DESIGN.md#light-under-water-caustics-shafts-and-the-meter), are
built by the `WATERS` M16 builder with the behaviour that design defines. No item, test, bound or reference
of `ATMOSPHERE` or `APPLE-PRESENTATION` changes, and a scene with no water provider and a camera with no
medium factor composes the shader text and draws the bytes it does today.

5. **A water provider on the direct-Sun path** (`WATERS` M16; `synthetic/scene_source.rs`, the light group and
   `thrum-render`'s atmosphere files). The forward shader's direct Sun is `sun_irradiance · sun_factor ·
   cloud_factor`. A water provider may register a Sun-path factor with its textures, composed there as the
   cloud factor is, and, for a receiver the provider reports as under water, the point at which that
   receiver's Sun visibility is read. C08 says caustics are an illumination contribution with an explicit
   owner; this is that contribution, and V3 still applies each direct path once. `CLOUDS`' factor is
   unchanged and multiplies with it.
6. **A lent Sun visibility** (`WATERS` M16; the lent environment products). An effect may read the scene's
   Sun visibility at a world point, one tap of the scene's cascades and height map, for volume samples of
   its own. `ATMOSPHERE`'s view volume reads the scene's shadow itself and is not changed by this.
7. **A medium factor on the incident meter** (`WATERS` M16; `synthetic/view.rs` and `synthetic/exposure.{rs,wgsl}`,
   V4's). The camera request's incident meter takes three numbers that multiply the light it reads. C07's
   request stays a declared physical incident meter: the factor is the medium's transmission to the metering
   point, as [ruling 3 for `CLOUDS`](#seam-rulings-for-clouds) gives the meter the cloud factor there.

Once built, rulings 1 and 2 belong to `EFFECT-GRAPH`'s seam, ruling 3 to the shell's owners, rulings 5 and 6
to `ATMOSPHERE` and ruling 7 to `APPLE-PRESENTATION`: later work there
keeps their tests passing, and `CLOUDS` uses the same accesses and adds none.

### Seam rulings for RIVERS

Running water is charts in `WATERS`' map, wave layers in its field, births in its foam and records in its
interaction step, so most of what `RIVERS` builds at run time lies in `WATERS`' crates, and after `SEA`'s last
milestone in the two sea crates. Each ruling is built by the `RIVERS` builder there, with the behaviour
[RIVERS](RIVERS/DESIGN.md#mechanisms) defines. No item, test, bound or reference of `WATERS` or `SEA` changes,
and all keep passing. A scene with no river, no flow bands and no falls compiles to the same shader text,
passes and storage, and `thrum-water-model` gives it the same answers to the bit. `WATERS`' builder edits the
same crates; whichever lands second integrates on top. Two of these replace what
[WATERS' design](WATERS/DESIGN.md#what-the-later-modules-build-on) expected of this module: the flow's roughness
is no longer a gain on the wind's cascades (ruling 3), and a reach no longer keeps its prepared flow whatever
its source's stage (ruling 2). [RIVERS' design](RIVERS/DESIGN.md#judged-from-first-principles) gives the reasons.

1. **A chart's end may be a brink** (`RIVERS` M2; the map's preparation in `thrum-water-model`, and the map on
   the GPU in M6). Beside a source end and a mouth, a chart may declare a brink end: a stretch of its footprint's
   edge where the ground drops away and the water leaves as a fall. Preparation's rule for a footprint's edge
   does not refuse a cell on a declared brink, and the chart's row there is its own, with no mix. The ground
   below a brink is in no body. A chart whose source is a fall has no source body, as a spring has none.
2. **A chart may hold several flow samples** (`RIVERS` M3 in `thrum-water-model`, M4 in `thrum-water`). Each
   sample is a chart's grids with a gauge level. The chart's row at a point is the mix, linear in the gauge, of
   the two samples that bracket it: the source body's level where the chart has one, else the river's own
   stage. Every chart of one river takes the same gauge. Between two samples each cell also has a prepared
   **wetting gauge**, and holds no water, wave support or shore margin below it: a cell that only the upper
   sample wets, and that is reached through another such cell, stays dry until that cell is wet, so a hollow
   behind a ridge is never filled from the level beside it. The solve writes the gauges, spreading water from
   the lower sample's wet cells and from where each inflow enters, so a reach the lower sample leaves dry
   wets from its landing when the lip above it runs. From its wetting gauge a cell's level and current run
   linearly to the upper sample's, which for a bank cell is the level of the water beside it. A chart with one sample is
   unchanged, and its stage adds to its level as [the map](WATERS/DESIGN.md#the-water-map) says.
3. **Flow bands** (`RIVERS` M3 in `thrum-water-model`, M5 in `thrum-sea` and `thrum-water`). The world's wave
   field gains two layers the wind does not fill, of the cascades' kind, generated only where a scene's profile
   names them. A chart's row holds a gain for each, which may be any size the depth limit allows, and every
   other body's is zero. Every reader of the wave layers, the game's query among them, reads them through the
   one sum, with the rules a cascade has. Their cadence is a knob for the display alone: on a frame whose
   query dispatch holds a point over a chart with a flow gain they are evaluated at that frame's `t`, whatever
   the cadence and the camera, so no query answer changes with either. `WATERS:M16`'s caustic field reads them too; whichever of it and
   `RIVERS` M5 lands second makes
   that so.
4. **Births from a chart's loss and at a plunge** (`RIVERS` M5 and M6; the foam's births in `thrum-water`), by
   the bounded rule the foam's other births use.
5. **A current ladder in the interaction step** (`RIVERS` M6; the interaction step in `thrum-water`. An f64
   reference of the ladder in `thrum-water-model` is a diagnostic a builder may add, and is not required). A
   region whose wet cells run at different currents is stepped with a
   few carriage currents whose span, as vectors, holds them, and each cell takes the blend of the outputs
   carried at the currents that bracket its own, as the depth ladder blends by depth. So two sets of rings raised at once
   on a pool and on a riffle are each carried by their own water, whatever the other's strength. On still
   water and on a uniform current the ladder has one current and the step is what it was.
6. **A river selection in `examples/water`** (`RIVERS` M4). The game gains a selection that is off by default.
   With it off, the game's ground, bodies, tick page, compiled graph and frames are what they were, and the
   hand-made channel is `WATERS`' stand-in as built. With it on, the island's ground gains a valley, a spring
   and a cliff, and its rivers are solved. `RIVERS` adds its cases and scenes to `scripts/check_water_browser.mjs`
   and `scripts/water_cost.py` and changes none of theirs.
7. **The sheet, and the Sun's shadow on water** (`RIVERS` M6 and M7; the water's contribution in `thrum-water`).
   A fall's sheet is drawn by a pass of the water's contribution that runs before its gather, under
   [rulings 1 and 2 for `WATERS`](#seam-rulings-for-waters), and writes colour, depth, motion and reactive
   coverage. Once `WATERS:M16` has built the lent Sun visibility, the direct Sun on a sheet, on foam and in the
   surface's Sun lobe takes it at the surface point, so water in a hill's shadow is not lit by the Sun. If
   `WATERS` has built that by then, nothing is added.

Once built, the changes inside `WATERS`' own files belong to `WATERS`: later work there keeps their tests
passing. The files `RIVERS` adds stay its own.

### Seam rulings for PUDDLES

Rain's water is a body in `WATERS`' map, wave layers in its field, a level of its interaction waves, its
film and levels of its mirror, and wet ground is a read in the forward shader, so most of what `PUDDLES` builds at run time lies in
other modules' files. Each ruling is built by the `PUDDLES` builder there, with the behaviour
[PUDDLES](PUDDLES/DESIGN.md#mechanisms) defines. No item, test, bound or reference of `WATERS`, `SEA`,
`RIVERS`, `ATMOSPHERE`, `COOKED-DRAW`, `CONTENT-LOOP` or `SHOWCASE` changes, and all keep passing. A scene with
no hollows body, no rain capability and no wetness provider compiles to the same shader text, passes and
storage and draws the same bytes, and `thrum-water-model` gives it the same answers to the bit. `WATERS`' and
`RIVERS`' builders edit the same crates; whichever lands second integrates on top. Three of these replace what
[WATERS' design](WATERS/DESIGN.md#what-the-later-modules-build-on) and the interface table expected of this
module: a puddle is not a basin with a seed (ruling 1), a raindrop is not an impulse record (ruling 3), and
the film is drawn off the shore (ruling 4). [PUDDLES' design](PUDDLES/DESIGN.md#judged-from-first-principles)
gives the reasons.

1. **A hollows body in the map** (`PUDDLES` M1 in `thrum-water-model`, M3 in `thrum-water`). Beside the sea,
   the basin and the chart, a game may declare hollows: a rectangle with a bed of its own, a label for each
   cell and a level for each hollow, which the game sets with the tick as it sets a stage. A point's water is
   its cell's hollow's level where that stands above the bed. Inside the rectangle the map's bed is the
   body's own; its row has no current and no wind gain. Its identity and generation do not change as its
   puddles join and part, and a change of its levels relabels the interaction waves' regions and resets no
   state. Preparation refuses a hollows body with a cell in another body's water or shore margin anywhere in
   that body's stage range, so this settles how puddles merge without changing what preparation refuses of a
   basin: two basins that would merge are still refused.
2. **A fine interaction level** (`PUDDLES` M8, in `thrum-water-model` and `thrum-water`, after `WATERS:M6` has built the interaction step it extends). A profile may name
   a third level, of 2 cm cells with a depth ladder from 5 mm and surface tension in its frequency, which
   takes the waves under 1 m from the near level, so that no wave is on two levels, and steps one tick at a
   time. With none, the two levels hold and step what they did.
3. **Rain bands** (`PUDDLES` M2 in `thrum-water-model`, M4 in `thrum-sea` and `thrum-water`). The world's wave
   field gains two layers that are a function of time and of the rain's committed rate over the last second, and
   keep nothing of their own between frames, generated only where a scene's
   profile names them. Every body's row has a rain gain, exposure times the depth response at the point, and
   every reader of the wave layers reads them through the one sum, with the rules a cascade has. They are
   layers beside [`RIVERS`' flow bands](#seam-rulings-for-rivers) and share nothing with them.
4. **The film over standing rain water** (`PUDDLES` M5; the water's contribution in `thrum-water`). The film
   `WATERS` draws on sand the waves wash is also drawn over a hollows body's ground, wherever the wetness
   function gives surface water and no puddle stands, with the cover as its weight. It stays a declared
   residual: no depth, no query, no optical depth.
5. **A wetness provider on the forward surface** (`PUDDLES` M5; `synthetic/scene_source.rs` and
   `synthetic.wgsl` in `thrum-render`, and the cooked material in `thrum-asset-gpu`). A scene may register a
   provider of ground wetness with its textures, composed into the scene's shader as the cloud factor is and
   read by every contribution that compiles it. A surface's base colour and roughness take the wet look
   before it is lit, after any material hook. The light, the shadows, the air, the cloud factor and
   `WATERS`' Sun-path factor are applied as they were. `WATERS` M16 adds to the same shader; whichever lands
   second integrates.
6. **A rain selection in two games** (`PUDDLES` M3 in `WATERS`' `examples/water`, M6 in `SHOWCASE`'s
   `examples/v1-game`). Each game gains a selection that is off by default. With it off, the game's ground,
   bodies, world, tick page, compiled graph and frames are what they were. With it on, the game keeps a rain
   state in its world and steps it, registers the wetness provider, and adds ground with hollows: a yard on
   the island, and in the valley a patch of ground beside the terrain, with the water's contribution.
   `PUDDLES` does not edit the showcase's terrain, generator, routes or exit configuration, and adds its cases
   to `scripts/check_water_browser.mjs`, `scripts/check_v1_browser.mjs` and `scripts/water_cost.py` and changes
   none of theirs. Putting rain in the V1 exit scene by default would change the exit, which is the owner's.
7. **The Sun's shadow on the film** (`PUDDLES` M7). [`RIVERS`' ruling 7](#seam-rulings-for-rivers) gives the
   surface's Sun lobe the lent Sun visibility once `WATERS:M16` has built it; the film's Sun lobe takes it
   too. Whichever of `RIVERS` M7 and `PUDDLES` M7 lands first builds the surface's.
8. **Mirror levels of a hollows body** (`PUDDLES` M7; the mirror views' selection and lookup in
   `thrum-water`, which `WATERS:M14` builds). A hollows body has no one level. It gives the mirror each
   puddle's level and the ground's height where its film stands. Each frame those are grouped into bands no
   taller than the profile's tolerance for hollows, 5 cm in the first, and a band's plane is its lowest
   water, so a view's cut removes nothing above a water it serves. Bands rank for the profile's views with
   every other level, by the screen area of their water and with `WATERS`' hysteresis. The lookup starts
   from the surface point's own height and refuses a sample that lies below it. Rain water in no band
   with a view takes the screen's reflection, as `WATERS`' third level does. `WATERS`' rule for its own
   bodies, its items and its tests are unchanged.

Once built, rulings 1 to 4 and 8 belong to `WATERS`, ruling 5 to the owners of the files it is in (`ATMOSPHERE` for
the scene's composed shader, `COOKED-DRAW` for the cooked material) and ruling 6 to each game's owner: later
work there keeps their tests passing. The files `PUDDLES` adds stay its own.

### Seam rulings for SCENE-VIEWS

A secondary view runs the scene's own shader from another camera, so everything the shader reads that belongs
to a view has to exist for a second view, and each of those has an owner. Each ruling is built by the
`SCENE-VIEWS` builder in the owner's files, with the behaviour [SCENE-VIEWS](SCENE-VIEWS/DESIGN.md#mechanisms)
defines. No item, test, bound or reference of an owning module changes, and all keep passing. A scene that
declares no view compiles to the same shader text, passes, pipelines and storage and draws the same bytes as
before, but for where ruling 3 lays its height map out.

1. **Slots on the contribution seam** (`SCENE-VIEWS` M1; `contribution.rs` and
   `synthetic/{contributed,scene_graph}.rs`, `EFFECT-GRAPH`'s). A contribution may declare secondary-view
   slots, ask for them in its upload, opt in to drawing in views, and read its own slots' colour, depth and
   record in its own passes, as [ruling 2 for `WATERS`](#seam-rulings-for-waters) reads scene colour and depth.
2. **A camera-drawn compaction against another volume** (`SCENE-VIEWS` M1; `visibility.rs` and
   `synthetic/{camera_culling,mesh_route,buckets}.rs`, `CULLING`'s). One more compaction of the scene's
   instances for each asked view, against that view's frustum, clip plane and least size, into lists of its
   own, as `CULLING` already culls casters for each Sun pass. Occlusion is never used in a view, and the main
   view's lists, bits and byte-equality with Direct are untouched.
3. **Lighting products for a second view** (`SCENE-VIEWS` M1; `synthetic/{shadow,cluster}.rs` and
   `atmosphere/{graph.rs,aerial.wgsl}`). A view has Sun cascades fitted to its own frustum by the scene's
   fit, a cluster table built by the scene's pass, and an aerial volume from `ATMOSPHERE`'s aerial pass.
   The caster height map and the air's shadow heights stay one of each for the frame, laid out round the
   main camera's horizontal position by the scene's fit, and a view reads them unchanged. They are a view's
   own only on the main camera's vertical, so a request from a camera off it is refused when it is asked for.
   The fit lays that map out from the position a camera declares and no longer from one taken back out of
   its view matrix, in every scene, so that every camera on a vertical has the one map
   ([why](SCENE-VIEWS/DESIGN.md#where-the-height-map-is-laid-out)). It is the one change here that reaches a
   scene with no view: its bytes differ only where its camera's matrix had rounded its position across a
   texel's edge of the map. No registered exact baseline is expected to move, and rule 1 of
   [Exact baselines](#exact-baselines) applies if one does.
   For a view that asks for **air from the plane**, which must be level, the pair at a point is the
   transport from where the point's line of sight crosses the view's clip plane to the point, sampled at
   the true positions beyond the plane with the line's own direction and the scene's Sun visibility there,
   and nothing before the crossing. The view's volume is the pass's own for an observer at the plane's
   height, on every line whether or not it crosses, so the pass gains no second integrator; the decoder
   reads it by a fragment's distance beyond its own crossing. That product, drawn by the main view from the
   crossing, is what a view's air is held to. Air from a plane that is not level is refused. The reader owns
   the stretch from its own camera to the plane. With no such request the pass writes what it writes today, and the main view's
   shader text is unchanged. A view has clear air where a scene declares the view volume.
4. **A view that is not presented** (`SCENE-VIEWS` M1; `synthetic/view.rs`, `APPLE-PRESENTATION`'s view
   record). [C07](../design/CONTRACTS.md) now names a secondary view: a product of its presenting view that
   publishes no frame contract, has no history key and is never given to a reconstruction provider. It takes
   the frame's jitter phase at its own extent and the frame's exposure sample.
5. **Cooked placements in a view** (`SCENE-VIEWS` M2; `thrum-asset-gpu`, `COOKED-DRAW`'s paths and
   `CONTENT-LOOP`'s residency). The cooked contribution opts in: the direct path draws its placements in a
   view, and the indirect path culls meshlets for the view into arguments of its own. If the indirect path
   cannot hold a second set inside a device's limits, a view of cooked placements resolves to the direct
   path and reports it. Residency, publication and the main view's path selection are untouched.

Once built, each belongs to the owner of the files it is in: later work there keeps its tests passing.

### Seam rulings for CLOUDS

Clouds shade and light the world through `ATMOSPHERE`, and four things they need are not in its lent interface
as `ATMOSPHERE:M5` built it. Each is built in `thrum-render`'s atmosphere files by the `CLOUDS` builder, with the
behaviour [CLOUDS](CLOUDS/DESIGN.md#what-clouds-asks-of-other-modules) defines. `ATMOSPHERE`'s items, tests,
bounds and references do not change and keep passing. A scene with no cloud provider composes the same shader
text and draws the same bytes as today, which is that module's cue E3. `ATMOSPHERE` M6 edits the same files;
whichever builder lands second integrates on top. A fifth ruling covers the two games clouds are shown in.

1. **A provider may write the Sun-path optical depth on the GPU** (`CLOUDS` M2). M5's field is CPU bytes with a
   rigid drift, read along the true Sun, and a receiver below its lowest Sun elevation or outside its region gets
   exactly 1. A provider may instead register a 2D texture that its own `simulate` pass writes, with, each
   frame, its plane, region, displacement and validity, the **path direction** its columns follow, a border
   width and one `tau_far`. Such a field has no lowest elevation: the provider keeps its path direction a
   continuous function of the true Sun (`CLOUDS` holds its elevation at a floor), and a receiver is projected
   to the plane along it at every Sun elevation. Its factor is `mix(exp(-tau_far), exp(-tau), w)`, where `w` is
   1 inside the region, falls smoothly to 0 across the border inside the region's edge, and is 0 outside. So
   the factor is continuous in the Sun's elevation and across the edge, whatever the field holds there. The
   CPU-fed form states none of these and its receivers get exactly what they do now. A receiver at or above the
   plane, and an invalid frame, still get exactly 1.
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
   `ATMOSPHERE`'s design left to `CLOUDS` for in-scatter under cloud: no shafts and no colour shift. A scene
   that declares the view volume does not take it, because there the air is lit sample by sample through the
   cloud factor; whichever of `CLOUDS` M8 and `ATMOSPHERE` M7 lands second makes that so.
5. **A cloud selection in two games** (`CLOUDS` M2 in `SHOWCASE`'s `examples/v1-game`, M11 in `WATERS`'
   `examples/water`). Each game gains a selection that is off by default. With it off, the game's world, tick page,
   compiled graph and frames are what they were, and no item of the owning module changes. With it on, the game
   keeps a weather state in its world, adds the cloud contribution after any sea, and the labelled stand-in
   shade provider is not used. `CLOUDS` does not edit the showcase's generator, routes or exit configuration.
   Putting clouds in the V1 exit scene by default would change the exit, which is the owner's. With the
   Ground capability the selection also gives the weather the game's surface field, made in the game from its
   own terrain heights (`CLOUDS` M14) or its own bed and water levels (`CLOUDS` M11), and declares the view
   volume.

Rulings 6 to 8 are `ATMOSPHERE`'s own milestones and not the `CLOUDS` builder's, because the view volume is
large, has its own inspections and must leave the V1 scene untouched:

6. **The view volume takes a provider's medium coefficients** (`ATMOSPHERE` M8), as
   [the interface](#interfaces) states. `ATMOSPHERE` never evaluates a density, a phase or a medium's own
   shadow, and a provider never integrates the air.
7. **A ground band of the Sun-path optical depth** (`ATMOSPHERE` M8). A provider may register optical depth
   toward the Sun at heights above the ground, with its mapping and a border width, and with where the land
   blocks the Sun beyond the shadow cascades kept apart from it. The band holds the whole path: the
   provider's march from each node to where the ray leaves the band, plus the 2D field's value there. The
   one cloud-factor function makes one read: the band at a receiver's own position for a receiver inside
   it, and the 2D field at or above the band's top and outside its region. It never adds the two, so an
   upper cloud dims a receiver in the band once. The factor is continuous at the band's top, where the
   band's value is the 2D field's, and mixes to the 2D field's across the band's border. Only the view
   volume's cells read the land's blocking; no surface does.
8. **The air's haze is an input** (`ATMOSPHERE` M7) of two numbers: its amount, as the clear air's visual
   range, and its kind, as an Ångström exponent from 0 to 2. The defaults are today's constant and an exponent of
   0, today's grey aerosol, so a scene that sets neither draws as before, and neither number moves the epoch when
   it moves. The weather state holds both (`CLOUDS` M13 the range, M16 the exponent); `ATMOSPHERE` turns them into
   the aerosol in each band ([its design](ATMOSPHERE/DESIGN.md#the-view-volume-haze-fog-and-light-near-the-camera)).

Once built, rulings 1 to 4 belong to `ATMOSPHERE`: later work there keeps their tests passing. Rulings 6 to 8
are its own from the start. `CONTENT-LOOP`'s
anticipated cook-time atlas is not asked for: a game stacks its shapes into one `TVOL` source of power-of-two
slabs, cooked as any volume is. Its mips stay inside a slab only down to the level with one texel a slab, so
`CLOUDS` reads no coarser level and holds its coordinate inside the slab
([CLOUDS](CLOUDS/DESIGN.md#placed-volumes)); nothing is asked of the cooker.

Track 3's interfaces are in [their own table](#interfaces-track-3).

## Exact baselines

An **exact baseline** is a set of retained bytes that a proof compares rendered output with, byte for byte. Each
has one owner. On 11 November `APPLE-PRESENTATION` M1 applied jitter to every game view, which moved every
rasterised pixel of `APPLE-TILE`'s M0 baseline. The builder changed two exact tests in `./dev check` to absorb it,
and the change merged. `APPLE-TILE` M2 was then claimed on evidence from a binary built before its rebase
([APPLE-TILE M2 review](../../.orchestrate/evidence/APPLE-TILE/M2-review.md), MF1;
[APPLE-PRESENTATION M1 review](../../.orchestrate/evidence/APPLE-PRESENTATION/M1-review-1.md), MF1). Three rules close those holes.

<a id="proofs-run-once-on-main"></a>

**Ruling: proofs run once, on main (15 November 2026).** Rule 2 first asked every builder for a proof of the
exact content it pushed. Main changed the covered paths about every 20 minutes and a proof took about 16, so
with more than three covered merges an hour a proof was usually void before it could be used. In one day four
milestones reached Ready for Review against 419 commits, 163 of 288 background jobs failed, and the one GPU lock
was busy 92% to 94% of the day, mostly in rebase, re-prove and merge attempts that failed. Of 79 verifier runs,
one failure was a real pixel change and 21 were flakes or infrastructure
([the report](../../.orchestrate/evidence/BASELINE-GUARD/M2-replan/proof-regime.md)). The owner approved its regime: "Ok, let's
get the script and skill changes done then once we've got orchestrate to where it needs to be, let's get the
lead on Thrum working." and "We want all these fixes in place before I head to bed tonight." In the lead's
session that day the owner said "Go ahead" to a gate that only warns, an hourly verifier on main, `gpu: 2,
chrome: 1`, `BASELINE-GUARD` re-planned, the rule and skill text updated, and an unintended image change
sitting on main for about two hours.

- **What changed.** Rules 2 and 3 are written again below. Rule 1 stands. A proof before the merge is no longer
  asked of anyone, and `./dev check` no longer refuses for the lack of one.
- **Earlier text.** Where a module's design, plan note or log says to run the verifier before `land`, to hold
  the `gpu` lock for it, or to run it again after a rebase, this ruling replaces that. Each module's text is
  corrected at its next re-plan. An item whose check is the verifier keeps its text and loses its `lock`.
- **The rulings on [a frame that varies under load](#a-frame-that-varies-under-load) and
  [a fixed capture that stalls](#a-fixed-capture-that-stalls).** On main, the script's second attempt stands in
  for the builder's second run: a run that passes on its retry is recorded by the script and needs no record
  from a module. Their conditions and records now apply to a `verify --full` run made for a planned refresh, and
  to a module that disputes a failure which held on main.
- **Not done here, recorded as later work.** A `./dev check` that tests only the crates a merge batch affects,
  with a full run on main every few hours; and a two-minute tier of the native matrix for routine runs on main,
  which needs a ruling with `APPLE-TILE`, whose sampling it is.

**1. An exact comparison is its owner's interface.** A builder never changes another module's retained
references, its comparison, its tolerance or its sampling (accepting the closest of several frames, for example),
nor an exact test of output another module owns, to make its own change pass. The registry, the verifier and the
reference guard of rule 2 are `BASELINE-GUARD`'s in the same way. When its change fails one, it stops, writes the proposal in
its log and ends `BLOCKED`; a planner settles it here with the owner, as for any interface. A change that needs
different output is built behind a selection whose default leaves every registered image exact, which is how MF1
was fixed: jitter is gated on explicit temporal sampling. When a milestone is meant to change the default output,
its planner names here, before it is built, which baselines move and which milestone of the owner refreshes them.
A refresh records the cause and the old and new hashes in both modules' logs. Nobody refreshes a baseline
silently. A loop builder pushes a covered-path change only through `land` with its check on, never
`land --no-check`. Any other thread that pushes one runs `./dev check` on the exact commit it pushes.

**2. After the merge: one run on main proves every landing.** The main-branch check `exact-images`, declared in
`ORCHESTRATE.md`, runs the verifier that [BASELINE-GUARD](BASELINE-GUARD/DESIGN.md) M1 built on main's tip:
`python3 scripts/exact_baselines.py verify --full`, in the script's own clean checkout, holding the `gpu` and
`chrome` locks. It draws fresh output on the Mac and in Chrome and compares it with every registered baseline,
byte for byte. One run covers every commit merged since the last one, whoever merged them.

- **When it runs.** When a milestone's claim asks for it (rule 3), and when the lead's hourly
  `main-check --due` finds that main has moved. Two runs start at least 30 minutes apart.
- **When it fails.** The script runs it again once. A failure that holds is traced over main's commits since
  the last pass to the first that fails, and the result names that commit and its module. That module fixes it
  as its next task. If no fix has merged an hour after the commit is named, the lead reverts the commit. Every
  other module keeps merging meanwhile.
- **What is accepted.** Main can hold an unintended image change for about two hours. Nothing ships from main,
  and no other builder is stopped by it.
- **Before the merge.** No image run is needed. `./dev check` runs the CPU-only reference guard. Each registry
  entry pins its definition (all fields except `pin` and `refresh`) and the paths and Git object IDs of its
  retained files and `comparison_files`. A changed pin requires a changed refresh naming an existing anchor
  here, a module and a milestone. The guard prints pins computed from HEAD and rejects uncommitted protected
  changes. Commit protected changes, copy the computed pins into the registry, record the refresh and commit
  before checking. An owner editing a comparison file follows the same procedure, naming its ruling or the
  ownership rule. An added or redefined baseline needs a pin and refresh; a planner retiring one moves its
  identity and owner to `retired` with a refresh naming the ruling. The registered set is checked against
  the registry at the merge base with main, so deleting an entry alone fails.
- **What does not change.** Bytes are compared exactly. The verifier does not retry inside a run, accept a
  nearer frame, skip a record or compare within a tolerance.
- **A planned refresh** is the one run made before a merge. A milestone that a ruling here allows to move a
  baseline regenerates the references on its candidate, runs `verify --full` there under the `gpu` lock, and
  updates and commits the pin and refresh, and merges the change, references and registry in one landing,
  so the next run on main passes with them.
- **What a planned refresh changes in the registry** (16 November 2026) is two fields of each baseline whose
  reference moved, and nothing else: the `pin`, to the value the guard prints for the committed references,
  and the `refresh`, to the anchor of the ruling that allows it with the refreshing module and milestone. A
  baseline whose reference did not move keeps both. Every definition, `covered_paths` and the retired list
  stay as they are, and so does every comparison file. Where a ruling or an item written before the guard
  says the registry stays as it is, it means this. Only the guard reads the two fields: the verifier, the
  comparisons and the game do not. So a commit whose whole diff is those fields changes nothing that was
  drawn or compared, and no image proof made before it is made again for it. A rebase that changes a pin,
  because main changed a comparison file, is settled the same way: the builder copies the pins the guard
  prints and keeps its own refresh.
- **A relocation is separate.** Moving the retained files to another folder changes paths, which a refresh does
  not. [The ruling on where retained references are kept](#ruling-on-relocated-references) says which paths
  follow the files and what still may not change.

<a id="the-covered-paths-are-derived"></a>

**Unlinked crates leave the covered paths: withdrawn (15 November 2026).** `BASELINE-GUARD` M2 was to take the
files of crates the game never links out of the proof's key, so that fewer rebases lapsed a proof. No proof is
asked before a merge now, so nothing lapses and the narrower key has no use. The work is set aside unmerged, and
[the design](BASELINE-GUARD/DESIGN.md#later-work) records it. The covered paths are the listed paths in the
table below, whole, and they key the verifier's record.

**3. At the claim: the shared run, as a planner-written check.** A builder cannot change an item's `check`, and
the loop reruns every `check` on a clean copy of main before it records a milestone. So each milestone that
edits covered paths carries an item whose `check` is `python3 scripts/exact_baselines.py verify`, with no
`lock`. Without `--full` the command draws nothing. It asks the main-branch check whether a passing run
contains the commit, waits for the run under way or starts one, and passes once one does. It fails when main
fails at or before the commit, naming the first failing commit, and on a branch it says the commit is not on
main yet. So a builder does not run it before merging: it sets the item `pass` with the evidence `main-branch
check exact-images`, and the loop's run on the merged tip is the proof. A module that draws nothing drops the
item at its next re-plan. Every exact or visual proof records the revision, clean-diff hash and binary hash it
ran on, and that revision is the commit claimed. A reviewer who finds a baseline, the registry, the verifier or
an exact test changed by a module that does not own it, with no ruling here, records a Must Fix.

| Baseline | Owner | What it pins | Compared by | Covered paths |
|---|---|---|---|---|
| `examples/v1-game/tests/fixtures/exact-images/native/` | `APPLE-TILE` | On the Mac, for both recipes: submitted SDR and scene-linear bytes at the moving route's checkpoints, Sun and night stills at 1080p, and letterbox bytes in two surface formats | `scripts/apple_tile_m1.py` and `scripts/apple_tile_m2.py`, on a release build of the commit and a fresh cook of the showcase valley | Listed: `crates/`, `examples/v1-game/`, `scripts/`, `tools/`, `.cargo/`, `.config/exact-baselines.json`, the workspace manifest, lockfile and toolchain file, and all registered retained files. |
| Four Chrome manifests: `examples/v1-game/tests/fixtures/exact-images/chrome/prechange-chrome-fixed-manifest.json.gz` (day, submitted SDR), and under `examples/v1-game/tests/fixtures/exact-images/chrome/` the `manifest.json.gz` of `chrome-prechange-linear-3` (day, scene-linear), `chrome-prechange-night-sdr` and `chrome-prechange-night-linear` | `APPLE-TILE` | Chrome's portable output on the moving route: for each of 481 records, the identity fields and the SHA-256 of the frame's decoded bytes | A fresh Wasm build and `fixed` capture of the commit, compared record by record with the retained digests by `scripts/apple_tile_m1_compare_browser.py` | The same |
| `examples/v1-game/tests/fixtures/showcase/valley-baseline.json` | `SHOWCASE` | The six route stops on both targets, within thresholds, not exactly. Not in the verifier | `python3 scripts/run_v1_showcase.py check` | The same |
| `examples/v1-game/tests/fixtures/showcase/exit-baseline.json`, created by `SHOWCASE` M5, with one baseline per row planned for M7 | `SHOWCASE` | The six stops of the exit scene, within thresholds, not exactly. Not in the verifier | `python3 scripts/run_v1_showcase.py check --exit` | The same |

The verifier, `python3 scripts/exact_baselines.py verify --full`, runs the first two rows.

**Ruling on the Chrome reference (with `APPLE-TILE`).** The complete reference captures, 481 frames each, were
kept only in the scratch of the builder that made them, so a clean checkout could not run the byte comparison.
The retained manifests are the reference: each holds every record's identity fields and the SHA-256 of its decoded
bytes. A fresh frame whose decoded bytes hash to the retained digest is taken as byte-equal. `BASELINE-GUARD` M1
adds this reference-by-digest mode to `scripts/apple_tile_m1_compare_browser.py`; the script's byte mode, the
retained files and `APPLE-TILE`'s items do not change. If a set does not match on untouched main, the builder
ends `BLOCKED` and `APPLE-TILE` refreshes it under rule 1.

<a id="ruling-on-native-mip-bias"></a>

**Ruling on the material mip bias (with `APPLE-TILE`).** `APPLE-PRESENTATION` M5 deliberately restored
the [designed mip bias](APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence) of −1 at native width in
`32dfd3c14`, including with temporal reconstruction off. That changes `apple-tile-native` starting at
`portable-sdr-approach`. The formula applies on every target. The lead extended the ruling after a matched
Chrome WebGPU control recovered all 1,924 retained frames by restoring zero bias in the spatial branch.
`APPLE-PRESENTATION` M14 regenerates the affected native references and all four registered Chrome manifests
with `APPLE-TILE`'s capture scripts. Both modules' logs record the cause and each replaced file's old and new
SHA-256. The refresh follows rule 2, including `verify --full` on the candidate before landing.
The registry changes the five refreshed baselines' pins and refresh records. Each refresh names
`ruling-on-native-mip-bias`, `APPLE-PRESENTATION` and `M14`. The comparison files and their sampling stay as
they are. Report any other baseline mismatch to the lead with its capture record.


<a id="ruling-on-atmosphere-m9"></a>

**Ruling on `ATMOSPHERE` M9 (with `APPLE-TILE` and `SHOWCASE`), 14 November 2026.** M9 is meant to change
the default output of every scene: the aerial pass no longer stops a line of sight at the model's ground
([its design](ATMOSPHERE/DESIGN.md#the-air-at-the-ground)). It is not built behind a selection, because the
stop is a defect of the one aerial product and a selection would keep it in every scene that did not ask.

- **Which baselines move.** All five registered sets are expected to: `apple-tile-native` and the four Chrome
  manifests. Each holds terrain that reads a line which met the ground: low ground 3 to 12 km from the
  valley's cameras, up to 0.4% of the covered pixels at the route's checkpoints, 4% to 5% in the sunset still
  and 0.1% to 0.2% in the night still
  ([the planner's reading of the mesh](../../.orchestrate/evidence/ATMOSPHERE/design-proof/README.md), a prediction and not a
  GPU run). Those pixels gain haze. The sky, the stars, the near ground, the props and the letterbox bars are
  predicted not to move. M9's builder records which sets did move.
- **Who refreshes them.** The change and the references land together, as rule 2's planned refresh, so no
  milestone of `APPLE-TILE`'s could hold them. By this ruling M9's builder regenerates each reference that moved, from the changed commit, with `APPLE-TILE`'s own
  scripts, in the landing that makes the change. In the registry it changes the `pin` and the `refresh` of
  each baseline whose reference moved, as rule 2's planned refresh says, and the refresh names
  `ruling-on-atmosphere-m9`, `ATMOSPHERE` and `M9`. Every other field of the registry, the comparisons, their
  sampling and every script stay as they are. The refresh is `APPLE-TILE`'s in every other respect: `ATMOSPHERE` M9.5 holds
  the verifier at the starting commit and at the claimed one and the old and new hashes in both modules'
  logs, M9.6 has a reviewer see that only distant low terrain moved, and `APPLE-TILE` M12 reruns its exact
  matrices on the refreshed references with its items unchanged.
- **Other baselines.** `SHOWCASE`'s six route stops are compared within thresholds; M9.7 runs that comparison
  with its baseline and thresholds as they are. If it fails, the builder ends `BLOCKED` and `SHOWCASE`
  settles it. Any other module's exact test that M9 moves is that module's under rule 1, and the builder
  ends `BLOCKED` with it.
- **Everyone else.** Nothing is owed: the run on main after M9's landing compares with the new references,
  and a module whose milestone carries the verifier as an item needs no change.

<a id="ruling-on-culling-m3"></a>

**Ruling on `CULLING` M3 (with `APPLE-TILE` and `BASELINE-GUARD`), 14 November 2026.** M3's builder stopped under rule 1: on Metal
the valley drawn Direct is not one image under GPU load, the correction moves `APPLE-TILE`'s references, and no
plan said who owned either ([its design](CULLING/DESIGN.md#m3-one-image-under-load) has the defect, the
mechanism and the evidence). The correction is an all-ones coverage output on the forward shader's opaque
fragment entries, which stops the driver's triangle merging. It is not built behind a selection, because the
variation is a defect of the one forward pass, and a selection would leave it in every scene that did not ask
and in every exact test of the default.

- **Who builds the correction.** `CULLING` M3's builder, in `thrum-render`'s forward shader and wherever else
  an opaque forward fragment entry takes an implicit derivative. No milestone is added to `APPLE-TILE`, which
  has a builder on M10. Once built, the output belongs to the forward pass: later work there keeps it, and an
  opaque fragment entry added by any module writes the same output.
- **Which baselines move.** `apple-tile-native` is known to: 8,303 pixels of `portable-sdr-approach` on the
  candidate, and the verifier stopped there, so the other native references and the four Chrome manifests are
  unmeasured. Pixels on shared triangle edges of textured or shadowed opaque geometry are expected to move in
  every set that holds such geometry; the sky, the stars and the letterbox bars are not. M3's builder records
  which sets moved.
- **Who refreshes them.** As for M9, the change and the references land together, as rule 2's planned
  refresh, so no milestone of `APPLE-TILE`'s could hold them. By
  this ruling M3's builder regenerates each reference that moved, from the changed commit, with `APPLE-TILE`'s
  own scripts, in the one landing that adds the output. In the registry it changes the `pin` and the `refresh`
  of each baseline whose reference moved, as rule 2's planned refresh says, and the refresh names
  `ruling-on-culling-m3`, `CULLING` and `M3`. Every other field of the registry, the comparisons, their
  sampling and every script stay as they are. The references stay `APPLE-TILE`'s in every other respect: `CULLING` M3.10 holds
  the verifier at the starting commit and at the claimed one and the old and new hashes in both modules' logs,
  M3.11 holds the proof that the corrected image no longer varies, M3.12 has a reviewer see that only shared
  triangle edges moved, and `APPLE-TILE` M12 reruns its exact matrices on the refreshed references with its
  items unchanged. No other change of M3 may move a reference.
- **Proved once, then merged.** (16 November 2026; replaces "Qualified before the push" of 15 November.) The
  earlier text had M3.11's campaign, M3.12's difference images and `verify --full` pass on a local candidate
  before the push, and had the campaign stand, path by path, for the commit pushed and the commit claimed.
  The builder enforced that order with a chain of jobs, each asserting the one before, which rebuilt the proof
  before the merge that [proofs run once, on main](#proofs-run-once-on-main) removed: a job audit of 16
  November traced about 30% of all failed background jobs and five failed GPU holds to it. M3 now lands as
  rule 2's planned refresh, like any other. The branch regenerates the references, runs `verify --full` once
  and commits the pins, and `merge` lands the output, the references and the pins together. No proof is made
  again for a rebase unless the rebase changes a registered reference. If the refresh proves wrong after it
  lands, rule 2's course applies: the module fixes it or the lead reverts the merge.
  [`CULLING`'s design](CULLING/DESIGN.md#how-it-lands) has the order, and forbids the builder a landing
  pipeline of its own.
- **The loaded proof is made on main.** (16 November 2026, later that day; replaces "tied to main through the
  references".) The registered references are drawn Direct. A build that lost the output's effect on Direct's
  pipelines draws the old bytes and fails the main-branch check on every run, but Frustum, Occlusion and the
  bucket route prepare pipelines of their own, which no reference draws. A campaign made on the branch
  therefore says nothing sure about a culled or bucket pipeline that the rebase changed. So M3.11's campaign,
  the build with the output taking turns with a control without it under counted load, is taken once after
  the merge, on a build of a commit of main that holds the landing, over the cube route and the bucket route.
  It is taken again only if M3 itself merges another change to what the game is built from, never because
  main moved. It is one job of the builder's, with nothing chained to it. If the build with the output
  differs there, the builder ends `BLOCKED` and a planner settles here what main keeps. After M3 no standing
  loaded proof guards those pipelines: the ruling on
  [a frame that varies under load](#a-frame-that-varies-under-load) stops any module whose exact comparison
  varies on content that holds the output. [The design](CULLING/DESIGN.md#what-a-campaign-stands-for) says
  what the campaign stands for.
- **Order with `ATMOSPHERE` M9.** Both landings regenerate references. Whichever lands second rebases, holds a
  verifier pass at its new starting commit (the main-branch check's, or one `verify --full` of its own) and
  regenerates from there. Each records its own old and new hashes, so the two causes stay apart.
- **Other baselines.** `SHOWCASE`'s six route stops are compared within thresholds, which stay as they are.
  Any other module's exact test that the output moves is that module's under rule 1, and the builder ends
  `BLOCKED` with it.
- **Everyone else.** Nothing is owed: the run on main after the landing compares with the new references. Exact
  tests that compare a run with itself need no change, and stop
  varying under load once they are built on it.

<a id="ruling-on-the-valley-fix"></a>

**Ruling on the valley's lookout rim and sunrise stop (with `SHOWCASE`, `APPLE-TILE`, `CULLING`, `EFFECT-GRAPH`
and `ATMOSPHERE`), 19 November 2026.** `ATMOSPHERE` M6's builder stopped under rule 1. At the held sunset stop a
bright band lies along the rim of the lookout, on the Mac and in Chrome, and `APPLE-TILE`'s sunset references
hold the same band. At the sunrise stop the sphere and the rock cast no shadow. Both causes are in the authored
valley and neither is in the sky or the Sun: the terrain mesh is too coarse at the rim for the normals it
stores, and at 6.1 h the lookout is still in the east ridge's shade
([SHOWCASE's design](SHOWCASE/DESIGN.md#the-lookouts-rim-and-the-sunrise-stop) has the measurements). The fix
changes the valley's terrain, so it moves every registered set, and no plan said who builds it or who refreshes.

- **Who owns the valley.** `SHOWCASE`. One generator authors the terrain and the exit scene's layout, the six
  stop presets came from its route slice, and its M8 judges the picture. The valley's source folder and the
  presets join what that module owns.
- **Who builds the fix.** `SHOWCASE` M9, a new milestone after its M6. `ATMOSPHERE` M6 waits for it and then
  judges the whole route on the changed valley. No milestone is added to `APPLE-TILE`, whose milestones are
  all Done.
- **What may change.** The terrain's mesh and normals, the layout regenerated on it, and the hour of the sunrise
  preset. The valley's cooked root and the exit store's root change with them. The renderer, the other five
  presets, every route's checkpoints and every comparison stay as they are.
- **Root pins.** A route file names the cooked root it was made for. M9's builder writes the new root, and the
  generation if it moved, into each file that names an old one: `moving-v1.json` (`APPLE-TILE`'s),
  `culling-fast-yaw.json` (`CULLING`'s), `effect-state.json` and `effect-after-opaque.json` (`EFFECT-GRAPH`'s),
  its own exit routes and launcher, and any other pin of those roots that `./dev check` reads. In a file that
  holds such a pin it changes nothing else (`SHOWCASE` M9.7). The refresh below is separate and changes
  `APPLE-TILE`'s references, registry fields and log. Those modules' items do not change, and their tests keep
  passing.
- **Which baselines move.** All five registered sets are expected to, since each draws the valley's
  terrain. M9's builder records which did. `SHOWCASE`'s own two thresholded baselines are calibrated again for
  the new roots, by the route decision's fixed rule.
- **Who refreshes them.** As for [`ATMOSPHERE` M9](#ruling-on-atmosphere-m9), the change and the references land
  together as rule 2's planned refresh. M9's builder regenerates each reference that moved, from the changed
  commit, with `APPLE-TILE`'s own scripts. In the registry it changes the `pin` and the `refresh` of each
  baseline whose reference moved, and the refresh names `ruling-on-the-valley-fix`, `SHOWCASE` and `M9`. Every
  other field of the registry, the comparisons, their sampling and every script stay as they are, apart from
  the paths that [the relocation in the same landing](#ruling-on-relocated-references) moves. Both
  modules' logs record the cause and each moved reference's old and new SHA-256. `SHOWCASE` M9.6 holds this.
- **A change to the look in `ATMOSPHERE` M6.** After the valley is fixed, M6's list may still need a change to
  the sky or the sunlight, which [its design](ATMOSPHERE/DESIGN.md#closing-g3s) allows. Such a change also
  moves these sets. M6's builder then refreshes each set that moved in the same way, in the landing that makes
  the change, and the refresh names `ruling-on-the-valley-fix`, `ATMOSPHERE` and `M6`. `ATMOSPHERE` M6.9 holds
  it. M6 does not change the valley's source; a defect it traces to the terrain goes to `SHOWCASE` as a
  proposal in its log.
- **Records on the old valley.** A record another module took on the old root stands as history for what it
  measured, and none is taken again for this. A milestone not yet claimed when M9 lands integrates on top, as
  after any merge.
- **Everyone else.** Nothing is owed: the run on main after M9's landing compares with the new references.

<a id="ruling-on-relocated-references"></a>

**Ruling on where retained references are kept (with `SHOWCASE`, `APPLE-TILE` and `BASELINE-GUARD`), 20 November
2026.** Evidence left Git for the working folder on 19 November, and the loop's merge guard now refuses a commit
that changes a file under `docs/evidence/`. The retained references were kept there, so `SHOWCASE` M9 could not
land its refresh. The lead ruled that a file a check reads is test data and not evidence, and the references
moved to test fixtures in M9's landing. The 16 November text on a planned refresh and the valley ruling were
written before that. Both say the registry's other fields and every comparison file stay as they are, and
[M9's review](../../.orchestrate/evidence/SHOWCASE/M9-review.md) failed M9.6 on that wording. This ruling says
what a relocation may change, so that the freeze keeps protecting what it was written for: what each comparison
compares and how it decides.

- **Where they are kept.** `APPLE-TILE`'s native references are in
  `examples/v1-game/tests/fixtures/exact-images/native/` and its four Chrome manifests under
  `examples/v1-game/tests/fixtures/exact-images/chrome/`. `SHOWCASE`'s two thresholded baselines are in
  `examples/v1-game/tests/fixtures/showcase/`. The table above names each.
- **What a relocation is.** Every retained file keeps its name under the new folder, and its bytes. A file the
  same landing does not regenerate has the same Git blob at its new path as it had at its old one.
- **What follows the files.** Only a path that says where the references are: the registry's `reference`, the
  matching entries of `covered_paths`, and the line of a comparison file that names the folder it reads them
  from, which is `BASELINE` in `scripts/apple_tile_m1.py`. A pin covers its baseline's definition and its
  files' paths, so each relocated baseline's `pin` changes, and its `refresh` with it. The refresh names the
  ruling that the landing's planned refresh names, or `ruling-on-relocated-references` when a relocation
  lands alone.
- **What does not change.** The registry's other fields: each baseline's identity, owner, scene, source,
  candidate, comparison, record count and lists of comparison files. The sampling and the exactness. Every
  other line of every comparison file. A relocation that needs more than a path is a change to the owner's
  interface under rule 1.
- **Proof.** A relocation draws nothing. The reference guard pins the blobs at their new paths, and the next
  run on main compares fresh output with them there. A reviewer compares the retained files before and after,
  by name and Git blob.
- **Earlier text.** Where a ruling or an item written before this says a refresh changes only the `pin` and
  the `refresh`, or that the comparison files are as they were, it means apart from these paths. `SHOWCASE`
  M9.6 is written again to say so. `ATMOSPHERE` M6.9 judges that milestone's own commits, which hold no
  relocation, and stands.
- **Another relocation** is named here first, with its old and new paths, as any change to an owner's
  interface is.

<a id="a-frame-that-varies-under-load"></a>

**Ruling on a frame that varies under load (with `APPLE-TILE`, `BASELINE-GUARD`, `CULLING` and `EFFECT-GRAPH`),
15 November 2026.** `EFFECT-GRAPH` M3's builder stopped under rule 1. At `324586ee` the verifier passed both
native matrices and three Chrome sets, and failed Chrome night scene-linear at one record of 481, with every
identity field equal. The builder asked for `APPLE-TILE` to resolve it; no plan said whose it was.

- **What it is.** In the reference, records 0 and 1 of that set are one tick submitted twice, with one digest.
  In the failed capture record 0 hashes to it and record 1 does not. The two differ at 12,637 pixels, by a median
  of 2 half-float steps, and those pixels are thin lines along the triangle edges of the far hills
  ([the planner's reading](../../.orchestrate/evidence/EFFECT-GRAPH/M3-replan/README.md)). `APPLE-TILE` M10 met the same a day
  earlier on a build without `EFFECT-GRAPH` M3: Chrome day SDR, record 227, 299 pixels, at most two byte values,
  and a retry that matched all 481 ([its record](../../.orchestrate/evidence/APPLE-TILE/M10/chrome-frame-227/chrome-diagnostic.json)).
  This is the signature of the defect in [`CULLING`'s design](CULLING/DESIGN.md#m3-one-image-under-load), where
  derivatives at shared triangle edges change with GPU load, now seen in Chrome, which that design lists as not
  measured. It is an inference from where the pixels lie, not a reading of Chrome's derivatives.
- **Whose it is.** Nothing is owed by `APPLE-TILE` or `BASELINE-GUARD`: the references, the comparisons, the
  registry and the verifier are right and stay as they are, and neither module gains a milestone. The correction
  is `CULLING` M3's, by the ruling above. `EFFECT-GRAPH` M3 does not wait for it. Every landing on a covered path
  meets the same variation, so a hold on one milestone would settle nothing, and a hold on all of them would stop
  the build until a milestone that must itself pass the verifier.
- **The rule, for content without `CULLING` M3's coverage output.** A failed verifier run stays failed: it
  writes no proof, and nothing from it counts towards one. The builder may set it aside as **load variation**
  and run the verifier again, in place of stopping under rule 1, only when all four of these hold:
  1. *Identities.* Every identity field of every record matches. Only image bytes, or their digests, differ.
  2. *Matching bytes.* For each failed image the builder holds bytes that match its reference: the retained
     reference on the Mac; in Chrome a frame whose decoded bytes hash to the retained digest, from another
     record of that digest in the same capture or from a fresh capture of the same content.
  3. *Where.* Between the failed image and the matching bytes, every differing pixel lies along an edge between
     triangles of opaque geometry: thin lines, with no patch filled, and none in the sky, among the stars or in
     the letterbox bars.
  4. *The same content passes.* The whole verifier, unchanged, then passes on the same covered content, by its
     content key. That run is the proof, and it is complete: every record of every set, in one run. If it
     fails, it is a failed run of its own and is judged by these same tests, and a record that fails in both
     runs is not variation.

  A failure that misses any of the four is rule 1's: the builder stops, writes the proposal in its log and ends
  `BLOCKED` with every run. An identity field that differs while the images are equal is judged by
  [the next ruling](#a-fixed-capture-that-stalls).
- **What does not change.** The verifier does not retry inside a run, accept a frame from a failed run, search
  for a nearer frame, skip a record or compare within a tolerance, and no reference is refreshed. The gate still
  passes covered content only on a complete pass.
- **The record.** In the module's evidence folder, for each run set aside: the failed run's record with its
  content key, revision and binary or Wasm hashes; each failed image and its matching bytes, lossless; an image
  of where they differ, with the count of differing pixels, their bounds and the largest difference; and the
  passing run's record on the same content key. The module's log gains one line that links it. At the review of
  a milestone that carries the verifier (rule 3), the reviewer reads each record. A run set aside without one,
  or a difference image that shows anything but those lines, is a Must Fix.
- **When it ends.** With the landing that puts `CULLING` M3's coverage output on main. On content that holds
  the output, a failure of this kind is evidence that the correction does not hold on that target: the builder
  ends `BLOCKED` with the record, and a planner settles it here. The same goes for a module's own exact
  comparison of two draws of the same content, when it differs only along such edges (16 November 2026): the
  verifier draws Direct, so a loss on a culled or bucket pipeline can show nowhere else.

<a id="a-fixed-capture-that-stalls"></a>

**Ruling on a Chrome record that holds the tick before (with `APPLE-TILE`, `BASELINE-GUARD`, `T2-CAPTURE`, `HOST`
and `REPO-RULES`), 15 November 2026.** `REPO-RULES` M1's builder stopped under rule 1. At `db4a38fc` the verifier
passed both native matrices and Chrome day SDR, and refused Chrome day scene-linear at record 98 of 481 on its
identity: the reference holds tick 218 with alpha 0, and the capture held tick 217 with alpha 1. The builder asked
for `APPLE-TILE` to resolve it.

- **What it is.** No image moved, and neither the reference nor the content key is stale. All 481 digests of the
  failed capture equal the retained ones, and tick 217 with alpha 1 is the instant of tick 218 with alpha 0. The
  same covered content, by its content key and with the same Wasm and native binaries, passed the whole verifier
  less than an hour before ([`WATERS`' record](../../.orchestrate/evidence/WATERS/M1-baselines.json)), on the same Chrome. A fixed
  capture gives the game a synthetic time at each callback, but in the browser the game's world stops committing
  due ticks once a callback's tick work has taken 2 ms of real time, and commits the rest at the next callback
  (`with_tick_work_slice` in `examples/v1-game/src/host/world.rs`). Record 98 is one of 140 that two ticks reach.
  Its callback took 12.9 ms, the longest of the 481, where the median is 1.6 ms
  ([the planner's reading](../../.orchestrate/evidence/REPO-RULES/M1-replan/README.md)). So the page stalled inside the first
  tick, and the second waited for record 99, where the capture is level with the reference again. This is an
  inference from the callback's time: no tick was timed. `EFFECT-GRAPH` M3 met the same once, on earlier content.
- **Whose it is.** Nothing is owed by `APPLE-TILE` or `BASELINE-GUARD`: the references, the comparisons, the
  registry and the verifier are right and stay as they are, no reference is refreshed, and neither module gains
  a milestone. The defect is that real time reaches a fixed capture through the world's overload policy, which is
  `HOST`'s ([its design](HOST/DESIGN.md)). Its correction is not planned:
  [`REPO-RULES`' plan](REPO-RULES/PLAN.md#open-for-the-human) holds that as an assumption for the human. No
  milestone waits for it.
- **The rule, until that correction lands.** A failed verifier run stays failed: it writes no proof, and nothing
  from it counts towards one. The builder may set it aside as a **stalled tick** and run the verifier again, in
  place of stopping under rule 1, only when all three of these hold:
  1. *Images.* Every record of the failed Chrome set holds the retained digest. The comparison stops at the
     first refusal, so the builder reads the capture's manifest against the retained one for all 481
     (`.orchestrate/evidence/REPO-RULES/M1-replan/identity_reading.py` is one way).
  2. *Identities.* The only identity fields that differ are `tick`, `source_tick` and `alpha`, and in every
     record where they differ the reference holds a tick with alpha 0 and the capture holds the tick before it
     with alpha 1. Every other identity field of every record matches.
  3. *The change passes.* The whole verifier, unchanged, then passes on the same covered content, or on the
     same branch rebased onto a later main. That run is the proof, and it is complete: every record of every
     set, in one run. If it fails, it is a failed run of its own, judged by these tests and by the ruling
     above, and a record that fails in both runs is not a stalled tick.

  A failure that misses any of the three is rule 1's, any other identity difference included: the builder stops,
  writes the proposal in its log and ends `BLOCKED` with every run.
- **What does not change.** The comparison is not taught that the two identities are one instant, and its
  identity fields stay as they are. The verifier does not retry inside a run, accept a record from a failed run
  or skip one, and no reference is refreshed. The gate still passes covered content only on a complete pass.
- **The record.** In the module's evidence folder, for each run set aside: the failed run's record with its
  content key, revision and Wasm hashes; the failed capture's manifest; the records that differ, with both
  identities; the time of each callback from the capture's `runtime.json`, where the scratch still holds it;
  and the passing run's record. The module's log gains one line that links it. At the review of a milestone
  that carries the verifier (rule 3), the reviewer reads each record. A run set aside without one is a Must Fix.
- **`REPO-RULES` M1.** The failed half of its record is on its branch. Its builder rebases, runs the verifier
  unchanged and lands as for any covered change. None of its items changes.
- **When it ends.** With the landing that makes a fixed capture commit every due tick whatever real time its
  tick work takes. On content that holds it, a failure of this kind is evidence that the correction does not
  hold: the builder ends `BLOCKED` with the record, and a planner settles it here.

<a id="ruling-on-a-capture-that-drops-frames"></a>

**Ruling on a motion capture that drops frames under load (with `T2-CAPTURE`, `APPLE-TILE`, `BASELINE-GUARD` and
`REVIEW-ENGINE`), 16 November 2026.** `REVIEW-ENGINE` M3's builder stopped with every item passed: the
main-branch check kept failing before any pixel was compared
([its handoff](../../.orchestrate/evidence/REVIEW-ENGINE/M3-handoff.md)). The owner asked for the capture to keep every frame on
a loaded machine and to exit 75 when it is incomplete.

- **What it is.** Four runs of the verifier failed on 16 November, each at a different realtime motion arm of
  `APPLE-TILE`'s native matrices and two of them on one commit. One retained record holds 129 ring omissions
  and none from the writer, with five modules' GPU tests running beside it. No image differed and no reference
  is stale. Those arms run on elapsed time, and their six-slot readback ring fills when other GPU work delays
  readbacks ([`T2-CAPTURE`'s design](T2-CAPTURE/DESIGN.md#a-full-ring-and-a-run-that-could-not-judge)).
- **Whose it is.** The capture is `T2-CAPTURE`'s, and the work is its M3. `APPLE-TILE` and `BASELINE-GUARD`
  gain no milestone and no item: `APPLE-TILE` M10 is waiting for an attended sitting, and this must not wait
  with it. `REVIEW-ENGINE` M3 changes nothing and stays blocked until a shared run covers its commit, which the
  lead may get before M3 lands.
- **What M3's builder changes in other modules' files, and nothing else there.** In `APPLE-TILE`'s
  `scripts/apple_tile_m0.py`, `apple_tile_m1.py` and `apple_tile_m2.py`: the realtime motion arms select `wait`
  admission, and a matrix exits 75 when a capture it ran was incomplete and nothing it had compared differed.
  In `BASELINE-GUARD`'s `scripts/exact_baselines.py` and `exact_baselines_native.py`: a run whose matrix exited
  75 exits 75, its record says it could not judge and names the capture, and the scratch rule keeps that
  capture as it keeps the latest failure.
- **What does not change.** Every retained reference, compared checkpoint, sampling and equality rule, and the
  completeness rule: every submission has its image, with no ring or writer omission. The verifier still exits
  non-zero on any difference and never 75 for one. It does not retry inside a run, accept a nearer frame, skip
  a record or compare within a tolerance. The motion arms compared no pixels before and compare none now.
- **The registry.** The matrices and the verifier are pinned comparison files, so every baseline's pin changes
  and no reference moves. The builder copies the pins the guard prints and sets each `refresh` to
  `ruling-on-a-capture-that-drops-frames`, `T2-CAPTURE` and `M3`, in the landing that changes the files. There
  are no reference hashes to record.
- **Earlier rulings.** [Load variation](#a-frame-that-varies-under-load) and
  [a stalled tick](#a-fixed-capture-that-stalls) are differences, so they stay failures and their rules stand.
  Where `T2-CAPTURE`'s earlier text says a capture never holds the renderer back, its M3 section now says when
  one may. `BASELINE-GUARD`'s limit on editing the matrices binds its own builders.
- **Everyone else.** Nothing is owed. A module's own realtime capture keeps `omit` unless that module selects
  `wait`, which it may do only for a claim that every submitted image was kept.
- **Not part of this.** `.orchestrate/evidence/APPLE-TILE/M10/window-proof.py` ending "CUA app selection timed out" is
  a wait for a computer-use action in the attended window proof. It stays with `APPLE-TILE` M10's sitting.

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
- **Sea** (widened on 14 November to all water, [below](#water-as-one-system)): an open sea from waterline to horizon across calm to gale, bodies that float on it through one
  asynchronous query, a wake that persists and moves another body, transported foam from breaking waves and hull
  contact, and sea optics through the shared light contract, at full refresh with governor knobs. The owner
  added three things the same day: surf, reflection of scenery that is off screen, and a camera under water.
- **Clouds:** one provider that draws a weather-driven sky from clear to overcast and placed cloud volumes from
  the baked library with the same march and light, shades the world through one optical depth, lights correctly
  from noon to sunset under G3s, and runs shadow-only, at full refresh with governor knobs. The owner added mist
  and fog on 14 November ([the atmosphere as a whole](#the-atmosphere-as-a-whole)).

**Consolidation** means one module per provider. The two cloud paths become one `CLOUDS` module. Judged from first
principles on 14 November, it is one medium and not a hybrid of two paths: the deterministic weather field is the
authority, convective cells, a deck and shapes cooked from density volumes are sources of its one density, and
which a sky has is the game's data ([CLOUDS](CLOUDS/DESIGN.md#judged-from-first-principles)).

## The atmosphere as a whole

The owner, 14 November 2026: "Oh and for clouds we also need to be taking into account mist / fog. We want a
plan that's able to deliver on top quality atmosphere." Sky, clouds, haze, mist and fog are one medium, and its
state has two kinds, each with one owner.

| Kind of state | Owner | Why there |
|---|---|---|
| What is in the air and where: cloud, fog and mist as one condensate, the amount and kind of haze, wind, and the weather state a game steps | `CLOUDS` | Fog is cloud at the ground. One field gives one density, one optical depth toward the Sun, one light model and one headless query, so a deck can come down onto the hills and a game can ask how far it can see |
| What light does in the air: the gases and the aerosol's optics, the charts, the sky, and the transport from the camera to any point and from the Sun to any receiver | `ATMOSPHERE` | Near the camera fog overlaps the air, which C08 requires one owner to integrate jointly, and every surface already reads that owner's aerial product |
| The surface field and the fog table | The game | Content, like the type table |

The picture is made by two integrators that share the one density and light: `ATMOSPHERE`'s view volume to a
handover a few hundred metres out, where fog meets the ground, water, props and the camera, and `CLOUDS`' trace
beyond, where mist is a soft shape on the land. The assumptions this replaces are listed in
[CLOUDS' design](CLOUDS/DESIGN.md#judged-again-for-the-whole-atmosphere) and
[ATMOSPHERE's](ATMOSPHERE/DESIGN.md#what-this-replaces). In short: fog is no longer a separate provider below
the cloud base; the camera and the land may stand in cloud; haze is no longer a constant; shafts of light are
no longer left out; `ATMOSPHERE` gains two milestones of its own after G3s; and RENDER-SPEC §5's joint froxel
integrator, which had no owner, is `ATMOSPHERE`'s view volume. No other module's design, plan or item changes:
`WATERS` reads fog through the lent aerial operation and writes the depth the trace stops at, and a
`SCENE-VIEWS` mirror carries clear air until its own plan says otherwise. The V1 exit scene declares none of
this and does not change.

## Water as one system

**Decision, 14 November 2026: Thrum's water is one system, designed in [WATERS](WATERS/DESIGN.md) and built by
four modules.** The owner asked that the sea work think "about water as a whole. So puddles, rivers, water falls,
streams, lakes, and more", "built cohesively". Until then `SEA` planned an open sea, and inland water lived in
[older designs](../design/modules/WATER.md) no module was planned from.

The system is a map that says where water lies and how it moves in bulk, waves that ride on it, material it
carries, and one optics. Every kind of water is data for those four, so there is one surface, one optics, one
foam and one query, and a new kind of water adds data and never a renderer. The modules follow the system's
state, each kind with one owner:

| Module | Owns | Why it is its own |
|---|---|---|
| `SEA` | The wind's wave field and its oracle | It depends on nothing about where water lies, every body reads it, and its first two milestones were planned and under way. They are unchanged |
| `WATERS` | The map, the surface, optics, interaction waves, foam and the query, with surf, mirrored scenery and the view from under water, proved on a sea, a beach, a lake, a pond and a hand-made channel | The shared system has to exist once, and it is proved on still water, where a bed and a shore first appear, before running water is drawn on it |
| `RIVERS` | The flow solver that writes charts, and waterfalls | A prepared flow is a different kind of work, on the CPU and in files of its own, and falls are a new kind of geometry |
| `PUDDLES` | Rain's hollows and their fill, rain's rings, and the wetness of ground | Its state is a game's own, stepped on the CPU, and it reaches into the forward pass's surfaces, which no other water module touches |

`SEA`'s old M3 to M10 became `WATERS`' M2 to M11, written again for every body of water. A module called `WATER`
would share its evidence folder's name with the 1 November `.orchestrate/evidence/water` on this Mac's file system, so
the module is `WATERS`. The older water designs and [the water decision](../design/decisions/E1-GPU-WATER.md) are
evidence `WATERS`' design judged; where they differ, that design holds.

**Build order.** `SEA` M1 and M2; `WATERS` M1 and M12 (surf on the CPU) beside `SEA` M2; `WATERS` M2 to M7, M13
(surf drawn), M8, M9, M15 (under water) and M16 (light under water); `RIVERS` from `WATERS:M9`, with its solver earlier; `PUDDLES` from
`WATERS:M8`, with its CPU milestones earlier; `WATERS` M10, M14 (mirrored scenery, on `SCENE-VIEWS:M3`) and M11 close the shared system. `CLOUDS`
M11 needs `WATERS:M8`. `WATERS`' milestones keep the numbers other plans name, so its file order, and not its
numbering, is its build order.

**The owner's three additions, 14 November 2026.** "Scenery reflections we must have, that's a main stay in high
quality modern games. That's the standard we're after. Camera being able to go under water we need to support
as well. Breaking waves is something we'd like to have as well." [WATERS](WATERS/DESIGN.md#the-request) settles
each: surf comes from the sea's own long waves, carried over the bed by a prepared phase lag, so `SEA` is
unchanged and asked for nothing; the view from under water is water's own work; and scenery that is off screen
is read from a second draw of the scene, which is the one thing here water cannot own and is why `SCENE-VIEWS`
exists. `CLOUDS` is unchanged: clouds still reach a reflection, and the window overhead from under water,
through the environment product.

**Light under water, 14 November 2026.** The first plan for the camera under water left out caustics, shafts
of light and an exposure that follows the light down, which a shipped game's underwater view has. They are
one mechanism and all `WATERS`' ([its design](WATERS/DESIGN.md#light-under-water-caustics-shafts-and-the-meter),
M16): the waves' focusing of the Sun, carried down from the surface the water is drawn from. It reaches what
lies under water through `ATMOSPHERE`'s direct-Sun path, so a shadow holds no net, stands in the water as
shafts in the water's own composition, and the camera's meter reads the light at the lens.
[Rulings 5 to 7](#seam-rulings-for-waters) settle what that asks of `ATMOSPHERE` and `APPLE-PRESENTATION`.

## Module briefs

Briefs for the modules not yet planned. Modules with a design are linked from the table above.

### APPLE-PRESENTATION M4–M11 (G4)

Planned on 14 November 2026: [design](APPLE-PRESENTATION/DESIGN.md#m4-to-m12-settled-14-november-2026),
[plan](APPLE-PRESENTATION/PLAN.md). [The drafted milestones](../../.orchestrate/evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md)
became done-when items, with the drafted M10 split in two: the order is M4 (extent and generation transitions),
M5 (dynamic scale), M6 (EDR), M7 (the governor and its knob registry), M8 (MetalFX), M9 (prepared paths and
launch-to-ready), M10 (cooked shader libraries and parallel preparation), M12 (the pipeline archive) and M11
(the paired proof, which closes G4). Only two wait on another module: M8 on `APPLE-TILE:M11` and M11 on
`APPLE-TILE:M12`. `SHOWCASE` M7 now waits for `APPLE-PRESENTATION:M11`, and `WATERS` M11 and `CLOUDS` M12 for
its M7 and M8. The owner's look at EDR on the built-in display is asked for in M11 and recorded as an
observation; under decision 4 of [the brief](BRIEF.md#decisions) it gates nothing, G4's close included.

### CULLING (G3c)

Planned on 11 November 2026: [design](CULLING/DESIGN.md), [plan](CULLING/PLAN.md). Its planner settled the open
decision this brief carried: [each entity has one visibility authority](CULLING/DESIGN.md#one-visibility-authority-per-entity).
G3c closes on the scenes that exist, the mixed-mesh field and the valley as it stands, so it does not wait for
`SHOWCASE`.

### CONTENT-LOOP (G5)

Planned on 13 November 2026: [design](CONTENT-LOOP/DESIGN.md), [plan](CONTENT-LOOP/PLAN.md). The first sea or
cloud frame that samples a cooked product needs its M1; `SHOWCASE` needs its last milestone, M3. Parked and
staying parked: CAS garbage collection and leases on the store, layered EAC, parameter files as a product kind.

M4, added on 16 November 2026, is built at once and ahead of M2: the cooker's watcher scans its source tree on
its own clock, because macOS left saves unreported on the fleet's Mac and `thrum-cook`'s watcher test failed the
full check for every module ([diagnosis](../../.orchestrate/evidence/CONTENT-LOOP/M4-diagnosis.md)). `ATMOSPHERE` M9 is Blocked
on it. It has no `depends_on` entry, so that its proved items stand, and the lead unblocks it when M4 is Done.

### SHOWCASE (G7)

Planned on 13 November 2026: [design](SHOWCASE/DESIGN.md), [plan](SHOWCASE/PLAN.md). Its milestones are M5 to M8;
M1 to M4 were the route slice, built before the loop under
[the route decision](../design/decisions/G7-SHOWCASE-ROUTE.md) and kept in
[its record](../../.orchestrate/evidence/SHOWCASE/ROUTE-SLICE-RECORD.md). M5 and M6 need `CONTENT-LOOP:M2` and `CULLING:M3`; M7
needs the closing milestones it builds on and M8 the last milestone of every other track 1 module. The swarm game, `SWARM`, starts when M8 is Done.

### EFFECT-GRAPH (track 2 foundation)

Planned on 11 November 2026: [design](EFFECT-GRAPH/DESIGN.md), [plan](EFFECT-GRAPH/PLAN.md). The first sea or
cloud frame needs its last milestone, M3.

### SEA, WATERS and CLOUDS

`SEA` was planned on 13 November 2026 and narrowed to the wave field on 14 November: [design](SEA/DESIGN.md),
[plan](SEA/PLAN.md). `WATERS` was planned on 14 November: [design](WATERS/DESIGN.md), [plan](WATERS/PLAN.md).
`CLOUDS` was planned on 13 November: [design](CLOUDS/DESIGN.md), [plan](CLOUDS/PLAN.md), from
[its brief](CLOUDS/BRIEF.md). It takes from `WATERS`' design that water writes scene depth and comes first in a
scene's contribution list, and it puts clouds into the environment reflection product water reads.

### SCENE-VIEWS

Planned on 14 November 2026: [design](SCENE-VIEWS/DESIGN.md), [plan](SCENE-VIEWS/PLAN.md). Three milestones:
entities in a view on the Mac, cooked placements with a mirror example, then Chrome, cost and the close.
`WATERS` M14 needs its last, M3. Its planner judged ray query for the Apple recipe and did not select it:
the raster view has to exist for Chrome whatever else does, and wgpu's ray query is experimental. Later
consumers whose camera stands elsewhere (a probe, a portal, a monitor in the world) are refused today: a view
reads the frame's terrain shadows, which are its own only on the main camera's vertical. Serving one adds a
slot with terrain shadows of its own, and changes no request.

### RIVERS and PUDDLES

Both build on [the water system](WATERS/DESIGN.md#what-the-later-modules-build-on) and add no second surface,
optics, foam or query. Each was planned by a `plan-module` run from its brief, with research into shipped
and published practice and reproductions of its own mechanisms. A cue that a seam of `WATERS` cannot carry is
settled here, with `WATERS`.

**`RIVERS`** was planned on 14 November 2026: [design](RIVERS/DESIGN.md), [plan](RIVERS/PLAN.md). Seven
milestones: the solver, falls and a river that answers its source on the CPU, after `WATERS:M1`; then the
island's rivers, running water in a calm, the waterfall, and the close, after `WATERS:M9`. Its brief asked for
a stream from a spring that pools, quickens and reaches the lake or the sea, water that reads as running, floats
carried, foam where it loses height and at rocks, and a waterfall with its sheet, plunge, foam and rings. Its
planner settled the brief's open mechanisms, and [seven rulings](#seam-rulings-for-rivers) say what that asks of
`WATERS`. Mist at a fall is not planned and is [the owner's](#owner-decisions).

**`PUDDLES`** was planned on 14 November 2026: [design](PUDDLES/DESIGN.md), [plan](PUDDLES/PLAN.md). Eight
milestones: puddles, then wet ground and a drop's rings, on the CPU after `WATERS:M1`; the yard's puddles, the
rain's rings, a footfall's rings and the wet ground on the island after `WATERS:M8`; the V1 valley after
`SHOWCASE:M5`; and the close,
after `WATERS:M10`, `M14` and `M16`. Its brief asked for rain that wets the ground it can reach and leaves
shelter dry; puddles that form in the ground's hollows as it rains, join when they overflow, ripple under
drops and feet, reflect the sky and what stands in them, and dry out; sand that darkens where waves have
run, under the sheen and foam `WATERS` draws there as [its film](WATERS/DESIGN.md#the-surface-and-the-frame-contract);
and the same on the island and in the V1 valley behind a selection, in Chrome too. Its planner settled the
brief's open mechanisms, the first of them how puddles merge and part, and
[eight rulings](#seam-rulings-for-puddles) say what that asks of `WATERS`, the forward shader and the two
games. Rain seen falling is not planned and is [the owner's](#owner-decisions).

## Track 3: the swarm-survival game, after V1

Added 10 November 2026. [The brief](../design/NEXT-GOAL-SWARM-GAME.md) is the owner's page; this section turns it into
modules. Nothing here gates the exit. Until 14 November nothing here started before it either; since then the
engine modules do, transport among them since 15 November, and only the game waits.

<a id="the-split-of-14-november-2026"></a>

### The split of 14 November 2026

The owner, 14 November 2026: "Let's move the 2d engine side of things to before v1 and decouple them. But we
don't want to start working on the swarm game till v1 lands."

| Part | Modules | Starts |
|---|---|---|
| Reusable engine work | `SIM-2D`, `RENDER-2D`, `SESSION`, `GAME-UI` | As soon as the milestones it uses are Done. None waits for `SHOWCASE` |
| Transport | `TRANSPORT` | Before or after V1, when a builder slot is free. It waits for no milestone and is last in line for a builder |
| The game | `SWARM`, all eight milestones, the owner's pick of a concept included | When `SHOWCASE:M8` is Done |

<a id="transports-timing"></a>

**Transport's timing.** The owner, 15 November 2026: "it can come before V1 if we have space for it, but it's
not urgent". So `TRANSPORT` M1 has no edge to `SHOWCASE`, and `TRANSPORT` is the lowest priority of every
module: the lead starts its builder only when a builder slot is free. The ruling replaces "transport stays after
V1" in decision 1 of [the brief](BRIEF.md#goal-and-exit-rule), on which this plan had held M1 behind
`SHOWCASE:M8` since `TRANSPORT`'s design review earlier that day. Transport is still no part of the V1 exit.

An engine milestone keeps an edge only where it uses another milestone's output:

| Milestone | Waits for | Because |
|---|---|---|
| `SIM-2D` M1, `TRANSPORT` M1 | Nothing | Core's simulation (G2S) and the shells they run in are Done. `TRANSPORT` M1 starts when a builder slot is free, [as above](#transports-timing) |
| `RENDER-2D` M1 | `EFFECT-GRAPH:M3`, `APPLE-PRESENTATION:M3` | It refuses a contribution that declares the after-opaque stage, which M3 defines, and it declares a scene with no reconstruction, which exists in the game from TAAU on. It no longer waits for `APPLE-TILE:M12`: it uses the split graph and the pre-tone input, which are Done, and nothing of M10 to M12 |
| `RENDER-2D` M4 | `CONTENT-LOOP:M1` | The sheet product is lent as a lease and switched by the publication, both M1's. It no longer waits for `CONTENT-LOOP:M3`, which closes G5 and adds nothing a sheet uses |
| `RENDER-2D` M5 | `SIM-2D:M1` | It draws the Crowd trace |
| `SESSION` M1, M2, M5 | `SIM-2D:M3`, `TRANSPORT:M1`, `TRANSPORT:M2` | As before. M2 and what follows it wait on `TRANSPORT`, which is last in line for a builder, and not on V1 |
| `SESSION` M7 | `RENDER-2D:M5` | The windowed match is the crowd view with a session attached |
| `GAME-UI` M1 | `RENDER-2D:M5`, `APPLE-PRESENTATION:M6` | It runs the sprite pass a second time over the sprite scene M5 leaves, and it draws inside the EDR present pass, which M6 builds |

Engine work that the game's plan had carried moved to engine modules, so that it is built and proved without
the game and the game changes no engine crate for it:

- **A driven world**, a world that commits the ticks its match names, is `SESSION` M7
  ([ruling 3](#seam-rulings-for-session)), proved with the reference crowd in a window. It was `SWARM` M7's.
- **One iPhone shell for every game** is `GAME-UI` M6 ([ruling 5](#seam-rulings-for-game-ui)), proved with the
  crowd view as a second app. It was `SWARM` M6's.

Two things stay with the game because they name it: the repository guard's entry for a second external game,
and whatever the window and page still assume of the V1 game when `SWARM` first runs.

Where an engine item was worded for this game it is now worded for any game: `GAME-UI`'s level-up choice is a
modal choice of four cards. The engine modules' proof scenes are the reference crowd, the crowd view, and the
sprite and interface scenes in the V1 game; none is the swarm game, and no engine check builds a package of it.

**Shared files.** `RENDER-2D` M1 and M4 and `GAME-UI` M1 change the frame runtime and the content loop while
`APPLE-TILE` M10 to M12, `APPLE-PRESENTATION` M4 to M12 and `CONTENT-LOOP` M2 and M3 are built in the same
files. That is not an edge: each builder integrates on what has landed and keeps the other's tests passing, as
the rulings below say. If it costs V1 too much, the owner can ask for `RENDER-2D` M1 to wait for
`APPLE-TILE:M12` ([RENDER-2D's plan](RENDER-2D/PLAN.md#open-for-the-human)).

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
| `CREATOR`, with `PLATFORM-RUNTIME-IOS` and `PLATFORM-RUNTIME-WEB` | An app runner on all three targets for a game that brings no sky kit. The iOS shell lives in `examples/v1-game-ios` today and serves one game. Built in `GAME-UI:M6` for iOS and for a second program on the window and the page, under [its ruling 5](#seam-rulings-for-game-ui); what is left when the game first runs is `SWARM:M2`'s, under [its ruling 2](#seam-rulings-for-swarm) | Any second game |
| Repository guard | `.config/architecture.toml` names one external game; it must admit a second. Built in `SWARM:M2` under [ruling 1](#seam-rulings-for-swarm) | `SWARM` |
| Core (W1) and `SIM-2D` | A world image: the bytes the digest covers, with a way back into a fresh runtime in another process. Built in `SESSION:M1` under [ruling 1](#seam-rulings-for-session) | `SESSION` |
| `SIM-2D` | The reference crowd's player movement as one function its own rule calls, so a predictor can call it too. Built in `SESSION:M4` under [ruling 2](#seam-rulings-for-session) | `SESSION` |
| Core (F1 and W1), with `CREATOR` | A world lane that commits ticks when its driver says, in place of its own clock, for a host's and a client's copy of a match. Built in `SESSION:M7` under [ruling 3](#seam-rulings-for-session) | Any game in a match |

`RENDER-2D`'s own sprite pass is declared in the one graph by `thrum-render`, as the 3D scene's forward pass is.
An engine module outside `thrum-render` attaches to a 2D scene through `EFFECT-GRAPH`'s seam, at the upload and
simulate stages; a 2D scene has no shadow, opaque or after-opaque stage. A second way to declare passes is not
built.

<a id="seam-rulings-for-render-2d"></a>

**Seam rulings for `RENDER-2D`.** `RENDER-2D` needs three changes inside state other modules own, which no
interface offered. Each is built by the `RENDER-2D` builder of the milestone named, on what the owning modules
have landed by then, with the behaviour [RENDER-2D](RENDER-2D/DESIGN.md#mechanisms) defines. No item of
the owning module changes, its exact comparisons are not loosened, and a scene that asks for none of it compiles
to the same passes and draws the same bytes. Once built, each belongs to its owner: later work there keeps its
tests passing, and a V1 builder who lands in the same files afterwards keeps `RENDER-2D`'s. The rulings were
written while V1 was being built, so that no V1 plan changed; `CONTENT-LOOP`'s design says each ask becomes a
milestone there, and ruling 2 replaces that for the sheet product.

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

**Seam rulings for `SESSION`.** `SESSION` needs three changes inside state other modules own, which no interface
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
   The types registered under the profile before this ruling, the crowd's and the fixtures of `thrum-core`'s
   tests, gain the decoding half in the same milestone. A test `SIM-2D` was accepted with may change for that
   alone: its types gain the decoding half and its code follows the encoding interface's new signatures, and
   no test, assertion, input or expected value is removed or altered.
2. **The reference crowd in a match, in `SESSION:M4`** (`SIM-2D`; `examples/crowd-2d`). The player's movement
   becomes one function in the rule package, called by the crowd's own rule and by
   [the predicted controller](SESSION/DESIGN.md#11-the-predicted-controller). It is a change of shape only: the
   rule package still passes `SIM-2D`'s guard with nothing suppressed.
3. **A driven world, in `SESSION:M7`** (core F1 and W1, with `CREATOR`; `thrum-core`'s world lane and clocked
   world, the frame host's world observer in `thrum-render`, and `thrum-app`). A world may be
   [driven](SESSION/DESIGN.md#12-a-driven-world): its driver names how many ticks to commit at each advance,
   none, one or several, and where the presentation stands between the last two. A driven world raises no
   overload for ticks nobody asked for. A world on the fixed clock behaves as it did, and its tests pass
   unchanged. The engine side names no session and no link: the driver is the game's, which asks `SESSION`.
   Until 14 November this was `SWARM`'s ruling 4.

<a id="seam-rulings-for-game-ui"></a>

**Seam rulings for `GAME-UI`.** `GAME-UI` needs five changes inside state other modules own, which no interface
offered. Each is built by the `GAME-UI` builder of the milestone named, after `RENDER-2D`'s last milestone and,
for ruling 1, `APPLE-PRESENTATION:M6`, on what the owning modules have landed by then, with the behaviour
[GAME-UI](GAME-UI/DESIGN.md#mechanisms) defines. No item of the owning module changes and its exact comparisons are not loosened. A host prepared with
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
5. **One iPhone shell for every game, in `GAME-UI:M6`** (`PLATFORM-RUNTIME-IOS` and `CREATOR`, with what
   `COOKED-DRAW`, `RENDER-2D` and this module added; `examples/v1-game-ios`, `thrum-app`, and the capture and
   packaging scripts). The UIKit lifecycle, touch delivery, the render owner and its surface lease, rotation,
   device-loss recovery and the reports of extent, density and insets move into `thrum-app`,
   [generic over a game](GAME-UI/DESIGN.md#12-one-shell-for-every-game) as the Mac shell is.
   `examples/v1-game-ios` becomes its first user and passes every simulator check those modules left,
   unchanged; the crowd view with a status display is its second, and runs from the window and the page too.
   Neither keeps a copy of that policy. Until 14 November this was `SWARM`'s ruling 3.

<a id="seam-rulings-for-swarm"></a>

**Seam rulings for `SWARM`.** `SWARM` needs two changes inside state other modules own, which no interface
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
   program on both, `GAME-UI` gives them events, density, insets and text drops, and `GAME-UI:M6` runs a
   second program with an interface on them. Whatever they still assume
   of the V1 game or the sky kit when a game with a 2D scene, an interface and no sky kit runs, is capturable
   and is packaged for Chrome, is made a property of the game.

Ruling 1 is overtaken by [track 4](#the-proving-ground-and-where-a-game-lives): games live in their own
repository, so the guard has no second game to admit, and `SWARM` is re-planned onto the game gate before its M2.

Rulings 3 and 4, the iOS shell for a second game and a driven world, moved on 14 November to
[`GAME-UI`'s ruling 5](#seam-rulings-for-game-ui) and [`SESSION`'s ruling 3](#seam-rulings-for-session): they
are engine work, built before the game starts.

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
| **Driven world**: a world whose driver names how many ticks to commit at each advance and where the presentation stands between the last two, in place of the lane's fixed clock. The engine side names no session | core and `CREATOR`'s app runner → any game in a match, by [ruling 3](#seam-rulings-for-session) | [SESSION](SESSION/DESIGN.md#12-a-driven-world); C02 |
| **Session**: a host and a client driven by time: join and leave, roster events for each player slot, the host's admitted set for each tick, how many ticks to commit now, the predicted controller and its corrections, and commands that happen once in order. A game says which of its actions are commands | `SESSION` → `SWARM` | [SESSION](SESSION/DESIGN.md#interfaces); C13, A0 and the A5 decisions |
| **Connection state**: what a screen can show for this peer and for each slot, with the reason for a refusal or a close | `SESSION` → `SWARM`, which shows it in `GAME-UI`'s nodes; `GAME-UI` links neither | [SESSION](SESSION/DESIGN.md#9-negotiation-and-leaving) |
| **Rule set in the gameplay manifest**: a match names its deterministic rule set and build, by an identity derived from their sources, and refuses a peer that differs. A proposed addition to C13, written into the contract by `SESSION:M6` | `SESSION`, with `SIM-2D`, core and the game | [SESSION](SESSION/DESIGN.md#interfaces) |
| Quality knobs | `RENDER-2D` → the governor | C04; the interface above, unchanged. `RENDER-2D` registers none: a 2D scene has no scale, cadence or step budget to lower |
| The app runner and typed controls, with one iOS shell generic over a game from `GAME-UI:M6` | `CREATOR` and `PLATFORM-RUNTIME-IOS` → `SWARM`, any game | C11, [CREATOR](CREATOR/DESIGN.md); [GAME-UI](GAME-UI/DESIGN.md#12-one-shell-for-every-game) |
| Engine capture, one for each peer process | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE/DESIGN.md); unchanged |

### Build order after the V1 exit

This heading is kept for the pages that link to it. Since [14 November](#the-split-of-14-november-2026) steps
1 to 4 below are built beside V1, each milestone as soon as the ones it uses are Done, and only step 5, the
game, waits for `SHOWCASE`. `TRANSPORT` is the exception in priority, not in edges: it starts
[when a builder slot is free](#transports-timing). A design that changed needs a review that returns Design
Approved before a builder starts its module.

1. **The two proofs that can overturn the plan.** `SIM-2D`, first milestone: one input trace gives equal digests
   on Mac, in Chrome and on the iOS simulator at the brief's crowd size, with tick, rewind and replay costs
   recorded. `TRANSPORT`, first milestone, when a builder slot is free: one native host holds a Mac client, a
   Chrome client and an iOS-simulator client over real sockets under the A0 conditioner. They share no files
   with each other or with the renderer.
2. **Sprites.** `RENDER-2D`, which builds its asks of `APPLE-TILE` and `CONTENT-LOOP` itself under
   [the seam rulings](#seam-rulings-for-render-2d). It starts after `EFFECT-GRAPH:M3` and
   `APPLE-PRESENTATION:M3`; its last milestone draws `SIM-2D`'s crowd and waits for `SIM-2D`'s first.
3. **A match.** `SESSION`, after `SIM-2D`'s last milestone, since its first milestone works in the same files.
   Its matches on the in-memory conditioned link follow `TRANSPORT`'s first milestone, which delivers that link,
   and its socket milestones follow `TRANSPORT`'s second. Its last milestone, the windowed match on a driven
   world, follows `RENDER-2D`'s last.
4. **Interface, and a shell for every game.** `GAME-UI`, after `RENDER-2D`'s last milestone, whose sprite scene
   its first milestone checks against, and after `APPLE-PRESENTATION:M6`, whose EDR present pass it draws in.
   It builds the interface-stage and font-product asks itself, and in its last milestone the shared iPhone
   shell, under [the seam rulings](#seam-rulings-for-game-ui).
5. **The game, after V1.** `SWARM` opens with the owner's choice of concept, after `SHOWCASE:M8`. Then a
   single-player slice on `SIM-2D`, `SESSION`'s first milestone and `RENDER-2D`, the full run on `GAME-UI`'s
   fourth milestone, the iPhone simulator on its last, a rehearsed match on `SESSION`'s fourth milestone,
   co-op on sockets on its last, and the close. It builds its asks of the guard and the shells itself under
   [the seam rulings](#seam-rulings-for-swarm).

A lone builder takes the engine modules in that order, with transport last: `SIM-2D`, `RENDER-2D`, `SESSION`'s
first milestone and `GAME-UI`, then `TRANSPORT` and the rest of `SESSION`. With two builders, V1's milestones
come first whenever one can start, and `TRANSPORT` takes a slot only when no other module has a milestone
ready for it ([the owner, 15 November](#transports-timing)).

**Reaching the targets.** `TRANSPORT`, `SESSION` and `SWARM` open local sockets and drive Chrome, which Codex's
`workspace-write` sandbox does not allow without a request for each run. The owner settled it on 14 November:
[the project settings](../../ORCHESTRATE.md) give Codex full access, so no request is needed. `TRANSPORT` puts every socket proof
behind [one proof command](TRANSPORT/DESIGN.md#proof-and-builder-handoff) and keeps its own sockets out of
`./dev check`. `SESSION` does the same with [one match command](SESSION/DESIGN.md#proof-and-builder-handoff), through
`TRANSPORT`'s relay. `SWARM` puts every proof that needs a window, Chrome, the simulator or a socket behind
[one command of its own](SWARM/DESIGN.md#proof-and-builder-handoff), which reuses `SESSION`'s match harness and
the relay.

**Who gates a socket test.** The module that owns a test decides whether it runs in `./dev check`, and no module
makes a claim about another's tests. There is no repository-wide ban on sockets in `./dev check`: the cooker's
loopback dev-server tests (`CONTENT-LOOP`: `thrum-cook`'s `serve_and_watch` and the fetched-store cases of
`v1-game`'s `cooked_assets_gpu`) have run there since 6 November and stay. Where `TRANSPORT`, `SESSION` or `SWARM`
says nothing in `./dev check` opens a socket, starts Chrome or calls the simulator, it means what that module
adds.

### What finished means (track 3)

Proposed, for the owner to change: two to four players, on any mix of Mac, Chrome and the iOS simulator, join one
host and play a whole run of an original swarm survivor together, with thousands of enemies and projectiles on
screen, smooth at the display's full refresh on the Mac, and the peers still agree under the A0 conditioner. The
engine modules hold no game types, and a second small 2D scene builds on them without `SWARM`.

### Module briefs (track 3)

A `plan-module` run turns each brief into a design, a plan and a done-when file. Costs are recorded as tripwires;
no budget below is a gate. The quality bar is a shipped game, named for each milestone by the planner.

#### SIM-2D

Planned on 13 November 2026: [design](SIM-2D/DESIGN.md), [plan](SIM-2D/PLAN.md). It waits for nothing. Its planner settled the two
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
drop from a link, a launch argument or a paste. [A probe](../../.orchestrate/evidence/GAME-UI/planning-probe.md) sized a cooked
font and checked the rules for spacing, event ownership and direction keys. Six milestones, all after
`RENDER-2D:M5`, whose sprite scene the first checks against, and `APPLE-PRESENTATION:M6`, whose EDR present pass
the interface is drawn in. The sixth, added on 14 November, is the iPhone shell shared by every game. The asks of `APPLE-TILE`,
`RENDER-2D`, `CONTENT-LOOP` and the shells are built in M1 to M5 under
[the seam rulings](#seam-rulings-for-game-ui). `SWARM` needs M4 for its screens on the Mac and in Chrome and M6
for the simulator. Five findings bind `SWARM`'s planner: text exists only at the sizes and scales a font was
cooked at, and a size that was not cooked is refused; the interface blends in linear light, so a half-transparent
panel is lighter than in a paint tool; whether a choice pauses the run is the game's, and the interface only
says what was activated; there is no text field, scrolling list or transition system; and the app runner
`SWARM` gets must deliver events, density, insets and text drops as the V1 game's shells now do.

#### TRANSPORT

Planned on 13 November 2026: [design](TRANSPORT/DESIGN.md), [plan](TRANSPORT/PLAN.md). Its planner settled the
three choices this brief carried: Quinn with `web-transport-quinn` on the host and a hand-written binding of the
browser's `WebTransport`; one I/O thread that meets the world lane only at bounded queues; and no WebSocket
fallback. [A probe](../../.orchestrate/evidence/TRANSPORT/planning-probe.md) reached one host from the Mac, Chrome and the
simulator with datagrams, so the fallback this brief named, input on the reliable stream for Chrome, is held in
reserve and not built. Two milestones, the first waiting for nothing and started
[when a builder slot is free](#transports-timing); `SESSION` needs M1 for the in-memory conditioned link and M2 before its socket
milestones. M1 also delivers the three socket adapters and a cost record under each congestion controller. Four findings bind `SESSION`: a join's baseline
should flow from the host down; a frozen browser page keeps its link open, so silent input is `SESSION`'s to time
out; a datagram can arrive long after it was sent, so old input and state are rejected by content; and a close can
cut off the last reliable messages, so one that must arrive is answered before the link is closed.

#### SESSION

Planned on 13 November 2026: [design](SESSION/DESIGN.md), [plan](SESSION/PLAN.md). Its planner settled the three
choices this brief carried: the crowd is drawn at the confirmed tick and only a player's own character is
predicted; digests are compared twice a second to start, a setting of the match; and a modal choice is a command
on the reliable lane, admitted at one tick and sent to every peer in that tick's set.
[A timing model](../../.orchestrate/evidence/SESSION/planning-probe.md) settled two more: the host's admitted input travels as
datagrams that repeat what a client has not acknowledged, and a client sizes its input lead from what the host
reports. Seven milestones; the seventh, added on 14 November, is a windowed match on a driven world. `SWARM`
needs M7 for co-op and can rehearse on the in-memory link from M4. The world
image and one function of the reference crowd are built in M1 and M4 under
[the seam rulings](#seam-rulings-for-session). Four findings bind `SWARM`'s planner: a client's world must step
when the session says, which M7's driven world allows; a hero the crowd can push is
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
rehearsed match (after `SESSION:M4`), the iPhone simulator (after `GAME-UI:M6`), co-op on sockets (after
`SESSION:M7`), and the close with the owner's play and the record that ends track 3. Of the asks this brief
named, the guard and what the window and page still assume are built in M2 under
[the seam rulings](#seam-rulings-for-swarm); the iPhone shell and the driven world moved to `GAME-UI` M6 and
`SESSION` M7 on 14 November.

## Track 4: the code review and the game's separation

Added 14 November 2026 from two requests of the owner's that day. On the
[whole-repository code review](../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md) (20 High, 81 Medium, 61 Low and
17 Nit findings, in thirteen themes T1 to T13): "can you not just plan and drive that work?" On the game: "it
sounds like we're blurring the lines between the engine and the game a ton. Please let's separate out the game as
much as we can, it shouldn't even sit in the same code base ideally." Six modules, rows 26 to 31. None gates the
V1 exit, and none changes an item of another module.

### Why new modules

The review's findings sit in code that existing modules own, and eight of those modules have builders at work. A
module's design is approved word for word, so adding a milestone and its mechanism to an owning module's design
would stop its builder until the design is reviewed again. The fixes are therefore cut into new modules by
isolation, so that each can be built without colliding with the others, and built inside the owners' code under
[the rulings below](#seam-rulings-for-track-4), as `WATERS`, `RENDER-2D` and `GAME-UI` build inside other modules'
state. Ownership does not move: once built, each fix belongs to the module that owns the code, and later work
there keeps its tests passing.

### Modules and what each owns (track 4)

| Module | Owns (state and code) | Builds inside |
|---|---|---|
| `REPO-RULES` | The shape rules, their baseline and the `--zero` proof (`scripts/shape_rules.py`, `.config/shape-baseline.json`); the evidence size rule, the bulk manifest and its restore command; the one rename and split of names and files; repository layout ([design](REPO-RULES/DESIGN.md)) | `scripts/repo_check.py`; every crate, for the rename only |
| `REVIEW-ENGINE` | Nothing of its own: the review's fixes in `thrum-core`, `thrum-network`, `thrum-sea`, `thrum-sea-model`, `thrum-asset`, `thrum-asset-gpu`, `thrum-cook` and `thrum-controls`, and the iPhone app's device request ([design](REVIEW-ENGINE/DESIGN.md)) | Core (G1, G2S), `SEA`'s and so `WATERS`' crates, `CONTENT-LOOP`'s and `COOKED-DRAW`'s, `PLATFORM-RUNTIME-IOS`'s |
| `TEST-SUPPORT` | `crates/thrum-test-support`: the GPU gate and the test device. `crates/thrum-fixtures`: fixtures shared between packages, as plain data. The registration of both and the `test-crate-edges` rule on who may depend on them ([design](TEST-SUPPORT/DESIGN.md)) | Every crate's tests |
| `REVIEW-RENDER` | Nothing of its own: the review's fixes in `thrum-render` ([design](REVIEW-RENDER/DESIGN.md)) | `APPLE-TILE`'s, `APPLE-PRESENTATION`'s, `ATMOSPHERE`'s, `CULLING`'s and `EFFECT-GRAPH`'s files |
| `HOST` | The device recipe and its fault watch; the world lifecycle policy for pause and overload; what a surface outcome means; render-once and await-drawn; the contract a game implements to be hosted ([design](HOST/DESIGN.md)) | `thrum-render`'s capabilities (`APPLE-TILE`), `thrum-core`'s clock, `thrum-app` (`CREATOR`, `PLATFORM-RUNTIME-MAC`, `PLATFORM-RUNTIME-WEB`), the iPhone app (`PLATFORM-RUNTIME-IOS`), the V1 game's host code |
| `GAME-SPLIT` | The game gate and the games repository's layout; the rule that no engine package reaches a game; the proving ground as the engine's own test content, with the frozen qualification content the exact baselines are drawn from; the move of the game ([design](GAME-SPLIT/DESIGN.md)) | `examples/v1-game`, `examples/v1-game-ios`, `.config/architecture.toml`, the engine's tests that read game files, `TEST-SUPPORT`'s `thrum-fixtures` (its first fixture) |

### Where each theme of the review is fixed

| Theme | Fixed by |
|---|---|
| Step 1, the canonical shapes and their rules | The shapes are in [patterns](../../.agents/skills/thrum-engineering/references/patterns.md); the rules are `REPO-RULES` M1 |
| Confirmed bugs | H01: `REVIEW-ENGINE` M1 (iPhone features and limits), then `HOST` M1 (one recipe). H14: `REVIEW-ENGINE` M1. H09, M41, H11: `REVIEW-RENDER` M1 |
| T1 Rust and WGSL records | `REVIEW-ENGINE` M2 (sea) and M3 (cooked assets); `REVIEW-RENDER` M2 |
| T2 typed keys | `REVIEW-ENGINE` M2 (sea); `REVIEW-RENDER` M3 |
| T6 test support and one GPU gate | `TEST-SUPPORT` M1 and M2 |
| T5 the engine and game boundary | `GAME-SPLIT` M1 to M5; H19 in `REVIEW-ENGINE` M3; M56 in `HOST` M4 |
| T4 one device recipe and host core | `HOST` M1 to M4 |
| T3 errors | `REVIEW-ENGINE` M4, `REVIEW-RENDER` M4, `HOST` M5; the games keep string errors until they are their own repository's concern |
| T12 cost | H16 and H18: `REVIEW-ENGINE` M2 and M3. M52, M67, M64: `REVIEW-ENGINE` M5. M40, M38: `REVIEW-RENDER` M5. M01: `HOST` M3 |
| T9 and T10 names and splits | `REPO-RULES` M3, one landing |
| T13 hygiene | `REPO-RULES` M2 (bulk files, H03) and M4 |
| Not scheduled | T7 (shader source), T8 (configuration by environment variable, beyond a rule that stops new reads in libraries) and T11 (mechanisms built several times) are not in the review's order of attack. M24, M71 and M58 wait for the performance phase the brief puts after the exit, and M31 for the governor's cadence controls. Each is an assumption in the owning plan |

### What starts at once, and what waits

Running when this was planned: `CULLING` M3, `APPLE-PRESENTATION` M3, `APPLE-TILE` M10, `CREATOR` M3, `EFFECT-GRAPH`
M3, `ATMOSPHERE` M9, `CLOUDS` M1 and `WATERS` M1. A new milestone never rewrites a file one of them holds; it waits
for that milestone. Later milestones of those modules are built on what has landed here.

| Starts | Milestones | Because |
|---|---|---|
| At once | `REPO-RULES` M1; `REVIEW-ENGINE` M1, then M2 and M3 | Scripts and docs; `thrum-core`, the iPhone app's request, `thrum-sea`, `thrum-asset-gpu` and `thrum-cook`, where no builder is at work |
| When `REPO-RULES` M1 is Done | `REPO-RULES` M2; `TEST-SUPPORT` M1; `REVIEW-ENGINE` M4 | They use the rule engine. None touches a held file |
| When `TEST-SUPPORT` M1 is Done | `GAME-SPLIT` M1 | The game gate's `boundary` runs `TEST-SUPPORT`'s `test-crate-edges` rule. It touches no held file |
| When `ATMOSPHERE` M9, `APPLE-TILE` M10, `APPLE-PRESENTATION` M3, `EFFECT-GRAPH` M3 and `CULLING` M3 are Done | `REVIEW-RENDER` M1 to M5 | Every one of them holds files in `thrum-render` |
| When those, less `ATMOSPHERE` M9, and `CREATOR` M3 are Done | `HOST` M1 to M5 | They hold `thrum-app`, the frame runtime and the V1 game's host and capture files |
| When `CULLING` M3, `EFFECT-GRAPH` M3 and `APPLE-PRESENTATION` M3 are Done | `GAME-SPLIT` M2 | They hold the visibility tests, the game's effect and the temporal witness |
| After `HOST` | `TEST-SUPPORT` M2 (`HOST` M1, and `GAME-SPLIT` M2); `GAME-SPLIT` M3 (`HOST` M4) | The test device is the recipe's, and the last `#[path]` includes of game source go with `GAME-SPLIT` M2; the game must hold no host code before it is cut |
| After every fix above | `REPO-RULES` M3, then M4 | A rename collides with everything, so it lands once, last |
| After `SHOWCASE` M8 | `GAME-SPLIT` M4 | The V1 exit closes before the game moves |
| After the last milestone whose check names `-p v1-game` | `GAME-SPLIT` M5 | A builder cannot edit a pinned check |

`thrum-sea` becomes `WATERS`' to extend at its M2, and `thrum-core` is where `SIM-2D` M1 works. `REVIEW-ENGINE` M1
and M2 are best landed before those start; if they have started, each builder integrates on what has landed.
`REPO-RULES` M3 lands as one change: a builder in the renderer at that moment rebases once.

<a id="seam-rulings-for-track-4"></a>

### Seam rulings for track 4

Each change below is built by the track 4 builder of the milestone named, inside state another module owns. No
item, test assertion, bound or reference of the owning module changes, no exact comparison is loosened, and every
registered baseline stays exact: each milestone that edits a covered path carries the verifier. Once built, each
belongs to its owner.

1. **The rules bind everyone from `REPO-RULES` M1.** They are ratchets: a file may not gain a violation and a new
   file starts at zero, so a running builder is refused only for a shape it adds. A rule's total may only go
   down: the accepted total is the lowest the checked-in baseline has held in Git history, so rewriting the
   baseline does not admit a new violation, and a moved file carries its allowance. A rule is registered when it
   reaches main, so a violation that reaches main before the rule does is in its baseline. Another module adds
   a rule through the same engine, with fixtures, in the milestone that first needs it.
2. **Bulk evidence** (`REPO-RULES` M2). A file over the cap leaves the tree and stays in Git history, listed in
   `.orchestrate/evidence/BULK.json`; the link check accepts a link to a listed path. Evidence folders stay their
   modules'. No review, record or design is edited, and a registered exact-baseline reference never leaves. From
   then on a builder keeps large captures in scratch or the local store, as
   [the design](REPO-RULES/DESIGN.md#mechanisms) says. The local store is one folder beside the main checkout,
   the same from every worktree, so what a builder puts there a reviewer's clean checkout restores.
3. **One publication model in core** (`REVIEW-ENGINE` M1; G2S). The engine's page is the only one. The V1 game's
   dead patch closure is deleted by the same builder.
4. **The sea's keys, records and upload** (`REVIEW-ENGINE` M2 and M5; `SEA`, and `WATERS` from its M2). The wave
   field interface and `SEA`'s bounds are unchanged. `WATERS`, `RIVERS` and `PUDDLES` extend the typed table, not
   positions.
5. **The cooked loader** (`REVIEW-ENGINE` M3; `CONTENT-LOOP`, `COOKED-DRAW`). The cooked format, every content
   address and `COOKED-DRAW`'s equivalence are unchanged. `examples/asset-loop` is removed. The edit-to-visible
   tripwire measures the engine's loader from then on, so its baseline is recorded again once, with the old and
   new figures side by side; `scripts/check_g5_tripwires.sh` keeps its name for `CONTENT-LOOP` M3.
6. **The renderer** (`REVIEW-RENDER` M1 to M5). The light group gains a second buffer, so every reader of the
   contribution seam's light layout follows in the same landing (`thrum-asset-gpu`, `thrum-sea`). In that landing
   the scene shader's two indirect-light functions also take a world position and a pixel position, with no
   value changed ([ruling 4 on work left until after V1](#held-open-for-after-v1)). Pass names stay
   exported constants and the frame timing feed gives `CREATOR`'s profiler what it gave. The timing-demand switch
   keeps its meaning. A change that cannot keep a retained image exact is not built: the builder ends `BLOCKED`.
7. **The device recipe** (`HOST` M1; `APPLE-TILE`'s capabilities and registry, the three shells). Recipe selection
   does not change. The browser keeps today's request as the recipe's named portable profile, so no Chrome
   baseline moves; compressed textures in Chrome would change default output and are `CONTENT-LOOP`'s to propose
   under [rule 1](#exact-baselines). On the Mac an uncaptured GPU error is recorded and survived. A test that
   needs a reduced device asks the recipe for a restriction, which only lowers limits and removes features; no
   shell passes one.
8. **The host core and the game contract** (`HOST` M2 to M5; `CREATOR`, the platform modules, core's clock).
   `SESSION` M7's driven world extends the one lifecycle policy. `GAME-UI` M6 still moves the iPhone app's UIKit
   lifecycle into `thrum-app`, onto this contract. `APPLE-PRESENTATION`'s seams 5 to 7, `WATERS`' ruling 3,
   `RENDER-2D`'s and `GAME-UI`'s shell rulings are built on whichever of these has landed.
9. **Test support** (`TEST-SUPPORT` M1 and M2; every module's tests). A test's device and gate change; what it
   asserts does not. No test that ran on the Mac becomes one that skips, and a skip elsewhere is a line on
   standard error that an ordinary run shows. `thrum-test-support` is a dev-dependency only. `thrum-fixtures`
   holds each shared fixture once, as plain data: any package's tests may use it, and the proving ground alone
   may take it as a normal dependency. `TEST-SUPPORT` owns both packages and that rule; `GAME-SPLIT` M2 creates
   `thrum-fixtures` with its first fixture, the mixed-mesh field.
10. **Names** (`REPO-RULES` M3). `synthetic` becomes `scene` and milestone codes leave file names, in one landing.
    A name that an open check, `./dev` or the baseline registry uses keeps working. No `DESIGN.md` is edited:
    approved designs keep saying `synthetic`, and `thrum-engineering`'s table maps old paths to new. The graph
    and timing import nothing from the scene renderer from then on; the frame runtime and the presenter still
    do, held from growing by a rule, until a module is planned to put an interface between them.
11. **The proving ground and the game** (`GAME-SPLIT` M2 to M5; `SHOWCASE`, `CULLING`'s visibility tests,
    `APPLE-PRESENTATION`'s temporal witness, `BASELINE-GUARD`'s registry). Until M5 the proving ground is the
    package `v1-game` at `examples/v1-game`, so every approved check that names it runs as written, and its
    `proving` feature is on by default. M5 changes paths in `.config/exact-baselines.json` and no reference.
    Everything the verifier and its drivers read (the scenes, the valley's sources, the presets, the routes, the
    pages) stays in Thrum as the proving ground's frozen content; the game takes its own copies, and the
    verifier passes with the games repository unavailable. The verifier is `BASELINE-GUARD`'s; `GAME-SPLIT` M4
    has it hand `THRUM_GAMES`, alone of the `THRUM_` variables, to the commands it starts, and changes nothing
    else in it.

### The proving ground, and where a game lives

An engine milestone proves itself in its own crate's tests or in the proving ground, never in a game. Where an
approved design says a selection or a test lives in `examples/v1-game` (`CLOUDS`, `PUDDLES`, `ATMOSPHERE` M7 and
M8, `CONTENT-LOOP` M2, `RENDER-2D`, `GAME-UI`), that is the proving ground, and those plans stand as written.
`PUDDLES` M6 draws rain over the populated valley: `SHOWCASE` M5 builds that scene before the game moves, and the
proving ground keeps its frozen copy.

A game lives in the games repository, `/Users/ryannel/Workspace/thrum-games`, and reaches the engine only through
packages under `crates/`, by sibling path. Its plan, done-when file, reviews and evidence stay here, because the
loop runs in one repository; its builder commits the game's code there and proves it through
[the game gate](GAME-SPLIT/DESIGN.md#mechanisms).

- **`SHOWCASE`** builds M5 to M8 in `examples/v1-game` as planned. `GAME-SPLIT` M4 then moves what it owns and
  shows the V1 exit from outside ([how](GAME-SPLIT/DESIGN.md#how-the-v1-gate-is-verified-once-the-game-lives-outside)).
- **`SWARM`** is built in the games repository from its first line, at `thrum-games/swarm/`, by the owner's
  decision of 14 November. Its M2 now waits for `GAME-SPLIT:M4`. Its design and checks still say `examples/swarm`,
  and its ruling 1 (a second external game in the repository guard) is overtaken: the guard has no game to admit.
  `SWARM` is re-planned onto the gate before its M2 starts; that is a planning task, with nothing for the owner
  to decide.

### Interfaces (track 4)

| Interface | Provider → consumers | Defined in |
|---|---|---|
| **Shape rules**: `shape_rules.py check`, the ratchet baseline, `--rule R --zero PATH` as a proof that a path is clean, and how a module adds a rule | `REPO-RULES:M1` → every module | [REPO-RULES](REPO-RULES/DESIGN.md#mechanisms) |
| **Bulk evidence**: the manifest, `evidence_store.py check`, `restore` and `put` | `REPO-RULES:M2` → every builder and reviewer | [REPO-RULES](REPO-RULES/DESIGN.md#mechanisms) |
| **GPU gate and test device**: a test asks for a GPU and runs, is skipped with a `thrum-gpu-skip:` line on standard error, or fails under `THRUM_REQUIRE_GPU=1` | `TEST-SUPPORT:M1` → every GPU test | [TEST-SUPPORT](TEST-SUPPORT/DESIGN.md#mechanisms) |
| **Shared fixtures**: `thrum-fixtures`, plain data defined once; `test_role`, `proving_ground` and the `test-crate-edges` rule | `TEST-SUPPORT:M1` (the rule), `GAME-SPLIT:M2` (the package and the field) → `CULLING`'s visibility tests, the proving ground, `REPO-RULES`' `library-env`, the game gate | [TEST-SUPPORT](TEST-SUPPORT/DESIGN.md#mechanisms) |
| **Device recipe**: a device, its capability record and its fault watch from one request, native or portable, optionally restricted | `HOST:M1` → shells, captures, benches, `TEST-SUPPORT` | C04, [HOST](HOST/DESIGN.md#mechanisms) |
| **World lifecycle policy**, **render once** and **await drawn** | `HOST:M2` and `M3` → shells, captures, tests; `SESSION:M7` | [HOST](HOST/DESIGN.md#mechanisms) |
| **Game contract**: what a game implements to run in the window, on the page and offscreen; and the render owner's **frame job and reply** | `HOST:M4` → games, kit scenes, the iPhone app; `GAME-UI:M6` adds the iPhone | C11, [HOST](HOST/DESIGN.md#mechanisms) |
| **Game gate**: a game in the games repository built and checked against the engine checkout under test, with both revisions recorded | `GAME-SPLIT:M1` → `SHOWCASE` after the move, `SWARM` | [GAME-SPLIT](GAME-SPLIT/DESIGN.md#mechanisms) |

<a id="held-open-for-after-v1"></a>

## Held open for after V1: cluster level of detail, dynamic GI and virtual shadow maps

Added 18 November 2026. The owner, 17 November 2026, as the lead's task reports it: Nanite-class geometry,
Lumen-class global illumination and virtual shadow maps are left until after V1, on condition that V1 lays no
foundation that forces rework. [The decision](../design/decisions/POST-V1-FOUNDATIONS.md) judges the three
changes a read-only check proposed, with the research, and says what was rejected and why. The rulings below are
what it asks of modules. None adds a milestone, none is on the road to the exit, and none moves an exact
baseline. C05, C07 and C08 each gained the lines these rulings rest on.

1. **Mesh products and rows** (nothing to build; `CONTENT-LOOP`, read by `CULLING`, `COOKED-DRAW`,
   `MESH-SHADING` and `RAY-TRACING`). Every cooked mesh product carries its meshlet sections. A mesh row keeps
   its product's vertex numbering and index order and is held with the product's hash, so the product's meshlet
   table and bounds describe the row. Cluster facts arrive by a new container format and cooker version.
   [CONTENT-LOOP's design](CONTENT-LOOP/DESIGN.md#what-later-modules-will-ask) holds the three promises, and any
   milestone that converts, uploads or draws a cooked mesh keeps them. A version field or reserved fields in the
   meshlet bounds record were rejected.
2. **The forward shader's two steps** (nothing to build; the forward pass, as
   [APPLE-TILE's design](APPLE-TILE/DESIGN.md#mechanisms) describes it). `shade` keeps a surface step and a light
   step with only the surface's values, the world position, the view direction and the pixel position between
   them (C08). It binds every milestone that edits `shade` or adds a term to it, among them `RAY-TRACING` M3 to
   M5, `PUDDLES`' wet ground, `WATERS` M16's Sun-path factor and `REVIEW-RENDER` M2. A light term takes those
   values as arguments; a surface effect changes the surface before any light is evaluated. Nobody splits the
   shader text for this now.
3. **Guide buffers** (nothing to build; C07). Normals and roughness, when a later module builds them, are the
   output of a thin depth prepass declared in the one graph, selected at preparation, and absent from a scene
   that does not ask. They are never a fourth forward colour target: `SceneSurface`'s three colour formats stay
   three through `APPLE-PRESENTATION` M4 to M12 and for every forward contributor. The buffers are render-extent
   attachments under `APPLE-PRESENTATION`'s extent and generation rules. The prepass's fragment entries write
   [`CULLING` M3's coverage output](#ruling-on-culling-m3), and `CULLING`'s occlusion prepass stays private: it
   draws only last frame's visible set, so it cannot be the producer. A contribution joins through one optional
   method on the contribution seam whose default draws nothing.
4. **The indirect-light query** (`REVIEW-RENDER` M1, item M1.5; `synthetic.wgsl`, its two composed variants,
   `thrum-asset-gpu`'s cooked volume and the test shaders that stub the functions). `ambient_irradiance` and
   `indirect_radiance` take the lit point's world position and pixel position, in the landing that splits the
   light buffer. Nothing reads them, so every byte is as it was, and M1's verifier item holds it. The cooked
   volume is a medium and not a caller of the query: it reads the ambient light as a source, by direction, through
   `ambient_source`, which that landing splits out of `ambient_irradiance`, so its one read and its pixels stay.
   `RAY-TRACING` M5 gives the pixel its first reader under [track 5's ruling 6](#seam-rulings-for-track-5), on
   the signature it finds. `ATMOSPHERE`'s lent `environment_diffuse` and `environment_reflection` do not change:
   they are the sky as a source, asked by direction, and `WATERS`, `CLOUDS` and `SEA` keep reading them as they
   do.
5. **Metal's one draw for each meshlet** (a note for track 5). The code review's M71, which `REVIEW-ENGINE`
   parked, is the finding `MESH-SHADING` closes for cooked placements, and its natural entry point.
   `REVIEW-ENGINE`'s plan now says so. Ruling 1 is what lets entities follow without a new cook, if
   `MESH-SHADING` M3's record asks for them.

| Module | What changed on 18 November 2026 |
|---|---|
| `CONTENT-LOOP` | Design: three promises on mesh products and rows. No item |
| `REVIEW-RENDER` | Design: the indirect-light query, in mechanism 1. M1 gains item M1.5 |
| `APPLE-TILE` | Design: forward shading held open for a prepass. No item |
| `APPLE-PRESENTATION`, `CULLING` | A log line each. No design line and no item |
| `REVIEW-ENGINE` | The M71 assumption in its plan points at `MESH-SHADING`. Its design's parked note follows at its next re-plan |
| `MESH-SHADING`, `RAY-TRACING`, `ATMOSPHERE`, `WATERS`, `CLOUDS`, `SEA` | Nothing |

## Track 5: ray tracing and mesh shaders on the Mac

Added 17 November 2026, from the owner's words that day:

> Just do ray tracing and mesh shaders on mac, let's leave PC for now.

The lead's task asked for hardware ray tracing and mesh shaders on the Apple tile recipe, verified on this M3
Max under [the hardware-native decision](../design/decisions/HARDWARE-NATIVE-ENGINE.md), with the portable path
kept as fallback and reference, each path promoted only by a measured win, V1 not delayed, and the desktop-high
recipe out of scope. Two modules build it: [RAY-TRACING](RAY-TRACING/DESIGN.md) and
[MESH-SHADING](MESH-SHADING/DESIGN.md). Before either was designed,
[a probe](../../.orchestrate/evidence/RAY-TRACING/planning-probe/README.md) showed that the locked wgpu reaches both features on
this Mac through its public API, so neither needs raw Metal.

### What the two features serve

Decided from what the renderer does today and what shipped renderers spend these features on. Each module's
design gives the research.

| Problem in Thrum | Served | Why |
|---|---|---|
| The Sun's shadow on surfaces: two 1024² cascades, then a height map, with bias and a low-Sun correction | `RAY-TRACING`: a ray against the scene's own triangles, hard first, then soft | A ray is right at any distance and at contact. Visibility is the cheapest use of a ray |
| Lamps cast no shadow | `RAY-TRACING`: one ray for each lamp a surface faces | Raster needs a shadow map for each lamp. A ray needs none |
| Nothing darkens where objects meet | `RAY-TRACING`: ambient occlusion by ray | The scene's own geometry, including what is off screen |
| A cooked mesh costs one Metal draw for each meshlet, rejected ones included, and its shadows draw it whole | `MESH-SHADING`: the object stage culls and the mesh stage draws, in one draw for each placement | wgpu has no GPU-counted multi-draw on Metal, and the stages are the public way to draw a GPU-chosen set |
| Reflections of scenery, on water and on glossy things | Not planned | They shade what a ray hits. The consumer is water's mirror lookup, `WATERS` M14 on `SCENE-VIEWS`, which is not built; [owner decision 23](#owner-decisions) stands until a plan replaces it |
| Light bounced from the ground and walls | Not planned | A radiance cache with state of its own, on hit shading. A later module on `RAY-TRACING`'s structure |
| The exit scene's 10,000 entities drawn a meshlet at a time | Not planned yet | It changes `CULLING`'s route, whose every mode is byte-equal to an oracle. `MESH-SHADING` M3's record decides whether it is planned |

### Why two new modules

A module is cut round state. Ray tracing brings state no module owns: the structures and the traced
visibility history. The mesh stages are a third way to draw a cooked mesh. `COOKED-DRAW` owns the choice between
the two there are, and `CONTENT-LOOP` owns the meshes' residency; both are on the road to the exit and their
designs are approved word for word. Work that is off that road is not added to them. `MESH-SHADING` builds
inside their files under the rulings below and hands what it builds to them, as `SCENE-VIEWS` and `RIVERS` do.

The two share no state and one function, the launch list. They are two modules so that two builders can work
at once, and so that one path failing to measure a win does not hold the other.

### Modules and what each owns (track 5)

| Module | Owns (state and code) | Design |
|---|---|---|
| `RAY-TRACING` | The mesh structures and the scene structure; ray instances and their masks; the traced method of the scene's Sun visibility; lamp shadows; the traced visibility pass with its history, soft Sun shadows and ambient occlusion; this Mac's opt-in record for ray paths (a `raytrace` module in `thrum-render`, and [ruled changes](#seam-rulings-for-track-5) elsewhere) | [Design](RAY-TRACING/DESIGN.md) |
| `MESH-SHADING` | The object and mesh stage shaders and pipelines of the cooked draw; a mesh's meshlet storage on the GPU; this Mac's opt-in record for the mesh stages (files of its own in `thrum-asset-gpu`, and [ruled changes](#seam-rulings-for-track-5) elsewhere) | [Design](MESH-SHADING/DESIGN.md) |

### Off the critical path

No piece of either module earns a place on the road to the exit. The exit asks for cascaded shadows and GPU
culling, and both are built.

- No V1 module's design, plan or done-when item changes, and nothing is added to any `SHOWCASE` milestone's
  `depends_on`.
- A launch that names no hardware feature makes the device it makes today, and a scene that asks for nothing
  draws the bytes it draws today. Each milestone proves it against [the exact baselines](#exact-baselines).
- Neither module changes the Mac window's default, before the exit or after it, and the verifier's runs name
  no hardware feature, so no registered baseline moves. Each module's last milestone waits for `SHOWCASE` M8
  so that its record describes the valley as the exit leaves it.
- A builder takes a milestone here only when no V1 milestone can start. Order among the other work that is
  off the exit is the lead's.
- Milestones running in the same crates when this was planned: `SCENE-VIEWS` M1 in `thrum-render` and
  `REVIEW-ENGINE` M4 in `thrum-asset-gpu`. Still to come there: `APPLE-PRESENTATION` M4 to M12, `ATMOSPHERE`
  M6 to M8, `REVIEW-RENDER`, `HOST` and `CONTENT-LOOP` M3. A track 5 milestone adds to a file one of them
  holds and never rewrites it, and where one of them has changed a shape (typed errors, one struct for each
  shader record, typed pass names) it uses the shape it finds.

`RAY-TRACING` M1 and `MESH-SHADING` M1 can start at once, in different crates. `RAY-TRACING` M3 waits for TAAU
(`APPLE-PRESENTATION` M3) and for the populated valley (`SHOWCASE` M5), which the proving ground keeps.

<a id="seam-rulings-for-track-5"></a>

### Seam rulings for track 5

Each is built by the milestone named, inside state another module owns, with the behaviour the builder's
design defines. No item of the owning module changes and no exact comparison is loosened. Once built, each
belongs to its owner: later work there keeps its tests passing.

1. **Hardware features at launch** (`RAY-TRACING` M1 and `MESH-SHADING` M1; `thrum-render`'s
   `capabilities.rs`, the device requests in `thrum-app`, the proving ground's hosts and
   `thrum-test-support`). One function of the registry makes the device for every host, from the host's own
   request, the adapter and a launch's list of names (`ray-query`, `mesh-stages`), each given as required or
   preferred. It adds the features, limits and experimental token the list needs. A required name is met or
   refused by name. A preferred name is left out where the adapter does not offer it, and where the device
   request with it fails the function asks once more without the preferred names; the registry records what
   was enabled and why any name was not. With no list the request is today's. Whichever milestone lands first
   builds the function and the other adds its name. `HOST` M1 gathers the device requests into its recipe when
   it is built, and the recipe carries the list. No host adds `ray-query` or `mesh-stages` unasked, and the
   registry holds no list of machines for either.
2. **Mesh pipelines in the creation ledger** (`MESH-SHADING` M1; `runtime/creation.rs`). The creation authority
   gains an entry for mesh pipelines beside `render` and `compute`, counted as they are.
3. **A mesh structure for each arena row** (`RAY-TRACING` M1; `synthetic/mesh_arena.rs`, `mesh_route.rs`). In a
   scene whose preparation resolved [ray demand](RAY-TRACING/DESIGN.md#the-structure-m1-m2), the arena's
   buffers carry the usage a structure build reads, and a row's mesh structure is built with the row's write,
   switched with the row and retired with it. In any other scene the arena is today's, on any device.
   `CULLING`'s route and `CONTENT-LOOP`'s row replacement behave as before.
4. **Cooked residency** (`RAY-TRACING` M2 and `MESH-SHADING` M1 and M2; `thrum-asset-gpu`'s `load.rs`, `lib.rs`,
   `draw.rs`, `placement.rs` and `material.rs`). In a contribution whose scene's surface carries ray demand, a
   resident cooked mesh's buffers carry the usages a structure build reads and its mesh structure is part of
   its residency; under any other surface it uploads nothing more and changes no usage, on any device.
   `CookedDrawPath` gains `MeshStages`. In a contribution that resolved to the mesh stages, each mesh the
   device's limits can bind and dispatch also carries the usages the stages read and its meshlet storage; any
   other mesh draws by the path `COOKED-DRAW` selects, and a contribution on any other path uploads nothing
   more. A material reload prepares, draws and installs its hook for the indexed and the stage pipeline as one
   candidate, and is otherwise unchanged. A content publication keeps its three kinds of reader and its eight
   rules: a mesh's structure and meshlet storage are prepared, shown and retired with the mesh. C05 is
   unchanged. `COOKED-DRAW`'s selection between the direct and indirect paths is unchanged.
5. **Ray instances on the contribution seam** (`RAY-TRACING` M2; `contribution.rs`). `SceneContribution` gains
   one optional method through which a contribution offers, each frame, every opaque placement it holds in
   the scene, whether or not a view or a pass draws it. The default offers none. `thrum-render` gains no dependency on `thrum-asset-gpu`.
6. **The scene's shader and light group** (`RAY-TRACING` M3 to M5; `synthetic.wgsl`,
   `synthetic_environment.wgsl`, `synthetic_plain.wgsl`, the light group's layout, and which shadow passes
   `synthetic/shadow.rs` records). In a scene that asked, `sun_visibility` is the traced operation, lamps
   trace in `local_radiance`, the ambient term takes the traced occlusion, the light group holds the scene
   structure and the visibility history, and the cascades are drawn only while something reads them. In a
   scene that did not ask, the shader text and every byte are today's. `ATMOSPHERE`'s direct-Sun path, its
   products and the Sun-path optical depth are unchanged: the traced value takes the place of the map lookup
   and nothing else.
7. **The frame graph, history and timing** (`RAY-TRACING` M1, M3 and M4; `graph.rs`, the frame host,
   `timing.rs`). The structure build and the traced visibility pass are passes of the one graph, with their
   reads and writes declared. The visibility history is accepted, reset and committed by
   `APPLE-PRESENTATION`'s history rules. The frame's timing record gains the timed-pass span: from the begin
   of the first timed pass the frame encodes to the end of the pass that consumes its output, beside the
   pass times and the scene sum it has, which keep their meaning. The time between those passes lies inside
   it. It is a pass interval and not the whole frame's GPU time: transfers a frame queues with
   `Queue::write_buffer` run before its first pass and lie outside the span, and every record that reads the
   span names it so and says which transfers those are. `RAY-TRACING` M3 or `MESH-SHADING` M2 adds the span,
   whichever lands first, and the other uses it as found.
8. **Scene layouts for the object and mesh stages** (`MESH-SHADING` M1: the view and light group layouts and
   `SceneSurface`; M2: the caster view layout of `synthetic/shadow.rs`, which each Sun map pass binds at
   group 0 and lends as `SceneShadowSurface::view_layout`). On a generation that enabled the stages, those
   layouts are visible to the object and mesh stages as they are to the vertex stage. Nothing else about
   them changes, and on any other generation they are unchanged.
9. **Secondary views** (no build). A view of `SCENE-VIEWS` shades with the map method and its own cascade and
   draws cooked meshes by the path that module resolves, whatever the main view resolved. Rays or the stages
   in a view are proposals for that module's next plan.
10. **Resolved selections** (`RAY-TRACING` M3, `MESH-SHADING` M1). The report that names the culling mode also
    names the hardware features the device enabled, the Sun shadow method, the penumbra, lamp shadows, ambient
    occlusion and the cooked draw path, each as resolved, with the reason where it did not resolve as asked.
    `SHOWCASE`'s exit report prints what it reads; no item of `SHOWCASE` changes.
11. **Pipeline preparation** (no build). A scene with a ray or mesh-stage request creates its pipelines
    through the creation authority before it is ready, from WGSL. `APPLE-PRESENTATION` M10's shaders compiled
    ahead of time and M12's pipeline archive cover a launch with no hardware request, which is what their
    strict runs take. Bringing track 5's shaders into the library cook is a proposal for that module once M10
    is Done.
12. **A preferred recipe** (`MESH-SHADING` M3 or `RAY-TRACING` M6, whichever lands first, and the other
    uses it as found; `capabilities.rs`, `capabilities/recipe.rs` and the proving ground's Mac host). The registry's recipe resolution gains one
    input beside the named recipe: a recipe the host prefers. It is read only when no recipe is named. Where
    the facts that admit a named Apple tile admit it, the generation resolves to Apple tile. Where they do
    not, it resolves to portable and the registry records why. A preference is never an error. A named
    recipe, `portable` or `apple-tile`, resolves exactly as today, and so does a launch that names none and
    states no preference, so no launch that exists today moves and no item of `APPLE-TILE` changes.
    `MESH-SHADING`'s launch selection states a preference for Apple tile, and only on a generation that
    enabled `mesh-stages`. `RAY-TRACING`'s states the same preference, and only on a generation that
    enabled `ray-query`.
13. **Candidate preparation with the valley's cloud sky** (`RAY-TRACING` M5, in `thrum-clouds` and the
    proving ground). The production list includes cooked geometry and `Shade`. `Shade` stages for traced and
    Maps surfaces through the [M3 preparation boundary](RAY-TRACING/DESIGN.md#scene-preparation-and-maps-fallback-m3)
    and keeps its cloud lighting and Sun-path product. `CLOUDS` M7 provides the overcast preset for inspection.

### Interfaces (track 5)

| Interface | Provider → consumers | Defined in |
|---|---|---|
| **Hardware features at launch**: a launch names the hardware features it wants, each as required or preferred, and one function makes the device. A required name is enabled or the launch is refused by name. A preferred name never fails a launch: it is left out where the adapter does not offer it or the device request with it fails. The registry records what was enabled and why any name was not. Default: none. No host adds either name unasked | `RAY-TRACING:M1`, `MESH-SHADING:M1` → hosts, tests, captures | C04, [RAY-TRACING](RAY-TRACING/DESIGN.md#hardware-features-at-launch) |
| **Ray instances**: a contribution offers each frame, for each opaque thing it holds in the scene, drawn or not, a mesh structure, a transform and a mask (casts shadows, occludes ambient light); the scene structure is rebuilt from them each frame | `RAY-TRACING:M2` → `CONTENT-LOOP`'s cooked contribution; any later contribution with opaque geometry | [RAY-TRACING](RAY-TRACING/DESIGN.md#the-structure-m1-m2) |
| **Traced visibility**: a scene's preparation takes the Sun's shadow method (`Maps` or `Traced` with penumbra on or off), lamp shadows and ambient occlusion with a radius. Admission before allocation requires ray queries, the Apple tile recipe and candidate preparation support from every contribution. If admission or traced preparation fails, the scene draws Maps with lamp shadows and occlusion off and reports why. Ordinary Maps callers keep the existing rebuild hooks. Ray effects are off by default | `RAY-TRACING:M3` (Sun, lamps), `M4` (penumbra), `M5` (ambient occlusion) → games, the proving ground | C04, C08, [Selection](RAY-TRACING/DESIGN.md#selection-fallback-and-the-opt-in), [contribution admission](RAY-TRACING/DESIGN.md#scene-preparation-and-maps-fallback-m3) |
| **Cooked draw path**: `Direct`, `Indirect` or `MeshStages`, resolved from the registry for the scene and for each mesh, and reported with the reason where it did not resolve as asked. `Auto` is `COOKED-DRAW`'s selection on every device and never the stages. `MeshStages` is met only on a device that enabled the stages and on the Apple tile recipe. Anywhere else, which includes the portable recipe whether named or resolved, and where a stage pipeline fails to build, the scene draws what `Auto` draws. It is not refused | `MESH-SHADING:M1`, on `COOKED-DRAW`'s selection → games, the proving ground | C04, [MESH-SHADING](MESH-SHADING/DESIGN.md#selection-fallback-and-the-opt-in) |


## Track 6: the cleanup of 20 November

Added 20 November 2026 from the owner's request that day: "We want to do a code cleanup and consolidation for
Thrum." Its purpose, in the lead's words for this plan: make Thrum cheaper to change, with "a shorter merge
gate, no tools, records or code that nothing uses, and no proof heavier than the fault it guards", and without
changing "what the engine draws, simulates or sends". It starts from the cleanup review of that day, kept in the
main checkout at `/Users/ryannel/Workspace/thrum/tmp/cleanup-2026-11-20/`: 26 reviewers read the repository at main
`99f8033a0` and 57 verifiers tried to refute what they found. Where a verifier corrected a finding, the plan
follows the verifier. Three new modules, rows 34 to 36, and three milestones added to `TEST-SUPPORT`. None gates
the V1 exit. A second read of the plan, `HARDENING.md` in the same folder, was applied on 20 November with the
owner's decision that the merge tier is targeted.

### Why new modules, and three of them

As in [track 4](#why-new-modules), the work sits in code that other modules own, and seven of those modules have
builders at work, so it is cut into new modules that build inside the owners' code. Ownership does not move.
Milestones of one module run in order, so the work is three modules, to let three Claude builders (Opus 5.5 at
medium effort) work at once on files that do not overlap. They are ordered by payoff: gate time first, because
every merge pays it.

| Module | Takes from the review | Owns | Builds inside |
|---|---|---|---|
| `GATE-TIME` | Section 1, and the reference tests the 17 November bar may retire | The tiers as built ([design](GATE-TIME/DESIGN.md)), under [the test tiers](../../ORCHESTRATE.md#test-tiers): the main-tier list with `./dev long-tests` and the main-branch check `long-tests`, the merge selection with `./dev check-all` and the main-branch check `workspace-tests`, and how the check runs tests: side by side through cargo-nextest, with its configuration `.config/nextest.toml` and the limit on one test; the two rules for how a merge-tier test is written, one test binary a package, and the record of where a builder's session goes, by [ruling 13](#seam-rulings-for-track-6); the guard rules `limit-exempt`, `main-tier` and `test-binary`, by ruling 14 | Tests of `WATERS`, `SHOWCASE`, `CULLING`, `CLOUDS`, `ATMOSPHERE`, `SCENE-VIEWS` and `CONTENT-LOOP`; `REPO-RULES`' check command and repository guards; one wait in `APPLE-TILE`'s frame host, by [ruling 12](#seam-rulings-for-track-6); from M11 the test manifest and test files of every package |
| `DEAD-TOOLS` | Section 2, and the code half of section 6: fixtures and record writers that use `docs/evidence` | The rules `evidence-path`, `script-caller`, `bench-runner` and `shader-text-edit`, where the evidence manifest is kept, and where a provenance record is tracked ([design](DEAD-TOOLS/DESIGN.md)) | `scripts/`, `tools/xtask`, benches of `thrum-render` and the game, `docs/evidence`, `docs/provenance`, `REPO-RULES`' evidence store |
| `DEAD-CODE` | Sections 3 and 4, and the water settings that are parsed twice | `thrum_render::binding`, and the five defaults of `SceneContribution` ([design](DEAD-CODE/DESIGN.md)) | `thrum-core`, `thrum-sea`, `thrum-water`, `thrum-render`, `thrum-rain-model`, the water scene, the game's host |
| `TEST-SUPPORT` M5, M6, M3 and M4 | Section 5, the test kit | The kit in `thrum-test-support`, and the rules `test-readback` and `test-deadline` ([design](TEST-SUPPORT/DESIGN.md#the-kit)); from M6 the kit's small target, and how a test is written, in the engineering skill's block and pattern entries, by ruling 14 | Every crate's tests; the engineering skill |

### What can run side by side

Each design lists the files each milestone touches, under "Files each milestone touches", and the lead confirms
them against the builders at work before it starts one. Milestones of one module run in the order its plan
gives, which is the order of each cell below. By the owner's word of 20 November, "Cleanup first", no feature
milestone starts until this track has landed, so the only other builders are the ones finishing the milestones
this track waits for.

**`GATE-TIME` M1 merges first.** The other wave 1 milestones are built beside it and queue their merges behind
it, because each merge before M1 pays about 50 minutes of the merge lock and each one after pays about half that. After
M1 merges the lead fast-forwards the main checkout, where the script reads the settings, or the new main-branch
check never starts. The same holds after `GATE-TIME` M7.

| Wave | `GATE-TIME` | `DEAD-TOOLS` | `DEAD-CODE` | `TEST-SUPPORT` | Files they share |
|---|---|---|---|---|---|
| 1, at once | M1: six tests marked, `./dev long-tests`, the main-tier list, one settings line. M6: tests behind a Cargo feature compiled at every merge and run on main. M8: tests run side by side, the test build a measurement picks, step times and the limit on one test. M7: the merge selection, `./dev check-all`, one settings line | M5: five new files under `docs/provenance`. M6: four frozen rules. M1: `scripts/`, the script-suite step, three designs, ledger rows | M1, once `GATE-TIME` M1 is Done: `thrum-core`'s simulation, `thrum-sea`, two files of `thrum-water` | M5, once M2 is Done: the kit's functions and two frozen rules | `tools/xtask/src/main.rs`: `GATE-TIME` M1, M6, M8 and M7 and `DEAD-TOOLS` M1 edit different arms of it and one help text, which M1 puts on a line for each command. The rule files `scripts/shape_matchers.py`, `shape_rules.py` and `test_shape_rules.py`: `DEAD-TOOLS` M6 and `TEST-SUPPORT` M5 each add rules. `scripts/repo_check.py`: `GATE-TIME` M1 and M6. Whichever lands second rebases |
| 2 | M2, after `TEST-SUPPORT` M2: test files of `thrum-water`, `thrum-clouds`, the game and the atmosphere. M10: a record of where a builder's session goes, with no tracked file. M12, after `TEST-SUPPORT` M6: three rules in the rule files, and two lines of the check's report. M9: the test files that hold a test over ten seconds, in the game and the river, water-model and sea crates. M13: the check command's second run of a stopped test, and the limit in `.config/nextest.toml`. M11: the test manifest and test files of every package, folded into one test binary a package | M2: benches and the `baseline-inventory` command. M3: `docs/evidence`, the evidence store, five documents | M2, after `GATE-TIME` M6: `contribution.rs` and every implementation. M3: `thrum-core`'s runtime, three small deletions | M6: the kit's small target, and the block and entries on writing a test in the engineering skill. M3, after `GATE-TIME` M2 and M6 and `DEAD-CODE` M2: test files of every crate and kit scene | `GATE-TIME` M2 and `DEAD-TOOLS` M2 both edit `examples/v1-game/Cargo.toml`, in different tables. `DEAD-TOOLS` M3 edits a test module that `GATE-TIME` M2's `atmosphere/gpu/tests.rs` declares. `TEST-SUPPORT` M3 is the widest set: its design says what it runs beside. `GATE-TIME` M11 edits the manifests and test files of every package, so the lead does not run it beside `TEST-SUPPORT` M3 or M4 where it can choose, and whichever lands second rebases. `GATE-TIME` M12 adds rules to the rule files where `TEST-SUPPORT` M3 edits one rule's exemptions, and both it and `TEST-SUPPORT` M6 edit the Tests section of the engineering patterns, M6 first |
| 3, each after another module's milestone | M3, after `WATERS` M7: the foam test and the water scene's other slow tests. M4, after `REVIEW-RENDER` M2. M15: the bounded Cargo cache for merge verification. M5, after M15 and `DEAD-TOOLS` M2: the post-rollout record and contributor guide | M4, after `TEST-SUPPORT` M2: the cost scripts | M4, after `WATERS` M7. M5, after `HOST` M3. M6, after `REVIEW-RENDER` M3 and `GATE-TIME` M6 | M4, after `REVIEW-RENDER` M2 | `GATE-TIME` M8 and `DEAD-CODE` M5 both edit the root `Cargo.toml`, in different tables. `GATE-TIME` M8 edits the check's test step and `DEAD-TOOLS` M2 another arm of the same file. `GATE-TIME` M9 and `TEST-SUPPORT` M3 both edit test files under `crates/`: the lead does not run them side by side where it can choose |

Three files are everyone's: `.config/shape-baseline.json`, which each landing that lowers a rule's count
rewrites, `docs/design/maturity.json` and `Cargo.lock`. A builder changes them when its landing moves them,
whatever its row says, and one that meets a conflict there rebases and makes its edit again. `REPO-RULES` M3, the
one rename, waits for all of track 6.

Waves 2 and 3 wait for `TEST-SUPPORT` M2, `WATERS` M7, `REVIEW-RENDER` M2 and M3 and `HOST` M3. The track cannot
land while any of them is held.

### Seam rulings for track 6

1. **A Done item's check is its record.** [The project settings](../../ORCHESTRATE.md#targets) say so from this
   plan on. Track 6 moves, merges and deletes tests and scripts that only Done items name, and edits no Done
   item. An owning module's design that names a proof this track moved or retired is corrected at that
   module's next re-plan, as after [the ruling of 15 November](#proofs-run-once-on-main).
2. **Long tests** (`GATE-TIME` M1, M9 and M3; `WATERS`, `SHOWCASE`, `CULLING`, and from M9 any module with a
   test over about ten seconds that cannot be made fast). By the owner's decisions of 20
   November, kept as [the test tiers](../../ORCHESTRATE.md#test-tiers), no test over about a minute runs in a
   merge's check, and no short run of one stays there. From `GATE-TIME` M9 the bound is about ten seconds: a
   slower test is made fast with every assertion kept, or becomes a long test with the fault it leaves unguarded
   before a merge written beside it in the main-tier list. The ten-minute gale, the moored foam's ten minutes, the
   exit scene's stepped hour, the field flythroughs and the shore-blend solve still run, whole and with the
   assertions they had, in the main-branch check `long-tests` after merges: in release, or in the debug build for
   a test that fails in release at the starting commit, whose owner the log names. `WATERS`', `SHOWCASE`'s
   and `CULLING`'s designs and Done items stand, because the proof they name still runs. A failure on main is
   fixed by the module whose commit caused it, as for exact images. The builder of `GATE-TIME` M1 writes the one
   settings line, in its design's words.
3. **References retired under the 17 November bar** (`GATE-TIME` M2 and M4; `ATMOSPHERE`, `SCENE-VIEWS`,
   `CONTENT-LOOP`). The atmosphere's guards P3, P4 and P11 stay. Nothing an open item of `ATMOSPHERE` M6 reads
   changes: its referee and `products/tests/` are untouched. Each retirement is the owner's call and an
   assumption in [GATE-TIME's plan](GATE-TIME/PLAN.md).
4. **The evidence manifest** (`DEAD-TOOLS` M3; `REPO-RULES` M2's bulk evidence interface). The manifest moves
   under `.config/` and the two become one. `evidence_store.py check` and `restore` keep their names and
   results, and the link check still accepts a link to a listed path.
5. **`SceneContribution`** (`DEAD-CODE` M2; every contribution). Five hooks gain defaults equal to today's empty
   bodies; `passes`, `rebuild` and `encode_upload` stay required. No implementation's behaviour changes, and a
   builder in another module may delete a stub of its own or leave it.
6. **`thrum_render::binding`** (`DEAD-CODE` M6). New public API in `thrum-render`, accepted because five crates
   and the examples already write these entries by hand. Every layout stays what it was.
7. **Exact baselines.** No track 6 milestone may move a retained image. Each that touches a covered path carries
   the verifier, and none edits a file the registry pins. A script a pinned file imports, directly or through
   another script, is not pinned: a milestone may edit it, and it keeps its name and everything the pinned files
   use from it. `DEAD-TOOLS` M6, M3 and M4 edit four such scripts, as
   [their design](DEAD-TOOLS/DESIGN.md#mechanisms) lists, `TEST-SUPPORT` M5 edits two of the same four, and
   `GATE-TIME` M1 and M6 edit `repo_check.py`.
8. **Provenance records** (`DEAD-TOOLS` M5; `SEA`, `WATERS`, `CLOUDS`, `PUDDLES`, `RIVERS`). By the owner's
   decision of 20 November, a module's record of what it copied, from where and under what licence is tracked at
   `docs/provenance/<MODULE>.md`. It is not evidence, because a person must read it to publish the code. Each
   module still owns what its record says. `CLOUDS` M6, the first milestone that copies a library, waits for
   `DEAD-TOOLS` M5. The five designs still name the working-folder file; [the settings](../../ORCHESTRATE.md#starting-from-a-clean-clone)
   say that means the tracked record, and each design is corrected at its module's next re-plan.

9. **Tests behind a Cargo feature** (`GATE-TIME` M6; the modules whose tests they are, among them
   `RAY-TRACING`, `MESH-SHADING`, `CONTENT-LOOP` and `SCENE-VIEWS`). Every merge compiles them, and the main-branch check `long-tests` runs
   them, from the main-tier list. The list covers a package's feature tests whole or by test binary, and
   `thrum-render` is covered whole, so a new gated test file there runs with no edit to the list. Anywhere else
   the docs check fails a test file whose `cfg` names a feature, or a test target that requires one, until the
   list names it. No test is edited to join the list: one that fails at the commit M6 starts
   from is listed as not run, with its failure, and its owning module fixes it. Open items that run these tests
   by name keep their text. `DEAD-CODE` M2 and M6 and `TEST-SUPPORT` M3 edit files that hold them, wait for M6
   and carry the main-branch item.
10. **The merge selection** (`GATE-TIME` M7; every module, and `REPO-RULES`, whose check command it is). By the
    owner's decision of 20 November, `./dev check` runs the tests of the packages a change can reach, by Cargo's
    graph, and the main-branch check `workspace-tests` runs the whole workspace. A shared input, or a path the
    selection has no rule for, selects everything. A test that reads another package's files when it runs is
    reached by a declared edge, kept with the path rules: `crates/thrum-asset-gpu/src/` selects `thrum-render`,
    for the renderer's guard on where cooked pipelines are created, and a module that adds such a reader
    declares its edge in the same landing. The risk accepted is in
    [the test tiers](../../ORCHESTRATE.md#test-tiers): a missed edge sits on main until the next whole run, and
    until it is fixed or reverted a later merge can be refused for it, and so can any milestone's Ready for
    Review, which runs the whole workspace on main's tip. A module that adds a folder outside a package gets the
    whole workspace on every change there until it adds a path rule. No merge waits for the whole run.
11. **Frozen rules** (`DEAD-TOOLS` M6, `TEST-SUPPORT` M5; `REPO-RULES`' rule engine, and every builder). Six
    rules enter the shape check at the count of the commit they land on, before the cleanups that lower them:
    `evidence-path`, `script-caller`, `bench-runner`, `shader-text-edit`, `test-readback` and `test-deadline`.
    They go in through the engine with fixtures, as the track 4 rules did, and the engine itself does not
    change. From the day one lands, a landing in any module that adds one more of its shape is refused, with a
    message that says what to write instead. `script-caller` refuses a script that nothing names. A done-when
    item's check names one, whether its milestone is open or Done, so a proof tool is tracked under `scripts/`
    at the path its item gives and lands with no extra step: the nine scripts that open items of `SWARM`,
    `GAME-UI`, `RENDER-2D`, `ATMOSPHERE`, `REPO-RULES` and `GAME-SPLIT` name and nobody has written yet land
    that way. Records under `docs/evidence` name nothing. A change that removes the last thing naming a script
    or a bench deletes it, or names it elsewhere, in the same landing; that binds a planner who drops the last
    check naming a script that exists, because the next merge in any module would fail on it. `shader-text-edit` holds a shape whose replacement is
    [not planned yet](#not-planned-in-track-6); until that design exists a new variant is made the way the
    engineering patterns already ask, through a hook with a stub.

12. **How the check runs tests** (`GATE-TIME` M8; every module that writes a test, and `REPO-RULES`, whose check
    command it is). By the owner's words of 20 November, "A 20min merge check still sounds crazy. Tests are meant
    to be fast", a merge's check is planned to about five minutes for a typical landing. From M8 `./dev check`
    runs tests through cargo-nextest, each test in a process of its own beside the tests of every other binary,
    and needs it installed. So a test relies on no other test of its binary having run and on no static it
    shares with one. A test that needs a device to itself joins a group in `.config/nextest.toml`, and one that
    starts a thread pool is counted there as heavy, in the landing that adds it. A test that runs for over sixty
    seconds fails the check, and a failed test is not run again. Test builds may be optimised where M8's
    measurement shows a gain; nothing but a test is built differently. No module but `GATE-TIME` adds a name to
    the list of tests exempt from the limit. Tests side by side found one race in the engine, older than M8:
    `FrameHost::capture_pixels` failed when another thread's poll of the same device took its readback's
    completion. By the lead's decision of 21 November M8 fixes it in `APPLE-TILE`'s frame host, as its item
    M8.9: the capture keeps polling until its own readback arrives or `GPU_WAIT` has passed, and no pixel or
    frame timing changes. `APPLE-TILE` keeps the frame host and, once landed, the test; its log records the
    hand-over. Any other engine fault the runner exposes goes to its owner as a proposal.

13. **How a test is written, and one test binary a package** (`GATE-TIME` M10, M9, M3 and M11; every module that
    writes a test). By the owner's approval of 21 November, "These all sound really reasonable", kept in
    [the test tiers](../../ORCHESTRATE.md#test-tiers): a merge-tier test uses the smallest world that shows its
    behaviour, and logic is tested without a device. Both bind a test written or changed from that day. M9 and
    M3 apply them to the tests over ten seconds that exist, and no module's faster tests are swept. From M11 a
    package's integration tests are modules of one test target, `it`. M11 folds the test files that nothing
    holds, in every package: an open item's check that names the binary, the main-tier list, a long test or a
    test behind a Cargo feature in the file, a `required-features` or `harness = false` entry, a build for
    another target or a script keeps a file a target of its own. It changes no assertion and no open item. An
    item approved before M11 keeps its check, and its builder adds the file as a target of its own; a planner's
    new item, and the unstarted items of a module that is re-planned, run a file's tests as
    `cargo test -p <package> --test it <file>::`. The repository guards fail a test file that no target
    compiles. Each owning module keeps its tests. M10 measures where a builder's session goes and changes no
    rule: what follows from its record is the owner's to decide.

14. **The guard rules, and how to write a test** (`GATE-TIME` M12 and M13, `TEST-SUPPORT` M6; `REPO-RULES`' rule
    engine, and every builder and planner). By the owner's approval of 21 November, "Yes, plan it", the
    fast-test habits are held in the merge check, which Codex and Claude builders both pass, and not in a hook.
    From M12 three counts enter the shape check as they stand and only fall, with the engine unchanged:
    `limit-exempt`, the tests exempt from the limit on one test; `main-tier`, the entries of the main-tier list
    and the tests marked `long:`; and `test-binary`, the lines of `.config/unplanned-test-binaries.txt`, which
    lists the integration test targets that no plan names. A plan names a target when a done-when item's check
    runs it with `--test`, open or Done, or the main-tier list names it, and a target that is neither named nor
    listed fails: so a planner who wants a builder to create a test binary names it in an item's check, and a
    builder adds any other test to a test file that exists. That binds a planner as ruling 11 does for a
    script: one who drops the last check naming a binary that exists leaves the next merge in any module to
    fail on it, so the name is kept until the file has joined `it`. A main-tier test is added only by a landing that a
    done-when item asks for, which raises `main-tier`'s version; the item says what only that test can show, as
    [the project settings](../../ORCHESTRATE.md#what-is-proved-and-how-hard) now ask of every plan, with a
    guard or a record named for each check. From M13 a test is stopped at ten seconds, and one that passes
    within ten when the check reruns it serially does not fail the merge. M14 gives only the `./dev check-all`
    rerun 30 seconds, with proof under builder load. It precedes GATE-TIME M5. `TEST-SUPPORT` owns how a test
    is written: its M6 adds the kit's small target and one block in the
    engineering skill, with the pattern entries M12's messages link to, and M12 waits for it. `GATE-TIME` owns
    the tiers and the limits, which the entries link to and do not restate.

15. **Merge verification's Cargo cache** (`GATE-TIME` M15 and M5, `REPO-RULES`, and the lead). M15 owns the
    bounded cache and the test-command cutover for both singleton and batch merges. `REPO-RULES` keeps the
    check's coverage and verdict. The lead coordinates the cutover and disk cleanup. M5 measures the first 20
    qualifying Rust landings after activation. [Cache design](GATE-TIME/DESIGN.md#cargo-cache-for-merge-verification).

<a id="not-planned-in-track-6"></a>

### Not planned in track 6

Each of these needs a design of its own, or a decision, before it is planned.

- **Shader variants made by exact-text replacement and marker slicing**, in the renderer and the cooked shader
  (L3-11, A1-08, L6-04, L6-05): the 14 November review's unscheduled theme T7.
- **Two camera draw routes**, the cube route and the bucket route (R1-10).
- **2D carried through the 3D frame host** by four parallel enums and a `Deref` that panics (R2-09), and a scene
  source that makes a 2D game implement the 3D half (L4-09). These decide how `SWARM` reaches the engine.
- **A CPU-fed cloud field** kept as a second production path (R3-01); removing a launch option is the owner's.
- **Graph resources and passes identified by integers and arithmetic** (E2-07, E1-09).
- **Four height-lattice types** across the water models (L3-06).
- **Eleven cost scripts and the browser scripts**, each with its own harness (P1-05, P1-04, P2-02, P2-08).
- **Three Mac window proofs** threaded through the production render owner (S1-08).
- **The state-delta and state-ack codecs** (N1-01, L4-01) stay, because `SESSION`'s design says keep.
- **A sweep of the tests under ten seconds** for the two rules of [ruling 13](#seam-rulings-for-track-6), and
  **changing open items so that the binaries they name fold at once**: each is an assumption in
  [GATE-TIME's plan](GATE-TIME/PLAN.md). One test binary for each water model crate (W2-01) is now `GATE-TIME`
  M11's, less the binaries that open items name.
- **Selecting a merge's tests more finely than by package** (L1-12): `GATE-TIME` M7 selects by package, and
  nothing says which test binary of a package a source file reaches without a coverage map.
- **From the hardening pass, not applied**: moving `scripts/open_check_names.py` ahead of `REPO-RULES` M3, and
  an owner for fifteen medium findings no plan names (its A8); its eleven proposals other than the frozen rules.
- **Three benches the review did not look at**: `thrum-core`'s `v1_workload`, `thrum-water-model`'s `surf` and
  the game's `recipe_cost`.
- **The grid-parity and CIEDE2000 gates of `run_v1_showcase.py`** (P1-10): `SHOWCASE`'s and `APPLE-PRESENTATION`'s.
- **The frozen pre-sprite graph snapshot** (G2-09): a guard, kept under an assumption in
  [DEAD-TOOLS' plan](DEAD-TOOLS/PLAN.md).
- **Dead scripts the review confirmed at low payoff** (L5-02, P2-04, P1-02, P2-05, L2-07), about 4,700 lines.
- **Plan text left for the modules' planners**: `GAME-SPLIT` M5 does not wait for `REVIEW-RENDER` M5 or `SWARM`
  M8, whose checks name the package it renames (L8-09); `ATMOSPHERE` M6.4 and M6.9 name folders where they could
  name what must not change (L8-11); `REPO-RULES` M3.2 would force an edit to a pinned comparison script (L8-02).

### Interfaces (track 6)

| Interface | Provider → consumers | Defined in |
|---|---|---|
| **Long test**: a test over about a minute, and from `GATE-TIME:M9` one over about ten seconds that cannot be made fast, marked ignored with a reason that starts `long:`, in a binary the main-tier list names, with the fault it leaves unguarded before a merge beside it; the main-branch check `long-tests` runs it | `GATE-TIME:M1` and `M9` → any module with such a test | [GATE-TIME](GATE-TIME/DESIGN.md#mechanisms), [the test tiers](../../ORCHESTRATE.md#test-tiers) |
| **The main-tier list**, `.config/long-tests.txt`: each long-test binary with its package and build, and each package with tests behind a Cargo feature, with those features, whole or by test binary | `GATE-TIME:M1` and `M6` → any module that adds a long test, or a test behind a Cargo feature in a package the list does not cover whole | [GATE-TIME](GATE-TIME/DESIGN.md#mechanisms) |
| **The merge selection**: `./dev check` runs the tests of the packages a change can reach, by Cargo's graph and the declared edges; `./dev check-all` and the main-branch check `workspace-tests` run the whole workspace | `GATE-TIME:M7` → every builder, and any module with a test that reads another package's files at run time | [GATE-TIME](GATE-TIME/DESIGN.md#mechanisms), [the test tiers](../../ORCHESTRATE.md#test-tiers) |
| **The runner**: `./dev check` runs each test in its own process, side by side through cargo-nextest. `.config/nextest.toml` holds the groups and heavy tests. From M13 a test stopped at ten seconds runs once more serially, with the same cap. From M14 the `./dev check-all` serial rerun has a 30-second cap and reports passes above ten seconds | `GATE-TIME:M8`, `M13` and `M14` → every module that writes a test | [GATE-TIME](GATE-TIME/DESIGN.md#mechanisms), [the test tiers](../../ORCHESTRATE.md#test-tiers) |
| **Two rules for a merge-tier test**, written or changed from 21 November 2026: it uses the smallest world that shows its behaviour, and it tests logic without a device, leaving a GPU test what a device alone can show | `GATE-TIME` → every module that writes a test, and every planner and reviewer | [the test tiers](../../ORCHESTRATE.md#test-tiers), [GATE-TIME](GATE-TIME/DESIGN.md#mechanisms) |
| **One test binary a package**: a test file under a package's `tests/` folder is a module of the package's `it` target and runs as `cargo test -p <package> --test it <file>::`; the repository guards fail a file there that no target compiles; a file is a target of its own only while an open item, the main-tier list, a Cargo feature, a long test or a script holds it | `GATE-TIME:M11` → every module that adds a test file, and every planner who writes a check that runs one | [GATE-TIME](GATE-TIME/DESIGN.md#one-test-binary-a-package), [the test tiers](../../ORCHESTRATE.md#test-tiers) |
| **Guard rules**: `limit-exempt`, `main-tier` and `test-binary`, each a count that can only fall; `main-tier` rises only through its version, in a landing a done-when item asks for; a test target is named by a plan when an item's check runs it with `--test` or the main-tier list names it, and one that is neither named nor on `.config/unplanned-test-binaries.txt` fails | `GATE-TIME:M12` → every builder and planner | [GATE-TIME](GATE-TIME/DESIGN.md#the-guard-rules), [the test tiers](../../ORCHESTRATE.md#test-tiers) |
| **Writing a test**: the small target `thrum_test_support::target`; one block in the engineering skill; the pattern entries `the-smallest-world`, `logic-without-a-device`, `simulated-time`, `an-existing-test-binary` and `a-main-tier-test` | `TEST-SUPPORT:M6` → every builder who writes a test, and `GATE-TIME:M12`, whose messages link to the entries | [TEST-SUPPORT](TEST-SUPPORT/DESIGN.md#writing-a-test) |
| **Frozen rules**: `evidence-path`, `script-caller`, `bench-runner`, `shader-text-edit`, `test-readback` and `test-deadline`, each a count that can only fall | `DEAD-TOOLS:M6` and `TEST-SUPPORT:M5` → every builder and planner | [DEAD-TOOLS](DEAD-TOOLS/DESIGN.md#mechanisms), [TEST-SUPPORT](TEST-SUPPORT/DESIGN.md#the-kit) |
| **Provenance record**: what a module copied or translated, from where and under what licence, tracked at `docs/provenance/<MODULE>.md` | `DEAD-TOOLS:M5` → `SEA`, `WATERS`, `CLOUDS`, `PUDDLES`, `RIVERS` and any module that copies a file | [DEAD-TOOLS](DEAD-TOOLS/DESIGN.md#mechanisms) |
| **Where evidence goes**: run records in the working folder, test data in fixtures, held by the rule `evidence-path`; the evidence manifest under `.config/` | `DEAD-TOOLS:M3` and `M4` → every builder and planner | [DEAD-TOOLS](DEAD-TOOLS/DESIGN.md#mechanisms) |
| **Contribution defaults**: five hooks of `SceneContribution` need not be written | `DEAD-CODE:M2` → every contribution | [DEAD-CODE](DEAD-CODE/DESIGN.md#mechanisms) |
| **Layout entries**: `thrum_render::binding`, and a graph resource declared from a texture | `DEAD-CODE:M6` → every crate that creates a bind-group layout or declares a graph texture | [DEAD-CODE](DEAD-CODE/DESIGN.md#mechanisms) |
| **The test kit**: readback, the prologue, waits, scratch folders and the record reflection check | `TEST-SUPPORT:M5`, `M3` and `M4` → every GPU test under `crates/` and in the kit scenes | [TEST-SUPPORT](TEST-SUPPORT/DESIGN.md#the-kit) |

## Owner decisions

Nothing below is decided by this plan.

1. **Port timing.** [The water decision](../design/decisions/E1-GPU-WATER.md) says implementation follows G7, and the
   port decision says the owner schedules it. The readiness conditions are met before G7, so the owner can start
   `SEA` and `CLOUDS` builders at their CPU milestones now, at their GPU milestones at step 5–6, or after G7.
   On 13 November the owner said planning can start and "you can bring that across when it makes sense".
   `SEA` and `WATERS` are planned on the reading that their plans decide when; that reading is
   [an assumption](WATERS/PLAN.md#open-for-the-human) the owner can overturn. `CLOUDS` is planned on the same
   reading, under [the same assumption](CLOUDS/PLAN.md#open-for-the-human).
2. **Licence.** Broadsides code is marked `UNLICENSED` and its cloud library carries an owner-only licence.
   Thrum's demos are meant to have public source. On 14 November the owner said Broadsides may be copied and
   reused as is. `SEA` and `WATERS` read that as the licence for what Broadsides' authors wrote, with third-party material
   keeping its own, as [an assumption](WATERS/PLAN.md#open-for-the-human). `CLOUDS` reads it the same way, under
   [its own assumption](CLOUDS/PLAN.md#open-for-the-human): the twelve accepted volumes and their growth track
   enter as the example's content from its M6, with their licence record, which grants nothing to third parties.
   What terms the library carries if Thrum's source is published is the owner's to say.
3. **The V1 exit stays as written.** Sea and clouds are not added to it. Adding them is the owner's call.
4. **Builders.** The profile says one Codex builder. Steps 2–4 of the build order hold work that is safe beside
   `APPLE-TILE` if the owner allows a second.
5. **Broadsides threads.** This plan asks nothing more of them. Whether they stay paused is the owner's call.
6. **Spending.** None is requested. Physical-device runs stay owed and are reported at the exit.

For [water as one system](#water-as-one-system), each recorded as an assumption in
[WATERS' plan](WATERS/PLAN.md#open-for-the-human):

16. **The split and the order.** Four modules, still water before running water before rain. The owner can ask
    for rivers or puddles sooner, or for one module.
17. **Flow that is prepared, not live.** Rivers do not flood, burst or change level downstream while the game
    runs. This sets aside the water decision's "all water simulation runs on the GPU" for a river's bulk flow.
18. **Surf, mirrored scenery and the camera under water**, which the owner asked for on 14 November, are planned
    as the plan's assumptions say: surf from wave theory and not from a baked simulation, with no spray;
    mirrors for two water levels in a view, through a new module, `SCENE-VIEWS`; and under water with the
    Sun's net of light, shafts and a picture that darkens with depth by a quarter of the light lost, and
    with no drops on the lens, particles or bubbles.

For [the atmosphere as a whole](#the-atmosphere-as-a-whole), each recorded as an assumption in
[CLOUDS' plan](CLOUDS/PLAN.md#open-for-the-human) or [ATMOSPHERE's](ATMOSPHERE/PLAN.md#open-for-the-owner):

19. **Where fog sits in the order.** Fog near the camera waits for `ATMOSPHERE` M6, so for TAAU and for G3s to
    close. The owner can ask for the view volume sooner, at the cost of track 2 work in the atmosphere files
    while G3s is still open.
20. **What the mist does by itself.** It forms after dusk and burns off after sunrise from the weather state,
    and low cloud can be climbed through to a summit above it. Narrower readings drop items the plan names.
21. **Lamps in fog** are not planned: the Sun, the sky and the ground light the air; local lights do not.

For `SCENE-VIEWS`, each recorded as an assumption in [its plan](SCENE-VIEWS/PLAN.md#open-for-the-human):

22. **What a reflection leaves out.** No sky, water, clouds or fog of its own, nothing below a least size,
    and Sun shadows from one cascade. Each is a cost the owner can choose to pay.
23. **No ray-traced reflection.** Reflections are rasterised on every device, the Mac included.

For `RIVERS`, each recorded as an assumption in [its plan](RIVERS/PLAN.md#open-for-the-human):

24. **No mist at a waterfall.** A fall has its sheet, its plunge, its foam and its rings, and no cloud of spray.
    Mist is a medium in the air, which is the weather's through `ATMOSPHERE`'s view volume, so it needs a seam
    with `CLOUDS` after its M14. The owner can ask for it; the fall record already carries what it needs.
25. **A river that answers its lake.** Raising a lake swells its outlet, by the outlet's own sill, at once along
    the whole river. This goes beyond decision 17's "prepared, not live" without a live solver; a rise still
    does not travel downstream.
26. **Rivers behind a selection.** The island shows the solved rivers only when the selection is on, until
    `WATERS` closes, so `WATERS`' reviews see the island they were written for.

For `PUDDLES`, each recorded as an assumption in [its plan](PUDDLES/PLAN.md#open-for-the-human):

27. **No rain in the air.** Rain is seen by what it does to the ground and the water: no streaks, splashes,
    drops on the lens or sound. Those need an owner for particles and a seam with the weather, which `CLOUDS`
    does not plan. The owner can ask for it; the rain record and the exposure field hold what it would read.
28. **Rain and drying are the game's two rates**, uniform over a scene, falling straight, and tied to neither
    the clouds nor the Sun.
29. **Puddles only where a game marks ground for them.** Elsewhere ground wets and shines and holds no puddle.
    Rain feeds no lake or river, and wet ground keeps no tracks.
30. **Rain behind a selection in the V1 valley**, off by default, so the exit scene is unchanged.

For [track 5](#track-5-ray-tracing-and-mesh-shaders-on-the-mac), each recorded as an assumption in
[RAY-TRACING's plan](RAY-TRACING/PLAN.md#open-for-the-human) or
[MESH-SHADING's](MESH-SHADING/PLAN.md#open-for-the-human):

31. **What rays are spent on.** Shadows from the Sun and lamps, soft Sun shadows and ambient occlusion.
    Reflections by ray and global illumination are not planned; decision 23 stands. The owner can ask for
    either, as a later plan on the same structure.
32. **What the mesh stages draw.** Cooked placements. Whether the exit scene's 10,000 entities follow is
    decided by the next plan from `MESH-SHADING` M3's record. The owner can ask for them now.
33. **Nothing in track 5 is a default.** No ray effect and no mesh-stage path is on by default on any
    machine. A person turns each on with a launch setting, it falls back to the path it replaces where its
    feature is missing, and `RAY-TRACING` M6 and `MESH-SHADING` M3 record how it looks and what it costs on
    this Mac, timed while other builders work. Making either a default needs a measured win on a quiet
    machine and a rule for which Macs it covers, and in the showcase it would change the V1 exit. The two can
    be turned on together; neither module checks the pair. The mesh stages and the ray effects draw only on
    the Apple tile recipe, so each setting also selects Apple tile where no recipe is named, and a launch
    that names the portable recipe stays on the existing path.
34. **Effects with no portable version.** Lamp shadows and ambient occlusion appear only on a device that
    traces rays, on the Apple tile recipe. On the portable recipe a scene that asks for a ray effect draws without it and reports why. No screen-space stand-in is planned for Chrome or the portable recipe.
35. **Where track 5 sits in the queue.** Behind every V1 milestone. Its order among the other work off the
    exit is the lead's; the owner can set it.
36. **wgpu's experimental features.** Both modules depend on them. A fault in wgpu waits for
    `APPLE-PRESENTATION` M10's pinned copy, and nothing is reported upstream without the owner.

For [track 3](#track-3-the-swarm-survival-game-after-v1):

7. **When track 3 starts.** Decided on 14 November 2026: the engine modules are built beside V1 and the game
   after it ([the split](#the-split-of-14-november-2026)). This plan reads "the swarm game" as all of `SWARM`,
   the owner's pick of a concept included; the owner can ask for that pick sooner. Transport's timing was
   decided on 15 November 2026: "it can come before V1 if we have space for it, but it's not urgent", so
   `TRANSPORT` waits for nothing and is [last in line for a builder](#transports-timing). Whether the game goes before,
   after or beside sea and clouds is still the owner's order to set.
8. **The game's identity.** Its name, premise, characters and look are the owner's taste. `SWARM`'s first
   milestone lays out options and waits for the owner's pick. Its last waits for the owner to play the game.
9. **Art and licence.** The plan assumes art made in this repository, original or PD and CC0. A paid generation
   service is spending. The licence the game and its art are published under is the owner's.
10. **A public host.** Play across the internet needs a machine others can reach, a domain and a certificate,
    which is spending and deploying. Until the owner asks for one, "online" is proved between separate processes
    on this Mac over real QUIC and WebTransport under the A0 conditioner. Putting the game on the website is
    publishing.
11. **Sockets and Chrome under the Codex sandbox.** Decided on 14 November 2026: the project settings give
    Codex `danger-full-access`.
12. **Sound.** The brief does not ask for it and no audio module is planned. A1 stays a design.
13. **The purpose line.** The brief still says "3D engine" and requirement E08 of the architecture brief
    still puts pure 2D outside scope. The 10 November goal overrides both; rewording them is the owner's page.
14. **What finishes the goal.** [The proposal](#what-finished-means-track-3) stands in for an exit the owner has
    not written.
15. **Builders.** Steps 1 and 2 of track 3's order hold three lanes that share no files. With the engine
    modules now beside V1, the settings' two builders choose between a V1 milestone and an engine one; this
    plan says V1 first, and the owner can say otherwise.

## Assumptions for modules not yet planned

Each stands until the owner overturns it, and moves to its module's plan, under `Open for the owner`, when that module is planned.

- The sea's assumptions have moved to [SEA's](SEA/PLAN.md#open-for-the-human) and
  [WATERS'](WATERS/PLAN.md#open-for-the-human) plans.
- The `RIVERS` assumptions have moved to [its plan](RIVERS/PLAN.md#open-for-the-human).
- The `PUDDLES` assumptions have moved to [its plan](PUDDLES/PLAN.md#open-for-the-human).
- The clouds' assumption has moved to [CLOUDS's plan](CLOUDS/PLAN.md#open-for-the-human).
- The `SCENE-VIEWS` assumptions have moved to [its plan](SCENE-VIEWS/PLAN.md#open-for-the-human).
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
- 2026-11-14 `CLOUDS` revised after [its design review](../../.orchestrate/evidence/CLOUDS/design-review.md).
  [Ruling 1](#seam-rulings-for-clouds) now gives a GPU-written Sun-path field a path direction, a border and a
  far value, so its factor is continuous in the Sun's elevation and across the field's edge; the Sun-path row
  says the same. The note on stacked volumes states how far their mips stay inside a slab. No other ruling,
  contract, exact baseline or other module's done-when item changed.
- 2026-11-14 `SEA` judged again from first principles, on the owner's words of 14 November quoted in
  [its design](SEA/DESIGN.md#the-request): Broadsides is a resource to copy from where a design finds it useful,
  and never dictates one. The layering, query, wake, foam and optics are kept. The ambient sea is now evaluated
  every frame at the display's time, and each part beyond the open sea is a capability a scene can leave out.
  [The port decision](../design/decisions/SEA-AND-CLOUD-PORT.md) and the water decision's open-sea row were
  brought into step, and owner decision 2 now records the sea's reading of the licence. The body-query row says
  "the times they were made from". No seam ruling, contract, exact baseline or other module's item changed.
- 2026-11-14 `CLOUDS` judged again from first principles, on the owner's words of 14 November quoted in
  [its design](CLOUDS/DESIGN.md#the-request). Kept: the weather field and its CPU query, the Sun-path product,
  placed volumes as cells of the field, and light and shade through `ATMOSPHERE`. Changed: one medium whose
  sources are data in place of two paths and a hybrid, capabilities a scene leaves out, the droplets' own phase
  for the first scattering order, drift at the display's time, shapes a game can place itself, and the accepted
  library as the example's content from M6 with no licence wait. [The port decision](../design/decisions/SEA-AND-CLOUD-PORT.md)'s
  cloud half, the module table's row and owner decision 2 were brought into step. No seam ruling, contract, exact
  baseline or other module's item changed.
- 2026-11-14 Water re-planned as one system, on the owner's words of 14 November quoted in
  [WATERS' design](WATERS/DESIGN.md#the-request). `SEA` is narrowed to the wave field, with M1 and M2 unchanged;
  `WATERS` is added with a design, a plan and eleven milestones that carry `SEA`'s old M3 to M10 for every body of
  water; `RIVERS` and `PUDDLES` are added with [briefs](#rivers-and-puddles).
  [Water as one system](#water-as-one-system) gives the decision. The body-query, medium-segment, knob and
  control rows now name `WATERS`; rows for the wave field, the water map and impulse records were added; the
  three sea rulings are now [rulings for `WATERS`](#seam-rulings-for-waters), with ruling 2 covering see-through
  and mirror-still water; `CLOUDS` M11 waits on `WATERS:M8`. Track 3's rows are renumbered 19 to 24. No
  contract, exact baseline or other module's item changed.
- 2026-11-14 `WATERS` re-planned after [its design review](../../.orchestrate/evidence/WATERS/design-review.md) asked for changes.
  The water-map row now carries a chart's ends and joins, stage ranges, what preparation refuses and the shore
  margin. `RIVERS`' brief gains the chart's ends and how a reach answers its source's stage; `PUDDLES`' brief
  gains the seam question of basins that merge, and its wet shore is the sand's darkening under the film
  `WATERS` draws. No contract, seam ruling, exact baseline or other module's item changed.
- 2026-11-14 The owner overturned three of `WATERS`' assumptions: "Scenery reflections we must have ... Camera
  being able to go under water we need to support as well. Breaking waves is something we'd like to have as
  well." `WATERS` re-planned: four milestones added (M12 surf on the CPU, M13 surf drawn, M15 under water, M14
  mirrored scenery) and its existing numbers kept. `SCENE-VIEWS` added at row 16 with a brief, the mirror-view
  interface and ruling 4; later rows renumbered. `SEA`, `CLOUDS` and every other module's done-when file are
  unchanged.
- 2026-11-14 The owner: "Oh and for clouds we also need to be taking into account mist / fog. We want a plan
  that's able to deliver on top quality atmosphere." Ownership settled in
  [the atmosphere as a whole](#the-atmosphere-as-a-whole): the weather, in `CLOUDS`, owns every condensate from
  cloud tops to ground mist and the amount of haze; `ATMOSPHERE` owns the air's optics and the view volume that
  integrates air, haze and a medium jointly near the camera. `CLOUDS` gained M13 to M16 and three items in M11;
  `ATMOSPHERE` gained M7 and M8 after G3s. Rulings 6 to 8 and three interfaces were added and ruling 4 narrowed.
  `CLOUDS` M1's items are unchanged. No contract's text and no exact baseline changed.
- 2026-11-14 `SCENE-VIEWS` planned, with a design, plan and done-when file, from the lead's task and
  [its brief](#scene-views). Three milestones: entities in a view (after `CULLING:M3`, `ATMOSPHERE:M5` and
  `EFFECT-GRAPH:M3`), cooked placements with the mirror example, and Chrome, cost and the close. The
  mirror-view row became the secondary-view interface, [five seam rulings](#seam-rulings-for-scene-views)
  settle its asks of `EFFECT-GRAPH`, `CULLING`, `ATMOSPHERE`, `APPLE-PRESENTATION` and the cooked draw, and
  C07 gained a paragraph naming a secondary view. `WATERS` M14 now waits for `SCENE-VIEWS:M3`. No exact
  baseline and no other module's done-when item changed.
- 2026-11-14 `WATERS` re-planned for light under water, on the bar the owner's words of that day set: the
  caustic field, shafts and a meter that follows the light, in a new M16 after M15. Rows for the Sun-path
  factor, the lent Sun visibility and the meter's medium factor were added, with
  [rulings 5 to 7](#seam-rulings-for-waters) for `ATMOSPHERE` and `APPLE-PRESENTATION`. `WATERS` M1 is
  unchanged. C07's incident meter gained a clause for the medium's transmission. No other contract text, no
  exact baseline and no other module's item changed.
- 2026-11-14 `RIVERS` planned from [its brief](#rivers-and-puddles): [design](RIVERS/DESIGN.md), seven
  milestones. The river and fall-record rows were added, with [seven rulings](#seam-rulings-for-rivers) for
  what it builds in `WATERS`' crates. Two of them replace what `WATERS`' design expected of it: the flow has
  wave layers of its own in place of a gain on the wind's cascades, which give a stream nothing in a calm,
  and a river is solved at several levels of its source in place of one prepared flow. `WATERS`' design, plan
  and items are unchanged; its log records the rulings. No contract text, no exact baseline and no other
  module's item changed.
- 2026-11-14 `PUDDLES` planned from [its brief](#rivers-and-puddles): [design](PUDDLES/DESIGN.md), seven
  milestones. Rows for the rain record and ground wetness were added, the water-map row names the hollows
  kind, and the impulse-records row no longer lists rain's drops. [Seven rulings](#seam-rulings-for-puddles)
  settle what it builds in `WATERS`' crates, the forward shader and the two games. Three replace what
  `WATERS`' design expected: all the hollows of a stretch of ground are one body in place of a basin for each
  puddle, which is how puddles merge with nothing rebuilt; rain's rings are wave layers in place of impulse
  records; and the shore's film is also drawn over standing rain water. `WATERS`' design, plan and items are
  unchanged; its log records the rulings. No contract text, no exact baseline and no other module's item
  changed.
- 2026-11-14 Four designs revised after their design reviews: `ATMOSPHERE`'s view volume, `SCENE-VIEWS`,
  `RIVERS` and `PUDDLES`. Interfaces changed: the ground band of the Sun-path optical depth holds the whole
  path and is read once, never added to the 2D field ([ruling 7 for `CLOUDS`](#seam-rulings-for-clouds),
  with `CLOUDS`' ruling text and M14.3 brought into line); a ray that leaves the view volume corrects the sky
  once and added emitters by transmittance alone; a mirror view's air starts at its plane and the reader
  owns the stretch before it ([ruling 3 for `SCENE-VIEWS`](#seam-rulings-for-scene-views), with `WATERS`'
  mirror text brought into line).
- 2026-11-14 The same revision, for water: a chart's samples carry a wetting gauge
  ([ruling 2 for `RIVERS`](#seam-rulings-for-rivers)); the interaction step gains a current ladder in place
  of a weighted mean current (ruling 5); `PUDDLES`' fine interaction level moved to a new milestone after
  `WATERS:M6` (ruling 2), and a hollows body gives the mirror bands of levels in place of the mean of its
  ground ([ruling 8 for `PUDDLES`](#seam-rulings-for-puddles)); the rain bands read the rain's rate over the
  last second, so stopping the rain lets its rings run out (ruling 3), and shelter is up to two layers in a
  column. No contract text and no exact baseline
  changed, and no item of a module other than these four, `CLOUDS` M14.3 and `WATERS` M14.2 changed.
- 2026-11-14 `SCENE-VIEWS` and `RIVERS` revised after their second design reviews, one Must Fix each. A
  view's air is laid out from the crossing and held to `ATMOSPHERE`'s own product drawn from there, with no
  transport bound of its own ([ruling 3 for `SCENE-VIEWS`](#seam-rulings-for-scene-views)). A chart's
  wetting gauges are written by the solve from the lower sample's water and from each inflow, so a reach
  that was dry wets when its lip runs, and carriage currents span a region's currents as vectors
  ([rulings 2 and 5 for `RIVERS`](#seam-rulings-for-rivers)). No design, item, test or bound of `WATERS`,
  `PUDDLES` or `ATMOSPHERE` changed, and no contract text.
- 2026-11-14 `SCENE-VIEWS` revised after its third design review, one Must Fix. A view's aerial volume is
  the pass's own for an observer at the plane's height on every line, so a fragment beside a line that
  never crosses the plane reads what the main view reads from the crossing; air from the plane now needs a
  level plane ([ruling 3 for `SCENE-VIEWS`](#seam-rulings-for-scene-views)). Every plane `WATERS` and
  `PUDDLES` give a view is a water level, so no design, item, test or bound of `WATERS`, `PUDDLES` or
  `ATMOSPHERE` changed, and no contract text. Open for `ATMOSPHERE`'s next plan, in its log: the aerial
  pass stops a column at the model's ground, which thins haze just above the horizon for any observer
  near sea level, the main view's included.
- 2026-11-14 `ATMOSPHERE` re-planned for [the air at the ground](ATMOSPHERE/DESIGN.md#the-air-at-the-ground),
  acting on `SCENE-VIEWS`' proposal: a new milestone M9, built before M6, in which the aerial pass carries a
  line of sight on under the model's ground, with the air's density by its own law and the Sun's light dimmed
  by that air. No interface's arguments or units change. [Exact baselines](#exact-baselines) gains a ruling:
  M9 changes the default output, all five registered sets are expected to move, and M9's builder refreshes
  them in its own landing. `WATERS` M2 now waits for `ATMOSPHERE:M9`. `ATMOSPHERE` M7.3 gains probes near
  the ground. No design or item of `SCENE-VIEWS`, `CLOUDS` or `APPLE-TILE` changed, no other item of
  `WATERS`, and no contract text.
- 2026-11-14 Dependency audit, on the lead's task from the owner's word that too few milestones can start at
  once: [the record](../../.orchestrate/evidence/dependency-audit-2026-11-14/README.md) gives a verdict for each of the 82
  edges. Five only set an order: `SCENE-VIEWS` M1 now waits for `CULLING:M1`, `PUDDLES` M6 for `SHOWCASE:M5`,
  `SHOWCASE` M8 and not M7 for `PLATFORM-RUNTIME-MAC:M3` and `CREATOR:M3`, and `SWARM` M3 and M6, not M2, for
  `GAME-UI`. `SHOWCASE` M7.1 no longer asks for `ATMOSPHERE`'s track-2 milestones. The swarm gate stays, by the
  owner's decision. Nine milestones can start once the running ones finish, as before: the limit is one
  milestone a module, two designs not approved and `APPLE-PRESENTATION` M4 to M11 not planned.
- 2026-11-14 Two changes, from the lead's task. (1) The owner's decision of that day, quoted in
  [the split](#the-split-of-14-november-2026): the five engine modules of track 3 lost their gate on
  `SHOWCASE:M8` and keep only the edges that section lists; `SWARM` keeps the gate. A driven world moved from
  `SWARM` M7 to a new `SESSION` M7 and the shared iPhone shell from `SWARM` M6 to a new `GAME-UI` M6, with their
  rulings. (2) `APPLE-PRESENTATION` M4 to M12 planned, seams 5 to 9 settled, and `SHOWCASE` M7, `WATERS` M11
  and `CLOUDS` M12 given their real edges. `APPLE-PRESENTATION` M1 to M3 are unchanged.
- 2026-11-14 Track 4 added from two requests of the owner's, quoted in its section: the code review's fixes and
  the game's separation, as six modules (rows 26 to 31) with designs, plans and done-when files. No item of any
  other module changed. `SWARM` M2 gained a dependency on `GAME-SPLIT:M4`, since the swarm game is built in the
  games repository; `SWARM`'s paths are re-planned before that milestone. Decided here: fixes are built in new
  modules under rulings, because eight owning modules have builders at work and an approved design cannot gain a
  mechanism without stopping them; the game moves after `SHOWCASE:M8`; no Git history is rewritten.
- 2026-11-14 [Exact baselines](#exact-baselines) gained a ruling on `CULLING` M3, from the lead's task after
  its builder stopped under rule 1. `CULLING` M3 builds the forward pass's coverage output and regenerates the
  `APPLE-TILE` references it moves in the same landing; the references stay `APPLE-TILE`'s. `CULLING` M3.10 was
  rewritten and M3.11 to M3.13 added. No item of any other module changed, and no module or milestone was added.
- 2026-11-15 Track 4's six designs revised after their first design reviews (16 Must Fix), on the lead's task.
  Settled between modules: shared fixtures are `TEST-SUPPORT`'s second package, `thrum-fixtures`, which the
  proving ground may take as a normal dependency and `GAME-SPLIT` M2 fills (ruling 9); the device recipe takes a
  restriction for tests (ruling 7); `library-env` exempts the registered test-support package; `TEST-SUPPORT` M2
  now waits for `GAME-SPLIT:M2`. `REVIEW-RENDER` M5 keeps the timestamp resolve in a later submission, because
  the retained Metal evidence outranks the code review's M40. The owner's decisions stand: no history rewrite,
  the game at `/Users/ryannel/Workspace/thrum-games` with path dependencies, `CLOUDS` and `WATERS` unchanged.
  No item of any module outside track 4 changed.
- 2026-11-15 The ruling on `CULLING` M3 gained "Qualified before the push", after that design's review (MF2):
  the coverage output and the refreshed references are qualified on a local candidate and then pushed together.
  Ownership, the registry, the comparisons and every other module's items are unchanged.
- 2026-11-15 `GAME-SPLIT` revised after its second design review (one Must Fix). `GAME-SPLIT` M1 now waits for
  `TEST-SUPPORT:M1`, which delivers the `test-crate-edges` rule the game gate's `boundary` runs; the start order
  and row 31 say so. Ruling 11 gained the verifier's one change: it hands `THRUM_GAMES` to the commands it
  starts. No item of any other module changed.
- 2026-11-15 [Exact baselines](#exact-baselines) gained a ruling on a frame that varies under load, after
  `EFFECT-GRAPH` M3's builder stopped on one Chrome record of 481 and named `APPLE-TILE`. The planner's reading
  of the retained frames puts the difference on shared triangle edges, the variation `CULLING` M3 corrects. A
  failed run of that kind may be set aside, with a record, when the whole verifier then passes on the same
  content; the rule ends with `CULLING` M3's landing. `EFFECT-GRAPH` M3 gained M3.9. The registry, the
  verifier, the comparisons, every reference and every other module's items are unchanged.
- 2026-11-15 [Exact baselines](#exact-baselines) gained a ruling on a Chrome record that holds the tick before,
  after `REPO-RULES` M1's builder stopped on one identity of 481 with every image equal and named `APPLE-TILE`.
  The planner's reading: no drift and no stale key, since the same content passed within the hour; a stall of
  the page left one due tick for the next callback of a fixed capture. A failed run of that kind may be set
  aside, with a record, when the whole verifier then passes. No milestone was added to `APPLE-TILE`; the
  correction is `HOST`'s to make and is not planned. The registry, the verifier, the comparisons, every
  reference and every module's items are unchanged.
- 2026-11-15 `TRANSPORT` M1 waits for `SHOWCASE:M8` again, after its design review of 15 November (`d686fe75`,
  one Must Fix): the brief's decision 1 says "transport stays after V1", and the owner's words of 14 November
  name the 2D engine side. The split, the edge table, the build order, row 21 and owner decision 7 say so.
  `SESSION`'s files are unchanged: its M2 onward wait through `TRANSPORT:M1`, and the sentence in its design
  that it "waits for nothing of V1's" now holds for its own edges only; reword it when that plan next changes.
- 2026-11-15 The owner ruled on transport's timing: "it can come before V1 if we have space for it, but it's not
  urgent". `TRANSPORT` M1 no longer waits for `SHOWCASE:M8`, and `TRANSPORT` is the lowest priority: the lead
  starts its builder only when a builder slot is free. Row 21, the split, the edge table, the build order, its
  entry and owner decision 7 say so, and the brief's decision 1 carries the ruling. `SESSION`'s files are
  unchanged: its M2 onward wait on `TRANSPORT:M1` alone, so its design's "waits for nothing of V1's" holds again.
- 2026-11-15 [Exact baselines](#exact-baselines) rule 2 gained "The covered paths are derived", on the lead's
  task with the owner's approval. `BASELINE-GUARD` gained M2: the proof key covers the files a verified build or
  comparison reads, taken from Cargo's resolved graph, the scripts' references and the compilers' dependency
  records, in place of a hand list that took in every crate and script. The verifier's command, the registry's
  baselines, every reference, every comparison and every other module's items are unchanged.
- 2026-11-15 "The covered paths are derived" revised after `BASELINE-GUARD`'s fourth design review (MF3): a
  file is covered unless a rule withholds it, in place of a set found from the scripts' imports and quoted
  paths, so a script's optional and listed data files are in the key. A tripwire stands in the verification
  root for each script the key leaves out. No baseline, comparison or other module's item changed.
- 2026-11-15 "The covered paths are derived" replaced by "Unlinked crates leave the covered paths", on the
  lead's task after `BASELINE-GUARD`'s fifth design review returned MF3 a third time. M2 now keeps M1's listed
  paths and leaves out only the files of workspace packages Cargo does not link into `v1-game` or
  `thrum-cook`; the derived key, the verification root, unread sources and tripwires are withdrawn and recorded
  as later work in that design. The paragraph keeps its anchor. No baseline, comparison or other module's item
  changed.
- 2026-11-15 [Exact baselines](#exact-baselines) gained the ruling
  [proofs run once, on main](#proofs-run-once-on-main), on the lead's task with the owner's approval. Rule 2
  moved from a proof before each push to one run on main after the merge, by the main-branch check
  `exact-images`; rule 3's item now passes from that shared run and takes no lock; the gate in `./dev check`
  only warns. "The covered paths are derived" is withdrawn, and `BASELINE-GUARD` M2 is planned again as the
  reference guard. `ORCHESTRATE.md` declares `gpu: 2` and `chrome: 1` as shared and `simulator` as exclusive.
  No baseline, comparison or item text of another module changed; only items' `lock` fields did.
- 2026-11-16 `CULLING` M3's builder stopped before its landing: M3.10 and the ruling on M3 said the registry
  stays as it is, and the reference guard, which landed after both were written, refuses moved references
  without a new pin and refresh. Rule 2 now says what a planned refresh changes in the registry: the `pin` and
  `refresh` of each baseline whose reference moved, and nothing else, in a commit that repeats no image proof.
  The rulings on `CULLING` M3 and `ATMOSPHERE` M9 say the same and gained anchors for a refresh to name,
  `ruling-on-culling-m3` and `ruling-on-atmosphere-m9`. `CULLING` M3.10 and `ATMOSPHERE` M9.5 are written
  again for it. Nothing of `BASELINE-GUARD` or `APPLE-TILE` changed.
- 2026-11-16 The ruling on `CULLING` M3 lost "Qualified before the push" and gained "Proved once, then
  merged", on the lead's task after a job audit traced about 30% of failed background jobs to the pipeline
  that enforced the old order. M3 lands as rule 2's planned refresh; its campaign runs once and is tied to main
  through the references. `CULLING` M3.10 and M3.11 are written again. The second of two refreshes may take
  the main-branch check's pass as its starting record. No reference, comparison, registry rule or other
  module's item changed.
- 2026-11-16 The display phase added as an interface, with a seam ruling for `EFFECT-GRAPH` M4, on the lead's
  task after `CLOUDS` M2 ended `BLOCKED needs_dependency`. `CLOUDS` M2 now needs `EFFECT-GRAPH:M4`. No other
  module's item or file changed. `SEA`, `WATERS`, `RENDER-2D`, `HOST` and `SESSION` are named in the ruling:
  their planners read it there.
- 2026-11-16 The ruling on `CULLING` M3 gained "The loaded proof is made on main", after a design review
  found that the registered references are drawn Direct and so cannot tie a campaign made on the branch to
  the culled and bucket pipelines of the merged build. M3.11's campaign is taken once on main after the
  merge. `CULLING` M3.11 is written again. The ruling on a frame that varies under load now also stops a
  module whose own exact comparison varies along triangle edges on content that holds the output. No
  reference, comparison, registry rule or other module's item changed.
- 2026-11-16 The display phase and its ruling revised after the design reviews of `EFFECT-GRAPH` and `CLOUDS`
  each found one Must Fix in it. The phase describes the consumed target, the host's observed time plus its
  lead; the offscreen runner reads the clock once; a prediction past the committed tick is carried a tick at
  a time. `EFFECT-GRAPH` M4.1 to M4.4 and `CLOUDS` M2.7 and M2.11 are written again and `CLOUDS` gains M2.13.
  No other module's item or file changed.
- 2026-11-16 `SCENE-VIEWS` ruling 3 and the secondary-view interface now say where a view's camera stands,
  after its design review found one Must Fix: a view reads the frame's caster height map and air shadow
  heights, which are its own only on the main camera's vertical, so a request from a camera off it is
  refused. Every water mirror is on it. `SCENE-VIEWS` M1.1 and M2.1 are written again and it gains M1.13. No
  other module's item, file or design changed.
- 2026-11-16 `SCENE-VIEWS` ruling 3 now has the scene's fit lay the caster height map out from the position a
  camera declares, in every scene, after its sixth design review carried the Must Fix forward: taken back
  out of a view matrix, that position rounds differently by direction and height, so on a texel's edge two
  cameras on one vertical could have maps 32 m apart. A scene with no view changes only at such a camera.
  A reproduction of the showcase valley's cameras expects no registered exact baseline to move
  ([part G](../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)); the verifier decides. `SCENE-VIEWS` M1.1, M1.7
  and M1.13 are written again and it gains M1.14. No other module's item or design changed.
- 2026-11-16 `T2-CAPTURE` gained M3 and the ruling on
  [a motion capture that drops frames under load](#ruling-on-a-capture-that-drops-frames), after the
  main-branch check failed four times at realtime motion arms under other modules' GPU tests. A realtime capture
  may now wait for its readback ring instead of dropping, and an incomplete Mac capture exits 75. M3's builder
  edits `APPLE-TILE`'s matrices and `BASELINE-GUARD`'s verifier under that ruling; neither module's items change.
- 2026-11-17 [Ruling 8 for `CLOUDS`](#seam-rulings-for-clouds) and the air's-haze interface now carry the
  haze's kind, an Ångström exponent, beside its amount, after `ATMOSPHERE`'s f64 reference sky showed the
  colour it gives visible at a low Sun in haze of 20 and 3 km
  ([the check](../../.orchestrate/evidence/ATMOSPHERE/haze-kind-check.md)). The weather state holds it (`CLOUDS` M16) and
  `ATMOSPHERE` M7 applies it; the default, 0, is today's grey aerosol, so no scene and no exact baseline changes.
  No contract changed.
- 2026-11-17 Track 5 added from the owner's words, quoted in its section: `RAY-TRACING` (six milestones) and
  `MESH-SHADING` (three), each with a design, plan and done-when file, at rows 32 and 33. A planning probe
  showed wgpu 30.0.1 reaches ray queries and the object and mesh stages on this Mac. Eleven seam rulings, four
  interfaces and owner decisions 31 to 36 were added, and the hardware-native decision's Apple tile row and
  sequencing were brought into step. No V1 module's design, plan or done-when item changed, nothing was added
  to a `SHOWCASE` dependency, and no contract or exact baseline changed. Both design reviews are owed.
- 2026-11-18 Track 5 rulings 1, 4 and 10, the cooked draw path interface and owner decision 33 were amended
  after [MESH-SHADING's design review](../../.orchestrate/evidence/MESH-SHADING/design-review.md): the window asks for
  `mesh-stages` unasked only in a qualified configuration, meshlet storage follows the contribution's resolved
  path and each mesh's eligibility, a material reload covers both forward pipelines, and `Auto` reads the Sun
  shadow method. `RAY-TRACING`'s half of ruling 4 keeps its meaning. No contract or exact baseline changed.
- 2026-11-18 Track 5 rulings 1, 3, 4 and 5, the three ray interfaces, the module table's row 32 and owner
  decisions 33 and 34 were amended after [RAY-TRACING's design review](../../.orchestrate/evidence/RAY-TRACING/design-review.md):
  the window asks for `ray-query` unasked only in a qualified configuration, ray structures follow the scene's
  ray demand and not the device, a contribution offers what it holds and not what a pass draws, ray effects
  resolve only on the Apple tile recipe, and promotion needs a gain as well as no higher cost. No V1 module's
  item, contract or exact baseline changed.
- 2026-11-18 Track 5 rulings 1 and 7, the launch and traced-visibility interfaces, the module table's row 32
  and owner decisions 33 and 34 were amended after [RAY-TRACING's second design review](../../.orchestrate/evidence/RAY-TRACING/design-review.md)
  and the lead's direction, quoted in [its design](RAY-TRACING/DESIGN.md#the-request): no ray effect is promoted
  or chosen by the engine, a ray request that cannot be met falls back to the map frame, a launch gives a
  hardware name as required or preferred, and the frame's timing record gains a whole-frame GPU span.
  `MESH-SHADING`'s half of ruling 1 keeps its meaning. No V1 module's item, contract or exact baseline changed.
- 2026-11-18 Track 5 rulings 1, 7, 8 and 10, the launch and cooked-draw-path interfaces, the module table's
  row 33 and owner decisions 32 and 33 were amended after
  [MESH-SHADING's second design review](../../.orchestrate/evidence/MESH-SHADING/design-review.md) and the lead's direction,
  quoted in [its design](MESH-SHADING/DESIGN.md#the-request): the mesh stages are an opt-in a launch and a
  scene ask for and no host or `Auto` selects them, a `MeshStages` request that cannot be met falls back with
  a reason, the caster view layout is visible to the object and mesh stages on a device that enabled them,
  and whichever of `RAY-TRACING` M3 and `MESH-SHADING` M2 lands first adds the whole-frame GPU span. No V1
  module's item, contract or exact baseline changed, and no file of `RAY-TRACING` changed.
- 2026-11-18 Track 5 ruling 7, the cooked-draw-path interface and owner decision 33 were amended and ruling 12
  was added after [MESH-SHADING's third design review](../../.orchestrate/evidence/MESH-SHADING/design-review.md):
  `MeshStages` is met only on the Apple tile recipe, a host may prefer Apple tile where no recipe is named,
  and the timing record's span is named the timed-pass span, with the queued transfers it leaves out
  disclosed. No V1 module's item, contract or exact baseline changed, and no file of `RAY-TRACING` changed.
  Its design still calls the span the whole frame's and says its opt-in leaves the recipe as it is today.
- 2026-11-18 Track 5 ruling 12 and owner decision 33 were amended after
  [RAY-TRACING's third design review](../../.orchestrate/evidence/RAY-TRACING/design-review.md): `RAY-TRACING` M6 builds
  the recipe preference if `MESH-SHADING` M3 has not, and its launch selection prefers Apple tile on a
  generation that enabled `ray-query`. Its design now names ruling 7's timed-pass span as that ruling does.
  No V1 module's item, contract or exact baseline changed, and no file of `MESH-SHADING` changed.
- 2026-11-18 `SHOWCASE` M5 ended blocked on a Sun-shadow strip under the exit scene's balloons. Added
  `ATMOSPHERE` M10, the aloft-casters interface and [its seam ruling](#seam-ruling-for-atmosphere-m10): the
  caster height map holds the ground and what stands on it, and a marked row is left out of it. `SHOWCASE` M5
  now waits for `ATMOSPHERE:M10`. No contract or registered exact baseline changed, and no item of `CULLING`.
- 2026-11-18 `SCENE-VIEWS` M3 ended blocked asking for a timing sitting with the other builders paused, the
  second last milestone to stop that way after `CULLING` M3. The project's rule now makes every timing a
  tripwire taken under load, a last milestone's included; quiet figures wait for one batched window if they are
  still wanted, and only a sitting a design names, as `SHOWCASE` M8's, pauses the others
  ([settings](../../ORCHESTRATE.md#targets)). `SCENE-VIEWS` M3.4 was written again to it. No interface,
  contract or exact baseline changed, and no item of another module.
- 2026-11-18 `SESSION` M1's review found that [ruling 1](#seam-rulings-for-session) asked for two things no build
  could give together: a compiler that refuses a type with half the encoding, and `SIM-2D`'s accepted tests
  left as they were, whose fixtures register such types. The ruling now says those fixtures gain the decoding
  half and nothing else in an accepted test changes. `SESSION`'s design also has a rule set's settings in its
  identity, so the crowd's Crowd and Mixed rules state two. No item of `SIM-2D`, contract or exact baseline
  changed.
- 2026-11-19 `ATMOSPHERE` M6 ended blocked on "Assign the shipped valley fix and authorize the affected
  APPLE-TILE exact-reference refresh." [The ruling on the valley's lookout rim and sunrise stop](#ruling-on-the-valley-fix)
  gives the valley's source and stop presets to `SHOWCASE`, adds `SHOWCASE` M9 to fix the rim's mesh and move
  the sunrise stop's hour, lets that milestone refresh `APPLE-TILE`'s five sets and write the new cooked root
  into the route files of `APPLE-TILE`, `CULLING` and `EFFECT-GRAPH`, and lets `ATMOSPHERE` M6 refresh the sets
  once more if its list needs a change to the look. `ATMOSPHERE` M6 now waits for `SHOWCASE` M9 and carries the
  verifier as M6.9. No item of `APPLE-TILE`, `CULLING` or `EFFECT-GRAPH`, comparison or contract changed.
- 2026-11-20 `SHOWCASE` M9's review failed M9.6 on the lead's move of the retained references out of
  `docs/evidence/`, which the item's wording forbade. [Exact baselines](#exact-baselines) gained
  [the ruling on where retained references are kept](#ruling-on-relocated-references): a relocation keeps
  every file's name and bytes, and only the paths that say where the references are follow it. The text on a
  planned refresh and the valley ruling point at it, and `SHOWCASE` M9.6 is written again. No comparison,
  sampling, reference byte or contract changed, and no item of `APPLE-TILE`, `BASELINE-GUARD` or `ATMOSPHERE`.
- 2026-11-20 [Track 6](#track-6-the-cleanup-of-20-november) added from the owner's request, "We want to do a
  code cleanup and consolidation for Thrum", and the cleanup review of that day: `GATE-TIME`, `DEAD-TOOLS` and
  `DEAD-CODE`, rows 34 to 36, and M3 and M4 of `TEST-SUPPORT`. The project settings gained the rule that a Done
  item's check is its record, and three rules for wording an item. Open items of `HOST`, `CLOUDS`, `SHOWCASE`,
  `WATERS`, `ATMOSPHERE`, `SWARM` and `PUDDLES` were reworded where a check passed on a missing path or an item
  asked for a file under `docs/evidence`; each module's log says which. `REPO-RULES` M3 now waits for track 6.
  No Done item, exact baseline, comparison or contract changed.
- 2026-11-20 Track 6 revised after its first design reviews and three owner decisions of that day. The gale and
  every other test over about a minute leave the merge's check whole, under
  [the test tiers](../../ORCHESTRATE.md#test-tiers) now in the project settings, and run in release on main
  (seam ruling 2). All nine benches go. Provenance records are tracked under `docs/provenance` (seam ruling 8):
  `DEAD-TOOLS` gained M5, built first, and `CLOUDS` M6 waits for it. Seam ruling 7 now says what the designs
  say: a pinned comparison file is never edited, and a script it imports keeps its name and what the pinned
  files use. Open items of `CLOUDS` and `WATERS` that named the working-folder provenance file were reworded.
- 2026-11-20 Track 6 revised a third time, in one pass over `GATE-TIME`, `DEAD-TOOLS`, `DEAD-CODE` and
  `TEST-SUPPORT`: the second design reviews' Must Fix findings, the hardening pass, and the owner's decision that
  the merge tier is targeted, which the lead gave as "Every merge runs only the tests for what the change
  touches, and the full workspace runs on main." New milestones: `GATE-TIME` M6 (tests behind a Cargo feature)
  and M7 (the merge selection), `DEAD-TOOLS` M6 and `TEST-SUPPORT` M5 (six rules frozen before their cleanups).
  Seam rulings 9 to 11 and four interfaces were added, the wave table gained `TEST-SUPPORT`, and the test tiers
  in the project settings were written again. No Done item, exact baseline, comparison or contract changed.
- 2026-11-20 `GATE-TIME` and `DEAD-TOOLS` revised after their third design reviews. Seam rulings 9 to 11 and
  three interfaces now say what the designs say: the main-tier list covers a package's feature tests whole or
  by binary; the merge selection has declared edges, and its accepted risk includes every milestone's Ready
  for Review; `script-caller` takes a done-when item's check as naming a script, so a proof tool is tracked.
  The test tiers in the project settings carry the same risk text.
- 2026-11-20 `GATE-TIME` revised a fifth time, for the owner's words with M1 landed and a merge's check at 21 to
  24 minutes: "A 20min merge check still sounds crazy. Tests are meant to be fast." The goal is about five
  minutes for a typical landing, and a merge-tier test takes about ten seconds or less; both are assumptions for
  the owner. A measurement put 20 of the 23 minutes in test binaries run one after another. New milestones: M8
  (tests side by side through cargo-nextest, a measured test build, a limit on one test) and M9 (tests over ten
  seconds made fast or moved to the main tier). Seam ruling 2 changed, ruling 12 and one interface were added,
  and the test tiers in the project settings say the same. No other module's dependency changed.
- 2026-11-21 `GATE-TIME` M8 gained one fix in the engine, by the lead's decision: the wait in
  `FrameHost::capture_pixels`, in `APPLE-TILE`'s frame host, which failed when another thread's poll took its
  readback's completion and stopped M8's merge. Seam ruling 12 and track 6's table say so, and `APPLE-TILE`'s
  log records the hand-over. No module's dependency changed.
- 2026-11-21 `GATE-TIME` revised for four changes the owner approved that day, "These all sound really
  reasonable": the smallest world, logic without a device, fewer test binaries, and a measurement of where a
  builder's session goes. All four are `GATE-TIME`'s. Two are rules in the project settings' test tiers and
  routes of M9 and M3; M10 (the measurement) and M11 (one test binary a package) are new. Seam ruling 13 and two
  interfaces were added. `TEST-SUPPORT`'s plan did not change, and no module's dependency or open item changed.
- 2026-11-21 `GATE-TIME` and `TEST-SUPPORT` revised for the test guard rules the owner approved that day, "Yes,
  plan it". `GATE-TIME` gained M12 (three counts that only fall, and two report lines) and M13 (the stop at ten
  seconds, with one second run alone); `TEST-SUPPORT` gained M6 (the kit's small target, and the block and
  entries on writing a test), which M12 waits for. Seam ruling 14 and two interfaces were added, and the
  project settings gained three rules for the tests a plan adds. The hourly whole run on main keeps no `gpu`
  lock. No other module's dependency or open item changed.
