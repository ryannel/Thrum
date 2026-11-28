use thrum_core::profile::Number;
use thrum_sim3d::{SweepGap, depth, distance, sweep};

use crate::cases::{GAPS, STOP, depth_cases, frames, n, ratio, start_cases, v};

#[test]
fn hand_worked_depths_and_deep_initial_contacts() {
    for frame in frames() {
        for case in depth_cases() {
            let label = format!("{} {}", frame.name, case.id);
            let first = frame.shape(case.mover);
            let second = frame.shape(case.struck);
            let hit = depth(first, second).unwrap();
            if let Some(normal) = case.normal {
                assert!(
                    (hit.depth - case.depth).abs() <= ratio(1, 1 << 16),
                    "{label}: {hit:?}"
                );
                assert!(
                    (hit.normal - frame.vector(normal)).length() <= ratio(1, 1 << 10),
                    "{label}: {hit:?}"
                );
            } else {
                assert_eq!(hit.depth, case.depth, "{label} {hit:?}");
                for axis in case.perpendicular_to {
                    assert!(
                        frame.vector(axis).dot(hit.normal).abs() <= ratio(1, 1 << 10),
                        "{label}: {hit:?}"
                    );
                }
            }
            if case.depth == Number::ZERO {
                continue;
            }
            assert!(
                (hit.normal.length() - n(1)).abs() <= ratio(1, 1 << 20),
                "{label}: {hit:?}"
            );
            let parted = first.translated(hit.normal * hit.depth);
            let tolerance = SweepGap::Tolerance.units();
            assert!(
                distance(parted, second).unwrap() <= tolerance,
                "{label}: {hit:?}"
            );
            assert!(
                depth(parted, second).unwrap().depth <= tolerance,
                "{label}: {hit:?}"
            );
            if case.id == "coincident-spheres" {
                for sign in [-1, 1] {
                    let shifted = first.translated(hit.normal * ratio(sign, 2));
                    assert!(
                        (depth(shifted, second).unwrap().depth - ratio(3, 2)).abs()
                            <= ratio(1, 1 << 16)
                    );
                }
            }

            let axis = if hit.normal.x.abs() < ratio(3, 4) {
                v(1, 0, 0)
            } else {
                v(0, 1, 0)
            };
            let along = hit.normal.cross(axis);
            for gap in GAPS {
                for motion in [-hit.normal, along, hit.normal, STOP] {
                    let swept = sweep(first, motion, second, gap)
                        .unwrap()
                        .unwrap_or_else(|| panic!("{label} missed"));
                    assert_eq!(swept.fraction, Number::ZERO, "{label}");
                    assert_eq!(swept.depth, hit.depth, "{label}");
                    assert_eq!(swept.normal, hit.normal, "{label}");
                }
            }
        }
    }
}

#[test]
fn shallow_initial_contacts_and_clear_starts() {
    for frame in frames() {
        for gap in GAPS {
            for case in start_cases(gap) {
                let label = format!("{} {} {gap:?}", frame.name, case.id);
                let first = frame.shape(case.mover);
                let second = frame.shape(case.struck);
                let result = sweep(first, frame.vector(case.motion), second, gap).unwrap();
                if let Some(expected) = case.expected {
                    let hit = result.unwrap_or_else(|| panic!("missed {label}"));
                    assert_eq!(hit.fraction, Number::ZERO, "{label}");
                    assert_eq!(hit.depth, Number::ZERO, "{label}");
                    assert!(
                        (hit.normal - frame.vector(expected.normal)).length() <= ratio(1, 1 << 10),
                        "{label}: {hit:?}"
                    );
                } else {
                    assert!(result.is_none(), "{label}: {result:?}");
                }
            }
        }
    }
}
