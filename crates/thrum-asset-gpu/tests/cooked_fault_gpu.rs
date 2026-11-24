//! Real staging failure, whole-publication refusal, and recovery with bounded slots.

#![cfg(feature = "fault-probe")]

mod static_scene;

use static_scene::{Source, World};
use std::time::Duration;
use thrum_asset::Profile;
use thrum_asset_gpu::{
    COPY_UNIT, CookedAssets, CookedConfig, Drawn, FaultProbe, LeaseRequest, MeshPlacement,
    STAGING_CHUNKS, await_drawn,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::runtime::{FrameConfig, FrameHost};
use thrum_render::synthetic::{SyntheticScene, SyntheticView};

type Host = FrameHost<World, Source>;

fn drive(
    host: &mut Host,
    device: &wgpu::Device,
    clock: &mut Duration,
    predicate: impl Fn() -> bool,
) {
    thrum_test_support::wait::until("staging result", Duration::from_secs(30), || {
        if predicate() {
            return Some(());
        }
        *clock += Duration::from_millis(16);
        host.drive(*clock, None).expect("frame");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("complete");
        while host.poll_completed().expect("retire").is_some() {}
        // Render the endpoint without a readback, so a paused publication still progresses.
        let submitted = host.render_once().expect("progress paused publication");
        host.device_capabilities()
            .site()
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("endpoint frame completes");
        None
    });
}

#[test]
fn staging_fill_failure_frees_slot_retains_set_and_next_publication_recovers() {
    use thrum_render::capabilities::{
        DeviceProfile, DeviceRecipe, DeviceRestriction, OpenedDevice,
    };
    let Some(gpu) = thrum_test_support::gpu(
        "staging_fill_failure_frees_slot_retains_set_and_next_publication_recovers",
        Some(wgpu::Backend::Metal),
    ) else {
        return;
    };
    let root = std::env::temp_dir().join(format!("thrum-lent-fault-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let source = root.join("source");
    std::fs::create_dir_all(&source).expect("source");
    std::fs::create_dir_all(source.join("meshes")).expect("meshes");
    std::fs::create_dir_all(source.join("textures")).expect("textures");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 64, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("textures/checker.png"), 64, [200, 40, 20])
        .expect("texture");
    demo::write_cloud_volume(&source.join("density.tvol"), 128, 1).expect("volume");
    let config = CookConfig {
        source: source.clone(),
        store: root.join("store"),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    };
    let mut cooker = Cooker::open(config.clone()).expect("cooker");
    assert_eq!(cooker.cook_all().expect("cook").failed, []);
    let store = thrum_asset::AssetStore::new(&config.store);
    let original = store
        .load_manifest(&store.read_head().expect("HEAD").expect("published"))
        .expect("manifest");
    let resident_texture = original
        .resolve("textures/checker", &[Profile::Portable])
        .expect("placement texture")
        .product;
    pollster::block_on(async {
        let OpenedDevice {
            device,
            queue,
            capabilities,
            ..
        } = DeviceRecipe {
            profile: DeviceProfile::Portable,
            restriction: Some(&DeviceRestriction {
                limits: Some(wgpu::Limits {
                    max_texture_dimension_3d: 128,
                    ..wgpu::Limits::default()
                }),
                features: Some(wgpu::Features::empty()),
                ..Default::default()
            }),
            ..Default::default()
        }
        .open_admitted(&gpu.instance, &gpu.adapter)
        .await
        .expect("restricted Metal device");
        let view = SyntheticView::perspective(
            [0.0, 2.0, 3.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            0.92,
            1.0,
            0.1,
            30.0,
        )
        .expect("camera");
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = view;
        scene.instances[0].deactivate();
        let mut host = FrameHost::new(
            &capabilities,
            &queue,
            FrameConfig::offscreen([64; 2]),
            World,
            Source(view),
            scene,
            1.0,
        )
        .await
        .expect("host");
        let mut assets = CookedConfig::new(
            config.store.clone(),
            vec![MeshPlacement::new("meshes/terrain", "textures/checker")],
        );
        assets.upload_budget = COPY_UNIT;
        assets.poll_interval = Duration::from_millis(1);
        assets.leased.push(LeaseRequest {
            name: "density".into(),
            dimension: wgpu::TextureDimension::D3,
        });
        let fault = FaultProbe::default();
        assets.fault = Some(fault.clone());
        let assets = CookedAssets::new(assets).expect("assets");
        let lease = assets.lease();
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        await_drawn(&mut host, &stats, Drawn::Generation(1))
            .unwrap_or_else(|error| panic!("generation 1: {error}"));
        let old = lease.offered().expect("old");
        fault.fail_next_fill();
        demo::write_cloud_volume(&source.join("density.tvol"), 128, 2).expect("edit");
        assert_eq!(cooker.cook_all().expect("cook failed upload").failed, []);
        drive(&mut host, &device, &mut clock, || {
            stats.lock().expect("stats").failures == 1
        });
        assert_eq!(stats.lock().expect("stats").drawn, Some(1));
        assert_eq!(
            lease.offered().expect("retained").products[0].view,
            old.products[0].view
        );
        assert!(stats.lock().expect("stats").errors[0].contains("staging map"));
        demo::write_cloud_volume(&source.join("density.tvol"), 128, 3).expect("good edit");
        assert_eq!(cooker.cook_all().expect("recovery cook").failed, []);
        await_drawn(&mut host, &stats, Drawn::Generation(3))
            .unwrap_or_else(|error| panic!("generation 3: {error}"));
        let previous_pixels = host.capture_pixels().expect("last good pixels");
        let previous_lease = lease.offered().expect("last good lease");
        for generation in 4..=6 {
            demo::write_checker_png(
                &source.join("textures/checker.png"),
                64,
                [20, 100 + generation, 200],
            )
            .expect("coupled texture edit");
            match generation {
                4 => {
                    std::fs::remove_file(source.join("density.tvol")).expect("missing leased name");
                }
                // The invalid leased product is already resident through a placement.
                // Reuse must validate its new use too, before refusing the whole set.
                5 => demo::write_checker_png(&source.join("density.png"), 64, [200, 40, 20])
                    .expect("cached invalid dimension"),
                _ => {
                    std::fs::remove_file(source.join("density.png")).expect("remove invalid");
                    demo::write_cloud_volume(&source.join("density.tvol"), 256, 7)
                        .expect("device refused extent");
                }
            }
            assert_eq!(cooker.cook_all().expect("bad publication").failed, []);
            if generation == 5 {
                let manifest = store
                    .load_manifest(&store.read_head().expect("HEAD").expect("published"))
                    .expect("invalid publication manifest");
                assert_eq!(
                    manifest
                        .resolve("density", &[Profile::Portable])
                        .expect("density")
                        .product,
                    resident_texture,
                    "the invalid lease exercises the resident texture reuse path"
                );
            }
            let expected = u64::from(generation - 2);
            drive(&mut host, &device, &mut clock, || {
                stats.lock().expect("stats").failures == expected
            });
            assert_eq!(stats.lock().expect("stats").drawn, Some(3));
            assert_eq!(
                lease.offered().expect("retained lease").products[0].view,
                previous_lease.products[0].view
            );
            assert_eq!(
                host.capture_pixels().expect("refused pixels"),
                previous_pixels
            );
            assert!(
                stats
                    .lock()
                    .expect("stats")
                    .errors
                    .last()
                    .expect("reason")
                    .contains("lease density")
            );
        }
        demo::write_cloud_volume(&source.join("density.tvol"), 128, 8).expect("fixed extent");
        assert_eq!(cooker.cook_all().expect("fixed cook").failed, []);
        await_drawn(&mut host, &stats, Drawn::Generation(7))
            .unwrap_or_else(|error| panic!("generation 7: {error}"));
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.failures, 4);
        assert_eq!(
            stats
                .swaps
                .iter()
                .map(|swap| swap.generation)
                .collect::<Vec<_>>(),
            [1, 3, 7]
        );
        assert!(stats.staging_peak_bytes <= COPY_UNIT * STAGING_CHUNKS);
        assert!(stats.max_frame_copy_bytes <= COPY_UNIT);
    });
    std::fs::remove_dir_all(root).expect("cleanup");
}
