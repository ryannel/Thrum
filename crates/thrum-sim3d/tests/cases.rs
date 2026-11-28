// This fixture is also compiled by the cross-target runner.
use thrum_core::profile::Number;
use thrum_sim3d::{DOMAIN_BOUND, Shape, SweepGap, Vector3};

pub(crate) fn n(value: i32) -> Number {
    Number::from_i32(value)
}

pub(crate) fn ratio(numerator: i32, denominator: i32) -> Number {
    n(numerator) / n(denominator)
}

pub(crate) fn v(x: i32, y: i32, z: i32) -> Vector3 {
    Vector3::new(n(x), n(y), n(z))
}

pub(crate) fn x(value: Number) -> Vector3 {
    Vector3::new(value, Number::ZERO, Number::ZERO)
}

pub(crate) fn sphere(center: Vector3, radius: Number) -> Shape {
    Shape::Sphere { center, radius }
}

pub(crate) fn capsule(ends: [Vector3; 2], radius: Number) -> Shape {
    Shape::Capsule { ends, radius }
}

pub(crate) fn box_at(center: Vector3, half_edges: [Vector3; 3]) -> Shape {
    Shape::Cuboid { center, half_edges }
}

pub(crate) fn cube(center: Vector3, half: Number) -> Shape {
    box_at(center, Vector3::AXES.map(|axis| axis * half))
}

pub(crate) fn triangle(corners: [Vector3; 3]) -> Shape {
    Shape::Triangle { corners }
}

pub(crate) fn face() -> Shape {
    triangle([v(0, -4, -4), v(0, 4, -4), v(0, 0, 4)])
}

pub(crate) fn turned_box() -> Shape {
    box_at(Vector3::ZERO, [v(2, 2, 1), v(2, -1, -2), v(-1, 2, -2)])
}

pub(crate) const GAPS: [SweepGap; 2] = [SweepGap::Tolerance, SweepGap::HalfTolerance];
pub(crate) const STOP: Vector3 = Vector3::ZERO;
pub(crate) const FORWARD: Vector3 = Vector3::AXES[0];

#[derive(Clone, Copy)]
pub(crate) struct Frame {
    pub(crate) name: &'static str,
    signs: [i32; 3],
    offset: Vector3,
}

impl Frame {
    pub(crate) fn vector(self, value: Vector3) -> Vector3 {
        Vector3::new(
            value.x * n(self.signs[0]),
            value.y * n(self.signs[1]),
            value.z * n(self.signs[2]),
        )
    }

    pub(crate) fn point(self, point: Vector3) -> Vector3 {
        self.offset + self.vector(point)
    }

    pub(crate) fn shape(self, shape: Shape) -> Shape {
        match shape {
            Shape::Sphere { center, radius } => sphere(self.point(center), radius),
            Shape::Capsule { ends, radius } => capsule(ends.map(|point| self.point(point)), radius),
            Shape::Cuboid { center, half_edges } => {
                box_at(self.point(center), half_edges.map(|edge| self.vector(edge)))
            }
            Shape::Triangle { corners } => triangle(corners.map(|point| self.point(point))),
        }
    }
}

pub(crate) fn frames() -> [Frame; 9] {
    let corner = DOMAIN_BOUND - 48;
    [
        Frame {
            name: "origin",
            signs: [1; 3],
            offset: STOP,
        },
        Frame {
            name: "ppp",
            signs: [1, 1, 1],
            offset: v(corner, corner, corner),
        },
        Frame {
            name: "ppn",
            signs: [1, 1, -1],
            offset: v(corner, corner, -corner),
        },
        Frame {
            name: "pnp",
            signs: [1, -1, 1],
            offset: v(corner, -corner, corner),
        },
        Frame {
            name: "pnn",
            signs: [1, -1, -1],
            offset: v(corner, -corner, -corner),
        },
        Frame {
            name: "npp",
            signs: [-1, 1, 1],
            offset: v(-corner, corner, corner),
        },
        Frame {
            name: "npn",
            signs: [-1, 1, -1],
            offset: v(-corner, corner, -corner),
        },
        Frame {
            name: "nnp",
            signs: [-1, -1, 1],
            offset: v(-corner, -corner, corner),
        },
        Frame {
            name: "nnn",
            signs: [-1, -1, -1],
            offset: v(-corner, -corner, -corner),
        },
    ]
}

