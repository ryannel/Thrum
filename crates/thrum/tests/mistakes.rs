use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Command,
};

use thrum::prelude::*;

#[derive(Resource, Default)]
struct Ledger(u32);
#[derive(Component)]
struct Health(u32);
#[derive(Component)]
struct Left;
#[derive(Component)]
struct Right;
#[derive(Component)]
struct Unknown;
#[derive(Scene, Default)]
struct Play;
#[derive(Scene)]
struct Missing;
#[derive(Phase)]
enum Stage {
    A,
    B,
}

fn diagnostic(error: &Error, parts: &[&str]) {
    let text = error.to_string();
    for part in parts {
        assert!(text.contains(part), "missing {part:?} in {text}");
    }
}

fn default_scene(scene: &mut Scene<Play>) {
    scene.rules(|| {});
}

#[test]
#[expect(
    clippy::items_after_statements,
    reason = "each diagnostic control is shown beside its assertion"
)]
fn startup_names_missing_resources_and_repeated_declarations() {
    fn missing_resource(_: Res<'_, Ledger>) {}
    let game =
        Game::new("missing resource").scene(|scene: &mut Scene| scene.rules(missing_resource));
    diagnostic(
        &game.test().err().expect("test fixture provides this value"),
        &[
            "T0201",
            "missing_resource",
            "Ledger",
            "no provider",
            "Game::resource",
        ],
    );

    fn duplicate_query(_: Query<'_, (&mut Health, &Health)>) {}
    let game = Game::new("duplicate query").scene(|scene: &mut Scene| scene.rules(duplicate_query));
    diagnostic(
        &game.test().err().expect("test fixture provides this value"),
        &[
            "T0202",
            "duplicate_query",
            "parameter 1",
            "Health",
            "twice",
            "Drop",
        ],
    );

    fn duplicate_entities(_: Entities<'_>, _: Entities<'_>) {}
    let game =
        Game::new("duplicate entities").scene(|scene: &mut Scene| scene.rules(duplicate_entities));
    diagnostic(
        &game.test().err().expect("test fixture provides this value"),
        &[
            "T0203",
            "duplicate_entities",
            "parameter 1",
            "parameter 2",
            "Changes",
            "writes",
            "Drop",
        ],
    );

    fn duplicate_phases(scene: &mut Scene<Play>) {
        scene.phases(Stage::A);
        scene.phases(Stage::B);
    }
    let game = Game::new("duplicate phases").scene(duplicate_phases);
    diagnostic(
        &game.test().err().expect("test fixture provides this value"),
        &["T0205", "Play", "called twice", "once"],
    );
}

#[derive(Params)]
struct Writer<'t> {
    ledger: ResMut<'t, Ledger>,
}
#[derive(Params)]
struct Reader<'t> {
    ledger: Res<'t, Ledger>,
}

#[test]
fn startup_resource_aliasing_checks_through_parameter_structs() -> Result {
    fn direct(_: ResMut<'_, Ledger>, _: Res<'_, Ledger>) {}
    fn nested(mut writer: Writer<'_>, _: Res<'_, Ledger>) {
        writer.ledger.0 += 1;
    }
    fn both(_: Writer<'_>, _: Writer<'_>) {}

    let direct = Game::new("direct")
        .resource(Ledger::default())
        .scene(|scene: &mut Scene| scene.rules(direct));
    diagnostic(
        &direct
            .test()
            .err()
            .expect("test fixture provides this value"),
        &[
            "T0203",
            "direct",
            "parameter 1",
            "parameter 2",
            "Ledger",
            "writes",
            "Drop",
        ],
    );
    let nested = Game::new("nested")
        .resource(Ledger::default())
        .scene(|scene: &mut Scene| scene.rules(nested));
    diagnostic(
        &nested
            .test()
            .err()
            .expect("test fixture provides this value"),
        &[
            "T0203",
            "nested",
            ".ledger",
            "parameter 2",
            "Ledger",
            "writes",
            "Drop",
        ],
    );
    let both = Game::new("both")
        .resource(Ledger::default())
        .scene(|scene: &mut Scene| scene.rules(both));
    diagnostic(
        &both.test().err().expect("test fixture provides this value"),
        &[
            "T0203",
            "both",
            ").ledger",
            "parameter 2",
            "Ledger",
            "writes",
            "Drop",
        ],
    );
    let mut permitted = Game::new("read twice")
        .resource(Ledger(7))
        .scene(|scene: &mut Scene| {
            scene.rules(|left: Reader<'_>, right: Res<'_, Ledger>| {
                assert_eq!(left.ledger.0, right.0);
            });
        })
        .test()?;
    permitted.step()
}

