//! Shaded black and emissive panels, compared to the ordinary camera at the
//! ray's crossing. The allowance carries local stored-volume steps through
//! the unchanged decoder; no independent air integrator supplies expectations.
use super::*;
use crate::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    synthetic::{CaptureSource, FrameLighting, InstanceRecord},
};
use glam::Vec3;

const EXTENT: u32 = 65;
const PIXEL: usize = 32 * EXTENT as usize + 32;

fn camera(position: Vec3, direction: Vec3) -> SyntheticView {
    SyntheticView::perspective(
        position.to_array(),
        (position + direction * 8192.0).to_array(),
        Vec3::Y.to_array(),
        60.0_f32.to_radians(),
        1.0,
        0.1,
        10000.0,
    )
    .expect("panel camera")
}

fn lighting(height: f32, sun: Vec3) -> FrameLighting {
    FrameLighting::Atmosphere(
        AtmosphereFrame::new(sun.to_array(), 100_000.0, [1.0; 3], height, 1).expect("air"),
    )
}

async fn reference(
    capabilities: &DeviceCapabilities,
    queue: &wgpu::Queue,
    scene: &SyntheticScene,
    sun: Vec3,
) -> (Vec<[f32; 4]>, Vec<u16>) {
    let device = capabilities.device();
    let prepared = PreparedSynthetic::prepare_with_capabilities(
        capabilities,
        queue,
        scene,
        lighting(scene.view.camera_position()[1], sun),
        1.0,
    )
    .await
    .expect("reference scene");
    let target = OffscreenTarget::with_capture_source(
        device,
        &prepared,
        EXTENT,
        EXTENT,
        CaptureSource::SceneLinear,
    )
    .expect("reference target");
    let mut pixels = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode_with_readback(&mut encoder, 1, &target, None, &mut pixels)
        .expect("ordinary graph");
    let submitted = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .expect("reference completion");
    let mut bytes = Vec::new();
    assert!(
        pixels
            .try_collect_into(1, &mut bytes)
            .expect("reference pixels")
    );
    let pixels = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|pixel| {
            std::array::from_fn(|channel| {
                half::f16::from_le_bytes([pixel[channel * 2], pixel[channel * 2 + 1]]).to_f32()
            })
        })
        .collect();
    let volume = aerial::volume(
        device,
        queue,
        prepared.atmosphere.as_ref().expect("air").aerial_textures(),
    );
    (pixels, volume)
}

fn step(value: f32) -> f32 {
    let stored = half::f16::from_f32(value);
    half::f16::from_bits(stored.to_bits() + 1).to_f32() - stored.to_f32()
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the positive distances under test address the volume's 33 bounded layers"
)]
fn layers(distance: f32) -> (usize, f32) {
    let low = (((distance / 32000.0).sqrt() * 32.0).floor() as usize).min(31);
    let first = 32000.0 * (low as f32 / 32.0).powi(2);
    let second = 32000.0 * ((low + 1) as f32 / 32.0).powi(2);
    (low, ((distance - first) / (second - first)).clamp(0.0, 1.0))
}

fn texel(volume: &[u16], texture: usize, layer: usize, x: usize, y: usize, channel: usize) -> f32 {
    let offset = texture * 32 * 32 * 33 * 4 + (layer * 32 * 32 + y * 32 + x) * 4 + channel;
    half::f16::from_bits(volume[offset]).to_f32()
}

fn allowance(reference: [f32; 4], volume: &[u16], distance: f32, radiance: f32) -> [f32; 3] {
    let (low, _) = layers(distance);
    std::array::from_fn(|channel| {
        let mut q = 0.0;
        for (texture, scale) in [(0, 100_000.0 / 1024.0), (1, radiance)] {
            // ATMOSPHERE defines Q at the largest stored magnitude among
            // the texels read. Carry each stored channel through its decode.
            let maximum = [low, low + 1]
                .into_iter()
                .flat_map(|layer| {
                    [15, 16]
                        .into_iter()
                        .flat_map(move |y| [15, 16].map(move |x| (layer, x, y)))
                })
                .map(|(layer, x, y)| texel(volume, texture, layer, x, y, channel))
                .fold(0.0_f32, f32::max);
            q += step(maximum) * scale / 1024.0;
        }
        // Four filtered reads in the plain decoder, as SCENE-VIEWS specifies.
        step(reference[channel]) + 4.0 * q
    })
}

fn empty_never_crossing(volume: &[u16], distance: f32) -> [f32; 3] {
    let (low, blend) = layers(distance);
    std::array::from_fn(|channel| {
        // At the centre, each of rows 15 and 16 has half the weight. With
        // this level camera, row 16 points down and never crosses the plane.
        let sample = |layer| {
            [15, 16]
                .into_iter()
                .map(|x| texel(volume, 0, layer, x, 15, channel) * 0.25)
                .sum::<f32>()
        };
        ((1.0 - blend) * sample(low) + blend * sample(low + 1)) * 100_000.0 / (1024.0 * 1024.0)
    })
}

