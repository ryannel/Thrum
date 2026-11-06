use super::super::*;
use crate::synthetic::*;
use std::{sync::mpsc, time::Duration};

#[derive(Debug)]
struct Sample {
    values: [f32; 4],
    flags: u32,
    serial: u64,
    tone_map: u32,
}

fn read_sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mut encoder: wgpu::CommandEncoder,
    sample: &wgpu::Buffer,
) -> Sample {
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exposure correctness readback"),
        size: SAMPLE_BYTES,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_buffer_to_buffer(sample, 0, &readback, 0, SAMPLE_BYTES);
    let (send, receive) = mpsc::channel();
    encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
        send.send(result).unwrap();
    });
    let submission = queue.submit([encoder.finish()]);
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive.recv().unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range().unwrap();
    let words: [u32; 8] = std::array::from_fn(|word| {
        u32::from_le_bytes(mapped[word * 4..word * 4 + 4].try_into().unwrap())
    });
    Sample {
        values: std::array::from_fn(|word| f32::from_bits(words[word])),
        flags: words[4],
        serial: u64::from(words[5]) | (u64::from(words[6]) << 32),
        tone_map: words[7],
    }
}

async fn device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("real adapter for exposure correctness");
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .unwrap()
}

fn resolve(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    exposure: &ExposureResolve,
    light: &wgpu::Buffer,
    policy: [u8; 32],
    values: [f32; 16],
) -> Sample {
    queue.write_buffer(light, 0, values.map(f32::to_le_bytes).as_flattened());
    exposure.update(queue, policy);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&exposure.pipeline);
        pass.set_bind_group(0, &exposure.groups[0], &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    read_sample(device, queue, encoder, &exposure.samples[0])
}

fn light(sun_y: f32, direct: [f32; 3], sky: [f32; 3]) -> [f32; 16] {
    let mut values = [0.0; 16];
    values[1] = sun_y;
    values[4..7].copy_from_slice(&direct);
    values[12..15].copy_from_slice(&sky);
    values
}

#[test]
fn gpu_meter_calibration_fixed_bits_bounds_zero_and_invalid_inputs() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let serials = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut serial_bytes = [0; 32];
        serial_bytes[8..12].copy_from_slice(&17_u32.to_le_bytes());
        queue.write_buffer(&serials, 0, &serial_bytes);
        let exposure = ExposureResolve::new(&device, &light_buffer, &serials);
        for gain in [1.0 / 16_000.0, 0.25, 1.234_567, 10.0] {
            let inverse: f32 = 1.0 / gain;
            let sample = resolve(
                &device,
                &queue,
                &exposure,
                &light_buffer,
                ExposurePolicy::Fixed { inverse }.pack(16_000.0, false, SdrToneMap::Reinhard),
                [0.0; 16],
            );
            assert_eq!(sample.values[1].to_bits(), inverse.to_bits());
            assert_eq!(sample.values[0].to_bits(), PRE_EXPOSURE.to_bits());
            assert_eq!(sample.flags, 4);
            assert_eq!(sample.serial, 17);
        }
        let policy = ExposurePolicy::incident(0.0, 0.0, 18.0).unwrap();
        for (values, expected_lux, expected_ev, flags) in [
            (
                light(1.0, [100_000.0; 3], [0.0; 3]),
                100_000.0_f32,
                40_000.0_f32.log2(),
                0,
            ),
            (light(0.5, [5.0; 3], [0.0; 3]), 2.5, 0.0, 0),
            (light(-0.5, [100_000.0; 3], [2.5; 3]), 2.5, 0.0, 0),
            (light(0.0, [0.0; 3], [0.0; 3]), 0.0, 0.0, 3),
            (light(0.0, [0.0; 3], [0.25; 3]), 0.25, 0.0, 3),
            (
                light(1.0, [10_000_000.0; 3], [0.0; 3]),
                10_000_000.0,
                18.0,
                2,
            ),
        ] {
            let sample = resolve(
                &device,
                &queue,
                &exposure,
                &light_buffer,
                policy.pack(16_000.0, true, SdrToneMap::Reinhard),
                values,
            );
            assert!(
                (sample.values[3] - expected_lux).abs() <= expected_lux.max(1.0) * 1e-5,
                "{sample:?}"
            );
            assert!((sample.values[2] - expected_ev).abs() < 1e-4, "{sample:?}");
            assert!((sample.values[1] / (1.2 * expected_ev.exp2()) - 1.0).abs() < 1e-4);
            assert_eq!(sample.flags, flags);
        }
        let sky = light(1.0, [0.0; 3], [10.0, 20.0, 30.0]);
        let base = resolve(
            &device,
            &queue,
            &exposure,
            &light_buffer,
            policy.pack(16_000.0, true, SdrToneMap::Reinhard),
            sky,
        );
        let mut probe_warning = sky;
        probe_warning[15] = 4.0; // Atmosphere marker plus three negative normal probes.
        let warned = resolve(
            &device,
            &queue,
            &exposure,
            &light_buffer,
            policy.pack(16_000.0, true, SdrToneMap::Reinhard),
            probe_warning,
        );
        assert_eq!(
            warned.values.map(f32::to_bits),
            base.values.map(f32::to_bits)
        );
        assert_eq!(warned.flags, base.flags | 16, "probe risk is informational");
        let brighter = resolve(
            &device,
            &queue,
            &exposure,
            &light_buffer,
            ExposurePolicy::incident(1.0, 0.0, 18.0).unwrap().pack(
                16_000.0,
                true,
                SdrToneMap::Reinhard,
            ),
            sky,
        );
        assert!(
            (base.values[3] - 18.596).abs() < 1e-4,
            "Rec.709 physical lux"
        );
        assert!((base.values[1] / brighter.values[1] - 2.0).abs() < 1e-4);
        for values in [
            light(f32::NAN, [1.0; 3], [1.0; 3]),
            light(1.0, [-1.0; 3], [1.0; 3]),
            light(1.0, [1.0; 3], [f32::INFINITY; 3]),
        ] {
            let sample = resolve(
                &device,
                &queue,
                &exposure,
                &light_buffer,
                policy.pack(16_000.0, true, SdrToneMap::Reinhard),
                values,
            );
            assert_eq!(sample.flags, 12);
            assert_eq!(sample.values[1].to_bits(), 16_000.0_f32.to_bits());
            assert!(sample.values.iter().all(|value| value.is_finite()));
        }
        let unphysical = resolve(
            &device,
            &queue,
            &exposure,
            &light_buffer,
            policy.pack(16_000.0, false, SdrToneMap::Reinhard),
            sky,
        );
        assert_eq!(
            unphysical.flags, 12,
            "diagnostic/photometric data must not be metered as atmosphere"
        );
        assert!(scope.pop().await.is_none());
    });
}

