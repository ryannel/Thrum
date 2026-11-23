#![allow(
    dead_code,
    reason = "shared public-seam fixture supports several focused GPU tests"
)]
pub(crate) mod island;
pub(crate) mod surf;
pub(crate) mod surf_frame;
use std::time::Duration;
use thrum_app::{
    Game, Gpu,
    sky::{SkyGame, SkyScene, StillWorld},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{FrameHost, FrameSample, SurfaceOutput},
    synthetic::OUTPUT_FORMAT,
};

pub(crate) type Host = FrameHost<StillWorld, SkyScene>;

pub(crate) fn device(
    gpu: &thrum_test_support::Gpu,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let profile = if std::env::var("THRUM_WATER_RECIPE").is_ok_and(|v| v == "portable") {
        thrum_test_support::Profile::Portable
    } else {
        thrum_test_support::Profile::Native
    };
    let (device, queue) = gpu.device(profile, &thrum_test_support::Restriction::default());
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    (device, queue, caps)
}
pub(crate) struct Fixture {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) caps: DeviceCapabilities,
    pub(crate) host: Host,
    pub(crate) game: SkyGame,
    pub(crate) output: wgpu::Texture,
    pub(crate) extent: [u32; 2],
    pub(crate) last_elapsed: Duration,
    pub(crate) report: std::sync::Arc<std::sync::Mutex<thrum_water::WaterReport>>,
    witness: Option<wgpu::Texture>,
}
impl Fixture {
    pub(crate) fn signal(
        &mut self,
        amplitude: f64,
    ) -> std::sync::Arc<std::sync::Mutex<thrum_water_model::Map>> {
        let (_, map) = water_scene::water().expect("map");
        let spectrum = thrum_sea_model::Spectrum {
            long: vec![component(amplitude)],
            cascades: std::array::from_fn(|i| {
                thrum_sea_model::Cascade::new([160.0, 37.0, 9.0][i], &[]).expect("empty cascade")
            }),
            tail_slope_variance: 0.0,
        };
        let water = thrum_water::Water::new(
            std::sync::Arc::clone(&map),
            0,
            spectrum,
            thrum_water::Profile::default(),
        )
        .expect("signal");
        self.host
            .set_scene_contribution(Box::new(water))
            .expect("signal surface");
        map
    }
    pub(crate) fn new(gpu: &thrum_test_support::Gpu, extent: [u32; 2]) -> Self {
        Self::paired(gpu, extent, true)
    }
    pub(crate) fn paired(gpu: &thrum_test_support::Gpu, extent: [u32; 2], water: bool) -> Self {
        Self::with_sampling(gpu, extent, water, false, false)
    }
    pub(crate) fn temporal(gpu: &thrum_test_support::Gpu, extent: [u32; 2]) -> Self {
        Self::with_sampling(gpu, extent, true, true, true)
    }
    pub(crate) fn antialiased(gpu: &thrum_test_support::Gpu, extent: [u32; 2]) -> Self {
        Self::with_sampling(gpu, extent, true, true, false)
    }
    fn with_sampling(
        gpu: &thrum_test_support::Gpu,
        extent: [u32; 2],
        water: bool,
        temporal_sampling: bool,
        overlay: bool,
    ) -> Self {
        let (mut game, report) = water_scene::build_paired(
            std::env::var_os(thrum_app::sky::STORE_VARIABLE).is_some(),
            water,
        )
        .expect("scene");
        game = game.with_temporal_sampling(temporal_sampling);
        if overlay {
            game =
                game.with_display_overlay(thrum_render::synthetic::DisplayOverlay::TemporalInputs);
        }
        Self::from_game(gpu, extent, game, report)
    }
    pub(crate) fn from_game(
        gpu: &thrum_test_support::Gpu,
        extent: [u32; 2],
        mut game: SkyGame,
        report: water_scene::SharedReport,
    ) -> Self {
        let (device, queue, caps) = device(&gpu.fresh_adapter());
        let host = game
            .prepare(
                Gpu {
                    device: &device,
                    queue: &queue,
                    capabilities: &caps,
                    surface: SurfaceProfile {
                        format: OUTPUT_FORMAT,
                        extent,
                    },
                },
                Box::new(|| {}),
            )
            .expect("game prepare")
            .host;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water proof output"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            device,
            queue,
            caps,
            host,
            game,
            output,
            extent,
            last_elapsed: Duration::ZERO,
            report,
            witness: None,
        }
    }
    pub(crate) fn warm_cooked(&mut self) {
        for index in 0..120 {
            self.frame(f64::from(index) / 120.0);
            match self.game.poll_status() {
                Some(thrum_app::sky::ContentStatus::Drawn(_)) => {
                    self.frame(f64::from(index + 1) / 120.0);
                    return;
                }
                Some(thrum_app::sky::ContentStatus::Failed(error)) => panic!("{error}"),
                _ => {}
            }
        }
        panic!("cooked island did not draw");
    }
    pub(crate) fn parameters(&mut self, json: &str) {
        let report = self.game.controls_mut().apply_file(json).expect("controls");
        assert!(report.problems.is_empty(), "{report}");
        let (edits, _) = self.game.take_edits();
        SkyGame::apply(&mut self.host, edits).expect("apply");
    }
    pub(crate) fn replace_device(&mut self, gpu: &thrum_test_support::Gpu) {
        let adapter = gpu.fresh_adapter();
        let (device, queue, _) = device(&adapter);
        let caps = DeviceCapabilities::capture(&adapter.adapter, &device, 2);
        let surface = SurfaceProfile {
            format: OUTPUT_FORMAT,
            extent: self.extent,
        };
        let source = self
            .host
            .device_prepare_inputs()
            .expect("retain scene on device replacement");
        let mut prepared = pollster::block_on(source.prepare(&caps, &queue, Some(surface)))
            .expect("prepare replacement");
        self.host
            .publish_device(&mut prepared)
            .expect("publish replacement");
        self.output = device.create_texture(&self.output_descriptor());
        self.device = device;
        self.queue = queue;
        self.caps = caps;
        self.witness = None;
    }

    fn output_descriptor(&self) -> wgpu::TextureDescriptor<'static> {
        wgpu::TextureDescriptor {
            label: Some("water proof output"),
            size: self.output.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }
    }
    pub(crate) fn frame(&mut self, seconds: f64) -> FrameSample {
        self.submit_frame(seconds, true)
    }
    pub(crate) fn frame_without_timings(&mut self, seconds: f64) -> FrameSample {
        self.submit_frame(seconds, false)
    }
    fn submit_frame(&mut self, seconds: f64, timings: bool) -> FrameSample {
        let begin = std::time::Instant::now();
        self.host.set_timing_requested(timings);
        let view = self
            .output
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.host
            .drive(
                Duration::from_secs_f64(seconds),
                Some(&SurfaceOutput {
                    view: &view,
                    extent: self.extent,
                }),
            )
            .expect("submitted scene");
        self.witness = self.host.submitted_temporal_witness().cloned();
        let sample = thrum_test_support::wait::until("completion", Duration::from_secs(30), || {
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("poll");
            self.host.poll_completed().expect("completed")
        });
        assert!(
            self.host.contribution_error().is_none(),
            "{:?}",
            self.host.contribution_error()
        );
        self.last_elapsed = begin.elapsed();
        sample
    }
    /// The output texture's packed pixels.
    pub(crate) fn image(&self) -> Vec<u8> {
        thrum_test_support::readback::texture(
            &self.device,
            &self.queue,
            &self.output,
            thrum_test_support::readback::Mip::level(0),
            ..,
        )
    }
    pub(crate) fn witness(&self) -> Vec<[f32; 4]> {
        let texture = self.witness.as_ref().expect("temporal input witness");
        let raw = thrum_test_support::readback::texture(
            &self.device,
            &self.queue,
            texture,
            thrum_test_support::readback::Mip::level(0),
            ..,
        );
        raw.as_chunks::<8>()
            .0
            .iter()
            .map(|p| {
                std::array::from_fn(|i| half_value(u16::from_le_bytes([p[2 * i], p[2 * i + 1]])))
            })
            .collect()
    }
}
pub(crate) fn component(amplitude: f64) -> thrum_sea_model::Component {
    thrum_sea_model::Component {
        wave_vector: [std::f64::consts::TAU / 64.0, 0.0],
        amplitude,
        phase: 0.4,
    }
}
pub(crate) fn matrix(extent: [u32; 2], height: f32) -> glam::Mat4 {
    let [x, _, z] = water_scene::camera_position(height, 90.0);
    let view = thrum_render::synthetic::SyntheticView::perspective(
        [x, height, z],
        [x + 100.0, height - water_scene::CAMERA_LOOK_DROP, z],
        [0.0, 1.0, 0.0],
        water_scene::CAMERA_FOV_DEGREES.to_radians(),
        extent[0] as f32 / extent[1] as f32,
        0.1,
        30_000.0,
    )
    .expect("camera");
    glam::Mat4::from_cols_array_2d(&view.view_proj_columns())
}
pub(crate) fn world_at(
    matrix: glam::Mat4,
    extent: [u32; 2],
    pixel: [u32; 2],
    depth: f32,
) -> glam::Vec3 {
    matrix.inverse().project_point3(glam::vec3(
        (pixel[0] as f32 + 0.5) * 2.0 / extent[0] as f32 - 1.0,
        1.0 - (pixel[1] as f32 + 0.5) * 2.0 / extent[1] as f32,
        depth,
    ))
}
fn half_value(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 31);
    let fraction = f32::from(bits & 1023) / 1024.0;
    if exponent == 0 {
        sign * fraction * 2.0_f32.powi(-14)
    } else {
        sign * (1.0 + fraction) * 2.0_f32.powi(exponent - 15)
    }
}

pub(crate) fn foam_stocks(fixture: &Fixture, point: &thrum_water::foam::Witness) -> Vec<[f32; 4]> {
    // Two banks of two levels; the held far-sea means that follow are not stocks.
    let mut cells = foam_buffer(fixture, &point.buffer);
    cells.truncate(4 * (point.side * point.side) as usize);
    cells
}

pub(crate) fn foam_buffer<T: bytemuck::Pod>(fixture: &Fixture, buffer: &wgpu::Buffer) -> Vec<T> {
    thrum_test_support::readback::buffer(&fixture.device, &fixture.queue, buffer, ..)
        .chunks_exact(size_of::<T>())
        .map(bytemuck::pod_read_unaligned)
        .collect()
}
