# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**No builder starts until the owner has approved [the design](DESIGN.md).** The owner, 25 November 2026: "V1
first, Rust only, I'll approve the design." A design review that returns Design Approved is not that approval;
the lead starts WP1 when the owner has said yes.

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the worked examples](DESIGN.md#worked-examples),
[Mechanisms](DESIGN.md#mechanisms), [what a builder may change](DESIGN.md#what-a-builder-may-change) and
[the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package.

## Open for the user

- Assumption: the first release has no rigid-body physics. It has 2D touching, nearest and sweep queries, which carry the swarm game and most arcade games; rigid bodies, joints and any 3D collision wait for a physics module planned after it. You can ask for physics in the first release, and its engine module is then built first.
- Assumption: the first release animates sprites by frame and has no skeletal animation, which needs skinning in the renderer and clips in the cooker, a module of its own.
- Assumption: the first release has no audio. The engine has none, and no agent can hear a result, so every sound would wait for you. The SDK keeps the place where a sound is played.
- Assumption: SDK work packages start as soon as you approve the design and are built beside the road to V1, taking a builder only when no V1 work package is waiting for one. The other reading of "V1 first" starts no SDK builder until the exit closes, which leaves the swarm game waiting for the SDK after V1.
- Assumption: every game's rules use numbers that give the same result on every device, a game played alone included, so turning multiplayer on rewrites nothing. The price is that rules use the SDK's number types and not `f32`, and a game played online cannot use an outside maths or physics library in its rules.
- Assumption: a game that can be played online keeps its rules in a crate of their own, so they can be checked. A game played alone is one crate.
- Assumption: the crate a game depends on is named `thrum`, a game is run with `cargo play`, and the twelve words in the design are the SDK's vocabulary. They are proposed in the glossary for you to agree or change.
- Assumption: the first release leaves out multiplayer for 3D games, gamepads, saved games, entities attached to other entities, scene files and an editor, and a changed rule rebuilds and restarts the game. Sea, clouds and weather join the SDK when their own modules close.
- Assumption: the starter, stones and the lighthouse are rewritten on the SDK and the sky kit they use is then removed, so there is one way to make a game. That replaces the route milestone G6 delivered.
- Assumption: three fresh agents prove the documentation, one each for a 2D, a 3D and an online game, each in about 100 game lines, the bar G6 set.

## WP1 An agent writes a game's entities and rules in plain Rust against one crate, and proves them in an ordinary test with no window

Start with the item most likely to overturn the design: write example D's rule signatures and example A's scene
function against an empty crate and make them compile and run under `Game::test()`. If the shape cannot be built,
end `BLOCKED` with the difference, as [what a builder may change](DESIGN.md#what-a-builder-may-change) says.

What a game writes today is `examples/crowd-2d/src/rules.rs`: registration, permits, an encoding for each
component, a candidate and edits. The SDK generates all of it. `thrum-core` owns the simulation:
`RegisteredSystem::direct` gives writes in place that roll back together, and structural edits resolve when the
tick ends, which is why spawns and despawns land there. Additions to `thrum-core` are allowed under
[ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk), and nothing of its existing API or tests changes.

Looks, input and `Space2d` are not in this work package: its test games use plain components, and its scenes
have no camera. The derive crate is a new package: register it and `thrum` in `.config/architecture.toml`. New
tests go in `thrum`'s one test target, `it`. `scripts/sdk_surface.py` is yours to write, and it joins
`./dev check` in this landing.

## WP2 A crowd of 10,000 written on the SDK, steered by four players, steps identically on the Mac, in Chrome and on the iPhone simulator, and costs what the hand-written crowd does

`crowd-sdk` is a new rule crate at `examples/crowd-sdk`, registered with its conformance entry as `crowd-2d` is.
It is the reference crowd's behaviours written as example D writes them, and is not a port of `crowd-2d`'s code:
its digests are its own, retained on purpose with the conformance tool. `SIM-2D` left `scripts/sim2d_profile.py`,
`scripts/sim2d_conformance.py`, the profile's lints in `.config/sim2d-profile` and the vectors in
`crates/thrum-core/src/profile`. Extend them under [ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk):
additions only, with `crowd-2d`'s retained traces and the profile's refusals unchanged.

Take WP2.8's record first on a rough version, because it chooses the default size of a world, and again at the
claimed commit. The input frame's limits are in [the session guide](../../SESSION.md). `per_player` is written
here so that WP5 can hand it to `SESSION`'s predicted controller: keep its function free of everything but its
player's entity and input.

## WP3 A small 2D game runs on the Mac and in Chrome from one command, and an agent gets a picture of it without a screen

## WP4 A small 3D game with the sky, the Sun, cooked models and tuning sliders runs on the Mac and in Chrome

## WP5 The crowd is hosted on the Mac and joined from Chrome and a second Mac process, and no rule changed to allow it

## WP6 A game shows a status display, a menu and a modal choice, worked by key, pointer and touch

## WP7 A game made with the SDK plays on the iPhone simulator

## WP8 A newcomer builds a 2D game, a 3D game and an online game from the guides alone

## WP9 The showcase is written on the SDK, and passes the V1 exit check from the games repository

## WP10 The SDK is the one way to make a game: the old kit is gone and the engine's guides point at it
