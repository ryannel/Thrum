//! The sprite field's authored art and mapping of ordinary V1 simulation pages.

use std::num::NonZeroU32;

use crate::{
    exit_scene::Drift,
    scene::{ENTITY_COUNT, PEER_COUNT, peer_ownership, register_peers},
    simulation::{ComponentEntity, PublishedPage, SimConfig, Simulation, Velocity},
};
use thrum_core::{StableId, WorldEpoch};
use thrum_render::sprite::{
    EmbeddedSheet, SpritePose, SpriteRect, SpriteRow, SpriteScene, SpriteSink,
};

#[cfg(not(target_os = "ios"))]
use thrum_render::sprite::{SpriteFrame, SpriteView};

const SIDE: u32 = 20;
const FRAME_COUNT: u32 = 4;
const MARK: [SpriteRow; 2] = [
    SpriteRow {
        layer: -1,
        tint: [0.11, 0.30, 0.68, 0.76],
        additive: 1.0,
        scale: [6.0; 2],
        ..SpriteRow::new(3, [-0.14, 0.14])
    },
    SpriteRow {
        layer: 1,
        tint: [0.22, 0.65, 1.0, 0.88],
        additive: 1.0,
        scale: [4.0; 2],
        ..SpriteRow::new(3, [0.01, 0.11])
    },
];

pub(crate) fn simulation(count: u32, interactive: bool) -> Result<Simulation, String> {
    let maximum = u32::try_from(ENTITY_COUNT).expect("V1 grid fits in u32");
    let peers = u32::try_from(PEER_COUNT).expect("V1 peer count fits in u32");
    if !(peers..=maximum).contains(&count) {
        return Err(format!(
            "sprite field row count must be in {PEER_COUNT}..={ENTITY_COUNT}"
        ));
    }
    // Reuse the game's closed-drift component, rules and publisher.
    let mut sim = Simulation::new_exit(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("sprite field tick rate"),
            entity_capacity: count as usize,
            peer_capacity: PEER_COUNT,
            actions_per_frame: 8,
            movement_speed: if interactive { 0.9 } else { 6.0 },
        },
        false,
    )
    .map_err(|err| format!("sprite field simulation: {err:?}"))?;
    for slot in 0..count {
        let position = f64::from(slot) / f64::from(count);
        let grid_slot = slot * maximum / count;
        let group = slot % 11;
        let drift = if slot < 8 {
            Drift(
                [0.0, 0.09 + f64::from(slot % 2) * 0.06, 0.0],
                [0.66 - f64::from(slot) * 0.06, 0.12, 0.0],
                360,
                slot * 129 % 360,
            )
        } else {
            Drift(
                [
                    (f64::from(grid_slot % 125) - 62.0) * 0.027,
                    (f64::from(grid_slot / 125) - 39.5) * 0.024,
                    0.0,
                ],
                [
                    0.052 + f64::from(group) * 0.003,
                    0.023 + 0.020 * position,
                    0.0,
                ],
                173 + group * 17,
                slot * 37,
            )
        };
        let player = interactive && slot + 1 == count;
        sim.seed_drift(
            ComponentEntity {
                id: StableId(u128::from(slot)),
                position: if player {
                    crate::simulation::Position([-1.0, -0.93, 0.0])
                } else {
                    drift.at(0)
                },
                velocity: Velocity::default(),
                health: 100,
                ownership: peer_ownership(count as usize, slot as usize)?,
                visual: Some(u64::from(group)),
            },
            (!player).then_some(drift),
        )
        .map_err(|err| format!("sprite field entity {slot}: {err:?}"))?;
    }
    register_peers(&mut sim, count as usize)?;
    Ok(sim)
}

pub(crate) fn scene(count: u32) -> SpriteScene {
    let mut scene = SpriteScene::new(
        EmbeddedSheet {
            extent: [SIDE * FRAME_COUNT, SIDE],
            rgba: pixels().into(),
            frames: (0..FRAME_COUNT)
                .map(|frame| SpriteRect {
                    origin: [SIDE * frame, 0],
                    size: [SIDE; 2],
                })
                .collect(),
            texels_per_unit: 540.0,
        },
        u32::try_from(MARK.len()).expect("fixed frame row count"),
    );
    scene.world_capacity = count;
    scene.order_band = 0.012;
    scene
}

#[derive(Clone, Copy)]
pub(crate) struct FieldSheets {
    slot: u32,
    frames: [u32; 3],
}

pub(crate) fn cooked_sheets(
    scene: &mut SpriteScene,
    source: thrum_asset_gpu::CookedSource,
) -> Result<(FieldSheets, thrum_asset_gpu::CookedAssets), thrum_render::runtime::FrameError> {
    const SHEET: &str = "field";
    let mut config = thrum_asset_gpu::CookedConfig::new(source, Vec::new());
    config.lease_sheet(SHEET);
    config.upload_budget = thrum_asset_gpu::COPY_UNIT;
    let assets = thrum_asset_gpu::CookedAssets::new(config)
        .map_err(|error| thrum_render::runtime::FrameError::Scene(error.to_string()))?;
    scene.set_sheet_source(assets.lease());
    let sheet = scene.add_cooked_sheet(SHEET)?;
    Ok((
        FieldSheets {
            slot: sheet.slot(),
            frames: ["walk.0", "walk.1", "walk.2"].map(|name| sheet.frame(name)),
        },
        assets,
    ))
}

