use crate::Error;
use thrum_cloud_model::{Parameters, Profile, State, Type};
use thrum_render::{
    contribution::{EffectState, EffectStep, EffectTime},
    runtime::DeviceSite,
};

pub(crate) const PASS: &str = "cloud weather map";

/// Presentation resolution and refresh bound, in weather seconds.
#[derive(Clone, Debug)]
pub struct WeatherProfile {
    /// Square extent of each level.
    pub size: u32,
    /// Metres per texel, fine to coarse.
    pub texels: Vec<f64>,
    /// Maximum age of any row in weather seconds.
    pub maximum_age: f64,
}

/// The tick at which a row's columns were evaluated.
#[derive(Clone, Copy, Debug)]
pub struct RowStamp {
    /// Committed source tick.
    pub tick: u64,
    /// Weather time at that tick.
    pub time: f64,
    /// Controls at that tick.
    pub parameters: Parameters,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    integers: [u32; 4],
    origin: [f32; 4],
    weather: [f32; 4],
    physical: [f32; 4],
    vertical: [f32; 4],
    noise: [u32; 4],
    rows: [u32; 4],
    turret: [f32; 4],
}

struct Pending {
    stamp: Option<RowStamp>,
    origins: Vec<[f64; 2]>,
    stamps: Vec<Vec<RowStamp>>,
    ranges: Vec<(u32, u32)>,
}

/// Persistent, time-sliced procedural columns in the air frame.
///
/// Two RGBA32 textures hold base/top/coverage/type and age/tau/rise/fall.
/// Integer cell identities occupy a separate RGBA32Uint texture: site X/Z,
/// generation and a nine-bit mask naming each neighbouring generation. Physical resources are exposed
/// read-only for downstream GPU consumers and target readback.
pub struct WeatherMap {
    profile: WeatherProfile,
    model: Profile,
    seed: u32,
    types: Vec<Type>,
    textures: [wgpu::Texture; 3],
    uniforms: Vec<wgpu::Buffer>,
    axes: Vec<wgpu::Buffer>,
    groups: Vec<wgpu::BindGroup>,
    pipeline: wgpu::ComputePipeline,
    clock: EffectState,
    origins: Vec<[f64; 2]>,
    stamps: Vec<Vec<RowStamp>>,
    pending: Pending,
    last_stamp: Option<RowStamp>,
}

