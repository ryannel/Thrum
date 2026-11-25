//! Whole raw Metal recording only. No surface, queue or completion authority.
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_metal::{MTLCommandBuffer, MTLCommandBufferStatus, MTLTexture};

pub(crate) type CommandBuffer = ProtocolObject<dyn MTLCommandBuffer>;

#[derive(Clone)]
pub(crate) struct Texture(Retained<ProtocolObject<dyn MTLTexture>>);

// SAFETY: constructed only from wgpu-hal's Metal Texture (itself Send + Sync).
// This wrapper exposes no CPU contents or mutable descriptor. Native recording
// remains on the exclusive coordinator; other threads only retain/drop it at
// completion. The wgpu generation owns initialization, queue ordering and lifetime.
#[expect(
    unsafe_code,
    reason = "A retained view of wgpu-hal's Send/Sync GPU-owned Metal texture, with coordinator-only recording"
)]
// SAFETY: only the wgpu-hal-owned, immutable GPU resource retain crosses threads.
unsafe impl Send for Texture {}
// SAFETY: the same GPU-only access invariant applies to shared references; all
// native writes are ordered commands, never unsynchronized CPU texture mutation.
#[expect(
    unsafe_code,
    reason = "Completion may release immutable native texture retains on a wgpu callback thread"
)]
// SAFETY: all texture mutation is coordinator-ordered GPU work; callbacks only drop.
unsafe impl Sync for Texture {}

impl std::ops::Deref for Texture {
    type Target = ProtocolObject<dyn MTLTexture>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
/// A read-only retain of the dedicated segment, inspected after queue completion.
pub(crate) struct Receipt(Retained<ProtocolObject<dyn MTLCommandBuffer>>);

#[expect(
    unsafe_code,
    reason = "The coordinator transfers this read-only retain to its queue-completion callback; no recording or submission API is exposed"
)]
// SAFETY: Metal command buffers permit completion observation from another
// thread. This wrapper only reads status/times after wgpu queue completion.
unsafe impl Send for Receipt {}

impl Receipt {
    pub(crate) fn completed(&self, timing_requested: bool) -> Result<Option<[f64; 2]>, ()> {
        match self.0.status() {
            MTLCommandBufferStatus::Error => return Err(()),
            MTLCommandBufferStatus::Completed if timing_requested => {}
            _ => return Ok(None),
        }
        let times = [self.0.GPUStartTime(), self.0.GPUEndTime()];
        Ok(
            (times[0].is_finite() && times[1].is_finite() && times[0] > 0.0 && times[1] > times[0])
                .then_some(times),
        )
    }
}

#[expect(
    unsafe_code,
    reason = "Preparation-only retained Metal handles; the non-reentrant HAL guard is released before any other wgpu operation"
)]
pub(crate) fn texture(texture: &wgpu::Texture) -> Option<Texture> {
    // SAFETY: the generation retains the wgpu owner through completion and never
    // destroys or aliases this allocation. Only a balanced native retain escapes.
    let guard = unsafe { texture.as_hal::<wgpu::hal::api::Metal>() }?;
    Some(Texture(guard.raw_handle().into()))
}

#[expect(
    unsafe_code,
    reason = "The segment hands a dedicated wgpu-owned Metal buffer to the provider for recording only"
)]
pub(super) fn record(
    encoder: &mut wgpu::CommandEncoder,
    record: impl FnOnce(&CommandBuffer),
) -> Receipt {
    // SAFETY: this encoder has no wgpu recording. The provider only records
    // on it and ends its own encoders; wgpu alone finishes and submits it.
    unsafe {
        encoder.as_hal_mut::<wgpu::hal::api::Metal, _, _>(|raw| {
            let buffer = raw
                .expect("registry admitted Metal")
                .raw_command_buffer()
                .expect("wgpu opened raw buffer");
            record(buffer);
            Receipt(buffer.into())
        })
    }
}
