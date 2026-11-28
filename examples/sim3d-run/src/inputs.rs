use std::str::SplitAsciiWhitespace;

use thrum_core::profile::Number;
use thrum_sim3d::{Shape, SweepGap, Vector3};

use crate::Query;

const HAND_WORKED: &str = include_str!("../tests/fixtures/pair-inputs.txt");

pub(crate) fn append_hand_worked(output: &mut String) {
    for row in HAND_WORKED.lines() {
        let mut input = Inputs(row.split_ascii_whitespace());
        output.push_str(input.token());
        let query = match input.token() {
            "distance" => Query::Distance(input.shape(), input.shape()),
            "overlap" => Query::Overlap(input.shape(), input.shape()),
            "depth" => Query::Depth(input.shape(), input.shape()),
            "ray" => Query::Ray(input.vector(), input.vector(), input.shape()),
            "sweep" => {
                let gap = match input.token() {
                    "full" => SweepGap::Tolerance,
                    "half" => SweepGap::HalfTolerance,
                    other => panic!("unknown sweep gap {other}"),
                };
                Query::Sweep(
                    gap,
                    input.shape(),
                    input.vector(),
                    input.shape(),
                    input.vector(),
                )
            }
            other => panic!("unknown collision query {other}"),
        };
        assert!(input.0.next().is_none(), "trailing input: {row}");
        query.write_answer(output);
    }
}

struct Inputs<'a>(SplitAsciiWhitespace<'a>);

impl<'a> Inputs<'a> {
    fn token(&mut self) -> &'a str {
        self.0.next().expect("complete fixture row")
    }

    fn number(&mut self) -> Number {
        let bits = u32::from_str_radix(self.token(), 16).expect("hexadecimal binary32 word");
        let encoded_exponent = i32::try_from((bits >> 23) & 0xff).expect("eight exponent bits");
        assert!(encoded_exponent < 255, "fixture number must be finite");
        let mantissa = (bits & 0x007f_ffff) | (u32::from(encoded_exponent != 0) << 23);
        // All binary32 significands fit exactly as profile integers. Applying
        // the exponent in power-of-two steps also preserves signed zero.
        let mut number = Number::from_i32(i32::try_from(mantissa).expect("24-bit significand"));
        let mut exponent = encoded_exponent.max(1) - 150;
        while exponent < 0 {
            let step = (-exponent).min(30);
            number = number / Number::from_i32(1 << step);
            exponent += step;
        }
        while exponent > 0 {
            let step = exponent.min(30);
            number = number * Number::from_i32(1 << step);
            exponent -= step;
        }
        if bits >> 31 == 0 { number } else { -number }
    }

    fn vector(&mut self) -> Vector3 {
        Vector3::new(self.number(), self.number(), self.number())
    }

    fn shape(&mut self) -> Shape {
        match self.token() {
            "sphere" => Shape::Sphere {
                center: self.vector(),
                radius: self.number(),
            },
            "capsule" => Shape::Capsule {
                ends: std::array::from_fn(|_| self.vector()),
                radius: self.number(),
            },
            "cuboid" => Shape::Cuboid {
                center: self.vector(),
                half_edges: std::array::from_fn(|_| self.vector()),
            },
            "triangle" => Shape::Triangle {
                corners: std::array::from_fn(|_| self.vector()),
            },
            other => panic!("unknown shape {other}"),
        }
    }
}
