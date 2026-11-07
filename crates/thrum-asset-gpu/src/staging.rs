//! Staged uploads: filled off the render thread, copied on the GPU under a per-frame byte budget.
//!
//! `Queue::write_buffer`/`write_texture` copy into wgpu's staging memory on the calling thread
//! and flush the whole lot at the next `Queue::submit`, so a 100 MiB volume written that way lands
//! in one frame's submission. Here the loader thread writes product bytes once into its own
//! `mapped_at_creation` staging buffer (rows re-padded to the copy alignment) and unmaps it. The
//! render thread then records at most a budget of `copy_buffer_to_*` commands into each frame's
//! own encoder; the frame host stays the only submitter. A set becomes drawable in the frame that
//! records its last copy, because those copies precede the forward pass in that command buffer.

use thrum_asset::{Ktx2, Ktx2Format};

/// Largest single copy the planner emits; budgets are spent in units no larger than this.
pub const COPY_UNIT: u64 = 1 << 20;

/// Where one staged range goes.
pub(crate) enum Destination {
    Buffer {
        buffer: wgpu::Buffer,
        offset: u64,
    },
    Texture {
        texture: wgpu::Texture,
        mip_level: u32,
        origin: wgpu::Origin3d,
        extent: wgpu::Extent3d,
        bytes_per_row: u32,
        rows_per_image: u32,
    },
}

/// One recorded copy.
pub(crate) struct Copy {
    source_offset: u64,
    size: u64,
    destination: Destination,
}

/// Staging bytes and the copies that drain them. Recording is tentative: [`Self::encode`] records
/// from the last committed copy without consuming anything, and only [`Self::commit`] (called
/// once the encoder holding those copies was submitted) advances. A discarded frame's copies are
/// simply recorded again.
pub(crate) struct StagedUpload {
    staging: Option<wgpu::Buffer>,
    copies: Vec<Copy>,
    total: u64,
    next: usize,
}

/// Copies recorded in one encoder, not yet committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// Index one past the last recorded copy.
    pub(crate) end: usize,
    /// Bytes recorded.
    pub(crate) bytes: u64,
}

impl StagedUpload {
    /// Bytes this upload moves (including row padding).
    pub(crate) fn total(&self) -> u64 {
        self.total
    }

    /// Whether every copy has been committed.
    pub(crate) fn is_done(&self) -> bool {
        self.next == self.copies.len()
    }

    /// Whether `recorded` reaches the last copy.
    pub(crate) fn completes(&self, recorded: Recorded) -> bool {
        recorded.end == self.copies.len()
    }

    /// Record copies from the last committed one until `budget` bytes have been spent (always at
    /// least one copy, so a set always progresses).
    pub(crate) fn encode(&self, encoder: &mut wgpu::CommandEncoder, budget: u64) -> Recorded {
        let mut recorded = Recorded {
            end: self.next,
            bytes: 0,
        };
        let Some(staging) = self.staging.as_ref() else {
            return recorded;
        };
        for copy in &self.copies[self.next..] {
            if recorded.bytes > 0 && recorded.bytes + copy.size > budget {
                break;
            }
            match &copy.destination {
                Destination::Buffer { buffer, offset } => encoder.copy_buffer_to_buffer(
                    staging,
                    copy.source_offset,
                    buffer,
                    *offset,
                    copy.size,
                ),
                Destination::Texture {
                    texture,
                    mip_level,
                    origin,
                    extent,
                    bytes_per_row,
                    rows_per_image,
                } => encoder.copy_buffer_to_texture(
                    wgpu::TexelCopyBufferInfo {
                        buffer: staging,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: copy.source_offset,
                            bytes_per_row: Some(*bytes_per_row),
                            rows_per_image: Some(*rows_per_image),
                        },
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture,
                        mip_level: *mip_level,
                        origin: *origin,
                        aspect: wgpu::TextureAspect::All,
                    },
                    *extent,
                ),
            }
            recorded.bytes += copy.size;
            recorded.end += 1;
        }
        recorded
    }

    /// The encoder holding `recorded` was submitted.
    pub(crate) fn commit(&mut self, recorded: Recorded) {
        if recorded.end > self.next && recorded.end <= self.copies.len() {
            self.next = recorded.end;
        }
        if self.is_done() {
            // Submitted copies keep the buffer alive until the GPU is done with it.
            self.staging = None;
        }
    }
}

