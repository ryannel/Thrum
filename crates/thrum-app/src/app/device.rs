//! The platform's adapter/device recipe, shared by initial setup and recovery.

use std::sync::Arc;

use thrum_render::capabilities::{
    DeviceCapabilities, DeviceRecipe, DeviceWatch, HardwareRequest, OpenedDevice,
    sdr_surface_config,
};
use winit::event_loop::EventLoopProxy;

use super::UserEvent;

pub(super) struct WindowDevice {
    pub(super) instance: wgpu::Instance,
    pub(super) surface: Arc<wgpu::Surface<'static>>,
    pub(super) hardware: Vec<HardwareRequest>,
}

pub(super) struct PlatformGpu {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) watch: DeviceWatch,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) adapter_name: String,
}

impl WindowDevice {
    pub(super) async fn create(
        &self,
        generation: u64,
        extent: [u32; 2],
        wake: &EventLoopProxy<UserEvent>,
    ) -> Result<PlatformGpu, String> {
        let signal = wake.clone();
        let OpenedDevice {
            adapter,
            device,
            queue,
            capabilities,
            watch,
        } = DeviceRecipe {
            generation,
            hardware: &self.hardware,
            ..Default::default()
        }
        .open_with_wake(&self.instance, Some(&self.surface), move |generation| {
            let _ = signal.send_event(UserEvent::DeviceLost(generation));
        })
        .await
        .map_err(|error| error.to_string())?;
        let config = sdr_surface_config(&self.surface, &adapter, extent)
            .ok_or("no compatible SDR surface configuration")?;
        Ok(PlatformGpu {
            device,
            queue,
            capabilities,
            watch,
            config,
            adapter_name: adapter.get_info().name,
        })
    }
}
