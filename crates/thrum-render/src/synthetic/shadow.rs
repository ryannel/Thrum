//! Two prepared finite reverse-Z Sun cascades and one top-down caster height
//! map for the synthetic perspective view. Receivers beyond the cascades' reach
//! march the height map toward the Sun. One depth array belongs to each target;
//! the prepared pipelines and light matrices belong to its device generation and
//! checked camera sample.

use glam::{Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

use super::view::{Projection, SyntheticView};
use super::{InstanceRecord, RenderError};
use crate::contribution::{SceneContribution, SceneShadowSurface, ShadowBounds};
use crate::graph::{CompiledPass, GraphDepthAttachment, ResourceId};

pub(super) const SIZE: u32 = 1024;
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub(super) const SAMPLE_BYTES: usize = 288;
const CASTER_VIEW_BYTES: usize = 80;
/// The caster view uniform's bytes, for graph ranges.
pub(super) const CASTER_VIEW_RANGE: u64 = CASTER_VIEW_BYTES as u64;
/// Two Sun cascades, then the top-down caster height map.
pub(super) const MAPS: usize = 3;
/// The same layer count for graph ranges and pipeline roles.
pub(super) const LAYERS: u32 = 3;
const _: () = assert!(LAYERS as usize == MAPS);
/// Half-extent of the height map around the camera. At 1024 texels this keeps
/// 32 m terrain texels while covering peaks more than 12 km upstream.
const HORIZON_RADIUS: f32 = 16384.0;
const RECEIVER_REACH: f32 = 512.0;
const BIAS_MIN_WORLD: f32 = 0.006;
const BIAS_MAX_WORLD: f32 = 0.15;
const BIAS_SLOPE_WORLD: f32 = 0.06;
const SUN_ANGULAR_RADIUS: f32 = 0.004_654_21;
// Bound the near-only blocker range by an eight-texel solar penumbra.
// The last cascade represents more distant blockers and their broader support.
const LOCAL_PENUMBRA_TEXELS: f32 = 8.0;

fn split_depth(near: f32, far: f32) -> f32 {
    near.midpoint(far).midpoint((near * far).sqrt())
}

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
}

impl CasterBox {
    pub(super) const UNIT: Self = Self {
        center: [0.0; 3],
        half: [1.0; 3],
    };

