//! M10's first boundary experiment. This must pass before route integration.
#![cfg(target_os = "macos")]

use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::NSString;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLComputeCommandEncoder,
    MTLComputePipelineState, MTLDevice, MTLLibrary, MTLOrigin, MTLSize, MTLTexture,
};
use thrum_render::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, ResourceDescriptor, ResourceScope, TextureKind,
};

fn fixture_graph() -> thrum_render::graph::CompiledGraph {
    let mut graph = FrameGraph::default();
    let mut texture = |scope| {
        graph
            .resource(
                ResourceDescriptor::Texture {
                    dimension: TextureKind::D2,
                    width: 32,
                    height: 4,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: wgpu::TextureFormat::Rgba16Float,
                    usage: wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING,
                },
                scope,
            )
            .expect("fixture graph texture")
    };
    let scene = texture(ResourceScope::Transient);
    let previous = texture(ResourceScope::Imported { initialized: true });
    let output = texture(ResourceScope::Persistent {
        owner: 1,
        epoch: 1,
        initialized: true,
    });
    let next = texture(ResourceScope::History {
        owner: 2,
        epoch: 1,
        initialized: true,
    });
    let access = |resource, kind| Access {
        resource,
        kind,
        range: 0..1,
        store: true,
        view: None,
    };
    let pass = |name: &str, accesses| Pass {
        name: name.into(),
        scope: PassScope::Shared,
        pipeline: None,
        accesses,
        external_side_effect: true,
    };
    graph.pass(pass(
        "wgpu producer",
        vec![access(scene.clone(), AccessKind::CopyWrite)],
    ));
    graph.native_pass(pass(
        "whole raw copy/select",
        vec![
            access(scene, AccessKind::CopyRead),
            access(previous.clone(), AccessKind::CopyRead),
            access(previous, AccessKind::SampledRead),
            access(output.clone(), AccessKind::StorageWrite),
            access(next.clone(), AccessKind::StorageWrite),
        ],
    ));
    graph.pass(pass(
        "wgpu readback consumer",
        vec![
            access(output, AccessKind::CopyRead),
            access(next, AccessKind::CopyRead),
        ],
    ));
    graph
        .compile(1, &std::collections::BTreeSet::new())
        .expect("declared native fixture")
}

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

#[expect(
    unsafe_code,
    reason = "Retain one native texture while its wgpu owner stays live, then release the non-reentrant HAL guard before taking another"
)]
fn native_texture(texture: &wgpu::Texture) -> Retained<ProtocolObject<dyn MTLTexture>> {
    // SAFETY: the caller retains the live wgpu texture through GPU completion.
    // Only a balanced native retain escapes; no mutation, destruction or wgpu
    // operation occurs while the device's non-reentrant snatch guard is held.
    let guard = unsafe { texture.as_hal::<wgpu::hal::api::Metal>() }.expect("Metal texture");
    guard.raw_handle().into()
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

    fn copy(&self, encoder: &mut wgpu::CommandEncoder, select_history: bool) {
        for target in [&self.output, &self.next] {
            encoder.copy_texture_to_texture(
                self.scene.as_image_copy(),
                target.as_image_copy(),
                EXTENT,
            );
            if !select_history {
                continue;
            }
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
        select_history: bool,
    ) {
        // The wgpu owners stay live through GPU completion. Balanced native
        // retains hold the handles during recording without retaining HAL guards.
        // All textures were initialized via wgpu; extents/formats/usages match. Metal
        // tracked resources order blit/compute and subsequent same-queue consumers.
        // Only native encoders are ended here; wgpu owns the command buffer itself.
        let scene = native_texture(&self.scene);
        let previous = native_texture(&self.previous);
        let output = native_texture(&self.output);
        let next = native_texture(&self.next);
        let record = |raw: Option<&mut wgpu::hal::metal::CommandEncoder>| {
            let buffer = raw
                .expect("Metal encoder")
                .raw_command_buffer()
                .expect("raw command buffer");
            let blit = buffer.blitCommandEncoder().expect("blit");
            for target in [&*output, &*next] {
                // SAFETY: both textures have the same checked extent/format and stay retained.
                unsafe {
                    blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        &scene, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 }, MTLSize { width: 32, height: 4, depth: 1 }, target, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 });
                }
                if !select_history {
                    continue;
                }
                // SAFETY: the 8x2 crop lies inside both retained 32x4 textures.
                unsafe {
                    blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                        &previous, 0, 0, MTLOrigin { x: 4, y: 1, z: 0 }, MTLSize { width: 8, height: 2, depth: 1 }, target, 0, 0, MTLOrigin { x: 4, y: 1, z: 0 });
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
                compute.setTexture_atIndex(Some(&previous), 0);
            }
            // SAFETY: pipeline texture 1 is a retained storage-writable output texture.
            unsafe {
                compute.setTexture_atIndex(Some(&output), 1);
            }
            // SAFETY: pipeline texture 2 is retained and does not alias previous or output.
            unsafe {
                compute.setTexture_atIndex(Some(&next), 2);
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
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("poll");
    receive
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("map callback")
        .expect("readback");
    buffer
        .slice(..)
        .get_mapped_range()
        .expect("mapped bytes")
        .to_vec()
}

#[test]
fn whole_raw_encoder_copy_select_matches_prepared_wgsl_in_one_submit() {
    let (actual, pattern) = fixture(Fault::None);
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
    for fault in [
        Fault::NoCommands,
        Fault::NoTile,
        Fault::NoReset,
        Fault::StaleSource,
        Fault::StaleHistory,
    ] {
        let (broken, _) = fixture(fault);
        assert_ne!(broken[0], broken[2], "{fault:?} must fail exact output");
        assert_ne!(broken[1], broken[3], "{fault:?} must fail exact history");
    }
    println!(
        "M10 boundary: eight single-submit frames exact; all five omission/stale/reset controls rejected"
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fault {
    None,
    NoCommands,
    NoTile,
    NoReset,
    StaleSource,
    StaleHistory,
}

async fn metal_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("Metal adapter");
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("Metal device")
}

#[expect(
    unsafe_code,
    reason = "A dedicated no-op raw callback tests locked-wgpu API separation without mutating or ending the native command buffer"
)]
fn raw_noop(encoder: &mut wgpu::CommandEncoder) {
    // SAFETY: no raw object escapes and no commands, wgpu calls, end or commit
    // happen in this callback. The test intentionally tries the other API only
    // after this call has returned, and expects wgpu's refusal before recording.
    unsafe {
        encoder.as_hal_mut::<wgpu::hal::api::Metal, _, _>(|raw| assert!(raw.is_some()));
    }
}

