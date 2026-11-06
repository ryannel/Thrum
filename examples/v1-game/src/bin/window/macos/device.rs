//! The platform's adapter/device recipe, shared by initial setup and recovery.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use thrum_render::capabilities::DeviceCapabilities;
use winit::event_loop::EventLoopProxy;

use super::UserEvent;

#[derive(Clone)]
pub(super) struct DeviceWatch {
    pub(super) generation: u64,
    lost: Arc<AtomicBool>,
}

impl DeviceWatch {
    pub(super) fn register(
        device: &wgpu::Device,
        generation: u64,
        notify: impl Fn(u64) + Send + 'static,
    ) -> Self {
        let lost = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&lost);
        device.set_device_lost_callback(move |_, _| {
            signal.store(true, Ordering::Release);
            notify(generation);
        });
        Self { generation, lost }
    }

    pub(super) fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
}

pub(super) struct DeviceRecipe {
    pub(super) instance: wgpu::Instance,
    pub(super) surface: Arc<wgpu::Surface<'static>>,
}

pub(super) struct PlatformGpu {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) watch: DeviceWatch,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) adapter_name: String,
}

impl DeviceRecipe {
    pub(super) async fn create(
        &self,
        generation: u64,
        extent: [u32; 2],
        wake: &EventLoopProxy<UserEvent>,
    ) -> Result<PlatformGpu, String> {
        // wgpu's public contract asks for a fresh Adapter for each Device.
        let adapter = self
            .instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&self.surface),
                power_preference: wgpu::PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .map_err(|error| format!("adapter: {error:?}"))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: DeviceCapabilities::requested_timing_features(&adapter),
                ..Default::default()
            })
            .await
            .map_err(|error| format!("device: {error:?}"))?;
        let signal = wake.clone();
        let watch = DeviceWatch::register(&device, generation, move |generation| {
            let _ = signal.send_event(UserEvent::DeviceLost(generation));
        });
        let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
        let mut config = self
            .surface
            .get_default_config(&adapter, extent[0].max(1), extent[1].max(1))
            .ok_or("no compatible surface configuration")?;
        let formats = self.surface.get_capabilities(&adapter).formats;
        config.format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ]
        .into_iter()
        .find(|format| formats.contains(format))
        .ok_or("no supported 8-bit SDR surface format")?;
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

#[cfg(test)]
#[path = "device_tests.rs"]
mod tests;