pub(crate) fn canonical_shapes() -> [Shape; 4] {
    [
        sphere(STOP, n(1)),
        capsule([v(0, -2, 0), v(0, 2, 0)], n(1)),
        box_at(STOP, [v(1, 0, 0), v(0, 2, 0), v(0, 0, 2)]),
        face(),
    ]
}

pub(crate) struct PairCase {
    pub(crate) id: String,
    pub(crate) mover: Shape,
    pub(crate) struck: Shape,
    pub(crate) gap: Number,
}

pub(crate) fn overlap_cases() -> Vec<PairCase> {
    let shapes = canonical_shapes();
    let mut cases = Vec::new();
    for (a, mover) in shapes[..3].iter().copied().enumerate() {
        for (b, struck) in shapes.iter().copied().enumerate().skip(a) {
            let contact = if b == 3 { n(1) } else { n(2) };
            for (label, separation, gap) in [
                ("apart", contact + n(1), n(1)),
                ("cross", contact - ratio(1, 2), n(0)),
                ("touch", contact, n(0)),
            ] {
                cases.push(PairCase {
                    id: format!("{a}-{b}-{label}"),
                    mover: mover.translated(x(-separation)),
                    struck,
                    gap,
                });
            }
            if b < 3 {
                let small = [
                    sphere(STOP, ratio(1, 4)),
                    capsule(
                        [v(0, -1, 0) * ratio(1, 4), v(0, 1, 0) * ratio(1, 4)],
                        ratio(1, 4),
                    ),
                    cube(STOP, ratio(1, 4)),
                ];
                cases.push(PairCase {
                    id: format!("{a}-{b}-inside"),
                    mover: small[a],
                    struck,
                    gap: n(0),
                });
                if a != b {
                    cases.push(PairCase {
                        id: format!("{a}-{b}-contains"),
                        mover: small[b],
                        struck: mover,
                        gap: n(0),
                    });
                }
            } else {
                cases.push(PairCase {
                    id: format!("{a}-{b}-contains-triangle"),
                    mover: match a {
                        0 => sphere(STOP, n(8)),
                        1 => capsule([v(0, -2, 0), v(0, 2, 0)], n(8)),
                        _ => cube(STOP, n(8)),
                    },
                    struck,
                    gap: n(0),
                });
            }
        }
    }

    let degenerates = [
        sphere(STOP, n(0)),
        capsule([STOP; 2], n(1)),
        box_at(STOP, [STOP, v(0, 2, 0), v(0, 0, 2)]),
        turned_box(),
        triangle([v(0, -1, 0), STOP, v(0, 1, 0)]),
    ];
    // A sphere of radius three reaches the turned box along its first half-edge.
    let axis = v(2, 2, 1);
    for (index, (struck, touching, beyond, inside)) in [
        (degenerates[0], FORWARD, v(2, 0, 0), STOP),
        (degenerates[1], v(2, 0, 0), v(3, 0, 0), STOP),
        (degenerates[2], FORWARD, v(2, 0, 0), STOP),
        (degenerates[3], axis * n(2), axis * n(3), STOP),
        (degenerates[4], FORWARD, v(2, 0, 0), STOP),
    ]
    .into_iter()
    .enumerate()
    {
        let radius = if index == 3 { n(3) } else { n(1) };
        let away = if index == 3 { n(3) } else { n(1) };
        for (label, center, gap) in [
            ("apart", beyond, away),
            ("touch", touching, n(0)),
            ("cross", inside, n(0)),
        ] {
            cases.push(PairCase {
                id: format!("degenerate-{index}-{label}"),
                mover: sphere(center, radius),
                struck,
                gap,
            });
        }
    }
    for (index, axis) in [v(1, 3, 11), v(1, 1, 3), v(3, -7, 5)]
        .into_iter()
        .enumerate()
    {
        for (label, center, gap) in [
            ("inside", STOP, n(0)),
            ("endpoint", axis * n(4), n(0)),
            ("beyond", axis * ratio(17, 4), axis.length() / n(4)),
        ] {
            cases.push(PairCase {
                id: format!("collinear-triangle-{index}-{label}"),
                mover: sphere(center, n(0)),
                struck: triangle([-axis, axis * n(2), axis * n(4)]),
                gap,
            });
        }
    }
    cases
}

