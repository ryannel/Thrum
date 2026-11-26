//! The recorded memory lengths and binding sizes must reach a real Metal dispatch.

thrum_test_support::macos_gpu_test! {
    fn metallib_dispatch_uses_the_wgsl_stage_record() {
        metal::check();
    }
}

#[cfg(target_os = "macos")]
mod metallib_cook;

#[cfg(target_os = "macos")]
mod metallib_route;

#[cfg(target_os = "macos")]
mod metal {
    use std::borrow::Cow;

    use thrum_test_support::{Scratch, readback::Readback};
    use wgpu::{
        hal::metal::{capture_stage_translations, encode_stage_records, split_stage_records},
        util::DeviceExt as _,
    };

    use super::{metallib_cook::cook, metallib_route};

    const SOURCE: &str = r"
        @group(0) @binding(0) var<storage, read> input: array<u32>;
        @group(0) @binding(1) var<storage, read_write> output: array<u32>;
        var<workgroup> values: array<u32, 4>;

        @compute @workgroup_size(4)
        fn accumulate(@builtin(local_invocation_index) i: u32) {
            values[i] = input[i] * (i + 1u);
            workgroupBarrier();
            if i == 0u {
                output[0] = arrayLength(&input);
                output[1] = values[0] + values[1] + values[2] + values[3];
            }
        }";

    pub(super) fn storage(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    pub(super) fn pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        module: &wgpu::ShaderModule,
        entry_point: &str,
    ) -> wgpu::ComputePipeline {
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("stage record witness"),
            layout: Some(layout),
            module,
            entry_point: Some(entry_point),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    }

    #[expect(
        unsafe_code,
        reason = "runs Metal produced by the pinned translator from this test's checked WGSL"
    )]
    pub(super) fn cooked_module(
        device: &wgpu::Device,
        blob: &[u8],
        function: &str,
        size: [u32; 3],
    ) -> wgpu::ShaderModule {
        let [x, y, z] = size;
        // SAFETY: The cooked library derives from the fixture WGSL and explicit layout.
        // The negative case changes only its stage record directory and must fail pipeline creation.
        unsafe {
            device.create_shader_module_passthrough(wgpu::ShaderModuleDescriptorPassthrough {
                label: Some("stage record witness"),
                entry_points: Cow::Owned(vec![wgpu::PassthroughShaderEntryPoint {
                    name: Cow::Borrowed(function),
                    workgroup_size: (x, y, z),
                }]),
                metallib: Some(Cow::Borrowed(blob)),
                ..Default::default()
            })
        }
    }

    fn dispatch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bindings: &wgpu::BindGroupLayout,
        pipeline: &wgpu::ComputePipeline,
    ) -> Vec<u32> {
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sized input"),
            contents: bytemuck::cast_slice(&[2_u32, 3, 5, 7, 11, 13]),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stage record witness output"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: bindings,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        dispatch_read(device, queue, &group, pipeline, &output)
    }

    pub(super) fn dispatch_read(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        group: &wgpu::BindGroup,
        pipeline: &wgpu::ComputePipeline,
        output: &wgpu::Buffer,
    ) -> Vec<u32> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let readback = Readback::buffer(device, &mut encoder, output, ..);
        let bytes = readback.wait(device, queue.submit([encoder.finish()]));
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .copied()
            .map(u32::from_le_bytes)
            .collect()
    }

    pub(super) fn check() {
        let Some(gpu) = thrum_test_support::gpu(
            "metallib_dispatch_uses_the_wgsl_stage_record",
            Some(wgpu::Backend::Metal),
        ) else {
            return;
        };
        let (device, queue, _) = gpu
            .device_with_hardware(
                thrum_test_support::Profile::Native,
                &thrum_test_support::Restriction::default(),
                1,
                &[thrum_render::capabilities::HardwareRequest {
                    feature: thrum_render::capabilities::HardwareFeature::MetalShaderPassthrough,
                    required: true,
                }],
            )
            .expect("Metal passthrough device");
        let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[storage(0, true), storage(1, false)],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bindings)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(SOURCE.into()),
        });
        let (reference, translations) =
            capture_stage_translations(|| pipeline(&device, &layout, &module, "accumulate"));
        let [translation] = translations.as_slice() else {
            panic!("expected a single backend translation, got {translations:?}");
        };
        let record = &translation.record;
        assert_eq!(record.workgroup_size, [4, 1, 1]);
        assert_eq!(record.workgroup_memory_sizes, [16]);
        assert_eq!(record.sized_bindings.len(), 2);
        assert_ne!(record.immutable_buffer_mask, 0);

        let scratch = Scratch::new("metallib-reflection");
        let blob = cook(translation, &scratch).expect("offline cook from the backend translation");
        let (_, library) = split_stage_records(&blob).expect("split stage records");
        let module = cooked_module(&device, &blob, &record.function, record.workgroup_size);
        let cooked = pipeline(&device, &layout, &module, &record.function);
        let reference = dispatch(&device, &queue, &bindings, &reference);
        assert_eq!(reference, [6, 51]);
        assert_eq!(dispatch(&device, &queue, &bindings, &cooked), reference);

        let missing = encode_stage_records(&[], library);
        let module = cooked_module(&device, &missing, &record.function, record.workgroup_size);
        let scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
        let _invalid = pipeline(&device, &layout, &module, &record.function);
        let error = pollster::block_on(scope.pop())
            .expect("the named stage record is required")
            .to_string();
        assert!(
            error.contains(&format!(
                "Metal stage record missing for '{}'",
                record.function
            )),
            "{error}"
        );

        metallib_route::visibility(&device, &queue);
        metallib_route::diffuse(&device, &queue);
    }
}
