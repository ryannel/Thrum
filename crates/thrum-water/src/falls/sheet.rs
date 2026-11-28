use super::{Falls, Segment, active};
use crate::{foam::Foam, interaction::Interaction};
use bytemuck::{Pod, Zeroable};
use glam::{DVec2, DVec3, Vec3, Vec4};
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
const SPLASHES_PER_SEGMENT: usize = 24;
const RIBBONS_PER_SEGMENT: usize = 6;
const STRAND_SIDES: usize = 8;
const STRAND_EDGE: f32 = 4.0;
const STRAND_ROLL_START: f32 = 0.39;
const VERTICES_PER_SEGMENT: usize =
    RIBBONS_PER_SEGMENT * 6 * STRAND_SIDES * (ROWS + 3) + 6 * SPLASHES_PER_SEGMENT;
const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    world_time: [f32; 4],
    velocity_along: [f32; 4],
    discharge: [f32; 4],
    across_fraction: [f32; 4],
    edge_receiver: [f32; 4],
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
        interaction: Option<&Interaction>,
        foam: Option<&Foam>,
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
        let mut layout_entries = vec![binding::uniform(0, wgpu::ShaderStages::VERTEX_FRAGMENT, 0)];
        let mut group_entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: clock.as_entire_binding(),
        }];
        if let Some(waves) = interaction {
            layout_entries.extend(Interaction::sample_layout(1, wgpu::ShaderStages::VERTEX));
            group_entries.extend(waves.sample_entries(1));
        }
        if let Some(stocks) = foam {
            layout_entries.extend(Foam::sample_layout().map(|mut entry| {
                entry.visibility = wgpu::ShaderStages::VERTEX;
                entry
            }));
            group_entries.extend(stocks.sample_entries());
        }
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &layout_entries,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &layout,
            entries: &group_entries,
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
        let stocks = foam.map_or_else(
            || "fn fall_foam(_point: vec2<f32>) -> vec2<f32> { return vec2(0.0); }".into(),
            |foam| foam.stock_source()
                + "fn fall_foam(point: vec2<f32>) -> vec2<f32> { return foam_at(point, vec4(0.0)).stocks.xy; }",
        );
        let source = format!(
            "{library}\n{}\nconst FALL_LOOP: f32 = {:?};\nconst FALL_CONTACT_DROPS: f32 = {}.0;\nconst FALL_TEAR: f32 = {:?};\nconst FALL_STRAND_EDGE: f32 = {STRAND_EDGE:?};\nconst FALL_STRAND_ROLL: f32 = {STRAND_ROLL_START:?};\nfn fall_cloud(point: vec3<f32>) -> f32 {{ return {cloud}; }}\n{}\n{stocks}\n{}\n{}",
            crate::VIEW_RECORDS,
            thrum_sea_model::LOOP_SECONDS,
            SPLASHES_PER_SEGMENT / 2,
            falls.profile.sheet.tear,
            Interaction::sample_source(3, 1, 2, interaction.is_some()),
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
        let level = segment
            .plunge
            .map_or(segment.bottom.height, |hit| hit.level);
        // A path entering at the bank is already below the pool's level,
        // so its foot stays on the rock as the surface moves.
        let contact = match segment.plunge {
            None => 0.0,
            Some(hit) if segment.bottom.height < hit.level - 0.01 => 1.0,
            Some(_) => 2.0,
        };
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
                let normal = DVec3::new(across.x, 0.0, across.y)
                    .cross(velocity)
                    .normalize_or_zero();
                let forward = DVec3::new(velocity.x, 0.12, velocity.z);
                let outward = if normal.dot(forward) >= 0.0 {
                    normal
                } else {
                    -normal
                };
                if !edge {
                    let bulb = (4.0 * fraction * (1.0 - fraction)).max(0.0);
                    world += outward * bulb * 0.65 * physical.thickness;
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
                edge_receiver: [f32::from(edge), level as f32, contact, 0.0],
            }
        };
        let strip = |t| [at(t, 0), at(t, 1), at(t, 2)];
        let stations = || {
            [-0.65, -0.33, 0.0]
                .into_iter()
                .chain((1..=rows).map(|row| row as f64 / rows as f64))
        };
        let half = strip(0.5)[1];
        let foot = strip(1.0);
        if half.discharge[2] < 0.53 {
            filament(vertices, |fraction| at(fraction, 1), stations());
        } else {
            let thickness = f64::from(foot[1].discharge[3]) / sheet.tear;
            let spread = ((2.48 - thickness) / 0.73).clamp(0.0, 1.0) as f32;
            let mut top = ribbons(strip(-1.0), spread);
            for fraction in stations() {
                let bottom = ribbons(strip(fraction), spread);
                for (a, b) in top.into_iter().zip(bottom) {
                    if spread > 0.0 {
                        stitch_strand(vertices, a, b, spread);
                    } else {
                        stitch(vertices, a, b);
                    }
                }
                top = bottom;
            }
        }
        if segment.plunge.is_some() {
            splash(vertices, foot);
        }
    }
}

