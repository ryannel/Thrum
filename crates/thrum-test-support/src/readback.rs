//! Read GPU buffers and textures back to the CPU in a test, with one bounded wait.
//!
//! [`buffer`] and [`texture`] submit their own copy. A test that must read inside the encoder under test appends
//! the copy with [`Readback::buffer`] or [`Readback::texture`], submits that encoder itself, and passes the
//! submission to [`Readback::wait`]. Texture bytes come back tightly packed, without the copy's row padding.

use std::ops::{Bound, Range, RangeBounds};
use std::time::Duration;

/// The longest a readback waits for its submission before the test fails.
pub const WAIT: Duration = Duration::from_secs(120);

/// One mip level of a texture: all of its array layers or depth slices, or one of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mip {
    /// Mip level.
    pub level: u32,
    /// One array layer, or one depth slice of a 3D texture; `None` reads them all.
    pub layer: Option<u32>,
}

impl Mip {
    /// Every layer or slice of one mip level.
    #[must_use]
    pub const fn level(level: u32) -> Self {
        Self { level, layer: None }
    }

    /// One layer or depth slice of one mip level.
    #[must_use]
    pub const fn layer(level: u32, layer: u32) -> Self {
        Self {
            level,
            layer: Some(layer),
        }
    }
}

/// How the staging buffer's rows map to the packed bytes returned.
#[derive(Clone, Copy, Debug)]
struct Rows {
    /// Bytes of one row of blocks, as returned.
    packed: usize,
    /// Bytes of one row in the staging buffer.
    padded: usize,
    /// Rows of blocks across every image read.
    count: usize,
}

/// A copy appended to an encoder, read once that encoder's submission completes.
#[must_use = "a readback reads nothing until it is waited on"]
pub struct Readback {
    staging: wgpu::Buffer,
    rows: Rows,
    /// The packed bytes to return, a range of the whole read.
    bytes: Range<usize>,
    /// Bytes skipped at the start of the staging buffer, for a buffer range that starts unaligned.
    lead: usize,
}

fn bounds(range: &impl RangeBounds<u64>, len: u64) -> Range<u64> {
    let start = match range.start_bound() {
        Bound::Included(&start) => start,
        Bound::Excluded(&start) => start + 1,
        Bound::Unbounded => 0,
    };
    let end = match range.end_bound() {
        Bound::Included(&end) => end + 1,
        Bound::Excluded(&end) => end,
        Bound::Unbounded => len,
    };
    assert!(
        start <= end && end <= len,
        "readback range {start}..{end} outside {len} bytes"
    );
    start..end
}

fn usize_from(value: u64) -> usize {
    usize::try_from(value).expect("readback size fits in memory")
}

fn staging(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size: size.max(wgpu::COPY_BUFFER_ALIGNMENT),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    })
}

impl Readback {
    /// Append a copy of `range` of `buffer`, which needs `COPY_SRC`, to `encoder`.
    ///
    /// # Panics
    /// Fails if `range` lies outside the buffer.
    pub fn buffer(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        buffer: &wgpu::Buffer,
        range: impl RangeBounds<u64>,
    ) -> Self {
        let range = bounds(&range, buffer.size());
        // Copies start and end on the copy alignment; the extra bytes are dropped on return.
        let align = wgpu::COPY_BUFFER_ALIGNMENT;
        let start = range.start / align * align;
        let end = range.end.div_ceil(align) * align;
        let staging = staging(device, end - start);
        if end > start {
            encoder.copy_buffer_to_buffer(buffer, start, &staging, 0, end - start);
        }
        let len = usize_from(range.end - range.start);
        Self {
            staging,
            rows: Rows {
                packed: len,
                padded: len,
                count: 1,
            },
            bytes: 0..len,
            lead: usize_from(range.start - start),
        }
    }

