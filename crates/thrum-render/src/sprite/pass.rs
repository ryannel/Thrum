//! Sprite sheet bindings, changed-row uploads and the single ordered sprite draw.

use super::{
    SpriteFrame, SpriteFrameStats, SpriteRow, SpriteSampling, SpriteScene,
    world::{self, OrderedRow, SpritePose, SpriteWorld},
};
use crate::error::RenderError;
use crate::{
    capabilities::DeviceCapabilities,
    extract::SourceTick,
    graph::{CompiledPass, GraphColorAttachment, PipelineKey, ResourceId},
    records::{SpriteFrameRecord as GpuRect, SpriteRow as GpuRow, SpriteView as GpuView},
    render_config::SCENE_FORMAT,
    runtime::Instant,
    timing::GpuTimings,
};
use bytemuck::Zeroable;
use coverage::coverage_at;

mod cooked;
mod coverage;
use cooked::{CookedFrames, Missing};
mod sheet;
use wgpu::util::DeviceExt;

pub(crate) const PASS: &str = crate::pass_key::PassKey::Sprites.constant_name();
pub(super) const SHEET_SLOTS: usize = 8;

fn row_storage(world: u32, frame: u32, max_binding: u64) -> Result<(u32, u64), RenderError> {
    let total = world
        .checked_add(frame)
        .ok_or(RenderError::SpriteRowCapacity)?;
    if total == 0 {
        return Err(RenderError::EmptySpriteRowCapacity);
    }
    let bytes = u64::from(total) * size_of::<GpuRow>() as u64;
    if bytes > max_binding {
        return Err(RenderError::SpriteRowStorageLimit);
    }
    Ok((total, bytes))
}

#[cfg(test)]
mod tests;

pub(super) struct SpritePass {
    pub(super) key: PipelineKey,
    pipeline: wgpu::RenderPipeline,
    pub(super) rows: wgpu::Buffer,
    pub(super) order: wgpu::Buffer,
    pub(super) frames: wgpu::Buffer,
    pub(super) sheets: Vec<(wgpu::Texture, wgpu::TextureView)>,
    sampler: wgpu::Sampler,
    frame_bases: Vec<u32>,
    embedded_frames: Vec<GpuRect>,
    cooked: CookedFrames,
    world_capacity: usize,
    row_capacity: usize,
    world_source: Option<SourceTick>,
    cpu_rows: Vec<GpuRow>,
    frame_keys: Vec<OrderedRow>,
    frame_order: Vec<u32>,
    current_order: Vec<u32>,
    visible_world: Vec<u32>,
    uploaded_order: Vec<u32>,
}

pub(super) struct SpriteBindings {
    pub(super) view: wgpu::Buffer,
    group: wgpu::BindGroup,
    count: u32,
    sheet_revision: u64,
}

pub(super) const VIEW_BYTES: u64 = size_of::<GpuView>() as u64;

impl SpritePass {
    pub(super) fn target_inputs(&self) -> Self {
        // Target preparation binds the resident buffers without owning their upload state.
        Self {
            key: self.key,
            pipeline: self.pipeline.clone(),
            rows: self.rows.clone(),
            order: self.order.clone(),
            frames: self.frames.clone(),
            sheets: self.sheets.clone(),
            sampler: self.sampler.clone(),
            frame_bases: Vec::new(),
            embedded_frames: Vec::new(),
            cooked: CookedFrames::default(),
            world_capacity: self.world_capacity,
            row_capacity: self.row_capacity,
            world_source: None,
            cpu_rows: Vec::new(),
            frame_keys: Vec::new(),
            frame_order: Vec::new(),
            current_order: Vec::new(),
            visible_world: Vec::new(),
            uploaded_order: Vec::new(),
        }
    }

