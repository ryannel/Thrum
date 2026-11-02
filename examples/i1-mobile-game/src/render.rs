//! A single-color GPU response for the first phone-shell lifecycle proof.
//! The world remains in the headless authority; this owns presentation only.

use std::sync::Arc;

use wgpu::{
    Backends, Color, CommandEncoderDescriptor, CurrentSurfaceTexture, Device, DeviceDescriptor,
    Instance, InstanceDescriptor, LoadOp, Operations, PowerPreference, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface,
    SurfaceConfiguration, TextureViewDescriptor,
};
use winit::window::Window;

pub struct ClearRenderer {
    instance: Instance,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    surface: Option<Surface<'static>>,
    config: Option<SurfaceConfiguration>,
}

/// Whether one requested response reached the display surface.
pub enum DrawOutcome {
    /// A frame was submitted for presentation.
    Presented,
    /// No frame was acquired; keep one redraw request pending for retry.
    Deferred,
}

impl ClearRenderer {
    pub fn new(window: Arc<Window>) -> Result<Self, String> {
        #[cfg(target_os = "ios")]
        let backends = Backends::METAL;
        #[cfg(target_os = "android")]
        let backends = Backends::VULKAN;
        let mut descriptor = InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        let instance = Instance::new(descriptor);
        let surface = instance
            .create_surface(window.clone())
            .map_err(|error| format!("create surface: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| format!("request adapter: {error}"))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default()))
                .map_err(|error| format!("request device: {error}"))?;
        let mut renderer = Self {
            instance,
            adapter,
            device,
            queue,
            surface: Some(surface),
            config: None,
        };
        renderer.resize(window.inner_size().width, window.inner_size().height)?;
        Ok(renderer)
    }

    pub fn surface_lost(&mut self) {
        self.config = None;
        self.surface = None;
    }

    pub fn surface_ready(&mut self, window: Arc<Window>) -> Result<(), String> {
        if self.surface.is_none() {
            self.surface = Some(
                self.instance
                    .create_surface(window.clone())
                    .map_err(|error| format!("recreate surface: {error}"))?,
            );
        }
        self.resize(window.inner_size().width, window.inner_size().height)
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            self.config = None;
            return Ok(());
        }
        if self
            .config
            .as_ref()
            .is_some_and(|config| config.width == width && config.height == height)
        {
            return Ok(());
        }
        let surface = self.surface.as_ref().ok_or("surface unavailable")?;
        let config = surface
            .get_default_config(&self.adapter, width, height)
            .ok_or("no compatible surface format")?;
        surface.configure(&self.device, &config);
        self.config = Some(config);
        Ok(())
    }

    pub fn draw(&mut self, color: Color) -> Result<DrawOutcome, String> {
        let Some(surface) = &self.surface else {
            return Ok(DrawOutcome::Deferred);
        };
        if self.config.is_none() {
            return Ok(DrawOutcome::Deferred);
        }
        let frame = match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                return Ok(DrawOutcome::Deferred);
            }
            CurrentSurfaceTexture::Outdated => {
                if let Some(config) = &self.config {
                    surface.configure(&self.device, config);
                }
                return Ok(DrawOutcome::Deferred);
            }
            CurrentSurfaceTexture::Lost => {
                self.surface_lost();
                return Err("surface lost during acquire".into());
            }
            CurrentSurfaceTexture::Validation => {
                return Err("surface acquire validation error".into());
            }
        };
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(color),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        Ok(DrawOutcome::Presented)
    }
}
