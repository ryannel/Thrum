//! The ordinary main-view renderer is the profile reference. Its camera and
//! cascade preparation are independent of the secondary path under test.
use super::*;
use crate::synthetic::{CaptureSource, InstanceRecord};

pub(super) fn box_instance(slot: u32, center: [f32; 3], half: [f32; 3]) -> InstanceRecord {
    let mut instance = InstanceRecord::identity(slot, 0);
    for axis in 0..3 {
        instance.current[axis][axis] = half[axis];
        instance.normal[axis][axis] = half[axis].recip();
        instance.current[3][axis] = center[axis];
    }
    instance.previous = instance.current;
    instance
}

async fn ordinary(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    light: SyntheticLight,
) -> Vec<[f32; 4]> {
    let prepared = PreparedSynthetic::prepare_with_light(device, queue, 1, scene, light)
        .await
        .expect("reference");
    let target =
        OffscreenTarget::with_capture_source(device, &prepared, 64, 64, CaptureSource::SceneLinear)
            .expect("reference target");
    let mut pixels = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 1, &target, None, &mut pixels)
        .expect("ordinary main view");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("reference wait");
    let mut bytes = Vec::new();
    assert!(pixels.try_collect_into(1, &mut bytes).expect("pixels"));
    bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|pixel| {
            std::array::from_fn(|channel| {
                half::f16::from_le_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]).to_f32()
            })
        })
        .collect()
}

#[cfg(target_os = "macos")]
#[test]
fn view_sun_profiles_match_ordinary_main_without_borrowing_its_cascades() {
    let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
        "view_sun_profiles_match_ordinary_main_without_borrowing_its_cascades",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::synthetic::tests::support::NO_FEATURES,
    ) else {
        return;
    };
    pollster::block_on(async {
        let camera = |forward| {
            SyntheticView::perspective(
                [0.0, 6.0, 0.0],
                [0.0, 0.0, forward],
                [0.0, 1.0, 0.0],
                0.8,
                1.0,
                0.1,
                100.0,
            )
            .expect("camera")
        };
        let mut scene = SyntheticScene::grid(2).expect("scene");
        scene.sun_shadows = true;
        scene.instances = vec![
            box_instance(0, [0.0, -0.5, -15.0], [12.0, 0.5, 12.0]),
            box_instance(1, [1.0, 2.0, -15.0], [1.0, 2.0, 1.0]),
        ];
        let light = SyntheticLight {
            direction_to_light: [
                std::f32::consts::FRAC_1_SQRT_2,
                std::f32::consts::FRAC_1_SQRT_2,
                0.0,
            ],
            irradiance: [1.0; 3],
        };
        let mut profiles = Vec::new();
        for count in 0..=2 {
            scene.view = camera(15.0);
            let mut prepared =
                PreparedSynthetic::prepare_with_light(&device, &queue, 1, &scene, light)
                    .await
                    .expect("scene");
            prepared
                .prepare_secondary_views(
                    &queue,
                    &[SecondaryViewSlot::new(1.0).expect("slot")],
                    [64, 64],
                    &[],
                )
                .expect("views");
            prepared.set_contribution_passes(ContributionPasses {
                upload: true,
                compute: false,
            });
            let target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
            let mut asker = Asker {
                slot: prepared.secondary_handles[0].clone(),
                request: Some(
                    SecondaryViewRequest::new(&camera(-15.0), [64, 64], None, ViewAir::Camera)
                        .expect("request")
                        .with_shadows(count, 64.0)
                        .expect("profile"),
                ),
            };
            let (actual, _) = render(&device, &queue, &prepared, &target, &mut asker);
            scene.view = camera(-15.0)
                .with_sun_shadow_profile(count, 64.0)
                .expect("reference profile");
            let reference = ordinary(&device, &queue, &scene, light).await;
            let mut covered = 0;
            let mut max_steps = 0.0_f32;
            for (actual, expected) in actual.iter().zip(&reference) {
                if actual[3] == 0.0 {
                    continue;
                }
                covered += 1;
                for channel in 0..3 {
                    let stored = half::f16::from_f32(expected[channel]);
                    let step =
                        half::f16::from_bits(stored.to_bits() + 1).to_f32() - expected[channel];
                    let steps = (actual[channel] - expected[channel]).abs() / step;
                    max_steps = max_steps.max(steps);
                    assert!(
                        steps <= 1.0,
                        "count={count}, channel={channel}: actual={} expected={} steps={steps}",
                        actual[channel],
                        expected[channel]
                    );
                }
            }
            eprintln!(
                "Sun profile {count}: {covered} covered pixels, maximum {max_steps} stored steps"
            );
            assert!(covered > 500);
            profiles.push(actual);
        }
        assert_ne!(
            profiles[0], profiles[1],
            "finite cascades must change the height-map-only result"
        );
    });
}
