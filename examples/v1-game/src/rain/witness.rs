//! Model samples for the native comparison on the actual valley projection.

use thrum_rain_model::{Wetness, wetness_at};

use super::{RainSelection, ValleyRain, ground};

/// Samples on the cooked floor, a rock's exposed side and shaded overhang,
/// and cell centres in the actual valley patch.
#[doc(hidden)]
pub struct RainWitness {
    /// World-space position and normal, padded for the GPU reader.
    pub point: [[f32; 4]; 2],
    /// The model's wetness at the sample.
    pub wetness: Wetness,
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
pub fn rain_witness_points(selection: RainSelection) -> Result<Vec<RainWitness>, String> {
    let (world, surface) = ValleyRain::prepare(selection)?;
    let (_, water) = surface.proof_products();
    let canopy = ground::canopy(&super::model_profile()).map_err(|error| error.to_string())?;
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
    let bed = world.tree.ground();
    for (i, (&height, &hollow)) in bed.heights().iter().zip(world.tree.labels()).enumerate() {
        let [x, z] = bed.centre(i);
        let mut point = make([x as f32, height as f32, z as f32], [0.0, 1.0, 0.0], true);
        if hollow != thrum_water_model::NO_HOLLOW {
            let level = world
                .state
                .level(&world.tree, world.state.holder(&world.tree, hollow));
            if (level - height).abs() <= 0.000_04 {
                // The exact waterline has its own quantisation threshold.
                continue;
            }
        }
        point.puddle = water
            .sample([x, z])
            .map(|sample| [sample.row.level, sample.depth]);
        points.push(point);
    }
    Ok(points)
}
