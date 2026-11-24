use super::Uniform;

#[test]
fn row_cadence_bounds_weather_time_through_variable_rates_and_hitches() {
    use super::{RowStamp, update_rows};
    use thrum_cloud_model::Parameters;
    let parameters = Parameters {
        coverage: 0.0,
        kind: 0.0,
        wind: [0.0; 2],
        time_scale: 1.0,
    };
    for age in [0.5, 8.0 / 120.0, 32.0 / 120.0, 128.0 / 120.0] {
        let mut time = 0.0;
        let mut rows = vec![
            RowStamp {
                tick: 0,
                time,
                parameters
            };
            256
        ];
        let mut full = 0;
        for tick in 1..=10000 {
            let dt = match (tick / 1000) % 5 {
                0 => 1.0 / 60.0,
                1 => 8.0 / 60.0,
                2 => 1.0,
                3 => [0.001, 0.01, 0.02, 0.005, 0.04]
                    [usize::try_from(tick % 5).expect("bounded index")],
                _ => match tick % 17 {
                    0 => 0.0,
                    1 => 0.49,
                    2 => 0.001,
                    3 => 1.0,
                    4 => 8.0 / 60.0,
                    5 => 5.0,
                    _ => 1.0 / 60.0,
                },
            };
            time += dt;
            let (_, count) = update_rows(
                &mut rows,
                RowStamp {
                    tick,
                    time,
                    parameters,
                },
                dt,
                age,
                false,
            );
            if dt == 0.0 {
                assert_eq!(count, 0, "paused weather writes nothing");
            }
            if count == 256 {
                full += 1;
            }
            assert!(
                rows.iter().all(|r| time - r.time <= age + 1e-10),
                "weather-time age bound at tick {tick}, dt {dt}, cadence {age}"
            );
        }
        assert!(full > 0, "hitch must refill");
    }
}

#[test]
fn coherent_global_ramp_does_not_leave_half_second_old_rows() {
    use super::{RowStamp, ramp_row_age, update_rows};
    use thrum_cloud_model::Parameters;

    let mut previous = RowStamp {
        tick: 0,
        time: 0.0,
        parameters: Parameters {
            coverage: 0.72,
            kind: 0.0,
            wind: [10.0, 0.0],
            time_scale: 1.0,
        },
    };
    let mut rows = vec![previous; 256];
    let dt = 1.0 / 60.0;
    for tick in 1..=160 {
        let mut next = previous;
        next.tick = tick;
        next.time += dt;
        next.parameters.coverage -= 0.002 * dt;
        let age = 0.5_f64.min(ramp_row_age(previous, next, dt));
        let (_, count) = update_rows(&mut rows, next, dt, age, false);
        assert!(count > 9 && count < 256, "refresh a larger strip");
        assert!(
            rows.iter()
                .all(|row| row.parameters.coverage - next.parameters.coverage <= 0.00005),
            "the rotating strips sample close points of the coverage ramp"
        );
        previous = next;
    }
    assert!(
        ramp_row_age(previous, previous, dt).is_infinite(),
        "steady controls retain the profile cadence"
    );
}

#[test]
fn shader_validates_and_uniform_offsets_match_rust() {
    use wgpu::naga;
    let module =
        naga::front::wgsl::parse_str(include_str!("../weather.wgsl")).expect("weather WGSL");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .expect("valid weather compute");
    thrum_test_support::records::named(
        "weather shader",
        include_str!("../weather.wgsl"),
        &[thrum_test_support::record!(
            Uniform as "Params",
            integers,
            origin,
            weather,
            physical,
            vertical,
            noise,
            rows,
            turret
        )],
    );
}
