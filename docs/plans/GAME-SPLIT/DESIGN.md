# Game split: the game in its own repository, the engine proved without it

14 November 2026, revised 15 November after [its design reviews](../../../.orchestrate/evidence/GAME-SPLIT/design-review.md).
[Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-4-the-code-review-and-the-games-separation) ·
[the code review](../../../.orchestrate/evidence/code-review-2026-11-14/CODE-REVIEW.md), theme T5 (H02, H13, H19, H20, M72, M73) ·
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
  crates/thrum-test-support  the GPU gate and the test device (dev-dependency only)
  crates/thrum-fixtures      scene data that engine tests and the proving ground both draw
  examples/                  kit scenes that teach the public API, and the proving ground
thrum-games/                 a sibling Git repository, local, no remote
  v1-game/                   the showcase game: scenes, rules, controls, routes, launcher, its baselines
  swarm/                     later, from its first line
```

- **A game** depends on packages under the engine's `crates/` by sibling path, and on nothing else of the
  engine's. It holds what a second game would not copy: scenes, rules, controls, content, entry points.
- **The proving ground** is engine test content. It keeps a frozen copy of everything the engine's baselines and
  approved checks draw and read ([the frozen qualification content](#the-frozen-qualification-content)), the
  workload, the rigs and the modules' selections. From the move on, an engine milestone proves itself in its own
  crate or in the proving ground, never in a game.
- **The engine is checked alone.** `./dev check` and the exact-baseline verifier build no game and read nothing
  from a games repository.

## Mechanisms

**1. The game gate** (WP1). One engine-side command, `python3 scripts/game_gate.py <game> <command>`, builds a game
from the games repository against the engine checkout the command runs in, and runs the game's own command there.
It is how a builder in a worktree, the loop's clean checkout and a reviewer each test the game against the exact
engine commit in question, when the game's manifests say `../../thrum/crates/…`.

- It finds the games repository beside the main checkout (through Git's common directory, so it works from any
  worktree). When `THRUM_GAMES` is set, that path is the only place it looks. Nothing else in Thrum locates the
  games repository.
- It builds through a folder of links in which `thrum` is the checkout under test and `thrum-games` is the game's
  tree. Cargo resolves a path dependency's `..` lexically, so the game's manifests need no change. This was tried
  while planning: one game built against two engine trees, the sibling and a linked one, and each build ran the
  tree it was pointed at.
- It never writes into the games repository (a build through links would rewrite the game's lockfile, so it works
  from a detached copy), refuses a games tree with uncommitted changes unless told to take it as it stands, and
  records both revisions.
- `boundary` runs [REPO-RULES](../REPO-RULES/DESIGN.md)' shape rules on the game: its path dependencies are only
  packages under the engine's `crates/`, nothing reaches out of the game by any other route, and
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md)'s `test-crate-edges` holds, so a game may use the engine's test
  packages in its tests and ship neither. The gate has no rule of its own for this. The rule and the registration
  it reads are `TEST-SUPPORT` WP1's, so WP1 waits for that milestone as well as for `REPO-RULES` WP1.
- With no games repository it says so and exits with its own code.

**2. No hidden reach** (WP2). `package-reach` goes to zero.

- The mixed-mesh field becomes the first fixture in `crates/thrum-fixtures`, which this milestone creates under
  [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md#mechanisms)'s interface: plain data with no engine or game type, and no
  dependency. That design owns the package and the rule on who may depend on it; it is the one authority, and the
  field's data is defined there and nowhere else.
- The field has two readers, and neither is a copy. `thrum-render`'s visibility tests take it as a dev-dependency.
  The proving ground hosts the field in its library, on the Mac and in the browser, so it takes the fixtures
  package as a normal dependency, the one such edge the rule allows. Each converts the data to its own types.
  The GPU gate's package stays a dev-dependency everywhere.
- The temporal witness and the cloud shader are reached through `thrum-render`'s public API.

Then the proof that matters: the engine's packages build their tests with the game's folders absent, and the
proving ground builds for the Mac and for wasm32 with the field read from the fixture.

**3. The game is extractable** (WP3). Inside the package, everything that qualifies an engine module sits behind a
`proving` feature that is on by default. With it off, the package builds for the Mac and the browser and the
game's own tests pass, which is what the copy that leaves will be. The feature is on by default, and no test
target is declared with `required-features`, because Cargo passes a named test it skipped: every approved
`cargo test -p v1-game --test …` in another module must keep running the test it names. Rigs that need no hosted
scene move to the crate they test. A normal web or iPhone build ships no qualification hook. A `game-names` rule
keeps a game's package name, crate name and control keys out of `crates/`.

**4. The move** (WP4), after the V1 exit. `thrum-games` is created with one commit: the package as built with
`proving` off, with what `SHOWCASE` owns. In Thrum, what stays at `examples/v1-game` is the proving ground, with
the game's launcher, exit configuration and exit baselines gone. Both sides keep a copy of what they both draw
or read, a preset or a route included: the engine's is the frozen qualification content, which its baselines
pin, and the game's is the game, free to change. That holds for a fixture too: if the game still draws the field's prop meshes in its valley, the
engine's are the fixture and the game's are its own source from the first commit, because a game cannot depend
on the fixtures package.

**5. The name** (WP5). The proving ground keeps the folder and package name `v1-game` while any approved open
check names it, because a builder cannot edit a pinned check. When the last such milestone is Done it is renamed
to `examples/proving-ground`, package `proving-ground`,
the registry's covered paths follow with no reference changed, and the game's iPhone app, by then on the shared
shell `GAME-UI` WP6 builds, follows the game.

## The frozen qualification content

The engine's qualification must not depend on a game that is free to change. So everything the exact-baseline
verifier, its drivers and the other modules' approved checks read stays in Thrum, at the path they read it, and
is the proving ground's from the move on. Today that is:

- the proving ground package and its scenes: the grid, the field (from the fixture) and the valley, with the
  valley's sources under `examples/v1-game/assets/showcase`, which the verifier cooks;
- the presets the native matrices name (`showcase-sunset-clear-sun` and `showcase-night-upslope` under
  `examples/v1-game/params/`) and every preset and route the browser bundle carries, the moving route among them;
- the pages and the build script under `examples/v1-game/web/`, which the Chrome captures load;
- the drivers under `scripts/`: the verifier, its native and Chrome captures and comparisons, and whatever they
  import. The verifier imports the cooked-store helper from `scripts/run_v1_showcase.py`; that helper stays an
  engine script when the launcher's game half becomes a forward to the gate.

The list is what was read at planning. The test is the behaviour, not the list: with the games repository
unavailable, the verifier passes and every approved check that names the proving ground finds what it names.
Unavailable has to mean to the whole run. The verifier starts its builds and captures with every `THRUM_`
variable removed, so from WP4 it keeps `THRUM_GAMES`, and that one only: a command it starts then looks for the
games repository where the run was told to, and finds none. The
game takes its own live copies of the presets, routes and sources it uses; from then on a change to the game's
copy touches no engine reference, and a change to the engine's copy is a baseline change under
[the baseline rules](../ARCHITECTURE.md#exact-baselines). No reference image is refreshed by the move.

## How a game is built by the loop once it lives outside

The loop runs in one repository, so a game's plan, done-when file, record, reviews and evidence stay in Thrum's
`docs/plans` and `.orchestrate/evidence`: that is planning, not game code. A game module's builder (`SHOWCASE` after the
move, `SWARM` from its start) works in its Thrum worktree for those files and commits the game's code on the
games repository's `main`. Only one game module builds at a time (`SWARM` waits for `SHOWCASE`), so that branch
has one writer. Each of its checks goes through the gate, so the loop's clean checkout tests the committed game
against the engine commit it is recording. An engine module's builder never edits the games repository.

## How the V1 gate is verified once the game lives outside

`SHOWCASE` builds and closes the V1 exit before the move, unchanged. WP4 then shows the exit's function from
outside. The gate passes a game's own arguments through, so each command line that `SHOWCASE` WP5.5 and M7.2 to
M7.8 pinned on `scripts/run_v1_showcase.py` runs as `python3 scripts/game_gate.py v1-game` with the same
arguments, on the game in the games repository and the engine at the commit under test, with the thresholds and
baselines as `SHOWCASE` left them. `exit` with no further argument runs them all:

- each of the three rows runs the six stops and the exit flythrough and reports what the exit configuration says;
- the rows agree with each other and with the exit baselines, and the three controls fail as they must;
- the Mac panel and the Chrome page declare the same controls, and the six presets apply on every row;
- the live clauses, on the Mac and in Chrome: the time controls scrub the day, a saved edit to a prop mesh and to
  the ground texture appears without a restart, and the profiler's rows are current. The edit is made to the
  game's own asset, in the tree the gate built from, so a watcher still pointed at Thrum's frozen copy fails.

What the move does not re-judge is the frame evidence: `SHOWCASE` WP7.9 and M7.10's whole-frame records, M8's
visual reviews and its attended cadence. Those were accepted at the commit `SHOWCASE` claimed and stay valid
history in `.orchestrate/evidence/SHOWCASE/`. The game's `frames` command still runs through the gate, as a record and
never a verdict. So after the move the gate claims function and image agreement at an engine commit, and nothing
new about looks or cadence.

The record names both revisions and the V1 exit record gains a dated note. `scripts/run_v1_showcase.py` stays in
Thrum as a forward to the gate, so every command line recorded in evidence still runs. After that the exit's
function is re-verified at any engine commit by one command: `python3 scripts/game_gate.py v1-game exit`.

## Why the move waits for the exit

Moving sooner means building `SHOWCASE` WP5 to M8 across two repositories, on a gate that does not exist yet, on
the path every V1 module's closing proof runs through; it also needs `SHOWCASE` re-planned and re-reviewed. The
boundary is what the owner asked for, and it is real from WP3: rules at zero, the host in the engine, the game
buildable alone. The move itself is then a copy.

## Exact baselines

No baseline moves. The registered references are drawn through the proving ground, from
[the frozen qualification content](#the-frozen-qualification-content), and the proving ground stays at its path
until WP5. WP5 changes paths in `.config/exact-baselines.json` and no reference, under
[the ruling](../ARCHITECTURE.md#seam-rulings-for-track-4). `SHOWCASE`'s threshold baselines are the game's and go
with it.

## Interfaces

| Interface | For |
|---|---|
| **Game gate**: `game_gate.py <game> test`, `boundary`, `exit`, `ios-sim`, and a game's own commands; the record with both revisions | `SHOWCASE` after the move, `SWARM`, reviewers |
| **Games repository layout**: one folder a game, path dependencies on `../../thrum/crates/…`, no remote | `SWARM` |
| **Proving ground**: where an engine milestone's hosted proof lives, and the frozen qualification content | Every engine module, `BASELINE-GUARD`'s verifier |
| The mixed-mesh field, in `thrum-fixtures` (the package and its rule are [TEST-SUPPORT](../TEST-SUPPORT/DESIGN.md#interfaces)'s) | `CULLING`'s visibility tests, the proving ground |

## What it must not do

Rewrite Git history; push or add a remote to the games repository; change another module's items, tests or
references; rename what an open check names; put engine proving code in the games repository.

## Proof

The gate is proved on a scratch games repository with two engine trees that differ. The boundary is proved by
rules at zero, by building the engine with the game absent, and by the proving ground's Mac and wasm32 builds
with the field read from the fixture. The move is proved by the gate's `boundary`, `test` and `exit` on the real
repository, and by the verifier run with the games repository unavailable.
