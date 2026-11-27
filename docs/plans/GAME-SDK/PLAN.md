# GAME-SDK plan

A developer or an agent builds a 2D, a 3D or an online game in plain Rust against one documented crate.
[Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk)

**The owner approved [the design](DESIGN.md#decisions) on 27 November 2026, so builders start with WP1.**

Read [the game at a glance](DESIGN.md#the-game-at-a-glance), [the core](DESIGN.md#the-core),
[the reference](DESIGN.md#the-reference), [the worked examples](DESIGN.md#worked-examples),
[what a builder may change](DESIGN.md#what-a-builder-may-change)
and [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk) before any work package.

## Open for the user

- Assumption: SDK work packages never take a builder V1 is waiting for.
- Assumption: your yes to 3D collision queries brings camera pitch and a boom that avoids the level with it, as [that entry](DESIGN.md#camera-pitch-and-rigs) said.
- Assumption: the two engine modules your yeses need are `SIM-3D` and `AUDIO`, each planned by its own planner.
- Assumption: a pad works on the Mac and in Chrome. The phone stays touch alone in the first release.
- Assumption: you judge how a pad feels and how a game sounds once, in WP19, with a pad in hand.
- Assumption: [round 4's smaller additions](DESIGN.md#what-round-4-found), beyond the four you were told of, are wanted.
- Assumption: a save's kept entities come back under the kept parents they hung from, and a relic on a player's back comes back on the ground.
- Assumption: one terrain is drawn and stood on from one source, and a level may place things that are drawn and not solid.

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

WP11.5 is round 4's ownership: a spawn's own `.child`, `Query::under`, and the query additions that go with
disabling. Keep which prefab an entity came from as world state here, because WP13's kept entities read it.

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

WP2.11 and WP2.12 are round 4's: `CharacterSpawned`, `PerPlayer::departed` and what a `Hit` says of its mover.

## WP12 A platformer's hero runs, jumps and lands on a level loaded from a file, proved with no window

## WP3 A small 2D game runs on the Mac and in Chrome from one command, pauses, and an agent gets a picture of it without a screen

## WP4 A small 3D game with the sky, the Sun, textured models that move and tuning sliders runs on the Mac and in Chrome

## WP6 A game shows a status display, a menu and a modal choice, worked by key, pointer and touch

## WP13 A game saves a run with the things in it and resumes it from its title screen, keeps each player's own progress, and a recorded run replays in a test

## WP5 The crowd is hosted on the Mac and joined from Chrome and a second Mac process through the standard lobby, and no rule changed to allow it

## WP14 A small 3D game is hosted on the Mac and joined from Chrome, and each player's own character answers at once

## WP7 A game made with the SDK plays on the iPhone simulator

## WP15 A game is played with a gamepad on the Mac and in Chrome: its sticks, buttons and triggers do what the keys do, a menu is worked by pad, prompts show the pad's own buttons, and the pad rumbles

## WP16 A 3D character walks over hills, climbs steps, is stopped by walls and sees along a ray, proved with no window, and answers at once in a match

## WP17 A third-person 3D game on the Mac and in Chrome: the ground drawn is the ground stood on, the camera looks up and down and keeps out of the hillside, and the starter's rock stops the ball

## WP18 A game has sound on the Mac and in Chrome: a hit is heard where it happens, a thing hums while it exists, music changes with a cross-fade, and the player sets the volumes

## WP8 A newcomer builds a 2D game, a 3D game and an online game from the guides alone

## WP19 The owner plays a 2D game and a 3D game with a pad in hand and the sound on, and says what is wrong

## WP9 The showcase is written on the SDK, and passes the V1 exit check from the games repository

## WP10 The SDK is the one way to make a game: the old kit is gone and the engine's guides point at it