#[expect(clippy::cast_precision_loss, reason = "ribbon indices are bounded")]
fn ribbons(section: [Vertex; 3], spread: f32) -> [[Vertex; 3]; RIBBONS_PER_SEGMENT] {
    let at = |distance: f32| {
        let (a, b, t) = if distance <= 0.5 {
            (section[0], section[1], 2.0 * distance)
        } else {
            (section[1], section[2], 2.0 * distance - 1.0)
        };
        let interpolate = |a, b| Vec4::from_array(a).lerp(Vec4::from_array(b), t).to_array();
        Vertex {
            world_time: interpolate(a.world_time, b.world_time),
            velocity_along: interpolate(a.velocity_along, b.velocity_along),
            discharge: interpolate(a.discharge, b.discharge),
            across_fraction: interpolate(a.across_fraction, b.across_fraction),
            edge_receiver: [0.0, a.edge_receiver[1], a.edge_receiver[2], 0.0],
        }
    };
    let step = 1.0 / RIBBONS_PER_SEGMENT as f32;
    let division = |index: usize| {
        let equal = index as f32 * step;
        if index == 0 || index == RIBBONS_PER_SEGMENT {
            return equal;
        }
        let lip = at(equal).velocity_along[3];
        equal + spread * step * 0.39 * (lip * 43.7).sin()
    };
    std::array::from_fn(|index| {
        let (middle, half_width) = if spread > 0.0 {
            let (left, right) = (division(index), division(index + 1));
            (f32::midpoint(left, right), 0.5 * (right - left))
        } else {
            let half_width = 0.5 / RIBBONS_PER_SEGMENT as f32;
            ((2.0 * index as f32 + 1.0) * half_width, half_width)
        };
        let centre = at(middle);
        let [ax, _, az, fraction] = centre.across_fraction;
        let along = centre.velocity_along[3];
        let fold = (0.5 + 0.5 * (along * 5.63).sin()) * (0.5 + 0.5 * (along * 12.29).cos());
        let separation = ((fraction - 0.41 - 0.23 * fold) / 0.43).clamp(0.0, 1.0);
        let split = separation * separation * (3.0 - 2.0 * separation);
        let film = centre.discharge[2];
        let tearing = 0.20 + 0.80 * (1.0 - film);
        let narrowed = half_width * (1.0 - split * tearing * (0.10 + 0.34 * fold));
        let velocity = Vec3::from_slice(&centre.velocity_along);
        let normal = Vec3::new(ax, 0.0, az).cross(velocity).normalize_or_zero();
        let outward = normal
            * if normal.dot(Vec3::new(velocity.x, 0.12, velocity.z)) >= 0.0 {
                1.0
            } else {
                -1.0
            };
        let depth = (0.034 + 0.24 * fold) * split * (1.0 - fraction).max(0.0).sqrt();
        [-1.0_f32, 0.0, 1.0].map(|edge| {
            let mut vertex = at(edge.mul_add(narrowed, middle));
            let position =
                Vec3::from_slice(&vertex.world_time) + outward * depth * (1.0 - 0.42 * edge.abs());
            vertex.world_time = position.extend(vertex.world_time[3]).to_array();
            vertex.edge_receiver[0] = edge.abs();
            vertex.edge_receiver[3] = along;
            vertex
        })
    })
}

fn stitch_strand(vertices: &mut Vec<Vertex>, top: [Vertex; 3], bottom: [Vertex; 3], spread: f32) {
    let top = strand_section(top, spread);
    let bottom = strand_section(bottom, spread);
    for (side, &first) in top.iter().enumerate() {
        let next = (side + 1) % STRAND_SIDES;
        vertices.extend([
            first,
            top[next],
            bottom[side],
            top[next],
            bottom[next],
            bottom[side],
        ]);
    }
}

