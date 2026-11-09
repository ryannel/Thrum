//! G5 in the game: cooked mesh, texture and volume drawn in the V1 scene with its selected G3s
//! lighting, uploaded under a per-frame budget, and replaced live as sources are edited.
#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thrum_asset::Profile;
use thrum_asset_gpu::{
    COPY_UNIT, CookedAssets, CookedConfig, CookedDrawPath, CookedStats, STAGING_CHUNKS,
};
use thrum_cook::{CookConfig, Cooker, Durability, Quality, demo};
use thrum_render::capabilities::DeviceCapabilities;
use thrum_render::present::SurfaceProfile;
use v1_game::{FrameConfig, FrameHost, FrameSample, SurfaceOutput};

const EXTENT: [u32; 2] = [640, 360];

/// Upload budget small enough that every product here spans several frames.
const BUDGET: u64 = COPY_UNIT;

/// Staging may never hold more than this, whatever a publication's size.
const STAGING_BOUND: u64 = STAGING_CHUNKS * BUDGET;

fn assert_staging_bounded(stats: &CookedStats) {
    #[cfg(feature = "draw-probe")]
    if stats.draw_path == Some(CookedDrawPath::Direct) {
        assert_eq!(stats.probe_resources, (0, false));
    }
    assert!(
        stats.staging_peak_bytes > 0 && stats.staging_peak_bytes <= STAGING_BOUND,
        "staging peaked at {} bytes against a bound of {STAGING_BOUND}",
        stats.staging_peak_bytes
    );
}

fn transform(scale: f32, rotation: glam::Quat, at: [f32; 3]) -> [[f32; 4]; 4] {
    glam::Mat4::from_scale_rotation_translation(glam::Vec3::splat(scale), rotation, at.into())
        .to_cols_array_2d()
}

/// The game's own placement of the demo products, with a budget that forces multi-frame uploads.
fn placements(store: PathBuf) -> CookedConfig {
    let mut config = v1_game::cooked::cooked_config(store);
    config.upload_budget = BUDGET;
    config
}

fn source_tree(root: &Path, volume_size: u32) -> CookConfig {
    let source = root.join("source");
    for directory in ["meshes", "textures"] {
        std::fs::create_dir_all(source.join(directory)).expect("source tree");
    }
    demo::write_grid_gltf(&source.join("meshes"), "terrain", 128, 0.25).expect("mesh");
    demo::write_checker_png(&source.join("textures/checker.png"), 512, [200, 40, 40])
        .expect("texture");
    demo::write_cloud_volume(&source.join("cloud.tvol"), volume_size, 1).expect("volume");
    CookConfig {
        source,
        store: root.join("store"),
        profiles: Profile::ALL.to_vec(),
        quality: Quality::Fast,
        durability: Durability::Process,
    }
}

fn wait_for(
    host: &mut FrameHost,
    device: &wgpu::Device,
    stats: &Mutex<CookedStats>,
    generation: u64,
    clock: &mut Duration,
) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut frames = 0;
    loop {
        if stats.lock().expect("stats").drawn == Some(generation) {
            return frames;
        }
        let errors = stats.lock().expect("stats").errors.clone();
        assert!(errors.is_empty(), "loader errors: {errors:?}");
        assert!(
            Instant::now() < deadline,
            "generation {generation} never drawn"
        );
        *clock += Duration::from_millis(16);
        host.drive(*clock, None).expect("frame");
        frames += 1;
        while host.pending_frames() >= 2 {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .expect("GPU wait");
            host.poll_completed().expect("GPU progress");
        }
    }
}

async fn game_device(path: CookedDrawPath) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real adapter required for the in-game G5 proof");
    // Ask for every compressed family the adapter offers, as the game's platform layer does.
    // Timestamps too, so contributed frames exercise the renderer's timing collector.
    let features = DeviceCapabilities::requested_content_features(&adapter)
        | DeviceCapabilities::requested_timing_features(&adapter);
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: features,
            ..Default::default()
        })
        .await
        .expect("device");
    let caps = selected_capabilities(&adapter, &device, 1, path);
    (device, queue, caps)
}

fn changed_pixels(a: &[u8], b: &[u8]) -> usize {
    a.as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(x, y)| x.abs_diff(*y) > 12))
        .count()
}

fn save(name: &str, pixels: &[u8]) {
    if let Some(directory) = std::env::var_os("THRUM_G5_GAME_CAPTURE_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("capture directory");
        let mut ppm = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
        for pixel in pixels.as_chunks::<4>().0 {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(directory.join(format!("{name}.ppm")), ppm).expect("capture");
    }
}

#[test]
fn cooked_edits_become_visible_in_the_game_with_its_lighting() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        cooked_edits_become_visible_in_the_game_with_its_lighting_case(path);
    }
}