fn wgpu_copy(device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder) {
    let buffer = || {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("API separation fixture"),
            size: 8,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    };
    encoder.copy_buffer_to_buffer(&buffer(), 0, &buffer(), 0, 8);
}

#[test]
#[should_panic(
    expected = "Mixing the wgpu encoding API with the raw encoding API is not permitted"
)]
fn locked_wgpu_refuses_raw_then_wgpu_even_for_noop_raw_callback() {
    let (device, _queue) = pollster::block_on(metal_device());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    raw_noop(&mut encoder);
    wgpu_copy(&device, &mut encoder);
}

#[test]
#[should_panic(
    expected = "Mixing the wgpu encoding API with the raw encoding API is not permitted"
)]
fn locked_wgpu_refuses_wgpu_then_raw_even_for_noop_raw_callback() {
    let (device, _queue) = pollster::block_on(metal_device());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    wgpu_copy(&device, &mut encoder);
    raw_noop(&mut encoder);
}

#[expect(
    clippy::print_stdout,
    reason = "The device fixture records its actual Metal adapter in the retained test log"
)]
fn fixture(fault: Fault) -> (Vec<Vec<u8>>, Vec<u8>) {
    pollster::block_on(async {
        let graph = fixture_graph();
        assert_eq!(
            graph.pass_names().collect::<Vec<_>>(),
            [
                "wgpu producer",
                "whole raw copy/select",
                "wgpu readback consumer"
            ]
        );
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("Metal adapter");
        println!("M10 Metal adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("device");
        let mut native = Textures::new(&device);
        let mut twin = Textures::new(&device);
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
        let binds: [_; 2] = std::array::from_fn(|side| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &if side == 0 { [0, 1, 2] } else { [2, 1, 0] }
                    .iter()
                    .enumerate()
                    .map(|(i, &view)| wgpu::BindGroupEntry {
                        binding: u32::try_from(i).expect("binding"),
                        resource: wgpu::BindingResource::TextureView(&views[view]),
                    })
                    .collect::<Vec<_>>(),
            })
        });
        let upload = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut last = None;
        for frame in 0..8_u16 {
            let reset = frame == 3;
            if reset {
                twin.initialize(&queue);
                if fault != Fault::NoReset {
                    native.initialize(&queue);
                }
            }
            let select_history = frame % 2 == 1 && !reset;
            let mut producer =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            // Both scene inputs change in this submission, after initialization. Each
            // finite nonnegative half pattern is exact, including tiny/subnormal texels.
            let pattern: Vec<u8> = (0..512_u16)
                .flat_map(|i| (i + frame * 512).to_le_bytes())
                .collect();
            queue.write_buffer(&upload, 0, &pattern);
            for (skip, textures) in [
                (fault == Fault::StaleSource && frame != 0, &native),
                (false, &twin),
            ] {
                if skip {
                    continue;
                }
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
            if fault != Fault::NoCommands {
                native.native(&mut raw, &kernel, fault == Fault::NoTile, select_history);
            }
            let mut consumer =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            twin.copy(&mut consumer, select_history);
            {
                let mut pass = consumer.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &binds[usize::from(frame % 2)], &[]);
                pass.dispatch_workgroups(1, 1, 1);
            }
            let buffers = [&native.output, &native.next, &twin.output, &twin.next]
                .map(|t| readback(&device, &mut consumer, t));
            let mut commands = [
                Some(producer.finish()),
                Some(raw.finish()),
                Some(consumer.finish()),
            ];
            queue.submit(
                graph
                    .pass_order
                    .iter()
                    .map(|&pass| commands[pass].take().expect("one encoder per graph pass")),
            );
            let result = buffers
                .iter()
                .map(|b| bytes(&device, b))
                .collect::<Vec<_>>();
            if fault == Fault::None {
                assert_eq!(result[0], result[2], "frame {frame} output");
                assert_eq!(result[1], result[3], "frame {frame} history");
                let bit = u8::from(if frame < 3 {
                    frame % 2 == 0
                } else {
                    frame % 2 == 1
                });
                assert_eq!(
                    &result[0][..8],
                    &[0, 60 * bit, 0, 60 * bit, 0, 60 * bit, 0, 60 * bit],
                    "frame {frame} submitted tile state"
                );
                if select_history {
                    let prior: Vec<u8> = (144..176_u16)
                        .flat_map(|i| (i + (frame - 1) * 512).to_le_bytes())
                        .collect();
                    assert_eq!(
                        &result[0][288..352],
                        prior,
                        "frame {frame} selects prior submitted crop"
                    );
                }
            }
            if fault != Fault::StaleHistory {
                std::mem::swap(&mut native.previous, &mut native.next);
            }
            std::mem::swap(&mut twin.previous, &mut twin.next);
            last = Some((result, pattern));
        }
        last.expect("eight fixture frames")
    })
}