pub(crate) struct RayCase {
    pub(crate) id: String,
    pub(crate) start: Vector3,
    pub(crate) motion: Vector3,
    pub(crate) struck: Shape,
    pub(crate) expected: Option<(Number, Vector3, Vector3, Number)>,
}

pub(crate) fn ray_cases() -> Vec<RayCase> {
    let [ball, capsule, cuboid, face] = canonical_shapes();
    let mut cases = Vec::new();
    let turned_axis = v(2, 2, 1);
    for (kind, shape, axis, scale, extent) in [
        ("sphere", ball, FORWARD, n(1), n(1)),
        ("capsule", capsule, FORWARD, n(1), n(1)),
        ("cuboid", cuboid, FORWARD, n(1), n(1)),
        ("turned", turned_box(), turned_axis, n(3), n(1)),
    ] {
        let start = axis * n(5);
        let depth = scale * ratio(1, 2);
        let inside = axis * ratio(1, 2);
        let expected_outside = Some((ratio(1, 2), axis * extent, axis / scale, n(0)));
        let expected_inside = Some((n(0), inside, axis / scale, depth));
        for (case, ray_start, motion, expected) in [
            ("hit", start, axis * n(-8), expected_outside),
            ("inside", inside, -axis, expected_inside),
            ("short", start, axis * n(-3), None),
            ("away", start, axis * n(8), None),
            ("beside", start + v(0, 0, 12), axis * n(-8), None),
            ("still-in", inside, STOP, expected_inside),
            ("still-out", start, STOP, None),
        ] {
            cases.push(RayCase {
                id: format!("{kind}-{case}"),
                start: ray_start,
                motion,
                struck: shape,
                expected,
            });
        }
    }
    let through_point = v(1, 1, 7);
    for (id, shape) in [
        ("point-sphere-tilted-ray", sphere(STOP, n(0))),
        (
            "point-capsule-tilted-ray",
            Shape::Capsule {
                ends: [STOP; 2],
                radius: n(0),
            },
        ),
    ] {
        cases.push(RayCase {
            id: id.into(),
            start: -through_point,
            motion: through_point * n(2),
            struck: shape,
            expected: Some((
                ratio(1, 2),
                STOP,
                -through_point / through_point.length(),
                n(0),
            )),
        });
    }
    for sign in [-1, 1] {
        let start = x(n(2 * sign));
        cases.push(RayCase {
            id: format!("triangle-side-{sign}"),
            start,
            motion: -start * n(2),
            struck: face,
            expected: Some((ratio(1, 2), STOP, x(n(sign)), n(0))),
        });
    }
    let line = v(1, 1, 11);
    let arm = v(3, -2, 1);
    let tilted = triangle([-line - arm, line - arm, arm * n(2)]);
    let degenerate = v(1, 1, 3);
    let tilted_normal = v(23, 32, -5);
    let unit_normal = tilted_normal / tilted_normal.length();
    cases.push(RayCase {
        id: "triangle-tilted-transverse".into(),
        start: tilted_normal,
        motion: tilted_normal * n(-2),
        struck: tilted,
        expected: Some((ratio(1, 2), STOP, unit_normal, n(0))),
    });
    for (id, start, motion, struck) in [
        ("triangle-edge-on", v(0, -8, 0), v(0, 16, 0), face),
        (
            "triangle-no-area",
            FORWARD,
            v(-2, 0, 0),
            triangle([v(0, -1, 0), STOP, v(0, 1, 0)]),
        ),
        ("triangle-short", v(-4, 0, 0), v(3, 0, 0), face),
        ("triangle-away", v(-4, 0, 0), v(-3, 0, 0), face),
        ("triangle-beside", v(-4, 6, 0), v(8, 0, 0), face),
        ("triangle-tilted-edge-on", -line * n(3), line * n(6), tilted),
        (
            "triangle-tilted-parallel",
            -line * n(3) + FORWARD,
            line * n(6),
            tilted,
        ),
        (
            "triangle-tilted-no-area",
            -FORWARD,
            FORWARD * n(2),
            triangle([-degenerate, degenerate * n(2), degenerate * n(4)]),
        ),
        ("triangle-still", STOP, STOP, face),
    ] {
        cases.push(RayCase {
            id: id.into(),
            start,
            motion,
            struck,
            expected: None,
        });
    }
    cases
}

