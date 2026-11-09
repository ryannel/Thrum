//! Bounded asynchronous readback of an offscreen target's display output.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use super::{OffscreenTarget, RenderError};

/// One bounded asynchronous sRGB-encoded SDR RGBA8 readback slot, reusable
/// after collection. Each pixel retains four bytes including opaque alpha.
pub struct PixelReadback {
    pub(super) buffer: wgpu::Buffer,
    pub(super) generation: u64,
    pub(super) extent: [u32; 2],
    pub(super) row_bytes: u32,
    pub(super) pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}
impl PixelReadback {
    pub(crate) fn cancel_unsubmitted(&mut self) {
        // The caller has dropped the unsubmitted encoder. Its deferred map
        // action never ran, so there is no GPU mapping to unmap or wait for.
        self.pending = None;
    }
    pub(super) fn check_target(&self, target: &OffscreenTarget) -> Result<(), RenderError> {
        if self.pending.is_some() {
            return Err(RenderError::Readback);
        }
        if self.generation != target.generation || self.extent != target.extent() {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }

    pub(super) fn copy_into(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
    ) -> Result<(), RenderError> {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target
                    .textures
                    .texture(&target.ids.output)
                    .ok_or(RenderError::GraphPassMismatch)?,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.row_bytes),
                    rows_per_image: Some(self.extent[1]),
                },
            },
            wgpu::Extent3d {
                width: self.extent[0],
                height: self.extent[1],
                depth_or_array_layers: 1,
            },
        );
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.buffer, wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        self.pending = Some(receive);
        Ok(())
    }
    /// Nonblocking collection, removing row padding. The caller polls the device.
    /// # Errors
    /// Refuses stale generations, failed mapping or a disconnected completion callback.
    pub fn try_collect(&mut self, generation: u64) -> Result<Option<Vec<u8>>, RenderError> {
        let mut pixels = Vec::new();
        self.try_collect_into(generation, &mut pixels)
            .map(|ready| ready.then_some(pixels))
    }

    pub(crate) fn try_collect_into(
        &mut self,
        generation: u64,
        pixels: &mut Vec<u8>,
    ) -> Result<bool, RenderError> {
        if self.generation != generation {
            return Err(RenderError::StaleTarget);
        }
        let Some(pending) = &self.pending else {
            return Ok(false);
        };
        match pending.try_recv() {
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) | Ok(Err(_)) => {
                self.pending = None;
                return Err(RenderError::Readback);
            }
            Ok(Ok(())) => {}
        }
        self.pending = None;
        let mapped = self
            .buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|_| RenderError::Readback)?;
        pixels.clear();
        pixels.reserve(self.extent[0] as usize * self.extent[1] as usize * 4);
        for row in mapped.chunks_exact(self.row_bytes as usize) {
            pixels.extend_from_slice(&row[..self.extent[0] as usize * 4]);
        }
        drop(mapped);
        self.buffer.unmap();
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
