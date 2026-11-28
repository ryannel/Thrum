//! The renderer's pass registry: one typed key for each graph pass the
//! renderer declares, carrying its name, its timing bucket and the role of the
//! pipeline it records with. Graph declarations, recorder binding and timing
//! all read these keys, so no pass is found by its name or by a bare integer.
//!
//! The pipeline role numbers are kept as they were before the registry, so a
//! prepared pipeline's identity is unchanged; [`PipelineRole::value`] is the one
//! place they are written, and a test holds them free of collisions.

use std::borrow::Cow;

use crate::graph::PipelineKey;
use crate::timing::TimingCategory;

/// One atmosphere preparation stage. Its chart inputs are the table in
/// `atmosphere::graph`; its validity bit lives with the atmosphere's cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum AtmosphereStage {
    /// Transmittance chart.
    Transmittance,
    /// Multiple-scattering chart.
    MultipleScattering,
    /// Camera sky-view chart.
    SkyView,
    /// Classify the overlay and publish the governed camera haze scale.
    OverlayLighting,
    /// Diffuse irradiance into the environment light.
    Diffuse,
    /// Aerial perspective volumes.
    Aerial,
    /// Ground irradiance chart.
    Ground,
    /// Split-sum DFG chart.
    Dfg,
    /// One mip of the reflection cube.
    Reflection(u32),
}

/// Reflection cube mips the atmosphere prepares.
pub(crate) const REFLECTION_MIPS: u32 = 7;

impl AtmosphereStage {
    /// Every stage in graph declaration order: the transport charts, ground
    /// before sky view, then diffuse, aerial, DFG and each reflection mip.
    pub(crate) const ORDER: [Self; 15] = [
        Self::Transmittance,
        Self::MultipleScattering,
        Self::Ground,
        Self::SkyView,
        Self::OverlayLighting,
        Self::Diffuse,
        Self::Aerial,
        Self::Dfg,
        Self::Reflection(0),
        Self::Reflection(1),
        Self::Reflection(2),
        Self::Reflection(3),
        Self::Reflection(4),
        Self::Reflection(5),
        Self::Reflection(6),
    ];

    /// Stable diagnostic and timing name.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Transmittance => "atmosphere transmittance",
            Self::MultipleScattering => "atmosphere multiple scattering",
            Self::SkyView => "atmosphere sky view",
            Self::OverlayLighting => "atmosphere chart overlay lighting",
            Self::Diffuse => "atmosphere diffuse irradiance",
            Self::Aerial => "atmosphere aerial perspective",
            Self::Ground => "atmosphere ground irradiance",
            Self::Dfg => "atmosphere DFG integration",
            Self::Reflection(mip) => REFLECTION_NAMES[mip as usize],
        }
    }

    /// The timing bucket. Specular preparation shares the sky-view bucket.
    pub(crate) const fn timing(self) -> TimingCategory {
        match self {
            Self::Transmittance => TimingCategory::AtmosphereTransmittance,
            Self::MultipleScattering | Self::Ground => TimingCategory::AtmosphereMultipleGround,
            Self::SkyView | Self::Dfg | Self::Reflection(_) => TimingCategory::AtmosphereSky,
            Self::OverlayLighting | Self::Diffuse => TimingCategory::AtmosphereDiffuse,
            Self::Aerial => TimingCategory::AtmosphereAerial,
        }
    }
}

const REFLECTION_NAMES: [&str; REFLECTION_MIPS as usize] = [
    "atmosphere reflection mip 0",
    "atmosphere reflection mip 1",
    "atmosphere reflection mip 2",
    "atmosphere reflection mip 3",
    "atmosphere reflection mip 4",
    "atmosphere reflection mip 5",
    "atmosphere reflection mip 6",
];

