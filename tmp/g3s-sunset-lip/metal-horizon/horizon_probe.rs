use super::{buffer_with, depth_target};
use std::{fs, path::Path, time::Duration};

fn module(device: &wgpu::Device, source: String) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("horizon probe"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    entry: &str,
    entries: &[(u32, wgpu::BindingType)],
) -> (wgpu::ComputePipeline, wgpu::BindGroupLayout) {
    let entries: Vec<_> = entries.iter().map(|&(binding, ty)| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty,
        count: None,
    }).collect();
    let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(entry), entries: &entries,
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(entry), bind_group_layouts: &[Some(&bindings)], immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry), layout: Some(&layout), module: shader, entry_point: Some(entry),
        compilation_options: Default::default(), cache: None,
    });
    (pipeline, bindings)
}

fn group<'a>(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    entries: impl IntoIterator<Item = (u32, wgpu::BindingResource<'a>)>,
) -> wgpu::BindGroup {
    let entries: Vec<_> = entries.into_iter().map(|(binding,resource)| {
        wgpu::BindGroupEntry { binding, resource }
    }).collect();
    device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout, entries: &entries })
}

fn read(device: &wgpu::Device, buffer: &wgpu::Buffer, path: &Path) {
    buffer.slice(..).map_async(wgpu::MapMode::Read, |result| result.expect("readback"));
    device.poll(wgpu::PollType::Wait {
        submission_index: None, timeout: Some(Duration::from_secs(15)),
    }).expect("GPU completion");
    let mapped = buffer.slice(..).get_mapped_range().expect("mapped");
    fs::write(path, &*mapped).expect("save result");
}

