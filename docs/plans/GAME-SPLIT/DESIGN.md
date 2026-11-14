# Game split: the game in its own repository, the engine proved without it

14 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../evidence/code-review-2026-11-14/CODE-REVIEW.md), theme T5 (H02, H13, H19, H20, M72, M73) ·
[exact baselines](../ARCHITECTURE.md#exact-baselines)

## The request

The owner, 14 November 2026: "it sounds like we're blurring the lines between the engine and the game a ton.
Please let's separate out the game as much as we can, it shouldn't even sit in the same code base ideally."

The lead's task carries the decisions already made:

- "Design the end state where the game lives in its own repository and depends only on the engine's public API:
  engine host/shell code moves into engine crates …, qualification moves into the owning crates or an engine-side
  harness, shared fixtures move to engine test support, and repo_check enforces the boundary. Stage it: first make
  the boundary real inside this repository, then move the game out."
- "the game moves to a new local Git repository at /Users/ryannel/Workspace/thrum-games (v1-game now; the swarm
  game later), with no remote until the owner asks for one. It consumes the engine through Cargo path dependencies
  on a sibling Thrum checkout, so engine changes surface in the game at once; switching to pinned git revisions is
  a later step once the engine has releases. The last stage creates that repository, moves the game with its
  history-free copy, and leaves Thrum with no game code. The upcoming swarm game follows the same shape from the
  start."
- "V1 still exits through the showcase game, so keep its milestones buildable throughout and say how the V1 gate
  is verified once the game lives outside."

## What `examples/v1-game` is today

One package of about 26,000 lines that is three things at once:

1. **Engine host code**: a forwarding frame host, window, browser and capture glue. [HOST](../HOST/DESIGN.md) moves
   it into engine crates.
2. **The engine's proving ground**: the scenes the engine is qualified on (the grid, the field, the valley), the
   10,000-entity workload, rigs for `EFFECT-GRAPH`, `CONTENT-LOOP`, `COOKED-DRAW` and `APPLE-TILE`, and selections
   that other modules add (`CLOUDS`, `PUDDLES`, `RENDER-2D`, `GAME-UI`). The five registered exact baselines are
   drawn through it, and 67 approved checks in other modules run `cargo test -p v1-game`, 54 of them in
   milestones not yet Done.
3. **The showcase game**: what `SHOWCASE` owns, the exit scene with its content, routes, exit configuration,
   launcher and exit record.

Engine tests also compile the game's source by `#[path]`, the game reads a private fixture out of `thrum-render`,
and `thrum-cook`'s tests depend on an example. The dependency guard sees Cargo edges only, so it sees none of
this.

## The end state

```
thrum/                       the engine, and nothing that is a game
  crates/                    engine crates, the host and shells included
  crates/thrum-test-support  the GPU gate and shared fixtures (dev-only)
  examples/                  kit scenes that teach the public API, and the proving ground
thrum-games/                 a sibling Git repository, local, no remote
  v1-game/                   the showcase game: scenes, rules, controls, routes, launcher, its baselines
  swarm/                     later, from its first line
```

- **A game** depends on packages under the engine's `crates/` by sibling path, and on nothing else of the
  engine's. It holds what a second game would not copy: scenes, rules, controls, content, entry points.
- **The proving ground** is engine test content. It keeps a frozen copy of the scenes that the engine's baselines
  and approved checks draw, the workload, the rigs and the modules' selections. From the move on, an engine
  milestone proves itself in its own crate or in the proving ground, never in a game.
- **The engine is checked alone.** `./dev check` builds no game and needs no games repository.

## Mechanisms

**1. The game gate** (M1). One engine-side command, `python3 scripts/game_gate.py <game> <command>`, builds a game
from the games repository against the engine checkout the command runs in, and runs the game's own command there.
It is how a builder in a worktree, the loop's clean checkout and a reviewer each test the game against the exact
engine commit in question, when the game's manifests say `../../thrum/crates/…`.

- It finds the games repository beside the main checkout (through Git's common directory, so it works from any
  worktree), or where `THRUM_GAMES` says.
- It builds through a folder of links in which `thrum` is the checkout under test and `thrum-games` is the game's
  tree. Cargo resolves a path dependency's `..` lexically, so the game's manifests need no change. This was tried
  while planning: one game built against two engine trees, the sibling and a linked one, and each build ran the
  tree it was pointed at.
- It never writes into the games repository (a build through links would rewrite the game's lockfile, so it works
  from a detached copy), refuses a games tree with uncommitted changes unless told to take it as it stands, and
  records both revisions.
