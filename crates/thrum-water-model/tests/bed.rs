//! Vertical-ray rasterization against independently specified ground.
use crate::support;
use support::fixtures::close;
use thrum_water_model::{Bed, BedPyramid};

#[test]
fn pyramid_encloses_bilinear_ground_on_partial_odd_blocks() {
    let bed = support::fixtures::grid([17, 13], |x, z| (x * 0.7).sin() * 3.0 + z * 0.2);
    let pyramid = BedPyramid::new(&bed);
    for point in [[0.2, 0.3], [7.3, 6.7], [15.9, 11.9]] {
        for radius in [0.0, 0.2, 1.5, 3.7] {
            let [lo, hi] = pyramid.range(point, radius).unwrap();
            for z in -10..=10 {
                for x in -10..=10 {
                    let p = [
                        point[0] + f64::from(x) * radius / 10.0,
                        point[1] + f64::from(z) * radius / 10.0,
                    ];
                    if let Some(h) = bed.height(p) {
                        assert!(h >= lo - 1e-10 && h <= hi + 1e-10);
                    }
                }
            }
        }
    }
    assert!(pyramid.range([-10.0; 2], 1.0).is_none());
}

#[test]
fn fallback_only_fills_uncovered_centres() {
    let triangle = |height| [[0.0, height, 0.0], [4.0, height, 0.0], [0.0, height, 4.0]];
    for triangles in [
        vec![triangle(-2.0)],
        vec![triangle(-3.0), triangle(-2.0)],
        vec![triangle(-2.0), triangle(-3.0)],
    ] {
        let bed = Bed::from_triangles([0.0; 2], 1.0, [5, 5], 0.0, &triangles).unwrap();
        close(bed.height([1.0, 1.0]).unwrap(), -2.0);
        close(bed.height([4.0, 4.0]).unwrap(), 0.0);
    }
}

#[test]
fn tilted_plane_and_highest_overlap_are_exact_at_centres() {
    let plane = |x, z| [x, 2.0 + 0.3 * x - 0.2 * z, z];
    let [a, b, c, d] = [
        plane(0.0, 0.0),
        plane(10.0, 0.0),
        plane(10.0, 10.0),
        plane(0.0, 10.0),
    ];
    let bed =
        Bed::from_triangles([0.0; 2], 1.0, [11, 11], -100.0, &[[a, b, c], [a, c, d]]).unwrap();
    for z in 0..11 {
        for x in 0..11 {
            close(
                bed.height([f64::from(x), f64::from(z)]).unwrap(),
                2.0 + 0.3 * f64::from(x) - 0.2 * f64::from(z),
            );
        }
    }
    let high = [[0.0, 8.0, 0.0], [10.0, 8.0, 0.0], [0.0, 8.0, 10.0]];
    let bed = Bed::from_triangles(
        [0.0; 2],
        1.0,
        [11, 11],
        -100.0,
        &[[a, b, c], [a, c, d], high],
    )
    .unwrap();
    close(bed.height([2.0, 2.0]).unwrap(), 8.0);
}

#[test]
fn tessellated_bowl_matches_quadratic_within_one_cell_height_span() {
    let vertex = |x: f64, z: f64| [x, (x - 5.0).powi(2) * 0.1 + (z - 5.0).powi(2) * 0.1, z];
    let mut triangles = vec![];
    for z in 0..10 {
        for x in 0..10 {
            let (x, z) = (f64::from(x), f64::from(z));
            let [a, b, c, d] = [
                vertex(x, z),
                vertex(x + 1.0, z),
                vertex(x + 1.0, z + 1.0),
                vertex(x, z + 1.0),
            ];
            triangles.extend([[a, b, c], [a, c, d]]);
        }
    }
    let bed = Bed::from_triangles([0.5; 2], 1.0, [10, 10], -100.0, &triangles).unwrap();
    for z in 0..10 {
        for x in 0..10 {
            let p = [f64::from(x) + 0.5, f64::from(z) + 0.5];
            let expected = 0.1 * ((p[0] - 5.0).powi(2) + (p[1] - 5.0).powi(2));
            assert!((bed.height(p).unwrap() - expected).abs() <= 0.05 + 1e-12);
        }
    }
    assert!(Bed::new([0.0; 2], 0.0, [2, 2], vec![0.0; 4]).is_err());
}
