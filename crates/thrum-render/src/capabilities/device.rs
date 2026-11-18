//! The device request shared by shells, offscreen hosts and diagnostics.

use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use super::{DeviceCapabilities, HardwareLaunchError, HardwareRequest};

/// First generation of an engine-owned GPU device.
pub const FIRST_DEVICE_GENERATION: u64 = 1;

/// Features and limits for an ordinary device request.
#[derive(Clone, Copy, Debug, Default)]
pub enum DeviceProfile {
    /// Use the adapter's limits and all timing and content features it offers.
    #[default]
    Native,
    /// Keep the browser's WebGPU default limits and timing features. Cooked
    /// textures stay uncompressed so Chrome's established output stays exact.
    Portable,
}

/// Replace specific parts of the selected request with lower capabilities.
#[derive(Default)]
pub struct DeviceRestriction {
    /// Complete replacement limits, each at most as capable as the profile.
    pub limits: Option<wgpu::Limits>,
    /// Exact subset of the profile's enabled features.
    pub features: Option<wgpu::Features>,
    /// Feature mask to remove a family without probing the adapter at the call site.
    pub remove_features: wgpu::Features,
}

/// Input to the shared device request. Shells leave the restriction unset.
pub struct DeviceRecipe<'a> {
    /// Native or portable request profile.
    pub profile: DeviceProfile,
    /// Identity assigned by the host's recovery state.
    pub generation: u64,
    /// Optional reduction for diagnostics and tests.
    pub restriction: Option<&'a DeviceRestriction>,
    /// Explicit opt-in hardware requests for this launch.
    pub hardware: &'a [HardwareRequest],
}

impl Default for DeviceRecipe<'_> {
    fn default() -> Self {
        Self {
            profile: DeviceProfile::Native,
            generation: FIRST_DEVICE_GENERATION,
            restriction: None,
            hardware: &[],
        }
    }
}

/// A device, its enabled capabilities and its generation's fault watch.
pub struct OpenedDevice {
    /// Adapter retained for surface configuration.
    pub adapter: wgpu::Adapter,
    /// Device handle.
    pub device: wgpu::Device,
    /// Queue paired with the device.
    pub queue: wgpu::Queue,
    /// The registry's immutable record of what the device enabled.
    pub capabilities: DeviceCapabilities,
    /// Callback state for this device generation.
    pub watch: DeviceWatch,
}

impl DeviceRecipe<'_> {
    /// Select a surface-compatible adapter and open the device.
    ///
    /// # Errors
    /// Reports an invalid restriction or a hardware, adapter or device failure.
    pub async fn open(
        &self,
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<OpenedDevice, HardwareLaunchError> {
        self.open_with_wake(instance, surface, |_| {}).await
    }

    /// Open a device and notify the platform when its generation is lost.
    ///
    /// # Errors
    /// Reports an invalid restriction or a hardware, adapter or device failure.
    pub async fn open_with_wake(
        &self,
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
        wake: impl Fn(u64) + Send + 'static,
    ) -> Result<OpenedDevice, HardwareLaunchError> {
        let options = wgpu::RequestAdapterOptions {
            compatible_surface: surface,
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        };
        let (adapter, device, queue, capabilities) = DeviceCapabilities::open_planned_generation(
            async || instance.request_adapter(&options).await,
            self.generation,
            self.hardware,
            |adapter| {
                let mut features = DeviceCapabilities::requested_timing_features(adapter);
                let adapter_limits = adapter.limits();
                let adapter_features = adapter.features();
                let profile_limits = match self.profile {
                    DeviceProfile::Native => {
                        features |= DeviceCapabilities::requested_content_features(adapter);
                        adapter_limits.clone()
                    }
                    DeviceProfile::Portable => wgpu::Limits::default(),
                };
                let (limits, hardware_offer) = if let Some(restriction) = self.restriction {
                    let limits = restriction
                        .limits
                        .clone()
                        .unwrap_or_else(|| profile_limits.clone());
                    let subset = restriction.features.unwrap_or(features);
                    if !features.contains(subset) || !limits.check_limits(&profile_limits) {
                        return Err(HardwareLaunchError::InvalidRestriction);
                    }
                    features = subset - restriction.remove_features;
                    (limits.clone(), (features, limits))
                } else {
                    (profile_limits, (adapter_features, adapter_limits))
                };
                Ok((
                    wgpu::DeviceDescriptor {
                        required_features: features,
                        required_limits: limits,
                        ..Default::default()
                    },
                    hardware_offer,
                ))
            },
        )
        .await?;
        let watch = DeviceWatch::register(&device, self.generation, wake);
        Ok(OpenedDevice {
            adapter,
            device,
            queue,
            capabilities,
            watch,
        })
    }
}

