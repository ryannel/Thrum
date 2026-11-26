//! Renderer-owned interpretation of the bounded scene timestamp layout.

use super::{CullMode, OffscreenTarget, PreparedSynthetic};
use crate::pass_key::{AtmosphereStage, PassKey};
use crate::timing::TimingCategory;
use std::sync::Arc;

impl OffscreenTarget {
    pub(crate) fn timing_metadata_capacity(&self) -> usize {
        self.graph(false)
            .plan()
            .pass_order
            .len()
            .max(self.graph(true).plan().pass_order.len())
            .max(self.effects.names.len())
    }

    pub(crate) fn effect_pass_names(&self) -> impl Iterator<Item = Arc<str>> + '_ {
        self.effects.names.iter().cloned()
    }
    pub(crate) fn untimed_pass_names(
        &self,
        readback: bool,
        presented: bool,
    ) -> impl Iterator<Item = Arc<str>> + '_ {
        let plan = self.graph(readback).plan();
        let executed = if presented {
            usize::MAX
        } else {
            plan.pre_acquire_passes.unwrap_or(usize::MAX)
        };
        plan.pass_bindings()
            .take(executed)
            .filter(|(_, pass)| {
                matches!(
                    pass.kind,
                    Some(
                        crate::graph::PassKind::Copy
                            | crate::graph::PassKind::Native
                            | crate::graph::PassKind::AccelerationBuild
                    )
                )
            })
            .map(|(_, pass)| Arc::clone(&pass.name))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SceneTimingLayout {
    shadows: Shadows,
    cascades: bool,
    clusters: bool,
    transforms: bool,
    sdr_tone: Option<PassKey>,
    aerial: AerialPass,
    star_pass: Option<&'static str>,
    /// Bit `index` of [`CULLING_PASSES`] is set for each camera culling pass
    /// the frame records exactly once. Frustum: one `camera visibility`
    /// compute pass, timed even when the active prefix is empty. Occlusion:
    /// the prepass list, the depth prepass, the pyramid and the final cull.
    culling: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AerialPass {
    Chart,
    ChartBeforeOverlay,
    ViewVolume,
}

/// Which Sun shadow passes a frame records.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shadows {
    Off,
    /// Every admitted caster in every Sun pass.
    Unculled,
    /// The Sun caster cull runs first: timed once and counted in the near
    /// shadow bucket.
    Culled,
}

/// The camera culling passes `mode` records.
const fn culling_passes(mode: CullMode) -> u8 {
    match mode {
        CullMode::Direct => 0,
        CullMode::Frustum => 0b00001,
        CullMode::Occlusion => 0b11110,
    }
}

impl From<&PreparedSynthetic> for SceneTimingLayout {
    fn from(prepared: &PreparedSynthetic) -> Self {
        Self {
            shadows: match (prepared.shadowed(), prepared.caster_culling.is_some()) {
                (false, _) => Shadows::Off,
                (true, false) => Shadows::Unculled,
                (true, true) => Shadows::Culled,
            },
            cascades: prepared.shadowed() && prepared.surface_uses_cascades(),
            clusters: prepared.gpu_clustered(),
            transforms: prepared.active_instance_count() != 0,
            sdr_tone: Some(PassKey::Display),
            aerial: if prepared
                .atmosphere
                .as_ref()
                .is_some_and(|air| air.view_volume.is_some())
            {
                AerialPass::ViewVolume
            } else if prepared
                .atmosphere
                .as_ref()
                .is_some_and(crate::atmosphere::AtmospherePrepared::has_overlay)
            {
                AerialPass::ChartBeforeOverlay
            } else {
                AerialPass::Chart
            },
            star_pass: None,
            culling: culling_passes(prepared.camera_culling()),
        }
    }
}

impl SceneTimingLayout {
    pub(crate) fn for_target(mut self, target: &OffscreenTarget, readback: bool) -> Self {
        self.star_pass = target.draw_stars.then_some(super::stars::PASS_NAME);
        self.sdr_tone = target
            .graph(readback)
            .plan()
            .pass_bindings()
            .find_map(|(_, pass)| pass.key.filter(|&key| key == PassKey::Display));
        self
    }
}

/// Every camera culling pass name, in any layout.
pub(crate) const CULLING_PASSES: [&str; 5] = [
    crate::visibility::PASS,
    crate::visibility::PREPASS_LIST_PASS,
    super::camera_culling::DEPTH_PREPASS_PASS,
    crate::visibility::PYRAMID_PASS,
    crate::visibility::FINAL_PASS,
];

