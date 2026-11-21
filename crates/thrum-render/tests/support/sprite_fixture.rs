// Shared Metal device, frame source, drawable and capture for the sprite proofs.

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
        CaptureLimits, FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource,
        SpriteSceneRequests, SubmittedImage, SurfaceOutput,
    },
    sprite::{
        EmbeddedSheet, SpriteFrame, SpritePose, SpriteRect, SpriteRow, SpriteScene, SpriteSink,
        SpriteView,
    },
    synthetic::{CaptureSource, FrameLighting, SyntheticView},
};

pub(crate) const RECIPES: [&str; 2] = ["portable", "apple-tile"];
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub(crate) type Host = FrameHost<World, Scene>;

/// The Metal test device at generation 1 with `recipe` selected.
/// None means the gate reported a skip.
pub(crate) fn open(test: &str, recipe: &str) -> Option<thrum_test_support::Opened> {
    let mut opened = thrum_test_support::open(
        test,
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    )?;
    opened.capabilities = with_recipe(opened.capabilities, recipe);
    Some(opened)
}

/// Another Metal device on an admitted adapter, at a later `generation`, with `recipe` selected.
pub(crate) fn metal(
    gpu: &thrum_test_support::Gpu,
    recipe: &str,
    generation: u64,
) -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let (device, queue, caps) = gpu
        .device_with_hardware(
            thrum_test_support::Profile::Native,
            &thrum_test_support::Restriction::default(),
            generation,
            &[],
        )
        .expect("prepare the Metal test device");
    (device, queue, with_recipe(caps, recipe))
}

fn with_recipe(caps: DeviceCapabilities, recipe: &str) -> DeviceCapabilities {
    let caps = caps
        .with_recipe_override(recipe)
        .expect("Metal supports the requested recipe");
    assert_eq!(caps.adapter().backend, wgpu::Backend::Metal);
    caps
}

pub(crate) struct World {
    pub(crate) tick: u64,
    pub(crate) timeline: u64,
    pub(crate) alpha: f32,
    pub(crate) running: bool,
}

impl Default for World {
    fn default() -> Self {
        Self {
            tick: 0,
            timeline: 0,
            alpha: 1.0,
            running: true,
        }
    }
}

impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            timeline: self.timeline,
            tick: self.tick,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _now: Duration) -> Result<WorldAdvance, String> {
        self.tick += u64::from(self.running);
        Ok(WorldAdvance {
            tick: self.tick,
            overload: None,
            completed_ticks: u32::from(self.running),
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(1),
                tick: self.tick,
            },
            timeline: self.timeline,
            page: self.source(),
        })
    }
    fn presentation_alpha_at(&self, _: Duration) -> f32 {
        self.alpha
    }
    fn display_phase_at(&self, _: Duration) -> f32 {
        self.alpha
    }
    fn is_interpolating(&self) -> bool {
        false
    }
    fn tick_due_time(&self) -> Option<Duration> {
        None
    }
}

#[derive(Clone)]
pub(crate) struct Scene {
    pub(crate) rows: Vec<SpriteRow>,
    pub(crate) world_rows: Vec<(u32, u64, u128, SpriteRow)>,
    pub(crate) published_poses: std::collections::BTreeMap<u32, Option<SpritePose>>,
    pub(crate) scale: u8,
    pub(crate) centre: [f64; 2],
    pub(crate) lighting: Option<FrameLighting>,
    pub(crate) view: SyntheticView,
}
impl Scene {
    pub(crate) fn new(rows: Vec<SpriteRow>, scale: u8) -> Self {
        Self {
            rows,
            world_rows: Vec::new(),
            published_poses: std::collections::BTreeMap::new(),
            scale,
            centre: [0.0; 2],
            lighting: None,
            view: SyntheticView::default(),
        }
    }
}
impl SceneSource<SourceTick> for Scene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        assert!(self.lighting.is_some(), "2D host called 3D extraction");
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(0, 1, 7, [0.0, 0.0, 0.5], 0);
        })
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        Ok(SceneFrame {
            view: self.view,
            lighting: self.lighting.expect("2D host sampled the 3D camera"),
        })
    }
    fn extract_sprites(&self, sink: &mut SpriteSink<'_>, _: &SourceTick) {
        for &(slot, generation, identity, row) in &self.world_rows {
            if let Some(&previous) = self.published_poses.get(&slot) {
                sink.push_pair(slot, generation, identity, row, previous);
            } else {
                sink.push(slot, generation, identity, row);
            }
        }
    }
    fn sprites(&self, _: SourceTick, _: f32, extent: [u32; 2]) -> Result<SpriteFrame<'_>, String> {
        let pixels_per_unit = f32::from(self.scale);
        let [x, y] = extent.map(|side| f64::from(side % 2) / (2.0 * f64::from(self.scale)));
        Ok(SpriteFrame {
            view: SpriteView {
                centre: [self.centre[0] + x, self.centre[1] - y],
                pixels_per_unit,
            },
            rows: &self.rows,
        })
    }
}

