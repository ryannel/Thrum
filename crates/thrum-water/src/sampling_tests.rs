//! Affine slopes have a phase-independent cubic reconstruction variance.
//! Compare the actual four-fetch shader with an independent 16-node convolution.
use wgpu::util::DeviceExt;

fn kernel(x: f64) -> f64 {
    let x = x.abs();
    if x < 1.0 {
        (4.0 - 6.0 * x * x + 3.0 * x.powi(3)) / 6.0
    } else if x < 2.0 {
        (2.0 - x).powi(3) / 6.0
    } else {
        0.0
    }
}

fn reference(x: f64, y: f64) -> [f64; 5] {
    let mut result = [0.0; 5];
    for row in 0..16 {
        for column in 0..16 {
            let weight = kernel(x - f64::from(column)) * kernel(y - f64::from(row));
            let a = f64::from(column) / 8.0;
            let b = f64::from(row) / 16.0;
            for (sum, value) in result.iter_mut().zip([a, b, a * a, a * b, b * b]) {
                *sum += weight * value;
            }
        }
    }
    result
}

// All fixture values are nonnegative binary rationals, normal in binary16.
fn fixture_half(value: f32) -> u16 {
    if value == 0.0 {
        return 0;
    }
    let bits = value.to_bits();
    u16::try_from((((bits >> 23) - 112) << 10) | ((bits >> 13) & 1023)).expect("half fixture")
}

fn affine_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("affine LEAN fixture"),
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 2,
        },
        mip_level_count: 5,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for mip in 0..5 {
        let step = 1_u32 << mip;
        let width = 16 / step;
        let mut data = Vec::new();
        for layer in 0..2 {
            for y in 0..width {
                for x in 0..width {
                    let mut mean = [0.0_f32; 4];
                    for dy in 0..step {
                        for dx in 0..step {
                            let a = f32::from(u16::try_from(x * step + dx).expect("x")) / 8.0;
                            let b = f32::from(u16::try_from(y * step + dy).expect("y")) / 16.0;
                            let value = if layer == 0 {
                                [a, b, 0.0, 0.0]
                            } else {
                                [a * a, a * b, b * b, 0.0]
                            };
                            for (sum, value) in mean.iter_mut().zip(value) {
                                *sum += value;
                            }
                        }
                    }
                    let count = f32::from(u16::try_from(step * step).expect("count"));
                    data.extend(mean.map(|v| fixture_half(v / count)));
                }
            }
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(width),
            },
            wgpu::Extent3d {
                width,
                height: width,
                depth_or_array_layers: 2,
            },
        );
    }
    texture
}

#[test]
fn reconstructed_moments_have_no_magnified_texel_lattice() {
    let Some(thrum_test_support::Opened { device, queue, .. }) = thrum_test_support::open(
        "water moment reconstruction",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &thrum_test_support::Restriction::default(),
    ) else {
        return;
    };
    let texture = affine_texture(&device, &queue);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        anisotropy_clamp: 16,
        ..Default::default()
    });
    let output = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("reconstructed moments"),
        contents: &[0; 98 * 64],
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let source = format!(
        "{}\n{}",
        concat!(
            include_str!("sampling_weights.wgsl"),
            include_str!("sampling.wgsl")
        ),
        r"
@group(0) @binding(0) var field:texture_2d_array<f32>;
@group(0) @binding(1) var field_sampler:sampler;
@group(0) @binding(2) var<storage,read_write> result:array<vec4<f32>>;
@compute @workgroup_size(1) fn probe(@builtin(global_invocation_id) id:vec3<u32>) {
    let phase=id.x%49u;let x=4.0+f32(phase%7u)/6.0;let y=7.0+f32(phase/7u)/6.0;
    let uv=(vec2(x,y)+0.5)/16.0;
    let footprint=select(0.00001,0.25,id.x>=49u);
    let dx=vec2(footprint,0.0);let dy=vec2(0.0,footprint);
    result[id.x*4u]=sample_field(field,field_sampler,uv,0,dx,dy);
    result[id.x*4u+1u]=sample_field(field,field_sampler,uv,1,dx,dy);
    result[id.x*4u+2u]=textureSampleGrad(field,field_sampler,uv,0,dx,dy);
    result[id.x*4u+3u]=textureSampleGrad(field,field_sampler,uv,1,dx,dy);
}
"
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("moment probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let site = thrum_render::runtime::DeviceSite::untracked(device.clone());
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("moment probe"),
        layout: None,
        module: &shader,
        entry_point: Some("probe"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("moment probe"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(98, 1, 1);
    }
    let readback =
        thrum_test_support::readback::Readback::buffer(&device, &mut encoder, &output, ..);
    let submission = queue.submit([encoder.finish()]);
    let mapped = readback.wait(&device, submission);
    let values: &[[f32; 4]] = bytemuck::cast_slice(&mapped);
    let mut old_variance = Vec::new();
    let mut max_moment_error = 0.0_f64;
    let mut max_variance_error = 0.0_f32;
    for phase in 0..49 {
        let x = 4.0 + f64::from(u32::try_from(phase % 7).expect("x")) / 6.0;
        let y = 7.0 + f64::from(u32::try_from(phase / 7).expect("y")) / 6.0;
        let mean = values[phase * 4];
        let second = values[phase * 4 + 1];
        let expected = reference(x, y);
        for (actual, expected) in [mean[0], mean[1], second[0], second[1], second[2]]
            .into_iter()
            .zip(expected)
        {
            let error = (f64::from(actual) - expected).abs();
            max_moment_error = max_moment_error.max(error);
            // Hardware bilinear weights have finite subtexel precision. The
            // fixture's largest neighbouring moment delta is below 0.25, so
            // a half step at 8-bit precision contributes below 0.0005.
            assert!(error < 0.0006, "phase {phase}: {actual} vs {expected}");
        }
        let variance = second[0] - mean[0] * mean[0];
        let error = (variance - 1.0 / (64.0 * 3.0)).abs();
        max_variance_error = max_variance_error.max(error);
        assert!(error < 0.0006, "variance {variance}");
        let old = values[phase * 4 + 2];
        let old_second = values[phase * 4 + 3];
        old_variance.push(old_second[0] - old[0] * old[0]);
        let minified = (phase + 49) * 4;
        assert_eq!(
            values[minified],
            values[minified + 2],
            "minified mean unchanged"
        );
        assert_eq!(
            values[minified + 1],
            values[minified + 3],
            "minified moments unchanged"
        );
    }
    let min = old_variance.iter().copied().fold(f32::INFINITY, f32::min);
    let max = old_variance
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        max - min > 0.003,
        "bilinear negative control must expose the lattice"
    );
    println!(
        "maximum moment error {max_moment_error}; variance error {max_variance_error}; bilinear variance range {}",
        max - min
    );
}
