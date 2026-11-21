//! Closed forms for slabs and ground, and independently authored voxel sums.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{Density, Fidelity, Shape, Weather};

#[test]
fn deck_can_enclose_a_hill_and_vertical_depth_has_a_closed_form() {
    let field = support::field(11, 0.0);
    let state = support::state(1.0, 1.0);
    let c = field.query([0.0; 3], &state);
    let surface = ground_support::flat(c.base + 0.4 * (c.top - c.base));
    let ground = ground_support::table(vec![]);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let p = [0.0, c.base + 0.4 * (c.top - c.base), 0.0];
    let a = w.extinction(p, &state, |_| None);
    support::close(a.value, 0.05 * c.coverage, 1e-14);
    assert_eq!(a.fidelity, Fidelity::Exact);
    assert_eq!(
        w.extinction([0.0, c.top + 1.0, 0.0], &state, |_| None)
            .value,
        0.0
    );
    let tau = w
        .optical_depth(
            [0.0, c.base - 10.0, 0.0],
            [0.0, c.top + 10.0, 0.0],
            &state,
            10_000,
            Density::Coarse,
            |_| None,
        )
        .expect("deck segment");
    let closed = 0.05 * c.coverage * (c.top - c.base) * 0.75;
    support::close(tau.value, closed, 1e-8);
}

#[test]
fn ground_integrals_range_and_cell_quadrature() {
    let field = support::field(3, 0.0);
    let surface = ground_support::flat(0.0);
    let ground = ground_support::table(vec![ground_support::row()]);
    let mut state = support::state(0.0, 0.0);
    ground_support::controls(&mut state, vec![1.0], 0.0);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let tau = w
        .optical_depth(
            [0.0, -10.0, 0.0],
            [0.0, 110.0, 0.0],
            &state,
            12_000,
            Density::Coarse,
            |_| None,
        )
        .expect("ground segment");
    support::close(tau.value, 0.02 * (100.0 - 20.0 / 2.0), 1e-11);
    support::close(w.column([0.0; 3], &state).optical_depth, 1.8, 1e-14);
    let a = w.extinction([0.0, 20.0, 0.0], &state, |_| None);
    support::close(a.value, 0.02, 1e-15);
    support::close(
        w.visual_range([0.0, 20.0, 0.0], &state, |_| None).value,
        3.912 / (0.02 + 3.912 / 60_000.0),
        1e-12,
    );
    assert_eq!(a.fidelity, Fidelity::Exact);
    let mut cells = support::state(0.8, 0.0);
    cells.step(1, 720.0).expect("cells age");
    let w = Weather {
        field: &field,
        surface: None,
        ground: None,
    };
    // This diagonal intersects the deeper turret at weather time 720 s.
    let from = [-1000.0, 700.0, 500.0];
    let to = [1000.0, 1100.0, 500.0];
    // Test-owned trapezoid quadrature, rather than the model's midpoint rule.
    let length = 2000.0_f64.hypot(400.0);
    let mut own = 0.0;
    for i in 0..=20_000 {
        let t = f64::from(i) / 20_000.0;
        own += field.density(
            std::array::from_fn(|axis| from[axis] + (to[axis] - from[axis]) * t),
            &cells,
            Density::Coarse,
            |_| None,
            1.0,
        ) * length
            / 20_000.0
            * if i == 0 || i == 20_000 { 0.5 } else { 1.0 };
    }
    assert!(own > 0.0);
    support::close(
        w.optical_depth(from, to, &cells, 20_000, Density::Coarse, |_| None)
            .expect("cells")
            .value,
        own,
        2e-6,
    );
}

