//! First integration proof of slot preparation, upload asks and the opaque pass.
use super::*;
use crate::contribution::{ContributionFrame, ContributionPasses, SceneContribution, SceneSurface};
use crate::synthetic::{OffscreenTarget, SyntheticLight, SyntheticScene};
mod aerial;
mod casters;
mod limits;
mod panels;
mod sun;
mod visibility;

struct Asker {
    slot: SecondaryView,
    request: Option<SecondaryViewRequest>,
}

#[test]
fn secondary_fragment_variants_validate_with_the_shared_shading_source() {
    for atmosphere in [false, true] {
        for air_shadow in [false, true] {
            let source = shader_source(&crate::synthetic::scene_source::secondary(
                atmosphere,
                air_shadow && atmosphere,
                false,
                crate::synthetic::scene_source::SunShadows::from(air_shadow && atmosphere),
            ));
            let module = wgpu::naga::front::wgsl::parse_str(&source).expect("secondary WGSL");
            wgpu::naga::valid::Validator::new(
                wgpu::naga::valid::ValidationFlags::all(),
                wgpu::naga::valid::Capabilities::all(),
            )
            .validate(&module)
            .expect("secondary entry validation");
        }
    }
}

impl SceneContribution for Asker {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, _: &SceneSurface<'_>) -> Result<(), crate::runtime::FrameError> {
        Ok(())
    }
    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, _: &ContributionFrame) {
        if let Some(request) = &self.request {
            self.slot.request(request).expect("ask");
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn slot_asks_draw_a_camera_and_clip_without_touching_the_main_view() {
    let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
        "slot_asks_draw_a_camera_and_clip_without_touching_the_main_view",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::synthetic::tests::support::NO_FEATURES,
    ) else {
        return;
    };
    pollster::block_on(async {
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = SyntheticView::perspective(
            [0.0, 0.0, -2.0],
            [0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("main");
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].emissive_cutoff = [1024.0, 512.0, 256.0, 0.0];
        let mut prepared = PreparedSynthetic::prepare_with_light(
            &device,
            &queue,
            1,
            &scene,
            SyntheticLight {
                irradiance: [0.0; 3],
                ..SyntheticLight::default()
            },
        )
        .await
        .expect("prepared");
        prepared
            .prepare_secondary_views(
                &queue,
                &[SecondaryViewSlot::new(1.0).expect("declaration")],
                [64, 64],
                &scene.local_lights,
            )
            .expect("slots");
        prepared.set_contribution_passes(ContributionPasses {
            upload: true,
            compute: false,
        });
        let target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
        let slot = prepared.secondary_handles[0].clone();
        let camera = SyntheticView::perspective(
            [0.0, 0.0, -2.0],
            [0.0, 0.0, 0.5],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            20.0,
        )
        .expect("secondary");
        let mut asker = Asker {
            slot: slot.clone(),
            request: Some(
                SecondaryViewRequest::new(&camera, [64, 64], None, ViewAir::Camera)
                    .expect("request"),
            ),
        };
        let (full, main) = render(&device, &queue, &prepared, &target, &mut asker);
        let covered = full
            .iter()
            .filter(|pixel| pixel[3].to_bits() == 1.0_f32.to_bits())
            .count();
        assert!(covered > 20, "view should see the box: {covered}");
        for pixel in full.iter().filter(|pixel| pixel[3] > 0.0) {
            assert_eq!(*pixel, [1.0, 0.5, 0.25, 1.0]);
        }
        asker.request = Some(
            SecondaryViewRequest::new(
                &camera,
                [64, 64],
                Some(crate::contribution::ViewClipPlane::new([0.0, 1.0, 0.0, 0.0]).expect("plane")),
                ViewAir::Camera,
            )
            .expect("clipped request"),
        );
        let (clipped, main_clipped) = render(&device, &queue, &prepared, &target, &mut asker);
        assert_eq!(
            main, main_clipped,
            "clipping the secondary leaves main bytes intact"
        );
        let clipped_count = clipped.iter().filter(|pixel| pixel[3] > 0.0).count();
        assert!(
            clipped_count > 0 && clipped_count < covered,
            "clip removes half the box: {clipped_count}/{covered}"
        );
        asker.request = None;
        let (_, main_idle) = render(&device, &queue, &prepared, &target, &mut asker);
        assert_eq!(main, main_idle, "an idle slot leaves main bytes intact");
        assert!(
            slot.asked().is_none(),
            "asking does not survive the next upload"
        );
    });
}

fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
    asker: &mut Asker,
) -> (Vec<[f32; 4]>, Vec<u8>) {
    let asked = asker.request.is_some();
    let [width, height] = asker.slot.maximum_extent();
    let mut main = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_contributed(
            &mut encoder,
            1,
            target,
            None,
            Some(&mut main),
            Some(asker),
            None,
            &mut Vec::new(),
        )
        .expect("encode");
    let colour = &prepared.secondary_handles[0].colour;
    let readback = asked.then(|| {
        thrum_test_support::readback::Readback::texture(
            device,
            &mut encoder,
            colour,
            thrum_test_support::readback::Mip::level(0),
            ..,
        )
    });
    let submitted = queue.submit([encoder.finish()]);
    let view = if let Some(readback) = readback {
        let pixels = readback.wait(device, submitted);
        pixels
            .chunks_exact(colour.width() as usize * 8)
            .take(height as usize)
            .flat_map(|row| row[..width as usize * 8].as_chunks::<8>().0)
            .map(|pixel| {
                std::array::from_fn(|channel| {
                    half::f16::from_le_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]).to_f32()
                })
            })
            .collect()
    } else {
        // Nothing asked: the slot's view stays cleared, and the main readback needs the frame done.
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("readback");
        vec![[0.0; 4]; width as usize * height as usize]
    };
    let mut bytes = Vec::new();
    assert!(main.try_collect_into(1, &mut bytes).expect("main readback"));
    (view, bytes)
}

