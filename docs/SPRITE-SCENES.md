# Build a 2D sprite scene

The Crowd view is a runnable example. It maps the reference simulation's published bodies to sprite rows and follows
the first player when you select the close view. From the workspace root on a Mac, open it with the native shell.

```sh
rustup run stable cargo run --locked --offline --release -p crowd-2d-view
```

Use the same scene without a window to inspect a held close view. This command writes the renderer's SDR output and
submission record in a new folder under `tmp/`.

```sh
target/release/crowd-2d-view fixed 10000 3456 2234 close tmp/my-crowd-still
```

To start your own game, copy the example into a new workspace package. Change its package and binary names, add the
folder to the root Cargo workspace, and register its dependencies in `.config/architecture.toml`. Keep the
`thrum-app::Game` and `thrum-render::runtime::SceneSource` adapters. Replace the example's Crowd-specific world and
scene with your game's world and scene. Run the new package by name.

## Prepare the scene and the sheets

Prepare a 2D host with the platform's configured drawable. In `Game::prepare`, build a `FrameConfig` with
`render_extent: gpu.surface.extent` and `surface: Some(gpu.surface)`. Pass it, your world, your scene source,
the available sheets, and `SpriteSceneRequests::default()` to `FrameHost::new_2d`. Return
`Prepared::inline(host)` for an inline world. This launch uses the 2D renderer with one output pixel per drawable
pixel. A 2D scene has no temporal reconstruction.

Start with an embedded sheet as in the example's `configuration` function. Give `EmbeddedSheet` its
physical texel extent, top-to-bottom straight-alpha RGBA8 sRGB bytes, frame rectangles and texels per world unit.
Each embedded pivot is the frame's top-left corner. As your art grows, use named cooked frames and a content lease.
The scene supports eight sheets in total, with embedded sheets registered first. [Scene preparation](../examples/crowd-2d-view/src/lib.rs) · [Embedded sheet example](../examples/crowd-2d-view/src/scene.rs) · [Cooked sheet guide](SPRITE-SHEETS.md)

## Map tick pages and sample the view

Set `world_capacity` to the world's physical slot capacity, including slots available for spawns. Set
`row_capacity` to the greatest number of frame rows you will submit at once. Select the components you need
in the world's publication, with temporal values enabled for poses that interpolate. Implement
`SceneSource::extract_sprites` by visiting the latest tick page. Read the game component, select the sheet and
frame and place it in world units. Pass each entity's physical slot, generation, stable identity and row to
`SpriteSink::push_pair`. Supply a previous pose only when the same entity occupied the same slot and generation
on the prior tick of the same timeline. With `None`, the engine shows the committed pose at every alpha.
Choose animated frames from the tick so they hold during pause and replay consistently.

Return a `SpriteFrame` from `SceneSource::sprites`. Its `SpriteView` gives the camera centre and
physical output pixels per world unit. X points right and Y points up. A borrowed `&[SpriteRow]` supplies
frame rows for a cursor, damage numbers or other presentation marks. Frame rows have no entity identity and
do not interpolate. The example follows a player by mixing its adjacent published positions at the frame alpha.
[Tick-page mapping and camera](../examples/crowd-2d-view/src/scene.rs)

To try a frame row in the Crowd example, change the final argument to `SpriteScene::new` from `0` to `1`.
Then replace `rows: &[]` in the scene's `SpriteFrame` with `rows: &MARKS`, using the existing glow frame.

```rust
const MARKS: [SpriteRow; 1] = [SpriteRow {
    layer: 2,
    additive: 0.6,
    ..SpriteRow::new(4, [6.0, 6.0])
}];
```

## Choose order and scale

Layers draw in increasing order. Within a layer world rows sort by value, then stable identity at equal values.
The default value is minus Y. A body lower on the screen therefore draws later in exact order. Use an explicit
value for a different anchor, such as the base of a tall sprite. A positive `order_band` in world units retains
the previous order of nearby bodies until one has passed the other by more than the band. Start at zero. Measure
swaps where bodies overlap before choosing a band for your game.

Frame rows sort by value within their layer, with input list order breaking ties. Each goes immediately before
the first world row on that layer with a strictly greater value, or at the end of the layer. With a zero band,
world rows of equal value draw before frame rows. With band 1, overlapping world rows A and B can retain order
A, B as their values move from 0 and 1 to 0.75 and 0.25. A frame row F at 0.25 then gives F, A, B. Use a higher
layer for a mark that must cover a body through jitter.

For a view that always shows a fixed world height, divide the drawable height by that height. For integer pixel
art, set the view scale to the sheet's density times a whole-number zoom. Align texel edges to drawable pixel
edges and show more world when the window grows. The engine receives the physical extent, so the same calculation
works after a Retina resize. When a drawable dimension is odd, shift the view centre by half an output pixel
along that axis to keep an integer-aligned sprite centred to the nearest pixel.

The blend happens in **linear light**. The renderer decodes the sheet's sRGB texels, applies linear tint and
flash and composites the resulting premultiplied colour before encoding the image for SDR. Midway through an
alpha blend, the encoded colour can look brighter than the arithmetic midpoint of the original RGBA8 codes.
