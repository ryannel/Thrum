//! Real Metal attachment readback; no reimplementation of the forward shader.
use super::*;
use glam::{Mat3, Mat4, Vec3, Vec4};

fn decode_half(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = u32::from((bits >> 10) & 31);
    let fraction = u32::from(bits & 1023);
    if exponent == 0 {
        let value = fraction as f32 * 2.0_f32.powi(-24);
        return if sign == 0 { value } else { -value };
    }
    f32::from_bits(sign | ((exponent + 112) << 23) | (fraction << 13))
}

async fn metal() -> (wgpu::Device, wgpu::Queue, wgpu::AdapterInfo) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("checked Metal motion witness");
    assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("checked Metal motion witness");
    (device, queue, info)
}

fn frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &mut PreparedSynthetic,
    target: &mut OffscreenTarget,
    scene: &SyntheticScene,
    lighting: FrameLighting,
) -> Vec<[f32; 2]> {
    prepared
        .update_frame(queue, target, scene, lighting, 1.0)
        .expect("checked Metal motion witness");
    prepared
        .upload_instances(queue, &scene.instances, &[0])
        .expect("checked Metal motion witness");
    let [width, height] = target.extent();
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("M1 real motion readback"),
        size: u64::from(stride * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared
        .encode(&mut encoder, 41, target, None)
        .expect("checked Metal motion witness");
    encoder.copy_texture_to_buffer(
        target
            .textures
            .texture(&target.ids.motion)
            .expect("checked Metal motion witness")
            .as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).expect("checked Metal motion witness");
    });
    queue.submit([encoder.finish()]);
    prepared.commit_frame();
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("checked Metal motion witness");
    receive
        .recv()
        .expect("checked Metal motion witness")
        .expect("checked Metal motion witness");
    let bytes = buffer
        .slice(..)
        .get_mapped_range()
        .expect("checked Metal motion witness");
    let mut values = Vec::new();
    for row in bytes.chunks_exact(stride as usize) {
        for texel in row[..width as usize * 4].as_chunks::<4>().0 {
            values.push(std::array::from_fn(|axis| {
                decode_half(u16::from_le_bytes(
                    texel[axis * 2..axis * 2 + 2]
                        .try_into()
                        .expect("checked Metal motion witness"),
                ))
            }));
        }
    }
    drop(bytes);
    buffer.unmap();
    values
}

