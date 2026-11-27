//! Camera-centred reverse-Z Sun cascades and one top-down caster height
//! map for the synthetic perspective view. Receivers beyond the cascades' reach
//! march the height map toward the Sun. One depth array belongs to each target;
//! the prepared pipelines and light matrices belong to its device generation and
//! checked camera sample.

use crate::binding;
use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::view::SyntheticView;
use super::{InstanceRecord, RenderError};
use crate::contribution::{
    SceneContribution, SceneShadowSurface, SecondaryShadowCascade, ShadowBounds,
};
use crate::graph::{CompiledPass, GraphDepthAttachment, ResourceId};
use crate::records::{ShadowCascades, ShadowView};
use crate::shader_constants::{SHADOW_HEIGHT_LAYER, SUN_ANGULAR_RADIUS, SUN_CASCADE_COUNT};

pub(super) const SIZE: u32 = 1024;
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const SAMPLE_BYTES: usize = size_of::<ShadowCascades>();
const CASTER_VIEW_BYTES: usize = size_of::<ShadowView>();
/// The caster view uniform's bytes, for graph ranges.
pub(super) const CASTER_VIEW_RANGE: u64 = CASTER_VIEW_BYTES as u64;
/// Sun cascades, then the top-down caster height map.
pub(super) const MAPS: usize = LAYERS as usize;
pub(super) const HEIGHT_LAYER: usize = SHADOW_HEIGHT_LAYER as usize;
/// The same layer count for graph ranges and pipeline roles.
pub(super) use crate::shader_constants::SHADOW_MAP_LAYERS as LAYERS;
/// Half-extent of the height map around the camera. At 1024 texels this keeps
/// 32 m terrain texels while covering peaks more than 12 km upstream.
const HORIZON_RADIUS: f32 = 16384.0;
const RECEIVER_REACH: f32 = 2048.0;
const CASCADE_GUARD_TEXELS: f32 = 16.0;
const CASCADE_SNAP_TEXELS: f32 = 0.5;
const MINIMUM_DEPTH_BIAS: f32 = 4e-7;
pub(super) mod cast_width;
mod secondary;

const CAST_WIDTH_TEXELS: f32 = 1.5;

#[derive(Clone)]
struct Cascade {
    buffer: wgpu::Buffer,
    view_group: wgpu::BindGroup,
}

/// A caster's local bounding box: the unit cube for legacy scenes, or the
/// selected arena mesh's bounds. Fits and containment transform it by the
/// instance basis, so every Sun cascade and the height layer see the caster's
/// actual extent, on screen or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct CasterBox {
    pub(super) center: [f32; 3],
    pub(super) half: [f32; 3],
    pub(super) cast_width: f32,
}

impl CasterBox {
    pub(super) const UNIT: Self = Self {
        center: [0.0; 3],
        half: [1.0; 3],
        cast_width: 2.0,
    };

    pub(super) fn from_mesh(row: &super::mesh_arena::MeshRow) -> Self {
        let [minimum, maximum] = row.bounds;
        Self {
            center: std::array::from_fn(|axis| minimum[axis].midpoint(maximum[axis])),
            half: std::array::from_fn(|axis| (maximum[axis] - minimum[axis]) * 0.5),
            cast_width: row.cast_width,
        }
    }

    pub(super) fn world_cast_width(local: f32, instance: &InstanceRecord) -> f32 {
        let scale = instance.current[..3]
            .iter()
            .map(|column| Vec4::from_array(*column).truncate().length())
            .fold(f32::INFINITY, f32::min);
        local * scale
    }

