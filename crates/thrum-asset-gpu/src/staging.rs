//! Staged uploads through a bounded ring of reusable staging chunks, filled off the render thread
//! and copied on the GPU under a per-frame byte budget.
//!
//! `Queue::write_buffer`/`write_texture` copy into wgpu's staging memory on the calling thread
//! and flush the whole lot at the next `Queue::submit`, so a 100 MiB volume written that way lands
//! in one frame's submission. Here an upload is planned as copies of at most [`COPY_UNIT`] packed
//! in order into chunks of one frame's budget. Each device generation owns one [`StagingRing`] of
//! at most [`STAGING_CHUNKS`] chunk buffers, so staging never holds more than that many chunks
//! however large the publication.
//!
//! A [`Filler`] writes product bytes once into a free chunk (rows re-padded to the copy
//! alignment) and unmaps it: the loader thread fills ahead and waits for free chunks, and a remote
//! feed with no thread (the browser) fills whatever is free each frame on the thread that renders,
//! without waiting. The render thread records at most a budget of `copy_buffer_to_*` commands from
//! filled chunks into each frame's own encoder and never waits; on native it never fills either. A
//! frame whose next chunk is not filled yet records nothing. A chunk that cannot be filled fails
//! the upload, which the render thread then drops. Once a chunk's last copy is committed (its encoder was submitted) the chunk is mapped
//! again, and the map completes, returning it to the ring, when the GPU has finished reading it.
//! A set becomes drawable in the frame that records its last copy, because those copies precede
//! the forward pass in that command buffer.

use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Condvar;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use thrum_asset::Ktx2;

use crate::CookedStats;

/// Largest single copy the planner emits; budgets are spent in units no larger than this.
pub const COPY_UNIT: u64 = 1 << 20;

/// Staging chunks each device generation may hold. A chunk holds one frame's upload budget (at
/// least [`COPY_UNIT`]), so staging peaks at this many budgets: enough for one chunk being filled
/// while others wait for the GPU to finish reading frames in flight.
pub const STAGING_CHUNKS: u64 = 4;

/// Bytes a staged buffer piece reads from.
pub(crate) trait Bytes: Send + Sync {
    fn bytes(&self) -> &[u8];
}

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

/// How one copy's bytes are written into its chunk.
enum Fill {
    /// `len` bytes of a buffer source from `start`, zero-padded to the copy size.
    Packed {
        source: usize,
        start: usize,
        len: usize,
    },
    /// Block rows of a texture level, re-strided from `row_bytes` to `padded`.
    Rows {
        source: usize,
        level: usize,
        z: u32,
        depth: u32,
        row: u32,
        rows: u32,
        rows_per_image: u32,
        row_bytes: u32,
        padded: u32,
    },
}

/// One recorded copy, from its chunk at `offset`.
struct Copy {
    chunk: usize,
    offset: u64,
    size: u64,
    fill: Fill,
    destination: Destination,
}

enum Source {
    Bytes(Arc<dyn Bytes>),
    Texture(Arc<Ktx2>),
}

/// A planned upload, shared by its filler and its recorder. Filled chunks arrive in order.
struct Stream {
    sources: Vec<Source>,
    copies: Vec<Copy>,
    /// Per chunk, the index one past its last copy.
    chunk_ends: Vec<usize>,
    filled: Mutex<VecDeque<wgpu::Buffer>>,
    /// A chunk could not be filled; the upload will never complete.
    failed: AtomicBool,
    ring: StagingRing,
}

impl Drop for Stream {
    fn drop(&mut self) {
        // A superseded upload's filled chunks were never recorded; they go straight back.
        if let Ok(filled) = self.filled.get_mut() {
            for chunk in filled.drain(..) {
                self.ring.release(&chunk);
            }
        }
    }
}

