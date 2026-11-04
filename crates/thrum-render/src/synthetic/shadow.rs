//! Two prepared finite reverse-Z Sun cascades for the synthetic perspective view.
//! One depth array belongs to each target; the prepared pipelines and light
//! matrices belong to its device generation and checked camera sample.

use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::view::{Projection, SyntheticView};
use super::{InstanceRecord, RenderError};

pub(super) const SIZE: u32 = 1024;
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const SAMPLE_BYTES: usize = 160;
const MAPS: usize = 2;
const LAYERS: u32 = 2;
const BIAS_BASE_WORLD: f32 = 0.006;
const BIAS_SLOPE_WORLD: f32 = 0.06;

fn split_depth(near: f32, far: f32) -> f32 {
    near.midpoint(far).midpoint((near * far).sqrt())
}

struct Cascade {
    matrix: Mat4,
    depth_row: [f32; 4],
    depth_span: f32,
    view_group: wgpu::BindGroup,
}

impl Cascade {
    fn contains_caster(&self, instance: &InstanceRecord) -> bool {
        let [x, y, z, offset] = self.depth_row;
        let center = instance.current[3];
        let depth = x * center[0] + y * center[1] + z * center[2] + offset;
        let extent = (0..3)
            .map(|axis| {
                let column = instance.current[axis];
                (x * column[0] + y * column[1] + z * column[2]).abs()
            })
            .sum::<f32>();
        depth - extent >= -1e-4 && depth + extent <= 1.0 + 1e-4
    }
}

pub(super) struct ShadowPrepared {
    pipeline: wgpu::RenderPipeline,
    cascades: [Cascade; MAPS],
    pub(super) sample_data: [u8; SAMPLE_BYTES],
    pub(super) sample_buffer: wgpu::Buffer,
    pub(super) sampler: wgpu::Sampler,
}

pub(super) struct ShadowTarget {
    _texture: wgpu::Texture,
    views: [wgpu::TextureView; MAPS],
    pub(super) array_view: wgpu::TextureView,
    pub(super) sample_data: [u8; SAMPLE_BYTES],
}

impl ShadowPrepared {
    pub(super) fn contains_caster(&self, instance: &InstanceRecord) -> bool {
        self.cascades
            .iter()
            .all(|cascade| cascade.contains_caster(instance))
    }

    pub(super) fn new(
        device: &wgpu::Device,
        view: &SyntheticView,
        instances: &[InstanceRecord],
        light_direction: Vec3,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, RenderError> {
        let Projection::Perspective {
            view: camera_view,
            fx,
            fy,
            near,
            far,
        } = view.projection()
        else {
            return Err(RenderError::InvalidView);
        };
        let split = split_depth(near, far);
        let overlap = (split * 0.05).max(0.05);
        let matrix_data = [
            cascade_matrix(
                camera_view,
                fx,
                fy,
                near,
                split + overlap,
                light_direction,
                instances,
            )?,
            cascade_matrix(
                camera_view,
                fx,
                fy,
                split - overlap,
                far,
                light_direction,
                instances,
            )?,
        ];
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic Sun cascade view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            }],
        });
        let cascades = matrix_data.map(|(matrix, depth_span)| {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic Sun cascade matrix"),
                contents: &matrix_bytes(matrix),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic Sun cascade view"),
                layout: &view_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            Cascade {
                matrix,
                depth_row: [
                    matrix.x_axis.z,
                    matrix.y_axis.z,
                    matrix.z_axis.z,
                    matrix.w_axis.z,
                ],
                depth_span,
                view_group,
            }
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic Sun shadow"),
            bind_group_layouts: &[Some(&view_layout), Some(instance_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic Sun caster"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("synthetic Sun caster"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Greater),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let mut samples = [0_u8; SAMPLE_BYTES];
        for (index, cascade) in cascades.iter().enumerate() {
            samples[index * 64..(index + 1) * 64].copy_from_slice(&matrix_bytes(cascade.matrix));
            let scale = 1.0 / cascade.depth_span;
            samples[144 + index * 4..148 + index * 4]
                .copy_from_slice(&(BIAS_BASE_WORLD * scale).to_le_bytes());
            samples[152 + index * 4..156 + index * 4]
                .copy_from_slice(&(BIAS_SLOPE_WORLD * scale).to_le_bytes());
        }
        samples[128..132].copy_from_slice(&split.to_le_bytes());
        samples[132..136].copy_from_slice(&overlap.to_le_bytes());
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic Sun cascade samples"),
            contents: &samples,
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("synthetic Sun comparison"),
            compare: Some(wgpu::CompareFunction::GreaterEqual),
            ..Default::default()
        });
        Ok(Self {
            pipeline,
            cascades,
            sample_data: samples,
            sample_buffer,
            sampler,
        })
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ShadowTarget,
        instances: &wgpu::BindGroup,
        count: u32,
        generation: u64,
        mut timings: Option<&mut crate::timing::GpuTimings>,
    ) -> Result<(), RenderError> {
        for (index, cascade) in self.cascades.iter().enumerate() {
            let name = if index == 0 {
                "synthetic Sun shadow near"
            } else {
                "synthetic Sun shadow far"
            };
            let writes = match timings.as_deref_mut() {
                Some(timings) => timings.pass_writes(generation, name)?,
                None => None,
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(name),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.views[index],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: writes,
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &cascade.view_group, &[]);
            pass.set_bind_group(1, instances, &[]);
            pass.draw(0..36, 0..count);
        }
        Ok(())
    }
}

