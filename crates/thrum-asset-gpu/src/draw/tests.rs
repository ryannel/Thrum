//! The cooked volume through the contribution a game uses, with the scene's Sun
//! optical depth on, so the cooked shader's cloud-factor variant draws it.
use std::{path::Path, time::Duration};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::{AtmosphereFrame, SunOpticalDepth},
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SyntheticScene, SyntheticView},
};

use crate::{CookedAssets, CookedConfig, Drawn, VolumePlacement, await_drawn};

const PAGE: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 1,
};
const SIDE: u32 = 32;

struct World;
impl FrameWorld for World {
    type Page = SourceTick;
    fn source(&self) -> SourceTick {
        PAGE
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
            timeline: 1,
            page: PAGE,
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

struct Source(SyntheticView, FrameLighting);
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        into.extract(*page, [0.0; 3], |_| {})
    }
    fn frame(
        &self,
        _: SourceTick,
        _: f32,
        _: [u32; 2],
        _: Option<FrameLighting>,
    ) -> Result<SceneFrame, FrameError> {
        Ok(SceneFrame {
            view: self.0,
            lighting: self.1,
        })
    }
}

/// The frame's captured pixels, drawn with the named cooked volume, or without
/// the cooked contribution when `volume` is `None`. `cloud` is the Sun optical
/// depth of the cloud layer above the volume.
fn draw(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    volume: Option<&str>,
    cloud: f32,
) -> Vec<u8> {
    // Looking up at the volume with the Sun behind it, so the brightest sky
    // fills the frame behind the volume.
    let view = SyntheticView::perspective(
        [0.0, 2.0, 6.0],
        [0.0, 6.0, 0.0],
        [0.0, 1.0, 0.0],
        0.9,
        1.0,
        0.1,
        4000.0,
    )
    .expect("camera");
    let lighting = FrameLighting::Atmosphere(
        AtmosphereFrame::new([0.0, 0.5547, -0.8321], 100_000.0, [1.0; 3], 2.0, 1).expect("Sun"),
    );
    let mut scene = SyntheticScene::grid(1).expect("scene");
    scene.view = view;
    scene.instances[0].deactivate();
    scene.sun_optical_depth = Some(
        SunOpticalDepth::new(
            [2, 2],
            &[cloud; 4],
            20.0,
            [-64.0, -64.0, 64.0, 64.0],
            0.1,
            1,
        )
        .expect("Sun optical depth"),
    );
    let mut host = pollster::block_on(FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: [SIDE; 2],
            surface: None,
        },
        World,
        Source(view, lighting),
        scene,
        1.0,
    ))
    .expect("host");
    let Some(name) = volume else {
        return host.capture_pixels().expect("sky frame");
    };
    let mut config = CookedConfig::new(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cloud-volumes"),
        Vec::new(),
    );
    let scale = 4.0;
    config.volume = Some(VolumePlacement {
        name: name.into(),
        transform: glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(scale),
            glam::Quat::IDENTITY,
            glam::Vec3::new(-scale * 0.5, 4.0, -scale * 0.5),
        )
        .to_cols_array_2d(),
        extinction: 4.0,
        steps: 16,
        light_steps: 4,
    });
    let cooked = CookedAssets::new(config).expect("cooked");
    let stats = cooked.stats();
    host.set_scene_contribution(Box::new(cooked))
        .expect("cooked contribution");
    if let Err(error) = await_drawn(&mut host, &stats, Drawn::Any) {
        panic!("cooked volume never drew: {error}");
    }
    host.capture_pixels().expect("volume frame")
}

fn luminance(pixels: &[u8]) -> u64 {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|texel| texel[..3].iter().map(|&c| u64::from(c)).sum::<u64>())
        .sum()
}

#[test]
fn a_cloud_shadowed_cooked_volume_keeps_the_sky_when_empty_and_darkens_it_when_dense() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "cloud-shadowed cooked volume",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    // Under a thick cloud layer the volume's Sun term is the cloud factor's
    // to remove: only the cloud-factor variant lets density darken the sky.
    let shade = 8.0;
    let sky = draw(&caps, &queue, None, shade);
    let clear = draw(&caps, &queue, Some("volumes/clear"), shade);
    let dense = draw(&caps, &queue, Some("volumes/dense"), shade);
    let sunlit = draw(&caps, &queue, Some("volumes/dense"), 0.0);
    eprintln!(
        "luminance sky={} clear={} dense={} dense without cloud shade={}",
        luminance(&sky),
        luminance(&clear),
        luminance(&dense),
        luminance(&sunlit)
    );
    assert!(clear == sky, "zero density changed the sky");
    assert!(
        luminance(&dense) < luminance(&sky),
        "density did not darken the sky"
    );
    assert!(
        luminance(&dense) < luminance(&sunlit),
        "the cloud layer's Sun optical depth did not reach the cooked volume"
    );
}