impl Stream {
    fn copies_of(&self, chunk: usize) -> std::ops::Range<usize> {
        let start = chunk
            .checked_sub(1)
            .map_or(0, |before| self.chunk_ends[before]);
        start..self.chunk_ends[chunk]
    }

    /// Write `chunk`'s copies into the mapped `buffer`, unmap it and queue it for recording.
    fn fill(&self, chunk: usize, buffer: wgpu::Buffer) -> Result<(), String> {
        let written = self.write_chunk(chunk, &buffer);
        if let Err(error) = written {
            return self.abandon(&buffer, format!("staging map: {error}"));
        }
        buffer.unmap();
        match self.filled.lock() {
            Ok(mut filled) => {
                filled.push_back(buffer);
                Ok(())
            }
            Err(_) => self.abandon(&buffer, "staging queue poisoned".to_owned()),
        }
    }

    /// The mapped-write operation. The probe refuses this operation, then `fill` handles
    /// its error through the same production branch as a failed mapped-range acquisition.
    fn write_chunk(&self, chunk: usize, buffer: &wgpu::Buffer) -> Result<(), String> {
        #[cfg(feature = "fault-probe")]
        if self
            .ring
            .0
            .fault
            .as_ref()
            .is_some_and(crate::FaultProbe::take)
        {
            return Err("injected fill failure".to_owned());
        }
        let mut mapped = buffer
            .get_mapped_range_mut(..)
            .map_err(|error| error.to_string())?;
        for copy in &self.copies[self.copies_of(chunk)] {
            self.write(copy, &mut mapped);
        }
        Ok(())
    }

    /// Fail the upload and take `buffer` out of the ring's account, so the slot can be recreated.
    fn abandon(&self, buffer: &wgpu::Buffer, reason: String) -> Result<(), String> {
        self.failed.store(true, Ordering::Relaxed);
        self.ring.forget(buffer);
        Err(reason)
    }

    /// Write whole ranges into mapped (possibly write-combining) memory; never read it.
    fn write(&self, copy: &Copy, out: &mut wgpu::BufferViewMut) {
        let at = usize::try_from(copy.offset).unwrap_or(usize::MAX);
        match copy.fill {
            Fill::Packed { source, start, len } => {
                let Source::Bytes(bytes) = &self.sources[source] else {
                    return;
                };
                let size = usize::try_from(copy.size).unwrap_or(len);
                out.slice(at..at + len)
                    .copy_from_slice(&bytes.bytes()[start..start + len]);
                if size > len {
                    out.slice(at + len..at + size).fill(0);
                }
            }
            Fill::Rows {
                source,
                level,
                z,
                depth,
                row,
                rows,
                rows_per_image,
                row_bytes,
                padded,
            } => {
                let Source::Texture(texture) = &self.sources[source] else {
                    return;
                };
                let data = texture.level_data(&texture.levels()[level]);
                let (row_bytes, padded) = (row_bytes as usize, padded as usize);
                for slice in 0..depth {
                    for line in 0..rows {
                        let from = ((z + slice) * rows_per_image + row + line) as usize * row_bytes;
                        let to = at + (slice * rows_per_image + line) as usize * padded;
                        out.slice(to..to + row_bytes)
                            .copy_from_slice(&data[from..from + row_bytes]);
                    }
                }
            }
        }
    }
}

/// The bounded set of staging chunks one device generation uploads through.
#[derive(Clone)]
pub(crate) struct StagingRing(Arc<Ring>);

struct Ring {
    #[cfg(feature = "fault-probe")]
    fault: Option<crate::FaultProbe>,
    device: wgpu::Device,
    chunk_size: u64,
    limit: usize,
    state: Mutex<RingState>,
    #[cfg(not(target_arch = "wasm32"))]
    returned: Condvar,
    stats: Arc<Mutex<CookedStats>>,
}

#[derive(Default)]
struct RingState {
    /// Mapped chunks ready to fill.
    free: Vec<wgpu::Buffer>,
    /// Chunks that exist (free, filling, filled, recorded or returning).
    created: usize,
}

