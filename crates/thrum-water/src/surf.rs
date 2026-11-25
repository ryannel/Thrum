//! Immutable surf data, lent to the field cache, column query and whitewater.
pub(crate) mod cache;
mod shoaling;
#[cfg(test)]
mod tests;

use crate::{WaterError, map_gpu::BODY_LIMIT};
use bytemuck::{Pod, Zeroable};
use thrum_render::binding;
use thrum_water_model::{Map, SurfProfile};
use wgpu::{BindGroupEntry, BindGroupLayoutEntry, ShaderStages, TextureView, util::DeviceExt};

use wgpu::TextureViewDimension::{D2, D2Array};

const CHANNELS: usize = 4;
const SECTORS: usize = 8;
const RUNGS: usize = 6;
const LAG_LAYERS: usize = SECTORS * RUNGS / CHANNELS;
const EXPOSURE_LAYERS: usize = SECTORS / CHANNELS;
const BINDINGS: u32 = 5;
pub(crate) const QUERY_ONE_SHEET: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShoreBody {
    stages: [f32; 2],
    base: u32,
    count: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SurfRecord {
    controls: [f32; 4],
    bounds: [f32; 4],
    coarse: [f32; 4],
    bodies: [ShoreBody; BODY_LIMIT],
    shoaling: [[f32; 4]; shoaling::ROWS],
}

pub(crate) struct SurfGpu {
    lag: TextureView,
    exposure: TextureView,
    section: TextureView,
    pyramid: TextureView,
    settings: wgpu::Buffer,
    bytes: [u64; 4],
}

impl SurfGpu {
    pub(crate) fn source(group: u32, first: u32) -> String {
        format!(
            "const SURF_GROUP = {group}u; const SURF_FIRST = {first}u;\n\
             const SURF_SECTORS = {SECTORS}u; const SURF_RUNGS = {RUNGS}u;\n\
             const SURF_LAG_LAYERS = {LAG_LAYERS}u; const SURF_EXPOSURE_LAYERS = {EXPOSURE_LAYERS}u;\n"
        ) + &shoaling::source()
            + include_str!("surf.wgsl")
            + include_str!("surf/shoaling.wgsl")
    }

    pub(crate) fn query_source(group: u32, first: u32) -> String {
        Self::source(group, first)
            + &format!("const ONE_SHEET = {QUERY_ONE_SHEET}u;\n")
            + include_str!("surf/cut.wgsl")
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "device-bounded extents and f32 physical records"
    )]
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        map: &Map,
        profile: &SurfProfile,
    ) -> Result<Self, WaterError> {
        let (bodies, stages) = stage_records(map)?;
        let [width, height] = map.bed().lattice().2;
        let texture = |label, size, format| product(device, label, size, format);
        let lag = texture(
            "surf lags",
            [width, height, (stages * LAG_LAYERS).max(1)],
            wgpu::TextureFormat::Rgba32Float,
        )?;
        let exposure = texture(
            "surf exposures",
            [width, height, (stages * (EXPOSURE_LAYERS + 1)).max(1)],
            wgpu::TextureFormat::Rgba32Float,
        )?;
        let section_rows = profile.section.rows();
        let phase_count = profile
            .section
            .rows()
            .next()
            .expect("validated section")
            .1
            .len();
        let section = texture(
            "breaker section",
            [phase_count, section_rows.len(), 1],
            wgpu::TextureFormat::Rgba32Float,
        )?;
        let levels = map.surf_bed_pyramid().expect("prepared surf").levels();
        let pyramid_height = levels.map(|(size, _)| size[1]).sum();
        let pyramid = texture(
            "surf bed pyramid",
            [width, pyramid_height, 1],
            wgpu::TextureFormat::Rg32Float,
        )?;

        let mut scratch = vec![[0.0; CHANNELS]; width * height];
        for (id, body) in bodies.iter().enumerate() {
            if let Some(shore) = map.shore_transform(id) {
                let base = body.base;
                write_planes(
                    queue,
                    &lag,
                    base * LAG_LAYERS as u32,
                    shore.lag_planes(),
                    &mut scratch,
                );
                write_planes(
                    queue,
                    &exposure,
                    base * EXPOSURE_LAYERS as u32,
                    shore.exposure_planes(),
                    &mut scratch,
                );
                // Keep references in the same texture binding as exposure.
                // Their planes follow all the packed sector exposure planes.
                let first = (stages * EXPOSURE_LAYERS) as u32 + base;
                for (offset, plane) in shore.contour_planes().enumerate() {
                    for (texel, &[x, z, weight]) in scratch.iter_mut().zip(plane) {
                        *texel = [x as f32, z as f32, weight as f32, 0.0];
                    }
                    write(
                        queue,
                        &exposure,
                        [0, 0, first + offset as u32],
                        [width, height],
                        &scratch,
                    );
                }
            }
        }
        let mut onset = f64::INFINITY;
        let mut previous = 0.0;
        let mut table = Vec::with_capacity(phase_count * section_rows.len());
        for (index, row) in section_rows {
            if row.iter().any(|point| point.throw > 0.0) {
                onset = onset.min(previous);
            }
            table.extend(
                row.iter()
                    .map(|point| [point.height as f32, point.throw as f32, index as f32, 0.0]),
            );
            previous = index;
        }
        write(
            queue,
            &section,
            [0; 3],
            [phase_count, section.height() as usize],
            &table,
        );
        let mut y = 0;
        for (size, bounds) in map.surf_bed_pyramid().expect("prepared surf").levels() {
            let extrema: Vec<_> = bounds
                .iter()
                .map(|[lo, hi]| [(*lo as f32).next_down(), (*hi as f32).next_up()])
                .collect();
            write(queue, &pyramid, [0, y, 0], size, &extrema);
            y += size[1] as u32;
        }

        let minimum_level = (0..BODY_LIMIT)
            .filter_map(|id| map.body(id))
            .map(|body| {
                let level = match &body.kind {
                    thrum_water_model::BodyKind::Chart(chart) => {
                        chart.levels.iter().copied().fold(body.level, f64::min)
                    }
                    _ => body.level,
                };
                level + body.stage.min
            })
            .fold(f64::INFINITY, f64::min);
        let record = SurfRecord {
            controls: [
                profile.lag_floor as f32,
                profile.breaker_ratio as f32,
                profile.cut.cell as f32,
                profile.cut.tolerance as f32,
            ],
            bounds: [
                (profile.section.maximum_throw() as f32).next_up(),
                (onset as f32).next_down(),
                (minimum_level as f32).next_down(),
                (stages * EXPOSURE_LAYERS) as f32,
            ],
            coarse: [
                profile.cut.coarse_cells as f32,
                profile.cut.margin_floor as f32,
                profile.cut.bend_factor as f32,
                0.0,
            ],
            bodies,
            shoaling: shoaling::table(),
        };
        let settings = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surf settings"),
            contents: bytemuck::bytes_of(&record),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bytes = [&lag, &exposure, &section, &pyramid].map(texture_bytes);
        let view = |texture: wgpu::Texture, dimension| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(dimension),
                ..Default::default()
            })
        };
        Ok(Self {
            lag: view(lag, D2Array),
            exposure: view(exposure, D2Array),
            section: view(section, D2),
            pyramid: view(pyramid, D2),
            settings,
            bytes,
        })
    }

    pub(crate) fn layout(
        first: u32,
        visibility: ShaderStages,
    ) -> [BindGroupLayoutEntry; BINDINGS as usize] {
        let texture = |offset, dimension| {
            binding::texture(
                first + offset,
                visibility,
                wgpu::TextureSampleType::Float { filterable: false },
                dimension,
            )
        };
        [
            binding::uniform(first, visibility, size_of::<SurfRecord>() as u64),
            texture(1, D2Array),
            texture(2, D2Array),
            texture(3, D2),
            texture(4, D2),
        ]
    }

    pub(crate) fn entries(&self, first: u32) -> [BindGroupEntry<'_>; BINDINGS as usize] {
        let texture = |offset, view| BindGroupEntry {
            binding: first + offset,
            resource: wgpu::BindingResource::TextureView(view),
        };
        [
            BindGroupEntry {
                binding: first,
                resource: self.settings.as_entire_binding(),
            },
            texture(1, &self.lag),
            texture(2, &self.exposure),
            texture(3, &self.section),
            texture(4, &self.pyramid),
        ]
    }

    pub(crate) fn bytes(&self) -> impl Iterator<Item = (String, u64)> {
        [
            "surf lags",
            "surf exposures",
            "breaker section",
            "surf bed pyramid",
        ]
        .into_iter()
        .map(String::from)
        .zip(self.bytes)
    }
}

