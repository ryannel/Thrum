use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use thrum_asset_gpu::{CookedAssets, CookedDrawPath, CookedStats};
use thrum_render::{
    present::SurfaceProfile,
    runtime::{FrameSample, SubmittedImage},
    synthetic::CaptureSource,
};
use v1_game::{CameraRoute, FrameConfig, FrameHost, SurfaceOutput};

use super::support::{Gpu, read_buffer};

const ROUTE: &str = include_str!("../../routes/moving-v1.json");
const PROOF_EXTENT: [u32; 2] = [320, 180];

mod cost;

fn output() -> PathBuf {
    std::env::var_os("THRUM_MESH_STAGE_PROOF_DIR")
        .expect("proof output")
        .into()
}

fn store() -> PathBuf {
    std::env::var_os("THRUM_MESH_STAGE_VALLEY_STORE")
        .expect("cooked authored valley")
        .into()
}

struct Valley<'a> {
    gpu: &'a Gpu,
    host: FrameHost,
    drawable: wgpu::TextureView,
    extent: [u32; 2],
    stats: Arc<Mutex<CookedStats>>,
    shadow_counts: Arc<Mutex<Option<wgpu::Buffer>>>,
}

#[derive(Clone, Copy)]
enum Witness {
    None,
    SunMaps,
    Temporal,
}

impl<'a> Valley<'a> {
    fn new(
        gpu: &'a Gpu,
        path: CookedDrawPath,
        cull: bool,
        extent: [u32; 2],
        hour: f32,
        shadow_counts: bool,
    ) -> Self {
        let (camera, mut cooked) = v1_game::valley_config(store().into()).expect("authored valley");
        cooked.draw_path = path;
        cooked.mesh_stage_cull = cull;
        let counts = Arc::new(Mutex::new(None));
        if shadow_counts {
            cooked.mesh_stage_shadow_rejections = Some(Arc::clone(&counts));
        }
        let cooked = CookedAssets::new(cooked).expect("valley residency");
        let stats = cooked.stats();
        let config = FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent,
            }),
        };
        let mut setup = v1_game::selected_setup().expect("game setup");
        if extent == PROOF_EXTENT {
            setup.display_overlay = thrum_render::synthetic::DisplayOverlay::TemporalInputs;
        }
        let mut host = pollster::block_on(FrameHost::new_with_setup(
            &gpu.device,
            &gpu.queue,
            &gpu.capabilities,
            config,
            setup,
        ))
        .expect("production game host");
        host.set_scene_camera(camera).expect("authored camera");
        host.set_time_of_day(hour / 24.0).expect("Sun");
        host.set_scene_contribution(Box::new(cooked))
            .expect("attach valley");
        host.pause().expect("pause while loading");
        let mut valley = Self {
            gpu,
            host,
            drawable: drawable(&gpu.device, extent),
            extent,
            stats,
            shadow_counts: counts,
        };
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut warm = 0;
        while warm < 4 {
            valley.frame(Duration::ZERO, Witness::None);
            let stats = valley.stats.lock().expect("cooked observations");
            assert!(stats.errors.is_empty(), "{:?}", stats.errors);
            if stats.drawn.is_some() {
                warm += 1;
                assert_eq!(stats.draw_path, Some(path), "{stats:?}");
                assert_eq!(
                    stats.recipe,
                    Some(thrum_render::capabilities::Recipe::AppleTile)
                );
                if path == CookedDrawPath::MeshStages {
                    assert_eq!(stats.mesh_stage_draws, 3, "{stats:?}");
                    assert_eq!(stats.indexed_indirect_draws, 0, "{stats:?}");
                }
            }
            assert!(Instant::now() < deadline, "valley mesh residency deadline");
        }
        valley
    }

    fn install_route(&mut self, hour: f32) -> CameraRoute {
        let mut data: serde_json::Value = serde_json::from_str(ROUTE).expect("route");
        data["render_extent"] = serde_json::json!(self.extent);
        data["start"]["solar_hours"] = serde_json::json!(hour);
        data["overlay"] = serde_json::json!("off");
        let route = CameraRoute::parse(
            &data.to_string(),
            self.host.scene_camera().expect("camera"),
            60,
        )
        .expect("checked moving route");
        let store = thrum_asset::AssetStore::new(store());
        let head = store
            .read_head()
            .expect("read valley head")
            .expect("published valley");
        assert_eq!(head.manifest.to_string(), route.cooked_root);
        assert_eq!(
            self.stats.lock().expect("stats").drawn,
            Some(route.cooked_generation)
        );
        self.host.resume(Duration::ZERO).expect("start fixed world");
        while self.host.tick() < route.origin_tick - 1 {
            let elapsed = Duration::from_secs_f64(self.host.route_step_ms(0.5) / 1000.0);
            self.host
                .drive(elapsed, None)
                .expect("step to route origin");
        }
        self.host
            .set_camera_route(route.clone())
            .expect("install at route origin");
        route
    }

    fn frame(&mut self, now: Duration, witness: Witness) -> (FrameSample, Vec<u8>) {
        let submitted = self
            .host
            .drive(
                now,
                Some(&SurfaceOutput {
                    view: &self.drawable,
                    extent: self.extent,
                }),
            )
            .expect("ordinary game frame");
        assert!(submitted.submitted, "{submitted:?}");
        let bytes = match witness {
            Witness::SunMaps => self.read_texture(
                self.host
                    .submitted_shadow_witness()
                    .expect("three Sun depth maps"),
                4,
                wgpu::TextureAspect::DepthOnly,
            ),
            Witness::Temporal => self.read_texture(
                self.host
                    .submitted_temporal_witness()
                    .expect("temporal inputs"),
                8,
                wgpu::TextureAspect::All,
            ),
            Witness::None => Vec::new(),
        };
        (self.complete(), bytes)
    }

    fn complete(&mut self) -> FrameSample {
        let deadline = Instant::now() + Duration::from_secs(25);
        let sample = loop {
            self.gpu
                .device
                .poll(wgpu::PollType::Poll)
                .expect("Metal poll");
            if let Some(sample) = self.host.poll_completed().expect("frame completed") {
                break sample;
            }
            assert!(Instant::now() < deadline, "GPU timing deadline");
            std::thread::yield_now();
        };
        sample
    }

    fn image(&mut self, sample: &FrameSample) -> SubmittedImage {
        let deadline = Instant::now() + Duration::from_secs(25);
        loop {
            self.gpu
                .device
                .poll(wgpu::PollType::Poll)
                .expect("Metal poll");
            if let Some(image) = self.host.try_submitted_image().expect("T2 submitted image") {
                assert_eq!(image.sample.submission_order, sample.submission_order);
                return image;
            }
            assert!(Instant::now() < deadline, "submitted image deadline");
            std::thread::yield_now();
        }
    }

    fn read_texture(
        &self,
        texture: &wgpu::Texture,
        pixel: u32,
        aspect: wgpu::TextureAspect,
    ) -> Vec<u8> {
        let extent = texture.size();
        let row = extent.width * pixel;
        assert_eq!(row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
        let buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("valley submitted map readback"),
            size: u64::from(row * extent.height * extent.depth_or_array_layers),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                aspect,
                ..texture.as_image_copy()
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(extent.height),
                },
            },
            extent,
        );
        read_buffer(self.gpu, encoder, &buffer)
    }
}

