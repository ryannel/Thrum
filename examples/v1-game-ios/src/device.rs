//! The iPhone device request used by both preparation and recovery.

use thrum_render::capabilities::{DeviceRecipe, HardwareLaunchError, OpenedDevice};

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
    let OpenedDevice {
        adapter,
        device,
        queue,
        ..
    } = pollster::block_on(DeviceRecipe::default().open(instance, None)).map_err(|error| {
        match error {
            HardwareLaunchError::Adapter(error) => DeviceRequestError::Adapter(error),
            HardwareLaunchError::Device { source, .. } => DeviceRequestError::Device(source),
            _ => unreachable!("the default iOS request has no hardware overrides"),
        }
    })?;
    Ok((adapter, device, queue))
}
