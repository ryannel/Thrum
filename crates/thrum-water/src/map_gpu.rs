//! Prepared map products and the shared camera-independent WGSL sampler.
use crate::WaterError;
use bytemuck::{Pod, Zeroable};
use thrum_render::binding;
use thrum_render::contribution::EffectTime;
use thrum_water_model::{BodyKind, Map};
use wgpu::util::DeviceExt;

mod current_track;
use current_track::CurrentTrack;

pub(crate) const BODY_LIMIT: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BodyRecord {
    level: [f32; 4],
    gains: [f32; 4],
    layers: [u32; 4],
    ends: [[f32; 4]; 2],
    joins: [[f32; 4]; 2],
    track: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MapRecord {
    origin_spacing_margin: [f32; 4],
    size_count_sea: [u32; 4],
    swash: [f32; 4],
    bodies: [BodyRecord; BODY_LIMIT],
}

/// Bed lattice and body records belonging to one device.
pub struct MapGpu {
    pub(crate) texture: wgpu::Texture,
    pub(crate) info: wgpu::Buffer,
    revision: Option<u64>,
    uploaded_stages: [f64; BODY_LIMIT],
    sampled_map: Option<Map>,
    recorded: Option<StageEndpoints>,
    submitted: Option<StageEndpoints>,
    previous_levels: Option<[f32; BODY_LIMIT]>,
    record: MapRecord,
    tracks: [Option<CurrentTrack>; BODY_LIMIT],
    support: Vec<[f32; 4]>,
}

#[derive(Clone, Copy)]
struct StageEndpoints {
    time: EffectTime,
    from: [f64; BODY_LIMIT],
    to: [f64; BODY_LIMIT],
}

impl StageEndpoints {
    fn record(time: EffectTime, current: [f64; BODY_LIMIT], previous: Option<&Self>) -> Self {
        let Some(previous) =
            previous.filter(|p| p.time.epoch == time.epoch && p.time.timeline == time.timeline)
        else {
            return Self {
                time,
                from: current,
                to: current,
            };
        };
        if previous.time.tick == time.tick {
            // Resampling a tick keeps its endpoints, even if new edits arrived.
            return Self { time, ..*previous };
        }
        let from = if previous.time.tick.checked_add(1) == Some(time.tick) {
            previous.to
        } else {
            current
        };
        Self {
            time,
            from,
            to: current,
        }
    }

    fn stages(&self) -> [f64; BODY_LIMIT] {
        let alpha = f64::from(self.time.alpha);
        if alpha >= 1.0 {
            return self.to;
        }
        std::array::from_fn(|id| (self.to[id] - self.from[id]).mul_add(alpha, self.from[id]))
    }
}

fn stages(map: &Map) -> [f64; BODY_LIMIT] {
    std::array::from_fn(|id| map.body(id).map_or(0.0, |body| body.stage.value))
}

/// WGSL declarations and sampling functions, with map resources in group 3.
pub fn declarations() -> String {
    declarations_at(3)
}

/// Map shader declarations with the specified resource group.
pub fn declarations_at(group: u32) -> String {
    format!(
        "const BODY_LIMIT = {BODY_LIMIT}u;\nconst WATER_MAP_GROUP = {group}u;\n{}\n{}",
        include_str!("map.wgsl"),
        include_str!("gains.wgsl"),
    )
}

/// Map entries within the water resource group.
pub fn bindings(visibility: wgpu::ShaderStages) -> [wgpu::BindGroupLayoutEntry; 2] {
    [
        binding::texture(
            5,
            visibility,
            wgpu::TextureSampleType::Float { filterable: false },
            wgpu::TextureViewDimension::D2Array,
        ),
        binding::uniform(6, visibility, size_of::<MapRecord>() as u64),
    ]
}

impl MapGpu {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "map lattice and rows use GPU metres"
    )]
    /// Upload the bed, rows and prepared connectivity.
    /// # Errors
    /// Refuses unsupported body kinds and maps outside the device limits.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, map: &Map) -> Result<Self, WaterError> {
        use WaterError::UnsupportedMap;
        let limits = device.limits();
        let (origin, spacing, size) = map.bed().lattice();
        let mut record = MapRecord::zeroed();
        record.origin_spacing_margin = [
            origin[0] as f32,
            origin[1] as f32,
            spacing as f32,
            map.profile().margin_height as f32,
        ];
        let width = u32::try_from(size[0]).map_err(|_| UnsupportedMap("water map width"))?;
        let height = u32::try_from(size[1]).map_err(|_| UnsupportedMap("water map height"))?;
        record.size_count_sea = [width, height, 0, u32::MAX];
        record.swash[0] = map.profile().swash_ratio as f32;
        let mut layers = 1;
        let mut tracks = std::array::from_fn(|_| None);
        for id in 0..=BODY_LIMIT {
            let Some(body) = map.body(id) else { break };
            let destination = record
                .bodies
                .get_mut(id)
                .ok_or(UnsupportedMap("water map body limit exceeded"))?;
            record.size_count_sea[2] += 1;
            destination.layers[1] = layers;
            layers += 1;
            match &body.kind {
                BodyKind::Sea => record.size_count_sea[3] = id as u32,
                BodyKind::Basin { .. } => destination.layers[0] = 1,
                BodyKind::Chart(chart) if map.river_of(id).is_none() => {
                    destination.layers[0] = 2;
                    destination.layers[2] = layers;
                    layers += 2;
                    // A chart that joins another moving chart needs the
                    // neighbour's own phase coordinate through that join.
                    let still_joins = chart.ends.iter().filter(|end| !end.is_brink()).all(|end| {
                        !matches!(
                            map.body(end.body).map(|body| &body.kind),
                            Some(BodyKind::Chart(_))
                        )
                    });
                    if still_joins && let Some(track) = CurrentTrack::prepare(chart, map.bed()) {
                        destination.track[0] = track.reference as f32;
                        destination.track[1] = if track.axis == 0 { 1.0 } else { 2.0 };
                        tracks[id] = Some(track);
                        layers += 1;
                    }
                    for (n, end) in chart.ends.iter().filter(|end| !end.is_brink()).enumerate() {
                        let segment = destination
                            .ends
                            .get_mut(n)
                            .ok_or(UnsupportedMap("water chart end limit exceeded"))?;
                        *segment = [
                            end.segment[0][0] as f32,
                            end.segment[0][1] as f32,
                            end.segment[1][0] as f32,
                            end.segment[1][1] as f32,
                        ];
                        destination.joins[n] = [
                            end.body as f32,
                            end.length as f32,
                            f32::from(end.mouth),
                            0.0,
                        ];
                        destination.layers[3] += 1;
                    }
                }
                _ => {
                    return Err(UnsupportedMap(
                        "water GPU map needs sea, basins or steady charts",
                    ));
                }
            }
        }
        if width.max(height) > limits.max_texture_dimension_2d
            || layers > limits.max_texture_array_layers
            || size_of::<MapRecord>() as u64 > limits.max_uniform_buffer_binding_size
            || record.origin_spacing_margin.iter().any(|n| !n.is_finite())
            || record.origin_spacing_margin[2] <= 0.0
            || !record.swash[0].is_finite()
        {
            return Err(UnsupportedMap(
                "water map exceeds the device or f32 lattice limits",
            ));
        }
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("water map lattice"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let texels = map.bed().heights().len();
        let mut support = vec![[0.0; 4]; texels];
        for (pixel, &height) in support.iter_mut().zip(map.bed().heights()) {
            pixel[0] = height as f32;
        }
        write_layer(queue, &texture, 0, &support);
        for (id, body) in record
            .bodies
            .iter()
            .enumerate()
            .take(record.size_count_sea[2] as usize)
        {
            if let Some(BodyKind::Chart(chart)) = map.body(id).map(|b| &b.kind) {
                for (i, pixel) in support.iter_mut().enumerate() {
                    *pixel = [
                        chart.levels[i] as f32,
                        chart.currents[i][0] as f32,
                        chart.currents[i][1] as f32,
                        f32::from(chart.footprint[i]),
                    ];
                }
                write_layer(queue, &texture, body.layers[2], &support);
                if let Some(track) = &tracks[id] {
                    for (i, pixel) in support.iter_mut().enumerate() {
                        let section = if track.axis == 0 {
                            i % size[0]
                        } else {
                            i / size[0]
                        };
                        let [coordinate, speed] = track.at(section);
                        *pixel = [coordinate as f32, speed as f32, 0.0, 0.0];
                    }
                    write_layer(queue, &texture, body.layers[2] + 2, &support);
                }
            }
        }
        let info = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water map records"),
            contents: bytemuck::bytes_of(&record),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let mut gpu = Self {
            texture,
            info,
            revision: None,
            uploaded_stages: [0.0; BODY_LIMIT],
            sampled_map: None,
            recorded: None,
            submitted: None,
            previous_levels: None,
            record,
            tracks,
            support,
        };
        let authored = stages(map);
        gpu.upload(queue, map, authored, authored);
        Ok(gpu)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped chart lattice in GPU metres"
    )]
    pub(crate) fn wave_track(
        &self,
        body: usize,
        world: [f64; 2],
        current: [f64; 2],
        bed: &thrum_water_model::Bed,
    ) -> [f32; 4] {
        let Some(track) = self.tracks.get(body).and_then(Option::as_ref) else {
            return [0.0; 4];
        };
        let (origin, spacing, size) = bed.lattice();
        let axis = track.axis;
        let grid = ((world[axis] - origin[axis]) / spacing).clamp(
            0.0,
            f64::from(u32::try_from(size[axis] - 1).expect("GPU lattice")),
        );
        let first = grid.floor() as usize;
        let next = (first + 1).min(size[axis] - 1);
        let [coordinate, speed] = track.at(first);
        let [next_coordinate, next_speed] = track.at(next);
        let alpha = grid.fract();
        let coordinate = (next_coordinate - coordinate).mul_add(alpha, coordinate);
        let speed = (next_speed - speed).mul_add(alpha, speed);
        let blend = (current[axis].abs() / speed).clamp(0.0, 1.0);
        [
            coordinate as f32,
            track.reference as f32,
            blend as f32,
            if axis == 0 { 1.0 } else { 2.0 },
        ]
    }

    /// View over the map's layers, using explicit array dimensionality.
    pub fn view(&self) -> wgpu::TextureView {
        self.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
    }

    /// Map resources within the water group. Supply a texture view from [Self::view].
    pub fn entries<'a>(&'a self, view: &'a wgpu::TextureView) -> [wgpu::BindGroupEntry<'a>; 2] {
        [
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: self.info.as_entire_binding(),
            },
        ]
    }

    pub(crate) fn record<'a>(
        &'a mut self,
        queue: &wgpu::Queue,
        map: &'a Map,
        time: EffectTime,
    ) -> &'a Map {
        let authored = stages(map);
        let recorded = StageEndpoints::record(time, authored, self.submitted.as_ref());
        self.upload(queue, map, authored, recorded.stages());
        self.recorded = Some(recorded);
        self.sampled_map.as_ref().unwrap_or(map)
    }

    pub(crate) fn record_query(&mut self, queue: &wgpu::Queue, map: &Map, frame: &Self) {
        let authored = stages(map);
        let displayed = frame
            .submitted
            .map_or(authored, |endpoint| endpoint.stages());
        self.upload(queue, map, authored, displayed);
    }

    pub(crate) fn commit(&mut self) {
        self.submitted = self.recorded;
        self.previous_levels = Some(self.record.bodies.map(|b| b.level[0] + b.level[1]));
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "prepared water rows are read in GPU metres"
    )]
    fn upload(
        &mut self,
        queue: &wgpu::Queue,
        map: &Map,
        authored: [f64; BODY_LIMIT],
        displayed: [f64; BODY_LIMIT],
    ) {
        let revision = map.spatial_revision();
        let displayed_bits = displayed.map(f64::to_bits);
        let uploaded_bits = self.uploaded_stages.map(f64::to_bits);
        let changed = self.revision != Some(revision) || uploaded_bits != displayed_bits;
        if changed {
            self.sampled_map = (authored.map(f64::to_bits) != displayed_bits).then(|| {
                let count = self.record.size_count_sea[2] as usize;
                map.sample_stages(&displayed[..count])
                    .expect("bounded tick stages")
            });
        }
        let map = self.sampled_map.as_ref().unwrap_or(map);
        let mut write = changed;
        for id in 0..self.record.size_count_sea[2] as usize {
            let body = map.body(id).expect("prepared body");
            let record = &mut self.record.bodies[id];
            let chart = record.layers[0] == 2;
            let level = [body.level as f32, body.stage.value as f32];
            let spatial = self.revision.is_none()
                || uploaded_bits[id] != displayed_bits[id]
                || record.level[0].to_bits() != level[0].to_bits()
                || chart && changed;
            if changed {
                record.level[..2].copy_from_slice(&level);
                record.gains = body.gains.map(|g| g as f32);
            }
            if spatial {
                let origins = map.support_origins(id);
                for (pixel, origin) in self.support.iter_mut().zip(origins) {
                    *pixel = origin.map_or([0.0; 4], |i| {
                        if chart {
                            let row = map.row(id, map.bed().point(i));
                            [
                                row.level as f32,
                                row.current[0] as f32,
                                row.current[1] as f32,
                                1.0,
                            ]
                        } else {
                            [0.0, 0.0, 0.0, 1.0]
                        }
                    });
                }
                write_layer(queue, &self.texture, record.layers[1], &self.support);
                if chart {
                    for (pixel, origin) in self.support.iter_mut().zip(origins) {
                        *pixel = origin.map_or([0.0; 4], |i| {
                            map.row(id, map.bed().point(i)).gains.map(|g| g as f32)
                        });
                    }
                    write_layer(queue, &self.texture, record.layers[2] + 1, &self.support);
                }
            }
            let old = self
                .previous_levels
                .map_or(record.level[0] + record.level[1], |p| p[id]);
            write |= record.level[2].to_bits() != old.to_bits();
            record.level[2] = old;
        }
        if write {
            queue.write_buffer(&self.info, 0, bytemuck::bytes_of(&self.record));
        }
        self.revision = Some(revision);
        self.uploaded_stages = displayed;
    }
}

fn write_layer(queue: &wgpu::Queue, texture: &wgpu::Texture, layer: u32, data: &[[f32; 4]]) {
    let mut size = texture.size();
    size.depth_or_array_layers = 1;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(16 * size.width),
            rows_per_image: Some(size.height),
        },
        size,
    );
}
