//! Two simultaneous valley residencies on one ray-enabled device.

use super::*;
use thrum_render::synthetic::RayBuildProbe;

pub(crate) fn prove_demand() {
    let Some(metal) = Metal::new(1) else {
        return;
    };
    let store = valley_store("demand-valley");
    let (mut plain, plain_stats, _) = valley(&store, &metal, false, false);
    let (mut traced, traced_stats, _) = valley(&store, &metal, true, false);
    let generation = generation(&store);
    let plain_raster = settle(&mut plain, &plain_stats, generation);
    let traced_raster = settle(&mut traced, &traced_stats, generation);
    assert_eq!(plain_raster, traced_raster);
    if let Some(path) = std::env::var_os("THRUM_RAY_DEMAND_BYTES") {
        std::fs::write(path, &plain_raster).expect("write the submitted ordinary bytes");
    }
    assert!(plain.ray_structure_instances().is_none());
    assert!(traced.ray_structure_instances().is_some());
    assert_eq!(
        plain.capture_pixels().expect("still independent"),
        plain_raster
    );
    let ordinary = plain_stats.lock().expect("ordinary stats");
    let demanded = traced_stats.lock().expect("traced stats");
    assert_eq!(ordinary.submitted_ray_builds, 0);
    assert_eq!(demanded.submitted_ray_builds, 3);
    assert_eq!(ordinary.ray_mesh_storage.len(), 3);
    assert_eq!(demanded.ray_mesh_storage.len(), 3);
    for ((plain_name, plain, no_blas), (ray_name, ray, blas)) in ordinary
        .ray_mesh_storage
        .iter()
        .zip(&demanded.ray_mesh_storage)
    {
        use wgpu::BufferUsages as Usage;
        assert_eq!(plain_name, ray_name);
        assert_eq!(
            *plain,
            [
                Usage::VERTEX | Usage::COPY_DST,
                Usage::VERTEX | Usage::COPY_DST,
                Usage::INDEX | Usage::COPY_DST
            ]
        );
        assert_eq!(
            *ray,
            [
                plain[0] | Usage::BLAS_INPUT,
                plain[1],
                plain[2] | Usage::BLAS_INPUT
            ]
        );
        assert!(!no_blas && *blas);
    }
    let usage = |rows: &Stats| {
        rows.lock().expect("stats").ray_mesh_storage.iter().map(|(name, usages, blas)| {
        json!({"mesh":name, "position_attributes_index":usages.map(|usage| format!("{usage:?}")), "blas":blas})
    }).collect::<Vec<_>>()
    };
    drop((ordinary, demanded));
    report(
        &json!({"raster_digest":digest(&plain_raster), "ray_demand_off":usage(&plain_stats), "ray_demand_on":usage(&traced_stats)}),
    );
    drop((plain, traced));
    std::fs::remove_dir_all(store).expect("remove owned cooked fixture");
}

pub(super) fn valley_store(label: &str) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/showcase");
    let cook = configuration(source, scratch(label));
    let cooked = Cooker::open(cook.clone())
        .expect("valley cooker")
        .cook_all()
        .expect("valley cook");
    assert!(cooked.failed.is_empty(), "{:?}", cooked.failed);
    cook.store
}

pub(super) fn valley(
    store: &Path,
    metal: &Metal,
    ray_demand: bool,
    record_builds: bool,
) -> (Host, Stats, Option<RayBuildProbe>) {
    let setup = GameSetup {
        display_overlay: if ray_demand {
            DisplayOverlay::TracedGeometry
        } else {
            DisplayOverlay::Off
        },
        route_overlay: v1_game::RouteOverlay::Proof,
        ..GameSetup::default()
    };
    let caps = if record_builds {
        metal.caps.clone().with_ray_preparation_observation()
    } else {
        metal.caps.clone()
    };
    let mut game =
        pollster::block_on(setup.prepare(&metal.device, &metal.queue, &caps, frame_config()))
            .expect("valley host");
    let probe = record_builds.then(|| {
        game.observe_ray_mesh_builds()
            .expect("Metal build timestamps")
    });
    let (camera, assets) =
        v1_game::prepare_cooked_valley(store.to_owned().into()).expect("cooked valley");
    let stats = assets.stats();
    game.set_game_contribution(Box::new(assets))
        .expect("attach valley");
    game.set_scene_camera(camera)
        .expect("authored valley camera");
    game.set_display_overlay(DisplayOverlay::Off);
    game.set_time_of_day(0.5).expect("fixed noon");
    game.world_mut().pause().expect("frozen authority");
    (game, stats, probe)
}