#[test]
fn identical_footprints_do_not_make_identical_visibility() {
    let make = |layers: [f64; 4]| {
        let mut v = Vec::new();
        for _ in 0..4 {
            for &layer in &layers {
                for _ in 0..4 {
                    v.push(layer);
                }
            }
        }
        Shape::new([4; 3], 10.0, v).expect("shape")
    };
    let shapes = [make([1.0, 1.0, 0.0, 1.0]), make([1.0, 0.0, 1.0, 1.0])];
    assert_eq!(shapes[0].footprint(), shapes[1].footprint());
    let field = support::field(5, 0.0);
    let w = Weather {
        field: &field,
        surface: None,
        ground: None,
    };
    let mut exact = Vec::new();
    let mut footprints = Vec::new();
    for (i, shape) in shapes.iter().enumerate() {
        let state = support::formed(shape);
        let p = [0.0, 715.0, 0.0];
        let a = w.extinction(p, &state, |_| Some(shape));
        assert_eq!(a.fidelity, Fidelity::Exact);
        // Test's own layer reading: the second slab is full only in shape zero.
        support::close(a.value, if i == 0 { 0.05 } else { 0.0 }, 1e-15);
        let tau = w
            .optical_depth(
                [-10.0, 715.0, 0.0],
                [10.0, 715.0, 0.0],
                &state,
                2000,
                Density::Coarse,
                |_| Some(shape),
            )
            .expect("voxel sight line");
        support::close(tau.value, if i == 0 { 1.0 } else { 0.0 }, 1e-12);
        exact.push(tau.value);
        let approx = w.extinction(p, &state, |_| None);
        assert_eq!(approx.fidelity, Fidelity::Footprint);
        let closed = 1.5 / 40.0 / 0.75;
        support::close(approx.value, closed, 1e-14);
        let approx_tau = w
            .optical_depth(
                [-10.0, 715.0, 0.0],
                [10.0, 715.0, 0.0],
                &state,
                2000,
                Density::Coarse,
                |_| None,
            )
            .expect("footprint sight line");
        assert_eq!(approx_tau.fidelity, Fidelity::Footprint);
        support::close(approx_tau.value, 20.0 * closed, 1e-12);
        footprints.push(approx_tau.value);
        for p in [[0.0, 699.0, 0.0], [0.0, 741.0, 0.0], [100.0, 715.0, 0.0]] {
            let a = w.extinction(p, &state, |_| None);
            assert_eq!(a.value, 0.0);
            assert_eq!(a.fidelity, Fidelity::Exact);
        }
        let vertical = w
            .optical_depth(
                [0.0, 700.0, 0.0],
                [0.0, 740.0, 0.0],
                &state,
                10_000,
                Density::Coarse,
                |_| None,
            )
            .expect("footprint integral");
        support::close(vertical.value, 30.0 * 0.05, 1e-12);
    }
    assert_ne!(exact[0], exact[1]);
    assert_eq!(footprints[0], footprints[1]);
}

#[test]
fn footprint_sum_survives_erosion_scale_rotation_and_partial_envelopes() {
    let shape = support::shape();
    for erosion in [0.0, 0.4, 1.0] {
        let field = support::field(5, erosion);
        let weather = Weather {
            field: &field,
            surface: None,
            ground: None,
        };
        for scale in [0.5, 1.0, 3.0] {
            for yaw in [0.0_f64, 0.7, 1.8] {
                for envelope in [0.25, 1.0] {
                    let mut state = support::state(0.0, 0.0);
                    let mut placed = support::directed(&shape, 1, [35.0, -17.0]);
                    placed.scale = scale;
                    placed.yaw = yaw;
                    placed.target = envelope;
                    state.add(placed).expect("placed");
                    state.step(1, 10.0).expect("form");
                    for local in [[-30.0, -20.0], [30.0, -20.0], [0.0, 10.0], [100.0, 0.0]] {
                        // Independent placement transform and footprint lookup.
                        let x = 35.0 + scale * (local[0] * yaw.cos() - local[1] * yaw.sin());
                        let z = -17.0 + scale * (local[0] * yaw.sin() + local[1] * yaw.cos());
                        let column = shape.footprint().column(local);
                        let tau = weather
                            .optical_depth(
                                [x, 700.0, z],
                                [x, 700.0 + 120.0 * scale, z],
                                &state,
                                12_000,
                                Density::Coarse,
                                |_| None,
                            )
                            .expect("footprint integral");
                        support::close(tau.value, column[0] * scale * envelope * 0.05, 2e-8);
                        assert_eq!(
                            tau.fidelity,
                            if column[0] > 0.0 {
                                Fidelity::Footprint
                            } else {
                                Fidelity::Exact
                            }
                        );
                        for height in [-1.0, 121.0] {
                            assert_eq!(
                                weather
                                    .extinction([x, 700.0 + height * scale, z], &state, |_| None)
                                    .value,
                                0.0
                            );
                        }
                        // Supplying voxels still uses the unchanged Field path.
                        let p = [x, 700.0 + 15.0 * scale, z];
                        assert_eq!(
                            weather
                                .extinction(p, &state, |_| Some(&shape))
                                .value
                                .to_bits(),
                            field
                                .density(p, &state, Density::Coarse, |_| Some(&shape), 1.0)
                                .to_bits()
                        );
                    }
                }
            }
        }
    }
}
