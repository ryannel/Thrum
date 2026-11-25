//! Translate a known high-contrast object diagonally on a fixed camera. The
//! reference supersamples the same current endpoint at four times the extent.
use crate::support;
use std::io::Write;

use std::time::Duration;
use thrum_core::sim::WorldAdvance;
use thrum_render::{
    capabilities::DeviceCapabilities,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, FrameConfig, FrameError, FrameHost, FrameWorld, SceneFrame, SceneSource,
        SurfaceOutput,
    },
    synthetic::{
        CaptureSource, FrameLighting, MetalFxProbe, Reconstruction, SyntheticLight, SyntheticScene,
        SyntheticView,
    },
};

const EDGE: usize = 64;
const FRAMES: u16 = 24;

struct World(u16);
impl FrameWorld for World {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        SourceTick {
            epoch: 1,
            tick: u64::from(self.0),
            timeline: 1,
        }
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, FrameError> {
        self.0 = u16::try_from(now.as_millis()).expect("bounded fixture tick");
        support::World.advance(now).map(|mut advance| {
            advance.tick = u64::from(self.0);
            advance
        })
    }
    fn publication(&mut self) -> Result<thrum_core::sim::WorldPublication<SourceTick>, FrameError> {
        let mut publication = support::World.publication()?;
        publication.id.tick = u64::from(self.0);
        publication.page = self.source();
        Ok(publication)
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

struct Source(SyntheticView);
impl SceneSource<SourceTick> for Source {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        let phase = f64::from(u16::try_from(page.tick).expect("fixture tick"));
        into.extract(*page, [0.0; 3], |sink| {
            sink.push(
                0,
                1,
                1,
                [-0.65 + 0.054 * phase, 0.48 - 0.039 * phase, 0.5],
                0,
            );
        })
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
            lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        })
    }
}

fn arm(
    caps: &DeviceCapabilities,
    queue: &wgpu::Queue,
    mode: Reconstruction,
    probe: MetalFxProbe,
    factor: u32,
) -> Vec<Vec<u8>> {
    let device = caps.site();
    let mut scene = SyntheticScene::grid(1).expect("translating cube");
    scene.view = match mode {
        Reconstruction::MetalFx => scene.view.with_metalfx().with_metalfx_probe(probe),
        Reconstruction::Taau => scene.view.with_reconstruction(true),
        _ => scene.view,
    };
    let extent = [u32::try_from(EDGE).expect("fixture extent") * factor; 2];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = support::drawable(device, format, extent);
    let surface = SurfaceOutput {
        view: &texture,
        extent,
    };
    let source = Source(scene.view);
    let mut host = pollster::block_on(Box::pin(FrameHost::new(
        caps,
        queue,
        FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile { format, extent }),
        },
        World(0),
        source,
        scene,
        1.0,
    )))
    .expect("prepared fixture");
    assert_eq!(host.reconstruction(), mode);
    host.enable_submitted_capture_from(0, CaptureLimits::default(), CaptureSource::Sdr)
        .expect("submitted SDR tap");
    (1..=FRAMES)
        .map(|tick| {
            let report = host
                .drive(Duration::from_millis(u64::from(tick)), Some(&surface))
                .expect("translating frame");
            assert!(report.surface_presented);
            thrum_test_support::wait::until("fixture image", Duration::from_secs(10), || {
                device.poll(wgpu::PollType::Poll).expect("poll fixture");
                drop(host.poll_completed().expect("completed fixture"));
                let image = host.try_submitted_image().expect("submitted image")?;
                let pixels = image.rgba.clone();
                host.recycle_submitted_image(image);
                Some(pixels)
            })
        })
        .collect()
}

fn box_reference(image: &[u8]) -> Vec<u8> {
    const STRIDE: usize = 4;
    (0..EDGE * EDGE)
        .flat_map(|index| {
            let x = (index % EDGE) * STRIDE;
            let y = (index / EDGE) * STRIDE;
            (0..4).map(move |channel| {
                let total: usize = (y..y + STRIDE)
                    .flat_map(|yy| {
                        (x..x + STRIDE).map(move |xx| {
                            usize::from(image[(yy * EDGE * STRIDE + xx) * 4 + channel])
                        })
                    })
                    .sum();
                u8::try_from((total + 8) / 16).expect("channel average")
            })
        })
        .collect()
}

fn edge_error(output: &[Vec<u8>], reference: &[Vec<u8>], mask: &[Vec<u8>]) -> u64 {
    let mut total = 0;
    for ((actual, expected), edges) in output.iter().zip(reference).zip(mask).skip(4) {
        for y in 1..EDGE - 1 {
            for x in 1..EDGE - 1 {
                let index = (y * EDGE + x) * 4;
                let adjacent = [index - 4, index + 4, index - EDGE * 4, index + EDGE * 4];
                if adjacent.into_iter().all(|at| {
                    (0..3).all(|channel| edges[index + channel].abs_diff(edges[at + channel]) < 8)
                }) {
                    continue;
                }
                total += (0..3)
                    .map(|channel| {
                        u64::from(actual[index + channel].abs_diff(expected[index + channel]))
                    })
                    .sum::<u64>();
            }
        }
    }
    total
}

pub(super) fn metalfx_motion_and_jitter_signs_match_translating_edges() {
    let Some(opened) = thrum_test_support::open(
        "MetalFX motion and jitter",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Native,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let caps = opened
        .capabilities
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let reference: Vec<_> = arm(
        &caps,
        &opened.queue,
        Reconstruction::Off,
        MetalFxProbe::None,
        4,
    )
    .iter()
    .map(|image| box_reference(image))
    .collect();
    let twin = arm(
        &caps,
        &opened.queue,
        Reconstruction::Taau,
        MetalFxProbe::None,
        1,
    );
    let errors = [
        MetalFxProbe::None,
        MetalFxProbe::ReverseMotion,
        MetalFxProbe::ReverseJitterY,
        MetalFxProbe::ReverseJitterX,
    ]
    .map(|probe| {
        let frames = arm(&caps, &opened.queue, Reconstruction::MetalFx, probe, 1);
        let errors = [
            edge_error(&frames, &reference, &reference),
            edge_error(&frames, &twin, &reference),
        ];
        writeln!(
            std::io::stdout().lock(),
            "{probe:?}: edge errors against supersampled reference and TAAU {errors:?}"
        )
        .expect("write target proof");
        errors
    });
    assert!(
        errors[0][0] > 0 && errors[0][1] > 0,
        "object edge must be observed"
    );
    for control in &errors[1..] {
        assert!(
            errors[0][0] < control[0],
            "the translating reference must reject the sign control: {errors:?}"
        );
        assert!(
            errors[0][1] < control[1],
            "TAAU must reject the sign control: {errors:?}"
        );
    }
}
