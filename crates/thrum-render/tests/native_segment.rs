//! M10's first boundary experiment. This must pass before route integration.
#![cfg(target_os = "macos")]

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLComputeCommandEncoder,
    MTLComputePipelineState, MTLDevice, MTLLibrary, MTLOrigin, MTLSize,
};

const EXTENT: wgpu::Extent3d = wgpu::Extent3d {
    width: 32,
    height: 4,
    depth_or_array_layers: 1,
};

const MSL: &str = r"
#include <metal_stdlib>
using namespace metal;
kernel void toggle(texture2d<half, access::read> previous [[texture(0)]],
                   texture2d<half, access::write> output [[texture(1)]],
                   texture2d<half, access::write> next [[texture(2)]]) {
    uint bit = uint(previous.read(uint2(0)).x);
    half4 value = bit == 0u ? half4(1.0h) : half4(0.0h);
    output.write(value, uint2(0));
    next.write(value, uint2(0));
}
";

const WGSL: &str = r"
@group(0) @binding(0) var previous: texture_2d<f32>;
@group(0) @binding(1) var output: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var next: texture_storage_2d<rgba16float, write>;
@compute @workgroup_size(1) fn toggle() {
    let bit = u32(textureLoad(previous, vec2u(0), 0).x);
    let value = select(vec4f(0.0), vec4f(1.0), bit == 0u);
    textureStore(output, vec2u(0), value);
    textureStore(next, vec2u(0), value);
}
";

fn texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("native copy/select RGBA16F"),
        size: EXTENT,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    })
}

#[expect(
    unsafe_code,
    reason = "M10 isolated Metal fixture, retained device guard used only during pipeline preparation"
)]
fn native_pipeline(device: &wgpu::Device) -> Retained<ProtocolObject<dyn MTLComputePipelineState>> {
    // SAFETY: the guard retains the live device; pipeline creation occurs before recording.
    let metal = unsafe { device.as_hal::<wgpu::hal::api::Metal>() }.expect("Metal device");
    let library = metal
        .raw_device()
        .newLibraryWithSource_options_error(&NSString::from_str(MSL), None)
        .expect("native tile library");
    let function = library
        .newFunctionWithName(&NSString::from_str("toggle"))
        .expect("tile kernel");
    metal
        .raw_device()
        .newComputePipelineStateWithFunction_error(&function)
        .expect("native tile pipeline")
}

struct Textures {
    scene: wgpu::Texture,
    previous: wgpu::Texture,
    output: wgpu::Texture,
    next: wgpu::Texture,
}

impl Textures {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            scene: texture(device),
            previous: texture(device),
            output: texture(device),
            next: texture(device),
        }
    }

    fn initialize(&self, queue: &wgpu::Queue) {
        for texture in [&self.scene, &self.previous, &self.output, &self.next] {
            queue.write_texture(
                texture.as_image_copy(),
                &[0; 1024],
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(4),
                },
                EXTENT,
            );
        }
    }

    fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        for target in [&self.output, &self.next] {
            encoder.copy_texture_to_texture(
                self.scene.as_image_copy(),
                target.as_image_copy(),
                EXTENT,
            );
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d { x: 4, y: 1, z: 0 },
                    ..self.previous.as_image_copy()
                },
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d { x: 4, y: 1, z: 0 },
                    ..target.as_image_copy()
                },
                wgpu::Extent3d {
                    width: 8,
                    height: 2,
                    depth_or_array_layers: 1,
                },
            );
        }
    }

    #[expect(
        unsafe_code,
        reason = "Whole raw encoder, bounded tracked RGBA16F textures retained through completion; no wgpu recording on this encoder"
    )]
    fn native(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &ProtocolObject<dyn MTLComputePipelineState>,
        skip_tile: bool,
    ) {
        // SAFETY: all guards and their wgpu owners outlive recording and submission.
        // All textures were initialized via wgpu; extents/formats/usages match. Metal
        // tracked resources order blit/compute and subsequent same-queue consumers.
        // Only native encoders are ended here; wgpu owns the command buffer itself.
        let scene = unsafe { self.scene.as_hal::<wgpu::hal::api::Metal>() }.expect("scene");
        // SAFETY: retains the initialized previous texture, as above.
        let previous =
            unsafe { self.previous.as_hal::<wgpu::hal::api::Metal>() }.expect("previous");
        // SAFETY: retains the initialized output texture, as above.
        let output = unsafe { self.output.as_hal::<wgpu::hal::api::Metal>() }.expect("output");
        // SAFETY: retains the initialized next texture, distinct from previous.
        let next = unsafe { self.next.as_hal::<wgpu::hal::api::Metal>() }.expect("next");
        let record = |raw: Option<&mut wgpu::hal::metal::CommandEncoder>| {
            let buffer = raw
                .expect("Metal encoder")
                .raw_command_buffer()
                .expect("raw command buffer");
            let blit = buffer.blitCommandEncoder().expect("blit");
            for target in [output.raw_handle(), next.raw_handle()] {
                // SAFETY: both textures have the same checked extent/format and stay retained.
                unsafe {
                    blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        scene.raw_handle(), 0, 0, MTLOrigin { x: 0, y: 0, z: 0 }, MTLSize { width: 32, height: 4, depth: 1 }, target, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 });
                }
                // SAFETY: the 8x2 crop lies inside both retained 32x4 textures.
                unsafe {
                    blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        previous.raw_handle(), 0, 0, MTLOrigin { x: 4, y: 1, z: 0 }, MTLSize { width: 8, height: 2, depth: 1 }, target, 0, 0, MTLOrigin { x: 4, y: 1, z: 0 });
                }
            }
            blit.endEncoding();
            if skip_tile {
                return;
            }
            let compute = buffer.computeCommandEncoder().expect("compute");
            compute.setComputePipelineState(pipeline);
            // SAFETY: pipeline texture 0 is a retained read-only previous texture.
            unsafe {
                compute.setTexture_atIndex(Some(previous.raw_handle()), 0);
            }
            // SAFETY: pipeline texture 1 is a retained storage-writable output texture.
            unsafe {
                compute.setTexture_atIndex(Some(output.raw_handle()), 1);
            }
            // SAFETY: pipeline texture 2 is retained and does not alias previous or output.
            unsafe {
                compute.setTexture_atIndex(Some(next.raw_handle()), 2);
            }
            compute.dispatchThreads_threadsPerThreadgroup(
                MTLSize {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
                MTLSize {
                    width: 1,
                    height: 1,
                    depth: 1,
                },
            );
            compute.endEncoding();
        };
        // SAFETY: this dedicated encoder has no wgpu commands; callback only ends
        // its native blit/compute encoders, leaving the command buffer to wgpu.
        unsafe {
            encoder.as_hal_mut::<wgpu::hal::api::Metal, _, _>(record);
        }
    }
}

