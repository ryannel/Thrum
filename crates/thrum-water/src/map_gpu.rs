//! Prepared map products and the shared camera-independent WGSL sampler.
use crate::WaterError;
use bytemuck::{Pod, Zeroable};
use thrum_render::binding;
use thrum_render::contribution::EffectTime;
use thrum_water_model::{BodyKind, Gauge, Map};
use wgpu::util::DeviceExt;

mod bounded_track;
mod current_track;
mod hollows;
mod rivers;
use bounded_track::{BoundedTrack, uniform_current};
use current_track::CurrentTrack;
use hollows::HollowsGpu;

struct SharedTrack {
    layer: u32,
    track: BoundedTrack,
}

struct WaveTracks {
    groups: [usize; BODY_LIMIT],
    bounded: [Option<SharedTrack>; BODY_LIMIT],
    parallel: [Option<CurrentTrack>; BODY_LIMIT],
}

impl WaveTracks {
    fn new(map: &Map) -> Self {
        // A fully joined row can be drawn by either chart. Connected charts
        // share the same carriage even when their authored currents differ.
        let mut groups = std::array::from_fn(|id| id);
        for id in 0..BODY_LIMIT {
            if let Some(BodyKind::Chart(chart)) = map.body(id).map(|body| &body.kind) {
                for end in chart.ends.iter().filter(|end| !end.is_brink()) {
                    if matches!(
                        map.body(end.body).map(|body| &body.kind),
                        Some(BodyKind::Chart(_))
                    ) {
                        let from = groups[id];
                        let to = groups[end.body];
                        for group in &mut groups {
                            if *group == from {
                                *group = to;
                            }
                        }
                    }
                }
            }
        }
        Self {
            groups,
            bounded: std::array::from_fn(|_| None),
            parallel: std::array::from_fn(|_| None),
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "prepared chart records use GPU metres"
    )]
    fn prepare(
        &mut self,
        map: &Map,
        id: usize,
        chart: &thrum_water_model::Chart,
        layers: &mut u32,
    ) -> [f32; 4] {
        let group = self.groups[id];
        let linked = self
            .groups
            .iter()
            .enumerate()
            .any(|(member, &value)| member != id && value == group);
        if !linked && let Some(track) = CurrentTrack::prepare(chart, map.bed()) {
            let record = [
                track.reference as f32,
                if track.axis == 0 { 1.0 } else { 2.0 },
                0.0,
                0.0,
            ];
            self.parallel[id] = Some(track);
            *layers += 1;
            return record;
        }
        if !linked
            && chart.ends.iter().all(thrum_water_model::End::is_brink)
            && uniform_current(chart)
        {
            return [0.0; 4];
        }
        let shared = self.bounded[group].get_or_insert_with(|| {
            let charts: Vec<_> = self
                .groups
                .iter()
                .enumerate()
                .filter(|&(_, &value)| value == group)
                .filter_map(|(member, _)| match &map.body(member)?.kind {
                    BodyKind::Chart(chart) => Some(chart),
                    _ => None,
                })
                .collect();
            let (_, spacing, size) = map.bed().lattice();
            let layer = *layers;
            *layers += 1;
            SharedTrack {
                layer,
                track: BoundedTrack::prepare(&charts, size, spacing),
            }
        });
        [shared.layer as f32, 0.0, shared.track.period_limit, -1.0]
    }
}

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
    uploaded_stages: Stages,
    sampled_map: Option<Map>,
    recorded: Option<StageEndpoints>,
    submitted: Option<StageEndpoints>,
    previous_levels: Option<[f32; BODY_LIMIT]>,
    record: MapRecord,
    tracks: WaveTracks,
    support: Vec<[f32; 4]>,
    hollows: Option<HollowsGpu>,
    rivers: [Option<rivers::ChartLayers>; BODY_LIMIT],
}

#[derive(Clone, Copy)]
struct StageEndpoints {
    time: EffectTime,
    from: Stages,
    to: Stages,
}

