# Draw an interface in a game frame

Add `thrum-ui` to a game that needs an in-frame status display or menu. Use `thrum-asset-gpu` for
cooked sheets. The game owns its values, tree, scopes and actions. The frame host draws the
interface after scene reconstruction, at the drawable's pixel extent.

From the Thrum repository root, cook the sample assets, exercise the tree and router, then launch the
V1 interface on a Mac. The example prints the row counts and checks that the button activation and
whole ticket arrive. The window shows the health bar, timer and counters over the sprite scene.

```sh
cargo run --locked --offline -p thrum-cook -- cook tests/fixtures/game-ui/font-source tmp/my-interface/store
cargo run --locked --offline -p v1-game --example interface_guide -- tmp/my-interface/store
THRUM_SCENE=interface THRUM_INTERFACE_TEXT=status THRUM_FONT_STORE="$PWD/tmp/my-interface/store" \
  cargo run --locked --offline -p v1-game
```

For the live menu, close the status window and run the next command. Press **M** for the four
choices. Use the arrow keys and Enter, click a card, or press Escape to close it. The launch ticket
opens the join screen. With the sample ticket, **Connect** displays a refusal from the game's demo rules.

```sh
THRUM_SCENE=interface THRUM_INTERFACE_TEXT=menu THRUM_FONT_STORE="$PWD/tmp/my-interface/store" \
  cargo run --locked --offline -p v1-game -- --join-ticket 'coast/AB+42'
```

## Cook a font

Put a font file, its licence and `font.json` in one folder under the source root. Name the folder
for the product you will look up. This is the sample's `type/font.json`. The `file` is relative to
that folder and the outline width is in points.

```json
{
  "kind": "outline",
  "file": "Roboto-Regular.ttf",
  "sizes": [
    {"points": 12},
    {"points": 16, "outline": 1},
    {"points": 24}
  ]
}
```

With no `scales` field, the outline font cooks at scales 1, 2 and 3. Specify `"scales": [2, 3]`
when those are the scales your game will offer. With no `characters` field, an outline font uses
Basic Latin, Latin-1 Supplement, Latin Extended-A and common punctuation. Supply a string of
characters when you need a smaller set. A pixel font supplies a grid PNG, cell size, baseline and
character-to-cell table with advances. Use whole multiples of its authored glyph size. Both kinds
cook to a font table and pixel-kind sheets. The font cooker runs during authoring. See the working
[pixel font source](../tests/fixtures/game-ui/font-source/pixel/font.json).

## Prepare the host and lay out a tree

Use the complete [compilable example](../examples/v1-game/examples/interface_guide.rs) for the
configuration, status tree, scope, touch routing and whole text drop. Integrate those pieces into your
game's `SceneSource` and input owner.

1. Before preparing the frame host, create a `CookedConfig` and register the font manifest name with
   `lease_font`. Request a cooked scale with `font_scale_request().set(scale)`. Create `CookedAssets`
   from that configuration and keep its `ProductLease`.
2. Return `Some(InterfaceConfig)` from `SceneSource::interface_config`. List the embedded sheets
   first, followed by leased names in `InterfaceSheets`. Supply the product lease as the sheet
   source. The example uses slot 0 for an embedded white pixel and slot 1 for the cooked font
   sheet. Set `row_capacity` for your largest paint list. After preparation, pass the
   `CookedAssets` to `FrameHost::set_scene_contribution`. When a scene already has a contribution,
   register the font on its existing cooked asset configuration so both use the same publication.
3. Build one retained `Tree` from declarations with stable node IDs. Use a fill, panel or image for
   the background, a bar for a fraction and text for values. Overlay anchors place the status
   group inside the safe area. Stacks arrange children in a row or column. Update values with
   `set_fraction` and `set_text`. Give text an offered point size, and use tabular digits for timers.
