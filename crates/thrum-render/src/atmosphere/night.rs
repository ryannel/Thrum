//! Fixed Earth night transport, prepared once on the view's device.

use crate::binding;
use wgpu::util::DeviceExt as _;

use crate::graph::{Access, AccessKind, FrameGraph, GraphError, ResourceDescriptor, ResourceScope};

pub(super) const CONTROL_BYTES: u64 = size_of::<[u32; 4]>() as u64;
const WIDTH: u32 = 96;
const HEIGHT: u32 = 64;
const PROFILE: &[u8; (WIDTH * HEIGHT * 4) as usize] = include_bytes!("night/earth.rg16f");

#[derive(Clone)]
pub(super) struct NightProfile {
    texture: wgpu::Texture,
    pub(super) control: wgpu::Buffer,
    pub(super) view: wgpu::TextureView,
    pub(super) enabled: bool,
}

impl NightProfile {
    /// Reuse the world's immutable emission field with an observer's own enable
    /// control. Changing observer altitude never changes the world's record.
    pub(super) fn observer(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enabled: bool,
    ) -> Self {
        let control = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("observer night transport control"),
            size: CONTROL_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut profile = Self {
            texture: self.texture.clone(),
            view: self.view.clone(),
            control,
            enabled: !enabled,
        };
        profile.select(queue, enabled);
        profile
    }

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
            contents: bytemuck::bytes_of(&night_control(enabled)),
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
        queue.write_buffer(
            &self.control,
            0,
            bytemuck::bytes_of(&night_control(enabled)),
        );
        true
    }

    pub(super) fn graph_reads(&self, graph: &mut FrameGraph) -> Result<[Access; 2], GraphError> {
        let night_profile = graph.resource(
            ResourceDescriptor::of_texture(&self.texture),
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
            Access::new(night_profile, 0..1, AccessKind::SampledRead),
            Access::new(night_control, 0..CONTROL_BYTES, AccessKind::UniformRead),
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
        binding::texture(
            6,
            wgpu::ShaderStages::COMPUTE,
            wgpu::TextureSampleType::Float { filterable: true },
            wgpu::TextureViewDimension::D2,
        ),
        binding::uniform(7, wgpu::ShaderStages::COMPUTE, CONTROL_BYTES),
    ]
}

fn night_control(enabled: bool) -> [u32; 4] {
    [u32::from(enabled), 0, 0, 0]
}