    pub(super) fn prepare(
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        scene: &SpriteScene,
        key: PipelineKey,
    ) -> Result<Self, RenderError> {
        if let Some(excess) = scene.sheets.get(SHEET_SLOTS) {
            return Err(RenderError::SpriteSheetLimit {
                name: excess.name.clone(),
                limit: SHEET_SLOTS,
            });
        }
        let limits = capabilities.limits();
        let max_binding = limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size);
        let (total, rows_bytes) =
            row_storage(scene.world_capacity, scene.row_capacity, max_binding)?;
        if scene
            .cooked
            .iter()
            .enumerate()
            .any(|(index, sheet)| sheet.slot as usize != scene.sheets.len() + index)
        {
            return Err(RenderError::InvalidSpriteSheet(
                "embedded sheet slots changed after cooked sheet registration".into(),
            ));
        }
        if scene.sheets.is_empty() && scene.cooked.is_empty() {
            return Err(RenderError::MissingSpriteSheets);
        }
        if !scene.order_band.is_finite() || scene.order_band < 0.0 {
            return Err(RenderError::InvalidSpriteOrderBand);
        }
        let frame_count: usize = scene
            .sheets
            .iter()
            .map(|entry| entry.sheet.frames.len())
            .sum();
        if frame_count as u64 > max_binding / size_of::<GpuRect>() as u64 {
            return Err(RenderError::SpriteFrameCapacity);
        }
        for entry in &scene.sheets {
            let source = &entry.sheet;
            let [width, height] = source.extent;
            if source
                .extent
                .into_iter()
                .any(|side| side == 0 || side > limits.max_texture_dimension_2d)
                || (u64::from(width) * u64::from(height)).checked_mul(4)
                    != Some(source.rgba.len() as u64)
                || !source.texels_per_unit.is_finite()
                || source.texels_per_unit <= 0.0
                || source.frames.is_empty()
                || source.frames.iter().any(|rect| {
                    (0..2).any(|axis| {
                        rect.size[axis] == 0
                            || rect.origin[axis]
                                .checked_add(rect.size[axis])
                                .is_none_or(|end| end > source.extent[axis])
                    })
                })
            {
                return Err(RenderError::InvalidSpriteSheet(entry.name.clone()));
            }
        }
        let device = capabilities.site();
        let mut table = Vec::with_capacity(frame_count);
        let mut frame_bases = Vec::with_capacity(scene.sheets.len());
        let mut sheets = Vec::with_capacity(SHEET_SLOTS);
        for entry in &scene.sheets {
            frame_bases
                .push(u32::try_from(table.len()).map_err(|_| RenderError::SpriteFrameCapacity)?);
            sheets.push(sheet::upload(device, queue, entry, &mut table)?);
        }
        if sheets.len() < SHEET_SLOTS {
            sheets.push(sheet::placeholder(device));
        }
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite filtering"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let frames = frame_buffer(device, &mut table);
        let rows = buffer(
            device,
            "sprite rows",
            rows_bytes,
            wgpu::BufferUsages::STORAGE,
        );
        let order = buffer(
            device,
            "sprite order",
            u64::from(total) * 4,
            wgpu::BufferUsages::STORAGE,
        );
        let shader = crate::shaders::Shader::Sprite.compile(device, PASS);
        let pipeline = device.render(&wgpu::RenderPipelineDescriptor {
            label: Some(PASS),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: SCENE_FORMAT,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            key,
            pipeline,
            rows,
            order,
            frames,
            sheets,
            sampler,
            frame_bases,
            embedded_frames: table,
            cooked: CookedFrames::default(),
            world_capacity: scene.world_capacity as usize,
            row_capacity: scene.row_capacity as usize,
            world_source: None,
            cpu_rows: vec![GpuRow::zeroed(); total as usize],
            frame_keys: Vec::with_capacity(scene.row_capacity as usize),
            frame_order: Vec::with_capacity(scene.row_capacity as usize),
            current_order: Vec::with_capacity(total as usize),
            visible_world: Vec::with_capacity(scene.world_capacity as usize),
            uploaded_order: Vec::with_capacity(total as usize),
        })
    }

    pub(super) fn forget_world_source(&mut self) {
        self.world_source = None;
    }

    pub(super) fn bind(&self, device: &wgpu::Device) -> SpriteBindings {
        let view = buffer(
            device,
            "sprite view",
            VIEW_BYTES,
            wgpu::BufferUsages::UNIFORM,
        );
        SpriteBindings {
            group: self.bind_group(device, &view, 0),
            view,
            count: 0,
            sheet_revision: 0,
        }
    }

    fn bind_group(
        &self,
        device: &wgpu::Device,
        view: &wgpu::Buffer,
        cooked_slots: usize,
    ) -> wgpu::BindGroup {
        let frames = self.cooked.buffer.as_ref().unwrap_or(&self.frames);
        let placeholder = &self.sheets.last().expect("at least one bound sheet").1;
        let embedded_slots = self.sheets.len().saturating_sub(1);
        let mut entries = Vec::with_capacity(5 + SHEET_SLOTS);
        for (binding, source) in [view, &self.rows, frames, &self.order]
            .into_iter()
            .enumerate()
        {
            entries.push(wgpu::BindGroupEntry {
                binding: u32::try_from(binding).expect("sprite buffer binding"),
                resource: source.as_entire_binding(),
            });
        }
        for slot in 0..SHEET_SLOTS {
            let texture = if cooked_slots != 0 && slot >= embedded_slots {
                self.cooked
                    .view(slot - embedded_slots)
                    .unwrap_or(placeholder)
            } else {
                self.sheets.get(slot).map_or(placeholder, |(_, view)| view)
            };
            entries.push(wgpu::BindGroupEntry {
                binding: 4 + u32::try_from(slot).expect("bounded sheet slot"),
                resource: wgpu::BindingResource::TextureView(texture),
            });
        }
        entries.push(wgpu::BindGroupEntry {
            binding: 12,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite inputs"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &entries,
        })
    }

    pub(super) fn sync_sheets(
        &mut self,
        capabilities: &DeviceCapabilities,
        scene: &SpriteScene,
        world: &mut SpriteWorld,
        bindings: &mut SpriteBindings,
    ) -> Result<std::time::Duration, RenderError> {
        if scene.cooked.is_empty() {
            return Ok(std::time::Duration::ZERO);
        }
        let changed = self.cooked.update(
            scene,
            capabilities.site(),
            capabilities.generation(),
            &self.embedded_frames,
        )?;
        if bindings.sheet_revision != self.cooked.revision {
            bindings.group =
                self.bind_group(capabilities.site(), &bindings.view, scene.cooked.len());
            bindings.sheet_revision = self.cooked.revision;
        }
        let same_tick = self.world_source == world.source();
        if changed || !same_tick {
            world.refresh_cooked(scene, same_tick, |row| self.resolve(scene, row).ok());
            let start = Instant::now();
            world.order_tick(scene.order_band);
            let ordering = start.elapsed();
            // Repack retained rows if a publication changes their table indices or density.
            self.forget_world_source();
            return Ok(ordering);
        }
        Ok(std::time::Duration::ZERO)
    }

    fn resolve(&self, scene: &SpriteScene, row: SpriteRow) -> Result<FrameInput, Missing> {
        if (row.sheet as usize) < scene.sheets.len() {
            embedded(scene, &self.frame_bases, row).ok_or(Missing::Frame)
        } else {
            self.cooked
                .frame(row.sheet as usize - scene.sheets.len(), row.frame)
        }
    }

    pub(super) fn upload(
        &mut self,
        queue: &wgpu::Queue,
        scene: &SpriteScene,
        world: &SpriteWorld,
        frame: SpriteFrame<'_>,
        extent: [u32; 2],
        alpha: f32,
        measure_coverage: bool,
        bindings: &mut SpriteBindings,
    ) -> Result<crate::sprite::graph::SpriteUpload, RenderError> {
        let clip_scale = frame.view.clip_scale(extent)?;
        let mut stats = SpriteFrameStats {
            sheet_generation: self.cooked.generation(),
            view_centre: frame.view.centre,
            ..SpriteFrameStats::default()
        };
        let mut moving = 0;
        let mut dropped = world.dropped;
        let new_tick = self.world_source != world.source();
        if new_tick {
            self.upload_world(queue, scene, world, &mut stats);
            self.world_source = world.source();
        }

        self.frame_keys.clear();
        self.frame_order.clear();
        let mut coverage = 0.0;
        self.visible_world.clear();
        for &slot in &world.order {
            let row = world.row(slot);
            let Ok(input) = self
                .resolve(scene, row.current)
                .inspect_err(|reason| count_missing(&mut stats, *reason))
            else {
                dropped = dropped.saturating_add(1);
                continue;
            };
            let gpu = &self.cpu_rows[slot as usize];
            let Some(area) = coverage_at(input, gpu, frame.view, extent, alpha, measure_coverage)
            else {
                dropped = dropped.saturating_add(1);
                continue;
            };
            self.visible_world.push(slot);
            moving += u32::from(row.moving);
            coverage += area;
        }

        let mut frame_bytes = 0;
        let mut frame_run = None;
        for (index, &row) in frame.rows.iter().enumerate() {
            if self.frame_keys.len() == self.row_capacity {
                break;
            }
            let Some(key) = world::describe(scene, row, true, index as u128) else {
                continue;
            };
            let Ok(input) = self
                .resolve(scene, row)
                .inspect_err(|reason| count_missing(&mut stats, *reason))
            else {
                continue;
            };
            let gpu = pack(input, row, row.into());
            let Some(area) = coverage_at(input, &gpu, frame.view, extent, 1.0, measure_coverage)
            else {
                continue;
            };
            let slot = self.world_capacity + self.frame_keys.len();
            if bytemuck::bytes_of(&self.cpu_rows[slot]) != bytemuck::bytes_of(&gpu) {
                self.cpu_rows[slot] = gpu;
                frame_run.get_or_insert(slot);
            } else if let Some(first) = frame_run.take() {
                frame_bytes += write_rows(queue, &self.rows, &self.cpu_rows, first, slot);
            }
            self.frame_order
                .push(u32::try_from(slot).map_err(|_| RenderError::SpriteRowCapacity)?);
            self.frame_keys.push(key);
            coverage += area;
        }
        let count = self.frame_keys.len();
        dropped =
            dropped.saturating_add(u32::try_from(frame.rows.len() - count).unwrap_or(u32::MAX));
        if let Some(first) = frame_run {
            frame_bytes += write_rows(
                queue,
                &self.rows,
                &self.cpu_rows,
                first,
                self.world_capacity + count,
            );
        }
        let uploaded_frame_rows = u32::try_from(frame_bytes / size_of::<GpuRow>())
            .map_err(|_| RenderError::SpriteRowCapacity)?;

        let start = Instant::now();
        let key = |slot: u32| {
            let index = slot as usize;
            if index < self.world_capacity {
                world.row(slot).ordered.key
            } else {
                self.frame_keys[index - self.world_capacity].key
            }
        };
        self.frame_order
            .sort_unstable_by(|left, right| key(*left).compare(key(*right)));
        self.current_order.clear();
        world::merge_order(
            &self.visible_world,
            &self.frame_order,
            &mut self.current_order,
            |left, right| key(*left).compare(key(*right)),
        );
        stats.order_cpu = start.elapsed();
        if (new_tick || self.current_order != self.uploaded_order) && !self.current_order.is_empty()
        {
            let bytes = bytemuck::cast_slice(&self.current_order);
            queue.write_buffer(&self.order, 0, bytes);
            stats.order_upload_bytes = bytes.len();
        }
        self.uploaded_order.clone_from(&self.current_order);
        let [x, y, x_low, y_low] = split(frame.view.centre);
        queue.write_buffer(
            &bindings.view,
            0,
            bytemuck::bytes_of(&GpuView {
                clip_per_world: clip_scale,
                centre_high: [x, y],
                centre_low: [x_low, y_low],
                alpha,
                pixels_per_unit: frame.view.pixels_per_unit,
            }),
        );

        bindings.count =
            u32::try_from(self.current_order.len()).map_err(|_| RenderError::SpriteRowCapacity)?;
        stats.world_rows =
            u32::try_from(self.visible_world.len()).map_err(|_| RenderError::SpriteRowCapacity)?;
        stats.frame_rows = u32::try_from(count).map_err(|_| RenderError::SpriteRowCapacity)?;
        stats.shaded_pixels = coverage;
        Ok(crate::sprite::graph::SpriteUpload {
            rows: stats.world_upload_rows + uploaded_frame_rows,
            bytes: size_of::<GpuView>()
                + frame_bytes
                + stats.world_upload_bytes
                + stats.order_upload_bytes,
            dropped,
            moving,
            clip_scale,
            stats,
        })
    }

    fn upload_world(
        &mut self,
        queue: &wgpu::Queue,
        scene: &SpriteScene,
        world: &SpriteWorld,
        stats: &mut SpriteFrameStats,
    ) {
        // These buffers belong to the queue rather than a frame slot. The
        // host's exclusive frame token serializes encoder preparation, writes
        // and submission; earlier submissions see their earlier queue contents.
        let mut run = None;
        for (index, saved) in world.slots.iter().enumerate() {
            let updated = saved
                .filter(|row| Some(row.source) == world.source())
                .and_then(|row| {
                    let input = self.resolve(scene, row.current).ok()?;
                    Some(pack(input, row.current, row.previous))
                });
            let changed = updated
                .filter(|row| bytemuck::bytes_of(row) != bytemuck::bytes_of(&self.cpu_rows[index]));
            if let Some(row) = changed {
                self.cpu_rows[index] = row;
                run.get_or_insert(index);
                stats.world_upload_rows += 1;
            } else if let Some(first) = run.take() {
                stats.world_upload_bytes +=
                    write_rows(queue, &self.rows, &self.cpu_rows, first, index);
            }
        }
        if let Some(first) = run {
            stats.world_upload_bytes += write_rows(
                queue,
                &self.rows,
                &self.cpu_rows,
                first,
                self.world_capacity,
            );
        }
    }

    pub(super) fn draw(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        compiled: &CompiledPass,
        resource: &ResourceId,
        view: &wgpu::TextureView,
        bindings: &SpriteBindings,
        timings: Option<&mut GpuTimings>,
    ) -> Result<u32, RenderError> {
        let mut pass = compiled
            .begin_render(
                encoder,
                [Some(GraphColorAttachment {
                    resource,
                    range: 0..1,
                    view,
                    clear: wgpu::Color::BLACK,
                })],
                None,
                self.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindings.group, &[]);
        pass.draw(0..4, 0..bindings.count);
        Ok(1)
    }
}

