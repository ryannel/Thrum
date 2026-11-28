use thrum::prelude::*;

use crate::prefabs::rustc;

fn assert_text(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(text.contains(part), "missing {part:?} in {text}");
    }
}

#[test]
fn required_input_is_a_compile_error() {
    let output = rustc(
        r#"
        use thrum::prelude::*;
        #[derive(Component)] struct Shot;
        #[derive(Component)] struct Velocity;
        prefabs! { const SHOT: Prefab<(Shot, Velocity)> = "shot"; }
        fn fire(mut entities: Entities<'_>) { entities.spawn(SHOT.at(0, 0)); }
        fn main() {}
    "#,
        false,
    );
    assert!(!output.status.success());
    assert_text(
        &String::from_utf8_lossy(&output.stderr),
        &[
            "T0110",
            "Shot",
            "Velocity",
            "missing required components",
            "Prefab::with",
        ],
    );
}

#[test]
fn linked_declarations_are_checked_before_any_rule_runs() {
    for (names, definition, in_rule) in [
        (
            r#"const BAT: Prefab = "bat"; const LOST: Prefab = "lost";"#,
            "game.prefab(BAT, ())",
            true,
        ),
        (r#"const LOST: Prefab = "lost";"#, "game", true),
        (
            r#"const BAT: Prefab = "bat"; const LOST: Prefab = "lost";"#,
            "game.prefab(BAT, LOST.variant())",
            false,
        ),
        (
            r#"const BAT: Prefab = "bat"; const LOST: Prefab = "lost";"#,
            "game.prefab(BAT, PrefabDef::new(()).part(Part::new(\"eye\"), LOST.at_local(1, 2)))",
            false,
        ),
    ] {
        let rule = if in_rule {
            String::from(
                "scene.rules(|tick: Tick, mut entities: Entities<'_>| { if tick.seconds() == seconds(100) { entities.spawn(LOST.at(0, 0)); } });",
            )
        } else {
            String::from("let _ = scene;")
        };
        let source = format!(
            r#"
            use thrum::prelude::*;
            prefabs! {{ {names} }}
            fn main() {{
                let game = Game::new("missing prefab").tick_rate(1);
                let game = {definition}.scene(|scene: &mut Scene| {{ {rule} }});
                let error = game.test().err().expect("must refuse at startup");
                println!("{{error}}");
            }}
        "#
        );
        let output = rustc(&source, true);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_text(
            &String::from_utf8_lossy(&output.stdout),
            &[
                "T0210",
                "lost",
                "LOST",
                "prefabs! block linked_game",
                "declared without a definition",
                "Game::prefab",
            ],
        );
    }
}

#[test]
fn prefab_components_are_checked_against_rule_parameters_at_startup() {
    let output = rustc(
        r#"
        use thrum::prelude::*;
        #[derive(Component)] struct Health(u32);
        #[derive(Component)] struct Left;
        #[derive(Component)] struct Right;
        prefabs! { const BAT: Prefab = "bat"; }
        #[derive(Params)]
        struct Fetch<'a> {
            left: Query<'a, Option<&'a mut Health>, With<Left>>,
            right: Query<'a, &'a Health, With<Right>>,
        }
        fn conflicting(_: Fetch) {}
        fn main() {
            let game = Game::new("query overlap").prefab(BAT, (Left, Right, Health(3)))
                .scene(|scene: &mut Scene| scene.rules(conflicting));
            println!("{}", game.test().err().expect("overlap must refuse at startup"));
        }
    "#,
        true,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_text(
        &String::from_utf8_lossy(&output.stdout),
        &[
            "T0303",
            "conflicting",
            "left",
            "right",
            "Health",
            "prefab bat",
            "one writes",
            "disjoint",
        ],
    );
}

#[derive(Resource, Default)]
struct Pair(Option<(Entity, Entity)>);

fn setup(mut entities: Entities<'_>, mut pair: ResMut<'_, Pair>) {
    let parent = entities.spawn((Transform2d::at(10, 0).scaled(0),));
    let child = entities.spawn((Transform2d::at(1, 2),));
    pair.0 = Some((parent, child));
}

#[test]
fn invalid_family_edits_leave_the_tick_and_places_unchanged() -> Result {
    for (kind, required) in [
        ("part", ["T0309", "eye", "no such part", "PrefabDef::part"]),
        ("cycle", ["T0310", "Entity", "cycle", "descendants"]),
        ("zero", ["T0311", "Entity", "scale 0", "Use keep_local"]),
        ("tiny", ["T0311", "Entity", "1/10,000", "Use keep_local"]),
        ("residual", ["T0312", "Entity", "residual", "exceeds bound"]),
    ] {
        let mut game = Game::new("refusal")
            .max_entities(8)
            .resource(Pair::default())
            .scene(move |scene: &mut Scene| {
                scene.on_enter(setup);
                match kind {
                    "part" => scene.rules(|pair: Res<'_, Pair>, family: Family<'_>| {
                        family.part(
                            pair.0.expect("fixture provides this value").0,
                            Part::new("eye"),
                        );
                    }),
                    "cycle" => scene.rules(|pair: Res<'_, Pair>, mut entities: Entities<'_>| {
                        let (parent, child) = pair.0.expect("fixture provides this value");
                        entities
                            .attach(child)
                            .to(parent)
                            .keep_local(Transform2d::ORIGIN);
                        entities
                            .attach(parent)
                            .to(child)
                            .keep_local(Transform2d::ORIGIN);
                    }),
                    _ => scene.rules(|pair: Res<'_, Pair>, mut entities: Entities<'_>| {
                        let (parent, child) = pair.0.expect("fixture provides this value");
                        entities.attach(child).to(parent).keep_world();
                    }),
                }
            })
            .test()?;
        let (parent, child) = game
            .resource::<Pair>()?
            .0
            .expect("fixture provides this value");
        if kind == "tiny" {
            game.set(
                parent,
                Transform2d::at(10, 0).scaled(Num::ONE / Num::from(20_000)),
            )?;
        } else if kind == "residual" {
            // The old transform is finite, but inverting the parent overflows.
            game.set(
                parent,
                Transform2d::at(3, 0).scaled(Num::ONE / Num::from(10_000)),
            )?;
            let huge = (0..9).fold(Num::ONE, |value, _| value * Num::from(10_000));
            game.set(child, Transform2d::at(huge, 2))?;
        }
        let before = game.digest();
        let local = game.get::<Transform2d>(child)?;
        let error = game.step().expect_err(kind);
        assert_text(&error.to_string(), &required);
        assert_eq!(game.digest(), before);
        assert!(game.get::<Transform2d>(child)?.same_encoding(&local));
        game.run(move |family: Family<'_>| {
            assert_eq!(family.parent(child), None);
            assert!(family.world2d(child).same_encoding(&local));
        })?;
    }
    Ok(())
}

#[test]
fn a_missing_named_part_refuses_the_prefab_tick() {
    let output = rustc(
        r#"
        use thrum::prelude::*;
        prefabs! { const BAT: Prefab = "bat"; }
        const EYE: Part = Part::new("eye");
        fn inspect(bat: Single<'_, Entity>, mut entities: Entities<'_>) {
            entities.part(*bat, EYE);
            entities.despawn(*bat);
        }
        fn main() -> Result {
            let mut game = Game::new("missing part").max_entities(4)
                .prefab(BAT, ())
                .scene(|scene: &mut Scene| scene.rules(inspect)).test()?;
            let bat = game.spawn(BAT.at(2, 3))?;
            let digest = game.digest();
            let error = game.step().expect_err("the bat's prefab has no eye");
            assert_eq!(game.digest(), digest);
            assert!(game.alive(bat));
            assert_eq!(game.get::<Transform2d>(bat)?, Transform2d::at(2, 3));
            println!("{error}");
            Ok(())
        }
        "#,
        true,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_text(
        &String::from_utf8_lossy(&output.stdout),
        &["T0309", "inspect", "eye", "no such part", "PrefabDef::part"],
    );
}
