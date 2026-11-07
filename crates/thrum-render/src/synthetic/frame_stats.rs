//! Renderer-owned interpretation of the bounded scene timestamp layout.

use super::{CullMode, PreparedSynthetic, history_gpu};

#[derive(Clone, Copy)]
pub(crate) struct SceneTimingLayout {
    shadows: ShadowTiming,
    clusters: bool,
    transforms: bool,
    /// Bit `index` of [`CULLING_PASSES`] is set for each camera culling pass
    /// the frame records exactly once. Frustum: one `camera visibility`
    /// compute pass, timed even when the active prefix is empty. Occlusion:
    /// the prepass list, the depth prepass, the pyramid and the final cull.
    culling: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShadowTiming {
    None,
    Taps,
    Cells,
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
            shadows: if prepared.horizon.is_some() {
                ShadowTiming::Cells
            } else if prepared.shadowed() {
                ShadowTiming::Taps
            } else {
                ShadowTiming::None
            },
            clusters: prepared.gpu_clustered(),
            transforms: prepared.active_instance_count() != 0,
            culling: culling_passes(prepared.camera_culling()),
        }
    }
}

/// Scene pass buckets (forward includes a contribution's compute pass), the optional surface blit
/// and the contribution's compute pass alone.
pub(crate) type PassDurations = ([f64; 11], Option<f64>, Option<f64>);

/// Every camera culling pass name, in any layout.
const CULLING_PASSES: [&str; 5] = [
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
    const ALL: [&str; 8] = [
        "synthetic Sun shadow near",
        "synthetic Sun shadow far",
        "synthetic Sun horizon heights",
        "synthetic cluster build",
        "synthetic exposure resolve",
        "synthetic forward opaque",
        history_gpu::PASS_NAME,
        "synthetic tone encode",
    ];
    let names = ALL.into_iter().filter(|&name| match name {
        "synthetic Sun shadow near"
        | "synthetic Sun shadow far"
        | "synthetic Sun horizon heights" => layout.shadows != ShadowTiming::None,
        "synthetic cluster build" => layout.clusters,
        history_gpu::PASS_NAME => layout.transforms,
        _ => true,
    });
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
    let mut horizon = None;
    let mut visibility = None;
    let mut culled = 0_u8;
    let mut scene_passes = Vec::with_capacity(passes.len());
    for pass in passes {
        if pass.name.as_ref() == super::AIR_SHADOW_PASS {
            if air_shadow.is_some() || layout.shadows == ShadowTiming::None {
                return None;
            }
            air_shadow = Some(pass.nanoseconds);
        } else if pass.name.as_ref() == super::horizon::PASS {
            if horizon.is_some() || layout.shadows != ShadowTiming::Cells {
                return None;
            }
            horizon = Some(pass.nanoseconds);
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
        } else if pass.name.as_ref() == super::CONTRIBUTION_COMPUTE {
            if contribution.is_some() {
                return None;
            }
            contribution = Some(pass.nanoseconds);
        } else {
            scene_passes.push(pass);
        }
    }
    if culled != culling || horizon.is_some() != (layout.shadows == ShadowTiming::Cells) {
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
            scene[1] += air_shadow.unwrap_or(0.0) + horizon.unwrap_or(0.0);
            (
                scene,
                presented.then(|| passes[offset + count].nanoseconds),
                contribution,
            )
        })
}

#[cfg(test)]
mod tests;
