# Game SDK: one crate a game is written against

26 November 2026. Rewritten that day from [the owner's review](../../../.orchestrate/evidence/GAME-SDK/owner-review-2026-11-26.md)
and three dogfood games, and revised twice after design reviews of the rewrite, whose eighteen and then ten
findings [the log](LOG.md) answers one by one. It replaces the design of 25 November. [Plan](PLAN.md) · [done-when file](DONE.json) ·
[log](LOG.md) · [module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [glossary](../GLOSSARY.md) ·
[the creator guide](../../design/ENGINE-EXPERIENCE.md), whose ambition this module builds ·
[the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) ·
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

**The owner approves this design before any builder starts.** A design review that returns Design Approved is
not that approval. The owner judges the API by reading [the worked examples](#worked-examples), and answers
[the scope questions](#scope-questions-for-the-owner), which this design does not decide.

## The request

The owner, 25 November 2026: "What I would like you to plan is a high end, intuitive, expressive, readable, SDK
that game developers can use to build new games. this should be what V1 uses ideally and what we will use to
build the swarm game. It's VITAL that we get this right as it's the entire interface that developers and coding
agents will use to build a variety of games. It's the front of the product we're building. So it needs to be as
intuitive, well documented and easy to use as possible." And: "V1 first, Rust only, I'll approve the design."

The owner, 26 November 2026, on this rewrite: "We're looking for something that's readable, expressive,
intuitive and powerful. Don't optimise at all for reducing the amount of re-work needed. Let's just find the
best possible design."

The outcome: a developer or agent who has never seen Thrum builds a small 3D game, a small 2D game and a
networked game from the SDK's documentation alone, and the code reads as plainly as the game it describes. One
structure and one vocabulary across 2D, 3D, single-player and multiplayer; good-looking defaults; mistakes that
explain themselves; every public item documented with an example.

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
  floats. A state-replicated mode, for private information or an outside simulation, is not built
  ([numbers](#9-numbers), [rules](#3-rules-and-the-tick)).
- **D2 Authored objects are Rust.** A prefab with editable defaults, typed assets and optional children, and
  one model vocabulary whether a thing is placed or moves. No visual editor
  ([prefabs](#5-prefabs-children-and-parts), [looks](#11-looks-what-is-drawn)).
- **D3 The first character is a player's.** The first steerable thing in a first game belongs to a player,
  stays responsive in a match against fixed 2D scenery, and enters a standard solo, host and join lobby with
  its movement function unchanged. Dynamic interactions use confirmed state
  ([players](#6-players-actions-and-control), [multiplayer](#15-multiplayer-as-an-opt-in)).
- **D4 Typed content IDs from day one.** Gameplay content is typed, validated and named by stable IDs from the
  first release. Versioned saves that a later build loads, and data packs a game ships without recompiling,
  are year-one commitments, advertised when they hold ([content and saves](#12-content-assets-and-saves)).
- **R1** The first playable thing is a character you control. **R2** Written order tells the truth: a rule's
  finished edits, structural ones included, are what the next rule sees. **R3** A game defines a reusable
  thing, changes one detail and attaches another thing. **R4** Portable numbers only where the game's result
  depends on them. **R5** Phases, timers and resources have an owner and a lifetime. **R6** Static and moving
  objects share one visual vocabulary. **R7** Player input, local bindings and prediction eligibility are
  separate. **R8** A co-op choice and a lobby are ordinary things to author. **R9** It is easy to prove the
  game the author meant. **R10** Authored data and saved progress have stable identities.

**What three games found.** Three agents then each wrote a whole game against the design as the review amended
it: an online 3D arena shooter, a 2D precision platformer and a 3D third-person exploration game (their reports
and source are in the main checkout under `tmp/sdk-dogfood/`). All three said they would not yet choose the
SDK over Godot or Unity for their game, and all three praised the same core: rules in written order, the typed
wall between rules and the screen, phases, and tests with no window. This design keeps that core, gives a named
type and a signature to everything they had to guess, and designs out what fought them
([what the games asked for](#what-the-dogfood-games-asked-for)).

## Purpose and scope

Thrum's engine modules are built and proved, and each has its own public API: a frame host, a world lane, a
registered simulation, sprite rows, cooked rows, an interface tree, a session, a link. A game written against
them today is mostly plumbing. The crowd view, a sprite scene with no rules of its own, is about 1,100 lines, and
the reference crowd's one rule is about 800. Nothing in either reads like a game.

This module is the front door: one crate, `thrum`, that a game depends on and nothing else. A game maker writes
what the game *is* (its things, its rules, what the player sees) and the SDK turns that into the engine's
registrations, schedules, extraction and hosting. It adds no second engine underneath: every kind of state keeps
the owner it has ([owners](#state-and-owners)).

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
4. 2D and 3D looks under one vocabulary: sprites, shapes, tile layers and sprite animation; cooked models with
   materials and textures whether placed or moving, sky, Sun, ground and lights.
5. Typed content with stable IDs, a typed asset catalog, and saves into and out of a running game.
6. Settings on the tuning panel; the profiler; the interface.
7. Multiplayer as an opt-in that changes no rule, for 2D games.
8. Tests with no window, recorded runs, pictures from a command, and errors with stable codes.
9. Reference documentation with an example on every public item, guides, templates and a new-game command.
10. The showcase moved onto the SDK, after the V1 exit.

**Not in the first release**, by the owner's scope (O4) and the engine as built. For each line a game hits, the
API's shape is [designed and reserved](#reserved-shapes), and whether it joins the first release is
[a question for the owner](#scope-questions-for-the-owner).

- Rigid-body physics, skeletal animation and audio (O4).
- 3D collision queries, a 3D character move and multiplayer for 3D games.
- Gamepads, text fields and scrolling lists, which `GAME-UI` does not build.
- Predicted projectiles, predicted contact between players, and hits rewound to what the shooter saw.
- A visual editor (D2), a scene file format, and hot reload of code. A changed rule rebuilds and restarts.
- Water, clouds and weather. `WATERS` and `CLOUDS` are still building. The words `scene.water(..)` and
  `scene.weather(..)` are kept free for them.
- A crates.io release. A game reaches `thrum` by a path to an engine checkout, as
  [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) settled.

## What a game maker must see

| Cue | Where it must hold | Bar |
|---|---|---|
| K1 A first game is one file and one command, and its first page ends with a character you steer | The 2D and the 3D starter, on the Mac and in Chrome | Godot's "Your first 2D game" and macroquad's first window: nothing to configure before something moves |
| K2 The code reads as the game | Every worked example and template: a scene function reads top to bottom as a description of play, and no line names a device, a frame host, a page, a permit or a schema number | Bevy's `breakout` example, with less ceremony |
| K3 One structure | 2D, 3D, alone and online: the same `Game`, scenes, rules and frame functions. Turning multiplayer on changes no rule | Unreal's gameplay framework, where one set of rules serves a listen server and a client |
| K4 Defaults look finished | A 3D scene that asks for `Sky::clear()` and nothing else gets the physical sky and Sun, shadows, exposure, tone mapping, reconstruction, EDR where the display has it and the governor. A 2D scene gets crisp sprites, drawn in order | The showcase's own look, with no setting touched |
| K5 Mistakes explain themselves | At compile time, at startup and while running, across [the listed mistakes](#16-mistakes-that-explain-themselves), each with a stable code | Rust's own compiler messages: the object, the cause, the next step |
| K6 Every public item is documented with an example that compiles | All of `thrum` | The Rust standard library's reference |
| K7 The docs are enough | A fresh agent builds a small 2D game, a small 3D game and a networked game from the guides alone, then makes a change the guides did not show | The first hour with Godot's or Bevy's getting-started guides |
| K8 The speed is kept | The reference crowd's behaviours at 10,000 bodies, written as SDK rules, beside the hand-written reference; a game links, compiles and prepares nothing it does not use | The engine's own figures, as tripwires |
| K9 A game is proved without a window | Rules stepped, input scripted, entities found and state read in an ordinary `#[test]`; a recorded run replayed in a test; one command writes a picture | An agent can prove a rule, keep a level beatable and look at a frame |
| K10 Nothing is guessed | Every call a worked example makes has a named type and a signature in [the mechanisms](#mechanisms) | The dogfood reports' lists of guesses, each closed |

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
   joining, a save restored. That is why a game can be replayed, tested and played online without being
   rewritten.
4. **Everything happens in the order it is written.** A scene function reads top to bottom as one tick, and
   what a line did is what the next line sees.

## Worked examples

These are real games against the proposed API. They are the shape the owner approves:
[what a builder may change](#what-a-builder-may-change) says how far a builder may depart from them. Every call
they make is defined in [the mechanisms](#mechanisms). Example G alone is written against
[reserved shapes](#reserved-shapes) that the first release does not build, and says where.

### A. A first 2D game

Rockfall: steer a ship under falling rocks. The first page of the guide ends with this whole program. The
ship already answers the keys and a drag on a phone, and a second player gets a ship of their own beside it.

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Rockfall").scene(play).run()
}

/// What a player can do. The bindings are the defaults; a player can change them.
#[derive(Actions)]
struct Pilot {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::drag_x())]
    steer: Axis,
}

fn play(scene: &mut Scene) {
    scene.camera(Camera2d::fit(24.0, 18.0)); // this rectangle is always in view, whatever the window
    scene.players().spawn(ship).control(steer_ship);
}

/// One ship for each player, side by side.
fn ship(joined: Joined) -> impl Bundle {
    let x = joined.player.index() as i32 * 2;
    (Shape2d::triangle(1.0), Transform2d::at(x, -7), Collider2d::circle(0.5))
}

/// Runs once a tick for each player's ship, with that player's input.
fn steer_ship(at: &mut Transform2d, pilot: &Pilot, tick: Tick) {
    at.position.x = (at.position.x + pilot.steer * 9.0 * tick.dt()).clamp(-11.5, 11.5);
}
```

The second page adds the rocks, the score and the end. This is the whole game, still one file:

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
([scope question 1](#scope-questions-for-the-owner)). `art` is [the typed asset catalog](#12-content-assets-and-saves), which `cargo play` writes from `assets/`.

### C. A slice of the V1 showcase

The exit scene: the cooked valley, 10,000 entities of which 1,500 drift, six stops and the hour of the day.

```rust
use thrum::prelude::*;

#[derive(Actions)]
struct Guide {
    #[bind(Key::Tab, Touch::tap())]
    next_stop: Button,
}

/// Registered with `Game::local_settings::<Day>()`: the hour lights this screen and no rule reads it.
#[derive(Settings)]
struct Day {
    #[slider(4.0..=22.0, default = 9.0, step = 0.05, unit = " h", label = "Hour")]
    hour: f32,
}

#[derive(Component)]
struct Drifts {
    around: Vec3,
    radius: Num,
    per_second: Angle,
    starts_at: Angle, // each mover's own, so equal speeds do not move in step
}

#[derive(Resource, Default)]
struct Tour {
    stop: u32,
}

fn valley(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.place(Model::new(art::meshes::VALLEY).material(Material::textured(art::materials::GROUND)));
    // `.at(..)`, `.turned(..)` and `.scaled(..)` would stand it somewhere other than the origin
    scene.camera(STOPS[0].camera());
    scene.resource(Tour::default());
    scene.on_enter(populate);
    scene.rules((drift, tour));
    scene.each_frame((sun_follows_hour, fly_to_stop));
}

fn populate(mut entities: Entities, mut random: Random) {
    for index in 0..10_000 {
        let home = layout::home(index);
        let prop = layout::PROPS[index % layout::PROPS.len()].at_point(home);
        if index < 1_500 {
            entities.spawn(prop.with(Drifts {
                around: home,
                radius: random.between(2.0, 9.0),
                per_second: Angle::turns(random.between(0.02, 0.08)),
                starts_at: random.angle(),
            }));
        } else {
            entities.spawn(prop);
        }
    }
}

fn drift(mut movers: Query<(&Drifts, &mut Transform3d)>, tick: Tick) {
    movers.par_each(|(drifts, mut at)| {
        let turn = drifts.starts_at + drifts.per_second * tick.seconds();
        at.position = drifts.around + turn.direction_xz() * drifts.radius;
    });
}

fn tour(guides: Inputs<Guide>, mut tour: ResMut<Tour>) {
    if guides.any(|guide| guide.next_stop.pressed) {
        tour.stop = (tour.stop + 1) % STOPS.len() as u32;
    }
}

fn sun_follows_hour(mut sky: Screen<Sky>, day: Screen<Day>) {
    sky.sun = SunAngle::at_hour(day.hour, VALLEY_LATITUDE);
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
Levels are content: adding one is a file. `src/rules.rs` is the game, and it is [checked](#9-numbers) to give
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
pub const HERO_SIZE: Vec2 = Vec2::lit(0.8, 1.4);

prefabs! {
    pub const HERO: Prefab = "hero";
    pub const COIN: Prefab = "coin";
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

    let moved = tiles.move_box(at.position, HERO_SIZE, hero.velocity * tick.dt()).on(TERRAIN);
    at.position = moved.position;
    hero.velocity = moved.slide(hero.velocity); // what a wall, a floor or a ceiling stopped is gone
    hero.on_floor = moved.on_floor();
    if hero.on_floor {
        hero.coyote.start(tick, seconds(0.1)); // six ticks
    }
}

fn collect_coins(hero: Single<Entity, With<Hero>>, space: Space2d, mut entities: Entities, mut run: ResMut<Run>) {
    for coin in space.touching(*hero).on(COINS) {
        if entities.despawn(coin.entity) {
            run.coins += 1;
        }
    }
}

fn touch_spikes(hero: Single<&Transform2d, With<Hero>>, tiles: Level2d, mut stage: PhaseMut<Stage>) {
    if tiles.touching_box(hero.position, HERO_SIZE).on(SPIKES).any() {
        // A tick makes one transition, and the first request stands. This rule is written before
        // `reach_the_end`, so a hero who touches spikes in the tick they reach the end dies.
        stage.go_to(Stage::Dying);
    }
}

fn touch_flags(mut hero: Single<(&mut Hero, &Transform2d)>, tiles: Level2d) {
    let (hero, at) = &mut *hero;
    if let Some(flag) = tiles.marks(FLAG).find(|flag| flag.distance(at.position) < 1.0) {
        hero.checkpoint = flag;
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

fn back_to_checkpoint(mut hero: Single<(&mut Hero, &mut Transform2d)>) {
    let (hero, at) = &mut *hero;
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
    scene.each_frame((animate_hero, dust_on_jump, pause_menu));
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
        pause.toggle(); // while paused no rule runs and the tick does not move, so every timer waits
    }
    if !pause.is_on() {
        return;
    }
    ui.modal("Paused", |ui| {
        if ui.button("Resume").clicked() {
            pause.off();
        }
        if ui.button("Restart level").clicked() {
            request.scene(Level { def: level.def });
            pause.off();
        }
    });
}
```

```rust
// src/main.rs
mod art; // written by `cargo play` from `assets/`
mod rules;
mod screens;

use thrum::prelude::*;

fn main() -> thrum::Result {
    game().run()
}

pub fn game() -> Game {
    Game::new("Skylark")
        .settings::<rules::Feel>()
        .content::<rules::LevelDef>("levels")
        .saved::<rules::Records>(Slot::new("records")) // best times: read at startup, written when they change
        .add(rules::prefabs)
        .scene(screens::title)
        .scene((rules::level, screens::level))
        .scene((rules::cleared, screens::cleared))
}
```

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

fn move_bodies(mut bodies: Query<(&Velocity, &mut Transform2d)>, tick: Tick) {
    bodies.par_each(|(velocity, mut at)| {
        at.position = (at.position + velocity.0 * tick.dt()).clamp_to(ARENA);
    });
}

fn enemies_hurt_heroes(mut heroes: Query<(Entity, &mut Health), With<Hero>>, space: Space2d) {
    for (hero, mut health) in &mut heroes {
        health.hurt(space.touching(hero).on(ENEMIES).count() as i32);
    }
}

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
([how](#15-multiplayer-as-an-opt-in)). A game that wants its own screens uses
[`Session`](#15-multiplayer-as-an-opt-in).

What made this possible is in the rules already: each player's hero is spawned by `scene.players()`, its
movement is a control function a match can run ahead, the upgrade is an offer, and the numbers agree on every
device. A game played alone is the same game with one player.

- *Play alone*: the arena's first tick has one player, and their hero stands in the line.
- Three friends join in the lobby and the host presses *Start*: four heroes stand in a row, on every machine.
- A friend joins while a wave is running: they are sent the world as it stands, and their hero is spawned at
  the tick they enter. One hero for each player in every case.

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
so the host, the friend's machine and a replay agree ([seats](#6-players-actions-and-control)).

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

### G. A third-person 3D slice, on reserved shapes

Hollow Vale: walk a valley, carry a relic on your back, stay out of a warden's sight. **This example is written
against [reserved shapes](#reserved-shapes)**: `Space3d`, `Level3d` and `move_capsule` wait on
[scope question 1](#scope-questions-for-the-owner), and the gamepad bindings on question 2. Everything else in
it is first-release API. It is here so the owner can judge the 3D game the design leads to.

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
    scene.on_enter(begin_or_resume);
    scene.players().spawn(PILGRIM).control(walk);
    scene.rules((take_or_set_down, wardens_look, wardens_move, wardens_catch));
}

/// "Forward" is away from the camera, and the camera's angle arrives as input.
fn walk(at: &mut Transform3d, body: &mut Body, input: &Walk, tuning: Res<Tuning>, ground: Level3d, tick: Tick) {
    let forward = input.look.direction_xz();
    let wish = (forward * input.go.y + forward.right() * input.go.x) * tuning.walk_speed;
    body.velocity = vec3(wish.x, body.velocity.y - tuning.gravity * tick.dt(), wish.z);

    let moved = ground
        .move_capsule(at.position, PILGRIM_SHAPE, body.velocity * tick.dt())
        .max_slope(Angle::degrees(40.0))
        .step_up(0.3);
    at.position = moved.position;
    body.velocity = moved.slide(body.velocity);
    if wish != Vec3::ZERO {
        at.face(wish);
    }
}

fn take_or_set_down(
    pilgrim: Single<(Entity, &Player, &Transform3d)>,
    inputs: Inputs<Walk>,
    space: Space3d,
    mut entities: Entities,
) {
    let (me, player, at) = *pilgrim;
    if !inputs.of(*player).take.pressed {
        return;
    }
    let back = entities.part(me, BACK);
    if let Some(carried) = entities.children(back).next() {
        entities.detach(carried).keep_world(); // it stays where it is, on the ground
    } else if let Some(relic) = space.within(at.position, 1.8).on(RELICS).first() {
        entities.attach(relic.entity).to(back).keep_local(Transform3d::ORIGIN);
    }
}

fn wardens_look(
    mut wardens: Query<(&Transform3d, &mut Mind), With<Warden>>,
    pilgrim: Single<&Transform3d, With<Player>>,
    space: Space3d,
    tuning: Res<Tuning>,
    tick: Tick,
) {
    for (at, mut mind) in &mut wardens {
        let eye = at.position + Vec3::UP * 1.6;
        let chest = pilgrim.position + Vec3::UP * 1.0;
        let in_sight = eye.distance(chest) < tuning.sight && space.ray(eye, chest).on(SOLID).first_hit().is_none();
        if in_sight {
            *mind = Mind::Chasing { last_seen: pilgrim.position, gives_up: Timer::after(tick, seconds(4)) };
        }
    }
}

pub fn vale_screen(scene: &mut Scene<Vale>) {
    scene.sky(Sky::clear());
    scene.ground(Ground::heights(art::terrain::VALE)); // one heightfield: the engine draws it and rules stand on it
    scene.camera(
        Camera3d::orbiting_my_character()
            .yaw(Walk::LOOK)
            .distance(6.5)
            .height(1.5)
            .damping(seconds(0.12))
            .avoiding(SOLID),
    );
    scene.each_frame((sun_follows_the_hour, journal, save_at_the_shrine));
}
```

A warden's state machine is an enum with timers in it (`Mind`), which rewinds and saves with the world. The
journey is saved at the shrine and resumed from the title screen:

```rust
#[derive(Save)]
#[save(version = 2, from = ValeSaveV1)]
pub struct ValeSave {
    pub hour: Num,
    pub placed: Vec<Id<RelicDef>>,
    pub carrying: Option<Id<RelicDef>>,
}

/// Reads the world at a committed tick and returns what to keep. It runs when a save is asked for.
pub fn capture(day: Res<Day>, relics: Query<(&Relic, &Placed)>, family: Family) -> ValeSave { /* .. */ }

/// The valley's first function. A save chosen on the title screen arrives here, as input to this tick.
fn begin_or_resume(restored: Restored<ValeSave>, mut entities: Entities, mut day: ResMut<Day>) -> thrum::Result {
    match restored.get() {
        Some(save) => restore(save, &mut entities, &mut day), // an error here refuses the tick: never half-restored
        None => new_journey(&mut entities, &mut day),
    }
}

// In the game: `.save::<ValeSave>(capture)`. On the title screen:
fn title(mut ui: Ui, saves: Saves, mut request: Request) {
    ui.center(|ui| {
        ui.title("Hollow Vale");
        if let Loaded::Ready(_) = saves.peek::<ValeSave>(Slot::AUTO) {
            if ui.button("Continue").clicked() {
                request.scene(Vale).restoring::<ValeSave>(Slot::AUTO);
            }
        }
        if ui.button("New journey").clicked() {
            request.scene(Vale);
        }
    });
}
```

## What the dogfood games asked for

Where each thing the three games had to guess, or had to work round, is settled.

| What they met | Settled as | Where |
|---|---|---|
| How a rule changes a phase and how anything reads it; whether scene rules run before phase rules; which of two changes in one tick wins; a phase that lasts a fixed time | `Phase<P>` and `PhaseMut<P>`; one written order for the whole scene; a tick makes one transition, the first request for a phase or a scene stands, and a later one says so; `.lasts(..).then(..)` | [4](#4-scenes-phases-and-time) |
| Whether going to the running scene restarts it; a scene that takes an argument; naming a scene from the rules | `scenes.restart()`; a scene is a type whose fields are its arguments | [4](#4-scenes-phases-and-time) |
| How a prefab is declared, what type its name has, how one spawn changes one field, a prefab that needs a value from whoever spawns it | `Prefab<Needs>` constants declared in a `prefabs!` block, `Game::prefab`, `.variant()`, `.with(..)`, `.tweak(..)` | [5](#5-prefabs-children-and-parts) |
| Attaching one thing to another while the game runs; reaching a child; a child's place in the world; taking a thing out of play without despawning it | `entities.attach(..)`, `detach`, `part`, `children`, `world3d`; `disable` and `enable` | [5](#5-prefabs-children-and-parts) |
| Where `control` runs among the rules | Where it is written | [4](#4-scenes-phases-and-time), [6](#6-players-actions-and-control) |
| A controlled-character function that cannot read settings, the phase or the level, or raise an event; a factory that cannot place its character | It takes any rule parameter. A table says which a match can run ahead, and a game with multiplayer on must say `.unpredicted()` to take the others | [6](#6-players-actions-and-control) |
| `Controls::run` as a path to a field; local-only keys; the pointer; a camera angle that has to go through the tick; a pressed button that can be lost | Action maps read as plain fields; `LocalActions`, `MyInput`, `Pointer`; `Heading`; a press is never lost | [6](#6-players-actions-and-control) |
| Bots; who the host is; a table for each player; `PlayerJoined` meaning two things; what happens to an absent player's character | `PlayersMut::add_bot` and `BotInputs`; `Players`; `PerPlayer<T>`; one meaning; `.when_away(..)` | [6](#6-players-actions-and-control) |
| `Send` shadowing Rust's own trait; `Busy` in a menu; a refused command that tells nobody; a vote; opening an offer | `Outbox` and `Delivery`; `Offers::open`, `Offer::to_all()`, `tally` | [7](#7-commands-offers-and-requests) |
| A box that moves against the level and says what it hit; no way to make fixed collision; a tile map; a level chosen at run time | `Level2d::move_box` and `Moved2d`; `LevelMut2d::load`; `Tileset` and `Tilemap` | [8](#8-space-colliders-queries-the-level-and-moving-a-character) |
| Which of `x * 0.5`, `x > 0.0`, `vec2(x, 0.0)` and `x.max(0.0)` compile; a number in a `const`; which float types frame code sees | One page | [9](#9-numbers) |
| `Local<T>` reading as Bevy's; no frame-side "on enter"; choosing a sprite's clip in the world; a respawn drawn as a streak; a camera that orbits, stays inside the level or cuts | `Screen<T>`; `scene.on_show`; `Shown<L>`; `teleport`; camera rigs | [10](#10-frame-functions-and-the-frame) |
| Pause; deadlines that run out behind the pause menu; the key that opened a modal cannot close it | While paused no rule runs and the game's tick does not move; local actions are read under a modal | [4](#4-scenes-phases-and-time), [14](#14-the-interface) |
| Typed content end to end: the derive, the format, registration, validation, IDs, tests | `#[derive(Content)]`, `Game::content`, `Content<T>`, `Id<T>` | [12](#12-content-assets-and-saves) |
| Who writes the asset catalog | `cargo play`, into `src/art.rs` | [12](#12-content-assets-and-saves) |
| Saves into and out of a running game; a first launch; a version; Chrome's storage | A saved resource; `Game::save`, `Saving`, `Saves`, `Loaded`, `Restored<T>` | [12](#12-content-assets-and-saves) |
| Finding an entity the game spawned; setting an axis; reading the phase; content in a test; clicking a card; a second machine; a recorded run | `TestGame`, `TestScreen`, `TestMatch`, `play_recording` | [17](#17-tests-without-a-window-recordings-and-pictures) |
| A filter that exists only for the access check; fourteen loops over one hero | The check is made on the entities that exist; `Single` | [3](#3-rules-and-the-tick) |
| No row, side place, timed message, fade, disabled card, key prompt or label at a point in the world | In the interface's vocabulary | [14](#14-the-interface) |
| 3D collision and a character move, gamepads, audio, predicted projectiles, hits rewound to what the shooter saw | Shapes reserved; each a question for the owner | [Reserved shapes](#reserved-shapes) |

## Research and selection

Each engine was read for how a game's code is shaped, not for its features. The owner's review of 26 November
compared the first design with seven of them line by line, and its findings are the decisions above.

| Engine | Shape of game code | What Thrum takes | What Thrum leaves |
|---|---|---|---|
| [Bevy 0.20](https://bevy.org/news/bevy-0-20/) | Entities and plain-data components; systems are functions whose parameters say what they read and write; states with enter and exit; `Single`; relationships | Components, queries and functions with typed parameters, and its names for them (`Query`, `Res`, `With`, `Single`), because it is the Rust idiom agents and developers already know; state entry and exit, as phases | The schedule graph, and deferred edits whose moment of landing a reader has to look up. The `App` and plugin layer. `Local<T>`, whose meaning here would be the opposite |
| [Godot 4.6](https://godotengine.org/releases/4.6/) | A tree of nodes with scripts; a scene is a reusable tree; `CharacterBody2D.move_and_slide()`; `TileMapLayer`; step-by-step first-game guides that steer a player first | A character move that reports its contacts; a tile map with collision; children and named parts; a first page that ends with a character you steer | State held inside objects with methods: it cannot be digested, rewound or checked equal between two machines |
| Unity | GameObjects, prefabs and prefab variants, values tuned in the Inspector; Entities and Netcode for bulk work and prediction | Prefabs with variants and overrides; settings tuned live | A method call for each object each frame; ghosts and prediction declared field by field |
| Unreal | A gameplay framework that keeps apart the rules, the shared state, a player's controller and what one player sees; a controller that possesses a pawn, a person's or an AI's; control rotation owned by the client | That separation; a player who outlives their character; a bot as a player; a heading the screen owns and sends as input | Its size, and replication declared property by property |
| Photon Fusion 2 | A predicted kinematic controller that collides with the level; `FixedUpdateNetwork` and `Render` named at the call site | A controlled character that runs ahead against fixed collision; the two clocks visible in the type of every function | Lag-compensated hit boxes and predicted spawns, which are [reserved](#reserved-shapes) |
| macroquad | One `async fn main` with a loop that draws | The first minute: one file, one command, shapes with no asset | Immediate-mode state, which cannot be replayed or shared |
| [lightyear](https://github.com/cBournhonesque/lightyear) and bevy_ggrs | Deterministic replication: only inputs travel | The constraint it puts on game code, made the only way to write a rule | Bolting it on afterwards |

**The choices, and what would change each.**

1. **Plain-data components and rule functions, not objects with methods.** Thrum's simulation is a fixed tick
   over registered state that is rewound, replayed, digested and shared by causes. Only state the engine can see
   whole can do that. Methods on plain data, such as `health.hurt(2)`, are welcome. *It would change* if
   multiplayer and replay were dropped from the SDK's aims.
2. **One written order.** A scene function is one list, top to bottom: enter functions, players' control
   functions, rules and phases, each where it is written. Considered: Bevy's graph, which finds parallelism
   between systems and is the largest source of surprises in Bevy code. Thrum's parallelism is inside a rule,
   where a crowd's cost is. *It would change* if a profile showed whole rules that could run side by side
   dominating a tick.
3. **What a rule did is what the next rule sees** (R2). Component writes, spawns, despawns and attachments all
   land when the rule that made them finishes. Considered: the engine's own shape, where structural edits land
   when the tick ends, which the first design passed on to game makers. It made a stomp, a pick-up and a wave's
   last clean-up each a tick late in ways an author finds only in play.
4. **Two kinds of function, told apart by type.** A rule cannot take `Ui` or `Screen<T>`, and a frame function
   cannot take `ResMut<T>` or `Entities`; each mistake is a compile error that says why.
5. **Looks are components, and how one screen dresses them is not.** A sprite is on the entity it draws. Which
   clip plays, a flash and a decorative turn are `Shown<L>`, set by a frame function (R6).
6. **One vocabulary for 2D and 3D, with the dimension in the name where the two differ**: `Transform2d` and
   `Transform3d`, `Space2d` and `Space3d`.
7. **Portable numbers in rules, floats on the screen** (D1, R4). Considered: `f32` everywhere for games played
   alone. That makes "turn on multiplayer" a rewrite and gives up replay across devices.
8. **A scene is a type, a prefab and a layer are typed constants, and an action is a field.** Each is found by
   an editor's completion, checked by the compiler and named the same in a save, a recording and an error.
   Considered: a scene named by its function, as the first design had. A rule could then not name a scene
   without reaching outside the checked rules, and a scene could not take an argument the compiler checks.
9. **The interface is described each frame and kept by the engine.** This is where current interface kits have
   settled (React, SwiftUI, Compose).
10. **One package.** A game is one Cargo package whose rules are a module. Considered: a rule crate for every
    game that can be played online, as the first design had. It asked a first-time maker to learn a workspace
    before a wall.

## Mechanisms

### 1. One crate, one package

A game is one Cargo package that depends on `thrum` and names no other engine crate.
`use thrum::prelude::*` brings the fifteen words and the maths. No public item of `thrum` names a type of
`wgpu`, `winit` or another engine crate.

- **Everything a game writes compiles with no window.** The whole description of a game (scenes, rules, frame
  functions, the interface, cameras, looks) is plain data and functions, and builds with no renderer, window,
  GPU, network or platform crate. That is what `thrum` is with its default features off, and what a test, a
  headless host and the rules check use. The default features add `run()` and what it needs. The `multiplayer`
  feature, off by default, is the only way `thrum-network` and `thrum-transport` enter a game's closure.
- **`pub fn game() -> Game`** is the one value `main` runs and a test steps. The templates are written so.
- **A game's rules are its `rules` module**: `src/rules.rs`, or `src/rules/` and everything under it. The
  module holds the game's components, scenes' types, action maps, prefab names and rules. It is what
  [the check](#9-numbers) holds to the portable profile, and it is checked by being built **on its own**,
  beside the generated asset catalog and against `thrum` alone. So a rule cannot call a helper elsewhere in
  the game, and a helper inside the module is checked with the rules that reach it.
- **Every function that makes or changes the world is defined in that module**, which the SDK checks at
  startup from each function's path, in `test()`, in `run()` and in `cargo play --check` alike. Those
  functions are the ones a game registers by these routes and no others: a rule in `scene.rules` or a phase's
  `rules`; a control function in `.control(..)`; **a spawn function in `scene.players().spawn(..)`**, which
  makes a player's character; an enter or an exit function of a scene or a phase; a condition in `.when(..)`;
  and the function a lobby is given in `.starts_with(..)`. One defined elsewhere, such as a spawn function in
  the screens' file that places a character from this machine's state, is refused before the first tick with
  the function and the module named. A value the description gives the world directly (a resource's starting
  value, a prefab's definition, a shared setting's default) is not code that runs at a tick: it is part of
  [the rule-set identity](#15-multiplayer-as-an-opt-in), so two machines that differ in one are refused
  before a match starts.
- A one-file game, such as example A, has no `rules` module. It runs, and `cargo play` says once that its rules
  are not checked and how to move them. `Game::multiplayer` refuses to start such a game. The templates have
  the module from the first line.
- A game that outgrows one package may move its rules module to a package of its own, whose manifest says
  `[package.metadata.thrum] rules = true`. Nothing else changes: it is the same check on the same code.

**Which packages are held to `thrum` alone.** The repository's guard holds it, from WP8
([ruling 5](../ARCHITECTURE.md#seam-rulings-for-game-sdk)): every package in a games repository, under the
engine's `templates/` and under the engine's `examples/` names no engine crate but `thrum`, through any kind
of dependency. Not covered are the packages the registry marks as written on the engine's own API, each with
its reason: the proving ground and its iPhone app; the engine's own proofs under `examples/`; the old starter,
stones and lighthouse until WP10 removes them; and the showcase in the games repository until WP9 rewrites it.

### 2. The world: entities, components, resources

The world is `thrum-core`'s registered simulation. The SDK adds no storage.

**A component** is `Copy` plain data. `#[derive(Component)]` writes its registration, its stable schema
identity (from its name and fields, never from a Rust type ID, as C01 requires), its encoding and its
finiteness check. What a component, a resource, an event, a command and a scene's fields may hold:

| Kind | Types |
|---|---|
| Numbers | `bool`, the fixed-width integers, `Num`, `Vec2`, `Vec3`, `Angle`, `Rot3`, `Seconds` |
| Time | `TickNumber`, `Timer`, `Cooldown` |
| Names | `Entity`, `Player`, `Id<T>`, `Prefab<N>`, `Layer`, `Mark`, `Part` |
| Text | `Text<N>`: up to `N` bytes of UTF-8, such as a player's name |
| Shapes | `Option<T>`, a fixed array `[T; N]` and a tuple of these; a struct or an enum that derives `Data` |
| In a resource only | `List<T, N>`, a list of at most `N`; `PerPlayer<T>`, one `T` for each player |

A `Vec`, a `String`, a reference, a raw float and `usize` are each a compile error that names what to use.
Long text and tables belong in [content](#12-content-assets-and-saves), which rules read and cannot change.

**A resource** is the same for data the world has one of. It has one of two lifetimes, said where it is
registered:

- `scene.resource(value)` is a **scene resource**. It is set to that value each time the scene is entered.
- `Game::resource(value)` is a **game resource**. It keeps its value across scenes until a rule changes it.
- The running scene's own value, such as `Level { def }`, is read as `Res<Level>` and cannot be written.

**An entity** is a handle, valid until the entity is despawned. Entities belong to the scene they were spawned
in and leave with it. The world holds a fixed number, because the engine reserves its storage once:
`Game::max_entities(n)` sets it, and a spawn past it is refused with a message.

**What the world holds is fixed before the first tick.** The SDK makes the list of every component and resource
from what the game's description shows it at startup:

1. the signatures of every function the game's scenes list;
2. every prefab's bundle, and the return type of every spawn function given to `scene.players()`;
3. the SDK's own components;
4. `Game::component::<T>()`, for a component none of those shows. It is rare: it takes a rule that spawns a
   bare tuple holding a component that no prefab and no signature names.

Spawning a component the list lacks is refused with a message that names the rule, the component and the
prefab or line to add.

### 3. Rules, and the tick

A rule is a function. Its parameters say what it reads and writes.

| Parameter | What it gives |
|---|---|
| `Query<D, F>` | The entities that have `D`, filtered by `With<T>` and `Without<T>`, visited in the order they were spawned. `&T` reads, `&mut T` writes, `Entity` names. `get`, `get_mut`, `of_player`, `count`, `is_empty`, `iter`, `par_each` |
| `Single<D, F>` | The one entity that matches, used as its item. None or two refuses the tick and says which. `Option<Single<..>>` is none or one |
| `Res<T>`, `ResMut<T>` | A resource |
| `Entities` | `spawn`, `despawn`, `insert` and `remove` a component, `attach` and `detach`, `disable` and `enable`, and the tree: `parent`, `children`, `part`, `world2d`, `world3d`, `alive` |
| `Family` | The tree alone, read-only, for a rule that changes no structure |
| `Inputs<A>` | Each player's action map `A` for this tick: `of(player) -> &A`, `iter()`, `any(..)` |
| `BotInputs<A>` | `set(player, value)` for a bot's input, and `is_bot(player)` |
| `Players` | Who is in the game, to look at: `iter`, `count`, `host`, `name`, `status`, `character` |
| `PlayersMut` | Everything `Players` has, and what changes the roster: `add_bot`, `remove_bot`, `respawn`, `possess` |
| `Commands<T>` | The commands of type `T` admitted at this tick, with who sent each, in the order each player sent them |
| `Offers<T>` | `open`, `accept`, `tally`, `close`, and for a bot `pending` and `answer_for` |
| `Events<T>`, `Emit<T>` | Events of this tick: the game's own, and `PlayerJoined`, `PlayerLeft`, `PlayerAway`, `PlayerBack` |
| `Tick` | The tick's number, its length `dt()`, `seconds()`, `in_scene()`, `in_phase()`, and `every`, `after`, `reached` |
| `Random` | Draws keyed by the game's seed, the tick and the rule that asks: `between`, `chance`, `angle`, `direction`, `pick`, and `for_entity(e)` inside `par_each` |
| `Space2d` | The colliders, where they are now |
| `Level2d`, `LevelMut2d` | The scene's fixed collision, tiles and marks; and loading them |
| `Scenes` | `go_to(scene)` and `restart()` |
| `Phase<P>`, `PhaseMut<P>` | The scene's phase: `get`, `is`, `time`; and `go_to` |
| `Content<T>` | The game's records of type `T`: `get(id)`, `find(name)`, `iter()` |
| `Saving`, `Restored<T>` | Asking for a save; a save this tick brought in |

**A read-only parameter** is one that cannot change the world: a `Query` or a `Single` with no `&mut`,
`Res<T>`, `Family`, `Inputs<A>`, `Players`, `Events<T>`, `Tick`, `Random`, `Space2d`, `Level2d`, `Phase<P>`,
`Content<T>` and `Restored<T>`. A spawn function, a condition and a capture function take read-only
parameters and nothing else, and a frame function takes the read-only ones [its table](#10-frame-functions-and-the-frame)
lists. That is a bound on the parameter's type, so `mut players: PlayersMut` in any of them does not compile:
what a function may do is fixed by the type it asks for and never by how it binds the value.

**Written order (R2).** A scene function is one list. Each tick, the SDK walks it top to bottom and runs each
entry once: a `scene.rules(..)` block's rules in turn, a `scene.players()..control(f)` line for each player's
character in seat order, and a `scene.phase(p)` block's rules when `p` is the scene's phase.

**What a rule sees.**

- **While a rule runs, the world stands still around it.** Its own writes to components and resources are in
  place at once. Its spawns, despawns, inserted and removed components, attachments, and disabling are held
  until it finishes, so nothing appears or vanishes under a loop.
- **When a rule finishes, everything it did has happened.** The next rule's queries visit an entity spawned
  by the last one and do not visit one it despawned. Space queries answer from where things are now.
- `entities.despawn(e)` returns whether this call is the one that despawns it: `false` when it is already
  gone, or when an earlier call in the same rule asked first. So two heroes on one gem count it once.
- An event a rule sends is read by later rules of the tick, by frame functions, and by no rule of the next
  tick.
- The whole tick still commits or rolls back as one. A rule that fails leaves the world as it was before the
  tick, not as it was before the rule.

The engine resolves structural edits when a tick ends today. The SDK asks core for a barrier after each
registered system, inside the tick's one transaction
([ruling 2](../ARCHITECTURE.md#seam-rulings-for-game-sdk)): the staged batch is applied and the next system
sees the result, and the undo covers the tick. WP1's first item proves it before anything is built on it.

**What a rule may touch comes from its signature alone.** A signature names components in **accesses**: each
`&T` and `&mut T` inside a `Query` or a `Single`, and each `&T` and `&mut T` a control function takes for its
character. **Two accesses of one function are in conflict when they name the same component, at least one of
them writes it, and one entity is reached by both.** A writer beside a writer and a writer beside a reader are
the same conflict: either would hand the function two Rust references to one value. It holds between two
parameters and inside one.

- **Inside one parameter, both accesses always reach the same entity, so the signature alone decides.**
  `Query<(&mut Health, &Health)>`, a `Single` written the same way, and a control function that takes both
  `&mut Transform2d` and `&Transform2d` for its character are each refused at startup, whatever entities
  exist, with the function, the parameter and the component named. The fix is to drop the reader: a writer
  reads too.
- **Between two parameters, the SDK decides it from the sets of components that exist**, and not from the
  filters alone. At startup it tests every prefab's set against every rule. While the game runs, a set is new
  when an entity first has it, by a spawn of a bare tuple or by `insert` or `remove`: the SDK tests each new
  set when the rule that made it finishes, which is the only moment an entity's membership of a query changes.
  A set that two parameters of some rule conflict over refuses that tick and names the rule, the two
  parameters, the component and the entity. So no rule ever starts with an entity that two of its parameters
  could both reach.
- **A control function's own accesses reach one entity: the character its player has at that tick.** Its
  `Query` and `Single` parameters are tested against that entity and no other, so a query of other things
  stays usable beside them: `&mut Transform2d` for the character and `Query<&Transform2d, With<Enemy>>` for
  the bats need no filter while no character is an enemy. The test is made on the character that is: at
  startup, on what the line's `spawn` makes; when a rule changes the character's set of components; and when
  `PlayersMut::possess` gives the player another entity, tested when the rule that called it finishes. A
  character that a query of its own control function also reaches, over a component either writes, refuses
  that tick, possession unchanged, and names the function, the parameter, the component, the player and the
  entity.
- **Parameters that match no entity in common need no filter.** `Query<&mut Transform3d, With<Pilgrim>>`
  beside `Query<&Transform3d, With<Boulder>>` needs no `Without<Boulder>`, because nothing in the game is
  both. A `Without` still states it for a reader, and for free.
- **What a rule looks things up through reads the world as the rule found it.** `Space2d`, `Level2d`, `Family`
  and the tree's methods on `Entities` (`world2d`, `world3d`) answer from where things stood when the rule
  started, so they never read a value the rule is writing.

**A rule is a function of its parameters** (D1). The same world, input and tick give the same result on every
machine and in every replay. The SDK holds up its side by type: nothing a rule can be handed leads to a clock,
a file, the network, the window or one player's screen. The other side is the author's: **a rule reads nothing
but its parameters and changes nothing but through them.** [The check](#9-numbers) refuses the ordinary ways of
breaking that.

**Parallel work** is inside a rule: `query.par_each(..)` runs the closure over the engine's chunks. The
closure writes only the entity it is given and reads the rule's other parameters. The result is the same as
the serial loop, by construction.

**Cadence and conditions.** `rule.every(seconds(1))` runs a rule once each interval, counted from when its
phase, or its scene, was entered, first at the end of the first interval. `rules.when(condition)` runs a rule
or a group only in ticks where a read-only function returns true. `tick.every(..)` is for beats meant to be in
step across the whole world. A cadence that belongs to one thing is a `Cooldown` on that thing.

**Failure.** A rule returns nothing, or a `thrum::Result`. A returned error refuses the tick, on every target:
the engine rolls back what earlier rules of the tick did, and the game stops with the rule's name, the tick and
the error. In a test, `step()` returns that error and the game can be stepped again. A panic is a bug in the
game. On the Mac and the iPhone simulator, in `run()` and in a test, it is caught and reported as a returned
error is. On the browser target the program stops, because that target cannot unwind: before it does, the
SDK's panic hook puts the rule, the tick and the message on the page and in the console. So a rule that can
fail in play returns an error.

**The tick** is 60 Hz unless `Game::tick_rate` says otherwise. Speeds are written per second and multiplied by
`tick.dt()`.

### 4. Scenes, phases and time

**A scene is a type and a function.** The type derives `Scene`, and its fields are what the scene is entered
with. The function describes it: `fn level(scene: &mut Scene<Level>)`. `Scene` alone is `Scene<()>`, for a game
with one scene. `Game::scene` takes one function, or a tuple of functions that describe the same scene, which
is how a game keeps a scene's rules in `rules.rs` and its screen in `screens.rs`. `scene.add(part)` does the
same from inside a scene function.

```rust
impl Scenes {
    /// Leaves for `next` when this tick ends. False, with nothing changed, when this tick's one transition
    /// has already been asked for.
    pub fn go_to<S: SceneType>(&mut self, next: S) -> bool;
    /// Enters the running scene again, with the value it was entered with. False as `go_to` is.
    pub fn restart(&mut self) -> bool;
}
```

`#[derive(Scene)]` implements the trait `SceneType`, `#[derive(Phase)]` implements `PhaseType` and
`#[derive(Content)]` implements `ContentType`. A trait cannot share a name with the parameter types `Scene<S>`,
`Phase<P>` and `Content<T>`, so bounds are written on these, and a game maker writes the derive and never the
trait's name.

- The game starts in the first scene listed, entered with its `Default` value. A first scene whose type has no
  default is a compile error that says to list another first or derive one. `game.test_in(scene)` and
  `cargo play --scene <name>` start elsewhere.
- Which scene is running, and its value, are world state. A match's players are always in the same scene, and
  a late joiner is sent it.
- **A tick makes one transition at most**: a change of scene or a change of phase, and never both. The
  requests of a tick are taken in this order, and the first stands:
  1. one an enter or exit function made while the last tick's transition was landing;
  2. a `request.scene` admitted at this tick;
  3. the rules' own, in written order: `scenes.go_to`, `scenes.restart` and `PhaseMut::go_to` alike;
  4. a phase's `lasts`, which counts as asked after every rule.

  A later request returns `false` and changes nothing, whichever kind it or the standing one is, and its caller
  may ask again in a later tick. The rest of the tick's rules still run, in the scene and phase the tick began
  in, and the transition lands when the tick ends. So the rule written first decides, between two scenes, two
  phases, or a phase and a scene: in example D a hero who touches spikes in the tick they reach the end dies,
  and no clear is recorded.
- **A request made while a transition is landing waits a tick.** An enter or exit function runs as part of a
  landing, so a `go_to` it makes cannot join the transition under way: it is held, returns `true` if it is
  the first held, and is the next tick's first request. That tick's rules run once in the scene and phase
  just entered, their own requests return `false`, and the held one lands when it ends. A chain of scenes
  that each leave on entry therefore moves one a tick and cannot loop inside a tick.
- **A held request for a phase lives no longer than its scene.** A phase belongs to the scene whose function
  asks for it, so a `PhaseMut::go_to` made during a landing is held only when that scene is the one running
  once the landing ends: asked by an enter function of the scene being entered or of its first phase, or by
  a phase's enter or exit function while the scene stays. Asked by an exit function of a scene that is being left, or of that
  scene's phase as the scene is left, a `restart` included, it is **refused**: it returns `false`, nothing
  is held, and the next tick is not reserved, so that tick's first request is whichever rule asks first. A
  level whose exit function asks for `Stage::Dying` on the way to a title scene with no phases therefore
  changes nothing, and the title's own rules may ask for a scene in its first tick. A request for a *scene*
  is the game's and not one scene's, so `scenes.go_to` and `restart` from any enter or exit function are held
  as above, which is what example E's `next_wave` relies on.
- **Leaving:** the phase's exit functions, then the scene's, then every entity of the scene is despawned.
  **Entering:** scene resources are set, then the scene's `on_enter` functions and its `players()` lines run
  in the order written, then the first phase's enter functions. All of it happens in the tick that asked, so
  the next tick's first rule finds the scene whole. A character therefore exists before any `on_enter`
  written after its `players()` line, and after any written before it.
- A scene's kind (2D or 3D) follows from its camera. A scene needs no camera: without one it shows its
  background and its interface, which is what a menu and a result page are.
- **A frame function asks for a scene with `request.scene(..)`**, and it is honoured only where the running
  scene says `scene.accepts::<Target>()`. In a match the host's request is the one admitted, unless the scene
  says `.accepts_from_anyone::<Target>()`. Anything more particular is a command and a rule.

**A phase** is a value of an enum that derives `Phase`. `scene.phases(first)` gives the scene phases and names
the one it starts in.

```rust
scene.phase(Stage::Wave)          // a block in the scene's written order
    .on_enter(next_wave)          // once, each time the phase is entered
    .rules((steer, fire, hit))    // each tick while it is the phase, where this block is written
    .on_exit(clear_enemies)       // once, each time it is left
    .lasts(seconds(60))           // then leaves by itself
    .then(Stage::Intermission);

impl<P: PhaseType> Phase<P> {
    pub fn get(&self) -> P;
    pub fn is(&self, phase: P) -> bool;
    pub fn time(&self) -> Seconds;        // since it was entered
}
impl<P: PhaseType> PhaseMut<P> {          // everything `Phase<P>` has, and:
    pub fn go_to(&mut self, next: P) -> bool;
}
```

- A change of phase is one of [the tick's requests](#4-scenes-phases-and-time) and lands when the tick ends:
  the old phase's exit functions, then the new one's enter functions.
- Going to the phase the scene is already in leaves it and enters it again.
- The first phase's enter functions run when the scene is entered.
- Rules outside any phase block run in every phase.
- A frame function and a control function read `Phase<P>`. Only a rule takes `PhaseMut<P>`.

**Time belongs to the thing being timed** (R5). All of it is counted in ticks and is world state, so it
rewinds, replays and saves.

```rust
pub struct Timer;      // a deadline that may not be set
impl Timer {
    pub const IDLE: Timer;
    pub fn after(tick: Tick, wait: Seconds) -> Timer;
    pub fn start(&mut self, tick: Tick, wait: Seconds);
    pub fn clear(&mut self);
    pub fn running(&self, tick: Tick) -> bool;    // set, and not yet reached: a window
    pub fn finished(&self, tick: Tick) -> bool;   // set, and reached
    pub fn left(&self, tick: Tick) -> Seconds;
}

pub struct Cooldown;   // something that may happen again after a wait
impl Cooldown {
    pub const READY: Cooldown;
    pub fn ready(&self, tick: Tick) -> bool;
    /// If ready, starts the wait and returns true. A new `every` is taken from the next time on.
    pub fn take(&mut self, tick: Tick, every: Seconds) -> bool;
}
```

Coyote time, a jump buffer and a dash are `Timer` windows. A weapon's rate is a `Cooldown` on the hero that
holds it. A phase with a length uses `lasts`.

**Pause.** While a game is paused **no rule runs and the game's tick does not move**. `Tick` gives the same
number, so every timer, cadence and animation in the world waits, and no rule has to know. Frame functions
still run, so a menu works.

- A frame function takes `Pause`: `on()`, `off()`, `toggle()`, `is_on()`. `ui.modal(..).pausing()` holds the
  pause while the modal is open, for a signpost or an inventory that stops the world.
- **Alone, a pause takes effect at once.** The driver steps no tick, and when the pause ends it carries on
  from that moment: no tick is owed for the time paused, so nothing runs fast to catch up.
- **In a match the clock does not stop.** Whether anyone can pause is the game's choice:
  `Multiplayer::players(..).pause(Pausing::Nobody)`, the default, `Host` or `Anyone`. A pause and its end are
  input admitted at a tick from a player the game allows, and whether the match is paused, and by whom, is
  world state, so every screen can show it and a late joiner is sent it. The host goes on committing ticks
  and every machine goes on receiving them, so the end of a pause reaches the host as any input does, from
  whichever player sent it, and there is nothing to catch up afterwards. A paused tick admits the end of the
  pause and the roster's changes and nothing else: no rule, control function or timer runs, no player's
  buttons or axes are read, so the world's last count for each button stays where it was, and the game's
  own tick, which is what `Tick` gives and what every timer counts, stays where it was.
- **What happens during a pause waits for its end.** A player who joins, leaves, goes away or comes back
  changes the roster at once, and their event and their character come at the first tick after the pause. A
  press made while the presser's own screen shows the pause is never counted, so it is not read after it:
  [the player's machine keeps the two apart](#6-players-actions-and-control), since no count on the wire
  could. A command, an answer, a scene request or a shared
  setting's edit sent during a pause is held by its sender's SDK and sent when the pause ends. One already
  on its way that is admitted in a paused tick is not read, and its sender's SDK, which the session tells
  the tick each command was taken at, sends it again after the pause.
- `scene.unpausable()` is for a scene where it makes no sense.

### 5. Prefabs, children and parts

A prefab is a named thing to spawn (D2, R3). Its name is a typed constant, and the game gives each name a
definition once, before the first tick.

```rust
prefabs! {                                 // a game's names, declared together
    pub const BAT: Prefab = "bat";
    pub const SHOT: Prefab<(Shot, Velocity)> = "shot";   // needs these from whoever spawns it
}

impl Game {
    pub fn prefab<N>(self, name: Prefab<N>, definition: impl Into<PrefabDef>) -> Game;
}

BAT.variant()                              // a definition that starts as another prefab's
    .with(Health(8))                       // replaces one component, or adds it
    .tweak(|sprite: &mut Sprite| sprite.tint = Color::GOLD)   // changes one field
    .child(AURA.at_local(0.0, 0.2))        // a child, spawned and despawned with it
    .part(EYE, WARDEN_EYE.at_local(0.0, 1.6, 0.2));           // a child with a name

BAT.at(3.0, 0.0)                           // a spawn: the prefab at a place
BAT.at_point(point).turned(angle)
SHOT.with((shot, velocity)).at_point(muzzle)   // what `SHOT` needs, then where
ELITE_BAT.at(3.0, 0.0).with(Stunned::for_(tick, seconds(1)))  // one spawn's own extra component
ELITE_BAT.at(3.0, 0.0).tweak(|sprite: &mut Sprite| sprite.tint = team_colour)
```

- A definition is a bundle (a tuple of components) or `.variant()` of another prefab with changes. A variant
  follows its base: it is resolved when the game starts, so an edit to `BAT` reaches `ELITE_BAT`.
- `Prefab<N>` says in its type what a spawn must bring. `SHOT.at(..)` without its `Shot` and `Velocity` is a
  compile error, so no prefab holds a made-up owner or a zero that means "not set".
- **A name is made in a `prefabs!` block and nowhere else, and every block the game's code refers to is
  known to the SDK before play, whether or not the description names it.** A block adds itself to a list the
  built program carries, by registration at link time (the technique of Rust's `inventory` crate, which Bevy
  uses to register types with no line of game code), and each name holds a reference to its block, so a block
  any code spawns from is in the program and in the list. The SDK reads the list when the game starts, on
  every target, in `test()`, in `run()` and in `cargo play --check` alike. So a game that declares `BOSS` in
  a block of its own, spawns it only in a rule's body at wave ten and forgets its definition is refused
  before the first tick, though no other line names that block. Nothing is listed twice, and no line of the
  game roots a block. A block that no code refers to may be left out of the program: nothing can spawn from
  it.
- **Every declared name is checked when the game starts.** One with no definition is refused, with the name,
  its block and `Game::prefab`. Each definition is checked too: its components are known to the world, its
  assets exist and are of the right kind, its children and parts name prefabs that exist, and no rule has two
  parameters [in conflict](#3-rules-and-the-tick) over it. A prefab a content record names is checked with
  the content, also before play. So choosing between `BAT` and `ELITE_BAT` at a wave's tenth second needs no
  check at that tick: both were checked at startup.
- **No reference an author wrote waits for its use to be checked.** A `Prefab` value is a declared name or
  comes from a content record, and both are checked before play. A check at the moment of use is kept for a
  name that first becomes known while the game runs. The first release has none: a
  [data pack](#12-content-assets-and-saves)'s names, when packs come, are checked as the pack is loaded.
- A prefab also has its name in an error, a recording and [a data pack](#12-content-assets-and-saves).
- A bare tuple is still a bundle: `entities.spawn((Rock, model, at))`. A function that returns a bundle or a
  spawn is still a good way to compute one. A prefab is for what is spawned by name, varied or checked early.
- A look that needs a transform brings one at the origin when the bundle has none.

**Children.** A child stores a local transform, is drawn and queried at its parent's world transform, and is
despawned with its parent. Entities with no parent pay for no hierarchy.

```rust
impl Entities {
    pub fn attach(&mut self, child: Entity) -> Attach;       // .to(parent) then .keep_world() or .keep_local(pose)
    pub fn detach(&mut self, child: Entity) -> Detach;       // .keep_world() or .keep_local()
    pub fn parent(&self, child: Entity) -> Option<Entity>;
    pub fn children(&self, parent: Entity) -> impl Iterator<Item = Entity>;   // in the order attached
    pub fn part(&self, parent: Entity, name: Part) -> Entity; // refuses the tick if its prefab has no such part
    pub fn world2d(&self, entity: Entity) -> Transform2d;     // where it is in the world, through its parents
    pub fn world3d(&self, entity: Entity) -> Transform3d;
    pub fn disable(&mut self, entity: Entity);                // with its children: not drawn, queried or touched
    pub fn enable(&mut self, entity: Entity);
}
```

- A **part** is a child with a name: a warden's eye, a hand, a place on a back to carry things. A part with no
  look is a socket. `entities.part(warden, EYE)` reaches it, and a frame function reaches it through `Family`.
- Attaching says whether the child keeps its place in the world or takes a given local place. A cycle is
  refused. A reference that is not ownership, such as a target, stays an ordinary `Entity` field.
- **`keep_world()` stores the local transform that recomposes nearest to where the child stood, and promises
  a bound, not exactness** ([C01](../../design/CONTRACTS.md)). A place in the world is a position, a turn and
  a scale in the profile's numbers, and so is a local transform, and most places have no local transform that
  recomposes to them bit for bit: a child at x = 0.1 under a parent at x = 10 scaled by two comes back at
  0.10000038, with nothing but a whole-number move and a power-of-two scale involved.
  - **The bound.** After `keep_world()` the child's recomposed position is within one part in 2^17, about 8
    in a million, of the largest of one unit, the child's distance from the world's origin and its new
    parent's; its scale is within the same fraction of what it was; and its turn is within 0.001 of a degree.
    Under parents that are moved, turned and scaled by one factor each, which is what `.scaled(s)` gives, and
    whose scale through all their parents is between 1/10,000 and 10,000, ordinary rounding is far inside it.
  - **Exactness is a property of the numbers, not of the call.** The child stands exactly where it stood
    when every number on the way is representable: for instance a child and parents at whole-number
    positions, with quarter turns and scales that are powers of two. The reference says so and promises it
    of nothing else.
  - **Refusal.** The attachment is refused, as a cycle is, when the new parent's scale through its parents
    is zero or outside 1/10,000 to 10,000, which is C01's singular and ill-conditioned parent; when the
    result would need shear; or when the recomposed place is past the bound, which the SDK tests on every
    `keep_world()`, as C01's excessive recomposition error. The tick is refused with a typed error that names
    the child, the parent and the cause, and the child keeps the parent, the local place and the place in
    the world it had.

  `detach(..).keep_world()` is held to the same rule, and `keep_local(pose)` cannot fail this way.
- A **disabled** entity keeps its components and its place and is skipped by queries, space queries and
  drawing until it is enabled. It is how a wreck leaves the floor and comes back.
- Attaching, detaching and disabling land when the rule finishes, as a spawn does.

### 6. Players, actions and control

**An action map** is a struct whose fields are the things a player can do (R7). A rule reads it as plain
fields.

```rust
#[derive(Actions)]
pub struct Controls {
    #[bind(Keys::A_D, Keys::LEFT_RIGHT, Touch::stick_x())]
    pub run: Axis,                 // one number from -1 to 1
    #[bind(Keys::WASD, Touch::stick())]
    pub walk: Stick,               // a direction no longer than 1
    #[bind(Key::Space, Touch::button("Jump"))]
    pub jump: Button,              // .held, .pressed, .released
    #[bind(Mouse::look_x())]
    pub look: Heading,             // an angle that the player's own machine keeps
    #[sent]
    pub aim: Heading,              // no device: a frame function sets it
}
```

| Kind | In a rule | Bound to |
|---|---|---|
| `Button` | `jump.held`, `jump.pressed`, `jump.released`: three `bool`s, and `jump.presses`, how many times it went down since the last tick | A key, a pointer button, `Touch::tap()`, `Touch::button(label)`: any number of on-screen buttons |
| `Axis` | A `Num` from −1 to 1. It multiplies and compares as one | A pair of keys, `Touch::drag_x()`, one axis of a touch stick |
| `Stick` | A `Vec2` no longer than 1, with `.x` and `.y`. It multiplies as one | Four keys, `Touch::stick()` |
| `Heading` | An `Angle`, with `direction()` and `direction_xz()` | `Mouse::look_x()` with the pointer captured, or `#[sent]` |

- `#[derive(Actions)]` also writes a name for each field, `Controls::JUMP`, for the places that name an action
  and do not read it: a test, a key prompt, a camera rig, a remap screen.
- **The bindings on the map are the defaults.** The shell samples them, and each player can change theirs:
  a frame function takes `Bindings`, with `rebind(Controls::JUMP, input)`, `reset()` and
  `prompt(Controls::JUMP)`, the label to show for it. A player's bindings are local settings. A field with no
  `#[bind]` and no `#[sent]` is a compile error.
- The action maps that a game's rules read are its action set, known from their signatures. The set is the
  same on every peer, fixes the layout of the input frame, and is part of the rule-set identity. Startup
  checks that it fits the frame, with the counts and the limit.
- **A press is read once: never twice, and never lost while no more presses are made between two admitted
  frames than the count holds**, which is at least three and which the reference states beside this
  guarantee. Every input frame carries, for each button, whether
  it is held and a small running count of the times it has gone down, beside one bit that says a player's
  machine sampled this frame. The world keeps, for each player and button, the last count it read and whether
  the button was held. Presses do not travel as `SESSION`'s timed action, which a late frame drops. At each
  tick:
  - **A sampled frame** gives `presses`, how far the count has moved on from the last one read, and `held`.
    `pressed` is `presses > 0`. `released` is true when the button is up and was held at the tick before, or
    went down and up again since. Two taps between two admitted frames are `presses == 2` in one tick, with
    `pressed` and `released` both true.
  - **A frame the session repeats** for one that is late or missing carries the same count, so it gives no
    press and keeps what was held. The press a late frame carried is in the next frame that arrives, because
    the count is a running one.
  - **The neutral frame** the session puts in when a player's frames have been missing too long, or the
    player is away, has no sampled bit. Everything held is released, every axis is zero, and **the last
    count read is kept**. When the player's frames come back, a press made before the gap is not read again,
    and each press made during it is read once, in the first tick admitted after.
  - **The count wraps, and that is the guarantee's bound.** A count that holds `n` presses wraps at `n + 1`,
    so `n + 1` taps in one gap that end with the button up read as no tap at all. `n` is at least three. The
    reference gives the built `n` in the sentence that says a press is never lost, and more presses than
    that in one gap are counted modulo `n + 1`.
  - A machine starts its counts from the world's, which a baseline carries, so joining and reconnecting
    press nothing.
  - **A pause is kept apart from play where the press is made**, on the presser's own machine, because the
    count alone cannot do it: a tap during a pause and a tap just after it, with the frames between them
    lost, would arrive as the same count. A player's screen **shows the pause** from the picture in which its
    world first has the paused state until the picture in which its world has the pause's end. Alone that is
    at once. In a match it is when the host's ticks reach that machine, a moment after the host's own, and
    that is all a remote player can know of where the pause began and ended. A modal that holds a pause
    shows it from the frame it opens.
    - While a player's screen shows the pause, their machine **counts no press**. A key that goes down then
      is ignored until it is let go, as a key that went down in another input context is. A key held since
      before the pause showed keeps reading as held, so a direction held through a pause carries on with no
      `released`, and letting it go is sampled when it happens.
    - The world reads no button and no axis in a paused tick, and keeps each last count as it was.
    - So a tap made while the presser's screen shows the pause moves no count and is never read, whichever
      of their frames arrive. A tap made once their screen shows the game running again is counted and read
      once, in the first tick that admits a sampled frame of theirs, however late. With the last count read
      at five, the first leaves the count at five and the second makes it six.
    - A press made while the presser's screen still showed play was made in play, even where the host had
      already paused: it is counted, no paused tick reads it, and it is read once in the first tick after
      the pause, as a command already on its way is sent again.
- **A `Heading` belongs to the player's machine.** Bound to the mouse, it turns at the frame rate with the
  pointer captured, so a camera that reads it has no delay, and its latest value travels with each tick's
  input, so "forward" in a rule is the way the player was looking.
- **`#[sent]`** is for a value a frame function works out, such as the direction from a craft to the pointer.
  The frame function takes `MyInput<A>` mutably and assigns the field.
- **Local actions never leave the machine.** `#[derive(LocalActions)]` is for keys only the screen cares about:
  pause, a scoreboard, a journal. A frame function reads them with `MyInput<Menu>`. A rule cannot take them,
  they are in no input frame, digest or replay, and they are still read while a modal is open.
- **An input context** says which action maps this screen's keys, pointer and touches drive now, so one key
  can jump in play and confirm in a menu while the world keeps ticking (R7).

  ```rust
  pub const SHOPPING: InputContext = InputContext::new("shopping");

  #[derive(LocalActions)]
  #[context(SHOPPING)]                 // sampled only while this screen's context is SHOPPING
  pub struct ShopKeys {
      #[bind(Key::Space, Touch::button("Buy"))]
      pub buy: Button,                 // Space is `jump` in play and `buy` here
      #[bind(Keys::LEFT_RIGHT)]
      pub browse: Axis,
  }

  fn shop(mut context: Screen<InputContext>, keys: MyInput<ShopKeys>, /* .. */) {
      *context = if near_the_stall { SHOPPING } else { InputContext::PLAY };
      // ..
  }
  ```

  - A map says its context with `#[context(NAME)]`. A shared map with none is in `InputContext::PLAY`, and a
    local map with none is always on, which is why a pause key works everywhere.
  - A screen has one context at a time, `PLAY` unless `scene.input_context(NAME)` says otherwise. It is
    `Screen<InputContext>`, this screen's own state, and a frame function sets it.
  - A map whose context is off is sampled as idle: its buttons are up and its axes zero. A shared map is still
    in every input frame with the same layout, so every machine's rules read the same idle input, and a
    context never changes what a rule is or what a match agrees on.
  - **A held input across a change.** When a context goes off, what its maps held is let go, and a held
    button reads `released`. When it comes on, a key that has been down since before it last went off drives
    its action again, so a direction held through a menu carries on. A key that went down while the context
    was off is ignored by it until the key is let go, so the Space that bought the potion does not jump.
  - Bindings are changed and prompted for by action, as before, whatever context the action's map is in.
- A frame function also reads the shared map as sampled this frame, `MyInput<Controls>`, and the `Pointer`:
  where it is, `pointer.ray(&camera)`, `camera.ground_point(&pointer)` in 3D and `camera.world_point(&pointer)`
  in 2D.

**A player** is who an input came from. Who is in the game is world state.

```rust
impl Players {
    pub fn iter(&self) -> impl Iterator<Item = Player>;       // in seat order
    pub fn count(&self) -> u32;
    pub fn host(&self) -> Player;
    pub fn name(&self, player: Player) -> Text<24>;
    pub fn status(&self, player: Player) -> PlayerStatus;     // Here, Away { since }, Bot
    pub fn character(&self, player: Player) -> Option<Entity>;
}
impl PlayersMut {                                             // everything `Players` has, and:
    pub fn add_bot(&mut self) -> Option<Player>;              // None when every seat is taken
    pub fn remove_bot(&mut self, bot: Player);
    pub fn respawn(&mut self, player: Player);                // runs the scene's spawn for them again
    pub fn possess(&mut self, player: Player, character: Entity);
}
```

- **Looking and changing are two types**, as `Phase<P>` and `PhaseMut<P>` are. `Players` has no method that
  changes anything, so it is what a frame function, a spawn function and a condition are given. `PlayersMut`
  is a rule's alone: a frame function that asks for it does not compile, with the message that frame
  functions cannot change the world.
- `PlayerJoined` is sent once for a player, at the tick they enter the game, to whichever scene is running. A
  later scene is not told again: it finds who is here with `Players`, and its `players()` line has already
  spawned their characters. `PlayerLeft`, `PlayerAway` and `PlayerBack` come at the tick of each.
- `PerPlayer<T>` in a resource keeps one value for each seat, such as a score that outlives a character. It
  returns to its default when the seat's player leaves.
- A frame function takes `Players` too, and `Me`: `me.player()` and `me.character()`.

**Seats.** A world has four seats, `SESSION`'s four slots, and `Multiplayer::players` says how many of them a
match uses. A `Player` names a seat. Which seats are taken, and whether by a person or a bot, is world state
the SDK keeps, with the session slot that sits in each: `SESSION`'s slot numbers never reach a game.

- A person who joins takes the lowest free seat. `add_bot` takes the highest free seat.
- **A person is never turned away for a bot.** When a person is admitted and bots hold every free seat, the
  person takes the lowest seat a bot holds. In that tick the bot leaves as any player does: `PlayerLeft` is
  sent for it, its character is despawned, its part in any open offer ends and the seat's `PerPlayer` values
  return to their defaults. Then the person joins as any player does: `PlayerJoined`, and a character from
  the scene's `players()` line. Nothing of the bot's passes to the person.
- **When people hold every seat the match uses, a newcomer is refused as full, and the SDK is what refuses
  them.** `SESSION` has four slots whatever the game asked for, admits a connection to any free one and
  knows nothing of seats: in a match of two seats with a host and a guest, it would admit a third person.
  The limit is kept in two places, with `SESSION` as built:
  1. *Before the handshake.* The host's SDK is what hands each incoming link to `SESSION`. While people hold
     every seat and none of them is away, it hands over none: it closes the link with the reason full, and
     the newcomer's lobby says so.
  2. *At the tick.* While a seated player is away, a link must reach the handshake, because only `SESSION`
     can tell that player's return, by their token, from a newcomer. A newcomer who is given a slot then, or
     who arrives beside another for the last seat, is joined by `SESSION` at a tick as anyone is, and the
     world, finding no free seat and no bot, gives that slot **no seat**: no `PlayerJoined`, no character, nothing reads its frames, and world state marks the
     slot unseated. Their machine reads that in the world it is sent, leaves the match and shows the reason
     full. An unseated slot is never seated later: its machine joins again. One that does not leave authors
     no frames, and `SESSION`'s own absence limit closes it.
- **An away player's seat is theirs for the grace.** It counts as held by a person. `SESSION` keeps their
  slot for their token and the SDK keeps the seat that slot sits in, so their return finds both, whoever
  else knocked meanwhile.
- Joins that `SESSION` admits in one tick are seated in slot order.
- A join is admitted input and the seats are world state, so the host, the joining machine, whose baseline
  is the world before that tick, and a replay all seat the same player in the same place, or agree that the
  slot has no seat.

**A controlled character** (D3, R1). One line of a scene gives every player a character and says how it moves:

```rust
scene.players()
    .spawn(hero_at_start)              // a prefab, or a function that returns what to spawn
    .control(run_and_jump)             // runs here in the written order, once for each player's character
    .during(Stage::Playing)            // optional: only in this phase
    .when_away(Away::Idle);            // optional: Idle (the default), Remove, or Bot
```

- **`spawn`** takes a prefab or a function. The function may take `Joined` (who it is for) and any
  [read-only parameter](#3-rules-and-the-tick), so it can place the character at a level's start or in a
  line. It runs for each player already here when the scene is entered and for each who arrives later. The
  character is given that `Player` as a component. A spawn function makes world state, so in a checked game
  it is [defined in the `rules` module](#1-one-crate-one-package) with the rules.
- **`control`** takes a function whose `&T` and `&mut T` parameters are components of that one character, with
  `&A` for that player's action map. It is the character's whole movement: it writes the pose itself. It
  names a component of its character once, and [what else it may reach](#3-rules-and-the-tick) is tested
  against the character its player has.
- A character is despawned when its player leaves. While a player is away their character stays and is
  controlled with idle input, unless `when_away` says otherwise. A rule that despawns a character, as a death
  does, brings it back with `players.respawn(player)`.
- A scene may have more than one `players()` line, such as one for the craft and one for a spectator.

**What a control function may read, and what a match can run ahead.** It may take any parameter a rule may.
In a match, the local player's own character answers at once when everything its function reads is on this
list:

| Parameter | Alone | In a match, for the local player's character |
|---|---|---|
| Its own components, `&A`, `Tick`, `Entity`, `Random`, `Content<T>` | Read | Run ahead exactly. Content and the seed are the same on every machine and never change. Everything `Tick` gives is run ahead: the tick's number, and `in_scene()` and `in_phase()`, which count from entries [the step's state carries](#15-multiplayer-as-an-opt-in) |
| `Res<T>`, `Phase<P>`, `Level2d` | Read | Run ahead with the value at the last confirmed tick. A change arrives a moment later, [as a correction](#15-multiplayer-as-an-opt-in). Free for settings, the phase, the level and anything that changes rarely. A `.during(phase)` on the line reads the phase the same way |
| `Emit<E>` | Sent | The event reaches this screen's frame functions when it is predicted, [once](#10-frame-functions-and-the-frame), so a jump's dust is not late. Rules read it at the confirmed tick |
| `Query`, `Single`, `Space2d`, `Inputs`, `Players`, `Family`, `Events<E>`, `Commands<T>`, `Restored<T>` | Read | **Not run ahead.** They read what other machines move or send, which this machine has only once the host confirms the tick: an event an earlier rule sent is one of those |
| `ResMut`, `Entities`, `PhaseMut`, `Scenes`, `Offers`, `LevelMut2d`, `PlayersMut`, `BotInputs`, `Saving` | Written | **Not run ahead.** They change more than the character |

Every parameter a rule may take is in one row. A function that takes anything from the last two rows still
works everywhere. Its character is then shown as
the host confirms it, a round trip late. So that nobody pays that by accident, a game with multiplayer on
refuses to start until that line says so: `.control(walk).unpredicted()`. The message names the parameter
that costs prediction. Alone, nothing is asked. Predicting more than the character
(its shots, the players it touches) is [reserved](#reserved-shapes).

**A bot** is a player whose input a rule writes. `PlayersMut::add_bot` takes [a seat](#6-players-actions-and-control)
and is announced as any player is. A rule written before the control line sets its input with
`BotInputs<A>`, and from there on nothing tells it from a person: the same spawn, control function, score
table and offers. An offer put to a bot is answered by a rule, with
[`Offers::answer_for`](#7-commands-offers-and-requests), since a bot has no screen. A bot's seat and input
are world state, so a bot match replays and a late joiner sees it.

### 7. Commands, offers and requests

**A command** is a value of a type that derives `Command`, sent from a frame function with `Outbox`.

```rust
impl Outbox {
    pub fn send<C: Command>(&mut self, command: C) -> Delivery;
}
#[must_use]
pub enum Delivery {
    Queued,   // it will be admitted, unless the link drops first
    Closed,   // the match has ended
}
```

- The SDK keeps commands that cannot go yet and sends them in order, so a menu never handles a full queue.
  In a match a host's commands for one tick share its one input frame, which the SDK
  [budgets whole](#12-content-assets-and-saves): what does not fit a tick's frame goes in the next.
- An admitted command is read by `Commands<T>` at one tick, once, after the same player's earlier commands, on
  every machine.
- If the link drops while a command is on its way, the game cannot know whether it was admitted, and the
  player comes back to a fresh copy of the world. So an interface is drawn from what the world says, and a
  repeated command is made harmless. An [offer](#7-commands-offers-and-requests) does both for a choice.
- A rule that refuses a command says so with an event, which the sender's frame functions read.
- Startup checks that each command type's encoding fits what one action of an input frame may carry.

**An offer** is a question the rules put to players, answered once (R8).

```rust
impl<T: Data> Offers<T> {
    pub fn open(&mut self, offer: Offer<T>) -> OfferId;
    /// The answers admitted this tick: one for each player and offer, from a player it was put to, for a
    /// choice it held, while it was open. Anything else is dropped.
    pub fn accept(&mut self) -> impl Iterator<Item = Answer<T>>;   // .player, .choice, .offer
    pub fn tally(&self, offer: OfferId) -> Tally<T>;               // how many chose each, and who has not
    pub fn close(&mut self, offer: OfferId);
    /// What `player` has been asked and has not answered: `.offer` and `.choices()`.
    pub fn pending(&self, player: Player) -> Option<Pending<T>>;
    /// A bot's answer, taken by the next `accept` through the same check as a person's. False when `bot` is
    /// not a bot, the offer is not open to it, the choice is not one it holds, or it has answered.
    pub fn answer_for(&mut self, bot: Player, offer: OfferId, choice: T) -> bool;
}
Offer::to(player)          // one player
Offer::to_each_player()    // each player answers for themselves
Offer::to_all()            // one question to everyone: a vote
    .choices([..])
    .expires(tick.after(seconds(15)))   // or .until_phase_ends()
```

- Open offers are world state: a late joiner and a reconnecting player are shown theirs, and the cards stay
  until the answer has landed.
- A player who joins while `to_each_player` or `to_all` is open is asked. A player's part in an offer ends
  when they leave, and an answer of theirs not yet accepted is dropped.
- A person answers from their screen and a bot through a rule. Both answers are taken by `accept`, in seat
  order, and nothing downstream can tell them apart.
- An `OfferId` holds the scene's entry and a count, so a second run cannot answer the first run's offer.
- A frame function takes `MyOffer<T>`: `open()` gives this player's open offer and its choices, `choose(v)`
  sends the answer, and `status()` is `Waiting`, `Sending` or `Taken`. The game still decides what a choice
  does.

### 8. Space: colliders, queries, the level, and moving a character

**Colliders.** `Collider2d::circle(r)`, `Collider2d::rect(w, h)` and `Collider2d::capsule(r, length)`, each with
`.offset(x, y)` and `.on(LAYER)`. A collider is on one layer. Colliders detect and never push: there are no
rigid bodies (O4). In a 3D scene a `Collider2d` lies on the ground plane, x and z, so triggers, pick-ups and
catch radii work in a 3D game with the first release's queries.

**`Space2d` answers from where things are now** (R2): after every rule before this one, the last rule's spawns
and despawns included.

```rust
impl Space2d {
    pub fn touching(&self, entity: Entity) -> Found;           // what its collider overlaps
    pub fn within(&self, point: Vec2, radius: Num) -> Found;
    pub fn in_rect(&self, rect: Rect) -> Found;
    pub fn nearest(&self, point: Vec2) -> Found;               // nearest first; .within(r) bounds the search
    pub fn sweep_circle(&self, from: Vec2, to: Vec2, radius: Num) -> Swept;
    pub fn motion_of(&self, entity: Entity) -> Motion;         // its path this tick: .sweep(), .delta()
    pub fn at_tick_start(&self) -> Space2d;                    // the same queries, on where things began the tick
    pub fn move_box(&self, from: Vec2, size: Vec2, step: Vec2) -> Move2d;      // against the level and colliders
    pub fn move_circle(&self, from: Vec2, radius: Num, step: Vec2) -> Move2d;
}
// Found:  .on(LAYER) .except(entity) then .first() .any() .count() or iterate: Near { entity, position, distance }
// Swept:  .on(LAYER) .except(entity) then .first_hit() -> Option<Hit { entity, point, normal, fraction }>
```

A `Space2d` sweep meets colliders, and every collider is on an entity, so its `Hit` always has one. The level
has no entities, so [a ray against the level](#8-space-colliders-queries-the-level-and-moving-a-character)
returns a hit of its own kind, with the layer in the entity's place.

- **One index for each layer.** `.on(LAYER)` searches that layer and nothing else, so an enemy that asks for
  the nearest of four heroes looks at four colliders with 10,000 enemies around it. A query with no `.on`
  asks every layer and merges the answers in the query's own order.
- **Two orders, by what was asked.** An overlap (`touching`, `within`, `in_rect`, `nearest`) answers nearest
  first, by distance from the point or the entity asked about. A sweep (`sweep_circle`, `motion_of(..).sweep()`)
  answers earliest contact first, by `fraction` along the path, so `first_hit()` is the first thing met and
  not the thing whose centre is nearest. Ties go to the earlier spawned in both.
- **A layer's index is rebuilt only when a rule has changed that layer** since it was last built: moved one of
  its colliders, spawned one or despawned one. A crowd's steering pass, written before the crowd moves, reads
  the index built at the tick's start, and so does every reader until something moves.
- `at_tick_start()` is for a rule that wants the older view on purpose, such as steering that must not depend
  on which bat moved first.
- **`motion_of(entity).sweep()`** sweeps the entity's own collider along the path it took this tick, from
  where it stood at the tick's start, or where it was spawned, to where it is now, against the paths the
  others took. Earliest contact first, ties by spawn order. It is how a fast shot hits and how a stomp knows
  it came from above.
- `nearest` without `within` looks at every collider on the layer, which is right for a small layer. The
  guides say to give a radius on a crowded one.

**The level is a scene's fixed collision** (R7). It does not move, so it is not made of entities: it is not
counted in the world's size, and not written to the digest or sent to a late joiner tile by tile. What the
world holds is **the level's recipe**: which tile map is loaded, by that map's own identity within the game's
content, and each fixed shape a rule has added since, in the order added. The recipe is world state, so it is digested, rewound and sent in a
baseline, and the collision is built from it, the same on every machine: a tile map is content every machine
holds before the run starts, and an added shape is in the recipe itself.

```rust
scene.tiles(Tileset::new(art::TILES)      // how this scene reads a tile map
    .solid('#', TERRAIN)                  // a solid tile on a layer, drawn from the sheet by its neighbours
    .one_way('=', TERRAIN)                // solid from above only
    .spawns('o', COIN)                    // a prefab at that tile
    .marks('@', START));                  // a named place, with no tile

impl LevelMut2d {                         // in an enter function, or any rule
    pub fn load(&mut self, map: &Tilemap);                    // replaces the level: solids, spawns, marks, tiles
    pub fn add_rect(&mut self, rect: Rect, layer: Layer);     // fixed shapes a game places itself
    pub fn add_segment(&mut self, from: Vec2, to: Vec2, layer: Layer);
    pub fn clear(&mut self);
}
impl Level2d {
    pub fn move_box(&self, from: Vec2, size: Vec2, step: Vec2) -> Move2d;
    pub fn move_circle(&self, from: Vec2, radius: Num, step: Vec2) -> Move2d;
    pub fn touching_box(&self, center: Vec2, size: Vec2) -> FoundTiles;   // .on(LAYER) then .any() or iterate
    pub fn solid_at(&self, point: Vec2) -> Option<Layer>;
    pub fn ray(&self, from: Vec2, to: Vec2) -> SweptLevel;
    pub fn mark(&self, mark: Mark) -> Vec2;                   // the one place with this mark
    pub fn marks(&self, mark: Mark) -> impl Iterator<Item = Vec2>;
    pub fn bounds(&self) -> Rect;
}
// SweptLevel:  .on(LAYER) then .first_hit() -> Option<LevelHit { layer, point, normal, fraction }>
```

- A `Tilemap` is content: rows of glyphs in a level's file, as example D's is. It comes from content and
  nowhere else. **Every tile map in the cooked content has an identity of its own**: the record it is in and
  its place in that record, which is the path of fields that leads to it, with its position wherever the
  path passes through a list. Cooking gives it,
  the same on every machine, since the content is part of the rule set. So a record with
  `upper: Tilemap` and `lower: Tilemap`, or with `floors: Vec<Tilemap>`, holds maps that are told apart, and
  one a test adds with `with_content` has an identity the same way. That identity is what `load` writes into
  the recipe and what the built collision is kept under, so a joiner sent a baseline builds the floor the
  host loaded and a step run ahead never borrows the other floor's walls. It is loaded
  where the game says, usually an enter function, so a level chosen at run time is the ordinary case. The
  tiles are drawn as one tile layer, not as an entity each. A level a rule makes for itself is made of
  `add_rect` and `add_segment`.
- **A change to the level is a rule's act, and the recipe is what changes.** It is held while the rule runs,
  lands when the rule finishes as a spawn does, and is undone with a refused tick. Nothing is loaded at a
  tick: the tile maps are part of the game's content, held before the first tick, so a tick's collision
  depends on the world at that tick and on nothing that arrives late
  ([C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)).
- The recipe holds a fixed number of added shapes, 256 unless `scene.level_shapes(n)` says otherwise, because
  the world's storage is reserved once. One more refuses the tick and names that line. `load` and `clear`
  start the recipe again.
- **A joiner, a rewind and a step run ahead each get the level of their own tick.** A joiner is sent the
  recipe in its baseline and builds the collision before it steps. A rewind restores the recipe with the
  rest of the world. The SDK keeps the built collision of every recipe still inside the rewind and
  prediction span, found by the recipe's identity, so going back to a tick before a gate closed finds the
  open gate without a rebuild. A control function run ahead reads the level its state names, and a change of
  level reaches it [as a correction](#15-multiplayer-as-an-opt-in). So a tile map a file chose and a
  rectangle a rule added give the same answers to `move_box`, `touching_box` and `ray` after a baseline and
  after a rewind as they gave the host the first time.

**Moving a character** is one call that slides along what it meets and reports it.

```rust
let moved = tiles.move_box(at.position, HERO_SIZE, hero.velocity * tick.dt()).on(TERRAIN);

pub struct Moved2d {
    pub position: Vec2,                              // where the box ends up
    // ..
}
impl Moved2d {
    pub fn on_floor(&self) -> bool;
    pub fn on_ceiling(&self) -> bool;
    pub fn wall(&self) -> Option<Side>;              // Left or Right
    pub fn contacts(&self) -> &[Contact];            // each: .normal, .layer, .entity (None for the level)
    pub fn floor(&self) -> Option<Contact>;          // what it stands on, such as a moving platform
    pub fn slide(&self, velocity: Vec2) -> Vec2;     // the velocity with what was stopped taken out
    pub fn blocked(&self) -> bool;
}
```

- `move_box` returns a `Move2d`, and `.on(layers)`, with one layer or a tuple of them, makes the move and
  returns the `Moved2d`. `.up(direction)` before it is for a game whose floor is not below. The whole step is
  one call, both axes, so no game moves an axis at a time or compares positions to learn what happened.
- `Level2d::move_box` meets the level only, and a match can run it ahead. `Space2d::move_box` also meets the
  colliders on the layers it is given, such as moving platforms, and reports the entity it stands on. A
  control function that uses it is `.unpredicted()` in a match.
- One-way tiles stop a box that comes from above and let it through from below.

In 3D the same shapes are [reserved](#reserved-shapes): `Collider3d`, `Space3d`, `Level3d` and `move_capsule`.

### 9. Numbers

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

`Num` has `sqrt`, `abs`, `floor`, `ceil`, `round`, `min`, `max`, `clamp`, `lerp` and `sign`. `Angle` has `sin`,
`cos`, `direction`, `direction_xz` and `between`, built from exactly rounded operations with no platform maths
library, with conformance vectors on the three targets
([ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)). Vectors have `length`, `length_squared`,
`normalized_or_zero`, `clamp_length`, `clamp_to(rect)`, `distance`, `dot` and `lerp`.
`normalized_or_zero` of a zero vector is zero, so a bat on top of its target does not make a tick fail.

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
game's [`rules` module](#1-one-crate-one-package) on its own and holds it to the profile, as the profile holds
a rule package today: no raw-float arithmetic or methods, no unordered containers, clocks, threads,
environment, files, sockets, processes or standard input, and no outside crate. Each refusal names the line,
the reason and what to write. It is a guard against mistakes, not a sandbox: `unsafe` code can still reach out,
and what gets past is caught, where it shows, by the digests a match compares.

### 10. Frame functions, and the frame

A frame function runs once for each picture the engine encodes, when the engine samples the scene for that
picture. For every picture the engine chooses a pair of adjacent published ticks and how far between them the
picture stands ([C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)). The
frame functions run on that pair at that fraction, and the draw that follows uses the same. That is **the world
as shown**:

- a pose is where the picture has it, between the pair's two ticks. A camera set from a hero's pose and the
  hero's sprite are therefore one instant, whatever the tick rate and the display rate;
- everything else is as of the pair's newer tick;
- in a match, the local player's own character is at its predicted pose, in a query and in the draw alike.

A frame function writes only what belongs to this screen.

| Parameter | What it gives |
|---|---|
| `Query<D, F>`, `Single<D, F>`, `Res<T>`, `Phase<P>`, `Content<T>`, `Family`, `Players` | The world as shown, read-only. Poses are `Pose2d` and `Pose3d`. `Players` has no method that changes the roster; `PlayersMut` is a rule's and does not compile here |
| `Events<T>` | The events of every tick since this function last ran, each once, in tick order. `missed()` counts any that were dropped |
| `Screen<T>` | State of this screen: the camera, the sky, a local setting, [the input context](#6-players-actions-and-control), the game's own menu state. Never part of the world |
| `Shown<L>` | How this screen dresses a look, for this picture: `shown.of(entity)` |
| `Ui` | The interface for this frame |
| `Draw2d`, `Draw3d` | Things drawn for this picture only, among the world's: a sprite, a shape, a line, a marker, and `burst`, a short effect the SDK keeps and plays out |
| `MyInput<A>`, `Pointer`, `Bindings` | This player's actions as sampled this frame, local actions, the pointer, and their bindings |
| `Outbox`, `Request`, `MyOffer<T>` | The ways a frame function reaches the world: a command, a request for a scene, an answer |
| `Pause` | Stopping and starting the tick |
| `Saves` | Reading, writing and deleting saves |
| `Me`, `Session` | Which player this screen is, and the match |
| `Frame` | The frame's time: `dt()` and `seconds()` in `f32`, and `seconds_until(tick_number)` |

**`Screen<T>`** is one value for this screen, whichever frame function asks. The SDK's own (`Camera2d`,
`Camera3d`, `Sky`) are set from the scene's description each time the scene is entered. A local setting is the
panel's value. A game's own type starts as its `Default` and lasts for the whole game, unless a scene says
`scene.screen(value)`, which sets it each time that scene is entered. `scene.on_show(f)` runs a frame function
once, when this screen first shows the scene.

**`Shown<L>` is how one screen dresses a look** (R6). The look on the entity is world state: which sprite or
model it is. What a screen does with it is not:

```rust
shown.of(entity)
    .play(art::hero::RUN)      // a clip; asking again for the one that is playing does not restart it
    .play_once(art::hero::LAND)
    .flip_x(facing_left)
    .tint(Color::RED).flash(0.3)
    .turn_y(angle).offset(vec3f(0.0, bob, 0.0))   // a decorative turn or bob
    .hide();                   // on this screen only
shown.of(entity).finished()    // whether a `play_once` clip has ended
```

A clip chosen this way is timed by the frame. A look that every player must see at the same moment, such as a
coin that spins in step, is `Sprite::new(..).looping(clip)` on the entity, timed from the tick. A rule that
depends on a moment of an animation owns that moment as a `Timer`.

**A cut.** Every pose is drawn between its last two ticks, so a move that is not a movement must say so.
`at.teleport(point)` on a `Transform2d` or `Transform3d` sets the position and marks the tick: the entity is
drawn at the new place at once, a camera following it cuts with it, and a predicted character is not eased
toward it. `camera.cut()` does the same for a camera a frame function moved.

**Cameras.** The scene describes its camera, and a rig keeps it there with no frame function:

```rust
Camera2d::fit(24.0, 18.0)                     // this rectangle is always in view
Camera2d::fit_height(14.0)                    // this height, and as much width as the window gives
    .following_my_character()                 // or .following(entity)
    .damping(seconds(0.15))
    .inside_level()                           // never shows past the level's bounds, or .inside(rect)
Camera3d::looking_at(eye, target).fov(35.0)
Camera3d::following_my_character().offset(0.0, 1.5, 4.0).damping(seconds(0.2))
Camera3d::orbiting_my_character().yaw(Walk::LOOK).distance(6.5).height(1.5)   // .avoiding(SOLID) is reserved
```

A frame function that wants something else writes `Screen<Camera2d>` or `Screen<Camera3d>` itself, as
example C does, and `ease_to` and `cut` are there for it. The camera, sky and background belong to the
screen, so two players of one match can look at different places.

**Events reach the frame through a bounded cursor.** Pictures and ticks do not come at the same rate, so the
tick an event happened in may be an endpoint of no picture. With each tick it commits, the SDK publishes that
tick's events in a ring that holds at least one second of ticks. Each `Events<T>` of a frame function keeps a
cursor, the last tick it has delivered.

- A run whose newer tick is `n` delivers the events of every tick after the cursor, up to `n`. A tick that
  fell between two pictures loses nothing.
- A run on the same pair as the last delivers nothing a second time.
- When more ticks have passed than the ring holds, as after a hitch or a hidden tab, the oldest are dropped
  and `missed()` says how many. Events are for show, so nothing that must happen hangs on one: a save, a
  score and an unlock are world state.
- A rewind, a resync and a change of scene reset the cursor. A tick stepped again after a rewind delivers its
  events again, because the second run is the one that stands.
- **An event the local player's own control function sends while it is run ahead is shown then, and each
  is shown once.** The session may run that step several times for one tick, again after each correction, and
  the host's own run of the tick arrives later, and the runs need not send the same events.
  - **What one emission is.** An emission of the local player's control line is named by four things: the
    control line, the game's tick the step is for, the event's type, and its ordinal, which counts that
    step's sends of that type from zero in the order the function makes them. Two runs of one tick that
    each send a `Dust` send "the same" first `Dust`, whatever its fields, and a run that sends two sends
    that one and a second.
  - **What is shown.** For each control line, and for each tick still inside the prediction span or just
    confirmed, the SDK keeps how many emissions of each type this screen has been given. A run of that tick,
    whether the first run ahead, one run again after a correction or the confirmed one, delivers the
    emissions whose ordinal is at or past that number, and the number moves on. So prediction that sends
    one dust burst at tick 40 shows it at once; the confirmed tick 40 that sends that burst and a second
    shows the second, late, and not the first again; and a confirmed tick that sends fewer takes nothing
    back. The fields a frame function reads are those of the run that delivered the emission.
  - **How the SDK gets them.** `SESSION`'s step returns its state and nothing else, and its observer reports
    comparisons and corrections. The emissions leave through neither. The step is the SDK's own function,
    and the `Rules` value `SESSION` hands it on every run is the SDK's own type: beside the rule set's
    constants it holds this machine's note of emissions, which each run of the step writes under the four
    names above and the SDK reads after the session advances. The note is not in the step's state, is
    compared with nothing and changes nothing the step returns, so the step stays the pure function
    `SESSION` asks for. The confirmed run's emissions come as every tick's events do, in the ring, where
    each event carries the line and player that sent it and its ordinal.
  - Every other machine, and every rule, reads the event at the confirmed tick.

**A picture that is never shown.** The engine runs the frame functions when it encodes a picture, and only
afterwards asks the display for somewhere to put it. Sometimes there is nowhere. The frame functions still
ran, once, and nothing runs them again for that picture:

- **Everything a frame function does happens once, in the run where it does it.** An event is delivered to one
  run. A click is handed to one run. A command sent takes effect when it is sent.
- **What must be seen does not hang on one picture.** A `burst` is kept by the SDK as this screen's state and
  drawn in every picture until it ends, so one started in a dropped picture shows in the next.
- **What is drawn for one picture goes with that picture.** A sprite a frame function draws with `Draw2d`, and
  the interface it describes, are drawn again by the next run.

Interpolation between ticks is the engine's, on the GPU, for every look. A game writes none. A component is
published to frame functions only if one names it, so a component that only rules use costs nothing to show.

### 11. Looks: what is drawn

A look is a component, and an entity with a look and a transform is drawn. **A thing has the same look whether
it is placed or moves** (D2, R6).

| | 2D | 3D |
|---|---|---|
| Where | `Transform2d`: position, turn, scale | `Transform3d`: position, rotation, scale |
| What | `Sprite` (a frame of a sheet: layer, tint, glow, a looping clip), `Shape2d` (a circle, a rectangle, a triangle, a line, with no asset) | `Model` (a cooked mesh with a `Material`; `Model::cube()` and `Model::sphere()` with no asset), `Light::point(..)` and `Light::spot(..)` |
| Camera | `Camera2d` | `Camera3d` |
| Around it | `scene.background(colour)`, `scene.backdrop(Backdrop::new(image).parallax(0.3))`, the level's tile layer | `scene.sky(..)`, `scene.ground(..)`, `scene.place(..)` |
| Touching | `Collider2d`, `Space2d`, `Level2d` | `Collider2d` on the ground plane. The rest is [reserved](#reserved-shapes) |

- **`Model::new(art::meshes::ROCK).material(..)`** is one vocabulary. `Material::rough(colour)`,
  `Material::metal(colour)`, `Material::textured(art::materials::GRANITE)` and `.glowing(colour, strength)`
  work the same on an entity and on a placement.
- **`scene.place(model)`** stands a model in the scene with no entity: `.at(..)`, `.turned(..)`, `.scaled(..)`.
  It is for what has no gameplay identity, and it is the cheaper path: the engine's cooked placement, culled a
  meshlet at a time. Giving the same model to an entity is how it comes to move, and nothing about its look
  changes.
- **An entity's `Model` is drawn as an entity row of the 3D scene**, with its placement at two ticks, culled on
  the GPU and interpolated: position and scale linearly, the turn by the shorter way. Its lit faces and its
  shadow turn together between ticks. Two things the engine's entity rows do not do today are asked of the
  renderer ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-game-sdk)): the turn and the scale at both ticks,
  and a material that takes a texture.
- A transform is where the model's own origin stands. `Transform3d::at(x, y, z)`, `.turned_y(angle)`,
  `.facing(direction)` and `.scaled(s)` build one. A mesh whose origin is not at its base is fixed in the
  asset, or with `Model::new(..).origin_at_base()`, which is part of the look and says so.
- `Transform2d::at(x, y)`, `.turned(angle)`, `.scaled(s)`. Draw order in 2D is layer, then lower on the screen
  in front, then spawn order.
- `Draw3d` draws a marker, a line and a label at a point in the world, for this picture.
- Extraction is the SDK's, written once: it maps the looks of entities whose rows changed to `RENDER-2D`'s
  sprite rows and to the 3D scene's entity rows. An entity no rule writes is extracted once.
- The engine chooses the recipe for the device, as
  [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) says. A game never names
  one. Launch settings, such as the one that turns ray tracing on, pass through `run()` unchanged.

### 12. Content, assets and saves

Three kinds of data, each with a stable identity (D4, R10).

**Assets** are what is drawn. A game keeps them under `assets/`, and `cargo play` cooks them, watches them and
writes **the asset catalog**, `src/art.rs`: one typed constant for each asset, in a module for each folder.

```rust
// src/art.rs: written by `cargo play`. Do not edit.
pub mod enemies { pub const BAT: SpriteAsset = SpriteAsset::new("enemies/bat"); }
pub mod meshes  { pub const ROCK: MeshAsset = MeshAsset::new("meshes/rock"); }
pub mod hero    { pub const RUN: ClipAsset = ClipAsset::new("hero/run"); }
```

- The kind is in the type, so `Sprite::new(art::meshes::ROCK)` does not compile, and an editor completes
  `art::` to what exists. A saved asset is redrawn in the running game without a stall, as
  [CONTENT-LOOP](../CONTENT-LOOP/DESIGN.md) built. The catalog is committed with the game; `cargo play --check`
  fails when it is stale.
- `Sprite::named("enemies/bat")` and its like take a name found at run time, for content that arrives after
  the game was compiled. A name that matches nothing draws a visible placeholder and is reported once, with
  the nearest names that exist.

**Content** is what the rules read: waves, levels, enemies' numbers.

```rust
#[derive(Content)]
pub struct WaveDef {
    pub enemy: Prefab,             // a prefab, by name
    pub count: u32,
    pub speed: Num,                // written as 2.5 in the file
    pub next: Option<Id<WaveDef>>, // a link to another record
    pub taunts: Vec<String>,       // content may hold lists and text: rules read it and cannot change it
}

Game::new("Swarm").content::<WaveDef>("waves")      // assets/content/waves/*.ron, one record a file

impl<T: ContentType> Content<T> {
    pub fn get(&self, id: Id<T>) -> &T;
    pub fn find(&self, name: &str) -> Option<Id<T>>;
    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)>;   // in name order
}
pub const FIRST_WAVE: Id<WaveDef> = Id::new("first");          // checked when the game starts
```

- A record's **ID** is its file's name. `Id<T>` is `Copy` plain data: it goes in a component, a resource, a
  scene's fields, a command and a save, and it means the same record in a later build.
- Files are RON. A decimal in a file becomes a `Num` when the content is cooked, rounded once, the same on
  every host.
- **What a record may hold.** Everything world state may hold ([the table](#2-the-world-entities-components-resources)),
  except `Entity` and `Player`, which name nothing before a run; and, because rules read content and cannot
  change it, `String`, `Vec<T>`, a `Tilemap`, a typed asset name such as `SpriteAsset`, and another struct or
  enum that derives `Content`, nested to any depth. A record may hold several tile maps, in fields or in a
  list: each has [an identity of its own](#8-space-colliders-queries-the-level-and-moving-a-character). **A record holds no raw float**, whoever reads it: a
  number in content is a `Num`, an `Angle` or a `Seconds`, and a frame function that wants a float for a glow
  or a width asks for `.f32()`. So a record never mixes a gameplay number with a presentation float, and no
  rule can be handed one. An `f32` field is a compile error that says to use `Num`, as it is in a component.
- **Everything is checked before play**, in `test()`, in `run()` and in `cargo play --check`, in this order:
  each file against its type; each `Id` and `Prefab` it names; each asset's kind; then the game's own
  validator for the type. A mistake names the file, the field and the nearest name that exists.
- **A game's own validator** is a function the derive is told of, and that is the only link:

  ```rust
  #[derive(Content)]
  #[content(validate = WaveDef::validate)]          // fn(&WaveDef) -> thrum::Result
  pub struct WaveDef { /* as above */ }

  impl WaveDef {
      fn validate(&self) -> thrum::Result {
          if self.count == 0 {
              return Err(thrum::Error::invalid("count", "a wave needs at least one enemy"));
          }
          Ok(())
      }
  }
  ```

  ```ron
  // assets/content/waves/empty.ron
  WaveDef(enemy: "bat", count: 0, speed: 2.5, next: None, taunts: [])
  ```

  With that file in the game, `game().test()` returns an error before any tick, and `cargo play` prints it:
  the file `waves/empty.ron`, the field `count`, and "a wave needs at least one enemy". The loader calls the
  function the attribute names for every record of the type, a pack's and a test's `with_content` included,
  and calls nothing else: a method named `validate` that no attribute names is an ordinary method, and the
  compiler's unused warning is what an author who forgot the attribute sees.
- The game's content, with its prefabs, is part of the rule-set identity, so a match agrees on it before it
  starts.
- A test reads the game's real content from its folder, and
  [`with_content(name, value)`](#17-tests-without-a-window-recordings-and-pictures) adds a record with no
  file, before the checks above run.
- **Data packs** are the year-one commitment (D4): a folder of content and assets a game loads beside its own,
  adding records and variants of existing prefabs without a recompile. New mechanics stay Rust. The IDs, the
  checks and the identity above are what a pack needs, and they are in the first release.

**Saves** are what a player keeps. A save is a type the game owns, never an image of the world, so it can
outlive the build that wrote it.

```rust
#[derive(Save)]
#[save(version = 2, from = RecordsV1)]     // `impl From<RecordsV1> for Records` carries an old file forward
pub struct Records { pub best: List<(Id<LevelDef>, Seconds), 64>, pub coins: u32 }
```

*The short way: a saved resource.* `Game::saved::<Records>(Slot::new("records"))` makes `Records` a game
resource that is read before the first tick and written whenever a tick commits a change to it. Rules read and
write it as any resource, and never touch a file. A first launch starts from its `Default`. It is what a best
time, an unlock and an option need, and it is one line. **The value it starts with is part of how a run
starts**, with the seed and the first scene. A recording holds the value its run began with, and a replay
starts from that value on any device, never from the replaying device's slot, and writes no slot. In a match
the value is the host's: it is in the host's world before anyone joins, so a baseline carries it, and only the
host's machine writes the slot.

*The long way: a run saved and resumed.*

```rust
Game::new("Hollow Vale").save::<ValeSave>(capture)   // `capture` reads the world and returns the save

impl Saving {                                // in a rule: "save now", such as at a shrine
    pub fn request<T: Save>(&mut self, slot: Slot);
}
impl Saves {                                 // in a frame function
    pub fn save<T: Save>(&mut self, slot: Slot);             // the same request, from a menu
    /// `Loading`, `Missing`, `Ready(..)`, `Newer { version }` or `Failed(reason)`.
    pub fn peek<T: Save>(&self, slot: Slot) -> Loaded<&T>;
    pub fn status(&self, slot: Slot) -> SaveStatus;          // Idle, Writing, Written { at }, Failed(reason)
    pub fn delete(&mut self, slot: Slot);
    pub fn slots(&self) -> impl Iterator<Item = Slot>;
}
request.scene(Vale).restoring::<ValeSave>(Slot::AUTO);   // enter a scene with a save
fn begin_or_resume(restored: Restored<ValeSave>, ..)   // an enter function: `restored.get() -> Option<&ValeSave>`
```

- **Every request names its save type, and the type selects the capture.** `Game::save::<T>(capture)`
  registers the one capture for `T`, and a game may register several types. `saving.request::<T>(slot)`,
  `saves.save::<T>(slot)` and `.restoring::<T>(slot)` each say which, and a slot's file records the type and
  version it holds. Asking a slot for a type it does not hold is `Loaded::Failed`.
- **Out.** A save is asked for by a rule or a frame function. The SDK runs the capture function on the next
  committed tick, with [read-only parameters](#3-rules-and-the-tick), and writes the file away from the tick.
  How it went is `saves.status(slot)`, which is state and cannot be missed as an event can.
- **In: a save enters the world as admitted input, in pieces.** `request.scene(..).restoring::<T>(slot)`
  reads the file on the machine that asks. The SDK cuts the save's encoded bytes into pieces, each one action
  of an input frame, and sends them ahead of the request, over as many ticks as they take. Alone, they are
  admitted in one tick. The world gathers the pieces as world state. At the tick the
  last is admitted, every machine checks the length and the checksum, decodes the save and enters the scene
  with it. So a peer already playing receives the save as it receives any input, a peer who joins part-way
  is sent what has been gathered, a later joiner is sent the restored world, and a recording holds the
  pieces and replays the restore on any device. The scene's enter functions take `Restored<T>`.
- **In a match the pieces are budgeted against the host's whole frame.** Only the host restores, and the
  host's input for a tick is one frame that `SESSION` encodes whole into 920 bytes: a 1,000-byte datagram
  less its 64-byte header and the 16-byte prefix. The frame's record header, each action's own header and
  everything else the host sends that tick come out of the same 920, and an action carries at most 256
  bytes. So each tick the SDK first places the host's commands and answers for that tick, then fills the
  room that is left with pieces, the last of them cut to fit, and never hands `SESSION` a frame it would
  refuse. With nothing else to send that is about 850 bytes of a save a tick, in three full pieces and a
  short one. A command the host sends while a restore is under way goes first and the restore takes a tick
  longer. A piece is never sent on its own account ahead of a command the player made earlier.
- **A save has a size limit**, 64 KiB encoded unless `#[save(max_kib = n)]` says otherwise, because the world
  reserves the room it is gathered in. At about 850 bytes a tick, 64 KiB takes about 80 ticks to bring in
  to a match, a second and a third at 60 ticks a second, during which play goes on. A capture that encodes
  to more writes nothing and is `SaveStatus::Failed`. A file over the limit is `Loaded::Failed`, and nothing
  is sent.
- **A restore lands whole or not at all, and its failure never leaves the SDK's step.** When the gathered
  save fails its check, is of a newer version than the game, or an enter function that takes it returns an
  error, the world is left as it was before that tick and the tick is stepped again without the restore, so
  the scene is not entered. Both attempts are inside the one world step the driver asked for: the SDK
  undoes the first within the tick's transaction, runs the second and returns success. `SESSION` is told of
  no error, because an error from the authority's step ends the match before anything could be tried
  again. Every machine has the same input, finds the same failure inside its own step and commits the same
  tick. `saves.status(slot)` and the title screen's `Loaded` give the reason. A match is not ended by a save
  that will not load.
- Storage is each target's own: a folder on the Mac, the browser's storage in Chrome, the app's folder on the
  iPhone simulator. Chrome's is asynchronous, which is why reading is `Loaded` and not a `Result`.
- In a match, a saved resource and a save are the host's: only the host's screen may ask for a restore, and
  only the host's machine writes a slot.
- **Versions.** A save's version and its chain of older types are declared from the first release, and a test
  restores an old file. The promise that a released save loads in a later build is year one's (D4), advertised
  when a released game has kept it.

### 13. Settings and the panel

`#[derive(Settings)]` on a struct makes each field a control: a slider, a toggle, a text or a readout, with
its default beside its range. They are `thrum-controls`' typed controls ([CREATOR](../CREATOR/DESIGN.md)), so
the Mac panel, the Chrome page and parameter files work as they do today, and the profiler is in the panel of
every game.

| | A shared setting: `Game::settings::<T>()` | A local setting: `Game::local_settings::<T>()` |
|---|---|---|
| Owner | The world. `T` is a game resource | This screen |
| Read by | Rules, control functions and frame functions, as `Res<T>` | Frame functions, as `Screen<T>`. A rule cannot take it |
| Fields | What a resource may hold. A raw float is a compile error that says to use `Num`, or to make the setting local | Those, and `f32` |
| A panel edit | Enters as a command and changes the world at a tick. A replay holds it | Changes this screen at once. It never leaves the machine |
| In a match | The host's edits are the ones admitted | Each player has their own |

### 14. The interface

A frame function describes the interface with `Ui`. The SDK keeps [GAME-UI](../GAME-UI/DESIGN.md)'s tree and
changes only what differs from the last frame, so an interface that did not change costs one draw.

| | |
|---|---|
| Places | `top_left`, `top_center`, `top_right`, `left`, `center`, `right`, `bottom_left`, `bottom_center`, `bottom_right`; `ui.modal(title, ..)`; `ui.at_world(point, ..)`, a place that follows a point in the world |
| Layout | `ui.row(..)`, `ui.column(..)`, `ui.grid(columns, ..)`, `ui.space(..)` |
| Showing | `text` (wrapped to its place), `title`, `bar`, `image`, `time(seconds)`, `status(..)`, `prompt(Controls::JUMP)` (the key or button a player has bound), `toast(text, seconds(2))`, `fade(colour, amount)` over the whole screen |
| Doing | `button(label).clicked()`, `card(title, text).chosen()`, `toggle`, `stepper(label, value, range)`, `copyable(text)`; `.enabled(false)` and `.locked()` on any of them; `.key(k)` |

- A control is the same control from one frame to the next by its place in the description. Controls made in
  a loop take a `.key(..)` that stays with what each stands for, so focus and a press under way follow it.
- **A modal** is an exclusive scope: while it is open this screen's
  [input context](#6-players-actions-and-control) is off, by the rule for any change of context. What the
  context's maps held is let go, a key held since before the modal opened drives its action again when it
  closes, and a key that went down inside the modal does not. Local actions with no context are still read,
  so the key that opened a pause menu closes it.
  `.pausing()` holds [the pause](#4-scenes-phases-and-time) while it is open.
- Focus, direction keys, touch sizes, safe areas and density are `GAME-UI`'s and need nothing from the game.
  The SDK's default font and sizes ship with it, so a first game needs no font.
- A text field and a scrolling list are not in `GAME-UI`. Their shapes are [reserved](#reserved-shapes).

### 15. Multiplayer as an opt-in

`Game::multiplayer(Multiplayer::players(1..=4))` with the `multiplayer` feature (D3). The SDK then implements
`SESSION`'s world for the game: the rule-set identity from the game's rules, content and build and from every
starting value its description registers (resources, prefabs and shared settings' defaults), the world image
and digest from the registered state, the action wire from the action maps and commands, the roster as
[seats](#6-players-actions-and-control), and the driven world. A game writes none of the steps of [the session guide](../../SESSION.md).

- **Who does what** is `SESSION`'s and unchanged: a native process hosts, browsers and the simulator join, the
  world is shown at the confirmed tick, and a match ends with its host.
- **The local player's own character answers at once** when its control function
  [can be run ahead](#6-players-actions-and-control). The SDK gives that function to `SESSION`'s predicted
  controller as its step. The controller's state is everything the next step depends on: the character's
  entity and the components the function names; **the clock the function can read**, which is the game's
  tick and the ticks at which the running scene and its phase were entered; this player's input state, which
  is each button's held state and last count; whether the match is paused; and what the function reads of
  the confirmed world, which is the resources it names, the phase, read by a `.during(..)` line too, and the
  level's identity. Content and the seed are the rule set's constants. `SESSION` asks that every dependency
  of the step be in its state or its constants, and this is how.
- **The two entries are in the state because `Tick` counts from them.** `tick.in_scene()`, `tick.in_phase()`
  and `Phase::time()` are the game's tick less an entry. A peer who joins at tick 300 reads both entries
  from the world it is sent, so a controller that lets its hero move two seconds into the scene steps a
  scene entered at tick 100 and one entered at tick 290 differently, as the host does. Entering the same
  phase again changes its entry, so the confirmed state differs from the predicted one and the step is
  corrected, as for any other change of what it reads.
- **The whole state is compared with the host's, bit for bit**, as `SESSION` compares it, and not the
  character's components alone. A step run ahead carries what it read of the confirmed world forward
  unchanged, so when the host changes any of it (a shared setting, the phase, the level, a pause) the
  confirmed state differs from the predicted one even where the character has not moved. The controller then
  returns to the confirmed state and replays its unconfirmed input with the new value. A hero standing still
  when the host raises the run speed is corrected with nothing to see, and its next step is at the new speed.
  So what is promised is convergence, and its terms are these. **A step run ahead equals the host's step of
  that tick when two things hold: the host admitted, for that tick, the frame this machine stepped with, and
  nothing the step reads had changed on the host that this machine had not yet been sent.** Where either
  fails, the tick is corrected when its confirmation arrives: a frame the host held or replaced because it
  came late, and an edit the host made a moment ago, are both ordinary causes, and each can happen again at
  any later tick. After a correction, the steps run ahead from the confirmed state are equal to the host's
  under the same two terms.
- **The predicted step sees the clock the host's step sees**: the state carries the tick and both entries,
  so `tick.every`, `tick.in_scene()`, a `Timer` and a `Cooldown` are predicted as the host steps them, from a
  baseline at any tick.
- **What the local player's screen shows.** Their own character is shown at the predicted pose, in a frame
  function's query and in the draw alike, and a camera following it moves with it. Everything else is shown at
  the confirmed tick. When the host's result differs, the controller returns to the confirmed state and
  replays, and the SDK eases the shown pose to the corrected one over a few pictures, except across a
  `teleport`.
- **Under a bad link.** A player whose input is late keeps their last axes for a moment and then idles, and
  their `PlayerStatus` says so. Nobody else waits for them. A player whose link drops is `Away` for
  `Multiplayer::grace(seconds(30))`, keeps their seat, score and character, and is `Left` after it. A press is
  [read once](#6-players-actions-and-control), late and never twice.
- **Seats, bots and joins** follow [one policy](#6-players-actions-and-control): a person takes a free seat,
  and where bots hold every free seat takes a bot's. `SESSION` admits a connection to a slot of its own table,
  which knows no bots and not how many seats the game asked for, and tells the world who joined at a tick.
  The seat that slot's player takes is the SDK's choice, made from world state at that tick, and so is
  taking none: a match of fewer than four seats is kept to its number by the SDK, which hands `SESSION` no
  link while people hold every seat and none is away, and gives no seat to a slot the world has no room
  for. An away player's return is `SESSION`'s, by token, and finds the seat the SDK kept.
- **A pause** is world state over ticks the host goes on committing
  ([pause](#4-scenes-phases-and-time)), so anyone the game allows can end one from any machine and `SESSION`'s
  clock is used as built.
- **The lobby** (R8). `Game::lobby(Lobby::standard().starts(scene))` is a scene the SDK supplies, described in
  [example F](#f-the-same-game-online-with-bots), and the first scene of a game that has one. **It is an
  ordinary scene on the match's clock.** When a player chooses *Host*, the SDK starts `SESSION`'s host and its
  ticks at once, with the lobby as the running scene. `SESSION`'s own wait before a first tick is never where
  players gather, and its flag for a peer that has finished loading is not a player's ready mark. A peer who
  joins is sent the lobby as it stands, as a late joiner of any scene is. So everything done in the lobby
  reaches the host as input at a tick and is world state:
  - `.ready_up()` gives each player a ready mark, which they set and clear themselves with a command of the
    SDK's. *Start* is then offered to the host once every player here is ready.
  - `.host_panel(pick_arena)` shows the host a panel over the lobby's setup, a `Data` value of the game's:
    `fn pick_arena(ui: &mut Ui, setup: &mut Setup)`. The SDK hands it the setup as the world has it and sends
    what it changed as the host's command, so every screen shows the host's choice.
  - *Start* is the host's command. `.starts(Arena)` enters that scene, and `.starts_with(arena_from)` makes
    the scene from the setup, `fn arena_from(setup: &Setup) -> Arena`, so the host's choices are the fields
    the scene is entered with.
  - *Play alone* is the same lobby with no session: one player, who presses *Start*.
- **`Session`**, in a frame function, is for a game with its own screens: `state()` is `Alone`, `Hosting {
  ticket, players }`, `Joining { progress }`, `Playing` or `Closed { reason }`, with `host()`, `join(ticket)`,
  `leave()`, `can_host()` and `dropped_ticket()`.
- **Not offered in the first release**, said plainly: a player's shots and the players they touch are not run
  ahead; a hit is decided at the shared tick, not rewound to what the shooter saw; there is no host
  migration; and 3D games are not proved in a match. The first three have [reserved shapes](#reserved-shapes).

### 16. Mistakes that explain themselves

Every message names the object, the cause and the next step, and has a stable code (`T0412`) that the guide
*When something goes wrong* explains with a failing example and its fix. `cargo play --check
--message-format=json` gives each as a record with its code, source span and suggestion, for an agent.
Compile-time messages are written on the SDK's own traits, so they are the compiler's errors with the SDK's
words.

| When | Mistake | The message says |
|---|---|---|
| Compile | A rule takes `Ui`, `Screen<T>`, `Shown<L>`, `Draw2d`, `Outbox`, `Me`, `Pause` or another frame parameter | That a rule must give the same result on every machine, and to move this to a frame function |
| Compile | A frame function takes `ResMut<T>`, `Entities`, `Emit<T>`, `PhaseMut<P>`, `PlayersMut` or a `&mut` query | That frame functions cannot change the world, and to send a command or write a rule |
| Compile | A spawn function, a condition or a capture function takes a parameter that writes, such as `PlayersMut`, `ResMut<T>` or `Entities` | That it may only read, and which parameter to change |
| Compile | A frame function asks for `&Transform2d` or `&Transform3d` | To ask for `&Pose2d` or `&Pose3d`, the pose as this picture shows it |
| Compile | World state holds a `Vec`, a `String`, a reference, a raw float or `usize`; a content record holds a raw float | What to use: `Num`, `Text<N>`, `List<T, N>`, content, or a local setting |
| Compile | A spawn of a `Prefab<N>` without its `N`; `Sprite::new` given a mesh; a field of an action map with no binding | Rust's own type error, with the SDK's note on what is missing |
| Compile | The first scene listed has no default | To list another first or derive `Default` |
| Check | Raw-float arithmetic, an unordered container, a file, a clock or an outside crate in the `rules` module; a path from it to the rest of the game | The line, why it can differ between machines, and what to write |
| Startup | One parameter names a component twice and writes it, such as `Query<(&mut Health, &Health)>`, or a control function takes `&mut T` and `&T` of one component | The function, the parameter and the component, and to drop the reader |
| Startup | Two parameters of one rule name the same component of one prefab, and one of them writes it; a control function's query reaches the character its line spawns, over a component either writes | The rule, the parameters, the component and the prefab |
| Startup | A rule reads a resource nothing provides | The rule, the resource, and `scene.resource(..)` or `Game::resource(..)` |
| Startup | A rule, a control function, a spawn function, an enter or exit function, a condition or a lobby's `starts_with` function is defined outside the `rules` module, in a game that has one | The function and the module |
| Startup | The action maps do not fit an input frame; a command type is too large | The counts or the size, and the limit |
| Startup | A prefab name declared in any `prefabs!` block the game's code refers to has no definition, whether a rule's body, a base, a child or a part uses it and whether or not the description names its block; a definition names an asset that does not exist; a content file fails its type, names a record that does not exist or fails the game's validator | The prefab or the file, the field, and the nearest name |
| Startup | With multiplayer on: the game has no `rules` module; a control function reads what cannot be run ahead | How to move the rules; the parameter and `.unpredicted()` |
| Startup, in `run()` | No asset store, a stale asset catalog, or a missing prerequisite | What is missing and the command that provides it |
| Running | A spawn of a component the world does not know | The rule, the component, and the prefab or `Game::component` |
| Running | `Single` finds none or two; `entities.part` on a prefab with no such part; `level.mark` finds none or two | The rule and what was found |
| Running | An entity comes to match two parameters of one rule that name the same component, one of which writes it; a player's character, by a change of its components or by `possess`, comes to be reached by a query of its own control function over a component either writes | The rule, the parameters, the component and the entity, and for a character its player |
| Running | `keep_world` under a parent whose scale through its parents is zero or outside 1/10,000 to 10,000, where the result would need shear, or where the recomposed place is past the bound | The child, the parent and the cause, with the residual and the bound; the child is where it was |
| Running | A level is given more fixed shapes than its recipe holds | The count and `scene.level_shapes` |
| Running | `go_to` a scene the game was not given; a `request.scene` the running scene does not accept | The scene and the line to add |
| Running | Two characters for one player on one `players()` line | The line, the player and both entities |
| Running | A spawn past the world's size | The size and `Game::max_entities` |
| Running | A rule returns an error, or panics | The rule, the tick and the error, [as each target allows](#3-rules-and-the-tick) |
| Running, on screen | An entity with a look in a scene with no camera, or a 3D look under a 2D camera | Once: the scene, the look and `scene.camera(..)` |
| Running, on screen | A named asset that matches nothing | The name and the nearest names |
| Running, on screen | A session is refused or closed, or the match has no seat for this player | `SESSION`'s reason, or that the match is full, in words a player can read |

A message says only what its check knows. The startup checks run in `test()` and in `run()` alike, before the
first tick, and a failed one is the error `test()` and `run()` return; the rows marked `run()` and "on screen"
need a window or an asset store and run in `run()` and `cargo play --shot`. `thrum::Error` is one enum with a variant for each kind of remedy, in
[the adopted error shape](../../../.agents/skills/thrum-engineering/references/patterns.md).

### 17. Tests without a window, recordings and pictures

`game().test()` returns a `thrum::Result<TestGame>`: the game with no window, device or clock, on the same
rules and the same engine path as `run()` (R9). A game played alone has its one player from the start, with
their character.

**Starting is fallible, and fixtures go in before it.** `test()` is `test_setup().start()`:

```rust
impl Game {
    pub fn test(self) -> thrum::Result<TestGame>;
    pub fn test_in<S: SceneType>(self, scene: S) -> thrum::Result<TestGame>;
    pub fn test_setup(self) -> TestSetup;
}
impl TestSetup {                                   // each returns the setup, to chain
    pub fn in_scene<S: SceneType>(self, scene: S) -> TestSetup;
    pub fn with_content<T: ContentType>(self, name: &str, record: T) -> TestSetup;   // a record with no file
    pub fn with_saved<T: Save>(self, value: T) -> TestSetup;       // what a saved resource starts as
    pub fn with_save<T: Save>(self, slot: Slot, save: T) -> TestSetup;   // a slot's file, for `restoring`
    pub fn seed(self, seed: u64) -> TestSetup;
    pub fn start(self) -> thrum::Result<TestGame>;
}
```

`start` does these in order, and the first failure is the error it returns: it installs the fixtures; checks
the content, the fixtures' records included; runs every [startup check](#16-mistakes-that-explain-themselves);
gives each saved resource its fixture or its `Default`, since a test reads and writes no real storage; and
enters the first scene, which sets its resources and runs its enter functions and its `players()` lines for
the one player. So an enter function that reads content or a saved resource finds the fixture, and a game
that fails a startup check never runs one. No tick has been stepped when `start` returns.

| | |
|---|---|
| Start | `test()`, `test_in(scene)`, and `test_setup()` with `in_scene`, `with_content`, `with_saved`, `with_save`, `seed`, then `start()` |
| Time | `step()`, `step_ticks(n)`, `step_for(seconds)`, `step_until(limit, \|game\| ..)`, which fails with the last tick it reached and never waits on a clock; `rewind(ticks)`; `digest()` |
| Players | `join()`, `leave(p)`, `away(p)`, `back(p)`, `add_bot()`; every input call is for the first player unless `.player(p)` says otherwise |
| Input | `hold(Controls::JUMP)`, `release(..)`, `press(..)` (down for one tick), `steer(Controls::RUN, 1.0)`, `send(command)`, `choose(value)` for an open offer; `stall_input(ticks)`, which loses the player's frames for that long as a bad link does: their last frame is repeated, then the neutral one |
| Finding | `character(player)`, `single::<F>()`, `find::<F>()`, `nearest::<F>(point)`, `children(e)`, `part(e, name)` |
| Reading | `get::<T>(e)`, `resource::<T>()`, `phase::<P>()`, `scene::<S>()`, `in_scene::<S>()`, `count::<F>()`, `alive(e)`, `world2d(e)`, `events::<E>()` since the last call, `offer::<T>(player)`, `missing_assets()` |
| Changing | `spawn(..)`, `despawn(e)`, `set(e, component)`, `set_resource(value)`, `go_to(scene)`, `go_to_phase(p)` |
| Saves | `save::<T>()`, which runs the capture; `restore(scene, save)`; `saved::<T>()` |
| The screen | `screen()`, or `screen_of(player)`: a `TestScreen` |

**What the lookups return, and when a join lands.** A test must be able to tell "not there" from "not yet".

| Call | Returns | Notes |
|---|---|---|
| `join()` | `thrum::Result<Player>` | The seat is known at once, a bot's where bots hold every free one. The player enters the game at the next `step()`, where `PlayerJoined` is read and their character is spawned. The error is that the game is full: people, the away among them and joins still waiting for their tick, hold every seat the game uses. Nothing has changed when it is returned |
| `character(player)` | `thrum::Result<Entity>` | Three different errors: the join is still waiting for its tick; the player is here with no character, because a rule despawned it or the scene has no `players()` line; no such player |
| `single::<F>()` | `thrum::Result<Entity>` | An error for none and for two, with the count |
| `find::<F>()`, `children(e)` | `Vec<Entity>` | In spawn order, and in the order attached. Empty when there are none |
| `nearest::<F>(point)` | `Option<Entity>` | |
| `get::<T>(e)`, `resource::<T>()`, `scene::<S>()`, `phase::<P>()` | `thrum::Result` of a copy | An error when the entity is gone or has no `T`, nothing provides the resource, or another scene is running |
| `alive(e)`, `in_scene::<S>()` | `bool` | |
| `count::<F>()` | `usize` | |
| `spawn(..)`, `despawn(e)`, `set(..)`, `set_resource(..)`, `go_to(..)`, `go_to_phase(..)` | `thrum::Result`, of the `Entity` for `spawn` | They act between ticks and have landed when they return. `go_to` enters the scene at once, as `start` does |
| `step()` and the other time calls | `thrum::Result<()>` | The error of a refused tick, after which the game can be stepped again |

A match's calls are fallible in the same way, which is why example F writes `?` on each:

| Call | Returns | Notes |
|---|---|---|
| `game().test_match()` | `thrum::Result<TestMatch>` | Runs what `test()` runs before the host's first tick, and returns the same errors, with the refusal to start a match that [multiplayer adds](#16-mistakes-that-explain-themselves) |
| `TestMatch::join(link)` | `thrum::Result<Peer>` | The handle `peer(..)` takes, at once. The error is that the match is full: the host's SDK turned the link away before the handshake, because people hold every seat and none is away. A link the host lets through returns its peer, seated or not |
| `peer(p).my_character()` | `thrum::Result<Entity>` | `character`'s three errors and two more: the peer is still loading the world; the peer was given no seat and has left, having arrived while an away player's seat was kept, or beside another newcomer for the last one |

- **`TestScreen`** runs the scene's frame functions for one player against the world as it stands, with no
  renderer: `click("Resume")`, `choose(key)`, `press(Menu::PAUSE)`, `has_text(..)`, `find(key)`, `camera2d()`,
  `bursts()`. So a test can open a pause menu, pick an upgrade card and see `offer.status()` change.
- **`game().test_match()`** returns a result of a `TestMatch`: a host and its peers in one process, each with a world of
  its own, stepped in simulated time over `SESSION`'s rehearsal. `join(Link::delayed(..))`, `peer(p)` with a
  `TestGame`'s input calls and `shown::<Pose2d>(e)`, `host()`, `disconnect(p)`, `reconnect(p)`,
  `digests_agree()`. It proves a late join, a slow peer, a choice made twice and a dropped link.
- `Tick::test(number)` makes a tick for a unit test of a pure function.
- **A recording** is the input a run admitted, with the game's seed, the scene it started in and the value
  each saved resource started with. A save restored during the run is in that input, as its pieces. `cargo play --record <file>` writes one while a person plays. `game.play_recording(file)` replays
  it in a test and leaves the game where the run ended, and `cargo play --replay <file>` shows it. A recording
  made on one device replays on another (D1). After a change to the rules it replays the same input against
  the new rules, which is what makes it a test that a level is still beatable.
- **A picture.** `cargo play --shot <file>` runs a game offscreen for a number of ticks, under a recording if
  one is given, and writes the frame the engine submitted, through
  [T2 capture](../T2-CAPTURE/DESIGN.md). It needs no screen and no permission.

### 18. Documentation, guides and templates

- **Reference.** Every public item of `thrum` has a doc comment with an example, and every example is a doc
  test. `./dev check` fails a public item without one. The example is on the item an editor's completion
  finds.
- **Guides**, in `docs/sdk/`: *Start here* (the four sentences and the fifteen words); *Your first 2D game*,
  whose first page ends with a ship you steer; *Your first 3D game*; *Make it multiplayer*; *Numbers* (the
  page of [mechanism 9](#9-numbers)); *How do I…* (short answers, each a few lines of code, by task); *When
  something goes wrong* (by code). A guide teaches only what is built, and each code block in a guide is
  compiled by the docs check.
- **One file for agents.** `docs/sdk/REFERENCE.md` is generated from the reference and the guides. The docs
  check fails when it is stale.
- **Templates**: `2d`, `3d` and `multiplayer`. `./dev new <folder> --template <name>` writes a game that
  builds, runs and passes its check as written, with `src/rules.rs`, a test, and an `AGENTS.md` that points an
  agent at the one file. A game made this way lives outside the engine checkout and reaches `thrum` by path.
- **Judged by a change the guides did not show**, as well as by a clean start: a second enemy variant, a
  reconnect during a choice.
- The engine's own guides stay as the reference for what is under the SDK.

### 19. Running a game

One command for each target, the same for every game, from the game's folder. The template ships them as Cargo
aliases onto the engine's tool.

| Command | What it does |
|---|---|
| `cargo play` | Cooks and watches `assets/`, writes the asset catalog, checks the rules, builds, opens the Mac window with its panel |
| `cargo play --web` | The same for Chrome: builds for wasm32, serves the page and the store on loopback, prints the URL |
| `cargo play --ios` | Builds and starts the game on the iPhone simulator, through [GAME-UI](../GAME-UI/DESIGN.md)'s one shell |
| `cargo play --check` | The rules check and every startup check, with no window. `--message-format=json` for an agent |
| `cargo play --record <file>`, `--replay <file>` | [Records a run, and shows one](#17-tests-without-a-window-recordings-and-pictures) |
| `cargo play --shot <file>` | Writes a picture |
| `cargo play --scene <name>` | Starts in another scene, with its default value |

The game's `main` is the same file on every target. The SDK supplies each platform's entry.

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
| Which scene and phase run, the tick each was entered at and the transition asked for; the seats, with who is a bot, each button's last count and any slot given no seat; whether the match is paused and the game's tick; open offers; the level's recipe, with its tile map's identity; the pieces of a save being brought in | The world, as resources the SDK registers | Keeps them as world state, so they are digested, rewound and sent to a late joiner |
| Events for the frame, effects under way, `Screen<T>` with the input context, `Shown<L>`, bindings, the built collision of each recent level recipe | The SDK, as this screen's or this machine's state. It is not world state and is in no digest | The ring of recent ticks' events, each cursor, the count of each control line's emissions shown for each recent tick, the bursts still playing; on each machine, whether its screen shows the pause, which decides what its sampler counts |
| Saves and recordings | The SDK, in each target's storage | Runs the capture on a committed tick, writes away from the tick, brings a save in as input |
| The game's scenes, rules, looks and content | The game | — |

### A tick, walked

1. The driver (the fixed clock alone, the session in a match) names a tick to commit. Alone, while the game is
   paused it names none.
2. Input is admitted: each player's action maps, with each button's presses counted from its running count;
   the commands and answers for this tick; the roster's events, seated by the one policy; a requested scene;
   the pieces of a save. A joined slot the world has no seat for is marked unseated. In a paused tick of a
   match only the roster's events and the pause's end are taken, no button or axis is read, and the walk
   ends here with the game's tick unmoved.
3. The scene's written order is walked. Each entry runs with the world as the last one left it, and when it
   finishes its structural edits and its changes to the level are applied, new sets of components are tested
   for conflicts, and the layers it changed are marked for rebuilding.
4. The tick's one transition lands, if one was asked for: a change of phase or a change of scene, whichever
   was asked first. Exit functions, the leaving scene's entities, enter functions. A request one of those
   functions makes is held for the next tick, except a request for a phase of a scene that is being left,
   which is refused.
5. The engine checks every number is finite and commits. A refused tick leaves the world as it was. A
   restore that fails is undone here and the tick stepped again without it, inside this same step.
6. The tick page is published, and with it the tick's events for the frame functions that read them. Only the
   rows that changed are extracted to sprite and entity rows. A save that was asked for is captured.

### A frame, walked

1. The shell is asked for a frame and samples the bindings, the local actions and the pointer.
2. The engine chooses the pair of ticks and the fraction for this picture and samples the scene. The camera
   rigs move. The frame functions run then, on that pair at that fraction.
3. The interface is reconciled with last frame's and laid out only if a box could have moved.
4. The engine draws from the same pair at the same fraction: the scene's passes for its kind, interpolating on
   the GPU, then exposure, tone, the interface stage and present.
5. The SDK reads whether the picture was submitted or dropped. A dropped picture runs nothing again.

Rules run on the world's own thread where the shell has one, as today. Frame functions never wait for them:
they read published pages, on the thread that samples the scene.

## Interfaces

**Provided.**

| Interface | To | What it is |
|---|---|---|
| **The `thrum` crate** | Every game, the kit games, the templates | Mechanisms 1 to 17 |
| **The game commands** | Every game | Mechanism 19, and `./dev new` |
| **The SDK guides and the one file** | Game makers and their agents | Mechanism 18 |

**Asked of other modules.** Each is new work inside state another module owns, built by this module's builder
under [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk), on what the owner has landed, and the
owner's afterwards. No contract's text changes. C01's stable identities, C02's committed state, C11's event
order and C13's authority are what the mechanisms above stand on.

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
   storage for saves on each target; a driver that can stop stepping; the game commands in `./dev`.
4. **The interface and the match** (`GAME-UI`, `SESSION`): used as built. A press count and a sampled bit on
   the input frame's axes; the predicted controller's whole state carrying what the step reads, compared as
   `SESSION` compares it; seats, with bots, that the SDK keeps in world state over `SESSION`'s slots; a lobby
   that runs on `SESSION`'s ticks from the moment of hosting; a pause kept as world state while the host goes
   on committing; a save brought in as pieces budgeted against the host's whole 920-byte frame, with a
   failed restore retried inside the world's one step; a match of fewer than four seats kept to its number
   by the SDK, which is what hands `SESSION` each link and what seats each joined slot; a note of a step's
   emissions held in the `Rules` value the SDK gives the predicted controller; and a player's name sent as
   their first command all fit what `SESSION` has, and none asks it to change.
5. **The repository guard** (`REPO-RULES`, `GAME-SPLIT`): `thrum` registered, and
   [the rule](#1-one-crate-one-package) that every package of a game names no engine crate but `thrum`.
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
| A capability left out | Nothing: a 2D game compiles no 3D pass, a game with no interface has no stage, a game without the `multiplayer` feature links no network crate |

**Tripwires**, recorded and investigated while building goes on; none is a pass condition beyond a missing
measurement or a collapse:

- the reference crowd's behaviours at 10,000 bodies as SDK rules cost about 25% more a tick than the
  hand-written reference, the brief's figure for a comparable regression;
- any allocation in a warmed tick or frame of the 2D starter that the same scene on the engine's API does not
  make;
- a frame of the showcase on the SDK that costs about 25% more than the same frame on today's API.

## Reserved shapes

What the first release does not build, designed so that it fits when it comes. Nothing here is built, and no
guide teaches it, until [the owner says so](#scope-questions-for-the-owner) and an engine module provides it.

**3D space and a 3D character.** The 2D vocabulary with a 3 on it.

```rust
Collider3d::sphere(r) | ::capsule(r, height) | ::cuboid(x, y, z)      // .offset(..) .on(LAYER)
impl Space3d {  // touching, within, nearest, motion_of and at_tick_start, as Space2d has, and:
    pub fn ray(&self, from: Vec3, to: Vec3) -> Swept3d;                // .on(LAYER).first_hit()
    pub fn sweep_sphere(&self, from: Vec3, to: Vec3, radius: Num) -> Swept3d;
    pub fn move_capsule(&self, from: Vec3, shape: Capsule, step: Vec3) -> Move3d;
}
impl Level3d {  // the scene's fixed collision: placed models that say `.solid(LAYER)`, and the ground
    pub fn move_capsule(&self, from: Vec3, shape: Capsule, step: Vec3) -> Move3d;   // a match can run it ahead
    pub fn ray(&self, from: Vec3, to: Vec3) -> SweptLevel3d;   // a hit names a layer, as `Level2d::ray`'s does
    pub fn ground_height(&self, x: Num, z: Num) -> Num;
}
// Move3d: .max_slope(angle) .step_up(height) .on(layers) -> Moved3d, which has what Moved2d has
scene.ground(Ground::heights(art::terrain::VALE));     // one heightfield the engine draws and rules stand on
scene.place(model).solid(SOLID);                       // a placed model that is also fixed collision
Camera3d::orbiting_my_character().avoiding(SOLID);     // a boom that shortens at a wall
```

**Gamepads.** More bindings on the same maps, and focus in the interface by pad: `Pad::LEFT_STICK`,
`Pad::RIGHT_STICK_X`, `Pad::SOUTH`, `Pad::START`. No rule changes.

**Audio.** A frame function plays a sound, as it draws a burst: on an event, for this screen.

```rust
fn sounds(did: Events<Jumped>, mut audio: Audio) {
    for jump in did.read() { audio.play(art::sounds::JUMP).at(jump.at); }
}
audio.music(art::music::VALE).fade_in(seconds(2));
audio.looping(art::sounds::HUM).following(warden).volume(0.6);
```

**Predicting more than the character.** A control line names what runs ahead with it:

```rust
scene.players().spawn(CRAFT).control(drive)
    .predicts(Predict::spawns(BOLT).moved_by(fly_bolts))   // my bolts leave my muzzle at once
    .against(CRAFTS);                                       // and my craft meets the others where I see them
```

**Hits rewound to what the shooter saw.** The input frame carries the tick the shooter's screen showed, and a
space keeps a short history: `space.as_seen_by(player).motion_of(bolt).sweep().on(CRAFTS)`.

**Skeletal animation.** `Model::new(..).rig(art::rigs::PILGRIM)`, with `shown.of(entity).play(clip)` as sprites
have.

**A text field and a scrolling list.** `ui.text_field(label, &mut text)` and `ui.scroll(..)`.

**A host with no window.** `cargo play --host` runs the game's rules as a match's host with no screen, so a
match need not end when a player's machine leaves.

**Levels from an editor's file.** `Tilemap` and a list of placements cooked from an LDtk or Tiled file, and
from the nodes of a glTF scene, with the editor's names mapped to prefabs.

## Scope questions for the owner

**Not decided here.** The owner's first-release scope (O4) stands until the owner changes it. Each line is one
question, with what the dogfood games showed and the planner's recommendation. The design above works either
way: a "yes" adds work packages and an engine module, and changes no shape.

1. **3D collision queries and the 3D character move in the first release?** Both 3D games wrote about 450
   lines of their own physics, and both named it the first reason to choose Godot. Without it the 3D SDK
   serves showcases and not games. **Recommended: yes, as queries and a kinematic move only, with no rigid
   bodies.** It needs a deterministic 3D index and fixed collision from meshes and a heightfield, as an engine
   module of its own.
2. **Gamepads in the first release?** A platformer and a third-person game are played on one. **Recommended:
   yes.** The bindings are designed. It needs pad input in the shells and focus by pad in `GAME-UI`.
3. **Audio in the first release?** All three games named it as the gap between a prototype and a game.
   **Recommended: yes, the small form above** (one-shot, looping and music from frame functions), behind an
   engine module. No agent can hear it, so its items wait for the owner's ear.
4. **Predicted projectiles and predicted contact between players in the first release?** The arena shooter
   could not ship without them. The swarm game, a co-op crowd, does not need them. **Recommended: no, after
   the first release**, with the shape reserved.
5. **Hits rewound to what the shooter saw?** Only a competitive shooter needs it. **Recommended: no, after the
   first release**, and the design says plainly that hits are decided at the shared tick.
6. **Multiplayer for 3D games in the first release?** It follows question 1: with a 3D character move, a 3D
   control function runs ahead as a 2D one does. **Recommended: no for the first release**, proved in the
   release after, because the first release proves a match in 2D for the swarm game.
7. **A host with no window?** `SESSION` already hosts headless. **Recommended: yes**, as one command in the
   online work package.
8. **Skeletal animation** stays out (O4). The exploration game's verdict was that a figure with no animation
   reads as a prototype. **Recommended: keep it out of the first release**, and plan it first among the
   engine modules that follow.
9. **A text field and scrolling lists?** A player's name comes from the system account until there is a field.
   **Recommended: no for the first release.**
10. **Levels from an editor's file (LDtk, Tiled, glTF placements)?** Hand-typed tile rows do not scale past a
    demo, and D2 rules out an editor of Thrum's own, not reading someone else's. **Recommended: after the first
    release, with data packs**, since both are content pipelines.

## Physics, animation and audio

Decided by the owner on 25 November 2026 ("Planner's default"), and unchanged here: 2D queries and sprite
animation are in the first release; rigid bodies, skeletal animation and audio are out.

| | In the engine today | The first release |
|---|---|---|
| **Physics** | 2D overlap, nearest and sweep queries on circles (`SIM-2D`). No rigid bodies, no joints, no 3D collision | **2D queries in, rigid bodies out.** This design adds, within 2D queries, rectangles and capsules, a level's fixed collision and a character move that slides and reports its contacts, because a platformer cannot be written without them. Nothing pushes anything. 3D is [question 1](#scope-questions-for-the-owner) |
| **Animation** | Sprite frames by name (`RENDER-2D`). No skeletons | **Sprite animation in**: a clip a screen plays (`Shown<Sprite>`) and a clip that loops in step for everyone (`Sprite::looping`). **Skeletal animation out** |
| **Audio** | Nothing. A1 is a design | **Out.** Its shape is reserved, and it is [question 3](#scope-questions-for-the-owner) |

## The work packages

Thirteen, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds
the items of the first three. Numbers are names, not the order: WP3, WP5, WP6 and WP7 keep the numbers other
modules already wait on.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Mechanisms 2, 3 and 4 with no window, for [the headless subset](#what-wp1-compiles): the world, rules in written order with each rule's edits landing when it ends, scenes as types, phases, timers, the test harness; the surface and documentation checks | Nothing |
| WP11 | Mechanism 5: prefab names in `prefabs!` blocks that the program lists for the SDK, variants and what a spawn must bring; children, parts, attaching and disabling, with turns that are quarter turns; `keep_world` with its bound and its refusals; the access check made on what exists | Nothing |
| WP2 | Mechanisms 6 and 9 and `Space2d`: the numbers page and the rules check; action maps and how a button's presses are counted, players and seats, a controlled character, bots; the crowd on the SDK, equal on three targets, with its cost | `SIM-2D:WP3`, Done |
| WP12 | The rest of mechanism 8 and content: the level's recipe and fixed collision, tile maps, the character move; typed content, its validator and IDs; and a shared setting as a game resource with its defaults, which rules read and a test sets, with no panel. Example D's rules, proved with no window | Nothing |
| WP3 | 2D on screen: mechanisms 10, 11 (2D), 19 and the asset catalog for the Mac and Chrome, with the first `run()`, cameras and rigs, `Shown`, local actions, input contexts, the pointer, `Pause` alone and `--shot`. Rockfall and Skylark without their interface: the ships, the rocks, the crash and the change of scene; the level, its camera, the hero's clips and the dust | Nothing still open |
| WP4 | 3D on screen: mechanism 11 (3D), with entities' models drawn as entity rows that turn, scale and take a texture, placements, and the rest of mechanism 13: local settings and the panel, and a shared setting's panel edit admitted at a tick. The starter, stones and the lighthouse are written on the SDK as new kit games | For its change to the instance passes, the V1 work packages that hold them |
| WP6 | Mechanisms 7 and 14: the interface, modals, commands, offers with a bot's answer, and requests, and `TestScreen`. The interface of Rockfall (its score and result page) and of Skylark (its pause menu) | `GAME-UI:WP4` |
| WP13 | Saves and recordings: a saved resource, a run saved and resumed alone, `--record`, `--replay` and `play_recording` | Its own WP6 |
| WP5 | Mechanism 15: the crowd hosted and joined, a control function run ahead with what it reads, the standard lobby on the match's clock, seats and bots in a match, a pause in a match, a save restored in a match, `TestMatch` | `SESSION:WP7` |
| WP7 | The iPhone simulator | `GAME-UI:WP6` |
| WP8 | Mechanism 18, [the guard](#1-one-crate-one-package), and three fresh agents | Its own WP1 to WP7 |
| WP9 | The showcase on the SDK, in the games repository | `GAME-SPLIT:WP4`, which follows the V1 exit |
| WP10 | The old creator route removed | Its own WP9 |

Each work package's surface arrives with its reference documentation and its rows of
[the mistakes table](#16-mistakes-that-explain-themselves), because WP1's guard holds every later one to them.
A work package is compared with the worked examples only for the surface it builds.

What the planner who gives a later work package its items carries into them:

Each proof is listed under the first work package that has everything it uses, so no item waits on a later
one. WP12's use only WP1, WP11 and WP2 and what WP12 builds; WP3's use no command and no `Ui`; WP6's use no
match; and what needs a match is WP5's.

- **WP12:** a level loaded from a content file in an enter function; a box that lands on a floor, stops at a
  wall, passes up through a one-way tile and stands on it; `moved.slide`; a `teleport` that sets the place
  and marks the tick; a `ray` that hits a wall of the level and reads its layer; a tile map from a file and a
  rectangle a rule added later, both giving the same `move_box` and `ray` answers after a rewind and a
  replay; one content record that holds two tile maps with a wall in different places, and a list of them,
  where loading each gives its own wall to `move_box` before and after a rewind to a tick at which the
  other was loaded; a content record refused before play by its type and by the game's validator; rules that read a
  shared setting registered with its defaults and set by the test; and example D's two coyote tests, the
  jump inside the window and the press after it that does not jump.
- **WP3:** a picture the engine encodes and drops, with an event delivered to its run and a `burst` started
  in it shown in the next; a `teleport` with no picture between the two places; a pause, toggled by a local
  action, in which no timer moves and which the same key ends; a tap of a shared button made during that
  pause, which no tick after it reads, a tap made after it, which is read once, and a direction held through
  it, which is still held after with no `released`; a key bound in two input contexts driving only
  the active one's action, and a key that went down in one not pressing the other's; a frame function that
  asks for `PlayersMut`, which does not compile.
- **WP4:** example B's cooked, textured rock, captured in motion: an entity whose `Model` names a cooked mesh,
  turning in place, with its lit faces and its cast shadow turning together between ticks. A built-in cube
  does not stand in for it. And example B's ball, steered by the keys on the Mac and in Chrome.
- **WP6:** an offer answered twice, answered late, answered for a bot by a rule, and shown to a player who
  joins while it is open; a command sent in a dropped picture, admitted once; a `request.scene` the scene
  does not accept; a pause menu as a modal that the key which opened it closes.
- **WP13:** a save written by one build's type and restored by its successor's; two save types in one game,
  each request reaching its own capture; a restore whose enter function fails, a save over its limit and a
  damaged one, each leaving the world as it was and the scene not entered, with the step that tried it
  returning no error and the next step running; a recording that holds a restored
  save and a saved resource's starting value, replayed with another value in the slot to the same digest; a
  recording made on the Mac replayed in Chrome to the same digest.
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
  seats, characters and digest on the host, the joiner and a replay; in a match of two seats with a host
  and a guest, a third person turned away as full with no event and no character, the same third person
  given no seat when they arrive while the guest is away, and the guest returning within the grace to their
  seat and character; a ready mark and a host's panel choice made in the lobby and carried into the scene it
  starts; a pause ended from a peer, with no burst of ticks after; an open offer shown again after a
  reconnect; a rectangle a rule added to the level, met by a peer who joined after it; and a save within 1
  KiB of its 64 KiB limit that the host restores while peers play and while the host sends a command in
  some of the same ticks, arriving over many ticks with no frame `SESSION` refuses and every world's digest
  equal afterwards, and a damaged one, after which the match goes on, the scene is not entered and the
  digests are equal.
- **WP8:** the guard's refusals; an unfamiliar change made from the guides alone.

### What WP1 compiles

WP1 has no window, no input, no prefab and no space query. Its test games are written from this subset:

- **The game and its scenes:** `Game::new`, `.scene(..)` with one function and with a tuple, `.resource(..)`,
  `.component::<T>()`, `.max_entities(..)`, `.tick_rate(..)`, and `.test()` and `.test_in(..)`, which return
  a result; scene types that
  derive `Scene`, with and without fields; in a scene, `resource`, `on_enter`, `on_exit`, `rules`, `phases`,
  `phase(..)` with `on_enter`, `rules`, `on_exit`, `lasts` and `then`, and `add`.
- **Rule parameters:** `Query` with `With` and `Without`, `Single`, `Res`, `ResMut`, `Entities` (`spawn` of a
  tuple, `despawn`, `insert`, `remove`), `Events` and `Emit` for the game's own events, `Tick`, `Random`,
  `Scenes`, `Phase` and `PhaseMut`; `every`, `when` and `par_each`.
- **Data:** the `Component`, `Resource`, `Data`, `Event`, `Scene` and `Phase` derives, `Timer` and `Cooldown`,
  with plain components in place of transforms and looks. `Num` and `Vec2` are the profile's numbers as they
  stand, made from integers. Literals, angles and `Vec3` arrive with WP2.
- **The harness:** `step`, `step_ticks`, `step_for`, `step_until`, `spawn`, `despawn`, `set`, `get`,
  `resource`, `set_resource`, `count`, `find`, `single`, `alive`, `phase`, `scene`, `in_scene`, `go_to`,
  `go_to_phase`, `events`, `digest` and `rewind`.

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
   check made on what exists, frame functions with no window, and the interface kept from a per-frame
   description. Early sign: WP1, WP2, WP5 and WP6. The builder writes the difference in the log and ends
   `BLOCKED`, because a change of shape is the owner's to approve.
3. **A convincing fake of K2:** an example that reads plainly because the plumbing moved into a helper file
   of the game. Early sign: the reviewer's count of game lines includes every file of the game.
4. **The surface is too wide to document well.** It grew to close what three games guessed at. Early sign:
   WP1's surface check, which fails a public item with no example, and WP8's agents.
5. **Angles in the profile differ on one target.** Early sign: WP2's conformance run. The fallback is a table.
6. **Compile errors from typed parameters read badly.** Early sign: each work package's mistakes item, which
   holds the message text.
7. **The layer costs the crowd its speed**, now with a barrier after each rule and layers rebuilt on change.
   Early sign: WP2's cost record.
8. **Turning, textured models change passes the V1 exit stands on.** Early sign: an exact image that moves in
   WP4. A row that does not turn and has no texture is placed and shaded exactly as today.
9. **A wrong substitute for K7:** a fresh agent that succeeds because it read the examples' source in the
   engine checkout. Its brief allows the guides, the one file and the template, and its record lists every
   file it opened.
10. **A target's program cannot carry the list of `prefabs!` blocks.** Registration at link time is
    ordinary on the Mac and the simulator; on the browser target it needs the program's constructors run at
    start. Early sign: WP11.3 on the Mac, and WP2's conformance runs, whose trace row carries how many
    prefab names the SDK knew before it read the game's description, in Chrome and on the simulator. If a
    target cannot carry the list, the builder ends `BLOCKED`, because rooting a block by a line of game code
    is a change of shape.

## Proof

- **Correctness**, where a fault is silent and costly: written order and what a rule sees; equal digests when
  a run is repeated, rewound and replayed, and on the three targets; a refused tick leaving the world
  unchanged; the access checks; a match that stays equal. CPU tests on small worlds.
- **Equal digests are never the only witness.** A digest that left something out would agree with itself. So
  a refused tick is proved by reading the components, the resources and the entities back, value by value;
  the digest is shown to change when any one of them changes; and the crowd's trace carries witnesses of the
  work it claims.
- **Mistakes**: one test for each row of [the table](#16-mistakes-that-explain-themselves), holding the
  message's code, object, cause and next step.
- **The surface**: no engine type in `thrum`'s public items; an example on each; guides that compile.
- **Looks**: the kit games drawn on the Mac and in Chrome, inspected in motion through T2 capture.
- **The docs**: three fresh agents, commissioned by the lead as `CREATOR`'s was, one for each kind of game,
  each then asked for a change the guides did not show.
- **Cost**: the tripwire records above.

## What a builder may change

The worked examples are the shape the owner approved. A builder may change a **spelling**: a method's name, an
argument's order, a derive's attribute, where the game's code would read as well or better. Each is a line in
the log. A builder may not change the **shape**: add a concept a game maker must learn, remove one of the
fifteen words, change what a rule sees or when its edits land, change the written order, change what a control
function may read or which of it a match runs ahead, move a number between the portable kind and the float
kind, move a line of plumbing into game code, or make a game name an engine crate. Where the shape cannot be
built, the builder ends `BLOCKED` with the difference, and the planner brings it to the owner.

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
- **How many presses a button's count holds between two frames**, above the three the design asks for. WP2's
  builder chooses from what fits the input frame, and the reference says it.