/// One piece of source data and where it goes.
enum Piece<'a> {
    Buffer {
        bytes: &'a [u8],
        buffer: wgpu::Buffer,
    },
    Texture {
        texture: &'a Ktx2,
        destination: wgpu::Texture,
    },
}

/// Collects pieces on the loader thread, then writes one staging buffer.
#[derive(Default)]
pub(crate) struct StagingPlan<'a> {
    pieces: Vec<Piece<'a>>,
}

impl<'a> StagingPlan<'a> {
    pub(crate) fn buffer(&mut self, bytes: &'a [u8], buffer: wgpu::Buffer) {
        self.pieces.push(Piece::Buffer { bytes, buffer });
    }

    pub(crate) fn texture(&mut self, texture: &'a Ktx2, destination: wgpu::Texture) {
        self.pieces.push(Piece::Texture {
            texture,
            destination,
        });
    }

    /// Lay out, allocate, fill and unmap the staging buffer.
    ///
    /// # Errors
    /// The staging size exceeds the device's buffer limit, or mapping failed.
    pub(crate) fn finish(self, device: &wgpu::Device) -> Result<StagedUpload, String> {
        let mut copies = Vec::new();
        let mut writes: Vec<(u64, Write<'a>)> = Vec::new();
        let mut cursor = 0u64;
        for piece in self.pieces {
            match piece {
                Piece::Buffer { bytes, buffer } => {
                    cursor = cursor.next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
                    let size = (bytes.len() as u64).next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
                    writes.push((cursor, Write::Packed(bytes)));
                    let mut done = 0;
                    while done < size {
                        let part = (size - done).min(COPY_UNIT);
                        copies.push(Copy {
                            source_offset: cursor + done,
                            size: part,
                            destination: Destination::Buffer {
                                buffer: buffer.clone(),
                                offset: done,
                            },
                        });
                        done += part;
                    }
                    cursor += size;
                }
                Piece::Texture {
                    texture,
                    destination,
                } => {
                    cursor = plan_texture(texture, &destination, cursor, &mut copies, &mut writes);
                }
            }
        }
        let total = cursor.max(wgpu::COPY_BUFFER_ALIGNMENT);
        if total > device.limits().max_buffer_size {
            return Err(format!(
                "staging needs {total} bytes, over this device's buffer limit"
            ));
        }
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thrum-asset-gpu staging"),
            size: total,
            usage: wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        });
        {
            let mut mapped = staging
                .get_mapped_range_mut(..)
                .map_err(|error| format!("staging map: {error}"))?;
            for (offset, write) in writes {
                write.fill(
                    &mut mapped,
                    usize::try_from(offset).map_err(|error| error.to_string())?,
                );
            }
        }
        staging.unmap();
        Ok(StagedUpload {
            staging: Some(staging),
            copies,
            total: cursor,
            next: 0,
        })
    }
}

/// How bytes are laid into staging.
enum Write<'a> {
    Packed(&'a [u8]),
    /// Tightly packed rows re-strided to `padded` bytes.
    Rows {
        data: &'a [u8],
        row: usize,
        padded: usize,
    },
}

impl Write<'_> {
    /// Write into mapped (possibly write-combining) staging memory: whole ranges, no reads.
    fn fill(&self, out: &mut wgpu::BufferViewMut, at: usize) {
        match *self {
            Write::Packed(bytes) => out.slice(at..at + bytes.len()).copy_from_slice(bytes),
            Write::Rows { data, row, padded } => {
                for (index, source) in data.chunks_exact(row).enumerate() {
                    let start = at + index * padded;
                    out.slice(start..start + row).copy_from_slice(source);
                }
            }
        }
    }
}

/// One copy region of a texture level, in slices and block rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Region {
    z: u32,
    depth: u32,
    row: u32,
    rows: u32,
    bytes: u64,
}

