//! Probe the real forward and display attachments at one dim neutral pixel.

use std::io::Write as _;

use super::{Sample, device, read_sample};
use crate::atmosphere::AtmosphereFrame;
use crate::capabilities::DeviceCapabilities;
use crate::synthetic::{
    FrameLighting, OffscreenTarget, PreparedSynthetic, SdrToneMap, SyntheticScene, SyntheticView,
};

const GENERATION: u64 = 41;
const SIDE: u32 = 16;
// Use the scalar radiance of a 1% Lambertian reflector under 0.000671 lux
// as an emission card, so the storage oracle has no BRDF or SH fit error.
const CARD_RADIANCE: f32 = 0.01 * 0.000_671 / std::f32::consts::PI;
const PROBE_SHADER: &str = r"
@group(0) @binding(0) var hdr: texture_2d<f32>;
@group(0) @binding(1) var<storage,read_write> sampled: vec4<f32>;
@compute @workgroup_size(1)
fn probe() {
    sampled = textureLoad(hdr,vec2<i32>(textureDimensions(hdr)/2u),0);
}
";

struct Probe {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    result: wgpu::Buffer,
    readback: wgpu::Buffer,
}

impl Probe {
    fn new(device: &wgpu::Device, target: &OffscreenTarget) -> Self {
        assert_eq!(
            target.textures.texture(&target.ids.scene).unwrap().format(),
            wgpu::TextureFormat::Rgba16Float
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDR attachment probe"),
            source: wgpu::ShaderSource::Wgsl(PROBE_SHADER.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("HDR attachment probe"),
            layout: None,
            module: &module,
            entry_point: Some("probe"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let result = buffer(
            device,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = buffer(
            device,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HDR attachment probe"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&target.scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: result.as_entire_binding(),
                },
            ],
        });
        Self {
            pipeline,
            group,
            result,
            readback,
        }
    }

    fn capture(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        prepared: &PreparedSynthetic,
        target: &OffscreenTarget,
    ) -> (Sample, [f32; 4], [u8; 4]) {
        let mut pixels = target.readback(device);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prepared
            .encode_with_readback(&mut encoder, GENERATION, target, None, &mut pixels)
            .unwrap();
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&self.result, 0, &self.readback, 0, 16);
        let (send, receive) = std::sync::mpsc::channel();
        encoder.map_buffer_on_submit(&self.readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).unwrap();
        });
        let side = prepared.history.write_side();
        let sample = read_sample(device, queue, encoder, &prepared.exposure.samples[side]);
        receive.recv().unwrap().unwrap();
        let hdr = {
            let mapped = self.readback.slice(..).get_mapped_range().unwrap();
            std::array::from_fn(|i| f32::from_le_bytes(mapped.as_chunks::<4>().0[i]))
        };
        self.readback.unmap();
        let pixels = pixels.try_collect(GENERATION).unwrap().unwrap();
        let pixel = usize::try_from((SIDE / 2) * (SIDE + 1)).unwrap();
        (sample, hdr, pixels.as_chunks::<4>().0[pixel])
    }
}

fn buffer(device: &wgpu::Device, usage: wgpu::BufferUsages) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("HDR attachment probe data"),
        size: 16,
        usage,
        mapped_at_creation: false,
    })
}

fn assert_dim_card(sample: &Sample, hdr: [f32; 4], display: [u8; 4]) {
    assert_eq!(sample.values[0].to_bits(), 64.0_f32.to_bits(), "{sample:?}");
    assert_eq!(sample.flags, 3);
    assert_eq!(sample.values[2].to_bits(), (-12.0_f32).to_bits());
    assert!((sample.values[1] / (1.2 / 4096.0) - 1.0).abs() < 1e-5);
    for stored in &hdr[..3] {
        // Up to one fp16 step plus short-path RGB transmittance error.
        assert!(
            (stored / (CARD_RADIANCE * 64.0) - 1.0).abs() < 0.003,
            "{hdr:?}"
        );
    }
    let exposed = CARD_RADIANCE / sample.values[1];
    let linear = exposed / (1.0 + exposed);
    let srgb = 1.055 * linear.powf(1.0 / 2.4) - 0.055;
    assert!(srgb * 255.0 > 10.0);
    assert!(
        display[..3]
            .iter()
            .all(|&code| (f32::from(code) - srgb * 255.0).abs() <= 1.5),
        "{display:?} vs {}",
        srgb * 255.0
    );
    assert_eq!(display[3], 255);
    writeln!(
        std::io::stdout().lock(),
        "dim control: serial={} scale={} HDR={} SDR={}",
        sample.serial,
        sample.values[0],
        hdr[0],
        display[0]
    )
    .unwrap();
}