impl WeatherMap {
    pub(crate) fn types(&self) -> &[Type] {
        &self.types
    }
    pub(crate) fn uniforms(&self) -> &[wgpu::Buffer] {
        &self.uniforms
    }
    pub(crate) fn axes(&self) -> &[wgpu::Buffer] {
        &self.axes
    }
    pub(crate) fn height_range(&self) -> [f64; 2] {
        [
            self.types
                .iter()
                .map(|t| t.base)
                .fold(f64::INFINITY, f64::min),
            self.types
                .iter()
                .map(|t| t.base + t.depth[1])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    }
    pub(crate) fn model_profile(&self) -> Profile {
        self.model
    }
    pub(crate) fn seed(&self) -> u32 {
        self.seed
    }
    pub(crate) fn density_data(&self, state: &State) -> (Profile, Type) {
        (
            self.model,
            resolve_type(&self.types, state.parameters().kind),
        )
    }
    /// Prepare the field once, without any preset or look in the engine.
    /// # Errors
    /// Rejects invalid model data, map dimensions and unavailable device limits.
    pub fn new(
        device: &DeviceSite,
        seed: u32,
        model: Profile,
        types: Vec<Type>,
        profile: WeatherProfile,
    ) -> Result<Self, Error> {
        thrum_cloud_model::Field::new(seed, model, types.clone()).map_err(Error::Model)?;
        if profile.size == 0
            || profile.texels.is_empty()
            || profile.texels.iter().any(|x| !x.is_finite() || *x <= 0.0)
            || !profile.maximum_age.is_finite()
            || profile.maximum_age <= 0.0
        {
            return Err(Error::Unsupported("weather map dimensions or cadence"));
        }
        let limits = device.limits();
        if profile.size > limits.max_texture_dimension_2d
            || profile.texels.len() > limits.max_texture_array_layers as usize
            || limits.max_storage_textures_per_shader_stage < 3
        {
            return Err(Error::DeviceLimit("weather map texture limits"));
        }
        let layers = u32::try_from(profile.texels.len())
            .map_err(|_| Error::Unsupported("weather levels"))?;
        let textures = std::array::from_fn(|i| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(
                    [
                        "weather columns",
                        "weather optical depth",
                        "weather cell identities",
                    ][i],
                ),
                size: wgpu::Extent3d {
                    width: profile.size,
                    height: profile.size,
                    depth_or_array_layers: layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: if i == 2 {
                    wgpu::TextureFormat::Rgba32Uint
                } else {
                    wgpu::TextureFormat::Rgba32Float
                },
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("weather field"),
            source: wgpu::ShaderSource::Wgsl(include_str!("weather.wgsl").into()),
        });
        let pipeline = device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });
        let views: Vec<_> = textures
            .iter()
            .map(|texture| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let uniforms: Vec<_> = profile
            .texels
            .iter()
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("weather parameters"),
                    size: size_of::<Uniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let axes: Vec<_> = profile
            .texels
            .iter()
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("weather integer texel axes"),
                    size: u64::from(profile.size) * 32,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let layout = pipeline.get_bind_group_layout(0);
        let groups = uniforms
            .iter()
            .zip(&axes)
            .map(|(uniform, axes)| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("weather level"),
                    layout: &layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniform.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&views[0]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&views[1]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&views[2]),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: axes.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();
        let levels = profile.texels.len();
        let stamps = || {
            (0..levels)
                .map(|_| Vec::with_capacity(profile.size as usize))
                .collect()
        };
        let pending = Pending {
            stamp: None,
            origins: vec![[0.0; 2]; levels],
            stamps: stamps(),
            ranges: vec![(0, 0); levels],
        };
        let committed_stamps = stamps();
        Ok(Self {
            profile,
            model,
            seed,
            types,
            textures,
            uniforms,
            axes,
            groups,
            pipeline,
            clock: EffectState::new(0),
            origins: vec![[0.0; 2]; levels],
            stamps: committed_stamps,
            pending,
            last_stamp: None,
        })
    }

    /// Reset on a new device or changed descriptor epoch.
    pub fn rebuild(&mut self, generation: u64, epoch: u64) {
        self.clock.rebuild(generation, epoch);
    }

