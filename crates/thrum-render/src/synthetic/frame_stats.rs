//! Renderer-owned interpretation of the bounded scene timestamp layout.

use super::{CullMode, PreparedSynthetic, history_gpu};

#[derive(Clone, Copy)]
pub(crate) struct SceneTimingLayout {
    shadows: bool,
    clusters: bool,
    transforms: bool,
    /// Frustum camera culling: one `camera visibility` compute pass per frame,
    /// timed even when the active prefix is empty.
    camera: CullMode,
}

impl From<&PreparedSynthetic> for SceneTimingLayout {
    fn from(prepared: &PreparedSynthetic) -> Self {
        Self {
            shadows: prepared.shadowed(),
            clusters: prepared.gpu_clustered(),
            transforms: prepared.active_instance_count() != 0,
            camera: prepared.camera_culling(),
        }
    }
}

/// Scene pass buckets (forward includes a contribution's compute pass), the optional surface blit
/// and the contribution's compute pass alone.
pub(crate) type PassDurations = ([f64; 11], Option<f64>, Option<f64>);

pub(crate) fn pass_durations(
    passes: &[crate::timing::PassTiming],
    presented: bool,
    layout: SceneTimingLayout,
) -> Option<PassDurations> {
    const ALL: [&str; 8] = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic Sun horizon heights",
        "synthetic cluster build",
        "synthetic forward opaque",
        history_gpu::PASS_NAME,
        "synthetic exposure resolve",
        "synthetic tone encode",
    ];
    let names = ALL.into_iter().filter(|&name| match name {
        "synthetic Sun shadow near"
        | "synthetic Sun shadow far"
        | "synthetic Sun horizon heights" => layout.shadows,
        "synthetic cluster build" => layout.clusters,
        history_gpu::PASS_NAME => layout.transforms,
        _ => true,
    });
    let count = names.clone().count();
    // A scene contribution adds one timed compute pass that the graph may order anywhere before
    // forward; it is reported on its own and the fixed scene sequence is matched without it.
    // The Sun air shadow heights pass, present with shadows under atmosphere lighting, is likewise
    // matched by name and shares the far shadow bucket with the horizon height map.
    // Frustum camera visibility is opaque-phase compute that feeds forward's indirect draw; it is
    // matched by name, required exactly when the slot layout culls, and counted in forward.
    let mut contribution = None;
    let mut air_shadow = None;
    let mut visibility = None;
    let mut scene_passes = Vec::with_capacity(passes.len());
    for pass in passes {
        if pass.name.as_ref() == super::AIR_SHADOW_PASS {
            if air_shadow.is_some() || !layout.shadows {
                return None;
            }
            air_shadow = Some(pass.nanoseconds);
        } else if pass.name.as_ref() == crate::visibility::PASS {
            if visibility.is_some() || layout.camera != CullMode::Frustum {
                return None;
            }
            visibility = Some(pass.nanoseconds);
        } else if pass.name.as_ref() == super::CONTRIBUTION_COMPUTE {
            if contribution.is_some() {
                return None;
            }
            contribution = Some(pass.nanoseconds);
        } else {
            scene_passes.push(pass);
        }
    }
    if layout.camera == CullMode::Frustum && visibility.is_none() {
        return None;
    }
    let passes = scene_passes.as_slice();
    let mut atmosphere = [0.0; 5];
    let mut offset = 0;
    for (stage, index) in crate::atmosphere::TIMING_ORDER {
        let name = crate::atmosphere::STAGE_NAMES[stage];
        if passes
            .get(offset)
            .is_some_and(|pass| pass.name.as_ref() == name)
        {
            atmosphere[index] += passes[offset].nanoseconds;
            offset += 1;
        }
    }
    (passes.len() == offset + count + usize::from(presented)
        && passes[offset..]
            .iter()
            .zip(names)
            .all(|(pass, name)| pass.name.as_ref() == name)
        && (!presented || passes[offset + count].name.as_ref() == crate::present::PASS_NAME))
        .then(|| {
            let mut scene = [0.0; 11];
            scene[6..].copy_from_slice(&atmosphere);
            for pass in passes[offset..].iter().take(count) {
                let slot = match pass.name.as_ref() {
                    "synthetic Sun shadow near" => 0,
                    // The horizon height map shares the far shadow bucket.
                    "synthetic Sun shadow far" | "synthetic Sun horizon heights" => 1,
                    "synthetic cluster build" => 2,
                    "synthetic forward opaque" => 3,
                    history_gpu::PASS_NAME => 4,
                    _ => 5,
                };
                // Preserve the existing display bucket, including its scalar resolve.
                scene[slot] += pass.nanoseconds;
            }
            // The contribution's compute pass is opaque-phase work that feeds forward: counted
            // in the forward bucket so totals include it, and also returned on its own.
            scene[3] += contribution.unwrap_or(0.0) + visibility.unwrap_or(0.0);
            scene[1] += air_shadow.unwrap_or(0.0);
            (
                scene,
                presented.then(|| passes[offset + count].nanoseconds),
                contribution,
            )
        })
}

#[cfg(test)]
mod tests;