/// One stage of a secondary scene view.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum SecondaryStage {
    /// The view's opaque draw.
    Opaque,
    /// Its local-light clusters.
    Clusters,
    /// Its aerial perspective volume.
    Aerial,
    /// One layer of its Sun cascades.
    Sun(u32),
    /// Its visibility compaction.
    Visibility,
    /// Its Sun caster cull.
    Casters,
}

impl SecondaryStage {
    /// Pipeline roles a view reserves, one per stage.
    pub(crate) const COUNT: u32 = 13;

    const fn offset(self) -> u32 {
        match self {
            Self::Opaque => 0,
            Self::Clusters => 1,
            Self::Aerial => 2,
            Self::Sun(0) => 3,
            Self::Sun(1) => 4,
            Self::Visibility => 5,
            Self::Casters => 6,
            Self::Sun(layer) => 5 + layer,
        }
    }

    const fn prefix(self) -> &'static str {
        match self {
            Self::Opaque => "secondary view opaque",
            Self::Clusters => "secondary view clusters",
            Self::Aerial => "secondary view aerial perspective",
            Self::Sun(_) => "secondary view Sun cascade",
            Self::Visibility => "secondary view visibility",
            Self::Casters => "secondary view Sun caster cull",
        }
    }
}

/// Identity of one prepared pipeline within a producer's closure. Each value is
/// written once, in [`Self::value`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum PipelineRole {
    /// The scene's forward opaque draw.
    Opaque,
    /// The 2D scene's sprite draw, in its own closure.
    Sprites,
    /// The interface sprite draw on the 8-bit display image.
    Interface,
    /// The display tone stage.
    Display,
    /// Near Sun cascade.
    ShadowNear,
    /// Far Sun cascade.
    ShadowFar,
    /// A coarser Sun cascade, by layer.
    SunCascade(u32),
    /// Local-light cluster build.
    Cluster,
    /// Transform history publication.
    History,
    /// Exposure resolve.
    Exposure,
    /// Sun horizon height map.
    HorizonHeights,
    /// Atmospheric Sun shadow heights.
    AirShadow,
    /// Star field.
    Stars,
    /// One atmosphere stage.
    Atmosphere(AtmosphereStage),
    /// The Sun caster cull.
    CasterCull,
    /// Camera visibility compaction: the frustum list or the occlusion prepass list.
    CameraVisibility,
    /// The final occlusion cull.
    FinalCull,
    /// The depth pyramid build.
    DepthPyramid,
    /// The occlusion depth prepass.
    DepthPrepass,
    /// Temporal exposure texture.
    TemporalExposure,
    /// Temporal input witness.
    TemporalWitness,
    /// Portable temporal upscaling.
    Taau,
    /// One stage of traced Sun visibility.
    SoftSun(u32),
    Ambient(u32),
    /// Raster positions used by the ambient trace.
    AmbientReceiver,
    /// The surface present blit.
    Surface,
    /// The scene contribution's compute pass.
    ContributionCompute,
    /// One contributed simulate pass.
    Effect(u32),
    /// One contributed pass after opaque.
    AfterOpaque(u32),
    /// One stage of one secondary view.
    Secondary {
        /// View index.
        view: u32,
        /// Stage of that view.
        stage: SecondaryStage,
    },
}

const EFFECT_BASE: u32 = 2000;
const AFTER_OPAQUE_BASE: u32 = 3000;
// Keep per-view ranges above the historical two-letter roles.
const SECONDARY_BASE: u32 = 0x1_0000;
const ATMOSPHERE_BASE: u32 = 10;

