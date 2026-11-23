#[cfg(target_arch = "wasm32")]
pub(crate) mod ground_reference;

use crate::noise::{Noise, SHAPE_PERIOD, SHAPE_SIZE};
use crate::{Error, RowStamp, WeatherMap};
use thrum_cloud_model::State;
use thrum_render::{
    atmosphere::{GpuSunMapping, SunOpticalDepth},
    contribution::{EffectState, EffectStep, EffectTime},
    runtime::DeviceSite,
};

pub(crate) const PATH_PASSES: [&str; 3] = [
    "cloud Sun path near",
    "cloud Sun path middle",
    "cloud Sun path far",
];
pub(crate) const GROUND_PASS: &str = "cloud ground field";
pub(crate) const FAR_PASS: &str = "cloud far transmittance";
const SHADER: &str = concat!(
    include_str!("sun_path/params.wgsl"),
    include_str!("sun_path.wgsl"),
    "\n",
    include_str!("erosion.wgsl")
);

#[cfg(any(test, target_arch = "wasm32"))]
const GROUND_REFERENCE_SHADER: &str = concat!(
    include_str!("sun_path/params.wgsl"),
    include_str!("sun_path/ground_reference.wgsl"),
    "\n",
    include_str!("erosion.wgsl")
);

/// One camera-following window of Sun-ray columns.
#[derive(Clone, Copy, Debug)]
pub struct SunLevel {
    /// Width of a column at the layer's base, in metres.
    pub texel: f64,
    /// Height samples, including the top and bottom, a multiple of four.
    pub slices: u32,
    /// Maximum age of this level's rows, in weather seconds.
    pub maximum_age: f64,
}

/// Resolution, reach and age bounds of the optical-depth product.
#[derive(Clone, Debug)]
pub struct SunProfile {
    /// Square extent of each Sun-ray window.
    pub size: u32,
    /// Fine, middle and horizon windows.
    pub levels: [SunLevel; 3],
    /// Composed ground-field extent.
    pub ground_size: u32,
    /// Floor of the path direction's elevation in radians.
    pub lowest_elevation: f64,
    /// Ground-field border width as a fraction of the field's width.
    pub border_fraction: f64,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    weather: [[f32; 4]; 4],
    origin: [f32; 4],
    direction: [f32; 4],
    physical: [f32; 4],
    erosion: [f32; 4],
    rows: [u32; 4],
}

/// Retained optical depths toward the Sun, and the atmosphere's ground field.
pub struct SunPath {
    profile: SunProfile,
    products: [wgpu::Texture; 3],
    ground: wgpu::Texture,
    mapping: wgpu::Buffer,
    params: Vec<wgpu::Buffer>,
    pipelines: [wgpu::ComputePipeline; 3],
    groups: Vec<wgpu::BindGroup>,
    pub(crate) shape_noise: Noise,
    noise_preparation: Option<wgpu::CommandBuffer>,
    clock: EffectState,
    origins: [[f64; 2]; 3],
    pending_origins: [[f64; 2]; 3],
    direction: [f32; 3],
    pending_direction: [f32; 3],
    runs: bool,
    ranges: [(u32, u32); 3],
    stamps: [Vec<RowStamp>; 3],
    pending_stamps: [Vec<RowStamp>; 3],
    last_weather_time: f64,
    pending_time: f64,
}