    /// The box's half-extent along `axis` and its centre at each translation
    /// endpoint, both in world units. The unit box reproduces the legacy cube
    /// arithmetic exactly.
    fn along(self, axis: Vec3, instance: &InstanceRecord) -> (f32, [Vec3; 2]) {
        let columns = std::array::from_fn::<_, 3, _>(|index| {
            let column = instance.current[index];
            Vec3::new(column[0], column[1], column[2])
        });
        let extent = (0..3)
            .map(|index| self.half[index] * axis.dot(columns[index]).abs())
            .sum::<f32>();
        let offset = (0..3).fold(Vec3::ZERO, |sum, index| {
            sum + columns[index] * self.center[index]
        });
        let centers = [instance.previous[3], instance.current[3]]
            .map(|center| Vec3::new(center[0], center[1], center[2]) + offset);
        (extent, centers)
    }
}

/// The caster set for one cascade: the arena's index buffer and either the
/// legacy cube over physical rows, or one draw per bucket over its members.
/// Membership is every admitted instance, never the camera-visible list.
pub(super) struct CasterDraws<'a> {
    pub(super) indices: &'a wgpu::Buffer,
    /// The existing caster facing rule: no culling while any instance is
    /// mirrored or degenerate.
    pub(super) two_sided: bool,
    /// The bucket route's member list, bound per draw at its segment offset.
    pub(super) members: Option<&'a wgpu::BindGroup>,
    /// Sun caster culling: this pass's culled list, bound like the member
    /// list, and its per-bucket indirect records.
    pub(super) culled: Option<(&'a wgpu::BindGroup, &'a wgpu::Buffer)>,
    pub(super) draws: &'a [CasterDraw],
}

#[derive(Clone)]
pub(super) struct CasterDraw {
    /// The bucket's index, which selects its culled indirect record.
    pub(super) bucket: u32,
    pub(super) member_offset: u32,
    pub(super) indices: std::ops::Range<u32>,
    pub(super) base_vertex: i32,
    pub(super) instances: u32,
}

