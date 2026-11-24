//! Platform-owned, detached graphics preparation. The mailbox contains no world
//! or game reference; dropping the game also abandons late preparation results.

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, JsValue, prelude::wasm_bindgen};
use web_sys::HtmlCanvasElement;

use crate::surface::{SurfaceAction, surface_action};

use thrum_render::{
    capabilities::{
        DeviceCapabilities, DeviceProfile, DeviceRecipe, DeviceRestriction, DeviceWatch,
        OpenedDevice, configure_surface, sdr_surface_config,
    },
    present::SurfaceProfile,
    runtime::{DevicePrepareInputs, FrameHost, FrameWorld, PreparedDevice, SceneSource},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = GPUCanvasContext)]
    type CanvasContext;

    #[wasm_bindgen(method)]
    fn unconfigure(this: &CanvasContext);

    #[wasm_bindgen(catch, method, js_name = getConfiguration)]
    fn configuration(this: &CanvasContext) -> Result<Option<js_sys::Object>, JsValue>;
}

pub(super) struct GraphicsDevice {
    pub(super) device: wgpu::Device,
    pub(super) queue: wgpu::Queue,
    pub(super) capabilities: DeviceCapabilities,
    pub(super) fault: DeviceWatch,
    pub(super) config: wgpu::SurfaceConfiguration,
    pub(super) adapter_name: String,
}

pub(super) async fn open_canvas_device(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'static>,
    extent: [u32; 2],
    generation: u64,
    remove_features: wgpu::Features,
) -> Result<GraphicsDevice, String> {
    let restriction = DeviceRestriction {
        remove_features,
        ..Default::default()
    };
    let OpenedDevice {
        adapter,
        device,
        queue,
        capabilities,
        watch: fault,
    } = DeviceRecipe {
        profile: DeviceProfile::Portable,
        generation,
        restriction: (!remove_features.is_empty()).then_some(&restriction),
        ..Default::default()
    }
    .open(instance, Some(surface))
    .await
    .map_err(|error| error.to_string())?;
    let config = sdr_surface_config(surface, &adapter, extent)
        .ok_or_else(|| "no compatible SDR canvas configuration".to_owned())?;
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
    retry_extent: Option<[u32; 2]>,
}

impl Replacement {
    async fn prepare(
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'static>,
        inputs: DevicePrepareInputs,
        extent: [u32; 2],
        generation: u64,
        remove_features: wgpu::Features,
    ) -> Result<Box<Self>, String> {
        let gpu =
            open_canvas_device(instance, surface, extent, generation, remove_features).await?;
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
            retry_extent: None,
        }))
    }

    fn try_publish<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        surface: &wgpu::Surface<'static>,
        canvas: &HtmlCanvasElement,
        host: &mut FrameHost<W, V>,
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
                .resize_scene_surface(extent)
                .map_err(|error| error.to_string())?;
            self.gpu.config.width = extent[0];
            self.gpu.config.height = extent[1];
            let context = canvas
                .get_context("webgpu")
                .ok()
                .flatten()
                .and_then(|context| context.dyn_into::<CanvasContext>().ok())
                .ok_or("replacement canvas context is unavailable")?;
            // The retired device has no drawable in use. Clear the last
            // configuration so a thrown configure cannot leave it looking
            // like this attempt succeeded, even on the same candidate.
            context.unconfigure();
            configure_surface(surface, &self.gpu.device, &self.gpu.config)
                .map_err(|error| error.to_string())?;
            if matches!(context.configuration(), Ok(None)) {
                return Err("replacement canvas configuration did not become usable".to_owned());
            }
            self.configured_extent = Some(extent);
        }
        // Check a real candidate lease before publishing. Dropping the
        // lease does not submit or present.
        let action = surface_action(surface.get_current_texture());
        if self.gpu.fault.is_lost() {
            return Err("replacement graphics device was lost during activation".to_owned());
        }
        let reconfigure = match action {
            SurfaceAction::Draw { frame, reconfigure } => {
                drop(frame);
                reconfigure
            }
            SurfaceAction::Skip { .. } => return Ok(None),
            SurfaceAction::Reconfigure { validation } => {
                // Reconfigure the same candidate after a transient drawable
                // loss. Bound repeated Lost/Outdated outcomes even when the
                // browser does not expose getConfiguration.
                if !validation && self.retry_extent == Some(extent) {
                    return Err("replacement canvas configuration did not become usable".to_owned());
                }
                self.retry_extent = (!validation).then_some(extent);
                self.configured_extent = None;
                return Ok(None);
            }
        };
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
    remove_features: wgpu::Features,
    pub(super) error: Option<String>,
    last_attempted: u64,
    pub(super) generation: u64,
    pub(super) attempts: u64,
    pub(super) recovered: u64,
    pub(super) awaiting_first_present: bool,
}

impl Recovery {
    pub(super) fn new(
        instance: wgpu::Instance,
        surface: Rc<wgpu::Surface<'static>>,
        generation: u64,
        remove_features: wgpu::Features,
    ) -> Self {
        Self {
            instance,
            surface,
            pending: None,
            remove_features,
            error: None,
            last_attempted: generation,
            generation,
            attempts: 0,
            recovered: 0,
            awaiting_first_present: false,
        }
    }

    pub(super) fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    pub(super) fn request<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        host: &FrameHost<W, V>,
        extent: [u32; 2],
    ) -> Result<(), String> {
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
        let remove_features = self.remove_features;
        wasm_bindgen_futures::spawn_local(async move {
            let result = Replacement::prepare(
                &instance,
                &surface,
                inputs,
                extent,
                generation,
                remove_features,
            )
            .await;
            if let Some(destination) = destination.upgrade() {
                *destination.borrow_mut() = Some(result);
            }
        });
        Ok(())
    }

    pub(super) fn poll<W: FrameWorld, V: SceneSource<W::Page>>(
        &mut self,
        canvas: &HtmlCanvasElement,
        host: &mut FrameHost<W, V>,
        extent: [u32; 2],
    ) -> Option<(GraphicsDevice, bool)> {
        let result = self.pending.as_ref()?.borrow_mut().take()?;
        match result.and_then(|mut replacement| {
            let ready = replacement.try_publish(&self.surface, canvas, host, extent)?;
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
