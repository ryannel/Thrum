# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**No builder starts until the owner has approved [the design](DESIGN.md).** The owner, 25 November 2026: "V1
first, Rust only, I'll approve the design." A design review that returns Design Approved is not that approval;
the lead starts WP1 when the owner has said yes. The design was rewritten on 26 November 2026 from the owner's
review and three dogfood games, and [its decisions](DESIGN.md#decisions) are the owner's.

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the worked examples](DESIGN.md#worked-examples),
[Mechanisms](DESIGN.md#mechanisms), [what a builder may change](DESIGN.md#what-a-builder-may-change) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package. Build nothing from
[the reserved shapes](DESIGN.md#reserved-shapes).

## Open for the user

- Assumption: the first-release scope you set on 25 November stands, so 3D collision and a 3D character move, gamepads, audio, predicted projectiles, hits rewound for the shooter, 3D multiplayer, skeletal animation, text fields and levels from an editor's file are designed and not built. [The design's ten scope questions](DESIGN.md#scope-questions-for-the-owner) ask you about each, and recommend adding 3D queries with a character move, gamepads, a small audio and a host with no window.
- Assumption: SDK work packages start as soon as you approve the design and are built beside the road to V1, taking a builder only when no V1 work package is waiting for one. The other reading of "V1 first" starts no SDK builder until the exit closes.
- Assumption: the crate a game depends on is named `thrum`, a game is run with `cargo play`, and the fifteen words in the design are the SDK's vocabulary. They are proposed in the glossary for you to agree or change.
- Assumption: a scene is a type whose fields are what it is entered with, such as `GameOver { dodged }`, and not a bare function, so a rule can name a scene and the compiler checks what it is given.
- Assumption: a player's default keys are written on the action map, beside each action, and a player can change their own.
- Assumption: a control function may read anything a rule may. In a game with multiplayer on, one that reads what a match cannot run ahead must say `.unpredicted()`, and its character is then shown as the host confirms it.
- Assumption: in a 3D scene the first release's 2D colliders lie on the ground plane, so pick-ups and triggers work in a 3D game before 3D collision exists.
- Assumption: within "2D queries", the first release adds rectangles, a level's fixed collision, tile maps and a character move that slides and reports what it met. Nothing pushes anything, so these are not rigid bodies.
- Assumption: in a match, a saved resource and a saved run are the host's.
- Assumption: a one-file game runs with its rules unchecked and is told so. A game needs a `rules` module before it can turn multiplayer on.
- Assumption: a rule that panics cannot be recovered in Chrome, where the game stops with the rule, the tick and the message shown. On the Mac, the iPhone simulator and in a test it is reported with the world intact.
- Assumption: the starter, stones and the lighthouse are rewritten on the SDK and the sky kit they use is then removed, so there is one way to make a game.
- Assumption: three fresh agents prove the documentation, one each for a 2D, a 3D and an online game, and each is then asked for a change the guides did not show.
- Assumption: models that turn, change size or carry a texture as they move need a change to the engine's 3D drawing. It waits for the V1 work packages that hold that code, so the SDK's 3D part can land after its 2D part.

## WP1 An agent writes a game's scenes, phases and rules in plain Rust against one crate, in the order they run, and proves them in an ordinary test with no window

Start with WP1.1, the item most likely to overturn the design: a rule's spawns and despawns land when the rule
ends, inside the tick's one transaction. `thrum-core` resolves structural edits when a tick ends today, in
`crates/thrum-core/src/sim/transaction.rs` and `runtime.rs`. The barrier after each registered system is an
addition under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk), for a schedule that asks for it, and
nothing of core's existing API or tests changes. If it cannot be built so, end `BLOCKED` with what you found,
as [what a builder may change](DESIGN.md#what-a-builder-may-change) says.

[The headless subset](DESIGN.md#what-wp1-compiles) is all WP1 compiles: no look, camera, input, player, prefab,
`Space2d`, number literal or `run()`. What a game writes today is `examples/crowd-2d/src/rules.rs`:
registration, permits, an encoding for each component, a candidate and edits. The SDK generates all of it over
`thrum-core`'s `RegisteredSystem::direct` and `GameRuntime`.

The derive crate is a new package: register it and `thrum` in `.config/architecture.toml`. New tests go in
`thrum`'s one test target, `it`. `scripts/sdk_surface.py` is yours to write, and it joins `./dev check` in this
landing.

## WP11 A game names a reusable thing, spawns it and variants of it, attaches one thing to another and takes a thing out of play

`Transform2d` and `Transform3d` arrive here as plain data, made from integers, because a child needs a local
place. Looks arrive with WP3 and WP4.

## WP2 A crowd of 10,000 written on the SDK, steered by four players, steps identically on the Mac, in Chrome and on the iPhone simulator, and costs what the hand-written crowd does

`crowd-sdk` is a new game at `examples/crowd-sdk`, registered with its conformance entry as `crowd-2d` is. It
is the reference crowd's behaviours written as example E writes them, and is not a port of `crowd-2d`'s code:
its digests are its own, retained on purpose with the conformance tool. `SIM-2D` left
`scripts/sim2d_profile.py`, `scripts/sim2d_conformance.py`, the profile's lints in `.config/sim2d-profile` and
the vectors in `crates/thrum-core/src/profile`. Extend them under
[ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk): additions only, with `crowd-2d`'s retained traces and
the profile's refusals unchanged.

Take WP2.8's record first on a rough version, because it chooses the default size of a world, and again at the
claimed commit. The input frame's limits are in [the session guide](../../SESSION.md). The heroes' control
function is written here so that WP5 can hand it to `SESSION`'s predicted controller unchanged. Presses travel
as a count on the frame's axes, not as timed actions.

Every package under `examples/crowd-sdk` names `thrum` and no other engine crate, because
[WP8's guard](DESIGN.md#1-one-crate-one-package) will cover it. The conformance runner takes Chrome's lock and
the simulator's turns itself, so run WP2.6 and WP2.7 under no lock. For WP2.10, `scripts/test_game_rules.py` is
yours to write, on scratch games outside the checkout as `scripts/test_game_gate.py` makes one.
`cargo play --check`, which wraps the check, arrives with WP3.

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
