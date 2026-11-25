# Game SDK: one crate a game is written against

25 November 2026. [Plan](PLAN.md) · [done-when file](DONE.json) · [log](LOG.md) ·
[module list](../ARCHITECTURE.md#track-7-the-game-sdk) · [glossary](../GLOSSARY.md) ·
[the creator guide](../../design/ENGINE-EXPERIENCE.md), whose ambition this module builds ·
[the performance policy](../../design/AI-PERFORMANCE.md#performance-investment-from-the-first-design) ·
[the hardware-native decision](../../design/decisions/HARDWARE-NATIVE-ENGINE.md)

**The owner approves this design before any builder starts.** A design review that returns Design Approved is
not that approval. The owner judges the API by reading [the worked examples](#worked-examples), and decides
[what the first release leaves out](#physics-animation-and-audio).

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
2. 2D and 3D under that one structure: sprites and shapes; cooked models, sky, Sun, ground and lights.
3. Input actions, commands and players.
4. Assets by name with live edits; settings on the tuning panel; the profiler.
5. The interface: status displays, menus and modal choices.
6. Multiplayer as an opt-in that changes no rule.
7. Errors that name the object, the cause and the next step.
8. Reference documentation with an example on every public item, guides, templates and a new-game command.
9. The showcase moved onto the SDK, after the V1 exit.

**Not in the first release**, each said where it bears on a game maker:

- Rigid-body physics, skeletal animation and audio. [The owner decides](#physics-animation-and-audio); this plan
  assumes they are left out.
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
| **Command** | A message a player sends once, such as a menu choice. It arrives exactly once, in order | `send.command(Choice::Upgrade(upgrade))` |
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
`cargo play --web` serves it to Chrome.

### B. A first 3D scene

Today's starter (a cooked rock on lit ground under the physical sky, with Sun sliders) with one rule added, so it
is a game and not a still.

```rust
use thrum::prelude::*;

fn main() -> thrum::Result {
    Game::new("Thrum starter").settings::<Sun>().scene(clearing).run()
}

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

fn move_sun(mut sky: Local<Sky>, sun: Res<Sun>) {
    sky.sun = SunAngle::degrees(sun.elevation, sun.azimuth);
}
```

The sky, the Sun's light and shadows, exposure, tone mapping, reconstruction and the governor are the engine's
defaults. The two sliders appear in the Mac panel and on the Chrome page, a parameter file sets them, and the
profiler is a toggle beside them.

### C. A slice of the V1 showcase

The exit scene: the cooked valley, 10,000 entities of which 1,500 drift, six stops and the hour of the day.

```rust
use thrum::prelude::*;

const NEXT_STOP: Button = Button::new("next stop");

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
    scene.camera(STOPS[0].camera());
    scene.bind(NEXT_STOP, (Key::Tab, Touch::tap()));
    scene.resource(Tour::default());
    scene.on_enter((place_terrain, populate));
    scene.rules((drift, tour));
    scene.each_frame((sun_follows_hour, fly_to_stop));
}

fn place_terrain(mut entities: Entities) {
    entities.spawn(Model::new("showcase/valley").texture("showcase/ground"));
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

fn sun_follows_hour(mut sky: Local<Sky>, day: Res<Day>) {
    sky.sun = SunAngle::at_hour(day.hour, VALLEY_LATITUDE);
}

fn fly_to_stop(mut camera: Local<Camera3d>, tour: Res<Tour>, frame: Frame) {
    camera.ease_to(STOPS[tour.stop as usize].camera(), frame.dt());
}
```

`STOPS` and `layout` are the game's own data. The 8,500 props that no rule writes cost nothing after they are
placed, and `par_each` spreads the movers over the engine's worker threads. GPU culling, cooked mesh rows, the
recipe for this Mac and the Chrome build are not in the game's code at all.

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

#[derive(Command)]
pub enum Choice {
    Upgrade(Upgrade),
}

#[derive(Data)]
pub enum Upgrade {
    FasterFire,
    MoreDamage,
    Heal,
}

pub fn arena(scene: &mut Scene) {
    scene.space2d(Space2d::cells(4.0)); // part of the rules, so the same for every player
    scene.resource(Run::default());
    scene.rules((
        heroes_join_and_leave,
        start_wave,
        (
            per_player(move_hero),
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
        Hero { fire_every: seconds(0.4), damage: 1 },
        player,
        Health(100),
        Velocity(Vec2::ZERO),
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

/// A hero walks where its own player steers. A per-player rule reads only that player's entity and input,
/// so in a match the engine can run it ahead for the local player and their hero answers at once.
fn move_hero(velocity: &mut Velocity, input: &Input) {
    velocity.0 = input.stick(MOVE) * 6.0;
}

fn spawn_enemies(run: Res<Run>, heroes: Query<&Transform2d, With<Hero>>, mut entities: Entities, mut random: Random) {
    for _ in 0..run.wave * heroes.count() as u32 {
        let Some(near) = random.pick(&heroes) else { return };
        entities.spawn(bat(near.position + random.direction() * 30.0));
    }
}

fn steer_enemies(mut enemies: Query<(Entity, &Transform2d, &mut Velocity), With<Enemy>>, space: Space2d) {
    enemies.par_each(|(me, at, mut velocity)| {
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

fn take_choices(choices: Commands<Choice>, mut heroes: Query<(&Player, &mut Hero, &mut Health)>) {
    for (player, Choice::Upgrade(upgrade)) in choices.read() {
        for (owner, mut hero, mut health) in &mut heroes {
            if *owner != player {
                continue;
            }
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

fn upgrade_cards(mut ui: Ui, run: Res<Run>, mut send: Send) {
    if run.phase != Phase::Intermission {
        return;
    }
    ui.modal("Choose an upgrade", |ui| {
        for upgrade in [Upgrade::FasterFire, Upgrade::MoreDamage, Upgrade::Heal] {
            if ui.card(upgrade.name(), upgrade.blurb()).chosen() {
                send.command(Choice::Upgrade(upgrade));
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

/// Only the rules under test: a scene is a function, so a test builds the one it needs.
fn fight(scene: &mut Scene) {
    scene.space2d(Space2d::cells(4.0));
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
`inputs.of(player)`, the upgrade choice is a command, and the numbers agree on every device. A game played
alone is the same game with one player, who joins at the first tick.

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
  tick, random draws, space queries, the maths, and the look components as data. It builds with no renderer,
  window, GPU, network or platform crate in its dependency closure. A headless server, a test and a rule crate
  use only this half.
- The play half is `run()`, scenes' cameras and sky, frame functions, the interface, settings, assets and the
  session. The `multiplayer` Cargo feature, off by default, is the only way `thrum-network` and
  `thrum-transport` enter a game's closure.

How the two halves are laid out in crates is the builder's, registered in `.config/architecture.toml`. The
rule that binds: a crate that names `thrum` with default features off gets the rules half and compiles alone.

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

Components and resources are registered from the signatures of the rules that name them. A game writes no list
of them.

### 3. Rules, and the tick

A rule is a function. Its parameters say what it reads and writes, and the SDK turns them into the access
declaration the engine's schedule validates.

| Parameter | What it gives |
|---|---|
| `Query<D, F>` | The entities that have `D`, filtered by `With<T>` and `Without<T>`, visited in a fixed order. `&T` reads, `&mut T` writes, `Entity` names |
| `Res<T>`, `ResMut<T>` | A resource |
| `Entities` | `spawn`, `despawn`, `insert`, `remove` |
| `Inputs` | Each player's actions for this tick |
| `Commands<T>` | The commands of type `T` admitted at this tick, with who sent each, in order |
| `Events<T>`, `Emit<T>` | Events of this tick: `PlayerJoined`, `PlayerLeft`, `PlayerAway`, `PlayerBack`, and the game's own |
| `Tick` | The tick's number, its length `dt()`, and `every`, `after`, `reached` |
| `Random` | Draws keyed by the game's seed, the tick and what is asked, so a replay draws the same |
| `Space2d` | Overlap, nearest and sweep queries over entities with a `Collider2d`, by layer |
| `Scenes` | `go_to(scene)` |

**What a rule sees.** Rules of the running scene run once each tick, in the order the scene lists them. A rule
sees what earlier rules of the same tick wrote to components and resources. **Spawns and despawns land when the
tick ends**: an entity spawned in a tick is first visited in the next, and one despawned is still visited by
later rules of its last tick. `Space2d` answers from where things stood when the tick began. This is the
engine's own shape (in-place writes that roll back together, with structural changes at the barrier), so the SDK
adds no machinery to provide it, and the guides state it in these words.

**What a rule may not do.** Read a clock, a file, the network or anything else outside its parameters. Nothing a
rule can be handed leads there: its parameters are the whole of what it can reach. `per_player(rule)` is
narrower still: its function is given one player's entity and that player's input, and nothing else.

**Parallel work** is inside a rule: `query.par_each(..)` runs the closure over the engine's chunks. The closure
writes only the entity it is given and reads the rule's other parameters. The result is the same as the serial
loop, by construction.

**Conditions.** `rules.when(condition)` runs a rule or a group only in ticks where a read-only function returns
true.

**Failure.** A rule returns nothing, or a `thrum::Result`. An error refuses the tick: the world stays as it was
at the last tick, and the game stops with the rule's name and the error. A panic in a rule is reported the same
way.

**The tick** is 60 Hz unless `Game::tick_rate` says otherwise, the brief's "configurable, not an engine
ceiling". Speeds are written per second and multiplied by `tick.dt()`.

### 4. Frame functions, and the frame

A frame function runs once for each picture, on the thread that owns the window. It reads the world **as
shown**: transforms are where the picture has them, between the last two ticks, and everything else is as of the
newer tick. It writes only what belongs to this player's screen.

| Parameter | What it gives |
|---|---|
| `Query<D, F>`, `Res<T>` | The world as shown, read-only |
| `Events<T>` | Events of the ticks first shown since the last frame, each once |
| `Local<T>` | State of this screen: the camera, the sky, the game's own menu state. Never part of the world |
| `Ui` | The interface for this frame |
| `Draw2d` | Sprites and shapes for this frame only, drawn among the world's |
| `Send` | `command(..)`: the one way a frame function affects the world |
| `Me`, `Session` | Which player this screen is, and the match |
| `Frame` | The frame's time and its `dt()` |
| `Go` | `go_to(scene)`, as a command |

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

### 6. Actions, commands and players

- An **action** is a constant: `Button`, `Axis` (one number from −1 to 1) or `Stick` (two). A scene binds it to
  keys, the pointer and touch. The shell samples bindings, and each tick admits one input frame for each
  player. A rule reads `inputs.of(player)`: `held`, `pressed` (went down this tick), `axis`, `stick`.
  `inputs.first()` is the first player, for a game played alone.
- A **command** is a value of a type that derives `Command`, sent from a frame function. It is admitted at one
  tick, in order, exactly once. Alone it arrives at the next tick. In a match it is `SESSION`'s ordered command.
- A **player** is a slot. `PlayerJoined` arrives as an event at the tick a player enters, and alone that is the
  first tick. A game that spawns its characters from `PlayerJoined` is ready for a match without knowing it.
- Settings the rules read are world state, changed by a command the panel sends.

Input reaches rules only in these three forms, which is `SWARM`'s
[three drivers](../SWARM/DESIGN.md#7-who-admits-the-frames-alone-and-in-a-match): alone, hosting and joined, the
rules cannot tell which. The engine's input frame carries a fixed number of axes and at most eight timed actions
of up to 256 bytes ([the guide](../../SESSION.md)). A game whose actions cannot fit is refused at startup, with
the count.

### 7. Looks: what is drawn

A look is a component, and an entity with a look and a transform is drawn. Looks are world state like any
component.

| | 2D | 3D |
|---|---|---|
| Where | `Transform2d`: position, turn, scale | `Transform3d`: position, rotation, scale |
| What | `Sprite` (a frame of a sheet, by name: layer, tint, flash, glow, animation), `Shape2d` (a circle, a box, a line, with no asset) | `Model` (a cooked mesh by name, its texture and material; `Model::cube()` and `Model::sphere()` with no asset), `Light` |
| Camera | `Camera2d`: a centre and how much of the world is shown | `Camera3d`: an eye, a target and a field of view |
| Around it | `scene.background(colour)` | `scene.sky(..)`, `scene.ground(..)` |
| Touching | `Collider2d` and `Space2d` | Not in the first release |

- Extraction is the SDK's, written once: it maps the looks of entities whose rows changed to
  `RENDER-2D`'s sprite rows and to cooked mesh rows, and the engine interpolates on the GPU. An entity no rule
  writes is extracted once.
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
  map's order, an outside crate. In a rule crate, the profile's lints refuse those with a message that says
  why, as they do today. A single-crate game is not linted, and is told so when it turns multiplayer on.

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

- `Game::settings::<T>()` makes `T` world state: rules and frame functions read `Res<T>`, and a panel edit
  enters as a command. In a match the host's edits are the ones admitted.
- `Game::local_settings::<T>()` makes `T` this screen's: frame functions read `Local<T>`.

### 11. The interface

A frame function describes the interface with `Ui`: places (`top_left`, `center`, a modal), and in them text,
bars, images, buttons and cards. The SDK keeps [GAME-UI](../GAME-UI/DESIGN.md)'s tree and changes only what
differs from the last frame, so an interface that did not change costs what `GAME-UI` says it does: one draw.
`button(..).clicked()` is true in the frame after `GAME-UI` hands the game an activation. A modal is an
exclusive scope: while it is open the game's actions are let go, and what is still held comes back when it
closes. Focus, direction keys, touch sizes, safe areas and density are `GAME-UI`'s and need nothing from the
game.

Text exists at the sizes a font was cooked at. The SDK's default font and sizes ship with it, so a first game
needs no font.

### 12. Multiplayer as an opt-in

`Game::multiplayer(Multiplayer::players(1..=4))` with the `multiplayer` feature. The SDK then implements
`SESSION`'s world for the game: the rule-set identity from the game's rules and build, the world image and
digest from the registered state, the action wire from the declared actions and commands, the roster as player
events, and the driven world. A game writes none of the steps of [the session guide](../../SESSION.md).

- **Who does what** is `SESSION`'s and unchanged: a native process hosts, browsers and the simulator join, the
  crowd is shown at the confirmed tick, and a peer too slow is dropped.
- **The local player's own character answers at once** when its movement is a `per_player` rule: the SDK gives
  that function to `SESSION`'s predicted controller, with the components it writes as the controller's state.
- **`Session`** in a frame function: `host()`, `join(ticket)`, `state()`, `leave()`. A ticket arrives as a link,
  a paste or a launch argument.
- **A game that can be played online keeps its rules in a rule crate**, as example D does, and
  `cargo play --check` holds that crate to the profile. The `multiplayer` template is laid out so. A game whose
  rules are not in one is told at startup, with how to move them.

### 13. Mistakes that explain themselves

Every message names the object, the cause and the next step. The listed mistakes, each proved by a test in the
work package that builds its surface:

| When | Mistake | The message says |
|---|---|---|
| Compile | A rule takes `Ui`, `Local<T>`, `Draw2d`, `Send`, `Me`, `Session` or `Frame` | That a rule must give the same result on every player's machine, and to move this to a frame function |
| Compile | A frame function takes `ResMut<T>`, `Entities`, `Emit<T>` or a `&mut` query | That frame functions cannot change the world, and to send a command or write a rule |
| Compile | A component holds a `Vec`, a `String`, a reference or a raw float | That components are plain data, and what to use |
| Compile | `inputs.stick(..)` on a `Button`, and the like | Rust's own type error, on the constant |
| Startup | Two parameters of one rule write the same component | The rule and the component, and to split the query with `Without` |
| Startup | A rule reads a resource no scene provides | The rule, the resource and `scene.resource(..)` |
| Startup | A rule reads an action no scene binds | The action and `scene.bind(..)` |
| Startup | The game's actions do not fit an input frame | The counts and the limit |
| Startup | A scene has no camera, or a 2D camera with 3D looks | The scene and what to add |
| Startup | No asset store, or a missing prerequisite | What is missing and the command that provides it |
| Running | `go_to` a scene the game was not given | The scene and `.scene(..)` |
| Running | A spawn past the world's size | The size and `Game::max_entities` |
| Running | An asset name that matches nothing | The name, the rule and the nearest names |
| Running | A rule fails or panics | The rule, the tick and the error |
| Running | A session is refused or closed | `SESSION`'s reason, in words a player can read |

Startup checks run in `run()` and in `test()`, before a window opens. `thrum::Error` is one enum with a variant
for each kind of remedy, in [the adopted error shape](../../../.agents/skills/thrum-engineering/references/patterns.md).

### 14. Tests without a window, and pictures for an agent

`Game::test()` returns the game with no window, device or clock: `join()`, `hold` and `press` for a player's
actions, `send` for a command, `step()` and `step_for(..)`, `spawn`, `set`, `count`, `get`, `resource`,
`digest()`, and `rewind(ticks)`. It runs the same rules, on the same engine path, as `run()`.

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
  outside the engine checkout and reaches `thrum` by path.
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
| `cargo play --check` | The rule crate's profile check, where the game has one |

The game's `main` is the same file on every target. The SDK supplies each platform's entry.

## The system as it runs

### State and owners

| State | Owner | The SDK's part |
|---|---|---|
| Entities, components, resources, the tick, rewind, publication | `thrum-core` (G1, G2S) | Registers what the rules name; adapts rule functions to registered systems |
| The rules profile, the digest, 2D queries | `SIM-2D` | `Num` and friends are its numbers; `Space2d` is its index |
| The frame, the recipe, reconstruction, EDR, the governor | `APPLE-TILE`, `APPLE-PRESENTATION` | None. A game gets them by default |
| Sprite rows, sheets, the 2D scene | `RENDER-2D` | Maps `Sprite`, `Shape2d` and `Transform2d` to rows |
| Sky, Sun, air | `ATMOSPHERE` | `Sky` sets its inputs |
| Cooked products and live edits | `CONTENT-LOOP`, `COOKED-DRAW` | Resolves names; maps `Model` to cooked rows |
| Typed controls, parameter files, the profiler | `CREATOR` | `Settings` declares controls |
| The interface tree, events, fonts | `GAME-UI` | `Ui` keeps the tree |
| The device, pause, recovery, the game contract | `HOST` | Implements the game contract once, for every game |
| The match, the roster, prediction | `SESSION`, over `TRANSPORT` | Implements the session's world once |
| The shells | `PLATFORM-RUNTIME-*`, `CREATOR`, `GAME-UI` WP6 | `run()` picks the one for the target |
| The game's scenes, rules, looks and content | The game | — |

### A tick, walked

1. The driver (the fixed clock alone, the session in a match) names a tick to commit.
2. Input is admitted: each player's frame, the commands for this tick, the roster's events.
3. The space index is built from where things stood.
4. The running scene's rules run in order, writing in place.
5. The engine resolves spawns and despawns, checks every number is finite, and commits. A refused tick leaves
   the world as it was.
6. The tick page is published. Only the rows that changed are extracted to sprite and mesh rows.

### A frame, walked

1. The shell is asked for a frame and samples the bindings.
2. Frame functions run on the newest page pair. They set the camera and sky, describe the interface and send
   commands.
3. The interface is reconciled with last frame's and laid out only if a box could have moved.
4. The engine draws: the scene's passes for its kind, interpolating between the page pair on the GPU, then
   exposure, tone, the interface stage and present.

Rules run on the world's own thread where the shell has one, as today. Frame functions never wait for them:
they read published pages.

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
   dependency a rule crate may have.
2. **The registered simulation** (core): queries over more than two components with filters, where the SDK
   cannot build them from columns as they are.
3. **The shells and the tool** (`CREATOR`, `HOST`, `GAME-UI`): what the window, the page and the phone shell
   still assume of the V1 game or the sky kit; the game commands in `./dev`.
4. **The repository guard** (`REPO-RULES`, `GAME-SPLIT`): `thrum` registered, and a rule that a game and a kit
   scene name `thrum` and no other engine crate.

## Cost

The SDK is a layer over engine paths that are already costed. It must add nothing that scales with the number
of entities beyond what the engine does.

| Work | Account |
|---|---|
| A rule | A direct call to a monomorphised function. No boxed call for each entity, no allocation in a tick |
| A query | A walk over the engine's columns in slot order. `par_each` uses the engine's chunks |
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

The owner decides this. The plan assumes the recommendation, as [assumptions](PLAN.md#open-for-the-user).

| | In the engine today | Recommendation for the first release |
|---|---|---|
| **Physics** | 2D overlap, nearest and sweep queries (`SIM-2D`). No rigid bodies, no joints, no 3D collision | **Queries in, rigid bodies out.** `Collider2d` and `Space2d` carry the swarm game and most arcade games. A rigid-body module has to be deterministic on three targets to work with a match, which rules out wrapping an existing library as it stands; it is its own module, planned after the first release. 3D collision queries come with it |
| **Animation** | Sprite frames by name (`RENDER-2D`). No skeletons, no clips; the showcase's entities do not animate | **Sprite animation in** (`Sprite::animated(..)`, timed from the tick). **Skeletal animation out**: it needs skinning in the renderer and clips in the cooker, a module of its own |
| **Audio** | Nothing. A1 is a design | **Out.** It needs an engine module first, and no agent can hear its result, so every item would wait for the owner. The SDK keeps the place for it: a sound is played from a frame function on an event, as `flash_where_enemies_die` draws |

Bringing any of the three in changes the order: its engine module is planned and built before the SDK's work
package that surfaces it.

## The work packages

Ten, in the order they are built. [The plan](PLAN.md) names each and [the done-when file](DONE.json) holds the
items of the first two.

| Work package | Builds | Waits for |
|---|---|---|
| WP1 | Mechanisms 2, 3 and 5 with no window: components, resources, rules, entities, events, scenes, the test harness; the surface and documentation checks | Nothing |
| WP2 | Mechanisms 6 and 8, `Space2d` and `per_player`; the crowd on the SDK, equal on three targets, with its cost | `SIM-2D:WP3`, Done |
| WP3 | 2D on screen: mechanisms 4, 7 (2D), 9, 14 and 16 for the Mac and Chrome; Rockfall, whose score and result text arrive with WP6 | Nothing still open |
| WP4 | 3D on screen: mechanism 7 (3D) and mechanism 10; the starter, stones and the lighthouse on the SDK | Nothing still open |
| WP5 | Mechanism 12: the crowd hosted and joined | `SESSION:WP7` |
| WP6 | Mechanism 11 | `GAME-UI:WP4` |
| WP7 | The iPhone simulator | `GAME-UI:WP6` |
| WP8 | Mechanism 15, and three fresh agents | Its own WP1 to WP7 |
| WP9 | The showcase on the SDK, in the games repository | `GAME-SPLIT:WP4`, which follows the V1 exit |
| WP10 | The old kit removed, the engine's pointers and the maturity ledger | Its own WP9 |

Each work package's surface arrives with its reference documentation and its rows of
[the mistakes table](#13-mistakes-that-explain-themselves), because WP1's guard holds every later one to them.

## Risks

1. **The API in the examples cannot be built as written on the engine.** The likeliest places: typed parameters
   over core's permits, `per_player` as a predicted controller, and the interface kept from a per-frame
   description. Early sign: WP1's first item, WP5 and WP6. The builder writes the difference in the log and
   ends `BLOCKED`, because a change of shape is the owner's to approve.
2. **A convincing fake of K2:** an example that reads plainly because the plumbing moved into a helper file of
   the game. Early sign: the reviewer's count of game lines includes every file of the game.
3. **Angles in the profile differ on one target.** Early sign: WP2's conformance run. The fallback is a table.
4. **Compile errors from typed parameters read badly**, as Bevy's once did. Early sign: WP1's mistakes item,
   which holds the message text.
5. **The layer costs the crowd its speed.** Early sign: WP2's cost record.
6. **The docs teach what is not built.** Early sign: a guide's code block that does not compile fails the docs
   check.
7. **A wrong substitute for K7:** a fresh agent that succeeds because it read the examples' source in the
   engine checkout. Its brief allows the guides, the one file and the template, and its record lists every file
   it opened.

## Proof

- **Correctness**, where a fault is silent and costly: rule order and what a rule sees; equal digests when a
  run is repeated, rewound and replayed, and on the three targets; a refused tick leaving the world unchanged;
  the access checks; a match that stays equal. CPU tests on small worlds.
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
words, change what a rule sees or when spawns land, move a line of plumbing into game code, or make a game name
an engine crate. Where the shape cannot be built, the builder ends `BLOCKED` with the difference, and the
planner brings it to the owner.

When a work package is Done its examples in this design are replaced by the ones that compile, so the design
keeps reading as what is true.

## Open questions

- **How many entities a world holds by default.** WP2's builder chooses from the cost record and documents it.
- **Whether `per_player` can carry every game's own-character movement**, or some games need to say more.
  WP5 settles it with the crowd's players as the case.
- **Whether a single-crate game can be checked for portable rules**, so that multiplayer needs no rule crate.
  WP5 reports what the profile's lints can do on a module.
- **How the rule-set identity is derived with no line of game code.** `SESSION` hashes the rule sources in a
  build script of the game, and a rule crate may have none. WP5 settles where the template carries it.
- **3D collision queries** wait for the physics decision.
