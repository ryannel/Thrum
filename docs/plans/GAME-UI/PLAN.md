# GAME-UI plan

A game draws health bars, timers, menus and a join screen in its own frame. [Design](DESIGN.md) ·
[done-when file](DONE.json), which owns the criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md)

Read [Mechanisms](DESIGN.md#mechanisms), [Interfaces](DESIGN.md#interfaces),
[Proof and builder handoff](DESIGN.md#proof-and-builder-handoff),
[Limits on every milestone](DESIGN.md#limits-on-every-milestone) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-ui) before starting any milestone, and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before landing.
[The planning probe](../../evidence/GAME-UI/planning-probe.md) says what was calculated before the design was
written; its code was thrown away on purpose, so build from the design.

## Open for the human

- Assumption: GAME-UI waits for all of `RENDER-2D`, and so for the V1 exit, because it draws with the sprite path and changes files `RENDER-2D`'s builder works in. The module list said its first milestone would do; starting then would put two builders in the same files.
- Assumption: "2D support in the engine" includes text and interface drawn in the frame, and this module builds the smallest part of that the game uses: text in Latin letters with accents (the languages of Western and Central Europe), bars, panels, buttons, one level of modal, and a join screen. Other scripts (Arabic, Hebrew, Chinese, Japanese, Korean, Indic), right-to-left text, translation tools, typed text, screen readers, scrolling lists and animated transitions are not built.
- Assumption: keys, pointer and touch only. No gamepad.
- Assumption: nobody types a join ticket, because it is about a hundred characters. It is pasted, or it arrives in a link that opens the game. There is no text field and no on-screen keyboard. On an iPhone it arrives by link only.
- Assumption: text is made crisp by rasterising each font ahead of time at the sizes the game names, for displays of 1×, 2× and 3×. On a display in between (Windows at 125% or 150%, a zoomed browser) the interface takes the nearest of those, so it stays crisp and is a little larger or smaller than the system's own. The other choice rasterises text while the game runs, at any size, which puts font code in the shipped game and changes `RENDER-2D`'s approved design.
- Assumption: the interface blends in linear light, as sprites do. A half-transparent black panel is lighter than in Photoshop or Figma (black at half alpha over white is 188, not 128), and light text over a dark scene is slightly heavier. If you overturn `RENDER-2D`'s assumption on this, both change together.
- Assumption: on an EDR display the interface's white is ordinary white, not the display's peak, and the interface is drawn straight over the finished picture. A highlight brighter than white, such as the Sun, still shows through a half-transparent panel at its share of that brightness, so text over a panel over it can be hard to read. The other choice draws the interface to a layer of its own and dims the scene beneath it, at the cost of a full-screen target and a composite in every frame.
- Assumption: test fonts are public domain, CC0 or Apache-2.0. Most free fonts are under the Open Font License, which requires changed versions of the font to keep that licence, and the project's rule is no share-alike data. Whether the game may ship such a font is your call, with `SWARM`'s art.
- Assumption: the interface is proved from 320×320 to 3456×2234 points, at 1×, 2× and 3×, with safe-area insets up to 64 points, and on the iPhone simulator for function only.
- Assumption: on the Mac, proofs feed key and pointer events to the game's window in code, since posting real ones needs a permission and an unlocked screen. Chrome and the simulator get real events. Real Mac input, and how the menus feel under the hand, are judged when someone plays `SWARM`.
- Assumption: test panels and menus are plain and made by the builder. The game's look is `SWARM`'s, and your judgement of it is not an item here.
- Assumption: the four changes GAME-UI needs inside other modules (a stage for the interface in the frame runtime, a second use of the sprite pass, a font product in the content loop, input and an interface scene in the shells and the V1 game) are built by GAME-UI's own builder under rulings in the module list. No milestone is added to those modules.

## M1 Panels and marks a game draws appear over its own frame, exact to the pixel, on the Mac and in Chrome, and a game with no interface pays nothing

Criteria: M1.1–M1.12 in the done-when file.

Start by reading what V1 and `RENDER-2D` left: the design was written before either was built. When it was
planned the tone stage wrote an `Rgba8Unorm` image of encoded values (`crates/thrum-render/src/synthetic/display.rs`,
`synthetic_display.wgsl`), capture read it and `present.wgsl` copied it. The interface stage is built there under
ruling 1 and the sprite pass's second use under ruling 2. That is the 8-bit mode. The float mode is the EDR
present pass `APPLE-PRESENTATION` M6 builds (`present.rs`, `present.wgsl`), which this Mac selects on its
built-in display; [mechanism 1](DESIGN.md#1-the-interface-stage) says where the stage sits in each, and forcing
SDR and forcing a headroom are that module's overrides. `thrum-ui` is a new
crate: register it, and the cooker's new dependencies in M2, in `.config/architecture.toml`. In this milestone
it only carries a paint list of rows from an embedded sheet; text and the tree come later. The interface scene
is a new `THRUM_SCENE` selection beside `RENDER-2D`'s sprite scene in `examples/v1-game/src/content.rs`.
`scripts/game_ui.py` is new and this module's.

## M2 Text is drawn from a cooked font, every glyph crisp at 1×, 2× and 3×, and an edited font shows in the running game

Criteria: M2.1–M2.14 in the done-when file.

The font product is built in `thrum-cook`, `thrum-asset` and `thrum-asset-gpu` under ruling 3, on the sheet
product, the lease and the one switch that `RENDER-2D:M4` built there; read those first. A font's sheets are
ordinary pixel-kind sheets, so nothing in the sprite pass changes for text. Residency's publication gains the
interface as a reader under [mechanism 11](DESIGN.md#11-a-list-and-its-sheets-change-together); here its
prepared layout is runs only, since the tree comes in M3.

## M3 A status display of a health bar, a timer and counters stays readable and in place at every window size, density and safe area

Criteria: M3.1–M3.14 in the done-when file.

The Mac shell reports only an extent to a game today (`crates/thrum-app/src/app.rs`, `game.rs`); density and
insets are added there and to the page under ruling 4, behind the feature the design names. M3.14 extends M2's
prepared layout from runs to boxes.

## M4 A level-up choice and a join screen are worked by key, pointer and touch, and nothing the interface consumes reaches the game

Criteria: M4.1–M4.13 in the done-when file.

The Mac shell passes a game key codes and nothing else, and the page keeps its own DOM listeners
(`examples/v1-game/web/index.html`); pointer, touch, text drops and the clipboard are added under ruling 4.
`scripts/check_v1_browser.mjs` already sends keys through the browser's debugging protocol, which also sends
mouse and touch events. The join screen shows states the scene scripts; it links neither `SESSION` nor
`TRANSPORT`.

## M5 The status display, the level-up choice and the join screen work on the iPhone simulator, and a game maker can build an interface from the guide

Criteria: M5.1–M5.8 in the done-when file.

The iOS shell is `examples/v1-game-ios`: its `gesture.rs` turns touches into one steering gesture, which now
receives only what the interface passes, and `ios/uitest` holds the UI test that makes real touches.
`RENDER-2D:M5` left the sprite scene running there and `scripts/render2d_sim.py` to drive it. The creator guide
is `docs/design/ENGINE-EXPERIENCE.md` and the ledger is `docs/design/maturity.json`.
