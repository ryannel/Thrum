//! A second member's GPU fault must not disturb the first member's Maps residency.

thrum_test_support::macos_gpu_test! {
fn first_submission_keeps_the_ordered_members_resident_maps_receiver() {
use super::support::{LIGHT, Source, World, camera};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use thrum_asset_gpu::{COPY_UNIT, CookedAssets, CookedConfig, Drawn, MeshPlacement, await_drawn};
use thrum_render::{
    capabilities::{HardwareRequest, RayStructureSelection},
    contribution::{
        ContributionCandidate, ContributionFrame, ContributionPasses, SceneContribution,
        SceneContributions, SceneSurface,
    },
    present::SurfaceProfile,
    runtime::{FrameConfig, FrameError, FrameHost, SurfaceOutput},
    synthetic::{ShadowSelection, SunShadowMethod, SyntheticScene},
};
use thrum_test_support::{Profile, Restriction, wait};

struct PipelineFault {
    attempts: Arc<Mutex<Vec<bool>>>,
}

impl SceneContribution for PipelineFault {
    fn supports_candidate_preparation(&self) -> bool {
        true
    }

    fn prepare_candidate(
        &self,
        surface: &SceneSurface<'_>,
    ) -> Result<ContributionCandidate, FrameError> {
        self.attempts.lock().expect("test record").push(surface.ray_demand);
        if surface.ray_demand {
            // A valid module with an invalid pipeline entry produces a backend
            // validation result after the first member has returned its candidate.
            let module = surface.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("fault fixture shader"),
                source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn valid_entry() {}".into()),
            });
            let _pipeline = surface.device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("traced-only pipeline fault"),
                layout: None,
                module: &module,
                entry_point: Some("missing_entry"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        }
        Ok(ContributionCandidate::new(self.passes(), self.effect_graph(), |_: &mut Self| {}))
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.attempts.lock().expect("test record").push(surface.ray_demand);
        Ok(())
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}
    let Some(gpu) = thrum_test_support::gpu("ordered Maps fallback", Some(wgpu::Backend::Metal))
    else {
        return;
    };
    if !gpu
        .adapter
        .features()
        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
    {
        thrum_test_support::skip("ordered Maps fallback", "Metal adapter has no ray queries");
        return;
    }
    let request = HardwareRequest::parse("ray-query").expect("ray query request");
    let (device, queue, caps) = gpu
        .device_with_hardware(Profile::Portable, &Restriction::default(), 1, &request)
        .expect("Metal ray queries");
    let caps = caps
        .with_recipe_override("apple-tile")
        .expect("Apple tile recipe");
    let profile = SurfaceProfile {
        extent: [32; 2],
        format: wgpu::TextureFormat::Rgba8Unorm,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ordered Maps fallback output"),
        size: wgpu::Extent3d {
            width: 32,
            height: 32,
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
        extent: profile.extent,
    };
    let mut control = None;
    for requested in [false, true] {
        let eye = camera([0.4, 1.7, 5.0], [0.0, 0.75, 0.0]);
        let mut scene = SyntheticScene::grid(1).expect("one entity slot");
        scene.instances[0].deactivate();
        scene.sun_shadows = false;
        scene.sun_shadow_method = if requested {
            SunShadowMethod::Traced
        } else {
            SunShadowMethod::Maps
        };
        scene.lamp_shadows = requested;
        let mut host = pollster::block_on(FrameHost::new(
            &caps,
            &queue,
            FrameConfig {
                render_extent: profile.extent,
                surface: Some(profile),
            },
            World,
            Source(eye),
            scene,
            1.0,
        ))
        .expect("receiver host");
        let bare = host.capture_pixels().expect("bare scene");

        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/maps-fallback");
        let mut config = CookedConfig::new(
            fixture,
            vec![
                MeshPlacement::new("meshes/rock", "textures/white")
                    .scale(1.3)
                    .on_ground(),
            ],
        );
        config.upload_budget = COPY_UNIT;
        let cooked = CookedAssets::new(config).expect("receiver placement");
        let stats = cooked.stats();
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let fault = PipelineFault { attempts: Arc::clone(&attempts) };
        host.attach_scene_contribution(Box::new(SceneContributions::new(vec![
            Box::new(cooked),
            Box::new(fault),
        ])))
        .expect("initial attachment falls back");
        await_drawn(&mut host, &stats, Drawn::Any).expect("resident receiver");
        let (drawn, upload) = {
            let stats = stats.lock().expect("test record");
            (
                stats.drawn.expect("resident generation"),
                stats.swaps[0].uploaded_bytes,
            )
        };
        assert!(
            upload > COPY_UNIT,
            "the receiver needed a second upload budget"
        );
        let revision = Some([17; 32]);
        let mut visual =
            pollster::block_on(host.visual_prepare_inputs().prepare(LIGHT, 1.0, revision))
                .expect("visual resources");
        assert!(host.publish_visual(&mut visual).expect("visual fallback"));
        host.enable_submitted_capture(1).expect("submitted capture");
        assert!(
            host.drive(Duration::from_millis(17), Some(&output))
                .expect("first frame")
                .submitted
        );
        let first = wait::until("first submitted receiver", Duration::from_secs(3), || {
            device.poll(wgpu::PollType::Poll).expect("Metal progress");
            host.try_submitted_image().expect("submitted readback")
        });
        assert_eq!(first.sample.visual_revision, revision);
        assert_eq!(
            first.sample.shadow_selection,
            ShadowSelection {
                requested_sun: if requested {
                    SunShadowMethod::Traced
                } else {
                    SunShadowMethod::Maps
                },
                requested_lamps: requested,
                fallback: requested.then_some(RayStructureSelection::PreparationFailed),
                ..ShadowSelection::default()
            }
        );
        let stats = stats.lock().expect("test record");
        assert_eq!(stats.drawn, Some(drawn));
        assert_eq!(stats.drawn_device_generation, Some(1));
        assert_eq!(stats.last_frame_copy_bytes, 0);
        assert_ne!(
            first.rgba, bare,
            "the receiver appears in the first submission"
        );
        if requested {
            assert_eq!(
                *attempts.lock().expect("test record"),
                [true, false, true, false]
            );
            assert_eq!(control.take(), Some((drawn, first.rgba)));
        } else {
            assert_eq!(*attempts.lock().expect("test record"), [false, false]);
            control = Some((drawn, first.rgba));
        }
    }
}
}