fn cooked_edits_become_visible_in_the_game_with_its_lighting_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-game-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 96);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    let report = cooker.cook_all().expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("prepared game scene");
        let before = host.capture_pixels().expect("game before cooked assets");
        save("before", &before);

        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats: Arc<Mutex<CookedStats>> = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("cooked assets attach to the game scene");
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        let first = host.capture_pixels().expect("game with cooked assets");
        save("cooked", &first);
        {
            let stats = stats.lock().expect("stats");
            let swap = &stats.swaps[0];
            assert!(
                swap.upload_frames > 1,
                "a multi-megabyte set must spread over frames: {swap:?}"
            );
            assert!(
                stats.max_frame_copy_bytes <= BUDGET,
                "per-frame copies stay within budget: {}",
                stats.max_frame_copy_bytes
            );
        }
        let cooked_area = changed_pixels(&before, &first);
        assert!(
            cooked_area > 640 * 360 / 50,
            "cooked mesh and volume are visible in the game: {cooked_area} pixels changed"
        );

        // Texture edit: only the texture recooks and uploads; the mesh is reused.
        demo::write_checker_png(
            &cook.source.join("textures/checker.png"),
            512,
            [40, 60, 210],
        )
        .expect("edit texture");
        let report = cooker
            .cook_changed(&[PathBuf::from("textures/checker.png")])
            .expect("recook");
        let generation = report.published.expect("published").generation;
        wait_for(&mut host, &device, &stats, generation, &mut clock);
        let recoloured = host.capture_pixels().expect("after texture edit");
        save("texture-edit", &recoloured);
        assert!(
            changed_pixels(&first, &recoloured) > 500,
            "texture edit is visible"
        );

        // Volume edit: a different density field.
        demo::write_cloud_volume(&cook.source.join("cloud.tvol"), 96, 7).expect("edit volume");
        let report = cooker
            .cook_changed(&[PathBuf::from("cloud.tvol")])
            .expect("recook");
        let generation = report.published.expect("published").generation;
        wait_for(&mut host, &device, &stats, generation, &mut clock);
        let reshaped = host.capture_pixels().expect("after volume edit");
        save("volume-edit", &reshaped);
        assert!(
            changed_pixels(&recoloured, &reshaped) > 200,
            "volume edit is visible"
        );

        // A short sequence for visual review: the world keeps moving around the cooked content.
        if std::env::var_os("THRUM_G5_GAME_CAPTURE_DIR").is_some() {
            for index in 0..6 {
                for _ in 0..6 {
                    clock += Duration::from_millis(16);
                    host.drive(clock, None).expect("frame");
                    while host.pending_frames() > 0 {
                        device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: Some(Duration::from_secs(10)),
                            })
                            .expect("GPU wait");
                        host.poll_completed().expect("GPU progress");
                    }
                }
                save(
                    &format!("sequence-{index}"),
                    &host.capture_pixels().expect("sequence frame"),
                );
            }
        }
        let stats = stats.lock().expect("stats");
        let swaps = &stats.swaps;
        assert_eq!(swaps.len(), 3);
        // Reuse: the texture edit uploads no mesh or volume bytes, the volume edit no texture.
        assert!(swaps[1].uploaded_bytes < swaps[0].uploaded_bytes);
        assert!(swaps[2].uploaded_bytes < swaps[0].uploaded_bytes);
        assert!(
            swaps
                .iter()
                .all(|swap| swap.texture_profile.is_some() && swap.volume_profile.is_some())
        );
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn off_screen_meshlets_are_culled_and_bc7_is_drawn_where_supported() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        off_screen_meshlets_are_culled_and_bc7_is_drawn_where_supported_case(path);
    }
}

fn off_screen_meshlets_are_culled_and_bc7_is_drawn_where_supported_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-cull-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 32);
    Cooker::open(cook.clone())
        .expect("cooker")
        .cook_all()
        .expect("cook");
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let bc = device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("prepared game scene");
        let mut config = placements(cook.store.clone());
        config.volume = None;
        // Straddle the left edge of the view: about half the meshlets are outside it.
        config.meshes[0].transform = transform(
            0.6,
            glam::Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 * 0.8),
            [-1.25, -0.1, 0.9],
        );
        config.texture_preference = Some(vec![Profile::DesktopBc]);
        let assets = CookedAssets::new(config).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        let deadline = Instant::now() + Duration::from_secs(10);
        if path == CookedDrawPath::Indirect {
            let (visible, total) = loop {
                if let Some(counts) = stats.lock().expect("stats").visible_meshlets {
                    break counts;
                }
                assert!(Instant::now() < deadline, "no cull readback");
                clock += Duration::from_millis(16);
                host.drive(clock, None).expect("frame");
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .expect("GPU wait");
                host.poll_completed().expect("GPU progress");
            };

            assert!(
                visible > total / 5 && visible < total * 4 / 5,
                "frustum culling keeps only the on-screen part: {visible} of {total}"
            );
        } else {
            assert_eq!(stats.lock().expect("stats").visible_meshlets, None);
        }
        save("cull-bc7", &host.capture_pixels().expect("pixels"));
        let profile = stats.lock().expect("stats").swaps[0].texture_profile;
        if bc {
            assert_eq!(
                profile,
                Some(Profile::DesktopBc),
                "BC7 drawn on a BC device"
            );
        } else {
            assert_eq!(profile, Some(Profile::Portable));
        }
    });
    std::fs::remove_dir_all(&root).ok();
}

/// Wait, without rendering, until the loader has staged `generation`.
fn wait_staged(stats: &Mutex<CookedStats>, generation: u64) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while stats.lock().expect("stats").staged != Some(generation) {
        assert!(
            Instant::now() < deadline,
            "generation {generation} never staged"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Drive every host on the same clock, one submitted frame each, until all reached `generation`.
fn lockstep(
    hosts: &mut [(FrameHost, Arc<Mutex<CookedStats>>)],
    device: &wgpu::Device,
    output: &SurfaceOutput<'_>,
    clock: &mut Duration,
    generation: u64,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while hosts
        .iter()
        .any(|(_, stats)| stats.lock().expect("stats").drawn != Some(generation))
    {
        assert!(
            Instant::now() < deadline,
            "generation {generation} never drawn"
        );
        *clock += Duration::from_millis(16);
        for (host, _) in hosts.iter_mut() {
            assert!(host.drive(*clock, Some(output)).expect("frame").submitted);
            settle(host, device);
        }
    }
}

fn settle(host: &mut FrameHost, device: &wgpu::Device) {
    while host.pending_frames() > 0 {
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
        host.poll_completed().expect("GPU progress");
    }
}

/// Recording is not publication: frames encoded and then discarded (no drawable) must leave
/// upload progress, residency and the drawn set exactly as they were. Two hosts on one device
/// follow the same clock; one encodes and discards frames mid-upload while the other skips
/// rendering. Both must then swap on the same submitted frame and draw identical pixels.
#[test]
fn discarded_frames_leave_no_trace_on_cooked_uploads() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        discarded_frames_leave_no_trace_on_cooked_uploads_case(path);
    }
}

