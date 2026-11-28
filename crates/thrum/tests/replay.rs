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
    static REFUSE: AtomicBool = AtomicBool::new(false);
    for panic in [false, true] {
        REFUSE.store(true, Ordering::Relaxed);
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
                        if REFUSE.swap(false, Ordering::Relaxed) {
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

fn refuse_manual(
    mut bodies: Query<'_, (Entity, &mut Body)>,
    mut count: ResMut<'_, Counter>,
    mut entities: Entities<'_>,
) -> Result {
    for (entity, body) in &mut bodies {
        body.0 = 99;
        assert!(entities.despawn(entity));
    }
    entities.spawn((Body(2),));
    count.value = 99;
    Err(Error::invalid("counter", "manual refusal"))
}

#[test]
fn manual_reads_and_refusals_preserve_authority_and_history() -> Result {
    let mut game = game(41)?;
    game.step_ticks(3)?;
    let before = game.digest();
    let count = game.run(|count: Res<'_, Counter>| count.value)?;
    assert_eq!(count, 3);
    assert!(
        game.run(refuse_manual)
            .expect_err("rule refuses")
            .to_string()
            .contains("refuse_manual at tick 3")
    );
    assert_eq!(game.digest(), before);
    game.rewind(1)?;
    game.step()?;
    assert_eq!(game.digest(), before);
    Ok(())
}

#[derive(Resource, Default)]
struct Draws {
    condition: u32,
    rule: u32,
}

fn condition_draw(mut random: Random) -> u32 {
    random.below(u32::MAX)
}

fn rule_draw(mut random: Random) -> u32 {
    random.below(u32::MAX)
}

#[test]
fn function_pointer_rules_repeat_independently_after_rebuilding() -> Result {
    fn loot(mut random: Random) -> (&'static str, u32) {
        ("loot", random.below(u32::MAX))
    }
    fn weather(mut random: Random) -> (&'static str, u32) {
        ("weather", random.below(u32::MAX))
    }
    type Callback = fn(Random) -> (&'static str, u32);
    let rules: [Callback; 2] = [loot, weather];
    let setup = || {
        Game::new("pointer streams")
            .scene(|_: &mut Scene| {})
            .test_setup()
            .seed(17)
            .start()
    };
    let mut first = setup()?;
    let mut second = setup()?;
    for _ in 0..3 {
        let left = first.run(rules[0])?;
        let right = first.run(rules[1])?;
        assert_ne!(left.1, right.1);
        assert_eq!(right, first.run(rules[1])?);
        assert_eq!(left, first.run(rules[0])?);
        assert_eq!(left, second.run(rules[0])?);
        assert_eq!(right, second.run(rules[1])?);
        first.step()?;
        second.step()?;
    }
    first.rewind(1)?;
    second.rewind(1)?;
    assert_eq!(first.run(rules[0])?, second.run(rules[0])?);
    Ok(())
}

#[test]
fn scheduled_and_manual_calls_share_their_function_pointer_identity() -> Result {
    type Callback = fn(Random, ResMut<'_, Draws>);
    fn loot(mut random: Random, mut draws: ResMut<'_, Draws>) {
        draws.condition = random.below(u32::MAX);
    }
    fn weather(mut random: Random, mut draws: ResMut<'_, Draws>) {
        draws.rule = random.below(u32::MAX);
    }
    let rules: [Callback; 2] = [loot, weather];
    let mut game = Game::new("scheduled pointer streams")
        .resource(Draws::default())
        .scene(move |scene: &mut Scene| scene.rules((rules[0], rules[1])))
        .test()?;
    game.step()?;
    let scheduled = game.resource::<Draws>()?;
    assert_ne!(scheduled.condition, scheduled.rule);
    let digest = game.digest();
    game.run(rules[1])?;
    game.run(rules[0])?;
    assert_eq!(game.digest(), digest);
    game.rewind(1)?;
    game.step()?;
    assert_eq!(game.digest(), digest);
    Ok(())
}

#[test]
fn named_manual_rules_and_scheduled_condition_have_independent_streams() -> Result {
    let mut game = Game::new("condition and guarded rule")
        .resource(Draws::default())
        .scene(|scene: &mut Scene| {
            fn even(random: Random) -> bool {
                condition_draw(random).is_multiple_of(2)
            }
            fn guarded(random: Random, mut draws: ResMut<'_, Draws>) {
                draws.rule = rule_draw(random);
                draws.condition += 1;
            }
            scene.rules(guarded.when(even));
        })
        .test_setup()
        .seed(17)
        .start()?;
    let left = game.run(condition_draw)?;
    assert_eq!(left, game.run(condition_draw)?);
    assert_ne!(left, game.run(rule_draw)?);

    // If the condition and the rule share a stream, every recorded draw is even.
    for _ in 0..32 {
        let before = game.resource::<Draws>()?.condition;
        game.step()?;
        let draws = game.resource::<Draws>()?;
        if draws.condition != before && !draws.rule.is_multiple_of(2) {
            return Ok(());
        }
    }
    panic!("the selected rule reused the condition's even draw")
}

#[derive(Component)]
struct Pattern([u16; 4]);

#[derive(Resource)]
struct Pair((u32, bool));

#[test]
fn derived_schema_uses_resolved_fields_in_nested_aliases() {
    type Entries<T> = Option<[(T,); 2]>;
    #[derive(Component)]
    struct Score<T>(Entries<T>);

    assert_eq!(Score::<u32>::SCHEMA, Score::<u64>::SCHEMA);
    assert_ne!(Score::<u32>::SCHEMA_ID, Score::<u64>::SCHEMA_ID);
}

#[test]
fn portable_array_components_and_tuple_resources_rewind() -> Result {
    let mut game = Game::new("compound data")
        .resource(Pair((3, false)))
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((Pattern([1, 2, 3, 4]),));
            });
            scene.rules(
                |mut pattern: Single<'_, &mut Pattern>, mut pair: ResMut<'_, Pair>| {
                    pattern.0[2] += 1;
                    pair.0 = (4, true);
                },
            );
        })
        .test()?;
    let entity = game.single::<With<Pattern>>()?;
    let pattern = game.get::<Pattern>(entity)?;
    let copied = pattern;
    assert_eq!(pattern, copied);
    let start = game.digest();
    game.step()?;
    assert_eq!(game.get::<Pattern>(entity)?.0, [1, 2, 4, 4]);
    assert_eq!(game.resource::<Pair>()?.0, (4, true));
    let end = game.digest();
    assert_ne!(start, end);
    game.rewind(1)?;
    assert_eq!(game.digest(), start);
    game.step()?;
    assert_eq!(game.digest(), end);
    Ok(())
}

#[test]
fn a_panic_with_live_column_loans_restores_every_mutated_column() -> Result {
    #[derive(Component)]
    struct Other(u32);

    let mut game = Game::new("panic during column writes")
        .max_entities(2)
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((Body(1), Other(2)));
                entities.spawn((Body(3), Other(4)));
            });
            scene.rules(|mut rows: Query<'_, (&mut Body, &mut Other)>| -> Result {
                let (body, other) = rows.iter_mut().next().expect("two rows");
                body.0 = 10;
                other.0 = 20;
                panic!("after two writes");
            });
        })
        .test()?;
    let before = game.digest();
    assert!(
        game.step()
            .expect_err("rule panics")
            .to_string()
            .contains("after two writes")
    );
    assert_eq!(game.digest(), before);
    Ok(())
}

