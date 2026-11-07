use super::*;

fn rock() -> MeshPlacement {
    MeshPlacement::new("meshes/rock", "textures/white")
}

#[test]
fn built_placements_are_rigid_and_compose_in_call_order() {
    let stone = rock()
        .rotate_degrees([0.0, 0.0, 1.0], 90.0)
        .yaw_degrees(90.0)
        .scale(2.0)
        .at([1.0, 0.0, -3.0]);
    assert_eq!(stone.check(0), Ok(()));
    let model = glam::Mat4::from_cols_array_2d(&stone.transform);
    // +X stands up, then the yaw leaves vertical vectors vertical.
    assert!(
        model
            .transform_vector3(glam::Vec3::X)
            .abs_diff_eq(glam::Vec3::new(0.0, 2.0, 0.0), 1e-5)
    );
    // North (-Z) turns to face east (+X) under a 90° clockwise yaw.
    let turned = rock().yaw_degrees(90.0);
    let north =
        glam::Mat4::from_cols_array_2d(&turned.transform).transform_vector3(glam::Vec3::NEG_Z);
    assert!(north.abs_diff_eq(glam::Vec3::X, 1e-5), "{north}");
    assert_eq!(model.w_axis.truncate(), glam::Vec3::new(1.0, 0.0, -3.0));
}

#[test]
fn non_rigid_transforms_are_refused_by_name_and_cause() {
    let mut stretched = rock();
    stretched.transform[1][1] = 3.0;
    let message = stretched.check(4).unwrap_err().to_string();
    assert!(
        message.contains("mesh placement 4 (\"meshes/rock\")"),
        "{message}"
    );
    assert!(message.contains("unevenly (X 1, Y 3, Z 1)"), "{message}");
    assert!(message.contains(".scale"), "{message}");

    let mut mirrored = rock();
    mirrored.transform[0][0] = -1.0;
    assert_eq!(mirrored.check(0).unwrap_err().fault, Fault::Mirrored);
    assert_eq!(
        rock().scale(-1.0).check(0).unwrap_err().fault,
        Fault::Mirrored
    );
    assert!(matches!(
        rock().scale(0.0).check(0).unwrap_err().fault,
        Fault::Flattened(_)
    ));

    let mut sheared = rock();
    sheared.transform[1][0] = 0.5;
    sheared.transform[1][1] = (1.0_f32 - 0.25).sqrt();
    assert_eq!(sheared.check(0).unwrap_err().fault, Fault::Sheared);

    let mut projective = rock();
    projective.transform[0][3] = 0.1;
    assert_eq!(projective.check(0).unwrap_err().fault, Fault::Projective);
    assert_eq!(
        rock().at([f32::NAN, 0.0, 0.0]).check(0).unwrap_err().fault,
        Fault::NotFinite
    );
    // A zero axis cannot name a turn.
    assert_eq!(
        rock()
            .rotate_degrees([0.0; 3], 10.0)
            .check(0)
            .unwrap_err()
            .fault,
        Fault::NotFinite
    );
}

#[test]
fn resting_placements_put_the_lowest_vertex_at_their_height() {
    // A tapered shape: its box corner at (-1.5, -0.8) is not a vertex.
    let points = [
        glam::Vec3::new(-1.5, 0.0, 0.0),
        glam::Vec3::new(1.4, 0.0, 0.0),
        glam::Vec3::new(0.0, -0.8, 0.0),
        glam::Vec3::new(0.0, 0.9, 0.0),
        glam::Vec3::new(0.0, 0.0, 1.0),
    ];
    let lying = rock().at([2.0, 0.5, 0.0]).scale(2.0).on_ground();
    let model = placed_model(&lying, &points);
    assert!((model.w_axis.y - (0.5 + 1.6)).abs() < 1e-5);
    // Standing the mesh's X axis on end rests its -X tip (-1.5) on the ground.
    let standing = rock().rotate_degrees([0.0, 0.0, 1.0], 90.0).on_ground();
    let model = placed_model(&standing, &points);
    assert!((model.w_axis.y - 1.5).abs() < 1e-5, "{}", model.w_axis.y);
    // Tilted 45°, the lowest vertex touches the ground; a box corner would leave it raised.
    let tilted = rock().rotate_degrees([0.0, 0.0, 1.0], 45.0).on_ground();
    let model = placed_model(&tilted, &points);
    let lowest = points
        .iter()
        .map(|&point| model.transform_point3(point).y)
        .fold(f32::INFINITY, f32::min);
    assert!(lowest.abs() < 1e-5, "{lowest}");
    // Without the request, or without points, the origin stays where it was put.
    assert_eq!(
        placed_model(&rock().at([0.0, 0.5, 0.0]), &points).w_axis.y,
        0.5
    );
    assert_eq!(placed_model(&lying, &[]).w_axis.y, 0.5);
}
