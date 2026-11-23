//! Public frame error matching on the renderer's Metal target.
mod support;

use std::time::Duration;
use thrum_core::sim::{WorldAdvance, WorldPublication};
use thrum_render::{
    capabilities::DeviceCapabilities,
    error::RenderError,
    extract::SourceTick,
    runtime::{FrameConfig, FrameError, FrameHost, FrameWorld, SceneSource, SurfaceOutput},
    synthetic::{FrameLighting, SyntheticLight, SyntheticScene},
};

struct RefusingWorld(support::World);

impl FrameWorld for RefusingWorld {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        self.0.source()
    }
    fn has_current_page(&self) -> bool {
        self.0.has_current_page()
    }
    fn is_threaded(&self) -> bool {
        self.0.is_threaded()
    }
    fn advance(&mut self, _: Duration) -> Result<WorldAdvance, FrameError> {
        Err(FrameError::World("authoritative clock refused".into()))
    }
    fn publication(&mut self) -> Result<WorldPublication<Self::Page>, FrameError> {
        self.0.publication()
    }
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        self.0.presentation_alpha_at(lead)
    }
    fn display_phase_at(&self, lead: Duration) -> f32 {
        self.0.display_phase_at(lead)
    }
    fn is_interpolating(&self) -> bool {
        self.0.is_interpolating()
    }
    fn tick_due_time(&self) -> Option<Duration> {
        self.0.tick_due_time()
    }
}

#[test]
fn the_default_sprite_callback_returns_a_matchable_render_failure() {
    let source = support::Source {
        view: SyntheticScene::grid(1).unwrap().view,
        lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        instances: Vec::new(),
        lights: Vec::new(),
    };
    let tick = SourceTick {
        epoch: 1,
        tick: 0,
        timeline: 0,
    };
    assert!(matches!(
        source.sprites(tick, 0.0, [640, 480]),
        Err(FrameError::Render(RenderError::MissingSpriteSource)),
    ));
}

#[test]
fn usage_world_and_gpu_faults_have_distinct_kinds() {
    let Some(thrum_test_support::Opened {
        gpu, device, queue, ..
    }) = thrum_test_support::open(
        "frame error kinds",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    )
    else {
        return;
    };
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    let scene = SyntheticScene::grid(1).unwrap();
    let camera = scene.view;
    let source = || support::Source {
        view: camera,
        lighting: FrameLighting::Diagnostic(SyntheticLight::default()),
        instances: vec![([0.0; 3], 0)],
        lights: Vec::new(),
    };
    let extent = [32, 32];
    let config = FrameConfig {
        render_extent: extent,
        surface: None,
    };
    let mut host = pollster::block_on(FrameHost::new(
        &capabilities,
        &queue,
        config,
        RefusingWorld(support::World),
        source(),
        scene.clone(),
        1.0,
    ))
    .expect("headless host");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("drawable supplied to a headless host"),
        size: wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(matches!(
        host.resize_surface(extent),
        Err(FrameError::InvalidState("host has no surface presenter"))
    ));
    assert!(matches!(
        host.drive(
            Duration::ZERO,
            Some(&SurfaceOutput {
                view: &view,
                extent
            })
        ),
        Err(FrameError::InvalidState("host has no surface presenter"))
    ));
    assert!(matches!(
        host.drive(Duration::ZERO, None),
        Err(FrameError::World(message)) if message == "authoritative clock refused"
    ));
    let (restricted, queue) = gpu.device(
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction {
            storage_buffers: Some(1),
            ..Default::default()
        },
    );
    let capabilities = DeviceCapabilities::capture(&gpu.adapter, &restricted, 1);
    let preparation = pollster::block_on(FrameHost::new(
        &capabilities,
        &queue,
        config,
        RefusingWorld(support::World),
        source(),
        scene,
        1.0,
    ));
    assert!(matches!(preparation, Err(FrameError::Gpu { .. })));
}
