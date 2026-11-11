//! Bounded asynchronous readback of an offscreen target's display output.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use super::{OffscreenTarget, RenderError};
mod copy_timing;

/// Prepared engine texture copied by the submitted-image tap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CaptureSource {
    /// Final tone-mapped, sRGB-encoded RGBA8 engine output.
    #[default]
    Sdr,
    /// Pre-tone scene-linear RGBA16Float, including the prepared star composite.
    SceneLinear,
}

impl CaptureSource {
    /// Stored bytes per pixel, including alpha.
    #[must_use]
    pub const fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::Sdr => 4,
            Self::SceneLinear => 8,
        }
    }
}

/// One bounded asynchronous raw pixel readback slot, reusable after collection.
/// The prepared source determines the stored format and bytes per pixel.
pub struct PixelReadback {
    pub(super) source: CaptureSource,
    pub(super) exposure_words: Option<[u32; 8]>,
    pub(super) has_exposure: bool,
    pub(super) buffer: wgpu::Buffer,
    pub(super) generation: u64,
    pub(super) extent: [u32; 2],
    pub(super) row_bytes: u32,
    pub(super) copy_timing: Option<copy_timing::CopyTiming>,
    pub(super) resolving_copy_timing: bool,
    pub(super) pending: Option<Receiver<Result<(), wgpu::BufferAsyncError>>>,
}
impl PixelReadback {
    pub(crate) fn exposure_gpu_words(&self) -> Option<[u32; 8]> {
        self.exposure_words
    }

    pub(crate) fn cancel_unsubmitted(&mut self) {
        // The caller has dropped the unsubmitted encoder. Its deferred map
        // action never ran, so there is no GPU mapping to unmap or wait for.
        self.pending = None;
        if let Some(timer) = &mut self.copy_timing {
            let _ = timer.timings.discard_unsubmitted(self.generation);
        }
    }
    pub(super) fn check_target(&self, target: &OffscreenTarget) -> Result<(), RenderError> {
        if self.pending.is_some() {
            return Err(RenderError::Readback);
        }
        if self.generation != target.generation
            || self.extent != target.extent()
            || self.source != target.capture_source
        {
            return Err(RenderError::StaleTarget);
        }
        Ok(())
    }

    pub(super) fn copy_into(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        exposure: Option<&wgpu::Buffer>,
    ) -> Result<(), RenderError> {
        if let Some(timer) = &mut self.copy_timing {
            timer.marker(self.generation, encoder, false)?;
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target
                    .textures
                    .texture(match self.source {
                        CaptureSource::Sdr => &target.ids.output,
                        CaptureSource::SceneLinear => &target.ids.scene,
                    })
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
        let pixel_bytes = u64::from(self.row_bytes) * u64::from(self.extent[1]);
        self.has_exposure = exposure.is_some();
        if let Some(exposure) = exposure {
            encoder.copy_buffer_to_buffer(
                exposure,
                0,
                &self.buffer,
                pixel_bytes,
                super::exposure::SAMPLE_BYTES,
            );
        } else {
            encoder.clear_buffer(
                &self.buffer,
                pixel_bytes,
                Some(super::exposure::SAMPLE_BYTES),
            );
        }
        if let Some(timer) = &mut self.copy_timing {
            timer.marker(self.generation, encoder, true)?;
        }
        let (send, receive) = mpsc::channel();
        encoder.map_buffer_on_submit(&self.buffer, wgpu::MapMode::Read, .., move |result| {
            let _ = send.send(result);
        });
        self.pending = Some(receive);
        Ok(())
    }
    pub(crate) fn enable_copy_timing(
        &mut self,
        capabilities: &crate::capabilities::DeviceCapabilities,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderError> {
        self.copy_timing = copy_timing::CopyTiming::new(capabilities, queue, &self.buffer)?;
        Ok(())
    }
    pub(crate) fn finish_copy_timing(
        &mut self,
        encoder: &wgpu::CommandEncoder,
    ) -> Result<(), RenderError> {
        if let Some(timer) = &mut self.copy_timing {
            timer.timings.finish_render(self.generation, encoder)?;
        }
        self.resolving_copy_timing = false;
        Ok(())
    }
    pub(crate) fn copy_timing_ready_to_resolve(&self) -> bool {
        !self.resolving_copy_timing
            && self
                .copy_timing
                .as_ref()
                .is_some_and(|timer| timer.timings.ready_to_resolve())
    }
    pub(crate) fn resolve_copy_timing(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), RenderError> {
        if self.copy_timing_ready_to_resolve() {
            self.resolving_copy_timing = self
                .copy_timing
                .as_mut()
                .expect("ready timer")
                .timings
                .resolve_if_ready(self.generation, encoder)?;
        }
        Ok(())
    }
    pub(crate) fn copy_duration(&mut self) -> Result<std::task::Poll<Option<f64>>, RenderError> {
        let Some(timer) = &mut self.copy_timing else {
            return Ok(std::task::Poll::Ready(None));
        };
        Ok(timer.collect(self.generation)?.map(Some))
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
        pixels.reserve(
            self.extent[0] as usize
                * self.extent[1] as usize
                * self.source.bytes_per_pixel() as usize,
        );
        let pixel_bytes = self.row_bytes as usize * self.extent[1] as usize;
        self.exposure_words = self.has_exposure.then(|| {
            std::array::from_fn(|index| {
                let offset = pixel_bytes + index * 4;
                u32::from_le_bytes(
                    mapped[offset..offset + 4]
                        .try_into()
                        .expect("complete exposure word"),
                )
            })
        });
        for row in mapped[..pixel_bytes].chunks_exact(self.row_bytes as usize) {
            pixels.extend_from_slice(
                &row[..self.extent[0] as usize * self.source.bytes_per_pixel() as usize],
            );
        }
        drop(mapped);
        self.buffer.unmap();
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
