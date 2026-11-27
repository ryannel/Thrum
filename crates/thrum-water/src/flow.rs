//! Scheduling and shader constants for the optional river wave field.
use thrum_render::contribution::{
    ContributionFrame, QualityInvalidation, QualityKnob, QualitySettings,
};
use thrum_water_model::Map;

pub(super) const CADENCE: &str = "river.flow_band_cadence";

#[derive(Clone, Copy)]
enum FieldWrite {
    Held,
    View(f64),
    Capture,
}

pub(super) struct Flow {
    pub(super) quality: QualitySettings,
    bounds: Vec<[glam::Vec3; 2]>,
    seen: bool,
    visible: bool,
    next: u64,
    committed: Option<f64>,
    pending: FieldWrite,
}

impl Flow {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounds use the renderer's f32 world coordinates"
    )]
    pub(super) fn new(map: &Map) -> Self {
        let bounds = (0..crate::map_gpu::BODY_LIMIT)
            .filter_map(|id| {
                let (river, chart) = map.river_of(id)?;
                let chart = &map.river(river)?.charts[chart];
                let [lo, hi] = map.surface_bounds(id)?;
                let (bottom, top) = chart
                    .samples
                    .iter()
                    .flat_map(|sample| &sample.levels)
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &level| {
                        (low.min(level), high.max(level))
                    });
                Some([
                    glam::Vec3::new(lo[0] as f32, (bottom - 2.0) as f32, lo[1] as f32),
                    glam::Vec3::new(hi[0] as f32, (top + 2.0) as f32, hi[1] as f32),
                ])
            })
            .collect();
        Self {
            quality: QualitySettings::new(vec![QualityKnob {
                name: CADENCE.into(),
                effect: "Submitted view frames between flow band evaluations",
                value: 1,
                range: 1..=4,
                invalidation: QualityInvalidation::Age,
                governor_steps: Some([1, 2, 4]),
            }])
            .expect("river flow cadence"),
            bounds,
            seen: false,
            visible: false,
            next: 0,
            committed: None,
            pending: FieldWrite::Held,
        }
    }

    pub(super) fn record(&mut self, frame: &ContributionFrame, now: f64, invalid: bool) -> f64 {
        let matrix = glam::Mat4::from_cols_array_2d(&frame.view_proj);
        let [x, y, z, w] = std::array::from_fn(|i| matrix.row(i));
        let planes = [w + x, w - x, w + y, w - y, z, w - z];
        self.visible = self.bounds.iter().any(|&[low, high]| {
            !planes.iter().any(|plane| {
                let axis = plane.truncate();
                let maximum = (low * axis).max(high * axis).element_sum();
                let guard = 16.0
                    * f32::EPSILON
                    * (low.abs().max(high.abs()).dot(axis.abs()) + plane.w.abs() + 1.0);
                maximum + plane.w < -guard
            })
        });
        let cadence = u64::from(self.quality.knobs()[0].value);
        self.pending = if frame.time.capture {
            FieldWrite::Capture
        } else if invalid
            || (self.visible
                && (!self.seen || self.committed.is_none() || self.next.is_multiple_of(cadence)))
        {
            FieldWrite::View(now)
        } else {
            FieldWrite::Held
        };
        match self.pending {
            FieldWrite::Held => now - self.committed.unwrap_or(now),
            _ => 0.0,
        }
    }

    pub(super) fn runs(&self) -> bool {
        !matches!(self.pending, FieldWrite::Held)
    }

    pub(super) fn submit(&mut self, advance: bool) {
        match self.pending {
            FieldWrite::Held => {}
            FieldWrite::View(now) => self.committed = Some(now),
            // The capture's scratch write cannot publish a display time.
            FieldWrite::Capture => self.committed = None,
        }
        if advance {
            self.seen = self.visible;
            self.next = self.next.wrapping_add(1);
        }
    }

    pub(super) fn reset(&mut self) {
        self.committed = None;
        self.pending = FieldWrite::Held;
        self.next = 0;
        self.seen = false;
    }

    pub(super) fn needs_query(&self, time: f64) -> bool {
        self.committed
            .is_none_or(|committed| committed.to_bits() != time.to_bits())
    }

    pub(super) fn submit_query(&mut self, time: f64) {
        self.committed = Some(time);
    }
}

pub(super) fn constants(profile: thrum_water_model::FlowBandsProfile, first: u32) -> String {
    let [short, long] = profile.patches;
    let [short_height, long_height] = profile.rms.map(|rms| 4.0 * rms);
    format!(
        "\nconst FLOW_FIRST: i32 = {first};\nconst FLOW_PATCHES = vec2<f32>({short:?}, {long:?});\nconst FLOW_HEIGHTS = vec2<f32>({short_height:?}, {long_height:?});\n"
    ) + include_str!("flow/gains.wgsl")
}