impl ShadowTarget {
    pub(super) fn new(device: &wgpu::Device, prepared: &ShadowPrepared) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("synthetic Sun cascades"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let views = std::array::from_fn(|index| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("synthetic Sun cascade layer"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: u32::try_from(index).expect("two cascade layers"),
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("synthetic Sun cascade array"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self {
            _texture: texture,
            views,
            array_view,
            sample_data: prepared.sample_data,
        }
    }
}

fn matrix_bytes(matrix: Mat4) -> [u8; 64] {
    let mut bytes = [0_u8; 64];
    for (index, value) in matrix.to_cols_array().into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn cascade_matrix(
    camera_view: Mat4,
    fx: f32,
    fy: f32,
    near: f32,
    far: f32,
    direction: Vec3,
    instances: &[InstanceRecord],
) -> Result<(Mat4, f32), RenderError> {
    let camera_world = camera_view.inverse();
    let corners: [Vec3; 8] = std::array::from_fn(|index| {
        let depth = if index < 4 { near } else { far };
        let x = if index & 1 == 0 { -1.0 } else { 1.0 };
        let y = if index & 2 == 0 { -1.0 } else { 1.0 };
        camera_world.transform_point3(Vec3::new(x * depth / fx, y * depth / fy, -depth))
    });
    let center = corners.into_iter().sum::<Vec3>() / 8.0;
    let radius = corners
        .into_iter()
        .map(|corner| corner.distance(center))
        .fold(0.0, f32::max);
    let radius = (radius * 16.0).ceil() / 16.0;
    let up = if direction.cross(Vec3::Y).length_squared() < 0.01 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let rotation = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, -direction, up);
    let center_light = rotation.transform_point3(center);
    let texel = 2.0 * radius / SIZE as f32;
    let snapped_x = (center_light.x / texel).round() * texel;
    let snapped_y = (center_light.y / texel).round() * texel;
    let light_view = Mat4::from_translation(Vec3::new(-snapped_x, -snapped_y, 0.0)) * rotation;
    let mut min_depth = f32::INFINITY;
    let mut max_depth = f32::NEG_INFINITY;
    for corner in corners {
        let depth = -light_view.transform_point3(corner).z;
        min_depth = min_depth.min(depth);
        max_depth = max_depth.max(depth);
    }
    for instance in instances {
        let center = Vec3::new(
            instance.current[3][0],
            instance.current[3][1],
            instance.current[3][2],
        );
        let extent = (0..3)
            .map(|axis| {
                Vec3::new(
                    instance.current[axis][0],
                    instance.current[axis][1],
                    instance.current[axis][2],
                )
                .length()
            })
            .sum::<f32>();
        let depth = -light_view.transform_point3(center).z;
        min_depth = min_depth.min(depth - extent);
        max_depth = max_depth.max(depth + extent);
    }
    min_depth -= 0.5;
    max_depth += 0.5;
    let span = max_depth - min_depth;
    if !radius.is_finite() || radius <= 0.0 || !span.is_finite() || span <= 0.0 {
        return Err(RenderError::InvalidView);
    }
    let standard = glam::camera::rh::proj::directx::orthographic(
        -radius, radius, -radius, radius, min_depth, max_depth,
    );
    let reverse = Mat4::from_cols(
        Vec4::X,
        Vec4::Y,
        Vec4::new(0.0, 0.0, -1.0, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
    );
    let matrix = reverse * standard * light_view;
    if !matrix.to_cols_array().iter().all(|value| value.is_finite()) {
        return Err(RenderError::InvalidView);
    }
    Ok((matrix, span))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn practical_split_and_reverse_light_depth() {
        let near = 0.1_f32;
        let far = 20.0_f32;
        let split = split_depth(near, far);
        assert!((split - 5.732).abs() < 0.001);
        let camera =
            glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let light = Vec3::new(1.0, 0.0, 1.0).normalize();
        let (matrix, _) =
            cascade_matrix(camera, 1.0, 1.0, near, split, light, &[]).expect("finite cascade");
        let receiver = matrix.project_point3(Vec3::ZERO);
        let caster = matrix.project_point3(Vec3::new(1.0, 0.0, 1.0));
        assert!((receiver.x - caster.x).abs() < 1e-5);
        assert!((receiver.y - caster.y).abs() < 1e-5);
        assert!(
            caster.z > receiver.z,
            "upstream caster must win reverse depth"
        );
    }

    #[test]
    fn sub_texel_camera_translation_keeps_shadow_xy_stable() {
        let light = Vec3::new(1.0, 0.0, 1.0).normalize();
        let first =
            glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let shifted = glam::camera::rh::view::look_at_mat4(
            Vec3::new(0.0, 0.0001, 3.0),
            Vec3::new(0.0, 0.0001, 0.0),
            Vec3::Y,
        );
        let (a, _) =
            cascade_matrix(first, 1.0, 1.0, 0.1, 5.732, light, &[]).expect("first cascade");
        let (b, _) =
            cascade_matrix(shifted, 1.0, 1.0, 0.1, 5.732, light, &[]).expect("shifted cascade");
        let point = Vec3::new(0.2, -0.1, 0.0);
        let before = a.project_point3(point);
        let after = b.project_point3(point);
        assert!((before.x - after.x).abs() < 1e-5);
        assert!((before.y - after.y).abs() < 1e-5);
    }
}
