//! Immutable facts captured once by the platform for one adapter generation.
//! Render preparation consumes this record; pass recording never probes a device.

/// Enabled GPU capabilities and identity for one device generation.
#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    device: wgpu::Device,
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
    /// Capture the adapter and enabled device at platform creation.
    #[must_use]
    pub fn capture(adapter: &wgpu::Adapter, device: &wgpu::Device, generation: u64) -> Self {
        Self::from_info(device, generation, adapter.get_info())
    }

    /// Capture from a device when the adapter handle is no longer retained.
    /// The live platform should pass its adapter directly to `capture`.
    #[must_use]
    pub fn capture_device(device: &wgpu::Device, generation: u64) -> Self {
        Self::from_info(device, generation, device.adapter_info())
    }

    fn from_info(device: &wgpu::Device, generation: u64, adapter: wgpu::AdapterInfo) -> Self {
        Self {
            device: device.clone(),
            generation,
            adapter,
            features: device.features(),
            limits: device.limits(),
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
        self.device == *device
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }
}