pub(crate) fn scene_config(extent: [u32; 2]) -> FrameConfig {
    // The surface is deliberately different from the requested internal grid.
    FrameConfig {
        render_extent: [27, 25],
        surface: Some(profile(extent)),
    }
}

pub(crate) fn profile(extent: [u32; 2]) -> SurfaceProfile {
    SurfaceProfile {
        extent,
        format: FORMAT,
    }
}

pub(crate) fn host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    scene: Scene,
    sheet: EmbeddedSheet,
) -> Host {
    configured_host(caps, queue, extent, scene, SpriteScene::new(sheet, 32))
}

pub(crate) fn configured_host(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    extent: [u32; 2],
    scene: Scene,
    config: SpriteScene,
) -> Host {
    pollster::block_on(FrameHost::new_2d(
        caps,
        queue,
        scene_config(extent),
        World::default(),
        scene,
        config,
        SpriteSceneRequests::default(),
    ))
    .expect("prepare the sprite scene")
}

pub(crate) fn sheet(extent: [u32; 2], pixels: &[u8], frames: &[SpriteRect]) -> EmbeddedSheet {
    EmbeddedSheet {
        extent,
        rgba: pixels.into(),
        frames: frames.into(),
        texels_per_unit: 1.0,
    }
}

pub(crate) fn drawable(device: &wgpu::Device, [width, height]: [u32; 2]) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite proof drawable"),
            format: FORMAT,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

pub(crate) fn capture(
    host: &mut Host,
    device: &wgpu::Device,
    extent: [u32; 2],
    source: CaptureSource,
) -> SubmittedImage {
    host.enable_submitted_capture_from(42, CaptureLimits::default(), source)
        .expect("open a bounded capture session");
    let output = drawable(device, extent);
    let encoded = host
        .advance(Duration::ZERO)
        .expect("advance the fixture world")
        .encode_scene()
        .expect("record the admitted scene");
    let attached = encoded
        .frame_extent()
        .expect("encoded frame has a presenter");
    assert_eq!(attached.attachments, [[0; 2], extent, [0; 2], extent]);
    assert_eq!(attached.source, extent);
    assert_eq!(attached.viewport, [0, 0, extent[0], extent[1]]);
    let report = encoded
        .submit(Some(&SurfaceOutput {
            view: &output,
            extent,
        }))
        .expect("submit the recorded frame");
    assert!(report.surface_presented && report.submitted);
    let image =
        thrum_test_support::wait::until("submitted-image tap", Duration::from_secs(10), || {
            host.poll_completed().expect("GPU polling remains healthy");
            host.try_submitted_image()
                .expect("read the submitted image")
        });
    assert!(image.sample.surface_presented);
    assert_eq!(image.extent, extent);
    assert_eq!(image.pixel_source, source);
    assert_eq!(image.sample.history_identity, [0; 2]);
    host.disable_submitted_capture();
    drain(host);
    image
}

pub(crate) fn drain(host: &mut Host) -> Vec<thrum_render::runtime::FrameSample> {
    let mut samples = Vec::new();
    thrum_test_support::wait::until("frame retirement", Duration::from_secs(10), || {
        while let Some(sample) = host.poll_completed().expect("GPU polling remains healthy") {
            samples.push(sample);
        }
        (host.pending_frames() == 0).then_some(())
    });
    samples
}
