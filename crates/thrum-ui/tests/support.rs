//! Minimal host, drawable and submitted capture shared by the Metal stage proofs.

use std::time::Duration;

use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, FrameConfig, FrameError, FrameHost, FrameSample, FrameWorld, SceneFrame,
        SceneSource, SpriteSceneRequests, SubmittedImage, SurfaceOutput,
    },
    sprite::{EmbeddedSheet, NamedSheet, SpriteFrame, SpriteRect, SpriteScene, SpriteView},
    synthetic::CaptureSource,
};
use thrum_ui::{InterfaceConfig, PaintList, PaintListBuilder, SpriteRow};

pub(crate) const RECIPES: [&str; 2] = ["portable", "apple-tile"];
pub(crate) const SDR: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub(crate) const FLOAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(crate) const MARK: [[u8; 4]; 6] = [
    [255, 41, 8, 255],
    [40, 210, 64, 255],
    [13, 1, 252, 255],
    [19, 27, 0, 127],
    [212, 63, 209, 255],
    [40, 233, 249, 255],
];

pub(crate) fn device(label: &str, recipe: &str) -> Option<thrum_test_support::Opened> {
    let mut opened = thrum_test_support::open(
        label,
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    )?;
    opened.capabilities = opened
        .capabilities
        .with_recipe_override(recipe)
        .expect("Metal test recipe");
    assert_eq!(opened.capabilities.adapter().backend, wgpu::Backend::Metal);
    Some(opened)
}

#[derive(Default)]
pub(crate) struct World;

impl FrameWorld for World {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            timeline: 0,
            tick: 0,
        }
    }

    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        Ok(WorldAdvance {
            tick: 0,
            overload: None,
            completed_ticks: 0,
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }

    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: 0,
            },
            timeline: 0,
            page: self.source(),
        })
    }

    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        1.0
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

pub(crate) struct Scene {
    pub(crate) interface: Option<InterfaceConfig>,
    pub(crate) paint: Option<PaintList>,
    pub(crate) sprites: Vec<SpriteRow>,
}

impl SceneSource<SourceTick> for Scene {
    fn extract(&self, _: &mut RenderExtractor, _: &SourceTick) -> ExtractReport {
        panic!("fixture uses a 2D scene")
    }

    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<thrum_render::synthetic::FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        panic!("fixture uses a 2D scene")
    }

    fn sprites(
        &self,
        _: SourceTick,
        _: f32,
        [width, height]: [u32; 2],
    ) -> Result<SpriteFrame<'_>, FrameError> {
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [f64::from(width) / 2.0, -f64::from(height) / 2.0],
                pixels_per_unit: 1.0,
            },
            rows: &self.sprites,
        })
    }

    fn interface_config(&self) -> Option<InterfaceConfig> {
        self.interface.clone()
    }

    fn paint_list(&mut self, _: [u32; 2]) -> Option<PaintList> {
        self.paint.clone()
    }
}

pub(crate) fn embedded(extent: [u32; 2], rgba: &[u8], frames: &[SpriteRect]) -> EmbeddedSheet {
    EmbeddedSheet {
        extent,
        rgba: rgba.into(),
        frames: frames.into(),
        texels_per_unit: 1.0,
    }
}

pub(crate) fn interface(capacity: u32) -> InterfaceConfig {
    let mut bytes: Vec<_> = MARK.into_iter().flatten().collect();
    bytes.extend([255; 4]);
    bytes.resize(36, 0);
    let mut sheet = NamedSheet::pixel(
        "proof marks and fill",
        embedded(
            [3, 3],
            &bytes,
            &[
                SpriteRect {
                    origin: [0, 0],
                    size: [3, 2],
                },
                SpriteRect {
                    origin: [0, 2],
                    size: [1, 1],
                },
            ],
        ),
    );
    sheet.tiles = vec![0, 1];
    InterfaceConfig {
        sheets: vec![sheet],
        row_capacity: capacity,
    }
}

pub(crate) fn paint(extent: [u32; 2], scale: u8) -> PaintList {
    let mut list = PaintListBuilder::new(extent);
    let mut mark = SpriteRow::new(0, [2.0, 3.0]);
    mark.scale = [f32::from(scale); 2];
    list.push(mark);
    let mut scrim = SpriteRow::new(1, [4.0, 0.0]);
    scrim.scale = [5.0, 5.0];
    scrim.tint = [0.15, 0.0, 0.55, 0.5];
    list.push(scrim);
    list.push(SpriteRow::new(1, [1.0, 1.0]));
    list.finish()
}

pub(crate) type Host = FrameHost<World, Scene>;

pub(crate) fn host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    format: wgpu::TextureFormat,
    interface: Option<InterfaceConfig>,
    background: f32,
) -> Result<Host, FrameError> {
    let white = embedded(
        [1, 1],
        &[255; 4],
        &[SpriteRect {
            origin: [0; 2],
            size: [1; 2],
        }],
    );
    let mut sprites = SpriteScene::new(white, 1);
    sprites.sheets[0].tiles = vec![0];
    let mut row = SpriteRow::new(0, [0.0, 0.0]);
    row.tint = [background, background, background, 1.0];
    row.scale = [128.0; 2];
    pollster::block_on(FrameHost::new_2d(
        caps,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { extent, format }),
        },
        World,
        Scene {
            interface,
            paint: None,
            sprites: vec![row],
        },
        sprites,
        SpriteSceneRequests::default(),
    ))
}

pub(crate) fn drawable(
    device: &wgpu::Device,
    [width, height]: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("interface proof drawable"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) fn frame(host: &mut Host, view: &wgpu::TextureView, extent: [u32; 2]) {
    let report = host
        .drive(Duration::ZERO, Some(&SurfaceOutput { view, extent }))
        .expect("drive one real host frame");
    assert!(report.surface_presented && report.submitted);
}

pub(crate) fn sample(host: &mut Host) -> FrameSample {
    thrum_test_support::wait::until("interface frame report", Duration::from_secs(5), || {
        host.poll_completed().expect("poll frame reports")
    })
}

pub(crate) fn capture(
    host: &mut Host,
    view: &wgpu::TextureView,
    extent: [u32; 2],
) -> SubmittedImage {
    let source = if host.is_edr() {
        CaptureSource::Edr
    } else {
        CaptureSource::Sdr
    };
    host.enable_submitted_capture_from(
        71,
        CaptureLimits {
            staging_slots: 1,
            cpu_buffers: 1,
            byte_allowance: 1024 * 1024,
        },
        source,
    )
    .expect("admit the submitted output tap");
    frame(host, view, extent);
    drop(sample(host));
    let image = thrum_test_support::wait::until(
        "interface submitted capture",
        Duration::from_secs(5),
        || {
            drop(host.poll_completed().expect("progress capture queue"));
            host.try_submitted_image().expect("poll submitted image")
        },
    );
    host.disable_submitted_capture();
    assert_eq!(image.extent, extent);
    assert_eq!(image.pixel_source, source);
    image
}

pub(crate) fn decode(code: f64) -> f64 {
    if code <= 0.04045 {
        code / 12.92
    } else {
        ((code + 0.055) / 1.055).powf(2.4)
    }
}

pub(crate) fn encode(linear: f64) -> f64 {
    if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

pub(crate) fn half(channel: &[u8]) -> f64 {
    half::f16::from_le_bytes([channel[0], channel[1]]).to_f64()
}
