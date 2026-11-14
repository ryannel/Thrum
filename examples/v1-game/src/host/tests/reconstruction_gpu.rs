//! Explicit game-path M3 oracle, with independent prepared reference and reset hosts.
use super::super::*;
use std::{io::Write, path::Path, time::Duration};
use thrum_render::{capabilities::DeviceCapabilities, present::SurfaceProfile};

const EXTENT: [u32; 2] = [640, 360];

fn submitted(
    host: &mut FrameHost,
    device: &wgpu::Device,
    surface: &SurfaceOutput<'_>,
) -> (Vec<u8>, FrameSample, [u32; 8]) {
    let report = host
        .drive(Duration::ZERO, Some(surface))
        .expect("held game frame");
    assert!(report.submitted && report.completed_ticks == 0 && !report.dropped_view);
    let sample = loop {
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        if let Some(sample) = host.poll_completed().unwrap() {
            break sample;
        }
    };
    let image = loop {
        if let Some(image) = host.try_submitted_image().unwrap() {
            break image;
        }
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    };
    assert_eq!(image.sample.submission_order, sample.submission_order);
    assert_eq!(image.source.tick, 0);
    let exposure = image.exposure_gpu_words.expect("submitted exposure record");
    let pixels = image.rgba.clone();
    host.recycle_submitted_image(image);
    (pixels, sample, exposure)
}

fn ppm(path: &Path, bytes: &[u8]) {
    let mut output = format!("P6\n{} {}\n255\n", EXTENT[0], EXTENT[1]).into_bytes();
    for pixel in bytes.as_chunks::<4>().0 {
        output.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, output).unwrap();
}

fn submitted_sdr(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    surface: &SurfaceOutput<'_>,
) -> Vec<u8> {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("M3 submitted identity-present copy"),
        size: 640 * 360 * 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        surface.view.texture().as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(640 * 4),
                rows_per_image: Some(360),
            },
        },
        surface.view.texture().size(),
    );
    let (send, receive) = std::sync::mpsc::channel();
    encoder.map_buffer_on_submit(&buffer, wgpu::MapMode::Read, .., move |result| {
        send.send(result).unwrap();
    });
    queue.submit([encoder.finish()]);
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    receive.recv().unwrap().unwrap();
    let mut bytes = buffer.slice(..).get_mapped_range().unwrap().to_vec();
    for pixel in bytes.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    bytes
}

fn half(bits: u16) -> f32 {
    let sign = u32::from(bits & 0x8000) << 16;
    let exponent = u32::from((bits >> 10) & 31);
    let fraction = u32::from(bits & 1023);
    if exponent == 0 {
        let value = f32::from(bits & 1023) * 2.0_f32.powi(-24);
        return if sign == 0 { value } else { -value };
    }
    f32::from_bits(
        sign | if exponent == 31 {
            0x7f80_0000 | (fraction << 13)
        } else {
            ((exponent + 112) << 23) | (fraction << 13)
        },
    )
}

