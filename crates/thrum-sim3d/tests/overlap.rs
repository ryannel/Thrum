use thrum_core::profile::Number;
use thrum_sim3d::{distance, overlap};

use crate::cases::{frames, overlap_cases, ratio};

#[test]
fn hand_worked_distances_and_overlaps_in_both_orders() {
    for frame in frames() {
        for case in overlap_cases() {
            let a = frame.shape(case.mover);
            let b = frame.shape(case.struck);
            for (first, second) in [(a, b), (b, a)] {
                let gap = distance(first, second).unwrap();
                let label = format!("{} {}", frame.name, case.id);
                if case.gap == Number::ZERO {
                    assert_eq!(gap, Number::ZERO, "{label}, {first:?} {second:?}");
                } else {
                    assert!(
                        (gap - case.gap).abs() <= ratio(1, 1 << 16),
                        "{label}: {gap:?} != {:?}",
                        case.gap
                    );
                }
                let intersects = overlap(first, second).unwrap();
                assert_eq!(intersects, case.gap == Number::ZERO, "{label}");
                assert_eq!(intersects, gap == Number::ZERO, "{label}");
            }
        }
    }
}