pub(crate) fn sweep_movers() -> [Shape; 3] {
    [
        sphere(STOP, n(1)),
        capsule([x(ratio(-1, 2)), x(ratio(1, 2))], ratio(1, 2)),
        cube(STOP, n(1)),
    ]
}

#[derive(Clone, Copy)]
pub(crate) struct SweepExpected {
    pub(crate) fraction: Number,
    pub(crate) normal: Vector3,
    pub(crate) point: Option<Vector3>,
    pub(crate) precise: bool,
}

pub(crate) struct SweepCase {
    pub(crate) id: String,
    pub(crate) mover: Shape,
    pub(crate) motion: Vector3,
    pub(crate) struck: Shape,
    pub(crate) struck_motion: Vector3,
    pub(crate) expected: Option<SweepExpected>,
}

pub(crate) fn sweep_cases() -> Vec<SweepCase> {
    let mut cases = Vec::new();
    for (a, mover) in sweep_movers().into_iter().enumerate() {
        for (b, struck) in canonical_shapes().into_iter().enumerate() {
            let face_x = if b == 3 { 0 } else { -1 };
            let fraction = ratio(7 + face_x, 16);
            let contact_point = if a == 2 && b != 0 {
                None
            } else {
                Some(v(face_x, 0, 0))
            };
            let mover_y = if a == 1 { ratio(1, 2) } else { n(1) };
            let struck_y = n([1, 3, 2, 4][b]);
            let passing = Vector3::new(
                n(-8),
                struck_y + mover_y + SweepGap::Tolerance.units() * n(2),
                n(if b == 3 { -4 } else { 0 }),
            );
            for (label, start, motion, expected) in [
                (
                    "head",
                    v(-8, 0, 0),
                    v(16, 0, 0),
                    Some(SweepExpected {
                        fraction,
                        normal: -FORWARD,
                        point: contact_point,
                        precise: true,
                    }),
                ),
                (
                    "slant",
                    Vector3::new(n(-8), -fraction * n(4), n(0)),
                    v(16, 4, 0),
                    Some(SweepExpected {
                        fraction,
                        normal: -FORWARD,
                        point: contact_point,
                        precise: b >= 2,
                    }),
                ),
                ("pass", passing, v(16, 0, 0), None),
                ("leave", v(-8, 0, 0), v(-12, 0, 0), None),
            ] {
                cases.push(SweepCase {
                    id: format!("{a}-{b}-{label}"),
                    mover: mover.translated(start),
                    motion,
                    struck,
                    struck_motion: STOP,
                    expected,
                });
            }
            if b == 3 {
                cases.push(SweepCase {
                    id: format!("{a}-{b}-back"),
                    mover: mover.translated(v(8, 0, 0)),
                    motion: v(-16, 0, 0),
                    struck,
                    struck_motion: STOP,
                    expected: Some(SweepExpected {
                        fraction,
                        normal: FORWARD,
                        point: (a < 2).then_some(STOP),
                        precise: true,
                    }),
                });
            }
        }
    }
    let edge_triangle = triangle([STOP, v(0, 4, 0), v(0, 0, 4)]);
    for (obstacle, struck) in [("box", cube(STOP, n(1))), ("triangle", edge_triangle)] {
        for (feature, raw, magnitude, point) in [
            (
                "edge",
                v(-3, -4, 0),
                n(5),
                if obstacle == "box" {
                    v(-1, -1, 0)
                } else {
                    v(0, 0, 1)
                },
            ),
            (
                "corner",
                v(-2, -3, -6),
                n(7),
                if obstacle == "box" {
                    v(-1, -1, -1)
                } else {
                    STOP
                },
            ),
        ] {
            // Integer offsets along a 3:4 or 2:3:6 normal give exact touching coordinates.
            let direction = raw / magnitude;
            let start = point + raw * n(3);
            let ball = sphere(start, magnitude);
            let cap = capsule([start, start + raw], magnitude);
            for (kind, mover) in [("sphere", ball), ("capsule", cap)] {
                cases.push(SweepCase {
                    id: format!("{kind}-{obstacle}-{feature}"),
                    mover,
                    motion: raw * n(-4),
                    struck,
                    struck_motion: STOP,
                    expected: Some(SweepExpected {
                        fraction: ratio(1, 2),
                        normal: direction,
                        point: Some(point),
                        precise: true,
                    }),
                });
            }
            // Independent constraints first bind simultaneously for two touching
            // box faces, so their diagonal is the limiting separation direction.
            let offset = v(-1, -1, if feature == "corner" { -1 } else { 0 });
            let start = point + offset * n(9);
            cases.push(SweepCase {
                id: format!("box-{obstacle}-{feature}"),
                mover: cube(start, n(1)),
                motion: offset * n(-16),
                struck,
                struck_motion: STOP,
                expected: Some(SweepExpected {
                    fraction: ratio(1, 2),
                    normal: offset / offset.length(),
                    point: (feature == "corner").then_some(point),
                    precise: true,
                }),
            });
        }
    }

    cases.push(SweepCase {
        id: "through-thick-cuboid".into(),
        mover: sphere(v(-10, 0, 0), n(1)),
        motion: v(20, 0, 0),
        struck: cube(STOP, n(3)),
        struck_motion: STOP,
        expected: Some(SweepExpected {
            fraction: ratio(3, 10),
            normal: -FORWARD,
            point: Some(v(-3, 0, 0)),
            precise: true,
        }),
    });
    for (id, struck) in [
        ("through-thin-triangle", face()),
        (
            "through-flat-box",
            box_at(STOP, [STOP, v(0, 2, 0), v(0, 0, 2)]),
        ),
    ] {
        let radius = ratio(1, 16);
        cases.push(SweepCase {
            id: id.into(),
            mover: sphere(v(-1249, 0, 0), radius),
            motion: v(1250, 0, 0),
            struck,
            struck_motion: STOP,
            expected: Some(SweepExpected {
                fraction: (n(1249) - radius) / n(1250),
                normal: -FORWARD,
                point: Some(STOP),
                precise: true,
            }),
        });
    }
    cases
}

