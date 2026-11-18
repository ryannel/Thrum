# Cluster level of detail, dynamic GI and virtual shadow maps: left until after V1, with three things held open

18 November 2026. **Selected; design lines and one folded signature change. No V1 milestone, no exact baseline.**

The owner, 17 November 2026, as the lead's planning task of 18 November reports it (no word-for-word quote is on
file): Nanite-class geometry, Lumen-class global illumination and virtual shadow maps are left until after V1, on
condition that V1 lays no foundation that forces rework. A read-only check the lead commissioned proposed three
cheap changes before V1. Each is judged here against what the technique needs and what shipped engines do. The
binding lines are in [C05, C07 and C08](../CONTRACTS.md) and in
[the module list's rulings](../../plans/ARCHITECTURE.md#held-open-for-after-v1).

## What each technique will ask of what V1 builds

| Technique | It needs | Where Thrum stands |
|---|---|---|
| Cluster level of detail (Nanite-class) | Meshlets arranged in a hierarchy. Each cluster carries its own error and bounds and those of the group it was simplified from, and a cut through the hierarchy is chosen for each view on the GPU | Cooked mesh products hold meshlets and one bounds record for each (sphere and normal cone). Entities draw whole-mesh rows converted from those products |
| Dynamic global illumination (Lumen-class) | Light evaluated at a point that is not a raster fragment: a ray hit or a cache texel. A gather for each pixel, which needs depth, normal and roughness before the pixel is shaded. The result read back while shading, by world position from a world-space cache or by pixel from a screen-space gather | One `shade` function fetches the surface and lights it. Indirect light is the sky's, asked by direction alone |
| Virtual shadow maps | Depth before shading, to mark the pages a frame needs. A visibility function at the shading point. Shadow draws culled for each page | `sun_visibility` is already a function of the receiver. A depth prepass exists only inside `CULLING`'s occlusion mode, for last frame's visible set |

## 1. Cooked mesh products: accepted in part

**Accepted, as promises with nothing to build.**

- Every mesh product carries its meshlet sections, whichever path draws it.
- A mesh row is its product's geometry in its product's order, held with the product's content hash, so the
  product's meshlet table and bounds describe the row. The row conversion already copies vertices one for one
  and the index list unchanged. The promise is that no later conversion reorders, welds or re-indexes.
- Cluster facts arrive by a cooker bump: a wider bounds element or new sections, under a new container format and
  cooker version.

**Rejected: a version field, or fields reserved for level of detail, in the meshlet bounds record.** The check
asked for the record to be versioned. It already is, three ways, and none of them is in the record. The container
header carries a format the reader refuses when it does not know it. Each section carries its tag and its element
size, and the reader refuses a size it was not built for. The cooker's version is in every action key, so a bump
recooks every product. Products are derived and content-addressed: nothing is ever migrated in place, so nothing
needs to read an old record as a new one. The record is also uploaded to the GPU unchanged, so a version inside it
would spend a word of every meshlet's record to repeat what the header says once.

Reserving the fields now would be a guess. Bevy's meshlet asset, the closest open implementation, replaced its
per-meshlet record between releases (three bounding spheres became a box, an error and a separate BVH of group
bounds) under one whole-asset version number. meshoptimizer's cluster hierarchy builder, published with its 1.0
release, uses another. The fields are chosen when the hierarchy is designed.

**Rejected: uploading meshlet tables beside rows now.** Nothing would read them, and they cost GPU memory in the
exit scene.

## 2. The forward shader and guide buffers: accepted, with the mechanism settled

**Accepted.** The forward shader keeps two steps, a surface step and a light step, with only the surface's values
between them. Normals and roughness are an optional output of a view's graph.

**The mechanism: a thin depth prepass, not a fourth forward target.** Three reasons.

- Order. A gather for each pixel has to run before the forward pass shades, because the forward pass reads its
  result by pixel. A fourth colour target of the forward pass exists only after it.
- Reach. The forward pass's three targets are a fixed triple that every forward contributor's pipelines match
  (the scene, cooked meshes, sky, stars, sea, water). A fourth changes all of them. A prepass is one optional
  method on the contribution seam whose default draws nothing, as the shadow draw is.
- Practice. Godot's Forward+ renderer writes a normal and roughness buffer in its depth prepass and resolves
  VoxelGI and SDFGI before the forward pass reads them. Unity's HDRP forward path always writes a normal buffer in
  its depth prepass for screen-space effects. Neither keeps a G-buffer.

The two steps are what make the prepass cheap to add and hit shading possible: the prepass runs the surface step
alone, and a ray hit or a cache texel runs the light step alone. `shade` is close to that shape today. Its light
terms already take surface values, a world position and a pixel position, except `sun_visibility`, which takes
the vertex output.

**Rejected: building the prepass or splitting `shade` now.** No reader exists, a prepass doubles vertex work in
every frame, and a split of the shader text is a baseline risk taken for nothing. The rule binds later edits
instead. Albedo and hit distance stay options of the frame contract with no producer planned.

## 3. Indirect light by position: accepted for surfaces, rejected for the sky's own products

**Accepted.** Indirect light arriving at a surface the scene shades is one query, by world position, shading
normal, view direction, roughness and pixel position. `ambient_irradiance` and `indirect_radiance` gain the
position and the pixel in [REVIEW-RENDER](../../plans/REVIEW-RENDER/DESIGN.md) M1, which already rewrites the
first and every reader of it. Nothing reads the new arguments, so no byte moves. It is done there because the
callers are three today, and a traced occlusion, a world-space cache and a screen-space gather each need one of
the two arguments. The cooked volume in `thrum-asset-gpu` is a medium, so it is not one of those callers: it
reads the ambient light as a source, by direction, through `ambient_source`, which the same landing splits out of
`ambient_irradiance`.

**Rejected: the same change to the atmosphere's lent `environment_diffuse` and `environment_reflection`.** These
are what `WATERS` and `CLOUDS` read, and they are a different thing: the sky as a distant source, seen from the
camera's location. Direction is the right key for a source at infinity. Global illumination changes what arrives
at a surface, not what the sky emits, so a medium or water's own optics keeps reading the source as it does.
`WATERS`, `CLOUDS`, `SEA` and `ATMOSPHERE` change nothing.

## 4. Metal's one draw for each meshlet

The code review's M71 is where the mesh stages enter: [MESH-SHADING](../../plans/MESH-SHADING/DESIGN.md) replaces
that expansion for cooked placements. Promise 1's second line is what lets entities follow later without a new
cook.

## Not asked of V1

No cluster hierarchy in the cooker, no prepass, no guide buffer, no radiance cache, no virtual shadow pages, no
visibility buffer. The exit keeps cascaded shadows, the sky's indirect light and whole-mesh entity draws.

## Sources

- Karis, [Nanite](https://advances.realtimerendering.com/s2021/Karis_Nanite_SIGGRAPH_Advances_2021_final.pdf),
  SIGGRAPH 2021.
- meshoptimizer, [clusterlod.h](https://github.com/zeux/meshoptimizer/blob/master/demo/clusterlod.h) and
  [Billions of triangles in minutes](https://zeux.io/2025/09/30/billions-of-triangles-in-minutes/), 2025.
- Bevy 0.17, [meshlet asset](https://docs.rs/bevy_pbr/0.17.3/src/bevy_pbr/meshlet/asset.rs.html):
  `MESHLET_MESH_ASSET_VERSION`, `MeshletCullData`, `BvhNode`.
- Godot, [internal rendering architecture](https://docs.godotengine.org/en/4.6/engine_details/architecture/internal_rendering_architecture.html):
  "parts of the forward renderer are performed with a deferred approach", VoxelGI and SDFGI.
- Unity HDRP, [forward and deferred rendering](https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@15.0/manual/Forward-And-Deferred-Rendering):
  the depth prepass writes depth and a normal buffer.
- Epic staff on [the depth prepass](https://forums.unrealengine.com/t/single-layer-water-purpose-of-depth-prepass/2502588/2):
  "That's the pass which marks VSM pages for update."
