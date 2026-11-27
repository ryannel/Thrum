use super::*;

fn sun(degrees: f32) -> Inputs {
    let (y, x) = degrees.to_radians().sin_cos();
    Inputs {
        direction: [x, y, 0.0],
        altitude: 50.0,
        illuminance: 100.0,
        colour: [1.0; 3],
        profile: 0,
        aerosol: crate::atmosphere::Aerosol::default(),
        night: false,
    }
}

fn submit(
    schedule: &mut ChartSchedule,
    inputs: Inputs,
    cadence: u32,
    camera_cut: bool,
    overlay: bool,
) -> ChartSubmission {
    let frame = schedule.plan(inputs, cadence, camera_cut, overlay);
    schedule.submitted(&frame);
    frame
}

#[test]
fn cadence_one_uses_chart_keys_and_actual_overlay_writes() {
    let mut schedule = ChartSchedule::default();
    let submit = |schedule: &mut ChartSchedule, input, cut, overlay| {
        submit(schedule, input, 1, cut, overlay).writes
    };
    let mut light = sun(5.0);
    assert_eq!(submit(&mut schedule, light, false, false), ALL);
    light = sun(5.5);
    assert_eq!(submit(&mut schedule, light, false, false), ALL);
    light.altitude = 51.0;
    assert_eq!(submit(&mut schedule, light, false, false), ALL);
    light.illuminance *= 0.5;
    assert_eq!(submit(&mut schedule, light, false, false), 2);
    light.colour[1] *= 0.5;
    assert_eq!(submit(&mut schedule, light, false, false), 2);
    assert_eq!(submit(&mut schedule, light, false, false), 0);
    assert_eq!(submit(&mut schedule, light, true, false), 0);

    assert_eq!(submit(&mut schedule, light, false, true), ALL ^ 1);
    light.illuminance *= 0.5;
    assert_eq!(submit(&mut schedule, light, false, false), 2);
    light.colour[2] *= 0.5;
    assert_eq!(submit(&mut schedule, light, false, false), 2);
    assert_eq!(submit(&mut schedule, light, false, false), 0);
    assert_eq!(submit(&mut schedule, light, true, false), 0);

    // A directly uploaded overlay has no graph notification. Every frame is
    // treated as a possible write by the caller, even after uploads stop.
    for dim in [false, true] {
        if dim {
            light.illuminance *= 0.5;
        }
        assert_eq!(submit(&mut schedule, light, false, true), ALL ^ 1);
    }
}

#[test]
fn spread_submissions_latch_inputs_retry_and_drain() {
    for cadence in [4, 6, 8] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(0.0), cadence, false, false);
        let mut light = sun(0.0);
        let mut next_stage = 0;
        for step in 1..=cadence {
            light = sun(step as f32 * 0.5);
            let abandoned = schedule.plan(light, cadence, false, false);
            let submitted = submit(&mut schedule, light, cadence, false, false);
            assert_eq!(abandoned.writes, submitted.writes);
            assert_eq!(submitted.whole_update, None);
            assert_eq!(submitted.written.submission, 1);
            assert!(submitted.writes.count_ones() <= 9_u32.div_ceil(cadence));
            for index in 0..STAGES {
                if submitted.writes & (1 << index) != 0 {
                    assert_eq!(index, next_stage);
                    let source = schedule.sources[index].unwrap();
                    assert_eq!(source.submission, 1);
                    assert_eq!(source.inputs, sun(0.5));
                    next_stage += 1;
                }
            }
        }
        assert_eq!(next_stage, STAGES);
        for _ in 0..cadence {
            submit(&mut schedule, light, cadence, false, false);
        }
        assert_eq!(
            submit(&mut schedule, light, cadence, false, false).writes,
            0
        );
        assert!(schedule.refresh.is_none());
        assert!(
            schedule
                .sources
                .iter()
                .all(|source| source.unwrap().inputs == light)
        );
    }
}