// The final stage interval can be shorter than half a metre. Keep both endpoint
// values so the reader blends the final interval using its actual width.
#[expect(
    clippy::cast_possible_truncation,
    reason = "checked layer indices and GPU stage precision"
)]
fn stage_records(map: &Map) -> Result<([ShoreBody; BODY_LIMIT], usize), WaterError> {
    let mut bodies = [ShoreBody::zeroed(); BODY_LIMIT];
    let mut stages = 0;
    for (id, record) in bodies.iter_mut().enumerate() {
        if let Some(shore) = map.shore_transform(id) {
            let samples = shore.stages();
            *record = ShoreBody {
                stages: [samples[0] as f32, samples[samples.len() - 1] as f32],
                base: u32::try_from(stages)
                    .map_err(|_| WaterError::UnsupportedMap("surf stage layers"))?,
                count: u32::try_from(samples.len())
                    .map_err(|_| WaterError::UnsupportedMap("surf stage layers"))?,
            };
            stages += samples.len();
        }
    }
    Ok((bodies, stages))
}

fn product(
    device: &wgpu::Device,
    label: &str,
    size: [usize; 3],
    format: wgpu::TextureFormat,
) -> Result<wgpu::Texture, WaterError> {
    let limits = device.limits();
    let [width, height, depth_or_array_layers] = size.map(|n| u32::try_from(n).unwrap_or(u32::MAX));
    if width.max(height) > limits.max_texture_dimension_2d
        || depth_or_array_layers > limits.max_texture_array_layers
    {
        return Err(WaterError::UnsupportedMap(
            "surf textures exceed the device texture limits",
        ));
    }
    Ok(device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    }))
}

