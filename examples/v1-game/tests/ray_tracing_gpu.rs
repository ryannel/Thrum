//! Cooked publications through the game, with ray answers and raster bytes
//! from the same recorded scene.

mod ray_tracing {
    #[cfg(target_os = "macos")]
    mod proof;

    #[cfg(target_os = "macos")]
    use proof::{Fixture, Metal, Snapshot, report};
    #[cfg(target_os = "macos")]
    use std::time::{Duration, Instant};
    #[cfg(target_os = "macos")]
    use thrum_render::synthetic::mesh_rows::MeshRowStatus;
    #[cfg(target_os = "macos")]
    use v1_game::GameHost as _;

    thrum_test_support::macos_gpu_test! {
    fn sun_out() {
        proof::prove_sun_out();
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn contributions() {
        proof::prove_contributions();
    }
    }

    thrum_test_support::macos_gpu_test! {
    #[ignore = "contended valley timing is run by scripts/ray_tracing_cost.py"]
    fn shadow_cost() {
        proof::measure_shadows();
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn edit() {
        let Some(metal) = Metal::new(1) else {
            return;
        };
        let mut fixture = Fixture::new("edit");
        let (mut game, stats, _) = fixture.host(&metal, false);
        drop(proof::settle(&mut game, &stats, fixture.generation()));
        let queries = proof::edit_rays(&game);
        let mut previous = Snapshot::capture(&mut game, &metal, &queries);
        let old_hits = [queries[0].identity, 0, queries[2].identity, 0];
        assert_eq!(previous.hits, old_hits);
        report(&previous.record("initial", &stats));

        for (product, answers) in [
            ("placement", [0, old_hits[0], old_hits[2], 0]),
            ("entity", [0, old_hits[0], 0, old_hits[2]]),
        ] {
            let generation = fixture.edit(product, true, 8);
            let updates = game.cooked_rows().expect("rows").updates();
            let deadline = Instant::now() + Duration::from_secs(30);
            while updates.status(generation) != Some(MeshRowStatus::Ready) {
                let next = Snapshot::capture(&mut game, &metal, &queries);
                assert_eq!(next, previous, "an upload frame showed part of an edit");
                report(&next.record("before switch", &stats));
                assert!(Instant::now() < deadline, "publication never became ready");
            }
            let draws_before = stats.lock().expect("stats").submitted_frames;
            game.advance(Duration::from_millis(16))
                .expect("advance")
                .encode_scene()
                .expect("record switch")
                .skip();
            assert_eq!(stats.lock().expect("stats").submitted_frames, draws_before);
            assert_eq!(updates.status(generation), Some(MeshRowStatus::Ready));
            let held = proof::cast(
                &metal,
                |layout| game.ray_structure_probe_group(layout),
                &queries,
            );
            assert_eq!(held, previous.hits, "the discarded switch reached the GPU");
            report(&previous.record(&format!("{product} discarded switch"), &stats));
            let next = Snapshot::capture(&mut game, &metal, &queries);
            assert_eq!(
                next.hits, answers,
                "the replayed switch must trace the new geometry"
            );
            assert_ne!(
                next.pixels, previous.pixels,
                "the switch must change the drawn image"
            );
            assert_eq!(stats.lock().expect("stats").drawn, Some(generation));
            report(&next.record(&format!("{product} switch replay"), &stats));
            assert_eq!(Snapshot::capture(&mut game, &metal, &queries), next);
            previous = next;
        }

        // Change both readers in one publication. The entity row cannot fit, so
        // a valid replacement placement must remain hidden as well.
        let _ = fixture.edit_both_with_oversized_row();
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let next = Snapshot::capture(&mut game, &metal, &queries);
            assert_eq!(
                next, previous,
                "refusal changed raster bytes or ray answers"
            );
            if let Some(reason) = stats.lock().expect("stats").errors.last().cloned() {
                assert!(reason.contains("Capacity"), "{reason}");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "the oversized row was never refused"
            );
        }
        assert_eq!(Snapshot::capture(&mut game, &metal, &queries), previous);
        report(&previous.record("refused publication", &stats));
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn lifecycle() {
        let Some(metal) = Metal::new(1) else {
            return;
        };
        let mut fixture = Fixture::new("lifecycle");
        let (mut game, stats, probe) = fixture.host(&metal, true);
        let probe = probe.expect("mesh build observer");
        prepared_arena(&probe, 1);
        let initial = proof::settle(&mut game, &stats, fixture.generation());
        let initial_builds = probe.take_recorded();
        assert_eq!(initial_builds.len(), 2, "two distinct cooked uploads");
        assert!(
            initial_builds
                .iter()
                .all(|build| !build.mesh.starts_with("arena mesh "))
        );
        assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 2);
        assert_eq!(stats.lock().expect("stats").ray_mesh_storage.len(), 2);
        let queries = proof::edit_rays(&game);
        let first = Snapshot::capture(&mut game, &metal, &queries);
        report(&first.record("initial residency", &stats));
        for _ in 0..4 {
            assert_eq!(game.capture_pixels().expect("steady frame"), initial);
            assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 2);
            assert!(probe.take_recorded().is_empty());
        }
        let next = fixture.edit("placement", true, 8);
        let changed = proof::settle(&mut game, &stats, next);
        let [edit] = probe
            .take_recorded()
            .try_into()
            .ok()
            .expect("one edited structure");
        assert_eq!(edit.mesh, "placement");
        assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 3);
        let edited = Snapshot::capture(&mut game, &metal, &queries);
        assert_eq!(edited.pixels, changed);
        report(&edited.record("one edited placement product rebuilt", &stats));

        let next = fixture.edit("entity", true, 8);
        let changed = proof::settle(&mut game, &stats, next);
        let [edit] = probe
            .take_recorded()
            .try_into()
            .ok()
            .expect("one edited entity row");
        assert!(edit.mesh.starts_with("arena mesh "));
        assert_eq!(edit.triangles, 128);
        assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 3);
        let before = Snapshot::capture(&mut game, &metal, &queries);
        assert_eq!(before.pixels, changed);
        report(&before.record("one edited entity row rebuilt", &stats));