impl StagingRing {
    /// A ring of at most [`STAGING_CHUNKS`] chunks, each one `budget` (at least [`COPY_UNIT`],
    /// at most the device's buffer limit). Chunks are created on first need and then reused.
    pub(crate) fn new(
        device: &wgpu::Device,
        budget: u64,
        stats: Arc<Mutex<CookedStats>>,
        #[cfg(feature = "fault-probe")] fault: Option<crate::FaultProbe>,
    ) -> Self {
        let chunk_size = budget
            .max(COPY_UNIT)
            .next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
            .min(device.limits().max_buffer_size);
        Self(Arc::new(Ring {
            #[cfg(feature = "fault-probe")]
            fault,
            device: device.clone(),
            chunk_size,
            limit: usize::try_from(STAGING_CHUNKS).unwrap_or(usize::MAX),
            state: Mutex::new(RingState::default()),
            #[cfg(not(target_arch = "wasm32"))]
            returned: Condvar::new(),
            stats,
        }))
    }

    /// A free mapped chunk, creating one while under the limit; `None` when all are in use.
    fn try_acquire(&self) -> Option<wgpu::Buffer> {
        let ring = &*self.0;
        {
            let mut state = ring.state.lock().ok()?;
            if let Some(chunk) = state.free.pop() {
                return Some(chunk);
            }
            if state.created >= ring.limit {
                return None;
            }
            state.created += 1;
            let bytes = state.created as u64 * ring.chunk_size;
            if let Ok(mut stats) = ring.stats.lock() {
                stats.staging_bytes = bytes;
                stats.staging_peak_bytes = stats.staging_peak_bytes.max(bytes);
            }
        }
        Some(ring.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("thrum-asset-gpu staging chunk"),
            size: ring.chunk_size,
            usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: true,
        }))
    }

    /// Wait (off the render thread) for a free chunk until `give_up` says the work is moot.
    /// Map completions are driven here too, so a host that is not polling cannot starve it.
    #[cfg(not(target_arch = "wasm32"))]
    fn acquire_waiting(&self, give_up: impl Fn() -> bool) -> Option<wgpu::Buffer> {
        loop {
            if let Some(chunk) = self.try_acquire() {
                return Some(chunk);
            }
            if give_up() {
                return None;
            }
            self.0.device.poll(wgpu::PollType::Poll).ok();
            let state = self.0.state.lock().ok()?;
            if state.free.is_empty() {
                drop(
                    self.0
                        .returned
                        .wait_timeout(state, Duration::from_millis(2))
                        .ok()?,
                );
            }
        }
    }

    /// Destroy a chunk that will not come back and free its slot.
    fn forget(&self, chunk: &wgpu::Buffer) {
        chunk.destroy();
        if let Ok(mut state) = self.0.state.lock() {
            state.created = state.created.saturating_sub(1);
            if let Ok(mut stats) = self.0.stats.lock() {
                stats.staging_bytes = state.created as u64 * self.0.chunk_size;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.0.returned.notify_one();
    }

    /// Return `chunk` once the GPU has finished every submitted copy from it.
    fn release(&self, chunk: &wgpu::Buffer) {
        let ring = Arc::clone(&self.0);
        let returned = chunk.clone();
        chunk.map_async(wgpu::MapMode::Write, .., move |result| {
            let Ok(mut state) = ring.state.lock() else {
                return;
            };
            if result.is_ok() {
                state.free.push(returned);
                #[cfg(not(target_arch = "wasm32"))]
                ring.returned.notify_one();
            } else {
                // A lost device: forget the chunk so the account stays true.
                state.created = state.created.saturating_sub(1);
                if let Ok(mut stats) = ring.stats.lock() {
                    stats.staging_bytes = state.created as u64 * ring.chunk_size;
                }
            }
        });
    }
}

/// Copies recorded in one encoder, not yet committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Recorded {
    /// Index one past the last recorded copy.
    pub(crate) end: usize,
    /// Bytes recorded.
    pub(crate) bytes: u64,
}

