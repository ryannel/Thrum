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
    let (_, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some("Params"))
        .expect("parameters");
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("parameters struct")
    };
    assert_eq!(*span as usize, size_of::<Uniform>());
    let offsets = [
        std::mem::offset_of!(Uniform, integers),
        std::mem::offset_of!(Uniform, origin),
        std::mem::offset_of!(Uniform, weather),
        std::mem::offset_of!(Uniform, physical),
        std::mem::offset_of!(Uniform, vertical),
        std::mem::offset_of!(Uniform, noise),
        std::mem::offset_of!(Uniform, rows),
    ];
    assert_eq!(members.len(), offsets.len());
    for (member, offset) in members.iter().zip(offsets) {
        assert_eq!(member.offset as usize, offset);
    }
}
