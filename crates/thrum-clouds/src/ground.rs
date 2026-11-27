pub(crate) mod band;
pub(crate) mod lighting;
pub(crate) mod medium;

use crate::Error;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use thrum_cloud_model::{Affinity, Fog, Ground, State, Surface};
use thrum_render::binding;

/// World-anchored surface field and fog table supplied by the game.
#[derive(Clone)]
pub struct GroundConfig {
    /// The same surface field read by the headless query.
    pub surface: Arc<Surface>,
    /// The same fog table read by the headless query.
    pub fog: Arc<Ground>,
}

impl GroundConfig {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bound the uploaded f32 inputs"
    )]
    fn heights(&self) -> [f32; 4] {
        let mut terrain_min = f32::INFINITY;
        let mut terrain_max = f32::NEG_INFINITY;
        let mut datum_max = f32::NEG_INFINITY;
        for [height, datum, _] in self.surface.nodes() {
            terrain_min = terrain_min.min(height as f32);
            terrain_max = terrain_max.max(height as f32);
            datum_max = datum_max.max(datum as f32);
        }
        let fog_top = self.fog.rows().iter().fold(terrain_max, |top, row| {
            let pooling = row.pooling as f32;
            let even = terrain_max + row.depth as f32;
            let pooled = datum_max + row.depth as f32 - row.depression as f32;
            top.max((1.0 - pooling) * even + pooling * pooled)
        });
        [terrain_min, fog_top, terrain_max, 0.0]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GroundFrame {
    origin: [f32; 2],
    spacing: f32,
    shore_margin: f32,
    travel: [f32; 2],
    time: f32,
    seed: u32,
    upper_heights: [f32; 2],
    has_active_rows: u32,
    padding: u32,
    ground_heights: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GroundRow {
    shape: [f32; 4],
    bank: [f32; 4],
    optics: [f32; 4],
    affinity: [u32; 2],
    padding: [f32; 2],
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "weather GPU arithmetic uses f32"
)]
impl GroundRow {
    fn new(row: Fog) -> Self {
        Self {
            shape: [row.depth, row.depression, row.softness, row.pooling].map(|value| value as f32),
            bank: [row.centre[0], row.centre[1], row.radius[0], row.radius[1]]
                .map(|value| value as f32),
            optics: [
                row.extinction() as f32,
                0.0,
                row.noise as f32,
                row.noise_scale.recip() as f32,
            ],
            affinity: [
                match row.affinity {
                    Affinity::Any => 0,
                    Affinity::Water => 1,
                    Affinity::Land => 2,
                },
                u32::from(row.drifting),
            ],
            padding: [0.0; 2],
        }
    }
}

/// GPU surface field, fog table and display coordinates.
pub struct GroundField {
    pub(crate) surface: wgpu::Texture,
    pub(crate) frame: wgpu::Buffer,
    pub(crate) rows: wgpu::Buffer,
    view: wgpu::TextureView,
    parameters: GroundFrame,
    table: Vec<GroundRow>,
    fog: Arc<Ground>,
}

impl GroundField {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validate f32 surface data before upload"
    )]
    /// Allocate a surface field and its fog rows on a prepared device.
    /// # Errors
    /// Rejects fields outside the device limits or GPU coordinate precision.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &GroundConfig,
        seed: u32,
    ) -> Result<Self, Error> {
        let (origin, size, spacing) = config.surface.grid();
        let limits = device.limits();
        if size
            .iter()
            .any(|&side| side > limits.max_texture_dimension_2d as usize)
        {
            return Err(Error::DeviceLimit("cloud surface field extent"));
        }
        let row_bytes = config
            .fog
            .rows()
            .len()
            .saturating_mul(size_of::<GroundRow>());
        if row_bytes == 0 {
            return Err(Error::Unsupported("Ground requires at least one fog row"));
        }
        if row_bytes as u64 > limits.max_storage_buffer_binding_size {
            return Err(Error::DeviceLimit("cloud fog table storage"));
        }
        let parameters = GroundFrame {
            origin: origin.map(|value| value as f32),
            spacing: spacing as f32,
            shore_margin: config.fog.shore_margin() as f32,
            travel: [0.0; 2],
            time: 0.0,
            seed,
            ground_heights: config.heights(),
            ..GroundFrame::zeroed()
        };
        if parameters.origin.iter().any(|value| !value.is_finite())
            || !parameters.spacing.is_normal()
            || !parameters.shore_margin.is_normal()
        {
            return Err(Error::Unsupported("cloud surface field coordinates"));
        }
        let extent = wgpu::Extent3d {
            width: size[0] as u32,
            height: size[1] as u32,
            depth_or_array_layers: 1,
        };
        let data: Vec<_> = config
            .surface
            .nodes()
            .map(|[height, datum, shore]| {
                let shore = shore.clamp(-f64::from(f32::MAX) * 0.25, f64::from(f32::MAX) * 0.25);
                [height as f32, datum as f32, shore as f32, 0.0]
            })
            .collect();
        if data.iter().flatten().any(|value| !value.is_finite()) {
            return Err(Error::Unsupported("cloud surface field height"));
        }
        let surface = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cloud surface field"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        queue.write_texture(
            surface.as_image_copy(),
            bytemuck::cast_slice(&data),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16 * extent.width),
                rows_per_image: Some(extent.height),
            },
            extent,
        );
        let view = surface.create_view(&wgpu::TextureViewDescriptor::default());
        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud ground frame"),
            size: size_of::<GroundFrame>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let rows = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud fog table"),
            size: row_bytes as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let table = config
            .fog
            .rows()
            .iter()
            .copied()
            .map(GroundRow::new)
            .collect();
        Ok(Self {
            surface,
            frame,
            rows,
            view,
            parameters,
            table,
            fog: Arc::clone(&config.fog),
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "weather GPU arithmetic uses f32"
    )]
    /// Update the frame inputs from a committed weather state and display travel.
    pub fn upload(&mut self, queue: &wgpu::Queue, state: &State, travel: [f64; 2]) {
        self.parameters.travel = travel.map(|value| value as f32);
        self.parameters.time = state.time() as f32;
        for (index, (gpu, row)) in self.table.iter_mut().zip(self.fog.rows()).enumerate() {
            gpu.optics[1] = state.ground().amount(index, row.radiative) as f32;
        }
        self.parameters.has_active_rows =
            u32::from(self.table.iter().any(|row| row.optics[1] > 0.0));
        queue.write_buffer(&self.frame, 0, bytemuck::bytes_of(&self.parameters));
        queue.write_buffer(&self.rows, 0, bytemuck::cast_slice(&self.table));
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "use the same limits as the upper Sun-path textures"
    )]
    pub(crate) fn set_upper_heights(&mut self, heights: [f64; 2]) {
        self.parameters.upper_heights = heights.map(|value| value as f32);
    }

    pub(super) fn heights(&self) -> [f32; 4] {
        self.parameters.ground_heights
    }

    /// Density shader, including the three bindings beginning at `first`.
    pub fn source(group: u32, first: u32) -> String {
        format!(
            "@group({group}) @binding({first}) var ground_surface_texture: texture_2d<f32>;\n\
             @group({group}) @binding({frame}) var<uniform> ground_frame: GroundFrame;\n\
             @group({group}) @binding({rows}) var<storage, read> ground_rows: array<GroundRow>;\n{}",
            include_str!("ground.wgsl"),
            frame = first + 1,
            rows = first + 2,
        )
    }

    /// Surface, frame and row entries beginning at `first`.
    pub fn layout(first: u32) -> [wgpu::BindGroupLayoutEntry; 3] {
        let stages = wgpu::ShaderStages::COMPUTE;
        [
            binding::texture(
                first,
                stages,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
            binding::uniform(first + 1, stages, size_of::<GroundFrame>() as u64),
            binding::storage(first + 2, stages, true, size_of::<GroundRow>() as u64),
        ]
    }

    /// Bind the surface, frame and row data beginning at `first`.
    pub fn entries(&self, first: u32) -> [wgpu::BindGroupEntry<'_>; 3] {
        [
            wgpu::BindGroupEntry {
                binding: first,
                resource: wgpu::BindingResource::TextureView(&self.view),
            },
            wgpu::BindGroupEntry {
                binding: first + 1,
                resource: self.frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: first + 2,
                resource: self.rows.as_entire_binding(),
            },
        ]
    }
}

