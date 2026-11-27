//! Exercise the Mac panel's Ground controls through the game's prepared Metal scene.
#![cfg(feature = "proving")]

#[cfg(target_os = "macos")]
#[expect(
    dead_code,
    reason = "the shared fixture supports the other placed-sky proofs"
)]
mod fixture;

thrum_test_support::macos_gpu_test! {
    fn ground_presets_and_fields_edit_the_prepared_scene() {
        use thrum_clouds::FieldOverlay;
        use thrum_render::runtime::CreationPhase;
        use v1_game::{CloudSelection, GameHost as _, GameSetup, GroundWeather};

        let Some(gpu) = thrum_test_support::open(
            "cloud Ground controls",
            Some(wgpu::Backend::Metal),
            thrum_test_support::Profile::Portable,
            &thrum_test_support::Restriction::default(),
        ) else {
            return;
        };
        let selection = CloudSelection::Ground(GroundWeather::FarMist);
        let fixture = fixture::Fixture::new(true);
        let mut host = pollster::block_on(fixture.host_with_clouds(&gpu, selection));
        let mut frames = fixture::Frames::new(&gpu);
        frames.placed_submitted(&mut host, |row| row.volume_instances == 1);
        let mut controls = GameSetup { clouds: selection, ..GameSetup::default() }
            .game_controls().expect("Ground controls");
        let inventory = host.cloud_resource_bytes();
        let before = host.creation_checkpoint();
        for (file, expected, radiative) in [
            (include_str!("../../params/clouds-dawn-mist.params.json"), [1.0, 0.0, 1.0, 60., 0.], true),
            (include_str!("../../params/clouds-sea-fog.params.json"), [0.0, 0.75, 0.9, 35., 0.], false),
            (include_str!("../../params/clouds-ground-study.params.json"), [0.6, 0.3, 0.8, 85., 1.3], false),
        ] {
            let outcome = controls.model_mut().apply_file(file).expect("valid file");
            assert!(outcome.problems.is_empty(), "{outcome}");
            let edits = controls.take_edits().clouds;
            let ground = edits.ground.expect("ground preset edit");
            let [mist, fog] = ground.amounts;
            assert_eq!(ground.radiative_day, radiative);
            for (actual, expected) in [mist, fog, ground.moisture, ground.visual_range / 1000., ground.angstrom_exponent]
                .into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-6);
            }
            host.edit_cloud_controls_for_test(edits);
            frames.submit(&mut host);
        }
        assert_eq!(controls.replay().clouds.overlay, Some(FieldOverlay::GroundDatum));
        host.world_mut().pause().expect("hold the ground state");
        frames.submit(&mut host);
        let input = host.cloud_placed_input().expect("cloud input").clone();
        for field in [FieldOverlay::GroundSources, FieldOverlay::GroundDatum, FieldOverlay::GroundBand, FieldOverlay::Off] {
            input.set_field_overlay(field).expect("diagnostic field");
            frames.submit(&mut host);
        }
        let after = host.creation_checkpoint();
        assert_eq!(before.named_pipelines, after.named_pipelines);
        assert_eq!(before.phase(CreationPhase::Transition), after.phase(CreationPhase::Transition));
        assert_eq!(before.phase(CreationPhase::Play), after.phase(CreationPhase::Play));
        assert_eq!(inventory, host.cloud_resource_bytes());
    }
}
