# RENDER-2D: thousands of crisp sprites, drawn without paying for anything 3D

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C01, C04, C05, C06, C07 and C12 in [CONTRACTS](../../design/CONTRACTS.md) and
[mechanism 2 of the module list](../ARCHITECTURE.md#mechanisms-settled-here) govern.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module RENDER-2D.
It needs: SHOWCASE, APPLE-TILE, CONTENT-LOOP", which is row 18 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../BRIEF.md) carries
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
| How order is resolved: sorted rows or depth | Sorted rows, on the CPU, with a band that stops jitter from reordering | [Mechanism 4](#4-order) |
| Reconstruction for sprites | None. A 2D scene is drawn at the surface's own pixel size, with no jitter and no history | [Mechanism 2](#2-the-view-and-the-output-pixel) |
| Whether bright effects use EDR | The frame keeps values above white and hands them to the output half. SDR output clips them. How they look on an EDR panel is not judged here | [Mechanism 7](#7-colour) |
| How text shares the sprite path with `GAME-UI` | A glyph is a frame on a sheet and a line of text is a list of rows. `GAME-UI` runs the same pass at its own stage | [Interfaces](#interfaces) |

This design reconciles the code on main at `54ab55bb`; it is not a blind study. Read for it: `thrum-render`'s
`runtime.rs`, `extract.rs`, `contribution.rs`, `synthetic/{scene_graph,view,display,upload}.rs` and
`synthetic_display.wgsl`; `thrum-app`'s `game.rs`, `offscreen.rs` and `web.rs`; `examples/v1-game/src/content.rs`;
the designs of `APPLE-TILE`, `EFFECT-GRAPH`, `CONTENT-LOOP` and `SIM-2D`; and
[a planning probe](../../evidence/RENDER-2D/planning-probe.md) run for it.

**This module is planned before the code it builds on is finished.** `APPLE-TILE` M10 to M12,
`APPLE-PRESENTATION` M3 to M11, `CONTENT-LOOP` M1 to M3 and `EFFECT-GRAPH` M3 are still to be built. So this design
names the behaviour it needs from them and never one of their APIs, and the builder of each milestone first reads
what was actually built ([handoff](#proof-and-builder-handoff)).

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
them; lending them to another module is `CONTENT-LOOP` M1, not built yet. There is no
orthographic view, no sprite and no sheet product. Both tone operators change colours below white, and the 8-bit
output is dithered.

**Commissioned.** The eight mechanisms below, proved by a sprite scene inside the V1 game on `mac-metal`,
`chrome-webgpu` and `ios-sim`, and by a view of `SIM-2D`'s reference crowd on `mac-metal` and `chrome-webgpu`.

**Not built.** 2D lights and shadows. Tile-map tools. Skeletal 2D animation. A particle system. Sprites inside a
3D scene, or one run that switches between a 2D and a 3D scene. A view that rotates. Culling of rows that are off
screen: every row is ordered and submitted, and the GPU clips. Snapping of positions to a pixel grid. Blend modes
other than alpha and additive. Multisampling. Nine-slice panels, text layout and the interface stage, which are
`GAME-UI`'s. A window, page or iOS app for a second game, and a one-command 2D starter, which are
[`CREATOR`'s ask for `SWARM`](../ARCHITECTURE.md#what-track-3-asks-of-existing-modules). Any change to the 3D
scene's output.

**Words used here.** A **scene kind** is 2D or 3D. A **sheet** is a set of sprite images cooked together: its
**pages** are the layers of one array texture, and a **frame** is one named rectangle on a page. A **sprite row**
is one sprite to draw: a **world row** comes from the tick page and a **frame row** from the game each frame. An
**output pixel** is one pixel of the surface.

## What a player must see

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **C1 A crowd.** Thousands of sprites on screen, smooth at the display's refresh | 2,000, 5,000 and 10,000 moving rows, packed three deep, with spawn and despawn churn; the built-in display's full 3456×2234 pixels and 1920×1080; Chrome | A sparse crowd, or a small window. Rejected by `SIM-2D`'s packed crowd at the display's pixel size, with shaded pixels recorded beside each time. Times are baselines ([cost](#cost-account)) |
| **C2 Crisp and still.** A texel never changes size, swims or sparkles; edges are clean | Scales from 1 to 6 output pixels a texel for pixel art, whole and fractional; 0.25 to 2 for smooth art; any sub-pixel position; view scrolling slowly and fast; rotated sprites; every window size and both densities | Crisp at one window size. Nearest sampling, which is sharp in a still and shimmers in motion. Rejected by a box-filter oracle over scroll offsets, with nearest sampling as a control that must fail |
| **C3 Order holds still.** Where two sprites overlap, the same one stays in front until they really pass each other | Equal sort values; slots reused; rows that jitter by less than the band; sheets and blends mixed | An unstable sort, or an order that only looks stable because nothing overlaps. Rejected by overlapping rows of known colours read back at the overlap |
| **C4 Smooth between ticks.** Sprites glide when the display runs faster than the simulation | 60 Hz ticks on a 120 Hz display; a repeated tick; a pause; a spawn into a reused slot; a new timeline | The same pose shown twice, or a new sprite sliding in from where the slot's last owner died. Rejected by positions at known alphas, and by a reused slot that must not move |
| **C5 Sprite effects.** Frame animation in step with the world, tint, hit flash, transparency, additive glow | Any mix, interleaved in one pass | Additive drawn in a second pass, out of order. Rejected by a normal sprite over an additive one over a normal one, compared with a CPU blend |
| **C6 Nothing 3D is paid for.** | Every pass, resource and pipeline of a 2D scene | A flag that hides the 3D output while its work continues. Rejected by the compiled pass list, the generation's storage and the creation counters, with the 3D scene as a control |
| **C7 Art edits appear live.** Saving a sprite's source changes the running game in one frame | A changed image; frames added, removed or repacked; a bad save | A reload that restarts the scene, or new pixels under old rectangles. Rejected by frames compared with all-old and all-new references |
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
- **Pixel art that moves smoothly is sampled by coverage.** The sample is moved to the texel centre except within
  one output pixel of a texel edge, where a linear sampler ramps across it
  ([d7samurai](https://gist.github.com/d7samurai/9f17966ba6130a75d1bfb0f1894ed377), who asks for premultiplied
  textures and a transparent border; the same method ships in emulators as "sharp bilinear"). The older way is to
  draw to a low-resolution target and scale it by a whole number, as Celeste is reported to do at 320×180. That
  is exact, and it ties motion and window size to whole texels. Coverage sampling gives the same exact image when
  a game does align to whole texels, and a stable one when it does not, so it is chosen.
- **Limits.** WebGPU's defaults allow 16 sampled textures and 8 storage buffers in a stage, 256 array layers and
  8192 texels a side ([limits](https://www.w3.org/TR/webgpu/#limits), read 13 November). The capability record is
  the authority; M1 records what Chrome reports.
- **What would change the selection.** If the full-size scene-linear target and the two output passes cost too
  much in Chrome, a 2D recipe that writes display-encoded colour directly is designed here before anyone builds
  it. If a packed crowd's overdraw is the cost, an opaque-core pass with depth is designed here first.

## Mechanisms

### 1. The scene kind

A frame host is created for one scene kind, 2D or 3D, chosen at launch or page load like the culling mode. It is
not switched while the game runs. Everything around the scene is the one host as it stands: slots, the encoded
frame token, submission and completion, device generations, engine capture, timing, the output half.

A 2D scene's compiled graph holds its own passes and the output half's, and nothing else: an upload pass when a
contribution is attached, one render pass named for the sprites, then tone, capture and present. Its generation
allocates the sprite target, the row, order and frame-table buffers, and a view record. It allocates no sky
table, cascade, cluster, depth, motion, reactive, mesh, star or reconstruction resource and creates none of their
pipelines.

A request that only a 3D scene can meet is refused when the scene is prepared, with an error that names it: a
contribution that declares a shadow, opaque or after-opaque stage, a cooked placement, temporal sampling, a
culling mode. It is never accepted and ignored. A contribution that declares only upload and simulate stages
attaches to a 2D scene through [the existing seam](../EFFECT-GRAPH/DESIGN.md#effects-and-stages-m2-m3); no second
way to declare passes is built.

The game learns the kind that was prepared, with the other
[resolved selections](../ARCHITECTURE.md#interfaces).

### 2. The view and the output pixel

The world is the plane of C01: X to the right, Y up, in world units. A 2D view is a centre, a scale in output
pixels per world unit, and the output's extent. It does not rotate. The game supplies it for each presented
frame, at that frame's interpolation alpha, as it supplies a 3D camera today; the engine keeps positions relative
to the view's origin, as 3D views do. A view whose centre or scale is not finite, or whose scale is not above
zero, refuses the frame, as an invalid 3D view does.

**One output pixel is one pixel of the surface.** The sprite target has the surface's pixel extent. There is no
render scale, no jitter, no history and no reconstruction pass, and present copies one to one. When the window is
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
the same order as world rows.

Both capacities are declared when the scene is prepared. A row beyond capacity, and a row with a value that is
not finite or a position the view's origin cannot represent, is dropped and counted, as a 3D instance is.

### 4. Order

Rows are drawn back to front by one total key: layer, then sort value, then stable identity. A frame row has no
identity, so at an equal layer and sort value it follows every world row, in its place in the list. The sort
value defaults to minus Y, so a sprite lower on the screen is in front; a game may supply its own. Equal values
never fall in an arbitrary order, because the last term breaks the tie.

**The band.** In a packed crowd, neighbours at nearly the same height push each other back and forth, and an exact
sort would swap them every few ticks. Each world row therefore keeps a held sort value that moves only when the
true value leaves a band around it: when the true value is more than `b` above or below the held one, the held
one follows at distance `b`. The key uses the held value. A row that jitters by less than `b` never changes
place, and a row that really travels is ordered at most `b` late. `b` is a value of the scene, in world units; 0
turns the band off. It is presentation memory: it is reset when a row's history is invalid, and it never reaches
the simulation.

The order is rebuilt on the CPU whenever rows change, and uploaded as a list of row indices. A depth buffer was
not chosen: blended edges need back-to-front drawing whatever a depth test does, so depth would add a second path
for the opaque parts only. A GPU sort was not chosen: the CPU orders 10,000 rows in 71 µs
([probe](../../evidence/RENDER-2D/planning-probe.md#2-what-does-ordering-thousands-of-rows-cost-on-the-cpu)).

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

- **Pixel** sheets are sampled by coverage. The sample goes to the texel centre, except within half an output
  pixel of a texel edge, where it ramps across the edge. For axis-aligned sprites at a scale of 1 or more this is
  the exact area of each texel under each output pixel, so every texel keeps the same weight on screen at every
  sub-pixel offset, and at a whole-number scale on the pixel grid it is the texel itself. A rotated sprite gets
  an edge one output pixel wide. Pixel sheets are never block-compressed.
- **Smooth** sheets are sampled trilinearly from mip levels, and may take the profile's compressed format as
  other textures do.

Below one output pixel a texel, both kinds read mip levels, which are box-filtered in linear light from the
premultiplied texels. Frames are separated by a gutter wide enough that no mip level the sheet holds reaches a
neighbour, and the shader keeps a sample inside its own frame. The smallest scale claimed is 0.25. Below it a
sprite is still drawn, from its smallest level, and no claim is made for how it looks.

A sprite's quad reaches one texel past its frame into transparent texels, so its outline is antialiased by the
same sampling and needs no multisampling. A frame marked as a **tile** has no such margin: two tiles that share
an edge share its vertices, and no background shows between them at any scale or offset.

### 7. Colour

Sheets are sRGB. The cooker, or the engine for an embedded sheet, premultiplies in linear light. The sprite
target is scene-linear half-float at the output's extent, and it is the pre-tone input that
[`APPLE-TILE:M3`](../APPLE-TILE/DESIGN.md#mechanisms) lets a producer bind, with a fixed exposure of 1.

A 2D scene selects a **display-referred transfer** in the output half: a value from 0 to 1 is sRGB-encoded with
no tone curve and no dither. An opaque texel therefore leaves the frame as the 8-bit code it was authored with.
All 256 codes survive the half-float target with 0.46 of a code to spare
([probe](../../evidence/RENDER-2D/planning-probe.md#3-do-authored-8-bit-colours-survive-the-half-float-scene-target)).

Blending is in linear light, which is the engine's one colour contract (C07). Soft art therefore looks lighter
over dark than in a paint tool that blends in display space: white at half alpha over black is code 188, not
128. Hard-edged pixel art is unaffected.

Tints and additive sprites may push a value above 1. The target keeps it. SDR output clips each channel at white,
as every SDR 2D game does. Where the output half has resolved an EDR display mode, the value reaches it
unchanged; how it then looks on the panel is the output half's, and no item here judges it.

### 8. Sheets

**Source.** A sheet is a folder in the scene's assets: PNG files and one description. Each PNG is one frame, named
by its file name, or a grid of equal cells named by file name and cell number. The description gives the kind,
the density in texels per world unit, the pivot of each frame (the centre if it says nothing), which frames are
tiles, and grid cell sizes. Its syntax is the builder's.

**Product.** The cooker trims each frame's transparent border, packs the frames into pages of one size, and
writes one product: the pages as one array texture, for each profile, and a frame table with each frame's name,
page, rectangle, trim, source size, pivot and tile mark. A frame larger than a page is refused by name. This is
the sprite-sheet product the module list asks of `CONTENT-LOOP`.

**Residency.** A sheet is a [lease](../CONTENT-LOOP/DESIGN.md#leases-m1): residency uploads the pages under its
own budget and lends them after the frame that recorded the last copy is submitted. In a 2D scene residency runs
with leases only. The frame table is read from the product the lease names, by its content hash.

**One switch.** An edited sheet's pages and its frame table change in one submitted frame, under
[the content publication](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2). No frame shows new pixels
under old rectangles. A discarded switch frame commits nothing.

**Frames by name.** A game asks a sheet for a frame by name and keeps the handle. The handle stays valid across
edits and shows whatever the current sheet gives that name, wherever the cooker packed it. A name the current
sheet lacks draws nothing and is counted in the frame's record. A sheet that is not resident yet, at first load
or after device loss, is absent: its rows draw nothing, and they all appear in one frame when it arrives.

**Embedded sheets.** A game may also hand over a sheet's images as bytes when the scene is prepared, with the
same kinds, frames and names. Tests, tools and a fallback use this, and it is how M1 to M3 draw before the cooked
product exists. An embedded sheet is rebuilt from its retained bytes after device loss.

### Check that it can work

- **A second producer for the output half.** The tone stage already consumes "one explicitly bound scene-linear
  colour plus its matching exposure and extent/sample identity" (`APPLE-TILE:M3`). A 2D scene is another producer
  of that one input. The graph is already compiled per generation and variant.
- **Rows.** The 3D instance field is the precedent: 10,000 rows of 192 bytes, changed-row upload and GPU
  interpolation, in the V1 game today.
- **Order.** 71 µs for 10,000 rows natively. The band is a comparison and an assignment per row.
- **One draw with eight sheets.** Eight array textures and one sampler fit WebGPU's defaults. WGSL forbids an
  implicit-derivative sample inside a branch on the sheet number; coverage sampling already computes the
  footprint itself, so derivatives are taken before the branch and the sample inside it is explicit.
- **Coverage.** The probe compared it with the exact box filter at ten scales and 200 offsets: equal to rounding
  error at every scale of 1 or more, while nearest sampling made a texel 2 or 3 pixels wide by turns at 2.37.
  A GPU blends with fewer weight bits than the probe, so M3 measures the renderer's own difference first.
- **Lifecycle.** A 2D scene has no history, so a resize, a discarded frame and a capture frame have nothing to
  carry or roll back beyond what the host already does for uploads.

## The system as it runs

| State | Owner |
|---|---|
| The 2D scene: its pass, view record, rows, order, band memory, frame tables on the GPU, sheet slots | This module (`thrum-render`) |
| Scene-kind selection in the frame host and the split graph; the display-referred transfer | `APPLE-TILE`'s frame runtime, extended under [ruling 1](#interfaces) |
| Render extent, reconstruction, the governor | `APPLE-PRESENTATION`, extended under ruling 1 so a scene can declare none |
| The sheet product, its cooker and runtime view; pages as a lease; residency in a 2D scene | `CONTENT-LOOP` (`thrum-cook`, `thrum-asset`, `thrum-asset-gpu`), extended under ruling 2 |
| The contribution list and the tentative-frame protocol | `EFFECT-GRAPH`, extended under ruling 2 |
| The sprite scene in the V1 game | `SHOWCASE`'s `examples/v1-game`, extended under ruling 3 |
| The crowd view | This module (`examples/crowd-2d-view`, game content) |
| The reference crowd, its rules and traces | `SIM-2D` |
| The tick, committed state, the tick page | `thrum-core` |

**A frame, in order.** The host reads the tick page. On a new tick the game's scene source extracts world rows;
the engine updates the band memory, rebuilds the order and uploads changed rows and the order list. The game
gives the view for this frame's alpha and its frame rows. Residency's upload pass copies. The sprite pass clears
the target to the scene's background colour and draws once. Tone encodes it, capture reads it if asked, and present copies it to the surface. On
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
after the owner's last V1 milestone is Done, with the behaviour defined here, and it then belongs to the owner.

1. **The frame runtime** (`APPLE-TILE`, `APPLE-PRESENTATION`): scene-kind selection, the display-referred
   transfer, and a scene that declares no reconstruction. Built in M1.
2. **Content** (`CONTENT-LOOP`, `EFFECT-GRAPH`): the sheet product, pages as a lease, and residency and the
   contribution list in a 2D scene. Built in M4.
3. **The V1 game and its shells** (`SHOWCASE`, `T2-CAPTURE`, `PLATFORM-RUNTIME-IOS`): a `sprites` scene
   selection, off by default, that the window, the page, the iOS shell and the capture scripts accept. Built in
   M1, and for the simulator in M5.

**Contracts.** No contract's text changes. The scene kind is a load-time selection in C04's sense, not a live
change. C05's preview clause is met as `CONTENT-LOOP` meets it. C07 says each view publishes one frame contract
for reconstruction providers; ruling 1 settles that a view may declare no reconstruction, and it then publishes
its projection, exposure and extent, writes no motion, depth, reactive mask or jitter, and no provider may be
selected for it. If the design's reviewer reads that as a change to C07, the sentence is added to C07 through the
module list before M1 is built. E08 of the architecture brief still puts pure 2D outside scope, which the
owner's page overrides; rewording it is the owner's.

## Cost account

No budget is set (brief, decision 3). Estimates, to be replaced by measurements:

| Work | Estimate |
|---|---|
| Row storage | About 64 bytes a row (Bevy's is 80): 640 KB at 10,000 |
| Upload a tick | Changed rows only. A crowd where everything moves uploads all of them, about 38 MB a second at 60 Hz, and a 40 KB order list |
| Order a tick | 71 µs at 10,000 natively; Chrome not measured |
| Draw | One call; four vertices a row |
| Fill | The sprite target at 3456×2234 is 7.7 million pixels: 62 MB as half-float, plus the output half's 31 MB intermediate. A crowd three deep shades about three times its own area |
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

1. **The host change moves a 3D image or breaks a lifecycle rule.** Early sign: the verifier, or a V1 test, in
   M1. The 3D kind is the default and must compile to the same passes and bytes.
2. **V1 ends with a frame runtime this design did not foresee.** Early sign: M1's first reading. The builder
   writes the difference in the log and ends `BLOCKED` if a mechanism here cannot stand.
3. **The full-size target is the cost in Chrome.** Early sign: M1's cost rows. The answer is under research.
4. **Overdraw of a packed crowd.** Early sign: M2's and M5's rows. The answer is under research.
5. **Jitter is larger than a band that still looks right.** Early sign: M5's reversal counts and the inspected
   crowd. The band is set from the measured jitter; if no value serves, the order key is redesigned here.
6. **Exact colour fails by a code on one target**, from the GPU's sRGB decode or a fast-math `pow`. Early sign:
   M1.5. Stop and diagnose; do not widen the comparison silently.
7. **The lease is not the shape a sheet needs.** `CONTENT-LOOP` M1 is unbuilt. Early sign: M4's first reading.
8. **A convincing fake of C2:** crisp stills of an aligned card. Motion at a fractional scale is the proof.
9. **A convincing fake of C4:** frames captured at the tick rate. The proof reads positions at alphas between
   ticks.
10. **Linear-light blending surprises an artist.** It is stated in the guide, and it is an assumption in the plan
    for the owner to overturn before `SWARM` makes art.

## Proof and builder handoff

- **Targets.** `mac-metal`, on both native recipes where an item says so. `chrome-webgpu` through
  `scripts/check_v1_browser.mjs` for the sprite scene and through this module's own script for the crowd view;
  both need a browser launch the Codex sandbox grants per run. `ios-sim` for function only, in M5, through the
  existing iOS shell.
- **Scenes.** The **sprite scene** is a selection in `examples/v1-game`, off by default: a test card of static
  rows, and the game's 10,000 simulated entities as world rows. It reaches all three shells and their capture
  scripts, as the other modules' diagnostics do. The **crowd view** is a new package beside `SIM-2D`'s rule
  package, on `thrum-app`'s window and page.
- **Art.** Test cards are generated in code. The few sprites the scenes need are drawn by the builder, original
  and simple. Nothing is lifted from another game or from Broadsides.
- **Oracles.** Every exact comparison is against a CPU reference written from the source texels and this design's
  rules, never against a retained image and never through the renderer's shaders. No baseline is registered.
  These comparisons, and M3.1's retained bound, are this module's under
  [rule 1 of exact baselines](../ARCHITECTURE.md#exact-baselines).
- **Capture.** Inspected items use T2 engine capture and name their mode. Motion is judged from `realtime`
  captures at 1920×1080 as consecutive frames at 1x; the display's full 3456×2234 is captured as `fixed` stills
  and timed in the cost records, because twenty seconds of it would not fit capture's output bound.
- **Measured bounds.** M3.1 measures how far the renderer's coverage sampling is from the box filter on each
  target and retains it; later items test against that record, not a number written here.
- **Costs never stop the build.** A row that trips a tripwire is recorded as measured, with what was found.
- **Exact baselines.** This module edits `crates/`, `examples/v1-game/` and `scripts/`, which are covered paths,
  and moves no retained image. Each milestone carries the verifier.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/render-2d-<milestone>/`; selected evidence
  in `docs/evidence/RENDER-2D/`.
- **Maturity.** Ledger rows for 2D are added in M5, with implementation, correctness, visuals and device results
  kept apart.

## Open questions

| Question | Settled by |
|---|---|
| What do the full-size target and the two output passes cost on the Mac and in Chrome? | M1 records it |
| How far is a GPU's coverage sampling from the box filter? | M3.1 measures and retains it |
| How much does `SIM-2D`'s crowd jitter, and what band hides it? | M5 measures it and sets the band |
| Does a packed crowd of 10,000 fit the refresh interval at the display's full size? | M5 records it; a miss is a finding, not a stop |
| Does `SWARM` want more than eight sheets, or display-space blending? | Its planner asks here |
