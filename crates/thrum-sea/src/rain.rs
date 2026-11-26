//! Filtered rain fields, regenerated from the model's scheduled drops. No pass reads
//! the output of an earlier frame.
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_water_model::RainBands;
use wgpu::util::DeviceExt;

const SIZE: u32 = 512;
const TILES: i32 = 16;
const HEADS: usize = 2 * 16 * 16 + 1;
const LAYERS: u32 = 4;
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
}

fn entry(binding: u32, resource: wgpu::BindingResource<'_>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource }
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
    drops: wgpu::Buffer,
    tiles: wgpu::Buffer,
    immutable_bytes: u64,
    passes: Vec<Dispatch>,
    drop_records: Vec<[f32; 4]>,
    tile_lists: Vec<Vec<u32>>,
    tile_records: Vec<u32>,
    counts: [usize; 2],
}

impl Bands {
    /// Prepare the immutable ring table and the frame's working storage.
    ///
    /// # Errors
    /// Refuses profiles whose working storage exceeds the device's storage binding limit.
    #[expect(
        clippy::too_many_lines,
        reason = "prepare field, pack and reduction pipeline in binding order"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated ring samples and default patch parameters use f32"
    )]
    pub fn new(device: &DeviceSite, rain: &RainBands) -> Result<Self, super::AmbientError> {
        let profile = rain.profile();
        let ring = rain.table();
        let bytes_limit = device.limits().max_storage_buffer_binding_size;
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
            let span = (2.0 * ring.reach() * f64::from(TILES) / side).ceil() + 1.0;
            max_drops += drops;
            max_indices += drops * span.powi(2);
        }
        let table_bytes = ring.radii as f64 * ring.ages as f64 * ring.classes.len() as f64 * 16.0;
        if max_drops * 16.0 > bytes_limit as f64
            || max_indices * 4.0 > bytes_limit as f64
            || table_bytes > bytes_limit as f64
            || ring.classes.len() > 64
        {
            return Err(super::AmbientError::DeviceLimit {
                resource: "rain bands".into(),
                limit: "max_storage_buffer_binding_size",
            });
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
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
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let buffer = |label, data, usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage,
            })
        };
        let lengths = profile.patches.map(|x| x as f32);
        let coordinates = BandFrame {
            lengths_reach: [lengths[0], lengths[1], ring.reach() as f32, 0.0],
            table: [ring.radii as u32, ring.ages as u32, 0, 0],
            steps: [ring.radius_step as f32, ring.age_step as f32, 0.0, 0.0],
        };
        let frame = buffer(
            "rain table coordinates",
            bytemuck::bytes_of(&coordinates),
            wgpu::BufferUsages::UNIFORM,
        );
        let samples: Vec<_> = ring
            .classes
            .iter()
            .flat_map(|class| {
                class
                    .height
                    .iter()
                    .zip(&class.slope)
                    .zip(&class.rate)
                    .map(|((&h, &s), &dt)| [h as f32, s as f32, dt as f32, 0.0])
            })
            .collect();
        let table = buffer(
            "rain ring table",
            bytemuck::cast_slice(&samples),
            wgpu::BufferUsages::STORAGE,
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
        let drops = working("rain drops", max_drops.max(1.0) as u64 * 16);
        #[expect(clippy::cast_sign_loss, reason = "allocation bounds are nonnegative")]
        let tiles = working("rain tile lists", max_indices as u64 * 4);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(include_str!("rain.wgsl").into()),
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
                storage(1),
                storage(2),
                storage(3),
                output(4),
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
                ..Default::default()
            })
        };
        let base = mip_view(0);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(PASS),
            layout: &pack_layout,
            entries: &[
                entry(0, frame.as_entire_binding()),
                entry(1, table.as_entire_binding()),
                entry(2, drops.as_entire_binding()),
                entry(3, tiles.as_entire_binding()),
                entry(4, wgpu::BindingResource::TextureView(&base)),
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
            drops,
            tiles,
            immutable_bytes: frame.size() + table.size(),
            passes,
            drop_records: Vec::new(),
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
        self.counts = [0; 2];
        self.drop_records.clear();
        for list in &mut self.tile_lists {
            list.clear();
        }
        if rain.active(time) {
            let reach = rain.table().reach();
            for (band, side) in rain.profile().patches.into_iter().enumerate() {
                let scale = f64::from(TILES) / side;
                for drop in rain
                    .drops(band, time)
                    .into_iter()
                    .filter(|drop| drop.weight > 0.0)
                {
                    self.counts[band] += 1;
                    let [x, z] = drop.place;
                    let index = u32::try_from(self.drop_records.len()).expect("bounded rain drops");
                    self.drop_records.push([
                        x as f32,
                        z as f32,
                        (time - drop.time) as f32,
                        drop.class as f32 + 0.5 * drop.weight as f32,
                    ]);
                    let bounds = |v: f64| {
                        ((v - reach) * scale).floor() as i32..=((v + reach) * scale).floor() as i32
                    };
                    for row in bounds(z).take(16) {
                        for col in bounds(x).take(16) {
                            let tile = usize::try_from(
                                row.rem_euclid(TILES) * TILES + col.rem_euclid(TILES),
                            )
                            .expect("wrapped tile index");
                            self.tile_lists[band * 256 + tile].push(index);
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

    /// Append the rain passes and return the whole field's surface read.
    pub fn declare(&self, graph: &mut EffectGraph) -> EffectAccess {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Persistent {
                owner: 0x7261_696e,
                epoch: 0,
                initialized: true,
            },
        ));
        let access = |mip, count, kind| EffectAccess {
            resource,
            range: mip * u64::from(LAYERS)..(mip + count) * u64::from(LAYERS),
            kind,
            view: Some(wgpu::TextureViewDimension::D2Array),
            history: None,
        };
        for mip in 0..u64::from(MIPS) {
            let mut accesses = vec![access(mip, 1, AccessKind::StorageWrite)];
            if mip != 0 {
                accesses.push(access(mip - 1, 1, AccessKind::SampledRead));
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
        access(0, u64::from(MIPS), AccessKind::SampledRead)
    }

    /// Allocated bytes for the rain working field and drop lists.
    pub fn bytes(&self) -> u64 {
        (0..MIPS)
            .map(|mip| u64::from((SIZE >> mip).pow(2) * LAYERS * 8))
            .sum::<u64>()
            + self.drops.size()
            + self.tiles.size()
            + self.immutable_bytes
    }

    /// The prepared field, available for readback by the conformance witness.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::BandFrame;
    use thrum_test_support::{record, records};

    #[test]
    fn band_coordinates_match_the_shader_layout() {
        records::named(
            "rain bands",
            include_str!("rain.wgsl"),
            &[record!(BandFrame, lengths_reach, table, steps)],
        );
    }
}