#[cfg(target_os = "macos")]
#[test]
fn plane_panels_match_the_ordinary_camera_at_each_crossing() {
    let Some(crate::test_device::Opened { device, queue, .. }) = crate::test_device::open(
        "plane_panels_match_the_ordinary_camera_at_each_crossing",
        Some(wgpu::Backend::Metal),
        crate::capabilities::DeviceProfile::Portable,
        &crate::synthetic::tests::support::NO_FEATURES,
    ) else {
        return;
    };
    pollster::block_on(async {
        let capabilities = DeviceCapabilities::capture_device(&device, 1);
        for (below, distance, degrees) in [
            (2.0, 20.0, 10.0_f32),
            (2.0, 3000.0, 10.0),
            (40.0, 20.0, 10.0),
            (40.0, 3000.0, 10.0),
            (2.0, 1000.0, 0.2),
        ] {
            let pitch = degrees.to_radians();
            let direction = Vec3::new(0.0, pitch.sin(), -pitch.cos());
            let eye = Vec3::new(0.0, 50.0 - below, 0.0);
            let crossing = eye + direction * (below / direction.y);
            for sun in [
                Vec3::new(
                    std::f32::consts::FRAC_1_SQRT_2,
                    std::f32::consts::FRAC_1_SQRT_2,
                    0.0,
                ),
                Vec3::new(
                    0.0,
                    10.0_f32.to_radians().sin(),
                    10.0_f32.to_radians().cos(),
                ),
            ] {
                for radiance in [0.0, 100.0] {
                    let mut scene = SyntheticScene::grid(1).expect("panel scene");
                    scene.view = camera(Vec3::new(0.0, 100.0, 0.0), direction);
                    let center = crossing + direction * distance;
                    let mut panel = InstanceRecord::identity(0, 0);
                    for axis in 0..3 {
                        let half = if axis == 2 { 0.01 } else { distance * 0.1 };
                        panel.current[axis][axis] = half;
                        panel.normal[axis][axis] = half.recip();
                        panel.current[3][axis] = center[axis];
                    }
                    panel.previous = panel.current;
                    scene.instances = vec![panel];
                    scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
                    scene.materials[0].surface[2] = 1.0;
                    scene.materials[0].surface[1] = 0.045;
                    scene.materials[0].emissive_cutoff = [radiance, radiance, radiance, 0.0];
                    let mut prepared = PreparedSynthetic::prepare_with_capabilities(
                        &capabilities,
                        &queue,
                        &scene,
                        lighting(100.0, sun),
                        1.0,
                    )
                    .await
                    .expect("scene");
                    prepared
                        .prepare_secondary_views(
                            &queue,
                            &[SecondaryViewSlot::new(1.0).expect("slot")],
                            [EXTENT, EXTENT],
                            &[],
                        )
                        .expect("view");
                    prepared.set_contribution_passes(ContributionPasses {
                        upload: true,
                        compute: false,
                    });
                    let target =
                        OffscreenTarget::new(&device, &prepared, EXTENT, EXTENT).expect("target");
                    let mut asker = Asker {
                        slot: prepared.secondary_handles[0].clone(),
                        request: Some(
                            SecondaryViewRequest::new(
                                &camera(eye, direction),
                                [EXTENT, EXTENT],
                                Some(
                                    crate::contribution::ViewClipPlane::new([0.0, 1.0, 0.0, -50.0])
                                        .expect("plane"),
                                ),
                                ViewAir::Plane,
                            )
                            .expect("ask"),
                        ),
                    };
                    let (actual, _) = render(&device, &queue, &prepared, &target, &mut asker);
                    assert_eq!(actual[PIXEL][3], 1.0, "panel covered");
                    scene.view = camera(crossing, direction);
                    let (expected, volume) = reference(&capabilities, &queue, &scene, sun).await;
                    if degrees < 1.0 {
                        let actual_volume = aerial::volume(
                            &device,
                            &queue,
                            prepared.secondary[0]
                                .aerial
                                .as_ref()
                                .expect("view air")
                                .textures(),
                        );
                        let mut maximum = 0.0_f32;
                        let mut changed = 0;
                        for (&a, &e) in actual_volume.iter().zip(&volume) {
                            let value = half::f16::from_bits(e).to_f32();
                            let steps =
                                (half::f16::from_bits(a).to_f32() - value).abs() / step(value);
                            maximum = maximum.max(steps);
                            changed += usize::from(steps > 1.0);
                        }
                        eprintln!(
                            "horizon volume max steps={maximum} channels beyond one step={changed}, actual={:?} expected={:?}, crossing={crossing:?}",
                            actual[PIXEL], expected[PIXEL]
                        );
                    }
                    let limit = allowance(
                        expected[PIXEL],
                        &volume,
                        distance - 0.01 / direction.z.abs(),
                        radiance,
                    );
                    for channel in 0..3 {
                        let difference = (actual[PIXEL][channel] - expected[PIXEL][channel]).abs();
                        eprintln!(
                            "panel below={below} stretch={distance} pitch={degrees} emission={radiance} channel={channel}: difference={difference} allowance={}",
                            limit[channel]
                        );
                        assert!(difference <= limit[channel], "crossing reference missed");
                    }
                    if degrees < 1.0 && radiance == 0.0 {
                        let emptied =
                            empty_never_crossing(&volume, distance - 0.01 / direction.z.abs());
                        for channel in 0..3 {
                            let difference = (actual[PIXEL][channel] - emptied[channel]).abs();
                            eprintln!(
                                "empty never-crossing row channel={channel}: difference={difference}, allowance={}",
                                limit[channel]
                            );
                            assert!(
                                difference > limit[channel],
                                "empty rows must fail the same allowance"
                            );
                        }
                    }
                }
            }
        }
    });
}
