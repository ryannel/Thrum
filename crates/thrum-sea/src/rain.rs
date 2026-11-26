//! Filtered rain fields, regenerated from the model's scheduled drops. No pass reads
//! the output of an earlier frame.
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_water_model::{Drop, RainBands, RingTable};
use wgpu::util::DeviceExt;

pub(super) const SIZE: u32 = 512;
const TILES: i32 = 64;
const TILES_PER_PATCH: usize = (TILES * TILES) as usize;
const HEADS: usize = 2 * TILES_PER_PATCH + 1;
pub(super) const LAYERS: u32 = 4;
/// First of the four rain planes in the ambient cascade array.
pub const FIRST_LAYER: u32 = super::filtered::CASCADE_PLANES;
/// Total layers of the cascade array with rain enabled.
pub const SHARED_LAYERS: u32 = FIRST_LAYER + LAYERS;
const MIPS: u32 = SIZE.ilog2() + 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// Timing name for evaluating the two ring fields.
pub const PASS: &str = "rain bands";
/// Number of compute passes for the field and its filtered moments.
pub const PASS_COUNT: usize = MIPS as usize;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BandFrame {
    lengths_reach: [f32; 4],
    table: [u32; 4],
    steps: [f32; 4],
    scales: [f32; 4],
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
}

fn shader_source() -> String {
    format!(
        "const RAIN_SIZE = {SIZE}u;\nconst RAIN_TILES = {TILES}u;\n{}",
        include_str!("rain.wgsl")
    )
}

struct Dispatch {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    groups: [u32; 3],
}

/// Two periodic textures and two moment layers in one mipmapped wave field.
pub struct Bands {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    base_layer: u32,
    drops: wgpu::Buffer,
    tiles: wgpu::Buffer,
    table: RingTexture,
    immutable_bytes: u64,
    passes: Vec<Dispatch>,
    drop_records: Vec<[f32; 4]>,
    schedule: Vec<Drop>,
    tile_lists: Vec<Vec<u32>>,
    tile_records: Vec<u32>,
    counts: [usize; 2],
}

impl Bands {
    /// Prepare the immutable ring table and the frame's working storage.
    ///
    /// # Errors
    /// Refuses profiles whose working storage exceeds the device's storage binding limit.
    pub fn new(device: &DeviceSite, rain: &RainBands) -> Result<Self, super::AmbientError> {
        Self::prepare(device, rain, None)
    }

