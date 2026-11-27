use super::{Falls, Segment, active};
use bytemuck::{Pod, Zeroable};
use glam::{DVec2, DVec3};
use thrum_render::{
    atmosphere::EnvironmentProducts,
    binding,
    contribution::{
        AfterOpaqueImages, AfterOpaqueKind, AfterOpaquePass, ContributionFrame, EffectCadence,
        SceneSurface,
    },
};
use thrum_water_model::{Map, PathPoint};

pub(in crate::falls) const ROWS: usize = 32;
pub(crate) const PASS: &str = "waterfall sheet";
const VERTICES_PER_SEGMENT: usize = 24 * (ROWS + 3) + 36;
const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    world_time: [f32; 4],
    velocity_along: [f32; 4],
    discharge: [f32; 4],
    across_fraction: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Clock {
    phase: [f32; 4],
}

pub(crate) struct Sheet {
    mesh: wgpu::Buffer,
    clock: wgpu::Buffer,
    group: wgpu::BindGroup,
    pipelines: [wgpu::RenderPipeline; 2],
    vertices: Vec<Vertex>,
    key: Option<([u64; 2], u32)>,
    levels: Vec<u64>,
    next_levels: Vec<u64>,
}

pub(crate) fn declaration() -> AfterOpaquePass {
    AfterOpaquePass {
        name: PASS.into(),
        kind: AfterOpaqueKind::Render,
        cadence: EffectCadence::EveryFrame,
        accesses: Vec::new(),
        depth: false,
        write_depth: true,
        refraction: false,
        images: AfterOpaqueImages::None,
    }
}

