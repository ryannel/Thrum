# Scene views: the scene drawn again from another camera

14 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md) · [the first consumer, `WATERS`](../WATERS/DESIGN.md#mirror-views-scenery-the-camera-cannot-see).
C04, C06, C07, C08 and C12 in [CONTRACTS](../../design/CONTRACTS.md) and
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) govern.

## The request

The owner, 14 November 2026, of water:

> Scenery reflections we must have, that's a main stay in high quality modern games. That's the standard we're
> after.

`WATERS` settled how water reflects scenery the camera cannot see: it reads a second draw of the scene, made
from the camera's mirror image in the water's plane. A second draw needs the scene's culling, lights, shadows
and air for another view, which no effect may reach, so [the module list](../ARCHITECTURE.md#scene-views) made
it a module of its own. The lead's task for this plan: "plan the new module SCENE-VIEWS that WATERS M14 depends
on. Choose methods from first principles and current practice."

## Purpose and scope

A **secondary view** is the scene's opaque geometry drawn in the same frame from a camera another module names,
into a texture that module reads while it shades the main view. It is a product of the frame, as a Sun cascade
is. It is never presented, and it has no history.

| User | Needs | What that requires of the design |
|---|---|---|
| `WATERS` M14, for a lake, a pond and the sea under an island | The far shore, hills, trees and the craft as a camera at the mirror image of the main one would see them, with the light the main view gives them, up to two a frame | The same shading, with what belongs to a view made again for the view; a cut at a plane; the air of the reflected path |
| `PUDDLES`, `RIVERS`, through `WATERS` | The same, with no work of their own | Nothing beyond the interface |
| A game that has no such water | Nothing | A scene that declares no view is what it was, bit for bit |
| A later consumer: a probe, a portal, a monitor in the world | A view from any camera | The request is a camera, and nothing in it says "mirror" |

**Commissioned:** secondary views on `mac-metal`, on both recipes, and `chrome-webgpu`, of everything a scene
draws as opaque geometry: its entities, on both of `CULLING`'s routes, and its cooked placements.

**Not built**, with the reason:

- The sky, stars, water, clouds and any other after-opaque effect in a view. A consumer composes its own
  background, and `WATERS` takes the sky and its clouds from the environment product.
- Fog in a view. A scene with `ATMOSPHERE`'s view volume draws its views in clear air
  ([open for the human](PLAN.md#open-for-the-human)).
- A history for a view, so motion vectors, reactive coverage and reconstruction of a view. A view is read
  inside the main view's shading, and the main view's reconstruction is the only one.
- Occlusion culling in a view. It would need a depth pyramid and visible-last bits for each view.
- Levels of detail. The engine has none to choose from; a view drops what is small in it instead.
- Cube faces, a view of a view, and views that outlive a frame. No consumer asks for them.
- Ray-traced reflection. [Judged below](#research-and-selection) and not selected.
- The iOS simulator. It is not a target of `WATERS`.

**What exists.** One prepared scene draws one view. Its forward shader reads a view group (the view record,
the Sun cascades, the air's shadow heights, the visible list) and a light group (the Sun, local lights and the
cluster table built for the view, the aerial volume computed for the view, the environment products, exposure).
The Sun's cascades already draw the scene from other viewpoints, depth only, and `CULLING` already culls
casters against each Sun pass's own volume and never against the camera. Read for this design: the whole of
`contribution.rs` and `synthetic/forward.rs`; the bindings, `sun_visibility` and `local_radiance` in
`synthetic.wgsl`; the heads of `synthetic/{view,shadow,cluster,air_shadow}.rs`; the aerial pass in
`atmosphere/graph.rs`; and the designs of `CULLING`, `EFFECT-GRAPH`, `ATMOSPHERE` and `WATERS`.

## Cues

The bar is a planar reflection in a shipped game, as Unreal's planar reflections and GTA V's water pass draw
one: the same world, lit the same, from the other side. In words, on both targets.

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **V1** The same scene in the same light: a thing in a view looks as the main view would show it from there, Sun-lit and shaded sides, shadows, lamps, the haze of its own distance | Any rigid camera and field of view; an extent of a sixteenth of the main view's pixels to all of them; dawn to night; entities and cooked placements; every culling mode | A capture lit by ambient light alone, or one that reuses the main view's cluster table, aerial volume or cascades. Rejected by the view against the main view drawn from the same camera, and by a lamp and a hill that stand outside the main frustum |
| **V2** What the main camera cannot see is there | Things behind the camera, above the frame and behind a nearer hill; 1 to 100,513 instances | The main view's visible list drawn again. Rejected by a view that faces the other way |
| **V3** A view does not depend on where the main camera looks | The request held while the main camera turns, tilts and moves | Cascades, clusters or visibility fitted to the main frustum. Rejected by the view's bytes with the main camera turned through 180° |
| **V4** A clean cut at a plane | A plane at any height; things that cross it; a camera 5 cm to 40 m from it | A clip the consumer makes afterwards from depth, which leaves the ground between the camera and the plane in front of everything. Rejected by a post standing through the plane and a view camera under the ground |
| **V5** Absent means absent | A scene that declares no view; a declared view nobody asks for in a frame | Targets that are filled and not read. Rejected by identical shader text, passes, pipelines, resources and frames, and by pass names missing from the frame's timing |
| **V6** Steady, and of this frame | Pan, tilt, a time scrub, a mover crossing; with and without temporal sampling | A view drawn a frame late, or one with its own jitter and history. Rejected by a reflection that must meet its source in every frame, and by an edge's place under the frame's jitter |
| **V7** A view's cost follows what it shows | No view, one and two; each knob at both ends | A second full scene at any setting. Rejected by the cost record beside the main view's passes |

## Research and selection

- **Unreal's planar reflections** ([manual](https://dev.epicgames.com/documentation/en-us/unreal-engine/planar-reflections-in-unreal-engine),
  as fetched for `WATERS`' design and again by search for this one). The level is drawn again from the
  reflected direction; cost follows the scene's complexity and not the capture's resolution; it is cut down by
  a lower resolution, a shorter distance and coarser detail. Its clip plane is a project-wide shader feature:
  [Epic's forum answer](https://forums.unrealengine.com/t/missing-vital-scenecapturereflectactor-for-real-time-planar-reflections/11287?page=2)
  (search summary) puts it at about 15% of the base pass whether or not a reflection is drawn, because it is
  compiled into every material. Taken: a clip test must live only in the view's own pipelines.
- **GTA V** ([Courrèges's frame study](https://www.adriancourreges.com/blog/2015/11/02/gta-v-graphics-study/),
  as summarised for `WATERS`' design). The scene is drawn again in about 650 draw calls into a 240×120 texture
  for the water. Evidence that a shipped reflection is small and draws less than the main view.
- **This renderer's Sun passes** (code read, listed above). A cascade is a second draw with its own matrix,
  its own culled caster list and its own depth layer, and a contribution joins it through `shadow_bounds` and
  `draw_shadow`. A secondary view is the same shape with colour.
- **Ray query** ([wgpu's ray tracing notes](https://wgpu.rs/doc/wgpu/documentation/extensions/ray_tracing/index.html)
  and the W3C group's [2026 activity](https://www.w3.org/groups/wg/gpu/activity/2026/), by search). In wgpu,
  acceleration structures and ray queries sit behind `EXPERIMENTAL_RAY_QUERY`, "subject to change", with ray
  tracing pipelines still in development. For the web they are two proposals, gpuweb 11054 and 11056, and not
  specification text.
- **Not read:** Unreal's source, a frame study of Red Dead Redemption 2, and Kooima's generalised projection
  beyond its result. A search for how shipped engines jitter a reflection capture found nothing, so
  [part A](../../evidence/SCENE-VIEWS/design-proof/README.md) measures it.

| Question | Candidate | Verdict |
|---|---|---|
| What makes the image | Ray query against the scene's geometry | Not selected. It cannot be the portable method, since Chrome has no ray query, so the raster view has to exist whatever else does. On the Mac it would add acceleration structures for every mesh and mover and a second shading path for hit points, behind an experimental feature. It stays a provider `WATERS` could put in its reflection slot later, promoted by a measured win |
| | A cube probe | `WATERS` rejected it: right at one point only |
| | **A second forward draw of the scene** | **Selected** |
| How one frame carries more than one view | A second prepared scene, extracted and published again | Not selected: instances, meshes, materials, lights and the environment would be uploaded twice, and two scenes' histories would have to agree |
| | **Slots declared when the scene is prepared, asked for frame by frame, drawn inside the one frame graph** | **Selected.** The graph, pipelines and targets are fixed at preparation as every other pass's are, and an idle slot is a runtime skip |
| What a view culls with | The main view's visible list | Fails V2 |
| | A reduced set chosen on the CPU | Not selected: the instance field is culled on the GPU and its CPU has no bounds to choose from |
| | **Its own frustum compaction, as each Sun pass has** | **Selected**, with no occlusion test |
| The Sun's shadows in a view | The main view's cascades | Fails V3. The shader picks a cascade by the fragment's depth in its own view, and the cascades are fitted to the main frustum, so a bank's shadows would appear in the water as the bank came on screen |
| | Cascades fitted round the camera for every view | Not selected: it coarsens the main view's shadows in every scene that declares a view |
| | **Cascades of the view's own, fitted to its frustum, with the scene's height map beyond them** | **Selected** |
| The cut at the plane | An oblique near plane in the projection | Not selected. It needs no shader change and [part C](../../evidence/SCENE-VIEWS/design-proof/README.md) finds its depth still usable, but the scene's shader turns stored depth into view depth by the standard projection's formula, to pick a cascade and a cluster slice. An oblique matrix would need that rewritten for every drawer, which moves every exact baseline |
| | A hardware clip distance | A candidate for a recipe later: it is an optional feature in WebGPU, and it needs a vertex variant in every drawer |
| | **A test of each fragment against the plane, in the view's own fragment entry** | **Selected.** A view's pipelines are separate from the main view's in any case, because a view has one colour target where the main view has three |

## Mechanisms

### Slots and requests

A contribution declares, when the scene is prepared, how many secondary views it may ask for and each one's
extent as a share of the render extent, from a quarter to the whole of each side. Each is a **slot**: its
targets, per-view products, pipelines and graph passes exist from preparation, so asking never compiles
anything. A scene's slots are bounded by its profile, four in the first, and a declaration over the bound, or
over what the capability record allows, is refused at preparation.

Each frame, in its upload, the declaring contribution **asks** for a slot or leaves it idle. A request is a
camera and nothing more specific:

- a rigid, right-handed view transform and a symmetric perspective with a near and a far distance;
- optionally a **clip plane**, in world space;
- optionally a **fold height** for the air ([below](#what-a-view-owns-and-what-it-shares));
- the least size, in the view's pixels, of an instance or a meshlet group worth drawing.

An improper transform is refused. A clip plane nearer the view's camera than 5 cm is moved to 5 cm, so a
request never degenerates as a camera comes down to the water; at most that much geometry below the plane is
then admitted. An idle slot records no pass, and its targets keep nothing a consumer may read.

A mirror is not a special case. The consumer builds a proper camera at the mirror image of the main one. A
reflection matrix would turn every triangle's winding; a rigid camera does not, and the image the consumer
reads is addressed by projecting world points through the view's own matrix, so it needs no flip.

### What a view owns and what it shares

The forward shader is shared text. What it reads splits by whether the quantity depends on the view.

| Made again for each asked view | Shared with the main view |
|---|---|
| The view record and its auxiliary record | Instances and their history, the mesh arena, materials |
| The visible lists and indirect records, from a compaction against the view's frustum and clip plane | The Sun, the sky's diffuse and reflection products, the cloud factor and any other Sun-path provider |
| The Sun cascades and their uniform, fitted to the view's frustum by the scene's own fit | The caster height map and the air's shadow heights, which are laid out round the main camera in world space |
| The cluster table of local lights | The local-light list |
| The aerial volume | The frame's exposure sample and storage scale |
| Colour with coverage, and depth | |

- **Visibility.** One more compaction for each asked view, over the same instances, against the view's six
  frustum planes, its clip plane and its least size, into the view's own lists and indirect records: the
  shape of `CULLING`'s Sun-caster culling. It keeps slot order within a bucket, as Frustum does. A scene in
  Direct mode draws every instance in a view too. Occlusion is never used in a view, whatever the scene's mode.
- **Shadows.** A view's fragments pick a cascade by their depth in the view, so the view has cascades of its
  own: the scene's fit applied to the view's frustum out to the view's shadow reach, drawn with the casters
  culled for each. Beyond the reach the scene's height map answers, as it does for the main view. The first
  profile gives a view one cascade to 128 m; the count, from none to two, and the reach are knobs. With none,
  the view has hill-scale shadows only.
- **Lights.** A cluster table is built for the view's own frustum by the pass that builds the main view's.
- **Air.** The aerial volume is computed again along the view's own lines of sight. With a fold height, a
  sample below it takes the altitude of its image above it. For a mirror the fold is the mirror's plane, so
  the view's air is the air of the reflected path unfolded, and the atmosphere is never asked for an altitude
  below the camera's side of the plane. Taken as it stands, the aerosol on that part of the path would be up
  to 3.4% too dense for a camera 40 m up, and the view's camera would stand that far under the water
  ([part B](../../evidence/SCENE-VIEWS/design-proof/README.md)). A view has clear air even where the main view
  has the view volume.
- **Exposure and encoding.** A view's colour is in the scene target's own encoding for that view,
  `(T_v · L + S_v) · E`: radiance through the view's air, times the frame's exposure scale. Coverage is 1 where
  geometry drew and 0 elsewhere, in alpha, and uncovered colour is zero. Depth is the standard reversed depth
  of the view's projection, so the view's inverse matrix turns a texel back into a world point.

### The view's pass

A view is drawn by the scene's forward pipelines with a **view fragment entry**: the scene's own `shade`, one
colour output with coverage, and one test that discards a fragment below the clip plane. The test is in no
main-view pipeline, so the main view pays nothing for it. There is no sky, star or after-opaque draw in the
pass, and no motion or reactive target.

**Contributions.** A contribution that draws opaque geometry may opt in to views. It is then given each asked
view's frustum, clip plane and least size with the frame's upload, culls for it in the contribution compute
pass, and draws in the view's pass with the view's groups bound, through pipelines it builds from the view's
surface (the same shader source, the view fragment entry, the view's formats). One that does not opt in is not
in any view and is unchanged. The cooked placements of `thrum-asset-gpu` opt in, on both of `COOKED-DRAW`'s
paths: the direct path draws its placements again, and the indirect path culls meshlets for the view.

**Jitter.** With temporal sampling selected, a view is rasterised with the frame's jitter phase at its own
extent, and the record a consumer reads carries the jittered matrix. A consumer that reads the view by
projection then lands on the same world point in every frame, while the view's texel grid moves under it, so
the main view's reconstruction averages the view's edges as it averages its own. In a section, an edge in a
half-size view read this way sits within 0.02 px of the true edge and wanders 0.19 px as it slides; with no
jitter in the view it wanders 1.95 px, which is crawl
([part A](../../evidence/SCENE-VIEWS/design-proof/README.md)). Without temporal sampling a view is not jittered.

### What the consumer reads

The asking contribution's own passes may read the slot's colour and depth as textures, in the frame they were
asked for, under the access [ruling 2 for `WATERS`](../ARCHITECTURE.md#seam-rulings-for-waters) gives for
scene colour and depth, and the view's record as a uniform: the jittered and unjittered view-projection, their
inverses, the camera and the extent. Nothing else of a view is lent. A consumer filters the image, into mips
or otherwise, in resources of its own.

### Lifecycle

A view holds no state between frames, so a discarded frame, a capture, a cut and a pause need nothing. Slot
targets follow the render extent through a resize. Device replacement and republication rebuild slots with
the scene. A view's passes carry their own names in the frame timing feed (C12) and are absent from it in a
frame that asks for none.

### Knobs

Named runtime values in `EFFECT-GRAPH`'s list, each with a declared effect and none with a history to
invalidate: a view's extent, its far distance, its least size, and the count and reach of its cascades. The
count of slots is the profile's. Each is the asking contribution's to register, so the governor moves water's
mirror through water's knobs.

### Check that it can work

- **V1.** A view runs the main view's shader with the main view's inputs, and each input that depends on the
  view is produced by the code that produces the main view's, from the view's camera. So a view of camera X
  must equal the main view drawn from X, where both cover, up to the sky the view leaves out. M1.1 holds it to
  that. Unrun.
- **V2, V3.** Nothing a view reads is fitted to the main frustum: its lists, cascades, clusters and aerial
  volume are its own, and the height map and the air's shadow heights are laid out round the main camera's
  position and not its direction. Moving the main camera does move those two; M1.4 turns it only.
- **V4.** The plane test is per fragment and exact to a pixel. The depth it leaves is the standard reversed
  depth, which resolves 0.33 mm at 3 km (part C), so `WATERS`' lookup loses nothing to the cut.
- **V6.** Part A, above. That reconstruction then holds a reflection steady is `WATERS` M14.6's to judge.
- **V7.** Unrun. M3 takes the record.
- **A hard component on paper: the cooked indirect path in a view.** Its meshlet culling reads one frustum
  from the contribution frame and writes one set of indirect arguments. For a view it needs the same pass
  again with the view's planes and a second set of arguments, and a second pipeline set from the view's
  surface. Both exist in kind. Whether its buffers can be doubled inside the limits Chrome reports is M2's
  first finding, and the direct path is the declared fallback for a view if they cannot.

## The system as it runs

| State | Owner |
|---|---|
| Slots, requests, a view's record, its pass and fragment entry, its targets, the opt-in for contributions | This module, in `thrum-render` |
| The compaction a view culls with | Built here in `CULLING`'s files, [by ruling](#what-scene-views-asks-of-other-modules) |
| A view's cascades, cluster table and aerial volume | Built here in the forward pass's and `ATMOSPHERE`'s files, by ruling |
| Cooked placements in a view | Built here in `thrum-asset-gpu`, by ruling |
| Which views a frame has, and what is read from them | The asking contribution: `WATERS` |
| The mirror example, `examples/mirror` (package `mirror-scene`) | This module |

**A frame, in order.** `upload`: each contribution with slots asks or leaves them idle. `simulate` and the
main view's culling as before. For each asked view: its compaction, its cluster table and its aerial volume.
The Sun passes: the main view's cascades, then each asked view's. Each asked view's pass. Then the main
forward pass, the sky and stars, and `after-opaque`, where the consumer reads its views. A frame that asks
for nothing runs none of the view steps.

**The example.** `examples/mirror` is a creator scene on the sky kit: ground, a tower, rocks, a post standing
in a pool, a lamp, a far hill, and a flat pool. The pool is the example's own diagnostic effect: it declares
one slot, asks each frame for the camera's mirror image in the pool's plane, and shades each pool pixel with
one read of the view where that pixel's surface point projects, which on a flat mirror is exact. It is a
fixture and a demonstration, and no engine crate holds it.

**Creator journey.** An effect author declares slots with their extents, asks in upload with a camera, reads
colour, depth and the record in the effect's own pass, and registers the knobs. Changing a request needs no
rebuild. Changing the count of slots or an extent share is a new preparation.

## What SCENE-VIEWS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-scene-views). Each is built by this module's
builder in the owner's files, with the owner's items, tests, bounds and references unchanged and passing, and
a scene that declares no view compiling to the same shader text, passes, pipelines and storage and drawing
the same bytes.

1. **The contribution seam** (`EFFECT-GRAPH`'s, M1): a contribution may declare slots, ask in upload, opt in to
   drawing in views, and read its own slots in its own passes.
2. **A camera-drawn compaction against another volume** (`CULLING`'s files, M1), as Sun casters have.
3. **Per-view lighting products** (the forward pass's shadow and cluster files and `ATMOSPHERE`'s aerial pass,
   M1): cascades, a cluster table and an aerial volume for a second view, and the aerial pass's fold height.
4. **The view record for a view that is not presented** (`APPLE-PRESENTATION`'s, M1). C07 now names a
   secondary view: it publishes no frame contract and has no history key.
5. **Cooked placements in a view** (`thrum-asset-gpu`, `COOKED-DRAW`'s paths and `CONTENT-LOOP`'s residency, M2).

`WATERS` is asked for one thing: its M14 reads a view as [this design](#what-the-consumer-reads) lends it.

## Cost account

Estimates, unmeasured. A view is the scene's opaque draw again at its extent, less what its culling removes.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| Compaction | Frame, each asked view | The scene's instances once more | As the main view's Frustum compaction |
| Cluster table and aerial volume | Frame, each asked view | 32×18×16 cells; 32×32×32 texels | Small beside a draw |
| Cascades | Frame, each asked view | One 1024² depth layer in the first profile | A Sun pass with fewer casters |
| The view's pass | Frame, each asked view | A quarter of the main view's pixels at half each side; one colour and one depth target stored | The largest term. On a tile GPU each view is a render pass with two stores |
| Memory | | Colour and depth at 1280×720 for a 2560×1440 frame: about 11 MiB a slot; a cascade layer 4 MiB | |

**Records and tripwires, never gates** ([brief](../BRIEF.md#goal-and-exit-rule), decision 3).
`scripts/scene_view_cost.py`, created in M3, takes a paired record of one scene at one view with capture off:
no view asked, one and two, with each view pass's GPU time by name beside the main view's, the bytes of each
resource and the load on the machine. A wire trips when the whole frame is over the refresh interval, when a
pass is about 25% over its last comparable record, or when a view costs more than the main view's opaque
draw at the same pixel count. A tripped wire is investigated in the milestone that tripped it. **No timing
figure decides whether an item passes.**

## Risks

1. **A view costs a second scene.** Early sign: M3's record. The designed remedies are the knobs: extent, far
   distance, least size, no cascade. The wrong fix is drawing a view every other frame, which fails V6.
2. **The view fragment entry diverges from the main one.** Early sign: M1.1. The entry shares `shade` and adds
   only the plane test and the output; a second copy of the lighting is the wrong fix.
3. **Discard costs the view its hidden-surface removal on Apple GPUs.** Early sign: M3's record on the Apple
   recipe. The designed remedy is a clip-distance variant for the recipe that has it, promoted by a paired win.
4. **Chrome refuses the added bindings, buffers or a format.** Early sign: M2's first finding for the cooked
   path and M3's first Chrome run. Limits are read from the capability record and refused at preparation.
5. **Shadows in a view read coarse or swim**, because one cascade spans the view's whole reach. Early sign:
   M2.5. The remedies are the second cascade and a shorter reach.
6. **Another builder is in the same files.** `ATMOSPHERE` M6 and M7, `CULLING` and `APPLE-PRESENTATION` edit
   files this module edits. Whichever lands second integrates on top, and the exact baselines catch a slip.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. Chrome checks need a browser launch outside
  the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
- **Capture.** Inspected items use T2 engine capture in the mode they name, at 2560×1440. The route, framing
  and length are the builder's and the reviewer's to choose; frames are judged whole and at native size.
- **Exact baselines.** Every milestone edits covered paths and carries the verifier
  ([rules](../ARCHITECTURE.md#exact-baselines)). No retained image moves.
- **Engine tests hold no game content.** The tests of `thrum-render` and `thrum-asset-gpu` make their own
  scenes. The pool, the tower and the lamp are the example's.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/scene-views-<milestone>/`, selected
  evidence in `docs/evidence/SCENE-VIEWS/`.
- **Dependencies.** The example's package is registered in `.config/architecture.toml`.
- **Maturity.** A ledger row for secondary views is added in M3, with implementation, correctness, visuals
  and device apart.

## Open questions

| Question | Settled by |
|---|---|
| Does a view equal the main view from its camera to the last bit, or to a bound? | M1.1 measures first, then asserts |
| Can the cooked indirect path hold a second set of culling buffers inside Chrome's limits? | M2.1; the direct path is the fallback for a view |
| What does a view cost, and what does each knob buy? | M3.4 |
| Is one cascade to 128 m enough for a reflection? | M2.5; [risk 5](#risks) |
| Is a clip-distance variant a measured win on the Apple recipe? | After M3, as a recipe proposal in the log |
