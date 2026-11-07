use super::*;
use crate::atmosphere::AtmosphereFrame;
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::{FrameLighting, OffscreenTarget, SyntheticScene};

#[test]
fn opaque_depth_store_and_star_pass_follow_the_current_target_view() {
    pollster::block_on(async {
        let (device, queue, capabilities) = metal().await;
        let lighting = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, -1.0, 0.0], 127_500.0, [1.0; 3], 2.0, 1)
                .expect("Earth atmosphere"),
        );
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.star_catalogue = Some(&[[0.0, 0.0, -1.0, 9.6e-6]]);
        let day = SyntheticView::perspective(
            [0.0, 0.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("perspective view");
        let night = day
            .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
            .expect("celestial basis");
        scene.view = day;
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &capabilities,
            &queue,
            &scene,
            lighting,
            1.0,
        )
        .await
        .expect("star scene preparation");
        let mut target = OffscreenTarget::new(&device, &prepared, 65, 33).expect("target");
        assert_variant(&target, false);
        for (view, stars) in [(night, true), (day, false)] {
            scene.view = view;
            prepared
                .update_frame(&queue, &mut target, &scene, lighting, 1.0)
                .expect("update existing target");
            assert_variant(&target, stars);
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode(&mut encoder, 41, &target, None)
                .expect("execute selected graph");
            let submission = queue.submit([encoder.finish()]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .expect("submitted variant completes");
            prepared.commit_frame();
        }
    });
}

async fn metal() -> (wgpu::Device, wgpu::Queue, DeviceCapabilities) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Metal adapter required for the star graph regression");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Metal device");
    let capabilities = DeviceCapabilities::capture(&adapter, &device, 41);
    (device, queue, capabilities)
}

fn assert_variant(target: &OffscreenTarget, stars: bool) {
    for capture in [false, true] {
        let plan = target.graph(capture).plan();
        let passes = plan
            .pass_bindings()
            .map(|(_, pass)| pass)
            .collect::<Vec<_>>();
        let forward = passes
            .iter()
            .position(|pass| pass.name == "synthetic forward opaque")
            .expect("forward pass");
        let tone = passes
            .iter()
            .position(|pass| pass.name == "synthetic tone encode")
            .expect("display pass");
        let star = passes.iter().position(|pass| pass.name == PASS_NAME);
        assert_eq!(star.is_some(), stars);
        if let Some(star) = star {
            assert!(forward < star && star < tone);
        }
        let depth = passes[forward]
            .attachment_ops(&target.ids.depth, 0..1, 0.0)
            .expect("forward depth");
        assert_eq!(
            depth.store,
            if stars {
                wgpu::StoreOp::Store
            } else {
                wgpu::StoreOp::Discard
            }
        );
        assert_eq!(
            passes.last().expect("last pass").name,
            if capture {
                "output readback"
            } else {
                "synthetic tone encode"
            }
        );
    }
}

#[test]
fn above_horizon_control_separates_daylight_from_night() {
    pollster::block_on(async {
        let (device, queue, capabilities) = metal().await;
        let mut scene = SyntheticScene::grid(1).expect("scene");
        // Brightness exceeds Sirius by a factor of two. The ray is well above
        // the horizon and its projection is centred on an unobstructed sky.
        scene.star_catalogue = Some(&[[0.0, 0.8, -0.6, 2.0e-5]]);
        let day = atmosphere([0.866_025_4, 0.5, 0.0]);
        for extent in [[1280, 720], [1920, 1080], [3840, 2160]] {
            let [width, height] = extent;
            let view = SyntheticView::perspective(
                [0.0, 0.0, 3.0],
                [0.0, 0.8, 2.4],
                [0.0, 1.0, 0.0],
                0.8,
                width as f32 / height as f32,
                0.1,
                20.0,
            )
            .expect("unoccluded sky camera")
            .with_incident_exposure(0.0, -12.0, 18.0)
            .expect("showcase meter")
            .with_low_light_adaptation()
            .expect("low-light meter");
            let stars = view
                .with_celestial_basis(glam::Mat3::IDENTITY.to_cols_array_2d())
                .expect("control direction is already in world space");
            scene.view = view;
            let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                &capabilities,
                &queue,
                &scene,
                day,
                1.0,
            )
            .await
            .expect("prepare star-bearing atmosphere");
            let mut target =
                OffscreenTarget::new(&device, &prepared, width, height).expect("target");
            for (lighting, daylight) in [(day, true), (atmosphere([0.0, -1.0, 0.0]), false)] {
                let [without, with] = [view, stars].map(|view| {
                    scene.view = view;
                    prepared
                        .update_frame(&queue, &mut target, &scene, lighting, 1.0)
                        .expect("update star referee");
                    assert_variant(&target, view.celestial_basis().is_some());
                    capture(&device, &queue, &mut prepared, &target)
                });
                if daylight {
                    let delta = without
                        .iter()
                        .zip(&with)
                        .map(|(&a, &b)| a.abs_diff(b))
                        .max()
                        .expect("nonempty capture");
                    assert!(
                        delta <= 1,
                        "{extent:?} daylight control changes {delta} codes"
                    );
                } else {
                    let (width, height) = (width as usize, height as usize);
                    let increase = (height / 2 - 4..=height / 2 + 4)
                        .flat_map(|y| {
                            (width / 2 - 4..=width / 2 + 4).map(move |x| (y * width + x) * 4)
                        })
                        .flat_map(|offset| offset..offset + 3)
                        .map(|channel| with[channel].saturating_sub(without[channel]))
                        .max()
                        .expect("nonempty central footprint");
                    assert!(
                        increase >= 3,
                        "{extent:?} central night control is too dim: {increase}"
                    );
                }
            }
        }
    });
}

fn atmosphere(direction: [f32; 3]) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new(direction, 127_500.0, [1.0; 3], 2.0, 1)
            .expect("Earth atmosphere")
            .with_nightglow(),
    )
}

fn capture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &OffscreenTarget,
) -> Vec<u8> {
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 41, target, None, &mut readback)
        .expect("encode optical referee");
    let submission = queue.submit([encoder.finish()]);
    prepared.commit_frame();
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .expect("optical referee completes");
    readback
        .try_collect(41)
        .expect("readback")
        .expect("mapped pixels")
}
