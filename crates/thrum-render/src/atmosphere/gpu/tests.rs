mod ground;
mod mapping;
mod reference;

#[test]
fn encoded_constant_sky_integrates_in_physical_illuminance_units() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("real adapter required for diffuse units regression");
        #[cfg(target_os = "macos")]
        assert_eq!(adapter.get_info().backend, wgpu::Backend::Metal);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("diffuse units device");
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
        let encoded = [0x2400_u16, 0x2800, 0x2c00, 0x3c00].map(u16::to_le_bytes);
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
            size: 64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("constant sky illuminance readback"),
            size: 64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let (pipeline, group) = super::prepare_diffuse(
            &device,
            &uniform,
            &sky.create_view(&wgpu::TextureViewDescriptor::default()),
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
        encoder.copy_buffer_to_buffer(&directional, 0, &readback, 0, 64);
        let (send, receive) = std::sync::mpsc::channel();
        encoder.map_buffer_on_submit(&readback, wgpu::MapMode::Read, .., move |result| {
            send.send(result).expect("diffuse readback receiver alive");
        });
        let submission = queue.submit([encoder.finish()]);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(std::time::Duration::from_secs(20)),
            })
            .expect("diffuse units GPU completion");
        assert!(scope.pop().await.is_none(), "diffuse pipeline validates");
        receive
            .recv()
            .expect("diffuse mapping callback")
            .expect("diffuse readback mapped");
        let bytes = readback
            .slice(..)
            .get_mapped_range()
            .expect("mapped result");
        for (channel, radiance) in [1.0 / 65_536.0, 1.0 / 32_768.0, 1.0 / 16_384.0]
            .into_iter()
            .enumerate()
        {
            let start = 48 + channel * 4; // directional[3].rgb, horizontal sky E.
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
