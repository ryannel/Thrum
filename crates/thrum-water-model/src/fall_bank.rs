//! A fall's clearance over the full range of the receiving map row.
use crate::{Below, BodyKind, Fall, FallSamples, Gauge, Map, Stage, WaterError};

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum Control {
    Stage(usize),
    Gauge(usize),
}

impl Control {
    fn stage(self, map: &mut Map) -> &mut Stage {
        match self {
            Self::Stage(body) => &mut map.bodies[body].stage,
            Self::Gauge(river) => {
                let Gauge::Stage(stage) = &mut map.rivers[river].gauge else {
                    unreachable!("a spring gauge owns its stage")
                };
                stage
            }
        }
    }
}

impl Map {
    /// Refuse any recorded fall whose receiving water can reach the lip.
    /// # Errors
    /// Names a drowned fall or a missing receiving body or path.
    pub fn validate_falls(&self, samples: &FallSamples) -> Result<(), WaterError> {
        for fall in samples.records() {
            self.validate_fall(fall)?;
        }
        Ok(())
    }

    /// Check a fall against the receiving row over the map's declared ranges.
    /// # Errors
    /// Names a drowned fall or a missing receiving body or path.
    pub fn validate_fall(&self, fall: &Fall) -> Result<(), WaterError> {
        let mut map = self.clone();
        for segment in &fall.segments {
            let (Some(lip), Some(foot)) = (segment.path.first(), segment.path.last()) else {
                return Err(WaterError::new(&fall.name, "fall has no path"));
            };
            let (Below::Body(name) | Below::Reach(name)) = &segment.below;
            let Some(receiver) = map.bodies.iter().position(|b| b.name == *name) else {
                return Err(WaterError::new(&fall.name, "fall receiver is missing"));
            };
            let mut controls = Vec::new();
            map.fall_controls(receiver, true, &mut controls);
            controls.sort_unstable();
            controls.dedup();
            if !map.clears_lip(receiver, foot.place, lip.height, &controls) {
                return Err(WaterError::new(
                    &fall.name,
                    "receiving water can drown the fall",
                ));
            }
        }
        Ok(())
    }

    fn fall_controls(&self, body: usize, joined: bool, controls: &mut Vec<Control>) {
        if let Some((river, _)) = self.river_of(body) {
            match self.rivers[river].gauge {
                Gauge::Body(id) => controls.push(Control::Stage(id)),
                Gauge::Chart { body, .. } => self.fall_controls(body, false, controls),
                Gauge::Stage(_) => controls.push(Control::Gauge(river)),
            }
        } else {
            controls.push(Control::Stage(body));
        }
        if joined && let BodyKind::Chart(chart) = &self.bodies[body].kind {
            for end in chart.ends.iter().filter(|end| !end.is_brink()) {
                self.fall_controls(end.body, true, controls);
            }
        }
    }

    fn clears_lip(
        &mut self,
        receiver: usize,
        place: [f64; 2],
        lip: f64,
        controls: &[Control],
    ) -> bool {
        if self.fall_row_bounds(receiver, place)[1] < lip {
            return true;
        }
        let mut widest = None;
        let mut width = 0.0;
        for &control in controls {
            let stage = control.stage(self);
            stage.value = f64::midpoint(stage.min, stage.max);
            if stage.max - stage.min > width {
                widest = Some(control);
                width = stage.max - stage.min;
            }
        }
        if self.row(receiver, place).level >= lip {
            return false;
        }
        let Some(split) = widest else {
            return true;
        };
        let original = *split.stage(self);
        // At this scale a remaining overlap cannot certify clearance. This
        // also closes the search for a lip tangent to an interior maximum.
        if !(original.min < original.value && original.value < original.max) {
            return false;
        }
        split.stage(self).max = original.value;
        let lower = self.clears_lip(receiver, place, lip, controls);
        split.stage(self).min = original.value;
        split.stage(self).max = original.max;
        let upper = lower && self.clears_lip(receiver, place, lip, controls);
        *split.stage(self) = original;
        upper
    }

    fn fall_row_bounds(&self, body: usize, place: [f64; 2]) -> [f64; 2] {
        let own = self.chart_level_range(body, place);
        let BodyKind::Chart(chart) = &self.bodies[body].kind else {
            return own;
        };
        let mut row = own;
        let mut total = [1.0, 1.0];
        let mut exact = [f64::INFINITY, f64::NEG_INFINITY];
        for end in chart.ends.iter().filter(|end| !end.is_brink()) {
            let other = self.fall_row_bounds(end.body, place);
            // For a mouth the weight grows as the other body rises relative
            // to the chart. The spatial join has constant weight at this point.
            let min = self.join_weight(end, place, own[1], other[0]);
            let max = self.join_weight(end, place, own[0], other[1]);
            if max >= 1.0 {
                exact = [exact[0].min(other[0]), exact[1].max(other[1])];
            }
            if min >= 1.0 {
                return exact;
            }
            let weight = |w: f64, accumulated: f64| {
                if w >= 1.0 {
                    1.0
                } else {
                    w / (w + (1.0 - w) * accumulated)
                }
            };
            let a = weight(min, total[1]);
            let b = weight(max, total[0]);
            let mix = |x: f64, y: f64, w: f64| x * (1.0 - w) + y * w;
            row = [
                mix(row[0], other[0], a).min(mix(row[0], other[0], b)),
                mix(row[1], other[1], a).max(mix(row[1], other[1], b)),
            ];
            total[0] += min / (1.0 - min);
            total[1] += max / (1.0 - max);
        }
        [row[0].min(exact[0]), row[1].max(exact[1])]
    }
}
