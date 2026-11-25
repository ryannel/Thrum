use super::{Body, Hull, scene::Fleet};
use bytemuck::{Pod, Zeroable};
use glam::{DMat4, Mat4, Vec3, vec3};
use std::{
    ops::Range,
    sync::{Arc, Mutex},
};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, SceneContribution, SceneSurface,
};
use thrum_water::{BodyBounds, OpticsBodies};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    colour: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    previous: [[f32; 4]; 4],
    valid: u32,
}

type Matrix = [[f32; 4]; 4];

struct Prepared {
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    instances: wgpu::Buffer,
    queue: wgpu::Queue,
    craft: Range<u32>,
    float: Range<u32>,
    bounds: [[Vec3; 2]; 2],
}

pub(super) struct Draw {
    pub(super) fleet: Arc<Mutex<Fleet>>,
    prepared: Option<Prepared>,
    previous: Vec<Matrix>,
    recorded: Vec<Matrix>,
    opaque: OpticsBodies,
    visible_count: usize,
}

impl Draw {
    pub(super) fn new(
        fleet: Arc<Mutex<Fleet>>,
        opaque: OpticsBodies,
        visible_count: usize,
    ) -> Self {
        Self {
            fleet,
            prepared: None,
            previous: Vec::new(),
            recorded: Vec::new(),
            opaque,
            visible_count,
        }
    }
}

impl SceneContribution for Draw {
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
        let device = surface.device;
        let mut vertices = Vec::new();
        craft(&mut vertices);
        let split = u32::try_from(vertices.len()).map_err(|_| {
            thrum_render::runtime::FrameError::InvalidState(
                "floating mesh exceeds the vertex index range",
            )
        })?;
        float(&mut vertices);
        let end = u32::try_from(vertices.len()).map_err(|_| {
            thrum_render::runtime::FrameError::InvalidState(
                "floating mesh exceeds the vertex index range",
            )
        })?;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("floating craft and buoys"),
            source: wgpu::ShaderSource::Wgsl(
                format!("{}\n{}", surface.shader_source, include_str!("draw.wgsl")).into(),
            ),
        });
        let targets: Vec<_> = surface
            .color_formats
            .iter()
            .map(|format| {
                Some(wgpu::ColorTargetState {
                    format: *format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect();
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some("floating craft and buoys"), layout: Some(surface.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module, entry_point: Some("floating_vertex"), compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
                            7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Uint32],
                    }),
                ],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: surface.depth_format, depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::GreaterEqual), stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module, entry_point: Some("floating_fragment"), compilation_options: wgpu::PipelineCompilationOptions::default(), targets: &targets,
            }),
            multiview_mask: None, cache: None,
        });
        let (craft_vertices, float_vertices) = vertices.split_at(split as usize);
        let bounds = [local_bounds(craft_vertices), local_bounds(float_vertices)];
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("floating hull geometry"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let count = self.fleet.lock().expect("floating fleet").current.len();
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("floating hull poses"),
            size: (size_of::<Instance>() * count) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.previous.clear();
        self.prepared = Some(Prepared {
            pipeline,
            vertices,
            instances,
            queue: surface.queue.clone(),
            craft: 0..split,
            float: split..end,
            bounds,
        });
        Ok(())
    }

    fn encode_upload(&mut self, _: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let Some(prepared) = &self.prepared else {
            return;
        };
        let fleet = self.fleet.lock().expect("floating fleet");
        let alpha = f64::from(frame.time.alpha);
        self.recorded.clear();
        self.recorded.extend(
            fleet
                .previous
                .iter()
                .zip(&fleet.current)
                .map(|(old, new)| pose(old, new, alpha)),
        );
        let boxes = self
            .recorded
            .iter()
            .zip(&fleet.current)
            .take(self.visible_count)
            .filter(|(_, body)| !matches!(body.hull, Hull::Craft) || fleet.craft_present)
            .filter_map(|(pose, body)| {
                let bounds = prepared.bounds[usize::from(matches!(body.hull, Hull::Float))];
                placed_bounds(Mat4::from_cols_array_2d(pose), bounds)
            })
            .collect::<Vec<_>>();
        self.opaque.publish(&boxes);
        drop(fleet);
        let inputs: Vec<_> = self
            .recorded
            .iter()
            .enumerate()
            .map(|(index, &model)| {
                let old = self.previous.get(index).filter(|_| !frame.history_reset);
                Instance {
                    model,
                    previous: old.copied().unwrap_or(model),
                    valid: u32::from(old.is_some()),
                }
            })
            .collect();
        prepared
            .queue
            .write_buffer(&prepared.instances, 0, bytemuck::cast_slice(&inputs));
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        let Some(prepared) = &self.prepared else {
            return;
        };
        let fleet = self.fleet.lock().expect("floating fleet");
        pass.set_pipeline(&prepared.pipeline);
        pass.set_vertex_buffer(0, prepared.vertices.slice(..));
        pass.set_vertex_buffer(1, prepared.instances.slice(..));
        for (index, body) in (0_u32..).zip(&fleet.current).take(self.visible_count) {
            let vertices = match body.hull {
                Hull::Craft if !fleet.craft_present => continue,
                Hull::Craft => &prepared.craft,
                Hull::Float => &prepared.float,
            };
            pass.draw(vertices.clone(), index..index + 1);
        }
    }

    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            std::mem::swap(&mut self.recorded, &mut self.previous);
        }
    }
}

