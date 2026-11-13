//! CONTENT-LOOP leases exercised through the game's ordinary frame host.
#![cfg(target_os = "macos")]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use thrum_asset::{AssetStore, Ktx2, Profile};
use thrum_asset_gpu::{
    COPY_UNIT, CookedAssets, CookedConfig, CookedStats, LeaseRequest, ProductLease, STAGING_CHUNKS,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::{capabilities::DeviceCapabilities, contribution::SceneContributions};
use v1_game::{
    FrameConfig, FrameHost,
    lent::{LentDiagnostic, LentWitness},
};

fn presenting() -> FrameConfig {
    FrameConfig {
        surface: Some(thrum_render::present::SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: [640, 360],
        }),
        ..FrameConfig::default()
    }
}

fn fixture(root: &std::path::Path) -> CookConfig {
    let source = root.join("source");
    std::fs::create_dir_all(source.join("meshes")).expect("source");
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 8, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("foam.linear.png"), 32, [200, 40, 20]).expect("texture");
    for index in 0..12 {
        demo::write_cloud_volume(&source.join(format!("cloud{index}.tvol")), 128, index + 1)
            .expect("volume");
    }
    CookConfig {
        source,
        store: root.join("store"),
        profiles: vec![Profile::Portable, Profile::DesktopBc],
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

fn config(cook: &CookConfig) -> CookedConfig {
    let mut config = CookedConfig::new(cook.store.clone(), vec![]);
    config.upload_budget = COPY_UNIT;
    config.texture_preference = Some(vec![Profile::Portable]);
    config.poll_interval = Duration::from_millis(1);
    config.leased = (0..12)
        .map(|index| LeaseRequest {
            name: format!("cloud{index}"),
            dimension: wgpu::TextureDimension::D3,
        })
        .collect();
    config.leased.push(LeaseRequest {
        name: "foam".into(),
        dimension: wgpu::TextureDimension::D2,
    });
    config
}

async fn gpu(
    instance: &wgpu::Instance,
    generation: u64,
    compressed: bool,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Metal adapter");
    let features = if compressed {
        DeviceCapabilities::requested_content_features(&adapter)
    } else {
        wgpu::Features::empty()
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await
        .expect("device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
    (device, queue, capabilities)
}

fn finish(host: &mut FrameHost, device: &wgpu::Device) {
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("GPU complete");
    while host.poll_completed().expect("retire").is_some() {}
}

fn frame(host: &mut FrameHost, device: &wgpu::Device, clock: &mut Duration) {
    *clock += Duration::from_millis(16);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lent test drawable"),
        size: wgpu::Extent3d {
            width: 640,
            height: 360,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    host.drive(
        *clock,
        Some(&v1_game::SurfaceOutput {
            view: &view,
            extent: [640, 360],
        }),
    )
    .expect("frame");
    finish(host, device);
}

fn until(
    lease: &ProductLease,
    host: &mut FrameHost,
    device: &wgpu::Device,
    clock: &mut Duration,
    predicate: impl Fn() -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(90);
    while !predicate() {
        assert!(Instant::now() < deadline, "publication timed out");
        // Discard every upload recording, including the one carrying the last copy. None
        // may offer an unsubmitted set; the following ordinary frame re-records its copies.
        let before = lease.offered();
        *clock += Duration::from_millis(16);
        host.advance(*clock)
            .expect("advance discard")
            .encode_scene()
            .expect("encode discard")
            .skip();
        let after = lease.offered();
        assert!(
            match (&before, &after) {
                (None, None) => true,
                (Some(before), Some(after)) => Arc::ptr_eq(before, after),
                _ => false,
            },
            "discarded last-copy recording must keep the previous offer"
        );
        frame(host, device, clock);
    }
}

fn read(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &wgpu::Buffer) -> Vec<u8> {
    let target = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &target, 0, buffer.size());
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    target.map_async(wgpu::MapMode::Read, .., move |result| {
        tx.send(result).expect("map result");
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("map completion");
    rx.recv().expect("callback").expect("map");
    target.get_mapped_range(..).expect("mapped").to_vec()
}

fn assert_texels(
    cook: &CookConfig,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    witness: &Mutex<LentWitness>,
    compressed: bool,
) {
    let witness = witness.lock().expect("witness");
    let set = witness.offered.as_ref().expect("sampled set");
    assert_eq!(set.products.len(), 13);
    let store = AssetStore::new(&cook.store);
    let head = store.read_head().expect("HEAD").expect("published");
    let manifest = store.load_manifest(&head).expect("manifest");
    for (product, output) in set.products.iter().zip(&witness.outputs) {
        let portable = manifest
            .resolve(&product.name, &[Profile::Portable])
            .expect("portable");
        assert_eq!(product.portable_hash, portable.product);
        let reference =
            Ktx2::open(store.map(&product.portable_hash).expect("map product")).expect("ktx");
        let actual = read(device, queue, output);
        let volume = product.texture.dimension() == wgpu::TextureDimension::D3;
        if volume && compressed {
            assert_eq!(product.profile, Profile::DesktopBc);
        }
        let mut offset = 0;
        for (mip, level) in reference.levels().iter().enumerate() {
            let bytes = reference.level_data(level);
            let size = bytes.len() * if volume { 4 } else { 1 };
            let decoded = &actual[offset..offset + size];
            if volume && compressed {
                if mip == 0 {
                    let mse = decoded
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(bytes)
                        .map(|(a, b)| (f64::from(a[0]) - f64::from(*b)).powi(2))
                        .sum::<f64>()
                        / bytes.len() as f64;
                    let db = 10.0 * (255.0 * 255.0 / mse).log10();
                    assert!(db >= 30.0, "{}: {db} dB", product.name);
                }
            } else if volume {
                assert!(
                    decoded
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(bytes)
                        .all(|(a, b)| a[0] == *b),
                    "{} mip {mip}",
                    product.name
                );
            } else {
                assert_eq!(decoded, bytes, "{} mip {mip}", product.name);
            }
            offset += size;
        }
        assert_eq!(offset, actual.len());
    }
}

fn attach(
    host: &mut FrameHost,
    config: CookedConfig,
    consumer_first: bool,
) -> (
    ProductLease,
    Arc<Mutex<CookedStats>>,
    Arc<Mutex<LentWitness>>,
) {
    let assets = CookedAssets::new(config).expect("assets");
    let lease = assets.lease();
    let stats = assets.stats();
    let (consumer, witness) = LentDiagnostic::new(lease.clone());
    let entries: Vec<Box<dyn thrum_render::contribution::SceneContribution>> = if consumer_first {
        vec![Box::new(consumer), Box::new(assets)]
    } else {
        vec![Box::new(assets), Box::new(consumer)]
    };
    host.set_scene_contribution(Box::new(SceneContributions::new(entries)))
        .expect("attach");
    (lease, stats, witness)
}

#[test]
fn leased_texels_commit_edit_capture_discard_and_recovery() {
    for compressed in [false, true] {
        for consumer_first in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "thrum-lent-{}-{compressed}-{consumer_first}",
                std::process::id()
            ));
            std::fs::remove_dir_all(&root).ok();
            let cook = fixture(&root);
            let mut cooker = Cooker::open(cook.clone()).expect("cooker");
            assert_eq!(cooker.cook_all().expect("cook").failed, []);
            pollster::block_on(async {
                let instance =
                    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
                let (device, queue, capabilities) = gpu(&instance, 1, compressed).await;
                let mut host =
                    FrameHost::new_with_capabilities(&device, &queue, &capabilities, presenting())
                        .await
                        .expect("host");
                let (lease, stats, witness) = attach(&mut host, config(&cook), consumer_first);
                let mut clock = Duration::ZERO;
                assert!(lease.offered().is_none());
                until(&lease, &mut host, &device, &mut clock, || {
                    lease.offered().is_some()
                });
                assert_eq!(
                    stats.lock().expect("stats").drawn,
                    None,
                    "last-copy frame still shows old set"
                );
                clock += Duration::from_millis(16);
                host.advance(clock)
                    .expect("advance")
                    .encode_scene()
                    .expect("encode switch")
                    .skip();
                assert_eq!(
                    stats.lock().expect("stats").drawn,
                    None,
                    "discard commits nothing"
                );
                host.capture_pixels().expect("capture switch");
                assert_eq!(stats.lock().expect("stats").drawn, Some(1));
                assert!(
                    witness
                        .lock()
                        .expect("witness")
                        .invalidations
                        .iter()
                        .all(|n| *n == 0)
                );
                frame(&mut host, &device, &mut clock);
                assert!(
                    witness
                        .lock()
                        .expect("witness")
                        .changed
                        .iter()
                        .all(|changed| *changed)
                );
                assert_texels(&cook, &device, &queue, &witness, compressed);
                frame(&mut host, &device, &mut clock);
                assert!(
                    witness
                        .lock()
                        .expect("witness")
                        .changed
                        .iter()
                        .all(|changed| !changed)
                );
                let old = lease.offered().expect("old");
                let pipelines = host.pipeline_creation_count();
                let bind_groups = witness.lock().expect("witness").bind_groups_created;
                demo::write_cloud_volume(&cook.source.join("cloud0.tvol"), 128, 98).expect("edit");
                assert_eq!(cooker.cook_all().expect("recook").failed, []);
                until(&lease, &mut host, &device, &mut clock, || {
                    lease.offered().is_some_and(|set| set.generation == 2)
                });
                clock += Duration::from_millis(16);
                host.advance(clock)
                    .expect("advance")
                    .encode_scene()
                    .expect("edited switch")
                    .skip();
                assert_eq!(stats.lock().expect("stats").drawn, Some(1));
                host.capture_pixels().expect("edited capture");
                frame(&mut host, &device, &mut clock);
                assert_eq!(
                    witness.lock().expect("witness").changed,
                    [
                        true, false, false, false, false, false, false, false, false, false, false,
                        false, false
                    ]
                );
                let edited = lease.offered().expect("edited");
                let product = Ktx2::open(
                    AssetStore::new(&cook.store)
                        .map(&edited.products[0].hash)
                        .expect("edited product"),
                )
                .expect("edited KTX2");
                let expected_upload: u64 = product
                    .levels()
                    .iter()
                    .map(|level| {
                        u64::from(
                            level
                                .bytes_per_row(product.format())
                                .next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT),
                        ) * u64::from(level.rows_per_image(product.format()))
                            * u64::from(level.depth)
                    })
                    .sum();
                assert_eq!(
                    stats.lock().expect("stats").swaps[1].uploaded_bytes,
                    expected_upload,
                    "only the edited product's padded rows are uploaded"
                );
                assert_eq!(
                    witness.lock().expect("witness").bind_groups_created,
                    bind_groups + u64::from(edited.products[0].texture.mip_level_count())
                );
                assert_eq!(
                    host.pipeline_creation_count(),
                    pipelines,
                    "an edit creates no pipeline"
                );
                for (a, b) in old.products.iter().zip(&edited.products).skip(1) {
                    assert_eq!(a.hash, b.hash);
                    assert_eq!(a.serial, b.serial);
                    assert_eq!(a.texture, b.texture);
                    assert_eq!(a.view, b.view);
                }
                assert_ne!(old.products[0].hash, edited.products[0].hash);
                assert_texels(&cook, &device, &queue, &witness, compressed);
                frame(&mut host, &device, &mut clock);
                assert!(
                    witness
                        .lock()
                        .expect("witness")
                        .changed
                        .iter()
                        .all(|changed| !changed)
                );
                // No rendered frame between these publications: the staging ring cannot
                // hold all twelve volumes, so generation 3 is superseded while uploading.
                for (generation, seed) in [(3, 77), (4, 88)] {
                    for index in 0..12 {
                        demo::write_cloud_volume(
                            &cook.source.join(format!("cloud{index}.tvol")),
                            128,
                            seed + index,
                        )
                        .expect("superseding edit");
                    }
                    assert_eq!(cooker.cook_all().expect("superseding cook").failed, []);
                    let deadline = Instant::now() + Duration::from_secs(30);
                    while stats.lock().expect("stats").staged != Some(generation) {
                        assert!(
                            Instant::now() < deadline,
                            "loader never posts newer generation"
                        );
                        std::thread::park_timeout(Duration::from_millis(1));
                    }
                }
                until(&lease, &mut host, &device, &mut clock, || {
                    stats.lock().expect("stats").drawn == Some(4)
                });
                assert!(
                    !stats
                        .lock()
                        .expect("stats")
                        .swaps
                        .iter()
                        .any(|swap| swap.generation == 3)
                );
                {
                    let stats = stats.lock().expect("stats");
                    assert_eq!(stats.swaps.len(), 3);
                    assert!(stats.swaps[1].uploaded_bytes < stats.swaps[0].uploaded_bytes / 10);
                    assert!(stats.max_frame_copy_bytes <= COPY_UNIT);
                    assert!(stats.staging_peak_bytes <= COPY_UNIT * STAGING_CHUNKS);
                }
                let (new_device, new_queue, new_capabilities) = gpu(&instance, 2, compressed).await;
                let mut candidate = host
                    .device_prepare_inputs()
                    .expect("inputs")
                    .prepare(&new_capabilities, &new_queue, presenting().surface)
                    .await
                    .expect("replacement");
                device.destroy();
                host.publish_device(&mut candidate).expect("publish device");
                assert!(lease.offered().is_none());
                assert!(witness.lock().expect("witness").offered.is_none());
                until(&lease, &mut host, &new_device, &mut clock, || {
                    stats.lock().expect("stats").drawn_device_generation == Some(2)
                });
                frame(&mut host, &new_device, &mut clock);
                assert!(
                    lease
                        .offered()
                        .expect("restaged")
                        .products
                        .iter()
                        .all(|product| product.device_generation == 2)
                );
                assert_texels(&cook, &new_device, &new_queue, &witness, compressed);
            });
            std::fs::remove_dir_all(root).expect("cleanup");
        }
    }
}

#[test]
fn placement_and_lease_switch_together_in_both_contribution_orders() {
    for consumer_first in [false, true] {
        let root = std::env::temp_dir().join(format!(
            "thrum-lent-coupled-{}-{consumer_first}",
            std::process::id()
        ));
        std::fs::remove_dir_all(&root).ok();
        let source = root.join("source");
        std::fs::create_dir_all(source.join("meshes")).expect("source");
        std::fs::create_dir_all(source.join("textures")).expect("source");
        demo::write_grid_gltf(&source.join("meshes"), "terrain", 64, 0.25).expect("mesh");
        demo::write_checker_png(
            &source.join("textures/checker.linear.png"),
            1024,
            [200, 40, 20],
        )
        .expect("texture");
        let cook = CookConfig {
            source,
            store: root.join("store"),
            profiles: vec![Profile::Portable],
            quality: Quality::Fast,
            durability: Durability::Process,
        };
        let mut cooker = Cooker::open(cook.clone()).expect("cooker");
        assert_eq!(cooker.cook_all().expect("cook").failed, []);
        let configured = || {
            let mut config = v1_game::cooked::cooked_config(cook.store.clone());
            config.volume = None;
            config.meshes[0].texture = "textures/checker".into();
            config.upload_budget = COPY_UNIT;
            config.leased.push(LeaseRequest {
                name: "textures/checker".into(),
                dimension: wgpu::TextureDimension::D2,
            });
            config
        };
        pollster::block_on(async {
            let instance =
                wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
            let (device, queue, caps) = gpu(&instance, 1, false).await;
            let mut host = FrameHost::new_with_capabilities(&device, &queue, &caps, presenting())
                .await
                .expect("host");
            host.pause().expect("pause");
            let (lease, stats, witness) = attach(&mut host, configured(), consumer_first);
            let deadline = Instant::now() + Duration::from_secs(60);
            let old_pixels = loop {
                let pixels = host.capture_pixels().expect("first capture");
                if stats.lock().expect("stats").drawn == Some(1) {
                    break pixels;
                }
                assert!(Instant::now() < deadline, "first load");
            };
            let old_words = read(
                &device,
                &queue,
                &witness.lock().expect("witness").outputs[0],
            );
            demo::write_checker_png(
                &cook.source.join("textures/checker.linear.png"),
                1024,
                [20, 200, 100],
            )
            .expect("edit shared texture");
            assert_eq!(cooker.cook_all().expect("cook edit").failed, []);
            let mut frames = Vec::new();
            let mut discarded = false;
            let mut clock = Duration::ZERO;
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                if !discarded && lease.offered().is_some_and(|set| set.generation == 2) {
                    clock += Duration::from_millis(16);
                    host.advance(clock)
                        .expect("advance switch")
                        .encode_scene()
                        .expect("record switch")
                        .skip();
                    assert_eq!(stats.lock().expect("stats").drawn, Some(1));
                    discarded = true;
                }
                let pixels = host.capture_pixels().expect("edit frame");
                let words = read(
                    &device,
                    &queue,
                    &witness.lock().expect("witness").outputs[0],
                );
                frames.push((pixels, words));
                if stats.lock().expect("stats").drawn == Some(2) {
                    frames.push((
                        host.capture_pixels().expect("after switch"),
                        read(
                            &device,
                            &queue,
                            &witness.lock().expect("witness").outputs[0],
                        ),
                    ));
                    break;
                }
                assert!(Instant::now() < deadline, "edit never switches");
            }
            assert!(discarded, "switch recording was discarded");
            let mut fresh = FrameHost::new_with_capabilities(&device, &queue, &caps, presenting())
                .await
                .expect("fresh host");
            fresh.pause().expect("pause reference");
            let (_, fresh_stats, fresh_witness) = attach(&mut fresh, configured(), consumer_first);
            let deadline = Instant::now() + Duration::from_secs(60);
            let new_pixels = loop {
                let pixels = fresh.capture_pixels().expect("new reference");
                if fresh_stats.lock().expect("stats").drawn == Some(2) {
                    break pixels;
                }
                assert!(Instant::now() < deadline, "new reference never loads");
            };
            let new_words = read(
                &device,
                &queue,
                &fresh_witness.lock().expect("witness").outputs[0],
            );
            assert_ne!(
                old_pixels, new_pixels,
                "placement visibly samples the edited texture"
            );
            assert_ne!(
                old_words, new_words,
                "consumer visibly samples the edited texture"
            );
            for (index, (pixels, words)) in frames.iter().enumerate() {
                assert!(
                    (pixels == &old_pixels && words == &old_words)
                        || (pixels == &new_pixels && words == &new_words),
                    "frame {index}: mixed readers or partial publication"
                );
            }
            let stats = stats.lock().expect("shared upload stats");
            assert!(stats.max_frame_copy_bytes <= COPY_UNIT);
            assert!(stats.staging_peak_bytes <= COPY_UNIT * STAGING_CHUNKS);
        });
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