#[derive(Clone, Copy)]
pub(super) enum CasterContribution<'a> {
    Main(&'a dyn SceneContribution),
    Secondary(&'a dyn SceneContribution, usize, SecondaryShadowCascade),
}

#[derive(Clone)]
struct FittedShadow {
    count: u32,
    matrices: [Mat4; MAPS],
    samples: ShadowCascades,
}

impl FittedShadow {
    fn contains_caster(&self, instance: &InstanceRecord, caster: CasterBox) -> bool {
        self.matrices[..self.count as usize]
            .iter()
            .chain(std::iter::once(&self.matrices[HEIGHT_LAYER]))
            .all(|matrix| matrix_contains_caster(*matrix, instance, caster))
    }

    fn caster_view(&self, index: usize) -> ShadowView {
        ShadowView {
            light_view_proj: self.matrices[index].to_cols_array_2d(),
            minimum_cast_width: [
                self.samples
                    .cascade_metrics
                    .get(index)
                    .map_or(0.0, |metrics| CAST_WIDTH_TEXELS * metrics[1]),
                0.0,
                0.0,
                0.0,
            ],
        }
    }
}

struct CascadeFit {
    matrix: Mat4,
    span: f32,
    texel: f32,
}

fn matrix_contains_caster(matrix: Mat4, instance: &InstanceRecord, caster: CasterBox) -> bool {
    let row = matrix.row(2);
    let (extent, centers) = caster.along(row.truncate(), instance);
    centers.iter().all(|center| {
        let depth = row.truncate().dot(*center) + row.w;
        depth - extent >= -1e-4 && depth + extent <= 1.0 + 1e-4
    })
}

#[derive(Clone)]
pub(super) struct ShadowPrepared {
    pipeline: wgpu::RenderPipeline,
    pipeline_two_sided: wgpu::RenderPipeline,
    /// For scenes that select meshes: the member list's layout, and culled
    /// and two-sided pipelines that fetch each instance from it.
    members: Option<(wgpu::BindGroupLayout, [wgpu::RenderPipeline; 2])>,
    cascades: [Cascade; MAPS],
    queue: wgpu::Queue,
    view_layout: crate::runtime::PipelineInput<wgpu::BindGroupLayout>,
    inputs: FitInputs,
    fitted: FittedShadow,
    pub(super) sample_buffer: wgpu::Buffer,
}

#[derive(Clone)]
pub(super) struct ShadowTarget<const MAP_COUNT: usize = MAPS> {
    views: [wgpu::TextureView; MAP_COUNT],
    pub(super) array_view: wgpu::TextureView,
}

impl ShadowPrepared {
    pub(super) fn count(&self) -> u32 {
        self.inputs
            .profile
            .map_or(SUN_CASCADE_COUNT, |(count, _)| count)
    }

    pub(super) fn contains_caster(&self, instance: &InstanceRecord, caster: CasterBox) -> bool {
        self.fitted.contains_caster(instance, caster)
    }

    /// `casters` holds each slot's box when the scene selects meshes, and
    /// prepares the member pipelines; `None` keeps the unit cube.
    pub(super) fn new(
        device: &crate::runtime::DeviceSite,
        queue: &wgpu::Queue,
        view: &SyntheticView,
        (instances, casters): (&[InstanceRecord], Option<&[CasterBox]>),
        light_direction: Vec3,
        instance_layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, RenderError> {
        let inputs = FitInputs::new(view, (instances, casters), light_direction)?;
        let fitted = inputs.fit(None)?;
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("synthetic Sun cascade view"),
            entries: &[binding::uniform(
                0,
                wgpu::ShaderStages::VERTEX
                    | super::forward::mesh_stage_visibility(device.features()),
                CASTER_VIEW_RANGE,
            )],
        });
        let cascades = std::array::from_fn(|index| {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic Sun cascade matrix"),
                contents: bytemuck::bytes_of(&fitted.caster_view(index)),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | super::draw_witness_usage(),
            });
            let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("synthetic Sun cascade view"),
                layout: &view_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            Cascade { buffer, view_group }
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("synthetic Sun shadow"),
            bind_group_layouts: &[Some(&view_layout), Some(instance_layout)],
            immediate_size: 0,
        });
        let shader = crate::shaders::Shader::Shadow.compile(device, "synthetic Sun caster");
        let create = |cull_mode, layout: &wgpu::PipelineLayout, entry_point| {
            device.render(&wgpu::RenderPipelineDescriptor {
                label: Some("synthetic Sun caster"),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(entry_point),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState {
                    cull_mode,
                    ..Default::default()
                },
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
            })
        };
        let pipeline = create(Some(wgpu::Face::Back), &layout, "vertex");
        let pipeline_two_sided = create(None, &layout, "vertex");
        let members = casters.is_some().then(|| {
            let member_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("synthetic Sun caster members"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(4),
                    },
                    count: None,
                }],
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("synthetic Sun bucket shadow"),
                bind_group_layouts: &[
                    Some(&view_layout),
                    Some(instance_layout),
                    Some(&member_layout),
                ],
                immediate_size: 0,
            });
            let pipelines =
                [Some(wgpu::Face::Back), None].map(|cull| create(cull, &layout, "member_vertex"));
            (member_layout.into_raw(), pipelines)
        });
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic Sun cascade samples"),
            contents: bytemuck::bytes_of(&fitted.samples),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | super::draw_witness_usage(),
        });
        Ok(Self {
            pipeline,
            pipeline_two_sided,
            members,
            cascades,
            queue: queue.clone(),
            view_layout,
            inputs,
            fitted,
            sample_buffer,
        })
    }

    pub(super) fn surface(&self) -> SceneShadowSurface<'_> {
        SceneShadowSurface {
            view_layout: &self.view_layout,
            depth_format: FORMAT,
        }
    }

    /// The member list's layout for the bucket route's caster bind group,
    /// when the scene selects meshes.
    pub(super) fn member_layout(&self) -> Option<&wgpu::BindGroupLayout> {
        self.members.as_ref().map(|(layout, _)| layout)
    }

    /// Each pass's caster view (light matrix and upstream clip flag), the
    /// bytes its draw reads, for the caster cull to test against.
    pub(super) fn caster_views(&self) -> Vec<&wgpu::Buffer> {
        self.cascades
            .iter()
            .map(|cascade| &cascade.buffer)
            .collect()
    }

    pub(super) fn update(
        &mut self,
        view: &SyntheticView,
        casters: (&[InstanceRecord], Option<&[CasterBox]>),
        light_direction: Vec3,
    ) -> Result<(), RenderError> {
        let mut inputs = FitInputs::new(view, casters, light_direction)?;
        inputs.profile = self.inputs.profile;
        self.fitted = inputs.fit(None)?;
        self.inputs = inputs;
        Ok(())
    }

    /// Upload the fit for this recording, including retries after discarded encoders.
    /// Extreme contributed bounds can overflow the fit. Keep the base shadows in that case.
    pub(super) fn write_recording(&self, bounds: Option<ShadowBounds>, cascades: bool) -> bool {
        let cooked = bounds.and_then(|bounds| self.inputs.fit(Some(bounds)).ok());
        let fitted = cooked.as_ref().unwrap_or(&self.fitted);
        let count = if cascades { self.count() as usize } else { 0 };
        for index in (0..count).chain(std::iter::once(HEIGHT_LAYER)) {
            let cascade = &self.cascades[index];
            self.queue.write_buffer(
                &cascade.buffer,
                0,
                bytemuck::bytes_of(&fitted.caster_view(index)),
            );
        }
        self.queue
            .write_buffer(&self.sample_buffer, 0, bytemuck::bytes_of(&fitted.samples));
        cooked.is_some()
    }

    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    pub(super) fn inspection_volumes(
        &self,
        bounds: Option<ShadowBounds>,
    ) -> [SecondaryShadowCascade; MAPS] {
        let cooked = bounds.and_then(|bounds| self.inputs.fit(Some(bounds)).ok());
        cooked
            .as_ref()
            .unwrap_or(&self.fitted)
            .matrices
            .map(SecondaryShadowCascade::new)
    }

    pub(super) fn encode_cascade<const MAP_COUNT: usize>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ShadowTarget<MAP_COUNT>,
        instances: &wgpu::BindGroup,
        casters: &CasterDraws<'_>,
        generation: u64,
        index: usize,
        compiled: &CompiledPass,
        resource: &ResourceId,
        timings: Option<&mut crate::timing::GpuTimings>,
        contribution: Option<CasterContribution<'_>>,
    ) -> Result<(), RenderError> {
        let (cascade, view) = self
            .cascades
            .get(index)
            .zip(target.views.get(index))
            .ok_or(RenderError::StaleTarget)?;
        let mut pass = compiled
            .begin_render(
                encoder,
                [],
                Some(GraphDepthAttachment {
                    resource,
                    range: u64::try_from(index).expect("three maps")
                        ..u64::try_from(index + 1).expect("three maps"),
                    view,
                    clear: 0.0,
                }),
                generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        let pipeline = match (casters.members, &self.members) {
            (None, _) if casters.two_sided => &self.pipeline_two_sided,
            (None, _) => &self.pipeline,
            (Some(_), Some((_, [culled, two_sided]))) => {
                if casters.two_sided {
                    two_sided
                } else {
                    culled
                }
            }
            (Some(_), None) => return Err(RenderError::GraphPassMismatch),
        };
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &cascade.view_group, &[]);
        pass.set_bind_group(1, instances, &[]);
        pass.set_index_buffer(casters.indices.slice(..), wgpu::IndexFormat::Uint32);
        for draw in casters.draws {
            if let Some((list, args)) = casters.culled {
                pass.set_bind_group(2, list, &[draw.member_offset]);
                pass.draw_indexed_indirect(
                    args,
                    u64::from(draw.bucket) * crate::visibility::ARGS_BYTES,
                );
                continue;
            }
            if let Some(members) = casters.members {
                pass.set_bind_group(2, members, &[draw.member_offset]);
            }
            pass.draw_indexed(draw.indices.clone(), draw.base_vertex, 0..draw.instances);
        }
        match contribution {
            Some(CasterContribution::Main(contribution)) => contribution.draw_shadow(&mut pass),
            Some(CasterContribution::Secondary(contribution, index, cascade)) => {
                contribution.draw_secondary_shadow(index, cascade, &mut pass);
            }
            None => (),
        }
        Ok(())
    }
}