fn spawn_both(mut entities: Entities<'_>) {
    entities.spawn((Health(1), Left, Right));
}
fn write_write(_: Query<'_, (&mut Health, &Left)>, _: Query<'_, (&mut Health, &Right)>) {}
fn write_read(_: Query<'_, (&mut Health, &Left)>, _: Query<'_, (&Health, &Right)>) {}

#[derive(Params)]
struct Nested<'t> {
    left: Query<'t, (&'t mut Health, &'t Left)>,
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "the SDK hands rules their declared parameters by value"
)]
fn inside_params(nested: Nested<'_>, _: Query<'_, (&Health, &Right)>) {
    assert!(nested.left.count() < 2);
}

fn conflicting<M>(name: &str, rules: impl Rules<M> + 'static) -> Result {
    let mut game = Game::new(name)
        .scene(|scene: &mut Scene| scene.rules((spawn_both, rules)))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &[
            "T0303",
            name,
            "parameter 1",
            "parameter 2",
            "Health",
            "Entity(1)",
            "writes",
            "disjoint",
        ],
    );
    assert_eq!(game.digest(), before);
    assert_eq!(game.count::<()>(), 0);
    Ok(())
}

#[test]
#[expect(
    clippy::items_after_statements,
    reason = "each diagnostic control is shown beside its assertion"
)]
fn runtime_rejects_query_aliasing_when_the_component_set_exists() -> Result {
    conflicting("write_write", write_write)?;
    conflicting("write_read", write_read)?;
    conflicting("inside_params", inside_params)?;
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the SDK hands rules their declared parameters by value"
    )]
    fn insert_right(mut entities: Entities<'_>, left: Query<'_, Entity, With<Left>>) {
        entities.insert(
            left.iter()
                .next()
                .expect("test fixture provides this value"),
            Right,
        );
    }
    let mut game = Game::new("insert conflict")
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((Health(8), Left));
            });
            scene.rules((insert_right, write_read));
        })
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &[
            "T0303",
            "write_read",
            "parameter 1",
            "parameter 2",
            "Health",
            "Entity(1)",
            "disjoint",
        ],
    );
    assert_eq!(game.digest(), before);
    assert_eq!(game.count::<With<Right>>(), 0);
    Ok(())
}

#[test]
fn disjoint_queries_can_read_or_write_the_same_component_type() -> Result {
    let mut game = Game::new("disjoint")
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((Health(1), Left));
                entities.spawn((Health(2), Right));
            });
            scene.rules((write_write, write_read));
        })
        .test()?;
    game.step()?;
    Game::new("without screen")
        .scene(default_scene)
        .test()?
        .step()
}

#[test]
fn optional_writers_share_empty_cells_and_lend_present_disjoint_rows() -> Result {
    let mut game = Game::new("optional writers")
        .max_entities(3)
        .scene(|scene: &mut Scene| {
            scene.on_enter(|mut entities: Entities<'_>| {
                entities.spawn((Health(1), Left));
                entities.spawn((Health(2), Right));
                entities.spawn((Left, Right));
            });
            scene.rules(
                |mut left: Query<'_, Option<&mut Health>, With<Left>>,
                 mut right: Query<'_, Option<&mut Health>, With<Right>>| {
                    std::thread::scope(|scope| {
                        scope.spawn(|| {
                            for health in (&mut left).into_iter().flatten() {
                                health.0 += 10;
                            }
                        });
                        for health in (&mut right).into_iter().flatten() {
                            health.0 += 20;
                        }
                    });
                },
            );
        })
        .test()?;
    let before = game.digest();
    game.step()?;
    let entities = game.find::<With<Health>>();
    assert_eq!(game.get::<Health>(entities[0])?, Health(11));
    assert_eq!(game.get::<Health>(entities[1])?, Health(22));
    game.rewind(1)?;
    assert_eq!(game.digest(), before);
    Ok(())
}