fn drawable(device: &wgpu::Device, extent: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("valley proof drawable"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

#[test]
#[ignore = "requires cooked valley from mesh_stage_proof.py"]
fn valley_cull_equal() {
    let gpu = Gpu::new(true, "apple-tile", |_| {});
    let root = output();
    for cull in [false, true] {
        let mut valley = Valley::new(
            &gpu,
            CookedDrawPath::MeshStages,
            cull,
            PROOF_EXTENT,
            12.0,
            false,
        );
        let route = valley.install_route(12.0);
        valley
            .host
            .enable_submitted_capture_from(1, CaptureSource::SceneLinear)
            .expect("T2 scene-linear");
        let mut tick = route.origin_tick;
        for route_tick in 0..=route.keys.last().expect("route end").tick {
            let time = Duration::from_secs_f64(valley.host.route_step_ms(0.5) / 1000.0);
            let (sample, temporal) = valley.frame(time, Witness::Temporal);
            assert_eq!(sample.tick, tick, "missing or duplicate fixed route sample");
            let geometry = temporal
                .as_chunks::<8>()
                .0
                .iter()
                .filter(|pixel| u16::from_le_bytes([pixel[2], pixel[3]]) & 0x7fff != 0)
                .count();
            assert!(
                geometry > 1000,
                "route tick {route_tick} omitted the valley geometry ({geometry} pixels)"
            );
            let image = valley.image(&sample);
            let name = root.join(format!("valley-{route_tick}.raw"));
            let temporal_name = root.join(format!("valley-{route_tick}-temporal.raw"));
            if cull {
                assert_eq!(
                    image.rgba,
                    std::fs::read(name).expect("unculled frame"),
                    "scene-linear submission at route tick {route_tick}"
                );
                assert_eq!(
                    temporal,
                    std::fs::read(temporal_name).expect("unculled temporal inputs"),
                    "temporal inputs at route tick {route_tick}"
                );
            } else {
                std::fs::write(name, &image.rgba).expect("unculled frame");
                std::fs::write(temporal_name, temporal).expect("unculled temporal inputs");
            }
            valley.host.recycle_submitted_image(image);
            tick += 1;
        }
        println!(
            "valley mesh-stage cull={cull}: {} submitted scene-linear and temporal pairs, cuts {:?}",
            tick - route.origin_tick,
            route.cuts
        );
    }
}

fn shadow_maps(gpu: &Gpu, path: CookedDrawPath, cull: bool, hour: f32, witness: bool) -> Vec<u8> {
    let mut valley = Valley::new(gpu, path, cull, PROOF_EXTENT, hour, witness);
    let counts = valley.shadow_counts.lock().expect("shadow counts").clone();
    if let Some(counts) = &counts {
        gpu.queue.write_buffer(counts, 0, &[0; 32]);
    }
    let (_, maps) = valley.frame(Duration::ZERO, Witness::SunMaps);
    if let Some(counts) = &counts {
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sun map object-stage counts"),
            size: counts.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut copy = gpu.device.create_command_encoder(&Default::default());
        copy.copy_buffer_to_buffer(counts, 0, &readback, 0, counts.size());
        let words: Vec<_> = read_buffer(gpu, copy, &readback)
            .chunks_exact(4)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("one counter")))
            .collect();
        assert!(words[0] > 0);
        assert!(
            words[1..7].iter().sum::<u32>() > 0,
            "vacuous shadow cull: {words:?}"
        );
        assert_eq!(words[7], 0, "shadow caster used a cone test");
        println!("Sun {hour} h, mesh workgroups launched and six plane rejection counts {words:?}");
    }
    maps
}

