//! Exercise discoverable controls of the prepared cloud sky on Metal.
#![cfg(feature = "proving")]

#[cfg(target_os = "macos")]
#[expect(
    dead_code,
    reason = "the common fixture also supports lease replacement and the plain sky"
)]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn sky_knob_endpoints_only_change_presentation() {
        verify_sky_knobs();
    }
}

#[cfg(target_os = "macos")]
fn verify_sky_knobs() {
    use thrum_render::{
        contribution::QualityInvalidation,
        runtime::{CreationPhase, FrameWorld},
    };
    use thrum_test_support::readback::{self, Mip};
    use v1_game::{CloudSelection, GameHost as _};

    let Some(gpu) = thrum_test_support::open(
        "cloud sky quality settings",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let fixture = fixture::Fixture::new(true);
    let mut host = pollster::block_on(fixture.host_with_clouds(&gpu, CloudSelection::WorldLight));
    let mut frames = fixture::Frames::new(&gpu);
    frames.placed_submitted(&mut host, |row| row.volume_instances == 1);
    host.world_mut().pause().expect("valid qualification input");
    frames.submit(&mut host);
    frames.submit(&mut host);
    let columns = host
        .cloud_placed_input()
        .expect("valid qualification input")
        .weather_columns()
        .expect("valid qualification input");
    let parked = host
        .world_mut()
        .publication()
        .expect("valid qualification input")
        .page
        .weather()
        .expect("valid qualification input");
    let inventory = host.cloud_resource_bytes();
    let read = || readback::texture(&gpu.device, &gpu.queue, &columns, Mip::level(0), ..);
    let original = read();
    assert!(
        original.iter().any(|byte| *byte != 0),
        "need a populated map"
    );
    let surface = [
        ("cloud weather map cadence", QualityInvalidation::Age),
        ("cloud near Sun path cadence", QualityInvalidation::Age),
        ("cloud middle Sun path cadence", QualityInvalidation::Age),
        ("cloud far Sun path cadence", QualityInvalidation::Age),
        ("cloud sky overlay cadence", QualityInvalidation::Age),
        ("cloud refresh exponent", QualityInvalidation::EffectHistory),
        (
            "cloud local light samples",
            QualityInvalidation::EffectHistory,
        ),
        ("cloud layer scale", QualityInvalidation::EffectHistory),
        (
            "cloud trace step length",
            QualityInvalidation::EffectHistory,
        ),
        ("cloud trace sample cap", QualityInvalidation::EffectHistory),
        ("cloud placed range", QualityInvalidation::EffectHistory),
    ];
    let all_knobs = host.quality_knobs();
    for (name, expected) in surface {
        let knob = all_knobs
            .iter()
            .find(|knob| knob.name.ends_with(name))
            .expect(name);
        assert_eq!(knob.invalidation, expected, "{name}");
        let mut previous = knob.value;
        for value in [*knob.range.start(), *knob.range.end()] {
            let before = host.creation_checkpoint();
            host.set_quality_knob(&knob.name, value)
                .expect("valid qualification input");
            frames.submit(&mut host);
            let after = host.creation_checkpoint();
            assert_eq!(before.named_pipelines, after.named_pipelines, "{name}");
            assert_eq!(
                before.phase(CreationPhase::Play),
                after.phase(CreationPhase::Play),
                "{name}"
            );
            assert_eq!(
                before.phase(CreationPhase::Transition),
                after.phase(CreationPhase::Transition),
                "{name}"
            );
            let current = host
                .quality_knobs()
                .into_iter()
                .find(|entry| entry.name == knob.name)
                .expect("valid qualification input");
            assert_eq!(current.value, value, "{name}");
            assert_eq!(host.cloud_resource_bytes(), inventory, "{name}");
            let current_state = host
                .world_mut()
                .publication()
                .expect("valid qualification input")
                .page
                .weather()
                .expect("valid qualification input");
            assert_eq!(current_state, parked, "{name}");
            assert_eq!(read(), original, "{name}");
            let layer = host
                .cloud_sky_products()
                .expect("valid qualification input");
            assert_eq!(
                layer.history_valid,
                previous == value || expected == QualityInvalidation::Age,
                "{name}"
            );
            frames.submit(&mut host);
            assert!(
                host.cloud_sky_products()
                    .expect("valid qualification input")
                    .history_valid,
                "{name} exceeded one frame"
            );
            previous = value;
        }
    }
}
