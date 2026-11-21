//! Renderer-owned interpretation of the bounded scene timestamp layout.

use super::{CullMode, OffscreenTarget, PreparedSynthetic};
use crate::pass_key::{AtmosphereStage, PassKey};
use crate::timing::TimingCategory;

impl OffscreenTarget {
    pub(crate) fn effect_pass_names(&self) -> impl Iterator<Item = String> + '_ {
        self.effects.names.iter().cloned()
    }
    pub(crate) fn untimed_pass_names(
        &self,
        readback: bool,
        presented: bool,
    ) -> impl Iterator<Item = String> + '_ {
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
            .map(|(_, pass)| pass.name.clone())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SceneTimingLayout {
    shadows: Shadows,
    cascades: bool,
    clusters: bool,
    transforms: bool,
    star_pass: Option<&'static str>,
    /// Bit `index` of [`CULLING_PASSES`] is set for each camera culling pass
    /// the frame records exactly once. Frustum: one `camera visibility`
    /// compute pass, timed even when the active prefix is empty. Occlusion:
    /// the prepass list, the depth prepass, the pyramid and the final cull.
    culling: u8,
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
            star_pass: None,
            culling: culling_passes(prepared.camera_culling()),
        }
    }
}

impl From<(&PreparedSynthetic, &OffscreenTarget)> for SceneTimingLayout {
    fn from((prepared, target): (&PreparedSynthetic, &OffscreenTarget)) -> Self {
        Self::from(prepared).with_stars(target)
    }
}

impl SceneTimingLayout {
    pub(crate) fn with_stars(mut self, target: &OffscreenTarget) -> Self {
        self.star_pass = target.draw_stars.then_some(super::stars::PASS_NAME);
        self
    }
}

/// Named executed scene passes, the optional surface blit and the contribution's
/// compute pass alone. Summaries never reinterpret an ordinal.
pub(crate) type PassDurations = (crate::timing::SceneGpuTimings, Option<f64>, Option<f64>);

/// Every camera culling pass name, in any layout.
pub(crate) const CULLING_PASSES: [&str; 5] = [
    crate::visibility::PASS,
    crate::visibility::PREPASS_LIST_PASS,
    super::camera_culling::DEPTH_PREPASS_PASS,
    crate::visibility::PYRAMID_PASS,
    crate::visibility::FINAL_PASS,
];

pub(crate) fn pass_durations(
    passes: &[crate::timing::PassTiming],
    presented: bool,
    layout: SceneTimingLayout,
) -> Option<PassDurations> {
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
    let raw_passes = passes;
    let names = ALL
        .into_iter()
        .filter(|&key| match key {
            PassKey::ShadowNear | PassKey::ShadowFar => layout.cascades,
            PassKey::HorizonHeights => layout.shadows != Shadows::Off,
            PassKey::Cluster => layout.clusters,
            PassKey::History => layout.transforms,
            _ => true,
        })
        .map(PassKey::constant_name);
    let count = names.clone().count();
    // A scene contribution adds one timed compute pass that the graph may order anywhere before
    // forward; it is reported on its own and the fixed scene sequence is matched without it.
    // The Sun air shadow heights pass, present with shadows under atmosphere lighting, is likewise
    // matched by name and shares the far shadow bucket with the horizon height map.
    // Camera culling passes are opaque-phase work around forward's indirect draws: matched by
    // name, each required exactly once when the slot layout uses them, and counted in forward.
    let culling = layout.culling;
    let mut contribution = None;
    let mut air_shadow = None;
    let mut visibility = None;
    let mut culled = 0_u8;
    let mut stars = None;
    let mut caster_cull = None;
    let mut scene_passes = Vec::with_capacity(passes.len());
    for pass in passes {
        if pass.category == TimingCategory::Temporal {
            continue;
        }
        if matches!(
            pass.category,
            TimingCategory::Effects | TimingCategory::SecondaryViews
        ) {
            // These are prepared contribution passes. Keep their original names in
            // SceneGpuTimings below; they do not belong to the fixed scene sequence.
        } else if pass.name.as_ref() == super::AIR_SHADOW_PASS {
            if air_shadow.is_some() || layout.shadows == Shadows::Off {
                return None;
            }
            air_shadow = Some(pass.nanoseconds);
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
            visibility = Some(visibility.unwrap_or(0.0) + pass.nanoseconds);
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
            scene_passes.push(pass);
        }
    }
    if culled != culling
        || (layout.star_pass.is_some() && stars.is_none())
        || (layout.shadows == Shadows::Culled) != caster_cull.is_some()
    {
        return None;
    }
    let passes = scene_passes.as_slice();

    let mut offset = 0;
    for stage in AtmosphereStage::ORDER {
        let name = stage.name();
        if passes
            .get(offset)
            .is_some_and(|pass| pass.name.as_ref() == name)
        {
            offset += 1;
        }
    }
    (passes.len() == offset + count + usize::from(presented)
        && passes[offset..]
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == name)
        && (!presented || passes[offset + count].category == TimingCategory::Surface))
        .then(|| {
            (
                crate::timing::SceneGpuTimings::new(raw_passes),
                presented.then(|| passes[offset + count].nanoseconds),
                contribution,
            )
        })
}

#[cfg(test)]
mod tests;