impl SunPath {
    pub(crate) fn uniforms(&self) -> &[wgpu::Buffer] {
        &self.params
    }
    pub(crate) fn disable(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.mapping, 28, &0.0_f32.to_le_bytes());
    }
    /// Prepare three packed fp16 height windows and their composed ground field.
    /// # Errors
    /// Rejects unsupported dimensions, invalid cadences or a nonpositive Sun floor.
    pub fn new(
        device: &DeviceSite,
        weather: &WeatherMap,
        profile: SunProfile,
        target: Option<(wgpu::Texture, wgpu::Buffer)>,
    ) -> Result<Self, Error> {
        validate_profile(device, weather, &profile)?;
        let texture = |name, width, layers| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(name),
                size: wgpu::Extent3d {
                    width,
                    height: width,
                    depth_or_array_layers: layers,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let products = std::array::from_fn(|i| {
            texture("cloud Sun path", profile.size, profile.levels[i].slices / 4)
        });
        let (ground, mapping) = if let Some((ground, mapping)) = target {
            if ground.width() != profile.ground_size
                || ground.height() != profile.ground_size
                || ground.format() != wgpu::TextureFormat::Rgba16Float
            {
                return Err(Error::Unsupported(
                    "registered ground-field dimensions or format",
                ));
            }
            SunOpticalDepth::from_gpu(ground.clone(), mapping.clone(), 0)
                .map_err(|_| Error::Unsupported("registered ground-field usage"))?;
            (ground, mapping)
        } else {
            (
                texture("cloud ground field", profile.ground_size, 1),
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("cloud ground mapping and far value"),
                    size: size_of::<GpuSunMapping>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
            )
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cloud Sun path"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let shape_noise = Noise::new(device, SHAPE_SIZE, "cloud shape noise");
        let noise_preparation = Some(shape_noise.prepare(device, weather.seed(), "shape"));
        let pipelines = std::array::from_fn(|i| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(["cloud Sun path", GROUND_PASS, FAR_PASS][i]),
                layout: None,
                module: &module,
                entry_point: Some(["path", "ground", "far"][i]),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        });
        let params: Vec<_> = (0..4)
            .map(|_| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("cloud Sun path parameters"),
                    size: size_of::<Uniform>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            })
            .collect();
        let weather_views: Vec<_> = weather.textures()[..2]
            .iter()
            .map(|t| {
                t.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let product_views: Vec<_> = products
            .iter()
            .map(|t| {
                t.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    ..Default::default()
                })
            })
            .collect();
        let ground_view = ground.create_view(&wgpu::TextureViewDescriptor::default());
        let mut groups = Vec::new();
        for i in 0..3 {
            groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cloud Sun path level"),
                layout: &pipelines[0].get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&weather_views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&weather_views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&product_views[i]),
                    },
                    shape_noise.bind(9),
                    wgpu::BindGroupEntry {
                        binding: 10,
                        resource: wgpu::BindingResource::Sampler(&shape_noise.sampler),
                    },
                ],
            }));
        }
        let entries = [
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params[3].as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&product_views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&product_views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(&product_views[2]),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&ground_view),
            },
        ];
        groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud ground composition"),
            layout: &pipelines[1].get_bind_group_layout(0),
            entries: &entries,
        }));
        groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cloud far transmittance"),
            layout: &pipelines[2].get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&product_views[2]),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: mapping.as_entire_binding(),
                },
            ],
        }));
        let size = profile.size as usize;
        Ok(Self {
            profile,
            products,
            ground,
            mapping,
            params,
            pipelines,
            groups,
            shape_noise,
            noise_preparation,
            clock: EffectState::new(0),
            origins: [[0.0; 2]; 3],
            pending_origins: [[0.0; 2]; 3],
            direction: [0.0, 1.0, 0.0],
            pending_direction: [0.0, 1.0, 0.0],
            runs: false,
            ranges: [(0, 0); 3],
            stamps: std::array::from_fn(|_| Vec::with_capacity(size)),
            pending_stamps: std::array::from_fn(|_| Vec::with_capacity(size)),
            last_weather_time: 0.0,
            pending_time: 0.0,
        })
    }

    pub(crate) fn prepare_noise(&mut self, queue: &wgpu::Queue) {
        if let Some(command) = self.noise_preparation.take() {
            queue.submit([command]);
        }
    }

    /// Register the ground field with the atmosphere; no CPU-fed copy is made.
    /// # Errors
    /// Returns the atmosphere's validation error for an incompatible resource.
    pub fn provider(
        &self,
        epoch: u64,
    ) -> Result<SunOpticalDepth, thrum_render::error::RenderError> {
        SunOpticalDepth::from_gpu(self.ground.clone(), self.mapping.clone(), epoch)
    }
    /// Reconstruct the simulation clock with the product's device generation.
    pub fn rebuild(&mut self, generation: u64, epoch: u64) {
        self.clock.rebuild(generation, epoch);
    }

    /// Upload a frame's mapping and select a tick's product update.
    /// # Errors
    /// Rejects a nonfinite camera, Sun, or unsupported density range.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "finite positions and wrapped noise phases narrow relative to air anchors"
    )]
    pub fn upload(
        &mut self,
        queue: &wgpu::Queue,
        weather: &WeatherMap,
        time: EffectTime,
        state: &State,
        camera: [f64; 2],
        sun: [f32; 3],
        cut: bool,
    ) -> Result<(), Error> {
        let step = self.clock.begin(time, u64::MAX, [0; 2]);
        self.runs = step != EffectStep::Keep;
        let heights = weather.height_range();
        let (model, kind) = weather.density_data(state);
        if !camera.iter().all(|x| x.is_finite())
            || !sun.iter().all(|x| x.is_finite())
            || sun.iter().map(|x| x * x).sum::<f32>() < 0.5
        {
            return Err(Error::Unsupported("Sun path frame coordinates"));
        }
        let elevation = f64::from(sun[1])
            .clamp(-1.0, 1.0)
            .asin()
            .max(self.profile.lowest_elevation);
        let horizontal = f64::from(sun[0]).hypot(f64::from(sun[2]));
        let azimuth = if horizontal > 1e-12 {
            [
                f64::from(sun[0]) / horizontal,
                f64::from(sun[2]) / horizontal,
            ]
        } else {
            [1.0, 0.0]
        };
        self.pending_direction = [
            (azimuth[0] * elevation.cos()) as f32,
            elevation.sin() as f32,
            (azimuth[1] * elevation.cos()) as f32,
        ];
        let jumped = self
            .pending_direction
            .iter()
            .zip(self.direction)
            .any(|(a, b)| (*a - b).abs() > 0.0005);
        // A paused world's light can be edited without advancing its tick.
        // Such an edit invalidates the ray direction, independently of weather.
        self.runs |= jumped;
        if self.runs {
            let stamp = RowStamp {
                tick: time.tick,
                time: state.time(),
                parameters: state.parameters(),
            };
            self.pending_time = state.time();
            for i in 0..3 {
                let width = self.profile.levels[i].texel * f64::from(self.profile.size);
                self.pending_origins[i] = std::array::from_fn(|a| {
                    ((camera[a] - state.travel()[a]) / (width / 4.0)).floor() * (width / 4.0)
                        - width / 2.0
                });
                let origin = self.pending_origins[i];
                let rows = &mut self.pending_stamps[i];
                rows.clone_from(&self.stamps[i]);
                rows.resize(self.profile.size as usize, stamp);
                let reset = step == EffectStep::Reset
                    || jumped
                    || !super::weather::same_origin(origin, self.origins[i]);
                let (start, count) = super::weather::update_rows(
                    rows,
                    stamp,
                    (state.time() - self.last_weather_time).max(0.0),
                    self.profile.levels[i].maximum_age,
                    reset,
                );
                self.ranges[i] = (start, count);
                let mut w = [[0.0; 4]; 4];
                for (j, entry) in w.iter_mut().enumerate() {
                    *entry = [
                        (weather.origins()[j][0] - origin[0]) as f32,
                        (weather.origins()[j][1] - origin[1]) as f32,
                        weather.profile().texels[j] as f32,
                        weather.profile().size as f32,
                    ];
                }
                let uniform = Uniform {
                    weather: w,
                    origin: [
                        0.0,
                        0.0,
                        self.profile.levels[i].texel as f32,
                        self.profile.size as f32,
                    ],
                    direction: [
                        self.pending_direction[0],
                        self.pending_direction[1],
                        self.pending_direction[2],
                        0.0,
                    ],
                    physical: [
                        heights[0] as f32,
                        heights[1] as f32,
                        kind.extinction() as f32,
                        model.erosion as f32,
                    ],
                    erosion: [
                        model.detail_mean as f32,
                        (origin[0].rem_euclid(SHAPE_PERIOD) / SHAPE_PERIOD) as f32,
                        (origin[1].rem_euclid(SHAPE_PERIOD) / SHAPE_PERIOD) as f32,
                        self.profile.levels[i].texel as f32,
                    ],
                    rows: [start, count, self.profile.levels[i].slices, 0],
                };
                queue.write_buffer(&self.params[i], 0, bytemuck::bytes_of(&uniform));
            }
            let outer = self.pending_origins[2];
            let mut w = [[0.0; 4]; 4];
            for (i, entry) in w.iter_mut().enumerate().take(3) {
                *entry = [
                    (self.pending_origins[i][0] - outer[0]) as f32,
                    (self.pending_origins[i][1] - outer[1]) as f32,
                    self.profile.levels[i].texel as f32,
                    self.profile.levels[i].slices as f32,
                ];
            }
            let uniform = Uniform {
                weather: w,
                origin: [
                    0.0,
                    0.0,
                    (self.profile.levels[2].texel * f64::from(self.profile.size)
                        / f64::from(self.profile.ground_size)) as f32,
                    self.profile.ground_size as f32,
                ],
                direction: [0.0; 4],
                physical: [self.profile.size as f32, 0.0, 0.0, 0.0],
                erosion: [0.0; 4],
                rows: [0; 4],
            };
            queue.write_buffer(&self.params[3], 0, bytemuck::bytes_of(&uniform));
        }
        self.write_mapping(queue, state, time, heights, cut);
        self.prepare_noise(queue);
        Ok(())
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "display travel is combined in f64 before narrowing world-relative GPU window coordinates"
    )]
    fn write_mapping(
        &self,
        queue: &wgpu::Queue,
        state: &State,
        time: EffectTime,
        heights: [f64; 2],
        cut: bool,
    ) {
        let origins = if self.runs {
            self.pending_origins
        } else {
            self.origins
        };
        let direction = if self.runs {
            self.pending_direction
        } else {
            self.direction
        };
        let travel = state.display_travel(
            f64::from(time.display_phase),
            time.tick_duration.as_secs_f64(),
            cut,
        );
        let width = self.profile.levels[2].texel * f64::from(self.profile.size);
        let metadata = GpuSunMapping {
            region: [
                (origins[2][0] + travel[0]) as f32,
                (origins[2][1] + travel[1]) as f32,
                (origins[2][0] + width + travel[0]) as f32,
                (origins[2][1] + width + travel[1]) as f32,
            ],
            plane_drift: [
                heights[0] as f32,
                0.0,
                0.0,
                f32::from(u8::from(self.runs || self.clock.committed().is_some())),
            ],
            path_border: [
                direction[0],
                direction[1],
                direction[2],
                (width * self.profile.border_fraction) as f32,
            ],
            far: [0.0, 1.0, 0.0, 0.0],
            near: [
                (origins[0][0] + travel[0]) as f32,
                (origins[0][1] + travel[1]) as f32,
                (self.profile.levels[0].texel * f64::from(self.profile.size)) as f32,
                0.0,
            ],
            middle: [
                (origins[1][0] + travel[0]) as f32,
                (origins[1][1] + travel[1]) as f32,
                (self.profile.levels[1].texel * f64::from(self.profile.size)) as f32,
                0.0,
            ],
        };
        let bytes = bytemuck::bytes_of(&metadata);
        queue.write_buffer(&self.mapping, 0, &bytes[..48]);
        queue.write_buffer(&self.mapping, 52, &bytes[52..]);
    }
    /// Record one level, the ground composition or the far-value reduction.
    pub fn encode(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        if !self.runs(index) {
            return;
        }
        let pipeline = if index < 3 { 0 } else { index - 2 };
        pass.set_pipeline(&self.pipelines[pipeline]);
        pass.set_bind_group(0, &self.groups[index], &[]);
        match index {
            0..=2 => pass.dispatch_workgroups(
                self.profile.size.div_ceil(8),
                self.ranges[index].1.div_ceil(8),
                1,
            ),
            3 => pass.dispatch_workgroups(
                self.profile.ground_size.div_ceil(8),
                self.profile.ground_size.div_ceil(8),
                1,
            ),
            _ => pass.dispatch_workgroups(1, 1, 1),
        }
    }
    /// Commit only for a submitted encoder.
    pub fn submitted(&mut self) {
        self.clock.submitted();
        if self.runs {
            self.origins = self.pending_origins;
            self.direction = self.pending_direction;
            std::mem::swap(&mut self.stamps, &mut self.pending_stamps);
            self.last_weather_time = self.pending_time;
        }
        self.runs = false;
    }
    /// Current recording selected a new tick.
    pub fn runs(&self, index: usize) -> bool {
        index < self.groups.len() && self.runs && (index >= 3 || self.ranges[index].1 > 0)
    }
    /// Last submitted tick and weather time of each level's rows.
    pub fn row_stamps(&self) -> &[Vec<RowStamp>; 3] {
        &self.stamps
    }
    /// Declared dimensions and weather-time cadences.
    pub fn profile(&self) -> &SunProfile {
        &self.profile
    }
    /// Physical height windows.
    pub fn products(&self) -> &[wgpu::Texture; 3] {
        &self.products
    }
    /// Ground field read by the atmosphere.
    pub fn ground(&self) -> &wgpu::Texture {
        &self.ground
    }
    /// GPU mapping, including the mean-transmittance far value.
    pub fn mapping(&self) -> &wgpu::Buffer {
        &self.mapping
    }
}

