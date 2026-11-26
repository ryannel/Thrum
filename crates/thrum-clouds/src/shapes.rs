use std::sync::Arc;

use thrum_asset_gpu::{LentProduct, ProductLease};
use thrum_cloud_model::{Directed, Field, PlacedLibrary, ShapeSource, State, Type};
use thrum_render::{binding, runtime::DeviceSite};
use wgpu::util::DeviceExt;

use crate::{Error, WeatherMap};
pub(crate) mod map;
#[cfg(test)]
mod tests;

const MAX_INSTANCES: usize = 1024;
const MAX_BLOCKS: usize = 4096;
const MAX_VOLUMES: usize = 16;
const READ: wgpu::ShaderStages = wgpu::ShaderStages::COMPUTE;

pub(crate) fn shader(group: u32) -> String {
    format!(
        "@group({group}) @binding(0) var<storage, read> placed_instances: PlacedInstances;\n\
         @group({group}) @binding(1) var<storage, read> placed_footprints: array<vec4<f32>>;\n\
         @group({group}) @binding(2) var placed_library: texture_3d<f32>;\n\
         @group({group}) @binding(3) var placed_growth: texture_3d<f32>;\n\
         @group({group}) @binding(4) var placed_sampler: sampler;\n{}\n{}",
        include_str!("placed_volume.wgsl"),
        include_str!("shapes.wgsl"),
    )
}

/// Geometry, residency and quality controls for the optional Shapes capability.
#[derive(Clone, Debug)]
pub struct ShapesConfig {
    /// Same authored footprints the game's weather state uses.
    pub library: Arc<PlacedLibrary>,
    /// A read-only lease for the library and growth stacks, in that order.
    pub lease: ProductLease,
    /// Distance at which the radial fade to the column form starts and ends.
    pub range: [f64; 2],
    /// Maximum number of instances admitted in volume form.
    pub capacity: usize,
}

impl ShapesConfig {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.capacity > MAX_VOLUMES
            || !self.range.iter().all(|value| value.is_finite())
            || self.range[0] < 0.0
            || self.range[0] >= self.range[1]
        {
            return Err(Error::Unsupported("placed volume capacity or range"));
        }
        Ok(())
    }

    fn heights(&self, state: &State, kind: Type, pitch: f64, mut heights: [f64; 2]) -> [f64; 2] {
        for shape in state.shapes() {
            if let Some(slab) = state
                .shape_source(shape.id)
                .and_then(|source| self.library.geometry(source))
            {
                heights[0] = heights[0].min(kind.base + slab.floor * shape.scale);
                heights[1] = heights[1].max(kind.base + (slab.floor + slab.extent) * shape.scale);
            }
        }
        if let Some(placement) = state.hashed_shapes() {
            for slab in self.library.shapes().iter().chain(self.library.growth()) {
                let scale = placement.scale_limit(pitch, slab);
                heights[0] = heights[0].min(kind.base + slab.floor.min(0.0) * scale);
                heights[1] =
                    heights[1].max(kind.base + (slab.floor + slab.extent).max(0.0) * scale);
            }
        }
        heights
    }
}

