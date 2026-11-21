//! The opted-in forward mesh pipeline and the limits on each mesh's storage and draw.

use thrum_render::binding;
use thrum_render::contribution::{SceneShadowSurface, SceneSurface};
use thrum_render::runtime::DeviceSite;

use crate::material::MeshSurface;

const GROUP: u32 = 4;
const VIEW_BINDING: u32 = 9;
const RUN: u32 = 64;
const STAGES: wgpu::ShaderStages = wgpu::ShaderStages::TASK.union(wgpu::ShaderStages::MESH);
const NO_REJECTION_REPORT: &str = "fn cooked_report_rejection(_reason: u32) {}";
#[cfg(feature = "draw-probe")]
const CULL_OUTCOMES: u64 = 8; // Kept, six clip planes, normal cone.
#[cfg(feature = "draw-probe")]
const CULL_WITNESS_BYTES: u64 = CULL_OUTCOMES * size_of::<u32>() as u64;

pub(crate) fn prepare(
    surface: &SceneSurface<'_>,
    target: &MeshSurface,
    source: &str,
    config: &crate::CookedConfig,
) -> Result<Prepared, String> {
    use std::future::Future as _;
    let scopes = [
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Internal,
        wgpu::ErrorFilter::Validation,
    ]
    .map(|filter| surface.device.push_error_scope(filter));
    let layout = Layout::new(surface, config);
    #[cfg(feature = "draw-probe")]
    let entry = if config.fail_mesh_pipeline {
        "missing_mesh_entry"
    } else {
        "cooked_mesh"
    };
    #[cfg(not(feature = "draw-probe"))]
    let entry = "cooked_mesh";
    let pipeline = layout.compile(surface.device, target, source, entry);
    let shadow = surface
        .shadow
        .as_ref()
        .map(|shadow| layout.compile_shadow(surface.device, shadow, config));
    // Native Metal validates synchronously. Pop in stack order even when a scope has failed.
    let mut failure = None;
    for scope in scopes.into_iter().rev() {
        let mut pop = std::pin::pin!(scope.pop());
        let result = pop
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
        let error = match result {
            std::task::Poll::Ready(None) => continue,
            std::task::Poll::Ready(Some(error)) => format!("mesh-stage pipeline: {error}"),
            std::task::Poll::Pending => "mesh-stage pipeline validation did not complete".into(),
        };
        failure.get_or_insert(error);
    }
    failure.map_or(
        Ok(Prepared {
            layout,
            forward: pipeline,
            shadow,
        }),
        Err,
    )
}

pub(crate) struct Prepared {
    pub(crate) layout: Layout,
    pub(crate) forward: wgpu::RenderPipeline,
    pub(crate) shadow: Option<wgpu::RenderPipeline>,
}

pub(crate) fn groups(meshlets: u32) -> u32 {
    meshlets.div_ceil(RUN)
}

pub(crate) fn exclusion(
    mesh: &thrum_asset::MeshProduct,
    limits: &wgpu::Limits,
) -> Option<&'static str> {
    let streams = [
        size_of_val(mesh.positions()),
        size_of_val(mesh.attributes()),
        size_of_val(mesh.meshlet_bounds()),
        size_of_val(mesh.meshlets()),
        size_of_val(mesh.meshlet_vertices()),
        size_of_val(mesh.meshlet_triangles()),
    ];
    let largest = streams.into_iter().max().unwrap_or(0) as u64;
    let largest = largest
        .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT)
        .max(16);
    if largest > limits.max_buffer_size {
        return Some("max_buffer_size");
    }
    if largest > limits.max_storage_buffer_binding_size {
        return Some("max_storage_buffer_binding_size");
    }
    let groups = groups(u32::try_from(mesh.meshlets().len()).unwrap_or(u32::MAX));
    if groups > limits.max_task_workgroup_total_count {
        return Some("max_task_workgroup_total_count");
    }
    if groups > limits.max_task_workgroups_per_dimension {
        return Some("max_task_workgroups_per_dimension");
    }
    if limits
        .max_mesh_workgroup_total_count
        .min(limits.max_mesh_workgroups_per_dimension)
        < RUN
    {
        return Some("mesh workgroups per object workgroup");
    }
    if limits.max_task_payload_size < RUN * 4 {
        return Some("max_task_payload_size");
    }
    if mesh.meshlets().iter().any(|m| {
        m.vertex_count > limits.max_mesh_output_vertices.min(64)
            || m.triangle_count > limits.max_mesh_output_primitives.min(124)
    }) {
        return Some("mesh stage output capacity");
    }
    None
}

