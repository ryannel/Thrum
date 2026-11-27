//! Model samples for the native comparison on the actual valley projection.

use thrum_rain_model::{Wetness, wetness_at};

use super::{RainSelection, RainValleyError, ValleyRain, ground};

/// Samples on the cooked floor, a rock's exposed side and shaded overhang,
/// and cell centres in the actual valley patch.
#[doc(hidden)]
pub struct RainWitness {
    /// World-space position and normal, padded for the GPU reader.
    pub point: [[f32; 4]; 2],
    /// The model's wetness at the sample.
    pub wetness: Wetness,
    /// Canopy exposure before the normal and store weighting.
    pub exposure: f64,
    /// The model's level and depth for a patch cell in water or its shore margin.
    pub puddle: Option<[f64; 2]>,
    /// The sample belongs to a patch cell, including dry cells.
    pub patch: bool,
}

/// Sample the CPU counterpart of the production rain field.
///
/// # Errors
/// Returns invalid authored ground or shelters.
#[doc(hidden)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the oracle uses the exact submitted f32 coordinates"
)]
pub fn rain_witness_points(selection: RainSelection) -> Result<Vec<RainWitness>, RainValleyError> {
    let canopy = ground::canopy().map_err(|error| RainValleyError::Model(error.to_string()))?;
    let world = ValleyRain::prepare_model(selection, &canopy).map_err(RainValleyError::Model)?;
    let water = world.water_map().map_err(RainValleyError::Model)?;
    let make = |position: [f32; 3], normal: [f32; 3], patch: bool| {
        let p = position.map(f64::from);
        let n = normal.map(f64::from);
        let wetness = if patch {
            world
                .state
                .wetness(&world.tree, &world.stores, 0, &canopy, p, n, None)
                .expect("patch proof uses its own ground lattice")
        } else {
            wetness_at(&world.stores, 0, &canopy, p, n, None)
        };
        RainWitness {
            point: [
                [position[0], position[1], position[2], 0.0],
                [normal[0], normal[1], normal[2], 0.0],
            ],
            wetness,
            exposure: canopy.exposure(p),
            puddle: None,
            patch,
        }
    };
    let mut points = vec![
        // Shipped terrain vertices to the left of the authored patch.
        make([-5.379_483, 239.950_96, 9.707_909], [0.0, 1.0, 0.0], false),
        make(
            [-5.379_483, 239.893_25, 4.867_127_4],
            [0.25, 0.968_245_86, 0.0],
            false,
        ),
        // Same rain column, above and under the cooked lookout rock.
        make([3.8, 242.2, -7.0], [0.0, 1.0, 0.0], false),
        make([3.8, 240.65, -7.0], [0.0, 1.0, 0.0], false),
        make([3.8, 241.2, -7.0], [1.0, 0.0, 0.0], false),
    ];
    // A populated-valley boulder row near the shelf stop, using its authored
    // instance transform and points on its imported upward mesh faces.
    let prop = crate::exit_scene::layout().entities[3801];
    let (angle, scale) = prop.orientation();
    let (sine, cosine) = angle.sin_cos();
    let origin = prop.position().map(|v| v as f32);
    for (position, normal) in ground::boulder_samples() {
        let rotate = |[x, y, z]: [f32; 3]| [cosine * x + sine * z, y, cosine * z - sine * x];
        let turned = rotate(position);
        points.push(make(
            std::array::from_fn(|i| origin[i] + scale * turned[i]),
            rotate(normal),
            false,
        ));
    }
    // The slope stop's boulder is outside the earlier lookout-only canopy.
    let [x, y, z] = crate::exit_scene::layout().entities[7899]
        .position()
        .map(|v| v as f32);
    points.push(make([x, y + 3.0, z], [0.0, 1.0, 0.0], false));
    points.push(make([x, y + 0.05, z], [0.0, 1.0, 0.0], false));
    if selection == RainSelection::Soaked {
        let [crown, under] = &points[points.len() - 2..] else {
            unreachable!()
        };
        assert!(crown.wetness.pores - under.wetness.pores > 0.2);
    }
    let bed = world.tree.ground();
    let [width, height] = bed.size();
    let near_level = |index: usize| {
        let label = world.tree.labels()[index];
        if label == thrum_water_model::NO_HOLLOW {
            return false;
        }
        let level = world
            .state
            .level(&world.tree, world.state.holder(&world.tree, label));
        (level - bed.heights()[index]).abs() <= 0.000_04
    };
    for (i, &elevation) in bed.heights().iter().enumerate() {
        let (x, z) = (i % width, i / width);
        // The margin can be supplied by any of the eight surrounding cells.
        // Omit the admission check only where one of those wet/dry thresholds
        // falls inside the agreed 40-micrometre level tolerance.
        let columns = x.saturating_sub(1)..=(x + 1).min(width - 1);
        if (z.saturating_sub(1)..=(z + 1).min(height - 1)).any(|row| {
            columns
                .clone()
                .any(|column| near_level(row * width + column))
        }) {
            continue;
        }
        let [x, z] = bed.centre(i);
        let mut point = make(
            [x as f32, elevation as f32, z as f32],
            [0.0, 1.0, 0.0],
            true,
        );
        point.puddle = water
            .sample([x, z])
            .map(|sample| [sample.row.level, sample.depth]);
        points.push(point);
    }
    Ok(points)
}