- `boundary` runs [REPO-RULES](../REPO-RULES/DESIGN.md)' shape rules on the game: its path dependencies are only
  packages under the engine's `crates/`, and nothing reaches out of the game by any other route.
- With no games repository it says so and exits with its own code.

**2. No hidden reach** (M2). `package-reach` goes to zero: the field the visibility tests draw becomes a fixture
in `thrum-test-support`, and the temporal witness and the cloud shader are reached through `thrum-render`'s public
API. Then the proof that matters: the engine's packages build their tests with the game's folders absent.

**3. The game is extractable** (M3). Inside the package, everything that qualifies an engine module sits behind a
`proving` feature that is on by default. With it off, the package builds for the Mac and the browser and the
game's own tests pass, which is what the copy that leaves will be. The feature is on by default, and no test
target is declared with `required-features`, because Cargo passes a named test it skipped: every approved
`cargo test -p v1-game --test …` in another module must keep running the test it names. Rigs that need no hosted
scene move to the crate they test. A normal web or iPhone build ships no qualification hook. A `game-names` rule
keeps a game's package name, crate name and control keys out of `crates/`.

**4. The move** (M4), after the V1 exit. `thrum-games` is created with one commit: the package as built with
`proving` off, with what `SHOWCASE` owns. In Thrum, what stays at `examples/v1-game` is the proving ground, with
the game's launcher, exit configuration, presets and live baselines gone. Both sides keep a copy of the sources
they both draw (the valley, the workload's rules): the engine's is frozen test content that its baselines pin,
and the game's is the game, free to change.

**5. The name** (M5). The proving ground keeps the folder and package name `v1-game` while any approved open
check names it, because a builder cannot edit a pinned check. When the last such milestone is Done it is renamed
to `examples/proving-ground`, package `proving-ground`,
the registry's covered paths follow with no reference changed, and the game's iPhone app, by then on the shared
shell `GAME-UI` M6 builds, follows the game.

## How a game is built by the loop once it lives outside

The loop runs in one repository, so a game's plan, done-when file, record, reviews and evidence stay in Thrum's
`docs/plans` and `docs/evidence`: that is planning, not game code. A game module's builder (`SHOWCASE` after the
move, `SWARM` from its start) works in its Thrum worktree for those files and commits the game's code on the
games repository's `main`. Only one game module builds at a time (`SWARM` waits for `SHOWCASE`), so that branch
has one writer. Each of its checks goes through the gate, so the loop's clean checkout tests the committed game
against the engine commit it is recording. An engine module's builder never edits the games repository.

## How the V1 gate is verified once the game lives outside

`SHOWCASE` builds and closes the V1 exit before the move, unchanged. M4 then shows the same exit from outside:
through the gate, each of the three rows runs the six stops and the exit flythrough and reports what the exit
configuration says, and the exit scene agrees with its retained baselines, with the engine at the commit under
test. The record names both revisions and the V1 exit record gains a dated note. `scripts/run_v1_showcase.py`
stays in Thrum as a forward to the gate, so every command line recorded in evidence still runs. After that the
exit is re-verified at any engine commit by one command: `python3 scripts/game_gate.py v1-game exit`.

## Why the move waits for the exit

Moving sooner means building `SHOWCASE` M5 to M8 across two repositories, on a gate that does not exist yet, on
the path every V1 module's closing proof runs through; it also needs `SHOWCASE` re-planned and re-reviewed. The
boundary is what the owner asked for, and it is real from M3: rules at zero, the host in the engine, the game
buildable alone. The move itself is then a copy.

## Exact baselines

No baseline moves. The registered references are drawn through the proving ground, which stays at its path until
M5. M5 changes paths in `.config/exact-baselines.json` and no reference, under
[the ruling](../ARCHITECTURE.md#seam-rulings-for-track-4). `SHOWCASE`'s threshold baselines are the game's and go
with it.

## Interfaces

| Interface | For |
|---|---|
| **Game gate**: `game_gate.py <game> test`, `boundary`, `exit`, `ios-sim`, and a game's own commands; the record with both revisions | `SHOWCASE` after the move, `SWARM`, reviewers |
| **Games repository layout**: one folder a game, path dependencies on `../../thrum/crates/…`, no remote | `SWARM` |
| **Proving ground**: where an engine milestone's hosted proof lives | Every engine module |

## What it must not do

Rewrite Git history; push or add a remote to the games repository; change another module's items, tests or
references; rename what an open check names; put engine proving code in the games repository.

## Proof

The gate is proved on a scratch games repository with two engine trees that differ. The boundary is proved by
rules at zero and by building the engine with the game absent. The move is proved by the gate's `boundary`, `test`
and `exit` on the real repository, and by the verifier.