    /// Prepare rain in the last four layers of an ambient cascade array.
    /// # Errors
    /// Refuses an incompatible array or a profile beyond the device's limits.
    pub fn in_cascades(
        device: &DeviceSite,
        rain: &RainBands,
        array: &wgpu::Texture,
    ) -> Result<Self, super::AmbientError> {
        if array.width() != SIZE
            || array.height() != SIZE
            || array.depth_or_array_layers() != SHARED_LAYERS
            || array.mip_level_count() != MIPS
            || array.format() != FORMAT
        {
            return Err(super::AmbientError::DeviceLimit {
                resource: "shared wind and rain cascade array".into(),
                limit: "rain array layout",
            });
        }
        Self::prepare(device, rain, Some(array))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "prepare field, pack and reduction pipeline in binding order"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated ring samples and default patch parameters use f32"
    )]
    fn prepare(
        device: &DeviceSite,
        rain: &RainBands,
        cascades: Option<&wgpu::Texture>,
    ) -> Result<Self, super::AmbientError> {
        let profile = rain.profile();
        let ring = rain.table();
        let limits = device.limits();
        let bytes_limit = limits.max_storage_buffer_binding_size;
        // The model inverts a Poisson count, capped at 64 drops per cell/bin. Bound
        // the same inversion using its largest representable uniform variate.
        let bound = |side: f64| {
            let cell = side / profile.cells as f64;
            let lambda = 0.5 * profile.count * cell * cell * profile.bin;
            let mut term = (-lambda).exp();
            let mut cdf = term;
            for count in 0..64 {
                if cdf >= 1.0 - f64::EPSILON / 2.0 {
                    return count;
                }
                term *= lambda / f64::from(count + 1);
                cdf += term;
            }
            64
        };
        let mut max_drops = 0.0;
        let mut max_indices = HEADS as f64;
        for side in profile.patches {
            let bins = (ring.life() / profile.bin).ceil() + 1.0;
            let drops = bins * (profile.cells as f64).powi(2) * f64::from(bound(side));
            let span = (2.0 * (ring.reach() + 1e-6) * f64::from(TILES) / side).ceil() + 1.0;
            max_drops += drops;
            max_indices += drops * span.powi(2);
        }
        // A source near a corner can supply up to four periodic images.
        let images_bytes = max_drops * 4.0 * 16.0;
        if images_bytes > bytes_limit as f64 || max_indices * 4.0 > bytes_limit as f64 {
            return Err(super::AmbientError::DeviceLimit {
                resource: "rain bands".into(),
                limit: "max_storage_buffer_binding_size",
            });
        }

        if ring.radii.max(ring.ages) > limits.max_texture_dimension_2d as usize
            || ring.classes.len() > (limits.max_texture_array_layers as usize).min(64)
        {
            return Err(super::AmbientError::DeviceLimit {
                resource: "rain ring lookup".into(),
                limit: "ring texture extent or array layers",
            });
        }

        let texture = cascades.cloned().unwrap_or_else(|| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(PASS),
                size: wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: LAYERS,
                },
                mip_level_count: MIPS,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        });
        let base_layer = texture.depth_or_array_layers() - LAYERS;
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            base_array_layer: base_layer,
            array_layer_count: Some(LAYERS),
            ..Default::default()
        });
        let buffer = |label, data, usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage,
            })
        };
        let lengths = profile.patches.map(|x| x as f32);
        let (table, scales) = RingTexture::new(device, ring);
        let table_view = table.texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let ring_filter = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rain ring interpolator"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let coordinates = BandFrame {
            lengths_reach: [lengths[0], lengths[1], ring.reach() as f32, 0.0],
            table: [ring.radii as u32, ring.ages as u32, 0, 0],
            steps: [ring.radius_step as f32, ring.age_step as f32, 0.0, 0.0],
            scales,
        };
        let frame = buffer(
            "rain table coordinates",
            bytemuck::bytes_of(&coordinates),
            wgpu::BufferUsages::UNIFORM,
        );
        let working = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        #[expect(clippy::cast_sign_loss, reason = "allocation bounds are nonnegative")]
        let drops = working("rain drops", images_bytes.max(16.0) as u64);
        #[expect(clippy::cast_sign_loss, reason = "allocation bounds are nonnegative")]
        let tiles = working("rain tile lists", max_indices as u64 * 4);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(shader_source().into()),
        });
        let compute = wgpu::ShaderStages::COMPUTE;
        let storage = |n| binding::storage(n, compute, true, 0);
        let output = |n| {
            binding::storage_texture(
                n,
                compute,
                wgpu::StorageTextureAccess::WriteOnly,
                FORMAT,
                wgpu::TextureViewDimension::D2Array,
            )
        };
        let pack_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries: &[
                binding::uniform(0, compute, 0),
                binding::texture(
                    1,
                    compute,
                    wgpu::TextureSampleType::Float { filterable: true },
                    wgpu::TextureViewDimension::D2Array,
                ),
                storage(2),
                storage(3),
                output(4),
                binding::sampler(7, compute, wgpu::SamplerBindingType::Filtering),
            ],
        });
        let reduce_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rain mip"),
            entries: &[
                binding::texture(
                    5,
                    compute,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2Array,
                ),
                output(6),
            ],
        });
        let pipeline = |layout, entry| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(entry),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let pack = pipeline(&pack_layout, "rain_bands");
        let reduce = pipeline(&reduce_layout, "rain_mip");
        let mip_view = |mip| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(1),
                base_array_layer: base_layer,
                array_layer_count: Some(LAYERS),
                ..Default::default()
            })
        };
        let base = mip_view(0);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &pack_layout,
            entries: &[
                entry(0, frame.as_entire_binding()),
                entry(1, wgpu::BindingResource::TextureView(&table_view)),
                entry(2, drops.as_entire_binding()),
                entry(3, tiles.as_entire_binding()),
                entry(4, wgpu::BindingResource::TextureView(&base)),
                entry(7, wgpu::BindingResource::Sampler(&ring_filter)),
            ],
        });
        let mut passes = vec![Dispatch {
            pipeline: pack,
            group,
            groups: [SIZE / 8, SIZE / 8, 2],
        }];
        for mip in 1..MIPS {
            let parent = mip_view(mip - 1);
            let child = mip_view(mip);
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rain mip"),
                layout: &reduce_layout,
                entries: &[
                    entry(5, wgpu::BindingResource::TextureView(&parent)),
                    entry(6, wgpu::BindingResource::TextureView(&child)),
                ],
            });
            let groups = (SIZE >> mip).div_ceil(8);
            passes.push(Dispatch {
                pipeline: reduce.clone(),
                group,
                groups: [groups, groups, LAYERS],
            });
        }
        Ok(Self {
            texture,
            view,
            base_layer,
            drops,
            tiles,
            immutable_bytes: frame.size() + table.bytes(),
            table,
            passes,
            drop_records: Vec::new(),
            schedule: Vec::new(),
            tile_lists: vec![Vec::new(); HEADS - 1],
            tile_records: Vec::new(),
            counts: [0; 2],
        })
    }

    /// Record the drops alive at this time and bin them by their conservative tile bounds.
    ///
    /// # Panics
    /// Panics if an input exceeds the working capacity admitted at preparation.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated tile bounds and GPU world metres"
    )]
    pub fn record(&mut self, queue: &wgpu::Queue, rain: &RainBands, time: f64) {
        self.table.upload(queue);
        self.counts = [0; 2];
        self.drop_records.clear();
        for list in &mut self.tile_lists {
            list.clear();
        }
        if rain.active(time) {
            // Include an f32 coordinate guard at shared tile edges.
            let reach = rain.table().reach() + 1e-6;
            for (band, side) in rain.profile().patches.into_iter().enumerate() {
                let scale = f64::from(TILES) / side;
                rain.drops_into(band, time, &mut self.schedule);
                for drop in self.schedule.iter().filter(|drop| drop.weight > 0.0) {
                    self.counts[band] += 1;
                    let images = |p: f64| {
                        // Ring reach is below half of the period, so at most
                        // one neighbour crosses the boundary in each axis.
                        let shift = if p < reach {
                            side
                        } else if p > side - reach {
                            -side
                        } else {
                            0.0
                        };
                        [Some(p), (shift != 0.0).then_some(p + shift)]
                    };
                    let [x, z] = drop.place.map(images);
                    let data = [
                        (time - drop.time) as f32,
                        drop.class as f32 + 0.5 * drop.weight as f32,
                    ];
                    let first = band * TILES_PER_PATCH;
                    let lists = &mut self.tile_lists[first..first + TILES_PER_PATCH];
                    for z in z.into_iter().flatten() {
                        for x in x.into_iter().flatten() {
                            bin_image(&mut self.drop_records, lists, [x, z], data, scale, reach);
                        }
                    }
                }
            }
        }
        self.tile_records.clear();
        self.tile_records.resize(HEADS, 0);
        for (tile, list) in self.tile_lists.iter().enumerate() {
            self.tile_records[tile] =
                u32::try_from(self.tile_records.len()).expect("bounded tile list");
            self.tile_records.extend(list);
        }
        self.tile_records[HEADS - 1] =
            u32::try_from(self.tile_records.len()).expect("bounded tile list");
        queue.write_buffer(&self.tiles, 0, bytemuck::cast_slice(&self.tile_records));
        if !self.drop_records.is_empty() {
            queue.write_buffer(&self.drops, 0, bytemuck::cast_slice(&self.drop_records));
        }
    }

    /// Number of drops supplied to each band at the recorded time.
    pub fn drop_counts(&self) -> [usize; 2] {
        self.counts
    }

    /// The four sample layers (height/slope/rate and moments per patch).
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Encode one pass of the wave field.
    pub fn encode(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        let dispatch = &self.passes[index];
        pass.set_pipeline(&dispatch.pipeline);
        pass.set_bind_group(0, &dispatch.group, &[]);
        let [x, y, z] = dispatch.groups;
        pass.dispatch_workgroups(x, y, z);
    }

    /// Append the rain passes and return the field's surface reads.
    pub fn declare(&self, graph: &mut EffectGraph) -> Vec<EffectAccess> {
        let resource = if self.base_layer == 0 {
            let resource = graph.resources.len();
            graph.resources.push((
                ResourceDescriptor::of_texture(&self.texture),
                ResourceScope::Persistent {
                    owner: 0x7261_696e,
                    epoch: 0,
                    initialized: true,
                },
            ));
            resource
        } else {
            super::Ambient::cascade_resource()
        };
        let table = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.table.texture),
            ResourceScope::Imported { initialized: true },
        ));
        let access = |mip, kind| {
            let start =
                mip * u64::from(self.texture.depth_or_array_layers()) + u64::from(self.base_layer);
            EffectAccess {
                resource,
                range: start..start + u64::from(LAYERS),
                kind,
                view: Some(wgpu::TextureViewDimension::D2Array),
                history: None,
            }
        };
        for mip in 0..u64::from(MIPS) {
            let mut accesses = vec![access(mip, AccessKind::StorageWrite)];
            if mip == 0 {
                accesses.push(EffectAccess {
                    resource: table,
                    range: 0..u64::from(self.table.texture.depth_or_array_layers()),
                    kind: AccessKind::SampledRead,
                    view: Some(wgpu::TextureViewDimension::D2Array),
                    history: None,
                });
            } else {
                accesses.push(access(mip - 1, AccessKind::SampledRead));
            }
            graph.passes.push(EffectPass {
                name: if mip == 0 {
                    PASS.into()
                } else {
                    format!("rain mip {mip}")
                },
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses,
            });
        }
        (0..u64::from(MIPS))
            .map(|mip| access(mip, AccessKind::SampledRead))
            .collect()
    }

    /// Allocated bytes for the rain working field and drop lists.
    pub fn bytes(&self) -> u64 {
        let texture = if self.base_layer == 0 {
            (0..MIPS)
                .map(|mip| u64::from((SIZE >> mip).pow(2) * LAYERS * 8))
                .sum::<u64>()
        } else {
            0
        };
        texture + self.drops.size() + self.tiles.size() + self.immutable_bytes
    }

    /// The prepared field, available for readback by the conformance witness.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