/// The render thread's side of an upload. Recording is tentative: [`Self::encode`] records from
/// the last committed copy without consuming anything, and only [`Self::commit`] (called once
/// the encoder holding those copies was submitted) advances and returns finished chunks. A
/// discarded frame's copies are simply recorded again.
pub(crate) struct StagedUpload {
    stream: Arc<Stream>,
    /// Filled chunks from `first_held`, in order.
    held: VecDeque<wgpu::Buffer>,
    first_held: usize,
    next: usize,
    total: u64,
}

impl Drop for StagedUpload {
    fn drop(&mut self) {
        for chunk in self.held.drain(..) {
            self.stream.ring.release(&chunk);
        }
    }
}

impl StagedUpload {
    /// Bytes this upload moves (including row padding).
    pub(crate) fn total(&self) -> u64 {
        self.total
    }

    /// Whether a chunk could not be filled, so the upload can never complete.
    pub(crate) fn failed(&self) -> bool {
        self.stream.failed.load(Ordering::Relaxed)
    }

    /// Whether `recorded` reaches the last copy.
    pub(crate) fn completes(&self, recorded: Recorded) -> bool {
        recorded.end == self.stream.copies.len()
    }

    /// Record copies from the last committed one until `budget` bytes have been spent (at least
    /// one copy when its chunk is filled) or the next copy's chunk is not filled yet.
    pub(crate) fn encode(&mut self, encoder: &mut wgpu::CommandEncoder, budget: u64) -> Recorded {
        if let Ok(mut filled) = self.stream.filled.try_lock() {
            self.held.extend(filled.drain(..));
        }
        let mut recorded = Recorded {
            end: self.next,
            bytes: 0,
        };
        for copy in &self.stream.copies[self.next..] {
            let Some(staging) = copy
                .chunk
                .checked_sub(self.first_held)
                .and_then(|index| self.held.get(index))
            else {
                break;
            };
            if recorded.bytes > 0 && recorded.bytes + copy.size > budget {
                break;
            }
            match &copy.destination {
                Destination::Buffer { buffer, offset } => {
                    encoder.copy_buffer_to_buffer(staging, copy.offset, buffer, *offset, copy.size);
                }
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
                            offset: copy.offset,
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

    /// The encoder holding `recorded` was submitted: advance, and return every chunk whose copies
    /// are all submitted (it rejoins the ring when the GPU has read it).
    pub(crate) fn commit(&mut self, recorded: Recorded) {
        if recorded.end > self.next && recorded.end <= self.stream.copies.len() {
            self.next = recorded.end;
        }
        while self
            .stream
            .chunk_ends
            .get(self.first_held)
            .is_some_and(|&end| end <= self.next)
            && let Some(chunk) = self.held.pop_front()
        {
            self.stream.ring.release(&chunk);
            self.first_held += 1;
        }
    }
}

/// How far a [`Filler`] got.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Progress {
    /// Every chunk is filled.
    Done,
    /// The upload was dropped (superseded or retired).
    Stopped,
    /// No chunk is free; try again later.
    Waiting,
    /// A chunk could not be filled; the upload is marked failed.
    Failed(String),
}

/// The filling side of an upload. It holds the upload weakly, so a superseded upload stops
/// being filled.
pub(crate) struct Filler {
    stream: Weak<Stream>,
    ring: StagingRing,
    chunks: usize,
    next: usize,
}

impl Filler {
    /// Fill every chunk that is free now, without waiting.
    pub(crate) fn fill_available(&mut self) -> Progress {
        while self.next < self.chunks {
            let Some(stream) = self.stream.upgrade() else {
                return Progress::Stopped;
            };
            let Some(chunk) = self.ring.try_acquire() else {
                return Progress::Waiting;
            };
            if let Err(reason) = stream.fill(self.next, chunk) {
                return Progress::Failed(reason);
            }
            self.next += 1;
        }
        Progress::Done
    }

    /// Fill chunks, waiting for free ones, until done, dropped, `stop`, or `deadline` passes
    /// ([`Progress::Waiting`]), so the caller can look for a newer publication in between.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn fill_until(&mut self, stop: &AtomicBool, deadline: Instant) -> Progress {
        while self.next < self.chunks {
            let dropped = || stop.load(Ordering::Relaxed) || self.stream.strong_count() == 0;
            let Some(chunk) = self
                .ring
                .acquire_waiting(|| dropped() || Instant::now() >= deadline)
            else {
                return if dropped() {
                    Progress::Stopped
                } else {
                    Progress::Waiting
                };
            };
            let Some(stream) = self.stream.upgrade() else {
                self.ring.release_unused(chunk);
                return Progress::Stopped;
            };
            if let Err(reason) = stream.fill(self.next, chunk) {
                return Progress::Failed(reason);
            }
            self.next += 1;
        }
        Progress::Done
    }
}

impl StagingRing {
    /// Put back a mapped chunk that was acquired but never filled.
    #[cfg(not(target_arch = "wasm32"))]
    fn release_unused(&self, chunk: wgpu::Buffer) {
        if let Ok(mut state) = self.0.state.lock() {
            state.free.push(chunk);
        }
    }
}

/// One piece of source data and where it goes.
enum Piece {
    Buffer {
        bytes: Arc<dyn Bytes>,
        buffer: wgpu::Buffer,
    },
    Texture {
        texture: Arc<Ktx2>,
        destination: wgpu::Texture,
    },
}

/// Collects pieces on the loader thread, then plans their copies into chunks.
#[derive(Default)]
pub(crate) struct StagingPlan {
    pieces: Vec<Piece>,
}

/// Packs copies in order into chunks of `size` bytes, each copy 256-byte aligned.
struct Packer {
    size: u64,
    chunk: usize,
    offset: u64,
    used: bool,
}

impl Packer {
    fn place(&mut self, bytes: u64) -> Result<(usize, u64), String> {
        if bytes > self.size {
            return Err(format!(
                "a {bytes}-byte staged copy exceeds the {}-byte staging chunk",
                self.size
            ));
        }
        let mut offset = self
            .offset
            .next_multiple_of(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
        if self.used && offset + bytes > self.size {
            self.chunk += 1;
            offset = 0;
        }
        self.used = true;
        self.offset = offset + bytes;
        Ok((self.chunk, offset))
    }
}

impl StagingPlan {
    pub(crate) fn buffer(&mut self, bytes: Arc<dyn Bytes>, buffer: wgpu::Buffer) {
        self.pieces.push(Piece::Buffer { bytes, buffer });
    }