impl<const MAP_COUNT: usize> ShadowTarget<MAP_COUNT> {
    pub(super) fn new(texture: &wgpu::Texture) -> Self {
        let views = std::array::from_fn(|index| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("synthetic Sun cascade layer"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: u32::try_from(index).expect("Sun map layer index"),
                array_layer_count: Some(1),
                ..Default::default()
            })
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("synthetic Sun cascade array"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        Self { views, array_view }
    }
}

#[derive(Clone)]
struct FitInputs {
    camera: Vec3,
    rotation: Mat4,
    caster_depth: Option<(f32, f32)>,
    caster_height: Option<(f32, f32)>,
    profile: Option<(u32, f32)>,
}

impl FitInputs {
    /// Refuses a box list that does not cover every slot.
    fn new(
        view: &SyntheticView,
        (instances, casters): (&[InstanceRecord], Option<&[CasterBox]>),
        light_direction: Vec3,
    ) -> Result<Self, RenderError> {
        if casters.is_some_and(|casters| casters.len() != instances.len()) {
            return Err(RenderError::ShadowCasterCount);
        }
        let rotation = light_rotation(light_direction);
        let set = (instances, casters);
        Ok(Self {
            camera: Vec3::from_array(view.camera_position()),
            rotation,
            caster_depth: caster_depth_bounds(rotation, set),
            caster_height: caster_depth_bounds(light_rotation(Vec3::NEG_Y), set),
            profile: view.sun_shadow_profile(),
        })
    }