#[test]
#[expect(
    clippy::items_after_statements,
    reason = "each diagnostic control is shown beside its assertion"
)]
fn runtime_reports_unknown_component_capacity_single_scene_and_rule_failure() -> Result {
    fn unknown(mut entities: Entities<'_>) {
        entities.spawn((Unknown,));
    }
    let mut game = Game::new("unknown component")
        .scene(|scene: &mut Scene| scene.rules(unknown))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &[
            "T0301",
            "unknown",
            "Unknown",
            "is not registered",
            "Game::component",
        ],
    );
    assert_eq!(game.digest(), before);

    fn over_capacity(mut entities: Entities<'_>) {
        entities.spawn(());
        entities.spawn(());
    }
    let mut game = Game::new("capacity")
        .max_entities(1)
        .scene(|scene: &mut Scene| scene.rules(over_capacity))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &["T0302", "over_capacity", "capacity", "Game::max_entities"],
    );
    assert_eq!(game.digest(), before);

    fn only_one(_: Single<'_, &Health>) {}
    let mut game = Game::new("single")
        .scene(|scene: &mut Scene| scene.rules(only_one))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &["T0304", "only_one", "matched 0", "exactly one"],
    );
    assert_eq!(game.digest(), before);
    game.spawn((Health(1),))?;
    game.spawn((Health(2),))?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &["T0304", "only_one", "matched 2", "exactly one"],
    );
    assert_eq!(game.digest(), before);

    fn missing_scene(mut scenes: Scenes<'_>) {
        scenes.go_to(Missing);
    }
    let mut game = Game::new("unknown scene")
        .scene(|scene: &mut Scene| scene.rules(missing_scene))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &[
            "T0305",
            "missing_scene",
            "Missing",
            "never registered",
            "Game::scene",
        ],
    );
    assert_eq!(game.digest(), before);

    fn rule_error() -> Result {
        Err(Error::invalid("quantity", "too large"))
    }
    let mut game = Game::new("error")
        .scene(|scene: &mut Scene| scene.rules(rule_error))
        .test()?;
    let before = game.digest();
    diagnostic(
        &game
            .step()
            .expect_err("test fixture deliberately refuses this operation"),
        &[
            "T0401",
            "rule_error",
            "tick 1",
            "quantity",
            "too large",
            "Correct",
        ],
    );
    assert_eq!(game.digest(), before);
    Ok(())
}

fn compiler_dependencies() -> (PathBuf, Vec<PathBuf>) {
    let binary = std::env::current_exe().expect("locate the integration test");
    let directory = binary
        .parent()
        .expect("the integration test is in a build directory");
    if directory.file_name().is_some_and(|name| name == "out") {
        // The merge runner's Cargo puts each unit in build/<package>/<unit>/out.
        let build = directory
            .ancestors()
            .nth(3)
            .expect("Cargo's build directory");
        let dependencies = children(build)
            .flat_map(|package| children(&package))
            .map(|unit| unit.join("out"))
            .filter(|out| out.is_dir())
            .collect();
        (build.join("thrum"), dependencies)
    } else {
        // A package-only Cargo test may keep every linked unit in deps/.
        (directory.to_path_buf(), vec![directory.to_path_buf()])
    }
}

fn children(directory: &Path) -> impl Iterator<Item = PathBuf> + use<> {
    fs::read_dir(directory)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
}

fn sdk_library(directory: &Path, dependencies: &[PathBuf]) -> PathBuf {
    dependencies
        .iter()
        .filter(|path| path.starts_with(directory))
        .flat_map(|path| children(path))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("libthrum-"))
                && path.extension().is_some_and(|ext| ext == "rlib")
        })
        .max_by_key(|path| {
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
        })
        .expect("Cargo linked the SDK library into this test")
}