    pub(crate) fn texture(&mut self, texture: Arc<Ktx2>, destination: wgpu::Texture) {
        self.pieces.push(Piece::Texture {
            texture,
            destination,
        });
    }

    /// Plan every copy into `ring`'s chunks. Nothing is written yet: the returned [`Filler`]
    /// writes chunks as they become free, and the [`StagedUpload`] records them.
    ///
    /// # Errors
    /// A single copy is larger than a staging chunk.
    pub(crate) fn finish(self, ring: &StagingRing) -> Result<(StagedUpload, Filler), String> {
        let mut packer = Packer {
            size: ring.0.chunk_size,
            chunk: 0,
            offset: 0,
            used: false,
        };
        let mut sources = Vec::with_capacity(self.pieces.len());
        let mut copies = Vec::new();
        for piece in self.pieces {
            let source = sources.len();
            match piece {
                Piece::Buffer { bytes, buffer } => {
                    let len = bytes.bytes().len();
                    let size = (len as u64).next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
                    let mut done = 0;
                    while done < size {
                        let part = (size - done).min(COPY_UNIT);
                        let (chunk, offset) = packer.place(part)?;
                        let start = usize::try_from(done).map_err(|error| error.to_string())?;
                        copies.push(Copy {
                            chunk,
                            offset,
                            size: part,
                            fill: Fill::Packed {
                                source,
                                start,
                                len: (len - start).min(usize::try_from(part).unwrap_or(len)),
                            },
                            destination: Destination::Buffer {
                                buffer: buffer.clone(),
                                offset: done,
                            },
                        });
                        done += part;
                    }
                    sources.push(Source::Bytes(bytes));
                }
                Piece::Texture {
                    texture,
                    destination,
                } => {
                    plan_texture(&texture, source, &destination, &mut packer, &mut copies)?;
                    sources.push(Source::Texture(texture));
                }
            }
        }
        let mut chunk_ends: Vec<usize> = Vec::new();
        for (index, copy) in copies.iter().enumerate() {
            if copy.chunk == chunk_ends.len() {
                chunk_ends.push(index + 1);
            } else if let Some(end) = chunk_ends.last_mut() {
                *end = index + 1;
            }
        }
        let total = copies.iter().map(|copy| copy.size).sum();
        let chunks = chunk_ends.len();
        let stream = Arc::new(Stream {
            sources,
            copies,
            chunk_ends,
            filled: Mutex::new(VecDeque::new()),
            failed: AtomicBool::new(false),
            ring: ring.clone(),
        });
        let filler = Filler {
            stream: Arc::downgrade(&stream),
            ring: ring.clone(),
            chunks,
            next: 0,
        };
        Ok((
            StagedUpload {
                stream,
                held: VecDeque::new(),
                first_held: 0,
                next: 0,
                total,
            },
            filler,
        ))
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
fn plan_texture(
    texture: &Ktx2,
    source: usize,
    destination: &wgpu::Texture,
    packer: &mut Packer,
    copies: &mut Vec<Copy>,
) -> Result<(), String> {
    let format = texture.format();
    let (block_width, block_height) = format.block_dimensions();
    for (mip, level) in texture.levels().iter().enumerate() {
        let row = level.bytes_per_row(format);
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let rows = level.rows_per_image(format);
        let physical_width = level.width.next_multiple_of(block_width);
        let depth = if texture.is_3d() { level.depth } else { 1 };
        for region in regions(depth, rows, u64::from(padded)) {
            let (chunk, offset) = packer.place(region.bytes)?;
            copies.push(Copy {
                chunk,
                offset,
                size: region.bytes,
                fill: Fill::Rows {
                    source,
                    level: mip,
                    z: region.z,
                    depth: region.depth,
                    row: region.row,
                    rows: region.rows,
                    rows_per_image: rows,
                    row_bytes: row,
                    padded,
                },
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
    }
    Ok(())
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

    #[test]
    fn copies_pack_in_order_into_aligned_chunks_and_never_span_two() {
        let mut packer = Packer {
            size: 4 * COPY_UNIT,
            chunk: 0,
            offset: 0,
            used: false,
        };
        let mut placed = Vec::new();
        for bytes in [
            COPY_UNIT,
            100,
            COPY_UNIT,
            COPY_UNIT,
            COPY_UNIT,
            300,
            4 * COPY_UNIT,
        ] {
            placed.push((packer.place(bytes).expect("fits a chunk"), bytes));
        }
        for &((chunk, offset), bytes) in &placed {
            assert_eq!(offset % 256, 0);
            assert!(offset + bytes <= 4 * COPY_UNIT, "chunk {chunk} at {offset}");
        }
        let chunks: Vec<usize> = placed.iter().map(|((chunk, _), _)| *chunk).collect();
        assert_eq!(chunks, [0, 0, 0, 0, 1, 1, 2]);
        assert!(packer.place(4 * COPY_UNIT + 1).is_err());
    }
}