    fn fit(&self, bounds: Option<ShadowBounds>) -> Result<FittedShadow, RenderError> {
        let (count, reach) = self.profile.unwrap_or((SUN_CASCADE_COUNT, RECEIVER_REACH));
        let rotation = self.rotation;
        let caster_depth = match (self.caster_depth, bounds) {
            (depth, None) => depth,
            (depth, Some(bounds)) => {
                let (lo, hi) = bounds.depth_interval(depth_axis(rotation).to_array());
                if !lo.is_finite() || !hi.is_finite() {
                    return Err(RenderError::InvalidView);
                }
                Some(depth.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))))
            }
        };
        let caster_height = bounds.map_or(self.caster_height, |bounds| {
            let height = bounds.depth_interval(Vec3::Y.to_array());
            Some(
                self.caster_height
                    .map_or(height, |(lo, hi)| (lo.min(height.0), hi.max(height.1))),
            )
        });
        let (heights, height_span) = height_matrix(self.camera, caster_height)?;
        debug_assert!((heights.row(2).y * height_span - 1.0).abs() < 1e-3);
        let mut matrices = [Mat4::IDENTITY; MAPS];
        matrices[HEIGHT_LAYER] = heights;
        let mut samples = ShadowCascades {
            profile: [count as f32, reach, reach * 0.6, 0.0],
            height_view_proj: heights.to_cols_array_2d(),
            ..bytemuck::Zeroable::zeroed()
        };
        for (index, matrix) in matrices.iter_mut().enumerate().take(count as usize) {
            let radius = reach / (1u32 << (count as usize - index - 1)) as f32;
            let cascade = cascade_matrix(self.camera, radius, rotation, caster_depth)?;
            *matrix = cascade.matrix;
            samples.light_view_proj[index] = cascade.matrix.to_cols_array_2d();
            samples.cascade_metrics[index] = [
                radius,
                cascade.texel,
                cascade.span.recip(),
                (0.25 * cascade.texel / cascade.span).max(MINIMUM_DEPTH_BIAS),
            ];
        }
        if let Some((_, max_y)) = caster_height {
            let sun_y = rotation.row(2).y;
            let minimum_sun_y = sun_y * SUN_ANGULAR_RADIUS.cos()
                - (1.0 - sun_y * sun_y).max(0.0).sqrt() * SUN_ANGULAR_RADIUS.sin();
            samples.height_support[0] = max_y;
            samples.height_support[1] = minimum_sun_y;
            // Reverse-Z top-down depth is affine in world height: d = a*y + b.
            let a = heights.row(2).y;
            let b = heights.row(2).w;
            let uv_per_metre = heights.row(0).truncate().length() * 0.5;
            let texel = 2.0 * HORIZON_RADIUS / SIZE as f32;
            samples.horizon = [a, b, uv_per_metre, texel];
        }
        Ok(FittedShadow {
            count,
            matrices,
            samples,
        })
    }
}