#[test]
fn late_overlay_write_reaches_each_product_and_drains() {
    let mut schedule = ChartSchedule::default();
    let light = sun(0.0);
    submit(&mut schedule, light, 4, false, false);
    let frame = submit(&mut schedule, light, 4, false, true);
    assert_eq!(frame.writes, 0b110);
    for position in 1..8 {
        submit(&mut schedule, light, 4, false, position == 1);
    }
    assert_eq!(submit(&mut schedule, light, 4, false, false).writes, 0);
    assert_eq!(schedule.sources[0].unwrap().overlay_revision, 0);
    for source in &schedule.sources[1..] {
        assert_eq!(source.unwrap().overlay_revision, 2);
    }
}

#[test]
fn discontinuities_publish_the_current_snapshot() {
    let control = sun(0.0);
    for (inputs, cut, reason) in [
        (control, true, WholeUpdate::CameraCut),
        (
            Inputs {
                profile: 1,
                ..control
            },
            false,
            WholeUpdate::AirChange,
        ),
        (
            Inputs {
                night: true,
                ..control
            },
            false,
            WholeUpdate::AirChange,
        ),
        (
            Inputs {
                aerosol: crate::atmosphere::Aerosol::new(control.aerosol.extinction_550_per_m, 1.0)
                    .unwrap(),
                ..control
            },
            false,
            WholeUpdate::AirChange,
        ),
        (sun(2.1), false, WholeUpdate::SunJump),
    ] {
        let mut schedule = ChartSchedule::default();
        let first = submit(&mut schedule, control, 8, false, false);
        assert_eq!(first.writes, ALL);
        assert_eq!(first.whole_update, Some(WholeUpdate::Initialization));
        let frame = submit(&mut schedule, inputs, 8, cut, false);
        assert_eq!(frame.writes, ALL);
        assert_eq!(frame.whole_update, Some(reason));
        assert!(
            schedule
                .sources
                .iter()
                .all(|source| source.unwrap().submission == 1)
        );
    }
    // A new prepared generation owns a fresh schedule.
    assert_eq!(
        submit(&mut ChartSchedule::default(), control, 8, false, false).writes,
        ALL
    );
}

#[test]
fn lengthening_finishes_the_active_refresh() {
    let mut schedule = ChartSchedule::default();
    submit(&mut schedule, sun(0.0), 4, false, false);
    for step in 1..=4 {
        let frame = submit(
            &mut schedule,
            sun(step as f32),
            if step < 2 { 4 } else { 8 },
            false,
            false,
        );
        assert_eq!(frame.written.submission, 1);
    }
    let frame = submit(&mut schedule, sun(5.0), 8, false, false);
    assert_eq!(frame.written.submission, 5);
    assert_eq!(schedule.refresh.unwrap().cadence, 8);
}

#[test]
fn recovery_satisfies_the_new_age_bound_from_its_first_submission() {
    for (old, new, moving) in [(6, 4, 6), (8, 6, 3)] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(0.0), old, false, false);
        for step in 1..=moving {
            submit(&mut schedule, sun(step as f32), old, false, false);
        }
        for offset in 0..2 * new {
            let step = moving + 1 + offset;
            let frame = submit(
                &mut schedule,
                sun(step as f32),
                u32::try_from(new).unwrap(),
                false,
                false,
            );
            if offset == 0 {
                assert_eq!(frame.writes, ALL);
                assert_eq!(frame.whole_update, Some(WholeUpdate::Recovery));
            } else {
                assert_eq!(frame.whole_update, None);
            }
            for (index, source) in schedule.sources.iter().enumerate() {
                let age = step - source.unwrap().submission;
                let bound = if index == 0 { new - 1 } else { 2 * new - 2 };
                assert!(
                    age <= bound,
                    "stage {index}, age {age}, bound {bound}, frame {step}"
                );
            }
        }
    }
}
