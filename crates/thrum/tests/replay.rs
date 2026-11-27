use std::sync::atomic::{AtomicBool, Ordering};

use thrum::prelude::*;

#[derive(Component)]
struct Body(u32);
#[derive(Resource, Default)]
struct Counter {
    value: u32,
    draw: Num,
    other: Num,
}
#[derive(Scene, Default)]
struct Play;
#[derive(Scene)]
struct Over(u32);
#[derive(Phase)]
enum Stage {
    A,
    B,
    C,
}

fn first_random(mut random: Random, mut value: ResMut<'_, Counter>) {
    value.draw = random.between(Num::ZERO, Num::ONE);
}
fn second_random(mut random: Random, mut value: ResMut<'_, Counter>) {
    value.other = random.between(Num::ZERO, Num::ONE);
}

fn initial(mut entities: Entities<'_>) {
    entities.spawn((Body(0),));
}

fn change(
    mut entities: Entities<'_>,
    mut bodies: Query<'_, (Entity, &mut Body)>,
    mut count: ResMut<'_, Counter>,
) {
    count.value += 1;
    for (entity, body) in &mut bodies {
        body.0 += 5;
        if body.0 >= 10 {
            entities.despawn(entity);
        }
    }
    entities.spawn((Body(1),));
}

fn play(scene: &mut Scene<Play>) {
    scene.on_enter(initial);
    scene.phases(Stage::A);
    scene.phase(Stage::A).lasts(seconds(2)).then(Stage::B);
    scene.phase(Stage::B).rules(
        |tick: Tick, mut scenes: Scenes<'_>, counter: Res<'_, Counter>| {
            if tick.seconds() >= seconds(4) {
                assert!(scenes.go_to(Over(counter.value)));
            }
        },
    );
    scene.phase(Stage::C);
    scene.rules((change, first_random, second_random));
}

fn game(seed: u64) -> Result<thrum::TestGame> {
    Game::new("replay")
        .resource(Counter::default())
        .tick_rate(1)
        .scene(play)
        .scene(|scene: &mut Scene<Over>| scene.rules((first_random, second_random)))
        .test_setup()
        .seed(seed)
        .start()
}

#[test]
fn digests_and_random_streams_replay() -> Result {
    let mut first = game(17)?;
    let mut second = game(17)?;
    let mut other_seed = game(18)?;
    let mut digests = Vec::new();
    for _ in 0..6 {
        first.step()?;
        second.step()?;
        other_seed.step()?;
        let left = first.resource::<Counter>()?;
        let right = second.resource::<Counter>()?;
        assert_eq!(left.draw, right.draw);
        assert_ne!(left.draw, left.other);
        assert_ne!(left.draw, other_seed.resource::<Counter>()?.draw);
        assert_eq!(first.digest(), second.digest());
        digests.push(first.digest());
    }
    first.rewind(5)?;
    for expected in digests.iter().skip(1) {
        first.step()?;
        assert_eq!(first.digest(), *expected);
    }
    Ok(())
}

#[test]
fn digest_changes_independently_for_world_fields_and_lives() -> Result {
    let mut game = game(0)?;
    let entity = game.single::<With<Body>>()?;
    let before = game.digest();
    game.set(entity, Body(2))?;
    assert_ne!(game.digest(), before);
    game.set(entity, Body(0))?;
    assert_eq!(game.digest(), before);
    game.set_resource(Counter {
        value: 9,
        ..Counter::default()
    })?;
    let resource = game.digest();
    assert_ne!(resource, before);
    let new = game.spawn((Body(1),))?;
    let spawned = game.digest();
    assert_ne!(resource, spawned);
    assert!(game.despawn(new)?);
    let despawned = game.digest();
    assert_ne!(spawned, despawned);
    game.go_to_phase(Stage::C)?;
    assert_ne!(game.digest(), despawned);
    let phase = game.digest();
    game.go_to(Over(9))?;
    assert_ne!(game.digest(), phase);
    Ok(())
}

#[test]
fn later_error_and_panic_abort_the_whole_tick_and_can_retry() -> Result {
    for panic in [false, true] {
        let refuse = AtomicBool::new(true);
        let mut game = Game::new("rollback")
            .tick_rate(1)
            .resource(Counter::default())
            .scene(|scene: &mut Scene| {
                scene.on_enter(initial);
                scene.rules((
                    change,
                    |bodies: Query<'_, &Body>| {
                        assert_eq!(bodies.count(), 2);
                        assert_eq!(bodies.iter().map(|body| body.0).collect::<Vec<_>>(), [5, 1]);
                    },
                    move |mut entities: Entities<'_>, bodies: Query<'_, Entity>| {
                        assert!(
                            entities.despawn(
                                bodies
                                    .iter()
                                    .next()
                                    .expect("test fixture provides this value")
                            )
                        );
                    },
                    move || -> Result {
                        if refuse.swap(false, Ordering::Relaxed) {
                            assert!(!panic, "test panic");
                            return Err(Error::invalid("counter", "test refusal"));
                        }
                        Ok(())
                    },
                ));
            })
            .test()?;
        let entity = game.single::<With<Body>>()?;
        let before = game.digest();
        let message = game
            .step()
            .expect_err("test fixture deliberately refuses this operation")
            .to_string();
        assert!(message.contains("tick 1"), "{message}");
        assert!(
            message.contains(if panic { "test panic" } else { "test refusal" }),
            "{message}"
        );
        assert_eq!(game.digest(), before);
        assert_eq!(game.get::<Body>(entity)?, Body(0));
        assert_eq!(game.resource::<Counter>()?, Counter::default());
        assert_eq!(game.count::<()>(), 1);
        game.step()?;
        assert_eq!(game.get::<Body>(game.single::<With<Body>>()?)?, Body(1));
        assert_eq!(game.resource::<Counter>()?.value, 1);
        assert!(!game.alive(entity));
    }
    Ok(())
}