fn cascade_matrix(
    eye: Vec3,
    radius: f32,
    rotation: Mat4,
    caster_depth: Option<(f32, f32)>,
) -> Result<CascadeFit, RenderError> {
    let half_width =
        radius / (1.0 - 2.0 * (CASCADE_GUARD_TEXELS + CASCADE_SNAP_TEXELS) / SIZE as f32);
    let center = rotation.transform_point3(eye);
    let texel = 2.0 * half_width / SIZE as f32;
    let x = (center.x / texel).round() * texel;
    let y = (center.y / texel).round() * texel;
    let light_view = Mat4::from_translation(Vec3::new(-x, -y, 0.0)) * rotation;
    let (caster_min, caster_max) = caster_depth.unwrap_or((-center.z - radius, -center.z + radius));
    let min_depth = (-center.z - radius).min(caster_min) - 0.5;
    let max_depth = (-center.z + radius).max(caster_max) + 0.5;
    let span = max_depth - min_depth;
    if !half_width.is_finite()
        || !texel.is_finite()
        || texel <= 0.0
        || !span.is_finite()
        || span <= 0.0
    {
        return Err(RenderError::InvalidView);
    }
    let matrix = Mat4::from_cols(
        Vec4::X / half_width,
        Vec4::Y / half_width,
        Vec4::Z / span,
        Vec4::new(0.0, 0.0, max_depth / span, 1.0),
    ) * light_view;
    if !matrix.is_finite() {
        return Err(RenderError::InvalidView);
    }
    Ok(CascadeFit {
        matrix,
        span,
        texel,
    })
}

