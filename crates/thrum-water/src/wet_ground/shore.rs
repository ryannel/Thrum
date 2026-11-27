//! Sea and basin shore rows packed into the existing wetness field.
//! Heights are static. Only body rows and connectivity need tick uploads.

use crate::map_gpu::BODY_LIMIT;
use thrum_sea_model::Spectrum;
use thrum_water_model::{BodyKind, Map};

use super::{GroundClass, WetGround};

/// The map lattice's position in the wetness atlas.
#[derive(Clone, Debug)]
pub(super) struct ShoreLayout {
    pub(super) dynamic: usize,
    pub(super) cells: usize,
    pub(super) wave_heights: [f32; 4],
}

const BODY_ROWS: usize = 1 + BODY_LIMIT;

impl ShoreLayout {
    pub(super) fn row_count(&self) -> usize {
        BODY_ROWS + self.cells.div_ceil(4)
    }
}

impl WetGround {
    /// Prepare a ground field from the sea and basins of the supplied water map.
    /// The field borrows their still levels, connectivity and wave heights.
    ///
    /// Returns none if the map has a chart, flow bands, or more bodies than the
    /// renderer admits. Hollows, if present, belong to the rain ground lattice.
    #[must_use]
    pub fn new_mapped_shore(map: &Map, spectrum: &Spectrum, class: GroundClass) -> Option<Self> {
        if !supports_shore_map(map) {
            return None;
        }
        let mut wet = Self::prepare(
            None,
            None,
            &[class],
            0,
            Some((map, crate::records::layer_heights(spectrum))),
            0.0,
            0.0,
        )?;
        wet.publish_mapped_shore(map, &[0.0; BODY_LIMIT], class)
            .then_some(wet)
    }

    /// Publish one tick of shore levels, wave support and each body's damp band
    /// from the same map used at preparation. Margins use the map's body order.
    /// Returns false if the prepared field, map layout or band count is incompatible.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "shore rows store GPU metres and admitted atlas addresses"
    )]
    pub fn publish_mapped_shore(&mut self, map: &Map, margins: &[f64], class: GroundClass) -> bool {
        let Some(layout) = &self.shore_layout else {
            return false;
        };
        let (origin, spacing, size) = map.bed().lattice();
        let bodies = (0..BODY_LIMIT).map_while(|id| map.body(id));
        let count = bodies.clone().count();
        if layout.cells != map.bed().heights().len()
            || self.rows[4].map(f32::to_bits)
                != [
                    origin[0] as f32,
                    origin[1] as f32,
                    spacing as f32,
                    size[0] as f32,
                ]
                .map(f32::to_bits)
            || self.rows[5][0].to_bits() != (size[1] as f32).to_bits()
            || margins.len() < count
            || margins[..count]
                .iter()
                .any(|&margin| !margin.is_finite() || margin < 0.0)
            || !supports_shore_map(map)
        {
            return false;
        }
        let profile = map.profile();
        let heights = layout.wave_heights;
        let start = layout.dynamic;
        let mut highest_wet_edge = f32::NEG_INFINITY;
        let (records, masks) = self.rows.split_at_mut(start + BODY_ROWS);
        records[start + 1..].fill([0.0; 4]);
        let changed = self.shore_revision != Some(map.spatial_revision());
        let masks = &mut masks[..layout.cells.div_ceil(4)];
        if changed {
            masks.fill([0.0; 4]);
        }
        for (id, (body, &margin)) in bodies.zip(margins).enumerate() {
            let kind = match body.kind {
                BodyKind::Sea => 1.0,
                BodyKind::Basin { .. } => 2.0,
                _ => continue,
            };
            let significant_height = body
                .gains
                .iter()
                .zip(heights)
                .map(|(&gain, height)| (gain * f64::from(height)).powi(2))
                .sum::<f64>()
                .sqrt();
            let swash = (profile.swash_ratio * significant_height).min(profile.margin_height);
            let row = [
                (body.level + body.stage.value) as f32,
                swash as f32,
                margin as f32,
                kind,
            ];
            records[start + 1 + id] = row;
            highest_wet_edge = highest_wet_edge.max(row[0] + row[1] + row[2]);
            if changed {
                let bit = f32::from(1_u16 << id);
                for (pack, origins) in masks.iter_mut().zip(map.support_origins(id).chunks(4)) {
                    for (mask, origin) in pack.iter_mut().zip(origins) {
                        if origin.is_some() {
                            *mask += bit;
                        }
                    }
                }
            }
        }
        records[start] = [
            count as f32,
            profile.margin_height as f32,
            (start + BODY_ROWS) as f32,
            highest_wet_edge,
        ];
        self.shore_revision = Some(map.spatial_revision());
        self.publish(&[class], &[[[0.0; 2]; 3]], &[])
    }
}

fn supports_shore_map(map: &Map) -> bool {
    map.body(BODY_LIMIT).is_none()
        && map.flow_bands().is_none()
        && (0..BODY_LIMIT)
            .map_while(|id| map.body(id))
            .all(|body| !matches!(body.kind, BodyKind::Chart(_)))
}
