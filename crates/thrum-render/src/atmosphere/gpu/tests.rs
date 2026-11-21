mod aerial;
mod aerial_ground;
pub(in crate::atmosphere) mod diffuse;
mod ground;
mod mapping;
mod multiple_interpolation;
mod sky_probe;
mod transmittance;

/// The portable test device on `backend`, with the empty feature mask, and the adapter it opened on.
fn open(
    test: &str,
    backend: Option<wgpu::Backend>,
) -> Option<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)> {
    let crate::test_device::Opened {
        gpu, device, queue, ..
    } = crate::test_device::open(
        test,
        backend,
        crate::capabilities::DeviceProfile::Portable,
        &crate::capabilities::DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        },
    )?;
    let info = gpu.adapter.get_info();
    #[cfg(target_os = "macos")]
    assert_eq!(info.backend, wgpu::Backend::Metal);
    Some((device, queue, info))
}

fn sky_samples(device: &wgpu::Device) -> [wgpu::Buffer; 2] {
    let sample = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("transport-only fixture exposure"),
        size: 32,
        usage: wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });
    [sample.clone(), sample]
}

#[test]
fn encoded_constant_sky_integrates_in_physical_illuminance_units() {
    let Some((device, queue, _)) = open(
        "encoded_constant_sky_integrates_in_physical_illuminance_units",
        None,
    ) else {
        return;
    };
    pollster::block_on(async {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("constant sky frame"),
            size: super::UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let solar_illuminance = 127_000.0;
        let mut frame = [0.0_f32; 32];
        frame[20] = 1.0; // Unit receiver-to-Sun direction, +X.
        frame[23] = solar_illuminance;
        frame[24..28].copy_from_slice(&[1.0, 1.0, 1.0, 1.0 / 1024.0]);
        frame[28..32].copy_from_slice(&super::super::model::sky_mapping(0.0));
        queue.write_buffer(&uniform, 0, frame.map(f32::to_le_bytes).as_flattened());

        let sky = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("constant encoded sky radiance"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // Known per-unit radiances [2^-16, 2^-15, 2^-14], stored with the
        // fixed 1024 gain as exactly representable halfs [2^-6, 2^-5, 2^-4].
        let encoded = [0x2400_u16, 0x2800, 0x2c00, 0].map(u16::to_le_bytes);
        queue.write_texture(
            sky.as_image_copy(),
            encoded.as_flattened(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(1),
            },
            sky.size(),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let directional = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("constant sky illuminance"),
            size: crate::light::ENVIRONMENT_BYTES,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let sky_view = sky.create_view(&wgpu::TextureViewDescriptor::default());
        // The diffuse stage samples only the sky view.
        let views = crate::atmosphere::stage::Charts::from_fn(|_| sky_view.clone());
        let (pipeline, group) = super::prepare_diffuse(
            &crate::runtime::DeviceSite::untracked(wgpu::Device::clone(&device)),
            &uniform,
            &views,
            &sampler,
            &directional,
        );
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let readback = thrum_test_support::readback::Readback::buffer(
            &device,
            &mut encoder,
            &directional,
            ..crate::light::ENVIRONMENT_BYTES,
        );
        let submission = queue.submit([encoder.finish()]);
        let bytes = readback.wait(&device, submission);
        assert!(scope.pop().await.is_none(), "diffuse pipeline validates");
        for (channel, radiance) in [1.0 / 65_536.0, 1.0 / 32_768.0, 1.0 / 16_384.0]
            .into_iter()
            .enumerate()
        {
            let start = channel * 4; // Horizontal sky irradiance.
            let actual =
                f32::from_le_bytes(bytes[start..start + 4].try_into().expect("f32 channel"));
            let expected = std::f32::consts::PI * radiance * solar_illuminance;
            assert!(
                actual.is_finite() && (actual - expected).abs() <= expected * 1e-5,
                "channel {channel}: {actual} lux, expected pi * L * Sun = {expected} lux"
            );
        }
    });
}
