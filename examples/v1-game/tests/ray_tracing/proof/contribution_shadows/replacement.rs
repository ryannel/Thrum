use super::*;
use thrum_render::{
    capabilities::RayStructureSelection,
    contribution::{RayInstances, ShadowBounds},
    present::SurfaceProfile,
    runtime::{FrameSample, SurfaceOutput},
    synthetic::ShadowSelection,
};

pub(super) fn prove(metal: &Metal, fixture: &Fixture) {
    let replacement = metal.replacement();
    for phase in [Phase::Attachment, Phase::Visual, Phase::Device] {
        prove_phase(metal, &replacement, fixture, phase);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Attachment,
    Visual,
    Device,
}

fn prove_phase(metal: &Metal, replacement: &Metal, fixture: &Fixture, phase: Phase) {
    let mut baseline = None;
    let revision = (phase == Phase::Visual).then_some([9; 32]);
    for requested in [SunShadowMethod::Maps, SunShadowMethod::Traced] {
        let traced = requested == SunShadowMethod::Traced;
        let view = SyntheticView::perspective(
            [3.1, 2.2, 4.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            0.8,
            EXTENT[0] as f32 / EXTENT[1] as f32,
            0.05,
            32.0,
        )
        .expect("replacement camera");
        let lighting = FrameLighting::Diagnostic(SyntheticLight::default());
        let mut scene = SyntheticScene::grid(1).expect("replacement scene");
        scene.instances[0].ids[3] = 0;
        scene.view = view;
        scene.sun_shadows = true;
        scene.sun_shadow_method = requested;
        scene.lamp_shadows = traced;
        scene.local_lights = vec![SyntheticPointLight {
            position: [0.0, 1.1, -2.0],
            radius: 5.0,
            candela: [950.0; 3],
        }];
        let profile = SurfaceProfile {
            format: wgpu::TextureFormat::Rgba8Unorm,
            extent: EXTENT,
        };
        let mut config = frame_config();
        config.surface = Some(profile);
        let mut host = pollster::block_on(EngineHost::new(
            &metal.caps,
            &metal.queue,
            config,
            World,
            Camera(view, lighting),
            scene,
            1.0,
        ))
        .expect("replacement host");
        let bare = host
            .capture_pixels()
            .expect("baseline without contribution");

        let cooked = CookedAssets::new(CookedConfig::new(
            fixture.cook.store.clone(),
            vec![MeshPlacement::new("receiver", "colour")],
        ))
        .expect("cooked receiver");
        let stats = cooked.stats();
        let attempts = Arc::new(Mutex::new(Vec::new()));
        host.attach_scene_contribution(Box::new(Fault {
            cooked,
            attempts: Arc::clone(&attempts),
            phase,
        }))
        .expect("attach cooked contribution");
        if phase != Phase::Attachment {
            assert_ne!(
                settled_pixels(&mut host, &stats, 1, fixture.generation()),
                bare
            );
        }
        let active = match phase {
            Phase::Attachment => metal,
            Phase::Visual => {
                let mut candidate = pollster::block_on(
                    host.visual_prepare_inputs()
                        .prepare(lighting, 1.0, revision),
                )
                .expect("prepare visual");
                assert!(host.publish_visual(&mut candidate).expect("publish visual"));
                metal
            }
            Phase::Device => {
                let mut candidate =
                    pollster::block_on(host.device_prepare_inputs().expect("inputs").prepare(
                        &replacement.caps,
                        &replacement.queue,
                        Some(profile),
                    ))
                    .expect("prepare device");
                host.publish_device(&mut candidate).expect("publish device");
                replacement
            }
        };
        assert_eq!(host.contribution_error(), None);
        let expected = ShadowSelection {
            requested_sun: requested,
            requested_lamps: traced,
            fallback: traced.then_some(RayStructureSelection::PreparationFailed),
            ..ShadowSelection::default()
        };
        let first = first_frame(&mut host, active, profile);
        assert_eq!(first.shadow_selection, expected);
        assert_eq!(first.visual_revision, revision);
        assert!(
            host.ray_structure_probe_group(&active.device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[]
                },
            ))
            .is_none()
        );
        let generation = active.caps.generation();
        let after = settled_pixels(&mut host, &stats, generation, fixture.generation());
        let attempts = attempts.lock().expect("rebuild attempts");
        if traced {
            match phase {
                Phase::Attachment => assert_eq!(*attempts, [(1, true), (1, false)]),
                Phase::Visual => assert_eq!(*attempts, [(1, true), (1, true), (1, false)]),
                Phase::Device => assert_eq!(*attempts, [(1, true), (2, true), (2, false)]),
            }
            assert_eq!(&after, baseline.as_ref().expect("Maps control"));
            report(
                &json!({"case": "cooked pipeline fallback", "phase": format!("{phase:?}"), "generation": generation,
                "rebuild_attempts": *attempts, "selection": format!("{expected:?}"),
                "matching_map_bytes": after.len()}),
            );
        } else {
            match phase {
                Phase::Attachment => assert_eq!(*attempts, [(1, false)]),
                Phase::Visual => assert_eq!(*attempts, [(1, false), (1, false)]),
                Phase::Device => assert_eq!(*attempts, [(1, false), (2, false)]),
            }
            assert_ne!(after, bare);
            baseline = Some(after);
        }
    }
}