    pub(super) fn from_bounds([minimum, maximum]: [[f32; 3]; 2]) -> Self {
        Self {
            center: std::array::from_fn(|axis| minimum[axis].midpoint(maximum[axis])),
            half: std::array::from_fn(|axis| (maximum[axis] - minimum[axis]) * 0.5),
        }
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

pub(super) struct CasterDraw {
    /// The bucket's index, which selects its culled indirect record.
    pub(super) bucket: u32,
    pub(super) member_offset: u32,
    pub(super) indices: std::ops::Range<u32>,
    pub(super) base_vertex: i32,
    pub(super) instances: u32,
}

struct FittedShadow {
    matrices: [Mat4; MAPS],
    samples: [u8; SAMPLE_BYTES],
    partitioned: bool,
}

impl FittedShadow {
    fn contains_caster(&self, instance: &InstanceRecord, caster: CasterBox) -> bool {
        self.matrices.iter().enumerate().all(|(index, matrix)| {
            matrix_contains_caster(*matrix, instance, caster, index == 0 && self.partitioned)
        })
    }

    fn caster_view(&self, index: usize) -> [u8; CASTER_VIEW_BYTES] {
        let mut bytes = [0; CASTER_VIEW_BYTES];
        bytes[..64].copy_from_slice(&matrix_bytes(self.matrices[index]));
        bytes[64..68].copy_from_slice(&u32::from(index == 0 && self.partitioned).to_le_bytes());
        bytes
    }
}

struct CascadeFit {
    matrix: Mat4,
    span: f32,
    upstream_depth: f32,
    excluded_depth: f32,
}

fn matrix_contains_caster(
    matrix: Mat4,
    instance: &InstanceRecord,
    caster: CasterBox,
    clip_upstream: bool,
) -> bool {
    let row = matrix.row(2);
    let (extent, centers) = caster.along(row.truncate(), instance);
    centers.iter().all(|center| {
        let depth = row.truncate().dot(*center) + row.w;
        depth - extent >= -1e-4 && (clip_upstream || depth + extent <= 1.0 + 1e-4)
    })
}

pub(super) struct ShadowPrepared {
    pipeline: wgpu::RenderPipeline,
    pipeline_two_sided: wgpu::RenderPipeline,
    /// For scenes that select meshes: the member list's layout, and culled
    /// and two-sided pipelines that fetch each instance from it.
    members: Option<(wgpu::BindGroupLayout, [wgpu::RenderPipeline; 2])>,
    cascades: [Cascade; MAPS],
    queue: wgpu::Queue,
    view_layout: wgpu::BindGroupLayout,
    inputs: FitInputs,
    fitted: FittedShadow,
    pub(super) sample_buffer: wgpu::Buffer,
}

pub(super) struct ShadowTarget {
    views: [wgpu::TextureView; MAPS],
    pub(super) array_view: wgpu::TextureView,
}

impl ShadowPrepared {
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
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(CASTER_VIEW_RANGE),
                },
                count: None,
            }],
        });
        let cascades = std::array::from_fn(|index| {
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic Sun cascade matrix"),
                contents: &fitted.caster_view(index),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("synthetic Sun caster"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shadow.wgsl").into()),
        });
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
            (member_layout, pipelines)
        });
        let sample_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("synthetic Sun cascade samples"),
            contents: &fitted.samples,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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
        let inputs = FitInputs::new(view, casters, light_direction)?;
        self.fitted = inputs.fit(None)?;
        self.inputs = inputs;
        Ok(())
    }

    /// Upload the fit for this recording, including retries after discarded encoders.
    /// Extreme contributed bounds can overflow the fit. Keep the base shadows in that case.
    pub(super) fn write_recording(&self, bounds: Option<ShadowBounds>) -> bool {
        let cooked = bounds.and_then(|bounds| self.inputs.fit(Some(bounds)).ok());
        let fitted = cooked.as_ref().unwrap_or(&self.fitted);
        for (index, cascade) in self.cascades.iter().enumerate() {
            self.queue
                .write_buffer(&cascade.buffer, 0, &fitted.caster_view(index));
        }
        self.queue
            .write_buffer(&self.sample_buffer, 0, &fitted.samples);
        cooked.is_some()
    }

    pub(super) fn encode_cascade(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &ShadowTarget,
        instances: &wgpu::BindGroup,
        casters: &CasterDraws<'_>,
        generation: u64,
        index: usize,
        compiled: &CompiledPass,
        resource: &ResourceId,
        timings: Option<&mut crate::timing::GpuTimings>,
        contribution: Option<&dyn SceneContribution>,
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
        if let Some(contribution) = contribution {
            contribution.draw_shadow(&mut pass);
        }
        Ok(())
    }
}