pub(crate) fn validate_pass_timings(
    passes: &[crate::timing::PassTiming],
    presented: bool,
    layout: SceneTimingLayout,
) -> Option<(Option<f64>, Option<f64>)> {
    // The fixed scene sequence, in recording order.
    const ALL: [PassKey; 8] = [
        PassKey::ShadowNear,
        PassKey::ShadowFar,
        PassKey::HorizonHeights,
        PassKey::Cluster,
        PassKey::Exposure,
        PassKey::Opaque,
        PassKey::History,
        PassKey::Display,
    ];
    let mut names = ALL
        .into_iter()
        .filter(|&key| match key {
            PassKey::ShadowNear | PassKey::ShadowFar => layout.cascades,
            PassKey::HorizonHeights => layout.shadows != Shadows::Off,
            PassKey::Cluster => layout.clusters,
            PassKey::History => layout.transforms,
            PassKey::Display => layout.sdr_tone == Some(key),
            _ => true,
        })
        .map(PassKey::constant_name);
    // A scene contribution adds one timed compute pass that the graph may order anywhere before
    // forward; it is reported on its own and the fixed scene sequence is matched without it.
    // The Sun air shadow heights pass, present with shadows under atmosphere lighting, is likewise
    // matched by name and shares the far shadow bucket with the horizon height map.
    // Camera culling passes are opaque-phase work around forward's indirect draws: matched by
    // name, each required exactly once when the slot layout uses them, and counted in forward.
    let culling = layout.culling;
    let expects_volume = layout.aerial == AerialPass::ViewVolume;
    let mut contribution = None;
    let mut air_shadow = None;
    let mut view_volume = false;
    let mut culled = 0_u8;
    let mut stars = None;
    let mut caster_cull = None;
    let mut stage_order = AtmosphereStage::ORDER;
    if layout.aerial == AerialPass::ChartBeforeOverlay {
        // The overlay is traced after the clear charts. Diffuse and reflection
        // wait for it, while the earlier aerial chart still contains clear air.
        let index = |stage| {
            stage_order
                .iter()
                .position(|&entry| entry == stage)
                .expect("atmosphere stage")
        };
        let (aerial, diffuse) = (
            index(AtmosphereStage::Aerial),
            index(AtmosphereStage::Diffuse),
        );
        stage_order.swap(aerial, diffuse);
    }
    let mut stages = Some(stage_order.into_iter());
    let mut surface = None;
    for pass in passes {
        if pass.category == TimingCategory::Temporal
            || pass.name.as_ref() == crate::sprite::interface::PASS
        {
            continue;
        }
        if matches!(
            pass.category,
            TimingCategory::Effects | TimingCategory::SecondaryViews
        ) {
            // These are prepared contribution passes. Keep their original names in
            // SceneGpuTimings below; they do not belong to the fixed scene sequence.
        } else if pass.name.as_ref() == super::AIR_SHADOW_PASS {
            if air_shadow.is_some() || layout.shadows == Shadows::Off || expects_volume {
                return None;
            }
            air_shadow = Some(pass.nanoseconds);
        } else if expects_volume && pass.name.as_ref() == AtmosphereStage::Aerial.name() {
            // The joint volume reads the Sun cascades, so its aerial work is
            // recorded after them instead of among the prefix charts.
            if view_volume {
                return None;
            }
            view_volume = true;
        } else if pass.name.as_ref() == super::stars::PASS_NAME {
            if stars.is_some() || layout.star_pass != Some(pass.name.as_ref()) {
                return None;
            }
            stars = Some(pass.nanoseconds);
        } else if let Some(index) = CULLING_PASSES
            .iter()
            .position(|&name| name == pass.name.as_ref())
        {
            let bit = 1 << index;
            if culling & bit == 0 || culled & bit != 0 {
                return None;
            }
            culled |= bit;
        } else if pass.name.as_ref() == crate::visibility::casters::PASS {
            if caster_cull.is_some() || layout.shadows != Shadows::Culled {
                return None;
            }
            caster_cull = Some(pass.nanoseconds);
        } else if pass.name.as_ref() == super::CONTRIBUTION_COMPUTE {
            if contribution.is_some() {
                return None;
            }
            contribution = Some(pass.nanoseconds);
        } else {
            if let Some(stages) = &mut stages
                && stages.any(|stage| stage.name() == pass.name.as_ref())
            {
                continue;
            }
            stages = None;
            match names.next() {
                Some(expected) if expected == pass.name.as_ref() => {}
                None if presented
                    && surface.is_none()
                    && pass.category == TimingCategory::Surface =>
                {
                    surface = Some(pass.nanoseconds);
                }
                _ => return None,
            }
        }
    }
    if culled != culling
        || expects_volume != view_volume
        || (layout.star_pass.is_some() && stars.is_none())
        || (layout.shadows == Shadows::Culled) != caster_cull.is_some()
    {
        return None;
    }
    (names.next().is_none() && surface.is_some() == presented).then_some((surface, contribution))
}

#[cfg(test)]
pub(crate) mod tests;