fn settled_pixels(
    host: &mut EngineHost<World, Camera>,
    stats: &Stats,
    device_generation: u64,
    asset_generation: u64,
) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let pixels = host.capture_pixels().expect("capture cooked receiver");
        let stats = stats.lock().expect("cooked status");
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(asset_generation)
            && stats.drawn_device_generation == Some(device_generation)
        {
            return pixels;
        }
        assert!(Instant::now() < deadline, "cooked receiver never drew");
    }
}

fn first_frame(
    host: &mut EngineHost<World, Camera>,
    metal: &Metal,
    profile: SurfaceProfile,
) -> FrameSample {
    let texture = metal.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("replacement fixture surface"),
        size: wgpu::Extent3d {
            width: EXTENT[0],
            height: EXTENT[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: profile.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let output = SurfaceOutput {
        view: &view,
        extent: EXTENT,
    };
    assert!(
        host.drive(Duration::from_millis(17), Some(&output))
            .expect("draw replacement")
            .submitted
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(sample) = host.poll_completed().expect("replacement report") {
            return sample;
        }
        assert!(
            Instant::now() < deadline,
            "replacement frame never completed"
        );
        std::thread::yield_now();
    }
}

struct Fault {
    cooked: CookedAssets,
    attempts: Arc<Mutex<Vec<(u64, bool)>>>,
    phase: Phase,
}

impl SceneContribution for Fault {
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        let mut attempts = self.attempts.lock().expect("rebuild attempts");
        let inject = match self.phase {
            Phase::Attachment => true,
            Phase::Visual => !attempts.is_empty(),
            Phase::Device => surface.generation == 2,
        };
        attempts.push((surface.generation, surface.ray_demand));
        drop(attempts);
        let mut surface = surface.clone();
        if inject && surface.ray_demand {
            surface.fragment_entry = "injected_missing_entry";
        }
        self.cooked.rebuild(&surface)
    }
    fn passes(&self) -> ContributionPasses {
        self.cooked.passes()
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.cooked.encode_upload(encoder, frame);
    }
    fn computes(&self, frame: &ContributionFrame) -> bool {
        self.cooked.computes(frame)
    }
    fn ray_instances(&self, instances: &mut RayInstances<'_>) {
        self.cooked.ray_instances(instances);
    }
    fn shadow_bounds(&self) -> Option<ShadowBounds> {
        self.cooked.shadow_bounds()
    }
    fn draw_shadow(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.cooked.draw_shadow(pass);
    }
    fn encode_compute(&mut self, pass: &mut wgpu::ComputePass<'_>, frame: &ContributionFrame) {
        self.cooked.encode_compute(pass, frame);
    }
    fn encode_after_compute(&mut self, encoder: &mut wgpu::CommandEncoder) {
        self.cooked.encode_after_compute(encoder);
    }
    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        self.cooked.draw(pass);
    }
    fn submitted(&mut self, history_advanced: bool) {
        self.cooked.submitted(history_advanced);
    }
}