#[derive(Clone)]
pub(crate) struct Layout {
    group: wgpu::BindGroupLayout,
    pipeline: wgpu::PipelineLayout,
    view: wgpu::Buffer,
    view_size: Option<wgpu::BufferSize>,
    #[cfg(feature = "draw-probe")]
    rejections: Option<wgpu::Buffer>,
    #[cfg(feature = "draw-probe")]
    shadow_rejections: Option<wgpu::Buffer>,
}

impl Layout {
    fn new(surface: &SceneSurface<'_>, config: &crate::CookedConfig) -> Self {
        let stream_entries: [_; 7] = std::array::from_fn(|i| {
            let binding = u32::try_from(i).expect("mesh group binding fits u32");
            match i {
                0 => binding::uniform(binding, STAGES, 0),
                1 => binding::storage(binding, wgpu::ShaderStages::TASK, true, 0),
                _ => binding::storage(binding, wgpu::ShaderStages::MESH, true, 0),
            }
        });
        // wgpu 30 Metal misindexes buffers in groups shared by vertex,
        // fragment, compute and mesh stages. Bind the same authoritative view
        // range through the stage-only placement group, with no extra upload.
        let mut default_entries = stream_entries.to_vec();
        default_entries.push(wgpu::BindGroupLayoutEntry {
            binding: VIEW_BINDING,
            visibility: STAGES,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: surface.mesh_view_uniform.size,
            },
            count: None,
        });
        #[cfg(feature = "draw-probe")]
        let (entries, rejections, shadow_rejections) =
            rejection_layout(surface, config, &default_entries);
        #[cfg(not(feature = "draw-probe"))]
        let entries = {
            let _ = config;
            default_entries
        };
        let group = surface
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("cooked mesh stages"),
                entries: &entries,
            });
        let pipeline = surface
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("cooked mesh stages"),
                bind_group_layouts: &[
                    Some(surface.view_layout),
                    Some(surface.light_layout),
                    Some(surface.material_layout),
                    None,
                    Some(&group),
                ],
                immediate_size: 0,
            });
        Self {
            group,
            pipeline,
            view: surface.mesh_view_uniform.buffer.clone(),
            view_size: surface.mesh_view_uniform.size,
            #[cfg(feature = "draw-probe")]
            rejections,
            #[cfg(feature = "draw-probe")]
            shadow_rejections,
        }
    }

    pub(crate) fn compile(
        &self,
        device: &DeviceSite,
        target: &MeshSurface,
        source: &str,
        mesh_entry: &str,
    ) -> wgpu::RenderPipeline {
        let report = NO_REJECTION_REPORT;
        #[cfg(feature = "draw-probe")]
        let report = if self.rejections.is_some() {
            &format!(
                "@group(4) @binding(7) var<storage, read_write> counts: array<atomic<u32>, {CULL_OUTCOMES}>;
                 fn cooked_report_rejection(reason: u32) {{ atomicAdd(&counts[reason], 1u); }}"
            )
        } else {
            report
        };
        // ViewRecord belongs to the composed renderer shader. The alias
        // uses the renderer's existing uniform, without a cooked upload ABI.
        let shader = format!(
            "enable wgpu_mesh_shader;\n{source}
             @group({GROUP}) @binding({VIEW_BINDING}) var<uniform> mesh_view: ViewRecord;\n{}\n{}\n{}\n{report}",
            include_str!("meshlet_cull.wgsl"),
            include_str!("mesh_stage_streams.wgsl"),
            include_str!("mesh_stages.wgsl"),
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked mesh stages"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        device.mesh(&wgpu::MeshPipelineDescriptor {
            label: Some("cooked mesh stages"),
            layout: Some(&self.pipeline),
            task: Some(wgpu::TaskState {
                module: &module,
                entry_point: Some("cooked_object"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            mesh: wgpu::MeshState {
                module: &module,
                entry_point: Some(mesh_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some(target.fragment_entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &target.color_formats,
            }),
            primitive: crate::material::cooked_primitive(),
            depth_stencil: Some(target.depth()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn compile_shadow(
        &self,
        device: &DeviceSite,
        surface: &SceneShadowSurface<'_>,
        config: &crate::CookedConfig,
    ) -> wgpu::RenderPipeline {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cooked mesh-stage Sun casters"),
            bind_group_layouts: &[
                Some(surface.view_layout),
                None,
                None,
                None,
                Some(&self.group),
            ],
            immediate_size: 0,
        });
        let report = NO_REJECTION_REPORT;
        #[cfg(feature = "draw-probe")]
        let report = if self.shadow_rejections.is_some() {
            &rejection_shader(8)
        } else {
            report
        };
        let shader = format!(
            "{}{report}",
            concat!(
                "enable wgpu_mesh_shader;\n",
                include_str!("shadow.wgsl"),
                "\n",
                include_str!("meshlet_cull.wgsl"),
                "\n",
                include_str!("mesh_stage_streams.wgsl"),
                "\n",
                include_str!("mesh_stage_shadow.wgsl"),
                "\n",
            )
        );
        let primitive = crate::draw::caster_primitive();
        #[cfg(not(feature = "draw-probe"))]
        let _ = config;
        #[cfg(feature = "draw-probe")]
        let (primitive, shader) = match config.mesh_stage_shadow_fault {
            Some(crate::draw_probe::ShadowFault::CullBackFaces) => (
                wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    ..primitive
                },
                shader,
            ),
            Some(crate::draw_probe::ShadowFault::IdentityPlacement) => (
                primitive,
                shader.replace(
                    "caster_position(mesh_position(vertex), mesh_placement.model)",
                    "light_view_projection * vec4<f32>(mesh_position(vertex), 1.0)",
                ),
            ),
            None => (primitive, shader),
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cooked mesh-stage Sun casters"),
            source: wgpu::ShaderSource::Wgsl(shader.into()),
        });
        device.mesh(&wgpu::MeshPipelineDescriptor {
            label: Some("cooked mesh-stage Sun casters"),
            layout: Some(&layout),
            task: Some(wgpu::TaskState {
                module: &module,
                entry_point: Some("shadow_object"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            mesh: wgpu::MeshState {
                module: &module,
                entry_point: Some("shadow_mesh"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: None,
            primitive,
            depth_stencil: Some(crate::draw::caster_depth(surface)),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    pub(crate) fn bind(
        &self,
        device: &wgpu::Device,
        buffers: [&wgpu::Buffer; 7],
    ) -> wgpu::BindGroup {
        let stream_entries: [_; 7] = std::array::from_fn(|i| wgpu::BindGroupEntry {
            binding: u32::try_from(i).expect("mesh group binding fits u32"),
            resource: buffers[i].as_entire_binding(),
        });
        let mut entries = stream_entries.to_vec();
        entries.push(wgpu::BindGroupEntry {
            binding: VIEW_BINDING,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.view,
                offset: 0,
                size: self.view_size,
            }),
        });
        #[cfg(feature = "draw-probe")]
        entries.extend(
            [(7, &self.rejections), (8, &self.shadow_rejections)]
                .into_iter()
                .filter_map(|(binding, buffer)| {
                    Some(wgpu::BindGroupEntry {
                        binding,
                        resource: buffer.as_ref()?.as_entire_binding(),
                    })
                }),
        );
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cooked mesh stages"),
            layout: &self.group,
            entries: &entries,
        })
    }
}

#[cfg(feature = "draw-probe")]
fn rejection_shader(binding: u32) -> String {
    format!(
        "@group(4) @binding({binding}) var<storage, read_write> counts: array<atomic<u32>, {CULL_OUTCOMES}>;
         fn cooked_report_rejection(reason: u32) {{ atomicAdd(&counts[reason], 1u); }}"
    )
}

#[cfg(feature = "draw-probe")]
fn rejection_layout(
    surface: &SceneSurface<'_>,
    config: &crate::CookedConfig,
    defaults: &[wgpu::BindGroupLayoutEntry],
) -> (
    Vec<wgpu::BindGroupLayoutEntry>,
    Option<wgpu::Buffer>,
    Option<wgpu::Buffer>,
) {
    let mut entries = defaults.to_vec();
    let mut make = |binding, witness: &std::sync::Arc<std::sync::Mutex<Option<wgpu::Buffer>>>| {
        entries.push(binding::storage(
            binding,
            wgpu::ShaderStages::TASK,
            false,
            CULL_WITNESS_BYTES,
        ));
        let buffer = surface.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cooked object-stage rejection witness"),
            size: CULL_WITNESS_BYTES,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        if let Ok(mut witness) = witness.lock() {
            *witness = Some(buffer.clone());
        }
        buffer
    };
    let rejections = config
        .mesh_stage_rejections
        .as_ref()
        .map(|witness| make(7, witness));
    let shadow_rejections = config
        .mesh_stage_shadow_rejections
        .as_ref()
        .filter(|_| surface.shadow.is_some())
        .map(|witness| make(8, witness));
    (entries, rejections, shadow_rejections)
}

pub(crate) fn draw(
    pass: &mut wgpu::RenderPass<'_>,
    bind: &wgpu::BindGroup,
    meshlets: u32,
    view_offset: u32,
) {
    pass.set_bind_group(GROUP, bind, &[view_offset]);
    pass.draw_mesh_tasks(groups(meshlets), 1, 1);
}
