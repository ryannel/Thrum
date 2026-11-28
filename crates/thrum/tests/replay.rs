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

#[derive(Resource, Default)]
struct LootLog {
    loot: [u32; 3],
    weather: [u32; 3],
    first: [u32; 3],
    condition: bool,
}

fn three(mut draw: Random) -> [u32; 3] {
    [
        draw.below(u32::MAX),
        draw.below(u32::MAX),
        draw.below(u32::MAX),
    ]
}

fn loot(draw: Random, mut log: ResMut<'_, LootLog>) {
    log.loot = three(draw);
}

fn weather(draw: Random, mut log: ResMut<'_, LootLog>) {
    log.weather = three(draw);
}

fn omen(mut draw: Random) -> u32 {
    draw.below(u32::MAX)
}

#[test]
fn loot_is_build_stable_and_has_one_stream_through_every_route() -> Result {
    const FIRST_LOOT: [u32; 3] = [3_485_800_425, 1_699_994_307, 565_622_368];

    fn alone(scene: &mut Scene) {
        scene.rules(loot);
    }
    fn tuple(scene: &mut Scene) {
        scene.rules((weather, loot));
    }
    fn phase(scene: &mut Scene) {
        scene.phases(Stage::A);
        scene.phase(Stage::A).rules(loot);
    }
    fn cadence(scene: &mut Scene) {
        scene.rules(loot.every(seconds(1)));
    }
    fn condition(scene: &mut Scene) {
        scene.rules(loot.when(|| true));
    }
    fn expiry(scene: &mut Scene) {
        scene.phases(Stage::A);
        scene.phase(Stage::A).lasts(seconds(1)).then_run(loot);
    }
    fn request(mut scenes: Scenes<'_>) {
        assert!(scenes.go_to(Over(0)));
    }
    fn arriving(scene: &mut Scene<Over>) {
        scene.on_enter(loot);
    }

    let setup = || {
        Game::new("random routes")
            .tick_rate(1)
            .resource(LootLog::default())
    };
    let descriptions: [fn(&mut Scene); 6] = [alone, tuple, phase, cadence, condition, expiry];
    for describe in descriptions {
        let mut game = setup().scene(describe).test_setup().seed(17).start()?;
        game.step()?;
        assert_eq!(game.resource::<LootLog>()?.loot, FIRST_LOOT);
    }
    let mut landing = setup()
        .scene(|scene: &mut Scene| scene.rules(request))
        .scene(arriving)
        .test_setup()
        .seed(17)
        .start()?;
    landing.step()?;
    assert_eq!(landing.resource::<LootLog>()?.loot, FIRST_LOOT);

    let mut manual = setup()
        .scene(|_: &mut Scene| {})
        .test_setup()
        .seed(17)
        .start()?;
    manual.step()?;
    manual.run(loot)?;
    assert_eq!(manual.resource::<LootLog>()?.loot, FIRST_LOOT);
    let digest = manual.digest();
    manual.run(loot)?;
    assert_eq!(manual.resource::<LootLog>()?.loot, FIRST_LOOT);
    assert_eq!(manual.digest(), digest);
    Ok(())
}

#[test]
fn repeated_runs_count_inside_the_tick_and_replay_at_another_tick_and_seed() -> Result {
    fn capture(mut log: ResMut<'_, LootLog>) {
        log.first = log.loot;
    }
    let setup = |seed| {
        Game::new("random count")
            .resource(LootLog::default())
            .scene(|scene: &mut Scene| scene.rules((loot, capture, loot, weather)))
            .test_setup()
            .seed(seed)
            .start()
    };

    let mut first = setup(17)?;
    let mut rebuilt = setup(17)?;
    let mut other_seed = setup(18)?;
    first.step()?;
    rebuilt.step()?;
    other_seed.step()?;
    let expected = first.resource::<LootLog>()?;
    assert_ne!(expected.first, expected.loot);
    assert_ne!(expected.first, expected.weather);
    assert_ne!(expected.loot, expected.weather);
    assert_eq!(first.digest(), rebuilt.digest());
    assert_ne!(expected.first, other_seed.resource::<LootLog>()?.first);
    assert_ne!(expected.weather, other_seed.resource::<LootLog>()?.weather);
    first.step()?;
    let replay = first.digest();
    assert_ne!(expected.first, first.resource::<LootLog>()?.first);
    assert_ne!(expected.weather, first.resource::<LootLog>()?.weather);
    first.rewind(1)?;
    assert_eq!(first.resource::<LootLog>()?, expected);
    first.step()?;
    assert_eq!(first.digest(), replay);

    // The manual read is the first run of loot even after two scheduled runs.
    first.run(loot)?;
    let after_read = first.resource::<LootLog>()?;
    assert_eq!(after_read.loot, after_read.first);
    first.step()?;
    rebuilt.step_ticks(2)?;
    assert_eq!(first.digest(), rebuilt.digest());
    Ok(())
}

