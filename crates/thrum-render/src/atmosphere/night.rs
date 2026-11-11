//! Fixed Earth night transport, prepared once on the view's device.

use wgpu::util::DeviceExt as _;

use crate::graph::{Access, AccessKind, FrameGraph, GraphError, ResourceDescriptor, ResourceScope};

pub(super) const CONTROL_BYTES: u64 = 16;
const WIDTH: u32 = 96;
const HEIGHT: u32 = 64;
const PROFILE: &[u8; (WIDTH * HEIGHT * 4) as usize] = include_bytes!("night/earth.rg16f");

pub(super) struct NightProfile {
    texture: wgpu::Texture,
    pub(super) control: wgpu::Buffer,
    pub(super) view: wgpu::TextureView,
    pub(super) enabled: bool,
}

impl NightProfile {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, enabled: bool) -> Self {
        let size = wgpu::Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Earth night transport profile"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            texture.as_image_copy(),
            PROFILE,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH * 4),
                rows_per_image: Some(HEIGHT),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let control = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Earth night profile selection"),
            contents: [u32::from(enabled).to_le_bytes(), [0; 4], [0; 4], [0; 4]].as_flattened(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            texture,
            control,
            view,
            enabled,
        }
    }

    pub(super) fn select(&mut self, queue: &wgpu::Queue, enabled: bool) -> bool {
        if self.enabled == enabled {
            return false;
        }
        self.enabled = enabled;
        queue.write_buffer(&self.control, 0, &u32::from(enabled).to_le_bytes());
        true
    }

    pub(super) fn graph_reads(&self, graph: &mut FrameGraph) -> Result<[Access; 2], GraphError> {
        let night_profile = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width: self.texture.width(),
                height: self.texture.height(),
                layers: 1,
                mips: 1,
                samples: 1,
                format: self.texture.format(),
                usage: self.texture.usage(),
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let night_control = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: CONTROL_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        Ok([
            Access {
                view: None,
                resource: night_profile,
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            },
            Access {
                view: None,
                resource: night_control,
                range: 0..CONTROL_BYTES,
                kind: AccessKind::UniformRead,
                store: true,
            },
        ])
    }

    pub(super) fn bindings(&self) -> [wgpu::BindGroupEntry<'_>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&self.view),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: self.control.as_entire_binding(),
            },
        ]
    }
}

pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        super::gpu::sampled_entry(6, wgpu::ShaderStages::COMPUTE),
        wgpu::BindGroupLayoutEntry {
            binding: 7,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(CONTROL_BYTES),
            },
            count: None,
        },
    ]
}