fn discarded_frames_leave_no_trace_on_cooked_uploads_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-discard-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 160);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        // Discarding happens when a drawable is not acquired, so these hosts present.
        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("discard test drawable"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
        let output = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        let config = FrameConfig {
            surface: Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: EXTENT,
            }),
            ..FrameConfig::default()
        };
        let mut hosts = Vec::new();
        for _ in 0..2 {
            let mut host = FrameHost::new_with_capabilities(&device, &queue, &capabilities, config)
                .await
                .expect("prepared game scene");
            let assets =
                CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
            let stats = assets.stats();
            host.set_scene_contribution(Box::new(assets))
                .expect("attach");
            assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
            hosts.push((host, stats));
        }
        let mut clock = Duration::ZERO;
        // Both hosts upload and draw generation 1 on the same frames.
        for (_, stats) in &hosts {
            wait_staged(stats, 1);
        }
        lockstep(&mut hosts, &device, &output, &mut clock, 1);
        // A multi-frame volume edit, staged on both before either renders again.
        demo::write_cloud_volume(&cook.source.join("cloud.tvol"), 160, 9).expect("edit volume");
        let generation = cooker
            .cook_changed(&[PathBuf::from("cloud.tvol")])
            .expect("recook")
            .published
            .expect("published")
            .generation;
        for (_, stats) in &hosts {
            wait_staged(stats, generation);
        }
        // One submitted frame commits the first part of the upload on both hosts, so the
        // discards below land mid-upload, after committed progress.
        clock += Duration::from_millis(16);
        for (host, _) in &mut hosts {
            assert!(host.drive(clock, Some(&output)).expect("frame").submitted);
            settle(host, &device);
        }
        for (_, stats) in &hosts {
            let stats = stats.lock().expect("stats");
            assert_eq!(stats.drawn, Some(1), "the edit is still uploading");
            assert!(
                stats.last_frame_copy_bytes > 0,
                "part of the edit was committed"
            );
        }
        let submitted_before = hosts[0].1.lock().expect("stats").submitted_frames;
        for _ in 0..12 {
            clock += Duration::from_millis(16);
            let (discarding, discarding_stats) = &mut hosts[0];
            let recorded = discarding_stats.lock().expect("stats").recorded_frames;
            let encoded = discarding
                .advance(clock)
                .expect("advance")
                .encode_scene()
                .expect("encode");
            assert!(
                discarding_stats.lock().expect("stats").recorded_frames > recorded,
                "the contribution recorded into the discarded frame"
            );
            assert!(
                encoded.skip().dropped_view,
                "the encoded frame was discarded"
            );
            hosts[1]
                .0
                .advance_without_rendering(clock)
                .expect("advance");
        }
        for (_, stats) in &hosts {
            let stats = stats.lock().expect("stats");
            assert_eq!(stats.drawn, Some(1), "discarded frames swap nothing in");
        }
        assert_eq!(
            hosts[0].1.lock().expect("stats").submitted_frames,
            submitted_before,
            "nothing was submitted while discarding"
        );
        // Then discard before every submitted frame, so the frame recording the final copies
        // (the swap) is also discarded once before it is submitted.
        let deadline = Instant::now() + Duration::from_secs(30);
        while hosts
            .iter()
            .any(|(_, stats)| stats.lock().expect("stats").drawn != Some(generation))
        {
            assert!(
                Instant::now() < deadline,
                "generation {generation} never drawn"
            );
            clock += Duration::from_millis(16);
            let drawn = hosts[0].1.lock().expect("stats").drawn;
            assert!(
                hosts[0]
                    .0
                    .advance(clock)
                    .expect("advance")
                    .encode_scene()
                    .expect("encode")
                    .skip()
                    .dropped_view
            );
            assert_eq!(
                hosts[0].1.lock().expect("stats").drawn,
                drawn,
                "a discarded swap frame publishes nothing"
            );
            hosts[1]
                .0
                .advance_without_rendering(clock)
                .expect("advance");
            clock += Duration::from_millis(16);
            for (host, _) in &mut hosts {
                assert!(host.drive(clock, Some(&output)).expect("frame").submitted);
                settle(host, &device);
            }
        }
        let swaps: Vec<_> = hosts
            .iter()
            .map(|(_, stats)| {
                let stats = stats.lock().expect("stats");
                let swap = stats.swaps.last().expect("swap").clone();
                (swap.upload_frames, swap.uploaded_bytes)
            })
            .collect();

        assert!(swaps[0].0 > 1, "the edit spans frames: {swaps:?}");
        assert_eq!(swaps[0], swaps[1], "discarded frames advanced no upload");
        let pixels: Vec<_> = hosts
            .iter_mut()
            .map(|(host, _)| host.capture_pixels().expect("pixels"))
            .collect();
        save("discard-retry", &pixels[0]);
        assert!(
            pixels[0] == pixels[1],
            "retried uploads draw byte-identical frames"
        );
    });
    std::fs::remove_dir_all(&root).ok();
}