#[test]
fn metal_static_jitter_is_zero_at_every_texel_and_motion_control_is_nonzero() {
    pollster::block_on(async {
        let (device, queue, info) = metal().await;
        println!("M1 target mac-metal: {info:?}");
        let mut scene = SyntheticScene::grid(1).expect("checked Metal motion witness");
        scene.view = scene.view.with_jitter_phase(0, [64, 64]);
        let mut prepared = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .expect("checked Metal motion witness");
        let mut target =
            OffscreenTarget::new(&device, &prepared, 64, 64).expect("checked Metal motion witness");
        let light = FrameLighting::Diagnostic(SyntheticLight::default());
        let first = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
        assert!(
            first.contains(&[65504.0; 2]),
            "spawn has explicit invalid motion"
        );
        let first_colour = submitted_colour(&device, &queue, &target);
        let mut jitter_changed_colour = false;
        for _ in 0..35 {
            let values = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
            jitter_changed_colour |= submitted_colour(&device, &queue, &target) != first_colour;
            assert!(
                values
                    .iter()
                    .all(|&value| value.map(|axis| axis.to_bits() & 0x7fff_ffff) == [0; 2]),
                "any nonzero texel fails"
            );
        }
        assert!(
            jitter_changed_colour,
            "explicit raster jitter must change the colour control"
        );
        println!("M1 static: 35 jitter phases, all 4096 texels numerically exact zero per frame");
        scene.view = scene.view.with_jitter_sequence(0, [64, 64]);
        frame(&device, &queue, &mut prepared, &mut target, &scene, light);
        let spatial_colour = submitted_colour(&device, &queue, &target);
        for _ in 0..35 {
            let phase = prepared.jitter_phase;
            let values = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
            let submitted = prepared.view_sample;
            let record = submitted.pack_view(&submitted, 1.0);
            assert_eq!(&record[228..232], &phase.to_le_bytes());
            assert_ne!(&record[152..160], &[0; 8]);
            assert_eq!(&record[..64], &record[160..224]);
            assert_eq!(prepared.jitter_phase, (phase + 1) % 32);
            assert!(
                values
                    .iter()
                    .all(|v| v.iter().all(|axis| axis.abs().to_bits() == 0))
            );
            assert_eq!(submitted_colour(&device, &queue, &target), spatial_colour);
        }
        println!(
            "MF1 spatial: 35 submissions retain nonzero sample offsets and advance phase with raster jitter disabled"
        );
        let mut replacement = PreparedSynthetic::prepare(&device, &queue, 41, &scene)
            .await
            .expect("replacement preparation");
        replacement.inherit_submitted_view(&prepared);
        assert_eq!(
            replacement.view_sample, prepared.view_sample,
            "an unsubmitted preparation must not replace the submitted predecessor"
        );
        assert_eq!(
            replacement.jitter_phase, prepared.jitter_phase,
            "resource replacement preserves the continuous view sequence"
        );
        scene.instances[0].current[3][0] += 0.1;
        let moved = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
        assert!(moved.iter().any(|v| v[0] < -1.0 && v[0] > -100.0));
        scene.view = scene.view.with_history_reset(true);
        let cut = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
        assert!(cut.contains(&[65504.0; 2]));
    });
}

fn submitted_colour(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    target: &OffscreenTarget,
) -> Vec<u8> {
    let mut readback = target.readback(device);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    readback
        .copy_into(&mut encoder, target, None)
        .expect("submitted colour copy");
    queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("colour completion");
    readback
        .try_collect(41)
        .expect("colour readback")
        .expect("completed colour")
}

fn camera(yaw: f32, phase: f32) -> SyntheticView {
    SyntheticView::perspective(
        [0.0, 2.0, 0.0],
        [yaw.sin(), 2.0, -yaw.cos()],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        100.0,
    )
    .expect("checked Metal motion witness")
    .with_celestial_basis(Mat3::from_rotation_y(phase).to_cols_array_2d())
    .expect("checked Metal motion witness")
}

fn solar_pixel_motion(angle: f64) -> [f64; 2] {
    let tangent = (0.5_f64).tan();
    let sx = tangent / 64.0;
    let sy = sx; // visible upper-half disk pixel (32,31)
    let previous_x = angle.cos() * sx - angle.sin();
    let previous_z = -angle.sin() * sx - angle.cos();
    [
        (previous_x / (-previous_z) / tangent - 1.0 / 64.0) * 32.0,
        (sy / (-previous_z) / tangent - 1.0 / 64.0) * (-32.0),
    ]
}

#[test]
fn metal_perspective_geometry_sky_and_stars_are_zero_under_static_jitter() {
    pollster::block_on(async {
        let (device, queue, info) = metal().await;
        println!("M1 static infinity target mac-metal: {info:?}");
        let mut scene = SyntheticScene::grid(1).expect("one foreground object");
        scene.instances[0].current[3] = [0.0, 2.0, -3.0, 1.0];
        scene.instances[0].previous = scene.instances[0].current;
        scene.star_catalogue = Some(&[[0.0, 0.15, -0.988_686, 9.6e-6]]);
        scene.view = camera(0.02, 0.0).with_jitter_phase(0, [64, 64]);
        let light = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, -1.0, 0.0], 127_500.0, [1.0; 3], 2.0, 1)
                .expect("night atmosphere"),
        );
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &DeviceCapabilities::capture_device(&device, 41),
            &queue,
            &scene,
            light,
            1.0,
        )
        .await
        .expect("perspective temporal scene");
        let mut target = OffscreenTarget::new(&device, &prepared, 64, 64).expect("motion target");
        frame(&device, &queue, &mut prepared, &mut target, &scene, light);
        for phase in 0..35 {
            let pixels = frame(&device, &queue, &mut prepared, &mut target, &scene, light);
            let unexpected: Vec<_> = pixels
                .iter()
                .enumerate()
                .filter(|(_, value)| value.map(|axis| axis.to_bits() & 0x7fff_ffff) != [0; 2])
                .take(8)
                .collect();
            assert!(
                unexpected.is_empty(),
                "static perspective phase {phase}: {unexpected:?}"
            );
        }
        println!(
            "M1 static perspective: geometry, sky and stars, 35 phases, all 4096 texels exact zero"
        );
    });
}