fn staging(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

#[test]
fn metal_flat_horizon() {
    let dir = std::env::var("THRUM_METAL_HORIZON").expect("probe directory");
    let dir = Path::new(&dir);
    let output = std::env::var("THRUM_HORIZON_OUTPUT").expect("output directory");
    let output = Path::new(&output);

    let (device, queue) = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions::default())
            .await.expect("Metal required");
        let info = adapter.get_info();
        assert_eq!(info.backend, wgpu::Backend::Metal);
        eprintln!("Metal adapter {}", info.name);
        adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: wgpu::Features::TIMESTAMP_QUERY, ..Default::default()
        }).await.expect("Metal timestamp device")
    });
    let input = std::env::var("THRUM_HORIZON_INPUT").unwrap_or_else(|_| dir.display().to_string());
    let input = Path::new(&input);
    let raster = fs::read(input.join("raster.bin")).expect("spur raster");
    let raster: Vec<f32> = raster.as_chunks::<4>().0.iter()
        .map(|word| f32::from_le_bytes(*word)).collect();
    assert_eq!(raster.len(), 1024 * 1024);
    let base = depth_target(&device, &queue, &raster, [1024,1024]);
    let parents = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("horizon parents"),
        size: wgpu::Extent3d { width: 512, height: 512, depth_or_array_layers: 1 },
        mip_level_count: 10, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    });
    let mips: Vec<_> = (0..10).map(|level| parents.create_view(&wgpu::TextureViewDescriptor {
        base_mip_level: level, mip_level_count: Some(1), ..Default::default()
    })).collect();
    let mip_chain = parents.create_view(&wgpu::TextureViewDescriptor::default());
    let texture = |sample_type| wgpu::BindingType::Texture {
        sample_type, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false,
    };
    let depth = texture(wgpu::TextureSampleType::Depth);
    let float = texture(wgpu::TextureSampleType::Float { filterable: false });
    let write = wgpu::BindingType::StorageTexture {
        access: wgpu::StorageTextureAccess::WriteOnly,
        format: wgpu::TextureFormat::R32Float, view_dimension: wgpu::TextureViewDimension::D2,
    };
    let tex = wgpu::BindingResource::TextureView;
    let reduce = module(&device, fs::read_to_string(dir.join("reduce.wgsl")).expect("reduction"));
    let (first, first_layout) = pipeline(&device, &reduce, "depth_parent", &[(0,depth),(2,write)]);
    let (later, later_layout) = pipeline(&device, &reduce, "mip_parent", &[(1,float),(2,write)]);
    let first_group = group(&device, &first_layout, [(0,tex(&base)),(2,tex(&mips[0]))]);
    let later_groups: Vec<_> = mips.windows(2).map(|pair| group(
        &device, &later_layout, [(1,tex(&pair[0])),(2,tex(&pair[1]))],
    )).collect();

    let ray_bytes = fs::read(input.join("rays.bin")).expect("rays");
    let count = u32::try_from(ray_bytes.len() / 48).expect("ray count");
    let rays = buffer_with(&device,&queue,&ray_bytes,wgpu::BufferUsages::STORAGE);
    let result_size = u64::from(count)*16;
    let result = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: result_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false,
    });
    let trace = module(&device, fs::read_to_string(dir.join("trace.wgsl")).expect("traversal"));
    let storage = |read_only| wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only },
        has_dynamic_offset: false, min_binding_size: None,
    };
    let common = [(0,depth),(2,storage(true)),(3,storage(false))];
    let (legacy, legacy_layout) = pipeline(&device, &trace, "legacy", &common);
    let hierarchy_bindings = [(0,depth),(1,float),(2,storage(true)),(3,storage(false))];
    let (tree, tree_layout) = pipeline(&device, &trace, "tree", &hierarchy_bindings);
    let (dda, dda_layout) = pipeline(&device, &trace, "dda", &hierarchy_bindings);
    let legacy_group = group(&device, &legacy_layout, [
        (0,tex(&base)),(2,rays.as_entire_binding()),(3,result.as_entire_binding()),
    ]);
    let tree_group = group(&device, &tree_layout, [
        (0,tex(&base)),(1,tex(&mip_chain)),(2,rays.as_entire_binding()),(3,result.as_entire_binding()),
    ]);

    let dda_group = group(&device, &dda_layout, [
        (0,tex(&base)),(1,tex(&mip_chain)),(2,rays.as_entire_binding()),(3,result.as_entire_binding()),
    ]);
    let traversals = [(&legacy,&legacy_group),(&tree,&tree_group),(&dda,&dda_group)];
    let outputs = std::array::from_fn::<_, 3, _>(|_| staging(&device,result_size));
    let samples = 12;
    let query_count = samples * 8;
    let query = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: None, ty: wgpu::QueryType::Timestamp, count: query_count,
    });
    let query_bytes = u64::from(query_count)*8;
    let resolve = device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: query_bytes,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let times = staging(&device,query_bytes);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let stamps = |begin| wgpu::ComputePassTimestampWrites {
        query_set: &query, beginning_of_pass_write_index: Some(begin),
        end_of_pass_write_index: Some(begin+1),
    };
    for sample in 0..samples {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("horizon parent mips"), timestamp_writes: Some(stamps(sample*8)),
            });
            pass.set_pipeline(&first);
            pass.set_bind_group(0,&first_group,&[]);
            pass.dispatch_workgroups(64,64,1);
            pass.set_pipeline(&later);
            for (level, group) in later_groups.iter().enumerate() {
                let width = (256u32 >> level).div_ceil(8);
                pass.set_bind_group(0,group,&[]);
                pass.dispatch_workgroups(width,width,1);
            }
        }
        let order = match sample % 6 {
            0 => [0,1,2], 1 => [0,2,1], 2 => [1,0,2],
            3 => [1,2,0], 4 => [2,0,1], _ => [2,1,0],
        };
        for row in order {
            let (pipeline, group) = traversals[row as usize];
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("horizon rays"),
                    timestamp_writes: Some(stamps(sample*8+2+2*row)),
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0,group,&[]);
                pass.dispatch_workgroups(count.div_ceil(64),1,1);
            }
            if sample == 0 {
                encoder.copy_buffer_to_buffer(&result,0,&outputs[row as usize],0,Some(result_size));
            }
        }
    }
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(&legacy);
        pass.set_bind_group(0,&legacy_group,&[]);
        pass.dispatch_workgroups(1,1,1);
    }
    encoder.resolve_query_set(&query,0..query_count,&resolve,0);
    encoder.copy_buffer_to_buffer(&resolve,0,&times,0,Some(query_bytes));
    queue.submit([encoder.finish()]);
    for (file, buffer) in ["legacy.bin","tree.bin","dda.bin"].into_iter().zip(&outputs) {
        read(&device,buffer,&output.join(file));
    }
    read(&device,&times,&output.join("timestamps.bin"));
    eprintln!("timestamp period {} ns; {count} terrain rays", queue.get_timestamp_period());
}
