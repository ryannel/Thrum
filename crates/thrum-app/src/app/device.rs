//! The platform's adapter/device recipe, shared by initial setup and recovery.

use std::sync::Arc;

use super::{display::Headroom, output_scale::OutputRequest};

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
    pub(super) metal_preparation: Option<thrum_render::capabilities::metallib::Preparation>,
    pub(super) headroom: Option<Arc<Headroom>>,
    pub(super) edr_readback: bool,
}

pub(super) struct PlatformGpu {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) watch: DeviceWatch,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) edr: Option<wgpu::SurfaceConfiguration>,
    pub(super) adapter_name: String,
}

impl PlatformGpu {
    pub(super) fn config_for(
        &self,
        extent: [u32; 2],
        format: wgpu::TextureFormat,
    ) -> wgpu::SurfaceConfiguration {
        let mut config = if format == wgpu::TextureFormat::Rgba16Float {
            self.edr.as_ref().unwrap_or(&self.config).clone()
        } else {
            self.config.clone()
        };
        [config.width, config.height] = extent;
        config
    }
}

impl WindowDevice {
    pub(super) async fn create(
        &self,
        generation: u64,
        output: OutputRequest,
        wake: &EventLoopProxy<UserEvent>,
        keep_edr: bool,
    ) -> Result<PlatformGpu, String> {
        let signal = wake.clone();
        let OpenedDevice {
            adapter,
            device,
            queue,
            mut capabilities,
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
        capabilities.set_metal_preparation(self.metal_preparation.clone());
        let config = sdr_surface_config(&self.surface, &adapter, output.extent())
            .ok_or("no compatible SDR surface configuration")?;
        let edr = self.headroom.as_ref().and_then(|_| {
            // Metal keeps render-only drawables framebuffer-only. Request copy
            // access for the session that reads the float drawable.
            let mut usages = wgpu::TextureUsages::RENDER_ATTACHMENT;
            if self.edr_readback {
                usages |= wgpu::TextureUsages::COPY_SRC;
            }
            let surface = self.surface.get_capabilities(&adapter);
            (surface.formats.contains(&wgpu::TextureFormat::Rgba16Float)
                && surface.usages.contains(usages))
            .then(|| wgpu::SurfaceConfiguration {
                format: wgpu::TextureFormat::Rgba16Float,
                color_space: wgpu::SurfaceColorSpace::ExtendedSrgbLinear,
                usage: usages,
                ..config.clone()
            })
        });
        let wants_edr = keep_edr
            || self
                .headroom
                .as_ref()
                .is_some_and(|state| state.load().1 > 1.0);
        let config = if wants_edr {
            edr.clone().unwrap_or(config)
        } else {
            config
        };
        Ok(PlatformGpu {
            device,
            queue,
            capabilities,
            watch,
            config,
            edr,
            adapter_name: adapter.get_info().name,
        })
    }
}
