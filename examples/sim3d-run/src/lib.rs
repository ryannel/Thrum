//! Cross-target, bit-for-bit conformance for the 3D collision pair tests.

mod inputs;
mod seeded;

use std::fmt::Write;

use thrum_core::profile::Number;
use thrum_sim3d::{Shape, SweepGap, Vector3, depth, distance, overlap, ray, sweep_two};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

enum Query {
    Distance(Shape, Shape),
    Overlap(Shape, Shape),
    Depth(Shape, Shape),
    Ray(Vector3, Vector3, Shape),
    Sweep(SweepGap, Shape, Vector3, Shape, Vector3),
}

impl Query {
    fn write_answer(self, output: &mut String) {
        match self {
            Self::Distance(a, b) => {
                output.push_str(" distance");
                number(output, distance(a, b).expect("valid fixture"));
            }
            Self::Overlap(a, b) => {
                let answer = overlap(a, b).expect("valid fixture");
                write!(output, " overlap {}", u8::from(answer)).expect("writing to a String");
            }
            Self::Depth(a, b) => {
                let answer = depth(a, b).expect("valid fixture");
                output.push_str(" depth");
                number(output, answer.depth);
                vector(output, answer.normal);
            }
            Self::Ray(start, motion, shape) => {
                output.push_str(" ray");
                if let Some(hit) = ray(start, motion, shape).expect("valid fixture") {
                    output.push_str(" hit");
                    number(output, hit.fraction);
                    vector(output, hit.point);
                    vector(output, hit.normal);
                    number(output, hit.depth);
                } else {
                    output.push_str(" miss");
                }
            }
            Self::Sweep(gap, a, motion, b, struck_motion) => {
                output.push_str(" sweep");
                if let Some(hit) =
                    sweep_two(a, motion, b, struck_motion, gap).expect("valid fixture")
                {
                    output.push_str(" hit");
                    number(output, hit.fraction);
                    vector(output, hit.point);
                    vector(output, hit.normal);
                    number(output, hit.depth);
                } else {
                    output.push_str(" miss");
                }
            }
        }
        output.push('\n');
    }
}

fn number(output: &mut String, value: Number) {
    write!(output, " {:08x}", value.to_bits()).expect("writing to a String");
}

fn vector(output: &mut String, value: Vector3) {
    for component in [value.x, value.y, value.z] {
        number(output, component);
    }
}

/// Return named vectors with every result word in hexadecimal.
pub fn vector_rows() -> String {
    let mut output = String::new();
    inputs::append_hand_worked(&mut output);
    seeded::append_seeded(&mut output);
    output
}

/// Execute the vectors inside the browser and return the browser driver's JSON packet.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn vectors() -> String {
    serde_json::to_string(&vector_rows()).expect("a string encodes to JSON")
}
