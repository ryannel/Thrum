//! GPU admission for tests. This package is a dev-dependency only.
use std::io::Write as _;
#[cfg(feature = "render")]
pub use thrum_render::capabilities::DeviceProfile as Profile;
#[cfg(feature = "render")]
use thrum_render::capabilities::{
    DeviceCapabilities, DeviceRecipe, DeviceRestriction, HardwareLaunchError, HardwareRequest,
};

#[cfg(feature = "render")]
mod light_graph;
#[cfg(feature = "render")]
pub use light_graph::BeforeLightSplit;

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
#[cfg(all(target_arch = "wasm32", feature = "render"))]
pub async fn browser_gpu() -> Result<(Gpu, wgpu::Device, wgpu::Queue), Box<dyn std::error::Error>> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let thrum_render::capabilities::OpenedDevice {
        adapter,
        device,
        queue,
        ..
    } = DeviceRecipe {
        profile: Profile::Portable,
        restriction: Some(&DeviceRestriction {
            features: Some(wgpu::Features::empty()),
            ..Default::default()
        }),
        ..Default::default()
    }
    .open(&instance, None)
    .await?;
    Ok((Gpu { instance, adapter }, device, queue))
}

/// Report that a test's GPU prerequisite could not be met.
///
/// # Panics
/// Fails with the same reason when `THRUM_REQUIRE_GPU=1`.
pub fn skip(test: &str, reason: &str) {
    let message = format!("thrum-gpu-skip: {test}: {reason}");
    assert!(
        std::env::var_os("THRUM_REQUIRE_GPU").as_deref() != Some(std::ffi::OsStr::new("1")),
        "{message}"
    );
    writeln!(std::io::stderr().lock(), "{message}").expect("write visible GPU skip reason");
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
    gpu_on(test, &instance)
}

/// Admit an adapter on a test-owned instance whose backend selection is already fixed.
///
/// # Panics
/// Fails with the skip reason when `THRUM_REQUIRE_GPU=1` and no adapter is available.
pub fn gpu_on(test: &str, instance: &wgpu::Instance) -> Option<Gpu> {
    match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) {
        Ok(adapter) => Some(Gpu {
            instance: instance.clone(),
            adapter,
        }),
        Err(error) => {
            skip(test, &format!("adapter unavailable: {error}"));
            None
        }
    }
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
impl Restriction {
    /// Translate the gate's storage cap and feature mask to the engine recipe's restriction.
    #[must_use]
    pub fn for_profile(&self, profile: Profile, adapter: &wgpu::Adapter) -> DeviceRestriction {
        let (limits, features) = self.apply(profile.limits(adapter), wgpu::Features::all());
        DeviceRestriction {
            limits: self.storage_buffers.map(|_| limits),
            remove_features: wgpu::Features::all() - features,
            ..Default::default()
        }
    }
}

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
}

#[cfg(feature = "render")]
impl Gpu {
    /// Open a portable device with the profile's timing features or a subset.
    ///
    /// # Panics
    /// Fails if the admitted adapter cannot create the requested device.
    pub fn portable(&self, features: Option<wgpu::Features>) -> (wgpu::Device, wgpu::Queue) {
        self.device(
            Profile::Portable,
            &Restriction {
                features,
                ..Default::default()
            },
        )
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
        self.device_with_hardware_adjusted(profile, restriction, generation, requests, |_| {})
    }

    /// Open a generation with an adjustment to the test profile's base device descriptor.
    /// The adjustment runs again if the registry retries without a preference.
    ///
    /// # Errors
    /// Reports registry admission or device creation failures.
    pub fn device_with_hardware_adjusted(
        &self,
        profile: Profile,
        restriction: &Restriction,
        generation: u64,
        requests: &[HardwareRequest],
        adjust: impl Fn(&mut wgpu::DeviceDescriptor<'static>),
    ) -> Result<(wgpu::Device, wgpu::Queue, DeviceCapabilities), HardwareLaunchError> {
        let restriction = restriction.for_profile(profile, &self.adapter);
        let recipe = DeviceRecipe {
            profile,
            generation,
            restriction: Some(&restriction),
            hardware: requests,
        };
        let mut admitted = Some(self.adapter.clone());
        let thrum_render::capabilities::OpenedDevice {
            device,
            queue,
            capabilities,
            ..
        } = pollster::block_on(recipe.open_from(
            async || {
                if let Some(adapter) = admitted.take() {
                    return Ok(adapter);
                }
                self.instance
                    .request_adapter(&wgpu::RequestAdapterOptions::default())
                    .await
            },
            |_| {},
            adjust,
        ))?;
        Ok((device, queue, capabilities))
    }
}

#[cfg(test)]
mod tests;