fn drawable(device: &wgpu::Device) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("cooked test drawable"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

fn presenting() -> FrameConfig {
    FrameConfig {
        surface: Some(SurfaceProfile {
            format: wgpu::TextureFormat::Bgra8Unorm,
            extent: EXTENT,
        }),
        ..FrameConfig::default()
    }
}

/// Drive one presented frame to completion; return its report.
fn presented_frame(
    host: &mut FrameHost,
    device: &wgpu::Device,
    view: &wgpu::TextureView,
    clock: &mut Duration,
) -> FrameSample {
    *clock += Duration::from_millis(16);
    let output = SurfaceOutput {
        view,
        extent: EXTENT,
    };
    assert!(host.drive(*clock, Some(&output)).expect("frame").submitted);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(sample) = host.poll_completed().expect("GPU progress") {
            return sample;
        }
        assert!(Instant::now() < deadline, "frame never completed");
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(10)),
            })
            .expect("GPU wait");
    }
}

/// Frames until the next swap, checking every completed frame's timing and the history flag.
fn until_swap(
    host: &mut FrameHost,
    device: &wgpu::Device,
    view: &wgpu::TextureView,
    stats: &Mutex<CookedStats>,
    clock: &mut Duration,
) {
    let swaps = stats.lock().expect("stats").swaps.len();
    let deadline = Instant::now() + Duration::from_secs(30);
    while stats.lock().expect("stats").swaps.len() == swaps {
        assert!(Instant::now() < deadline, "no swap");
        timed(&presented_frame(host, device, view, clock), device, false);
    }
    let stats = stats.lock().expect("stats");
    assert!(stats.errors.is_empty(), "loader errors: {:?}", stats.errors);
    assert_eq!(
        stats.last_frame_history,
        Some(false),
        "a new set's first frame has no motion history"
    );
}

/// Every contributed frame keeps complete timings; `drawing` frames also time the cull.
fn timed(sample: &FrameSample, device: &wgpu::Device, drawing: bool) {
    assert!(!sample.timing_fault, "contributed frame timing fault");
    if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
        let scene = sample.gpu_pass_ns.expect("scene pass times");
        assert!(sample.gpu_present_ns.is_some(), "present time");
        if drawing {
            let contribution = sample
                .gpu_contribution_ns
                .expect("contribution compute time");
            assert!(
                scene[3] >= contribution,
                "forward includes the contribution"
            );
        }
    }
}

#[test]
fn contributed_frames_keep_complete_gpu_timings_and_reset_history_for_new_sets() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        contributed_frames_keep_complete_gpu_timings_and_reset_history_for_new_sets_case(path);
    }
}

fn contributed_frames_keep_complete_gpu_timings_and_reset_history_for_new_sets_case(
    path: CookedDrawPath,
) {
    let root = std::env::temp_dir().join(format!("thrum-g5-timing-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 64);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let view = drawable(&device);
        let mut host =
            FrameHost::new_with_capabilities(&device, &queue, &capabilities, presenting())
                .await
                .expect("prepared game scene");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        until_swap(&mut host, &device, &view, &stats, &mut clock);
        timed(
            &presented_frame(&mut host, &device, &view, &mut clock),
            &device,
            path == CookedDrawPath::Indirect,
        );
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(true));
        // A capture does not become the previous frame; history continues across it.
        host.capture_pixels().expect("capture");
        timed(
            &presented_frame(&mut host, &device, &view, &mut clock),
            &device,
            path == CookedDrawPath::Indirect,
        );
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(true));
        // An edit swaps in a new set, whose first frame again has no history.
        demo::write_checker_png(
            &cook.source.join("textures/checker.png"),
            512,
            [40, 200, 40],
        )
        .expect("edit texture");
        cooker
            .cook_changed(&[PathBuf::from("textures/checker.png")])
            .expect("recook");
        until_swap(&mut host, &device, &view, &stats, &mut clock);
        timed(
            &presented_frame(&mut host, &device, &view, &mut clock),
            &device,
            path == CookedDrawPath::Indirect,
        );
        assert_eq!(stats.lock().expect("stats").last_frame_history, Some(true));
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_replaced_device_restages_and_draws_the_current_set() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_replaced_device_restages_and_draws_the_current_set_case(path, path);
        let other = if path == CookedDrawPath::Direct {
            CookedDrawPath::Indirect
        } else {
            CookedDrawPath::Direct
        };
        a_replaced_device_restages_and_draws_the_current_set_case(path, other);
    }
}

fn a_replaced_device_restages_and_draws_the_current_set_case(
    path: CookedDrawPath,
    replacement_path: CookedDrawPath,
) {
    let root = std::env::temp_dir().join(format!("thrum-g5-device-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 64);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let gpu = |generation, path| {
            let instance = &instance;
            async move {
                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions::default())
                    .await
                    .expect("real adapter");
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        required_features: DeviceCapabilities::requested_content_features(&adapter)
                            | DeviceCapabilities::requested_timing_features(&adapter),
                        ..Default::default()
                    })
                    .await
                    .expect("device");
                let capabilities = selected_capabilities(&adapter, &device, generation, path);
                (device, queue, capabilities)
            }
        };
        let (old_device, old_queue, old_capabilities) = gpu(1, path).await;
        let mut host = FrameHost::new_with_capabilities(
            &old_device,
            &old_queue,
            &old_capabilities,
            presenting(),
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        let old_view = drawable(&old_device);
        until_swap(&mut host, &old_device, &old_view, &stats, &mut clock);
        let (device, queue, capabilities) = gpu(2, replacement_path).await;
        let mut candidate = host
            .device_prepare_inputs()
            .expect("inputs")
            .prepare(
                &capabilities,
                &queue,
                Some(presenting().surface.expect("surface")),
            )
            .await
            .expect("replacement");
        old_device.destroy();
        host.publish_device(&mut candidate)
            .expect("publish replacement");
        let view = drawable(&device);
        // The same generation is staged and uploaded again for the new device.
        until_swap(&mut host, &device, &view, &stats, &mut clock);
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.draw_path, Some(replacement_path));
        assert_eq!(stats.drawn_device_generation, Some(2));
        if replacement_path == CookedDrawPath::Direct {
            assert_eq!(stats.visible_meshlets, None);
        }
        let swaps = &stats.swaps;
        assert_eq!(swaps.len(), 2);
        assert_eq!(swaps[0].generation, swaps[1].generation);
        if path == replacement_path {
            assert_eq!(swaps[0].uploaded_bytes, swaps[1].uploaded_bytes);
        } else if replacement_path == CookedDrawPath::Direct {
            assert!(
                swaps[1].uploaded_bytes < swaps[0].uploaded_bytes,
                "direct omits meshlet cull metadata uploads"
            );
        } else {
            assert!(
                swaps[1].uploaded_bytes > swaps[0].uploaded_bytes,
                "indirect restages its cull metadata"
            );
        }
    });
    std::fs::remove_dir_all(&root).ok();
}
/// A capture can draw (and swap in) a new set, but it never becomes the scene's previous frame,
/// so the set's first normal frame still has no motion history. Then a mesh edit to fewer
/// meshlets swaps while a count may be mapping: every reported count stays within the total of
/// the set it counted.
#[test]
fn captures_never_establish_history_and_cull_counts_follow_their_set() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        captures_never_establish_history_and_cull_counts_follow_their_set_case(path);
    }
}