fn pose(old: &Body, new: &Body, alpha: f64) -> Matrix {
    let rotation = old.rotation.slerp(new.rotation, alpha);
    let position = old.position.lerp(new.position, alpha);
    DMat4::from_rotation_translation(rotation, position)
        .as_mat4()
        .to_cols_array_2d()
}

fn triangle(vertices: &mut Vec<Vertex>, [a, b, c]: [Vec3; 3], colour: [f32; 3]) {
    let normal = (b - a).cross(c - a).normalize_or_zero().to_array();
    vertices.extend([a, b, c].map(|p| Vertex {
        position: p.to_array(),
        normal,
        colour,
    }));
}

fn strip<const N: usize>(
    vertices: &mut Vec<Vertex>,
    lower: [Vec3; N],
    upper: [Vec3; N],
    colour: [f32; 3],
) {
    for i in 0..N {
        let j = (i + 1) % N;
        triangle(vertices, [lower[i], upper[j], lower[j]], colour);
        triangle(vertices, [lower[i], upper[i], upper[j]], colour);
    }
}

fn cap<const N: usize>(vertices: &mut Vec<Vertex>, ring: [Vec3; N], colour: [f32; 3]) {
    for i in 1..N - 1 {
        triangle(vertices, [ring[0], ring[i + 1], ring[i]], colour);
    }
}

fn round_strip<const N: usize>(
    vertices: &mut Vec<Vertex>,
    lower: [Vec3; N],
    upper: [Vec3; N],
    colour: [f32; 3],
) {
    let rise = upper[0].y - lower[0].y;
    let run = lower[0].x - upper[0].x;
    let vertex = |p: Vec3| {
        let normal = (p.with_y(0.0).normalize_or_zero() * rise + Vec3::Y * run).normalize();
        Vertex {
            position: p.to_array(),
            normal: normal.to_array(),
            colour,
        }
    };
    for i in 0..N {
        let j = (i + 1) % N;
        vertices.extend([lower[i], upper[j], lower[j], lower[i], upper[i], upper[j]].map(vertex));
    }
}