fn texture_bytes(texture: &wgpu::Texture) -> u64 {
    u64::from(texture.width())
        * u64::from(texture.height())
        * u64::from(texture.depth_or_array_layers())
        * u64::from(texture.format().block_copy_size(None).expect("surf format"))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "lag and exposure textures store f32 samples"
)]
fn write_planes<'a>(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    base: u32,
    mut planes: impl ExactSizeIterator<Item = &'a [f64]>,
    scratch: &mut [[f32; CHANNELS]],
) {
    let size = [texture.width() as usize, texture.height() as usize];
    for offset in 0..planes.len().div_ceil(CHANNELS) {
        for channel in 0..CHANNELS {
            let plane = planes.next().expect("complete packed plane");
            for (texel, &value) in scratch.iter_mut().zip(plane) {
                texel[channel] = if value.is_finite() {
                    value as f32
                } else {
                    f32::MAX
                };
            }
        }
        write(queue, texture, [0, 0, base + offset as u32], size, scratch);
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "texture extents fit the device limits"
)]
fn write<T: Pod>(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    start: [u32; 3],
    size: [usize; 2],
    data: &[T],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: start[0],
                y: start[1],
                z: start[2],
            },
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some((size[0] * size_of::<T>()) as u32),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: size[0] as u32,
            height: size[1] as u32,
            depth_or_array_layers: 1,
        },
    );
}
