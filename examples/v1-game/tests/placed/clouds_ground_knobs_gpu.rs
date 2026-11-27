//! Ground cadence is a retained presentation setting on Metal.
#![cfg(feature = "proving")]

#[cfg(target_os = "macos")]
#[expect(
    dead_code,
    reason = "the shared fixture supports the other placed-sky proofs"
)]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn ground_cadences_keep_weather_columns_and_queries() {
        use thrum_cloud_model::Field;
        use thrum_render::{
            contribution::QualityInvalidation,
            runtime::{CreationPhase, FrameWorld},
        };
        use thrum_test_support::readback::{self, Mip};
        use v1_game::{CloudSelection, GameHost as _, GroundWeather};

        let Some(gpu) = thrum_test_support::open(
            "cloud Ground cadence",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        ) else {
            return;
        };
        let fixture = fixture::Fixture::new(true);
        let mut host = pollster::block_on(fixture.host_with_clouds(
            &gpu, CloudSelection::Ground(GroundWeather::Mist),
        ));
        let mut frames = fixture::Frames::new(&gpu);
        frames.placed_submitted(&mut host, |row| row.volume_instances == 1);
        host.world_mut().pause().expect("hold the world");
        frames.submit(&mut host);
        let state = |host: &mut fixture::Host| {
            host.world_mut().publication().expect("held weather page").page.weather().expect("weather")
        };
        let parked = state(&mut host);
        let config = fixture::Fixture::profile();
        let field = Field::new(config.seed, config.model, config.types).expect("weather field");
        let point = [0.0, 700.0, -1100.0];
        let expected = field.query(point, &parked);
        let columns = host.cloud_placed_input().expect("placed input")
            .weather_columns().expect("weather columns");
        let read = || readback::texture(&gpu.device, &gpu.queue, &columns, Mip::level(0), ..);
        let original = read();
        assert!(original.iter().any(|byte| *byte != 0));
        let before = host.creation_checkpoint();
        for name in ["cloud ground band cadence", "cloud medium coefficients cadence"] {
            let knob = host.quality_knobs().into_iter().find(|knob| knob.name.ends_with(name))
                .expect("Ground cadence declaration");
            assert_eq!(knob.invalidation, QualityInvalidation::Age);
            for value in [*knob.range.end(), *knob.range.start()] {
                host.set_quality_knob(&knob.name, value).expect("set cadence");
                for _ in 0..=*knob.range.end() {
                    frames.submit(&mut host);
                }
                assert!(host.quality_knobs().iter().any(|current| current.name == knob.name && current.value == value));
                let current = state(&mut host);
                assert_eq!(current, parked);
                assert_eq!(field.query(point, &current), expected);
                assert_eq!(read(), original);
            }
        }
        let after = host.creation_checkpoint();
        assert_eq!(before.named_pipelines, after.named_pipelines);
        assert_eq!(before.phase(CreationPhase::Transition), after.phase(CreationPhase::Transition));
        assert_eq!(before.phase(CreationPhase::Play), after.phase(CreationPhase::Play));
    }
}