fn readback(
    device: &wgpu::Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &wgpu::Texture,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("exact native fixture readback"),
        size: 1024,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(4),
            },
        },
        EXTENT,
    );
    buffer
}

fn bytes(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).expect("map result");
        });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    receive.recv().expect("map callback").expect("readback");
    buffer
        .slice(..)
        .get_mapped_range()
        .expect("mapped bytes")
        .to_vec()
}

#[test]
fn whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit() {
    let (actual, pattern) = fixture(false, false);
    assert_eq!(actual[0], actual[2], "exact output");
    assert_eq!(actual[1], actual[3], "exact history");
    assert_eq!(
        &actual[0][0..8],
        &[0, 60, 0, 60, 0, 60, 0, 60],
        "native storage write toggled tile"
    );
    assert_eq!(
        &actual[0][8..256],
        &pattern[8..256],
        "same-submit producer actually consumed"
    );
    let (no_tile, _) = fixture(false, true);
    assert_ne!(
        no_tile[0], no_tile[2],
        "omitting native storage writes must fail exact output"
    );
    assert_ne!(
        no_tile[1], no_tile[3],
        "omitting native storage writes must fail exact history"
    );
    let (no_native, _) = fixture(true, false);
    assert_ne!(
        no_native[0], no_native[2],
        "no-op native work must fail exact output"
    );
    assert_ne!(
        no_native[1], no_native[3],
        "no-op native work must fail exact history"
    );
}

fn fixture(skip_native: bool, skip_tile: bool) -> (Vec<Vec<u8>>, Vec<u8>) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let native = Textures::new(&device);
        let twin = Textures::new(&device);
        native.initialize(&queue);
        twin.initialize(&queue);
        let kernel = native_pipeline(&device);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("prepared exact tile twin"),
            source: wgpu::ShaderSource::Wgsl(WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("prepared WGSL tile"),
            layout: None,
            module: &shader,
            entry_point: Some("toggle"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let views = [&twin.previous, &twin.output, &twin.next]
            .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()));
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &views
                .iter()
                .enumerate()
                .map(|(i, view)| wgpu::BindGroupEntry {
                    binding: u32::try_from(i).expect("binding"),
                    resource: wgpu::BindingResource::TextureView(view),
                })
                .collect::<Vec<_>>(),
        });
        let mut producer =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // Both scene inputs change in this submission, after initialization. Each
        // finite nonnegative half pattern is exact, including tiny/subnormal texels.
        let pattern: Vec<u8> = (0..512_u16)
            .flat_map(|i| (i % 0x3c00).to_le_bytes())
            .collect();
        let upload = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        upload
            .slice(..)
            .get_mapped_range_mut()
            .expect("upload mapping")
            .copy_from_slice(&pattern);
        upload.unmap();
        for textures in [&native, &twin] {
            producer.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer: &upload,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(4),
                    },
                },
                textures.scene.as_image_copy(),
                EXTENT,
            );
        }
        let mut raw = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        if !skip_native {
            native.native(&mut raw, &kernel, skip_tile);
        }
        let mut consumer =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        twin.copy(&mut consumer);
        {
            let mut pass = consumer.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }
        let buffers = [&native.output, &native.next, &twin.output, &twin.next]
            .map(|t| readback(&device, &mut consumer, t));
        queue.submit([producer.finish(), raw.finish(), consumer.finish()]);
        let result = buffers
            .iter()
            .map(|b| bytes(&device, b))
            .collect::<Vec<_>>();
        (result, pattern)
    })
}