pub(crate) struct DepthCase {
    pub(crate) id: &'static str,
    pub(crate) mover: Shape,
    pub(crate) struck: Shape,
    pub(crate) depth: Number,
    pub(crate) normal: Option<Vector3>,
    pub(crate) perpendicular_to: Vec<Vector3>,
}

pub(crate) fn depth_cases() -> Vec<DepthCase> {
    let half = ratio(1, 2);
    let ball = sphere(STOP, n(1));
    let big = cube(STOP, n(4));
    let near_face = x(n(3) + half);
    let mut cases = Vec::new();
    for (id, mover, struck, depth, normal) in [
        ("sphere-sphere", sphere(FORWARD, n(1)), ball, n(1), FORWARD),
        (
            "radii-cross",
            sphere(v(2, 0, 0), n(2)),
            capsule([v(0, -2, 0), v(0, 2, 0)], n(1)),
            n(1),
            FORWARD,
        ),
        ("cuboid-cuboid", cube(v(4, 0, 0), n(1)), big, n(1), FORWARD),
        (
            "capsule-cuboid",
            capsule([near_face + v(0, -6, 0), near_face + v(0, 6, 0)], half),
            big,
            n(1),
            FORWARD,
        ),
        (
            "sphere-cuboid",
            sphere(near_face, n(1)),
            big,
            n(1) + half,
            FORWARD,
        ),
        ("point-cuboid", sphere(near_face, n(0)), big, half, FORWARD),
        (
            "capsule-triangle",
            capsule([v(-1, 0, 0), v(3, 0, 0)], half),
            face(),
            n(1) + half,
            FORWARD,
        ),
        (
            "cuboid-triangle",
            cube(x(half), n(1)),
            face(),
            half,
            FORWARD,
        ),
        (
            "contained",
            cube(v(2, 0, 0), half),
            big,
            n(2) + half,
            FORWARD,
        ),
        ("separate", sphere(v(6, 0, 0), n(1)), big, n(0), STOP),
    ] {
        cases.push(DepthCase {
            id,
            mover,
            struck,
            depth,
            normal: Some(normal),
            perpendicular_to: Vec::new(),
        });
    }
    let flat = box_at(STOP, [v(3, 0, 0), v(0, 3, 0), STOP]);
    cases.push(DepthCase {
        id: "capsule-flat-cuboid",
        mover: capsule([v(-2, 0, 0), v(2, 0, 0)], n(1)),
        struck: flat,
        depth: n(1),
        normal: None,
        perpendicular_to: vec![FORWARD, v(0, 1, 0)],
    });
    for (id, line) in [
        ("capsule-x", v(2, 0, 0)),
        ("capsule-y", v(0, 2, 0)),
        ("capsule-z", v(0, 0, 2)),
        ("capsule-turned", v(3, 4, 0)),
    ] {
        cases.push(DepthCase {
            id,
            mover: ball,
            struck: capsule([-line, line], n(1)),
            depth: n(2),
            normal: None,
            perpendicular_to: vec![line / line.length()],
        });
    }
    cases.push(DepthCase {
        id: "collinear-capsules",
        mover: capsule([v(-3, 0, 0), v(3, 0, 0)], n(1)),
        struck: capsule([STOP, v(4, 0, 0)], n(1)),
        depth: n(2),
        normal: None,
        perpendicular_to: vec![FORWARD],
    });
    for (index, line) in [v(1, 1, 11), v(3, -7, 5)].into_iter().enumerate() {
        let ids = [
            ("skew-line-capsules", "skew-line-sphere"),
            ("reflected-line-capsules", "reflected-line-sphere"),
        ];
        for (id, mover, struck) in [
            (
                ids[index].0,
                capsule([line * n(-2), line * n(2)], n(1)),
                capsule([line * n(-3), line * n(4)], n(1)),
            ),
            (
                ids[index].1,
                ball,
                capsule([line * n(-2), line * n(3)], n(1)),
            ),
        ] {
            cases.push(DepthCase {
                id,
                mover,
                struck,
                depth: n(2),
                normal: None,
                perpendicular_to: vec![line / line.length()],
            });
        }
    }
    cases.push(DepthCase {
        id: "coincident-spheres",
        mover: ball,
        struck: ball,
        depth: n(2),
        normal: None,
        perpendicular_to: Vec::new(),
    });
    cases
}

