use super::*;
use crate::{
    contribution::{
        ContributionFrame, ContributionPasses, EffectAccess, EffectCadence, EffectGraph,
        EffectPass, SceneContribution, SceneSurface,
    },
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
};

const OVERLAY_PASS: &str = "publish stationary cloud overlay";

pub(in super::super) struct OneOverlay {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    pub(in super::super) runs: bool,
    pub(in super::super) executions: u32,
}

impl OneOverlay {
    pub(in super::super) fn register(probe: &mut Probe<'_>, cloudy: bool) -> Self {
        let device = probe.caps.device();
        let cube = probe.prepared.overlay_witness();
        let view = cube.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let graph = EffectGraph {
            resources: vec![(
                ResourceDescriptor::of_texture(&cube),
                ResourceScope::Persistent {
                    owner: 1,
                    epoch: 1,
                    initialized: true,
                },
            )],
            passes: vec![EffectPass {
                name: OVERLAY_PASS.into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![EffectAccess {
                    resource: 0,
                    range: 0..6,
                    kind: AccessKind::StorageWrite,
                    view: Some(wgpu::TextureViewDimension::D2Array),
                    history: None,
                }],
            }],
            sky_overlays: vec![0],
            ..Default::default()
        };
        probe.prepared.set_effect_graph(graph);
        probe.target = OffscreenTarget::new(device, &probe.prepared, 16, 16).unwrap();
        let texel = if cloudy {
            "64.0, 48.0, 32.0, 0.5"
        } else {
            "0.0, 0.0, 0.0, 1.0"
        };
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(OVERLAY_PASS),
            source: wgpu::ShaderSource::Wgsl(
                format!("@group(0) @binding(0) var overlay_image: texture_storage_2d_array<rgba16float, write>;
                 @compute @workgroup_size(8, 8)
                 fn main(@builtin(global_invocation_id) p: vec3<u32>) {{
                     textureStore(overlay_image, vec2<i32>(p.xy), i32(p.z), vec4({texel}));
                 }}")
                .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(OVERLAY_PASS),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(OVERLAY_PASS),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
        });
        Self {
            pipeline,
            group,
            runs: true,
            executions: 0,
        }
    }

    pub(in super::super) fn submit_frame(&mut self, probe: &mut Probe<'_>, frame: AtmosphereFrame) {
        probe.update(frame);
        let device = probe.caps.device();
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut segments = Vec::new();
        probe
            .prepared
            .encode_contributed(
                &mut encoder,
                probe.caps.generation(),
                &probe.target,
                None,
                None,
                Some(self),
                None,
                &mut segments,
                None,
            )
            .unwrap();
        wait(
            device,
            probe
                .queue
                .submit(segments.into_iter().chain([encoder.finish()])),
        );
        probe.prepared.commit_frame();
    }
}

impl SceneContribution for OneOverlay {
    fn effect_runs(&self, _: usize, _: &ContributionFrame) -> bool {
        self.runs
    }

    fn encode_effect(&mut self, _: usize, pass: &mut wgpu::ComputePass<'_>, _: &ContributionFrame) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(6, 6, 6);
        self.executions += 1;
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses::default()
    }

    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), crate::runtime::FrameError> {
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {}
}

fn compare(after: &[[f32; 4]], old: &[[f32; 4]], expected: &[[f32; 4]]) {
    assert_eq!(after.len(), 7);
    for (level, ((after, old), expected)) in after.iter().zip(old).zip(expected).enumerate() {
        for band in 0..3 {
            let tolerance = expected[band].abs().max(1.0) * 0.002;
            assert!(
                (after[band] - expected[band]).abs() <= tolerance,
                "level {level} band {band}: {after:?} != {expected:?}"
            );
            assert!(
                (after[band] - old[band]).abs() > tolerance,
                "level {level} band {band} retained the old light"
            );
        }
    }
}

#[test]
fn light_edit_refreshes_all_levels_after_the_registered_overlay_stops_writing() {
    assert_reflection_light_edit(false);
}

#[test]
fn governor_entry_refreshes_all_levels_after_a_held_light_edit() {
    assert_reflection_light_edit(true);
}

fn assert_reflection_light_edit(edit_before_governor: bool) {
    let Some(crate::test_device::Opened {
        gpu, device, queue, ..
    }) = crate::test_device::open(
        "Metal governed reflection light edit",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction::default(),
    )
    else {
        return;
    };
    crate::timestamp_permit::hold();
    let caps = DeviceCapabilities::capture(&gpu.adapter, &device, 1);
    pollster::block_on(async {
        let old = sun(30.0, 90_000.0, [1.0; 3]);
        let dim = sun(30.0, 45_000.0, [1.0; 3]);
        let mut spread = Probe::new(&caps, &queue, old, true).await;
        let mut prior = Probe::new(&caps, &queue, old, true).await;
        let mut expected = Probe::new(&caps, &queue, dim, true).await;
        let mut overlay = OneOverlay::register(&mut spread, true);
        overlay.submit_frame(&mut spread, old);
        assert_eq!(overlay.executions, 1);
        overlay.runs = false;
        if edit_before_governor {
            overlay.submit_frame(&mut spread, dim);
            overlay.submit_frame(&mut spread, dim);
            spread.cadence(4);
        } else {
            spread.cadence(8);
            overlay.submit_frame(&mut spread, old);
        }
        for (control, light) in [(&mut prior, old), (&mut expected, dim)] {
            control.overlay(CLOUDY);
            control.draw(light);
        }
        let forward = ForwardProbe::new(&device, &spread.prepared, 7, 1.0);
        let lent = observer_products(&spread);
        let probe = observer::Probe::new(
            &device,
            &lent,
            "",
            ["vec4(environment_reflection(q.xyz, q.w, 1.0).radiance, 1.0)"],
        );
        let queries: [[f32; 4]; 7] =
            std::array::from_fn(|level| [0.0, 0.0, 1.0, 0.045 + 0.955 * level as f32 / 6.0]);
        let sample = |products: &crate::atmosphere::EnvironmentProducts| {
            probe
                .sample(&device, &queue, products, &queries, 0)
                .into_iter()
                .map(|[value]| value)
                .collect::<Vec<_>>()
        };

        overlay.submit_frame(&mut spread, dim);
        assert_eq!(overlay.executions, 1);
        let actual_lent = sample(&lent);
        let old_lent = sample(&observer_products(&prior));
        let new_lent = sample(&observer_products(&expected));
        let actual_forward = forward.sample(&spread, 0);
        let old_forward = forward.sample(&prior, 1);
        let new_forward = forward.sample(&expected, 2);
        compare(&actual_lent, &old_lent, &new_lent);
        compare(&actual_forward, &old_forward, &new_forward);
    });
}