#[test]
fn rollback_and_rewind_restore_the_exact_component_encoding() -> Result {
    #[derive(Component)]
    struct Velocity(Option<[(Num,); 1]>);

    fn reverse(mut velocity: Single<'_, &mut Velocity>) {
        let value = &mut velocity.0.as_mut().expect("starting speed")[0].0;
        *value = -*value;
    }
    fn refuse() -> Result {
        Err(Error::invalid("velocity", "refused tick"))
    }
    fn direction(game: &thrum::TestGame) -> Result<i32> {
        let value = game.get::<Velocity>(game.single::<With<Velocity>>()?)?;
        Ok((Num::ONE / value.0.expect("starting speed")[0].0)
            .to_i32()
            .signum())
    }
    let setup = |refusing| {
        Game::new("velocity")
            .max_entities(1)
            .scene(move |scene: &mut Scene| {
                scene.on_enter(|mut entities: Entities<'_>| {
                    entities.spawn((Velocity(Some([(Num::ZERO,)])),));
                });
                scene.rules(reverse);
                if refusing {
                    scene.rules(refuse);
                }
            })
            .test()
    };
    let mut game = setup(true)?;
    let starting = game.digest();
    for _ in 0..2 {
        let error = game.step().unwrap_err();
        assert_eq!(error.code(), "T0401");
        assert!(error.to_string().contains("refuse at tick 1"));
        assert_eq!(game.digest(), starting);
        assert_eq!(direction(&game)?, 1);
    }

    let mut game = setup(false)?;
    let starting = game.digest();
    game.step()?;
    let reversed = game.digest();
    assert_eq!(direction(&game)?, -1);
    assert_ne!(reversed, starting);
    game.rewind(1)?;
    assert_eq!(direction(&game)?, 1);
    assert_eq!(game.digest(), starting);
    game.step()?;
    assert_eq!(game.digest(), reversed);
    Ok(())
}
