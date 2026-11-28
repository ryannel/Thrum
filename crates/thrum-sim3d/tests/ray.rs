use thrum_sim3d::{SweepGap, ray};

use crate::cases::{frames, n, ratio, ray_cases};

#[test]
fn hand_worked_rays() {
    for frame in frames() {
        for case in ray_cases() {
            let hit = ray(
                frame.point(case.start),
                frame.vector(case.motion),
                frame.shape(case.struck),
            )
            .unwrap();
            let label = format!("{} {}", frame.name, case.id);
            if let Some((fraction, point, normal, depth)) = case.expected {
                let hit = hit.unwrap_or_else(|| panic!("missed {label}"));
                assert!(
                    (hit.fraction - fraction).abs() <= ratio(1, 1 << 20),
                    "{label} {hit:?}"
                );
                assert!(
                    (hit.point - frame.point(point)).length() <= SweepGap::Tolerance.units(),
                    "{label} {hit:?}"
                );
                assert!(
                    (hit.normal - frame.vector(normal)).length() <= ratio(1, 1 << 10),
                    "{label} {hit:?}"
                );
                assert!(
                    (hit.normal.length() - n(1)).abs() <= ratio(1, 1 << 20),
                    "{label} {hit:?}"
                );
                assert!(
                    (hit.depth - depth).abs() <= ratio(1, 1 << 16),
                    "{label} {hit:?} expected {depth:?}"
                );
            } else {
                assert!(hit.is_none(), "{label} {hit:?}");
            }
        }
    }
}
