use thrum::prelude::*;

#[test]
fn a_held_phase_request_belongs_to_its_scene_entry() -> Result {
    #[derive(Phase)]
    enum OldStage {
        Start,
        Later,
    }
    #[derive(Phase)]
    enum NewStage {
        Start,
        Other,
        Later,
    }
    #[derive(Scene, Default)]
    struct Old;
    #[derive(Scene)]
    struct New(bool);

    let setup = || {
        Game::new("held phase")
            .scene(|scene: &mut Scene<Old>| {
                scene.phases(OldStage::Start);
                scene.phase(OldStage::Later);
                scene.on_enter(|mut phase: PhaseMut<'_, OldStage>| {
                    assert!(phase.go_to(OldStage::Later));
                });
            })
            .scene(|scene: &mut Scene<New>| {
                scene.phases(NewStage::Start);
                scene.phase(NewStage::Other);
                scene.phase(NewStage::Later);
                scene.on_enter(|new: Res<'_, New>, mut phase: PhaseMut<'_, NewStage>| {
                    if new.0 {
                        assert!(phase.go_to(NewStage::Later));
                    }
                });
            })
            .test()
    };
    for (request, expected) in [(false, NewStage::Start), (true, NewStage::Later)] {
        let mut game = setup()?;
        game.go_to(New(request))?;
        assert_eq!(game.phase::<NewStage>()?, NewStage::Start);
        game.step()?;
        let digest = game.digest();
        assert_eq!(game.phase::<NewStage>()?, expected);
        game.rewind(1)?;
        game.step()?;
        assert_eq!(game.digest(), digest);
    }
    Ok(())
}

#[derive(Resource, Default)]
struct Trace(u64);
#[derive(Resource)]
struct Local(u32);
#[derive(Component)]
struct Mark;
#[derive(Scene, Default)]
struct Play(i32);
#[derive(Scene)]
struct Finished(i32);
#[derive(Phase)]
enum Stage {
    A,
    B,
    C,
}

