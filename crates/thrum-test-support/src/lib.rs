//! GPU admission for tests. This package is a dev-dependency only.
use std::io::Write as _;
#[cfg(feature = "render")]
use thrum_render::capabilities::{DeviceCapabilities, HardwareLaunchError, HardwareRequest};

/// Gate result retaining the instance and admitted adapter.
pub struct Gpu {
    /// Instance owning the selected backend.
    pub instance: wgpu::Instance,
    /// Admitted adapter.
    pub adapter: wgpu::Adapter,
}

/// Admit the browser WebGPU backend at portable limits without a canvas.
///
/// # Errors
/// Reports unavailable WebGPU or a failed portable device request.
#[cfg(target_arch = "wasm32")]
pub async fn browser_gpu() -> Result<(Gpu, wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?;
    Ok((Gpu { instance, adapter }, device, queue))
}

fn absent(test: &str, reason: &str) -> Option<Gpu> {
    let message = format!("thrum-gpu-skip: {test}: {reason}");
    assert!(
        std::env::var_os("THRUM_REQUIRE_GPU").as_deref() != Some(std::ffi::OsStr::new("1")),
        "{message}"
    );
    writeln!(std::io::stderr().lock(), "{message}").expect("write visible GPU skip reason");
    None
}

/// Admit a GPU, optionally requiring a particular backend. None means a reported skip.
///
/// # Panics
/// Fails with the skip reason when `THRUM_REQUIRE_GPU=1` and no adapter is available.
pub fn gpu(test: &str, backend: Option<wgpu::Backend>) -> Option<Gpu> {
    admit(
        test,
        backend.map_or(wgpu::Backends::all(), wgpu::Backends::from),
    )
}

fn admit(test: &str, backends: wgpu::Backends) -> Option<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
        Ok(adapter) => Some(Gpu { instance, adapter }),
        Err(error) => absent(test, &format!("{backends:?} adapter unavailable: {error}")),
    }
}

/// Test-device profile, to be supplied by HOST's recipe in M2.
#[cfg(feature = "render")]
#[derive(Clone, Copy)]
pub enum Profile {
    /// WebGPU default limits.
    Portable,
    /// Adapter limits.
    Native,
}

/// A restriction can only remove capabilities from the selected profile.
#[cfg(feature = "render")]
#[derive(Default)]
pub struct Restriction {
    /// Cap storage buffers per shader stage.
    pub storage_buffers: Option<u32>,
    /// Remove all features outside this mask.
    pub features: Option<wgpu::Features>,
}

#[cfg(feature = "render")]
impl Restriction {
    fn apply(
        &self,
        mut limits: wgpu::Limits,
        features: wgpu::Features,
    ) -> (wgpu::Limits, wgpu::Features) {
        if let Some(cap) = self.storage_buffers {
            limits.max_storage_buffers_per_shader_stage =
                limits.max_storage_buffers_per_shader_stage.min(cap);
        }
        (
            limits,
            self.features.map_or(features, |mask| features & mask),
        )
    }
}

#[cfg(feature = "render")]
impl Profile {
    /// The gate's base recipe, for tests that opt into hardware through the renderer registry.
    #[must_use]
    pub fn descriptor(
        self,
        adapter: &wgpu::Adapter,
        restriction: &Restriction,
    ) -> wgpu::DeviceDescriptor<'static> {
        let limits = match self {
            Self::Portable => wgpu::Limits::default(),
            Self::Native => adapter.limits(),
        };
        let features = adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                | wgpu::Features::TEXTURE_COMPRESSION_ASTC
                | wgpu::Features::TEXTURE_COMPRESSION_BC
                | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D);
        let (limits, features) = restriction.apply(limits, features);
        wgpu::DeviceDescriptor {
            required_limits: limits,
            required_features: features,
            ..Default::default()
        }
    }
}

#[cfg(feature = "render")]
impl Gpu {
    /// Fresh adapter for device-replacement tests, retaining the same instance.
    /// Backend device identities are only comparable within that instance.
    /// # Panics
    /// Fails if the already admitted instance cannot provide another adapter.
    #[must_use]
    pub fn fresh_adapter(&self) -> Self {
        let adapter = pollster::block_on(
            self.instance
                .request_adapter(&wgpu::RequestAdapterOptions::default()),
        )
        .expect("replacement adapter on admitted GPU instance");
        Self {
            instance: self.instance.clone(),
            adapter,
        }
    }

    /// Open the ordinary device for this profile. Restrictions never add capabilities.
    /// # Panics
    /// Fails if the admitted adapter cannot create the requested device.
    pub fn device(
        &self,
        profile: Profile,
        restriction: &Restriction,
    ) -> (wgpu::Device, wgpu::Queue) {
        let (device, queue, _) = self
            .device_with_hardware(profile, restriction, 1, &[])
            .expect("admitted GPU test device");
        (device, queue)
    }

    /// Open a device for this profile with explicit named hardware and capture its registry.
    ///
    /// # Errors
    /// Reports registry admission or device creation failures.
    pub fn device_with_hardware(
        &self,
        profile: Profile,
        restriction: &Restriction,
        generation: u64,
        requests: &[HardwareRequest],
    ) -> Result<(wgpu::Device, wgpu::Queue, DeviceCapabilities), HardwareLaunchError> {
        let mut admitted = Some(self.adapter.clone());
        let (_, device, queue, capabilities) =
            pollster::block_on(DeviceCapabilities::open_generation_from(
                async || {
                    if let Some(adapter) = admitted.take() {
                        return Ok(adapter);
                    }
                    self.instance
                        .request_adapter(&wgpu::RequestAdapterOptions::default())
                        .await
                },
                generation,
                requests,
                |adapter| profile.descriptor(adapter, restriction),
                |adapter| {
                    let (limits, features) =
                        restriction.apply(adapter.limits(), adapter.features());
                    (features, limits)
                },
            ))?;
        Ok((device, queue, capabilities))
    }
}

#[cfg(test)]
mod tests;
