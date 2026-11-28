use thrum::prelude::*;

#[test]
fn manual_rule_commits_the_exact_resource_encoding() -> Result {
    #[derive(Resource)]
    enum Velocity {
        Horizontal(Num),
    }
    let mut game = Game::new("velocity")
        .resource(Velocity::Horizontal(Num::ZERO))
        .scene(|_: &mut Scene| {})
        .test()?;
    let starting = game.digest();
    game.run(|mut velocity: ResMut<'_, Velocity>| {
        let Velocity::Horizontal(value) = &mut *velocity;
        *value = -*value;
    })?;
    let Velocity::Horizontal(value) = game.resource::<Velocity>()?;
    assert_eq!((Num::ONE / value).to_i32().signum(), -1);
    assert_ne!(game.digest(), starting);
    Ok(())
}

#[derive(Component)]
struct Health(i32);
#[derive(Component)]
struct Hero;
#[derive(Component)]
struct Marker;
#[derive(Component)]
struct Explicit;
#[derive(Event)]
struct Hit(i32);
#[derive(Resource, Default)]
struct Ledger {
    order: u64,
    spawned: Option<Entity>,
    original: Option<Entity>,
}

#[derive(Params)]
struct Combat<'t> {
    heroes: Query<'t, &'t mut Health, With<Hero>>,
    ledger: ResMut<'t, Ledger>,
    hits: Emit<'t, Hit>,
}

#[derive(Phase)]
enum Stage {
    Active,
    Later,
}