#[expect(clippy::cast_precision_loss, reason = "the strand has eight sides")]
fn strand_section([left, centre, right]: [Vertex; 3], spread: f32) -> [Vertex; STRAND_SIDES] {
    let [ax, _, az, fraction] = centre.across_fraction;
    let axis = Vec3::new(ax, 0.0, az);
    let velocity = Vec3::from_slice(&centre.velocity_along);
    let normal = axis.cross(velocity).normalize_or_zero();
    let outward = normal * normal.dot(Vec3::new(velocity.x, 0.12, velocity.z)).signum();
    let width =
        0.5 * Vec3::from_slice(&left.world_time).distance(Vec3::from_slice(&right.world_time));
    // Keep one connected shell from crest to contact. It is flat until the
    // strands have left the tongue, so there is no join across the falling face.
    let roll = (4.0 * (fraction - STRAND_ROLL_START)).clamp(0.0, 1.0) * spread;
    let interpolate = |a, b, t| Vec4::from_array(a).lerp(Vec4::from_array(b), t).to_array();
    std::array::from_fn(|i| {
        // Follow the actual flow orientation so the shell winds outwards at either bank.
        let index = if normal.dot(outward) >= 0.0 {
            (STRAND_SIDES - i) % STRAND_SIDES
        } else {
            i
        };
        let angle = index as f32 / STRAND_SIDES as f32;
        let (depth, width_phase) = (std::f32::consts::TAU * angle).sin_cos();
        let side = if width_phase < 0.0 { left } else { right };
        let across = width_phase.abs();
        let mut vertex = centre;
        vertex.world_time = interpolate(centre.world_time, side.world_time, across);
        vertex.velocity_along = interpolate(centre.velocity_along, side.velocity_along, across);
        vertex.discharge = interpolate(centre.discharge, side.discharge, across);
        let point =
            Vec3::from_slice(&vertex.world_time) + outward * depth * roll * (0.025 + width * 0.37);
        vertex.world_time = point.extend(vertex.world_time[3]).to_array();
        vertex.edge_receiver[0] = STRAND_EDGE + angle;
        vertex
    })
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
        let throat = tip.max(0.16).sqrt();
        let radius = (1.15 * vertex.discharge[3].sqrt()).clamp(0.028, 0.098) * throat;
        let world = Vec3::from_slice(&vertex.world_time);
        let before = Vec3::from_slice(&sample((fraction - 0.09).max(0.0)).world_time);
        let after = Vec3::from_slice(&sample((fraction + 0.09).clamp(0.0, 1.0)).world_time);
        let smoothing = (1.0 - 4.0 * (fraction - 0.5).powi(2)).max(0.0) as f32;
        let leave_lip = ((fraction as f32 + 0.11) / 0.38).clamp(0.0, 1.0);
        let lift = leave_lip * leave_lip * (3.0 - 2.0 * leave_lip);
        let centre = world.lerp((before + after) * 0.5, 0.66 * smoothing)
            + outward * lift * (radius + (0.03 + 0.05 * smoothing) * tip);
        let age = vertex.world_time[3];
        vertex.across_fraction[1] = -2.0 - distance;
        [
            (0.0, across),
            (1.0, outward * (0.12 + 0.88 * lift)),
            (2.0, -across),
            (3.0, -outward * (0.12 + 0.88 * lift)),
        ]
        .map(|(side, normal)| {
            vertex.world_time = (centre + normal * radius).extend(age).to_array();
            vertex.edge_receiver[0] = side;
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

// Render the liquid thrown back out of the impact as separate drops. The
// plunge's impulse and foam birth still use the original receiving level.
#[expect(clippy::cast_precision_loss, reason = "the drop count is bounded")]
fn splash(vertices: &mut Vec<Vertex>, foot: [Vertex; 3]) {
    for drop in 0..SPLASHES_PER_SEGMENT {
        let fraction = (drop as f32 + 0.5) / SPLASHES_PER_SEGMENT as f32;
        let mut vertex = foot[1];
        vertex.world_time = Vec4::from_array(foot[0].world_time)
            .lerp(Vec4::from_array(foot[2].world_time), fraction)
            .to_array();
        vertex.world_time[1] = vertex.edge_receiver[1];
        vertex.velocity_along[3] = (foot[2].velocity_along[3] - foot[0].velocity_along[3])
            .mul_add(fraction, foot[0].velocity_along[3]);
        vertex.discharge[2] = drop as f32;
        vertex.across_fraction[1] = -1.0;
        vertices.extend([0.0, 1.0, 2.0, 2.0, 1.0, 3.0].map(|corner| {
            vertex.across_fraction[3] = corner;
            vertex
        }));
    }
}
