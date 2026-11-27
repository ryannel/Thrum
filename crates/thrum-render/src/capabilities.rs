//! Immutable facts captured once by the platform for one adapter generation.
//! Render preparation consumes this record; pass recording never probes a device.

use crate::runtime::DeviceSite;
#[cfg(target_os = "macos")]
pub mod metallib;

mod device;
mod hardware;
mod recipe;
#[cfg(target_os = "macos")]
pub use apple::drawable_extent;
pub use device::{
    DeviceProfile, DeviceRecipe, DeviceRestriction, DeviceWatch, FIRST_DEVICE_GENERATION,
    OpenedDevice, PREFERRED_SDR_FORMAT, SurfaceConfigError, configure_surface, sdr_surface_config,
    sdr_surface_format,
};
#[cfg(not(target_arch = "wasm32"))]
pub use device::{configure_surface_once, retry_surface_configuration};
pub use hardware::{HardwareFeature, HardwareLaunchError, HardwareRequest, HardwareSelection};
pub use recipe::{Recipe, RecipeError};

/// Generation-selected temporal provider. Portable TAAU is the admitted default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reconstructor {
    /// Shader-based temporal reconstruction, available on every recipe.
    Taau,
    /// MetalFX temporal scaling on the Apple recipe.
    MetalFx,
}

/// The outcome of resolving a scene's request for a ray structure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RayStructureSelection {
    /// The scene asked for no structure.
    NotRequested,
    /// Ray queries are enabled and the recipe admits the structure.
    Enabled,
    /// The device generation did not enable ray queries.
    MissingRayQuery,
    /// The resolved recipe does not support ray effects.
    UnsupportedRecipe,
    /// A scene contribution does not support candidate preparation.
    UnsupportedContribution,
    /// The scene could not prepare its resources and fell back.
    PreparationFailed,
}

/// The requested or resolved method of surface Sun visibility.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SunShadowMethod {
    /// Raster cascades and caster heights.
    #[default]
    Maps,
    /// Hard centre ray on the main view.
    Traced,
}

/// Requested and admitted traced ambient occlusion for one scene preparation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AmbientOcclusionSelection {
    /// Maximum distance requested, in metres.
    pub requested_radius: Option<f32>,
    /// Admitted maximum distance, in metres. The runtime extent knob can disable tracing.
    pub radius: Option<f32>,
    /// Reason a requested effect could not prepare.
    pub fallback: Option<RayStructureSelection>,
}

impl AmbientOcclusionSelection {
    pub(crate) fn resolve(requested_radius: Option<f32>, admission: RayStructureSelection) -> Self {
        let admitted = admission == RayStructureSelection::Enabled;
        Self {
            requested_radius,
            radius: requested_radius.filter(|_| admitted),
            fallback: (!admitted && requested_radius.is_some()).then_some(admission),
        }
    }
}

/// Selection of the scene's direct-light shadow methods for one preparation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShadowSelection {
    /// Requested Sun method.
    pub requested_sun: SunShadowMethod,
    /// Resolved Sun method.
    pub sun: SunShadowMethod,
    /// Requested lamp shadows.
    pub requested_lamps: bool,
    /// Whether the scene traces lamp shadows.
    pub lamps: bool,
    /// Shared refusal reason for any requested traced methods.
    pub fallback: Option<RayStructureSelection>,
}

impl ShadowSelection {
    pub(crate) fn resolve(
        sun: SunShadowMethod,
        lamps: bool,
        admission: RayStructureSelection,
    ) -> Self {
        let admitted = admission == RayStructureSelection::Enabled;
        Self {
            requested_sun: sun,
            sun: if admitted { sun } else { SunShadowMethod::Maps },
            requested_lamps: lamps,
            lamps: admitted && lamps,
            fallback: (!admitted && (sun == SunShadowMethod::Traced || lamps)).then_some(admission),
        }
    }
}
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
    reconstructor: Reconstructor,
    hardware: Vec<HardwareSelection>,
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    pub(crate) observe_ray_preparation: bool,
    #[cfg(feature = "creation-counters")]
    pub(crate) fail_traced_pipeline: bool,
    #[cfg(feature = "creation-counters")]
    pub(crate) fail_traced_pipeline_internal: bool,
}