/// Candidate start-contact cases use all twelve shape pairs at each requested gap.
pub(crate) fn start_cases(gap: SweepGap) -> Vec<SweepCase> {
    let mut cases = Vec::new();
    let tolerance = SweepGap::Tolerance.units();
    for (a, mover) in sweep_movers().into_iter().enumerate() {
        for (b, struck) in canonical_shapes().into_iter().enumerate() {
            let touch_x = if b == 3 { n(-1) } else { n(-2) };
            for (label, separation) in [
                ("close", gap.units() / n(2)),
                ("shallow", -tolerance / n(2)),
                ("clear", tolerance * n(2)),
            ] {
                for (index, motion) in [FORWARD, v(0, 1, 0), -FORWARD, STOP]
                    .into_iter()
                    .enumerate()
                {
                    if label == "clear" && index == 0 {
                        continue;
                    }
                    cases.push(SweepCase {
                        id: format!("{a}-{b}-{label}-{index}"),
                        mover: mover.translated(x(touch_x - separation)),
                        struck,
                        motion,
                        struck_motion: STOP,
                        expected: (label != "clear").then_some(SweepExpected {
                            fraction: n(0),
                            normal: -FORWARD,
                            point: None,
                            precise: true,
                        }),
                    });
                }
            }
        }
    }
    cases
}