/// Shape selection for the most recently submitted frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShapesReport {
    /// In-range instances admitted to the volume form.
    pub volume_instances: usize,
    /// In-range instances placed in column form because of lease or capacity.
    pub volume_refusals: usize,
    /// Placement operations the authoritative weather state refused.
    pub directed_refusals: u64,
    /// Instance and placed-weather window bytes queued by this recording, excluding cooked products.
    pub placement_upload_bytes: usize,
    /// Whether this submission dispatched the placed-weather map.
    pub placed_map_updated: bool,
    /// Whether this recording observed a pending lease change.
    pub history_invalid: bool,
    /// Lease changes consumed by advancing submissions.
    pub invalidations: u64,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    centre_yaw: [f32; 4],
    form: [f32; 4],
    geometry: [f32; 4],
    keys: [u32; 4],
    footprint: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceHeader {
    bounds: [f32; 4],
    optical: [f32; 4],
    count: [u32; 4],
    grid: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instances {
    header: InstanceHeader,
    slots: [u32; MAX_BLOCKS],
    items: [Instance; MAX_INSTANCES],
}

impl Instances {
    fn upload_changed(
        &self,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        uploaded: &mut Self,
    ) -> usize {
        // Inactive rows are unreachable through the header. Retain their last
        // uploaded values so reactivating a row can still be compared correctly.
        let slots = self.header.count[1] as usize * self.header.count[2] as usize;
        let items = self.header.count[0] as usize;
        upload_changes(
            queue,
            buffer,
            0,
            std::slice::from_ref(&self.header),
            std::slice::from_mut(&mut uploaded.header),
        ) + upload_changes(
            queue,
            buffer,
            std::mem::offset_of!(Instances, slots),
            &self.slots[..slots],
            &mut uploaded.slots[..slots],
        ) + upload_changes(
            queue,
            buffer,
            std::mem::offset_of!(Instances, items),
            &self.items[..items],
            &mut uploaded.items[..items],
        )
    }
}

type Candidate = (Directed, ShapeSource, Option<usize>);

pub(crate) struct Shapes {
    config: ShapesConfig,
    field: Field,
    device: DeviceSite,
    instances: wgpu::Buffer,
    candidates: Vec<Candidate>,
    staging: Box<Instances>,
    uploaded: Box<Instances>,
    footprints: wgpu::Buffer,
    sampler: wgpu::Sampler,
    empty: wgpu::TextureView,
    pub(crate) layout: thrum_render::runtime::PipelineInput<wgpu::BindGroupLayout>,
    pub(crate) group: wgpu::BindGroup,
    bound: [Option<(u64, u64)>; 2],
    committed: [Option<(u64, u64)>; 2],
    pending: ShapesReport,
    invalidations: u64,
    slab_side: u32,
}

impl Shapes {
    pub(crate) fn set_range(&mut self, range: [f64; 2]) -> bool {
        let changed = self.config.range.map(f64::to_bits) != range.map(f64::to_bits);
        self.config.range = range;
        changed
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "physical footprint units fit in f32"
    )]
    pub(crate) fn new(
        device: &DeviceSite,
        config: ShapesConfig,
        weather: &WeatherMap,
    ) -> Result<Self, Error> {
        use bytemuck::Zeroable;

        let slab_side = u32::try_from(config.library.shapes()[0].footprint.layout().0[0])
            .map_err(|_| Error::Unsupported("placed slab resolution"))?;
        let footprints: Vec<[f32; 4]> = config
            .library
            .shapes()
            .iter()
            .chain(config.library.growth())
            .flat_map(|slab| slab.footprint.columns())
            .map(|&[tau, low, high]| [tau as f32, low as f32, high as f32, 0.0])
            .collect();
        let bytes = bytemuck::cast_slice(&footprints);
        if bytes.len() as u64
            > device
                .limits()
                .max_storage_buffer_binding_size
                .min(device.limits().max_buffer_size)
        {
            return Err(Error::DeviceLimit("placed footprint storage"));
        }
        let footprints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cloud footprints"),
            contents: bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("placed cloud instances"),
            size: size_of::<Instances>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let empty = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("empty placed cloud stack"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D3,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("placed clouds"),
            entries: &[
                binding::storage(0, READ, true, size_of::<Instances>() as u64),
                binding::storage(1, READ, true, 16),
                binding::texture(
                    2,
                    READ,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D3,
                ),
                binding::texture(
                    3,
                    READ,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D3,
                ),
                binding::sampler(4, READ, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let group = bind(
            device,
            &layout,
            &instances,
            &footprints,
            [&empty, &empty],
            &sampler,
        );
        let field = Field::new(
            weather.seed(),
            weather.model_profile(),
            weather.types().to_vec(),
        )
        .map_err(Error::Model)?;
        Ok(Self {
            config,
            field,
            device: device.clone(),
            instances,
            candidates: Vec::new(),
            staging: Box::new(Instances::zeroed()),
            uploaded: Box::new(Instances::zeroed()),
            footprints,
            sampler,
            empty,
            layout,
            group,
            bound: [None; 2],
            committed: [None; 2],
            pending: ShapesReport::default(),
            invalidations: 0,
            slab_side,
        })
    }

    pub(crate) fn append_reads(
        &self,
        graph: &mut thrum_render::contribution::EffectGraph,
    ) -> Vec<thrum_render::contribution::EffectAccess> {
        use thrum_render::{
            contribution::EffectAccess,
            graph::{AccessKind, ResourceDescriptor, ResourceScope, TextureKind},
        };
        let mut reads = Vec::new();
        let mut add = |descriptor, size, kind| {
            let resource = graph.resources.len();
            graph
                .resources
                .push((descriptor, ResourceScope::Imported { initialized: true }));
            reads.push(EffectAccess {
                resource,
                range: 0..size,
                kind,
                history: None,
                view: None,
            });
        };
        for buffer in [&self.instances, &self.footprints] {
            add(
                ResourceDescriptor::Buffer {
                    bytes: buffer.size(),
                    usage: buffer.usage(),
                },
                buffer.size(),
                AccessKind::StorageRead,
            );
        }
        for count in [
            self.config.library.shapes().len(),
            self.config.library.growth().len(),
        ] {
            let stacks = u32::try_from(count)
                .expect("stack count bounded by cooked volume extent")
                .max(1);
            let mips = self.slab_side.ilog2() + 1;
            add(
                ResourceDescriptor::Texture {
                    dimension: TextureKind::D3 {
                        depth: self.slab_side * stacks,
                    },
                    width: self.slab_side,
                    height: self.slab_side,
                    layers: 1,
                    mips,
                    samples: 1,
                    format: wgpu::TextureFormat::R8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                },
                u64::from(mips),
                AccessKind::SampledRead,
            );
        }
        reads
    }

    pub(crate) fn resource_bytes(&self) -> Vec<(String, u64)> {
        let mut bytes = vec![
            ("cloud footprints".into(), self.footprints.size()),
            ("placed cloud instances".into(), self.instances.size()),
        ];
        if let Some(set) = self.config.lease.offered() {
            for (product, key) in set.products.iter().zip(self.bound) {
                if key == Some((product.device_generation, product.serial)) {
                    let texture = &product.texture;
                    let format = texture.format();
                    let (bx, by) = format.block_dimensions();
                    let block_bytes = u64::from(format.block_copy_size(None).unwrap_or(0));
                    let size = (0..texture.mip_level_count())
                        .map(|mip| {
                            u64::from((texture.width() >> mip).max(1).div_ceil(bx))
                                * u64::from((texture.height() >> mip).max(1).div_ceil(by))
                                * u64::from((texture.depth_or_array_layers() >> mip).max(1))
                                * block_bytes
                        })
                        .sum();
                    bytes.push((format!("{} {:?}", product.name, format), size));
                }
            }
        }
        bytes
    }

    pub(crate) fn heights(
        &self,
        state: &State,
        kind: Type,
        pitch: f64,
        heights: [f64; 2],
    ) -> [f64; 2] {
        self.config.heights(state, kind, pitch, heights)
    }

    fn offer(&mut self, generation: u64) {
        let set = self.config.lease.offered();
        let counts = [
            self.config.library.shapes().len(),
            self.config.library.growth().len(),
        ];
        let products: [Option<&LentProduct>; 2] = std::array::from_fn(|index| {
            set.as_ref()?.products.get(index).filter(|product| {
                let texture = &product.texture;
                product.device_generation == generation
                    && texture.dimension() == wgpu::TextureDimension::D3
                    && texture.width() == self.slab_side
                    && texture.height() == self.slab_side
                    && texture.depth_or_array_layers() as usize
                        == self.slab_side as usize * counts[index]
            })
        });
        let keys = products.map(|product| product.map(|p| (p.device_generation, p.serial)));
        if keys != self.bound {
            self.bound = keys;
            self.group = bind(
                &self.device,
                &self.layout,
                &self.instances,
                &self.footprints,
                products.map(|product| product.map_or(&self.empty, |p| &p.view)),
                &self.sampler,
            );
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "placed addresses and camera-relative distances are profile-bounded"
    )]
    pub(crate) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        generation: u64,
        state: &State,
        kind: Type,
        weather: &WeatherMap,
        camera: [f64; 2],
    ) -> Result<usize, Error> {
        use bytemuck::Zeroable;
        self.offer(generation);
        let data = &mut self.staging;
        data.header = InstanceHeader::zeroed();
        let origin = weather.origins()[0];
        data.header.optical = [
            kind.extinction() as f32,
            kind.base as f32,
            kind.profile[0] as f32,
            kind.profile[1] as f32,
        ];
        let mut bounds = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        let mut pending = ShapesReport {
            directed_refusals: state.refusals(),
            history_invalid: self.bound != self.committed,
            invalidations: self.invalidations,
            ..Default::default()
        };
        let travel = state.travel();
        let distance = |centre: [f64; 2]| {
            (centre[0] + travel[0] - camera[0]).hypot(centre[1] + travel[1] - camera[1])
        };
        let active = &mut self.candidates;
        Self::collect_candidates(&self.field, state, weather, data, active)?;
        active.sort_unstable_by(|(a, _, _), (b, _, _)| {
            distance(a.centre)
                .total_cmp(&distance(b.centre))
                .then(a.id.cmp(&b.id))
        });
        for (shape, source, block) in active.drain(..) {
            let Some(slab) = self.config.library.geometry(source) else {
                continue;
            };
            let stack = usize::from(source == ShapeSource::Growth);
            let weight = 1.0 - smooth_range(distance(shape.centre), self.config.range);
            let admitted = weight > 0.0
                && pending.volume_instances < self.config.capacity
                && self.bound[stack].is_some();
            if admitted {
                pending.volume_instances += 1;
            } else if weight > 0.0 {
                pending.volume_refusals += 1;
            }
            let centre = std::array::from_fn::<_, 2, _>(|i| shape.centre[i] - origin[i]);
            let radius = slab.footprint.radius() * shape.scale;
            for i in 0..2 {
                bounds[i] = bounds[i].min(centre[i] - radius);
                bounds[i + 2] = bounds[i + 2].max(centre[i] + radius);
            }
            let (sin, cos) = shape.yaw.sin_cos();
            let (keys, blend) = self.config.library.keys(source, shape.growth);
            let offset = if stack == 0 {
                0
            } else {
                u32::try_from(self.config.library.shapes().len()).expect("volume stack size")
            };
            let start = keys.map(|key| (offset + key) * self.slab_side * self.slab_side);
            data.items[data.header.count[0] as usize] = Instance {
                centre_yaw: [centre[0] as f32, centre[1] as f32, cos as f32, sin as f32],
                form: [
                    shape.scale as f32,
                    shape.envelope as f32,
                    if admitted { weight as f32 } else { 0.0 },
                    blend as f32,
                ],
                geometry: [slab.extent as f32, slab.floor as f32, radius as f32, 0.0],
                keys: [keys[0], keys[1], start[0], start[1]],
                footprint: [self.slab_side, u32::from(stack != 0), 0, 0],
            };
            data.header.count[0] += 1;
            if let Some(block) = block {
                data.slots[block] = data.header.count[0];
            }
        }
        if data.header.count[0] > 0 {
            data.header.bounds = bounds.map(|value| value as f32);
        }
        let bytes = data.upload_changed(queue, &self.instances, &mut self.uploaded);
        pending.placement_upload_bytes = bytes;
        self.pending = pending;
        Ok(bytes)
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "block grid uses floor coordinates relative to the weather anchor"
    )]
    fn collect_candidates(
        field: &Field,
        state: &State,
        weather: &WeatherMap,
        data: &mut Instances,
        active: &mut Vec<Candidate>,
    ) -> Result<(), Error> {
        active.clear();
        let Some(block_width) = field.placed_block_width(state) else {
            active.extend(
                state
                    .shapes()
                    .iter()
                    .filter(|shape| shape.envelope > 0.0)
                    .filter_map(|shape| Some((shape.clone(), state.shape_source(shape.id)?, None))),
            );
            if active.len() > MAX_INSTANCES {
                return Err(Error::DeviceLimit("directed column-form instances"));
            }
            return Ok(());
        };
        let outer = weather.origins()[3];
        let span = weather.profile().texels[3] * f64::from(weather.profile().size);
        let bounds = outer.map(|point| {
            [
                (point / block_width).floor() as i64,
                ((point + span) / block_width).ceil() as i64,
            ]
        });
        let dimensions =
            bounds.map(|[start, end]| usize::try_from(end - start).unwrap_or(usize::MAX));
        if dimensions[0].saturating_mul(dimensions[1]) > MAX_BLOCKS || dimensions.contains(&0) {
            return Err(Error::DeviceLimit("placed block coverage"));
        }
        let origin = weather.origins()[0];
        data.header.grid = [
            (bounds[0][0] as f64 * block_width - origin[0]) as f32,
            (bounds[1][0] as f64 * block_width - origin[1]) as f32,
            block_width as f32,
            0.0,
        ];
        data.header.count[1] = u32::try_from(dimensions[0]).expect("bounded grid width");
        data.header.count[2] = u32::try_from(dimensions[1]).expect("bounded grid height");
        data.slots[..dimensions[0] * dimensions[1]].fill(0);
        for z in bounds[1][0]..bounds[1][1] {
            for x in bounds[0][0]..bounds[0][1] {
                if let Some((shape, source)) = field.hashed_shape([x, z], state) {
                    let column = usize::try_from(x - bounds[0][0]).expect("within block bounds");
                    let row = usize::try_from(z - bounds[1][0]).expect("within block bounds");
                    active.push((shape, source, Some(row * dimensions[0] + column)));
                }
            }
        }
        if active.len() > MAX_INSTANCES {
            return Err(Error::DeviceLimit("placed column-form instances"));
        }
        Ok(())
    }

    pub(crate) fn history_invalid(&self) -> bool {
        self.pending.history_invalid
    }

    pub(crate) fn submitted(&mut self, advances: bool) -> ShapesReport {
        if advances && self.pending.history_invalid {
            self.committed = self.bound;
            self.invalidations += 1;
        }
        ShapesReport {
            invalidations: self.invalidations,
            ..self.pending
        }
    }
}

fn upload_changes<T: bytemuck::Pod>(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: usize,
    staging: &[T],
    uploaded: &mut [T],
) -> usize {
    let mut run_start = None;
    let mut written = 0;
    for i in 0..=staging.len() {
        if i < staging.len() && bytemuck::bytes_of(&staging[i]) != bytemuck::bytes_of(&uploaded[i])
        {
            run_start.get_or_insert(i);
        } else if let Some(start) = run_start.take() {
            let bytes = bytemuck::cast_slice(&staging[start..i]);
            queue.write_buffer(buffer, (offset + start * size_of::<T>()) as u64, bytes);
            uploaded[start..i].copy_from_slice(&staging[start..i]);
            written += bytes.len();
        }
    }
    written
}

fn smooth_range(value: f64, range: [f64; 2]) -> f64 {
    let fraction = ((value - range[0]) / (range[1] - range[0])).clamp(0.0, 1.0);
    fraction * fraction * (3.0 - 2.0 * fraction)
}

fn bind(
    device: &DeviceSite,
    layout: &wgpu::BindGroupLayout,
    instances: &wgpu::Buffer,
    footprints: &wgpu::Buffer,
    stacks: [&wgpu::TextureView; 2],
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("placed cloud resources"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: instances.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: footprints.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(stacks[0]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(stacks[1]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