fn captures_never_establish_history_and_cull_counts_follow_their_set_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-capture-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 32);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let view = drawable(&device);
        let mut host =
            FrameHost::new_with_capabilities(&device, &queue, &capabilities, presenting())
                .await
                .expect("prepared game scene");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        wait_staged(&stats, 1);
        let deadline = Instant::now() + Duration::from_secs(30);
        while stats.lock().expect("stats").drawn != Some(1) {
            assert!(Instant::now() < deadline, "captures never drew the set");
            host.capture_pixels().expect("capture");
        }
        let mut clock = Duration::ZERO;
        timed(
            &presented_frame(&mut host, &device, &view, &mut clock),
            &device,
            path == CookedDrawPath::Indirect,
        );
        assert_eq!(
            stats.lock().expect("stats").last_frame_history,
            Some(false),
            "a capture-drawn set has no history on its first normal frame"
        );
        let counted = |stats: &Mutex<CookedStats>| {
            let visible = stats.lock().expect("stats").visible_meshlets;
            if path == CookedDrawPath::Direct {
                assert_eq!(visible, None);
            }
            if let Some((kept, total)) = visible {
                assert!(kept <= total, "count {kept} paired with total {total}");
            }
            visible
        };
        let mut before = None;
        while path == CookedDrawPath::Indirect && before.is_none() {
            assert!(Instant::now() < deadline, "no cull count");
            presented_frame(&mut host, &device, &view, &mut clock);
            before = counted(&stats);
        }
        demo::write_grid_gltf(&cook.source.join("meshes"), "terrain", 32, 1.0).expect("edit mesh");
        let report = cooker
            .cook_changed(&[
                PathBuf::from("meshes/terrain.gltf"),
                PathBuf::from("meshes/terrain.bin"),
            ])
            .expect("recook");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(report.published.map(|head| head.generation), Some(2));
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let (drawn, visible, errors) = {
                let stats = stats.lock().expect("stats");
                (
                    stats.drawn,
                    stats.visible_meshlets,
                    (stats.staged, stats.errors.clone()),
                )
            };
            assert!(
                Instant::now() < deadline,
                "new total never reported: drawn {drawn:?}, count {visible:?}, before {before:?}, \
                 errors {errors:?}"
            );
            presented_frame(&mut host, &device, &view, &mut clock);
            let now = counted(&stats);
            if stats.lock().expect("stats").drawn == Some(2)
                && (path == CookedDrawPath::Direct
                    || now.map(|(_, total)| total) < before.map(|(_, total)| total))
            {
                break;
            }
        }
    });
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn cooked_mesh_outside_camera_casts_onto_visible_terrain() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        cooked_mesh_outside_camera_casts_onto_visible_terrain_case(path);
    }
}

fn cooked_mesh_outside_camera_casts_onto_visible_terrain_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-off-camera-sun-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 16);
    // Flat white reference planes isolate shadow visibility from surface normals and texture.
    demo::write_grid_gltf(&cook.source.join("meshes"), "terrain", 8, 0.0).expect("flat plane");
    demo::write_checker_png(&cook.source.join("textures/checker.png"), 64, [235; 3])
        .expect("white texture");
    Cooker::open(cook.clone())
        .expect("cooker")
        .cook_all()
        .expect("cook");

    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("game");
        host.set_scene_camera(
            v1_game::SceneCamera::perspective(
                [0.0, 202.0, 4.0],
                [0.0, 200.0, 0.0],
                0.65,
                0.1,
                30_000.0,
            )
            .expect("terrain camera"),
        )
        .expect("no route installed");
        host.pause().expect("freeze authority");
        host.set_time_of_day(0.5).expect("noon");

        let mut config = placements(cook.store.clone());
        config.volume = None;
        config.meshes[0].transform = transform(2.0, glam::Quat::IDENTITY, [0.0, 200.0, 0.0]);
        let mut roof = config.meshes[0].clone();
        // Noon is toward +Y and -Z at −45°. The roof is above the entire camera
        // frustum and beyond the receiver reach. It casts toward the receiver centre.
        // At 1.4 km axial distance the finite Sun has a ~13 m diameter
        // footprint. A 32 m roof keeps this receiver inside a true umbra.
        roof.transform = transform(16.0, glam::Quat::IDENTITY, [0.0, 1_200.0, -1_000.0]);

        let mut capture = |config| {
            let assets = CookedAssets::new(config).expect("rigid placements");
            let stats = assets.stats();
            host.set_scene_contribution(Box::new(assets))
                .expect("attach");
            assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
            wait_staged(&stats, 1);
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let pixels = host.capture_pixels().expect("terrain capture");
                if stats.lock().expect("stats").drawn == Some(1) {
                    break pixels;
                }
                assert!(Instant::now() < deadline, "generation 1 never drawn");
            }
        };
        let lit = capture(config.clone());
        config.meshes.push(roof);
        let shadowed = capture(config);
        save("off-camera-sun-lit", &lit);
        save("off-camera-sun-shadowed", &shadowed);

        let [width, height] = EXTENT.map(|side| side as usize);
        let mut darkened = 0;
        for y in height / 2 - 8..height / 2 + 8 {
            for x in width / 2 - 8..width / 2 + 8 {
                let pixel = (y * width + x) * 4;
                if lit[pixel].saturating_sub(shadowed[pixel]) > 20 {
                    darkened += 1;
                }
            }
        }
        assert!(
            darkened >= 200,
            "off-camera caster darkened only {darkened} of 256 receiver pixels"
        );
    });
    std::fs::remove_dir_all(&root).expect("remove test store");
}