fn craft(vertices: &mut Vec<Vertex>) {
    const DECK: f32 = 1.36;
    let outline = [
        [2.9, 0.0],
        [1.72, 0.74],
        [0.2, 0.89],
        [-2.43, 0.76],
        [-2.43, -0.76],
        [0.2, -0.89],
        [1.72, -0.74],
    ];
    let ring = |y, scale: f32| outline.map(|[x, z]| vec3(x, y + 0.06 * x.max(0.0), z * scale));
    strip(
        vertices,
        ring(-0.46, 0.54),
        ring(0.42, 0.90),
        [0.25, 0.18, 0.114],
    );
    strip(
        vertices,
        ring(0.42, 0.90),
        ring(0.56, 0.94),
        [0.67, 0.54, 0.31],
    );
    strip(
        vertices,
        ring(0.56, 0.94),
        ring(DECK, 1.0),
        [0.011, 0.13, 0.16],
    );
    cap(vertices, ring(DECK, 1.0), [0.30, 0.17, 0.085]);
    strip(
        vertices,
        ring(DECK - 0.03, 1.012),
        ring(DECK + 0.02, 1.012),
        [0.13, 0.072, 0.042],
    );
    let cabin = |y, front, back, half| {
        [
            vec3(front, DECK + y, half),
            vec3(back, DECK + y, half),
            vec3(back, DECK + y, -half),
            vec3(front, DECK + y, -half),
        ]
    };
    strip(
        vertices,
        cabin(0.02, 0.68, -0.95, 0.59),
        cabin(0.47, 0.65, -0.92, 0.56),
        [0.61, 0.58, 0.46],
    );
    strip(
        vertices,
        cabin(0.47, 0.65, -0.92, 0.56),
        cabin(0.94, 0.37, -0.84, 0.49),
        [0.018, 0.047, 0.062],
    );
    strip(
        vertices,
        cabin(0.94, 0.50, -0.93, 0.54),
        cabin(1.03, 0.50, -0.93, 0.54),
        [0.63, 0.61, 0.52],
    );
    cap(vertices, cabin(1.03, 0.50, -0.93, 0.54), [0.63, 0.61, 0.52]);
    for side in [-1.0, 1.0] {
        let a = vec3(-0.24, DECK + 0.47, side * 0.568);
        let b = vec3(-0.16, DECK + 0.47, side * 0.568);
        let c = vec3(-0.25, DECK + 0.94, side * 0.498);
        let d = vec3(-0.33, DECK + 0.94, side * 0.498);
        let frame = [0.63, 0.61, 0.52];
        if side > 0.0 {
            triangle(vertices, [a, b, c], frame);
            triangle(vertices, [a, c, d], frame);
        } else {
            triangle(vertices, [a, c, b], frame);
            triangle(vertices, [a, d, c], frame);
        }
    }
}

fn float(vertices: &mut Vec<Vertex>) {
    const SIDES: u8 = 24;
    let ring = |y, radius| {
        std::array::from_fn::<_, { SIDES as usize }, _>(|i| {
            let angle = f32::from(u8::try_from(i).expect("buoy ring")) * std::f32::consts::TAU
                / f32::from(SIDES);
            vec3(radius * angle.cos(), y, radius * angle.sin())
        })
    };
    round_strip(
        vertices,
        ring(-0.10, 0.27),
        ring(0.21, 0.44),
        [0.21, 0.037, 0.019],
    );
    round_strip(
        vertices,
        ring(0.21, 0.44),
        ring(0.45, 0.44),
        [0.63, 0.087, 0.019],
    );
    round_strip(
        vertices,
        ring(0.45, 0.44),
        ring(0.56, 0.39),
        [0.74, 0.62, 0.29],
    );
    round_strip(
        vertices,
        ring(0.56, 0.39),
        ring(0.85, 0.11),
        [0.67, 0.12, 0.018],
    );
    cap(vertices, ring(0.85, 0.11), [0.49, 0.081, 0.015]);
}

fn local_bounds(vertices: &[Vertex]) -> [Vec3; 2] {
    vertices.iter().fold(
        [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)],
        |[low, high], vertex| {
            let point = Vec3::from_array(vertex.position);
            [low.min(point), high.max(point)]
        },
    )
}

fn placed_bounds(model: Mat4, [low, high]: [Vec3; 2]) -> Option<BodyBounds> {
    let centre = model.transform_point3((low + high) * 0.5);
    let radius = (high - low) * 0.5;
    let extent = model.x_axis.truncate().abs() * radius.x
        + model.y_axis.truncate().abs() * radius.y
        + model.z_axis.truncate().abs() * radius.z
        + Vec3::splat(0.02);
    BodyBounds::new((centre - extent).to_array(), (centre + extent).to_array())
}