impl DeviceCapabilities {
    pub(crate) fn no_reconstructor(
        requested: crate::synthetic::Reconstruction,
    ) -> Result<(), crate::runtime::FrameError> {
        use crate::synthetic::Reconstruction;
        if requested == Reconstruction::Off {
            Ok(())
        } else {
            Err(crate::runtime::FrameError::SpriteReconstruction(requested))
        }
    }

    /// Resolve the provider once during preparation. Native providers require
    /// separate qualification before they can replace this portable default.
    #[must_use]
    pub fn reconstructor(&self) -> Reconstructor {
        self.reconstructor
    }
    /// Registry default for fullscreen output on fast Mac displays, shared by both recipes.
    #[cfg(target_os = "macos")]
    #[must_use]
    pub const fn mac_fullscreen_output_scale() -> crate::synthetic::RenderScale {
        crate::synthetic::RenderScale::Native
    }

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

    /// Select the build's offline stage libraries and preparation mode for this Metal generation.
    /// Call before sharing the capabilities with the host or creating pipelines through its site.
    #[cfg(target_os = "macos")]
    pub fn set_metal_preparation(&mut self, preparation: Option<metallib::Preparation>) {
        if self.adapter.backend == wgpu::Backend::Metal {
            let admitted = self.hardware.iter().any(|selection| {
                selection.request.feature == HardwareFeature::MetalShaderPassthrough
                    && selection.declined.is_none()
            }) && self.features.contains(wgpu::Features::PASSTHROUGH_SHADERS);
            let archive_admitted = self.hardware.iter().any(|selection| {
                selection.request.feature == HardwareFeature::MetalPipelineArchive
                    && selection.declined.is_none()
            }) && self.features.contains(wgpu::Features::PIPELINE_CACHE);
            let preparation = preparation.and_then(|config| {
                config.attach(
                    &self.site,
                    admitted,
                    archive_admitted,
                    self.generation == FIRST_DEVICE_GENERATION,
                )
            });
            self.site.set_metal_preparation(preparation);
        }
    }

    /// Serialize an explicitly harvested archive at the end of preparation.
    ///
    /// # Errors
    /// The cache could not be serialized or the new file could not be written.
    #[cfg(target_os = "macos")]
    pub fn save_metal_archive(&self) -> std::io::Result<()> {
        self.site
            .metal_preparation()
            .map_or(Ok(()), metallib::PipelinePreparation::save_archive)
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
            reconstructor: Reconstructor::Taau,
            features: device.features(),
            limits: device.limits(),
            site,
            generation,
            adapter,
            hardware: Vec::new(),
            #[cfg(all(feature = "creation-counters", target_os = "macos"))]
            observe_ray_preparation: false,
            #[cfg(feature = "creation-counters")]
            fail_traced_pipeline: false,
            #[cfg(feature = "creation-counters")]
            fail_traced_pipeline_internal: false,
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

    pub(crate) fn resolve_ray_structure(&self, requested: bool) -> RayStructureSelection {
        if !requested {
            RayStructureSelection::NotRequested
        } else if self.recipe != Ok(Recipe::AppleTile) {
            RayStructureSelection::UnsupportedRecipe
        } else if !self
            .features
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        {
            RayStructureSelection::MissingRayQuery
        } else {
            RayStructureSelection::Enabled
        }
    }

    /// Inject a shader validation failure after allocating the traced candidate's structure.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn with_traced_pipeline_failure(mut self) -> Self {
        self.fail_traced_pipeline = true;
        self
    }

    /// Supply a native-link Internal error at the traced pipeline scope boundary.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn with_traced_pipeline_internal_failure(mut self) -> Self {
        self.fail_traced_pipeline_internal = true;
        self
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

    /// Launch requests and their disposition on this generation.
    #[must_use]
    pub fn hardware(&self) -> &[HardwareSelection] {
        &self.hardware
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

    /// Time initial mesh builds for ray-demanding scenes prepared with this
    /// diagnostic copy. The generation's device and capabilities are unchanged.
    #[cfg(all(feature = "creation-counters", target_os = "macos"))]
    #[must_use]
    pub fn with_ray_preparation_observation(mut self) -> Self {
        self.observe_ray_preparation = true;
        self
    }

    /// Format guarantees derived from this generation's enabled features. The
    /// engine uses the portable format contract, even when an adapter offers more.
    #[must_use]
    pub fn texture_format(&self, format: wgpu::TextureFormat) -> wgpu::TextureFormatFeatures {
        format.guaranteed_format_features(self.features)
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