#[test]
fn compiler_names_forbidden_world_fields_and_missing_first_default() {
    let (sdk_directory, dependencies) = compiler_dependencies();
    let sdk = sdk_library(&sdk_directory, &dependencies);
    let source = String::from("use thrum::prelude::*;\n")
        + &["Component", "Resource", "Event", "Scene"]
            .into_iter()
            .flat_map(|kind| {
                ["Vec<i32>", "String", "&'static i32", "f32", "usize"]
                    .into_iter()
                    .enumerate()
                    .map(move |(index, ty)| {
                        format!(
                            "#[derive({kind})] struct {kind}{index} {{ invalid_{index}: {ty} }}\n"
                        )
                    })
            })
            .collect::<String>()
        + "#[derive(Scene)] struct NoDefault;\nfn main() { let _ = Game::new(\"bad\").scene(|_: &mut Scene<NoDefault>| {}); }\n";
    let mut rustc = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()));
    rustc.args(["--edition=2024", "--emit=metadata", "--error-format=json"]);
    for directory in dependencies {
        rustc
            .arg("-L")
            .arg(format!("dependency={}", directory.display()));
    }
    let mut child = rustc
        .arg("--extern")
        .arg(format!("thrum={}", sdk.display()))
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("test fixture provides this value");
    child
        .stdin
        .take()
        .expect("test fixture provides this value")
        .write_all(source.as_bytes())
        .expect("test fixture provides this value");
    let output = child
        .wait_with_output()
        .expect("test fixture provides this value");
    assert!(!output.status.success());
    let text = String::from_utf8(output.stderr).expect("rustc returned UTF-8");
    let errors: Vec<serde_json::Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("rustc returned JSON"))
        .filter(|message: &serde_json::Value| message["level"] == "error")
        .collect();

    let expected_fields = ["Vec<i32>", "String", "&i32", "f32", "usize"];
    for (index, declaration) in source.lines().enumerate().skip(1).take(20) {
        assert_diagnostic(
            &errors,
            index + 1,
            declaration,
            &[
                "T0101",
                expected_fields[(index - 1) % expected_fields.len()],
                "has no portable world representation",
                "use a fixed-width integer, Num, or a bounded value built from Data fields",
            ],
        );
    }
    assert_diagnostic(
        &errors,
        23,
        "Game::new(\"bad\").scene",
        &[
            "T0103",
            "NoDefault",
            "cannot supply a default value",
            "derive or implement Default on this scene, or register a defaultable scene first",
        ],
    );
}

fn assert_diagnostic(
    errors: &[serde_json::Value],
    line: usize,
    declaration: &str,
    expected: &[&str],
) {
    assert!(
        errors.iter().any(|error| {
            let at_declaration = error["spans"].as_array().is_some_and(|spans| {
                spans.iter().any(|span| {
                    span["is_primary"] == true
                        && span["line_start"] == line
                        && span["text"].as_array().is_some_and(|source| {
                            source.iter().any(|part| {
                                part["text"]
                                    .as_str()
                                    .is_some_and(|text| text.contains(declaration))
                            })
                        })
                })
            });
            at_declaration
                && error["rendered"]
                    .as_str()
                    .is_some_and(|text| expected.iter().all(|part| text.contains(part)))
        }),
        "line {line} ({declaration}) needs a diagnostic containing {expected:?}"
    );
}

#[test]
fn the_registered_runtime_refuses_non_finite_starting_and_entering_values() -> Result {
    #[derive(Resource)]
    struct Speed(Num);
    #[derive(Scene)]
    struct Finished(Num);

    let bad = Num::ONE / Num::ZERO;
    let initial = Game::new("bad starting resource")
        .resource(Speed(bad))
        .scene(|_: &mut Scene| {})
        .test();
    assert!(
        initial
            .err()
            .expect("invalid initial resource")
            .to_string()
            .contains("non-finite Num")
    );

    let local = Game::new("bad scene resource")
        .scene(move |scene: &mut Scene| scene.resource(Speed(bad)))
        .test();
    assert!(
        local
            .err()
            .expect("invalid scene resource")
            .to_string()
            .contains("non-finite Num")
    );

    let mut game = Game::new("bad landing")
        .scene(move |scene: &mut Scene| {
            scene.rules(move |mut scenes: Scenes<'_>| {
                scenes.go_to(Finished(bad));
            });
        })
        .scene(|_: &mut Scene<Finished>| {})
        .test()?;
    let initial = game.digest();
    assert!(
        game.go_to(Finished(bad))
            .expect_err("invalid explicit scene")
            .to_string()
            .contains("non-finite Num")
    );
    assert!(
        game.step()
            .expect_err("invalid scene request")
            .to_string()
            .contains("non-finite Num")
    );
    assert_eq!(game.digest(), initial);
    Ok(())
}
