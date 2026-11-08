//! Independent finite-Sun reference using solid-angle-weighted 3D rays.

const COLUMNS: i32 = 256;
const ROWS: i32 = 128;

/// Integrate opaque triangle visibility across the actual angular disk.
pub(super) fn visibility(
    origin: [f64; 3],
    to_sun: [f64; 3],
    angular_radius: f64,
    triangles: &[[[f64; 3]; 3]],
) -> f64 {
    let sun_length = dot(to_sun, to_sun).sqrt();
    let to_sun = to_sun.map(|axis| axis / sun_length);
    let axis = if to_sun[1].abs() > 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let across = cross(to_sun, axis);
    let right_length = dot(across, across).sqrt();
    let right = across.map(|axis| axis / right_length);
    let up = cross(right, to_sun);
    let radius = angular_radius.tan();
    let mut total = 0.0;
    let mut open = 0.0;

    for column in 0..COLUMNS {
        let x = 2.0 * (f64::from(column) + 0.5) / f64::from(COLUMNS) - 1.0;
        let half_chord = (1.0 - x * x).sqrt();
        for row in 0..ROWS {
            let y = half_chord * (2.0 * (f64::from(row) + 0.5) / f64::from(ROWS) - 1.0);
            let direction = std::array::from_fn(|axis| {
                to_sun[axis] + radius * (x * right[axis] + y * up[axis])
            });
            // Perspective projection from the disk plane to unit directions.
            let weight = half_chord / dot(direction, direction).powf(1.5);
            total += weight;
            if !triangles.iter().any(|&tri| hits(origin, direction, tri)) {
                open += weight;
            }
        }
    }
    open / total
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn hits(origin: [f64; 3], direction: [f64; 3], [a, b, c]: [[f64; 3]; 3]) -> bool {
    let edge_ab = std::array::from_fn(|axis| b[axis] - a[axis]);
    let edge_ac = std::array::from_fn(|axis| c[axis] - a[axis]);
    let across = cross(direction, edge_ac);
    let determinant = dot(edge_ab, across);
    let determinant_scale =
        (dot(edge_ab, edge_ab) * dot(edge_ac, edge_ac) * dot(direction, direction)).sqrt();
    if determinant.abs() <= 8.0 * f64::EPSILON * determinant_scale {
        return false;
    }
    let from_a = std::array::from_fn(|axis| origin[axis] - a[axis]);
    let u = dot(from_a, across) / determinant;
    let side = cross(from_a, edge_ab);
    let v = dot(direction, side) / determinant;
    u >= 0.0 && v >= 0.0 && u + v <= 1.0 && dot(edge_ac, side) / determinant > 0.0
}

#[test]
fn overlapping_half_disk_casters_leave_one_quarter_open() {
    let above = [
        [10.0, 0.0, -1.0],
        [10.0, 1.0, -1.0],
        [10.0, 1.0, 1.0],
        [10.0, 0.0, 1.0],
    ];
    let right = above.map(|[x, y, z]| [x, z, y]);
    let blockers = [
        [above[0], above[1], above[2]],
        [above[0], above[2], above[3]],
        [right[0], right[1], right[2]],
        [right[0], right[2], right[3]],
    ];
    let radius = f64::from(super::SUN_ANGULAR_RADIUS);
    let half = visibility([0.0; 3], [1.0, 0.0, 0.0], radius, &blockers[..2]);
    let overlap = visibility([0.0; 3], [1.0, 0.0, 0.0], radius, &blockers);
    let coincident = [blockers[0], blockers[1], blockers[0], blockers[1]];
    let duplicated = visibility([0.0; 3], [1.0, 0.0, 0.0], radius, &coincident);
    assert!((half - 0.5).abs() < 1e-10, "half-disk control: {half}");
    assert!((overlap - 0.25).abs() < 1e-10, "overlap: {overlap}");
    assert!(
        (duplicated - half).abs() < 1e-10,
        "coincident: {duplicated}"
    );

    let scaled = blockers.map(|tri| tri.map(|p| p.map(|axis| 1e-8 * axis)));
    let small_overlap = visibility([0.0; 3], [1.0, 0.0, 0.0], radius, &scaled);
    assert!(
        (small_overlap - overlap).abs() < 1e-10,
        "scaled: {small_overlap}"
    );
}