impl PipelineRole {
    /// The role number recorded in a [`PipelineKey`]. The two-letter values
    /// are the historical ASCII tags ("SC", "VI", "VJ", "VP", "DP", "T@",
    /// "TP", "PR"), kept so prepared pipeline identities do not change.
    pub(crate) const fn value(self) -> u32 {
        match self {
            Self::Opaque | Self::Sprites => 0,
            Self::Display => 1,
            Self::ShadowNear => 2,
            Self::ShadowFar => 3,
            Self::SunCascade(index) => 1200 + index,
            Self::Cluster => 4,
            Self::History => 5,
            Self::Exposure => 6,
            Self::HorizonHeights => 7,
            Self::AirShadow => 8,
            Self::Stars => 9,
            Self::Atmosphere(stage) => ATMOSPHERE_BASE + atmosphere_slot(stage),
            Self::ContributionCompute => 1000,
            Self::Interface => 1001,
            Self::Effect(index) => EFFECT_BASE + index,
            Self::AfterOpaque(index) => AFTER_OPAQUE_BASE + index,
            Self::Secondary { view, stage } => {
                SECONDARY_BASE + view * SecondaryStage::COUNT + stage.offset()
            }
            Self::DepthPrepass => 0x4450,
            Self::TemporalExposure => 0x5440,
            Self::TemporalWitness => 0x5441,
            Self::Taau => 0x5450,
            Self::SoftSun(stage) => 0x5355 + stage,
            Self::Ambient(stage) => 0x414f + stage,
            Self::AmbientReceiver => 0x4152,
            Self::Surface => 0x5052,
            Self::CasterCull => 0x5343,
            Self::CameraVisibility => 0x5649,
            Self::FinalCull => 0x564A,
            Self::DepthPyramid => 0x5650,
        }
    }

    /// This role's key for one device generation and producer revision.
    pub(crate) const fn pipeline_key(self, device_generation: u64, revision: u64) -> PipelineKey {
        PipelineKey {
            device_generation,
            revision,
            role: self.value(),
        }
    }

    /// `base` with this role.
    pub(crate) const fn key(self, base: PipelineKey) -> PipelineKey {
        PipelineKey {
            role: self.value(),
            ..base
        }
    }
}

// The atmosphere's original stage numbering: the three transport charts, then
// diffuse, aerial, ground, DFG and the reflection mips.
const fn atmosphere_slot(stage: AtmosphereStage) -> u32 {
    match stage {
        AtmosphereStage::Transmittance => 0,
        AtmosphereStage::MultipleScattering => 1,
        AtmosphereStage::SkyView => 2,
        AtmosphereStage::Diffuse => 3,
        AtmosphereStage::Aerial => 4,
        AtmosphereStage::Ground => 5,
        AtmosphereStage::Dfg => 6,
        AtmosphereStage::Reflection(mip) => 7 + mip,
        AtmosphereStage::OverlayLighting => 7 + REFLECTION_MIPS,
    }
}

