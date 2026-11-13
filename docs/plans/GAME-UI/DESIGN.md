# GAME-UI: health bars, timers, menus and a join screen drawn in the game's own frame

13 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) · [module list](../ARCHITECTURE.md) ·
[the brief](../BRIEF.md). C05, C06, C07, C11 and C12 in [CONTRACTS](../../design/CONTRACTS.md),
[the U1 design](../../design/modules/AUDIO-UI.md#u1-operating-model) and
[RENDER-2D](../RENDER-2D/DESIGN.md) govern.

## The request

The owner's words weren't passed on with this task; this design answers the lead's task, "Plan module GAME-UI. It
needs: RENDER-2D", which is row 20 of the module list. What the owner asked for is on
[the page they set on 10 November 2026](../../design/NEXT-GOAL-SWARM-GAME.md), which [the brief](../BRIEF.md) carries
as decision 7:

> - **A swarm survivor.** One or more players hold out against ever-growing crowds of enemies, with automatic
>   attacks and upgrades between waves. It's an original game: its own name, characters, art and rules.
> - **2D support in the engine.** The engine supports 2D games as well as 3D, and this game proves it.
> - **Multiplayer.** Several players play together over the network.

The module list reads "2D support in the engine" as including text and interface drawn in the frame, which is an
agent's reading and stays an assumption in [the plan](PLAN.md). Its brief for this module asked for "the smallest
slice of U1 that this game uses" and left two choices to this planner. They are settled here:

| Choice the module list left open | Settled as | Where |
|---|---|---|
| The text stack, after a licence and Wasm audit | Glyphs are rasterised when a font is cooked, at the sizes and scales the game names, with `skrifa`, `harfrust` and `zeno` in the cooker only. A shipped game holds no font parser, shaper or rasteriser | [Mechanism 4](#4-fonts), [research](#research-and-selection) |
| How a join address or code is entered without a text-input stack | It is never typed. A ticket reaches the join screen as a text drop: from a link that opened the game, a launch argument or a paste | [Mechanism 9](#9-text-drops-and-the-clipboard) |

This design reconciles the code on main at `8f038dec`; it is not a blind study. Read for it: `thrum-render`'s
`synthetic/display.rs`, `present.wgsl` and `contribution.rs`; `thrum-app`'s `game.rs`, `app.rs` and `lib.rs`;
`examples/v1-game-ios`'s shell, gesture and route files; `scripts/check_v1_browser.mjs`; the designs of
`RENDER-2D`, `APPLE-TILE`, `APPLE-PRESENTATION`, `CONTENT-LOOP`, `SESSION` and `TRANSPORT`; and
[a planning probe](../../evidence/GAME-UI/planning-probe.md) run for it.

**This module is planned before the code it builds on exists.** `RENDER-2D` is not built, and neither are
`APPLE-TILE` M10 to M12, `APPLE-PRESENTATION` M3 to M11 or `CONTENT-LOOP` M1 to M3. So this design names the
behaviour it needs from them and never one of their APIs, and the builder of each milestone first reads what was
actually built ([handoff](#proof-and-builder-handoff)).

## Purpose and scope

A game builds a small tree of text, bars, panels and buttons. The engine lays it out for the window, draws it
over the game's own frame, and gives each key, pointer and touch event to the interface or to the game, never
both.

**Consumers.** `SWARM` builds its status display, its level-up choice, its menus and its join screen with it.
Any 2D or 3D game can.

**What exists.** `thrum-controls` and its AppKit and DOM panels, which are demo controls outside the frame and
stay as they are. No text is drawn in a frame. The Mac shell passes key codes to a game and nothing else; the
browser page keeps its own DOM events; the iOS shell turns touches into one steering gesture. The output half
tones one scene-linear colour into an `Rgba8Unorm` image of encoded sRGB values, which capture reads and present
copies to the surface. [APPLE-PRESENTATION's EDR design](../APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence) adds a
second form, not built when this was planned: on an EDR surface the present pass itself maps the scene to linear
values in an `Rgba16Float` drawable, with no encoded image and no later copy. [Mechanism 1](#1-the-interface-stage)
has a form for each.

**Commissioned.** The eleven mechanisms below, proved by an interface scene inside the V1 game on `mac-metal`,
`chrome-webgpu` and `ios-sim`: a status display, a modal choice and a join screen over `RENDER-2D`'s sprite
scene, and text over the 3D valley.

**Not built, and recorded as unqualified.** Scripts that need shaping or reordering (Arabic, Hebrew, Indic, CJK),
right-to-left layout, a localisation system, typed text and IME, accessibility bridges and screen readers, a
gamepad. Also not built: scrolling and clipped regions, an animation or transition system, an authored file
format for trees, rich text, text selection, sound. Fonts that change size continuously. The game's own screens,
art and font, which are `SWARM`'s. An app runner for a second game, which is
[`CREATOR`'s ask for `SWARM`](../ARCHITECTURE.md#what-track-3-asks-of-existing-modules).

**Words used here.** The **interface** is one tree of **nodes** and what it draws. A **control** is a node a
player can work. A **scope** is a subtree that holds focus, such as a modal. A **point** is the interface's unit
of length; the **interface scale** is how many output pixels one point is. An **output pixel** is one pixel of
the image the surface is presented from. A **contact** is one thing a player holds down: a key, a pointer button
or a touch. A **text drop** is a string the shell hands over whole. The **paint list** is the interface's rows
for one frame, in drawing order. A **font product** is a cooked font: a table and its sheets. A **display
mode** is the form the output half's last image takes: the **8-bit mode** or the **float mode** of
[mechanism 1](#1-the-interface-stage). An **element** is a separately placed part of a screen, such as a bar, a
timer or a counter with its label; text over its panel, a fill over its track and a child inside its parent are
one element. A list's **stamp** and a **prepared layout** are defined in
[mechanism 11](#11-a-list-and-its-sheets-change-together).

## What a player must see

| Cue | Range it must hold over | Plausible wrong substitute, and what rejects it |
|---|---|---|
| **U1 In the frame.** The interface is part of the picture the engine submits and captures | A 2D scene and a 3D scene; both native recipes and Chrome; every display mode the output half offers | A DOM or AppKit layer over the canvas. An interface drawn before reconstruction, which swims under jitter. Rejected by engine capture, and by text over the valley that is the same bytes in every frame while the camera moves |
| **U2 Crisp text.** Every glyph on screen is the bitmap rasterised for its size and scale | Interface scales 1, 2 and 3; every size a font was cooked at; any position; an outline font and a pixel font | One atlas scaled to fit, sharp at one density. Rejected by equality with the cooked texels at each scale, and by a size that was not cooked being refused |
| **U3 In place and readable.** The status display keeps its place and stays inside the safe area; no element covers another and no text is cut off | Every extent from 320×320 to 3456×2234 points; insets up to 64 points a side; scales 1, 2 and 3; portrait and landscape | A layout right at one window size. Rejected by a sweep of the range, with no overflow, no element over another and no elision |
| **U4 Values change without shiver.** A timer, counters and a bar change every frame and nothing else moves | Digits that change width; a right-aligned counter that grows a digit; a bar at any fraction | Gaps between digits that change as the number does. Rejected by rigid runs and tabular digits, read back as pixels |
| **U5 Readable over a crowd.** Text and bars stay legible over thousands of moving sprites | The sprite scene's 10,000 rows, light and dark backgrounds | Bare text that vanishes over a bright sprite. Rejected by inspection of outlined text and panels over the field |
| **U6 A modal anyone can work.** Three or four choices, by key, pointer or touch, with the focused one obvious | Keys alone, pointer alone, touch alone, and a mix; a row, a column and a grid | A menu that works by mouse only, or whose focus is invisible. Rejected by each choice made by each means |
| **U7 What the interface takes, the game never gets.** An event the interface consumes never moves the player, and movement held under a modal carries on after it | Keys and touches held when a modal opens, pressed while it is open, and still held when it closes; a lost window focus | A flag the game is expected to check. Half a press reaching the game. Rejected by a random-interleaving test and by a player row that must not move |
| **U8 Join without typing.** A player gets a host's ticket onto the join screen and sees what the connection is doing | A link, a launch argument and a paste; a ticket of 100 characters; refusals and closes with their reasons | A text field that needs a keyboard nobody built. Rejected by the ticket arriving whole on each target |
| **U9 It survives the lifecycle.** | Resize, a change of scale, safe-area change on rotation, a discarded frame, a capture frame, a replaced device, background and foreground | An interface rebuilt from nothing, losing focus or an open modal. Rejected by focus on the same control after each |
| **U10 Free when unused.** A game with no interface pays nothing, and an interface that has not changed does no work | Every existing scene; an interface left alone for a minute | A stage that runs empty. Rejected by the compiled pass list, creation counters and byte counts |
| **U11 A second game builds on it.** | The interface scene, and `SWARM` later | An engine crate that names a game's type or a screen. Rejected by a question on the code |
| **U12 One font and one scale in a frame.** When a font or interface art is edited, or the scale changes, every frame is whole from the old or from the new | A font whose glyphs grow wider at the same cooked size; an art frame that changes size; scales 1, 2 and 3; a discarded switch frame; values changing throughout | Sheets switched first and text laid out a frame later, which draws new glyphs at old spacing and settles on the right picture. Rejected by comparing every submitted frame of the change with a fresh start on the old sheets and on the new |

The bar is the status display of Vampire Survivors, Brotato and Halls of Torment, the level-up choice of Vampire
Survivors and the boon choice of Hades, and text as crisp as the system draws at the same size. It is judged in
words, in the running scene.

## Research and selection

- **Where glyphs are rasterised.** Current general-purpose interfaces rasterise on demand into an atlas that
  grows: Dear ImGui since 1.92 ("Glyphs are loaded and rasterized dynamically", its release notes; read 13
  November in a mirror), and by the planner's memory egui, Bevy and Unreal's Slate, which were not read for this
  design. Games with a fixed look bake fonts ahead, as BMFont atlases and TextMesh Pro's static atlases do.
  **Cooked is chosen.** This game's text is authored Latin strings at a handful of sizes on three targets whose
  densities are whole numbers. Baking gives exact glyphs with no font code in a shipped game, fits C05's rule
  that shipping never links authoring, and fits [the sheet interface](../RENDER-2D/DESIGN.md#8-sheets), in which
  a sheet's images are fixed when it is prepared or cooked. It gives up sizes and characters not named at cook,
  and exact text at scales between whole numbers.
- **Signed distance fields were not chosen.** They scale freely, and they soften small text and round corners.
  They would also add a third kind of sampling to the sprite pass. A status display needs a few known sizes
  drawn exactly.
- **The stack**, from the crates.io registry on 13 November 2026. All are pure Rust.

  | Crate | Version | Licence | Use here |
  |---|---|---|---|
  | `skrifa` | 0.50.0 | MIT or Apache-2.0 | Reads the font: character map, outlines, metrics, light hinting. Chosen |
  | `harfrust` | 0.16.0 | MIT | Shapes pairs of glyphs, to read kerning out of the font's positioning tables. Chosen |
  | `zeno` | 0.3.3 | MIT or Apache-2.0 | Turns an outline, or its stroke, into coverage. Chosen |
  | `swash` | 0.2.10 | MIT or Apache-2.0 | Does all three; a forum post calls it no longer actively worked on. Not chosen |
  | `fontdue` | 0.9.5 | MIT, Apache-2.0 or Zlib | Parser and rasteriser, no positioning tables. Not chosen |
  | `ab_glyph` | 0.2.32 | Apache-2.0 | Rasteriser on an older parser, last released September 2025. Not chosen |
  | `rustybuzz`, `ttf-parser` | 0.20.1, 0.25.1 | MIT; MIT or Apache-2.0 | Last released in 2024; `harfrust` and `skrifa` succeed them. Not chosen |
  | `parley`, `cosmic-text` | 0.13.0, 0.19.0 | MIT or Apache-2.0 | Full paragraph layout with scripts and fallback, wider than this slice. Not chosen |
  | `taffy` | 0.15.1 | MIT | Flexbox and grid layout. Not chosen: the layout here is a page of rules |

  The three chosen crates run only in `thrum-cook`, which is native and authoring-only, so nothing here has to
  build for Wasm. The runtime reads a table.
- **Text is blended in linear light.** FreeType's own guidance is that "Text must be rendered with linear alpha
  blending and gamma correction", and that without it "dark text looks burnt-out, pixely and blotchy on bright
  background, bright text too frail on dark backgrounds"
  ([FreeType](https://freetype.org/freetype2/docs/hinting/text-rendering-general.html), read 13 November). It is
  also the engine's one colour contract, and what `RENDER-2D` does for sprites.
- **An interface over an HDR frame.** NVIDIA's guidance composites an sRGB interface over the tone-mapped
  scene, and warns of what a bright scene does to it: "white in your UI looks a bit gray when it is composited
  on a really bright scene", and under a part-transparent panel "with an HDR image and a 1000 nit highlight,
  that 20% bleed-through is now 200 nits making for difficult reading". Its remedies are a brightness scale on
  the interface, or an interface drawn to a layer of its own and composited by a shader that dims the scene
  beneath it ([Rendering a Game for HDR Display](https://developer.nvidia.com/rendering-game-hdr-display), read
  13 November). Unity's HDR output manual calls the level interface white sits at "paper white"; that is from a
  search summary and the manual was not read. **Chosen:** the interface is drawn
  after the transform with its white at SDR white, straight into the frame. The layer is not built: it costs a
  full-screen target and a composite in every frame, to help a case the sprite game's SDR-range art does not
  produce. What the float mode therefore lets through is said in [mechanism 2](#2-colour-and-blending).
- **Layout.** Clay's model (sizes that are fixed, fit their content, grow to fill, or a fraction of the parent;
  a direction, padding, a gap and an alignment; floating elements for modals;
  [README](https://github.com/nicbarker/clay), read 13 November) covers a status display and a menu in about a
  page of rules. Its README does not give the order of its passes; the order in
  [mechanism 6](#6-the-tree-and-its-layout) is this design's.
- **Direction keys.** The W3C's draft for spatial navigation picks, among candidates beyond the focused box in
  the direction pressed, first those that overlap it across the direction and then the nearest by a weighted
  distance, with ties in document order
  ([CSS Spatial Navigation Level 1](https://www.w3.org/TR/css-nav-1/), working draft of 26 November 2019, read 13
  November). [Mechanism 7](#7-focus) is a smaller rule of the same shape.
- **Pasting.** A page's paste event carries the pasted text with no permission prompt. On iOS an app that
  reads the pasteboard without a system paste control makes the system ask the player. Both are from the
  planner's memory and were not read for this design; [mechanism 9](#9-text-drops-and-the-clipboard) uses a link
  on iOS so that it does not depend on the second.
- **Touch targets.** Apple's layout guidance asks for 44×44 points, which also clears the 24 CSS pixels of WCAG
  2.2's criterion 2.5.8. Both figures are from secondary sources read on 13 November.
- **What would change the selection.**
  - If a GPU's sRGB encoding does not return an authored code exactly ([mechanism 2](#2-colour-and-blending)),
    the stage is redesigned here to draw into a layer of its own with the encoding in its shader.
  - If text over a scrim cannot be read over an EDR highlight in a scene a game ships, the stage is redesigned
    here with that same layer, composited by a shader that limits the scene beneath it.
  - If the 8-bit mode's stage pass shows in a tripwire, its draw is joined to the pass that writes the display
    image, which is designed here first.
  - If a game needs sizes, characters or scales that cannot be named when it is cooked, a glyph cache filled
    while the game runs is designed here first. It needs a sheet whose pages can change, which is a change to
    `RENDER-2D`'s interface.
  - If the direction rule strands a control in a menu `SWARM` draws, that menu names its neighbours.

## Mechanisms

### 1. The interface stage

A frame host may be prepared with an interface. It then has one more stage in the output half: after the scene
half, after reconstruction and after the display transform, and before capture reads the frame and the surface
shows it. The stage draws the frame's paint list into **the target the display transform wrote**, at that
target's extent. So the interface is in every submitted frame and in engine capture, and never under jitter,
history, exposure or a tone curve.

The output half has two display modes, and the stage has a form for each:

| | The 8-bit mode | The float mode |
|---|---|---|
| **Where** | Chrome, the simulator, a Mac with no EDR display, and a Mac with SDR forced. The only mode on main when this was planned | A Mac whose session starts on a display with EDR headroom, as this Mac's built-in display has. [APPLE-PRESENTATION](../APPLE-PRESENTATION/DESIGN.md#techniques-and-evidence) designs it and its M6 builds it |
| **The transform writes** | Encoded sRGB into an `Rgba8Unorm` display image. Present copies that image to the surface | Linear values from 0 to the headroom H, with 1.0 at SDR white, into an `Rgba16Float` target, in the present pass. The target is the drawable, or the final engine texture that [T2](../T2-CAPTURE/DESIGN.md) has the host make for a captured frame whose drawable cannot be copied |
| **The stage draws** | Into the display image, between the transform and present | Into that same float target, after the transform's draw and before the frame is tapped or presented. It is recorded in the frame's presentation phase, after the drawable is acquired |
| **Capture reads** | The display image, after the stage | The float target, after the stage, with its encoding in the capture's record as T2 asks |

In the float mode the stage's rows are drawn in the present pass's own render pass where the pass as built
allows it, which adds no load and no store; a pass of its own that loads and stores the target is the builder's
fallback, and M1.10 records which was built. Nothing else about the float mode is the builder's to choose.

In both modes interface white is SDR white, whatever the display's headroom: the code 255 in the 8-bit mode and
the value 1.0 in the float mode, which APPLE-PRESENTATION's transform defines as SDR reference white.

A host prepared without an interface is unchanged in either mode: it compiles the same passes, allocates the
same storage and draws the same bytes as before this module. A host with one changes no pixel that no row
covers: not a byte in the 8-bit mode, and not a bit of a half-float in the float mode, a value above 1 included.

The stage is the output half's, for either scene kind. It is one more use of
[the sprite pass](../RENDER-2D/DESIGN.md#interfaces), given the mode's target, a view in output pixels and the
paint list as its rows. Its pipeline for each mode the host can select is made when the host is prepared with an
interface, so a generation build that changes the mode compiles nothing during play. It keeps no state from
frame to frame, so a discarded frame, a capture frame and a resize have nothing to roll back.

A paint list is laid out for one extent and says which. A frame whose target has another extent, because the
surface changed after the shell laid out, draws the list unscaled from the top-left corner and counts it in the
frame's record; the shell lays out for the new extent before its next request.

### 2. Colour and blending

Interface art and glyphs are premultiplied in linear light, as every sheet is, and in both modes the stage
blends in linear light: source plus destination times one minus source alpha. A row's tint is clamped to the
range 0 to 1 before the blend, so a row's colour never exceeds its alpha and the interface never adds light
above SDR white.

**In the 8-bit mode** the display image holds encoded sRGB values, so the stage reaches it through a view that
decodes on read and encodes on write. The scene beneath is whatever the display transform wrote. An opaque,
untinted texel leaves the frame as its authored 8-bit code. A tinted or part-transparent one is the linear-light
blend over the scene's displayed colour, to within one code. A scrim of black at half alpha over white is code
188, not 128, as [RENDER-2D](../RENDER-2D/DESIGN.md#7-colour) says of sprites.

**In the float mode** the target is linear already, so the blend is the target's own and no view is involved.
An opaque, untinted texel leaves the frame as the linear value of its authored code: put through the sRGB curve
it rounds back to that code, with 0.46 of a code to spare
([probe 5](../../evidence/GAME-UI/planning-probe.md#5-do-authored-colours-and-a-one-code-allowance-survive-a-half-float-target)).
Opaque white is exactly 1.0 at any headroom. A tinted or part-transparent texel is the same blend over the
scene's displayed value, to within one code of the mode's encoding. The result is not clamped, so a scene value
above 1 keeps its share beneath a part-transparent row: black at half alpha over a highlight of 4.0 leaves 2.0.

**A mode's encoding** is how a comparison reads a frame. In the 8-bit mode it is the stored code. In the float
mode it is the stored value put through the sRGB curve, continued above 1 by the same formula, times 255 and
not rounded. "Within one code" is a difference of at most 1 in that measure. Half-float storage accounts for at
most 0.27 of it up to a headroom of 16 (probe 5).

Light text over a dark scene is a little heavier than a paint tool shows it and dark text over light a little
finer, which is the correct weight by FreeType's account. In the float mode a highlight brighter than SDR white
shows through a scrim at its share of that brightness, and white text beside it reads as grey. That is accepted
for this module and named in the ledger; [research](#research-and-selection) says what would change it.

Colours a game gives a node are sRGB codes.

### 3. The paint list and the pass

The interface turns its tree into sprite rows in drawing order: a parent before its children, children in tree
order, and a scope above everything that is not in it. Rows share one layer and one sort value, so the pass
draws them in list order. Nothing is clipped, so a node that must not spill is laid out not to.

Every drawn thing is a row of a frame of a sheet:

- a **glyph** is a frame of a font's sheet, at a scale of exactly 1 for an outline font and a whole number for a
  pixel font;
- an **image** is a frame of an art sheet, sampled as its sheet's kind is in the scene;
- a **fill** is a one-texel white tile, tinted and scaled to a box, from a small embedded sheet the interface
  always brings;
- a **panel** is nine tile frames, the cells of a 3×3 grid image: corners at their own size, edges stretched
  along the box and the centre both ways. Its art must be uniform along each stretched edge;
- a **bar** is a track and a fill whose length is its fraction of the track, snapped to a whole output pixel.

The interface has its own set of up to eight sheets, declared when it is prepared and leased as the scene's
are. Row capacity is declared with it; a row beyond it is dropped and counted. An unchanged tree builds no list
and uploads nothing: the stage draws the rows it already holds.

### 4. Fonts

A **font source** is a folder in the game's assets: a font file and a description. The description names the
sizes in points, the scales (1, 2 and 3 unless it says otherwise), the characters, and for each size an optional
outline width. With no character list the set is Basic Latin, the Latin-1 Supplement, Latin Extended-A and the
common punctuation outside them: dashes, curly quotes, the ellipsis, the bullet, the multiplication and minus
signs, the euro sign and four arrows. That covers the languages of Western and Central Europe written in Latin
letters. The description's syntax is the builder's.

**An outline font** is rasterised once for each size and scale, at size × scale output pixels, from its
outlines with light hinting (vertical only), each glyph on a whole-pixel origin. A glyph becomes one frame of
white at the coverage as alpha, with its pivot at the pen's origin on the baseline. An outlined size adds a
second frame for each glyph, the coverage of its outline stroked to that width, drawn beneath the first in a
colour of its own.

**A pixel font** is a grid image and a description of its cells, characters and advances. Its texels are kept
as authored, it is cooked once, and it is drawn at whole multiples of its own height. An outline for it is one
texel, grown at cook.

**The font product** is a **font table** and one pixel-kind sheet for each scale. The table holds, for each
size and scale: the line's ascent, descent and gap; each character's frame and advance; the kerning of each
pair that has any; the widest digit; and a missing-glyph box. **Advances and kerning are rounded to whole output
pixels when the font is cooked.** The table also lists which base letter and combining mark compose to which
character of the set. A malformed table is refused before any of it is used, by name.

Only the sheet of the scale in use is resident: 1.6 MiB, 5.4 MiB or 11.4 MiB for one face at four sizes
([probe 1](../../evidence/GAME-UI/planning-probe.md#1-how-large-is-a-cooked-font)). The font product is the one
the module list asks of `CONTENT-LOOP`. It hot-reloads as a sheet does, and the text laid out from it changes
in the same submitted frame: [mechanism 11](#11-a-list-and-its-sheets-change-together) says how. No frame is
blank, and none draws a new glyph at the old font's spacing or in the old font's box. A save that cannot be
cooked, or that drops a size or a scale a live text uses, is refused whole: the last good font stays drawn and
the reason is named.

### 5. Text runs

A **run** is one string in one font, size and colour. Its glyphs sit at whole-pixel offsets from its origin,
found by adding each advance and each pair's kerning; the origin snaps to a whole output pixel. So a run is
rigid: the same string has the same shape wherever it is drawn and whenever it moves
([probe 2](../../evidence/GAME-UI/planning-probe.md#2-does-rounding-the-pen-keep-spacing-even)).

- A size or scale the font was not cooked at is refused, by name, when the text is made. Nothing is resampled.
- A character the font lacks draws the missing-glyph box at a fixed advance and is counted.
- A base letter followed by a combining mark draws as the composed character when the font has it.
- **Tabular digits**, when a text asks for them, give every digit the widest digit's advance, so a number
  keeps its width as it counts.
- A text breaks at a newline, wraps at spaces inside a width, aligns left, centre or right, and when its lines
  do not fit its box drops the rest and ends in an ellipsis. It never draws outside its box.

Ligatures are not formed. A game may also place a run among its 2D scene's frame rows, in world units; it is
then sampled as any pixel sheet is, and is exact only at a whole-number scale on the pixel grid.

### 6. The tree and its layout

A game builds the tree in code and keeps it. Each node has an identity the game gives it, which survives
relayout, a change of scale and a replaced device. The game changes values (a text, a fraction, a colour,
whether a node is shown) and the tree notes whether a box could have moved.

Nodes are: a **stack**, which lays its children in a row or a column with padding, a gap and an alignment; an
**overlay**, which lays its children over one another; **text**; **image**; **fill**; **panel**; **bar**;
**button**, which is a control; and **scope**. A node's width and height are each fixed in points, fit to its
content, a fraction of its parent, or a share of what the parent has left, with an optional least and most. A
child of an overlay is anchored to an edge, a corner or the centre of its parent, or to a point the game gives
each frame, for a mark that follows something in the world.

**Order of work**, over the whole tree, when a box could have moved: widths that fit their content, from the
leaves up; widths that share or take a fraction, from the root down; text wrapped to its width; heights the same
two ways; positions. When shares cannot fit, they shrink to their least, then text elides, and what still does
not fit is reported as an overflow that names the node. Nothing else is ever drawn outside its parent. A node
that takes a fraction of a parent which fits its content has no answer, and is refused when the tree is built.
A node that is not shown takes no room and no input.

Layout is in points. A box's edges are rounded to whole output pixels only when it is painted; hit tests use the
unrounded boxes. The root's box is the output extent less the safe-area insets; a node marked full-bleed, such
as a scrim, takes the whole extent.

### 7. Focus

One control has focus, in the top open scope; with no scope open nothing has focus and every key is the game's.
Opening a scope gives focus to the control it names; closing it returns focus to where it was. A focused
control that is hidden, disabled or removed passes focus to the scope's named control, or failing that to the
next in tree order. A scope traps focus: nothing outside it can be focused, hovered or pressed while
it is open.

A key press that is bound to an **intent** works the interface: activate, cancel, a direction, next or
previous. The bindings are a table the game sets; the default is Enter and Space, Escape, the arrow keys, and
Tab with and without Shift.

- **A direction** moves focus to a control in the scope whose centre lies beyond the focused one's in that
  direction. Those whose extent across the direction overlaps the focused control's are taken first, and among
  them the one with the least gap along the direction. Failing any, the least sum of the gap and twice the
  offset across. Ties go to the smaller offset between centres, then to tree order. With no candidate, focus
  stays. A control may name its neighbour in a direction, which then wins.
- **Next and previous** walk the scope's controls in tree order and wrap.
- **Activate** works the focused control when the key goes down, and **cancel** asks the game to close the
  scope. A key that repeats while held repeats a direction, next or previous, and nothing else.

In a row, a column or a grid every control is reachable by direction keys and every move is undone by its
opposite; elsewhere next and previous still reach everything
([probe 4](../../evidence/GAME-UI/planning-probe.md#4-can-direction-keys-reach-every-control)).

A pointer that moves over a control focuses it, so keys and pointer never disagree. The focused control is
drawn in its focused style after a key or pointer event, and not after a touch. A button's styles (idle,
focused, pressed, disabled) are the game's.

### 8. Who gets an event

The shell gives every event to the interface first. The interface consumes it or passes it to the game; an
event never reaches both.

**Each contact has one owner, from press to release.** At a press:

- If an **exclusive scope** is open, the interface owns the contact, whether or not it hits a control. A scope
  is exclusive unless it says that keys bound to no intent may pass.
- Otherwise the interface owns a pointer button or a touch that lands on a control, and a key bound to an
  intent while a scope is open. The game owns everything else.

Every later event of that contact goes to its owner: moves, repeats and the release. A button activates when
the contact that pressed it is released inside it, and not if it slid off. A pointer move with no button down
is the interface's over a control or while an exclusive scope is open, and the game's elsewhere.

**When an exclusive scope opens**, the game is told once that its input was interrupted and must be let go.
The contacts it held are suspended: their later events are swallowed, and a suspended key never moves focus.
**When the last exclusive scope closes**, the game is told which contacts are still down and worked no control,
with where each touch now is, and they are the game's again. So a player who holds a direction through a
level-up keeps walking after it, and the key that made the choice does nothing more. A lost window focus or a
backgrounded app interrupts the game and cancels every press in the interface.

A scope may open **disarmed** for a time the game gives. It consumes as an exclusive scope does and acts on
nothing until the time has passed, so a key already on its way down does not make a choice.

**Hit boxes.** A control's hit box is its box grown to at least 44 points a side. Where grown boxes overlap, a
point goes to the control whose drawn box is nearest, then to the later in tree order. A disabled control takes
the press and does nothing. A control removed while pressed cancels its press.

The rule held on 20,000 random interleavings in
[probe 3](../../evidence/GAME-UI/planning-probe.md#3-does-the-event-ownership-rule-ever-hand-the-game-half-of-a-press).

What the interface hands the game is small: a control was activated, a scope was cancelled, a text drop
arrived. The game turns those into its own commands; in a match, a choice becomes
[a command `SESSION` admits](../SESSION/DESIGN.md#interfaces). Whether the game pauses under a modal is the
game's.

### 9. Text drops and the clipboard

Nothing is typed, and there is no text field and no on-screen keyboard. A **text drop** is a string the shell
hands the interface whole, with where it came from:

| Target | A link or launch | A paste |
|---|---|---|
| Mac | A launch argument | Command-V |
| Chrome | The fragment of the page's address | The page's paste event |
| iOS | An address the app is opened with | Not built: reading the pasteboard without a system control prompts the player |

The top scope's node that accepts drops takes it; with none, the game is handed it. A drop longer than that
node allows, or holding a control character, is refused and counted. A text node may elide in the middle, so a
ticket of a hundred characters shows its two ends. The interface also asks the shell to put a string on the
clipboard, for a host's ticket; that is built on the Mac, the only target that hosts.

The join screen itself is the game's: `SESSION`'s connection state and `TRANSPORT`'s ticket reach it as values
the game puts into nodes. This module links neither.

### 10. Scale, density and safe area

One point is a whole number of output pixels: the interface scale, which is one of the scales the game's fonts
were cooked at. The game chooses it. The default is the cooked scale nearest the display's density, the higher
of two equally near, so the interface is the size the platform's own is on a display of whole density, and
crisp on every display. On a display of density 1.5 the default is 2 and the interface is a third larger than
the platform's.

A change of scale is taken only when the new scale's font sheets are resident, and in one submitted frame.
Until then the interface is drawn whole at the old scale. Text measures differently at each scale, because
advances are rounded for each, so the new scale has a layout of its own, and it changes with its sheets under
[mechanism 11](#11-a-list-and-its-sheets-change-together). The game is told when the display's density changes,
and the default follows it.

The shell reports the output extent, the display's density and the safe-area insets, and reports them again
when any changes. On the Mac and in Chrome the insets are what the window and the page report, which is zero on
this machine; on iOS they are the view's, and they change with rotation.

### 11. A list and its sheets change together

What a paint list draws was measured from particular sheets: glyph advances from a font's table at one scale,
an image's size from an art sheet's frames. A list therefore carries a **stamp**: the generation of each
interface sheet it was built from, and the scale. **The stage draws a list only with exactly the sheets its
stamp names.** A font edit, an edit to interface art and a change of scale all go through this one rule, which
is what keeps a new glyph out of an old run and an old box.

- **Prepare.** When a publication changes a sheet the interface uses, or the game asks for another scale, the
  game hands the interface the new table or frame sizes while the new sheets are still becoming resident. The
  interface lays its texts and its tree out a second time from them: runs, wraps, elisions, boxes and a second
  paint list with the new stamp. That is the **prepared layout**. The layout being drawn stays the one that is
  drawn, hit-tested and focused. A value the game sets goes to both, so neither freezes.
- **Refuse.** A new table that lacks a size or a scale a live text uses makes the interface refuse its part.
  Under [CONTENT-LOOP's rule 7](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) the publication is
  then refused for every reader, with the reason named, and the prepared layout is dropped. A refused change of
  scale is reported to the game.
- **Ready.** The interface is one more reader of the publication, under that design's rule 3. Its part is ready
  when the render owner holds the prepared list, which reaches it with a frame request. From the request after
  the prepared layout exists, each request carries both lists. Residency still owns the publication; the
  interface reports to it through the game, which connects the two.
- **Switch.** The render owner holds at most two lists. In each frame the stage draws the newest list it holds
  whose stamp names only sheets that frame is lent. Which sheets a frame is lent is fixed before the frame is
  extracted, as it is for every reader. So the frame before the switch shows the old text whole, and the switch
  frame shows new glyphs at new offsets in new boxes, with every other reader of the publication.
- **Commit.** A frame's record names the stamp it drew. When the shell reads a submitted frame with the new
  stamp, the prepared layout becomes the one drawn and hit-tested, focus stays on the same nodes by identity,
  and the old layout is dropped; after a change of scale the old scale's sheets are given back then. Until the
  shell has read that record a press is tested against the old boxes, for the frame or two the record takes.
- **A discarded switch frame** commits nothing. Both layouts stay, and the next frame recorded makes the same
  switch. A publication superseded or refused while it prepares drops the prepared layout, and nothing is
  rolled back, because the layout being drawn was never touched. A resize lays both layouts out for the new
  extent; a replaced device is handed both lists again.
- **Never mixed.** A frame that holds no list for the sheets it is lent draws no interface row and counts that
  in its record. Readiness makes this unreachable in a publication; it is the answer to a fault, and it is
  never a list drawn with other sheets.

Between prepare and commit the interface does its layout twice and holds two lists of rows. That is paid only
through an authoring edit or a change of scale. An edit may create the second list's storage, as
[CONTENT-LOOP](../CONTENT-LOOP/DESIGN.md#one-publication-one-switch-m1-m2) lets an edit create buffers; it
compiles nothing.

### Check that it can work

- **One more use of the sprite pass.** `RENDER-2D` provides the pass "at another stage" with a target, a view
  and rows as inputs, and proves that exact at whole-number scales on the pixel grid. A glyph at scale 1 with a
  whole-pixel pivot and origin is that case.
- **Linear blending into an encoded image.** Blending through an sRGB view is what the GPU does for any sRGB
  target. WebGPU allows a view that differs from its texture only in being sRGB, when the texture lists it;
  that is from the planner's reading of the specification before this design, and M1 confirms it in Chrome.
  Pixels no row covers are loaded and stored untouched.
- **The float mode.** The present pass's transform has already written displayed linear values when the stage
  draws, so a row is never tone-mapped and its white is 1.0 by construction. A float target blends in linear
  light with no view. A draw touches only the pixels its rows cover. A 2D scene has no tone curve to betray an
  interface drawn too early, so the placement is proved over the valley (M1.4). Authored codes and the one-code
  allowance survive half-float storage up to a headroom of 16 (probe 5).
- **One font in a frame.** The stage chooses a list by its stamp and the sheets the frame is lent, and it
  holds a list for the new sheets before they can be lent, because the publication is not ready without it. So
  no order of events draws a list with sheets it was not measured from. In the review's case, two glyphs whose
  advances grow from 8 to 12 pixels in a 16-pixel box, the frame before the switch draws the old glyphs at 0
  and 8 and the switch frame draws the new ones as the new layout wrapped or elided them.
- **Rigid runs.** Whole-pixel advances make every offset a whole number by construction.
- **Layout.** Each pass visits a node once, and sizes that depend on a parent are resolved after it, so there is
  no cycle to converge. A fraction of a parent that fits its content is refused when the tree is built.
- **Events.** Probe 3. The owner of a contact is one stored value, so a release cannot go two ways.
- **Lifecycle.** The tree, focus, scopes and contact owners are CPU state that no device event touches. After a
  device is replaced the stage is prepared again and its sheets are leased again; until they are resident its
  rows draw nothing, and the interface routes events as before.

## The system as it runs

| State | Owner |
|---|---|
| The tree, layout, focus, scopes, contact owners, the paint list, text runs, the prepared layout, each list's stamp | This module (`thrum-ui`, a new crate) |
| A publication, and whether it is preparing, ready or shown; the interface reports its part as a reader | `CONTENT-LOOP`'s residency, extended under ruling 3 |
| Which held list a frame draws | The interface stage, from the stamp and the sheets the frame is lent |
| The interface stage in the output half | `APPLE-TILE` and `APPLE-PRESENTATION`'s frame runtime, extended under [ruling 1](#interfaces) |
| The sprite pass, sheets and frames | `RENDER-2D`; its second use is wired under ruling 2 |
| The font product, its cooker and runtime view; font sheets as leases | `CONTENT-LOOP` (`thrum-cook`, `thrum-asset`, `thrum-asset-gpu`), extended under ruling 3 |
| Events, the output's extent, density and insets, text drops, the clipboard | The shells: `thrum-app`'s window and page (`CREATOR`), `examples/v1-game-ios` (`PLATFORM-RUNTIME-IOS`), extended under ruling 4 |
| The interface scene | `SHOWCASE`'s `examples/v1-game`, extended under ruling 4 |
| Demo controls and their panels | `thrum-controls`, unchanged |
| What an activation means; whether a modal pauses; every screen | The game |

**Where it lives.** The tree, its layout, focus and routing live with the shell's event loop, so an event is
consumed or passed at once. The paint list crosses to the render owner with a frame request, as a game's other
presentation edits do.

**A frame, in order.** The shell routes the events that arrived. The game applies what the interface handed it
and sets new values. If a box could have moved the tree is laid out; if anything drawn changed the paint list is
rebuilt. The list goes with the frame request. The scene half draws. In the 8-bit mode the display transform
writes the display image, the stage draws the list over it, capture reads it if asked, and present copies it.
In the float mode the present pass's transform writes the float target, the stage draws the list over it,
capture reads it if asked, and the frame is presented.

**A font edit, in order.** A font source is saved and cooked. Residency starts a publication and the game hands
the interface the new table. The interface lays out a prepared layout and its list goes with the next frame
request, beside the list being drawn. The new sheets finish their copies and the publication is ready. The
next frame recorded is lent the new sheets, so the stage draws the prepared list. The shell reads that frame
submitted and the prepared layout becomes the one it draws and hit-tests.

**A level-up, in order.** The player holds D and walks. The game opens the choice as an exclusive scope,
disarmed for a moment. The game is told its input was interrupted and stops the player; D is suspended. Focus
is on the first card. The player presses the right arrow and Enter: focus moves, then the second card is
activated. The game turns that into its command and closes the scope. The game is told D is still down, and the
player walks on. Enter's release is swallowed.

**A game's journey.** Put a font file and a description in a folder; the cooker makes the product. Prepare the
host with an interface, its sheets and a row capacity. Build a tree. Each frame, set values and hand over the
list. Take activations and drops as they come. Nothing in the game names a renderer type beyond the sprite
interface it already uses.

## Interfaces

Provided, as [the module list](../ARCHITECTURE.md#interfaces-track-3) names them:

| Interface | To | What it is |
|---|---|---|
| **Interface tree** | `SWARM`, any game | Mechanisms 3 and 5 to 7: nodes, layout, text, focus, the paint list |
| **Event routing** | The shells, then the game | Mechanism 8: every event to the interface first; consumed or passed; interruption and what is still held; activations, cancels and drops handed to the game |
| **Font product** | `SWARM`, any game | Mechanisms 4 and 11: the source folder, sizes, scales and characters; the table and its sheets; an edit that changes text and sheets in one frame |
| **Interface stage** | Any scene kind | Mechanisms 1 and 2, in the 8-bit mode and the float mode. Provided by the frame runtime under ruling 1, with the behaviour defined here |
| **Text drops and the clipboard** | `SWARM` | Mechanism 9 |

Asked of other modules. Each is new work inside state another module owns, settled as a
[seam ruling in the module list](../ARCHITECTURE.md#seam-rulings-for-game-ui): this module's builder builds it,
after the owner's last milestone is Done, with the behaviour defined here, and it then belongs to the owner.

1. **The frame runtime** (`APPLE-TILE`, `APPLE-PRESENTATION`): the interface stage in both display modes, the
   float mode's inside the present pass that `APPLE-PRESENTATION` M6 builds. Built in M1.
2. **The sprite pass** (`RENDER-2D`): its use at the interface stage, with its own sheets and rows. Built in M1.
3. **Content** (`CONTENT-LOOP`): the font product, its sheets as leases, and its switch, with the interface as
   one more reader of a publication under mechanism 11. Built in M2.
4. **The shells and the V1 game** (`CREATOR`, `SHOWCASE`, `T2-CAPTURE`, `PLATFORM-RUNTIME-IOS`): an `interface`
   scene selection, off by default; events, extent, density and insets, text drops and the clipboard from each
   shell. Built in M1 to M5. The shells' translation of their own events sits behind a feature of `thrum-app`,
   so a game with no interface links none of this module.

**Contracts.** No contract's text changes. C11 already gives the interface the first chance at an event and
keeps a global pause the game's. C06 and C07 are met by the stage being a declared stage of the one executor,
after reconstruction, with no device or submission of its own. C05's rule that shipping never links authoring
is why glyphs are cooked. The U1 design is wider than this slice; where they differ (a glyph cache filled at
run time, an authored schema, an accessibility tree) this design builds less and says so above.

`thrum-ui` depends on `thrum-render` for rows and frames. It never depends on `thrum-cook`, `thrum-asset-gpu`,
`thrum-network`, a socket crate or a platform's interface kit, and `thrum-render` never depends on it.

## Cost account

No budget is set (brief, decision 3). Estimates, to be replaced by measurements:

| Work | Estimate |
|---|---|
| No interface | Nothing: no pass, no resource, no byte |
| An unchanged interface | One draw of the rows already held. No layout, no list, no upload |
| The stage's pass, 8-bit mode | It loads and stores the display image, 8.3 MB at 1920×1080 and 30.9 MB at 3456×2234, to draw a few thousand small quads. This is the cost most likely to matter and M1 records it |
| The stage, float mode | Draws added to the present pass, with no load or store of their own. As a pass of its own it would load and store 16.6 MB at 1920×1080 and 61.8 MB at 3456×2234. M1 records which was built and what it costs |
| A font or art edit, or a change of scale | Layout twice and two lists of rows from prepare to commit, a few frames. Nothing outside that window |
| Rows | A status display is about 200 rows and a modal with four cards about 600; capacity for the proof is 2,000, about 128 KB at `RENDER-2D`'s estimated 64 bytes |
| A value that changes every frame | The tree is laid out again only if a box could move; a counter with tabular digits does not move one. A few dozen rows are rebuilt and uploaded |
| Layout | One visit a pass over tens to a few hundred nodes; microseconds expected, measured at 100, 500 and 2,000 |
| An event | One hit test over the open scope's controls |
| A font | One sheet resident: 1.6, 5.4 or 11.4 MiB for one face at four sizes, about twice that with outlines |
| Cooking a font | Rasterising about 1,300 glyphs a scale and shaping about 110,000 pairs for kerning, once for each edit |

**Tripwires**, flagged in the record and investigated while building goes on; none is a pass condition:

- about 25% paired regression on the valley route, or on the sprite scene's field, with no interface;
- the stage costing more with an empty list than its load and store explain;
- any byte uploaded or node laid out for an unchanged interface;
- a response to an event that takes more frames to reach a submitted frame than a game's own edits do.

How a menu feels under the hand is not measured here. It needs a person, and is owed at `SWARM`'s close.

## Risks

1. **The stage moves a pixel of a scene that has no interface.** Early sign: the verifier, or M1.3. The stage is
   absent unless asked for.
2. **A GPU's sRGB view does not return an authored code exactly on one target.** Early sign: M1.1 and M1.8. Stop
   and diagnose; the answer is under research. Do not widen the comparison.
3. **The stage's load and store show in the frame.** Early sign: M1's cost rows. The answer is under research.
4. **V1 ends with an output half that is neither of mechanism 1's two modes**, or whose float mode differs
   from APPLE-PRESENTATION's design: a transform outside the present pass, a target that is not linear, a white
   that is not 1.0. Early sign: M1's first reading, and M1.5's question. The builder writes the difference in
   the log and ends `BLOCKED`; the stage for a third mode is designed here.
5. **The sprite pass as built cannot run a second time with other sheets.** Early sign: M1's first reading of
   what `RENDER-2D:M5` left. It is built under ruling 2.
6. **Light hinting is not offered by the stack as expected, or small text looks soft at scale 1.** Early sign:
   M2.11's stills. The cooker's hinting is the builder's to tune; the size list is the game's.
7. **A convincing fake of U2:** text exact at the Mac's scale of 2 and scaled elsewhere. Scales 1 and 3 are
   compared too.
8. **A convincing fake of U7:** a test that sends tidy press and release pairs. The proof interleaves at random
   and opens scopes mid-press.
9. **Injected events on the Mac hide a fault in what the system delivers.** They enter where the window's own
   events do, and the ledger names system delivery as not run.
10. **A convincing fake of mechanism 11:** sheets switched first and text laid out a frame later, which settles
    on the right picture. M2.8 and M3.14 compare every submitted frame of the change, with a font whose glyphs
    grow wider at the same cooked size.
11. **A font's licence.** Many free fonts are under the Open Font License, whose modified versions must keep that
    licence. It is an assumption in the plan that test fonts are public domain, CC0 or Apache-2.0.

## Proof and builder handoff

- **Targets.** `mac-metal`, on both native recipes where an item says so. `chrome-webgpu` through
  `scripts/check_v1_browser.mjs`, which needs a browser launch the Codex sandbox grants for each run. `ios-sim`
  for function only, in M5, through the existing iOS shell.
- **The interface scene** is a selection in `examples/v1-game`, off by default, over `RENDER-2D`'s sprite scene:
  a status display (a health bar, a timer, three counters), a level-up choice of four cards, a join screen, and
  a player row that the game moves from the input it is passed. A second form puts text and a panel over the
  valley. Its screens are game code; no engine crate names them.
- **Events in proofs.** In Chrome they are real: the script sends key, mouse and touch events through the
  browser's debugging protocol, as it does today. On the simulator they are real touches from the existing UI
  test. On the Mac they are read from a script and enter at the function the window's own events enter, because
  posting events to a window needs a permission and an unlocked screen.
- **Art and fonts.** Panels and marks are generated in code or drawn by the builder, plain and original. The
  outline font and the pixel font are public domain, CC0 or Apache-2.0, kept with their licence text, or drawn
  by the builder. Nothing is lifted from another game or from Broadsides.
- **Display modes in proofs.** M1 proves the stage in every mode the output half offers, each read in its own
  encoding ([mechanism 2](#2-colour-and-blending)); the float mode is run with its headroom forced to 1 and
  above 1. What later milestones add (glyphs, layout, routing) is decided before the stage, so their exact
  comparisons and their inspected captures run in the 8-bit mode, forced on this Mac where the host would select
  the float one. A float frame is compared as stored half-floats and never through a converted preview. How
  the interface looks on an EDR panel is not judged here, and the ledger says so.
- **Oracles.** Every exact comparison is against a CPU reference built from source or cooked texels, the font
  table and this design's rules, never against a retained image and never through the renderer's shaders. No
  baseline is registered. These comparisons are this module's under
  [rule 1 of exact baselines](../ARCHITECTURE.md#exact-baselines).
- **Capture.** Inspected items use T2 engine capture and name their mode. Motion is judged from `realtime`
  captures at 1920×1080 as consecutive frames at 1x. The interface scale is forced for a capture, so scales 1, 2
  and 3 are all seen on this Mac.
- **Costs never stop the build.** A row that trips a tripwire is recorded as measured, with what was found.
- **Exact baselines.** This module edits `crates/`, `examples/v1-game/` and `scripts/`, which are covered paths,
  and moves no retained image. Each milestone carries the verifier.
- **GPU.** Every GPU check holds the `gpu` lock. Scratch goes in `tmp/game-ui-<milestone>/`; selected evidence
  in `docs/evidence/GAME-UI/`.
- **Maturity.** Ledger rows for the interface are added in M5, with implementation, correctness, visuals and
  device results kept apart.

## Limits on every milestone

Each has its reason beside it.

- A host with no interface must compile the same passes and draw the same bytes. If it does not, stop and fix
  that first, because every other module's output rides on it.
- If your change fails an exact test, a baseline or a witness another module owns, do not edit it. Write the
  proposal in the log and end `BLOCKED`; [exact baselines](../ARCHITECTURE.md#exact-baselines) rule 1 makes it
  the owner's.
- If the code `RENDER-2D` or V1 left cannot carry a mechanism here, write the difference in the log and end
  `BLOCKED`. Do not build a second output path, a second sprite pass or a loader of your own, because each would
  be a second owner of state that has one.
- An exact comparison that fails is diagnosed, never widened. A one-code allowance is for blended texels only.
- No font code in a shipped game: `skrifa`, `harfrust` and `zeno` are dependencies of `thrum-cook` alone
  (C05).
- No time stops the build (brief, decision 3). Record a row that trips a tripwire as measured, note what you
  found in the log, and keep building.
- Record only runs of the commit being claimed, with the load on the machine.
- Chrome and the simulator start outside the Codex sandbox. If a launch is refused, leave the item open and say
  so; a Mac result does not stand in for either.
- No check waits on a person: a locked screen, a withheld clipboard or a denied permission is recorded and the
  check passes or fails on what it can still prove.
- Fonts and art are public domain, CC0, Apache-2.0 or your own, with the licence text beside them. Lift nothing
  from another game or from Broadsides.
- Run the exact-baseline verifier before `land`, and again when `land` refuses a rebased commit for lack of a
  proof.

## Open questions

| Question | Settled by |
|---|---|
| Does each target's sRGB view return authored codes exactly? | M1.1 and M1.8 |
| What does the stage's pass cost on the Mac and in Chrome? | M1 records it |
| Does the output half after V1 offer exactly the 8-bit mode and the float mode as designed, for a 2D scene too? | M1's first reading; M1.5's question |
| Does the present pass as built let the stage's rows share its render pass? | M1; M1.10 records which |
| Can text over a scrim be read over an EDR highlight? | Not judged here; owed with the EDR panel's look, at `SWARM`'s close |
| Is small text clear at a scale of 1? | M2.11 and M2.12 |
| Does `SWARM` want more sizes, an outline on every size, or more than eight interface sheets? | Its planner asks here |
| Does the owner want fonts under the Open Font License? | The plan's assumption, for the owner |