#[test]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the projected fixture star is inside the 64-pixel target"
)]
fn metal_sky_pan_and_solar_sidereal_phase_match_independent_projection() {
    pollster::block_on(async {
        let (device, queue, info) = metal().await;
        println!("M1 target mac-metal: {info:?}");
        let mut scene = SyntheticScene::grid(1).expect("checked Metal motion witness");
        scene.instances[0].ids[3] = 0; // unobstructed infinity witness
        scene.star_catalogue = Some(&[[0.0, 0.15, -0.988_686, 9.6e-6]]);
        scene.view = camera(0.0, 0.0);
        let night = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, -1.0, 0.0], 127_500.0, [1.0; 3], 2.0, 1)
                .expect("checked Metal motion witness"),
        );
        let mut prepared = PreparedSynthetic::prepare_with_capabilities(
            &DeviceCapabilities::capture_device(&device, 41),
            &queue,
            &scene,
            night,
            1.0,
        )
        .await
        .expect("checked Metal motion witness");
        let mut target =
            OffscreenTarget::new(&device, &prepared, 64, 64).expect("checked Metal motion witness");
        frame(&device, &queue, &mut prepared, &mut target, &scene, night);
        // CPU ray from focal length and camera yaw, rather than the shader inverse.
        scene.view = camera(0.02, 0.0);
        let pan = frame(&device, &queue, &mut prepared, &mut target, &scene, night);
        let tangent = (0.5_f64).tan();
        for (x, y) in [(5_u32, 5_u32), (32, 32), (58, 58)] {
            let sx = (2.0 * (f64::from(x) + 0.5) / 64.0 - 1.0) * tangent;
            let sy = (1.0 - 2.0 * (f64::from(y) + 0.5) / 64.0) * tangent;
            let a = 0.02_f64;
            let world = [a.cos() * sx + a.sin(), sy, a.sin() * sx - a.cos()];
            let previous_x = world[0] / (-world[2]) / tangent;
            let previous_y = world[1] / (-world[2]) / tangent;
            let expected = [
                (previous_x - (2.0 * (f64::from(x) + 0.5) / 64.0 - 1.0)) * 32.0,
                (previous_y - (1.0 - 2.0 * (f64::from(y) + 0.5) / 64.0)) * (-32.0),
            ];
            let actual = pan[(y * 64 + x) as usize];
            println!("M1 sky pan {x},{y}: GPU {actual:?}, independent CPU {expected:?}");
            for axis in 0..2 {
                assert!(
                    (f64::from(actual[axis]) - expected[axis]).abs() < 0.05,
                    "sky pan {x},{y} actual={actual:?} expected={expected:?}"
                );
            }
        }
        // An encoded but unsubmitted camera must never become the predecessor.
        let submitted_view = scene.view;
        let submitted_phase = prepared.jitter_phase;
        scene.view = camera(-0.4, 0.0);
        prepared
            .update_frame(&queue, &mut target, &scene, night, 1.0)
            .expect("abandoned view");
        let mut abandoned =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode(&mut abandoned, 41, &target, None)
            .expect("unsubmitted encoding");
        drop(abandoned);
        assert_eq!(prepared.jitter_phase, submitted_phase);
        scene.view = submitted_view;
        let after_abandon = frame(&device, &queue, &mut prepared, &mut target, &scene, night);
        let unexpected: Vec<_> = after_abandon
            .iter()
            .enumerate()
            .filter(|(_, value)| value.map(|axis| axis.to_bits() & 0x7fff_ffff) != [0; 2])
            .take(8)
            .collect();
        // Keep the exact-zero assertion, but report all independent phase witnesses
        // before failing so an abandoned-view defect cannot hide their evidence.
        println!("M1 after abandoned view, first nonzero texels: {unexpected:?}");
        // Hold the camera, rotate the catalogue only: star pixels must move, sky stays zero.
        scene.view = camera(0.02, 0.01);
        let phase = frame(&device, &queue, &mut prepared, &mut target, &scene, night);
        let ray = Vec3::new(0.0, 0.15, -0.988_686);
        let project = |phase| {
            let view = Mat4::from_cols_array_2d(&camera(0.02, phase).view_proj_columns());
            let clip = view * Mat4::from_rotation_y(phase) * Vec4::from((ray, 0.0));
            clip.truncate() / clip.w
        };
        let now = project(0.01);
        let before = project(0.0);
        let pixel = [
            ((now.x * 0.5 + 0.5) * 64.0) as usize,
            ((0.5 - now.y * 0.5) * 64.0) as usize,
        ];
        let value = phase[pixel[1] * 64 + pixel[0]];
        let expected = [(before.x - now.x) * 32.0, (before.y - now.y) * (-32.0)];
        println!("M1 sidereal phase only: GPU {value:?}, CPU {expected:?}");
        let sidereal_expected = expected;
        let sidereal_actual = value;
        let steady_sky = phase[0];
        println!("M1 steady sky during sidereal phase edit: {steady_sky:?}");
        // Solar disk: held camera and a rigid solar rotation, read its actual pixels.
        scene.star_catalogue = None;
        let sun = |angle: f32| {
            FrameLighting::Atmosphere(
                AtmosphereFrame::new(
                    [angle.sin(), 0.0, -angle.cos()],
                    127_500.0,
                    [1.0; 3],
                    2.0,
                    1,
                )
                .expect("checked Metal motion witness"),
            )
        };
        scene.view = camera(0.0, 0.0);
        frame(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            sun(0.0),
        );
        let solar = frame(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            sun(0.01),
        );
        let expected = solar_pixel_motion(0.01);
        let actual = solar[31 * 64 + 32];
        println!("M1 solar phase only: GPU {actual:?}, independent CPU {expected:?}");
        for axis in 0..2 {
            assert!(
                (f64::from(actual[axis]) - expected[axis]).abs() < 0.05,
                "solar phase {actual:?} vs {expected:?}"
            );
        }
        assert!(actual[0].abs() > 0.1, "solar phase-only disk motion");
        let small = frame(
            &device,
            &queue,
            &mut prepared,
            &mut target,
            &scene,
            sun(0.0101),
        );
        let actual = small[31 * 64 + 32];
        let expected = solar_pixel_motion(0.0001);
        println!("M1 small solar phase: GPU {actual:?}, CPU {expected:?}");
        assert!(
            actual[0].abs() > 0.001,
            "small solar steps cannot snap to zero"
        );
        for axis in 0..2 {
            assert!((f64::from(actual[axis]) - expected[axis]).abs() < 0.0001);
        }
        for axis in 0..2 {
            assert!(
                (sidereal_actual[axis] - sidereal_expected[axis]).abs() < 0.05,
                "sidereal phase {sidereal_actual:?} vs {sidereal_expected:?}"
            );
        }
        assert!(sidereal_actual[0].abs() > 0.1);
        assert_eq!(steady_sky.map(|axis| axis.to_bits() & 0x7fff_ffff), [0; 2]);
        assert!(
            unexpected.is_empty(),
            "after abandoned view: {unexpected:?}"
        );
    });
}
