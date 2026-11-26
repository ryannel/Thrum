# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**No builder starts until the owner has approved [the design](DESIGN.md).** A design review that returns
Design Approved is not that approval; the lead starts WP1 when the owner has said yes.
[Its decisions](DESIGN.md#decisions) are the owner's.

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the worked examples](DESIGN.md#worked-examples),
[Mechanisms](DESIGN.md#mechanisms), [what a builder may change](DESIGN.md#what-a-builder-may-change) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package. Build nothing from
[the reserved shapes](DESIGN.md#reserved-shapes).

## Open for the user

- Assumption: the first-release scope you set on 25 November stands until you answer [the design's ten scope questions](DESIGN.md#scope-questions-for-the-owner).
- Assumption: SDK work packages start once you approve the design, and take a builder only when no V1 work package is waiting for one.
- Assumption: the crate is named `thrum`, a game is run with `cargo play`, and the design's fifteen words are the SDK's vocabulary, proposed in the glossary.
- Assumption: a scene is a type whose fields are what it is entered with, such as `GameOver { dodged }`.
- Assumption: a player's default keys are written on the action map, beside each action, and a player can change their own.
- Assumption: a control function may read anything a rule may. With multiplayer on, one that reads what a match cannot run ahead must say `.unpredicted()`.
- Assumption: in a 3D scene the first release's 2D colliders lie on the ground plane.
- Assumption: "2D queries" includes rectangles, a level's fixed collision, tile maps and a character move that slides. Nothing pushes anything.
- Assumption: in a match, a saved resource and a saved run are the host's.
- Assumption: a one-file game runs with its rules unchecked and is told so. A game needs a `rules` module before it can turn multiplayer on.
- Assumption: a rule that panics stops the game in Chrome, with the message shown. Elsewhere it is reported with the world intact.
- Assumption: the starter, stones and the lighthouse are rewritten on the SDK and the sky kit is then removed.
- Assumption: three fresh agents prove the documentation, one each for a 2D, a 3D and an online game.
- Assumption: models that turn, scale or carry a texture as they move wait for the V1 work packages that hold the engine's 3D drawing.
- Assumption: a tick makes one change of scene or of phase, and the one asked for first stands. A change asked for while one is landing waits a tick.
- Assumption: a person who joins a game that bots have filled takes a bot's seat and starts with a new character, not the bot's.
- Assumption: a paused match keeps its clock running underneath, so whoever the game allows can end the pause. Alone, the clock stops.
- Assumption: a game declares its prefab names in `prefabs!` blocks, and a name with no definition is found when the game starts, with no line of the game listing the blocks.
- Assumption: one key can mean one thing in play and another in a menu through named input contexts, one active at a time on each screen.
- Assumption: a content record holds no plain float. A save brought into a running match is at most 64 KiB unless the game says otherwise, and one that size takes about a second and a third to arrive.
- Assumption: a press is made in play or in a pause by what the presser's own screen showed. One made in a pause is never read, and one made in play that reaches a paused match is read when the pause ends.
- Assumption: a match of fewer than four players is kept to its number by the SDK. A newcomer who arrives while an away player's seat is kept connects, gets no seat and is told the match is full.
- Assumption: a thing attached so that it keeps its place in the world keeps it to within about 8 parts in a million, and is refused under a parent scaled outside 1/10,000 to 10,000.
- Assumption: the 3D starter's first page ends with a ball the player rolls, which passes through the rock.

## WP1 An agent writes a game's scenes, phases and rules in plain Rust against one crate, in the order they run, and proves them in an ordinary test with no window

Start with WP1.1, the item most likely to overturn the design. `thrum-core` resolves structural edits when
a tick ends today, in `crates/thrum-core/src/sim/transaction.rs` and `runtime.rs`. The barrier after each
registered system is an addition under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk). If it
cannot be built so, end `BLOCKED` with what you found.

[The headless subset](DESIGN.md#what-wp1-compiles) is all WP1 compiles. What a game writes today is
`examples/crowd-2d/src/rules.rs`; the SDK generates all of it over `thrum-core`'s `RegisteredSystem::direct`
and `GameRuntime`.

The derive crate is a new package: register it and `thrum` in `.config/architecture.toml`. New tests go in
`thrum`'s one test target, `it`. `scripts/sdk_surface.py` is yours to write, and it joins `./dev check` in this
landing.

## WP11 A game names a reusable thing, spawns it and variants of it, attaches one thing to another and takes a thing out of play

`Transform2d` and `Transform3d` arrive here as plain data, made from integers. Turns here are quarter turns,
which need none of WP2's angle maths; WP2.3 proves `keep_world` under any other angle.

Start WP11.3 with the `prefabs!` block that nothing in the game's description names: it shows whether
[the program's list of blocks](DESIGN.md#5-prefabs-children-and-parts) works ([risk 10](DESIGN.md#risks)).
Looks arrive with WP3 and WP4.

## WP2 A crowd of 10,000 written on the SDK, steered by four players, steps identically on the Mac, in Chrome and on the iPhone simulator, and costs what the hand-written crowd does

`crowd-sdk` is a new game at `examples/crowd-sdk`, registered with its conformance entry as `crowd-2d` is. It
is the reference crowd's behaviours written as example E writes them, not a port: its digests are its own. `SIM-2D` left
`scripts/sim2d_profile.py`, `scripts/sim2d_conformance.py`, the profile's lints in `.config/sim2d-profile` and
the vectors in `crates/thrum-core/src/profile`. Extend them under
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk): additions only, with `crowd-2d`'s retained traces and
the profile's refusals unchanged.

Take WP2.8's record first on a rough version, because it chooses the default size of a world, and again at the
claimed commit. The input frame's limits are in [the session guide](../../SESSION.md): three 16-bit axes for
everything an action set carries, so the width of [a button's count](DESIGN.md#6-players-actions-and-control)
is yours to choose and document. Write the heroes' control function so that WP5 can hand it to `SESSION`'s
predicted controller unchanged.

Every package under `examples/crowd-sdk` names `thrum` and no other engine crate, because
[WP8's guard](DESIGN.md#1-one-crate-one-package) will cover it. The conformance runner takes Chrome's lock and
the simulator's turns itself, so run WP2.6 and WP2.7 under no lock. For WP2.10, `scripts/test_game_rules.py` is
yours to write, on scratch games outside the checkout as `scripts/test_game_gate.py` makes one. It also runs
each scratch game's own test, for the startup refusals.

## WP12 A platformer's hero runs, jumps and lands on a level loaded from a file, proved with no window

## WP3 A small 2D game runs on the Mac and in Chrome from one command, pauses, and an agent gets a picture of it without a screen

## WP4 A small 3D game with the sky, the Sun, textured models that move and tuning sliders runs on the Mac and in Chrome

## WP6 A game shows a status display, a menu and a modal choice, worked by key, pointer and touch

## WP13 A game saves a run and resumes it from its title screen, and a recorded run replays in a test

## WP5 The crowd is hosted on the Mac and joined from Chrome and a second Mac process through the standard lobby, and no rule changed to allow it

## WP7 A game made with the SDK plays on the iPhone simulator

## WP8 A newcomer builds a 2D game, a 3D game and an online game from the guides alone

## WP9 The showcase is written on the SDK, and passes the V1 exit check from the games repository

## WP10 The SDK is the one way to make a game: the old kit is gone and the engine's guides point at it
