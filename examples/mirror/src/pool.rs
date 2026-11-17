use glam::{Mat4, Vec3};
use thrum_app::sky::Camera;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectCadence,
        EffectGraph, SceneContribution, SceneSurface, SecondaryView, SecondaryViewRequest,
        SecondaryViewSlot, ViewAir, ViewClipPlane,
    },
    synthetic::SyntheticView,
};
use wgpu::util::DeviceExt;

const PASS: &str = "mirror pool";

/// A rectangular horizontal pool in world metres.
#[derive(Clone, Copy)]
pub struct PoolPlane {
    /// Surface height.
    pub level: f32,
    /// Minimum x and z.
    pub min: [f32; 2],
    /// Maximum x and z.
    pub max: [f32; 2],
}

/// The example owns the pool, its view request and the after-opaque composition.
pub struct Pool {
    plane: PoolPlane,
    prepared: Option<Prepared>,
    asked: bool,
}

struct Prepared {
    slot: SecondaryView,
    environment: EnvironmentProducts,
    group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl Pool {
    /// Compose the normal two-stretch planar reflection.
    #[must_use]
    pub fn new(plane: PoolPlane) -> Self {
        Self {
            plane,
            prepared: None,
            asked: false,
        }
    }

    /// Build a proper reflected camera with the same projection as the real camera.
    ///
    /// # Errors
    /// Returns the renderer's camera validation error.
    pub fn reflected_view(
        camera: Camera,
        eye: [f32; 3],
        level: f32,
        extent: [u32; 2],
    ) -> Result<SyntheticView, thrum_render::synthetic::RenderError> {
        let target = [
            camera.target[0],
            2.0 * level - camera.target[1],
            camera.target[2],
        ];
        SyntheticView::perspective(
            [eye[0], 2.0 * level - eye[1], eye[2]],
            target,
            [0.0, 1.0, 0.0],
            camera.vertical_fov_degrees.to_radians(),
            extent[0] as f32 / extent[1] as f32,
            0.1,
            32_000.0,
        )
    }
}

fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: PASS.into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: true,
        refraction: false,
    }
}

impl SceneContribution for Pool {
    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![SecondaryViewSlot::new(1.0).expect("full extent")]
    }

    fn effect_graph(&self) -> EffectGraph {
        EffectGraph {
            after_opaque: vec![declaration()],
            ..EffectGraph::default()
        }
    }

    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }

    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.prepared = Some(Prepared::new(surface, self.plane)?);
        Ok(())
    }

    fn encode_upload(&mut self, _encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.asked = false;
        let Some(prepared) = &self.prepared else {
            return;
        };
        // The fourth row of a perspective view-projection is the camera's
        // forward axis. Remove its jitter term from the row carrying the up axis.
        let rows = Mat4::from_cols_array_2d(&frame.view_proj).transpose();
        let forward = rows.w_axis.truncate().normalize();
        let up = rows.y_axis.truncate();
        let fov = (2.0 * (up - forward * up.dot(forward)).length_recip().atan()).to_degrees();
        let camera = Camera::look_at(
            frame.camera,
            (Vec3::from_array(frame.camera) + forward * 128.0).to_array(),
            fov,
        );
        let extent = prepared.slot.maximum_extent();
        let level = self.plane.level;
        let request = Self::reflected_view(camera, frame.camera, level, extent)
            .ok()
            .and_then(|view| {
                let plane = ViewClipPlane::new([0.0, 1.0, 0.0, -level]).ok()?;
                SecondaryViewRequest::new(&view, extent, Some(plane), ViewAir::Plane).ok()
            });
        self.asked = request.is_some_and(|request| prepared.slot.request(&request).is_ok());
    }

    fn draw_after_opaque(
        &self,
        _index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared
            && self.asked
        {
            pass.set_pipeline(&p.pipeline);
            pass.set_bind_group(
                2,
                p.environment
                    .group(frame.exposure_side)
                    .expect("exposure side"),
                &[],
            );
            pass.set_bind_group(3, &p.group, &[]);
            pass.draw(0..6, 0..1);
        }
    }

    fn encode_compute(&mut self, _pass: &mut wgpu::ComputePass<'_>, _frame: &ContributionFrame) {}
    fn computes(&self, _frame: &ContributionFrame) -> bool {
        false
    }
    fn encode_after_compute(&mut self, _encoder: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _pass: &mut wgpu::RenderPass<'_>) {}
    fn submitted(&mut self, _history_advanced: bool) {}
}

impl Prepared {
    fn new(surface: &SceneSurface<'_>, plane: PoolPlane) -> Result<Self, String> {
        let slot = surface
            .secondary_views
            .first()
            .ok_or("missing mirror view slot")?
            .clone();
        let environment = surface
            .environment_products()
            .ok_or("mirror needs the scene atmosphere")?;
        let device = surface.device;
        let uniform = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                min_binding_size: None,
                has_dynamic_offset: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                uniform(0),
                uniform(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(PASS),
            contents: bytemuck::cast_slice(&[
                plane.min[0],
                plane.min[1],
                plane.max[0],
                plane.max[1],
                plane.level,
                0.0,
                0.0,
                0.0,
            ]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: slot.record().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(slot.colour()),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline = pipeline(surface, &environment, &layout);
        Ok(Self {
            slot,
            environment,
            group,
            pipeline,
        })
    }
}

fn pipeline(
    surface: &SceneSurface<'_>,
    environment: &EnvironmentProducts,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let device = surface.device;
    let inputs = declaration().input_layout(device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(PASS),
        bind_group_layouts: &[
            Some(surface.view_layout),
            Some(&inputs),
            Some(&environment.layout),
            Some(layout),
        ],
        immediate_size: 0,
    });
    let source = format!(
        "{}\n{}\n{}",
        environment.shader_source.replace("@group(0)", "@group(2)"),
        SecondaryView::record_wgsl(),
        include_str!("pool.wgsl")
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(PASS),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.render(&wgpu::RenderPipelineDescriptor {
        label: Some(PASS),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("pool_vertex"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: surface.depth_format,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Greater),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("pool_fragment"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &surface.color_formats.map(|format| {
                Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            }),
        }),
        multiview_mask: None,
        cache: None,
    })
}
