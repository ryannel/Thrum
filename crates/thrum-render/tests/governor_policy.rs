//! The governor consumes virtual host time and explicitly marked observations.

use std::time::Duration;
use thrum_render::{
    governor::{ChangeReason, DisplayRecord, FrameCost, Governor, ThermalState, TimingSource},
    synthetic::RenderScale,
};

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn display(interval: u64) -> DisplayRecord {
    DisplayRecord {
        interval: ms(interval),
        thermal: ThermalState::Nominal,
    }
}

fn cost(ms_work: u64, source: TimingSource) -> FrameCost {
    FrameCost {
        elapsed: ms(ms_work),
        scale_dependent: ms(ms_work),
        source,
    }
}

fn observe(policy: &mut Governor, order: u64, duration: u64) {
    policy.observe(order, policy.mark(), cost(duration, TimingSource::Gpu));
}

#[test]
fn panic_requires_three_consecutive_over_budget_frames() {
    let mut policy = Governor::new(RenderScale::Native, display(16));
    for (order, duration) in [(1, 15), (2, 15), (3, 13), (4, 15), (5, 15)] {
        observe(&mut policy, order, duration);
        assert!(!policy.commit(ms(order * 16)));
    }
    observe(&mut policy, 6, 15);
    assert!(policy.commit(ms(96)));
    assert_eq!(policy.state().scale, RenderScale::Percent85);
    assert_eq!(policy.state().reason, ChangeReason::OverBudget);
}

#[test]
fn refresh_changes_the_budget_without_committing_quality() {
    let mut policy = Governor::new(RenderScale::Native, display(20));
    for order in 1..13 {
        observe(&mut policy, order, 12);
        assert!(!policy.commit(ms(order * 20)));
    }
    observe(&mut policy, 13, 18);
    observe(&mut policy, 14, 18);
    let old_budget_mark = policy.mark();
    let before = policy.state();
    assert_eq!(policy.budget(), ms(17));
    policy.boundary(ms(260), display(10), false);
    assert_eq!(policy.budget(), Duration::from_micros(8500));
    assert_eq!(policy.state(), before);
    policy.observe(15, old_budget_mark, cost(18, TimingSource::Gpu));
    for order in 16..19 {
        observe(&mut policy, order, 12);
        assert_eq!(policy.commit(ms(260 + (order - 15) * 10)), order == 18);
    }
    assert_eq!(policy.state().scale, RenderScale::Percent85);
}

#[test]
fn recovery_has_a_deadband_and_a_one_step_minimum_period() {
    let mut policy = Governor::new(RenderScale::Percent67, display(20));
    for order in 1..15 {
        observe(&mut policy, order, 16);
        assert!(!policy.commit(ms(order * 200)));
    }
    let stable_start = 3000;
    for order in 15..27 {
        observe(&mut policy, order, 10);
        assert_eq!(
            policy.commit(ms(stable_start + (order - 15) * 20)),
            order == 26
        );
    }
    assert_eq!(policy.state().scale, RenderScale::Percent75);
    for order in 27..40 {
        observe(&mut policy, order, 10);
    }
    assert!(!policy.commit(ms(4219)));
    assert!(policy.commit(ms(4220)));
    assert_eq!(policy.state().scale, RenderScale::Percent85);
    assert_eq!(policy.state().reason, ChangeReason::Recovery);
}

#[test]
fn scale_recovery_projects_work_at_the_thermal_tier_floor() {
    let mut record = display(16);
    record.thermal = ThermalState::Fair;
    let mut policy = Governor::new(RenderScale::Percent85, record);
    assert!(policy.commit(ms(16)));
    assert_eq!(policy.state().tier, 1);

    for order in 1..=12 {
        observe(&mut policy, order, 11);
        assert!(!policy.commit(ms(1016 + order * 16)));
    }
    for order in 13..=24 {
        observe(&mut policy, order, 8);
        assert_eq!(policy.commit(ms(1016 + order * 16)), order == 24);
    }
    assert_eq!(policy.state().scale, RenderScale::Native);
    assert_eq!(policy.state().tier, 1);
}

fn build_hold(switched: bool) {
    let mut policy = Governor::new(RenderScale::Percent85, display(16));
    let committed = policy.state();
    observe(&mut policy, 1, 15);
    observe(&mut policy, 2, 15);
    let before_build = policy.mark();
    policy.boundary(ms(40), display(16), true);
    let building = policy.mark();
    assert!(building.held());
    observe(&mut policy, 3, 15);
    policy.observe_missed(building, 20, false);
    assert!(!policy.commit(ms(1000)));
    let mut next = 4;
    if switched {
        policy.boundary(ms(1200), display(16), true);
        let first = policy.mark();
        assert!(first.held());
        observe(&mut policy, next, 15);
        policy.observe_missed(first, 20, false);
        assert!(!policy.commit(ms(1200)));
        next += 1;
    }
    assert_eq!(policy.state(), committed);
    policy.boundary(ms(1300), display(16), false);
    policy.observe(next, before_build, cost(15, TimingSource::Gpu));
    policy.observe(next + 1, building, cost(15, TimingSource::Gpu));
    policy.observe_missed(building, 20, false);
    assert!(!policy.commit(ms(1300)));
    for index in 0..3 {
        observe(&mut policy, next + 2 + index, 15);
        assert_eq!(policy.commit(ms(1316 + index * 16)), index == 2);
    }
    assert_eq!(policy.state().scale, RenderScale::Percent75);
}

#[test]
fn a_failed_or_cancelled_build_reopens_on_the_retained_generation() {
    build_hold(false);
}

#[test]
fn a_switch_excludes_the_first_frame_of_the_new_generation() {
    build_hold(true);
}

#[test]
fn counted_display_link_misses_are_a_separate_pressure_source() {
    let mut policy = Governor::new(RenderScale::Native, display(16));
    policy.observe_missed(policy.mark(), 2, false);
    policy.observe_missed(policy.mark(), 0, true);
    policy.observe_missed(policy.mark(), 2, false);
    assert!(!policy.commit(ms(40)));
    policy.observe_missed(policy.mark(), 1, false);
    assert!(policy.commit(ms(50)));
    assert_eq!(policy.source(), Some(TimingSource::DisplayLink));
}

#[test]
fn animation_frame_budget_allows_idle_at_the_display_cadence() {
    let mut policy = Governor::new(RenderScale::Native, display(16));
    for order in 1..15 {
        policy.observe(order, policy.mark(), cost(16, TimingSource::AnimationFrame));
        assert!(!policy.commit(ms(order * 16)));
    }
    for order in 15..18 {
        policy.observe(order, policy.mark(), cost(32, TimingSource::AnimationFrame));
        assert_eq!(policy.commit(ms(order * 32)), order == 17);
    }
}