fn buffer(
    device: &wgpu::Device,
    label: &str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn write_rows(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    rows: &[GpuRow],
    first: usize,
    end: usize,
) -> usize {
    let bytes = bytemuck::cast_slice(&rows[first..end]);
    if !bytes.is_empty() {
        queue.write_buffer(buffer, (first * size_of::<GpuRow>()) as u64, bytes);
    }
    bytes.len()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "split f64 supplies two f32 components at the GPU boundary"
)]
fn split([x, y]: [f64; 2]) -> [f32; 4] {
    let high = [x as f32, y as f32];
    [
        high[0],
        high[1],
        (x - f64::from(high[0])) as f32,
        (y - f64::from(high[1])) as f32,
    ]
}

#[derive(Clone, Copy)]
pub(super) struct FrameInput {
    index: u32,
    pub(super) size: [u32; 2],
    pub(super) offset: [f32; 2],
    pub(super) density: f32,
    sampling: SpriteSampling,
    tile: bool,
}

fn embedded(scene: &SpriteScene, bases: &[u32], row: SpriteRow) -> Option<FrameInput> {
    let slot = row.sheet as usize;
    let sheet = scene.sheets.get(slot)?;
    let rect = sheet.sheet.frames.get(row.frame as usize)?;
    Some(FrameInput {
        index: bases[slot] + row.frame,
        size: rect.size,
        offset: [0.0; 2],
        density: sheet.sheet.texels_per_unit,
        sampling: sheet.sampling,
        tile: sheet.tiles.contains(&row.frame),
    })
}

