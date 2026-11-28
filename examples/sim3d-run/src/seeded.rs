use std::fmt::Write;

use thrum_core::profile::{Number, random_fraction, random_u64};
use thrum_sim3d::{Shape, SweepGap, Vector3};

use crate::Query;

const SEED: u64 = 0x7157_3549_5033_4401;

pub(crate) fn append_seeded(output: &mut String) {
    for case in 0..1_000 {
        let mut draw = Draw { case, stream: 0 };
        let center = draw.vector(32_700);
        let offset = draw.vector(7);
        let a = draw.shape(center, 3);
        let b = draw.shape(center + offset, 4);
        let axis = draw.axis();
        let motion = axis * n(24);
        let shot = center - axis * n(12);
        let mover = a.translated(shot - center);
        let struck_motion = if case % 2 == 0 {
            draw.vector(6)
        } else {
            Vector3::ZERO
        };
        let gap = if case % 2 == 0 {
            SweepGap::Tolerance
        } else {
            SweepGap::HalfTolerance
        };
        for (query, operation) in [
            ("distance", Query::Distance(a, b)),
            ("overlap", Query::Overlap(a, b)),
            ("depth", Query::Depth(a, b)),
            ("ray", Query::Ray(shot, motion, b)),
            ("sweep", Query::Sweep(gap, mover, motion, b, struck_motion)),
        ] {
            write!(output, "seeded/{case:04}/{query}").expect("writing to a String");
            operation.write_answer(output);
        }
    }
}

struct Draw {
    case: u64,
    stream: u32,
}

impl Draw {
    fn next(&mut self) -> u64 {
        let value = random_u64(SEED, self.case, u128::from(self.case), self.stream);
        self.stream += 1;
        value
    }

    fn vector(&mut self, extent: i32) -> Vector3 {
        let mut coordinate = || (random_fraction(self.next()) * n(2) - n(1)) * n(extent);
        Vector3::new(coordinate(), coordinate(), coordinate())
    }

    fn axis(&mut self) -> Vector3 {
        let axis = Vector3::AXES[usize::try_from(self.next() % 3).expect("three axes")];
        if self.next() % 2 == 0 { axis } else { -axis }
    }

    fn shape(&mut self, center: Vector3, kinds: u64) -> Shape {
        let radius = random_fraction(self.next()) * n(3);
        match self.next() % kinds {
            0 => Shape::Sphere { center, radius },
            1 => {
                let edge = self.vector(3);
                Shape::Capsule {
                    ends: [center - edge, center + edge],
                    radius,
                }
            }
            2 => {
                let half_edges = if self.next() % 2 == 0 {
                    Vector3::AXES.map(|axis| axis * (random_fraction(self.next()) * n(3)))
                } else {
                    [v(2, 2, 1), v(2, -1, -2), v(-1, 2, -2)].map(|axis| {
                        axis * n(
                            i32::try_from(self.next() % 3 + 1).expect("factor from one to three")
                        ) / n(2)
                    })
                };
                Shape::Cuboid { center, half_edges }
            }
            _ => Shape::Triangle {
                corners: [center, center + self.vector(4), center + self.vector(4)],
            },
        }
    }
}

fn n(value: i32) -> Number {
    Number::from_i32(value)
}

fn v(x: i32, y: i32, z: i32) -> Vector3 {
    Vector3::new(n(x), n(y), n(z))
}