/// A contributed pass's or secondary view's position as a key index. Admission
/// bounds both far below `u32::MAX`; a larger count saturates, as role
/// arithmetic did before the registry.
pub(crate) fn pass_index(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// One graph pass the renderer declares.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub(crate) enum PassKey {
    /// Forward opaque draw.
    Opaque,
    /// Display tone encode.
    Display,
    /// Display-referred tone encode of the 2D scene.
    DisplayReferred,
    /// Near Sun cascade.
    ShadowNear,
    /// Far Sun cascade.
    ShadowFar,
    /// A coarser Sun cascade, by layer.
    SunCascade(u32),
    /// Sun horizon height map.
    HorizonHeights,
    /// Atmospheric Sun shadow heights.
    AirShadow,
    /// The Sun caster cull.
    CasterCull,
    /// Local-light cluster build.
    Cluster,
    /// Transform history publication.
    History,
    /// Exposure resolve.
    Exposure,
    /// Star field.
    Stars,
    /// One atmosphere stage.
    Atmosphere(AtmosphereStage),
    /// Frustum camera visibility.
    CameraVisibility,
    /// Occlusion prepass list.
    PrepassList,
    /// Occlusion depth prepass.
    DepthPrepass,
    /// Depth pyramid build.
    DepthPyramid,
    /// Final occlusion cull.
    FinalCull,
    /// Camera culling count copy.
    CullingStats,
    /// Temporal exposure texture.
    TemporalExposure,
    /// Temporal input witness.
    TemporalWitness,
    /// Portable temporal upscaling.
    Taau,
    /// MetalFX and its prepared TAAU twin.
    MetalFx,
    /// One stage of traced Sun visibility.
    SoftSun(crate::synthetic::SoftSunStage),
    CombinedVisibility,
    AmbientReceiver,
    Ambient(crate::synthetic::AmbientStage),
    /// The scene contribution's upload stage.
    ContributionUpload,
    /// The scene contribution's compute pass.
    ContributionCompute,
    /// One contributed simulate pass; its name comes from the contribution.
    Effect(u32),
    /// One contributed pass after opaque; its name comes from the contribution.
    AfterOpaque(u32),
    /// The refraction copy before after-opaque effects.
    AfterOpaqueCopy,
    /// One stage of one secondary view.
    Secondary {
        /// View index.
        view: u32,
        /// Stage of that view.
        stage: SecondaryStage,
    },
    /// Ray structure build.
    RayStructureBuild,
    /// Prepared provider work before its whole native encoder.
    NativeProviderBefore(&'static str),
    /// A native provider and its prepared twin, identified by their graph name.
    NativeProvider(&'static str),
    /// The 2D scene's sprite draw.
    Sprites,
    Interface,
    /// Output readback copy.
    Readback,
    /// The surface present blit.
    Surface,
}

const SUN_CASCADE_NAMES: [&str; 8] = [
    "synthetic Sun cascade 2",
    "synthetic Sun cascade 3",
    "synthetic Sun cascade 4",
    "synthetic Sun cascade 5",
    "synthetic Sun cascade 6",
    "synthetic Sun cascade 7",
    "synthetic Sun cascade 8",
    "synthetic Sun cascade 9",
];

const SUN_OUTLINE_NAMES: [&str; 30] = [
    "synthetic Sun cascade 0 outline 1",
    "synthetic Sun cascade 0 outline 2",
    "synthetic Sun cascade 0 outline 3",
    "synthetic Sun cascade 1 outline 1",
    "synthetic Sun cascade 1 outline 2",
    "synthetic Sun cascade 1 outline 3",
    "synthetic Sun cascade 2 outline 1",
    "synthetic Sun cascade 2 outline 2",
    "synthetic Sun cascade 2 outline 3",
    "synthetic Sun cascade 3 outline 1",
    "synthetic Sun cascade 3 outline 2",
    "synthetic Sun cascade 3 outline 3",
    "synthetic Sun cascade 4 outline 1",
    "synthetic Sun cascade 4 outline 2",
    "synthetic Sun cascade 4 outline 3",
    "synthetic Sun cascade 5 outline 1",
    "synthetic Sun cascade 5 outline 2",
    "synthetic Sun cascade 5 outline 3",
    "synthetic Sun cascade 6 outline 1",
    "synthetic Sun cascade 6 outline 2",
    "synthetic Sun cascade 6 outline 3",
    "synthetic Sun cascade 7 outline 1",
    "synthetic Sun cascade 7 outline 2",
    "synthetic Sun cascade 7 outline 3",
    "synthetic Sun cascade 8 outline 1",
    "synthetic Sun cascade 8 outline 2",
    "synthetic Sun cascade 8 outline 3",
    "synthetic Sun cascade 9 outline 1",
    "synthetic Sun cascade 9 outline 2",
    "synthetic Sun cascade 9 outline 3",
];

impl PassKey {
    pub(crate) const fn sun_cascade(index: u32) -> Self {
        match index {
            0 => Self::ShadowNear,
            1 => Self::ShadowFar,
            _ => Self::SunCascade(index),
        }
    }

    pub(crate) const fn shadow_layer(self) -> Option<u32> {
        match self {
            Self::ShadowNear => Some(0),
            Self::ShadowFar => Some(1),
            Self::SunCascade(index) => Some(index),
            Self::HorizonHeights => Some(crate::shader_constants::SHADOW_HEIGHT_LAYER),
            _ => None,
        }
    }

    /// Stable diagnostic and timing name. Contributed passes are named by their
    /// contribution and return `None`; declare them with that name.
    pub(crate) fn name(self) -> Option<Cow<'static, str>> {
        if let Self::Secondary { view, stage } = self {
            return Some(Cow::Owned(match stage {
                SecondaryStage::Sun(layer) => format!("{} {view} {layer}", stage.prefix()),
                _ => format!("{} {view}", stage.prefix()),
            }));
        }
        self.fixed_name().map(Cow::Borrowed)
    }

    /// The name of a pass that has one name in every graph.
    pub(crate) const fn fixed_name(self) -> Option<&'static str> {
        Some(match self {
            Self::Opaque => "synthetic forward opaque",
            Self::Display => "synthetic tone encode",
            Self::DisplayReferred => "display-referred tone encode",
            Self::ShadowNear => "synthetic Sun shadow near",
            Self::ShadowFar => "synthetic Sun shadow far",
            Self::SunCascade(index) if index >= crate::shader_constants::SHADOW_MAP_LAYERS => {
                SUN_OUTLINE_NAMES[(index - crate::shader_constants::SHADOW_MAP_LAYERS) as usize]
            }
            Self::SunCascade(index) => SUN_CASCADE_NAMES[index as usize - 2],
            Self::HorizonHeights => "synthetic Sun horizon heights",
            Self::AirShadow => "synthetic Sun air shadow heights",
            Self::CasterCull => "synthetic Sun caster cull",
            Self::Cluster => "synthetic cluster build",
            Self::History => "synthetic transform history",
            Self::Exposure => "synthetic exposure resolve",
            Self::Stars => "atmosphere stars",
            Self::Atmosphere(stage) => stage.name(),
            Self::CameraVisibility => "camera visibility",
            Self::PrepassList => "camera prepass list",
            Self::DepthPrepass => "camera depth prepass",
            Self::DepthPyramid => "camera depth pyramid",
            Self::FinalCull => "camera final cull",
            Self::CullingStats => "camera culling stats",
            Self::TemporalExposure => "temporal exposure texture",
            Self::TemporalWitness => "temporal input witness",
            Self::Taau => "portable TAAU",
            Self::MetalFx => "MetalFX temporal",
            Self::SoftSun(stage) => stage.name(),
            Self::CombinedVisibility => crate::synthetic::COMBINED_TRACE,
            Self::Ambient(stage) => stage.name(),
            Self::AmbientReceiver => "ambient receiver raster",
            Self::ContributionUpload => "contribution upload",
            Self::ContributionCompute => "contribution compute",
            Self::AfterOpaqueCopy => "effect refraction copy",
            Self::RayStructureBuild => "ray structure build",
            Self::NativeProviderBefore(name) | Self::NativeProvider(name) => name,
            Self::Sprites => "sprites",
            Self::Interface => "interface stage",
            Self::Readback => "output readback",
            Self::Surface => "V1 surface present",
            Self::Effect(_) | Self::AfterOpaque(_) | Self::Secondary { .. } => return None,
        })
    }

    /// The fixed name, for the modules that export it as a constant.
    ///
    /// # Panics
    ///
    /// Panics, at compile time where used in a constant, for an indexed or
    /// contributed key.
    pub(crate) const fn constant_name(self) -> &'static str {
        match self.fixed_name() {
            Some(name) => name,
            None => panic!("an indexed or contributed pass has no fixed name"),
        }
    }

    /// The bucket this pass's GPU time lands in.
    pub(crate) const fn timing(self) -> TimingCategory {
        match self {
            Self::ShadowNear | Self::CasterCull => TimingCategory::ShadowNear,
            Self::ShadowFar | Self::SunCascade(_) | Self::HorizonHeights | Self::AirShadow => {
                TimingCategory::ShadowFar
            }
            Self::Cluster => TimingCategory::Clusters,
            Self::Opaque
            | Self::Stars
            | Self::ContributionCompute
            | Self::CameraVisibility
            | Self::PrepassList
            | Self::DepthPrepass
            | Self::DepthPyramid
            | Self::FinalCull => TimingCategory::Forward,
            Self::History => TimingCategory::History,
            Self::Exposure | Self::Display | Self::DisplayReferred => TimingCategory::Tone,
            Self::Atmosphere(stage) => stage.timing(),
            Self::TemporalExposure | Self::TemporalWitness | Self::Taau | Self::MetalFx => {
                TimingCategory::Temporal
            }
            Self::Effect(_)
            | Self::AfterOpaque(_)
            | Self::SoftSun(_)
            | Self::Ambient(_)
            | Self::AmbientReceiver
            | Self::CombinedVisibility => TimingCategory::Effects,
            Self::Secondary { .. } => TimingCategory::SecondaryViews,
            Self::Sprites | Self::Interface => TimingCategory::Sprites,
            Self::Surface => TimingCategory::Surface,
            Self::CullingStats
            | Self::ContributionUpload
            | Self::AfterOpaqueCopy
            | Self::RayStructureBuild
            | Self::NativeProviderBefore(_)
            | Self::NativeProvider(_)
            | Self::Readback => TimingCategory::Transfer,
        }
    }

    /// The prepared pipeline this pass records with; copies, native segments
    /// and acceleration builds have none.
    pub(crate) const fn pipeline_role(self) -> Option<PipelineRole> {
        Some(match self {
            Self::Opaque => PipelineRole::Opaque,
            Self::Sprites => PipelineRole::Sprites,
            Self::Interface => PipelineRole::Interface,
            Self::Display | Self::DisplayReferred => PipelineRole::Display,
            Self::ShadowNear => PipelineRole::ShadowNear,
            Self::ShadowFar => PipelineRole::ShadowFar,
            Self::SunCascade(index) => PipelineRole::SunCascade(index),
            Self::HorizonHeights => PipelineRole::HorizonHeights,
            Self::AirShadow => PipelineRole::AirShadow,
            Self::CasterCull => PipelineRole::CasterCull,
            Self::Cluster => PipelineRole::Cluster,
            Self::History => PipelineRole::History,
            Self::Exposure => PipelineRole::Exposure,
            Self::Stars => PipelineRole::Stars,
            Self::Atmosphere(stage) => PipelineRole::Atmosphere(stage),
            Self::CameraVisibility | Self::PrepassList => PipelineRole::CameraVisibility,
            Self::DepthPrepass => PipelineRole::DepthPrepass,
            Self::DepthPyramid => PipelineRole::DepthPyramid,
            Self::FinalCull => PipelineRole::FinalCull,
            Self::TemporalExposure => PipelineRole::TemporalExposure,
            Self::TemporalWitness => PipelineRole::TemporalWitness,
            Self::Taau | Self::MetalFx => PipelineRole::Taau,
            Self::SoftSun(stage) => PipelineRole::SoftSun(stage as u32),
            Self::CombinedVisibility => PipelineRole::SoftSun(0),
            Self::Ambient(stage) => PipelineRole::Ambient(stage as u32),
            Self::AmbientReceiver => PipelineRole::AmbientReceiver,
            Self::ContributionCompute => PipelineRole::ContributionCompute,
            Self::Effect(index) => PipelineRole::Effect(index),
            Self::AfterOpaque(index) => PipelineRole::AfterOpaque(index),
            Self::Secondary { view, stage } => PipelineRole::Secondary { view, stage },
            Self::Surface => PipelineRole::Surface,
            Self::CullingStats
            | Self::ContributionUpload
            | Self::AfterOpaqueCopy
            | Self::RayStructureBuild
            | Self::NativeProviderBefore(_)
            | Self::NativeProvider(_)
            | Self::Readback => return None,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