/// Blocking HTTP/1.1 GET over loopback, standing in for the browser's `fetch`.
struct LoopbackFetch;

impl thrum_asset::Fetch for LoopbackFetch {
    fn get(
        &self,
        url: &str,
    ) -> impl Future<Output = Result<thrum_asset::FetchReply, thrum_asset::AssetError>> + use<>
    {
        use std::io::{Read, Write};
        let transport =
            |error: std::io::Error| thrum_asset::AssetError::Transport(error.to_string());
        let reply = (|| {
            let rest = url.strip_prefix("http://").expect("http origin");
            let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
            let mut stream = std::net::TcpStream::connect(authority).map_err(transport)?;
            write!(
                stream,
                "GET {path} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
            )
            .map_err(transport)?;
            let mut response = Vec::new();
            stream.read_to_end(&mut response).map_err(transport)?;
            let split = response
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .ok_or(thrum_asset::AssetError::Malformed("HTTP header"))?;
            let status = std::str::from_utf8(&response[9..12])
                .ok()
                .and_then(|status| status.parse().ok())
                .ok_or(thrum_asset::AssetError::Malformed("HTTP status"))?;
            Ok(thrum_asset::FetchReply {
                status,
                body: thrum_asset::ProductBytes::copy_from(&response[split + 4..]),
            })
        })();
        std::future::ready(reply)
    }
}

/// A portable-only fetch draws like the mapped portable store even on a device that can
/// sample compressed variants listed in the manifest.
#[test]
fn a_fetched_generation_draws_like_the_mapped_store() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_fetched_generation_draws_like_the_mapped_store_case(path);
    }
}

fn a_fetched_generation_draws_like_the_mapped_store_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-remote-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 64);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    let server = thrum_cook::DevServer::start(
        "127.0.0.1:0",
        thrum_asset::AssetStore::new(&cook.store),
        Arc::new(thrum_cook::Publications::default()),
    )
    .expect("dev server");
    let config = |source: thrum_asset_gpu::CookedSource| {
        let mapped = matches!(&source, thrum_asset_gpu::CookedSource::Store(_));
        let mut config = v1_game::cooked::cooked_config(source);
        config.upload_budget = BUDGET;
        config.volume = None;
        if mapped {
            config.texture_preference = Some(vec![Profile::Portable]);
        }
        config
    };
    pollster::block_on(async {
        let mut client =
            thrum_asset::DevClient::new(LoopbackFetch, &format!("http://{}", server.address()))
                .expect("client");
        let fetched = client.poll().await.expect("fetch").expect("a publication");
        let (device, queue, capabilities) = game_device(path).await;
        let compressed = fetched
            .manifest()
            .resolve("textures/checker", &[Profile::DesktopBc])
            .expect("compressed variant advertised");
        assert!(fetched.product(&compressed.product).is_none());
        #[cfg(target_os = "macos")]
        assert!(
            device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
        );
        let feed = thrum_asset_gpu::RemoteFeed::new(fetched);
        let mut frames = Vec::new();
        for source in [
            thrum_asset_gpu::CookedSource::Store(cook.store.clone()),
            thrum_asset_gpu::CookedSource::Remote(feed.clone()),
        ] {
            let mut host = FrameHost::new_with_capabilities(
                &device,
                &queue,
                &capabilities,
                FrameConfig::default(),
            )
            .await
            .expect("prepared game scene");
            let assets = CookedAssets::new(config(source)).expect("rigid placements");
            let stats = assets.stats();
            host.set_scene_contribution(Box::new(assets))
                .expect("attach");
            assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
            let mut clock = Duration::ZERO;
            wait_for(&mut host, &device, &stats, 1, &mut clock);
            // Same world time for both, so only the product path can differ.
            while clock < Duration::from_millis(16 * 120) {
                clock += Duration::from_millis(16);
                host.drive(clock, None).expect("frame");
                settle(&mut host, &device);
            }
            frames.push(host.capture_pixels().expect("capture"));
            let stats = stats.lock().expect("stats");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            assert_eq!(stats.swaps[0].texture_profile, Some(Profile::Portable));
        }
        save("remote-store", &frames[0]);
        save("remote-fetched", &frames[1]);
        assert_eq!(changed_pixels(&frames[0], &frames[1]), 0);
    });
    drop(server);
    std::fs::remove_dir_all(&root).ok();
}

/// A later publication pushed into the remote feed swaps in on the render thread, uploading only
/// the edited product; the unchanged mesh is reused from the drawn set.
#[test]
fn a_fetched_edit_swaps_in_and_reuses_unchanged_products() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_fetched_edit_swaps_in_and_reuses_unchanged_products_case(path);
    }
}