#[cfg(test)]
mod tests;

fn validate_profile(
    device: &DeviceSite,
    weather: &WeatherMap,
    profile: &SunProfile,
) -> Result<(), Error> {
    if profile.size == 0
        || profile.ground_size < 4
        || !profile.ground_size.is_multiple_of(2)
        || profile.levels.iter().any(|l| {
            !l.texel.is_finite()
                || l.texel <= 0.0
                || l.slices < 8
                || l.slices % 4 != 0
                || !l.maximum_age.is_finite()
                || l.maximum_age <= 0.0
        })
        || profile.levels.windows(2).any(|x| x[0].texel >= x[1].texel)
        || !profile.lowest_elevation.is_finite()
        || !(0.0..std::f64::consts::FRAC_PI_2).contains(&profile.lowest_elevation)
        || profile.lowest_elevation == 0.0
        || !profile.border_fraction.is_finite()
        || !(0.0..0.5).contains(&profile.border_fraction)
        || profile.border_fraction == 0.0
        || weather.profile().texels.len() != 4
    {
        return Err(Error::Unsupported(
            "Sun path dimensions, cadence, border or weather levels",
        ));
    }
    let limits = device.limits();
    if profile.size.max(profile.ground_size) > limits.max_texture_dimension_2d
        || profile
            .levels
            .iter()
            .any(|level| level.slices / 4 > limits.max_texture_array_layers)
    {
        return Err(Error::DeviceLimit("Sun path texture limits"));
    }
    Ok(())
}
