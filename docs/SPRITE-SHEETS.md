# Cooked sprite sheets

Put PNGs and a `sheet.json` in a folder below the cooker's source root. The folder's relative
path is the sheet's manifest name. The PNGs immediately in that folder belong to the sheet.
A PNG normally produces a frame named by its filename without the extension. A grid produces
one frame per cell, with zero-based suffixes in row-major order. For example, `walk.png` with
two columns creates `walk.0` and `walk.1` in its first row.

Here is a description for an `actors` folder containing `idle.png`, `walk.png` (an exact multiple
of 24×24), and `floor.png`.

```json
{
  "kind": "pixel",
  "density": 24,
  "grids": { "walk.png": [24, 24] },
  "pivots": { "idle": [12, 22], "walk.0": [12, 22], "walk.1": [12, 22] },
  "tiles": ["floor"]
}
```

- **Pixel** keeps source texels uncompressed and uses coverage filtering. Align the rendered grid
  to whole output pixels for exact whole-scale sprites.
- **Smooth** uses trilinear filtering. The cooker can encode these pages as BC7 or ASTC for a
  compatible profile. Portable pages stay uncompressed.
- **Density** is the source texel count per world unit and must be finite and positive.
- **Tiles** keep the full cell or image extent and extrude edge texels into their gutters.
  Align neighbouring tiles by their authored extents.
- Optional **page_size** caps the width and height of each page. It defaults to 2048 and accepts
  multiples of four from 12 to 2048. Each rectangle must fit with its aligned gutter.
- Each **pivot** is an X-right, Y-down point in original-frame texels measured from the top-left
  corner of the untrimmed image or cell. If omitted, it is half the original width and height.
  Author the same foot or other landmark as the pivot of each animation frame.

PNG colour enters as straight-alpha sRGB. The cooker converts colour to linear light and
premultiplies it by alpha before filtering the mip levels. The sprite pass composites in linear
light. A half-transparent colour will therefore blend differently from an editor compositing
directly on sRGB codes.

## Use a sheet in a 2D scene

With a `CookedConfig` pointing at a mapped store or a fetched feed, call
`config.lease_sheet("actors")`, then construct `CookedAssets::new(config)`. Put any embedded
sheets in the `SpriteScene` first. Connect `scene.set_sheet_source(assets.lease())` and call
`scene.add_cooked_sheet("actors")?`. Keep the returned sheet's `slot()` and its `frame("walk.0")`
handle, then assign them to `SpriteRow::sheet` and `SpriteRow::frame`. Prepare the 2D host with
this scene and attach `assets` as a scene contribution. A scene may register up to eight sheets.

The row's world position is the cooked frame's pivot. If transparent pixels are trimmed, the
renderer retains the trim origin and places the cropped texel at
`trim_origin + texel_in_rectangle - pivot` relative to the row. Signed scale and rotation act
around that pivot. This works for interpolated world rows and for frame rows. An embedded
frame still uses its rectangle's top-left corner as the anchor.

A kept handle follows the same name through edits, even if its picture moves to another page.
A pending sheet or absent frame draws no sprite and is counted in that frame. After a good
save the renderer switches the new pages and the new table together. The watcher reports
source failures and residency reports rejected products while the game continues to draw
the last good publication.

## Try the V1 sprite field

Generate its original three-cell `field/walk.png` and two sample edits, then start the watcher.

```sh
python3 scripts/sprite_sheet_fixture.py tmp/my-sheets/source tmp/my-sheets/edits
cargo run --locked --offline -p thrum-cook -- watch tmp/my-sheets/source tmp/my-sheets/store --serve 127.0.0.1:7878
```

In another terminal, open the Mac sprite field.

```sh
THRUM_SCENE=sprites THRUM_SPRITE_ROWS=500 THRUM_SPRITE_SHEETS_STORE=tmp/my-sheets/store \
  cargo run --locked --offline -p v1-game
```

For Chrome, [set up the Wasm tools](../examples/starter/README.md), build and serve the proving
page, then open the URL with the cooker's origin.

```sh
sh examples/v1-game/web/build.sh --proving
python3 -m http.server 8765 --bind 127.0.0.1 --directory target/v1-game-web-proving
```

Open [the moving sprite field](http://127.0.0.1:8765/?scene=sprites&rows=500&assets=http%3A%2F%2F127.0.0.1%3A7878)
and replace `source/field/walk.png` with a saved revision. The field uses `walk.0`, `walk.1`
and `walk.2`. The generated `edits/recolour.png` and `edits/repack.png` can each be saved as
`walk.png` to see a live switch.


## Record the live switches

The [Mac capture driver](../scripts/prove_sprite_sheets_mac.py) replaces the PNG twice while the
T2 engine capture records the moving field. Use the project's GPU lock for the Mac run and the
Chrome lock for the browser check, which records its single recolour through the same path it
qualifies. Build `v1-game` and `thrum-cook` before recording on the Mac.

```sh
python3 scripts/prove_sprite_sheets_mac.py tmp/my-sheet-mac
node scripts/check_v1_browser.mjs --case sprite-sheets --output tmp/my-sheet-chrome
```

Run the [capture packager](../scripts/review_sprite_sheet_capture.py) with the capture directory
(`tmp/my-sheet-mac/capture` or `tmp/my-sheet-chrome`) and a new output directory. It preserves
submission gaps at 1× and saves the eight frames around each switch as whole views and
native-size crops.
