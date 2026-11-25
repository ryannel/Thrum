# RENDER-2D: thousands of crisp sprites, drawn without paying for anything 3D

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../VISION.md). C01, C04, C05, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md) and
[mechanism 2 of the module list](../ARCHITECTURE.md#mechanisms-settled-here) govern.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module RENDER-2D.
It needs: SHOWCASE, APPLE-TILE, CONTENT-LOOP", which is row 18 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../VISION.md) carries
as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

That page also sets two defaults, which are an agent's and not the owner's: "Top-down 2D with sprites, on V1's
targets: Mac, Chrome and the iOS simulator" and "Thousands of enemies and projectiles on screen at once".

The module list settled that [2D is its own scene half](../ARCHITECTURE.md#mechanisms-settled-here), not sprites in
the 3D forward pass, and left four choices to this planner. They are settled here:

| Choice the module list left open | Settled as | Where |
|---|---|---|
| How order is resolved: sorted rows or depth | Sorted rows, on the CPU, in an order kept from tick to tick: two rows near each other exchange places only when one has passed the other by more than a band | [Mechanism 4](#4-order) |
| Frame reconstruction for sprites | None. A 2D scene is drawn at the surface's own pixel size, with no jitter and no history, as C07's no-reconstruction form | [Mechanism 2](#2-the-view-and-the-output-pixel) |
| Whether bright effects use EDR | The frame keeps values above white and hands them to the output half. The 8-bit display mode clips them at white, and the float mode keeps them up to the display's headroom. How they look on an EDR panel is not judged here | [Mechanism 7](#7-colour) |
| How text shares the sprite path with `GAME-UI` | A glyph is a frame on a sheet and a line of text is a list of rows. `GAME-UI` runs the same pass at its own stage | [Interfaces](#interfaces) |

This design reconciles the code on main at `54ab55bb`; it is not a blind study. Read for it: `thrum-render`'s
`runtime.rs`, `extract.rs`, `contribution.rs`, `synthetic/{scene_graph,view,display,upload}.rs` and
`synthetic_display.wgsl`; `thrum-app`'s `game.rs`, `offscreen.rs` and `web.rs`; `examples/v1-game/src/content.rs`;
the designs of `APPLE-TILE`, `EFFECT-GRAPH`, `CONTENT-LOOP` and `SIM-2D`; and
[a planning probe](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md) run for it.

**This module is planned before the code it builds on is finished.** `APPLE-TILE` WP10 to WP12,
`APPLE-PRESENTATION` WP3 to WP11, `CONTENT-LOOP` WP1 to WP3 and `EFFECT-GRAPH` WP3 are still to be built. So this design
names the behaviour it needs from them and never one of their APIs, and the builder of each work package first reads
what was actually built ([handoff](#proof-and-builder-handoff)).

**Built beside V1 (14 November 2026).** The owner said: "Let's move the 2d engine side of things to before v1
and decouple them. But we don't want to start working on the swarm game till v1 lands." So this module no
longer waits for the V1 exit. A work package waits only for what it uses: WP1 for `EFFECT-GRAPH:WP3`, whose
after-opaque stage it refuses in a 2D scene, and `APPLE-PRESENTATION:WP3`, whose reconstruction a 2D scene
declares none of; WP4 for `CONTENT-LOOP:WP1`, whose lease and publication a sheet uses; WP5 for `SIM-2D:WP1`, whose
crowd it draws. `APPLE-TILE` WP10 to WP12, `APPLE-PRESENTATION` WP4 to WP12 and `CONTENT-LOOP` WP2 and WP3 are built
in the same files at the same time. Whichever lands second builds on the first and keeps its tests passing; in
particular a governor or a reconstructor that lands after WP1 gives a 2D scene no scale, no jitter and no
history, as [C07's no-reconstruction form](#2-the-view-and-the-output-pixel) says.

## Purpose and scope

A game declares a 2D scene, hands over sheets of sprites and maps its simulated entities to sprite rows. The
engine draws thousands of them crisply at the display's refresh, and prepares and runs nothing 3D.

**Consumers.** `SWARM` draws its game with it. `GAME-UI` draws text and panels with the same pass at its own
stage. `SESSION` uses the crowd view to look at each peer.

**What exists.** A 3D scene only. `FrameHost` holds one prepared 3D scene with its sky tables, Sun cascades, light
clusters, depth, motion and reactive targets. Its instance field uploads only changed rows and interpolates
translation between tick endpoints on the GPU, which the V1 game uses for 10,000 entities. The output half tones
one bound scene-linear colour into an 8-bit intermediate and presents it. Engine capture, device generations and
the tentative-frame rule are built. The content loop cooks meshes, textures and volumes and residency draws
them; lending them to another module is `CONTENT-LOOP` WP1, not built yet. There is no
orthographic view, no sprite and no sheet product. Both tone operators change colours below white, and the 8-bit
output is dithered.

**Commissioned.** The eight mechanisms below, proved by a sprite scene inside the V1 game on `mac-metal`,
`chrome-webgpu` and `ios-sim`, and by a view of `SIM-2D`'s reference crowd on `mac-metal` and `chrome-webgpu`.

**Not built.** 2D lights and shadows. Tile-map tools. Skeletal 2D animation. A particle system. Sprites inside a
3D scene, or one run that switches between a 2D and a 3D scene. A view that rotates. Culling of rows that are off
screen: every row is ordered and submitted, and the GPU clips. Snapping of positions to a pixel grid. Blend modes
other than alpha and additive. Multisampling. Nine-slice panels, text layout and the interface stage, which are
`GAME-UI`'s. An iOS app for a second game, which is
[`GAME-UI` WP6](../ARCHITECTURE.md#seam-rulings-for-game-ui), and a one-command 2D starter, which nobody plans.
Any change to the 3D scene's output.

**Words used here.** A **scene kind** is 2D or 3D. A **sheet** is a set of sprite images cooked together: its
**pages** are the layers of one array texture, and a **frame** is one named rectangle on a page. A **sprite row**
is one sprite to draw: a **world row** comes from the tick page and a **frame row** from the game each frame. An
**output pixel** is one pixel of the surface.

## What a player must see

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **C1 A crowd.** Thousands of sprites on screen, smooth at the display's refresh | 2,000, 5,000 and 10,000 moving rows, packed three deep, with spawn and despawn churn; the built-in display's full 3456×2234 pixels and 1920×1080; Chrome | A sparse crowd, a small window or the V1 sprite field. Rejected by timing the rendered replay of `SIM-2D`'s packed Crowd trace, with shaded pixels recorded beside each time. Times are baselines ([cost](#cost-account)) |
| **C2 Crisp and still.** Pixel art stays legible through a scroll, with clean edges and no visible contrast pulse or crawl | Pixel art from 1 to 6 output pixels per texel, with exact aligned whole scales, and smooth art from 0.25 to 2. Subpixel scroll and rotation use the WP3 motion scales. The output follows window size and both densities | Crisp stills can hide a pulse in motion. Too much softening can erase fine marks. Judge the same one-texel stripes and checker at normal size in elapsed-time motion and fixed stills |
| **C3 Order holds still.** Where two world rows with history overlap on one layer, the same one stays in front until one has passed the other by more than the band | Equal sort values; slots reused; rows that jitter inside the band, standing and while travelling together; three or more rows overlapping at once; sheets and blends mixed | An unstable sort. An order that only looks stable because nothing overlaps. A band on each row's own value, which lets two rows that travel together swap. A band so wide that nothing reorders. Rejected by overlapping rows of known colours read back at the overlap, by [the travelling pair](#4-order), and by a row that passes its neighbour and must exchange |
| **C4 Smooth between ticks.** Sprites glide when the display runs faster than the simulation | 60 Hz ticks on a 120 Hz display; a repeated tick; a pause; a spawn into a reused slot; a new timeline | The same pose shown twice, or a new sprite sliding in from where the slot's last owner died. Rejected by positions at known alphas, and by a reused slot that must not move |
| **C5 Sprite effects.** Frame animation in step with the world, tint, hit flash, transparency, additive glow | Any mix, interleaved in one pass | Additive drawn in a second pass, out of order. Rejected by a normal sprite over an additive one over a normal one, compared with a CPU blend |
| **C6 Nothing 3D is paid for.** | Every pass, resource and pipeline of a 2D scene | A flag that hides the 3D output while its work continues. Rejected by the compiled pass list, the generation's storage and the creation counters, with the 3D scene as a control |
| **C7 Art edits appear live.** Saving a sprite's source switches the art in one frame while the game keeps moving | A changed image; frames added, removed or repacked; a bad save | New pixels under old rectangles, or an atomic switch after a visible freeze. Rejected by all-old and all-new comparisons and elapsed-time motion through the edit |
| **C8 It survives the lifecycle.** | Resize, a change of density, a discarded frame, a capture frame, a replaced device | A target left at the old size and scaled. Rejected by equality with a fresh scene at the new size |
| **C9 A second game builds on it.** | The sprite scene in the V1 game and the crowd view | An engine crate that names a game's type. Rejected by a question on the code |

The bar is crowd legibility as in Vampire Survivors and Halls of Torment, and pixel stability as in Celeste. It is
judged in words, in the running scenes.

## Research and selection

- **A separate 2D path is current practice.** The module list cites Bevy's `core_2d`, Godot and Unity's URP. This
  design takes that as settled.
- **Bevy draws sprites as instanced quads and breaks a batch at every texture change.** Its sprite renderer
  uploads 80 bytes an instance, sorts the phase by `z` and starts a new batch whenever the image handle changes
  (`if batch_image_handle != extracted_sprite.image_handle_id`, in
  [`bevy_sprite_render` 0.18.1](https://docs.rs/crate/bevy_sprite_render/0.18.1/source/src/render/mod.rs); read 13
  November). Sprites from two images that alternate in depth order therefore cost a draw each. Godot's guide gives
  the same rule and the same remedy, an atlas
  ([batching](https://docs.godotengine.org/en/3.5/tutorials/performance/batching.html)). A crowd sorted by height
  on screen interleaves its sheets, so Thrum binds every sheet of a scene at once and the draw never breaks.
- **Premultiplied alpha is what makes edges and mixed blends right.** Filtering and premultiplied blending are both
  linear, so a filtered edge has no dark fringe, and one blend state covers alpha and additive: "It is a superset
  of both conventional and additive blending"
  ([Hargreaves](https://www.shawnhargreaves.com/blog/premultiplied-alpha.html)).
- **The pixel-art filter depends on scale and subpixel position.** The original sharp bilinear
  sample ([d7samurai](https://gist.github.com/d7samurai/9f17966ba6130a75d1bfb0f1894ed377)) implements a
  one-output-pixel box and preserves an aligned integer image. At 1.5× it also preserves high-frequency
  content that aliases on the output grid. One bright texel in a repeating three-output-pixel stripe period
  contributes weights `[0.75, 0.75, 0]` or `[0.25, 1, 0.25]` at two phases. Both sum to 1.5. The SDR reds
  are respectively `[215, 215, 21]` and `[132, 244, 132]`. The same cause is visible in the submitted
  Metal and Chrome strips and checker in [WP3's first review](../../../.orchestrate/evidence/RENDER-2D/WP3-review.md).
  A bounded, positive prefilter lowers the aliased harmonics before output sampling and blends transparent edges
  without ringing. A tent gives this roll-off with compact support. [Turkowski, Filters for Common Resampling Tasks](https://people.csail.mit.edu/ericchan/bib/pdf/ResamplingFilters.pdf)
  Pixel snapping or an integer-scaled intermediate would step motion or change the requested framing.
  Frame accumulation would add history and lag to a view drawn directly at its output extent.
  At 2× the retained Metal pan shows thinning stripes and a checker that fades to dots. The compact tent
  sharpens phases that already have strong contrast. At the texel-edge crossing a symmetric footprint
  confined to half an output pixel still gives the two texels equal weight. A 2× correction needs to
  handle those crossings as well as the phases between them.
  [2× phase crops and review](../../../.orchestrate/evidence/RENDER-2D/design-review.md)
- **Shipped games order by height with a fixed tie-break, and nothing published goes further.** The usual advice
  for two sprites that flicker is to sort on the feet and break ties with a unique identity, so equal keys never
  fall in an arbitrary order (for example
  [a Construct thread](https://www.construct.net/en/forum/construct-2/how-do-i-18/order-properly-88434) and
  [GameDev.net](https://www.gamedev.net/blog/1480/entry-2259786-flickering-overlapping-objects-common-bug/); searched
  13 November). That cures equal keys and does nothing for neighbours pushed back and forth across each other. No
  source was found for hysteresis in sprite order, so the band of [mechanism 4](#4-order) is this design's own,
  with its argument written there and [a probe](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md#4-does-a-kept-order-with-a-band-stop-reversals-and-what-does-it-cost) behind it.
- **Limits.** WebGPU's defaults allow 16 sampled textures and 8 storage buffers in a stage, 256 array layers and
  8192 texels a side ([limits](https://www.w3.org/TR/webgpu/#limits), read 13 November). The capability record is
  the authority; WP1 records what Chrome reports.
- **What would change the selection.** If the full-size scene-linear target and the two output passes cost too
  much in Chrome, a 2D recipe that writes display-encoded colour directly is designed here before anyone builds
  it. If a packed crowd's overdraw is the cost, an opaque-core pass with depth is designed here first. If WP5.3
  finds overlapping rows visibly out of order through [a chain](#4-order), a repair that moves such a row past the
  chain is designed here first; it gives up the pair rule for the rows it moves. If a real scene gives most of its
  rows new sort values in one tick often enough to show in the ordering times, an exact rebuild for such a tick,
  counted in the frame record, is designed here first.

## Mechanisms

### 1. The scene kind

A frame host is created for one scene kind, 2D or 3D, chosen at launch or page load like the culling mode. It is
not switched while the game runs. Everything around the scene is the one host as it stands: slots, the encoded
frame token, submission and completion, device generations, engine capture, timing, the output half.

A 2D scene's compiled graph holds its own passes and the output half's. With no contribution attached that is
one render pass named for the sprites, then tone, capture and present, and nothing else; in
[the float display mode](#7-colour) it is the sprite pass, then present, which applies the transfer, and capture.
A contribution that
declares only upload and simulate stages attaches through
[the existing seam](../EFFECT-GRAPH/DESIGN.md#effects-and-stages-wp2-wp3) and adds its own passes at those two
stages, before the sprite pass; no second way to declare passes is built.

The scene's generation allocates the sprite target, the row, order and frame-table buffers, and a view record. It
allocates no sky table, cascade, cluster, depth, motion, reactive, mesh, star or reconstruction resource and
creates none of their pipelines. The registry resolves no reconstruction, frame-interpolation or denoising
provider for it, on any device, and the resolved selections say so.

A request that only a 3D scene can meet is refused when the scene is prepared, with an error that names it: a
contribution that declares a shadow, opaque or after-opaque stage, a cooked placement, temporal sampling, a
culling mode, a provider of the frame contract asked for by name. It is never accepted and ignored.

The game learns the kind that was prepared, with the other
[resolved selections](../ARCHITECTURE.md#interfaces).

### 2. The view and the output pixel

The world is the plane of C01: X to the right, Y up, in world units. A 2D view is a centre, a scale in output
pixels per world unit, and the output's extent. It does not rotate. The game supplies it for each presented
frame, at that frame's interpolation alpha, as it supplies a 3D camera today; the engine keeps positions relative
to the view's origin, as 3D views do. A view whose centre or scale is not finite, or whose scale is not above
zero, refuses the frame, as an invalid 3D view does.

**One output pixel is one pixel of the surface.** The sprite target has the surface's pixel extent. There is no
render scale, no jitter, no history and no reconstruction pass, and present copies one to one. This is the
no-reconstruction form of [C07](../../design/CONTRACTS.md#c07-render-scene-views-and-temporal-data--v2-with-p4v3v4):
the view publishes one unjittered projection, its exposure and its output extent, and none of the inputs a
provider needs. When the window is
resized or moves to a display of another density, the extent changes through the frame runtime's ordinary extent
transition and the game is asked for a view at the new extent. The governor has no scale to lower for a 2D scene,
and this module registers no quality knob.

A sheet declares its density in texels per world unit, so a sprite's scale on screen is the view's scale divided
by its sheet's density. The policy is the game's: fit a fixed height of world into any window, or keep a whole
number of output pixels to a texel and show more world. The engine rounds nothing. When the scale is a whole
number and the view and a row put texel corners on pixel corners, the output is the source texels, each repeated
as a block ([mechanism 6](#6-sampling-two-kinds-of-sheet)). That is claimed for whole scales from 1 to 6, frames
up to one page and any extent a surface can have: a pixel centre is then at least a twelfth of a texel from any
texel edge, far outside the error of interpolating across a quad.

### 3. Sprite rows

A row says: where (position, rotation, scale on each axis, where a negative scale flips), what (a frame of a
sheet), in what order (a layer and a sort value), and how (a tint, a flash colour and amount, and how additive).

**World rows** are extracted from the tick page at each committed tick by the game's scene source, as 3D
instances are, and carry the entity's stable identity and slot. Rows are stored in slot order and only rows that
changed are uploaded. The vertex stage interpolates position, rotation (by the shorter arc) and scale between the
previous and the committed endpoint at the frame's alpha. Everything else steps at the tick.

A world row interpolates only when the same identity, at the same slot generation, was extracted at the previous
tick of the same timeline. Otherwise it is drawn at its committed pose. So a sprite that spawns into a reused
slot, or a world that was rewound, never slides.

**Frame animation** is the game changing a row's frame by tick. It is therefore in step with the world: it holds
during a pause and replays after a rewind.

**Frame rows** are a list the game gives with each frame, for things that are not simulated: a cursor, a damage
number, a mark. They have the same fields, no identity and no interpolation, and are drawn in the same pass and
interleaved with world rows by [mechanism 4](#4-order).

Both capacities are declared when the scene is prepared. A row beyond capacity, and a row with a value that is
not finite or a position the view's origin cannot represent, is dropped and counted, as a 3D instance is.

### 4. Order

Layers draw in increasing order. The exact world-row key within a layer is sort value then stable identity.
The sort value defaults to minus Y, so a sprite lower on the screen draws in front in exact order. A game may
supply its own value, in world units. Frame rows on a layer are sorted by value, with input list order breaking
ties.

**The band.** In a packed crowd, neighbours at nearly the same height push each other back and forth, and an exact
sort would swap them every few ticks. So the order is not rebuilt from the key at each tick. It is **kept**: one
list of every world row, back to front, carried from tick to tick and changed only by two rows that are next to
each other in it exchanging places. Which two exchange depends on whether they are **near**, which means that
their bounding circles at the committed pose, each grown by `b`, touch:

- Two near rows on one layer exchange only when the sort value of the one moving in front is more than `b` above
  the other's.
- Other world-row pairs exchange on the exact key. Layers always draw in increasing order.

Exchanges go on until no two neighbours in the list should exchange. A world row with no valid history (a spawn,
a reused slot, a new timeline) enters the list in exact key order with the rows on either side of it.
`b` is a value of the scene, in world units. At 0 the world list follows the exact key at every tick. The kept
order is presentation memory and never reaches the simulation. How the exchanges are found is the builder's.

**Frame rows.** Each frame row goes immediately before the first world row being drawn on its layer with a
strictly greater sort value, or at the end of its layer if there is none. The world order is kept and frame rows
at the same insertion point follow their sort and tie order. At zero band this puts every equal-value world row
before the frame row. With a positive band, an earlier greater value can put the frame row behind a later
equal-value world row. For example, with band 1, overlapping world rows A and B can keep order A, B as their
values move from 0 and 1 to 0.75 and 0.25. A frame row F at 0.25 then draws in order F, A, B. A mark that must
cover a body through jitter goes on a higher layer. The public row documentation and 2D guide explain this rule.

**What that guarantees**, for any `b` above 0, any number of rows and any motion, within one timeline:

1. *One total order.* The order is a list, so however many rows overlap there is no cycle and no tie.
2. *The pair rule.* Two rows with history that are near at a tick change their relative order at that tick only
   if the one moving in front is more than `b` above the other. Relative order changes only when that pair
   exchanges, and that is the only exchange the list allows a near pair.
3. *No reversal and return without real movement.* If two rows overlap at three ticks, in one order, then the
   other, then the first again, one of them moved, or changed its sort value or size, by more than `b` in
   between. Rows that overlap have circles that touch. If the two stayed near throughout, the two reversals
   needed the difference of their sort values above `b` and then below minus `b`, a swing of more than `2b`, so
   one value changed by more than `b`. If at some tick they were not near, their centres were more than `2b`
   further apart than circles that touch allow, so one moved more than `b`. So a crowd whose rows each stay
   within a range of `b` never shows an overlap flip and flip back, whatever else moves through it.
4. *It still reorders.* After every tick no two neighbours in the list are waiting to exchange: near neighbours
   are within `b` of true order and other neighbours are in exact order. A row that passes the row beside it by
   more than `b` exchanges with it at that tick.

**The travelling pair.** The design review of 13 November gave the case that broke an earlier draft, which held a
band around each row's own sort value. With `b` at 1, overlapping rows A and B have sort values 0.0 and 0.1, then
2.0 and 2.1, then 2.2 and 2.1, then 2.1 and 2.3. Each row's own band is used up by the journey, so held values
follow the true ones and the pair swaps at the third tick and back at the fourth. Under the pair rule A stays
behind B throughout, because A is never more than 0.1 above B. No filter on one row's value can do this: the rule
has to be about the pair.

**What it does not guarantee.** Rows that are not neighbours in the list can be more than `b` out of true order,
through a chain. Take three rows that all overlap, kept in the order A, B, C, with A 0.9 `b` above B and B 0.9 `b`
above C. No pair may exchange, yet A is drawn behind C from 1.8 `b` above it. No total order avoids this: putting
A in front of C means exchanging a pair that has not passed, which is the flicker the band exists to stop. This
design keeps the pair rule and gives up a bound on rows further apart. In the probe's packed crowd of 10,000,
with `b` at the range of the jitter, 0.04% of overlapping pairs were more than `b` out of order at a tick, about
one in a million more than `2b`, and the worst was 2.4 `b`. Without the near test, with the band between every
two rows, those were 1.2%, 0.18% and 4.6 `b`, and they grew with the size of the crowd, because every row at
nearly the same height joins the chain whether or not it is anywhere near. That is why the near test is part of
the rule. WP5.3 records the same figures for `SIM-2D`'s crowd.

The order is updated on the CPU at each tick and uploaded as a list of row indices. A depth buffer was not
chosen: blended edges need back-to-front drawing whatever a depth test does, so depth would add a second path for
the opaque parts only. A GPU sort was not chosen: the kept order of 10,000 rows took 121 µs a tick in the probe
([probe 4](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md#4-does-a-kept-order-with-a-band-stop-reversals-and-what-does-it-cost)).

### 5. One pass, one draw

The sprite pass has one pipeline with one blend state, source plus destination times one minus source alpha, on
colour and alpha, and no depth. Texels are premultiplied. A row's tint multiplies the texel; its flash replaces
the colour with the flash colour inside the sprite's own alpha, by the flash amount; its additive value scales the
alpha it writes toward zero, so a fully additive sprite adds its colour and hides nothing.

A scene declares up to eight sheets, and all of them are bound for the whole pass: each is one array texture of
pages, with its frame table. The count is a design constant, half of WebGPU's default for sampled textures, and a
ninth sheet is refused at preparation. Every row of the frame, whatever its sheet, layer and blend, is drawn by
one instanced draw that walks the order list. Unused slots hold a placeholder, so the pipeline and its layouts
never change when a sheet arrives, changes or leaves.

### 6. Sampling: two kinds of sheet

A sheet is **pixel** or **smooth**.

- **Pixel** sheets use spatial filtering in the sprite's projected local axes in the existing sprite
  draw. The base kernel in each axis is a one-output-pixel box. Near 1.5×, convolve it with a unit-area
  tent. Start with a tent radius of `20 (s - 1)² (2 - s)²` output pixels for axis scale `1 < s < 2`.
  Integrate the kernel over the source texel intervals for normalized, nonnegative weights. Keep this
  filter's support within 1.25 source texels on either side in each magnified axis.

  Near 2×, reconstruct over a frame-local neighbourhood using continuous projected texel coordinates.
  Cover the full off-grid transition, including the phases where output sample centres cross texel
  edges on one or both axes. The builder can vary the weights with subpixel phase or neighbouring
  texels, consider both axes jointly and use a wider footprint within the existing gutter. The
  reconstruction meets the authored sample continuously at alignment and transitions smoothly in scale
  to the surrounding filters. The draw interpolates through each projected edge across a symmetric
  shoulder. The weights compose in the sprite's two local axes to preserve the checker pattern through
  crossings on both axes. Pairwise transfers between the weights temper brightness changes in high-contrast marks.
  Reconstruction retains coverage alpha and fades to ordinary coverage colour as its contributing
  texels become transparent. The CPU overdraw estimate and the emitted standalone quad use the same
  pixel-filter support radius. Judge the original stripes and checker through repeated crossings in
  WP3.8 and tune against the still and motion inspections. Fade the magnification corrections as the
  smaller axis approaches minification, from full strength at 1.25× to zero at 1×.

  Aligned whole scales retain the original texels at 1× through 6× at every backing extent. Pixel
  sheets are never block-compressed.
- **Smooth** sheets are sampled trilinearly from mip levels, and may take the profile's compressed format as
  other textures do.

Filter premultiplied scene-linear RGBA. At 1× the finest alternating pattern lies at the output grid's
Nyquist limit. The byte guarantee is for grid alignment, and the visual claim for scrolling is judged
at the scales and viewing conditions in WP3.

Below one output pixel a texel, both kinds read mip levels, which are box-filtered in linear light from the
premultiplied texels. Frames are separated by a gutter wide enough that no mip level the sheet holds reaches a
neighbour, and the shader keeps a sample inside its own frame. The smallest scale claimed is 0.25. Below it a
sprite is still drawn, from its smallest level, and no claim is made for how it looks.

A standalone sprite's quad covers the support of the filter past its frame into the transparent gutter,
so its outline is antialiased by the same sampling. The widened magnification filter fits the existing
gutter. A frame marked as a **tile** has no expanded geometry. Filter taps past a tile edge use that frame's
extruded edge. Adjacent tiles keep their shared vertices and ownership of the output pixels along the join.

### 7. Colour

Sheets are sRGB. The cooker, or the engine for an embedded sheet, premultiplies in linear light. The sprite
target is scene-linear half-float at the output's extent, and it is the pre-tone input that
[`APPLE-TILE:WP3`](../APPLE-TILE/DESIGN.md#mechanisms) lets a producer bind, with a fixed exposure of 1.

A 2D scene selects a **display-referred transfer** in the output half: a value from 0 to 1 is sRGB-encoded with
no tone curve and no dither. An opaque texel therefore leaves the frame as the 8-bit code it was authored with.
All 256 codes survive the half-float target with 0.46 of a code to spare
([probe](../../../.orchestrate/evidence/RENDER-2D/planning-probe.md#3-do-authored-8-bit-colours-survive-the-half-float-scene-target)).

Blending is in linear light, which is the engine's one colour contract (C07). Soft art therefore looks lighter
over dark than in a paint tool that blends in display space. White at half alpha over black is code 188.
An opaque, aligned texel passes through at its authored colour.

Tints and additive sprites may push a value above 1. The target keeps it, and what leaves the frame depends on
the display mode.

**The two display modes.** The output half resolves one display mode for a generation, the 8-bit mode or the
float mode, as [APPLE-PRESENTATION](../APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence) designs them. The
display-referred transfer has a form for each. WP1 built the first. WP6 builds the second, because the EDR
surface landed after WP1 and gave a 2D scene only the 8-bit form's codes, decoded onto the float drawable.

| | The 8-bit mode | The float mode |
|---|---|---|
| **Where** | Chrome, the simulator, a Mac with no EDR display, and a Mac with SDR forced | A Mac whose surface is the `Rgba16Float` EDR surface |
| **The transfer writes** | sRGB codes into the `Rgba8Unorm` display image, each channel clipped at white. Present copies the image to the surface | The sprite target's linear values into the float target, in the present pass, with 1.0 at SDR white. The target is the drawable, or the texture that stands for it in a test or a capture. A value from 0 to 1 is written as it is. A value above 1 is kept up to the display's current headroom H and clipped at H, channel by channel. There is no tone curve and no dither |
| **An opaque texel leaves as** | Its authored 8-bit code | The linear value that the sRGB curve rounds back to that code |
| **Capture reads** | The display image | The float target the surface was given, through engine capture's tap as for a 3D host, with its encoding in the capture's record |

At H = 1 the float mode clips at white as the 8-bit mode does, so its frame, encoded to 8 bits, is the 8-bit
mode's to within one code at every pixel. H is the frame host's display headroom, which the platform publishes.
A change of H reaches the next frame and creates nothing, and no submitted value is above H.

The float form reads the sprite target itself. A float-mode generation therefore holds no 8-bit display image
and runs no 8-bit transfer: the mode costs one full-screen pass, not two. Both forms' pipelines are made when the
host is prepared, wherever the float mode can be selected, so a generation build that changes the mode compiles
nothing. A 2D host changes mode through the generation transition a 3D host uses, in each direction the output
half offers.

A capture that asks by name for the 8-bit stage on a float-mode host is either served by building that stage
for the capture, through the existing capture-stage change, or refused by name; which is the builder's. It is
never answered with another stage's pixels.

Clipping at H, not a highlight curve, is this design's choice: it is what the 8-bit mode does at white, it keeps
every value below H exact, and at H = 1 the two modes agree. It shifts the hue of a saturated highlight brighter
than the display can show. How the float frame looks on the panel is the output half's, and no item here judges
it.

### 8. Sheets

**Source.** A sheet is a folder in the scene's assets: PNG files and one description. Each PNG is one frame, named
by its file name, or a grid of equal cells named by file name and cell number. The description gives the kind,
the density in texels per world unit, the pivot of each frame (the centre if it says nothing), which frames are
tiles, and grid cell sizes. Its syntax is the builder's.

**Product.** The cooker trims each frame's transparent border, packs the frames into pages of one size, and
writes one product: the pages as one array texture, for each profile, and a frame table with each frame's name,
page, rectangle, trim, source size, pivot and tile mark. A frame larger than a page is refused by name. This is
the sprite-sheet product the module list asks of `CONTENT-LOOP`.
The cooker produces the same mip levels, transparent gutters and extruded tile edges as the embedded path.
Both products use the sampling rules in [mechanism 6](#6-sampling-two-kinds-of-sheet).

**Placement.** A row's position anchors the pivot. A cooked pivot is in original, untrimmed frame texels
(each cell is its own frame), measured from the top-left corner, X right and Y down. The default is half the
original width and height. For a corner or sample at `q` relative to the trimmed rectangle, its offset from
the pivot is `trim_origin + q - pivot`, where `trim_origin` is the rectangle's top-left in the original frame.
Convert that offset to world units using the sheet density, apply the row's signed scale and the Y-axis
conversion, then rotate counterclockwise and translate by the row position. World rows use the interpolated
pose and the committed frame. Frame rows use the supplied pose. Bounds follow the same placement, and texture
sampling stays relative to the packed rectangle. Existing embedded frames keep their top-left anchor.

**Residency.** A sheet is a [lease](../CONTENT-LOOP/DESIGN.md#leases-wp1): residency uploads the pages under its
own budget and lends them after the frame that recorded the last copy is submitted. In a 2D scene residency runs
with leases only. The frame table is read from the product the lease names, by its content hash.

**One switch.** An edited sheet's pages and its frame table change in one submitted frame, under
[the content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-wp1-wp2). No frame shows new pixels
under old rectangles. A discarded switch frame commits nothing. On Mac and Chrome, the game keeps updating and
submitting frames with the old sheet while the edit cooks, the changed product loads and the budgeted upload
stages it. Fetch and preparation must let the frame loop advance while the replacement is pending. WP4 records
the moving scene on elapsed time from before the save through first use and checks for a visible pause against
the same run's pre-save motion. The reload has no separate numerical timing budget.

**Frames by name.** A game asks a sheet for a frame by name and keeps the handle. The handle stays valid across
edits and shows whatever the current sheet gives that name, wherever the cooker packed it. A name the current
sheet lacks draws nothing and is counted in the frame's record. A sheet that is not resident yet, at first load
or after device loss, is absent: its rows draw nothing, and they all appear in one frame when it arrives.

**Embedded sheets.** A game may also hand over a sheet's images as bytes when the scene is prepared, with the
same kinds, frames and names. Tests, tools and a fallback use this, and it is how WP1 to WP3 draw before the cooked
product exists. An embedded sheet is rebuilt from its retained bytes after device loss.

### Check that it can work

- **A second producer for the output half.** The tone stage already consumes "one explicitly bound scene-linear
  colour plus its matching exposure and extent/sample identity" (`APPLE-TILE:WP3`). A 2D scene is another producer
  of that one input. The graph is already compiled per generation and variant.
- **Rows.** The 3D instance field is the precedent: 10,000 rows of 192 bytes, changed-row upload and GPU
  interpolation, in the V1 game today.
- **Order.** The four guarantees of [mechanism 4](#4-order) follow from the rule by the arguments given there.
  The probe ran the rule on the travelling pair, on 144,000 ticks of random motion with spawns and despawns, and
  on a packed crowd. Every change of relative order obeyed the pair rule. Of 8,229 times an overlapping pair
  reversed and returned, none did so without a move of more than `b`, where the earlier draft's band on each
  row's own value did so 2,805 times on the same motion. In the crowd of 10,000, overlapping pairs that reversed
  and reversed back within 30 ticks fell from 844 a tick to 7. The kept order cost 121 µs for 10,000 rows,
  against 76 µs for an exact sort.
- **One draw with eight sheets.** Eight array textures and one sampler fit WebGPU's defaults. WGSL forbids an
  implicit-derivative sample inside a branch on the sheet number; coverage sampling already computes the
  footprint itself, so derivatives are taken before the branch and the sample inside it is explicit.
- **Magnification.** The original coverage probe measured texel weight and agreed with a texel-area reference.
  WP3's first review showed why that missed the visible defect. The widened kernel retains the weight identity
  and attenuates the alias before the SDR transfer. The planning calculation in the log checks that direction.
  The device captures decide whether the retained detail and residual motion meet the visual bar.
- **Lifecycle.** A 2D scene has no history, so a resize, a discarded frame and a capture frame have nothing to
  carry or roll back beyond what the host already does for uploads.

## The system as it runs

| State | Owner |
|---|---|
| The 2D scene: its pass, view record, rows, the kept order, frame tables on the GPU, sheet slots | This module (`thrum-render`) |
| Scene-kind selection in the frame host and the split graph; the display-referred transfer's 8-bit form | `APPLE-TILE`'s frame runtime, extended under [ruling 1](#interfaces) |
| The transfer's float form in the EDR present pass, a 2D host's change of display mode, and engine capture's tap on a 2D host's float frame | `APPLE-PRESENTATION`, with `T2-CAPTURE` for the tap, extended under ruling 1 in WP6 |
| Render extent, reconstruction, the governor, and the refusal of a provider asked for a 2D scene | `APPLE-PRESENTATION`, extended under ruling 1 so a scene can declare none |
| The sheet product, its cooker and runtime view; pages as a lease; residency in a 2D scene | `CONTENT-LOOP` (`thrum-cook`, `thrum-asset`, `thrum-asset-gpu`), extended under ruling 2 |
| The contribution list and the tentative-frame protocol | `EFFECT-GRAPH`, extended under ruling 2 |
| The sprite scene in the V1 game | `SHOWCASE`'s `examples/v1-game`, extended under ruling 3 |
| The crowd view | This module (`examples/crowd-2d-view`, game content) |
| The reference crowd, its rules and traces | `SIM-2D` |
| The tick, committed state, the tick page | `thrum-core` |

**A frame, in order.** The host reads the tick page. On a new tick the game's scene source extracts world rows;
the engine updates the kept order and uploads changed rows and the order list. The game
gives the view for this frame's alpha and its frame rows. Residency's upload pass copies. The sprite pass clears
the target to the scene's background colour and draws once. Tone encodes it, capture reads it if asked, and present copies it to the surface. In
the float mode present writes the transfer into the float target and capture reads that target. On
submission residency commits and a sheet switch takes effect; on discard nothing commits.

**An artist's journey.** Put PNG files and a description in a folder; the cooker packs them and the game shows
them. Save a changed PNG: the watcher cooks that sheet, residency uploads its pages, and one frame later every
sprite shows the new art. Add a frame: handles to the other names still show the same pictures.

**A game's journey.** Declare a 2D scene with its sheets and capacities. Map components to rows in the scene
source. Return a view each frame. Nothing else in the game names a renderer type.

## Interfaces

Provided, as [the module list](../ARCHITECTURE.md#interfaces-track-3) names them:

| Interface | To | What it is |
|---|---|---|
| **Sprite rows** | `SWARM`, any 2D game | Mechanisms 2 to 5: the view, world rows and frame rows, the order key and band |
| **Sheets** | `SWARM`, `GAME-UI` | Mechanisms 6 to 8: the source folder, the two kinds, frames by name, embedded sheets |
| **The sprite pass at another stage** | `GAME-UI` | The pass takes its target, view and rows as inputs. `GAME-UI` runs a second instance at the interface stage with a view in output pixels and its paint list as rows. A glyph is a frame, so a font product cooks to a sheet. This module proves only the scene's instance |
| **Crowd view** | `SESSION` | A program that draws `SIM-2D`'s reference crowd, as a diagnostic |

Asked of other modules. Each is new work inside state another module owns, settled as a
[seam ruling in the module list](../ARCHITECTURE.md#seam-rulings-for-render-2d): this module's builder builds it,
on what the owning module has landed by then, with the behaviour defined here, and it then belongs to the owner.

1. **The frame runtime** (`APPLE-TILE`, `APPLE-PRESENTATION`): scene-kind selection, the display-referred
   transfer, and a scene that declares no reconstruction. Built in WP1. The transfer's
   [float form](#7-colour), a 2D host's change of display mode and its capture are built in WP6.
2. **Content** (`CONTENT-LOOP`, `EFFECT-GRAPH`): the sheet product, pages as a lease, and residency and the
   contribution list in a 2D scene. WP4 extends the texture cooker in `thrum-cook` and runtime views in
   `thrum-asset`. The game connects `thrum-asset-gpu` residency to the sprite scene, so `thrum-render` can
   consume the views without a residency dependency.
3. **The V1 game and its shells** (`SHOWCASE`, `T2-CAPTURE`, `PLATFORM-RUNTIME-IOS`): a `sprites` scene
   selection, off by default, that the window, the page, the iOS shell and the capture scripts accept. Built in
   WP1, and for the simulator in WP5.

**Contracts.** One contract changed for this design, on 13 November 2026, settled in the module list under
ruling 1. C07 and [the hardware-native frame contract](../../design/decisions/HARDWARE-NATIVE-ENGINE.md#frame-contract)
required every view to publish jittered and previous projections, motion vectors, depth, a reactive mask and
reset flags, which a 2D scene never makes. They now name two forms:

| Form | Declared by | Publishes | Providers |
|---|---|---|---|
| Full | Every view of a 3D scene, as before | Everything C07 listed | Selected through the registry, as before |
| No reconstruction | A view of a 2D scene, when the scene is prepared | One unjittered projection, exposure and output extent. No jitter, previous projection, motion, depth, reactive mask, reset flag or guide buffer, and none is allocated. No history key and no render scale | None is resolved, on any device. One asked for by name is refused by name at preparation, never ignored and never fed made-up inputs |

The frame runtime (`APPLE-TILE`) produces the declaration and `APPLE-PRESENTATION` consumes it, in its registry
and governor; both sides are built in WP1 under ruling 1. No 3D view's obligations change, so nothing a V1 module
builds or has built changes. No other contract's text changes. The scene kind is a load-time selection in C04's
sense, not a live change. C05's preview clause is met as `CONTENT-LOOP` meets it. E08 of the architecture brief
still puts pure 2D outside scope, which the owner's page overrides; rewording it is the owner's.

## Cost account

No budget is set (brief, decision 3). Estimates, to be replaced by measurements:

| Work | Estimate |
|---|---|
| Row storage | About 64 bytes a row (Bevy's is 80): 640 KB at 10,000 |
| Upload a tick | Changed rows only. A crowd where everything moves uploads all of them, about 38 MB a second at 60 Hz, and a 40 KB order list |
| Order a tick | 121 µs at 10,000 natively for the kept order in the probe's crowd, where an exact sort is 76 µs; Chrome not measured. The worst case is a tick that gives every row with history a new value: about 25 million exchanges and 10 ms at 10,000. Rows without history are sorted, not exchanged in, so a rewind or a wave of spawns does not pay it |
| Draw | One call; four vertices a row |
| Fill | The sprite target at 3456×2234 is 7.7 million pixels: 62 MB as half-float, plus the output half's 31 MB intermediate. The float mode holds no intermediate, and its drawable is half-float. A crowd three deep shades about three times its own area |
| Pixel filter | The widened 1.5× footprint meets at most four source texels per axis. Pair adjacent positive weights for up to four hardware bilinear samples per affected fragment. The aligned and base box paths can use one sample. The off-grid 2× cost depends on the chosen neighbourhood and scales with shaded pixels and overdraw. Reuse level zero, the sampler and the sheet bindings, and derive scale-dependent terms once per quad where possible |
| A sheet | 16 MiB a 2048² page uncompressed, a third more with mips |
| 3D work | None |

**Tripwires**, flagged in the record and investigated while building goes on; none is a pass condition:

- a frame whose CPU and GPU time together exceed the display's refresh interval at 10,000 rows on the Mac;
- cost that grows faster than rows times shaded pixels;
- a row about a quarter slower than the last comparable one;
- about 25% paired regression on the valley route with no 2D scene.

Whether motion is smooth on the built-in panel itself is not measured here. That needs an attended run, which
`PLATFORM-RUNTIME-MAC` owns for the 3D path; for 2D it is owed and the ledger says so.

## Risks

1. **A 2D change moves a 3D image or breaks a lifecycle rule.** The 3D kind is the default and must compile
   to the same passes and bytes. Fix a regression before continuing with 2D work.
2. **A mechanism needs a change to the shared runtime.** Reuse the frame host, presentation path and content
   loader. If the shared runtime cannot support the mechanism, record the gap in the log and end `BLOCKED`.
3. **The full-size target is the cost in Chrome.** Early sign: WP1's cost rows. The answer is under research.
4. **Overdraw of a packed crowd.** Early sign: WP2's and WP5's rows. The answer is under research.
5. **Jitter is larger than a band that still looks right, or chains leave overlapping rows visibly out of
   order.** Early sign: WP5.3's reversal and out-of-order figures and the inspected crowd. The band is set from the
   measured jitter; if no value serves, the order is redesigned here, starting from the repair under
   [research](#research-and-selection).
6. **Exact colour fails by a code on one target**, from the GPU's sRGB decode or a fast-math `pow`. Early sign:
   WP1.5. Stop and diagnose; do not widen the comparison silently.
7. **The lease is not the shape a sheet needs.** `CONTENT-LOOP` WP1 is unbuilt. Early sign: WP4's first reading.
8. **C2 can appear to pass** in crisp stills of an aligned card or in a scroll that washes away the checker.
   Compare the original one-texel patterns in motion and keep isolated marks legible.
9. **A convincing fake of C4:** frames captured at the tick rate. The proof reads positions at alphas between
   ticks.
10. **Linear-light blending surprises an artist.** It is stated in the guide, and it is an assumption in the plan
    for the owner to overturn before `SWARM` makes art.

## Proof and builder handoff

- **Targets.** `mac-metal`, on both native recipes where an item says so. `chrome-webgpu` through
  `scripts/check_v1_browser.mjs` for the sprite scene and through this module's own script for the crowd view;
  both need a browser launch the Codex sandbox grants per run. `ios-sim` for function only, in WP5, through the
  existing iOS shell.
- **Scenes.** The **sprite scene** is a selection in `examples/v1-game`, off by default: a test card of static
  rows, and the game's 10,000 simulated entities as world rows. It reaches all three shells and their capture
  scripts, as the other modules' diagnostics do. The **crowd view** is a new package beside `SIM-2D`'s rule
  package, on `thrum-app`'s window and page.
- **Art.** Test cards are generated in code. The few sprites the scenes need are drawn by the builder, original
  and simple. Nothing is lifted from another game or from Broadsides.
- **Generation inventory.** WP3.12 lands first. Its one-frame sheet has a 12×12 atlas with gutters and one
  32-byte frame record. Keep the existing 64×48 Metal witness and 3D control. The device run observes live
  allocations, actual pipeline creation and submitted pass timings at the point this test guards.
  The checkpoint uses the focused inventory check. WP3.13 uses the shared `long-tests` result on main
  covering the completed work package, including the test-only checkpoint.
- **Exact sprite checks.** The aligned image, resize, tile and mip witnesses use expectations taken from
  the source texels and this design's rules. No sprite image is registered as an exact baseline. These tests
  are this module's under [rule 1 of exact baselines](../ARCHITECTURE.md#exact-baselines).
- **Mac window resize (WP3.6).** The unattended check records AppKit points, scale and backing extent for the
  three requested window sizes and the resize transition. When the surface supplies drawables, join each
  window's presented submission and the first presented submission after resize to the engine-captured pixels
  by submission ID. Check extents, one-to-one present and the aligned card. A run with observed surface
  occlusion and no drawable for a required sample can report geometry only for that sample.
  A timeout, renderer error or missing geometry is a check
  failure. Console state is recorded as metadata. The reviewer needs a complete presented-window record covering
  the landed path, from a run with a drawable. Use a retained record if it covers that path. For a new run,
  preflight for a visible window before taking the GPU lock. This uses the engine tap and the shell's presentation
  record, with no desktop capture or panel claim. Density 1 is exercised in the extent and Chrome proofs.
- **The float mode (WP6).** In `crates/thrum-render/src`, read `synthetic/display/edr.rs`,
  `runtime/presentation.rs`, `runtime/submitted_capture.rs` and `runtime/extent.rs` first. The 3D host's EDR present, its late copy of the
  drawable and its change of display mode are the route to reuse, and each stops at a 3D host today. Build no
  second present path and no second tap. In a GPU test a float texture given to the frame host as its surface
  stands for the EDR drawable, as `APPLE-PRESENTATION`'s capture tests do it, and the V1 game's proof forces the
  display record as that module's arms do, so nothing needs the panel awake. New tests go in the sprite and 2D
  scene test binaries that exist. No item of `APPLE-PRESENTATION` or `T2-CAPTURE` changes, and a 3D host compiles
  the same passes and draws the same bytes in both modes.
- **Capture.** Inspected items use T2 engine capture and name their mode. Motion is judged from `realtime`
  captures at 1920×1080 as consecutive frames at 1x; the display's full 3456×2234 is captured as `fixed` stills
  and timed in the cost records, because twenty seconds of it would not fit capture's output bound.
- **Camera proof and diagnostics.** WP3.2 and WP3.7 step the camera over retained rows and test that the
  submitted pixels follow it. Scene-linear texel sums and phase contrast can help diagnose a capture.
  The old Metal and Chrome coverage-error limits belong to the box-filter run and are retired as checks.
- **Filter cost.** Use the existing profile record for the WP3.1 comparison of sprite-pass timings before
  and after the change at 1920×1080 on Metal and Chrome for the 1.5× scroll. Include recipe, device, shaded
  pixels, overdraw and load. The timings are tripwires. WP5 measures the packed crowd.
- **Cost records.** WP5.4 and WP5.5 launch the crowd view with `SIM-2D`'s packed Crowd trace at 2,000, 5,000 and
  10,000 bodies. In each timed take and size, match retained digests at ticks 120 and 240, and link timing and
  overdraw samples in that interval to the submissions and tick publications of that same replay. Record build,
  revision, backing extent and machine load. Keep the trace's packing and the view's sprite size when
  investigating a tripwire. Record the observed cost and finding, then continue the build. The V1 sprite field
  keeps its WP2 cost command.
- **Exact baselines.** This module edits `crates/`, `examples/v1-game/` and `scripts/`, which are covered paths,
  and moves no retained image. Each work package uses the shared main-branch verifier. Diagnose a failed
  source-texel comparison against its original expectation. If another module owns a failing test, baseline
  or witness that requires a change, retain it and log the proposed fix before ending `BLOCKED`.
  [Project proof rules](../../../ORCHESTRATE.md#targets)
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/render-2d-<work-package>/`; selected evidence
  in `.orchestrate/evidence/RENDER-2D/`.
- **Creator guide and maturity.** WP5 updates [the creator guide](../../design/ENGINE-EXPERIENCE.md) and adds
  2D rows to [the maturity ledger](../../design/maturity.json) for implementation, correctness, visuals and
  device results.

## Open questions

| Question | Settled by |
|---|---|
| What do the full-size target and the two output passes cost on the Mac and in Chrome? | WP1 records it |
| Is the widened filter sharp enough with no visible contrast pulse? | WP3.8–WP3.10 inspect the submitted images |
| How much does `SIM-2D`'s crowd jitter, what band hides it, and how far out of order do chains leave overlapping rows? | WP5.3 measures all three and sets the band |
| Does a packed crowd of 10,000 fit the refresh interval at the display's full size? | WP5 records it; a miss is a finding, not a stop |
| Does `SWARM` want more than eight sheets, or display-space blending? | Its planner asks here |
