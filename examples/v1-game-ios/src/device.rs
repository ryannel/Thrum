//! The iPhone device request used by both preparation and recovery.

use thrum_render::capabilities::DeviceCapabilities;

/// Failure to obtain the iPhone shell's GPU resources.
#[derive(Debug)]
pub enum DeviceRequestError {
    /// No compatible adapter could be selected.
    Adapter(wgpu::RequestAdapterError),
    /// The selected adapter could not create a device.
    Device(wgpu::RequestDeviceError),
}

impl std::fmt::Display for DeviceRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Adapter(error) => write!(f, "iOS adapter: {error}"),
            Self::Device(error) => write!(f, "iOS device: {error}"),
        }
    }
}

impl std::error::Error for DeviceRequestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Adapter(error) => error,
            Self::Device(error) => error,
        })
    }
}

/// Request the adapter's content features and native limits.
///
/// # Errors
/// Returns an adapter or device creation failure.
pub fn request_device(
    instance: &wgpu::Instance,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), DeviceRequestError> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(DeviceRequestError::Adapter)?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: DeviceCapabilities::requested_timing_features(&adapter)
            | DeviceCapabilities::requested_content_features(&adapter),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .map_err(DeviceRequestError::Device)?;
    Ok((adapter, device, queue))
}