#[test]
fn dim_lambert_equivalent_survives_hdr_display_and_exposure_side_reuse() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let capabilities = DeviceCapabilities::capture_device(&device, GENERATION);
        let light = FrameLighting::Atmosphere(
            AtmosphereFrame::new([0.0, -1.0, 0.0], 0.0, [1.0; 3], 2.0, 1).unwrap(),
        );
        let mut scene = SyntheticScene::grid(1).unwrap();
        scene.materials[0].base_rgba = [0.0, 0.0, 0.0, 1.0];
        scene.materials[0].surface[2] = 1.0;
        scene.materials[0].emissive_cutoff = [CARD_RADIANCE, CARD_RADIANCE, CARD_RADIANCE, 0.0];
        let fixed = scene.view.with_exposure(1.0).unwrap();
        let metered = scene
            .view
            .with_incident_exposure(0.0, -12.0, 18.0)
            .unwrap()
            .with_low_light_adaptation()
            .unwrap();
        scene.view = fixed;
        let mut prepared =
            PreparedSynthetic::prepare_with_lighting(&capabilities, &queue, &scene, light, 1.0)
                .await
                .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, SIDE, SIDE).unwrap();
        let probe = Probe::new(&device, &target);
        for (view, is_night) in [
            (fixed, false),
            (metered, true),
            (metered, true),
            (fixed, false),
        ] {
            scene.view = view;
            prepared
                .update_frame(&queue, &mut target, &scene, light, 1.0)
                .unwrap();
            let side = prepared.history.write_side();
            let mut abandoned =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prepared
                .encode(&mut abandoned, GENERATION, &target, None)
                .unwrap();
            drop(abandoned);
            let (sample, hdr, display) = probe.capture(&device, &queue, &prepared, &target);
            assert_eq!(
                prepared.history.write_side(),
                side,
                "capture does not commit history"
            );
            let (submitted, stored, displayed) = probe.capture(&device, &queue, &prepared, &target);
            prepared.commit_frame();
            assert_ne!(prepared.history.write_side(), side);
            assert_eq!(submitted.serial, sample.serial);
            assert_eq!(
                submitted.values.map(f32::to_bits),
                sample.values.map(f32::to_bits)
            );
            assert_eq!(stored.map(f32::to_bits), hdr.map(f32::to_bits));
            assert_eq!(displayed, display);
            if is_night {
                assert_dim_card(&sample, hdr, display);
            } else {
                assert_eq!(sample.flags, 4);
                assert_eq!(sample.values[0].to_bits(), super::PRE_EXPOSURE.to_bits());
                assert_eq!(hdr.map(f32::to_bits), [0, 0, 0, 1.0_f32.to_bits()]);
                assert_eq!(display, [0, 0, 0, 255]);
            }
        }
        assert!(scope.pop().await.is_none());
    });
}

#[test]
fn falling_twilight_radiance_does_not_brighten_the_metered_sky() {
    pollster::block_on(async {
        let (device, queue) = device().await;
        let capabilities = DeviceCapabilities::capture_device(&device, GENERATION);
        let mut scene = SyntheticScene::grid(1).unwrap();
        // Approximate the fixed valley upper-sky witness at sea-level-relative
        // altitude 245 m, looking 21.5 degrees above the horizon.
        let altitude = 245.0;
        let ray_angle = 21.5_f32.to_radians();
        let (sin, cos) = ray_angle.sin_cos();
        scene.view = SyntheticView::perspective(
            [0.0, altitude, 16.0],
            [0.0, altitude + 100.0 * sin, 16.0 - 100.0 * cos],
            [0.0, 1.0, 0.0],
            0.001,
            1.0,
            0.1,
            30_000.0,
        )
        .unwrap()
        .with_sdr_tone_map(SdrToneMap::NeutralHighlights)
        .with_incident_exposure(0.0, -12.0, 18.0)
        .unwrap()
        .with_low_light_adaptation()
        .unwrap();
        let lighting = |hours: f32| {
            let (sin, cos) = ((hours / 24.0 - 0.5) * std::f32::consts::TAU).sin_cos();
            let vertical = cos * std::f32::consts::FRAC_1_SQRT_2;
            FrameLighting::Atmosphere(
                AtmosphereFrame::new(
                    [-sin, vertical, -vertical],
                    127_000.0,
                    [1.0; 3],
                    altitude,
                    1,
                )
                .unwrap()
                .with_nightglow(),
            )
        };
        let mut prepared = PreparedSynthetic::prepare_with_lighting(
            &capabilities,
            &queue,
            &scene,
            lighting(18.85),
            1.0,
        )
        .await
        .unwrap();
        let mut target = OffscreenTarget::new(&device, &prepared, SIDE, SIDE).unwrap();
        let probe = Probe::new(&device, &target);
        let [twilight, night] = [18.85, 19.65].map(|hours| {
            prepared
                .update_frame(&queue, &mut target, &scene, lighting(hours), 1.0)
                .unwrap();
            let (sample, hdr, _) = probe.capture(&device, &queue, &prepared, &target);
            prepared.commit_frame();
            let stored = hdr[..3]
                .iter()
                .zip([0.2126, 0.7152, 0.0722])
                .map(|(channel, weight)| channel * weight)
                .sum::<f32>();
            let physical = stored / sample.values[0];
            (physical, physical / sample.values[1])
        });
        assert!(night.0 > 0.0 && night.0 < twilight.0 * 0.01);
        assert!(
            night.1 < twilight.1 * 0.90,
            "displayed linear luma rebounded: {twilight:?} to {night:?}"
        );
    });
}
