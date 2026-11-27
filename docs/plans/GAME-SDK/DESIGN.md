# Game SDK: one crate a game is written against

27 November 2026. Revised that day so its core shape can be locked: the core is stated once, with its
invariants and its public names, and [an extension map](#the-extension-map) says how each feature it does not
build would be added. It replaces the design of 26 November. [Plan](PLAN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [glossary](../GLOSSARY.md) ·
[the creator guide](../../design/ENGINE-EXPERIENCE.md), whose ambition this module builds ·
[the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) ·
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

**The owner approves this design before any builder starts.** A design review that returns Design Approved is
not that approval. To approve the shape, read [the game at a glance](#the-game-at-a-glance) (one page),
[the core](#the-core) and [the extension map](#the-extension-map). [The worked examples](#worked-examples) are
the evidence that the core reads as a game, and [the scope questions](#scope-questions-for-the-owner) are the
owner's to answer.

## The request

The owner, 25 November 2026: "What I would like you to plan is a high end, intuitive, expressive, readable, SDK
that game developers can use to build new games. this should be what V1 uses ideally and what we will use to
build the swarm game. It's VITAL that we get this right as it's the entire interface that developers and coding
agents will use to build a variety of games. It's the front of the product we're building. So it needs to be as
intuitive, well documented and easy to use as possible." And: "V1 first, Rust only, I'll approve the design."

The owner, 26 November 2026: "We're looking for something that's readable, expressive, intuitive and powerful.
Don't optimise at all for reducing the amount of re-work needed. Let's just find the best possible design."

The owner, 27 November 2026, after four more agents each wrote a whole game against the design: "Let's just be
clear on the dog fooding. It seems like it's complaining about missing features, that's fine. But what we
should be very confident in is our approach and shape of our core SDK design. If we're missing features, that's
fine, as long as we know how we would add them in a delightful way. So let's flesh the plan out enough so
that's clear. And let's bring multiplayer back into scope. Out goal is to lock in the shape of the SDK, not
design every feature."

The outcome: a developer or agent who has never seen Thrum builds a small 3D game, a small 2D game and a
networked game from the SDK's documentation alone, and the code reads as plainly as the game it describes. One
structure and one vocabulary across 2D, 3D, single-player and multiplayer; good-looking defaults; mistakes that
explain themselves; every public item documented with an example. And a reader can judge the SDK's shape with
confidence, because the design says how every feature it does not build would be added without bending it.

## Decisions

Each is the owner's. A builder builds what it says and nothing it rules out.

**25 November 2026.**

- **O1 V1 first.** The V1 exit does not wait for the SDK. V1 lands on today's API, SDK work packages then move
  the showcase onto the SDK, and [the swarm game](../../design/NEXT-GOAL-SWARM-GAME.md) is built on it from its
  first line. No work package on the road to the exit waits on this module.
- **O2 Rust only.** Games are written in Rust. There is no scripting layer.
- **O3 The owner approves the design** before any build.
- **O4 First-release scope** ("Planner's default"): 2D queries and sprite animation are in; rigid bodies,
  skeletal animation and audio are out. It stands unless the owner changes it
  ([scope questions](#scope-questions-for-the-owner)).

**26 November 2026.** The owner answered "Yes I agree" to all four decisions and all ten recommendations of
[the review](../../../.orchestrate/evidence/GAME-SDK/owner-review-2026-11-26.md).

- **D1 Input-only replay, on every device.** A Thrum game's shared rules give the same result from the same
  admitted input on every device, a game played alone included. Each player's presentation uses ordinary
  floats. A state-replicated mode, for private information or an outside simulation, is not built.
- **D2 Authored objects are Rust.** A prefab with editable defaults, typed assets and optional children, and
  one model vocabulary whether a thing is placed or moves. No visual editor.
- **D3 The first character is a player's.** The first steerable thing in a first game belongs to a player,
  stays responsive in a match against fixed scenery, and enters a standard solo, host and join lobby with its
  movement function unchanged. Dynamic interactions use confirmed state.
- **D4 Typed content IDs from day one.** Gameplay content is typed, validated and named by stable IDs from the
  first release. Versioned saves that a later build loads, and data packs a game ships without recompiling,
  are year-one commitments, advertised when they hold.
- **R1** The first playable thing is a character you control. **R2** Written order tells the truth: a rule's
  finished edits, structural ones included, are what the next rule sees. **R3** A game defines a reusable
  thing, changes one detail and attaches another thing. **R4** Portable numbers only where the game's result
  depends on them. **R5** Phases, timers and resources have an owner and a lifetime. **R6** Static and moving
  objects share one visual vocabulary. **R7** Player input, local bindings and prediction eligibility are
  separate. **R8** A co-op choice and a lobby are ordinary things to author. **R9** It is easy to prove the
  game the author meant. **R10** Authored data and saved progress have stable identities.

**27 November 2026.** The owner's words are in [the request](#the-request).

- **O5 The shape is what is locked, not the feature list.** The core is settled and stated in one place. A
  feature may be missing from the first release; how it would be added may not be. Every feature a dogfood
  game missed, or the first release leaves out, has an entry in [the extension map](#the-extension-map) that
  adds names and changes none.
- **O6 Multiplayer is in the first release for 3D games as well as 2D.** Scope question 6 is answered yes.
  Whether predicted projectiles and contact, and hits judged as the shooter saw them, are built in the first
  release stay [questions 4 and 5](#scope-questions-for-the-owner).

**What seven games found.** On 26 November three agents each wrote a whole game against the design as the
review amended it, and the design was rewritten from their reports. On 27 November four more wrote a 2D
platformer, a 3D exploration game, an online 2D swarm survivor and an online 3D arena game against that
rewrite, with scope questions 1 to 3 taken as yes (reports and source in the main checkout under
`tmp/sdk-dogfood-2/`). The swarm survivor's author would choose the SDK (7 out of 10). The other three would
not yet: the platformer (7), the exploration game (5) and the arena game (2), which the first release could
not then put online at all. All seven praised the same core: rules in written order, the typed wall between
rules and the screen, phases, and tests with no window. Round 2 added a controlled character that is
predicted with no code, bots as players and content with validators. Most of what round 2 asked for is a
missing feature, and is in the extension map. Where a report showed the core itself fighting a genre, the
core is changed:

| What fought the game | The core now says | Where |
|---|---|---|
| A talk box that pauses the world could never close, a pause menu could not save, and a pause meant one thing alone and another in a match | A pause is one thing everywhere: the game's clock stands while ticks still commit, so commands, answers, requests and saves are admitted, and a rule marked `.even_when_paused()` runs | [4](#4-scenes-phases-time-and-pause) |
| A save resumed into the scene's first phase, ran the saved phase's enter functions again and restarted its clock | A scene's enter function resumes a phase with its time, and no phase is entered | [4](#4-scenes-phases-time-and-pause) |
| A ray saw through a hill, and a bolt needed every pier twice, because the level and the colliders were asked separately | Rays, sweeps and moves asked of a space meet the level too. The level alone is what a match runs ahead. One hit type, with normals and a sweep that starts in contact defined | [7](#7-space-and-the-level) |
| A shape written once for the collider and again for the move, with its dimensions guessed | One `move_shape`, in 2D and 3D, given the character's own collider | [7](#7-space-and-the-level) |
| A shot kept its pilot's seat, so whoever sat there next earned its point | A `Player` names one stay in a seat, as an `Entity` names one life of a slot | [5](#5-players-actions-and-control) |
| A spawned thing could not be attached, stored or parented by the rule that spawned it | `spawn` returns its `Entity` at once, and a spawn may say `.under(parent)` | [2](#2-the-world-entities-components-resources-and-prefabs) |
| A cooldown ring, a fade timed to a respawn and a clock on the status display each needed a rule that copied time into a resource | A frame function takes `Tick` and the level, read-only | [9](#9-frame-functions-looks-and-the-interface) |
| Nine parameters on every weapon rule, and ten arguments on the helper they shared | `#[derive(Params)]`: a struct of parameters is a parameter | [3](#3-rules-and-the-tick) |
| Cards chosen mid-fight stole the walk stick, and three level-ups in a second had to be queued by hand | Outside a modal the interface takes no direction input, and a player's open offers queue | [6](#6-commands-offers-and-requests), [9](#9-frame-functions-looks-and-the-interface) |
| Enemies meant to appear off screen appeared on a wide one, because rules cannot read a camera | `Camera2d::exactly(w, h)` shows that rectangle and no more, so a rule may assume it | [9](#9-frame-functions-looks-and-the-interface) |
| A 3D game could not be played online | Nothing in a match knows 2D from 3D, and the first release proves it (O6) | [11](#11-multiplayer-an-opt-in-over-input-only-networking) |

## Purpose and scope

Thrum's engine modules are built and proved, and each has its own public API: a frame host, a world lane, a
registered simulation, sprite rows, cooked rows, an interface tree, a session, a link. A game written against
them today is mostly plumbing. This module is the front door: one crate, `thrum`, that a game depends on and
nothing else. A game maker writes what the game *is* (its things, its rules, what the player sees) and the SDK
turns that into the engine's registrations, schedules, extraction and hosting. It adds no second engine
underneath: every kind of state keeps the owner it has ([owners](#state-and-owners)).

**Who it is for.** A developer who knows Rust and has never seen Thrum, and a coding agent in the same position.
Agents cannot watch a window, so testing a game without one, replaying a recorded run and getting a picture
from a command are part of the product.

**First release.** What is commissioned here:

1. The game structure: game, scenes with typed arguments, phases, entities, components, resources, prefabs,
   rules and frame functions.
2. Players, typed action maps with input contexts, a controlled character, bots, commands, offers and the
   standard lobby.
3. 2D space: colliders, queries on the world as it stands, a level's fixed collision loaded from content, tile
   maps, and a character move that reports its contacts.
4. 2D and 3D looks under one vocabulary: sprites and clips cooked from source images, shapes and tile layers;
   cooked models with materials and textures whether placed or moving, sky, Sun, ground and lights.
5. Typed content with stable IDs, a typed asset catalog, and saves into and out of a running game.
6. Settings on the tuning panel; the profiler; the interface.
7. Multiplayer as an opt-in that changes no rule, for 2D and 3D games (O6).
8. Tests with no window, recorded runs, pictures from a command, and errors with stable codes.
9. Reference documentation with an example on every public item, guides, templates and a new-game command.
10. The showcase moved onto the SDK, after the V1 exit.

**Not in the first release**, by the owner's scope (O4) and the engine as built. Each has an entry in
[the extension map](#the-extension-map), and the ones marked there are
[questions for the owner](#scope-questions-for-the-owner).

- Rigid-body physics, skeletal animation and audio (O4).
- 3D collision queries and a 3D character move; navigation; camera pitch; 3D level building.
- Gamepads; text fields, scrolling lists and sliders, which `GAME-UI` does not build.
- Predicted projectiles and contact, hits judged as the shooter saw them, per-player saves, host migration
  and a host with no window.
- A visual editor (D2), levels from an editor's file, and hot reload of code. A changed rule rebuilds and
  restarts.
- Water, clouds and weather. `WATERS` and `CLOUDS` are still building.
- A crates.io release. A game reaches `thrum` by a path to an engine checkout, as
  [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) settled.

### Scope questions for the owner

**Not decided here.** The owner's first-release scope (O4, O6) stands until the owner changes it. Each line is
one question, with the planner's recommendation. The design works either way: a "yes" adds work packages, and
for most an engine module, and changes no shape, which is what its entry in the extension map shows.

1. **3D collision queries and the 3D character move in the first release?** Without them the 3D SDK serves
   showcases and not games, and a 3D match has nothing to run into. **Recommended: yes, as queries and a
   kinematic move only, with no rigid bodies** ([entry](#3d-space-and-a-kinematic-character)). It needs a
   deterministic 3D index and fixed collision from meshes and a heightfield, as an engine module of its own.
2. **Gamepads in the first release?** A platformer and a third-person game are played on one. **Recommended:
   yes** ([entry](#gamepads)). It needs pad input in the shells and focus by pad in `GAME-UI`.
3. **Audio in the first release?** Every dogfood game named it as the gap between a prototype and a game.
   **Recommended: yes, the small form** ([entry](#audio)), behind an engine module. No agent can hear it, so
   its items wait for the owner's ear.
4. **Predicted projectiles and predicted contact between players in the first release?** A fast
   player-against-player game feels a round trip late without them. The swarm game, a co-op crowd, needs
   neither. **Recommended: no, first in the release after** ([entry](#predicted-projectiles-and-contact)):
   it is a declaration on a control line that changes no rule, and only a person playing such a game can
   judge it.
5. **Hits judged as the shooter saw them in the first release?** Only a competitive shooter needs it.
   **Recommended: no, after question 4** ([entry](#hits-judged-as-the-shooter-saw-them)). Until then the
   design says plainly that a hit is decided at the shared tick.
6. **Multiplayer for 3D games in the first release? Decided: yes** (the owner, 27 November 2026, O6).
7. **A host with no window?** `SESSION` already hosts headless. **Recommended: yes**, as one command in the
   online work package ([entry](#smaller-entries)).
8. **Skeletal animation** stays out (O4). Both third-person games read as prototypes without it.
   **Recommended: keep it out of the first release**, and plan it first among the engine modules that follow
   ([entry](#skeletal-animation)).
9. **A text field and scrolling lists?** A player's name comes from the system account until there is a field.
   **Recommended: no for the first release** ([entry](#text-fields-lists-and-more-controls)).
10. **Levels from an editor's file (LDtk, Tiled, glTF placements)?** Hand-typed tile rows and placement tables
    do not scale past a demo, and D2 rules out an editor of Thrum's own, not reading someone else's.
    **Recommended: after the first release, with data packs** ([entry](#levels-from-an-editors-file)).
11. **Saved progress for every player, kept on each player's own machine?** In the first release a match's
    saves are the host's, so a guest keeps nothing. The swarm game's long game depends on it.
    **Recommended: yes** ([entry](#saved-progress-for-every-player)), in the saves and online work packages.

### Physics, animation and audio

Decided by the owner on 25 November 2026 ("Planner's default"), and unchanged here.

| | The first release |
|---|---|
| **Physics** | **2D queries in, rigid bodies out.** Within 2D queries: rectangles and capsules beside the engine's circles, a level's fixed collision and a character move that slides and reports its contacts. Nothing pushes anything. 3D is question 1 |
| **Animation** | **Sprite animation in**: a clip a screen plays (`Shown<Sprite>`) and a clip that loops in step for everyone (`Sprite::looping`). **Skeletal animation out** |
| **Audio** | **Out.** It is question 3 |

## What a game maker must see

| Cue | Where it must hold | Bar |
|---|---|---|
| K1 A first game is one file and one command, and its first page ends with a character you steer | The 2D and the 3D starter, on the Mac and in Chrome | Godot's "Your first 2D game" and macroquad's first window: nothing to configure before something moves |
| K2 The code reads as the game | Every worked example and template: a scene function reads top to bottom as a description of play, and no line names a device, a frame host, a page, a permit or a schema number | Bevy's `breakout` example, with less ceremony |
| K3 One structure | 2D, 3D, alone and online: the same `Game`, scenes, rules and frame functions. Turning multiplayer on changes no rule | Unreal's gameplay framework, where one set of rules serves a listen server and a client |
| K4 Defaults look finished | A 3D scene that asks for `Sky::clear()` and nothing else gets the physical sky and Sun, shadows, exposure, tone mapping, reconstruction, EDR where the display has it and the governor. A 2D scene gets crisp sprites, drawn in order | The showcase's own look, with no setting touched |
| K5 Mistakes explain themselves | At compile time, at startup and while running, across [the listed mistakes](#12-mistakes-that-explain-themselves), each with a stable code | Rust's own compiler messages: the object, the cause, the next step |
| K6 Every public item is documented with an example that compiles | All of `thrum` | The Rust standard library's reference |
| K7 The docs are enough | A fresh agent builds a small 2D game, a small 3D game and a networked game from the guides alone, then makes a change the guides did not show | The first hour with Godot's or Bevy's getting-started guides |
| K8 The speed is kept | The reference crowd's behaviours at 10,000 bodies, written as SDK rules, beside the hand-written reference; a game links, compiles and prepares nothing it does not use | The engine's own figures, as tripwires |
| K9 A game is proved without a window | Rules stepped, input scripted, entities found and state read in an ordinary `#[test]`; a recorded run replayed in a test; one command writes a picture | An agent can prove a rule, keep a level beatable and look at a frame |
| K10 Nothing is guessed | Every call a worked example makes has its name and signature under **Names** in [the core](#the-core) | The dogfood reports' lists of guesses, each closed |
| K11 Nothing bends the shape | Every entry of [the extension map](#the-extension-map) adds names at a named extension point and changes no name or invariant of the core | A feature added a year on reads as if it had always been there |

## The game at a glance

Fifteen words carry the whole SDK. Each means the same thing in a 2D game, a 3D game, alone and online.

| Word | What it is | In code |
|---|---|---|
| **Game** | The program: its scenes and what holds for all of them | `Game::new("Rockfall").scene(play).run()` |
| **Scene** | One part of the game with its own setup, rules and look: a level, a menu, a match. One runs at a time. A scene is a type, and its fields are what it is entered with | `scenes.go_to(GameOver { dodged: 12 })` |
| **Phase** | One stage of a scene, with its own rules: a countdown, a wave, an intermission | `scene.phase(Stage::Wave).rules((..))` |
| **Entity** | One thing in the world | `entities.spawn(ROCK.at(x, 10.0))` |
| **Component** | A plain piece of data on an entity | `#[derive(Component)] struct Health(i32);` |
| **Resource** | A plain piece of data the world has one of | `#[derive(Resource)] struct Score { dodged: u32 }` |
| **Prefab** | A named, reusable thing to spawn, with its defaults, its variants and its children | `prefabs! { pub const BAT: Prefab = "bat"; }` |
| **Rule** | A function that moves the world on by one tick. Rules are the only code that changes the world | `fn fall(mut rocks: Query<(&Rock, &mut Transform2d)>, tick: Tick)` |
| **Frame function** | A function that runs once for each picture: it reads the world as shown and sets what only this screen shows | `fn status(mut ui: Ui, run: Res<Run>)` |
| **Action** | A named thing a player can do, a field of an action map, bound to keys, pointer and touch | `#[bind(Key::Space)] jump: Button` |
| **Command** | A message a player sends once, such as a menu choice | `outbox.send(PickArena(arena))` |
| **Player** | Who an input came from: a person at a device, or a bot. A game played alone has one | `inputs.of(player).jump.pressed` |
| **Look** | A component the engine draws: `Sprite`, `Shape2d`, `Model`, `Light` | `Sprite::new(art::BAT)` |
| **Content** | A typed record the game's rules read, loaded from a file and named by a stable ID | `waves.get(run.wave)` |
| **Setting** | A named value on the tuning panel | `#[slider(4.0..=14.0, default = 9.0)] run_speed: Num` |

Four sentences hold the model together, and the guides open with them:

1. **Rules change the world, at the tick.** The world is its entities and resources, and nothing else.
2. **Frame functions show the world, at the frame.** They cannot change it. They set what this screen shows,
   and they send input.
3. **Everything that changes the world comes in as input at a tick**: a held action, a command, a player
   joining, a pause, a save restored. That is why a game can be replayed, tested and played online without
   being rewritten.
4. **Everything happens in the order it is written.** A scene function reads top to bottom as one tick, and
   what a line did is what the next line sees.

A feature the SDK gains later arrives as more names at one of [eight extension points](#the-extension-map): a
parameter a rule may take, a parameter a frame function may take, a look, an action kind, a line of a game or
a scene, a kind of input, a kind of asset or content, a command of the tool. None of the four sentences
changes.

## The core

This is what the owner locks. Each section gives **what always holds** and then the **names** a game touches,
with signatures. A name that is in neither this chapter nor a worked example is not in the first release.
Each invariant here is a mechanism a builder builds as written
([what a builder may change](#what-a-builder-may-change)).

### 1. A game: one crate, one package, one value

- A game is one Cargo package that depends on `thrum` and names no other engine crate.
  `use thrum::prelude::*` brings the fifteen words and the maths. No public item of `thrum` names a type of
  `wgpu`, `winit` or another engine crate.
- **Everything a game writes compiles with no window.** Scenes, rules, frame functions, the interface, cameras
  and looks are plain data and functions. That is `thrum` with its default features off, and what a test, a
  headless host and the rules check use. The default features add `run()`. The `multiplayer` feature, off by
  default, is the only way the network crates enter a game's closure.
- **`pub fn game() -> Game` is the one value `main` runs and a test steps.** The templates keep it in
  `src/lib.rs`, so tests under `tests/` reach it as well as tests beside the code.
- **A game's rules are its `rules` module**: `src/rules.rs`, or `src/rules/` and everything under it. It holds
  the game's components, scene types, action maps, prefab names and rules. [The check](#8-numbers) builds it
  on its own, beside the generated asset catalog and against `thrum` alone, and holds it to the portable
  profile. So a rule cannot call a helper elsewhere in the game.
- **Every function that makes or changes the world is defined in that module**, checked at startup from each
  function's path, in `test()`, `run()` and `cargo play --check` alike. Those are the functions registered by
  these routes and no others: a rule in `scene.rules` or a phase's `rules`; a control function; a spawn
  function given to `scene.players().spawn(..)`; an enter or exit function of a scene or a phase; a condition
  in `.when(..)`; a capture's condition; and the function a lobby is given in `.starts_with(..)`. A value the
  description gives the world directly (a resource's starting value, a prefab's definition, a shared setting's
  default) is part of [the rule-set identity](#11-multiplayer-an-opt-in-over-input-only-networking).
- A one-file game has no `rules` module. It runs, `cargo play` says once that its rules are not checked and
  how to move them, and `Game::multiplayer` refuses to start it. The templates have the module from the first
  line. A game that outgrows one package may move the module to a package of its own, whose manifest says
  `[package.metadata.thrum] rules = true`: the same check on the same code.
- **The repository's guard** holds every package in a games repository, under the engine's `templates/` and
  under its `examples/` to `thrum` alone, from WP8
  ([ruling 5](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), with the packages written on the engine's own
  API listed in the registry, each with its reason.

**Names.**

```rust
Game::new("Skylark")
    .scene(f)  |  .scene((rules::level, screens::level))   // one function, or several that describe one scene
    .add(rules::prefabs)                        // fn(Game) -> Game, to keep a part of the description elsewhere
    .resource(value)  .component::<T>()  .prefab(NAME, definition)
    .settings::<T>()  .local_settings::<T>()  .content::<T>("folder")
    .saved::<T>(slot)  .save::<T>(capture)  .save_if::<T>(capture, condition)
    .multiplayer(Multiplayer::players(1..=4))  .lobby(Lobby::standard().starts(Arena))
    .max_entities(n)  .tick_rate(hz)
    .run() -> thrum::Result
    .test() | .test_in(scene) | .test_setup() | .test_match()      // section 13
```

### 2. The world: entities, components, resources and prefabs

- **The world is `thrum-core`'s registered simulation, and the SDK adds no storage.** The world is its
  entities and resources. Everything the SDK itself must remember about a run (the scene and phase, seats,
  open offers, the pause, the level's recipe) is a resource it registers there, so it is digested, rewound
  and sent to a late joiner with the rest.
- **A component is `Copy` plain data.** `#[derive(Component)]` writes its registration, its stable schema
  identity (from its name and fields, never from a Rust type ID, as C01 requires), its encoding and its
  finiteness check. A resource, an event, a command and a scene's fields hold the same kinds:

  | Kind | Types |
  |---|---|
  | Numbers | `bool`, the fixed-width integers, `Num`, `Vec2`, `Vec3`, `Angle`, `Rot3`, `Seconds` |
  | Time | `TickNumber`, `Timer`, `Cooldown` |
  | Names | `Entity`, `Player`, `Id<T>`, `Prefab<N>`, `Layer`, `Mark`, `Part` |
  | Text | `Text<N>`: up to `N` bytes of UTF-8, such as a player's name |
  | Shapes | `Option<T>`, a fixed array `[T; N]` and a tuple of these; a struct or an enum that derives `Data` |
  | In a resource only | `List<T, N>`, a list of at most `N`; `PerPlayer<T>`, one `T` for each seat |

  A `Vec`, a `String`, a reference, a raw float and `usize` are each a compile error that names what to use. A
  `List` is an ordinary value besides, so a rule may keep one as a local between two passes. Long text and
  tables belong in [content](#10-content-assets-saves-and-settings), which rules read and cannot change.
- **A resource has one of two lifetimes, said where it is registered.** `scene.resource(value)` is set to that
  value each time the scene is entered. `Game::resource(value)` keeps its value across scenes. The running
  scene's own value, such as `Level { def }`, is read as `Res<Level>` and cannot be written.
- **An entity is a handle, valid until the entity is despawned.** Entities belong to the scene they were
  spawned in and leave with it. The world holds a fixed number, `Game::max_entities(n)`, because the engine
  reserves its storage once, and a spawn past it is refused with a message.
- **`spawn` returns the entity's handle at once.** The entity itself appears when the rule finishes
  ([section 3](#3-rules-and-the-tick)), so until then the handle can be stored in a component, attached under
  a parent and given to a later spawn as its parent, and nothing else.
- **What the world holds is fixed before the first tick**, from what the game's description shows at startup:
  the signatures of every function the scenes list; every prefab's bundle and every spawn function's return
  type; the SDK's own components; and `Game::component::<T>()` for the rare one none of those shows. Spawning
  a component the list lacks is refused, naming the rule, the component and the line to add.
- **A prefab is a named thing to spawn** (D2, R3). Its name is a typed constant made in a `prefabs!` block and
  nowhere else, and the game gives each name one definition before the first tick: a bundle, or `.variant()`
  of another prefab with changes. A variant follows its base and is resolved at startup.
- **`Prefab<N>` says in its type what a spawn must bring.** `SHOT.at(..)` without its `Shot` and `Velocity` is
  a compile error, so no prefab holds a made-up owner or a zero that means "not set". `.with(c)` means one
  thing everywhere: this spawn, or this variant, has `c`. It brings what the prefab needs, replaces a
  default, or adds.
- **No name an author wrote waits for its use to be checked.** Every `prefabs!` block the game's code refers
  to registers itself with the built program at link time (the technique of Rust's `inventory` crate), so the
  SDK knows every declared name before play on every target, with no line of the game listing the blocks. At
  startup each name must have a definition, and each definition's components, assets, children and parts are
  checked, with [the access check](#3-rules-and-the-tick) against every rule. A prefab a content record
  names is checked with the content.
- A bare tuple is still a bundle, and a function that returns a bundle or a spawn is still a good way to
  compute one. A prefab is for what is spawned by name, varied or checked early. A look that needs a
  transform brings one at the origin when the bundle has none.
- **Children.** A child stores a local transform, is drawn and queried at its parent's world transform, and
  is despawned with its parent. A **part** is a child with a name, declared in the prefab: a warden's eye, a
  place on a back. A part with no look is a socket. Entities with no parent pay for no hierarchy. A cycle is
  refused. A reference that is not ownership, such as a target, stays an ordinary `Entity` field.
- **Attaching says where the child ends up.** `keep_local(pose)` cannot fail. `keep_world()` stores the local
  transform that recomposes nearest to where the child stood and **promises a bound, not exactness**
  ([C01](../../design/CONTRACTS.md)): one part in 2^17, exact only where every number on the way composes
  exactly. It is refused, with the tick and with nothing changed, under a parent whose scale through its
  parents is zero or outside 1/10,000 to 10,000, where the result would need shear, or past the bound, which
  is tested on every call. `detach` is held to the same rule.
- **A disabled entity** keeps its components and its place and is skipped by queries, space queries and
  drawing, with its children, until it is enabled.

**Names.**

```rust
#[derive(Component)]  #[derive(Resource)]  #[derive(Data)]  #[derive(Event)]

prefabs! {                                             // a game's names, declared together
    pub const BAT: Prefab = "bat";
    pub const SHOT: Prefab<(Shot, Velocity)> = "shot";   // needs these from whoever spawns it
}
Game::prefab<N>(self, name: Prefab<N>, definition: impl Into<PrefabDef>) -> Game
BAT.variant()                                          // a definition that starts as another prefab's
    .with(Health(8))  .tweak(|sprite: &mut Sprite| sprite.tint = Color::GOLD)
    .child(AURA.at_local(0.0, 0.2))  .part(EYE, WARDEN_EYE.at_local(0.0, 1.6, 0.2))
PrefabDef::new(bundle).child(..).part(..)              // the same for a prefab that is no variant

// A spawn: a prefab at a place, with this spawn's own differences.
BAT.at(3.0, 0.0)  .at_point(p)  .at_local(x, y)  .turned(angle)  .scaled(s)
   .with(component)  .tweak(|c: &mut C| ..)  .under(parent)

// Entities, in a rule. Every edit lands when the rule finishes.
entities.spawn(spawn_or_tuple) -> Entity       .despawn(e) -> bool          // true for the call that despawns it
        .insert(e, component)                  .remove::<C>(e)
        .attach(child).to(parent).keep_world() | .keep_local(pose)
        .detach(child).keep_world() | .keep_local() | .at(pose)
        .disable(e)                            .enable(e)
// The tree, read as the rule found it. `Family` is these alone, for a function that changes nothing.
        .alive(e) -> bool       .parent(child) -> Option<Entity>       .children(parent)   // in the order attached
        .part(parent, PART) -> Entity          // refuses the tick if its prefab has no such part
        .world2d(e) -> Transform2d             .world3d(e) -> Transform3d   // its place in the world, through its parents

Transform2d::at(x, y) .turned(angle) .scaled(s)          // fields: position, turn, scale
Transform3d::at(x, y, z) .turned_y(angle) .facing(direction) .scaled(s)   // fields: position, rotation, scale
    // on either: teleport(point); in 3D: face(direction), turn_y(angle), forward() -> Vec3, heading() -> Angle
Layer::new("enemies")   Mark::new("start")   Part::new("back")           // typed constants
```

### 3. Rules, and the tick

- **A rule is a function, and its parameters say what it reads and writes.** A rule is a function of its
  parameters (D1): the same world, input and tick give the same result on every machine and in every replay.
  Nothing a rule can be handed leads to a clock, a file, the network, the window or one player's screen, and
  [the check](#8-numbers) refuses the ordinary ways of reaching round that.
- **Written order (R2).** A scene function is one list. Each tick the SDK walks it top to bottom and runs each
  entry once: a `scene.rules(..)` block's rules in turn, a `scene.players()..control(f)` line for each
  player's character in seat order, and a `scene.phase(p)` block's rules when `p` is the scene's phase.
  `rules` may be called any number of times and tuples nest, so no list is too long to write.
- **While a rule runs, the world stands still around it.** Its own writes to components and resources are in
  place at once. Its spawns, despawns, inserted and removed components, attachments, disabling and changes to
  the level are held until it finishes, so nothing appears or vanishes under a loop.
- **When a rule finishes, everything it did has happened.** The next rule's queries visit an entity the last
  one spawned and not one it despawned, and space queries answer from where things are now. The engine
  resolves structural edits when a tick ends today. The SDK asks core for a barrier after each registered
  system, inside the tick's one transaction
  ([ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), and WP1's first item proves it before anything
  is built on it.
- `entities.despawn(e)` returns whether this call is the one that despawns it, so two heroes on one gem count
  it once.
- An event a rule sends is read by later rules of the tick, by frame functions, and by no rule of the next
  tick.
- **The whole tick commits or rolls back as one.** A rule returns nothing or a `thrum::Result`. A returned
  error refuses the tick on every target: the world is as it was before the tick, the game stops with the
  rule's name, the tick and the error, and in a test `step()` returns it and the game can be stepped again.
  A panic is a bug in the game. On the Mac and the simulator it is caught and reported the same way. On the
  browser target, which cannot unwind, the program stops after the SDK's hook has put the rule, the tick and
  the message on the page.
- **What a rule may touch comes from its signature alone.** A signature names components in accesses: each
  `&T` and `&mut T` in a `Query` or a `Single`, and each a control function takes for its character. **Two
  accesses of one function conflict when they name the same component, at least one writes it, and one entity
  is reached by both.**
  - Inside one parameter both always reach the same entity, so `Query<(&mut Health, &Health)>` is refused at
    startup whatever entities exist. A writer reads too.
  - Between two parameters the SDK decides from **the sets of components that exist**, not from filters: every
    prefab's set at startup, and each new set when the rule that made it finishes, by a bare spawn, an
    `insert` or a `remove`. A set two parameters of some rule conflict over refuses that tick and names the
    rule, the parameters, the component and the entity. So parameters that match nothing in common need no
    `Without`, and no rule ever starts with an entity two of its parameters could both reach.
  - A control function's own accesses reach one entity, the character its player has, and its queries are
    tested against that entity: at startup, when a rule changes the character's components, and when
    `possess` gives the player another.
  - What a rule looks things up through (`Space2d`, `Level2d`, `Family`, the tree on `Entities`) reads the
    world as the rule found it, so it never reads a value the rule is writing.
- **A struct of parameters is a parameter.** `#[derive(Params)]` on a struct whose fields are rule parameters
  makes one a rule may take, with methods of the game's own, so rules that share work share one line. The
  access check and the read-only bound see through it to its fields.
- **A read-only parameter** cannot change the world: a `Query` or a `Single` with no `&mut`, `Res<T>`,
  `Family`, `Inputs<A>`, `Players`, `Events<T>`, `Tick`, `Random`, `Space2d`, `Level2d`, `Phase<P>`,
  `Content<T>` and `Restored<T>`. A spawn function, a condition and a capture take read-only parameters and
  nothing else, by a bound on the type: what a function may do is fixed by the type it asks for and never by
  how it binds the value.
- **Parallel work is inside a rule.** `query.par_each(..)` runs the closure over the engine's chunks. It
  writes only the entity it is given, and the result is the serial loop's by construction.
- **Cadence and conditions.** `rule.every(seconds(1))` runs a rule once each interval, counted from when its
  phase or scene was entered, first at the end of the first interval. `rules.when(condition)` runs a rule or a
  group only in ticks where a read-only function returns true. A cadence that belongs to one thing is a
  `Cooldown` on that thing.
- **The tick** is 60 Hz unless `Game::tick_rate` says otherwise. Speeds are written per second and multiplied
  by `tick.dt()`.

**Names.** Every parameter a rule may take:

| Parameter | What it gives |
|---|---|
| `Query<D, F>` | The entities that have `D`, filtered by `With<T>` and `Without<T>`, in the order they were spawned. `&T` reads, `&mut T` writes, `Entity` names. `iter`, `get(e)`, `get_mut(e)` (each takes an `Entity` or an `Option<Entity>`), `of_player(p)`, `count`, `is_empty`, `par_each` |
| `Single<D, F>` | The one entity that matches, used as its item; `into_inner()` to take it apart. None or two refuses the tick and says which. `Option<Single<..>>` is none or one |
| `Res<T>`, `ResMut<T>` | A resource |
| `Entities`, `Family` | [Section 2](#2-the-world-entities-components-resources-and-prefabs) |
| `Inputs<A>`, `BotInputs<A>`, `Players`, `PlayersMut` | [Section 5](#5-players-actions-and-control) |
| `Commands<T>`, `Offers<T>` | [Section 6](#6-commands-offers-and-requests) |
| `Events<T>`, `Emit<T>` | Events of this tick: `events.read()`, `emit.send(e)`. The game's own, and `PlayerJoined`, `PlayerLeft`, `PlayerAway`, `PlayerBack` |
| `Tick` | `number()`, `dt()`, `seconds()`, `in_scene()`, `in_phase()`, and `every(d)`, `after(d)`, `reached(t)` |
| `Random` | Draws keyed by the game's seed, the tick and the rule that asks: `between(a, b)`, `below(n)`, `chance(p)`, `angle()`, `direction()`, `pick(..)`, and `for_entity(e)` inside `par_each` |
| `Space2d`, `Level2d`, `LevelMut2d` | [Section 7](#7-space-and-the-level) |
| `Scenes`, `Phase<P>`, `PhaseMut<P>` | [Section 4](#4-scenes-phases-time-and-pause) |
| `Content<T>`, `Saving`, `Restored<T>` | [Section 10](#10-content-assets-saves-and-settings) |
| A struct that derives `Params` | Its fields, each a parameter from this table |

```rust
scene.rules((steer, fire.every(seconds(0.4)), (hit, die).when(wave_is_on), end_talk.even_when_paused()));

#[derive(Params)]                           // 't is the tick the rule runs in
pub struct Strikes<'t> { shades: Query<'t, &'t mut Health, With<Shade>>, ledger: ResMut<'t, Ledger>, struck: Emit<'t, Struck> }
impl Strikes<'_> { pub fn strike(&mut self, shade: Entity, damage: i32, by: Player) { /* .. */ } }
fn pulse_halos(halos: Query<(&Halo, &Transform2d)>, space: Space2d, mut strikes: Strikes) { /* .. */ }
```

### 4. Scenes, phases, time and pause

**Scenes.**

- **A scene is a type and a function.** The type derives `Scene`, and its fields are what the scene is entered
  with. The function describes it: `fn level(scene: &mut Scene<Level>)`. `Scene` alone is `Scene<()>`, for a
  game with one scene. `Game::scene` takes one function or a tuple of functions that describe the same scene,
  which is how a game keeps a scene's rules in `rules.rs` and its screen in `screens.rs`.
- The game starts in the first scene listed, entered with its `Default` value. A first scene with no default
  is a compile error. `game.test_in(scene)` and `cargo play --scene <name>` start elsewhere.
- **Which scene is running, and its value, are world state.** A match's players are always in the same scene,
  and a late joiner is sent it.
- **A tick makes one transition at most**: a change of scene or a change of phase, never both. The tick's
  requests are taken in this order, and the first stands: one an enter or exit function made while the last
  tick's transition was landing; a `request.scene` admitted at this tick; the rules' own, in written order,
  `scenes.go_to`, `scenes.restart` and `PhaseMut::go_to` alike; a phase's `lasts`, which counts as asked after
  every rule. A later request returns `false` and changes nothing, and its caller may ask again in a later
  tick. The rest of the tick's rules still run, in the scene and phase the tick began in, and the transition
  lands when the tick ends. So the rule written first decides, whichever kinds the two requests are.
- **A request made while a transition is landing waits a tick.** A `go_to` made by an enter or exit function
  is held, and is the next tick's first request. A held request for a phase lives no longer than its scene:
  asked by an exit function of a scene that is being left, it is refused and nothing is held.
- **Leaving:** the phase's exit functions, then the scene's, then every entity of the scene is despawned.
  **Entering:** scene resources are set, then the scene's `on_enter` functions and its `players()` lines run
  in the order written, then the first phase's enter functions. All of it happens in the tick that asked, so
  the next tick's first rule finds the scene whole.
- A scene's kind (2D or 3D) follows from its camera. A scene needs no camera: without one it shows its
  background and its interface, which is what a menu and a result page are.
- **A frame function asks for a scene with `request.scene(..)`**, honoured only where the running scene says
  `scene.accepts::<Target>()`. In a match the host's request is the one admitted, unless the scene says
  `.accepts_from_anyone::<Target>()`. Anything more particular is a command and a rule.

**Phases.**

- A phase is a value of an enum that derives `Phase`. `scene.phases(first)` gives the scene phases and names
  the one it starts in. A `scene.phase(p)` block is a place in the scene's written order.
- A change of phase is one of the tick's requests and lands when the tick ends: the old phase's exit
  functions, then the new one's enter functions. Going to the phase the scene is in leaves and enters it.
- Rules outside any phase block run in every phase. A frame function and a control function read `Phase<P>`.
  Only a rule takes `PhaseMut<P>`.
- `lasts` takes a time, or a read-only function that returns one, asked when the phase is entered, so a
  night's length can come from its content file.
- **A scene entered from a save resumes its phase and enters none.** An `on_enter` function of the scene may
  call `PhaseMut::resume(phase, time)`: the scene then starts in that phase with that much time on its clock,
  a `lasts` counts on from there, and no phase's enter function runs, because nothing was entered. Called
  anywhere else it returns `false` and changes nothing.

**Time belongs to the thing being timed** (R5). All of it is counted in ticks and is world state, so it
rewinds, replays and saves. Coyote time, a jump buffer and a dash are `Timer` windows. A weapon's rate is a
`Cooldown` on the hero that holds it. A phase with a length uses `lasts`. A `Timer` or a `Cooldown` in a save
is written as the time it has left, and comes back with it.

**Pause.** A pause is one thing, alone and in a match.

- **Whether the game is paused, and by whom, is world state**, and asking for a pause and ending one are input
  admitted at a tick. Alone the one player may always pause. In a match it is the game's choice:
  `Multiplayer::players(..).pause(Pausing::Nobody)`, the default, `Host` or `Anyone`.
- **While paused, ticks still commit and the game's clock stands.** `Tick` gives the same number, so every
  timer, cadence, phase clock and tick-timed animation waits. No rule or control function runs, and no held
  input is read. There is nothing to catch up afterwards.
- **What still happens in a paused tick:**
  - Frame functions run, so a menu works.
  - The roster changes at once. A join's event and character come at the first tick after the pause.
  - **Commands, answers, scene requests and a shared setting's edits are admitted.** A rule marked
    `.even_when_paused()` runs in its written place and reads the commands and answers its signature names. A
    command or an answer no such rule takes waits in the world for the pause's end and is read once then.
  - A scene request lands, and ends the pause. A save asked for is captured.
- So what a menu does under its own pause works: quit to the title, save here, restart the level, or end the
  talk whose box holds the pause.
- **A press made while the presser's own screen shows the pause is never read as play**, and one made in play
  is read once ([section 5](#5-players-actions-and-control)).
- `ui.modal(..).pausing()` asks for the pause while the modal is open, where this player may pause, and is an
  ordinary modal where they may not. `scene.unpausable()` is for a scene where a pause makes no sense.

**Names.**

```rust
#[derive(Scene)] pub struct Level { pub def: Id<LevelDef> }       // implements `SceneType`
#[derive(Phase)] pub enum Stage { Playing, Dying }                 // implements `PhaseType`

// In a scene function, each line a place in the written order:
scene.resource(value)  .on_enter(f)  .on_exit(f)  .rules((..))  .add(part)
scene.players().spawn(..).control(..)                              // section 5
scene.phases(Stage::Playing)
scene.phase(Stage::Wave)
    .on_enter(next_wave)  .rules((steer, fire, hit))  .on_exit(clear_enemies)
    .lasts(seconds(60))  .then(Stage::Intermission)
scene.accepts::<Level>()  .accepts_from_anyone::<Level>()  .unpausable()

Scenes:       go_to(scene) -> bool,  restart() -> bool        // false when the tick's one transition is taken
Phase<P>:     get() -> P,  is(p) -> bool,  time() -> Seconds  // since it was entered, or resumed
PhaseMut<P>:  everything `Phase<P>` has, and go_to(p) -> bool,  resume(p, time) -> bool
Timer:        a deadline that may not be set.  Timer::IDLE,  Timer::after(tick, wait),  start(tick, wait),  clear(),
              running(tick): set and not yet reached, a window;  finished(tick): set and reached;  left(tick) -> Seconds
Cooldown:     something that may happen again after a wait.  Cooldown::READY,  ready(tick),
              take(tick, every) -> bool: if ready, starts the wait and returns true
Pause:        on(),  off(),  toggle(),  is_on() -> bool,  by() -> Option<Player>              // in a frame function
Request:      scene(Level { def }) .restoring::<T>(slot);   quit()                             // in a frame function
```

`#[derive(Scene)]`, `#[derive(Phase)]` and `#[derive(Content)]` implement `SceneType`, `PhaseType` and
`ContentType`. Bounds are written on those, and a game maker writes the derive and never the trait's name.

### 5. Players, actions and control

**Actions.**

- **An action map is a struct whose fields are the things a player can do** (R7), and a rule reads it as plain
  fields. `#[derive(Actions)]` also writes a name for each field, `Controls::JUMP`, for the places that name
  an action and do not read it: a test, a key prompt, a camera rig, a remap screen.
- **The bindings on the map are the defaults.** Each player can change theirs, and a player's bindings and
  local settings are kept on their own machine. A field with no `#[bind]` and no `#[sent]` is a compile error.
- **The action maps a game's rules read are its action set**, known from their signatures. It is the same on
  every peer, fixes the layout of the input frame, and is part of the rule-set identity. It must fit
  `SESSION`'s frame: three 16-bit axes, into which the SDK packs every button, `Axis` and `Stick`, and up to
  eight actions of 256 bytes. A `Heading` and a `#[sent]` value travel as one action that carries the latest
  value, which a late or repeated frame leaves standing. Startup refuses a set that does not fit, with the
  counts and the limit.
- **A press is read once: never twice, and never lost while no more presses fall between two admitted frames
  than the count holds**, which is at least three and which the reference states. A button travels as whether
  it is held and a small running count of presses, beside a bit that says the frame was sampled, and the
  world keeps the last count it read. So a frame the session repeats presses nothing; the neutral frame for
  an absent player releases everything and keeps the count; a press on a late frame is read from the next
  that arrives; and a joiner starts its counts from the world's.
- **A pause and play are kept apart where the press is made.** While a player's screen shows the pause, their
  machine counts no press, and a key that goes down then is ignored until it is let go. A key held since
  before stays held. A press made while their screen still showed play is read once, in the first tick after.
- **A `Heading` belongs to the player's machine.** Bound to the mouse, it turns at the frame rate with the
  pointer captured, so a camera that reads it has no delay, and its latest value travels with each tick's
  input, so "forward" in a rule is the way the player was looking. `#[sent]` is for a value a frame function
  works out, such as the direction from a craft to the pointer.
- **Local actions never leave the machine.** `#[derive(LocalActions)]` is for keys only the screen cares
  about: pause, a scoreboard, a card's hotkey. A frame function reads them with `MyInput<Menu>`. A rule cannot
  take them, they are in no input frame, digest or replay, and they are still read while a modal is open.
- **An input context** says which action maps this screen's keys, pointer and touches drive now, so one key
  can jump in play and buy in a shop while the world keeps ticking. A screen has one at a time,
  `Screen<InputContext>`. A shared map with no `#[context(NAME)]` is in `InputContext::PLAY`, and a local map
  with none is always on. A map whose context is off is sampled as idle, and a shared map stays in every input
  frame with the same layout, so a context never changes what a match agrees on. A key that went down while a
  context was off is ignored by it until the key is let go.

**Players.**

- **A player is who an input came from, and who is in the game is world state.** `Players` only looks, and is
  what a frame function, a spawn function and a condition are given. `PlayersMut` changes the roster and is a
  rule's alone.
- **A `Player` names one stay in a seat.** It is a seat and a count of the joins that seat has seen, as an
  `Entity` is a slot and a generation. A value kept after its player left, such as a shot's `pilot`, never
  names whoever sits there next: `players.status(p)` is `Gone`, `PerPlayer::get(p)` is `None` and
  `Query::of_player(p)` finds nothing. `player.index()` is the seat, for a place in a line or a colour.
- `PlayerJoined` is sent once for a player, at the tick they enter the game, to whichever scene is running. A
  later scene finds who is here with `Players`. `PlayerLeft` carries the player and their name, so a rule can
  keep what should outlive them. `PlayerAway` and `PlayerBack` come at the tick of each.
- `PerPlayer<T>` keeps one value for each seat, such as a score that outlives a character. It returns to its
  default when the seat's player leaves.
- **Seats.** A world has four seats, `SESSION`'s four slots, and `Multiplayer::players` says how many a match
  uses. Which are taken, by a person or a bot, is world state, and `SESSION`'s slot numbers never reach a
  game. A person takes the lowest free seat and a bot the highest. **A person is never turned away for a
  bot**: where bots hold every free seat the person takes the lowest a bot holds, and in that tick the bot
  leaves as any player does. When people hold every seat a newcomer is refused as full, by the SDK: it hands
  `SESSION` no link while no seat is free or away, and gives no seat to a slot the world has no room for. An
  away player's seat is theirs for the grace.

**A controlled character** (D3, R1). One line of a scene gives every player a character and says how it moves.

- **`spawn`** takes a prefab or a function. The function may take `Joined` (who it is for) and any read-only
  parameter, so it can place the character at a level's start or in a line. It runs for each player already
  here when the scene is entered and for each who arrives later. The character is given that `Player` as a
  component. It makes world state, so it is defined in the `rules` module.
- **`control`** takes a function whose `&T` and `&mut T` parameters are components of that one character, with
  `&A` for that player's action map. It is the character's whole movement: it writes the pose itself. It runs
  where its line is written, once for each player's character.
- A character is despawned when its player leaves. While a player is away their character stays under idle
  input, unless `when_away` says otherwise. A rule that despawns a character brings it back with
  `players.respawn(player)`. A scene may have more than one `players()` line.
- **A control function may take any parameter a rule may, and a table says which of them a match runs
  ahead.** In a match the local player's own character answers at once when everything its function reads is
  in the first three rows:

  | Parameter | In a match, for the local player's character |
  |---|---|
  | Its own components, `&A`, `Tick`, `Entity`, `Random`, `Content<T>` | Run ahead exactly. Content and the seed never change, and everything `Tick` gives counts from entries the step's state carries |
  | `Res<T>`, `Phase<P>`, `Level2d` | Run ahead with the value at the last confirmed tick. A change arrives a moment later, as a correction. Free for settings, the phase, the level and anything that changes rarely |
  | `Emit<E>` | The event reaches this screen's frame functions when it is predicted, once, so a jump's dust is not late. Rules read it at the confirmed tick |
  | `Query`, `Single`, `Space2d`, `Inputs`, `Players`, `Family`, `Events<E>`, `Commands<T>`, `Restored<T>` | **Not run ahead.** They read what other machines move or send |
  | `ResMut`, `Entities`, `PhaseMut`, `Scenes`, `Offers`, `LevelMut2d`, `PlayersMut`, `BotInputs`, `Saving` | **Not run ahead.** They change more than the character |

  A function that takes anything from the last two rows still works everywhere, with its character shown as
  the host confirms it, a round trip late. So that nobody pays that by accident, a game with multiplayer on
  refuses to start until that line says `.unpredicted()`, and the message names the parameter that costs
  prediction. Alone, nothing is asked. A new rule parameter joins one row of this table when it is added, and
  [predicting more than the character](#predicted-projectiles-and-contact) moves named things up a row.
- **A bot is a player whose input a rule writes.** `PlayersMut::add_bot` takes a seat and is announced as any
  player is. A rule written before the control line sets the bot's map with `BotInputs<A>`: a button is set
  as held or not, and the world derives `pressed` and `released` from one tick to the next as it does for a
  person. From there nothing tells a bot from a person: the same spawn, control function, score table and
  offers, which a rule answers for it.

**Names.**

```rust
#[derive(Actions)]
pub struct Controls {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::stick_x())]
    pub run: Axis,                 // a `Num` from -1 to 1
    #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]
    pub walk: Stick,               // a `Vec2` no longer than 1
    #[bind(Key::Space, Mouse::LEFT, Touch::button("Jump"), Touch::tap())]
    pub jump: Button,              // .held .pressed .released: bool; .presses: how many since the last tick
    #[bind(Mouse::look_x())]
    pub look: Heading,             // an `Angle` the player's own machine keeps: .direction() .direction_xz()
    #[sent]
    pub aim: Heading,              // no device: a frame function sets it through `MyInput<Controls>`
}
#[derive(LocalActions)] #[context(SHOPPING)] pub struct ShopKeys { /* the same kinds of field */ }
pub const SHOPPING: InputContext = InputContext::new("shopping");      // InputContext::PLAY is the default

Inputs<A>:     of(player) -> &A,  iter(),  any(|a| ..)                  // in a rule
BotInputs<A>:  set(player, A),  is_bot(player)              // built from Button::held(bool), Axis::of(n), Stick::toward(v)
MyInput<A>:    this player's map as sampled this frame; `&mut` to set a `#[sent]` field    // in a frame function
Pointer:       position(), ray(&camera);  camera.world_point(&pointer), camera.ground_point(&pointer)
Bindings:      rebind(Controls::JUMP, input),  reset(),  prompt(Controls::JUMP)

Players:       iter() in seat order,  count(),  host() -> Player,  name(p) -> Text<24>,
               status(p) -> Here | Away { since } | Bot | Gone,  character(p) -> Option<Entity>
PlayersMut:    everything `Players` has, and add_bot() -> Option<Player> (None when every seat is taken),
               remove_bot(bot),  respawn(p) (runs the scene's spawn for them again),  possess(p, character)
PerPlayer<T>:  per[player],  get(player) -> Option<&T>,  get_mut(player),  iter() -> (Player, &T)
Player:        index() -> u32;   Player::FIRST
Me:            player(),  character()                         // in a frame function: which player this screen is
PlayerJoined { player }   PlayerLeft { player, name }   PlayerAway { player }   PlayerBack { player }

scene.players()
    .spawn(hero_at_start)              // a prefab, or a read-only function that returns what to spawn
    .control(run_and_jump)             // runs here in the written order, once for each player's character
    .during(Stage::Playing)            // optional: only in this phase
    .when_away(Away::Idle)             // optional: Idle (the default), Remove, or Bot
    .unpredicted();                    // required in a match when `control` reads what cannot be run ahead
fn hero_at_start(joined: Joined, tiles: Level2d) -> impl Spawn      // joined.player
fn run_and_jump(hero: &mut Hero, at: &mut Transform2d, body: &Collider2d, controls: &Controls, tick: Tick)
```

### 6. Commands, offers and requests

- **A command is a message a player sends once**, a value of a type that derives `Command`, sent from a frame
  function with `Outbox`. The SDK keeps commands that cannot go yet and sends them in order, so a menu never
  handles a full queue.
- **An admitted command is read once**, by `Commands<T>`, with who sent it, after the same player's earlier
  commands, on every machine: in the tick it is admitted, or, where a pause holds that tick and no rule that
  runs through the pause takes it, in the first tick after the pause.
- If the link drops while a command is on its way, the game cannot know whether it was admitted, and the
  player comes back to a fresh copy of the world. So an interface is drawn from what the world says, and a
  repeated command is made harmless. An offer does both for a choice.
- A rule that refuses a command says so with an event, which the sender's frame functions read. Startup
  checks that each command type's encoding fits one action of an input frame.
- **An offer is a question the rules put to players, answered once** (R8). Open offers are world state: a late
  joiner and a reconnecting player are shown theirs, and the cards stay until the answer has landed.
- `accept` gives the answers admitted this tick: one for each player and offer, from a player it was put to,
  for a choice it held, while it was open. Anything else is dropped.
- **A player's open offers of one type queue.** Their screen is shown the oldest, and the next when its
  answer has landed, so three level-ups in a second are three questions in turn with nothing to keep by hand.
- A player who joins while `to_each_player` or `to_all` is open is asked. A player's part in an offer ends
  when they leave. A person answers from their screen and a bot through a rule, both are taken by `accept` in
  seat order, and nothing downstream can tell them apart. An `OfferId` holds the scene's entry and a count,
  so a second run cannot answer the first run's offer.

**Names.**

```rust
#[derive(Command)] pub struct PickArena(pub Id<ArenaDef>);
impl Outbox { pub fn send<C: Command>(&mut self, command: C) -> Delivery; }   // in a frame function
#[must_use] pub enum Delivery { Queued, Closed }      // it will be admitted unless the link drops; the match has ended
for (player, pick) in commands.read() { .. }          // in a rule: `Commands<PickArena>`

impl<T: Data> Offers<T> {
    pub fn open(&mut self, offer: Offer<T>) -> OfferId;
    pub fn accept(&mut self) -> impl Iterator<Item = Answer<T>>;   // .player .choice .offer
    pub fn tally(&self, offer: OfferId) -> Tally<T>;               // how many chose each, and who has not
    pub fn close(&mut self, offer: OfferId);
    pub fn pending(&self, player: Player) -> Option<Pending<T>>;   // their oldest unanswered: .offer .choices()
    pub fn answer_for(&mut self, bot: Player, offer: OfferId, choice: T) -> bool;   // through the same check
}
Offer::to(player) | Offer::to_each_player() | Offer::to_all()      // one player; each for themselves; a vote
    .choices(choices)                                              // an array or an iterator
    .expires(tick.after(seconds(15))) | .until_phase_ends()
MyOffer<T>:  open() -> Option<..>, .choices();  choose(value);  status() -> Waiting | Sending | Taken   // frame
```

### 7. Space and the level

- **Colliders detect and never push.** There are no rigid bodies (O4). A collider is on one layer. Its sizes
  are full widths and heights, a capsule's length includes its caps, and it is centred on its entity's
  position unless `.offset(..)` says otherwise. In a 3D scene a `Collider2d` lies on the ground plane, x and
  z, so triggers, pick-ups and catch radii work in a 3D game with the first release's queries.
- **`Space2d` answers from where things are now** (R2): after every rule before this one, the last rule's
  spawns and despawns included. `at_tick_start()` is for a rule that wants the older view on purpose.
- **One index for each layer.** `.on(LAYER)`, with one layer or a tuple, searches those layers and nothing
  else, so an enemy that asks for the nearest of four heroes looks at four colliders with 10,000 enemies
  around it. A layer's index is rebuilt only when a rule has changed that layer since it was last built.
- **Two orders, by what was asked.** An overlap (`touching`, `within`, `in_rect`, `nearest`) answers nearest
  first. A ray or a sweep answers earliest contact first, by `fraction` along the path. Ties go to the
  earlier spawned.
- **`motion_of(entity).sweep()`** sweeps the entity's own collider along the path it took this tick, from
  where it stood at the tick's start or was spawned, against the paths the others took. It is how a fast shot
  hits and how a stomp knows it came from above.
- **The level is a scene's fixed collision** (R7). It does not move, so it is not made of entities. What the
  world holds is **the level's recipe**: which tile map is loaded, by that map's identity in the game's
  content, and each fixed shape a rule has added since, in order. The recipe is digested, rewound and sent in
  a baseline, and the collision is built from it, the same on every machine. The SDK keeps the built
  collision of every recipe still inside the rewind and prediction span, by the recipe's identity, so a
  joiner, a rewind and a step run ahead each get the level of their own tick.
- **A change to the level is a rule's act.** It lands when the rule finishes and is undone with a refused
  tick. The recipe holds a fixed number of added shapes, 256 unless `scene.level_shapes(n)` says otherwise.
- **A `Tilemap` is content**: rows of glyphs in a record's file. Each has an identity of its own, its record
  and the path of fields to it, so a record may hold several. A tile is one unit square unless
  `Tileset::tile_size` says otherwise, and a glyph's spawn and its mark stand at the centre of its cell. The
  tiles are drawn as one tile layer, not an entity each. A validator and a test read a map's cells.
- **Rays, sweeps and moves asked of a space meet everything on the layers asked: entities' colliders and the
  level's fixed collision.** One `Hit` says which: its `entity` is `None` for the level. So one ray tells
  whether a guard sees past a hill, and one sweep whether a bolt reached the pier or the hull first. Overlaps
  find entities. **`Level2d` asks the fixed collision alone, and that is the part a match can run ahead.**
- **A hit's normal points out of what was struck, toward the mover.** A sweep that starts in contact reports
  that contact at fraction zero, with the normal of the shallowest way out, never a zero one.
- **Moving a character is one call that slides along what it meets and reports it**: both axes in one step,
  so no game moves an axis at a time or compares positions to learn what happened. It is given the
  character's own collider, which a control function takes as a component, so a shape is written once.
  `Level2d::move_shape` meets the level and can be run ahead. `Space2d::move_shape` also meets the colliders
  on the layers it is given, such as a moving platform, reports the entity it stands on, and is
  `.unpredicted()` in a match. One-way tiles stop a shape that comes from above.

**Names.**

```rust
Collider2d::circle(r) | ::rect(w, h) | ::capsule(r, length)      .offset(x, y)  .on(LAYER)
scene.space2d(Space2d::cells(4.0));          // part of the rules, so the same for every player

// Space2d, in a rule:
space.touching(entity) | .within(point, radius) | .in_rect(rect) | .nearest(point)   -> Found
     .ray(from, to) | .sweep_circle(from, to, radius) | .motion_of(entity).sweep()   -> Swept      // motion_of(e).delta() too
     .move_shape(from, &shape, step) -> Move2d          .at_tick_start() -> Space2d
// Found:  .on(layers) .except(entity) .within(r) then .first() .any() .count() or iterate -> Near { entity, position, distance }
// Swept:  .on(layers) .except(entity) then .first_hit() -> Option<Hit>, or .hits() in order, for a shot that pierces
pub struct Hit { pub entity: Option<Entity>, pub layer: Layer, pub point: Vec2, pub normal: Vec2, pub fraction: Num }

scene.tiles(Tileset::new(art::TILES)          // how this scene reads a tile map
    .solid('#', TERRAIN)  .one_way('=', TERRAIN)  .spawns('o', COIN)  .marks('@', START)  .tile_size(1.0));
LevelMut2d:  load(&tilemap) (replaces the level: solids, spawns, marks, tiles),  add_rect(rect, LAYER),
             add_segment(from, to, LAYER),  clear()                    // in an enter function, or any rule
Level2d:     move_shape(from, &shape, step) -> Move2d,  ray(from, to) -> Swept,
             touching(at, &shape) -> FoundTiles: .on(layers) then .any(), or each Tile { cell, layer },
             solid_at(point) -> Option<Layer>,  mark(MARK) -> Vec2 (the one place),  marks(MARK),  bounds() -> Rect
Tilemap:     size(),  rows(),  cells() -> (cell, glyph),  cells_with(glyph) -> the centre of each such cell
Move2d:      [.up(direction)] .on(layers) -> Moved2d
Moved2d:     position: Vec2;  on_floor(),  on_ceiling(),  wall() -> Option<Side>,  blocked(),
             contacts() -> &[Hit] (everything it met, in order),  floor() -> Option<Hit> (what it stands on),
             slide(velocity) -> Vec2 (the velocity with what was stopped taken out)
```

In 3D the same names carry a 3: [the entry](#3d-space-and-a-kinematic-character) is owner question 1.

### 8. Numbers

Two kinds of number, and one question tells them apart: **can this value affect play?** (D1, R4)

- If it can, it is world state. Rules compute it, with `Num`, `Vec2`, `Vec3`, `Angle`, `Rot3` and `Seconds`:
  [the deterministic rules profile](../SIM-2D/DESIGN.md#mechanisms)'s numbers, where each operation is rounded
  once, so one input trace gives one result on the Mac, in Chrome and on the iPhone simulator.
- If it cannot, it is presentation. Frame functions compute it, with `f32`, `Vec2f`, `Vec3f` and `Rot3f`, and
  any maths library.

**In a rule.**

| You write | Is it allowed | Why |
|---|---|---|
| `speed * 0.9`, `x + 1.5`, `x / 2.0` | Yes | A float literal beside a `Num`, a vector or an `Angle` is a `Num` |
| `x > 0.0`, `x == 0.0`, `x.max(0.0)`, `x.clamp(-12.0, 12.0)` | Yes | The same, for comparisons and methods |
| `x * 2`, `count as i32 * 3`, `Num::from(count)` | Yes | An integer of size up to 16,777,216 converts exactly. A larger one is rounded once, as the profile rounds every operation: 16,777,217 becomes 16,777,216 |
| `vec2(1.5, 0.0)`, `vec2(x, 0.0)`, `vec3(wish.x, fall, wish.z)` | Yes | Each argument is a `Num`, a literal or an integer |
| `seconds(0.4)`, `Angle::degrees(35.0)`, `Angle::turns(0.1)` | Yes | Constructors take a literal |
| `Rock { fall_speed: 6.5 }`, `const GRAVITY: Num = -30.0;` | **No** | A literal alone, where a `Num` is expected, is an `f32` to Rust. Write `num(6.5)` and `num(-30.0)` |
| `const GRAVITY: Num = num(-30.0);`, `Vec2::lit(0.8, 1.4)`, `Seconds::lit(0.4)` | Yes | `num` and the `lit` constructors are `const`, for constants and tables |
| `let k = 0.5_f32.sqrt();`, `a_float * another_float`, `x.to_f32().sin()` | **No**, refused by the check | A float computed by the platform can differ between devices |
| `use std::collections::HashMap`, a file, a clock, a thread, an outside crate | **No**, refused by the check | Each reaches outside the tick |

**Names.** `Num` has `sqrt`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `clamp`, `lerp`, `sign` and
`to_i32`. `Angle` has `sin`, `cos`, `direction`, `direction_xz`, `Angle::of(vector)` (the way a vector points),
`Angle::between(a, b)` and `toward(target, at_most)` (a turn limited to a rate), built from exactly rounded
operations with no platform maths library, with conformance vectors on the three targets
([ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)). Vectors have `length`, `length_squared`,
`normalized_or_zero`, `clamp_length`, `clamp_to(rect)`, `distance`, `dot`, `lerp` and, in 3D, `right`.
`normalized_or_zero` of a zero vector is zero, so a bat on top of its target does not make a tick fail.
`Seconds` adds, subtracts and scales, and one divided by another is a `Num`. An `Axis` is used as a `Num` and
a `Stick` as a `Vec2`. An integer indexes a table with `as usize`.

**In a frame function.**

- A pose is read as `Pose2d` or `Pose3d`: the transform as this picture shows it, between ticks and predicted,
  in floats. `Query<&Pose2d>` is the screen's way to ask where something is. A frame function that asks for
  `&Transform2d` is told to ask for `&Pose2d`.
- Every other component is read as it is stored. A `Num` in one becomes a float with `.f32()`, a vector with
  `.f32()` and an `Angle` with `.radians()`. Anything that takes a place to draw at accepts either kind, so
  `draw.burst(art::POP, death.at)` needs no conversion.
- Floats go back into the world only as input: an `Axis`, a `Stick` or a `Heading` a frame function sets is
  put on the wire's grid, and a command's fields are plain data.

**Directions.** In 2D, x is right, y is up, and an angle of zero points along x and grows anticlockwise. In
3D, x is right, y is up and −z is forward. `direction_xz()` of a zero heading is forward and it grows to the
left. A `Stick` pushed up is forward: `stick.y` is along the heading and `stick.x` is to its right.

**The check.** `cargo play` runs it before it builds, and `cargo play --check` runs it alone. It builds the
game's [`rules` module](#1-a-game-one-crate-one-package-one-value) on its own and holds it to the profile, as the profile holds
a rule package today: no raw-float arithmetic or methods, no unordered containers, clocks, threads,
environment, files, sockets, processes or standard input, and no outside crate. Each refusal names the line,
the reason and what to write. It is a guard against mistakes, not a sandbox: `unsafe` code can still reach out,
and what gets past is caught, where it shows, by the digests a match compares.

### 9. Frame functions, looks and the interface

**The frame.**

- **A frame function runs once for each picture the engine encodes**, on the pair of adjacent published ticks
  the engine chose for that picture and the fraction between them
  ([C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)), and the draw that
  follows uses the same. That is **the world as shown**: a pose is where the picture has it, between the two
  ticks; everything else is as of the newer tick; and in a match the local player's own character is at its
  predicted pose, in a query and in the draw alike.
- **A frame function writes only what belongs to this screen**, and reaches the world only by sending input.
- **`Screen<T>` is one value for this screen, whichever frame function asks.** The SDK's own (`Camera2d`,
  `Camera3d`, `Sky`) are set from the scene's description each time the scene is entered. A local setting is
  the panel's value. A game's own type starts as its `Default` and lasts for the whole game, unless a scene
  says `scene.screen(value)`. `scene.on_show(f)` runs a frame function once, when this screen first shows the
  scene.
- **`Shown<L>` is how one screen dresses a look** (R6). The look on the entity is world state: which sprite or
  model it is. Which clip plays, a flash, a decorative turn and hiding it on this screen are not.
- **What a frame function sets or draws lasts for that picture, unless the call names a clip or a time.** A
  tint, an offset, a drawn line and the interface are described again by the next run. A clip asked for with
  `play` goes on until another is asked for, and asking again for the one playing does not restart it. A
  `flash(0.3)`, a `burst` and anything drawn `.lasting(seconds)` are kept by the SDK as this screen's state and
  played out, so one started in a picture that is never shown appears in the next.
- A clip chosen with `Shown` is timed by the frame. A look every player must see at the same moment, such as
  a coin that spins in step, is `Sprite::new(..).looping(clip)` on the entity, timed from the tick. A rule
  that depends on a moment of an animation owns that moment as a `Timer`.
- **A cut.** Every pose is drawn between its last two ticks, so a move that is not a movement must say so.
  `at.teleport(point)` sets the position and marks the tick: the entity is drawn at the new place at once, a
  camera following it cuts with it, and a predicted character is not eased toward it.
- **Cameras.** The scene describes its camera, and a rig keeps it there with no frame function. The rig is
  this screen's state: a frame function changes its numbers through `Screen<Camera2d>` or `Screen<Camera3d>`
  (a boom's length, who it follows, the field of view) and the rig goes on working. The camera, sky and
  background belong to the screen, so two players of one match can look at different places. A rule cannot
  read a camera. What rules may assume of a view is a constant the game gives both: `Camera2d::exactly(w, h)`
  shows that rectangle and no more on every window.
- **Events reach the frame through a bounded cursor.** Each `Events<T>` of a frame function is given the
  events of every tick since it last ran, each once, in tick order, from a ring of at least a second of
  ticks. `missed()` counts any a hitch dropped: events are for show, and nothing that must happen hangs on
  one. An event the local player's own control function sends while it is run ahead is shown then, and once:
  an emission is named by its control line, tick, type and ordinal, so a confirmed tick that sends one more
  than prediction did shows only that one, and one that sends fewer takes nothing back.
- **Everything a frame function does happens once, in the run where it does it**, whether or not the display
  takes that picture: an event is delivered to one run, a click is handed to one run, and a command sent
  takes effect when it is sent.
- Interpolation between ticks is the engine's, on the GPU, for every look. A game writes none. A component is
  published to frame functions only if one names it.

**Looks.**

- **A look is a component, and an entity with a look and a transform is drawn. A thing has the same look
  whether it is placed or moves** (D2, R6). `Model::new(mesh).material(..)` is one vocabulary on an entity
  and on a placement.
- `scene.place(model)` stands a model in the scene with no entity, for what has no gameplay identity. It is
  the cheaper path, the engine's cooked placement. Giving the same model to an entity is how it comes to
  move.
- An entity's `Model` is drawn as an entity row of the 3D scene, with its placement at two ticks, culled on
  the GPU and interpolated: position and scale linearly, the turn by the shorter way, its lit faces and its
  shadow together ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-game-sdk)).
- Draw order in 2D is layer, then lower on the screen in front, then spawn order.
- Extraction is the SDK's, written once, over the rows that changed. The engine chooses the recipe for the
  device, as [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) says, and a game
  never names one.

**The interface.**

- **A frame function describes the interface each frame with `Ui`, and the SDK keeps
  [GAME-UI](../GAME-UI/DESIGN.md)'s tree**, changing only what differs, so an interface that did not change
  costs one draw. A control is the same control from one frame to the next by its place in the description,
  and controls made in a loop take a `.key(..)`.
- **A modal is an exclusive scope.** While it is open this screen's
  [input context](#5-players-actions-and-control) is off, by the rule for any change of context, and
  direction keys move its focus. Local actions with no context are still read, so the key that opened a pause
  menu closes it. `.pausing()` asks for [the pause](#4-scenes-phases-time-and-pause).
- **Outside a modal the interface takes no direction input.** A status display, a row of cards dealt
  mid-fight and a prompt are worked by the pointer, by touch, and by the game's own keys:
  `card.pressed_by(Cards::FIRST)` presses it with a local action. So the stick that walks a hero is never the
  stick that picks a card.
- Focus, touch sizes, safe areas and density are `GAME-UI`'s. The SDK's default font and sizes ship with it,
  so a first game needs no font.

**Names.** Every parameter a frame function may take:

| Parameter | What it gives |
|---|---|
| `Query<D, F>`, `Single<D, F>`, `Res<T>`, `Phase<P>`, `Content<T>`, `Family`, `Players` | The world as shown, read-only. A pose is `Pose2d` or `Pose3d`, in floats, and a frame function that asks for `&Transform2d` is told to ask for `&Pose2d` |
| `Tick`, `Level2d` | The newer tick of the pair, so `timer.left(tick)`, `tick.in_scene()` and `phase.time()` read on a status display as they do in a rule; and the level's fixed collision as of that tick |
| `Events<T>` | `read()`, and `missed()` |
| `Screen<T>` | State of this screen: the camera, the sky, a local setting, the input context, the game's own menu state |
| `Shown<L>` | How this screen dresses a look: `shown.of(entity)` |
| `Ui` | The interface for this frame |
| `Draw2d`, `Draw3d` | Things drawn among the world's: a sprite, a shape, a line, a marker, a label at a point, and `burst` |
| `MyInput<A>`, `Pointer`, `Bindings` | This player's actions as sampled this frame, local actions, the pointer, and their bindings |
| `Outbox`, `Request`, `MyOffer<T>` | The ways a frame function reaches the world: a command, a request, an answer |
| `Pause`, `Saves` | Asking for a pause; reading, writing and deleting saves |
| `Me`, `Session` | Which player this screen is, and the match |
| `Frame` | `dt()` and `seconds()` in `f32`, and `seconds_until(tick_number)` |

```rust
// In a scene function, the lines that describe a screen:
scene.camera(..)  .background(colour)  .backdrop(Backdrop::new(art::SKYLINE).parallax(0.3))
scene.sky(Sky::clear())  .ground(Ground::flat())  .place(model).at(x, y, z).turned(angle).scaled(s)
scene.each_frame((animate_hero, status, pause_menu))  .on_show(f)  .screen(value)  .input_context(NAME)

// Looks, each a component:
Sprite::new(art::enemies::BAT) .layer(1) .tint(colour) .glow(0.5) .looping(clip)
Shape2d::circle(r) | ::rect(w, h) | ::triangle(size) | ::line(a, b)      .color(colour)
Model::new(art::meshes::ROCK) | Model::cube() | Model::sphere()            .material(m) .origin_at_base()
Material::rough(colour) | ::metal(colour) | ::textured(art::materials::GRANITE)     .glowing(colour, strength)
Light::point(colour, strength) | Light::spot(colour, strength, angle)

shown.of(entity)
    .play(art::hero::RUN)  .play_once(art::hero::LAND)        // .finished() says a `play_once` clip has ended
    .flip_x(facing_left)  .tint(Color::RED)  .flash(0.3)  .scale(1.2)
    .turn_y(angle)  .offset(vec3f(0.0, bob, 0.0))  .hide();

Camera2d::fit(24.0, 18.0)                     // at least this rectangle is in view, whatever the window
Camera2d::exactly(40.0, 22.5)                 // this rectangle and no more: bars fill the rest
Camera2d::fit_height(14.0)                    // this height, and as much width as the window gives
    .following_my_character() | .following(entity)
    .damping(seconds(0.15))  .inside_level() | .inside(rect)
Camera3d::looking_at(eye, target).fov(35.0)
Camera3d::following_my_character().offset(0.0, 1.5, 4.0).damping(seconds(0.2))
Camera3d::orbiting_my_character().yaw(Walk::LOOK).distance(6.5).height(1.5)
// Through `Screen<Camera2d>` or `Screen<Camera3d>`:
camera.rig()                                  // the rig's numbers, to change: distance, height, offset, damping, fov, follow(entity)
camera.ease_to(other, seconds(1.2));  camera.cut();  camera.shake(amount);
camera.view() -> Rectf;  camera.screen_point(world) -> Option<Vec2f>;      // None behind the camera

draw.burst(art::effects::DUST, at);   draw.line(a, b).color(c).lasting(seconds(0.12));   draw.marker(at);
```

| `Ui` | |
|---|---|
| Places | `top_left`, `top_center`, `top_right`, `left`, `center`, `right`, `bottom_left`, `bottom_center`, `bottom_right`; `ui.modal(title, ..)`; `ui.at_world(point, ..)`, a place that follows a point in the world, and `.pinned_to_edge()` keeps it on screen when the point is not |
| Layout | `ui.row(..)`, `ui.column(..)`, `ui.grid(columns, ..)`, `ui.space(..)` |
| Showing | `text` (wrapped to its place), `title`, `bar`, `image`, `time(seconds)`, `status(..)`, `prompt(Controls::JUMP)` (the key or button a player has bound), `toast(text, seconds(2))`, `fade(colour, amount)` over the whole screen |
| Doing | `button(label).clicked()`, `card(title, text).chosen()`, `toggle`, `stepper(label, value, range)`, `copyable(text)`; `.enabled(false)`, `.locked()`, `.key(k)` and `.pressed_by(action)` on any of them |

### 10. Content, assets, saves and settings

Three kinds of data, each with a stable identity (D4, R10), and the tuning panel.

**Assets are what is drawn.**

- A game keeps them under `assets/`, and `cargo play` cooks them, watches them and writes **the asset
  catalog**, `src/art.rs`: one typed constant for each asset, in a module for each folder. The kind is in the
  type, so `Sprite::new(art::meshes::ROCK)` does not compile, and an editor completes `art::` to what exists.
  The catalog is committed with the game, and `cargo play --check` fails when it is stale.
- A saved asset is redrawn in the running game without a stall, as [CONTENT-LOOP](../CONTENT-LOOP/DESIGN.md)
  built.
- `Sprite::named("enemies/bat")` and its like take a name found at run time. A name that matches nothing draws
  a visible placeholder and is reported once, with the nearest names that exist.
- How a sheet, a clip and a tile get from a source image into the catalog is
  [an entry of the extension map](#sprites-clips-and-tiles-from-source-images) that the first release builds.

**Content is what the rules read**: waves, levels, enemies' numbers.

- A record's **ID is its file's name**. `Id<T>` is `Copy` plain data: it goes in a component, a resource, a
  scene's fields, a command and a save, and it means the same record in a later build.
- Files are RON. A decimal in a file becomes a `Num` when the content is cooked, rounded once, the same on
  every host.
- **A record holds what world state may hold**, except `Entity` and `Player`; and, because rules cannot change
  it, `String`, `Vec<T>`, a `Tilemap`, a typed asset name, a `Prefab<N>` checked against its declaration, and
  another struct or enum that derives `Content`. **A record holds no raw float**, whoever reads it.
- **Everything is checked before play**, in `test()`, `run()` and `cargo play --check`, in this order: each
  file against its type; each `Id` and `Prefab` it names; each asset's kind; then the game's own validator, the
  function `#[content(validate = ..)]` names and nothing else. A mistake names the file, the field and the
  nearest name that exists.
- The game's content, with its prefabs, is part of the rule-set identity, so a match agrees on it before it
  starts. A test reads the game's real content, and `with_content(name, value)` adds a record with no file.
- **Data packs** are the year-one commitment (D4), and [an entry of the map](#smaller-entries). The IDs, the
  checks and the identity above are what a pack needs, and they are in the first release.

**Saves are what a player keeps.** A save is a type the game owns, never an image of the world, so it can
outlive the build that wrote it.

- **A saved resource** is the short way: `Game::saved::<Records>(slot)` makes `Records` a game resource that
  is read before the first tick and written whenever a tick commits a change to it. Rules never touch a file.
  **The value it starts with is part of how a run starts**, with the seed and the first scene: a recording
  holds it, and a replay starts from it on any device and writes no slot.
- **A run saved and resumed** is the long way. `Game::save::<T>(capture)` registers the one capture for `T`,
  a read-only function that returns the save. Every request names its type, and a slot's file records the
  type and version it holds.
- **Out.** A save is asked for by a rule or a frame function. The capture runs on the next committed tick,
  paused or not, and the file is written away from the tick. `saves.status(slot)` says how it went.
  `save_if(capture, condition)` gives the one place that says when a save is allowed, and both routes and
  `saves.can_save::<T>()` follow it.
- **In: a save enters the world as admitted input, in pieces.** `request.scene(..).restoring::<T>(slot)` reads
  the file on the machine that asks and sends its bytes ahead of the request, each piece one action of an
  input frame, budgeted after that tick's commands against the frame `SESSION` encodes whole (about 850
  bytes a tick). The world gathers them as world state, under a limit of 64 KiB unless `#[save(max_kib = n)]`
  says otherwise, and at the tick the last is admitted every machine checks, decodes and enters the scene
  with it. The scene's enter functions take `Restored<T>`, and
  [resume the phase](#4-scenes-phases-time-and-pause) the save names. A recording holds the pieces.
- **A restore lands whole or not at all, and its failure never leaves the SDK's step.** When the gathered save
  fails its check, is of a newer version, or an enter function that takes it returns an error, the world is
  left as it was and the tick is stepped again without the restore, inside the one world step the driver
  asked for. A match is not ended by a save that will not load.
- **What a save may hold** is what a resource may hold, except `Entity` and `Player`, which name nothing in
  another run. A `Timer` is written as the time left. An `Id` whose record a later build removed makes the
  file `Loaded::Failed`, with the field named.
- Storage is each target's own: a folder on the Mac, the browser's storage in Chrome, the app's folder on the
  simulator. Chrome's is asynchronous, which is why reading is `Loaded` and not a `Result`.
- **In a match, a saved resource and a save are the host's**: only the host's screen may ask for a restore,
  and only the host's machine writes a slot. [Progress each player keeps](#saved-progress-for-every-player)
  is question 11.
- **Versions.** A save's version and its chain of older types are declared from the first release, and a test
  restores an old file. The promise that a released save loads in a later build is year one's (D4).

**Settings.** `#[derive(Settings)]` on a struct makes each field a control on the panel, with its default
beside its range. They are `thrum-controls`' typed controls ([CREATOR](../CREATOR/DESIGN.md)), so the Mac
panel, the Chrome page and parameter files work as they do today, and the profiler is in every game's panel.

| | A shared setting: `Game::settings::<T>()` | A local setting: `Game::local_settings::<T>()` |
|---|---|---|
| Owner | The world. `T` is a game resource | This screen, kept on this machine |
| Read by | Rules, control functions and frame functions, as `Res<T>` | Frame functions, as `Screen<T>`. A rule cannot take it |
| Fields | What a resource may hold. A raw float is a compile error | Those, and `f32` |
| A panel edit | Enters as a command and changes the world at a tick. A replay holds it | Changes this screen at once |
| In a match | The host's edits are the ones admitted | Each player has their own |

**Names.**

```rust
// src/art.rs: written by `cargo play`. Do not edit.
pub mod enemies { pub const BAT: SpriteAsset = SpriteAsset::new("enemies/bat"); }
pub mod meshes  { pub const ROCK: MeshAsset = MeshAsset::new("meshes/rock"); }
pub mod hero    { pub const RUN: ClipAsset = ClipAsset::new("hero/run"); }

#[derive(Content)]
#[content(validate = WaveDef::validate)]          // fn(&WaveDef) -> thrum::Result, with thrum::Error::invalid(field, why)
pub struct WaveDef { pub enemy: Prefab, pub count: u32, pub speed: Num, pub next: Option<Id<WaveDef>>, pub taunts: Vec<String> }
Game::new("Swarm").content::<WaveDef>("waves")    // assets/content/waves/*.ron, one record a file
impl<T: ContentType> Content<T> {
    pub fn get(&self, id: Id<T>) -> &T;
    pub fn find(&self, name: &str) -> Option<Id<T>>;
    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)>;   // in name order
}
pub const FIRST_WAVE: Id<WaveDef> = Id::new("first");          // checked when the game starts

#[derive(Save)]
#[save(version = 2, from = RecordsV1)]            // `impl From<RecordsV1> for Records` carries an old file forward
pub struct Records { pub best: List<(Id<LevelDef>, Seconds), 64>, pub coins: u32 }
Slot::new("records")   Slot::AUTO
impl Saving { pub fn request<T: Save>(&mut self, slot: Slot); }        // in a rule: "save now", such as at a shrine
impl Saves {                                                           // in a frame function
    pub fn save<T: Save>(&mut self, slot: Slot);
    pub fn can_save<T: Save>(&self) -> bool;
    pub fn peek<T: Save>(&self, slot: Slot) -> Loaded<&T>;   // Loading, Missing, Ready(..), Newer { version }, Failed(reason)
    pub fn status(&self, slot: Slot) -> SaveStatus;          // Idle, Writing, Written { at }, Failed(reason)
    pub fn delete(&mut self, slot: Slot);
    pub fn slots(&self) -> impl Iterator<Item = Slot>;
}
fn begin_or_resume(restored: Restored<ValeSave>, ..)         // an enter function: restored.get() -> Option<&ValeSave>

#[derive(Settings)]
pub struct Feel { #[slider(4.0..=14.0, default = 9.0, label = "Run speed")] pub run_speed: Num }   // also #[toggle], #[readout]
```

### 11. Multiplayer, an opt-in over input-only networking

- **Multiplayer is one line and changes no rule** (K3, D3): `Game::multiplayer(Multiplayer::players(1..=4))`
  with the `multiplayer` feature. A game played alone is the same game with one player.
- **Only input travels** (D1). Every machine steps the same rules from the same admitted input: held actions,
  commands, answers, the roster's changes, a pause, a scene request, the pieces of a save. Nothing of the
  world is replicated while a match runs, so 700 enemies cost the link what none do. A joiner is sent the
  world once, as a baseline, and then steps it. Digests are compared, and a difference is repaired.
- **The SDK implements `SESSION`'s world for the game**, and a game writes none of the steps of
  [the session guide](../../SESSION.md): the rule-set identity, from the game's rules, content and build and
  every starting value its description registers; the world image and digest, from the registered state; the
  action wire, from the action maps and commands; the roster, as [seats](#5-players-actions-and-control); and
  the driven world. Who does what is `SESSION`'s and unchanged: a native process hosts, browsers and the
  simulator join, the world is shown at the confirmed tick, and a match ends with its host.
- **Nothing in a match knows 2D from 3D** (O6). What travels is input, what is compared is a digest of
  registered state, and what runs ahead is a control function. `Transform3d`, `Vec3`, `Rot3` and `Angle` are
  the profile's numbers, with conformance vectors on the three targets, so a 3D world digests, rewinds and
  joins as a 2D one does. A 3D control function runs ahead on the same rows of
  [the table](#5-players-actions-and-control): its components, its action map with its `Heading`, the tick,
  and the level's fixed collision, which is `Level2d` on the ground plane in the first release and `Level3d`
  with question 1. The local player's character is shown at its predicted `Pose3d`, and a camera rig that
  follows it or reads its `Heading` has no delay. The first release proves it twice: a rehearsed 3D match
  with no window in WP5, and the 3D starter hosted on the Mac and joined from Chrome in WP14.
- **What a player's own machine runs ahead has three depths.** Each is a declaration, each leaves every rule
  as it was, and the first release builds the first:

  | Depth | What answers at once | How a game says it | First release |
  |---|---|---|---|
  | 1 | The player's own character, against fixed collision | A control function that reads only what can be run ahead | Yes |
  | 2 | What that character launches, and the other players it runs into | `.predicts(..)` and `.against(..)` on the control line ([entry](#predicted-projectiles-and-contact)) | Question 4 |
  | 3 | A hit judged against what the shooter's screen showed | `space.as_seen_by(player)` in the rule that judges it ([entry](#hits-judged-as-the-shooter-saw-them)) | Question 5 |

  Until depth 2 is built a player's shots and the players they touch are shown at the confirmed tick, and
  until depth 3 a hit is decided at the shared tick. The guides say both plainly.
- **How depth 1 runs.** The SDK gives the control function to `SESSION`'s predicted controller as its step.
  The controller's state is everything the next step depends on: the character's entity and the components
  the function names; the clock it can read, which is the game's tick and the ticks at which the running
  scene and its phase were entered; this player's input state; whether the match is paused; and what the
  function reads of the confirmed world, which is the resources it names, the phase and the level's identity.
  Content and the seed are the rule set's constants.
- **The whole state is compared with the host's, bit for bit**, so a change to anything the step reads
  corrects it even where the character has not moved. **A step run ahead equals the host's step of that tick
  when the host admitted the frame this machine stepped with and nothing the step reads had changed on the
  host that this machine had not yet been sent.** Where either fails, the tick is corrected when its
  confirmation arrives: the controller returns to the confirmed state and replays its unconfirmed input, and
  the SDK eases the shown pose to the corrected one over a few pictures, except across a `teleport`.
- **Under a bad link.** A player whose input is late keeps their last axes for a moment and then idles, and
  their `PlayerStatus` says so. Nobody else waits for them. A player whose link drops is `Away` for
  `Multiplayer::grace(seconds(30))`, keeps their seat, score and character, and has left after it.
- **The lobby is an ordinary scene on the match's clock** (R8), of type `Lobby`, and the first scene of a game
  that has one. When a player chooses *Host*, the SDK starts `SESSION`'s host and its ticks at once with the
  lobby running, and a peer who joins is sent it as it stands. So a ready mark, a host's panel choice and
  *Start* are input at a tick and world state. *Play alone* is the same lobby with no session.
  `scenes.go_to(Lobby)` returns everyone to it.
- **`Session`**, in a frame function, is for a game with its own screens, and for a screen that asks whether
  it is alone.
- **Not offered in the first release**: depths 2 and 3 above, and
  [host migration](#smaller-entries).

**Names.**

```rust
Game::new("Swarm")
    .multiplayer(Multiplayer::players(1..=4).pause(Pausing::Nobody).grace(seconds(30)))
    .lobby(Lobby::standard()           // alone, host or join by a link, a paste or a launch argument; who is here; why a match closed
        .ready_up()                    // a ready mark each player sets for themselves
        .host_panel(pick_arena)        // fn(&mut Ui, &mut Setup): the game's own setup, sent as the host's command
        .starts(Arena) | .starts_with(arena_from))     // fn(&Setup) -> Arena

Session:  state() -> Alone | Hosting { ticket, players } | Joining { progress } | Playing | Closed { reason }
          host(), join(ticket), leave(), can_host(), dropped_ticket()
```

### 12. Mistakes that explain themselves

- **Every message names the object, the cause and the next step, and has a stable code** (`T0412`) that the
  guide *When something goes wrong* explains with a failing example and its fix. `cargo play --check
  --message-format=json` gives each as a record with its code, source span and suggestion, for an agent.
- **A mistake is caught at the earliest moment that can know it**: by the compiler where a type can say it,
  on the SDK's own traits so the words are the SDK's; by the rules check; at startup, in `test()` and `run()`
  alike, as the error they return, before the first tick; and otherwise in the tick, which is refused with
  the world unchanged.
- A message says only what its check knows. `thrum::Error` is one enum with a variant for each kind of remedy,
  in [the adopted error shape](../../../.agents/skills/thrum-engineering/references/patterns.md).

| When | Mistake | The message says |
|---|---|---|
| Compile | A rule takes `Ui`, `Screen<T>`, `Shown<L>`, `Draw2d`, `Outbox`, `Me`, `Pause` or another frame parameter | That a rule must give the same result on every machine, and to move this to a frame function |
| Compile | A frame function takes `ResMut<T>`, `Entities`, `Emit<T>`, `PhaseMut<P>`, `PlayersMut` or a `&mut` query | That frame functions cannot change the world, and to send a command or write a rule |
| Compile | A spawn function, a condition or a capture takes a parameter that writes | That it may only read, and which parameter to change |
| Compile | A frame function asks for `&Transform2d` or `&Transform3d` | To ask for `&Pose2d` or `&Pose3d` |
| Compile | World state holds a `Vec`, a `String`, a reference, a raw float or `usize`; a content record holds a raw float | What to use: `Num`, `Text<N>`, `List<T, N>`, content, or a local setting |
| Compile | A spawn of a `Prefab<N>` without its `N`; `Sprite::new` given a mesh; a field of an action map with no binding; a first scene with no default | Rust's own type error, with the SDK's note on what is missing |
| Check | Raw-float arithmetic, an unordered container, a file, a clock or an outside crate in the `rules` module; a path from it to the rest of the game | The line, why it can differ between machines, and what to write |
| Startup | One parameter names a component twice and writes it; two parameters of one rule, or a control function's query and its character, conflict over a prefab | The function, the parameters, the component and the prefab, and what to drop |
| Startup | A rule reads a resource nothing provides | The rule, the resource, and `scene.resource(..)` or `Game::resource(..)` |
| Startup | A function that makes or changes the world is defined outside the `rules` module, in a game that has one | The function and the module |
| Startup | The action maps do not fit an input frame; a command type is too large | The counts or the size, and the limit |
| Startup | A declared prefab name has no definition; a definition names an asset that does not exist; a content file fails its type, names a record that does not exist or fails the game's validator | The prefab or the file, the field, and the nearest name |
| Startup | With multiplayer on: the game has no `rules` module; a control function reads what cannot be run ahead | How to move the rules; the parameter and `.unpredicted()` |
| Startup, in `run()` | No asset store, a stale asset catalog, or a missing prerequisite | What is missing and the command that provides it |
| Running | A spawn of a component the world does not know; a spawn past the world's size; more fixed shapes than the level's recipe holds | The rule, what ran out or is unknown, and the line to add |
| Running | `Single` finds none or two; `entities.part` on a prefab with no such part; `level.mark` finds none or two | The rule and what was found |
| Running | An entity, or a player's character, comes to be reached by two parameters of one function over a component one of them writes | The rule, the parameters, the component and the entity, and for a character its player |
| Running | `keep_world` where it is refused | The child, the parent and the cause, with the residual and the bound; the child is where it was |
| Running | `go_to` a scene the game was not given; a `request.scene` the running scene does not accept; two characters for one player on one `players()` line | The scene or the line, and what to add |
| Running | A rule returns an error, or panics | The rule, the tick and the error, [as each target allows](#3-rules-and-the-tick) |
| Running, on screen | An entity with a look in a scene with no camera, or a 3D look under a 2D camera; a named asset that matches nothing | Once: the scene and the look, or the name and the nearest names |
| Running, on screen | A session is refused or closed, or the match has no seat for this player | `SESSION`'s reason, or that the match is full, in words a player can read |

### 13. Tests without a window, recordings and pictures

- **`game().test()` is the game with no window, device or clock, on the same rules and the same engine path
  as `run()`** (R9). A game played alone has its one player from the start, with their character.
- **Starting is fallible, and fixtures go in before it.** `test()` is `test_setup().start()`, which in order
  installs the fixtures; checks the content; runs every startup check; gives each saved resource its fixture
  or its `Default`, since a test reads and writes no real storage; and enters the first scene. The first
  failure is the error it returns. No tick has been stepped when it returns.
- **A test can tell "not there" from "not yet".** Lookups return a `thrum::Result` whose error says which: a
  join still waiting for its tick, a player with no character, an entity that is gone.
- **What a test changes, it changes between ticks**, and it has landed when the call returns. `game.run(rule)`
  runs any rule function once the same way, so a game's own helpers that take `Entities` or a `Query` set a
  test up.
- **`TestScreen`** runs the scene's frame functions for one player against the world as it stands, with no
  renderer, so a test can open a pause menu, pick an upgrade card and see `offer.status()` change.
- **`game().test_match()`** is a host and its peers in one process, each with a world of its own, stepped in
  simulated time over `SESSION`'s rehearsal. It proves a late join, a slow peer, a choice made twice and a
  dropped link, in 2D and in 3D.
- **A recording** is the input a run admitted, with the game's seed, the scene it started in and the value
  each saved resource started with. A recording made on one device replays on another (D1). After a change to
  the rules it replays the same input against the new rules, which is what makes it a test that a level is
  still beatable.
- **A picture.** `cargo play --shot <file>` runs a game offscreen for a number of ticks, under a recording if
  one is given, and writes the frame the engine submitted, through [T2 capture](../T2-CAPTURE/DESIGN.md). It
  needs no screen and no permission.

**Names.**

```rust
game().test() | .test_in(scene) -> thrum::Result<TestGame>          .test_match() -> thrum::Result<TestMatch>
game().test_setup() .in_scene(s) .with_content(name, record) .with_saved(value) .with_save(slot, save) .seed(n) .start()
```

| `TestGame` | |
|---|---|
| Time | `step()`, `step_ticks(n)`, `step_for(seconds)`, `step_until(limit, \|game\| ..)`, which fails with the last tick it reached and never waits on a clock; `rewind(ticks)`; `digest()`; `pause()` and `unpause()` |
| Players | `join()`, `leave(p)`, `away(p)`, `back(p)`, `add_bot()`; every input call is for the first player unless `.player(p)` says otherwise |
| Input | `hold(Controls::JUMP)`, `release(..)`, `press(..)` (down for one tick), `steer(Controls::RUN, 1.0)`, `send(command)`, `choose(value)` for an open offer; `stall_input(ticks)`, which loses the player's frames as a bad link does |
| Finding | `character(player)`, `single::<F>()`, `find::<F>()`, `nearest::<F>(point)`, `children(e)`, `part(e, name)` |
| Reading | `get::<T>(e)`, `resource::<T>()`, `phase::<P>()`, `scene::<S>()`, `in_scene::<S>()`, `count::<F>()`, `alive(e)`, `world2d(e)`, `world3d(e)`, `level2d()`, `events::<E>()` since the last call, `offer::<T>(player)`, `missing_assets()` |
| Changing | `spawn(..)`, `despawn(e)`, `set(e, component)`, `set_resource(value)`, `go_to(scene)`, `go_to_phase(p)`, `run(rule)` |
| Saves | `save::<T>()`, which runs the capture and returns the save; `restore(scene, save)`; `saved::<T>()`; `slot::<T>(slot)`, what a slot holds |
| The screen | `screen()`, or `screen_of(player)`: a `TestScreen` with `click("Resume")`, `choose(key)`, `press(Menu::PAUSE)`, `has_text(..)`, `find(key)`, `camera2d()`, `camera3d()`, `bursts()` |
| Recordings | `play_recording(file)`, which leaves the game where the run ended |

| Call | Returns | Notes |
|---|---|---|
| `join()` | `thrum::Result<Player>` | The seat is known at once. The player enters the game at the next `step()`. The error is that the game is full, with nothing changed |
| `character(player)` | `thrum::Result<Entity>` | Three errors: the join is still waiting for its tick; the player is here with no character; no such player |
| `single::<F>()` | `thrum::Result<Entity>` | An error for none and for two, with the count |
| `find::<F>()`, `children(e)` | `Vec<Entity>` | In spawn order, and in the order attached |
| `get`, `resource`, `scene`, `phase` | `thrum::Result` of a copy | An error when the entity is gone or has no `T`, nothing provides the resource, or another scene is running |
| `spawn`, `despawn`, `set`, `set_resource`, `go_to`, `go_to_phase`, `run` | `thrum::Result` | They act between ticks. `go_to` enters the scene at once, as `start` does |
| `step()` and the other time calls | `thrum::Result<()>` | The error of a refused tick, after which the game can be stepped again. A step while paused is a paused tick |
| `TestMatch::join(link)` | `thrum::Result<Peer>` | The error is that the match is full. A link the host lets through returns its peer, seated or not |
| `peer(p).my_character()` | `thrum::Result<Entity>` | `character`'s errors and two more: the peer is still loading the world; the peer was given no seat and has left |

`TestMatch` has `host()` and `peer(p)`, each with a `TestGame`'s calls and `shown::<Pose2d>(e)` or
`shown::<Pose3d>(e)`, the pose that machine's screen shows; `join(Link::delayed(millis(120)))`,
`disconnect(p)`, `reconnect(p)`, `step_for(..)`, `digests_agree()`; and for the lobby `host().set_setup(value)`
and `host().start(scene)`. `Tick::test(number)` makes a tick for a unit test of a pure function.

### 14. Running a game, documentation and templates

One command for each target, the same for every game, from the game's folder. The template ships them as
Cargo aliases onto the engine's tool. The game's `main` is the same file on every target.

| Command | What it does |
|---|---|
| `cargo play` | Cooks and watches `assets/`, writes the asset catalog, checks the rules, builds, opens the Mac window with its panel |
| `cargo play --web` | The same for Chrome: builds for wasm32, serves the page and the store on loopback, prints the URL |
| `cargo play --ios` | Builds and starts the game on the iPhone simulator, through [GAME-UI](../GAME-UI/DESIGN.md)'s one shell |
| `cargo play --check` | The rules check and every startup check, with no window. `--message-format=json` for an agent |
| `cargo play --record <file>`, `--replay <file>` | Records a run while a person plays, and shows one |
| `cargo play --shot <file>` | Writes a picture |
| `cargo play --scene <name>` | Starts in another scene, with its default value |

- **Reference.** Every public item of `thrum` has a doc comment with an example, and every example is a doc
  test. `./dev check` fails a public item without one.
- **Guides**, in `docs/sdk/`: *Start here* (the four sentences and the fifteen words); *Your first 2D game*,
  whose first page ends with a ship you steer; *Your first 3D game*; *Make it multiplayer*; *Numbers*; *How do
  I…* (short answers by task); *When something goes wrong* (by code). A guide teaches only what is built, and
  each code block in a guide is compiled by the docs check.
- **One file for agents.** `docs/sdk/REFERENCE.md` is generated from the reference and the guides, and the
  docs check fails when it is stale.
- **Templates**: `2d`, `3d` and `multiplayer`. `./dev new <folder> --template <name>` writes a game that
  builds, runs and passes its check as written, with `src/lib.rs` holding `game()`, `src/rules.rs`, a test,
  and an `AGENTS.md` that points an agent at the one file. A game made this way lives outside the engine
  checkout and reaches `thrum` by path.
- **Judged by a change the guides did not show**, as well as by a clean start: a second enemy variant, a
  reconnect during a choice.

## The extension map

What the first release does not build, and how each would be added (O5). **Nothing here is built, and no guide
teaches it, until the owner says so**, except the one entry marked for the first release. Each entry says when
it comes, where it plugs in, the names it adds, what must hold, and a few lines of the game code it allows.

**Eight extension points.** Every entry adds names at one or more of these, and changes no name or invariant
of [the core](#the-core). A game that does not use an entry compiles and pays nothing for it.

| | Extension point | What may be added there | The rule it must keep |
|---|---|---|---|
| 1 | A rule parameter | A new thing rules ask of the world or change in it | Deterministic, in the profile's numbers, digested. It joins one row of [the control table](#5-players-actions-and-control) |
| 2 | A frame parameter, or a word of one such as `Ui` | A new thing one screen does | It cannot change the world except by sending input |
| 3 | A look, with its `Shown<L>` | A new component the engine presents | Which one it is, is world state. How a screen dresses it is not |
| 4 | An action kind or a binding source | A new shape of input, or a new device | It fits the input frame and reads as a plain field |
| 5 | A line of a game, a scene or a control line | A new thing a description declares | It is part of the rule-set identity when rules can see its effect |
| 6 | A kind of admitted input | A new thing that enters the world at a tick | A recording holds it, and every machine admits the same |
| 7 | A kind of asset or content | A new file the cooker reads into the catalog or a record | Typed, checked before play, named by a stable identity |
| 8 | A command of the tool or a call of the test harness | A new way to run or prove a game | The game's code does not change |

### 3D space and a kinematic character

*Owner question 1, recommended yes. Extension points 1 and 5. Needs an engine module: a deterministic 3D
index and fixed collision from meshes and a heightfield.*

Everything [section 7](#7-space-and-the-level) says holds with a 3 on it: colliders detect and never push, a
space meets the level, one `move_shape` takes the character's own collider, and a hit's normal points out of
what was struck. A 3D level is the same recipe: which terrain and which solid models are loaded, by their
identity in content, and the shapes a rule added. Its collision is cooked beside the mesh, from its triangles,
and from the heightfield. It is content, so it is in the rule-set identity and a test builds it with no
renderer. `Level3d` joins the control table's second row, so a 3D character runs ahead against the ground it
stands on.

```rust
Collider3d::sphere(r) | ::capsule(r, height) | ::cuboid(x, y, z)      .offset(x, y, z)  .on(LAYER)
// Space3d:    touching, within, nearest, motion_of, at_tick_start, ray, sweep_sphere, move_shape
// Level3d:    move_shape, ray, sweep_sphere, touching, ground_height(x, z), mark, bounds
// LevelMut3d: load(&Terrain), add(model, pose, LAYER), add_box(center, size, LAYER), clear()
// Move3d:     .max_slope(angle) .step_up(height) .snap_down(height) .on(layers) -> Moved3d; a hit has .surface

fn walk(at: &mut Transform3d, body: &mut Body, shape: &Collider3d, ground: Level3d, tick: Tick) {
    let moved = ground.move_shape(at.position, shape, body.velocity * tick.dt())
        .max_slope(Angle::degrees(40.0)).step_up(0.3).snap_down(0.35).on((TERRAIN, SOLID));
    at.position = moved.position;
    body.velocity = moved.slide(body.velocity);
}
let in_sight = space.ray(eye, chest).on(SOLID).first_hit().is_none();   // a hill hides as a boulder does
```

### Gamepads

*Owner question 2, recommended yes. Extension points 4 and 2. Needs pad input in the shells and focus by pad
in `GAME-UI`.*

A pad fills the same `Axis`, `Stick` and `Button` a key does, so no rule changes. In a modal the pad moves
focus. `ui.prompt` shows the glyph of the device the player last used.

```rust
// Pad::LEFT_STICK RIGHT_STICK (and _X _Y of each) DPAD DPAD_X DPAD_Y SOUTH EAST WEST NORTH
//      LEFT_SHOULDER RIGHT_SHOULDER LEFT_TRIGGER RIGHT_TRIGGER START SELECT
#[bind(Keys::A_D, Pad::LEFT_STICK_X, Pad::DPAD_X, Touch::stick_x())]
pub run: Axis,

fn feel_the_landing(landed: Events<Landed>, mut rumble: Rumble) {
    for hit in landed.read() { rumble.pulse(hit.speed.f32() / 20.0, 0.15); }
}
if let Some(input) = bindings.listen() { bindings.rebind(Controls::JUMP, input); }   // a remap screen
bindings.sensitivity(Walk::LOOK, 1.4);   bindings.invert(Walk::LOOK, true);   my_input.device()   // Keys, Pad, Touch
```

### Audio

*Owner question 3, recommended yes in this small form. Extension points 2, 3 and 7. Needs an engine module,
and the owner's ear for its items.*

Sound is presentation. A rule cannot take `Audio`, and nothing heard changes the world. There are three forms,
and each has an owner, so nothing leaks:

- **A sound that happens** is played by a frame function on an event, as a burst is drawn. A predicted event
  is heard once. `.limit(n)` keeps at most `n` voices of one sound, so forty hits a second are four voices.
- **A sound a thing makes** is a look, `Sound`, on the entity. It starts and stops with the entity as a
  `Light` does, and `Shown<Sound>` is how one screen turns it up or down.
- **Music is this screen's state**, `Screen<Music>`: a frame function says what should be playing, a change
  cross-fades, and saying the same track again does nothing. `scene.music(track)` sets it on entry. Volumes
  are a local setting, `Mix`, with `master`, `music` and `effects`.

```rust
.prefab(WARDEN, (Warden, model, Sound::looping(art::sounds::HUM).radius(14.0)))   // heard while it exists

fn sounds(hits: Events<Struck>, mut audio: Audio) {
    for hit in hits.read() { audio.play(art::sounds::HIT).at(hit.at).pitch_between(0.9, 1.1).limit(4); }
}
fn score(mut music: Screen<Music>, hunt: Res<Hunt>) {
    music.track = if hunt.chasing { art::music::HUNT } else { art::music::DUSK };
}
```

### Predicted projectiles and contact

*Owner question 4, recommended first in the release after. Extension points 5 and 1. Uses `SESSION` as built.*

This is depth 2 of [what a machine runs ahead](#11-multiplayer-an-opt-in-over-input-only-networking). Depth 1
predicts one entity, the character. Depth 2 lets a control line declare more, and nothing else changes:

- **`.predicts(PREFAB.moved_by(f).at_most(n))`**: what this player's control function launches through its
  `Launch` parameter joins that player's predicted set, up to `n` at a time. A launched thing carries its
  launcher's `Player`, as a character does, and is named by that player, the tick and its ordinal, as a
  predicted event is, so the bolt this screen predicted and the bolt the host confirms are one thing, shown
  once. `f` is a control function for each launched thing: its own components, the tick and the level.
- **`.against(LAYER)`**: space queries the control function asks on that layer move from the table's "not run
  ahead" row to "run ahead with the value at the last confirmed tick". The other craft stand where the host
  last confirmed them, and a difference arrives as a correction, as a changed setting does.
- **What a launched thing does to the world stays an ordinary rule at the confirmed tick**: damage, a score,
  a despawn. So no rule is written twice, and a prediction the host does not confirm is eased away and plays
  no second sound.
- It fits `SESSION`'s predicted controller as built: its state grows from one character to a declared,
  bounded set, compared whole, bit for bit.

```rust
scene.players().spawn(CRAFT).control(drive)
    .predicts(BOLT.moved_by(fly_bolt).at_most(8))     // my bolts leave my muzzle at once
    .against(HULLS);                                  // and my craft meets the others where I see them

fn drive(at: &mut Transform3d, craft: &mut Craft, shape: &Collider3d, input: &Fly,
         mut bolts: Launch<Bolt>, space: Space3d, tick: Tick) {
    let moved = space.move_shape(at.position, shape, craft.velocity * tick.dt()).on((SOLID, HULLS));
    at.position = moved.position;
    if input.fire.held && craft.gun.take(tick, seconds(0.2)) {
        bolts.spawn(BOLT.with(Bolt::fired_along(at.forward())).at_point(at.position));
    }
}
fn fly_bolt(at: &mut Transform3d, bolt: &Bolt, tick: Tick) { at.position += bolt.velocity * tick.dt(); }
```

### Hits judged as the shooter saw them

*Owner question 5, recommended after question 4. Extension points 6, 5 and 1. Uses `SESSION` as built.*

This is depth 3. Other players are shown at the confirmed tick, a moment behind, so a shooter who aims at what
they see misses what the world holds. The fix is the field's lag compensation, as input-only networking allows
it:

- **Each player's input carries the tick their screen was showing** when it was sampled. It is admitted input,
  so a recording holds it.
- **A space remembers a layer for a stated time**, as world state, so every machine and every replay judges
  the same hit, and a late joiner is sent the memory with the world.
- **`space.as_seen_by(player)`** is the same space with that layer where that player's screen had it. A view
  older than the memory is held at its edge, so a bad link cannot reach far into the past. It is asked each
  tick with that tick's input, so for a bolt in flight the view moves on by itself.

```rust
scene.space3d(Space3d::cells(4.0).remembers(HULLS, seconds(0.25)));

fn bolts_hit(bolts: Query<(Entity, &Player), With<Bolt>>, space: Space3d, mut hulls: Query<&mut Hull>) {
    for (bolt, pilot) in &bolts {
        let seen = space.as_seen_by(*pilot);                  // the hulls where that pilot's screen had them
        let Some(hit) = seen.motion_of(bolt).sweep().on((HULLS, SOLID)).first_hit() else { continue };
        if let Some(mut hull) = hulls.get_mut(hit.entity) { hull.hurt(36); }
    }
}
```

### Skeletal animation

*Owner question 8, recommended first among the engine modules after the release. Extension points 3 and 7.*

A pose is presentation. `Model` gains a rig, and `Shown<Model>` plays clips as `Shown<Sprite>` does. No rule
reads a bone: a rule that depends on a moment of an animation owns that moment as a `Timer`, as today. A part
may ride a bone, which changes where it is drawn and not where rules find it.

```rust
.prefab(PILGRIM, (Pilgrim, Model::new(art::meshes::PILGRIM).rig(art::rigs::PILGRIM)))

fn animate(pilgrim: Single<(Entity, &Body)>, mut shown: Shown<Model>, mut audio: Audio) {
    let (entity, body) = pilgrim.into_inner();
    let speed = body.velocity.f32().length();
    let clip = if speed > 0.1 { art::pilgrim::WALK } else { art::pilgrim::IDLE };
    let playing = shown.of(entity).play(clip).blend(0.15).speed(speed / 1.4);     // feet that do not slide
    for step in playing.marks(art::pilgrim::FOOTSTEP) { audio.play(art::sounds::STEP).at(step.at); }
}
```

### Text fields, lists and more controls

*Owner question 9, recommended after the first release. Extension point 2. Needs `GAME-UI`.*

Each is a word of `Ui`. What is being typed is this screen's state, and it reaches the world as a command.

```rust
fn name_entry(mut ui: Ui, mut draft: Screen<Draft>, mut outbox: Outbox) {
    ui.center(|ui| {
        ui.text_field("Name", &mut draft.name);
        ui.slider("Music", &mut draft.music, 0.0..=1.0);
        ui.picker("Difficulty", &mut draft.difficulty, Difficulty::ALL);
        ui.scroll(|ui| for line in &draft.log { ui.text(line).style(Style::small().color(Color::GREY)); });
        if ui.button("Done").clicked() { let _ = outbox.send(SetName(Text::from(&draft.name))); }
    });
}
```

### Levels from an editor's file

*Owner question 10, recommended after the first release, with data packs. Extension point 7.*

A `Tilemap`, and in 3D a list of placements, is cooked from an editor's file where a record names one. The
scene maps the editor's names to layers, prefabs and marks as it maps glyphs today. `load`, the recipe, a
map's identity and a validator's read of its cells are unchanged.

```rust
// assets/content/levels/rooftops.ron:  LevelDef(name: "Rooftops", par: 42.5, tiles: File("rooftops.ldtk#Rooftops"))
scene.tiles(Tileset::new(art::ROOFS).solid("Brick", TERRAIN).spawns("Coin", COIN).marks("Start", START));
// In 3D, the nodes of a glTF scene: a node named for a prefab is spawned, and the rest are placed.
level.load(&vale.placements);     // Placements, with .named("lamp", LAMP) and .solid("wall", SOLID) on the scene
```

### Saved progress for every player

*Owner question 11, recommended yes, in the saves and online work packages. Extension points 5 and 6.*

In the first release a match's saves are the host's. This gives every seat a saved value that lives on that
player's own machine:

- **It comes in as that player's input.** Their machine reads its slot before they join and sends it in
  pieces in their own input frames, as [a restore's pieces](#10-content-assets-saves-and-settings) travel in
  the host's. The world gathers them, and `PlayerJoined` is sent once the value has landed, so a rule that
  meets a player finds their progress.
- **It goes out with nothing sent.** Every machine holds the same world, so each writes its own seat's value
  to its own slot when a committed tick changes it. A bot's is the default.
- **A guest's file is the guest's word.** `#[save(validate = f)]` runs on every machine as the value lands,
  and one that fails becomes the default. A recording holds the pieces.

```rust
Game::new("Lampwake").saved_per_player::<Progress>(Slot::new("progress"))

fn bank_light(mut progress: ResMut<PerPlayer<Progress>>, tally: Res<Tally>, players: Players) {
    for player in players.iter() { progress[player].light += tally.light_of(player); }
}
```

### Sprites, clips and tiles from source images

***In the first release, built in WP3**, because sprites and sprite animation are (O4). Extension point 7.*

A sheet is an image and a file beside it that names its frames, its clips and its tiles. `cargo play` cooks it
and writes a constant for each name into the catalog. A misspelled frame fails `cargo play --check` with the
file and the nearest name, and a saved sheet is redrawn in the running game. No run-time name changes.

```ron
// assets/courier/courier.sheet.ron, beside courier.png
Sheet(grid: (32, 48), frames: ["idle", "stride", "pass", "reach", "jump"],
      clips: { "run": Clip(frames: ["stride", "pass", "reach", "pass"], fps: 11) })
// assets/roofs/roofs.sheet.ron: a tile that names its edges is drawn by its neighbours
Sheet(grid: (16, 16), tiles: { "brick": Tile(fill: 0, top: 1), "wire": Tile(fill: 2) })
```

```rust
shown.of(courier).play(art::courier::RUN);                    // a ClipAsset the catalog now has
Tileset::new(art::ROOFS)
    .solid('#', STREET).draws('#', art::roofs::BRICK)         // two surfaces on one layer, each with its own tile
    .one_way('=', STREET).draws('=', art::roofs::WIRE)
```

### Camera pitch and rigs

*With question 1: a third-person game needs both. Extension points 4 and 2.*

`Aim` is an action kind beside `Heading`: a yaw and a pitch the player's own machine keeps, with limits on the
pitch, travelling as a `Heading` does. A rig takes it.
[A rig's numbers](#9-frame-functions-looks-and-the-interface) are already a frame function's to change, and
with question 1 a boom avoids the level.

```rust
#[bind(Mouse::look(), Pad::RIGHT_STICK)]
#[aim(pitch = -35.0..=65.0)]
pub look: Aim,                               // .yaw .pitch: Angle; .direction(): Vec3; .direction_xz()

scene.camera(Camera3d::orbiting_my_character().look(Walk::LOOK).distance(6.5).avoiding((TERRAIN, SOLID)));
fn sprint_pulls_back(mut camera: Screen<Camera3d>, gait: Single<&Gait, With<Player>>) {
    camera.rig().distance = if gait.sprinting() { 8.0 } else { 6.5 };      // the boom still avoids walls
}
```

### Navigation and path queries

*After the first release, first among what follows question 1. Extension points 5 and 1.*

A scene says who must find a way, and the SDK bakes the answer from the level's recipe and keeps it by the
recipe's identity, as it keeps built collision. So it is the same on every machine, follows a gate a rule
closed, and is asked through the level, read-only, by a rule, a condition and a test.

```rust
scene.navigation(Nav::agent(0.5, 1.8).max_slope(Angle::degrees(30.0)).on((TERRAIN, SOLID)));

fn wardens_move(mut wardens: Query<(&mut Transform3d, &Mind), With<Warden>>, ground: Level3d, tick: Tick) {
    for (mut at, mind) in &mut wardens {
        let Some(corner) = ground.next_corner(at.position, mind.goal()) else { continue };   // None: no way
        at.position += (corner - at.position).normalized_or_zero() * 3.0 * tick.dt();
    }
}
assert!(game.level3d().path(trailhead, summit).is_some());     // a test: the summit can still be reached
```

### 3D level building and texturing

*After the first release, with question 1 for what is solid. Extension points 5 and 7.*

More words on the lines a 3D scene already has. What is solid is in the level's recipe and belongs to the
rules. How the ground is dressed belongs to the screen, and scatter has no collision and no identity.

```rust
scene.ground(Ground::heights(art::terrain::VALE)
    .layer(art::materials::GRASS)
    .layer(art::materials::SCREE).steeper_than(Angle::degrees(35.0))
    .scatter(art::meshes::TUFT, 0.4));
scene.place(Model::new(art::meshes::CAIRN)).on_ground(40.0, -12.0).solid(SOLID);     // its triangles collide
scene.place(Model::cuboid(6.0, 3.0, 0.5).material(stone)).at(10.0, 1.5, 4.0).solid(SOLID);
```

### Smaller entries

Each plugs in at the extension point named, with the code it allows and what must hold.

| Entry | When | Point | The game code it allows | What must hold |
|---|---|---|---|---|
| **A host with no window** | Owner question 7, recommended yes, in the online work package | 8 | `cargo play --host --port 7777`, which prints the ticket to join by | No line of the game changes. It is `thrum` with its default features off. What a host's screen may do (a scene request, a shared setting, *Start*) falls to the lowest seat |
| **Text in the world** | After the first release | 2 | `draw.text(hit.at, hit.damage).color(Color::GOLD).rising(1.5).lasting(seconds(0.6))` | Drawn as sprites in one batch, so sixty damage numbers are not sixty interface nodes. Played out by the SDK, as a burst is |
| **Two views of one component in one rule** | After the first release | 1 | `fn revive(mut wardens: Views<(Query<&Vitals>, Query<&mut Vitals>)>)`, then `wardens.first()` and `wardens.second()` | It lends one at a time, so the access check has nothing to refuse. Until then a rule makes two passes over one query and carries a local `List` between them |
| **A second state machine in a scene** | After the first release | 5 | `scene.phases(Chapter::Arrival); scene.phases(Mode::Walking);` with `Phase<Mode>` and `scene.phase(Mode::Fallen).lasts(seconds(2))` | Each machine has its own blocks and clock. "A tick makes one transition" becomes one for each machine, or a change of scene instead of any, which is the same rule for a scene with one |
| **Lobby choices for every player** | After the first release | 5 | `Lobby::standard().player_panel(pick_calling).starts_with(night_from)`, with `fn night_from(&Setup, &PerPlayer<Calling>) -> Night` | A player's choice is world state, changed by their own command. A panel may take any frame parameter, such as `Content<T>` to list what there is to pick |
| **A timeline inside a phase** | After the first release | 5 | `scene.phase(Stage::Night).at(seconds(90), ring_of_shades).at(seconds(300), call_the_bell)` | Each runs once, in the block's place in the written order, and a phase resumed past one does not run it again. A timeline from a content file stays a rule with a cursor |
| **Space queries from a frame function** | After the first release | 2 | `fn prompt(me: Me, space: Space3d, mut ui: Ui)` | It answers from the newer published tick and costs each screen an index. Until then a rule writes what a screen needs, such as a `Focus` component |
| **Rigid bodies** | Out by the owner's scope (O4). Needs a deterministic solver | 1 | `scene.rules((apply_thrust, physics::step, crates_break))`, `Body2d::dynamic().mass(4.0)` and `Events<Contact2d>` | The step is a rule the game places in its written order. Colliders stay detect-only on an entity with no body. In a match bodies are confirmed state, as a crowd is |
| **Data packs** | The year-one commitment (D4) | 5, 7 | `Game::new("Swarm").packs("packs")` | A pack adds records and variants of existing prefabs without a recompile. Its names are checked as it is loaded, and the packs are part of the rule-set identity. New mechanics stay Rust |
| **Water, clouds and weather** | When `WATERS` and `CLOUDS` are built | 5, 1 | `scene.water(Water::sea())`, and in a rule `level.water_height(x, z, tick)` | What one screen shows of it is `Screen<T>`. Where play depends on it, a rule asks the level |
| **Host migration** | After the first release. Needs `SESSION` | 5 | `Multiplayer::players(1..=4).host_migration(true)` | Every machine already holds the whole world and steps it from input, so another takes over as host at a tick. No rule changes |

## Worked examples

These are real games against the proposed API, and the evidence that [the core](#the-core) reads as a game.
[What a builder may change](#what-a-builder-may-change) says how far a builder may depart from them. Every call
they make has its name in the core. Example G alone is written against entries of
[the extension map](#the-extension-map) that the first release does not build, and says where.

### A. A first 2D game

Rockfall: steer a ship under falling rocks. The first page of the guide ends with a program of 25 lines:
`main`, the `Pilot` map, a `play` scene of two lines (its camera and its players' line), `ship` and
`steer_ship`, exactly as they stand below. The ship already answers the keys and a drag on a phone, and a
second player gets a ship of their own beside it. The second page adds the rocks, the score and the end. This
is the whole game, still one file:

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    game().run()
}

/// One value that `main` runs and a test steps.
pub fn game() -> Game {
    Game::new("Rockfall")
        .prefab(ROCK, (Shape2d::circle(0.6).color(Color::GREY), Collider2d::circle(0.6).on(ROCKS)))
        .scene(play)
        .scene(game_over)
}

const ROCKS: Layer = Layer::new("rocks");

prefabs! {
    const ROCK: Prefab<Rock> = "rock"; // whoever spawns a rock says how fast it falls
}

#[derive(Scene, Default)]
struct Play;

/// A scene's fields are what it is entered with, so the score travels to the result.
#[derive(Scene)]
struct GameOver {
    dodged: u32,
}

#[derive(Actions)]
struct Pilot {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::drag_x())]
    steer: Axis,
}

#[derive(Component)]
struct Rock {
    fall_speed: Num,
}

#[derive(Resource, Default)]
struct Score {
    dodged: u32,
}

fn play(scene: &mut Scene<Play>) {
    scene.background(Color::NIGHT);
    scene.camera(Camera2d::fit(24.0, 18.0));
    scene.resource(Score::default()); // a scene's resource starts afresh each time the scene is entered
    scene.players().spawn(ship).control(steer_ship);
    scene.rules((drop_rocks.every(seconds(0.4)), fall, crash));
    scene.each_frame(show_score);
}

fn ship(joined: Joined) -> impl Bundle {
    let x = joined.player.index() as i32 * 2;
    (Shape2d::triangle(1.0), Transform2d::at(x, -7), Collider2d::circle(0.5))
}

fn steer_ship(at: &mut Transform2d, pilot: &Pilot, tick: Tick) {
    at.position.x = (at.position.x + pilot.steer * 9.0 * tick.dt()).clamp(-11.5, 11.5);
}

fn drop_rocks(mut entities: Entities, mut random: Random) {
    let rock = Rock { fall_speed: random.between(4.0, 9.0) };
    entities.spawn(ROCK.with(rock).at(random.between(-11.5, 11.5), 10.0));
}

fn fall(
    mut rocks: Query<(Entity, &Rock, &mut Transform2d)>,
    mut entities: Entities,
    mut score: ResMut<Score>,
    tick: Tick,
) {
    for (rock, falling, mut at) in &mut rocks {
        at.position.y -= falling.fall_speed * tick.dt();
        if at.position.y < -10.0 && entities.despawn(rock) {
            score.dodged += 1;
        }
    }
}

/// Written after `steer_ship` and `fall`, so it tests the ships and the rocks where they are now. Every
/// player's ship is asked, and the run ends for everyone when a rock reaches any of them.
fn crash(ships: Query<Entity, With<Player>>, space: Space2d, score: Res<Score>, mut scenes: Scenes) {
    for ship in &ships {
        if space.touching(ship).on(ROCKS).any() {
            scenes.go_to(GameOver { dodged: score.dodged }); // the first request stands; a second is refused
        }
    }
}

fn show_score(mut ui: Ui, score: Res<Score>) {
    ui.top_left(|ui| ui.text(format!("Dodged {}", score.dodged)));
}

fn game_over(scene: &mut Scene<GameOver>) {
    scene.background(Color::NIGHT);
    scene.accepts::<Play>(); // this scene's screen may ask for a new run
    scene.each_frame(show_result);
}

fn show_result(mut ui: Ui, over: Res<GameOver>, mut request: Request) {
    ui.center(|ui| {
        ui.title("Game over");
        ui.text(format!("You dodged {} rocks.", over.dodged));
        if ui.button("Play again").clicked() {
            request.scene(Play); // Enter, a click and a tap all press the button
        }
    });
}

#[test]
fn a_rock_that_reaches_the_ship_ends_the_run() -> thrum::Result {
    let mut game = game().test()?; // every startup check runs here, and a failed one is this error
    let ship = game.character(Player::FIRST)?;
    let at = game.get::<Transform2d>(ship)?.position;
    game.spawn(ROCK.with(Rock { fall_speed: num(6.0) }).at(at.x, at.y + 3.0))?;

    game.step_until(seconds(2), |game| Ok(game.in_scene::<GameOver>()))?;

    assert_eq!(game.scene::<GameOver>()?.dodged, 0);
    Ok(())
}
```

What the reader did not write: a window, a device, a clock, a schedule, component registration, interpolation
between ticks, sprite batching, a web page, or a line for Chrome. `cargo play` opens it on the Mac and
`cargo play --web` serves it to Chrome. It needs no art: shapes draw with no asset. `game_over` has no camera:
a scene without one shows its background and its interface and draws nothing else.

### B. A first 3D scene

A ball you roll round a cooked, textured rock, on lit ground under the physical sky, with Sun sliders. The
first page of the 3D guide ends with it, so the first thing a reader does in 3D is steer. The rock turns, and
the example teaches the one question a maker asks of every value: can it affect play?

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Thrum starter").local_settings::<Sun>().scene(clearing).run()
}

#[derive(Actions)]
struct Drive {
    #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]
    go: Stick,
}

/// Where the Sun stands is this screen's own, so these are local settings and may be plain `f32`.
#[derive(Settings)]
struct Sun {
    #[slider(-8.0..=60.0, default = 25.0, step = 0.5, unit = "°", label = "Sun elevation")]
    elevation: f32,
    #[slider(0.0..=360.0, default = 150.0, step = 1.0, unit = "°", label = "Sun azimuth")]
    azimuth: f32,
}

#[derive(Component)]
struct Rock;

fn clearing(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.ground(Ground::flat());
    scene.camera(Camera3d::following_my_character().offset(0.0, 2.0, 6.0).damping(seconds(0.2)));
    scene.on_enter(place_rock);
    scene.players().spawn(ball).control(roll);
    scene.each_frame((move_sun, turn_rock));
}

fn place_rock(mut entities: Entities) {
    let granite = Material::textured(art::materials::GRANITE);
    entities.spawn((Rock, Model::new(art::meshes::ROCK).material(granite), Transform3d::at(0.0, 0.0, 0.0)));
}

/// One ball for each player. A built-in sphere needs no asset.
fn ball(joined: Joined) -> impl Bundle {
    let x = joined.player.index() as i32 * 2;
    (Model::sphere().material(Material::metal(Color::GOLD)), Transform3d::at(x, 0.5, 4))
}

/// Runs once a tick for each player's ball, with that player's input. A stick pushed up is forward, which is −z.
fn roll(at: &mut Transform3d, drive: &Drive, tick: Tick) {
    at.position += vec3(drive.go.x, 0.0, -drive.go.y) * 4.0 * tick.dt();
}

fn move_sun(mut sky: Screen<Sky>, sun: Screen<Sun>) {
    sky.sun = SunAngle::degrees(sun.elevation, sun.azimuth);
}

/// The turn is for show: nothing in the game depends on it. So it belongs to this screen, is worked out for
/// each picture, and uses ordinary floats.
fn turn_rock(rock: Single<Entity, With<Rock>>, mut shown: Shown<Model>, frame: Frame) {
    shown.of(*rock).turn_y(Angle::turns(0.1 * frame.seconds()));
}
```

When the turn matters to play, because it aims a light a guard looks along, it is world state, a rule writes
it, and it uses the numbers that agree on every device:

```rust
#[derive(Component)]
struct Turns {
    per_second: Angle,
}

fn turn(mut things: Query<(&Turns, &mut Transform3d)>, tick: Tick) {
    for (turns, mut at) in &mut things {
        at.turn_y(turns.per_second * tick.dt());
    }
}
```

The sky, the Sun's light and shadows, exposure, tone mapping, reconstruction and the governor are the engine's
defaults. The two sliders appear in the Mac panel and on the Chrome page, a parameter file sets them, and the
profiler is a toggle beside them. Either way the rock turns smoothly between ticks and its shadow turns with
it. The ball is moved by a control function that writes its place and nothing else: the first release has no
3D collision, so it rolls through the rock, and the guide says so
([scope question 1](#scope-questions-for-the-owner)). `art` is
[the typed asset catalog](#10-content-assets-saves-and-settings), which `cargo play` writes from `assets/`.
With `.multiplayer(..)` and the standard lobby on the game, as [example F](#f-the-same-game-online-with-bots)
adds them, a friend's ball rolls beside yours and each of you sees your own answer at once (O6): `roll` reads
only what a match runs ahead.

### C. A slice of the V1 showcase

The exit scene: the cooked valley, 10,000 entities of which 1,500 drift, six stops and the hour of the day.

```rust
fn valley(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.place(Model::new(art::meshes::VALLEY).material(Material::textured(art::materials::GROUND)));
    scene.camera(STOPS[0].camera());
    scene.resource(Tour::default());
    scene.on_enter(populate); // 10,000 spawns from `layout::PROPS`, 1,500 of them `.with(Drifts { .. })`
    scene.rules((drift, tour)); // `tour` moves on a stop when any player presses `next_stop`
    scene.each_frame((sun_follows_hour, fly_to_stop)); // the hour is a local setting, as example B's Sun is
}

fn drift(mut movers: Query<(&Drifts, &mut Transform3d)>, tick: Tick) {
    movers.par_each(|(drifts, mut at)| {
        let turn = drifts.starts_at + drifts.per_second * tick.seconds();
        at.position = drifts.around + turn.direction_xz() * drifts.radius;
    });
}

fn fly_to_stop(mut camera: Screen<Camera3d>, tour: Res<Tour>) {
    camera.ease_to(STOPS[tour.stop as usize].camera(), seconds(1.2));
}
```

`STOPS` and `layout` are the game's own data, and `layout::PROPS` is a table of prefabs. The valley is placed,
not spawned: it has no gameplay identity, so the scene describes it once and the engine draws it by its
cheapest path. A prop is the same `Model` and material on an entity. The 8,500 that no rule writes cost nothing
after they are placed, and `par_each` spreads the movers over the engine's worker threads. GPU culling, cooked
mesh rows, the recipe for this Mac and the Chrome build are not in the game's code at all.

### D. A precision platformer

Skylark: run, jump with coyote time and a buffered jump, die on spikes, restart at the flag you last touched.
Levels are content: adding one is a file. `src/rules.rs` is the game, and it is [checked](#8-numbers) to give
the same run on every device.

```rust
// src/rules.rs
use thrum::prelude::*;

#[derive(Actions)]
pub struct Controls {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::stick_x())]
    pub run: Axis,
    #[bind(Key::Space, Touch::button("Jump"))]
    pub jump: Button,
}

/// "Play the next level" is a value: `scenes.go_to(Level { def: next })`.
#[derive(Scene)]
pub struct Level {
    pub def: Id<LevelDef>,
}

#[derive(Scene)]
pub struct Cleared {
    pub level: Id<LevelDef>,
    pub time: Seconds,
}

/// One file under `assets/content/levels/` for each level.
#[derive(Content)]
pub struct LevelDef {
    pub name: String,
    pub tiles: Tilemap,
    pub par: Seconds,
    pub next: Option<Id<LevelDef>>, // checked when the game starts: it names a level that exists
}

#[derive(Phase)]
pub enum Stage {
    Playing,
    Dying,
}

#[derive(Component, Default)]
pub struct Hero {
    pub velocity: Vec2,
    pub on_floor: bool,
    pub coyote: Timer,        // still allowed to jump, just after leaving the ground
    pub buffered_jump: Timer, // a jump pressed just before landing
    pub checkpoint: Vec2,
}

/// The whole match plays with one feel: the host's panel sets it and the rules read it.
#[derive(Settings)]
pub struct Feel {
    #[slider(4.0..=14.0, default = 9.0, label = "Run speed")]
    pub run_speed: Num,
    #[slider(8.0..=24.0, default = 16.0, label = "Jump speed")]
    pub jump_speed: Num,
    #[slider(20.0..=80.0, default = 48.0, label = "Gravity")]
    pub gravity: Num,
}

#[derive(Event)]
pub struct Jumped {
    pub at: Vec2,
}

pub const TERRAIN: Layer = Layer::new("terrain");
pub const SPIKES: Layer = Layer::new("spikes");
pub const COINS: Layer = Layer::new("coins");
pub const START: Mark = Mark::new("start");
pub const FLAG: Mark = Mark::new("flag");

prefabs! {
    pub const HERO: Prefab = "hero";
    pub const COIN: Prefab = "coin";
}

pub fn prefabs(game: Game) -> Game {
    let body = Collider2d::rect(0.8, 1.4); // the hero's shape, written once: its move is given this collider
    game.prefab(HERO, (Hero::default(), Sprite::new(art::hero::SHEET).layer(2), body))
        .prefab(COIN, (Sprite::new(art::pickups::COIN), Collider2d::circle(0.3).on(COINS)))
}

pub fn level(scene: &mut Scene<Level>) {
    scene.tiles(
        Tileset::new(art::TILES)
            .solid('#', TERRAIN)
            .solid('^', SPIKES)
            .spawns('o', COIN)
            .marks('@', START)
            .marks('F', FLAG),
    );
    scene.on_enter(build_level);
    scene.players().spawn(hero_at_start).control(run_and_jump).during(Stage::Playing);
    scene.phases(Stage::Playing);
    scene.phase(Stage::Playing).rules((collect_coins, touch_spikes, touch_flags, reach_the_end));
    scene.phase(Stage::Dying).lasts(seconds(0.6)).then(Stage::Playing).on_exit(back_to_checkpoint);
}

fn build_level(level: Res<Level>, levels: Content<LevelDef>, mut tiles: LevelMut2d) {
    tiles.load(&levels.get(level.def).tiles); // the solid tiles, the coins and the marks, from the file
}

fn hero_at_start(tiles: Level2d) -> impl Spawn {
    let start = tiles.mark(START);
    HERO.with(Hero { checkpoint: start, ..Hero::default() }).at_point(start)
}

/// The hero's whole movement. Everything it reads is something a match can run ahead with: the hero, its
/// player's controls, the tick, the shared feel and the level's fixed collision.
fn run_and_jump(
    hero: &mut Hero,
    at: &mut Transform2d,
    body: &Collider2d,
    controls: &Controls,
    feel: Res<Feel>,
    tiles: Level2d,
    mut jumped: Emit<Jumped>,
    tick: Tick,
) {
    hero.velocity.x = controls.run * feel.run_speed;
    hero.velocity.y -= feel.gravity * tick.dt();

    if controls.jump.pressed {
        hero.buffered_jump.start(tick, seconds(0.1));
    }
    if hero.buffered_jump.running(tick) && hero.coyote.running(tick) {
        hero.velocity.y = feel.jump_speed;
        hero.buffered_jump.clear();
        hero.coyote.clear();
        jumped.send(Jumped { at: at.position });
    }
    if controls.jump.released && hero.velocity.y > 0.0 {
        hero.velocity.y *= 0.5; // let go early for a short hop
    }

    let moved = tiles.move_shape(at.position, body, hero.velocity * tick.dt()).on(TERRAIN);
    at.position = moved.position;
    hero.velocity = moved.slide(hero.velocity); // what a wall, a floor or a ceiling stopped is gone
    hero.on_floor = moved.on_floor();
    if hero.on_floor {
        hero.coyote.start(tick, seconds(0.1)); // six ticks
    }
}

fn touch_spikes(hero: Single<(&Transform2d, &Collider2d), With<Hero>>, tiles: Level2d, mut stage: PhaseMut<Stage>) {
    let (at, body) = *hero;
    if tiles.touching(at.position, body).on(SPIKES).any() {
        // A tick makes one transition, and the first request stands. This rule is written before
        // `reach_the_end`, so a hero who touches spikes in the tick they reach the end dies.
        stage.go_to(Stage::Dying);
    }
}

fn reach_the_end(
    hero: Single<&Transform2d, With<Hero>>,
    level: Res<Level>,
    tiles: Level2d,
    mut scenes: Scenes,
    tick: Tick,
) {
    if hero.position.x > tiles.bounds().max.x - 1.0 {
        // False, with nothing changed, in a tick where `touch_spikes` asked first: no clear is recorded.
        scenes.go_to(Cleared { level: level.def, time: tick.in_scene() });
    }
}

fn back_to_checkpoint(hero: Single<(&mut Hero, &mut Transform2d)>) {
    let (mut hero, mut at) = hero.into_inner();
    hero.velocity = Vec2::ZERO;
    at.teleport(hero.checkpoint); // a cut: no picture shows the hero part-way between the two places
}
```

`Run` is a scene resource with the coin count, registered in `level` in the full game. `Cleared`'s enter
function writes the time into `Records` when it beats the best, so a best time is recorded only by a tick whose
one transition was the clear. A level is a file:

```ron
// assets/content/levels/rooftops.ron
LevelDef(
    name: "Rooftops",
    par: 42.5,
    next: Some("foundry"),
    tiles: [
        "                          F      ",
        "        o o       #####  ###     ",
        " @    #####   ^^             o   ",
        "########################   ######",
    ],
)
```

What this screen shows is another file. Nothing in it can change a run:

```rust
// src/screens.rs
use crate::rules::*;
use thrum::prelude::*;

/// Keys only this screen cares about. They never travel to the rules or to another player.
#[derive(LocalActions)]
pub struct Menu {
    #[bind(Key::Escape, Touch::button("II"))]
    pub pause: Button,
}

pub fn level(scene: &mut Scene<Level>) {
    scene.camera(Camera2d::fit_height(14.0).following_my_character().damping(seconds(0.15)).inside_level());
    scene.backdrop(Backdrop::new(art::SKYLINE).parallax(0.3));
    scene.accepts::<Level>(); // the pause menu may restart the level
    scene.each_frame((animate_hero, dust_on_jump, show_clock, pause_menu));
}

/// A frame function reads the tick, so the clock on screen is the run's own time, and it stands in a pause.
fn show_clock(mut ui: Ui, tick: Tick) {
    ui.top_right(|ui| ui.time(tick.in_scene()));
}

/// Which clip plays is this screen's choice, so it is made here and not kept in the world.
fn animate_hero(hero: Single<(Entity, &Hero)>, mut shown: Shown<Sprite>) {
    let (entity, hero) = *hero;
    let clip = if !hero.on_floor {
        art::hero::JUMP
    } else if hero.velocity.x != 0.0 {
        art::hero::RUN
    } else {
        art::hero::IDLE
    };
    shown.of(entity).play(clip).flip_x(hero.velocity.x < 0.0);
}

fn dust_on_jump(jumped: Events<Jumped>, mut draw: Draw2d) {
    for jump in jumped.read() {
        draw.burst(art::effects::DUST, jump.at);
    }
}

fn pause_menu(mut ui: Ui, mut pause: Pause, menu: MyInput<Menu>, level: Res<Level>, mut request: Request) {
    if menu.pause.pressed {
        pause.toggle(); // while paused no rule runs and the game's clock stands, so every timer waits
    }
    if !pause.is_on() {
        return;
    }
    ui.modal("Paused", |ui| {
        if ui.button("Resume").clicked() {
            pause.off();
        }
        if ui.button("Restart level").clicked() {
            request.scene(Level { def: level.def }); // admitted under the pause, and the change of scene ends it
        }
    });
}
```

`src/lib.rs` holds `game()`: `Game::new("Skylark")` with `.settings::<rules::Feel>()`,
`.content::<rules::LevelDef>("levels")`, `.saved::<rules::Records>(Slot::new("records"))` for best times,
`.add(rules::prefabs)` and three scenes, each a pair such as `.scene((rules::level, screens::level))`. `main`
runs it. `collect_coins` and `touch_flags` are a few lines each, as example E's `collect_gems` is.

The feel of a platformer is proved without a window, and a whole level stays beatable:

```rust
#[test]
fn a_jump_just_after_the_ledge_still_jumps() -> thrum::Result {
    let (mut game, hero) = hero_just_off_the_ledge()?;
    game.step_ticks(2)?; // in the air, inside the six ticks of coyote time
    game.press(Controls::JUMP);
    game.step()?;

    assert!(game.get::<Hero>(hero)?.velocity.y > 0.0);
    Ok(())
}

/// Without this one, a hero who could jump from anywhere in the air would pass the test above.
#[test]
fn a_jump_after_the_coyote_time_does_not_jump() -> thrum::Result {
    let (mut game, hero) = hero_just_off_the_ledge()?;
    game.step_ticks(8)?; // in the air, past the six ticks
    game.press(Controls::JUMP);
    game.step()?;

    let hero = game.get::<Hero>(hero)?;
    assert!(!hero.on_floor && hero.velocity.y < 0.0); // still falling
    Ok(())
}

/// The hero has run off the first roof and this is its first tick in the air.
fn hero_just_off_the_ledge() -> thrum::Result<(TestGame, Entity)> {
    let mut game = game().test_in(Level { def: Id::new("rooftops") })?;
    let hero = game.character(Player::FIRST)?;
    game.set(hero, Transform2d::at(5.5, 2.7))?; // the last tile of the first roof
    game.steer(Controls::RUN, 1.0);
    game.step_until(seconds(1), |game| Ok(!game.get::<Hero>(hero)?.on_floor))?;
    Ok((game, hero))
}

/// Recorded once with `cargo play --record tests/runs/rooftops.run`. It fails when a change to the jump or
/// to the level means the recorded run no longer reaches the end.
#[test]
fn rooftops_is_still_beatable() -> thrum::Result {
    let mut game = game().test()?;
    game.play_recording("tests/runs/rooftops.run")?;
    assert!(game.in_scene::<Cleared>());
    Ok(())
}
```

### E. The swarm game's core loop

A hero for each player holds out against a growing crowd, with attacks that fire by themselves and a choice of
upgrade between waves.

```rust
// src/rules.rs: the run. It gives the same result on every player's machine.
use thrum::prelude::*;

#[derive(Actions)]
pub struct Steer {
    #[bind(Keys::WASD, Keys::ARROWS, Touch::stick())]
    pub walk: Stick,
}

pub const HEROES: Layer = Layer::new("heroes");
pub const ENEMIES: Layer = Layer::new("enemies");
pub const GEMS: Layer = Layer::new("gems");

// Names are declared together, so a name with no definition is found when the game starts.
prefabs! {
    pub const HERO: Prefab = "hero";
    pub const BAT: Prefab = "bat";
    pub const ELITE_BAT: Prefab = "elite bat";
    pub const GEM: Prefab = "gem";
    pub const SHOT: Prefab<(Shot, Velocity)> = "shot"; // the hero that fires it gives these
}

#[derive(Scene, Default)]
pub struct Arena;

#[derive(Scene)]
pub struct Ended {
    pub won: bool,
    pub wave: u32,
}

#[derive(Phase)]
pub enum Stage {
    Gathering,
    Wave,
    Intermission,
}

#[derive(Component)]
pub struct Hero {
    pub fire: Cooldown,
    pub fire_every: Seconds,
    pub damage: i32,
}
#[derive(Component)]
pub struct Enemy;
#[derive(Component)]
pub struct Shot {
    pub damage: i32,
    pub fades: Timer,
}
#[derive(Component)]
pub struct Gem;
#[derive(Component)]
pub struct Velocity(pub Vec2);
#[derive(Component)]
pub struct Health(pub i32);

impl Health {
    pub fn hurt(&mut self, by: i32) {
        self.0 = (self.0 - by).max(0);
    }
    pub fn is_spent(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Resource, Default)]
pub struct Run {
    pub wave: u32,
    pub gems: u32,
}

#[derive(Event)]
pub struct EnemyDied {
    pub at: Vec2,
}

#[derive(Data)]
pub enum Upgrade {
    FasterFire,
    MoreDamage,
    Heal,
}

pub fn prefabs(game: Game) -> Game {
    let hero = Hero { fire: Cooldown::READY, fire_every: seconds(0.4), damage: 1 };
    let knight = Sprite::new(art::heroes::KNIGHT).layer(2);
    let bat = Sprite::new(art::enemies::BAT).layer(1);
    game.prefab(HERO, (hero, Health(100), knight, Collider2d::circle(0.5).on(HEROES)))
        .prefab(BAT, (Enemy, Health(3), Velocity(Vec2::ZERO), bat, Collider2d::circle(0.35).on(ENEMIES)))
        .prefab(ELITE_BAT, BAT.variant().with(Health(8)).tweak(|sprite: &mut Sprite| sprite.tint = Color::GOLD))
        .prefab(GEM, (Gem, Sprite::new(art::pickups::GEM), Collider2d::circle(0.3).on(GEMS)))
        .prefab(SHOT, (Sprite::new(art::shots::BOLT).layer(3).glow(0.5), Collider2d::circle(0.15)))
}

/// One tick of the arena, top to bottom.
pub fn arena(scene: &mut Scene<Arena>) {
    scene.space2d(Space2d::cells(4.0)); // part of the rules, so the same for every player
    scene.resource(Run::default());
    scene.players().spawn(hero_in_line).control(move_hero);
    scene.phases(Stage::Gathering);
    scene.phase(Stage::Gathering).lasts(seconds(3)).then(Stage::Wave);
    scene.phase(Stage::Wave)
        .lasts(seconds(60))
        .then(Stage::Intermission)
        .on_enter(next_wave)
        .rules((
            spawn_enemies.every(seconds(1)),
            steer_enemies,
            fire_at_nearest,
            move_bodies,
            enemies_hurt_heroes,
            shots_hit,
            enemies_die,
            collect_gems,
            heroes_fall,
        ))
        .on_exit(clear_enemies);
    scene.phase(Stage::Intermission)
        .lasts(seconds(15))
        .then(Stage::Wave)
        .on_enter(offer_upgrades)
        .rules(take_upgrades);
}

fn hero_in_line(joined: Joined) -> impl Spawn {
    HERO.at(joined.player.index() as i32 * 3 - 4, 0)
}

/// A hero's whole movement: the hero, its player's input and the tick. In a match the local player's hero
/// runs ahead with it and answers at once.
fn move_hero(at: &mut Transform2d, steer: &Steer, tick: Tick) {
    at.position = (at.position + steer.walk * 6.0 * tick.dt()).clamp_to(ARENA);
}

/// Runs as the wave phase is entered. A request made while a transition lands is the next tick's first
/// request, so after the last wave the arena ends one tick later, and no rule of that tick can ask first.
fn next_wave(mut run: ResMut<Run>, mut scenes: Scenes) {
    if run.wave == LAST_WAVE {
        scenes.go_to(Ended { won: true, wave: run.wave });
    } else {
        run.wave += 1;
    }
}

/// Once a second while the wave lasts: a bat for each hero, times the wave's number.
fn spawn_enemies(run: Res<Run>, heroes: Query<&Transform2d, With<Hero>>, mut entities: Entities, mut random: Random) {
    for _ in 0..run.wave * heroes.count() {
        let Some(near) = random.pick(&heroes) else { return };
        let bat = if random.chance(0.1) { ELITE_BAT } else { BAT };
        entities.spawn(bat.at_point(near.position + random.direction() * 30.0));
    }
}

fn steer_enemies(mut enemies: Query<(Entity, &Transform2d, &mut Velocity), With<Enemy>>, space: Space2d) {
    enemies.par_each(|(me, at, mut velocity)| {
        // The heroes are a layer of their own, so this looks at four colliders and not at the crowd.
        let Some(hero) = space.nearest(at.position).on(HEROES).first() else { return };
        let chase = (hero.position - at.position).normalized_or_zero();
        let mut apart = Vec2::ZERO;
        for other in space.within(at.position, 1.0).on(ENEMIES).except(me) {
            let away = at.position - other.position;
            apart += away / away.length_squared().max(0.02);
        }
        velocity.0 = (chase * 2.5 + apart * 0.6).clamp_length(3.0);
    });
}

fn fire_at_nearest(mut heroes: Query<(&mut Hero, &Transform2d)>, space: Space2d, mut entities: Entities, tick: Tick) {
    for (mut hero, at) in &mut heroes {
        let Some(target) = space.nearest(at.position).on(ENEMIES).within(12.0).first() else { continue };
        let every = hero.fire_every;
        if !hero.fire.take(tick, every) {
            continue; // each hero keeps its own time, so a late joiner does not fire in step with the rest
        }
        let aim = (target.position - at.position).normalized_or_zero();
        let shot = Shot { damage: hero.damage, fades: Timer::after(tick, seconds(1.5)) };
        entities.spawn(SHOT.with((shot, Velocity(aim * 18.0))).at_point(at.position));
    }
}

// `move_bodies` adds each body's velocity to its place under one `par_each`, and `enemies_hurt_heroes`
// takes one health from a hero for each enemy that `space.touching(hero).on(ENEMIES)` finds.

fn shots_hit(
    shots: Query<(Entity, &Shot)>,
    mut enemies: Query<&mut Health, With<Enemy>>,
    space: Space2d,
    mut entities: Entities,
    tick: Tick,
) {
    for (shot, hit) in &shots {
        // The path the shot took this tick against the path each bat took, so a fast shot cannot pass
        // through a bat between two ticks. A shot fired this tick starts where it was spawned.
        if let Some(struck) = space.motion_of(shot).sweep().on(ENEMIES).first_hit() {
            if let Some(mut health) = enemies.get_mut(struck.entity) {
                health.hurt(hit.damage);
            }
            entities.despawn(shot);
        } else if hit.fades.finished(tick) {
            entities.despawn(shot);
        }
    }
}

fn enemies_die(
    enemies: Query<(Entity, &Health, &Transform2d), With<Enemy>>,
    mut entities: Entities,
    mut died: Emit<EnemyDied>,
) {
    for (enemy, health, at) in &enemies {
        if health.is_spent() && entities.despawn(enemy) {
            entities.spawn(GEM.at_point(at.position));
            died.send(EnemyDied { at: at.position });
        }
    }
}

/// The gems `enemies_die` just dropped are already here: its spawns landed when it finished.
fn collect_gems(
    heroes: Query<&Transform2d, With<Hero>>,
    space: Space2d,
    mut entities: Entities,
    mut run: ResMut<Run>,
) {
    for hero in &heroes {
        for gem in space.within(hero.position, 1.5).on(GEMS) {
            if entities.despawn(gem.entity) {
                run.gems += 1; // `despawn` is false when another hero took this gem in the same tick
            }
        }
    }
}

fn heroes_fall(heroes: Query<&Health, With<Hero>>, run: Res<Run>, mut scenes: Scenes) {
    if !heroes.is_empty() && heroes.iter().all(Health::is_spent) {
        scenes.go_to(Ended { won: false, wave: run.wave });
    }
}

fn clear_enemies(enemies: Query<Entity, With<Enemy>>, mut entities: Entities) {
    for enemy in &enemies {
        entities.despawn(enemy);
    }
}

/// Each player is asked once in each intermission. The offer is world state, so the cards come back after a
/// lost connection, and an answer that arrives after the wave has started is dropped with the offer.
fn offer_upgrades(mut offers: Offers<Upgrade>) {
    let choices = [Upgrade::FasterFire, Upgrade::MoreDamage, Upgrade::Heal];
    offers.open(Offer::to_each_player().choices(choices).until_phase_ends());
}

fn take_upgrades(mut offers: Offers<Upgrade>, mut heroes: Query<(&mut Hero, &mut Health)>) {
    for answer in offers.accept() {
        let Some((mut hero, mut health)) = heroes.of_player(answer.player) else { continue };
        match answer.choice {
            Upgrade::FasterFire => hero.fire_every = hero.fire_every * 0.85,
            Upgrade::MoreDamage => hero.damage += 1,
            Upgrade::Heal => *health = Health(100),
        }
    }
}
```

```rust
// src/screens.rs: what each player sees. Nothing in this file can change the run.
use crate::rules::*;
use thrum::prelude::*;

pub fn arena(scene: &mut Scene<Arena>) {
    scene.background(Color::rgb(0.05, 0.06, 0.08));
    scene.camera(Camera2d::fit_height(40.0).following_my_character());
    scene.each_frame((pop_where_enemies_die, status, upgrade_cards));
}

fn pop_where_enemies_die(died: Events<EnemyDied>, mut draw: Draw2d) {
    for death in died.read() {
        draw.burst(art::effects::POP, death.at);
    }
}

fn status(mut ui: Ui, run: Res<Run>, heroes: Query<(&Player, &Health), With<Hero>>, me: Me) {
    ui.top_left(|ui| {
        for (player, health) in &heroes {
            ui.bar(health.0 as f32 / 100.0).mine(*player == me.player());
        }
    });
    ui.top_center(|ui| ui.text(format!("Wave {}", run.wave)));
    ui.top_right(|ui| ui.text(format!("{} gems", run.gems)));
}

fn upgrade_cards(mut ui: Ui, mut offer: MyOffer<Upgrade>) {
    let Some(open) = offer.open() else { return }; // from the world: it stays until this player's answer lands
    ui.modal("Choose an upgrade", |ui| {
        for upgrade in open.choices() {
            if ui.card(upgrade.name(), upgrade.blurb()).key(upgrade).chosen() {
                offer.choose(upgrade);
            }
        }
        ui.status(offer.status()); // "Sending…" until the host has taken the choice
    });
}
```

These cards come between waves, so a modal suits them. A game that deals cards in the middle of a fight
describes them in `ui.bottom_center` instead: outside a modal the walk stick stays the hero's, and each card
takes `.pressed_by(Cards::FIRST)`, a local action. Several level-ups in a second queue, oldest first.

The fight is proved with the real combat rules and no window. The test tells the two bats apart, so a weapon
that picked the first bat spawned would fail it:

```rust
#[derive(Scene, Default)]
struct Fight;

/// A scene is a function, so a test builds the one it needs from the game's own rules.
fn fight(scene: &mut Scene<Fight>) {
    scene.space2d(Space2d::cells(4.0));
    scene.players().spawn(HERO).control(move_hero);
    scene.rules((fire_at_nearest, move_bodies, shots_hit, enemies_die));
}

#[test]
fn a_hero_shoots_the_nearer_bat_first() -> thrum::Result {
    let mut game = Game::new("fight").add(prefabs).scene(fight).test()?;
    let far = game.spawn(BAT.at(-8.0, 0.0))?; // spawned first, and further away
    let near = game.spawn(BAT.at(3.0, 0.0))?;

    game.step_until(seconds(3), |game| Ok(!game.alive(near)))?;

    assert!(game.alive(far));
    assert_eq!(game.get::<Health>(far)?.0, 3);
    assert_eq!(game.count::<With<Gem>>(), 1);
    Ok(())
}
```

### F. The same game, online, with bots

No rule changes. The game says how many may play and takes the standard lobby:

```rust
pub fn game() -> Game {
    Game::new("Swarm")
        .multiplayer(Multiplayer::players(1..=4))
        .lobby(Lobby::standard().starts(Arena))
        .add(rules::prefabs)
        .scene((rules::arena, screens::arena))
        .scene((rules::ended, screens::ended))
}
```

The standard lobby offers playing alone, hosting where this device can host, and joining from a link, a paste or
a launch argument. It shows the host a link to send, shows everyone who is here and how many may come, gives
*Start* to the host alone, and after a refused or closed match says why and offers to try again or go back.
`Lobby::standard().ready_up()` adds a ready mark each player sets for themselves, and `.host_panel(pick_arena)`
gives the host a panel of the game's own, whose choices the scene it starts is entered with. The lobby is an
ordinary scene on the match's clock, so a ready mark and a host's choice travel as any input does
([how](#11-multiplayer-an-opt-in-over-input-only-networking)). A game that wants its own screens uses
[`Session`](#11-multiplayer-an-opt-in-over-input-only-networking).

What made this possible is in the rules already: each player's hero is spawned by `scene.players()`, its
movement is a control function a match can run ahead, the upgrade is an offer, and the numbers agree on every
device. A game played alone is the same game with one player.

Alone, the arena's first tick has one player and one hero. With three friends in the lobby, four heroes stand
in a row on every machine. A friend who joins while a wave is running is sent the world as it stands, and
their hero is spawned at the tick they enter.

A bot is a player whose input a rule writes. Nothing else in the game knows it is not a person: it has a hero
from the same line of the scene, the same control function moves it, and it is offered the same upgrades, which
a rule answers for it.

```rust
pub fn arena(scene: &mut Scene<Arena>) {
    // ..as above, with two lines added before the players' line:
    scene.on_enter(fill_empty_seats);
    scene.rules(bots_flee); // written before the heroes move, so this tick's hero reads this tick's input
    scene.players().spawn(hero_in_line).control(move_hero);
    // ..and one rule added to the intermission, before `take_upgrades`:
    scene.phase(Stage::Intermission).rules((bots_choose, take_upgrades));
}

/// `PlayersMut` is the roster a rule may change. A frame function is given `Players`, which can only look.
fn fill_empty_seats(mut players: PlayersMut) {
    while players.count() < 4 && players.add_bot().is_some() {}
}

/// A bot's answer goes through the same check as a person's: once, for a choice it was offered.
fn bots_choose(mut offers: Offers<Upgrade>, players: Players, mut random: Random) {
    for bot in players.iter().filter(|player| players.status(*player) == PlayerStatus::Bot) {
        let Some(open) = offers.pending(bot) else { continue };
        if let Some(choice) = random.pick(open.choices()) {
            offers.answer_for(bot, open.offer, *choice);
        }
    }
}

fn bots_flee(mut bots: BotInputs<Steer>, heroes: Query<(&Player, &Transform2d), With<Hero>>, space: Space2d) {
    for (player, at) in &heroes {
        if !bots.is_bot(*player) {
            continue;
        }
        let nearest = space.nearest(at.position).on(ENEMIES).first();
        let away = nearest.map_or(Vec2::ZERO, |bat| (at.position - bat.position).normalized_or_zero());
        bots.set(*player, Steer { walk: Stick::toward(away) });
    }
}
```

**A person is never turned away for a bot.** A friend who joins this arena while bots fill it takes the
lowest seat a bot holds: in that tick the bot leaves as any player does, with its hero and its part in an open
offer, and the friend joins with a hero of their own. Seats, bots and joins are world state and admitted input,
so the host, the friend's machine and a replay agree ([seats](#5-players-actions-and-control)).

A match is proved in one process, in simulated time, with a slow link:

```rust
#[test]
fn a_late_joiner_gets_a_hero_that_answers_at_once() -> thrum::Result {
    let mut table = game().test_match()?; // a host and its peers, each with a world of its own
    table.host().start(Arena)?;
    table.step_for(seconds(5))?;

    let late = table.join(Link::delayed(millis(120)))?;
    table.step_for(seconds(1))?;
    let hero = table.peer(late).my_character()?;
    let before = table.peer(late).shown::<Pose2d>(hero)?.position.x;

    table.peer(late).steer(Steer::WALK, Stick::RIGHT);
    table.step_for(millis(50))?; // less than one trip to the host

    assert!(table.peer(late).shown::<Pose2d>(hero)?.position.x > before); // predicted on their own screen
    table.step_for(seconds(2))?;
    assert!(table.digests_agree()); // and every world is still the same world
    Ok(())
}
```

### G. A third-person 3D slice, on the extension map

Hollow Vale: walk a valley, carry a relic on your back, stay out of a warden's sight, alone or with a friend.
**This example is written against entries of [the extension map](#the-extension-map)**: `Space3d`, `Level3d`
and `move_shape` in 3D wait on [scope question 1](#scope-questions-for-the-owner), and the gamepad bindings on
question 2. Everything else in it is first-release API. It is here so the owner can judge the 3D game the
design leads to.

```rust
#[derive(Actions)]
pub struct Walk {
    #[bind(Keys::WASD, Pad::LEFT_STICK, Touch::stick())]
    pub go: Stick,
    /// Which way the player is looking. It is kept on their own machine at the frame rate, so the camera
    /// turns with no delay, and its latest value travels with each tick's input.
    #[bind(Mouse::look_x(), Pad::RIGHT_STICK_X)]
    pub look: Heading,
    #[bind(Key::E, Pad::SOUTH, Touch::button("Take"))]
    pub take: Button,
}

pub const BACK: Part = Part::new("back"); // a named place on the pilgrim, declared in its prefab

pub fn vale(scene: &mut Scene<Vale>) {
    scene.phases(Chapter::Arrival);
    scene.on_enter((load_the_valley, begin_or_resume)); // the ground and its solid models, from content; then the journey
    scene.players().spawn(PILGRIM).control(walk);
    scene.rules((take_or_set_down, wardens_look, wardens_move, wardens_catch));
}

/// "Forward" is away from the camera, and the camera's angle arrives as input. The pilgrim's own collider is
/// the shape that moves. Everything here is something a match runs ahead.
fn walk(
    at: &mut Transform3d,
    body: &mut Body,
    shape: &Collider3d,
    input: &Walk,
    tuning: Res<Tuning>,
    ground: Level3d,
    tick: Tick,
) {
    let forward = input.look.direction_xz();
    let wish = (forward * input.go.y + forward.right() * input.go.x) * tuning.walk_speed;
    body.velocity = vec3(wish.x, body.velocity.y - tuning.gravity * tick.dt(), wish.z);

    let moved = ground
        .move_shape(at.position, shape, body.velocity * tick.dt())
        .max_slope(Angle::degrees(40.0))
        .step_up(0.3)
        .on((TERRAIN, SOLID));
    at.position = moved.position;
    body.velocity = moved.slide(body.velocity);
    if wish != Vec3::ZERO {
        at.face(wish);
    }
}

fn take_or_set_down(
    pilgrims: Query<(Entity, &Player, &Transform3d), With<Pilgrim>>,
    inputs: Inputs<Walk>,
    space: Space3d,
    mut entities: Entities,
) {
    for (pilgrim, player, at) in &pilgrims {
        if !inputs.of(*player).take.pressed {
            continue;
        }
        let back = entities.part(pilgrim, BACK);
        if let Some(carried) = entities.children(back).next() {
            entities.detach(carried).keep_world(); // it stays where it is, on the ground
        } else if let Some(relic) = space.within(at.position, 1.8).on(RELICS).first() {
            entities.attach(relic.entity).to(back).keep_local(Transform3d::ORIGIN);
        }
    }
}

fn wardens_look(
    mut wardens: Query<(&Transform3d, &mut Mind), With<Warden>>,
    pilgrims: Query<&Transform3d, With<Pilgrim>>,
    space: Space3d,
    tuning: Res<Tuning>,
    tick: Tick,
) {
    for (at, mut mind) in &mut wardens {
        let eye = at.position + Vec3::UP * 1.6;
        for pilgrim in &pilgrims {
            let chest = pilgrim.position + Vec3::UP * 1.0;
            // One ray: a space meets the level too, so a hill hides a pilgrim as a boulder does.
            let hidden = space.ray(eye, chest).on((TERRAIN, SOLID)).first_hit().is_some();
            if eye.distance(chest) < tuning.sight && !hidden {
                *mind = Mind::Chasing { last_seen: pilgrim.position, gives_up: Timer::after(tick, seconds(4)) };
            }
        }
    }
}

pub fn vale_screen(scene: &mut Scene<Vale>) {
    scene.sky(Sky::clear());
    scene.ground(Ground::heights(art::terrain::VALE)); // how the ground the rules stand on is drawn
    scene.camera(
        Camera3d::orbiting_my_character()
            .yaw(Walk::LOOK)
            .distance(6.5)
            .height(1.5)
            .damping(seconds(0.12))
            .avoiding((TERRAIN, SOLID)),
    );
    scene.each_frame((sun_follows_the_hour, journal, save_at_the_shrine));
}
```

A warden's state machine is an enum with timers in it (`Mind`), which rewinds and saves with the world. The
journey is saved at the shrine and resumed from the title screen, into the chapter it was saved in:

```rust
#[derive(Save)]
#[save(version = 2, from = ValeSaveV1)]
pub struct ValeSave {
    pub hour: Num,
    pub chapter: Chapter,
    pub chapter_time: Seconds,
    pub placed: Vec<Id<RelicDef>>,
    pub carrying: Option<Id<RelicDef>>,
}

/// Reads the world at a committed tick and returns what to keep. It runs when a save is asked for.
pub fn capture(day: Res<Day>, chapter: Phase<Chapter>, relics: Query<(&Relic, &Placed)>, family: Family) -> ValeSave {
    /* .. */
}

/// A save chosen on the title screen arrives here, as input to this tick.
fn begin_or_resume(
    restored: Restored<ValeSave>,
    mut chapter: PhaseMut<Chapter>,
    mut entities: Entities,
    mut day: ResMut<Day>,
) -> thrum::Result {
    match restored.get() {
        Some(save) => {
            // The chapter carries on from where it was. No chapter is entered, so its sting does not play again.
            chapter.resume(save.chapter, save.chapter_time);
            restore(save, &mut entities, &mut day) // an error here refuses the tick: never half-restored
        }
        None => new_journey(&mut entities, &mut day),
    }
}

// In the game: `.save::<ValeSave>(capture)`. The title screen shows *Continue* where
// `saves.peek::<ValeSave>(Slot::AUTO)` is `Loaded::Ready`, and asks with
// `request.scene(Vale).restoring::<ValeSave>(Slot::AUTO)`.
```

With a friend it is the same valley. The game says `.multiplayer(Multiplayer::players(1..=2))` and takes the
standard lobby, and no rule above changes (O6): `walk` reads only what a match runs ahead, so a guest's pilgrim
answers at once against the ground it stands on. [Example F's test](#f-the-same-game-online-with-bots) proves
it here with `shown::<Pose3d>` in place of `shown::<Pose2d>`.

## Research and selection

Bevy 0.20, Godot 4.6, Unity, Unreal's gameplay framework, Photon Fusion 2, macroquad, lightyear and bevy_ggrs
were each read for how a game's code is shaped, not for its features, and the owner's review of 26 November
compared the first design with seven of them line by line. Its findings are the decisions above. The choices
that give the core its shape, and what would change each:

1. **Plain-data components and rule functions, not objects with methods** (from Bevy, with its names: `Query`,
   `Res`, `With`, `Single`). Thrum's simulation is a fixed tick over registered state that is rewound,
   replayed, digested and shared by causes, and only state the engine can see whole can do that. Godot's state
   inside objects cannot. *It would change* if multiplayer and replay were dropped from the SDK's aims.
2. **One written order, with no schedule graph.** Bevy's graph finds parallelism between systems and is the
   largest source of surprises in Bevy code. Thrum's parallelism is inside a rule, where a crowd's cost is.
   *It would change* if a profile showed whole rules that could run side by side dominating a tick.
3. **What a rule did is what the next rule sees** (R2), in place of edits that land when a tick ends, which
   made a stomp, a pick-up and a wave's last clean-up each a tick late.
4. **Two kinds of function, told apart by type** (from Fusion's `FixedUpdateNetwork` and `Render`). A rule
   cannot take `Ui` and a frame function cannot take `ResMut<T>`, and each mistake is a compile error.
5. **Looks are components, and how one screen dresses them is not** (R6).
6. **One vocabulary for 2D and 3D, with the dimension in the name where the two differ.**
7. **Portable numbers in rules, floats on the screen** (D1, R4). `f32` everywhere for games played alone would
   make "turn on multiplayer" a rewrite and give up replay across devices.
8. **A scene is a type, a prefab and a layer are typed constants, and an action is a field**, so each is found
   by an editor's completion, checked by the compiler and named the same in a save, a recording and an error.
9. **The interface is described each frame and kept by the engine**, where current interface kits have settled
   (React, SwiftUI, Compose).
10. **One package, with a `rules` module checked by being built alone**, not a rule crate a first-time maker
    must learn a workspace for.
11. **A controlled character that a match runs ahead, and a confirmed world round it** (from Fusion's
    predicted controller and Factorio's latency hiding), not whole-world rollback, which `SESSION` measured at
    about 50 ms a correction for a crowd of 10,000. Unreal's separation of a player, their controller and
    their pawn gives a player who outlives their character and a bot that is a player.
12. **Only input travels** (from lightyear's and bevy_ggrs's deterministic replication), made the only way to
    write a rule and not bolted on afterwards.

## The system as it runs

### State and owners

| State | Owner | The SDK's part |
|---|---|---|
| Entities, components, resources, the tick, rewind, publication | `thrum-core` (G1, G2S) | Registers what the game's description names; adapts rule functions to registered systems; asks for the barrier after each |
| The rules profile, the digest, the 2D index, fixed collision and the character move | `SIM-2D` | `Num` and friends are its numbers; `Space2d` is its index, one for each layer; `Level2d` is its fixed collision |
| The frame, the recipe, reconstruction, EDR, the governor | `APPLE-TILE`, `APPLE-PRESENTATION` | None. A game gets them by default |
| Sprite rows, sheets, the 2D scene | `RENDER-2D` | Maps `Sprite`, `Shape2d`, the tile layer and `Transform2d` to rows |
| Sky, Sun, air | `ATMOSPHERE` | `Sky` sets its inputs |
| Cooked products and live edits | `CONTENT-LOOP`, `COOKED-DRAW` | Resolves the catalog's names; gives an entity's `Model` its cooked mesh row and a placement to the cooked placement as built; cooks content files |
| The 3D scene's entity rows and the passes that place them | `thrum-render` | Maps an entity's `Model`, its material and `Transform3d` to a row |
| Typed controls, parameter files, the profiler | `CREATOR` | `Settings` declares controls |
| The interface tree, events, fonts | `GAME-UI` | `Ui` keeps the tree |
| The device, recovery, the game contract | `HOST` | Implements the game contract once, for every game |
| The match, the roster, prediction | `SESSION`, over `TRANSPORT` | Implements the session's world once, and gives a control function to the predicted controller as its step |
| Which scene and phase run, the tick each was entered at and the transition asked for; the seats, with who is a bot, each button's last count and any slot given no seat; whether the game is paused, by whom, and the game's tick; open offers, and commands and answers waiting for a pause to end; the level's recipe, with its tile map's identity; the pieces of a save being brought in | The world, as resources the SDK registers | Keeps them as world state, so they are digested, rewound and sent to a late joiner |
| Events for the frame, effects under way, `Screen<T>` with the input context, `Shown<L>`, bindings, the built collision of each recent level recipe | The SDK, as this screen's or this machine's state. It is not world state and is in no digest | The ring of recent ticks' events, each cursor, the count of each control line's emissions shown for each recent tick, the bursts still playing; on each machine, whether its screen shows the pause, which decides what its sampler counts |
| Saves and recordings | The SDK, in each target's storage | Runs the capture on a committed tick, writes away from the tick, brings a save in as input |
| The game's scenes, rules, looks and content | The game | — |

Rules run on the world's own thread where the shell has one, as today. Frame functions never wait for them:
they read published pages, on the thread that samples the scene.

## Interfaces

**Provided.**

| Interface | To | What it is |
|---|---|---|
| **The `thrum` crate** | Every game, the kit games, the templates | [The core](#the-core), sections 1 to 13 |
| **The game commands** | Every game | [Section 14](#14-running-a-game-documentation-and-templates), and `./dev new` |
| **The SDK guides and the one file** | Game makers and their agents | [Section 14](#14-running-a-game-documentation-and-templates) |

**Asked of other modules.** Each is new work inside state another module owns, built by this module's builder
under [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk), on what the owner has landed, and the
owner's afterwards. No contract's text changes. C01's stable identities, C02's committed state, C11's event
order and C13's authority are what the core stands on.

1. **The rules profile** (`SIM-2D`): literal constructors that are `const`; angles and turning; `thrum` as a
   dependency checked rules may have; refusals of files, sockets, processes and standard input; a check of a
   game's `rules` module built on its own, outside the engine's registry; and, as additions to `thrum-sim2d`,
   exact tests for rectangles and capsules beside the index's circles, fixed collision built from tiles and
   shapes, and the box and circle move that reports contacts.
2. **The registered simulation** (core): queries over several components with filters; a barrier after each
   registered system at which the tick's staged structural edits are applied, inside the tick's one
   transaction, for a schedule that asks for it; and what a parent link and a disabled entity need.
3. **The shells and the tool** (`CREATOR`, `HOST`, `GAME-UI`): what the window, the page and the phone shell
   still assume of the V1 game or the sky kit; local actions, the pointer and its capture handed to the SDK;
   storage for saves on each target; a driver that need not step a paused tick that admits nothing; the game
   commands in `./dev`.
4. **The interface and the match** (`GAME-UI`, `SESSION`): used as built. A press count and a sampled bit on
   the input frame's axes; the predicted controller's whole state carrying what the step reads, compared as
   `SESSION` compares it; seats, with bots, that the SDK keeps in world state over `SESSION`'s slots; a lobby
   that runs on `SESSION`'s ticks from the moment of hosting; a pause kept as world state while the host goes
   on committing; a save brought in as pieces budgeted against the host's whole 920-byte frame, with a
   failed restore retried inside the world's one step; a match of fewer than four seats kept to its number
   by the SDK, which is what hands `SESSION` each link and what seats each joined slot; a note of a step's
   emissions held in the `Rules` value the SDK gives the predicted controller; a player's name sent as
   their first command; a `Heading` carried as a latest-value action; and a 3D world's image, digest and
   control function all fit what `SESSION` has, and none asks it to change.
5. **The repository guard** (`REPO-RULES`, `GAME-SPLIT`): `thrum` registered, and
   [the rule](#1-a-game-one-crate-one-package-one-value) that every package of a game names no engine crate but `thrum`.
6. **The 3D scene's entity rows** (`thrum-render`, and `CONTENT-LOOP`'s cooked mesh rows): the turn and the
   scale at both ticks, interpolated by every pass that places an entity row; a row's mesh chosen by name
   when its entity is spawned; and a row's material taking a texture.

## Cost

The SDK is a layer over engine paths that are already costed. It must add nothing that scales with the number
of entities beyond what the engine does.

| Work | Account |
|---|---|
| A rule | One call each tick through the engine's registered callback, into a function monomorphised for the rule's parameters. The loop over entities is inside it: no dynamic call for each entity and no allocation in a tick |
| The barrier after a rule | Nothing when the rule staged no structural edit, which is most rules in most ticks. Otherwise the staged batch, once |
| A query | A walk over the engine's columns. `par_each` uses the engine's chunks |
| A space query | One index for each layer, with its storage kept. A layer is rebuilt at most once after each rule that changed it, and not at all in a tick that did not: a crowd that moves once a tick is indexed once a tick, as the reference crowd is |
| The level | Built once for each recipe, when a rule that changed it finishes, and kept by the recipe's identity while a rewind or a step run ahead may ask for it. A character move tests the tiles its box sweeps |
| A control function | One call for each player's character |
| The access check | Against signatures and prefabs at startup, when an entity with a new set of components first appears, and when a player is given another character |
| Extraction | Changed rows only, mapped by code written once in the SDK. A tile layer is extracted when it is loaded |
| A frame function | Runs once a frame, over what it names |
| The interface | A description of tens to a few hundred nodes compared with the last one. An unchanged interface draws once |
| A save | The capture, once, on a committed tick; the file is written off the tick. A restore is gathered a frame's room of pieces a tick, about 850 bytes in a match, and decoded once |
| A pause | A paused tick admits input and runs the rules marked for it. Alone, a paused tick that admits nothing need not be stepped |
| A capability left out | Nothing: a 2D game compiles no 3D pass, a game with no interface has no stage, a game without the `multiplayer` feature links no network crate |

**Tripwires**, recorded and investigated while building goes on; none is a pass condition beyond a missing
measurement or a collapse:

- the reference crowd's behaviours at 10,000 bodies as SDK rules cost about 25% more a tick than the
  hand-written reference, the brief's figure for a comparable regression;
- any allocation in a warmed tick or frame of the 2D starter that the same scene on the engine's API does not
  make;
- a frame of the showcase on the SDK that costs about 25% more than the same frame on today's API.

## The work packages

Fourteen, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds
the items of the first three. Numbers are names, not the order: WP3, WP5, WP6 and WP7 keep the numbers other
modules already wait on. No work package builds an entry of [the extension map](#the-extension-map) but the
one WP3 is given: a "yes" to a scope question adds work packages, which a planner then plans.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Sections 2, 3 and 4 with no window, for [the headless subset](#what-wp1-compiles): the world, rules in written order with each rule's edits landing when it ends, parameter structs, scenes as types, phases with `resume`, timers, the test harness; the surface and documentation checks | Nothing |
| WP11 | Prefabs and the tree of section 2: prefab names in `prefabs!` blocks that the program lists for the SDK, variants and what a spawn must bring; children, parts, attaching and disabling, with turns that are quarter turns; `keep_world` with its bound and its refusals; the access check made on what exists | Nothing |
| WP2 | Sections 5 and 8 and `Space2d`: the numbers page and the rules check; action maps and how a button's presses are counted, players as stays in seats, a controlled character, bots; the crowd on the SDK, equal on three targets, with its cost | `SIM-2D:WP3`, Done |
| WP12 | The rest of section 7 and content: the level's recipe and fixed collision, tile maps, a space that meets the level, the character move; typed content, its validator and IDs; and a shared setting as a game resource with its defaults, which rules read and a test sets, with no panel. Example D's rules, proved with no window | Nothing |
| WP3 | 2D on screen: section 9 in 2D, section 14's commands and the asset catalog for the Mac and Chrome, with [sheets cooked from source images](#sprites-clips-and-tiles-from-source-images), the first `run()`, cameras and rigs, `Shown`, local actions, input contexts, the pointer, the pause alone and `--shot`. Rockfall and Skylark without their interface | Nothing still open |
| WP4 | 3D on screen: section 9 in 3D, with entities' models drawn as entity rows that turn, scale and take a texture, placements, and the rest of settings: local settings and the panel, and a shared setting's panel edit admitted at a tick. The starter, stones and the lighthouse are written on the SDK as new kit games | For its change to the instance passes, the V1 work packages that hold them |
| WP6 | Section 6 and the interface: modals, commands, offers that queue with a bot's answer, requests, what a menu does under its own pause, and `TestScreen`. The interface of Rockfall (its score and result page) and of Skylark (its pause menu) | `GAME-UI:WP4` |
| WP13 | Saves and recordings: a saved resource, a run saved and resumed alone into its phase, `--record`, `--replay` and `play_recording` | Its own WP6 |
| WP5 | Section 11: the crowd hosted and joined, a control function run ahead with what it reads, the standard lobby on the match's clock, seats and bots in a match, a pause in a match, a save restored in a match, `TestMatch`, and a 3D match rehearsed with no window | `SESSION:WP7` |
| WP14 | A 3D match on screen (O6): the 3D starter hosted on the Mac and joined from Chrome and a second Mac process through the standard lobby, each player's ball answering at once on their own screen | Its own WP4 and WP5 |
| WP7 | The iPhone simulator | `GAME-UI:WP6` |
| WP8 | The guides, the reference file and the templates of section 14, [the guard](#1-a-game-one-crate-one-package-one-value), and three fresh agents | Its own WP1 to WP7 |
| WP9 | The showcase on the SDK, in the games repository | `GAME-SPLIT:WP4`, which follows the V1 exit |
| WP10 | The old creator route removed | Its own WP9 |

Each work package's surface arrives with its reference documentation and its rows of
[the mistakes table](#12-mistakes-that-explain-themselves), because WP1's guard holds every later one to them.
A work package is compared with the worked examples only for the surface it builds.

What the planner who gives a later work package its items carries into them. Each proof is listed under the
first work package that has everything it uses, so no item waits on a later one.

- **WP12:** a level loaded from a content file in an enter function; a shape given as the character's own
  collider that lands on a floor, stops at a wall, passes up through a one-way tile and stands on it;
  `moved.slide`; a `teleport` that sets the place and marks the tick; a ray and a `motion_of(..).sweep()` asked
  of the space on two layers, one the level's and one of colliders, each reporting whichever stands nearer
  along the path, with no entity for the level; a hit's normal pointing toward the mover on a floor, a wall
  and a ceiling, and a sweep that starts in contact reporting fraction zero with a normal that is not zero; a
  tile map from a file and a rectangle a rule added later, both giving the same `move_shape` and `ray` answers
  after a rewind and a replay; one content record that holds two tile maps with a wall in different places,
  and a list of them, where loading each gives its own wall before and after a rewind to a tick at which the
  other was loaded; a validator that counts a map's cells of one glyph; a content record refused before play
  by its type and by the game's validator; rules that read a shared setting registered with its defaults and
  set by the test; and example D's two coyote tests.
- **WP3:** a picture the engine encodes and drops, with an event delivered to its run and a `burst` started
  in it shown in the next; a `teleport` with no picture between the two places; a pause, toggled by a local
  action, in which the game's tick stands, no timer moves, a rule marked `.even_when_paused()` runs and no
  other does, and which the same key ends; a tap of a shared button made during that pause, which no tick
  after it reads, a tap made after it, which is read once, and a direction held through it, which is still
  held after with no `released`; a key bound in two input contexts driving only the active one's action, and
  a key that went down in one not pressing the other's; a frame function that reads a `Timer` through `Tick`
  and shows the same time left a rule reads; a rig whose distance a frame function changes, which goes on
  following; `Camera2d::exactly` showing the same rectangle in a wide window and a tall one; a sheet with a
  clip and two tiles cooked from an image and its file, a misspelled frame refused by the check with the
  nearest name; a frame function that asks for `PlayersMut`, which does not compile.
- **WP4:** example B's cooked, textured rock, captured in motion: an entity whose `Model` names a cooked mesh,
  turning in place, with its lit faces and its cast shadow turning together between ticks. A built-in cube
  does not stand in for it. And example B's ball, steered by the keys on the Mac and in Chrome.
- **WP6:** an offer answered twice, answered late, answered for a bot by a rule, and shown to a player who
  joins while it is open; three offers opened to one player in one tick, shown and answered one at a time in
  the order opened; a command sent in a dropped picture, admitted once; a `request.scene` the scene does not
  accept; a pause menu as a modal that the key which opened it closes; a modal that holds the pause, whose
  button sends a command that a rule marked `.even_when_paused()` reads, after which the world closes the
  modal and the pause ends; a command no such rule takes, sent during a pause and read once in the first tick
  after it; a `request.scene` sent from a pause menu, which lands and ends the pause; a card outside a modal
  pressed by a local action while the stick that walks the hero goes on walking it.
- **WP13:** a save written by one build's type and restored by its successor's; two save types in one game,
  each request reaching its own capture; a save asked for from a pause menu, written while the pause holds; a
  save restored into a phase that lasts ten seconds, four seconds in, which ends six seconds later with no
  enter function of any phase run; a `Timer` saved with three seconds left that has three left when
  restored; a restore whose enter function fails, a save over its limit and a damaged one, each leaving the
  world as it was and the scene not entered, with the step that tried it returning no error and the next step
  running; a recording that holds a restored save and a saved resource's starting value, replayed with
  another value in the slot to the same digest; a recording made on the Mac replayed in Chrome to the same
  digest.
- **WP5:** a control function that reads a shared setting, the phase, the level and `tick.in_scene()`, run
  ahead for a player who joined at a tick other than zero, in a scene entered at a tick other than zero:
  every step it runs ahead with the frame the host then admits for that tick, while nothing it reads has
  changed, equals the host's, and a hero the function holds still for the scene's first two seconds is
  released at the host's tick and not the joiner's; a frame the host holds, after which that tick is
  corrected; the host changing that setting while the character stands still, after which the peer is
  corrected and its later steps are equal again on the same terms; the same phase entered again, corrected
  the same way; a function that sends one dust event when run ahead and two at the confirmed tick, where
  the player's screen is given the first once, when predicted, and the second once, at the confirmation,
  and one that sends the same event in every run, given once across a correction; the startup refusal
  without `.unpredicted()`; a press sent on a late frame and one made across a neutral frame, each read
  once; with the last count read at five, a tap made while a peer's screen shows a pause and a tap made
  after it shows the game running, each with that peer's frames lost through the pause's end, where the
  first is never read and the second is read once; a person joining a table bots have filled, with the same
  seats, characters and digest on the host, the joiner and a replay, and a shot the bot had fired naming
  nobody afterwards; in a match of two seats with a host and a guest, a third person turned away as full
  with no event and no character, the same third person given no seat when they arrive while the guest is
  away, and the guest returning within the grace to their seat and character; a ready mark and a host's
  panel choice made in the lobby and carried into the scene it starts, and a rule that returns everyone to
  the lobby; a pause ended from a peer, with no burst of ticks after, and a command a peer sent during it
  read once; an open offer shown again after a reconnect; a rectangle a rule added to the level, met by a
  peer who joined after it; a save within 1 KiB of its 64 KiB limit that the host restores while peers play
  and while the host sends a command in some of the same ticks, arriving over many ticks with no frame
  `SESSION` refuses and every world's digest equal afterwards, and a damaged one, after which the match goes
  on, the scene is not entered and the digests are equal; **and a rehearsed 3D match**: a control function
  that writes a `Transform3d` from a `Stick` and a `Heading`, run ahead for a late joiner whose shown
  `Pose3d` moves in less than one trip to the host, a turn eased the shorter way after a correction, and
  every world's digest equal afterwards.
- **WP14:** example B's game with `.multiplayer(..)` and the standard lobby, hosted on the Mac and joined from
  Chrome and a second Mac process: a late joiner's ball on the host's screen, the joiner's own ball moving on
  their screen before the host confirms it, and the three digests equal; captured in motion on the Mac and
  in Chrome.
- **WP8:** the guard's refusals; an unfamiliar change made from the guides alone.

### What WP1 compiles

WP1 has no window, no input, no prefab and no space query. Its test games are written from this subset:

- **The game and its scenes:** `Game::new`, `.scene(..)` with one function and with a tuple, `.resource(..)`,
  `.component::<T>()`, `.max_entities(..)`, `.tick_rate(..)`, and `.test()` and `.test_in(..)`, which return
  a result; scene types that
  derive `Scene`, with and without fields; in a scene, `resource`, `on_enter`, `on_exit`, `rules`, `phases`,
  `phase(..)` with `on_enter`, `rules`, `on_exit`, `lasts` and `then`, and `add`.
- **Rule parameters:** `Query` with `With` and `Without`, `Single`, `Res`, `ResMut`, `Entities` (`spawn` of a
  tuple, which returns its `Entity`, `despawn`, `insert`, `remove`), `Events` and `Emit` for the game's own
  events, `Tick`, `Random`, `Scenes`, `Phase` and `PhaseMut` with `resume`, and a struct that derives
  `Params`; `every`, `when` and `par_each`.
- **Data:** the `Component`, `Resource`, `Data`, `Event`, `Scene` and `Phase` derives, `Timer` and `Cooldown`,
  with plain components in place of transforms and looks. `Num` and `Vec2` are the profile's numbers as they
  stand, made from integers. Literals, angles and `Vec3` arrive with WP2.
- **The harness:** `step`, `step_ticks`, `step_for`, `step_until`, `spawn`, `despawn`, `set`, `get`,
  `resource`, `set_resource`, `count`, `find`, `single`, `alive`, `phase`, `scene`, `in_scene`, `go_to`,
  `go_to_phase`, `run`, `events`, `digest` and `rewind`.

From the examples, that is: example A's `Score`, `drop_rocks` with its cadence, `fall`, and `Play` and
`GameOver` joined by `go_to` with the score as the scene's field; and example E's `Run`, `Stage`, its three
phase blocks with their lengths, `next_wave`, `enemies_die`, `heroes_fall` and `clear_enemies`, with a tuple
in place of each prefab and a plain position in place of `Transform2d`.

## Risks

1. **Core cannot apply structural edits after each system inside one transaction.** Early sign: WP1's first
   item, which is written to be tried before anything else. If it cannot be built, the builder ends `BLOCKED`
   and the planner brings the owner the choice, because R2 is the owner's.
2. **The API in the examples cannot be built as written.** The likeliest places: an action map read as plain
   fields over the input frame, a control function with its context as a predicted controller, the access
   check made on what exists and seen through a parameter struct, frame functions with no window, and the
   interface kept from a per-frame description. Early sign: WP1, WP2, WP5 and WP6. The builder writes the
   difference in the log and ends `BLOCKED`, because a change of shape is the owner's to approve.
3. **A convincing fake of K2:** an example that reads plainly because the plumbing moved into a helper file
   of the game. Early sign: the reviewer's count of game lines includes every file of the game.
4. **The surface is too wide to document well.** Early sign: WP1's surface check, which fails a public item
   with no example, and WP8's agents.
5. **Angles and 3D turns in the profile differ on one target.** Early sign: WP2's conformance run. The fallback
   is a table.
6. **Compile errors from typed parameters read badly.** Early sign: each work package's mistakes item, which
   holds the message text.
7. **The layer costs the crowd its speed**, with a barrier after each rule and layers rebuilt on change.
   Early sign: WP2's cost record.
8. **Turning, textured models change passes the V1 exit stands on.** Early sign: an exact image that moves in
   WP4. A row that does not turn and has no texture is placed and shaded exactly as today.
9. **A wrong substitute for K7:** a fresh agent that succeeds because it read the examples' source in the
   engine checkout. Its brief allows the guides, the one file and the template, and its record lists every
   file it opened.
10. **A target's program cannot carry the list of `prefabs!` blocks.** Registration at link time is
    ordinary on the Mac and the simulator; on the browser target it needs the program's constructors run at
    start. Early sign: WP11.3 on the Mac, and WP2's conformance runs, whose trace row carries how many
    prefab names the SDK knew before it read the game's description. If a target cannot carry the list, the
    builder ends `BLOCKED`, because rooting a block by a line of game code is a change of shape.
11. **A 3D game's action set does not fit `SESSION`'s input frame.** Three 16-bit axes carry every button,
    `Axis` and `Stick` of the set, and a `Heading` rides an action. Early sign: WP2.4, which holds a set of a
    `Stick`, a `Heading` and four `Button`s that must fit. If an ordinary set cannot, the builder writes a
    proposal in `SESSION`'s log (ruling 4) and ends `BLOCKED`.
12. **An entry of the extension map turns out to need a core name changed.** That would mean the shape was
    not locked. Early sign: the design review of this design, which checks each entry against the core, and
    the plan of the first entry the owner asks for. The planner brings the difference to the owner before
    anything is built.

## Proof

- **Correctness**, where a fault is silent and costly: written order and what a rule sees; equal digests when
  a run is repeated, rewound and replayed, and on the three targets; a refused tick leaving the world
  unchanged; the access checks; a match that stays equal, in 2D and in 3D. CPU tests on small worlds.
- **Equal digests are never the only witness.** A digest that left something out would agree with itself. So
  a refused tick is proved by reading the components, the resources and the entities back, value by value;
  the digest is shown to change when any one of them changes; and the crowd's trace carries witnesses of the
  work it claims.
- **Mistakes**: one test for each row of [the table](#12-mistakes-that-explain-themselves), holding the
  message's code, object, cause and next step.
- **The surface**: no engine type in `thrum`'s public items; an example on each; guides that compile.
- **The shape**: a design reviewer checks each entry of the extension map against the core: it names its
  extension point, adds names and changes none.
- **Looks**: the kit games drawn on the Mac and in Chrome, inspected in motion through T2 capture.
- **The docs**: three fresh agents, commissioned by the lead as `CREATOR`'s was, one for each kind of game,
  each then asked for a change the guides did not show.
- **Cost**: the tripwire records above.

## What a builder may change

The core and the worked examples are the shape the owner approved. A builder may change a **spelling**: a
method's name, an argument's order, a derive's attribute, where the game's code would read as well or better.
Each is a line in the log. A builder may not change the **shape**: add a concept a game maker must learn,
remove one of the fifteen words, change what a rule sees or when its edits land, change the written order,
change what a control function may read or which of it a match runs ahead, change what happens in a pause,
move a number between the portable kind and the float kind, move a line of plumbing into game code, make a
game name an engine crate, build an entry of the extension map, or build something that closes one of the
eight extension points. Where the shape cannot be built, the builder ends `BLOCKED` with the difference, and
the planner brings it to the owner.

When a work package is Done its examples in this design are replaced by the ones that compile, so the design
keeps reading as what is true.

## Open questions

- **How many entities a world holds by default.** WP2's builder chooses from the cost record and documents it.
- **How the SDK learns the module a registered function is defined in.** WP1's builder chooses, and says in
  the log what a game maker would see if it guessed wrong.
- **How the rule-set identity is derived with no line of game code.** `SESSION` hashes the rule sources in a
  build script of the game. WP5 settles where the template carries it.
- **Whether 64 KiB is the right default limit for a save.** At the host's frame budget it takes about a
  second and a third to bring in to a match. WP5 records the time it measures.
- **How an action set is packed into the input frame**: how many presses a button's count holds, above the
  three the design asks for, and how fine a stick is. WP2's builder chooses from what fits, and the reference
  says it.
