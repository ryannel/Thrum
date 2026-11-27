//! Island surface affinity and Ground render products.
#[cfg(target_os = "macos")]
mod support;

thrum_test_support::macos_gpu_test! {
fn the_advected_water_row_keeps_the_island_shore_in_world_space() {
    use support::Fixture;
    use thrum_cloud_model::{Density, Field};
    use water_scene::{clouds::Selection, island::Wind};

    support::island::with_cooked_island("the_advected_water_row_keeps_the_island_shore_in_world_space", || {
    let Some(gpu) = thrum_test_support::gpu("sea fog stops", Some(wgpu::Backend::Metal)) else { return };
    let Some(selection) = Selection::from_query("?clouds=sea-fog&bank-view=inside&weather-speed=8&cloud-height=40") else {
        panic!("sea fog selection");
    };
    let scene = selection.build(false, true).expect("island Ground scene");
    let ground = scene.ground().expect("same CPU and GPU ground");
    let tap = scene.weather.expect("Ground weather");
    let map = water_scene::island::map(Wind::Trade).expect("rendered island map");
    let bed = map.bed();
    let (origin, spacing, size) = bed.lattice();
    assert_eq!(ground.surface.grid(), (origin, size, spacing));
    let [wet, dry] = [[-250.0, 0.0], [50.0, 40.0]];
    let [wet_node, dry_node] = [wet, dry].map(|point| ground.surface.sample(point));
    assert!(wet_node[2] > 0.0 && dry_node[2] < -ground.fog.shore_margin());
    let mut state = selection.state(Some(&ground)).expect("weather state");
    let (config, types) = model();
    let field = Field::new(39, config, types).expect("valid probe field");
    for tick in 1..=45 {
        state.step(tick, 1.0).expect("weather tick");
        let inland = [dry[0], dry_node[0] + 4.0, dry[1]];
        assert_eq!(ground.fog.density(&ground.surface, &field, inland, &state, Density::Uneroded).to_bits(), 0.0_f64.to_bits());
        if (20..=23).contains(&tick) {
            let water = [wet[0], wet_node[0] + 4.0, wet[1]];
            assert!(ground.fog.density(&ground.surface, &field, water, &state, Density::Uneroded) > 0.0);
        }
    }
    let extent = [64, 36];
    let depth = support::depth::DepthTap::default();
    let game = scene.game.with_contribution(Box::new(depth.contribution(extent)),
        || thrum_app::sky::ContentStatus::Drawn(1));
    let mut scene = Fixture::from_game(&gpu, extent, game, scene.report);
    scene.warm_cooked();
    scene.frame_without_timings(0.2);
    let products = tap.ground_condensate_products().expect("ground-band and near medium");
    let coefficients = thrum_test_support::readback::texture(&scene.device, &scene.queue,
        &products.medium.coefficients, thrum_test_support::readback::Mip::level(0), ..);
    assert!(coefficients.as_chunks::<8>().0.iter().any(|pixel|
        support::half_value(u16::from_le_bytes([pixel[0], pixel[1]])) > 0.0));
    let sky = tap.sky_products().expect("submitted Ground layer");
    assert_eq!(sky.fresh_front.size().depth_or_array_layers, 4);
    assert_stops(&scene, &sky, &depth.read(&scene), selection);
    });
}
}