#[test]
fn a_read_on_a_discarded_tick_leaves_the_next_draw_unchanged() -> Result {
    let setup = || {
        Game::new("manual streams")
            .resource(LootLog::default())
            .scene(|_: &mut Scene| {})
            .test_setup()
            .seed(17)
            .start()
    };
    let mut first = setup()?;
    let mut rewound = setup()?;
    let start = rewound.digest();
    first.run(loot)?;
    rewound.step()?;
    rewound.run(omen)?;
    rewound.rewind(1)?;
    assert_eq!(rewound.digest(), start);
    rewound.run(loot)?;
    assert_eq!(rewound.resource::<LootLog>()?, first.resource::<LootLog>()?);
    assert_eq!(rewound.digest(), first.digest());
    Ok(())
}

#[test]
fn random_conditions_and_lengths_share_their_harness_draws() -> Result {
    fn condition(mut random: Random) -> bool {
        random.below(u32::MAX).is_multiple_of(2)
    }
    fn duration(mut random: Random) -> Seconds {
        seconds(i32::try_from(random.below(10) + 1).unwrap())
    }
    fn mark(mut log: ResMut<'_, LootLog>) {
        log.condition = true;
    }
    let mut game = Game::new("read-only random")
        .tick_rate(1)
        .resource(LootLog::default())
        .scene(|scene: &mut Scene| {
            scene.phases(Stage::A);
            scene
                .phase(Stage::A)
                .lasts(duration)
                .rules(mark.when(condition));
        })
        .test_setup()
        .seed(17)
        .start()?;
    let clock = game.run(|phase: Phase<'_, Stage>| phase.clock())?;
    assert_eq!(clock.left(), Some(game.run(duration)?));
    game.step()?;
    assert_eq!(game.resource::<LootLog>()?.condition, game.run(condition)?);
    Ok(())
}

#[test]
fn random_refuses_closures_and_pointers_before_they_run() -> Result {
    #[derive(Params)]
    struct Bundle<'t> {
        _score: Res<'t, Counter>,
        random: Random,
    }
    fn bundled(mut bundle: Bundle<'_>) {
        bundle.random.below(10);
    }
    fn pointer(_: Random) {
        panic!("erased pointer must never run");
    }
    let table: [fn(Random); 1] = [pointer];
    let closure = |_: Random| -> Result { panic!("random closure must never run") };

    for startup in [
        Game::new("closure")
            .scene(move |s: &mut Scene| s.rules(closure))
            .test(),
        Game::new("pointer")
            .scene(move |s: &mut Scene| s.rules(table[0]))
            .test(),
    ] {
        let Err(error) = startup else {
            panic!("startup admitted an unnamed Random callable");
        };
        assert_eq!(error.code(), "T0204");
        assert!(error.to_string().contains("Write a named function"));
    }

    let mut game = Game::new("manual streams")
        .resource(Counter::default())
        .scene(|_: &mut Scene| {})
        .test()?;
    let start = game.digest();
    let callback: fn(Bundle<'_>) = bundled;
    for error in [
        game.run(closure).unwrap_err(),
        game.run(table[0]).unwrap_err(),
        game.run(callback).unwrap_err(),
    ] {
        assert_eq!(error.code(), "T0204");
        assert!(error.to_string().contains("Write a named function"));
    }
    assert_eq!(game.digest(), start);

    let ordinary = |mut value: ResMut<'_, Counter>| value.value += 1;
    let plain: fn(ResMut<'_, Counter>) = |mut value| value.value += 2;
    let mut control = Game::new("plain callables")
        .resource(Counter::default())
        .scene(move |s: &mut Scene| s.rules((ordinary, plain)))
        .test()?;
    control.step()?;
    assert_eq!(control.resource::<Counter>()?.value, 3);
    control.run(ordinary)?;
    control.run(plain)?;
    assert_eq!(control.resource::<Counter>()?.value, 6);
    Ok(())
}

#[test]
fn same_qualified_random_names_are_refused_at_startup_and_in_the_harness() -> Result {
    let first = {
        fn same(mut random: Random) {
            random.below(10);
        }
        same
    };
    let second = {
        fn same(mut random: Random) {
            random.below(11);
        }
        same
    };
    let startup = Game::new("two scenes")
        .scene(move |s: &mut Scene| s.rules(first))
        .scene(move |s: &mut Scene<Over>| s.rules(second))
        .test();
    let Err(error) = startup else {
        panic!("allowed the same Random path twice");
    };
    assert_eq!(error.code(), "T0206");
    assert!(error.to_string().contains("::same"));
    assert!(error.to_string().contains("another name"));

    let mut scheduled = Game::new("one scheduled")
        .scene(move |s: &mut Scene| s.rules(first))
        .test()?;
    let mut manual = Game::new("two manual").scene(|_: &mut Scene| {}).test()?;
    manual.run(first)?;
    for game in [&mut scheduled, &mut manual] {
        let digest = game.digest();
        assert_eq!(game.run(second).unwrap_err().code(), "T0206");
        assert_eq!(game.digest(), digest);
    }

    let plain_first = {
        fn same() {}
        same
    };
    let plain_second = {
        fn same() {}
        same
    };
    let mut control = Game::new("same plain names")
        .scene(move |s: &mut Scene| s.rules((plain_first, plain_second)))
        .test()?;
    control.step()?;
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