    /// Append a copy of one mip, or one layer of it, of a 2D, 3D or array `texture`, which needs `COPY_SRC`,
    /// to `encoder`. `range` selects bytes of the packed read: rows of blocks, then layers or slices.
    ///
    /// # Panics
    /// Fails if the mip or layer is outside the texture, the format has no single copyable aspect, or `range`
    /// lies outside the packed bytes.
    pub fn texture(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        mip: Mip,
        range: impl RangeBounds<u64>,
    ) -> Self {
        let format = texture.format();
        assert!(
            mip.level < texture.mip_level_count(),
            "mip {} of a {}-level texture",
            mip.level,
            texture.mip_level_count()
        );
        let size = texture
            .size()
            .mip_level_size(mip.level, texture.dimension())
            .physical_size(format);
        let layers = size.depth_or_array_layers;
        let (first, layer_count) = match mip.layer {
            Some(layer) => {
                assert!(
                    layer < layers,
                    "layer {layer} of {layers} at mip {}",
                    mip.level
                );
                (layer, 1)
            }
            None => (0, layers),
        };
        let block_bytes = format
            .block_copy_size(None)
            .expect("readback reads a format with one copyable aspect");
        let (block_width, block_height) = format.block_dimensions();
        let packed = size.width / block_width * block_bytes;
        let padded = packed.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let rows_per_image = size.height / block_height;
        let count = rows_per_image * layer_count;
        let staging = staging(device, u64::from(padded) * u64::from(count));
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip.level,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: first,
                },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(rows_per_image),
                },
            },
            wgpu::Extent3d {
                depth_or_array_layers: layer_count,
                ..size
            },
        );
        let rows = Rows {
            packed: packed as usize,
            padded: padded as usize,
            count: count as usize,
        };
        let total = (rows.packed * rows.count) as u64;
        let bytes = bounds(&range, total);
        Self {
            staging,
            rows,
            bytes: usize_from(bytes.start)..usize_from(bytes.end),
            lead: 0,
        }
    }

    /// Wait at most [`WAIT`] for `submission`, the one holding this copy, and return the packed bytes.
    ///
    /// # Panics
    /// Fails if the submission does not complete within [`WAIT`] or the staging buffer cannot be mapped.
    #[must_use]
    pub fn wait(self, device: &wgpu::Device, submission: wgpu::SubmissionIndex) -> Vec<u8> {
        let (send, receive) = std::sync::mpsc::channel();
        self.staging
            .map_async(wgpu::MapMode::Read, .., move |result| {
                send.send(result)
                    .expect("readback waits for its map callback");
            });
        if let Err(error) = device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(WAIT),
        }) {
            panic!("readback submission did not complete within {WAIT:?}: {error}");
        }
        let mapped = receive
            .try_recv()
            .or_else(|_| {
                // The submission is done; one more poll runs a map callback the wait left queued.
                device
                    .poll(wgpu::PollType::Poll)
                    .expect("poll after readback submission");
                receive.try_recv()
            })
            .expect("readback map callback after its submission completed");
        mapped.expect("readback staging buffer mapped");
        let view = self
            .staging
            .get_mapped_range(..)
            .expect("readback staging range");
        let Rows {
            packed,
            padded,
            count,
        } = self.rows;
        let mut out = Vec::with_capacity(self.bytes.len());
        // Walk only the rows that hold the requested bytes.
        let first = self.bytes.start / packed.max(1);
        for row in first..count {
            let row_start = row * packed;
            if row_start >= self.bytes.end {
                break;
            }
            let from = self.bytes.start.max(row_start) - row_start;
            let to = self.bytes.end.min(row_start + packed) - row_start;
            let base = self.lead + row * padded;
            out.extend_from_slice(&view[base + from..base + to]);
        }
        drop(view);
        self.staging.unmap();
        out
    }
}

/// Map a test's already-submitted staging buffer without polling or waiting.
/// Call this from the submission callback when the test needs to observe the
/// asynchronous delivery path. The buffer is unmapped before `publish` runs.
/// A cancelled or failed map delivers `None`.
pub fn map_submitted_buffer(
    staging: &wgpu::Buffer,
    publish: impl FnOnce(Option<Vec<u8>>) + wgpu::WasmNotSend + 'static,
) {
    let buffer = staging.clone();
    staging.map_async(wgpu::MapMode::Read, .., move |result| {
        let bytes = if result.is_ok() {
            let bytes = buffer
                .get_mapped_range(..)
                .ok()
                .map(|mapped| mapped.to_vec());
            buffer.unmap();
            bytes
        } else {
            None
        };
        publish(bytes);
    });
}

/// Read `range` of `buffer`, which needs `COPY_SRC`, on a submission of its own.
///
/// # Panics
/// As [`Readback::buffer`] and [`Readback::wait`].
#[must_use]
pub fn buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    range: impl RangeBounds<u64>,
) -> Vec<u8> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test readback"),
    });
    let readback = Readback::buffer(device, &mut encoder, buffer, range);
    let submission = queue.submit([encoder.finish()]);
    readback.wait(device, submission)
}

/// Read one mip, or one layer of it, of `texture`, which needs `COPY_SRC`, on a submission of its own.
/// `range` selects bytes of the packed read.
///
/// # Panics
/// As [`Readback::texture`] and [`Readback::wait`].
#[must_use]
pub fn texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    mip: Mip,
    range: impl RangeBounds<u64>,
) -> Vec<u8> {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("test readback"),
    });
    let readback = Readback::texture(device, &mut encoder, texture, mip, range);
    let submission = queue.submit([encoder.finish()]);
    readback.wait(device, submission)
}

#[cfg(test)]
mod tests;
