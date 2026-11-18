//! Embedded sheet preparation, view/row upload and the single sprite draw.

use super::{EmbeddedSheet, SpriteFrame, SpriteScene};
use crate::{
    capabilities::DeviceCapabilities,
    graph::{CompiledPass, GraphColorAttachment, PipelineKey, ResourceId},
    synthetic::{RenderError, SCENE_FORMAT},
    timing::GpuTimings,
};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub(crate) const PASS: &str = "sprites";
pub(super) const SHEET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const SHADER: &str = include_str!("pass.wgsl");

#[cfg(test)]
mod tests;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuView {
    clip_per_world: [f32; 2],
    padding: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuRow {
    pivot: [f32; 2],
    right: [f32; 2],
    down: [f32; 2],
    frame: u32,
    padding: u32,
    tint: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuRect {
    origin: [u32; 2],
    size: [u32; 2],
}

pub(super) struct SpritePass {
    pub(super) key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    pub(super) rows_bytes: u64,
    pub(super) frames: wgpu::Buffer,
    pub(super) sheet: wgpu::Texture,
    sheet_view: wgpu::TextureView,
    row_capacity: usize,
    scratch: Vec<GpuRow>,
}

pub(super) struct SpriteBindings {
    pub(super) view: wgpu::Buffer,
    pub(super) rows: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: u32,
}

pub(super) const VIEW_BYTES: u64 = size_of::<GpuView>() as u64;

impl SpritePass {
    pub(super) fn prepare(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SpriteScene,
        key: PipelineKey,
    ) -> Result<Self, RenderError> {
        let limits = capabilities.limits();
        let sheet = &scene.sheet;
        let [width, height] = sheet.extent;
        let pixel_bytes = u64::from(width) * u64::from(height);
        let rows_bytes = u64::from(scene.row_capacity) * size_of::<GpuRow>() as u64;
        let frame_bytes = sheet.frames.len() as u64 * size_of::<GpuRect>() as u64;
        let max_binding = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        if sheet
            .extent
            .into_iter()
            .any(|side| side == 0 || side > limits.max_texture_dimension_2d)
            || pixel_bytes.checked_mul(4) != Some(sheet.rgba.len() as u64)
            || !sheet.texels_per_unit.is_finite()
            || sheet.texels_per_unit <= 0.0
            || scene.row_capacity == 0
            || rows_bytes > max_binding
            || sheet.frames.is_empty()
            || frame_bytes > max_binding
            || sheet.frames.iter().any(|rect| {
                (0..2).any(|axis| {
                    rect.size[axis] == 0
                        || rect.origin[axis]
                            .checked_add(rect.size[axis])
                            .is_none_or(|end| end > sheet.extent[axis])
                })
            })
        {
            return Err(RenderError::Validation(
                "invalid embedded sheet or sprite row capacity".into(),
            ));
        }
        let device = capabilities.site();
        let frames: Vec<_> = sheet
            .frames
            .iter()
            .map(|rect| GpuRect {
                origin: rect.origin,
                size: rect.size,
            })
            .collect();
        let frames = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite frames"),
            contents: bytemuck::cast_slice(&frames),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let (sheet, sheet_view) = upload_sheet(device, queue, sheet);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprites"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some(PASS),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            key,
            pipeline,
            rows_bytes,
            frames,
            sheet,
            sheet_view,
            row_capacity: scene.row_capacity as usize,
            scratch: Vec::with_capacity(scene.row_capacity as usize),
        })
    }

