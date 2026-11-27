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

fn daylight(step: f32) -> Inputs {
    sun(30.0 + step * Tuning::default().sun.to_degrees() * 0.1)
}

fn submit(
    schedule: &mut ChartSchedule,
    inputs: Inputs,
    cadence: u32,
    camera_cut: bool,
    overlay: bool,
) -> ChartSubmission {
    let frame = schedule.plan(inputs, cadence, camera_cut, overlay, overlay);
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
    let phase_edit = crate::synthetic::SyntheticView::default().with_phase_history_reset();
    assert!(phase_edit.history_reset());
    assert!(!phase_edit.camera_cut());
    let cut_with_edit = phase_edit
        .with_history_reset(true)
        .with_phase_history_reset();
    assert!(cut_with_edit.camera_cut());
    for cadence in [4, 6, 8] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(30.0), cadence, false, false);
        let mut light = sun(30.0);
        let mut next_stage = 0;
        for step in 1..=cadence {
            light = daylight(step as f32);
            let abandoned = schedule.plan(light, cadence, false, false, false);
            let submitted = submit(
                &mut schedule,
                light,
                cadence,
                phase_edit.camera_cut(),
                false,
            );
            assert_eq!(abandoned.writes, submitted.writes);
            assert_eq!(submitted.whole_update, None);
            assert_eq!(submitted.written.submission, 1);
            assert!(submitted.writes.count_ones() <= 9_u32.div_ceil(cadence));
            for (index, source) in schedule.sources.iter().enumerate() {
                if submitted.writes & (1 << index) != 0 {
                    assert_eq!(index, next_stage);
                    let source = source.unwrap();
                    assert_eq!(source.submission, 1);
                    assert_eq!(source.inputs, daylight(1.0));
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
    let light = sun(30.0);
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
    let control = sun(30.0);
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
        (sun(32.1), false, WholeUpdate::SunJump),
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
    submit(&mut schedule, sun(30.0), 4, false, false);
    for step in 1..=4 {
        let frame = submit(
            &mut schedule,
            daylight(step as f32),
            if step < 2 { 4 } else { 8 },
            false,
            false,
        );
        assert_eq!(frame.written.submission, 1);
    }
    let frame = submit(&mut schedule, daylight(5.0), 8, false, false);
    assert_eq!(frame.written.submission, 5);
    assert_eq!(schedule.refresh.unwrap().spread, 8);
}

#[test]
fn recovery_satisfies_the_new_age_bound_from_its_first_submission() {
    for (old, new, moving) in [(6, 4, 6), (8, 6, 3)] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(30.0), old, false, false);
        for step in 1..=moving {
            submit(&mut schedule, daylight(step as f32), old, false, false);
        }
        for offset in 0..2 * new {
            let step = moving + 1 + offset;
            let frame = submit(
                &mut schedule,
                daylight(step as f32),
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

fn assert_source_bound(schedule: &ChartSchedule) {
    let live = schedule.last.unwrap().inputs;
    let widest = Tuning {
        twilight: None,
        ..schedule.tuning
    };
    for source in schedule.sources {
        assert!(live.change(source.unwrap().inputs, widest) <= 3.0);
    }
}

#[test]
fn governed_light_edits_dirty_the_cloudy_levels_without_an_overlay_write() {
    let initial = daylight(0.0);
    for overlay in [false, true] {
        let due = if overlay { ALL ^ 1 } else { 2 };
        let mut schedule = ChartSchedule::default();
        let mut step = |inputs| {
            let frame = schedule.plan(inputs, 8, false, overlay, false);
            schedule.submitted(&frame);
            frame
        };
        step(initial);
        let mut light = initial;
        light.illuminance *= 0.5;
        assert_eq!(step(light).writes, due);
        light.colour[0] *= 0.5;
        assert_eq!(step(light).writes, due);
        light.illuminance = 0.0;
        assert_eq!(step(light).writes, due);
        assert_eq!(step(light).writes, 0);
        light.illuminance = initial.illuminance;
        assert_eq!(step(light).writes, due);
        light.illuminance *= 1.001;
        let mut writes = 0;
        for position in 0..8 {
            let frame = step(light);
            assert_eq!(frame.position, Some((8, position)));
            assert_eq!(writes & frame.writes, 0);
            writes |= frame.writes;
        }
        assert_eq!(writes, due);
    }
}

#[test]
fn chooses_spread_from_each_input_measure() {
    let first = daylight(0.0);
    let tuning = Tuning::default();
    for (change, spread) in [(0.3, 3), (0.19, 5), (0.6, 1)] {
        let relative = 1.0 - tuning.light * change;
        for second in [
            sun(30.0 + tuning.sun.to_degrees() * change),
            Inputs {
                altitude: first.altitude + tuning.altitude * change,
                ..first
            },
            Inputs {
                illuminance: first.illuminance / relative,
                ..first
            },
            Inputs {
                colour: [1.0 / relative, 1.0, 1.0],
                ..first
            },
        ] {
            let mut schedule = ChartSchedule::default();
            submit(&mut schedule, first, 8, false, false);
            let frame = submit(&mut schedule, second, 8, false, false);
            assert_eq!(frame.position, Some((spread, 0)));
            assert!((frame.change.unwrap() - change).abs() < 0.001);
            assert_eq!(frame.whole_update, None);
        }
    }
}

#[test]
fn fast_first_steps_shorten_only_the_refresh_they_start() {
    for (cadence, change, expected) in [(8, 1.1, 1), (4, 0.4, 2)] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, daylight(0.0), cadence, false, false);
        for step in 1..=cadence {
            submit(&mut schedule, daylight(step as f32), cadence, false, false);
        }
        let first = cadence as f32 + 10.0 * change;
        for position in 0..expected {
            let frame = submit(
                &mut schedule,
                daylight(first + position as f32),
                cadence,
                false,
                false,
            );
            assert_eq!(frame.position, Some((expected, position)));
            assert_eq!(frame.whole_update, None);
            assert!(frame.writes.count_ones() <= 9_u32.div_ceil(expected));
            assert_source_bound(&schedule);
        }
        let frame = submit(
            &mut schedule,
            daylight(first + expected as f32),
            cadence,
            false,
            false,
        );
        assert_eq!(frame.position, Some((cadence, 0)));
        assert_source_bound(&schedule);
    }
}

#[test]
fn cutting_and_resuming_obeys_the_three_tolerance_bound() {
    let mut schedule = ChartSchedule::default();
    submit(&mut schedule, daylight(0.0), 8, false, false);
    for step in 1..=7 {
        submit(&mut schedule, daylight(step as f32), 8, false, false);
        assert_source_bound(&schedule);
    }
    let cut = submit(&mut schedule, daylight(12.0), 8, false, false);
    assert_eq!(cut.whole_update, Some(WholeUpdate::Motion));
    assert_eq!(cut.writes, ALL);
    assert_source_bound(&schedule);
    assert_eq!(
        submit(&mut schedule, daylight(13.0), 8, false, false).position,
        Some((8, 0))
    );
    let mut degrees = 30.0 + 13.0 * schedule.tuning.sun.to_degrees() * 0.1;
    for step in (0..=20).chain((0..20).rev()) {
        degrees += 0.02 + (1.5 - 0.02) * step as f32 / 20.0;
        submit(&mut schedule, sun(degrees), 8, false, false);
        assert_source_bound(&schedule);
    }
}

#[test]
fn authored_clock_and_scrub_steps_anchor_the_tuning() {
    for cadence in [4, 6, 8] {
        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(30.0), cadence, false, false);
        assert_eq!(
            submit(&mut schedule, sun(30.02), cadence, false, false).position,
            Some((cadence, 0))
        );
        let first = submit(&mut schedule, sun(31.52), cadence, false, false);
        assert_eq!(first.whole_update, Some(WholeUpdate::Motion));
        assert_eq!(first.writes, ALL);
        let full = submit(&mut schedule, sun(33.02), cadence, false, false);
        assert_eq!(full.position, Some((1, 0)));
        assert_eq!(full.writes, ALL);
        assert_source_bound(&schedule);
        assert_eq!(
            submit(&mut schedule, sun(33.04), cadence, false, false).position,
            Some((cadence, 0))
        );

        let mut schedule = ChartSchedule::default();
        submit(&mut schedule, sun(1.5), cadence, false, false);
        for elevation in [0.0, -1.5, -3.0] {
            let frame = submit(&mut schedule, sun(elevation), cadence, false, false);
            assert_eq!(frame.position, Some((1, 0)));
            assert_eq!(frame.writes, ALL);
            assert_source_bound(&schedule);
        }
    }
}

#[test]
fn tighter_sun_tolerance_shortens_a_new_refresh_or_cuts_a_running_one() {
    let widest = Tuning {
        twilight: None,
        ..Tuning::default()
    };
    let step = widest.sun.to_degrees() * 0.1;
    for crossing in [5, 9] {
        let elevation = 8.0_f32;
        let mut schedule = ChartSchedule {
            tuning: Tuning {
                twilight: Some(Twilight {
                    tight_band: [-1.0, elevation.to_radians().sin()],
                    widest_at: [-1.0, elevation.to_radians().sin()],
                    sun: widest.sun / 4.0,
                }),
                ..widest
            },
            ..ChartSchedule::default()
        };
        let input = |index: u32| sun(elevation + (crossing as f32 - 0.5 - index as f32) * step);
        submit(&mut schedule, input(0), 8, false, false);
        for index in 1..crossing {
            let frame = submit(&mut schedule, input(index), 8, false, false);
            assert_eq!(frame.position, Some((8, index - 1)));
            assert_source_bound(&schedule);
        }
        let frame = submit(&mut schedule, input(crossing), 8, false, false);
        if crossing == 5 {
            assert_eq!(frame.whole_update, Some(WholeUpdate::Motion));
            assert_eq!(frame.writes, ALL);
        } else {
            assert_eq!(frame.position, Some((2, 0)));
            assert_eq!(frame.writes.count_ones(), 5);
            let retained = schedule.sources[STAGES - 1].unwrap();
            assert_eq!(retained.submission, 1);
            assert!((input(crossing).change(retained.inputs, widest) - 0.8).abs() < 0.001);
            let next = submit(&mut schedule, input(crossing + 1), 8, false, false);
            assert_eq!(next.position, Some((2, 1)));
            assert_eq!(next.writes.count_ones(), 4);
        }
        assert_source_bound(&schedule);
    }
}
