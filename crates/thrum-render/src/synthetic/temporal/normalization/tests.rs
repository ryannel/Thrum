use super::*;
use std::io::Write;

#[test]
fn metalfx_normalization_half_limit() {
    use thrum_test_support::{Profile, readback};
    let Some(opened) = thrum_test_support::open(
        "MetalFX normalization half limit",
        Some(wgpu::Backend::Metal),
        Profile::Native,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let device = &opened.device;
    let queue = &opened.queue;
    let normalize = Normalize::new(&DeviceSite::untracked(device.clone()));
    let tex_size = wgpu::Extent3d {
        width: 4,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = |usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("half-float bound fixture"),
            size: tex_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage,
            view_formats: &[],
        })
    };
    let input = texture(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    let output = texture(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC);
    let input_pixels = [32752.0, 32768.0, -0.5, f32::NAN]
        .map(|value| [value, 0.0, 0.0, 1.0].map(|channel| half::f16::from_f32(channel).to_bits()));
    queue.write_texture(
        input.as_image_copy(),
        bytemuck::bytes_of(&input_pixels),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(32),
            rows_per_image: Some(1),
        },
        tex_size,
    );
    let buffer = |label, size, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let sample = buffer(
        "source exposure",
        SAMPLE_BYTES,
        wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    );
    let output_exposure = buffer(
        "tone exposure",
        SAMPLE_BYTES,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let repairs = buffer(
        "half-float repairs",
        REPAIR_BYTES,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let policy = crate::records::ExposureSample {
        values: [0.25, 0.500_2, 17.0, 40_000.0],
        metadata: [8, 21, 0, 1],
    };
    queue.write_buffer(&sample, 0, bytemuck::bytes_of(&policy));
    let view =
        |texture: &wgpu::Texture| texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bindings = [
        sample.as_entire_binding(),
        output_exposure.as_entire_binding(),
        wgpu::BindingResource::TextureView(&view(&input)),
        wgpu::BindingResource::TextureView(&view(&output)),
        repairs.as_entire_binding(),
    ];
    let entries: Vec<_> = bindings
        .into_iter()
        .enumerate()
        .map(|(index, resource)| wgpu::BindGroupEntry {
            binding: u32::try_from(index).expect("five bindings"),
            resource,
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(PASS),
        layout: &normalize.native.get_bind_group_layout(0),
        entries: &entries,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&normalize.native);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    queue.submit([encoder.finish()]);
    let raw = readback::texture(device, queue, &output, readback::Mip::level(0), ..);
    let red: Vec<_> = raw
        .as_chunks::<8>()
        .0
        .iter()
        .map(|pixel| half::f16::from_bits(u16::from_le_bytes([pixel[0], pixel[1]])).to_f32())
        .collect();
    let expected = 32752.0 * (0.25 / (policy.values[0] * policy.values[1]));
    assert!(
        expected < 65504.0 && 32768.0 * (0.25 / (policy.values[0] * policy.values[1])) > 65504.0
    );
    assert!(
        (red[0] - expected).abs() <= 16.0,
        "half rounding {red:?}, expected {expected}"
    );
    assert_eq!(&red[1..], &[65504.0, 0.0, 0.0]);
    let counts = readback::buffer(device, queue, &repairs, ..);
    assert_eq!(bytemuck::cast_slice::<_, u32>(&counts), [1, 1, 1]);
    let bytes = readback::buffer(device, queue, &output_exposure, ..);
    let normalized = bytemuck::pod_read_unaligned::<crate::records::ExposureSample>(&bytes);
    assert_eq!(
        normalized.values.map(f32::to_bits),
        [0.25_f32, 1.0, 17.0, 40_000.0].map(f32::to_bits)
    );
    assert_eq!(normalized.metadata, policy.metadata);
    writeln!(
        std::io::stdout().lock(),
        "half-float outputs {red:?}; high, low, non-finite repairs [1, 1, 1]"
    )
    .expect("write boundary proof");
}
