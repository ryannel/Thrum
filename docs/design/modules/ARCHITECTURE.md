# Thrum module list: the road to the V1 exit, with sea and clouds

10 November 2026. This is the project plan the orchestrate loop builds from: the modules, what each owns, the build
order, the interfaces between them and a brief for each. [The build plan](../../BUILD-PLAN.md) owns the purpose and
the V1 exit. [The architecture brief](../ARCHITECTURE-BRIEF.md) owns the engine-wide responsibility map (F1–T2) and
[CONTRACTS](../CONTRACTS.md) owns shared invariants. A module here is a unit of build ownership: one builder can
work on it without touching the others, and each kind of state has exactly one module in charge of it.

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

## Two tracks

**Track 1, the V1 exit.** Unchanged. Its remaining gates are G3b, G3s, G3t, G3c, G4, G5, G6 and G7. Nothing in
track 2 gates it.

**Track 2, sea and clouds.** Two engine modules, `SEA` and `CLOUDS`, built in Thrum on the foundation track 1
lays. They are finished here, not in Broadsides; [the reasons](#where-sea-and-clouds-are-finished) are below.
Track 2 adds one foundation module, `EFFECT-GRAPH`, and asks track 1 for three things it was already going to
build. When builders start track 2 is the owner's decision; this plan only fixes the order.

## Modules and what each owns

| Module | Gate | Owns (state and code) | Status on 10 November |
|---|---|---|---|
| `PLATFORM-RUNTIME-MAC` | G3b | Qualification of the Mac surface, present and cadence path | M2 building; M3 waits on `APPLE-TILE:M3` |
| `PLATFORM-RUNTIME-IOS`, `PLATFORM-RUNTIME-WEB`, `T2-CAPTURE`, `COOKED-DRAW` | G3b, T2, G5 | iOS and web shells, engine capture, the direct cooked draw path | Done |
| `APPLE-TILE` | G3t | Recipe selection from the capability registry, the frame lifecycle and its one split scene/output graph, the pre-tone scene-linear input, the native Metal segment | M0 ready; M0–M3, M10–M12 open |
| `APPLE-PRESENTATION` | G4 | The view record (jitter, matrices), motion and reactive targets, history keys and acceptance, reconstruction, render extent and generation transitions, exposure retention, EDR present, the display record, the quality governor, pipeline preparation | Design exists; M1–M3 are now in its done-when file; M4–M11 await migration |
| `ATMOSPHERE` | G3s | The one sky, Sun, aerial perspective, night and stars; the environment identity; the direct-Sun path to receivers | M1–M2 done, M3 parked, M4 deferred; M5 ready, M6 waits on `APPLE-PRESENTATION:M3` |
| `CULLING` | G3c | Instance visibility lists, occlusion history, the mesh-bucket route and Sun-caster culling | Frustum and occlusion run on the field; plan owed |
| `CONTENT-LOOP` | G5 | Cooked products, the manifest, runtime mapping and GPU residency of cooked meshes, textures and volumes | Native loop complete; closing milestone and plan owed |
| `CREATOR` | G6 | The typed controls model and its views, parameter files, the app runner, sky kit and starter, the creator docs, profiler readouts | Mac built; M1 ready |
| `SHOWCASE` | G7 | The exit scene and route in `examples/v1-game`, the V1 exit record | Route slice built; plan owed, last in order |
| `EFFECT-GRAPH` | track 2 | Graph resource kinds and cadence that simulated effects need; how an engine module declares its own passes | Not started; plan owed |
| `SEA` | track 2 | Sea state on the GPU (spectral cascades, wake fields, foam material), the body-query ring, sea optics | [Brief](SEA-BRIEF.md); plan owed |
| `CLOUDS` | track 2 | The weather field, cloud density and lighting caches, the cloud optical-depth product, placed cloud volumes | [Brief](CLOUDS-BRIEF.md); plan owed |

State that no module above owns stays where it is: committed world state and the tick page in `thrum-core`
(G1/G2S, done), input and replay in `thrum-network` and core (done for V1), and game content (the valley, presets,
hull data, cloud genus tables, looks) in `examples/` and never in engine crates.

## Coverage of the V1 exit

| Exit clause | Closed by |
|---|---|
| One engine powers `examples/v1-game` on this M3 Max and its 120 Hz display, and the same scene in Chrome | `SHOWCASE`; `PLATFORM-RUNTIME-MAC` M2 and M3 |
| Physical sky, Sun and aerial perspective with time controls | `ATMOSPHERE` M6 |
| Cooked meshes and textures; hot reload | `CONTENT-LOOP` close; `CREATOR` |
| Live camera, lights and cascaded shadows | G3a (done); shadow quality on the route in `ATMOSPHERE` M6 |
| GPU culling | `CULLING` |
| MetalFX, EDR and a quality governor; no pipeline compilation during play | `APPLE-TILE` M10–M12; `APPLE-PRESENTATION` M4–M11 |
| Display-link interpolation | G3b (function done); Mac cadence in `PLATFORM-RUNTIME-MAC` |
| Profiler and native/browser demo UI | `CREATOR` M1 and M2 |
| 10,000 entities and 1,000–2,000 movers as the representative scene | `SHOWCASE`, with `CULLING` and `CONTENT-LOOP` |
| Headless corrected-run replay | G2S (done) |
| A fresh agent builds a small scene from public docs alone | `CREATOR` M3 |
| Outstanding physical-device runs reported | `SHOWCASE` |

## Build order

Riskiest and most depended-on first. A milestone's `needs` in its done-when file is the authority; this is the
reasoning behind those entries. The profile allows one Codex builder, so the lanes below say what is safe to run
beside `APPLE-TILE` if the owner allows a second builder, and what a lone builder picks up while `APPLE-TILE` or the
Mac qualification waits on an attended session.

1. **In flight.** `PLATFORM-RUNTIME-MAC` M2. `APPLE-TILE` M0, then M1–M3. `APPLE-TILE` is first because everything
   after it reads its graph, its recipe selection or its pre-tone input.
2. **Ready now, in other files.** `CREATOR` M1 and M2 (`thrum-app`, `thrum-controls`, `examples/starter`, docs).
   `ATMOSPHERE` M5 (`thrum-render/src/atmosphere`, the forward light group, the contribution seam).
3. **After `APPLE-TILE:M0`.** `APPLE-PRESENTATION` M1 (jitter, matrices and motion), which runs on the current
   portable output and needs only M0's proof tools.
4. **After `APPLE-TILE:M2`.** `APPLE-PRESENTATION` M2 (temporal inputs and history; it shares `runtime.rs`).
   `EFFECT-GRAPH` and `CULLING`, which both edit the graph and scene files M2 moves.
5. **After `APPLE-TILE:M3`.** `APPLE-PRESENTATION` M3 (TAAU in the game). `PLATFORM-RUNTIME-MAC` M3.
6. **After TAAU.** `ATMOSPHERE` M6, because antialiasing changes every edge and gradient the sky review judges.
   `APPLE-PRESENTATION` M4–M7 (extent, scale, EDR, governor). `APPLE-TILE` M10–M11, then `APPLE-PRESENTATION` M8
   (MetalFX), M9 and M10. `CONTENT-LOOP` and `CREATOR` M3 close at any point here.
7. **The exit.** `APPLE-TILE` M12 and `APPLE-PRESENTATION` M11 (paired proof), then `SHOWCASE`.
8. **Track 2.** `SEA` and `CLOUDS` each start with a CPU authority and oracle milestone that needs nothing from the
   renderer. Their GPU milestones start where [the readiness conditions](#where-the-ports-readiness-conditions-are-met)
   are met. Their closing milestones need the governor.

**Plans owed before a builder reaches them**, in this order, each a `plan-module` run from its brief below:
`APPLE-PRESENTATION` M4–M11 (migration of the existing prose plan), `CULLING`, `EFFECT-GRAPH`, `CONTENT-LOOP`,
`SEA`, `CLOUDS`, `SHOWCASE`. None can start today: each waits on `APPLE-TILE`, on a milestone that itself waits on
`APPLE-TILE`, or on the owner's port timing, so no ready work is unplanned.

**Design reviews owed.** `CREATOR`, `ATMOSPHERE` (its 10 November revision) and `APPLE-PRESENTATION` are new to
the loop, so each needs `agent_loop.py review --module <M> --milestone design` to return Design Approved before
its builder starts.

## Where the port's readiness conditions are met

[The port decision](../decisions/SEA-AND-CLOUD-PORT.md#readiness-6-november) lists five conditions. Each is met at
a named milestone.

| Condition (6 November) | Met by | State |
|---|---|---|
| G3s: one shared sky, Sun, light-transmittance and environment-reflection products | `ATMOSPHERE:M5` (identity, lent products, the one cloud optical-depth path) | Ready to build |
| Graph: 3D and cube resources, cadence; async-compute eligibility as a declared seam | `EFFECT-GRAPH:M1`, after `APPLE-TILE:M2` | Plan owed |
| Frame and V4: effects supply motion, reactivity and reset epochs; reconstruction | `APPLE-PRESENTATION:M2` (third forward target, history protocol) and `M3` (TAAU) | In the done-when file, waiting on `APPLE-TILE` |
| G5: cook and map 3D-volume products; cloud volumes loaded | `CONTENT-LOOP` (density volume product, R8 portable and BC4 desktop) | Plan owed; volumes already cook and one draws |
| Recipes and governor: resource and quality selection, history-safe cadence | `APPLE-TILE:M1` (recipe selection), `APPLE-PRESENTATION:M7` (governor), `M8` for the Apple reconstructor | Planned |

So a first sea or cloud frame can be built in Thrum once `APPLE-TILE:M3`, `APPLE-PRESENTATION:M3`,
`ATMOSPHERE:M5` and `EFFECT-GRAPH:M1` are Done. That point falls at step 5–6 of the build order, before G7. The
ports close after `APPLE-PRESENTATION:M7`. Demo scenes in Chrome also need `CREATOR:M1`.

## Interfaces

Existing contracts are named by number. New interfaces are named here and defined in the design of the module
that provides them; they are provisional until that module's design review passes. A module does not change an
interface by itself: a builder writes a proposal in its plan's log and a planner settles it here.

| Interface | Provider → consumers | Defined in |
|---|---|---|
| Tick page, committed and previous endpoints, tick-derived time | core → every simulated effect | C02 |
| Capability registry, recipes, quality values | `APPLE-TILE` (selection), `APPLE-PRESENTATION` (governor) → all GPU modules | C04, [hardware-native decision](../decisions/HARDWARE-NATIVE-ENGINE.md) |
| Graph declaration, GPU lifetime, device generations | `APPLE-TILE` → all GPU modules | C06, [graph decision](../decisions/G3-EXECUTING-RENDER-GRAPH.md) |
| **Effect passes and resources**: a module declares its own compute and render passes at named frame stages, with 3D, cube and mipped storage-array textures, persistent and wrapping history resources, and a cadence | `EFFECT-GRAPH` → `SEA`, `CLOUDS`, `ATMOSPHERE` | `EFFECT-GRAPH.md` (owed) |
| Pre-tone scene-linear input with matching exposure, extent and sample identity | `APPLE-TILE:M3` → `APPLE-PRESENTATION` | [APPLE-TILE](APPLE-TILE.md) |
| **Temporal contribution**: every drawer writes motion from unjittered positions, marks invalid history, writes the reactive target, and honours the frame's reset flag | `APPLE-PRESENTATION` M1–M2 → `ATMOSPHERE`, `CONTENT-LOOP`, `CULLING`, `SEA`, `CLOUDS` | C07, [APPLE-PRESENTATION](APPLE-PRESENTATION.md#seams-for-thrum-core-proposals) seams 1–4 |
| **Environment identity and products**: source epoch and time, the true Sun (direction, raw illuminance, radius), and read-only sky, transmittance, aerial and reflection products with their WGSL lookups | `ATMOSPHERE:M5` → forward contributors, `SEA`, `CLOUDS` | [ATMOSPHERE](ATMOSPHERE.md#environment-identity-and-products), C08 |
| **Sun-path optical depth**: one provider's `tau` toward the Sun; V3 applies `exp(-tau)` once on each direct-Sun receiver | `CLOUDS` (a diagnostic stand-in in `ATMOSPHERE:M5`) → `ATMOSPHERE` receivers, `SEA` | C08, [ENVIRONMENT](ENVIRONMENT.md), [ATMOSPHERE](ATMOSPHERE.md#environment-identity-and-products) |
| Ordered medium segments `(T,S)` and the reflection provider | `SEA`, `CLOUDS` → V3 composition in `ATMOSPHERE` and the forward pass | C08 |
| Cooked products, including density volumes | `CONTENT-LOOP` → `CLOUDS`, `SEA` (foam atlas), games | C05, [G5 decision](../decisions/G5-CONTENT-ADDRESSED-LOOP.md) |
| **Body query**: one batched asynchronous GPU query, one to two ticks late, with support and age | `SEA` → games and P3 | [E1 water decision](../decisions/E1-GPU-WATER.md), C09; `SEA.md` (owed) |
| **Quality knobs**: each cadence and step budget is a runtime value with a declared history invalidation | `SEA`, `CLOUDS`, `ATMOSPHERE` → the governor in `APPLE-PRESENTATION:M7` | C04, port decision |
| Typed controls, parameter files, the scene shell on Mac and web | `CREATOR` → `SHOWCASE`, demo scenes for `SEA` and `CLOUDS` | C11, [G6 decision](../decisions/G6-CREATOR-EXPERIENCE.md), [CREATOR](CREATOR.md) |
| Engine capture modes and evidence | `T2-CAPTURE` → every visual milestone | C12, [T2 capture](T2-CAPTURE.md) |

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

## Where sea and clouds are finished

**Decision: both are finished as Thrum modules. No further finishing happens in Broadsides.** The Broadsides
branches are frozen references at the commits the briefs pin. Reasons:

1. **Cost.** Thrum's bar is the display's full refresh. Broadsides presents the stock sea at about 21 frames a
   second at 1440p, with or without clouds, and its sea pass alone takes about 21 ms of GPU time. Its fair-weather
   cloud preset added about 289 ms a frame in a paired run on 6 November. Looks finished at those costs are
   references, not products
   ([performance rules](../../../.agents/skills/thrum-engineering/references/performance.md)). Shipped engines
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
`mac-metal` and `chrome-webgpu`: [SEA](SEA-BRIEF.md#what-finished-means), [CLOUDS](CLOUDS-BRIEF.md#what-finished-means).
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

Briefs for modules whose plan is owed. Modules with a design are linked from the table above.

### APPLE-PRESENTATION M4–M11 (G4)

Migrate the existing [prose milestones](APPLE-PRESENTATION-PLAN.md) into the done-when file as written: extent
and generation transitions, dynamic scale, EDR, the governor, MetalFX, launch-to-ready, offline metallibs and the
paired proof. Settle seams 5–9 here with the frame-runtime, platform and asset owners. `needs`: M4 and M6 on
`APPLE-TILE:M3`, M8 on `APPLE-TILE:M11`, M11 on `APPLE-TILE:M12`. The governor (M7) must accept the quality-knob
interface above, so `SEA` and `CLOUDS` can register knobs without a second governor. Research depth: none new;
restate the sizing after M3's first measurements.

### CULLING (G3c)

Close the gate on the exit scene. Built and on main: GPU frustum compaction and selectable occlusion on the
instance field, the mesh-bucket route in Direct and Frustum, and per-light Sun-caster culling
([evidence](../../evidence/render/G3C-OCCLUSION-M3A-2026-11-07.md)). Remaining: occlusion on realistic meshes with
the mesh tie witness and the widened-rectangle proof that evidence owes; the default decided from paired rows on
the exit scene, with Frustum kept unless Occlusion wins there; a live toggle; and the iOS direct fallback rows.
Open decision for its planner, with `CONTENT-LOOP`: how the exit scene's 10,000 entities draw cooked meshes. Two
visibility systems exist (instance culling here, meshlet culling in `thrum-asset-gpu`), and each entity must have
one authority for its visibility. Contracts: C06, C07, the temporal contribution interface. `needs`:
`APPLE-TILE:M2`. Completion: visibility and image equivalence against Direct on the exit route on Mac and Chrome,
costs recorded as tripwires.

### CONTENT-LOOP (G5)

Close the gate and serve track 2. The native loop, the Wasm client and both cooked draw paths are built
([evidence](../../evidence/integration/G5-CONTENT-ADDRESSED-LOOP-2026-11-06.md)). Remaining for V1: the untested
staging-fill failure path, an independent closing review of code and evidence, and current maturity rows. For
track 2: a density-volume product a module can map and sample (R8 on the portable recipe, BC4 on desktop, the
iPhone encoding recorded as owed), with placement, identity and history invalidation as explicit inputs, as the
port decision requires. Parked and staying parked: CAS garbage collection and leases, layered EAC, parameter
files as a product kind. Contracts: C05, C06. `needs`: `APPLE-PRESENTATION:M2`, which adds the third forward
target to the cooked pipelines in `draw.rs`, so the gate's closing review reads the cooked draw path once, in the
shape the exit ships.

### SHOWCASE (G7)

Assemble the exit scene and close V1. The local launch and the six-stop review route are built under
[the route decision](../decisions/G7-SHOWCASE-ROUTE.md), which lists what final G7 still owes. Remaining: the
valley holding 10,000 entities with 1,000–2,000 movers drawn as cooked meshes through GPU culling; the route on
Mac Apple tile and portable and on Chrome portable, sharing scene and controls; correctness and a clean-context
visual review; whole-frame records with visible stutter, startup and hitches investigated; the visual digest,
platform status and the list of physical-device runs owed. `needs`: the last milestone of every other track 1
module. Its attended items (the owner's walk of the route on the built-in display) go in its last milestone.

### EFFECT-GRAPH (track 2 foundation)

Let an engine module run its own simulation and presentation passes through the one executor, replacing what
Broadsides did with pass lists and target pools. Today the graph declares 2D textures with layers and mips,
buffers, and transient, imported, persistent and history scopes, and a scene contribution gets one upload pass,
one compute pass and a draw in the forward pass. Sea and clouds need: 3D and cube textures; storage-texture
arrays with mips; persistent state advanced by tick-derived time; ping-pong and world-anchored wrapping
histories; time-sliced and epoch-skipped passes whose cadence is a runtime value; compute passes ordered before
the fragment passes that read them; module passes after opaque geometry that read scene depth and a refraction
source, and before the pre-tone input; and a declared async-compute eligibility that changes nothing yet. Keep
it to what `SEA`, `CLOUDS` and `ATMOSPHERE` will use; this is not a general plugin framework. Contracts: C06,
C07, the [graph decision](../decisions/G3-EXECUTING-RENDER-GRAPH.md), which already names 3D and cube
descriptors. `needs`: `APPLE-TILE:M2`. Completion: a diagnostic module exercising each kind passes graph
validation, recovers across device replacement, costs nothing when absent, and runs on Mac and Chrome.

### SEA and CLOUDS

[SEA brief](SEA-BRIEF.md) and [CLOUDS brief](CLOUDS-BRIEF.md).

## Owner decisions

Nothing below is decided by this plan.

1. **Port timing.** [The water decision](../decisions/E1-GPU-WATER.md) says implementation follows G7, and the
   port decision says the owner schedules it. The readiness conditions are met before G7, so the owner can start
   `SEA` and `CLOUDS` builders at their CPU milestones now, at their GPU milestones at step 5–6, or after G7.
   Until the owner says, those modules have briefs and no done-when file, so no builder can start them.
2. **Licence.** Broadsides code is marked `UNLICENSED` and its cloud library carries an owner-only licence.
   Thrum's demos are meant to have public source. Lifting either into this repository needs the owner's licence
   for it first.
3. **The V1 exit stays as written.** Sea and clouds are not added to it. Adding them is the owner's call.
4. **Builders.** The profile says one Codex builder. Steps 2–4 of the build order hold work that is safe beside
   `APPLE-TILE` if the owner allows a second.
5. **Broadsides threads.** This plan asks nothing more of them. Whether they stay paused is the owner's call.
6. **Spending.** None is requested. Physical-device runs stay owed and are reported at the exit.

## Log

- 2026-11-10 First module list. Planned from the owner's request above. `CREATOR`, `ATMOSPHERE` M5–M6 and
  `APPLE-PRESENTATION` M1–M3 gained done-when files; the other modules gained briefs. Assumptions that belong to
  a planned module are in that module's plan log. The two below belong to modules without a plan yet.
- Assumption: finished sea excludes inland water, coasts and surf, and the three cues the 10 November spikes left
  open (spray release at chase distance, the attached sheet, a visibly rising gas cloud); they are parked spikes.
- Assumption: finished clouds excludes the ten-genera, squall and warm-front scope of the Broadsides game, and
  the consolidation the owner asked for is the port decision's hybrid of weather layers and placed volumes.