        let replacement = metal.replacement();
        let mut candidate =
            pollster::block_on(game.device_prepare_inputs().expect("inputs").prepare(
                &replacement.caps.clone().with_ray_preparation_observation(),
                &replacement.queue,
                None,
            ))
            .expect("replacement device");
        game.publish_device(&mut candidate).expect("publish device");
        let rebuilt_arena = game
            .observe_ray_mesh_builds()
            .expect("replacement observer");
        prepared_arena(&rebuilt_arena, 2);
        assert_eq!(proof::settle(&mut game, &stats, next), changed);
        let restored_builds = rebuilt_arena.take_recorded();
        assert_eq!(restored_builds.len(), 2, "only the two cooked uploads");
        assert!(
            restored_builds
                .iter()
                .all(|build| !build.mesh.starts_with("arena mesh "))
        );
        let after = Snapshot::capture(&mut game, &replacement, &queries);
        assert_eq!(before, after);
        assert_eq!(
            stats.lock().expect("stats").drawn_device_generation,
            Some(2)
        );
        assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 2);
        report(&after.record("replacement device residency", &stats));
        game.world_mut().resume(Duration::from_secs(1))
            .expect("resume the game");
        let frozen_tick = game.world().tick();
        for offset in [17, 34, 51] {
            game.advance_without_rendering(Duration::from_millis(1_000 + offset))
                .expect("play the next tick");
            drop(game.capture_pixels().expect("moving frame"));
            assert!(rebuilt_arena.take_recorded().is_empty());
            assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 2);
        }
        assert!(game.world().tick() > frozen_tick);
    }
    }

    #[cfg(target_os = "macos")]
    fn prepared_arena(probe: &thrum_render::synthetic::RayBuildProbe, device_generation: u64) {
        let meshes = probe
            .take_recorded()
            .into_iter()
            .map(|build| (build.mesh, build.triangles))
            .collect::<Vec<_>>();
        assert_eq!(
            meshes.len(),
            5,
            "every arena row built before the first recording"
        );
        for (index, (name, triangles)) in meshes.iter().enumerate() {
            assert_eq!(*name, format!("arena mesh {index}"));
            assert!(*triangles > 0);
        }
        report(&serde_json::json!({
            "phase": "ready before first frame", "device_generation": device_generation,
            "arena_mesh_builds": meshes, "ordinary_frames_begun": 0
        }));
    }

    thrum_test_support::macos_gpu_test! {
    fn demand() {
        proof::prove_demand();
    }
    }

    thrum_test_support::macos_gpu_test! {
    fn casters() {
        proof::prove_casters();
    }
    }

    thrum_test_support::macos_gpu_test! {
    #[ignore = "the contended Metal measurement runs through scripts/ray_tracing_cost.py"]
    fn valley_cost() {
        proof::measure_valley();
    }
    }
}
