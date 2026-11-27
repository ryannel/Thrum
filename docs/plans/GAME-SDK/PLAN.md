# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**No builder starts until the owner has approved [the design](DESIGN.md).** A design review that returns
Design Approved is not that approval.

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the core](DESIGN.md#the-core),
[the reference](DESIGN.md#the-reference), [the worked examples](DESIGN.md#worked-examples),
[what a builder may change](DESIGN.md#what-a-builder-may-change)
and [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package.

## Open for the user

- Assumption: your first-release scope stands until you answer [the design's scope questions](DESIGN.md#scope-questions-for-the-owner).
- Assumption: SDK work packages start once you approve the design, and never take a builder V1 is waiting for.
- Assumption: the crate is `thrum`, a game runs with `cargo play`, and the design's fifteen words are proposed in the glossary.
- Assumption: the core changes in [the design's two tables](DESIGN.md#decisions) stand.
- Assumption: a scene is a type whose fields are what it is entered with, such as `GameOver { dodged }`.
- Assumption: default keys are written on the action map, and a player can change their own.
- Assumption: a control function may read anything a rule may, and online says `.unpredicted()` where that cannot be run ahead.
- Assumption: in a 3D scene the first release's 2D colliders lie on the ground plane.
- Assumption: "2D queries" includes rectangles, a level's fixed collision, tile maps and a sliding character move. Nothing pushes anything.
- Assumption: a saved resource and a saved run are the match's, on the host's machine. A profile is each player's own, on theirs, built with a yes to scope question 11. A guest back within the grace keeps what the host confirmed meanwhile; after it, what their machine last heard.
- Assumption: every game that runs keeps its rules in a checked `rules` module, a `mod rules { .. }` block in a one-file game. A game without one runs only in a test.
- Assumption: `cargo run` plays only rules the check last passed, and refuses after an edit to them until `cargo play` runs it again.
- Assumption: a save keeps a timer's whole wait and a phase's clock, so a resumed run shows the same progress and ends its phase on time.
- Assumption: a rule that panics stops the game in Chrome, with its message. Elsewhere it is reported with the world intact.
- Assumption: the starter, stones and the lighthouse are rewritten on the SDK, and the sky kit is then removed.
- Assumption: three fresh agents prove the documentation: a 2D, a 3D and an online game.
- Assumption: models that turn, scale or carry a texture wait for the V1 work packages that hold 3D drawing.
- Assumption: a tick makes one change of scene or of phase, and the first asked for stands. A scene has one set of phases, for everyone; what happens to one character is kept on it.
- Assumption: a person who joins a game full of bots takes a bot's seat, with a new character.
- Assumption: a pause is the same alone and in a match: the clock stands while a menu's command and a rule marked for it still work. A replay does not wait through it, and a save steps no tick.
- Assumption: a game declares prefab names in `prefabs!` blocks and the content IDs its code names in `ids!` blocks. A name with nothing behind it is found at startup.
- Assumption: one key can mean one thing in play and another in a menu, through input contexts.
- Assumption: a content record holds no plain float. A save brought into a match is at most 64 KiB unless the game says otherwise.
- Assumption: a press made while the presser's screen shows a pause is never read as play.
- Assumption: a match of fewer than four players is kept to its number, and a newcomer is told it is full.
- Assumption: a thing attached keeping its place in the world keeps it within [the design's bound](DESIGN.md#2-the-world-entities-components-resources-and-prefabs), and is refused past it.
- Assumption: the 3D starter's first page ends with a ball the player rolls, which passes through the rock.
- Assumption: a 3D match is proved twice: with no window in WP5, and on screen in WP14.
- Assumption: sheets, clips and tiles are cooked from an image and a small file beside it, in WP3.
- Assumption: a player has one character at a time, and taking another player's fails and changes nothing. Anything else they own is an ordinary entity.
- Assumption: no engine or backend type reaches a game through `thrum`.
- Assumption: in [the extension map](DESIGN.md#the-extension-map), a predicted shot is one the control function fires, and it flies on without its launcher; a data pack adds no prefab names; a guest's profile travels as commands; a `#[seen]` press carries its picture, place and aim; a tile's smaller shape is said on the scene's tile line.
- Assumption: a rule may turn a player to face a way, the pointer follows the input context, an offer with no expiry stays open, and what a tile map places is made by a spawn function.

## WP1 An agent writes a game's scenes, phases and rules in plain Rust against one crate, in the order they run, and proves them in an ordinary test with no window

Start with WP1.1, the item most likely to overturn the design. `thrum-core` resolves structural edits when
a tick ends, in `sim/transaction.rs` and `sim/runtime.rs`, and the barrier after each registered system is
an addition under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk). If it cannot be built so, end
`BLOCKED`.

[The headless subset](DESIGN.md#what-wp1-compiles) is all WP1 compiles. What a game writes today is
`examples/crowd-2d/src/rules.rs`; the SDK generates all of it over `thrum-core`'s `RegisteredSystem::direct`
and `GameRuntime`.

The derive crate is a new package: register it and `thrum` in `.config/architecture.toml`. New tests go in
`thrum`'s one test target, `it`. `scripts/sdk_surface.py` is yours to write, and it joins `./dev check` in this
landing.

## WP11 A game names a reusable thing, spawns it and variants of it, attaches one thing to another and takes a thing out of play

Start WP11.3 with the `prefabs!` block that nothing in the game's description names: it shows whether
[the program's list of blocks](DESIGN.md#2-the-world-entities-components-resources-and-prefabs) works
([risk 10](DESIGN.md#risks)).

## WP2 A crowd of 10,000 written on the SDK, steered by four players, steps identically on the Mac, in Chrome and on the iPhone simulator, and costs what the hand-written crowd does

`crowd-sdk` is a new game at `examples/crowd-sdk`, registered with its conformance entry as `crowd-2d` is:
the reference crowd's behaviours as example E writes them, with digests of its own.
The profile's tools are `scripts/sim2d_profile.py`, `scripts/sim2d_conformance.py`, the lints in
`.config/sim2d-profile` and the vectors in `crates/thrum-core/src/profile`. Extend them under
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk): additions only, with `crowd-2d`'s retained traces and
the profile's refusals unchanged.

Take WP2.8's record first on a rough version, because it chooses a world's default size, and again at the
claimed commit. The input frame's limits are in [the session guide](../../SESSION.md). Write the heroes'
control function so that WP5 can hand it to `SESSION`'s predicted controller unchanged.

Every package under `examples/crowd-sdk` names `thrum` and no other engine crate, because
[WP8's guard](DESIGN.md#1-a-game-one-crate-one-package-one-value) will cover it. For WP2.10 you write
`scripts/test_game_rules.py`, on scratch games outside the checkout as `scripts/test_game_gate.py` makes one.

## WP12 A platformer's hero runs, jumps and lands on a level loaded from a file, proved with no window

## WP3 A small 2D game runs on the Mac and in Chrome from one command, pauses, and an agent gets a picture of it without a screen

## WP4 A small 3D game with the sky, the Sun, textured models that move and tuning sliders runs on the Mac and in Chrome

## WP6 A game shows a status display, a menu and a modal choice, worked by key, pointer and touch

## WP13 A game saves a run and resumes it from its title screen, and a recorded run replays in a test

## WP5 The crowd is hosted on the Mac and joined from Chrome and a second Mac process through the standard lobby, and no rule changed to allow it

## WP14 A small 3D game is hosted on the Mac and joined from Chrome, and each player's own character answers at once

## WP7 A game made with the SDK plays on the iPhone simulator

## WP8 A newcomer builds a 2D game, a 3D game and an online game from the guides alone

## WP9 The showcase is written on the SDK, and passes the V1 exit check from the games repository

## WP10 The SDK is the one way to make a game: the old kit is gone and the engine's guides point at it