fn count_missing(stats: &mut SpriteFrameStats, missing: Missing) {
    let counter = match missing {
        Missing::Sheet => &mut stats.pending_sheet_rows,
        Missing::Frame => &mut stats.missing_frame_rows,
    };
    *counter = counter.saturating_add(1);
}

fn frame_buffer(device: &wgpu::Device, table: &mut Vec<GpuRect>) -> wgpu::Buffer {
    if table.is_empty() {
        table.push(GpuRect::zeroed());
    }
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite frames"),
        contents: bytemuck::cast_slice(table),
        usage: wgpu::BufferUsages::STORAGE,
    })
}

fn pack(input: FrameInput, row: SpriteRow, previous: SpritePose) -> GpuRow {
    let units = input.density;
    let [x, y] = row.scale.map(|scale| scale / units);
    let [old_x, old_y] = previous.scale.map(|scale| scale / units);
    let (rotation, delta) = angles(previous.rotation, row.rotation);
    GpuRow {
        current: split(row.position),
        previous: split(previous.position),
        scales: [old_x, old_y, x, y],
        rotation,
        rotation_delta: delta,
        frame: input.index,
        sheet: row.sheet,
        tint: row.tint,
        flash: row.flash,
        additive: row.additive,
        padding0: 0,
        trim_from_pivot: input.offset,
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "one normalized turn fits in f32 GPU radians"
)]
fn angles(previous: f32, current: f32) -> (f32, f32) {
    let turn = std::f64::consts::TAU;
    let half = std::f64::consts::PI;
    let previous = f64::from(previous) % turn;
    let current = f64::from(current) % turn;
    let delta = (current - previous + half).rem_euclid(turn) - half;
    (previous as f32, delta as f32)
}