fn a_fetched_edit_swaps_in_and_reuses_unchanged_products_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-remote-edit-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 64);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    cooker.cook_all().expect("cook");
    let publications = Arc::new(thrum_cook::Publications::default());
    let server = thrum_cook::DevServer::start(
        "127.0.0.1:0",
        thrum_asset::AssetStore::new(&cook.store),
        Arc::clone(&publications),
    )
    .expect("dev server");
    pollster::block_on(async {
        let mut client =
            thrum_asset::DevClient::new(LoopbackFetch, &format!("http://{}", server.address()))
                .expect("client");
        let feed = thrum_asset_gpu::RemoteFeed::new(
            client.poll().await.expect("fetch").expect("generation 1"),
        );
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("prepared game scene");
        let mut config =
            v1_game::cooked::cooked_config(thrum_asset_gpu::CookedSource::Remote(feed.clone()));
        config.upload_budget = BUDGET;
        config.volume = None;
        let assets = CookedAssets::new(config).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        let first = host.capture_pixels().expect("capture");

        demo::write_checker_png(
            &cook.source.join("textures/checker.png"),
            512,
            [40, 200, 40],
        )
        .expect("texture edit");
        let report = cooker.cook_all().expect("recook");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        publications.announce(2);
        let fetched = client.poll().await.expect("fetch").expect("generation 2");
        assert!(
            client.holds(
                &fetched
                    .manifest()
                    .resolve("meshes/terrain", &[Profile::Portable])
                    .expect("mesh")
                    .product
            )
        );
        feed.publish(fetched);
        wait_for(&mut host, &device, &stats, 2, &mut clock);
        let second = host.capture_pixels().expect("capture");
        save("remote-edit", &second);
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.swaps.len(), 2);
        assert!(
            stats.swaps[1].uploaded_bytes < stats.swaps[0].uploaded_bytes,
            "only the texture uploads again: {:?}",
            stats.swaps
        );
        assert_staging_bounded(&stats);
        assert!(
            changed_pixels(&first, &second) > 640 * 360 / 50,
            "the edit is visible"
        );
    });
    drop(server);
    std::fs::remove_dir_all(&root).ok();
}

/// A mesh drawn through a mirroring node (negative determinant) faces and lights like the same
/// geometry stored without one: the mirrored grid describes identical world triangles, so with
/// the scene's back-face culling and lighting the two captures match.
#[test]
fn a_mirrored_node_draws_with_the_facing_and_lighting_of_its_plain_twin() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_mirrored_node_draws_with_the_facing_and_lighting_of_its_plain_twin_case(path);
    }
}

fn a_mirrored_node_draws_with_the_facing_and_lighting_of_its_plain_twin_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-mirrored-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let plain = source_tree(&root.join("plain"), 16);
    let mirrored = source_tree(&root.join("mirrored"), 16);
    demo::write_mirrored_grid_gltf(&mirrored.source.join("meshes"), "terrain", 128, 0.25)
        .expect("mirrored mesh");
    for cook in [&plain, &mirrored] {
        Cooker::open(cook.clone())
            .expect("cooker")
            .cook_all()
            .expect("cook");
    }

    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("game");
        host.set_scene_camera(
            v1_game::SceneCamera::perspective(
                [0.0, 203.5, 3.0],
                [0.0, 200.0, 0.0],
                0.9,
                0.1,
                30_000.0,
            )
            .expect("grid camera"),
        )
        .expect("no route installed");
        host.pause().expect("freeze authority");
        host.set_time_of_day(0.4).expect("morning");

        let mut capture = |store: PathBuf| {
            let mut config = placements(store);
            config.volume = None;
            config.meshes[0].transform = transform(2.0, glam::Quat::IDENTITY, [0.0, 200.0, 0.0]);
            let assets = CookedAssets::new(config).expect("rigid placements");
            let stats = assets.stats();
            host.set_scene_contribution(Box::new(assets))
                .expect("attach");
            assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
            wait_staged(&stats, 1);
            let deadline = Instant::now() + Duration::from_secs(30);
            loop {
                let pixels = host.capture_pixels().expect("grid capture");
                if stats.lock().expect("stats").drawn == Some(1) {
                    break pixels;
                }
                assert!(Instant::now() < deadline, "generation 1 never drawn");
            }
        };
        let plain_pixels = capture(plain.store.clone());
        let mirrored_pixels = capture(mirrored.store.clone());
        save("mirrored-plain", &plain_pixels);
        save("mirrored-node", &mirrored_pixels);

        let total = plain_pixels.len() / 4;
        let changed = changed_pixels(&plain_pixels, &mirrored_pixels);
        assert!(
            changed * 1000 <= total,
            "the mirrored node changed {changed} of {total} pixels"
        );
    });
    std::fs::remove_dir_all(&root).expect("remove test store");
}

/// A publication several times larger than staging streams through the bounded ring: its chunks
/// are filled, copied and reused, staging never exceeds the bound, and an edit of the large
/// volume does the same again.
#[test]
fn a_publication_larger_than_staging_streams_through_the_bounded_ring() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_publication_larger_than_staging_streams_through_the_bounded_ring_case(path);
    }
}

fn a_publication_larger_than_staging_streams_through_the_bounded_ring_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-ring-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 256);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    let report = cooker.cook_all().expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        demo::write_cloud_volume(&cook.source.join("cloud.tvol"), 256, 2).expect("volume edit");
        let report = cooker.cook_all().expect("recook");
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        wait_for(&mut host, &device, &stats, 2, &mut clock);
        let stats = stats.lock().expect("stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        for swap in &stats.swaps {
            assert!(
                swap.uploaded_bytes > 2 * STAGING_BOUND,
                "each publication outgrows staging: {swap:?}"
            );
        }
        assert!(stats.max_frame_copy_bytes <= BUDGET);
        assert_staging_bounded(&stats);
    });
    std::fs::remove_dir_all(&root).expect("remove test store");
}