thrum_test_support::macos_gpu_test! {
fn accepted_lake_stage_updates_the_weather_surface_at_a_held_tick() {
    use support::Fixture;
    use water_scene::{clouds::Selection, island::Wind};

    support::island::with_cooked_island("accepted_lake_stage_updates_the_weather_surface_at_a_held_tick", || {
    let Some(gpu) = thrum_test_support::gpu("lake stage and weather", Some(wgpu::Backend::Metal)) else { return };
    let selection = Selection::from_query("?clouds=dawn-mist").expect("dawn mist");
    let scene = selection.build(false, true).expect("lake scene");
    let weather = scene.weather.expect("live Ground");
    let mut fixture = Fixture::from_game(&gpu, [64, 36], scene.game, scene.report);
    let mut map = water_scene::island::map(Wind::Trade).expect("island water authority");
    let mut before = weather.ground_surface().expect("initial surface");
    for (stage, json) in [(0.0, "{}"), (0.35, r#"{"lake.stage":0.35}"#),
        (1.0, r#"{"lake.stage":1.0}"#), (0.0, r#"{"lake.stage":0.0}"#)] {
        map.set_stage(1, stage).expect("accepted stage");
        fixture.parameters(json);
        assert_eq!(fixture.frame_without_timings(0.0).tick, 0);
        let current = weather.ground_surface().expect("published surface");
        let lake = [180.0, 5.0];
        let water = map.sample(lake).expect("water row").row.level;
        let [height, _, shore] = current.sample(lake);
        assert!((height - water).abs() < 1e-6 && shore > 0.0);
        eprintln!("Metal held tick 0, lake stage {stage:.2}, water {water:.2}, Ground {height:.2}, shore {shore:.2}");
        if stage != 0.0 {
            assert!(!std::sync::Arc::ptr_eq(&before, &current), "stage must replace the surface");
        }
        fixture.parameters(json);
        assert!(std::sync::Arc::ptr_eq(&current, &weather.ground_surface().expect("retained surface")));
        before = current;
    }
    });
}
}

#[cfg(target_os = "macos")]
fn model() -> (thrum_cloud_model::Profile, Vec<thrum_cloud_model::Type>) {
    (
        thrum_cloud_model::Profile {
            pitch: 4600.0,
            turret_pitch: 1300.0,
            turret_least: 0.22,
            turret_fade: 0.12,
            period: 1800.0,
            detail_mean: 0.5,
            erosion: 0.61,
        },
        vec![
            thrum_cloud_model::Type {
                base: 1600.0,
                depth: [465.0, 680.0],
                liquid_water: 0.000_350,
                droplet_radius: 10e-6,
                deck: 1.0,
                profile: [0.65, 0.19],
            };
            2
        ],
    )
}

#[cfg(target_os = "macos")]
#[expect(
    clippy::cast_precision_loss,
    reason = "bounded test target coordinates"
)]
fn assert_stops(
    scene: &support::Fixture,
    sky: &thrum_clouds::SkyProducts,
    depth: &[f32],
    selection: water_scene::clouds::Selection,
) {
    use thrum_render::runtime::{FrameWorld, SceneSource};
    use thrum_test_support::readback::{self, Mip};

    let texture =
        |texture| readback::texture(&scene.device, &scene.queue, texture, Mip::level(0), ..);
    let front = texture(&sky.fresh_front);
    let transport = texture(&sky.fresh_transport);
    let layer = sky.fresh_front.size();
    let [width, height] = scene.extent;
    let inverse = glam::Mat4::from_cols_array_2d(&sky.raster_view_projection).inverse();
    let view = scene
        .host
        .source()
        .frame(scene.host.world().source(), 1.0, scene.extent, None)
        .expect("same camera")
        .view;
    let camera = glam::Vec3::from_array(view.camera_position());
    let read_half = |bytes: &[u8], index: usize| {
        support::half_value(u16::from_le_bytes([bytes[index], bytes[index + 1]]))
    };
    let mut clear_near = 0;
    let mut fog_before_water = 0;
    for y in height / 2..height {
        for x in 0..width {
            let z = depth[(y * width + x) as usize];
            if z <= 0.0 {
                continue;
            }
            let ndc = [
                2.0 * (x as f32 + 0.5) / width as f32 - 1.0,
                1.0 - 2.0 * (y as f32 + 0.5) / height as f32,
                z,
            ];
            let expected = inverse
                .project_point3(glam::Vec3::from_array(ndc))
                .distance(camera);
            let slot = x % 2 + 2 * (y % 2);
            let offset = (((slot * layer.height + y / 2) * layer.width + x / 2) * 8 + 6) as usize;
            let actual = read_half(&front, offset) * 1000.0;
            assert!(
                (actual - expected).abs() < (0.004 * expected).max(0.04),
                "{selection:?} at {x},{y}: {actual} m vs written depth {expected} m"
            );
            let transmission = read_half(&transport, offset);
            clear_near += usize::from(expected < 100.0 && transmission > 0.999);
            fog_before_water += usize::from(expected > 300.0 && transmission < 0.95);
        }
    }
    assert!(
        clear_near > 100,
        "near water must stop before the far integrator"
    );
    assert!(
        fog_before_water > 1,
        "distant water must keep the fog ahead of it"
    );
}