#[cfg(target_os = "macos")]
#[test]
fn slot_local_lights_match_the_ordinary_view_with_the_main_camera_away() {
    let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
        "slot_local_lights_match_the_ordinary_view_with_the_main_camera_away",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::synthetic::tests::support::NO_FEATURES,
    ) else {
        return;
    };
    pollster::block_on(async {
        let camera = |x| {
            SyntheticView::perspective(
                [0.0, x, -2.0],
                [0.0, x, 0.5],
                [0.0, 1.0, 0.0],
                1.0,
                1.0,
                0.1,
                20.0,
            )
            .expect("camera")
        };
        let mut scene = SyntheticScene::grid(1).expect("scene");
        scene.view = camera(100.0);
        scene.local_lights = vec![crate::synthetic::SyntheticPointLight {
            position: [0.0, 0.0, -0.2],
            radius: 3.0,
            candela: [500.0, 100.0, 20.0],
        }];
        let light = SyntheticLight {
            irradiance: [0.0; 3],
            ..SyntheticLight::default()
        };
        let mut prepared = PreparedSynthetic::prepare_with_light(&device, &queue, 1, &scene, light)
            .await
            .expect("prepared");
        prepared
            .prepare_secondary_views(
                &queue,
                &[SecondaryViewSlot::new(1.0).expect("slot")],
                [64, 64],
                &scene.local_lights,
            )
            .expect("views");
        prepared.set_contribution_passes(ContributionPasses {
            upload: true,
            compute: false,
        });
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("target");
        let mut asker = Asker {
            slot: prepared.secondary_handles[0].clone(),
            request: Some(
                SecondaryViewRequest::new(&camera(0.0), [64, 64], None, ViewAir::Camera)
                    .expect("ask"),
            ),
        };
        let (first, _) = render(&device, &queue, &prepared, &target, &mut asker);
        scene.view = camera(-100.0);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                crate::synthetic::FrameLighting::Diagnostic(light),
                1.0,
            )
            .expect("turn main");
        let (second, _) = render(&device, &queue, &prepared, &target, &mut asker);
        assert_eq!(
            first, second,
            "the requested image ignores the main light grid"
        );
        scene.view = camera(0.0);
        let reference = PreparedSynthetic::prepare_with_light(&device, &queue, 1, &scene, light)
            .await
            .expect("reference");
        let reference_target = OffscreenTarget::with_capture_source(
            &device,
            &reference,
            64,
            64,
            crate::synthetic::CaptureSource::SceneLinear,
        )
        .expect("reference target");
        let mut readback = reference_target.readback(&device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        reference
            .encode_with_readback(&mut encoder, 1, &reference_target, None, &mut readback)
            .expect("ordinary main");
        let submitted = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submitted),
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .expect("reference wait");
        let mut bytes = Vec::new();
        assert!(
            readback
                .try_collect_into(1, &mut bytes)
                .expect("reference pixels")
        );
        let mut covered = 0;
        let mut lit = 0;
        for (actual, pixel) in first.iter().zip(bytes.as_chunks::<8>().0) {
            if actual[3] <= 0.0 {
                continue;
            }
            covered += 1;
            lit += usize::from(actual[0] > 0.001);
            for channel in 0..3 {
                let expected =
                    half::f16::from_le_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]);
                let step =
                    half::f16::from_bits(expected.to_bits() + 1).to_f32() - expected.to_f32();
                assert!(
                    (actual[channel] - expected.to_f32()).abs() <= step,
                    "covered channel differs: {} vs {}, step {step}",
                    actual[channel],
                    expected.to_f32()
                );
            }
        }
        assert!(
            covered > 20 && lit > 20,
            "reference rejects an unlit or empty substitute: {covered}/{lit}"
        );
    });
}
