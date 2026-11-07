use super::SceneCamera;
use glam::{Mat4, Vec3, Vec4};
use thrum_render::synthetic::SyntheticView;

fn camera() -> SceneCamera {
    SceneCamera::perspective(
        [0.0, 127.114_25, 16.0],
        [8.0, 123.114_25, -60.0],
        46.0_f32.to_radians(),
        0.1,
        30_000.0,
    )
    .expect("authored valley camera is valid")
}

fn project(view: SyntheticView, position: [f32; 3]) -> Vec3 {
    let matrix = Mat4::from_cols_array_2d(&view.view_proj_columns());
    let clip = matrix * Vec3::from_array(position).extend(1.0);
    clip.truncate() / clip.w
}

#[test]
fn authored_pose_keeps_target_centred_and_reverse_depth_interval() {
    let camera = camera();
    let [pitch, yaw] = camera.orientation();
    let view = camera.view([1920, 1080], pitch, yaw).unwrap();
    assert_eq!(view.camera_position(), [0.0, 127.114_25, 16.0]);
    let target = project(view, [8.0, 123.114_25, -60.0]);
    assert!(target.x.abs() < 1e-5 && target.y.abs() < 1e-5);
    let inverse = Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
    for (depth, expected_distance) in [(1.0, 0.1), (0.0, 30_000.0)] {
        let world = inverse * Vec4::new(0.0, 0.0, depth, 1.0);
        let distance = (world.truncate() / world.w - Vec3::from_array(camera.eye)).length();
        assert!((distance / expected_distance - 1.0).abs() < 0.002);
    }
}

#[test]
fn resizing_changes_horizontal_field_while_preserving_eye_and_vertical_field() {
    let camera = camera();
    let [pitch, yaw] = camera.orientation();
    let wide = camera.view([1600, 900], pitch, yaw).unwrap();
    let narrow = camera.view([900, 1600], pitch, yaw).unwrap();
    assert_eq!(wide.camera_position(), narrow.camera_position());
    let point = [10.0, 130.0, -60.0];
    let a = project(wide, point);
    let b = project(narrow, point);
    assert!((a.y - b.y).abs() < 1e-6);
    assert!((b.x / a.x - (1600.0_f32 / 900.0).powi(2)).abs() < 1e-5);
    let turned = camera.view([1600, 900], 0.15, 0.2).unwrap();
    assert_eq!(turned.camera_position(), wide.camera_position());
    assert_ne!(turned.view_proj_columns(), wide.view_proj_columns());
    assert!(camera.view([0, 900], pitch, yaw).is_err());
    assert!(camera.view([1600, 0], pitch, yaw).is_err());
}

#[test]
fn invalid_authored_cameras_are_refused_before_activation() {
    let valid_eye = [0.0, 127.0, 16.0];
    let valid_target = [0.0, 123.0, -60.0];
    for eye in [
        [0.0, -0.01, 16.0],
        [0.0, 100_001.0, 16.0],
        [0.0, f32::NAN, 16.0],
        [f32::INFINITY, 0.0, 16.0],
    ] {
        assert!(SceneCamera::perspective(eye, valid_target, 1.0, 0.1, 30_000.0).is_err());
    }
    for target in [valid_eye, [0.0, 128.0, 16.0], [f32::NAN, 123.0, -60.0]] {
        assert!(SceneCamera::perspective(valid_eye, target, 1.0, 0.1, 30_000.0).is_err());
    }
    for (fov, near, far) in [
        (0.0, 0.1, 30_000.0),
        (f32::NAN, 0.1, 30_000.0),
        (1.0, 0.0, 30_000.0),
        (1.0, 1.0, 0.1),
        (1.0, 0.1, f32::INFINITY),
    ] {
        assert!(SceneCamera::perspective(valid_eye, valid_target, fov, near, far).is_err());
    }
    for altitude in [0.0, 100_000.0] {
        assert!(
            SceneCamera::perspective(
                [0.0, altitude, 1.0],
                [0.0, altitude, 0.0],
                1.0,
                0.1,
                30_000.0
            )
            .is_ok()
        );
    }
}
