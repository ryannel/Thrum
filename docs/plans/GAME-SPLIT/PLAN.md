# GAME-SPLIT plan

The showcase game moves to its own repository and uses only the engine's public API, and the engine proves itself
without it. [Design](DESIGN.md) · [done-when file](DONE.json), which owns the criteria · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation)

Read [Mechanisms](DESIGN.md#mechanisms), [the rulings](../ARCHITECTURE.md#seam-rulings-for-track-4) and
[Exact baselines](../ARCHITECTURE.md#exact-baselines) before any milestone.

## Open for the human

- Assumption: the game moves out after the V1 exit closes (`SHOWCASE` WP8), not before. Until then it is separated inside this repository: no engine code reaches it, the engine's host code is out of it, and it builds alone. The other choice moves it as soon as that is true and builds the rest of the showcase across two repositories, on the road every V1 module closes through.
- Assumption: Thrum keeps a frozen copy of the V1 scenes, and of the presets, routes and valley sources its reference images are drawn from, as engine test content, because those images and 54 approved checks in other modules' open milestones use them. It is not the game and does not change with it; the game takes its own copies and is free to change them. It keeps the name `v1-game` until the last check that names it is Done, then is renamed.
- Assumption: the game's plans, reviews and evidence stay in Thrum's `docs/`, because the build loop runs in one repository. Only code and content move.
- Assumption: `thrum-games` has no remote and no backup beyond this Mac until you ask for one.

## WP1 From any engine checkout, one command builds and tests a game in a games repository against that checkout

Nothing here needs the real games repository: the test makes its own in scratch. A probe while planning built one
game against two engine trees through a folder of links; it left a lockfile in the game's folder, which is why
the gate works from a detached copy. `./dev check` must not call the gate. `TEST-SUPPORT` WP1 has landed the
`test-crate-edges` rule and the `test_role` registration it reads: `boundary` runs that rule, and does not copy or
change it. `thrum-fixtures` does not exist in the real tree until WP2, so the scratch engine tree registers its
own fixtures package.

## WP2 No engine crate reaches into a game or an example, and the engine's tests build with the games taken away

The reaches are `crates/thrum-render/tests/visibility_buckets.rs` and `tests/support/mixed_field_host.rs` (three
game source files: `field.rs`, `field_meshes.rs`, `environment.rs`), `crates/thrum-asset-gpu/src/draw/tests.rs`
(the cloud shader), and six sites that read `thrum-render`'s `synthetic/temporal/witness.json`, the web build
script among them. The field's sources name `thrum_render` types today; as a fixture they are plain data, and the
game's `scene.rs` and `host/scene_source.rs` and the test's support module each convert. `crates/thrum-fixtures`
is `TEST-SUPPORT`'s package, created here: `TEST-SUPPORT` WP1 has landed the `test-crate-edges` rule and the
registration it reads. The visibility tests are `CULLING`'s and the witness is `APPLE-PRESENTATION`'s: move where
the data comes from and leave what the tests assert. `REVIEW-ENGINE` WP3 has removed `thrum-cook`'s dependency on
an example.

## WP3 The game builds and passes its own tests with every engine rig compiled out, and a normal build ships no test hooks

`HOST` WP4 has left the package holding scenes, rules, controls and rigs. The rigs are `effect`, `lent`, `cooked`,
the two probe binaries, the `qualify_*` browser exports and the iPhone app's qualification commands. Other
modules' builders add selections and tests to this package while you work: the feature must be on by default so
their checks run as written.

## WP4 The showcase game lives in `thrum-games`, builds against the engine through its public API alone, and passes the V1 exit check from there

This is the one milestone that works outside this repository: create `/Users/ryannel/Workspace/thrum-games`, and
nothing else outside Thrum. Do not add a remote or push it. The exit record is `SHOWCASE`'s; add the dated note and
change nothing else in it. `AGENTS.md` says to work only in this repository: it gains the games repository as the
one named exception for a game module's builder. Copy to the game; do not move out of Thrum anything on
[the frozen list](DESIGN.md#the-frozen-qualification-content). `scripts/exact_baselines.py` imports `store_root`
from `scripts/run_v1_showcase.py`, so that helper stays engine-side when the launcher becomes a forward. The
verifier is `BASELINE-GUARD`'s. By ruling 11 it keeps `THRUM_GAMES` in the environment of the commands it
starts; change nothing else in it.

## WP5 The proving ground carries its own name, the game's iPhone app has followed the game, and nothing in Thrum is called a game

The new name is in the design: `examples/proving-ground`, package `proving-ground`. The registry in
`.config/exact-baselines.json` is `BASELINE-GUARD`'s: change paths only, by the ruling.