impl StageEndpoints {
    fn record(time: EffectTime, current: &Stages, previous: Option<&Self>) -> Self {
        let Some(previous) =
            previous.filter(|p| p.time.epoch == time.epoch && p.time.timeline == time.timeline)
        else {
            return Self {
                time,
                from: *current,
                to: *current,
            };
        };
        if previous.time.tick == time.tick {
            // Resampling a tick keeps its endpoints, even if new edits arrived.
            return Self { time, ..*previous };
        }
        let from = if previous.time.tick.checked_add(1) == Some(time.tick) {
            previous.to
        } else {
            *current
        };
        Self {
            time,
            from,
            to: *current,
        }
    }

    fn stages(&self) -> Stages {
        let alpha = f64::from(self.time.alpha);
        if alpha >= 1.0 {
            return self.to;
        }
        Stages {
            bodies: std::array::from_fn(|id| {
                let from = self.from.bodies[id];
                (self.to.bodies[id] - from).mul_add(alpha, from)
            }),
            gauges: std::array::from_fn(|id| {
                let to = self.to.gauges[id]?;
                let from = self.from.gauges[id].unwrap_or(to);
                Some((to - from).mul_add(alpha, from))
            }),
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Stages {
    bodies: [f64; BODY_LIMIT],
    gauges: [Option<f64>; BODY_LIMIT],
}

impl Stages {
    fn same(&self, other: &Self) -> bool {
        self.bodies.map(f64::to_bits) == other.bodies.map(f64::to_bits)
            && self.gauges.map(|v| v.map(f64::to_bits)) == other.gauges.map(|v| v.map(f64::to_bits))
    }
}

fn stages(map: &Map) -> Stages {
    Stages {
        bodies: std::array::from_fn(|id| map.body(id).map_or(0.0, |body| body.stage.value)),
        gauges: std::array::from_fn(|id| match map.river(id)?.gauge {
            Gauge::Stage(stage) => Some(stage.value),
            _ => None,
        }),
    }
}

/// WGSL declarations and sampling functions, with map resources in group 3.
pub fn declarations() -> String {
    declarations_at(3)
}

/// Map shader declarations with the specified resource group.
pub fn declarations_at(group: u32) -> String {
    map_source(group, false, false)
}

fn map_source(group: u32, hollows: bool, rivers: bool) -> String {
    let sample_hollows = if hollows {
        "    let local = hollows_sample(point);\n    if local.has_bed { return local; }\n"
    } else {
        ""
    };
    // The steady specialization keeps its original source and texture layout.
    let (chart_cell, river_source) = if rivers {
        (
            "river_chart_cell(cell, k, body)",
            include_str!("map_gpu/rivers.wgsl"),
        )
    } else {
        (
            "textureLoad(bed_map, cell.corners[k], i32(body.layers.z), 0)",
            "",
        )
    };
    let source = format!(
        "const BODY_LIMIT = {BODY_LIMIT}u;\nconst WATER_MAP_GROUP = {group}u;\n{}            let raw = {chart_cell};\n{}{sample_hollows}{}\n{}{river_source}",
        include_str!("map.wgsl"),
        include_str!("map_gpu/chart_tail.wgsl"),
        include_str!("map_sample.wgsl"),
        include_str!("gains.wgsl"),
    );
    if hollows {
        source + include_str!("map_gpu/hollows.wgsl")
    } else {
        source
    }
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
        if map.body(BODY_LIMIT).is_some() {
            return Err(UnsupportedMap("water map body limit exceeded"));
        }
        if width.max(height) > limits.max_texture_dimension_2d {
            return Err(UnsupportedMap("water map exceeds the device lattice limit"));
        }
        let mut tracks = WaveTracks::new(map);
        let mut rivers = std::array::from_fn(|_| None);
        for (id, destination) in record.bodies.iter_mut().enumerate() {
            let Some(body) = map.body(id) else { break };
            record.size_count_sea[2] += 1;
            if !matches!(&body.kind, BodyKind::Hollows(_)) {
                destination.layers[1] = layers;
                layers += 1;
            }
            match &body.kind {
                BodyKind::Sea => record.size_count_sea[3] = id as u32,
                BodyKind::Basin { .. } => destination.layers[0] = 1,
                BodyKind::Hollows(_) => destination.layers[0] = 3,
                BodyKind::Chart(chart) => {
                    destination.layers[0] = 2;
                    destination.layers[2] = layers;
                    layers += 2;
                    destination.track = tracks.prepare(map, id, chart, &mut layers);
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
                    rivers[id] = rivers::ChartLayers::prepare(
                        map,
                        id,
                        &mut layers,
                        limits.max_texture_array_layers,
                    )?;
                }
            }
        }
        if layers > limits.max_texture_array_layers
            || size_of::<MapRecord>() as u64 > limits.max_uniform_buffer_binding_size
            || record.origin_spacing_margin.iter().any(|n| !n.is_finite())
            || record.origin_spacing_margin[2] <= 0.0
            || !record.swash[0].is_finite()
        {
            return Err(UnsupportedMap(
                "water map exceeds the device or f32 lattice limits",
            ));
        }
        let hollows = HollowsGpu::new(device, map)?;
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
                if let Some(samples) = &rivers[id] {
                    samples.write(queue, &texture, map, &mut support);
                }
                if let Some(track) = &tracks.parallel[id] {
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
        for shared in tracks.bounded.iter().flatten() {
            write_layer(queue, &texture, shared.layer, shared.track.cells());
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
            uploaded_stages: Stages::default(),
            sampled_map: None,
            recorded: None,
            submitted: None,
            previous_levels: None,
            record,
            tracks,
            support,
            hollows,
            rivers,
        };
        let authored = stages(map);
        gpu.upload(queue, map, &authored, &authored);
        if let Some(hollows) = &mut gpu.hollows {
            hollows.record(queue, map, None);
            hollows.commit();
        }
        Ok(gpu)
    }

    /// Map shader declarations specialized to the products that were prepared.
    pub fn source(&self, group: u32) -> String {
        map_source(
            group,
            self.hollows.is_some(),
            self.rivers.iter().any(Option::is_some),
        )
    }

    pub(crate) fn hollows_bindings(
        &self,
        visibility: wgpu::ShaderStages,
    ) -> Option<wgpu::BindGroupLayoutEntry> {
        self.hollows.as_ref().map(|h| h.binding(visibility))
    }

    /// The additional map entry when a scene declares a hollows body.
    pub fn hollows_entry(&self) -> Option<wgpu::BindGroupEntry<'_>> {
        self.hollows.as_ref().map(HollowsGpu::entry)
    }

    pub(crate) fn hollow_levels(&self) -> &[[u32; 2]] {
        self.hollows.as_ref().map_or(&[], HollowsGpu::display)
    }

    pub(crate) fn hollows_bytes(&self) -> Option<u64> {
        self.hollows.as_ref().map(HollowsGpu::bytes)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped chart lattice in GPU metres"
    )]
    fn wave_track(
        &self,
        body: usize,
        world: [f64; 2],
        current: [f64; 2],
        bed: &thrum_water_model::Bed,
    ) -> [f64; 4] {
        let mut record = self.record.bodies[body].track.map(f64::from);
        if let Some(shared) = &self.tracks.bounded[self.tracks.groups[body]] {
            let sample = shared.track.sample(world, bed);
            for axis in 0..2 {
                let authored = f64::from(sample[2 + axis]);
                let scale =
                    (authored * current[axis] / (authored * authored).max(1e-12)).clamp(0.0, 1.0);
                record[axis] = f64::from(sample[axis]) * scale;
            }
            return record;
        }
        let Some(track) = &self.tracks.parallel[body] else {
            return record;
        };
        let (origin, spacing, size) = bed.lattice();
        let axis = track.axis;
        let grid = ((world[axis] - origin[axis]) / spacing).clamp(
            0.0,
            f64::from(u32::try_from(size[axis] - 1).expect("GPU lattice")),
        );
        let first = (grid.floor() as usize).min(size[axis] - 2);
        let next = first + 1;
        let [coordinate, speed] = track.at(first);
        let [next_coordinate, next_speed] = track.at(next);
        let alpha = grid - f64::from(u32::try_from(first).expect("GPU lattice"));
        let delta = next_coordinate - coordinate;
        let coordinate = delta.mul_add(alpha, coordinate);
        let speed = (next_speed - speed).mul_add(alpha, speed);
        let blend = (current[axis].abs() / speed).clamp(0.0, 1.0);
        [
            coordinate,
            track.reference,
            blend,
            if axis == 0 { 1.0 } else { 2.0 },
        ]
    }

    /// Track and spatial derivatives for the sparse body queries. Differentiate
    /// the same joined row used for the centre, without changing its owner.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "chart derivatives use GPU metres"
    )]
    pub(crate) fn query_track(
        &self,
        body: usize,
        point: [f64; 2],
        current: [f64; 2],
        map: &Map,
    ) -> ([f32; 4], [[f32; 4]; 2]) {
        let bed = map.bed();
        let track = self.wave_track(body, point, current, bed);
        if track[3] == 0.0 {
            return (track.map(|v| v as f32), [[0.0; 4]; 2]);
        }
        // A symmetric difference also defines the derivative at cell creases.
        // Keep the interval large enough to resolve f32 track samples.
        let step = bed.lattice().1 / 128.0;
        let mut current_gradient = [0.0; 4];
        let mut track_gradient = [0.0; 4];
        for axis in 0..2 {
            let mut lo = point;
            let mut hi = point;
            lo[axis] -= step;
            hi[axis] += step;
            let a = map.supported_row(body, lo).current;
            let b = map.supported_row(body, hi).current;
            let before = self.wave_track(body, lo, a, bed);
            let after = self.wave_track(body, hi, b, bed);
            let second = if track[3] > 0.0 { 2 } else { 1 };
            for (component, index) in [0, second].into_iter().enumerate() {
                current_gradient[2 * axis + component] =
                    ((b[component] - a[component]) / (2.0 * step)) as f32;
                track_gradient[2 * axis + component] =
                    ((after[index] - before[index]) / (2.0 * step)) as f32;
            }
        }
        (track.map(|v| v as f32), [current_gradient, track_gradient])
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
        let recorded = StageEndpoints::record(time, &authored, self.submitted.as_ref());
        self.upload(queue, map, &authored, &recorded.stages());
        if let Some(hollows) = &mut self.hollows {
            hollows.record(queue, map, Some(time));
        }
        self.recorded = Some(recorded);
        self.sampled_map.as_ref().unwrap_or(map)
    }

    pub(crate) fn record_query(&mut self, queue: &wgpu::Queue, map: &Map, frame: &Self) {
        let authored = stages(map);
        let displayed = frame
            .submitted
            .map_or(authored, |endpoint| endpoint.stages());
        self.upload(queue, map, &authored, &displayed);
        if let (Some(hollows), Some(from)) = (&mut self.hollows, &frame.hollows) {
            hollows.copy_display(queue, from);
        }
    }

    pub(crate) fn commit(&mut self) {
        self.submitted = self.recorded;
        self.previous_levels = Some(self.record.bodies.map(|b| b.level[0] + b.level[1]));
        if let Some(hollows) = &mut self.hollows {
            hollows.commit();
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "prepared water rows are read in GPU metres"
    )]
    fn upload(&mut self, queue: &wgpu::Queue, map: &Map, authored: &Stages, displayed: &Stages) {
        let revision = map.spatial_revision();
        let changed = self.revision != Some(revision) || !self.uploaded_stages.same(displayed);
        if changed {
            self.sampled_map = (!authored.same(displayed)).then(|| {
                let count = self.record.size_count_sea[2] as usize;
                map.sample_stages_and_gauges(&displayed.bodies[..count], &displayed.gauges)
                    .expect("bounded tick stages")
            });
        }
        let map = self.sampled_map.as_ref().unwrap_or(map);
        let mut write = changed;
        for id in 0..self.record.size_count_sea[2] as usize {
            let body = map.body(id).expect("prepared body");
            let record = &mut self.record.bodies[id];
            if record.layers[0] == 3 {
                continue;
            }
            let chart = record.layers[0] == 2;
            let level = [body.level as f32, body.stage.value as f32];
            let spatial = self.revision.is_none()
                || self.uploaded_stages.bodies[id].to_bits() != displayed.bodies[id].to_bits()
                || record.level[0].to_bits() != level[0].to_bits()
                || chart && changed;
            if changed {
                record.level[..2].copy_from_slice(&level);
                record.gains = body.gains.map(|g| g as f32);
                if let Some(samples) = &self.rivers[id] {
                    let (layer, weight) = samples.interval(map);
                    record.level[3] = samples.first() as f32;
                    record.joins[0][3] = layer as f32;
                    record.joins[1][3] = weight;
                }
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
        self.uploaded_stages = *displayed;
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
