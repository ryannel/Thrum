//! Actual pass observations: caching and cancelled recording must agree with
//! submitted products, independently of the implementation's validity bits.

use crate::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{FrameLighting, OffscreenTarget, PreparedSynthetic, SyntheticScene, SyntheticView},
    timing::GpuTimings,
};
use std::time::Duration;

fn wait(device: &wgpu::Device, submission: wgpu::SubmissionIndex) {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("cache observation completes");
}

fn submitted_names(
    prepared: &mut PreparedSynthetic,
    target: &OffscreenTarget,
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
) -> Vec<String> {
    let device = capabilities.device();
    let generation = capabilities.generation();
    let mut timings = GpuTimings::new(capabilities, queue, prepared.scene_pass_count()).unwrap();
    assert!(
        timings.is_enabled(),
        "Metal cache qualification requires actual timestamp queries"
    );
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, generation, target, Some(&mut timings))
        .unwrap();
    timings.finish_render(generation, &encoder).unwrap();
    wait(device, queue.submit([encoder.finish()]));
    prepared.commit_frame();
    let mut resolve = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    assert!(timings.resolve_if_ready(generation, &mut resolve).unwrap());
    wait(device, queue.submit([resolve.finish()]));
    timings
        .try_collect(generation)
        .unwrap()
        .expect("mapped raw pass observations")
        .iter()
        .map(|pass| pass.name.to_string())
        .collect()
}

fn cube_passes(names: &[String]) -> usize {
    names
        .iter()
        .filter(|name| name.starts_with("atmosphere reflection mip "))
        .count()
}

fn has_dfg(names: &[String]) -> bool {
    names
        .iter()
        .any(|name| name == "atmosphere DFG integration")
}

#[test]
fn metal_reflection_cache_tracks_sky_but_not_amplitude_view_or_unsubmitted_work() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real cache adapter");
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .expect("cache device");
        let capabilities = DeviceCapabilities::capture(&adapter, &device, 54);
        let mut scene = SyntheticScene::grid(1).unwrap();
        let initial = AtmosphereFrame::new([0.0, 1.0, 0.0], 1.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            FrameLighting::Atmosphere(initial),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).unwrap();
        let mut cancelled =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared.encode(&mut cancelled, 54, &target, None).unwrap();
        drop(cancelled); // Cold DFG/sample-buffer recording did not reach the GPU.
        let cold = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert_eq!(cube_passes(&cold), 7);
        assert!(has_dfg(&cold));
        let cached = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert_eq!(cube_passes(&cached), 0);
        assert!(!has_dfg(&cached));
        let amplitude = AtmosphereFrame::new([0.0, 1.0, 0.0], 2.0, [1.0; 3], 2.0, 1).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(amplitude),
                1.0,
            )
            .unwrap();
        let amplitude_names = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert!(
            amplitude_names
                .iter()
                .any(|name| name == "atmosphere diffuse irradiance")
        );
        assert_eq!(cube_passes(&amplitude_names), 0);
        assert!(!has_dfg(&amplitude_names));
        scene.view = SyntheticView::perspective(
            [0.4, 0.2, 2.0],
            [0.0; 3],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(amplitude),
                1.0,
            )
            .unwrap();
        let view = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert_eq!(cube_passes(&view), 0);
        assert!(!has_dfg(&view));
        let changed = AtmosphereFrame::new([0.8, 0.6, 0.0], 2.0, [1.0; 3], 2.0, 1).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(changed),
                1.0,
            )
            .unwrap();
        let mut cancelled =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared.encode(&mut cancelled, 54, &target, None).unwrap();
        drop(cancelled); // No submit and no commit: none of its mip writes happened.
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(changed),
                1.0,
            )
            .unwrap();
        let retry = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert_eq!(cube_passes(&retry), 7);
        assert!(!has_dfg(&retry));
        let profile = AtmosphereFrame::new([0.8, 0.6, 0.0], 2.0, [1.0; 3], 2.0, 2).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(profile),
                1.0,
            )
            .unwrap();
        let rebuilt = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert!(
            rebuilt
                .iter()
                .any(|name| name == "atmosphere transmittance")
        );
        assert_eq!(cube_passes(&rebuilt), 7);
        assert!(
            !has_dfg(&rebuilt),
            "BRDF does not depend on atmosphere profile"
        );
        let altitude = AtmosphereFrame::new([0.8, 0.6, 0.0], 2.0, [1.0; 3], 1200.0, 2).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(altitude),
                1.0,
            )
            .unwrap();
        let altitude_names = submitted_names(&mut prepared, &target, &capabilities, &queue);
        assert_eq!(cube_passes(&altitude_names), 7);
        assert!(!has_dfg(&altitude_names));
        for frame in [altitude.with_nightglow(), altitude] {
            prepared
                .update_frame(
                    &queue,
                    &mut target,
                    &scene,
                    FrameLighting::Atmosphere(frame),
                    1.0,
                )
                .unwrap();
            let toggled = submitted_names(&mut prepared, &target, &capabilities, &queue);
            assert_eq!(
                cube_passes(&toggled),
                7,
                "night source changes all reflection mips"
            );
            for name in &super::super::STAGE_NAMES[2..5] {
                assert!(toggled.iter().any(|actual| actual == name), "{name}");
            }
            for name in super::super::STAGE_NAMES[..2]
                .iter()
                .chain(&super::super::STAGE_NAMES[5..7])
            {
                assert!(!toggled.iter().any(|actual| actual == name), "{name}");
            }
            let cached = submitted_names(&mut prepared, &target, &capabilities, &queue);
            assert!(
                cached.iter().all(|name| !name.starts_with("atmosphere ")),
                "a committed night profile leaves no repeated LUT work"
            );
        }
    });
}
