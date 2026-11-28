use thrum_core::profile::Number;
use thrum_sim3d::{DOMAIN_BOUND, SweepGap, SweepHit, distance, overlap, sweep, sweep_two};

use crate::cases::{Frame, GAPS, STOP, SweepCase, frames, n, ratio, sphere, sweep_cases, v};

pub(crate) fn check_hit(frame: Frame, case: &SweepCase, gap: SweepGap, hit: Option<SweepHit>) {
    let label = format!("{} {} {gap:?}", frame.name, case.id);
    let Some(expected) = case.expected else {
        assert!(hit.is_none(), "{label}: {hit:?}");
        return;
    };
    let hit = hit.unwrap_or_else(|| panic!("missed {label}"));
    let first = frame
        .shape(case.mover)
        .translated(frame.vector(case.motion) * hit.fraction);
    let second = frame
        .shape(case.struck)
        .translated(frame.vector(case.struck_motion) * hit.fraction);
    let separation = distance(first, second).unwrap();
    assert!(!overlap(first, second).unwrap(), "{label}: {hit:?}");
    assert!(
        separation <= gap.units(),
        "{label}: gap {separation:?} in {hit:?}"
    );
    assert!(
        hit.fraction <= expected.fraction + ratio(1, 1 << 20),
        "{label}: late {hit:?}"
    );
    assert_eq!(hit.depth, Number::ZERO, "{label}");
    assert!(
        (hit.normal.length() - n(1)).abs() <= ratio(1, 1 << 20),
        "{label}: {hit:?}"
    );
    let expected_normal = frame.vector(expected.normal);
    assert!(
        hit.normal.dot(expected_normal) >= ratio(3, 4),
        "{label}: {hit:?} vs {expected_normal:?}"
    );
    if expected.precise {
        let error = hit.normal - expected_normal;
        assert!(
            [error.x, error.y, error.z]
                .into_iter()
                .all(|component| component.abs() <= ratio(1, 1 << 10)),
            "{label}: {hit:?} vs {expected_normal:?}"
        );
    }

    let tolerance = SweepGap::Tolerance.units();
    let point = sphere(hit.point, Number::ZERO);
    assert!(
        distance(point, second).unwrap() <= tolerance,
        "{label}: {hit:?}"
    );
    assert!(
        distance(point, first).unwrap() <= separation + tolerance,
        "{label}: {hit:?}"
    );
    if let Some(point) = expected.point {
        assert!(
            (hit.point - frame.point(point)).length() <= tolerance,
            "{label}: {hit:?} vs {:?}",
            frame.point(point)
        );
    }
}

#[test]
fn hand_worked_contacts_on_the_way() {
    let cases = sweep_cases();
    for frame in frames() {
        for case in &cases {
            let mover = frame.shape(case.mover);
            let struck = frame.shape(case.struck);
            assert!(
                distance(mover, struck).unwrap() > SweepGap::Tolerance.units(),
                "{} {}",
                frame.name,
                case.id
            );
            for gap in GAPS {
                let hit = sweep(mover, frame.vector(case.motion), struck, gap).unwrap();
                check_hit(frame, case, gap, hit);
            }
        }
    }
}

/// Measure the engine's reported gaps with candidate tolerances before retaining
/// the constant. Every row records the hit or the absence of one.
#[test]
fn measure_candidate_gaps() {
    let mut failures = 0;
    let mut maxima = [Number::ZERO; 2];
    for frame in frames() {
        for case in sweep_cases() {
            let first = frame.shape(case.mover);
            let second = frame.shape(case.struck);
            let motion = frame.vector(case.motion);
            for (index, gap) in GAPS.into_iter().enumerate() {
                let hit = sweep(first, motion, second, gap).unwrap();
                let separation = hit
                    .map(|hit| distance(first.translated(motion * hit.fraction), second).unwrap());
                let passes = match separation {
                    Some(value) => {
                        maxima[index] = maxima[index].max(value);
                        case.expected.is_some() && value > Number::ZERO && value <= gap.units()
                    }
                    None => case.expected.is_none(),
                };
                failures += i32::from(!passes);
                println!(
                    "{} {} {gap:?} gap={separation:?} pass={passes}",
                    frame.name, case.id
                );
            }
        }
    }
    let corner = v(DOMAIN_BOUND, DOMAIN_BOUND, DOMAIN_BOUND);
    let first = sphere(-corner, Number::ZERO);
    let second = sphere(corner, Number::ZERO);
    let motion = corner * n(2);
    let hit = sweep_two(first, motion, second, STOP, SweepGap::Tolerance).unwrap();
    let separation =
        hit.map(|hit| distance(first.translated(motion * hit.fraction), second).unwrap());
    let passes = separation
        .is_some_and(|value| value > Number::ZERO && value <= SweepGap::Tolerance.units());
    if let Some(value) = separation {
        maxima[0] = maxima[0].max(value);
    }
    failures += i32::from(!passes);
    println!("domain-diagonal Tolerance gap={separation:?} pass={passes}");
    println!(
        "maximum gap full={:?} half={:?}, failures={failures}",
        maxima[0], maxima[1]
    );
    assert_eq!(failures, 0);
}
