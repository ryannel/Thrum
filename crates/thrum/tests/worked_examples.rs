mod rockfall {
    use thrum::prelude::*;

    #[derive(Scene, Default)]
    struct Play;
    #[derive(Scene)]
    struct GameOver {
        dodged: u32,
    }
    #[derive(Resource, Default)]
    struct Score {
        dodged: u32,
    }
    #[derive(Component)]
    struct Rock {
        fall_speed: Num,
    }
    #[derive(Component)]
    struct Position(Vec2);

    fn drop_rocks(mut entities: Entities<'_>, mut random: Random) {
        let rock = Rock {
            fall_speed: random.between(Num::from(4), Num::from(9)),
        };
        entities.spawn((
            rock,
            Position(vec2(random.between(Num::from(-11), Num::from(11)), 10)),
        ));
    }

    fn fall(
        mut rocks: Query<'_, (Entity, &Rock, &mut Position)>,
        mut entities: Entities<'_>,
        mut score: ResMut<'_, Score>,
        tick: Tick,
    ) {
        for (rock, falling, at) in &mut rocks {
            at.0.y -= falling.fall_speed * tick.dt();
            if at.0.y < Num::from(-10) && entities.despawn(rock) {
                score.dodged += 1;
            }
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK lends Res to rules by value"
    )]
    fn finish(score: Res<'_, Score>, mut scenes: Scenes<'_>) {
        if score.dodged != 0 {
            scenes.go_to(GameOver {
                dodged: score.dodged,
            });
        }
    }

    fn play(scene: &mut Scene<Play>) {
        scene.resource(Score::default());
        scene.rules((drop_rocks.every(seconds(0.4)), fall, finish));
    }

    #[test]
    fn the_score_travels_from_play_to_game_over() -> Result {
        let mut game = Game::new("Rockfall")
            .scene(play)
            .scene(|_: &mut Scene<GameOver>| {})
            .test()?;
        game.step_until(seconds(7), |game| Ok(game.in_scene::<GameOver>()))?;
        assert!(game.scene::<GameOver>()?.dodged > 0);
        Ok(())
    }
}

mod swarm {
    use thrum::prelude::*;

    #[derive(Scene, Default)]
    struct Arena;
    #[derive(Scene)]
    struct Ended {
        won: bool,
        wave: u32,
    }
    #[derive(Phase)]
    enum Stage {
        Gathering,
        Wave,
        Intermission,
    }
    #[derive(Resource, Default)]
    struct Run {
        wave: u32,
        gems: u32,
    }
    #[derive(Component)]
    struct Hero;
    #[derive(Component)]
    struct Enemy;
    #[derive(Component)]
    struct Gem;
    #[derive(Component)]
    struct Health(i32);
    #[derive(Component)]
    struct Position(Vec2);
    #[derive(Event)]
    struct EnemyDied {
        at: Vec2,
    }

    impl Health {
        fn is_spent(&self) -> bool {
            self.0 == 0
        }
    }

    fn next_wave(mut run: ResMut<'_, Run>, mut scenes: Scenes<'_>) {
        run.wave += 1;
        if run.wave > 2 {
            scenes.go_to(Ended {
                won: true,
                wave: run.wave,
            });
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK lends Query to rules by value"
    )]
    fn enemies_die(
        enemies: Query<'_, (Entity, &Health, &Position), With<Enemy>>,
        mut entities: Entities<'_>,
        mut died: Emit<'_, EnemyDied>,
    ) {
        for (enemy, health, at) in &enemies {
            if health.is_spent() && entities.despawn(enemy) {
                entities.spawn((Gem, Position(at.0)));
                died.send(EnemyDied { at: at.0 });
            }
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK lends Query and Res to rules by value"
    )]
    fn heroes_fall(
        heroes: Query<'_, &Health, With<Hero>>,
        run: Res<'_, Run>,
        mut scenes: Scenes<'_>,
    ) {
        if !heroes.is_empty() && heroes.iter().all(Health::is_spent) {
            scenes.go_to(Ended {
                won: false,
                wave: run.wave,
            });
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK lends Query to rules by value"
    )]
    fn clear_enemies(enemies: Query<'_, Entity, With<Enemy>>, mut entities: Entities<'_>) {
        for enemy in &enemies {
            entities.despawn(enemy);
        }
    }

    fn arena(scene: &mut Scene<Arena>) {
        scene.resource(Run::default());
        scene.on_enter(|mut entities: Entities<'_>| {
            entities.spawn((Hero, Health(4)));
            entities.spawn((Enemy, Health(0), Position(vec2(2, 0))));
            entities.spawn((Enemy, Health(3), Position(vec2(3, 0))));
        });
        scene.phases(Stage::Gathering);
        scene
            .phase(Stage::Gathering)
            .lasts(seconds(3))
            .then(Stage::Wave);
        scene
            .phase(Stage::Wave)
            .lasts(seconds(60))
            .then(Stage::Intermission)
            .on_enter(next_wave)
            .rules((enemies_die, heroes_fall))
            .on_exit(clear_enemies);
        scene
            .phase(Stage::Intermission)
            .lasts(seconds(15))
            .then(Stage::Wave);
    }

    #[test]
    fn the_wave_drops_a_gem_and_leaves_the_arena_clear() -> Result {
        let mut game = Game::new("Swarm")
            .tick_rate(1)
            .component::<Gem>()
            .scene(arena)
            .scene(|_: &mut Scene<Ended>| {})
            .test()?;
        game.step_for(seconds(3))?;
        assert_eq!(game.phase::<Stage>()?, Stage::Wave);
        assert_eq!(game.resource::<Run>()?.wave, 1);
        game.step()?;
        assert_eq!(game.events::<EnemyDied>(), [EnemyDied { at: vec2(2, 0) }]);
        assert_eq!(game.count::<With<Gem>>(), 1);
        game.step_for(seconds(59))?;
        assert_eq!(game.phase::<Stage>()?, Stage::Intermission);
        assert_eq!(game.count::<With<Enemy>>(), 0);
        game.step_for(seconds(15))?;
        assert_eq!(game.phase::<Stage>()?, Stage::Wave);
        assert_eq!(game.resource::<Run>()?.wave, 2);
        Ok(())
    }
}
