//! Immutable facts captured once by the platform for one adapter generation.
//! Render preparation consumes this record; pass recording never probes a device.

use crate::runtime::DeviceSite;

/// Enabled GPU capabilities and identity for one device generation.
#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    /// The generation's pipeline creation authority, minted once here and
    /// shared by every clone of this record.
    site: DeviceSite,
    generation: u64,
    adapter: wgpu::AdapterInfo,
    features: wgpu::Features,
    limits: wgpu::Limits,
}

impl DeviceCapabilities {
    /// Optional timestamp feature to request while the platform creates its
    /// device. The enabled result is captured in this record afterward.
    #[must_use]
    pub fn requested_timing_features(adapter: &wgpu::Adapter) -> wgpu::Features {
        adapter.features() & wgpu::Features::TIMESTAMP_QUERY
    }
    /// Compressed-texture families the adapter offers, so cooked content can use its native
    /// profile (ASTC on Apple GPUs, BC on desktop) instead of the uncompressed portable one.
    #[must_use]
    pub fn requested_content_features(adapter: &wgpu::Adapter) -> wgpu::Features {
        adapter.features()
            & (wgpu::Features::TEXTURE_COMPRESSION_ASTC
                | wgpu::Features::TEXTURE_COMPRESSION_BC
                | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D)
    }
    /// Capture the adapter and enabled device at platform creation. This mints
    /// the generation's [`DeviceSite`]; capture each generation once.
    #[must_use]
    pub fn capture(adapter: &wgpu::Adapter, device: &wgpu::Device, generation: u64) -> Self {
        let site = DeviceSite::mint(device, generation);
        Self::from_info(site, generation, adapter.get_info())
    }

    /// Capture from a device when the adapter handle is no longer retained.
    /// The live platform should pass its adapter directly to `capture`. The
    /// record's site is [untracked](DeviceSite::untracked): re-capturing a raw
    /// handle never reaches, or forges, a generation's ledger.
    #[must_use]
    pub fn capture_device(device: &wgpu::Device, generation: u64) -> Self {
        let site = DeviceSite::untracked(device.clone());
        Self::from_info(site, generation, device.adapter_info())
    }

    fn from_info(site: DeviceSite, generation: u64, adapter: wgpu::AdapterInfo) -> Self {
        let device: &wgpu::Device = &site;
        Self {
            features: device.features(),
            limits: device.limits(),
            site,
            generation,
            adapter,
        }
    }

    /// Device generation used by pipelines and submissions.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Adapter identity and driver facts for recipe selection and cache keys.
    #[must_use]
    pub fn adapter(&self) -> &wgpu::AdapterInfo {
        &self.adapter
    }

    /// Features enabled on this device, rather than merely offered by the adapter.
    #[must_use]
    pub fn features(&self) -> wgpu::Features {
        self.features
    }

    /// Limits enabled on this device.
    #[must_use]
    pub fn limits(&self) -> &wgpu::Limits {
        &self.limits
    }

    /// Whether this record describes the supplied device handle.
    #[must_use]
    pub fn matches(&self, device: &wgpu::Device) -> bool {
        *self.site == *device
    }

    /// The generation's pipeline creation authority.
    #[must_use]
    pub fn site(&self) -> &DeviceSite {
        &self.site
    }

    pub(crate) fn device(&self) -> &DeviceSite {
        &self.site
    }
}