#[cfg(not(target_os = "ios"))]
pub(crate) fn frame(extent: [u32; 2], moving: Option<(u64, f32)>) -> SpriteFrame<'static> {
    let centre = moving.map_or([0.0; 2], |(tick, alpha)| {
        let phase = (f64::from(u32::try_from(tick % 360).expect("bounded phase"))
            + f64::from(alpha))
            * std::f64::consts::TAU
            / 360.0;
        [0.24 * phase.sin(), 0.10 * (2.0 * phase).sin()]
    });
    SpriteFrame {
        view: SpriteView {
            centre,
            pixels_per_unit: extent[1] as f32 * 0.455,
        },
        rows: &MARK,
    }
}

pub(crate) fn extract(
    into: &mut SpriteSink<'_>,
    page: &PublishedPage,
    cooked: Option<&FieldSheets>,
    player: Option<u32>,
) {
    const TINTS: [[f32; 4]; 4] = [
        [0.24, 0.80, 0.69, 1.0],
        [0.24, 0.40, 0.76, 1.0],
        [0.95, 0.43, 0.20, 1.0],
        [0.58, 0.28, 0.63, 1.0],
    ];
    let tick = page.id().tick;
    page.visit(|handle, id, position, visual| {
        let Some(visual) = visual else { return };
        let phase = u64::from(handle.slot) * 17;
        let animation = u32::try_from(((tick + phase) / 7) % 3).expect("three authored frames");
        let palette = usize::try_from(visual % 4).expect("four authored tints");
        let featured = handle.slot < 8;
        let mut row = SpriteRow::new(animation, [position.0[0], position.0[1]]);
        row.tint = TINTS[palette];
        if featured {
            row.layer = 1;
            row.scale = [4.0; 2];
            if (tick + phase) % 53 < 7 {
                row.flash = [1.0, 0.84, 0.53, 0.93];
            }
            if handle.slot % 3 == 2 {
                row.additive = 0.8;
            }
        } else if handle.slot % 67 == 0 {
            row.frame = 3;
            row.additive = 1.0;
        }
        if let Some(cooked) = cooked {
            row.sheet = cooked.slot;
            row.frame = cooked.frames[animation as usize];
            row.tint = [1.0; 4];
            row.flash = [0.0; 4];
            row.additive = 0.0;
        }
        if player == Some(handle.slot) {
            row.sheet = 0;
            row.frame = 3;
            row.tint = [1.0, 0.77, 0.19, 1.0];
            row.additive = 0.2;
            row.layer = 2;
            row.scale = [3.0; 2];
        }
        // Every field entity is seeded with a visual for its entire lifetime.
        // The tick page guards the previous position by identity and generation.
        let previous = page.previous_position(handle).map(|position| SpritePose {
            position: [position.0[0], position.0[1]],
            ..row.into()
        });
        into.push_pair(handle.slot, handle.generation, id.0, row, previous);
    });
}

fn pixels() -> Vec<u8> {
    let mut rgba = Vec::with_capacity((SIDE * SIDE * FRAME_COUNT * 4) as usize);
    for y in 0_i32..20 {
        for frame in 0_i32..4 {
            for x in 0_i32..20 {
                let [dx, dy] = [x * 2 - 19, y * 2 - 19];
                let r2 = dx * dx + dy * dy;
                let texel = if frame == 3 {
                    match r2 {
                        0..=57 => [218, 230, 255, 230],
                        58..=144 => [160, 188, 250, 120],
                        145..=226 => [94, 136, 225, 72],
                        227..=350 => [64, 100, 204, 30],
                        _ => [0; 4],
                    }
                } else if dx.abs() + (dy + 1).abs() > 19 || dy > 13 + frame {
                    [0; 4]
                } else if dx.abs() + dy.abs() > 14 {
                    [34, 44, 66, 235]
                } else if (y == 7 + frame || y == 8 + frame) && (5..15).contains(&x) {
                    [237, 243, 248, 255]
                } else if dy > 4 {
                    [100, 129, 154, 240]
                } else {
                    [167, 194, 212, 250]
                };
                rgba.extend(texel);
            }
        }
    }
    rgba
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
#[cfg(feature = "proving")]
pub(crate) fn sample_stats(
    sample: &thrum_render::runtime::FrameSample,
) -> Option<serde_json::Value> {
    let sprites = sample.sprites?;
    Some(serde_json::json!({
        "world_rows": sprites.world_rows,
        "sheet_generation": sprites.sheet_generation,
        "pending_sheet_rows": sprites.pending_sheet_rows,
        "missing_frame_rows": sprites.missing_frame_rows,
        "view_centre": sprites.view_centre,
        "frame_rows": sprites.frame_rows,
        "world_upload_rows": sprites.world_upload_rows,
        "world_upload_bytes": sprites.world_upload_bytes,
        "order_upload_bytes": sprites.order_upload_bytes,
        "order_cpu_ns": sprites.order_cpu.as_nanos(),
        "shaded_pixels": sprites.shaded_pixels,
        "draws": sprites.draws,
        "upload_bytes": sample.uploaded_bytes,
        "moving": sample.moving,
        "dropped": sample.dropped_presentation,
        "alpha": sample.presentation_alpha,
        "tick": sample.tick,
        "surface_presented": sample.surface_presented,
    }))
}