impl Sheet {
    pub(crate) fn new(
        surface: &SceneSurface<'_>,
        environment: &EnvironmentProducts,
        library: &str,
        falls: &Falls,
    ) -> Result<Self, String> {
        let device = surface.device;
        let capacity = falls
            .records
            .iter()
            .map(|(_, record)| record.segment_capacity())
            .sum::<usize>();
        let size = (capacity * VERTICES_PER_SEGMENT * size_of::<Vertex>()) as u64;
        if size > surface.limits.max_buffer_size
            || capacity > u32::MAX as usize / VERTICES_PER_SEGMENT
        {
            return Err("fall paths exceed the device's prepared sheet capacity".into());
        }
        let mesh = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(PASS),
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let clock = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("waterfall sheet clock"),
            size: size_of::<Clock>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let drawing = declaration().input_layout(device);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[binding::uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT, 0)],
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: clock.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[
                Some(surface.view_layout),
                Some(&drawing),
                Some(&environment.layout),
                Some(&layout),
            ],
            immediate_size: 0,
        });
        let cloud = if surface.sun_optical_depth {
            "cloud_factor(point)"
        } else {
            "1.0"
        };
        let source = format!(
            "{library}\n{}\nconst FALL_LOOP: f32 = {:?};\nfn fall_cloud(point: vec3<f32>) -> f32 {{ return {cloud}; }}\n{}\n{}",
            crate::VIEW_RECORDS,
            thrum_sea_model::LOOP_SECONDS,
            include_str!("../foam/noise.wgsl"),
            concat!(include_str!("pattern.wgsl"), include_str!("sheet.wgsl"))
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let targets = surface.color_formats.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let pipelines = [false, true].map(|depth| {
            let mut targets = targets.clone();
            targets[0].as_mut().expect("colour").blend = Some(wgpu::BlendState::ALPHA_BLENDING);
            if !depth {
                targets[1].as_mut().expect("motion").write_mask = wgpu::ColorWrites::empty();
            }
            let maximum = wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
            };
            targets[2].as_mut().expect("reactive").blend = Some(wgpu::BlendState {
                color: maximum,
                alpha: maximum,
            });
            device.render(&wgpu::RenderPipelineDescriptor {
                label: Some(PASS),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("sheet_vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &ATTRIBUTES,
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: surface.depth_format,
                    depth_write_enabled: Some(depth),
                    depth_compare: Some(wgpu::CompareFunction::GreaterEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if depth { "sheet_solid" } else { "sheet_thin" }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &targets,
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        Ok(Self {
            mesh,
            clock,
            group,
            pipelines,
            vertices: Vec::new(),
            key: None,
            levels: Vec::new(),
            next_levels: Vec::new(),
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "wrapped seconds and frame delta fit shader floats"
    )]
    pub(crate) fn record(
        &mut self,
        queue: &wgpu::Queue,
        falls: &Falls,
        map: &Map,
        now: f64,
        previous: f64,
        reset: bool,
    ) {
        let spacing = falls.quality.knobs()[0].value;
        let key = (map.query_revision(), spacing);
        // Interpolated maps can share a revision within one tick even though
        // their receiving stages and source gauges differ.
        self.next_levels.clear();
        self.next_levels.extend(
            (0..)
                .map_while(|i| map.body(i))
                .map(|body| body.stage.value.to_bits()),
        );
        self.next_levels
            .extend(falls.records.iter().map(|(id, _)| map.gauge(*id).to_bits()));
        if self.key != Some(key) || self.levels != self.next_levels {
            std::mem::swap(&mut self.levels, &mut self.next_levels);
            self.vertices.clear();
            for segments in active(&falls.records, map, falls.profile.sheet) {
                mesh(
                    &mut self.vertices,
                    &segments,
                    f64::from(spacing) * 0.001,
                    falls.profile.sheet,
                );
            }
            if !self.vertices.is_empty() {
                queue.write_buffer(&self.mesh, 0, bytemuck::cast_slice(&self.vertices));
            }
            self.key = Some(key);
        }
        queue.write_buffer(
            &self.clock,
            0,
            bytemuck::bytes_of(&Clock {
                phase: [
                    now.rem_euclid(thrum_sea_model::LOOP_SECONDS) as f32,
                    if reset {
                        0.0
                    } else {
                        (now - previous).max(0.0) as f32
                    },
                    f32::from(reset),
                    0.0,
                ],
            }),
        );
    }

    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
        environment: &EnvironmentProducts,
    ) {
        if self.vertices.is_empty() {
            return;
        }
        let count = u32::try_from(self.vertices.len()).expect("prepared waterfall vertex count");
        pass.set_bind_group(
            2,
            environment
                .group(frame.exposure_side)
                .expect("exposure side"),
            &[],
        );
        pass.set_bind_group(3, &self.group, &[]);
        pass.set_vertex_buffer(0, self.mesh.slice(..));
        for pipeline in &self.pipelines {
            pass.set_pipeline(pipeline);
            pass.draw(0..count, 0..1);
        }
    }

    pub(crate) fn resource_bytes(&self) -> [(String, u64); 2] {
        [
            (PASS.into(), self.mesh.size()),
            ("waterfall sheet clock".into(), self.clock.size()),
        ]
    }
}

fn point_at(segment: &Segment, fraction: f64) -> (DVec3, DVec3, f64, f64) {
    let distance = fraction * segment.bottom.distance;
    let path = &segment.data.path;
    let index = path
        .partition_point(|point| point.distance <= distance)
        .clamp(1, path.len() - 1);
    let a = path[index - 1];
    let stored = path[index];
    let b = if stored.distance > segment.bottom.distance && a.distance < segment.bottom.distance {
        segment.bottom
    } else {
        stored
    };
    let world = |p: PathPoint| DVec3::new(p.place[0], p.height, p.place[1]);
    let alpha = (distance - a.distance) / (b.distance - a.distance).max(1e-9);
    let position = world(a).lerp(world(b), alpha);
    let velocity = (world(b) - world(a)) / (b.time - a.time).max(1e-9);
    (
        position,
        velocity,
        (b.time - a.time).mul_add(alpha, a.time),
        distance,
    )
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "row counts are clamped to the prepared vertex capacity and output uses shader metres"
)]
fn mesh(
    vertices: &mut Vec<Vertex>,
    segments: &[Segment],
    spacing: f64,
    sheet: thrum_water_model::SheetProfile,
) {
    let rows = segments
        .iter()
        .map(|segment| (segment.bottom.distance / spacing).ceil())
        .fold(1.0, f64::max)
        .min(ROWS as f64) as usize;
    for (index, segment) in segments.iter().enumerate() {
        let lip = segment.data.lip.map(DVec2::from_array);
        let centre = (lip[0] + lip[1]) * 0.5;
        let across = (lip[1] - lip[0]).normalize_or_zero();
        let at = |fraction: f64, side: usize| {
            let sample = |segment: &Segment| {
                if fraction >= 0.0 {
                    return point_at(segment, fraction);
                }
                let lip = segment.data.path[0];
                let Some(tongue) = segment.tongue else {
                    let (point, velocity, time, distance) = point_at(segment, 0.0);
                    return (
                        point + DVec3::Y * segment.data.launch_depth,
                        velocity,
                        time,
                        distance,
                    );
                };
                let world =
                    |point: PathPoint| DVec3::new(point.place[0], point.height, point.place[1]);
                let z = world(lip) + DVec3::Y * segment.data.launch_depth;
                let a = world(tongue);
                (
                    z.lerp(a, -fraction),
                    (z - a) / -tongue.time,
                    -fraction * tongue.time,
                    -fraction * tongue.distance,
                )
            };
            let (mut world, mut velocity, mut time, mut distance) = sample(segment);
            let edge = side != 1;
            let outer = side / 2;
            let neighbour = if outer == 0 {
                index.checked_sub(1)
            } else {
                index.checked_add(1)
            };
            let joined = neighbour.and_then(|i| segments.get(i)).filter(|other| {
                edge && DVec2::from_array(other.data.lip[1 - outer]).distance_squared(lip[outer])
                    < 1e-10
            });
            let mut discharge = segment.data.discharge;
            let mut launch = segment.data.launch_speed;
            let mut launch_depth = segment.data.launch_depth;
            let mut top = segment.data.path[0].height;
            let lip_place = if edge { lip[outer] } else { centre };
            if let Some(neighbour) = joined {
                let (p, v, t, d) = sample(neighbour);
                world = (world + p) * 0.5;
                velocity = (velocity + v) * 0.5;
                time = f64::midpoint(time, t);
                distance = f64::midpoint(distance, d);
                discharge = f64::midpoint(discharge, neighbour.data.discharge);
                launch = f64::midpoint(launch, neighbour.data.launch_speed);
                launch_depth = f64::midpoint(launch_depth, neighbour.data.launch_depth);
                top = f64::midpoint(top, neighbour.data.path[0].height);
            } else {
                let offset = lip_place - centre;
                world += DVec3::new(offset.x, 0.0, offset.y);
            }
            let physical = sheet.sample(discharge, launch, top - world.y, distance.max(0.0));
            if fraction >= 0.0 {
                let join = (1.0 - distance / (0.25 + 3.0 * launch_depth)).clamp(0.0, 1.0);
                world.y += launch_depth * join * join;
                if !edge {
                    // The full part of each ribbon has a rounded wet face.
                    // Its outer edges still meet the adjacent ribbons.
                    let belly =
                        0.65 * physical.thickness * (4.0 * fraction * (1.0 - fraction)).max(0.0);
                    let normal = DVec3::new(across.x, 0.0, across.y)
                        .cross(velocity)
                        .normalize_or_zero();
                    let forward = DVec3::new(velocity.x, 0.0, velocity.z);
                    world += normal
                        * belly
                        * if normal.dot(forward) >= 0.0 {
                            1.0
                        } else {
                            -1.0
                        };
                }
            }
            Vertex {
                world_time: world.as_vec3().extend(time as f32).to_array(),
                velocity_along: velocity
                    .as_vec3()
                    .extend(lip_place.dot(across) as f32)
                    .to_array(),
                discharge: [
                    launch as f32,
                    physical.white as f32,
                    physical.presence as f32,
                    physical.thickness as f32,
                ],
                across_fraction: [
                    across.x as f32,
                    segment.bottom.distance as f32,
                    across.y as f32,
                    fraction as f32,
                ],
            }
        };
        let strip = |t| [at(t, 0), at(t, 1), at(t, 2)];
        let stations = || {
            [-0.65, -0.33, 0.0]
                .into_iter()
                .chain((1..=rows).map(|row| row as f64 / rows as f64))
        };
        let half = strip(0.5)[1];
        if half.discharge[2] < 0.53 {
            filament(vertices, |fraction| at(fraction, 1), stations());
            continue;
        }
        let mut top = strip(-1.0);
        for fraction in stations() {
            let bottom = strip(fraction);
            stitch(vertices, top, bottom);
            top = bottom;
        }
        if segment.plunge.is_some() {
            splash(vertices, top);
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "station and geometry values are emitted as shader floats"
)]
fn filament(
    vertices: &mut Vec<Vertex>,
    sample: impl Fn(f64) -> Vertex,
    stations: impl Iterator<Item = f64>,
) {
    use glam::Vec3;

    let section = |fraction: f64| {
        let mut vertex = sample(fraction);
        let [ax, distance, az, _] = vertex.across_fraction;
        let across = Vec3::new(ax, 0.0, az);
        let velocity = Vec3::from_slice(&vertex.velocity_along);
        let normal = across.cross(velocity).normalize_or_zero();
        let reference = Vec3::new(velocity.x, 0.10, velocity.z);
        let outward = if normal.dot(reference) >= 0.0 {
            normal
        } else {
            -normal
        };
        let tip = 1.0 - fraction.clamp(0.0, 1.0) as f32;
        let radius = (0.65 * vertex.discharge[3].sqrt()).clamp(0.015, 0.073) * tip.sqrt();
        let world = Vec3::from_slice(&vertex.world_time);
        let before = Vec3::from_slice(&sample((fraction - 0.09).max(0.0)).world_time);
        let after = Vec3::from_slice(&sample((fraction + 0.09).clamp(0.0, 1.0)).world_time);
        let smoothing = (1.0 - 4.0 * (fraction - 0.5).powi(2)).max(0.0) as f32;
        let centre = world.lerp((before + after) * 0.5, 0.66 * smoothing)
            + outward * (radius + (0.03 + 0.05 * smoothing) * tip);
        let age = vertex.world_time[3];
        vertex.across_fraction[1] = -2.0 - distance;
        [across, outward, -across, -outward].map(|normal| {
            vertex.world_time = (centre + normal * radius).extend(age).to_array();
            vertex
        })
    };
    let mut top = section(-1.0);
    for station in stations {
        let bottom = section(station);
        for (i, &first) in top.iter().enumerate() {
            let next = (i + 1) % top.len();
            vertices.extend([
                first,
                top[next],
                bottom[i],
                top[next],
                bottom[next],
                bottom[i],
            ]);
        }
        top = bottom;
    }
}

fn stitch(vertices: &mut Vec<Vertex>, top: [Vertex; 3], bottom: [Vertex; 3]) {
    for edge in 0..2 {
        vertices.extend([
            top[edge],
            top[edge + 1],
            bottom[edge],
            top[edge + 1],
            bottom[edge + 1],
            bottom[edge],
        ]);
    }
}

// The thin crown above the pool belongs to the visible fall. The foam and
// wave fields still receive the original impact at the receiving level.
fn splash(vertices: &mut Vec<Vertex>, foot: [Vertex; 3]) {
    use glam::{Vec2, Vec3};

    let crown = |distance: f32, height: f32| {
        foot.map(|mut vertex| {
            let [x, y, z, age] = vertex.world_time;
            let [vx, _, vz, lip] = vertex.velocity_along;
            let direction = Vec2::new(vx, vz).normalize_or_zero();
            let thickness = vertex.discharge[3];
            let reach = (6.0 * thickness.sqrt()).clamp(0.12, 1.2);
            let bubble_height = (1.8 * thickness).clamp(0.025, 0.16);
            let drift = direction * (distance * reach - 0.08);
            vertex.world_time = [
                x + drift.x,
                height.mul_add(bubble_height, y),
                z + drift.y,
                age,
            ];
            vertex.velocity_along = Vec3::new(direction.x, 0.025, direction.y)
                .extend(lip)
                .to_array();
            vertex.across_fraction[1] = -1.0;
            vertex.across_fraction[3] = distance;
            vertex
        })
    };
    let mut top = crown(0.0, 0.8);
    for (distance, height) in [(0.22, 1.0), (0.55, 0.58), (1.0, 0.0)] {
        let row = crown(distance, height);
        stitch(vertices, top, row);
        top = row;
    }
}
