//! The water surface's gain and filtering inputs for the optional rain bands.
use thrum_render::{
    binding,
    contribution::{QualityInvalidation, QualityKnob, QualitySettings},
    runtime::DeviceSite,
};
use thrum_sea::rain::Bands;
use thrum_water_model::RainBands;

pub(super) const KNOB: &str = "water.rain_cadence";
pub(super) fn quality() -> QualitySettings {
    QualitySettings::new(vec![QualityKnob {
        name: KNOB.into(),
        effect: "Display frames between rain band evaluations",
        value: 1,
        range: 1..=4,
        invalidation: QualityInvalidation::EffectHistory,
        governor_steps: None,
    }])
    .expect("rain cadence declaration")
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RainSurface {
    lengths: [f32; 4],
    canopy: [f32; 4],
    grid: [u32; 4],
    footprint_scale: [f32; 4],
}
pub(super) struct Rain {
    pub(super) bands: Bands,
    surface: wgpu::Buffer,
    canopy: wgpu::TextureView,
    canopy_bytes: u64,
    sampler: wgpu::Sampler,
    settings: RainSurface,
    since_run: u32,
    last_time: Option<f64>,
    recorded_time: f64,
    pub(super) pending: bool,
    active: bool,
    pub(super) recorded_drops: [usize; 2],
}

impl Rain {
    pub(super) fn active(&self) -> bool {
        self.active
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated canopy and depth response in GPU units"
    )]
    pub(super) fn new(
        device: &DeviceSite,
        queue: &wgpu::Queue,
        model: &RainBands,
        cascades: &wgpu::Texture,
        sampler: wgpu::Sampler,
    ) -> Result<Self, String> {
        let grid = model
            .canopy_grid()
            .ok_or("rain bands need prepared canopy columns")?;
        let response = &model.table().depth_response;
        let [columns, height] = grid.size;
        let (origin, offset) = super::canopy::origin_parts(&grid);
        let (mut records, width, code) = if grid.sparse_cells.is_empty() {
            if columns.checked_mul(height) != Some(grid.columns.len()) {
                return Err("rain canopy columns do not match its dimensions".into());
            }
            (
                grid.columns
                    .iter()
                    .copied()
                    .map(super::canopy::layers)
                    .collect(),
                columns.max(1),
                0,
            )
        } else {
            let sparse =
                super::canopy::SparseColumns::pack(&grid).ok_or("invalid sparse canopy")?;
            let mut texels = Vec::with_capacity(sparse.texels.len() + 1);
            texels.push([offset[0], offset[1], 0.0, 0.0]);
            texels.extend(sparse.texels);
            (texels, 1024, sparse.code)
        };
        let depth_start = records.len();
        records.extend(
            response
                .iter()
                .map(|&[depth, gain]| [depth as f32, gain as f32, 0.0, 0.0]),
        );
        let rows = records.len().div_ceil(width);
        let limit = device.limits().max_texture_dimension_2d as usize;
        if width > limit || rows > limit {
            return Err("rain canopy exceeds the device texture dimension limit".into());
        }
        records.resize(width * rows, [0.0; 4]);
        let extent = wgpu::Extent3d {
            width: width as u32,
            height: rows as u32,
            depth_or_array_layers: 1,
        };
        let canopy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rain canopy and depth response"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            canopy.as_image_copy(),
            bytemuck::cast_slice(&records),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(extent.width * 16),
                rows_per_image: None,
            },
            extent,
        );
        let surface = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rain surface settings"),
            size: size_of::<RainSurface>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let [a, b] = model.profile().patches;
        let settings = RainSurface {
            lengths: [a as f32, b as f32, 0.0, 0.0],
            canopy: [
                origin[0],
                origin[1],
                grid.cell as f32,
                grid.edge_width as f32,
            ],
            grid: [columns as u32, height as u32, response.len() as u32, 0],
            // Preserve the physical filtering range used by the two bands.
            footprint_scale: [
                (a / 1.28) as f32,
                (b / 2.1) as f32,
                code as f32,
                depth_start as f32,
            ],
        };
        Ok(Self {
            bands: Bands::in_cascades(device, model, cascades)
                .map_err(|error| error.to_string())?,
            surface,
            canopy: canopy.create_view(&wgpu::TextureViewDescriptor::default()),
            canopy_bytes: size_of_val(records.as_slice()) as u64,
            sampler,
            settings,
            since_run: 0,
            last_time: None,
            pending: false,
            active: false,
            recorded_time: 0.0,
            recorded_drops: [0; 2],
        })
    }

    pub(super) fn bindings() -> [wgpu::BindGroupLayoutEntry; 3] {
        let stages = wgpu::ShaderStages::VERTEX_FRAGMENT;
        [
            binding::uniform(19, stages, 0),
            binding::texture(
                20,
                stages,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2,
            ),
            binding::sampler(21, stages, wgpu::SamplerBindingType::Filtering),
        ]
    }

    pub(super) fn entries(&self) -> [wgpu::BindGroupEntry<'_>; 3] {
        [
            wgpu::BindGroupEntry {
                binding: 19,
                resource: self.surface.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 20,
                resource: wgpu::BindingResource::TextureView(&self.canopy),
            },
            wgpu::BindGroupEntry {
                binding: 21,
                resource: wgpu::BindingResource::Sampler(&self.sampler),
            },
        ]
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "surface moments and wave height in GPU units"
    )]
    pub(super) fn record(
        &mut self,
        queue: &wgpu::Queue,
        rain: Option<&RainBands>,
        time: f64,
        cadence: u32,
        full: bool,
    ) {
        let live = rain.filter(|model| model.active(time));
        let active = live.is_some();
        self.pending = active
            && (full
                || self.last_time.is_none_or(|old| time <= old)
                || self.since_run + 1 >= cadence);
        if let Some(model) = live.filter(|_| self.pending) {
            self.bands.record(queue, model, time);
            self.recorded_drops = self.bands.drop_counts();
        }
        if !active {
            self.recorded_drops = [0; 2];
            self.invalidate();
        }
        self.active = active;
        self.recorded_time = time;
        self.settings.grid[3] = u32::from(active);
        self.settings.lengths[2] = live.map_or(0.0, |model| model.moments(time) as f32);
        self.settings.lengths[3] =
            live.map_or(0.0, |model| (4.0 * model.variance(time).sqrt()) as f32);
        queue.write_buffer(&self.surface, 0, bytemuck::bytes_of(&self.settings));
    }

    pub(super) fn invalidate(&mut self) {
        self.last_time = None;
    }

    pub(super) fn submitted(&mut self) {
        if self.pending {
            self.last_time = Some(self.recorded_time);
            self.since_run = 0;
        } else if self.active {
            self.since_run += 1;
        } else {
            self.invalidate();
        }
    }

    pub(super) fn bytes(&self) -> u64 {
        self.bands.bytes() + self.surface.size() + self.canopy_bytes
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::RainSurface;
    use thrum_test_support::{record, records};

    #[test]
    fn surface_settings_match_the_shader_layout() {
        records::named(
            "rain surface",
            &records::declarations(crate::RAIN_GAIN_WGSL),
            &[record!(RainSurface, lengths, canopy, grid, footprint_scale)],
        );
    }
}
