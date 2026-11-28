use std::fmt::Write;

use thrum_core::profile::Number;
use thrum_sim3d::{Shape, SweepGap, Vector3, depth};

use crate::cases::{
    Frame, GAPS, STOP, SweepCase, depth_cases, depth_motions, frames, moving_cases, overlap_cases,
    ratio, ray_cases, start_cases, sweep_cases, v,
};

#[test]
fn hand_worked_inputs() {
    for frame in frames() {
        for case in overlap_cases() {
            let mover = frame.shape(case.mover);
            let struck = frame.shape(case.struck);
            for (order, first, second) in [("ab", mover, struck), ("ba", struck, mover)] {
                for query in ["distance", "overlap"] {
                    emit(frame, &case.id, &format!("{query}-{order}"), query, |row| {
                        pair(row, first, second)
                    });
                }
            }
        }
        for case in ray_cases() {
            emit(frame, &case.id, "ray", "ray", |row| {
                vector(row, frame.point(case.start));
                vector(row, frame.vector(case.motion));
                shape(row, frame.shape(case.struck));
            });
        }
        for case in sweep_cases() {
            for gap in GAPS {
                cast(frame, "sweep", &case, gap);
            }
        }
        for case in depth_cases() {
            let first = frame.shape(case.mover);
            let second = frame.shape(case.struck);
            emit(frame, case.id, "depth", "depth", |row| {
                pair(row, first, second)
            });
            if case.depth == Number::ZERO {
                continue;
            }
            let hit = depth(first, second).expect("hand-worked shapes are admitted");
            for (operation, moved) in [
                ("parted", first.translated(hit.normal * hit.depth)),
                ("along-tie", first.translated(hit.normal * ratio(1, 2))),
                ("against-tie", first.translated(hit.normal * ratio(-1, 2))),
            ] {
                if operation == "parted" {
                    emit(frame, case.id, "parted-distance", "distance", |row| {
                        pair(row, moved, second)
                    });
                }
                if operation == "parted" || case.id == "coincident-spheres" {
                    emit(frame, case.id, operation, "depth", |row| {
                        pair(row, moved, second)
                    });
                }
            }
            for gap in GAPS {
                for (index, motion) in depth_motions(hit.normal).into_iter().enumerate() {
                    let id = format!("{}-deep-{index}", case.id);
                    emit(frame, &id, gap_name(gap), "sweep", |row| {
                        sweep_inputs(row, gap, first, motion, second, STOP);
                    });
                }
            }
        }

        for gap in GAPS {
            for case in start_cases(gap) {
                cast(frame, "initial", &case, gap);
            }
        }
        for case in moving_cases() {
            cast(frame, "moving", &case, SweepGap::Tolerance);
        }
    }
    let corner = thrum_sim3d::DOMAIN_BOUND;
    let corner = v(corner, corner, corner);
    let mut row = String::from("corners/domain-diagonal/sweep sweep");
    sweep_inputs(
        &mut row,
        SweepGap::Tolerance,
        crate::cases::sphere(-corner, Number::ZERO),
        corner + corner,
        crate::cases::sphere(corner, Number::ZERO),
        STOP,
    );
    println!("input {row}");
}

fn emit(frame: Frame, id: &str, suffix: &str, query: &str, inputs: impl FnOnce(&mut String)) {
    let mut row = format!("{}/{id}/{suffix} {query}", frame.name);
    inputs(&mut row);
    println!("input {row}");
}

fn gap_name(gap: SweepGap) -> &'static str {
    match gap {
        SweepGap::Tolerance => "full",
        SweepGap::HalfTolerance => "half",
    }
}

fn cast(frame: Frame, category: &str, case: &SweepCase, gap: SweepGap) {
    let id = format!("{category}-{}", case.id);
    emit(frame, &id, gap_name(gap), "sweep", |row| {
        sweep_inputs(
            row,
            gap,
            frame.shape(case.mover),
            frame.vector(case.motion),
            frame.shape(case.struck),
            frame.vector(case.struck_motion),
        );
    });
}

fn sweep_inputs(
    row: &mut String,
    gap: SweepGap,
    first: Shape,
    motion: Vector3,
    second: Shape,
    other_motion: Vector3,
) {
    write!(row, " {}", gap_name(gap)).expect("writing to a String");
    shape(row, first);
    vector(row, motion);
    shape(row, second);
    vector(row, other_motion);
}

fn pair(row: &mut String, first: Shape, second: Shape) {
    shape(row, first);
    shape(row, second);
}

fn number(row: &mut String, value: Number) {
    write!(row, " {:08x}", value.to_bits()).expect("writing to a String");
}

fn vector(row: &mut String, value: Vector3) {
    for component in [value.x, value.y, value.z] {
        number(row, component);
    }
}

fn shape(row: &mut String, shape: Shape) {
    match shape {
        Shape::Sphere { center, radius } => {
            row.push_str(" sphere");
            vector(row, center);
            number(row, radius);
        }
        Shape::Capsule { ends, radius } => {
            row.push_str(" capsule");
            for end in ends {
                vector(row, end);
            }
            number(row, radius);
        }
        Shape::Cuboid { center, half_edges } => {
            row.push_str(" cuboid");
            vector(row, center);
            for edge in half_edges {
                vector(row, edge);
            }
        }
        Shape::Triangle { corners } => {
            row.push_str(" triangle");
            for corner in corners {
                vector(row, corner);
            }
        }
    }
}
