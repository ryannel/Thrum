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
it a module of its own. The lead's task for this plan: "plan the new module SCENE-VIEWS that WATERS WP14 depends
on. Choose methods from first principles and current practice."

## Purpose and scope

A **secondary view** is the scene's opaque geometry drawn in the same frame from a camera another module names,
into a texture that module reads while it shades the main view. It is a product of the frame, as a Sun cascade
is. It is never presented, and it has no history.

| User | Needs | What that requires of the design |
|---|---|---|
| `WATERS` WP14, for a lake, a pond and the sea under an island | The far shore, hills, trees and the craft as a camera at the mirror image of the main one would see them, with the light the main view gives them, up to two a frame | The same shading, with what belongs to a view made again for the view; a cut at a plane; air that starts at the water's level plane, so each stretch of a reflected path has one owner |
| `PUDDLES`, `RIVERS`, through `WATERS` | The same, with no work of their own | Nothing beyond the interface |
| A game that has no such water | Nothing | A scene that declares no view is what it was, bit for bit, but for [one correction every scene has](#where-the-height-map-is-laid-out): where its height map lies when its camera stands on a texel's edge |
| A later consumer: a monitor in the world, a portal, a probe that stands elsewhere | A view from a camera somewhere else | [Not built](#the-camera-stands-on-the-main-cameras-vertical). The request is a camera and nothing in it says "mirror", so serving one adds to a slot and changes no request |

**Commissioned:** secondary views on `mac-metal`, on both recipes, and `chrome-webgpu`, of everything a scene
draws as opaque geometry: its entities, on both of `CULLING`'s routes, and its cooked placements.

**Not built**, with the reason:

- The sky, stars, water, clouds and any other after-opaque effect in a view. A consumer composes its own
  background, and `WATERS` takes the sky and its clouds from the environment product.
- Fog in a view. A scene with `ATMOSPHERE`'s view volume draws its views in clear air
  ([open for the human](PLAN.md#open-for-the-human)).
- Air that starts at a tilted plane. Each line of sight would start at its own altitude: lines one row of
  the volume apart start tens to hundreds of metres apart, and further the nearer a line lies to the plane
  ([part F](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)). A blend of neighbouring columns would then
  be no one observer's, and nothing `ATMOSPHERE` makes could be its reference. Every water plane is level.
  A view may still be cut at a tilted plane, with its air from its camera.
- A camera off the main camera's vertical: a monitor in the world, a portal, a probe that stands elsewhere,
  a mirror in a tilted plane. Beyond a view's cascades the Sun's shadows come from the frame's caster height
  map, which is laid out round the main camera's horizontal position, and a camera anywhere else would need
  that map and the air's shadow heights made again round itself: every caster within 16 km drawn once more,
  each frame. No consumer asks for it, and every water mirror stands under the main camera. Such a request
  is [refused by name](#the-camera-stands-on-the-main-cameras-vertical). Serving one later is a slot
  declared with terrain shadows of its own, which adds to what is built here and changes none of it.
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
| **V1** The same scene in the same light: a thing in a view looks as the main view would show it from there, Sun-lit and shaded sides, shadows, lamps, the haze of its own distance | Any rigid camera on the main camera's vertical, with none on it excepted: at any height, facing any way, with any field of view, wherever the vertical stands; an extent of a sixteenth of the main view's pixels to all of them; dawn to night; entities and cooked placements; every culling mode | A capture lit by ambient light alone, or one that reuses the main view's cluster table, aerial volume or cascades. Rejected by the view against the main view drawn from the same camera, and by a lamp and a hill that stand outside the main frustum |
| **V2** What the main camera cannot see is there | Things behind the camera, above the frame and behind a nearer hill; 1 to 100,513 instances | The main view's visible list drawn again. Rejected by a view that faces the other way |
| **V3** A view does not depend on where the main camera looks | The request held while the main camera turns, tilts, and rises or sinks where it stands, wherever that is, a texel's edge of the height map included | Cascades, clusters or visibility fitted to the main frustum. Rejected by the view's bytes with the main camera turned through 180°, and by the height map's layout as it turns on a texel's edge |
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
  [part A](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md) measures it. A search for which far-shadow
  products a shipped engine gives a reflection capture found only forum reports of shadows missing from
  Unreal 4's planar reflections, so that choice below rests on this renderer's own code and measurement.

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
| The Sun's shadows beyond a view's cascades | A caster height map and air shadow heights of the view's own, laid out round its camera | Not selected. Right for any camera, but it is every caster within 16 km drawn top-down and reduced once more, each frame for each view, and for a mirror's camera it draws the bytes the frame already holds. It is the route for [a camera elsewhere](#purpose-and-scope), when a consumer has one |
| | The frame's map, for any camera that stands inside it | Fails V1. The march toward the Sun ends where it leaves the map, so off the map's centre a receiver loses hills that the main view from that camera has. With the map centred 40 km away, 283 pixels of a 128×128 frame differ by up to 2,904 stored steps ([the builder's Metal run](../../../.orchestrate/evidence/SCENE-VIEWS/WP1/9be4a4887-portable.log), `shadow/tests/scene_view_anchor.rs`). Two maps that overlap are not two equal reads |
| | **The frame's map, for a camera on the main camera's vertical** | **Selected.** The scene's fit lays the map out by the camera's horizontal position alone, so there the frame's map [is the view's own](#what-a-view-owns-and-what-it-shares) |
| Which position the fit lays the height map out round | The one it takes back out of the camera's view matrix, as the fit had it | Fails V1 and V3 on a texel's edge. That position carries the matrix's rounding, which follows the camera's direction and height, so a camera and its mirror image in a level plane laid their maps out 32 m apart (the sixth design review's probe, and [part G](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)) |
| | That fit, with equality promised only where two cameras snap alike | Not selected. The limit would be a rounding no consumer can test for, V3 would fail inside it, and a camera in motion meets it: one in 19,000 of part G's cameras at positions unrelated to the grid. Refusing a request there instead would drop a reflection for a frame |
| | **The position the camera declares** | **Selected.** Nothing is rounded, so every camera on a vertical [has the one map](#where-the-height-map-is-laid-out). It adds no work to a frame, and it steadies the main view's own far shadows as it turns on an edge |
| The cut at the plane | An oblique near plane in the projection | Not selected. It needs no shader change and [part C](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md) finds its depth still usable, but the scene's shader turns stored depth into view depth by the standard projection's formula, to pick a cascade and a cluster slice. An oblique matrix would need that rewritten for every drawer, which moves every exact baseline |
| | A hardware clip distance | A candidate for a recipe later: it is an optional feature in WebGPU, and it needs a vertex variant in every drawer |
| | **A test of each fragment against the plane, in the view's own fragment entry** | **Selected.** A view's pipelines are separate from the main view's in any case, because a view has one colour target where the main view has three |

## Mechanisms

### Slots and requests

A contribution declares, when the scene is prepared, how many secondary views it may ask for and the largest
extent of each as a share of the render extent, from a quarter to the whole of each side. Each is a **slot**:
its targets at that largest extent, its per-view products, pipelines and graph passes exist from preparation,
so asking never compiles or creates anything. A scene's slots are bounded by its profile, four in the first, and a declaration over the bound, or
over what the capability record allows, is refused at preparation.

Each frame, in its upload, the declaring contribution **asks** for a slot or leaves it idle. A request is a
camera and nothing more specific:

- a rigid, right-handed view transform and a symmetric perspective with a near and a far distance;
- optionally a **clip plane**, in world space;
- optionally, with a clip plane that is level, **air from the plane**
  ([below](#what-a-view-owns-and-what-it-shares));
- an extent no larger than the slot's, drawn as a viewport inside the slot's targets;
- the least size, in the view's pixels, of an instance or a meshlet group worth drawing.

An improper transform is refused. So is air from a plane that is not **level**, by name and when it is
asked for, never taken and ignored. A plane is level when each horizontal component of its unit normal is
under 1e-6: over the aerial volume's 32 km that is 3 cm of height, and 0.003% of the aerosol's density.
A clip plane nearer the view's camera than 5 cm is moved to 5 cm, so a request never degenerates as a camera comes down to the water; at most that much geometry below the plane is
then admitted. An idle slot records no pass, and its targets keep nothing a consumer may read.

A mirror is not a special case. The consumer builds a proper camera at the mirror image of the main one. A
reflection matrix would turn every triangle's winding; a rigid camera does not, and the image the consumer
reads is addressed by projecting world points through the view's own matrix, so it needs no flip.

#### The camera stands on the main camera's vertical

A request's camera has the two horizontal coordinates of the frame's main camera, bit for bit, as the two
positions are declared. Its height, its direction and its field are free. The engine compares the declared
positions and never one it recovers from a matrix, and the frame's camera facts give a contribution the main
camera's position, so a consumer copies the two coordinates and the comparison cannot fail by rounding. A
mirror in a level plane is such a camera: only its height changes. A request from anywhere else is refused
by name when it is asked for, never taken and drawn with shadows that are not its own, and the slot is idle
in that frame. [The terrain shadows](#what-a-view-owns-and-what-it-shares) are the reason.

### What a view owns and what it shares

The forward shader is shared text. What it reads splits by whether the quantity depends on the view.

| Made again for each asked view | Shared with the main view |
|---|---|
| The view record and its auxiliary record | Instances and their history, the mesh arena, materials |
| The visible lists and indirect records, from a compaction against the view's frustum and clip plane | The Sun, the sky's diffuse and reflection products, the cloud factor and any other Sun-path provider |
| The Sun cascades and their uniform, fitted to the view's frustum by the scene's own fit | The Sun's terrain shadows: the caster height map and the air's shadow heights, one of each for the frame, laid out round the main camera's horizontal position |
| The cluster table of local lights | The local-light list |
| The aerial volume | The frame's exposure sample and storage scale |
| Colour with coverage, and depth | |

- **Visibility.** One more compaction for each asked view, over the same instances, against the view's six
  frustum planes, its clip plane and its least size, into the view's own lists and indirect records: the
  shape of `CULLING`'s Sun-caster culling. It keeps slot order within a bucket, as Frustum does. A scene in
  Direct mode draws every instance in a view too. Occlusion is never used in a view, whatever the scene's mode.
- **Shadows.** A view's fragments pick a cascade by their depth in the view, so the view has cascades of its
  own: the scene's fit applied to the view's frustum out to the view's shadow reach, drawn with the casters
  culled for each. The first profile gives a view one cascade to 128 m; the count, from none to two, and the
  reach are knobs. With none, the view has hill-scale shadows only.
- **Terrain shadows.** Beyond the reach the caster height map answers, marched toward the Sun, as it does
  for the main view, and the shadowed air reads the air's shadow heights, which are reduced from that map.
  The frame has one of each and a view reads them. On the main camera's vertical they are the view's own,
  exactly, for every camera on it. The scene's fit lays the map out as a square 32 km on a side round the
  camera's horizontal position [as the camera declares it](#where-the-height-map-is-laid-out), snapped to
  the map's 32 m texel, over the height range of the scene's casters, and nothing else of the camera enters
  it (`height_matrix` in `synthetic/shadow.rs`). Which casters are drawn into it depends on the camera
  through that square alone, and the air's heights are a function of the map and the Sun. So two cameras
  on one vertical have one map, one address record and one set of air heights, whatever their heights and
  directions. Off the vertical they do not, and overlap is not enough: the march ends where it leaves the
  map, at a different distance from a receiver in each. That is why
  [a request off the vertical is refused](#the-camera-stands-on-the-main-cameras-vertical).
- **Lights.** A cluster table is built for the view's own frustum by the pass that builds the main view's.
- **Air.** The aerial volume is computed again along the view's own lines of sight, from the view's camera.
  A mirror's camera stands under its plane, where there is no air: light reaches the main camera from a body
  by way of the water, body to water and then water to camera, and the mirrored camera's straight line to the
  body is only the first of those stretches, beyond where it crosses the plane. So a request with a level
  clip plane may ask for **air from the plane**. Then the pair at a point is the transport from where the
  point's line of sight crosses the plane to the point, and nothing before it. Every sample is a real point
  beyond the plane, taken with the line's own direction, the Sun's light and the scene's Sun visibility at
  that point, and bounded by the atmosphere's top and the ground as any ray is, so the phase angle and the
  shadows are the true stretch's.
  - *The volume* is `ATMOSPHERE`'s aerial volume for an observer at the plane's height. On every line of
    the volume it holds the column the pass stores for an observer at that height who looks along the line,
    with its layers at their own distances from that observer. The pass's column depends on the observer's
    altitude and the line's direction, and on nothing else of where the observer stands (`aerial.wgsl`
    integrates from `(0, h, 0)`), and under a level plane every crossing has the plane's altitude. So the
    volume is the one an ordinary main camera of the view's orientation and projection has, standing
    anywhere on the plane. The pass is given the plane's height where it is given a camera's, and a view
    adds no integrator. A height outside what the pass takes for an observer, 0 to 100 km, is held to that
    range, so a tide below the datum has its air from 0 m.
  - *The decoder* reads the volume at a fragment's screen position, by the fragment's distance beyond its
    own line's crossing, and takes the scene's Sun visibility along that stretch.
  - *Lines that never cross.* A line of the volume that runs along the plane or away from it is no
    fragment's own: a drawn fragment is beyond the plane and the camera before it, so its line crosses. But
    the decoder blends the columns of the lines round a fragment, and round a fragment near the plane's
    horizon some of those never cross. They hold what every line holds, the product's column from the
    plane's height, air below the plane included. That air is the filter's support and never a fragment's
    own, and it is what a main camera standing on the plane holds on the same lines. The revision before
    this one left them empty, which took 39% of the in-scatter off a line 0.2° above a lake's horizon
    ([part F](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md), after the third design review).

  So a view's air has [an exact reference](#what-a-views-air-is-held-to). Laid out from the camera, the
  layer the crossing falls in would blend air with none: a panel 20 m beyond a crossing 230 m from the
  camera would take twice its in-scatter ([part E](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)). A
  camera on the kept side of the plane has its air from the camera.
  The stretch from the main camera to the plane is the reader's, which has it already: the lent aerial pair
  from its own camera to its own surface point. Each stretch has one owner and is applied once (C08).
  The first version of this design folded altitudes about the plane and kept the line's direction; that put
  the wrong phase angle on the stretch below the plane, 40 times the true in-scatter in the design review's
  case, and a reader could not take it off again ([part D](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)).
  A view has clear air even where the main view has the view volume, so with air from the plane a reader
  still applies the main view's fog once, on its own stretch. With no such request the aerial pass and the
  decoder do what they do today, and the main view's shader text never changes.
- **Exposure and encoding.** A view's colour is in the scene target's own encoding for that view,
  `(T_v · L + S_v) · E`: radiance through the view's air, from its camera or from the plane as the request
  says, times the frame's exposure scale. Coverage is 1 where
  geometry drew and 0 elsewhere, in alpha, and uncovered colour is zero. Depth is the standard reversed depth
  of the view's projection, so the view's inverse matrix turns a texel back into a world point.

#### Where the height map is laid out

The scene's fit lays the caster height map out round the horizontal position **the camera declares**: the
position a view is made with, which the view keeps and the frame's camera facts report. It is snapped to
the map's 32 m texel, and nothing else of the camera enters the layout. This is a change to the forward
pass's fit, made for every scene under [ruling 3](#what-scene-views-asks-of-other-modules), and it is the
one thing this module changes in a scene that declares no view. The cascades' fit is not touched.

**What it replaces.** The fit took the position back out of the camera's view matrix. That position carries
the matrix's rounding, which follows where the camera faces and how high it stands. So on a texel's edge two
cameras on one vertical, or one camera turning where it stands, could lay the map out a texel apart.
[Part G](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md) reproduces it with the fit's own arithmetic, after
the sixth design review's probe. From `(16, 10, 0)`, looking along `(0, 0, −1)` puts the map's centre at one
texel and looking along `(−2, −1.7, −1)` at the next. The camera `(16, 10, 0)` looking along
`(−2, −1.7, −1)` and its mirror image in the level plane 5 m up, `(16, 0, 0)` looking along `(−2, 1.7, −1)`,
lie 32 m apart. Of 78 verticals with a coordinate on an edge, every one held cameras whose maps lay apart.
Two maps a texel apart hold different casters in the 32 m strip at each end, and the march toward the Sun
ends where it leaves the map, so a view that read one could not be held equal to a main view that had
fitted the other.

**What holds now.** Two cameras on one vertical declare the same two horizontal numbers, bit for bit, and
the fit is a function of those numbers, the scene's casters and the Sun. So:

- The frame's main camera, a request on its vertical, and an ordinary scene whose own camera stands where
  the request's does all have one map: the same layout and address record, value for value. In part G, on
  90 verticals at 10 heights from 500 m below the datum to 9 km above it, none differs, the 78 on an edge
  included. This is the equality V1, WP1.1 and WP2.1 promise, and no admitted camera is excepted from it.
- Turning, tilting, raising and lowering the main camera where it stands does not move the map, on an edge
  as anywhere else, so V3 has no exception either.
- The comparison that admits a request and the fit that lays out its shadows read the same declared
  numbers. Admission and equality cannot disagree about which cameras stand on one vertical.

WP1.14 holds the fit to this at the review's cameras, and WP1.1 draws the mirror pair. Chrome runs the same
fit, so WP3.1 inherits both.

**What it changes in a scene with no view.** A frame's bytes differ from before only where the camera's
matrix had rounded its position into the neighbouring texel. In part G that takes a camera within four
steps of a 32-bit float of a texel's edge, 8 µm at 16 m from the origin and 6 mm at 20 km, facing some
ways and not others: 101 of 1.9 million cameras at positions unrelated to the grid. There the map now lies
where the declared position puts it and stays there as the camera turns; before, such a camera's far
shadows could step a texel as it turned. The registered exact baselines are drawn from the showcase
valley's authored camera, which stands at `z = 16`, on an edge. Part G reproduces that camera and the
moving route's 721 half-tick poses: in every one the two positions fall in one texel, so no retained image
is expected to move. That is a CPU reproduction; WP1.10's verifier decides.

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
([part A](../../../.orchestrate/evidence/SCENE-VIEWS/design-proof/README.md)). Without temporal sampling a view is not jittered.

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
extent moves a viewport inside the slot's prepared targets, up to the slot's largest, and each cascade a view
may use is prepared with the slot, so no knob creates a pipeline or a resource. The count of slots and each
slot's largest extent are fixed at preparation. Each is the asking contribution's to register, so the governor moves water's
mirror through water's knobs.

### Check that it can work

- **V1.** A view runs the main view's shader with the main view's inputs, and each input that depends on the
  view is produced by the code that produces the main view's, from the view's camera. The terrain shadows
  are the frame's, which on the main camera's vertical are the ones the scene's fit gives the view's camera,
  [whichever camera on it](#where-the-height-map-is-laid-out).
  The frame's lighting (the Sun, the sky's products, exposure) is the host's, and no camera derives it. So a
  view of camera X on that vertical must equal the main view drawn from X in the same lighting, where both
  cover, up to the sky the view leaves out, when the two are drawn under one shadow profile. A view's own
  profile is smaller than the main view's (one cascade, no fog), so that is the comparison WP1.1 makes, to
  one step of each stored format: with X at the main camera's position; with X 80 m along the vertical
  from it, where a hill's shadow reaches the receivers through the height map alone; and with the main
  camera on a texel's edge and X at its mirror image in a level plane, a pair whose matrices recover
  positions on either side of the edge. At one position the builder's Metal run has it at zero steps with
  none, one and two cascades ([proof](../../../.orchestrate/evidence/SCENE-VIEWS/WP1/9be4a4887-proof.md)). The displaced
  case and the edge are unrun on a GPU; part G has the edge's layout on the CPU.
- **V2, V3.** Nothing a view reads is fitted to the main frustum: its lists, cascades, clusters and aerial
  volume are its own, and the terrain shadows are laid out by the horizontal position the two cameras
  declare, not by the main camera's direction or height. WP1.3 turns the main camera where it stands, and
  WP1.14 turns it on a texel's edge.
- **V4.** The plane test is per fragment and exact to a pixel. The depth it leaves is the standard reversed
  depth, which resolves 0.33 mm at 3 km (part C), so `WATERS`' lookup loses nothing to the cut.
- **V6.** Part A, above. That reconstruction then holds a reflection steady is `WATERS` WP14.6's to judge.
- **The air of a mirrored path.** Part D, in a section with the aerosol's phase function: a view with air
  from the plane, composed with the main camera's own stretch, is the true two-stretch path to rounding for
  a Sun ahead, behind and low, under a ridge that shades part of the path, and with a fog bank between the
  camera and the water. The first design's whole-path image, with the camera's stretch taken off by the
  reader, is 40 times the true in-scatter in the review's case and 2.2% off in total behind the bank.
- **The plane's horizon.** Part F, with the pass's ground bound: in the third design review's case, a
  lake 50 m up, a level camera 2 m below it and a line 0.2° up, 40% of a fragment's read lies on a line that
  never crosses. With that line holding the product's column the read is the main view's from the crossing,
  texel for texel, and within 0.03% of a fine quadrature of the fragment's own line at 1 km. With it empty
  the read is 39% low.
- **V7.** Unrun. WP3 takes the record.
- **A hard component on paper: the cooked indirect path in a view.** Its meshlet culling reads one frustum
  from the contribution frame and writes one set of indirect arguments. For a view it needs the same pass
  again with the view's planes and a second set of arguments, and a second pipeline set from the view's
  surface. Both exist in kind. Whether its buffers can be doubled inside the limits Chrome reports is WP2's
  first finding, and the direct path is the declared fallback for a view if they cannot.

### What a view's air is held to

Two things could be asked of a view's air. One is this module's.

**Transport accuracy is `ATMOSPHERE`'s, and stays where it is.** With air from the plane a view's volume
is, by construction, the aerial pass's volume for an observer at the plane's height. It is as accurate as
the air of a main camera standing on the water, no more and no less.
[`ATMOSPHERE`](../ATMOSPHERE/DESIGN.md#numerical-acceptance) bounds that product's storage against its own
integrator (P10) and its decoding against its own stored texels (H4), and says that no test bounds solar
aerial transport against an independent reference. A view inherits exactly that. This module promises no
transport bound, and none of its items compares a view with an independent quadrature. In part E's stand-in
the pass's quadrature lies about one stored step from a fine one; nobody has measured the real one, and it
is not measured here.

**This module is held to a view's air being that product, for the right stretch.** The reference is
`ATMOSPHERE`'s own product, made by its code as it stands with no request, from a camera the test puts on
the plane: anywhere on it for the stored volume, and at a fragment's crossing for its colour. It exists
once `ATMOSPHERE:WP5` is Done, which WP1 waits for, and in Chrome the reference is
Chrome's own. Two comparisons keep what is stored apart from how it is read.

- **The stored volume** (WP1.5): transport and storage. Every texel's column of a view's volume, whether or
  not its line crosses the plane, is the column the unmodified aerial pass stores at the same texel for a
  camera of the same orientation and projection standing on the plane. The allowance is one stored step in
  each channel: the two runs do the same arithmetic on inputs that differ in their last bits.
- **The shaded fragment** (WP1.12): decoding, shadowed and not. Under a level plane, the stored colour of a
  fragment at a pixel of the view is the stored colour the scene's main view gives that fragment at the same
  pixel, from a camera of the same orientation, projection and extent standing where that pixel's line
  crosses the plane, in a scene with no view volume, with temporal sampling off and a surface whose own
  radiance does not depend on where along the line the camera stands. Both then run one decoder on equal
  volumes, at one screen position and one distance, over the same world points. Every texel either read
  blends is equal, those of lines that never cross included, so this holds at the plane's horizon as it
  does away from it. The scene's Sun visibility along the stretch is the same in both, and a caster on the
  stretch before the crossing is in neither. The allowance is **rounding**: one step of the stored
  colour, plus `Q` for each filtered read of
  the volume whose value enters that fragment's colour, four in the plain decoder and eighteen in the
  shadowed one as they stand. `Q` is one stored step of the volume carried through the decode, as
  [`ATMOSPHERE`'s handoff](../ATMOSPHERE/DESIGN.md#numerical-acceptance) defines it. The test works the
  allowance out and prints it beside the difference.

**What that rejects.** Part E, as in-scatter on a black panel, in stored steps of 0.098%, for a line 10° up:

| Wrong path | Camera 2 m below, panel 20 m beyond | 2 m, 3 km | 40 m, 20 m | 40 m, 3 km |
|---|---|---|---|---|
| Air that starts at the view's camera, or the first design's folded altitudes (the nearer of the two) | 591 | 5 | 11,622 | 95 |
| The right stretch lit at the camera's own stretch's angle, Sun across; Sun low behind | 433; 45 | 433; 45 | 433; 45 | 433; 45 |
| The right stretch in a column laid out from the camera | 1 | 0 | 1,064 | 0 |

A panel 3 km beyond a crossing 11 m from the camera cannot tell where the air starts: the stretch before the
crossing is 5 steps, inside what rounding allows. The other three cases can, and WP1.12 names them. A column
laid out from the camera can pass the fragment, where the crossing is near the camera or the panel far. It
fails the stored volume in every near layer, where the layer's end is a different distance beyond the
crossing.

**At the horizon.** WP1.12 takes the third design review's case: a plane 50 m above sea level, a level
camera 2 m below it with a 60° field, and a panel 1 km beyond the crossing on a line 0.2° up. 40% of the
weight of that fragment's read lies on a row that never crosses the plane. With that row empty the
in-scatter is 39% low, 403 stored steps in part F's stand-in, where the review's own run found 75.6 times
the allowance. The test reads the view's volume back and shows that the same read with the never-crossing
lines emptied misses the allowance, so the case can tell.

**What the product itself loses at the horizon.** The aerial pass stops a descending column where it meets
the model's ground and holds it constant beyond. A fragment just above an observer's horizon blends such a
column, so it reads less in-scatter than its own line holds. In part F's stand-in:

| Observer | Panel 1 km away | Panel 3 km away |
|---|---|---|
| On a plane 50 m up (a view, and the main camera standing there) | within 0.03% | up to 4.0% low: the row below meets the ground at 2.8 km |
| On a plane at sea level, the model's ground | 7% to 87% low | 7% to 88% low |
| An ordinary main camera 3 m above the sea | 6% to 39% low | 7% to 62% low |

The range in each cell is the horizon falling just over a row of the volume or just under one, so the loss
moves as a camera tilts. It is the product as it stands for every observer near the ground, in the main
view and in a view alike, and it is transport accuracy: `ATMOSPHERE`'s. It is
[proposed there](../ATMOSPHERE/LOG.md), with the remedy of Hillaire's reference implementation, which lifts a
voxel under the ground onto it
([`RenderSkyRayMarching.hlsl`](https://github.com/sebh/UnrealEngineSkyAtmosphere/blob/master/Resources/RenderSkyRayMarching.hlsl),
fetched for this revision). A view is made by the same pass, so it takes whatever that pass becomes with no
change here. A rule of the view's own for those lines would be a second aerial product, with no reference.
Until then WP1.12's horizon case and the example stand where the product is sound.

**The composed colour** (WP2.3, and WP3.1 in Chrome). The pool composes the camera's stretch with the view's.
The expectation is built from three stored colours of the scene's ordinary main view, with the pool's
effect off and nothing shading the camera's stretch:

- `V`, the marker from the reflection point, at the pixel its reflected line takes in a camera of the
  view's orientation, projection and extent. By the rule above it is what the view holds.
- `B` and `W`, from the main camera, of a panel lying in the pool's plane at the reflection point: black,
  and at a known radiance `L₀`. So `B = E · S_c` and `T_c = (W − B) / (E · L₀)`: the camera's stretch as the
  aerial decoder gives it. The lent pair the pool applies is the same read of the same volume at the same
  point.

The diagnostic pool reflects all of what it reads, so its stored colour is `B + T_c · V`. The allowance is
rounding again: one step of each of the four stored colours and `Q` for each filtered read behind them,
carried through that expression. The pool's one read of the view blends the texels round the point, so `V`
is taken as its range over those pixels. Part E, on a black marker 100 m beyond the reflection point: from
a camera 30 m from that point, leaving the camera's stretch out is 152 steps off, and a view that holds it
as well is 265. From 1.5 m they are 9 and 15, which rounding can hide, so WP2.3 names 30 m. Its test
evaluates both wrong compositions, and each must miss the allowance.

**A miss.** A difference over an allowance has a cause to find. It is never recorded as the bound, and a
builder who cannot close one ends `BLOCKED` with the measurement.

## The system as it runs

| State | Owner |
|---|---|
| Slots, requests, a view's record, its pass and fragment entry, its targets, the opt-in for contributions | This module, in `thrum-render` |
| The compaction a view culls with | Built here in `CULLING`'s files, [by ruling](#what-scene-views-asks-of-other-modules) |
| A view's cascades, cluster table and aerial volume | Built here in the forward pass's and `ATMOSPHERE`'s files, by ruling |
| The caster height map and the air's shadow heights | The forward pass's, as before: one of each for the frame, laid out from the main camera's declared position, read by the main view and by every view |
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
one read of the view where that pixel's surface point projects, which on a flat mirror is exact. It asks for
air from the plane and applies the lent aerial pair from the main camera to the surface point itself. The
scene stands 250 m above sea level, about where the V1 valley does, and its hill within 5 km. In a 60° view
the row of the volume under the horizon then meets `ATMOSPHERE`'s ground 6.9 km away or more, so
[the product's loss at the horizon](#what-a-views-air-is-held-to) is out of the example. Its
effect can be left out of a frame, which is how WP2.3's references are drawn. It is a fixture and a
demonstration, and no engine crate holds it.

**Creator journey.** An effect author declares slots with their extents, asks in upload with a camera on
the main camera's vertical, reads
colour, depth and the record in the effect's own pass, and registers the knobs. Changing a request, its
extent included, needs no rebuild. Changing the count of slots or a slot's largest extent is a new preparation.

## What SCENE-VIEWS asks of other modules

Settled in [the module list](../ARCHITECTURE.md#seam-rulings-for-scene-views). Each is built by this module's
builder in the owner's files, with the owner's items, tests, bounds and references unchanged and passing, and
a scene that declares no view compiling to the same shader text, passes, pipelines and storage and drawing
the same bytes, but for [where ruling 3 lays its height map out](#where-the-height-map-is-laid-out).

1. **The contribution seam** (`EFFECT-GRAPH`'s, WP1): a contribution may declare slots, ask in upload, opt in to
   drawing in views, and read its own slots in its own passes.
2. **A camera-drawn compaction against another volume** (`CULLING`'s files, WP1), as Sun casters have.
3. **Per-view lighting products** (the forward pass's shadow and cluster files, `ATMOSPHERE`'s aerial pass and
   the aerial decoders, WP1): cascades, a cluster table and an aerial volume for a second view, which reads
   the frame's caster height map and air shadow heights; that map laid out, in every scene, from the
   position its camera declares; and air from a level plane: the pass run for an observer at the plane's height, and the decoder a view's pipelines
   compose.
4. **The view record for a view that is not presented** (`APPLE-PRESENTATION`'s, WP1). C07 now names a
   secondary view: it publishes no frame contract and has no history key.
5. **Cooked placements in a view** (`thrum-asset-gpu`, `COOKED-DRAW`'s paths and `CONTENT-LOOP`'s residency, WP2).

`WATERS` is asked for one thing: its M14 reads a view as [this design](#what-the-consumer-reads) lends it.

## Cost account

Estimates, unmeasured. A view is the scene's opaque draw again at its extent, less what its culling removes.

| Work | Rate | Size | Estimate |
|---|---|---|---|
| Compaction | Frame, each asked view | The scene's instances once more | As the main view's Frustum compaction |
| Cluster table and aerial volume | Frame, each asked view | 32×18×16 cells; 32×32×32 texels, from the plane where asked | Small beside a draw |
| Cascades | Frame, each asked view | One 1024² depth layer in the first profile | A Sun pass with fewer casters |
| Terrain shadows | Frame | The frame's one height map and one reduction, whatever the count of views | Nothing added by a view |
| The view's pass | Frame, each asked view | A quarter of the main view's pixels at half each side; one colour and one depth target stored | The largest term. On a tile GPU each view is a render pass with two stores |
| Memory | | Colour and depth at 1280×720 for a 2560×1440 frame: about 11 MiB a slot; a cascade layer 4 MiB | |

**Records and tripwires, never gates** ([brief](../VISION.md#goal-and-exit-rule), decision 3).
`scripts/scene_view_cost.py`, created in WP3, takes a paired record of one scene at one view with capture off:
no view asked, one and two, with each view pass's GPU time by name beside the main view's, the bytes of each
resource and the load on the machine. A wire trips when the whole frame is over the refresh interval, when a
pass is about 25% over its last comparable record, or when a view costs more than the main view's opaque
draw at the same pixel count. A tripped wire is investigated in the work package that tripped it. **No timing
figure decides whether an item passes.**

**Taken under load, with no quiet sitting** (18 November 2026). The record is taken while other builders
work, under the shared `gpu` lock, and keeps the machine's load average and its busiest processes beside the
figures. Its check fails only on a missing measurement. This module claims no figure for a quiet machine and
asks for no sitting with the other builders paused, in its last work package as in any other: the evidence, the
author guide and the maturity ledger call the record's figures taken under load. WP3's record is the first
paired one, so it is the record a later one is compared with, and it has none before it to be compared with.
If figures for a quiet machine are wanted later, the collector's one command retakes the record in a window
the lead batches for every module. No work package waits for that window.

## Risks

1. **A view costs a second scene.** Early sign: WP3's record. The designed remedies are the knobs: extent, far
   distance, least size, no cascade. The wrong fix is drawing a view every other frame, which fails V6.
2. **The view fragment entry diverges from the main one.** Early sign: WP1.1. The entry shares `shade` and adds
   only the plane test and the output; a second copy of the lighting is the wrong fix.
3. **Discard costs the view its hidden-surface removal on Apple GPUs.** Early sign: WP3's record on the Apple
   recipe. The designed remedy is a clip-distance variant for the recipe that has it, promoted by a paired win.
4. **Chrome refuses the added bindings, buffers or a format.** Early sign: WP2's first finding for the cooked
   path and WP3's first Chrome run. Limits are read from the capability record and refused at preparation.
5. **Shadows in a view read coarse or swim**, because one cascade spans the view's whole reach. Early sign:
   WP2.5. The remedies are the second cascade and a shorter reach.
6. **Another builder is in the same files.** `ATMOSPHERE` WP6 and WP7, `CULLING` and `APPLE-PRESENTATION` edit
   files this module edits. Whichever lands second integrates on top, and the exact baselines catch a slip.
7. **A reflection's haze thins just above the water's horizon**, at sea level most. It is `ATMOSPHERE`'s
   product [as it stands](#what-a-views-air-is-held-to), and the main view has it too. Early sign: WP2.5's
   hill, if the example is moved down to sea level, and `WATERS` WP14 on the sea. The wrong fix is a rule of
   the view's own for the lines below its horizon.
8. **A consumer needs a camera off the main camera's vertical.** Early sign: the refusal, by name. The
   designed remedy is a slot with terrain shadows of its own, planned when that consumer is. The wrong fix
   is taking the request onto the frame's map, which misses V1 by thousands of stored steps.
9. **A retained image, or another module's exact test, was drawn from a camera whose matrix rounds across
   a texel's edge**, so [the declared position](#where-the-height-map-is-laid-out) moves its height map.
   Part G expects none among the registered baselines. Early sign: WP1.10, or an exact test in `./dev check`.
   [Exact baselines](../ARCHITECTURE.md#exact-baselines) rule 1 then applies: the builder ends `BLOCKED`
   with the camera and the two layouts, and the planner settles the move with the owner. The wrong fix is
   taking the position from the matrix again for the main view, which brings back the two maps.

## Proof and builder handoff

- **Targets.** `mac-metal` on both recipes and `chrome-webgpu`. Chrome checks need a browser launch outside
  the Codex sandbox, granted per run; if one is refused, leave the item open and say so.
- **Capture.** Inspected items use T2 engine capture in the mode they name, at 2560×1440. The route, framing
  and length are the builder's and the reviewer's to choose; frames are judged whole and at native size.
- **Exact baselines.** Every work package edits covered paths and carries the verifier
  ([rules](../ARCHITECTURE.md#exact-baselines)). No retained image moves.
- **Engine tests hold no game content.** The tests of `thrum-render` and `thrum-asset-gpu` make their own
  scenes. The pool, the tower and the lamp are the example's.
- **Bounds.** An equality item compares like with like: the same camera, extent, shadow profile and frame
  lighting. Its
  allowance is one step of each stored format, because two pipelines may round the last bit differently;
  for WP1.6's rebuilt point it is a few steps of the stored depth and of a 32-bit float at the point's
  coordinates, which the test works out. A view's air (WP1.5, WP1.12, WP2.3) is compared with `ATMOSPHERE`'s
  own product from the plane, to [the allowances above](#what-a-views-air-is-held-to), and never with an
  independent quadrature. A larger difference has a cause to find and is never recorded as the bound. A
  builder who cannot close one ends `BLOCKED` with the measurement.
- **Panel-crossing regression (WP4).** The compact GPU readback keeps WP1.12's comparison to the ordinary
  main camera at the crossing, including the stored-step allowances, the wrong camera stretch, the grazing
  horizon control and the casters that shade before and beyond the plane. The acceptance run covers all
  cases in the existing `scene_view_air_shaded_gpu` test binary. The fixture may split those cases into
  separately named tests in that binary.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/scene-views-<work-package>/`, selected
  evidence in `.orchestrate/evidence/SCENE-VIEWS/`.
- **Dependencies.** The example's package is registered in `.config/architecture.toml`.
- **Maturity.** A ledger row for secondary views is added in WP3, with implementation, correctness, visuals
  and device apart.

## Open questions

| Question | Settled by |
|---|---|
| Does a view equal the main view from its camera to the last bit? | WP1.1, for every camera on the main camera's vertical: to one step of each stored format; more is a defect |
| Does laying the height map out from the declared position move a retained image? | Part G's reproduction of the showcase cameras says no. WP1.10's verifier decides; [risk 9](#risks) has the route if one moves |
| Does anything need a view from a camera off that vertical? | No consumer today. WP1.13 refuses it by name; [risk 8](#risks) has the remedy |
| Can the cooked indirect path hold a second set of culling buffers inside Chrome's limits? | WP2.1; the direct path is the fallback for a view |
| What does a view cost, and what does each knob buy? | WP3.4 |
| Is one cascade to 128 m enough for a reflection? | WP2.5; [risk 5](#risks) |
| Is a clip-distance variant a measured win on the Apple recipe? | After WP3, as a recipe proposal in the log |
| How far is a view's air from true transport? | Not this module's: it is `ATMOSPHERE`'s aerial product from the plane, unqualified there until that module's own campaign |
| Should the aerial pass stop a column at the ground, which thins haze just above the horizon near sea level? | `ATMOSPHERE`'s next plan; proposed in its log. A view follows the pass |
