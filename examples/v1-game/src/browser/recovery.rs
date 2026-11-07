//! Platform-owned, detached graphics preparation. The mailbox contains no world
//! or game reference; dropping the game also abandons late preparation results.

use std::{cell::RefCell, rc::Rc, sync::Arc};

use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{DevicePrepareInputs, PreparedDevice},
};

use super::GraphicsFault;
use crate::FrameHost;

pub(super) struct GraphicsDevice {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) fault: Arc<GraphicsFault>,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) adapter_name: String,
}

pub(super) async fn request_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'static>,
    extent: [u32; 2],
    generation: u64,
) -> Result<GraphicsDevice, String> {
    // WebGPU permits one requestDevice per adapter. Each attempt requests a
    // fresh surface-compatible adapter, including its format/features recipe.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|error| error.to_string())?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(|error| error.to_string())?;
    let fault = GraphicsFault::register(&device);
    let capabilities = DeviceCapabilities::capture(&adapter, &device, generation);
    let mut config = surface
        .get_default_config(&adapter, extent[0], extent[1])
        .ok_or_else(|| "no compatible canvas surface configuration".to_owned())?;
    let formats = surface.get_capabilities(&adapter).formats;
    config.format = [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|format| formats.contains(format))
    .ok_or_else(|| "no compatible 8-bit SDR canvas format".to_owned())?;
    let name = capabilities.adapter().name.clone();
    Ok(GraphicsDevice {
        device,
        queue,
        capabilities,
        fault,
        config,
        adapter_name: if name.is_empty() {
            "browser WebGPU (adapter name withheld)".to_owned()
        } else {
            name
        },
    })
}

struct Replacement {
    gpu: GraphicsDevice,
    prepared: PreparedDevice,
    configured_extent: Option<[u32; 2]>,
}

impl Replacement {
    async fn prepare(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
        inputs: DevicePrepareInputs,
        extent: [u32; 2],
        generation: u64,
    ) -> Result<Box<Self>, String> {
        let gpu = request_device(instance, surface, extent, generation).await?;
        let prepared = inputs
            .prepare(
                &gpu.capabilities,
                &gpu.queue,
                Some(SurfaceProfile {
                    format: gpu.config.format,
                    extent,
                }),
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(Box::new(Self {
            gpu,
            prepared,
            configured_extent: None,
        }))
    }

    fn try_publish(
        &mut self,
        surface: &wgpu::Surface<'static>,
        host: &mut FrameHost,
        extent: [u32; 2],
    ) -> Result<Option<bool>, String> {
        if self.gpu.fault.is_lost() {
            return Err("replacement graphics device was lost before publication".to_owned());
        }
        // configure writes the DOM canvas dimensions on WebGPU. Never turn a
        // legitimately zero-sized canvas back into the saved nonzero extent.
        if extent.contains(&0) {
            return Ok(None);
        }
        if self.configured_extent != Some(extent) {
            self.prepared
                .resize_surface(extent)
                .map_err(|error| error.to_string())?;
            self.gpu.config.width = extent[0];
            self.gpu.config.height = extent[1];
            surface.configure(&self.gpu.device, &self.gpu.config);
            self.configured_extent = Some(extent);
        }
        // WebGPU configure returns void and can defer failure until acquire.
        // Check a real candidate lease; dropping it does not submit/present.
        let reconfigure = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(lease) => {
                drop(lease);
                false
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(lease) => {
                drop(lease);
                true
            }
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                return Err("replacement canvas configuration did not become usable".to_owned());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(None);
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err("replacement canvas surface validation fault".to_owned());
            }
        };
        if self.gpu.fault.is_lost() {
            return Err("replacement graphics device was lost during activation".to_owned());
        }
        host.publish_device(&mut self.prepared)
            .map_err(|error| error.to_string())?;
        Ok(Some(reconfigure))
    }
}

type Mailbox = Rc<RefCell<Option<Result<Box<Replacement>, String>>>>;

pub(super) struct Recovery {
    instance: wgpu::Instance,
    surface: Rc<wgpu::Surface<'static>>,
    pending: Option<Mailbox>,
    pub(super) error: Option<String>,
    last_attempted: u64,
    pub(super) generation: u64,
    pub(super) attempts: u64,
    pub(super) recovered: u64,
    pub(super) awaiting_first_present: bool,
}

impl Recovery {
    pub(super) fn new(instance: wgpu::Instance, surface: Rc<wgpu::Surface<'static>>) -> Self {
        Self {
            instance,
            surface,
            pending: None,
            error: None,
            last_attempted: FrameHost::DEVICE_GENERATION,
            generation: FrameHost::DEVICE_GENERATION,
            attempts: 0,
            recovered: 0,
            awaiting_first_present: false,
        }
    }

    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn request(&mut self, host: &FrameHost, extent: [u32; 2]) -> Result<(), String> {
        if self.is_pending() {
            return Err("graphics recovery is already preparing".to_owned());
        }
        let generation = self
            .last_attempted
            .checked_add(1)
            .ok_or_else(|| "graphics generation exhausted".to_owned())?;
        let inputs = host
            .device_prepare_inputs()
            .map_err(|error| error.to_string())?;
        let pending = Rc::new(RefCell::new(None));
        let destination = Rc::downgrade(&pending);
        let instance = self.instance.clone();
        let surface = Rc::clone(&self.surface);
        self.last_attempted = generation;
        self.attempts += 1;
        self.error = None;
        self.pending = Some(pending);
        wasm_bindgen_futures::spawn_local(async move {
            let result =
                Replacement::prepare(&instance, &surface, inputs, extent, generation).await;
            if let Some(destination) = destination.upgrade() {
                *destination.borrow_mut() = Some(result);
            }
        });
        Ok(())
    }

    pub(super) fn poll(
        &mut self,
        host: &mut FrameHost,
        extent: [u32; 2],
    ) -> Option<(GraphicsDevice, bool)> {
        let result = self.pending.as_ref()?.borrow_mut().take()?;
        match result.and_then(|mut replacement| {
            let ready = replacement.try_publish(&self.surface, host, extent)?;
            Ok((replacement, ready))
        }) {
            Ok((replacement, Some(reconfigure))) => {
                self.pending = None;
                self.generation = replacement.gpu.capabilities.generation();
                self.recovered += 1;
                self.awaiting_first_present = true;
                Some((replacement.gpu, reconfigure))
            }
            Ok((replacement, None)) => {
                if let Some(pending) = &self.pending {
                    *pending.borrow_mut() = Some(Ok(replacement));
                }
                None
            }
            Err(error) => {
                self.pending = None;
                self.error = Some(error);
                None
            }
        }
    }
}