    pub(super) fn bind(&self, device: &wgpu::Device) -> SpriteBindings {
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let view = buffer("sprite view", VIEW_BYTES, wgpu::BufferUsages::UNIFORM);
        let rows = buffer("sprite rows", self.rows_bytes, wgpu::BufferUsages::STORAGE);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: rows.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.frames.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.sheet_view),
                },
            ],
        });
        SpriteBindings {
            view,
            rows,
            group,
            count: 0,
        }
    }

    pub(super) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        sheet: &EmbeddedSheet,
        frame: SpriteFrame<'_>,
        extent: [u32; 2],
        bindings: &mut SpriteBindings,
    ) -> Result<crate::sprite::graph::SpriteUpload, RenderError> {
        let view = frame.view;
        let clip_per_world = view.clip_scale(extent)?;
        self.scratch.clear();
        for row in frame.rows {
            if self.scratch.len() == self.row_capacity {
                break;
            }
            let Some(rect) = sheet.frames.get(row.frame as usize) else {
                continue;
            };
            #[expect(
                clippy::cast_possible_truncation,
                reason = "camera-relative values enter the GPU domain once and are checked below"
            )]
            let pivot = [0, 1].map(|axis| (row.position[axis] - view.centre[axis]) as f32);
            let [x, y] = row.scale.map(|axis| axis / sheet.texels_per_unit);
            let (sin, cos) = row.rotation.sin_cos();
            let right = [cos * x, sin * x];
            let down = [sin * y, -cos * y];
            let valid = (0..2).all(|axis| {
                ((pivot[axis].abs()
                    + right[axis].abs() * rect.size[0] as f32
                    + down[axis].abs() * rect.size[1] as f32)
                    * clip_per_world[axis])
                    .is_finite()
            });
            if !valid
                || x == 0.0
                || y == 0.0
                || !row
                    .tint
                    .into_iter()
                    .all(|value| value.is_finite() && (0.0..=65_504.0).contains(&value))
                || row.tint[3] > 1.0
            {
                continue;
            }
            self.scratch.push(GpuRow {
                pivot,
                right,
                down,
                frame: row.frame,
                padding: 0,
                tint: row.tint,
            });
        }
        let bytes = bytemuck::cast_slice(&self.scratch);
        queue.write_buffer(
            &bindings.view,
            0,
            bytemuck::bytes_of(&GpuView {
                clip_per_world,
                padding: [0.0; 2],
            }),
        );
        if !bytes.is_empty() {
            queue.write_buffer(&bindings.rows, 0, bytes);
        }
        bindings.count =
            u32::try_from(self.scratch.len()).map_err(|_| RenderError::SceneCapacity)?;
        Ok(crate::sprite::graph::SpriteUpload {
            rows: bindings.count,
            bytes: size_of::<GpuView>() + bytes.len(),
            dropped: u32::try_from(frame.rows.len() - self.scratch.len()).unwrap_or(u32::MAX),
            projection: glam::Mat4::from_scale(glam::Vec3::new(
                clip_per_world[0],
                clip_per_world[1],
                1.0,
            ))
            .to_cols_array_2d(),
        })
    }

    pub(super) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        compiled: &CompiledPass,
        resource: &ResourceId,
        view: &wgpu::TextureView,
        bindings: &SpriteBindings,
        timings: Option<&mut GpuTimings>,
    ) -> Result<(), RenderError> {
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(GraphColorAttachment {
                    resource,
                    range: 0..1,
                    view,
                    clear: wgpu::Color::BLACK,
                })],
                None,
                self.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings.group, &[]);
        pass.draw(0..6, 0..bindings.count);
        Ok(())
    }
}

fn upload_sheet(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &EmbeddedSheet,
) -> (wgpu::Texture, wgpu::TextureView) {
    let extent = wgpu::Extent3d {
        width: source.extent[0],
        height: source.extent[1],
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("embedded sprite sheet"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SHEET_FORMAT,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let pixels: Vec<_> = source
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|rgba| {
            let alpha = f32::from(rgba[3]) / 255.0;
            let mut values = [alpha; 4];
            for (dest, &code) in values[..3].iter_mut().zip(rgba) {
                let srgb = f32::from(code) / 255.0;
                let linear = if srgb <= 0.04045 {
                    srgb / 12.92
                } else {
                    ((srgb + 0.055) / 1.055).powf(2.4)
                };
                *dest = linear * alpha;
            }
            values
                .into_iter()
                .map(|value| half::f16::from_f32(value).to_bits())
        })
        .collect();
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(&pixels),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8 * extent.width),
            rows_per_image: Some(extent.height),
        },
        extent,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    (texture, view)
}
