# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**No builder starts until the owner has approved [the design](DESIGN.md).** The owner, 25 November 2026: "V1
first, Rust only, I'll approve the design." A design review that returns Design Approved is not that approval;
the lead starts WP1 when the owner has said yes.

Decided by the owner, 25 November 2026 ("Planner's default"): the first release has 2D queries and sprite
animation, and no rigid bodies, skeletal animation or audio
([the design](DESIGN.md#physics-animation-and-audio)).

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the worked examples](DESIGN.md#worked-examples),
[Mechanisms](DESIGN.md#mechanisms), [what a builder may change](DESIGN.md#what-a-builder-may-change) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package.

## Open for the user

- Assumption: SDK work packages start as soon as you approve the design and are built beside the road to V1, taking a builder only when no V1 work package is waiting for one. The other reading of "V1 first" starts no SDK builder until the exit closes, which leaves the swarm game waiting for the SDK after V1.
- Assumption: every game's rules use numbers that give the same result on every device, a game played alone included, so turning multiplayer on rewrites nothing. The price is that rules use the SDK's number types and not `f32`, and a game played online cannot use an outside maths or physics library in its rules.
- Assumption: a game that can be played online keeps its rules in a crate of their own, so they can be checked. A game played alone is one crate.
- Assumption: the crate a game depends on is named `thrum`, a game is run with `cargo play`, and the twelve words in the design are the SDK's vocabulary. They are proposed in the glossary for you to agree or change.
- Assumption: the first release leaves out multiplayer for 3D games, gamepads, saved games, entities attached to other entities, scene files and an editor, and a changed rule rebuilds and restarts the game. Sea, clouds and weather join the SDK when their own modules close.
- Assumption: the starter, stones and the lighthouse are rewritten on the SDK and the sky kit they use is then removed, so there is one way to make a game. That replaces the route milestone G6 delivered.
- Assumption: three fresh agents prove the documentation, one each for a 2D, a 3D and an online game, each in about 100 game lines, the bar G6 set.
- Assumption: a game writes one line, `scene.component::<T>()`, for a component it only ever spawns and never names in a rule's or a frame function's parameters. Every other component is found with nothing written. Forgetting the line is reported at the first such spawn, not at startup. A game with multiplayer on also writes it for a component that only a frame function names, and forgetting that one is reported at startup.
- Assumption: in a match the local player's own character answers at once only when its whole movement is a per-player rule, which reads that character, that player's input and the tick. A character whose movement must read more of the world, such as walls, is shown as the host confirms it.
- Assumption: a rule that panics cannot be recovered in Chrome, where the game stops with the rule, the tick and the message shown. On the Mac, the iPhone simulator and in a test it is reported with the world intact. A returned error is handled the same everywhere.
- Assumption: the check of a rule crate guards against ordinary mistakes (a file, a socket, a clock, a raw float, an outside crate) and is not a sandbox. A single-crate game's rules are not checked.
- Assumption: a nearest query with no radius looks at every collider on the layer it asks, so a game gives a radius on a crowded layer.
- Assumption: in the first release a 3D model that a rule can move has a colour and a material and no texture. A textured mesh is scenery, which a scene places once and nothing moves, as the showcase's valley is. Textures on models that move need engine work, planned after the first release; you can ask for it sooner.
- Assumption: when the display cannot take a picture the engine has prepared, the game's code for that picture is not run a second time. An effect started in it is shown in the next picture; something drawn for that one picture only is lost.
- Assumption: models that turn or change size between ticks need a change to the engine's 3D drawing. It waits for the V1 work packages that hold that code, so the SDK's 3D part can land after its 2D part.

## WP1 An agent writes a game's entities and rules in plain Rust against one crate, and proves them in an ordinary test with no window

Start with the item most likely to overturn the design: write the test games of
[the headless subset](DESIGN.md#what-wp1-compiles) against an empty crate and make them compile and run under
`Game::test()`. That subset is all WP1 compiles: no look, camera, input, command, player, `Space2d`, number
literal or `run()`. If the shape cannot be built, end `BLOCKED` with the difference, as
[what a builder may change](DESIGN.md#what-a-builder-may-change) says.

What a game writes today is `examples/crowd-2d/src/rules.rs`: registration, permits, an encoding for each
component, a candidate and edits. The SDK generates all of it over `thrum-core`'s `RegisteredSystem::direct`
and `GameRuntime`, as [the world](DESIGN.md#2-the-world-entities-components-resources) says. Additions to
`thrum-core` are allowed under [ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk), and nothing of its
existing API or tests changes.

Startup checks are proved here through `test()` only; `run()` and a panic in Chrome are WP3's.

The derive crate is a new package: register it and `thrum` in `.config/architecture.toml`. New tests go in
`thrum`'s one test target, `it`. `scripts/sdk_surface.py` is yours to write, and it joins `./dev check` in this
landing.

## WP2 A crowd of 10,000 written on the SDK, steered by four players, steps identically on the Mac, in Chrome and on the iPhone simulator, and costs what the hand-written crowd does

`crowd-sdk` is a new rule crate at `examples/crowd-sdk`, registered with its conformance entry as `crowd-2d` is.
It is the reference crowd's behaviours written as example D writes them, and is not a port of `crowd-2d`'s code:
its digests are its own, retained on purpose with the conformance tool. `SIM-2D` left `scripts/sim2d_profile.py`,
`scripts/sim2d_conformance.py`, the profile's lints in `.config/sim2d-profile` and the vectors in
`crates/thrum-core/src/profile`. Extend them under [ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk):
additions only, with `crowd-2d`'s retained traces and the profile's refusals unchanged.

Take WP2.8's record first on a rough version, because it chooses the default size of a world, and again at the
claimed commit. The input frame's limits are in [the session guide](../../SESSION.md). `per_player` is written
here so that WP5 can hand it to `SESSION`'s predicted controller unchanged: its function is the character's
whole movement step, as `PlayerMovement::step` is in the reference crowd, and takes nothing but its character's
components, its player's input and the tick.

Every package under `examples/crowd-sdk`, a runner of its own included, names `thrum` and no other engine
crate, because [WP8's guard](DESIGN.md#1-one-crate-two-halves) will cover it.

The conformance runner takes Chrome's lock and the simulator's turns itself, so run WP2.6 and WP2.7 under no
lock. Register `crowd-sdk`'s witnesses in its conformance entry as `crowd-2d`'s are.

For WP2.10, `scripts/test_game_rules.py` is yours to write, on a scratch game outside the checkout as
`scripts/test_game_gate.py` makes one. `cargo play --check`, which wraps the profile check, arrives with WP3.

## WP3 A small 2D game runs on the Mac and in Chrome from one command, and an agent gets a picture of it without a screen

## WP4 A small 3D game with the sky, the Sun, cooked models and tuning sliders runs on the Mac and in Chrome

## WP5 The crowd is hosted on the Mac and joined from Chrome and a second Mac process, and no rule changed to allow it

## WP6 A game shows a status display, a menu and a modal choice, worked by key, pointer and touch

## WP7 A game made with the SDK plays on the iPhone simulator

## WP8 A newcomer builds a 2D game, a 3D game and an online game from the guides alone

## WP9 The showcase is written on the SDK, and passes the V1 exit check from the games repository

## WP10 The SDK is the one way to make a game: the old kit is gone and the engine's guides point at it
