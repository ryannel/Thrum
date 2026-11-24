//! Showcase gameplay and simulation running with the proving ground compiled out.
//! The default build runs these suites through their individually pinned targets.

#[cfg(not(feature = "proving"))]
mod exit_scene;
#[cfg(not(feature = "proving"))]
mod simulation;

#[test]
fn showcase_builds_its_authored_world() {
    let world = v1_game::exit_authority().expect("showcase world");
    assert_eq!(world.tick(), 0);
    assert!(v1_game::controls::GameControls::new().is_ok());
}