#[derive(Default)]
struct Faults {
    retired: AtomicBool,
    loss: OnceLock<String>,
    uncaptured: OnceLock<String>,
}

/// Shared fault state for exactly one device generation.
#[derive(Clone)]
pub struct DeviceWatch {
    /// Generation reported by this watch and its wake callback.
    pub generation: u64,
    faults: Arc<Faults>,
}

impl DeviceWatch {
    #[expect(
        clippy::print_stderr,
        reason = "diagnose the first asynchronous GPU error on headless hosts as well as shells"
    )]
    fn register(
        device: &wgpu::Device,
        generation: u64,
        wake: impl Fn(u64) + Send + 'static,
    ) -> Self {
        let faults = Arc::new(Faults::default());
        let signal = Arc::clone(&faults);
        device.set_device_lost_callback(move |reason, message| {
            drop(signal.loss.set(format!("{reason:?}: {message}")));
            signal.retired.store(true, Ordering::Release);
            wake(generation);
        });
        let errors = Arc::clone(&faults);
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            if errors.uncaptured.set(error.to_string()).is_ok() {
                eprintln!("GPU generation {generation}: {error}");
            }
        }));
        Self { generation, faults }
    }

    /// Whether the generation can no longer accept GPU work.
    #[must_use]
    pub fn is_lost(&self) -> bool {
        self.faults.retired.load(Ordering::Acquire)
    }

    /// Retire this generation on a delayed execution error, even when wgpu
    /// does not emit its device-lost callback.
    pub fn retire(&self) {
        self.faults.retired.store(true, Ordering::Release);
    }

    /// First loss callback message from this generation.
    #[must_use]
    pub fn loss_message(&self) -> Option<&str> {
        self.faults.loss.get().map(String::as_str)
    }

    /// First uncaptured error from this generation. The generation may still be live.
    #[must_use]
    pub fn uncaptured_error(&self) -> Option<&str> {
        self.faults.uncaptured.get().map(String::as_str)
    }
}

/// Preferred SDR drawable format, also used before an iOS surface is leased.
pub const PREFERRED_SDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// Select the engine's preferred SDR format from the surface's offer.
#[must_use]
pub fn sdr_surface_format(formats: &[wgpu::TextureFormat]) -> Option<wgpu::TextureFormat> {
    [
        PREFERRED_SDR_FORMAT,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|format| formats.contains(format))
}

/// Obtain a surface configuration using the engine's SDR preference.
#[must_use]
pub fn sdr_surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    extent: [u32; 2],
) -> Option<wgpu::SurfaceConfiguration> {
    let mut config = surface.get_default_config(adapter, extent[0].max(1), extent[1].max(1))?;
    config.format = sdr_surface_format(&surface.get_capabilities(adapter).formats)?;
    Some(config)
}

/// Validation failures from both attempts to configure a native surface.
#[derive(Debug)]
pub struct SurfaceConfigError {
    first: wgpu::Error,
    retry: wgpu::Error,
}

impl std::fmt::Display for SurfaceConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "surface configuration: {}; retry: {}",
            self.first, self.retry
        )
    }
}

impl std::error::Error for SurfaceConfigError {}

/// Configure and retry once if native validation rejects a stale drawable.
/// WebGPU configure returns void and the shell checks the next drawable.
///
/// # Errors
/// Reports both native validation failures when the retry also fails.
pub fn configure_surface(
    surface: &wgpu::Surface<'_>,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> Result<(), SurfaceConfigError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let configure = || {
            let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
            surface.configure(device, config);
            pollster::block_on(errors.pop())
        };
        // wgpu retires the stale presentation on the first configure even
        // when it reports an outstanding drawable left by a lost device.
        if let Some(first) = configure()
            && let Some(retry) = configure()
        {
            return Err(SurfaceConfigError { first, retry });
        }
    }
    #[cfg(target_arch = "wasm32")]
    surface.configure(device, config);
    Ok(())
}
