# Game SDK: one crate a game is written against

25 November 2026, revised the same day after [design reviews 1 and 2](LOG.md). [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [glossary](../GLOSSARY.md) ·
[the creator guide](../../design/ENGINE-EXPERIENCE.md), whose ambition this module builds ·
[the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) ·
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

**The owner approves this design before any builder starts.** A design review that returns Design Approved is
not that approval. The owner judges the API by reading [the worked examples](#worked-examples). The owner has
decided [what the first release leaves out of physics, animation and audio](#physics-animation-and-audio).

## The request

The owner, 25 November 2026: "What I would like you to plan is a high end, intuitive, expressive, readable, SDK
that game developers can use to build new games. this should be what V1 uses ideally and what we will use to
build the swarm game. It's VITAL that we get this right as it's the entire interface that developers and coding
agents will use to build a variety of games. It's the front of the product we're building. So it needs to be as
intuitive, well documented and easy to use as possible."

The owner's decisions the same day: "V1 first, Rust only, I'll approve the design."

- **V1 first.** The V1 exit does not wait for the SDK. V1 lands on today's API. SDK work packages then move the
  showcase onto the SDK, and [the swarm game](../../design/NEXT-GOAL-SWARM-GAME.md) is built on it from its first
  line. No work package on the road to the exit waits on this module
  ([the module list](../ARCHITECTURE.md#off-the-v1-path-track-7) names them).
- **Rust only.** Games are written in Rust. There is no scripting layer.
- **The owner approves the design**, as the first paragraph says.

The outcome the lead's task sets: a developer or agent who has never seen Thrum builds a small 3D game, a small
2D game and a networked game from the SDK's documentation alone, and the code reads as plainly as the game it
describes. One structure and one vocabulary across 2D, 3D, single-player and multiplayer; good-looking defaults;
mistakes that explain themselves; every public item documented with an example.

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
Agents cannot watch a window, so testing a game without one and getting a picture of it from a command are part
of the product, not conveniences.

**First release.** What is commissioned here:

1. The game structure: game, scenes, entities, components, resources, rules, frame functions.
2. 2D and 3D under that one structure: sprites and shapes; cooked models, scenery, sky, Sun, ground and lights.
3. Input actions, commands and players.
4. Assets by name with live edits; settings on the tuning panel; the profiler.
5. The interface: status displays, menus and modal choices.
6. Multiplayer as an opt-in that changes no rule.
7. Errors that name the object, the cause and the next step.
8. Reference documentation with an example on every public item, guides, templates and a new-game command.
9. The showcase moved onto the SDK, after the V1 exit.

**Not in the first release**, each said where it bears on a game maker:

- Rigid-body physics, skeletal animation and audio.
  [The owner decided this](#physics-animation-and-audio) on 25 November 2026: 2D queries and sprite animation
  are in, and these three are out.
- A texture on a model that an entity carries. An entity's `Model` has a colour and a material. A textured
  mesh is [scenery](#7-looks-what-is-drawn), which stands still.
- Water, clouds and weather. `WATERS` and `CLOUDS` are still building. The words `scene.water(..)` and
  `scene.weather(..)` are kept free for them.
- Multiplayer for 3D games. The structure allows it and the first release proves it in 2D only, as the swarm
  game needs.
- Gamepads, text fields, scrolling lists and interface transitions, which `GAME-UI` does not build.
- Parent and child entities, saved games, a scene file format, an editor, and hot reload of code. A changed rule
  rebuilds and restarts.
- A crates.io release. A game reaches `thrum` by a path to an engine checkout, as
  [GAME-SPLIT](../GAME-SPLIT/DESIGN.md) settled.

## What a game maker must see

| Cue | Where it must hold | Bar |
|---|---|---|
| K1 A first game is one file and one command | The 2D and the 3D starter, on the Mac and in Chrome | Godot's "Your first 2D game" and macroquad's first window: nothing to configure before something moves |
| K2 The code reads as the game | Every worked example and template: the rule list of a scene reads as a description of play, and no line names a device, a frame host, a page, a permit or a schema number | Bevy's `breakout` example, with less ceremony |
| K3 One structure | 2D, 3D, alone and online: the same `Game`, `Scene`, rules and frame functions. Turning multiplayer on changes no rule | Unreal's gameplay framework, where one set of rules serves a listen server and a client |
| K4 Defaults look finished | A 3D scene that asks for `Sky::clear()` and nothing else gets the physical sky and Sun, shadows, exposure, tone mapping, reconstruction, EDR where the display has it and the governor. A 2D scene gets crisp sprites, drawn in order | The showcase's own look, with no setting touched |
| K5 Mistakes explain themselves | At compile time, at startup and while running, across [the listed mistakes](#13-mistakes-that-explain-themselves) | Rust's own compiler messages: the object, the cause, the next step |
| K6 Every public item is documented with an example that compiles | All of `thrum` | The Rust standard library's reference |
| K7 The docs are enough | A fresh agent builds a small 2D game, a small 3D game and a networked game from the guides alone, each in about 100 game lines, the G6 bar | The first hour with Godot's or Bevy's getting-started guides |
| K8 The speed is kept | The reference crowd's behaviours at 10,000 bodies, written as SDK rules, beside the hand-written reference; a game links, compiles and prepares nothing it does not use | The engine's own figures, as tripwires |
| K9 A game is tested without a window | Rules stepped, input scripted and state read in an ordinary `#[test]`; one command writes a picture of a running game | An agent can prove a rule and look at a frame |

## The game at a glance

Twelve words carry the whole SDK. Each means the same thing in a 2D game, a 3D game, alone and online.

| Word | What it is | In code |
|---|---|---|
| **Game** | The program: its scenes and what holds for all of them | `Game::new("Rockfall").scene(play).run()` |
| **Scene** | One part of the game with its own setup, rules and look: a level, a menu, a match. One runs at a time | `fn play(scene: &mut Scene)` |
| **Entity** | One thing in the world | `entities.spawn((Rock { .. }, Transform2d::at(x, 10.0)))` |
| **Component** | A plain piece of data on an entity | `#[derive(Component)] struct Health(i32);` |
| **Resource** | A plain piece of data the whole world has one of | `#[derive(Resource)] struct Score { dodged: u32 }` |
| **Rule** | A function that moves the world on by one tick. Rules are the only code that changes the world | `fn fall(mut rocks: Query<(&Rock, &mut Transform2d)>, tick: Tick)` |
| **Frame function** | A function that runs once for each picture: it reads the world as shown and sets what only this player sees | `fn follow_my_hero(mut camera: Local<Camera2d>, ..)` |
| **Action** | A named thing a player can do, bound to keys, pointer and touch | `const MOVE: Stick = Stick::new("move");` |
| **Command** | A message a player sends once, such as a menu choice. One that is admitted reaches the rules once, after the same player's earlier ones, and never twice. `send` says when one is not taken, and one on its way when a link drops may be lost | `send.command(Choice::Upgrade { wave, upgrade })` |
| **Player** | Who an input came from. A game played alone has one | `inputs.of(player).stick(MOVE)` |
| **Look** | A component the engine draws: `Sprite`, `Shape2d`, `Model`, `Light` | `Sprite::new("enemies/bat")` |
| **Setting** | A named value on the tuning panel | `#[slider(-8.0..=60.0)] elevation: f32` |

Three sentences hold the model together, and the guides open with them:

1. **Rules change the world, at the tick.** The world is its entities and resources, and nothing else.
2. **Frame functions show the world, at the frame.** They cannot change it. They can send actions and commands.
3. **Everything that changes the world comes in as input at a tick**: a held action, a command, a player
   joining. That is why a game can be replayed, tested and played online without being rewritten.

## Worked examples

These are real games against the proposed API. They are the shape the owner approves:
[what a builder may change](#what-a-builder-may-change) says how far a builder may depart from them.

### A. A first 2D game

Rockfall: steer a ship under falling rocks. The whole game is one file.

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Rockfall").scene(play).scene(game_over).run()
}

const STEER: Axis = Axis::new("steer");
const AGAIN: Button = Button::new("again");
const ROCKS: Layer = Layer::new("rocks");

#[derive(Component)]
struct Ship;

#[derive(Component)]
struct Rock {
    fall_speed: Num,
}

#[derive(Resource, Default)]
struct Score {
    dodged: u32,
}

fn play(scene: &mut Scene) {
    scene.background(Color::NIGHT);
    scene.camera(Camera2d::showing_height(18.0)); // 18 world units tall, whatever the window
    scene.bind(STEER, (Keys::A_D, Keys::LEFT_RIGHT, Touch::drag()));
    scene.resource(Score::default());
    scene.on_enter(launch_ship);
    scene.rules((steer_ship, drop_rocks, fall, crash));
    scene.each_frame(show_score);
}

fn launch_ship(mut entities: Entities) {
    entities.spawn((Ship, Transform2d::at(0.0, -7.0), Sprite::new("ship"), Collider2d::circle(0.5)));
}

fn steer_ship(mut ships: Query<&mut Transform2d, With<Ship>>, inputs: Inputs, tick: Tick) {
    let steer = inputs.first().axis(STEER);
    for mut ship in &mut ships {
        ship.position.x = (ship.position.x + steer * 9.0 * tick.dt()).clamp(-12.0, 12.0);
    }
}

fn drop_rocks(mut entities: Entities, mut random: Random, tick: Tick) {
    if tick.every(seconds(0.4)) {
        entities.spawn((
            Rock { fall_speed: random.between(4.0, 9.0) },
            Transform2d::at(random.between(-12.0, 12.0), 10.0),
            Sprite::new("rock"),
            Collider2d::circle(0.6).on(ROCKS),
        ));
    }
}

fn fall(
    mut rocks: Query<(Entity, &Rock, &mut Transform2d)>,
    mut entities: Entities,
    mut score: ResMut<Score>,
    tick: Tick,
) {
    for (rock, falling, mut at) in &mut rocks {
        at.position.y -= falling.fall_speed * tick.dt();
        if at.position.y < -10.0 {
            entities.despawn(rock);
            score.dodged += 1;
        }
    }
}

fn crash(ships: Query<Entity, With<Ship>>, space: Space2d, mut scenes: Scenes) {
    for ship in &ships {
        if space.touching(ship).on(ROCKS).any() {
            scenes.go_to(game_over);
        }
    }
}

fn show_score(mut ui: Ui, score: Res<Score>) {
    ui.top_left(|ui| ui.text(format!("Dodged {}", score.dodged)));
}

fn game_over(scene: &mut Scene) {
    scene.background(Color::NIGHT);
    scene.bind(AGAIN, (Key::Enter, Touch::tap()));
    scene.rules(play_again);
    scene.each_frame(show_result);
}

fn play_again(inputs: Inputs, mut scenes: Scenes) {
    if inputs.first().pressed(AGAIN) {
        scenes.go_to(play);
    }
}

fn show_result(mut ui: Ui, score: Res<Score>) {
    ui.center(|ui| {
        ui.title("Game over");
        ui.text(format!("You dodged {} rocks. Press Enter to play again.", score.dodged));
    });
}
```

What the reader did not write: a window, a device, a clock, a schedule, component registration, interpolation
between ticks, sprite batching, a web page, or a line for Chrome. `cargo play` opens it on the Mac and
`cargo play --web` serves it to Chrome. `game_over` has no camera: a scene without one shows its background and
its interface and draws nothing else ([scenes](#5-scenes)).

### B. A first 3D scene

Today's starter (a cooked rock on lit ground under the physical sky, with Sun sliders) with one rule added, so it
is a game and not a still.

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Thrum starter").local_settings::<Sun>().scene(clearing).run()
}

/// Where the Sun stands is this screen's own, so these sliders are local settings and may be plain `f32`.
#[derive(Settings)]
struct Sun {
    #[slider(-8.0..=60.0, step = 0.5, unit = "°", label = "Sun elevation")]
    elevation: f32,
    #[slider(0.0..=360.0, step = 1.0, unit = "°", label = "Sun azimuth")]
    azimuth: f32,
}

impl Default for Sun {
    fn default() -> Self {
        Self { elevation: 25.0, azimuth: 150.0 }
    }
}

#[derive(Component)]
struct Spins {
    turns_per_second: Num,
}

fn clearing(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.ground(Ground::flat());
    scene.camera(Camera3d::looking_at(vec3(0.0, 1.6, 5.5), vec3(0.0, 0.7, 0.0)).fov(35.0));
    scene.on_enter(place_rock);
    scene.rules(spin);
    scene.each_frame(move_sun);
}

fn place_rock(mut entities: Entities) {
    entities.spawn((
        Model::new("meshes/rock").material(Material::rough(Color::rgb(0.34, 0.29, 0.23))).on_ground(),
        Transform3d::at(0.0, 0.0, 0.0),
        Spins { turns_per_second: num(0.1) },
    ));
}

fn spin(mut things: Query<(&Spins, &mut Transform3d)>, tick: Tick) {
    for (spins, mut at) in &mut things {
        at.turn_y(Angle::turns(spins.turns_per_second * tick.dt()));
    }
}

fn move_sun(mut sky: Local<Sky>, sun: Local<Sun>) {
    sky.sun = SunAngle::degrees(sun.elevation, sun.azimuth);
}
```

The sky, the Sun's light and shadows, exposure, tone mapping, reconstruction and the governor are the engine's
defaults. The two sliders appear in the Mac panel and on the Chrome page, a parameter file sets them, and the
profiler is a toggle beside them. The rock is an entity, so it is drawn as one of the 3D scene's entity rows:
it turns smoothly between ticks, and its shadow turns with it ([looks](#7-looks-what-is-drawn)). The sliders
move the sky on this screen only: no rule reads them, so they are
[local settings](#10-settings-and-the-panel).

### C. A slice of the V1 showcase

The exit scene: the cooked valley, 10,000 entities of which 1,500 drift, six stops and the hour of the day.

```rust
use thrum::prelude::*;

const NEXT_STOP: Button = Button::new("next stop");

/// Registered with `Game::local_settings::<Day>()`: the hour lights this screen and no rule reads it.
#[derive(Settings)]
struct Day {
    #[slider(4.0..=22.0, step = 0.05, unit = " h", label = "Hour")]
    hour: f32,
}

#[derive(Component)]
struct Drifts {
    around: Vec3,
    radius: Num,
    turns_per_second: Num,
}

#[derive(Resource, Default)]
struct Tour {
    stop: u32,
}

fn valley(scene: &mut Scene) {
    scene.sky(Sky::clear());
    scene.scenery(Scenery::new("showcase/valley").texture("showcase/ground")); // stands still, so not an entity
    scene.camera(STOPS[0].camera());
    scene.bind(NEXT_STOP, (Key::Tab, Touch::tap()));
    scene.resource(Tour::default());
    scene.on_enter(populate);
    scene.rules((drift, tour));
    scene.each_frame((sun_follows_hour, fly_to_stop));
}

fn populate(mut entities: Entities, mut random: Random) {
    for index in 0..10_000 {
        let home = layout::home(index);
        let prop = Model::new(layout::PROPS[index % layout::PROPS.len()]);
        if index < 1_500 {
            entities.spawn((prop, Transform3d::at_point(home), Drifts {
                around: home,
                radius: random.between(2.0, 9.0),
                turns_per_second: random.between(0.02, 0.08),
            }));
        } else {
            entities.spawn((prop, Transform3d::at_point(home)));
        }
    }
}

fn drift(mut movers: Query<(&Drifts, &mut Transform3d)>, tick: Tick) {
    movers.par_each(|(drifts, mut at)| {
        let turn = Angle::turns(drifts.turns_per_second * tick.seconds());
        at.position = drifts.around + turn.direction_xz() * drifts.radius;
    });
}

fn tour(inputs: Inputs, mut tour: ResMut<Tour>) {
    if inputs.first().pressed(NEXT_STOP) {
        tour.stop = (tour.stop + 1) % STOPS.len() as u32;
    }
}

fn sun_follows_hour(mut sky: Local<Sky>, day: Local<Day>) {
    sky.sun = SunAngle::at_hour(day.hour, VALLEY_LATITUDE);
}

fn fly_to_stop(mut camera: Local<Camera3d>, tour: Res<Tour>, frame: Frame) {
    camera.ease_to(STOPS[tour.stop as usize].camera(), frame.dt());
}
```

`STOPS` and `layout` are the game's own data. The valley is scenery: a textured mesh the scene places once,
drawn by the engine's cooked placement as the showcase draws it today. The props are entities, drawn as entity
rows. The 8,500 that no rule writes cost nothing after they are placed, and `par_each` spreads the movers over
the engine's worker threads. GPU culling, cooked mesh rows, the recipe for this Mac and the Chrome build are
not in the game's code at all.

### D. The swarm game's core loop

A hero for each player holds out against a growing crowd, with attacks that fire by themselves and a choice of
upgrade between waves. The rules are one file, and what a player sees is another.

```rust
// rules/src/lib.rs — the run. It gives the same result on every player's machine.
use thrum::prelude::*;

pub const MOVE: Stick = Stick::new("move");
pub const HEROES: Layer = Layer::new("heroes");
pub const ENEMIES: Layer = Layer::new("enemies");
pub const GEMS: Layer = Layer::new("gems");

#[derive(Component)]
pub struct Hero {
    pub fire_every: Seconds,
    pub damage: i32,
    pub chose_in_wave: u32,
}
#[derive(Component)]
pub struct Enemy;
#[derive(Component)]
pub struct Shot {
    pub damage: i32,
    pub ends: TickNumber,
}
#[derive(Component)]
pub struct Gem;
#[derive(Component)]
pub struct Health(pub i32);
#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Resource, Default)]
pub struct Run {
    pub wave: u32,
    pub phase: Phase,
    pub phase_ends: TickNumber,
    pub gems: u32,
    pub team_level: u32,
}

#[derive(Data, Default, PartialEq)]
pub enum Phase {
    #[default]
    Gathering,
    Wave,
    Intermission,
    Won,
    Lost,
}

#[derive(Event)]
pub struct EnemyDied {
    pub at: Vec2,
}

/// A choice answers one offer, and says which: the wave whose intermission showed the cards.
#[derive(Command)]
pub enum Choice {
    Upgrade { wave: u32, upgrade: Upgrade },
}

#[derive(Data)]
pub enum Upgrade {
    FasterFire,
    MoreDamage,
    Heal,
}

pub fn arena(scene: &mut Scene) {
    scene.space2d(Space2d::cells(4.0)); // part of the rules, so the same for every player
    scene.action(MOVE); // the rules read it; the crate that draws this scene binds it to keys
    scene.component::<Gem>(); // no rule or frame function names a gem, so the scene says it exists
    scene.resource(Run::default());
    scene.rules((
        heroes_join_and_leave,
        per_player(move_hero),
        start_wave,
        (
            spawn_enemies,
            steer_enemies,
            fire_at_nearest,
            move_bodies,
            enemies_hurt_heroes,
            shots_hit,
            enemies_die,
            collect_gems,
            end_wave,
        )
            .when(in_wave),
        take_choices.when(in_intermission),
    ));
}

pub fn hero(player: Player) -> impl Bundle {
    (
        Hero { fire_every: seconds(0.4), damage: 1, chose_in_wave: 0 },
        player,
        Health(100),
        Transform2d::at(num(player.index()) * 3.0 - 4.5, 0.0),
        Sprite::new("heroes/knight").layer(2),
        Collider2d::circle(0.5).on(HEROES),
    )
}

fn bat(at: Vec2) -> impl Bundle {
    (
        Enemy,
        Health(3),
        Velocity(Vec2::ZERO),
        Transform2d::at_point(at),
        Sprite::new("enemies/bat").layer(1),
        Collider2d::circle(0.35).on(ENEMIES),
    )
}

/// `PlayerJoined` comes for every player who is here when the arena starts, and for each who arrives later.
fn heroes_join_and_leave(
    joined: Events<PlayerJoined>,
    left: Events<PlayerLeft>,
    heroes: Query<(Entity, &Player), With<Hero>>,
    mut entities: Entities,
) {
    for join in joined.read() {
        entities.spawn(hero(join.player));
    }
    for leave in left.read() {
        for (hero, player) in &heroes {
            if *player == leave.player {
                entities.despawn(hero);
            }
        }
    }
}

/// A hero walks where its own player steers. This is the hero's whole movement: a per-player rule is given
/// its player's character, that player's input and the tick, and nothing else, so in a match the engine can
/// run it ahead for the local player and their hero answers at once. The tick is the one every rule sees.
fn move_hero(at: &mut Transform2d, input: &Input, tick: Tick) {
    at.position = (at.position + input.stick(MOVE) * 6.0 * tick.dt()).clamp_to(ARENA);
}

fn spawn_enemies(run: Res<Run>, heroes: Query<&Transform2d, With<Hero>>, mut entities: Entities, mut random: Random) {
    for _ in 0..run.wave * heroes.count() as u32 {
        let Some(near) = random.pick(&heroes) else { return };
        entities.spawn(bat(near.position + random.direction() * 30.0));
    }
}

fn steer_enemies(mut enemies: Query<(Entity, &Transform2d, &mut Velocity), With<Enemy>>, space: Space2d) {
    enemies.par_each(|(me, at, mut velocity)| {
        // The heroes are a layer of their own, so this looks at four colliders and not at the crowd.
        let Some(hero) = space.nearest(at.position).on(HEROES).first() else { return };
        let chase = (hero.position - at.position).normalized();
        let mut apart = Vec2::ZERO;
        for other in space.within(at.position, 1.0).on(ENEMIES).except(me) {
            let away = at.position - other.position;
            apart += away / away.length_squared().max(0.02);
        }
        velocity.0 = (chase * 2.5 + apart * 0.6).clamp_length(3.0);
    });
}

fn fire_at_nearest(heroes: Query<(&Hero, &Transform2d)>, space: Space2d, mut entities: Entities, tick: Tick) {
    for (hero, at) in &heroes {
        if !tick.every(hero.fire_every) {
            continue;
        }
        let Some(target) = space.nearest(at.position).on(ENEMIES).within(12.0).first() else { continue };
        let aim = (target.position - at.position).normalized();
        entities.spawn((
            Shot { damage: hero.damage, ends: tick.after(seconds(1.5)) },
            Velocity(aim * 18.0),
            Transform2d::at_point(at.position),
            Sprite::new("shots/bolt").layer(3).glow(0.5),
        ));
    }
}

fn move_bodies(mut bodies: Query<(&Velocity, &mut Transform2d)>, tick: Tick) {
    bodies.par_each(|(velocity, mut at)| {
        at.position = (at.position + velocity.0 * tick.dt()).clamp_to(ARENA);
    });
}

fn enemies_hurt_heroes(mut heroes: Query<(Entity, &mut Health), With<Hero>>, space: Space2d) {
    for (hero, mut health) in &mut heroes {
        health.0 -= space.touching(hero).on(ENEMIES).count() as i32;
    }
}

fn shots_hit(
    shots: Query<(Entity, &Shot, &Transform2d, &Velocity)>,
    mut enemies: Query<&mut Health, With<Enemy>>,
    space: Space2d,
    mut entities: Entities,
    tick: Tick,
) {
    for (shot, hit, at, velocity) in &shots {
        // Sweep from where the shot was, so a fast shot cannot pass through a bat between two ticks.
        let from = at.position - velocity.0 * tick.dt();
        let struck = space.sweep(from, at.position, 0.15).on(ENEMIES).first();
        if let Some(mut health) = struck.and_then(|struck| enemies.get_mut(struck.entity)) {
            health.0 -= hit.damage;
            entities.despawn(shot);
        } else if tick.reached(hit.ends) {
            entities.despawn(shot);
        }
    }
}

fn enemies_die(enemies: Query<(Entity, &Health, &Transform2d), With<Enemy>>, mut entities: Entities, mut died: Emit<EnemyDied>) {
    for (enemy, health, at) in &enemies {
        if health.0 <= 0 {
            entities.despawn(enemy);
            entities.spawn((Gem, Transform2d::at_point(at.position), Sprite::new("pickups/gem"), Collider2d::circle(0.3).on(GEMS)));
            died.send(EnemyDied { at: at.position });
        }
    }
}

fn collect_gems(heroes: Query<&Transform2d, With<Hero>>, space: Space2d, mut entities: Entities, mut run: ResMut<Run>) {
    for hero in &heroes {
        for gem in space.within(hero.position, 1.5).on(GEMS) {
            if entities.despawn(gem.entity) {
                run.gems += 1; // `despawn` is false when another hero took this gem in the same tick
            }
        }
    }
    run.team_level = 1 + run.gems / 20;
}

fn start_wave(mut run: ResMut<Run>, heroes: Query<(), With<Hero>>, tick: Tick) {
    let waiting = matches!(run.phase, Phase::Gathering | Phase::Intermission);
    if waiting && !heroes.is_empty() && tick.reached(run.phase_ends) {
        run.wave += 1;
        run.phase = Phase::Wave;
        run.phase_ends = tick.after(seconds(60.0));
    }
}

fn end_wave(
    mut run: ResMut<Run>,
    heroes: Query<&Health, With<Hero>>,
    enemies: Query<Entity, With<Enemy>>,
    mut entities: Entities,
    tick: Tick,
) {
    if heroes.iter().all(|health| health.0 <= 0) {
        run.phase = Phase::Lost;
    } else if tick.reached(run.phase_ends) {
        enemies.iter().for_each(|enemy| { entities.despawn(enemy); });
        run.phase = if run.wave == 10 { Phase::Won } else { Phase::Intermission };
        run.phase_ends = tick.after(seconds(15.0));
    }
}

/// A hero takes one upgrade in each intermission. A choice counts only for the offer it answers: one that
/// arrives late, in a later intermission, names an earlier wave and changes nothing. A second choice for the
/// same offer changes nothing either, so a choice made twice, or made again after a lost connection, is safe.
fn take_choices(choices: Commands<Choice>, run: Res<Run>, mut heroes: Query<(&Player, &mut Hero, &mut Health)>) {
    for (player, Choice::Upgrade { wave, upgrade }) in choices.read() {
        for (owner, mut hero, mut health) in &mut heroes {
            if *owner != player || wave != run.wave || hero.chose_in_wave == run.wave {
                continue;
            }
            hero.chose_in_wave = run.wave;
            match upgrade {
                Upgrade::FasterFire => hero.fire_every = hero.fire_every * 0.85,
                Upgrade::MoreDamage => hero.damage += 1,
                Upgrade::Heal => health.0 = 100,
            }
        }
    }
}

fn in_wave(run: Res<Run>) -> bool {
    run.phase == Phase::Wave
}

fn in_intermission(run: Res<Run>) -> bool {
    run.phase == Phase::Intermission
}
```

```rust
// src/main.rs — what each player sees. Nothing in this file can change the run.
use swarm_rules::*;
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Swarm").scene(run).run()
}

fn run(scene: &mut Scene) {
    scene.add(arena); // the rules
    scene.background(Color::rgb(0.05, 0.06, 0.08));
    scene.camera(Camera2d::showing_height(40.0));
    scene.bind(MOVE, (Keys::WASD, Keys::ARROWS, Touch::stick()));
    scene.each_frame((follow_my_hero, flash_where_enemies_die, status, upgrade_cards));
}

fn follow_my_hero(mut camera: Local<Camera2d>, heroes: Query<(&Player, &Transform2d), With<Hero>>, me: Me) {
    if let Some((_, at)) = heroes.iter().find(|(player, _)| **player == me.player()) {
        camera.center = at.position;
    }
}

fn flash_where_enemies_die(died: Events<EnemyDied>, mut draw: Draw2d) {
    for death in died.read() {
        draw.burst("effects/pop", death.at);
    }
}

fn status(mut ui: Ui, run: Res<Run>, heroes: Query<(&Player, &Health), With<Hero>>, me: Me) {
    ui.top_left(|ui| {
        for (player, health) in &heroes {
            ui.bar(health.0 as f32 / 100.0).mine(*player == me.player());
        }
    });
    ui.top_center(|ui| ui.text(format!("Wave {}", run.wave)));
    ui.top_right(|ui| ui.text(format!("Level {}", run.team_level)));
}

fn upgrade_cards(mut ui: Ui, run: Res<Run>, heroes: Query<(&Player, &Hero)>, me: Me, mut send: Send) {
    let chosen = heroes.iter().any(|(player, hero)| *player == me.player() && hero.chose_in_wave == run.wave);
    if run.phase != Phase::Intermission || chosen {
        return;
    }
    // The cards are offered from what the world says, not from what was sent. They stay until the choice has
    // landed, so a choice lost on the way is made again. The choice names the wave it was offered in, so
    // `take_choices` ignores one that lands twice or lands in a later intermission.
    ui.modal("Choose an upgrade", |ui| {
        for upgrade in [Upgrade::FasterFire, Upgrade::MoreDamage, Upgrade::Heal] {
            if ui.card(upgrade.name(), upgrade.blurb()).chosen() {
                send.command(Choice::Upgrade { wave: run.wave, upgrade });
            }
        }
    });
}
```

A rule is tested like any function, with no window and no GPU:

```rust
#[test]
fn a_hero_shoots_the_nearest_bat_and_it_leaves_a_gem() {
    let mut game = Game::new("test").scene(fight).test();
    game.spawn(hero(Player::FIRST));
    game.spawn(bat(vec2(3.0, 0.0)));

    game.step_for(seconds(2.0));

    assert_eq!(game.count::<With<Enemy>>(), 0);
    assert_eq!(game.count::<With<Gem>>(), 1);
}

/// Only the rules under test: a scene is a function, so a test builds the one it needs. It has no camera
/// and binds no key, because a test draws nothing and scripts its input.
fn fight(scene: &mut Scene) {
    scene.space2d(Space2d::cells(4.0));
    scene.component::<Gem>();
    scene.rules((fire_at_nearest, move_bodies, shots_hit, enemies_die));
}
```

### E. The same game, online

No rule changes. The game says how many may play and adds a scene where players gather.

```rust
fn main() -> thrum::Result {
    Game::new("Swarm")
        .multiplayer(Multiplayer::players(1..=4))
        .scene(lobby)
        .scene(run)
        .run()
}

fn lobby(scene: &mut Scene) {
    scene.background(Color::rgb(0.05, 0.06, 0.08));
    scene.each_frame(gather);
}

fn gather(mut ui: Ui, mut session: Session, mut scenes: Go) {
    ui.center(|ui| match session.state() {
        SessionState::Alone => {
            if ui.button("Play alone").clicked() {
                scenes.go_to(run);
            }
            if ui.button("Host a game").clicked() {
                session.host();
            }
            if let Some(ticket) = session.dropped_ticket() {
                session.join(ticket); // a link opened, a paste or a launch argument: nobody types it
            }
        }
        SessionState::Hosting { ticket, players } => {
            ui.text(format!("{players} of 4 here. Send your friends this link:"));
            ui.copyable(ticket.link());
            if ui.button("Start").clicked() {
                scenes.go_to(run);
            }
        }
        SessionState::Joining { progress } => {
            ui.bar(progress);
        }
        SessionState::Playing => {}
        SessionState::Closed { reason } => {
            ui.text(reason.to_string());
        }
    });
}
```

What made this possible is in the rules already: heroes are spawned from `PlayerJoined`, every rule reads
`inputs.of(player)`, the upgrade choice is a command that names its offer, and the numbers agree on every
device. A game played
alone is the same game with one player, who joins at the first tick.

The lobby keeps nobody back. A scene is given `PlayerJoined` at its first tick for every player already in the
game, and again for each who arrives later ([players](#6-actions-commands-and-players)). So:

- *Play alone*, chosen after any time in the lobby: the arena's first tick brings `PlayerJoined` for the one
  player, their hero is spawned, and `start_wave` finds a hero.
- Three friends join the host in the lobby and the host presses *Start*: the arena's first tick brings four
  `PlayerJoined`, in slot order, on every machine, and four heroes stand in a row.
- A friend joins while a wave is running: they are sent the world as it stands, with its heroes, and their own
  `PlayerJoined` comes at the tick they enter. One hero for each player in every case, because a scene's
  entities leave with it and a player is announced to a scene once.

The lobby has no camera, so it shows its background and its interface and draws nothing else.

## Research and selection

Each engine was read for how a game's code is shaped, not for its features.

| Engine | Shape of game code | What Thrum takes | What Thrum leaves |
|---|---|---|---|
| [Bevy 0.20](https://bevy.org/news/bevy-0-20/) (October 2026) | Entities and plain-data components; systems are functions whose parameters say what they read and write; scene functions (`bsn!`, since [0.19](https://bevy.org/news/bevy-0-19/)) compose entity templates as patches; observers | Components, queries and functions with typed parameters, and its names for them (`Query`, `Res`, `With`), because it is the Rust idiom agents and developers already know; templates as functions | The schedule graph: system sets, labels and ordering constraints. 0.20 added schedule randomisation to find order a game depends on without saying so. Thrum has no such order to find. Also the `App` and plugin layer, and the `bsn!` macro language, which its own release notes call rough |
| [Godot 4.6](https://godotengine.org/releases/4.6/) | A tree of nodes with scripts; a scene is a reusable tree; named input actions; `_process` and `_physics_process`; step-by-step first-game guides | Named input actions; the split between the tick and the frame; a scene as a self-contained part of a game; the "Your first 2D game" and "Your first 3D game" guides as the bar for ours | State held inside objects with methods: it cannot be digested, rewound or checked equal between two machines |
| Unity | GameObjects with MonoBehaviours; prefabs; values tuned in the Inspector. DOTS exists because an `Update` on each of 10,000 objects is slow | Settings a maker tunes live; prefab-like templates | A method call for each object each frame |
| Unreal | Actors, with a gameplay framework that keeps apart the rules (GameMode), the shared state (GameState), a player's input (PlayerController) and what one player sees (HUD) | That separation, which is what makes one game serve a host and a client: rules, world, actions for each player, frame functions | Its size, and replication declared property by property |
| [Fyrox 1.0](https://fyrox.rs/blog/post/fyrox-game-engine-1-0-0/) (March 2026) | A scene graph with Rust scripts and an editor | Evidence that an object graph in Rust needs handles, pools and reflection derives on every script to save and reload it | The object graph |
| macroquad | One `async fn main` with a loop that draws | The first minute: one file, one command, something on screen with no asset. Built-in shapes and primitives serve it | Immediate-mode state, which cannot be replayed or shared |
| [lightyear](https://github.com/cBournhonesque/lightyear) and bevy_ggrs | Deterministic replication: only inputs travel, and the simulation is rolled back and replayed | The current practice for crowds online, and the constraint it puts on game code: rules are pure functions of state and input. Thrum's `SESSION` already works this way | Bolting it on afterwards: both ask a game to register what is rolled back and to avoid system-local state. Here it is the only way to write a rule |

**The choices, and what would change each.**

1. **Plain-data components and rule functions, not objects with methods.** Thrum's simulation is a fixed tick over
   registered state that is rewound, replayed, digested and shared by causes
   ([mechanism 1 of track 3](../ARCHITECTURE.md#mechanisms-settled-here)). Only state the engine can see whole can
   do that. Considered: a node tree with scripts, which is friendlier for a first hour and is where Godot, Unity
   and Fyrox sit. It was turned down because a game written that way could not be put in a match or replayed
   without being rewritten, and the owner asked for one structure. *It would change* if multiplayer were dropped
   from the SDK's aims.
2. **Rules run in the order written.** One list for each scene, top to bottom. Considered: Bevy's graph, where
   the engine orders systems from constraints. It finds more parallelism between systems, and it is the largest
   source of surprises in Bevy code. Thrum's parallelism is inside a rule (`par_each` over columns), which is
   where a crowd's cost is. *It would change* if a profile showed whole rules that could run side by side
   dominating a tick.
3. **Two kinds of function, told apart by type.** A rule cannot take `Ui` or `Local<T>`, and a frame function
   cannot take `ResMut<T>` or `Entities`; each mistake is a compile error that says why. Considered: one kind of
   system on two schedules, as Bevy has, which lets a frame system write game state and makes interpolation the
   game's job.
4. **Looks are components.** A sprite is on the entity it draws. Considered: a function from state to picture
   for each kind of entity, which keeps presentation out of the world by construction and is what the engine's
   extraction does today. It reads less plainly, and every maker already knows the component form. The cost is
   a few bytes of look in the digest.
5. **One vocabulary for 2D and 3D, with the dimension in the name where the two differ**: `Transform2d` and
   `Transform3d`, `Camera2d` and `Camera3d`. Considered: one `Transform` for both, as Bevy has, which gives a 2D
   game a z and a quaternion it never wanted; and two modules with the same names, which makes a search and a
   compiler message ambiguous.
6. **Numbers that agree on every device, for every game.** [Mechanism 8](#8-numbers-that-agree-on-every-device).
   Considered: ordinary `f32` for games played alone and the portable number only online. That makes "turn on
   multiplayer" a rewrite, which is what the owner asked to avoid.
7. **Actions are typed constants; components, resources, commands and settings are derives.** A constant needs
   no macro, shows in an editor's completion and makes `inputs.stick(DASH)` a type error. A derive is used only
   where hand-writing would be long and easy to get wrong: a component's encoding is about forty lines in the
   reference crowd.
8. **The interface is described each frame and kept by the engine.**
   [Mechanism 11](#11-the-interface). This is where current interface kits have settled (React, SwiftUI,
   Compose, and Bevy's scene work). Considered: handing the maker `GAME-UI`'s kept tree to build once and
   mutate, which costs less to build and reads worse.

## Mechanisms

### 1. One crate, two halves

A game depends on `thrum` and names no other engine crate. `use thrum::prelude::*` brings the twelve words and
the maths. No public item of `thrum` names a type of `wgpu`, `winit` or another engine crate: what a game sees
of the engine's own types is re-exported under `thrum`'s names, and the rest stays inside.

`thrum` has a **rules half** and a **play half**.

- The rules half is components, resources, queries, rules, entities, events, input, commands, players, the
  tick, random draws, space queries, the maths, and the look components and shared settings as data. It
  builds with no renderer, window, GPU, network or platform crate in its dependency closure. A headless
  server, a test and a rule crate use only this half.
- The play half is `run()`, scenes' cameras and sky, frame functions, the interface, the panel and local
  settings, assets and the session. The `multiplayer` Cargo feature, off by default, is the only way
  `thrum-network` and `thrum-transport` enter a game's closure.

How the two halves are laid out in crates is the builder's, registered in `.config/architecture.toml`. The
rule that binds: a crate that names `thrum` with default features off gets the rules half and compiles alone.

**Which packages are held to `thrum` alone.** The repository's guard holds it, from WP8
([ruling 5](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), and it does not hang on a package naming `thrum`:

- **Covered:** every package in a games repository, every package under the engine's `templates/` and every
  package under the engine's `examples/`. That is each game's main package and every package the game keeps
  beside it, such as its rule crate or a helper.
- **The rule:** a covered package names no engine crate but `thrum`. None of its dependencies (ordinary,
  development, build or for one target, under any alias, from a path, a git address or a registry) is a
  package registered under the engine's `crates/`, other than `thrum`. A path of its that leaves its game
  reaches `thrum` and nothing else. So a helper package that names `thrum-render`, and a game that names
  `thrum-app` and no `thrum`, are both refused.
- **Not covered:** the packages the registry marks as written on the engine's own API, each with its reason.
  When WP8 lands those are the proving ground and its iPhone app; the engine's own proofs under `examples/`
  (`crowd-2d`, `crowd-2d-run`, `crowd-2d-view`, `link-proof`, `match-proof`, `mirror` and `water`); the old
  starter, stones and lighthouse, until WP10 removes them; and the showcase in the games repository, with
  its iPhone app if that has followed it, named by their folders there, until WP9 rewrites it. A package is marked where it is registered, by the module that owns
  it, and a game made for game makers is never marked.

The showcase is on the list whether or not it has moved yet, so `GAME-SPLIT` WP4 and WP8 may land in either
order. WP9 takes it off the list in the landing that rewrites it, and from then it is held like any game.

### 2. The world: entities, components, resources

The world is `thrum-core`'s registered simulation. The SDK adds no storage.

- A **component** is `Copy` plain data: numbers, `Vec2`, `Vec3`, `Entity`, `Player`, fixed arrays, and types that
  derive `Data`. `#[derive(Component)]` writes its registration, its stable schema identity (from its name and
  fields, never from a Rust type ID, as C01 requires), its encoding and its finiteness check. A field of a type
  that cannot be stored, such as a `Vec` or a `String`, is a compile error that says so.
- A **resource** is the same for data the world has one of. It may also hold the SDK's bounded lists.
- An **entity** is a handle, valid until the entity is despawned. A bundle is a tuple of components, and a
  template is a function that returns one: `fn bat(at: Vec2) -> impl Bundle`.
- A look that needs a transform brings one at the origin when the bundle has none.
- The world holds a fixed number of entities, because the engine reserves its storage once. The default is the
  builder's to choose and document, and `Game::max_entities(n)` sets it. A spawn that would pass it is refused
  with [a message](#13-mistakes-that-explain-themselves) and the world is unchanged.

**What the world holds is fixed before the first tick.** The engine builds its storage and its schedule once,
at startup, from a list of every component and resource. The SDK makes that list from three places, and a game
writes the third only for what the first two cannot show:

1. **The signatures of every function the game's scenes list**: rules, conditions, enter and leave functions,
   per-player functions and frame functions. A component in a `Query`, a `With`, a `Without` or an `Attach`, and
   a resource in a `Res`, a `ResMut` or a `scene.resource(..)`, are known from there.
2. **The SDK's own components**: the transforms, the looks, `Collider2d` and `Player`.
3. **`scene.component::<T>()`**, for a component that is only ever spawned and never named in a signature, such
   as example D's `Gem`. The body of a rule is not visible to the SDK, so a bundle spawned inside one does not
   announce what it holds.

Every scene given to the game is read, not only the one running, so the list is one list for the whole game.
Spawning or counting a component the list lacks is refused with
[a message](#13-mistakes-that-explain-themselves) that names the rule, the component and the line to add. It
is refused at that spawn, and not at startup, because nothing before it names the type.

In a game with multiplayer on, the list must come out the same from the rules alone, because the rules are what
every peer shares. A component that only a frame function names is then a startup error, which says to add
`scene.component::<T>()` beside the rules.

**What a rule may touch comes from its signature alone.** Each `&T` in a query reads `T` and each `&mut T`
writes it. `Entities` asks for the engine's one structural permit, which covers spawning any bundle and
despawning any entity, so what a rule spawns has to be known to the world and never declared for access.
`Attach<T>` writes `T`.

### 3. Rules, and the tick

A rule is a function. Its parameters say what it reads and writes, and the SDK turns them into the access
declaration the engine's schedule validates.

| Parameter | What it gives |
|---|---|
| `Query<D, F>` | The entities that have `D`, filtered by `With<T>` and `Without<T>`, visited in a fixed order. `&T` reads, `&mut T` writes, `Entity` names |
| `Res<T>`, `ResMut<T>` | A resource |
| `Entities` | `spawn` a bundle and `despawn` an entity |
| `Attach<T>` | `add(entity, value)` and `remove(entity)`: one component put on or taken off a living entity. It lands when the tick ends, as a spawn does |
| `Inputs` | Each player's actions for this tick, for the actions the game's scenes declare |
| `Commands<T>` | The commands of type `T` admitted at this tick, with who sent each, in the order each player sent them |
| `Events<T>`, `Emit<T>` | Events of this tick: `PlayerJoined`, `PlayerLeft`, `PlayerAway`, `PlayerBack`, and the game's own. An event a rule sends is read by later rules of the tick and by none in the next |
| `Tick` | The tick's number, its length `dt()`, and `every`, `after`, `reached` |
| `Random` | Draws keyed by the game's seed, the tick and what is asked, so a replay draws the same |
| `Space2d` | `touching`, `within`, `nearest` and `sweep` over entities with a `Collider2d`, by layer |
| `Scenes` | `go_to(scene)` |

**What a rule sees.** Rules of the running scene run once each tick, in the order the scene lists them. A rule
sees what earlier rules of the same tick wrote to components and resources. **Spawns and despawns land when the
tick ends**: an entity spawned in a tick is first visited in the next, and one despawned is still visited by
later rules of its last tick. `Space2d` answers from where things stood when the tick began. This is the
engine's own shape (in-place writes that roll back together, with structural changes at the barrier), so the SDK
adds no machinery to provide it, and the guides state it in these words.

**A rule is a function of its parameters.** The same world, input and tick give the same result on every
machine and in every replay. The SDK holds up its side by type: nothing a rule can be handed leads to a clock,
a file, the network, the window or one player's screen, and a rule cannot take a frame function's parameters.
The other side is the author's, because a rule is ordinary Rust and Rust lets any function open a file. The
contract, in the words the guides use: **a rule reads nothing but its parameters and changes nothing but
through them.** No file, socket, process, standard input, environment variable, clock, thread or changing
static, and no result that hangs on a hash map's order or a pointer's value.

What checks the contract:

- **In a rule crate, the profile check** (`cargo play --check`) refuses the ordinary ways of breaking it, each
  with a message that says why. As the profile does today: raw-float arithmetic and methods, unordered
  containers, clocks, threads, the environment, outside crates, build scripts and suppressed lints. Added for
  the SDK ([ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)): reading or writing a file (`std::fs`),
  opening a socket (`std::net`), starting a process (`std::process`) and reading standard input.
- **It is a guard against mistakes, not a sandbox.** `unsafe` code, a foreign function or a static with
  interior mutability can still reach out, as [the portable rules guide](../../SIM2D-RULES.md) says of the
  profile. What gets past is caught, where it shows, by the digests a match compares.
- **A single-crate game is not checked.** Its rules carry the same contract, and it is told so when it turns
  multiplayer on.

**A per-player rule is a character's whole movement step.** `per_player(f)` runs `f` once a tick for each
player's character: the one entity that carries that `Player` and the components `f` names. `f` may take `&T`
and `&mut T` for components of that entity, `&Input` for that player's input, and `Tick`. `Tick` there is
the whole of what it is in any rule: the number of the world's tick being stepped, `dt()`, `every`, `after`
and `reached`. A character that moves on every other tick is therefore a per-player rule like any other.
Nothing else: no query, resource, `Space2d`, `Entities` or `Random`, each of which is a compile error there.
So the function
writes the pose itself, as example D's `move_hero` writes its hero's `Transform2d`, and does not leave a
velocity for a later rule to apply. A per-player rule stands at the top level of a scene's rule list and runs
in every tick of that scene. Putting one under `when` is a startup error, because whether it ran would then
hang on something outside it; what gates a character's movement is held on the character, as a component the
function reads. Two entities that match one per-player rule for one player refuse the tick. These limits are
what let a match run the function ahead of the host ([multiplayer](#12-multiplayer-as-an-opt-in)). They hold
alone as well, so the function written for a game played alone is the one a match predicts, unchanged.

**Space queries search one layer.** A collider is on one layer, and each layer has its own index, built once a
tick from that layer's colliders in slot order, from where they stood when the tick began. `.on(LAYER)`
searches that layer's index and nothing else. So the cost of a query follows the layer it asks and not the
world: an enemy that asks for the nearest of four heroes looks at four colliders with 10,000 enemies around
it, and a nearer enemy can never stand in front of a hero. Answers come in the index's fixed order: nearest
first by squared distance, ties by slot. `nearest(..).within(r)` looks only in the cells that radius reaches.
Without `within`, `nearest` looks at every collider on the layer, which is right for a small layer such as the
heroes; the guides say to give a radius on a crowded one, and a search that widens ring by ring is not built
in the first release. A query with no `.on` asks every layer and merges the answers in the same order. *It
would change* if a game needed the nearest of a crowded layer, with no radius, from every member of a crowd.

**Parallel work** is inside a rule: `query.par_each(..)` runs the closure over the engine's chunks. The closure
writes only the entity it is given and reads the rule's other parameters. The result is the same as the serial
loop, by construction.

**Conditions.** `rules.when(condition)` runs a rule or a group only in ticks where a read-only function returns
true.

**Failure.** A rule returns nothing, or a `thrum::Result`. A returned error refuses the tick, on every target:
the engine rolls back what earlier rules of the tick wrote, the world is as it was at the last tick, and the
game stops with the rule's name, the tick and the error. In a test, `step()` returns that error and the game
can be stepped again. A panic is a bug in the game, and what follows it depends on the target, because Rust's
browser target aborts on a panic and cannot unwind:

| Target | When a rule panics |
|---|---|
| The Mac and the iPhone simulator, in `run()` and in `Game::test()` | The panic is caught and reported as a returned error is, with the previous world intact |
| The browser target (`wasm32-unknown-unknown`), in Chrome and in a `Game::test()` built for it | The game's program stops. Before it does, the SDK's panic hook puts the rule, the tick and the message on the page and in the console. Nothing is rolled back, because nothing runs afterwards |

So a rule that can fail in play returns an error, and the guides say so. A build of the browser target that
unwinds is not on the stable toolchain, and the SDK does not depend on one.

**The tick** is 60 Hz unless `Game::tick_rate` says otherwise, the brief's "configurable, not an engine
ceiling". Speeds are written per second and multiplied by `tick.dt()`.

### 4. Frame functions, and the frame

A frame function runs once for each picture the engine encodes, when the engine samples the scene for that
picture. For every
picture the engine chooses a pair of adjacent published ticks and how far between them the picture stands
([C02](../../design/CONTRACTS.md#c02-time-and-committed-state--f1-coordinator-w1-world)). On the Mac that pair
can be older than the newest one published. The engine hands the SDK that pair and that fraction, the frame
functions run on them, and the draw that follows uses the same pair and the same fraction. That is **the world
as shown**:

- a transform is where the picture has it, between the pair's two ticks at the picture's fraction. A camera
  set from a hero's transform and the hero's sprite are therefore one instant, whatever the tick rate and the
  display rate;
- everything else is as of the pair's newer tick;
- in a match, the local player's own character is at its predicted pose, in a query and in the draw alike
  ([multiplayer](#12-multiplayer-as-an-opt-in)).

A frame function writes only what belongs to this player's screen.

| Parameter | What it gives |
|---|---|
| `Query<D, F>`, `Res<T>` | The world as shown, read-only |
| `Events<T>` | The events of every tick since this function last ran, each once, in tick order. `missed()` counts any that were dropped |
| `Local<T>` | State of this screen: the camera, the sky, the game's own menu state. Never part of the world |
| `Ui` | The interface for this frame |
| `Draw2d` | Sprites and shapes for this picture only, drawn among the world's. `burst` starts a short effect that the SDK keeps and plays out over the pictures that follow |
| `Send` | `command(..)`: the one way a frame function affects the world. It says whether the command was taken |
| `Me`, `Session` | Which player this screen is, and the match |
| `Frame` | The frame's time and its `dt()` |
| `Go` | `go_to(scene)`, as a command |

**Events reach the frame through a bounded cursor.** Pictures and ticks do not come at the same rate, so the
tick an event happened in may be an endpoint of no picture at all. With each tick it commits, the SDK
publishes that tick's events for the frame functions that read them, in a ring that holds the most recent
ticks: at least one second of them, the size the builder's to choose and document. Each `Events<T>` of a frame
function keeps a cursor, the last tick it has delivered.

- A run of the frame functions whose newer tick is `n` delivers the events of every tick after the cursor,
  up to `n`, and moves the cursor to `n`. A tick that fell between two pictures loses nothing: the death that
  happened in it is delivered with the next.
- A run on the same pair as the last delivers nothing a second time.
- When more ticks have passed than the ring holds, as after a hitch or a hidden tab, the oldest are dropped
  and `missed()` says how many. These events are for show, so nothing in the world hangs on them.
- A rewind, a resync and a change of scene reset the cursor to the tick the world then stands at. Events still
  waiting are dropped. A tick stepped again after a rewind delivers its events again, because the second run
  is the one that stands.

**A picture that is never shown.** The engine runs the frame functions when it encodes a picture, and only
afterwards asks the display for somewhere to put it. Sometimes there is nowhere: the drawable times out, the
window is covered or the surface changed. The engine then drops what it encoded, and the SDK reads that from
the frame host's record of what it last submitted. The frame functions still ran, once, and nothing runs them
again for that picture:

- **Everything a frame function does happens once, in the run where it does it.** An event is delivered to
  one run. A click is handed to one run. A command sent, a `go_to` and a call on `Session` take effect when
  they are made, whether that run's picture is shown or not, so a choice is sent once and never again for a
  picture that had to be made twice. `Local<T>` keeps what the run wrote.
- **What must be seen does not hang on one picture.** A `burst` is not part of the picture it was started in.
  The SDK keeps it as this screen's state, timed by the frame's clock from the run that started it, and draws
  it in every picture until it ends. One that has not been in a submitted picture when its time runs out is
  kept for the next picture the engine submits, and dropped after a second with none. So the death at tick 41
  whose picture was dropped flashes in the next picture that is shown, though that picture's run was given no
  event. A flash a game times itself is the same: it sets a time in `Local<T>` in the run that reads the
  event, and draws from that in the runs that follow.
- **What is drawn for one picture goes with that picture.** A sprite or a shape that a frame function draws
  with `Draw2d`, and the interface it describes, belong to the run's own picture and are drawn again by the
  next run. A one-picture drawing made only in the run that reads an event is lost when that picture is
  dropped, which is why the guides start a burst there.

Considered: moving the cursor only when a picture is submitted, and giving the same events to the next run.
That shows a one-picture drawing, and it makes every frame function that counts events, eases a camera or
sends on an event do so twice on a dropped picture, a fault that shows only when a drawable times out.

Interpolation between ticks is the engine's, on the GPU, for every look. A game writes none. A component is
published to frame functions only if one names it, which the SDK knows from their signatures, so a component
that only rules use costs nothing to show.

### 5. Scenes

A scene is a function that describes one part of the game: `fn play(scene: &mut Scene)`. It names what runs
when the scene is entered and left, its rules, its frame functions, its camera, its background or sky, its
bindings and its space. `scene.add(part)` composes scenes from parts, which is how the swarm game's rules live
in one crate and its screens in another.

- The first scene given to `Game` is where the game starts.
- `scenes.go_to(next)` changes scene when the tick ends. Entities spawned in a scene leave with it. Resources
  stay until replaced.
- Which scene is running is world state, so a match's players are always in the same scene.
- A scene's kind (2D or 3D) follows from its camera. A 2D scene compiles none of the 3D passes and a 3D scene
  none of the sprite pass, as [RENDER-2D](../RENDER-2D/DESIGN.md) built.
- A scene needs no camera. Without one it shows its background and its interface and draws no look, which is
  what a menu, a lobby and a result page are. An entity with a look in such a scene is not drawn and is
  [reported once](#13-mistakes-that-explain-themselves).
- The camera belongs to one player's screen, not to the world. A game with no window, such as a test, has no
  camera and needs none, so nothing about a camera is checked there.

### 6. Actions, commands and players

**Actions.** An action is a constant: `Button`, `Axis` (one number from −1 to 1) or `Stick` (two). A rule reads
`inputs.of(player)`: `held`, `pressed` (went down this tick), `axis`, `stick`. `inputs.first()` is the first
player, for a game played alone.

- A scene **declares** the actions its rules read. `scene.bind(ACTION, keys)` declares one and binds it to
  keys, the pointer and touch. `scene.action(ACTION)` declares one without binding it, for a rule crate whose
  screens are written elsewhere, as example D's `arena` does.
- The actions that all of a game's scenes declare are its action set, each known by its name. The set is the
  same on every peer, fixes the layout of the input frame, and is part of the rule-set identity.
- The shell samples the running scene's bindings, and each tick admits one input frame for each player.
- `Inputs` in a signature does not say which actions a rule reads, so two mistakes are found later than
  compile time. At startup, `run()` refuses a declared action that no scene binds. And a rule that reads an
  action no scene declared refuses its tick, at the read, with the rule, the action and the line to add. A
  test scripts its input and needs no binding.

**Commands.** A command is a value of a type that derives `Command`, sent from a frame function. What a game
can rely on:

- `send.command(..)` says at once whether the command was taken. It is not taken when too many commands are
  still on their way (`Busy`: send it in a later frame) or when the match has closed (`Closed`). A command
  that was not taken is never read.
- A command that is admitted is read by `Commands<T>` at one tick, once, after the same player's earlier
  commands, on every machine. No command is read twice.
- Alone, a command that was taken is admitted at the next tick. In a match it is `SESSION`'s ordered command:
  the host admits it at the first tick with room, and it does not expire while the link holds.
- If the link drops while a command is on its way, the game cannot know whether it was admitted. The session
  does not send it again, and the player comes back to a fresh copy of the world
  ([the session guide](../../SESSION.md)). So a game offers a choice from what the world says and makes a
  repeated choice harmless, as example D's upgrade does: the cards stay until the hero's own state shows the
  choice, and the rule ignores a second one.
- An admitted command is read at the tick the host had room for it, which can be later than the player
  meant: `SESSION`'s ordered commands carry no tick they expire at. So a command that answers an offer names
  the offer, and the rule that reads it checks the offer still stands. Example D's choice carries the wave
  whose intermission showed the cards, and `take_choices` drops one whose wave is not the current one, so an
  answer that arrives in a later intermission cannot spend that intermission's choice.

**Players.** A player is a slot, and who is in the game is world state. `PlayerJoined` is an event a scene's
rules read at the tick a player enters the game, and at a scene's first tick for every player already in it,
in slot order. Alone, the one player joins at the first tick. A scene's entities leave with it, and a player
is announced to a scene once. So a scene that spawns a character for each `PlayerJoined` has one for each
player, however they came: before the scene started, as from a lobby, or during it. `PlayerLeft`,
`PlayerAway` and `PlayerBack` come at the tick of each. A game written this way is ready for a match without
knowing it.

**Settings the rules read** are world state, changed by a command the panel sends
([settings](#10-settings-and-the-panel)).

Input reaches rules only in these three forms, which is `SWARM`'s
[three drivers](../SWARM/DESIGN.md#7-who-admits-the-frames-alone-and-in-a-match): alone, hosting and joined, the
rules cannot tell which. The engine's input frame carries a fixed number of axes and at most eight timed actions
of up to 256 bytes ([the guide](../../SESSION.md)). What can be known before the first tick is checked at
startup: that the action set fits the frame, with the counts and the limit, and that each command type's
encoding fits the bytes one action may have. Room in the queue while the game runs cannot be known then, and
is what `send.command` reports.

### 7. Looks: what is drawn

A look is a component, and an entity with a look and a transform is drawn. Looks are world state like any
component.

| | 2D | 3D |
|---|---|---|
| Where | `Transform2d`: position, turn, scale | `Transform3d`: position, rotation, scale |
| What | `Sprite` (a frame of a sheet, by name: layer, tint, flash, glow, animation), `Shape2d` (a circle, a box, a line, with no asset) | `Model` (a cooked mesh by name, with a colour and a material; `Model::cube()` and `Model::sphere()` with no asset), `Light` |
| Camera | `Camera2d`: a centre and how much of the world is shown | `Camera3d`: an eye, a target and a field of view |
| Around it | `scene.background(colour)` | `scene.sky(..)`, `scene.ground(..)`, `scene.scenery(..)` |
| Touching | `Collider2d` and `Space2d` | Not in the first release |

- Extraction is the SDK's, written once: it maps the looks of entities whose rows changed to
  `RENDER-2D`'s sprite rows and to the 3D scene's entity rows, and the engine interpolates on the GPU. An
  entity no rule writes is extracted once.
- **An entity's `Model` is drawn by one route: the 3D scene's entity rows.** The engine draws a cooked mesh
  in two ways. An *entity row* is one instance of a mesh in the scene's mesh arena, with its placement at two
  ticks, culled on the GPU and interpolated: the route the showcase's 10,000 entities take. A *cooked
  placement* (`thrum-asset-gpu`'s `MeshPlacement`, which the old starter's rock uses) is one textured mesh
  with one matrix and no tick, culled a meshlet at a time. A `Model` on an entity always takes the first: the
  SDK admits the cooked product's geometry to the mesh arena through `CONTENT-LOOP`'s cooked mesh rows, the
  entity's `Transform3d` at the pair's two ticks is the row's placement, and its colour and material are the
  row's material. The SDK makes no cooked placement for an entity, so nothing an entity carries is drawn from
  a record that cannot move. `.on_ground()` lifts the row by the mesh's lowest point, read from the cooked
  product, when the row is extracted; it is not world state. A saved mesh is redrawn live, as cooked mesh rows
  are today. `Model::cube()` and `Model::sphere()` are rows of meshes the SDK brings.
- **In 3D the whole placement is interpolated: position, turn and scale.** The entity rows carry only a
  position for each of the two ticks today, and the scene's passes interpolate only that, so a model that
  turned or grew in place would stand still between ticks. WP4 adds the turn and the scale at both ticks to
  the extraction and to the row ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), and every pass
  that places an entity row uses the one interpolated placement: the scene's vertices and normals, the shadow
  caster, the culling bounds for the camera and for the shadow, motion vectors, and the ray-traced instances
  where ray tracing is on. Position and scale go linearly, and the turn takes the shorter way round. A row
  whose turn and scale are the same at both ticks is placed exactly as it is today, so no exact image of the
  engine's changes. The case that tells this from a drifting prop, and from a built-in cube, is example B's
  rock: a lopsided **cooked** model that stays in one place and turns. Its lit faces and its shadow turn
  together, smoothly, between ticks, and WP4 is judged on it.
- **Scenery is a textured mesh that stands still.** `scene.scenery(Scenery::new("showcase/valley")
  .texture("showcase/ground"))` places a cooked mesh once, with its texture, material, place, turn and size.
  It is the engine's cooked placement as built: textured, culled a meshlet at a time, shadowed, live when the
  asset is saved. It is part of the scene's description, like the ground, and not an entity: no rule reads or
  moves it. An entity's `Model` has no texture in the first release, because the 3D scene's entity materials
  take none today; the type has no `texture` method, and the reference says to use scenery or a colour. *It
  would change* when the entity rows' materials take a texture, which is engine work planned after the first
  release; `Model` then gains the method and no game changes.
- Draw order in 2D is layer, then lower on the screen in front, then identity, which `RENDER-2D` keeps stable.
- The camera, sky and background belong to the screen (`Local<..>`), not the world, so two players of one match
  can look at different places.
- The engine chooses the recipe for the device, as
  [the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md) says. A game never names one.
  Launch settings, such as the one that turns ray tracing on, pass through `run()` to the engine unchanged.

### 8. Numbers that agree on every device

`Num`, `Vec2`, `Vec3`, `Quat` and `Angle` are `thrum`'s maths, used in rules and in frame functions alike. They
are [the deterministic rules profile](../SIM-2D/DESIGN.md#mechanisms)'s numbers: each operation is rounded once,
so one input trace gives one result on the Mac, in Chrome and on the iPhone simulator.

- **Literals read plainly**: `vec2(1.5, 0.0)`, `speed * 0.9`. A literal's value is fixed when the game is
  compiled, so it is the same everywhere.
- **Angles and turning** are built from the profile's exactly rounded operations, with no platform maths
  library, so `Angle::turns(..).direction()` and a rotation in 3D agree on every device. The profile today has
  no trigonometry: these are additions to it, with conformance vectors on the three targets
  ([ruling 1](../ARCHITECTURE.md#seam-rulings-for-game-sdk)).
- **A game played alone** uses the same numbers. Its rules are portable without its author doing anything.
- **What can break it** is code that reaches outside: a raw float computed with the platform's maths, a hash
  map's order, a file, a clock, an outside crate. [A rule's contract](#3-rules-and-the-tick) says what a rule
  may not reach and what checks it: the profile's lints in a rule crate, and nothing in a single-crate game,
  which is told so when it turns multiplayer on.

### 9. Assets

A game names an asset by its path under `assets/`, without the extension: `Sprite::new("enemies/bat")`,
`Model::new("meshes/rock")`. This is G5's naming. In a development build `cargo play` cooks `assets/`, watches
it, and a saved file is redrawn in the running game without a stall, as [CONTENT-LOOP](../CONTENT-LOOP/DESIGN.md)
built. A shipping build links no authoring code.

A name that matches nothing draws a visible placeholder and is reported once, with the rule that spawned it and
the nearest names that do exist. The test harness lists every such name a run used.

### 10. Settings and the panel

`#[derive(Settings)]` on a struct makes each field a control: a slider, a toggle, a text or a readout. They are
`thrum-controls`' typed controls ([CREATOR](../CREATOR/DESIGN.md)), so the Mac panel, the Chrome page and
parameter files work as they do today, and the profiler is in the panel of every game.

A setting has one owner, and the game says which when it registers the struct:

| | A shared setting: `Game::settings::<T>()` | A local setting: `Game::local_settings::<T>()` |
|---|---|---|
| Owner | The world. `T` is a resource | This screen |
| Read by | Rules and frame functions, as `Res<T>` | Frame functions, as `Local<T>`. A rule cannot take it |
| Fields | What a resource may hold: `Num`, integers, `bool`, a `Data` enum. A raw float is a compile error that says to use `Num`, or to make the setting local | Those, and `f32` |
| A panel edit | Enters as a command and changes the world at a tick, like any input. The panel's value becomes the field's type once, on the machine where the edit was made, and the command carries that value, so every peer applies the same bits. A replay holds it | Changes this screen at once. It never leaves the machine and is in no digest and no replay |
| In a match | The host's edits are the ones admitted. Another player's panel shows the value and cannot change it | Each player has their own |
| A parameter file | Its values enter as the same commands, at the first tick | Read at startup and whenever the file changes |

Examples B and C's Sun and hour light one player's sky and no rule reads them, so they are local settings and
their sliders are `f32`. A setting that changes play is shared:

```rust
/// The whole match plays at one difficulty: the host's panel sets it and the rules read it.
#[derive(Settings)]
struct Difficulty {
    #[slider(0.5..=3.0, step = 0.1, label = "Enemy speed")]
    enemy_speed: Num,
}

fn enemies_speed_up(mut enemies: Query<&mut Velocity, With<Enemy>>, difficulty: Res<Difficulty>) {
    for mut velocity in &mut enemies {
        velocity.0 = velocity.0 * difficulty.enemy_speed;
    }
}
```

### 11. The interface

A frame function describes the interface with `Ui`: places (`top_left`, `center`, a modal), and in them text,
bars, images, buttons and cards. The SDK keeps [GAME-UI](../GAME-UI/DESIGN.md)'s tree and changes only what
differs from the last frame, so an interface that did not change costs what `GAME-UI` says it does: one draw.
`button(..).clicked()` is true in the frame after `GAME-UI` hands the game an activation. A modal is an
exclusive scope: while it is open the game's actions are let go, and what is still held comes back when it
closes. Focus, direction keys, touch sizes, safe areas and density are `GAME-UI`'s and need nothing from the
game.

A control is the same control from one frame to the next by its place in the description. Where controls are
made in a loop and their number, order or labels can change, the game gives each a key that stays with what it
stands for, so `GAME-UI`'s focus and a press under way follow the control and not its position:

```rust
for upgrade in offered {
    if ui.card(upgrade.name(), upgrade.blurb()).key(upgrade).chosen() { /* .. */ }
}
```

Text exists at the sizes a font was cooked at. The SDK's default font and sizes ship with it, so a first game
needs no font.

### 12. Multiplayer as an opt-in

`Game::multiplayer(Multiplayer::players(1..=4))` with the `multiplayer` feature. The SDK then implements
`SESSION`'s world for the game: the rule-set identity from the game's rules and build, the world image and
digest from the registered state, the action wire from the declared actions and commands, the roster as player
events, and the driven world. A game writes none of the steps of [the session guide](../../SESSION.md).

- **Who does what** is `SESSION`'s and unchanged: a native process hosts, browsers and the simulator join, the
  crowd is shown at the confirmed tick, and a peer too slow is dropped.
- **The local player's own character answers at once** when its movement is
  [a per-player rule](#3-rules-and-the-tick). The SDK gives that function to `SESSION`'s predicted controller
  as the controller's step. The controller's state is the components the function names and the number of
  the tick they stand at, read from the player's character and the world at the confirmed tick, and compared
  by their encoded bits. Its input is the player's own input as the wire carries it, and its constant is the
  length of a tick. The function is the whole step, so running it ahead moves the pose and not only a
  velocity.
- **The predicted step sees the tick the host's step sees.** `SESSION`'s controller hands a step its state,
  its input and its constants, and no tick. The SDK carries the tick in the state: the step from a state at
  tick `n` hands the function `Tick` number `n + 1` and returns the new components at `n + 1`. The
  controller steps one tick at a time from the confirmed state, in prediction and when it replays after a
  correction, so the number the function is handed is always the tick the host will commit that step at. A
  player who joins a running match starts from a baseline whose state holds that baseline's tick, so this
  holds from any tick, not only from zero. A per-player rule that uses `tick.every(..)`, `after` or `reached`
  is therefore predicted as the host steps it, with `SESSION` as built and the function WP2 accepted
  unchanged.
- **What the local player's screen shows.** Their own character is shown from the controller: the components
  the per-player function names are at the predicted pose, interpolated between predicted ticks, both in a
  frame function's query and in the draw. `follow_my_hero` and the hero's sprite therefore move together as
  soon as the stick moves, before the host has confirmed anything. The crowd and the other players' heroes are
  shown at the confirmed tick. When the host's result differs, the controller returns to the confirmed state
  and replays, and the SDK eases the shown pose to the corrected one over a few pictures. A game writes none
  of this.
- **`Session`** in a frame function: `host()`, `join(ticket)`, `state()`, `leave()`. A ticket arrives as a link,
  a paste or a launch argument.
- **A game that can be played online keeps its rules in a rule crate of its own**, as example D does: a
  package inside the game's folder, beside the package that draws the game, whose manifest says it is a rule
  crate. `cargo play --check` holds that crate to the profile. It finds the crate from the game's own
  manifests and not from the engine's registry of rule packages, so a game outside the engine checkout is
  checked for its own rules, and a violation in them fails its check. The engine's external game check admits
  the path from a game's package to its own rule crate as it admits a path to the engine's crates
  ([rulings 1 and 5](../ARCHITECTURE.md#seam-rulings-for-game-sdk)). The `multiplayer` template is laid out so
  and passes both. A game whose rules are not in a rule crate is told at startup, with how to move them.

### 13. Mistakes that explain themselves

Every message names the object, the cause and the next step. The listed mistakes, each proved by a test in the
work package that builds its surface:

| When | Mistake | The message says |
|---|---|---|
| Compile | A rule takes `Ui`, `Local<T>`, `Draw2d`, `Send`, `Me`, `Session` or `Frame` | That a rule must give the same result on every player's machine, and to move this to a frame function |
| Compile | A frame function takes `ResMut<T>`, `Entities`, `Attach<T>`, `Emit<T>` or a `&mut` query | That frame functions cannot change the world, and to send a command or write a rule |
| Compile | A component, a resource or a shared setting holds a `Vec`, a `String`, a reference or a raw float | That world state is plain data, and what to use: `Num` for a number, and `local_settings` for a setting only this screen reads |
| Compile | `inputs.stick(..)` on a `Button`, and the like | Rust's own type error, on the constant |
| Compile | `per_player` is given a function that takes a query, a resource, `Space2d`, `Entities` or `Random` | That a per-player rule must be able to run ahead on one machine, and to keep what it needs on the character |
| Startup | Two parameters of one rule write the same component | The rule and the component, and to split the query with `Without` |
| Startup | A rule reads a resource no scene provides | The rule, the resource and `scene.resource(..)` |
| Startup | A `per_player` rule is under `when` | The rule, why it must run every tick, and to hold what gates it on the character |
| Startup | The game's actions do not fit an input frame, or a command type is larger than a frame's action | The counts or the size, and the limit |
| Startup | With multiplayer on, a component only a frame function names | The component, and `scene.component::<T>()` beside the rules |
| Startup, in `run()` | A declared action that no scene binds | The action and `scene.bind(..)` |
| Startup, in `run()` | No asset store, or a missing prerequisite | What is missing and the command that provides it |
| Running | A spawn or a count of a component the game does not know | The rule, the component, and to name it in a query or add `scene.component::<T>()` |
| Running | A rule reads an action no scene declares | The rule, the action, and `scene.action(..)` or `scene.bind(..)` |
| Running | Two entities match one `per_player` rule for one player | The rule, the player and both entities |
| Running | `go_to` a scene the game was not given | The scene and `.scene(..)` |
| Running | A spawn past the world's size | The size and `Game::max_entities` |
| Running | A rule returns an error, or panics | The rule, the tick and the error, [as each target allows](#3-rules-and-the-tick) |
| Running, on screen | An entity with a look in a scene with no camera, or a 3D look under a 2D camera and the reverse | Once: the scene, the look and `scene.camera(..)`. The look is not drawn and the game goes on |
| Running, on screen | An asset name that matches nothing | The name, the rule and the nearest names |
| Running, on screen | An entity's `Model` names a mesh the scene's mesh arena refuses | Once: the name, the arena's reason and, for a mesh that stands still, `scene.scenery(..)`. The placeholder is drawn |
| Running, on screen | A session is refused or closed | `SESSION`'s reason, in words a player can read |

`send.command` when the queue is full is not a mistake that stops a game: it returns `Busy` to the frame
function that called it.

**Where each check runs.** The rules half's startup checks run in `test()` and in `run()` alike, before the
first tick. The rows marked `run()` and "on screen" belong to the play half: they run in `run()` and in
`cargo play --shot`, the startup ones before a window opens, and never in `test()`, which has no camera,
binding, asset store or session. WP1 proves the rules half's rows through `test()`. WP3, which builds the
first runner, proves that `run()` makes the same checks. `thrum::Error` is one enum with a variant for each
kind of remedy, in [the adopted error shape](../../../.agents/skills/thrum-engineering/references/patterns.md).

### 14. Tests without a window, and pictures for an agent

`Game::test()` returns the game with no window, device or clock: `join()`, `hold` and `press` for a player's
actions, `send` for a command, `step()` and `step_for(..)`, `spawn`, `set`, `count`, `get`, `resource`,
`digest()`, and `rewind(ticks)`. It runs the same rules, on the same engine path, as `run()`. It has no
camera, binds no key and runs no frame function, so a scene written for a test needs none of them, and `send`
answers as `send.command` does.

`cargo play --shot <file>` runs a game offscreen for a number of ticks, under a scripted input file if one is
given, and writes the frame the engine submitted, through [T2 capture](../T2-CAPTURE/DESIGN.md). It needs no
screen and no permission.

### 15. Documentation, guides and templates

- **Reference.** Every public item of `thrum` has a doc comment with an example, and every example is a doc test.
  `./dev check` fails a public item without one.
- **Guides**, in `docs/sdk/`: *Start here* (the three sentences and the twelve words); *Your first 2D game*;
  *Your first 3D game*; *Make it multiplayer*; *How do I…* (a page of short answers, each a few lines of code);
  *When something goes wrong*. A guide teaches only what is built, and each code block in a guide is compiled
  by the docs check.
- **One file for agents.** `docs/sdk/REFERENCE.md` is generated from the reference and the guides: the whole
  SDK in one file an agent can read in one go. The docs check fails when it is stale.
- **Templates**: `2d`, `3d` and `multiplayer`. `./dev new <folder> --template <name>` writes a game that builds
  and runs as written, with its `AGENTS.md`, which points an agent at the one file. A game made this way lives
  outside the engine checkout and reaches `thrum` by path. The `multiplayer` template is two packages, the game
  and its rule crate, and as written it passes the engine's external game check and its own profile check.
- The engine's own guides ([the session guide](../../SESSION.md), [sprite scenes](../../SPRITE-SCENES.md),
  [portable rules](../../SIM2D-RULES.md)) stay as the reference for what is under the SDK. The creator guide's
  opening points at *Start here*.

### 16. Running a game

One command for each target, the same for every game, from the game's folder. The template ships them as Cargo
aliases onto the engine's tool.

| Command | What it does |
|---|---|
| `cargo play` | Cooks and watches `assets/`, builds, opens the Mac window with its panel |
| `cargo play --web` | The same for Chrome: builds for wasm32, serves the page and the store on loopback, prints the URL |
| `cargo play --ios` | Builds and starts the game on the iPhone simulator, through [GAME-UI](../GAME-UI/DESIGN.md)'s one shell |
| `cargo play --shot <file>` | [Writes a picture](#14-tests-without-a-window-and-pictures-for-an-agent) |
| `cargo play --check` | The profile check of the game's own rule crate, where the game has one |

The game's `main` is the same file on every target. The SDK supplies each platform's entry.

## The system as it runs

### State and owners

| State | Owner | The SDK's part |
|---|---|---|
| Entities, components, resources, the tick, rewind, publication | `thrum-core` (G1, G2S) | Registers what the rules name; adapts rule functions to registered systems |
| The rules profile, the digest, 2D queries | `SIM-2D` | `Num` and friends are its numbers; `Space2d` is its index, one for each layer |
| The frame, the recipe, reconstruction, EDR, the governor | `APPLE-TILE`, `APPLE-PRESENTATION` | None. A game gets them by default |
| Sprite rows, sheets, the 2D scene | `RENDER-2D` | Maps `Sprite`, `Shape2d` and `Transform2d` to rows |
| Sky, Sun, air | `ATMOSPHERE` | `Sky` sets its inputs |
| Cooked products and live edits | `CONTENT-LOOP`, `COOKED-DRAW` | Resolves names; gives an entity's `Model` its cooked mesh row, and gives `Scenery` to the cooked placement as built |
| The 3D scene's entity rows and the passes that place them | `thrum-render` (`CULLING`, `RAY-TRACING` and the scene's other owners) | Maps an entity's `Model` and `Transform3d` to a row, with its placement at both ticks |
| Typed controls, parameter files, the profiler | `CREATOR` | `Settings` declares controls |
| The interface tree, events, fonts | `GAME-UI` | `Ui` keeps the tree |
| The device, pause, recovery, the game contract | `HOST` | Implements the game contract once, for every game |
| The match, the roster, prediction | `SESSION`, over `TRANSPORT` | Implements the session's world once, and gives a per-player rule to the predicted controller as its step |
| Events for the frame, and effects under way | The SDK, as a presentation product published beside each tick page, which C02 allows. It is not world state and is in no digest | The ring of recent ticks' events, each frame function's cursor, and the bursts still playing on this screen |
| The shells | `PLATFORM-RUNTIME-*`, `CREATOR`, `GAME-UI` WP6 | `run()` picks the one for the target |
| The game's scenes, rules, looks and content | The game | — |

### A tick, walked

1. The driver (the fixed clock alone, the session in a match) names a tick to commit.
2. Input is admitted: each player's frame, the commands for this tick, the roster's events, and at a scene's
   first tick `PlayerJoined` for every player already in the game.
3. The space indexes are built, one for each layer, from where things stood.
4. The running scene's rules run in order, writing in place.
5. The engine resolves spawns and despawns, checks every number is finite, and commits. A refused tick leaves
   the world as it was.
6. The tick page is published, and with it the tick's events for the frame functions that read them. Only the
   rows that changed are extracted to sprite and entity rows.

### A frame, walked

1. The shell is asked for a frame and samples the bindings.
2. The engine chooses the pair of ticks and the fraction for this picture and samples the scene. The frame
   functions run then, on that pair at that fraction. They set the camera and sky, describe the interface,
   read the events past their cursors and send commands.
3. The interface is reconciled with last frame's and laid out only if a box could have moved.
4. The engine draws from the same pair at the same fraction: the scene's passes for its kind, interpolating on
   the GPU, then exposure, tone, the interface stage and present.
5. The SDK reads whether the picture was submitted or dropped. A dropped picture runs nothing again; a burst
   it would have shown first is kept for the next
   ([frame functions](#4-frame-functions-and-the-frame)).

Rules run on the world's own thread where the shell has one, as today. Frame functions never wait for them:
they read published pages, on the thread that samples the scene.

### From an empty folder

`./dev new` writes the template. `cargo play` cooks, builds and runs it. Startup validates the whole game before
a window opens. An edit to an asset or a parameter file shows in the running game. An edit to code rebuilds and
restarts.

## Interfaces

**Provided.**

| Interface | To | What it is |
|---|---|---|
| **The `thrum` crate** | Every game, the kit scenes, the templates | Mechanisms 1 to 14 |
| **The game commands** | Every game | Mechanism 16, and `./dev new` |
| **The SDK guides and the one file** | Game makers and their agents | Mechanism 15 |

**Asked of other modules.** Each is new work inside state another module owns, built by this module's builder
under [the seam rulings](../ARCHITECTURE.md#seam-rulings-for-game-sdk), on what the owner has landed, and the
owner's afterwards. No contract's text changes. C01's stable identities, C02's committed state, C11's event
order and C13's authority are what the mechanisms above stand on.

1. **The rules profile** (`SIM-2D`): literal construction, angles and turning, and `thrum`'s rules half as a
   dependency a rule crate may have; refusals of files, sockets, processes and standard input; and a check of a
   rule crate that a game's own manifest names, outside the engine's registry.
2. **The registered simulation** (core): queries over more than two components with filters, where the SDK
   cannot build them from columns as they are.
3. **The shells and the tool** (`CREATOR`, `HOST`, `GAME-UI`): what the window, the page and the phone shell
   still assume of the V1 game or the sky kit; the game commands in `./dev`. Whether each encoded picture
   was submitted or dropped is read from the game contract's readback of the frame host, as built.
4. **The repository guard** (`REPO-RULES`, `GAME-SPLIT`): `thrum` registered; the external game check admits a
   path between two packages of one game; and [the rule](#1-one-crate-two-halves) that every package of a
   game, a kit game or a template names no engine crate but `thrum`, with the registry's list of packages on
   the engine's own API. The showcase is on that list until WP9, so it passes the external game check on
   today's API whichever of the move and WP8 lands first.
5. **The 3D scene's entity rows** (`thrum-render`, and `CONTENT-LOOP`'s cooked mesh rows): the turn and the
   scale at both ticks, in the extraction and the row, interpolated by every pass that places an entity row;
   and a row's mesh chosen by name when its entity is spawned, where a slot's mesh is fixed when the scene is
   prepared today. The cooked placement and its draw paths are used as built, for scenery, and are not
   changed.

## Cost

The SDK is a layer over engine paths that are already costed. It must add nothing that scales with the number
of entities beyond what the engine does.

| Work | Account |
|---|---|
| A rule | One call each tick through the engine's registered callback, into a function monomorphised for the rule's parameters. The loop over entities is inside it: no dynamic call for each entity and no allocation in a tick |
| A query | A walk over the engine's columns in slot order. `par_each` uses the engine's chunks |
| A space query | One index for each layer, built once a tick with its storage kept. A query costs what the layer it asks holds near the point, and `nearest` with no radius what the whole layer holds |
| Startup | Signatures to access declarations, once |
| Extraction | Changed rows only, mapped by code written once in the SDK |
| A frame function | Runs once a frame, over what it names |
| The interface | A description of tens to a few hundred nodes compared with the last one. An unchanged interface draws once |
| Asset names | Resolved once for each changed row |
| A capability left out | Nothing: a 2D game compiles no 3D pass, a game with no interface has no stage, a game without the `multiplayer` feature links no network crate |

**Tripwires**, recorded and investigated while building goes on; none is a pass condition beyond a missing
measurement or a collapse:

- the reference crowd's behaviours at 10,000 bodies as SDK rules cost about 25% more a tick than the hand-written
  reference, the brief's figure for a comparable regression;
- any allocation in a warmed tick or frame of the 2D starter that the same scene on the engine's API does not
  make;
- a frame of the showcase on the SDK that costs about 25% more than the same frame on today's API.

Tuning belongs to the performance phase the brief puts after the exit.

## Physics, animation and audio

Decided. The owner, 25 November 2026, asked whether the first release should keep the planner's default (2D
queries and sprite animation in; rigid bodies, skeletal animation and audio out) or add physics and audio,
answered: "Planner's default". So the third column is what is built, and nothing here is left open.

| | In the engine today | The first release |
|---|---|---|
| **Physics** | 2D overlap, nearest and sweep queries (`SIM-2D`). No rigid bodies, no joints, no 3D collision | **Queries in, rigid bodies out.** `Collider2d` and `Space2d` carry the swarm game and most arcade games. A rigid-body module has to be deterministic on three targets to work with a match, which rules out wrapping an existing library as it stands; it is its own module, planned after the first release. 3D collision queries come with it |
| **Animation** | Sprite frames by name (`RENDER-2D`). No skeletons, no clips; the showcase's entities do not animate | **Sprite animation in** (`Sprite::animated(..)`, timed from the tick). **Skeletal animation out**: it needs skinning in the renderer and clips in the cooker, a module of its own |
| **Audio** | Nothing. A1 is a design | **Out.** It needs an engine module first, and no agent can hear its result, so every item would wait for the owner. The SDK keeps the place for it: a sound is played from a frame function on an event, as `flash_where_enemies_die` draws |

Each of the three that is out comes after the first release, as an engine module of its own, planned and built
before the SDK work package that surfaces it.

## The work packages

Ten, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds the
items of the first two.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Mechanisms 2, 3 and 5 with no window, for [the headless subset](#what-wp1-compiles): components, resources, rules, entities, events, scenes, the test harness; the surface and documentation checks | Nothing |
| WP2 | Mechanisms 6 and 8, `Space2d` and `per_player`; the profile's refusals of files and sockets and the check of a game's own rule crate; the crowd on the SDK, equal on three targets, with its cost | `SIM-2D:WP3`, Done |
| WP3 | 2D on screen: mechanisms 4, 7 (2D), 9, 14 and 16 for the Mac and Chrome, with the first `run()` and its startup checks beside `test()`'s; Rockfall, whose score and result text arrive with WP6 | Nothing still open |
| WP4 | 3D on screen: mechanism 7 (3D), with entities' models drawn as entity rows that turn and scale between ticks ([ruling 8](../ARCHITECTURE.md#seam-rulings-for-game-sdk)), scenery, and mechanism 10. The starter, stones and the lighthouse are written on the SDK as new kit games, and the old ones stay beside them until WP10 | For its change to the instance passes, the V1 work packages that hold them, which [the module list](../ARCHITECTURE.md#off-the-v1-path-track-7) names |
| WP5 | Mechanism 12: the crowd hosted and joined | `SESSION:WP7` |
| WP6 | Mechanism 11 | `GAME-UI:WP4` |
| WP7 | The iPhone simulator | `GAME-UI:WP6` |
| WP8 | Mechanism 15, [the guard](#1-one-crate-two-halves) that every package of a game, a kit game or a template names no engine crate but `thrum`, and three fresh agents | Its own WP1 to WP7 |
| WP9 | The showcase on the SDK, in the games repository, and taken off the guard's list in the same landing | `GAME-SPLIT:WP4`, which follows the V1 exit |
| WP10 | The old creator route removed: the old starter, stones and lighthouse with their entries on the guard's list, the sky kit, `./dev starter` and `./dev scene`. The engine's pointers and the maturity ledger | Its own WP9 |

Each work package's surface arrives with its reference documentation and its rows of
[the mistakes table](#13-mistakes-that-explain-themselves), because WP1's guard holds every later one to them.
A work package is compared with the worked examples only for the surface it builds: WP1 for the subset below,
WP2 for example D's rules, WP3 for example A, WP4 for examples B and C, WP5 for example E and WP6 for the
interface in A, D and E.

What the planner who gives a later work package its items carries into them, from the design reviews:

- **WP3:** a picture the engine encodes and drops, with an event delivered to its run and a command sent in
  it. The burst started there is in the next submitted picture, the command is admitted once, and the event
  is delivered to no later run.
- **WP4:** example B's cooked rock, captured in motion: an entity whose `Model` names a cooked mesh, standing
  in one place and turning, with its lit faces and its cast shadow turning together between ticks. A built-in
  cube does not stand in for it.
- **WP5:** a per-player rule that moves its character only on every other tick, predicted for a player who
  joined at a tick other than zero, with the predicted pose equal to the host's at every confirmed tick.
- **WP8:** the guard refuses a game whose main package names `thrum` and whose helper package names
  `thrum-render`, and a kit game that names `thrum-app` and no `thrum`; it passes the proving ground and,
  until WP9, the showcase on today's API.

### What WP1 compiles

WP1 has no window, no input and no space query, so it cannot compile example A or example D as written. Its
test games are written from this subset and no more:

- **The game and its scenes:** `Game::new`, `.scene(..)`, `.max_entities(..)`, `.tick_rate(..)` and `.test()`;
  in a scene, `resource`, `component`, `on_enter`, `rules` and `add`. No camera, background, binding or frame
  function: every WP1 scene is headless, which mechanism 13 allows.
- **Rule parameters:** `Query` with `With` and `Without`, `Res`, `ResMut`, `Entities`, `Attach`, `Events` and
  `Emit` for the game's own events, `Tick`, `Random` and `Scenes`; `when` and `par_each`.
- **Data:** the `Component`, `Resource` and `Data` derives, bundles and template functions, with plain
  components in place of transforms and looks. `Num` and `Vec2` are the profile's numbers as they stand today,
  made from integers. Literals, angles, `Vec3` and `Quat` arrive with WP2.
- **The harness:** `step`, `step_for`, `spawn`, `set`, `count`, `get`, `resource`, `digest` and `rewind`.
  `run()` arrives with WP3.

From the examples, that is: example A's `Score`, `drop_rocks`, `fall` and its two scenes joined by `go_to`,
with a plain position in place of `Transform2d` and a comparison of positions in place of `Space2d`; and
example D's `Run` and `Phase`, `enemies_die`, `start_wave`, `end_wave`, the `in_wave` and `in_intermission`
conditions and its grouped rule list under `when`. Input, commands, players, `per_player`, `Space2d` and the
numbers' literals are WP2's. Looks, cameras, frame functions and the parity of `run()` with `test()` are WP3's.

## Risks

1. **The API in the examples cannot be built as written on the engine.** The likeliest places: typed parameters
   over core's permits, `per_player` as a predicted controller, frame functions run on the pair the engine
   chose, and the interface kept from a per-frame description. Early sign: WP1's first item, WP3, WP5 and WP6. The builder writes the difference in the log and
   ends `BLOCKED`, because a change of shape is the owner's to approve.
2. **A convincing fake of K2:** an example that reads plainly because the plumbing moved into a helper file of
   the game. Early sign: the reviewer's count of game lines includes every file of the game.
3. **Angles in the profile differ on one target.** Early sign: WP2's conformance run. The fallback is a table.
4. **Compile errors from typed parameters read badly**, as Bevy's once did. Early sign: WP1's mistakes item,
   which holds the message text.
5. **The layer costs the crowd its speed.** Early sign: WP2's cost record.
6. **The docs teach what is not built.** Early sign: a guide's code block that does not compile fails the docs
   check.
7. **Turning models change passes the V1 exit stands on.** Early sign: an exact image that moves in WP4. A
   row that does not turn is placed exactly as today, and WP4's change to those passes waits for the V1
   work packages that hold them. A second sign: the turn is proved on a built-in cube while the cooked rock
   is drawn another way and stands still, which is why WP4 is judged on the cooked rock.
8. **A wrong substitute for K7:** a fresh agent that succeeds because it read the examples' source in the
   engine checkout. Its brief allows the guides, the one file and the template, and its record lists every file
   it opened.

## Proof

- **Correctness**, where a fault is silent and costly: rule order and what a rule sees; equal digests when a
  run is repeated, rewound and replayed, and on the three targets; a refused tick leaving the world unchanged;
  the access checks; a match that stays equal. CPU tests on small worlds.
- **Equal digests are never the only witness.** A digest that left something out would agree with itself. So
  a refused tick is proved by reading the components, the resources and the entities back, value by value;
  the digest is shown to change when any one of them changes; and the crowd's trace carries witnesses of the
  work it claims, each of which must be more than nothing on every target and in the takes that are costed:
  four heroes each moved by their own player's input, hits, deaths, respawns, a body turned by an angle, and a
  tick a rule refused.
- **Mistakes**: one test for each row of [the table](#13-mistakes-that-explain-themselves), holding the
  message's object, cause and next step.
- **The surface**: no engine type in `thrum`'s public items; an example on each; guides that compile.
- **Looks**: the kit games drawn on the Mac and in Chrome, inspected in motion through T2 capture.
- **The docs**: three fresh agents, commissioned by the lead as `CREATOR`'s was, one for each kind of game.
- **Cost**: the tripwire records above.

## What a builder may change

The worked examples are the shape the owner approved. A builder may change a **spelling**: a method's name, an
argument's order, a derive's attribute, where the game's code would read as well or better. Each is a line in
the log. A builder may not change the **shape**: add a concept a game maker must learn, remove one of the twelve
words, change what a rule sees or when spawns land, change what a command or a per-player rule promises, move
a line of plumbing into game code, or make a game name an engine crate. Where the shape cannot be built, the builder ends `BLOCKED` with the difference, and the
planner brings it to the owner.

When a work package is Done its examples in this design are replaced by the ones that compile, so the design
keeps reading as what is true.

## Open questions

- **How many entities a world holds by default.** WP2's builder chooses from the cost record and documents it.
- **Whether a per-player rule's limits carry every game's own-character movement.** It may read its
  character's components, its player's input and the tick. A game whose character's step must read more of the
  world, such as walls, is not served by it in the first release and moves that character with an ordinary
  rule, shown at the confirmed tick. WP5 reports what the crowd's players needed.
- **Whether a single-crate game can be checked for portable rules**, so that multiplayer needs no rule crate.
  WP5 reports what the profile's lints can do on a module.
- **How the rule-set identity is derived with no line of game code.** `SESSION` hashes the rule sources in a
  build script of the game, and a rule crate may have none. WP5 settles where the template carries it.
- **3D collision queries** come with the physics module, after the first release.
- **Whether the showcase needs a textured model that moves.** The exit scene's entities have none today.
  WP9 reports any it finds, and textures on entity rows are then planned as engine work.
