use crate::Error;
use thrum_cloud_model::{Ground, Phase, PhaseTable};
use thrum_render::binding;

const PHASE_ENTRIES: u32 = 1025;
const PHASE_ROW_BYTES: u64 = 16 + (PHASE_ENTRIES as u64 * 4).next_multiple_of(16);
pub(crate) const FIRST: u32 = 14;

pub(crate) struct LightTable {
    pub(crate) buffer: wgpu::Buffer,
    phases: Vec<Phase>,
    radius: Option<f32>,
}

impl LightTable {
    pub(crate) fn new(device: &wgpu::Device, fog: &Ground) -> Result<Self, Error> {
        let phases: Vec<_> = fog
            .rows()
            .iter()
            .map(|r| Phase::fit(r.droplet_radius * 2e6))
            .collect::<Result<_, _>>()
            .map_err(Error::Model)?;
        let size = PHASE_ROW_BYTES * phases.len() as u64;
        if size > device.limits().max_storage_buffer_binding_size {
            return Err(Error::DeviceLimit("cloud fog phase tables"));
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cloud fog phase tables"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            buffer,
            phases,
            radius: None,
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "unit droplet moments in GPU precision"
    )]
    pub(crate) fn upload(&mut self, queue: &wgpu::Queue, radius: f32) -> Result<(), Error> {
        if self.radius == Some(radius) {
            return Ok(());
        }
        for (row, &phase) in self.phases.iter().enumerate() {
            let start = row as u64 * PHASE_ROW_BYTES;
            let droplet = [
                phase.forward_share(),
                phase.parameters()[3],
                phase.broad_mean_cosine(),
                phase.mean_cosine(),
            ]
            .map(|moment| moment as f32);
            let table =
                PhaseTable::new(phase, f64::from(radius), PHASE_ENTRIES).map_err(Error::Model)?;
            queue.write_buffer(&self.buffer, start, bytemuck::cast_slice(&droplet));
            queue.write_buffer(
                &self.buffer,
                start + 16,
                bytemuck::cast_slice(table.values()),
            );
        }
        self.radius = Some(radius);
        Ok(())
    }

    pub(crate) fn source(group: u32) -> String {
        format!(
            "{}\nconst GROUND_PHASE_ENTRIES:u32={PHASE_ENTRIES}u;\n{}
             @group({group}) @binding(17) var ground_band_light: texture_3d<f32>;
             @group({group}) @binding(18) var<uniform> ground_light_window: GroundBandWindow;
             @group({group}) @binding(19) var<storage, read> ground_phase: array<GroundOptics>;
             {}{}",
            super::GroundField::source(group, FIRST),
            include_str!("window.wgsl"),
            include_str!("light_type.wgsl"),
            include_str!("light.wgsl"),
        )
    }

    pub(crate) fn layout() -> [wgpu::BindGroupLayoutEntry; 3] {
        let stages = wgpu::ShaderStages::COMPUTE;
        [
            binding::texture(
                17,
                stages,
                wgpu::TextureSampleType::Float { filterable: true },
                wgpu::TextureViewDimension::D3,
            ),
            binding::uniform(
                18,
                stages,
                size_of::<thrum_render::atmosphere::GroundBandMapping>() as u64,
            ),
            binding::storage(19, stages, true, PHASE_ROW_BYTES),
        ]
    }

    pub(crate) fn entries<'a>(
        &'a self,
        band: &'a super::band::Band,
    ) -> [wgpu::BindGroupEntry<'a>; 3] {
        [
            wgpu::BindGroupEntry {
                binding: 17,
                resource: wgpu::BindingResource::TextureView(&band.view),
            },
            wgpu::BindGroupEntry {
                binding: 18,
                resource: band.mapping.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 19,
                resource: self.buffer.as_entire_binding(),
            },
        ]
    }
}

pub(crate) fn trace_source(ground: bool) -> String {
    if ground {
        LightTable::source(2) + include_str!("trace.wgsl") + include_str!("deck.wgsl")
    } else {
        concat!(
            include_str!("light_type.wgsl"),
            include_str!("disabled.wgsl"),
            include_str!("no_deck.wgsl"),
        )
        .into()
    }
}
