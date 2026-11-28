use thrum_sim3d::{SweepGap, sweep_two};

use crate::{
    cases::{frames, moving_cases},
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
