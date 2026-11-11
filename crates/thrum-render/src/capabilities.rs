//! Immutable facts captured once by the platform for one adapter generation.
//! Render preparation consumes this record; pass recording never probes a device.

use crate::runtime::DeviceSite;
mod recipe;
pub use recipe::{Recipe, RecipeError};
#[cfg(any(target_os = "macos", target_os = "ios"))]
pub(crate) mod apple;

/// Enabled GPU capabilities and identity for one device generation.
#[derive(Clone, Debug)]
pub struct DeviceCapabilities {
    /// The generation's pipeline creation authority, minted once here and
    /// shared by every clone of this record.
    site: DeviceSite,
    generation: u64,
    adapter: wgpu::AdapterInfo,
    features: wgpu::Features,
    limits: wgpu::Limits,
    downlevel: Option<wgpu::DownlevelFlags>,
    unified_memory: Option<bool>,
    recipe: Result<Recipe, RecipeError>,
}

impl DeviceCapabilities {
    /// Optional timestamp feature to request while the platform creates its
    /// device. The enabled result is captured in this record afterward.
    #[must_use]
    pub fn requested_timing_features(adapter: &wgpu::Adapter) -> wgpu::Features {
        adapter.features()
            & (wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS)
    }
    /// Compressed-texture families the adapter offers, so cooked content can use its native
    /// profile (ASTC on Apple GPUs, BC on desktop) instead of the uncompressed portable one.
    #[must_use]
    pub fn requested_content_features(adapter: &wgpu::Adapter) -> wgpu::Features {
        adapter.features()
            & (wgpu::Features::TEXTURE_COMPRESSION_ASTC
                | wgpu::Features::TEXTURE_COMPRESSION_BC
                | wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D)
    }
    /// Capture the adapter and enabled device at platform creation. This mints
    /// the generation's [`DeviceSite`]; capture each generation once.
    #[must_use]
    pub fn capture(adapter: &wgpu::Adapter, device: &wgpu::Device, generation: u64) -> Self {
        let site = DeviceSite::mint(device, generation);
        let mut record = Self::from_info(
            site,
            generation,
            adapter.get_info(),
            Some(adapter.get_downlevel_capabilities().flags),
        );
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            record.unified_memory = apple::unified_memory(adapter);
        }
        record.resolve_environment();
        record
    }

    /// Capture from a device when the adapter handle is no longer retained.
    /// The live platform should pass its adapter directly to `capture`. The
    /// record's site is [untracked](DeviceSite::untracked): re-capturing a raw
    /// handle never reaches, or forges, a generation's ledger.
    #[must_use]
    pub fn capture_device(device: &wgpu::Device, generation: u64) -> Self {
        let site = DeviceSite::untracked(device.clone());
        let mut record = Self::from_info(site, generation, device.adapter_info(), None);
        record.resolve_environment();
        record
    }

    fn from_info(
        site: DeviceSite,
        generation: u64,
        adapter: wgpu::AdapterInfo,
        downlevel: Option<wgpu::DownlevelFlags>,
    ) -> Self {
        let device: &wgpu::Device = &site;
        Self {
            downlevel,
            unified_memory: None,
            recipe: Ok(Recipe::Portable),
            features: device.features(),
            limits: device.limits(),
            site,
            generation,
            adapter,
        }
    }

    fn resolve_environment(&mut self) {
        #[cfg(target_arch = "wasm32")]
        let requested: Option<String> = None;
        #[cfg(not(target_arch = "wasm32"))]
        let requested = match std::env::var("THRUM_RECIPE") {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => {
                self.recipe = Err(RecipeError::InvalidOverride);
                return;
            }
        };
        self.recipe = self.resolve_override(requested.as_deref());
    }

    fn resolve_override(&self, requested: Option<&str>) -> Result<Recipe, RecipeError> {
        recipe::resolve(
            self.adapter.backend,
            self.unified_memory,
            self.adapter.transient_saves_memory,
            requested,
        )
    }

    /// Select an explicit diagnostic recipe before constructing any targets.
    /// A whole recipe change requires preparing and publishing a new generation.
    /// # Errors
    /// Refuses invalid requests and Apple selection with unsupported/unknown facts.
    pub fn with_recipe_override(mut self, requested: &str) -> Result<Self, RecipeError> {
        self.recipe = Ok(self.resolve_override(Some(requested))?);
        Ok(self)
    }

    /// Resolved recipe, or a rejected diagnostic request. Preparation checks this
    /// before allocating GPU resources.
    /// # Errors
    /// Returns the invalid or unsupported request captured at platform creation.
    pub fn recipe(&self) -> Result<Recipe, RecipeError> {
        self.recipe
    }

    /// Actual native unified-memory fact, unknown without a retained native adapter.
    #[must_use]
    pub fn unified_memory(&self) -> Option<bool> {
        self.unified_memory
    }

    /// Stable diagnostic digest of this generation's identity and enabled facts.
    /// Offered features and platform display state are deliberately excluded.
    #[must_use]
    pub fn registry_digest(&self) -> String {
        let identity = format!(
            "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
            self.adapter,
            self.features,
            self.limits,
            self.downlevel,
            self.unified_memory,
            self.recipe,
            self.generation
        );
        let hash = identity
            .bytes()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3)
            });
        format!("{hash:016x}")
    }

    /// Complete driver-reported Metal resource residency, including shared assets,
    /// buffers, prepared variants and any simultaneously retained generations.
    /// An explicit diagnostic read, outside pass recording. Other backends return unknown.
    #[must_use]
    pub fn resident_device_bytes(&self) -> Option<u64> {
        if self.adapter.backend != wgpu::Backend::Metal {
            return None;
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            apple::resident_bytes(&self.site)
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            None
        }
    }

    /// Device generation used by pipelines and submissions.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Adapter identity and driver facts for recipe selection and cache keys.
    #[must_use]
    pub fn adapter(&self) -> &wgpu::AdapterInfo {
        &self.adapter
    }

    /// Features enabled on this device, rather than merely offered by the adapter.
    #[must_use]
    pub fn features(&self) -> wgpu::Features {
        self.features
    }

    /// Adapter-read execution facts, unknown when no adapter was retained.
    #[must_use]
    pub fn downlevel(&self) -> Option<wgpu::DownlevelFlags> {
        self.downlevel
    }

    /// Explicit diagnostic mask. Only removes support; cannot invent adapter facts.
    #[must_use]
    pub fn without_indirect_execution(mut self) -> Self {
        if let Some(flags) = &mut self.downlevel {
            flags.remove(wgpu::DownlevelFlags::INDIRECT_EXECUTION);
        }
        self
    }

    /// Limits enabled on this device.
    #[must_use]
    pub fn limits(&self) -> &wgpu::Limits {
        &self.limits
    }

    /// Whether this record describes the supplied device handle.
    #[must_use]
    pub fn matches(&self, device: &wgpu::Device) -> bool {
        *self.site == *device
    }

    /// The generation's pipeline creation authority.
    #[must_use]
    pub fn site(&self) -> &DeviceSite {
        &self.site
    }

    pub(crate) fn device(&self) -> &DeviceSite {
        &self.site
    }
}
