//! Exercise stack isolation on the actual read-only resident texture.

#[cfg(target_os = "macos")]
mod scene_fixture;

thrum_test_support::macos_gpu_test! {
    fn cooked_slabs_stay_isolated_at_every_readable_mip() {
        run();
    }
}

#[cfg(target_os = "macos")]
fn run() {
    use std::{fs, time::Duration};
    use thrum_asset::Profile;
    use thrum_asset_gpu::{CookedAssets, CookedConfig, LeaseRequest};
    use thrum_cook::{CookConfig, Cooker, Durability, Quality, encode_volume};
    use thrum_render::runtime::FrameConfig;
    use thrum_test_support::{Restriction, Scratch};

    let Some(thrum_test_support::Opened {
        device,
        queue,
        capabilities,
        ..
    }) = thrum_test_support::open(
        "placed volume slab isolation",
        Some(wgpu::Backend::Metal),
        thrum_test_support::Profile::Portable,
        &Restriction::default(),
    )
    else {
        return;
    };

    let root = Scratch::new("placed-volume-stack");
    let source = root.join("source");
    let store = root.join("store");
    fs::create_dir(&source).expect("source folder");
    let bytes = [29u8, 223, 67, 194]
        .map(|value| [value; 8 * 8 * 8])
        .concat();
    fs::write(source.join("stack.tvol"), encode_volume(8, 8, 32, &bytes)).expect("volume source");
    let mut cooker = Cooker::open(CookConfig {
        source,
        store: store.clone(),
        profiles: vec![Profile::Portable],
        quality: Quality::Fast,
        durability: Durability::Process,
    })
    .expect("cooker");
    let report = cooker.cook_all().expect("cook");
    assert!(report.failed.is_empty(), "{:?}", report.failed);

    let mut config = CookedConfig::new(store, Vec::new());
    config.leased.push(LeaseRequest {
        name: "stack".into(),
        dimension: wgpu::TextureDimension::D3,
    });
    let assets = CookedAssets::new(config).expect("lease configuration");
    let lease = assets.lease();
    let view = thrum_render::synthetic::SyntheticView::perspective(
        [0.0, 40.0, 0.0],
        [0.0, 100.0, -300.0],
        [0.0, 1.0, 0.0],
        1.0,
        1.0,
        0.1,
        200_000.0,
    )
    .expect("resident-stack view");
    let mut host = pollster::block_on(scene_fixture::prepare_view(
        &capabilities,
        &queue,
        None,
        FrameConfig {
            render_extent: [64, 64],
            surface: None,
        },
        [0.0, 0.866_025_4, 0.5],
        0,
        &view,
    ));
    host.set_scene_contribution(Box::new(assets))
        .expect("attach residency");
    let resident =
        thrum_test_support::wait::until("resident lease", Duration::from_secs(5), || {
            host.capture_pixels().expect("submit residency");
            lease.offered()
        });
    let product = &resident.products[0];
    assert_eq!(product.profile, Profile::Portable);
    assert_eq!(product.texture.format(), wgpu::TextureFormat::R8Unorm);
    assert!(product.texture.mip_level_count() > 4);
    compare_samples(&device, &queue, &product.view);
}

#[cfg(target_os = "macos")]
fn compare_samples(device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) {
    use thrum_render::runtime::DeviceSite;
    let site = DeviceSite::untracked(device.clone());
    let source = concat!(
        include_str!("../src/placed_volume.wgsl"),
        r"
@group(0) @binding(0) var stack: texture_3d<f32>;
@group(0) @binding(1) var volume_sampler: sampler;
@group(0) @binding(2) var<storage, read_write> samples: array<vec4<f32>>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let level = f32(id.y) * 0.5;
    let edge = f32(id.z) * 0.5;
    let weight = placed_form_weights(edge);
    let near_edge = placed_slab(stack, volume_sampler, id.x, vec3(0.5, 0.5, edge), level);
    samples[(id.x * 10u + id.y) * 3u + id.z] = vec4(near_edge, weight, weight.x + weight.y);
}
"
    );
    let shader = site.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sample isolated cloud slabs"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = site.compute(&wgpu::ComputePipelineDescriptor {
        label: Some("sample isolated cloud slabs"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let sampler = site.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let output = site.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cloud slab sample results"),
        size: 4 * 10 * 3 * 16,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let layout = pipeline.get_bind_group_layout(0);
    let group = site.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("cloud slab samples"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
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
        pass.dispatch_workgroups(4, 10, 3);
    }
    queue.submit([encoder.finish()]);
    let bytes = thrum_test_support::readback::buffer(device, queue, &output, ..);
    let samples: &[[f32; 4]] = bytemuck::cast_slice(&bytes);
    for (values, density) in samples.as_chunks::<30>().0.iter().zip([29u8, 223, 67, 194]) {
        for (sample, volume_weight) in values.iter().zip([0.0, 0.5, 1.0].into_iter().cycle()) {
            assert!(
                (sample[0] - f32::from(density) / 255.0).abs() < 0.000_01,
                "{sample:?}"
            );
            assert_eq!(&sample[1..], &[1.0 - volume_weight, volume_weight, 1.0]);
        }
    }
}