fn linear_cycle(
    sum: &mut [[f32; 3]],
    first: &mut Option<Vec<[f32; 3]>>,
    bytes: &[u8],
    frame: usize,
    directory: &Path,
) {
    assert_eq!(bytes.len(), 640 * 360 * 8);
    for (sum, pixel) in sum.iter_mut().zip(bytes.as_chunks::<8>().0) {
        for (channel, value) in sum.iter_mut().enumerate() {
            let sample = half(u16::from_le_bytes([
                pixel[channel * 2],
                pixel[channel * 2 + 1],
            ]));
            assert!(sample.is_finite(), "nonfinite submitted linear sample");
            *value += sample;
        }
    }
    if frame % 32 != 31 {
        return;
    }
    let average: Vec<_> = sum.iter().map(|pixel| pixel.map(|v| v / 32.0)).collect();
    let raw: Vec<_> = average
        .iter()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    std::fs::write(
        directory.join(format!("reference-linear-{}.f32le", frame + 1)),
        raw,
    )
    .unwrap();
    if let Some(previous) = first {
        let mut maximum_absolute = 0.0_f32;
        let mut maximum_rounding_units = 0.0_f32;
        for (a, b) in previous.iter().flatten().zip(average.iter().flatten()) {
            let difference = (a - b).abs();
            // Two 32-term f32 means: a conservative combined summation bound.
            let bound = 64.0 * f32::EPSILON * a.abs().max(b.abs()).max(f32::MIN_POSITIVE);
            maximum_absolute = maximum_absolute.max(difference);
            maximum_rounding_units = maximum_rounding_units.max(difference / bound);
        }
        let report = serde_json::json!({"pixels":sum.len(),"phases_per_cycle":32,"cycles":2,"maximum_absolute_difference":maximum_absolute,"maximum_rounding_units":maximum_rounding_units,"within_float_rounding":maximum_rounding_units<=1.0,"source":"T2 submitted SceneLinear RGB16F; independent ordered f32 means"});
        std::fs::write(
            directory.join("linear-cycles.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        assert!(
            maximum_rounding_units <= 1.0,
            "linear cycle means differ: {report}"
        );
    } else {
        *first = Some(average);
    }
    sum.fill([0.0; 3]);
}

async fn host(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    mode: &str,
    preset: &serde_json::Value,
) -> (
    FrameHost,
    std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>,
) {
    let mut setup = crate::selected_setup().unwrap();
    setup.reconstruction = match mode {
        "reference" => thrum_render::synthetic::Reconstruction::Reference,
        "off" => thrum_render::synthetic::Reconstruction::Off,
        _ => thrum_render::synthetic::Reconstruction::Taau,
    };
    let mut host = FrameHost::new_with_setup(
        device,
        queue,
        caps,
        FrameConfig {
            render_extent: EXTENT,
            surface: Some(SurfaceProfile {
                format: wgpu::TextureFormat::Bgra8Unorm,
                extent: EXTENT,
            }),
        },
        setup,
    )
    .await
    .unwrap();
    let stats = crate::attach_cooked_scene(&mut host)
        .unwrap()
        .expect("cooked valley");
    host.pause().unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        drop(host.capture_pixels().unwrap());
        let stats = stats.lock().unwrap();
        assert!(stats.errors.is_empty(), "{:?}", stats.errors);
        if stats.drawn == Some(1) {
            break;
        }
        assert!(Instant::now() < deadline, "cooked residency");
    }
    host.set_time_of_day(
        serde_json::from_value::<f32>(preset["sky.solar_time"].clone()).unwrap() / 24.0,
    )
    .unwrap();
    host.set_camera_yaw(
        serde_json::from_value::<f32>(preset["camera.yaw"].clone())
            .unwrap()
            .to_radians(),
    )
    .unwrap();
    host.set_camera_pitch(
        serde_json::from_value::<f32>(preset["camera.pitch"].clone())
            .unwrap()
            .to_radians(),
    )
    .unwrap();
    host.enable_submitted_capture_from(
        1,
        if mode == "reference" {
            thrum_render::synthetic::CaptureSource::SceneLinear
        } else {
            thrum_render::synthetic::CaptureSource::Sdr
        },
    )
    .unwrap();
    (host, stats)
}

async fn checkpoint(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    surface: &SurfaceOutput<'_>,
    output: &Path,
    name: &str,
) {
    let preset: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("params/showcase-{name}.params.json")).unwrap(),
    )
    .unwrap();
    let directory = output.join(name);
    std::fs::create_dir_all(&directory).unwrap();
    let mut log = std::fs::File::create(directory.join("frames.jsonl")).unwrap();
    for mode in ["reference", "taau", "off", "broken"] {
        let (mut host, stats) = host(device, queue, caps, mode, &preset).await;
        let identity = (
            host.tick(),
            host.time_of_day().to_bits(),
            host.camera_orientation().map(f32::to_bits),
            host.scene_camera().unwrap().eye().map(f32::to_bits),
        );
        let mut held_exposure = None;
        let mut held_celestial = None;
        let mut reference_sum = vec![[0.0; 3]; 640 * 360];
        let mut first_reference = None;
        for frame in 0..64 {
            if mode == "broken" {
                host.inner.invalidate_history();
            }
            let (bytes, sample, exposure) = submitted(&mut host, device, surface);
            if mode == "reference" {
                linear_cycle(
                    &mut reference_sum,
                    &mut first_reference,
                    &bytes,
                    frame,
                    &directory,
                );
            }
            let state = (
                host.tick(),
                host.time_of_day().to_bits(),
                host.camera_orientation().map(f32::to_bits),
                host.scene_camera().unwrap().eye().map(f32::to_bits),
            );
            assert_eq!(state, identity, "held world/camera/solar state");
            let content = stats.lock().unwrap();
            assert_eq!(content.drawn, Some(1), "held cooked generation");
            assert!(content.errors.is_empty(), "{:?}", content.errors);
            let metering = exposure[..4].to_vec();
            let celestial = DayCycle::sample(
                host.tick(),
                sample.presentation_alpha,
                host.inner.source().day_phase_offset,
            );
            let celestial_state = (
                celestial.simulation_time_seconds.to_bits(),
                celestial
                    .equatorial_to_world
                    .map(|column| column.map(f32::to_bits)),
                sample.presentation_alpha.to_bits(),
            );
            assert_eq!(
                *held_celestial.get_or_insert(celestial_state),
                celestial_state,
                "held celestial phase and sampling alpha"
            );
            assert_eq!(
                *held_exposure.get_or_insert_with(|| metering.clone()),
                metering,
                "held exposure and storage p"
            );
            let record = serde_json::json!({
                "mode":mode,"frame":frame,"state":format!("{state:?}"),
                "celestial":celestial_state,"cooked_generation":content.drawn,
                "exposure":exposure,"history":sample.history_identity,
                "passes":sample.gpu_pass_ns.as_ref().map(|t|t.passes().iter().map(|p|
                    serde_json::json!({"name":p.name.as_ref(),"ns":p.nanoseconds})).collect::<Vec<_>>()),
            });
            writeln!(log, "{record}").unwrap();
            if frame == 31 || frame == 63 {
                let sdr = if mode == "reference" {
                    submitted_sdr(device, queue, surface)
                } else {
                    bytes
                };
                ppm(&directory.join(format!("{mode}-{}.ppm", frame + 1)), &sdr);
            }
        }
    }
}

