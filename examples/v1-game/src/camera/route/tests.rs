use super::{CameraRoute, ROUTE_FRAME_CAP};
use crate::{RouteOverlay, SceneCamera};

const EYE: [f32; 3] = [0.0, 244.92, 16.0];

fn camera() -> SceneCamera {
    SceneCamera::perspective(
        EYE,
        [0.0, 240.92, -60.0],
        46_f32.to_radians(),
        0.1,
        30_000.0,
    )
    .expect("valley camera")
}

fn key(tick: u64, x: f32, yaw: f32) -> serde_json::Value {
    serde_json::json!({"tick": tick, "eye": [x, EYE[1], EYE[2]], "yaw_degrees": yaw, "pitch_degrees": -3.0})
}

fn file(keys: &[serde_json::Value], cuts: &[u64], checkpoints: &[(u64, &str)]) -> String {
    serde_json::json!({
        "format": "thrum-v1-moving-route",
        "version": 1,
        "cooked_root": "root",
        "cooked_generation": 1,
        "tick_hz": 60,
        "phase": [1, 2],
        "origin_tick": 100,
        "start": {"solar_hours": 17.3},
        "render_extent": [1280, 720],
        "keys": keys,
        "cuts": cuts,
        "checkpoints": checkpoints.iter().map(|(tick, name)| serde_json::json!({"tick": tick, "name": name})).collect::<Vec<_>>(),
        "overlay": "proof",
    })
    .to_string()
}

fn cut_route() -> CameraRoute {
    let keys = vec![
        key(0, 0.0, 0.0),
        key(10, 1.0, 10.0),
        key(10, 5.0, 40.0),
        key(20, 7.0, 60.0),
    ];
    CameraRoute::parse(&file(&keys, &[10], &[(5, "a"), (15, "b")]), camera(), 60).expect("route")
}

#[test]
fn worked_cut_example_and_clamp() {
    let route = cut_route();
    assert_eq!(route.overlay, RouteOverlay::Proof);
    // s = tick − 1 − origin + alpha.
    let before = route.pose(110, 0.5);
    assert!((before.eye[0] - 0.95).abs() < 1e-6, "{before:?}");
    assert_eq!(before.epoch, 1);
    let after = route.pose(111, 0.5);
    assert!((after.eye[0] - 5.1).abs() < 1e-5, "{after:?}");
    assert!((after.yaw - 41_f32.to_radians()).abs() < 1e-6);
    assert_eq!(after.epoch, 2);
    // At the origin s = −0.5, clamped to key 0; past the end, the last key.
    assert_eq!(route.pose(100, 0.5).eye[0], 0.0);
    assert_eq!(route.pose(90, 0.0).eye[0], 0.0);
    assert_eq!(route.pose(500, 0.5).eye[0], 7.0);
    assert_eq!(route.pose(500, 0.5).epoch, 2);
}

#[test]
fn start_must_match_the_authored_pose() {
    let route = cut_route();
    let [pitch, yaw] = camera().orientation();
    assert!(
        route
            .check_start(camera(), (-3_f32).to_radians(), yaw)
            .is_ok()
    );
    assert!(route.check_start(camera(), pitch, yaw + 0.1).is_err());
    let moved = camera().at([1.0, EYE[1], EYE[2]]).expect("moved");
    assert!(
        route
            .check_start(moved, (-3_f32).to_radians(), yaw)
            .is_err()
    );
}

#[test]
fn refusals() {
    let ok = || vec![key(0, 0.0, 0.0), key(20, 1.0, 10.0)];
    let parse = |text: String| CameraRoute::parse(&text, camera(), 60);
    assert!(parse(file(&ok(), &[], &[(5, "a")])).is_ok());
    // A held preset is not a route.
    assert!(parse(r#"{"world.paused": true, "sky.solar_time": 12}"#.to_owned()).is_err());
    // Clock rate and phase.
    assert!(CameraRoute::parse(&file(&ok(), &[], &[]), camera(), 30).is_err());
    let mut half = serde_json::from_str::<serde_json::Value>(&file(&ok(), &[], &[])).unwrap();
    half["phase"] = serde_json::json!([1, 10]);
    assert!(parse(half.to_string()).is_err());
    // A yaw step over π.
    assert!(parse(file(&[key(0, 0.0, 0.0), key(20, 1.0, 190.0)], &[], &[])).is_err());
    // Equal ticks without a declared cut, and a cut without a pair.
    assert!(
        parse(file(
            &[
                key(0, 0.0, 0.0),
                key(5, 1.0, 0.0),
                key(5, 2.0, 0.0),
                key(9, 2.0, 0.0)
            ],
            &[],
            &[]
        ))
        .is_err()
    );
    assert!(parse(file(&ok(), &[10], &[])).is_err());
    // Three keys at one tick.
    assert!(
        parse(file(
            &[
                key(0, 0.0, 0.0),
                key(5, 1.0, 0.0),
                key(5, 2.0, 0.0),
                key(5, 3.0, 0.0),
                key(9, 2.0, 0.0)
            ],
            &[5],
            &[]
        ))
        .is_err()
    );
    // A route ending on a cut.
    assert!(
        parse(file(
            &[key(0, 0.0, 0.0), key(5, 1.0, 0.0), key(5, 2.0, 0.0)],
            &[5],
            &[]
        ))
        .is_err()
    );
    // A key below sea level, and one not at tick 0 first.
    let mut low = key(20, 1.0, 0.0);
    low["eye"][1] = serde_json::json!(-1.0);
    assert!(parse(file(&[key(0, 0.0, 0.0), low], &[], &[])).is_err());
    assert!(parse(file(&[key(1, 0.0, 0.0), key(20, 1.0, 0.0)], &[], &[])).is_err());
    // Checkpoints: unsettled after the origin or a cut, past the end, duplicate names.
    assert!(parse(file(&ok(), &[], &[(3, "a")])).is_err());
    let cut = vec![
        key(0, 0.0, 0.0),
        key(10, 1.0, 0.0),
        key(10, 2.0, 0.0),
        key(20, 2.0, 0.0),
    ];
    assert!(parse(file(&cut, &[10], &[(12, "a")])).is_err());
    assert!(parse(file(&cut, &[10], &[(14, "a")])).is_ok());
    assert!(parse(file(&ok(), &[], &[(21, "a")])).is_err());
    assert!(parse(file(&ok(), &[], &[(5, "a"), (6, "a")])).is_err());
    // The whole run must fit the frame cap.
    let long = vec![key(0, 0.0, 0.0), key(ROUTE_FRAME_CAP - 100, 1.0, 0.0)];
    assert!(parse(file(&long, &[], &[])).is_err());
}
