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
| 13 | SHOWCASE | The V1 exit: the showcase valley on Mac and in Chrome, reviewed, with its closing record | The last milestone of modules 1 to 10 and 12 |
| 14 | SEA | An open sea from waterline to horizon, with floating bodies, wakes and foam | ATMOSPHERE M5, EFFECT-GRAPH, APPLE-TILE M3, APPLE-PRESENTATION M3 and M7, CONTENT-LOOP M1; the owner's port timing |
| 15 | CLOUDS | A weather-driven sky from clear to overcast, with placed cloud volumes that shade the world | ATMOSPHERE M5, EFFECT-GRAPH, APPLE-TILE M3, APPLE-PRESENTATION M3 and M7, CONTENT-LOOP M1; the owner's port timing |
| 16 | SIM-2D | Thousands of 2D bodies step identically on every device, and find neighbours and hits cheaply | SHOWCASE |
| 17 | TRANSPORT | Two processes exchange messages over real internet protocols, one of them a browser | SHOWCASE |
| 18 | RENDER-2D | Thousands of crisp sprites, drawn without paying for anything 3D | SHOWCASE, APPLE-TILE, CONTENT-LOOP |
| 19 | SESSION | Two to four players share one run and keep seeing the same crowd | SIM-2D, TRANSPORT |
| 20 | GAME-UI | Health bars, timers, menus and a join screen drawn in the game's own frame | RENDER-2D |
| 21 | SWARM | The swarm-survival game itself: its name, rules, characters and art | SIM-2D, RENDER-2D, GAME-UI, TRANSPORT, SESSION |

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
build. When builders start track 2 is the owner's decision; this plan only fixes the order.

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
| `SHOWCASE` | G7 | The exit scene and route in `examples/v1-game`, the V1 exit record | [Brief](#showcase-g7) |
| `EFFECT-GRAPH` | track 2 | Graph resource kinds and cadence that simulated effects need; how an engine module declares its own passes; the list of contributions a scene holds | [Design](EFFECT-GRAPH/DESIGN.md) |
| `SEA` | track 2 | Sea state on the GPU (spectral cascades, wake fields, foam material), the body-query ring, sea optics | [Brief](SEA/BRIEF.md) |
| `CLOUDS` | track 2 | The weather field, cloud density and lighting caches, the cloud optical-depth product, placed cloud volumes | [Brief](CLOUDS/BRIEF.md) |

State that no module above owns stays where it is: committed world state and the tick page in `thrum-core`
(G1 and G2S), input and replay in `thrum-network` and core, and game content (the valley, presets, hull data,
cloud genus tables, looks) in `examples/` and never in engine crates. The browser shell in `thrum_app::web` was
built before the loop and has [a design](PLATFORM-RUNTIME-WEB/DESIGN.md) and no done-when file; it joins the table
above as `PLATFORM-RUNTIME-WEB` when work on it is next planned.
[Track 3's modules](#modules-and-what-each-owns-track-3) are in their own table.

## Coverage of the V1 exit

| Exit clause | Closed by |
|---|---|
| One engine powers `examples/v1-game` on this M3 Max and its 120 Hz display, and the same scene in Chrome | `SHOWCASE`; `PLATFORM-RUNTIME-MAC` M2 and M3 |
| Physical sky, Sun and aerial perspective with time controls | `ATMOSPHERE` M6 |
| Cooked meshes and textures; hot reload | `CONTENT-LOOP` M3; `CREATOR` |
| Live camera, lights and cascaded shadows | G3a, closed before the loop; shadow quality on the route in `ATMOSPHERE` M6 |
| GPU culling | `CULLING` M3 |
| MetalFX, EDR and a quality governor; no pipeline compilation during play | `APPLE-TILE` M10–M12; `APPLE-PRESENTATION` M4–M11 |
| Display-link interpolation | G3b's function rows, closed before the loop; Mac cadence in `PLATFORM-RUNTIME-MAC` |
| Profiler and native/browser demo UI | `CREATOR` M1 and M2 |
| 10,000 entities and 1,000–2,000 movers as the representative scene | `SHOWCASE`, with `CULLING` and `CONTENT-LOOP` |
| Headless corrected-run replay | G2S, closed before the loop |
| A fresh agent builds a small scene from public docs alone | `CREATOR` M3 |
| Outstanding physical-device runs reported | `SHOWCASE` |

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
7. **The exit.** `APPLE-TILE` M12 and `APPLE-PRESENTATION` M11 (paired proof), then `SHOWCASE`.
8. **Track 2.** `SEA` and `CLOUDS` each start with a CPU authority and oracle milestone that needs nothing from the
   renderer. Their GPU milestones start where [the readiness conditions](#where-the-ports-readiness-conditions-are-met)
   are met. Their closing milestones need the governor.

**The order in which the remaining modules are planned**, each a `plan-module` run from its brief below:
`APPLE-PRESENTATION` M4–M11 (migration of [the drafted milestones](../evidence/APPLE-PRESENTATION/M4-M11-milestone-drafts.md)),
`CONTENT-LOOP`, `SEA`, `CLOUDS`, `SHOWCASE`. Each waits on `APPLE-TILE`, on a milestone
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
| **Sun-path optical depth**: one provider's `tau` toward the Sun; V3 applies `exp(-tau)` once on each direct-Sun receiver | `CLOUDS` (a diagnostic stand-in in `ATMOSPHERE:M5`) → `ATMOSPHERE` receivers, `SEA` | C08, [ENVIRONMENT](../design/modules/ENVIRONMENT.md), [ATMOSPHERE](ATMOSPHERE/DESIGN.md#environment-identity-and-products) |
| Ordered medium segments `(T,S)` and the reflection provider | `SEA`, `CLOUDS` → V3 composition in `ATMOSPHERE` and the forward pass | C08 |
| Cooked products and placements | `CONTENT-LOOP` → games, `CREATOR`'s sky kit | C05, [G5 decision](../design/decisions/G5-CONTENT-ADDRESSED-LOOP.md) |
| **Content publication**: one cooked generation changes through every reader a scene has (the placement set, each lease, each cooked mesh row) in one submitted frame. The scene's residency owns it: each reader prepares under its own bound without showing, the publication is ready when the slowest reader is, a ready publication is not superseded, and one a reader cannot take is refused for all. This is C05's qualified visual preview for cooked products; C05 is unchanged | `CONTENT-LOOP:M1` (placements and leases), `CONTENT-LOOP:M2` (rows) → `CULLING`'s route, lease consumers (`SEA`, `CLOUDS`), games | C05, [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) |
| **Leases**: an engine module names a set of cooked textures and volumes; the scene's one residency uploads them and lends read-only views, as a whole set, only after the frame that recorded the last copy is submitted, in the frame the content publication switches placements and rows. Each product carries identity and a change serial; a consumer keeps a change pending until a submitted frame that advances history has used it. Placement and meaning are the consumer's inputs | `CONTENT-LOOP:M1` → `CLOUDS` (density volumes), `SEA` (foam texture); `RENDER-2D` and `GAME-UI` later | [CONTENT-LOOP](CONTENT-LOOP/DESIGN.md#leases-m1) |
| **Body query**: one batched asynchronous GPU query, one to two ticks late, with support and age | `SEA` → games and P3 | [E1 water decision](../design/decisions/E1-GPU-WATER.md), C09; `SEA`'s design |
| **Quality knobs**: each cadence and step budget is a runtime value with a declared history invalidation | `SEA`, `CLOUDS`, `ATMOSPHERE` → the governor in `APPLE-PRESENTATION:M7` | C04, port decision |
| Typed controls, parameter files, the scene shell on Mac and web | `CREATOR` → `SHOWCASE`, demo scenes for `SEA` and `CLOUDS` | C11, [G6 decision](../design/decisions/G6-CREATOR-EXPERIENCE.md), [CREATOR](CREATOR/DESIGN.md) |
| **Frame timing feed**: completed frame samples with pass-name GPU timing, and one timing-demand switch on the frame host (on by default; off writes no timestamps and the sample says GPU time was not requested) | The frame runtime (`APPLE-TILE:M3` for pass names; the switch by the ruling below) → `CREATOR`'s profiler, frames reports, Mac qualification, capture | C12, [CREATOR](CREATOR/DESIGN.md#the-frame-timing-feed) |
| Engine capture modes and evidence | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE/DESIGN.md) |
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
M2, `APPLE-TILE` M3) need no new item: the gate covers their landings once it exists. `APPLE-PRESENTATION` M3 puts
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
restate the sizing after M3's first measurements.

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

Assemble the exit scene and close V1. The local launch and the six-stop review route are built under [the route decision](../design/decisions/G7-SHOWCASE-ROUTE.md), which lists what final G7 still owes; [the route slice's record](../evidence/SHOWCASE/ROUTE-SLICE-RECORD.md) holds its four finished milestones and their log. Remaining: the
valley holding 10,000 entities with 1,000–2,000 movers drawn as cooked meshes through GPU culling; the route on
Mac Apple tile and portable and on Chrome portable, sharing scene and controls; `CULLING`'s equivalence and
paired-cost commands rerun on the assembled scene, with its default mode set by the same rule; correctness and a
clean-context
visual review; whole-frame records with visible stutter, startup and hitches investigated; the visual digest,
platform status and the list of physical-device runs owed. `needs`: the last milestone of every other track 1
module. Its attended items (the owner's walk of the route on the built-in display) go in its last milestone.

### EFFECT-GRAPH (track 2 foundation)

Planned on 11 November 2026: [design](EFFECT-GRAPH/DESIGN.md), [plan](EFFECT-GRAPH/PLAN.md). The first sea or
cloud frame needs its last milestone, M3.

### SEA and CLOUDS

[SEA brief](SEA/BRIEF.md) and [CLOUDS brief](CLOUDS/BRIEF.md).

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
- *What keeps a miss from spreading.* The digest catches a divergence within the interval `SESSION` sets, and
  repair is the reliable full image A5 already selects. A client that changes its rules only desyncs itself.
- *Range.* 2 to 4 peers and the brief's thousands. If a full-world replay does not fit a frame at that size, the
  crowd is shown at the confirmed tick and only the player's own character is predicted, which needs no crowd
  replay. `SESSION`'s planner chooses between the two from `SIM-2D`'s measured costs.
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
| `SIM-2D` | W1 and P3, for crowds | The deterministic rules profile and the check that enforces it; the digest of committed state; the 2D spatial index and its overlap, nearest and neighbour queries | `thrum-core` |
| `RENDER-2D` | V2 and V3, for 2D | The 2D scene half and its passes; the 2D view record (orthographic camera, world units to pixels); sprite rows extracted from the tick page; draw order; sprite materials and blending; GPU residency of sprite sheets | `thrum-render` |
| `GAME-UI` | U1 | Text and interface drawn in the frame: the glyph cache, the interface tree and its layout, focus, the paint list; routing of key, pointer and touch events to the interface before the game | A new crate |
| `TRANSPORT` | N1 adapters, F1 network I/O | Connections and their state: native QUIC, the browser's WebTransport, the in-memory conditioned link, a host's TLS identity. It holds no game state and gives no meaning to a message | A new crate, so a game that is not networked links no sockets |
| `SESSION` | N1 | The match and each connection's epoch; the handshake and its manifests; clock sync and input lead; input staging and relay; baseline and join; digest exchange and repair; the prediction branch; retained results | `thrum-network` |
| `SWARM` | Game policy and content | The game: its name, premise, characters, art, rules, waves, upgrades and screens; the schemas and rule set it declares to `SESSION`; its sprite and font sources; the record that closes track 3 | A new folder in `examples/`, never an engine crate |

State no module above owns stays where it is: the tick, committed world state, rewind and the tick page in
`thrum-core` (G1 and G2S); cooked products in `CONTENT-LOOP`; the graph, the split and the output half in
`APPLE-TILE` and `APPLE-PRESENTATION`; the demo panels and the app runner in `CREATOR`.

### What track 3 asks of existing modules

Each ask is new work inside state that an existing module owns. It becomes a milestone of that module, numbered
after its V1 milestones and added by that module's planner when track 3 starts. None is added now, so no V1 plan
changes.

| Owner | Ask | For |
|---|---|---|
| `CONTENT-LOOP` | A sprite-sheet product (pages, frames, pivots) and a font product, cooked, mapped and hot-reloaded like meshes and textures | `RENDER-2D`, `GAME-UI` |
| `APPLE-TILE` | A profile selects the 2D scene half in place of the 3D one. The output half gains an interface stage at output extent, after reconstruction | `RENDER-2D`, `GAME-UI` |
| `APPLE-PRESENTATION` | The governor and reconstruction accept a scene that declares no temporal reconstruction, so sprites stay crisp | `RENDER-2D` |
| `CREATOR`, with `PLATFORM-RUNTIME-IOS` and `PLATFORM-RUNTIME-WEB` | An app runner on all three targets for a game that brings no sky kit. The iOS shell lives in `examples/v1-game-ios` today and serves one game | `SWARM` |
| Repository guard | `.config/architecture.toml` names one external game; it must admit a second | `SWARM` |

If `EFFECT-GRAPH` is Done when `RENDER-2D` is planned, the 2D scene half declares its passes through that seam.
A second way to declare passes is not built.

<a id="interfaces-track-3"></a>

### Interfaces (track 3)

New interfaces are defined in the design of the module that provides them and are provisional until its design
review passes. Changes to a numbered contract are settled with its producers and consumers, as the brief requires.

| Interface | Provider → consumers | Defined in |
|---|---|---|
| Tick page, committed and previous endpoints, rewind and replay | core → `SIM-2D`, `RENDER-2D`, `SESSION` | C02, [G2 decision](../design/decisions/G2-GENERIC-SIMULATION.md); unchanged |
| **Deterministic rules profile**: what a rule may compute with and in what order, so one input trace gives one result on every target | `SIM-2D` → `SWARM` (its rules), `SESSION` | `SIM-2D`'s design; C02 |
| **World digest**: one value for each committed tick over all authority state | `SIM-2D` → `SESSION` | `SIM-2D`'s design |
| **2D queries**: overlap, nearest and neighbours, with results in a fixed order | `SIM-2D` → `SWARM` | `SIM-2D`'s design; C03 |
| **Sprite rows**: a game's read-model components map to sheet frame, layer, tint and transform, interpolated between tick endpoints on the GPU | `RENDER-2D` → `SWARM`, any 2D game | `RENDER-2D`'s design; C07 |
| **2D scene half**: passes declared through the one graph, writing the pre-tone scene-linear input | `APPLE-TILE` → `RENDER-2D` | [APPLE-TILE](APPLE-TILE/DESIGN.md), C06; `RENDER-2D`'s design |
| Sprite-sheet and font products | `CONTENT-LOOP` → `RENDER-2D`, `GAME-UI`, games | C05 |
| **Interface stage**: a paint list drawn at output extent after reconstruction | `APPLE-TILE` and `APPLE-PRESENTATION` → `GAME-UI` | C06, C07; `GAME-UI`'s design |
| **Event routing**: the interface consumes an event or passes it on as a game action | shells → `GAME-UI` → the game | C11 |
| **Message link**: a reliable ordered stream, datagrams with a stated size limit, backpressure and lifecycle events, the same on every adapter | `TRANSPORT` → `SESSION` | `TRANSPORT`'s design; [NETWORKING](../design/modules/NETWORKING.md#transport-and-overload), A0 |
| **Session**: join and leave, the admitted input frames of every player for each tick, the baseline, the confirmed tick, the prediction branch, and connection state a screen can show | `SESSION` → `SWARM` | `SESSION`'s design; C13, A0 and the A5 decisions |
| **Rule set in the gameplay manifest**: a match names its deterministic rule set and build, and refuses a peer that differs. A proposed addition to C13 | `SESSION`, with `SIM-2D`, core and the game | `SESSION`'s design |
| Quality knobs | `RENDER-2D` → the governor | C04; the interface above, unchanged |
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
2. **Sprites.** `RENDER-2D`, after `APPLE-TILE`'s and `CONTENT-LOOP`'s asks. It can run beside step 1.
3. **A match without pictures.** `SESSION` on the in-memory conditioned link, after `SIM-2D`'s first milestone.
   Its socket milestones follow `TRANSPORT`'s first milestone.
4. **Interface.** `GAME-UI`, after `RENDER-2D`'s first milestone and the interface-stage ask.
5. **The game.** `SWARM` opens with the owner's choice of concept, which only `SWARM` waits on. Then a
   single-player slice after `SIM-2D` and `RENDER-2D`, the full run after `GAME-UI`, co-op after `SESSION` on
   sockets, and the close.

A lone builder takes them in that order: `SIM-2D`, `TRANSPORT`, `RENDER-2D`, `SESSION`, `GAME-UI`, `SWARM`.
`SWARM`'s options milestone is built first of all, so the owner's answer is in before any art is made.

**Reaching the targets.** `TRANSPORT`, `SESSION` and `SWARM` open local sockets and drive Chrome, which Codex's
`workspace-write` sandbox does not allow without a request for each run. Their planners settle how a builder and a
reviewer reach those targets; [the choice](#owner-decisions) is the owner's.

### What finished means (track 3)

Proposed, for the owner to change: two to four players, on any mix of Mac, Chrome and the iOS simulator, join one
host and play a whole run of an original swarm survivor together, with thousands of enemies and projectiles on
screen, smooth at the display's full refresh on the Mac, and the peers still agree under the A0 conditioner. The
engine modules hold no game types, and a second small 2D scene builds on them without `SWARM`.

### Module briefs (track 3)

A `plan-module` run turns each brief into a design, a plan and a done-when file. Costs are recorded as tripwires;
no budget below is a gate. The quality bar is a shipped game, named for each milestone by the planner.

#### SIM-2D

- **For.** A game runs a crowd of thousands whose every tick gives the same bytes on every target, and finds
  neighbours and hits cheaply.
- **Requirement.** "Thousands of enemies and projectiles on screen at once" and "Several players play together
  over the network".
- **Built already.** One atomic tick in canonical slot order, sparse undo and replay, and equality of serial and
  forced-parallel runs (G2S). The G2 decision records 61 µs for tick and page and 81 µs for an eight-tick rewind
  at 10,000 simple movers.
- **To build.** The rules profile and a check that fails a rule which breaks it. The digest. A uniform-grid index
  with overlap, nearest and neighbour queries in a fixed order. A reference crowd (chase, separation, projectile
  hits, spawn and despawn churn) that later modules reuse as their diagnostic scene.
- **Contracts.** C01, C02, C03; the rule-set addition to C13 with `SESSION`.
- **Open for its planner.** Fixed point or restricted floats, chosen by the three-target proof. Whether the
  queries sit in `thrum-core` or beside it.
- **First proof and what rejects it.** Equal digests on the three targets for a long trace with churn. If a
  difference cannot be removed inside the profile, mechanism 1 falls back and `SESSION` is planned on corrections.
- **Research.** Cross-platform determinism in shipped engines (Rapier's deterministic build, Photon Quantum's
  fixed point, Box2D 3); uniform grids and separation for crowds.
- **Done.** Equal digests across the targets and across serial and parallel runs; queries match a brute-force
  oracle; tick, rewind and replay costs at 2,000, 5,000 and 10,000 bodies are on record.

#### RENDER-2D

- **For.** A game draws thousands of sprites that read clearly and move smoothly, and pays for nothing 3D.
- **Cues.** A crowd at the brief's size at full refresh. Sprites crisp at every window size and pixel density,
  with no shimmer when the camera scrolls. Overlap order that never flickers. Motion smooth between ticks. Frame
  animation, hit flash and tint, and blended effects.
- **Built already.** The instance field's changed-row upload and GPU interpolation between tick endpoints, the
  graph, cooked textures.
- **To build.** The 2D scene half and view record, sprite rows, instanced drawing from sheet pages so a crowd is a
  few draws, a stable draw order, sheet residency.
- **Contracts.** C04, C05, C06, C07, C12. `needs`: `APPLE-TILE:M3`, and the `APPLE-TILE` and `CONTENT-LOOP` asks.
- **Open for its planner.** How order is resolved (sorted rows or depth). Reconstruction for sprites, with none as
  the default. Whether bright effects use EDR. How text shares the sprite path with `GAME-UI`.
- **Not in scope.** 2D lights, tile-map tools, skeletal 2D animation, sprites in a 3D scene.
- **Bar.** Crowd legibility as in Vampire Survivors and Halls of Torment; pixel stability as in Celeste.
- **Done.** `SIM-2D`'s reference crowd drawn on `mac-metal` and `chrome-webgpu`, and working on `ios-sim`; the
  frame record shows no sky, cascade or cluster work; a second small 2D scene builds on it.

#### GAME-UI

- **For.** A game shows a status display and menus in its own frame on all three targets.
- **Cues.** A health bar, timer and counters readable at every window size, at 1× and 2× density and inside the
  iOS safe area. A modal choice of three or four upgrades worked by key, pointer and touch. A join screen. An event
  the interface consumes never moves the player.
- **Built already.** [The U1 design](../design/modules/AUDIO-UI.md#u1-operating-model), which is wider than this game. Typed demo
  controls in `thrum-controls`, which stay for demos and the profiler and are not replaced.
- **To build.** The smallest slice of U1 that this game uses: Latin text from one font product, a retained tree
  with simple layout, focus, the paint list, event routing under C11.
- **Contracts.** C06, C07, C11. `needs`: `RENDER-2D`'s first milestone, the interface-stage and font-product asks.
- **Open for its planner.** The text stack, after a licence and Wasm audit of current Rust crates. How a join
  address or code is entered on each target without a full text-input stack.
- **Not in scope, and recorded as unqualified.** Complex scripts, localisation, IME, accessibility bridges.
- **Done.** The status display and modal on the three targets; focus and tree survive device loss; text stays
  crisp at both densities.

#### TRANSPORT

- **For.** Two processes exchange messages over the internet's real protocols, and one of them may be a browser.
- **Requirement.** "Several players play together over the network", on the brief's three targets.
- **Selected already.** Native QUIC through Quinn and WebTransport in the browser, datagrams and one reliable
  ordered stream (A0). WebTransport became Baseline in March 2026
  ([web features](https://web-platform-dx.github.io/web-features-explorer/features/webtransport/)); only Chrome
  is a target here.
- **To build.** The message link on three adapters: Quinn (Mac, the host and the iOS simulator), WebTransport in
  Wasm, and the in-memory conditioned link the A5 fixtures use. A host identity that Chrome accepts on a local
  machine. Counters for bytes, loss and queue age.
- **Contracts.** C11 (a suspended app has lost its socket), C12, C13, [NETWORKING](../design/modules/NETWORKING.md#transport-and-overload).
- **Open for its planner.** The WebTransport crate, after an audit of licence, datagram support, the Wasm client
  and upkeep (`wtransport`, `web-transport-quinn`, `webtrans`). The async runtime Quinn brings and where its
  thread sits beside the world lane. Whether the WebSocket fallback of A0 is built; the default is no.
- **First proof and what rejects it.** The three clients hold a session with one host at 100 ms round trip, 20 ms
  jitter and 2% loss, through disconnect and reconnect. If Chrome cannot hold datagrams to a local host, input
  moves to the reliable stream for that adapter and the cost is recorded.
- **Done.** The proof above; malformed input fuzzed; a game without networking links no transport code.

#### SESSION

- **For.** Two to four players share one run and keep seeing the same crowd.
- **Built already.** Envelopes, input batches, future-input staging, state deltas and acknowledgements in
  `thrum-network`; a four-peer, 600-tick run with no sockets; controller replay; core rewind.
- **To build.** The handshake with protocol, gameplay and rule-set identity. Clock sync and an input lead sized
  from measured round trip and jitter. Relay of admitted frames. The baseline for join and reconnect. Digest
  exchange and repair. Retained results, which A5 left open. The session loop a headless host runs.
- **Contracts.** C02, C03, C13, A0 and the A5 decisions. `needs`: `SIM-2D`'s first milestone; `TRANSPORT`'s first
  milestone for the socket milestones.
- **Open for its planner.** Whole-world rollback or a confirmed-tick crowd with a predicted player, from
  `SIM-2D`'s costs and how each feels. How often digests travel. How a modal choice in co-op reaches every peer in
  order: Vampire Survivors saw level-up screens arrive out of order after dropped packets and moved them to ordered
  commands.
- **Proof order.** Four peers on the conditioned in-memory link with the reference crowd: equal digests under
  loss, reorder and duplication; a forced divergence repaired; a join in progress; a reconnect; a slow peer that
  does not stall the host. Then the same on sockets across the three targets.
- **Costs to record.** Bytes for each peer, replay steps, correction distance, baseline size and join time. A0's
  proposed 64 kbit/s is a tripwire.
- **Done.** The proof order passes on sockets, and a game that is not networked pays nothing for it.

#### SWARM

- **For.** The owner's game: an original swarm survivor that proves the five modules above in a product.
- **Requirement.** "One or more players hold out against ever-growing crowds of enemies, with automatic attacks
  and upgrades between waves. It's an original game: its own name, characters, art and rules."
- **Owns the taste.** Its first milestone lays out a few concepts that differ in kind (a name, a premise, one
  hero, two enemies and a palette on one mock arena) and the owner picks one. The quality bar starts from the
  chosen concept.
- **To build.** A single-player slice with the crowd at the brief's size. The run: waves, pick-ups, upgrades,
  winning and losing, the status display and menus. Co-op for 2 to 4 with its own rules (shared or separate
  upgrades, what a fallen player does, whether a choice pauses the run). The three targets.
- **Contracts.** Every interface in the table above; C13 for what the game declares.
- **Art.** Made in this repository, original or under the project's PD and CC0 rules. A paid generation service is
  spending.
- **Bar.** Vampire Survivors, Brotato, Halls of Torment and Deep Rock Galactic: Survivor.
- **Done.** [What finished means](#what-finished-means-track-3). The owner's judgement of how it plays is an
  attended item in the last milestone.

## Owner decisions

Nothing below is decided by this plan.

1. **Port timing.** [The water decision](../design/decisions/E1-GPU-WATER.md) says implementation follows G7, and the
   port decision says the owner schedules it. The readiness conditions are met before G7, so the owner can start
   `SEA` and `CLOUDS` builders at their CPU milestones now, at their GPU milestones at step 5–6, or after G7.
   Until the owner says, those modules are planned no further than their briefs.
2. **Licence.** Broadsides code is marked `UNLICENSED` and its cloud library carries an owner-only licence.
   Thrum's demos are meant to have public source. Lifting either into this repository needs the owner's licence
   for it first.
3. **The V1 exit stays as written.** Sea and clouds are not added to it. Adding them is the owner's call.
4. **Builders.** The profile says one Codex builder. Steps 2–4 of the build order hold work that is safe beside
   `APPLE-TILE` if the owner allows a second.
5. **Broadsides threads.** This plan asks nothing more of them. Whether they stay paused is the owner's call.
6. **Spending.** None is requested. Physical-device runs stay owed and are reported at the exit.

For [track 3](#track-3-the-swarm-survival-game-after-v1):

7. **When track 3 starts.** The brief says after V1, and this plan holds every track 3 module until `SHOWCASE` is
   Done. Whether the game goes before, after or beside sea and clouds is the owner's order to set.
8. **The game's identity.** Its name, premise, characters and look are the owner's taste. `SWARM`'s first
   milestone lays out options and waits for the owner's pick.
9. **Art and licence.** The plan assumes art made in this repository, original or PD and CC0. A paid generation
   service is spending. The licence the game and its art are published under is the owner's.
10. **A public host.** Play across the internet needs a machine others can reach, a domain and a certificate,
    which is spending and deploying. Until the owner asks for one, "online" is proved between separate processes
    on this Mac over real QUIC and WebTransport under the A0 conditioner. Putting the game on the website is
    publishing.
11. **Sockets and Chrome under the Codex sandbox.** `TRANSPORT`, `SESSION` and `SWARM` need both for most checks.
    The choices are a request for each run as today, `Codex sandbox: danger-full-access`, or a Claude builder for
    those modules.
12. **Sound.** The brief does not ask for it and no audio module is planned. A1 stays a design.
13. **The purpose line.** The brief still says "3D engine" and requirement E08 of the architecture brief
    still puts pure 2D outside scope. The 10 November goal overrides both; rewording them is the owner's page.
14. **What finishes the goal.** [The proposal](#what-finished-means-track-3) stands in for an exit the owner has
    not written.
15. **Builders.** Steps 1 and 2 of track 3's order hold three lanes that share no files.

## Assumptions for modules not yet planned

Each stands until the owner overturns it, and moves to its module's plan, under `Open for the owner`, when that module is planned.

- Assumption: finished sea excludes inland water, coasts and surf, and the three cues the 10 November spikes left
  open (spray release at chase distance, the attached sheet, a visibly rising gas cloud); they are parked spikes.
- Assumption: finished clouds excludes the ten-genera, squall and warm-front scope of the Broadsides game, and
  the consolidation the owner asked for is the port decision's hybrid of weather layers and placed volumes.
- Assumption: "2D support in the engine" means a reusable sprite path (sprites, layers, a 2D camera, text and
  interface) that a second 2D game can build on. 2D lights, tile-map tools, skeletal 2D animation and 2D rigid
  bodies are not built.
- Assumption: "over the network" is proved by separate processes on this Mac over real QUIC and WebTransport under
  the A0 conditioner. A public host is the owner's to order.
- Assumption: a native process hosts; Chrome and the iOS simulator only join. Players join by address or code.
  Matchmaking, accounts, relays and host migration are not built.
- Assumption: Chrome is the only browser, so the WebSocket fallback of A0 is not built.
- Assumption: every peer simulates the one arena, so the interest management the build plan lists as "relevance"
  is not built for this game.
- Assumption: no sound, and no gamepad: keys, pointer and touch only.
- Assumption: the iOS simulator proves function for the game, never speed, as in V1.

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