struct RingTexture {
    texture: wgpu::Texture,
    upload: Option<Vec<[u16; 4]>>,
}

impl RingTexture {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "preparation validated dimensions and GPU units"
    )]
    fn new(device: &DeviceSite, table: &RingTable) -> (Self, [f32; 4]) {
        let mut maximum = [0.0_f64; 4];
        for class in &table.classes {
            for (channel, values) in [&class.height, &class.slope, &class.rate]
                .into_iter()
                .enumerate()
            {
                for value in values {
                    maximum[channel] = maximum[channel].max(value.abs());
                }
            }
        }
        let normalized = |value: f64, channel: usize| {
            if maximum[channel] == 0.0 {
                0
            } else {
                half::f16::from_f64(value / maximum[channel]).to_bits()
            }
        };
        let samples = table
            .classes
            .iter()
            .flat_map(|class| {
                class
                    .height
                    .iter()
                    .zip(&class.slope)
                    .zip(&class.rate)
                    .map(|((&h, &s), &dt)| {
                        [normalized(h, 0), normalized(s, 1), normalized(dt, 2), 0]
                    })
            })
            .collect();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rain ring lookup"),
            size: wgpu::Extent3d {
                width: table.radii as u32,
                height: table.ages as u32,
                depth_or_array_layers: table.classes.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        (
            Self {
                texture,
                upload: Some(samples),
            },
            maximum.map(|value| value as f32),
        )
    }

    fn upload(&mut self, queue: &wgpu::Queue) {
        if let Some(samples) = self.upload.take() {
            let size = self.texture.size();
            queue.write_texture(
                self.texture.as_image_copy(),
                bytemuck::cast_slice(&samples),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(size.width * 8),
                    rows_per_image: Some(size.height),
                },
                size,
            );
        }
    }

    fn bytes(&self) -> u64 {
        let size = self.texture.size();
        u64::from(size.width) * u64::from(size.height) * u64::from(size.depth_or_array_layers) * 8
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "bounded GPU coordinates and tile indices"
)]
fn bin_image(
    records: &mut Vec<[f32; 4]>,
    lists: &mut [Vec<u32>],
    [x, z]: [f64; 2],
    [age, class_weight]: [f32; 2],
    scale: f64,
    reach: f64,
) {
    let index = u32::try_from(records.len()).expect("bounded rain drops");
    records.push([x as f32, z as f32, age, class_weight]);
    let bounds = |p: f64| {
        let lo = ((p - reach) * scale).floor() as i32;
        let hi = ((p + reach) * scale).floor() as i32;
        lo.max(0)..=hi.min(TILES - 1)
    };
    let tile_width = scale.recip();
    let distance = |p: f64, cell: i32| {
        let edge = f64::from(cell) * tile_width;
        (edge - p).max(p - edge - tile_width).max(0.0)
    };
    for row in bounds(z) {
        let dz = distance(z, row);
        for col in bounds(x) {
            let dx = distance(x, col);
            if dx * dx + dz * dz <= reach * reach {
                let tile = usize::try_from(row * TILES + col).expect("bounded tile");
                lists[tile].push(index);
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{BandFrame, shader_source};
    use thrum_test_support::{record, records};

    #[test]
    fn band_coordinates_match_the_shader_layout() {
        records::named(
            "rain bands",
            &shader_source(),
            &[record!(BandFrame, lengths_reach, table, steps, scales)],
        );
    }
}