4. Lay out with `Tree::frame(Viewport { extent, scale, insets }, PaintInputs { ... })`. The extent
   is in drawable pixels and the insets are in interface points, ordered left, top, right, bottom.
   A full-bleed scrim can extend into the safe area around the content. Pass the font table and its
   sheet slot, the white tile, and the publication's `PaintStamp`. A `PaintList` holds rows in draw
   order, with positions in output pixels. You can also fill one directly with `PaintListBuilder`.
5. Return the committed and candidate lists from `SceneSource::paint_lists`. Lay out new font
   tables from `lease.font_candidate()` and call `answer_font(stamp, result)` with the result.
   Keep painting and hit-testing the previously displayed layout until you have a submitted
   frame with the new stamp. Discard a refused candidate and keep the old lists and sheets.
   Use `lease.committed_font()` for the current resident publication.

A point is an integer number of output pixels. Choose the cooked scale nearest the display density,
choosing the higher scale on a tie. For UIKit's insets in screen points, multiply by density and
divide by interface scale. Convert input positions from drawable pixels to interface points with
the displayed scale. Take new geometry when the shell reports a resize, density or safe-area change.
Changing scale also changes glyph advances, so prepare a second layout with the new font stamp.

## Open a scope and route events

Show a `NodeKind::Scope` subtree and call `Router::open` with its ID, its focus policy and the
time when it can accept input. Set the initial focused control in the policy. After a frame
submits, pass its corresponding `tree.input_layout()` to `Router::submitted`. Use the same
submission rule when closing a scope or changing which controls are visible.

Advance deadlines with `Router::advance`. Pass translated key, pointer and touch events to
`Router::route` before gameplay sees them. Bind keyboard input to `Intent` values such as
`Activate`, `Cancel` and direction. The router maintains one owner for a contact from press to
release. A button activates on release inside its hit box. Allow at least 44 points for a control.

- Dispatch `Routed::Game(event)` to gameplay. An empty result means the interface took the event.
- On `Interrupted`, release gameplay's held input. Route `Event::Interrupted` on background or
  lost focus to cancel presses while retaining the open scopes.
- Apply `Resumed(held)` after closing the last exclusive scope. Reacquire only the returned contacts
  at their reported positions. Handle any result of `Router::close` and later `advance` calls.
- Turn `Activated(id)` into a game action. Handle `Cancelled(id)` according to your game's rules.

Declare a `DropTarget(max_chars)` under the scope that accepts a ticket. Wrap a whole launch,
link or paste string in `Event::Text(value, source)`. The router returns `Routed::Text` with the
target ID, whole value and source, or passes it to the game when no scope accepts drops.
Overlong strings and control characters are refused. Use middle elision on the displayed text to
keep both ends visible. The Mac shell can also handle copy requests through `request_copy` and
`take_copy`. iOS receives invite URLs through UIKit. The V1 proving app accepts
`thrum-v1://join/<percent-encoded-ticket>`.

## Colour and supported scope

Interface tints and blending use linear light. In SDR, the stage blends in linear light and stores
an encoded SDR result. A half-opacity black scrim over SDR white leaves half the light, which encodes
to about 0.74 sRGB. Choose scrim opacity against the actual backdrop. At glyph edges the coverage
also blends in linear light. Light text on a dark surface can look heavier than the same coverage
blended in encoded values. Inspect the actual weights and contrast at the intended scale. Float
presentation places SDR white at 1.0 and can leave highlights above it.

Complex scripts (Arabic, Hebrew, Indic and CJK), right-to-left text and layout, typed text and IME,
localisation, accessibility bridges and screen readers, and gamepad input are outside this slice.
Scrolling, clipped regions, animation and transition systems, an authored tree format, rich text,
selection, sound and continuously sized fonts are also outside this slice. The game supplies its own
screens and assets. iOS pasteboard input and physical iPhone qualification are pending.

Start a 2D scene with the [sprite scene guide](SPRITE-SCENES.md). See the
[interface design](plans/GAME-UI/DESIGN.md) for the stage, text and publication contracts.