/// Readback handles for the prepared ground condensate provider's products.
#[derive(Clone)]
pub struct GroundProducts {
    /// Whole Sun-path depth, surface height and independent visibility.
    pub band: wgpu::Texture,
    /// Scene target and the camera records that located its samples.
    pub medium: thrum_render::atmosphere::ViewVolumeMediumTarget,
}

pub(crate) struct GroundGpu {
    pub(crate) field: GroundField,
    pub(crate) phases: lighting::LightTable,
    pub(crate) targets: GroundTargets,
}

pub(crate) struct GroundTargets {
    pub(crate) band: band::Band,
    pub(crate) medium: medium::Medium,
}

impl GroundTargets {
    pub(crate) fn new(
        surface: &thrum_render::contribution::SceneSurface<'_>,
        weather: &crate::WeatherMap,
        sun: &crate::SunPath,
        sun_mapping: &wgpu::Buffer,
        field: &GroundField,
        phases: &lighting::LightTable,
        sky: bool,
        shapes: Option<&crate::shapes::Shapes>,
    ) -> Result<Self, Error> {
        let band = band::Band::new(surface, field, sun, sun_mapping)?;
        let medium = medium::Medium::new(
            surface,
            weather,
            sun,
            sun_mapping,
            field,
            phases,
            &band,
            sky,
            shapes,
        )?;
        Ok(Self { band, medium })
    }
}

impl GroundGpu {
    pub(crate) fn new(
        surface: &thrum_render::contribution::SceneSurface<'_>,
        config: &GroundConfig,
        weather: &crate::WeatherMap,
        sun: &crate::SunPath,
        sky: bool,
        shapes: Option<&crate::shapes::Shapes>,
    ) -> Result<Self, Error> {
        let field = GroundField::new(surface.device, surface.queue, config, weather.seed())?;
        let phases = lighting::LightTable::new(surface.device, &config.fog)?;
        let targets = GroundTargets::new(
            surface,
            weather,
            sun,
            sun.mapping(),
            &field,
            &phases,
            sky,
            shapes,
        )?;
        Ok(Self {
            field,
            phases,
            targets,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GroundField, GroundFrame, GroundRow};

    #[test]
    fn ground_density_records_match_the_shader() {
        thrum_test_support::records::named(
            "ground density",
            &GroundField::source(0, 0),
            &[
                thrum_test_support::record!(
                    GroundFrame as "GroundFrame",
                    origin, spacing, shore_margin, travel, time, seed, upper_heights,
                    has_active_rows, padding, ground_heights
                ),
                thrum_test_support::record!(GroundRow as "GroundRow", shape, bank, optics, affinity, padding),
            ],
        );
    }
}
