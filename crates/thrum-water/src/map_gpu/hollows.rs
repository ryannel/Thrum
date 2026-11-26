//! Prepared bed and label cells, with separate submitted endpoints for their levels.
use super::BODY_LIMIT;
use crate::WaterError;
use bytemuck::{Pod, Zeroable};
use thrum_render::{binding, contribution::EffectTime};
use thrum_water_model::{BodyKind, Map};
use wgpu::util::DeviceExt;

const BINDING: u32 = 18;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Rectangle {
    origin_spacing: [f32; 4],
    size_offsets: [u32; 4],
    optics_offset: [u32; 4],
}

type Cell = [u32; 2];
type Levels = [Vec<f64>; BODY_LIMIT];

pub(super) struct HollowsGpu {
    buffer: wgpu::Buffer,
    level_offset: u64,
    recorded: Option<EffectTime>,
    submitted: Option<EffectTime>,
    from: Levels,
    to: Levels,
    committed_from: Levels,
    committed_to: Levels,
    display: Vec<Cell>,
    committed_display: Vec<Cell>,
    uploaded: Vec<Cell>,
}

impl HollowsGpu {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "checked GPU coordinates and level table"
    )]
    pub(super) fn new(device: &wgpu::Device, map: &Map) -> Result<Option<Self>, WaterError> {
        let mut rectangles = [Rectangle::zeroed(); BODY_LIMIT];
        let mut cells = Vec::new();
        let mut optical_types = Vec::<Cell>::new();
        let mut from: Levels = std::array::from_fn(|_| Vec::new());
        let mut level_count = 0;
        let limit = device.limits();
        let unsupported =
            || WaterError::UnsupportedMap("hollows exceed the device or f32 lattice limits");
        for (id, rectangle) in rectangles.iter_mut().enumerate() {
            let Some(BodyKind::Hollows(hollows)) = map.body(id).map(|body| &body.kind) else {
                continue;
            };
            let ([x, z], spacing, [width, height]) = hollows.lattice();
            let grid = [x as f32, z as f32, spacing as f32, 0.0];
            if grid.iter().any(|n| !n.is_finite())
                || grid[2] <= 0.0
                || hollows
                    .heights()
                    .iter()
                    .chain(hollows.levels())
                    .any(|&n| !(n as f32).is_finite())
            {
                return Err(unsupported());
            }
            let byte_count = size_of::<[Rectangle; BODY_LIMIT]>()
                + (cells.len()
                    + hollows.cell_count()
                    + level_count
                    + hollows.hollow_count()
                    + optical_types.len()
                    + 2 * hollows.optical_types().len())
                    * size_of::<Cell>();
            if byte_count as u64 > limit.max_storage_buffer_binding_size
                || byte_count as u64 > limit.max_buffer_size
            {
                return Err(unsupported());
            }
            *rectangle = Rectangle {
                origin_spacing: grid,
                size_offsets: [
                    u32::try_from(width).map_err(|_| unsupported())?,
                    u32::try_from(height).map_err(|_| unsupported())?,
                    u32::try_from(cells.len()).map_err(|_| unsupported())?,
                    u32::try_from(level_count).map_err(|_| unsupported())?,
                ],
                optics_offset: [
                    if hollows.optical_types().is_empty() {
                        u32::MAX
                    } else {
                        u32::try_from(optical_types.len()).map_err(|_| unsupported())?
                    },
                    0,
                    0,
                    0,
                ],
            };
            optical_types.extend(hollows.optical_types().iter().flat_map(|optics| {
                optics.map_or([[0; 2]; 2], |optics| {
                    let [r, g, b] = optics.scatter.map(f32::to_bits);
                    [[r, g], [b, optics.extinction.to_bits()]]
                })
            }));
            cells.extend(
                hollows
                    .heights()
                    .iter()
                    .zip(hollows.labels())
                    .map(|(&bed, &label)| [(bed as f32).to_bits(), label]),
            );
            from[id] = hollows.levels().to_vec();
            level_count += hollows.hollow_count();
        }
        if cells.is_empty() {
            return Ok(None);
        }
        let cell_count = u32::try_from(cells.len()).map_err(|_| unsupported())?;
        let types_start = u32::try_from(cells.len() + level_count).map_err(|_| unsupported())?;
        for rectangle in &mut rectangles {
            rectangle.size_offsets[3] += cell_count;
            if rectangle.optics_offset[0] != u32::MAX {
                rectangle.optics_offset[0] += types_start;
            }
        }
        let mut contents = bytemuck::cast_slice(&rectangles).to_vec();
        contents.extend_from_slice(bytemuck::cast_slice(&cells));
        let level_offset = contents.len() as u64;
        let display = vec![[0; 2]; level_count];
        contents.extend_from_slice(bytemuck::cast_slice(&display));
        contents.extend_from_slice(bytemuck::cast_slice(&optical_types));
        // A WGSL runtime array requires space for at least one cell. Every prepared
        // rectangle has at least one bed cell, even when there are no labelled hollows.
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water hollows"),
            contents: &contents,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        Ok(Some(Self {
            buffer,
            level_offset,
            recorded: None,
            submitted: None,
            to: from.clone(),
            committed_from: from.clone(),
            committed_to: from.clone(),
            from,
            committed_display: display.clone(),
            uploaded: display.clone(),
            display,
        }))
    }

    pub(super) fn binding(&self, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
        binding::storage(BINDING, visibility, true, self.bytes())
    }

    pub(super) fn bytes(&self) -> u64 {
        self.buffer.size()
    }

    pub(super) fn entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: BINDING,
            resource: self.buffer.as_entire_binding(),
        }
    }

    #[expect(clippy::cast_possible_truncation, reason = "GPU levels use f32 metres")]
    pub(super) fn record(&mut self, queue: &wgpu::Queue, map: &Map, time: Option<EffectTime>) {
        let old = self
            .submitted
            .zip(time)
            .filter(|(old, time)| old.epoch == time.epoch && old.timeline == time.timeline);
        let same_tick = old.is_some_and(|(old, now)| old.tick == now.tick);
        let adjacent = old.is_some_and(|(old, now)| old.tick.checked_add(1) == Some(now.tick));
        let alpha = time.map_or(1.0, |time| f64::from(time.alpha));
        let mut offset = 0;
        for (id, target) in self.to.iter_mut().enumerate() {
            let Some(BodyKind::Hollows(hollows)) = map.body(id).map(|body| &body.kind) else {
                continue;
            };
            target.copy_from_slice(if same_tick {
                &self.committed_to[id]
            } else {
                hollows.levels()
            });
            let from = &mut self.from[id];
            from.copy_from_slice(if same_tick {
                &self.committed_from[id]
            } else if adjacent {
                &self.committed_to[id]
            } else {
                target
            });
            for (&a, &b) in from.iter().zip(target.iter()) {
                let level = if alpha >= 1.0 {
                    b
                } else {
                    (b - a).mul_add(alpha, a)
                };
                let current = (level as f32).to_bits();
                let previous = old.map_or(current, |_| self.committed_display[offset][0]);
                self.display[offset] = [current, previous];
                offset += 1;
            }
        }
        self.recorded = time;
        self.upload(queue);
    }

    pub(super) fn commit(&mut self) {
        self.submitted = self.recorded;
        self.committed_from.clone_from(&self.from);
        self.committed_to.clone_from(&self.to);
        self.committed_display.copy_from_slice(&self.display);
    }

    pub(super) fn copy_display(&mut self, queue: &wgpu::Queue, frame: &Self) {
        self.display.copy_from_slice(&frame.committed_display);
        self.upload(queue);
    }

    fn upload(&mut self, queue: &wgpu::Queue) {
        if self.uploaded != self.display {
            queue.write_buffer(
                &self.buffer,
                self.level_offset,
                bytemuck::cast_slice(&self.display),
            );
            self.uploaded.copy_from_slice(&self.display);
        }
    }
}