/// Wait, without rendering, until the loader has posted `generation`.
fn wait_posted(stats: &Mutex<CookedStats>, generation: u64) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while stats.lock().expect("stats").staged != Some(generation) {
        assert!(
            Instant::now() < deadline,
            "generation {generation} never posted while an older upload was still filling"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// A newer publication supersedes an upload whose staging is still being filled: with no frames
/// rendered the ring never frees, so generation 2 cannot finish filling, yet generation 3 is posted
/// and drawn and generation 2 never is.
#[test]
fn a_newer_publication_supersedes_an_upload_still_being_filled() {
    for path in [CookedDrawPath::Indirect, CookedDrawPath::Direct] {
        a_newer_publication_supersedes_an_upload_still_being_filled_case(path);
    }
}

fn a_newer_publication_supersedes_an_upload_still_being_filled_case(path: CookedDrawPath) {
    let root = std::env::temp_dir().join(format!("thrum-g5-supersede-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 256);
    let mut cooker = Cooker::open(cook.clone()).expect("cooker");
    let report = cooker.cook_all().expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);
    pollster::block_on(async {
        let (device, queue, capabilities) = game_device(path).await;
        let mut host = FrameHost::new_with_capabilities(
            &device,
            &queue,
            &capabilities,
            FrameConfig::default(),
        )
        .await
        .expect("prepared game scene");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("rigid placements");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        assert_eq!(stats.lock().expect("stats").draw_path, Some(path));
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        for (generation, seed) in [(2, 2), (3, 3)] {
            demo::write_cloud_volume(&cook.source.join("cloud.tvol"), 256, seed).expect("edit");
            let report = cooker.cook_all().expect("recook");
            assert!(report.failed.is_empty(), "{:?}", report.failed);
            wait_posted(&stats, generation);
        }
        wait_for(&mut host, &device, &stats, 3, &mut clock);
        let stats = stats.lock().expect("stats");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        let swapped: Vec<u64> = stats.swaps.iter().map(|swap| swap.generation).collect();
        assert_eq!(
            swapped,
            [1, 3],
            "generation 2 was superseded before it was drawn"
        );
        assert_staging_bounded(&stats);
    });
    std::fs::remove_dir_all(&root).expect("remove test store");
}

fn selected_capabilities(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    generation: u64,
    path: CookedDrawPath,
) -> DeviceCapabilities {
    let caps = DeviceCapabilities::capture(adapter, device, generation);
    assert!(
        caps.downlevel()
            .expect("adapter facts")
            .contains(wgpu::DownlevelFlags::INDIRECT_EXECUTION)
    );
    match path {
        CookedDrawPath::Indirect => caps,
        CookedDrawPath::Direct => caps.without_indirect_execution(),
    }
}

#[test]
fn adapter_less_public_host_selects_direct() {
    let root = std::env::temp_dir().join(format!("thrum-g5-unknown-{}", std::process::id()));
    std::fs::remove_dir_all(&root).ok();
    let cook = source_tree(&root, 64);
    Cooker::open(cook.clone())
        .expect("cooker")
        .cook_all()
        .expect("cook");
    pollster::block_on(async {
        let (device, queue, _) = game_device(CookedDrawPath::Indirect).await;
        let mut host = FrameHost::new(&device, &queue, FrameConfig::default())
            .await
            .expect("public host");
        let assets = CookedAssets::new(placements(cook.store.clone())).expect("assets");
        let stats = assets.stats();
        host.set_scene_contribution(Box::new(assets))
            .expect("attach");
        let mut clock = Duration::ZERO;
        wait_for(&mut host, &device, &stats, 1, &mut clock);
        let stats = stats.lock().expect("stats");
        assert_eq!(stats.draw_path, Some(CookedDrawPath::Direct));
        assert_eq!(stats.visible_meshlets, None);
        #[cfg(feature = "draw-probe")]
        {
            assert_eq!(stats.probe_resources, (0, false));
            assert_eq!(stats.probe_commands.len(), 1);
            assert!(
                stats
                    .probe_commands
                    .iter()
                    .all(|command| command.contains("draw_indexed range=")
                        && !command.contains("indirect"))
            );
        }
    });
    #[cfg(feature = "draw-probe")]
    {
        // A child exercises the real environment-selected game attachment without mutating
        // this parallel test process's environment.
        let output = root.join("public-route");
        let status = std::process::Command::new(env!("CARGO_BIN_EXE_cooked_draw_probe"))
            .env("THRUM_SCENE", "diagnostic")
            .env("THRUM_ASSET_STORE", &cook.store)
            .env("THRUM_COOKED_DRAW_DIAGNOSTIC", "0")
            .env("COOKED_DRAW_OUTPUT", &output)
            .env("COOKED_DRAW_ADAPTER_LESS", "1")
            .env("COOKED_DRAW_MASK_INDIRECT", "0")
            .env("COOKED_DRAW_MODE", "production")
            .env("COOKED_DRAW_MESHES", "[\"meshes/terrain\"]")
            .status()
            .expect("public route process");
        assert!(status.success());
        let record: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join("report.json")).expect("public route witness"),
        )
        .expect("JSON witness");
        assert_eq!(record["flags"], "None");
        assert_eq!(record["path"], "Some(Direct)");
        assert_eq!(
            record["resource_observation"],
            serde_json::json!([0, false])
        );
    }
    std::fs::remove_dir_all(&root).ok();
}