/// Top-down reverse-Z orthographic map of the highest caster surface around the
/// camera, snapped to whole texels so a moving camera keeps stable heights.
fn height_matrix(eye: Vec3, caster_height: Option<(f32, f32)>) -> Result<(Mat4, f32), RenderError> {
    let rotation = light_rotation(Vec3::Y);
    let (low, high) = caster_height.unwrap_or((0.0, 1.0));
    let texel = 2.0 * HORIZON_RADIUS / SIZE as f32;
    let center = rotation.transform_point3(eye);
    let snapped_x = (center.x / texel).round() * texel;
    let snapped_y = (center.y / texel).round() * texel;
    let light_view = Mat4::from_translation(Vec3::new(-snapped_x, -snapped_y, 0.0)) * rotation;
    let min_depth = -high - 1.0;
    let max_depth = -low + 1.0;
    let span = max_depth - min_depth;
    if !span.is_finite() || span <= 0.0 {
        return Err(RenderError::InvalidView);
    }
    let standard = glam::camera::rh::proj::directx::orthographic(
        -HORIZON_RADIUS,
        HORIZON_RADIUS,
        -HORIZON_RADIUS,
        HORIZON_RADIUS,
        min_depth,
        max_depth,
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

fn light_rotation(direction: Vec3) -> Mat4 {
    let up = if direction.cross(Vec3::Y).length_squared() < 0.01 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    glam::camera::rh::view::look_at_mat4(Vec3::ZERO, -direction, up)
}

fn depth_axis(rotation: Mat4) -> Vec3 {
    -rotation.row(2).truncate()
}

/// Exact affine-box extent along light depth, shared by all cascades.
/// XY texel snapping does not change this depth axis. Each caster's box is
/// its mesh bounds, or the unit cube when the scene selects no meshes.
fn caster_depth_bounds(
    rotation: Mat4,
    (instances, casters): (&[InstanceRecord], Option<&[CasterBox]>),
) -> Option<(f32, f32)> {
    if instances.is_empty() {
        return None;
    }
    let depth_axis = depth_axis(rotation);
    let offset = -rotation.w_axis.z;
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    for (slot, instance) in instances.iter().enumerate() {
        let caster = casters.map_or(CasterBox::UNIT, |casters| casters[slot]);
        let (extent, centers) = caster.along(depth_axis, instance);
        for center in centers {
            let depth = depth_axis.dot(center) + offset;
            minimum = minimum.min(depth - extent);
            maximum = maximum.max(depth + extent);
        }
    }
    Some((minimum, maximum))
}

#[cfg(test)]
mod tests {
    mod disk;
    mod horizon;
    mod nested;
    mod observer;
    mod penumbra;
    mod scene_view_anchor;

    use super::*;

    mod layout;

    #[test]
    fn sun_cascades_use_reverse_32_bit_float_depth() {
        assert_eq!(FORMAT, wgpu::TextureFormat::Depth32Float);
        let direction = Vec3::new(1.0, 0.0, 1.0).normalize();
        let matrix = cascade_matrix(Vec3::ZERO, 8.0, light_rotation(direction), None)
            .unwrap()
            .matrix;
        let receiver = matrix.project_point3(Vec3::ZERO);
        let caster = matrix.project_point3(direction);
        assert!((receiver.truncate() - caster.truncate()).length() < 1e-5);
        assert!(caster.z > receiver.z);
    }

    #[test]
    fn shared_caster_bounds_enclose_every_affine_cube_corner() {
        let mut instance = InstanceRecord::identity(0, 0);
        instance.current[0] = [1.3, 0.4, 0.2, 0.0];
        instance.current[1] = [-0.2, 0.8, 0.5, 0.0];
        instance.current[2] = [0.1, -0.3, 1.1, 0.0];
        instance.current[3] = [2.0, -1.0, 4.0, 1.0];
        let rotation = light_rotation(Vec3::new(0.3, -0.8, 0.6).normalize());
        let (low, high) = caster_depth_bounds(rotation, (&[instance], None)).expect("one caster");
        for bits in 0..8 {
            let local = Vec3::new(
                if bits & 1 == 0 { -1.0 } else { 1.0 },
                if bits & 2 == 0 { -1.0 } else { 1.0 },
                if bits & 4 == 0 { -1.0 } else { 1.0 },
            );
            let transform = Mat4::from_cols_array_2d(&instance.current);
            let depth = -rotation
                .transform_point3(transform.transform_point3(local))
                .z;
            assert!(depth >= low - 1e-5 && depth <= high + 1e-5);
        }
    }

    #[test]
    fn caster_bounds_use_rendered_basis_at_both_translation_endpoints() {
        let mut instance = InstanceRecord::identity(0, 0);
        instance.current[0][0] = 2.0;
        instance.previous[0][0] = 0.1;
        instance.previous[3][0] = 8.0;
        let rotation = light_rotation(Vec3::new(0.3, -0.8, 0.6).normalize());
        let (low, high) = caster_depth_bounds(rotation, (&[instance], None)).expect("one caster");
        for center in [instance.previous[3], instance.current[3]] {
            let mut model = instance.current;
            model[3] = center;
            let transform = Mat4::from_cols_array_2d(&model);
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [-1.0, 1.0] {
                        let depth = -rotation
                            .transform_point3(transform.transform_point3(Vec3::new(x, y, z)))
                            .z;
                        assert!(depth >= low - 1e-5 && depth <= high + 1e-5);
                    }
                }
            }
        }
    }
}