fn turn(mut rules: Combat<'_>) {
    rules.ledger.order = rules.ledger.order * 10 + 1;
    for hp in &mut rules.heroes {
        hp.0 += 1;
    }
    rules.hits.send(Hit(4));
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn spawn(
    mut entities: Entities<'_>,
    mut ledger: ResMut<'_, Ledger>,
    query: Query<'_, (Entity, &Health)>,
    hits: Events<'_, Hit>,
) {
    ledger.order = ledger.order * 10 + 2;
    if ledger.original.is_none() {
        ledger.original = query.iter().next().map(|(entity, _)| entity);
    }
    if ledger.spawned.is_none() {
        let original = ledger.original.expect("test fixture provides this value");
        assert_eq!(
            query
                .get(original)
                .expect("test fixture provides this value")
                .1
                .0,
            6
        );
        let spawned = entities.spawn((Health(2), Hero, Explicit));
        assert!(query.get(spawned).is_none());
        entities.insert(original, Marker);
        ledger.spawned = Some(spawned);
        assert_eq!(hits.read().map(|event| event.0).collect::<Vec<_>>(), [4]);
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn pair(
    mut query: Query<'_, (Entity, &mut Health), With<Hero>>,
    mut rules: ResMut<'_, Ledger>,
    markers: Query<'_, Entity, With<Marker>>,
) {
    rules.order = rules.order * 10 + 3;
    let a = rules.original.expect("test fixture provides this value");
    let b = rules.spawned.expect("test fixture provides this value");
    assert!(query.pair_mut(a, a).is_none());
    let ((_, source), (_, dest)) = query
        .pair_mut(a, b)
        .expect("test fixture provides this value");
    dest.0 = source.0;
    assert_eq!(markers.iter().collect::<Vec<_>>(), [a]);
    assert_eq!(
        query.iter().map(|(entity, _)| entity).collect::<Vec<_>>(),
        [a, b]
    );
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn retire(mut entities: Entities<'_>, query: Query<'_, &Health>, ledger: Res<'_, Ledger>) {
    let original = ledger.original.expect("test fixture provides this value");
    assert!(entities.despawn(original));
    assert!(!entities.despawn(original));
    assert!(query.get(original).is_some());
}

fn second(mut combat: Combat<'_>) {
    combat.ledger.order = combat.ledger.order * 10 + 4;
    combat.heroes.par_each(|hp| hp.0 += 2);
    combat.hits.send(Hit(8));
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn validate(
    mut entities: Entities<'_>,
    remaining: Single<'_, (Entity, &Health), Without<Marker>>,
    ledger: Res<'_, Ledger>,
    hits: Events<'_, Hit>,
) {
    assert!(!entities.despawn(ledger.original.expect("test fixture provides this value")));
    let (entity, health) = remaining.into_inner();
    assert_eq!(
        entity,
        ledger.spawned.expect("test fixture provides this value")
    );
    assert_eq!(health.0, 8);
    assert_eq!(ledger.order, 1234);
    assert_eq!(hits.read().map(|event| event.0).collect::<Vec<_>>(), [4, 8]);
    entities.remove::<Hero>(entity);
}

fn active(tick: Tick) -> bool {
    tick.number() == TickNumber::default() || tick.seconds() == seconds(1)
}

fn play(scene: &mut Scene) {
    scene.on_enter(|mut entities: Entities<'_>| {
        entities.spawn((Health(5), Hero));
    });
    scene.phases(Stage::Active);
    scene.rules((turn, spawn).when(active));
    scene
        .phase(Stage::Active)
        .rules((pair, retire).when(active));
    scene.phase(Stage::Later).rules(|| -> () {
        panic!("inactive phase");
    });
    scene.rules((second, validate).when(active));
}

#[test]
fn edits_are_visible_at_the_next_rule_in_written_order() -> Result {
    let mut game = Game::new("rules")
        .tick_rate(1)
        .resource(Ledger::default())
        .component::<Explicit>()
        .scene(play)
        .test()?;
    game.step()?;
    let ledger = game.resource::<Ledger>()?;
    let entity = ledger.spawned.expect("test fixture provides this value");
    assert_eq!(game.count::<With<Hero>>(), 0);
    assert_eq!(game.get::<Health>(entity)?.0, 8);
    assert_eq!(game.get::<Explicit>(entity)?, Explicit);
    assert_eq!(game.events::<Hit>(), [Hit(4), Hit(8)]);
    assert_eq!(game.events::<Hit>().len(), 0);
    game.step()?;
    assert_eq!(game.events::<Hit>().len(), 0);
    game.run(|mut query: Query<'_, &mut Health>| query.par_each(|hp| hp.0 += 1))?;
    assert_eq!(game.get::<Health>(entity)?.0, 9);
    Ok(())
}

#[derive(Resource, Default)]
struct Counter(u32);

fn every_two(mut counter: ResMut<'_, Counter>) {
    counter.0 += 1;
}

#[test]
fn cadence_starts_after_the_first_interval() -> Result {
    let mut game = Game::new("cadence")
        .tick_rate(1)
        .resource(Counter::default())
        .scene(|scene: &mut Scene| scene.rules(every_two.every(seconds(2))))
        .test()?;
    for expected in [0, 1, 1, 2] {
        game.step()?;
        assert_eq!(game.resource::<Counter>()?.0, expected);
    }
    Ok(())
}

#[test]
fn parallel_query_matches_the_serial_loop() -> Result {
    fn scene(parallel: bool) -> Result<thrum::TestGame> {
        Game::new("query work")
            .scene(move |scene: &mut Scene| {
                scene.on_enter(|mut entities: Entities<'_>| {
                    for health in 0..256 {
                        entities.spawn((Health(health),));
                    }
                });
                scene.rules(move |mut query: Query<'_, &mut Health>| {
                    if parallel {
                        query.par_each(|health| health.0 += 3);
                    } else {
                        for health in &mut query {
                            health.0 += 3;
                        }
                    }
                });
            })
            .test()
    }
    let mut serial = scene(false)?;
    let mut parallel = scene(true)?;
    serial.step_ticks(3)?;
    parallel.step_ticks(3)?;
    assert_eq!(parallel.digest(), serial.digest());
    Ok(())
}

#[test]
fn a_parallel_panic_restores_every_chunk_and_can_retry() -> Result {
    use std::sync::atomic::{AtomicBool, Ordering};
    static REFUSE: AtomicBool = AtomicBool::new(true);

    fn increment(mut rows: Query<'_, &mut Health>) {
        rows.par_each(|health| {
            let before = health.0;
            health.0 += 1;
            assert!(
                before != 900 || !REFUSE.load(Ordering::Relaxed),
                "parallel refusal"
            );
        });
    }

    let setup = || {
        Game::new("parallel rollback")
            .max_entities(1024)
            .scene(|scene: &mut Scene| {
                scene.on_enter(|mut entities: Entities<'_>| {
                    for health in 0..1024 {
                        entities.spawn((Health(health),));
                    }
                });
                scene.rules(increment);
            })
            .test()
    };
    let mut game = setup()?;
    let before = game.digest();
    let refused = game.step().unwrap_err();
    assert!(refused.to_string().contains("parallel refusal"));
    assert_eq!(game.digest(), before);
    REFUSE.store(false, Ordering::Relaxed);
    game.step()?;
    let mut control = setup()?;
    control.step()?;
    assert_eq!(game.digest(), control.digest());
    game.rewind(1)?;
    assert_eq!(game.digest(), before);
    game.step()?;
    assert_eq!(game.digest(), control.digest());
    Ok(())
}

#[derive(Resource)]
struct Gate(bool);

#[test]
fn a_group_reads_its_condition_once_at_its_written_position() -> Result {
    let mut game = Game::new("group condition")
        .resource(Gate(true))
        .resource(Counter::default())
        .scene(|scene: &mut Scene| {
            scene.rules(
                (|mut gate: ResMut<'_, Gate>| gate.0 = false, every_two)
                    .when(|gate: Res<'_, Gate>| gate.0),
            );
        })
        .test()?;
    game.step_ticks(2)?;
    assert_eq!(game.resource::<Counter>()?.0, 1);
    game.rewind(2)?;
    game.step_ticks(2)?;
    assert_eq!(game.resource::<Counter>()?.0, 1);
    Ok(())
}

#[test]
fn four_entry_read_only_query_and_entity_single_use_the_reference_spelling() -> Result {
    #[derive(Component)]
    struct A;
    #[derive(Component)]
    struct B;
    #[derive(Component)]
    struct C;
    #[derive(Params)]
    struct ReadAll<'t> {
        all: Query<'t, (Entity, &'t A, &'t B, &'t C)>,
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK hands Params to the condition"
    )]
    fn one(params: ReadAll<'_>) -> bool {
        params.all.count() == 1
    }
    #[expect(
        clippy::needless_pass_by_value,
        reason = "exercise the reference Single parameter"
    )]
    fn retire(mut entities: Entities<'_>, entity: Single<'_, Entity, With<A>>) {
        assert!(entities.despawn(*entity));
    }

    let mut game = Game::new("typed rule access")
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((A, B, C));
            });
            scene.rules(retire.when(one));
        })
        .test()?;
    game.step()?;
    assert_eq!(game.count::<()>(), 0);
    Ok(())
}
