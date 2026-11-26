use glam::{Mat4, Vec3};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use thrum_app::sky::Camera;
use thrum_render::binding;
use thrum_render::{
    atmosphere::EnvironmentProducts,
    contribution::{
        AfterOpaqueKind, AfterOpaquePass, ContributionFrame, ContributionPasses, EffectCadence,
        EffectGraph, QualityError, QualityInvalidation, QualityKnob, QualitySettings,
        SceneContribution, SceneSurface, SecondaryView, SecondaryViewRequest, SecondaryViewSlot,
        ViewAir, ViewClipPlane,
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
    quality: QualitySettings,
    cost_requests: Option<Arc<AtomicU32>>,
}

struct Prepared {
    slot: SecondaryView,
    second: Option<SecondaryView>,
    environment: EnvironmentProducts,
    group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    #[cfg(not(target_arch = "wasm32"))]
    bindings: PoolBindings,
}

#[derive(Clone)]
struct PoolBindings {
    device: wgpu::Device,
    layout: wgpu::BindGroupLayout,
    params: wgpu::Buffer,
    sampler: wgpu::Sampler,
}

impl Pool {
    /// Compose the normal two-stretch planar reflection.
    #[expect(
        clippy::missing_panics_doc,
        reason = "the static knob definitions are valid"
    )]
    #[must_use]
    pub fn new(plane: PoolPlane) -> Self {
        Self {
            plane,
            prepared: None,
            asked: false,
            cost_requests: None,
            quality: QualitySettings::new(
                [
                    ("view extent percent", 100, 25..=100, "Raster viewport as a percentage of each prepared side"),
                    ("view far metres", 32_000, 64..=32_000, "Far clip distance in metres"),
                    ("view least pixels", 0, 0..=16, "Reject small projected instances and indirect cooked meshlets below this size in pixels"),
                    ("view Sun cascade count", 1, 0..=2, "Number of local Sun shadow cascades; zero keeps height-map shadows"),
                    ("view Sun reach metres", 128, 16..=512, "Far edge of local Sun cascades in metres"),
                ]
                .map(|(name, value, range, effect)| QualityKnob {
                    name: name.into(),
                    effect,
                    value,
                    range,
                    invalidation: QualityInvalidation::None,
                    governor_steps: None,
                })
                .into(),
            )
            .expect("valid mirror quality definitions"),
        }
    }

    /// Prepare two identical views for a paired cost run. The caller may
    /// request zero, one or two each frame. The pool samples the first.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub fn with_cost_requests(mut self, requests: Arc<AtomicU32>) -> Self {
        self.cost_requests = Some(requests);
        self
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
    ) -> Result<SyntheticView, thrum_render::error::RenderError> {
        Self::reflected_view_with_far(camera, eye, level, extent, 32_000.0)
    }

    fn reflected_view_with_far(
        camera: Camera,
        eye: [f32; 3],
        level: f32,
        extent: [u32; 2],
        far: f32,
    ) -> Result<SyntheticView, thrum_render::error::RenderError> {
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
            far,
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
        images: thrum_render::contribution::AfterOpaqueImages::None,
    }
}

impl SceneContribution for Pool {
    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.quality.knobs().to_vec()
    }

    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        self.quality.set(name, value)
    }

    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        self.quality.commit()
    }

    fn secondary_view_slots(&self) -> Vec<SecondaryViewSlot> {
        vec![
            SecondaryViewSlot::new(1.0).expect("full extent");
            1 + usize::from(self.cost_requests.is_some())
        ]
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn secondary_resize_job(
        &self,
    ) -> Result<
        Option<thrum_render::contribution::SecondaryResizeJob>,
        thrum_render::runtime::FrameError,
    > {
        use thrum_render::runtime::FrameError;
        let bindings = self
            .prepared
            .as_ref()
            .ok_or(FrameError::InvalidState("mirror not prepared"))?
            .bindings
            .clone();
        let graph = self.effect_graph();
        Ok(Some(Box::new(move |slots| {
            let slot = slots
                .first()
                .ok_or(FrameError::InvalidState("missing mirror view slot"))?
                .clone();
            let second = slots.get(1).cloned();
            let group = bindings.bind(&slot);
            Ok(thrum_render::contribution::SecondaryResize::new(
                graph,
                move |owner: &mut Self| {
                    let prepared = owner.prepared.as_mut().expect("mirror prepared");
                    prepared.group = group;
                    prepared.slot = slot;
                    prepared.second = second;
                },
            ))
        })))
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

    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        self.prepared = Some(
            Prepared::new(surface, self.plane).map_err(thrum_render::runtime::FrameError::Scene)?,
        );
        Ok(())
    }

    fn encode_upload(&mut self, _encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        self.asked = false;
        let Some(prepared) = &self.prepared else {
            return;
        };
        let count = self
            .cost_requests
            .as_ref()
            .map_or(1, |requested| requested.load(Ordering::Relaxed));
        if count == 0 {
            return;
        }
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
        let [extent_percent, far, least_pixels, cascades, reach] = self.quality.knobs() else {
            unreachable!("the pool declares five controls")
        };
        let maximum = prepared.slot.maximum_extent();
        let extent = maximum.map(|size| (size * extent_percent.value / 100).max(1));
        let level = self.plane.level;
        let request =
            Self::reflected_view_with_far(camera, frame.camera, level, maximum, far.value as f32)
                .ok()
                .and_then(|view| {
                    let plane = ViewClipPlane::new([0.0, 1.0, 0.0, -level]).ok()?;
                    SecondaryViewRequest::new(&view, extent, Some(plane), ViewAir::Plane)
                        .ok()
                        .and_then(|request| {
                            request.with_least_pixels(least_pixels.value as f32).ok()
                        })
                        .and_then(|request| {
                            request
                                .with_shadows(cascades.value, reach.value as f32)
                                .ok()
                        })
                });
        if let Some(request) = request {
            self.asked = prepared.slot.request(&request).is_ok();
            if count == 2
                && let Some(second) = &prepared.second
            {
                let result = second.request(&request);
                debug_assert!(result.is_ok(), "{result:?}");
            }
        }
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
        let uniform = |binding| binding::uniform(binding, wgpu::ShaderStages::VERTEX_FRAGMENT, 0);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                uniform(0),
                uniform(1),
                binding::texture(
                    2,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2,
                ),
                binding::sampler(
                    3,
                    wgpu::ShaderStages::FRAGMENT,
                    wgpu::SamplerBindingType::Filtering,
                ),
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
        let pipeline = pipeline(surface, &environment, &layout);
        let bindings = PoolBindings {
            device: (**device).clone(),
            layout: layout.into_raw(),
            params,
            sampler,
        };
        let group = bindings.bind(&slot);
        Ok(Self {
            slot,
            second: surface.secondary_views.get(1).cloned(),
            environment,
            group,
            pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            bindings,
        })
    }
}

impl PoolBindings {
    fn bind(&self, slot: &SecondaryView) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.params.as_entire_binding(),
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
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
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
