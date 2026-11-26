//! The water surface's gain and filtering inputs for the optional rain bands.
use thrum_render::{
    binding,
    contribution::{QualityInvalidation, QualityKnob, QualitySettings},
    runtime::DeviceSite,
};
use thrum_sea::rain::Bands;
use thrum_water_model::RainBands;
use wgpu::util::DeviceExt;

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
}
pub(super) struct Rain {
    pub(super) bands: Bands,
    surface: wgpu::Buffer,
    columns: wgpu::Buffer,
    settings: RainSurface,
    since_run: u32,
    last_time: Option<f64>,
    recorded_time: f64,
    pub(super) pending: bool,
    active: bool,
    pub(super) recorded_drops: [usize; 2],
}

impl Rain {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "validated canopy and depth response in GPU units"
    )]
    pub(super) fn new(device: &DeviceSite, model: &RainBands) -> Result<Self, String> {
        let grid = model
            .canopy_grid()
            .ok_or("rain bands need prepared canopy columns")?;
        let mut records: Vec<[f32; 4]> = model
            .table()
            .depth_response
            .iter()
            .map(|&[depth, gain]| [depth as f32, gain as f32, 0.0, 0.0])
            .collect();
        let response =
            u32::try_from(records.len()).map_err(|_| "rain depth response is too large")?;
        records.extend(
            grid.columns
                .iter()
                .map(|&[[h0, s0], [h1, s1]]| [h0 as f32, s0 as f32, h1 as f32, s1 as f32]),
        );
        if size_of_val(records.as_slice()) as u64 > device.limits().max_storage_buffer_binding_size
        {
            return Err("rain canopy exceeds the device storage binding limit".into());
        }
        let columns = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("rain depth response and canopy columns"),
            contents: bytemuck::cast_slice(&records),
            usage: wgpu::BufferUsages::STORAGE,
        });
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
                grid.origin[0] as f32,
                grid.origin[1] as f32,
                grid.cell as f32,
                grid.edge_width as f32,
            ],
            grid: [grid.size[0] as u32, grid.size[1] as u32, response, 0],
        };
        Ok(Self {
            bands: Bands::new(device, model).map_err(|error| error.to_string())?,
            surface,
            columns,
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
            binding::storage(20, stages, true, 0),
            binding::texture(
                21,
                stages,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D2Array,
            ),
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
                resource: self.columns.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 21,
                resource: wgpu::BindingResource::TextureView(self.bands.view()),
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
        rain: &RainBands,
        time: f64,
        cadence: u32,
        full: bool,
    ) {
        let active = rain.active(time);
        self.pending = active
            && (full
                || self.last_time.is_none_or(|old| time <= old)
                || self.since_run + 1 >= cadence);
        if self.pending || full {
            self.bands.record(queue, rain, time);
            self.recorded_drops = self.bands.drop_counts();
        }
        if !active {
            self.recorded_drops = [0; 2];
        }
        self.active = active;
        self.recorded_time = time;
        self.settings.grid[3] = u32::from(active);
        self.settings.lengths[2] = rain.moments(time) as f32;
        self.settings.lengths[3] = (4.0 * rain.variance(time).sqrt()) as f32;
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
        self.bands.bytes() + self.surface.size() + self.columns.size()
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
            &[record!(RainSurface, lengths, canopy, grid)],
        );
    }
}