pub(crate) fn moving_cases() -> Vec<SweepCase> {
    let ball = |at| sphere(at, ratio(1, 2));
    let mut cases = Vec::new();
    for (id, mover, motion, struck, struck_motion, expected) in [
        (
            "head",
            ball(STOP),
            v(6, 0, 0),
            ball(v(4, 0, 0)),
            v(-2, 0, 0),
            Some(SweepExpected {
                fraction: ratio(3, 8),
                normal: -FORWARD,
                point: Some(x(ratio(11, 4))),
                precise: true,
            }),
        ),
        (
            "overtake",
            ball(STOP),
            v(8, 0, 0),
            ball(v(3, 0, 0)),
            v(4, 0, 0),
            Some(SweepExpected {
                fraction: ratio(1, 2),
                normal: -FORWARD,
                point: Some(x(ratio(9, 2))),
                precise: true,
            }),
        ),
        (
            "cross",
            ball(v(-6, 0, 0)),
            v(12, 0, 0),
            ball(v(0, -8, 0)),
            v(0, 16, 0),
            Some(SweepExpected {
                fraction: ratio(9, 20),
                normal: Vector3::new(ratio(-3, 5), ratio(4, 5), n(0)),
                point: None,
                precise: false,
            }),
        ),
        (
            "slant",
            sweep_movers()[1].translated(v(-5, -2, 0)),
            v(4, 3, 0),
            cube(STOP, n(1)),
            v(-2, -1, 0),
            Some(SweepExpected {
                fraction: ratio(1, 2),
                normal: -FORWARD,
                point: Some(Vector3::new(n(-2), ratio(-1, 2), n(0))),
                precise: true,
            }),
        ),
        (
            "apart",
            ball(STOP),
            v(-3, 0, 0),
            ball(v(4, 0, 0)),
            v(2, 0, 0),
            None,
        ),
        (
            "together",
            ball(STOP),
            v(4, 0, 0),
            ball(v(2, 0, 0)),
            v(4, 0, 0),
            None,
        ),
    ] {
        cases.push(SweepCase {
            id: id.into(),
            mover,
            motion,
            struck,
            struck_motion,
            expected,
        });
    }
    cases
}

pub(crate) fn opposing_cases() -> [SweepCase; 2] {
    let diagonal = v(1, 1, 1);
    [
        ("opposing-solids", 32760, 50002, 8),
        ("opposing-points", DOMAIN_BOUND, 60000, 0),
    ]
    .map(|(id, corner, speed, radius)| {
        let extent = diagonal * n(corner);
        let motion = diagonal * n(speed);
        SweepCase {
            id: id.into(),
            mover: sphere(-extent, n(radius)),
            motion,
            struck: sphere(extent, n(radius)),
            struck_motion: -motion,
            expected: Some(SweepExpected {
                fraction: (n(corner) - n(radius) / diagonal.length()) / n(speed),
                normal: -diagonal / diagonal.length(),
                point: Some(STOP),
                precise: true,
            }),
        }
    })
}

pub(crate) fn domain_frames() -> [Frame; 8] {
    let [_, corners @ ..] = frames();
    corners.map(|frame| Frame {
        offset: STOP,
        ..frame
    })
}

pub(crate) fn depth_motions(normal: Vector3) -> [Vector3; 4] {
    let axis = if normal.x.abs() < ratio(3, 4) {
        FORWARD
    } else {
        v(0, 1, 0)
    };
    [-normal, normal.cross(axis), normal, STOP]
}
