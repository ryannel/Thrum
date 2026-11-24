# HOST plan

Every host gets its device, its fault handling and its pause and recovery behaviour from one place in the engine,
and a game is hosted through one contract. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the
criteria · [log](LOG.md) · [module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone. Chrome and the simulator start outside
the Codex sandbox; if a launch is refused, leave the item open and say so, and do not substitute another target's
result.

## Open for the human

- Assumption: Chrome keeps the device request it has today, so cooked textures in the browser stay uncompressed and no reference image moves. Turning compressed textures on in Chrome is a separate change to what Chrome draws.
- Assumption: on the Mac a GPU validation or out-of-memory error no longer ends the game: it is recorded and the game carries on, as it already does on iPhone.
- Assumption: a still scene keeps rendering at the display's full rate (finding M58). Idling it belongs with the quality governor.

## WP1 One device recipe: the Mac window, the browser page, the iPhone app, captures and benches ask for their device the same way and answer a GPU fault the same way

The requests today are in `crates/thrum-app/src/app/device.rs`, `web/recovery.rs` and `sky/capture.rs`,
`examples/v1-game-ios/src/ios/render.rs`, `examples/v1-game/src/window/capture.rs`, `src/bin/window/capture.rs` and
`src/browser.rs`, and the game's benches. `REVIEW-ENGINE` WP1 has already given the iPhone app its features and
limits; this replaces that request with the recipe. `thrum-render`'s capabilities and the registry are `APPLE-TILE`'s:
build the recipe beside them and change no recipe selection.

## WP2 Pausing, overload, a lost surface and a lost device behave the same in the Mac window, the browser page and the iPhone app

The four copies of the policy are `crates/thrum-app/src/app/pause.rs`, `examples/v1-game-ios/src/world_policy.rs`,
`recover_overload` in `crates/thrum-app/src/web.rs` and the episode counting in `examples/v1-game/src/host/world.rs`.
`APPLE-PRESENTATION` WP4 changes the render actors for render extent and may be in the same files: integrate on
what has landed. The contract, the sky kit's second construction path and the render owner's job and reply
types are WP4's: leave them.

## WP3 The game and the iPhone app hold the engine's frame host directly, a frame can be drawn without reading it back, and there is one way to capture the game to disk

The forwarding host is `examples/v1-game/src/host.rs`. The registered exact baselines are drawn through the
game's capture path, so the path that stays must give the bytes it gave.

## WP4 A game is hosted through one contract on the Mac, in Chrome and offscreen: it implements one runtime half and names its own controls, and a second game copies nothing from the first

`Game` is in `crates/thrum-app/src/game.rs`, the browser's contract in `web.rs`, and the sky kit builds its host
twice, in `sky.rs` and `sky/browser.rs`. `WATERS`' ruling 3 and `RENDER-2D`'s second program make the shell serve a
second game too; if either has landed, build on it. `GAME-UI` adds events, density and insets to the shells later
and builds on this contract. The frame job and reply are `RenderJob`, `FrameRequest` and `RenderReply` in
`crates/thrum-app/src/app/render_actor.rs` and `Job`, `FrameRequest`, `FrameReply` and `Reply` in
`examples/v1-game-ios/src/ios/render.rs`; the iPhone app already depends on `thrum-app`. Its owner thread and
UIKit code stay for `GAME-UI` WP6.

## WP5 A failure in the shells can be matched and printed: `thrum-app`'s public functions return typed errors

`CREATOR` owns the creator docs and the kit scenes; change in them only the error a creator's closure returns.