fn play(scene: &mut Scene<Play>) {
    scene.resource(Local(4));
    scene.phases(Stage::A);
    scene.on_enter(|mut trace: ResMut<'_, Trace>, mut entities: Entities<'_>| {
        trace.0 = trace.0 * 10 + 1;
        entities.spawn((Mark,));
    });
    scene
        .phase(Stage::A)
        .on_enter(|mut trace: ResMut<'_, Trace>| trace.0 = trace.0 * 10 + 2)
        .rules(
            |mut scenes: Scenes<'_>,
             mut trace: ResMut<'_, Trace>,
             mut local: ResMut<'_, Local>,
             tick: Tick| {
                assert_eq!(local.0, 4);
                local.0 = 9;
                trace.0 = trace.0 * 10 + 3;
                if tick.seconds() == seconds(1) {
                    assert!(scenes.go_to(Finished(42)));
                }
            },
        )
        .on_exit(|mut trace: ResMut<'_, Trace>| trace.0 = trace.0 * 10 + 5);
    scene.on_exit(|mut trace: ResMut<'_, Trace>, query: Query<'_, &Mark>| {
        assert_eq!(query.count(), 1);
        trace.0 = trace.0 * 10 + 6;
    });
}

fn additional(scene: &mut Scene<Play>) {
    scene.rules(|mut trace: ResMut<'_, Trace>, phase: Phase<'_, Stage>| {
        assert!(phase.is(Stage::A));
        trace.0 = trace.0 * 10 + 4;
    });
}

fn finished(scene: &mut Scene<Finished>) {
    scene.resource(Local(7));
    scene.on_enter(
        |mut trace: ResMut<'_, Trace>,
         local: Res<'_, Local>,
         scene: Res<'_, Finished>,
         entities: Query<'_, Entity>| {
            assert_eq!(local.0, 7);
            assert_eq!(scene.0, 42);
            assert!(entities.is_empty());
            trace.0 = trace.0 * 10 + 7;
        },
    );
    scene.on_enter(|mut trace: ResMut<'_, Trace>| trace.0 = trace.0 * 10 + 8);
    scene.phases(Stage::B);
    scene
        .phase(Stage::B)
        .on_enter(|mut trace: ResMut<'_, Trace>| trace.0 = trace.0 * 10 + 9);
    scene.rules(|mut scenes: Scenes<'_>, mut local: ResMut<'_, Local>| {
        assert_eq!(local.0, 7);
        local.0 = 8;
        assert!(scenes.restart());
    });
}

fn setup() -> Game {
    Game::new("scene lifecycle")
        .resource(Trace::default())
        .tick_rate(1)
        .scene((play, additional))
        .scene(finished)
}

#[test]
fn scene_transition_runs_hooks_clears_entities_and_restores_defaults() -> Result {
    let mut game = setup().test()?;
    assert_eq!(game.scene::<Play>()?, Play(0));
    assert_eq!(game.resource::<Trace>()?.0, 12);
    game.step()?;
    assert_eq!(game.scene::<Finished>()?, Finished(42));
    assert_eq!(game.resource::<Trace>()?.0, 123_456_789);
    assert_eq!(game.count::<()>(), 0);
    let digest = game.digest();
    game.rewind(1)?;
    game.step()?;
    assert_eq!(game.digest(), digest);
    game.step()?;
    assert_eq!(game.scene::<Finished>()?, Finished(42));
    assert_eq!(game.resource::<Local>()?, Local(7));
    assert_eq!(game.phase::<Stage>()?, Stage::B);
    let game = setup().test_in(Finished(42))?;
    assert_eq!(game.resource::<Trace>()?.0, 789);
    Ok(())
}

#[derive(Resource, Default)]
struct Order {
    choice: u32,
    hook: u32,
}

fn branches(scene: &mut Scene<Play>) {
    scene.resource(Order::default());
    scene.phases(Stage::A);
    scene.phase(Stage::A).lasts(seconds(1)).then(Stage::C);
    scene
        .phase(Stage::B)
        .on_enter(|mut order: ResMut<'_, Order>| order.hook += 1);
    scene.phase(Stage::C);
    scene.add(|part| part.rules(decide));
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn decide(
    mut phase: PhaseMut<'_, Stage>,
    mut scenes: Scenes<'_>,
    order: Res<'_, Order>,
    tick: Tick,
) {
    if tick.seconds() == seconds(1) {
        match order.choice {
            0 => {
                assert!(scenes.go_to(Finished(42)));
                assert!(!scenes.go_to(Finished(7)));
            }
            1 => {
                assert!(phase.go_to(Stage::B));
                assert!(!phase.go_to(Stage::C));
            }
            2 => {
                assert!(phase.go_to(Stage::B));
                assert!(!scenes.go_to(Finished(7)));
            }
            3 => {
                assert!(scenes.go_to(Finished(42)));
                assert!(!phase.go_to(Stage::B));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn first_request_wins_across_both_kinds_before_the_deadline() -> Result {
    for choice in 0..4 {
        let mut game = Game::new("first request")
            .tick_rate(1)
            .scene(branches)
            .scene(|_: &mut Scene<Finished>| {})
            .test()?;
        game.set_resource(Order { choice, hook: 0 })?;
        game.step()?;
        if choice == 1 || choice == 2 {
            assert_eq!(game.phase::<Stage>()?, Stage::B);
            assert_eq!(game.resource::<Order>()?.hook, 1);
            game.go_to_phase(Stage::B)?;
            assert_eq!(game.resource::<Order>()?.hook, 2);
        } else {
            assert_eq!(game.scene::<Finished>()?, Finished(42));
        }
    }
    Ok(())
}

#[derive(Resource, Default)]
struct ClockLog {
    clock: Option<PhaseClock>,
    enters: u32,
    leaves: u32,
    expired: u32,
    length: u32,
}

fn timed(scene: &mut Scene) {
    scene.phases(Stage::A);
    scene
        .phase(Stage::A)
        .on_enter(
            |mut log: ResMut<'_, ClockLog>, mut phase: PhaseMut<'_, Stage>| {
                log.enters += 1;
                assert!(!phase.resume(Stage::A, PhaseClock::at(seconds(1))));
            },
        )
        .rules(
            |mut log: ResMut<'_, ClockLog>, mut phase: PhaseMut<'_, Stage>| {
                assert!(!phase.resume(Stage::A, PhaseClock::at(seconds(1))));
                log.clock = Some(phase.clock());
            },
        )
        .on_exit(|mut log: ResMut<'_, ClockLog>| log.leaves += 1)
        .lasts(|log: Res<'_, ClockLog>| {
            seconds(i32::try_from(log.length).expect("test fixture provides this value"))
        })
        .then_run(|mut log: ResMut<'_, ClockLog>, phase: Phase<'_, Stage>| {
            assert_eq!(phase.time(), seconds(10));
            log.expired += 1;
        });
    scene.on_enter(|mut phase: PhaseMut<'_, Stage>, log: Res<'_, ClockLog>| {
        if let Some(clock) = log.clock {
            assert!(phase.resume(Stage::A, clock));
        }
    });
}

#[test]
fn resume_keeps_fixed_length_and_does_not_call_enter_or_expiry_again() -> Result {
    let mut game = Game::new("phase clock")
        .tick_rate(1)
        .resource(ClockLog {
            length: 10,
            ..ClockLog::default()
        })
        .scene(timed)
        .test()?;
    game.step_ticks(4)?;
    let mut log = game.resource::<ClockLog>()?;
    assert_eq!(
        log.clock.expect("test fixture provides this value").left(),
        Some(seconds(6))
    );
    log.length = 20;
    game.set_resource(log)?;
    game.go_to(())?;
    assert_eq!(game.resource::<ClockLog>()?.enters, 1);
    assert_eq!(game.resource::<ClockLog>()?.leaves, 1);
    game.step_ticks(5)?;
    assert_eq!(game.resource::<ClockLog>()?.expired, 0);
    game.step()?;
    assert_eq!(game.resource::<ClockLog>()?.expired, 1);
    game.step()?;
    assert_eq!(
        game.resource::<ClockLog>()?
            .clock
            .expect("test fixture provides this value")
            .left(),
        None
    );
    game.go_to(())?;
    game.step_ticks(10)?;
    assert_eq!(game.resource::<ClockLog>()?.expired, 1);
    assert_eq!(game.resource::<ClockLog>()?.enters, 1);
    assert_eq!(game.resource::<ClockLog>()?.leaves, 2);
    let mut at_deadline = Game::new("due phase clock")
        .tick_rate(1)
        .resource(ClockLog {
            length: 1,
            ..ClockLog::default()
        })
        .scene(|scene: &mut Scene| {
            scene.phases(Stage::A);
            scene
                .phase(Stage::A)
                .lasts(seconds(1))
                .rules(|phase: Phase<'_, Stage>, mut log: ResMut<'_, ClockLog>| {
                    log.clock = Some(phase.clock());
                })
                .then_run(|mut log: ResMut<'_, ClockLog>| log.expired += 1);
            scene.on_enter(|mut phase: PhaseMut<'_, Stage>, log: Res<'_, ClockLog>| {
                if let Some(clock) = log.clock {
                    assert!(phase.resume(Stage::A, clock));
                }
            });
        })
        .test()?;
    at_deadline.step()?;
    let at_expiry = at_deadline.resource::<ClockLog>()?;
    assert_eq!(
        at_expiry.clock.and_then(PhaseClock::left),
        Some(Seconds::ZERO)
    );
    assert_eq!(at_expiry.expired, 1);
    at_deadline.go_to(())?;
    at_deadline.step_ticks(2)?;
    assert_eq!(at_deadline.resource::<ClockLog>()?.expired, 2);

    Ok(())
}

#[test]
fn resumed_phase_keeps_elapsed_precision_cadence_and_deadline() -> Result {
    #[derive(Resource, Default)]
    struct Progress {
        saved: Option<PhaseClock>,
        threshold: bool,
        cadence: u32,
        expired: u32,
    }

    fn play(scene: &mut Scene) {
        scene.phases(Stage::A);
        scene.on_enter(
            |mut phase: PhaseMut<'_, Stage>, progress: Res<'_, Progress>| {
                if let Some(clock) = progress.saved {
                    assert!(phase.resume(Stage::A, clock));
                }
            },
        );
        scene
            .phase(Stage::A)
            .rules((
                |phase: Phase<'_, Stage>, tick: Tick, mut progress: ResMut<'_, Progress>| {
                    assert_eq!(tick.in_phase(), phase.time());
                    progress.threshold = phase.time() >= seconds(1);
                    progress.saved = Some(phase.clock());
                },
                (|mut progress: ResMut<'_, Progress>| progress.cadence += 1).every(seconds(1)),
            ))
            .lasts(seconds(2))
            .then_run(|mut progress: ResMut<'_, Progress>| progress.expired += 1);
    }

    for (rate, restore_at) in [(5, 4), (60, 1)] {
        let setup = || {
            Game::new("resume time and cadence")
                .tick_rate(rate)
                .resource(Progress::default())
                .scene(play)
                .test()
        };
        let mut uninterrupted = setup()?;
        let mut resumed = setup()?;
        uninterrupted.step_ticks(restore_at)?;
        resumed.step_ticks(restore_at)?;
        resumed.go_to(())?;

        let interval = u64::from(rate);
        for tick in (restore_at + 1)..=(2 * interval) {
            uninterrupted.step()?;
            resumed.step()?;

            let progress = resumed.resource::<Progress>()?;
            assert_eq!(progress, uninterrupted.resource::<Progress>()?);
            assert_eq!(progress.threshold, tick >= interval);
            assert_eq!(progress.cadence, u32::try_from(tick / interval).unwrap());
            assert_eq!(progress.expired, u32::from(tick == 2 * interval));
        }
        let digest = resumed.digest();
        resumed.rewind(1)?;
        resumed.step()?;
        assert_eq!(resumed.digest(), digest);
    }
    Ok(())
}

#[derive(Resource, Default)]
struct Watches {
    timer: Timer,
    gun: Cooldown,
    other: Cooldown,
}

#[test]
fn timers_and_cooldowns_use_simulated_time() -> Result {
    let mut game = Game::new("time")
        .tick_rate(1)
        .resource(Watches::default())
        .scene(|scene: &mut Scene| {
            scene.on_enter(|tick: Tick, mut watches: ResMut<'_, Watches>| {
                assert!(!watches.timer.running(tick));
                assert!(!watches.timer.finished(tick));
                watches.timer.start(tick, seconds(2));
                assert!(watches.timer.running(tick));
                assert!(watches.gun.take(tick, seconds(2)));
                watches.other = Cooldown::ready_after(tick, seconds(2));
            });
            scene.rules(|tick: Tick, mut watches: ResMut<'_, Watches>| {
                if tick.seconds() == seconds(1) {
                    assert_eq!(watches.timer.left(tick), seconds(1));
                    assert_eq!(watches.timer.fraction(tick), Num::ONE / Num::from(2));
                    assert_eq!(watches.gun.left(tick), seconds(1));
                    assert_eq!(watches.gun.fraction(tick), Num::ONE / Num::from(2));
                    assert!(!watches.gun.take(tick, seconds(4)));
                    assert!(!watches.other.ready(tick));
                }
                if tick.seconds() == seconds(2) {
                    assert!(!watches.timer.running(tick));
                    assert!(watches.timer.finished(tick));
                    assert_eq!(watches.timer.fraction(tick), Num::ONE);
                    assert!(watches.other.ready(tick));
                    assert!(watches.gun.take(tick, seconds(4)));
                    assert_eq!(watches.gun.left(tick), seconds(4));
                    watches.timer.clear();
                    assert!(!watches.timer.finished(tick));
                    assert!(!watches.timer.running(tick));
                }
                if tick.seconds() == seconds(6) {
                    assert_eq!(watches.gun.fraction(tick), Num::ONE);
                    assert!(watches.gun.take(tick, seconds(1)));
                }
            });
        })
        .test()?;
    game.step_ticks(6)?;
    let digest = game.digest();
    game.rewind(3)?;
    game.step_ticks(3)?;
    assert_eq!(game.digest(), digest);
    Ok(())
}

#[derive(Resource, Default)]
struct HeldLog {
    choice: u32,
    count: u32,
}
#[derive(Scene, Default)]
struct Holding;
#[derive(Scene)]
struct HoldingNext;
#[derive(Scene)]
struct Held;
#[derive(Scene)]
struct Later;

fn held_start(scene: &mut Scene<Holding>) {
    scene.phases(Stage::A);
    scene
        .phase(Stage::A)
        .on_enter(|log: Res<'_, HeldLog>, mut scenes: Scenes<'_>| {
            if log.choice == 0 {
                assert!(scenes.go_to(Held));
            }
        })
        .on_exit(
            |log: Res<'_, HeldLog>, mut scenes: Scenes<'_>, mut phase: PhaseMut<'_, Stage>| {
                if log.choice == 2 {
                    assert!(scenes.go_to(Held));
                }
                if log.choice == 3 {
                    assert!(phase.go_to(Stage::C));
                }
                if log.choice >= 5 {
                    assert!(!phase.go_to(Stage::C));
                }
            },
        );
    scene.phase(Stage::B);
    scene.phase(Stage::C);
    scene.on_exit(
        |log: Res<'_, HeldLog>, mut scenes: Scenes<'_>, mut phase: PhaseMut<'_, Stage>| {
            if log.choice == 1 {
                assert!(scenes.go_to(Held));
            }
            if log.choice >= 5 {
                assert!(!phase.go_to(Stage::C));
            }
        },
    );
    scene.rules(
        |mut scenes: Scenes<'_>,
         mut phase: PhaseMut<'_, Stage>,
         mut log: ResMut<'_, HeldLog>,
         tick: Tick| {
            log.count += 1;
            if tick.seconds() == seconds(1) {
                match log.choice {
                    0 => assert!(!scenes.go_to(Later)),
                    1 | 4 | 5 => assert!(scenes.go_to(HoldingNext)),
                    2 | 3 => assert!(phase.go_to(Stage::B)),
                    6 => assert!(scenes.restart()),
                    _ => unreachable!(),
                }
            } else if tick.seconds() == seconds(2) {
                assert_eq!(scenes.go_to(Later), log.choice == 6);
            }
        },
    );
}

fn held_next(scene: &mut Scene<HoldingNext>) {
    scene.rules(|mut scenes: Scenes<'_>, mut log: ResMut<'_, HeldLog>| {
        log.count += 1;
        assert_eq!(scenes.go_to(Later), log.choice == 5);
    });
}

fn held_next_with_phases(scene: &mut Scene<HoldingNext>) {
    held_next(scene);
    scene.phases(Stage::A);
    scene.phase(Stage::A);
    scene.phase(Stage::B);
    scene.on_enter(|mut phase: PhaseMut<'_, Stage>| {
        assert!(phase.go_to(Stage::B));
    });
}

#[test]
fn landing_requests_take_precedence_on_the_next_tick_and_replay() -> Result {
    for choice in 0..=6 {
        let next = if choice == 4 {
            held_next_with_phases
        } else {
            held_next
        };
        let mut game = Game::new("requests from landing")
            .tick_rate(1)
            .resource(HeldLog { choice, count: 0 })
            .scene(held_start)
            .scene(next)
            .scene(|_: &mut Scene<Held>| {})
            .scene(|_: &mut Scene<Later>| {})
            .test()?;
        let initial = game.digest();
        game.run(|| ())?;
        assert_eq!(game.digest(), initial);
        game.step()?;
        if choice == 0 {
            assert!(game.in_scene::<Held>());
            assert_eq!(game.resource::<HeldLog>()?.count, 1);
            continue;
        }
        let before = game.digest();
        game.step()?;
        let after = game.digest();
        assert_eq!(game.resource::<HeldLog>()?.count, 2);
        match choice {
            1 | 2 => assert!(game.in_scene::<Held>()),
            3 => assert_eq!(game.phase::<Stage>()?, Stage::C),
            4 => assert_eq!(game.phase::<Stage>()?, Stage::B),
            5 | 6 => assert!(game.in_scene::<Later>()),
            _ => unreachable!(),
        }
        game.rewind(1)?;
        assert_eq!(game.digest(), before);
        game.step()?;
        assert_eq!(game.digest(), after);
    }
    Ok(())
}

#[test]
fn then_run_requests_the_next_scene_after_the_final_rule() -> Result {
    let mut game = Game::new("then run")
        .tick_rate(1)
        .resource(Trace::default())
        .scene(|scene: &mut Scene| {
            scene.phases(Stage::A);
            scene
                .phase(Stage::A)
                .rules(|mut trace: ResMut<'_, Trace>| trace.0 += 1)
                .lasts(seconds(2))
                .then_run(|trace: Res<'_, Trace>, mut scenes: Scenes<'_>| {
                    assert_eq!(trace.0, 2);
                    assert!(scenes.go_to(Finished(2)));
                });
            scene.rules(|mut trace: ResMut<'_, Trace>, tick: Tick| {
                if tick.seconds() == seconds(2) {
                    trace.0 = 2;
                }
            });
        })
        .scene(|_: &mut Scene<Finished>| {})
        .test()?;
    game.step()?;
    assert!(game.in_scene::<()>());
    game.step()?;
    assert_eq!(game.scene::<Finished>()?, Finished(2));
    let digest = game.digest();
    game.rewind(1)?;
    game.step()?;
    assert_eq!(game.digest(), digest);
    Ok(())
}