    /// Select and upload a tentative update. Equal ticks and read-only captures
    /// write no texture; only `submitted` publishes row ages and anchors.
    /// # Errors
    /// Rejects positions or times beyond the integer addressing envelope.
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        time: EffectTime,
        state: &State,
        camera: [f64; 2],
    ) -> Result<(), Error> {
        let step = self.clock.begin(time, u64::MAX, [0; 2]);
        self.pending.stamp = None;
        if step == EffectStep::Keep {
            return Ok(());
        }
        let stamp = RowStamp {
            tick: time.tick,
            time: state.time(),
            parameters: state.parameters(),
        };
        let previous = self.last_stamp.unwrap_or(stamp);
        let elapsed = (stamp.time - previous.time).max(0.0);
        let maximum_age = self
            .profile
            .maximum_age
            .min(ramp_row_age(previous, stamp, elapsed));
        let resolved = resolve_type(&self.types, state.parameters().kind);
        for (level, texel) in self.profile.texels.iter().enumerate() {
            let block = texel * f64::from(self.profile.size) / 4.0;
            let origin = std::array::from_fn(|i| {
                ((camera[i] - state.travel()[i]) / block).floor() * block
                    - texel * f64::from(self.profile.size) / 2.0
            });
            self.pending.origins[level] = origin;
            let reset = step == EffectStep::Reset || !same_origin(self.origins[level], origin);
            if reset {
                let axes = texel_axes(origin, *texel, self.model, self.profile.size)?;
                queue.write_buffer(&self.axes[level], 0, bytemuck::cast_slice(&axes));
            }
            let rows = &mut self.pending.stamps[level];
            rows.clone_from(&self.stamps[level]);
            rows.resize(self.profile.size as usize, stamp);
            // A rotating contiguous strip with at least ceil(dt/age * rows)
            // rows guarantees a weather-time bound independently of frame rate.
            let (start, count) = update_rows(rows, stamp, elapsed, maximum_age, reset);
            self.pending.ranges[level] = (start, count);
            let uniform = pack(
                self.seed,
                self.model,
                state,
                origin,
                self.profile.texels[level],
                resolved,
                [
                    start,
                    count,
                    self.profile.size,
                    u32::try_from(level).map_err(|_| Error::Unsupported("weather level"))?,
                ],
            )?;
            queue.write_buffer(&self.uniforms[level], 0, bytemuck::bytes_of(&uniform));
        }
        self.pending.stamp = Some(stamp);
        Ok(())
    }

    /// Record the selected slices, without submission authority.
    pub fn encode(&self, pass: &mut wgpu::ComputePass<'_>) {
        if self.pending.stamp.is_some() {
            let pending = &self.pending;
            pass.set_pipeline(&self.pipeline);
            for (group, (_, count)) in self.groups.iter().zip(&pending.ranges) {
                if *count > 0 {
                    pass.set_bind_group(0, group, &[]);
                    pass.dispatch_workgroups(self.profile.size.div_ceil(8), count.div_ceil(8), 1);
                }
            }
        }
    }

    /// Commit only after the containing encoder was submitted.
    pub fn submitted(&mut self) {
        self.clock.submitted();
        if let Some(stamp) = self.pending.stamp.take() {
            std::mem::swap(&mut self.origins, &mut self.pending.origins);
            std::mem::swap(&mut self.stamps, &mut self.pending.stamps);
            self.last_stamp = Some(stamp);
        }
    }

    /// Whether the current recording has selected work.
    pub fn runs(&self) -> bool {
        self.pending.stamp.is_some() && self.pending.ranges.iter().any(|r| r.1 > 0)
    }
    /// Physical columns, optical data, and integer cell identities.
    pub fn textures(&self) -> &[wgpu::Texture; 3] {
        &self.textures
    }
    /// Air-frame origins used by the current recording, or the committed frame.
    pub fn origins(&self) -> &[[f64; 2]] {
        if self.pending.stamp.is_some() {
            &self.pending.origins
        } else {
            &self.origins
        }
    }
    /// Submitted row timestamps, independent of display-frame interpolation.
    pub fn row_stamps(&self) -> &[Vec<RowStamp>] {
        &self.stamps
    }
    /// Presentation dimensions and cadence.
    pub fn profile(&self) -> &WeatherProfile {
        &self.profile
    }
}

fn ramp_row_age(previous: RowStamp, next: RowStamp, elapsed: f64) -> f64 {
    let change = (next.parameters.coverage - previous.parameters.coverage)
        .abs()
        .max((next.parameters.kind - previous.parameters.kind).abs());
    if change == 0.0 {
        return f64::INFINITY;
    }
    // A global ramp can move the optical edge of a deck much faster than
    // local cloud evolution. Refresh the strips faster during that ramp so
    // adjacent rows do not switch wide patches between old and new opacity.
    const RAMP_CONTROL_CHANGE: f64 = 0.001;
    RAMP_CONTROL_CHANGE * (elapsed / change)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "bounded ceil of the clamped row count"
)]
pub(crate) fn rows_to_update(dt: f64, age: f64, size: u32) -> u32 {
    (dt / age * f64::from(size))
        .ceil()
        .clamp(0.0, f64::from(size)) as u32
}

#[expect(
    clippy::float_cmp,
    reason = "these are discrete snapped window identities, not approximate geometric comparisons"
)]
pub(crate) fn same_origin(a: [f64; 2], b: [f64; 2]) -> bool {
    a == b
}

