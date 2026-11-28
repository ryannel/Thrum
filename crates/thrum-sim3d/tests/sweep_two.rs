use thrum_sim3d::{DOMAIN_BOUND, SweepGap, depth, distance, sweep_two};

use crate::{
    cases::{domain_frames, frames, moving_cases, n, opposing_cases, ratio},
    sweep::check_hit,
};

#[test]
fn hand_worked_contacts_with_two_motions() {
    let cases = moving_cases();
    for frame in frames() {
        for case in &cases {
            let gap = SweepGap::Tolerance;
            let hit = sweep_two(
                frame.shape(case.mover),
                frame.vector(case.motion),
                frame.shape(case.struck),
                frame.vector(case.struck_motion),
                gap,
            )
            .unwrap();
            check_hit(frame, case, gap, hit);
        }
    }
}

#[test]
fn opposing_paths_cross_at_domain_scale() {
    for frame in domain_frames() {
        for case in opposing_cases() {
            let gap = SweepGap::Tolerance;
            let first = frame.shape(case.mover);
            let second = frame.shape(case.struck);
            let motion = frame.vector(case.motion);
            let struck_motion = frame.vector(case.struck_motion);
            assert!(distance(first, second).unwrap() > gap.units());
            assert!(
                distance(first.translated(motion), second.translated(struck_motion)).unwrap()
                    > gap.units()
            );
            if case.id == "opposing-solids" {
                let center_crossing = ratio(32760, 50002);
                let penetration = depth(
                    first.translated(motion * center_crossing),
                    second.translated(struck_motion * center_crossing),
                )
                .unwrap();
                assert_eq!(penetration.depth, n(16));
            } else {
                let crossing = ratio(DOMAIN_BOUND, 60_000);
                let before = crossing - ratio(1, 1 << 24);
                let gap_at = |fraction| {
                    distance(
                        first.translated(motion * fraction),
                        second.translated(struck_motion * fraction),
                    )
                    .expect("both poses are in the domain")
                };
                assert_eq!(before.to_bits() + 1, crossing.to_bits());
                assert_eq!(gap_at(crossing), n(0));
                let last_clear = gap_at(before);
                assert!(last_clear > ratio(1, 128) && last_clear <= gap.units());
            }
            let hit = sweep_two(first, motion, second, struck_motion, gap).unwrap();
            check_hit(frame, &case, gap, hit);
        }
    }
}