#[test]
#[ignore = "requires cooked valley from mesh_stage_proof.py"]
fn shadow_equal() {
    let gpu = Gpu::new(true, "apple-tile", |_| {});
    for hour in [12.0, 17.3] {
        let all = shadow_maps(&gpu, CookedDrawPath::MeshStages, false, hour, false);
        let culled = shadow_maps(&gpu, CookedDrawPath::MeshStages, true, hour, true);
        assert_eq!(all.len(), 3 * 1024 * 1024 * 4);
        assert!(
            all == culled,
            "Sun {hour} h first changed byte {:?}",
            all.iter().zip(&culled).position(|(a, b)| a != b)
        );
        println!("Sun {hour} h: all 3 x 1024 x 1024 Depth32Float texels match byte for byte");
    }
}

#[test]
#[ignore = "requires cooked valley from mesh_stage_proof.py"]
fn shadow_parity() {
    let gpu = Gpu::new(true, "apple-tile", |_| {});
    for hour in [12.0, 17.3] {
        let indexed = shadow_maps(&gpu, CookedDrawPath::Indirect, false, hour, false);
        let stages = shadow_maps(&gpu, CookedDrawPath::MeshStages, false, hour, false);
        save_shadow_pair(&output(), hour, &indexed, &stages);
    }
}

fn save_shadow_pair(root: &Path, hour: f32, indexed: &[u8], stages: &[u8]) {
    assert_eq!(indexed.len(), 3 * 1024 * 1024 * 4);
    assert_eq!(stages.len(), indexed.len());
    std::fs::write(root.join(format!("shadow-{hour}-indexed.raw")), indexed).expect("indexed maps");
    std::fs::write(root.join(format!("shadow-{hour}-stages.raw")), stages).expect("mesh maps");
}

#[test]
#[ignore = "capture via scripts/mesh_stage_capture.py"]
fn fixed_stops() {
    let extent = [1920, 1080];
    let gpu = Gpu::new(true, "apple-tile", |_| {});
    let folder = output();
    let mut frames = Vec::new();
    for path in [CookedDrawPath::Indirect, CookedDrawPath::MeshStages] {
        for hour in [12.0, 17.3] {
            let mut valley = Valley::new(&gpu, path, true, extent, hour, false);
            let route = valley.install_route(hour);
            for tick in 0..=360 {
                let stop = [0, 90, 180, 200, 201, 230, 290, 340, 360].contains(&tick);
                if stop {
                    valley
                        .host
                        .enable_submitted_capture_from(tick + 1, CaptureSource::Sdr)
                        .expect("T2 fixed stop tap");
                }
                let time = Duration::from_secs_f64(valley.host.route_step_ms(0.5) / 1000.0);
                let (sample, _) = valley.frame(time, Witness::None);
                assert_eq!(sample.tick, route.origin_tick + tick);
                if stop {
                    let image = valley.image(&sample);
                    let name = format!("{path:?}-{hour}-{tick}.rgba");
                    std::fs::write(folder.join(&name), &image.rgba).expect("fixed stop");
                    valley.host.recycle_submitted_image(image);
                    valley.host.disable_submitted_capture();
                    let stats = valley.stats.lock().expect("resolved cooked draw path");
                    let pose = route.pose(sample.tick, sample.presentation_alpha);
                    frames.push(serde_json::json!({
                        "file": name, "route_tick": tick, "solar_hours": hour, "path": format!("{:?}", stats.draw_path),
                        "mesh_stage_draws": stats.mesh_stage_draws, "indirect_draws": stats.indexed_indirect_draws,
                        "camera_eye": pose.eye, "history_epoch": pose.epoch, "submission": sample.submission_order,
                    }));
                }
            }
        }
    }
    std::fs::write(folder.join("fixed-stops.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "target": "mac-metal", "mode": "fixed", "extent": extent, "recipe": "AppleTile",
        "adapter": gpu.device.adapter_info().name, "registry_digest": gpu.capabilities.registry_digest(),
        "frames": frames,
    })).expect("record encoding")).expect("fixed stops record");
}
