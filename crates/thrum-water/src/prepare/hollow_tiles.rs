use thrum_render::runtime::DeviceSite;
use thrum_water_model::{Hollows, NO_HOLLOW};
use wgpu::{Buffer, BufferDescriptor, BufferUsages, Queue};

/// One rectangle's fixed mesh vertices and its wet cells in rising-bed order.
pub(super) struct Rectangle {
    base: u32,
    size: [usize; 2],
    by_hollow: Vec<Vec<(f32, usize)>>,
    wet: Vec<usize>,
    marked: Vec<bool>,
}

impl Rectangle {
    #[expect(clippy::cast_possible_truncation, reason = "match GPU f32 bed heights")]
    pub(super) fn new(base: u32, hollows: &Hollows) -> Self {
        let mut by_hollow = vec![Vec::new(); hollows.hollow_count()];
        for (cell, (&label, &height)) in hollows.labels().iter().zip(hollows.heights()).enumerate()
        {
            if label != NO_HOLLOW {
                by_hollow[label as usize].push((height as f32, cell));
            }
        }
        for cells in &mut by_hollow {
            cells.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        }
        Self {
            base,
            size: hollows.lattice().2,
            wet: vec![0; by_hollow.len()],
            by_hollow,
            marked: vec![false; hollows.cell_count()],
        }
    }

    fn update(&mut self, levels: &mut impl Iterator<Item = [u32; 2]>) -> bool {
        let mut changed = false;
        for (cells, wet) in self.by_hollow.iter().zip(&mut self.wet) {
            let [current, _] = levels.next().expect("prepared hollow level");
            let count = cells.partition_point(|(height, _)| *height < f32::from_bits(current));
            changed |= *wet != count;
            *wet = count;
        }
        changed
    }

    fn append(&mut self, indices: &mut Vec<u32>) {
        self.marked.fill(false);
        let [width, height] = self.size;
        for (cells, &wet) in self.by_hollow.iter().zip(&self.wet) {
            for &(_, cell) in &cells[..wet] {
                let (x, z) = (cell % width, cell / width);
                for row in z.saturating_sub(1)..=(z + 1).min(height - 1) {
                    for column in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        let neighbour = row * width + column;
                        if !std::mem::replace(&mut self.marked[neighbour], true) {
                            let base =
                                self.base + 4 * u32::try_from(neighbour).expect("bounded cell");
                            indices.extend([
                                base,
                                base + 2,
                                base + 1,
                                base + 1,
                                base + 2,
                                base + 3,
                            ]);
                        }
                    }
                }
            }
        }
    }
}

/// Index buffer reserved once at preparation. Only its active prefix is drawn.
pub(crate) struct HollowTiles {
    pub(crate) buffer: Buffer,
    rectangles: Vec<Rectangle>,
    indices: Vec<u32>,
}

impl HollowTiles {
    pub(crate) fn draws_cell_at(&self, vertex: u32) -> bool {
        self.rectangles
            .iter()
            .find_map(|rectangle| {
                let cell = usize::try_from(vertex.checked_sub(rectangle.base)? / 4).ok()?;
                rectangle.marked.get(cell).copied()
            })
            .unwrap_or(false)
    }

    pub(super) fn new(
        device: &DeviceSite,
        rectangles: Vec<Rectangle>,
    ) -> Result<Option<Self>, String> {
        if rectangles.is_empty() {
            return Ok(None);
        }
        let capacity = 6 * rectangles
            .iter()
            .map(|body| body.marked.len())
            .sum::<usize>();
        let size = (capacity * size_of::<u32>()) as u64;
        if size > device.limits().max_buffer_size {
            return Err("water hollow indices exceed the maximum buffer size".into());
        }
        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("water hollows tile indices"),
            size,
            usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Some(Self {
            buffer,
            rectangles,
            indices: Vec::with_capacity(capacity),
        }))
    }

    pub(crate) fn record(&mut self, queue: &Queue, display: &[[u32; 2]]) {
        let mut levels = display.iter().copied();
        let mut changed = false;
        for rectangle in &mut self.rectangles {
            changed |= rectangle.update(&mut levels);
        }
        debug_assert!(levels.next().is_none());
        if !changed {
            return;
        }
        self.indices.clear();
        for rectangle in &mut self.rectangles {
            rectangle.append(&mut self.indices);
        }
        if !self.indices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.indices));
        }
    }

    pub(crate) fn index_count(&self) -> u32 {
        u32::try_from(self.indices.len()).expect("bounded hollow indices")
    }
}