impl ShadowTarget {
    pub(super) fn new(texture: &wgpu::Texture) -> Self {
        let views = std::array::from_fn(|index| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("synthetic Sun cascade layer"),
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: u32::try_from(index).expect("three map layers"),
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

fn matrix_bytes(matrix: Mat4) -> [u8; 64] {
    let mut bytes = [0_u8; 64];
    for (index, value) in matrix.to_cols_array().into_iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

struct FitInputs {
    projection: Projection,
    rotation: Mat4,
    caster_depth: Option<(f32, f32)>,
    caster_height: Option<(f32, f32)>,
}

impl FitInputs {
    /// Refuses a box list that does not cover every slot.
    fn new(
        view: &SyntheticView,
        (instances, casters): (&[InstanceRecord], Option<&[CasterBox]>),
        light_direction: Vec3,
    ) -> Result<Self, RenderError> {
        if casters.is_some_and(|casters| casters.len() != instances.len()) {
            return Err(RenderError::SceneCapacity);
        }
        let rotation = light_rotation(light_direction);
        let set = (instances, casters);
        Ok(Self {
            projection: view.projection(),
            rotation,
            caster_depth: caster_depth_bounds(rotation, set),
            caster_height: caster_depth_bounds(light_rotation(Vec3::NEG_Y), set),
        })
    }

    fn fit(&self, bounds: Option<ShadowBounds>) -> Result<FittedShadow, RenderError> {
        let Projection::Perspective {
            view: camera_view,
            fx,
            fy,
            near,
            far,
        } = self.projection
        else {
            return Err(RenderError::InvalidView);
        };
        let limit = near + RECEIVER_REACH;
        let receiver_far = if limit > near { far.min(limit) } else { far };
        let split = if receiver_far < far {
            // A long view spends most of its first map on close receivers.
            let logarithmic = (near * receiver_far).sqrt();
            logarithmic.mul_add(0.875, near.midpoint(receiver_far) * 0.125)
        } else {
            split_depth(near, receiver_far)
        };
        let overlap = (split * 0.05)
            .max(0.05)
            .min((split - near).min(receiver_far - split) * 0.5);
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
        let eye = camera_view.inverse().w_axis.truncate();
        let (heights, height_span) = height_matrix(eye, caster_height)?;
        debug_assert!((heights.row(2).y * height_span - 1.0).abs() < 1e-3);
        let local = cascade_matrix(
            camera_view,
            fx,
            fy,
            near,
            split + overlap,
            rotation,
            caster_depth,
            caster_height,
            false,
        )?;
        // The last map also serves near receivers when their blocker falls
        // outside the near map. Include those receivers in its finite-Sun fit.
        let distant = cascade_matrix(
            camera_view,
            fx,
            fy,
            near,
            receiver_far,
            rotation,
            caster_depth,
            caster_height,
            true,
        )?;
        let matrices = [local.matrix, distant.matrix, heights];
        let partitioned = local.excluded_depth > 0.0;
        let mut samples = [0_u8; SAMPLE_BYTES];
        samples[272..276].copy_from_slice(&(local.excluded_depth / local.span).to_le_bytes());
        let last_depth_at_near_clip =
            1.0 - (local.upstream_depth - distant.upstream_depth) / distant.span;
        samples[276..280].copy_from_slice(&last_depth_at_near_clip.to_le_bytes());
        if let Some((_, max_y)) = caster_height {
            let sun_y = rotation.row(2).y;
            let minimum_sun_y = sun_y * SUN_ANGULAR_RADIUS.cos()
                - (1.0 - sun_y * sun_y).max(0.0).sqrt() * SUN_ANGULAR_RADIUS.sin();
            samples[176..180].copy_from_slice(&max_y.to_le_bytes());
            samples[180..184].copy_from_slice(&minimum_sun_y.to_le_bytes());
        }
        samples[192..256].copy_from_slice(&matrix_bytes(heights));
        if caster_height.is_some() {
            // Reverse-Z top-down depth is affine in world height: d = a*y + b.
            let a = heights.row(2).y;
            let b = heights.row(2).w;
            let uv_per_metre = heights.row(0).truncate().length() * 0.5;
            let texel = 2.0 * HORIZON_RADIUS / SIZE as f32;
            for (offset, value) in [a, b, uv_per_metre, texel].into_iter().enumerate() {
                samples[256 + offset * 4..260 + offset * 4].copy_from_slice(&value.to_le_bytes());
            }
        }
        for (index, CascadeFit { matrix, span, .. }) in [local, distant].into_iter().enumerate() {
            samples[index * 64..(index + 1) * 64].copy_from_slice(&matrix_bytes(matrix));
            let scale = 1.0 / span;
            // The plane correction is exact only within one triangle. Allow half a
            // texel in metres where filtering spans neighboring receiver triangles.
            let half_texel = (matrix.row(0).truncate().length() * SIZE as f32).recip();
            let base = half_texel.clamp(BIAS_MIN_WORLD, BIAS_MAX_WORLD);
            let uv_per_metre = matrix.row(0).truncate().length() * 0.5;
            let penumbra = SUN_ANGULAR_RADIUS.tan() * span * uv_per_metre;
            samples[160 + index * 4..164 + index * 4].copy_from_slice(&penumbra.to_le_bytes());
            samples[168 + index * 4..172 + index * 4]
                .copy_from_slice(&(BIAS_MIN_WORLD * scale).to_le_bytes());
            samples[184 + index * 4..188 + index * 4].copy_from_slice(&scale.to_le_bytes());
            samples[144 + index * 4..148 + index * 4]
                .copy_from_slice(&(base * scale).to_le_bytes());
            samples[152 + index * 4..156 + index * 4]
                .copy_from_slice(&(BIAS_SLOPE_WORLD * scale).to_le_bytes());
        }
        samples[128..132].copy_from_slice(&split.to_le_bytes());
        samples[132..136].copy_from_slice(&overlap.to_le_bytes());
        if receiver_far < far {
            let fade_start = receiver_far - (receiver_far - near) * 0.1;
            samples[136..140].copy_from_slice(&receiver_far.to_le_bytes());
            samples[140..144].copy_from_slice(&fade_start.to_le_bytes());
        }
        Ok(FittedShadow {
            matrices,
            samples,
            partitioned,
        })
    }
}

fn cascade_matrix(
    camera_view: Mat4,
    fx: f32,
    fy: f32,
    near: f32,
    far: f32,
    rotation: Mat4,
    caster_depth: Option<(f32, f32)>,
    caster_height: Option<(f32, f32)>,
    sun_cone: bool,
) -> Result<CascadeFit, RenderError> {
    let camera_world = camera_view.inverse();
    let corners: [Vec3; 8] = std::array::from_fn(|index| {
        let depth = if index < 4 { near } else { far };
        let x = if index & 1 == 0 { -1.0 } else { 1.0 };
        let y = if index & 2 == 0 { -1.0 } else { 1.0 };
        camera_world.transform_point3(Vec3::new(x * depth / fx, y * depth / fy, -depth))
    });
    let center = corners.into_iter().sum::<Vec3>() / 8.0;
    let receiver_radius = corners
        .into_iter()
        .map(|corner| corner.distance(center))
        .fold(0.0, f32::max);
    // A finite Sun's cone reaches outside the central-ray receiver footprint.
    // The last cascade includes the maximum axial gap before snapping. At low Sun
    // that gap reaches kilometres, so the near map stays unpadded for contact
    // resolution and leaves blockers beyond its own support to the last cascade.
    let receiver_max_depth = corners
        .into_iter()
        .map(|corner| -rotation.transform_point3(corner).z)
        .fold(f32::NEG_INFINITY, f32::max);
    let blocker_min_depth = caster_depth.map_or(receiver_max_depth, |(lo, _)| lo);
    let mut maximum_gap = (receiver_max_depth - blocker_min_depth).max(0.0);
    if let Some((_, maximum_y)) = caster_height {
        let sun_y = rotation.row(2).y;
        let low = sun_y * SUN_ANGULAR_RADIUS.cos()
            - (1.0 - sun_y * sun_y).max(0.0).sqrt() * SUN_ANGULAR_RADIUS.sin();
        if low > 1e-6 {
            let receiver_min_y = corners
                .into_iter()
                .map(|p| p.y)
                .fold(f32::INFINITY, f32::min);
            maximum_gap = maximum_gap.min((maximum_y - receiver_min_y).max(0.0) / low);
        }
    }
    let cone_guard = if sun_cone {
        maximum_gap * SUN_ANGULAR_RADIUS.tan()
    } else {
        0.0
    };
    let padded_radius = (receiver_radius + cone_guard) / (1.0 - 4.0 / SIZE as f32);
    let radius = (padded_radius * 16.0).ceil() / 16.0;
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
    let full_min_depth = min_depth.min(blocker_min_depth);
    if let Some((caster_min, caster_max)) = caster_depth {
        let caster_limit = if sun_cone {
            caster_min
        } else {
            let local_gap = LOCAL_PENUMBRA_TEXELS * texel / SUN_ANGULAR_RADIUS.tan();
            caster_min.max(min_depth - local_gap)
        };
        min_depth = min_depth.min(caster_limit);
        max_depth = max_depth.max(caster_max);
    }
    let excluded_depth = (min_depth - full_min_depth).max(0.0);
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
    Ok(CascadeFit {
        matrix,
        span,
        upstream_depth: min_depth,
        excluded_depth,
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

/// Exact affine-box extent along light depth, shared by both cascades.
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
    mod observer;
    mod penumbra;

    use super::*;

    #[test]
    fn only_the_last_cascade_pads_for_distant_finite_sun_blockers() {
        // A Sun 2° up behind kilometres of caster: the cone guard is metres to
        // tens of metres. The near map must keep its receiver-fitted texels.
        let direction = Vec3::new(0.0, 2f32.to_radians().sin(), 2f32.to_radians().cos());
        let view = SyntheticView::perspective(
            [0.0, 2.0, 0.0],
            [0.0, 1.0, -10.0],
            [0.0, 1.0, 0.0],
            0.8,
            16.0 / 9.0,
            0.1,
            2000.0,
        )
        .expect("perspective receiver");
        let inputs = FitInputs::new(&view, (&[], None), direction).expect("no casters");
        let low = ShadowBounds::new([-50.0, -1.0, -50.0], [50.0, 1.0, 50.0]).expect("low caster");
        let tall = ShadowBounds::new([-50.0, -1.0, -3000.0], [50.0, 400.0, 3000.0])
            .expect("tall distant caster");
        let half_extent = |fitted: &FittedShadow, index: usize| {
            fitted.matrices[index].row(0).truncate().length().recip()
        };
        let low = inputs.fit(Some(low)).expect("low fit");
        let tall = inputs.fit(Some(tall)).expect("tall fit");
        assert!((half_extent(&low, 0) - half_extent(&tall, 0)).abs() <= 1e-3);
        assert!(half_extent(&tall, 1) > half_extent(&low, 1) + 10.0);
        let remote_caster = direction * 1000.0;
        assert!(tall.partitioned);
        assert!(tall.matrices[0].project_point3(remote_caster).z > 1.0);
        assert!((0.0..=1.0).contains(&tall.matrices[1].project_point3(remote_caster).z));
        assert!((0.0..=1.0).contains(&tall.matrices[0].project_point3(Vec3::ZERO).z));
    }

    #[test]
    fn fitted_sun_support_and_surface_tolerance_keep_world_units() {
        let direction = Vec3::new(1.0, 1.0, 1.0).normalize();
        let view = SyntheticView::perspective(
            [0.0, 2.0, 3.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            1.0,
            1.0,
            0.1,
            100.0,
        )
        .expect("perspective receiver");
        let inputs = FitInputs::new(&view, (&[], None), direction).expect("no casters");
        let bounds = ShadowBounds::new([-100.0; 3], [100.0, 2000.0, 100.0]).expect("finite caster");
        let fitted = inputs.fit(Some(bounds)).expect("finite fit");
        let value =
            |offset| f32::from_le_bytes(fitted.samples[offset..offset + 4].try_into().unwrap());
        assert!((value(176) - 2000.0).abs() < 1e-5);
        let minimum_y = value(180);
        assert!(minimum_y < direction.y && minimum_y > 0.0);
        let tangent = direction.cross(Vec3::Y).normalize();
        let bitangent = direction.cross(tangent);
        for index in 0..32 {
            let phase = index as f32 * std::f32::consts::TAU / 32.0;
            let sample = direction * SUN_ANGULAR_RADIUS.cos()
                + (tangent * phase.cos() + bitangent * phase.sin()) * SUN_ANGULAR_RADIUS.sin();
            assert!(sample.y >= minimum_y - 1e-7);
        }
        for (index, matrix) in fitted.matrices[..2].iter().enumerate() {
            let span = matrix.row(2).truncate().length().recip();
            assert!((value(168 + index * 4) * span - BIAS_MIN_WORLD).abs() < 1e-6);
            assert!((value(184 + index * 4) * span - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn last_cascade_covers_a_narrow_view_receiver_with_an_excluded_caster() {
        let view = SyntheticView::perspective(
            Vec3::ZERO.to_array(),
            Vec3::NEG_Z.to_array(),
            Vec3::Y.to_array(),
            2.0 * 0.1_f32.atan(),
            1.0,
            0.1,
            100.0,
        )
        .expect("narrow perspective receiver");
        let bounds =
            ShadowBounds::new([59.0, -1.0, -6.0], [61.0, 1.0, -4.0]).expect("upstream caster");
        let fitted = FitInputs::new(&view, (&[], None), Vec3::X)
            .expect("no casters")
            .fit(Some(bounds))
            .expect("partitioned shadow fit");
        let receiver = Vec3::new(0.0, 0.0, -5.0);
        let caster = receiver + 60.0 * Vec3::X;
        let near = fitted.matrices[0];
        let last = fitted.matrices[1];
        assert!(fitted.partitioned);
        assert!(near.project_point3(caster).z > 1.0);
        let far_receiver = last.project_point3(receiver);
        let far_caster = last.project_point3(caster);
        let clip_depth = f32::from_le_bytes(fitted.samples[276..280].try_into().unwrap());
        assert!(far_receiver.truncate().abs().max_element() < 1.0);
        assert!((0.0..clip_depth).contains(&far_receiver.z));
        assert!((clip_depth..=1.0).contains(&far_caster.z));
    }

    #[test]
    fn both_cascades_use_32_bit_float_depth() {
        assert_eq!(FORMAT, wgpu::TextureFormat::Depth32Float);
    }

    #[test]
    fn practical_split_and_reverse_light_depth() {
        let near = 0.1_f32;
        let far = 20.0_f32;
        let split = split_depth(near, far);
        assert!((split - 5.732).abs() < 0.001);
        let camera =
            glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let light = Vec3::new(1.0, 0.0, 1.0).normalize();
        let matrix = cascade_matrix(
            camera,
            1.0,
            1.0,
            near,
            split,
            light_rotation(light),
            None,
            None,
            true,
        )
        .expect("finite cascade")
        .matrix;
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
        let rotation = light_rotation(light);
        let a = cascade_matrix(first, 1.0, 1.0, 0.1, 5.732, rotation, None, None, true)
            .expect("first cascade")
            .matrix;
        let b = cascade_matrix(shifted, 1.0, 1.0, 0.1, 5.732, rotation, None, None, true)
            .expect("shifted cascade")
            .matrix;
        let point = Vec3::new(0.2, -0.1, 0.0);
        let before = a.project_point3(point);
        let after = b.project_point3(point);
        assert!((before.x - after.x).abs() < 1e-5);
        assert!((before.y - after.y).abs() < 1e-5);
    }

    #[test]
    fn shared_caster_bounds_enclose_every_affine_cube_corner() {
        let mut instance = InstanceRecord::identity(0, 0);
        instance.current[0] = [1.3, 0.4, 0.2, 0.0];
        instance.current[1] = [-0.2, 0.8, 0.5, 0.0];
        instance.current[2] = [0.1, -0.3, 1.1, 0.0];
        instance.current[3] = [2.0, -1.0, 4.0, 1.0];
        let rotation = light_rotation(Vec3::new(0.3, -0.8, 0.6).normalize());
        let (low, high) =
            caster_depth_bounds(rotation, (&[instance.clone()], None)).expect("one caster");
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
        let (low, high) =
            caster_depth_bounds(rotation, (&[instance.clone()], None)).expect("one caster");
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