pub(crate) fn update_rows(
    rows: &mut [RowStamp],
    stamp: RowStamp,
    elapsed: f64,
    maximum_age: f64,
    reset: bool,
) -> (u32, u32) {
    let size =
        u32::try_from(rows.len()).expect("row count is bounded by the admitted texture extent");
    // A refresh wave can straddle the end of the wrapping texture. Start at
    // its circular boundary, rather than the lowest array index of the wave;
    // the latter skips its tail after enough revolutions and violates age.
    let oldest = rows.iter().map(|row| row.tick).min().unwrap_or(0);
    let first = rows
        .iter()
        .enumerate()
        .position(|(i, row)| {
            row.tick == oldest && rows[(i + rows.len() - 1) % rows.len()].tick != oldest
        })
        .unwrap_or(0);
    let (start, count) = if reset || elapsed >= maximum_age {
        (0, size)
    } else {
        (
            u32::try_from(first).expect("selected row belongs to the admitted extent"),
            rows_to_update(elapsed, maximum_age, size),
        )
    };
    for i in 0..count {
        rows[((start + i) % size) as usize] = stamp;
    }
    (start, count)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "validated and clamped type-table coordinate"
)]
fn resolve_type(types: &[Type], kind: f64) -> Type {
    let position = kind * (types.len() - 1) as f64;
    let i = (position.floor() as usize).min(types.len() - 2);
    let a = types[i];
    let b = types[i + 1];
    let f = position - i as f64;
    let mix = |x: f64, y: f64| x + (y - x) * f;
    Type {
        base: mix(a.base, b.base),
        depth: std::array::from_fn(|j| mix(a.depth[j], b.depth[j])),
        liquid_water: mix(a.liquid_water, b.liquid_water),
        droplet_radius: mix(a.droplet_radius, b.droplet_radius),
        deck: mix(a.deck, b.deck),
        profile: std::array::from_fn(|j| mix(a.profile[j], b.profile[j])),
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "finite float narrowing and deliberately wrapping integer hash coordinates, checked before conversion"
)]
fn pack(
    seed: u32,
    model: Profile,
    state: &State,
    origin: [f64; 2],
    texel: f64,
    kind: Type,
    rows: [u32; 4],
) -> Result<Uniform, Error> {
    let site = origin.map(|x| (x / model.pitch).floor());
    let generation = (state.time() / model.period).floor();
    if site
        .iter()
        .chain([generation].iter())
        .any(|x| !x.is_finite() || x.abs() > f64::from(i32::MAX - 1024))
    {
        return Err(Error::Unsupported("integer weather addressing range"));
    }
    let s = site.map(|x| x as i32);
    let g = generation as i32;
    let fraction = std::array::from_fn::<_, 2, _>(|i| origin[i] / model.pitch - site[i]);
    Ok(Uniform {
        integers: [s[0] as u32, s[1] as u32, g as u32, seed],
        origin: [
            fraction[0] as f32,
            fraction[1] as f32,
            (state.time() / model.period - generation) as f32,
            (texel / model.pitch) as f32,
        ],
        weather: [
            state.parameters().coverage as f32,
            state.parameters().kind as f32,
            model.period as f32,
            model.pitch as f32,
        ],
        physical: [
            kind.base as f32,
            kind.depth[0] as f32,
            kind.depth[1] as f32,
            kind.extinction() as f32,
        ],
        vertical: [
            kind.deck as f32,
            kind.profile[0] as f32,
            kind.profile[1] as f32,
            0.0,
        ],
        noise: [
            s[0].div_euclid(4) as u32,
            s[1].div_euclid(4) as u32,
            g.div_euclid(2) as u32,
            ((state.time() / model.period - generation) * 4_294_967_296.0).floor() as u32,
        ],
        rows,
        turret: [
            model.turret_least as f32,
            model.turret_fade as f32,
            0.0,
            0.0,
        ],
    })
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "validated integer lattice words and fractions narrow independently, without subtracting large GPU positions"
)]
fn texel_axes(
    origin: [f64; 2],
    texel: f64,
    model: Profile,
    size: u32,
) -> Result<Vec<[u32; 4]>, Error> {
    let mut output = Vec::with_capacity(size as usize * 2);
    for base in origin {
        for i in 0..size {
            let point = base + (f64::from(i) + 0.5) * texel;
            let mut axis = [0; 4];
            for (words, pitch) in axis
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .zip([model.pitch, model.turret_pitch])
            {
                let coordinate = point / pitch;
                let cell = coordinate.floor();
                if !cell.is_finite() || cell.abs() > f64::from(i32::MAX - 1024) {
                    return Err(Error::Unsupported("weather texel axis addressing"));
                }
                words[0] = cell as i32 as u32;
                words[1] = ((coordinate - cell) as f32).to_bits();
            }
            output.push(axis);
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests;