fn encode_sample(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: &PreparedSynthetic,
    target: &OffscreenTarget,
) -> Sample {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    prepared.encode(&mut encoder, 41, target, None).unwrap();
    read_sample(
        device,
        queue,
        encoder,
        &prepared.exposure.samples[prepared.history.write_side()],
    )
}

#[test]
fn gpu_meter_follows_cached_light_policy_submission_cancellation_and_capture() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.view = scene.view.with_incident_exposure(0.0, 0.0, 18.0).unwrap();
        let frame = AtmosphereFrame::new([0.0, 1.0, 0.0], 127_000.0, [1.0; 3], 2.0, 1).unwrap();
        let mut prepared = PreparedSynthetic::prepare_with_lighting(
            &DeviceCapabilities::capture_device(&device, 41),
            &queue,
            &scene,
            FrameLighting::Atmosphere(frame),
            1.0 / 16_000.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, 16, 16).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .unwrap();
        let initial = encode_sample(&device, &queue, &prepared, &target);
        prepared.commit_frame();
        let retained_side = 1 - prepared.history.write_side();
        let retained_serial = initial.serial;
        assert_eq!(initial.tone_map, SdrToneMap::Reinhard as u32);
        // Prepared revision 2 is cancelled; revision 3 repeats revision 1's view
        // parity but must write opposite the actual retained history sample.
        scene.view = scene
            .view
            .with_incident_exposure(1.0, 0.0, 18.0)
            .unwrap()
            .with_sdr_tone_map(SdrToneMap::NeutralHighlights);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .unwrap();
        let mut cancelled =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared.encode(&mut cancelled, 41, &target, None).unwrap();
        drop(cancelled);
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(frame),
                1.0,
            )
            .unwrap();
        assert_eq!(target.frame_revision & 1, 1);
        assert_ne!(prepared.history.write_side(), retained_side);
        let second = encode_sample(&device, &queue, &prepared, &target);
        let retained = read_sample(
            &device,
            &queue,
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
            &prepared.exposure.samples[retained_side],
        );
        assert_eq!(retained.serial, retained_serial);
        assert_eq!(
            retained.values.map(f32::to_bits),
            initial.values.map(f32::to_bits)
        );
        assert_eq!(second.serial, initial.serial + 1);
        assert_eq!(second.tone_map, SdrToneMap::NeutralHighlights as u32);
        assert_eq!(retained.tone_map, initial.tone_map);
        assert!((second.values[3] / initial.values[3] - 1.0).abs() < 1e-5);
        assert!((initial.values[1] / second.values[1] - 2.0).abs() < 1e-4);
        // Capture submits an image but intentionally doesn't commit view history.
        assert_eq!(prepared.history.write_side(), 1 - retained_side);
        let mut pixels = target.readback(&device);
        let mut capture_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut capture_encoder, 41, &target, None, &mut pixels)
            .unwrap();
        let captured = read_sample(
            &device,
            &queue,
            capture_encoder,
            &prepared.exposure.samples[prepared.history.write_side()],
        );
        assert_eq!(pixels.try_collect(41).unwrap().unwrap().len(), 16 * 16 * 4);
        assert_eq!(captured.serial, second.serial);
        assert_eq!(captured.tone_map, second.tone_map);
        prepared.commit_frame();
        assert_eq!(prepared.history.write_side(), retained_side);
        let stronger = AtmosphereFrame::new([0.0, 1.0, 0.0], 254_000.0, [1.0; 3], 2.0, 1).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(stronger),
                1.0,
            )
            .unwrap();
        let intensity = encode_sample(&device, &queue, &prepared, &target);
        assert!(
            (intensity.values[3] / second.values[3] - 2.0).abs() < 1e-4,
            "cached sky must incorporate new source intensity"
        );
        prepared.commit_frame();
        prepared.synchronize_history(&queue, 1, 1);
        scene.view = scene.view.with_history_reset(true);
        target = OffscreenTarget::new(&device, &prepared, 32, 16).unwrap();
        prepared
            .update_frame(
                &queue,
                &mut target,
                &scene,
                FrameLighting::Atmosphere(stronger),
                1.0,
            )
            .unwrap();
        let reset = encode_sample(&device, &queue, &prepared, &target);
        assert_eq!(
            reset.serial,
            intensity.serial + 1,
            "reset never manufactures a second exposure cursor"
        );
        assert_eq!(reset.flags, 0);
        assert_eq!(reset.tone_map, SdrToneMap::NeutralHighlights as u32);
        assert!(scope.pop().await.is_none());
    });
}
