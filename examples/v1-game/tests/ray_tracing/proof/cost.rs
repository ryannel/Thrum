//! Time production builds where they enter the resident valley and its edit.

use super::demand::valley;
use super::*;
use thrum_render::synthetic::RayBuildProbe;

pub(crate) fn measure_valley() {
    let root = scratch("valley-cost");
    let cook = editable_valley(&root);
    let mut cooker = Cooker::open(cook.clone()).expect("cook real valley");
    assert_eq!(cooker.cook_all().expect("initial valley").failed, []);
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let generation = generation(&cook.store);
    let resident = resident_delta(&metal, &cook.store, generation);
    report(&resident);
    let (mut game, stats, probe) = valley(&cook.store, &metal, true, true);
    let probe = probe.expect("Metal build observer");
    let prepared = probe.take_recorded();
    assert_eq!(
        prepared.len(),
        2,
        "both arena orientations built in preparation"
    );
    for sample in &prepared {
        report_build(&metal, sample, "initial", 0, None, false);
    }
    let first = rendered_builds(&mut game, &metal, &stats, generation, &probe, "initial");
    assert_eq!(first, 3, "three cooked meshes built during staged loading");
    assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 3);
    drop(game.capture_pixels().expect("steady rendered valley"));
    assert!(probe.take_recorded().is_empty());

    // Preserve the terrain's topology and change the product's world-space height.
    let terrain = cook.source.join("terrain.gltf");
    let mut source: Value =
        serde_json::from_slice(&std::fs::read(&terrain).expect("terrain")).expect("terrain JSON");
    source["nodes"][0]["translation"] = json!([0.0, 0.125, 0.0]);
    std::fs::write(
        terrain,
        serde_json::to_vec(&source).expect("edited terrain"),
    )
    .expect("write edit");
    assert_eq!(cooker.cook_all().expect("edited valley").failed, []);
    let next = super::generation(&cook.store);
    assert!(next > generation);
    assert_eq!(
        rendered_builds(&mut game, &metal, &stats, next, &probe, "edit"),
        1
    );
    assert_eq!(stats.lock().expect("stats").submitted_ray_builds, 4);
    drop(game);
    std::fs::remove_dir_all(root).expect("remove owned cost fixture");
}

fn editable_valley(root: &Path) -> CookConfig {
    let authored = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/showcase");
    let source = root.join("source");
    std::fs::create_dir_all(&source).expect("owned valley fixture");
    for file in std::fs::read_dir(authored).expect("authored valley") {
        let file = file.expect("valley source entry");
        if file.file_type().expect("source kind").is_file() {
            std::fs::copy(file.path(), source.join(file.file_name())).expect("copy source");
        }
    }
    configuration(source, root.join("store"))
}

fn resident_delta(metal: &Metal, store: &Path, generation: u64) -> Value {
    let memory = || {
        wait(&metal.device);
        metal
            .caps
            .resident_device_bytes()
            .expect("Metal resident device size")
    };
    let (mut warm, warm_stats, _) = valley(store, metal, false, false);
    drop(settle(&mut warm, &warm_stats, generation));
    let base = memory();
    let (mut plain, plain_stats, _) = valley(store, metal, false, false);
    drop(settle(&mut plain, &plain_stats, generation));
    let plain_bytes = memory().checked_sub(base).expect("plain valley allocation");
    drop((plain, plain_stats));
    let base = memory();
    let (mut traced, traced_stats, _) = valley(store, metal, true, false);
    drop(settle(&mut traced, &traced_stats, generation));
    let ray_bytes = memory().checked_sub(base).expect("ray valley allocation");
    let delta = ray_bytes
        .checked_sub(plain_bytes)
        .expect("incremental ray allocation");
    assert!(delta > 0);
    json!({"type":"memory","plain_valley_device_bytes":plain_bytes,"ray_valley_device_bytes":ray_bytes,
        "incremental_resident_bytes":delta,"scope":"matched settled valley hosts, with the traced view prepared and ordinary SDR shown; no timestamp probe allocated"})
}

fn rendered_builds(
    game: &mut Host,
    metal: &Metal,
    stats: &Stats,
    goal: u64,
    probe: &RayBuildProbe,
    phase: &str,
) -> usize {
    let mut count = 0;
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        render_frame(game);
        let stats = stats.lock().expect("stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        let samples = probe.take_recorded();
        count += samples.len();
        for sample in &samples {
            let switched = stats.drawn == Some(goal);
            assert!(
                phase != "initial" || !switched,
                "mesh built in the first published frame"
            );
            report_build(
                metal,
                sample,
                phase,
                stats.submitted_frames,
                stats.drawn,
                switched,
            );
        }
        if stats.drawn == Some(goal) {
            return count;
        }
        assert!(
            Instant::now() < deadline,
            "valley did not publish generation {goal}"
        );
    }
}

fn report_build(
    metal: &Metal,
    sample: &thrum_render::synthetic::RayBuildSample,
    phase: &str,
    submission: u64,
    publication: Option<u64>,
    switched: bool,
) {
    let mut encoder = metal
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let buffer = sample.resolve(&metal.device, &mut encoder);
    metal.queue.submit([encoder.finish()]);
    let ticks = read(&metal.device, &buffer);
    let tick = |index: usize| u64::from_le_bytes(ticks.as_chunks::<8>().0[index]);
    let bracket = |end: usize| {
        tick(end)
            .checked_sub(tick(end - 1))
            .expect("ordered timestamp bracket") as f64
            * f64::from(metal.queue.get_timestamp_period())
    };
    let nanoseconds = bracket(1);
    let control = bracket(3);
    assert!(nanoseconds.is_finite() && nanoseconds > 0.0 && control.is_finite() && control > 0.0);
    report(
        &json!({"type":"mesh_build","phase":phase,"mesh":sample.mesh,"triangles":sample.triangles,
        "gpu_bracket_ns":nanoseconds,"control_bracket_ns":control,"estimated_mesh_build_ns":nanoseconds-control,
        "submission":submission,"displayed_publication":publication,"switch_in_this_frame":switched}),
    );
}
