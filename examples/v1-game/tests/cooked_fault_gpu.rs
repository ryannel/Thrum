//! Real staging failure, whole-publication refusal, and recovery with bounded slots.
#![cfg(target_os = "macos")]

use std::time::{Duration, Instant};
use thrum_asset::Profile;
use thrum_asset_gpu::{COPY_UNIT, CookedAssets, FaultProbe, LeaseRequest, STAGING_CHUNKS};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use v1_game::{FrameConfig, FrameHost};

fn drive(
    host: &mut FrameHost,
    device: &wgpu::Device,
    clock: &mut Duration,
    predicate: impl Fn() -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !predicate() {
        assert!(Instant::now() < deadline, "no progress");
        *clock += Duration::from_millis(16);
        host.drive(*clock, None).expect("frame");
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("complete");
        while host.poll_completed().expect("retire").is_some() {}
        host.capture_pixels().expect("progress paused publication");
    }
}

#[test]
fn staging_fill_failure_frees_slot_retains_set_and_next_publication_recovers() {
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
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits {
                    max_texture_dimension_3d: 128,
                    ..wgpu::Limits::default()
                },
                ..wgpu::DeviceDescriptor::default()
            })
            .await
            .expect("device");
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
            .await
            .expect("host");
        let mut assets = v1_game::cooked::cooked_config(config.store.clone());
        assets.volume = None;
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
        drive(&mut host, &device, &mut clock, || {
            stats.lock().expect("stats").drawn == Some(1)
        });
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
        drive(&mut host, &device, &mut clock, || {
            stats.lock().expect("stats").drawn == Some(3)
        });
        host.pause().expect("pause comparisons");
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
                5 => demo::write_checker_png(&source.join("density.png"), 32, [10, 20, 30])
                    .expect("invalid dimension"),
                _ => {
                    std::fs::remove_file(source.join("density.png")).expect("remove invalid");
                    demo::write_cloud_volume(&source.join("density.tvol"), 256, 7)
                        .expect("device refused extent");
                }
            }
            assert_eq!(cooker.cook_all().expect("bad publication").failed, []);
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
        drive(&mut host, &device, &mut clock, || {
            stats.lock().expect("stats").drawn == Some(7)
        });
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