#[test]
#[ignore = "explicit cooked-valley reconstruction oracle; run run_v1_showcase.py reconstruction"]
fn reconstruction_reference_on_submitted_game_frames() {
    pollster::block_on(async {
        let output =
            std::path::PathBuf::from(std::env::var("THRUM_RECONSTRUCTION_OUTPUT").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter)
                    | DeviceCapabilities::requested_content_features(&adapter),
                ..Default::default()
            })
            .await
            .unwrap();
        let caps = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
        std::fs::write(
            output.join("build.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "executable":std::env::current_exe().unwrap(),"adapter":adapter.get_info().name,
                "backend":format!("{:?}",adapter.get_info().backend),"extent":EXTENT,
            }))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            caps.recipe().unwrap(),
            thrum_render::capabilities::Recipe::Portable
        );
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("M3 game proof presentation"),
            size: wgpu::Extent3d {
                width: EXTENT[0],
                height: EXTENT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let surface = SurfaceOutput {
            view: &view,
            extent: EXTENT,
        };
        for name in [
            "sunrise",
            "noon",
            "sunset-clear-sun",
            "sunset-sun",
            "twilight",
            "night-upslope",
        ] {
            checkpoint(&device, &queue, &caps, &surface, &output, name).await;
        }
        Box::pin(cut(&device, &queue, &caps, &surface, &output)).await;
    });
}

async fn cut(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    caps: &DeviceCapabilities,
    surface: &SurfaceOutput<'_>,
    output: &Path,
) {
    let preset: serde_json::Value =
        serde_json::from_str(include_str!("../../../params/showcase-noon.params.json")).unwrap();
    let (mut actual, _) = host(device, queue, caps, "taau", &preset).await;
    let (mut suppressed, _) = host(device, queue, caps, "taau", &preset).await;
    for _ in 0..64 {
        submitted(&mut actual, device, surface);
        submitted(&mut suppressed, device, surface);
    }
    let (mut reset, _) = host(device, queue, caps, "taau", &preset).await;
    let camera = actual.scene_camera().unwrap();
    let mut eye = camera.eye();
    eye[0] += 0.1;
    let camera = camera.at(eye).unwrap();
    actual.set_scene_camera(camera).unwrap();
    reset.set_scene_camera(camera).unwrap();
    // Fault injection stays in this test: bypass only the host's cut invalidation.
    suppressed
        .inner
        .continuous_source_mut()
        .select_camera(camera)
        .unwrap();
    let (actual, a, _) = submitted(&mut actual, device, surface);
    let (reset, r, _) = submitted(&mut reset, device, surface);
    let (suppressed, s, _) = submitted(&mut suppressed, device, surface);
    ppm(&output.join("cut.ppm"), &actual);
    ppm(&output.join("independent-reset.ppm"), &reset);
    ppm(&output.join("suppressed-reset.ppm"), &suppressed);
    let result = serde_json::json!({"cut_equals_reset":actual==reset,"suppressed_differs":suppressed!=reset,"cut_history":a.history_identity,"reset_history":r.history_identity,"suppressed_history":s.history_identity});
    std::fs::write(
        output.join("cut.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    assert_eq!(a.history_identity[1], 0);
    assert_eq!(r.history_identity[1], 0);
    assert_ne!(s.history_identity[1], 0);
    assert_eq!(
        actual, reset,
        "post-cut output must equal independent reset render"
    );
    assert_ne!(suppressed, reset, "control must reject a suppressed reset");
}