/// Split a level of `depth` slices, each `rows` block rows of `padded` bytes, into copies of at
/// most [`COPY_UNIT`]: whole slices while a slice fits, otherwise block rows within each slice
/// (a single block row is the smallest region and may exceed the unit only on its own).
fn regions(depth: u32, rows: u32, padded: u64) -> Vec<Region> {
    let slice = padded * u64::from(rows);
    let mut out = Vec::new();
    if depth > 1 && slice <= COPY_UNIT {
        let per_copy = u32::try_from((COPY_UNIT / slice.max(1)).max(1)).unwrap_or(u32::MAX);
        let mut z = 0;
        while z < depth {
            let count = per_copy.min(depth - z);
            out.push(Region {
                z,
                depth: count,
                row: 0,
                rows,
                bytes: u64::from(count) * slice,
            });
            z += count;
        }
        return out;
    }
    let per_copy = u32::try_from((COPY_UNIT / padded.max(1)).max(1)).unwrap_or(u32::MAX);
    for z in 0..depth {
        let mut row = 0;
        while row < rows {
            let count = per_copy.min(rows - row);
            out.push(Region {
                z,
                depth: 1,
                row,
                rows: count,
                bytes: u64::from(count) * padded,
            });
            row += count;
        }
    }
    out
}

/// Plan every level of a KTX2 product into copies of at most [`COPY_UNIT`] (see [`regions`]).
fn plan_texture<'a>(
    texture: &'a Ktx2,
    destination: &wgpu::Texture,
    mut cursor: u64,
    copies: &mut Vec<Copy>,
    writes: &mut Vec<(u64, Write<'a>)>,
) -> u64 {
    let format: Ktx2Format = texture.format();
    let (block_width, block_height) = format.block_dimensions();
    for (mip, level) in texture.levels().iter().enumerate() {
        let row = level.bytes_per_row(format);
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let rows = level.rows_per_image(format);
        cursor = cursor.next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
        let data = texture.level_data(level);
        writes.push((
            cursor,
            Write::Rows {
                data,
                row: row as usize,
                padded: padded as usize,
            },
        ));
        let physical_width = level.width.next_multiple_of(block_width);
        let slice = u64::from(padded) * u64::from(rows);
        let depth = if texture.is_3d() { level.depth } else { 1 };
        for region in regions(depth, rows, u64::from(padded)) {
            copies.push(Copy {
                source_offset: cursor
                    + u64::from(region.z) * slice
                    + u64::from(region.row) * u64::from(padded),
                size: region.bytes,
                destination: Destination::Texture {
                    texture: destination.clone(),
                    mip_level: u32::try_from(mip).unwrap_or(0),
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: region.row * block_height,
                        z: region.z,
                    },
                    extent: wgpu::Extent3d {
                        width: physical_width,
                        height: region.rows * block_height,
                        depth_or_array_layers: region.depth,
                    },
                    bytes_per_row: padded,
                    rows_per_image: rows,
                },
            });
        }
        cursor += slice * u64::from(level.depth);
    }
    cursor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn covers(regions: &[Region], depth: u32, rows: u32, padded: u64) {
        let mut cells = vec![0u8; (depth * rows) as usize];
        for region in regions {
            assert!(region.bytes <= COPY_UNIT.max(padded), "{region:?}");
            assert_eq!(
                region.bytes,
                u64::from(region.depth) * u64::from(region.rows) * padded
            );
            for z in region.z..region.z + region.depth {
                for row in region.row..region.row + region.rows {
                    cells[(z * rows + row) as usize] += 1;
                }
            }
        }
        assert!(cells.iter().all(|&cell| cell == 1), "every row copied once");
    }

    #[test]
    fn a_volume_slice_larger_than_the_copy_unit_splits_into_block_rows() {
        // 2048x2048 R8: one slice is 4 MiB.
        let regions = regions(4, 2048, 2048);
        assert_eq!(regions.len(), 16);
        assert!(regions.iter().all(|region| region.bytes == COPY_UNIT));
        covers(&regions, 4, 2048, 2048);
    }

    #[test]
    fn small_slices_group_and_2d_levels_split_by_rows() {
        let grouped = regions(160, 160, 256);
        covers(&grouped, 160, 160, 256);
        assert!(
            grouped
                .iter()
                .all(|region| region.row == 0 && region.rows == 160)
        );
        assert_eq!(grouped.len(), 160usize.div_ceil(25));
        let flat = regions(1, 512, 4096);
        covers(&flat, 1, 512, 4096);
        assert_eq!(flat.len(), 2);
        // A single row wider than the unit is still one copy.
        covers(&regions(1, 3, 2 * COPY_UNIT), 1, 3, 2 * COPY_UNIT);
    }
}
