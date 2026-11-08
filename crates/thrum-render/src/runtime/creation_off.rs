//! Default build: the proof counters' call sites compile to direct wgpu calls.
//! Enable the `creation-counters` feature for the diagnostic in `creation.rs`.

/// The pipeline creation authority of one device generation. In default
/// builds it is the device alone, and [`render`](Self::render) and
/// [`compute`](Self::compute) are the direct wgpu calls.
#[derive(Clone, Debug)]
pub struct DeviceSite(wgpu::Device);

impl DeviceSite {
    #[inline]
    pub(crate) fn mint(device: &wgpu::Device, _generation: u64) -> Self {
        Self(device.clone())
    }

    /// A site for a raw device. Identical to a minted one in default builds.
    #[inline]
    #[must_use]
    pub fn untracked(device: wgpu::Device) -> Self {
        Self(device)
    }

    /// `create_render_pipeline`.
    #[inline]
    pub fn render(&self, desc: &wgpu::RenderPipelineDescriptor<'_>) -> wgpu::RenderPipeline {
        self.0.create_render_pipeline(desc)
    }

    /// `create_compute_pipeline`.
    #[inline]
    pub fn compute(&self, desc: &wgpu::ComputePipelineDescriptor<'_>) -> wgpu::ComputePipeline {
        self.0.create_compute_pipeline(desc)
    }

    /// Declare a transition on the calling thread. Nothing in default builds.
    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "mirrors the tracked site's transition call"
    )]
    pub fn transition(&self, _label: &'static str) -> CreationRegion {
        CreationRegion
    }

    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "mirrors the tracked site's ledger handle"
    )]
    pub(crate) fn ledger(&self) -> Ledger {
        Ledger
    }
}

impl std::ops::Deref for DeviceSite {
    type Target = wgpu::Device;

    #[inline]
    fn deref(&self) -> &wgpu::Device {
        &self.0
    }
}

/// Phase of an observation window.
#[derive(Clone, Copy)]
pub(crate) enum CreationPhase {
    Build,
    Transition,
}

/// No ledger: holds no device, lock or report.
#[derive(Clone)]
pub(crate) struct Ledger;

impl Drop for Ledger {
    fn drop(&mut self) {}
}

/// No region.
#[must_use = "the region closes when dropped"]
pub struct CreationRegion;

impl Drop for CreationRegion {
    fn drop(&mut self) {}
}

/// No detached handle.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct Detached;

impl Ledger {
    #[inline]
    #[expect(clippy::unused_self, reason = "mirrors the tracked region call")]
    pub(crate) fn region(&self, _phase: CreationPhase, _label: &'static str) -> CreationRegion {
        CreationRegion
    }

    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "mirrors the tracked ledger's readiness call"
    )]
    pub(crate) fn ready(&self) {}

    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "mirrors the tracked ledger's per-frame call"
    )]
    pub(crate) fn frame(&self) {}

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    #[expect(
        clippy::unused_self,
        reason = "mirrors the tracked ledger's detach call"
    )]
    pub(crate) fn detach(&self) -> Detached {
        Detached
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Detached {
    #[inline]
    #[expect(clippy::unused_self, reason = "mirrors the tracked region call")]
    pub(crate) fn region(&self, _phase: CreationPhase, _label: &'static str) -> CreationRegion {
        CreationRegion
    }
}
