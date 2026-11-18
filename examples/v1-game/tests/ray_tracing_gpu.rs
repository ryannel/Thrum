//! Cooked publications through the game, with ray answers and raster bytes
//! from the same recorded scene.

#[cfg(target_os = "macos")]
mod ray_tracing {
    mod proof;

    use proof::{Fixture, Metal, Snapshot, report};
    use std::time::{Duration, Instant};
    use thrum_render::synthetic::mesh_rows::MeshRowStatus;

    #[test]
    fn edit() {
        let metal = Metal::new(1);
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

    #[test]
    fn lifecycle() {
        let metal = Metal::new(1);
        let mut fixture = Fixture::new("lifecycle");
        let (mut game, stats, probe) = fixture.host(&metal, true);
        let probe = probe.expect("mesh build observer");
        let initial = proof::settle(&mut game, &stats, fixture.generation());
        let initial_builds = probe.take_recorded();
        assert_eq!(
            initial_builds.len(),
            7,
            "five arena meshes and two distinct placements"
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
        let before = Snapshot::capture(&mut game, &metal, &queries);
        assert_eq!(before.pixels, changed);
        report(&before.record("one edited product rebuilt", &stats));

        let replacement = Metal::new(2);
        let mut candidate =
            pollster::block_on(game.device_prepare_inputs().expect("inputs").prepare(
                &replacement.caps,
                &replacement.queue,
                None,
            ))
            .expect("replacement device");
        game.publish_device(&mut candidate).expect("publish device");
        let rebuilt_arena = game
            .observe_ray_mesh_builds()
            .expect("replacement observer");
        assert_eq!(proof::settle(&mut game, &stats, next), changed);
        assert_eq!(
            rebuilt_arena.take_recorded().len(),
            5,
            "every retained arena row rebuilt"
        );
        let after = Snapshot::capture(&mut game, &replacement, &queries);
        assert_eq!(before, after);
        assert_eq!(
            stats.lock().expect("stats").drawn_device_generation,
            Some(2)
        );
        assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 2);
        report(&after.record("replacement device residency", &stats));
    }

    #[test]
    fn demand() {
        proof::prove_demand();
    }

    #[test]
    fn casters() {
        proof::prove_casters();
    }

    #[test]
    #[ignore = "the contended Metal measurement runs through scripts/ray_tracing_cost.py"]
    fn valley_cost() {
        proof::measure_valley();
    }
}
